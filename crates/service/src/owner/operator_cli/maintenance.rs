//! An operation reserves only the setup whose mutable state it will use.
//! The reservation is retained with that operation; releasing it never starts
//! or stops the service, a DAW, or a different setup.
use super::*;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reservation {
    schema: u32,
    pub owner: ResumeRecord,
    pub environment: Environment,
    request: Artifact,
    held: bool,
}
fn path(m: &Manager, operation: &str) -> Result<PathBuf> {
    Ok(job_dir(m, operation)?.join("maintenance.json"))
}
pub(super) fn read(m: &Manager, operation: &str) -> Result<Option<Reservation>> {
    let path = path(m, operation)?;
    if !path.try_exists()? { return Ok(None); }
    let record: Reservation = serde_json::from_slice(&control_artifact(path)?.record_bytes(32768)?)?;
    require(record.schema == 1 && valid_product_environment(&record.environment.id)
        && record.owner.owner_operation == operation
        && record.request.path == job_dir(m, operation)?.join("request.json")
        && record.environment.root == m.root.join("environments").join(&record.environment.id),
        "setup_maintenance_binding")?;
    record.request.record_bytes(contract::REQUEST_LIMIT)?;
    Ok(Some(record))
}
pub(super) fn held(m: &Manager) -> Result<Vec<Reservation>> {
    m.derive_once("setup_maintenance_held", || read_held(m))
}
fn read_held(m: &Manager) -> Result<Vec<Reservation>> {
    let directory = m.root.join("operator");
    if !directory.try_exists()? { return Ok(vec![]); }
    let mut records = Vec::new();
    for (index, entry) in fs::read_dir(directory)?.enumerate() {
        require(index < 2048, "operator_inventory_bound")?;
        let entry = entry?;
        let Some(id) = entry.file_name().to_str().map(str::to_owned) else { continue };
        if !valid_hex(&id, 32) || !entry.file_type()?.is_dir() { continue; }
        if let Some(record) = read(m, &id)?.filter(|record| record.held) { records.push(record); }
    }
    Ok(records)
}
fn same_setup(a: &Environment, b: &Environment) -> bool { a.id == b.id || a.root == b.root }
pub(crate) fn blocks(m: &Manager, environment: &Environment) -> Result<bool> {
    Ok(held(m)?.iter().any(|record| same_setup(&record.environment, environment)))
}
pub(crate) fn owned(m: &Manager, operation: &str) -> Result<Option<Environment>> {
    Ok(read(m, operation)?.filter(|record| record.held).map(|record| record.environment))
}

/// Resolve scope from the accepted action's existing owner records. A display
/// name, arbitrary path, or a client's claimed process is never a scope. This
/// also serves passive views; execution owners verify inputs before mutation.
pub(super) fn environment(m: &Manager, action: &ui::Action) -> Result<Option<Environment>> {
    use ui::Action as A;
    let resolved = match action {
        A::PresetFiles { product, .. } => Some(environment_install::target_record(m, &product.environment)?),
        A::InstallerStart { onboarding } | A::InstallerStartWithPolicy { onboarding, .. }
        | A::InstallerScan { onboarding } => Some(m.derive_once(&format!("maintenance_onboarding:{onboarding}"),
            || Ok(onboarding::control_record(m, onboarding)?.environment))?),
        A::EnvironmentInstallerStart { environment, .. }
        | A::EnvironmentInstallerStartWithPolicy { environment, .. }
        | A::EnvironmentInstallerScan { environment } | A::EnvironmentRuntimeSelect { environment, .. }
        | A::EnvironmentRescan { environment } | A::EnvironmentRefresh { environment, .. }
        | A::PluginFinishSetup { environment, .. } | A::QuarantinedModuleRetry { environment, .. }
        | A::ProductUninstall { environment, .. } | A::SetupRemoveFromDaw { environment }
        | A::SetupDelete { environment, .. }
        | A::ProgramChoose { environment, .. } => Some(environment_install::target_record(m, environment)?),
        A::VendorSoftwareInstall { application, .. } | A::VendorApplicationOpen { application }
        | A::ProgramAdopt { application, .. } => Some(vendor_cli::application(m, application)?.environment),
        A::RendererDiscover {} => {
            let root = m.root.join("environments").join(renderer::ENVIRONMENT);
            let environment: Environment = read_json(&root.join("environment.json"))?;
            require(environment.id == renderer::ENVIRONMENT && environment.root == root,
                "setup_maintenance_scope_invalid")?;
            Some(environment)
        }
        A::RendererOpen { .. } | A::RendererSoftwareInstall { .. } | A::DependencyPrepare {} =>
            Some(renderer_cli::current_application(m)?.environment),
        A::EnvironmentRefreshResume { operation } if renderer_continuation::owns(m, operation)? =>
            Some(renderer_continuation::load(m, operation)?.before),
        A::EnvironmentRefreshResume { operation } => Some(environment_refresh::load(m, operation)?.environment),
        A::SoftwareInstallResume { onboarding } => onboarding::retained_environment_record(m, onboarding)?
            .map(|record| record.environment),
        A::InstallerCloseCheck { operation } => {
            if let Some(record) = read(m, operation)? { Some(record.environment) }
            else if uninstall::owns(m, operation)? { Some(uninstall::owner(m, operation)?.environment) }
            else { Some(environment_install::load(m, operation)?.environment) }
        }
        A::OrdinaryRollback { class_id, .. } | A::OrdinaryRestoreRecommended { class_id }
        | A::PluginAddBackToDaw { class_id, .. } | A::PluginRemoveFromDaw { class_id, .. }
        | A::BufferingSet { class_id, .. } | A::DeliverySet { class_id, .. } =>
            Some(m.composed_registry()?.classes.get(class_id).ok_or("class not registered")?.registration.environment.clone()),
        _ if preparation_cli::is_action(action) => preparation_cli::action_selection_record(m, &software_record(m)?, action)?
            .map(|selection| selection.environment),
        _ => None,
    };
    Ok(resolved)
}
/// An initial automatic request creates its setup before entering the installer
/// stage. Only its unique retained intent can name that newly created scope.
fn accepted_environment(m: &Manager, operation: &str, action: &ui::Action) -> Result<Environment> {
    if let Some((installer, daw, policy)) = action.software_install() {
        let records = onboarding::history_records(m)?;
        let mut matching = records.iter().filter(|record| record.automatic.as_ref()
            .is_some_and(|intent| intent.owner == operation && intent.active_operation == operation));
        let record = matching.next().ok_or("setup_maintenance_scope_unresolved")?;
        require(matching.next().is_none(), "setup_maintenance_scope_ambiguous")?;
        let intent = record.automatic.as_ref().unwrap();
        require(record.installer == installer && intent.journey.daw == daw
            && intent.installer_policy == policy && intent.vendor.is_none(),
            "setup_maintenance_automatic_binding")?;
        automatic_install::initial_installer_input(m, record, intent)?;
        return Ok(onboarding::load(m, &record.id)?.environment);
    }
    environment(m, action)?.ok_or_else(|| "setup_maintenance_scope_unresolved".into())
}
/// Caller holds the registry lock when this protects a mutation.
pub(super) fn require_publications_settled(m: &Manager, environment: &Environment) -> Result<()> {
    for (class, entry) in &m.registry()?.classes {
        if same_setup(&entry.registration.environment, environment) {
            require(!m.publication_pending(class)?, "operator_pending_transaction_reconcile")?;
        }
    }
    Ok(())
}
pub(super) fn action_lock(m: &Manager, action: &ui::Action) -> Result<Lock> {
    let id = if let ui::Action::SetupDelete { environment, .. } = action {
        require(valid_product_environment(environment), "setup_maintenance_scope_invalid")?;
        environment.clone()
    } else { environment(m, action)?.ok_or("setup_maintenance_scope_unresolved")?.id };
    m.lock(&format!("operator-environment-{id}.lock"))
}
pub(super) fn acquire(m: &Manager, operation: &str, vendor_operation: Option<String>,
    timeout: Duration, waits: &mut Vec<ui::LockFacts>) -> Result<()> {
    let request = control_artifact(job_dir(m, operation)?.join("request.json"))?;
    let action: ui::Request = serde_json::from_slice(&request.record_bytes(contract::REQUEST_LIMIT)?)?;
    let environment = accepted_environment(m, operation, &action.action)?;
    let retired = vendor_maintenance::observe_retired(m, &environment)?;
    {
        let _registry = acquire_readback(m, ui::OperatorLock::Registry, Some(operation), timeout, waits)?;
        retired.verify(m)?;
        capacity::require_environment_inactive(m, &environment)?;
        require(held(m)?.iter().all(|record| record.owner.owner_operation == operation
            || !same_setup(&record.environment, &environment)), "setup_maintenance_active")?;
        require(!vendor_maintenance::blocks(m, &environment)?, "setup_maintenance_active")?;
        require(!m.root.join("operator/resume.json").try_exists()?, "earlier_maintenance_requires_recovery")?;
        let record = Reservation { schema: 1, environment, request, held: true,
            owner: ResumeRecord { schema: 2, owner_operation: operation.into(), resume: false,
                software: software_record(m)?.manager.sha256, vendor_operation } };
        atomic_json(&path(m, operation)?, &record)?;
    }
    // Admission is now reserved. No new instance/keeper may enter this setup.
    let environment = owned(m, operation)?.ok_or("setup_maintenance_owner_missing")?;
    if capacity::owners_for(m, Some(&environment))?.iter().any(|owner| owner.kind == capacity::Kind::Keeper) {
        retire_setup_for_vendor(m, operation)?;
    }
    Ok(())
}
/// True means this operation used the current scoped protocol. Failed cleanup
/// retains the reservation; a terminal receipt alone cannot release custody.
pub(super) fn release(m: &Manager, operation: &str) -> Result<bool> {
    let Some(mut record) = read(m, operation)? else { return Ok(false) };
    if !record.held { return Ok(true); }
    let retired = vendor_maintenance::observe_retired(m, &record.environment)?;
    let _registry = m.lock("registry.lock")?;
    retired.verify(m)?;
    capacity::require_environment_inactive(m, &record.environment)?;
    for owner in capacity::owners_for(m, Some(&record.environment))? {
        require(!same_setup(&m.lease_environment(&owner.session)?, &record.environment),
            "setup_maintenance_cleanup_unconfirmed")?;
    }
    record.held = false;
    atomic_json(&path(m, operation)?, &record)?;
    Ok(true)
}
/// A terminal receipt can precede cleanup during a crash. Only a positively
/// dead executor and retired exact setup release its retained reservation.
/// Active or unobservable executors keep all of their custody unchanged.
pub(super) fn reconcile(m: &Manager, mut worker_live: impl FnMut(&str) -> Result<bool>) -> Result<Vec<String>> {
    let mut failures = vec![];
    for record in held(m)? {
        let owner = record.owner.owner_operation.as_str();
        if !matches!(worker_live(owner), Ok(false)) { continue; }
        let Ok(_worker) = m.lock(&format!("operator-worker-{owner}.lock")) else { continue; };
        if !matches!(worker_live(owner), Ok(false)) { continue; }
        let result = optional(&job_dir(m, owner)?.join("result.json"))?;
        if result["operation"] != owner || !matches!(result["state"].as_str(), Some("completed" | "refused")) {
            continue;
        }
        if let Err(error) = release(m, owner) { failures.push(format!("{owner}: {error}")); }
    }
    Ok(failures)
}
pub(super) fn transfer(m: &Manager, from: &str, to: &str, timeout: Duration,
    waits: &mut Vec<ui::LockFacts>) -> Result<bool> {
    let Some(mut previous) = read(m, from)? else { return Ok(false) };
    let request = control_artifact(job_dir(m, to)?.join("request.json"))?;
    let new_request: ui::Request = serde_json::from_slice(&request.record_bytes(contract::REQUEST_LIMIT)?)?;
    require(matches!(new_request.action, ui::Action::InstallerCloseCheck { operation } if operation == from),
        "setup_maintenance_continuation_binding")?;
    let _registry = acquire_readback(m, ui::OperatorLock::Registry, Some(to), timeout, waits)?;
    capacity::require_environment_inactive(m, &previous.environment)?;
    require(held(m)?.iter().all(|record| record.owner.owner_operation == from
        || !same_setup(&record.environment, &previous.environment)), "setup_maintenance_active")?;
    let mut next = previous.clone();
    next.request = request;
    next.owner.owner_operation = to.into();
    next.held = true;
    atomic_json(&path(m, to)?, &next)?;
    previous.held = false;
    atomic_json(&path(m, from)?, &previous)?;
    Ok(true)
}

/// Availability comes from the service and uses the same exact target as
/// mutation admission. A frontend must not reinterpret whole-machine counters.
pub(super) fn offer_refusal(m: &Manager, action: &ui::Action) -> Result<Option<String>> {
    use ui::Action as A;
    if !action.requires_inactive() || matches!(action, A::TransactionReconcile {}) {
        return Ok(None);
    }
    if matches!(action, A::RuntimeInstall {} | A::InstallerEnvironmentCreate {..}
        | A::InstallerNewAttempt {..} | A::SoftwareInstall {..} | A::SoftwareInstallWithPolicy {..}) {
        return Ok(None);
    }
    if let A::SetupDelete { environment, .. } = action {
        if deleted_setup::load(m, environment)?.is_some() { return Ok(None); }
    }
    let Some(environment) = environment(m, action)? else {
        return Ok(Some("This action's setup ownership could not be resolved".into()));
    };
    let key = format!("setup_availability:{}", environment.id);
    let setup_reason: Option<String> = m.derive_once(&key, || {
        if blocks(m, &environment)? || vendor_maintenance::blocks(m, &environment)? {
            return Ok(Some("Wait for this setup's maintenance operation to finish".into()));
        }
        if let Err(error) = vendor_maintenance::observe_retired(m, &environment) {
            return Ok(Some(format!("This setup's previous operation has not retired: {error}")));
        }
        for (class, entry) in &m.composed_registry()?.classes {
            if same_setup(&entry.registration.environment, &environment) && m.publication_pending(class)? {
                return Ok(Some("Reconcile this setup's interrupted publication first".into()));
            }
        }
        Ok(None)
    })?;
    if setup_reason.is_some() { return Ok(setup_reason); }
    let class = match action {
        A::BufferingSet {class_id,..} | A::DeliverySet {class_id,..} | A::PluginRemoveFromDaw {class_id,..}
            => Some(class_id.as_str()),
        _ => None,
    };
    m.derive_once(&format!("setup_inactive:{}:{}",environment.id,class.unwrap_or("")), || {
        let inactive = match class {
            Some(class) => m.require_inactive(Some(class)),
            None => capacity::require_environment_inactive(m, &environment),
        };
        Ok(inactive.err().map(|error|format!("Close this setup's affected plug-in or application before changing it: {error}")))
    })
}
pub(super) fn apply_offer(m: &Manager, offer: &mut ui::AvailableAction) -> Result<()> {
    if offer.disabled_reason.is_none() { offer.disabled_reason = offer_refusal(m, &offer.action)?; }
    Ok(())
}
pub(super) fn apply_product(m: &Manager, product: &mut ui::Product) -> Result<()> {
    for offer in &mut product.actions { apply_offer(m, offer)?; }
    if let Some(workflow) = &mut product.compatibility {
        for offer in workflow.primary.iter_mut().chain(&mut workflow.alternatives) { apply_offer(m, offer)?; }
    }
    Ok(())
}
pub(super) fn apply_snapshot(m: &Manager, snapshot: &mut ui::Snapshot) -> Result<()> {
    for offer in snapshot.actions.iter_mut()
        .chain(snapshot.installer_setups.iter_mut().flat_map(|row| std::iter::once(&mut row.rename)
            .chain(row.primary.iter_mut()).chain(row.secondary.iter_mut())))
        .chain(snapshot.environment_installers.iter_mut().flat_map(|row| row.choices.iter_mut()
            .chain(&mut row.actions).chain(row.runtime.iter_mut().flat_map(|runtime|&mut runtime.choices))))
        .chain(snapshot.onboarding.iter_mut().flat_map(|row|&mut row.actions))
        .chain(snapshot.environments.iter_mut().flat_map(|row|&mut row.actions))
        .chain(snapshot.vendor_applications.iter_mut().flat_map(|row|&mut row.actions))
        .chain(snapshot.deleted_setups.iter_mut().flat_map(|row|&mut row.actions))
        .chain(snapshot.uninstallers.iter_mut().map(|row|&mut row.action)) {
        apply_offer(m, offer)?;
    }
    for product in &mut snapshot.products { apply_product(m, product)?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_installer_runtime_preserves_unrelated_views_but_refuses_execution() {
        for started in [false, true] {
            let (f, mut record) = onboarding::tests::close_policy_unstarted_fixture();
            let (f, candidate) = test_fixture::projection_candidate_in(f, false);
            preparation::record_candidate(&f.m, &candidate).unwrap();
            preparation::enable(&f.m, &candidate, false).unwrap();
            let operation = "cd".repeat(16);
            if started {
                record = onboarding::reserve(&f.m, &record.id, &operation).unwrap();
                atomic_json(&onboarding::directory(&f.m, &record.id).unwrap().join(format!("{operation}-result.json")),
                    &json!({"schema":2,"operation":operation,"state":"completed",
                        "cleanup_confirmed":true,"owned_live":0})).unwrap();
            }
            let _service = super::super::tests::idle_service(&f.m);
            readiness::with_unit_state_fixture(&[onboarding::unit(&operation).unwrap()], "inactive", || {
                let assert_readable = || {
                    let overview = current_overview(&f.m).unwrap();
                    assert!(overview.current.products.iter().any(|p|p.environment == f.r.environment.id));
                    let detail = product_detail(&f.m, &f.r.environment.id, &f.r.module.sha256,
                        &f.r.metadata.class_id).unwrap();
                    assert_eq!(detail.product.environment, f.r.environment.id);
                    let diagnostics = snapshot_idle_test(&f.m).unwrap();
                    assert!(diagnostics.products.iter().any(|p|p.environment == f.r.environment.id));
                };
                assert_readable();
                let input = &record.environment.runner.entry_point;
                fs::set_permissions(input, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(input, b"changed historical runtime").unwrap();
                fs::set_permissions(input, fs::Permissions::from_mode(0o400)).unwrap();
                assert_readable();
                let actions = if started {
                    vec![ui::Action::InstallerScan {onboarding:record.id.clone()}]
                } else { vec![ui::Action::InstallerStart {onboarding:record.id.clone()},
                    ui::Action::InstallerStartWithPolicy {onboarding:record.id.clone(),powershell:ui::Powershell::Inherited}] };
                for action in actions {
                    assert_eq!(environment(&f.m, &action).unwrap(), Some(record.environment.clone()));
                    let retained = ["onboarding", "environments", "inventory", "preparation", "installers"]
                        .into_iter().map(|name|f.m.root.join(name)).filter(|path|path.exists())
                        .map(|path| {let before=test_fixture::snapshot(&path); (path,before)}).collect::<Vec<_>>();
                    let registry = f.m.registry().unwrap();
                    let error = execute_with_receipt_policy(&f.m, &action, Some(&"ef".repeat(16)),
                        &idle_capacity_test, Duration::from_secs(1), &mut vec![]).unwrap_err();
                    assert_eq!(error.to_string(), "artifact missing or changed");
                    assert_eq!(serde_json::to_value(f.m.registry().unwrap()).unwrap(), serde_json::to_value(&registry).unwrap());
                    for (path,before) in retained {assert_eq!(test_fixture::snapshot(&path), before);}
                    assert!(!job_dir(&f.m, &"ef".repeat(16)).unwrap().exists(), "no reservation, scan or launch");
                }
                let mut invalid = record.clone(); invalid.environment.id = "00".repeat(16);
                atomic_json(&onboarding::directory(&f.m, &record.id).unwrap().join("record.json"), &invalid).unwrap();
                assert!(environment(&f.m, &ui::Action::InstallerScan {onboarding:record.id.clone()}).is_err());
            });
        }
    }

    fn request(m: &Manager, environment: &Environment) -> String {
        begin_request(m, &ui::Request { schema:ui::OPERATOR_SCHEMA,
            state_token:token(m).unwrap(),
            action:ui::Action::EnvironmentRescan {environment:environment.id.clone()} }).unwrap()
    }
    fn second_setup(f: &test_fixture::Fixture) -> Environment {
        let mut registration = f.r.clone();
        registration.metadata.class_id = "02".repeat(16);
        registration.environment.id = "22".repeat(16);
        registration.environment.root = f.m.root.join("environments").join(&registration.environment.id);
        private_dir(&registration.environment.root).unwrap();
        atomic_json(&registration.environment.root.join("environment.json"), &registration.environment).unwrap();
        let module = registration.environment.root.join("compatdata/pfx/drive_c/other.dll");
        private_dir(module.parent().unwrap()).unwrap();
        fs::copy(&registration.module.path, &module).unwrap();
        registration.module.path = module;
        f.m.register(registration.clone()).unwrap();
        registration.environment
    }
    fn retained_user(m: &Manager, environment: &Environment) -> PathBuf {
        let session = random_id().unwrap();
        let report = m.root.join("runtime/results").join(format!("windows-{session}.json"));
        private_dir(report.parent().unwrap()).unwrap();
        let lease = m.root.join("runtime/leases").join(format!("{session}.json"));
        let owner = environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&session).join("owner.json");
        private_dir(owner.parent().unwrap()).unwrap();
        atomic_json(&owner, &json!({"session":session,"report":report,"lease":lease,
            "keeper":false,"inspect":false,"vendor_access":false,
            "registration":{"environment":environment,"metadata":{"class_id":"02".repeat(16)}}})).unwrap();
        m.retain_lease_owner(&owner).unwrap();
        private_dir(lease.parent().unwrap()).unwrap();
        atomic_json(&lease, &report).unwrap();
        lease
    }
    #[test]
    fn settings_test_refuses_affected_live_users_and_preserves_unrelated_setup() {
        let (f, base) = preparation_cli::tests::projection_fixture();
        atomic_json(&f.m.root.join("software.json"), &preparation_cli::tests::projection_software(&base)).unwrap();
        linux_vst_bridge::preparation::record_candidate(&f.m, &base).unwrap();
        let baseline = linux_vst_bridge::preparation::enable(&f.m, &base, false).unwrap();
        let action = ui::Action::CandidateSettingsTest {candidate:base.id().unwrap(),
            settings:ui::LocalSettings {graphics:Some(ui::GraphicsBackend::WineD3d11),
                accessibility:ui::AccessibilityChoice::DisabledForHost},
            expected_current:Some(ui::PublicationIdentity {id:baseline.id,sha256:baseline.sha256})};
        let other = second_setup(&f);
        let sibling = retained_user(&f.m, &other);
        let sibling_before = fs::read(&sibling).unwrap();
        assert!(offer_refusal(&f.m, &action).unwrap().is_none());
        require_operator_inactive_with(&f.m, &action, &idle_capacity_test, None, Duration::from_secs(1), &mut vec![]).unwrap();
        let affected = retained_user(&f.m, &base.selection.environment);
        let affected_before = fs::read(&affected).unwrap();
        assert!(offer_refusal(&f.m, &action).unwrap().is_some());
        assert!(require_operator_inactive_with(&f.m, &action, &idle_capacity_test, None, Duration::from_secs(1), &mut vec![]).is_err());
        assert_eq!(fs::read(&affected).unwrap(), affected_before);
        assert_eq!(fs::read(&sibling).unwrap(), sibling_before);
    }

    #[test]
    fn maintenance_reserves_only_its_setup_and_preserves_unrelated_user() {
        let (f, _) = preparation_cli::tests::projection_fixture();
        f.m.register(f.r.clone()).unwrap();
        let other = second_setup(&f);
        let lease = retained_user(&f.m, &other);
        let before = fs::read(&lease).unwrap();
        let operation = request(&f.m, &f.r.environment);
        acquire(&f.m, &operation, None, Duration::from_secs(1), &mut vec![]).unwrap();
        assert!(blocks(&f.m, &f.r.environment).unwrap());
        assert!(!blocks(&f.m, &other).unwrap());
        assert!(!f.m.root.join("operator/resume.json").exists());
        let offer = ui::Action::EnvironmentRescan {environment:f.r.environment.id.clone()};
        assert!(offer_refusal(&f.m, &offer).unwrap().unwrap().contains("this setup"));
        assert_eq!(fs::read(&lease).unwrap(), before);
        assert!(release(&f.m, &operation).unwrap());
        assert!(!blocks(&f.m, &f.r.environment).unwrap());
        assert_eq!(fs::read(&lease).unwrap(), before);
        assert!(path(&f.m, &operation).unwrap().exists(), "retirement preserves its ownership record");
    }
    #[test]
    fn exact_request_change_refuses_reservation_read_without_erasing_it() {
        let (f, _) = preparation_cli::tests::projection_fixture();
        f.m.register(f.r.clone()).unwrap();
        let operation = request(&f.m, &f.r.environment);
        acquire(&f.m, &operation, None, Duration::from_secs(1), &mut vec![]).unwrap();
        let record = path(&f.m, &operation).unwrap();
        let before = fs::read(&record).unwrap();
        fs::write(job_dir(&f.m, &operation).unwrap().join("request.json"), b"{}").unwrap();
        assert!(read(&f.m, &operation).is_err());
        assert!(release(&f.m, &operation).is_err());
        assert_eq!(fs::read(record).unwrap(), before);
    }
    #[test]
    fn automatic_install_resolves_created_setup_and_reaches_real_installer_stage_with_unrelated_dsp() {
        for policy in [None, Some(ui::Powershell::Inherited)] {
            let (f, record) = onboarding::tests::close_policy_unstarted_fixture();
            let action = ui::Action::software_install_action(record.installer.clone(), ui::DawChoice::Reaper, policy);
            let operation = test_submit_offered(&f.m, &action).unwrap();
            let other = second_setup(&f);
            let lease = retained_user(&f.m, &other);
            let before = fs::read(&lease).unwrap();
            let error = readiness::with_unit_state_fixture(&[onboarding::unit(&operation).unwrap()], "inactive", ||
                onboarding::with_installer_unit_refusal(|| automatic_install::execute(&f.m, &action, &operation,
                    &idle_capacity_test, Duration::from_secs(1), &mut vec![]))).unwrap_err();
            assert!(error.to_string().contains("fixture_installer_unit_refusal"), "{error}");
            let current = onboarding::load(&f.m, &record.id).unwrap();
            assert_eq!(current.installation_operation.as_deref(), Some(operation.as_str()));
            assert_eq!(accepted_environment(&f.m, &operation, &action).unwrap(), current.environment);
            let mut wrong = current.clone();
            wrong.automatic.as_mut().unwrap().journey.daw = ui::DawChoice::Other;
            atomic_json(&onboarding::directory(&f.m, &record.id).unwrap().join("record.json"), &wrong).unwrap();
            assert!(accepted_environment(&f.m, &operation, &action).is_err());
            assert_eq!(fs::read(&lease).unwrap(), before);
            assert!(!blocks(&f.m, &other).unwrap());
            assert!(!blocks(&f.m, &record.environment).unwrap());
        }
    }
    #[test]
    fn terminal_reservation_recovery_preserves_live_or_unknown_executors() {
        let (f, _) = preparation_cli::tests::projection_fixture();
        f.m.register(f.r.clone()).unwrap();
        let operation = request(&f.m, &f.r.environment);
        acquire(&f.m, &operation, None, Duration::from_secs(1), &mut vec![]).unwrap();
        write_operation(&f.m, &operation, &json!({"schema":2,"operation":operation,"state":"completed"}), false).unwrap();
        let before = fs::read(path(&f.m, &operation).unwrap()).unwrap();
        assert!(reconcile(&f.m, |_| Ok(true)).unwrap().is_empty());
        assert!(reconcile(&f.m, |_| Err("fixture_unobservable".into())).unwrap().is_empty());
        assert_eq!(fs::read(path(&f.m, &operation).unwrap()).unwrap(), before);
        assert!(reconcile(&f.m, |_| Ok(false)).unwrap().is_empty());
        assert!(!blocks(&f.m, &f.r.environment).unwrap());
        assert!(path(&f.m, &operation).unwrap().is_file());
    }
    #[test]
    fn interrupted_publication_blocks_only_its_own_setup() {
        let (f, _) = preparation_cli::tests::projection_fixture();
        f.m.register(f.r.clone()).unwrap();
        let other = second_setup(&f);
        let pending = f.m.root.join("transactions").join(format!("{}.pending.json", "02".repeat(16)));
        private_dir(pending.parent().unwrap()).unwrap();
        fs::write(&pending, b"retained interrupted publication").unwrap();
        assert!(require_publications_settled(&f.m, &f.r.environment).is_ok());
        assert!(require_publications_settled(&f.m, &other).is_err());
        let action = ui::Action::EnvironmentRescan { environment: f.r.environment.id.clone() };
        assert!(offer_refusal(&f.m, &action).unwrap().is_none());
        require_operator_inactive_with(&f.m, &action, &idle_capacity_test,
            None, Duration::from_secs(1), &mut vec![]).unwrap();
        assert_eq!(fs::read(&pending).unwrap(), b"retained interrupted publication");
    }

}

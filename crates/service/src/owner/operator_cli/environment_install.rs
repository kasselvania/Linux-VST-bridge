//! Companion installation extends the existing operator transaction and installer supervisor.
//! The environment revision, preparation history and publication remain their existing owners.
use super::*;

use linux_vst_bridge::environment_revision::InstallerBinding as Binding;
const MISSING_INSTALLATION: &str = "The installation folder for this software can't be found. Reconnect its drive or restore the folder, then try again.";

fn initialized_prefix(environment: &Environment) -> bool {
    environment.root.join("compatdata/pfx").is_dir()
        && environment.root.join("compatdata/pfx/system.reg").is_file()
}
fn initialized_receipt(value: &Value) -> bool {
    value["startup"]["stages"].as_array().is_some_and(|stages| stages.iter()
        .any(|row| row["stage"] == "prefix_initialization_exit" && row["exit"] == 0))
}
pub(super) fn prefix_preservation_required(m: &Manager, environment: &Environment) -> Result<bool> {
    if !environment.root.is_dir() {return Ok(true);}
    if initialized_prefix(environment) || m.registry()?.classes.values()
        .any(|entry| entry.registration.environment.id == environment.id) {return Ok(true);}
    for record in onboarding::history_records(m)?.into_iter()
        .filter(|r| r.environment.id == environment.id) {
        // An attempt closed before its installer started ran nothing here.
        if record.installation_operation.is_some()
            && !onboarding::result(m,&record).is_ok_and(|v|onboarding::closed_unstarted(&v)) {
            // Old MF2 producers did not record startup stages. Their absence
            // cannot establish that a retained installation is uninitialized.
            return Ok(true);
        }
    }
    for record in records(m)?.into_iter().filter(|r| r.environment.id == environment.id) {
        if record.preserve_existing_prefix == Some(true) || initialized_receipt(&result(m,&record)?) {
            return Ok(true);
        }
    }
    for selector in vendor_cli::selectors(m)? {
        let app:vendor_application::Application=read_json(
            &vendor_cli::directory(m,&selector)?.join("application.json"))?;
        if app.environment.id == environment.id {return Ok(true);}
    }
    Ok(false)
}
fn require_preserved_prefix(m: &Manager, environment: &Environment, bound: Option<bool>) -> Result<bool> {
    // Omitted legacy fields use retained setup facts; absence is never evidence
    // that an installation was new. Reservation preserves a current observation
    // through the later worker launch without reading any vendor account data.
    let required=bound == Some(true) || prefix_preservation_required(m,environment)?;
    require(!required || initialized_prefix(environment),MISSING_INSTALLATION)?;
    Ok(required)
}
pub(crate) fn latest(m: &Manager, environment: &str) -> Result<Option<Binding>> {
    let history=records(m)?;
    let selected=history.into_iter().filter(|record|record.environment.id==environment)
        .max_by_key(|record|record.environment.revision);
    if let Some(record)=&selected {current_environment(m,&record.environment)?;}
    Ok(selected)
}
pub(crate) fn resolves_to(m: &Manager, before: &Environment, after: &Environment) -> Result<bool> {
    linux_vst_bridge::environment_revision::resolves_to(m, before, after)
}
pub(crate) fn current_environment(m: &Manager, bound: &Environment) -> Result<Environment> {
    linux_vst_bridge::environment_revision::current_environment(m, bound)
}
pub(super) fn load(m: &Manager, operation: &str) -> Result<Binding> {
    linux_vst_bridge::environment_revision::installer_record(m, operation)
}
pub(super) fn records(m: &Manager) -> Result<Vec<Binding>> {
    linux_vst_bridge::environment_revision::installer_records(m)
}
pub(crate) fn result(m: &Manager, r: &Binding) -> Result<Value> {
    let v = optional(&job_dir(m,&r.operation)?.join("installation-result.json"))?;
    require(v["schema"] == 2 && v["operation"] == r.operation,
        "installer_result_identity")?;
    onboarding::validate_installer_transaction(&v,&r.operation)?;
    Ok(v)
}

pub(crate) fn all_retired(m: &Manager) -> Result<bool> {
    for r in records(m)? {
        if !onboarding::retired(&result(m,&r)?) {return Ok(false);}
    }
    Ok(true)
}

pub(crate) fn target(m: &Manager, environment: &str) -> Result<Environment> {
    target_with_application(m,environment,registered_application_target)
}
/// Passive maintenance scope uses retained ownership, even when execution inputs
/// changed. Mutation owners still use `target` and verify their execution assets.
pub(crate) fn target_record(m: &Manager, environment: &str) -> Result<Environment> {
    m.derive_once(&format!("maintenance_target:{environment}"), ||
        target_from_registry(m,environment,m.composed_registry()?.as_ref(),|m,id|
            Ok(registered_application_record(m,id)?.map(|app|app.environment))))
}
fn registered_application_record(m: &Manager, environment: &str) -> Result<Option<vendor_application::Application>> {
    for selector in vendor_cli::selectors(m)? {
        let original:vendor_application::Application=read_json(
            &vendor_cli::directory(m,&selector)?.join("application.json"))?;
        if original.environment.id != environment {continue;}
        let app=vendor_cli::application(m,&selector)?;
        app.validate_record(&m.root)?;
        return Ok(Some(app));
    }
    Ok(None)
}
fn registered_application_target(m: &Manager, environment: &str) -> Result<Option<Environment>> {
    let app=registered_application_record(m,environment)?;
    if let Some(app)=&app {app.verify(&m.root)?;}
    Ok(app.map(|app|app.environment))
}
fn target_with_application(m: &Manager, environment: &str,
    verified:impl FnOnce(&Manager,&str)->Result<Option<Environment>>) -> Result<Environment> {
    target_from_registry(m,environment,&m.registry()?,verified)
}
fn target_from_registry(m: &Manager, environment: &str, db: &Registry,
    verified:impl FnOnce(&Manager,&str)->Result<Option<Environment>>) -> Result<Environment> {
    if let Some(r) = onboarding::retained_environment_record(m,environment)? {
        require(r.installation_operation.is_none()
            || onboarding::retired(&onboarding::result(m,&r)?),
            "installer_initial_retirement_required")?;
        let current=current_environment(m,&r.environment)?;
        for entry in db.classes.values().filter(|e|e.registration.environment.id == environment) {
            require(resolves_to(m,&entry.registration.environment,&current)?,
                "installer_environment_binding_changed")?;
        }
        return Ok(current);
    }
    let mut owners = db.classes.values().filter(|e|e.registration.environment.id == environment);
    if let Some(first)=owners.next() {
        let current=current_environment(m,&first.registration.environment)?;
        for entry in owners {
            require(resolves_to(m,&entry.registration.environment,&current)?,
                "installer_environment_binding_changed")?;
        }
        return Ok(current);
    }
    let current=verified(m,environment)?.ok_or("installer_existing_setup_required")?;
    require(current.id == environment && current_environment(m,&current)? == current,
        "installer_environment_binding_changed")?;
    Ok(current)
}

pub(super) fn reserve(m: &Manager, environment: &str, installer: &str,
    operation: &str) -> Result<Binding> {
    reserve_with(m,environment,installer,operation,
        || vendor_maintenance::observe_retired(m,&target(m,environment)?).map(|_|true),onboarding::live)
}
fn reserve_with(m: &Manager, environment: &str, installer: &str,
    operation: &str, vendor_retirement: impl FnOnce()->Result<bool>,
    mut is_live: impl FnMut(&str)->Result<bool>) -> Result<Binding> {
    require(valid_product_environment(environment),"installer_environment_identity")?;
    // A disconnected setup must retain its identity. Check this before any
    // historical readback can replace the useful refusal with a missing-file IO
    // error; the normal ownership owners still perform all positive admission.
    require(m.root.join("environments").join(environment).is_dir(),MISSING_INSTALLATION)?;
    let _ownership = m.lock("onboarding.lock")?;
    let _registry = m.lock("registry.lock")?;
    let before = target(m,environment)?;
    capacity::require_environment_inactive(m,&before)?;
    require(vendor_retirement()?, "operator_installer_active")?;
    // A final receipt may precede the supervisor unit's exit. Both its owned
    // resources and its exact process owner must retire before another launch.
    for owner in onboarding::history_records(m)?.into_iter().filter(|record| record.environment.id == environment)
        .filter_map(|r|r.installation_operation)
        .chain(records(m)?.into_iter().filter(|record| record.environment.id == environment).map(|r|r.operation)) {
        require(!is_live(&owner)?, "operator_installer_active")?;
    }
    environment_runtime::require_settled(m,environment)?;
    let preserve_existing_prefix = require_preserved_prefix(m,&before,None)?;
    drop(_registry);
    let presets=presets::protect_environment(m,&before,operation)?;
    let automatic_request = environment_refresh::capture_request(m,&before)?;
    before.runner.verify()?;
    installer_import::load(m,installer)?;
    let _registry=m.lock("registry.lock")?;
    presets.recheck(m)?;
    capacity::require_environment_inactive(m,&before)?;
    require(target_record(m,environment)?==before,"installer_environment_binding_changed")?;
    let mut after = before.clone();
    after.revision = after.revision.checked_add(1).ok_or("environment_revision_exhausted")?;
    let r = Binding {schema:1,operation:operation.into(),installer:installer.into(),
        before,environment:after,created_at:std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?.as_micros().try_into()?,
        preserve_existing_prefix:Some(preserve_existing_prefix),automatic_request};
    let dir = job_dir(m,operation)?;
    require(!dir.join("installation.json").try_exists()?, "installer_transaction_already_used")?;
    atomic_json(&dir.join("installation-result.json"),
        &json!({"schema":2,"operation":operation,"state":"reserved",
            "cleanup_confirmed":false,"owned_live":null}))?;
    atomic_json(&dir.join("installation.json"),&r)?;
    // Invalidate old registration/inspection authority before a vendor can mutate
    // shared state. No old publication or vendor data is rewritten or deleted.
    atomic_json(&r.environment.root.join("environment.json"),&r.environment)?;
    Ok(r)
}

pub(super) fn launch(m: &Manager, r: &Binding) -> Result<()> {
    let dir = job_dir(m,&r.operation)?;
    let report = dir.join("installation-result.json");
    let spec = dir.join("installer-spec.json");
    let input = launch_input(m,r)?;
    atomic_json(&spec,&input)?;
    onboarding::launch_spec(m,&r.operation,&spec,&report)
}
fn accepted_action(m: &Manager, r: &Binding) -> Result<ui::Action> {
    let (owned,action)=environment_revision::installer_input(m,&r.operation)?;
    // Only an omitted legacy preservation observation may differ. Execution
    // still applies the existing preservation owner to the retained prefix.
    require(owned.schema==r.schema && owned.operation==r.operation && owned.installer==r.installer
        && owned.before==r.before && owned.environment==r.environment && owned.created_at==r.created_at
        && owned.automatic_request==r.automatic_request
        && (owned.preserve_existing_prefix==r.preserve_existing_prefix
            || owned.preserve_existing_prefix.is_none() || r.preserve_existing_prefix.is_none()),
        "installer_transaction_binding")?;
    Ok(action)
}
pub(super) fn launch_input(m: &Manager, r: &Binding) -> Result<Value> {
    let action=accepted_action(m,r)?;
    let policy=action.environment_installer_start().ok_or("installer_transaction_binding")?.2;
    let preserve_existing_prefix = require_preserved_prefix(m,&r.environment,r.preserve_existing_prefix)?;
    require(target(m,&r.environment.id)? == r.environment,
        "installer_environment_binding_changed")?;
    let sw = software(m)?;
    let installer = installer_import::load(m,&r.installer)?;
    let dir = job_dir(m,&r.operation)?;
    let report = dir.join("installation-result.json");
    let mut launch_spec = json!({"schema":2,"operation":r.operation,
        "environment":r.environment,"installer":installer.artifact,"format":installer.format,
        "installer_launch":sw.installer_launch,"report":report,
        "preserve_existing_prefix":preserve_existing_prefix});
    linux_vst_bridge::installer_policy::bind_for_launch(m, &mut launch_spec, &sw, policy)?;
    launch_spec["managed_first_launch"] = linux_vst_bridge::vendor_profile::installer_first_launch()?;
    Ok(launch_spec)
}

pub(super) fn control(m: &Manager, operation: &str) -> Result<Binding> {
    // Resource custody survives missing installer bytes, runtime changes and stale publications.
    load(m,operation)
}

pub(super) fn focus(m: &Manager, operation: &str) -> Result<Value> {
    let _ownership = m.lock("onboarding.lock")?;
    let r = control(m,operation)?;
    require(onboarding::live(operation)?,"installer_focus_owner")?;
    let request = random_id()?;
    atomic_json(&job_dir(m,operation)?.join(format!("{operation}-focus.json")),
        &json!({"operation":operation,"request":request}))?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let v = result(m,&r)?;
        if v["focus_result"]["request"] == request {
            require(v["focus_result"]["result"] == "focused","installer_focus_refused")?;
            return Ok(v["focus_result"].clone());
        }
        require(Instant::now() < deadline,"installer_focus_timeout")?;
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub(super) fn stop(m: &Manager, operation: &str) -> Result<Value> {
    let _ownership = m.lock("onboarding.lock")?;
    let r = control(m,operation)?;
    require(Command::new("systemctl").args(["--user","stop",&onboarding::unit(operation)?])
        .status()?.success(),"installer_stop_failed")?;
    let v = result(m,&r)?;
    require(!onboarding::live(operation)? && onboarding::retired(&v),"installer_cleanup_unconfirmed")?;
    Ok(v)
}

pub(super) fn finish(m: &Manager, operation: &str) -> Result<()> {
    finish_with(m,operation,onboarding::live)
}

pub(crate) const CLOSE_CHECK: &str = "Check that it closed";
/// "Check that it closed", for every kind of installer or uninstaller run
/// whose closing was never confirmed (the manager or the computer stopped
/// meanwhile). When its unit is gone and no program runs from its setup, its
/// result is recorded as closed. Nothing is run or stopped.
pub(crate) fn confirm_closed(m: &Manager, operation: &str,
    is_live: &dyn Fn(&str) -> Result<bool>) -> Result<Value> {
    let _ownership = m.lock("onboarding.lock")?;
    let (path, root) = if uninstall::owns(m,operation)? {
        (job_dir(m,operation)?.join("uninstall-result.json"), uninstall::owner(m,operation)?.environment.root)
    } else if job_dir(m,operation)?.join("installation.json").try_exists()? {
        (job_dir(m,operation)?.join("installation-result.json"), control(m,operation)?.environment.root)
    } else if let Some(record) = onboarding::history_records(m)?.into_iter()
        .find(|r| r.installation_operation.as_deref() == Some(operation)) {
        (onboarding::directory(m,&record.id)?.join(format!("{operation}-result.json")), record.environment.root)
    } else {
        return Err("This installer run can't be found. Refresh and try again.".into());
    };
    let mut v = optional(&path)?;
    if onboarding::retired(&v) { return Ok(v); }
    require(!is_live(operation)?, "It is still running. Use Stop instead.")?;
    let resolved = root.canonicalize().unwrap_or_else(|_| root.clone());
    let running = setup_removal::running_inside(&[&root, &resolved])?;
    if let Some(program) = running.first() {
        return Err(format!("{program} is still running from this setup. Close it, then check again.").into());
    }
    closed(&mut v);
    v["schema"] = 2.into();
    v["operation"] = operation.into();
    v["cleanup_confirmed"] = true.into();
    v["owned_live"] = 0.into();
    v["closing_confirmed_by"] = "census".into();
    atomic_json(&path,&v)?;
    Ok(v)
}
/// What the supervisor would have recorded had it seen the run close: a
/// program that exited 0 without an error keeps its installation outcome;
/// anything else that never finished is a failure.
fn closed(v: &mut Value) {
    if matches!(v["state"].as_str(), Some("completed" | "failed" | "cancelled")) { return; }
    let exited = v["state"] == "cleanup_unconfirmed" && v["raw_exit"] == 0 && v["error"].is_null()
        && (v["transaction"]["launch_binding"]["completion_contract"] != "retained_handle_status_v1"
            || v["transaction"]["root_completion"]["status"] == "succeeded")
        && !matches!(v["transaction"]["outcome"].as_str(), Some("cancelled" | "child_failed"
            | "outer_nonzero_stage_unknown" | "installed_dependency_failed"));
    v["state"] = if exited { "completed" } else { "failed" }.into();
    if exited && v["transaction"]["outcome"] == "cleanup_unconfirmed" {
        let outcome = v["transaction"]["durable_installation"].as_str()
            .filter(|durable| matches!(*durable, "installed" | "partial_installation" | "not_installed"))
            .unwrap_or("completed").to_owned();
        v["transaction"]["outcome"] = outcome.into();
    }
}
fn finish_with(m: &Manager, operation: &str, is_live: impl FnOnce(&str)->Result<bool>) -> Result<()> {
    if !job_dir(m,operation)?.join("installation.json").try_exists()? {return Ok(());}
    let r = control(m,operation)?;
    if is_live(operation)? {let _ = stop(m,operation);}
    let mut v = result(m,&r)?;
    if !onboarding::retired(&v) {
        // If a worker died before launch, the current exact unit proves no vendor
        // process was started. Later missing retirement remains an ownership gap.
        if v["state"] == "reserved" && !job_dir(m,operation)?.join("installer-spec.json").try_exists()? {
            v["state"] = "failed".into();v["cleanup_confirmed"] = true.into();v["owned_live"] = 0.into();
            v["error"] = "installer_not_launched_rescan_required".into();
        } else {
            v["state"] = "cleanup_unconfirmed".into();
            v["error"] = "installer_supervisor_terminated_without_retirement".into();
        }
        atomic_json(&job_dir(m,operation)?.join("installation-result.json"),&v)?;
    }
    Ok(())
}

pub(super) fn projection(m: &Manager, products: &[ui::Product], busy: Option<&str>,
    mut is_live: impl FnMut(&str)->Result<bool>) -> Result<Vec<ui::EnvironmentInstaller>> {
    let db = m.registry()?;
    let history = records(m)?;
    let installers = installer_import::list_identity_records(m)?;
    let mut targets = std::collections::BTreeMap::new();
    let mut setup_names=std::collections::BTreeMap::new();
    for r in onboarding::history_records(m)? {
        if r.installation_operation.is_none()
            || (onboarding::retired(&onboarding::result(m,&r)?)
                && !is_live(r.installation_operation.as_ref().unwrap())?) {
            if let Some(installer)=installers.iter().find(|i|i.id==r.installer) {
                setup_names.insert(r.environment.id.clone(),installer_import::presentation(m,installer)?.display_label);
            }
            targets.insert(r.environment.id.clone(),r.environment);
        }
    }
    for entry in db.classes.values() {
        let env = current_environment(m,&entry.registration.environment)?;
        targets.insert(env.id.clone(),env);
    }
    let mut out = vec![];
    for (id,env) in targets {
        let env = current_environment(m,&env)?;
        let affected:Vec<_> = products.iter().filter(|p|p.environment == id)
            .map(|p|p.name.clone()).collect();
        let name = if affected.is_empty() {format!("{} · existing setup {}",setup_names.get(&id)
                .map(String::as_str).unwrap_or("Plug-in"),id.chars().take(8).collect::<String>())}
            else {affected.join(", ")};
        // Recovery follows unresolved custody, never wall-clock ordering.
        let mut unresolved=history.iter().filter(|r|r.environment.id == id)
            .filter_map(|r|match result(m,r) {
                Ok(value)=>match is_live(&r.operation) {
                    Ok(live) if live || !onboarding::retired(&value)=>Some(Ok(r)),
                    Ok(_)=>None,Err(error)=>Some(Err(error)),
                },Err(error)=>Some(Err(error)),
            });
        let active=unresolved.next().transpose()?;
        require(unresolved.next().is_none(),"installer_environment_ownership_conflict")?;
        let last = active.or_else(||history.iter().rev().find(|r|r.environment.id == id));
        let result = last.map(|r|result(m,r)).transpose()?.unwrap_or(Value::Null);
        let mut actions = vec![];
        let runtime_unavailable=env.runner.validate_record().err();
        let new_work_reason=busy.or(runtime_unavailable.as_ref().map(|_|
            "This setup's pinned compatibility runtime is unavailable. Restore its original files before starting new work."));
        let pending_runtime=environment_runtime::pending(m,&id)?;
        let mut reason = if pending_runtime {
            actions.push(action("Finish interrupted runtime change",ui::Action::TransactionReconcile {},new_work_reason));
            Some("Finish the interrupted runtime change before running another installer in this setup.".into())
        } else {new_work_reason.map(str::to_owned)};
        // An uninstaller run that is still open, or never shown closed, keeps
        // the bridge paused; its own controls lead this card.
        if let Some(operation) = uninstall::unresolved(m,&id)? {
            let live = is_live(&operation)?;
            reason = Some(if live { "An uninstaller is running in this setup. Follow its window to finish, or stop it.".into() }
                else { uninstall::UNCONFIRMED_CLOSE.into() });
            if live {
                actions.push(action("Focus uninstaller",ui::Action::EnvironmentInstallerFocus {
                    operation:operation.clone()},None));
                actions.push(action("Stop uninstaller",ui::Action::EnvironmentInstallerStop {operation},None));
            } else {
                actions.push(action(CLOSE_CHECK,ui::Action::InstallerCloseCheck {operation},None));
            }
        } else if let Some(operation) = uninstall::unchecked(m,&id)? {
            // It closed, but its setup's plug-ins weren't all checked again.
            // Installers and runtime changes stay available (they check them
            // again too); only another uninstall waits. While the uninstall's
            // own action runs, that action is the busy reason. Once it ended,
            // a pause it still holds is what Finish checking takes over.
            let ended = matches!(optional(&job_dir(m,&operation)?.join("result.json"))?["state"].as_str(),
                Some("completed" | "refused"));
            let own_pause = ended && reservation_owner(m, &operation)?.is_some();
            actions.push(action(uninstall::FINISH_CHECKING,ui::Action::InstallerCloseCheck {operation},
                if own_pause { None } else { busy }));
        }
        if let Some(r) = last {
            if active.is_some() {
                reason = Some("Finish or stop this installer; its cleanup must be confirmed".into());
                if is_live(&r.operation)? {
                    actions.push(action("Focus installer",ui::Action::EnvironmentInstallerFocus {
                        operation:r.operation.clone()},None));
                    actions.push(action("Stop installer",ui::Action::EnvironmentInstallerStop {
                        operation:r.operation.clone()},None));
                } else {
                    reason = Some("The manager couldn't confirm that this installer closed. Use Check that it closed.".into());
                    actions.push(action(CLOSE_CHECK,ui::Action::InstallerCloseCheck {operation:r.operation.clone()},None));
                }
            }
            if active.is_none() && onboarding::retired(&result) && result["state"] != "completed" {
                let unavailable=installer_import::load_record(m,&r.installer).err();
                actions.push(action("Retry",accepted_action(m,r)?,reason.as_deref().or(unavailable.as_ref()
                    .map(|_|"Imported installer file is unavailable. Import the original file again."))));
            }
        }
        if active.is_none() && onboarding::inventory_refresh_record_required(m,&env,
            &software_record(m)?.host,&software_record(m)?.source_sha256)? {
            actions.push(action("Rescan and continue preparation",ui::Action::EnvironmentInstallerScan {
                environment:id.clone()},reason.as_deref()));
        }
        let choices = installers.iter().map(|i| {
            let label = installer_import::presentation(m,i)?.display_label;
            let missing=installer_import::load_record(m,&i.id).err();
            Ok(action(&format!("Run {label} in this setup"),ui::Action::environment_installer_action(
                id.clone(),i.id.clone(),None),reason.as_deref().or(missing.as_ref()
                    .map(|_|"Imported installer file is unavailable. Import the original file again."))))
        }).collect::<Result<Vec<_>>>()?;
        let selected_software = software_record(m)?;
        for installer in &installers {
            if !linux_vst_bridge::installer_policy::can_resolve_adapter_record(&installer.format, &selected_software) { continue; }
            let label = installer_import::presentation(m,installer)?.display_label;
            let missing = installer_import::load_record(m,&installer.id).err();
            let disabled = reason.as_deref().or(missing.as_ref().map(|_|"Imported installer file is unavailable. Import the original file again."));
            for (description, powershell) in [
                ("normal PowerShell behavior", linux_vst_bridge::installer_policy::Powershell::Inherited),
                ("PowerShell intentionally unavailable", linux_vst_bridge::installer_policy::Powershell::IntentionallyUnavailable),
            ] {
                actions.push(action(&format!("Run {label} with {description}"), ui::Action::environment_installer_action(
                    id.clone(),installer.id.clone(),Some(powershell)), disabled));
            }
        }
        let mut runtime=environment_runtime::projection(m,&env,reason.as_deref())?;
        // Discovered plug-ins can belong to the owned setup before their first
        // publication. Name them in the same pre-execution consequence panel.
        for name in &affected {
            if !runtime.affected.contains(name) {runtime.affected.push(name.clone());}
        }
        let runtime=Some(runtime);
        actions.push(action("Choose program",ui::Action::ProgramChoose {
            environment:id.clone(),expected_environment:environment_runtime::identity(&env)?,
            executable:ui::ProgramExecutable {path:PathBuf::new(),sha256:String::new()}},reason.as_deref()));
        let dependency_help=plugin_services::assistance(m,&env,&selected_software.host,&selected_software.source_sha256)?;
        out.push(ui::EnvironmentInstaller {environment:id,name,affected,runtime,dependency_help,
            consequence:"This installer can change every plug-in in this compatibility space, vendor applications, authorization and shared dependencies. Stopping ends owned processes; it cannot undo vendor changes. Existing plug-ins need a fresh scan and preparation before use. The environment identity is preserved; vendor state is not copied or rolled back.".into(),
            choices,operation:last.map(|r|r.operation.clone()),installer:last.map(|r|r.installer.clone()),result,actions});
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_application_inputs_preserve_scope_but_not_execution_authority() {
        let (f, _) = preparation_cli::tests::projection_fixture();
        assert!(f.m.registry().unwrap().classes.is_empty());
        assert!(onboarding::history_records(&f.m).unwrap().is_empty());
        let image=f.r.environment.root.join("chosen.exe");
        fs::write(&image,b"first-party selected program fixture").unwrap();
        let app=vendor_application::Application {schema:3,id:vendor_application::ApplicationId::Selected,
            environment:f.r.environment.clone(),executable:Artifact {sha256:digest(&image).unwrap(),path:image.clone()},
            helpers:vec![],installer_sha256:None,installer_result:None,observed_installer_version:String::new(),
            user_selection:Some(vendor_application::ProgramSelection {working_directory:f.r.environment.root.clone(),
                arguments:vec![],previous:None})};
        let selector=app.selector().unwrap();
        let record=vendor_cli::directory(&f.m,&selector).unwrap().join("application.json");
        private_dir(record.parent().unwrap()).unwrap();
        atomic_json(&record,&app).unwrap();
        assert_eq!(target(&f.m,&app.environment.id).unwrap(),app.environment);
        let original=fs::read(&record).unwrap();
        for path in [&image,&app.environment.runner.entry_point] {
            let bytes=fs::read(path).unwrap();
            fs::write(path,b"changed input").unwrap();
            assert_eq!(target_record(&f.m,&app.environment.id).unwrap(),app.environment);
            assert_eq!(maintenance::environment(&f.m,&ui::Action::EnvironmentRescan {
                environment:app.environment.id.clone()}).unwrap(),Some(app.environment.clone()));
            assert_eq!(target(&f.m,&app.environment.id).unwrap_err().to_string(),"artifact missing or changed");
            assert_eq!(fs::read(&record).unwrap(),original);
            fs::write(path,bytes).unwrap();
        }
        let mut invalid=app.clone();invalid.executable.sha256="00".repeat(32);
        atomic_json(&record,&invalid).unwrap();
        assert!(target_record(&f.m,&app.environment.id).is_err(),"exact selector still binds its record");
    }
    #[test]
    fn verified_application_owns_a_target_without_onboarding_or_published_classes() {
        let f=test_fixture::Fixture::new();
        assert!(f.m.registry().unwrap().classes.is_empty());
        assert!(onboarding::history_records(&f.m).unwrap().is_empty());
        let environment=f.r.environment.clone();
        assert_eq!(target_with_application(&f.m,&environment.id,|_,_|Ok(Some(environment.clone()))).unwrap(),environment);
        assert_eq!(target_with_application(&f.m,&environment.id,|_,_|Err("application_content_changed".into()))
            .unwrap_err().to_string(),"application_content_changed");
        assert_eq!(target_with_application(&f.m,&environment.id,|_,_|Ok(None))
            .unwrap_err().to_string(),"installer_existing_setup_required");
        let mut foreign=environment.clone();foreign.id=random_id().unwrap();
        assert_eq!(target_with_application(&f.m,&environment.id,|_,_|Ok(Some(foreign)))
            .unwrap_err().to_string(),"installer_environment_binding_changed");
    }
    #[test]
    fn valid_registry_owner_does_not_acquire_an_unrelated_app_verification_requirement() {
        let f=test_fixture::Fixture::new();let environment=f.r.environment.clone();
        assert_eq!(target_with_application(&f.m,&environment.id,|_,_|Err("application_content_changed".into()))
            .unwrap_err().to_string(),"application_content_changed");
        f.m.register(f.r.clone()).unwrap();
        assert_eq!(target_with_application(&f.m,&environment.id,|_,_|panic!("Registry custody already owns this target"))
            .unwrap(),environment);
    }
    fn fixture() -> (test_fixture::Fixture,installer_import::Installer) {
        fixture_with_catalogue(false)
    }
    fn fixture_with_catalogue(with_catalogue:bool) -> (test_fixture::Fixture,installer_import::Installer) {
        let (f,profile,_,native) = test_fixture::prepared();
        let mut bytes=vec![0;1024];
        bytes[..2].copy_from_slice(b"MZ");bytes[60]=128;
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148]=2;bytes[150]=2;bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let path=f.outer.join("companion.exe");fs::write(&path,bytes).unwrap();
        let installer=installer_import::import(&f.m,file(&path).unwrap()).unwrap();
        let a=f.r.host.clone();
        let mut sw=Software {manager:a.clone(),supervisor:a.clone(),ownership:a.clone(),host:a,
            source_manifest:Artifact {path:f.r.host.path.with_file_name("host-source-manifest.json"),
                sha256:f.r.host_source_sha256.clone()},source_sha256:f.r.host_source_sha256.clone(),
                operator_frontend:None,native_catalogue:None,preparation_kit:None,installer_launch:None};
        if with_catalogue {
            let path=sw.manager.path.with_file_name("catalogue.json");
            atomic_json(&path,&catalogue::Catalogue {schema:3,natives:vec![native],hosts:vec![],onboarding_runtime:None,
                environments:vec![catalogue::EnvironmentBinding {family:profile.requirements.environment_family,
                    environment:f.r.environment.clone()}]}).unwrap();
            sw.native_catalogue=Some(Artifact {sha256:digest(&path).unwrap(),path});
        }
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();
        fs::write(f.r.environment.root.join("compatdata/pfx/system.reg"),b"first-party registry fixture").unwrap();
        (f,installer)
    }
    fn job(m:&Manager,environment:&str,installer:&str)->String {
        let op=random_id().unwrap();let dir=job_dir(m,&op).unwrap();private_dir(&dir).unwrap();
        atomic_json(&dir.join("request.json"),&ui::Request {schema:ui::OPERATOR_SCHEMA,
            state_token:"fixture".into(),action:ui::Action::EnvironmentInstallerStart {
                environment:environment.into(),installer:installer.into()}}).unwrap();op
    }
    fn complete(m:&Manager,r:&Binding,state:&str) {
        atomic_json(&job_dir(m,&r.operation).unwrap().join("installation-result.json"),
            &json!({"schema":2,"operation":r.operation,"state":state,
                "cleanup_confirmed":true,"owned_live":0})).unwrap();
    }
    fn wrong_installer_setup(f: &test_fixture::Fixture, installer: &installer_import::Installer,
        state: Option<&str>) -> onboarding::Record {
        let mut environment = f.r.environment.clone();
        environment.id = "cd".repeat(16);
        environment.root = f.m.root.join("environments").join(&environment.id);
        private_dir(&environment.root).unwrap();
        atomic_json(&environment.root.join("environment.json"), &environment).unwrap();
        if state.is_some() {
            private_dir(&environment.root.join("compatdata/pfx")).unwrap();
            fs::write(environment.root.join("compatdata/pfx/system.reg"),b"first-party registry fixture").unwrap();
        }
        let record = onboarding::Record {schema:1,id:environment.id.clone(),environment,
            installer:installer.id.clone(),created_at:1,creation_operation:"12".repeat(16),
            installation_operation:state.map(|_|"ab".repeat(16)),published:false,
            automatic:None,previous_attempt:None};
        let dir = onboarding::directory(&f.m,&record.id).unwrap();
        private_dir(&dir).unwrap();
        atomic_json(&dir.join("record.json"),&record).unwrap();
        if let Some(state) = state {
            let op = record.installation_operation.as_ref().unwrap();
            atomic_json(&dir.join(format!("{op}-result.json")),&json!({"schema":2,
                "operation":op,"state":state,"cleanup_confirmed":true,"owned_live":0,
                "startup":{"stages":[{"stage":"prefix_initialization_exit","exit":0}]}})).unwrap();
        }
        record
    }
    fn different_installer(f: &test_fixture::Fixture, installer: &installer_import::Installer)
        -> installer_import::Installer {
        let mut bytes = fs::read(&installer.artifact.path).unwrap();
        bytes[900] = 42;
        // Deliberately the same basename: only content distinguishes this file.
        let path = f.outer.join("other").join("companion.exe");
        private_dir(path.parent().unwrap()).unwrap();
        fs::write(&path,bytes).unwrap();
        installer_import::import(&f.m,file(&path).unwrap()).unwrap()
    }
    #[test]
    fn completed_setup_repair_reuses_exact_installer_and_refuses_changed_bytes_before_mutation() {
        let (f, installer) = fixture();
        let original = wrong_installer_setup(&f,&installer,Some("completed"));
        let rows = projection(&f.m,&[],None,|_|Ok(false)).unwrap();
        let row = rows.iter().find(|row|row.environment == original.id).unwrap();
        let offered = row.choices.iter().find(|offer|offer.action.environment_installer_start()
            .is_some_and(|(env,id,policy)|env == original.id && id == installer.id && policy.is_none())).unwrap();
        assert!(offered.disabled_reason.is_none(),"the original file can run again after completion");
        let environment_before = fs::read(original.environment.root.join("environment.json")).unwrap();
        let unrelated = fs::read(f.r.environment.root.join("environment.json")).unwrap();
        let history = fs::read(onboarding::directory(&f.m,&original.id).unwrap().join("record.json")).unwrap();
        let registry = fs::read(f.m.root.join("registry.json")).unwrap();
        let bytes = fs::read(&installer.artifact.path).unwrap();
        let replace_fixture_bytes = |bytes: &[u8]| {
            // Imported files are sealed read-only. Deliberately corrupt this
            // disposable fixture, then restore its mode before admission.
            let mode = fs::metadata(&installer.artifact.path).unwrap().permissions();
            fs::set_permissions(&installer.artifact.path,fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(&installer.artifact.path,bytes).unwrap();
            fs::set_permissions(&installer.artifact.path,mode).unwrap();
        };
        for unknown in [true,false] {
            let selected = if unknown {"ff".repeat(32)} else {installer.id.clone()};
            if !unknown {replace_fixture_bytes(b"changed imported installer");}
            let op = job(&f.m,&original.id,&selected);
            assert!(reserve_with(&f.m,&original.id,&selected,&op,||Ok(true),|_|Ok(false)).is_err());
            assert!(!job_dir(&f.m,&op).unwrap().join("installation.json").exists());
            assert_eq!(fs::read(original.environment.root.join("environment.json")).unwrap(),environment_before);
            assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),registry);
        }
        replace_fixture_bytes(&bytes);
        let before = preconditions::observe(&f.m,&offered.action).unwrap().unwrap();
        let op = job(&f.m,&original.id,&installer.id);
        let bound = reserve_with(&f.m,&original.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        assert_eq!(bound.installer,installer.id);
        assert_eq!(bound.environment.id,original.environment.id);
        assert_eq!(bound.environment.root,original.environment.root);
        assert_eq!(bound.before,original.environment);
        assert_eq!(bound.preserve_existing_prefix,Some(true));
        let after = preconditions::observe(&f.m,&offered.action).unwrap().unwrap();
        assert!(preconditions::require_same(&before,&after).is_err(),"an earlier setup revision cannot authorize a later repair");
        assert_eq!(fs::read(f.r.environment.root.join("environment.json")).unwrap(),unrelated);
        assert_eq!(fs::read(onboarding::directory(&f.m,&original.id).unwrap().join("record.json")).unwrap(),history);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),registry);
    }
    #[test]
    fn wrong_installer_replacement_preserves_setup_before_launch_after_cancel_and_empty_discovery() {
        for state in [None,Some("cancelled"),Some("completed")] {
            let (f,wrong) = fixture();
            let initial = wrong_installer_setup(&f,&wrong,state);
            let right = different_installer(&f,&wrong);
            assert_ne!(right.id,wrong.id);
            let dir = onboarding::directory(&f.m,&initial.id).unwrap();
            let history = fs::read(dir.join("record.json")).unwrap();
            let vendor_state = initial.environment.root.join("fixture-state");
            fs::write(&vendor_state,b"first-party retained state").unwrap();
            let unrelated = fs::read(f.r.environment.root.join("environment.json")).unwrap();
            let registry = fs::read(f.m.root.join("registry.json")).unwrap();
            if state == Some("completed") {
                private_dir(&f.m.root.join("inventory")).unwrap();
                let scan = inventory::Scan {schema:1,id:random_id().unwrap(),
                    environment:initial.environment.clone(),host:f.r.host.clone(),
                    host_source_sha256:f.r.host_source_sha256.clone(),completed_at:1,
                    modules:vec![],changes:Default::default()};
                atomic_json(&f.m.root.join("inventory").join(format!("{}.json",initial.id)),&scan).unwrap();
            }
            let rows = projection(&f.m,&[],None,|_|Ok(false)).unwrap();
            let target = rows.iter().find(|r|r.environment == initial.id).unwrap();
            let choice = target.choices.iter().find(|a|matches!(&a.action,
                ui::Action::EnvironmentInstallerStart {environment,installer}
                    if environment == &initial.id && installer == &right.id)).unwrap();
            assert!(choice.disabled_reason.is_none());
            let op = job(&f.m,&initial.id,&right.id);
            let r = reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),|_|Ok(false)).unwrap();
            assert_eq!(r.installer,right.id);
            assert_eq!(r.before,initial.environment);
            assert_eq!(r.environment.id,initial.id);
            assert_eq!(r.environment.root,initial.environment.root);
            assert_eq!(r.environment.runner,initial.environment.runner);
            assert_eq!(r.environment.revision,initial.environment.revision+1);
            assert_eq!(fs::read(dir.join("record.json")).unwrap(),history);
            assert_eq!(fs::read(vendor_state).unwrap(),b"first-party retained state");
            assert_eq!(fs::read(f.r.environment.root.join("environment.json")).unwrap(),unrelated);
            assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),registry);
        }
    }
    #[test]
    fn wrong_installer_replacement_requires_receipt_and_process_retirement() {
        let (f,wrong) = fixture();
        let initial = wrong_installer_setup(&f,&wrong,Some("cancelled"));
        let right = different_installer(&f,&wrong);
        let op = job(&f.m,&initial.id,&right.id);
        drop(f.m.lock("onboarding.lock").unwrap());
        let before = test_fixture::snapshot(&f.m.root);
        let owner = initial.installation_operation.as_ref().unwrap();
        assert_eq!(reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),
            |id|Ok(id == owner)).unwrap_err().to_string(),"operator_installer_active");
        assert_eq!(test_fixture::snapshot(&f.m.root),before);
        let receipt = onboarding::directory(&f.m,&initial.id).unwrap().join(format!("{owner}-result.json"));
        let mut unresolved: Value = read_json(&receipt).unwrap();
        unresolved["cleanup_confirmed"] = false.into();
        unresolved["owned_live"] = Value::Null;
        atomic_json(&receipt,&unresolved).unwrap();
        assert!(projection(&f.m,&[],None,|_|Ok(false)).unwrap().iter()
            .all(|target| target.environment != initial.id));
        assert_eq!(reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),|_|Ok(false))
            .unwrap_err().to_string(),"installer_initial_retirement_required");
        // A stopped replacement with a live owner is also excluded; Retry must
        // keep that replacement's bytes after its owner actually retires.
        unresolved["cleanup_confirmed"] = true.into();unresolved["owned_live"] = 0.into();
        atomic_json(&receipt,&unresolved).unwrap();
        let r = reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        complete(&f.m,&r,"cancelled");
        let second = job(&f.m,&initial.id,&wrong.id);
        let rows = projection(&f.m,&[],None,|id|Ok(id == op)).unwrap();
        let target = rows.iter().find(|t|t.environment == initial.id).unwrap();
        assert!(target.choices.iter().all(|a|a.disabled_reason.is_some()));
        assert!(target.actions.iter().any(|a|matches!(a.action,ui::Action::EnvironmentInstallerStop {..})));
        assert_eq!(reserve_with(&f.m,&initial.id,&wrong.id,&second,||Ok(true),|id|Ok(id == op))
            .unwrap_err().to_string(),"operator_installer_active");
        let rows = projection(&f.m,&[],None,|_|Ok(false)).unwrap();
        let retry = rows.iter().find(|t|t.environment == initial.id).unwrap().actions.iter()
            .find(|a|a.label == "Retry").unwrap();
        assert_eq!(retry.action,ui::Action::EnvironmentInstallerStart {
            environment:initial.id,installer:right.id});
    }
    #[test]
    fn replacement_refuses_missing_initialized_state_and_reads_legacy_binding() {
        for missing_root in [false,true] {
            let (f,wrong)=fixture();let initial=wrong_installer_setup(&f,&wrong,Some("cancelled"));
            let right=different_installer(&f,&wrong);let op=job(&f.m,&initial.id,&right.id);
            let binding=reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),|_|Ok(false)).unwrap();
            assert_eq!(binding.preserve_existing_prefix,Some(true));
            // Test69 records omit the new field. Existing successful prefix
            // initialization in their retained receipt still requires preservation.
            let mut old=serde_json::to_value(&binding).unwrap();old.as_object_mut().unwrap().remove("preserve_existing_prefix");
            let legacy:Binding=serde_json::from_value(old).unwrap();
            assert_eq!(legacy.preserve_existing_prefix,None);
            for changed in ["installer","environment","before","preservation","automatic request"] {
                let mut copy=binding.clone();
                match changed {
                    "installer"=>copy.installer="ef".repeat(32),
                    "environment"=>copy.environment.root=f.outer.join("unowned-environment"),
                    "before"=>copy.before.revision+=1,
                    "preservation"=>copy.preserve_existing_prefix=Some(false),
                    _=>copy.automatic_request=Some(f.r.host.clone()),
                }
                assert_eq!(launch_input(&f.m,&copy).unwrap_err().to_string(),"installer_transaction_binding","{changed}");
            }
            let missing=if missing_root {initial.environment.root.clone()}
                else {initial.environment.root.join("compatdata/pfx")};
            fs::remove_dir_all(missing).unwrap();
            for record in [&binding,&legacy] {
                assert_eq!(launch(&f.m,record).unwrap_err().to_string(),MISSING_INSTALLATION);
                assert!(!job_dir(&f.m,&op).unwrap().join("installer-spec.json").exists());
            }
        }
        let (f,wrong)=fixture();let initial=wrong_installer_setup(&f,&wrong,Some("completed"));
        fs::remove_dir_all(initial.environment.root.join("compatdata/pfx")).unwrap();
        let right=different_installer(&f,&wrong);let op=job(&f.m,&initial.id,&right.id);
        let before=fs::read(initial.environment.root.join("environment.json")).unwrap();
        assert_eq!(reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),|_|Ok(false))
            .unwrap_err().to_string(),MISSING_INSTALLATION);
        assert_eq!(fs::read(initial.environment.root.join("environment.json")).unwrap(),before);
        assert!(!job_dir(&f.m,&op).unwrap().join("installation.json").exists());
        let (f,wrong)=fixture();let initial=wrong_installer_setup(&f,&wrong,Some("completed"));
        fs::remove_dir_all(&initial.environment.root).unwrap();
        let right=different_installer(&f,&wrong);let op=job(&f.m,&initial.id,&right.id);
        let history=fs::read(onboarding::directory(&f.m,&initial.id).unwrap().join("record.json")).unwrap();
        assert_eq!(reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),|_|Ok(false))
            .unwrap_err().to_string(),MISSING_INSTALLATION);
        assert!(!initial.environment.root.exists());assert!(!job_dir(&f.m,&op).unwrap().join("installation.json").exists());
        assert_eq!(fs::read(onboarding::directory(&f.m,&initial.id).unwrap().join("record.json")).unwrap(),history);
        let (f,wrong)=fixture();let initial=wrong_installer_setup(&f,&wrong,None);
        let right=different_installer(&f,&wrong);let op=job(&f.m,&initial.id,&right.id);
        let binding=reserve_with(&f.m,&initial.id,&right.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        assert_eq!(binding.preserve_existing_prefix,Some(false));
    }
    #[test]
    fn ordinary_existing_setup_and_retry_select_same_scoped_advice() {
        let (f, initial) = onboarding::tests::close_policy_fixture();
        let initial_op = initial.installation_operation.as_ref().unwrap();
        atomic_json(&onboarding::directory(&f.m, &initial.id).unwrap().join(format!("{initial_op}-result.json")),
            &json!({"schema":2,"operation":initial_op,"state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
        let history = fs::read(onboarding::directory(&f.m, &initial.id).unwrap().join("record.json")).unwrap();
        let first = job(&f.m, &initial.id, &initial.installer);
        private_dir(&initial.environment.root.join("compatdata/pfx")).unwrap();
        fs::write(initial.environment.root.join("compatdata/pfx/system.reg"),b"first-party retained state").unwrap();
        let binding = reserve_with(&f.m, &initial.id, &initial.installer, &first, || Ok(true), |_|Ok(false)).unwrap();
        let input = launch_input(&f.m, &binding).unwrap();
        assert_eq!(input["schema"], 3);
        assert_eq!(input["installer_capability"]["windows_scripting"]["powershell"], "intentionally_unavailable");
        assert_eq!(input["installer_capability"]["operation"], first);
        assert_eq!(binding.environment.runner, initial.environment.runner);
        assert_eq!(binding.environment.root, initial.environment.root);
        complete(&f.m, &binding, "cancelled");
        let next = job(&f.m, &initial.id, &initial.installer);
        let retry = reserve_with(&f.m, &initial.id, &initial.installer, &next, || Ok(true), |_|Ok(false)).unwrap();
        let retry_input = launch_input(&f.m, &retry).unwrap();
        assert_eq!(retry_input["installer_capability"]["operation"], next);
        assert_eq!(retry_input["installer_capability"]["environment"]["revision"], retry.environment.revision);
        assert_eq!(retry.environment.id, initial.environment.id);
        assert_eq!(retry.environment.root, initial.environment.root);
        assert_eq!(retry.environment.runner, initial.environment.runner);
        assert_eq!(retry_input["installer_capability"]["windows_scripting"]["powershell"], "intentionally_unavailable");
        assert_eq!(fs::read(onboarding::directory(&f.m, &initial.id).unwrap().join("record.json")).unwrap(), history);
    }
    #[test]
    fn actual_same_byte_retry_preserves_both_explicit_policies_on_matched_and_unmatched_installers() {
        use linux_vst_bridge::installer_policy::Powershell;
        for (powershell,matched) in [Powershell::Inherited,Powershell::IntentionallyUnavailable]
            .into_iter().flat_map(|policy|[true,false].map(move |matched|(policy,matched))) {
            let (f,initial)=onboarding::tests::close_policy_fixture();
            let initial_op=initial.installation_operation.as_ref().unwrap();
            atomic_json(&onboarding::directory(&f.m,&initial.id).unwrap().join(format!("{initial_op}-result.json")),
                &json!({"schema":2,"operation":initial_op,"state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
            private_dir(&initial.environment.root.join("compatdata/pfx")).unwrap();
            fs::write(initial.environment.root.join("compatdata/pfx/system.reg"),b"first-party retained state").unwrap();
            let selected=if matched {initial.installer.clone()} else {
                mod fixture {include!("../../../tests/fixtures/nsis_installer.rs");}
                let path=f.outer.join("unmatched-installer.exe");
                fs::write(&path,fixture::fixture(Some(("FileVersion","unknown-version")))).unwrap();
                installer_import::import(&f.m,file(&path).unwrap()).unwrap().id
            };
            let mut previous=None;
            for attempt in 0..2 {
                let units:Vec<_>=std::iter::once(initial_op.clone()).chain(records(&f.m).unwrap().into_iter()
                    .map(|record|record.operation)).map(|operation|onboarding::unit(&operation).unwrap()).collect();
                let snapshot=readiness::with_unit_state_fixture(&units,"inactive",||snapshot_idle_test(&f.m)).unwrap();
                let target=snapshot.environment_installers.iter().find(|target|target.environment==initial.id).unwrap();
                let offer=if attempt==0 {
                    target.actions.iter().find(|offer|offer.action.environment_installer_start()
                        .is_some_and(|(environment,installer,policy)|environment==initial.id
                            && installer==selected && policy==Some(powershell))).unwrap()
                } else {target.actions.iter().find(|offer|offer.label=="Retry").unwrap()};
                if let Some(previous)=&previous {assert_eq!(&offer.action,previous);}
                assert_eq!(offer.action.environment_installer_start().unwrap().2,Some(powershell));
                // Choosing a new installer is still an ordinary choice, not a
                // policy inherited from the previous same-byte Retry.
                assert!(target.choices.iter().any(|offer|offer.action.environment_installer_start()
                    .is_some_and(|(_,installer,policy)|installer==selected && policy.is_none())));
                let chosen=offer.action.clone();
                let operation=readiness::with_unit_state_fixture(&units,"inactive",||test_submit_offered(&f.m,&chosen)).unwrap();
                let (environment,installer,_)=chosen.environment_installer_start().unwrap();
                let binding=reserve_with(&f.m,environment,installer,&operation,||Ok(true),|_|Ok(false)).unwrap();
                let input=launch_input(&f.m,&binding).unwrap();
                assert_eq!(input["installer_capability"]["windows_scripting"]["powershell"],serde_json::to_value(powershell).unwrap());
                assert_eq!(input["installer_capability"]["operation"],operation);
                assert_eq!(input["installer_capability"]["environment"],serde_json::to_value(&binding.environment).unwrap());
                complete(&f.m,&binding,if attempt==0 {"cancelled"} else {"failed"});previous=Some(chosen);
            }
        }
    }
    #[test]
    fn registered_uuid_setup_accepts_both_explicit_installer_policies_without_changing_identity() {
        let (f,installer)=fixture_with_catalogue(true);let environment=f.r.environment.clone();
        assert!(environment.id.contains('-'));
        let mut sw=software_record(&f.m).unwrap();sw.installer_launch=Some(sw.host.clone());
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();
        for powershell in [ui::Powershell::Inherited,ui::Powershell::IntentionallyUnavailable] {
            let units:Vec<_>=records(&f.m).unwrap().into_iter().map(|record|onboarding::unit(&record.operation).unwrap()).collect();
            let snapshot=readiness::with_unit_state_fixture(&units,"inactive",||snapshot_idle_test(&f.m)).unwrap();
            let offered=available(&snapshot).into_iter().find(|offer|matches!(&offer.action,
                ui::Action::EnvironmentInstallerStartWithPolicy {environment:id,installer:selected,powershell:policy}
                    if id==&environment.id && selected==&installer.id && policy==&powershell)).unwrap();
            let request=ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:snapshot.state_token.clone(),action:offered.action.clone()};
            validate(&request,&snapshot).unwrap();
            let operation=random_id().unwrap();private_dir(&job_dir(&f.m,&operation).unwrap()).unwrap();
            atomic_json(&job_dir(&f.m,&operation).unwrap().join("request.json"),&request).unwrap();
            let binding=reserve_with(&f.m,&environment.id,&installer.id,&operation,||Ok(true),|_|Ok(false)).unwrap();
            let spec=launch_input(&f.m,&binding).unwrap();
            assert_eq!(spec["installer_capability"]["environment"]["id"],environment.id);
            assert_eq!(spec["installer_capability"]["windows_scripting"]["powershell"],serde_json::to_value(powershell).unwrap());
            assert_eq!(binding.environment.root,environment.root);assert_eq!(binding.environment.runner,environment.runner);
            complete(&f.m,&binding,"cancelled");
        }
    }
    #[test]
    fn registered_companion_preserves_identity_and_invalidates_all_siblings() {
        let (f,installer)=fixture();let env=&f.r.environment;
        let mut sibling=f.r.clone();sibling.metadata.class_id="02".repeat(16);
        sibling.metadata.name="Sibling".into();f.m.register(sibling.clone()).unwrap();
        let original=fs::read(f.m.root.join("registry.json")).unwrap();
        let op=job(&f.m,&env.id,&installer.id);
        let r=reserve_with(&f.m,&env.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        assert_eq!(r.environment.id,env.id);assert_eq!(r.environment.root,env.root);
        assert_eq!(r.environment.runner,env.runner);assert_eq!(r.environment.revision,2);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),original);
        assert!(f.r.validate_record(&f.m.root).is_err());
        assert!(sibling.validate_record(&f.m.root).is_err());
        assert!(!onboarding::all_retired(&f.m).unwrap());
        assert!(onboarding::load(&f.m,&env.id).is_err());
        assert_eq!(target(&f.m,&env.id).unwrap(),r.environment);
        let catalogue=catalogue::Catalogue {schema:1,natives:vec![],hosts:vec![],onboarding_runtime:None,
            environments:vec![catalogue::EnvironmentBinding {family:profiles::Family::ManagedInstallerV1,
                environment:env.clone()}]};
        let bindings=managed_environment_bindings(&f.m,Some(&catalogue),&f.m.registry().unwrap()).unwrap();
        assert_eq!(bindings[0].environment,r.environment);
        let rows=environment_projection_from(&f.m,&software_record(&f.m).unwrap(),&bindings,
            &f.m.registry().unwrap(),Some("installer active")).unwrap();
        assert_eq!(rows[0].revision,2);
        let second=job(&f.m,&env.id,&installer.id);
        assert_eq!(reserve_with(&f.m,&env.id,&installer.id,&second,
            || vendor_maintenance::observe_retired(&f.m,&target(&f.m,&env.id)?).map(|_|true),
            |_|Ok(false)).unwrap_err().to_string(),"setup_installer_cleanup_unconfirmed");
        complete(&f.m,&r,"cancelled");
        assert!(onboarding::all_retired(&f.m).unwrap());
        let next=reserve_with(&f.m,&env.id,&installer.id,&second,||Ok(true),|_|Ok(false)).unwrap();
        assert_eq!(next.environment.revision,3);
        assert_eq!(result(&f.m,&r).unwrap()["state"],"cancelled");
        let mut earlier=r.clone();earlier.created_at=u64::MAX;
        atomic_json(&job_dir(&f.m,&r.operation).unwrap().join("installation.json"),&earlier).unwrap();
        let rows=projection(&f.m,&[],None,|op|Ok(op==second)).unwrap();
        assert!(rows[0].actions.iter().any(|offer|matches!(&offer.action,
            ui::Action::EnvironmentInstallerStop {operation} if operation==&second)));
    }
    #[test]
    fn unregistered_setup_resolves_revision_without_rewriting_initial_history() {
        let (f,installer)=fixture();let mut env=f.r.environment.clone();
        env.id="cd".repeat(16);env.root=f.m.root.join("environments").join(&env.id);
        private_dir(&env.root).unwrap();atomic_json(&env.root.join("environment.json"),&env).unwrap();
        private_dir(&env.root.join("compatdata/pfx")).unwrap();
        fs::write(env.root.join("compatdata/pfx/system.reg"),b"first-party retained state").unwrap();
        let initial="ab".repeat(16);let dir=onboarding::directory(&f.m,&env.id).unwrap();
        private_dir(&dir).unwrap();
        atomic_json(&dir.join("record.json"),&onboarding::Record {schema:1,id:env.id.clone(),
            installer:installer.id.clone(),environment:env.clone(),created_at:1,creation_operation:"12".repeat(16),
            installation_operation:Some(initial.clone()),published:false,automatic:None,previous_attempt:None}).unwrap();
        atomic_json(&dir.join(format!("{initial}-result.json")),&json!({"schema":2,
            "operation":initial,"state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
        let retained=fs::read(dir.join("record.json")).unwrap();
        let op=job(&f.m,&env.id,&installer.id);
        let r=reserve_with(&f.m,&env.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        assert_eq!(onboarding::load(&f.m,&env.id).unwrap().environment,r.environment);
        assert_eq!(fs::read(dir.join("record.json")).unwrap(),retained);
        // Crash before launch is recoverable without inventing vendor success.
        finish_with(&f.m,&op,|_|Ok(false)).unwrap();
        let v=result(&f.m,&r).unwrap();assert_eq!(v["state"],"failed");
        assert!(onboarding::retired(&v));assert!(onboarding::all_retired(&f.m).unwrap());
        let rows=projection(&f.m,&[],None,|_|Ok(false)).unwrap();
        let row=rows.iter().find(|row|row.environment == env.id).unwrap();
        assert!(row.actions.iter().any(|a|matches!(a.action,ui::Action::EnvironmentInstallerScan { .. })));
        assert!(row.actions.iter().any(|a|matches!(a.action,ui::Action::EnvironmentInstallerStart { .. })));
    }
    #[test]
    fn exact_control_survives_missing_inputs_and_malformed_binding_refuses() {
        let (f,installer)=fixture();let op=job(&f.m,&f.r.environment.id,&installer.id);
        let r=reserve_with(&f.m,&f.r.environment.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        fs::remove_file(&installer.artifact.path).unwrap();
        fs::remove_file(&r.environment.runner.proton).unwrap();
        assert!(control(&f.m,&op).is_ok());assert!(launch(&f.m,&r).is_err());
        let mut changed=r.clone();changed.environment.id="ff".repeat(16);
        atomic_json(&job_dir(&f.m,&op).unwrap().join("installation.json"),&changed).unwrap();
        assert!(control(&f.m,&op).is_err());assert!(all_retired(&f.m).is_err());
    }
    #[test]
    fn ordinary_current_offer_keeps_stop_when_installer_runtime_and_module_disappear() {
        use linux_vst_bridge::{preparation as prep,runtime_delivery};
        let (f,base)=preparation_cli::tests::projection_fixture();
        fs::write(base.selection.environment.root.join("compatdata/pfx/system.reg"),b"first-party retained state").unwrap();
        atomic_json(&f.m.root.join("software.json"),
            &preparation_cli::tests::projection_software(&base)).unwrap();
        prep::record_candidate(&f.m,&base).unwrap();prep::enable(&f.m,&base,false).unwrap();
        let mut bytes=vec![0;1024];bytes[..2].copy_from_slice(b"MZ");bytes[60]=128;
        bytes[128..132].copy_from_slice(b"PE\0\0");bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148]=2;bytes[150]=2;bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let source=f.outer.join("companion.exe");fs::write(&source,bytes).unwrap();
        let installer=installer_import::import(&f.m,file(&source).unwrap()).unwrap();
        let op=job(&f.m,&base.selection.environment.id,&installer.id);
        let r=reserve_with(&f.m,&base.selection.environment.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        let runtime_path=f.m.root.join("runners").join(runtime_delivery::ID).join("runtime.json");
        let runtime_dir=runtime_path.parent().unwrap();
        let artifact=|relative:&str| {
            let path=runtime_dir.join(relative);private_dir(path.parent().unwrap()).unwrap();
            fs::write(&path,b"runtime identity").unwrap();
            Artifact {sha256:digest(&path).unwrap(),path}
        };
        let proton=artifact("GE-Proton11-7-x86_64/proton");
        let entry=artifact("SteamLinuxRuntime_4/_v2-entry-point");
        let tree=artifact("runtime-tree.json");
        let runner=Runner {id:runtime_delivery::ID.into(),version:"retained-test".into(),
            proton:proton.path.clone(),entry_point:entry.path.clone(),files:vec![proton,entry,tree],policy:None};
        atomic_json(&runtime_path,&json!({"schema":1,"id":runtime_delivery::ID,
            "downloads":runtime_delivery::downloads(),"runner":runner})).unwrap();
        fs::set_permissions(&runtime_path,fs::Permissions::from_mode(0o400)).unwrap();
        fs::remove_file(&installer.artifact.path).unwrap();
        fs::remove_file(&runner.proton).unwrap();
        fs::remove_file(&r.environment.runner.proton).unwrap();
        fs::remove_file(&base.selection.module.path).unwrap();
        let captured=current::capture_with_installer_live(&f.m,|owned|Ok(owned==op)).unwrap();
        let stop=available(&captured.snapshot).into_iter().find(|a|matches!(&a.action,
            ui::Action::EnvironmentInstallerStop {operation} if operation==&op)).unwrap();
        assert!(stop.disabled_reason.is_none());
        assert!(captured.snapshot.environment_installers[0].choices.iter().all(|a|a.disabled_reason.is_some()));
        let request=ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:captured.snapshot.state_token.clone(),
            action:stop.action.clone()};
        validate_installer_control_with(&f.m,&request,|owned|Ok(owned==op)).unwrap();
        assert!(launch(&f.m,&r).is_err());
        assert_eq!(captured.snapshot.products[0].disposition,"needs_attention");
    }
    #[test]
    fn unfinished_supervisor_retains_cleanup_gap_and_global_exclusion() {
        let (f,installer)=fixture();let op=job(&f.m,&f.r.environment.id,&installer.id);
        let r=reserve_with(&f.m,&f.r.environment.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        atomic_json(&job_dir(&f.m,&op).unwrap().join("installer-spec.json"),&json!({"operation":op})).unwrap();
        finish_with(&f.m,&op,|_|Ok(false)).unwrap();
        assert_eq!(result(&f.m,&r).unwrap()["state"],"cleanup_unconfirmed");
        assert!(!onboarding::all_retired(&f.m).unwrap());
        let rows=projection(&f.m,&[],None,|_|Ok(false)).unwrap();
        assert!(rows[0].choices.iter().all(|a|a.disabled_reason.is_some()));
    }
    #[test]
    fn terminal_completion_failures_survive_existing_setup_result_readback() {
        let (f,installer)=fixture();let op=job(&f.m,&f.r.environment.id,&installer.id);
        let r=reserve_with(&f.m,&f.r.environment.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
        let path=job_dir(&f.m,&op).unwrap().join("installation-result.json");
        for (outcome,error) in [("failed","installer_time_bound"),("failed","installer_owner_RuntimeError"),
            ("installer_completion_unconfirmed","installer_completion_unconfirmed")] {
            let value=json!({"schema":2,"operation":op,"state":"failed","error":error,
                "cleanup_confirmed":true,"owned_live":0,"transaction":{"schema":1,"operation":op,
                    "outcome":outcome,"durable_installation":"installed","root_completion":{"status":"unconfirmed"}}});
            atomic_json(&path,&value).unwrap();
            let read=result(&f.m,&r).unwrap();assert_eq!(read,value);assert!(onboarding::retired(&read));
        }
    }
    #[test]
    fn close_census_cannot_supply_missing_or_conflicting_root_completion() {
        for status in [None,Some("unconfirmed"),Some("failed"),Some("cancelled"),Some("succeeded")] {
            let mut value=json!({"state":"cleanup_unconfirmed","raw_exit":0,"error":null,
                "transaction":{"outcome":"cleanup_unconfirmed","durable_installation":"installed",
                    "launch_binding":{"completion_contract":"retained_handle_status_v1"}}});
            if let Some(status)=status {value["transaction"]["root_completion"]=json!({"status":status});}
            closed(&mut value);
            assert_eq!(value["state"],if status==Some("succeeded") {"completed"} else {"failed"});
            if status!=Some("succeeded") {assert_ne!(value["transaction"]["outcome"],"installed");}
        }
        for reason in ["root_exit_not_observed","root_exit_status_conflict"] {
            let (f,installer)=fixture();let op=job(&f.m,&f.r.environment.id,&installer.id);
            reserve_with(&f.m,&f.r.environment.id,&installer.id,&op,||Ok(true),|_|Ok(false)).unwrap();
            atomic_json(&job_dir(&f.m,&op).unwrap().join("installation-result.json"),&json!({
                "schema":2,"operation":op,"state":"cleanup_unconfirmed","cleanup_confirmed":false,
                "raw_exit":0,"error":null,"transaction":{"outcome":"cleanup_unconfirmed",
                    "durable_installation":"installed","launch_binding":{"completion_contract":"retained_handle_status_v1"},
                    "root_completion":{"status":"unconfirmed","reason":reason}}})).unwrap();
            let value=confirm_closed(&f.m,&op,&|_|Ok(false)).unwrap();
            assert_eq!(value["cleanup_confirmed"],true);assert_eq!(value["owned_live"],0);
            assert_eq!(value["closing_confirmed_by"],"census");assert_eq!(value["state"],"failed");
            assert_eq!(value["transaction"]["root_completion"]["reason"],reason);
            assert_ne!(value["transaction"]["outcome"],"installed");
        }
    }
}

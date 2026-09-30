//! MF1: closed operator requests dispatched to existing canonical owners.
use super::*;
use linux_vst_bridge::operator_model as ui;
use linux_vst_bridge::renderer_application as renderer;
use serde_json::{json, Value};
mod current;
const ASC: &str = "arturia-software-center";
const SERVICE: &str = "linux-vst-bridge.service";
const VENDOR: &str = "linux-vst-bridge-vendor-arturia-software-center.service";
#[cfg(test)]
thread_local! {
    static SCAN_SPAWN_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("unknown").into()
}
fn active(unit: &str) -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", unit])
        .status()
        .is_ok_and(|s| s.success())
}
fn vendor_state_live(state: &str) -> Result<bool> {
    match state {
        "active" | "activating" | "deactivating" | "reloading" => Ok(true),
        "inactive" | "failed" => Ok(false),
        _ => Err("operator_vendor_unit_state_unavailable".into()),
    }
}
fn vendor_live() -> Result<bool> {
    vendor_state_live(&readiness::bounded_user_unit_state(VENDOR)
        .map_err(|_| "operator_vendor_unit_state_unavailable")?)
}
fn service(action: &str) -> Result<()> {
    require(
        Command::new("systemctl")
            .args(["--user", action, SERVICE])
            .status()?
            .success(),
        "operator_service_transition_failed",
    )
}
fn optional(path: &Path) -> Result<Value> {
    if path.try_exists()? {
        require(
            file(path)?.metadata()?.len() <= 8 * 1024 * 1024,
            "operator_record_bound",
        )?;
        read_json(path)
    } else {
        Ok(Value::Null)
    }
}
fn token(m: &Manager) -> Result<String> {
    token_with_registry(m, &m.registry()?)
}
fn token_with_registry(m: &Manager, registry: &Registry) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(
        &json!({"software":optional(&m.root.join("software.json"))?,"registry":registry,
            "managed_runtime":optional(&linux_vst_bridge::runtime_delivery::record_path(m))?,
            "preparation":optional(&m.root.join("preparation/revision.json"))?,
            "workspace":optional(&m.root.join("daw-workspaces/fl-studio/workspace.json"))?,
            "terminal_summaries":capacity::terminal_summaries(m)?,
            "installer_presentations":installer_import::presentation_token(m)?,
            "onboarding_history":onboarding::history_records(m)?}),
    )?)))
}
fn action(label: &str, action: ui::Action, reason: Option<&str>) -> ui::AvailableAction {
    ui::AvailableAction {
        label: label.into(),
        action,
        disabled_reason: reason.map(Into::into),
    }
}
fn session_projection(
    capacity: Option<&CapacityReadback>,
    summaries: Vec<capacity::TerminalSummary>,
) -> Vec<Value> {
    let mut rows:Vec<_> = capacity
        .into_iter().flat_map(|status|status.owners.iter())
        .filter(|owner| owner.kind == capacity::Kind::Dsp)
        .map(|owner| json!({
            "session":owner.session,
            "class_id":owner.class_id,
            "state":if capacity.is_some_and(|status|status.cleanup_unconfirmed){
                "cleanup_unconfirmed"
            }else if owner.terminal.is_some(){
                "failed"
            }else{
                "active"
            },
            "terminal":owner.terminal,
            "recent":false,
        }))
        .collect();
    rows.extend(summaries.into_iter().map(|summary|json!({
        "session":summary.session,
        "class_id":summary.class_id,
        "state":"failed",
        "terminal":summary.terminal,
        "recent":true,
        "cleanup_confirmed":summary.cleanup_confirmed,
        "transport_retired":summary.transport_retired,
        "observed_at":summary.observed_at,
    })));
    rows
}
fn quarantined_product(scan: &inventory::Scan, module_index: usize,
    module: inventory::Module, stale: Option<&str>, busy: Option<&str>,
    retry_disabled: Option<&str>)
    -> Result<ui::Product> {
    let reason=module.quarantine_reason.as_deref()
        .ok_or("operator_quarantine_reason_absent")?;
    let inspection_error=module.inspection_error.clone();
    let mut limitations:Vec<String>=stale.into_iter().map(str::to_owned).collect();
    if let Some(error)=&inspection_error { limitations.push(format!("Scanner reported: {error}")); }
    limitations.push(reason.into());
    let actions=vec![action("Retry this exact module scan",
        ui::Action::QuarantinedModuleRetry {environment:scan.environment.id.clone(),
            scan:scan.id.clone(),module_index,module_sha256:module.artifact.sha256.clone(),
            report_sha256:module.report.sha256.clone()},busy.or(stale).or(retry_disabled))];
    Ok(ui::Product {class_id:String::new(),name:module.artifact.path.file_name()
        .unwrap_or_default().to_string_lossy().into_owned(),vendor:"Unresolved factory".into(),
        role:"unknown".into(),version:String::new(),
        disposition:if stale.is_some(){"needs_attention"}else{"quarantined"}.into(),
        active_revision:None,recommended_revision:None,environment:scan.environment.id.clone(),
        runner:scan.environment.runner.id.clone(),module_sha256:module.artifact.sha256,
        limitations,history:vec![],actions,compatibility:None,
        details:json!({"scan":scan.id,"scanner_host_sha256":scan.host.sha256,
            "scanner_source_sha256":scan.host_source_sha256,"current":stale.is_none(),
            "inspection_error":inspection_error,"quarantine_reason":reason,
            "report_sha256":module.report.sha256,"activation_permitted":false})})
}
fn app_directory(m: &Manager) -> PathBuf {
    m.root.join("vendor-applications").join(ASC)
}
fn asc_projection(m: &Manager, busy: Option<&str>) -> Result<Option<ui::VendorApplication>> {
    let app = app_directory(m).join("application.json");
    if !app.exists() { return Ok(None); }
    let a: vendor_application::Application = read_json(&app)?;
    let live = vendor_live()?;
    let valid = a.verify(&m.root).is_ok();
    let reason = if !valid {
        Some("Registered ASC executable or environment changed")
    } else { busy };
    Ok(Some(ui::VendorApplication {
        details: json!({"environment":a.environment.id}),
        id: ASC.into(), name: "Arturia Software Center".into(),
        version: a.observed_installer_version,
        state: if live { "running" } else if vendor_retired(m)? {
            "closed"
        } else { "cleanup_unconfirmed" }.into(),
        actions: vec![
            action("Open ASC", ui::Action::VendorApplicationOpen {
                application: ASC.into(),
            }, if live { Some("ASC is already running") } else { reason }),
            action("Focus ASC", ui::Action::VendorApplicationFocus {
                application: ASC.into(),
            }, if live { None } else { Some("ASC is not running") }),
            action("Stop owned ASC session", ui::Action::VendorApplicationStop {
                application: ASC.into(),
            }, if live { None } else { Some("ASC is not running") }),
        ],
    }))
}
pub(super) fn vendor_retired(m: &Manager) -> Result<bool> {
    if !renderer_cli::all_retired(m)? || !dependency_cli::all_retired(m)? {
        return Ok(false);
    }
    if !app_directory(m).exists() {
        return Ok(true);
    }
    if vendor_live()? {
        return Ok(false);
    }
    let p = app_directory(m).join("operation-result.json");
    if !p.exists() {
        return Ok(true);
    }
    Ok(read_json::<vendor_application::OperationResult>(&p)?.retired())
}
fn count_entries(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let n = fs::read_dir(path)?
        .take(4097)
        .collect::<std::io::Result<Vec<_>>>()?
        .len();
    require(n <= 4096, "operator_entry_bound")?;
    Ok(n)
}
fn capture_state(m: &Manager) -> Result<Value> {
    let mut collecting = 0;
    for path in crash_capture::incidents(m)? {
        let state = optional(&path.join("status.json"))?;
        if matches!(state["state"].as_str(), Some("claimed" | "collecting")) {
            collecting += 1;
        }
    }
    Ok(
        json!({"armed":m.root.join("runtime/incidents/next.json").exists(),"active_retention":collecting}),
    )
}
#[derive(Deserialize)]
struct CapacityReadback {
    schema: u32,
    dsp: usize,
    maintenance: usize,
    keepers: usize,
    cleanup_unconfirmed: bool,
    owners: Vec<capacity::Owner>,
    limits: CapacityLimits,
}
#[derive(Deserialize)]
struct CapacityLimits {
    global_dsp: usize,
}
fn live_capacity(m: &Manager) -> Result<CapacityReadback> {
    let reply = capacity_reply(m)?;
    require(reply["ok"] == true, "capacity_readback_unavailable")?;
    let value: CapacityReadback = serde_json::from_value(reply["capacity"].clone())?;
    require(value.schema == 1, "operator_capacity_schema")?;
    Ok(value)
}
pub(super) fn pending_transactions(m: &Manager) -> Result<usize> {
    let transactions = m.root.join("transactions");
    let mut pending = 0;
    if transactions.exists() {
        require(
            count_entries(&transactions)? <= 4096,
            "operator_transaction_bound",
        )?;
        for item in fs::read_dir(transactions)? {
            if item?
                .file_name()
                .to_string_lossy()
                .ends_with(".pending.json")
            {
                pending += 1;
            }
        }
    }
    Ok(pending)
}
fn activity_with_capacity(m: &Manager, cap: Option<&CapacityReadback>) -> Result<ui::Activity> {
    let pending = pending_transactions(m)?;
    let stale = stale_transports(cap)?;
    Ok(ui::Activity {
        schema: ui::OPERATOR_SCHEMA,
        system: system_from_capacity(cap, pending, stale),
        capture: capture_state(m)?,
        operation: optional(&m.root.join("operator/latest.json"))?
            .as_object().map(|v| Value::Object(v.clone())),
        workspace_product_install_ready: daw_workspace::serum2_install_finish_ready(m)?,
    })
}
fn stale_transports(cap: Option<&CapacityReadback>) -> Result<usize> {
    let root = transport_storage::root();
    let mut stale = 0;
    if let Some(cap) = cap {
        if root.exists() {
            require(count_entries(&root)? <= 4096, "operator_transport_bound")?;
            for item in fs::read_dir(root)? {
                let item = item?;
                let name = item.file_name().to_string_lossy().into_owned();
                if item.file_type()?.is_dir()
                    && valid_hex(&name, 32)
                    && !cap.owners.iter().any(|o| o.session == name)
                {
                    stale += 1;
                }
            }
        }
    }
    Ok(stale)
}
fn system_from_capacity(cap: Option<&CapacityReadback>, pending: usize, stale: usize) -> ui::System {
    ui::System {
            service: if cap.is_some() {
                "active"
            } else {
                "capacity unavailable"
            }
            .into(),
            keepers: cap.map_or(0, |c| c.keepers),
            dsp: cap.map_or(0, |c| c.dsp),
            maintenance: cap.map_or(0, |c| c.maintenance),
            ceiling: cap.map_or(0, |c| c.limits.global_dsp),
            pending_transactions: pending,
            stale_transports: stale,
            cleanup_unconfirmed: cap.is_none_or(|c| c.cleanup_unconfirmed),
    }
}
fn activity(m: &Manager) -> Result<ui::Activity> {
    let cap = live_capacity(m).ok();
    activity_with_capacity(m, cap.as_ref())
}
fn pulse_generation(m: &Manager) -> Result<String> {
    use std::os::unix::fs::MetadataExt;
    let paths = ["software.json", "registry.json", "preparation/revision.json",
        "daw-workspaces/fl-studio/workspace.json", "operator/latest.json",
        "installers", "onboarding", "inventory", "transactions", "performance",
        "runtime/leases", "runtime/owner.sock", "runners/managed-ge-proton11-7-slr4-20260805/runtime.json"];
    let mut stamps = Vec::with_capacity(paths.len());
    for relative in paths {
        match fs::symlink_metadata(m.root.join(relative)) {
            Ok(meta) => stamps.push(json!([relative,meta.dev(),meta.ino(),meta.len(),
                meta.mtime(),meta.mtime_nsec(),meta.ctime(),meta.ctime_nsec()])),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound =>
                stamps.push(json!([relative,"absent"])),
            Err(e) => return Err(e.into()),
        }
    }
    let performance = m.root.join("performance");
    if performance.is_dir() {
        let mut entries = fs::read_dir(performance)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for (n, entry) in entries.into_iter().enumerate() {
            require(n < 4096,"operator_pulse_performance_bound")?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path)?;
            stamps.push(json!([path.file_name().map(|name|name.to_string_lossy().into_owned()),meta.dev(),meta.ino(),meta.len(),
                meta.mtime(),meta.mtime_nsec(),meta.ctime(),meta.ctime_nsec()]));
        }
    }
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(&stamps)?)))
}
fn pulse_cleanup(m: &Manager) -> Option<bool> {
    let mut peer = UnixStream::connect(m.root.join("runtime/owner.sock")).ok()?;
    peer.set_read_timeout(Some(Duration::from_millis(200))).ok()?;
    peer.set_write_timeout(Some(Duration::from_millis(200))).ok()?;
    peer.write_all(b"LVP1\n").ok()?;
    let mut state = [0_u8;1];
    peer.read_exact(&mut state).ok()?;
    match state[0] {0=>Some(false),1=>Some(true),_=>None}
}
fn operation_state_live(state: &str) -> bool {
    matches!(state, "queued" | "waiting" | "running" | "vendor_running")
}
fn pulse(m: &Manager) -> Result<ui::Pulse> {
    // The selected service's full LVC1 capacity response verifies product
    // envelopes and is deliberately reserved for a fresh overview or action.
    // Pulse only detects changes. It never certifies cleanup or admits work.
    let service_state = readiness::service_state();
    let owners = acquire_readback(m, ui::OperatorLock::Registry, None,
        Duration::from_millis(200), &mut vec![])
        .and_then(|_guard| capacity::owners(m)).ok();
    let pending = pending_transactions(m)?;
    let operation = optional(&m.root.join("operator/latest.json"))?
        .as_object().map(|v|Value::Object(v.clone()));
    let operation_live = operation.as_ref().and_then(|v|v["state"].as_str())
        .is_some_and(operation_state_live);
    Ok(ui::Pulse {schema:ui::OPERATOR_SCHEMA,
        service_state:service_state.into(),
        dsp:owners.as_ref().map(|o|o.iter().filter(|v|v.kind == capacity::Kind::Dsp).count()),
        keepers:owners.as_ref().map(|o|o.iter().filter(|v|v.kind == capacity::Kind::Keeper).count()),
        maintenance:owners.as_ref().map(|o|o.iter().filter(|v|matches!(v.kind,
            capacity::Kind::Inspection | capacity::Kind::VendorAccess)).count()),
        pending_transactions:pending,
        cleanup_unconfirmed:pulse_cleanup(m),
        current_generation:pulse_generation(m)?,operation,operation_live})
}
fn inactive_reason(
    cap: Option<&CapacityReadback>,
    vendor_retired: bool,
    pending: usize,
    reconcile: bool,
) -> Option<&'static str> {
    match cap {
        None => Some("Service capacity unavailable; actions requiring inactivity are unsafe"),
        Some(c) if c.cleanup_unconfirmed => Some("Previous instance cleanup is unconfirmed"),
        Some(c) if c.dsp > 0 || c.maintenance > 0 => {
            Some("Close active bridged instances before this action")
        }
        _ if !vendor_retired => {
            Some("Close Arturia Software Center and wait for its owned operation to retire")
        }
        _ if pending > 0 && !reconcile => Some("Reconcile the interrupted publication first"),
        _ => None,
    }
}
pub(super) fn require_engineering_inactive(m: &Manager) -> Result<()> {
    let cap = live_capacity(m)?;
    let _guard = m.lock("registry.lock")?;
    require(
        cap.owners == capacity::owners(m)?,
        "operator_owners_changed",
    )?;
    if let Some(reason) = inactive_reason(
        Some(&cap),
        vendor_retired(m)? && onboarding::all_retired(m)?,
        pending_transactions(m)?,
        false,
    ) {
        return Err(reason.into());
    }
    m.require_inactive(None)
}
fn require_operator_inactive_with(
    m: &Manager,
    a: &ui::Action,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
    operation: Option<&str>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
) -> Result<()> {
    if !a.requires_inactive() {
        return Ok(());
    }
    let cap = capacity_read().ok_or("capacity_readback_unavailable")?;
    let _admission = acquire_readback(m, ui::OperatorLock::Registry, operation, timeout, waits)?;
    if let Some(reason) = inactive_reason(
        Some(&cap),
        vendor_retired(m)? && onboarding::all_retired(m)?,
        pending_transactions(m)?,
        matches!(a, ui::Action::TransactionReconcile {}),
    ) {
        return Err(reason.into());
    }
    // Recheck durable owners at the mutation boundary, under the same lock
    // used by service admission; a peer lost since LVC1 must not disappear.
    m.require_inactive(None)
}
fn history(m: &Manager, key: &str, entry: &Entry) -> Result<Vec<ui::History>> {
    let mut ancestors = std::collections::BTreeSet::new();
    let mut current = entry.managed_revision.clone();
    for _ in 0..256 {
        let Some(r) = current else { break };
        let revision = m.load_revision(key, &r)?;
        ancestors.insert(revision.id);
        current = revision.parent;
    }
    let mut result = Vec::new();
    let dir = m.root.join("publications").join(key).join("revisions");
    if !dir.exists() {
        return Ok(result);
    }
    for item in fs::read_dir(dir)?.take(257) {
        require(result.len() < 256, "operator_history_bound")?;
        let path = item?.path().join("revision.json");
        if !path.exists() {
            continue;
        }
        let raw: publication::Revision = read_json(&path)?;
        let reference = publication::RevisionRef {
            id: raw.id.clone(),
            sha256: digest(&path)?,
        };
        let r = m.load_revision(key, &reference)?;
        let ordinary =
            r.qualification.is_none() && r.profile.claim == profiles::Claim::VerifiedExactFixture;
        result.push(ui::History {
            revision: r.profile.revision,
            claim: serde_json::to_value(r.profile.claim)?
                .as_str()
                .unwrap_or("unknown")
                .into(),
            publication: r.id.clone(),
            active: entry.publication == Publication::Published
                && entry.managed_revision.as_ref() == Some(&reference),
            rollback_allowed: ordinary && ancestors.contains(&r.id),
        });
    }
    result.sort_by_key(|r| r.revision);
    Ok(result)
}
fn managed_environment_bindings(
    m: &Manager,
    catalogue: Option<&linux_vst_bridge::catalogue::Catalogue>,
    registry: &Registry,
) -> Result<Vec<linux_vst_bridge::catalogue::EnvironmentBinding>> {
    let mut bindings = catalogue.map_or_else(Vec::new, |c| c.environments.clone());
    for retained in onboarding::history_records(m)? {
        if !registry
            .classes
            .values()
            .any(|entry| entry.registration.environment.id == retained.environment.id)
        {
            continue;
        }
        let binding = linux_vst_bridge::catalogue::EnvironmentBinding {
            family: linux_vst_bridge::profiles::Family::ManagedInstallerV1,
            environment: retained.environment,
        };
        if let Some(existing) = bindings
            .iter()
            .find(|candidate| candidate.environment.id == binding.environment.id)
        {
            require(
                existing == &binding,
                "operator_managed_environment_binding_conflict",
            )?;
        } else {
            require(bindings.len() < 16, "operator_environment_bound")?;
            bindings.push(binding);
        }
    }
    if let Some(binding) = linux_vst_bridge::frg1::adopted_environment(m)? {
        if let Some(existing) = bindings.iter()
            .find(|candidate| candidate.environment.id == binding.environment.id) {
            require(existing == &binding,"operator_managed_environment_binding_conflict")?;
        } else {
            require(bindings.len() < 16,"operator_environment_bound")?;
            bindings.push(binding);
        }
    }
    Ok(bindings)
}
fn managed_rescan_binding_from(
    m: &Manager,
    bindings: &[linux_vst_bridge::catalogue::EnvironmentBinding],
    registry: &Registry,
    environment: &str,
) -> Result<Option<Environment>> {
    let env = &bindings
        .iter()
        .find(|candidate| candidate.environment.id == environment)
        .ok_or("operator_environment_absent")?
        .environment;
    let owners: Vec<_> = registry
        .classes
        .values()
        .filter(|entry| entry.registration.environment.id == environment)
        .collect();
    if owners.is_empty() {
        return Ok(linux_vst_bridge::frg1::adopted_environment(m)?
            .filter(|binding| binding.environment == *env)
            .map(|binding| binding.environment));
    }
    require(
        owners
            .iter()
            .all(|entry| entry.registration.environment == *env),
        "operator_managed_environment_mismatch",
    )?;
    if let Some(retained) = onboarding::retained_environment(m, environment)? {
        require(
            retained.environment == *env,
            "operator_onboarding_environment_mismatch",
        )?;
        require(
            onboarding::retired(&onboarding::result(m, &retained)?),
            "installer_retirement_required",
        )?;
    }
    Ok(Some(env.clone()))
}
fn quarantined_retry_disabled(m: &Manager,
    bindings: &[linux_vst_bridge::catalogue::EnvironmentBinding],
    registry: &Registry, environment: &Environment) -> Result<Option<&'static str>> {
    if !bindings.iter().any(|binding| binding.environment.id == environment.id) {
        return Ok(Some("This environment is not currently managed"));
    }
    match managed_rescan_binding_from(m, bindings, registry, &environment.id)? {
        Some(current) => {
            require(current == *environment,
                "operator_managed_environment_binding_conflict")?;
            Ok(None)
        }
        None => Ok(Some("This environment is not currently managed")),
    }
}
fn managed_rescan_binding(
    m: &Manager,
    catalogue: Option<&linux_vst_bridge::catalogue::Catalogue>,
    registry: &Registry,
    environment: &str,
) -> Result<Option<Environment>> {
    managed_rescan_binding_from(
        m,
        &managed_environment_bindings(m, catalogue, registry)?,
        registry,
        environment,
    )
}
#[cfg(test)]
pub(super) fn environment_projection(
    m: &Manager,
    sw: &Software,
    catalogue: Option<&linux_vst_bridge::catalogue::Catalogue>,
    registry: &Registry,
    busy: Option<&str>,
) -> Result<Vec<ui::Environment>> {
    let bindings = managed_environment_bindings(m, catalogue, registry)?;
    environment_projection_from(m, sw, &bindings, registry, busy)
}
fn environment_projection_from(
    m: &Manager,
    sw: &Software,
    bindings: &[linux_vst_bridge::catalogue::EnvironmentBinding],
    registry: &Registry,
    busy: Option<&str>,
) -> Result<Vec<ui::Environment>> {
    let adopted = linux_vst_bridge::frg1::adopted_environment(m)?;
    bindings
        .iter()
        .map(|entry| {
            let scan = optional(
                &m.root
                    .join("inventory")
                    .join(format!("{}.json", entry.environment.id)),
            )?;
            let actions = if let Some(environment) = managed_rescan_binding_from(
                m,
                bindings,
                registry,
                &entry.environment.id,
            )?
            {
                if onboarding::inventory_refresh_required(
                    m, &environment, &sw.host, &sw.source_sha256,
                )? || (adopted.as_ref().is_some_and(|binding| binding.environment == environment)
                    && linux_vst_bridge::frg1::inventory_refresh_required(m, &environment)?) {
                    vec![action(
                        "Refresh installed products",
                        ui::Action::EnvironmentRescan {
                            environment: environment.id,
                        },
                        busy,
                    )]
                } else {
                    vec![]
                }
            } else {
                vec![]
            };
            Ok(ui::Environment {
                id: entry.environment.id.clone(),
                family: format!("{:?}", entry.family),
                runner: entry.environment.runner.id.clone(),
                revision: entry.environment.revision,
                authorization: "Managed by the vendor and user; account state is not inspected"
                    .into(),
                last_scan: json!({"id":scan["id"],"completed_at":scan["completed_at"],
                    "module_count":scan["modules"].as_array().map(Vec::len),"changes":scan["changes"]}),
                actions,
            })
        })
        .collect()
}
// A fresh product install has no native catalogue until a managed native
// publication exists. An empty registry can bootstrap the first inventory;
// afterward only the exact sealed FRG1 publication/restoration remains valid
// without an ordinary catalogue.
fn operator_catalogue(m: &Manager, sw: &Software, registry: &Registry)
    -> Result<Option<linux_vst_bridge::catalogue::Catalogue>> {
    if sw.native_catalogue.is_none() {
        require(linux_vst_bridge::frg1::catalogue_free_registry(m, registry)?,
            "native_catalogue_absent_run_product_setup")?;
        Ok(None)
    } else {
        Ok(Some(sw.catalogue(m)?))
    }
}
const OPERATOR_WAIT: Duration = Duration::from_secs(10);
#[cfg(test)]
fn canonical_lock(m: &Manager) -> Result<Lock> {
    Ok(m.lock_bounded(
        ui::OperatorLock::Canonical,
        ui::LockPurpose::ActionSerialization,
        None,
        OPERATOR_WAIT,
    )?
    .0)
}
fn snapshot(m: &Manager) -> Result<ui::Snapshot> {
    snapshot_for_operation(m, None, OPERATOR_WAIT, &mut vec![], &|| {
        live_capacity(m).ok()
    })
}
fn overview(m: &Manager) -> Result<ui::InteractiveOverview> {
    linux_vst_bridge::with_readback_digests(|| {
        std::thread::scope(|scope| {
        let probe = scope.spawn(|| {
            let started = Instant::now();
            (readiness::collect(m), started.elapsed().as_millis())
        });
        let captured = current::capture(m)?;
        let (platform, probe_ms) = probe.join().map_err(|_| "platform_probe_failed")?;
        #[cfg(feature = "pb0-c0-audit")]
        eprintln!("PB0_PHASE {}",json!({"platform_probes_ms":probe_ms}));
        #[cfg(not(feature = "pb0-c0-audit"))]
        let _ = probe_ms;
        let resolve_at = Instant::now();
        let readiness = readiness::resolve_captured(&captured.snapshot, &platform,
            &captured.profiles, &captured.revisions, observation::now()?);
        #[cfg(feature = "pb0-c0-audit")]
        eprintln!("PB0_PHASE {}",json!({"readiness_resolution_ms":resolve_at.elapsed().as_millis()}));
        #[cfg(not(feature = "pb0-c0-audit"))]
        let _ = resolve_at;
        captured.recheck(m)?;
        let generation = captured.current_generation.clone();
        let current = captured.snapshot;
        Ok(ui::InteractiveOverview {schema: 1, operator_schema: ui::OPERATOR_SCHEMA,
            scope: "current_only".into(),current_generation:generation,
            current, readiness})
        })
    })
}
fn product_detail(m: &Manager, environment: &str, module: &str,
    class: &str) -> Result<ui::CurrentProductDetail> {
    require(valid_product_environment(environment) && valid_hex(module, 64)
        && (class.is_empty() || valid_hex(class, 32)), "operator_product_identity")?;
    let captured = current::capture(m)?;
    let sw = software(m)?;
    let db = m.registry()?;
    let product = project_current_product(m, &captured, &sw, &db, environment, module, class)?;
    let (environments, vendor_applications) = scoped_product_context(m, &sw, &db,
        environment, captured.busy)?;
    captured.recheck(m)?;
    Ok(ui::CurrentProductDetail {schema:1,operator_schema:ui::OPERATOR_SCHEMA,
        state_token:captured.snapshot.state_token,
        current_generation:captured.current_generation,product,environments,
        vendor_applications})
}
fn valid_product_environment(id: &str) -> bool {
    valid_hex(id, 32) || (id.len() == 36 && id.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    }))
}
fn project_current_product(m: &Manager, captured: &current::CurrentOverviewContext,
    sw: &Software, db: &Registry,
    environment: &str, module: &str, class: &str) -> Result<ui::Product> {
    let mut matching = captured.snapshot.products.iter().filter(|product|
        product.environment == environment && product.module_sha256 == module
            && product.class_id == class);
    let mut product = matching.next().ok_or("operator_product_not_current")?.clone();
    require(matching.next().is_none(), "operator_product_ambiguous")?;
    if let Some(entry) = db.classes.get(class).filter(|entry|
        entry.registration.environment.id == environment
            && entry.registration.module.sha256 == module) {
        product.history = history(m, class, entry)?;
        for prior in &product.history {
            if prior.rollback_allowed && !prior.active {
                product.actions.push(action(&format!("Roll back to revision {}",prior.revision),
                    ui::Action::OrdinaryRollback {class_id:class.into(),
                        publication:prior.publication.clone()}, captured.busy));
            }
        }
        if captured.profiles.iter().any(|profile|
            profile.class.class_id == class && Some(profile.revision) != product.active_revision) {
            product.actions.push(action("Restore recommended revision",
                ui::Action::OrdinaryRestoreRecommended {class_id:class.into()}, captured.busy));
        }
        product.actions.push(action("Arm crash capture for next launch",
            ui::Action::CaptureArm {class_id:class.into()},
            if product.details["publication_valid"] != true
                || product.details["publication_selected"] != true
                || !product.details["qualification"].is_null() {
                Some("An exact ordinary publication is required")
            } else { None }));
    }
    preparation_cli::project(m, sw,
        std::slice::from_mut(&mut product), captured.busy)?;
    Ok(product)
}
fn scoped_product_context(m: &Manager, sw: &Software, db: &Registry,
    environment: &str, busy: Option<&str>)
    -> Result<(Vec<ui::Environment>,Vec<ui::VendorApplication>)> {
    let catalogue = operator_catalogue(m, sw, db)?;
    let bindings = managed_environment_bindings(m, catalogue.as_ref(), db)?;
    let exact = bindings.into_iter().filter(|binding|
        binding.environment.id == environment).collect::<Vec<_>>();
    let environments = environment_projection_from(m, sw, &exact, db, busy)?;
    let arturia = app_directory(m).join("application.json");
    let mut vendor_applications = if arturia.exists()
        && read_json::<vendor_application::Application>(&arturia)?.environment.id == environment {
        asc_projection(m, busy)?.into_iter().collect::<Vec<_>>()
    } else { vec![] };
    if environment == renderer::ENVIRONMENT {
        if let Some(mut app) = renderer_cli::project(m, busy)? {
            dependency_cli::project(m, &mut app, busy)?;
            vendor_applications.push(app);
        }
    }
    Ok((environments,vendor_applications))
}
#[cfg(test)]
pub(super) fn snapshot_idle_test(m: &Manager) -> Result<ui::Snapshot> {
    snapshot_for_operation(m, None, OPERATOR_WAIT, &mut vec![], &idle_capacity_test)
}
#[cfg(test)]
fn idle_capacity_test() -> Option<CapacityReadback> {
    serde_json::from_value(json!({
        "schema": 1, "dsp": 0, "maintenance": 0, "keepers": 0,
        "cleanup_unconfirmed": false, "owners": [], "limits": {"global_dsp": 6}
    })).ok()
}
#[cfg(test)]
pub(super) fn test_submit_offered(m: &Manager, action: &ui::Action) -> Result<String> {
    let snapshot = snapshot_idle_test(m)?;
    let request = ui::Request { schema: ui::OPERATOR_SCHEMA,
        state_token: snapshot.state_token.clone(), action: action.clone() };
    validate(&request, &snapshot)?;
    launch_queued(m, &request, |_| Ok(true))?.operation.ok_or_else(||
        "operator_test_operation_missing".into())
}
#[cfg(test)]
pub(super) fn test_run_offered_worker(m: &Manager, operation: &str) -> Result<Value> {
    worker_with_capacity(m, operation, OPERATOR_WAIT, &idle_capacity_test)?;
    read_json(&job_dir(m, operation)?.join("result.json"))
}
#[cfg(test)]
pub(super) fn test_finalize_interrupted_worker(m: &Manager, operation: &str) -> Result<Value> {
    finish_operation(m, operation)?;
    read_json(&job_dir(m, operation)?.join("result.json"))
}
fn acquire_readback(
    m: &Manager,
    name: ui::OperatorLock,
    id: Option<&str>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
) -> Result<Lock> {
    let purpose = match name {
        ui::OperatorLock::Canonical => ui::LockPurpose::ActionSerialization,
        ui::OperatorLock::Registry if id.is_some() => ui::LockPurpose::OperatorValidationReadback,
        ui::OperatorLock::Registry => ui::LockPurpose::OperatorReadback,
        ui::OperatorLock::Receipt => ui::LockPurpose::OperationReceipt,
        ui::OperatorLock::Resume => ui::LockPurpose::ServiceRecovery,
    };
    match m.lock_bounded(name, purpose, id, timeout) {
        Ok((lock, facts)) => {
            waits.push(facts);
            Ok(lock)
        }
        Err(e) => {
            if let Some(f) = e.downcast_ref::<linux_vst_bridge::operator_lock::AcquisitionFailure>()
            {
                waits.push(f.facts.clone());
            }
            Err(e)
        }
    }
}
fn snapshot_for_operation(
    m: &Manager,
    id: Option<&str>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
) -> Result<ui::Snapshot> {
    snapshot_for_operation_depth(m, id, timeout, waits, capacity_read, true)
}
fn snapshot_for_operation_depth(
    m: &Manager,
    id: Option<&str>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
    deep: bool,
) -> Result<ui::Snapshot> {
    linux_vst_bridge::runtime_delivery::prepare_readback(m)?;
    linux_vst_bridge::with_readback_digests(||
        snapshot_readonly_depth(m,id,timeout,waits,capacity_read,deep))
}
fn snapshot_readonly_depth(
    m: &Manager,
    id: Option<&str>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
    deep: bool,
) -> Result<ui::Snapshot> {
    // Bounded wait order: operator serialization -> registry authority.
    let _projection = acquire_readback(m, ui::OperatorLock::Canonical, id, timeout, waits)?;
    // History migration can change the projection token. Complete it before
    // sampling registry/capacity authority; never hash provenance under registry.lock.
    if deep { linux_vst_bridge::preparation::materialize_retained_history(m)?; }
    // LVC1 itself takes registry.lock in the service. Never request it while
    // holding that lock. Its owner census must still match after acquisition.
    let deadline = Instant::now() + timeout;
    let mut cap = capacity_read();
    let mut registry = acquire_readback(
        m,
        ui::OperatorLock::Registry,
        id,
        deadline.saturating_duration_since(Instant::now()),
        waits,
    )?;
    if cap.is_none() {
        // LVC1 may itself have lost the same fail-fast registry race. After
        // contention clears, resample once without owning its required lock.
        // Unreachable/malformed/blocked service still never becomes healthy.
        drop(registry);
        cap = capacity_read();
        registry = acquire_readback(
            m,
            ui::OperatorLock::Registry,
            id,
            deadline.saturating_duration_since(Instant::now()),
            waits,
        )?;
    }
    let owners = capacity::owners(m)?;
    let cap = cap.filter(|c| c.owners == owners);
    let before = token(m)?;
    let db = m.registry()?;
    drop(registry);
    // Digests, runner verification, systemd and presentation run outside the
    // registry lock. The captured registry is checked again after projection.
    let sw = software(m)?;
    let profiles = profiles::installed_profiles()?;
    let canonical = m.project_managed_registry(
        &sw.host,
        &sw.source_sha256,
        &profiles,
        &db,
    )?;
    let pending = pending_transactions(m)?;
    let retired = vendor_retired(m)? && onboarding::all_retired(m)?;
    let busy = inactive_reason(cap.as_ref(), retired, pending, false);
    let mut products = Vec::new();
    for p in canonical.products {
        let entry = db
            .classes
            .get(&p.class_id)
            .ok_or("operator_registry_changed")?;
        let hist = if deep { history(m, &p.class_id, entry)? } else { vec![] };
        let recommended = profiles.iter().find(|r| r.class.class_id == p.class_id);
        let mut actions = Vec::new();
        for h in &hist {
            if h.rollback_allowed && !h.active {
                actions.push(action(
                    &format!("Roll back to revision {}", h.revision),
                    ui::Action::OrdinaryRollback {
                        class_id: p.class_id.clone(),
                        publication: h.publication.clone(),
                    },
                    busy,
                ));
            }
        }
        let active_revision = p.profile.as_ref().map(|p| p.revision);
        if recommended.is_some_and(|r| Some(r.revision) != active_revision) {
            actions.push(action(
                "Restore recommended revision",
                ui::Action::OrdinaryRestoreRecommended {
                    class_id: p.class_id.clone(),
                },
                busy,
            ));
        }
        actions.push(action(
            "Arm crash capture for next launch",
            ui::Action::CaptureArm {
                class_id: p.class_id.clone(),
            },
            if !p.publication_valid || p.publication != Publication::Published || p.qualification.is_some() {
                Some("An exact ordinary publication is required")
            } else {
                None
            },
        ));
        products.push(ui::Product {class_id:p.class_id.clone(),name:p.name.clone(),vendor:entry.registration.metadata.vendor.clone(),role:serde_json::to_value(&p.role)?.as_str().unwrap_or("unknown").into(),version:p.build.clone(),disposition:if p.refusal.is_none() && p.publication_valid && p.publication == Publication::Published {"ready"} else {"needs_attention"}.into(),active_revision,recommended_revision:recommended.map(|p|p.revision),environment:p.environment.clone(),runner:p.runner.clone(),module_sha256:p.module_sha256.clone(),limitations:serde_json::to_value(&p.limitations)?.as_array().map(|a|a.iter().filter_map(|v|v.as_str().map(Into::into)).collect()).unwrap_or_default(),history:hist,actions,compatibility:None,details:json!({"external_ids":p.external_ids,"profile":p.profile,"publication":p.active_revision,"module_valid":p.module_valid,"environment_valid":p.environment_valid,"environment_revision":p.environment_revision,"runner_valid":p.runner_valid,"native_valid":p.native_artifact_valid,"host_valid":p.installed_host_valid,"publication_valid":p.publication_valid,"publication_selected":p.publication == Publication::Published,"host_sha256":entry.registration.host.sha256,"host_source_sha256":entry.registration.host_source_sha256,"native_sha256":entry.registration.native.sha256,"qualification":p.qualification,"capabilities":p.capabilities,"compatibility":p.compatibility,"recommended_frames":p.recommended_frames,"performance":p.performance,"refusal":p.refusal})});
    }
    let catalogue = operator_catalogue(m, &sw, &db)?;
    let managed_bindings = managed_environment_bindings(m, catalogue.as_ref(), &db)?;
    let environments = environment_projection_from(m, &sw, &managed_bindings, &db, busy)?;
    let mut inventory_environments: Vec<Environment> = managed_bindings.iter()
        .map(|e| e.environment.clone()).collect();
    for retained in onboarding::history_records(m)? {
        if let Some(current) = inventory_environments.iter()
            .find(|environment| environment.id == retained.environment.id) {
            require(current == &retained.environment,
                "operator_managed_environment_binding_conflict")?;
        } else {
            inventory_environments.push(retained.environment);
        }
    }
    for env in &inventory_environments {
        let retry_disabled = quarantined_retry_disabled(m, &managed_bindings, &db, env)?;
        let path = m.root.join("inventory").join(format!("{}.json", env.id));
        if path.exists() {
            let scan: inventory::Scan = read_json(&path)?;
            require(
                scan.schema == 1 && scan.environment.id == env.id,
                "inventory_environment_binding",
            )?;
            for (module_index, module) in scan.modules.iter().cloned().enumerate() {
                let stale = inventory::stale_reason(
                    &module,
                    &scan.environment,
                    &scan.host,
                    &scan.host_source_sha256,
                    env,
                    &sw.host,
                    &sw.source_sha256,
                );
                if module.quarantine_reason.is_some() {
                    products.push(quarantined_product(&scan,module_index,module,stale,busy,
                        retry_disabled)?);
                    continue;
                }
                let current = stale.is_none();
                let inspection_hint = if current {
                    inventory::inspection_hint(&module).ok().flatten()
                } else {
                    None
                };
                for class in module.classes {
                    if class.category != "Audio Module Class"
                        || products.iter().any(|p| {
                            p.class_id == class.id
                                && p.module_sha256 == module.artifact.sha256
                                && p.environment == scan.environment.id
                        })
                    {
                        continue;
                    }
                    products.push(ui::Product {class_id:class.id,name:class.name,vendor:class.vendor,role:class.role,version:class.version,disposition:if current {"installed_unqualified"} else {"needs_attention"}.into(),active_revision:None,recommended_revision:None,environment:scan.environment.id.clone(),runner:scan.environment.runner.id.clone(),module_sha256:module.artifact.sha256.clone(),limitations:vec![stale.unwrap_or("Installed — not yet supported; not published to Bitwig").into()],history:vec![],actions:vec![],compatibility:None,details:json!({"scan":scan.id,"observed_at":scan.completed_at,"scanner_host_sha256":scan.host.sha256,"scanner_source_sha256":scan.host_source_sha256,"current":current,"inspection_error":module.inspection_error,"inspection_hint":inspection_hint,"activation_permitted":false})});
                }
            }
        }
    }
    if deep {
        if let Some(operation) = id {
            preparation_cli::project_for_operation(m, &sw, &mut products, busy, operation)?;
        } else {
            preparation_cli::project(m, &sw, &mut products, busy)?;
        }
    }
    let mut vendor_applications = asc_projection(m, busy)?.into_iter().collect::<Vec<_>>();
    if deep {
        if let Some(mut app) = renderer_cli::project(m, busy)? {
            dependency_cli::project(m, &mut app, busy)?;
            vendor_applications.push(app);
        }
    }
    let mut incidents = Vec::new();
    let mut paths = if deep { crash_capture::incidents(m)? } else { vec![] };
    if deep { paths.sort_by_key(|p| {
        fs::metadata(p.join("status.json"))
            .and_then(|m| m.modified())
            .ok()
    }); }
    for path in paths {
        let id = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("incident_identity")?
            .to_owned();
        let status = optional(&path.join("status.json"))?;
        let share = optional(&path.join("share.json"))?;
        incidents.push(ui::Incident {
            id: id.clone(),
            state: text(&status, "state"),
            export: if share.is_null() {
                None
            } else {
                Some(action(
                    "Export sanitized report",
                    ui::Action::IncidentExport { incident: id },
                    None,
                ))
            },
            summary: share,
        });
    }
    let live = activity_with_capacity(m, cap.as_ref())?;
    let mut onboarding = onboarding::projection(m, busy)?;
    let (workspaces, workspace_installers) = daw_workspace::projection(m, &onboarding)?;
    onboarding.retain(|row| !workspace_installers.contains(&row.installer));
    if let Some(receipt) = &live.operation {
        project_onboarding_failure(m, receipt, &mut onboarding)?;
    }
    let installer_setups = onboarding::setup_projection(m, &onboarding, &products, &workspace_installers)?;
    let recheck = acquire_readback(m, ui::OperatorLock::Registry, id, timeout, waits)?;
    let after = token(m)?;
    require(
        before == after && owners == capacity::owners(m)?,
        "operator_state_changed_refresh",
    )?;
    drop(recheck);
    let active_sessions=if deep { session_projection(cap.as_ref(),capacity::terminal_summaries(m)?) }
        else { vec![] };
    Ok(ui::Snapshot {
        onboarding,
        installer_setups,
        schema: ui::OPERATOR_SCHEMA,
        state_token: after,
        system: live.system,
        environments,
        vendor_applications,
        products,
        workspaces,
        active_sessions,
        capture: capture_state(m)?,
        recent_incidents: incidents,
        actions: vec![
            action("Install compatibility runtime (728 MB download)", ui::Action::RuntimeInstall {},
                if linux_vst_bridge::runtime_delivery::installed(m)?.is_some() {
                    Some("The selected compatibility runtime is already installed")
                } else {busy}),
            action("Create sanitized support export", ui::Action::SupportExport {}, None),
            action("Disarm crash capture", ui::Action::CaptureDisarm {}, None),
            action(
                "Reconcile interrupted transaction",
                ui::Action::TransactionReconcile {},
                inactive_reason(cap.as_ref(),retired,pending,true),
            ),
        ],
        operation: optional(&m.root.join("operator/latest.json"))?
            .as_object()
            .map(|v| Value::Object(v.clone())),
    })
}
fn project_onboarding_failure(
    m: &Manager,
    receipt: &Value,
    rows: &mut [ui::Onboarding],
) -> Result<()> {
    if receipt["state"] != "refused" {
        return Ok(());
    }
    let id = receipt["operation"]
        .as_str()
        .ok_or("operator_failure_identity")?;
    let request: ui::Request = read_json(&job_dir(m, id)?.join("request.json"))?;
    let failure: Option<ui::OperationFailure> = if receipt["failure"].is_null() {
        None
    } else {
        Some(serde_json::from_value(receipt["failure"].clone())?)
    };
    let matches = |r: &&mut ui::Onboarding| match &request.action {
        ui::Action::InstallerEnvironmentCreate { installer, .. } => {
            r.installer == *installer && r.environment.is_none()
        }
        ui::Action::InstallerNewAttempt { previous, .. } => {
            r.environment.as_ref() == Some(previous)
        }
        _ => false,
    };
    {
        for row in rows.iter_mut().filter(matches) {
            row.failure = failure.clone();
            if receipt["stage"] == "operator_request_admission"
                && receipt["worker_started"] == false
            {
                row.details["request_result"] = json!({"operation":id,"stage":"operator_request_admission","reason":receipt["reason"],"worker_started":false});
            }
        }
    }
    Ok(())
}
pub(super) fn available(snapshot: &ui::Snapshot) -> Vec<&ui::AvailableAction> {
    snapshot
        .actions
        .iter()
        .chain(snapshot.installer_setups.iter().map(|s| &s.rename))
        .chain(snapshot.onboarding.iter().flat_map(|p| p.actions.iter()))
        .chain(snapshot.products.iter().flat_map(|p| p.actions.iter()))
        .chain(snapshot.products.iter().flat_map(|p| p.compatibility.iter()
            .flat_map(|workflow| workflow.primary.iter().chain(workflow.alternatives.iter()))))
        .chain(snapshot.workspaces.iter().flat_map(|w| w.actions.iter()))
        .chain(snapshot.workspaces.iter().flat_map(|w| w.installer_choices.iter()))
        .chain(snapshot.workspaces.iter().flat_map(|w| w.products.iter())
            .flat_map(|p| p.actions.iter().chain(p.installer_choices.iter())))
        .chain(snapshot.environments.iter().flat_map(|p| p.actions.iter()))
        .chain(
            snapshot
                .vendor_applications
                .iter()
                .flat_map(|p| p.actions.iter()),
        )
        .chain(
            snapshot
                .recent_incidents
                .iter()
                .filter_map(|p| p.export.as_ref()),
        )
        .collect()
}
fn validate(request: &ui::Request, snapshot: &ui::Snapshot) -> Result<()> {
    require(request.schema == ui::OPERATOR_SCHEMA,
        "operator_schema_mismatch_update_manager_frontend")?;
    require(
        request.state_token == snapshot.state_token,
        "operator_stale_request_refresh",
    )?;
    let offered = available(snapshot)
        .into_iter()
        .find(|a| preparation_cli::offered(&request.action, &a.action))
        .ok_or("operator_action_not_available")?;
    require(
        offered.disabled_reason.is_none(),
        offered
            .disabled_reason
            .as_deref()
            .unwrap_or("operator_action_disabled"),
    )
}
fn validate_current_request(m: &Manager, request: &ui::Request) -> Result<()> {
    require(request.schema == ui::OPERATOR_SCHEMA,
        "operator_schema_mismatch_update_manager_frontend")?;
    if current_offer_action(&request.action) {
        let offered = overview(m)?;
        validate(request, &offered.current)?;
        if matches!(request.action, ui::Action::SupportExport {}) {
            let export = &offered.readiness.support_export_action;
            require(matches!(export.action, ui::Action::SupportExport {})
                && export.disabled_reason.is_none(), "operator_support_export_not_offered")?;
        }
        return Ok(());
    }
    if preparation_cli::is_action(&request.action) {
        let captured = current::capture(m)?;
        let sw = software(m)?;
        let db = m.registry()?;
        let selection = preparation_cli::action_selection(m, &sw,
            &request.action)?.ok_or("operator_product_action_identity")?;
        let product = project_current_product(m, &captured, &sw, &db, &selection.environment.id,
            &selection.module.sha256, &selection.class.id)?;
        captured.recheck(m)?;
        let mut offered = captured.snapshot;
        offered.products = vec![product];
        return validate(request, &offered);
    }
    if scoped_product_action(&request.action) {
        let captured = current::capture(m)?;
        let sw = software(m)?;
        let db = m.registry()?;
        let mut offered = captured.snapshot.clone();
        match &request.action {
            ui::Action::OrdinaryRollback { class_id, .. }
            | ui::Action::OrdinaryRestoreRecommended { class_id }
            | ui::Action::CaptureArm { class_id } => {
                let entry = db.classes.get(class_id).ok_or("operator_product_not_current")?;
                offered.products = vec![project_current_product(m, &captured, &sw, &db,
                    &entry.registration.environment.id, &entry.registration.module.sha256,
                    class_id)?];
            }
            ui::Action::EnvironmentRescan { environment } => {
                require(valid_product_environment(environment), "operator_product_identity")?;
                offered.environments = scoped_product_context(m, &sw, &db,
                    environment, captured.busy)?.0;
            }
            ui::Action::VendorApplicationOpen { application }
            | ui::Action::VendorApplicationFocus { application }
            | ui::Action::VendorApplicationStop { application } if application == ASC => {
                offered.vendor_applications = asc_projection(m, captured.busy)?
                    .into_iter().collect();
            }
            ui::Action::RendererOpen { .. } | ui::Action::RendererFocus { .. }
            | ui::Action::RendererStop { .. } => {
                offered.vendor_applications = scoped_product_context(m, &sw, &db,
                    renderer::ENVIRONMENT, captured.busy)?.1;
            }
            _ => return Err("operator_product_action_identity".into()),
        }
        captured.recheck(m)?;
        return validate(request, &offered);
    }
    validate(request, &snapshot(m)?)
}
fn scoped_product_action(action: &ui::Action) -> bool {
    matches!(action, ui::Action::OrdinaryRollback { .. }
        | ui::Action::OrdinaryRestoreRecommended { .. }
        | ui::Action::CaptureArm { .. }
        | ui::Action::EnvironmentRescan { .. }
        | ui::Action::VendorApplicationOpen { .. }
        | ui::Action::VendorApplicationFocus { .. }
        | ui::Action::VendorApplicationStop { .. }
        | ui::Action::RendererOpen { .. }
        | ui::Action::RendererFocus { .. }
        | ui::Action::RendererStop { .. })
}
fn current_offer_action(action: &ui::Action) -> bool {
    matches!(action, ui::Action::RuntimeInstall {} | ui::Action::SupportExport {}
        | ui::Action::TransactionReconcile {}
        | ui::Action::InstallerEnvironmentCreate { .. }
        | ui::Action::InstallerRename { .. }
        | ui::Action::InstallerNewAttempt { .. }
        | ui::Action::InstallerStart { .. }
        | ui::Action::InstallerStartWithPolicy { .. }
        | ui::Action::InstallerFocus { .. }
        | ui::Action::InstallerStop { .. }
        | ui::Action::InstallerScan { .. }
        | ui::Action::QuarantinedModuleRetry { .. })
}
fn validate_current_worker(m: &Manager, request: &ui::Request, id: &str,
    timeout: Duration, waits: &mut Vec<ui::LockFacts>,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>) -> Result<()> {
    require(request.schema == ui::OPERATOR_SCHEMA,
        "operator_schema_mismatch_update_manager_frontend")?;
    let _serialization = acquire_readback(m, ui::OperatorLock::Canonical,
        Some(id), timeout, waits)?;
    let deadline = Instant::now() + timeout;
    let mut cap = capacity_read();
    let mut registry = acquire_readback(m, ui::OperatorLock::Registry,
        Some(id), deadline.saturating_duration_since(Instant::now()), waits)?;
    if cap.is_none() {
        drop(registry);
        cap = capacity_read();
        registry = acquire_readback(m, ui::OperatorLock::Registry,
            Some(id), deadline.saturating_duration_since(Instant::now()), waits)?;
    }
    let owners = capacity::owners(m)?;
    require(cap.as_ref().is_none_or(|read| read.owners == owners),
        "operator_owners_changed")?;
    require(request.state_token == token(m)?, "operator_stale_request_refresh")?;
    drop(registry);
    Ok(())
}
fn job_dir(m: &Manager, id: &str) -> Result<PathBuf> {
    require(valid_hex(id, 32), "operator_operation_identity")?;
    Ok(m.root.join("operator").join(id))
}
/// A continuation never revives the original worker. Its exact request and
/// terminal refusal remain historical authority while a fresh offered request
/// completes only the missing preparation stages.
pub(super) fn resumable_check_source(m: &Manager, id: &str, action: &ui::Action) -> Result<bool> {
    require(matches!(action, ui::Action::CompatibilityCheck { .. }),
        "guided_check_source_action")?;
    let dir = job_dir(m, id)?;
    let original: ui::Request = read_json(&dir.join("request.json"))?;
    // The installed private UI2 generation used wire 11. Its retained request
    // can identify an old check; the *new* continuation still enters through
    // this manager's current schema and fresh offered-action validation.
    require(matches!(original.schema, 10..=13) && original.action == *action,
        "guided_check_source_request_changed")?;
    let result = optional(&dir.join("result.json"))?;
    require(result["schema"] == 1 && result["operation"] == id,
        "guided_check_source_receipt_changed")?;
    match result["state"].as_str() {
        Some("refused") => Ok(true),
        Some("queued" | "waiting" | "running" | "unconfirmed") => Ok(false),
        _ => Err("guided_check_source_not_interrupted".into()),
    }
}
fn write_operation(m: &Manager, id: &str, value: &Value, make_latest: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let _lock = loop {
        match m.lock("operator-receipt.lock") {
            Ok(lock) => break lock,
            Err(e)
                if e.is::<linux_vst_bridge::operator_lock::LockBusy>()
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(e) => return Err(e),
        }
    };
    let dir = job_dir(m, id)?;
    let prior = optional(&dir.join("result.json"))?;
    if matches!(prior["state"].as_str(), Some("completed" | "refused")) {
        let latest = m.root.join("operator/latest.json");
        if optional(&latest)?["operation"] == id {
            atomic_json(&latest, &prior)?;
        }
        return Ok(());
    }
    // Later vendor/cleanup owners retain the completed validation wait.
    let mut value = value.clone();
    if value.get("lock_waits").is_none() && prior.get("lock_waits").is_some() {
        value["lock_waits"] = prior["lock_waits"].clone();
    }
    atomic_json(&dir.join("result.json"), &value)?;
    let latest = m.root.join("operator/latest.json");
    if make_latest || optional(&latest)?["operation"] == id {
        atomic_json(&latest, &value)?;
    }
    Ok(())
}
fn refuse_unfinished(m: &Manager, id: &str, reason: &str) -> Result<()> {
    write_operation(
        m,
        id,
        &json!({"schema":1,"operation":id,"state":"refused","reason":reason}),
        false,
    )
}
fn wait_renderer_retirement(mut retired: impl FnMut() -> Result<bool>, mut wait: impl FnMut()) -> Result<()> {
    while !retired()? { wait(); }
    Ok(())
}
fn native_access_restoration_value(value:&ui::Activity,resume_pending:bool)->Result<Value> {
    let s=&value.system;
    require(s.service=="active" && s.keepers==2 && s.dsp==0 && s.maintenance==0
        && s.pending_transactions==0 && s.stale_transports==0 && !s.cleanup_unconfirmed
        && !resume_pending && value.capture["armed"]==false && value.capture["active_retention"]==0,
        "renderer_bridge_recovery_unconfirmed")?;
    Ok(json!({"bridge":"active","keepers":s.keepers,"dsp_leases":s.dsp,
        "maintenance_leases":s.maintenance,"pending_transactions":s.pending_transactions,
        "stale_transports":s.stale_transports,"pending_resume_owner":false,"capture":"off"}))
}
fn native_access_restoration(m:&Manager)->Result<Value> {
    native_access_restoration_value(&activity(m)?,resume_record(m)?.is_some())
}
fn native_access_session_summary(result:&Value,recovery:&Value)->Result<Value> {
    let state=result["state"].as_str().ok_or("renderer_session_state")?;
    require(matches!(state,"completed"|"cancelled"|"failed"),"renderer_session_state")?;
    let dependency=&result["dependency"];
    let disposition=dependency["dependency_cleanup_disposition"].as_str()
        .ok_or("renderer_dependency_cleanup_disposition")?;
    let cleanup_confirmed=match disposition {
        "graceful_service_retirement"=>dependency["service_retirement_confirmed"]==true
            && dependency["process_cleanup_confirmed"]==true
            && dependency["forced_cleanup_used"]==false,
        "exact_owned_session_cleanup"=>dependency["service_retirement_confirmed"]==false
            && dependency["process_cleanup_confirmed"]==true
            && dependency["forced_cleanup_used"]==true
            && dependency["service_stop_request_count"].as_u64().is_some_and(|v|v<=1)
            && dependency["stop_observation"]["classification"]!="NAD2_STOP_OBSERVATION_UNAVAILABLE"
            && dependency["post_cleanup"]["cgroup_empty"]==true
            && dependency["post_cleanup"]["exact_daemon_generation_absent"]==true
            && dependency["post_cleanup"]["listener_mask"]==0
            && dependency["post_cleanup"]["candidate_count"]==0
            && dependency["post_cleanup"]["candidate_observation_unavailable"]==0
            && dependency["post_cleanup"]["application_image_unchanged"]==true
            && dependency["post_cleanup"]["daemon_image_unchanged"]==true,
        "cleanup_unconfirmed"=>false,
        _=>return Err("renderer_dependency_cleanup_disposition".into()),
    };
    require(state=="failed" || cleanup_confirmed,"renderer_session_cleanup_unconfirmed")?;
    require(recovery["bridge"]=="active" && recovery["keepers"]==2
        && recovery["dsp_leases"]==0 && recovery["maintenance_leases"]==0
        && recovery["pending_transactions"]==0 && recovery["stale_transports"]==0
        && recovery["pending_resume_owner"]==false && recovery["capture"]=="off",
        "renderer_bridge_recovery_unconfirmed")?;
    Ok(json!({"state":state,"dependency_cleanup_disposition":disposition,
        "fresh_session_restartable":cleanup_confirmed,"bridge_recovery":recovery}))
}

fn finish_operation(m: &Manager, id: &str) -> Result<()> {
    let installer_cleanup: Result<()> = (|| {
        let request: ui::Request = read_json(&job_dir(m, id)?.join("request.json"))?;
        if let ui::Action::InstallerStart {
            onboarding: ref key,
        }
        | ui::Action::InstallerStartWithPolicy {
            onboarding: ref key,
            ..
        } = request.action
        {
            let r = onboarding::load(m, key)?;
            if r.installation_operation.as_deref() == Some(id) {
                if onboarding::live(id)? {
                    let _ = onboarding::stop(m, key, id);
                }
                onboarding::mark_dead(m, &r)?;
            }
        }
        Ok(())
    })();
    let preparation_cleanup = cleanup_preparation_work(m, id);
    // A failed installer readback must not leave the generic worker queued.
    let finalized = finish_operation_with(m, id, |saved| restore_service(m, saved));
    installer_cleanup?;
    preparation_cleanup?;
    finalized
}
fn cleanup_preparation_work(m: &Manager, id: &str) -> Result<()> {
    let fresh = preparation::build::cleanup_work(m, id);
    let source = (|| {
        let request: ui::Request = read_json(&job_dir(m, id)?.join("request.json"))?;
        if let ui::Action::CompatibilityResumeCheck { operation } = request.action {
            require(matches!(request.schema, 10..=13) && operation != id,
                "guided_check_cleanup_request_binding")?;
            let intent = preparation::guided_check_stage(m, &operation, "intent")?
                .ok_or("guided_check_cleanup_intent_missing")?;
            require(intent["schema"] == 1 && intent["operation"] == operation,
                "guided_check_cleanup_intent_binding")?;
            let original: ui::Action = serde_json::from_value(intent["action"].clone())?;
            require(resumable_check_source(m, &operation, &original)?,
                "guided_check_source_still_running")?;
            // ExecStopPost runs after this worker cgroup retires. Construction
            // used the original check's identity, not this fresh request's ID.
            // Cleanup preserves exact checkpointed/retained native output and
            // removes partial scratch before another offered continuation.
            preparation::build::cleanup_work(m, &operation)?;
        }
        Ok(())
    })();
    fresh?;
    source
}

fn finish_operation_with(
    m: &Manager,
    id: &str,
    restore: impl FnOnce(&ResumeRecord) -> Result<()>,
) -> Result<()> {
    // Reporting must not prevent this operation's service/keeper recovery.
    let finalized = refuse_unfinished(m, id, "operator_worker_terminated");
    let recovered = resume_owned_with(m, id, restore);
    finalized?;
    recovered
}
fn begin_request(m: &Manager, request: &ui::Request) -> Result<String> {
    let id = random_id()?;
    let dir = job_dir(m, &id)?;
    private_dir(&dir)?;
    atomic_json(&dir.join("request.json"), request)?;
    write_operation(
        m,
        &id,
        &json!({"schema":1,"operation":id,"state":"unconfirmed","stage":"operator_request_admission","action":request.action,"worker_started":false,"reason":"Request received; dispatch completion is not confirmed"}),
        true,
    )?;
    Ok(id)
}
#[cfg(test)]
fn launch_queued(
    m: &Manager,
    request: &ui::Request,
    launch: impl FnOnce(&str) -> Result<bool>,
) -> Result<ui::Receipt> {
    let id = begin_request(m, request)?;
    launch_reserved(m, request, &id, launch)
}
fn launch_reserved(
    m: &Manager,
    request: &ui::Request,
    id: &str,
    launch: impl FnOnce(&str) -> Result<bool>,
) -> Result<ui::Receipt> {
    write_operation(
        m,
        id,
        &json!({"schema":1,"operation":id,"state":"queued","action":request.action}),
        false,
    )?;
    if !launch(id).unwrap_or(false) {
        refuse_unfinished(m, id, "operator_worker_launch_failed")?;
        return Err("operator_worker_launch_failed".into());
    }
    Ok(ui::Receipt {
        schema: ui::OPERATOR_SCHEMA,
        accepted: true,
        operation: Some(id.into()),
        refusal: None,
    })
}
// Record a valid typed request before admission. Even a pre-worker refusal owns
// a durable result; late completion cannot replace a newer request's projection.
fn dispatch_recorded(
    m: &Manager,
    request: &ui::Request,
    run: impl FnOnce(&str) -> Result<ui::Receipt>,
) -> Result<ui::Receipt> {
    let id = begin_request(m, request)?;
    match run(&id) {
        Ok(r) => Ok(r),
        Err(e) => {
            let reason: String = e.to_string().chars().take(512).collect();
            write_operation(
                m,
                &id,
                &json!({"schema":1,"operation":id,"state":"refused","stage":"operator_request_admission","action":request.action,"reason":reason,"worker_started":false,"mutation_started":false}),
                false,
            )?;
            Ok(ui::Receipt {
                schema: ui::OPERATOR_SCHEMA,
                accepted: false,
                operation: Some(id),
                refusal: Some(reason),
            })
        }
    }
}
fn dispatch(m: &Manager, request: ui::Request) -> Result<ui::Receipt> {
    dispatch_recorded(m, &request, |id| {
        let _lock = m.lock("operator-dispatch.lock")?;
        validate_current_request(m, &request)?;
        let sw = software(m)?;
        sw.manager.verify()?;
        launch_reserved(m, &request, id, |id| {
            let status = Command::new("systemd-run")
                .args([
                    "--user",
                    "--collect",
                    "--property=UMask=0077",
                    "--property=KillMode=control-group",
                    "--property=StandardOutput=null",
                    "--property=StandardError=null",
                ])
                .arg(format!(
                    "--property=ExecStopPost={} operator finish {id}",
                    systemd(
                        sw.manager
                            .path
                            .to_str()
                            .ok_or("operator_executable_encoding")?
                    )
                ))
                .arg(format!("--unit=linux-vst-bridge-operator-{id}"))
                .arg(&sw.manager.path)
                .args(["operator", "worker", id])
                .status()?;
            Ok(status.success())
        })
    })
}

#[cfg(test)]
fn execute(m: &Manager, a: &ui::Action) -> Result<Value> {
    execute_with_receipt(m, a, None)
}
#[cfg(test)]
fn execute_with_receipt(m: &Manager, a: &ui::Action, operation: Option<&str>) -> Result<Value> {
    execute_with_receipt_capacity(m, a, operation, &|| live_capacity(m).ok())
}
#[cfg(test)]
fn execute_with_receipt_capacity(
    m: &Manager,
    a: &ui::Action,
    operation: Option<&str>,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
) -> Result<Value> {
    execute_with_receipt_policy(m, a, operation, capacity_read, OPERATOR_WAIT, &mut vec![])
}
#[cfg(test)]
pub(super) fn execute_idle_test_action(m: &Manager, a: &ui::Action) -> Result<Value> {
    execute_with_receipt_capacity(m, a, None, &|| {
        serde_json::from_value(json!({
            "schema": 1,
            "dsp": 0,
            "maintenance": 0,
            "keepers": 0,
            "cleanup_unconfirmed": false,
            "owners": [],
            "limits": {"global_dsp": 6}
        }))
        .ok()
    })
}
// Metadata-only stamp of durable control-plane preconditions. No installer or
// runner artifacts are traversed here; absence and exact filenames are included.
fn creation_control_stamp(m: &Manager) -> Result<Vec<(PathBuf, onboarding::FileIdentity)>> {
    let mut paths = vec![];
    for path in [
        m.root.join("operator/resume.json"),
        app_directory(m).join("operation.json"),
        app_directory(m).join("operation-result.json"),
        renderer_cli::directory(m).join("application.json"),
        renderer_cli::directory(m).join("current.json"),
    ] {
        if path.try_exists()? {
            paths.push(path);
        }
    }
    let directory = m.root.join("onboarding");
    if directory.exists() {
        for (index, entry) in fs::read_dir(directory)?.enumerate() {
            require(index < 128, "onboarding_count_bound")?;
            let entry = entry?;
            require(entry.file_type()?.is_dir(), "onboarding_record_directory")?;
            for (n, item) in fs::read_dir(entry.path())?.enumerate() {
                require(n < 128, "onboarding_operation_bound")?;
                let path = item?.path();
                let name = path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .ok_or("onboarding_record_name")?;
                if name == "record.json" || name.ends_with("-result.json") {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|p| Ok((p.clone(), onboarding::FileIdentity::read(&p)?)))
        .collect()
}
fn create_environment_owned(
    m: &Manager,
    selection: &ui::Action,
    owner: &str,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
) -> Result<Value> {
    let before = token(m)?;
    let prepared = match selection {
        ui::Action::InstallerEnvironmentCreate { installer, runner } => {
            onboarding::prepare_creation(m, installer, runner)?
        }
        ui::Action::InstallerNewAttempt { previous, runner } => {
            onboarding::prepare_attempt(m, previous, runner)?
        }
        _ => return Err("onboarding_creation_action".into()),
    };
    let controls = creation_control_stamp(m)?;
    let retired = vendor_retired(m)? && onboarding::all_retired(m)?;
    require(
        controls == creation_control_stamp(m)?,
        "operator_creation_posture_changed",
    )?;
    let cap = capacity_read();
    // Exactly one registry acquisition for the mutation. No owner below this
    // point reacquires it, hashes an installer/runner, or queries systemd.
    let guard = match m.lock_bounded(
        ui::OperatorLock::Registry,
        ui::LockPurpose::EnvironmentCreationAdmission,
        Some(owner),
        timeout,
    ) {
        Ok((guard, facts)) => {
            waits.push(facts);
            guard
        }
        Err(e) => {
            if let Some(f) = e.downcast_ref::<linux_vst_bridge::operator_lock::AcquisitionFailure>()
            {
                waits.push(f.facts.clone());
            }
            return Err(e);
        }
    };
    require(
        before == token(m)? && controls == creation_control_stamp(m)?,
        "operator_creation_posture_changed",
    )?;
    let cap = cap.ok_or("capacity_readback_unavailable")?;
    require(
        cap.owners == capacity::owners(m)?,
        "operator_creation_owners_changed",
    )?;
    if let Some(reason) = inactive_reason(Some(&cap), retired, pending_transactions(m)?, false) {
        return Err(reason.into());
    }
    m.require_inactive(None)?;
    let result = onboarding::create_prepared(m, prepared, owner, &guard);
    drop(guard);
    result
}
fn execute_with_receipt_policy(
    m: &Manager,
    a: &ui::Action,
    operation: Option<&str>,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
) -> Result<Value> {
    let mut projection = Some(acquire_readback(
        m,
        ui::OperatorLock::Canonical,
        operation,
        timeout,
        waits,
    )?);
    if matches!(
        a,
        ui::Action::InstallerEnvironmentCreate { .. } | ui::Action::InstallerNewAttempt { .. }
    ) {
        return create_environment_owned(
            m,
            a,
            operation.ok_or("operator_operation_identity")?,
            timeout,
            waits,
            capacity_read,
        );
    }
    require_operator_inactive_with(m, a, capacity_read, operation, timeout, waits)?;

    if preparation_cli::is_action(a) {
        let owner = operation.ok_or("operator_operation_identity")?;
        if matches!(
            a,
            ui::Action::PluginReinspect { .. }
                | ui::Action::PluginInspect { .. }
                | ui::Action::PluginPrepare { .. }
                | ui::Action::CompatibilityCheck { .. }
                | ui::Action::CompatibilityResumeCheck { .. }
        ) {
            let _environment = m.lock("operator-environment.lock")?;
            suspend(m, owner, None, timeout, waits)?;
            drop(projection.take());
            let result = preparation_cli::execute(m, a, owner, || {
                acquire_readback(
                    m,
                    ui::OperatorLock::Registry,
                    Some(owner),
                    timeout,
                    waits,
                )
            });
            let cleanup = resume_owned(m, owner);
            let value = result?;
            cleanup?;
            return Ok(value);
        }
        return preparation_cli::execute(m, a, owner, || {
            acquire_readback(
                m,
                ui::OperatorLock::Registry,
                Some(owner),
                timeout,
                waits,
            )
        });
    }
    match a {
        ui::Action::RuntimeInstall {} => {
            drop(projection.take());
            let runner = linux_vst_bridge::runtime_delivery::install(m)?;
            Ok(json!({"runtime":runner.id,"runner_key":catalogue::runner_key(&runner)?,
                "installed":true,"plugin_support":"not_qualified_by_runtime_installation"}))
        }
        ui::Action::SupportExport {} => {
            drop(projection.take());
            let current = overview(m)?;
            let owner = operation.ok_or("operator_operation_identity")?;
            let original: ui::Request = read_json(&job_dir(m, owner)?.join("request.json"))?;
            require(original.schema == ui::OPERATOR_SCHEMA
                && original.action == *a
                && original.state_token == current.current.state_token,
                "operator_stale_request_refresh")?;
            readiness::export(m, &current)
        }
        action @ (ui::Action::WorkspaceSelectInstaller { .. }
        | ui::Action::WorkspaceInstall {}
        | ui::Action::WorkspaceFinishInstall {}
        | ui::Action::WorkspaceLaunch {}
        | ui::Action::WorkspaceUninstall {}
        | ui::Action::WorkspaceFinishUninstall {}
        | ui::Action::WorkspaceFocus {}
        | ui::Action::WorkspaceStop {}
        | ui::Action::WorkspaceSelectProductInstaller { .. }
        | ui::Action::WorkspaceInstallProduct { .. }
        | ui::Action::WorkspaceFinishProductInstall { .. }) => {
            drop(projection.take());
            daw_workspace::execute_action(m, action, operation.ok_or("operator_operation_identity")?)
        }
        ui::Action::PluginReinspect { .. }
        | ui::Action::PluginInspect { .. }
        | ui::Action::PluginPrepare { .. }
        | ui::Action::CompatibilityCheck { .. }
        | ui::Action::CompatibilityResumeCheck { .. }
        | ui::Action::CompatibilityPublishTest { .. }
        | ui::Action::CompatibilityResult { .. }
        | ui::Action::CompatibilityFinishResult { .. }
        | ui::Action::ExperimentalReplace { .. }
        | ui::Action::CandidateWithdraw { .. }
        | ui::Action::ExperimentalEnable { .. }
        | ui::Action::ExperimentalDisable { .. }
        | ui::Action::CandidateObserve { .. }
        | ui::Action::CandidateReview { .. }
        | ui::Action::CandidatePublishOrdinary { .. } => unreachable!(),
        ui::Action::InstallerEnvironmentCreate { .. } | ui::Action::InstallerNewAttempt { .. } => {
            unreachable!()
        }
        ui::Action::InstallerRename { installer, label } => {
            let named = installer_import::rename(m, installer, label)?;
            Ok(json!({"installer":installer,"display_label":named.display_label,"presentation_only":true}))
        }
        ui::Action::InstallerStart { onboarding: id }
        | ui::Action::InstallerStartWithPolicy { onboarding: id, .. } => {
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            let prior = onboarding::load(m, id)?;
            require(
                prior.installation_operation.is_none(),
                "installer_initial_transaction_already_used",
            )?;
            suspend(m, owner, None, timeout, waits)?;
            let reserved = onboarding::reserve(m, id, owner);
            let r = match reserved {
                Ok(r) => r,
                Err(e) => {
                    let _ = resume_owned(m, owner);
                    return Err(e);
                }
            };
            let policy = match a {
                ui::Action::InstallerStartWithPolicy { powershell, .. } => Some(*powershell),
                _ => None,
            };
            let launched = onboarding::launch(m, &r, policy);
            drop(projection.take());
            if launched.is_ok() {
                write_operation(
                    m,
                    owner,
                    &json!({"schema":2,"operation":owner,"state":"vendor_running","action":a}),
                    false,
                )?;
                while onboarding::live(owner)? {
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
            let v = onboarding::result(m, &r)?;
            require(onboarding::retired(&v), "installer_cleanup_unconfirmed")?;
            resume_owned(m, owner)?;
            launched?;
            Ok(v)
        }
        ui::Action::InstallerFocus {
            onboarding: id,
            operation: owner,
        } => onboarding::focus(m, id, owner),
        ui::Action::InstallerStop {
            onboarding: id,
            operation: owner,
        } => {
            let v = onboarding::stop(m, id, owner)?;
            resume_owned(m, owner)?;
            Ok(v)
        }
        ui::Action::InstallerScan { onboarding: id } => {
            let _environment = m.lock("operator-environment.lock")?;
            let r = onboarding::load(m, id)?;
            require(
                onboarding::retired(&onboarding::result(m, &r)?),
                "installer_retirement_required",
            )?;
            let owner = operation.ok_or("operator_operation_identity")?;
            suspend(m, owner, None, timeout, waits)?;
            let result = rescan_environment(m, r.environment);
            let cleanup = resume_owned(m, owner);
            let value = result?;
            cleanup?;
            Ok(value)
        }
        ui::Action::CaptureArm { class_id } => {
            crash_capture::arm(m, Some(class_id))?;
            Ok(json!({"capture":"armed"}))
        }
        ui::Action::CaptureDisarm {} => {
            crash_capture::disarm(m)?;
            Ok(json!({"capture":"disarmed"}))
        }
        ui::Action::TransactionReconcile {} => {
            m.reconcile_inactive()?;
            resume_interrupted(m)?;
            Ok(json!({"reconciled":true}))
        }
        ui::Action::OrdinaryRollback {
            class_id,
            publication,
        } => Ok(serde_json::to_value(m.rollback_inactive(
            class_id,
            publication,
            None,
        )?)?),
        ui::Action::OrdinaryRestoreRecommended { class_id } => {
            managed_cli::restore_recommended(m, class_id)?;
            Ok(json!({"restored":true}))
        }
        ui::Action::IncidentExport { incident } => {
            require(valid_hex(incident, 32), "operator_incident_identity")?;
            let value = optional(
                &m.root
                    .join("runtime/incidents")
                    .join(incident)
                    .join("share.json"),
            )?;
            require(!value.is_null(), "operator_sanitized_incident_unavailable")?;
            let dir = m.root.join("exports");
            private_dir(&dir)?;
            let path = dir.join(format!("incident-{incident}.json"));
            atomic_json(&path, &value)?;
            Ok(json!({"export":path,"sanitized":true}))
        }
        ui::Action::DependencyPrepare {} => {
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            suspend(m, owner, Some(owner.into()), timeout, waits)?;
            let submission = match dependency_cli::launch(m, owner) {
                Ok(s) => s,
                Err(e) => {
                    if dependency_session::current(m)?["operation"] != owner {
                        resume_owned(m, owner)?;
                        return Err(e);
                    }
                    dependency_session::reconcile(m, owner)?
                }
            };
            write_operation(
                m,
                owner,
                &json!({"schema":1,"operation":owner,"state":if submission==dependency_session::Submission::AcknowledgmentUncertain{"submission_uncertain"}else{"vendor_running"},"action":a}),
                false,
            )?;
            drop(projection.take());
            let deadline = Instant::now() + Duration::from_secs(650);
            loop {
                dependency_session::reconcile(m, owner)?;
                if dependency_session::retired(m, owner)? {
                    break;
                }
                require(
                    Instant::now() < deadline,
                    "dependency_retirement_unconfirmed",
                )?;
                std::thread::sleep(Duration::from_millis(500));
            }
            resume_owned(m, owner)?;
            dependency_cli::retain(m, owner)?;
            Ok(
                json!({"dependency":"prepared_and_retired","live_readiness":"not_claimed","service":"resumed"}),
            )
        }
        ui::Action::DependencyStop { operation: target } => {
            stop_dependency_with(
                m,
                target,
                |op| dependency_session::stop(m, op),
                |saved| restore_service(m, saved),
            )?;
            Ok(json!({"dependency":"stopped","operation":target}))
        }
        ui::Action::RendererDiscover {} => renderer_cli::discover(m),
        ui::Action::RendererOpen {
            application,
            policy,
        } => {
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            suspend(m, owner, Some(owner.into()), timeout, waits)?;
            let submission = match renderer_cli::launch(m, application, *policy, owner) {
                Ok(state) => state,
                Err(e) => {
                    // Before reservation no unit can have been submitted. After
                    // reservation every error retains exact recovery authority.
                    if renderer_cli::current(m)?["operation"] != owner {
                        resume_owned(m, owner)?;
                        return Err(e);
                    }
                    renderer_cli::reconcile(m, owner)?
                }
            };
            write_operation(
                m,
                owner,
                &json!({"schema":1,"operation":owner,"state":if submission==renderer_session::Submission::AcknowledgmentUncertain{"submission_uncertain"}else{"vendor_running"},"action":a}),
                false,
            )?;
            drop(projection.take());
            // Normal application lifetime belongs to the exact operation, not a
            // wall-clock budget. Stop and cleanup retain their separate bounds.
            wait_renderer_retirement(
                || { renderer_cli::reconcile(m, owner)?; renderer_session::retired(m, owner) },
                || std::thread::sleep(Duration::from_millis(500)),
            )?;
            resume_owned(m, owner)?;
            let recovery=native_access_restoration(m)?;
            let result=renderer_cli::result(m,owner)?;
            let session=native_access_session_summary(&result,&recovery)?;
            Ok(
                json!({"application":"retired","service":"resumed","recovery":recovery,"session":session,"result":result}),
            )
        }
        ui::Action::RendererStop { operation: target } => {
            stop_renderer_with(
                m,
                target,
                |id| renderer_cli::stop(m, id),
                |saved| restore_service(m, saved),
            )?;
            let recovery=native_access_restoration(m)?;
            let result=renderer_cli::result(m,target)?;
            let session=native_access_session_summary(&result,&recovery)?;
            Ok(json!({"application":"stopped","operation":target,"recovery":recovery,"session":session,"result":result}))
        }
        ui::Action::RendererFocus { operation: target } => renderer_cli::focus(m, target),
        ui::Action::RendererObserve {
            operation: target,
            presentation,
        } => renderer_cli::observe(m, target, *presentation),
        ui::Action::VendorApplicationOpen { application } => {
            vendor_application::ApplicationId::parse(application)?;
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            let vendor_operation = random_id()?;
            suspend(m, owner, Some(vendor_operation.clone()), timeout, waits)?;
            let launched = vendor_cli::launch_owned(m, application, &vendor_operation);
            if let Err(e) = launched {
                let _ = resume_owned(m, owner);
                return Err(e);
            }
            if let Some(id) = operation {
                let receipt =
                    json!({"schema":1,"operation":id,"state":"vendor_running","action":a});
                write_operation(m, id, &receipt, false)?;
            }
            // This manager operation outlives the frontend. ASC's own existing
            // dedicated unit remains process owner; neither UI close nor this
            // operation creates an orphan or kills a continuing download.
            drop(projection.take());
            while vendor_live()? {
                std::thread::sleep(Duration::from_millis(500));
            }
            require(vendor_retired(m)?, "operator_vendor_cleanup_unconfirmed")?;
            resume_owned(m, owner)?;
            Ok(json!({"vendor":"retired","service":"resumed"}))
        }
        ui::Action::VendorApplicationStop { application } => {
            stop_vendor_with(
                m,
                application,
                |vendor| vendor_cli::cancel_owned(m, application, vendor),
                |saved| restore_service(m, saved),
            )?;
            Ok(json!({"vendor":"stopped"}))
        }
        ui::Action::VendorApplicationFocus { application } => {
            vendor_application::ApplicationId::parse(application)?;
            require(vendor_live()?, "operator_vendor_not_running")?;
            let operation = optional(&app_directory(m).join("operation-result.json"))?;
            let operation_id = operation["operation_id"]
                .as_str()
                .ok_or("operator_focus_owner_unsupported")?;
            let request = random_id()?;
            atomic_json(
                &app_directory(m).join("focus.json"),
                &json!({"request":request,"operation_id":operation_id}),
            )?;
            let deadline = Instant::now() + Duration::from_secs(4);
            loop {
                let value = optional(&app_directory(m).join("operation-result.json"))?;
                if value["focus_result"]["request"] == request {
                    require(
                        value["focus_result"]["result"] == "focused",
                        "operator_focus_refused",
                    )?;
                    return Ok(value["focus_result"].clone());
                }
                require(
                    Instant::now() < deadline,
                    "operator_focus_receipt_unavailable",
                )?;
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        ui::Action::EnvironmentRescan { environment } => {
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            suspend(m, owner, None, timeout, waits)?;
            let result = rescan(m, environment);
            let cleanup = resume_owned(m, owner);
            let value = result?;
            cleanup?;
            Ok(value)
        }
        ui::Action::QuarantinedModuleRetry { environment, scan, module_index,
            module_sha256, report_sha256 } => {
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            suspend(m, owner, None, timeout, waits)?;
            let result = retry_quarantined_module(m, environment, scan, *module_index,
                module_sha256, report_sha256);
            let cleanup = resume_owned(m, owner);
            let value = result?;
            cleanup?;
            Ok(value)
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumeRecord {
    schema: u32,
    owner_operation: String,
    resume: bool,
    software: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    vendor_operation: Option<String>,
}
fn resume_lock(m: &Manager) -> Result<Lock> {
    let deadline = Instant::now() + Duration::from_secs(75);
    loop {
        match m.lock("operator-resume.lock") {
            Ok(lock) => return Ok(lock),
            Err(e) => {
                if !e.is::<linux_vst_bridge::operator_lock::LockBusy>()
                    || Instant::now() >= deadline
                {
                    return Err(e);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
fn resume_record(m: &Manager) -> Result<Option<ResumeRecord>> {
    let value = optional(&m.root.join("operator/resume.json"))?;
    if value.is_null() {
        return Ok(None);
    }
    let saved: ResumeRecord = serde_json::from_value(value)?;
    require(
        saved.schema == 1
            && valid_hex(&saved.owner_operation, 32)
            && valid_hex(&saved.software, 64)
            && saved
                .vendor_operation
                .as_ref()
                .is_none_or(|id| valid_hex(id, 32)),
        "operator_resume_identity",
    )?;
    Ok(Some(saved))
}
// Caller holds operator-resume.lock. An outstanding recovery is never replaced.
fn create_resume(m: &Manager, saved: &ResumeRecord) -> Result<()> {
    require(resume_record(m)?.is_none(), "operator_resume_pending")?;
    job_dir(m, &saved.owner_operation)?;
    atomic_json(&m.root.join("operator/resume.json"), saved)
}
fn suspend(
    m: &Manager, owner: &str, vendor_operation: Option<String>,
    timeout: Duration, waits: &mut Vec<ui::LockFacts>,
) -> Result<()> {
    suspend_with(m, owner, vendor_operation, timeout, waits, (|| active(SERVICE), || service("stop")))
}
fn suspend_with(
    m: &Manager, owner: &str, vendor_operation: Option<String>,
    timeout: Duration, waits: &mut Vec<ui::LockFacts>,
    service_io: (impl FnOnce() -> bool, impl FnOnce() -> Result<()>),
) -> Result<()> {
    // Same lock order as recovery: resume, then registry. Hold through Stop so
    // even the matching owner cannot consume the record before suspension.
    let _resume = resume_lock(m)?;
    let _lock = acquire_readback(m, ui::OperatorLock::Registry, Some(owner), timeout, waits)?;
    m.require_inactive(None)?;
    require(vendor_retired(m)?, "operator_vendor_active")?;
    let was_active = service_io.0();
    create_resume(
        m,
        &ResumeRecord {
            schema: 1,
            owner_operation: owner.into(),
            resume: was_active,
            software: software(m)?.manager.sha256,
            vendor_operation,
        },
    )?;
    if was_active {
        service_io.1()?;
    }
    require(!reconcile_leases(m)?, "operator_cleanup_unconfirmed")?;
    Ok(())
}
fn resume_owned(m: &Manager, owner: &str) -> Result<()> {
    resume_owned_with(m, owner, |saved| restore_service(m, saved))
}
fn resume_owned_with(
    m: &Manager,
    owner: &str,
    restore: impl FnOnce(&ResumeRecord) -> Result<()>,
) -> Result<()> {
    let _resume = resume_lock(m)?;
    resume_locked(m, owner, restore)
}
// The lock covers read, restore and removal. A newer suspension cannot slip
// between the identity comparison and record retirement.
fn resume_locked(
    m: &Manager,
    owner: &str,
    restore: impl FnOnce(&ResumeRecord) -> Result<()>,
) -> Result<()> {
    job_dir(m, owner)?;
    let Some(saved) = resume_record(m)? else {
        return Ok(());
    };
    if saved.owner_operation != owner {
        return Ok(());
    }
    restore(&saved)?;
    fs::remove_file(m.root.join("operator/resume.json"))?;
    Ok(())
}
fn recovery_request(m: &Manager, saved: &ResumeRecord) -> Result<ui::Action> {
    let request: ui::Request =
        read_json(&job_dir(m, &saved.owner_operation)?.join("request.json"))?;
    // The installed private UI2 manager retained schema-11 requests. This is
    // historical readback only; live requests still require operator schema 12.
    require(
        matches!(request.schema, 5..=13),
        "operator_resume_request_schema",
    )?;
    Ok(request.action)
}
fn stop_dependency_with(
    m: &Manager,
    target: &str,
    close: impl FnOnce(&str) -> Result<()>,
    restore: impl FnOnce(&ResumeRecord) -> Result<()>,
) -> Result<()> {
    let _resume = resume_lock(m)?;
    let saved = resume_record(m)?.ok_or("dependency_resume_absent")?;
    require(
        saved.owner_operation == target && saved.vendor_operation.as_deref() == Some(target),
        "dependency_stop_wrong_operation",
    )?;
    require(
        recovery_request(m, &saved)? == ui::Action::DependencyPrepare {},
        "dependency_resume_action",
    )?;
    dependency_session::exact(m, target)?;
    close(target)?;
    resume_locked(m, target, restore)
}
fn stop_renderer_with(
    m: &Manager,
    target: &str,
    close: impl FnOnce(&str) -> Result<()>,
    restore: impl FnOnce(&ResumeRecord) -> Result<()>,
) -> Result<()> {
    let _resume = resume_lock(m)?;
    let saved = resume_record(m)?.ok_or("renderer_resume_owner_absent")?;
    require(
        saved.owner_operation == target && saved.vendor_operation.as_deref() == Some(target),
        "renderer_resume_owner_mismatch",
    )?;
    let ui::Action::RendererOpen {
        application,
        policy,
    } = recovery_request(m, &saved)?
    else {
        return Err("renderer_resume_action_mismatch".into());
    };
    require(
        renderer_cli::current(m)?
            == json!({"operation":target,"application":application,"policy":policy}),
        "renderer_resume_reservation_mismatch",
    )?;
    close(target)?;
    resume_locked(m, target, restore)
}
fn stop_vendor_with(
    m: &Manager,
    application: &str,
    cancel: impl FnOnce(&str) -> Result<()>,
    restore: impl FnOnce(&ResumeRecord) -> Result<()>,
) -> Result<()> {
    vendor_application::ApplicationId::parse(application)?;
    let _resume = resume_lock(m)?;
    let saved = resume_record(m)?.ok_or("operator_vendor_open_owner_absent")?;
    require(
        recovery_request(m, &saved)?
            == (ui::Action::VendorApplicationOpen {
                application: application.into(),
            }),
        "operator_vendor_open_owner_mismatch",
    )?;
    let vendor = saved
        .vendor_operation
        .as_deref()
        .ok_or("operator_vendor_open_binding_absent")?;
    let job = optional(&app_directory(m).join("operation.json"))?;
    require(
        job["operation_id"] == vendor,
        "operator_vendor_open_binding_changed",
    )?;
    // cancel_owned also checks this identity under the vendor registry lock,
    // then requires positive retirement of that same launch.
    cancel(vendor)?;
    resume_locked(m, &saved.owner_operation, restore)
}
fn resume_interrupted(m: &Manager) -> Result<()> {
    resume_interrupted_with(m, |saved| restore_service(m, saved))
}
fn resume_interrupted_with(
    m: &Manager,
    restore: impl FnOnce(&ResumeRecord) -> Result<()>,
) -> Result<()> {
    // Only explicit reconciliation can recover a completed prior owner. It
    // cannot steal an active Open/Rescan or adopt an ownerless legacy record.
    let _resume = resume_lock(m)?;
    let Some(saved) = resume_record(m)? else {
        return Ok(());
    };
    let _environment = m.lock("operator-environment.lock")?;
    if matches!(
        recovery_request(m, &saved)?,
        ui::Action::DependencyPrepare {}
    ) {
        require(
            saved.vendor_operation.as_deref() == Some(saved.owner_operation.as_str()),
            "dependency_resume_owner",
        )?;
        dependency_session::reconcile(m, &saved.owner_operation)?;
        require(
            dependency_session::retired(m, &saved.owner_operation)?,
            "dependency_custody_uncertain",
        )?;
        return resume_locked(m, &saved.owner_operation, restore);
    }
    if matches!(
        recovery_request(m, &saved)?,
        ui::Action::RendererOpen { .. }
    ) {
        require(
            saved.vendor_operation.as_deref() == Some(saved.owner_operation.as_str()),
            "renderer_resume_owner_mismatch",
        )?;
        renderer_cli::reconcile(m, &saved.owner_operation)?;
        require(
            renderer_session::retired(m, &saved.owner_operation)?,
            "renderer_custody_still_uncertain",
        )?;
        return resume_locked(m, &saved.owner_operation, restore);
    }
    require(
        matches!(
            recovery_request(m, &saved)?,
            ui::Action::InstallerStart { .. }
                | ui::Action::InstallerStartWithPolicy { .. }
                | ui::Action::InstallerScan { .. }
                | ui::Action::RendererOpen { .. }
                | ui::Action::VendorApplicationOpen { .. }
                | ui::Action::EnvironmentRescan { .. }
                | ui::Action::QuarantinedModuleRetry { .. }
                | ui::Action::PluginReinspect { .. }
                | ui::Action::PluginInspect { .. }
                | ui::Action::PluginPrepare { .. }
                | ui::Action::CompatibilityCheck { .. }
                | ui::Action::CompatibilityResumeCheck { .. }
        ),
        "operator_resume_action_mismatch",
    )?;
    let result = optional(&job_dir(m, &saved.owner_operation)?.join("result.json"))?;
    require(
        result["operation"] == saved.owner_operation
            && matches!(result["state"].as_str(), Some("completed" | "refused")),
        "operator_resume_owner_not_terminal",
    )?;
    resume_locked(m, &saved.owner_operation, restore)
}
fn restore_service(m: &Manager, saved: &ResumeRecord) -> Result<()> {
    require(
        onboarding::all_retired(m)?,
        "operator_installer_cleanup_unconfirmed",
    )?;
    require(vendor_retired(m)?, "operator_vendor_cleanup_unconfirmed")?;
    require(
        saved.software == software(m)?.manager.sha256,
        "operator_resume_software_changed",
    )?;
    if saved.resume {
        service("start")?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut peer = loop {
            match UnixStream::connect(m.root.join("runtime/owner.sock")) {
                Ok(p) => break p,
                Err(e) => {
                    if Instant::now() >= deadline {
                        return Err(e.into());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        };
        peer.set_read_timeout(Some(Duration::from_secs(70)))?;
        peer.set_write_timeout(Some(Duration::from_secs(2)))?;
        peer.write_all(b"LVE1\n")?;
        let mut receipt = [0; 11];
        peer.read_exact(&mut receipt)?;
        require(
            &receipt == b"LVE1 ready\n",
            "operator_keeper_resume_unconfirmed",
        )?;
    }
    Ok(())
}
pub(super) fn rescan(m: &Manager, environment: &str) -> Result<Value> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let sw = software(m)?;
    let db = m.registry()?;
    let c = operator_catalogue(m, &sw, &db)?;
    let env = managed_rescan_binding(m, c.as_ref(), &db, environment)?
        .ok_or("operator_environment_unmanaged")?;
    require(
        onboarding::inventory_refresh_required(
            m,
            &env,
            &sw.host,
            &sw.source_sha256,
        )? || linux_vst_bridge::frg1::adopted_environment(m)?
            .is_some_and(|binding| binding.environment == env)
            && linux_vst_bridge::frg1::inventory_refresh_required(m,&env)?,
        "managed_inventory_current",
    )?;
    rescan_environment_locked(m, env, sw, None)
}
#[derive(Clone, Debug)]
struct QuarantinedRetry {
    scan: String,
    module_index: usize,
    module_sha256: String,
    report_sha256: String,
}
fn retry_quarantined_module(m: &Manager, environment: &str, scan: &str,
    module_index: usize, module_sha256: &str, report_sha256: &str) -> Result<Value> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let sw = software(m)?;
    let db = m.registry()?;
    let catalogue = operator_catalogue(m, &sw, &db)?;
    let env = managed_rescan_binding(m, catalogue.as_ref(), &db, environment)?
        .ok_or("operator_environment_unmanaged")?;
    rescan_environment_locked(m, env, sw, Some(QuarantinedRetry {
        scan: scan.into(), module_index, module_sha256: module_sha256.into(),
        report_sha256: report_sha256.into(),
    }))
}
fn rescan_environment(m: &Manager, env: Environment) -> Result<Value> {
    rescan_environment_with_retry(m, env, None)
}
fn reusable_quarantine<'a>(prior: Option<&'a inventory::Scan>, env: &Environment,
    host: &Artifact, host_source_sha256: &str, module: &Artifact)
    -> Option<&'a inventory::Module> {
    prior.filter(|s| s.environment == *env && s.host == *host
        && s.host_source_sha256 == host_source_sha256)
        .and_then(|s| s.modules.iter().find(|p|
            p.artifact == *module && p.quarantine_reason.is_some()))
}
fn reusable_scan_module<'a>(prior: Option<&'a inventory::Scan>, env: &Environment,
    host: &Artifact, host_source_sha256: &str, module: &Artifact,
    retry_target: Option<&Artifact>) -> Result<Option<&'a inventory::Module>> {
    if let Some(target)=retry_target {
        if module==target { return Ok(None); }
        return Ok(Some(prior.and_then(|scan|scan.modules.iter()
            .find(|old|old.artifact==*module))
            .ok_or("operator_quarantine_retry_inventory_changed")?));
    }
    Ok(reusable_quarantine(prior,env,host,host_source_sha256,module))
}
fn exact_retry_target<'a>(prior: Option<&'a inventory::Scan>, env: &Environment,
    host: &Artifact, host_source_sha256: &str, modules: &[Artifact],
    retry: &QuarantinedRetry) -> Result<&'a inventory::Module> {
    require(valid_hex(&retry.scan,32) && valid_hex(&retry.module_sha256,64)
        && valid_hex(&retry.report_sha256,64),"operator_quarantine_retry_identity")?;
    let prior=prior.ok_or("operator_quarantine_retry_scan_absent")?;
    require(prior.schema==1 && prior.id==retry.scan && prior.environment==*env
        && prior.host.sha256==host.sha256
        && prior.host_source_sha256==host_source_sha256,
        "operator_quarantine_retry_scan_changed")?;
    require(prior.host.verify().is_ok() && host.verify().is_ok(),
        "operator_quarantine_retry_scan_changed")?;
    require(prior.modules.len()==modules.len() && prior.modules.iter().zip(modules)
        .all(|(old,current)|old.artifact==*current),
        "operator_quarantine_retry_inventory_changed")?;
    let selected=prior.modules.get(retry.module_index)
        .ok_or("operator_quarantine_retry_module_absent")?;
    require(selected.artifact.sha256==retry.module_sha256
        && selected.report.sha256==retry.report_sha256
        && selected.quarantine_reason.is_some(),"operator_quarantine_retry_module_changed")?;
    selected.report.verify()?;
    Ok(selected)
}
fn rescan_environment_with_retry(m: &Manager, env: Environment,
    retry: Option<QuarantinedRetry>) -> Result<Value> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let sw = software(m)?;
    rescan_environment_locked(m, env, sw, retry)
}
fn rescan_environment_locked(
    m: &Manager,
    env: Environment,
    sw: Software,
    retry: Option<QuarantinedRetry>,
) -> Result<Value> {
    let environment = env.id.clone();
    let modules = managed_cli::modules(&env)?;
    require(modules.len() <= 64, "operator_scan_module_bound")?;
    let prior_path = m.root.join("inventory").join(format!("{environment}.json"));
    let prior = if prior_path.exists() {
        Some(read_json::<inventory::Scan>(&prior_path)?)
    } else {
        None
    };
    let retry_target=retry.as_ref().map(|retry|exact_retry_target(prior.as_ref(),&env,
        &sw.host,&sw.source_sha256,&modules,retry).map(|module|module.artifact.clone()))
        .transpose()?;
    let start = Instant::now();
    let mut found = Vec::new();
    for module in modules {
        if let Some(old)=reusable_scan_module(prior.as_ref(),&env,&sw.host,
            &sw.source_sha256,&module,retry_target.as_ref())? {
            found.push(old.clone());
            continue;
        }
        require(
            start.elapsed() < Duration::from_secs(600),
            "operator_scan_time_bound",
        )?;
        let stamp = observation::ModuleStamp::read(&module.path)?;
        let (job, path) = spec(
            m,
            HostBinding {
                metadata: ClassSelection {
                    class_id: String::new(),
                },
                environment: env.clone(),
                module: module.clone(),
                host: sw.host.clone(),
                host_source_sha256: sw.source_sha256.clone(),
                compatibility: Compatibility::default(),
            },
            true,
            true,
            false,
        )?;
        let pending =
            PendingAdmission::new(job.lease.clone(), Arc::new(AtomicBool::new(false)));
        #[cfg(test)]
        SCAN_SPAWN_COUNT.with(|count| count.set(count.get() + 1));
        let child = spawn(&sw, &path, None)?;
        vendor_product_cli::finish_scan(child, &job, &path, pending)?;
        module.verify()?;
        require(
            observation::ModuleStamp::read(&module.path)? == stamp,
            "inventory_module_changed",
        )?;
        let report = Artifact {
            sha256: digest(&job.report)?,
            path: job.report,
        };
        let raw: Value = read_json(&report.path)?;
        let (classes, quarantine_reason) = match inventory::classes(&raw) {
            Ok(c) => (c, None),
            Err(e) => (vec![], Some(e.to_string())),
        };
        found.push(inventory::Module {
            artifact: module,
            classes,
            report,
            quarantine_reason,
            inspection_error: raw["error"].as_str().map(|s| s.chars().take(256).collect()),
        });
    }
    let scan = inventory::Scan {
        schema: 1,
        id: random_id()?,
        environment: env,
        host: sw.host,
        host_source_sha256: sw.source_sha256,
        completed_at: observation::now()?,
        changes: inventory::changes(prior.as_ref().map(|p| p.modules.as_slice()), &found),
        modules: found,
    };
    let dir = m.root.join("inventory");
    private_dir(&dir)?;
    private_dir(&dir.join("history"))?;
    atomic_json(
        &dir.join("history").join(format!("{}.json", scan.id)),
        &scan,
    )?;
    atomic_json(&dir.join(format!("{environment}.json")), &scan)?;
    Ok(json!({"scan":scan.id,"modules":scan.modules.len(),"activation_permitted":false,
        "retry":retry.map(|retry|json!({"prior_scan":retry.scan,
            "module_index":retry.module_index,"module_sha256":retry.module_sha256,
            "prior_report_sha256":retry.report_sha256}))}))
}
fn worker(m: &Manager, id: &str) -> Result<()> {
    worker_with_capacity(m, id, OPERATOR_WAIT, &|| live_capacity(m).ok())
}
fn worker_with_capacity(
    m: &Manager,
    id: &str,
    timeout: Duration,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
) -> Result<()> {
    // Prevent a duplicate worker from executing the same operation twice.
    let dir = job_dir(m, id)?;
    let _owner = m.lock(&format!("operator-worker-{id}.lock"))?;
    let prior = optional(&dir.join("result.json"))?;
    if matches!(prior["state"].as_str(), Some("completed" | "refused")) {
        return Ok(());
    }
    let request: ui::Request = read_json(&dir.join("request.json"))?;
    write_operation(
        m,
        id,
        &json!({"schema":1,"operation":id,"state":"waiting","stage":"operator_validation_readback","action":request.action}),
        false,
    )?;
    let mut waits = vec![];
    let validation = if matches!(request.action, ui::Action::SupportExport {}) {
        validate_current_request(m, &request)
    } else if current_offer_action(&request.action)
        || preparation_cli::is_action(&request.action)
        || scoped_product_action(&request.action) {
        // Admission already bound the exact manager offer. The action owner
        // performs its target-specific physical check before mutation.
        validate_current_worker(m, &request, id, timeout, &mut waits, capacity_read)
    } else {
        snapshot_for_operation(m, Some(id), timeout, &mut waits, capacity_read)
            .and_then(|snapshot| validate(&request, &snapshot))
    };
    if let Err(e) = validation {
        let failure = e
            .downcast_ref::<linux_vst_bridge::operator_lock::AcquisitionFailure>()
            .map(|f| {
                let retryable = f.facts.outcome == ui::LockOutcome::Timeout;
                ui::OperationFailure {
                    layer: ui::FailureLayer::ManagerControlPlane,
                    stage: ui::FailureStage::OperatorValidationReadback,
                    code: if !retryable {
                        ui::FailureCode::LockAccessError
                    } else if f.facts.name == ui::OperatorLock::Registry {
                        ui::FailureCode::RegistryLockTimeout
                    } else {
                        ui::FailureCode::SerializationLockTimeout
                    },
                    retryable,
                    mutation_started: false,
                    environment_created: false,
                    installer_launched: false,
                    lock: f.facts.clone(),
                }
            });
        let reason = if failure.as_ref().is_some_and(|f| f.retryable) {
            if matches!(
                request.action,
                ui::Action::InstallerEnvironmentCreate { .. }
                    | ui::Action::InstallerNewAttempt { .. }
            ) {
                "Manager busy validating state. Environment creation did not start; installer was not launched. Retry after canonical state is healthy.".to_owned()
            } else {
                "Manager busy validating state. The requested action did not start. Retry after canonical state is healthy.".to_owned()
            }
        } else {
            format!("Operator validation readback: {e}")
                .chars()
                .take(512)
                .collect()
        };
        return write_operation(
            m,
            id,
            &json!({"schema":1,"operation":id,"state":"refused","reason":reason,"failure":failure,"preparation_failure":preparation_cli::failure(&request.action),"lock_waits":waits}),
            false,
        );
    }
    // Snapshot's registry authority and serialization have both been released.
    // Action owners reacquire serialization and revalidate mutation authority.
    // Environment creation uses one bounded registry guard; validation alone
    // grants no mutation.
    write_operation(
        m,
        id,
        &json!({"schema":1,"operation":id,"state":"running","action":request.action,"lock_waits":waits}),
        false,
    )?;
    let result = execute_with_receipt_policy(
        m,
        &request.action,
        Some(id),
        capacity_read,
        timeout,
        &mut waits,
    );
    let value = match result {
        Ok(v) => {
            json!({"schema":1,"operation":id,"state":"completed","result":v,"lock_waits":waits})
        }
        Err(e) => {
            let failure = e
                .downcast_ref::<linux_vst_bridge::operator_lock::AcquisitionFailure>()
                .filter(|_| {
                    matches!(
                        request.action,
                        ui::Action::InstallerEnvironmentCreate { .. }
                            | ui::Action::InstallerNewAttempt { .. }
                    )
                })
                .map(|f| ui::OperationFailure {
                    layer: ui::FailureLayer::ManagerControlPlane,
                    stage: ui::FailureStage::EnvironmentCreationAdmission,
                    code: if f.facts.outcome != ui::LockOutcome::Timeout {
                        ui::FailureCode::LockAccessError
                    } else if f.facts.name == ui::OperatorLock::Registry {
                        ui::FailureCode::RegistryLockTimeout
                    } else {
                        ui::FailureCode::SerializationLockTimeout
                    },
                    retryable: f.facts.outcome == ui::LockOutcome::Timeout,
                    mutation_started: false,
                    environment_created: false,
                    installer_launched: false,
                    lock: f.facts.clone(),
                });
            let reason = if failure.as_ref().is_some_and(|f| f.retryable) {
                "Manager busy before environment creation. No environment was created; installer was not launched. Retry after healthy canonical refresh.".to_owned()
            } else {
                format!("Operator action: {e}").chars().take(512).collect()
            };
            json!({"schema":1,"operation":id,"state":"refused","reason":reason,"failure":failure,"preparation_failure":preparation_cli::failure(&request.action),"lock_waits":waits})
        }
    };
    write_operation(m, id, &value, false)
}
pub(super) fn run(m: &Manager, args: &[String]) -> Result<()> {
    match args {
        [a] if a == "overview" => {
            let current = overview(m)?;
            let serialization_at = Instant::now();
            let encoded = serde_json::to_string(&current)?;
            #[cfg(feature = "pb0-c0-audit")]
            eprintln!("PB0_PHASE {}",json!({"serialization_ms":serialization_at.elapsed().as_millis(),
                "products":current.current.products.len(),"setups":current.current.installer_setups.len()}));
            #[cfg(not(feature = "pb0-c0-audit"))]
            let _ = serialization_at;
            println!("{encoded}");
        }
        [a, environment, module, class] if a == "product" => {
            println!("{}",serde_json::to_string(&product_detail(m,environment,module,class)?)?);
        }
        #[cfg(feature = "pb0-c0-audit")]
        [a] if a == "support-export-preview" => {
            let current = overview(m)?;
            println!("{}", serde_json::to_string(&readiness::audit_export(m, &current)?)?);
        }
        [a] if a == "snapshot" => println!("{}", serde_json::to_string(&snapshot(m)?)?),
        [a] if a == "activity" => println!("{}", serde_json::to_string(&activity(m)?)?),
        [a] if a == "pulse" => println!("{}", serde_json::to_string(&pulse(m)?)?),
        [a] if a == "request" => {
            let mut bytes = Vec::new();
            std::io::stdin().take(16385).read_to_end(&mut bytes)?;
            require(bytes.len() <= 16384, "operator_request_bound")?;
            let req = serde_json::from_slice(&bytes)?;
            let receipt = match dispatch(m, req) {
                Ok(r) => r,
                Err(e) => ui::Receipt {
                    schema: ui::OPERATOR_SCHEMA,
                    accepted: false,
                    operation: None,
                    refusal: Some(e.to_string()),
                },
            };
            println!("{}", serde_json::to_string(&receipt)?);
        }
        [a, id] if a == "worker" => worker(m, id)?,
        [a, id] if a == "finish" => finish_operation(m, id)?,
        _ => return Err("operator snapshot | request".into()),
    }
    Ok(())
}

/// Retained per-product feedback is selected by exact request identity, never by
/// whichever global receipt happened to arrive last.
pub(super) fn product_receipt(
    m: &Manager,
    selection: &str,
    candidate: Option<&str>,
    exclude_operation: Option<&str>,
) -> Result<Option<Value>> {
    let dir = m.root.join("operator");
    if !dir.exists() {
        return Ok(None);
    }
    let mut found = vec![];
    for entry in fs::read_dir(dir)?.take(1025) {
        let p = entry?.path();
        let Some(id) = p.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if !valid_hex(id, 32) || exclude_operation == Some(id) || !p.is_dir() {
            continue;
        }
        let r = optional(&p.join("request.json"))?;
        let matched = r["action"]["selection"] == selection
            || candidate.is_some_and(|c| r["action"]["candidate"] == c);
        if matched {
            let result = optional(&p.join("result.json"))?;
            if !result.is_null() {
                found.push((fs::metadata(p.join("request.json"))?.modified()?, result));
            }
        }
    }
    found.sort_by_key(|(time, _)| *time);
    Ok(found.pop().map(|(_, r)| r))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "cpi2-test-contract")]
    use linux_vst_bridge::{catalogue::EnvironmentBinding, observation::Census, profiles::{Accessibility, Claim, Editor, Family, Limitation, Role}};
    #[test]
    fn empty_registry_bootstraps_without_inventing_a_native_catalogue() {
        let f = test_fixture::Fixture::new();
        let source = f.r.host.path.with_file_name("host-source-manifest.json");
        fs::write(&source, b"fixture source").unwrap();
        let sw = Software {
            installer_launch: None,
            preparation_kit: None,
            operator_frontend: None,
            manager: f.r.host.clone(),
            supervisor: f.r.host.clone(),
            ownership: f.r.host.clone(),
            host: f.r.host.clone(),
            source_manifest: Artifact { path: source.clone(), sha256: digest(&source).unwrap() },
            source_sha256: digest(&source).unwrap(),
            native_catalogue: None,
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let empty = f.m.registry().unwrap();
        assert!(operator_catalogue(&f.m, &sw, &empty).unwrap().is_none());
        let snapshot = snapshot_idle_test(&f.m).unwrap();
        assert!(snapshot.environments.is_empty());
        assert!(snapshot.products.is_empty());
        assert!(managed_rescan_binding(&f.m, None, &empty, &f.r.environment.id).is_err());
        let (occupied, _, _, _) = test_fixture::prepared();
        assert!(operator_catalogue(&f.m, &sw, &occupied.m.registry().unwrap()).is_err());
    }
    fn quarantined_scan(f: &test_fixture::Fixture) -> inventory::Scan {
        inventory::Scan {schema:1,id:"aa".repeat(16),environment:f.r.environment.clone(),
            host:f.r.host.clone(),host_source_sha256:f.r.host_source_sha256.clone(),
            completed_at:1,changes:inventory::Changes::default(),modules:vec![inventory::Module {
                artifact:f.r.module.clone(),classes:vec![],report:f.r.host.clone(),
                inspection_error:Some("TimeoutError: Windows call deadline: load_library".into()),
                quarantine_reason:Some("inventory_factory_absent_or_duplicate".into()),
            }]}
    }
    #[test]
    fn quarantine_projection_retains_scanner_cause_and_exact_retry_identity() {
        let f=test_fixture::Fixture::new();
        let scan=quarantined_scan(&f);
        let product=quarantined_product(&scan,0,scan.modules[0].clone(),None,None,None).unwrap();
        assert_eq!(product.limitations[0],
            "Scanner reported: TimeoutError: Windows call deadline: load_library");
        assert_eq!(product.limitations[1],"inventory_factory_absent_or_duplicate");
        assert_eq!(product.details["inspection_error"],
            "TimeoutError: Windows call deadline: load_library");
        assert_eq!(product.actions.len(),1);
        assert_eq!(product.actions[0].action,ui::Action::QuarantinedModuleRetry {
            environment:scan.environment.id.clone(),scan:scan.id.clone(),module_index:0,
            module_sha256:scan.modules[0].artifact.sha256.clone(),
            report_sha256:scan.modules[0].report.sha256.clone(),
        });
    }
    #[test]
    fn ordinary_rescan_reuses_quarantine_but_exact_retry_rescans_only_target() {
        let f=test_fixture::Fixture::new();
        let mut scan=quarantined_scan(&f);
        let other_path=f.outer.join("other.vst3");
        fs::write(&other_path,b"other module").unwrap();
        scan.modules.push(inventory::Module {artifact:Artifact {
            sha256:digest(&other_path).unwrap(),path:other_path},classes:vec![],
            report:f.r.host.clone(),inspection_error:None,quarantine_reason:None});
        let modules=scan.modules.iter().map(|module|module.artifact.clone()).collect::<Vec<_>>();
        assert!(reusable_scan_module(Some(&scan),&scan.environment,&scan.host,
            &scan.host_source_sha256,&modules[0],None).unwrap().is_some());
        let retry=QuarantinedRetry {scan:scan.id.clone(),module_index:0,
            module_sha256:modules[0].sha256.clone(),
            report_sha256:scan.modules[0].report.sha256.clone()};
        let target=exact_retry_target(Some(&scan),&scan.environment,&scan.host,
            &scan.host_source_sha256,&modules,&retry).unwrap().artifact.clone();
        assert!(reusable_scan_module(Some(&scan),&scan.environment,&scan.host,
            &scan.host_source_sha256,&modules[0],Some(&target)).unwrap().is_none());
        assert_eq!(reusable_scan_module(Some(&scan),&scan.environment,&scan.host,
            &scan.host_source_sha256,&modules[1],Some(&target)).unwrap()
            .unwrap().artifact,modules[1]);
        let mut changed=retry;
        changed.report_sha256="ff".repeat(32);
        assert!(exact_retry_target(Some(&scan),&scan.environment,&scan.host,
            &scan.host_source_sha256,&modules,&changed).is_err());
    }
    #[test]
    fn exact_retry_accepts_verified_scanner_relocation_but_rejects_changed_identity() {
        let f=test_fixture::Fixture::new();
        let scan=quarantined_scan(&f);
        let modules=scan.modules.iter().map(|module|module.artifact.clone()).collect::<Vec<_>>();
        let retry=QuarantinedRetry {scan:scan.id.clone(),module_index:0,
            module_sha256:modules[0].sha256.clone(),
            report_sha256:scan.modules[0].report.sha256.clone()};
        let generation=f.outer.join("immutable-generation");
        private_dir(&generation).unwrap();
        let relocated_path=generation.join("host.exe");
        fs::copy(&scan.host.path,&relocated_path).unwrap();
        let relocated=Artifact {path:relocated_path.clone(),sha256:scan.host.sha256.clone()};
        assert!(exact_retry_target(Some(&scan),&scan.environment,&relocated,
            &scan.host_source_sha256,&modules,&retry).is_ok());

        fs::write(&relocated_path,b"changed scanner bytes").unwrap();
        let changed=Artifact {sha256:digest(&relocated_path).unwrap(),path:relocated_path};
        assert!(exact_retry_target(Some(&scan),&scan.environment,&changed,
            &scan.host_source_sha256,&modules,&retry).is_err());
        assert!(exact_retry_target(Some(&scan),&scan.environment,&scan.host,
            &"ff".repeat(32),&modules,&retry).is_err());
    }
    #[test]
    fn retry_route_refuses_unmanaged_onboarding_environment_without_promoting_it() {
        use linux_vst_bridge::catalogue::{Catalogue,EnvironmentBinding};
        let (f,p,_,native)=test_fixture::prepared();
        let catalogue=Catalogue {schema:3,natives:vec![native],environments:vec![
            EnvironmentBinding {family:p.requirements.environment_family,
                environment:f.r.environment.clone()}],hosts:vec![],onboarding_runtime:None};
        let catalogue_path=f.m.root.join("software/catalogue.json");
        atomic_json(&catalogue_path,&catalogue).unwrap();
        let a=f.r.host.clone();
        let sw=Software {installer_launch:None,preparation_kit:None,manager:a.clone(),
            operator_frontend:None,supervisor:a.clone(),ownership:a.clone(),host:a,
            source_manifest:Artifact {path:f.r.host.path.with_file_name("host-source-manifest.json"),
                sha256:f.r.host_source_sha256.clone()},source_sha256:f.r.host_source_sha256.clone(),
            native_catalogue:Some(Artifact {sha256:digest(&catalogue_path).unwrap(),
                path:catalogue_path})};
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();
        let mut environment=f.r.environment.clone();
        environment.id="11".repeat(16);
        environment.root=f.m.root.join("environments").join(&environment.id);
        private_dir(&environment.root).unwrap();
        atomic_json(&environment.root.join("environment.json"),&environment).unwrap();
        let mut bytes=vec![0;1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60]=128;
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148]=2;
        bytes[150]=2;
        bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let source=f.outer.join("onboarding-installer.exe");
        fs::write(&source,bytes).unwrap();
        let installer=installer_import::import(&f.m,file(&source).unwrap()).unwrap();
        let directory=onboarding::directory(&f.m,&environment.id).unwrap();
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("record.json"),&onboarding::Record {schema:1,
            id:environment.id.clone(),installer:installer.id,environment:environment.clone(),
            created_at:1,creation_operation:"cd".repeat(16),
            installation_operation:None,published:false,previous_attempt:None}).unwrap();
        let module_path=environment.root.join(f.r.module.path
            .strip_prefix(&f.r.environment.root).unwrap());
        private_dir(module_path.parent().unwrap()).unwrap();
        fs::copy(&f.r.module.path,&module_path).unwrap();
        let mut scan=quarantined_scan(&f);
        scan.id="ef".repeat(16);
        scan.environment=environment.clone();
        scan.modules[0].artifact=Artifact {sha256:digest(&module_path).unwrap(),path:module_path};
        let inventory=f.m.root.join("inventory").join(format!("{}.json",environment.id));
        private_dir(inventory.parent().unwrap()).unwrap();
        atomic_json(&inventory,&scan).unwrap();
        let snapshot=snapshot_idle_test(&f.m).unwrap();
        let product=snapshot.products.iter().find(|product|
            product.environment==environment.id && product.disposition=="quarantined").unwrap();
        assert_eq!(product.actions.len(),1);
        assert_eq!(product.actions[0].disabled_reason.as_deref(),
            Some("This environment is not currently managed"));
        assert!(available(&snapshot).into_iter().all(|action|
            !matches!(action.action,ui::Action::QuarantinedModuleRetry { .. })
                || action.disabled_reason.is_some()));
        let before=fs::read(&inventory).unwrap();
        SCAN_SPAWN_COUNT.with(|count|count.set(0));
        let error=retry_quarantined_module(&f.m,&environment.id,&scan.id,0,
            &scan.modules[0].artifact.sha256,&scan.modules[0].report.sha256)
            .unwrap_err().to_string();
        assert_eq!(error,"operator_environment_absent");
        assert_eq!(SCAN_SPAWN_COUNT.with(std::cell::Cell::get),0);
        assert_eq!(fs::read(&inventory).unwrap(),before);
        assert!(!f.m.registry().unwrap().classes.values()
            .any(|entry|entry.registration.environment.id==environment.id));
    }
    fn view(token: &str, action: ui::AvailableAction) -> ui::Snapshot {
        ui::Snapshot {
            onboarding: vec![],
            installer_setups: vec![],
            schema: ui::OPERATOR_SCHEMA,
            state_token: token.into(),
            system: ui::System {
                service: "active".into(),
                keepers: 1,
                dsp: 0,
                maintenance: 0,
                ceiling: 6,
                pending_transactions: 0,
                stale_transports: 0,
                cleanup_unconfirmed: false,
            },
            environments: vec![],
            vendor_applications: vec![],
            products: vec![],
            workspaces: vec![],
            active_sessions: vec![],
            capture: Value::Null,
            recent_incidents: vec![],
            actions: vec![action],
            operation: None,
        }
    }
    #[cfg(feature = "cpi2-test-contract")]
    struct CatalogueFreeRetryFixture {
        fixture: test_fixture::Fixture,
        census: Census,
        software: Software,
        prior: inventory::Scan,
        _contract: frg1::TestContractGuard,
    }
    #[cfg(feature = "cpi2-test-contract")]
    fn catalogue_free_retry_fixture() -> CatalogueFreeRetryFixture {
        let (fixture, mut profile, mut census, native) = test_fixture::prepared();
        let m = &fixture.m;
        m.unpublish(&profile.class.class_id).unwrap();
        atomic_json(&m.root.join("registry.json"), &Registry::default()).unwrap();
        profile.id = "frg1.test.fragments".into();
        profile.revision = 11;
        profile.claim = Claim::ReviewCandidate;
        profile.role = Role::Effect;
        profile.class.name = "Efx FRAGMENTS".into();
        profile.class.vendor = "Arturia".into();
        profile.class.version = "1.3.1.6566".into();
        profile.class.subcategories = "Fx|Tools".into();
        profile.factory_vendor = "Arturia".into();
        profile.requirements.environment_family = Family::ArturiaPersistentV1;
        profile.requirements.environment_revision = 2;
        profile.requirements.native_sha256 = native.artifact.sha256.clone();
        profile.requirements.native_source_commit = native.source_commit.clone();
        profile.requirements.descriptor_sha256 = native.descriptor_sha256.clone();
        profile.capabilities.editor = Editor::DetachedDirectVendorLifecycle;
        profile.capabilities.accessibility = Accessibility::DisabledForVendorProcess;
        profile.limitations = vec![Limitation::ShortDeliveryGaps, Limitation::ExactOperatorArtifactOnly];
        let mut env = fixture.r.environment.clone();
        env.revision = 2;
        atomic_json(&env.root.join("environment.json"), &env).unwrap();
        let owner = env.root.join("compatdata/.ua1-owner.json");
        fs::write(&owner, b"exact retained ubuntu owner").unwrap();
        let relative = fixture.r.module.path.strip_prefix(env.root.join("compatdata"))
            .unwrap().to_str().unwrap().to_string();
        let contract = frg1::install_test_contract(profile.clone(), digest(&owner).unwrap(), relative);
        census.environment = EnvironmentBinding { family: Family::ArturiaPersistentV1,
            environment: env.clone() };
        census.selected = profile.class.clone();
        census.factory_vendor = profile.factory_vendor.clone();
        census.classes = vec![profile.class.class_id.clone()];
        let mut healthy = test_fixture::inspection_report(&census);
        let factory = healthy["records"].as_array_mut().unwrap().iter_mut()
            .find(|record| record["state"] == "ap8_factory").unwrap();
        factory["class_count"] = json!(1);
        factory["classes"].as_array_mut().unwrap().truncate(1);
        atomic_json(&census.report.path, &healthy).unwrap();
        census.report.sha256 = digest(&census.report.path).unwrap();
        let source_manifest = Artifact { path: fixture.r.host.path.with_file_name("host-source-manifest.json"),
            sha256: fixture.r.host_source_sha256.clone() };
        let software = Software { installer_launch: None, preparation_kit: None,
            operator_frontend: None, manager: fixture.r.host.clone(),
            supervisor: fixture.r.host.clone(), ownership: fixture.r.host.clone(),
            host: fixture.r.host.clone(), source_manifest,
            source_sha256: fixture.r.host_source_sha256.clone(), native_catalogue: None };
        atomic_json(&m.root.join("software.json"), &software).unwrap();
        frg1::adopt(m, &env.id).unwrap();
        let prior_report = fixture.outer.join("failed-scan-report.json");
        fs::write(&prior_report, b"{\"error\":\"first scan failed\",\"records\":[]}").unwrap();
        let other_path = census.module.path.with_file_name("z-other.vst3");
        fs::write(&other_path, b"other fixture module").unwrap();
        let other_artifact = Artifact { sha256: digest(&other_path).unwrap(), path: other_path };
        let mut other_healthy = healthy;
        other_healthy["records"][0]["module_sha256"] = json!(other_artifact.sha256);
        let other_report_path = fixture.outer.join("other-healthy-report.json");
        atomic_json(&other_report_path, &other_healthy).unwrap();
        let other_report = Artifact { sha256: digest(&other_report_path).unwrap(),
            path: other_report_path };
        let scan = inventory::Scan {
            schema: 1,
            id: "ab".repeat(16),
            environment: env,
            host: software.host.clone(),
            host_source_sha256: software.source_sha256.clone(),
            completed_at: 1,
            changes: inventory::Changes::default(),
            modules: vec![
                inventory::Module {
                    artifact: census.module.clone(),
                    classes: vec![],
                    report: Artifact {sha256:digest(&prior_report).unwrap(),path:prior_report},
                    inspection_error: Some("Windows host exited without successful close".into()),
                    quarantine_reason: Some("inventory_factory_absent_or_duplicate".into()),
                },
                inventory::Module {
                    artifact: other_artifact,
                    classes: inventory::classes(&other_healthy).unwrap(),
                    report: other_report,
                    inspection_error: None,
                    quarantine_reason: None,
                },
            ],
        };
        assert_eq!(managed_cli::modules(&scan.environment).unwrap().iter()
            .map(|module| &module.path).collect::<Vec<_>>(),
            scan.modules.iter().map(|module| &module.artifact.path).collect::<Vec<_>>());
        let inventory_root = m.root.join("inventory");
        private_dir(&inventory_root).unwrap();
        private_dir(&inventory_root.join("history")).unwrap();
        atomic_json(&inventory_root.join("history").join(format!("{}.json",scan.id)),&scan).unwrap();
        atomic_json(&inventory_root.join(format!("{}.json",scan.environment.id)),&scan).unwrap();
        CatalogueFreeRetryFixture { fixture, census, software, prior: scan, _contract: contract }
    }
    #[cfg(feature = "cpi2-test-contract")]
    #[test]
    fn catalogue_free_frg1_retry_worker_replaces_only_exact_quarantine() {
        let prepared = catalogue_free_retry_fixture();
        let prior = &prepared.prior;
        let m = &prepared.fixture.m;
        let mut sw = prepared.software.clone();
        assert!(sw.native_catalogue.is_none());
        assert!(m.registry().unwrap().classes.is_empty());

        // Only the external scanner process is deterministic test machinery;
        // request projection, worker validation, authority and inventory mutation
        // remain the production path.
        let launches = prepared.fixture.outer.join("scanner-launches");
        let script = m.root.join("software/test-supervisor.py");
        let source = serde_json::to_string(&prepared.census.report.path).unwrap();
        let count = serde_json::to_string(&launches).unwrap();
        fs::write(&script, format!("import json,pathlib,shutil,sys\njob=json.load(open(sys.argv[1]))\nwith open({source},'rb') as source,open(job['report'],'wb') as target: target.write(source.read())\nwith open({count},'a') as output: output.write('1\\n')\nprint('LVO0 '+job['session']+' ready',flush=True)\nshutil.rmtree(job['directory'])\nprint('LVO1 '+job['session']+' retired',flush=True)\n")).unwrap();
        sw.supervisor = Artifact {sha256:digest(&script).unwrap(),path:script};
        atomic_json(&m.root.join("software.json"),&sw).unwrap();

        let snapshot = snapshot_idle_test(m).unwrap();
        assert!(available(&snapshot).into_iter().all(|item|
            !matches!(item.action,ui::Action::EnvironmentRescan { .. })));
        let offered:Vec<_> = available(&snapshot).into_iter().filter(|item|
            matches!(item.action,ui::Action::QuarantinedModuleRetry { .. })).collect();
        assert_eq!(offered.len(),1);
        assert!(offered[0].disabled_reason.is_none());
        let action = offered[0].action.clone();
        assert_eq!(action,ui::Action::QuarantinedModuleRetry {
            environment:prior.environment.id.clone(),scan:prior.id.clone(),module_index:0,
            module_sha256:prior.modules[0].artifact.sha256.clone(),
            report_sha256:prior.modules[0].report.sha256.clone(),
        });
        let request = ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:snapshot.state_token,action};
        let operation = launch_queued(m,&request,|_|Ok(true)).unwrap().operation.unwrap();
        SCAN_SPAWN_COUNT.with(|count| count.set(0));
        worker_with_capacity(m,&operation,OPERATOR_WAIT,&||capacity_fixture(m)).unwrap();
        assert_eq!(worker_receipt(m,&operation)["state"],"completed");
        assert_eq!(SCAN_SPAWN_COUNT.with(std::cell::Cell::get),1);
        assert_eq!(fs::read_to_string(&launches).unwrap(),"1\n");
        let result = worker_receipt(m,&operation)["result"].clone();
        assert_eq!(result["retry"]["prior_scan"],prior.id);
        assert_eq!(result["retry"]["prior_report_sha256"],prior.modules[0].report.sha256);
        let inventory_root = m.root.join("inventory");
        let current:inventory::Scan = read_json(&inventory_root.join(format!("{}.json",prior.environment.id))).unwrap();
        assert_ne!(current.id,prior.id);
        assert_eq!(current.modules.len(),2);
        assert!(current.modules[0].quarantine_reason.is_none(), "{:?}", current.modules[0]);
        assert!(current.modules[0].inspection_error.is_none());
        assert!(!current.modules[0].classes.is_empty());
        assert_eq!(serde_json::to_value(&current.modules[1]).unwrap(),
            serde_json::to_value(&prior.modules[1]).unwrap());
        let retained:inventory::Scan = read_json(&inventory_root.join("history").join(format!("{}.json",prior.id))).unwrap();
        assert_eq!(serde_json::to_value(&retained).unwrap(),
            serde_json::to_value(prior).unwrap());
        assert!(available(&snapshot_idle_test(m).unwrap()).into_iter().all(|item|
            !matches!(item.action,ui::Action::QuarantinedModuleRetry { .. })));
    }
    #[cfg(feature = "cpi2-test-contract")]
    #[test]
    fn catalogue_free_frg1_retry_refuses_stale_or_unmanaged_inputs_without_scanning() {
        for case in ["adoption_absent", "adoption_changed", "environment", "scan",
            "module_index", "module_digest", "report_digest", "host", "source_manifest",
            "foreign_registry"] {
            let prepared = catalogue_free_retry_fixture();
            let m = &prepared.fixture.m;
            let prior = &prepared.prior;
            let mut environment = prior.environment.id.clone();
            let mut scan = prior.id.clone();
            let mut module_index = 0;
            let mut module_digest = prior.modules[0].artifact.sha256.clone();
            let mut report_digest = prior.modules[0].report.sha256.clone();
            match case {
                "adoption_absent" => {
                    fs::remove_file(m.root.join("qualifications/frg1/adoption.json")).unwrap();
                }
                "adoption_changed" => {
                    let path = m.root.join("qualifications/frg1/adoption.json");
                    let mut value: Value = read_json(&path).unwrap();
                    value["profile_fingerprint"] = json!("00".repeat(32));
                    atomic_json(&path, &value).unwrap();
                }
                "environment" => environment = "22".repeat(16),
                "scan" => scan = "cd".repeat(16),
                "module_index" => module_index = 1,
                "module_digest" => module_digest = "ef".repeat(32),
                "report_digest" => report_digest = "ef".repeat(32),
                "host" | "source_manifest" => {
                    let path = m.root.join("inventory").join(format!("{}.json", environment));
                    let mut changed: inventory::Scan = read_json(&path).unwrap();
                    if case == "host" { changed.host.sha256 = "ef".repeat(32); }
                    else { changed.host_source_sha256 = "ef".repeat(32); }
                    atomic_json(&path, &changed).unwrap();
                }
                "foreign_registry" => {
                    let (foreign, _, _, _) = test_fixture::prepared();
                    atomic_json(&m.root.join("registry.json"), &foreign.m.registry().unwrap()).unwrap();
                }
                _ => unreachable!(),
            }
            let current = m.root.join("inventory").join(format!("{}.json", prior.environment.id));
            let history = m.root.join("inventory/history").join(format!("{}.json", prior.id));
            let before_current = fs::read(&current).unwrap();
            let before_history = fs::read(&history).unwrap();
            SCAN_SPAWN_COUNT.with(|count| count.set(0));
            assert!(retry_quarantined_module(m, &environment, &scan, module_index,
                &module_digest, &report_digest).is_err(), "{case}");
            assert_eq!(SCAN_SPAWN_COUNT.with(std::cell::Cell::get), 0, "{case}");
            assert_eq!(fs::read(&current).unwrap(), before_current, "{case}");
            assert_eq!(fs::read(&history).unwrap(), before_history, "{case}");
        }
    }
    #[test]
    fn retired_terminal_summary_remains_in_manager_projection() {
        let class="01".repeat(16);
        let rows=session_projection(None,vec![capacity::TerminalSummary{
            schema:1,session:"ab".repeat(16),class_id:class.clone(),
            terminal:capacity::InstanceTerminal::WindowsHostExited,
            producer:3,status_domain:4,cleanup_confirmed:true,
            transport_retired:true,observed_at:1,
        }]);
        assert_eq!(rows,vec![json!({
            "session":"ab".repeat(16),
            "class_id":class,"state":"failed","terminal":"windows_host_exited",
            "recent":true,"cleanup_confirmed":true,"transport_retired":true,
            "observed_at":1,
        })]);
    }
    #[test]
    fn live_session_projection_retains_exact_read_only_identity() {
        let session = "cd".repeat(16);
        let class = "01".repeat(16);
        let capacity = CapacityReadback {
            schema: 1,
            dsp: 1,
            maintenance: 0,
            keepers: 0,
            cleanup_unconfirmed: false,
            owners: vec![capacity::Owner {
                session: session.clone(),
                class_id: class.clone(),
                kind: capacity::Kind::Dsp,
                terminal: None,
            }],
            limits: CapacityLimits { global_dsp: 6 },
        };
        let rows = session_projection(Some(&capacity), vec![]);
        assert_eq!(rows, vec![json!({
            "session": session,
            "class_id": class,
            "state": "active",
            "terminal": null,
            "recent": false,
        })]);
    }
    #[test]
    fn stale_foreign_candidate_busy_and_unknown_requests_cannot_dispatch() {
        let allowed = ui::Action::OrdinaryRollback {
            class_id: "a".repeat(32),
            publication: "b".repeat(32),
        };
        let s = view("current", action("Rollback", allowed.clone(), None));
        let mut r = ui::Request {
            schema: ui::OPERATOR_SCHEMA,
            state_token: "current".into(),
            action: allowed.clone(),
        };
        validate(&r, &s).unwrap();
        r.state_token = "previous software or registry".into();
        assert!(validate(&r, &s).is_err());
        r.state_token = "current".into();
        for old_schema in [8, 9, 10, 11] {
            r.schema = old_schema;
            assert_eq!(validate(&r, &s).unwrap_err().to_string(),
                "operator_schema_mismatch_update_manager_frontend");
        }
        for malformed in [json!({"schema":"12","state_token":"current","action":{"kind":"capture_disarm"}}),
            json!({"schema":null,"state_token":"current","action":{"kind":"capture_disarm"}})] {
            assert!(serde_json::from_value::<ui::Request>(malformed).is_err());
        }
        r.schema = ui::OPERATOR_SCHEMA;
        r.action = ui::Action::OrdinaryRollback {
            class_id: "a".repeat(32),
            publication: "c".repeat(32),
        };
        assert!(validate(&r, &s).is_err());
        r.action = ui::Action::OrdinaryRollback {
            class_id: "d".repeat(32),
            publication: "b".repeat(32),
        };
        assert!(validate(&r, &s).is_err());
        r.action = allowed.clone();
        let busy = view(
            "current",
            action("Rollback", allowed, Some("Active DSP lease")),
        );
        assert!(validate(&r, &busy).is_err());
    }
    #[test]
    fn current_schema_keeps_exact_old_operation_request_history_readable() {
        assert_eq!(ui::OPERATOR_SCHEMA, 13);
        let f = test_fixture::Fixture::new();
        let operation = "ab".repeat(16);
        let dir = job_dir(&f.m, &operation).unwrap();
        private_dir(&dir).unwrap();
        let old = ui::Request { schema: 8, state_token: "old-state".into(),
            action: ui::Action::DependencyPrepare {} };
        atomic_json(&dir.join("request.json"), &old).unwrap();
        atomic_json(&dir.join("result.json"),
            &json!({"schema":1,"operation":operation,"state":"completed"})).unwrap();
        let saved = ResumeRecord { schema: 1, owner_operation: operation.clone(),
            resume: true, software: "cd".repeat(32), vendor_operation: Some(operation.clone()) };
        assert_eq!(recovery_request(&f.m, &saved).unwrap(), old.action);
        assert_eq!(worker_receipt(&f.m, &operation)["state"], "completed");
        assert_eq!(worker_receipt(&f.m, &operation)["schema"], 1);
        let installed_ui2 = ui::Request { schema: 11, state_token: "installed-state".into(),
            action: ui::Action::CompatibilityFinishResult { operation: operation.clone() } };
        atomic_json(&dir.join("request.json"), &installed_ui2).unwrap();
        assert_eq!(recovery_request(&f.m, &saved).unwrap(), installed_ui2.action);
        assert_eq!(validate(&installed_ui2, &view("installed-state", action("Finish", installed_ui2.action.clone(), None)))
            .unwrap_err().to_string(), "operator_schema_mismatch_update_manager_frontend");
    }
    #[test]
    fn support_export_requires_the_current_overview_offer_and_token() {
        let f = test_fixture::Fixture::new();
        let source = f.r.host.path.with_file_name("host-source-manifest.json");
        fs::write(&source, b"fixture source").unwrap();
        let sw = Software {
            installer_launch: None,
            preparation_kit: None,
            operator_frontend: None,
            manager: f.r.host.clone(),
            supervisor: f.r.host.clone(),
            ownership: f.r.host.clone(),
            host: f.r.host.clone(),
            source_manifest: Artifact { path: source.clone(), sha256: digest(&source).unwrap() },
            source_sha256: digest(&source).unwrap(),
            native_catalogue: None,
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let offered = overview(&f.m).unwrap();
        assert!(matches!(offered.readiness.support_export_action.action,
            ui::Action::SupportExport {}));
        let mut request = ui::Request {schema: ui::OPERATOR_SCHEMA,
            state_token: offered.current.state_token.clone(),
            action: ui::Action::SupportExport {}};
        validate_current_request(&f.m, &request).unwrap();
        request.state_token = "stale".into();
        assert!(validate_current_request(&f.m, &request).is_err());
        request.state_token = offered.current.state_token;
        request.schema = 11;
        assert!(validate_current_request(&f.m, &request).is_err());
        let operation = test_submit_offered(&f.m, &ui::Action::SupportExport {}).unwrap();
        let result = test_run_offered_worker(&f.m, &operation).unwrap();
        assert_eq!(result["state"], "completed");
        let report = result["result"]["file"].as_str()
            .expect("the exact offered export writes a local file");
        let saved: Value = read_json(&f.m.root.join("support-exports").join(report)).unwrap();
        assert_eq!(saved["schema"], 2);
        assert!(saved["assessment"].get("state_token").is_none());
        assert!(saved["assessment"].get("support_export_action").is_none());
    }
    #[test]
    fn workspace_selection_accepts_only_exact_offered_import_and_release_syntax() {
        let installer = "ab".repeat(32);
        let offered = action(
            "Choose imported FL installer",
            ui::Action::WorkspaceSelectInstaller {
                installer: installer.clone(),
                release: String::new(),
            },
            None,
        );
        let mut snapshot = view(
            "current",
            action("Refresh", ui::Action::CaptureDisarm {}, None),
        );
        snapshot.workspaces.push(ui::DawWorkspace {
            id: "cd".repeat(16),
            name: "FL Studio".into(),
            state: "uninstalled".into(),
            selected_installer: installer.clone(),
            application_installers: vec![installer.clone()],
            selected_release: "26.1.6.0".into(),
            installed_advertised_release: None,
            observed_file_version: None,
            installed_image_sha256: None,
            active_installation_operation: None,
            cleanup: "confirmed".into(),
            first_useful_failure: None,
            actions: vec![],
            installer_choices: vec![offered],
            products: vec![],
            details: Value::Null,
        });
        let mut request = ui::Request {
            schema: ui::OPERATOR_SCHEMA,
            state_token: "current".into(),
            action: ui::Action::WorkspaceSelectInstaller {
                installer: installer.clone(),
                release: "27.0.0.0".into(),
            },
        };
        validate(&request, &snapshot).unwrap();
        if let ui::Action::WorkspaceSelectInstaller { release, .. } = &mut request.action {
            *release = "27..0".into();
        }
        assert!(validate(&request, &snapshot).is_err());
        request.action = ui::Action::WorkspaceSelectInstaller {
            installer: "ef".repeat(32),
            release: "27.0.0.0".into(),
        };
        assert!(validate(&request, &snapshot).is_err());
        snapshot.workspaces[0].products.push(ui::DawWorkspaceProduct {
            id: ui::WorkspaceProductId::Serum2,
            name: "Serum 2".into(),
            state: "not_selected".into(),
            selected_release: None,
            module_sha256: None,
            current_failure: None,
            actions: vec![action("Install Serum 2", ui::Action::WorkspaceInstallProduct {
                product: ui::WorkspaceProductId::Serum2,
            }, Some("Select exact installer first"))],
            installer_choices: vec![action("Choose Serum 2", ui::Action::WorkspaceSelectProductInstaller {
                product: ui::WorkspaceProductId::Serum2,
                installer: "12".repeat(32),
                release: String::new(),
            }, None)],
            details: Value::Null,
        });
        request.action = ui::Action::WorkspaceSelectProductInstaller {
            product: ui::WorkspaceProductId::Serum2,
            installer: "12".repeat(32),
            release: "2.1.5".into(),
        };
        validate(&request, &snapshot).unwrap();
        request.action = ui::Action::WorkspaceSelectProductInstaller {
            product: ui::WorkspaceProductId::Serum2,
            installer: "13".repeat(32),
            release: "2.1.5".into(),
        };
        assert!(validate(&request, &snapshot).is_err());
        request.action = ui::Action::WorkspaceInstallProduct { product: ui::WorkspaceProductId::Serum2 };
        assert!(validate(&request, &snapshot).is_err(), "disabled product action must remain unavailable");
        snapshot.workspaces[0].installer_choices[0].disabled_reason =
            Some("FL session is active".into());
        request.action = ui::Action::WorkspaceSelectInstaller {
            installer,
            release: "27.0.0.0".into(),
        };
        assert!(validate(&request, &snapshot).is_err());
    }
    fn capacity_json(blocked: bool, dsp: usize, maintenance: usize) -> Value {
        json!({"ok":true,"capacity":{"schema":1,"dsp":dsp,"maintenance":maintenance,"keepers":1,"cleanup_unconfirmed":blocked,"owners":[],"limits":{"global_dsp":6}}})
    }
    fn service_reply(m: &Manager, value: Value) -> std::thread::JoinHandle<()> {
        private_dir(&m.root.join("runtime")).unwrap();
        let socket = m.root.join("runtime/owner.sock");
        if socket.exists() {
            fs::remove_file(&socket).unwrap();
        }
        let listener = UnixListener::bind(&socket).unwrap();
        std::thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            let mut greeting = [0; 5];
            peer.read_exact(&mut greeting).unwrap();
            assert_eq!(&greeting, b"LVC1\n");
            let bytes = serde_json::to_vec(&value).unwrap();
            peer.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
            peer.write_all(&bytes).unwrap();
        })
    }
    fn overview_service_reply(m: &Manager, value: Value) -> std::thread::JoinHandle<()> {
        private_dir(&m.root.join("runtime")).unwrap();
        let socket=m.root.join("runtime/owner.sock");
        if socket.exists() {fs::remove_file(&socket).unwrap();}
        let listener=UnixListener::bind(&socket).unwrap();
        std::thread::spawn(move || {
            // Capacity and the first cleanup probe run concurrently. The
            // final recheck samples cleanup on both sides of registry.lock.
            let mut capacity = 0;
            let mut pulses = 0;
            for _ in 0..4 {
                let (mut peer,_)=listener.accept().unwrap();
                let mut greeting=[0;5];
                peer.read_exact(&mut greeting).unwrap();
                if &greeting==b"LVC1\n" {
                    capacity += 1;
                    let bytes=serde_json::to_vec(&value).unwrap();
                    peer.write_all(&(bytes.len() as u32).to_le_bytes()).unwrap();
                    peer.write_all(&bytes).unwrap();
                } else {
                    assert_eq!(&greeting,b"LVP1\n");
                    pulses += 1;
                    peer.write_all(&[0]).unwrap();
                }
            }
            assert_eq!((capacity,pulses),(1,3));
        })
    }
    #[test]
    fn pulse_cleanup_tracks_blocked_state_without_owner_change() {
        let f=test_fixture::Fixture::new();
        private_dir(&f.m.root.join("runtime")).unwrap();
        let socket=f.m.root.join("runtime/owner.sock");
        for blocked in [false,true] {
            if socket.exists() {fs::remove_file(&socket).unwrap();}
            let listener=UnixListener::bind(&socket).unwrap();
            let server=std::thread::spawn(move || {
                let (mut peer,_)=listener.accept().unwrap();
                let mut greeting=[0;5];
                peer.read_exact(&mut greeting).unwrap();
                assert_eq!(&greeting,b"LVP1\n");
                peer.write_all(&[u8::from(blocked)]).unwrap();
            });
            assert_eq!(pulse_cleanup(&f.m),Some(blocked));
            server.join().unwrap();
        }
    }
    #[test]
    fn pulse_classifies_all_retained_nonterminal_operation_states() {
        let f=test_fixture::Fixture::new();
        let latest=f.m.root.join("operator/latest.json");
        private_dir(latest.parent().unwrap()).unwrap();
        for (state,expected_live) in [
            ("queued",true),("waiting",true),("running",true),("vendor_running",true),
            ("completed",false),("refused",false),
        ] {
            atomic_json(&latest,&json!({"schema":1,"operation":"ab".repeat(16),
                "state":state})).unwrap();
            assert_eq!(pulse(&f.m).unwrap().operation_live,expected_live,"{state}");
        }
    }
    #[test]
    fn selected_performance_record_changes_pulse_generation() {
        let f=test_fixture::Fixture::new();
        let before=pulse_generation(&f.m).unwrap();
        let path=f.m.root.join("performance").join(format!("{}.json",f.r.key()));
        private_dir(path.parent().unwrap()).unwrap();
        atomic_json(&path,&Performance {schema:1,added_frames:512}).unwrap();
        let selected=pulse_generation(&f.m).unwrap();
        assert_ne!(before,selected);
        atomic_json(&path,&Performance {schema:1,added_frames:256}).unwrap();
        assert_ne!(selected,pulse_generation(&f.m).unwrap());
    }
    #[test]
    fn authoritative_blocked_readback_and_unavailable_service_never_claim_safe_cleanup() {
        let f = test_fixture::Fixture::new();
        let server = service_reply(&f.m, capacity_json(true, 1, 0));
        let a = activity(&f.m).unwrap();
        server.join().unwrap();
        assert!(a.system.capacity_available());
        assert!(a.system.cleanup_unconfirmed);
        assert_eq!(a.system.dsp, 1);
        assert_eq!(
            a.system.inactive_reason(),
            Some("Previous instance cleanup is unconfirmed")
        );
        let a = activity(&f.m).unwrap();
        assert!(!a.system.capacity_available());
        assert!(a.system.cleanup_unconfirmed);
        assert!(a.system.inactive_reason().unwrap().contains("unavailable"));
    }
    #[test]
    fn malformed_or_refused_capacity_is_unavailable_not_safe() {
        let f = test_fixture::Fixture::new();
        let mut missing = capacity_json(false, 0, 0);
        missing["capacity"]
            .as_object_mut()
            .unwrap()
            .remove("cleanup_unconfirmed");
        let mut future = capacity_json(false, 0, 0);
        future["capacity"]["schema"] = json!(2);
        for reply in [missing, future, json!({"ok":false,"refusal":"busy"})] {
            let server = service_reply(&f.m, reply);
            let read = activity(&f.m).unwrap();
            server.join().unwrap();
            assert!(!read.system.capacity_available());
            assert!(read.system.cleanup_unconfirmed);
        }
    }
    #[test]
    fn crafted_inactive_actions_refuse_each_live_boundary_before_any_mutation() {
        let f = test_fixture::Fixture::new();
        atomic_json(&f.m.root.join("sentinel.json"), &json!({"unchanged":true})).unwrap();
        let actions = [
            ui::Action::InstallerStart {
                onboarding: "ef".repeat(16),
            },
            ui::Action::InstallerScan {
                onboarding: "ef".repeat(16),
            },
            ui::Action::TransactionReconcile {},
            ui::Action::EnvironmentRescan {
                environment: f.r.environment.id.clone(),
            },
            ui::Action::QuarantinedModuleRetry {
                environment: f.r.environment.id.clone(),
                scan: "ab".repeat(16),
                module_index: 0,
                module_sha256: "cd".repeat(32),
                report_sha256: "ef".repeat(32),
            },
            ui::Action::VendorApplicationOpen {
                application: ASC.into(),
            },
            ui::Action::OrdinaryRollback {
                class_id: f.r.key(),
                publication: "a".repeat(32),
            },
            ui::Action::OrdinaryRestoreRecommended {
                class_id: f.r.key(),
            },
        ];
        for action in &actions {
            for (blocked, dsp, maintenance) in [(true, 0, 0), (false, 1, 0), (false, 0, 1)] {
                let server = service_reply(&f.m, capacity_json(blocked, dsp, maintenance));
                let e = execute(&f.m, action).unwrap_err().to_string();
                server.join().unwrap();
                assert!(e.contains("cleanup") || e.contains("active bridged"), "{e}");
            }
            let cap: CapacityReadback =
                serde_json::from_value(capacity_json(false, 0, 0)["capacity"].clone()).unwrap();
            assert!(inactive_reason(Some(&cap), false, 0, false)
                .unwrap()
                .contains("Software Center"));
            if !matches!(action, ui::Action::TransactionReconcile {}) {
                assert!(inactive_reason(Some(&cap), true, 1, false)
                    .unwrap()
                    .contains("Reconcile"));
            }
        }
        assert!(!f.m.root.join("registry.json").exists());
        assert!(!f.m.root.join("operator/resume.json").exists());
    }
    #[test]
    fn environment_creation_rechecks_live_capacity_at_mutation() {
        for (blocked, dsp, maintenance) in [(true, 0, 0), (false, 1, 0), (false, 0, 1)] {
            let (f, id) = onboarding_worker_fixture();
            let request: ui::Request =
                read_json(&job_dir(&f.m, &id).unwrap().join("request.json")).unwrap();
            let e = execute_with_receipt_capacity(&f.m, &request.action, Some(&id), &|| {
                let mut cap = capacity_fixture(&f.m).unwrap();
                cap.cleanup_unconfirmed = blocked;
                cap.dsp = dsp;
                cap.maintenance = maintenance;
                Some(cap)
            })
            .unwrap_err()
            .to_string();
            assert!(e.contains("cleanup") || e.contains("active bridged"), "{e}");
            assert!(!f.m.root.join("onboarding").exists());
            assert!(!f.m.root.join("operator/resume.json").exists());
        }
    }
    #[test]
    fn mutation_owners_recheck_global_inactivity_under_their_registry_lock() {
        let (f, p, c, n) = test_fixture::prepared();
        let r = observation::derive(&p, &c, &n).unwrap();
        let key = r.key();
        let first =
            f.m.managed_publish(&p, &c, r.clone(), &c.host, &c.host_source_sha256, None)
                .unwrap();
        let before = fs::read(f.m.root.join("registry.json")).unwrap();
        let mut binding: HostBinding = f.r.clone().into();
        binding.metadata.class_id = "aa".repeat(16);
        let (job, _) = spec(&f.m, binding, true, false, false).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        assert!(f.m.reconcile_inactive().is_err());
        assert!(f.m.rollback_inactive(&key, &first.id, None).is_err());
        assert!(f
            .m
            .managed_publish_inactive(&p, &c, r, &c.host, &c.host_source_sha256, None)
            .is_err());
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
    }
    #[test]
    fn terminal_receipt_waits_for_an_existing_writer_instead_of_leaving_running() {
        let f = test_fixture::Fixture::new();
        let request = ui::Request {
            schema: ui::OPERATOR_SCHEMA,
            state_token: "t".into(),
            action: ui::Action::CaptureDisarm {},
        };
        let id = launch_queued(&f.m, &request, |_| Ok(true))
            .unwrap()
            .operation
            .unwrap();
        let held = f.m.lock("operator-receipt.lock").unwrap();
        let m = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        let child_id = id.clone();
        let child =
            std::thread::spawn(move || finish_operation(&m, &child_id).map_err(|e| e.to_string()));
        std::thread::sleep(Duration::from_millis(40));
        assert_eq!(
            optional(&job_dir(&f.m, &id).unwrap().join("result.json")).unwrap()["state"],
            "queued"
        );
        drop(held);
        child.join().unwrap().unwrap();
        assert_eq!(
            optional(&job_dir(&f.m, &id).unwrap().join("result.json")).unwrap()["state"],
            "refused"
        );
    }
    #[test]
    fn failed_launch_and_dead_worker_have_terminal_receipts_without_overwriting_new_jobs() {
        let f = test_fixture::Fixture::new();
        let request = ui::Request {
            schema: ui::OPERATOR_SCHEMA,
            state_token: "t".into(),
            action: ui::Action::CaptureDisarm {},
        };
        for io_failure in [false, true] {
            let result = launch_queued(&f.m, &request, |_| {
                if io_failure {
                    Err("launch io error".into())
                } else {
                    Ok(false)
                }
            });
            assert!(result.is_err());
            let last = optional(&f.m.root.join("operator/latest.json")).unwrap();
            assert_eq!(last["state"], "refused");
            assert_eq!(last["reason"], "operator_worker_launch_failed");
            assert_eq!(
                optional(
                    &job_dir(&f.m, last["operation"].as_str().unwrap())
                        .unwrap()
                        .join("result.json")
                )
                .unwrap(),
                last
            );
        }
        for state in ["queued", "waiting", "running", "vendor_running"] {
            let id = launch_queued(&f.m, &request, |_| Ok(true))
                .unwrap()
                .operation
                .unwrap();
            write_operation(
                &f.m,
                &id,
                &json!({"schema":1,"operation":id,"state":state}),
                false,
            )
            .unwrap();
            finish_operation(&f.m, &id).unwrap();
            let last = optional(&f.m.root.join("operator/latest.json")).unwrap();
            assert_eq!(last["state"], "refused");
            assert_eq!(last["reason"], "operator_worker_terminated");
        }
        let old = launch_queued(&f.m, &request, |_| Ok(true))
            .unwrap()
            .operation
            .unwrap();
        let new = launch_queued(&f.m, &request, |_| Ok(true))
            .unwrap()
            .operation
            .unwrap();
        finish_operation(&f.m, &old).unwrap();
        assert_eq!(
            optional(&f.m.root.join("operator/latest.json")).unwrap()["operation"],
            new
        );
        let completed =
            json!({"schema":1,"operation":new,"state":"completed","result":{"done":true}});
        write_operation(&f.m, &new, &completed, false).unwrap();
        atomic_json(
            &f.m.root.join("operator/latest.json"),
            &json!({"operation":new,"state":"running"}),
        )
        .unwrap();
        finish_operation(&f.m, &new).unwrap();
        assert_eq!(
            optional(&f.m.root.join("operator/latest.json")).unwrap(),
            completed
        );
    }
    fn queued_test_action(m: &Manager, action: ui::Action) -> String {
        launch_queued(
            m,
            &ui::Request {
                schema: ui::OPERATOR_SCHEMA,
                state_token: "fixture".into(),
                action,
            },
            |_| Ok(true),
        )
        .unwrap()
        .operation
        .unwrap()
    }
    fn saved_test_resume(m: &Manager, owner: &str, vendor: Option<String>) -> ResumeRecord {
        let saved = ResumeRecord {
            schema: 1,
            owner_operation: owner.into(),
            resume: true,
            software: "ab".repeat(32),
            vendor_operation: vendor,
        };
        let _lock = resume_lock(m).unwrap();
        create_resume(m, &saved).unwrap();
        saved
    }
    #[test]
    fn interrupted_compatibility_continuation_restores_service_without_replaying_work() {
        for terminal in ["completed", "refused"] {
            let f = test_fixture::Fixture::new();
            let source = "cd".repeat(16);
            let owner = queued_test_action(&f.m, ui::Action::CompatibilityResumeCheck {
                operation: source.clone(),
            });
            let saved = saved_test_resume(&f.m, &owner, None);
            let resume_path = f.m.root.join("operator/resume.json");
            let resume_before = fs::read(&resume_path).unwrap();
            atomic_json(&f.m.root.join("registry.json"), &f.m.registry().unwrap()).unwrap();
            let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
            let scratch = f.m.root.join("preparation/work").join(&source).join("scratch");
            private_dir(scratch.parent().unwrap()).unwrap();
            fs::write(&scratch, b"cold recovery must not replay preparation").unwrap();
            assert!(resume_interrupted_with(&f.m, |_| panic!("owner is not terminal"))
                .unwrap_err().to_string().contains("operator_resume_owner_not_terminal"));
            assert_eq!(fs::read(&resume_path).unwrap(), resume_before);
            write_operation(&f.m, &owner,
                &json!({"schema":1,"operation":owner,"state":terminal}), false).unwrap();
            let receipt_before = fs::read(job_dir(&f.m, &owner).unwrap().join("result.json")).unwrap();
            assert_eq!(resume_interrupted_with(&f.m, |_| Err("service unavailable".into()))
                .unwrap_err().to_string(), "service unavailable");
            assert_eq!(fs::read(&resume_path).unwrap(), resume_before);
            let mut restores = 0;
            resume_interrupted_with(&f.m, |record| {
                assert_eq!(record.owner_operation, owner);
                assert_eq!(record.software, saved.software);
                assert!(record.resume && record.vendor_operation.is_none());
                restores += 1;
                Ok(())
            }).unwrap();
            assert_eq!(restores, 1);
            assert!(!resume_path.exists());
            resume_interrupted_with(&f.m, |_| panic!("already restored")).unwrap();
            assert_eq!(fs::read(job_dir(&f.m, &owner).unwrap().join("result.json")).unwrap(), receipt_before);
            assert_eq!(fs::read(&scratch).unwrap(), b"cold recovery must not replay preparation");
            assert!(!f.m.root.join("preparation/guided-checks").exists());
            assert!(!f.m.root.join("preparation/candidates").exists());
            assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry_before);
        }
    }
    #[test]
    fn delayed_finish_preserves_newer_service_recovery_until_its_owner_restores() {
        let f = test_fixture::Fixture::new();
        let a = queued_test_action(&f.m, ui::Action::CaptureDisarm {});
        let completed =
            json!({"schema":1,"operation":a,"state":"completed","result":{"capture":"disarmed"}});
        write_operation(&f.m, &a, &completed, false).unwrap();
        let b = queued_test_action(
            &f.m,
            ui::Action::EnvironmentRescan {
                environment: f.r.environment.id.clone(),
            },
        );
        let saved = saved_test_resume(&f.m, &b, None);
        let path = f.m.root.join("operator/resume.json");
        let before = fs::read(&path).unwrap();
        let latest = fs::read(f.m.root.join("operator/latest.json")).unwrap();
        let mut restores = 0;
        finish_operation_with(&f.m, &a, |_| {
            restores += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            restores, 0,
            "old finish must not even query/start recovery for B"
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(
            fs::read(f.m.root.join("operator/latest.json")).unwrap(),
            latest
        );
        assert_eq!(
            optional(&job_dir(&f.m, &a).unwrap().join("result.json")).unwrap(),
            completed
        );
        // The current record cannot be overwritten by another suspension.
        {
            let _lock = resume_lock(&f.m).unwrap();
            assert!(create_resume(&f.m, &saved).is_err());
        }
        assert_eq!(fs::read(&path).unwrap(), before);
        // Failure preserves the recovery obligation; a later exact-owner call
        // may complete it. These closures replace only external service I/O.
        assert!(resume_owned_with(&f.m, &b, |_| Err("service unavailable".into())).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        resume_owned_with(&f.m, &b, |r| {
            assert_eq!(r.owner_operation, b);
            assert!(r.resume);
            assert_eq!(r.software, saved.software);
            restores += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(restores, 1);
        assert!(!path.exists());
        finish_operation_with(&f.m, &a, |_| panic!("no record to recover")).unwrap();
    }
    #[test]
    fn renderer_stop_preserves_uncertain_resume_and_restores_exactly_once() {
        use renderer_session::{self as r, UnitState};
        let f = test_fixture::Fixture::new();
        let application = "ab".repeat(32);
        let owner = queued_test_action(
            &f.m,
            ui::Action::RendererOpen {
                application: application.clone(),
                policy: ui::RendererPolicy::Inherited,
            },
        );
        saved_test_resume(&f.m, &owner, Some(owner.clone()));
        let spec = json!({"operation":owner,"application_identity":application,"renderer_policy":"inherited","report":r::operation_dir(&f.m,&owner).unwrap().join("result.json")});
        assert_eq!(
            r::submit_with(
                &f.m,
                &spec,
                |_| Err("lost acknowledgment".into()),
                || Err("unit unavailable".into())
            )
            .unwrap(),
            r::Submission::AcknowledgmentUncertain
        );
        let path = f.m.root.join("operator/resume.json");
        let retained = fs::read(&path).unwrap();
        assert!(stop_renderer_with(
            &f.m,
            &"ff".repeat(16),
            |_| panic!("wrong stop"),
            |_| panic!("wrong restore")
        )
        .is_err());
        assert!(stop_renderer_with(
            &f.m,
            &owner,
            |id| r::stop_with(&f.m, id, || Ok(()), || Err("unknown".into())),
            |_| panic!("uncertain service must remain stopped")
        )
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), retained);
        let mut restores = 0;
        stop_renderer_with(
            &f.m,
            &owner,
            |id| {
                r::stop_with(
                    &f.m,
                    id,
                    || panic!("absent unit needs no signal"),
                    || {
                        Ok(UnitState {
                            exists: false,
                            empty: true,
                            observation: None,
                        })
                    },
                )
            },
            |saved| {
                assert_eq!(saved.owner_operation, owner);
                restores += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(restores, 1);
        assert!(!path.exists());
        resume_owned_with(&f.m, &owner, |_| {
            restores += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(restores, 1);
        assert!(r::terminal(&r::result(&f.m, &owner).unwrap(), &owner));
    }
    #[test]
    fn renderer_lost_ack_live_result_then_exact_stop_preserves_failure_and_resume_owner() {
        use renderer_session::{self as r, UnitState};
        let f = test_fixture::Fixture::new();
        let application = "ab".repeat(32);
        let owner = queued_test_action(
            &f.m,
            ui::Action::RendererOpen {
                application: application.clone(),
                policy: ui::RendererPolicy::Inherited,
            },
        );
        saved_test_resume(&f.m, &owner, Some(owner.clone()));
        let d = r::operation_dir(&f.m, &owner).unwrap();
        let spec = json!({"operation":owner,"application_identity":application,"renderer_policy":"inherited","report":d.join("result.json")});
        assert_eq!(
            r::submit_with(
                &f.m,
                &spec,
                |_| Ok(false),
                || Ok(UnitState {
                    exists: true,
                    empty: false,
                    observation: None
                })
            )
            .unwrap(),
            r::Submission::SubmittedOrLive
        );
        atomic_json(
            &d.join("result.json"),
            &json!({"schema":1,"operation":owner,"state":"running","renderer":{"cause":"gpu"}}),
        )
        .unwrap();
        let mut probes = 0;
        let mut stops = 0;
        let mut restores = 0;
        stop_renderer_with(
            &f.m,
            &owner,
            |id| {
                r::stop_with(
                    &f.m,
                    id,
                    || {
                        stops += 1;
                        Ok(())
                    },
                    || {
                        probes += 1;
                        Ok(UnitState {
                            exists: true,
                            empty: probes > 1,
                            observation: None,
                        })
                    },
                )
            },
            |_| {
                restores += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!((stops, restores), (1, 1));
        assert_eq!(r::result(&f.m, &owner).unwrap()["renderer"]["cause"], "gpu");
    }
    #[test]
    fn dependency_stop_preserves_uncertain_resume_and_restores_exactly_once() {
        use dependency_session::{self as r, UnitState};
        let f = test_fixture::Fixture::new();
        let application = "ab".repeat(32);
        let owner = queued_test_action(
            &f.m,
            ui::Action::DependencyPrepare {},
        );
        saved_test_resume(&f.m, &owner, Some(owner.clone()));
        let spec = json!({"operation":owner,"application_identity":application,"renderer_policy":"inherited","report":r::operation_dir(&f.m,&owner).unwrap().join("result.json")});
        assert_eq!(
            r::submit_with(
                &f.m,
                &spec,
                |_| Err("lost acknowledgment".into()),
                || Err("unit unavailable".into())
            )
            .unwrap(),
            r::Submission::AcknowledgmentUncertain
        );
        let path = f.m.root.join("operator/resume.json");
        let retained = fs::read(&path).unwrap();
        assert!(stop_dependency_with(
            &f.m,
            &"ff".repeat(16),
            |_| panic!("wrong stop"),
            |_| panic!("wrong restore")
        )
        .is_err());
        assert!(stop_dependency_with(
            &f.m,
            &owner,
            |id| r::stop_with(&f.m, id, || Ok(()), || Err("unknown".into())),
            |_| panic!("uncertain service must remain stopped")
        )
        .is_err());
        assert_eq!(fs::read(&path).unwrap(), retained);
        let mut restores = 0;
        stop_dependency_with(
            &f.m,
            &owner,
            |id| {
                r::stop_with(
                    &f.m,
                    id,
                    || panic!("absent unit needs no signal"),
                    || {
                        Ok(UnitState {
                            exists: false,
                            empty: true,
                            observation: None,
                        })
                    },
                )
            },
            |saved| {
                assert_eq!(saved.owner_operation, owner);
                restores += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(restores, 1);
        assert!(!path.exists());
        resume_owned_with(&f.m, &owner, |_| {
            restores += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(restores, 1);
        assert!(r::terminal(&r::result(&f.m, &owner).unwrap(), &owner));
    }
    #[test]
    fn dependency_lost_ack_live_result_then_exact_stop_preserves_failure_and_resume_owner() {
        use dependency_session::{self as r, UnitState};
        let f = test_fixture::Fixture::new();
        let application = "ab".repeat(32);
        let owner = queued_test_action(
            &f.m,
            ui::Action::DependencyPrepare {},
        );
        saved_test_resume(&f.m, &owner, Some(owner.clone()));
        let d = r::operation_dir(&f.m, &owner).unwrap();
        let spec = json!({"operation":owner,"application_identity":application,"renderer_policy":"inherited","report":d.join("result.json")});
        assert_eq!(
            r::submit_with(
                &f.m,
                &spec,
                |_| Ok(false),
                || Ok(UnitState {
                    exists: true,
                    empty: false,
                    observation: None
                })
            )
            .unwrap(),
            r::Submission::SubmittedOrLive
        );
        atomic_json(
            &d.join("result.json"),
            &json!({"schema":1,"operation":owner,"state":"running","dependency":{"cause":"not_ready"}}),
        )
        .unwrap();
        let mut probes = 0;
        let mut stops = 0;
        let mut restores = 0;
        stop_dependency_with(
            &f.m,
            &owner,
            |id| {
                r::stop_with(
                    &f.m,
                    id,
                    || {
                        stops += 1;
                        Ok(())
                    },
                    || {
                        probes += 1;
                        Ok(UnitState {
                            exists: true,
                            empty: probes > 1,
                            observation: None,
                        })
                    },
                )
            },
            |_| {
                restores += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!((stops, restores), (1, 1));
        assert_eq!(r::result(&f.m, &owner).unwrap()["dependency"]["cause"], "not_ready");
    }
    #[test]
    fn stop_asc_recovers_only_the_exact_vendor_open_operation() {
        for mismatch in [
            "none",
            "wrong_action",
            "wrong_launch",
            "missing_launch_binding",
            "changed_vendor_result",
        ] {
            let f = test_fixture::Fixture::new();
            let a = if mismatch == "wrong_action" {
                ui::Action::EnvironmentRescan {
                    environment: f.r.environment.id.clone(),
                }
            } else {
                ui::Action::VendorApplicationOpen {
                    application: ASC.into(),
                }
            };
            let owner = queued_test_action(&f.m, a);
            let stop = queued_test_action(
                &f.m,
                ui::Action::VendorApplicationStop {
                    application: ASC.into(),
                },
            );
            let vendor = "cd".repeat(16);
            saved_test_resume(
                &f.m,
                &owner,
                if mismatch == "missing_launch_binding" {
                    None
                } else {
                    Some(vendor.clone())
                },
            );
            private_dir(&app_directory(&f.m)).unwrap();
            atomic_json(&app_directory(&f.m).join("operation.json"),
                &json!({"operation_id":if mismatch=="wrong_launch"{"ee".repeat(16)}else{vendor.clone()}})).unwrap();
            atomic_json(&app_directory(&f.m).join("operation-result.json"),
                &json!({"schema":3,"operation_id":if mismatch=="changed_vendor_result"{"ef".repeat(16)}else{vendor.clone()},
                "state":"running","launcher_exit":null,"discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
            let path = f.m.root.join("operator/resume.json");
            let before = fs::read(&path).unwrap();
            let mut cancels = 0;
            let mut restores = 0;
            let result = stop_vendor_with(
                &f.m,
                ASC,
                |id| {
                    assert_eq!(id, vendor);
                    if mismatch == "changed_vendor_result" {
                        // Actual cancellation owner rejects drift before systemctl,
                        // application verification or any vendor/process action.
                        return vendor_cli::cancel_owned(&f.m, ASC, id);
                    }
                    cancels += 1;
                    Ok(())
                },
                |saved| {
                    assert_eq!(saved.owner_operation, owner);
                    assert_ne!(saved.owner_operation, stop);
                    restores += 1;
                    Ok(())
                },
            );
            if mismatch == "none" {
                result.unwrap();
                assert_eq!(cancels, 1);
                assert_eq!(restores, 1);
                assert!(!path.exists());
            } else {
                let reason = match mismatch {
                    "wrong_action" => "operator_vendor_open_owner_mismatch",
                    "wrong_launch" => "operator_vendor_open_binding_changed",
                    "missing_launch_binding" => "operator_vendor_open_binding_absent",
                    "changed_vendor_result" => "vendor_application_operation_changed",
                    _ => unreachable!(),
                };
                assert_eq!(result.unwrap_err().to_string(), reason);
                assert_eq!(cancels, 0);
                assert_eq!(restores, 0);
                assert_eq!(fs::read(&path).unwrap(), before);
            }
        }
    }
    #[test]
    fn ordinary_history_limits_rollback_to_ancestry_and_restore_preserves_bytes() {
        let (f, mut p, c, n) = test_fixture::prepared();
        let r = observation::derive(&p, &c, &n).unwrap();
        let key = r.key();
        let first =
            f.m.managed_publish(&p, &c, r.clone(), &c.host, &c.host_source_sha256, None)
                .unwrap();
        p.revision += 1;
        let abandoned =
            f.m.managed_publish(&p, &c, r.clone(), &c.host, &c.host_source_sha256, None)
                .unwrap();
        f.m.rollback(&key, &first.id, None).unwrap();
        p.revision += 1;
        let recommended =
            f.m.managed_publish(&p, &c, r.clone(), &c.host, &c.host_source_sha256, None)
                .unwrap();
        let saved = fs::read(
            f.m.root
                .join("publications")
                .join(&key)
                .join("revisions")
                .join(&recommended.id)
                .join("revision.json"),
        )
        .unwrap();
        let history = history(
            &f.m,
            &key,
            f.m.registry().unwrap().classes.get(&key).unwrap(),
        )
        .unwrap();
        assert!(
            history
                .iter()
                .find(|h| h.publication == first.id)
                .unwrap()
                .rollback_allowed
        );
        assert!(
            !history
                .iter()
                .find(|h| h.publication == abandoned.id)
                .unwrap()
                .rollback_allowed
        );
        let server = service_reply(&f.m, capacity_json(false, 0, 0));
        execute(
            &f.m,
            &ui::Action::OrdinaryRollback {
                class_id: key.clone(),
                publication: first.id.clone(),
            },
        )
        .unwrap();
        server.join().unwrap();
        assert_eq!(
            f.m.registry().unwrap().classes[&key].managed_revision,
            Some(first)
        );
        let restored =
            f.m.managed_publish(&p, &c, r, &c.host, &c.host_source_sha256, None)
                .unwrap();
        assert_eq!(f.m.load_revision(&key, &restored).unwrap().profile, p);
        assert_eq!(
            fs::read(
                f.m.root
                    .join("publications")
                    .join(&key)
                    .join("revisions")
                    .join(&recommended.id)
                    .join("revision.json")
            )
            .unwrap(),
            saved
        );
        p.claim = profiles::Claim::ReviewCandidate;
        assert!(f
            .m
            .managed_publish(&p, &c, f.r.clone(), &c.host, &c.host_source_sha256, None)
            .is_err());
    }
    #[test]
    fn vendor_deactivation_is_live_until_the_unit_finishes_cleanup() {
        let sequence = ["activating", "active", "deactivating", "inactive"];
        assert_eq!(
            sequence.map(|s| vendor_state_live(s).unwrap()),
            [true, true, true, false]
        );
        assert!(!vendor_state_live("failed").unwrap());
        assert!(vendor_state_live("unknown").is_err());
        assert!(vendor_state_live("").is_err());
    }
    #[test]
    fn failed_capacity_readback_does_not_guess_idle_ownership() {
        let f = test_fixture::Fixture::new();
        let _mutation = f.m.lock("registry.lock").unwrap();
        let read = activity(&f.m).unwrap();
        assert!(!read.system.capacity_available());
        assert!(read.system.cleanup_unconfirmed);
        assert!(read.operation.is_none());
    }
    #[test]
    fn ui2_problem_capture_is_admitted_during_cleanup_uncertainty_but_finish_is_not() {
        let f = test_fixture::Fixture::new();
        let problem = ui::Action::CompatibilityResult {
            candidate: "ab".repeat(32),
            expected_current: ui::PublicationIdentity {
                id: "cd".repeat(16), sha256: "ef".repeat(32),
            },
            result: ui::TestResultKind::Problem {
                category: ui::ProblemCategory::CleanupIncomplete,
            },
            passed: vec![], failed_area: None, note: "Cleanup did not complete".into(),
        };
        let finish = ui::Action::CompatibilityFinishResult { operation: "12".repeat(16) };
        let capacity = || serde_json::from_value(json!({
            "schema":1,"dsp":0,"maintenance":0,"keepers":0,
            "cleanup_unconfirmed":true,"owners":[],"limits":{"global_dsp":6}
        })).ok();
        assert!(!problem.requires_inactive());
        assert!(require_operator_inactive_with(&f.m, &problem, &capacity,
            None, Duration::from_millis(100), &mut vec![]).is_ok());
        assert!(require_operator_inactive_with(&f.m, &problem, &|| None,
            None, Duration::from_millis(100), &mut vec![]).is_ok());
        assert!(finish.requires_inactive());
        assert!(require_operator_inactive_with(&f.m, &finish, &capacity,
            None, Duration::from_millis(100), &mut vec![]).is_err());
        assert!(require_operator_inactive_with(&f.m, &finish, &|| None,
            None, Duration::from_millis(100), &mut vec![]).is_err());
    }
    fn onboarding_worker_fixture() -> (test_fixture::Fixture, String) {
        use linux_vst_bridge::catalogue::{Catalogue, EnvironmentBinding, OnboardingRuntimePolicy,
            STANDARD_ONBOARDING_RUNNER};
        let (f, p, _, native) = test_fixture::prepared();
        let mut standard = f.r.environment.clone();
        standard.id = "22".repeat(16);
        standard.root = f.m.root.join("environments").join(&standard.id);
        standard.runner.id = STANDARD_ONBOARDING_RUNNER.into();
        let environments = vec![EnvironmentBinding {
            family: p.requirements.environment_family.clone(),
            environment: f.r.environment.clone(),
        }, EnvironmentBinding {
            family: p.requirements.environment_family,
            environment: standard.clone(),
        }];
        let catalogue = Catalogue {
            schema: 4,
            natives: vec![native],
            environments: environments.clone(),
            hosts: vec![],
            onboarding_runtime: OnboardingRuntimePolicy::from_environments(&environments).unwrap(),
        };
        let path = f.m.root.join("software/catalogue.json");
        atomic_json(&path, &catalogue).unwrap();
        let source = Artifact {
            path: f.r.host.path.with_file_name("host-source-manifest.json"),
            sha256: f.r.host_source_sha256.clone(),
        };
        let a = f.r.host.clone();
        let sw = Software {
            installer_launch: None,
            preparation_kit: None,
            manager: a.clone(),
            operator_frontend: None,
            supervisor: a.clone(),
            ownership: a.clone(),
            host: a,
            source_manifest: source,
            source_sha256: f.r.host_source_sha256.clone(),
            native_catalogue: Some(Artifact {
                sha256: digest(&path).unwrap(),
                path,
            }),
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let mut bytes = vec![0; 1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60] = 128;
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148] = 2;
        bytes[150] = 2;
        bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let source = f.outer.join("operator-file");
        fs::write(&source, bytes).unwrap();
        let installer = installer_import::import(&f.m, file(&source).unwrap()).unwrap();
        let request = ui::Request {
            schema: ui::OPERATOR_SCHEMA,
            state_token: token(&f.m).unwrap(),
            action: ui::Action::InstallerEnvironmentCreate {
                installer: installer.id,
                runner: onboarding::runner_key(&standard.runner).unwrap(),
            },
        };
        let receipt = launch_queued(&f.m, &request, |_| Ok(true)).unwrap();
        (f, receipt.operation.unwrap())
    }
    #[test]
    #[ignore = "opt-in snapshot latency measurement"]
    fn snapshot_readback_latency_sample() {
        let (f, _) = onboarding_worker_fixture();
        let sample = || {
            let start = Instant::now();
            std::hint::black_box(snapshot_for_operation(
                &f.m, None, OPERATOR_WAIT, &mut vec![], &|| capacity_fixture(&f.m),
            ).unwrap());
            start.elapsed().as_micros()
        };
        let cold = sample();
        let mut warm: Vec<_> = (0..25).map(|_| sample()).collect();
        warm.sort_unstable();
        eprintln!("snapshot_readback_us cold={cold} p50={} p95={} max={}",
            warm[12], warm[23], warm[24]);
    }
    #[test]
    fn ui1_default_offer_is_one_exact_standard_runner_and_missing_policy_refuses_setup() {
        let (f, _) = onboarding_worker_fixture();
        let rows = onboarding::projection(&f.m, None).unwrap();
        let setups = onboarding::setup_projection(&f.m, &rows, &[], &Default::default()).unwrap();
        assert_eq!(setups.len(), 1);
        assert_eq!(setups[0].phase, ui::SetupPhase::Imported);
        assert_eq!(setups[0].history.len(), 1);
        let offer = setups[0].primary.as_ref().unwrap();
        let ui::Action::InstallerEnvironmentCreate { runner, .. } = &offer.action else { panic!("expected exact setup offer") };
        let catalogue = software(&f.m).unwrap().catalogue(&f.m).unwrap();
        assert_eq!(runner, &catalogue.onboarding_runtime.as_ref().unwrap().default_runner_key);
        assert_eq!(offer.label, "Continue setup");
        assert_eq!(rows[0].actions.len(), 1);
        let mut changed = catalogue.clone();
        changed.environments[1].environment.runner.id = "specialized-runner".into();
        let mut sw = software(&f.m).unwrap();
        let path = sw.native_catalogue.as_ref().unwrap().path.clone();
        atomic_json(&path, &changed).unwrap();
        sw.native_catalogue.as_mut().unwrap().sha256 = digest(&path).unwrap();
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        assert!(software(&f.m).unwrap().catalogue(&f.m).is_err());
        let mut unsupported = catalogue;
        unsupported.schema = 3;
        unsupported.onboarding_runtime = None;
        atomic_json(&path, &unsupported).unwrap();
        sw.native_catalogue.as_mut().unwrap().sha256 = digest(&path).unwrap();
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let rows = onboarding::projection(&f.m, None).unwrap();
        assert!(rows[0].actions.is_empty());
        let setups = onboarding::setup_projection(&f.m, &rows, &[], &Default::default()).unwrap();
        assert!(setups[0].primary.is_none());
    }
    #[test]
    fn current_setup_offer_receives_durable_ack_without_diagnostics_snapshot() {
        let (fixture, _) = onboarding_worker_fixture();
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        let current = overview(&fixture.m).unwrap();
        service.join().unwrap();
        let offer = current.current.installer_setups[0].primary.as_ref().unwrap();
        assert!(matches!(offer.action,ui::Action::InstallerEnvironmentCreate {..}));
        assert!(offer.disabled_reason.is_none());
        let request = ui::Request {schema:ui::OPERATOR_SCHEMA,
            state_token:current.current.state_token,action:offer.action.clone()};
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        let started = Instant::now();
        let receipt = dispatch_recorded(&fixture.m,&request,|id| {
            validate_current_request(&fixture.m,&request)?;
            launch_reserved(&fixture.m,&request,id,|_|Ok(true))
        }).unwrap();
        let elapsed = started.elapsed();
        service.join().unwrap();
        eprintln!("PB0_SOURCE_SETUP_ACK_MS {}",elapsed.as_millis());
        assert!(receipt.accepted && receipt.operation.is_some());
        assert!(elapsed < Duration::from_secs(2),"current Setup acknowledgement took {elapsed:?}");
        assert_eq!(optional(&fixture.m.root.join("operator/latest.json")).unwrap()["state"],"queued");
    }
    #[test]
    fn product_environment_accepts_retained_uuid_and_hex_identities() {
        assert!(valid_product_environment(
            "1de8d28b-d98b-4a41-8462-c07cca8e1609"));
        assert!(valid_product_environment(
            "7d8fce354e68595cdc20485f754a892d"));
        assert!(!valid_product_environment("1de8d28b/../record.json"));
        assert!(!valid_product_environment(
            "1de8d28b-d98b-4a41-8462-c07cca8e160z"));
    }
    #[test]
    fn current_product_offer_receives_durable_ack_without_diagnostics_snapshot() {
        let (fixture, candidate) = preparation_cli::tests::projection_fixture();
        atomic_json(&fixture.m.root.join("software.json"),
            &preparation_cli::tests::projection_software(&candidate)).unwrap();
        let selection = &candidate.selection;
        let mut installer_bytes=vec![0;1024];
        installer_bytes[..2].copy_from_slice(b"MZ");
        installer_bytes[60]=128;
        installer_bytes[128..132].copy_from_slice(b"PE\0\0");
        installer_bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        installer_bytes[148]=2;
        installer_bytes[150]=2;
        installer_bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let installer_path=fixture.outer.join("product-installer.exe");
        fs::write(&installer_path,installer_bytes).unwrap();
        let installer=installer_import::import(&fixture.m,file(&installer_path).unwrap()).unwrap();
        let directory=onboarding::directory(&fixture.m,&selection.environment.id).unwrap();
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("record.json"),&onboarding::Record {schema:1,
            id:selection.environment.id.clone(),installer:installer.id,
            environment:selection.environment.clone(),created_at:1,
            creation_operation:"cd".repeat(16),installation_operation:None,
            published:false,previous_attempt:None}).unwrap();
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        let detail = product_detail(&fixture.m, &selection.environment.id,
            &selection.module.sha256, &selection.class.id).unwrap();
        service.join().unwrap();
        assert_eq!(detail.schema, 1);
        assert_eq!(detail.operator_schema, ui::OPERATOR_SCHEMA);
        assert_eq!(detail.product.environment, selection.environment.id);
        assert!(detail.product.compatibility.is_some());
        let offered = detail.product.actions.iter().find(|offer|
            matches!(offer.action, ui::Action::PluginReinspect { .. })).unwrap();
        assert!(offered.disabled_reason.is_none());
        let mut wrong = offered.action.clone();
        if let ui::Action::PluginReinspect { audio_layout, .. } = &mut wrong {
            *audio_layout = Some(profiles::AudioLayoutPolicy::StereoMainPair);
        }
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        let refused = validate_current_request(&fixture.m,&ui::Request {
            schema:ui::OPERATOR_SCHEMA,state_token:detail.state_token.clone(),action:wrong,
        }).unwrap_err();
        service.join().unwrap();
        assert_eq!(refused.to_string(),"operator_action_not_available");
        let mut request = ui::Request {schema:ui::OPERATOR_SCHEMA,
            state_token:detail.state_token,action:offered.action.clone()};
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        let started = Instant::now();
        let receipt = dispatch_recorded(&fixture.m,&request,|id| {
            validate_current_request(&fixture.m,&request)?;
            launch_reserved(&fixture.m,&request,id,|_|Ok(true))
        }).unwrap();
        let elapsed = started.elapsed();
        service.join().unwrap();
        assert!(receipt.accepted && receipt.operation.is_some());
        assert!(elapsed < Duration::from_secs(2),
            "current product acknowledgement took {elapsed:?}");
        assert_eq!(optional(&fixture.m.root.join("operator/latest.json")).unwrap()["state"],"queued");
        request.state_token = "stale".into();
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        assert_eq!(validate_current_request(&fixture.m,&request).unwrap_err().to_string(),
            "operator_stale_request_refresh");
        service.join().unwrap();
        request.schema = 11;
        assert_eq!(validate_current_request(&fixture.m,&request).unwrap_err().to_string(),
            "operator_schema_mismatch_update_manager_frontend");
    }
    #[test]
    fn scoped_ordinary_product_keeps_exact_rollback_offer() {
        let (fixture, mut profile, census, native) = test_fixture::prepared();
        let registration = observation::derive(&profile, &census, &native).unwrap();
        let class = registration.key();
        let first = fixture.m.managed_publish(&profile, &census, registration.clone(),
            &census.host, &census.host_source_sha256, None).unwrap();
        profile.revision += 1;
        fixture.m.managed_publish(&profile, &census, registration.clone(),
            &census.host, &census.host_source_sha256, None).unwrap();
        let catalogue = linux_vst_bridge::catalogue::Catalogue {schema:3,natives:vec![native],
            environments:vec![census.environment],hosts:vec![],onboarding_runtime:None};
        let catalogue_path = fixture.m.root.join("software/native-catalogue.json");
        atomic_json(&catalogue_path,&catalogue).unwrap();
        let source_path = fixture.r.host.path.with_file_name("host-source-manifest.json");
        let software = Software {installer_launch:None,preparation_kit:None,
            operator_frontend:None,manager:fixture.r.host.clone(),
            supervisor:fixture.r.host.clone(),ownership:fixture.r.host.clone(),
            host:fixture.r.host.clone(),source_manifest:Artifact {
                path:source_path,sha256:fixture.r.host_source_sha256.clone()},
            source_sha256:fixture.r.host_source_sha256.clone(),
            native_catalogue:Some(Artifact {sha256:digest(&catalogue_path).unwrap(),
                path:catalogue_path})};
        atomic_json(&fixture.m.root.join("software.json"),&software).unwrap();
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        let detail = product_detail(&fixture.m, &registration.environment.id,
            &registration.module.sha256, &class).unwrap();
        service.join().unwrap();
        let rollback = detail.product.actions.iter().find(|offer|
            matches!(&offer.action, ui::Action::OrdinaryRollback { publication, .. }
                if publication == &first.id)).unwrap();
        assert!(rollback.disabled_reason.is_none());
        assert!(detail.product.history.iter().any(|entry|
            entry.publication == first.id && entry.rollback_allowed));
        assert_eq!(detail.environments.len(),1);
        let request = ui::Request {schema:ui::OPERATOR_SCHEMA,
            state_token:detail.state_token,action:rollback.action.clone()};
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        validate_current_request(&fixture.m,&request).unwrap();
        service.join().unwrap();
        let mut wrong = request;
        if let ui::Action::OrdinaryRollback {publication,..} = &mut wrong.action {
            *publication = "ff".repeat(16);
        }
        let service = overview_service_reply(&fixture.m, capacity_json(false,0,0));
        assert_eq!(validate_current_request(&fixture.m,&wrong).unwrap_err().to_string(),
            "operator_action_not_available");
        service.join().unwrap();
    }
    #[test]
    fn ui1_rename_requires_exact_offered_installer_and_changes_no_custody_record() {
        let (f, _) = onboarding_worker_fixture();
        let rows = onboarding::projection(&f.m, None).unwrap();
        let setups = onboarding::setup_projection(&f.m, &rows, &[], &Default::default()).unwrap();
        let sha = setups[0].installer.clone();
        let record_path = f.m.root.join("installers").join(format!("{sha}.json"));
        let original = fs::read(&record_path).unwrap();
        let mut view = view("current", action("Refresh", ui::Action::CaptureDisarm {}, None));
        view.installer_setups = setups;
        let mut request = ui::Request { schema: ui::OPERATOR_SCHEMA, state_token: "current".into(),
            action: ui::Action::InstallerRename { installer: sha.clone(), label: "Lunacy Audio".into() } };
        assert!(validate(&request, &view).is_ok());
        request.action = ui::Action::InstallerRename { installer: "ff".repeat(32), label: "Other".into() };
        assert!(validate(&request, &view).is_err());
        request.action = ui::Action::InstallerRename { installer: sha.clone(), label: "invalid/label".into() };
        assert!(validate(&request, &view).is_err());
        installer_import::rename(&f.m, &sha, "Lunacy Audio").unwrap();
        assert_eq!(fs::read(&record_path).unwrap(), original);
        assert_eq!(installer_import::presentation(&f.m, &installer_import::load(&f.m, &sha).unwrap())
            .unwrap().display_label, "Lunacy Audio");
    }
    #[test]
    fn ui1_workspace_application_installer_is_not_a_plugin_setup() {
        let (f, _) = onboarding_worker_fixture();
        let rows = onboarding::projection(&f.m, None).unwrap();
        let installer = rows[0].installer.clone();
        let workspace_owned = std::collections::BTreeSet::from([installer.clone()]);
        assert!(onboarding::setup_projection(&f.m, &rows, &[], &workspace_owned)
            .unwrap().is_empty());
        assert_eq!(onboarding::setup_projection(&f.m, &rows, &[], &Default::default())
            .unwrap()[0].installer, installer);
    }
    #[test]
    fn ui1_one_card_tracks_setup_phases_and_exact_discovered_product() {
        let (f, _) = onboarding_worker_fixture();
        let imported = onboarding::projection(&f.m, None).unwrap().remove(0);
        let environment = f.r.environment.id.clone();
        let mut current = imported.clone();
        current.environment = Some(environment.clone());
        current.state = "environment_ready".into();
        current.actions = vec![action("Run installer", ui::Action::InstallerStart {
            onboarding: environment.clone() }, None)];
        let mut rows = vec![imported, current.clone()];
        let project = |rows: &[ui::Onboarding], products: &[ui::Product]| {
            let setups = onboarding::setup_projection(&f.m, rows, products, &Default::default()).unwrap();
            assert_eq!(setups.len(), 1);
            assert_eq!(setups[0].history.len(), 2);
            setups.into_iter().next().unwrap()
        };
        let ready = project(&rows, &[]);
        assert_eq!(ready.phase, ui::SetupPhase::EnvironmentReady);
        assert!(matches!(ready.primary.unwrap().action, ui::Action::InstallerStart { .. }));
        current.state = "running".into();
        current.actions = vec![action("Focus installer", ui::Action::InstallerFocus {
            onboarding: environment.clone(), operation: "aa".repeat(16) }, None),
            action("Stop installer", ui::Action::InstallerStop {
                onboarding: environment.clone(), operation: "aa".repeat(16) }, None)];
        rows[1] = current.clone();
        let running = project(&rows, &[]);
        assert_eq!(running.phase, ui::SetupPhase::InstallerRunning);
        assert_eq!(running.secondary.len(), 1);
        current.state = "completed".into();
        current.details = json!({"installation":{"state":"completed",
            "transaction":{"schema":1,"outcome":"installed","durable_installation":"installed"}}});
        current.actions = vec![action("Scan installed products", ui::Action::InstallerScan {
            onboarding: environment.clone() }, None)];
        rows[1] = current.clone();
        let retired = project(&rows, &[]);
        assert_eq!(retired.phase, ui::SetupPhase::InstallerRetired);
        assert_eq!(retired.primary.unwrap().label, "Find installed plug-ins");
        let class = inventory::Class { id: f.r.metadata.class_id.clone(), name: "BEAM".into(),
            vendor: "Lunacy Audio".into(), version: "2.3.1".into(), category: "Audio Module Class".into(),
            subcategories: "Fx".into(), role: "effect".into() };
        let scan = inventory::Scan { schema: 1, id: "bb".repeat(16),
            environment: f.r.environment.clone(), host: f.r.host.clone(),
            host_source_sha256: f.r.host_source_sha256.clone(), completed_at: 1,
            modules: vec![inventory::Module { artifact: f.r.module.clone(),
                classes: vec![class], report: f.r.host.clone(),
                inspection_error: None, quarantine_reason: None }], changes: Default::default() };
        current.state = "installed_unqualified".into();
        current.details["scan"] = json!(scan.clone());
        current.details["previous_attempt"] = Value::Null;
        current.actions.clear();
        rows[1] = current.clone();
        let product = ui::Product { class_id: f.r.metadata.class_id.clone(), name: "BEAM".into(),
            vendor: "Lunacy Audio".into(), role: "effect".into(), version: "2.3.1".into(),
            disposition: "installed_unqualified".into(), active_revision: None, recommended_revision: None,
            environment: environment.clone(), runner: f.r.environment.runner.id.clone(),
            module_sha256: f.r.module.sha256.clone(), limitations: vec![], history: vec![],
            actions: vec![], compatibility: None, details: Value::Null };
        let found = project(&rows, std::slice::from_ref(&product));
        assert_eq!(found.phase, ui::SetupPhase::DiscoveryComplete);
        assert_eq!(found.discovered.len(), 1);
        assert_eq!(found.discovered[0].module_sha256, f.r.module.sha256);
        assert!(found.status.contains("discovery did not publish"));
        assert!(!found.status.contains("not yet available in Bitwig"));
        assert!(found.primary.is_none());
        current.details["installation"] = json!({"state":"failed",
            "transaction":{"schema":1,"outcome":"partial_installation",
                "durable_installation":"partial_installation"}});
        rows[1] = current.clone();
        let found_after_partial = project(&rows, std::slice::from_ref(&product));
        assert_eq!(found_after_partial.phase, ui::SetupPhase::DiscoveryComplete);
        assert_eq!(found_after_partial.discovered[0].module_sha256, f.r.module.sha256);
        assert!(found_after_partial.primary.is_none());
        current.details["installation"] = json!({"state":"failed",
            "transaction":{"schema":1,"outcome":"installed_dependency_failed",
                "durable_installation":"installed"}});
        let mut stale_scan = scan.clone();
        stale_scan.host_source_sha256 = "ef".repeat(32);
        current.state = "needs_attention".into();
        current.details["scan"] = json!(stale_scan);
        current.actions = vec![action("Scan installed products", ui::Action::InstallerScan {
            onboarding: environment.clone() }, None)];
        rows[1] = current.clone();
        let stale = project(&rows, std::slice::from_ref(&product));
        assert_eq!(stale.phase, ui::SetupPhase::SetupNeedsAttention);
        assert_eq!(stale.discovered.len(), 1);
        assert!(matches!(stale.primary.unwrap().action, ui::Action::InstallerScan { .. }));
        current.actions.clear();
        rows[1] = current.clone();
        let managed_stale = project(&rows, std::slice::from_ref(&product));
        assert!(managed_stale.primary.is_none());
        assert_eq!(managed_stale.discovered.len(), 1);
        let mut quarantined_scan = scan;
        quarantined_scan.modules[0].classes.clear();
        quarantined_scan.modules[0].quarantine_reason = Some("inventory_factory_absent".into());
        current.state = "quarantined".into();
        current.details["scan"] = json!(quarantined_scan);
        rows[1] = current.clone();
        let mut quarantined_product = product;
        quarantined_product.class_id.clear();
        quarantined_product.disposition = "quarantined".into();
        let quarantined = project(&rows, std::slice::from_ref(&quarantined_product));
        assert_eq!(quarantined.phase, ui::SetupPhase::SetupNeedsAttention);
        assert!(quarantined.status.contains("quarantined"));
        assert_eq!(quarantined.discovered.len(), 1);
        assert!(quarantined.discovered[0].class_id.is_empty());
        assert_eq!(quarantined.discovered[0].module_sha256, f.r.module.sha256);
        current.state = "cleanup_unconfirmed".into();
        current.details = Value::Null;
        rows[1] = current;
        let blocked = project(&rows, &[]);
        assert_eq!(blocked.phase, ui::SetupPhase::CleanupUnconfirmed);
        assert!(blocked.primary.is_none());
    }
    fn capacity_fixture(m: &Manager) -> Option<CapacityReadback> {
        // Actual capacity owner and registry read, only the Unix socket is omitted.
        capacity::status(m, capacity::fixture_limits(), 0, false)
            .ok()
            .and_then(|v| serde_json::from_value(serde_json::to_value(v).unwrap()).ok())
    }
    fn contended_registry<T>(m: &Manager,
        run: impl FnOnce(std::sync::mpsc::Sender<()>) -> T) -> T {
        let held = m.lock("registry.lock").unwrap();
        assert!(matches!(m.try_lock("registry.lock").unwrap(),
            linux_vst_bridge::operator_lock::LockAttempt::Busy));
        let (entered, release) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                release.recv().unwrap();
                // Begin the hold window immediately before the intended
                // acquisition, after unrelated fixture work has finished.
                std::thread::sleep(Duration::from_millis(200));
                drop(held);
            });
            run(entered)
        })
    }
    fn idle_test_capacity() -> Option<CapacityReadback> {
        Some(serde_json::from_value(capacity_json(false, 0, 0)["capacity"].clone()).unwrap())
    }
    #[test]
    fn preparation_inspection_admission_waits_for_polling_registry() {
        let f = test_fixture::Fixture::new();
        let owner = "ab".repeat(16);
        let mut waits = vec![];
        let (job, _) = contended_registry(&f.m, |entered| {
            preparation_cli::admitted_inspection_spec(&f.m, f.r.clone().into(), || {
                entered.send(()).unwrap();
                acquire_readback(
                    &f.m,
                    ui::OperatorLock::Registry,
                    Some(&owner),
                    Duration::from_secs(2),
                    &mut waits,
                )
            })
        })
        .unwrap();
        assert!(job.inspect && !job.first_audio);
        assert_eq!(waits.len(), 1);
        assert_eq!(waits[0].name, ui::OperatorLock::Registry);
        assert_eq!(waits[0].purpose, ui::LockPurpose::OperatorValidationReadback);
        assert_eq!(waits[0].operation.as_deref(), Some(owner.as_str()));
        assert!(waits[0].attempts > 1 && waits[0].outcome == ui::LockOutcome::Acquired);
        assert!(matches!(
            f.m.try_lock("registry.lock").unwrap(),
            linux_vst_bridge::operator_lock::LockAttempt::Acquired(_)
        ));
    }
    #[test]
    fn renderer_preflight_and_suspension_wait_for_polling_then_restore_once() {
        let (f, _) = onboarding_worker_fixture();
        let action = ui::Action::RendererOpen {
            application: "ab".repeat(32), policy: renderer_application::RendererPolicy::SoftwareRendering,
        };
        let owner = queued_test_action(&f.m, action.clone());
        let mut waits = vec![];
        contended_registry(&f.m, |entered| {
            entered.send(()).unwrap();
            require_operator_inactive_with(
                &f.m, &action, &idle_test_capacity, Some(&owner), Duration::from_secs(2), &mut waits,
            )
        }).unwrap();
        assert!(!f.m.root.join("operator/resume.json").exists());
        let stops = std::cell::Cell::new(0);
        contended_registry(&f.m, |entered| {
            entered.send(()).unwrap();
            suspend_with(&f.m, &owner, Some(owner.clone()), Duration::from_secs(2), &mut waits,
                (|| true, || { stops.set(stops.get()+1); Ok(()) }))
        }).unwrap();
        assert_eq!(stops.get(), 1);
        let saved = resume_record(&f.m).unwrap().unwrap();
        assert_eq!(saved.owner_operation, owner);
        assert_eq!(saved.vendor_operation.as_deref(), Some(owner.as_str()));
        assert!(waits.iter().all(|w| w.name == ui::OperatorLock::Registry
            && w.outcome == ui::LockOutcome::Acquired));
        assert!(waits.iter().any(|w| w.attempts > 1));
        assert_eq!(waits.len(), 2);
        let restores = std::cell::Cell::new(0);
        for _ in 0..2 {
            resume_owned_with(&f.m, &owner, |_| { restores.set(restores.get()+1); Ok(()) }).unwrap();
        }
        assert_eq!(restores.get(), 1);
        assert!(!f.m.root.join("operator/resume.json").exists());
    }
    #[test]
    fn managed_rescan_revalidates_authority_under_final_registry_custody() {
        let (f, owner) = onboarding_worker_fixture();
        let sw = software(&f.m).unwrap();
        let environment = f.r.environment.id.clone();
        let stale_snapshot = snapshot_idle_test(&f.m).unwrap();
        let refresh: Vec<_> = available(&stale_snapshot)
            .into_iter()
            .filter(|available| matches!(available.action, ui::Action::EnvironmentRescan { .. }))
            .collect();
        assert_eq!(refresh.len(), 1);
        assert_eq!(
            refresh[0].action,
            ui::Action::EnvironmentRescan {
                environment: environment.clone()
            }
        );

        let inventory = f.m.root.join("inventory");
        private_dir(&inventory).unwrap();
        let current = inventory.join(format!("{environment}.json"));
        let history = inventory.join("history");
        atomic_json(&current, &inventory::Scan {
            schema: 1,
            id: "ef".repeat(16),
            environment: f.r.environment.clone(),
            host: sw.host.clone(),
            host_source_sha256: sw.source_sha256.clone(),
            completed_at: 1,
            changes: Default::default(),
            modules: vec![],
        }).unwrap();
        assert!(available(&snapshot_idle_test(&f.m).unwrap())
            .into_iter()
            .all(|available| !matches!(
                available.action,
                ui::Action::InstallerScan { .. } | ui::Action::EnvironmentRescan { .. }
            )));
        fs::remove_file(&current).unwrap();
        assert!(!history.exists());
        let publications = test_fixture::snapshot(&f.m.publications);

        let stops = std::cell::Cell::new(0);
        let mut waits = vec![];
        suspend_with(
            &f.m,
            &owner,
            None,
            Duration::from_secs(2),
            &mut waits,
            (
                || true,
                || {
                    stops.set(stops.get() + 1);
                    Ok(())
                },
            ),
        )
        .unwrap();
        assert_eq!(stops.get(), 1);

        // This is the competing exact registry update after initial action
        // validation and suspension, but before the scan owns registry.lock.
        let registry_path = f.m.root.join("registry.json");
        let mut changed = f.m.registry().unwrap();
        changed
            .classes
            .values_mut()
            .find(|entry| entry.registration.environment.id == environment)
            .unwrap()
            .registration
            .environment
            .revision += 1;
        atomic_json(&registry_path, &changed).unwrap();
        let changed_registry = fs::read(&registry_path).unwrap();

        SCAN_SPAWN_COUNT.with(|count| count.set(0));
        let error = rescan(&f.m, &environment).unwrap_err().to_string();
        assert_eq!(error, "operator_managed_environment_mismatch");
        assert_eq!(SCAN_SPAWN_COUNT.with(std::cell::Cell::get), 0);

        let restores = std::cell::Cell::new(0);
        resume_owned_with(&f.m, &owner, |saved| {
            assert!(saved.resume);
            restores.set(restores.get() + 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(restores.get(), 1);
        assert!(!f.m.root.join("operator/resume.json").exists());
        assert_eq!(fs::read(&registry_path).unwrap(), changed_registry);
        assert_eq!(test_fixture::snapshot(&f.m.publications), publications);
        assert!(!current.exists());
        assert!(!history.exists());
    }
    #[test]
    fn native_access_restoration_requires_idle_bridge_two_keepers_and_no_resume_owner() {
        let base=json!({"schema":7,"system":{"service":"active","keepers":2,"dsp":0,"maintenance":0,
            "ceiling":6,"pending_transactions":0,"stale_transports":0,"cleanup_unconfirmed":false},
            "capture":{"armed":false,"active_retention":0},"operation":null});
        let activity:ui::Activity=serde_json::from_value(base.clone()).unwrap();
        assert_eq!(native_access_restoration_value(&activity,false).unwrap()["pending_resume_owner"],false);
        assert!(native_access_restoration_value(&activity,true).is_err());
        for (field,value) in [("service",json!("inactive")),("keepers",json!(1)),("dsp",json!(1)),
            ("maintenance",json!(1)),("pending_transactions",json!(1)),("stale_transports",json!(1)),
            ("cleanup_unconfirmed",json!(true))] {
            let mut bad=base.clone();bad["system"][field]=value;
            assert!(native_access_restoration_value(&serde_json::from_value(bad).unwrap(),false).is_err());
        }
        for (field,value) in [("armed",json!(true)),("active_retention",json!(1))] {
            let mut bad=base.clone();bad["capture"][field]=value;
            assert!(native_access_restoration_value(&serde_json::from_value(bad).unwrap(),false).is_err());
        }
    }
    #[test]
    fn native_access_session_summary_distinguishes_graceful_exact_owned_and_failed_cleanup() {
        let recovery=json!({"bridge":"active","keepers":2,"dsp_leases":0,"maintenance_leases":0,
            "pending_transactions":0,"stale_transports":0,"pending_resume_owner":false,"capture":"off"});
        let graceful=json!({"state":"completed","dependency":{"dependency_cleanup_disposition":"graceful_service_retirement",
            "service_retirement_confirmed":true,"process_cleanup_confirmed":true,"forced_cleanup_used":false}});
        assert_eq!(native_access_session_summary(&graceful,&recovery).unwrap()["fresh_session_restartable"],true);
        let exact=json!({"state":"cancelled","dependency":{"dependency_cleanup_disposition":"exact_owned_session_cleanup",
            "service_retirement_confirmed":false,"process_cleanup_confirmed":true,"forced_cleanup_used":true,
            "service_stop_request_count":1,"stop_observation":{"classification":"NAD2_STOP_SUBMITTED_NO_TRANSITION"},
            "post_cleanup":{"cgroup_empty":true,"exact_daemon_generation_absent":true,"listener_mask":0,
                "candidate_count":0,"candidate_observation_unavailable":0,"application_image_unchanged":true,
                "daemon_image_unchanged":true}}});
        assert_eq!(native_access_session_summary(&exact,&recovery).unwrap()["dependency_cleanup_disposition"],"exact_owned_session_cleanup");
        let mut unavailable=exact.clone();unavailable["dependency"]["stop_observation"]["classification"]=json!("NAD2_STOP_OBSERVATION_UNAVAILABLE");
        assert!(native_access_session_summary(&unavailable,&recovery).is_err());
        let failed=json!({"state":"failed","dependency":{"dependency_cleanup_disposition":"cleanup_unconfirmed"}});
        assert_eq!(native_access_session_summary(&failed,&recovery).unwrap()["fresh_session_restartable"],false);
    }
    #[test]
    fn renderer_preflight_contention_timeout_never_stops_or_reserves() {
        let (f, owner) = onboarding_worker_fixture();
        let held = f.m.lock("registry.lock").unwrap();
        let action = ui::Action::RendererOpen {
            application: "ab".repeat(32), policy: renderer_application::RendererPolicy::SoftwareRendering,
        };
        let mut waits = vec![];
        let error = require_operator_inactive_with(&f.m, &action, &idle_test_capacity,
            Some(&owner), Duration::from_millis(20), &mut waits).unwrap_err();
        assert!(error.is::<linux_vst_bridge::operator_lock::AcquisitionFailure>());
        let error = suspend_with(&f.m, &owner, Some(owner.clone()), Duration::from_millis(20), &mut waits,
            (|| panic!("must not inspect service before lock"), || panic!("must not stop service"))).unwrap_err();
        assert!(error.is::<linux_vst_bridge::operator_lock::AcquisitionFailure>());
        assert_eq!(waits.len(), 2);
        assert!(waits.iter().all(|w| w.outcome == ui::LockOutcome::Timeout));
        assert!(!f.m.root.join("operator/resume.json").exists());
        assert!(!f.m.root.join("vendor-applications/native-access/current.json").exists());
        drop(held);
    }
    #[test]
    fn renderer_preflight_rechecks_durable_owners_after_contention() {
        let (f, owner) = onboarding_worker_fixture();
        let sid = "cd".repeat(16);
        let report = f.m.root.join("runtime/results").join(format!("windows-{sid}.json"));
        let spec = f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions").join(&sid).join("owner.json");
        private_dir(spec.parent().unwrap()).unwrap();
        atomic_json(&spec, &json!({"session":sid,"report":report,"keeper":false,
            "registration":{"metadata":{"class_id":f.r.key()}}})).unwrap();
        private_dir(&f.m.root.join("runtime/leases")).unwrap();
        atomic_json(&f.m.root.join("runtime/leases").join(format!("{sid}.json")), &report).unwrap();
        let action = ui::Action::RendererOpen {
            application: "ab".repeat(32), policy: renderer_application::RendererPolicy::SoftwareRendering,
        };
        let mut waits = vec![];
        let error = contended_registry(&f.m, |entered| {
            entered.send(()).unwrap();
            require_operator_inactive_with(&f.m, &action, &idle_test_capacity,
                Some(&owner), Duration::from_secs(2), &mut waits)
        }).unwrap_err();
        assert_eq!(error.to_string(), "active_device_lease");
        let error = contended_registry(&f.m, |entered| {
            entered.send(()).unwrap();
            suspend_with(&f.m, &owner, Some(owner.clone()), Duration::from_secs(2), &mut waits,
                (|| panic!("must not inspect service"), || panic!("must not stop service")))
        }).unwrap_err();
        assert_eq!(error.to_string(), "active_device_lease");
        assert!(!f.m.root.join("operator/resume.json").exists());
    }
    fn worker_receipt(m: &Manager, id: &str) -> Value {
        read_json(&job_dir(m, id).unwrap().join("result.json")).unwrap()
    }
    #[test]
    fn new_attempt_uses_production_creation_owner_and_preserves_parent() {
        let (f, first) = onboarding_worker_fixture();
        let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
        let request: ui::Request =
            read_json(&job_dir(&f.m, &first).unwrap().join("request.json")).unwrap();
        let v = execute_with_receipt_policy(
            &f.m,
            &request.action,
            Some(&first),
            &|| capacity_fixture(&f.m),
            OPERATOR_WAIT,
            &mut vec![],
        )
        .unwrap();
        let parent = v["onboarding"].as_str().unwrap();
        let old_op = "cd".repeat(16);
        let r = onboarding::reserve(&f.m, parent, &old_op).unwrap();
        let result_path = onboarding::directory(&f.m, parent)
            .unwrap()
            .join(format!("{old_op}-result.json"));
        atomic_json(&result_path,&json!({"schema":2,"operation":old_op,"state":"cancelled","cleanup_confirmed":true,"owned_live":0,"raw_exit":-15})).unwrap();
        let record_path = onboarding::directory(&f.m, parent)
            .unwrap()
            .join("record.json");
        let before = fs::read(&record_path).unwrap();
        let before_result = fs::read(&result_path).unwrap();
        let action = ui::Action::InstallerNewAttempt {
            previous: parent.into(),
            runner: onboarding::runner_key(&r.environment.runner).unwrap(),
        };
        let next = execute_with_receipt_policy(
            &f.m,
            &action,
            Some(&"ef".repeat(16)),
            &|| capacity_fixture(&f.m),
            OPERATOR_WAIT,
            &mut vec![],
        )
        .unwrap();
        assert_ne!(next["onboarding"], parent);
        assert_eq!(onboarding::records(&f.m).unwrap().len(), 2);
        assert_eq!(fs::read(&record_path).unwrap(), before);
        assert_eq!(fs::read(&result_path).unwrap(), before_result);
        assert!(execute_with_receipt_policy(
            &f.m,
            &action,
            Some(&"01".repeat(16)),
            &|| capacity_fixture(&f.m),
            OPERATOR_WAIT,
            &mut vec![]
        )
        .is_err());
        assert!(!f.m.root.join("operator/resume.json").exists());
        assert_eq!(
            fs::read(f.m.root.join("registry.json")).unwrap(),
            registry_before
        );
    }
    #[test]
    fn projection_recheck_refuses_registry_change_during_expensive_work() {
        let (f, _) = onboarding_worker_fixture();
        let m = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        installer_import::VERIFY_BARRIER.with(|h| {
            *h.borrow_mut() = Some(Box::new(move || {
                let _guard = m.lock("registry.lock").unwrap();
                let mut db = m.registry().unwrap();
                db.revision += 1;
                atomic_json(&m.root.join("registry.json"), &db).unwrap();
            }))
        });
        let result = snapshot_for_operation(&f.m, None, OPERATOR_WAIT, &mut vec![], &|| {
            capacity_fixture(&f.m)
        });
        assert_eq!(
            result.unwrap_err().to_string(),
            "operator_state_changed_refresh"
        );
    }
    #[test]
    fn exact_readback_lock_purposes_for_acquisition_and_timeout() {
        let f = test_fixture::Fixture::new();
        let id = "ab".repeat(16);
        for (name, op, purpose) in [
            (
                ui::OperatorLock::Canonical,
                None,
                ui::LockPurpose::ActionSerialization,
            ),
            (
                ui::OperatorLock::Canonical,
                Some(id.as_str()),
                ui::LockPurpose::ActionSerialization,
            ),
            (
                ui::OperatorLock::Registry,
                None,
                ui::LockPurpose::OperatorReadback,
            ),
            (
                ui::OperatorLock::Registry,
                Some(id.as_str()),
                ui::LockPurpose::OperatorValidationReadback,
            ),
        ] {
            let mut waits = vec![];
            let guard =
                acquire_readback(&f.m, name, op, Duration::from_millis(20), &mut waits).unwrap();
            assert_eq!(waits[0].name, name);
            assert_eq!(waits[0].purpose, purpose);
            assert_eq!(waits[0].outcome, ui::LockOutcome::Acquired);
            assert!(
                acquire_readback(&f.m, name, op, Duration::from_millis(20), &mut waits).is_err()
            );
            assert_eq!(waits[1].name, name);
            assert_eq!(waits[1].purpose, purpose);
            assert_eq!(waits[1].outcome, ui::LockOutcome::Timeout);
            assert!(waits[1].attempts > 1);
            drop(guard);
        }
    }
    #[test]
    fn creation_refuses_verified_file_replacement_before_mutation() {
        for target in ["installer", "runner", "catalogue"] {
            let (f, id) = onboarding_worker_fixture();
            let request: ui::Request =
                read_json(&job_dir(&f.m, &id).unwrap().join("request.json")).unwrap();
            let ui::Action::InstallerEnvironmentCreate { installer, .. } = &request.action else {
                unreachable!()
            };
            let path = match target {
                "installer" => {
                    installer_import::load_record(&f.m, installer)
                        .unwrap()
                        .artifact
                        .path
                }
                "runner" => f.r.environment.runner.proton.clone(),
                _ => software(&f.m).unwrap().native_catalogue.unwrap().path,
            };
            let e = execute_with_receipt_capacity(&f.m, &request.action, Some(&id), &|| {
                let cap = capacity_fixture(&f.m);
                let replacement = path.with_extension("replacement");
                fs::copy(&path, &replacement).unwrap();
                fs::rename(replacement, &path).unwrap();
                cap
            })
            .unwrap_err()
            .to_string();
            assert_eq!(e, "onboarding_verified_file_changed");
            assert!(!f.m.root.join("onboarding").exists());
            assert!(!f.m.root.join("operator/resume.json").exists());
        }
    }
    fn mutation_contention_case(timeout: bool) {
        let (f, id) = onboarding_worker_fixture();
        let m = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        let op = id.clone();
        let (sampled, arrival) = std::sync::mpsc::channel();
        let (locked, barrier) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let count = std::cell::Cell::new(0);
            worker_with_capacity(
                &m,
                &op,
                if timeout {
                    Duration::from_millis(50)
                } else {
                    Duration::from_secs(2)
                },
                &|| {
                    let cap = capacity_fixture(&m);
                    count.set(count.get() + 1);
                    // Validation completed. Sample the real capacity owner, then
                    // arrange independent registry contention before mutation.
                    if count.get() == 2 {
                        sampled.send(()).unwrap();
                        barrier.recv_timeout(Duration::from_secs(3)).unwrap();
                    }
                    cap
                },
            )
        });
        arrival.recv_timeout(Duration::from_secs(3)).unwrap();
        let guard = f.m.lock("registry.lock").unwrap();
        locked.send(()).unwrap();
        if !timeout {
            std::thread::sleep(Duration::from_millis(40));
            drop(guard);
            thread.join().unwrap().unwrap();
        } else {
            thread.join().unwrap().unwrap();
            drop(guard);
        }
        let receipt = worker_receipt(&f.m, &id);
        let admissions: Vec<_> = receipt["lock_waits"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["purpose"] == "environment_creation_admission")
            .collect();
        assert_eq!(
            admissions.len(),
            1,
            "one mutation acquisition, no reacquisition: {receipt}"
        );
        assert_eq!(admissions[0]["name"], "registry.lock");
        assert_eq!(admissions[0]["operation"], id);
        assert!(admissions[0]["attempts"].as_u64().unwrap() > 1);
        for wait in receipt["lock_waits"].as_array().unwrap() {
            if wait["name"] == "operator-canonical.lock" {
                assert_eq!(wait["purpose"], "action_serialization");
            } else if wait["purpose"] != "environment_creation_admission" {
                assert_eq!(wait["purpose"], "operator_validation_readback");
            }
        }
        if timeout {
            assert_eq!(receipt["state"], "refused");
            assert_eq!(admissions[0]["outcome"], "timeout");
            let failure: ui::OperationFailure =
                serde_json::from_value(receipt["failure"].clone()).unwrap();
            assert_eq!(
                failure.stage,
                ui::FailureStage::EnvironmentCreationAdmission
            );
            assert_eq!(failure.code, ui::FailureCode::RegistryLockTimeout);
            assert!(
                failure.retryable
                    && !failure.mutation_started
                    && !failure.environment_created
                    && !failure.installer_launched
            );
            assert_eq!(failure.lock.holder, ui::LockHolder::Unknown);
            assert!(!f.m.root.join("onboarding").exists());
            let snap = snapshot_for_operation(&f.m, None, OPERATOR_WAIT, &mut vec![], &|| {
                capacity_fixture(&f.m)
            })
            .unwrap();
            assert_eq!(snap.onboarding[0].failure.as_ref(), Some(&failure));
        } else {
            assert_eq!(receipt["state"], "completed", "{receipt}");
            assert_eq!(admissions[0]["outcome"], "acquired");
            let records = onboarding::records(&f.m).unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].creation_operation, id);
            assert!(records[0].installation_operation.is_none());
            assert!(!records[0].published);
        }
        assert!(!f.m.root.join("operator/resume.json").exists());
        worker_with_capacity(&f.m, &id, OPERATOR_WAIT, &|| capacity_fixture(&f.m)).unwrap();
        assert_eq!(
            worker_receipt(&f.m, &id),
            receipt,
            "same terminal operation cannot run twice"
        );
    }
    #[test]
    fn mutation_wait_continues_same_operation_under_one_guard() {
        mutation_contention_case(false);
    }
    #[test]
    fn mutation_timeout_is_structured_without_environment_or_launch() {
        mutation_contention_case(true);
    }
    #[test]
    fn preworker_refusal_is_durable_and_bound_to_installer_without_mutation() {
        let (f, old) = onboarding_worker_fixture();
        let request: ui::Request =
            read_json(&job_dir(&f.m, &old).unwrap().join("request.json")).unwrap();
        let _busy = f.m.lock("operator-dispatch.lock").unwrap();
        let reply = dispatch(&f.m, request).unwrap();
        assert!(!reply.accepted);
        let id = reply.operation.unwrap();
        assert_ne!(id, old);
        let result = worker_receipt(&f.m, &id);
        assert_eq!(result["state"], "refused");
        assert_eq!(result["stage"], "operator_request_admission");
        assert_eq!(result["worker_started"], false);
        assert_eq!(result["mutation_started"], false);
        assert!(!f.m.root.join("onboarding").exists());
        assert!(!f.m.root.join("operator/resume.json").exists());
        write_operation(
            &f.m,
            &old,
            &json!({"schema":1,"operation":old,"state":"refused","reason":"late old error"}),
            false,
        )
        .unwrap();
        assert_eq!(
            optional(&f.m.root.join("operator/latest.json")).unwrap(),
            result
        );
        let snapshot = snapshot_for_operation(&f.m, None, OPERATOR_WAIT, &mut vec![], &|| {
            capacity_fixture(&f.m)
        })
        .unwrap();
        assert_eq!(
            snapshot.onboarding[0].details["request_result"]["operation"],
            id
        );
        assert_eq!(
            snapshot.onboarding[0].details["request_result"]["worker_started"],
            false
        );
    }
    #[test]
    fn admission_interruption_never_claims_a_queued_or_running_worker() {
        let (f, old) = onboarding_worker_fixture();
        let request: ui::Request =
            read_json(&job_dir(&f.m, &old).unwrap().join("request.json")).unwrap();
        let id = begin_request(&f.m, &request).unwrap();
        let result = worker_receipt(&f.m, &id);
        assert_eq!(result["state"], "unconfirmed");
        assert_eq!(result["worker_started"], false);
        assert!(!f.m.root.join("onboarding").exists());
        let newer = begin_request(&f.m, &request).unwrap();
        refuse_unfinished(&f.m, &id, "late admission failure").unwrap();
        assert_eq!(
            optional(&f.m.root.join("operator/latest.json")).unwrap()["operation"],
            newer
        );
    }
    #[test]
    fn expensive_installer_projection_leaves_registry_available() {
        let (f, _) = onboarding_worker_fixture();
        let m = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        let (entered, arrival) = std::sync::mpsc::channel();
        let (release, barrier) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            installer_import::VERIFY_BARRIER.with(|h| {
                *h.borrow_mut() = Some(Box::new(move || {
                    entered.send(()).unwrap();
                    barrier.recv_timeout(Duration::from_secs(3)).unwrap();
                }))
            });
            snapshot_for_operation(&m, None, Duration::from_secs(1), &mut vec![], &|| {
                capacity_fixture(&m)
            })
            .map_err(|e| e.to_string())
        });
        arrival.recv_timeout(Duration::from_secs(3)).unwrap();
        let acquired = f.m.lock("registry.lock");
        let available = acquired.is_ok();
        drop(acquired);
        release.send(()).unwrap();
        let result = thread.join().unwrap();
        assert!(
            available,
            "installer verification blocked registry authority"
        );
        result.unwrap();
    }
    #[test]
    fn terminated_worker_preserves_acquired_wait_context_and_first_terminal() {
        let (f, id) = onboarding_worker_fixture();
        let (_, facts) =
            f.m.lock_bounded(
                ui::OperatorLock::Registry,
                ui::LockPurpose::OperatorValidationReadback,
                Some(&id),
                Duration::from_millis(40),
            )
            .unwrap();
        let waits = json!([facts]);
        write_operation(
            &f.m,
            &id,
            &json!({"schema":1,"operation":id,"state":"running","lock_waits":waits}),
            false,
        )
        .unwrap();
        refuse_unfinished(&f.m, &id, "operator_worker_terminated").unwrap();
        let terminal = worker_receipt(&f.m, &id);
        assert_eq!(terminal["lock_waits"], waits);
        write_operation(
            &f.m,
            &id,
            &json!({"schema":1,"operation":id,"state":"completed"}),
            false,
        )
        .unwrap();
        assert_eq!(worker_receipt(&f.m, &id), terminal);
    }
    #[test]
    fn environment_worker_timeout_is_structured_retryable_and_never_mutates() {
        let (f, id) = onboarding_worker_fixture();
        let registry = f.m.lock("registry.lock").unwrap();
        // Exercise this worker's registry deadline independently of the
        // service's separately bounded capacity-readback wait.
        worker_with_capacity(&f.m, &id, Duration::from_millis(40), &|| None)
        .unwrap();
        let receipt = worker_receipt(&f.m, &id);
        assert_eq!(receipt["state"], "refused");
        let failure: ui::OperationFailure =
            serde_json::from_value(receipt["failure"].clone()).unwrap();
        assert_eq!(failure.code, ui::FailureCode::RegistryLockTimeout);
        assert!(
            failure.retryable
                && !failure.mutation_started
                && !failure.environment_created
                && !failure.installer_launched
        );
        assert_eq!(failure.lock.name, ui::OperatorLock::Registry);
        assert_eq!(
            failure.lock.purpose,
            ui::LockPurpose::OperatorValidationReadback
        );
        assert_eq!(failure.lock.operation.as_deref(), Some(id.as_str()));
        assert_eq!(failure.lock.holder, ui::LockHolder::Unknown);
        assert!(failure.lock.attempts >= 2);
        assert!(!f.m.root.join("onboarding").exists());
        assert!(!f.m.root.join("operator/resume.json").exists());
        drop(registry);
        // A duplicate worker/late ExecStopPost cannot rerun or overwrite terminal custody.
        worker_with_capacity(&f.m, &id, Duration::from_millis(40), &|| {
            capacity_fixture(&f.m)
        })
        .unwrap();
        refuse_unfinished(&f.m, &id, "late hook").unwrap();
        assert_eq!(worker_receipt(&f.m, &id), receipt);
        let snapshot = snapshot_for_operation(&f.m, None, OPERATOR_WAIT, &mut vec![], &|| {
            capacity_fixture(&f.m)
        })
        .unwrap();
        assert_eq!(snapshot.onboarding.len(), 1);
        assert_eq!(snapshot.onboarding[0].failure.as_ref(), Some(&failure));
        assert!(snapshot.onboarding[0]
            .actions
            .iter()
            .all(|a| a.disabled_reason.is_none()));
    }
    #[test]
    fn environment_worker_waits_then_creates_once_and_keeps_wait_receipt() {
        let (f, id) = onboarding_worker_fixture();
        let registry = f.m.lock("registry.lock").unwrap();
        let m = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        let op = id.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let first = std::cell::Cell::new(true);
            worker_with_capacity(&m, &op, Duration::from_secs(2), &|| {
                // An unavailable first read must still reach the worker's
                // registry wait and resample after contention clears.
                if first.replace(false) {
                    tx.send(()).unwrap();
                    None
                } else {
                    capacity_fixture(&m)
                }
            })
        });
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        std::thread::sleep(Duration::from_millis(30));
        drop(registry);
        thread.join().unwrap().unwrap();
        let receipt = worker_receipt(&f.m, &id);
        assert_eq!(receipt["state"], "completed", "{receipt}");
        assert!(receipt["lock_waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["name"] == "registry.lock"
                && w["attempts"].as_u64().unwrap() > 1
                && w["outcome"] == "acquired"));
        let records = onboarding::records(&f.m).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].creation_operation, id);
        assert!(records[0].installation_operation.is_none());
        assert!(!records[0].published);
        worker_with_capacity(&f.m, &id, OPERATOR_WAIT, &|| capacity_fixture(&f.m)).unwrap();
        assert_eq!(onboarding::records(&f.m).unwrap().len(), 1);
        assert_eq!(worker_receipt(&f.m, &id), receipt);
        assert!(!f.m.root.join("operator/resume.json").exists());
    }
    #[test]
    fn state_change_during_wait_refuses_original_request_without_mutation() {
        let (f, id) = onboarding_worker_fixture();
        let registry = f.m.lock("registry.lock").unwrap();
        let m = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        let op = id.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            worker_with_capacity(&m, &op, Duration::from_secs(2), &|| {
                let _ = tx.send(());
                capacity_fixture(&m)
            })
        });
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let mut db = f.m.registry().unwrap();
        db.revision += 1;
        atomic_json(&f.m.root.join("registry.json"), &db).unwrap();
        drop(registry);
        thread.join().unwrap().unwrap();
        let r = worker_receipt(&f.m, &id);
        assert_eq!(r["state"], "refused");
        assert_eq!(
            r["reason"],
            "Operator validation readback: operator_stale_request_refresh"
        );
        assert!(!f.m.root.join("onboarding").exists());
        assert!(!f.m.root.join("operator/resume.json").exists());
    }
    #[test]
    fn snapshot_capacity_and_creation_complete_without_reverse_wait_or_latest_theft() {
        let (f, id) = onboarding_worker_fixture();
        let registry = f.m.lock("registry.lock").unwrap();
        let a = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        let b = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        let op = id.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            worker_with_capacity(&a, &op, Duration::from_secs(2), &|| {
                let _ = tx.send(());
                capacity_fixture(&a)
            })
        });
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let snapshot = std::thread::spawn(move || {
            snapshot_for_operation(&b, None, Duration::from_secs(2), &mut vec![], &|| {
                capacity_fixture(&b)
            })
        });
        // Newer receipt is projection authority, regardless of A's late completion.
        let newer = "fe".repeat(16);
        private_dir(&job_dir(&f.m, &newer).unwrap()).unwrap();
        let latest = json!({"schema":1,"operation":newer,"state":"queued"});
        write_operation(&f.m, &newer, &latest, true).unwrap();
        drop(registry);
        worker.join().unwrap().unwrap();
        snapshot.join().unwrap().unwrap();
        assert_eq!(worker_receipt(&f.m, &id)["state"], "completed");
        assert_eq!(
            optional(&f.m.root.join("operator/latest.json")).unwrap(),
            latest
        );
        assert_eq!(onboarding::records(&f.m).unwrap().len(), 1);
    }
    #[test]
    fn readback_and_action_entry_share_one_bounded_lock_without_repeating_action() {
        let f = test_fixture::Fixture::new();
        let held = canonical_lock(&f.m).unwrap();
        let root = f.m.root.clone();
        let publications = f.m.publications.clone();
        let entered = Arc::new(AtomicBool::new(false));
        let after = entered.clone();
        let child = std::thread::spawn(move || {
            let m = Manager { root, publications };
            let _guard = canonical_lock(&m).unwrap();
            after.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(Duration::from_millis(75));
        assert!(!entered.load(Ordering::SeqCst));
        drop(held);
        child.join().unwrap();
        assert!(entered.load(Ordering::SeqCst));
    }
    #[test]
    fn software_and_registry_both_invalidate_an_operator_request_token() {
        let f = test_fixture::Fixture::new();
        atomic_json(&f.m.root.join("software.json"), &json!({"generation":1})).unwrap();
        let a = token(&f.m).unwrap();
        atomic_json(&f.m.root.join("software.json"), &json!({"generation":2})).unwrap();
        let b = token(&f.m).unwrap();
        assert_ne!(a, b);
        let mut db = f.m.registry().unwrap();
        db.revision += 1;
        atomic_json(&f.m.root.join("registry.json"), &db).unwrap();
        assert_ne!(b, token(&f.m).unwrap());
    }
}

#[cfg(test)]
mod renderer_lifetime_regression {
    use super::*;
    #[test]
    fn long_normal_session_keeps_recovery_owner_until_exact_retirement() {
        let mut observations=0;let mut waits=0;
        wait_renderer_retirement(|| {observations+=1;Ok(observations==2001)},||waits+=1).unwrap();
        assert_eq!(waits,2000); // More polls than the former 650-second/500ms cap.
        let mut waits=0;
        assert!(wait_renderer_retirement(||Err("exact_probe_failed".into()),||waits+=1).is_err());
        assert_eq!(waits,0);
    }
}

//! MF1: closed operator requests dispatched to existing canonical owners.
use super::*;
use linux_vst_bridge::operator_model as ui;
use serde_json::{json, Value};
const ASC: &str = "arturia-software-center";
const SERVICE: &str = "linux-vst-bridge.service";
const VENDOR: &str = "linux-vst-bridge-vendor-arturia-software-center.service";
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
    let output = Command::new("systemctl")
        .args(["--user", "show", VENDOR, "-p", "ActiveState", "--value"])
        .output()?;
    require(
        output.status.success(),
        "operator_vendor_unit_state_unavailable",
    )?;
    vendor_state_live(std::str::from_utf8(&output.stdout)?.trim())
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
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(
        &json!({"software":optional(&m.root.join("software.json"))?,"registry":m.registry()?}),
    )?)))
}
fn action(label: &str, action: ui::Action, reason: Option<&str>) -> ui::AvailableAction {
    ui::AvailableAction {
        label: label.into(),
        action,
        disabled_reason: reason.map(Into::into),
    }
}
fn app_directory(m: &Manager) -> PathBuf {
    m.root.join("vendor-applications").join(ASC)
}
fn vendor_retired(m: &Manager) -> Result<bool> {
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
fn pending_transactions(m: &Manager) -> Result<usize> {
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
    Ok(ui::Activity {
        schema: 2,
        system: ui::System {
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
        },
        capture: capture_state(m)?,
        operation: optional(&m.root.join("operator/latest.json"))?
            .as_object()
            .map(|v| Value::Object(v.clone())),
    })
}
fn activity(m: &Manager) -> Result<ui::Activity> {
    let cap = live_capacity(m).ok();
    activity_with_capacity(m, cap.as_ref())
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
fn require_operator_inactive(m: &Manager, a: &ui::Action) -> Result<()> {
    if !a.requires_inactive() {
        return Ok(());
    }
    let cap = live_capacity(m)?;
    let _admission = m.lock("registry.lock")?;
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
            active: entry.managed_revision.as_ref() == Some(&reference),
            rollback_allowed: ordinary && ancestors.contains(&r.id),
        });
    }
    result.sort_by_key(|r| r.revision);
    Ok(result)
}
pub(super) fn canonical_lock(m: &Manager) -> Result<Lock> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match m.lock("operator-canonical.lock") {
            Ok(lock) => return Ok(lock),
            Err(e) if e.to_string() == "operation already running" && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25))
            }
            Err(e) => return Err(e),
        }
    }
}
fn snapshot(m: &Manager) -> Result<ui::Snapshot> {
    let _projection = canonical_lock(m)?;
    let before = token(m)?;
    let sw = software(m)?;
    let canonical = m.managed_status(&sw.host, &sw.source_sha256)?;
    let db = m.registry()?;
    let cap = live_capacity(m).ok();
    let pending = pending_transactions(m)?;
    let retired = vendor_retired(m)? && onboarding::all_retired(m)?;
    let busy = inactive_reason(cap.as_ref(), retired, pending, false);
    let mut products = Vec::new();
    let profiles = profiles::installed_profiles()?;
    for p in canonical.products {
        let entry = db
            .classes
            .get(&p.class_id)
            .ok_or("operator_registry_changed")?;
        let hist = history(m, &p.class_id, entry)?;
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
            if !p.publication_valid || p.qualification.is_some() {
                Some("An exact ordinary publication is required")
            } else {
                None
            },
        ));
        products.push(ui::Product {class_id:p.class_id.clone(),name:p.name.clone(),vendor:entry.registration.metadata.vendor.clone(),role:serde_json::to_value(&p.role)?.as_str().unwrap_or("unknown").into(),version:p.build.clone(),disposition:if p.refusal.is_none() && p.publication_valid {"ready"} else {"needs_attention"}.into(),active_revision,recommended_revision:recommended.map(|p|p.revision),environment:p.environment.clone(),runner:p.runner.clone(),module_sha256:p.module_sha256.clone(),limitations:serde_json::to_value(&p.limitations)?.as_array().map(|a|a.iter().filter_map(|v|v.as_str().map(Into::into)).collect()).unwrap_or_default(),history:hist,actions,details:json!({"external_ids":p.external_ids,"profile":p.profile,"publication":p.active_revision,"module_valid":p.module_valid,"native_valid":p.native_artifact_valid,"host_valid":p.installed_host_valid,"capabilities":p.capabilities,"compatibility":p.compatibility,"recommended_frames":p.recommended_frames,"performance":p.performance,"refusal":p.refusal})});
    }
    let catalogue = sw.catalogue(m)?;
    let environments=catalogue.environments.iter().map(|e| -> Result<ui::Environment> { let scan=optional(&m.root.join("inventory").join(format!("{}.json",e.environment.id)))?; Ok(ui::Environment {id:e.environment.id.clone(),family:format!("{:?}",e.family),runner:e.environment.runner.id.clone(),revision:e.environment.revision,authorization:"Managed by the vendor and user; account state is not inspected".into(),last_scan:json!({"id":scan["id"],"completed_at":scan["completed_at"],"module_count":scan["modules"].as_array().map(Vec::len),"changes":scan["changes"]}),actions:vec![action("Rescan installed products",ui::Action::EnvironmentRescan{environment:e.environment.id.clone()},busy)]}) }).collect::<Result<Vec<_>>>()?;
    let inventory_environments: Vec<Environment> = catalogue
        .environments
        .iter()
        .map(|e| e.environment.clone())
        .chain(onboarding::records(m)?.into_iter().map(|r| r.environment))
        .collect();
    for env in &inventory_environments {
        let path = m.root.join("inventory").join(format!("{}.json", env.id));
        if path.exists() {
            let scan: inventory::Scan = read_json(&path)?;
            require(
                scan.schema == 1 && scan.environment.id == env.id,
                "inventory_environment_binding",
            )?;
            for module in scan.modules {
                let stale = inventory::stale_reason(
                    &module,
                    &scan.environment,
                    &scan.host,
                    &scan.host_source_sha256,
                    env,
                    &sw.host,
                    &sw.source_sha256,
                );
                if let Some(reason) = &module.quarantine_reason {
                    products.push(ui::Product {
                        class_id: String::new(),
                        name: module
                            .artifact
                            .path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                        vendor: "Unresolved factory".into(),
                        role: "unknown".into(),
                        version: String::new(),
                        disposition: if stale.is_some(){"needs_attention"}else{"quarantined"}.into(),
                        active_revision: None,
                        recommended_revision: None,
                        environment: scan.environment.id.clone(),
                        runner: scan.environment.runner.id.clone(),
                        module_sha256: module.artifact.sha256.clone(),
                        limitations: stale.into_iter().map(str::to_owned).chain(std::iter::once(reason.clone())).collect(),
                        history: vec![],
                        actions: vec![],
                        details: json!({"scan":scan.id,"scanner_host_sha256":scan.host.sha256,"scanner_source_sha256":scan.host_source_sha256,"current":stale.is_none(),"activation_permitted":false}),
                    });
                    continue;
                }
                let current = stale.is_none();
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
                    products.push(ui::Product {class_id:class.id,name:class.name,vendor:class.vendor,role:class.role,version:class.version,disposition:if current {"installed_unqualified"} else {"needs_attention"}.into(),active_revision:None,recommended_revision:None,environment:scan.environment.id.clone(),runner:scan.environment.runner.id.clone(),module_sha256:module.artifact.sha256.clone(),limitations:vec![stale.unwrap_or("Installed — not yet supported; not published to Bitwig").into()],history:vec![],actions:vec![],details:json!({"scan":scan.id,"observed_at":scan.completed_at,"scanner_host_sha256":scan.host.sha256,"scanner_source_sha256":scan.host_source_sha256,"current":current,"inspection_error":module.inspection_error,"activation_permitted":false})});
                }
            }
        }
    }
    let mut vendor_applications = Vec::new();
    let app = app_directory(m).join("application.json");
    if app.exists() {
        let a: vendor_application::Application = read_json(&app)?;
        let live = vendor_live()?;
        let valid = a.verify(&m.root).is_ok();
        let reason = if !valid {
            Some("Registered ASC executable or environment changed")
        } else {
            busy
        };
        vendor_applications.push(ui::VendorApplication {
            id: ASC.into(),
            name: "Arturia Software Center".into(),
            version: a.observed_installer_version,
            state: if live {
                "running"
            } else if vendor_retired(m)? {
                "closed"
            } else {
                "cleanup_unconfirmed"
            }
            .into(),
            actions: vec![
                action(
                    "Open ASC",
                    ui::Action::VendorApplicationOpen {
                        application: ASC.into(),
                    },
                    if live {
                        Some("ASC is already running")
                    } else {
                        reason
                    },
                ),
                action(
                    "Focus ASC",
                    ui::Action::VendorApplicationFocus {
                        application: ASC.into(),
                    },
                    if !live {
                        Some("ASC is not running")
                    } else {
                        None
                    },
                ),
                action(
                    "Stop owned ASC session",
                    ui::Action::VendorApplicationStop {
                        application: ASC.into(),
                    },
                    if !live {
                        Some("ASC is not running")
                    } else {
                        None
                    },
                ),
            ],
        });
    }
    let mut incidents = Vec::new();
    let mut paths = crash_capture::incidents(m)?;
    paths.sort_by_key(|p| {
        fs::metadata(p.join("status.json"))
            .and_then(|m| m.modified())
            .ok()
    });
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
    let after = token(m)?;
    require(before == after, "operator_state_changed_refresh")?;
    let live = activity_with_capacity(m, cap.as_ref())?;
    let onboarding = onboarding::projection(m, busy)?;
    Ok(ui::Snapshot {
        onboarding,
        schema: 2,
        state_token: after,
        system: live.system,
        environments,
        vendor_applications,
        products,
        active_sessions: cap
            .as_ref().into_iter().flat_map(|c|c.owners.iter())
            .filter(|o| o.kind == capacity::Kind::Dsp)
            .map(|o| json!({"class_id":o.class_id,"state":if cap.as_ref().is_some_and(|c|c.cleanup_unconfirmed){"cleanup_unconfirmed"}else{"active"}}))
            .collect(),
        capture: capture_state(m)?,
        recent_incidents: incidents,
        actions: vec![
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
fn available(snapshot: &ui::Snapshot) -> Vec<&ui::AvailableAction> {
    snapshot
        .actions
        .iter()
        .chain(snapshot.onboarding.iter().flat_map(|p| p.actions.iter()))
        .chain(snapshot.products.iter().flat_map(|p| p.actions.iter()))
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
    require(
        request.schema == 2 && request.state_token == snapshot.state_token,
        "operator_stale_request_refresh",
    )?;
    let offered = available(snapshot)
        .into_iter()
        .find(|a| a.action == request.action)
        .ok_or("operator_action_not_available")?;
    require(
        offered.disabled_reason.is_none(),
        offered
            .disabled_reason
            .as_deref()
            .unwrap_or("operator_action_disabled"),
    )
}
fn job_dir(m: &Manager, id: &str) -> Result<PathBuf> {
    require(valid_hex(id, 32), "operator_operation_identity")?;
    Ok(m.root.join("operator").join(id))
}
fn write_operation(m: &Manager, id: &str, value: &Value, make_latest: bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let _lock = loop {
        match m.lock("operator-receipt.lock") {
            Ok(lock) => break lock,
            Err(e) if e.to_string() == "operation already running" && Instant::now() < deadline => {
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
    atomic_json(&dir.join("result.json"), value)?;
    let latest = m.root.join("operator/latest.json");
    if make_latest || optional(&latest)?["operation"] == id {
        atomic_json(&latest, value)?;
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
fn finish_operation(m: &Manager, id: &str) -> Result<()> {
    let request: ui::Request = read_json(&job_dir(m, id)?.join("request.json"))?;
    if let ui::Action::InstallerStart {
        onboarding: ref key,
    } = request.action
    {
        let r = onboarding::load(m, key)?;
        if r.installation_operation.as_deref() == Some(id) {
            if onboarding::live(id)? {let _ = onboarding::stop(m, key, id);}
            onboarding::mark_dead(m,&r)?;
        }
    }
    finish_operation_with(m, id, |saved| restore_service(m, saved))
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
fn launch_queued(
    m: &Manager,
    request: &ui::Request,
    launch: impl FnOnce(&str) -> Result<bool>,
) -> Result<ui::Receipt> {
    let id = random_id()?;
    let dir = job_dir(m, &id)?;
    private_dir(&dir)?;
    atomic_json(&dir.join("request.json"), request)?;
    write_operation(
        m,
        &id,
        &json!({"schema":1,"operation":id,"state":"queued","action":request.action}),
        true,
    )?;
    if !launch(&id).unwrap_or(false) {
        refuse_unfinished(m, &id, "operator_worker_launch_failed")?;
        return Err("operator_worker_launch_failed".into());
    }
    Ok(ui::Receipt {
        schema: 2,
        accepted: true,
        operation: Some(id),
        refusal: None,
    })
}
fn dispatch(m: &Manager, request: ui::Request) -> Result<ui::Receipt> {
    let _lock = m.lock("operator-dispatch.lock")?;
    validate(&request, &snapshot(m)?)?;
    let sw = software(m)?;
    sw.manager.verify()?;
    launch_queued(m, &request, |id| {
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
}

#[cfg(test)]
fn execute(m: &Manager, a: &ui::Action) -> Result<Value> {
    execute_with_receipt(m, a, None)
}
fn execute_with_receipt(m: &Manager, a: &ui::Action, operation: Option<&str>) -> Result<Value> {
    let mut projection = Some(canonical_lock(m)?);
    require_operator_inactive(m, a)?;

    match a {
        ui::Action::InstallerEnvironmentCreate { installer, runner } => onboarding::create(
            m,
            installer,
            runner,
            operation.ok_or("operator_operation_identity")?,
        ),
        ui::Action::InstallerStart { onboarding: id } => {
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            let prior = onboarding::load(m, id)?;
            require(
                prior.installation_operation.is_none(),
                "installer_initial_transaction_already_used",
            )?;
            suspend(m, owner, None)?;
            let reserved = onboarding::reserve(m, id, owner);
            let r = match reserved {
                Ok(r) => r,
                Err(e) => {
                    let _ = resume_owned(m, owner);
                    return Err(e);
                }
            };
            let launched = onboarding::launch(m, &r);
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
            suspend(m, owner, None)?;
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
        ui::Action::VendorApplicationOpen { application } => {
            vendor_application::ApplicationId::parse(application)?;
            let _environment = m.lock("operator-environment.lock")?;
            let owner = operation.ok_or("operator_operation_identity")?;
            let vendor_operation = random_id()?;
            suspend(m, owner, Some(vendor_operation.clone()))?;
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
            suspend(m, owner, None)?;
            let result = rescan(m, environment);
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
                if e.to_string() != "operation already running" || Instant::now() >= deadline {
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
fn suspend(m: &Manager, owner: &str, vendor_operation: Option<String>) -> Result<()> {
    // Same lock order as recovery: resume, then registry. Hold through Stop so
    // even the matching owner cannot consume the record before suspension.
    let _resume = resume_lock(m)?;
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(vendor_retired(m)?, "operator_vendor_active")?;
    let was_active = active(SERVICE);
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
        service("stop")?;
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
    require(request.schema == 2, "operator_resume_request_schema")?;
    Ok(request.action)
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
    // Only explicit reconciliation can recover a completed prior owner. It
    // cannot steal an active Open/Rescan or adopt an ownerless legacy record.
    let _resume = resume_lock(m)?;
    let Some(saved) = resume_record(m)? else {
        return Ok(());
    };
    let _environment = m.lock("operator-environment.lock")?;
    require(
        matches!(
            recovery_request(m, &saved)?,
            ui::Action::InstallerStart { .. }
                | ui::Action::InstallerScan { .. }
                | ui::Action::VendorApplicationOpen { .. }
                | ui::Action::EnvironmentRescan { .. }
        ),
        "operator_resume_action_mismatch",
    )?;
    let result = optional(&job_dir(m, &saved.owner_operation)?.join("result.json"))?;
    require(
        result["operation"] == saved.owner_operation
            && matches!(result["state"].as_str(), Some("completed" | "refused")),
        "operator_resume_owner_not_terminal",
    )?;
    resume_locked(m, &saved.owner_operation, |saved| restore_service(m, saved))
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
fn rescan(m: &Manager, environment: &str) -> Result<Value> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let sw = software(m)?;
    let c = sw.catalogue(m)?;
    let env = c
        .environments
        .iter()
        .find(|e| e.environment.id == environment)
        .ok_or("operator_environment_absent")?
        .environment
        .clone();
    drop(_lock);
    rescan_environment(m, env)
}
fn rescan_environment(m: &Manager, env: Environment) -> Result<Value> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let sw = software(m)?;
    let environment = env.id.clone();
    let modules = managed_cli::modules(&env)?;
    require(modules.len() <= 64, "operator_scan_module_bound")?;
    let prior_path = m.root.join("inventory").join(format!("{environment}.json"));
    let prior = if prior_path.exists() {
        Some(read_json::<inventory::Scan>(&prior_path)?)
    } else {
        None
    };
    let start = Instant::now();
    let mut found = Vec::new();
    for module in modules {
        if let Some(old) = prior
            .as_ref()
            .filter(|s| {
                s.environment == env
                    && s.host == sw.host
                    && s.host_source_sha256 == sw.source_sha256
            })
            .and_then(|s| {
                s.modules
                    .iter()
                    .find(|p| p.artifact == module && p.quarantine_reason.is_some())
            })
        {
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
        let mut pending =
            PendingAdmission::new(job.lease.clone(), Arc::new(AtomicBool::new(false)));
        let child = spawn(&sw, &path, None)?;
        pending.expose();
        vendor_product_cli::finish_scan(child, &job, pending)?;
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
    Ok(json!({"scan":scan.id,"modules":scan.modules.len(),"activation_permitted":false}))
}
fn worker(m: &Manager, id: &str) -> Result<()> {
    let dir = job_dir(m, id)?;
    let request: ui::Request = read_json(&dir.join("request.json"))?;
    let result: Result<Value> = (|| {
        validate(
            &request,
            &snapshot(m).map_err(|e| format!("Operator validation readback: {e}"))?,
        )?;
        let state = json!({"schema":1,"operation":id,"state":"running","action":request.action});
        write_operation(m, id, &state, false)?;
        execute_with_receipt(m, &request.action, Some(id))
            .map_err(|e| format!("Operator action: {e}").into())
    })();
    let value = match result {
        Ok(v) => json!({"schema":1,"operation":id,"state":"completed","result":v}),
        Err(e) => json!({"schema":1,"operation":id,"state":"refused","reason":e.to_string()}),
    };
    write_operation(m, id, &value, false)?;
    Ok(())
}
pub(super) fn run(m: &Manager, args: &[String]) -> Result<()> {
    match args {
        [a] if a == "snapshot" => println!("{}", serde_json::to_string(&snapshot(m)?)?),
        [a] if a == "activity" => println!("{}", serde_json::to_string(&activity(m)?)?),
        [a] if a == "request" => {
            let mut bytes = Vec::new();
            std::io::stdin().take(16385).read_to_end(&mut bytes)?;
            require(bytes.len() <= 16384, "operator_request_bound")?;
            let req = serde_json::from_slice(&bytes)?;
            let receipt = match dispatch(m, req) {
                Ok(r) => r,
                Err(e) => ui::Receipt {
                    schema: 2,
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

#[cfg(test)]
mod tests {
    use super::*;
    fn view(token: &str, action: ui::AvailableAction) -> ui::Snapshot {
        ui::Snapshot {
            onboarding: vec![],
            schema: 2,
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
            active_sessions: vec![],
            capture: Value::Null,
            recent_incidents: vec![],
            actions: vec![action],
            operation: None,
        }
    }
    #[test]
    fn stale_foreign_candidate_busy_and_unknown_requests_cannot_dispatch() {
        let allowed = ui::Action::OrdinaryRollback {
            class_id: "a".repeat(32),
            publication: "b".repeat(32),
        };
        let s = view("current", action("Rollback", allowed.clone(), None));
        let mut r = ui::Request {
            schema: 2,
            state_token: "current".into(),
            action: allowed.clone(),
        };
        validate(&r, &s).unwrap();
        r.state_token = "previous software or registry".into();
        assert!(validate(&r, &s).is_err());
        r.state_token = "current".into();
        r.schema = 1;
        assert!(validate(&r, &s).is_err());
        r.schema = 2;
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
            ui::Action::InstallerEnvironmentCreate{installer:"ab".repeat(32),runner:"cd".repeat(32)},
            ui::Action::InstallerStart{onboarding:"ef".repeat(16)},
            ui::Action::InstallerScan{onboarding:"ef".repeat(16)},
            ui::Action::TransactionReconcile {},
            ui::Action::EnvironmentRescan {
                environment: f.r.environment.id.clone(),
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
            schema: 2,
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
            schema: 2,
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
        for state in ["queued", "running", "vendor_running"] {
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
                schema: 2,
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

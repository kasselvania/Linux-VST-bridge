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
    if active(VENDOR) {
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
fn activity(m: &Manager) -> Result<ui::Activity> {
    // Read live ownership only. Full profile/envelope hashing belongs to snapshot.
    let owners = capacity::owners(m)?;
    let limits = capacity::service_limits()?;
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
    let root = transport_storage::root();
    let mut stale = 0;
    if root.exists() {
        require(count_entries(&root)? <= 4096, "operator_transport_bound")?;
        for item in fs::read_dir(root)? {
            let item = item?;
            let name = item.file_name().to_string_lossy().into_owned();
            if item.file_type()?.is_dir()
                && valid_hex(&name, 32)
                && !owners.iter().any(|o| o.session == name)
            {
                stale += 1;
            }
        }
    }
    Ok(ui::Activity {
        schema: 1,
        system: ui::System {
            service: if active(SERVICE) { "active" } else { "stopped" }.into(),
            keepers: owners.iter().filter(|o|o.kind == capacity::Kind::Keeper).count(),
            dsp: owners.iter().filter(|o|o.kind == capacity::Kind::Dsp).count(),
            maintenance: owners.iter().filter(|o|matches!(o.kind,capacity::Kind::Inspection|capacity::Kind::VendorAccess)).count(),
            ceiling: limits.global_dsp,
            pending_transactions: pending,
            stale_transports: stale,
            cleanup_unconfirmed: false,
        },
        capture: capture_state(m)?,
        operation: optional(&m.root.join("operator/latest.json"))?
            .as_object()
            .map(|v| Value::Object(v.clone())),
    })
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
pub(super) // Serialize this frontend's heavy readback with its mutation entry points.
// Waiting is for the lock only: no operation is retried after it has begun.
fn canonical_lock(m: &Manager) -> Result<Lock> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match m.lock("operator-canonical.lock") {
            Ok(lock) => return Ok(lock),
            Err(e) if e.to_string() == "operation already running" && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(25)),
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
    let cap = capacity::status(m, capacity::service_limits()?, 0, false)?;
    let pending = canonical
        .products
        .iter()
        .filter(|p| p.recovery_pending)
        .count();
    let busy = if cap.cleanup_unconfirmed {
        Some("Previous instance cleanup is unconfirmed")
    } else if cap.dsp > 0 || cap.maintenance > 0 {
        Some("Close active bridged instances before this action")
    } else if !vendor_retired(m)? {
        Some("Close Arturia Software Center and wait for its owned operation to retire")
    } else if pending > 0 {
        Some("Reconcile the interrupted publication first")
    } else {
        None
    };
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
    for e in &catalogue.environments {
        let path = m
            .root
            .join("inventory")
            .join(format!("{}.json", e.environment.id));
        if path.exists() {
            let scan: inventory::Scan = read_json(&path)?;
            require(
                scan.schema == 1 && scan.environment.id == e.environment.id,
                "inventory_environment_binding",
            )?;
            for module in scan.modules {
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
                        disposition: "quarantined".into(),
                        active_revision: None,
                        recommended_revision: None,
                        environment: scan.environment.id.clone(),
                        runner: scan.environment.runner.id.clone(),
                        module_sha256: module.artifact.sha256.clone(),
                        limitations: vec![reason.clone()],
                        history: vec![],
                        actions: vec![],
                        details: json!({"scan":scan.id,"activation_permitted":false}),
                    });
                    continue;
                }
                let current = module.artifact.verify().is_ok() && scan.environment == e.environment;
                for class in module.classes {
                    if class.category != "Audio Module Class"
                        || products.iter().any(|p| {
                            p.class_id == class.id && p.module_sha256 == module.artifact.sha256
                        })
                    {
                        continue;
                    }
                    products.push(ui::Product {class_id:class.id,name:class.name,vendor:class.vendor,role:class.role,version:class.version,disposition:if current {"installed_unqualified"} else {"needs_attention"}.into(),active_revision:None,recommended_revision:None,environment:scan.environment.id.clone(),runner:scan.environment.runner.id.clone(),module_sha256:module.artifact.sha256.clone(),limitations:vec!["Installed — not yet supported; not published to Bitwig".into()],history:vec![],actions:vec![],details:json!({"scan":scan.id,"observed_at":scan.completed_at,"inspection_error":module.inspection_error,"activation_permitted":false})});
                }
            }
        }
    }
    let mut vendor_applications = Vec::new();
    let app = app_directory(m).join("application.json");
    if app.exists() {
        let a: vendor_application::Application = read_json(&app)?;
        let live = active(VENDOR);
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
    let live = activity(m)?;
    Ok(ui::Snapshot {
        schema: 1,
        state_token: after,
        system: live.system,
        environments,
        vendor_applications,
        products,
        active_sessions: cap
            .owners
            .into_iter()
            .filter(|o| o.kind == capacity::Kind::Dsp)
            .map(|o| json!({"class_id":o.class_id,"state":"active"}))
            .collect(),
        capture: capture_state(m)?,
        recent_incidents: incidents,
        actions: vec![
            action("Disarm crash capture", ui::Action::CaptureDisarm {}, None),
            action(
                "Reconcile interrupted transaction",
                ui::Action::TransactionReconcile {},
                if cap.dsp > 0 {
                    Some("Close active bridged instances")
                } else {
                    None
                },
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
        request.schema == 1 && request.state_token == snapshot.state_token,
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
fn dispatch(m: &Manager, request: ui::Request) -> Result<ui::Receipt> {
    let _lock = m.lock("operator-dispatch.lock")?;
    validate(&request, &snapshot(m)?)?;
    let sw = software(m)?;
    sw.manager.verify()?;
    let id = random_id()?;
    let dir = job_dir(m, &id)?;
    private_dir(&dir)?;
    atomic_json(&dir.join("request.json"), &request)?;
    let initial = json!({"schema":1,"operation":id,"state":"queued","action":request.action});
    atomic_json(&dir.join("result.json"), &initial)?;
    atomic_json(&m.root.join("operator/latest.json"), &initial)?;
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
            "--property=ExecStopPost={} operator resume",
            systemd(
                sw.manager
                    .path
                    .to_str()
                    .ok_or("operator_executable_encoding")?
            )
        ))
        .arg(format!("--unit=linux-vst-bridge-operator-{id}"))
        .arg(&sw.manager.path)
        .args(["operator", "worker", &id])
        .status()?;
    require(status.success(), "operator_worker_launch_failed")?;
    Ok(ui::Receipt {
        schema: 1,
        accepted: true,
        operation: Some(id),
        refusal: None,
    })
}
#[cfg(test)]
fn execute(m: &Manager, a: &ui::Action) -> Result<Value> {
    execute_with_receipt(m, a, None)
}
fn execute_with_receipt(m: &Manager, a: &ui::Action, operation: Option<&str>) -> Result<Value> {
    let mut projection = Some(canonical_lock(m)?);

    match a {
        ui::Action::CaptureArm { class_id } => {
            crash_capture::arm(m, Some(class_id))?;
            Ok(json!({"capture":"armed"}))
        }
        ui::Action::CaptureDisarm {} => {
            crash_capture::disarm(m)?;
            Ok(json!({"capture":"disarmed"}))
        }
        ui::Action::TransactionReconcile {} => {
            m.reconcile()?;
            resume(m)?;
            Ok(json!({"reconciled":true}))
        }
        ui::Action::OrdinaryRollback {
            class_id,
            publication,
        } => Ok(serde_json::to_value(m.rollback(
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
            suspend(m)?;
            let launched = vendor_cli::run(m, &["launch".into(), application.clone()]);
            if let Err(e) = launched {
                let _ = resume(m);
                return Err(e);
            }
            if let Some(id) = operation {
                let receipt =
                    json!({"schema":1,"operation":id,"state":"vendor_running","action":a});
                atomic_json(&job_dir(m, id)?.join("result.json"), &receipt)?;
                atomic_json(&m.root.join("operator/latest.json"), &receipt)?;
            }
            // This manager operation outlives the frontend. ASC's own existing
            // dedicated unit remains process owner; neither UI close nor this
            // operation creates an orphan or kills a continuing download.
            drop(projection.take());
            while active(VENDOR) {
                std::thread::sleep(Duration::from_millis(500));
            }
            require(vendor_retired(m)?, "operator_vendor_cleanup_unconfirmed")?;
            resume(m)?;
            Ok(json!({"vendor":"retired","service":"resumed"}))
        }
        ui::Action::VendorApplicationStop { application } => {
            vendor_cli::run(m, &["cancel".into(), application.clone()])?;
            resume(m)?;
            Ok(json!({"vendor":"stopped"}))
        }
        ui::Action::VendorApplicationFocus { application } => {
            vendor_application::ApplicationId::parse(application)?;
            require(active(VENDOR), "operator_vendor_not_running")?;
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
            suspend(m)?;
            let result = rescan(m, environment);
            let cleanup = resume(m);
            let value = result?;
            cleanup?;
            Ok(value)
        }
    }
}
fn suspend(m: &Manager) -> Result<()> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(vendor_retired(m)?, "operator_vendor_active")?;
    let was_active = active(SERVICE);
    atomic_json(
        &m.root.join("operator/resume.json"),
        &json!({"schema":1,"resume":was_active,"software":software(m)?.manager.sha256}),
    )?;
    if was_active {
        service("stop")?;
    }
    require(!reconcile_leases(m)?, "operator_cleanup_unconfirmed")?;
    Ok(())
}
fn resume(m: &Manager) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(75);
    let _resume = loop {
        match m.lock("operator-resume.lock") {
            Ok(lock) => break lock,
            Err(e) => {
                if e.to_string() != "operation already running" || Instant::now() >= deadline {
                    return Err(e);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    };
    let path = m.root.join("operator/resume.json");
    let saved = optional(&path)?;
    if saved.is_null() {
        return Ok(());
    }
    require(vendor_retired(m)?, "operator_vendor_cleanup_unconfirmed")?;
    require(
        saved["software"] == software(m)?.manager.sha256,
        "operator_resume_software_changed",
    )?;
    if saved["resume"] == true {
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
    fs::remove_file(path)?;
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
        validate(&request, &snapshot(m).map_err(|e| format!("Operator validation readback: {e}"))?)?;
        let state = json!({"schema":1,"operation":id,"state":"running","action":request.action});
        atomic_json(&dir.join("result.json"), &state)?;
        atomic_json(&m.root.join("operator/latest.json"), &state)?;
        execute_with_receipt(m, &request.action, Some(id)).map_err(|e| format!("Operator action: {e}").into())
    })();
    let value = match result {
        Ok(v) => json!({"schema":1,"operation":id,"state":"completed","result":v}),
        Err(e) => json!({"schema":1,"operation":id,"state":"refused","reason":e.to_string()}),
    };
    atomic_json(&dir.join("result.json"), &value)?;
    atomic_json(&m.root.join("operator/latest.json"), &value)?;
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
                    schema: 1,
                    accepted: false,
                    operation: None,
                    refusal: Some(e.to_string()),
                },
            };
            println!("{}", serde_json::to_string(&receipt)?);
        }
        [a, id] if a == "worker" => worker(m, id)?,
        [a] if a == "resume" => resume(m)?,
        _ => return Err("operator snapshot | request".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view(token: &str, action: ui::AvailableAction) -> ui::Snapshot {
        ui::Snapshot {
            schema: 1,
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
            schema: 1,
            state_token: "current".into(),
            action: allowed.clone(),
        };
        validate(&r, &s).unwrap();
        r.state_token = "previous software or registry".into();
        assert!(validate(&r, &s).is_err());
        r.state_token = "current".into();
        r.schema = 2;
        assert!(validate(&r, &s).is_err());
        r.schema = 1;
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
        execute(
            &f.m,
            &ui::Action::OrdinaryRollback {
                class_id: key.clone(),
                publication: first.id.clone(),
            },
        )
        .unwrap();
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
    fn idle_activity_does_not_take_the_registry_mutation_lock() {
        let f = test_fixture::Fixture::new();
        let _mutation = f.m.lock("registry.lock").unwrap();
        let read = activity(&f.m).unwrap();
        assert_eq!(read.system.dsp,0);
        assert_eq!(read.system.maintenance,0);
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
            let m = Manager {root,publications};
            let _guard = canonical_lock(&m).unwrap();
            after.store(true,Ordering::SeqCst);
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

//! Fixed manager entry point; no shell and no user-supplied executable/arguments.
use crate::model::{Activity, CurrentOverview, CurrentProductDetail, InstallerImportResult, InteractiveOverview, Pulse, Receipt, Request, Snapshot};
use crate::presentation::ProductKey;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{mpsc::Sender, OnceLock},
    time::{Duration, Instant},
};
pub enum Reply {
    Overview(Box<InteractiveOverview>),
    CurrentOverview(Box<CurrentOverview>),
    Product(Box<CurrentProductDetail>),
    Snapshot(Box<Snapshot>),
    Activity(Activity),
    Pulse(Pulse),
    Receipt(Receipt),
    Imported(InstallerImportResult),
    Cancelled,
    FrontendSuperseded,
    Error(String),
}
pub enum Query {
    PickInstaller,
    Overview,
    CurrentOverview,
    Product(ProductKey),
    Snapshot,
    Activity,
    Pulse,
    Action(Request),
}
// Capture this immutable frontend's content once, before its first request.
// Selection readback then remains a small record read even when an older
// manager is restored and cannot send the new superseded refusal itself.
static FRONTEND_IDENTITY: OnceLock<Result<Option<linux_vst_bridge::Artifact>, String>> = OnceLock::new();
fn frontend_identity(home: &std::path::Path, executable: &std::path::Path)
    -> Result<Option<linux_vst_bridge::Artifact>, String> {
    use std::os::unix::fs::PermissionsExt;
    let root = home.join(".local/share/linux-vst-bridge/managed/software");
    if executable.parent().is_none_or(|dir| dir.parent() != Some(root.as_path()))
        || executable.file_name() != Some(std::ffi::OsStr::new("linux-audio-compatibility-manager")) {
        return Ok(None);
    }
    let metadata = linux_vst_bridge::file(executable).and_then(|f| Ok(f.metadata()?))
        .map_err(|_| "Frontend identity unavailable")?;
    if executable.canonicalize().ok().as_deref() != Some(executable)
        || metadata.permissions().mode() & 0o222 != 0 || metadata.len() > 128 * 1024 * 1024 {
        return Err("Managed frontend identity changed".into());
    }
    let sha256 = linux_vst_bridge::digest(executable).map_err(|_| "Frontend identity unavailable")?;
    Ok(Some(linux_vst_bridge::Artifact { path:executable.to_owned(), sha256 }))
}
fn frontend_selected(home: &std::path::Path, frontend: &linux_vst_bridge::Artifact)
    -> Result<bool, String> {
    use std::os::unix::fs::PermissionsExt;
    let root = home.join(".local/share/linux-vst-bridge/managed");
    let selected: linux_vst_bridge::catalogue::Software =
        linux_vst_bridge::read_json(&root.join("software.json"))
            .map_err(|_| "Selected application identity is unavailable")?;
    let current = selected.operator_frontend.as_ref().ok_or("Selected frontend is unavailable")?;
    current.validate_record().map_err(|_| "Selected frontend identity is invalid")?;
    if current.path.parent() != selected.manager.path.parent()
        || current.path.parent().and_then(|dir|dir.parent()) != Some(root.join("software").as_path())
        || current.path.canonicalize().ok().as_ref() != Some(&current.path)
        || std::fs::metadata(&current.path).map_err(|_| "Selected frontend is unavailable")?
            .permissions().mode() & 0o222 != 0 {
        return Err("Selected frontend identity is invalid".into());
    }
    Ok(current.sha256 == frontend.sha256)
}
fn call(query: Query) -> Result<Reply, String> {
    let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
    let home = std::path::PathBuf::from(home);
    let identity = FRONTEND_IDENTITY.get_or_init(|| {
        let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
        frontend_identity(&home, &executable)
    }).as_ref().map_err(Clone::clone)?;
    if let Some(frontend) = identity {
        if !frontend_selected(&home, frontend)? { return Ok(Reply::FrontendSuperseded); }
    }
    let executable = home.join(".local/bin/linux-vst-bridge");
    let selected = if matches!(query, Query::PickInstaller) {
        let selected = rfd::FileDialog::new()
            .set_title("Add Windows installer")
            .add_filter("Windows installers", &["exe", "msi"])
            .pick_file();
        let Some(path) = selected else {
            return Ok(Reply::Cancelled);
        };
        let basename = path.file_name().and_then(|part| part.to_str())
            .ok_or("Selected installer name is unavailable")?.to_owned();
        Some((open_selected(&path)?, basename))
    } else {
        None
    };
    let verb = match &query {
        Query::PickInstaller => "import-installer",
        Query::Overview => "overview",
        Query::CurrentOverview => "current-overview",
        Query::Product(_) => "product",
        Query::Snapshot => "snapshot",
        Query::Activity => "activity",
        Query::Pulse => "pulse",
        Query::Action(_) => "request",
    };
    let mut command = Command::new(executable);
    if matches!(query, Query::PickInstaller) {
        command.arg(verb).arg(selected.as_ref().ok_or("Installer selection unavailable")?.1.as_str());
    } else {
        command.args(["operator", verb]);
        if let Query::Product(key) = &query {
            command.args([&key.environment, &key.module_sha256, &key.class_id]);
        }
    }
    let mut child = command
        .stdin(selected.map(|(file, _)| Stdio::from(file)).unwrap_or_else(Stdio::piped))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "Installed manager is unavailable")?;
    if let Query::Action(request) = &query {
        child
            .stdin
            .as_mut()
            .ok_or("Manager input unavailable")?
            .write_all(&serde_json::to_vec(request).map_err(|_| "Request encoding failed")?)
            .map_err(|_| "Request delivery failed")?;
    }
    drop(child.stdin.take());
    let collect = |mut pipe: Box<dyn Read + Send>, limit: u64| {
        std::thread::spawn(move || {
            let mut data = Vec::new();
            let result = pipe.by_ref().take(limit + 1).read_to_end(&mut data);
            let overflow = data.len() > limit as usize;
            let _ = std::io::copy(&mut pipe, &mut std::io::sink());
            (data, overflow, result.is_ok())
        })
    };
    let out = collect(
        Box::new(child.stdout.take().ok_or("Manager output unavailable")?),
        8 * 1024 * 1024,
    );
    let err = collect(
        Box::new(child.stderr.take().ok_or("Manager error unavailable")?),
        4096,
    );
    let deadline = Instant::now()
        + Duration::from_secs(if matches!(query, Query::PickInstaller) {
            300
        } else {
            45
        });
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|_| "Manager wait failed")? {
            break s;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "Manager readback timed out. Refresh to inspect any accepted operation.".into(),
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (data, overflow, read) = out.join().map_err(|_| "Manager reader failed")?;
    let (error, _, _) = err.join().map_err(|_| "Manager error reader failed")?;
    if !status.success() {
        let error = String::from_utf8_lossy(&error);
        let refusal = error.trim().strip_prefix("Error: ").unwrap_or(error.trim())
            .trim_matches('"');
        if refusal.starts_with("operator_frontend_superseded: ") {
            return Ok(Reply::FrontendSuperseded);
        }
        return Err(error.chars().take(512).collect());
    }
    if overflow || !read {
        return Err("Manager response exceeded its bound".into());
    }
    if matches!(query, Query::PickInstaller) {
        return decode_import(&data);
    }
    decode_reply(query, &data)
}
fn decode_import(data: &[u8]) -> Result<Reply, String> {
    let envelope: serde_json::Value =
        serde_json::from_slice(data).map_err(|_| "Invalid import receipt")?;
    if envelope["schema"] != 2 {
        return Err("Update the frontend and manager together: installer import result 2 required".into());
    }
    let value: InstallerImportResult = serde_json::from_value(envelope)
        .map_err(|_| "Incompatible installer import result")?;
    if value.installer_sha256.len() != 64
        || !value.installer_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.display_label.is_empty() || value.display_label.len() > 96
        || value.display_label.chars().any(char::is_control)
        || !matches!(value.format.as_str(), "pe_executable" | "msi_compound") {
        return Err("Installer import refused".into());
    }
    Ok(Reply::Imported(value))
}

fn decode_reply(query: Query, data: &[u8]) -> Result<Reply, String> {
    let envelope: serde_json::Value =
        serde_json::from_slice(data).map_err(|_| "Invalid manager response")?;
    if matches!(query, Query::CurrentOverview) {
        if envelope["schema"] != 1 || envelope["operator_schema"] != crate::model::OPERATOR_SCHEMA {
            return Err(format!("Update the frontend and manager together: operator model {} required",
                crate::model::OPERATOR_SCHEMA));
        }
        let overview: CurrentOverview = serde_json::from_slice(data)
            .map_err(|_| "Incompatible manager current overview; update the frontend and manager together")?;
        if overview.scope != "current_only" || overview.current.schema != crate::model::OPERATOR_SCHEMA {
            return Err("Inconsistent manager current overview; refresh before acting".into());
        }
        return Ok(Reply::CurrentOverview(Box::new(overview)));
    }
    if matches!(query, Query::Overview) {
        if envelope["schema"] != 1 || envelope["operator_schema"] != crate::model::OPERATOR_SCHEMA {
            return Err(format!("Update the frontend and manager together: operator model {} required",
                crate::model::OPERATOR_SCHEMA));
        }
        let overview: InteractiveOverview = serde_json::from_slice(data)
            .map_err(|_| "Incompatible manager overview; update the frontend and manager together")?;
        if overview.scope != "current_only" || overview.current.schema != crate::model::OPERATOR_SCHEMA
            || overview.readiness.schema != 1
            || overview.current.state_token != overview.readiness.state_token
            || overview.current.system != overview.readiness.system {
            return Err("Inconsistent manager overview; refresh before acting".into());
        }
        return Ok(Reply::Overview(Box::new(overview)));
    }
    if let Query::Product(key) = &query {
        if envelope["schema"] != 1 || envelope["operator_schema"] != crate::model::OPERATOR_SCHEMA {
            return Err(format!("Update the frontend and manager together: operator model {} required",
                crate::model::OPERATOR_SCHEMA));
        }
        let detail: CurrentProductDetail = serde_json::from_slice(data)
            .map_err(|_| "Incompatible manager product readback; update the frontend and manager together")?;
        if detail.state_token.is_empty() || detail.current_generation.is_empty()
            || ProductKey::from(&detail.product) != *key
            || detail.environments.iter().any(|environment| environment.id != key.environment)
            || detail.vendor_applications.iter().any(|application|
                application.details["environment"].as_str() != Some(key.environment.as_str())) {
            return Err("Inconsistent manager product readback; refresh before acting".into());
        }
        return Ok(Reply::Product(Box::new(detail)));
    }
    if envelope["schema"] != crate::model::OPERATOR_SCHEMA {
        return Err(format!("Update the frontend and manager together: operator model {} required",
            crate::model::OPERATOR_SCHEMA));
    }
    match query {
        Query::PickInstaller => unreachable!(),
        Query::Overview => unreachable!(),
        Query::CurrentOverview => unreachable!(),
        Query::Product(_) => unreachable!(),
        Query::Snapshot => {
            serde_json::from_slice::<Snapshot>(data).map(|s| Reply::Snapshot(Box::new(s)))
        }
        Query::Activity => serde_json::from_slice::<Activity>(data).map(Reply::Activity),
        Query::Pulse => serde_json::from_slice::<Pulse>(data).map(Reply::Pulse),
        Query::Action(_) => serde_json::from_slice::<Receipt>(data).map(Reply::Receipt),
    }
    .map_err(|_| "Incompatible manager response; update the frontend and manager together".into())
}
pub fn send(query: Query, sender: Sender<Reply>, ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let result = call(query).unwrap_or_else(Reply::Error);
        let _ = sender.send(result);
        ctx.request_repaint();
    });
}

#[path = "../../bridge-manager/src/installer_source.rs"]
mod installer_source;

fn open_selected(path: &std::path::Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "Choose a regular local installer file; links are not accepted")?;
    let md = f.metadata().map_err(|_| "Cannot read selected file")?;
    if !md.is_file()
        || !installer_source::accepted_owner(md.uid(), md.mode(), unsafe { libc::getuid() })
    {
        return Err("Choose a regular installer owned by you or protected by the system".into());
    }
    Ok(f)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frontend_content_binding_detects_restore_without_help_from_old_manager() {
        use std::os::unix::fs::PermissionsExt;
        let home = std::env::temp_dir().join(format!("frontend-selection-{}",
            linux_vst_bridge::random_id().unwrap()));
        std::fs::create_dir(&home).unwrap();
        let home = home.canonicalize().unwrap();
        let root = home.join(".local/share/linux-vst-bridge/managed");
        let dir = root.join("software/current");
        std::fs::create_dir_all(&dir).unwrap();
        let executable = dir.join("linux-audio-compatibility-manager");
        std::fs::write(&executable, b"current frontend content").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o400)).unwrap();
        let identity = frontend_identity(&home, &executable).unwrap().unwrap();
        let artifact = identity.clone();
        let mut selected = linux_vst_bridge::catalogue::Software {
            installer_launch:None, preparation_kit:None, operator_frontend:Some(artifact.clone()),
            manager:artifact.clone(), supervisor:artifact.clone(), ownership:artifact.clone(),
            host:artifact.clone(), source_manifest:artifact.clone(), source_sha256:artifact.sha256,
            native_catalogue:None,
        };
        let selection = root.join("software.json");
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(frontend_selected(&home, &identity).unwrap());
        let previous = root.join("software/previous/linux-audio-compatibility-manager");
        std::fs::create_dir_all(previous.parent().unwrap()).unwrap();
        std::fs::write(&previous, b"previous frontend content").unwrap();
        std::fs::set_permissions(&previous, std::fs::Permissions::from_mode(0o400)).unwrap();
        let predecessor = linux_vst_bridge::Artifact { path:previous,
            sha256:linux_vst_bridge::digest(&root.join("software/previous/linux-audio-compatibility-manager")).unwrap() };
        selected.operator_frontend = Some(predecessor.clone());
        selected.manager = predecessor;
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(!frontend_selected(&home, &identity).unwrap());
        selected.operator_frontend = Some(identity.clone());
        selected.manager = identity.clone();
        linux_vst_bridge::atomic_json(&selection, &selected).unwrap();
        assert!(frontend_selected(&home, &identity).unwrap());
        assert!(frontend_identity(&home, &home.join("unmanaged/frontend")).unwrap().is_none());
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(frontend_identity(&home, &executable).is_err());
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn current_overview_is_paired_record_readback_without_readiness() {
        let snapshot: Snapshot = serde_json::from_str(
            include_str!("../examples/library-preview.json")).unwrap();
        let mut value = serde_json::to_value(CurrentOverview {
            schema:1,operator_schema:crate::model::OPERATOR_SCHEMA,
            scope:"current_only".into(),current_generation:"exact-generation".into(),
            current:snapshot,
        }).unwrap();
        assert!(matches!(decode_reply(Query::CurrentOverview, &serde_json::to_vec(&value).unwrap()),
            Ok(Reply::CurrentOverview(_))));
        assert!(decode_reply(Query::Overview, &serde_json::to_vec(&value).unwrap()).is_err());
        value["readiness"] = serde_json::json!({"observed_at":1});
        assert!(decode_reply(Query::CurrentOverview, &serde_json::to_vec(&value).unwrap()).is_err());
        value.as_object_mut().unwrap().remove("readiness");
        value["operator_schema"] = (crate::model::OPERATOR_SCHEMA - 1).into();
        assert!(decode_reply(Query::CurrentOverview, &serde_json::to_vec(&value).unwrap()).is_err());
        value["operator_schema"] = crate::model::OPERATOR_SCHEMA.into();
        value["scope"] = "history".into();
        assert!(decode_reply(Query::CurrentOverview, &serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn selected_product_readback_requires_exact_paired_identity() {
        let snapshot: Snapshot = serde_json::from_str(
            include_str!("../examples/library-preview.json")).unwrap();
        let product = snapshot.products[0].clone();
        let key = ProductKey::from(&product);
        let mut value = serde_json::to_value(CurrentProductDetail {
            schema: 1, operator_schema: crate::model::OPERATOR_SCHEMA,
            state_token: snapshot.state_token, current_generation: "exact-generation".into(),
            system: snapshot.system,
            product, environments: vec![], vendor_applications: vec![],
        }).unwrap();
        assert!(matches!(decode_reply(Query::Product(key.clone()),
            &serde_json::to_vec(&value).unwrap()), Ok(Reply::Product(_))));
        value["product"]["class_id"] = "different".into();
        assert!(decode_reply(Query::Product(key.clone()),
            &serde_json::to_vec(&value).unwrap()).is_err());
        value["operator_schema"] = 11.into();
        assert!(decode_reply(Query::Product(key),
            &serde_json::to_vec(&value).unwrap()).is_err());
    }
    #[test]
    fn current_frontend_accepts_paired_and_refuses_old_manager() {
        let query = || {
            Query::Action(Request {
                schema: crate::model::OPERATOR_SCHEMA,
                state_token: "exact-state".into(),
                action: crate::model::Action::DependencyPrepare {},
            })
        };
        let mut receipt = serde_json::json!({
            "schema": crate::model::OPERATOR_SCHEMA, "accepted": true, "operation": "exact-operation", "refusal": null
        });
        match decode_reply(query(), &serde_json::to_vec(&receipt).unwrap()).unwrap() {
            Reply::Receipt(value) => {
                assert_eq!(value.schema, crate::model::OPERATOR_SCHEMA);
                assert!(value.accepted);
                assert_eq!(value.operation.as_deref(), Some("exact-operation"));
            }
            _ => panic!("expected dependency request receipt"),
        }
        let system = serde_json::json!({
            "service": "active", "keepers": 2, "dsp": 0, "maintenance": 0,
            "ceiling": 512, "pending_transactions": 0, "stale_transports": 0,
            "cleanup_unconfirmed": false
        });
        let activity = serde_json::json!({
            "schema": crate::model::OPERATOR_SCHEMA, "system": system, "capture": null, "operation": null
        });
        assert!(matches!(
            decode_reply(Query::Activity, &serde_json::to_vec(&activity).unwrap()),
            Ok(Reply::Activity(_))
        ));
        let pulse = serde_json::json!({"schema":crate::model::OPERATOR_SCHEMA,
            "service_state":"active","dsp":0,"keepers":2,"maintenance":0,
            "pending_transactions":0,"cleanup_unconfirmed":false,"current_generation":"exact-generation",
            "operation":null,"operation_live":false});
        assert!(matches!(decode_reply(Query::Pulse, &serde_json::to_vec(&pulse).unwrap()),
            Ok(Reply::Pulse(_))));
        let snapshot = serde_json::json!({
            "schema": crate::model::OPERATOR_SCHEMA, "state_token": "exact-state", "system": system,
            "capture": null, "operation": null, "onboarding": [], "environments": [],
            "vendor_applications": [], "products": [], "active_sessions": [],
            "recent_incidents": [], "actions": []
        });
        assert!(matches!(
            decode_reply(Query::Snapshot, &serde_json::to_vec(&snapshot).unwrap()),
            Ok(Reply::Snapshot(_))
        ));
        let Reply::Snapshot(decoded)=decode_reply(Query::Snapshot,&serde_json::to_vec(&snapshot).unwrap()).unwrap()
            else {panic!("expected snapshot")};
        assert!(decoded.managed_runtime_records.is_empty());
        let mut with_runtime=snapshot.clone();
        with_runtime["managed_runtime_records"]=serde_json::json!([{
            "id":"retained-runtime","record_path":"/managed/runners/retained/runtime.json",
            "runner":null,"failure":"managed_runtime_binding"}]);
        let Reply::Snapshot(decoded)=decode_reply(Query::Snapshot,&serde_json::to_vec(&with_runtime).unwrap()).unwrap()
            else {panic!("expected snapshot")};
        assert_eq!(decoded.managed_runtime_records[0]["failure"],"managed_runtime_binding");
        with_runtime["schema"]=20.into();
        assert!(decode_reply(Query::Snapshot,&serde_json::to_vec(&with_runtime).unwrap()).err().unwrap()
            .contains(&format!("operator model {} required",crate::model::OPERATOR_SCHEMA)));
        for schema in [
            serde_json::json!(6),
            serde_json::json!(7),
            serde_json::json!(8),
            serde_json::json!(9),
            serde_json::json!(10),
            serde_json::json!(11),
            serde_json::json!(12),
            serde_json::json!(14),
            serde_json::json!(20),
            serde_json::json!("9"),
            serde_json::Value::Null,
        ] {
            receipt["schema"] = schema;
            for request in [query(), Query::Snapshot, Query::Activity, Query::Pulse] {
                assert!(
                    decode_reply(request, &serde_json::to_vec(&receipt).unwrap())
                        .err()
                        .unwrap()
                        .contains(&format!("operator model {} required", crate::model::OPERATOR_SCHEMA))
                );
            }
        }
        receipt["schema"] = serde_json::json!(crate::model::OPERATOR_SCHEMA);
        receipt["arbitrary"] = serde_json::json!(true);
        assert!(decode_reply(query(), &serde_json::to_vec(&receipt).unwrap()).is_err());
    }
    #[test]
    fn paired_import_result_preserves_identity_and_refuses_old_manager() {
        let exact = serde_json::json!({"schema":2,"installer_sha256":"ab".repeat(32),
            "byte_size":2048,"format":"pe_executable","newly_imported":false,
            "display_label":"Lunacy Audio","created_at":42});
        match decode_import(&serde_json::to_vec(&exact).unwrap()).unwrap() {
            Reply::Imported(value) => {
                assert!(!value.newly_imported);
                assert_eq!(value.installer_sha256, "ab".repeat(32));
            }
            _ => panic!("expected typed import"),
        }
        let mut old = exact;
        old["schema"] = 1.into();
        assert!(decode_import(&serde_json::to_vec(&old).unwrap()).err().unwrap()
            .contains("Update the frontend and manager together"));
    }

    #[test]
    fn selected_descriptor_refuses_links_and_directories() {
        let root = std::env::temp_dir().join(format!("mf2-picker-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let p = root.join("selected.exe");
        std::fs::write(&p, b"bytes").unwrap();
        assert!(open_selected(&p).is_ok());
        let l = root.join("link");
        std::os::unix::fs::symlink(&p, &l).unwrap();
        assert!(open_selected(&l).is_err());
        assert!(open_selected(&root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}

//! One exact Native Access dependency; no caller-selectable service or executable.
use crate::{catalogue::Software, renderer_application as app, *};
use serde_json::{json, Value};
pub const INSTALLER: &str="Program Files/Native Instruments/Native Access/resources/daemon/win/NTKDaemon 1.32.0 Setup PC.exe";
pub const INSTALLER_SHA: &str = "5f2199f4e1409d6eea5edaea9c4a8af31e8ee8ac3790851aa44d33e87a46b218";
pub const DAEMON: &str = "Program Files/Common Files/Native Instruments/NTK/NTKDaemon.exe";
pub const OBSERVATION: &[u8] =
    include_bytes!("../../evidence/nad1/observation/repaired/observation.json");
pub const SERVICE: &str = "NTKDaemonService";
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Absent,
    Unregistered,
    Stopped,
    RunningNotReady,
    ForeignConflict,
    Ready,
    Unresolved,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Install,
    StartService,
    VerifiedNoop,
    Refuse,
}
pub fn transition(state: State) -> Transition {
    match state {
        State::Absent | State::Unregistered => Transition::Install,
        State::Stopped => Transition::StartService,
        State::Ready => Transition::VerifiedNoop,
        _ => Transition::Refuse,
    }
}
pub fn installer(application: &app::Application) -> Result<app::Image> {
    let p = application
        .environment
        .root
        .join("compatdata/pfx/drive_c")
        .join(INSTALLER);
    let value = app::Image {
        artifact: Artifact {
            path: p,
            sha256: INSTALLER_SHA.into(),
        },
        size: 35769456,
    };
    app::verify_image(&value)?;
    Ok(value)
}
pub fn bind(
    application: &app::Application,
    software: &Software,
    op: &str,
    report: &Path,
) -> Result<Value> {
    installer(application)?;
    // Recover only the qualified bundled image from the retained failed attempt.
    // No caller-selected image or silent reinstall; Python repeats admission.
    let dependency = report.parent().and_then(Path::parent).and_then(Path::parent)
        .ok_or("dependency_report_location")?;
    let daemon = application.environment.root.join("compatdata/pfx/drive_c").join(DAEMON);
    if daemon.symlink_metadata().is_ok() && dependency.join("artifact.json").symlink_metadata().is_err() {
        verify_recovery(application, dependency)?;
    }
    let mut v = app::bind(
        application,
        software,
        op,
        app::RendererPolicy::SoftwareRendering,
        report,
    )?;
    v["kind"] = json!("native_access_dependency");
    Ok(v)
}
pub const RECOVERY: &[u8] = include_bytes!("native_access_recovery.json");
fn verify_recovery(application: &app::Application, directory: &Path) -> Result<()> {
    let q: Value = serde_json::from_slice(RECOVERY)?;
    require(q["application_identity"] == application.identity()?, "dependency_recovery_application")?;
    verify_recovery_files(&application.environment.root.join("compatdata/pfx/drive_c"), directory, &q)
}
fn verify_recovery_files(drive: &Path, directory: &Path, q: &Value) -> Result<()> {
    let prior = directory.join("operations").join(q["operation"].as_str().ok_or("dependency_recovery_operation")?);
    for (name, witness) in q["sources"].as_object().ok_or("dependency_recovery_sources")? {
        app::verify_image(&app::Image { artifact: Artifact { path: prior.join(name),
            sha256: witness["sha256"].as_str().ok_or("dependency_recovery_digest")?.into() },
            size: witness["size"].as_u64().ok_or("dependency_recovery_size")? })?;
    }
    app::verify_image(&app::Image { artifact: Artifact {
        path: drive.join(DAEMON),
        sha256: q["daemon"]["sha256"].as_str().ok_or("dependency_recovery_digest")?.into() },
        size: q["daemon"]["size"].as_u64().ok_or("dependency_recovery_size")? })
}
pub fn record_path(m: &Manager) -> PathBuf {
    m.root
        .join("vendor-applications/native-access-dependency/prepared.json")
}
/// A preparation receipt is historical qualification, never a claim of live readiness.
/// Each application launch must freshly start/query SCM and bind owned listeners.
pub fn prepared(m: &Manager, application: &app::Application, software: &Software) -> Result<Value> {
    let v: Value = read_json(&record_path(m))?;
    require(
        v.as_object().is_some_and(|o| o.len() == 7)
            && v["schema"] == 1
            && v["application"] == application.identity()?
            && v["software_sha256"]
                == hex(&Sha256::digest(serde_json::to_vec(&serde_json::to_value(
                    software,
                )?)?))
            && v["installer_sha256"] == INSTALLER_SHA,
        "dependency_prepared_identity",
    )?;
    let op = v["operation"]
        .as_str()
        .ok_or("dependency_prepared_operation")?;
    require(valid_hex(op, 32), "dependency_prepared_operation")?;
    let receipt = crate::dependency_session::operation_dir(m, op)?.join("result.json");
    require(
        digest(&receipt)?
            == v["result_sha256"]
                .as_str()
                .ok_or("dependency_result_digest")?,
        "dependency_receipt_changed",
    )?;
    let result: Value = read_json(&receipt)?;
    require(
        result["dependency"]["ready_tested"] == true
            && result["dependency"]["service_retirement_confirmed"] == true
            && result["dependency"]["process_cleanup_confirmed"] == true
            && result["dependency"]["forced_cleanup_used"] == false
            && result["dependency"]["daemon"] == v["daemon"]
            && result["cleanup_confirmed"] == true
            && result["owned_live"] == 0
            && result["state"] == "completed",
        "dependency_preparation_incomplete",
    )?;
    let image: &Value = &v["daemon"];
    let sha = image["sha256"].as_str().ok_or("dependency_daemon_digest")?;
    let size = image["size"].as_u64().ok_or("dependency_daemon_size")?;
    require(valid_hex(sha, 64), "dependency_daemon_digest")?;
    app::verify_image(&app::Image {
        artifact: Artifact {
            path: application
                .environment
                .root
                .join("compatdata/pfx/drive_c")
                .join(DAEMON),
            sha256: sha.into(),
        },
        size,
    })?;
    installer(application)?;
    Ok(v)
}
pub fn retain_prepared(
    m: &Manager,
    application: &app::Application,
    software: &Software,
    op: &str,
) -> Result<()> {
    let path = crate::dependency_session::operation_dir(m, op)?.join("result.json");
    let r: Value = read_json(&path)?;
    require(
        r["operation"] == op
            && r["state"] == "completed"
            && r["cleanup_confirmed"] == true
            && r["owned_live"] == 0
            && r["dependency"]["ready_tested"] == true
            && r["dependency"]["service_retirement_confirmed"] == true
            && r["dependency"]["process_cleanup_confirmed"] == true
            && r["dependency"]["forced_cleanup_used"] == false,
        "dependency_qualification_failed",
    )?;
    let record = json!({"schema":1,"operation":op,"application":application.identity()?,"software_sha256":hex(&Sha256::digest(serde_json::to_vec(&serde_json::to_value(software)?)?)),"installer_sha256":INSTALLER_SHA,"daemon":r["dependency"]["daemon"],"result_sha256":digest(&path)?});
    let daemon=&r["dependency"]["daemon"];
    app::verify_image(&app::Image {artifact:Artifact {path:application.environment.root.join("compatdata/pfx/drive_c").join(DAEMON),sha256:daemon["sha256"].as_str().ok_or("dependency_daemon_digest")?.into()},size:daemon["size"].as_u64().ok_or("dependency_daemon_size")?})?;
    installer(application)?;
    // Operation receipts remain immutable; this pointer selects the verified preparation.
    atomic_json(&record_path(m), &record)?;
    prepared(m, application, software)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_state_law() {
        for (s, t) in [
            (State::Absent, Transition::Install),
            (State::Unregistered, Transition::Install),
            (State::Stopped, Transition::StartService),
            (State::Ready, Transition::VerifiedNoop),
            (State::RunningNotReady, Transition::Refuse),
            (State::ForeignConflict, Transition::Refuse),
            (State::Unresolved, Transition::Refuse),
        ] {
            assert_eq!(transition(s), t);
        }
        for v in ["reinstall", "kill", "direct", "arbitrary"] {
            assert!(serde_json::from_value::<State>(json!(v)).is_err());
        }
    }
    #[test]
    fn accepted_observation_is_absence() {
        let v: Value = serde_json::from_slice(OBSERVATION).unwrap();
        assert_eq!(v["disposition"], "NAD1_DEPENDENCY_ABSENT");
        assert_eq!(v["process_census"]["unavailable"], 0);
        assert_eq!(v["bundle"][0]["sha256"], INSTALLER_SHA);
    }
}

#[cfg(test)]
mod operator_boundary_tests {
    use crate::operator_model::Action;
    use serde_json::json;
    #[test]
    fn closed_prepare_never_admits_operator_parameters() {
        assert!(serde_json::from_value::<Action>(json!({"kind":"dependency_prepare"})).is_ok());
        for field in ["path","service","port","pid","prefix","command","args","environment","test_mode"] {
            let mut v=json!({"kind":"dependency_prepare"});v[field]=json!("arbitrary");
            assert!(serde_json::from_value::<Action>(v).is_err());
        }
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[test]
    fn recovery_checks_all_historical_sources_and_payload_without_writing_them() {
        let base=std::env::temp_dir().join(format!("nad1-recovery-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&base).unwrap();let base=base.canonicalize().unwrap();
        let op="a".repeat(32);let prior=base.join("operations").join(&op);
        fs::create_dir_all(&prior).unwrap();let daemon=base.join(DAEMON);fs::create_dir_all(daemon.parent().unwrap()).unwrap();
        let bytes=b"source-owned exact bytes";fs::write(&daemon,bytes).unwrap();
        let witness=json!({"sha256":hex(&Sha256::digest(bytes)),"size":bytes.len()});
        let mut q=json!({"operation":op,"sources":{},"daemon":witness});
        let mut paths=vec![daemon];
        for name in ["spec.json","result.json","installer.log"] {
            let p=prior.join(name);fs::write(&p,bytes).unwrap();paths.push(p);
            q["sources"][name]=witness.clone();
        }
        verify_recovery_files(&base,&base,&q).unwrap();
        for p in &paths {
            fs::write(p,b"changed").unwrap();assert!(verify_recovery_files(&base,&base,&q).is_err());
            fs::remove_file(p).unwrap();assert!(verify_recovery_files(&base,&base,&q).is_err());
            fs::write(p,bytes).unwrap();
        }
        verify_recovery_files(&base,&base,&q).unwrap();
        for p in paths {assert_eq!(fs::read(p).unwrap(),bytes);}
        fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn recovery_qualification_is_the_observed_bundle_member_not_filename_authority() {
        let q:Value=serde_json::from_slice(RECOVERY).unwrap();
        let observed:Value=serde_json::from_slice(include_bytes!("../../evidence/nad1/real-preparation/attempt-1/offline-attribution.json")).unwrap();
        assert_eq!(q["daemon"]["sha256"],observed["bundled_daemon_comparison"]["sha256"]);
        assert_eq!(q["daemon"]["size"],observed["bundled_daemon_comparison"]["size"]);
        assert_eq!(q["sources"]["result.json"]["sha256"],observed["original_result_sha256"]);
        assert_eq!(q["operation"],observed["operation"]);
        assert_eq!(q["installer_sha256"],INSTALLER_SHA);
    }
}

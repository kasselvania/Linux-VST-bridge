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
            && r["dependency"]["ready_tested"] == true,
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

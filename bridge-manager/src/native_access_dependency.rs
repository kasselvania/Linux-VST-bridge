//! One exact Native Access dependency; no caller-selectable service or executable.
use crate::{catalogue::Software, renderer_application as app, *};
use serde_json::{json, Value};
pub const INSTALLER: &str="Program Files/Native Instruments/Native Access/resources/daemon/win/NTKDaemon 1.32.0 Setup PC.exe";
pub const INSTALLER_SHA: &str = "5f2199f4e1409d6eea5edaea9c4a8af31e8ee8ac3790851aa44d33e87a46b218";
pub const DAEMON: &str = "Program Files/Common Files/Native Instruments/NTK/NTKDaemon.exe";
pub const OBSERVATION: &[u8] =
    include_bytes!("../../evidence/nad1/observation/repaired/observation.json");
pub const SERVICE: &str = "NTKDaemonService";
pub const LISTENERS: [u16; 2] = [5146, 5563];
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
    let recover = daemon.symlink_metadata().is_ok() && dependency.join("artifact.json").symlink_metadata().is_err();
    if recover {
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
    v["schema"] = json!(2);
    v["dependency_mode"] = json!(if recover { "recover_installed" } else { "prepare" });
    Ok(v)
}
pub const RECOVERY: &[u8] = include_bytes!("native_access_recovery.json");
fn verify_recovery(application: &app::Application, directory: &Path) -> Result<()> {
    let q: Value = serde_json::from_slice(RECOVERY)?;
    verify_recovery_inputs(application,directory,&q,INSTALLER,DAEMON).map(|_|())
}
fn safe_relative(path:&str)->Result<()> {
    let p=Path::new(path);
    require(!p.is_absolute() && p.components().all(|c|matches!(c,std::path::Component::Normal(_))),
        "dependency_recovery_relative_path")
}
fn recovery_image(drive:&Path,relative:&str,witness:&Value)->Result<()> {
    safe_relative(relative)?;
    app::verify_image(&app::Image { artifact: Artifact { path: drive.join(relative),
        sha256: witness["sha256"].as_str().ok_or("dependency_recovery_digest")?.into() },
        size: witness["size"].as_u64().ok_or("dependency_recovery_size")? })
}
fn verify_recovery_inputs(
    application:&app::Application,
    directory:&Path,
    q:&Value,
    installer_relative:&str,
    daemon_relative:&str,
)->Result<Value> {
    require(q.as_object().is_some_and(|o|o.len()==7) && q["schema"]==1
        && q["application_identity"]==application.identity()?,"dependency_recovery_application")?;
    let operation=q["operation"].as_str().ok_or("dependency_recovery_operation")?;
    require(valid_hex(operation,32),"dependency_recovery_operation")?;
    let sources=q["sources"].as_object().ok_or("dependency_recovery_sources")?;
    require(sources.len()==3 && sources.contains_key("spec.json") && sources.contains_key("result.json")
        && sources.contains_key(&format!("{operation}-dependency-1.log")),"dependency_recovery_sources")?;
    let prior=directory.join("operations").join(operation);let mut records=serde_json::Map::new();
    for (name,witness) in sources {
        require(Path::new(name).file_name().is_some_and(|v|v==name.as_str()),"dependency_recovery_source")?;
        let image=app::Image{artifact:Artifact{path:prior.join(name),
            sha256:witness["sha256"].as_str().ok_or("dependency_recovery_digest")?.into()},
            size:witness["size"].as_u64().ok_or("dependency_recovery_size")?};
        app::verify_image(&image)?;
        if name=="spec.json" || name=="result.json" {records.insert(name.clone(),exact_record(&image.artifact.path,"dependency_recovery_source_alias")?);}
    }
    let spec=&records["spec.json"];
    require(spec["operation"]==operation && spec["kind"]=="native_access_dependency"
        && spec["application"]==serde_json::to_value(application)?
        && spec["application_identity"]==q["application_identity"],"dependency_recovery_origin")?;
    let result=&records["result.json"];
    require(result["operation"]==operation && result["state"]=="failed"
        && result["cleanup_confirmed"]==true && result["owned_live"]==0
        && result["dependency"]["service_retirement_confirmed"]==true
        && result["dependency"]["process_cleanup_confirmed"]==true
        && result["dependency"]["forced_cleanup_used"]==false,"dependency_recovery_prior_not_retired")?;
    let drive=application.environment.root.join("compatdata/pfx/drive_c");
    recovery_image(&drive,installer_relative,&json!({"sha256":q["installer_sha256"],"size":q["installer_size"]}))?;
    require(q["daemon"].as_object().is_some_and(|o|o.len()==3) && q["daemon"]["architecture"]=="x64",
        "dependency_recovery_daemon")?;
    recovery_image(&drive,daemon_relative,&q["daemon"])?;
    Ok(json!({"schema":1,"authority":"qualified_bundle_payload_and_retired_installation",
        "prior_operation":operation,"prior_result_sha256":q["sources"]["result.json"]["sha256"],
        "qualification_sha256":hex(&Sha256::digest(serde_json::to_vec(q)?)),
        "daemon":q["daemon"],"installer_reexecuted":false,"prior_failure_preserved":true}))
}
fn exact_record<T: serde::de::DeserializeOwned>(path: &Path, why: &str) -> Result<T> {
    let md=fs::symlink_metadata(path)?;
    require(md.file_type().is_file() && md.nlink()==1 && path.canonicalize()?==path,why)?;
    read_json(path)
}
fn software_sha256(software: &Software) -> Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(&serde_json::to_value(software)?)?)))
}
fn metadata_stamp(md:&fs::Metadata)->(u64,u64,u64,i64,i64,i64,i64) {
    (md.dev(),md.ino(),md.len(),md.mtime(),md.mtime_nsec(),md.ctime(),md.ctime_nsec())
}
fn pointer_present(path:&Path)->Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_)=>Ok(true),
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(false),
        Err(e)=>Err(e.into()),
    }
}
fn common_session(
    application: &app::Application,
    software: &Software,
    installer:&Value,
    daemon:&Value,
    origin:Value,
) -> Result<Value> {
    let prefix=application.environment.root.join("compatdata/pfx");
    let prefix_md=fs::symlink_metadata(&prefix)?;
    require(prefix_md.is_dir() && prefix.canonicalize()? == prefix,
        "dependency_session_prefix_identity")?;
    let current=serde_json::to_value(software)?;
    let current_sha=software_sha256(software)?;
    Ok(json!({"schema":1,"authority":"closed_exact_installation_origin_and_current_software",
        "application":application.identity()?,"software_sha256":current_sha,
        "manager_sha256":software.manager.sha256,"environment":application.environment.id,
        "prefix":{"path_sha256":hex(&Sha256::digest(prefix.as_os_str().as_encoded_bytes())),
            "device":prefix_md.dev(),"inode":prefix_md.ino()},
        "installation":application.installation,"service":SERVICE,"listeners":LISTENERS,
        "installer":installer,"daemon":daemon,"origin":origin,"current_software":current}))
}
fn artifact_session_origin(
    application:&app::Application,
    software:&Software,
    directory:&Path,
    qualified:&Value,
)->Result<Value> {
    let pointer=directory.join("artifact.json");
    let record:Value=exact_record(&pointer,"dependency_session_artifact_alias")?;
    require(record.as_object().is_some_and(|o|o.len()==8)
        && record["schema"]==1
        && record["application"]==application.identity()?
        && record["installer_sha256"]==qualified["installer_sha256"],
        "dependency_session_artifact_identity")?;
    let operation=record["operation"].as_str().ok_or("dependency_session_artifact_operation")?;
    require(valid_hex(operation,32),"dependency_session_artifact_operation")?;
    let operation_dir=directory.join("operations").join(operation);
    let retained:Value=exact_record(&operation_dir.join("artifact.json"),"dependency_session_artifact_alias")?;
    require(retained==record,"dependency_session_artifact_pointer")?;
    let spec_path=operation_dir.join("spec.json");
    require(digest(&spec_path)?==record["spec_sha256"].as_str().ok_or("dependency_session_spec_digest")?,
        "dependency_session_spec_changed")?;
    let prior:Value=exact_record(&spec_path,"dependency_session_spec_alias")?;
    let prior_software=prior["software"].clone();
    let prior_software_sha=hex(&Sha256::digest(serde_json::to_vec(&prior_software)?));
    require(prior["schema"]==2 && prior["kind"]=="native_access_dependency"
        && prior["operation"]==operation && prior["application"]==serde_json::to_value(application)?
        && prior["application_identity"]==record["application"]
        && prior["software_sha256"]==record["software_sha256"]
        && prior_software_sha==record["software_sha256"],
        "dependency_session_artifact_origin")?;
    let root=operation_dir.join(format!("{operation}-installer-root.private.json"));
    require(digest(&root)?==record["root_sha256"].as_str().ok_or("dependency_session_root_digest")?,
        "dependency_session_root_changed")?;
    let root_record:Value=exact_record(&root,"dependency_session_root_alias")?;
    let frame=root_record["frame"].as_array().ok_or("dependency_session_root_frame")?;
    let installer_size=qualified["installer_size"].as_u64()
        .ok_or("dependency_session_installer_size")?;
    let installer_size_text=installer_size.to_string();
    require(frame.len()==7 && frame[0]=="NAD1_INSTALL_ROOT_V1" && frame[1]==operation
        && frame[3]==qualified["installer_sha256"]
        && frame[4].as_str()==Some(installer_size_text.as_str()),
        "dependency_session_root_frame")?;
    let origin=json!({"kind":"retained_installation_artifact","operation":operation,
            "artifact_sha256":digest(&pointer)?,
            "spec_sha256":record["spec_sha256"],"root_sha256":record["root_sha256"],
            "software_sha256":record["software_sha256"]});
    require(exact_record::<Value>(&pointer,"dependency_session_artifact_alias")?==record,
        "dependency_session_artifact_changed")?;
    let installer=json!({"sha256":qualified["installer_sha256"],"size":qualified["installer_size"]});
    common_session(application,software,&installer,&record["daemon"],origin)
}
fn recovery_session_origin(
    application:&app::Application,
    software:&Software,
    qualified:&Value,
    recovery:&Value,
)->Result<Value> {
    let origin=json!({"kind":"qualified_recovered_installation",
        "operation":qualified["operation"],"application_identity":qualified["application_identity"],
        "installer_sha256":qualified["installer_sha256"],"sources":qualified["sources"],
        "qualification_sha256":recovery["qualification_sha256"]});
    let installer=json!({"sha256":qualified["installer_sha256"],"size":qualified["installer_size"]});
    common_session(application,software,&installer,&qualified["daemon"],origin)
}
fn session_admission_inputs_with<F>(
    application:&app::Application,
    software:&Software,
    directory:&Path,
    qualified:&Value,
    verify:F,
)->Result<Value>
where F:FnOnce(bool)->Result<Option<Value>> {
    require(directory.canonicalize()?==directory,"dependency_session_directory_alias")?;
    let before=fs::symlink_metadata(directory)?;
    require(before.is_dir(),"dependency_session_directory")?;
    let pointer=directory.join("artifact.json");let had_pointer=pointer_present(&pointer)?;
    let checked=verify(!had_pointer)?;
    let result=if had_pointer {
        require(checked.is_none(),"dependency_session_origin_ambiguous")?;
        artifact_session_origin(application,software,directory,qualified)?
    } else {
        let recovery=checked.ok_or("dependency_session_recovery_missing")?;
        require(recovery["daemon"]==qualified["daemon"],"dependency_session_daemon_origin")?;
        recovery_session_origin(application,software,qualified,&recovery)?
    };
    require(pointer_present(&pointer)?==had_pointer
        && metadata_stamp(&before)==metadata_stamp(&fs::symlink_metadata(directory)?),
        "dependency_session_origin_changed")?;
    Ok(result)
}
/// Shared exact-input law. Production supplies only the compiled recovery
/// qualification and fixed image paths; sealed source-owned fixtures use the
/// same validator without adding an operator-selectable launch surface.
pub fn session_admission_inputs(
    application:&app::Application,
    software:&Software,
    directory:&Path,
    qualified:&Value,
    installer_relative:&str,
    daemon_relative:&str,
)->Result<Value> {
    let drive=application.environment.root.join("compatdata/pfx/drive_c");
    recovery_image(&drive,installer_relative,&json!({"sha256":qualified["installer_sha256"],"size":qualified["installer_size"]}))?;
    let result=session_admission_inputs_with(application,software,directory,qualified,
        |recovery|if recovery {Ok(Some(verify_recovery_inputs(application,directory,qualified,installer_relative,daemon_relative)?))}
            else {Ok(None)})?;
    recovery_image(&drive,daemon_relative,&result["daemon"])?;Ok(result)
}
/// Closed application-session admission. Historical installation origin and the
/// current immutable manager generation are separate facts; neither implies
/// readiness or successful service retirement.
pub fn session_admission(
    m: &Manager,
    application: &app::Application,
    software: &Software,
) -> Result<Value> {
    application.verify(&m.root)?;
    installer(application)?;
    require(read_json::<Value>(&m.root.join("software.json"))?==serde_json::to_value(software)?,
        "dependency_session_current_software")?;
    let directory=m.root.join("vendor-applications/native-access-dependency");
    let qualified:Value=serde_json::from_slice(RECOVERY)?;
    require(qualified["application_identity"]==application.identity()?
        && qualified["installer_sha256"]==INSTALLER_SHA && qualified["installer_size"]==35769456_u64,
        "dependency_session_recovery_qualification")?;
    let admission=session_admission_inputs(application,software,&directory,&qualified,INSTALLER,DAEMON)?;
    let daemon=&admission["daemon"];
    app::verify_image(&app::Image{artifact:Artifact{
        path:application.environment.root.join("compatdata/pfx/drive_c").join(DAEMON),
        sha256:daemon["sha256"].as_str().ok_or("dependency_session_daemon_digest")?.into()},
        size:daemon["size"].as_u64().ok_or("dependency_session_daemon_size")?})?;
    Ok(admission)
}
pub fn bind_session(
    m:&Manager,
    application:&app::Application,
    software:&Software,
    op:&str,
    policy:app::RendererPolicy,
    report:&Path,
) -> Result<Value> {
    let admission=session_admission(m,application,software)?;
    let mut spec=app::bind(application,software,op,policy,report)?;
    spec["dependency_session"]=admission;
    crate::kontakt8::bind(&mut spec,application,software)?;
    Ok(spec)
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

#[cfg(test)]
mod session_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn put(path:&Path,value:&Value) {
        if let Some(parent)=path.parent(){fs::create_dir_all(parent).unwrap();}
        atomic_json(path,value).unwrap();
    }
    struct Inputs {
        _fixture:crate::test_fixture::Fixture,
        application:app::Application,
        software:Software,
        directory:PathBuf,
        qualified:Value,
        installer:&'static str,
        daemon:&'static str,
    }
    fn witness(path:&Path)->Value {json!({"sha256":digest(path).unwrap(),"size":fs::metadata(path).unwrap().len()})}
    fn inputs()->Inputs {
        let fixture=crate::test_fixture::Fixture::new();
        let image=app::Image{artifact:fixture.r.module.clone(),size:fs::metadata(&fixture.r.module.path).unwrap().len()};
        let application=app::Application{schema:1,id:app::ID.into(),environment:fixture.r.environment.clone(),
            files:BTreeMap::from([("Native Access.exe".into(),image)]),installation:fixture.r.module.clone(),
            observation_sha256:"1".repeat(64),source_seal_sha256:"2".repeat(64)};
        let artifact=fixture.r.host.clone();
        let software:Software=serde_json::from_value(json!({"installer_launch":artifact,"manager":artifact,
            "supervisor":artifact,"ownership":artifact,"host":artifact,"source_manifest":artifact,
            "source_sha256":"3".repeat(64)})).unwrap();
        let directory=fixture.outer.join("dependency");fs::create_dir_all(&directory).unwrap();let directory=directory.canonicalize().unwrap();
        let installer="NAD1Fixture/Setup.exe";let daemon="NAD1Fixture/NTKDaemon.exe";
        let drive=application.environment.root.join("compatdata/pfx/drive_c");
        fs::create_dir_all(drive.join("NAD1Fixture")).unwrap();
        fs::write(drive.join(installer),b"source-owned installer").unwrap();
        fs::write(drive.join(daemon),b"source-owned daemon").unwrap();
        let recovery_operation="cd".repeat(16);let recovery=directory.join("operations").join(&recovery_operation);
        fs::create_dir_all(&recovery).unwrap();
        let prior=json!({"schema":2,"kind":"native_access_dependency","operation":recovery_operation,
            "application":application,"application_identity":application.identity().unwrap(),"software":software});
        put(&recovery.join("spec.json"),&prior);
        put(&recovery.join("result.json"),&json!({"operation":recovery_operation,"state":"failed",
            "cleanup_confirmed":true,"owned_live":0,"dependency":{"service_retirement_confirmed":true,
            "process_cleanup_confirmed":true,"forced_cleanup_used":false}}));
        let log=recovery.join(format!("{recovery_operation}-dependency-1.log"));fs::write(&log,b"source-owned retained failure").unwrap();
        let qualified=json!({"schema":1,"operation":recovery_operation,
            "application_identity":application.identity().unwrap(),
            "installer_sha256":digest(&drive.join(installer)).unwrap(),"installer_size":fs::metadata(drive.join(installer)).unwrap().len(),
            "daemon":{"sha256":digest(&drive.join(daemon)).unwrap(),"size":fs::metadata(drive.join(daemon)).unwrap().len(),"architecture":"x64"},
            "sources":{"spec.json":witness(&recovery.join("spec.json")),"result.json":witness(&recovery.join("result.json")),
                format!("{recovery_operation}-dependency-1.log"):witness(&log)}});
        Inputs{_fixture:fixture,application,software,directory,qualified,installer,daemon}
    }
    fn write_artifact(i:&Inputs)->Value {
        let operation="ab".repeat(16);let op=i.directory.join("operations").join(&operation);fs::create_dir_all(&op).unwrap();
        let historical=json!({"manager":{"path":"/historical/manager","sha256":"4".repeat(64)}});
        let historical_sha=hex(&Sha256::digest(serde_json::to_vec(&historical).unwrap()));
        let prior=json!({"schema":2,"kind":"native_access_dependency","operation":operation,
            "application":i.application,"application_identity":i.application.identity().unwrap(),
            "software":historical,"software_sha256":historical_sha});
        put(&op.join("spec.json"),&prior);let spec_sha=digest(&op.join("spec.json")).unwrap();
        let root=json!({"frame":["NAD1_INSTALL_ROOT_V1",operation,"5".repeat(64),i.qualified["installer_sha256"],i.qualified["installer_size"].as_u64().unwrap().to_string(),"10","20"]});
        let root_path=op.join(format!("{operation}-installer-root.private.json"));put(&root_path,&root);let root_sha=digest(&root_path).unwrap();
        let record=json!({"schema":1,"operation":operation,"application":i.application.identity().unwrap(),
            "software_sha256":historical_sha,"installer_sha256":i.qualified["installer_sha256"],"daemon":i.qualified["daemon"],
            "root_sha256":root_sha,"spec_sha256":spec_sha});
        put(&op.join("artifact.json"),&record);put(&i.directory.join("artifact.json"),&record);record
    }
    #[test]
    fn artifact_origin_binds_historical_artifact_and_current_software_separately() {
        let i=inputs();let record=write_artifact(&i);
        let admission=session_admission_inputs(&i.application,&i.software,&i.directory,&i.qualified,i.installer,i.daemon).unwrap();
        assert_eq!(admission["origin"]["kind"],"retained_installation_artifact");
        let historical_sha=record["software_sha256"].clone();
        assert_eq!(admission["origin"]["software_sha256"],historical_sha);
        assert_eq!(admission["software_sha256"],software_sha256(&i.software).unwrap());
        assert_ne!(admission["origin"]["software_sha256"],admission["software_sha256"]);
        assert_eq!(admission["service"],SERVICE);assert_eq!(admission["listeners"],json!(LISTENERS));
        assert!(!i.directory.join("prepared.json").exists());
        for (field,value) in [("application",json!("7".repeat(64))),("installer_sha256",json!("8".repeat(64))),
            ("daemon",json!({"sha256":"9".repeat(64),"size":1,"architecture":"x64"}))] {
            let mut bad=record.clone();bad[field]=value;fs::remove_file(i.directory.join("artifact.json")).unwrap();put(&i.directory.join("artifact.json"),&bad);
            assert!(session_admission_inputs(&i.application,&i.software,&i.directory,&i.qualified,i.installer,i.daemon).is_err());
            fs::remove_file(i.directory.join("artifact.json")).unwrap();put(&i.directory.join("artifact.json"),&record);
        }
    }
    #[test]
    fn qualified_recovery_origin_admits_only_with_absent_artifact_and_exact_sources() {
        let i=inputs();let admission=session_admission_inputs(&i.application,&i.software,&i.directory,&i.qualified,i.installer,i.daemon).unwrap();
        assert_eq!(admission["origin"]["kind"],"qualified_recovered_installation");
        assert_eq!(admission["origin"]["operation"],i.qualified["operation"]);
        assert_eq!(admission["origin"]["sources"],i.qualified["sources"]);
        assert!(!i.directory.join("artifact.json").exists());assert!(!i.directory.join("prepared.json").exists());
        let recovery=i.directory.join("operations").join(i.qualified["operation"].as_str().unwrap());
        for name in i.qualified["sources"].as_object().unwrap().keys() {
            let path=recovery.join(name);let bytes=fs::read(&path).unwrap();fs::write(&path,b"changed").unwrap();
            assert!(session_admission_inputs(&i.application,&i.software,&i.directory,&i.qualified,i.installer,i.daemon).is_err());fs::write(path,bytes).unwrap();
        }
        let daemon=i.application.environment.root.join("compatdata/pfx/drive_c").join(i.daemon);
        let bytes=fs::read(&daemon).unwrap();fs::write(&daemon,b"changed").unwrap();
        assert!(session_admission_inputs(&i.application,&i.software,&i.directory,&i.qualified,i.installer,i.daemon).is_err());fs::write(daemon,bytes).unwrap();
    }
    #[test]
    fn invalid_present_artifact_and_pointer_races_never_fall_back_to_recovery() {
        let i=inputs();put(&i.directory.join("artifact.json"),&json!({"schema":1}));
        assert!(session_admission_inputs(&i.application,&i.software,&i.directory,&i.qualified,i.installer,i.daemon).is_err());
        fs::remove_file(i.directory.join("artifact.json")).unwrap();
        let pointer=i.directory.join("artifact.json");
        assert!(session_admission_inputs_with(&i.application,&i.software,&i.directory,&i.qualified,|recovery|{
            assert!(recovery);put(&pointer,&json!({"appeared":true}));Ok(Some(json!({"daemon":i.qualified["daemon"]})))
        }).is_err());
        fs::remove_file(&pointer).unwrap();let _=write_artifact(&i);
        assert!(session_admission_inputs_with(&i.application,&i.software,&i.directory,&i.qualified,|recovery|{
            assert!(!recovery);fs::remove_file(&pointer).unwrap();Ok(None)
        }).is_err());
    }
}

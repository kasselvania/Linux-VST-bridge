use super::*;
pub(super) mod selection;
use serde_json::json;
use linux_vst_bridge::vendor_application::{
    self, Application, ApplicationId, LaunchMode, OperationResult,
};

pub fn run(m: &Manager, args: &[String]) -> Result<()> {
    run_selected(m, args, None)
}
pub(super) fn launch_owned(m: &Manager, application: &str, operation: &str) -> Result<()> {
    run_selected(m, &["launch".into(), application.into()], Some(operation))
}
pub(super) fn cancel_owned(m: &Manager, application: &str, operation: &str) -> Result<()> {
    run_selected(m, &["cancel".into(), application.into()], Some(operation))
}
fn current_application(m: &Manager, record: &Path) -> Result<Application> {
    let mut app:Application=read_json(record)?;
    app.environment=operator_cli::environment_install::current_environment(m,&app.environment)?;
    Ok(app)
}
pub(super) fn directory(m: &Manager, application: &str) -> Result<PathBuf> {
    ApplicationId::parse(application)?;
    Ok(m.root.join("vendor-applications").join(application))
}
pub(super) fn unit(application: &str) -> Result<String> {
    ApplicationId::parse(application)?;
    Ok(format!("linux-vst-bridge-vendor-{application}.service"))
}
pub(super) fn application(m: &Manager, selector: &str) -> Result<Application> {
    let path = directory(m, selector)?.join("application.json");
    let app = if path.exists() { current_application(m, &path)? } else {
        selection::observed(m, selector)?.ok_or("vendor_application_choice_unavailable")?.application
    };
    require(
        app.selector()? == selector,
        "vendor_application_selection_changed",
    )?;
    Ok(app)
}
pub(super) fn selectors(m: &Manager) -> Result<Vec<String>> {
    let root = m.root.join("vendor-applications");
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut selected = vec![];
    for (n, entry) in fs::read_dir(root)?.enumerate() {
        require(n < 128, "vendor_application_count_bound")?;
        let entry = entry?;
        let selector = entry.file_name().to_string_lossy().into_owned();
        if ApplicationId::parse(&selector).is_ok() && entry.path().join("application.json").exists()
        {
            require(entry.file_type()?.is_dir(), "vendor_application_directory")?;
            selected.push(selector);
        }
    }
    selected.sort();
    Ok(selected)
}
pub(super) fn live(selector: &str) -> Result<bool> {
    operator_cli::vendor_state_live(&readiness::bounded_user_unit_state(&unit(selector)?)?)
}
pub(super) fn result(m: &Manager, selector: &str, operation: &str) -> Result<OperationResult> {
    let directory = directory(m, selector)?;
    let job: vendor_application::Operation = read_json(&directory.join("operation.json"))?;
    let selected=application(m,selector)?;
    require(environment_revision::resolves_to(m,&job.application.environment,&selected.environment)?,
        "vendor_application_operation_environment_changed")?;
    let mut bound=job.application.clone();bound.environment=selected.environment.clone();
    require(
        job.operation_id == operation
            && bound == selected
            && job.report == directory.join("operation-result.json"),
        "vendor_application_operation_changed",
    )?;
    let mut result:OperationResult=if job.report.exists(){read_json(&job.report)?}else {
        serde_json::from_value(json!({"schema":3,"operation_id":operation,"state":"cleanup_unconfirmed",
            "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,"error":"vendor_submission_result_missing",
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"}))?
    };
    require(
        result.operation_id.as_deref() == Some(operation),
        "vendor_application_operation_changed",
    )?;
    if job.managed.is_some() && !result.dependency.as_ref().is_some_and(vendor_application::dependency_retired) && result.cleanup_confirmed {
        result.cleanup_confirmed = false;
        result.state = vendor_application::OperationState::CleanupUnconfirmed;
        result.error.get_or_insert("The app did not finish closing. Use Recover this application session; if it cannot complete, send Setup details.".into());
        result.error_code.get_or_insert("vendor_dependency_cleanup_unconfirmed".into());
    }
    Ok(result)
}
pub(super) fn reconcile(m:&Manager,selector:&str,operation:&str)->Result<()> {
    reconcile_with(m,selector,operation,||renderer_session::inspect_unit(&unit(selector)?))
}
fn reconcile_with(m:&Manager,selector:&str,operation:&str,
    inspect:impl FnOnce()->Result<renderer_session::UnitState>)->Result<()> {
    let directory=directory(m,selector)?;
    let _writer=m.lock(directory.join("writer.lock").strip_prefix(&m.root)?.to_str().ok_or("vendor_writer_gate")?)?;
    let prior=result(m,selector,operation)?;
    if prior.retired(){return Ok(())}
    let probe=inspect()?;
    require(probe.empty,"vendor_application_custody_uncertain")?;
    let job:vendor_application::Operation=read_json(&directory.join("operation.json"))?;
    let mut dependency=prior.dependency.clone();
    if job.managed.is_some() {
        let observed=dependency.as_mut().ok_or("vendor_dependency_cleanup_unconfirmed")?;
        if observed["dependency_cleanup_disposition"]=="not_started" && observed["service_actions_started"]==false {
            observed["process_cleanup_confirmed"]=json!(true);
        }
        require(vendor_application::dependency_retired(observed),"vendor_dependency_cleanup_unconfirmed")?;
    }
    let report=directory.join("operation-result.json");
    if report.exists() {
        let history=directory.join(format!("interrupted-{operation}-result.json"));
        if !history.exists(){fs::copy(&report,&history)?;}
    }
    // Exclusive writer + exact empty kernel cohort proves process retirement,
    // never installer success, authorization or musical functionality.
    let mut recovered=json!({"schema":3,"operation_id":operation,"state":"failed",
        "launcher_exit":null,"owned_live":0,"cleanup_confirmed":true,
        "error":"vendor_writer_retired_without_successful_result","discarded_diagnostic_bytes":0,
        "account_posture":"unknown"});
    if let Some(dependency)=dependency {recovered["dependency"]=dependency;}
    atomic_json(&report,&recovered)?;
    Ok(())
}
pub(super) fn all_retired(m: &Manager) -> Result<bool> {
    for selector in selectors(m)? {
        if live(&selector)? {
            return Ok(false);
        }
        let directory = directory(m, &selector)?;
        if directory.join("operation.json").exists() {
            let job: vendor_application::Operation = read_json(&directory.join("operation.json"))?;
            if !result(m, &selector, &job.operation_id)?.retired() {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn register(m: &Manager, args: &[String]) -> Result<()> {
    require(
        (3..=19).contains(&args.len()),
        "vendor-app register ENVIRONMENT EXECUTABLE [HELPERS]",
    )?;
    let _guard = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(
        operator_cli::vendor_retired(m)? && onboarding::all_retired(m)?,
        "vendor_application_operation_requires_retirement",
    )?;
    let record = onboarding::retained_environment_record(m, &args[1])?
        .ok_or("vendor_application_owned_installation_required")?;
    let env = operator_cli::environment_install::target(m, &record.id)?;
    let operation = record
        .installation_operation
        .as_ref()
        .ok_or("vendor_application_installation_missing")?;
    let witness = onboarding::directory(m, &record.id)?.join(format!("{operation}-result.json"));
    let mut artifacts = vec![];
    for selected in &args[2..] {
        let relative = Path::new(selected);
        require(
            !relative.is_absolute()
                && relative
                    .components()
                    .all(|c| matches!(c, std::path::Component::Normal(_))),
            "vendor_application_selection_path",
        )?;
        let path = env.root.join("compatdata/pfx/drive_c").join(relative);
        artifacts.push(Artifact {
            sha256: digest(&path)?,
            path,
        });
    }
    let app = Application {
        schema: 2,
        id: ApplicationId::Selected,
        environment: env,
        executable: artifacts.remove(0),
        helpers: artifacts,
        installer_sha256:Some(record.installer),
        installer_result: Some(Artifact {
            sha256: digest(&witness)?,
            path: witness,
        }),
        observed_installer_version: String::new(),user_selection:None,
    };
    register_application(m, &app, &_guard)
}
fn register_application(m: &Manager, app: &Application, guard: &Lock) -> Result<()> {
    guard.require_registry(m)?;
    app.verify(&m.root)?;
    let selector = app.selector()?;
    let directory = directory(m, &selector)?;
    private_dir(&directory)?;
    let path = directory.join("application.json");
    if path.exists() {
        require(
            current_application(m, &path)? == *app,
            "vendor_application_selection_changed",
        )?;
    } else {
        atomic_json(&path, app)?;
    }
    println!(
        "{}",
        serde_json::json!({"application":selector,"selected":app,"selection":"observed_artifacts"})
    );
    Ok(())
}
fn recheck_bound_assistance(m: &Manager, directory: &Path, app: &Application) -> Result<()> {
    let path=directory.join("operation.json");
    if !path.exists() { return Ok(()); }
    let prior:vendor_application::Operation=read_json(&path)?;
    if prior.application.environment==app.environment && prior.application.selector()?==app.selector()? {
        if let Some(managed)=prior.managed {
            // A previous binding cannot become a generic retry because its
            // resources disappeared or changed. New authorized environment
            // revisions make their own observations through the existing owner.
            managed.capabilities.verify_callback_inputs(app,&m.root).map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
                "The app's recorded files changed. Run its full installer again, or use Choose program to open it directly.".into()
            })?;
        }
    }
    Ok(())
}
fn run_selected(m: &Manager, args: &[String], operation: Option<&str>) -> Result<()> {
    if let Some(id) = operation {
        require(valid_hex(id, 32), "vendor_application_operation_identity")?;
    }
    if args.first().is_some_and(|s| s == "register") {
        return register(m, args);
    }
    require(args.len() == 2, "vendor-app ACTION APPLICATION")?;
    ApplicationId::parse(&args[1])?;
    let directory = directory(m, &args[1])?;
    let unit = unit(&args[1])?;
    private_dir(&directory)?;
    let record = directory.join("application.json");
    match args[0].as_str() {
        "rescan" => {
            require(
                args[1] == "arturia-software-center",
                "vendor_application_discovery_adapter_absent",
            )?;
            let _guard = m.lock("registry.lock")?;
            m.require_inactive(None)?;
            let environments: std::collections::BTreeMap<_, _> = m
                .registry()?
                .classes
                .values()
                .map(|e| {
                    (
                        e.registration.environment.id.clone(),
                        e.registration.environment.clone(),
                    )
                })
                .collect();
            require(
                environments.len() == 1,
                "vendor_application_environment_ambiguous",
            )?;
            let env = environments
                .into_values()
                .next()
                .ok_or("vendor_application_environment_absent")?;
            let env=operator_cli::environment_install::current_environment(m,&env)?;
            let operation = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(env.root.join("operation.lock"))?;
            use std::os::fd::AsRawFd;
            require(
                unsafe { libc::flock(operation.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
                "vendor_application_environment_busy",
            )?;
            let app = vendor_application::discover(&m.root, env)?;
            if record.exists() {
                require(
                    current_application(m,&record)? == app,
                    "vendor_application_update_requires_transition",
                )?;
            } else {
                atomic_json(&record, &app)?;
            }
            println!("{}", serde_json::to_string(&app)?);
            Ok(())
        }
        "status" => {
            let app=current_application(m,&record)?;
            app.verify(&m.root)?;
            let report = directory.join("operation-result.json");
            let mut operation = if report.exists() {
                serde_json::to_value(read_json::<OperationResult>(&report)?)?
            } else {
                serde_json::json!({"state":"absent"})
            };
            let live = Command::new("systemctl")
                .args([
                    "--user",
                    "is-active", &unit])
                .output()?
                .status
                .success();
            if !live && matches!(operation["state"].as_str(), Some("running" | "unknown")) {
                operation["state"] = "cleanup_unconfirmed".into();
            }
            println!(
                "{}",
                serde_json::json!({"schema":1,"application":app,"operation":operation,"owner_active":live})
            );
            Ok(())
        }
        "cancel" => {
            let _guard = m.lock("registry.lock")?;
            let job:vendor_application::Operation=read_json(&directory.join("operation.json"))?;
            let prior=result(m,&args[1],&job.operation_id)?;
            if !live(&args[1])? {
                reconcile(m,&args[1],&job.operation_id)?;
                println!("{}",serde_json::to_string(&result(m,&args[1],&job.operation_id)?)?);
                return Ok(());
            }
            if let Some(id) = operation {
                let job: serde_json::Value = read_json(&directory.join("operation.json"))?;
                require(
                    job["operation_id"] == id && prior.operation_id.as_deref() == Some(id),
                    "vendor_application_operation_changed",
                )?;
            }
            if prior.retired() {
                let state = Command::new("systemctl")
                    .args([
                        "--user",
                        "show", &unit,
                        "-p",
                        "ActiveState",
                        "--value"])
                    .output()?;
                if state.status.success()
                    && matches!(
                        std::str::from_utf8(&state.stdout)?.trim(),
                        "inactive" | "failed"
                    )
                {
                    println!("{}", serde_json::to_string(&prior)?);
                    return Ok(());
                }
            }
            let result = Command::new("systemctl")
                .args([
                    "--user",
                    "stop", &unit])
                .status()?;
            require(
                result.success(),
                "vendor_application_cancellation_unconfirmed",
            )?;
            let result: OperationResult = read_json(&directory.join("operation-result.json"))?;
            require(result.retired(), "vendor_application_cleanup_unconfirmed")?;
            if let Some(id) = operation {
                require(
                    result.operation_id.as_deref() == Some(id),
                    "vendor_application_operation_changed",
                )?;
            }
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        "launch"
        | "diagnose-agent"
        | "diagnose-runinprefix"
        | "diagnose-initialized"
        | "diagnose-accessibility" => {
            require(operation.is_some(),"vendor_application_operator_intent_required")?;
            let _writer=m.lock(directory.join("writer.lock").strip_prefix(&m.root)?.to_str().ok_or("vendor_writer_gate")?)?;
            let mode = LaunchMode::action(&args[0])?;
            let app=current_application(m,&record)?;
            app.verify(&m.root)?;
            recheck_bound_assistance(m,&directory,&app)?;
            // Observe resource bytes before admission. A structural nonmatch
            // preserves normal program launch with its specific explanation.
            let inspected = linux_vst_bridge::vendor_profile::inspect(&app, &m.root)?;
            let sw = if inspected.capabilities.is_some() || app.user_selection.is_some() {
                preparation::build::launch_software(m)?
            } else {software(m)?};
            let managed = inspected.capabilities.map(|capabilities|
                linux_vst_bridge::vendor_profile::ManagedLaunch::new(capabilities, sw.clone())).transpose()?;
            let launcher = if managed.is_some() || app.user_selection.is_some() {
                sw.installer_launch.clone()
            } else {None};
            let _guard = m.lock("registry.lock")?;
            m.require_inactive(None)?;
            require(current_application(m,&record)? == app, "vendor_application_selection_changed")?;
            require(
                app.selector()? == args[1],
                "vendor_application_selection_changed",
            )?;
            require(
                app.schema == 1 || mode == LaunchMode::Normal,
                "vendor_application_diagnostic_adapter_absent",
            )?;
            // systemd owns the launch beyond this CLI/SSH lifetime. An existing
            // unit refuses a second launch rather than duplicating the app.
            let job = directory.join("operation.json");
            if job.exists() {
                let old: OperationResult = read_json(&directory.join("operation-result.json"))?;
                require(
                    old.retired(),
                    "vendor_application_operation_requires_retirement",
                )?;
                let state = Command::new("systemctl")
                    .args(["--user", "show", &unit, "-p", "ActiveState", "--value"])
                    .output()?;
                require(
                    state.status.success()
                        && matches!(
                            std::str::from_utf8(&state.stdout)?.trim(),
                            "inactive" | "failed"
                        ),
                    "vendor_application_owner_active",
                )?;
                let history = directory.join(format!("retired-{}", random_id()?));
                private_dir(&history)?;
                fs::rename(&job, history.join("operation.json"))?;
                fs::rename(
                    directory.join("operation-result.json"),
                    history.join("result.json"),
                )?;
                let _ = Command::new("systemctl")
                    .args(["--user", "reset-failed", &unit])
                    .status()?;
            }
            let operation_id = match operation {
                Some(id) => id.to_owned(),
                None => random_id()?,
            };
            let supervised_service = managed.is_some();
            atomic_json(
                &job,
                &vendor_application::Operation {application:app,
                    report:directory.join("operation-result.json"),mode,operation_id:operation_id.clone(),launcher,
                    managed,capability_notice:inspected.missing_capability},
            )?;
            atomic_json(&directory.join("operation-result.json"),&json!({"schema":3,"operation_id":operation_id,"state":"unknown","launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,"error":null,"discarded_diagnostic_bytes":0,"account_posture":"unknown"}))?;
            let result = Command::new("systemd-run")
                .args([
                    "--user",
                    "--collect",
                    "--slice=app.slice",
                    "--property=UMask=0077",
                    if supervised_service {"--property=KillMode=mixed"} else {"--property=KillMode=control-group"},
                    if supervised_service {"--property=TimeoutStopSec=90"} else {"--property=TimeoutStopSec=30"},
                    "--property=StandardOutput=null",
                    "--property=StandardError=null",
                ])
                .arg(format!("--unit={unit}"))
                .arg("/usr/bin/python3")
                .arg(&sw.supervisor.path)
                .arg("--vendor-application")
                .arg(&job)
                .status()?;
            require(result.success(), "vendor_application_launch_unconfirmed")
        }
        _ => Err("vendor_application_action_unknown".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linux_vst_bridge::operator_model as ui;
    fn assisted() -> (test_fixture::Fixture,vendor_application::Operation) {
        let (f,app,_)=selection::tests::structured_application(27);
        let capabilities=linux_vst_bridge::vendor_profile::inspect(&app,&f.m.root).unwrap().capabilities.unwrap();
        selection::select(&f.m,&app.selector().unwrap()).unwrap();
        let software=Software{manager:f.r.host.clone(),supervisor:f.r.host.clone(),ownership:f.r.host.clone(),
            host:f.r.host.clone(),source_manifest:f.r.host.clone(),source_sha256:f.r.host.sha256.clone(),
            installer_launch:Some(f.r.host.clone()),preparation_kit:None,operator_frontend:None,native_catalogue:None};
        let directory=directory(&f.m,&app.selector().unwrap()).unwrap();
        let job=vendor_application::Operation{application:app,report:directory.join("operation-result.json"),
            mode:LaunchMode::Normal,operation_id:random_id().unwrap(),launcher:software.installer_launch.clone(),
            managed:Some(linux_vst_bridge::vendor_profile::ManagedLaunch::new(capabilities,software).unwrap()),capability_notice:None};
        atomic_json(&directory.join("operation.json"),&job).unwrap();
        (f,job)
    }
    #[test]
    fn ordinary_assisted_retry_cannot_silently_downgrade_changed_resources() {
        for missing in [false,true] {
            let (f,job)=assisted();let selector=job.application.selector().unwrap();
            let path=&job.managed.as_ref().unwrap().capabilities.resources["resources/app.asar"].artifact.path;
            if missing {fs::remove_file(path).unwrap();} else {fs::write(path,b"substituted resources").unwrap();}
            let before=fs::read(directory(&f.m,&selector).unwrap().join("operation.json")).unwrap();
            assert!(run_selected(&f.m,&["launch".into(),selector.clone()],Some(&random_id().unwrap())).is_err());
            assert_eq!(before,fs::read(directory(&f.m,&selector).unwrap().join("operation.json")).unwrap());
        }
    }
    #[test]
    fn empty_generic_cohort_cannot_erase_uncertain_service_custody() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        atomic_json(&job.report,&json!({"schema":3,"operation_id":job.operation_id,"state":"failed",
            "launcher_exit":null,"owned_live":0,"cleanup_confirmed":true,"error":"service cleanup failed",
            "discarded_diagnostic_bytes":0,"account_posture":"unknown",
            "dependency":{"process_cleanup_confirmed":true,"dependency_cleanup_disposition":"cleanup_unconfirmed"}})).unwrap();
        let before=fs::read(&job.report).unwrap();
        assert!(!result(&f.m,&selector,&job.operation_id).unwrap().retired());
        assert!(reconcile_with(&f.m,&selector,&job.operation_id,||Ok(renderer_session::UnitState{
            exists:true,empty:true,observation:None})).is_err());
        assert_eq!(before,fs::read(&job.report).unwrap());
    }
    fn selected() -> (test_fixture::Fixture,String,String,Application) {
        let f=test_fixture::Fixture::new();
        let app=Application{schema:2,id:ApplicationId::Selected,environment:f.r.environment.clone(),
            executable:f.r.module.clone(),helpers:vec![],installer_sha256:Some("ab".repeat(32)),
            installer_result:Some(f.r.host.clone()),observed_installer_version:String::new(),user_selection:None};
        let selector=app.selector().unwrap();let operation=random_id().unwrap();
        let directory=directory(&f.m,&selector).unwrap();private_dir(&directory).unwrap();
        atomic_json(&directory.join("application.json"),&app).unwrap();
        atomic_json(&directory.join("operation.json"),&vendor_application::Operation{application:app.clone(),
            report:directory.join("operation-result.json"),mode:LaunchMode::Normal,operation_id:operation.clone(),launcher:None, managed:None,capability_notice:None}).unwrap();
        (f,selector,operation,app)
    }
    #[test]
    fn historic_session_result_stays_bound_after_owned_shared_revision() {
        let (f,selector,operation,app)=selected();
        let directory=directory(&f.m,&selector).unwrap();
        atomic_json(&directory.join("operation-result.json"),&json!({"schema":3,"operation_id":operation,
            "state":"completed","launcher_exit":0,"owned_live":0,"cleanup_confirmed":true,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        let owner=random_id().unwrap();let job=f.m.root.join("operator").join(&owner);private_dir(&job).unwrap();
        atomic_json(&job.join("request.json"),&ui::Request{schema:ui::OPERATOR_SCHEMA,state_token:"test".into(),
            action:ui::Action::VendorSoftwareInstall{application:selector.clone(),daw:ui::DawChoice::Reaper}}).unwrap();
        let mut after=app.environment.clone();after.revision+=1;
        atomic_json(&job.join("vendor-maintenance.json"),&environment_revision::VendorBinding{schema:1,
            operation:owner,application:selector.clone(),selected:Artifact{sha256:digest(&directory.join("application.json")).unwrap(),path:directory.join("application.json")},
            vendor_operation:random_id().unwrap(),before:app.environment.clone(),environment:after.clone()}).unwrap();
        atomic_json(&after.root.join("environment.json"),&after).unwrap();
        assert!(result(&f.m,&selector,&operation).unwrap().successful());
        assert!(result(&f.m,&selector,&random_id().unwrap()).is_err());
        after.revision+=1;atomic_json(&after.root.join("environment.json"),&after).unwrap();
        assert!(result(&f.m,&selector,&operation).is_err(),"Unowned revision cannot be adopted");
    }
    #[test]
    fn missing_dispatch_result_remains_readable_and_requires_exact_empty_writer_recovery() {
        let (f,selector,operation,_)=selected();let directory=directory(&f.m,&selector).unwrap();
        let before=fs::read(directory.join("operation.json")).unwrap();
        assert!(!result(&f.m,&selector,&operation).unwrap().retired());
        assert!(reconcile_with(&f.m,&selector,&operation,||Ok(renderer_session::UnitState{
            exists:true,empty:false,observation:None})).is_err());
        assert!(!directory.join("operation-result.json").exists());
        let gate=directory.join("writer.lock");
        let held=f.m.lock(gate.strip_prefix(&f.m.root).unwrap().to_str().unwrap()).unwrap();
        assert!(reconcile_with(&f.m,&selector,&operation,||panic!("Live writer must retain custody")).is_err());
        drop(held);
        reconcile_with(&f.m,&selector,&operation,||Ok(renderer_session::UnitState{
            exists:false,empty:true,observation:None})).unwrap();
        let recovered=result(&f.m,&selector,&operation).unwrap();
        assert!(recovered.retired());assert!(!recovered.successful());
        assert_eq!(recovered.state,vendor_application::OperationState::Failed);
        assert_eq!(fs::read(directory.join("operation.json")).unwrap(),before);
        assert!(reconcile_with(&f.m,&selector,&random_id().unwrap(),||panic!("Changed job is refused")).is_err());
    }
}

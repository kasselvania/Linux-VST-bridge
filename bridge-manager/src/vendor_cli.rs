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
    reconcile_with(m,selector,operation,||renderer_session::inspect_unit(&unit(selector)?),
        |request|recover_census(m,request))
}
const SERVICE_STILL_RUNNING: &str =
    "The app's support service is still running. Restart the computer, then use Recover again.";
const SERVICE_UNCONFIRMED: &str = "The bridge could not confirm that the app's support service stopped. Restart the computer, then use Recover again; if it still cannot finish, send Setup details.";
const SESSION_FILES_CHANGED: &str = "The app's files changed after its session was interrupted, so the bridge cannot confirm it closed. Send Setup details.";
const RECOVER_CENSUS_SECONDS: u64 = 60;
/// Closed supervisor verdict for an interrupted profile-managed session. The
/// manager re-derives the verdict; a disagreeing record is malformed.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reverification {
    schema: u32,
    verdict: String,
    prefix_unchanged: bool,
    application_image_unchanged: bool,
    daemon_image_unchanged: bool,
    daemon_identity: String,
    candidate_count: u64,
    candidate_observation_unavailable: u64,
    listener_mask: Option<u64>,
}
impl Reverification {
    fn verdict(&self) -> Result<&'static str> {
        let identity=self.daemon_identity.as_str();
        require(self.schema==1 && matches!(identity,"unchanged"|"absent"|"not_admitted_in_session"|"changed"|"unavailable")
            && self.daemon_image_unchanged==matches!(identity,"unchanged"|"absent"|"not_admitted_in_session"),
            "vendor_recover_census_malformed")?;
        let derived=if self.candidate_count>0 || self.listener_mask.is_some_and(|mask|mask!=0) {"running"}
            else if !self.prefix_unchanged || !self.application_image_unchanged || identity=="changed" {"changed"}
            else if self.candidate_observation_unavailable>0 || self.listener_mask.is_none() || identity=="unavailable" {"unavailable"}
            else {"absent"};
        require(self.verdict==derived,"vendor_recover_census_malformed")?;
        Ok(derived)
    }
}
/// Exact records the supervisor re-observes. The daemon identity is the one
/// this session admitted, else the one bound at launch, else none.
fn recover_request(m:&Manager,selector:&str,job:&vendor_application::Operation,
    dependency:Option<&serde_json::Value>)->Result<serde_json::Value> {
    let managed=job.managed.as_ref().ok_or("vendor_dependency_cleanup_unconfirmed")?;
    let service=&managed.capabilities.profile.dependency;
    let relative=Path::new(&service.daemon);
    require(!relative.is_absolute() && relative.components().all(|c|matches!(c,std::path::Component::Normal(_))),
        "vendor_profile_relative_path")?;
    let recorded=dependency.map(|d|&d["daemon"]).filter(|d|d["sha256"].is_string() && d["size"].is_u64())
        .map(|d|json!({"sha256":d["sha256"],"size":d["size"]}));
    let bound=managed.capabilities.dependency.daemon.as_ref()
        .map(|image|json!({"sha256":image.artifact.sha256,"size":image.size}));
    Ok(json!({"schema":1,"environment":application(m,selector)?.environment,
        "application":job.application.executable,
        "daemon":{"relative":service.daemon,"identity":recorded.or(bound)},
        "listeners":service.listeners}))
}
/// Bounded read-only supervisor census; it never signals or adopts a process.
fn recover_census(m:&Manager,request:&serde_json::Value)->Result<serde_json::Value> {
    let sw=software_record(m)?;
    sw.supervisor.verify()?;sw.ownership.verify()?;
    let mut child=Command::new("/usr/bin/python3").args(["-E","-s"]).arg(&sw.supervisor.path)
        .arg("--vendor-recover-census").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let written=child.stdin.take().ok_or("vendor_recover_census_input")?.write_all(&serde_json::to_vec(request)?);
    let deadline=Instant::now()+Duration::from_secs(RECOVER_CENSUS_SECONDS);
    let status=loop {
        if let Some(status)=child.try_wait()? {break status;}
        if Instant::now()>=deadline {let _=child.kill();child.wait()?;return Err("vendor_recover_census_timeout".into());}
        std::thread::sleep(Duration::from_millis(20));
    };
    written?;
    let mut bytes=vec![];child.stdout.take().ok_or("vendor_recover_census_output")?.take(65537).read_to_end(&mut bytes)?;
    require(status.success() && bytes.len()<=65536,"vendor_recover_census_failed")?;
    Ok(serde_json::from_slice(&bytes)?)
}
/// The retained session record plus the fresh absence that retires it. The
/// interrupted result itself stays as history beside the new result.
fn reverified_dependency(prior:Option<serde_json::Value>,census:&Reverification)->serde_json::Value {
    let mut record=match prior {Some(value@serde_json::Value::Object(_))=>value,_=>json!({"session_record":"missing"})};
    record["interrupted_disposition"]=record["dependency_cleanup_disposition"].clone();
    record["process_cleanup_confirmed"]=json!(true);
    record["dependency_cleanup_disposition"]=json!("reverified_after_interruption");
    record["post_cleanup"]=json!({"schema":1,"cgroup_empty":true,
        "exact_daemon_generation_absent":census.candidate_count==0 && census.candidate_observation_unavailable==0,
        "listener_mask":census.listener_mask,"candidate_count":census.candidate_count,
        "candidate_observation_unavailable":census.candidate_observation_unavailable,
        "application_image_unchanged":census.application_image_unchanged,
        "daemon_image_unchanged":census.daemon_image_unchanged,"daemon_identity":census.daemon_identity,
        "session":"reverified_after_interruption",
        "authority":"empty_owned_unit_and_bounded_daemon_listener_census_after_interruption"});
    record
}
fn reconcile_with(m:&Manager,selector:&str,operation:&str,
    inspect:impl FnOnce()->Result<renderer_session::UnitState>,
    census:impl FnOnce(&serde_json::Value)->Result<serde_json::Value>)->Result<()> {
    let directory=directory(m,selector)?;
    let _writer=m.lock(directory.join("writer.lock").strip_prefix(&m.root)?.to_str().ok_or("vendor_writer_gate")?)?;
    let prior=result(m,selector,operation)?;
    if prior.retired(){return Ok(())}
    let probe=inspect()?;
    require(probe.empty,"vendor_application_custody_uncertain")?;
    let job:vendor_application::Operation=read_json(&directory.join("operation.json"))?;
    let mut dependency=prior.dependency.clone();
    if job.managed.is_some() {
        if let Some(observed)=dependency.as_mut() {
            if observed["dependency_cleanup_disposition"]=="not_started" && observed["service_actions_started"]==false {
                observed["process_cleanup_confirmed"]=json!(true);
            }
        }
        if !dependency.as_ref().is_some_and(vendor_application::dependency_retired) {
            // The owner stopped before recording retirement. The empty unit
            // proves its own processes are gone; the service is re-observed.
            let request=recover_request(m,selector,&job,dependency.as_ref())?;
            let observed=census(&request).and_then(|raw|Ok(serde_json::from_value::<Reverification>(raw)?))
                .map_err(|_|SERVICE_UNCONFIRMED)?;
            match observed.verdict().map_err(|_|SERVICE_UNCONFIRMED)? {
                "absent"=>{}
                "running"=>return Err(SERVICE_STILL_RUNNING.into()),
                "changed"=>return Err(SESSION_FILES_CHANGED.into()),
                _=>return Err(SERVICE_UNCONFIRMED.into()),
            }
            dependency=Some(reverified_dependency(dependency.take(),&observed));
        }
        require(dependency.as_ref().is_some_and(vendor_application::dependency_retired),"vendor_dependency_cleanup_unconfirmed")?;
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
fn submit_with(report: &Path, operation: &str, managed: bool,
    spawn: impl FnOnce()->std::io::Result<std::process::Child>) -> Result<()> {
    let mut child=match spawn() {
        Ok(child)=>child,
        Err(_)=> {
            // Spawn failure proves this submission command never executed.
            // A started command's failed/unknown acknowledgment proves no such
            // thing and retains uncertain custody instead.
            let mut value:serde_json::Value=read_json(report)?;
            require(value["operation_id"]==operation,"vendor_application_operation_changed")?;
            value["error"]="The app could not be started. Try again; if it still fails, send Setup details.".into();
            value["error_code"]="vendor_application_not_submitted".into();
            if managed {value["dependency"]=json!({"dependency_cleanup_disposition":"not_started",
                "service_actions_started":false,"process_cleanup_confirmed":false});}
            atomic_json(report,&value)?;
            return Err("The app could not be started. Try again; if it still fails, send Setup details.".into());
        }
    };
    require(child.wait()?.success(),"vendor_application_launch_unconfirmed")
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
            let mut command=Command::new("systemd-run");
            command.args([
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
                .arg(&job);
            submit_with(&directory.join("operation-result.json"),&operation_id,supervised_service,||command.spawn())
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
        let error=reconcile_with(&f.m,&selector,&job.operation_id,||Ok(renderer_session::UnitState{
            exists:true,empty:true,observation:None}),|_|Ok(verdict(json!({"verdict":"running","candidate_count":1})))).unwrap_err();
        assert_eq!(error.to_string(),SERVICE_STILL_RUNNING);
        assert_eq!(before,fs::read(&job.report).unwrap());
    }
    fn verdict(changes:serde_json::Value)->serde_json::Value {
        let mut value=json!({"schema":1,"verdict":"absent","prefix_unchanged":true,"application_image_unchanged":true,
            "daemon_image_unchanged":true,"daemon_identity":"unchanged","candidate_count":0,
            "candidate_observation_unavailable":0,"listener_mask":0});
        for (key,changed) in changes.as_object().unwrap() {value[key]=changed.clone();}
        value
    }
    fn empty_unit()->Result<renderer_session::UnitState> {
        Ok(renderer_session::UnitState{exists:false,empty:true,observation:None})
    }
    /// The power-loss record: the owner published a ready running session and
    /// never wrote its own retirement.
    fn interrupted(job:&vendor_application::Operation)->Vec<u8> {
        atomic_json(&job.report,&json!({"schema":3,"operation_id":job.operation_id,"state":"running",
            "launcher_exit":null,"owned_live":4,"cleanup_confirmed":false,"error":null,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown",
            "dependency":{"schema":1,"ready_tested":true,"daemon":{"sha256":"ee".repeat(32),"size":4096,"architecture":"x64"},
                "service_actions_started":true,"service_retirement_confirmed":false,"process_cleanup_confirmed":false,
                "dependency_cleanup_disposition":"cleanup_unconfirmed","post_cleanup":null}})).unwrap();
        fs::read(&job.report).unwrap()
    }
    #[test]
    fn interrupted_service_session_retires_after_clean_reverification_and_reopens() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        let before=interrupted(&job);
        let units=[unit(&selector).unwrap()];
        assert!(!readiness::with_unit_state_fixture(&units,"inactive",||all_retired(&f.m)).unwrap());
        let mut asked=None;
        reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|request|{asked=Some(request.clone());Ok(verdict(json!({})))}).unwrap();
        let request=asked.unwrap();let service=&job.managed.as_ref().unwrap().capabilities.profile.dependency;
        assert_eq!(request["daemon"],json!({"relative":service.daemon,"identity":{"sha256":"ee".repeat(32),"size":4096}}));
        assert_eq!(request["listeners"],json!(service.listeners));
        assert_eq!(request["application"],serde_json::to_value(&job.application.executable).unwrap());
        assert_eq!(request["environment"],serde_json::to_value(&application(&f.m,&selector).unwrap().environment).unwrap());
        let recovered=result(&f.m,&selector,&job.operation_id).unwrap();
        assert!(recovered.retired());assert!(!recovered.successful());
        assert_eq!(recovered.state,vendor_application::OperationState::Failed);
        let dependency=recovered.dependency.unwrap();
        assert_eq!(dependency["dependency_cleanup_disposition"],"reverified_after_interruption");
        assert_eq!(dependency["interrupted_disposition"],"cleanup_unconfirmed");
        assert_eq!(dependency["post_cleanup"]["session"],"reverified_after_interruption");
        assert_eq!(dependency["ready_tested"],true);
        assert_eq!(dependency["service_retirement_confirmed"],false,"A census is not a graceful service stop");
        let history=directory(&f.m,&selector).unwrap().join(format!("interrupted-{}-result.json",job.operation_id));
        assert_eq!(fs::read(history).unwrap(),before);
        // Launch admission reads the stored result directly; the card and
        // Continue read it through the bound operation.
        assert!(read_json::<OperationResult>(&job.report).unwrap().retired());
        assert!(readiness::with_unit_state_fixture(&units,"inactive",||all_retired(&f.m)).unwrap());
        reconcile_with(&f.m,&selector,&job.operation_id,||panic!("A retired session is not re-observed"),
            |_|panic!("A retired session is not re-observed")).unwrap();
    }
    #[test]
    fn interrupted_service_session_stays_unretired_while_anything_remains_or_is_unobserved() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        let before=interrupted(&job);
        let history=directory(&f.m,&selector).unwrap().join(format!("interrupted-{}-result.json",job.operation_id));
        for (census,expected) in [
            (verdict(json!({"verdict":"running","candidate_count":1})),SERVICE_STILL_RUNNING),
            (verdict(json!({"verdict":"running","listener_mask":2})),SERVICE_STILL_RUNNING),
            (verdict(json!({"verdict":"running","candidate_count":1,"candidate_observation_unavailable":1})),SERVICE_STILL_RUNNING),
            (verdict(json!({"verdict":"unavailable","candidate_observation_unavailable":1})),SERVICE_UNCONFIRMED),
            (verdict(json!({"verdict":"unavailable","listener_mask":null})),SERVICE_UNCONFIRMED),
            (verdict(json!({"verdict":"unavailable","daemon_identity":"unavailable","daemon_image_unchanged":false})),SERVICE_UNCONFIRMED),
            (verdict(json!({"verdict":"changed","application_image_unchanged":false})),SESSION_FILES_CHANGED),
            (verdict(json!({"verdict":"changed","prefix_unchanged":false})),SESSION_FILES_CHANGED),
            (verdict(json!({"verdict":"changed","daemon_identity":"changed","daemon_image_unchanged":false})),SESSION_FILES_CHANGED),
            // A verdict that disagrees with its own facts, or carries more, is malformed.
            (verdict(json!({"candidate_count":1})),SERVICE_UNCONFIRMED),
            (verdict(json!({"listener_mask":1})),SERVICE_UNCONFIRMED),
            (verdict(json!({"daemon_identity":"changed"})),SERVICE_UNCONFIRMED),
            (verdict(json!({"schema":2})),SERVICE_UNCONFIRMED),
            (verdict(json!({"pid":1234})),SERVICE_UNCONFIRMED),
            (json!("absent"),SERVICE_UNCONFIRMED),
        ] {
            let error=reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|_|Ok(census.clone())).unwrap_err();
            assert_eq!(error.to_string(),expected,"{census}");
            assert_eq!(fs::read(&job.report).unwrap(),before);assert!(!history.exists());
        }
        let error=reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,
            |_|Err("vendor_recover_census_timeout".into())).unwrap_err();
        assert_eq!(error.to_string(),SERVICE_UNCONFIRMED);
        assert!(reconcile_with(&f.m,&selector,&job.operation_id,||Ok(renderer_session::UnitState{
            exists:true,empty:false,observation:None}),|_|panic!("An occupied unit is never re-observed")).is_err());
        assert_eq!(fs::read(&job.report).unwrap(),before);assert!(!history.exists());
        assert!(!result(&f.m,&selector,&job.operation_id).unwrap().retired());
    }
    #[test]
    fn recover_request_crosses_the_actual_supervisor_census() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        let proc=f.outer.join("proc");private_dir(&proc.join("net")).unwrap();
        for name in ["tcp","tcp6"] {
            fs::write(proc.join("net").join(name),"  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n").unwrap();
        }
        let observe=|request:&serde_json::Value|->Reverification {
            let output=Command::new("python3").env("PYTHONDONTWRITEBYTECODE","1").args(["-I","-c",r#"
import json,pathlib,sys
sys.path.insert(0,sys.argv[1])
import session
print(json.dumps(session.vendor_recover_census(json.loads(sys.argv[2]),proc=pathlib.Path(sys.argv[3]))))
"#]).arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime")).arg(request.to_string()).arg(&proc).output().unwrap();
            assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
            serde_json::from_slice(&output.stdout).unwrap()
        };
        let request=recover_request(&f.m,&selector,&job,None).unwrap();
        let census=observe(&request);
        assert_eq!(census.verdict().unwrap(),"absent");
        assert_eq!(census.daemon_identity,"not_admitted_in_session");
        fs::write(proc.join("net/tcp"),"  sl  local_address rem_address   st\n   0: 0100007F:141A 00000000:0000 0A 0:0 0:0 0 0 0\n").unwrap();
        assert_eq!(observe(&request).verdict().unwrap(),"running");
        fs::remove_file(proc.join("net/tcp6")).unwrap();
        fs::write(proc.join("net/tcp"),"  sl  local_address rem_address   st\n").unwrap();
        assert_eq!(observe(&request).verdict().unwrap(),"unavailable");
        fs::write(&job.application.executable.path,b"MZ changed application").unwrap();
        assert_eq!(observe(&request).verdict().unwrap(),"changed");
    }
    #[test]
    fn owner_interrupted_without_a_session_record_reverifies_against_the_bound_daemon() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        atomic_json(&job.report,&json!({"schema":3,"operation_id":job.operation_id,"state":"cleanup_unconfirmed",
            "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,"error":"interrupted",
            "error_code":"managed_application_owner_interrupted","discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        let mut asked=None;
        reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|request|{asked=Some(request.clone());
            Ok(verdict(json!({"daemon_identity":"not_admitted_in_session"})))}).unwrap();
        assert!(job.managed.as_ref().unwrap().capabilities.dependency.daemon.is_none());
        assert_eq!(asked.unwrap()["daemon"]["identity"],serde_json::Value::Null);
        let recovered=result(&f.m,&selector,&job.operation_id).unwrap();
        assert!(recovered.retired());
        let dependency=recovered.dependency.unwrap();
        assert_eq!(dependency["session_record"],"missing");
        assert_eq!(dependency["post_cleanup"]["daemon_identity"],"not_admitted_in_session");
    }
    #[test]
    fn definite_submission_failure_can_recover_without_claiming_service_retirement() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        atomic_json(&job.report,&json!({"schema":3,"operation_id":job.operation_id,"state":"unknown",
            "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,"error":null,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        assert!(submit_with(&job.report,&job.operation_id,true,||Err(std::io::ErrorKind::NotFound.into())).is_err());
        assert!(!result(&f.m,&selector,&job.operation_id).unwrap().retired());
        reconcile_with(&f.m,&selector,&job.operation_id,||Ok(renderer_session::UnitState{
            exists:false,empty:true,observation:None}),|_|panic!("No service action needs re-observation")).unwrap();
        let recovered=result(&f.m,&selector,&job.operation_id).unwrap();
        assert!(recovered.retired());
        assert_eq!(recovered.dependency.unwrap()["service_actions_started"],false);
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
        let unmanaged=|_:&serde_json::Value|->Result<serde_json::Value> {panic!("Ordinary programs have no service census")};
        assert!(reconcile_with(&f.m,&selector,&operation,||Ok(renderer_session::UnitState{
            exists:true,empty:false,observation:None}),unmanaged).is_err());
        assert!(!directory.join("operation-result.json").exists());
        let gate=directory.join("writer.lock");
        let held=f.m.lock(gate.strip_prefix(&f.m.root).unwrap().to_str().unwrap()).unwrap();
        assert!(reconcile_with(&f.m,&selector,&operation,||panic!("Live writer must retain custody"),unmanaged).is_err());
        drop(held);
        reconcile_with(&f.m,&selector,&operation,||Ok(renderer_session::UnitState{
            exists:false,empty:true,observation:None}),unmanaged).unwrap();
        let recovered=result(&f.m,&selector,&operation).unwrap();
        assert!(recovered.retired());assert!(!recovered.successful());
        assert_eq!(recovered.state,vendor_application::OperationState::Failed);
        assert_eq!(fs::read(directory.join("operation.json")).unwrap(),before);
        assert!(reconcile_with(&f.m,&selector,&random_id().unwrap(),||panic!("Changed job is refused"),unmanaged).is_err());
    }
}

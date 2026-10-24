use super::*;
pub(super) mod selection;
pub(super) mod registration;
use serde_json::json;
#[cfg(test)]
use linux_vst_bridge::vendor_application::LaunchMode;
use linux_vst_bridge::vendor_application::{
    self, Application, ApplicationId, OperationResult,
};

pub(super) fn launch_owned(m: &Manager, application: &str, operation: &str) -> Result<()> {
    vendor_app::open(m, application, operation)
}
pub(super) fn cancel_owned(m: &Manager, application: &str, operation: &str) -> Result<()> {
    vendor_app::stop(m, application, operation)
}
pub(super) fn current_application(m: &Manager, record: &Path) -> Result<Application> {
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
    if let Some(plan) = &job.library_locations {
        if matches!(result.state, vendor_application::OperationState::Running
            | vendor_application::OperationState::Completed) {
            require(result.library_locations_consumed.as_ref() == Some(&plan.digest),
                "library_locations_consumption_unconfirmed")?;
        }
    }
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
const CHECK_NOT_RUN: &str = "The bridge could not finish checking whether the app's support service stopped. Update the bridge, then use Recover again; if it still cannot finish, send Setup details.";
const SERVICE_UNCHECKABLE: &str = "The bridge can't yet check this app's support service, so it can't close this session itself. Send Setup details.";
const APP_STILL_RUNNING: &str = "Part of the app is still running. Wait a minute and use Recover again. If that does not work, restart the computer, then use Recover again.";
const UNIT_UNAVAILABLE: &str = "The bridge could not check whether the app is still running. Try Recover again; if it still cannot finish, send Setup details.";
/// Bounds the whole check. Its process scan stops itself at 20 s, leaving the
/// rest for interpreter start, listener tables and image facts.
const RECOVER_CENSUS_LIMIT: Duration = Duration::from_secs(30);
const RECOVER_OUTPUT_BOUND: u64 = 65536;
/// Every refusal code a re-check can record. Setup details shows no other.
const RECOVER_CODES: [&str; 14] = ["vendor_recover_service_running", "vendor_recover_observation_unavailable",
    "vendor_recover_census_timeout", "vendor_recover_daemon_name_unobservable",
    "vendor_recover_census_spawn_failed", "vendor_recover_supervisor_changed",
    "vendor_recover_request_unavailable", "vendor_recover_census_request_refused",
    "vendor_recover_census_malformed", "vendor_recover_census_failed",
    "vendor_recover_census_output_bound", "vendor_application_custody_uncertain",
    "vendor_recover_unit_unavailable", "vendor_recover_census_internal"];
/// A plain refusal for the musician. Its fixed code and the closed census facts
/// are retained for Setup details; the underlying cause stays in the error chain.
#[derive(Debug)]
pub(super) struct RecoverRefusal {
    pub(super) code: &'static str,
    census: Option<Box<Reverification>>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
impl RecoverRefusal {
    fn new(code: &'static str) -> Self {
        debug_assert!(RECOVER_CODES.contains(&code), "{code}");
        Self { code, census: None, source: None }
    }
    fn caused(code: &'static str, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self { source: Some(source), ..Self::new(code) }
    }
    fn message(&self) -> &'static str {
        match self.code {
            "vendor_recover_service_running" => SERVICE_STILL_RUNNING,
            "vendor_recover_census_timeout" | "vendor_recover_observation_unavailable" => SERVICE_UNCONFIRMED,
            "vendor_recover_daemon_name_unobservable" => SERVICE_UNCHECKABLE,
            "vendor_application_custody_uncertain" => APP_STILL_RUNNING,
            "vendor_recover_unit_unavailable" => UNIT_UNAVAILABLE,
            // Spawn, changed supervisor, request, malformed or failed census.
            _ => CHECK_NOT_RUN,
        }
    }
}
impl std::fmt::Display for RecoverRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.message()) }
}
impl std::error::Error for RecoverRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref().map(|error| error as &(dyn std::error::Error + 'static))
    }
}
/// Closed supervisor facts for a session that ended without recording its own
/// retirement. The manager re-derives the verdict; a disagreeing record is
/// malformed. Only process and listener evidence decides; image facts are
/// recorded, and the next launch re-verifies every byte it uses.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reverification {
    schema: u32,
    verdict: String,
    prefix_unchanged: bool,
    application_image_unchanged: bool,
    daemon_image_unchanged: bool,
    daemon_identity: String,
    daemon_name_observable: bool,
    images_changed: Vec<String>,
    candidate_count: u64,
    candidate_observation_unavailable: u64,
    listener_mask: Option<u64>,
}
impl Reverification {
    fn derived(&self) -> &'static str {
        if self.candidate_count > 0 || self.listener_mask.is_some_and(|mask| mask != 0) {"running"}
        else if self.candidate_observation_unavailable > 0 || self.listener_mask.is_none() || !self.daemon_name_observable {"unavailable"}
        else {"absent"}
    }
    /// Every text field is one of a closed set, and the verdict follows from
    /// the facts. Only such a record is decisive or shown in Setup details.
    fn consistent(&self) -> bool {
        let identity = self.daemon_identity.as_str();
        self.schema == 1
            && vendor_application::REVERIFIED_DAEMON_IDENTITIES.contains(&identity)
            && self.daemon_image_unchanged == (identity == "unchanged")
            && self.images_changed == vendor_application::reverified_images_changed(self.application_image_unchanged, identity)
            && self.verdict == self.derived()
    }
    fn verdict(&self) -> std::result::Result<(), RecoverRefusal> {
        if !self.consistent() {
            return Err(RecoverRefusal::new("vendor_recover_census_malformed"));
        }
        match self.derived() {
            "running" => Err(RecoverRefusal::new("vendor_recover_service_running")),
            "unavailable" if !self.daemon_name_observable => Err(RecoverRefusal::new("vendor_recover_daemon_name_unobservable")),
            "unavailable" => Err(RecoverRefusal::new("vendor_recover_observation_unavailable")),
            _ => Ok(()),
        }
    }
}
/// The last re-check, typed so only fixed codes and closed facts are shown.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoverCheck {
    schema: u32,
    operation: String,
    result: String,
    code: Option<String>,
    census: Option<Reverification>,
}
/// Setup details for the current operation's last re-check, if any. Only a
/// closed record whose census agrees with its result and code is shown, and an
/// older refusal is hidden once the operation is retired.
pub(super) fn recover_check(m: &Manager, selector: &str, operation: &str) -> Option<serde_json::Value> {
    use std::os::unix::fs::OpenOptionsExt;
    let path = directory(m, selector).ok()?.join("recover-check.json");
    // Never follow a link or wait on a FIFO while refreshing a card.
    let file = fs::OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK).open(&path).ok()?;
    if !file.metadata().ok()?.is_file() { return None; }
    let mut bytes = vec![];
    file.take(16385).read_to_end(&mut bytes).ok()?;
    if bytes.len() > 16384 { return None; }
    let check: RecoverCheck = serde_json::from_slice(&bytes).ok()?;
    let census = check.census.as_ref().map(|census| (census.consistent(), census.verdict().err().map(|refusal| refusal.code)));
    let closed = match (check.result.as_str(), check.code.as_deref(), census) {
        ("retired", None, None | Some((true, None))) => true,
        ("refused", Some(code), None) => RECOVER_CODES.contains(&code),
        ("refused", Some(code), Some((true, Some(refused)))) => refused == code,
        _ => false,
    };
    if !closed || check.schema != 1 || check.operation != operation { return None; }
    if check.result == "refused" && result(m, selector, operation).is_ok_and(|current| current.retired()) { return None; }
    serde_json::to_value(check).ok()
}
/// Exact records the supervisor re-checks. The daemon identity is the one
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
fn recover_census(m:&Manager,request:&serde_json::Value)->std::result::Result<serde_json::Value,RecoverRefusal> {
    let sw=software_record(m).map_err(|error|RecoverRefusal::caused("vendor_recover_supervisor_changed",error))?;
    run_recover_census(Path::new("/usr/bin/python3"),&sw.supervisor,&sw.ownership,request,RECOVER_CENSUS_LIMIT)
}
/// Bounded read-only supervisor census; it never signals or adopts a process.
/// Every wait ends by `limit`; an unfinished census is killed and reaped.
fn run_recover_census(interpreter:&Path,supervisor:&Artifact,ownership:&Artifact,
    request:&serde_json::Value,limit:Duration)->std::result::Result<serde_json::Value,RecoverRefusal> {
    use RecoverRefusal as Refusal;
    supervisor.verify().and_then(|_|ownership.verify())
        .map_err(|error|Refusal::caused("vendor_recover_supervisor_changed",error))?;
    let input=serde_json::to_vec(request).map_err(|error|Refusal::caused("vendor_recover_request_unavailable",error.into()))?;
    if input.len()>16384 {return Err(Refusal::new("vendor_recover_request_unavailable"));}
    let deadline=Instant::now()+limit;
    let args=linux_vst_bridge::supervisor::arguments(supervisor,ownership)
        .map_err(|error|Refusal::caused("vendor_recover_supervisor_changed",error))?;
    let mut child=Command::new(interpreter).args(["-E","-s"]).args(args).arg("--vendor-recover-census")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()
        .map_err(|error|Refusal::caused("vendor_recover_census_spawn_failed",error.into()))?;
    let (sender,receiver)=std::sync::mpsc::channel();
    let (stdin,stdout)=(child.stdin.take(),child.stdout.take());
    // Both pipes are served by detached threads that report through channels.
    // This call waits on neither beyond `limit`; a pipe held open by some other
    // process can only keep its thread blocked, never this check.
    let reader=stdout.map(|mut stdout|std::thread::Builder::new().name("recover-census-output".into()).spawn(move||{
        let mut bytes=vec![];
        let read=(&mut stdout).take(RECOVER_OUTPUT_BOUND+1).read_to_end(&mut bytes).map(|_|bytes);
        let _=sender.send(read);
    }));
    let (input_sender,input_receiver)=std::sync::mpsc::channel();
    let writer=stdin.map(|mut stdin|std::thread::Builder::new().name("recover-census-input".into())
        .spawn(move||{let _=input_sender.send(stdin.write_all(&input));}));
    match (reader,writer) {
        (Some(Ok(_)),Some(Ok(_)))=>{}
        (reader,writer)=>{
            let _=child.kill();let _=child.wait();
            let cause=[reader.and_then(|spawned|spawned.err()),writer.and_then(|spawned|spawned.err())].into_iter().flatten().next();
            return Err(match cause {Some(error)=>Refusal::caused("vendor_recover_census_failed",error.into()),
                None=>Refusal::new("vendor_recover_census_failed")});
        }
    }
    let (mut status,mut output)=(None,None);
    let outcome=loop {
        if status.is_none() {
            match child.try_wait() {Ok(observed)=>status=observed,Err(_)=>break Err("vendor_recover_census_failed")}
        }
        if output.is_none() {
            output=if status.is_some() {receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())).ok()}
                else {receiver.try_recv().ok()};
        }
        match (&status,&output) {
            (_,Some(Ok(bytes))) if bytes.len() as u64>RECOVER_OUTPUT_BOUND=>break Err("vendor_recover_census_output_bound"),
            (_,Some(Err(_)))=>break Err("vendor_recover_census_failed"),
            (Some(status),Some(Ok(bytes))) if !status.success()=>break Err(census_refusal(bytes)),
            (Some(status),None) if !status.success()=>break Err("vendor_recover_census_failed"),
            (Some(_),Some(Ok(bytes)))=>break Ok(bytes.clone()),
            _=>{}
        }
        if Instant::now()>=deadline {break Err("vendor_recover_census_timeout");}
        std::thread::sleep(Duration::from_millis(10));
    };
    if status.is_none() {let _=child.kill();let _=child.wait();}
    let bytes=outcome.map_err(Refusal::new)?;
    // A census that answered has read its whole request; the write already ended.
    match input_receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Ok(()))=>{}
        Ok(Err(error))=>return Err(Refusal::caused("vendor_recover_census_failed",error.into())),
        Err(_)=>return Err(Refusal::new("vendor_recover_census_failed")),
    }
    serde_json::from_slice(&bytes).map_err(|error|Refusal::caused("vendor_recover_census_malformed",error.into()))
}
/// The census names its own refusals with a closed code on a failed exit.
fn census_refusal(bytes:&[u8])->&'static str {
    match serde_json::from_slice::<serde_json::Value>(bytes) {
        Ok(value) if value==json!({"schema":1,"error":"vendor_recover_census_request"})=>"vendor_recover_census_request_refused",
        Ok(value) if value==json!({"schema":1,"error":"vendor_recover_census_internal"})=>"vendor_recover_census_internal",
        _=>"vendor_recover_census_failed",
    }
}
/// The retained session record plus the fresh absence that retires it. The
/// earlier result itself stays as history beside the new result.
fn reverified_dependency(prior:Option<serde_json::Value>,census:&Reverification)->serde_json::Value {
    let mut record=match prior {Some(value@serde_json::Value::Object(_))=>value,_=>json!({"session_record":"missing"})};
    record["previous_disposition"]=record["dependency_cleanup_disposition"].clone();
    record["process_cleanup_confirmed"]=json!(true);
    record["dependency_cleanup_disposition"]=json!(vendor_application::REVERIFIED_CLEANUP);
    record["post_cleanup"]=json!({"schema":1,"cgroup_empty":true,
        "exact_daemon_generation_absent":census.candidate_count==0 && census.candidate_observation_unavailable==0,
        "listener_mask":census.listener_mask,"candidate_count":census.candidate_count,
        "candidate_observation_unavailable":census.candidate_observation_unavailable,
        "prefix_unchanged":census.prefix_unchanged,
        "application_image_unchanged":census.application_image_unchanged,
        "daemon_image_unchanged":census.daemon_image_unchanged,"daemon_identity":census.daemon_identity,
        "images_changed":census.images_changed,
        "session":vendor_application::REVERIFIED_CLEANUP,
        "authority":"empty_owned_unit_and_bounded_daemon_listener_census"});
    record
}
/// Re-observe current reality for an unretired session. The empty unit proves
/// the session's own processes are gone; a profile-managed support service is
/// re-checked because its owner never recorded retiring it.
fn recheck(m:&Manager,selector:&str,directory:&Path,prior:&OperationResult,
    inspect:impl FnOnce()->Result<renderer_session::UnitState>,
    census:impl FnOnce(&serde_json::Value)->std::result::Result<serde_json::Value,RecoverRefusal>)
    ->std::result::Result<(Option<serde_json::Value>,Option<Reverification>),RecoverRefusal> {
    let probe=inspect().map_err(|error|RecoverRefusal::caused("vendor_recover_unit_unavailable",error))?;
    if !probe.empty {return Err(RecoverRefusal::new("vendor_application_custody_uncertain"));}
    let job:vendor_application::Operation=read_json(&directory.join("operation.json"))
        .map_err(|error|RecoverRefusal::caused("vendor_recover_request_unavailable",error))?;
    let mut dependency=prior.dependency.clone();
    if job.managed.is_none() {return Ok((dependency,None));}
    if let Some(observed)=dependency.as_mut() {
        if observed["dependency_cleanup_disposition"]=="not_started" && observed["service_actions_started"]==false {
            observed["process_cleanup_confirmed"]=json!(true);
        }
    }
    if dependency.as_ref().is_some_and(vendor_application::dependency_retired) {return Ok((dependency,None));}
    let request=recover_request(m,selector,&job,dependency.as_ref())
        .map_err(|error|RecoverRefusal::caused("vendor_recover_request_unavailable",error))?;
    let observed:Reverification=serde_json::from_value(census(&request)?)
        .map_err(|error|RecoverRefusal::caused("vendor_recover_census_malformed",error.into()))?;
    if let Err(mut refusal)=observed.verdict() {
        // Malformed facts are never retained; they may carry arbitrary text.
        if observed.consistent() {refusal.census=Some(Box::new(observed));}
        return Err(refusal);
    }
    let dependency=reverified_dependency(dependency,&observed);
    if !vendor_application::dependency_retired(&dependency) {
        return Err(RecoverRefusal::new("vendor_recover_census_malformed"));
    }
    Ok((Some(dependency),Some(observed)))
}
fn reconcile_with(m:&Manager,selector:&str,operation:&str,
    inspect:impl FnOnce()->Result<renderer_session::UnitState>,
    census:impl FnOnce(&serde_json::Value)->std::result::Result<serde_json::Value,RecoverRefusal>)->Result<()> {
    let directory=directory(m,selector)?;
    let _writer=m.lock(directory.join("writer.lock").strip_prefix(&m.root)?.to_str().ok_or("vendor_writer_gate")?)?;
    let prior=result(m,selector,operation)?;
    if prior.retired(){return Ok(())}
    let check=|result:&str,code:Option<&str>,census:Option<&Reverification>|RecoverCheck {schema:1,
        operation:operation.into(),result:result.into(),code:code.map(Into::into),census:census.cloned()};
    let (dependency,census)=match recheck(m,selector,&directory,&prior,inspect,census) {
        Ok(checked)=>checked,
        Err(refusal)=>{
            // Diagnostics explain the refusal; they never decide custody.
            let _=atomic_json(&directory.join("recover-check.json"),&check("refused",Some(refusal.code),refusal.census.as_deref()));
            return Err(refusal.into());
        }
    };
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
    // Always replace an earlier refusal, so a retired session never shows one.
    let _=atomic_json(&directory.join("recover-check.json"),&check("retired",None,census.as_ref()));
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
/// Package gate: an app session left unretired by an interruption is checked
/// exactly as Recover checks it once its unit is empty. A live or
/// unobservable unit and any unclean verdict refuse with Recover's own words.
///
/// The census runs with the supervisor shipped in the package whose manager
/// runs the gate, supplied verified by `tools`: an older installed supervisor
/// may predate the census entirely. `tools` runs only if a census is needed.
pub(super) fn reverify_interrupted(m: &Manager, limit: Duration,
    tools: impl FnOnce() -> Result<(Artifact, Artifact)>) -> Result<()> {
    reverify_with_tools(m, live, |selector| renderer_session::inspect_unit(&unit(selector)?),
        Path::new("/usr/bin/python3"), limit, tools)
}
fn reverify_with_tools(m: &Manager, live: impl FnMut(&str) -> Result<bool>,
    inspect: impl Fn(&str) -> Result<renderer_session::UnitState>, interpreter: &Path, limit: Duration,
    tools: impl FnOnce() -> Result<(Artifact, Artifact)>) -> Result<()> {
    // Every census shares the caller's budget, so the whole re-check is bounded.
    let (deadline, mut tools, mut verified) = (Instant::now() + limit, Some(tools), None);
    reverify_interrupted_with(m, live, |selector, operation| reconcile_with(m, selector, operation,
        || inspect(selector), |request| {
            if verified.is_none() {
                let supply = tools.take().ok_or_else(|| RecoverRefusal::new("vendor_recover_supervisor_changed"))?;
                verified = Some(supply().map_err(|error| RecoverRefusal::caused("vendor_recover_supervisor_changed", error))?);
            }
            let (supervisor, ownership) = verified.as_ref().ok_or_else(|| RecoverRefusal::new("vendor_recover_supervisor_changed"))?;
            run_recover_census(interpreter, supervisor, ownership, request,
                RECOVER_CENSUS_LIMIT.min(deadline.saturating_duration_since(Instant::now())))
        }))
}
fn reverify_interrupted_with(m: &Manager, mut live: impl FnMut(&str) -> Result<bool>,
    mut recover: impl FnMut(&str, &str) -> Result<()>) -> Result<()> {
    for selector in selectors(m)? {
        let directory = directory(m, &selector)?;
        if !directory.join("operation.json").exists() || live(&selector)? {
            continue;
        }
        let job: vendor_application::Operation = read_json(&directory.join("operation.json"))?;
        if !result(m, &selector, &job.operation_id)?.retired() {
            recover(&selector, &job.operation_id)?;
        }
    }
    Ok(())
}
#[cfg(test)]
pub(super) mod interruption_tests {
    use super::*;
    /// A profile-managed session whose machine froze while its support
    /// service ran: ready, service actions started, cleanup unconfirmed.
    pub(crate) fn interrupted_session() -> (test_fixture::Fixture, String, vendor_application::Operation) {
        let (f,app,_)=selection::tests::structured_application(31);
        let capabilities=linux_vst_bridge::vendor_profile::inspect(&app,&f.m.root).unwrap().capabilities.unwrap();
        let selector=app.selector().unwrap();
        selection::select(&f.m,&selector).unwrap();
        let software=Software{manager:f.r.host.clone(),supervisor:f.r.host.clone(),ownership:f.r.host.clone(),
            host:f.r.host.clone(),source_manifest:f.r.host.clone(),source_sha256:f.r.host.sha256.clone(),
            installer_launch:Some(f.r.host.clone()),preparation_kit:None,operator_frontend:None,native_catalogue:None};
        let directory=directory(&f.m,&selector).unwrap();
        let job=vendor_application::Operation{application:app,report:directory.join("operation-result.json"),
            mode:LaunchMode::Normal,operation_id:random_id().unwrap(),launcher:software.installer_launch.clone(),
            managed:Some(linux_vst_bridge::vendor_profile::ManagedLaunch::new(capabilities,software).unwrap()),library_locations: None, capability_notice:None};
        atomic_json(&directory.join("operation.json"),&job).unwrap();
        atomic_json(&job.report,&json!({"schema":3,"operation_id":job.operation_id,"state":"running",
            "launcher_exit":null,"owned_live":4,"cleanup_confirmed":false,"error":null,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown",
            "dependency":{"schema":1,"ready_tested":true,"daemon":{"sha256":"ee".repeat(32),"size":4096,"architecture":"x64"},
                "service_actions_started":true,"service_retirement_confirmed":false,"process_cleanup_confirmed":false,
                "dependency_cleanup_disposition":"cleanup_unconfirmed","post_cleanup":null}})).unwrap();
        (f,selector,job)
    }
    /// The supervisor census verdict: clean absence, or a remaining daemon.
    pub(crate) fn census(running: bool) -> serde_json::Value {
        json!({"schema":1,"verdict":if running {"running"} else {"absent"},"prefix_unchanged":true,
            "application_image_unchanged":true,"daemon_image_unchanged":true,"daemon_identity":"unchanged",
            "daemon_name_observable":true,"images_changed":[],
            "candidate_count":u64::from(running),"candidate_observation_unavailable":0,"listener_mask":0})
    }
    /// The package gate's re-verification with an empty unit and a fixed census.
    pub(crate) fn reverify_with_census(m: &Manager, verdict: serde_json::Value) -> Result<()> {
        reverify_interrupted_with(m, |_| Ok(false), |selector, operation| reconcile_with(m, selector, operation,
            || Ok(renderer_session::UnitState{exists:false,empty:true,observation:None}), |_| Ok(verdict.clone())))
    }
    #[test]
    fn gate_reverification_retires_only_an_empty_unit_with_a_clean_census() {
        let (f,selector,job)=interrupted_session();
        let before=fs::read(&job.report).unwrap();
        // A live or unobservable unit is never re-observed.
        reverify_interrupted_with(&f.m,|_|Ok(true),|_,_|panic!("A live session is not re-observed")).unwrap();
        assert!(reverify_interrupted_with(&f.m,|_|Err("readiness_unit_state_unavailable".into()),
            |_,_|panic!("An unobservable session is not re-observed")).is_err());
        let refusal=reverify_with_census(&f.m,census(true)).unwrap_err();
        assert_eq!(refusal.to_string(),SERVICE_STILL_RUNNING);
        assert_eq!(refusal.downcast_ref::<RecoverRefusal>().unwrap().code,"vendor_recover_service_running");
        assert_eq!(fs::read(&job.report).unwrap(),before);
        assert!(!result(&f.m,&selector,&job.operation_id).unwrap().retired());
        reverify_with_census(&f.m,census(false)).unwrap();
        assert!(result(&f.m,&selector,&job.operation_id).unwrap().retired());
        // A retired session is left alone.
        reverify_interrupted_with(&f.m,|_|Ok(false),|_,_|panic!("A retired session is not re-observed")).unwrap();
    }
    #[test]
    fn gate_census_runs_the_package_supervisor_when_the_installed_one_predates_it() {
        let (f,selector,job)=interrupted_session();
        let before=fs::read(&job.report).unwrap();
        let script=|name:&str,body:&str|->Artifact {
            let path=f.outer.join(name);fs::write(&path,body).unwrap();Artifact{sha256:digest(&path).unwrap(),path}
        };
        let (python,limit)=(Path::new("python3"),Duration::from_secs(20));
        let ownership=script("ownership.py","");
        // A supervisor from before the census reads its mode as a spec path.
        let installed=script("installed.py","import pathlib,sys\npathlib.Path(sys.argv[1]).read_text()\n");
        let packaged=script("packaged.py",&format!("import json,sys\nassert sys.argv[1:]==['--vendor-recover-census']\n\
            json.load(sys.stdin)\nprint({:?})\n",census(false).to_string()));
        let empty=|_:&str|Ok(renderer_session::UnitState{exists:false,empty:true,observation:None});
        let check=||read_json::<serde_json::Value>(&directory(&f.m,&selector).unwrap().join("recover-check.json")).unwrap();
        let refusal=reconcile_with(&f.m,&selector,&job.operation_id,||empty(""),
            |request|run_recover_census(python,&installed,&ownership,request,limit)).unwrap_err();
        assert!(refusal.downcast_ref::<RecoverRefusal>().unwrap().code.starts_with("vendor_recover_census_"));
        assert_eq!(check()["result"],"refused");
        // Unavailable or changed package tools never run and change nothing.
        let code=|error:Box<dyn std::error::Error+Send+Sync>|error.downcast_ref::<RecoverRefusal>().unwrap().code;
        assert_eq!(code(reverify_with_tools(&f.m,|_|Ok(false),empty,python,limit,
            ||Err("package_artifact_changed".into())).unwrap_err()),"vendor_recover_supervisor_changed");
        let stale=Artifact{path:packaged.path.clone(),sha256:"00".repeat(32)};
        assert_eq!(code(reverify_with_tools(&f.m,|_|Ok(false),empty,python,limit,
            ||Ok((stale,ownership.clone()))).unwrap_err()),"vendor_recover_supervisor_changed");
        assert_eq!(fs::read(&job.report).unwrap(),before);
        // The running package's verified supervisor completes the same check.
        reverify_with_tools(&f.m,|_|Ok(false),empty,python,limit,||Ok((packaged.clone(),ownership.clone()))).unwrap();
        assert!(result(&f.m,&selector,&job.operation_id).unwrap().retired());
        assert_eq!(check()["result"],"retired");
    }
}
fn register_application(m: &Manager, app: &Application, guard: &Lock) -> Result<()> {
    guard.require_registry(m)?;
    app.verify(&m.root)?;
    publish_application(m, app, guard)
}
fn publish_application(m: &Manager, app: &Application, guard: &Lock) -> Result<()> {
    guard.require_registry(m)?;
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
/// True when the previous launch of this exact app had automatic support.
pub(super) fn recheck_bound_assistance(m: &Manager, directory: &Path, app: &Application) -> Result<bool> {
    let path=directory.join("operation.json");
    if !path.exists() { return Ok(false); }
    let prior:vendor_application::Operation=read_json(&path)?;
    let Some(managed)=prior.managed else { return Ok(false); };
    if prior.application.selector()?!=app.selector()? { return Ok(false); }
    if prior.application.environment==app.environment {
        // A previous binding cannot become a generic retry because its
        // resources disappeared or changed. New authorized environment
        // revisions make their own observations through the existing owner.
        managed.capabilities.verify_callback_inputs(app,&m.root).map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            "The app's recorded files changed. Run its full installer again, or use Choose program to open it directly.".into()
        })?;
    }
    Ok(true)
}
/// An app that had automatic support never silently reopens without it.
pub(super) const ASSISTANCE_LOST: &str = "This app had automatic setup before, but its files no longer match what this version of the bridge supports. Run its full installer again, or use Choose program to open it without automatic setup.";

#[cfg(test)]
mod tests {
    use super::*;
    use super::vendor_app::{cancel_with, recover_stopped, submit_with};
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
            managed:Some(linux_vst_bridge::vendor_profile::ManagedLaunch::new(capabilities,software).unwrap()),library_locations: None, capability_notice:None};
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
            assert!(vendor_app::open(&f.m,&selector,&random_id().unwrap()).is_err());
            assert_eq!(before,fs::read(directory(&f.m,&selector).unwrap().join("operation.json")).unwrap());
        }
    }
    #[test]
    fn launch_saved_under_an_earlier_profile_revision_still_opens_after_an_update() {
        // Profile data improves with updates; the previous launch record of the
        // same profile is rechecked against its own bound files, not refused.
        let (f,mut job)=assisted();let selector=job.application.selector().unwrap();
        let directory=directory(&f.m,&selector).unwrap();
        let save=|job:&vendor_application::Operation|atomic_json(&directory.join("operation.json"),job).unwrap();
        let capabilities=&mut job.managed.as_mut().unwrap().capabilities;
        capabilities.profile.rendering=ui::RendererPolicy::Inherited;
        capabilities.profile.browser_scheme="earlier-scheme".into();
        capabilities.profile_sha256=linux_vst_bridge::vendor_profile::profile_sha256(&capabilities.profile).unwrap();
        save(&job);
        assert!(recheck_bound_assistance(&f.m,&directory,&job.application).unwrap());
        assert!(linux_vst_bridge::vendor_profile::inspect(&job.application,&f.m.root).unwrap().capabilities.is_some());
        // Its own files are still checked, and another profile is never adopted.
        let mut other=job.clone();
        other.managed.as_mut().unwrap().capabilities.profile.id="other_vendor".into();
        save(&other);
        assert!(recheck_bound_assistance(&f.m,&directory,&job.application).is_err());
        save(&job);
        fs::write(&job.managed.as_ref().unwrap().capabilities.resources["resources/app.asar"].artifact.path,b"substituted").unwrap();
        assert!(recheck_bound_assistance(&f.m,&directory,&job.application).is_err());
    }
    #[test]
    fn app_that_had_automatic_setup_never_silently_reopens_without_it() {
        // An earlier profile revision bound fewer files; the current one needs a
        // file this app no longer has. Reopening says so instead of dropping support.
        let (f,mut job)=assisted();let selector=job.application.selector().unwrap();
        let directory=directory(&f.m,&selector).unwrap();
        let capabilities=&mut job.managed.as_mut().unwrap().capabilities;
        let dropped=capabilities.profile.resources.pop().unwrap();
        fs::remove_file(&capabilities.resources.remove(&dropped).unwrap().artifact.path).unwrap();
        capabilities.profile_sha256=linux_vst_bridge::vendor_profile::profile_sha256(&capabilities.profile).unwrap();
        atomic_json(&directory.join("operation.json"),&job).unwrap();
        assert!(recheck_bound_assistance(&f.m,&directory,&job.application).unwrap());
        let before=fs::read(directory.join("operation.json")).unwrap();
        let error=vendor_app::open(&f.m,&selector,&random_id().unwrap()).unwrap_err();
        assert_eq!(error.to_string(),ASSISTANCE_LOST);
        assert_eq!(before,fs::read(directory.join("operation.json")).unwrap());
        // After an environment revision the old files are not compared, but
        // the earlier automatic setup still is never silently dropped.
        let mut revised=job.clone();revised.application.environment.revision+=1;
        atomic_json(&directory.join("operation.json"),&revised).unwrap();
        fs::remove_file(&job.managed.as_ref().unwrap().capabilities.resources["resources/app.asar"].artifact.path).unwrap();
        assert!(recheck_bound_assistance(&f.m,&directory,&job.application).unwrap());
        let error=vendor_app::open(&f.m,&selector,&random_id().unwrap()).unwrap_err();
        assert_eq!(error.to_string(),ASSISTANCE_LOST);
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
        assert_eq!(error.downcast_ref::<RecoverRefusal>().unwrap().code,"vendor_recover_service_running");
        assert_eq!(before,fs::read(&job.report).unwrap());
    }
    fn verdict(changes:serde_json::Value)->serde_json::Value {
        let mut value=json!({"schema":1,"verdict":"absent","prefix_unchanged":true,"application_image_unchanged":true,
            "daemon_image_unchanged":true,"daemon_identity":"unchanged","daemon_name_observable":true,"images_changed":[],
            "candidate_count":0,"candidate_observation_unavailable":0,"listener_mask":0});
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
    fn unconfirmed_service_session_retires_after_clean_recheck_and_reopens() {
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
        assert_eq!(dependency["dependency_cleanup_disposition"],vendor_application::REVERIFIED_CLEANUP);
        assert_eq!(dependency["previous_disposition"],"cleanup_unconfirmed");
        assert_eq!(dependency["post_cleanup"]["session"],vendor_application::REVERIFIED_CLEANUP);
        assert_eq!(dependency["post_cleanup"]["images_changed"],json!([]));
        assert_eq!(dependency["ready_tested"],true);
        assert_eq!(dependency["service_retirement_confirmed"],false,"A census is not a graceful service stop");
        let history=directory(&f.m,&selector).unwrap().join(format!("interrupted-{}-result.json",job.operation_id));
        assert_eq!(fs::read(history).unwrap(),before);
        let check=recover_check(&f.m,&selector,&job.operation_id).unwrap();
        assert_eq!((&check["result"],&check["code"],&check["census"]["verdict"]),(&json!("retired"),&json!(null),&json!("absent")));
        assert!(recover_check(&f.m,&selector,&random_id().unwrap()).is_none());
        // Launch admission reads the stored result directly; the card and
        // Continue read it through the bound operation.
        assert!(read_json::<OperationResult>(&job.report).unwrap().retired());
        assert!(readiness::with_unit_state_fixture(&units,"inactive",||all_retired(&f.m)).unwrap());
        reconcile_with(&f.m,&selector,&job.operation_id,||panic!("A retired session is not re-checked"),
            |_|panic!("A retired session is not re-checked")).unwrap();
    }
    #[test]
    fn changed_or_unreadable_images_do_not_keep_a_session_that_nothing_runs() {
        // e.g. the app updated itself mid-session or its daemon was half written.
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        interrupted(&job);
        reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|_|Ok(verdict(json!({"prefix_unchanged":false,
            "application_image_unchanged":false,"daemon_image_unchanged":false,"daemon_identity":"unavailable",
            "images_changed":["application","daemon"]})))).unwrap();
        let recovered=result(&f.m,&selector,&job.operation_id).unwrap();
        assert!(recovered.retired());
        let post=&recovered.dependency.as_ref().unwrap()["post_cleanup"];
        assert_eq!((&post["images_changed"],&post["application_image_unchanged"],&post["daemon_image_unchanged"],&post["prefix_unchanged"]),
            (&json!(["application","daemon"]),&json!(false),&json!(false),&json!(false)));
        // The next Open re-verifies every byte; a changed app cannot launch silently.
        fs::write(&job.application.executable.path,b"MZ self-updated application").unwrap();
        let bound=fs::read(directory(&f.m,&selector).unwrap().join("operation.json")).unwrap();
        let error=vendor_app::open(&f.m,&selector,&random_id().unwrap()).unwrap_err();
        assert_eq!(error.to_string(),"artifact missing or changed");
        assert_eq!(bound,fs::read(directory(&f.m,&selector).unwrap().join("operation.json")).unwrap());
    }
    #[test]
    fn recover_releases_the_registry_while_the_app_writer_blocks_its_launch() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        interrupted(&job);
        let bound=fs::read(directory(&f.m,&selector).unwrap().join("operation.json")).unwrap();
        let registry=f.m.lock("registry.lock").unwrap();
        recover_stopped(&f.m,&selector,&job.operation_id,registry,|m,selector,operation|
            reconcile_with(m,selector,operation,empty_unit,|_|{
                // Plug-in admission takes this lock without waiting.
                drop(m.lock("registry.lock").expect("registry is free during the census"));
                let launch=vendor_app::open(m,selector,&random_id().unwrap()).unwrap_err();
                assert!(launch.is::<linux_vst_bridge::operator_lock::LockBusy>(),"{launch}");
                Ok(verdict(json!({})))
            })).unwrap();
        assert!(result(&f.m,&selector,&job.operation_id).unwrap().retired());
        assert_eq!(bound,fs::read(directory(&f.m,&selector).unwrap().join("operation.json")).unwrap());
    }
    #[test]
    fn a_retired_session_never_keeps_an_earlier_refusal_beside_it() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        atomic_json(&job.report,&json!({"schema":3,"operation_id":job.operation_id,"state":"unknown",
            "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,"error":null,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown",
            "dependency":{"dependency_cleanup_disposition":"not_started","service_actions_started":false,
                "process_cleanup_confirmed":false}})).unwrap();
        assert!(reconcile_with(&f.m,&selector,&job.operation_id,||Ok(renderer_session::UnitState{
            exists:true,empty:false,observation:None}),|_|panic!("not re-checked")).is_err());
        assert_eq!(recover_check(&f.m,&selector,&job.operation_id).unwrap()["result"],"refused");
        reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|_|panic!("No service action needs re-checking")).unwrap();
        let check=recover_check(&f.m,&selector,&job.operation_id).unwrap();
        assert_eq!((&check["result"],&check["code"],&check["census"]),(&json!("retired"),&json!(null),&json!(null)));
        // Only closed records with known codes reach Setup details.
        let path=directory(&f.m,&selector).unwrap().join("recover-check.json");
        for record in [json!({"schema":1,"operation":job.operation_id,"result":"refused","code":"arbitrary text","census":null}),
            json!({"schema":1,"operation":job.operation_id,"result":"retired","code":"vendor_recover_census_timeout","census":null}),
            json!({"schema":1,"operation":job.operation_id,"result":"refused","code":null,"census":null}),
            json!({"schema":1,"operation":job.operation_id,"result":"retired","code":null,"census":null,"path":"/home"})] {
            atomic_json(&path,&record).unwrap();
            assert!(recover_check(&f.m,&selector,&job.operation_id).is_none(),"{record}");
        }
        // An older refusal left beside an already retired operation is hidden.
        atomic_json(&path,&json!({"schema":1,"operation":job.operation_id,"result":"refused",
            "code":"vendor_recover_census_timeout","census":null})).unwrap();
        assert!(recover_check(&f.m,&selector,&job.operation_id).is_none());
        reconcile_with(&f.m,&selector,&job.operation_id,||panic!("retired"),|_|panic!("retired")).unwrap();
        let target=f.outer.join("elsewhere.json");
        atomic_json(&target,&json!({"schema":1,"operation":job.operation_id,"result":"retired","code":null,"census":null})).unwrap();
        fs::remove_file(&path).unwrap();std::os::unix::fs::symlink(&target,&path).unwrap();
        assert!(recover_check(&f.m,&selector,&job.operation_id).is_none(),"A linked record is never followed");
        // A FIFO never blocks a card refresh.
        fs::remove_file(&path).unwrap();
        let fifo=std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe {libc::mkfifo(fifo.as_ptr(),0o600)},0);
        let started=Instant::now();
        assert!(recover_check(&f.m,&selector,&job.operation_id).is_none());
        assert!(started.elapsed()<Duration::from_secs(2));
    }
    #[test]
    fn setup_details_show_only_census_facts_that_agree_with_the_recorded_result() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        interrupted(&job);
        let path=directory(&f.m,&selector).unwrap().join("recover-check.json");
        let record=|result:&str,code:Option<&str>,census:serde_json::Value|json!({"schema":1,
            "operation":job.operation_id,"result":result,"code":code,"census":census});
        let running=verdict(json!({"verdict":"running","candidate_count":1}));
        atomic_json(&path,&record("refused",Some("vendor_recover_service_running"),running.clone())).unwrap();
        assert_eq!(recover_check(&f.m,&selector,&job.operation_id).unwrap()["census"]["verdict"],"running");
        for (record,why) in [
            (record("retired",None,running.clone()),"a retired result with a running census"),
            (record("refused",Some("vendor_recover_service_running"),verdict(json!({}))),"a refusal with an absent census"),
            (record("refused",Some("vendor_recover_observation_unavailable"),running.clone()),"a code the census does not give"),
            (record("refused",Some("vendor_recover_census_malformed"),verdict(json!({"daemon_identity":"x".repeat(15000)}))),"free text in a census field"),
            (record("refused",Some("vendor_recover_census_malformed"),verdict(json!({"images_changed":["anything at all"]}))),"an unknown image name"),
            (record("refused",Some("vendor_recover_census_malformed"),verdict(json!({"verdict":"probably fine"}))),"a free-text verdict"),
        ] {
            atomic_json(&path,&record).unwrap();
            assert!(recover_check(&f.m,&selector,&job.operation_id).is_none(),"{why}");
        }
        // A malformed census is refused without retaining any of its text.
        let error=reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,
            |_|Ok(verdict(json!({"daemon_identity":"text from somewhere else"})))).unwrap_err();
        assert_eq!(error.downcast_ref::<RecoverRefusal>().unwrap().code,"vendor_recover_census_malformed");
        let check=recover_check(&f.m,&selector,&job.operation_id).unwrap();
        assert_eq!((&check["code"],&check["census"]),(&json!("vendor_recover_census_malformed"),&json!(null)));
    }
    #[test]
    fn unconfirmed_service_session_stays_unretired_while_anything_remains_or_is_unobserved() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        let before=interrupted(&job);
        let history=directory(&f.m,&selector).unwrap().join(format!("interrupted-{}-result.json",job.operation_id));
        let refused=|error:Box<dyn std::error::Error+Send+Sync>,code:&str,message:&str| {
            assert_eq!(error.to_string(),message,"{code}");
            assert_eq!(error.downcast_ref::<RecoverRefusal>().unwrap().code,code);
            assert_eq!(recover_check(&f.m,&selector,&job.operation_id).unwrap()["code"],code);
            assert_eq!(fs::read(&job.report).unwrap(),before);assert!(!history.exists());
        };
        for (census,code,message) in [
            (verdict(json!({"verdict":"running","candidate_count":1})),"vendor_recover_service_running",SERVICE_STILL_RUNNING),
            (verdict(json!({"verdict":"running","listener_mask":2})),"vendor_recover_service_running",SERVICE_STILL_RUNNING),
            (verdict(json!({"verdict":"running","candidate_count":1,"candidate_observation_unavailable":1})),"vendor_recover_service_running",SERVICE_STILL_RUNNING),
            (verdict(json!({"verdict":"unavailable","candidate_observation_unavailable":1})),"vendor_recover_observation_unavailable",SERVICE_UNCONFIRMED),
            (verdict(json!({"verdict":"unavailable","listener_mask":null})),"vendor_recover_observation_unavailable",SERVICE_UNCONFIRMED),
            (verdict(json!({"verdict":"unavailable","daemon_name_observable":false,"candidate_observation_unavailable":1})),"vendor_recover_daemon_name_unobservable",SERVICE_UNCHECKABLE),
            // A verdict that disagrees with its own facts, or carries more, is malformed.
            (verdict(json!({"candidate_count":1})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"listener_mask":1})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"daemon_identity":"changed"})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"daemon_identity":"absent"})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"application_image_unchanged":false})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"images_changed":["daemon"]})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"verdict":"changed"})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"schema":2})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (verdict(json!({"pid":1234})),"vendor_recover_census_malformed",CHECK_NOT_RUN),
            (json!("absent"),"vendor_recover_census_malformed",CHECK_NOT_RUN),
        ] {
            refused(reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|_|Ok(census.clone())).unwrap_err(),code,message);
        }
        // Only a slow or unobservable check is worth a restart.
        refused(reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,
            |_|Err(RecoverRefusal::new("vendor_recover_census_timeout"))).unwrap_err(),"vendor_recover_census_timeout",SERVICE_UNCONFIRMED);
        for code in ["vendor_recover_census_spawn_failed","vendor_recover_supervisor_changed","vendor_recover_request_unavailable",
            "vendor_recover_census_request_refused","vendor_recover_census_output_bound","vendor_recover_census_failed",
            "vendor_recover_census_internal"] {
            refused(reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|_|Err(RecoverRefusal::new(code))).unwrap_err(),code,CHECK_NOT_RUN);
        }
        refused(reconcile_with(&f.m,&selector,&job.operation_id,||Ok(renderer_session::UnitState{
            exists:true,empty:false,observation:None}),|_|panic!("An occupied unit is never re-checked")).unwrap_err(),
            "vendor_application_custody_uncertain",APP_STILL_RUNNING);
        let error=reconcile_with(&f.m,&selector,&job.operation_id,||Err("renderer_unit_state_unavailable".into()),
            |_|panic!("An unobserved unit is never re-checked")).unwrap_err();
        let cause=std::error::Error::source(error.downcast_ref::<RecoverRefusal>().unwrap()).unwrap().to_string();
        assert_eq!(cause,"renderer_unit_state_unavailable");
        refused(error,"vendor_recover_unit_unavailable",UNIT_UNAVAILABLE);
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
session.vendor_managed_root=lambda:pathlib.Path(sys.argv[4])
print(json.dumps(session.vendor_recover_census(json.loads(sys.argv[2]),proc=pathlib.Path(sys.argv[3]))))
"#]).arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bridge-manager/runtime")).arg(request.to_string()).arg(&proc).arg(&f.m.root).output().unwrap();
            assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
            serde_json::from_slice(&output.stdout).unwrap()
        };
        let request=recover_request(&f.m,&selector,&job,None).unwrap();
        let census=observe(&request);
        census.verdict().unwrap();
        assert!(census.prefix_unchanged && census.application_image_unchanged);
        assert_eq!(census.daemon_identity,"not_admitted_in_session");
        fs::write(proc.join("net/tcp"),"  sl  local_address rem_address   st\n   0: 0100007F:141A 00000000:0000 0A 0:0 0:0 0 0 0\n").unwrap();
        assert_eq!(observe(&request).verdict().unwrap_err().code,"vendor_recover_service_running");
        fs::remove_file(proc.join("net/tcp6")).unwrap();
        fs::write(proc.join("net/tcp"),"  sl  local_address rem_address   st\n").unwrap();
        assert_eq!(observe(&request).verdict().unwrap_err().code,"vendor_recover_observation_unavailable");
        fs::write(proc.join("net/tcp6"),"  sl  local_address rem_address   st\n").unwrap();
        fs::write(&job.application.executable.path,b"MZ changed application").unwrap();
        let census=observe(&request);
        census.verdict().unwrap();
        assert_eq!(census.images_changed,["application"]);
    }
    #[test]
    fn recover_census_bounds_the_actual_supervisor_process() {
        let f=test_fixture::Fixture::new();
        let script=|name:&str,body:&str|->Artifact {
            let path=f.outer.join(name);fs::write(&path,body).unwrap();Artifact{sha256:digest(&path).unwrap(),path}
        };
        // Product launches use the system interpreter; build images may only have it on PATH.
        let (python,request,limit)=(Path::new("python3"),json!({"schema":1}),Duration::from_secs(20));
        let ownership=script("fake-ownership.py","");
        let code=|result:std::result::Result<serde_json::Value,RecoverRefusal>|result.unwrap_err().code;
        let echo=script("echo.py","import json,sys\nassert sys.argv[1:]==['--vendor-recover-census']\nprint(json.dumps({'echo':json.load(sys.stdin)}))\n");
        assert_eq!(run_recover_census(python,&echo,&ownership,&request,limit).unwrap(),json!({"echo":{"schema":1}}));
        // Changed supervisor or ownership bytes never run.
        let marker=f.outer.join("ran");
        let changed=script("changed.py",&format!("open({:?},'w').write('ran')\n",marker));
        let stale=|artifact:&Artifact|Artifact{path:artifact.path.clone(),sha256:"00".repeat(32)};
        assert_eq!(code(run_recover_census(python,&stale(&changed),&ownership,&request,limit)),"vendor_recover_supervisor_changed");
        assert_eq!(code(run_recover_census(python,&changed,&stale(&ownership),&request,limit)),"vendor_recover_supervisor_changed");
        assert!(!marker.exists());
        assert_eq!(code(run_recover_census(Path::new("/nonexistent/python3"),&echo,&ownership,&request,limit)),
            "vendor_recover_census_spawn_failed");
        // A census that outlives its limit is killed and reaped. It records its
        // PID before sleeping; the test waits until it really started.
        let pid=f.outer.join("pid");
        let slow=script("slow.py",&format!("import os,time\nopen({:?}+'.tmp','w').write(str(os.getpid()))\nos.rename({:?}+'.tmp',{:?})\ntime.sleep(60)\n",pid,pid,pid));
        let started=Instant::now();
        let running=std::thread::scope(|scope|{
            let census=scope.spawn(||run_recover_census(python,&slow,&ownership,&request,Duration::from_secs(5)));
            while !pid.exists() && started.elapsed()<Duration::from_secs(5) {std::thread::sleep(Duration::from_millis(10));}
            let observed=fs::read_to_string(&pid).expect("the census started");
            assert!(Path::new("/proc").join(observed.trim()).exists(),"The census is still running before its limit");
            (census.join().unwrap(),observed)
        });
        assert_eq!(code(running.0),"vendor_recover_census_timeout");
        assert!(started.elapsed()<Duration::from_secs(15));
        assert!(!Path::new("/proc").join(running.1.trim()).exists(),"The timed-out census is reaped");
        let loud=script("loud.py","import sys\nsys.stdout.write('x'*70000)\nsys.stdout.flush()\n");
        assert_eq!(code(run_recover_census(python,&loud,&ownership,&request,limit)),"vendor_recover_census_output_bound");
        let failing=script("failing.py","import sys\nprint('{}')\nsys.exit(3)\n");
        assert_eq!(code(run_recover_census(python,&failing,&ownership,&request,limit)),"vendor_recover_census_failed");
        let prose=script("prose.py","print('not a verdict')\n");
        assert_eq!(code(run_recover_census(python,&prose,&ownership,&request,limit)),"vendor_recover_census_malformed");
        // The census names its own refusals; a request it rejects is a bridge fault.
        let actual=|name:&str|script(name,&fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bridge-manager/runtime").join(name)).unwrap());
        let (session,owner)=(actual("session.py"),actual("ownership.py"));
        assert_eq!(code(run_recover_census(python,&session,&owner,&request,limit)),"vendor_recover_census_request_refused");
        let internal=script("internal.py","print('{\"schema\":1,\"error\":\"vendor_recover_census_internal\"}')\nraise SystemExit(1)\n");
        assert_eq!(code(run_recover_census(python,&internal,&ownership,&request,limit)),"vendor_recover_census_internal");
        // A request the census could never read whole is refused before it runs.
        let large=json!({"schema":1,"padding":"x".repeat(20000)});
        assert_eq!(code(run_recover_census(python,&echo,&ownership,&large,limit)),"vendor_recover_request_unavailable");
    }
    #[test]
    fn owner_interrupted_without_a_session_record_reverifies_against_the_bound_daemon() {
        let (f,job)=assisted();let selector=job.application.selector().unwrap();
        atomic_json(&job.report,&json!({"schema":3,"operation_id":job.operation_id,"state":"cleanup_unconfirmed",
            "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,"error":"interrupted",
            "error_code":"managed_application_owner_interrupted","discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        let mut asked=None;
        reconcile_with(&f.m,&selector,&job.operation_id,empty_unit,|request|{asked=Some(request.clone());
            Ok(verdict(json!({"daemon_identity":"not_admitted_in_session","daemon_image_unchanged":false})))}).unwrap();
        assert!(job.managed.as_ref().unwrap().capabilities.dependency.daemon.is_none());
        assert_eq!(asked.unwrap()["daemon"]["identity"],serde_json::Value::Null);
        let recovered=result(&f.m,&selector,&job.operation_id).unwrap();
        assert!(recovered.retired());
        let dependency=recovered.dependency.unwrap();
        assert_eq!(dependency["session_record"],"missing");
        assert_eq!(dependency["post_cleanup"]["daemon_identity"],"not_admitted_in_session");
        assert_eq!(dependency["post_cleanup"]["daemon_image_unchanged"],false);
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
            report:directory.join("operation-result.json"),mode:LaunchMode::Normal,operation_id:operation.clone(),launcher:None, managed:None,library_locations: None, capability_notice:None}).unwrap();
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
    fn stop_reaches_the_live_supervisor_while_it_holds_its_writer() {
        use std::cell::{Cell,RefCell};
        let (f,selector,operation,_)=selected();
        let directory=directory(&f.m,&selector).unwrap();
        let report=directory.join("operation-result.json");
        atomic_json(&report,&json!({"schema":3,"operation_id":operation,"state":"unknown",
            "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        let gate=directory.join("writer.lock");
        let writer=RefCell::new(Some(f.m.lock(gate.strip_prefix(&f.m.root).unwrap().to_str().unwrap()).unwrap()));
        let live=Cell::new(true);
        cancel_with(&f.m,&selector,Some(&operation),Duration::from_millis(100),||Ok(live.get()),|| {
            // Only the external stop is simulated; the exact job/result,
            // registry and supervisor's lifetime writer are real.
            assert!(writer.borrow().is_some());
            atomic_json(&report,&json!({"schema":3,"operation_id":operation,"state":"cancelled",
                "launcher_exit":0,"owned_live":0,"cleanup_confirmed":true,
                "discarded_diagnostic_bytes":0,"account_posture":"unknown"}))?;
            live.set(false);drop(writer.borrow_mut().take());Ok(())
        }).unwrap();
        assert!(result(&f.m,&selector,&operation).unwrap().retired());
    }
    #[test]
    fn stop_releases_registry_through_the_supervisors_retirement_wait() {
        use std::cell::{Cell,RefCell};
        let (f,job)=assisted();let operation=&job.operation_id;
        let selector=job.application.selector().unwrap();
        let directory=directory(&f.m,&selector).unwrap();
        let mut other=job.application.environment.clone();other.id=random_id().unwrap();
        other.root=f.m.root.join("environments").join(&other.id);
        let lease=selection::tests::dsp_lease(&f,Some(&other));
        let running=json!({"schema":3,"operation_id":operation,"state":"running",
            "launcher_exit":null,"owned_live":2,"cleanup_confirmed":false,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"});
        atomic_json(&job.report,&running).unwrap();
        let gate=directory.join("writer.lock");
        let writer=RefCell::new(Some(f.m.lock(gate.strip_prefix(&f.m.root).unwrap().to_str().unwrap()).unwrap()));
        let submitted=Cell::new(false);let polls=Cell::new(0);
        cancel_with(&f.m,&selector,Some(operation),Duration::from_secs(2),|| {
            if !submitted.get() {
                std::thread::scope(|scope|scope.spawn(|| {
                    let _registry=f.m.lock("registry.lock").expect("initial unit observation never holds registry");
                    assert!(lease.exists());
                }).join().unwrap());
                return Ok(true);
            }
            polls.set(polls.get()+1);
            // A separate status/admission user acquires the real registry
            // while this exact supervisor still owns its lifetime writer.
            std::thread::scope(|scope|scope.spawn(|| {
                let _registry=f.m.lock("registry.lock").expect("Stop must release registry before waiting");
                assert!(lease.exists());
            }).join().unwrap());
            if polls.get()==1 {return Ok(true);}
            let mut retired=running.clone();retired["state"]="cancelled".into();
            retired["owned_live"]=0.into();retired["cleanup_confirmed"]=true.into();
            retired["dependency"]=json!({"process_cleanup_confirmed":true,
                "dependency_cleanup_disposition":"graceful_service_retirement",
                "post_cleanup":{"cgroup_empty":true,"exact_daemon_generation_absent":true,
                    "listener_mask":0,"candidate_count":0,"candidate_observation_unavailable":0,
                    "application_image_unchanged":true,"daemon_image_unchanged":true}});
            atomic_json(&job.report,&retired)?;drop(writer.borrow_mut().take());Ok(false)
        },|| {
            assert!(matches!(f.m.try_lock("registry.lock")?,linux_vst_bridge::operator_lock::LockAttempt::Busy));
            submitted.set(true);Ok(())
        }).unwrap();
        assert!(polls.get()>=2 && lease.exists());
        assert!(result(&f.m,&selector,operation).unwrap().retired());
    }
    #[test]
    fn stop_keeps_exact_target_and_cleanup_failures_uncertain() {
        use std::cell::Cell;
        for case in ["changed_job","changed_result","unit_unavailable","changed_during_observation","enqueue_failed",
            "changed_after_enqueue","cleanup_timeout","missing_dependency_cleanup"] {
            let (f,job)=assisted();let selector=job.application.selector().unwrap();
            let directory=directory(&f.m,&selector).unwrap();
            let running=json!({"schema":3,"operation_id":job.operation_id,"state":"unknown",
                "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,
                "discarded_diagnostic_bytes":0,"account_posture":"unknown"});
            atomic_json(&job.report,&running).unwrap();
            if case=="changed_job" {
                let mut changed=job.clone();changed.operation_id=random_id().unwrap();
                atomic_json(&directory.join("operation.json"),&changed).unwrap();
            }
            if case=="changed_result" {
                let mut changed=running.clone();changed["operation_id"]=random_id().unwrap().into();
                atomic_json(&job.report,&changed).unwrap();
            }
            let submitted=Cell::new(false);let observed=Cell::new(false);
            let error=cancel_with(&f.m,&selector,Some(&job.operation_id),Duration::from_millis(20),|| {
                observed.set(true);
                if case=="unit_unavailable" {return Err("unit_unavailable".into());}
                if case=="changed_during_observation" && !submitted.get() {
                    let mut changed=job.clone();changed.operation_id=random_id().unwrap();
                    atomic_json(&directory.join("operation.json"),&changed)?;
                }
                if submitted.get() {
                    drop(f.m.lock("registry.lock").expect("retirement never holds registry"));
                }
                Ok(!(submitted.get() && case=="missing_dependency_cleanup"))
            },|| {
                submitted.set(true);
                if case=="enqueue_failed" {return Err("enqueue_failed".into());}
                if case=="changed_after_enqueue" {
                    let mut changed=job.clone();changed.operation_id=random_id().unwrap();
                    atomic_json(&directory.join("operation.json"),&changed)?;
                }
                if case=="missing_dependency_cleanup" {
                    let mut changed=running.clone();changed["state"]="cancelled".into();
                    changed["owned_live"]=0.into();changed["cleanup_confirmed"]=true.into();
                    atomic_json(&job.report,&changed)?;
                }
                Ok(())
            }).unwrap_err().to_string();
            assert_eq!(observed.get(),!matches!(case,"changed_job"|"changed_result"),"{case}: {error}");
            assert_eq!(submitted.get(),!matches!(case,"changed_job"|"changed_result"|"unit_unavailable"|"changed_during_observation"),"{case}: {error}");
            assert!(!result(&f.m,&selector,&job.operation_id).is_ok_and(|r|r.retired()),"{case}");
            assert_eq!(error,match case {
                "unit_unavailable"=>"unit_unavailable","enqueue_failed"=>"enqueue_failed",
                "cleanup_timeout"|"missing_dependency_cleanup"=>"vendor_application_cleanup_unconfirmed",
                _=>"vendor_application_operation_changed",
            },"{case}");
        }
    }
    #[test]
    fn missing_dispatch_result_remains_readable_and_requires_exact_empty_writer_recovery() {
        let (f,selector,operation,_)=selected();let directory=directory(&f.m,&selector).unwrap();
        let before=fs::read(directory.join("operation.json")).unwrap();
        assert!(!result(&f.m,&selector,&operation).unwrap().retired());
        let unmanaged=|_:&serde_json::Value|->std::result::Result<serde_json::Value,RecoverRefusal> {panic!("Ordinary programs have no service census")};
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

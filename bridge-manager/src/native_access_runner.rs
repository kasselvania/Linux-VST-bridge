//! Explicit, reversible runner transition for the existing Native Access prefix.
use super::*;
use linux_vst_bridge::renderer_application as app;
use serde_json::json;
use std::os::fd::AsRawFd;

pub fn update(m: &Manager, input: &Path) -> Result<()> {
    let runner: Runner = read_json(input)?;
    runner.verify()?;
    let _canonical = m.lock("operator-canonical.lock")?;
    let _registry = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(renderer_cli::all_retired(m)? && dependency_cli::all_retired(m)?,
        "native_access_runner_session_active")?;
    let state = Command::new("systemctl").args(["--user", "show",
        "linux-vst-bridge.service", "--property=ActiveState", "--value"]).output()?;
    require(state.status.success() && state.stdout == b"inactive\n",
        "native_access_runner_stop_bridge_first")?;
    require(!m.registry()?.classes.values().any(|v|v.registration.environment.id==app::ENVIRONMENT),
        "native_access_runner_published_environment")?;
    let path = m.root.join("vendor-applications/native-access/application.json");
    let before: app::Application = read_json(&path)?;
    before.verify(&m.root)?;
    let operation = fs::OpenOptions::new().read(true).write(true)
        .open(before.environment.root.join("operation.lock"))?;
    require(unsafe {libc::flock(operation.as_raw_fd(),libc::LOCK_EX|libc::LOCK_NB)}==0,
        "native_access_runner_environment_busy")?;
    let mut onboarding = onboarding::load(m, app::ENVIRONMENT)?;
    require(onboarding.environment==before.environment,"native_access_runner_onboarding")?;
    require(runner != before.environment.runner,"native_access_runner_unchanged")?;
    let mut after=before.clone();
    after.environment.runner=runner;
    after.environment.revision=before.environment.revision.checked_add(1).ok_or("environment_revision_overflow")?;
    require(after.same_installation(&before),"native_access_runner_installation_changed")?;
    let environment=before.environment.root.join("environment.json");
    let record=m.root.join("onboarding").join(app::ENVIRONMENT).join("record.json");
    let backup=m.root.join("private-rollback").join(format!("native-access-runner-{}",random_id()?));
    private_dir(&backup)?;
    let originals=[(&environment,"environment.json"),(&record,"record.json"),(&path,"application.json")]
        .into_iter().map(|(p,name)|Ok((p, name, fs::read(p)?))).collect::<Result<Vec<_>>>()?;
    for (_,name,bytes) in &originals {
        let mut out=fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(backup.join(name))?;
        out.write_all(bytes)?;out.sync_all()?;
    }
    atomic_json(&backup.join("transition.json"),&json!({"schema":1,"state":"prepared",
        "before":before.environment,"after":after.environment,
        "prefix_recreated":false,"historical_results_rewritten":false}))?;
    onboarding.environment=after.environment.clone();
    let changed=(|| -> Result<()> {
        atomic_json(&environment,&after.environment)?;
        atomic_json(&record,&onboarding)?;
        atomic_json(&path,&after)?;
        after.verify(&m.root)?;
        onboarding::load(m,app::ENVIRONMENT)?;
        native_access_dependency::session_admission(m,&after,&software(m)?)?;
        Ok(())
    })();
    if let Err(error)=changed {
        for (p,_,bytes) in &originals {atomic_json(p,&serde_json::from_slice::<serde_json::Value>(bytes)?)?;}
        atomic_json(&backup.join("result.json"),&json!({"state":"rolled_back","error":error.to_string()}))?;
        return Err(error);
    }
    atomic_json(&backup.join("result.json"),&json!({"state":"completed","application":after.identity()?}))?;
    println!("{}",json!({"environment":after.environment.id,"revision":after.environment.revision,
        "runner":after.environment.runner.id,"rollback":backup,"application":after.identity()?}));
    Ok(())
}

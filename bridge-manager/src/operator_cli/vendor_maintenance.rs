//! Setup exclusivity belongs to the existing vendor job across its children.
//! The job-local lifecycle leaves shipped Intent and terminal receipt bytes intact.
use super::*;
use environment_revision::VendorBinding;

const STATE: &str = "vendor-maintenance-state.json";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct State {
    schema: u32,
    owner: String,
    executor: String,
    request: Artifact,
    environment: Environment,
    vendor_operation: String,
    held: bool,
}
fn state_for(m: &Manager, binding: &VendorBinding, intent: &automatic_install::Intent,
    held: bool) -> Result<State> {
    require(intent.owner == binding.operation && intent.vendor.as_ref().is_some_and(|v|
        v.application == binding.application && v.operation == binding.vendor_operation
            && v.selected == binding.selected), "vendor_maintenance_owner_binding")?;
    let request=control_artifact(job_dir(m,&intent.active_operation)?.join("request.json"))?;
    let input:ui::Request=read_json(&request.path)?;
    require(matches!(&input.action,
        ui::Action::VendorSoftwareInstall {application,..}|ui::Action::VendorApplicationOpen {application}
            if *application==binding.application)
        || matches!(&input.action,ui::Action::SoftwareInstallResume {onboarding}
            if *onboarding==binding.environment.id),"vendor_maintenance_executor_binding")?;
    Ok(State {schema:1,owner:intent.owner.clone(),executor:intent.active_operation.clone(),
        request,
        environment:binding.environment.clone(),vendor_operation:binding.vendor_operation.clone(),held})
}
fn intent(m: &Manager, binding: &VendorBinding) -> Result<automatic_install::Intent> {
    let intent = automatic_install::vendor_intent(m,&binding.application)?
        .ok_or("vendor_maintenance_intent_missing")?;
    require(intent.owner == binding.operation,"vendor_maintenance_owner_changed")?;
    Ok(intent)
}
fn state(m: &Manager, binding: &VendorBinding, intent: &automatic_install::Intent) -> Result<Option<State>> {
    let path=job_dir(m,&binding.operation)?.join(STATE);
    if !path.try_exists()? {return Ok(None)}
    let state:State=read_json(&path)?;
    require(state == state_for(m,binding,intent,state.held)?,"vendor_maintenance_generation_changed")?;
    Ok(Some(state))
}
fn same_setup(a:&Environment,b:&Environment)->bool {a.id==b.id || a.root==b.root}
/// Caller holds registry.lock. Missing lifecycle cannot establish retirement.
pub(crate) fn blocks(m:&Manager, environment:&Environment)->Result<bool> {
    for binding in automatic_install::current_vendor_bindings(m)? {
        if !same_setup(&binding.environment,environment) {continue}
        let intent=intent(m,&binding)?;
        if state(m,&binding,&intent)?.is_none_or(|state|state.held) {return Ok(true)}
    }
    Ok(false)
}
/// Resolve a child through its root's retained intent, never a caller's claimed setup.
pub(crate) fn owned(m:&Manager, operation:&str)->Result<Option<Environment>> {
    for binding in automatic_install::current_vendor_bindings(m)? {
        let intent=intent(m,&binding)?;
        if operation != intent.active_operation && !intent.work.iter().any(|work|
            work.active_operation.as_deref()==Some(operation)) {continue}
        let Some(state)=state(m,&binding,&intent)? else {continue};
        if !state.held {return Ok(None)}
        require(read_json::<Environment>(&binding.environment.root.join("environment.json"))?
            ==binding.environment,"vendor_maintenance_environment_changed")?;
        return Ok(Some(binding.environment));
    }
    Ok(None)
}
pub(crate) fn for_vendor(m:&Manager,operation:&str)->Result<Option<Environment>> {
    for binding in automatic_install::current_vendor_bindings(m)? {
        if binding.vendor_operation!=operation {continue}
        let intent=intent(m,&binding)?;
        return owned(m,&intent.active_operation);
    }
    Ok(None)
}
pub(crate) fn any_held(m:&Manager)->Result<bool> {
    for binding in automatic_install::current_vendor_bindings(m)? {
        if blocks(m,&binding.environment)? {return Ok(true)}
    }
    Ok(false)
}
pub(crate) fn action_environment(m:&Manager,action:&ui::Action)->Result<Option<Environment>> {
    match action {
        ui::Action::VendorSoftwareInstall {application,..} | ui::Action::VendorApplicationOpen {application} => {
            let app=vendor_cli::application(m,application)?;
            Ok(if app.user_selection.is_none() {Some(app.environment)} else {None})
        },
        ui::Action::EnvironmentInstallerScan {environment} | ui::Action::QuarantinedModuleRetry {environment,..} => Ok(Some(environment_install::target(m,environment)?)),
        _ if preparation_cli::is_action(action) => Ok(preparation_cli::action_selection(m,&software(m)?,action)?
            .map(|selection|selection.environment)),
        _=>Ok(None),
    }
}
/// Caller holds registry.lock; the existing binding has been retained first.
pub(crate) fn acquire(m:&Manager,binding:&VendorBinding,intent:&automatic_install::Intent)->Result<()> {
    capacity::require_environment_inactive(m,&binding.environment)?;
    for other in automatic_install::current_vendor_bindings(m)? {
        if other.operation==binding.operation || !same_setup(&other.environment,&binding.environment) {continue}
        let prior=intent_for_binding(m,&other)?;
        require(state(m,&other,&prior)?.is_some_and(|s|!s.held),"vendor_maintenance_other_owner")?;
    }
    atomic_json(&job_dir(m,&binding.operation)?.join(STATE),&state_for(m,binding,intent,true)?)
}
fn intent_for_binding(m:&Manager,binding:&VendorBinding)->Result<automatic_install::Intent> {intent(m,binding)}
fn custody_retired(m:&Manager,binding:&VendorBinding)->Result<()> {
    for owner in capacity::owners_for(m,Some(&binding.environment))? {
        let scope=m.lease_environment(&owner.session)?;
        require(!same_setup(&scope,&binding.environment),"vendor_maintenance_host_cleanup_unconfirmed")?;
    }
    Ok(())
}
/// Called after the synchronous executor has finished every child, on success
/// or clean failure. Failed work and pending publications remain untouched.
pub(crate) fn settle_executor(m:&Manager,executor:&str)->Result<()> {
    for binding in automatic_install::current_vendor_bindings(m)? {
        let intent=intent(m,&binding)?;
        if intent.active_operation!=executor {continue}
        if state(m,&binding,&intent).is_ok_and(|s|s.is_some_and(|s|!s.held)) {continue}
        let retirement=observe_retired(m,&binding.environment)?;
        let _registry=m.lock("registry.lock")?;
        let fresh=intent_for_binding(m,&binding)?;
        require(fresh.active_operation==executor,"vendor_maintenance_generation_changed")?;
        retirement.verify(m)?;
        let current:Environment=read_json(&binding.environment.root.join("environment.json"))?;
        if current!=binding.before {environment_revision::require_owned_revision(m,&binding.environment,&current)?;}
        custody_retired(m,&binding)?;
        atomic_json(&job_dir(m,&binding.operation)?.join(STATE),&state_for(m,&binding,&intent,false)?)?;
    }
    Ok(())
}
/// Existing recovery calls this only after positively observing the worker dead.
#[derive(Default)]
pub(crate) struct Recovery {
    pub settled: usize,
    pub failures: Vec<String>,
}
pub(crate) fn reconcile(m:&Manager,mut worker_live:impl FnMut(&str)->Result<bool>)->Result<Recovery> {
    let mut recovery=Recovery::default();
    for binding in automatic_install::current_vendor_bindings(m)? {
        let result=(||->Result<bool> {
        let intent=intent(m,&binding)?;
        if state(m,&binding,&intent).is_ok_and(|s|s.is_some_and(|s|!s.held)) {return Ok(false)}
        if worker_live(&intent.active_operation)? {return Ok(false)}
        let Ok(_worker)=m.lock(&format!("operator-worker-{}.lock",intent.active_operation)) else {return Ok(false)};
        if worker_live(&intent.active_operation)? {return Ok(false)}
        let retirement=observe_retired(m,&binding.environment)?;
        let _registry=m.lock("registry.lock")?;
        retirement.verify(m)?;
        let lease_root=m.root.join("runtime/leases");
        if lease_root.try_exists()? {
            let boot=kernel_boot()?;
            for entry in fs::read_dir(lease_root)? {
                let path=entry?.path();
                let Some(session)=path.file_stem().and_then(|v|v.to_str()) else {continue};
                if m.lease_environment(session).is_ok_and(|scope|same_setup(&scope,&binding.environment)) {
                    reconcile_lease_in_kernel(m,&path,boot.as_deref())?;
                }
            }
        }
        let current=intent_for_binding(m,&binding)?;
        require(current.active_operation==intent.active_operation,"vendor_maintenance_generation_changed")?;
        custody_retired(m,&binding)?;
        let environment:Environment=read_json(&binding.environment.root.join("environment.json"))?;
        if environment!=binding.before {environment_revision::require_owned_revision(m,&binding.environment,&environment)?;}
        atomic_json(&job_dir(m,&binding.operation)?.join(STATE),&state_for(m,&binding,&current,false)?)?;
        Ok(true)
        })();
        match result {
            Ok(true)=>recovery.settled+=1,
            Ok(false)=>{},
            Err(error)=>recovery.failures.push(format!("{}: {error}",binding.environment.id)),
        }
    }
    Ok(recovery)
}
thread_local! {
    static EXECUTOR: std::cell::RefCell<Option<String>> = const {std::cell::RefCell::new(None)};
}
pub(crate) fn with_executor<T>(operation:&str,run:impl FnOnce()->T)->T {
    struct Restore(Option<String>);
    impl Drop for Restore {fn drop(&mut self) {EXECUTOR.with(|value|*value.borrow_mut()=self.0.take());}}
    let prior=EXECUTOR.with(|value|value.replace(Some(operation.into())));
    let _restore=Restore(prior);run()
}
/// The thread-local value selects an existing owner; it grants no authority.
/// Every use revalidates its durable job and exact setup under registry.lock.
pub(crate) fn require_inactive(m:&Manager,environment:&Environment)->Result<()> {
    let executor=EXECUTOR.with(|value|value.borrow().clone());
    if let Some(bound)=executor.as_deref().map(|id|owned(m,id)).transpose()?.flatten() {
        require(bound==*environment,"vendor_maintenance_child_scope")?;
        capacity::require_environment_inactive(m,environment)
    } else {
        require(!blocks(m,environment)?,"setup_maintenance_active")?;
        m.require_inactive(None)
    }
}
/// Receipts in other setups do not own this setup's maintenance admission.
pub(crate) struct RetirementObservation {
    environment: Environment,
    selected: Vec<String>,
    files: Vec<(PathBuf,Option<Artifact>)>,
}
impl RetirementObservation {
    pub(crate) fn verify(&self,m:&Manager)->Result<()> {
        require(scoped_selectors(m,&self.environment)?==self.selected,"setup_vendor_selection_changed")?;
        for (path,expected) in &self.files {
            let current=if path.try_exists()? {Some(control_artifact(path.clone())?)} else {None};
            require(current==*expected,"setup_vendor_retirement_changed")?;
        }
        require_installers_retired(m,&self.environment)
    }
}
fn scoped_selectors(m:&Manager,environment:&Environment)->Result<Vec<String>> {
    vendor_cli::selectors(m)?.into_iter().filter_map(|selector| {
        match vendor_cli::application(m,&selector) {
            Ok(app) if same_setup(&app.environment,environment)=>Some(Ok(selector)),
            Ok(_)=>None,Err(error)=>Some(Err(error)),
        }
    }).collect()
}
/// External service observations happen before registry admission; exact files
/// and the selector inventory are checked again under that admission.
pub(crate) fn observe_retired(m:&Manager,environment:&Environment)->Result<RetirementObservation> {
    let selected=scoped_selectors(m,environment)?;
    let mut files=vec![];
    for selector in &selected {
        let directory=vendor_cli::directory(m,selector)?;
        for name in ["application.json","operation.json","operation-result.json"] {
            let path=directory.join(name);
            files.push((path.clone(),if path.try_exists()? {Some(control_artifact(path)?)} else {None}));
        }
        require(!vendor_cli::live(selector)?,"setup_vendor_application_live")?;
        let path=directory.join("operation.json");
        if path.try_exists()? {
            let job:vendor_application::Operation=read_json(&path)?;
            require(vendor_cli::result(m,selector,&job.operation_id)?.retired(),"setup_vendor_cleanup_unconfirmed")?;
        }
    }
    for (directory,retired) in [
        (renderer_session::directory(m),renderer_session::retired as fn(&Manager,&str)->Result<bool>),
        (linux_vst_bridge::dependency_session::directory(m),linux_vst_bridge::dependency_session::retired),
    ] {
        let pointer=directory.join("current.json");
        let captured=if pointer.try_exists()? {Some(control_artifact(pointer.clone())?)} else {None};
        files.push((pointer.clone(),captured));
        if !pointer.try_exists()? {continue}
        let current:Value=read_json(&pointer)?;
        let operation=current["operation"].as_str().ok_or("setup_legacy_operation_identity")?;
        require(valid_hex(operation,32),"setup_legacy_operation_identity")?;
        let job=directory.join("operations").join(operation);
        let spec:Value=read_json(&job.join("spec.json"))?;
        let application:linux_vst_bridge::renderer_application::Application=
            serde_json::from_value(spec["application"].clone())?;
        if !same_setup(&application.environment,environment) {continue}
        require(spec["operation"]==operation,"setup_legacy_operation_changed")?;
        for name in ["spec.json","result.json","recovery-result.json"] {
            let path=job.join(name);
            files.push((path.clone(),if path.try_exists()? {Some(control_artifact(path)?)} else {None}));
        }
        require(retired(m,operation)?,"setup_legacy_cleanup_unconfirmed")?;
    }
    let observation=RetirementObservation {environment:environment.clone(),selected,files};
    observation.verify(m)?;
    Ok(observation)
}
fn require_installers_retired(m:&Manager,environment:&Environment)->Result<()> {
    for record in onboarding::history_records(m)? {
        if same_setup(&record.environment,environment) && record.installation_operation.is_some() {
            require(onboarding::retired(&onboarding::result(m,&record)?),"setup_installer_cleanup_unconfirmed")?;
        }
    }
    for record in environment_install::records(m)? {
        if same_setup(&record.environment,environment) {
            require(onboarding::retired(&environment_install::result(m,&record)?),"setup_installer_cleanup_unconfirmed")?;
        }
    }
    for record in uninstall::records(m)? {
        if same_setup(&record.environment,environment) {
            require(onboarding::retired(&uninstall::result_of(m,&record.operation)?),"setup_uninstaller_cleanup_unconfirmed")?;
        }
    }
    Ok(())
}

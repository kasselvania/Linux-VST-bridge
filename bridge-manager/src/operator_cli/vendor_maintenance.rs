//! Setup exclusivity belongs to the existing installation job across its children.
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
/// A view of existing retained owners, never another registry or lease table.
#[derive(Clone)]
enum Target { Vendor(VendorBinding), Onboarding, Installer, InstallerReservation }
#[derive(Clone)]
struct Binding {
    operation:String,before:Environment,environment:Environment,
    vendor_operation:String,target:Target,
}
impl From<VendorBinding> for Binding {
    fn from(v:VendorBinding)->Self {Self {operation:v.operation.clone(),before:v.before.clone(),
        environment:v.environment.clone(),vendor_operation:v.vendor_operation.clone(),target:Target::Vendor(v)}}
}
#[derive(Clone)]
struct Execution {
    owner:String,active_operation:String,children:Vec<String>,
    intent:Option<automatic_install::Intent>,
}
fn initial_record(m:&Manager,id:&str)->Result<onboarding::Record> {
    let r:onboarding::Record=read_json(&onboarding::directory(m,id)?.join("record.json"))?;
    require(r.schema==1 && r.id==id && r.environment.id==id
        && r.environment.root==m.root.join("environments").join(id)
        && valid_hex(&r.creation_operation,32),"setup_maintenance_initial_binding")?;
    Ok(r)
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
struct Enrollment {schema:u32,request:Artifact,software:Artifact,environment:Environment}
fn enrollment(m:&Manager,operation:&str)->Result<Option<Enrollment>> {
    let receipt=optional(&job_dir(m,operation)?.join("result.json"))?;
    let Some(value)=receipt.get("setup_custody") else {return Ok(None)};
    let enrolled:Enrollment=serde_json::from_value(value.clone())?;
    validate_enrollment(m,operation,&enrolled)?;
    Ok(Some(enrolled))
}
fn validate_enrollment(m:&Manager,operation:&str,enrolled:&Enrollment)->Result<()> {
    require(enrolled.schema==1 && enrolled.request.path==job_dir(m,operation)?.join("request.json")
        && valid_hex(&enrolled.software.sha256,64) && enrolled.software.path.is_absolute()
        && valid_product_environment(&enrolled.environment.id)
        && enrolled.environment.root==m.root.join("environments").join(&enrolled.environment.id),
        "setup_maintenance_enrollment_binding")?;
    let request:ui::Request=serde_json::from_slice(&enrolled.request.record_bytes(16384)?)?;
    require(request.action.software_install().is_some()
        || request.action.environment_installer_start().is_some_and(|(environment,_,_)|
            environment==enrolled.environment.id)
        || matches!(&request.action,ui::Action::SoftwareInstallResume {onboarding} if *onboarding==enrolled.environment.id)
        || matches!(&request.action,ui::Action::EnvironmentRefreshResume {operation}
            if environment_install::load(m,operation)?.environment==enrolled.environment),
        "setup_maintenance_enrollment_binding")
}
pub(crate) fn enrolled(m:&Manager,operation:&str)->Result<bool> {Ok(enrollment(m,operation)?.is_some())}
/// Extend the existing untyped job receipt. Retained managers ignore this field;
/// their strict onboarding/installer record shapes remain completely unchanged.
pub(super) fn enroll(m:&Manager,operation:&str,environment:&Environment)->Result<()> {
    if let Some(prior)=enrollment(m,operation)? {
        return require(prior.environment==*environment,"setup_maintenance_enrollment_binding");
    }
    let metadata=Enrollment {schema:1,request:control_artifact(job_dir(m,operation)?.join("request.json"))?,
        software:software_record(m)?.manager,environment:environment.clone()};
    validate_enrollment(m,operation,&metadata)?;
    let mut receipt=optional(&job_dir(m,operation)?.join("result.json"))?;
    if receipt.is_null() {receipt=json!({"schema":1,"operation":operation,"state":"running"});}
    receipt["setup_custody"]=serde_json::to_value(metadata)?;
    write_operation(m,operation,&receipt,false)?;
    require(enrolled(m,operation)?,"setup_maintenance_enrollment_missing")
}
fn enrollments(m:&Manager)->Result<Vec<(String,ui::Action,Enrollment)>> {
    let root=m.root.join("operator");if !root.try_exists()? {return Ok(vec![])}
    let mut result=vec![];
    for (n,entry) in fs::read_dir(root)?.enumerate() {
        require(n<4096,"operator_setup_history_bound")?;
        let entry=entry?;let id=entry.file_name().to_string_lossy().into_owned();
        if !valid_hex(&id,32) {continue;}
        if let Some(enrollment)=enrollment(m,&id)? {
            let request:ui::Request=read_json(&enrollment.request.path)?;
            result.push((id,request.action,enrollment));
        }
    }
    Ok(result)
}
fn bindings(m:&Manager)->Result<Vec<Binding>> {
    let mut result:Vec<_>=automatic_install::current_vendor_bindings(m)?.into_iter().map(Binding::from).collect();
    let enrolled=enrollments(m)?;
    // A validated deletion already claimed/quiesced these setups. Unreadable
    // markers hide launch offers, but cannot establish custody retirement.
    let deleted:std::collections::BTreeSet<_>=deleted_setup::records(m)?.into_iter()
        .flat_map(|record|record.environments.into_iter().map(|environment|environment.id)).collect();
    let mut initial:std::collections::BTreeSet<_>=onboarding::history_records(m)?.into_iter().map(|record|record.id).collect();
    initial.extend(enrolled.iter().filter(|(_,action,enrollment)|!deleted.contains(&enrollment.environment.id)
        && (action.software_install().is_some() || matches!(action,ui::Action::SoftwareInstallResume {..})))
        .map(|(_,_,enrollment)|enrollment.environment.id.clone()));
    for id in initial {
        let record=initial_record(m,&id)?;
        if let Some(intent)=record.automatic.filter(|i|i.vendor.is_none()) {
            let known=enrolled.iter().any(|(owner,action,_)|*owner==intent.owner
                || matches!(action,ui::Action::SoftwareInstallResume {onboarding} if *onboarding==record.id));
            if !known && !job_dir(m,&intent.owner)?.join(STATE).try_exists()? {continue;}
            result.push(Binding {operation:intent.owner.clone(),before:record.environment.clone(),
                environment:record.environment,vendor_operation:intent.owner,target:Target::Onboarding});
        }
    }
    for record in environment_install::records(m)? {
        if deleted.contains(&record.environment.id) {continue;}
        let known=enrolled.iter().any(|(owner,action,_)|*owner==record.operation
            || matches!(action,ui::Action::EnvironmentRefreshResume {operation} if *operation==record.operation));
        if !known && !job_dir(m,&record.operation)?.join(STATE).try_exists()? {continue;}
        result.push(Binding {operation:record.operation.clone(),before:record.before,
            environment:record.environment,vendor_operation:record.operation,target:Target::Installer});
    }
    for (operation,action,enrollment) in enrolled {
        if deleted.contains(&enrollment.environment.id) {continue;}
        if action.environment_installer_start().is_none()
            || job_dir(m,&operation)?.join("installation.json").try_exists()? {continue;}
        require(action.environment_installer_start().is_some_and(|(environment,_,_)|
            environment==enrollment.environment.id),"setup_maintenance_enrollment_binding")?;
        result.push(Binding {operation:operation.clone(),before:enrollment.environment.clone(),
            environment:enrollment.environment,vendor_operation:operation,target:Target::InstallerReservation});
    }
    result.sort_by(|a,b|a.operation.cmp(&b.operation));
    Ok(result)
}
pub(crate) fn state_paths(m:&Manager)->Result<Vec<PathBuf>> {
    let mut paths=vec![];
    for binding in bindings(m)? {
        paths.push(job_dir(m,&binding.operation)?.join(STATE));
        if !matches!(binding.target,Target::Vendor(_)) {
            paths.push(job_dir(m,&intent(m,&binding)?.active_operation)?.join("result.json"));
        }
    }
    Ok(paths)
}
pub(crate) fn installation_authority(m:&Manager)->Result<Vec<Value>> {
    let mut rows=vec![];
    for binding in bindings(m)?.into_iter().filter(|b|!matches!(b.target,Target::Vendor(_))) {
        let execution=intent(m,&binding)?;
        let path=job_dir(m,&binding.operation)?.join(STATE);
        let state=if path.try_exists()? {Some(control_artifact(path)?)} else {None};
        rows.push(json!({"owner":binding.operation,"environment":binding.environment,
            "enrollment":enrollment(m,&execution.active_operation)?,"maintenance":state}));
    }
    Ok(rows)
}
fn intent(m:&Manager,binding:&Binding)->Result<Execution> {
    let intent=match &binding.target {
        Target::Vendor(v)=>Some(automatic_install::vendor_intent(m,&v.application)?
            .ok_or("vendor_maintenance_intent_missing")?),
        Target::Onboarding=>Some(initial_record(m,&binding.environment.id)?.automatic
            .ok_or("vendor_maintenance_intent_missing")?),
        Target::Installer=>if job_dir(m,&binding.operation)?.join("refresh.json").try_exists()? {
            Some(environment_refresh::load(m,&binding.operation)?.intent)
        } else {None},
        Target::InstallerReservation=>None,
    };
    require(intent.as_ref().is_none_or(|i|i.owner==binding.operation),"vendor_maintenance_owner_changed")?;
    Ok(Execution {owner:binding.operation.clone(),active_operation:intent.as_ref()
        .map_or_else(||binding.operation.clone(),|i|i.active_operation.clone()),
        children:intent.as_ref().map(|i|i.work.iter().filter_map(|w|w.active_operation.clone()).collect()).unwrap_or_default(),intent})
}
fn state_for(m:&Manager,binding:&Binding,execution:&Execution,held:bool)->Result<State> {
    require(execution.owner==binding.operation,"vendor_maintenance_owner_binding")?;
    let request=control_artifact(job_dir(m,&execution.active_operation)?.join("request.json"))?;
    let input:ui::Request=read_json(&request.path)?;
    let valid=match &binding.target {
        Target::Vendor(v)=> {
            require(execution.intent.as_ref().and_then(|i|i.vendor.as_ref()).is_some_and(|i|
                i.application==v.application && i.operation==v.vendor_operation && i.selected==v.selected),
                "vendor_maintenance_owner_binding")?;
            matches!(&input.action,ui::Action::VendorSoftwareInstall {application,..}
                |ui::Action::VendorApplicationOpen {application} if *application==v.application)
                || matches!(&input.action,ui::Action::SoftwareInstallResume {onboarding} if *onboarding==binding.environment.id)
        },
        Target::Onboarding=> {
            let i=execution.intent.as_ref().ok_or("setup_maintenance_owner_binding")?;
            let enrolled=enrollment(m,&execution.active_operation)?.ok_or("setup_maintenance_owner_binding")?;
            require(i.vendor.is_none() && enrolled.environment==binding.environment,"setup_maintenance_owner_binding")?;
            let record=initial_record(m,&binding.environment.id)?;
            require(environment_revision::same_space(&binding.environment,&record.environment),"setup_maintenance_owner_binding")?;
            automatic_install::initial_installer_input(m,&record,i)?;
            (execution.active_operation==execution.owner && input.action.software_install()
                .is_some_and(|(installer,_,_)|installer==record.installer))
                || matches!(&input.action,ui::Action::SoftwareInstallResume {onboarding} if *onboarding==record.id)
        },
        Target::Installer=> {
            let r=environment_install::load(m,&binding.operation)?;
            let enrolled=enrollment(m,&execution.active_operation)?.ok_or("setup_maintenance_owner_binding")?;
            require(enrolled.environment==if execution.active_operation==execution.owner {r.before.clone()} else {r.environment.clone()}
                && r.environment==binding.environment,"setup_maintenance_owner_binding")?;
            (execution.active_operation==execution.owner && input.action.environment_installer_start()
                .is_some_and(|(environment,installer,_)|environment==r.environment.id && installer==r.installer))
                || matches!(&input.action,ui::Action::EnvironmentRefreshResume {operation} if *operation==binding.operation)
        },
        Target::InstallerReservation=> {
            let enrolled=enrollment(m,&execution.active_operation)?.ok_or("setup_maintenance_owner_binding")?;
            require(execution.active_operation==execution.owner && enrolled.environment==binding.environment,
                "setup_maintenance_owner_binding")?;
            input.action.environment_installer_start().is_some_and(|(environment,installer,_)|
                environment==binding.environment.id && valid_hex(installer,64))
        },
    };
    require(valid,"vendor_maintenance_executor_binding")?;
    Ok(State {schema:1,owner:execution.owner.clone(),executor:execution.active_operation.clone(),request,
        environment:binding.environment.clone(),vendor_operation:binding.vendor_operation.clone(),held})
}
fn state(m: &Manager, binding: &Binding, intent: &Execution) -> Result<Option<State>> {
    let path=job_dir(m,&binding.operation)?.join(STATE);
    if !path.try_exists()? {return Ok(None)}
    let state:State=read_json(&path)?;
    let mut historic=binding.clone();let mut executor=intent.clone();
    if !state.held && !matches!(binding.target,Target::Vendor(_)) {
        if state.executor!=intent.active_operation {
            require(!enrolled(m,&intent.active_operation)?,"vendor_maintenance_generation_changed")?;
            let request:ui::Request=read_json(&job_dir(m,&intent.active_operation)?.join("request.json"))?;
            let resumed=match &binding.target {
                Target::Onboarding=>matches!(&request.action,ui::Action::SoftwareInstallResume {onboarding}
                    if *onboarding==binding.environment.id),
                Target::Installer=>matches!(&request.action,ui::Action::EnvironmentRefreshResume {operation}
                    if *operation==binding.operation),
                Target::Vendor(_)|Target::InstallerReservation=>false,
            };
            let receipt=optional(&job_dir(m,&intent.active_operation)?.join("result.json"))?;
            require(resumed && receipt["operation"]==intent.active_operation
                && matches!(receipt["state"].as_str(),Some("completed"|"refused")),
                "setup_maintenance_historic_executor_unretired")?;
            // A live worker owns this existing lock. No systemctl under registry.
            let _retired=m.lock(&format!("operator-worker-{}.lock",intent.active_operation))?;
        }
        require(same_setup(&state.environment,&binding.environment),"setup_maintenance_environment_changed")?;
        environment_revision::require_owned_revision(m,&state.environment,&read_json::<Environment>(
            &binding.environment.root.join("environment.json"))?)?;
        historic.environment=state.environment.clone();executor.active_operation=state.executor.clone();
    }
    require(state == state_for(m,&historic,&executor,state.held)?,"vendor_maintenance_generation_changed")?;
    Ok(Some(state))
}
fn same_setup(a:&Environment,b:&Environment)->bool {a.id==b.id || a.root==b.root}
/// Caller holds registry.lock. Missing lifecycle cannot establish retirement.
pub(crate) fn blocks(m:&Manager, environment:&Environment)->Result<bool> {
    for binding in bindings(m)? {
        if !same_setup(&binding.environment,environment) {continue}
        let intent=intent(m,&binding)?;
        if state(m,&binding,&intent)?.is_none_or(|state|state.held) {return Ok(true)}
    }
    Ok(false)
}
/// The accepted installer may convert only its own unstarted reservation into
/// the existing typed installation binding, while every other owner still holds.
pub(crate) fn admit_installer_reservation(m:&Manager,operation:&str,environment:&Environment)->Result<()> {
    for binding in bindings(m)? {
        if !same_setup(&binding.environment,environment) {continue;}
        if binding.operation==operation && matches!(binding.target,Target::InstallerReservation) {
            require(binding.environment==*environment && !job_dir(m,operation)?.join(STATE).try_exists()?,
                "installer_transaction_already_used")?;
            continue;
        }
        require(state(m,&binding,&intent(m,&binding)?)?.is_some_and(|s|!s.held),"setup_maintenance_active")?;
    }
    Ok(())
}
/// Resolve a child through its root's retained intent, never a caller's claimed setup.
pub(crate) fn owned(m:&Manager, operation:&str)->Result<Option<Environment>> {
    for binding in bindings(m)? {
        let intent=intent(m,&binding)?;
        if operation != intent.active_operation && !intent.children.iter().any(|child|child==operation) {continue}
        let Some(state)=state(m,&binding,&intent)? else {continue};
        if !state.held {return Ok(None)}
        require(read_json::<Environment>(&binding.environment.root.join("environment.json"))?
            ==binding.environment,"vendor_maintenance_environment_changed")?;
        return Ok(Some(binding.environment));
    }
    Ok(None)
}
pub(crate) fn for_vendor(m:&Manager,operation:&str)->Result<Option<Environment>> {
    for binding in bindings(m)? {
        if !matches!(binding.target,Target::Vendor(_)) || binding.vendor_operation!=operation {continue}
        let intent=intent(m,&binding)?;
        return owned(m,&intent.active_operation);
    }
    Ok(None)
}
pub(crate) fn any_held(m:&Manager)->Result<bool> {
    for binding in bindings(m)? {
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
        ui::Action::InstallerStart {onboarding} | ui::Action::InstallerStartWithPolicy {onboarding,..}
        | ui::Action::InstallerScan {onboarding} => Ok(Some(onboarding::load(m,onboarding)?.environment)),
        ui::Action::SoftwareInstallResume {onboarding} => {
            let record=onboarding::retained_environment_record(m,onboarding)?.ok_or("automatic_installation_record_missing")?;
            let intent=automatic_install::record_intent(m,&record)?;
            Ok(Some(if let Some(vendor)=intent.and_then(|i|i.vendor) {
                vendor_cli::application(m,&vendor.application)?.environment
            } else {record.environment}))
        },
        ui::Action::EnvironmentRefreshResume {operation}=>Ok(Some(environment_refresh::load(m,operation)?.environment)),
        action if action.environment_installer_start().is_some()=>Ok(Some(environment_install::target(m,
            action.environment_installer_start().unwrap().0)?)),
        ui::Action::EnvironmentRescan {environment} | ui::Action::EnvironmentInstallerScan {environment} | ui::Action::QuarantinedModuleRetry {environment,..} => Ok(Some(environment_install::target(m,environment)?)),
        _ if preparation_cli::is_action(action) => Ok(preparation_cli::action_selection(m,&software(m)?,action)?
            .map(|selection|selection.environment)),
        _=>Ok(None),
    }
}
/// Caller holds registry.lock; the existing binding has been retained first.
pub(crate) fn acquire(m:&Manager,binding:&VendorBinding,expected:&automatic_install::Intent)->Result<()> {
    let binding=Binding::from(binding.clone());let execution=intent(m,&binding)?;
    require(execution.active_operation==expected.active_operation,"vendor_maintenance_executor_binding")?;
    acquire_binding(m,&binding,&execution)
}
/// The existing accepted job receipt enrolls custody. Its missing lifecycle
/// refuses new admission without changing strict historical record shapes.
pub(crate) fn acquire_onboarding(m:&Manager,record:&onboarding::Record,intent:&automatic_install::Intent)->Result<()> {
    let binding=Binding {operation:intent.owner.clone(),before:record.environment.clone(),
        environment:record.environment.clone(),vendor_operation:intent.owner.clone(),target:Target::Onboarding};
    let retirement=observe_job_retired(m,&binding)?;
    enroll(m,&intent.active_operation,&record.environment)?;
    let _registry=m.lock("registry.lock")?;
    retirement.verify(m)?;
    let current=initial_record(m,&record.id)?;
    require(current.environment==record.environment,"setup_maintenance_environment_changed")?;
    let execution=self::intent(m,&binding)?;
    require(execution.active_operation==intent.active_operation,"vendor_maintenance_executor_binding")?;
    acquire_binding(m,&binding,&execution)
}
pub(crate) fn acquire_installer(m:&Manager,record:&environment_revision::InstallerBinding)->Result<()> {
    let binding=Binding {operation:record.operation.clone(),before:record.before.clone(),
        environment:record.environment.clone(),vendor_operation:record.operation.clone(),target:Target::Installer};
    acquire_binding(m,&binding,&intent(m,&binding)?)
}
pub(crate) fn reacquire_installer(m:&Manager,record:&environment_revision::InstallerBinding)->Result<()> {
    let binding=Binding {operation:record.operation.clone(),before:record.before.clone(),
        environment:record.environment.clone(),vendor_operation:record.operation.clone(),target:Target::Installer};
    let retirement=observe_job_retired(m,&binding)?;
    let execution=intent(m,&binding)?;
    enroll(m,&execution.active_operation,if execution.active_operation==execution.owner {&record.before} else {&record.environment})?;
    let _registry=m.lock("registry.lock")?;retirement.verify(m)?;
    require(environment_install::current_environment(m,&record.environment)?==record.environment,
        "setup_maintenance_environment_changed")?;
    acquire_installer(m,record)
}
pub(crate) fn for_installer(m:&Manager,operation:&str)->Result<Option<Environment>> {
    for binding in bindings(m)? {
        let matches=match &binding.target {
            Target::Installer=>binding.operation==operation,
            Target::Onboarding=>initial_record(m,&binding.environment.id)?.installation_operation.as_deref()==Some(operation),
            Target::Vendor(_)|Target::InstallerReservation=>false,
        };
        if matches {return owned(m,&intent(m,&binding)?.active_operation);}
    }
    Ok(None)
}
fn acquire_binding(m:&Manager,binding:&Binding,intent:&Execution)->Result<()> {
    capacity::require_environment_inactive(m,&binding.environment)?;
    for other in bindings(m)? {
        if other.operation==binding.operation || !same_setup(&other.environment,&binding.environment) {continue}
        let prior=intent_for_binding(m,&other)?;
        require(state(m,&other,&prior)?.is_some_and(|s|!s.held),"vendor_maintenance_other_owner")?;
    }
    atomic_json(&job_dir(m,&binding.operation)?.join(STATE),&state_for(m,binding,intent,true)?)
}
fn intent_for_binding(m:&Manager,binding:&Binding)->Result<Execution> {intent(m,binding)}
fn custody_retired(m:&Manager,binding:&Binding)->Result<()> {
    for owner in capacity::owners_for(m,Some(&binding.environment))? {
        let scope=m.lease_environment(&owner.session)?;
        require(!same_setup(&scope,&binding.environment),"vendor_maintenance_host_cleanup_unconfirmed")?;
    }
    Ok(())
}
/// Called after the synchronous executor has finished every child, on success
/// or clean failure. Failed work and pending publications remain untouched.
pub(crate) fn settle_executor(m:&Manager,executor:&str)->Result<()> {
    for binding in bindings(m)? {
        let intent=intent(m,&binding)?;
        if intent.active_operation!=executor {continue}
        if state(m,&binding,&intent).is_ok_and(|s|s.is_some_and(|s|!s.held)) {continue}
        let retirement=observe_job_retired(m,&binding)?;
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
    for binding in bindings(m)? {
        let result=(||->Result<bool> {
        let intent=intent(m,&binding)?;
        if state(m,&binding,&intent).is_ok_and(|s|s.is_some_and(|s|!s.held)) {return Ok(false)}
        if worker_live(&intent.active_operation)? {return Ok(false)}
        let Ok(_worker)=m.lock(&format!("operator-worker-{}.lock",intent.active_operation)) else {return Ok(false)};
        if worker_live(&intent.active_operation)? {return Ok(false)}
        let retirement=observe_job_retired(m,&binding)?;
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
fn observe_job_retired(m:&Manager,binding:&Binding)->Result<RetirementObservation> {
    let mut observation=observe_retired(m,&binding.environment)?;
    let owned=match binding.target {
        Target::Onboarding=>if let Some(operation)=initial_record(m,&binding.environment.id)?.installation_operation {
            let path=onboarding::directory(m,&binding.environment.id)?.join(format!("{operation}-result.json"));
            Some((operation,path))
        } else {None},
        Target::Installer=>Some((binding.operation.clone(),job_dir(m,&binding.operation)?.join("installation-result.json"))),
        Target::Vendor(_)=>None,
        Target::InstallerReservation=> {
            require(!onboarding::live(&binding.operation)?,"setup_installer_still_live")?;
            require(read_json::<Environment>(&binding.before.root.join("environment.json"))?==binding.before,
                "setup_maintenance_environment_changed")?;
            for name in ["installation.json","installer-spec.json","installation-result.json"] {
                let path=job_dir(m,&binding.operation)?.join(name);
                require(!path.try_exists()?,"setup_maintenance_reservation_changed")?;
                observation.files.push((path,None));
            }
            observation.files.push((binding.before.root.join("environment.json"),
                Some(control_artifact(binding.before.root.join("environment.json"))?)));
            None
        },
    };
    if let Some((operation,path))=owned {
        require(!onboarding::live(&operation)?,"setup_installer_still_live")?;
        observation.files.push((path.clone(),Some(control_artifact(path)?)));
    }
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

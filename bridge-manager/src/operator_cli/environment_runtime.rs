//! Runtime changes are revision edges owned by the existing operator job.
//! No prefix copy, vendor-state restoration, or alternate publication owner.
use super::*;
use sha2::Sha256;
use std::os::unix::fs::MetadataExt;
#[cfg(test)]
thread_local! {
    static RECONCILE_WORKER_LIVE: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
pub(super) enum Owner {
    Class { registration: Box<Registration> },
    Setup { record: Artifact, environment: Environment },
    VendorApplication { record: Artifact, environment: Environment, name: String },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub(super) enum State { Prepared, Committed, Executing, Scanned, Cancelled }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub schema: u32,
    pub operation: String,
    pub before: Environment,
    pub environment: Environment,
    pub affected: Vec<Owner>,
    pub state: State,
}
pub(super) fn identity(environment: &Environment) -> Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(environment)?)))
}
fn path(m: &Manager, operation: &str) -> Result<PathBuf> {
    Ok(job_dir(m,operation)?.join("runtime-transition.json"))
}
pub(super) fn load(m: &Manager, operation: &str) -> Result<Binding> {
    let binding: Binding = read_json(&path(m,operation)?)?;
    let request: ui::Request = read_json(&job_dir(m,operation)?.join("request.json"))?;
    require(binding.schema == 1 && binding.operation == operation && matches!(request.schema,22..=ui::OPERATOR_SCHEMA)
        && binding.before.id == binding.environment.id
        && binding.before.root == binding.environment.root
        && binding.environment.root == m.root.join("environments").join(&binding.environment.id)
        && valid_product_environment(&binding.environment.id)
        && binding.before.revision.checked_add(1) == Some(binding.environment.revision)
        && binding.before.runner != binding.environment.runner
        && request.action == (ui::Action::EnvironmentRuntimeSelect {
            environment: binding.before.id.clone(),
            runner: onboarding::runner_key(&binding.environment.runner)?,
            expected_environment: identity(&binding.before)?,
        }), "runtime_transition_binding")?;
    binding.before.runner.validate_identity()?;
    binding.environment.runner.validate_identity()?;
    require(!binding.affected.is_empty() && binding.affected.len() <= 256,
        "runtime_transition_owners")?;
    for owner in &binding.affected {
        let environment = match owner {
            Owner::Class {registration} => {
                registration.metadata.verify()?;
                &registration.environment
            }
            Owner::Setup {record,environment} => {
                require(record.path == m.root.join("onboarding").join(&environment.id).join("record.json")
                    && valid_hex(&record.sha256,64),"runtime_transition_setup_binding")?;
                environment
            }
            Owner::VendorApplication {record,environment,..} => {
                require(record.path.parent().and_then(Path::parent)
                    == Some(m.root.join("vendor-applications").as_path())
                    && record.path.file_name().is_some_and(|name|name=="application.json")
                    && valid_hex(&record.sha256,64),"runtime_transition_vendor_binding")?;
                environment
            }
        };
        require(environment.id == binding.before.id && environment.root == binding.before.root
            && environment.revision <= binding.before.revision,"runtime_transition_owner_binding")?;
    }
    Ok(binding)
}
pub(super) fn records(m: &Manager) -> Result<Vec<Binding>> {
    let directory = m.root.join("operator");
    if !directory.try_exists()? {return Ok(vec![]);}
    let mut records = vec![];
    for (count,entry) in fs::read_dir(directory)?.enumerate() {
        require(count < 4096,"operator_runtime_history_bound")?;
        let directory = entry?.path();
        let Some(id) = directory.file_name().and_then(|name|name.to_str()) else {continue};
        if valid_hex(id,32) && directory.join("runtime-transition.json").try_exists()? {
            records.push(load(m,id)?);
        }
    }
    records.sort_by_key(|record|(record.environment.id.clone(),record.environment.revision));
    Ok(records)
}
fn owners(m: &Manager, environment: &str) -> Result<Vec<Owner>> {
    let mut owners:Vec<_> = m.registry()?.classes.into_values()
        .filter(|entry|entry.registration.environment.id == environment)
        .map(|entry|Owner::Class {registration:Box::new(entry.registration)}).collect();
    if valid_hex(environment,32) {
        let record=onboarding::directory(m,environment)?.join("record.json");
        if record.try_exists()? {
            let retained:onboarding::Record=read_json(&record)?;
            require(retained.id==environment && retained.environment.id==environment,
                "runtime_transition_setup_binding")?;
            owners.push(Owner::Setup {record:control_artifact(record)?,environment:retained.environment});
        }
    }
    let directory = m.root.join("vendor-applications");
    if directory.try_exists()? {
        let mut paths = vec![];
        for (count,entry) in fs::read_dir(directory)?.enumerate() {
            require(count < 128,"runtime_transition_vendor_bound")?;
            let record = entry?.path().join("application.json");
            if record.try_exists()? {paths.push(record);}
        }
        paths.sort();
        for record in paths {
            let value = optional(&record)?;
            let bound:Environment = serde_json::from_value(value["environment"].clone())?;
            if bound.id != environment {continue;}
            let name = record.parent().and_then(Path::file_name).and_then(|name|name.to_str())
                .ok_or("runtime_transition_vendor_binding")?.to_owned();
            owners.push(Owner::VendorApplication {record:control_artifact(record)?,environment:bound,name});
        }
    }
    Ok(owners)
}
pub(super) fn installed(m: &Manager) -> Result<Vec<(String,Runner)>> {
    let sw=software_record(m)?;
    let mut runners=if sw.native_catalogue.is_some() {
        onboarding::runners_from_catalogue_with(Some(&sw.catalogue_record(m)?),false)?
    } else {vec![]};
    // Retained predecessors keep the normal return choice available after an update.
    let registry = m.registry()?;
    for runner in linux_vst_bridge::runtime_delivery::installed_identity_records(m)?.into_iter()
        .chain(registry.classes.values().map(|entry|entry.registration.environment.runner.clone()))
        .chain(records(m)?.into_iter().flat_map(|binding|[binding.before.runner,binding.environment.runner])) {
        runner.validate_identity()?;
        if runner.validate_record().is_err() {continue;}
        let key = onboarding::runner_key(&runner)?;
        if !runners.iter().any(|(id,_)|id==&key) {runners.push((key,runner));}
    }
    Ok(runners)
}
pub(super) fn select(m: &Manager, environment: &str, runner: &str, expected: &str,
    operation: &str, registry_admission: impl FnOnce()->Result<Lock>) -> Result<Binding> {
    select_admitted(m,environment,runner,expected,operation,(||vendor_retired(m), |_|Ok(())),registry_admission)
}
pub(super) fn change_and_scan(m:&Manager,environment:&str,runner:&str,expected:&str,operation:&str,
    mut registry_admission:impl FnMut()->Result<Lock>)->Result<Value> {
    let transition=select(m,environment,runner,expected,operation,&mut registry_admission)?;
    mark_executing(m,&transition)?;
    let scanned=rescan_environment_admitted(m,transition.environment.clone(),None,&mut registry_admission)?;
    mark_scanned(m,&transition)?;
    Ok(json!({"environment":transition.environment,"scan":scanned,
        "fresh_preparation_required":true,"vendor_state_rollback":false}))
}
#[cfg(test)]
fn select_with(m: &Manager, environment: &str, runner: &str, expected: &str,
    operation: &str, retired: impl FnOnce()->Result<bool>,
    checkpoint: impl FnOnce(&Binding)->Result<()>) -> Result<Binding> {
    select_admitted(m,environment,runner,expected,operation,(retired,checkpoint),||m.lock("registry.lock"))
}
fn select_admitted(m: &Manager, environment: &str, runner: &str, expected: &str,
    operation: &str, checks:(impl FnOnce()->Result<bool>,impl FnOnce(&Binding)->Result<()>),
    registry_admission:impl FnOnce()->Result<Lock>) -> Result<Binding> {
    let (retired,checkpoint)=checks;
    let _ownership = m.lock("onboarding.lock")?;
    let before = environment_install::target(m,environment)?;
    require(valid_hex(expected,64) && identity(&before)?==expected,"runtime_transition_prestate_changed")?;
    let selected = installed(m)?.into_iter().find(|(key,_)|key==runner)
        .ok_or("runtime_transition_installed_runner_missing")?.1;
    require(selected != before.runner,"runtime_transition_already_selected")?;
    // Hash immutable runner payloads before admission, without registry authority.
    let pinned_stamps=runner_stamps(&before.runner,&selected)?;
    before.runner.verify()?;selected.verify()?;
    require(runner_stamps(&before.runner,&selected)?==pinned_stamps,"runtime_transition_runner_changed")?;
    let affected = owners(m,environment)?;
    for owner in &affected {
        let bound = match owner {
            Owner::Class {registration} => &registration.environment,
            Owner::Setup {environment,..} => environment,
            Owner::VendorApplication {environment,..} => environment,
        };
        require(environment_install::current_environment(m,bound)?==before,
            "runtime_transition_owner_binding_changed")?;
    }
    let mut after = before.clone();after.runner = selected;
    after.revision = after.revision.checked_add(1).ok_or("environment_revision_exhausted")?;
    let mut binding = Binding {schema:1,operation:operation.into(),before,environment:after,
        affected,state:State::Prepared};
    let selected_key=onboarding::runner_key(&binding.environment.runner)?;
    let _registry = registry_admission()?;
    _registry.require_registry(m)?;
    m.require_inactive(None)?;
    require(capacity::owners(m)?.is_empty(),"runtime_transition_retirement_required")?;
    require(onboarding::all_retired(m)? && retired()?,"runtime_transition_retirement_required")?;
    require_settled(m,environment)?;
    require(environment_install::target(m,environment)?==binding.before
        && installed(m)?.iter().any(|(key,runner)|key==&selected_key && runner==&binding.environment.runner)
        && owners(m,environment)?==binding.affected,"runtime_transition_prestate_changed")?;
    require(!path(m,operation)?.try_exists()?,"runtime_transition_already_used")?;
    atomic_json(&path(m,operation)?,&binding)?;
    // Validate the retained request before the one authority mutation. The job
    // itself owns interruption recovery; no application or registry is rewritten.
    load(m,operation)?;
    checkpoint(&binding)?;
    require(read_json::<Environment>(&binding.before.root.join("environment.json"))?==binding.before
        && owners(m,environment)?==binding.affected,"runtime_transition_prestate_changed")?;
    require(runner_stamps(&binding.before.runner,&binding.environment.runner)?==pinned_stamps,
        "runtime_transition_runner_changed")?;
    atomic_json(&binding.environment.root.join("environment.json"),&binding.environment)?;
    binding.state = State::Committed;
    atomic_json(&path(m,operation)?,&binding)?;
    Ok(binding)
}
type RunnerStamp=(u64,u64,u64,i64,i64,i64,i64,u32);
fn runner_stamps(before:&Runner,after:&Runner)->Result<Vec<RunnerStamp>> {
    before.files.iter().chain(&after.files).map(|artifact| {
        let metadata=fs::symlink_metadata(&artifact.path)?;
        Ok((metadata.dev(),metadata.ino(),metadata.len(),metadata.mtime(),metadata.mtime_nsec(),
            metadata.ctime(),metadata.ctime_nsec(),metadata.mode()))
    }).collect()
}
pub(super) fn mark_executing(m: &Manager, binding: &Binding) -> Result<()> {
    advance(m,binding,State::Executing)
}
pub(super) fn mark_scanned(m: &Manager, binding: &Binding) -> Result<()> {
    advance(m,binding,State::Scanned)
}
fn advance(m: &Manager, binding: &Binding, state: State) -> Result<()> {
    let mut retained = load(m,&binding.operation)?;
    require(retained.before==binding.before && retained.environment==binding.environment
        && retained.affected==binding.affected
        && environment_install::current_environment(m,&binding.before)?==binding.environment,
        "runtime_transition_binding_changed")?;
    require(matches!((retained.state,state),(State::Committed,State::Executing)
        |(State::Executing,State::Scanned)),"runtime_transition_state")?;
    retained.state=state;atomic_json(&path(m,&binding.operation)?,&retained)
}
pub(super) fn finish(m: &Manager, operation: &str) -> Result<()> {
    if !path(m,operation)?.try_exists()? {return Ok(());}
    if load(m,operation)?.state != State::Prepared {return Ok(());}
    finish_admitted(m,operation,||acquire_readback(m,ui::OperatorLock::Registry,Some(operation),
        OPERATOR_WAIT,&mut vec![]))
}
fn finish_admitted(m:&Manager,operation:&str,registry_admission:impl FnOnce()->Result<Lock>)->Result<()> {
    if load(m,operation)?.state != State::Prepared {return Ok(());}
    let _registry=registry_admission()?;
    _registry.require_registry(m)?;
    settle(m,operation)
}
fn settle(m:&Manager,operation:&str)->Result<()> {
    let mut retained=load(m,operation)?;
    if retained.state != State::Prepared {return Ok(());}
    let current:Environment=read_json(&retained.before.root.join("environment.json"))?;
    retained.state=if current==retained.before {State::Cancelled}
        else if current==retained.environment {State::Committed}
        else {return Err("runtime_transition_recovery_binding_changed".into());};
    atomic_json(&path(m,operation)?,&retained)
}
pub(super) fn require_settled(m:&Manager,environment:&str)->Result<()> {
    require(!pending(m,environment)?,"runtime_transition_recovery_required")
}
pub(super) fn pending(m:&Manager,environment:&str)->Result<bool> {
    Ok(records(m)?.iter().any(|binding|binding.environment.id==environment
        && binding.state==State::Prepared))
}
pub(super) fn reconcile(m:&Manager,registry_admission:impl FnMut()->Result<Lock>)->Result<()> {
    reconcile_admitted(m,|operation| {
        #[cfg(test)]
        if let Some(live)=RECONCILE_WORKER_LIVE.with(|state|state.get()) {return Ok(live);}
        let state=readiness::bounded_user_unit_state(&format!("linux-vst-bridge-operator-{operation}.service"))?;
        match state.as_str() {
            "inactive"|"failed"=>Ok(false),
            "active"|"activating"|"deactivating"|"reloading"=>Ok(true),
            _=>Err("runtime_transition_worker_state_unavailable".into()),
        }
    },registry_admission)
}
#[cfg(test)]
fn reconcile_with(m:&Manager,mut worker_live:impl FnMut(&str)->Result<bool>)->Result<()> {
    reconcile_admitted(m,&mut worker_live,||acquire_readback(m,ui::OperatorLock::Registry,None,OPERATOR_WAIT,&mut vec![]))
}
fn reconcile_admitted(m:&Manager,mut worker_live:impl FnMut(&str)->Result<bool>,
    mut registry_admission:impl FnMut()->Result<Lock>)->Result<()> {
    let _environment=m.lock("operator-environment.lock")?;
    for binding in records(m)?.into_iter().filter(|binding|binding.state==State::Prepared) {
        let result=optional(&job_dir(m,&binding.operation)?.join("result.json"))?;
        require(result["schema"]==1 && result["operation"]==binding.operation
            && matches!(result["state"].as_str(),Some("completed"|"refused"))
            && !worker_live(&binding.operation)?,"runtime_transition_worker_retirement_required")?;
        finish_admitted(m,&binding.operation,&mut registry_admission)?;
    }
    Ok(())
}
pub(super) fn projection(m: &Manager, environment: &Environment, busy: Option<&str>)
    -> Result<ui::EnvironmentRuntime> {
    let affected = owners(m,&environment.id)?.into_iter().map(|owner|match owner {
        Owner::Class {registration} => registration.metadata.name,
        Owner::Setup {..} => "Original installed setup".into(),
        Owner::VendorApplication {name,..} => format!("Vendor application: {name}"),
    }).collect();
    let expected_environment=identity(environment)?;
    let pending=pending(m,&environment.id)?;
    let reason=if pending {Some("Finish the interrupted runtime change with Reconcile interrupted transaction before changing this setup again.")} else {busy};
    let choices=installed(m)?.into_iter().filter(|(_,runner)|runner!=&environment.runner)
        .map(|(key,runner)|action(&format!("Use {} · {}",runner.id,runner.version),
            ui::Action::EnvironmentRuntimeSelect {environment:environment.id.clone(),runner:key,
                expected_environment:expected_environment.clone()},reason)).collect();
    Ok(ui::EnvironmentRuntime {runner:environment.runner.id.clone(),revision:environment.revision,
        affected,choices,consequence:"Changing the runtime affects every plug-in and vendor application in this setup. Close them first. The compatibility space, installed files and installation history stay in place. The manager checks the installed plug-ins using the selected runtime. For each plug-in, choose Check compatibility, then Make available for testing or Replace it with this test configuration before trying it in your DAW. Returning to a previous runtime is another change and does not undo vendor changes made while using it.".into()})
}

#[cfg(test)]
mod tests {
    use super::*;
    use linux_vst_bridge::preparation as prep;
    fn fixture() -> (test_fixture::Fixture,prep::Candidate,Software,Runner) {
        fixture_with_publication(true)
    }
    fn fixture_with_publication(register_original:bool) -> (test_fixture::Fixture,prep::Candidate,Software,Runner) {
        let (f,c,sw,_)=preparation_cli::tests::guided_fixture_with_role(false);
        if register_original {f.m.register(f.r.clone()).unwrap();}
        let mut sibling=f.r.clone();sibling.metadata.class_id="02".repeat(16);
        sibling.metadata.name="Reference effect".into();
        sibling.metadata.subcategories="Fx|Filter".into();f.m.register(sibling).unwrap();
        let mut other=f.r.clone();other.metadata.class_id="03".repeat(16);
        other.environment.id="22222222-2222-4222-8222-222222222222".into();
        other.environment.root=f.m.root.join("environments").join(&other.environment.id);
        private_dir(&other.environment.root).unwrap();
        let path=f.outer.join("other-proton");fs::write(&path,b"another coherent runner").unwrap();
        other.environment.runner.id="unfamiliar-installed-runner".into();
        other.environment.runner.proton=path.clone();
        other.environment.runner.files[0]=Artifact {sha256:digest(&path).unwrap(),path};
        atomic_json(&other.environment.root.join("environment.json"),&other.environment).unwrap();
        other.module.path=other.environment.root.join("compatdata/pfx/drive_c/other.vst3");
        private_dir(other.module.path.parent().unwrap()).unwrap();fs::write(&other.module.path,b"other module").unwrap();
        other.module.sha256=digest(&other.module.path).unwrap();f.m.register(other.clone()).unwrap();
        (f,c,sw,other.environment.runner)
    }
    fn job(m:&Manager,before:&Environment,runner:&Runner)->String {
        let operation=random_id().unwrap();private_dir(&job_dir(m,&operation).unwrap()).unwrap();
        atomic_json(&job_dir(m,&operation).unwrap().join("request.json"),&ui::Request {
            schema:ui::OPERATOR_SCHEMA,state_token:"fixture".into(),
            action:ui::Action::EnvironmentRuntimeSelect {environment:before.id.clone(),
                runner:onboarding::runner_key(runner).unwrap(),expected_environment:identity(before).unwrap()}}).unwrap();
        operation
    }
    fn change(m:&Manager,before:&Environment,runner:&Runner)->Binding {
        let operation=job(m,before,runner);
        select_with(m,&before.id,&onboarding::runner_key(runner).unwrap(),
            &identity(before).unwrap(),&operation,||Ok(true),|_|Ok(())).unwrap()
    }
    fn fresh_candidate(f:&test_fixture::Fixture,previous:&prep::Candidate,sw:&Software,
        environment:&Environment)->prep::Candidate {
        // Deterministic scanner/inspector readback for the new exact revision.
        let path=f.m.root.join("inventory").join(format!("{}.json",environment.id));
        let mut scan:inventory::Scan=read_json(&path).unwrap();
        scan.id=random_id().unwrap();scan.environment=environment.clone();
        let factory=f.outer.join(format!("factory-{}.json",environment.revision));
        fs::copy(&scan.modules[0].report.path,&factory).unwrap();
        scan.modules[0].report=Artifact {sha256:digest(&factory).unwrap(),path:factory};
        atomic_json(&path,&scan).unwrap();
        let selection=prep::selections(&f.m,&sw.host,&sw.source_sha256).unwrap().into_iter()
            .find(|selection|selection.class.id==previous.selection.class.id).unwrap();
        let report=f.outer.join(format!("inspection-{}.json",environment.revision));
        fs::copy(&previous.inspection.report.path,&report).unwrap();
        let inspection=prep::inspect_record_with(selection.clone(),
            Artifact {sha256:digest(&report).unwrap(),path:report},prep::Origin::ManagedPreparation,
            previous.inspection.host.clone(),previous.inspection.source_manifest.clone()).unwrap();
        prep::retain_inspection(&f.m,&inspection).unwrap();
        let candidate=prep::prepared(selection,inspection,previous.native.clone(),previous.host.clone(),
            previous.source_manifest.clone(),previous.recipe_sha256.clone()).unwrap();
        let candidate=prep::configuration::carry_settings(candidate,Some(previous)).unwrap();
        let candidate=prep::bind_preparation_basis(candidate,
            Some(prep::preparation_basis(&f.m,Some(previous)).unwrap())).unwrap();
        prep::record_candidate_with_predecessor(&f.m,&candidate,Some(&previous.id().unwrap())).unwrap();
        candidate
    }
    fn publish_test(f:&test_fixture::Fixture,candidate:&prep::Candidate) {
        let current=f.m.registry().unwrap().classes.get(&candidate.selection.class.id)
            .and_then(|entry|entry.managed_revision.clone());
        let action=ui::Action::CompatibilityPublishTest {candidate:candidate.id().unwrap(),
            expected_current:current.map(|revision|ui::PublicationIdentity {id:revision.id,sha256:revision.sha256})};
        preparation_cli::execute(&f.m,&action,&random_id().unwrap(),||f.m.lock("registry.lock")).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&candidate.selection.class.id]
            .registration.environment,candidate.selection.environment);
    }
    fn lease(f:&test_fixture::Fixture,kind:capacity::Kind)->PathBuf {
        let operation=random_id().unwrap();
        let keeper=kind==capacity::Kind::Keeper;
        let report=f.m.root.join("runtime/results").join(format!("{}-{operation}.json",
            if keeper {"environment"} else {"windows"}));
        private_dir(report.parent().unwrap()).unwrap();
        let owner=f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&operation).join("owner.json");private_dir(owner.parent().unwrap()).unwrap();
        atomic_json(&owner,&json!({"session":operation,"report":report,"keeper":keeper,
            "inspect":matches!(kind,capacity::Kind::Keeper|capacity::Kind::Inspection),
            "vendor_access":kind==capacity::Kind::VendorAccess,
            "registration":{"metadata":{"class_id":"02".repeat(16)}}})).unwrap();
        let lease=f.m.root.join("runtime/leases").join(format!("{operation}.json"));
        private_dir(lease.parent().unwrap()).unwrap();atomic_json(&lease,&report).unwrap();lease
    }
    #[test]
    fn runtime_change_preserves_siblings_history_and_requires_fresh_return_preparation() {
        let (f,c,sw,runner)=fixture();
        prep::record_candidate(&f.m,&c).unwrap();
        let registry=fs::read(f.m.root.join("registry.json")).unwrap();
        let module=fs::read(&f.r.module.path).unwrap();
        let preference=f.m.root.join("performance").join(format!("{}.json",f.r.key()));
        private_dir(preference.parent().unwrap()).unwrap();atomic_json(&preference,&Performance::default()).unwrap();
        let preference_bytes=fs::read(&preference).unwrap();
        let initial=f.r.environment.clone();let next=change(&f.m,&initial,&runner);
        assert_eq!(next.affected.len(),2);assert_eq!(next.environment.id,initial.id);
        assert_eq!(next.environment.root,initial.root);assert_eq!(next.environment.revision,2);
        assert_eq!(environment_install::current_environment(&f.m,&initial).unwrap(),next.environment);
        let catalogue=sw.catalogue_record(&f.m).unwrap();
        let bindings=managed_environment_bindings(&f.m,Some(&catalogue),&f.m.registry().unwrap()).unwrap();
        assert_eq!(bindings[0].environment,next.environment);
        assert_eq!(managed_rescan_binding_from(&f.m,&bindings,&f.m.registry().unwrap(),&initial.id).unwrap(),Some(next.environment.clone()));
        assert!(prep::validate_current_candidate_record(&f.m,&c,&sw.host,&sw.source_sha256).is_err());
        assert!(prep::selections(&f.m,&sw.host,&sw.source_sha256).unwrap().is_empty());
        mark_executing(&f.m,&next).unwrap();
        let returned=change(&f.m,&next.environment,&initial.runner);
        assert_eq!(returned.environment.revision,3);assert_eq!(returned.environment.runner,initial.runner);
        assert_eq!(environment_install::current_environment(&f.m,&initial).unwrap(),returned.environment);
        assert!(prep::validate_current_candidate_record(&f.m,&c,&sw.host,&sw.source_sha256).is_err());
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),registry);
        assert_eq!(fs::read(&f.r.module.path).unwrap(),module);
        assert_eq!(fs::read(&preference).unwrap(),preference_bytes);
        assert_eq!(prep::candidates(&f.m,&sw.host,&sw.source_sha256).unwrap()[0].id().unwrap(),c.id().unwrap());
        let projected=projection(&f.m,&returned.environment,None).unwrap();
        assert_eq!(projected.affected.len(),2);
        assert!(projected.choices.iter().any(|offer|matches!(&offer.action,
            ui::Action::EnvironmentRuntimeSelect {runner:choice,..} if choice==&onboarding::runner_key(&runner).unwrap())));
        assert!(projected.consequence.contains("does not undo vendor changes"));
    }
    #[test]
    fn every_owner_must_retire_before_authority_changes() {
        for kind in [capacity::Kind::Dsp,capacity::Kind::Keeper,capacity::Kind::Inspection,capacity::Kind::VendorAccess] {
            let (f,_,_,runner)=fixture();let initial=f.r.environment.clone();
            let before=fs::read(initial.root.join("environment.json")).unwrap();
            let lease=lease(&f,kind);let operation=job(&f.m,&initial,&runner);
            assert!(select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),
                &identity(&initial).unwrap(),&operation,||Ok(true),|_|panic!("active owner passed admission")).is_err());
            assert_eq!(fs::read(initial.root.join("environment.json")).unwrap(),before);
            assert!(!path(&f.m,&operation).unwrap().exists());
            fs::remove_file(lease).unwrap();
            assert!(select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),
                &identity(&initial).unwrap(),&operation,||Ok(false),|_|panic!("vendor owner passed admission")).is_err());
        }
    }
    #[test]
    fn stale_prestate_target_and_unowned_runner_changes_refuse() {
        let (f,_,_,runner)=fixture();let initial=f.r.environment.clone();
        let operation=job(&f.m,&initial,&runner);
        let foreign=Manager {root:f.outer.join("foreign-manager"),publications:f.outer.join("foreign-publications")};
        for wrong_manager in [false,true] {
            let admission=||if wrong_manager {foreign.lock("registry.lock")} else {f.m.lock("other.lock")};
            let error=select_admitted(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),
                &identity(&initial).unwrap(),&operation,(||Ok(true),|_|panic!("foreign authority committed")),admission)
                .unwrap_err();
            assert_eq!(error.to_string(),"registry_guard_identity");
            assert_eq!(f.m.reconcile_inactive_with_registry(admission).unwrap_err().to_string(),
                "registry_guard_identity");
            assert_eq!(read_json::<Environment>(&initial.root.join("environment.json")).unwrap(),initial);
        }
        assert!(select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),
            &"ff".repeat(32),&operation,||Ok(true),|_|panic!("stale prestate committed")).is_err());
        fs::write(&runner.proton,b"changed runtime bytes").unwrap();
        assert!(select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),
            &identity(&initial).unwrap(),&operation,||Ok(true),|_|panic!("changed runner committed")).is_err());
        for revision in [initial.revision,initial.revision+1] {
            let mut forged=initial.clone();forged.runner=runner.clone();forged.revision=revision;
            atomic_json(&initial.root.join("environment.json"),&forged).unwrap();
            assert!(environment_install::current_environment(&f.m,&initial).is_err());
            assert!(environment_install::target(&f.m,&initial.id).is_err());
        }
    }
    #[test]
    fn interrupted_atomic_commit_recovers_exactly_and_never_claims_vendor_restore() {
        for committed in [false,true] {
            let (f,_,_,runner)=fixture();let initial=f.r.environment.clone();let operation=job(&f.m,&initial,&runner);
            let error=select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),
                &identity(&initial).unwrap(),&operation,||Ok(true),|binding| {
                    if committed {atomic_json(&binding.environment.root.join("environment.json"),&binding.environment)?;}
                    Err("fixture_interruption".into())
                }).unwrap_err();
            assert_eq!(error.to_string(),"fixture_interruption");
            finish(&f.m,&operation).unwrap();let binding=load(&f.m,&operation).unwrap();
            assert_eq!(binding.state,if committed {State::Committed} else {State::Cancelled});
            assert_eq!(environment_install::current_environment(&f.m,&initial).unwrap(),
                if committed {binding.environment.clone()} else {initial.clone()});
            if !committed {
                // A cancelled reservation cannot authorize a later manual metadata flip.
                atomic_json(&initial.root.join("environment.json"),&binding.environment).unwrap();
                assert!(environment_install::current_environment(&f.m,&initial).is_err());
            }
        }
    }
    #[test]
    fn vendor_binding_resolves_current_revision_and_stays_historical() {
        let (f,_,_,runner)=fixture();let initial=f.r.environment.clone();
        let application=renderer::Application {schema:1,id:renderer::ID.into(),environment:initial.clone(),
            files:std::collections::BTreeMap::new(),installation:f.r.host.clone(),
            observation_sha256:"aa".repeat(32),source_seal_sha256:"bb".repeat(32)};
        let directory=renderer_cli::directory(&f.m);private_dir(&directory).unwrap();
        let record=directory.join("application.json");atomic_json(&record,&application).unwrap();
        let original=fs::read(&record).unwrap();let next=change(&f.m,&initial,&runner);
        assert!(next.affected.iter().any(|owner|matches!(owner,Owner::VendorApplication {..})));
        let current=renderer_cli::current_application(&f.m).unwrap();assert_eq!(current.environment,next.environment);
        assert_ne!(current.identity().unwrap(),application.identity().unwrap());
        assert_eq!(fs::read(record).unwrap(),original);
    }
    #[test]
    fn fresh_scan_check_and_explicit_test_publication_work_in_both_directions() {
        let (f,original,sw,runner)=fixture_with_publication(false);
        prep::record_candidate(&f.m,&original).unwrap();publish_test(&f,&original);
        let initial=f.r.environment.clone();let next=change(&f.m,&initial,&runner);
        assert!(prep::validate_current_candidate_record(&f.m,&original,&sw.host,&sw.source_sha256).is_err());
        let trial=fresh_candidate(&f,&original,&sw,&next.environment);
        assert!(prep::validate_current_candidate_record(&f.m,&trial,&sw.host,&sw.source_sha256).is_ok());
        assert_ne!(trial.id().unwrap(),original.id().unwrap());publish_test(&f,&trial);
        mark_executing(&f.m,&next).unwrap();mark_scanned(&f.m,&next).unwrap();
        let returned=change(&f.m,&next.environment,&initial.runner);
        assert!(prep::validate_current_candidate_record(&f.m,&trial,&sw.host,&sw.source_sha256).is_err());
        assert!(prep::validate_current_candidate_record(&f.m,&original,&sw.host,&sw.source_sha256).is_err());
        let refreshed=fresh_candidate(&f,&trial,&sw,&returned.environment);
        assert_ne!(refreshed.id().unwrap(),original.id().unwrap());
        assert_ne!(refreshed.id().unwrap(),trial.id().unwrap());publish_test(&f,&refreshed);
        for candidate in [&original,&trial,&refreshed] {
            assert_eq!(prep::candidate_record(&f.m,&candidate.id().unwrap()).unwrap().id().unwrap(),candidate.id().unwrap());
        }
    }
    #[test]
    fn unpublished_installed_setup_projects_the_current_revision_and_return_choice() {
        let (f,_,_,runner)=fixture();let mut environment=f.r.environment.clone();
        environment.id="cd".repeat(16);environment.root=f.m.root.join("environments").join(&environment.id);
        private_dir(&environment.root).unwrap();atomic_json(&environment.root.join("environment.json"),&environment).unwrap();
        let mut bytes=vec![0;1024];bytes[..2].copy_from_slice(b"MZ");bytes[60]=128;
        bytes[128..132].copy_from_slice(b"PE\0\0");bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148]=2;bytes[150]=2;bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let installer=f.outer.join("reference-installer.exe");fs::write(&installer,bytes).unwrap();
        let installer=installer_import::import(&f.m,file(&installer).unwrap()).unwrap();
        let initial=random_id().unwrap();let directory=onboarding::directory(&f.m,&environment.id).unwrap();
        private_dir(&directory).unwrap();let record=directory.join("record.json");
        atomic_json(&record,&onboarding::Record {schema:1,id:environment.id.clone(),installer:installer.id,
            environment:environment.clone(),created_at:1,creation_operation:random_id().unwrap(),
            installation_operation:Some(initial.clone()),published:false,previous_attempt:None}).unwrap();
        atomic_json(&directory.join(format!("{initial}-result.json")),&json!({"schema":2,"operation":initial,
            "state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
        let historical=fs::read(&record).unwrap();let next=change(&f.m,&environment,&runner);
        assert!(matches!(next.affected.as_slice(),[Owner::Setup {..}]));
        let mut current=next.environment.clone();
        for step in 0..2 {
            let rows=environment_install::projection(&f.m,&[],None,|_|Ok(false)).unwrap();
            let row=rows.iter().find(|row|row.environment==environment.id).unwrap();let runtime=row.runtime.as_ref().unwrap();
            let exact=environment_install::current_environment(&f.m,&environment).unwrap();
            assert_eq!(exact,current);
            assert_eq!(runtime.revision,exact.revision);assert_eq!(runtime.runner,exact.runner.id);
            assert_eq!(onboarding::load(&f.m,&environment.id).unwrap().environment,exact);
            assert!(runtime.choices.iter().all(|offer|matches!(&offer.action,
                ui::Action::EnvironmentRuntimeSelect {expected_environment,..} if expected_environment==&identity(&exact).unwrap())));
            assert!(row.actions.iter().any(|offer|matches!(offer.action,ui::Action::EnvironmentInstallerScan {..})));
            if step==0 {current=change(&f.m,&current,&environment.runner).environment;}
        }
        assert_eq!(fs::read(record).unwrap(),historical);
    }
    #[test]
    fn runtime_owned_scan_waits_through_brief_registry_contention() {
        let (f,c,mut sw,runner)=fixture();let before=f.r.environment.clone();
        let script=f.m.root.join("software/runtime-scan-fixture.py");
        let report=serde_json::to_string(&c.selection.factory_report.path).unwrap();
        fs::write(&script,format!("import json,pathlib,shutil,sys\njob=json.load(open(sys.argv[1]))\nwith open({report},'rb') as source,open(job['report'],'wb') as target: target.write(source.read())\nprint('LVO0 '+job['session']+' ready',flush=True)\nshutil.rmtree(job['directory'])\nprint('LVO1 '+job['session']+' retired',flush=True)\n")).unwrap();
        sw.supervisor=Artifact {sha256:digest(&script).unwrap(),path:script};
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();
        let operation=job(&f.m,&before,&runner);let mut waits=vec![];let mut calls=0;
        let mut remaining_wait=Duration::from_secs(2);
        std::thread::scope(|scope| {
            let result=change_and_scan(&f.m,&before.id,&onboarding::runner_key(&runner).unwrap(),
                &identity(&before).unwrap(),&operation,|| {
                    calls+=1;
                    if calls==2 {
                        let (tx,rx)=std::sync::mpsc::channel();let manager=&f.m;
                        scope.spawn(move|| {
                            let _lock=manager.lock("registry.lock").unwrap();tx.send(()).unwrap();
                            std::thread::sleep(Duration::from_millis(180));
                        });
                        rx.recv().unwrap();
                    }
                    let started=Instant::now();
                    let guard=acquire_readback(&f.m,ui::OperatorLock::Registry,Some(&operation),remaining_wait,&mut waits);
                    remaining_wait=remaining_wait.saturating_sub(started.elapsed());guard
                }).unwrap();
            assert_eq!(result["scan"]["modules"],1);
        });
        assert_eq!(calls,2);assert_eq!(waits.len(),2);assert!(waits[1].attempts>1);
        assert!(waits[1].timeout_ms<=waits[0].timeout_ms);
        let binding=load(&f.m,&operation).unwrap();assert_eq!(binding.state,State::Scanned);
        let scan:inventory::Scan=read_json(&f.m.root.join("inventory").join(format!("{}.json",before.id))).unwrap();
        assert_eq!(scan.environment,binding.environment);assert_eq!(scan.modules.len(),1);
        assert!(capacity::owners(&f.m).unwrap().is_empty());
    }
    #[test]
    fn live_vendor_progress_uses_resolved_runtime_without_rewriting_history() {
        let (f,_,_,runner)=fixture();let before=f.r.environment.clone();
        let directory=app_directory(&f.m);private_dir(&directory).unwrap();
        let historical=vendor_application::Application {schema:1,
            id:vendor_application::ApplicationId::ArturiaSoftwareCenter,environment:before.clone(),
            executable:f.r.module.clone(),helpers:vec![],installer_sha256:"00".repeat(32),
            installer_result:f.r.host.clone(),observed_installer_version:"fixture".into()};
        let record=directory.join("application.json");atomic_json(&record,&historical).unwrap();
        let bytes=fs::read(&record).unwrap();let next=change(&f.m,&before,&runner);
        let mut resolved=historical.clone();resolved.environment=next.environment;
        let operation=random_id().unwrap();let report=directory.join("operation-result.json");
        atomic_json(&directory.join("operation.json"),&vendor_application::Operation {
            application:resolved.clone(),report:report.clone(),mode:vendor_application::LaunchMode::Normal,
            operation_id:operation.clone()}).unwrap();
        atomic_json(&report,&json!({"schema":3,"operation_id":operation,"state":"running",
            "launcher_exit":null,"owned_live":2,"cleanup_confirmed":false,
            "discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        let progress=current::live_vendor_progress_with(&f.m,||Ok(true)).unwrap().unwrap();
        assert_eq!(progress.application,resolved);assert_eq!(progress.operation_id,operation);
        assert_eq!(fs::read(record).unwrap(),bytes);
        atomic_json(&directory.join("operation.json"),&vendor_application::Operation {
            application:historical,report,mode:vendor_application::LaunchMode::Normal,
            operation_id:operation}).unwrap();
        assert!(current::live_vendor_progress_with(&f.m,||Ok(true)).is_err());
    }
    #[test]
    fn terminal_precommit_reservation_cannot_be_bypassed_after_contended_finish() {
        let (f,_,_,runner)=fixture();let initial=f.r.environment.clone();let operation=job(&f.m,&initial,&runner);
        assert!(select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),&identity(&initial).unwrap(),
            &operation,||Ok(true),|_|Err("fixture_interruption".into())).is_err());
        atomic_json(&job_dir(&f.m,&operation).unwrap().join("result.json"),
            &json!({"schema":1,"operation":operation,"state":"refused","reason":"fixture_interruption"})).unwrap();
        let held=f.m.lock("registry.lock").unwrap();
        assert!(finish_admitted(&f.m,&operation,||acquire_readback(&f.m,ui::OperatorLock::Registry,
            Some(&operation),Duration::from_millis(20),&mut vec![])).is_err());
        drop(held);
        let retry=job(&f.m,&initial,&runner);
        let error=select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),&identity(&initial).unwrap(),
            &retry,||Ok(true),|_|panic!("unsettled reservation was bypassed")).unwrap_err();
        assert_eq!(error.to_string(),"runtime_transition_recovery_required");
        assert!(!path(&f.m,&retry).unwrap().exists());
        assert_eq!(read_json::<Environment>(&initial.root.join("environment.json")).unwrap(),initial);
        assert!(environment_install::reserve(&f.m,&initial.id,&"aa".repeat(32),&random_id().unwrap())
            .unwrap_err().to_string().contains("runtime_transition_recovery_required"));
        let projection=projection(&f.m,&initial,None).unwrap();
        assert!(projection.choices.iter().all(|offer|offer.disabled_reason.is_some()));
        assert!(reconcile_with(&f.m,|_|Ok(true)).is_err());
        let rows=environment_install::projection(&f.m,&[],None,|_|Ok(false)).unwrap();
        let offer=rows.iter().find(|row|row.environment==initial.id).unwrap().actions.iter()
            .find(|offer|matches!(offer.action,ui::Action::TransactionReconcile {})).unwrap();
        assert!(offer.disabled_reason.is_none());
        let reader=Manager {root:f.m.root.clone(),publications:f.m.publications.clone()};
        let (start,started)=std::sync::mpsc::channel();let (held,acquired)=std::sync::mpsc::channel();
        let thread=std::thread::spawn(move|| {
            started.recv_timeout(Duration::from_secs(3)).unwrap();
            let _guard=reader.lock("registry.lock").unwrap();held.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(180));
        });
        PERFORMANCE_COMMIT_BARRIER.with(|barrier| *barrier.borrow_mut()=Some(Box::new(move|| {
            start.send(()).unwrap();acquired.recv_timeout(Duration::from_secs(3)).unwrap();
        })));
        RECONCILE_WORKER_LIVE.with(|state|state.set(Some(false)));
        let mut waits=vec![];
        let result=execute_with_receipt_policy(&f.m,&offer.action,None,&|| {
            capacity::status(&f.m,capacity::fixture_limits(),0,false).ok()
                .and_then(|value|serde_json::from_value(serde_json::to_value(value).unwrap()).ok())
        },Duration::from_secs(2),&mut waits);
        RECONCILE_WORKER_LIVE.with(|state|state.set(None));thread.join().unwrap();
        assert_eq!(result.unwrap(),json!({"reconciled":true}));
        assert!(waits.iter().any(|wait|wait.name==ui::OperatorLock::Registry
            && wait.attempts>1 && wait.outcome==ui::LockOutcome::Acquired));
        assert_eq!(load(&f.m,&operation).unwrap().state,State::Cancelled);
        let completed=select_with(&f.m,&initial.id,&onboarding::runner_key(&runner).unwrap(),&identity(&initial).unwrap(),
            &retry,||Ok(true),|_|Ok(())).unwrap();
        assert_eq!(environment_install::current_environment(&f.m,&initial).unwrap(),completed.environment);
        let held=f.m.lock("registry.lock").unwrap();
        finish(&f.m,&retry).unwrap(); // Terminal commit does not need registry again.
        drop(held);
    }
}

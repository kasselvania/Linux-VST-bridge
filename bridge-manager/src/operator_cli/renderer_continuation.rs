//! The accepted renderer job owns its session and subsequent DAW connection.
use super::*;
use automatic_install::Intent;
use linux_vst_bridge::preparation as prep;

#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Continuation {
    schema:u32,
    request:Artifact,
    selected:Artifact,
    pub before:Environment,
    pub intent:Intent,
}
fn path(m:&Manager,owner:&str)->Result<PathBuf> {
    Ok(job_dir(m,owner)?.join("renderer-continuation.json"))
}
fn validate(m:&Manager,retained:&Continuation)->Result<ui::Request> {
    let owner=&retained.intent.owner;
    require(retained.schema==1 && retained.intent.vendor.is_none()
        && retained.request.path==job_dir(m,owner)?.join("request.json")
        && retained.selected.path==renderer_session::directory(m).join("application.json"),
        "automatic_renderer_checkpoint_binding")?;
    let request:ui::Request=serde_json::from_slice(&retained.request.record_bytes(16384)?)?;
    let mut selected:renderer::Application=serde_json::from_slice(&retained.selected.record_bytes(1024*1024)?)?;
    selected.validate_record(&m.root)?;
    require(environment_revision::same_space(&selected.environment,&retained.before),
        "automatic_renderer_environment_binding")?;
    selected.environment=retained.before.clone();selected.validate_record(&m.root)?;
    require(matches!(&request.action,ui::Action::RendererSoftwareInstall {application,policy:_,daw}
        if application==&selected.identity()? && *daw==retained.intent.journey.daw),
        "automatic_renderer_request_binding")?;
    if let Some(previous)=&retained.intent.previous {
        let parent=previous.path.parent().and_then(|p|p.file_name()).and_then(|s|s.to_str())
            .ok_or("automatic_renderer_previous_binding")?;
        require(previous.path==job_dir(m,parent)?.join("retained-renderer-continuation.json"),
            "automatic_renderer_previous_binding")?;
        previous.record_bytes(1024*1024)?;
    }
    Ok(request)
}
pub(super) fn load(m:&Manager,owner:&str)->Result<Continuation> {
    let retained:Continuation=read_json(&path(m,owner)?)?;
    require(retained.intent.owner==owner,"automatic_renderer_checkpoint_binding")?;
    validate(m,&retained)?;
    if retained.intent.active_operation!=owner {
        let request:ui::Request=read_json(&job_dir(m,&retained.intent.active_operation)?.join("request.json"))?;
        require(request.action==ui::Action::EnvironmentRefreshResume {operation:owner.into()},
            "automatic_renderer_resume_binding")?;
    }
    Ok(retained)
}
pub(super) fn owns(m:&Manager,owner:&str)->Result<bool> {
    let request:ui::Request=read_json(&job_dir(m,owner)?.join("request.json"))?;
    match request.action {
        ui::Action::RendererSoftwareInstall {..}=>Ok(true),
        // An installer run in a setup owns its own connection checkpoint too.
        ref action if matches!(action,ui::Action::EnvironmentRefresh {..})
            || action.environment_installer_start().is_some()=>Ok(false),
        _=>Err("automatic_connection_resume_owner".into()),
    }
}
pub(super) fn records(m:&Manager)->Result<Vec<Continuation>> {
    let root=m.root.join("operator");if !root.exists(){return Ok(vec![]);}
    let mut out=vec![];
    for (count,entry) in fs::read_dir(root)?.enumerate() {
        require(count<4096,"automatic_renderer_history_bound")?;
        let entry=entry?;let owner=entry.file_name().to_string_lossy().into_owned();
        if !valid_hex(&owner,32) {continue;}
        if path(m,&owner)?.exists(){out.push(load(m,&owner)?);}
        else {
            let request=optional(&entry.path().join("request.json"))?;
            if request["action"]["kind"]=="renderer_software_install" {
                let result=optional(&entry.path().join("result.json"))?;
                require(!result["journey"].is_object() && !result["result"]["journey"].is_object()
                    && !renderer_session::operation_dir(m,&owner)?.join("spec.json").exists(),
                    "automatic_renderer_checkpoint_missing")?;
            }
        }
    }
    Ok(out)
}
pub(super) fn authority(m:&Manager)->Result<Vec<Artifact>> {
    let mut out=vec![];
    for retained in records(m)? {
        out.push(control_artifact(path(m,&retained.intent.owner)?)?);
        out.push(control_artifact(job_dir(m,&retained.intent.active_operation)?.join("request.json"))?);
    }
    Ok(out)
}
pub(super) fn watch_paths(m:&Manager)->Result<Vec<PathBuf>> {
    let mut out=vec![renderer_session::directory(m).join("application.json"),
        renderer_session::directory(m).join("current.json")];
    for retained in records(m)? {
        let directory=job_dir(m,&retained.intent.owner)?;
        let session=renderer_session::operation_dir(m,&retained.intent.owner)?;
        out.extend([directory.join("renderer-continuation.json"),directory.join("request.json"),
            job_dir(m,&retained.intent.active_operation)?.join("request.json"),session.join("spec.json"),
            session.join("result.json"),session.join("recovery-result.json"),
            retained.before.root.join("environment.json"),
            m.root.join("inventory").join(format!("{}.json",retained.before.id))]);
    }
    Ok(out)
}
fn latest(m:&Manager)->Result<Option<Continuation>> {
    let records=records(m)?;
    let predecessors=records.iter().filter_map(|r|r.intent.previous.as_ref())
        .filter_map(|p|p.path.parent()?.file_name()?.to_str()).collect::<Vec<_>>();
    let mut leaves=records.iter().filter(|r|!predecessors.contains(&r.intent.owner.as_str()));
    let leaf=leaves.next().cloned();
    require(leaves.next().is_none(),"automatic_renderer_ownership_conflict")?;
    Ok(leaf)
}
fn retain_previous(m:&Manager,prior:&Continuation)->Result<Artifact> {
    let target=job_dir(m,&prior.intent.owner)?.join("retained-renderer-continuation.json");
    if !target.exists(){atomic_json(&target,prior)?;}
    require(serde_json::to_value(read_json::<Continuation>(&target)?)?==serde_json::to_value(prior)?,
        "automatic_renderer_previous_changed")?;
    control_artifact(target)
}
pub(super) fn save(m:&Manager,environment:&Environment,intent:&Intent,operation:&str)->Result<()> {
    let mut retained=load(m,&intent.owner)?;
    let current=if job_dir(m,&intent.owner)?.join("renderer-maintenance.json").exists() {
        environment_revision::renderer_record(m,&intent.owner)?.environment
    } else {retained.before.clone()};
    require(current==*environment,"automatic_renderer_environment_changed")?;
    require(intent.owner==retained.intent.owner && intent.vendor.is_none()
        && intent.journey.daw==retained.intent.journey.daw,"automatic_renderer_request_binding")?;
    if operation!=retained.intent.owner {
        let request:ui::Request=read_json(&job_dir(m,operation)?.join("request.json"))?;
        require(request.action==ui::Action::EnvironmentRefreshResume {operation:intent.owner.clone()},
            "automatic_renderer_resume_binding")?;
    }
    retained.intent=intent.clone();retained.intent.active_operation=operation.into();
    atomic_json(&path(m,&intent.owner)?,&retained)?;
    write_operation(m,operation,&json!({"schema":2,"operation":operation,
        "state":if intent.journey.phase==ui::InstallationPhase::VendorRunning {"vendor_running"} else {"running"},
        "renderer_owner":intent.owner,"environment":environment.id,"journey":intent.journey}),false)
}
fn successful_result(m:&Manager,retained:&Continuation)->Result<Value> {
    let request=validate(m,retained)?;
    let ui::Action::RendererSoftwareInstall {application,policy,..}=request.action else {unreachable!()};
    let result=renderer_cli::result(m,&retained.intent.owner)?;
    require(renderer_session::terminal(&result,&retained.intent.owner)
        && result["state"]=="completed" && result["cancelled"]==false && result["error"].is_null()
        && result["application_identity"]==application
        && result["requested"]==serde_json::to_value(policy)?
        && native_access_dependency::session_cleanup_confirmed(&result)?,
        "automatic_renderer_completion_required")?;
    Ok(result)
}
fn commit(m:&Manager,retained:&Continuation)->Result<Environment> {
    commit_with_retirement(m,retained,renderer_session::retired(m,&retained.intent.owner)?)
}
fn commit_with_retirement(m:&Manager,retained:&Continuation,retired:bool)->Result<Environment> {
    require(retired,"renderer_retirement_required")?;
    successful_result(m,retained)?;
    let owner=&retained.intent.owner;
    let session=renderer_session::operation_dir(m,owner)?;
    let result=if session.join("recovery-result.json").exists(){session.join("recovery-result.json")}
        else {session.join("result.json")};
    let mut after=retained.before.clone();after.revision=after.revision.checked_add(1)
        .ok_or("renderer_environment_revision_bound")?;
    let edge=environment_revision::RendererBinding {schema:1,operation:owner.clone(),
        selected:retained.selected.clone(),spec:control_artifact(session.join("spec.json"))?,
        result:control_artifact(result)?,before:retained.before.clone(),environment:after.clone()};
    let record=job_dir(m,owner)?.join("renderer-maintenance.json");
    environment_revision::validate_renderer_binding(m,owner,&edge)?;
    let _registry=m.lock("registry.lock")?;
    let current:Environment=read_json(&after.root.join("environment.json"))?;
    require(current==retained.before || current==after,"automatic_renderer_environment_changed")?;
    if record.exists(){require(serde_json::to_value(environment_revision::renderer_record(m,owner)?)?
        ==serde_json::to_value(&edge)?,"automatic_renderer_edge_changed")?;}
    else {require(current==retained.before,"automatic_renderer_edge_missing")?;atomic_json(&record,&edge)?;}
    environment_revision::renderer_record(m,owner)?;
    if current!=after {atomic_json(&after.root.join("environment.json"),&after)?;}
    Ok(after)
}
pub(super) fn completed_receipt(m:&Manager,intent:&Intent,environment:&Environment)->Result<Value> {
    let retained=load(m,&intent.owner)?;
    let edge=environment_revision::renderer_record(m,&intent.owner)?;
    require(edge.before==retained.before && edge.environment==*environment
        && environment_revision::current_environment(m,&retained.before)?==*environment,
        "automatic_renderer_environment_changed")?;
    successful_result(m,&retained)
}
// submit_with publishes its immutable spec before its reservation or external
// unit submission. A retired operator with neither record and an absent, empty
// unit therefore never submitted this session, even if it died before launch.
fn never_submitted_with(m:&Manager,retained:&Continuation,
    probe:impl FnOnce()->Result<renderer_session::UnitState>)->Result<bool> {
    validate(m,retained)?;
    let owner=&retained.intent.owner;
    let session=renderer_session::operation_dir(m,owner)?;
    let gate=session.join("writer.lock");
    let _gate=if gate.try_exists()? {
        let operator_lock::LockAttempt::Acquired(guard)=m.try_lock(
            gate.strip_prefix(&m.root)?.to_str().ok_or("renderer_gate_path")?)? else {
            return Err("renderer_custody_still_uncertain".into());
        };Some(guard)
    } else {None};
    if session.join("spec.json").try_exists()? || !renderer_cli::result(m,owner)?.is_null()
        || renderer_cli::current(m)?["operation"]==*owner {
        return Ok(false);
    }
    let actor=optional(&job_dir(m,owner)?.join("result.json"))?;
    require(actor["operation"]==*owner
        && matches!(actor["state"].as_str(),Some("completed"|"refused")),
        "operator_resume_owner_not_terminal")?;
    require(!job_dir(m,owner)?.join("renderer-maintenance.json").exists()
        && retained.intent.scan.is_none() && retained.intent.work.is_empty()
        && read_json::<Environment>(&retained.before.root.join("environment.json"))?==retained.before,
        "automatic_renderer_environment_changed")?;
    let unit=probe()?;
    require(!unit.exists && unit.empty,"renderer_custody_still_uncertain")?;
    Ok(true)
}
pub(super) fn never_submitted_custody_with(m:&Manager,saved:&ResumeRecord,
    probe:impl FnOnce()->Result<renderer_session::UnitState>)->Result<Option<Vec<Artifact>>> {
    require(saved.vendor_operation.as_deref()==Some(saved.owner_operation.as_str()),
        "renderer_resume_owner_mismatch")?;
    let retained=load(m,&saved.owner_operation)?;
    require(retained.intent.active_operation==saved.owner_operation,"automatic_renderer_resume_binding")?;
    if !never_submitted_with(m,&retained,probe)? {return Ok(None);}
    Ok(Some(vec![control_artifact(path(m,&saved.owner_operation)?)?,
        retained.selected,control_artifact(retained.before.root.join("environment.json"))?]))
}
fn may_reuse_baseline(m:&Manager,prior:&Continuation,environment:&Environment,
    retired:impl FnOnce()->Result<bool>)->Result<()> {
    let never_submitted=never_submitted_with(m,prior,||renderer_session::inspect(&prior.intent.owner))?;
    let result=renderer_cli::result(m,&prior.intent.owner)?;
    let ui::Action::RendererSoftwareInstall {application:prior_app,policy:prior_policy,..}=validate(m,prior)?.action else {unreachable!()};
    require(never_submitted || result["application_identity"]==prior_app
        && result["requested"]==serde_json::to_value(prior_policy)?,
        "automatic_renderer_prior_result_binding")?;
    require(!job_dir(m,&prior.intent.owner)?.join("renderer-maintenance.json").exists()
        && prior.intent.scan.is_none() && prior.intent.work.is_empty()
        && (never_submitted || retired()? && renderer_session::terminal(&result,&prior.intent.owner)
            && matches!(result["state"].as_str(),Some("failed"|"cancelled")))
        && prior.before==*environment,"automatic_renderer_requires_connection_resume")?;
    Ok(())
}
fn begin(m:&Manager,application:&str,policy:renderer::RendererPolicy,daw:ui::DawChoice,
    owner:&str)->Result<Continuation> {
    let app=renderer_cli::current_application(m)?;
    require(app.identity()?==application,"renderer_application_selection_changed")?;app.verify(&m.root)?;
    require(renderer_cli::all_retired(m)?,"renderer_previous_cleanup_unconfirmed")?;
    let sw=prep::build::launch_software(m)?;
    let selected=control_artifact(renderer_session::directory(m).join("application.json"))?;
    let prior=latest(m)?;
    let mut intent=Intent {installer_policy:None,owner:owner.into(),active_operation:owner.into(),vendor:None,
        journey:ui::InstallationJourney {daw,phase:ui::InstallationPhase::PreparingEnvironment,
            plugins:vec![],failure:None},scan:None,prior_selections:vec![],prior_modules:vec![],
        failed_modules:vec![],work:vec![],predecessors:vec![],previous:None};
    let mut capture_baseline=true;
    if let Some(prior)=prior {
        require(prior.selected==selected,"automatic_renderer_application_changed")?;
        let empty=prior.intent.journey.plugins.is_empty() && prior.intent.scan.is_some()
            && prior.intent.journey.failure.as_ref().is_some_and(|f|f.code=="no_supported_instruments_or_effects_discovered");
        if prior.intent.journey.phase!=ui::InstallationPhase::Available && !empty {
            may_reuse_baseline(m,&prior,&app.environment,||renderer_session::retired(m,&prior.intent.owner))?;
            capture_baseline=false;
            intent.prior_selections=prior.intent.prior_selections.clone();
            intent.prior_modules=prior.intent.prior_modules.clone();
            intent.predecessors=prior.intent.predecessors.iter()
                .map(|predecessor|package_authority::renewed_predecessor(m,predecessor)).collect::<Result<Vec<_>>>()?;
        }
        intent.previous=Some(retain_previous(m,&prior)?);
    }
    if capture_baseline {
        intent.prior_selections=prep::selections(m,&sw.host,&sw.source_sha256)?.iter()
            .filter(|s|s.environment.id==app.environment.id)
            .map(|s|format!("{}:{}",s.module.sha256,s.class.id)).collect();
        let inventory=m.root.join("inventory").join(format!("{}.json",app.environment.id));
        intent.prior_modules=if inventory.exists(){read_json::<inventory::Scan>(&inventory)?.modules.iter()
            .map(|module|module.artifact.sha256.clone()).collect()}else{vec![]};
        for (class,entry) in m.registry()?.classes {
            if entry.registration.environment.id==app.environment.id {
                let reference=entry.managed_revision.as_ref().ok_or("renderer_existing_publication_authority_missing")?;
                intent.predecessors.push(m.load_revision(&class,reference)?);
            }
        }
    }
    let retained=Continuation {schema:1,request:control_artifact(job_dir(m,owner)?.join("request.json"))?,
        selected,before:app.environment,intent};validate(m,&retained)?;
    require(matches!(validate(m,&retained)?.action,ui::Action::RendererSoftwareInstall {policy:p,..} if p==policy),
        "automatic_renderer_request_binding")?;
    atomic_json(&path(m,owner)?,&retained)?;Ok(retained)
}
pub(super) fn execute(m:&Manager,action:&ui::Action,operation:&str,
    capacity:&dyn Fn()->Option<CapacityReadback>,timeout:Duration,waits:&mut Vec<ui::LockFacts>)->Result<Value> {
    let _intent=m.lock("automatic-renderer.lock")?;
    let mut retained=match action {
        ui::Action::RendererSoftwareInstall {application,policy,daw}=>begin(m,application,*policy,*daw,operation)?,
        ui::Action::EnvironmentRefreshResume {operation:owner}=>load(m,owner)?,
        _=>return Err("automatic_renderer_action".into()),
    };
    let environment=if matches!(action,ui::Action::RendererSoftwareInstall {..}) {
        let ui::Action::RendererSoftwareInstall {application,policy,..}=action else {unreachable!()};
        let _environment=m.lock("operator-environment.lock")?;
        save(m,&retained.before,&retained.intent,operation)?;
        suspend(m,operation,Some(operation.into()),timeout,waits)?;
        let launch=(||->Result<Environment> {
            let submission=match renderer_cli::launch(m,application,*policy,operation) {
                Ok(submission)=>submission,
                Err(error)=>{if renderer_cli::current(m)?["operation"]!=operation
                    && !renderer_session::operation_dir(m,operation)?.join("spec.json").exists() {
                        return Err(error);
                    }
                    renderer_cli::reconcile(m,operation)?},
            };
            retained.intent.journey.phase=ui::InstallationPhase::VendorRunning;
            save(m,&retained.before,&retained.intent,operation)?;
            if submission==renderer_session::Submission::AcknowledgmentUncertain {
                write_operation(m,operation,&json!({"schema":2,"operation":operation,
                    "state":"submission_uncertain","renderer_owner":operation,"journey":retained.intent.journey}),false)?;
            }
            wait_renderer_retirement(||{renderer_cli::reconcile(m,operation)?;
                renderer_session::retired(m,operation)},||std::thread::sleep(Duration::from_millis(500)))?;
            commit(m,&retained)
        })();
        if let Err(error)=&launch {
            retained.intent.journey.phase=ui::InstallationPhase::NeedsAttention;
            retained.intent.journey.failure=Some(automatic_install::failure(ui::InstallationFailureStage::VendorInstallation,error));
            let environment=if job_dir(m,&retained.intent.owner)?.join("renderer-maintenance.json").exists() {
                environment_revision::renderer_record(m,&retained.intent.owner)?.environment
            } else {retained.before.clone()};
            save(m,&environment,&retained.intent,operation)?;
        }
        // Full cohort retirement is checked by existing restoration. Release
        // the environment lock before the shared scanner acquires it again.
        resume_owned(m,operation)?;let environment=launch?;drop(_environment);environment
    } else {
        let _environment=m.lock("operator-environment.lock")?;
        let environment=commit(m,&retained)?;
        completed_receipt(m,&retained.intent,&environment)?;
        require(retained.intent.journey.phase!=ui::InstallationPhase::Available,"automatic_renderer_already_complete")?;
        drop(_environment);environment
    };
    let result=automatic_install::continue_renderer(m,&environment,&mut retained.intent,operation,capacity,timeout,waits);
    if let Err(error)=&result {
        retained.intent.journey.phase=ui::InstallationPhase::NeedsAttention;
        retained.intent.journey.failure=Some(automatic_install::failure(ui::InstallationFailureStage::Discovery,error));
        save(m,&environment,&retained.intent,operation)?;
    }
    result
}

pub(super) fn scan_custody(m:&Manager,saved:&ResumeRecord)->Result<Vec<Artifact>> {
    require(saved.vendor_operation.is_none(),"renderer_scan_owner_phase")?;
    let request:ui::Request=read_json(&job_dir(m,&saved.owner_operation)?.join("request.json"))?;
    let owner=match request.action {
        ui::Action::RendererSoftwareInstall {..}=>saved.owner_operation.as_str(),
        ui::Action::EnvironmentRefreshResume {ref operation} if owns(m,operation)?=>operation.as_str(),
        _=>return Err("renderer_scan_owner_phase".into()),
    };
    let retained=load(m,owner)?;
    require(retained.intent.active_operation==saved.owner_operation,"automatic_renderer_resume_binding")?;
    let edge=environment_revision::renderer_record(m,owner)?;
    completed_receipt(m,&retained.intent,&edge.environment)?;
    Ok(vec![control_artifact(path(m,owner)?)?,
        control_artifact(job_dir(m,owner)?.join("renderer-maintenance.json"))?,edge.spec,edge.result])
}

pub(super) fn current_journey(m:&Manager,retained:&Continuation)->Result<ui::InstallationJourney> {
    let mut journey=retained.intent.journey.clone();
    if journey.phase==ui::InstallationPhase::NeedsAttention && journey.plugins.is_empty()
        && retained.intent.scan.is_some() && retained.intent.work.is_empty()
        && retained.intent.failed_modules.is_empty()
        && journey.failure.as_ref().is_some_and(|f|f.code=="no_supported_instruments_or_effects_discovered") {
        let edge=environment_revision::renderer_record(m,&retained.intent.owner)?;
        completed_receipt(m,&retained.intent,&edge.environment)?;
        journey.phase=ui::InstallationPhase::AwaitingApplication;journey.failure=None;
    }
    automatic_install::explain_journey(m,&retained.intent,&mut journey);
    Ok(journey)
}
pub(super) fn project(m:&Manager,app:&mut ui::VendorApplication,
    setups:Option<&mut [ui::InstallerSetup]>,busy:Option<&str>)->Result<()> {
    let Some(retained)=latest(m)? else {
        for offer in &mut app.actions {
            if let ui::Action::RendererOpen {application,policy}=&offer.action {
                offer.action=ui::Action::RendererSoftwareInstall {application:application.clone(),policy:*policy,daw:ui::DawChoice::Other};
            }
        }
        return Ok(());
    };
    require(app.details["environment"]==retained.before.id,"automatic_renderer_application_changed")?;
    let journey=current_journey(m,&retained)?;
    app.details["journey"]=serde_json::to_value(&journey)?;
    let connecting=(job_dir(m,&retained.intent.owner)?.join("renderer-maintenance.json").exists()
        || successful_result(m,&retained).is_ok())
        && !matches!(journey.phase,ui::InstallationPhase::Available | ui::InstallationPhase::AwaitingApplication);
    let mut offers=vec![];
    let mut resume=false;
    for mut offer in std::mem::take(&mut app.actions) {
        if let ui::Action::RendererOpen {application,policy}=offer.action {
            if connecting {
                if !resume {
                    offers.push(action("Continue connecting plug-ins",ui::Action::EnvironmentRefreshResume {
                        operation:retained.intent.owner.clone()},busy.or(offer.disabled_reason.as_deref())));
                    resume=true;
                }
            } else {
                offer.action=ui::Action::RendererSoftwareInstall {application,policy,daw:ui::DawChoice::Other};
                offers.push(offer);
            }
        } else {offers.push(offer);}
    }
    app.actions=offers;
    if let Some(setups)=setups {
        for setup in setups.iter_mut().filter(|setup|setup.environment.as_deref()==Some(retained.before.id.as_str())) {
            if setup.journey.is_none() || setup.journey.as_ref().is_some_and(|j|
                j.phase==ui::InstallationPhase::Available) {
                setup.journey=Some(journey.clone());
                if connecting {setup.primary=app.actions.iter().find(|a|
                    matches!(a.action,ui::Action::EnvironmentRefreshResume {..})).cloned();}
            }
        }
    }
    Ok(())
}
pub(super) fn finish(m:&Manager,operation:&str,action:&ui::Action)->Result<()> {
    let owner=match action {
        ui::Action::RendererSoftwareInstall {..}=>operation,
        ui::Action::EnvironmentRefreshResume {operation}=>operation,
        _=>return Ok(()),
    };
    if !path(m,owner)?.exists(){return Ok(());}
    let mut retained=load(m,owner)?;
    if retained.intent.active_operation!=operation{return Ok(());}
    if !job_dir(m,owner)?.join("renderer-maintenance.json").exists()
        && renderer_cli::current(m)?["operation"]==owner {
        renderer_cli::reconcile(m,owner)?;
        if !renderer_session::retired(m,owner)? {renderer_cli::stop(m,owner)?;}
        if successful_result(m,&retained).is_ok() {commit(m,&retained)?;}
    }
    let state=optional(&job_dir(m,operation)?.join("result.json"))?;
    if !matches!(state["state"].as_str(),Some("completed"|"refused"))
        && retained.intent.journey.phase!=ui::InstallationPhase::Available
        && retained.intent.journey.failure.is_none() {
        retained.intent.journey.phase=ui::InstallationPhase::NeedsAttention;
        retained.intent.journey.failure=Some(automatic_install::failure(
            ui::InstallationFailureStage::Preparation,"automatic_renderer_connection_interrupted"));
        let environment=if job_dir(m,owner)?.join("renderer-maintenance.json").exists() {
            environment_revision::renderer_record(m,owner)?.environment
        } else {retained.before.clone()};
        save(m,&environment,&retained.intent,operation)?;
    }
    automatic_install::cleanup_connections(m,&retained.intent)
}

#[cfg(test)] mod tests {
    use super::*;
    use sha2::{Digest,Sha256};
    fn fixture()->(test_fixture::Fixture,Continuation) {
        let (f,_)=crate::preparation_cli::tests::projection_fixture();let mut before=f.r.environment.clone();
        before.id=renderer::ENVIRONMENT.into();before.root=f.m.root.join("environments").join(&before.id);
        fs::create_dir_all(&before.root).unwrap();atomic_json(&before.root.join("environment.json"),&before).unwrap();
        let observation:Value=serde_json::from_slice(renderer::OBSERVATION).unwrap();let observation=&observation["result"];
        // Retained public identity metadata tests job/history binding. There are
        // no proprietary images here and this fixture cannot authorize a launch.
        let image_root=before.root.join("compatdata/pfx/drive_c/Program Files/Source fixture");
        let files=observation["identity"]["files"].as_object().unwrap().iter().map(|(name,v)|
            (name.clone(),renderer::Image {artifact:Artifact {path:image_root.join(name),
                sha256:v["sha256"].as_str().unwrap().into()},size:v["size"].as_u64().unwrap()})).collect();
        let app=renderer::Application {schema:1,id:renderer::ID.into(),environment:before.clone(),files,
            installation:Artifact {path:f.m.root.join("onboarding").join(&before.id)
                .join(format!("{}-result.json",renderer::INSTALLATION)),sha256:observation["input"]["result_sha256"].as_str().unwrap().into()},
            observation_sha256:hex(&Sha256::digest(renderer::OBSERVATION)),source_seal_sha256:renderer::SEAL.into()};
        app.validate_record(&f.m.root).unwrap();assert!(app.verify(&f.m.root).is_err());
        let selected=renderer_session::directory(&f.m).join("application.json");private_dir(selected.parent().unwrap()).unwrap();
        atomic_json(&selected,&app).unwrap();let owner="ab".repeat(16);let directory=job_dir(&f.m,&owner).unwrap();private_dir(&directory).unwrap();
        let request=ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:"fixture".into(),action:ui::Action::RendererSoftwareInstall {
            application:app.identity().unwrap(),policy:renderer::RendererPolicy::SoftwareRendering,daw:ui::DawChoice::Bitwig}};
        atomic_json(&directory.join("request.json"),&request).unwrap();
        let intent=Intent {installer_policy:None,owner:owner.clone(),active_operation:owner.clone(),vendor:None,
            journey:ui::InstallationJourney {daw:ui::DawChoice::Bitwig,phase:ui::InstallationPhase::Discovering,plugins:vec![],failure:None},
            scan:None,prior_selections:vec![],prior_modules:vec![],failed_modules:vec![],work:vec![],predecessors:vec![],previous:None};
        let retained=Continuation {schema:1,request:control_artifact(directory.join("request.json")).unwrap(),
            selected:control_artifact(selected).unwrap(),before,intent};
        atomic_json(&path(&f.m,&owner).unwrap(),&retained).unwrap();
        let session=renderer_session::operation_dir(&f.m,&owner).unwrap();private_dir(&session).unwrap();
        let report=session.join("result.json");atomic_json(&session.join("spec.json"),&json!({"schema":1,"kind":"renderer_application",
            "application":app,"application_identity":app.identity().unwrap(),"operation":owner,
            "renderer_policy":renderer::RendererPolicy::SoftwareRendering,"report":report})).unwrap();
        atomic_json(&report,&json!({"schema":1,"operation":owner,"state":"completed","application_identity":app.identity().unwrap(),
            "requested":renderer::RendererPolicy::SoftwareRendering,"cancelled":false,"error":null,
            "cleanup_confirmed":true,"owned_live":0,"dependency":{"dependency_cleanup_disposition":"graceful_service_retirement",
                "service_retirement_confirmed":true,"process_cleanup_confirmed":true,"forced_cleanup_used":false}})).unwrap();
        (f,retained)
    }
    #[test] fn successful_retirement_can_resume_before_edge_and_settle_both_crash_points_once() {
        let (f,mut retained)=fixture();let owner=retained.intent.owner.clone();
        let request=validate(&f.m,&retained).unwrap();let ui::Action::RendererSoftwareInstall {application,policy,..}=request.action else {panic!()};
        let mut app=ui::VendorApplication {id:renderer::ID.into(),name:"Installed app".into(),version:"1".into(),state:"closed".into(),
            details:json!({"environment":retained.before.id}),actions:vec![action("Open",ui::Action::RendererOpen {application,policy},None)]};
        project(&f.m,&mut app,None,None).unwrap();
        assert!(app.actions.iter().any(|a|a.action==ui::Action::EnvironmentRefreshResume {operation:owner.clone()}));
        assert!(!app.actions.iter().any(|a|matches!(a.action,ui::Action::RendererSoftwareInstall {..})));
        let after=commit_with_retirement(&f.m,&retained,true).unwrap();assert_eq!(after.revision,retained.before.revision+1);
        let edge=job_dir(&f.m,&owner).unwrap().join("renderer-maintenance.json");let bytes=fs::read(&edge).unwrap();
        // Simulate the edge durably written before environment.json advances.
        atomic_json(&retained.before.root.join("environment.json"),&retained.before).unwrap();
        assert_eq!(commit_with_retirement(&f.m,&retained,true).unwrap(),after);
        assert_eq!(fs::read(&edge).unwrap(),bytes);
        let sw=software(&f.m).unwrap();let scan=inventory::Scan {schema:1,id:"cd".repeat(16),environment:after.clone(),host:sw.host,
            host_source_sha256:sw.source_sha256,completed_at:1,modules:vec![],changes:Default::default()};
        private_dir(&f.m.root.join("inventory")).unwrap();atomic_json(&f.m.root.join("inventory").join(format!("{}.json",after.id)),&scan).unwrap();
        let resume="ef".repeat(16);private_dir(&job_dir(&f.m,&resume).unwrap()).unwrap();
        atomic_json(&job_dir(&f.m,&resume).unwrap().join("request.json"),&ui::Request {schema:ui::OPERATOR_SCHEMA,
            state_token:"fixture".into(),action:ui::Action::EnvironmentRefreshResume {operation:owner.clone()}}).unwrap();
        let value=automatic_install::continue_renderer_fixture(&f.m,&after,&mut retained.intent,&resume,
            &mut |action,_,_|match action {ui::Action::EnvironmentRescan {environment}=>{
                assert_eq!(environment,&after.id);Ok(json!({"scan":scan.id}))},other=>panic!("retry must never relaunch: {other:?}")}).unwrap();
        assert_eq!(value["available"],false);assert_eq!(retained.intent.journey.phase,ui::InstallationPhase::NeedsAttention);
        let saved=load(&f.m,&owner).unwrap();assert_eq!(saved.intent.active_operation,resume);assert_eq!(saved.intent.journey.daw,ui::DawChoice::Bitwig);
        assert_eq!(current_journey(&f.m,&saved).unwrap().phase,ui::InstallationPhase::AwaitingApplication);
    }
    #[test] fn connection_after_native_access_waits_out_a_status_read_instead_of_refusing() {
        use linux_vst_bridge::operator_lock::{with_operator_registry_wait,LockBusy};
        let (f,retained)=fixture();
        // An open manager window's status read holds registry.lock briefly.
        let window=Manager {root:f.m.root.clone(),publications:f.m.publications.clone()};
        let held=window.lock("registry.lock").unwrap();
        // Before: the first step after the session refused at once ("operation already running").
        assert!(commit_with_retirement(&f.m,&retained,true).unwrap_err().is::<LockBusy>());
        assert!(!job_dir(&f.m,&retained.intent.owner).unwrap().join("renderer-maintenance.json").exists());
        // Within its bound the action waits; past it, the same busy refusal and no change.
        assert!(with_operator_registry_wait(Duration::from_millis(80),
            ||commit_with_retirement(&f.m,&retained,true)).unwrap_err().is::<LockBusy>());
        let release=std::thread::spawn(move||{std::thread::sleep(Duration::from_millis(300));drop(held);});
        let started=std::time::Instant::now();
        let after=with_operator_registry_wait(Duration::from_secs(5),
            ||commit_with_retirement(&f.m,&retained,true)).unwrap();
        release.join().unwrap();
        assert!(started.elapsed()>=Duration::from_millis(250));
        assert_eq!(after.revision,retained.before.revision+1);
    }
    #[test] fn malformed_spec_failed_result_and_live_cohort_never_publish_an_edge() {
        for case in 0..4 {
            let (f,retained)=fixture();let owner=&retained.intent.owner;
            let before=fs::read(retained.before.root.join("environment.json")).unwrap();
            let session=renderer_session::operation_dir(&f.m,owner).unwrap();
            if case==0 {let mut spec=optional(&session.join("spec.json")).unwrap();spec["application_identity"]=json!("ff".repeat(32));atomic_json(&session.join("spec.json"),&spec).unwrap();}
            if case==1 {let mut result=optional(&session.join("result.json")).unwrap();result["state"]=json!("failed");atomic_json(&session.join("result.json"),&result).unwrap();}
            if case==2 {let mut result=optional(&session.join("result.json")).unwrap();result["dependency"]["service_retirement_confirmed"]=json!(false);atomic_json(&session.join("result.json"),&result).unwrap();}
            assert!(commit_with_retirement(&f.m,&retained,case!=3).is_err());
            assert!(!job_dir(&f.m,owner).unwrap().join("renderer-maintenance.json").exists());
            assert_eq!(fs::read(retained.before.root.join("environment.json")).unwrap(),before);
        }
    }
    #[test] fn never_submitted_crash_restores_service_and_retains_the_exact_baseline() {
        let (f,mut prior)=fixture();let owner=prior.intent.owner.clone();
        let session=renderer_session::operation_dir(&f.m,&owner).unwrap();
        fs::remove_file(session.join("spec.json")).unwrap();fs::remove_file(session.join("result.json")).unwrap();
        prior.intent.prior_modules=vec!["12".repeat(32)];
        atomic_json(&path(&f.m,&owner).unwrap(),&prior).unwrap();
        write_operation(&f.m,&owner,&json!({"operation":owner,"state":"refused","reason":"operator_worker_terminated"}),false).unwrap();
        let absent=||Ok(renderer_session::UnitState {exists:false,empty:true,observation:None});
        assert!(never_submitted_with(&f.m,&prior,absent).unwrap());
        let sw=software(&f.m).unwrap();let saved=ResumeRecord {schema:2,owner_operation:owner.clone(),resume:true,
            software:sw.manager.sha256,vendor_operation:Some(owner.clone())};
        atomic_json(&m_resume_path(&f.m),&saved).unwrap();
        let custody=package_resume_custody_with_renderer_probe(&f.m,&saved,&validate(&f.m,&prior).unwrap().action,
            |_|absent()).unwrap();assert_eq!(custody.len(),3);
        let mut restored=0;
        resume_interrupted_with_renderer_probe(&f.m,|actual|{assert_eq!(actual.owner_operation,owner);restored+=1;Ok(())},
            |_|absent()).unwrap();
        assert_eq!(restored,1);assert!(!m_resume_path(&f.m).exists());
        let artifact=retain_previous(&f.m,&prior).unwrap();let retained:Continuation=serde_json::from_slice(&artifact.record_bytes(1024*1024).unwrap()).unwrap();
        assert_eq!(retained.intent.prior_modules,prior.intent.prior_modules);
        assert!(renderer_cli::result(&f.m,&owner).unwrap().is_null());
        assert!(!job_dir(&f.m,&owner).unwrap().join("renderer-maintenance.json").exists());
        // A real or unobservable unit is never reclassified as not submitted.
        assert!(never_submitted_with(&f.m,&prior,||Ok(renderer_session::UnitState {
            exists:true,empty:false,observation:None})).is_err());
        assert!(never_submitted_with(&f.m,&prior,||Err("unit_observation_failed".into())).is_err());
        atomic_json(&session.join("spec.json"),&json!({"operation":owner})).unwrap();
        assert!(!never_submitted_with(&f.m,&prior,||panic!("reserved session uses original reconciliation")).unwrap());
    }
    #[test] fn parent_owned_scan_pause_uses_committed_history_and_restores_exactly_once() {
        let (f,retained)=fixture();let owner=retained.intent.owner.clone();
        commit_with_retirement(&f.m,&retained,true).unwrap();
        let sw=software(&f.m).unwrap();let saved=ResumeRecord {schema:2,owner_operation:owner.clone(),resume:true,
            software:sw.manager.sha256,vendor_operation:None};
        atomic_json(&m_resume_path(&f.m),&saved).unwrap();
        write_operation(&f.m,&owner,&json!({"state":"refused","operation":owner,"reason":"interrupted_scan"}),false).unwrap();
        let custody=package_resume_custody(&f.m,&saved,&validate(&f.m,&retained).unwrap().action).unwrap();assert_eq!(custody.len(),4);
        let mut restored=0;resume_interrupted_with(&f.m,|actual|{assert_eq!(actual.owner_operation,owner);restored+=1;Ok(())}).unwrap();
        assert_eq!(restored,1);assert!(!m_resume_path(&f.m).exists());
        fs::remove_file(path(&f.m,&owner).unwrap()).unwrap();
        assert_eq!(records(&f.m).unwrap_err().to_string(),"automatic_renderer_checkpoint_missing");
    }
    fn m_resume_path(m:&Manager)->PathBuf {m.root.join("operator/resume.json")}
}

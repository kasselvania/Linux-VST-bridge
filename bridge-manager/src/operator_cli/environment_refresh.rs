//! Refresh an existing owned setup through the shared connection executor.
//! Its checkpoint belongs to the accepted operator job, never installer history.
use super::*;
use automatic_install::Intent;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Refresh {
    schema: u32,
    request: Artifact,
    preserved_performance: std::collections::BTreeMap<String,Performance>,
    pub environment: Environment,
    pub intent: Intent,
}
fn path(m: &Manager, operation: &str) -> Result<PathBuf> {
    Ok(job_dir(m, operation)?.join("refresh.json"))
}
pub(super) fn load(m: &Manager, operation: &str) -> Result<Refresh> {
    let retained: Refresh = read_json(&path(m, operation)?)?;
    require(retained.request.path==job_dir(m,operation)?.join("request.json"),"automatic_refresh_request_path")?;
    let request: ui::Request = serde_json::from_slice(&retained.request.record_bytes(16384)?)?;
    require(retained.schema == 1 && retained.intent.owner == operation
        && retained.intent.vendor.is_none()
        && matches!(request.action, ui::Action::EnvironmentRefresh {environment, daw}
            if environment == retained.environment.id && daw == retained.intent.journey.daw),
        "automatic_refresh_owner_binding")?;
    Ok(retained)
}
fn retain(m:&Manager,environment:Environment,intent:Intent,
    preserved_performance:std::collections::BTreeMap<String,Performance>)->Result<Refresh> {
    let retained=Refresh {schema:1,request:control_artifact(job_dir(m,&intent.owner)?.join("request.json"))?,
        environment,intent,preserved_performance};
    atomic_json(&path(m,&retained.intent.owner)?,&retained)?;
    load(m,&retained.intent.owner)
}
pub(super) fn records(m: &Manager) -> Result<Vec<Refresh>> {
    let root=m.root.join("operator");
    if !root.is_dir() {return Ok(vec![]);}
    let mut records=vec![];
    for (count,entry) in fs::read_dir(root)?.enumerate() {
        require(count<4096,"operator_refresh_history_bound")?;
        let entry=entry?;
        let owner=entry.file_name().to_string_lossy().into_owned();
        if !valid_hex(&owner,32) {continue;}
        if path(m,&owner)?.exists() {records.push(load(m,&owner)?);}
        else {
            let request=optional(&entry.path().join("request.json"))?;
            if request["action"]["kind"]=="environment_refresh" {
                let result=optional(&entry.path().join("result.json"))?;
                require(!result["journey"].is_object() && !result["result"]["journey"].is_object(),
                    "automatic_refresh_checkpoint_missing")?;
            }
        }
    }
    records.sort_by(|a,b|a.intent.owner.cmp(&b.intent.owner));
    Ok(records)
}
fn unfinished(m: &Manager, environment: &str) -> Result<Option<Refresh>> {
    unfinished_from(&records(m)?,environment)
}
fn unfinished_from(records:&[Refresh],environment:&str)->Result<Option<Refresh>> {
    let mut pending=records.iter().filter(|r|r.environment.id==environment
        && r.intent.journey.phase != ui::InstallationPhase::Available).collect::<Vec<_>>();
    require(pending.len()<=1,"automatic_refresh_multiple_unfinished")?;
    Ok(pending.pop().cloned())
}
pub(super) fn authority(m: &Manager) -> Result<Vec<Artifact>> {
    records(m)?.into_iter().map(|r|control_artifact(path(m,&r.intent.owner)?)).collect()
}
pub(super) fn save(m: &Manager, environment: &Environment, intent: &Intent,
    operation: &str) -> Result<()> {
    let mut retained=load(m,&intent.owner)?;
    require(retained.environment==*environment,"automatic_refresh_environment_changed")?;
    retained.intent=intent.clone();retained.intent.active_operation=operation.into();
    atomic_json(&path(m,&intent.owner)?,&retained)?;
    write_operation(m,operation,&json!({"schema":2,"operation":operation,"state":"running",
        "environment":environment.id,"journey":intent.journey}),false)
}
pub(super) fn require_owner(m: &Manager, environment: &Environment, intent: &Intent) -> Result<()> {
    let retained=load(m,&intent.owner)?;
    require(retained.environment==*environment,"automatic_refresh_environment_changed")
}
pub(super) fn require_preserved(m:&Manager,intent:&Intent,
    selection:&linux_vst_bridge::preparation::Selection)->Result<()> {
    let original=intent.predecessors.iter().find(|p|p.class_id==selection.class.id)
        .ok_or("automatic_refresh_preserved_publication_missing")?;
    require(original.registration.module==selection.module
        && original.registration.environment==selection.environment,"automatic_refresh_preserved_publication_changed")?;
    require_preserved_baseline(m,intent)
}
pub(super) fn require_preserved_baseline(m:&Manager,intent:&Intent)->Result<()> {
    let retained=load(m,&intent.owner)?;
    let db=m.registry()?;
    for original in &intent.predecessors {
    let class=&original.class_id;
    let entry=db.classes.get(class).ok_or("automatic_refresh_preserved_publication_changed")?;
    let reference=entry.managed_revision.as_ref().ok_or("automatic_refresh_preserved_publication_changed")?;
    require(entry.publication==Publication::Published && original.id==reference.id
        && original.registration==entry.registration
        && m.load_revision(class,reference)?==*original
        && retained.preserved_performance.get(class)==Some(&m.performance(&original.registration.key())?),
        "automatic_refresh_preserved_publication_changed")?;
    original.registration.module.verify()?;
    m.verify_completed_publication(original,reference)?;
    }
    Ok(())
}
pub(super) fn project(m: &Manager, setups: &mut [ui::InstallerSetup],
    environments: &[ui::Environment], products:&[ui::Product], busy: Option<&str>) -> Result<()> {
    let records=records(m)?;
    for setup in setups {
        let Some(environment)=setup.environment.as_deref() else {continue};
        let Some(context)=environments.iter().find(|r|r.id==environment) else {continue};
        let Some(scan)=context.actions.iter().find(|a|matches!(a.action,ui::Action::EnvironmentRescan {..})) else {continue};
        if let Some(retained)=unfinished_from(&records,environment)? {
            setup.journey=Some(retained.intent.journey.clone());
            setup.primary=Some(action("Continue connecting plug-ins",
                ui::Action::EnvironmentRefreshResume {operation:retained.intent.owner},
                if retained.intent.journey.failure.as_ref().is_some_and(|f|!f.retryable) {
                    Some("This setup needs its separate configuration review before connection can continue.")
                } else {busy.or(scan.disabled_reason.as_deref())}));
        } else if setup.journey.is_none() && setup.primary.is_none()
            && matches!(setup.phase,ui::SetupPhase::SetupNeedsAttention | ui::SetupPhase::DiscoveryComplete)
            && setup.discovered.iter().any(|d|
                !products.iter().any(|p|p.environment==d.environment && p.class_id==d.class_id
                    && p.module_sha256==d.module_sha256 && p.availability.as_ref().is_some_and(|a|a.available))) {
            setup.status="Find installed instruments and effects and connect missing plug-ins to your DAW. Working plug-ins keep their saved setup.".into();
            setup.primary=Some(action("Find and connect installed plug-ins",
                ui::Action::EnvironmentRefresh {environment:environment.into(),daw:ui::DawChoice::Other},
                busy.or(scan.disabled_reason.as_deref())));
        }
    }
    Ok(())
}
pub(super) fn execute(m: &Manager, action: &ui::Action, operation: &str,
    capacity: &dyn Fn()->Option<CapacityReadback>, timeout: Duration,
    waits: &mut Vec<ui::LockFacts>) -> Result<Value> {
    let environment=match action {
        ui::Action::EnvironmentRefresh {environment,..}=>environment.clone(),
        ui::Action::EnvironmentRefreshResume {operation:owner}=>load(m,owner)?.environment.id,
        _=>return Err("automatic_refresh_action".into()),
    };
    require(valid_product_environment(&environment),"operator_product_identity")?;
    let _intent=m.lock(&format!("automatic-refresh-{environment}.lock"))?;
    let sw=software(m)?;
    let db=m.registry()?;
    let catalogue=operator_catalogue(m,&sw,&db)?;
    let current=managed_rescan_binding(m,catalogue.as_ref(),&db,&environment)?
        .ok_or("operator_environment_not_managed")?;
    let mut retained=match action {
        ui::Action::EnvironmentRefresh {daw,..}=> {
            require(unfinished(m,&environment)?.is_none(),"automatic_refresh_requires_resume")?;
            let captured=current::capture(m)?;
            let ready:Vec<String>=captured.snapshot.products.iter().filter(|p|p.environment==environment
                && p.availability.as_ref().is_some_and(|a|a.available))
                .map(|p|format!("{}:{}",p.module_sha256,p.class_id)).collect();
            let mut predecessors=vec![];
            let mut preserved_performance=std::collections::BTreeMap::new();
            for (class,entry) in &db.classes {
                if ready.contains(&format!("{}:{}",entry.registration.module.sha256,class)) {
                    let selected=entry.managed_revision.as_ref().ok_or("automatic_refresh_preserved_publication_missing")?;
                    predecessors.push(m.load_revision(class,selected)?);
                    preserved_performance.insert(class.clone(),m.performance(&entry.registration.key())?);
                }
            }
            let intent=Intent {owner:operation.into(),active_operation:operation.into(),vendor:None,
                journey:ui::InstallationJourney {daw:*daw,phase:ui::InstallationPhase::Discovering,
                    plugins:vec![],failure:None},scan:None,prior_selections:ready,prior_modules:vec![],
                failed_modules:vec![],work:vec![],predecessors,previous:None};
            retain(m,current.clone(),intent,preserved_performance)?
        },
        ui::Action::EnvironmentRefreshResume {operation:owner}=> {
            let retained=load(m,owner)?;
            require(retained.environment==current,"automatic_refresh_environment_changed")?;
            require(retained.intent.journey.phase!=ui::InstallationPhase::Available,
                "automatic_refresh_already_complete")?;retained
        },
        _=>unreachable!(),
    };
    require_preserved_baseline(m,&retained.intent)?;
    let result=automatic_install::continue_refresh(m,&retained.environment,&mut retained.intent,
        operation,capacity,timeout,waits);
    if let Err(error)=&result {
        retained.intent.journey.phase=ui::InstallationPhase::NeedsAttention;
        if retained.intent.journey.failure.is_none() {
            retained.intent.journey.failure=Some(automatic_install::failure(
                ui::InstallationFailureStage::Discovery,error));
        }
        save(m,&retained.environment,&retained.intent,operation)?;
    }
    result
}
pub(super) fn finish(m: &Manager, operation: &str, action: &ui::Action) -> Result<()> {
    let owner=match action {
        ui::Action::EnvironmentRefresh {..}=>operation,
        ui::Action::EnvironmentRefreshResume {operation}=>operation,
        _=>return Ok(()),
    };
    if !path(m,owner)?.exists() {return Ok(());}
    let mut retained=load(m,owner)?;
    if retained.intent.active_operation!=operation {
        return Ok(());
    }
    let state=optional(&job_dir(m,operation)?.join("result.json"))?;
    if !matches!(state["state"].as_str(),Some("completed" | "refused"))
        && retained.intent.journey.phase!=ui::InstallationPhase::Available
        && retained.intent.journey.failure.is_none() {
        retained.intent.journey.phase=ui::InstallationPhase::NeedsAttention;
        retained.intent.journey.failure.get_or_insert_with(||automatic_install::failure(
            ui::InstallationFailureStage::Preparation,"automatic_refresh_interrupted"));
        save(m,&retained.environment,&retained.intent,operation)?;
    }
    automatic_install::cleanup_connections(m,&retained.intent)
}

#[cfg(test)] mod tests {
    use super::*;
    use linux_vst_bridge::preparation as prep;
    fn fixture(checkpoint:bool) -> (test_fixture::Fixture,Refresh,prep::Selection) {
        let (f,c,_,_)=preparation_cli::tests::guided_fixture_with_role(false);
        prep::retain_inspection(&f.m,&c.inspection).unwrap();prep::record_candidate(&f.m,&c).unwrap();
        let selected=prep::enable(&f.m,&c,false).unwrap();
        let predecessor=f.m.load_revision(&c.selection.class.id,&selected).unwrap();
        let inventory=f.m.root.join("inventory").join(format!("{}.json",c.selection.environment.id));
        let mut scan:inventory::Scan=read_json(&inventory).unwrap();
        let raw:Value=read_json(&c.selection.factory_report.path).unwrap();
        let report=f.outer.join("fresh-multiclass-factory.json");
        let mut missing=None;
        for value in 16u8..80 {
            let mut observed=raw.clone();let mut sibling=observed["records"][1]["classes"][0].clone();
            sibling["raw_tuid_hex"]=json!(format!("{value:02X}").repeat(16));
            sibling["name_hex"]=json!(hex(&"Unpublished sibling".encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>()));
            observed["records"][1]["classes"].as_array_mut().unwrap().push(sibling);
            observed["records"][1]["class_count"]=json!(3);
            observed["error"]=json!("multiple audio classes require explicit selection");
            observed["records"].as_array_mut().unwrap().push(json!({"state":"ap8_failure",
                "reason":"multiple audio classes require explicit selection"}));
            atomic_json(&report,&observed).unwrap();
            scan.modules[0].report=Artifact {path:report.clone(),sha256:digest(&report).unwrap()};
            scan.modules[0].classes=inventory::classes(&observed).unwrap();
            scan.modules[0].inspection_error=Some("multiple audio classes require explicit selection".into());
            atomic_json(&inventory,&scan).unwrap();
            let mut choices=prep::selections(&f.m,&c.selection.scanner,&c.selection.scanner_source).unwrap();
            choices.sort_by_key(|s|s.id().unwrap());
            if choices[0].class.id!=c.selection.class.id {missing=Some(choices[0].clone());break;}
        }
        let missing=missing.expect("fixture puts the missing class before the preserved binding check");
        let owner=random_id().unwrap();private_dir(&job_dir(&f.m,&owner).unwrap()).unwrap();
        atomic_json(&job_dir(&f.m,&owner).unwrap().join("request.json"),&ui::Request {
            schema:ui::OPERATOR_SCHEMA,state_token:"accepted-fixture".into(),
            action:ui::Action::EnvironmentRefresh {environment:c.selection.environment.id.clone(),daw:ui::DawChoice::Bitwig}}).unwrap();
        let preserved_performance=std::collections::BTreeMap::from([(c.selection.class.id.clone(),f.m.performance(&c.selection.class.id).unwrap())]);
        let intent=Intent {owner:owner.clone(),active_operation:owner,vendor:None,
            journey:ui::InstallationJourney {daw:ui::DawChoice::Bitwig,phase:ui::InstallationPhase::Discovering,
                plugins:vec![],failure:None},scan:None,
            prior_selections:vec![format!("{}:{}",c.selection.module.sha256,c.selection.class.id)],
            prior_modules:vec![],failed_modules:vec![],work:vec![],predecessors:vec![predecessor],previous:None};
        let retained=if checkpoint {retain(&f.m,c.selection.environment,intent,preserved_performance).unwrap()}
            else {Refresh {schema:1,request:control_artifact(job_dir(&f.m,&intent.owner).unwrap().join("request.json")).unwrap(),
                environment:c.selection.environment,intent,preserved_performance}};
        (f,retained,missing)
    }
    #[test] fn refresh_automatically_attempts_discovered_missing_sibling_and_preserves_working_publication() {
        let (f,mut retained,missing)=fixture(true);let owner=retained.intent.owner.clone();
        let registry=fs::read(f.m.root.join("registry.json")).unwrap();
        let publications=test_fixture::snapshot(&f.m.publications);
        let environment=fs::read(retained.environment.root.join("environment.json")).unwrap();
        let mut scanned=0;let mut checked=0;
        let value=automatic_install::continue_refresh_fixture(&f.m,&retained.environment,&mut retained.intent,
            &owner,&mut |action,_,_|match action {
                ui::Action::EnvironmentRescan {environment}=> {scanned+=1;
                    let scan:inventory::Scan=read_json(&f.m.root.join("inventory").join(format!("{environment}.json")))?;
                    Ok(json!({"scan":scan.id}))},
                ui::Action::CompatibilityCheck {selection,..}=> {assert_eq!(*selection,missing.id().unwrap());checked+=1;
                    Err("fixture_sibling_inspection_failed".into())},
                other=>panic!("unexpected mutation of working sibling: {other:?}"),
            }).unwrap();
        assert_eq!(scanned,1);assert_eq!(checked,1);assert_eq!(value["available"],false);
        assert_eq!(value["environment"],retained.environment.id);assert!(value.get("onboarding").is_none());
        assert_eq!(retained.intent.journey.plugins.len(),2);
        assert_eq!(retained.intent.journey.plugins.iter().filter(|p|p.stage==ui::InstallationPluginStage::Available).count(),1);
        assert!(retained.intent.failed_modules.is_empty(),"explicit class selection is not a failed discovery module");
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),registry);
        assert_eq!(test_fixture::snapshot(&f.m.publications),publications);
        assert_eq!(fs::read(retained.environment.root.join("environment.json")).unwrap(),environment);
    }
    #[test] fn failed_late_collection_cannot_checkpoint_or_drop_an_earlier_missing_sibling_on_retry() {
        let (f,mut retained,missing)=fixture(true);let owner=retained.intent.owner.clone();
        let class=retained.intent.predecessors[0].class_id.clone();
        let performance=retained.preserved_performance[&class].clone();let mut changed=performance.clone();changed.added_frames=if performance.added_frames==256 {512} else {256};
        let preference=f.m.root.join("performance").join(format!("{class}.json"));
        fs::create_dir_all(preference.parent().unwrap()).unwrap();atomic_json(&preference,&changed).unwrap();
        let mut run=|action:&ui::Action,_:&str,_:&mut dyn FnMut(&str)|match action {
            ui::Action::EnvironmentRescan {environment}=> {let scan:inventory::Scan=read_json(&f.m.root.join("inventory").join(format!("{environment}.json")))?;Ok(json!({"scan":scan.id}))},
            ui::Action::CompatibilityCheck {selection,..}=> {assert_eq!(*selection,missing.id().unwrap());Err("fixture_sibling_inspection_failed".into())},
            other=>panic!("unexpected {other:?}"),
        };
        assert_eq!(automatic_install::continue_refresh_fixture(&f.m,&retained.environment,&mut retained.intent,&owner,&mut run)
            .unwrap_err().to_string(),"automatic_refresh_preserved_publication_changed");
        assert!(retained.intent.scan.is_none());assert!(retained.intent.work.is_empty());assert!(retained.intent.journey.plugins.is_empty());
        save(&f.m,&retained.environment,&retained.intent,&owner).unwrap();
        atomic_json(&preference,&performance).unwrap();
        let mut retry=load(&f.m,&owner).unwrap();
        let value=automatic_install::continue_refresh_fixture(&f.m,&retry.environment,&mut retry.intent,&owner,&mut run).unwrap();
        assert_eq!(value["available"],false);assert_eq!(retry.intent.journey.plugins.len(),2);
        assert!(retry.intent.journey.plugins.iter().any(|p|p.class_id==missing.class.id
            && p.stage==ui::InstallationPluginStage::NeedsAttention));
    }
    #[test] fn normal_finalization_preserves_terminal_partial_results_and_recovers_interruption() {
        for state in ["completed","refused","running"] {
            let (f,mut retained,_)=fixture(true);let owner=retained.intent.owner.clone();
            if state!="running" {
                retained.intent.journey.phase=ui::InstallationPhase::NeedsAttention;
                retained.intent.journey.failure=Some(automatic_install::failure(
                    ui::InstallationFailureStage::Inspection,"exact_retained_failure"));
            }
            save(&f.m,&retained.environment,&retained.intent,&owner).unwrap();
            write_operation(&f.m,&owner,&json!({"state":state,"operation":owner,
                "journey":retained.intent.journey,"reason":"exact_retained_failure"}),false).unwrap();
            let result=job_dir(&f.m,&owner).unwrap().join("result.json");
            let before=fs::read(&result).unwrap();let checkpoint=fs::read(path(&f.m,&owner).unwrap()).unwrap();
            finish(&f.m,&owner,&ui::Action::EnvironmentRefresh {environment:retained.environment.id,
                daw:retained.intent.journey.daw}).unwrap();
            finish_operation_with(&f.m,&owner,|_|panic!("no service suspension in this fixture")).unwrap();
            if state=="running" {
                let current=load(&f.m,&owner).unwrap();
                assert_eq!(current.intent.journey.phase,ui::InstallationPhase::NeedsAttention);
                assert_eq!(current.intent.journey.failure.unwrap().code,"automatic_refresh_interrupted");
                assert_eq!(optional(&result).unwrap()["state"],"refused");
            } else {assert_eq!(fs::read(&result).unwrap(),before);
                assert_eq!(fs::read(path(&f.m,&owner).unwrap()).unwrap(),checkpoint);}
        }
    }
    #[test] fn missing_or_changed_refresh_checkpoint_is_authority_loss_not_a_fresh_baseline() {
        let (f,retained,_)=fixture(true);let owner=retained.intent.owner;
        write_operation(&f.m,&owner,&json!({"state":"running","journey":retained.intent.journey}),false).unwrap();
        fs::remove_file(path(&f.m,&owner).unwrap()).unwrap();
        assert_eq!(records(&f.m).unwrap_err().to_string(),"automatic_refresh_checkpoint_missing");
    }
    #[test] fn historical_partial_setup_stays_settled_but_a_missing_exact_sibling_gets_refresh() {
        let (f,retained,_)=fixture(false);
        let mut snapshot=snapshot_idle_test(&f.m).unwrap();
        let discovered=snapshot.products.iter().map(|p|ui::DiscoveredProduct {environment:p.environment.clone(),
            module_sha256:p.module_sha256.clone(),class_id:p.class_id.clone(),name:p.name.clone()}).collect();
        let mut setup=ui::InstallerSetup {installer:"ab".repeat(32),name:"Retained partial setup".into(),
            label_source:"operator_named".into(),byte_size:1024,format:"pe_executable".into(),imported_at:1,
            phase:ui::SetupPhase::SetupNeedsAttention,journey:None,status:"Installation changes observed; completion not verified".into(),
            environment:Some(retained.environment.id.clone()),compatibility:None,discovered,
            primary:None,secondary:vec![],rename:action("Rename",ui::Action::InstallerRename {
                installer:"ab".repeat(32),label:String::new()},None),history:vec![]};
        let history=setup.status.clone();
        assert_eq!(setup.discovered.len(),2);
        project(&f.m,std::slice::from_mut(&mut setup),&snapshot.environments,&snapshot.products,None).unwrap();
        assert!(matches!(setup.primary.as_ref().unwrap().action,ui::Action::EnvironmentRefresh {..}));
        setup.primary=None;setup.phase=ui::SetupPhase::DiscoveryComplete;
        project(&f.m,std::slice::from_mut(&mut setup),&snapshot.environments,&snapshot.products,None).unwrap();
        assert!(matches!(setup.primary.as_ref().unwrap().action,ui::Action::EnvironmentRefresh {..}));
        setup.primary=None;
        for product in &mut snapshot.products {product.availability=Some(ui::NativeAvailability {available:true,reason:None});}
        setup.status=history.clone();
        project(&f.m,std::slice::from_mut(&mut setup),&snapshot.environments,&snapshot.products,None).unwrap();
        assert!(setup.primary.is_none());assert!(setup.journey.is_none());assert_eq!(setup.status,history);
    }
}

//! Companion installation extends the existing operator transaction and installer supervisor.
//! The environment revision, preparation history and publication remain their existing owners.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub schema: u32,
    pub operation: String,
    pub installer: String,
    pub before: Environment,
    pub environment: Environment,
    pub created_at: u64,
}

pub(crate) fn same_space(before: &Environment, after: &Environment) -> bool {
    before.id == after.id && before.root == after.root && before.runner == after.runner
        && before.revision <= after.revision
}

pub(crate) fn current_environment(m: &Manager, bound: &Environment) -> Result<Environment> {
    require(bound.root == m.root.join("environments").join(&bound.id)
        && valid_product_environment(&bound.id), "installer_environment_identity")?;
    let current: Environment = read_json(&bound.root.join("environment.json"))?;
    require(same_space(bound, &current), "installer_environment_binding_changed")?;
    if current != *bound {
        // Only a retained installation transaction authorizes a revision change.
        let history=records(m)?;
        let mut step=bound.clone();
        for _ in 0..history.len() {
            if step == current {break;}
            step=history.iter().find(|r|r.before == step)
                .ok_or("installer_environment_revision_unowned")?.environment.clone();
        }
        require(step == current,"installer_environment_revision_unowned")?;
    }
    Ok(current)
}

pub(super) fn load(m: &Manager, operation: &str) -> Result<Binding> {
    let dir = job_dir(m, operation)?;
    let r: Binding = read_json(&dir.join("installation.json"))?;
    let request: ui::Request = read_json(&dir.join("request.json"))?;
    require(r.schema == 1 && r.operation == operation && valid_hex(&r.installer,64)
        && request.action == (ui::Action::EnvironmentInstallerStart {
            environment:r.environment.id.clone(), installer:r.installer.clone() })
        && same_space(&r.before,&r.environment)
        && r.before.revision.checked_add(1) == Some(r.environment.revision)
        && r.environment.root == m.root.join("environments").join(&r.environment.id)
        && valid_product_environment(&r.environment.id), "installer_transaction_binding")?;
    Ok(r)
}

pub(super) fn records(m: &Manager) -> Result<Vec<Binding>> {
    let root = m.root.join("operator");
    if !root.try_exists()? { return Ok(vec![]); }
    let mut out = vec![];
    for (n, entry) in fs::read_dir(root)?.enumerate() {
        require(n < 4096,"operator_installation_history_bound")?;
        let path = entry?.path();
        let Some(id) = path.file_name().and_then(|v|v.to_str()) else {continue};
        if valid_hex(id,32) && path.join("installation.json").try_exists()? {
            out.push(load(m,id)?);
        }
    }
    out.sort_by_key(|r|(r.created_at,r.environment.revision));
    Ok(out)
}

pub(super) fn result(m: &Manager, r: &Binding) -> Result<Value> {
    let v = optional(&job_dir(m,&r.operation)?.join("installation-result.json"))?;
    require(v["schema"] == 2 && v["operation"] == r.operation,
        "installer_result_identity")?;
    onboarding::validate_installer_transaction(&v,&r.operation)?;
    Ok(v)
}

pub(crate) fn all_retired(m: &Manager) -> Result<bool> {
    for r in records(m)? {
        if !onboarding::retired(&result(m,&r)?) {return Ok(false);}
    }
    Ok(true)
}

pub(super) fn target(m: &Manager, environment: &str) -> Result<Environment> {
    let db = m.registry()?;
    if let Some(r) = onboarding::retained_environment_record(m,environment)? {
        require(r.installation_operation.is_some()
            && onboarding::retired(&onboarding::result(m,&r)?),
            "installer_initial_retirement_required")?;
        require(db.classes.values().filter(|e|e.registration.environment.id == environment)
            .all(|e|same_space(&e.registration.environment,&r.environment)),
            "installer_environment_binding_changed")?;
        return current_environment(m,&r.environment);
    }
    let mut owners = db.classes.values().filter(|e|e.registration.environment.id == environment);
    let first = owners.next().ok_or("installer_existing_setup_required")?;
    let current = current_environment(m,&first.registration.environment)?;
    require(owners.all(|e|same_space(&e.registration.environment,&current)),
        "installer_environment_binding_changed")?;
    Ok(current)
}

pub(super) fn reserve(m: &Manager, environment: &str, installer: &str,
    operation: &str) -> Result<Binding> {
    reserve_with(m,environment,installer,operation,||vendor_retired(m))
}
fn reserve_with(m: &Manager, environment: &str, installer: &str,
    operation: &str, vendor_retirement: impl FnOnce()->Result<bool>) -> Result<Binding> {
    let _ownership = m.lock("onboarding.lock")?;
    let _registry = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(onboarding::all_retired(m)? && vendor_retirement()?, "operator_installer_active")?;
    let before = target(m,environment)?;
    before.runner.verify()?;
    installer_import::load(m,installer)?;
    let mut after = before.clone();
    after.revision = after.revision.checked_add(1).ok_or("environment_revision_exhausted")?;
    let r = Binding {schema:1,operation:operation.into(),installer:installer.into(),
        before,environment:after,created_at:std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?.as_micros().try_into()?};
    let dir = job_dir(m,operation)?;
    require(!dir.join("installation.json").try_exists()?, "installer_transaction_already_used")?;
    atomic_json(&dir.join("installation-result.json"),
        &json!({"schema":2,"operation":operation,"state":"reserved",
            "cleanup_confirmed":false,"owned_live":null}))?;
    atomic_json(&dir.join("installation.json"),&r)?;
    // Invalidate old registration/inspection authority before a vendor can mutate
    // shared state. No old publication or vendor data is rewritten or deleted.
    atomic_json(&r.environment.root.join("environment.json"),&r.environment)?;
    Ok(r)
}

pub(super) fn launch(m: &Manager, r: &Binding) -> Result<()> {
    require(target(m,&r.environment.id)? == r.environment,
        "installer_environment_binding_changed")?;
    let sw = software(m)?;
    let installer = installer_import::load(m,&r.installer)?;
    let dir = job_dir(m,&r.operation)?;
    let report = dir.join("installation-result.json");
    let spec = dir.join("installer-spec.json");
    atomic_json(&spec,&json!({"schema":2,"operation":r.operation,
        "environment":r.environment,"installer":installer.artifact,"format":installer.format,
        "installer_launch":sw.installer_launch,"report":report}))?;
    onboarding::launch_spec(m,&r.operation,&spec,&report)
}

pub(super) fn control(m: &Manager, operation: &str) -> Result<Binding> {
    // Resource custody survives missing installer bytes, runtime changes and stale publications.
    load(m,operation)
}

pub(super) fn focus(m: &Manager, operation: &str) -> Result<Value> {
    let _ownership = m.lock("onboarding.lock")?;
    let r = control(m,operation)?;
    require(onboarding::live(operation)?,"installer_focus_owner")?;
    let request = random_id()?;
    atomic_json(&job_dir(m,operation)?.join(format!("{operation}-focus.json")),
        &json!({"operation":operation,"request":request}))?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let v = result(m,&r)?;
        if v["focus_result"]["request"] == request {
            require(v["focus_result"]["result"] == "focused","installer_focus_refused")?;
            return Ok(v["focus_result"].clone());
        }
        require(Instant::now() < deadline,"installer_focus_timeout")?;
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub(super) fn stop(m: &Manager, operation: &str) -> Result<Value> {
    let _ownership = m.lock("onboarding.lock")?;
    let r = control(m,operation)?;
    require(Command::new("systemctl").args(["--user","stop",&onboarding::unit(operation)?])
        .status()?.success(),"installer_stop_failed")?;
    let v = result(m,&r)?;
    require(!onboarding::live(operation)? && onboarding::retired(&v),"installer_cleanup_unconfirmed")?;
    Ok(v)
}

pub(super) fn finish(m: &Manager, operation: &str) -> Result<()> {
    finish_with(m,operation,onboarding::live)
}
fn finish_with(m: &Manager, operation: &str, is_live: impl FnOnce(&str)->Result<bool>) -> Result<()> {
    if !job_dir(m,operation)?.join("installation.json").try_exists()? {return Ok(());}
    let r = control(m,operation)?;
    if is_live(operation)? {let _ = stop(m,operation);}
    let mut v = result(m,&r)?;
    if !onboarding::retired(&v) {
        // If a worker died before launch, the current exact unit proves no vendor
        // process was started. Later missing retirement remains an ownership gap.
        if v["state"] == "reserved" && !job_dir(m,operation)?.join("installer-spec.json").try_exists()? {
            v["state"] = "failed".into();v["cleanup_confirmed"] = true.into();v["owned_live"] = 0.into();
            v["error"] = "installer_not_launched_rescan_required".into();
        } else {
            v["state"] = "cleanup_unconfirmed".into();
            v["error"] = "installer_supervisor_terminated_without_retirement".into();
        }
        atomic_json(&job_dir(m,operation)?.join("installation-result.json"),&v)?;
    }
    Ok(())
}

pub(super) fn projection(m: &Manager, products: &[ui::Product], busy: Option<&str>,
    mut is_live: impl FnMut(&str)->Result<bool>) -> Result<Vec<ui::EnvironmentInstaller>> {
    let db = m.registry()?;
    let history = records(m)?;
    let installers = installer_import::list_identity_records(m)?;
    let mut targets = std::collections::BTreeMap::new();
    let mut setup_names=std::collections::BTreeMap::new();
    for r in onboarding::history_records(m)? {
        if r.installation_operation.is_some()
            && onboarding::retired(&onboarding::result(m,&r)?) {
            if let Some(installer)=installers.iter().find(|i|i.id==r.installer) {
                setup_names.insert(r.environment.id.clone(),installer_import::presentation(m,installer)?.display_label);
            }
            targets.insert(r.environment.id.clone(),r.environment);
        }
    }
    for entry in db.classes.values() {
        let env = current_environment(m,&entry.registration.environment)?;
        targets.insert(env.id.clone(),env);
    }
    let mut out = vec![];
    for (id,env) in targets {
        let affected:Vec<_> = products.iter().filter(|p|p.environment == id)
            .map(|p|p.name.clone()).collect();
        let name = if affected.is_empty() {format!("{} · existing setup {}",setup_names.get(&id)
                .map(String::as_str).unwrap_or("Plug-in"),id.chars().take(8).collect::<String>())}
            else {affected.join(", ")};
        // Recovery follows unresolved custody, never wall-clock ordering.
        let mut unresolved=history.iter().filter(|r|r.environment.id == id)
            .filter_map(|r|match result(m,r) {
                Ok(value) if !onboarding::retired(&value)=>Some(Ok(r)),
                Ok(_)=>None,Err(error)=>Some(Err(error)),
            });
        let active=unresolved.next().transpose()?;
        require(unresolved.next().is_none(),"installer_environment_ownership_conflict")?;
        let last = active.or_else(||history.iter().rev().find(|r|r.environment.id == id));
        let result = last.map(|r|result(m,r)).transpose()?.unwrap_or(Value::Null);
        let mut actions = vec![];
        let runtime_unavailable=env.runner.validate_record().err();
        let new_work_reason=busy.or(runtime_unavailable.as_ref().map(|_|
            "This setup's pinned compatibility runtime is unavailable. Restore its original files before starting new work."));
        let mut reason = new_work_reason.map(str::to_owned);
        if let Some(r) = last {
            if !onboarding::retired(&result) {
                reason = Some("Finish or stop this installer; its cleanup must be confirmed".into());
                if is_live(&r.operation)? {
                    actions.push(action("Focus installer",ui::Action::EnvironmentInstallerFocus {
                        operation:r.operation.clone()},None));
                    actions.push(action("Stop installer",ui::Action::EnvironmentInstallerStop {
                        operation:r.operation.clone()},None));
                }
            } else if onboarding::inventory_refresh_record_required(m,&env,
                &software_record(m)?.host,&software_record(m)?.source_sha256)? {
                actions.push(action("Rescan and continue preparation",ui::Action::EnvironmentInstallerScan {
                    environment:id.clone()},new_work_reason));
            }
            if onboarding::retired(&result) && result["state"] != "completed" {
                let unavailable=installer_import::load_record(m,&r.installer).err();
                actions.push(action("Retry this installer in the same setup",ui::Action::EnvironmentInstallerStart {
                    environment:id.clone(),installer:r.installer.clone()},new_work_reason.or(unavailable.as_ref()
                    .map(|_|"Imported installer file is unavailable. Import the original file again."))));
            }
        }
        let choices = installers.iter().map(|i| {
            let label = installer_import::presentation(m,i)?.display_label;
            let missing=installer_import::load_record(m,&i.id).err();
            Ok(action(&format!("Run {label} in this setup"),ui::Action::EnvironmentInstallerStart {
                environment:id.clone(),installer:i.id.clone()},reason.as_deref().or(missing.as_ref()
                    .map(|_|"Imported installer file is unavailable. Import the original file again."))))
        }).collect::<Result<Vec<_>>>()?;
        out.push(ui::EnvironmentInstaller {environment:id,name,affected,
            consequence:"This installer can change every plug-in in this compatibility space, vendor applications, authorization and shared dependencies. Stopping ends owned processes; it cannot undo vendor changes. Existing plug-ins need a fresh scan and preparation before use. The environment identity is preserved; vendor state is not copied or rolled back.".into(),
            choices,operation:last.map(|r|r.operation.clone()),installer:last.map(|r|r.installer.clone()),result,actions});
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (test_fixture::Fixture,installer_import::Installer) {
        let (f,_,_,_) = test_fixture::prepared();
        let mut bytes=vec![0;1024];
        bytes[..2].copy_from_slice(b"MZ");bytes[60]=128;
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148]=2;bytes[150]=2;bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let path=f.outer.join("companion.exe");fs::write(&path,bytes).unwrap();
        let installer=installer_import::import(&f.m,file(&path).unwrap()).unwrap();
        let a=f.r.host.clone();
        let sw=Software {manager:a.clone(),supervisor:a.clone(),ownership:a.clone(),host:a,
            source_manifest:Artifact {path:f.r.host.path.with_file_name("host-source-manifest.json"),
                sha256:f.r.host_source_sha256.clone()},source_sha256:f.r.host_source_sha256.clone(),
            operator_frontend:None,native_catalogue:None,preparation_kit:None,installer_launch:None};
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();
        (f,installer)
    }
    fn job(m:&Manager,environment:&str,installer:&str)->String {
        let op=random_id().unwrap();let dir=job_dir(m,&op).unwrap();private_dir(&dir).unwrap();
        atomic_json(&dir.join("request.json"),&ui::Request {schema:ui::OPERATOR_SCHEMA,
            state_token:"fixture".into(),action:ui::Action::EnvironmentInstallerStart {
                environment:environment.into(),installer:installer.into()}}).unwrap();op
    }
    fn complete(m:&Manager,r:&Binding,state:&str) {
        atomic_json(&job_dir(m,&r.operation).unwrap().join("installation-result.json"),
            &json!({"schema":2,"operation":r.operation,"state":state,
                "cleanup_confirmed":true,"owned_live":0})).unwrap();
    }
    #[test]
    fn registered_companion_preserves_identity_and_invalidates_all_siblings() {
        let (f,installer)=fixture();let env=&f.r.environment;
        let mut sibling=f.r.clone();sibling.metadata.class_id="02".repeat(16);
        sibling.metadata.name="Sibling".into();f.m.register(sibling.clone()).unwrap();
        let original=fs::read(f.m.root.join("registry.json")).unwrap();
        let op=job(&f.m,&env.id,&installer.id);
        let r=reserve_with(&f.m,&env.id,&installer.id,&op,||Ok(true)).unwrap();
        assert_eq!(r.environment.id,env.id);assert_eq!(r.environment.root,env.root);
        assert_eq!(r.environment.runner,env.runner);assert_eq!(r.environment.revision,2);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(),original);
        assert!(f.r.validate_record(&f.m.root).is_err());
        assert!(sibling.validate_record(&f.m.root).is_err());
        assert!(!onboarding::all_retired(&f.m).unwrap());
        assert!(onboarding::load(&f.m,&env.id).is_err());
        assert_eq!(target(&f.m,&env.id).unwrap(),r.environment);
        let catalogue=catalogue::Catalogue {schema:1,natives:vec![],hosts:vec![],onboarding_runtime:None,
            environments:vec![catalogue::EnvironmentBinding {family:profiles::Family::ManagedInstallerV1,
                environment:env.clone()}]};
        let bindings=managed_environment_bindings(&f.m,Some(&catalogue),&f.m.registry().unwrap()).unwrap();
        assert_eq!(bindings[0].environment,r.environment);
        let rows=environment_projection_from(&f.m,&software_record(&f.m).unwrap(),&bindings,
            &f.m.registry().unwrap(),Some("installer active")).unwrap();
        assert_eq!(rows[0].revision,2);
        let second=job(&f.m,&env.id,&installer.id);
        assert!(reserve_with(&f.m,&env.id,&installer.id,&second,||Ok(true)).is_err());
        complete(&f.m,&r,"cancelled");
        assert!(onboarding::all_retired(&f.m).unwrap());
        let next=reserve_with(&f.m,&env.id,&installer.id,&second,||Ok(true)).unwrap();
        assert_eq!(next.environment.revision,3);
        assert_eq!(result(&f.m,&r).unwrap()["state"],"cancelled");
        let mut earlier=r.clone();earlier.created_at=u64::MAX;
        atomic_json(&job_dir(&f.m,&r.operation).unwrap().join("installation.json"),&earlier).unwrap();
        let rows=projection(&f.m,&[],None,|op|Ok(op==second)).unwrap();
        assert!(rows[0].actions.iter().any(|offer|matches!(&offer.action,
            ui::Action::EnvironmentInstallerStop {operation} if operation==&second)));
    }
    #[test]
    fn unregistered_setup_resolves_revision_without_rewriting_initial_history() {
        let (f,installer)=fixture();let mut env=f.r.environment.clone();
        env.id="cd".repeat(16);env.root=f.m.root.join("environments").join(&env.id);
        private_dir(&env.root).unwrap();atomic_json(&env.root.join("environment.json"),&env).unwrap();
        let initial="ab".repeat(16);let dir=onboarding::directory(&f.m,&env.id).unwrap();
        private_dir(&dir).unwrap();
        atomic_json(&dir.join("record.json"),&onboarding::Record {schema:1,id:env.id.clone(),
            installer:installer.id.clone(),environment:env.clone(),created_at:1,creation_operation:"12".repeat(16),
            installation_operation:Some(initial.clone()),published:false,previous_attempt:None}).unwrap();
        atomic_json(&dir.join(format!("{initial}-result.json")),&json!({"schema":2,
            "operation":initial,"state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
        let retained=fs::read(dir.join("record.json")).unwrap();
        let op=job(&f.m,&env.id,&installer.id);
        let r=reserve_with(&f.m,&env.id,&installer.id,&op,||Ok(true)).unwrap();
        assert_eq!(onboarding::load(&f.m,&env.id).unwrap().environment,r.environment);
        assert_eq!(fs::read(dir.join("record.json")).unwrap(),retained);
        // Crash before launch is recoverable without inventing vendor success.
        finish_with(&f.m,&op,|_|Ok(false)).unwrap();
        let v=result(&f.m,&r).unwrap();assert_eq!(v["state"],"failed");
        assert!(onboarding::retired(&v));assert!(onboarding::all_retired(&f.m).unwrap());
        let rows=projection(&f.m,&[],None,|_|Ok(false)).unwrap();
        let row=rows.iter().find(|row|row.environment == env.id).unwrap();
        assert!(row.actions.iter().any(|a|matches!(a.action,ui::Action::EnvironmentInstallerScan { .. })));
        assert!(row.actions.iter().any(|a|matches!(a.action,ui::Action::EnvironmentInstallerStart { .. })));
    }
    #[test]
    fn exact_control_survives_missing_inputs_and_malformed_binding_refuses() {
        let (f,installer)=fixture();let op=job(&f.m,&f.r.environment.id,&installer.id);
        let r=reserve_with(&f.m,&f.r.environment.id,&installer.id,&op,||Ok(true)).unwrap();
        fs::remove_file(&installer.artifact.path).unwrap();
        fs::remove_file(&r.environment.runner.proton).unwrap();
        assert!(control(&f.m,&op).is_ok());assert!(launch(&f.m,&r).is_err());
        let mut changed=r.clone();changed.environment.id="ff".repeat(16);
        atomic_json(&job_dir(&f.m,&op).unwrap().join("installation.json"),&changed).unwrap();
        assert!(control(&f.m,&op).is_err());assert!(all_retired(&f.m).is_err());
    }
    #[test]
    fn ordinary_current_offer_keeps_stop_when_installer_runtime_and_module_disappear() {
        use linux_vst_bridge::{preparation as prep,runtime_delivery};
        let (f,base)=preparation_cli::tests::projection_fixture();
        atomic_json(&f.m.root.join("software.json"),
            &preparation_cli::tests::projection_software(&base)).unwrap();
        prep::record_candidate(&f.m,&base).unwrap();prep::enable(&f.m,&base,false).unwrap();
        let mut bytes=vec![0;1024];bytes[..2].copy_from_slice(b"MZ");bytes[60]=128;
        bytes[128..132].copy_from_slice(b"PE\0\0");bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[148]=2;bytes[150]=2;bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let source=f.outer.join("companion.exe");fs::write(&source,bytes).unwrap();
        let installer=installer_import::import(&f.m,file(&source).unwrap()).unwrap();
        let op=job(&f.m,&base.selection.environment.id,&installer.id);
        let r=reserve_with(&f.m,&base.selection.environment.id,&installer.id,&op,||Ok(true)).unwrap();
        let runtime_path=f.m.root.join("runners").join(runtime_delivery::ID).join("runtime.json");
        let runtime_dir=runtime_path.parent().unwrap();
        let artifact=|relative:&str| {
            let path=runtime_dir.join(relative);private_dir(path.parent().unwrap()).unwrap();
            fs::write(&path,b"runtime identity").unwrap();
            Artifact {sha256:digest(&path).unwrap(),path}
        };
        let proton=artifact("GE-Proton11-7-x86_64/proton");
        let entry=artifact("SteamLinuxRuntime_4/_v2-entry-point");
        let tree=artifact("runtime-tree.json");
        let runner=Runner {id:runtime_delivery::ID.into(),version:"retained-test".into(),
            proton:proton.path.clone(),entry_point:entry.path.clone(),files:vec![proton,entry,tree],policy:None};
        atomic_json(&runtime_path,&json!({"schema":1,"id":runtime_delivery::ID,
            "downloads":runtime_delivery::downloads(),"runner":runner})).unwrap();
        fs::set_permissions(&runtime_path,fs::Permissions::from_mode(0o400)).unwrap();
        fs::remove_file(&installer.artifact.path).unwrap();
        fs::remove_file(&runner.proton).unwrap();
        fs::remove_file(&r.environment.runner.proton).unwrap();
        fs::remove_file(&base.selection.module.path).unwrap();
        let captured=current::capture_with_installer_live(&f.m,|owned|Ok(owned==op)).unwrap();
        let stop=available(&captured.snapshot).into_iter().find(|a|matches!(&a.action,
            ui::Action::EnvironmentInstallerStop {operation} if operation==&op)).unwrap();
        assert!(stop.disabled_reason.is_none());
        assert!(captured.snapshot.environment_installers[0].choices.iter().all(|a|a.disabled_reason.is_some()));
        let request=ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:captured.snapshot.state_token.clone(),
            action:stop.action.clone()};
        validate_installer_control_with(&f.m,&request,|owned|Ok(owned==op)).unwrap();
        assert!(launch(&f.m,&r).is_err());
        assert_eq!(captured.snapshot.products[0].disposition,"needs_attention");
    }
    #[test]
    fn unfinished_supervisor_retains_cleanup_gap_and_global_exclusion() {
        let (f,installer)=fixture();let op=job(&f.m,&f.r.environment.id,&installer.id);
        let r=reserve_with(&f.m,&f.r.environment.id,&installer.id,&op,||Ok(true)).unwrap();
        atomic_json(&job_dir(&f.m,&op).unwrap().join("installer-spec.json"),&json!({"operation":op})).unwrap();
        finish_with(&f.m,&op,|_|Ok(false)).unwrap();
        assert_eq!(result(&f.m,&r).unwrap()["state"],"cleanup_unconfirmed");
        assert!(!onboarding::all_retired(&f.m).unwrap());
        let rows=projection(&f.m,&[],None,|_|Ok(false)).unwrap();
        assert!(rows[0].choices.iter().all(|a|a.disabled_reason.is_some()));
    }
}

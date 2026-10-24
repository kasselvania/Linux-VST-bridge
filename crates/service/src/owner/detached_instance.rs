//! Ordinary plug-in windows. These own one host and the ordinary setup lease;
//! they never reserve setup maintenance or become evidence of audible playback.
use super::*;
use crate::operator_model as ui;
use serde_json::{json, Value};

struct Runtime {
    manager: Arc<Manager>, software: Software, limits: capacity::Limits,
    keepers: Arc<Keepers>, blocked: Arc<AdmissionCleanup>, verification: Arc<LaunchVerification>,
}
static RUNTIME: std::sync::OnceLock<Runtime> = std::sync::OnceLock::new();
pub(super) fn initialize(manager: Arc<Manager>, software: Software, limits: capacity::Limits,
    keepers: Arc<Keepers>, blocked: Arc<AdmissionCleanup>, verification: Arc<LaunchVerification>) -> Result<()> {
    RUNTIME.set(Runtime {manager, software, limits, keepers, blocked, verification})
        .map_err(|_| "detached_instance_owner_already_initialized".into())
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Frontend {
    schema: u32, boot: String, uid: u32,
    graphical: transport_storage::GraphicalSession,
    manager: Artifact, frontend: Artifact,
}
thread_local! { static FRONTEND: std::cell::RefCell<Option<Frontend>> = const { std::cell::RefCell::new(None) }; }
pub(super) fn with_frontend<T>(m: &Manager, peer: &UnixStream, run: impl FnOnce() -> Result<T>) -> Result<T> {
    package_authority::require_subscribed_frontend(m, peer)?;
    let process = transport_storage::peer_process(peer)?;
    let graphical = transport_storage::graphical_session(peer, &process)?;
    let selected = software_record(m)?;
    let context = Frontend {schema:1, boot:kernel_boot()?.ok_or("detached_instance_requires_linux")?,
        uid:process.uid, graphical, manager:selected.manager,
        frontend:selected.operator_frontend.ok_or("package_frontend_absent")?};
    require(transport_storage::peer_process(peer)? == process, "detached_frontend_changed")?;
    struct Restore(Option<Frontend>);
    impl Drop for Restore { fn drop(&mut self) { FRONTEND.with(|context| *context.borrow_mut()=self.0.take()); } }
    let _restore=Restore(FRONTEND.with(|slot| slot.replace(Some(context))));
    run()
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Accepted { schema:u32, request:Artifact, frontend:Frontend }
pub(super) fn retain_context(m: &Manager, operation: &str, action: &ui::Action) -> Result<()> {
    if !matches!(action, ui::Action::PluginOpen {..}) { return Ok(()); }
    let frontend=FRONTEND.with(|slot| slot.borrow().clone()).ok_or("detached_frontend_context_missing")?;
    let directory=operator_cli::job_dir(m,operation)?;
    let request=artifact(directory.join("request.json"))?;
    atomic_json(&directory.join("detached-admission.json"), &Accepted {schema:1, request, frontend})
}
fn artifact(path: PathBuf) -> Result<Artifact> { Ok(Artifact {sha256:digest(&path)?, path}) }
fn context(m: &Manager, operation: &str, action: &ui::Action) -> Result<Frontend> {
    let directory=operator_cli::job_dir(m,operation)?;
    let accepted:Accepted=read_json(&directory.join("detached-admission.json"))?;
    require(accepted.schema==1 && accepted.frontend.schema==1
        && accepted.request.path==directory.join("request.json"), "detached_admission_binding")?;
    let request:ui::Request=serde_json::from_slice(&accepted.request.record_bytes(service::contract::REQUEST_LIMIT)?)?;
    require(request.schema==ui::OPERATOR_SCHEMA && request.action==*action, "detached_request_changed")?;
    recheck_context(m,&accepted.frontend)?;
    Ok(accepted.frontend)
}
fn recheck_context(m:&Manager, context:&Frontend)->Result<()> {
    let selected=software_record(m)?;
    require(selected.manager==context.manager && selected.operator_frontend.as_ref()==Some(&context.frontend),
        service::contract::FRONTEND_SUPERSEDED)?;
    let consumer=ui::ConsumerIdentity {boot_id:context.boot.clone(), uid:context.uid,
        pid:context.graphical.peer_pid, start_ticks:context.graphical.peer_start_ticks};
    require(transport_storage::consumer_current(&consumer)?, "detached_frontend_process_changed")
}
fn identity(product:&ui::ProductIdentity)->Result<Vec<u8>> {
    require(valid_hex(&product.class_id,32) && product.class_id==product.class_id.to_uppercase()
        && valid_hex(&product.module_sha256,64), "detached_product_identity")?;
    let value=format!("{}{}",product.class_id,product.module_sha256);
    (0..value.len()).step_by(2).map(|i|u8::from_str_radix(&value[i..i+2],16).map_err(Into::into)).collect()
}
fn selected(m:&Manager, product:&ui::ProductIdentity, publication:&ui::PublicationIdentity)->Result<Registration> {
    let registry=m.registry()?;
    let entry=registry.classes.get(&product.class_id).ok_or("detached_product_not_published")?;
    require(entry.publication==Publication::Published && entry.registration.environment.id==product.environment
        && entry.registration.module.sha256==product.module_sha256
        && entry.managed_revision.as_ref().is_some_and(|current|current.id==publication.id && current.sha256==publication.sha256)
        && !m.publication_pending(&product.class_id)?, "detached_publication_changed")?;
    Ok(entry.registration.clone())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    schema:u32, operation:String, product:ui::ProductIdentity, publication:ui::PublicationIdentity,
    session:String, owner:Artifact,
}
fn window_path(m:&Manager, operation:&str)->Result<PathBuf> {
    Ok(operator_cli::job_dir(m,operation)?.join("detached-window.json"))
}
fn live(m:&Manager, operation:&str, session:&str)->Result<(Window, SessionSpec)> {
    require(valid_hex(session,32), "detached_session_identity")?;
    let window:Window=read_json(&window_path(m,operation)?)?;
    require(window.schema==1 && window.operation==operation && window.session==session,
        "detached_window_identity")?;
    let lease=m.root.join("runtime/leases").join(format!("{session}.json"));
    let report:PathBuf=read_json(&lease)?;
    let job:SessionSpec=serde_json::from_slice(&window.owner.record_bytes(1024*1024)?)?;
    require(job.detached && !job.inspect && !job.keeper && !job.vendor_access && !job.finish_setup
        && job.shared_runtime && job.session==session && job.lease==lease && job.report==report
        && job.report==m.root.join("runtime/results").join(format!("windows-{session}.json"))
        && job.registration.environment.id==window.product.environment
        && job.registration.metadata.class_id==window.product.class_id
        && job.registration.module.sha256==window.product.module_sha256
        && job.directory==job.registration.environment.root.join("compatdata/pfx/drive_c/bridge/sessions").join(session)
        && window.owner.path==job.directory.join("owner.json"), "detached_window_binding")?;
    Ok((window,job))
}
pub(super) fn validate_control(m:&Manager, action:&ui::Action)->Result<()> {
    match action {
        ui::Action::PluginFocus {operation,session}|ui::Action::PluginClose {operation,session} => live(m,operation,session).map(|_|()),
        _=>Err("detached_control_action".into()),
    }
}
fn offered(label:&str, action:ui::Action, disabled_reason:Option<String>)->ui::AvailableAction {
    ui::AvailableAction {label:label.into(),action,disabled_reason}
}
pub(super) fn project(m:&Manager, products:&mut [ui::Product])->Result<()> {
    let registry=m.composed_registry()?;
    let directory=m.root.join("operator");
    let mut windows=vec![];
    if directory.try_exists()? {
        for (index,entry) in fs::read_dir(directory)?.enumerate() {
            require(index<2048,"operator_inventory_bound")?;
            let entry=entry?;
            let Some(operation)=entry.file_name().to_str().filter(|v|valid_hex(v,32)).map(str::to_owned) else {continue};
            let path=entry.path().join("detached-window.json");
            if !path.try_exists()? {continue;}
            let window:Window=read_json(&path)?;
            if m.root.join("runtime/leases").join(format!("{}.json",window.session)).try_exists()? {
                // An unresolved exact record cannot authorize a guessed window.
                if let Ok((window,_))=live(m,&operation,&window.session) { windows.push(window); }
            }
        }
    }
    for product in products {
        let key=ui::ProductIdentity {environment:product.environment.clone(),
            module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone()};
        if let Some(entry)=registry.classes.get(&key.class_id).filter(|entry|
            entry.publication==Publication::Published && entry.registration.environment.id==key.environment
                && entry.registration.module.sha256==key.module_sha256) {
            if let Some(publication)=&entry.managed_revision {
                let reason=if operator_cli::maintenance::blocks(m,&entry.registration.environment)?
                    || operator_cli::vendor_maintenance::blocks(m,&entry.registration.environment)? {
                    Some("This setup is being maintained. Open the plug-in when that work has finished.".into())
                } else {None};
                product.actions.push(offered("Open on its own",ui::Action::PluginOpen {product:key.clone(),
                    expected_current:ui::PublicationIdentity {id:publication.id.clone(),sha256:publication.sha256.clone()}},reason));
            }
        }
        for window in windows.iter().filter(|window|window.product==key) {
            product.actions.push(offered("Show the plug-in window",ui::Action::PluginFocus {
                operation:window.operation.clone(),session:window.session.clone()},None));
            product.actions.push(offered("Close this plug-in window",ui::Action::PluginClose {
                operation:window.operation.clone(),session:window.session.clone()},None));
        }
    }
    Ok(())
}
pub(super) fn execute(m:&Manager, action:&ui::Action, operation:&str)->Result<Value> {
    match action {
        ui::Action::PluginOpen {product,expected_current}=>open(m,action,operation,product,expected_current),
        ui::Action::PluginFocus {operation,session}=>focus(m,operation,session),
        ui::Action::PluginClose {operation,session}=>close(m,operation,session),
        _=>Err("detached_instance_action".into()),
    }
}
fn open(m:&Manager, action:&ui::Action, operation:&str, product:&ui::ProductIdentity,
    publication:&ui::PublicationIdentity)->Result<Value> {
    let runtime=RUNTIME.get().filter(|runtime|runtime.manager.root==m.root)
        .ok_or("detached_instance_service_unavailable")?;
    let context=context(m,operation,action)?;
    let deadline=Instant::now()+Duration::from_secs(KEEPER_OWNER_STARTUP_SECONDS);
    let identity=identity(product)?;
    let ((registration,execution),prepared)=runtime.verification.prepare(deadline,|| {
        let registration=selected(m,product,publication)?;
        require(m.resolve(&identity)?==registration,"detached_publication_changed")?;
        m.verify_served_host(&registration,&runtime.software.host,&runtime.software.source_sha256,&profiles::installed_profiles()?)?;
        let execution=package_authority::paired_components(m,&runtime.software,&registration)?;
        Ok((registration,execution))
    })?;
    let binding:HostBinding=registration.clone().into();
    let reservation=prepared.run(||wait_for_keeper(deadline,
        ||capacity::reserve_detached(m,&runtime.limits,&binding.environment,
            ||native_admission_blocked(m,&runtime.blocked,&binding.environment)),
        || {
            recheck_context(m,&context)?;
            require(selected(m,product,publication)?==registration,"detached_publication_changed")?;
            stage_keeper(m,&execution,&binding,&runtime.keepers,Some(&context.graphical))
        },std::thread::sleep))?;
    let keeper=keeper_session(&runtime.keepers,&binding.environment.id)?.ok_or("ready_keeper_ownership_missing")?;
    drop(reservation);
    // The keeper can change runtime-copy identities while starting. Prepare
    // again outside the registry guard before exposing this exact host lease.
    let ((mut job,path,publication_target),verified)=runtime.verification.prepare(deadline,|| {
        require(m.resolve(&identity)?==registration && selected(m,product,publication)?==registration,
            "detached_publication_changed")?;
        require(package_authority::paired_components(m,&runtime.software,&registration)?==execution,
            "detached_execution_changed")?;
        let (mut job,path)=spec(m,binding.clone(),false,false,false)?;
        job.detached=true;job.shared_runtime=true;job.graphical_session=Some(context.graphical.clone());
        atomic_json(&path,&job)?;
        let registry=m.registry()?;
        let entry=registry.classes.get(&product.class_id).ok_or("detached_publication_changed")?;
        require(entry.registration==registration,"detached_publication_changed")?;
        Ok((job,path,m.entry_target(entry)?))
    })?;
    verified.recheck()?;
    let reservation=capacity::reserve_detached(m,&runtime.limits,&binding.environment,
        ||native_admission_blocked(m,&runtime.blocked,&binding.environment))?;
    recheck_context(m,&context)?;
    require(selected(m,product,publication)?==registration
        && keeper_session(&runtime.keepers,&binding.environment.id)?.as_deref()==Some(keeper.as_str()),
        "detached_admission_changed")?;
    {
        let mut active=runtime.keepers.lock().map_err(|_|"environment ownership lock poisoned")?;
        require(observe_keeper(&binding.environment.id,&mut active)?==Some(KeeperAvailability::Ready),
            "detached_keeper_no_longer_ready")?;
    }
    require(fs::read_link(m.link(&product.class_id))?==publication_target,"detached_publication_changed")?;
    verified.recheck_artifacts([&execution.supervisor,&execution.ownership,&execution.host,
        &binding.module,&registration.native])?;
    let mut pending=PendingAdmission::new(job.lease.clone(),runtime.blocked.clone());
    let child=verified.run_prepared(||spawn(m,&execution,&path,None))?;
    // A started supervisor owns this exact process tree even before readiness.
    pending.expose();
    let window=Window {schema:1,operation:operation.into(),product:product.clone(),publication:publication.clone(),
        session:job.session.clone(),owner:artifact(path)?};
    let retained=atomic_json(&window_path(m,operation)?,&window);
    drop(reservation);
    let session=job.session.clone();let report=job.report.clone();let directory=job.directory.clone();
    let monitor_manager=Manager::new(m.root.clone(),m.publications.clone());
    std::thread::Builder::new().name("bridge-plugin-window".into()).spawn(move|| {
        if let Err(error)=monitor(&monitor_manager,&mut job,child,pending) {
            eprintln!("Plug-in window {session} retirement remains unconfirmed: {error}");
        }
    })?;
    retained?;
    let deadline=Instant::now()+Duration::from_secs(185);
    loop {
        if report.try_exists()? {
            let result:Value=read_json(&report)?;
            return Err(result["error"].as_str().unwrap_or("The plug-in window closed before opening was confirmed").to_owned().into());
        }
        let observed=report.with_extension("window.json");
        if observed.try_exists()? {
            let value:Value=read_json(&observed)?;
            if value["session"]==window.session && value["detached_open"]==true {
                return Ok(json!({"detached_instance":true,"session":window.session,"product":product,"opened":true}));
            }
        }
        if Instant::now()>=deadline {
            // Ask only this already-owned window to close; retain uncertainty.
            write_close(&directory,&window.session)?;
            return Err("The plug-in window did not confirm opening within its startup limit; its owner is closing it".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
fn monitor(m:&Manager, job:&mut SessionSpec, mut child:Child, mut pending:PendingAdmission)->Result<()> {
    if let Err(error)=supervisor_ready(&mut child,job,Duration::from_secs(4)) {
        // SIGTERM is cooperative; never kill the process tree or erase custody
        // because an acknowledgement was late. The exact lease remains held.
        if child.try_wait()?.is_none() { unsafe {libc::kill(child.id() as i32,libc::SIGTERM);} }
        eprintln!("Plug-in window readiness was not confirmed: {error}");
    }
    let status=loop {
        if let Some(status)=child.try_wait()? {break status;}
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut receipt=String::new();
    if let Some(stdout)=child.stdout.take() {stdout.take(128).read_to_string(&mut receipt)?;}
    let (_registry,_)=m.lock_bounded(ui::OperatorLock::Registry,ui::LockPurpose::ServiceRecovery,None,Duration::from_secs(2))?;
    pending.complete(&job.session,status.success(),&receipt,Some(&job.directory))
}
fn write_close(directory:&Path,session:&str)->Result<()> {
    let path=directory.join("instance.close");
    match fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&path) {
        Ok(mut file)=>{file.write_all(format!("{session}\n").as_bytes())?;file.sync_all()?;},
        Err(error) if error.kind()==std::io::ErrorKind::AlreadyExists=>{
            require(fs::read_to_string(&path)?==format!("{session}\n"),"detached_close_binding")?;
        },
        Err(error)=>return Err(error.into()),
    }
    Ok(())
}
fn close(m:&Manager,operation:&str,session:&str)->Result<Value> {
    let (_,job)=live(m,operation,session)?;
    write_close(&job.directory,session)?;
    let deadline=Instant::now()+Duration::from_secs(40);
    while job.lease.try_exists()? {
        require(Instant::now()<deadline,"The close request was sent; this plug-in's retirement is still unconfirmed")?;
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(json!({"session":session,"closed":true}))
}
fn focus(m:&Manager,operation:&str,session:&str)->Result<Value> {
    let (_,job)=live(m,operation,session)?;
    let request=random_id()?;
    atomic_json(&job.report.with_extension("focus.json"),&json!({"request":request}))?;
    let deadline=Instant::now()+Duration::from_secs(4);
    loop {
        let path=job.report.with_extension("focus-result.json");
        if path.try_exists()? {
            let result:Value=read_json(&path)?;
            if result["request"]==request {
                require(result["result"]=="focused","The plug-in's exact window could not be raised")?;
                return Ok(json!({"session":session,"focused":true}));
            }
        }
        require(Instant::now()<deadline,"The plug-in window did not acknowledge focus")?;
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::{Fixture, snapshot};
    fn retain(f:&Fixture,operation:&str)->(Window,SessionSpec) {
        let (mut job,path)=spec(&f.m,f.r.clone().into(),false,false,false).unwrap();
        job.detached=true;job.shared_runtime=true;
        atomic_json(&path,&job).unwrap();
        let window=Window {schema:1,operation:operation.into(),session:job.session.clone(),owner:artifact(path.clone()).unwrap(),
            product:ui::ProductIdentity {environment:f.r.environment.id.clone(),module_sha256:f.r.module.sha256.clone(),
                class_id:f.r.metadata.class_id.clone()},
            publication:ui::PublicationIdentity {id:"12".repeat(16),sha256:"34".repeat(32)}};
        private_dir(&operator_cli::job_dir(&f.m,operation).unwrap()).unwrap();
        atomic_json(&window_path(&f.m,operation).unwrap(),&window).unwrap();
        f.m.retain_lease_owner(&path).unwrap();atomic_json(&job.lease,&job.report).unwrap();
        (window,job)
    }
    #[test]
    fn controls_keep_exact_original_window_after_publication_changes_and_refuse_substitution() {
        let f=Fixture::new();let operation="ab".repeat(16);
        let (window,job)=retain(&f,&operation);
        let focus=ui::Action::PluginFocus {operation:operation.clone(),session:job.session.clone()};
        validate_control(&f.m,&focus).unwrap();
        // Controls do not resolve today's class/publication or select a sibling.
        f.m.register(f.r.clone()).unwrap();
        validate_control(&f.m,&focus).unwrap();
        let (sibling,other)=retain(&f,&"cd".repeat(16));
        assert!(validate_control(&f.m,&ui::Action::PluginClose {operation:operation.clone(),session:sibling.session}).is_err());
        let before=snapshot(&f.m.root);
        fs::write(&window.owner.path,b"{}").unwrap();
        assert!(validate_control(&f.m,&focus).is_err());
        assert!(other.lease.exists() && !other.directory.join("instance.close").exists());
        // The only changed bytes are the explicit adversarial input.
        fs::write(&window.owner.path,&before[&window.owner.path]).unwrap();
        assert!(job.lease.exists());
        assert_eq!(snapshot(&f.m.root),before);
        validate_control(&f.m,&focus).unwrap();
    }
    #[test]
    fn selected_publication_and_execution_bytes_are_separate_exact_boundaries() {
        let (f,candidate)=crate::preparation::tests::fixture();
        crate::preparation::record_candidate(&f.m,&candidate).unwrap();
        let revision=crate::preparation::enable(&f.m,&candidate,false).unwrap();
        let product=ui::ProductIdentity {environment:candidate.selection.environment.id.clone(),
            module_sha256:candidate.selection.module.sha256.clone(),class_id:candidate.selection.class.id.clone()};
        let expected=ui::PublicationIdentity {id:revision.id.clone(),sha256:revision.sha256.clone()};
        let registration=selected(&f.m,&product,&expected).unwrap();
        assert_eq!(f.m.resolve(&identity(&product).unwrap()).unwrap(),registration);
        let mut wrong=expected.clone();wrong.sha256="ef".repeat(32);
        assert!(selected(&f.m,&product,&wrong).is_err());
        let runtime=&registration.environment.runner.files[0];
        let bytes=fs::read(&runtime.path).unwrap();fs::write(&runtime.path,b"changed runtime").unwrap();
        let before=snapshot(&f.m.root);
        assert_eq!(selected(&f.m,&product,&expected).unwrap(),registration,"passive exact binding does not hash execution assets");
        assert!(f.m.resolve(&identity(&product).unwrap()).is_err(),"actual launch preparation refuses changed bytes");
        assert_eq!(snapshot(&f.m.root),before);
        fs::write(&runtime.path,bytes).unwrap();
    }
    #[cfg(target_os="linux")]
    #[test]
    fn exact_supervisor_receipt_retires_only_this_window_and_wrong_receipt_preserves_custody() {
        for exact in [false,true] {
            let f=Fixture::new();let (window,mut job)=retain(&f,&"ab".repeat(16));
            let (_,sibling)=retain(&f,&"cd".repeat(16));
            let blocked=Arc::new(AdmissionCleanup::default());
            let mut pending=PendingAdmission::new(job.lease.clone(),blocked.clone());pending.expose();
            let child=Command::new("/usr/bin/python3").args(["-c",
                "import pathlib,shutil,sys; print('LVO0 '+sys.argv[1]+' ready',flush=True); shutil.rmtree(sys.argv[2]); print('LVO1 '+sys.argv[3]+' retired',flush=True)",
                &job.session,job.directory.to_str().unwrap(),if exact {&job.session} else {"00"}])
                .stdout(Stdio::piped()).spawn().unwrap();
            assert_eq!(monitor(&f.m,&mut job,child,pending).is_ok(),exact);
            assert_eq!(job.lease.exists(),!exact);
            assert_eq!(blocked.blocked(),!exact);
            assert!(sibling.lease.exists() && sibling.directory.exists());
            assert!(window_path(&f.m,&window.operation).unwrap().exists(),"history is preserved after retirement");
        }
    }
}

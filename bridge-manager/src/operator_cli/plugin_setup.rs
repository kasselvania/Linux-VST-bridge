//! Visible, bounded access to one exact installed module before publication.
//! The existing supervisor owns the Windows process tree and its retirement.
use super::*;

fn waiting(module:&inventory::Module)->bool {
    module.inspection_error.as_deref().is_some_and(|error|
        error.contains("Windows call deadline:") || error.starts_with("PluginWindowWaiting:"))
}
fn offered(scan:&inventory::Scan,index:usize,module:&inventory::Module)->ui::Action {
    ui::Action::PluginFinishSetup {environment:scan.environment.id.clone(),scan:scan.id.clone(),
        module_index:index,module_sha256:module.artifact.sha256.clone(),report_sha256:module.report.sha256.clone()}
}
fn target(action:&ui::Action)->Result<(&str,QuarantinedRetry)> {
    let ui::Action::PluginFinishSetup {environment,scan,module_index,module_sha256,report_sha256}=action
        else{return Err("plugin_setup_action".into())};
    Ok((environment,QuarantinedRetry {scan:scan.clone(),module_index:*module_index,
        module_sha256:module_sha256.clone(),report_sha256:report_sha256.clone()}))
}
fn running(m:&Manager,operation:&str)->Result<SessionSpec> {
    let dir=job_dir(m,operation)?;
    require(optional(&dir.join("result.json"))?["state"]=="vendor_running","plugin_setup_not_running")?;
    let request:ui::Request=read_json(&dir.join("request.json"))?;
    let (environment,retry)=target(&request.action)?;
    let job:SessionSpec=read_json(&dir.join("setup-session.json"))?;
    require(job.finish_setup && job.inspect && !job.keeper && !job.shared_runtime
        && job.registration.environment.id==environment && job.registration.module.sha256==retry.module_sha256
        && job.report==m.root.join("runtime/results").join(format!("windows-{}.json",job.session))
        && job.directory==job.registration.environment.root.join("compatdata/pfx/drive_c/bridge/sessions").join(&job.session),
        "plugin_setup_control_binding")?;
    Ok(job)
}
fn controls(operation:&str)->Vec<ui::AvailableAction> {
    vec![action("Show the plug-in window",ui::Action::PluginSetupFocus {operation:operation.into()},None),
        action("Stop finishing setup",ui::Action::PluginSetupStop {operation:operation.into()},None)]
}
pub(super) fn validate_control(m:&Manager,request:&ui::Request)->Result<()> {
    require(request.state_token==token(m)?,"operator_stale_request_refresh")?;
    let operation=match &request.action {
        ui::Action::PluginSetupFocus {operation}|ui::Action::PluginSetupStop {operation}=>operation,
        _=>return Err("plugin_setup_control_action".into()),
    };
    running(m,operation).map(|_|())
}
/// Applied after installation and busy projections so an active session keeps
/// its exact Focus/Stop controls while new work is blocked.
pub(super) fn project(m:&Manager,products:&mut [ui::Product],setups:&mut [ui::InstallerSetup],
    busy:Option<&str>)->Result<()> {
    let sw=software_record(m)?;let db=m.registry()?;
    let c=operator_catalogue_readback(m,&sw,&db)?;
    let bindings=managed_environment_bindings(m,c.as_ref(),&db)?;
    let live=resume_record(m)?.and_then(|owner|running(m,&owner.owner_operation).ok()
        .map(|job|(owner.owner_operation,job)));
    for product in products {
        if let Some((operation,job))=&live {
            if job.registration.environment.id==product.environment
                && job.registration.module.sha256==product.module_sha256 {
                product.actions=controls(operation);continue;
            }
        }
        let path=m.root.join("inventory").join(format!("{}.json",product.environment));
        if !path.try_exists()? {continue;}
        let scan:inventory::Scan=read_json(&path)?;
        let Some((index,module))=scan.modules.iter().enumerate().find(|(_,module)|
            module.artifact.sha256==product.module_sha256 && waiting(module)) else{continue};
        let environment=quarantined_retry_binding(m,&bindings,&db,&product.environment,false)?;
        if environment.is_none() {continue;}
        let stale=environment.as_ref().and_then(|environment|inventory::record_stale_reason(module,
            &scan.environment,&scan.host,&scan.host_source_sha256,environment,&sw.host,&sw.source_sha256));
        let reason=busy.or(stale).or(environment.is_none().then_some("Finish closing this setup's installer first"));
        product.actions.insert(0,action("Open this plug-in to finish setup",offered(&scan,index,module),reason));
    }
    for setup in setups {
        if let Some((operation,job))=&live {
            if setup.environment.as_deref()==Some(job.registration.environment.id.as_str()) {
                let mut controls=controls(operation);
                setup.primary=Some(controls.remove(0));setup.secondary=controls;
                setup.status="Finish setup in the plug-in's window, then close it. The bridge will check it again automatically.".into();
                continue;
            }
        }
        let Some(environment)=setup.environment.as_deref() else{continue};
        let path=m.root.join("inventory").join(format!("{environment}.json"));
        if !path.try_exists()? {continue;}
        let scan:inventory::Scan=read_json(&path)?;
        let Some((index,module))=scan.modules.iter().enumerate().find(|(_,module)|waiting(module)) else{continue};
        if quarantined_retry_binding(m,&bindings,&db,environment,false)?.as_ref()!=Some(&scan.environment)
            || scan.host.sha256!=sw.host.sha256 || scan.host_source_sha256!=sw.source_sha256 {continue;}
        if let Some(previous)=setup.primary.take() {setup.secondary.insert(0,previous);}
        setup.primary=Some(action("Open this plug-in to finish setup",offered(&scan,index,module),busy));
        setup.status="This plug-in may need you to finish setup in its own window.".into();
    }
    Ok(())
}
fn visible_session(m:&Manager,action:&ui::Action,operation:&str,timeout:Duration,
    waits:&mut Vec<ui::LockFacts>)->Result<()> {
    let (environment,retry)=target(action)?;
    let _environment=m.lock("operator-environment.lock")?;
    suspend(m,operation,None,timeout,waits)?;
    let result:Result<()>=(|| {
        let _registry=m.lock("registry.lock")?;m.require_inactive(None)?;
        let sw=software(m)?;let db=m.registry()?;
        let c=operator_catalogue(m,&sw,&db)?;
        let bindings=managed_environment_bindings(m,c.as_ref(),&db)?;
        let env=quarantined_retry_binding(m,&bindings,&db,environment,true)?
            .ok_or("operator_environment_unmanaged")?;
        let scan:inventory::Scan=read_json(&m.root.join("inventory").join(format!("{environment}.json")))?;
        let modules=managed_cli::modules(&env)?;
        let module=exact_retry_target(Some(&scan),&env,&sw.host,&sw.source_sha256,&modules,&retry)?;
        require(waiting(module),"plugin_setup_no_window_wait")?;
        let class_id=module.classes.iter().filter(|class|class.category=="Audio Module Class")
            .map(|class|class.id.clone()).collect::<Vec<_>>();
        require(class_id.len()<=1,"multiple audio classes require explicit selection")?;
        let (mut job,path)=spec(m,HostBinding {metadata:ClassSelection {
            class_id:class_id.into_iter().next().unwrap_or_default()},environment:env,
            module:module.artifact.clone(),host:sw.host.clone(),host_source_sha256:sw.source_sha256.clone(),
            compatibility:Compatibility::default()},true,true,false)?;
        job.finish_setup=true;atomic_json(&path,&job)?;
        let pending=PendingAdmission::new(job.lease.clone(),Arc::new(AdmissionCleanup::default()));
        atomic_json(&job_dir(m,operation)?.join("setup-session.json"),&job)?;
        write_operation(m,operation,&json!({"schema":2,"operation":operation,"state":"vendor_running",
            "environment":environment,"plugin_setup":true}),false)?;
        let child=spawn(m,&sw,&path,None).map_err(|e|format!("visible setup launch: {e}"))?;
        drop(_registry);
        // The supervisor bounds the interactive session at 15 minutes. Its
        // exact LVO retirement receipt owns cleanup, as for ordinary scans.
        vendor_product_cli::finish_scan_bounded(child,&job,&path,pending,Duration::from_secs(930))
            .map_err(|e|format!("visible setup retirement: {e}"))?;
        let report:Value=read_json(&job.report).map_err(|e|format!("visible setup report: {e}"))?;
        job.registration.module.verify()?;
        require(report["cleanup_confirmed"]==true && report["transport_retired"]==true,
            "plugin_setup_cleanup_unconfirmed")?;
        match report["finish_setup"]["ended"].as_str() {
            Some("stopped")=>return Err("Setup was stopped. Close this task or open the plug-in again when you are ready.".into()),
            Some("deadline")=>return Err("The setup window reached its 15-minute limit and was closed. Open the plug-in again to continue.".into()),
            Some("window_lost")=>return Err("The plug-in's setup window disappeared unexpectedly. Its setup is kept; open it again to continue.".into()),
            Some("closed") if report["error"].is_null()=>{},
            _=>return Err(linux_vst_bridge::problem::from_report(&report)
                .map(|problem|problem.summary()).unwrap_or_else(||
                    "The plug-in stopped before its setup window closed normally. Its setup is kept.".into()).into()),
        }
        write_operation(m,operation,&json!({"schema":2,"operation":operation,"state":"running",
            "environment":environment,"plugin_setup":false}),false)?;
        Ok(())
    })();
    let restored=resume_owned(m,operation);result?;restored
}
pub(super) fn execute(m:&Manager,action:&ui::Action,operation:&str,
    capacity:&dyn Fn()->Option<CapacityReadback>,timeout:Duration,waits:&mut Vec<ui::LockFacts>)->Result<Value> {
    match action {
        ui::Action::PluginSetupFocus {operation:target}=>{
            let job=running(m,target)?;let request=random_id()?;
            atomic_json(&job.report.with_extension("focus.json"),&json!({"request":request}))?;
            let deadline=Instant::now()+Duration::from_secs(4);
            loop {
                let result=optional(&job.report.with_extension("focus-result.json"))?;
                if result["request"]==request {return Ok(result);}
                require(Instant::now()<deadline,"plugin_setup_focus_unavailable")?;
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        ui::Action::PluginSetupStop {operation:target}=>{
            let job=running(m,target)?;
            fs::write(job.directory.join("vendor.stop"),format!("{}\n",job.session))?;
            Ok(json!({"stop_requested":true,"operation":target}))
        }
        ui::Action::PluginFinishSetup {environment,..}=>{
            visible_session(m,action,operation,timeout,waits)?;
            let record=onboarding::retained_environment_record(m,environment)?;
            if let Some(record)=record {
                if let Some(intent)=environment_refresh::current_intent(m,&record.environment)? {
                    return environment_refresh::execute(m,&ui::Action::EnvironmentRefreshResume {operation:intent.owner},
                        operation,capacity,timeout,waits);
                }
                if automatic_install::record_intent(m,&record)?.is_some() {
                    return automatic_install::execute(m,&ui::Action::SoftwareInstallResume {onboarding:environment.clone()},
                        operation,capacity,timeout,waits);
                }
            }
            let (_,retry)=target(action)?;
            execute_with_receipt_policy(m,&ui::Action::QuarantinedModuleRetry {environment:environment.clone(),
                scan:retry.scan,module_index:retry.module_index,module_sha256:retry.module_sha256,
                report_sha256:retry.report_sha256},Some(operation),capacity,timeout,waits)
        }
        _=>Err("plugin_setup_action".into()),
    }
}

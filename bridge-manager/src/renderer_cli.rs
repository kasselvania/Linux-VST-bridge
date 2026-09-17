//! NAUI2 operation dispatch alongside ASC; all UI targets come from canonical records.
use super::*;
use linux_vst_bridge::{operator_model as ui, renderer_application as renderer};
use serde_json::{json, Value};

pub(super) fn directory(m: &Manager) -> PathBuf {m.root.join("vendor-applications/native-access")}
pub(super) fn current(m: &Manager) -> Result<Value> {
    let p=directory(m).join("current.json");
    if p.exists(){read_json(&p)}else{Ok(Value::Null)}
}
fn operation_dir(m: &Manager, op: &str) -> Result<PathBuf> {
    require(valid_hex(op,32),"renderer_operation_identity")?;
    Ok(directory(m).join("operations").join(op))
}
fn unit(op: &str) -> Result<String> {require(valid_hex(op,32),"renderer_operation_identity")?;Ok(format!("linux-vst-bridge-renderer-{op}.service"))}
pub(super) fn live(op: &str) -> Result<bool> {
    let output=Command::new("systemctl").args(["--user","show",&unit(op)?,"-p","ActiveState","--value"]).output()?;
    if !output.status.success(){return Err("renderer_unit_state_unavailable".into());}
    Ok(matches!(std::str::from_utf8(&output.stdout)?.trim(),"active"|"activating"|"deactivating"|"reloading"))
}
pub(super) fn result(m: &Manager, op: &str) -> Result<Value> {
    let p=operation_dir(m,op)?.join("result.json");
    if p.exists(){read_json(&p)}else{Ok(Value::Null)}
}
pub(super) fn retired_value(v: &Value, op: &str) -> bool {
    v["schema"]==1 && v["operation"]==op && v["cleanup_confirmed"]==true && v["owned_live"]==0
        && matches!(v["state"].as_str(),Some("completed"|"failed"|"cancelled"))
}
pub(super) fn all_retired(m: &Manager) -> Result<bool> {
    let c=current(m)?;if c.is_null(){return Ok(true);}
    let op=c["operation"].as_str().ok_or("renderer_current_identity")?;
    Ok(retired_value(&result(m,op)?,op) && !live(op)?)
}
pub(super) fn discover(m: &Manager) -> Result<Value> {
    // Census, hashing and runner checks stay outside registry authority.
    let app=renderer::discover(&m.root)?;
    let _guard=m.lock("registry.lock")?;m.require_inactive(None)?;
    require(all_retired(m)?,"renderer_previous_cleanup_unconfirmed")?;
    let d=directory(m);private_dir(&d)?;let p=d.join("application.json");
    if p.exists(){require(read_json::<renderer::Application>(&p)?==app,"renderer_application_update_requires_transition")?;}
    else{atomic_json(&p,&app)?;}
    Ok(json!({"application":app.identity()?,"installed_identity":"verified","renderer_cause":"unresolved"}))
}
pub(super) fn launch(m: &Manager, identity: &str, policy: renderer::RendererPolicy, op: &str) -> Result<()> {
    let app:renderer::Application=read_json(&directory(m).join("application.json"))?;
    require(app.identity()?==identity,"renderer_application_selection_changed")?;app.verify(&m.root)?;
    let sw=software(m)?;let d=operation_dir(m,op)?;
    let spec=renderer::bind(&app,&sw,op,policy,&d.join("result.json"))?;
    let _guard=m.lock("registry.lock")?;m.require_inactive(None)?;
    require(all_retired(m)?,"renderer_previous_cleanup_unconfirmed")?;
    private_dir(&directory(m).join("operations"))?;
    require(!d.exists(),"renderer_duplicate_operation")?;private_dir(&d)?;
    atomic_json(&d.join("spec.json"),&spec)?;
    // Persist exact reservation before systemd submission; uncertain acknowledgment
    // never permits another launch. Reconciliation must examine this unit/receipt.
    atomic_json(&directory(m).join("current.json"),&json!({"operation":op,"application":identity,"policy":policy}))?;
    let status=Command::new("systemd-run").args(["--user","--collect","--property=UMask=0077","--property=KillMode=control-group",
        "--property=TimeoutStopSec=30","--property=StandardOutput=null","--property=StandardError=null"])
        .arg(format!("--unit={}",unit(op)?)).arg("/usr/bin/python3").arg(&sw.supervisor.path)
        .arg("--vendor-application").arg(d.join("spec.json")).status()?;
    require(status.success(),"renderer_submission_unconfirmed")
}
fn exact_current(m: &Manager, op: &str) -> Result<()> {require(current(m)?["operation"]==op,"renderer_operation_changed")}
pub(super) fn stop(m: &Manager, op: &str) -> Result<()> {
    let _guard=m.lock("registry.lock")?;exact_current(m,op)?;
    if !retired_value(&result(m,op)?,op) || live(op)? {
        require(Command::new("systemctl").args(["--user","stop",&unit(op)?]).status()?.success(),"renderer_stop_unconfirmed")?;
    }
    require(retired_value(&result(m,op)?,op) && !live(op)?,"renderer_cleanup_unconfirmed")
}
pub(super) fn focus(m: &Manager, op: &str) -> Result<Value> {
    exact_current(m,op)?;require(live(op)?,"renderer_not_running")?;
    let request=random_id()?;
    atomic_json(&operation_dir(m,op)?.join(format!("{op}-focus.json")),&json!({"operation":op,"request":request}))?;
    let deadline=Instant::now()+Duration::from_secs(4);
    loop {let v=result(m,op)?;if v["operation"]==op && v["focus_result"]["request"]==request{return Ok(v["focus_result"].clone());}
        require(Instant::now()<deadline,"renderer_focus_unconfirmed")?;std::thread::sleep(Duration::from_millis(50));}
}
pub(super) fn observe(m: &Manager, op: &str, presentation: renderer::Presentation) -> Result<Value> {
    exact_current(m,op)?;let r=result(m,op)?;
    require(r["operation"]==op && r["effective"].is_object(),"renderer_observation_root_unconfirmed")?;
    let p=operation_dir(m,op)?.join("presentation.json");let _guard=m.lock("renderer-observation.lock")?;
    require(!p.exists(),"renderer_presentation_already_recorded")?;
    let v=json!({"schema":1,"operation":op,"application":current(m)?["application"],"presentation":presentation,"origin":"human_operator"});atomic_json(&p,&v)?;Ok(v)
}
pub(super) fn project(m: &Manager, busy: Option<&str>) -> Result<Option<ui::VendorApplication>> {
    let p=directory(m).join("application.json");let env=m.root.join("environments").join(renderer::ENVIRONMENT);
    if !p.exists() && !env.exists(){return Ok(None);}
    let mut actions=vec![];let mut details=json!({"cause":"unresolved","historical_process_generation":"unavailable"});
    let mut state="installed identity unchecked".to_owned();
    let action=|label:&str,a:ui::Action,reason:Option<&str>|ui::AvailableAction{label:label.into(),action:a,disabled_reason:reason.map(Into::into)};
    if !p.exists(){actions.push(action("Verify installed Native Access",ui::Action::RendererDiscover{},busy));}
    else {
        let app:renderer::Application=read_json(&p)?;let id=app.identity()?;let c=current(m)?;
        let op=c["operation"].as_str();let retired=all_retired(m)?;
        state=if retired{"closed"}else{"supervised / cleanup pending"}.into();
        // Do not hash the 215 MiB application on every UI poll. Launch revalidates
        // all bytes; projection exposes only the retained exact registration.
        for (policy,label) in [(renderer::RendererPolicy::Inherited,"Open Native Access — inherited rendering"),(renderer::RendererPolicy::SoftwareRendering,"Open Native Access — software rendering")]{
            actions.push(action(label,ui::Action::RendererOpen{application:id.clone(),policy},if !retired{Some("Retire the exact current application operation first")}else{busy}));
        }
        if let Some(op)=op {
            details=result(m,op)?;if details.is_null(){details=json!({"application_identity":id,"operation":op,"requested":c["policy"],"effective":null});}
            details["manager_operation"]=json!({"operation":op,"state":if retired{"completed"}else{"vendor_running"}});
            let running=live(op)?;
            actions.push(action("Focus Native Access",ui::Action::RendererFocus{operation:op.into()},if running{None}else{Some("No live owned application")}));
            actions.push(action("Stop owned Native Access",ui::Action::RendererStop{operation:op.into()},if retired{Some("This exact application operation is retired")}else{None}));
            let observation=operation_dir(m,op)?.join("presentation.json");
            if observation.exists(){details["presentation"]=read_json(&observation)?;}
            else if details["effective"].is_object(){for (presentation,label) in [(renderer::Presentation::BlankWhite,"Record: blank white window"),(renderer::Presentation::RenderedNonblank,"Record: visible application content"),(renderer::Presentation::Unavailable,"Record: presentation unavailable")]{
                actions.push(action(label,ui::Action::RendererObserve{operation:op.into(),presentation},None));}}
        }
    }
    Ok(Some(ui::VendorApplication{id:renderer::ID.into(),name:"Native Access".into(),version:"3.26.0".into(),state,actions,details}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_terminal_cleanup_is_required() {
        let op="aa".repeat(16);
        let v=json!({"schema":1,"operation":op,"cleanup_confirmed":true,"owned_live":0,"state":"completed"});
        assert!(retired_value(&v,&op));
        for (key,value) in [("schema",json!(2)),("operation",json!("bb".repeat(16))),("cleanup_confirmed",json!(false)),("owned_live",json!(1)),("state",json!("running"))] {
            let mut changed=v.clone();changed[key]=value;assert!(!retired_value(&changed,&op));
        }
        assert!(unit("../../other").is_err());
    }
    #[test]
    fn operator_cannot_supply_arguments_or_foreign_identity_fields() {
        for action in [json!({"kind":"renderer_open","application":"aa".repeat(32),"policy":"software_rendering","args":["--no-sandbox"]}),json!({"kind":"renderer_open","application":"aa".repeat(32),"policy":"no_sandbox"}),json!({"kind":"renderer_stop","operation":"aa".repeat(16),"pid":42})] {
            assert!(serde_json::from_value::<ui::Action>(action).is_err());
        }
    }
}

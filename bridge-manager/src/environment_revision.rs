//! Typed environment revision readback shared by preparation and operator owners.
//! Existing operator jobs remain the sole owners of every revision edge.
use crate::*;

pub fn valid_environment_id(id: &str) -> bool {
    valid_hex(id, 32) || (id.len() == 36 && id.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    }))
}

fn operator_dir(m: &Manager, id: &str) -> Result<PathBuf> {
    require(valid_hex(id, 32), "operator_operation_identity")?;
    Ok(m.root.join("operator").join(id))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallerBinding {
    pub schema: u32,
    pub operation: String,
    pub installer: String,
    pub before: Environment,
    pub environment: Environment,
    pub created_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preserve_existing_prefix: Option<bool>,
}

pub fn same_space(before: &Environment, after: &Environment) -> bool {
    before.id == after.id && before.root == after.root
        && before.revision <= after.revision
}
pub fn resolves_to(m: &Manager, before: &Environment, after: &Environment) -> Result<bool> {
    Ok(same_space(before,after) && current_environment(m,before)?==*after)
}

pub fn current_environment(m: &Manager, bound: &Environment) -> Result<Environment> {
    require(bound.root == m.root.join("environments").join(&bound.id)
        && valid_environment_id(&bound.id), "installer_environment_identity")?;
    let current: Environment = read_json(&bound.root.join("environment.json"))?;
    require_owned_revision(m, bound, &current)?;
    Ok(current)
}

/// Historical intent is readable at a later owned revision. This proves only
/// the exact typed revision chain; it grants no inspection or launch evidence.
pub fn require_owned_revision(m: &Manager, bound: &Environment,
    target: &Environment) -> Result<()> {
    require(bound.root == m.root.join("environments").join(&bound.id)
        && valid_environment_id(&bound.id), "installer_environment_identity")?;
    require(same_space(bound, target), "installer_environment_binding_changed")?;
    if target != bound {
        // Every exact revision edge belongs to its retained typed operator job.
        let history=installer_records(m)?;
        let runtime=runtime_records(m)?;
        let vendor=vendor_records(m)?;
        let renderer=renderer_records(m)?;
        let edges:Vec<_>=history.iter().map(|r|(&r.before,&r.environment))
            .chain(runtime.iter().filter(|r|r.state != RuntimeState::Cancelled)
                .map(|r|(&r.before,&r.environment)))
            .chain(vendor.iter().map(|r|(&r.before,&r.environment)))
            .chain(renderer.iter().map(|r|(&r.before,&r.environment))).collect();
        let mut step=bound.clone();
        for _ in 0..edges.len() {
            if step == *target {break;}
            let mut successors=edges.iter().filter(|(before,_)|**before==step);
            let next=successors.next().ok_or("installer_environment_revision_unowned")?.1;
            require(successors.next().is_none(),"environment_revision_ownership_conflict")?;
            step=next.clone();
        }
        require(step == *target,"installer_environment_revision_unowned")?;
    }
    Ok(())
}

pub fn installer_record(m: &Manager, operation: &str) -> Result<InstallerBinding> {
    let dir = operator_dir(m, operation)?;
    let r: InstallerBinding = read_json(&dir.join("installation.json"))?;
    let request: operator_model::Request = read_json(&dir.join("request.json"))?;
    require(r.schema == 1 && r.operation == operation && valid_hex(&r.installer,64)
        && request.action == (operator_model::Action::EnvironmentInstallerStart {
            environment:r.environment.id.clone(), installer:r.installer.clone() })
        && same_space(&r.before,&r.environment)
        && r.before.runner == r.environment.runner
        && r.before.revision.checked_add(1) == Some(r.environment.revision)
        && r.environment.root == m.root.join("environments").join(&r.environment.id)
        && valid_environment_id(&r.environment.id), "installer_transaction_binding")?;
    Ok(r)
}

pub fn installer_records(m: &Manager) -> Result<Vec<InstallerBinding>> {
    let root = m.root.join("operator");
    if !root.try_exists()? { return Ok(vec![]); }
    let mut out = vec![];
    for (n, entry) in fs::read_dir(root)?.enumerate() {
        require(n < 4096,"operator_installation_history_bound")?;
        let path = entry?.path();
        let Some(id) = path.file_name().and_then(|v|v.to_str()) else {continue};
        if valid_hex(id,32) && path.join("installation.json").try_exists()? {
            out.push(installer_record(m,id)?);
        }
    }
    out.sort_by_key(|r|(r.created_at,r.environment.revision));
    Ok(out)
}

/// A vendor mutation edge lives in the accepted operator job, alongside
/// installer/runtime edges. It is not evidence of vendor completion.
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VendorBinding {
    pub schema:u32,pub operation:String,pub application:String,pub selected:Artifact,
    pub vendor_operation:String,pub before:Environment,pub environment:Environment,
}
pub fn vendor_record(m:&Manager,operation:&str)->Result<VendorBinding> {
    let directory=operator_dir(m,operation)?;
    let r:VendorBinding=read_json(&directory.join("vendor-maintenance.json"))?;
    let request:operator_model::Request=read_json(&directory.join("request.json"))?;
    require(r.schema==1 && r.operation==operation && valid_hex(&r.vendor_operation,32)
        && matches!(&request.action,operator_model::Action::VendorSoftwareInstall{application,..} | operator_model::Action::VendorApplicationOpen{application} if application==&r.application)
        && r.selected.path==m.root.join("vendor-applications").join(&r.application).join("application.json")
        && crate::vendor_application::ApplicationId::parse(&r.application).is_ok()
        && same_space(&r.before,&r.environment) && r.before.runner==r.environment.runner
        && r.before.revision.checked_add(1)==Some(r.environment.revision)
        && r.environment.root==m.root.join("environments").join(&r.environment.id)
        && valid_environment_id(&r.environment.id),"vendor_environment_transaction_binding")?;
    // Immutable chosen-application witness remains distinct from mutable work.
    r.selected.verify()?;
    let selected:crate::vendor_application::Application=read_json(&r.selected.path)?;
    require(selected.selector()?==r.application && selected.environment.id==r.before.id
        && selected.environment.root==r.before.root,"vendor_environment_application_binding")?;
    Ok(r)
}
pub fn vendor_records(m:&Manager)->Result<Vec<VendorBinding>> {
    let root=m.root.join("operator");if !root.exists(){return Ok(vec![])}
    let mut records=vec![];
    for (n,entry) in fs::read_dir(root)?.enumerate() {
        require(n<4096,"operator_vendor_history_bound")?;
        let path=entry?.path();let Some(id)=path.file_name().and_then(|s|s.to_str())else{continue};
        if valid_hex(id,32) && path.join("vendor-maintenance.json").exists(){records.push(vendor_record(m,id)?);}
    }
    Ok(records)
}

/// A completed renderer session owns its actual revision edge. This reader
/// grants no launch authority and never impersonates an installer/vendor job.
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RendererBinding {
    pub schema:u32, pub operation:String, pub selected:Artifact,
    pub spec:Artifact, pub result:Artifact, pub before:Environment, pub environment:Environment,
}
pub fn renderer_record(m:&Manager,operation:&str)->Result<RendererBinding> {
    let directory=operator_dir(m,operation)?;
    let r:RendererBinding=read_json(&directory.join("renderer-maintenance.json"))?;
    validate_renderer_binding(m,operation,&r)?;Ok(r)
}
pub fn validate_renderer_binding(m:&Manager,operation:&str,r:&RendererBinding)->Result<()> {
    let directory=operator_dir(m,operation)?;
    let request:operator_model::Request=read_json(&directory.join("request.json"))?;
    let operator_model::Action::RendererSoftwareInstall {application,policy,..}=request.action else {
        return Err("renderer_environment_request_binding".into());
    };
    let operation_dir=crate::renderer_session::operation_dir(m,operation)?;
    let result_path=if operation_dir.join("recovery-result.json").exists() {
        operation_dir.join("recovery-result.json")
    } else {operation_dir.join("result.json")};
    let mut after=r.before.clone();after.revision=after.revision.checked_add(1)
        .ok_or("renderer_environment_revision_bound")?;
    require(r.schema==1 && r.operation==operation && request.schema>=26
        && r.environment==after && valid_environment_id(&r.before.id)
        && r.before.root==m.root.join("environments").join(&r.before.id)
        && r.selected.path==crate::renderer_session::directory(m).join("application.json")
        && r.spec.path==operation_dir.join("spec.json") && r.result.path==result_path,
        "renderer_environment_transaction_binding")?;
    let selected:crate::renderer_application::Application=serde_json::from_slice(&r.selected.record_bytes(1024*1024)?)?;
    selected.validate_record(&m.root)?;
    require(same_space(&selected.environment,&r.before),"renderer_environment_application_space")?;
    let spec:serde_json::Value=serde_json::from_slice(&r.spec.record_bytes(1024*1024)?)?;
    let app:crate::renderer_application::Application=serde_json::from_value(spec["application"].clone())?;
    app.validate_record(&m.root)?;
    let mut expected=selected;expected.environment=r.before.clone();
    require(app==expected && app.identity()?==application
        && spec["schema"]==1 && spec["kind"]=="renderer_application"
        && spec["operation"]==operation && spec["application_identity"]==application
        && spec["renderer_policy"]==serde_json::to_value(policy)?
        && spec["report"]==serde_json::to_value(operation_dir.join("result.json"))?,
        "renderer_environment_application_binding")?;
    let result:serde_json::Value=serde_json::from_slice(&r.result.record_bytes(1024*1024)?)?;
    require(crate::renderer_session::terminal(&result,operation)
        && result["state"]=="completed" && result["cancelled"]==false && result["error"].is_null()
        && result["application_identity"]==application
        && result["requested"]==serde_json::to_value(policy)?
        && crate::native_access_dependency::session_cleanup_confirmed(&result)?,
        "renderer_environment_completion_required")?;
    if r.result.path.file_name().is_some_and(|name|name=="recovery-result.json") {
        require(result["recovery"]["prior_result_sha256"]==digest(&operation_dir.join("result.json"))?,
            "renderer_environment_recovery_binding")?;
    }
    Ok(())
}
pub fn renderer_records(m:&Manager)->Result<Vec<RendererBinding>> {
    let root=m.root.join("operator");if !root.exists(){return Ok(vec![])}
    let mut records=vec![];
    for (count,entry) in fs::read_dir(root)?.enumerate() {
        require(count<4096,"operator_renderer_history_bound")?;
        let path=entry?.path();let Some(id)=path.file_name().and_then(|s|s.to_str())else{continue};
        if valid_hex(id,32) && path.join("renderer-maintenance.json").exists() {
            records.push(renderer_record(m,id)?);
        }
    }
    Ok(records)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag="kind", rename_all="snake_case", deny_unknown_fields)]
pub enum RuntimeOwner {
    Class { registration: Box<Registration> },
    Setup { record: Artifact, environment: Environment },
    VendorApplication { record: Artifact, environment: Environment, name: String },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum RuntimeState { Prepared, Committed, Executing, Scanned, Cancelled }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeBinding {
    pub schema: u32,
    pub operation: String,
    pub before: Environment,
    pub environment: Environment,
    pub affected: Vec<RuntimeOwner>,
    pub state: RuntimeState,
}
pub fn identity(environment: &Environment) -> Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(environment)?)))
}
fn runtime_path(m: &Manager, operation: &str) -> Result<PathBuf> {
    Ok(operator_dir(m,operation)?.join("runtime-transition.json"))
}
pub fn runtime_record(m: &Manager, operation: &str) -> Result<RuntimeBinding> {
    let binding: RuntimeBinding = read_json(&runtime_path(m,operation)?)?;
    let request: operator_model::Request = read_json(&operator_dir(m,operation)?.join("request.json"))?;
    require(binding.schema == 1 && binding.operation == operation && matches!(request.schema,22..=operator_model::OPERATOR_SCHEMA)
        && binding.before.id == binding.environment.id
        && binding.before.root == binding.environment.root
        && binding.environment.root == m.root.join("environments").join(&binding.environment.id)
        && valid_environment_id(&binding.environment.id)
        && binding.before.revision.checked_add(1) == Some(binding.environment.revision)
        && binding.before.runner != binding.environment.runner
        && request.action == (operator_model::Action::EnvironmentRuntimeSelect {
            environment: binding.before.id.clone(),
            runner: catalogue::runner_key(&binding.environment.runner)?,
            expected_environment: identity(&binding.before)?,
        }), "runtime_transition_binding")?;
    binding.before.runner.validate_identity()?;
    binding.environment.runner.validate_identity()?;
    require(!binding.affected.is_empty() && binding.affected.len() <= 256,
        "runtime_transition_owners")?;
    for owner in &binding.affected {
        let environment = match owner {
            RuntimeOwner::Class {registration} => {
                registration.metadata.verify()?;
                &registration.environment
            }
            RuntimeOwner::Setup {record,environment} => {
                require(record.path == m.root.join("onboarding").join(&environment.id).join("record.json")
                    && valid_hex(&record.sha256,64),"runtime_transition_setup_binding")?;
                environment
            }
            RuntimeOwner::VendorApplication {record,environment,..} => {
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
pub fn runtime_records(m: &Manager) -> Result<Vec<RuntimeBinding>> {
    let directory = m.root.join("operator");
    if !directory.try_exists()? {return Ok(vec![]);}
    let mut records = vec![];
    for (count,entry) in fs::read_dir(directory)?.enumerate() {
        require(count < 4096,"operator_runtime_history_bound")?;
        let directory = entry?.path();
        let Some(id) = directory.file_name().and_then(|name|name.to_str()) else {continue};
        if valid_hex(id,32) && directory.join("runtime-transition.json").try_exists()? {
            records.push(runtime_record(m,id)?);
        }
    }
    records.sort_by_key(|record|(record.environment.id.clone(),record.environment.revision));
    Ok(records)
}

#[cfg(test)] mod tests {
    use super::*;
    use serde_json::{json,Value};
    fn renderer_edge() -> (test_fixture::Fixture,RendererBinding) {
        let f=test_fixture::Fixture::new();
        let mut before=f.r.environment.clone();
        before.id=crate::renderer_application::ENVIRONMENT.into();
        before.root=f.m.root.join("environments").join(&before.id);
        fs::create_dir_all(&before.root).unwrap();
        let observed:Value=serde_json::from_slice(crate::renderer_application::OBSERVATION).unwrap();
        let observed=&observed["result"];
        // Public retained identity data exercises history decoding only; no
        // proprietary image bytes or fabricated launch verification are used.
        let executable_root=before.root.join("compatdata/pfx/drive_c/Program Files/Source fixture");
        let files=observed["identity"]["files"].as_object().unwrap().iter().map(|(name,image)|
            (name.clone(),crate::renderer_application::Image {artifact:Artifact {
                path:executable_root.join(name),sha256:image["sha256"].as_str().unwrap().into()},
                size:image["size"].as_u64().unwrap()})).collect();
        let app=crate::renderer_application::Application {schema:1,id:crate::renderer_application::ID.into(),
            environment:before.clone(),files,installation:Artifact {
                path:f.m.root.join("onboarding").join(&before.id).join(format!("{}-result.json",crate::renderer_application::INSTALLATION)),
                sha256:observed["input"]["result_sha256"].as_str().unwrap().into()},
            observation_sha256:hex(&Sha256::digest(crate::renderer_application::OBSERVATION)),
            source_seal_sha256:crate::renderer_application::SEAL.into()};
        let directory=crate::renderer_session::directory(&f.m);fs::create_dir_all(&directory).unwrap();
        let selected=directory.join("application.json");atomic_json(&selected,&app).unwrap();
        let operation="ab".repeat(16);let owned=operator_dir(&f.m,&operation).unwrap();
        fs::create_dir_all(&owned).unwrap();
        let session=crate::renderer_session::operation_dir(&f.m,&operation).unwrap();
        fs::create_dir_all(&session).unwrap();let result=session.join("result.json");
        let policy=operator_model::RendererPolicy::SoftwareRendering;
        let request=operator_model::Request {schema:operator_model::OPERATOR_SCHEMA,state_token:"captured".into(),
            action:operator_model::Action::RendererSoftwareInstall {application:app.identity().unwrap(),
                policy,daw:operator_model::DawChoice::Bitwig}};
        atomic_json(&owned.join("request.json"),&request).unwrap();
        let spec=session.join("spec.json");atomic_json(&spec,&json!({"schema":1,"kind":"renderer_application",
            "application":app,"application_identity":app.identity().unwrap(),"operation":operation,
            "renderer_policy":policy,"report":result})).unwrap();
        atomic_json(&result,&json!({"schema":1,"operation":operation,"state":"completed",
            "application_identity":app.identity().unwrap(),"requested":policy,"cancelled":false,
            "error":null,"cleanup_confirmed":true,"owned_live":0,"dependency":{
                "dependency_cleanup_disposition":"graceful_service_retirement","service_retirement_confirmed":true,
                "process_cleanup_confirmed":true,"forced_cleanup_used":false}})).unwrap();
        let artifact=|path:PathBuf|Artifact {sha256:digest(&path).unwrap(),path};
        let mut environment=before.clone();environment.revision+=1;
        let binding=RendererBinding {schema:1,operation,selected:artifact(selected),spec:artifact(spec),
            result:artifact(result),before,environment};
        atomic_json(&owned.join("renderer-maintenance.json"),&binding).unwrap();
        atomic_json(&binding.environment.root.join("environment.json"),&binding.environment).unwrap();
        (f,binding)
    }
    #[test] fn renderer_history_reads_exact_owned_edge_without_live_session_or_current_images() {
        let (f,binding)=renderer_edge();
        let before=fs::read(&binding.selected.path).unwrap();
        assert_eq!(current_environment(&f.m,&binding.before).unwrap(),binding.environment);
        assert!(!crate::renderer_session::directory(&f.m).join("current.json").exists());
        assert_eq!(fs::read(&binding.selected.path).unwrap(),before);
    }
    #[test] fn renderer_history_rejects_foreign_changed_failed_or_incomplete_session_evidence() {
        for case in 0..7 {
            let (f,mut binding)=renderer_edge();
            if case==0 {binding.operation="cd".repeat(16);}
            else if case==1 {binding.environment.runner.id="foreign-runtime".into();}
            else if case==2 {binding.result.path=binding.result.path.with_file_name("foreign-result.json");}
            else {
                let mut result:Value=read_json(&binding.result.path).unwrap();
                match case {
                    3=>result["state"]=json!("failed"),
                    4=>result["cancelled"]=json!(true),
                    5=>result["dependency"]["process_cleanup_confirmed"]=json!(false),
                    _=>result["application_identity"]=json!("foreign"),
                }
                atomic_json(&binding.result.path,&result).unwrap();binding.result.sha256=digest(&binding.result.path).unwrap();
            }
            let operation="ab".repeat(16);
            atomic_json(&operator_dir(&f.m,&operation).unwrap().join("renderer-maintenance.json"),&binding).unwrap();
            assert!(renderer_record(&f.m,&operation).is_err(),"case {case}");
        }
    }
    #[test] fn renderer_history_cannot_repoint_a_genuine_selected_app_to_a_coherent_foreign_setup() {
        let (f,mut binding)=renderer_edge();
        binding.before.id="cd".repeat(16);
        binding.before.root=f.m.root.join("environments").join(&binding.before.id);
        binding.environment=binding.before.clone();binding.environment.revision+=1;
        let mut spec:Value=read_json(&binding.spec.path).unwrap();
        let mut app:crate::renderer_application::Application=serde_json::from_value(spec["application"].clone()).unwrap();
        app.environment=binding.before.clone();let identity=app.identity().unwrap();
        spec["application"]=serde_json::to_value(app).unwrap();spec["application_identity"]=json!(identity);
        atomic_json(&binding.spec.path,&spec).unwrap();binding.spec.sha256=digest(&binding.spec.path).unwrap();
        let request_path=operator_dir(&f.m,&binding.operation).unwrap().join("request.json");
        let mut request:Value=read_json(&request_path).unwrap();request["action"]["application"]=json!(identity);
        atomic_json(&request_path,&request).unwrap();
        let mut result:Value=read_json(&binding.result.path).unwrap();result["application_identity"]=json!(identity);
        atomic_json(&binding.result.path,&result).unwrap();binding.result.sha256=digest(&binding.result.path).unwrap();
        atomic_json(&operator_dir(&f.m,&binding.operation).unwrap().join("renderer-maintenance.json"),&binding).unwrap();
        assert_eq!(renderer_record(&f.m,&binding.operation).unwrap_err().to_string(),
            "renderer_environment_application_space");
    }
}

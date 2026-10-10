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
        let edges:Vec<_>=history.iter().map(|r|(&r.before,&r.environment))
            .chain(runtime.iter().filter(|r|r.state != RuntimeState::Cancelled)
                .map(|r|(&r.before,&r.environment))).collect();
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

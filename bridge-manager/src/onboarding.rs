//! Unpublished, initial-install environments. No registration/profile/publication API.
use super::*;
use linux_vst_bridge::operator_model as ui;
use serde_json::{json, Value};
use std::os::unix::fs::MetadataExt;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub schema: u32,
    pub id: String,
    pub installer: String,
    pub environment: Environment,
    pub created_at: u64,
    pub creation_operation: String,
    pub installation_operation: Option<String>,
    pub published: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_attempt: Option<String>,
}
pub fn directory(m: &Manager, id: &str) -> Result<PathBuf> {
    require(valid_hex(id, 32), "onboarding_identity")?;
    Ok(m.root.join("onboarding").join(id))
}
fn load_bound(m: &Manager, id: &str) -> Result<(Record, bool)> {
    let r: Record = read_json(&directory(m, id)?.join("record.json"))?;
    require(
        r.schema == 1
            && r.id == id
            && !r.published
            && r.environment.id == id
            && r.environment.root == m.root.join("environments").join(id)
            && read_json::<Environment>(&r.environment.root.join("environment.json"))?
                == r.environment
            && valid_hex(&r.creation_operation, 32)
            && r.installation_operation
                .as_ref()
                .is_none_or(|s| valid_hex(s, 32)),
        "onboarding_binding",
    )?;
    let registry = m.registry()?;
    let managed: Vec<_> = registry
        .classes
        .values()
        .filter(|e| e.registration.environment.id == id)
        .collect();
    require(
        managed
            .iter()
            .all(|e| e.registration.environment == r.environment),
        "onboarding_managed_environment_mismatch",
    )?;
    if let Some(previous) = &r.previous_attempt {
        require(
            valid_hex(previous, 32) && previous != id,
            "onboarding_previous_identity",
        )?;
        let prior: Record = read_json(&directory(m, previous)?.join("record.json"))?;
        require(
            prior.id == *previous
                && prior.installer == r.installer
                && !prior.published
                && prior.installation_operation.is_some(),
            "onboarding_previous_binding",
        )?;
    }
    installer_import::load(m, &r.installer)?;
    r.environment.runner.verify()?;
    Ok((r, !managed.is_empty()))
}
pub fn load(m: &Manager, id: &str) -> Result<Record> {
    let (r, managed) = load_bound(m, id)?;
    require(!managed, "onboarding_environment_qualified")?;
    Ok(r)
}
/// Read the exact retained installation authority for inventory refresh only.
/// A managed environment is admitted only when every registry owner binds the
/// complete immutable environment record retained by onboarding.
pub fn load_for_scan(m: &Manager, id: &str) -> Result<(Record, bool)> {
    load_bound(m, id)
}
pub fn load_scan_request(m: &Manager, id: &str) -> Result<Record> {
    let (r, managed) = load_for_scan(m, id)?;
    if managed {
        require(scan_required(m, &r)?, "managed_inventory_current")?;
    }
    Ok(r)
}
pub fn records(m: &Manager) -> Result<Vec<Record>> {
    let p = m.root.join("onboarding");
    if !p.exists() {
        return Ok(vec![]);
    }
    // A registered environment belongs to qualification/product management.
    // Retain its immutable onboarding history, but do not offer initial-install
    // operations against it (including an inactive retained candidate).
    let protected: std::collections::BTreeSet<_> = m.registry()?.classes.values()
        .map(|e| e.registration.environment.id.clone()).collect();
    let mut rows = vec![];
    for (n, e) in fs::read_dir(p)?.enumerate() {
        require(n < 128, "onboarding_count_bound")?;
        let e = e?;
        let id = e.file_name().to_string_lossy().into_owned();
        if valid_hex(&id, 32) && !protected.contains(&id) {
            rows.push(load(m, &id)?);
        }
    }
    rows.sort_by_key(|r| r.created_at);
    Ok(rows)
}
/// Read-only installation history remains visible after product registration.
/// Mutation entry points continue to use load(), which refuses registered environments.
pub fn history_records(m: &Manager) -> Result<Vec<Record>> {
    let dir=m.root.join("onboarding"); if !dir.exists(){return Ok(vec![])}
    let mut out=vec![];
    for entry in fs::read_dir(dir)?.take(129) {
        require(out.len()<128,"onboarding_count_bound")?;
        let path=entry?.path();let id=path.file_name().and_then(|v|v.to_str()).ok_or("onboarding_identity")?;
        if !valid_hex(id,32){continue}
        let r:Record=read_json(&path.join("record.json"))?;
        require(r.schema==1 && r.id==id && r.environment.id==id && r.environment.root==m.root.join("environments").join(id)
           && read_json::<Environment>(&r.environment.root.join("environment.json"))?==r.environment,
           "onboarding_history_binding")?;
        out.push(r);
    }
    out.sort_by_key(|r|r.created_at);Ok(out)
}
pub fn runner_key(r: &Runner) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(r)?)))
}
pub fn runners(m: &Manager) -> Result<Vec<(String, Runner)>> {
    let sw = software(m)?;
    let c = sw.catalogue(m)?;
    let mut list = vec![];
    for e in c.environments {
        let r = e.environment.runner;
        r.verify()?;
        let id = runner_key(&r)?;
        if !list.iter().any(|(key, _)| key == &id) {
            list.push((id, r));
        }
    }
    Ok(list)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileIdentity {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
    mode: u32,
    uid: u32,
}
impl FileIdentity {
    pub(super) fn read(path: &Path) -> Result<Self> {
        let m = file(path)?.metadata()?;
        Ok(Self {
            dev: m.dev(),
            ino: m.ino(),
            size: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
            mode: m.mode(),
            uid: m.uid(),
        })
    }
}
pub struct PreparedCreation {
    installer: installer_import::Installer,
    runner: Runner,
    files: Vec<(PathBuf, FileIdentity)>,
    previous_attempt: Option<String>,
}
pub fn prepare_creation(m: &Manager, installer: &str, runner: &str) -> Result<PreparedCreation> {
    require(valid_hex(installer, 64), "installer_identity")?;
    let software_path = m.root.join("software.json");
    let software_stamp = FileIdentity::read(&software_path)?;
    let sw = software(m)?;
    let catalogue = sw
        .native_catalogue
        .as_ref()
        .ok_or("onboarding_runner_catalogue_absent")?;
    let catalogue_stamp = FileIdentity::read(&catalogue.path)?;
    let installed = sw.catalogue(m)?;
    let r = installed
        .environments
        .into_iter()
        .map(|e| e.environment.runner)
        .find(|r| runner_key(r).is_ok_and(|key| key == runner))
        .ok_or("onboarding_runner_not_installed")?;
    let record_path = m.root.join("installers").join(format!("{installer}.json"));
    let record_stamp = FileIdentity::read(&record_path)?;
    let artifact = installer_import::load_record(m, installer)?;
    let mut files = vec![
        (software_path, software_stamp),
        (catalogue.path.clone(), catalogue_stamp),
        (record_path, record_stamp),
    ];
    let mut paths = vec![artifact.artifact.path.clone()];
    paths.extend(r.files.iter().map(|a| a.path.clone()));
    files.extend(
        paths
            .into_iter()
            .map(|p| Ok((p.clone(), FileIdentity::read(&p)?)))
            .collect::<Result<Vec<_>>>()?,
    );
    // Full verification is a mutation prerequisite, performed without registry
    // ownership; metadata is checked both after hashing and under admission.
    artifact.artifact.verify()?;
    r.verify()?;
    let prepared = PreparedCreation {
        installer: artifact,
        runner: r,
        files,
        previous_attempt: None,
    };
    prepared.recheck(m)?;
    Ok(prepared)
}
pub fn prepare_attempt(m: &Manager, previous: &str, runner: &str) -> Result<PreparedCreation> {
    let r = load(m, previous)?;
    let record = directory(m, previous)?.join("record.json");
    let op = r
        .installation_operation
        .as_ref()
        .ok_or("previous_installer_not_started")?;
    let result_path = directory(m, previous)?.join(format!("{op}-result.json"));
    let stamps = vec![
        (record.clone(), FileIdentity::read(&record)?),
        (result_path.clone(), FileIdentity::read(&result_path)?),
    ];
    let v = result(m, &r)?;
    require_new_attempt(
        &v,
        records(m)?.iter().any(|x| x.previous_attempt.as_deref() == Some(previous)),
    )?;
    let mut prepared = prepare_creation(m, &r.installer, runner)?;
    prepared.files.extend(stamps);
    prepared.previous_attempt = Some(previous.into());
    prepared.recheck(m)?;
    Ok(prepared)
}
impl PreparedCreation {
    fn recheck(&self, m: &Manager) -> Result<()> {
        require(
            installer_import::load_record(m, &self.installer.id)? == self.installer,
            "onboarding_installer_changed",
        )?;
        for (path, stamp) in &self.files {
            require(
                &FileIdentity::read(path)? == stamp,
                "onboarding_verified_file_changed",
            )?;
        }
        Ok(())
    }
}
pub fn create_prepared(
    m: &Manager,
    prepared: PreparedCreation,
    owner: &str,
    guard: &Lock,
) -> Result<Value> {
    guard.require_registry(m)?;
    prepared.recheck(m)?;
    create_exact(
        m,
        &prepared.installer,
        prepared.runner,
        owner,
        guard,
        prepared.previous_attempt,
    )
}
fn create_exact(
    m: &Manager,
    installer: &installer_import::Installer,
    runner: Runner,
    owner: &str,
    guard: &Lock,
    previous_attempt: Option<String>,
) -> Result<Value> {
    guard.require_registry(m)?;
    require(valid_hex(owner, 32), "onboarding_creation_operation")?;
    m.require_inactive(None)?;
    if let Some(previous) = &previous_attempt {
        // Recheck under the same registry guard that commits the new environment.
        // Two prevalidated requests cannot both create a linked successor.
        // Re-read only small custody records here. Import/runner hashing belongs
        // to prepare_attempt, outside registry authority.
        let history = history_records(m)?;
        let prior = history.iter().find(|r| r.id == *previous)
            .ok_or("onboarding_previous_missing")?;
        require(
            !prior.published && prior.installer == installer.id
                && !m.registry()?.classes.values().any(|e| e.registration.environment.id == *previous),
            "onboarding_previous_binding",
        )?;
        require_new_attempt(
            &result(m, prior)?,
            history.iter().any(|x| x.previous_attempt.as_ref() == Some(previous)),
        )?;
    }
    let id = random_id()?;
    let root = m.root.join("environments").join(&id);
    private_dir(&root)?;
    for d in [
        "compatdata",
        "runtime-var",
        "host-cache",
        "host-config",
        "host-data",
        "host-tmp",
        "client",
        "home",
    ] {
        private_dir(&root.join(d))?;
    }
    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(root.join("operation.lock"))?
        .sync_all()?;
    let env = Environment {
        id: id.clone(),
        root,
        runner,
        revision: 1,
    };
    atomic_json(&env.root.join("environment.json"), &env)?;
    let r = Record {
        schema: 1,
        id: id.clone(),
        installer: installer.id.clone(),
        environment: env,
        created_at: observation::now()?,
        creation_operation: owner.into(),
        installation_operation: None,
        published: false,
        previous_attempt,
    };
    let dir = directory(m, &id)?;
    private_dir(&dir)?;
    atomic_json(&dir.join("record.json"), &r)?;
    Ok(json!({"onboarding":id,"state":"environment_ready","published":false}))
}
pub fn result(m: &Manager, r: &Record) -> Result<Value> {
    let Some(op) = &r.installation_operation else {
        return Ok(Value::Null);
    };
    let p = directory(m, &r.id)?.join(format!("{op}-result.json"));
    if !p.exists() {
        return Ok(json!({"state":"cleanup_unconfirmed","operation":op,"cleanup_confirmed":false}));
    }
    require(
        file(&p)?.metadata()?.len() <= 16384,
        "installer_result_bound",
    )?;
    let v: Value = read_json(&p)?;
    require(
        v["schema"] == 2 && v["operation"] == *op,
        "installer_result_identity",
    )?;
    validate_installer_transaction(&v, op)?;
    Ok(v)
}
fn validate_installer_transaction(v: &Value, op: &str) -> Result<()> {
    let Some(t) = v.get("transaction") else { return Ok(()) }; // retained MF2
    require(t["schema"] == 1 && t["operation"] == op, "installer_transaction_identity")?;
    require(matches!(t["outcome"].as_str(), Some("in_progress" | "completed" | "not_installed" | "partial_installation" | "installed" | "installed_dependency_failed" | "installed_postlaunch_failed" | "child_failed" | "outer_nonzero_stage_unknown" | "cancelled" | "cleanup_unconfirmed")), "installer_transaction_outcome")?;
    require(matches!(t["durable_installation"].as_str(), Some("installed" | "partial_installation" | "not_installed" | "indeterminate" | "unavailable")), "installer_transaction_witness")?;
    if let Some(b)=t.get("launch_binding").filter(|b|!b.is_null()) {
        require(b["schema"]==1 && b["operation"]==op && b["epoch"]==2, "installer_launch_binding_identity")?;
        require(b["artifact_sha256"].as_str().is_some_and(|v|valid_hex(v,64)) && b["token_sha256"].as_str().is_some_and(|v|valid_hex(v,64)), "installer_launch_binding_digest")?;
        require(matches!(b["status"].as_str(),Some("bound"|"unavailable")), "installer_launch_binding_status")?;
        if b["status"]=="bound" {require(b["root_ordinal"].as_u64().is_some_and(|n|(1..=512).contains(&n)), "installer_launch_binding_root")?;}
    }
    if let Some(p)=t.get("presence_close") {
        require(p["schema"]==1 && p["observation_count"].as_u64().is_some_and(|n|n<=64), "installer_presence_bound")?;
        let classes=p["operation_classes"].as_array().ok_or("installer_presence_classes")?;
        require(classes.len() as u64==p["observation_count"].as_u64().unwrap_or(u64::MAX) && classes.iter().all(|c|matches!(c.as_str(),Some("unknown"|"presence_query"|"close_request"|"capability_query"))), "installer_presence_classes")?;
        require(p["actual_match_or_close_result"]=="unavailable_without_exact_object_observation", "installer_presence_result_authority")?;
    }
    Ok(())
}
pub fn retired(v: &Value) -> bool {
    v["cleanup_confirmed"] == true
        && v["owned_live"] == 0
        && matches!(
            v["state"].as_str(),
            Some("completed" | "failed" | "cancelled")
        )
}
fn require_new_attempt(v: &Value, linked: bool) -> Result<()> {
    require(retired(v), "previous_attempt_not_terminal_and_retired")?;
    if let Some(t) = v.get("transaction") {
        require(
            !matches!(t["outcome"].as_str(), Some("in_progress" | "cleanup_unconfirmed")),
            "previous_attempt_not_terminal_and_retired",
        )?;
        require(t["durable_installation"] != "installed", "installed_attempt_requires_first_launch_review")?;
        require(
            matches!(t["durable_installation"].as_str(), Some("not_installed" | "partial_installation")),
            "previous_installation_outcome_unresolved",
        )?;
    } else {
        // Legacy MF2 receipts remain readable and retain their original retry law.
        require(matches!(v["state"].as_str(), Some("failed" | "cancelled")), "previous_installation_outcome_unresolved")?;
    }
    require(!linked, "fresh_attempt_already_created")
}
fn new_attempt_actions(
    v: &Value, previous: &str, runners: &[(String, Runner)], linked: bool, busy: Option<&str>,
) -> Vec<ui::AvailableAction> {
    if require_new_attempt(v, linked).is_err() { return vec![]; }
    runners.iter().map(|(key, runner)| ui::AvailableAction {
        label: format!("New isolated attempt · {}", runner.version),
        action: ui::Action::InstallerNewAttempt { previous: previous.into(), runner: key.clone() },
        disabled_reason: busy.map(Into::into),
    }).collect()
}
pub fn mark_dead(m: &Manager, r: &Record) -> Result<()> {
    let Some(op) = &r.installation_operation else {
        return Ok(());
    };
    if !live(op)? && !retired(&result(m, r)?) {
        let mut v = result(m, r)?;
        v["schema"] = 2.into();
        v["state"] = "cleanup_unconfirmed".into();
        v["error"] = "installer_supervisor_terminated_without_retirement".into();
        atomic_json(&directory(m, &r.id)?.join(format!("{op}-result.json")), &v)?;
    }
    Ok(())
}
pub fn all_retired(m: &Manager) -> Result<bool> {
    for r in records(m)? {
        if r.installation_operation.is_some() && !retired(&result(m, &r)?) {
            return Ok(false);
        }
    }
    Ok(true)
}
pub fn unit(op: &str) -> Result<String> {
    require(valid_hex(op, 32), "installer_operation_identity")?;
    Ok(format!("linux-vst-bridge-installer-{op}.service"))
}
pub fn live(op: &str) -> Result<bool> {
    let out = Command::new("systemctl")
        .args(["--user", "show", &unit(op)?, "-p", "ActiveState", "--value"])
        .output()?;
    require(out.status.success(), "installer_unit_unavailable")?;
    match std::str::from_utf8(&out.stdout)?.trim() {
        "active" | "activating" | "deactivating" | "reloading" => Ok(true),
        "inactive" | "failed" => Ok(false),
        _ => Err("installer_unit_state".into()),
    }
}
pub fn reserve(m: &Manager, id: &str, op: &str) -> Result<Record> {
    let _lock = m.lock("onboarding.lock")?;
    let mut r = load(m, id)?;
    require(
        r.installation_operation.is_none(),
        "installer_initial_transaction_already_used",
    )?;
    require(valid_hex(op, 32), "installer_operation_identity")?;
    r.installation_operation = Some(op.into());
    let dir = directory(m, id)?;
    atomic_json(
        &dir.join(format!("{op}-result.json")),
        &json!({"schema":2,"operation":op,"state":"queued","cleanup_confirmed":false,"owned_live":null}),
    )?;
    atomic_json(&dir.join("record.json"), &r)?;
    Ok(r)
}
pub fn launch(m: &Manager, r: &Record, policy: Option<linux_vst_bridge::installer_policy::Powershell>) -> Result<()> {
    let op = r
        .installation_operation
        .as_deref()
        .ok_or("installer_operation_absent")?;
    let sw = software(m)?;
    let installer = installer_import::load(m, &r.installer)?;
    let dir = directory(m, &r.id)?;
    let path = dir.join(format!("{op}-spec.json"));
    let mut spec = json!({"schema":2,"operation":op,"environment":r.environment,"installer":installer.artifact,
        "format":installer.format,"installer_launch":sw.installer_launch,"report":dir.join(format!("{op}-result.json"))});
    if let Some(policy) = policy {
        linux_vst_bridge::installer_policy::bind(&mut spec, &sw, policy)?;
    }
    atomic_json(&path, &spec)?;
    let status = Command::new("systemd-run")
        .args([
            "--user",
            "--collect",
            "--property=UMask=0077",
            "--property=KillMode=control-group",
            "--property=TimeoutStopSec=20",
            "--property=StandardOutput=null",
            "--property=StandardError=null",
        ])
        .arg(format!("--unit={}", unit(op)?))
        .arg("/usr/bin/python3")
        .arg(sw.supervisor.path)
        .arg("--install")
        .arg(path)
        .status();
    if !status.is_ok_and(|s| s.success()) {
        // No success is inferred from systemd-run refusal. Confirm no owned unit.
        if !live(op)? {
            atomic_json(
                &dir.join(format!("{op}-result.json")),
                &json!({"schema":2,"operation":op,"state":"failed","reason":"installer_launch_failed","cleanup_confirmed":true,"owned_live":0}),
            )?;
        }
        return Err("installer_launch_failed".into());
    }
    Ok(())
}
pub fn focus(m: &Manager, id: &str, op: &str) -> Result<Value> {
    let r = load(m, id)?;
    require(
        r.installation_operation.as_deref() == Some(op) && live(op)?,
        "installer_focus_owner",
    )?;
    let request = random_id()?;
    let dir = directory(m, id)?;
    atomic_json(
        &dir.join(format!("{op}-focus.json")),
        &json!({"operation":op,"request":request}),
    )?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let v = result(m, &r)?;
        if v["focus_result"]["request"] == request {
            require(
                v["focus_result"]["result"] == "focused",
                "installer_focus_refused",
            )?;
            return Ok(v["focus_result"].clone());
        }
        require(Instant::now() < deadline, "installer_focus_timeout")?;
        std::thread::sleep(Duration::from_millis(100));
    }
}
pub fn stop(m: &Manager, id: &str, op: &str) -> Result<Value> {
    let r = load(m, id)?;
    require(
        r.installation_operation.as_deref() == Some(op),
        "installer_stop_owner",
    )?;
    require(
        Command::new("systemctl")
            .args(["--user", "stop", &unit(op)?])
            .status()?
            .success(),
        "installer_stop_failed",
    )?;
    let v = result(m, &r)?;
    require(!live(op)? && retired(&v), "installer_cleanup_unconfirmed")?;
    Ok(v)
}
pub fn projection(m: &Manager, busy: Option<&str>) -> Result<Vec<ui::Onboarding>> {
    projection_with_live(m, busy, live)
}
fn projection_with_live(
    m: &Manager,
    busy: Option<&str>,
    mut is_live: impl FnMut(&str) -> Result<bool>,
) -> Result<Vec<ui::Onboarding>> {
    let mut rows = vec![];
    let runners = runners(m)?;
    let records = history_records(m)?;
    for installer in installer_import::list(m)? {
        let bound: Vec<_> = records
            .iter()
            .filter(|r| r.installer == installer.id)
            .collect();
        if bound.is_empty() {
            rows.push(ui::Onboarding {
                failure: None,
                installer: installer.id.clone(),
                name: installer.name_hint.clone(),
                byte_size: installer.byte_size,
                format: installer.format.clone(),
                environment: None,
                state: "imported".into(),
                required_human_action: "Review installer identity and choose a pinned runner"
                    .into(),
                details: json!({"vendor":null,"product":null,"published":false}),
                actions: runners
                    .iter()
                    .map(|(key, r)| ui::AvailableAction {
                        label: format!("Create isolated environment · {}", r.version),
                        action: ui::Action::InstallerEnvironmentCreate {
                            installer: installer.id.clone(),
                            runner: key.clone(),
                        },
                        disabled_reason: busy.map(Into::into),
                    })
                    .collect(),
            });
        }
        for r in bound {
            let managed=m.registry()?.classes.values().any(|e|e.registration.environment.id==r.id);
            let v = result(m, r)?;
            let mut actions = vec![];
            let mut state = "environment_ready".to_owned();
            let mut human = "Run the installer, then operate its screens yourself";
            if let Some(op) = &r.installation_operation {
                state = v["state"].as_str().unwrap_or("cleanup_unconfirmed").into();
                if is_live(op)? {
                    human = "Use the real installer. Closing this manager does not stop it";
                    if !v["startup"]["first_problem"].is_null() || !v["transaction"]["first_failure"].is_null() {
                        state = "needs_attention".into();
                        human = "An installer-stage problem was observed. Exact Stop remains available; cancellation preserves the earlier result";
                    }
                    for (label, a) in [
                        (
                            "Focus installer",
                            ui::Action::InstallerFocus {
                                onboarding: r.id.clone(),
                                operation: op.clone(),
                            },
                        ),
                        (
                            "Stop installer",
                            ui::Action::InstallerStop {
                                onboarding: r.id.clone(),
                                operation: op.clone(),
                            },
                        ),
                    ] {
                        actions.push(ui::AvailableAction {
                            label: label.into(),
                            action: a,
                            disabled_reason: None,
                        });
                    }
                } else if retired(&v) {
                    human = if v["transaction"]["durable_installation"] == "installed" { "Installation files and registration are present. Review post-install status before reinstalling; scanning does not publish to Bitwig" } else { "Review retained installation outcome, then scan this isolated environment. A nonzero outer exit does not prove nothing was installed" };
                    actions.push(ui::AvailableAction {
                        label: "Scan installed products".into(),
                        action: ui::Action::InstallerScan {
                            onboarding: r.id.clone(),
                        },
                        disabled_reason: busy.map(Into::into),
                    });
                } else {
                    state = "cleanup_unconfirmed".into();
                    human = "Cleanup is unresolved; further installation is refused";
                }
            } else {
                actions.push(ui::AvailableAction {
                    label: "Run installer".into(),
                    action: ui::Action::InstallerStart {
                        onboarding: r.id.clone(),
                    },
                    disabled_reason: busy.map(Into::into),
                });
            }
            if r.installation_operation.is_none() && software(m).is_ok_and(|sw| linux_vst_bridge::installer_policy::eligible_adapter(&installer.format, &sw).is_ok()) {
                actions.push(ui::AvailableAction {
                    label: "Run installer with PowerShell intentionally unavailable".into(),
                    action: ui::Action::InstallerStartWithPolicy { onboarding: r.id.clone(),
                        powershell: linux_vst_bridge::installer_policy::Powershell::IntentionallyUnavailable },
                    disabled_reason: busy.map(Into::into),
                });
            }
            actions.extend(new_attempt_actions(&v, &r.id, &runners,
                records.iter().any(|x| x.previous_attempt.as_deref() == Some(&r.id)), busy));
            let scan_path = m.root.join("inventory").join(format!("{}.json", r.id));
            let scan: Value = if scan_path.exists() {
                read_json(&scan_path)?
            } else {
                Value::Null
            };
            if !scan.is_null() && retired(&v) {
                let parsed: inventory::Scan = serde_json::from_value(scan.clone())?;
                let sw = software(m)?;
                state = scan_state(&parsed, &r.environment, &sw.host, &sw.source_sha256).into();
                human = "Review discovery below. No class has been published to Bitwig";
            }
            if managed {
                actions.clear();
                if retired(&v) && scan_required(m, r)? {
                    let (_, exact_managed) = load_for_scan(m, &r.id)?;
                    require(exact_managed, "onboarding_scan_managed_binding")?;
                    state = "needs_attention".into();
                    human = "Installation retained; refresh exact current inventory before preparing or replacing a managed publication";
                    actions.push(ui::AvailableAction {
                        label: "Refresh installed products".into(),
                        action: ui::Action::InstallerScan {
                            onboarding: r.id.clone(),
                        },
                        disabled_reason: busy.map(Into::into),
                    });
                } else {
                    human = "Installation retained; manage the exact discovered products below. Initial installation is closed";
                }
            }
            rows.push(ui::Onboarding{failure:None,installer:installer.id.clone(),name:installer.name_hint.clone(),byte_size:installer.byte_size,format:installer.format.clone(),environment:Some(r.id.clone()),state,required_human_action:human.into(),details:json!({"installation":v,"scan":scan,"runner":r.environment.runner.id,"published":false,"previous_attempt":r.previous_attempt,"created_at":r.created_at,"managed_product":managed}),actions});
        }
    }
    Ok(rows)
}

/// A managed installation may be rescanned only when its retained inventory is
/// absent or stale against the exact installed environment and scanner source.
pub fn scan_required(m: &Manager, r: &Record) -> Result<bool> {
    let path = m.root.join("inventory").join(format!("{}.json", r.id));
    if !path.try_exists()? {
        return Ok(true);
    }
    let scan: inventory::Scan = read_json(&path)?;
    let sw = software(m)?;
    Ok(scan_state(&scan, &r.environment, &sw.host, &sw.source_sha256) == "needs_attention")
}

fn scan_state(
    scan: &inventory::Scan,
    environment: &Environment,
    host: &Artifact,
    source: &str,
) -> &'static str {
    if scan.schema != 1
        || &scan.environment != environment
        || scan.host.sha256 != host.sha256
        || scan.host_source_sha256 != source
        || scan.modules.iter().any(|m| {
            inventory::stale_reason(
                m,
                &scan.environment,
                &scan.host,
                &scan.host_source_sha256,
                environment,
                host,
                source,
            )
            .is_some()
        })
    {
        return "needs_attention";
    }
    if scan.modules.iter().any(|m| m.quarantine_reason.is_some()) {
        return "quarantined";
    }
    if scan
        .modules
        .iter()
        .any(|m| m.classes.iter().any(|c| c.category == "Audio Module Class"))
    {
        "installed_unqualified"
    } else {
        "no_audio_plugin_discovered"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installer_transaction_is_operation_bound_and_legacy_stays_readable() {
        let mut v=json!({"schema":2,"operation":"a","state":"failed","error":"installer_launcher_failed"});
        assert!(validate_installer_transaction(&v,"a").is_ok());
        v["transaction"]=json!({"schema":1,"operation":"a","outcome":"outer_nonzero_stage_unknown","durable_installation":"partial_installation"});
        assert!(validate_installer_transaction(&v,"a").is_ok());
        assert!(validate_installer_transaction(&v,"b").is_err());
        v["transaction"]["outcome"]="guessed_vendor_cause".into();
        assert!(validate_installer_transaction(&v,"a").is_err());
        v["transaction"]["outcome"]="installed".into();v["transaction"]["schema"]=2.into();
        assert!(validate_installer_transaction(&v,"a").is_err());
    }
    #[test]
    fn is2_nested_custody_is_operation_bound_and_cannot_claim_helper_success() {
        let mut v=json!({"transaction":{"schema":1,"operation":"a","outcome":"completed","durable_installation":"not_installed",
            "launch_binding":{"schema":1,"operation":"a","epoch":2,"artifact_sha256":"a".repeat(64),"token_sha256":"b".repeat(64),"status":"bound","root_ordinal":1},
            "presence_close":{"schema":1,"observation_count":2,"operation_classes":["presence_query","close_request"],"actual_match_or_close_result":"unavailable_without_exact_object_observation"}}});
        assert!(validate_installer_transaction(&v,"a").is_ok());
        for (field,bad) in [("operation",json!("b")),("epoch",json!(1)),("root_ordinal",json!(0)),("token_sha256",json!("raw token"))] {
            let mut changed=v.clone();changed["transaction"]["launch_binding"][field]=bad;
            assert!(validate_installer_transaction(&changed,"a").is_err());
        }
        v["transaction"]["presence_close"]["actual_match_or_close_result"]="helper_zero_means_closed".into();
        assert!(validate_installer_transaction(&v,"a").is_err());
        v["transaction"]["presence_close"]["actual_match_or_close_result"]="unavailable_without_exact_object_observation".into();
        v["transaction"]["presence_close"]["observation_count"]=65.into();
        assert!(validate_installer_transaction(&v,"a").is_err());
    }
    #[test]
    fn installed_witness_refuses_reinstallation_without_mutation() {
        let (f,i)=fixture();
        let v=create_exact(&f.m,&i,f.r.environment.runner.clone(),&"ab".repeat(16),&f.m.lock("registry.lock").unwrap(),None).unwrap();
        let id=v["onboarding"].as_str().unwrap();let op="cd".repeat(16);let r=reserve(&f.m,id,&op).unwrap();
        let path=directory(&f.m,id).unwrap().join(format!("{op}-result.json"));
        atomic_json(&path,&json!({"schema":2,"operation":op,"state":"failed","cleanup_confirmed":true,"owned_live":0,"transaction":{"schema":1,"operation":op,"outcome":"installed_dependency_failed","durable_installation":"installed"}})).unwrap();
        let before=fs::read(&path).unwrap();
        assert!(prepare_attempt(&f.m,id,&runner_key(&r.environment.runner).unwrap()).err().unwrap().to_string().contains("installed_attempt_requires_first_launch_review"));
        assert_eq!(fs::read(&path).unwrap(),before);assert_eq!(records(&f.m).unwrap().len(),1);
    }
    fn fixture() -> (test_fixture::Fixture, installer_import::Installer) {
        let f = test_fixture::Fixture::new();
        let mut b = vec![0; 1024];
        b[..2].copy_from_slice(b"MZ");
        b[60] = 128;
        b[128..132].copy_from_slice(b"PE\0\0");
        b[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        b[148] = 2;
        b[150] = 2;
        b[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let p = f.outer.join("unknown");
        fs::write(&p, b).unwrap();
        let i = installer_import::import(&f.m, file(&p).unwrap()).unwrap();
        (f, i)
    }
    #[test]
    fn policy_action_requires_current_verified_pe_adapter() {
        use linux_vst_bridge::catalogue::{Catalogue, EnvironmentBinding};
        let (f,p,_,native)=test_fixture::prepared();
        let mut b=vec![0;1024];b[..2].copy_from_slice(b"MZ");b[60]=128;
        b[128..132].copy_from_slice(b"PE\0\0");b[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        b[148]=2;b[150]=2;b[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let input=f.outer.join("policy-pe");fs::write(&input,b).unwrap();
        let i=installer_import::import(&f.m,file(&input).unwrap()).unwrap();
        let path=f.m.root.join("software/catalogue.json");
        atomic_json(&path,&Catalogue { schema:3,natives:vec![native],hosts:vec![],
            environments:vec![EnvironmentBinding {family:p.requirements.environment_family,environment:f.r.environment.clone()}] }).unwrap();
        create_exact(&f.m,&i,f.r.environment.runner.clone(),&"ab".repeat(16),&f.m.lock("registry.lock").unwrap(),None).unwrap();
        let a=f.r.host.clone();
        let mut sw=Software { manager:a.clone(),supervisor:a.clone(),ownership:a.clone(),host:a.clone(),
            source_manifest:a.clone(),source_sha256:a.sha256.clone(),operator_frontend:None,
            native_catalogue:Some(Artifact {sha256:digest(&path).unwrap(),path}),preparation_kit:None,installer_launch:None };
        let count=|| projection(&f.m,None).unwrap().iter().flat_map(|r| &r.actions)
            .filter(|a| matches!(a.action,ui::Action::InstallerStartWithPolicy{..})).count();
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();assert_eq!(count(),0);
        sw.installer_launch=Some(a.clone());atomic_json(&f.m.root.join("software.json"),&sw).unwrap();assert_eq!(count(),1);
        let p=f.outer.join("compound");let mut b=vec![0;1024];
        b[..8].copy_from_slice(&[0xd0,0xcf,0x11,0xe0,0xa1,0xb1,0x1a,0xe1]);
        b[26]=3;b[28]=0xfe;b[29]=0xff;b[30]=9;b[32]=6;fs::write(&p,b).unwrap();
        let msi=installer_import::import(&f.m,file(&p).unwrap()).unwrap();
        create_exact(&f.m,&msi,f.r.environment.runner.clone(),&"cd".repeat(16),&f.m.lock("registry.lock").unwrap(),None).unwrap();
        assert_eq!(count(),1); // PE only, despite a second eligible ordinary install.
        assert!(projection(&f.m,None).unwrap().iter().filter(|r| r.installer==msi.id)
            .flat_map(|r| &r.actions).all(|a| !matches!(a.action,ui::Action::InstallerStartWithPolicy{..})));
        sw.installer_launch.as_mut().unwrap().sha256="00".repeat(32);
        atomic_json(&f.m.root.join("software.json"),&sw).unwrap();assert!(projection(&f.m,None).is_err());
        sw.installer_launch=None;atomic_json(&f.m.root.join("software.json"),&sw).unwrap();
        // Ordinary Start remains offered even when policy is ineligible.
        assert!(projection(&f.m,None).unwrap().iter().flat_map(|r| &r.actions).any(|a| matches!(a.action,ui::Action::InstallerStart{..})));
    }
    #[test]
    fn terminal_durable_outcome_controls_projection_and_guarded_new_attempt() {
        for (durable, outcome, cleanup, owned, allowed) in [
            ("not_installed", "not_installed", true, 0, true),
            ("partial_installation", "partial_installation", true, 0, true),
            ("installed", "installed", true, 0, false),
            ("partial_installation", "cleanup_unconfirmed", false, 0, false),
            ("not_installed", "not_installed", true, 1, false),
            ("not_installed", "in_progress", true, 0, false),
            ("unavailable", "completed", true, 0, false),
        ] {
            let (f, i) = fixture();
            let first = create_exact(&f.m, &i, f.r.environment.runner.clone(), &"ab".repeat(16),
                &f.m.lock("registry.lock").unwrap(), None).unwrap();
            let id = first["onboarding"].as_str().unwrap();
            let op = "cd".repeat(16);
            let prior = reserve(&f.m, id, &op).unwrap();
            let dir = directory(&f.m, id).unwrap();
            let rp = dir.join(format!("{op}-result.json"));
            let record = dir.join("record.json");
            let receipt = json!({"schema":2,"operation":op,"state":"completed",
                "cleanup_confirmed":cleanup,"owned_live":owned,"transaction":{
                    "schema":1,"operation":op,"outcome":outcome,"durable_installation":durable}});
            atomic_json(&rp, &receipt).unwrap();
            let private = dir.join("private-evidence.json");
            fs::write(&private, b"retained private evidence").unwrap();
            let retained = [&record, &rp, &private, &prior.environment.root.join("environment.json")]
                .into_iter().map(|p| (p.clone(), fs::read(p).unwrap())).collect::<Vec<_>>();
            let runner = prior.environment.runner.clone();
            let key = runner_key(&runner).unwrap();
            let offered = new_attempt_actions(&receipt, id, &[(key.clone(), runner.clone())], false, None);
            assert_eq!(offered.len(), usize::from(allowed), "{durable}/{outcome}");
            if allowed {
                assert_eq!(offered[0].action, ui::Action::InstallerNewAttempt { previous:id.into(), runner:key.clone() });
                let blocked = new_attempt_actions(&receipt, id, &[(key, runner.clone())], false, Some("active DSP"));
                assert_eq!(blocked[0].disabled_reason.as_deref(), Some("active DSP"));
            } else {
                assert!(prepare_attempt(&f.m, id, &key).is_err());
            }
            let prepared = PreparedCreation { installer:i.clone(), runner,
                files:vec![(record.clone(), FileIdentity::read(&record).unwrap()),
                    (rp.clone(), FileIdentity::read(&rp).unwrap())], previous_attempt:Some(id.into()) };
            let next = create_prepared(&f.m, prepared, &"ef".repeat(16), &f.m.lock("registry.lock").unwrap());
            assert_eq!(next.is_ok(), allowed, "{durable}/{outcome}: {next:?}");
            assert_eq!(records(&f.m).unwrap().len(), if allowed { 2 } else { 1 });
            if let Ok(next) = next {
                let r = load(&f.m, next["onboarding"].as_str().unwrap()).unwrap();
                assert_ne!(r.environment.root, prior.environment.root);
                assert_eq!(r.previous_attempt.as_deref(), Some(id));
                assert!(!r.published && r.installation_operation.is_none());
            }
            for (path, bytes) in retained { assert_eq!(fs::read(path).unwrap(), bytes); }
            assert!(!f.m.publications.exists());
            assert!(!f.m.root.join("operator/resume.json").exists());
        }
    }
    #[test]
    fn two_prevalidated_attempts_cannot_create_two_successors() {
        let (f, i) = fixture();
        let first = create_exact(&f.m, &i, f.r.environment.runner.clone(), &"ab".repeat(16),
            &f.m.lock("registry.lock").unwrap(), None).unwrap();
        let id = first["onboarding"].as_str().unwrap();
        let op = "cd".repeat(16);
        let prior = reserve(&f.m, id, &op).unwrap();
        let dir = directory(&f.m, id).unwrap();
        let rp = dir.join(format!("{op}-result.json"));
        let record = dir.join("record.json");
        let receipt = json!({"schema":2,"operation":op,"state":"completed","cleanup_confirmed":true,"owned_live":0,
            "transaction":{"schema":1,"operation":op,"outcome":"partial_installation","durable_installation":"partial_installation"}});
        atomic_json(&rp, &receipt).unwrap();
        let bytes = fs::read(&rp).unwrap();
        let prepare = || PreparedCreation { installer:i.clone(), runner:prior.environment.runner.clone(),
            files:vec![(record.clone(), FileIdentity::read(&record).unwrap()),
                (rp.clone(), FileIdentity::read(&rp).unwrap())], previous_attempt:Some(id.into()) };
        let a = prepare(); let b = prepare();
        create_prepared(&f.m, a, &"ef".repeat(16), &f.m.lock("registry.lock").unwrap()).unwrap();
        assert!(create_prepared(&f.m, b, &"12".repeat(16), &f.m.lock("registry.lock").unwrap())
            .unwrap_err().to_string().contains("fresh_attempt_already_created"));
        let key = runner_key(&prior.environment.runner).unwrap();
        assert!(new_attempt_actions(&receipt, id, &[(key.clone(), prior.environment.runner)], true, None).is_empty());
        assert!(prepare_attempt(&f.m, id, &key).err().unwrap().to_string().contains("fresh_attempt_already_created"));
        assert_eq!(records(&f.m).unwrap().len(), 2);
        assert_eq!(fs::read(rp).unwrap(), bytes);
        assert!(!f.m.publications.exists());
    }
    #[test]
    fn qualified_environment_leaves_initial_install_projection_without_erasing_history() {
        let (f,i)=fixture();
        let v=create_exact(&f.m,&i,f.r.environment.runner.clone(),&"ab".repeat(16),&f.m.lock("registry.lock").unwrap(),None).unwrap();
        let id=v["onboarding"].as_str().unwrap();let r=load(&f.m,id).unwrap();
        let path=directory(&f.m,id).unwrap().join("record.json");let before=fs::read(&path).unwrap();
        assert_eq!(records(&f.m).unwrap().len(),1);
        let mut registration=f.r.clone();registration.environment=r.environment;
        let mut db=f.m.registry().unwrap();db.classes.insert(registration.key(),Entry {registration,publication:Publication::Published,managed_revision:None});
        atomic_json(&f.m.root.join("registry.json"),&db).unwrap();
        assert!(records(&f.m).unwrap().is_empty());
        assert!(load(&f.m,id).is_err());assert!(reserve(&f.m,id,&"cd".repeat(16)).is_err());
        assert_eq!(fs::read(path).unwrap(),before);
    }
    #[test]
    fn managed_stale_inventory_offers_only_exact_rescan_and_current_inventory_closes_it() {
        use linux_vst_bridge::catalogue::{Catalogue, EnvironmentBinding};
        let (f, _, _, native) = test_fixture::prepared();
        let mut installer_bytes = vec![0; 1024];
        installer_bytes[..2].copy_from_slice(b"MZ");
        installer_bytes[60] = 128;
        installer_bytes[128..132].copy_from_slice(b"PE\0\0");
        installer_bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        installer_bytes[148] = 2;
        installer_bytes[150] = 2;
        installer_bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        let installer_path = f.outer.join("managed-rescan-installer");
        fs::write(&installer_path, installer_bytes).unwrap();
        let i = installer_import::import(&f.m, file(&installer_path).unwrap()).unwrap();
        let catalogue_path = f.m.root.join("software/catalogue.json");
        atomic_json(
            &catalogue_path,
            &Catalogue {
                schema: 3,
                natives: vec![native],
                hosts: vec![],
                environments: vec![EnvironmentBinding {
                    family: profiles::Family::ArturiaPersistentV1,
                    environment: f.r.environment.clone(),
                }],
            },
        )
        .unwrap();
        let source_path = f.m.root.join("software/host-source-manifest.json");
        fs::write(&source_path, b"current scanner source").unwrap();
        let source = Artifact {
            sha256: digest(&source_path).unwrap(),
            path: source_path,
        };
        let sw = Software {
            installer_launch: None,
            preparation_kit: None,
            operator_frontend: None,
            manager: f.r.host.clone(),
            supervisor: f.r.host.clone(),
            ownership: f.r.host.clone(),
            host: f.r.host.clone(),
            source_manifest: source.clone(),
            source_sha256: source.sha256.clone(),
            native_catalogue: Some(Artifact {
                sha256: digest(&catalogue_path).unwrap(),
                path: catalogue_path,
            }),
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();

        let created = create_exact(
            &f.m,
            &i,
            f.r.environment.runner.clone(),
            &"ab".repeat(16),
            &f.m.lock("registry.lock").unwrap(),
            None,
        )
        .unwrap();
        let id = created["onboarding"].as_str().unwrap();
        let op = "cd".repeat(16);
        let record = reserve(&f.m, id, &op).unwrap();
        atomic_json(
            &directory(&f.m, id)
                .unwrap()
                .join(format!("{op}-result.json")),
            &json!({"schema":2,"operation":op,"state":"completed",
                "cleanup_confirmed":true,"owned_live":0}),
        )
        .unwrap();
        let inventory = f.m.root.join("inventory");
        private_dir(&inventory).unwrap();
        let mut scan = inventory::Scan {
            schema: 1,
            id: "ef".repeat(16),
            environment: record.environment.clone(),
            host: sw.host.clone(),
            host_source_sha256: "12".repeat(32),
            completed_at: 1,
            modules: vec![],
            changes: Default::default(),
        };
        atomic_json(&inventory.join(format!("{id}.json")), &scan).unwrap();
        let mut registration = f.r.clone();
        registration.environment = record.environment.clone();
        registration.host = sw.host.clone();
        registration.host_source_sha256 = sw.source_sha256.clone();
        let mut db = f.m.registry().unwrap();
        db.classes.insert(
            registration.key(),
            Entry {
                registration,
                publication: Publication::Published,
                managed_revision: None,
            },
        );
        let registry_path = f.m.root.join("registry.json");
        atomic_json(&registry_path, &db).unwrap();
        let record_path = directory(&f.m, id).unwrap().join("record.json");
        let record_bytes = fs::read(&record_path).unwrap();
        let registry_bytes = fs::read(&registry_path).unwrap();

        db.classes
            .values_mut()
            .find(|e| e.registration.environment.id == id)
            .unwrap()
            .registration
            .environment
            .revision += 1;
        atomic_json(&registry_path, &db).unwrap();
        assert!(load_for_scan(&f.m, id)
            .unwrap_err()
            .to_string()
            .contains("onboarding_managed_environment_mismatch"));
        fs::write(&registry_path, &registry_bytes).unwrap();

        assert!(load(&f.m, id).is_err());
        let (scan_owner, managed) = load_for_scan(&f.m, id).unwrap();
        assert!(managed);
        assert_eq!(
            serde_json::to_vec(&scan_owner).unwrap(),
            serde_json::to_vec(&record).unwrap()
        );
        assert!(scan_required(&f.m, &scan_owner).unwrap());
        assert_eq!(
            serde_json::to_vec(&load_scan_request(&f.m, id).unwrap()).unwrap(),
            serde_json::to_vec(&record).unwrap()
        );
        let row = projection_with_live(&f.m, None, |_| Ok(false))
            .unwrap()
            .into_iter()
            .find(|row| row.environment.as_deref() == Some(id))
            .unwrap();
        assert_eq!(row.state, "needs_attention");
        assert_eq!(row.actions.len(), 1);
        assert_eq!(
            row.actions[0].action,
            ui::Action::InstallerScan {
                onboarding: id.into()
            }
        );
        assert!(row.actions[0].disabled_reason.is_none());
        let busy = projection_with_live(&f.m, Some("active DSP"), |_| Ok(false))
            .unwrap()
            .into_iter()
            .find(|row| row.environment.as_deref() == Some(id))
            .unwrap();
        assert_eq!(busy.actions.len(), 1);
        assert_eq!(busy.actions[0].disabled_reason.as_deref(), Some("active DSP"));

        scan.host_source_sha256 = sw.source_sha256;
        atomic_json(&inventory.join(format!("{id}.json")), &scan).unwrap();
        assert!(!scan_required(&f.m, &scan_owner).unwrap());
        assert!(load_scan_request(&f.m, id)
            .unwrap_err()
            .to_string()
            .contains("managed_inventory_current"));
        let row = projection_with_live(&f.m, None, |_| Ok(false))
            .unwrap()
            .into_iter()
            .find(|row| row.environment.as_deref() == Some(id))
            .unwrap();
        assert!(row.actions.is_empty());
        assert_eq!(fs::read(record_path).unwrap(), record_bytes);
        assert_eq!(fs::read(registry_path).unwrap(), registry_bytes);
    }
    #[test]
    fn new_isolated_environment_and_initial_transaction_are_durable_unpublished() {
        let (f, i) = fixture();
        let before = f.m.registry().unwrap();
        let v = create_exact(
            &f.m,
            &i,
            f.r.environment.runner.clone(),
            &"ab".repeat(16),
            &f.m.lock("registry.lock").unwrap(),
            None,
        )
        .unwrap();
        let id = v["onboarding"].as_str().unwrap();
        let r = load(&f.m, id).unwrap();
        assert_ne!(r.environment.root, f.r.environment.root);
        assert!(r.environment.root.join("home").is_dir());
        assert!(!r.published);
        let op = "cd".repeat(16);
        let reserved = reserve(&f.m, id, &op).unwrap();
        assert_eq!(
            load(&f.m, id).unwrap().installation_operation,
            Some(op.clone())
        );
        assert!(!all_retired(&f.m).unwrap());
        assert!(reserve(&f.m, id, &"ef".repeat(16)).is_err());
        assert!(stop(&f.m, id, &"ef".repeat(16)).is_err());
        assert!(focus(&f.m, id, &"ef".repeat(16)).is_err());
        for state in ["completed", "failed", "cancelled"] {
            let p = directory(&f.m, id)
                .unwrap()
                .join(format!("{op}-result.json"));
            atomic_json(&p,&json!({"schema":2,"operation":op,"state":state,"cleanup_confirmed":true,"owned_live":0})).unwrap();
            assert!(retired(&result(&f.m, &reserved).unwrap()));
        }
        assert!(all_retired(&f.m).unwrap());
        assert_eq!(
            serde_json::to_value(before).unwrap(),
            serde_json::to_value(f.m.registry().unwrap()).unwrap()
        );
        assert!(!f.m.publications.exists());
        assert!(load(&f.m, &f.r.environment.id).is_err());
        assert!(load(&f.m, "/home/deck/.wine").is_err());
        assert!(load(&f.m, "yabridge").is_err());
    }
    #[test]
    fn fresh_attempt_retains_cancelled_history_and_cannot_replay_it() {
        let (f, i) = fixture();
        let v = create_exact(
            &f.m,
            &i,
            f.r.environment.runner.clone(),
            &"ab".repeat(16),
            &f.m.lock("registry.lock").unwrap(),
            None,
        )
        .unwrap();
        let id = v["onboarding"].as_str().unwrap();
        let op = "cd".repeat(16);
        let parent = reserve(&f.m, id, &op).unwrap();
        let rp = directory(&f.m, id)
            .unwrap()
            .join(format!("{op}-result.json"));
        let record = directory(&f.m, id).unwrap().join("record.json");
        let key = runner_key(&parent.environment.runner).unwrap();
        assert!(prepare_attempt(&f.m, id, &key).is_err());
        atomic_json(&rp,&json!({"schema":2,"operation":op,"state":"cancelled","raw_exit":-15,"cleanup_confirmed":true,"owned_live":0})).unwrap();
        let bytes = fs::read(&record).unwrap();
        let result_bytes = fs::read(&rp).unwrap();
        // Fixture catalogue setup is shared with ordinary installed software.
        let prepared = PreparedCreation {
            installer: i.clone(),
            runner: parent.environment.runner.clone(),
            files: vec![
                (record.clone(), FileIdentity::read(&record).unwrap()),
                (rp.clone(), FileIdentity::read(&rp).unwrap()),
            ],
            previous_attempt: Some(id.into()),
        };
        let next = create_prepared(
            &f.m,
            prepared,
            &"ef".repeat(16),
            &f.m.lock("registry.lock").unwrap(),
        )
        .unwrap();
        let next_id = next["onboarding"].as_str().unwrap();
        let r = load(&f.m, next_id).unwrap();
        assert_ne!(id, next_id);
        assert_eq!(r.previous_attempt.as_deref(), Some(id));
        assert_eq!(r.installer, parent.installer);
        assert!(r.installation_operation.is_none());
        assert!(!r.published);
        assert_eq!(fs::read(&record).unwrap(), bytes);
        assert_eq!(fs::read(&rp).unwrap(), result_bytes);
        assert!(reserve(&f.m, id, &"ef".repeat(16)).is_err());
        assert!(prepare_attempt(&f.m, id, &key)
            .err()
            .unwrap()
            .to_string()
            .contains("fresh_attempt_already_created"));
        assert!(!f.m.publications.exists());
    }
    #[test]
    fn replaced_runner_foreign_environment_and_result_owner_refuse() {
        let (f, i) = fixture();
        let v = create_exact(
            &f.m,
            &i,
            f.r.environment.runner.clone(),
            &"ab".repeat(16),
            &f.m.lock("registry.lock").unwrap(),
            None,
        )
        .unwrap();
        let id = v["onboarding"].as_str().unwrap();
        let op = "cd".repeat(16);
        let r = reserve(&f.m, id, &op).unwrap();
        atomic_json(&directory(&f.m,id).unwrap().join(format!("{op}-result.json")),&json!({"schema":2,"operation":"ef".repeat(16),"state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
        assert!(result(&f.m, &r).is_err());
        let mut bad = r.clone();
        bad.environment.root = f.r.environment.root.clone();
        atomic_json(&directory(&f.m, id).unwrap().join("record.json"), &bad).unwrap();
        assert!(load(&f.m, id).is_err());
        atomic_json(&directory(&f.m, id).unwrap().join("record.json"), &r).unwrap();
        fs::write(&r.environment.runner.proton, b"changed").unwrap();
        assert!(load(&f.m, id).is_err());
    }
    #[test]
    fn no_exit_code_or_foreign_operation_is_cleanup_authority() {
        for value in [
            json!({"state":"completed","raw_exit":0}),
            json!({"state":"completed","cleanup_confirmed":true,"owned_live":1}),
            json!({"state":"running","cleanup_confirmed":true,"owned_live":0}),
        ] {
            assert!(!retired(&value));
        }
        for op in ["../../foreign", "", "abc;echo", "linux-vst-bridge"] {
            assert!(unit(op).is_err());
        }
    }
    #[test]
    fn inventory_currency_quarantine_and_absence_never_grant_publication() {
        let (f, _) = fixture();
        let mut scan = inventory::Scan {
            schema: 1,
            id: "ab".repeat(16),
            environment: f.r.environment.clone(),
            host: f.r.host.clone(),
            host_source_sha256: f.r.host_source_sha256.clone(),
            completed_at: 1,
            modules: vec![],
            changes: Default::default(),
        };
        let classify = |s: &inventory::Scan| {
            scan_state(s, &f.r.environment, &f.r.host, &f.r.host_source_sha256)
        };
        assert_eq!(classify(&scan), "no_audio_plugin_discovered");
        scan.host_source_sha256 = "cd".repeat(32);
        assert_eq!(classify(&scan), "needs_attention");
        scan.host_source_sha256 = f.r.host_source_sha256.clone();
        let report = f.outer.join("scan.json");
        fs::write(&report, b"{}").unwrap();
        let c = inventory::Class {
            id: "ef".repeat(16),
            name: "Discovered instrument".into(),
            vendor: "Unknown vendor".into(),
            version: "1".into(),
            category: "Audio Module Class".into(),
            subcategories: "Instrument".into(),
            role: "instrument".into(),
        };
        scan.modules.push(inventory::Module {
            artifact: f.r.module.clone(),
            classes: vec![c.clone()],
            report: Artifact {
                path: report.clone(),
                sha256: digest(&report).unwrap(),
            },
            inspection_error: None,
            quarantine_reason: None,
        });
        assert_eq!(classify(&scan), "installed_unqualified");
        let mut second = c;
        second.id = "12".repeat(16);
        scan.modules[0].classes.push(second);
        assert_eq!(scan.modules[0].classes.len(), 2);
        scan.modules[0].quarantine_reason = Some("factory timeout".into());
        assert_eq!(classify(&scan), "quarantined");
        assert!(!f.m.publications.exists());
        assert!(f.m.registry().unwrap().classes.is_empty());
    }
}

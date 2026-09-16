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
pub fn load(m: &Manager, id: &str) -> Result<Record> {
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
    require(
        !m.registry()?
            .classes
            .values()
            .any(|e| e.registration.environment.id == id),
        "onboarding_environment_qualified",
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
    require(
        retired(&v) && matches!(v["state"].as_str(), Some("failed" | "cancelled")),
        "previous_attempt_not_failed_and_retired",
    )?;
    require(v["transaction"]["durable_installation"] != "installed", "installed_attempt_requires_first_launch_review")?;
    require(
        !records(m)?
            .iter()
            .any(|x| x.previous_attempt.as_deref() == Some(previous)),
        "fresh_attempt_already_created",
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
    require(matches!(t["outcome"].as_str(), Some("completed" | "not_installed" | "partial_installation" | "installed" | "installed_dependency_failed" | "installed_postlaunch_failed" | "child_failed" | "outer_nonzero_stage_unknown" | "cancelled" | "cleanup_unconfirmed")), "installer_transaction_outcome")?;
    require(matches!(t["durable_installation"].as_str(), Some("installed" | "partial_installation" | "not_installed" | "indeterminate" | "unavailable")), "installer_transaction_witness")?;
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
pub fn launch(m: &Manager, r: &Record) -> Result<()> {
    let op = r
        .installation_operation
        .as_deref()
        .ok_or("installer_operation_absent")?;
    let sw = software(m)?;
    let installer = installer_import::load(m, &r.installer)?;
    let dir = directory(m, &r.id)?;
    let path = dir.join(format!("{op}-spec.json"));
    atomic_json(
        &path,
        &json!({"schema":2,"operation":op,"environment":r.environment,"installer":installer.artifact,
        "format":installer.format,"report":dir.join(format!("{op}-result.json"))}),
    )?;
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
                if live(op)? {
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
            if retired(&v)
                && matches!(v["state"].as_str(), Some("failed" | "cancelled"))
                && v["transaction"]["durable_installation"] != "installed"
                && !records
                    .iter()
                    .any(|x| x.previous_attempt.as_deref() == Some(&r.id))
            {
                for (key, runner) in &runners {
                    actions.push(ui::AvailableAction {
                        label: format!("New isolated attempt · {}", runner.version),
                        action: ui::Action::InstallerNewAttempt {
                            previous: r.id.clone(),
                            runner: key.clone(),
                        },
                        disabled_reason: busy.map(Into::into),
                    });
                }
            }
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
            if managed { actions.clear(); human="Installation retained; manage the exact discovered products below. Initial installation is closed"; }
            rows.push(ui::Onboarding{failure:None,installer:installer.id.clone(),name:installer.name_hint.clone(),byte_size:installer.byte_size,format:installer.format.clone(),environment:Some(r.id.clone()),state,required_human_action:human.into(),details:json!({"installation":v,"scan":scan,"runner":r.environment.runner.id,"published":false,"previous_attempt":r.previous_attempt,"created_at":r.created_at,"managed_product":managed}),actions});
        }
    }
    Ok(rows)
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

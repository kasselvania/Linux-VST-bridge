//! Uninstall through a setup's own Windows registration. Only an Uninstall key
//! that one of this setup's own installer runs added, with exactly the command
//! bytes offered, is ever run, through the same supervised installer path
//! (owned unit, bounded, cleanup census) and the same reservation as an
//! installer run in the setup: its current runtime, every other owner retired,
//! and a new environment revision before the vendor changes the prefix.
//! Plug-ins leave the DAW first; the result says plainly what really changed.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use linux_vst_bridge::environment_revision::{UninstallBinding, UNINSTALL_REVISION};
use linux_vst_bridge::publication::RevisionRef;
use linux_vst_bridge::uninstall_entry::{self, Command as UninstallCommand, Entry};

const CHANGED: &str = "This changed since it was offered. Refresh and try again.";
const BRIDGE_SETUP: &str = "This setup belongs to the bridge itself, not to software you installed, so it can't be uninstalled here.";
const UNINSTALLER_CHANGED: &str = "This setup's uninstaller program isn't the one its installer put there, so the manager won't run it. Send Setup details.";
const MISSING: &str = "This setup's folder can't be found. Reconnect its drive or restore the folder, then try again.";
const NOT_STARTED: &str = "The uninstaller couldn't be started, so nothing was uninstalled.";
pub(super) const UNCONFIRMED_CLOSE: &str = "The manager couldn't confirm that the uninstaller closed. Use Check that it closed on the setup's card; it then checks this setup's plug-ins again with the bridge paused.";
const OTHER_OWNER: &str = "Another installer, uninstaller or app is still running. Finish it first.";
pub(super) const FINISH_CHECKING: &str = "Finish checking this setup's plug-ins";
pub(super) const CHECK_PENDING: &str = "This setup's plug-ins weren't all checked again after its uninstaller closed, so some of them may not load yet. Use Finish checking this setup's plug-ins.";
const SCAN_AGAIN: &str = "use Finish checking this setup's plug-ins on the setup's card to try again";
/// The setup's own installation records show exactly what it added.
pub(super) const INSTALLED_WITH_THIS: &str = "installed_with_this";
/// They don't: every plug-in of the setup is treated as affected.
pub(super) const UNKNOWN: &str = "unknown";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Affected {
    pub class_id: String,
    pub name: String,
    pub module: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    pub schema: u32,
    pub operation: String,
    /// The setup's current environment when reserved, and the new revision.
    pub before: Environment,
    pub environment: Environment,
    pub registration: String,
    pub name: String,
    pub command_sha256: String,
    pub command: UninstallCommand,
    pub image: Artifact,
    pub coverage: String,
    pub affected: Vec<Affected>,
    pub created_at: u64,
}

/// Custody of one uninstaller run: enough to know whether it has closed.
/// Records from an earlier test build of this feature read the same way;
/// they are never run again.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Owner {
    pub schema: u32,
    pub operation: String,
    pub environment: Environment,
    pub created_at: u64,
}
const SCHEMA: u32 = 2;

/// What a run still owes its setup once the uninstaller closed: the plug-ins
/// it took out of the DAW, and its result once the setup was scanned and its
/// plug-ins checked again. Written before the uninstaller starts, so a check
/// the manager couldn't finish (it or the computer stopped) is finished later.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Recheck {
    pub schema: u32,
    pub operation: String,
    pub taken_out: Vec<String>,
    pub outcome: Option<Value>,
}

fn record_path(m: &Manager, operation: &str) -> Result<PathBuf> { Ok(job_dir(m, operation)?.join("uninstall.json")) }
fn result_path(m: &Manager, operation: &str) -> Result<PathBuf> { Ok(job_dir(m, operation)?.join("uninstall-result.json")) }
fn recheck_path(m: &Manager, operation: &str) -> Result<PathBuf> { Ok(job_dir(m, operation)?.join("uninstall-recheck.json")) }
fn read_recheck(m: &Manager, operation: &str) -> Result<Option<Recheck>> {
    let path = recheck_path(m, operation)?;
    if !path.try_exists()? { return Ok(None); }
    let recheck: Recheck = read_json(&path)?;
    require(recheck.schema == 1 && recheck.operation == operation, "uninstall_recheck_binding")?;
    Ok(Some(recheck))
}
/// The run's pending re-check. A run an earlier test build reserved has none;
/// its list is rebuilt from what it affected that is out of the DAW by choice.
fn recheck(m: &Manager, record: &Record) -> Result<Recheck> {
    if let Some(recheck) = read_recheck(m, &record.operation)? { return Ok(recheck); }
    let db = m.registry()?;
    let taken_out = record.affected.iter().filter(|a| db.classes.get(&a.class_id).is_some_and(|entry|
        entry.publication == Publication::Removed && entry.managed_revision.as_ref()
            .is_some_and(|current| setup_removal::removed_by_musician(m, &a.class_id, current))))
        .map(|a| a.class_id.clone()).collect();
    Ok(Recheck { schema: 1, operation: record.operation.clone(), taken_out, outcome: None })
}
/// Whether this run's setup still has to be scanned and checked again.
pub(super) fn needs_check(m: &Manager, operation: &str) -> Result<bool> {
    if !owns(m, operation)? { return Ok(false); }
    // A record from an earlier test build is custody only.
    let Ok(record) = control(m, operation) else { return Ok(false) };
    Ok(recheck(m, &record)?.outcome.is_none())
}
/// What the action is doing once the uninstaller closed.
pub(super) fn progress(n: usize, total: usize) -> String {
    let doing = if total == 0 { "Scanning this setup again".to_owned() }
        else { format!("Checking this setup's plug-ins ({n} of {total})") };
    format!("{doing}… The bridge is paused for all your setups until this finishes.")
}
/// A run of this setup that closed, but whose re-check never finished.
pub(super) fn unchecked(m: &Manager, environment: &str) -> Result<Option<String>> {
    for owner in records(m)?.into_iter().filter(|r| r.environment.id == environment) {
        if read_recheck(m, &owner.operation)?.is_some_and(|recheck| recheck.outcome.is_none())
            && onboarding::retired(&result_of(m, &owner.operation)?) {
            return Ok(Some(owner.operation));
        }
    }
    Ok(None)
}

pub(super) fn records(m: &Manager) -> Result<Vec<Owner>> {
    let root = m.root.join("operator");
    if !root.try_exists()? { return Ok(vec![]); }
    let mut out = vec![];
    for (n, entry) in fs::read_dir(root)?.enumerate() {
        require(n < 4096, "uninstall_history_bound")?;
        let path = entry?.path();
        let Some(id) = path.file_name().and_then(|v| v.to_str()) else { continue };
        if valid_hex(id, 32) && path.join("uninstall.json").try_exists()? {
            let owner: Owner = read_json(&path.join("uninstall.json"))?;
            require(matches!(owner.schema, 1 | SCHEMA) && owner.operation == id
                && owner.environment.root == m.root.join("environments").join(&owner.environment.id),
                "uninstall_record_binding")?;
            out.push(owner);
        }
    }
    out.sort_by_key(|owner| owner.created_at);
    Ok(out)
}
/// Custody of one run, whichever build wrote its record.
pub(super) fn owner(m: &Manager, operation: &str) -> Result<Owner> {
    let owner: Owner = read_json(&record_path(m, operation)?)?;
    require(matches!(owner.schema, 1 | SCHEMA) && owner.operation == operation
        && owner.environment.root == m.root.join("environments").join(&owner.environment.id),
        "uninstall_record_binding")?;
    Ok(owner)
}
/// Whether this operation is an uninstaller run.
pub(super) fn owns(m: &Manager, operation: &str) -> Result<bool> {
    Ok(valid_hex(operation, 32) && record_path(m, operation)?.try_exists()?)
}
/// Resource custody survives runtime changes and missing programs.
pub(super) fn control(m: &Manager, operation: &str) -> Result<Record> {
    let record: Record = read_json(&record_path(m, operation)?)?;
    require(record.schema == SCHEMA && record.operation == operation
        && record.environment.root == m.root.join("environments").join(&record.environment.id)
        && record.before.id == record.environment.id
        && record.before.revision.checked_add(1) == Some(record.environment.revision),
        "uninstall_record_binding")?;
    Ok(record)
}
pub(super) fn result(m: &Manager, record: &Record) -> Result<Value> {
    result_of(m, &record.operation)
}
pub(super) fn result_of(m: &Manager, operation: &str) -> Result<Value> {
    let v = optional(&result_path(m, operation)?)?;
    require(v["schema"] == 2 && v["operation"] == operation, "uninstall_result_identity")?;
    Ok(v)
}
/// Every uninstaller run has retired, exactly as installer runs must.
pub(crate) fn all_retired(m: &Manager) -> Result<bool> {
    for owner in records(m)? {
        if !onboarding::retired(&result_of(m, &owner.operation)?) { return Ok(false); }
    }
    Ok(true)
}
/// Nothing of this operation's uninstaller can still run (or none was reserved).
pub(super) fn settled(m: &Manager, operation: &str) -> Result<bool> {
    if !owns(m, operation)? { return Ok(true); }
    Ok(onboarding::retired(&result_of(m, operation)?))
}
/// The unresolved uninstaller of one environment, if any: never shown to
/// have closed (a run that is still going has no closing record yet).
pub(super) fn unresolved(m: &Manager, environment: &str) -> Result<Option<String>> {
    for owner in records(m)?.into_iter().filter(|r| r.environment.id == environment) {
        // A run whose closing is recorded is history; only the others are
        // asked whether their unit still runs.
        if !onboarding::retired(&result_of(m, &owner.operation)?) {
            return Ok(Some(owner.operation));
        }
    }
    Ok(None)
}
pub(super) fn focus(m: &Manager, operation: &str) -> Result<Value> {
    let _ownership = m.lock("onboarding.lock")?;
    let record = control(m, operation)?;
    require(onboarding::live(operation)?, "installer_focus_owner")?;
    let request = random_id()?;
    atomic_json(&job_dir(m, operation)?.join(format!("{operation}-focus.json")),
        &json!({"operation":operation,"request":request}))?;
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let v = result(m, &record)?;
        if v["focus_result"]["request"] == request {
            require(v["focus_result"]["result"] == "focused", "installer_focus_refused")?;
            return Ok(v["focus_result"].clone());
        }
        require(Instant::now() < deadline, "installer_focus_timeout")?;
        std::thread::sleep(Duration::from_millis(100));
    }
}
pub(super) fn stop(m: &Manager, operation: &str) -> Result<Value> {
    let _ownership = m.lock("onboarding.lock")?;
    let record = control(m, operation)?;
    require(Command::new("systemctl").args(["--user", "stop", &onboarding::unit(operation)?])
        .status()?.success(), "installer_stop_failed")?;
    let v = result(m, &record)?;
    require(!onboarding::live(operation)? && onboarding::retired(&v), "installer_cleanup_unconfirmed")?;
    Ok(v)
}
/// Worker cleanup when its owner ended: stop a live unit, then record an
/// unretired run as unconfirmed (or failed when it never launched).
pub(super) fn finish(m: &Manager, operation: &str) -> Result<()> {
    finish_with(m, operation, &onboarding::live)
}
fn finish_with(m: &Manager, operation: &str, is_live: &dyn Fn(&str) -> Result<bool>) -> Result<()> {
    if !record_path(m, operation)?.try_exists()? { return Ok(()); }
    let record = control(m, operation)?;
    if is_live(operation)? {
        let _ = Command::new("systemctl").args(["--user", "stop", &onboarding::unit(operation)?]).status();
    }
    let mut v = result(m, &record)?;
    if !onboarding::retired(&v) {
        let launched = job_dir(m, operation)?.join("uninstall-spec.json").try_exists()?;
        v["state"] = if launched { "cleanup_unconfirmed" } else { "failed" }.into();
        if !launched { v["cleanup_confirmed"] = true.into(); v["owned_live"] = 0.into(); }
        v["error"] = if launched { "uninstaller_supervisor_terminated_without_retirement" }
            else { "uninstaller_not_launched" }.into();
        atomic_json(&result_path(m, operation)?, &v)?;
    }
    Ok(())
}

fn drive_c(environment: &Environment) -> PathBuf { environment.root.join("compatdata/pfx/drive_c") }
fn relative_key(environment: &Environment, path: &Path) -> Option<String> {
    path.strip_prefix(drive_c(environment)).ok().map(|p| p.to_string_lossy().to_lowercase())
}

/// What one of this setup's installer runs recorded: the plug-in-relevant
/// files it added or changed (with their digests) and the Uninstall keys.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Run {
    pub files: BTreeMap<String, Option<String>>,
    pub registrations: BTreeSet<String>,
    /// The witness hit its file-count or time bound, or is unreadable.
    pub incomplete: bool,
}
fn transactions(m: &Manager, record: &onboarding::Record,
    installs: &[linux_vst_bridge::environment_revision::InstallerBinding]) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    if let Some(op) = &record.installation_operation {
        out.push(onboarding::directory(m, &record.id)?.join(format!("{op}-transaction-private.json")));
    }
    for installation in installs.iter().filter(|r| r.environment.id == record.id) {
        let op = &installation.operation;
        out.push(job_dir(m, op)?.join(format!("{op}-transaction-private.json")));
    }
    Ok(out)
}
fn read_run(path: &Path) -> Run {
    let Ok(value) = read_json::<Value>(path) else { return Run { incomplete: true, ..Run::default() } };
    let durable = &value["durable_installation"];
    let list = |kind: &str| -> Vec<String> { ["added", "changed"].iter()
        .flat_map(|change| durable["delta"][kind][*change].as_array().cloned().unwrap_or_default())
        .filter_map(|v| v.as_str().map(str::to_owned)).collect() };
    let after = &value["after"]["files"];
    Run {
        files: list("files").into_iter().map(|file| {
            let digest = after.get(file.as_str()).and_then(|row| row["sha256"].as_str()).map(str::to_owned);
            (file.to_lowercase(), digest)
        }).collect(),
        registrations: list("uninstall").into_iter().collect(),
        incomplete: !durable.is_object() || durable["incomplete"].as_array().is_none_or(|bounds| !bounds.is_empty()),
    }
}
fn runs(paths: &[PathBuf]) -> Result<Vec<Run>> {
    let mut out = vec![];
    for path in paths {
        if path.try_exists()? { out.push(read_run(path)); }
    }
    Ok(out)
}

/// Plug-in modules of one setup environment: registry classes and the scan.
pub(super) fn modules(m: &Manager, environment: &Environment, db: &Registry) -> Vec<Affected> {
    let mut out: Vec<Affected> = db.classes.iter()
        .filter(|(_, entry)| entry.registration.environment.id == environment.id)
        .map(|(class, entry)| Affected { class_id: class.clone(), name: entry.registration.metadata.name.clone(),
            module: entry.registration.module.path.clone() }).collect();
    let scan = m.root.join("inventory").join(format!("{}.json", environment.id));
    if let Ok(scan) = read_json::<inventory::Scan>(&scan) {
        for module in scan.modules {
            for class in module.classes.into_iter().filter(|c| c.category == "Audio Module Class") {
                if !out.iter().any(|a| a.class_id.eq_ignore_ascii_case(&class.id)) {
                    out.push(Affected { class_id: class.id.to_uppercase(), name: class.name, module: module.artifact.path.clone() });
                }
            }
        }
    }
    out
}

/// One offered uninstall of a setup, with what it takes out of the DAW.
#[derive(Clone, Debug)]
pub(super) struct Choice {
    pub entry: Entry,
    pub command: UninstallCommand,
    pub coverage: &'static str,
    pub affected: Vec<Affected>,
    /// The uninstaller program's digest as the setup's installer left it.
    pub witness: Option<String>,
}
/// Registrations this setup's own installer runs added. A run's plug-ins are
/// attributed to a registration only when that run added no other offered
/// registration and its record is complete; otherwise the uninstall affects
/// every plug-in of the setup.
pub(super) fn choose(environment: &Environment, entries: Vec<Entry>, runs: &[Run], modules: &[Affected]) -> Vec<Choice> {
    let drive = drive_c(environment);
    let offered: Vec<(Entry, UninstallCommand)> = entries.into_iter()
        .filter(|entry| runs.iter().any(|run| run.registrations.contains(&entry.id)))
        .filter(|entry| entry.display_name.is_some() && !entry.system_component)
        .filter_map(|entry| {
            let command = uninstall_entry::command(entry.uninstall_string.as_deref()?, &drive).ok()?;
            // Windows Installer names a product's key by its exact product code.
            if let UninstallCommand::Msi { product_code } = &command {
                if !uninstall_entry::key_name(&entry).eq_ignore_ascii_case(product_code) { return None; }
            }
            Some((entry, command))
        }).collect();
    let ids: BTreeSet<_> = offered.iter().map(|(entry, _)| entry.id.clone()).collect();
    offered.into_iter().map(|(entry, command)| {
        let own: Vec<&Run> = runs.iter().filter(|run| run.registrations.contains(&entry.id)).collect();
        let sole = own.iter().all(|run| !run.incomplete
            && run.registrations.iter().filter(|id| ids.contains(*id)).count() == 1);
        // Later runs' digests replace earlier ones.
        let files: BTreeMap<&String, &Option<String>> = own.iter().flat_map(|run| run.files.iter()).collect();
        let attributed: Vec<Affected> = modules.iter().filter(|module| relative_key(environment, &module.module)
            .is_some_and(|key| files.contains_key(&key))).cloned().collect();
        let witness = match &command {
            UninstallCommand::Executable { relative, .. } => files.get(&relative.to_lowercase()).and_then(|d| (*d).clone()),
            UninstallCommand::Msi { .. } => None,
        };
        let (coverage, affected) = if sole && !attributed.is_empty() { (INSTALLED_WITH_THIS, attributed) }
            else { (UNKNOWN, modules.to_vec()) };
        Choice { entry, command, coverage, affected, witness }
    }).collect()
}

/// A small per-setup index of the registry entries and installer records the
/// overview needs, valid only while every source file is unchanged. Written
/// by mutation workers; the overview never writes, and execution never reads it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Source { path: PathBuf, len: u64, modified_ns: i128 }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Index { schema: u32, sources: Vec<Source>, entries: Vec<Entry>, runs: Vec<Run> }
fn index_path(m: &Manager, environment: &str) -> PathBuf {
    m.root.join("uninstall-index").join(format!("{environment}.json"))
}
fn sources(environment: &Environment, transactions: &[PathBuf]) -> Result<Vec<Source>> {
    let prefix = environment.root.join("compatdata/pfx");
    let mut out = vec![];
    for path in [prefix.join("system.reg"), prefix.join("user.reg")].into_iter().chain(transactions.iter().cloned()) {
        match fs::metadata(&path) {
            Ok(meta) => {
                let modified = meta.modified()?.duration_since(std::time::UNIX_EPOCH)?;
                out.push(Source { path, len: meta.len(), modified_ns: modified.as_nanos().try_into()? });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(out)
}
fn fresh(m: &Manager, record: &onboarding::Record, environment: &Environment,
    installs: &[linux_vst_bridge::environment_revision::InstallerBinding]) -> Result<(Vec<Entry>, Vec<Run>)> {
    Ok((uninstall_entry::read(environment)?, runs(&transactions(m, record, installs)?)?))
}
fn indexed(m: &Manager, record: &onboarding::Record, environment: &Environment,
    installs: &[linux_vst_bridge::environment_revision::InstallerBinding]) -> Result<(Vec<Entry>, Vec<Run>)> {
    let current = sources(environment, &transactions(m, record, installs)?)?;
    if let Ok(index) = read_json::<Index>(&index_path(m, &environment.id)) {
        if index.schema == 1 && index.sources == current { return Ok((index.entries, index.runs)); }
    }
    fresh(m, record, environment, installs)
}
/// Rebuild one setup's index after a worker changed its prefix or records.
pub(super) fn refresh_index(m: &Manager, environment: &str) -> Result<()> {
    let Some(record) = onboarding::retained_environment_record(m, environment)? else { return Ok(()) };
    let current = environment_install::current_environment(m, &record.environment)?;
    let installs = environment_install::records(m)?;
    let transactions = transactions(m, &record, &installs)?;
    let sources = sources(&current, &transactions)?;
    let (entries, runs) = fresh(m, &record, &current, &installs)?;
    private_dir(&m.root.join("uninstall-index"))?;
    atomic_json(&index_path(m, environment), &Index { schema: 1, sources, entries, runs })
}

/// Uninstall choices for each settled setup card.
pub(super) fn project(m: &Manager, setups: &[ui::InstallerSetup], records: &[onboarding::Record],
    db: &Registry, bindings: &[catalogue::EnvironmentBinding], busy: Option<&str>) -> Result<Vec<ui::Uninstaller>> {
    let protected = setup_removal::protected(m, bindings)?;
    // Read once for every setup card.
    let installs = environment_install::records(m)?;
    let mut out = vec![];
    for setup in setups {
        let Some(environment) = setup.environment.as_deref() else { continue };
        let Some(record) = records.iter().find(|r| r.id == environment) else { continue };
        if protected.contains(environment) || setup.journey.as_ref().is_some_and(|journey| !matches!(journey.phase,
            ui::InstallationPhase::Available | ui::InstallationPhase::NeedsAttention
            | ui::InstallationPhase::AwaitingApplication)) { continue; }
        // One unreadable setup must not hide every other card.
        let Ok(current) = environment_install::current_environment(m, &record.environment) else { continue };
        let modules = modules(m, &current, db);
        let Ok((entries, runs)) = indexed(m, record, &current, &installs) else { continue };
        let reason = busy.or(unchecked(m, environment).ok().flatten().map(|_| CHECK_PENDING));
        for choice in choose(&current, entries, &runs, &modules) {
            let name = choice.entry.display_name.clone().unwrap_or_default();
            let label = if choice.coverage == INSTALLED_WITH_THIS { format!("Uninstall {name}") }
                else { format!("Uninstall {name} (affects this whole setup)") };
            out.push(ui::Uninstaller { environment: environment.into(), name,
                affected: choice.affected.iter().map(|a| a.name.clone()).collect(),
                affected_classes: choice.affected.iter().map(|a| a.class_id.clone()).collect(),
                coverage: choice.coverage.into(),
                action: action(&label, ui::Action::ProductUninstall {
                    environment: environment.into(), registration: choice.entry.id.clone(),
                    command_sha256: uninstall_entry::command_sha256(choice.entry.uninstall_string.as_deref().unwrap_or_default()),
                    confirmed: false }, reason) });
        }
    }
    Ok(out)
}

fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_micros().try_into()?)
}
fn pe_image(path: &Path) -> Result<bool> {
    let mut header = [0u8; 2];
    Ok(file(path)?.read_exact(&mut header).is_ok() && header == *b"MZ")
}

/// The exact offered uninstall, resolved again from fresh records against
/// the setup's current environment. Nothing is written.
pub(super) struct Plan {
    pub before: Environment,
    pub choice: Choice,
    pub image: Artifact,
}
pub(super) fn check(m: &Manager, environment: &str, registration: &str, command_sha256: &str) -> Result<Plan> {
    require(valid_product_environment(environment), CHANGED)?;
    let setup = onboarding::retained_environment_record(m, environment)?.ok_or(CHANGED)?;
    let sw = software_record(m)?;
    let catalogue = operator_catalogue_readback(m, &sw, &m.registry()?)?;
    let bindings = managed_environment_bindings(m, catalogue.as_ref(), &m.registry()?)?;
    require(!setup_removal::protected(m, &bindings)?.contains(environment), BRIDGE_SETUP)?;
    require(m.root.join("environments").join(environment).is_dir(), MISSING)?;
    // The setup as it is now, after any runtime change.
    let before = environment_install::target(m, environment)?;
    let modules = modules(m, &before, &m.registry()?);
    let (entries, runs) = fresh(m, &setup, &before, &environment_install::records(m)?)?;
    let choice = choose(&before, entries, &runs, &modules).into_iter()
        .find(|c| c.entry.id == registration).ok_or(CHANGED)?;
    let value = choice.entry.uninstall_string.as_deref().ok_or(CHANGED)?;
    require(uninstall_entry::command_sha256(value) == command_sha256, CHANGED)?;
    let path = match &choice.command {
        UninstallCommand::Executable { relative, .. } => drive_c(&before).join(relative),
        UninstallCommand::Msi { .. } => before.runner.proton.parent().ok_or("uninstall_runner")?
            .join("files/lib/wine/x86_64-windows/msiexec.exe"),
    };
    let image = Artifact { sha256: digest(&path)?, path };
    if matches!(choice.command, UninstallCommand::Executable { .. }) {
        // The program the setup's own installer left there, unchanged since.
        match &choice.witness {
            Some(recorded) => require(*recorded == image.sha256, UNINSTALLER_CHANGED)?,
            None => require(pe_image(&image.path)?, UNINSTALLER_CHANGED)?,
        }
    }
    Ok(Plan { before, choice, image })
}

/// What the run needs from its surroundings; tests supply fixtures.
pub(super) struct Hooks<'a> {
    pub launch: &'a dyn Fn(&Manager, &Record) -> Result<()>,
    pub is_live: &'a dyn Fn(&str) -> Result<bool>,
    pub vendor_retired: &'a dyn Fn() -> Result<bool>,
    pub running: &'a mut dyn FnMut() -> Result<()>,
    /// Progress once the uninstaller closed: plug-in n of N is being checked.
    pub checking: &'a mut dyn FnMut(usize, usize) -> Result<()>,
    pub rescan: &'a dyn Fn(&Manager, Environment) -> Result<Value>,
    /// Check one plug-in again in the setup's new revision and publish it
    /// (class, its current publication, a fresh work identity).
    pub revalidate: &'a dyn Fn(&Manager, &str, &RevisionRef, &str) -> Result<RevisionRef>,
}
/// Every refusal an installer run in this setup would get, checked without
/// writing anything: before plug-ins leave the DAW, and again under the
/// reservation's locks.
fn admissible(m: &Manager, plan: &Plan, hooks: &Hooks) -> Result<()> {
    let environment = plan.before.id.as_str();
    require(m.root.join("environments").join(environment).is_dir(), MISSING)?;
    m.require_inactive(None)?;
    require(onboarding::all_retired(m)? && (hooks.vendor_retired)()?, OTHER_OWNER)?;
    for owner in onboarding::history_records(m)?.into_iter().filter_map(|r| r.installation_operation)
        .chain(environment_install::records(m)?.into_iter().map(|r| r.operation))
        .chain(records(m)?.into_iter().map(|r| r.operation)) {
        require(!(hooks.is_live)(&owner)?, OTHER_OWNER)?;
    }
    environment_runtime::require_settled(m, environment)?;
    require(unchecked(m, environment)?.is_none(), CHECK_PENDING)?;
    require(environment_install::target(m, environment)? == plan.before, CHANGED)?;
    require(plan.before.root.join("compatdata/pfx/system.reg").is_file(), MISSING)?;
    plan.before.runner.verify()?;
    plan.image.verify().map_err(|_| UNINSTALLER_CHANGED)?;
    Ok(())
}
/// The same reservation as an installer run in this setup: its custody record
/// first, then the owned revision edge, then the new revision itself, all
/// before the vendor changes the prefix. `taken_out` are the plug-ins this
/// action took out of the DAW, to be put back if their files remain.
pub(super) fn reserve(m: &Manager, plan: &Plan, operation: &str, taken_out: &[String], hooks: &Hooks) -> Result<Record> {
    let _ownership = m.lock("onboarding.lock")?;
    let _registry = m.lock("registry.lock")?;
    admissible(m, plan, hooks)?;
    let mut after = plan.before.clone();
    after.revision = after.revision.checked_add(1).ok_or("environment_revision_exhausted")?;
    let dir = job_dir(m, operation)?;
    require(!record_path(m, operation)?.try_exists()?, "uninstall_operation_already_used")?;
    private_dir(&dir)?;
    let record = Record { schema: SCHEMA, operation: operation.into(), before: plan.before.clone(), environment: after.clone(),
        registration: plan.choice.entry.id.clone(), name: plan.choice.entry.display_name.clone().unwrap_or_default(),
        command_sha256: uninstall_entry::command_sha256(plan.choice.entry.uninstall_string.as_deref().unwrap_or_default()),
        command: plan.choice.command.clone(), image: plan.image.clone(), coverage: plan.choice.coverage.into(),
        affected: plan.choice.affected.clone(), created_at: now()? };
    atomic_json(&result_path(m, operation)?, &json!({"schema":2,"operation":operation,"state":"reserved",
        "cleanup_confirmed":false,"owned_live":null}))?;
    atomic_json(&recheck_path(m, operation)?, &Recheck { schema: 1, operation: operation.into(),
        taken_out: taken_out.to_vec(), outcome: None })?;
    atomic_json(&record_path(m, operation)?, &record)?;
    atomic_json(&dir.join(UNINSTALL_REVISION), &UninstallBinding { schema: 1, operation: operation.into(),
        registration: record.registration.clone(), before: plan.before.clone(), environment: after.clone() })?;
    // Every plug-in of this setup is checked again before it is used again.
    atomic_json(&after.root.join("environment.json"), &after)?;
    Ok(record)
}

/// Start the supervised uninstaller for a reserved record.
pub(super) fn launch(m: &Manager, record: &Record) -> Result<()> {
    let dir = job_dir(m, &record.operation)?;
    let report = result_path(m, &record.operation)?;
    let (format, product_code, arguments) = match &record.command {
        UninstallCommand::Executable { arguments, .. } => ("uninstall_executable", Value::Null, arguments.clone()),
        UninstallCommand::Msi { product_code } => ("uninstall_msi", json!(product_code), vec![]),
    };
    let spec = dir.join("uninstall-spec.json");
    atomic_json(&spec, &json!({"schema":2,"operation":record.operation,"environment":record.environment,
        "installer":record.image,"format":format,"report":report,"preserve_existing_prefix":true,
        "uninstall":{"schema":1,"registration":record.registration,"product_code":product_code,"arguments":arguments}}))?;
    onboarding::launch_spec(m, &record.operation, &spec, &report)
}

fn join(names: &[String]) -> String { names.join(", ") }
fn one(names: &[String]) -> bool { names.len() == 1 }
/// "X stays out of your DAW; add it back…" for plug-ins still taken out.
fn still_out(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!(" {} stays out of your DAW; add it back from Plug-ins if you still want it.", names[0]),
        _ => format!(" {} stay out of your DAW; add them back from Plug-ins if you still want them.", join(names)),
    }
}
/// Refusals in plain words; unknown ones stay exact.
fn plain(error: Box<dyn std::error::Error + Send + Sync>) -> String {
    let text = error.to_string();
    match text.as_str() {
        "installer_launch_failed" => NOT_STARTED.into(),
        "operator_installer_active" => OTHER_OWNER.into(),
        "installer_environment_binding_changed" | "uninstall_environment_transaction_binding"
            | "installer_initial_retirement_required" => CHANGED.into(),
        "active_device_lease" | "active_maintenance_lease" =>
            "Close the plug-ins open in your DAW first, then try again.".into(),
        "runtime_transition_recovery_required" =>
            "Finish the interrupted runtime change in this setup first.".into(),
        _ => text,
    }
}
/// Why one plug-in couldn't be checked again, in plain words. The exact
/// reason stays in the result for Setup details.
fn recheck_reason(code: &str) -> &'static str {
    match code {
        "unchanged_publication_selection_missing" => "not in the setup's latest scan",
        "unchanged_publication_content_or_runtime_changed" => "plug-in file or runtime changed",
        "unchanged_publication_host_changed" | "bridge_refresh_predecessor_changed"
            | "loaded_engine_admission_contract_missing" => "prepared by another version of the bridge",
        "unchanged_publication_preferences_changed" => "settings changed meanwhile",
        // Its bridge files or settings differ from what this bridge prepares now.
        "unchanged_publication_configuration_changed" => "prepared by another version of the bridge or with other settings",
        "unchanged_publication_inspection_changed" | "unchanged_publication_engine_changed" =>
            "the plug-in reported something different this time",
        "active_device_lease" | "active_maintenance_lease" | "operator_vendor_active"
            | "operation already running" => "the bridge was busy",
        "runtime_transition_recovery_required" => "the setup's runtime change isn't finished",
        _ if code.contains("artifact missing or changed") || code.contains("No such file") => "files changed",
        _ => "the check didn't pass",
    }
}
/// A failure that passes when tried again: the bridge was busy, something held
/// a lock, or the setup's runtime change wasn't finished. Anything else is
/// recorded as the result, so it never keeps the setup waiting.
fn temporary(code: &str) -> bool {
    matches!(code, "active_device_lease" | "active_maintenance_lease" | "operator_vendor_active"
        | "operation already running" | "runtime_transition_recovery_required"
        | "publication_performance_changed")
}

/// The setup's plug-ins checked again after the uninstaller changed it.
#[derive(Debug, Default)]
pub(super) struct Reconnected {
    /// Put back into the DAW: taken out for this uninstall, which didn't remove them.
    pub back: Vec<String>,
    /// Still in the DAW, now checked against the setup as it is.
    pub rechecked: Vec<String>,
    /// (name, exact reason)
    pub failed: Vec<(String, String)>,
}
/// Check every plug-in of the setup still registered at an older revision,
/// one at a time: the ones in the DAW, and the ones this action took out,
/// which go back. Files that are gone are left alone, and a plug-in already
/// checked before the manager stopped isn't checked twice.
fn reconnect(m: &Manager, record: &Record, taken_out: &[String], hooks: &mut Hooks) -> Result<Reconnected> {
    let mut out = Reconnected::default();
    let mut due = vec![];
    for (class, entry) in m.registry()?.classes.into_iter()
        .filter(|(_, e)| e.registration.environment.id == record.environment.id) {
        let Some(current) = entry.managed_revision.clone() else { continue };
        if setup_removal::gone(&entry.registration.module.path) { continue; }
        let back = taken_out.contains(&class);
        let name = entry.registration.metadata.name.clone();
        // Checked again already: before the manager stopped, or by a later
        // job in the setup (an installer run or a runtime change).
        if entry.registration.environment.revision >= record.environment.revision {
            if back && entry.publication == Publication::Published { out.back.push(name); }
            continue;
        }
        if entry.publication == Publication::Published || (back && entry.publication == Publication::Removed) {
            due.push((class, current, name, back));
        }
    }
    let total = due.len();
    for (n, (class, current, name, back)) in due.into_iter().enumerate() {
        // Progress is only shown; a failed write never stops the check.
        let _ = (hooks.checking)(n + 1, total);
        match (hooks.revalidate)(m, &class, &current, &random_id()?) {
            Ok(_) if back => out.back.push(name),
            Ok(_) => out.rechecked.push(name),
            Err(error) => out.failed.push((name, error.to_string())),
        }
    }
    Ok(out)
}
fn it(names: &[String]) -> &'static str { if one(names) { "it" } else { "them" } }
fn reconnected_text(reconnected: &Reconnected) -> String {
    let mut text = String::new();
    let back = &reconnected.back;
    if !back.is_empty() {
        text.push_str(&format!(" {} {} back in your DAW, checked again with this setup.", join(back),
            if one(back) { "is" } else { "are" }));
    }
    let rechecked = &reconnected.rechecked;
    if !rechecked.is_empty() {
        text.push_str(&format!(" {} {} checked again and {} in your DAW. If you opened {} while the bridge was paused, close {} and open {} again.",
            join(rechecked), if one(rechecked) { "was" } else { "were" }, if one(rechecked) { "stays" } else { "stay" },
            it(rechecked), it(rechecked), it(rechecked)));
    }
    // One sentence per reason, in the order the plug-ins were checked.
    let mut reasons: Vec<(&'static str, bool, Vec<String>)> = vec![];
    for (name, code) in &reconnected.failed {
        let reason = recheck_reason(code);
        match reasons.iter_mut().find(|(r, _, _)| *r == reason) {
            Some((_, _, names)) => names.push(name.clone()),
            None => reasons.push((reason, temporary(code), vec![name.clone()])),
        }
    }
    for (reason, temporary, names) in reasons {
        let next = if temporary { format!("; {SCAN_AGAIN}") }
            else { format!(". Run {} installer again in this setup, or delete the setup",
                if one(&names) { "its" } else { "their" }) };
        text.push_str(&format!(" {} couldn't be checked again: {reason}{next}.", join(&names)));
    }
    text
}

/// The whole uninstall, as one action: its plug-ins leave the DAW, the
/// setup's own uninstaller runs, and the setup is scanned and its remaining
/// plug-ins are checked again.
pub(super) fn execute(m: &Manager, environment: &str, registration: &str, command_sha256: &str,
    operation: &str, hooks: &mut Hooks) -> Result<Value> {
    let plan = check(m, environment, registration, command_sha256).map_err(plain)?;
    // Nothing leaves the DAW unless the uninstaller may run now.
    admissible(m, &plan, hooks).map_err(plain)?;
    let classes: Vec<_> = plan.choice.affected.iter().map(|a| a.class_id.clone()).collect();
    let db = m.registry()?;
    let taken_out: Vec<String> = classes.iter().filter(|class| db.classes.get(*class)
        .is_some_and(|entry| entry.publication == Publication::Published)).cloned().collect();
    let removed = setup_removal::remove_classes_from_daw(m, &classes)?;
    let record = reserve(m, &plan, operation, &taken_out, hooks)
        .map_err(|error| format!("{}{}", plain(error), still_out(&removed)))?;
    let launched = (hooks.launch)(m, &record);
    if launched.is_ok() {
        (hooks.running)()?;
        while (hooks.is_live)(operation)? { std::thread::sleep(Duration::from_millis(500)); }
    }
    if !onboarding::retired(&result(m, &record)?) {
        finish_with(m, operation, hooks.is_live)?;
    }
    require(onboarding::retired(&result(m, &record)?), UNCONFIRMED_CLOSE)?;
    let outcome = complete(m, operation, hooks)?;
    if let Err(error) = launched {
        let reason = plain(error);
        let detail = if reason == NOT_STARTED { String::new() } else { format!(" ({reason})") };
        let message = outcome["message"].as_str().unwrap_or(NOT_STARTED);
        return Err(message.replacen(NOT_STARTED, &format!("{NOT_STARTED}{detail}"), 1).into());
    }
    Ok(outcome)
}
/// Once the uninstaller closed, on its own or shown closed later: scan the
/// setup, take out plug-ins whose files are gone, and check every other
/// plug-in again, putting back the ones this run took out while their files
/// remain. Recorded once; until then, calling it again finishes the check.
pub(super) fn complete(m: &Manager, operation: &str, hooks: &mut Hooks) -> Result<Value> {
    let record = control(m, operation)?;
    let value = result(m, &record)?;
    require(onboarding::retired(&value), UNCONFIRMED_CLOSE)?;
    let mut recheck = recheck(m, &record)?;
    if let Some(outcome) = &recheck.outcome { return Ok(outcome.clone()); }
    let _ = (hooks.checking)(0, 0);
    let scanned = (hooks.rescan)(m, record.environment.clone());
    // A registry that can't be read reports nothing as uninstalled; the
    // setup's plug-ins are still checked.
    let registered = uninstall_entry::read(&record.environment).ok()
        .map(|entries| entries.iter().any(|entry| entry.id == record.registration));
    let collateral = collateral(m, &record)?;
    // The musician asked to uninstall the plug-ins this registration
    // installed: once it is uninstalled, they stay out even if a file of
    // theirs remains. Plug-ins of other products go back.
    let finished = value["state"] == "completed" && value["raw_exit"] == 0;
    let asked: Vec<String> = if finished && registered == Some(false) && record.coverage == INSTALLED_WITH_THIS {
        record.affected.iter().map(|a| a.class_id.clone()).collect() } else { vec![] };
    let put_back: Vec<String> = recheck.taken_out.iter().filter(|class| !asked.contains(class)).cloned().collect();
    let reconnected = if scanned.is_ok() { reconnect(m, &record, &put_back, hooks)? }
        else { Reconnected::default() };
    let _ = refresh_index(m, &record.environment.id);
    let db = m.registry()?;
    let failed: Vec<&String> = reconnected.failed.iter().map(|(name, _)| name).collect();
    let still: Vec<String> = if scanned.is_err() { vec![] } else {
        recheck.taken_out.iter().filter_map(|class| db.classes.get(class))
            .filter(|entry| entry.publication != Publication::Published && !setup_removal::gone(&entry.registration.module.path))
            .map(|entry| entry.registration.metadata.name.clone())
            .filter(|name| !failed.contains(&name)).collect()
    };
    let mut outcome = outcome(&record, &value, registered, &still, &collateral, &reconnected);
    match scanned {
        Err(error) => {
            outcome["message"] = format!("{} The setup couldn't be scanned again, so its plug-ins weren't checked again; {SCAN_AGAIN}.",
                outcome["message"].as_str().unwrap_or_default()).into();
            outcome["scan_error"] = error.to_string().into();
        }
        // A check that passes when tried again stays offered on the card.
        Ok(_) if reconnected.failed.iter().any(|(_, code)| temporary(code)) => {}
        Ok(_) => {
            recheck.outcome = Some(outcome.clone());
            atomic_json(&recheck_path(m, operation)?, &recheck)?;
        }
    }
    Ok(outcome)
}
/// Published plug-ins of the setup whose files the uninstaller also removed.
fn collateral(m: &Manager, record: &Record) -> Result<Vec<String>> {
    let classes: Vec<String> = m.registry()?.classes.iter()
        .filter(|(_, entry)| entry.registration.environment.id == record.environment.id
            && entry.publication == Publication::Published
            && setup_removal::gone(&entry.registration.module.path))
        .map(|(class, _)| class.clone()).collect();
    setup_removal::remove_classes_from_daw(m, &classes)
}

/// What really happened, from the supervisor's own result, the setup's
/// registry (`None` when it couldn't be read) and the plug-in files. A
/// cancelled, failed or declined uninstall is never reported as uninstalled.
pub(super) fn outcome(record: &Record, result: &Value, registered: Option<bool>, still: &[String],
    collateral: &[String], reconnected: &Reconnected) -> Value {
    let started = result["error"] != "uninstaller_not_launched" && result["reason"] != "installer_launch_failed";
    let finished = result["state"] == "completed" && result["raw_exit"] == 0;
    let (gone, remaining): (Vec<_>, Vec<_>) = record.affected.iter().partition(|a| setup_removal::gone(&a.module));
    let names = |rows: &[&Affected]| rows.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
    let uninstalled = finished && registered == Some(false);
    let mut message = if !started {
        NOT_STARTED.to_owned()
    } else if !finished {
        let why = if result["state"] == "cancelled" { "it was stopped" }
            else { "it was cancelled in its window or ended with an error" };
        format!("{}'s uninstaller didn't finish ({why}), so nothing is reported as uninstalled.", record.name)
    } else if registered.is_none() {
        format!("{}'s uninstaller closed, but the manager couldn't read this setup's Windows registry, so nothing is reported as uninstalled.", record.name)
    } else if registered == Some(true) {
        format!("{}'s uninstaller closed, but {} is still installed in this setup.", record.name, record.name)
    } else if remaining.is_empty() {
        format!("{} was uninstalled.", record.name)
    } else {
        format!("{} was uninstalled, but these plug-ins' files are still there: {}.", record.name,
            join(&names(&remaining)))
    };
    message.push_str(&reconnected_text(reconnected));
    message.push_str(&still_out(still));
    if !collateral.is_empty() {
        message.push_str(&format!(" {} also left your DAW because {} files are gone.", join(collateral),
            if one(collateral) { "its" } else { "their" }));
    }
    json!({"name":record.name,"finished":finished,"registration_remains":registered,
        "uninstalled":if uninstalled { names(&gone) } else { vec![] },"still_installed":names(&remaining),
        "removed_from_daw":still,"back_in_daw":reconnected.back,"rechecked":reconnected.rechecked,
        "recheck_failed":reconnected.failed.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        "recheck_errors":reconnected.failed.iter().map(|(name, code)| json!({"name":name,"reason":code})).collect::<Vec<_>>(),
        "also_removed":collateral,"message":message,"history_retained":true})
}

#[cfg(test)]
mod tests;

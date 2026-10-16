//! Uninstall through a setup's own Windows registration. Only an Uninstall key
//! that one of this setup's own installer runs added, with exactly the command
//! bytes offered, is ever run, through the same supervised installer path
//! (owned unit, bounded, cleanup census). Its plug-ins leave the DAW first;
//! a rescan afterwards says which ones are really gone.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use linux_vst_bridge::uninstall_entry::{self, Command as UninstallCommand};

const CHANGED: &str = "This changed since it was offered. Refresh and try again.";

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
    pub environment: Environment,
    pub registration: String,
    pub name: String,
    pub command_sha256: String,
    pub command: UninstallCommand,
    pub image: Artifact,
    pub affected: Vec<Affected>,
    pub created_at: u64,
}

fn record_path(m: &Manager, operation: &str) -> Result<PathBuf> { Ok(job_dir(m, operation)?.join("uninstall.json")) }
fn result_path(m: &Manager, operation: &str) -> Result<PathBuf> { Ok(job_dir(m, operation)?.join("uninstall-result.json")) }

pub(super) fn records(m: &Manager) -> Result<Vec<Record>> {
    let root = m.root.join("operator");
    if !root.try_exists()? { return Ok(vec![]); }
    let mut out = vec![];
    for (n, entry) in fs::read_dir(root)?.enumerate() {
        require(n < 4096, "uninstall_history_bound")?;
        let path = entry?.path();
        let Some(id) = path.file_name().and_then(|v| v.to_str()) else { continue };
        if valid_hex(id, 32) && path.join("uninstall.json").try_exists()? {
            let record: Record = read_json(&path.join("uninstall.json"))?;
            require(record.schema == 1 && record.operation == id
                && record.environment.root == m.root.join("environments").join(&record.environment.id),
                "uninstall_record_binding")?;
            out.push(record);
        }
    }
    out.sort_by_key(|record| record.created_at);
    Ok(out)
}
pub(super) fn result(m: &Manager, record: &Record) -> Result<Value> {
    let v = optional(&result_path(m, &record.operation)?)?;
    require(v["schema"] == 2 && v["operation"] == record.operation, "uninstall_result_identity")?;
    Ok(v)
}
/// Every uninstaller run has retired, exactly as installer runs must.
pub(crate) fn all_retired(m: &Manager) -> Result<bool> {
    for record in records(m)? {
        if !onboarding::retired(&result(m, &record)?) { return Ok(false); }
    }
    Ok(true)
}
/// Worker cleanup when its owner ended: stop a live unit, then record an
/// unretired run as unconfirmed (or failed when it never launched).
pub(super) fn finish(m: &Manager, operation: &str) -> Result<()> {
    if !record_path(m, operation)?.try_exists()? { return Ok(()); }
    let record: Record = read_json(&record_path(m, operation)?)?;
    if onboarding::live(operation)? {
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

/// Files each Uninstall registration's own installation added or changed, as
/// recorded by this setup's installer runs (initial and in-setup installers).
fn installed_by_registration(m: &Manager, record: &onboarding::Record)
    -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut transactions = vec![];
    if let Some(op) = &record.installation_operation {
        transactions.push(onboarding::directory(m, &record.id)?.join(format!("{op}-transaction-private.json")));
    }
    for installation in environment_install::records(m)?.into_iter().filter(|r| r.environment.id == record.id) {
        let op = &installation.operation;
        transactions.push(job_dir(m, op)?.join(format!("{op}-transaction-private.json")));
    }
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in transactions {
        if !path.try_exists()? { continue; }
        let Ok(value) = read_json::<Value>(&path) else { continue };
        let delta = &value["durable_installation"]["delta"];
        let list = |kind: &str| -> Vec<String> { ["added", "changed"].iter()
            .flat_map(|change| delta[kind][*change].as_array().cloned().unwrap_or_default())
            .filter_map(|v| v.as_str().map(str::to_owned)).collect() };
        let files: BTreeSet<String> = list("files").into_iter().map(|f| f.to_lowercase()).collect();
        for key in list("uninstall") {
            out.entry(key).or_default().extend(files.iter().cloned());
        }
    }
    Ok(out)
}

/// Plug-in modules of one setup environment: registry classes and the scan.
fn modules(m: &Manager, environment: &Environment, db: &Registry) -> Vec<Affected> {
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
pub(super) struct Choice {
    pub entry: uninstall_entry::Entry,
    pub command: UninstallCommand,
    pub affected: Vec<Affected>,
}
/// Registrations this setup's own installers added, each with the plug-ins
/// whose module files that same installation added.
pub(super) fn choices(m: &Manager, record: &onboarding::Record, db: &Registry) -> Result<Vec<Choice>> {
    let installed = installed_by_registration(m, record)?;
    if installed.is_empty() { return Ok(vec![]); }
    let environment = &record.environment;
    let modules = modules(m, environment, db);
    let mut out = vec![];
    for entry in uninstall_entry::read(environment)? {
        let Some(files) = installed.get(&entry.id) else { continue };
        let (Some(value), Some(_)) = (entry.uninstall_string.as_deref(), entry.display_name.as_deref()) else { continue };
        if entry.system_component { continue; }
        let Ok(command) = uninstall_entry::command(value, &drive_c(environment)) else { continue };
        let affected = modules.iter().filter(|module| relative_key(environment, &module.module)
            .is_some_and(|key| files.contains(&key))).cloned().collect();
        out.push(Choice { entry, command, affected });
    }
    Ok(out)
}

/// Uninstall choices for each settled setup card, plus "Uninstall in <app>"
/// for plug-ins a vendor app installed and manages.
pub(super) fn project(m: &Manager, setups: &[ui::InstallerSetup], records: &[onboarding::Record],
    db: &Registry, bindings: &[catalogue::EnvironmentBinding], busy: Option<&str>,
    vendor_applications: &[ui::VendorApplication]) -> Result<Vec<ui::Uninstaller>> {
    let protected = setup_removal::protected(m, bindings)?;
    let mut out = vec![];
    for setup in setups {
        let Some(environment) = setup.environment.as_deref() else { continue };
        let Some(record) = records.iter().find(|r| r.id == environment) else { continue };
        if protected.contains(environment) || setup.journey.as_ref().is_some_and(|journey| !matches!(journey.phase,
            ui::InstallationPhase::Available | ui::InstallationPhase::NeedsAttention
            | ui::InstallationPhase::AwaitingApplication)) { continue; }
        // One unreadable registry must not hide every other card.
        let Ok(found) = choices(m, record, db) else { continue };
        let covered: BTreeSet<_> = found.iter().flat_map(|c| c.affected.iter().map(|a| a.class_id.clone())).collect();
        for choice in found {
            let name = choice.entry.display_name.clone().unwrap_or_default();
            out.push(ui::Uninstaller { environment: environment.into(), name: name.clone(),
                affected: choice.affected.iter().map(|a| a.name.clone()).collect(),
                affected_classes: choice.affected.iter().map(|a| a.class_id.clone()).collect(),
                coverage: "installed_with_this".into(),
                action: action(&format!("Uninstall {name}"), ui::Action::ProductUninstall {
                    environment: environment.into(), registration: choice.entry.id.clone(),
                    command_sha256: uninstall_entry::command_sha256(choice.entry.uninstall_string.as_deref().unwrap_or_default()),
                    confirmed: false },
                    busy) });
        }
        // Plug-ins an app installed are uninstalled in that app.
        let managed: Vec<_> = modules(m, &record.environment, db).into_iter()
            .filter(|module| !covered.contains(&module.class_id)).collect();
        if managed.is_empty() { continue; }
        for app in vendor_applications.iter().filter(|app| app.details["environment"] == environment) {
            let Some(open) = app.actions.iter().find(|offer| matches!(offer.action, ui::Action::VendorApplicationOpen { .. })) else { continue };
            out.push(ui::Uninstaller { environment: environment.into(), name: app.name.clone(),
                affected: managed.iter().map(|a| a.name.clone()).collect(),
                affected_classes: managed.iter().map(|a| a.class_id.clone()).collect(),
                coverage: "vendor_app".into(),
                action: ui::AvailableAction { label: format!("Uninstall in {}", app.name), ..open.clone() } });
        }
    }
    Ok(out)
}

fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_micros().try_into()?)
}

/// Resolve the exact offered registration and command; nothing else may run.
pub(super) fn prepare(m: &Manager, environment: &str, registration: &str, command_sha256: &str,
    operation: &str) -> Result<Record> {
    let setup = onboarding::retained_environment_record(m, environment)?.ok_or(CHANGED)?;
    let sw = software_record(m)?;
    let catalogue = operator_catalogue_readback(m, &sw, &m.registry()?)?;
    let bindings = managed_environment_bindings(m, catalogue.as_ref(), &m.registry()?)?;
    require(!setup_removal::protected(m, &bindings)?.contains(environment),
        "This setup is managed by the product; uninstall in its app instead.")?;
    let choice = choices(m, &setup, &m.registry()?)?.into_iter()
        .find(|c| c.entry.id == registration).ok_or(CHANGED)?;
    let value = choice.entry.uninstall_string.as_deref().ok_or(CHANGED)?;
    require(uninstall_entry::command_sha256(value) == command_sha256, CHANGED)?;
    let environment = setup.environment.clone();
    let image = match &choice.command {
        UninstallCommand::Executable { relative, .. } => drive_c(&environment).join(relative),
        UninstallCommand::Msi { .. } => environment.runner.proton.parent().ok_or("uninstall_runner")?
            .join("files/lib/wine/x86_64-windows/msiexec.exe"),
    };
    let image = Artifact { sha256: digest(&image)?, path: image };
    Ok(Record { schema: 1, operation: operation.into(), environment, registration: registration.into(),
        name: choice.entry.display_name.clone().unwrap_or_default(), command_sha256: command_sha256.into(),
        command: choice.command, image, affected: choice.affected, created_at: now()? })
}

/// Retain the record, then start the supervised uninstaller. The caller holds
/// the environment lock and has suspended the service.
pub(super) fn launch(m: &Manager, record: &Record) -> Result<()> {
    let dir = job_dir(m, &record.operation)?;
    require(!record_path(m, &record.operation)?.try_exists()?, "uninstall_operation_already_used")?;
    let report = result_path(m, &record.operation)?;
    atomic_json(&report, &json!({"schema":2,"operation":record.operation,"state":"reserved",
        "cleanup_confirmed":false,"owned_live":null}))?;
    atomic_json(&record_path(m, &record.operation)?, record)?;
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

/// After the uninstaller retired and the setup was scanned again: which of its
/// plug-ins are really gone.
pub(super) fn outcome(record: &Record, removed_from_daw: &[String]) -> Value {
    let (gone, remaining): (Vec<_>, Vec<_>) = record.affected.iter()
        .partition(|a| !a.module.try_exists().unwrap_or(true));
    let names = |rows: &[&Affected]| rows.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
    let message = if remaining.is_empty() {
        format!("{} was uninstalled.", record.name)
    } else {
        format!("The uninstaller finished, but {}'s files are still there. {} stays removed from your DAW; add it back from Plug-ins if you still want it.",
            names(&remaining).join(", "), if remaining.len() == 1 { "It" } else { "They" })
    };
    json!({"uninstalled":names(&gone),"still_installed":names(&remaining),"removed_from_daw":removed_from_daw,
        "name":record.name,"message":message,"history_retained":true})
}

#[cfg(test)]
mod tests;

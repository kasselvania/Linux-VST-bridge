//! Musician-facing removal: take one plug-in out of the DAW and add it back,
//! take a whole setup's plug-ins out of the DAW, and delete a setup's Windows
//! environments. Publication, installer and vendor owners keep their own laws;
//! this module only chooses among their exact records and refuses whenever an
//! owner cannot be shown to have retired.
use super::*;
use std::fs::{File, OpenOptions};
use std::os::unix::{ffi::OsStrExt, fs::MetadataExt};
use linux_vst_bridge::deleted_setup::{self, RetiredRegistration, State as Deletion, Tombstone};
use linux_vst_bridge::publication::{self, RevisionRef};

pub(super) const REMOVED_FROM_DAW: &str =
    "Removed from your DAW. Projects that use it open with it missing until you add it back.";
const CHANGED: &str = "This changed since it was offered. Refresh and try again.";
const IN_USE: &str = "Something is still using this setup (an installer, an app or a plug-in). Close it and try again.";

fn loaded(name: &str) -> String {
    format!("{name} is still loaded in your DAW. Close it there (or close the project), then try again.")
}
/// The class-scoped refusal in plain words. Other refusals are shared text.
fn class_reason(reason: Option<&str>, name: &str) -> Option<String> {
    reason.map(|reason| if reason == CLASS_LOADED { loaded(name) } else { reason.to_owned() })
}
pub(super) fn removal_action(action: &ui::Action) -> bool {
    matches!(action, ui::Action::PluginRemoveFromDaw { .. } | ui::Action::PluginAddBackToDaw { .. }
        | ui::Action::SetupRemoveFromDaw { .. } | ui::Action::SetupDelete { .. })
}
/// Admission refusals for these actions name the loaded plug-in. Other
/// actions keep their existing refusal text.
pub(super) fn admission_refusal(m: &Manager, action: &ui::Action, reason: &str) -> String {
    let loaded_class = reason == CLASS_LOADED || reason == "active_device_lease";
    if !removal_action(action) || !loaded_class { return reason.into(); }
    match action {
        ui::Action::PluginRemoveFromDaw { class_id, .. } | ui::Action::PluginAddBackToDaw { class_id, .. } =>
            m.registry().ok().and_then(|db| db.classes.get(&class_id.to_uppercase())
                .map(|entry| loaded(&entry.registration.metadata.name))).unwrap_or_else(|| reason.into()),
        _ => "A plug-in from this setup is still loaded in your DAW. Close it there (or close the project), then try again.".into(),
    }
}
fn reference(identity: &ui::PublicationIdentity) -> RevisionRef {
    RevisionRef { id: identity.id.clone(), sha256: identity.sha256.clone() }
}
fn identity(reference: &RevisionRef) -> ui::PublicationIdentity {
    ui::PublicationIdentity { id: reference.id.clone(), sha256: reference.sha256.clone() }
}
fn now() -> Result<u64> {
    Ok(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_micros().try_into()?)
}
/// Publication owners take registry.lock without waiting. A short readback can
/// hold it; they refuse before any mutation, so the same call is retried.
fn retry_busy<T>(mut work: impl FnMut() -> Result<T>) -> Result<T> {
    let deadline = Instant::now() + OPERATOR_WAIT;
    loop {
        match work() {
            Err(error) if error.is::<linux_vst_bridge::operator_lock::LockBusy>()
                && Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            result => return result,
        }
    }
}
/// Owner refusals in the musician's words. Unknown reasons stay exact.
fn plain(name: &str, error: Box<dyn std::error::Error + Send + Sync>)
    -> Box<dyn std::error::Error + Send + Sync> {
    let text = error.to_string();
    match text.as_str() {
        "active_device_lease" => loaded(name).into(),
        "active_maintenance_lease" => "Wait for the current scan or installation to finish, then try again.".into(),
        "package_transition_needs_recovery" => "Finish the manager update first, then try again.".into(),
        "experimental_publication_changed" | "qualification_publication_changed"
            | "registration_absent" | "no_managed_revision" => CHANGED.into(),
        "foreign_publication" | "foreign_or_missing_publication" | "publication_already_absent" =>
            format!("{name}'s entry in your DAW's plug-in folder is not the one the manager made. Nothing was changed.").into(),
        _ if text.starts_with("bridge_update_required") =>
            format!("{name} was prepared by an older version of the manager. Use Check compatibility to prepare it again; that adds it back.").into(),
        _ if text.contains("artifact missing or changed") || text.contains("No such file") =>
            format!("{name}'s files changed since it was removed. Use Check compatibility to prepare it again.").into(),
        _ => error,
    }
}
fn environment_current(environment: &Environment) -> bool {
    read_json::<Environment>(&environment.root.join("environment.json"))
        .is_ok_and(|current| current == *environment)
}

/// "Remove from my DAW" for a consistent current publication; "Add back to my
/// DAW" for a retained publication the musician removed.
pub(super) fn plugin_offers(m: &Manager, class: &str, entry: &Entry, busy: Option<&str>)
    -> Vec<ui::AvailableAction> {
    let Some(current) = entry.managed_revision.as_ref() else { return vec![] };
    if !m.publication_pending(class).is_ok_and(|pending| !pending) { return vec![]; }
    let name = &entry.registration.metadata.name;
    let physical = publication::physical(&m.link(class));
    let revision = m.load_revision_record(class, current);
    match entry.publication {
        Publication::Published => {
            let Ok(revision) = revision else { return vec![] };
            if !matches!(&physical, Ok(Some(target)) if *target == revision.target) { return vec![]; }
            vec![action("Remove from my DAW", ui::Action::PluginRemoveFromDaw {
                class_id: class.into(), expected_current: identity(current) },
                class_reason(busy, name).as_deref())]
        }
        Publication::Removed => {
            if !matches!(physical, Ok(None)) { return vec![]; }
            let unavailable = match &revision {
                Err(_) => Some("Its prepared bridge files are no longer available. Use Check compatibility to prepare it again."),
                Ok(revision) if !environment_current(&revision.registration.environment) =>
                    Some("Its setup changed since it was removed. Use Check compatibility to prepare it again."),
                Ok(_) => None,
            };
            vec![action("Add back to my DAW", ui::Action::PluginAddBackToDaw {
                class_id: class.into(), expected_removed: identity(current) },
                unavailable.map(str::to_owned).or_else(|| class_reason(busy, name)).as_deref())]
        }
        Publication::Pending => vec![],
    }
}

/// A plug-in the musician removed reads as removed, not as a fault, and its
/// exact "Add back" leads. A removal recorded with a failed test keeps that
/// test's own presentation and is not offered back from here.
pub(super) fn present_removed(m: &Manager, product: &mut ui::Product, entry: &Entry) {
    let registration = &entry.registration;
    if entry.publication != Publication::Removed || entry.managed_revision.is_none()
        || !product.class_id.eq_ignore_ascii_case(&registration.metadata.class_id)
        || product.environment != registration.environment.id
        || product.module_sha256 != registration.module.sha256
        || !matches!(publication::physical(&m.link(&product.class_id)), Ok(None))
        || !m.publication_pending(&product.class_id).is_ok_and(|pending| !pending) {
        return;
    }
    let plain_removal = product.compatibility.as_ref().is_none_or(|workflow|
        workflow.phase == ui::CompatibilityPhase::ReadyForTest);
    if !plain_removal {
        product.actions.retain(|offer| !matches!(offer.action, ui::Action::PluginAddBackToDaw { .. }));
        return;
    }
    product.disposition = "removed_from_daw".into();
    product.availability = Some(ui::NativeAvailability { available: false,
        reason: Some(REMOVED_FROM_DAW.into()) });
    let add_back = product.actions.iter().find(|offer|
        matches!(offer.action, ui::Action::PluginAddBackToDaw { .. })).cloned();
    if let (Some(workflow), Some(offer)) = (product.compatibility.as_mut(), add_back) {
        if offer.disabled_reason.is_none() {
            workflow.summary = REMOVED_FROM_DAW.into();
            if workflow.primary.as_ref().is_none_or(|primary| matches!(primary.action,
                ui::Action::CompatibilityPublishTest { expected_current: None, .. }
                | ui::Action::ExperimentalEnable { .. })) {
                workflow.primary = Some(offer);
            }
        }
    }
}

pub(super) fn remove_from_daw(m: &Manager, class: &str, expected: &ui::PublicationIdentity)
    -> Result<Value> {
    require(valid_hex(class, 32), "operator_product_identity")?;
    let key = class.to_uppercase();
    let expected_ref = reference(expected);
    let entry = m.registry()?.classes.get(&key).cloned().ok_or(CHANGED)?;
    let name = entry.registration.metadata.name.clone();
    let outcome = |already: bool| json!({"class_id":key,"name":name,"removed_from_daw":true,
        "already_removed":already,"publication":expected,"add_back_available":true,
        "projects":"open_with_plugin_missing_until_added_back"});
    if entry.publication == Publication::Removed && entry.managed_revision.as_ref() == Some(&expected_ref)
        && publication::physical(&m.link(&key))?.is_none() {
        return Ok(outcome(true));
    }
    require(entry.publication == Publication::Published
        && entry.managed_revision.as_ref() == Some(&expected_ref), CHANGED)?;
    retry_busy(|| m.unpublish_exact(&key, &expected_ref)).map_err(|error| plain(&name, error))?;
    Ok(outcome(false))
}

pub(super) fn add_back(m: &Manager, class: &str, expected: &ui::PublicationIdentity) -> Result<Value> {
    require(valid_hex(class, 32), "operator_product_identity")?;
    let key = class.to_uppercase();
    let expected_ref = reference(expected);
    let entry = m.registry()?.classes.get(&key).cloned().ok_or(CHANGED)?;
    let name = entry.registration.metadata.name.clone();
    let outcome = |already: bool| json!({"class_id":key,"name":name,"added_back_to_daw":true,
        "already_available":already,"publication":expected,
        "next":"Rescan VST3 plug-ins in your DAW if it does not appear."});
    if entry.publication == Publication::Published && entry.managed_revision.as_ref() == Some(&expected_ref) {
        let revision = m.load_revision(&key, &expected_ref)?;
        if publication::physical(&m.link(&key))? == Some(revision.target) { return Ok(outcome(true)); }
    }
    require(entry.publication == Publication::Removed
        && entry.managed_revision.as_ref() == Some(&expected_ref), CHANGED)?;
    let _package = retry_busy(|| m.lock("package.lock"))
        .map_err(|_| "The manager is updating. Try again when the update finishes.")?;
    // The exact retained revision only: its runner, Windows module, host and
    // native bytes are verified again by the publication owner.
    retry_busy(|| m.rollback_managed_expected(&key, &expected_ref.id, &expected_ref))
        .map_err(|error| plain(&name, error))?;
    Ok(outcome(false))
}

/// Records of one installer setup, as the setup card shows it.
pub(super) struct SetupPlan {
    pub name: String,
    pub environments: Vec<Environment>,
    /// (class, name, current publication) of plug-ins currently in the DAW.
    pub in_daw: Vec<(String, String, RevisionRef)>,
    pub removed: Vec<RetiredRegistration>,
    pub plugins: Vec<String>,
    /// The first reason the setup cannot be deleted now, in plain words.
    pub blockers: Vec<String>,
}

/// Environments the manager pins for a fixture, workspace or shared renderer.
/// They are not installer setups and are never offered for deletion.
fn protected(m: &Manager, bindings: &[catalogue::EnvironmentBinding]) -> Result<std::collections::BTreeSet<String>> {
    let mut ids: std::collections::BTreeSet<String> = bindings.iter()
        .filter(|binding| binding.family != linux_vst_bridge::profiles::Family::ManagedInstallerV1)
        .map(|binding| binding.environment.id.clone()).collect();
    ids.insert(renderer::ENVIRONMENT.into());
    if let Some(adopted) = linux_vst_bridge::frg1::adopted_environment_record(m)? {
        ids.insert(adopted.environment.id);
    }
    Ok(ids)
}

fn setup_name(m: &Manager, installer: &str) -> Result<String> {
    Ok(installer_import::list_identity_records(m)?.into_iter().find(|record| record.id == installer)
        .map(|record| installer_import::presentation(m, &record).map(|p| p.display_label))
        .transpose()?.unwrap_or_else(|| "this setup".into()))
}

fn inventory_names(m: &Manager, environment: &str) -> Vec<String> {
    let path = m.root.join("inventory").join(format!("{environment}.json"));
    let Ok(scan) = read_json::<inventory::Scan>(&path) else { return vec![] };
    scan.modules.into_iter().flat_map(|module| module.classes)
        .filter(|class| class.category == "Audio Module Class").map(|class| class.name).collect()
}

pub(super) fn plan(m: &Manager, installer: &str, environment: &str, records: &[onboarding::Record],
    db: &Registry, is_live: &mut dyn FnMut(&str) -> Result<bool>) -> Result<SetupPlan> {
    let attempts: Vec<_> = records.iter().filter(|record| record.installer == installer).collect();
    require(attempts.iter().any(|record| record.id == environment), CHANGED)?;
    let ids: std::collections::BTreeSet<_> = attempts.iter().map(|record| record.id.clone()).collect();
    let mut plan = SetupPlan { name: setup_name(m, installer)?,
        environments: attempts.iter().map(|record| record.environment.clone()).collect(),
        in_daw: vec![], removed: vec![], plugins: vec![], blockers: vec![] };
    for (class, entry) in db.classes.iter().filter(|(_, entry)| ids.contains(&entry.registration.environment.id)) {
        let name = entry.registration.metadata.name.clone();
        plan.plugins.push(name.clone());
        let pending = m.publication_pending(class)?;
        let physical = publication::physical(&m.link(class));
        match (&entry.publication, &entry.managed_revision) {
            (Publication::Published, Some(current)) if !pending => {
                plan.in_daw.push((class.clone(), name, current.clone()));
            }
            (Publication::Removed, Some(_)) if !pending && matches!(physical, Ok(None)) => {
                plan.removed.push(RetiredRegistration { class_id: class.clone(), name, entry: entry.clone() });
            }
            _ => plan.blockers.push(format!("{name}'s DAW connection needs attention first. Use Reconcile interrupted transaction in Diagnostics, then try again.")),
        }
    }
    for id in &ids {
        for name in inventory_names(m, id) {
            if !plan.plugins.contains(&name) { plan.plugins.push(name); }
        }
    }
    for record in &attempts {
        let Some(operation) = &record.installation_operation else { continue };
        if is_live(operation)? {
            plan.blockers.push("An installer is still running in this setup. Finish or stop it first.".into());
        } else if !onboarding::retired(&onboarding::result(m, record)?) {
            plan.blockers.push("The manager can't confirm that this setup's installer closed completely, so it won't delete the setup. Send Setup details if this stays.".into());
        }
    }
    for installation in environment_install::records(m)?.into_iter().filter(|r| ids.contains(&r.environment.id)) {
        if is_live(&installation.operation)? {
            plan.blockers.push("An installer is still running in this setup. Finish or stop it first.".into());
        } else if !onboarding::retired(&environment_install::result(m, &installation)?) {
            plan.blockers.push("The manager can't confirm that an installer in this setup closed completely, so it won't delete the setup. Send Setup details if this stays.".into());
        }
    }
    for selector in vendor_cli::selectors(m)? {
        let directory = vendor_cli::directory(m, &selector)?;
        let Ok(app) = read_json::<linux_vst_bridge::vendor_application::Application>(
            &directory.join("application.json")) else { continue };
        if !ids.contains(&app.environment.id) { continue; }
        let app_name = app.display_name();
        if vendor_cli::live(&selector)? {
            plan.blockers.push(format!("{app_name} is still open. Close it first."));
            continue;
        }
        if directory.join("operation.json").try_exists()? {
            let job: linux_vst_bridge::vendor_application::Operation = read_json(&directory.join("operation.json"))?;
            if !vendor_cli::result(m, &selector, &job.operation_id)?.retired() {
                plan.blockers.push(format!("{app_name} didn't finish closing. Use Recover on its card first."));
            }
        }
    }
    for id in &ids {
        if environment_runtime::pending(m, id)? {
            plan.blockers.push("Finish the interrupted runtime change in this setup first.".into());
        }
    }
    plan.blockers.dedup();
    Ok(plan)
}

/// Every registry class bound to the setup that contains `environment`.
pub(super) fn setup_classes(m: &Manager, environment: &str) -> Result<Vec<String>> {
    let records = onboarding::history_records(m)?;
    let installer = records.iter().find(|record| record.id == environment).ok_or(CHANGED)?.installer.clone();
    let ids: std::collections::BTreeSet<_> = records.iter().filter(|record| record.installer == installer)
        .map(|record| record.id.clone()).collect();
    Ok(m.registry()?.classes.into_iter().filter(|(_, entry)| ids.contains(&entry.registration.environment.id))
        .map(|(class, _)| class).collect())
}

fn delete_reason(plan: &SetupPlan, busy: Option<&str>) -> Option<String> {
    if !plan.in_daw.is_empty() {
        let names: Vec<_> = plan.in_daw.iter().map(|(_, name, _)| name.as_str()).collect();
        return Some(format!("Remove {} from your DAW first.", names.join(", ")));
    }
    plan.blockers.first().cloned().or_else(|| busy.map(str::to_owned))
}

/// Add each settled setup's removal choices to its card, and retained
/// deleted-setup history. `scope` is the class-scoped inactivity reason.
pub(super) fn project(m: &Manager, setups: &mut [ui::InstallerSetup], records: &[onboarding::Record],
    db: &Registry, bindings: &[catalogue::EnvironmentBinding],
    scope: &dyn Fn(&str) -> Option<&'static str>,
    is_live: &mut dyn FnMut(&str) -> Result<bool>) -> Result<Vec<ui::DeletedSetup>> {
    let protected = protected(m, bindings)?;
    for setup in setups.iter_mut() {
        let Some(environment) = setup.environment.clone() else { continue };
        if protected.contains(&environment) || !records.iter().any(|record| record.id == environment) { continue; }
        // A running installation journey finishes (or stops) first.
        if setup.journey.as_ref().is_some_and(|journey| !matches!(journey.phase,
            ui::InstallationPhase::Available | ui::InstallationPhase::NeedsAttention
            | ui::InstallationPhase::AwaitingApplication)) { continue; }
        let plan = plan(m, &setup.installer, &environment, records, db, is_live)?;
        if !plan.in_daw.is_empty() {
            let reason = plan.in_daw.iter().find_map(|(class, name, _)| class_reason(scope(class), name));
            setup.secondary.push(action("Remove its plug-ins from my DAW",
                ui::Action::SetupRemoveFromDaw { environment: environment.clone() }, reason.as_deref()));
        }
        setup.secondary.push(action("Delete this setup", ui::Action::SetupDelete {
            installer: setup.installer.clone(), environment }, delete_reason(&plan, scope("")).as_deref()));
    }
    deleted_setup::records(m)?.into_iter().map(|record| {
        let interrupted = record.state == Deletion::Deleting;
        Ok(ui::DeletedSetup { installer: record.installer.clone(), environment: record.environment.clone(),
            name: record.name, state: if interrupted { "deleting" } else { "deleted" }.into(),
            deleted_at: record.deleted_at, plugins: record.plugins,
            actions: if interrupted { vec![action("Finish deleting this setup", ui::Action::SetupDelete {
                installer: record.installer, environment: record.environment }, scope(""))] } else { vec![] } })
    }).collect()
}

pub(super) fn remove_setup_from_daw(m: &Manager, environment: &str) -> Result<Value> {
    let records = onboarding::history_records(m)?;
    let installer = records.iter().find(|record| record.id == environment).ok_or(CHANGED)?.installer.clone();
    let plan = plan(m, &installer, environment, &records, &m.registry()?, &mut |_| Ok(false))?;
    require(!plan.in_daw.is_empty(), "Nothing from this setup is in your DAW.")?;
    let mut removed = vec![];
    for (class, name, current) in &plan.in_daw {
        if let Err(error) = retry_busy(|| m.unpublish_exact(class, current)) {
            let reason = plain(name, error).to_string();
            return Err(if removed.is_empty() { reason }
                else { format!("Removed {} from your DAW. {reason}", removed.join(", ")) }.into());
        }
        removed.push(name.clone());
    }
    Ok(json!({"setup":plan.name,"removed_from_daw":removed,"add_back_available":true,
        "projects":"open_with_plugins_missing_until_added_back"}))
}

/// The environment directory itself: exact location, owned, never a link.
fn checked_root(m: &Manager, environment: &Environment) -> Result<PathBuf> {
    require(linux_vst_bridge::environment_revision::valid_environment_id(&environment.id)
        && environment.root == m.root.join("environments").join(&environment.id), "setup_environment_identity")?;
    let parent = fs::symlink_metadata(m.root.join("environments"))?;
    let meta = match fs::symlink_metadata(&environment.root) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound =>
            return Err("This setup's folder can't be found. Reconnect its drive or restore the folder, then try again.".into()),
        Err(error) => return Err(error.into()),
    };
    require(parent.is_dir() && meta.is_dir() && !meta.file_type().is_symlink()
        && meta.uid() == unsafe { libc::getuid() }
        && environment.root.canonicalize()? == m.root.join("environments").canonicalize()?.join(&environment.id),
        "setup_environment_identity")?;
    Ok(environment.root.canonicalize()?)
}
/// The supervisor, installers, vendor apps, keepers and plug-in sessions all
/// hold this lock while they use the environment. Holding it exclusively
/// proves none of them is running there now.
fn hold(m: &Manager, environment: &Environment) -> Result<File> {
    let root = checked_root(m, environment)?;
    let lock = OpenOptions::new().read(true).write(true).create(true).mode(0o600)
        .custom_flags(libc::O_NOFOLLOW).open(root.join("operation.lock"))?;
    use std::os::fd::AsRawFd;
    require(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0, IN_USE)?;
    Ok(lock)
}
/// A session directory that was never retired, or a lease owned by this
/// environment, is uncertain custody. Refuse rather than delete under it.
fn require_retired_sessions(m: &Manager, environment: &Environment) -> Result<()> {
    let sessions = environment.root.join("compatdata/pfx/drive_c/bridge/sessions");
    if sessions.try_exists()? {
        require(fs::read_dir(&sessions)?.next().is_none(),
            "A plug-in session in this setup never finished closing. Restart the computer, then try again; if it stays, send Setup details.")?;
    }
    let leases = m.root.join("runtime/leases");
    if leases.try_exists()? {
        for entry in fs::read_dir(&leases)?.take(65) {
            let lease = entry?.path();
            let Some(sid) = lease.file_stem().and_then(|s| s.to_str()) else { continue };
            let report: PathBuf = read_json(&lease)?;
            let (owner, _) = m.lease_owner(sid, &report)?;
            require(owner["registration"]["environment"]["id"].as_str() != Some(environment.id.as_str()), IN_USE)?;
        }
    }
    Ok(())
}
/// Processes of this user whose Wine prefix, working directory or executable
/// lies inside the environment. A bounded /proc census; other users' processes
/// are not readable and cannot use this private environment.
fn running_inside(root: &Path) -> Result<Vec<String>> {
    let proc = Path::new("/proc");
    if !proc.is_dir() { return Ok(vec![]); }
    let me = unsafe { libc::getuid() };
    let mut found = vec![];
    for entry in fs::read_dir(proc)?.take(1 << 16) {
        let Ok(entry) = entry else { continue };
        let pid = entry.file_name();
        if !pid.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) { continue; }
        let dir = entry.path();
        if fs::metadata(&dir).map(|meta| meta.uid()).ok() != Some(me) { continue; }
        let mut inside = ["cwd", "exe"].iter().any(|link|
            fs::read_link(dir.join(link)).is_ok_and(|target| target.starts_with(root)));
        if !inside {
            if let Ok(file) = File::open(dir.join("environ")) {
                let mut bytes = vec![];
                if file.take(1 << 20).read_to_end(&mut bytes).is_ok() {
                    inside = bytes.split(|b| *b == 0).any(|variable| {
                        ["WINEPREFIX=", "STEAM_COMPAT_DATA_PATH="].iter().any(|key|
                            variable.strip_prefix(key.as_bytes()).is_some_and(|value|
                                Path::new(std::ffi::OsStr::from_bytes(value)).starts_with(root)))
                    });
                }
            }
        }
        if inside {
            found.push(fs::read_to_string(dir.join("comm")).map(|name| name.trim().to_owned())
                .unwrap_or_else(|_| "a program".into()));
        }
    }
    Ok(found)
}
fn require_nothing_running(root: &Path) -> Result<()> {
    let running = running_inside(root)?;
    require(running.is_empty(), &format!("{} is still running from this setup. Close it and try again.",
        running.first().map(String::as_str).unwrap_or("A program")))
}
/// Delete everything in the environment except its identity record and lock.
/// Links are removed, never followed; read-only folders are made writable.
fn remove_windows_files(m: &Manager, environment: &Environment) -> Result<()> {
    let root = checked_root(m, environment)?;
    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "environment.json" || name == "operation.lock" { continue; }
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)?;
        if !meta.is_dir() { fs::remove_file(&path)?; continue; }
        if let Err(first) = fs::remove_dir_all(&path) {
            if first.kind() != std::io::ErrorKind::PermissionDenied { return Err(first.into()); }
            make_writable(&path)?;
            fs::remove_dir_all(&path)?;
        }
    }
    File::open(&root)?.sync_all()?;
    Ok(())
}
fn make_writable(top: &Path) -> Result<()> {
    let me = unsafe { libc::getuid() };
    let mut stack = vec![top.to_path_buf()];
    let mut visited = 0usize;
    while let Some(dir) = stack.pop() {
        visited += 1;
        require(visited <= 4_000_000, "setup_deletion_bound")?;
        let meta = fs::symlink_metadata(&dir)?;
        if !meta.is_dir() || meta.uid() != me { continue; }
        if meta.mode() & 0o700 != 0o700 {
            fs::set_permissions(&dir, fs::Permissions::from_mode(meta.mode() | 0o700))?;
        }
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() { stack.push(entry.path()); }
        }
    }
    Ok(())
}
fn retain_inventory(m: &Manager, primary: &str, environment: &str) -> Result<()> {
    let source = m.root.join("inventory").join(format!("{environment}.json"));
    if !source.try_exists()? { return Ok(()); }
    let history = deleted_setup::history_directory(m, primary)?;
    private_dir(&history)?;
    let target = history.join(format!("inventory-{environment}.json"));
    require(!target.try_exists()?, "deleted_setup_inventory_conflict")?;
    fs::rename(&source, &target)?;
    File::open(&history)?.sync_all()?;
    File::open(m.root.join("inventory"))?.sync_all()?;
    Ok(())
}
fn summary(record: &Tombstone, already: bool) -> Value {
    json!({"setup":record.name,"setup_deleted":record.state == Deletion::Deleted,"already_deleted":already,
        "environments":record.environments.len(),"plugins":record.plugins,
        "history_retained":true,"installer_kept":true,"projects_changed":false})
}

pub(super) fn delete_setup(m: &Manager, installer: &str, environment: &str, operation: &str) -> Result<Value> {
    delete_setup_with(m, installer, environment, operation, &mut |op| onboarding::live(op))
}
fn delete_setup_with(m: &Manager, installer: &str, environment: &str, operation: &str,
    is_live: &mut dyn FnMut(&str) -> Result<bool>) -> Result<Value> {
    if let Some(record) = deleted_setup::load(m, environment)? {
        require(record.installer == installer, CHANGED)?;
        if record.state == Deletion::Deleted { return Ok(summary(&record, true)); }
        let held = record.environments.iter().map(|e| hold(m, e)).collect::<Result<Vec<_>>>()?;
        return finish(m, record, held);
    }
    let _ownership = retry_busy(|| m.lock("onboarding.lock"))?;
    let records = onboarding::history_records(m)?;
    let current = records.iter().find(|record| record.id == environment).ok_or(CHANGED)?;
    require(current.installer == installer, CHANGED)?;
    let sw = software_record(m)?;
    let catalogue = operator_catalogue_readback(m, &sw, &m.registry()?)?;
    let bindings = managed_environment_bindings(m, catalogue.as_ref(), &m.registry()?)?;
    require(!protected(m, &bindings)?.contains(environment), "This setup is managed by the product and can't be deleted.")?;
    let plan = plan(m, installer, environment, &records, &m.registry()?, is_live)?;
    if let Some(reason) = delete_reason(&plan, None) { return Err(reason.into()); }
    // Exclusive environment locks, then retained sessions and a process census.
    let held = plan.environments.iter().map(|e| hold(m, e)).collect::<Result<Vec<_>>>()?;
    for e in &plan.environments {
        require_retired_sessions(m, e)?;
        require_nothing_running(&checked_root(m, e)?)?;
    }
    let record = Tombstone { schema: 1, environment: environment.into(), installer: installer.into(),
        name: plan.name.clone(), environments: plan.environments.clone(), registrations: plan.removed.clone(),
        plugins: plan.plugins.clone(), operation: operation.into(), requested_at: now()?,
        deleted_at: None, state: Deletion::Deleting };
    {
        let registry = retry_busy(|| m.lock("registry.lock"))?;
        // Same exact registry facts under the lock that admits publications.
        let recheck = plan_registry(m, &plan)?;
        require(recheck, CHANGED)?;
        // History first: once marked, every reader treats these as history.
        deleted_setup::write(m, &record)?;
        m.retire_removed_registrations(&registry, &record.registrations)?;
    }
    finish(m, record, held)
}
fn plan_registry(m: &Manager, plan: &SetupPlan) -> Result<bool> {
    let ids: std::collections::BTreeSet<_> = plan.environments.iter().map(|e| e.id.clone()).collect();
    let db = m.registry()?;
    let bound: Vec<_> = db.classes.iter().filter(|(_, e)| ids.contains(&e.registration.environment.id)).collect();
    Ok(bound.len() == plan.removed.len() && plan.removed.iter().all(|retired|
        db.classes.get(&retired.class_id) == Some(&retired.entry)
            && !m.publication_pending(&retired.class_id).unwrap_or(true)
            && publication::physical(&m.link(&retired.class_id)).is_ok_and(|p| p.is_none())))
}
/// Continue from the retained tombstone while holding every environment lock.
fn finish(m: &Manager, mut record: Tombstone, held: Vec<File>) -> Result<Value> {
    require(held.len() == record.environments.len(), "setup_environment_lock")?;
    {
        let registry = retry_busy(|| m.lock("registry.lock"))?;
        m.retire_removed_registrations(&registry, &record.registrations)?;
    }
    for environment in record.environments.clone() {
        require_retired_sessions(m, &environment)?;
        require_nothing_running(&checked_root(m, &environment)?)?;
        retain_inventory(m, &record.environment, &environment.id)?;
        remove_windows_files(m, &environment)?;
    }
    record.state = Deletion::Deleted;
    record.deleted_at = Some(now()?);
    deleted_setup::write(m, &record)?;
    drop(held);
    Ok(summary(&record, false))
}

#[cfg(test)]
mod tests;

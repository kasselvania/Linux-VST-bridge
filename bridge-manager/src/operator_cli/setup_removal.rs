//! Musician-facing removal: take one plug-in out of the DAW and add it back,
//! take a whole setup's plug-ins out of the DAW, and delete a setup's Windows
//! environments. Publication, installer and vendor owners keep their own laws;
//! this module only chooses among their exact records and refuses whenever an
//! owner cannot be shown to have retired.
use super::*;
use std::ffi::{CStr, CString};
use std::fs::{File, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::{ffi::{OsStrExt, OsStringExt}, fs::MetadataExt};
use linux_vst_bridge::deleted_setup::{self, RetiredRegistration, State as Deletion, Tombstone};
use linux_vst_bridge::publication::{self, RevisionRef};

pub(super) const UNINSTALLED: &str =
    "Uninstalled. Its history is kept; install it again to use it.";
pub(super) const REMOVED_FROM_DAW: &str =
    "Removed from your DAW. Projects that use it open with it missing until you add it back.";
const CHANGED: &str = "This changed since it was offered. Refresh and try again.";
const IN_USE: &str = "Something is still using this setup (an installer, an app or a plug-in). Close it and try again.";
const KEEPER_CLOSING: &str = "The bridge is still closing this setup. Try again in a minute.";
/// An idle bridge host retires about 90 seconds after its last plug-in closed.
const KEEPER_WAIT: Duration = Duration::from_secs(100);
const MANAGED: &str = "This setup belongs to the bridge itself, not to software you installed, so it can't be deleted here.";
const SETUP_MOVED: &str = "This setup's folder isn't where the manager put it, or it was replaced by a link, so the manager won't delete it. Send Setup details.";
const TOO_DEEP: &str = "This setup has folders nested too deeply to delete safely. Nothing outside it was touched. Send Setup details.";
const NOT_OURS: &str = "A folder in this setup belongs to another user, so the manager won't delete it. Send Setup details.";
const UNREADABLE_APP: &str = "The manager can't read the record of an app it set up, so it won't delete a setup now. Send Setup details.";
const TOO_MANY_SESSIONS: &str = "More plug-in sessions are open than the manager can check. Close your DAW, then try again.";
const UNCHECKED_SESSION: &str = "A plug-in session record can't be checked, so the manager won't delete this setup now. Close your DAW and try again; if it stays, send Setup details.";
const RECONNECT: &str = "This setup's folder can't be found. Reconnect its drive or restore the folder, then try again.";
pub(super) const UNCONFIRMED: &str = "This needs the musician's confirmation (\"confirmed\": true). Nothing was changed.";
/// A removed plug-in whose setup changed in a way a re-check in place can't
/// follow (its runtime or the bridge changed too). Whether Check
/// compatibility on its card restores it is not yet verified (#396), so this
/// names what does.
const SETUP_CHANGED_SINCE: &str = "Its setup changed since it was removed, so it can't be added back as it was. Run its installer again in this setup, or delete the setup and set it up again.";
/// The installed bridge's files couldn't be read to compare: a passing state.
const BRIDGE_UNREAD: &str = "The manager couldn't read the bridge's files just now. Try again in a moment.";
fn link_was_missing(name: &str) -> String {
    format!("{name}'s entry in your DAW's plug-in folder was already gone, so the manager now records it as removed from your DAW.")
}
fn attention(name: &str) -> String {
    format!("{name}'s DAW connection needs attention first. Use Reconcile interrupted transaction in Diagnostics, then try again.")
}

/// The shared-service refusal while any bridged plug-in is open.
const ANY_LOADED: &str = "Close active bridged instances before this action";
fn recheck_first(name: &str) -> String {
    format!("Close the plug-ins open in your DAW first. Adding {name} back checks it again with the updated bridge.")
}
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
        "publication_link_present" => CHANGED.into(),
        "publication_recovery_pending" => attention(name).into(),
        "foreign_publication" | "foreign_or_missing_publication" | "publication_already_absent" =>
            format!("{name}'s entry in your DAW's plug-in folder is not the one the manager made. Nothing was changed.").into(),
        _ if text.starts_with("bridge_update_required") =>
            format!("{name} was prepared by an older version of the manager. Use Check compatibility to prepare it again; that adds it back.").into(),
        _ => error,
    }
}
/// Add back verifies the retained files again; a missing or changed one means
/// it has to be prepared again.
fn plain_add_back(name: &str, error: Box<dyn std::error::Error + Send + Sync>)
    -> Box<dyn std::error::Error + Send + Sync> {
    let text = error.to_string();
    if text.contains("artifact missing or changed") || text.contains("No such file") {
        format!("{name}'s files changed since it was removed. Use Check compatibility to prepare it again.").into()
    } else { plain(name, error) }
}
/// Deletion refusals that are not already plain sentences.
fn plain_deletion(error: Box<dyn std::error::Error + Send + Sync>) -> Box<dyn std::error::Error + Send + Sync> {
    match error.to_string().as_str() {
        "deleted_setup_inventory_conflict" =>
            "This setup's scan history was already recorded for a deleted setup, so the manager stopped. Send Setup details.".into(),
        "setup_deletion_bound" =>
            "This setup has more files than the manager deletes in one go. Use Finish deleting this setup to continue.".into(),
        "setup_environment_lock" =>
            "The manager couldn't hold every part of this setup at once. Try again; if it stays, send Setup details.".into(),
        "deleted_setup_registration_changed" =>
            "A plug-in of this setup changed while it was being deleted, so the manager stopped. Refresh and try again; if it stays, send Setup details.".into(),
        "deleted_setup_record_binding" | "deleted_setup_folder_identity" =>
            "This setup's deletion record can't be read, so the manager stopped. Send Setup details.".into(),
        "lease_identity" | "active_lease_unresolved" | "duplicate_lease_identity" => UNCHECKED_SESSION.into(),
        _ => error,
    }
}
/// The musician's own removal of this exact publication. A removal by a
/// failed test or another owner has no marker and keeps its own presentation.
fn removal_marker(m: &Manager, class: &str) -> PathBuf {
    m.root.join("daw-removals").join(format!("{}.json", class.to_uppercase()))
}
fn mark_removed(m: &Manager, class: &str, publication: &RevisionRef) -> Result<()> {
    private_dir(&m.root.join("daw-removals"))?;
    atomic_json(&removal_marker(m, class), publication)
}
fn clear_removed(m: &Manager, class: &str) {
    // Only presentation depends on it; a stale marker never matches a new publication.
    let _ = fs::remove_file(removal_marker(m, class));
}
pub(super) fn removed_by_musician(m: &Manager, class: &str, publication: &RevisionRef) -> bool {
    valid_hex(class, 32) && read_json::<RevisionRef>(&removal_marker(m, class))
        .is_ok_and(|marked| marked == *publication)
}
/// A file that is certainly gone. A file that can't be checked counts as
/// still there, so nothing is called uninstalled on an error.
pub(super) fn gone(path: &Path) -> bool {
    matches!(path.try_exists(), Ok(false))
}
/// The plug-in's setup has a newer, owned revision (an uninstaller or
/// installer ran in it) while its own file, the setup's runtime and the
/// bridge that scanned it are unchanged: Add back checks the same file and
/// class again in the setup as it is now. After a runtime change or a bridge
/// update it can't be checked in place; Add back then says what to do.
pub(super) fn setup_changed(m: &Manager, class: &str, publication: &RevisionRef) -> Result<bool> {
    let revision = m.load_revision_record(class, publication)?;
    let prior = &revision.registration;
    let current = linux_vst_bridge::environment_revision::current_environment(m, &prior.environment)?;
    if current == prior.environment || current.runner != prior.environment.runner
        || current.root != prior.environment.root || prior.module.verify().is_err()
        || !prepared_by_current_bridge(m, prior).map_err(|_| BRIDGE_UNREAD)? {
        return Ok(false);
    }
    // The re-check reads the setup's own scan, taken with the bridge it was prepared with.
    let scan = read_json::<inventory::Scan>(&m.root.join("inventory").join(format!("{}.json", current.id)));
    Ok(scan.is_ok_and(|scan| scan.environment == current && scan.host.sha256 == prior.host.sha256
        && scan.host_source_sha256 == prior.host_source_sha256))
}
/// The bridge that prepared this registration is the one installed now: a
/// re-check in place reuses its Windows host, so it can't follow an update.
/// An installed bridge not staged yet is another bridge; a read error is an
/// error, never taken as an answer.
fn prepared_by_current_bridge(m: &Manager, prior: &linux_vst_bridge::Registration) -> Result<bool> {
    let kit = linux_vst_bridge::preparation::build::recipe_available(m)?;
    match linux_vst_bridge::preparation::build::existing_runtime_record(m, &kit.sha256) {
        Ok(runtime) => Ok(runtime.host == prior.host && runtime.source_manifest.sha256 == prior.host_source_sha256),
        Err(error) if error.downcast_ref::<std::io::Error>()
            .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) => Ok(false),
        Err(error) => Err(error),
    }
}
fn environment_current(environment: &Environment) -> bool {
    read_json::<Environment>(&environment.root.join("environment.json"))
        .is_ok_and(|current| current == *environment)
}

/// "Remove from my DAW" for a consistent current publication; "Add back to my
/// DAW" for a retained publication the musician removed. `busy` is the class
/// refusal; `shared` the whole-service one, which applies when adding back
/// needs a fresh check with an updated bridge.
pub(super) fn plugin_offers(m: &Manager, class: &str, entry: &Entry, busy: Option<&str>,
    shared: Option<&str>) -> Vec<ui::AvailableAction> {
    let Some(current) = entry.managed_revision.as_ref() else { return vec![] };
    if !m.publication_pending(class).is_ok_and(|pending| !pending) { return vec![]; }
    let name = &entry.registration.metadata.name;
    let physical = publication::physical(&m.link(class));
    let revision = m.load_revision_record(class, current);
    match entry.publication {
        Publication::Published => {
            let Ok(revision) = revision else { return vec![] };
            // Its DAW entry is the manager's own, or was deleted by hand.
            if !matches!(&physical, Ok(Some(target)) if *target == revision.target)
                && !matches!(physical, Ok(None)) { return vec![]; }
            vec![action("Remove from my DAW", ui::Action::PluginRemoveFromDaw {
                class_id: class.into(), expected_current: identity(current) },
                class_reason(busy, name).as_deref())]
        }
        Publication::Removed => {
            // Gone from disk (uninstalled): nothing to add back.
            if !matches!(physical, Ok(None)) || !removed_by_musician(m, class, current)
                || gone(&entry.registration.module.path) { return vec![]; }
            let changed = revision.as_ref().is_ok_and(|r| !environment_current(&r.registration.environment));
            let checked = changed.then(|| setup_changed(m, class, current));
            let revalidates = matches!(checked, Some(Ok(true)));
            let unread = matches!(&checked, Some(Err(error)) if error.to_string() == BRIDGE_UNREAD);
            let unavailable = match &revision {
                Err(_) => Some("Its prepared bridge files are no longer available. Use Check compatibility to prepare it again."),
                Ok(_) if unread => Some(BRIDGE_UNREAD),
                Ok(_) if changed && !revalidates =>
                    Some(SETUP_CHANGED_SINCE),
                Ok(_) => None,
            };
            // After a bridge update, or a change to its setup, the same file
            // and class are checked again as part of Add back; that check
            // needs the shared service idle.
            let recheck = unavailable.is_none() && (revalidates
                || package_authority::add_back_needs_refresh(m, class, current).unwrap_or(false));
            let reason = unavailable.map(str::to_owned).or_else(|| class_reason(busy, name)).or_else(||
                recheck.then_some(shared).flatten().map(|reason|
                    if reason == ANY_LOADED { recheck_first(name) } else { reason.to_owned() }));
            vec![action("Add back to my DAW", ui::Action::PluginAddBackToDaw {
                class_id: class.into(), expected_removed: identity(current) }, reason.as_deref())]
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
        || !m.publication_pending(&product.class_id).is_ok_and(|pending| !pending)
        || !entry.managed_revision.as_ref().is_some_and(|current|
            removed_by_musician(m, &product.class_id, current)) {
        return;
    }
    // A failed test that removed it, or an unfinished result or check, keeps
    // its own presentation. A removal followed by a bridge update leaves the
    // inventory historical; that is still a plain removal Add back restores.
    if gone(&registration.module.path) {
        product.actions.retain(|offer| !matches!(offer.action, ui::Action::PluginAddBackToDaw { .. }));
        product.disposition = "uninstalled".into();
        product.availability = Some(ui::NativeAvailability { available: false, reason: Some(UNINSTALLED.into()) });
        if let Some(workflow) = product.compatibility.as_mut() {
            workflow.summary = UNINSTALLED.into();
            workflow.primary = None;
            workflow.alternatives.clear();
        }
        return;
    }
    let plain_removal = product.compatibility.as_ref().is_none_or(|workflow| !matches!(workflow.phase,
        ui::CompatibilityPhase::NeedsWork | ui::CompatibilityPhase::TestResultIncomplete
        | ui::CompatibilityPhase::AwaitingRetirement | ui::CompatibilityPhase::PublicationNeedsAttention
        | ui::CompatibilityPhase::Checking));
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
        mark_removed(m, &key, &expected_ref)?;
        return Ok(outcome(true));
    }
    require(entry.publication == Publication::Published
        && entry.managed_revision.as_ref() == Some(&expected_ref), CHANGED)?;
    if publication::physical(&m.link(&key))?.is_none() {
        // Deleted outside the manager: record what is already true.
        retry_busy(|| m.record_missing_publication_removed(&key, &expected_ref))
            .map_err(|error| plain(&name, error))?;
        mark_removed(m, &key, &expected_ref)?;
        let mut value = outcome(false);
        value["link_was_missing"] = true.into();
        value["message"] = link_was_missing(&name).into();
        return Ok(value);
    }
    retry_busy(|| m.unpublish_exact(&key, &expected_ref)).map_err(|error| plain(&name, error))?;
    mark_removed(m, &key, &expected_ref)?;
    Ok(outcome(false))
}

/// Add back the exact removed publication. If it was prepared before a bridge
/// update, `refreshed` checks the same module and class again with the current
/// bridge and publishes that, in the same action.
pub(super) fn add_back(m: &Manager, class: &str, expected: &ui::PublicationIdentity,
    refreshed: impl FnOnce(&str, &RevisionRef) -> Result<RevisionRef>) -> Result<Value> {
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
    // Decided under package.lock, so a bridge update can't slip in between
    // this check and republishing the old bridge.
    let package = retry_busy(|| m.lock("package.lock"))
        .map_err(|_| "The manager is updating. Try again when the update finishes.")?;
    let setup_changed = setup_changed(m, &key, &expected_ref).map_err(|error| plain_add_back(&name, error))?;
    let revision = m.load_revision_record(&key, &expected_ref).map_err(|error| plain_add_back(&name, error))?;
    // A runtime change or bridge update in a changed setup needs a full check.
    let recorded = &revision.registration.environment;
    require(setup_changed || !read_json::<Environment>(&recorded.root.join("environment.json"))
        .is_ok_and(|current| current != *recorded),
        &format!("{name}: {SETUP_CHANGED_SINCE}"))?;
    if setup_changed || package_authority::add_back_needs_refresh(m, &key, &expected_ref)
        .map_err(|error| plain_add_back(&name, error))? {
        // The refresh owner takes package.lock itself and prepares against
        // whatever bridge is current then.
        drop(package);
        let publication = refreshed(&key, &expected_ref).map_err(|error| {
            let text = error.to_string();
            if matches!(text.as_str(), "active_device_lease" | "active_maintenance_lease" | "operator_vendor_active") {
                recheck_first(&name).into()
            } else { plain_add_back(&name, error) }
        })?;
        clear_removed(m, &key);
        return Ok(json!({"class_id":key,"name":name,"added_back_to_daw":true,"already_available":false,
            "rechecked_with_current_bridge":true,"removed_publication":expected,"publication":publication,
            "message":format!("{name} was checked again{} and is back in your DAW.",
                if setup_changed { " with its setup as it is now" } else { " with the updated bridge" }),
            "next":"Rescan VST3 plug-ins in your DAW if it does not appear."}));
    }
    // The exact retained revision only: its runner, Windows module, host and
    // native bytes are verified again by the publication owner.
    retry_busy(|| m.rollback_managed_expected(&key, &expected_ref.id, &expected_ref))
        .map_err(|error| plain_add_back(&name, error))?;
    drop(package);
    clear_removed(m, &key);
    Ok(outcome(false))
}

/// Records of one installer setup, as the setup card shows it.
pub(super) struct SetupPlan {
    pub name: String,
    pub environments: Vec<Environment>,
    /// (class, name) of every plug-in bound to the setup: its admission scope.
    pub classes: Vec<(String, String)>,
    /// (class, name, current publication) of plug-ins currently in the DAW.
    pub in_daw: Vec<(String, String, RevisionRef)>,
    /// Published, but the DAW entry was deleted outside the manager.
    pub missing_link: Vec<(String, String, RevisionRef)>,
    pub removed: Vec<RetiredRegistration>,
    pub plugins: Vec<String>,
    /// The first reason the setup cannot be deleted now, in plain words.
    pub blockers: Vec<String>,
}

/// Environments the manager pins for a fixture, workspace or shared renderer.
/// They are not installer setups and are never offered for deletion.
pub(super) fn protected(m: &Manager, bindings: &[catalogue::EnvironmentBinding]) -> Result<std::collections::BTreeSet<String>> {
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
        classes: vec![], in_daw: vec![], missing_link: vec![], removed: vec![], plugins: vec![], blockers: vec![] };
    for (class, entry) in db.classes.iter().filter(|(_, entry)| ids.contains(&entry.registration.environment.id)) {
        let name = entry.registration.metadata.name.clone();
        plan.plugins.push(name.clone());
        plan.classes.push((class.clone(), name.clone()));
        let pending = m.publication_pending(class)?;
        let physical = publication::physical(&m.link(class));
        match (&entry.publication, &entry.managed_revision) {
            (Publication::Published, Some(current)) if !pending => {
                match (m.load_revision_record(class, current), &physical) {
                    (Ok(revision), Ok(Some(target))) if *target == revision.target =>
                        plan.in_daw.push((class.clone(), name, current.clone())),
                    (Ok(_), Ok(None)) => plan.missing_link.push((class.clone(), name, current.clone())),
                    (Ok(_), Ok(Some(_))) => plan.blockers.push(format!("{name}'s entry in your DAW's plug-in folder was changed outside the manager, so the manager won't change it. Send Setup details.")),
                    _ => plan.blockers.push(attention(&name)),
                }
            }
            (Publication::Removed, Some(_)) if !pending && matches!(physical, Ok(None)) => {
                plan.removed.push(RetiredRegistration { class_id: class.clone(), name, entry: entry.clone() });
            }
            _ => plan.blockers.push(attention(&name)),
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
        // An app record that can't be read might belong to this setup.
        let Ok(app) = read_json::<linux_vst_bridge::vendor_application::Application>(
            &directory.join("application.json")) else {
            plan.blockers.push(UNREADABLE_APP.into());
            continue;
        };
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
    for run in uninstall::records(m)?.into_iter().filter(|r| ids.contains(&r.environment.id)) {
        if is_live(&run.operation)? {
            plan.blockers.push("An uninstaller is still running in this setup. Finish it first.".into());
        } else if !onboarding::retired(&uninstall::result_of(m, &run.operation)?) {
            plan.blockers.push("The manager can't confirm that an uninstaller in this setup closed completely, so it won't delete the setup. Send Setup details if this stays.".into());
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

/// A finished installation journey is history; the card states what is in
/// the DAW now, whatever removed a plug-in since.
fn describe_daw_state(setup: &mut ui::InstallerSetup, plan: &SetupPlan) {
    let Some(journey) = setup.journey.as_ref().filter(|journey|
        journey.phase == ui::InstallationPhase::Available) else { return };
    let in_daw: std::collections::BTreeSet<_> = plan.in_daw.iter().map(|(class, _, _)| class.to_uppercase()).collect();
    let (now, gone): (Vec<_>, Vec<_>) = journey.plugins.iter()
        .filter(|plugin| plugin.stage == ui::InstallationPluginStage::Available)
        .partition(|plugin| in_daw.contains(&plugin.class_id.to_uppercase()));
    if gone.is_empty() { return; }
    let names = |plugins: &[&ui::InstallationPlugin]| plugins.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ");
    // Plug-ins whose files an uninstaller removed can't be added back.
    let uninstalled = !plan.removed.is_empty() && plan.removed.iter()
        .all(|retired| self::gone(&retired.entry.registration.module.path));
    setup.status = if now.is_empty() && uninstalled {
        "This setup's plug-ins were uninstalled. Delete the setup to remove the rest of its Windows files.".into()
    } else if now.is_empty() {
        "None of this setup's plug-ins are in your DAW now. Add them back from Plug-ins.".into()
    } else {
        format!("In your DAW: {}. Not in your DAW now: {}.", names(&now), names(&gone))
    };
}
fn delete_reason(plan: &SetupPlan, busy: Option<&str>) -> Option<String> {
    if !plan.in_daw.is_empty() {
        let names: Vec<_> = plan.in_daw.iter().map(|(_, name, _)| name.as_str()).collect();
        return Some(format!("Remove {} from your DAW first.", names.join(", ")));
    }
    plan.blockers.first().cloned().or_else(|| busy.map(str::to_owned))
}
/// The setup-scoped refusal: a plug-in of this setup is loaded, or shared
/// maintenance is running. Plug-ins of other setups don't count.
fn setup_scope(plan: &SetupPlan, scope: &dyn Fn(&str) -> Option<&'static str>) -> Option<String> {
    plan.classes.iter().find_map(|(class, name)| class_reason(scope(class), name))
        .or_else(|| scope("").map(str::to_owned))
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
        // One unreadable setup must not hide every other card.
        let plan = match plan(m, &setup.installer, &environment, records, db, is_live) {
            Ok(plan) => plan,
            Err(error) => {
                setup.secondary.push(action("Delete this setup", ui::Action::SetupDelete {
                    installer: setup.installer.clone(), environment, confirmed: false, forget_installer: false }, Some(&format!(
                    "The manager can't read this setup's records right now ({error}). Send Setup details."))));
                continue;
            }
        };
        describe_daw_state(setup, &plan);
        if plan.environments.iter().any(|e| protected.contains(&e.id)) { continue; }
        if !plan.in_daw.is_empty() || !plan.missing_link.is_empty() {
            let reason = plan.in_daw.iter().chain(&plan.missing_link)
                .find_map(|(class, name, _)| class_reason(scope(class), name));
            setup.secondary.push(action("Remove its plug-ins from my DAW",
                ui::Action::SetupRemoveFromDaw { environment: environment.clone() }, reason.as_deref()));
        }
        setup.secondary.push(action("Delete this setup", ui::Action::SetupDelete {
            installer: setup.installer.clone(), environment, confirmed: false, forget_installer: false },
            delete_reason(&plan, setup_scope(&plan, scope).as_deref()).as_deref()));
    }
    let scan = deleted_setup::scan(m)?;
    let mut out: Vec<_> = scan.records.into_iter().map(|record| {
        let interrupted = record.state == Deletion::Deleting;
        ui::DeletedSetup { installer: record.installer.clone(), environment: record.environment.clone(),
            name: record.name, state: if interrupted { "deleting" } else { "deleted" }.into(),
            deleted_at: record.deleted_at, plugins: record.plugins,
            actions: if interrupted { vec![action("Finish deleting this setup", ui::Action::SetupDelete {
                installer: record.installer, environment: record.environment, confirmed: false,
                forget_installer: false }, scope(""))] } else { vec![] } }
    }).collect();
    // Its setup stays hidden; the record itself needs Setup details.
    out.extend(scan.unreadable.into_iter().map(|environment| ui::DeletedSetup { installer: String::new(),
        environment, name: "A deleted setup".into(), state: "unreadable".into(), deleted_at: None,
        plugins: vec![], actions: vec![] }));
    Ok(out)
}

/// Take these classes out of the DAW where they are in it now, each by its
/// exact current publication. Returns the names removed.
/// Plug-ins already out of the DAW are recorded as the musician's choice too.
pub(super) fn remove_classes_from_daw(m: &Manager, classes: &[String]) -> Result<Vec<String>> {
    let mut removed = vec![];
    for class in classes {
        let key = class.to_uppercase();
        let Some(entry) = m.registry()?.classes.get(&key).cloned() else { continue };
        let name = entry.registration.metadata.name.clone();
        // An older publication without retained revisions is still taken out.
        let Some(current) = entry.managed_revision.as_ref() else {
            if entry.publication == Publication::Published {
                retry_busy(|| m.unpublish(&key)).map_err(|error| plain(&name, error))?;
                removed.push(name);
            }
            continue;
        };
        let linked = publication::physical(&m.link(&key))?.is_some();
        let result = match entry.publication {
            Publication::Published if linked => retry_busy(|| m.unpublish_exact(&key, current)),
            Publication::Published => retry_busy(|| m.record_missing_publication_removed(&key, current)),
            Publication::Removed if !linked => Ok(()),
            _ => Err(attention(&name).into()),
        };
        if let Err(error) = result {
            let reason = plain(&name, error).to_string();
            return Err(if removed.is_empty() { reason }
                else { format!("Removed {} from your DAW. {reason}", removed.join(", ")) }.into());
        }
        mark_removed(m, &key, current)?;
        if entry.publication == Publication::Published { removed.push(name); }
    }
    Ok(removed)
}

pub(super) fn remove_setup_from_daw(m: &Manager, environment: &str) -> Result<Value> {
    let records = onboarding::history_records(m)?;
    let installer = records.iter().find(|record| record.id == environment).ok_or(CHANGED)?.installer.clone();
    let plan = plan(m, &installer, environment, &records, &m.registry()?, &mut |_| Ok(false))?;
    require(!plan.in_daw.is_empty() || !plan.missing_link.is_empty(), "Nothing from this setup is in your DAW.")?;
    let mut removed = vec![];
    for (class, name, current) in &plan.in_daw {
        if let Err(error) = retry_busy(|| m.unpublish_exact(class, current)) {
            let reason = plain(name, error).to_string();
            return Err(if removed.is_empty() { reason }
                else { format!("Removed {} from your DAW. {reason}", removed.join(", ")) }.into());
        }
        mark_removed(m, class, current)?;
        removed.push(name.clone());
    }
    let missing = record_missing_links(m, &plan)?;
    let mut value = json!({"setup":plan.name,"removed_from_daw":removed,"already_missing":missing,
        "add_back_available":true,"projects":"open_with_plugins_missing_until_added_back"});
    if !missing.is_empty() {
        value["message"] = format!("Removed from your DAW. {} was already missing from your DAW's plug-in folder; it is now recorded as removed too.",
            missing.join(", ")).into();
    }
    Ok(value)
}
/// Plug-ins whose DAW entry was deleted by hand are recorded as removed.
fn record_missing_links(m: &Manager, plan: &SetupPlan) -> Result<Vec<String>> {
    let mut recorded = vec![];
    for (class, name, current) in &plan.missing_link {
        retry_busy(|| m.record_missing_publication_removed(class, current)).map_err(|error| plain(name, error))?;
        mark_removed(m, class, current)?;
        recorded.push(name.clone());
    }
    Ok(recorded)
}

/// The environment directory itself: exact location, owned, never a link.
fn checked_root(m: &Manager, environment: &Environment) -> Result<PathBuf> {
    require(linux_vst_bridge::environment_revision::valid_environment_id(&environment.id)
        && environment.root == m.root.join("environments").join(&environment.id), SETUP_MOVED)?;
    let parent = match fs::symlink_metadata(m.root.join("environments")) {
        Ok(parent) => parent,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(RECONNECT.into()),
        Err(error) => return Err(error.into()),
    };
    let meta = match fs::symlink_metadata(&environment.root) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(RECONNECT.into()),
        Err(error) => return Err(error.into()),
    };
    require(parent.is_dir() && meta.is_dir() && !meta.file_type().is_symlink()
        && meta.uid() == unsafe { libc::getuid() }
        && environment.root.canonicalize()? == m.root.join("environments").canonicalize()?.join(&environment.id),
        SETUP_MOVED)?;
    Ok(environment.root.canonicalize()?)
}
/// The setup's folder was deleted outside the manager, so its files are already
/// gone. Only on the very environments folder the deletion recorded when it
/// started: an empty mount point of an unplugged drive is a different folder,
/// and then the musician is asked to reconnect the drive.
fn already_gone(m: &Manager, environment: &Environment,
    recorded: Option<&deleted_setup::FolderIdentity>) -> Result<bool> {
    require(linux_vst_bridge::environment_revision::valid_environment_id(&environment.id)
        && environment.root == m.root.join("environments").join(&environment.id), SETUP_MOVED)?;
    match fs::symlink_metadata(&environment.root) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let same = recorded.is_some_and(|recorded| deleted_setup::folder_identity(&m.root.join("environments"))
                .is_ok_and(|now| now == *recorded));
            if same { Ok(true) } else { Err(RECONNECT.into()) }
        }
        Err(error) => Err(error.into()),
    }
}
/// The supervisor, installers, vendor apps, keepers and plug-in sessions take
/// this lock while they start work in the environment. Holding it exclusively
/// keeps new work out; the owner and process checks cover what already runs.
fn hold(m: &Manager, environment: &Environment) -> Result<File> {
    let root = checked_root(m, environment)?;
    let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600)
        .custom_flags(libc::O_NOFOLLOW).open(root.join("operation.lock"))?;
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let (keepers, others) = lease_owners(m, environment)?;
        return Err(if keepers > 0 && others == 0 { KEEPER_CLOSING } else { IN_USE }.into());
    }
    Ok(lock)
}
/// Live lease owners in this environment: (idle bridge hosts, anything else).
fn lease_owners(m: &Manager, environment: &Environment) -> Result<(usize, usize)> {
    let leases = m.root.join("runtime/leases");
    if !leases.try_exists()? { return Ok((0, 0)); }
    // Read without registry.lock: owners write through temporary files, and a
    // lease that disappears while it is read was released.
    let mut entries = vec![];
    for (seen, entry) in fs::read_dir(&leases)?.enumerate() {
        // Never a partial census: more entries than the bound refuse.
        require(seen < 4096, TOO_MANY_SESSIONS)?;
        let lease = entry?.path();
        if lease.extension().and_then(|e| e.to_str()) == Some("json") { entries.push(lease); }
    }
    require(entries.len() <= 64, TOO_MANY_SESSIONS)?;
    let (mut keepers, mut others) = (0, 0);
    for lease in entries {
        let Some(sid) = lease.file_stem().and_then(|s| s.to_str()) else { continue };
        let owner = read_json::<PathBuf>(&lease).and_then(|report| m.lease_owner(sid, &report));
        let (owner, path) = match owner {
            Ok(owner) => owner,
            Err(_) if !lease.try_exists()? => continue,
            Err(_) => return Err(UNCHECKED_SESSION.into()),
        };
        if owner["registration"]["environment"]["id"].as_str() != Some(environment.id.as_str())
            && !path.starts_with(&environment.root) { continue; }
        if owner["keeper"] == true { keepers += 1; } else { others += 1; }
    }
    Ok((keepers, others))
}
/// A lease owned by this environment, or a session directory a lease still
/// names, is live or uncertain custody: refuse rather than delete under it.
///
/// A session directory no lease names is normally what an ended session left
/// behind (one stopped with the bridge, for example). A starting session can
/// also have its directory for a moment before its lease and the setup's
/// operation lock. Neither can start here: Delete runs only when every
/// plug-in of the setup is out of the DAW, rechecked under registry.lock, so
/// the DAW can't open a session in it; inspections and installers are
/// operator actions, serialised with this one; and the process census that
/// follows refuses if anything still runs from the setup. A directory changed
/// in the last few seconds is still refused, to leave any such moment alone.
/// Others are deleted with the rest of the setup.
fn require_no_owners(m: &Manager, environment: &Environment) -> Result<()> {
    let (keepers, others) = lease_owners(m, environment)?;
    require(others == 0, IN_USE)?;
    require(keepers == 0, KEEPER_CLOSING)?;
    let sessions = environment.root.join("compatdata/pfx/drive_c/bridge/sessions");
    if sessions.try_exists()? {
        let leases = m.root.join("runtime/leases");
        for (seen, entry) in fs::read_dir(&sessions)?.enumerate() {
            require(seen < 4096, TOO_MANY_SESSIONS)?;
            let entry = entry?;
            let session = entry.file_name();
            require(!leases.join(format!("{}.json", session.to_string_lossy())).try_exists()?,
                "A plug-in session in this setup never finished closing. Restart the computer, then try again; if it stays, send Setup details.")?;
            require(!recently_changed(&entry.path())?, KEEPER_CLOSING)?;
        }
    }
    Ok(())
}
/// A session directory or its owner record changed in the last few seconds.
fn recently_changed(session: &Path) -> Result<bool> {
    let now = std::time::SystemTime::now();
    for path in [session.to_path_buf(), session.join("owner.json")] {
        match fs::symlink_metadata(&path) {
            // A time a little ahead of the clock counts as now; one far
            // ahead is no sign of a session starting.
            Ok(meta) => if match now.duration_since(meta.modified()?) {
                Ok(age) => age < Duration::from_secs(10),
                Err(ahead) => ahead.duration() < Duration::from_secs(10),
            } {
                return Ok(true);
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(false)
}
/// Processes of this user whose Wine prefix, working directory or executable
/// lies inside the environment, under its recorded or its resolved path. A
/// bounded /proc census; other users' processes can't use this private folder.
/// A census that can't list processes (no /proc, or one that doesn't even
/// show the manager itself) refuses instead of reporting nothing running.
pub(super) fn running_inside(roots: &[&Path]) -> Result<Vec<String>> {
    const UNCHECKED: &str = "The manager couldn't check which programs are running, so it changed nothing. Restart the computer, then try again.";
    let proc = Path::new("/proc");
    let listing = fs::read_dir(proc).map_err(|_| UNCHECKED)?;
    let me = unsafe { libc::getuid() };
    let own = std::process::id().to_string();
    let within = |path: &Path| roots.iter().any(|root| path.starts_with(root));
    let mut found = vec![];
    let mut saw_self = false;
    for entry in listing.take(1 << 16) {
        let Ok(entry) = entry else { continue };
        let pid = entry.file_name();
        if !pid.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) { continue; }
        saw_self |= pid.to_string_lossy() == own;
        let dir = entry.path();
        if fs::metadata(&dir).map(|meta| meta.uid()).ok() != Some(me) { continue; }
        let mut inside = ["cwd", "exe"].iter().any(|link|
            fs::read_link(dir.join(link)).is_ok_and(|target| within(&target)));
        if !inside {
            if let Ok(file) = File::open(dir.join("environ")) {
                let mut bytes = vec![];
                if file.take(1 << 20).read_to_end(&mut bytes).is_ok() {
                    inside = bytes.split(|b| *b == 0).any(|variable| {
                        ["WINEPREFIX=", "STEAM_COMPAT_DATA_PATH="].iter().any(|key|
                            variable.strip_prefix(key.as_bytes()).is_some_and(|value|
                                within(Path::new(std::ffi::OsStr::from_bytes(value)))))
                    });
                }
            }
        }
        if inside {
            found.push(fs::read_to_string(dir.join("comm")).map(|name| name.trim().to_owned())
                .unwrap_or_else(|_| "a program".into()));
        }
    }
    require(saw_self, UNCHECKED)?;
    Ok(found)
}
fn require_nothing_running(raw: &Path, root: &Path) -> Result<()> {
    let running = running_inside(&[raw, root])?;
    require(running.is_empty(), &format!("{} is still running from this setup. Close it and try again.",
        running.first().map(String::as_str).unwrap_or("A program")))
}
/// Mount points at or below any of `roots`, from /proc/self/mountinfo text.
fn mount_points_inside(mountinfo: &str, roots: &[&Path]) -> Vec<PathBuf> {
    mountinfo.lines().filter_map(|line| line.split(' ').nth(4)).map(|field| {
        // The kernel escapes space, tab, newline and backslash as \ooo.
        let bytes = field.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 4 <= bytes.len()
                && bytes[i + 1..i + 4].iter().all(|b| (b'0'..=b'7').contains(b)) {
                out.push((bytes[i + 1] - b'0') * 64 + (bytes[i + 2] - b'0') * 8 + (bytes[i + 3] - b'0'));
                i += 4;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        PathBuf::from(std::ffi::OsString::from_vec(out))
    }).filter(|point| roots.iter().any(|root| point.starts_with(root))).collect()
}
fn attached(point: &Path, roots: &[&Path]) -> String {
    let shown = roots.iter().find_map(|root| point.strip_prefix(root).ok())
        .map(|relative| relative.display().to_string()).filter(|relative| !relative.is_empty())
        .unwrap_or_else(|| "its main folder".into());
    format!("Another drive or folder is attached inside this setup ({shown}). The manager won't delete across it. Detach it first, or send Setup details.")
}
/// Deletion never crosses into another file system: refuse while anything
/// is mounted at or below the setup's folder.
fn require_no_mounts(raw: &Path, root: &Path) -> Result<()> {
    let text = fs::read_to_string("/proc/self/mountinfo").map_err(|_|
        "The manager can't check this setup for attached drives, so it won't delete it. Send Setup details.")?;
    match mount_points_inside(&text, &[raw, root]).first() {
        Some(point) => Err(attached(point, &[raw, root]).into()),
        None => Ok(()),
    }
}
/// Hold every environment of the setup and show nothing uses it. An idle
/// bridge host retires by itself shortly after its last plug-in closed, so
/// that alone is waited for (bounded) rather than refused.
fn claim(m: &Manager, environments: &[Environment], wait: Duration,
    recorded: Option<&deleted_setup::FolderIdentity>) -> Result<Vec<Option<File>>> {
    let deadline = Instant::now() + wait;
    loop {
        match claim_once(m, environments, recorded) {
            Err(error) if error.to_string() == KEEPER_CLOSING && Instant::now() < deadline =>
                std::thread::sleep(Duration::from_secs(1)),
            result => return result,
        }
    }
}
/// One lock per environment; None where its folder is already gone.
fn claim_once(m: &Manager, environments: &[Environment],
    recorded: Option<&deleted_setup::FolderIdentity>) -> Result<Vec<Option<File>>> {
    let mut held = vec![];
    for e in environments {
        held.push(if already_gone(m, e, recorded)? { None } else { Some(hold(m, e)?) });
    }
    for (e, lock) in environments.iter().zip(&held) {
        if lock.is_some() { require_unused(m, e)?; }
        // A program can still run from a folder deleted under it.
        else { require_nothing_running(&e.root, &e.root)?; }
    }
    Ok(held)
}
fn require_unused(m: &Manager, environment: &Environment) -> Result<()> {
    require_no_owners(m, environment)?;
    let root = checked_root(m, environment)?;
    require_no_mounts(&environment.root, &root)?;
    require_nothing_running(&environment.root, &root)
}

fn open_at(parent: &OwnedFd, name: &CStr, flags: i32) -> std::io::Result<OwnedFd> {
    let fd = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags | libc::O_CLOEXEC) };
    if fd < 0 { return Err(std::io::Error::last_os_error()); }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}
fn stat_fd(fd: &OwnedFd) -> std::io::Result<libc::stat> {
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd.as_raw_fd(), &mut st) } != 0 { return Err(std::io::Error::last_os_error()); }
    Ok(st)
}
fn unlink_at(parent: &OwnedFd, name: &CStr, flags: i32) -> std::io::Result<()> {
    if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), flags) } == 0 { return Ok(()); }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(error) }
}
/// The open file itself, through the kernel's own link: never the old path.
fn proc_path(fd: &OwnedFd) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", fd.as_raw_fd()))
}
struct Walk { device: u64, mount: Option<u64>, owner: u32, visited: usize }
/// The mount an open file is on, and whether it is that mount's root, where
/// the kernel reports them (Linux 5.8+). Catches same-device bind mounts.
fn mount_of(fd: &OwnedFd) -> (Option<u64>, bool) {
    let mut stx: libc::statx = unsafe { std::mem::zeroed() };
    if unsafe { libc::statx(fd.as_raw_fd(), c"".as_ptr(), libc::AT_EMPTY_PATH | libc::AT_SYMLINK_NOFOLLOW,
        libc::STATX_MNT_ID, &mut stx) } != 0 { return (None, false); }
    let root = libc::STATX_ATTR_MOUNT_ROOT as u64;
    ((stx.stx_mask & libc::STATX_MNT_ID != 0).then_some(stx.stx_mnt_id),
        stx.stx_attributes_mask & root != 0 && stx.stx_attributes & root != 0)
}
const CURRENT: &CStr = c".";
/// Delete every entry of an open directory except `keep`.
fn clear(dir: &OwnedFd, walk: &mut Walk, depth: usize, keep: &[&[u8]]) -> Result<()> {
    require(depth <= 256, TOO_DEEP)?;
    let names = fs::read_dir(proc_path(dir))?.map(|entry| entry.map(|e| e.file_name()))
        .collect::<std::io::Result<Vec<_>>>()?;
    for name in names {
        if keep.contains(&name.as_bytes()) { continue; }
        walk.visited += 1;
        require(walk.visited <= 4_000_000, "setup_deletion_bound")?;
        remove_entry(dir, &CString::new(name.into_vec())?, walk, depth)?;
    }
    Ok(())
}
fn remove_entry(parent: &OwnedFd, name: &CStr, walk: &mut Walk, depth: usize) -> Result<()> {
    // A directory is opened without following links; anything else, links
    // included, is unlinked in place.
    let child = match open_at(parent, name, libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW) {
        Ok(child) => child,
        Err(error) if matches!(error.raw_os_error(), Some(libc::ENOTDIR | libc::ELOOP)) =>
            return Ok(unlink_at(parent, name, 0)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let st = stat_fd(&child)?;
    if st.st_mode & libc::S_IFMT != libc::S_IFDIR {
        drop(child);
        return Ok(unlink_at(parent, name, 0)?);
    }
    let (mount, mount_root) = mount_of(&child);
    if st.st_dev != walk.device || mount_root || walk.mount.zip(mount).is_some_and(|(walk, child)| walk != child) {
        return Err(format!("Another drive or folder is attached inside this setup ({}). The manager stopped before crossing into it. Detach it, then use Finish deleting this setup.",
            name.to_string_lossy()).into());
    }
    require(st.st_uid == walk.owner, NOT_OURS)?;
    if st.st_mode & 0o700 != 0o700 {
        // Read-only folders (common in Windows installs) become writable.
        fs::set_permissions(proc_path(&child), fs::Permissions::from_mode((st.st_mode & 0o7777) | 0o700))?;
    }
    let dir = open_at(&child, CURRENT, libc::O_RDONLY | libc::O_DIRECTORY)?;
    drop(child);
    clear(&dir, walk, depth + 1, &[])?;
    drop(dir);
    Ok(unlink_at(parent, name, libc::AT_REMOVEDIR)?)
}
/// Delete everything in the environment except its identity record and lock,
/// through descriptors opened from the checked root: nothing is resolved by
/// path again, links are removed and never followed, and the walk stops at
/// another device instead of crossing it.
fn remove_windows_files(m: &Manager, environment: &Environment) -> Result<()> {
    let root = checked_root(m, environment)?;
    let expected = fs::symlink_metadata(&root)?;
    let path = CString::new(root.as_os_str().as_bytes())?;
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    require(fd >= 0, SETUP_MOVED)?;
    let dir = unsafe { OwnedFd::from_raw_fd(fd) };
    let st = stat_fd(&dir)?;
    require(st.st_dev == expected.dev() && st.st_ino == expected.ino(), SETUP_MOVED)?;
    let mut walk = Walk { device: expected.dev(), mount: mount_of(&dir).0, owner: unsafe { libc::getuid() }, visited: 0 };
    clear(&dir, &mut walk, 0, &[b"environment.json".as_slice(), b"operation.lock".as_slice()]).map_err(|error|
        if let Some(io) = error.downcast_ref::<std::io::Error>() {
            format!("Some of this setup's files couldn't be deleted ({io}). Use Finish deleting this setup to try again; if it stays, send Setup details.").into()
        } else { error })?;
    File::from(dir).sync_all()?;
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
        "installer_forgotten":record.forget_installer,
        "environments":record.environments.len(),"plugins":record.plugins,
        "history_retained":true,"installer_kept":!record.forget_installer,"projects_changed":false,
        "add_back_available":false})
}

pub(super) fn delete_setup(m: &Manager, installer: &str, environment: &str, forget_installer: bool,
    operation: &str) -> Result<Value> {
    delete_setup_with(m, installer, environment, forget_installer, operation, &mut |op| onboarding::live(op),
        KEEPER_WAIT).map_err(plain_deletion)
}
fn delete_setup_with(m: &Manager, installer: &str, environment: &str, forget_installer: bool, operation: &str,
    is_live: &mut dyn FnMut(&str) -> Result<bool>, keeper_wait: Duration) -> Result<Value> {
    if let Some(mut record) = deleted_setup::load(m, environment)? {
        require(record.installer == installer, CHANGED)?;
        // Finishing may add the musician's choice to forget the installer.
        if forget_installer && !record.forget_installer {
            let ids = record.environments.iter().map(|e| e.id.clone()).collect();
            require(!installer_used_elsewhere(m, installer, &ids)?, INSTALLER_SHARED)?;
            record.forget_installer = true;
            if record.state == Deletion::Deleted { record.state = Deletion::Deleting; record.deleted_at = None; }
            deleted_setup::write(m, &record)?;
        }
        if record.state == Deletion::Deleted { return Ok(summary(&record, true)); }
        let held = claim(m, &record.environments, keeper_wait, record.environments_folder.as_ref())?;
        return finish(m, record, held);
    }
    let _ownership = retry_busy(|| m.lock("onboarding.lock"))?;
    let records = onboarding::history_records(m)?;
    let current = records.iter().find(|record| record.id == environment).ok_or(CHANGED)?;
    require(current.installer == installer, CHANGED)?;
    let sw = software_record(m)?;
    let catalogue = operator_catalogue_readback(m, &sw, &m.registry()?)?;
    let bindings = managed_environment_bindings(m, catalogue.as_ref(), &m.registry()?)?;
    let protected = protected(m, &bindings)?;
    require(!protected.contains(environment), MANAGED)?;
    let mut planned = plan(m, installer, environment, &records, &m.registry()?, is_live)?;
    require(planned.environments.iter().all(|e| !protected.contains(&e.id)), MANAGED)?;
    if let Some(reason) = delete_reason(&planned, None) { return Err(reason.into()); }
    if forget_installer {
        let ids = planned.environments.iter().map(|e| e.id.clone()).collect();
        require(!installer_used_elsewhere(m, installer, &ids)?, INSTALLER_SHARED)?;
    }
    // Exclusive environment locks, then live owners, mounts and a process census.
    let held = claim(m, &planned.environments, keeper_wait, None)?;
    // Only now, with nothing refusing: DAW entries deleted by hand are
    // recorded as removed, and the plan is read again.
    if !planned.missing_link.is_empty() {
        record_missing_links(m, &planned)?;
        planned = plan(m, installer, environment, &records, &m.registry()?, is_live)?;
        if let Some(reason) = delete_reason(&planned, None) { return Err(reason.into()); }
    }
    let record = Tombstone { schema: 1, environment: environment.into(), installer: installer.into(),
        name: planned.name.clone(), environments: planned.environments.clone(), registrations: planned.removed.clone(),
        plugins: planned.plugins.clone(), operation: operation.into(), requested_at: now()?,
        deleted_at: None, state: Deletion::Deleting, forget_installer,
        environments_folder: Some(deleted_setup::folder_identity(&m.root.join("environments"))?) };
    {
        let registry = retry_busy(|| m.lock("registry.lock"))?;
        // Same exact registry facts under the lock that admits publications.
        let recheck = plan_registry(m, &planned)?;
        require(recheck, CHANGED)?;
        // History first: once marked, every reader treats these as history.
        // A full history refuses here, before anything is changed.
        deleted_setup::write(m, &record)?;
        m.retire_removed_registrations(&registry, &record.registrations)?;
    }
    finish(m, record, held)
}
const INSTALLER_SHARED: &str = "This installer was also run in another setup, so the manager keeps it. Delete without forgetting the installer file, or delete that setup first. Nothing was deleted.";
/// The installer was also run inside another setup that still exists.
fn installer_used_elsewhere(m: &Manager, installer: &str, own: &std::collections::BTreeSet<String>) -> Result<bool> {
    let deleted = deleted_setup::environments(m)?;
    let set_up_again = onboarding::history_records(m)?.into_iter()
        .any(|record| record.installer == installer && !own.contains(&record.id));
    Ok(set_up_again || environment_install::records(m)?.into_iter().any(|run| run.installer == installer
        && !own.contains(&run.environment.id) && !deleted.contains(&run.environment.id)))
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
fn finish(m: &Manager, mut record: Tombstone, held: Vec<Option<File>>) -> Result<Value> {
    require(held.len() == record.environments.len(), "setup_environment_lock")?;
    {
        let registry = retry_busy(|| m.lock("registry.lock"))?;
        m.retire_removed_registrations(&registry, &record.registrations)?;
    }
    for (environment, lock) in record.environments.clone().iter().zip(&held) {
        retain_inventory(m, &record.environment, &environment.id)?;
        // A folder already deleted outside the manager has nothing left to
        // delete; nothing may still run from it.
        if lock.is_none() {
            require_nothing_running(&environment.root, &environment.root)?;
            continue;
        }
        require_unused(m, environment)?;
        remove_windows_files(m, environment)?;
    }
    if record.forget_installer {
        // Checked again here: the same installer may have been set up again
        // since the deletion started. Then it is kept.
        let own = record.environments.iter().map(|e| e.id.clone()).collect();
        if installer_used_elsewhere(m, &record.installer, &own)? {
            record.forget_installer = false;
        } else {
            installer_import::forget(m, &record.installer, &deleted_setup::history_directory(m, &record.environment)?)?;
        }
    }
    record.state = Deletion::Deleted;
    record.deleted_at = Some(now()?);
    deleted_setup::write(m, &record)?;
    drop(held);
    Ok(summary(&record, false))
}

#[cfg(test)]
mod tests;

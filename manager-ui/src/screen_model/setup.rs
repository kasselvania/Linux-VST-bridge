//! Setup: add a Windows installer, follow each installation, finish it in the
//! apps it installed, and manage existing setups and their runtime. Every
//! control is a manager offer, except choosing files and moving between
//! pages, which the window does itself.
//!
//! Some rules here are the window's own and duplicate authority the manager
//! should hold; each is marked, and goes when the service supplies it (#489).
use super::journey::{self, Results};
use super::offers::{self, Importance, Offer, OfferRow};
use super::{forms, installer, product, AttemptKey, Choice, Control, Destination, Detail, Disclosure,
    Local, ProductKey, Text};
use crate::model::{Action, AvailableAction, DeletedSetup, EnvironmentInstaller, InstallationPhase,
    InstallerSetup, SetupPhase, Snapshot, VendorApplication};

pub fn phase_label(phase: &SetupPhase) -> &'static str {
    match phase {
        SetupPhase::Imported => "Ready to set up",
        SetupPhase::EnvironmentReady => "Ready to install",
        SetupPhase::InstallerRunning => "Vendor installer is open",
        SetupPhase::SetupNeedsAttention => "Installation needs your attention",
        SetupPhase::InstallerRetired | SetupPhase::ScanReady => "Ready to find installed plug-ins",
        SetupPhase::DiscoveryComplete => "Plug-ins found",
        SetupPhase::CleanupUnconfirmed => "Could not confirm the installer closed",
    }
}

fn phase_priority(phase: &SetupPhase) -> u8 {
    match phase {
        SetupPhase::InstallerRunning => 0,
        SetupPhase::CleanupUnconfirmed => 1,
        SetupPhase::SetupNeedsAttention => 2,
        SetupPhase::EnvironmentReady => 3,
        SetupPhase::InstallerRetired | SetupPhase::ScanReady => 4,
        SetupPhase::Imported => 5,
        SetupPhase::DiscoveryComplete => 6,
    }
}

/// Setups that need the musician sort first.
pub fn attention_priority(setup: &InstallerSetup) -> u8 {
    setup.journey.as_ref().map(|journey| match journey.phase {
        InstallationPhase::VendorRunning => 0,
        InstallationPhase::NeedsAttention => 2,
        InstallationPhase::AwaitingApplication => 2,
        InstallationPhase::PreparingEnvironment | InstallationPhase::Discovering
            | InstallationPhase::PreparingPlugins => 3,
        InstallationPhase::Available => 6,
    }).unwrap_or_else(|| phase_priority(&setup.phase))
}

/// Removal choices sit on every settled card; they are not pending setup work.
fn no_setup_work(setup: &InstallerSetup) -> bool {
    setup.primary.is_none() && setup.secondary.iter().all(|offer| forms::removal_offer(&offer.action))
}

pub fn retained_records_need_refresh(setup: &InstallerSetup, snapshot: &Snapshot) -> bool {
    setup.journey.is_none() && setup.phase == SetupPhase::SetupNeedsAttention
        && no_setup_work(setup)
        && setup.environment.is_some() && !setup.discovered.is_empty()
        && snapshot.onboarding.iter().any(|attempt|
            attempt.installer == setup.installer && attempt.environment == setup.environment
                && attempt.state == "needs_attention" && attempt.failure.is_none()
                && attempt.actions.is_empty() && attempt.details["managed_product"] == true
                && attempt.details["scan"].is_object()
                && attempt.details["installation"]["state"] == "completed"
                && attempt.details["installation"]["cleanup_confirmed"] == true
                && attempt.details["installation"]["owned_live"] == 0
                && attempt.details["installation"]["error"].is_null()
                && attempt.details["installation"]["failure"].is_null()
                && matches!(attempt.details["installation"]["transaction"]["outcome"].as_str(),
                    None | Some("installed")))
}

/// A generated installer label gives way to the names of the current
/// plug-ins it installed.
pub fn display_name(setup: &InstallerSetup, snapshot: &Snapshot) -> String {
    if setup.label_source != "generated" { return setup.name.clone(); }
    let mut names = Vec::new();
    for discovered in &setup.discovered {
        if snapshot.products.iter().any(|product|
            product.environment == discovered.environment
                && product.module_sha256 == discovered.module_sha256
                && product.class_id == discovered.class_id)
            && !names.contains(&discovered.name) {
            names.push(discovered.name.clone());
        }
    }
    match names.as_slice() {
        [] => setup.name.clone(),
        [name] => name.clone(),
        [first, rest @ ..] => format!("{first} and {} other plug-ins", rest.len()),
    }
}

pub fn partial_installation_needs_review(setup: &InstallerSetup, snapshot: &Snapshot) -> bool {
    setup.phase == SetupPhase::SetupNeedsAttention
        && snapshot.onboarding.iter().any(|attempt|
            attempt.installer == setup.installer && attempt.environment == setup.environment
                && attempt.state == "needs_attention" && attempt.failure.is_none()
                && attempt.details["installation"]["state"] == "completed"
                && attempt.details["installation"]["cleanup_confirmed"] == true
                && attempt.details["installation"]["owned_live"] == 0
                && attempt.details["installation"]["error"].is_null()
                && attempt.details["installation"]["failure"].is_null()
                && attempt.details["installation"]["transaction"]["outcome"] == "partial_installation")
}

/// A retained installation record whose plug-ins are all available now is
/// history, not unfinished setup. The window decides this from the records;
/// the service should say it on the setup (#489).
pub fn history_settled_by_current_products(setup: &InstallerSetup, snapshot: &Snapshot) -> bool {
    setup.journey.is_none() && setup.phase == SetupPhase::SetupNeedsAttention
        && no_setup_work(setup)
        && setup.environment.is_some() && !setup.discovered.is_empty()
        && (retained_records_need_refresh(setup, snapshot)
            || partial_installation_needs_review(setup, snapshot))
        && !snapshot.onboarding.iter().any(|attempt|
            attempt.installer == setup.installer && attempt.environment == setup.environment
                && (attempt.failure.is_some() || !attempt.actions.is_empty()
                    || !attempt.details["installation"]["error"].is_null()
                    || !attempt.details["installation"]["failure"].is_null()
                    || !attempt.details["installation"]["transaction"]["first_failure"].is_null()))
        && setup.discovered.iter().all(|discovered|
            setup.environment.as_deref() == Some(discovered.environment.as_str())
                && snapshot.products.iter().any(|product|
                    product.environment == discovered.environment
                        && product.module_sha256 == discovered.module_sha256
                        && product.class_id == discovered.class_id
                        && product.availability.as_ref().is_some_and(|availability| availability.available)))
}

/// Whether a setup still has work: an unfinished journey or setup phase that
/// its current plug-ins have not settled.
pub fn unfinished(setup: &InstallerSetup, snapshot: &Snapshot) -> bool {
    setup.journey.as_ref().map(|journey| journey.phase != InstallationPhase::Available)
        .unwrap_or(setup.phase != SetupPhase::DiscoveryComplete)
        && !history_settled_by_current_products(setup, snapshot)
}

/// What the Setup page is narrowed to and the musician's unsent choices.
#[derive(Clone, Copy, Debug, Default)]
pub struct Focus<'a> {
    /// The exact attempt opened from elsewhere.
    pub attempt: Option<&'a AttemptKey>,
    /// Only this installer's setup is shown.
    pub installer: Option<&'a str>,
    /// A different installer being chosen for one setup.
    pub replacement: Option<&'a InstallerReplacement>,
}

/// The apps of one setup, grouped under it.
pub struct ApplicationGroup<'a> {
    pub environment: &'a str,
    pub setup: Option<&'a InstallerSetup>,
    pub applications: Vec<&'a VendorApplication>,
}

pub fn application_groups<'a>(snapshot: &'a Snapshot, focus: &Focus) -> Vec<ApplicationGroup<'a>> {
    let mut groups: Vec<ApplicationGroup<'a>> = Vec::new();
    for app in &snapshot.vendor_applications {
        let Some(environment) = app.details["environment"].as_str() else { continue; };
        let setup = snapshot.installer_setups.iter().find(|setup|
            setup.environment.as_deref() == Some(environment));
        if focus.attempt.and_then(|key| key.environment.as_deref())
            .is_some_and(|selected| selected != environment)
            || focus.installer.is_some_and(|installer|
                setup.is_none_or(|setup| setup.installer != installer)) { continue; }
        if let Some(group) = groups.iter_mut().find(|group| group.environment == environment) {
            group.applications.push(app);
        } else {
            groups.push(ApplicationGroup {environment, setup, applications: vec![app]});
        }
    }
    groups
}

fn application_group_names(snapshot: &Snapshot, environment: &str) -> Vec<String> {
    let mut names = Vec::new();
    for product in &snapshot.products {
        if product.environment == environment && !names.contains(&product.name) {
            names.push(product.name.clone());
        }
    }
    names
}

/// An offer the setup's program choice makes instead of a plain button.
fn offer_uses_program_choice(snapshot: &Snapshot, setup: &InstallerSetup, offer: &AvailableAction) -> bool {
    let application = match &offer.action {
        Action::VendorSoftwareInstall {application, ..}
            | Action::VendorApplicationFocus {application}
            | Action::VendorApplicationStop {application} => application,
        _ => return false,
    };
    snapshot.vendor_applications.iter().any(|app| &app.id == application
        && app.details["choose_application"] == true
        && app.details["environment"].as_str() == setup.environment.as_deref())
}

/// First-installer offers a running installer in the same setup supersedes.
/// Hiding them is a window rule over manager offers; the service should stop
/// offering them while that installer runs (#489).
fn obsolete_initial_installer_offer(offer: &AvailableAction) -> bool {
    offer.action.software_install().is_some() || matches!(offer.action, Action::InstallerNewAttempt {..}
        | Action::InstallerStart {..} | Action::InstallerStartWithPolicy {..}
        | Action::InstallerFocus {..} | Action::InstallerStop {..} | Action::InstallerScan {..})
}

/// The manager's "Retry" of the last installer run in an existing setup.
pub fn installer_retry(offer: &AvailableAction) -> bool {
    offer.label == "Retry" && offer.action.environment_installer_start().is_some()
}

/// A local file-choice draft, not installation authority. Submitting still
/// requires the current manager offer for these exact bytes and setup.
#[derive(Clone, Debug, PartialEq)]
pub struct InstallerReplacement {
    pub original: String,
    pub environment: Option<String>,
    pub runtime: Option<(String, u64)>,
    pub selected: Option<String>,
}

/// Whether another installer can be chosen for this setup now, and the
/// setup's identity to check again when one is chosen. Refusing while an
/// installer or app runs, or before cleanup, is the window's own rule over
/// the manager's records (#489).
pub fn replacement_context(snapshot: &Snapshot, setup: &InstallerSetup) -> Option<InstallerReplacement> {
    if setup.phase == SetupPhase::CleanupUnconfirmed || setup.primary.iter().chain(&setup.secondary)
        .any(|offer| matches!(offer.action, Action::PluginSetupFocus {..}|Action::PluginSetupStop {..}|Action::InstallerFocus {..} | Action::InstallerStop {..}
            | Action::VendorApplicationFocus {..} | Action::VendorApplicationStop {..})) {
        return None;
    }
    let runtime = if let Some(environment) = &setup.environment {
        let mut targets = snapshot.environment_installers.iter().filter(|t| &t.environment == environment);
        let target = targets.next()?;
        if targets.next().is_some() || target.actions.iter().any(|offer| matches!(offer.action,
            Action::EnvironmentInstallerFocus {..} | Action::EnvironmentInstallerStop {..}))
            || (target.operation.is_some() && !installer_result_retired(&target.result)) {
            return None;
        }
        target.runtime.as_ref().map(|r| (r.runner.clone(), r.revision))
    } else {
        // A missing environment on an ambiguous history is not an unstarted setup.
        if setup.history.iter().any(|attempt| attempt.environment.is_some()) { return None; }
        None
    };
    Some(InstallerReplacement { original: setup.installer.clone(),
        environment: setup.environment.clone(), runtime, selected: None })
}

fn installer_result_retired(result: &serde_json::Value) -> bool {
    result["cleanup_confirmed"] == true && result["owned_live"] == 0
        && matches!(result["state"].as_str(), Some("completed" | "failed" | "cancelled"))
}

/// The manager's current offer for the chosen replacement, or why there is none.
pub fn replacement_offer(snapshot: &Snapshot, draft: &InstallerReplacement)
    -> Result<AvailableAction, &'static str> {
    let mut originals = snapshot.installer_setups.iter().filter(|s| s.installer == draft.original);
    let original = originals.next().ok_or("The original setup is unavailable. Refresh Setup.")?;
    if originals.next().is_some() { return Err("The original setup is ambiguous. Review Setup."); }
    let current = replacement_context(snapshot, original)
        .ok_or("Finish or stop the current operation and wait for cleanup before choosing another installer.")?;
    if current.environment != draft.environment || current.runtime != draft.runtime {
        return Err("This setup changed during file selection. Choose another installer again.");
    }
    let selected = draft.selected.as_ref().ok_or("Choose a different installer file.")?;
    if selected == &draft.original { return Err("That is the same installer. Choose a different file, or use Retry."); }
    let mut offer = if let Some(environment) = &draft.environment {
        snapshot.environment_installers.iter().find(|t| &t.environment == environment)
            .and_then(|target| target.choices.iter().find(|offer|offer.action.environment_installer_start()
                .is_some_and(|(env,installer,policy)|env==environment && installer==selected && policy.is_none()))).cloned()
    } else {
        // Already imported bytes may belong to another working setup. Never
        // turn this file choice into a continuation of that other setup.
        snapshot.installer_setups.iter().find(|s| &s.installer == selected
            && s.environment.is_none() && s.history.iter().all(|h|h.environment.is_none()))
            .and_then(|s| s.primary.as_ref()).filter(|offer|offer.action.software_install()
                .is_some_and(|(installer,_,policy)|installer==selected && policy.is_none())).cloned()
    }.ok_or("This file has no installation offer for the selected setup. Review Setup or choose another file.")?;
    offer.label = if draft.environment.is_some() { "Run installer in this setup" }
        else { "Install and connect" }.into();
    Ok(offer)
}

/// One plain sentence for a per-user default the supervisor filled, or could
/// not fill, before the app's last launch. A value the musician chose is silent.
pub fn vendor_default_notice(app: &VendorApplication) -> Option<String> {
    let entries = app.details["dependency"]["registry_defaults"].as_array()?;
    let name = &app.name;
    let set = format!("{name}'s download folder was empty, so the bridge set it to the Downloads folder inside {name}'s Windows setup.");
    // A missing folder means the launch's settings could not be read at all.
    entries.iter().find_map(|entry| match (entry["known_folder"].as_str(), entry["result"].as_str()?,
        entry["reason"].as_str()) {
        (Some("Downloads"), "applied", _) => Some(set.clone()),
        (Some("Downloads"), "applied_after_service_start", _) => Some(format!(
            "{set} If downloads still fail, close {name} and open it again once.")),
        (_, "failed", Some("cancelled")) => None,
        (Some("Downloads"), "failed", Some("query_unreadable")) => Some(format!(
            "The bridge couldn't read {name}'s download folder, so it left it unchanged. If downloads fail, set it in {name}'s preferences.")),
        (Some("Downloads") | None, "failed", _) => Some(format!(
            "Couldn't set {name}'s download folder; set it in {name}'s preferences before downloading.")),
        _ => None,
    })
}

/// The Setup page.
#[derive(Clone, Debug)]
pub struct Setup {
    pub heading: &'static str,
    pub add: Control,
    pub add_label: &'static str,
    pub daw: forms::DawPicker,
    /// The journey in a few words, or that actions wait for current status.
    pub progress: Text,
    pub existing: Option<ExistingSetups>,
    /// Shown while only one installer's setup is listed.
    pub showing_one: Option<(&'static str, Control)>,
    pub cards: Vec<SetupCard>,
    /// What adding an installer does, when nothing was added yet.
    pub empty: Vec<&'static str>,
    /// The focused installer is missing from this readback.
    pub missing: Option<&'static str>,
    pub apps: Vec<AppGroup>,
    pub deleted: Option<DeletedSetups>,
    pub footer: &'static str,
    pub history: Control,
}

impl Setup {
    /// `fresh`: the offers are current; `pending`: a request or readback is
    /// in flight. Without current status every offer is shown, held.
    ///
    /// A frontend that applies a click while it draws (choosing another
    /// installer, showing all setup) builds the same parts in this order,
    /// each with the focus as it is when that part is drawn.
    pub fn build(snapshot: &Snapshot, fresh: bool, pending: bool, focus: Focus) -> Self {
        let cards: Vec<_> = listed(snapshot, focus.installer).into_iter()
            .map(|setup| SetupCard::build(snapshot, setup, fresh, pending, focus.replacement)).collect();
        Self {
            heading: HEADING,
            add: add_installer(fresh, pending),
            add_label: ADD_LABEL,
            daw: forms::daw_picker(),
            progress: progress(fresh),
            existing: ExistingSetups::build(snapshot, fresh, pending),
            showing_one: focus.installer.map(|_| showing_one()),
            missing: (!snapshot.installer_setups.is_empty() && cards.is_empty()).then_some(MISSING),
            empty: if snapshot.installer_setups.is_empty() { EMPTY.to_vec() } else { vec![] },
            cards,
            apps: application_groups(snapshot, &focus).into_iter()
                .map(|group| AppGroup::build(snapshot, &group, fresh, pending)).collect(),
            deleted: DeletedSetups::build(snapshot, fresh, pending),
            footer: FOOTER,
            history: history(),
        }
    }
}

pub const HEADING: &str = "Setup";
pub const ADD_LABEL: &str = "Add a plug-in";
/// What adding an installer does, when nothing was added yet.
pub const EMPTY: [&str; 2] = [
    "Choose the installer you downloaded from your plug-in vendor. Selecting it adds a managed copy; the installer runs only when you choose to start it.",
    "Choose your DAW when starting installation. Complete any requests in the vendor's window. After installation finishes, the manager finds and connects the installed plug-ins automatically.",
];
pub const MISSING: &str = "The selected installer is unavailable in this readback. Refresh or show all setup.";
pub const FOOTER: &str = "Vendor accounts and authorization stay with you and the vendor.";

pub fn add_installer(fresh: bool, pending: bool) -> Control {
    Control { label: "Choose Windows installer", importance: Importance::Primary,
        enabled: fresh && !pending, does: Local::ChooseInstaller }
}

pub fn progress(fresh: bool) -> Text {
    if fresh {
        Text::hint("Choose software and your DAW · Complete vendor requests · Make music")
    } else { Text::caution_hint("Refreshing setup · actions wait for current status") }
}

pub fn showing_one() -> (&'static str, Control) {
    ("Showing the selected installer", Control { label: "Show all setup", importance: Importance::Secondary,
        enabled: true, does: Local::ShowAllSetup })
}

pub fn history() -> Control {
    Control::go("Technical setup history", Importance::Secondary, Destination::Diagnostics)
}

/// The setups listed, needing the musician first, narrowed to one installer.
pub fn listed<'a>(snapshot: &'a Snapshot, installer: Option<&str>) -> Vec<&'a InstallerSetup> {
    let mut setups: Vec<_> = snapshot.installer_setups.iter().filter(|setup|
        installer.is_none_or(|id| id == setup.installer)).collect();
    setups.sort_by_key(|setup| attention_priority(setup));
    setups
}

/// One installer's setup: its journey, next step, other choices and records.
#[derive(Clone, Debug)]
pub struct SetupCard {
    pub installer: String,
    pub name: String,
    pub title: &'static str,
    pub status: Text,
    pub journey: Option<Results>,
    pub primary: Option<Offer>,
    pub discovered: Option<Control>,
    pub secondary: OfferRow,
    /// The latest installer run in this same setup, with its exact controls.
    pub latest: Option<(String, OfferRow)>,
    pub replace: Replace,
    pub draft: Option<ReplacementDraft>,
    pub details: Disclosure,
    pub removal: Option<Disclosure>,
    /// The offers are current; otherwise every one is held.
    pub actionable: bool,
}

/// Choosing a different installer, or a required app's installer, for a setup.
#[derive(Clone, Debug)]
pub enum Replace {
    Allowed(Box<ReplaceChoices>),
    /// Why no installer can be chosen now.
    Waiting(&'static str),
}

#[derive(Clone, Debug)]
pub struct ReplaceChoices {
    pub dependency: Option<RequiredApp>,
    pub another: Control,
}

/// A required app, such as a licence manager, installed into the same setup.
#[derive(Clone, Debug)]
pub struct RequiredApp {
    pub title: &'static str,
    /// The setup's own dependency help from the manager.
    pub help: Vec<String>,
    pub explanation: &'static str,
    pub choose: Control,
}

/// The replacement being chosen for this setup.
#[derive(Clone, Debug)]
pub struct ReplacementDraft {
    pub selected: Option<SelectedInstaller>,
    pub keep: Control,
}

#[derive(Clone, Debug)]
pub struct SelectedInstaller {
    pub title: String,
    /// What running it in this same setup changes.
    pub same_setup: Option<(&'static str, String, Option<String>)>,
    pub offer: Result<Offer, &'static str>,
    /// The setup to show once the offer is pressed: the new one, when this
    /// installer starts a setup of its own.
    pub focus_after: Option<String>,
}

impl ReplacementDraft {
    /// The draft for this setup's installer, if one is being chosen for it.
    pub fn build(snapshot: &Snapshot, installer: &str, replacement: Option<&InstallerReplacement>,
        pending: bool) -> Option<Self> {
        let busy = snapshot.system.inactive_reason();
        replacement.filter(|draft| draft.original == installer).map(|draft| Self {
            selected: draft.selected.as_ref().map(|selected| SelectedInstaller {
                title: format!("Selected installer: {}", snapshot.installer_setups.iter()
                    .find(|s| &s.installer == selected).map(|s| s.name.as_str())
                    .unwrap_or("Selected Windows installer")),
                same_setup: snapshot.environment_installers.iter().find(|target|
                    draft.environment.as_ref() == Some(&target.environment)).map(|target| (
                        "Continue in this same setup.", target.consequence.clone(),
                        (!target.affected.is_empty()).then(|| format!("Affected plug-ins: {}", target.affected.join(", "))))),
                offer: replacement_offer(snapshot, draft)
                    .map(|offer| offers::offer(&offer, Importance::Primary, busy, pending)),
                focus_after: draft.environment.is_none().then(|| selected.clone()),
            }),
            keep: Control { label: "Keep current installer", importance: Importance::Secondary,
                enabled: !pending, does: Local::KeepCurrentInstaller },
        })
    }
}

impl SetupCard {
    pub fn build(snapshot: &Snapshot, setup: &InstallerSetup, fresh: bool, pending: bool,
        replacement: Option<&InstallerReplacement>) -> Self {
        let busy = snapshot.system.inactive_reason();
        let retained_refresh = retained_records_need_refresh(setup, snapshot);
        let partial = partial_installation_needs_review(setup, snapshot);
        let settled = history_settled_by_current_products(setup, snapshot);
        let current = setup.environment.as_deref().map(|environment| (snapshot, environment));
        let title = if let Some(journey) = &setup.journey { journey::current_label(journey, current) }
            else if settled {"Plug-ins available in your DAW"}
            else if retained_refresh {"Installation records need attention"}
            else if partial {"Review earlier installation"}
            else {phase_label(&setup.phase)};
        let continuation = snapshot.environment_installers.iter().find(|target|
            setup.environment.as_ref() == Some(&target.environment) && target.operation.is_some());
        let reviewing = replacement.is_some_and(|draft| draft.original == setup.installer);
        let shown_primary = setup.primary.as_ref().filter(|offer| (continuation.is_none()
            || !obsolete_initial_installer_offer(offer))
            && !reviewing && !offer_uses_program_choice(snapshot, setup, offer));
        let status = if setup.journey.is_some() { Text::body(&setup.status) }
            else if settled { Text::body(format!("{} plug-ins available in your DAW.", setup.discovered.len())) }
            else if retained_refresh { Text::body("Installation records need refreshing before making setup changes.") }
            else if partial { Text::body("A complete installation was not confirmed. Review the installation result before making setup changes. Plug-ins already available in your DAW remain available.") }
            else if setup.phase == SetupPhase::DiscoveryComplete {
                Text::body(format!("{} plug-ins found. Open them to see their current DAW connection.", setup.discovered.len()))
            } else if setup.phase == SetupPhase::Imported && setup.primary.is_none() { Text::caution(&setup.status) }
            else { Text::body(&setup.status) };
        let row = |actions: &[AvailableAction]| offers::row(actions, Importance::Secondary, busy, pending);
        let secondary: Vec<_> = setup.secondary.iter().filter(|offer| !forms::removal_offer(&offer.action)
            && (continuation.is_none() || !obsolete_initial_installer_offer(offer)) && !reviewing
                && offer.action.software_install().is_none_or(|(_,_,policy)|policy.is_none())
                && !offer_uses_program_choice(snapshot, setup, offer)).cloned().collect();
        let latest = continuation.map(|target| {
            let actions: Vec<_> = target.actions.iter().filter(|offer| installer_retry(offer)
                || matches!(offer.action,
                Action::EnvironmentInstallerFocus {..} | Action::EnvironmentInstallerStop {..}
                    | Action::EnvironmentInstallerScan {..}))
                .cloned().collect();
            (format!("Latest installer in this setup: {}",
                target.result["state"].as_str().unwrap_or("status unavailable").replace('_', " ")), row(&actions))
        });
        let replace = match replacement_context(snapshot, setup) {
            Some(context) => Replace::Allowed(Box::new(ReplaceChoices {
                dependency: context.environment.is_some().then(|| RequiredApp {
                    title: "Install a required app in this setup",
                    help: snapshot.environment_installers.iter().find(|target|
                        context.environment.as_ref() == Some(&target.environment))
                        .map(|target| target.dependency_help.clone()).unwrap_or_default(),
                    explanation: "If the plug-in asks for a licence manager or another dependency, download its Windows installer from the vendor. Choose it here; after it closes, the bridge checks this same setup again.",
                    choose: Control { label: "Choose dependency installer", importance: Importance::Secondary,
                        enabled: fresh && !pending, does: Local::ChooseReplacement(context.clone()) },
                }),
                another: Control { label: "Choose another installer", importance: Importance::Secondary,
                    enabled: fresh && !pending, does: Local::ChooseReplacement(context) },
            })),
            None => Replace::Waiting("To choose another installer, finish or stop the current operation and wait for cleanup."),
        };
        let draft = ReplacementDraft::build(snapshot, &setup.installer, replacement, pending);
        let held = |actions: &[AvailableAction]| Detail::Held { actionable: fresh, row: row(actions) };
        let mut details: Vec<Detail> = vec![Text::detail(format!("Recorded installer label: {}", setup.name)).into()];
        if let Some(environment) = &setup.environment {
            details.push(Text::detail(format!("Setup environment: {environment}")).into());
        }
        details.push(Text::body(&setup.status).into());
        if let Some(compatibility) = &setup.compatibility { details.push(Text::detail(compatibility).into()); }
        if continuation.is_none() && !reviewing {
            let advanced: Vec<_> = setup.primary.iter().chain(&setup.secondary)
                .filter(|offer| offer.action.software_install().is_some_and(|(_,_,policy)| policy.is_some()))
                .cloned().collect();
            details.push(held(&advanced));
        }
        for attempt in snapshot.onboarding.iter().filter(|attempt|
            attempt.installer == setup.installer && attempt.environment == setup.environment) {
            details.extend(installer::installer_lines(&attempt.details["installation"]).into_iter()
                .map(|line| Text::body(line).into()));
            details.extend(installer::installer_policy_lines(&attempt.details["installation"]).into_iter()
                .map(|line| Text::detail(line).into()));
            if let Some(failure) = &attempt.failure {
                details.extend(installer::failure_lines(failure).into_iter().map(|line| Text::body(line).into()));
            }
            if continuation.is_none() && !reviewing {
                let advanced: Vec<_> = attempt.actions.iter().filter(|offer|
                    matches!(offer.action, Action::InstallerStartWithPolicy {..})).cloned().collect();
                details.push(held(&advanced));
            }
        }
        details.push(held(std::slice::from_ref(&setup.rename)));
        details.push(Text::detail(format!("Installer SHA-256: {}", setup.installer)).into());
        let removal: Vec<_> = setup.secondary.iter().filter(|offer| forms::removal_offer(&offer.action))
            .cloned().collect();
        let uninstallers: Vec<_> = snapshot.uninstallers.iter().filter(|u|
            setup.environment.as_deref() == Some(u.environment.as_str())).collect();
        let removal = (!removal.is_empty() || !uninstallers.is_empty()).then(|| {
            let mut content = Vec::new();
            for uninstaller in &uninstallers {
                content.push(held(std::slice::from_ref(&uninstaller.action)));
                if !uninstaller.affected.is_empty() {
                    content.push(Text::detail(format!("Plug-ins it removes: {}", uninstaller.affected.join(", "))).into());
                }
            }
            content.push(Text::body("Remove its plug-ins from your DAW, or delete the whole Windows setup. Deleting also deletes everything installed in it, files saved inside it such as presets, and any licences or sign-ins stored there. Your project files are not changed.").into());
            content.push(held(&removal));
            Disclosure::new("Uninstall, remove or delete", content)
        });
        Self {
            installer: setup.installer.clone(),
            name: display_name(setup, snapshot),
            title,
            status,
            journey: setup.journey.as_ref().map(|journey| Results::build(journey, current, shown_primary)),
            primary: shown_primary.map(|offer| offers::offer(offer, Importance::Primary, busy, pending)),
            discovered: (!setup.discovered.is_empty()).then(|| Control::go("View discovered plug-ins",
                Importance::Secondary, Destination::Products(setup.discovered.iter().map(|product| ProductKey {
                    environment: product.environment.clone(),
                    module_sha256: product.module_sha256.clone(),
                    class_id: product.class_id.clone(),
                }).collect())).enabled(fresh)),
            secondary: row(&secondary),
            latest,
            replace,
            draft,
            details: Disclosure::new("Setup details and options", details),
            removal,
            actionable: fresh,
        }
    }
}

/// An existing setup's own controls: programs, its running installer, its
/// runtime and the installers that can run in it.
#[derive(Clone, Debug, PartialEq)]
pub struct SetupControls {
    pub environment: String,
    pub help: Vec<String>,
    pub programs_title: &'static str,
    pub programs_hint: &'static str,
    pub programs: OfferRow,
    /// Show and Stop for an installer running in this setup.
    pub running: OfferRow,
    pub consequence: String,
    pub affected: Text,
    pub identity: Disclosure,
    pub runtime: Option<RuntimeChoice>,
    pub last_installer: Option<LastInstaller>,
    /// Retry, rescan and the manager's other continuations.
    pub continuation: OfferRow,
    pub installers: InstallerChoice,
}

/// The setup's compatibility runtime and the installed runtimes it can move to.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeChoice {
    pub title: &'static str,
    pub current: String,
    pub consequence: String,
    pub affected_title: &'static str,
    pub affected: Vec<String>,
    pub placeholder: &'static str,
    /// Keyed by runner; each carries the manager's exact offer.
    pub choices: Vec<Choice<OfferRow>>,
    /// Where to get another runtime when none is installed.
    pub none: Option<&'static str>,
}

/// The latest installer run in this setup.
#[derive(Clone, Debug, PartialEq)]
pub struct LastInstaller {
    pub state: String,
    pub installation: Option<String>,
    pub warning: Option<&'static str>,
    pub details: Disclosure,
}

/// Installers already added that can run in this setup.
#[derive(Clone, Debug, PartialEq)]
pub struct InstallerChoice {
    /// The installer chosen before the musician chooses one.
    pub preselected: Option<String>,
    pub placeholder: &'static str,
    /// Keyed by installer: its offer, and its advanced options when offered.
    pub choices: Vec<Choice<(OfferRow, Option<Disclosure>)>>,
    pub none: Option<&'static str>,
}

impl SetupControls {
    pub fn build(target: &EnvironmentInstaller, pending: bool) -> Self {
        let row = |actions: &[AvailableAction]| offers::row(actions, Importance::Secondary, None, pending);
        let filtered = |keep: &dyn Fn(&AvailableAction) -> bool| -> Vec<AvailableAction> {
            target.actions.iter().filter(|offer| keep(offer)).cloned().collect()
        };
        let programs = filtered(&|offer| matches!(offer.action, Action::ProgramChoose {..}));
        let running = filtered(&|offer| matches!(offer.action,
            Action::EnvironmentInstallerFocus { .. } | Action::EnvironmentInstallerStop { .. }));
        let continuation = filtered(&|offer| installer_retry(offer)
            || offer.action.environment_installer_start().is_none() && !matches!(offer.action,
            Action::EnvironmentInstallerFocus { .. } | Action::EnvironmentInstallerStop { .. }
            | Action::ProgramChoose {..}));
        let state = target.result["state"].as_str().unwrap_or("unavailable").replace('_'," ");
        Self {
            environment: target.environment.clone(),
            help: target.dependency_help.clone(),
            programs_title: "Programs",
            programs_hint: "Choose a Windows program to remember and open in this setup. Its files and resources stay in place.",
            programs: row(&programs),
            running: row(&running),
            consequence: target.consequence.clone(),
            affected: if target.affected.is_empty() {
                Text::detail("This existing setup has no discovered plug-ins yet.")
            } else { Text::strong(format!("Affected plug-ins: {}", target.affected.join(", "))) },
            identity: Disclosure::new("Setup identity", vec![Text::detail(&target.environment).into()]),
            runtime: target.runtime.as_ref().map(|runtime| RuntimeChoice {
                title: "Compatibility runtime",
                current: format!("Current runtime: {} · setup revision {}", runtime.runner, runtime.revision),
                consequence: runtime.consequence.clone(),
                affected_title: "Affected plug-ins and applications",
                affected: runtime.affected.clone(),
                placeholder: "Choose an installed runtime",
                choices: runtime.choices.iter().filter_map(|offer| match &offer.action {
                    Action::EnvironmentRuntimeSelect { runner, .. } => Some(Choice { key: runner.clone(),
                        label: offer.label.clone(), content: row(std::slice::from_ref(offer)) }),
                    _ => None,
                }).collect(),
                none: runtime.choices.is_empty()
                    .then_some("To download Windows plug-in support, open Diagnostics → Manager actions."),
            }),
            last_installer: target.operation.is_some().then(|| LastInstaller {
                state: format!("Last installer: {state}"),
                installation: target.result["transaction"]["durable_installation"].is_string()
                    .then(|| installer::installer_installation_label(&target.result["transaction"])),
                warning: (target.result["state"] == "failed" || target.result["state"] == "cancelled")
                    .then_some("Files or vendor state may already have changed. Rescan to see what remains, or retry in this same compatibility space. Stopping does not restore prior vendor state."),
                details: Disclosure::new("Installer details", installer::installer_lines(&target.result)
                    .into_iter().map(|line| Text::detail(line).into()).collect()),
            }),
            continuation: row(&continuation),
            installers: InstallerChoice {
                preselected: target.installer.clone(),
                placeholder: "Choose an added Windows installer",
                choices: target.choices.iter().filter_map(|offer| match offer.action.environment_installer_start() {
                    Some((_, installer, None)) => {
                        let advanced = filtered(&|other| !installer_retry(other)
                            && other.action.environment_installer_start().is_some_and(|(_, chosen, policy)|
                                chosen == installer && policy.is_some()));
                        Some(Choice { key: installer.into(), label: offer.label.clone(),
                            content: (row(std::slice::from_ref(offer)), (!advanced.is_empty()).then(||
                                Disclosure::new("Advanced installer options", vec![Detail::Offers(row(&advanced))]))) })
                    }
                    _ => None,
                }).collect(),
                none: target.choices.is_empty().then_some("Choose a Windows installer to add a local installer first."),
            },
        }
    }
}

/// Existing setups the musician chooses between, to run something in one.
#[derive(Clone, Debug)]
pub struct ExistingSetups {
    pub title: &'static str,
    pub placeholder: &'static str,
    /// Keyed by environment.
    pub setups: Vec<Choice<ExistingSetup>>,
    pub actionable: bool,
}

#[derive(Clone, Debug)]
pub struct ExistingSetup {
    pub controls: SetupControls,
    /// Programs the musician chose for this setup.
    pub programs: Vec<UserProgram>,
    /// A running installer's exact Stop, shown even while another setup is chosen.
    pub stop: Option<OfferRow>,
}

#[derive(Clone, Debug)]
pub struct UserProgram {
    pub name: String,
    pub version: String,
    pub path: Option<String>,
    pub offers: OfferRow,
}

impl ExistingSetups {
    pub fn build(snapshot: &Snapshot, fresh: bool, pending: bool) -> Option<Self> {
        if snapshot.environment_installers.is_empty() { return None; }
        let busy = snapshot.system.inactive_reason();
        Some(Self {
            title: "Manage an existing setup",
            placeholder: "Choose the existing setup or plug-in",
            setups: snapshot.environment_installers.iter().map(|target| Choice {
                key: target.environment.clone(),
                label: target.name.clone(),
                content: ExistingSetup {
                    controls: SetupControls::build(target, pending),
                    programs: snapshot.vendor_applications.iter().filter(|app|
                        app.details["user_selected"] == true && app.details["environment"] == target.environment)
                        .map(|app| UserProgram { name: app.name.clone(), version: app.version.clone(),
                            path: app.details["executable"]["path"].as_str().map(str::to_owned),
                            offers: offers::row(&app.actions, Importance::Secondary, busy, pending) }).collect(),
                    stop: target.actions.iter().any(|offer| matches!(offer.action, Action::EnvironmentInstallerStop { .. }))
                        .then(|| offers::row(&target.actions, Importance::Secondary, None, pending)),
                },
            }).collect(),
            actionable: fresh,
        })
    }
}

/// The apps installed in one setup that install or manage its plug-ins.
#[derive(Clone, Debug)]
pub struct AppGroup {
    pub environment: String,
    pub title: String,
    pub plugins: Option<String>,
    pub apps: Vec<AppCard>,
    pub picker: Option<ProgramPicker>,
    pub actionable: bool,
}

#[derive(Clone, Debug)]
pub struct AppCard {
    pub id: String,
    pub name: String,
    /// The installation this app is part of: its headline and results.
    pub progress: Option<(&'static str, Results)>,
    /// Said when the app has no installation journey.
    pub hint: Option<&'static str>,
    /// The download-folder notice, then what installing with it changes.
    pub notes: Vec<String>,
    pub offers: OfferRow,
}

/// Choosing one installed program of this setup to open.
#[derive(Clone, Debug)]
pub struct ProgramPicker {
    pub title: &'static str,
    pub hint: &'static str,
    pub placeholder: &'static str,
    /// Keyed by application; what opening the chosen one offers.
    pub choices: Vec<Choice<Option<ProgramOpen>>>,
    /// Shown, not pressable, until a program is chosen. No reason is printed.
    pub unchosen: &'static str,
    /// A program no longer offered is forgotten while status is current.
    pub forget_missing: bool,
}

/// Opening one chosen program: what it changes, and the manager's offer.
#[derive(Clone, Debug)]
pub struct ProgramOpen {
    pub consequence: Option<String>,
    pub offer: Offer,
}

impl AppGroup {
    pub fn build(snapshot: &Snapshot, group: &ApplicationGroup, fresh: bool, pending: bool) -> Self {
        let busy = snapshot.system.inactive_reason();
        let names = application_group_names(snapshot, group.environment);
        let setup_name = group.setup.map(|setup| display_name(setup, snapshot))
            .unwrap_or_else(|| names.first().cloned().unwrap_or_else(|| group.applications[0].name.clone()));
        let current = Some((snapshot, group.environment));
        let choices: Vec<_> = group.applications.iter()
            .filter(|app| app.details["choose_application"] == true).collect();
        Self {
            environment: group.environment.into(),
            title: format!("Apps for {setup_name}"),
            plugins: (!names.is_empty()).then(|| format!("Plug-ins in this setup: {}", names.join(", "))),
            apps: group.applications.iter().filter(|app| app.details["choose_application"] != true).map(|app| {
                let journey = serde_json::from_value::<crate::model::InstallationJourney>(app.details["journey"].clone()).ok();
                AppCard {
                    id: app.id.clone(),
                    name: app.name.clone(),
                    progress: journey.as_ref().map(|journey| (journey::current_label(journey, current),
                        Results::build(journey, current, journey::continuation_offer(&app.actions)))),
                    hint: journey.is_none().then_some("Open this app to install or manage its plug-ins."),
                    notes: vendor_default_notice(app).into_iter().chain(app.actions.iter().find_map(|offer|
                        product::vendor_install_consequence(snapshot, &offer.action))).collect(),
                    // Show only while the app runs, and a refused Stop only while it is
                    // not closed: a window rule over the manager's offers (#489).
                    offers: offers::row(&app.actions.iter().filter(|offer| match offer.action {
                        Action::VendorApplicationFocus { .. } => matches!(app.state.as_str(), "starting" | "running"),
                        Action::VendorApplicationStop { .. } => app.state != "closed" || offer.disabled_reason.is_none(),
                        _ => true,
                    }).cloned().collect::<Vec<_>>(), Importance::Secondary, busy, pending),
                }
            }).collect(),
            picker: (!choices.is_empty()).then(|| ProgramPicker {
                title: "Choose the app to open",
                hint: "Select the installed program you want to use in this setup.",
                placeholder: "Choose installed program",
                choices: choices.iter().map(|app| Choice { key: app.id.clone(), label: app.name.clone(),
                    content: app.actions.iter().find(|offer| matches!(offer.action, Action::VendorSoftwareInstall { .. }))
                        .map(|offer| ProgramOpen { consequence: product::vendor_install_consequence(snapshot, &offer.action),
                            offer: offers::offer(offer, Importance::Primary, busy, pending) }) }).collect(),
                unchosen: "Open selected app",
                forget_missing: fresh,
            }),
            actionable: fresh,
        }
    }
}

/// Setups the musician deleted, with any deletion still to finish.
#[derive(Clone, Debug)]
pub struct DeletedSetups {
    pub title: String,
    pub start_open: bool,
    pub setups: Vec<DeletedSetupView>,
    pub actionable: bool,
}

#[derive(Clone, Debug)]
pub struct DeletedSetupView {
    pub environment: String,
    pub name: String,
    pub state: &'static str,
    pub plugins: Option<String>,
    pub offers: OfferRow,
}

impl DeletedSetups {
    pub fn build(snapshot: &Snapshot, fresh: bool, pending: bool) -> Option<Self> {
        if snapshot.deleted_setups.is_empty() { return None; }
        let busy = snapshot.system.inactive_reason();
        let view = |deleted: &DeletedSetup| DeletedSetupView {
            environment: deleted.environment.clone(),
            name: deleted.name.clone(),
            state: match deleted.state.as_str() {
                "deleting" => "Deleting didn't finish. Finish it to remove the rest of its Windows files.",
                "unreadable" => "Its history record can't be read. The setup stays hidden; send Setup details.",
                _ => "Deleted. Its Windows files are gone; its history is kept.",
            },
            plugins: (!deleted.plugins.is_empty()).then(|| format!("Plug-ins it held: {}", deleted.plugins.join(", "))),
            offers: offers::row(&deleted.actions, Importance::Secondary, busy, pending),
        };
        Some(Self {
            title: format!("Deleted setups ({})", snapshot.deleted_setups.len()),
            start_open: snapshot.deleted_setups.iter().any(|deleted| deleted.state == "deleting"),
            setups: snapshot.deleted_setups.iter().map(view).collect(),
            actionable: fresh,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_default_notice_names_only_what_the_bridge_did() {
        let app = |result: &str, reason: Option<&str>| VendorApplication { id: "a".into(), name: "Native Access".into(),
            version: "3".into(), state: "running".into(), actions: vec![],
            details: serde_json::json!({"dependency": {"registry_defaults":
                [{"known_folder": "Downloads", "result": result, "reason": reason}]}}) };
        let set = "Native Access's download folder was empty, so the bridge set it to the Downloads folder inside Native Access's Windows setup.";
        assert_eq!(vendor_default_notice(&app("applied", None)).unwrap(), set);
        assert_eq!(vendor_default_notice(&app("applied_after_service_start", None)).unwrap(),
            format!("{set} If downloads still fail, close Native Access and open it again once."));
        assert_eq!(vendor_default_notice(&app("failed", Some("timeout"))).unwrap(),
            "Couldn't set Native Access's download folder; set it in Native Access's preferences before downloading.");
        assert_eq!(vendor_default_notice(&app("failed", Some("query_unreadable"))).unwrap(),
            "The bridge couldn't read Native Access's download folder, so it left it unchanged. If downloads fail, set it in Native Access's preferences.");
        assert_eq!(vendor_default_notice(&app("failed", Some("cancelled"))), None);
        assert_eq!(vendor_default_notice(&app("already_set", None)), None);
        let mut unreadable = app("failed", Some("defaults_unavailable"));
        unreadable.details["dependency"]["registry_defaults"][0]["known_folder"] = serde_json::Value::Null;
        assert_eq!(vendor_default_notice(&unreadable).unwrap(),
            "Couldn't set Native Access's download folder; set it in Native Access's preferences before downloading.");
        let mut plain = app("applied", None);
        plain.details = serde_json::json!({"dependency": {"registry_defaults": null}});
        assert_eq!(vendor_default_notice(&plain), None);
        plain.details = serde_json::json!({"dependency": null});
        assert_eq!(vendor_default_notice(&plain), None);
    }

    use crate::operator::Page;
    use crate::preview_states::{self, STATES};
    use crate::screen_model::offers::Availability;

    fn sample(state: &str) -> Snapshot {
        preview_states::snapshot(Page::Setup, Some(state))
    }

    fn rows(details: &[Detail]) -> Vec<&OfferRow> {
        details.iter().flat_map(|detail| match detail {
            Detail::Offers(row) | Detail::Held { row, .. } => vec![row],
            Detail::Disclosure(inner) => rows(&inner.content),
            _ => vec![],
        }).collect()
    }

    fn controls_rows(controls: &SetupControls) -> Vec<&OfferRow> {
        let mut out = vec![&controls.programs, &controls.running, &controls.continuation];
        out.extend(controls.runtime.iter().flat_map(|runtime| runtime.choices.iter().map(|choice| &choice.content)));
        for choice in &controls.installers.choices {
            out.push(&choice.content.0);
            out.extend(choice.content.1.iter().flat_map(|advanced| rows(&advanced.content)));
        }
        out
    }

    /// Every manager offer on Setup, with the reasons printed beside it.
    fn main(offer: &Offer) -> (&Offer, Vec<&str>) {
        (offer, offer.availability.reason().into_iter().collect())
    }

    fn offers(setup: &Setup) -> Vec<(&Offer, Vec<&str>)> {
        let mut out: Vec<(&Offer, Vec<&str>)> = Vec::new();
        let mut all_rows: Vec<&OfferRow> = Vec::new();
        for card in &setup.cards {
            out.extend(card.primary.iter().map(main));
            all_rows.push(&card.secondary);
            all_rows.extend(card.latest.iter().map(|(_, row)| row));
            all_rows.extend(rows(&card.details.content));
            all_rows.extend(card.removal.iter().flat_map(|removal| rows(&removal.content)));
        }
        for existing in setup.existing.iter().flat_map(|existing| &existing.setups) {
            all_rows.extend(controls_rows(&existing.content.controls));
            all_rows.extend(existing.content.programs.iter().map(|program| &program.offers));
            all_rows.extend(existing.content.stop.iter());
        }
        for group in &setup.apps {
            all_rows.extend(group.apps.iter().map(|app| &app.offers));
            out.extend(group.picker.iter().flat_map(|picker| picker.choices.iter())
                .filter_map(|choice| choice.content.as_ref()).map(|open| main(&open.offer)));
        }
        all_rows.extend(setup.deleted.iter().flat_map(|deleted| deleted.setups.iter().map(|setup| &setup.offers)));
        for row in all_rows {
            out.extend(row.offers.iter().map(|offer| (offer, row.notes.iter().map(String::as_str).collect())));
        }
        out
    }

    fn manager_offers(snapshot: &Snapshot) -> Vec<&Action> {
        let mut offered: Vec<&Action> = Vec::new();
        for setup in &snapshot.installer_setups {
            offered.extend(setup.primary.iter().chain(&setup.secondary).chain([&setup.rename]).map(|offer| &offer.action));
        }
        for target in &snapshot.environment_installers {
            offered.extend(target.actions.iter().chain(&target.choices)
                .chain(target.runtime.iter().flat_map(|runtime| &runtime.choices)).map(|offer| &offer.action));
        }
        offered.extend(snapshot.vendor_applications.iter().flat_map(|app| &app.actions).map(|offer| &offer.action));
        offered.extend(snapshot.onboarding.iter().flat_map(|attempt| &attempt.actions).map(|offer| &offer.action));
        offered.extend(snapshot.uninstallers.iter().map(|uninstaller| &uninstaller.action.action));
        offered.extend(snapshot.deleted_setups.iter().flat_map(|deleted| &deleted.actions).map(|offer| &offer.action));
        offered
    }

    #[test]
    fn sample_states_offer_only_manager_actions_with_every_refusal_visible() {
        let mut checked = 0;
        for state in STATES {
            let snapshot = sample(state);
            let offered = manager_offers(&snapshot);
            for (fresh, pending) in [(true, false), (true, true), (false, false)] {
                let setup = Setup::build(&snapshot, fresh, pending, Focus::default());
                for (offer, notes) in offers(&setup) {
                    assert!(offered.contains(&&offer.intent), "{state}: {} is not a manager offer", offer.label);
                    if let Some(reason) = offer.availability.reason() {
                        assert!(notes.contains(&reason), "{state}: {} hides {reason}", offer.label);
                    }
                    if pending { assert!(!offer.availability.enabled(), "{state}: {} pressable while pending", offer.label); }
                    checked += 1;
                }
                // Without current status every offer is shown but held.
                assert!(setup.cards.iter().all(|card| card.actionable == fresh), "{state}");
                assert_eq!(setup.add.enabled, fresh && !pending, "{state}");
                assert_eq!(setup.cards.len(), snapshot.installer_setups.len(), "{state}");
            }
        }
        assert!(checked > 300, "only {checked} offers checked");
    }

    #[test]
    fn cards_list_what_needs_the_musician_first_and_narrow_to_one_installer() {
        let snapshot = sample("install_running");
        let setup = Setup::build(&snapshot, true, false, Focus::default());
        assert_eq!(setup.cards[0].title, "Vendor installer is open");
        assert!(setup.showing_one.is_none() && setup.missing.is_none() && setup.empty.is_empty());
        let one = setup.cards[2].installer.clone();
        let narrowed = Setup::build(&snapshot, true, false, Focus { installer: Some(&one), ..Focus::default() });
        assert_eq!(narrowed.cards.len(), 1);
        assert_eq!(narrowed.showing_one.as_ref().unwrap().1.does, Local::ShowAllSetup);
        let gone = Setup::build(&snapshot, true, false, Focus { installer: Some("missing"), ..Focus::default() });
        assert_eq!(gone.missing, Some(MISSING));
        let empty = Setup::build(&sample("first_use"), true, false, Focus::default());
        assert_eq!(empty.empty, EMPTY);
        let stale = Setup::build(&snapshot, false, false, Focus::default());
        assert_eq!(stale.progress, Text::caution_hint("Refreshing setup · actions wait for current status"));
    }

    #[test]
    fn install_progress_names_each_plug_in_and_the_continuation_its_card_shows() {
        let setup = Setup::build(&sample("install_attention"), true, false, Focus::default());
        let card = setup.cards.iter().find(|card| card.name == "Example Audio Bundle").unwrap();
        let journey = card.journey.as_ref().unwrap();
        assert_eq!(journey.daw, "Your DAW: Bitwig");
        assert_eq!(journey.problem.as_deref(),
            Some("DAW connection: The manager was busy finishing another task; choose \u{201c}Continue installation and DAW connection\u{201d}."));
        let stages: Vec<_> = journey.plugins.iter().map(|plugin| plugin.status.words).collect();
        assert_eq!(stages, ["Available in your DAW", "Connection needs attention", "A different setup is already available"]);
        assert!(journey.plugins[2].hint.is_some());
        assert_eq!(card.primary.as_ref().unwrap().label, "Continue installation and DAW connection");
        let unclosed = setup.cards.iter().find(|card| card.name == "Example Drums").unwrap();
        assert_eq!(unclosed.title, "Could not confirm the installer closed");
        assert!(matches!(unclosed.replace, Replace::Waiting(_)), "no other installer before cleanup");
    }

    #[test]
    fn required_apps_and_program_choices_keep_their_own_setup_and_refusal() {
        let snapshot = sample("install_app");
        let setup = Setup::build(&snapshot, true, false, Focus::default());
        let group = setup.apps.iter().find(|group| group.environment == "e4".repeat(16)).unwrap();
        assert_eq!(group.title, "Apps for Example Hub installer");
        let hub = group.apps.iter().find(|app| app.name == "Example Hub").unwrap();
        assert_eq!(hub.progress.as_ref().unwrap().0, "Continue in the installed app");
        assert!(hub.notes[0].starts_with("Example Hub's download folder was empty"));
        assert!(hub.notes[1].starts_with("Hub Synth will be unavailable"));
        let tool = group.apps.iter().find(|app| app.name == "Example Tool").unwrap();
        assert_eq!(tool.progress.as_ref().unwrap().0, "Application session complete");
        assert!(tool.offers.offers.iter().all(|offer| offer.label != "Stop Example Tool"),
            "a closed app's refused Stop is not shown");
        let picker = group.picker.as_ref().unwrap();
        assert_eq!(picker.choices.len(), 2);
        let library = picker.choices.iter().find(|choice| choice.label == "Example Library").unwrap();
        assert_eq!(library.content.as_ref().unwrap().offer.availability,
            Availability::Refused("Close Example Hub first".into()));
        // The hub's install offer is its card's main step; the program choice offers the others.
        let card = setup.cards.iter().find(|card| card.name == "Example Hub installer").unwrap();
        assert_eq!(card.primary.as_ref().unwrap().label, "Install plug-ins with Example Hub");
    }

    #[test]
    fn existing_setups_offer_their_runtime_and_installers_with_exact_offers() {
        let snapshot = sample("existing_setup");
        let setup = Setup::build(&snapshot, true, false, Focus::default());
        let existing = setup.existing.as_ref().unwrap();
        let ni = &existing.setups[0];
        assert_eq!((ni.key.as_str(), ni.label.as_str()), ("fixture-ni", "Native Instruments setup"));
        let runtime = ni.content.controls.runtime.as_ref().unwrap();
        assert_eq!(runtime.current, "Current runtime: managed-ge-proton11-7-slr4-20260805-r5 · setup revision 3");
        assert_eq!(runtime.choices.len(), 2);
        assert!(matches!(&runtime.choices[0].content.offers[0].intent,
            Action::EnvironmentRuntimeSelect { runner, .. } if *runner == runtime.choices[0].key));
        assert_eq!(runtime.choices[1].content.notes, ["Runtime r4 is older than this setup"]);
        assert!(runtime.none.is_none());
        let installers = &ni.content.controls.installers;
        assert_eq!(installers.preselected.as_deref(), Some("b1".repeat(32).as_str()));
        assert_eq!(installers.choices.len(), 1, "the PowerShell variant is an advanced option");
        assert!(installers.choices[0].content.1.is_some());
        let last = ni.content.controls.last_installer.as_ref().unwrap();
        assert_eq!(last.state, "Last installer: failed");
        assert!(last.warning.is_some());
        assert_eq!(ni.content.programs[0].name, "Example Editor");
        let arturia = &existing.setups[1];
        assert!(arturia.content.stop.is_some(), "a running installer's Stop stays reachable");
        assert_eq!(arturia.content.controls.runtime.as_ref().unwrap().none,
            Some("To download Windows plug-in support, open Diagnostics → Manager actions."));
        // The Kontakt setup can take a required app: the last run is retired.
        let card = setup.cards.iter().find(|card| card.name == "Native Instruments").unwrap();
        let Replace::Allowed(choices) = &card.replace else { panic!("a replacement choice") };
        let app = choices.dependency.as_ref().unwrap();
        assert_eq!(app.help, ["Kontakt asks for its licence service on first start. Install it here when asked."]);
        assert!(matches!(&app.choose.does, Local::ChooseReplacement(draft) if draft.original == card.installer));
        assert!(card.latest.as_ref().unwrap().0.ends_with("failed"));
    }

    #[test]
    fn removal_deleted_setups_and_uninstallers_keep_every_offer_and_reason() {
        let snapshot = sample("setup_removal");
        let setup = Setup::build(&snapshot, true, false, Focus::default());
        let serum = setup.cards.iter().find(|card| card.name == "Xfer Serum 2").unwrap();
        let removal = serum.removal.as_ref().unwrap();
        let labels: Vec<_> = rows(&removal.content).iter().flat_map(|row| &row.offers).map(|offer| offer.label.as_str()).collect();
        assert_eq!(labels, ["Uninstall Serum 2", "Uninstall Xfer Shared Runtime", "Remove from my DAW", "Delete setup"]);
        assert!(rows(&removal.content)[1].notes.contains(&"Close Serum 2 FX first".to_owned()));
        assert!(serum.secondary.offers.is_empty(), "removal lives in its own section");
        let unreadable = setup.cards.iter().find(|card| card.title == "Ready to set up").unwrap();
        assert_eq!(unreadable.status.role, crate::screen_model::TextRole::Caution);
        let deleted = setup.deleted.as_ref().unwrap();
        assert_eq!(deleted.title, "Deleted setups (3)");
        assert!(deleted.start_open);
        assert_eq!(deleted.setups[0].offers.offers[0].label, "Finish deleting");
        assert!(deleted.setups[1].offers.offers.is_empty());
        let policy = setup.cards.iter().find(|card| card.name == "Example Strings installer").unwrap();
        let advanced: Vec<_> = rows(&policy.details.content).iter().flat_map(|row| &row.offers)
            .map(|offer| offer.label.clone()).collect();
        assert!(advanced.contains(&"Install · PowerShell unavailable".to_owned()));
        assert!(advanced.contains(&"Start · PowerShell unavailable".to_owned()));
    }
}

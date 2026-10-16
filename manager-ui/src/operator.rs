use crate::{
    client::{self, Query, Reply},
    model::*,
    presentation::{self, AttemptKey, Destination, ProductKey},
};
use eframe::egui;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
fn installation_label(product: &ReadinessProduct) -> &'static str {
    let records = product.facts.iter().any(|fact| fact.name == "Validation scope"
        && fact.value.as_deref() == Some("control_records"));
    match (records, product.installation_health) {
        (true, InstallationHealth::Healthy) => "Current configuration: records and publication agree",
        (true, InstallationHealth::ActionRequired) => "Current configuration: needs attention",
        (true, InstallationHealth::Unknown) => "Current configuration: records incomplete",
        (false, InstallationHealth::Healthy) => "Current installation: healthy",
        (false, InstallationHealth::ActionRequired) => "Current installation: needs repair",
        (false, InstallationHealth::Unknown) => "Current installation: not verified",
    }
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub enum Page {
    #[default]
    Home,
    Plugins,
    Workspaces,
    Activity,
    Setup,
    Diagnostics,
}

impl Page {
    fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Plugins => "Plug-ins",
            Self::Workspaces => "Workspaces",
            Self::Activity => "Activity",
            Self::Setup => "Setup",
            Self::Diagnostics => "Diagnostics",
        }
    }
}

fn workspace_state_label(state: &str) -> &str {
    match state {
        "imported" => "Ready to install",
        "installing" => "Installing",
        "needs_user_action" => "Needs your attention",
        "installed" => "Installed",
        "ready" => "Ready",
        "starting" => "Starting",
        "running" => "Running",
        "stopping" => "Closing",
        "uninstalling" => "Uninstalling",
        "uninstalled" => "Uninstalled",
        "failed" => "Needs your attention",
        "cleanup_unconfirmed" => "Cleanup unconfirmed",
        _ => "State unavailable",
    }
}

fn workspace_product_state_label(state: &str) -> &str {
    match state {
        "not_selected" => "Not installed",
        "selected" => "Ready to install",
        "installing" => "Installing",
        "installed" => "Installed",
        "needs_user_action" | "needs_attention" | "failed" => "Needs your attention",
        _ => "State unavailable",
    }
}

fn plausible_installer_label(label: &str) -> bool {
    !label.trim().is_empty()
        && label.trim() == label
        && label.len() <= 96
        && !matches!(label, "." | "..")
        && !label.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':')
            || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
}
fn test_areas() -> [TestArea; 9] {
    [TestArea::DawLoad, TestArea::Midi, TestArea::Audio, TestArea::Editor,
        TestArea::Parameters, TestArea::Automation, TestArea::StateRecall,
        TestArea::ProcessingRestart, TestArea::Retirement]
}
fn test_area_label(area: TestArea) -> &'static str {
    match area {
        TestArea::DawLoad => "Loaded in the DAW", TestArea::Midi => "MIDI or note input",
        TestArea::Audio => "Audio or musical processing", TestArea::Editor => "Editor rendered and was usable",
        TestArea::Parameters => "Controls responded", TestArea::Automation => "Automation responded",
        TestArea::StateRecall => "Save and reopen recalled state",
        TestArea::ProcessingRestart => "Processing restarted cleanly",
        TestArea::Retirement => "Editor and DAW closed cleanly",
    }
}
fn problem_category_label(category: ProblemCategory) -> &'static str {
    match category {
        ProblemCategory::FailedToLoad => "Failed to load",
        ProblemCategory::BlankEditor => "Blank or white editor",
        ProblemCategory::EditorFroze => "Editor froze",
        ProblemCategory::EditorDisappeared => "Editor disappeared",
        ProblemCategory::NoAudio => "No audio",
        ProblemCategory::IncorrectAudio => "Incorrect audio",
        ProblemCategory::ControlsUnresponsive => "Controls did not respond",
        ProblemCategory::HostCrashed => "Plug-in or host crashed",
        ProblemCategory::CleanupIncomplete => "Cleanup did not complete",
        ProblemCategory::Other => "Other problem",
    }
}

fn workspace_primary_action(actions: &[AvailableAction]) -> Option<usize> {
    actions
        .iter()
        .position(|action| action.disabled_reason.is_none())
        .or_else(|| (!actions.is_empty()).then_some(0))
}
#[derive(Clone, Debug)]
struct RequestFeedback {
    action: Action,
    operation: Option<String>,
    text: String,
    terminal: bool,
    blocking: bool,
    release_after_snapshot: bool,
    acknowledgment_uncertain: bool,
    journey: Option<InstallationJourney>,
    transitions: Vec<String>,
}
impl RequestFeedback {
    fn captured(action: Action) -> Self {
        Self {
            action,
            operation: None,
            text: "Click received. Waiting for the current readback; no operation submitted yet."
                .into(),
            terminal: false,
            blocking: true,
            release_after_snapshot: false,
            acknowledgment_uncertain: false,
            journey: None,
            transitions: vec!["captured".into()],
        }
    }
    fn transition(&mut self, state: &str) {
        if self.transitions.last().is_none_or(|s| s != state) {
            if self.transitions.len() == 12 {
                self.transitions.remove(0);
            }
            self.transitions.push(state.into());
        }
    }
    fn receipt(&mut self, r: &Receipt) {
        self.acknowledgment_uncertain = r.accepted && r.operation.is_none();
        self.transition(if r.accepted { "accepted" } else { "refused" });
        self.operation = r.operation.clone();
        self.terminal = !r.accepted;
        self.release_after_snapshot = !r.accepted;
        self.text = if r.accepted {
            "Request accepted. Waiting for its result.".into()
        } else {
            format!(
                "This request was refused: {}",
                r.refusal.as_deref().unwrap_or("reason unavailable")
            )
        };
    }
    fn observe(&mut self, op: &serde_json::Value) {
        if self.operation.is_some() && self.operation.as_deref() == op["operation"].as_str()
            && (matches!(self.action,Action::SoftwareInstall { .. } | Action::SoftwareInstallWithPolicy {..} | Action::VendorSoftwareInstall { .. }
                | Action::SoftwareInstallResume { .. } | Action::RendererSoftwareInstall { .. })
                || matches!(&self.action,Action::EnvironmentRefreshResume {operation}
                    if op["renderer_owner"].as_str()==Some(operation.as_str()))) {
            if let Ok(journey)=serde_json::from_value::<InstallationJourney>(op["journey"].clone()) {
                if journey.phase == InstallationPhase::AwaitingApplication && journey.failure.is_none() {
                    self.journey=Some(journey);self.transition("awaiting_application");
                    self.terminal=true;self.blocking=false;self.release_after_snapshot=false;
                    self.text="The app is installed. Open it below to install your instruments and effects.".into();
                    return;
                }
            }
        }
        if self.terminal
            || self.operation.as_deref() != op["operation"].as_str()
            || self.operation.is_none()
        {
            return;
        }
        if let Ok(journey) = serde_json::from_value::<InstallationJourney>(op["journey"].clone()) {
            self.journey = Some(journey);
        }
        if let Some(
            state @ ("refused"
            | "completed"
            | "waiting"
            | "validating"
            | "vendor_running"
            | "queued"
            | "running"
            | "submission_uncertain"),
        ) = op["state"].as_str()
        {
            self.transition(state);
        }
        self.text = match op["state"].as_str() {
            Some("refused") => {
                self.terminal = true;
                self.release_after_snapshot = true;
                format!(
                    "This request was refused: {}",
                    op["reason"].as_str().unwrap_or("reason unavailable")
                )
            }
            Some("completed") => {
                self.terminal = true;
                self.release_after_snapshot = true;
                if let Some(message) = op["result"]["message"].as_str().filter(|_| matches!(self.action,
                    Action::PluginRemoveFromDaw { .. } | Action::PluginAddBackToDaw { .. }
                    | Action::SetupRemoveFromDaw { .. } | Action::ProductUninstall { .. }
                    | Action::InstallerCloseCheck { .. })) {
                    message.into()
                } else if self.journey.as_ref().is_some_and(|journey|
                    journey.phase == InstallationPhase::NeedsAttention) {
                    "Installation or DAW connection needs attention. Refreshing its results…".into()
                } else { "This operation completed. Refreshing its result…".into() }
            }
            Some("waiting" | "validating") => {
                "This operation is validating manager state. Do not click again.".into()
            }
            Some("submission_uncertain") => {
                self.release_after_snapshot = true;
                "Launch acknowledgment is uncertain. Use the exact Stop / reconcile control; no launch was resubmitted.".into()
            }
            Some("vendor_running") if matches!(self.action, Action::ProductUninstall { .. }) => {
                self.release_after_snapshot = true;
                "The vendor's uninstaller is running. Follow its window to finish.".into()
            }
            Some("vendor_running") => {
                if self.blocking {
                    self.release_after_snapshot = true;
                    "Vendor operation is running. Refreshing its controls…".into()
                } else {
                    "The operation is supervised and running. Use its exact Focus or Stop control below.".into()
                }
            }
            Some("queued") => "This operation is queued. Do not click again.".into(),
            Some("running") => match &self.action {
                Action::SoftwareInstall { .. } | Action::SoftwareInstallWithPolicy {..} | Action::VendorSoftwareInstall { .. }
                    | Action::SoftwareInstallResume { .. } | Action::RendererSoftwareInstall {..} | Action::EnvironmentRefresh { .. }
                    | Action::EnvironmentRefreshResume { .. } =>
                    self.journey.as_ref().map(journey_phase_label)
                        .unwrap_or("Installing and connecting your plug-ins").into(),
                Action::RuntimeInstall {} =>
                    "Downloading and preparing the compatibility runtime. This may take several minutes.".into(),
                Action::EnvironmentRescan { .. } =>
                    "Checking the installed products. The bridge will resume when this finishes.".into(),
                Action::EnvironmentRuntimeSelect { .. } =>
                    "Changing the setup's runtime and checking its installed plug-ins. Next, choose Prepare for your DAW for each plug-in.".into(),
                Action::CompatibilityCheck { .. } | Action::CompatibilityResumeCheck { .. } =>
                    "Checking this plug-in and preparing its matching bridge.".into(),
                Action::PluginPrepare { .. } => "Preparing this plug-in's matching bridge.".into(),
                Action::CandidateGraphicsAssess { .. } => "Assessing graphics in this plug-in's selected runtime.".into(),
                Action::CandidateGraphicsPrepare { .. } => "Preparing a reversible graphics settings trial.".into(),
                Action::CandidateSettingsPrepare { .. } => "Preparing a reversible compatibility settings trial. Your current selection stays in place.".into(),
                Action::CompatibilityPublishTest { .. } => "Selecting this plug-in's prepared bridge.".into(),
                Action::PluginRemoveFromDaw { .. } => "Removing this plug-in from your DAW.".into(),
                Action::PluginAddBackToDaw { .. } => "Adding this plug-in back to your DAW.".into(),
                Action::SetupRemoveFromDaw { .. } => "Removing this setup's plug-ins from your DAW.".into(),
                Action::SetupDelete { .. } => "Deleting this setup's Windows files. A large setup can take a minute.".into(),
                // Once the uninstaller closed, the setup's own progress.
                Action::ProductUninstall { .. } | Action::InstallerCloseCheck { .. }
                    if op["progress"].as_str().is_some() => op["progress"].as_str().unwrap_or_default().into(),
                Action::ProductUninstall { .. } =>
                    "Taking the affected plug-ins out of your DAW and starting the uninstaller.".into(),
                Action::InstallerCloseCheck { .. } => "Checking that it closed.".into(),
                _ => "This operation is in progress. Do not click again.".into(),
            },
            _ => return,
        };
    }
    fn refreshed(&mut self) {
        if self.release_after_snapshot {
            self.blocking = false;
            self.release_after_snapshot = false;
            if self.text == "This operation completed. Refreshing its result…" {
                self.text = "This operation completed. Its current result is shown below.".into();
            } else if self.text == "Installation or DAW connection needs attention. Refreshing its results…" {
                self.text = "Installation or DAW connection needs attention. Its results and continuation are shown below.".into();
            } else if self.text == "Vendor operation is running. Refreshing its controls…" {
                self.text = "The operation is supervised and running. Use its exact Focus or Stop control below.".into();
            }
        }
    }
    fn reconcile_snapshot(&mut self, s: &Snapshot) {
        // A lost acknowledgment is not a failed operation. Only exact canonical
        // onboarding ownership plus its offered recovery action can resolve it.
        if self.acknowledgment_uncertain && self.operation.is_none() {
            if let Action::InstallerStart { onboarding }
            | Action::InstallerStartWithPolicy { onboarding, .. } = &self.action
            {
                let owners: Vec<_> = s.onboarding.iter().filter(|r| r.environment.as_ref() == Some(onboarding))
                    .filter_map(|r| {
                        let op = r.details["installation"]["operation"].as_str()?;
                        let exact = r.actions.iter().any(|a| matches!(&a.action,
                            Action::InstallerStop { onboarding: id, operation } if id == onboarding && operation == op));
                        (exact && matches!(r.details["installation"]["state"].as_str().unwrap_or(r.state.as_str()), "starting" | "running" | "unknown")).then_some(op.to_owned())
                    }).collect();
                if owners.len() == 1 {
                    self.operation = Some(owners[0].clone());
                    self.transition("live_owner_reconciled");
                    self.release_after_snapshot = true;
                    self.text = "Submission acknowledgment was unavailable. The manager confirms this exact live installer; recovery controls are available. No request was resubmitted.".into();
                }
            }
        }
        let mut renderer_recovery = false;
        if let Action::RendererOpen {application,policy}
            | Action::RendererSoftwareInstall {application,policy,..}=self.action.clone()
        {
            for app in &s.vendor_applications {
                if app.details["application_identity"].as_str() == Some(application.as_str()) {
                    if self.acknowledgment_uncertain
                        && self.operation.is_none()
                        && app.details["requested"] == serde_json::to_value(policy).unwrap()
                        && matches!(
                            app.details["manager_operation"]["state"].as_str(),
                            Some("vendor_running" | "submission_uncertain")
                        )
                    {
                        if let Some(op) = app.details["operation"].as_str() {
                            if app.actions.iter().any(|a|matches!(&a.action,Action::RendererStop{operation} if operation==op)) {
                                self.operation=Some(op.into());self.release_after_snapshot=true;
                            }
                        }
                    }
                    if let Some(op) = app.details.get("manager_operation") {
                        // RendererOpen ends with its session; automatic installation
                        // continues into discovery/connection under the operator owner.
                        // A retired renderer child cannot complete that larger job.
                        if matches!(self.action,Action::RendererOpen { .. })
                            || matches!(op["state"].as_str(),Some("vendor_running"|"submission_uncertain")) {
                            renderer_recovery = self.operation.is_some()
                                && op["operation"].as_str() == self.operation.as_deref();
                            self.observe(op);
                        }
                    }
                }
            }
        }
        if matches!(self.action, Action::DependencyPrepare {}) {
            for app in &s.vendor_applications {
                if app.id != "native-access" {
                    continue;
                }
                let record = &app.details["dependency_manager_operation"];
                if let Some(op) = record["operation"].as_str() {
                    let exact = app.actions.iter().any(
                        |a| matches!(&a.action,Action::DependencyStop{operation} if operation==op),
                    );
                    if exact
                        && self.acknowledgment_uncertain
                        && self.operation.is_none()
                        && matches!(
                            record["state"].as_str(),
                            Some("submission_uncertain" | "vendor_running")
                        )
                    {
                        self.operation = Some(op.into());
                        self.release_after_snapshot = true;
                    }
                    if exact && self.operation.as_deref() == Some(op) {
                        renderer_recovery = true;
                        self.observe(record);
                    }
                }
            }
        }
        let matching_operation = self.operation.is_some()
            && s.operation
                .as_ref()
                .is_some_and(|op| op["operation"].as_str() == self.operation.as_deref());
        if let Some((environment,installer,_)) = self.action.environment_installer_start() {
            if let Some(target) = s.environment_installers.iter().find(|row|
                row.environment == environment && row.operation.as_deref() == self.operation.as_deref()) {
                self.observe(&target.result);
            } else if self.acknowledgment_uncertain && self.operation.is_none() {
                let owners:Vec<_> = s.environment_installers.iter().filter(|row|row.environment == environment
                    && row.installer.as_deref() == Some(installer))
                    .filter_map(|row|row.operation.as_ref().filter(|op|row.actions.iter().any(|offer|
                        matches!(&offer.action,Action::EnvironmentInstallerStop {operation} if operation == *op))))
                    .cloned().collect();
                if owners.len() == 1 {
                    // The exact start request in the live owner is checked by the
                    // manager. Never infer ownership from a display name.
                    self.operation = Some(owners[0].clone());
                    self.release_after_snapshot = true;
                }
            }
        }
        let exact_recovery = if let Action::InstallerStart { onboarding }
        | Action::InstallerStartWithPolicy { onboarding, .. } = &self.action
        {
            s.onboarding.iter().any(|r| {
                r.environment.as_ref() == Some(onboarding)
                    && r.actions.iter().any(|a| {
                        matches!(&a.action, Action::InstallerStop{onboarding:id,operation}
                    if id==onboarding && Some(operation.as_str())==self.operation.as_deref())
                    })
            })
        } else if let Some((environment,_,_)) = self.action.environment_installer_start() {
            s.environment_installers.iter().any(|row|row.environment == environment
                && row.actions.iter().any(|offer|matches!(&offer.action,
                    Action::EnvironmentInstallerStop {operation} if Some(operation.as_str()) == self.operation.as_deref())))
        } else {
            false
        };
        if exact_recovery && self.action.environment_installer_start().is_some() {
            self.release_after_snapshot = true;
            self.text = "The installer is running in this setup. Show or stop its window below.".into();
        }
        for product in &s.products {
            if let Some(op) = product.details["preparation"].get("operation") {
                self.observe(op);
            }
            if let Some(rows) = product.details["preparation"]["candidates"].as_array() {
                for row in rows {
                    if let Some(op) = row.get("operation") {
                        self.observe(op);
                    }
                }
            }
        }
        if let Some(op) = &s.operation {
            self.observe(op);
        }
        if self.terminal || matching_operation || exact_recovery || renderer_recovery {
            self.refreshed();
        }
    }

}
#[derive(Default)]
struct RouteFocus {
    setup: Option<AttemptKey>,
    setup_installer: Option<String>,
    setup_scroll: bool,
    activity: Option<String>,
    replacement: Option<InstallerReplacement>,
}

// This is a local file-choice draft, not installation authority. Submission
// still requires the current manager offer for these exact bytes and setup.
#[derive(Clone, Debug)]
struct InstallerReplacement {
    original: String,
    environment: Option<String>,
    runtime: Option<(String, u64)>,
    selected: Option<String>,
}
fn replacement_context(snapshot: &Snapshot, setup: &InstallerSetup) -> Option<InstallerReplacement> {
    if setup.phase == SetupPhase::CleanupUnconfirmed || setup.primary.iter().chain(&setup.secondary)
        .any(|offer| matches!(offer.action, Action::InstallerFocus {..} | Action::InstallerStop {..}
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
fn replacement_offer(snapshot: &Snapshot, draft: &InstallerReplacement)
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
fn imported_destination(snapshot: &Snapshot, installer: &str) -> Option<Page> {
    let setups = snapshot.installer_setups.iter().filter(|setup| setup.installer == installer).count();
    let workspaces = snapshot.workspaces.iter().filter(|workspace|
        workspace.application_installers.iter().any(|id| id == installer)).count();
    match (setups, workspaces) {
        (1, 0) => Some(Page::Setup),
        (0, 1) => Some(Page::Workspaces),
        _ => None,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestOrigin {
    InitialSnapshot,
    ProductDetail,
    DetailSnapshot,
    BackgroundActivity,
    SilentPostMutationRefresh,
    ExplicitRefresh,
    InstallerImport,
    UserAction,
}
impl RequestOrigin {
    fn foreground(self) -> bool {
        matches!(self, Self::InstallerImport | Self::UserAction)
    }
}

fn pulse_posture_changed(previous: &crate::model::Pulse, current: &crate::model::Pulse) -> bool {
    previous.current_generation != current.current_generation
        || previous.service_state != current.service_state
        || previous.dsp != current.dsp
        || previous.keepers != current.keepers
        || previous.maintenance != current.maintenance
        || previous.pending_transactions != current.pending_transactions
        || previous.cleanup_unconfirmed != current.cleanup_unconfirmed
}
fn pulse_system_changed(pulse: &crate::model::Pulse, system: &System) -> bool {
    pulse.service_state != "active"
        || pulse.dsp != Some(system.dsp)
        || pulse.keepers != Some(system.keepers)
        || pulse.maintenance != Some(system.maintenance)
        || pulse.pending_transactions != system.pending_transactions
        || pulse.cleanup_unconfirmed != Some(system.cleanup_unconfirmed)
}

pub struct Operator {
    snapshot: Option<Snapshot>,
    overview: Option<CurrentOverview>,
    readiness: Option<(ReadinessAssessment, Instant, String)>,
    product_detail: Option<CurrentProductDetail>,
    product_attempted: Option<ProductKey>,
    overview_fresh: bool,
    // Age expiry keeps the last observed display; a failed or changed readback does not.
    overview_renewing: bool,
    details_attempted: bool,
    sender: mpsc::Sender<Reply>,
    receiver: mpsc::Receiver<Reply>,
    window_wake: Option<client::WindowWake>,
    pending: bool,
    background_poll: bool,
    action_inflight: bool,
    frontend_superseded: bool,
    origin: Option<RequestOrigin>,
    queued_import: bool,
    explicit_refresh_queued: bool,
    queued_action: Option<Request>,
    feedback: Option<RequestFeedback>,
    last_poll: Instant,
    last_overview: Instant,
    operation_live: bool,
    prompt_pulse: bool,
    last_pulse: Option<crate::model::Pulse>,
    // An unavailable readback cannot clear a positively observed cleanup fault.
    cleanup_failure_observed: bool,
    readback_failures: u32,
    retry_after: Option<Instant>,
    message: String,
    library: crate::library::Library,
    refresh_after: bool,
    product_form: Option<Action>,
    workspace_select_form: Option<Action>,
    installer_rename_form: Option<Action>,
    /// A removal or deletion awaiting the musician's explicit confirmation.
    removal_form: Option<Action>,
    software_install_form: Option<Action>,
    software_install_daw: Option<DawChoice>,
    pending_import_route: Option<(String, bool, String)>,
    page: Page,
    focus: RouteFocus,
    preview: bool,
    updates: crate::updates::Updates,
}
impl Operator {
    pub fn new(ctx: &egui::Context, window_wake: Option<client::WindowWake>) -> Self {
        let (sender, receiver) = mpsc::channel();
        client::send(Query::Overview, sender.clone(), ctx.clone(), window_wake.clone());
        Self {
            snapshot: None,
            overview: None,
            readiness: None,
            product_detail: None,
            product_attempted: None,
            overview_fresh: false,
            overview_renewing: false,
            details_attempted: false,
            sender,
            receiver,
            window_wake,
            pending: true,
            background_poll: false,
            action_inflight: false,
            frontend_superseded: false,
            origin: Some(RequestOrigin::InitialSnapshot),
            queued_import: false,
            explicit_refresh_queued: false,
            queued_action: None,
            feedback: None,
            last_poll: Instant::now(),
            last_overview: Instant::now(),
            operation_live: false,
            prompt_pulse: false,
            last_pulse: None,
            cleanup_failure_observed: false,
            readback_failures: 0,
            retry_after: None,
            message: String::new(),
            library: crate::library::Library::default(),
            refresh_after: false,
            product_form: None,
            workspace_select_form: None,
            installer_rename_form: None,
            removal_form: None,
            software_install_form: None,
            software_install_daw: None,
            pending_import_route: None,
            page: Page::Home,
            focus: RouteFocus::default(),
            preview: false,
            updates: crate::updates::Updates::new(),
        }
    }
    /// The example owns only synthetic state; it never starts the manager client.
    #[allow(dead_code)]
    pub fn preview(snapshot: Snapshot, page: Page) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            snapshot: Some(snapshot),
            overview: None,
            readiness: None,
            product_detail: None,
            product_attempted: None,
            overview_fresh: false,
            overview_renewing: false,
            details_attempted: false,
            sender,
            receiver,
            window_wake: None,
            pending: false,
            background_poll: false,
            action_inflight: false,
            frontend_superseded: false,
            origin: None,
            queued_import: false,
            explicit_refresh_queued: false,
            queued_action: None,
            feedback: None,
            last_poll: Instant::now(),
            last_overview: Instant::now(),
            operation_live: false,
            prompt_pulse: false,
            last_pulse: None,
            cleanup_failure_observed: false,
            readback_failures: 0,
            retry_after: None,
            message: String::new(),
            library: crate::library::Library::default(),
            refresh_after: false,
            product_form: None,
            workspace_select_form: None,
            installer_rename_form: None,
            removal_form: None,
            software_install_form: None,
            software_install_daw: None,
            pending_import_route: None,
            page,
            focus: RouteFocus::default(),
            preview: true,
            updates: crate::updates::Updates::disabled(),
        }
    }
    /// Synthetic current-only overview for source previews. It never calls the manager.
    #[allow(dead_code)]
    pub fn preview_readiness(snapshot: Snapshot, readiness: ReadinessAssessment,
        page: Page) -> Self {
        Self::preview_current(snapshot, Some(readiness), page, false, false, false, false)
    }
    /// Exercise the shipped current-overview and exact product-detail paths.
    /// These are synthetic records; no request is sent to a live manager.
    pub fn preview_current(snapshot: Snapshot, readiness: Option<ReadinessAssessment>,
        page: Page, product_controls: bool, pending: bool, expand_details: bool, stale: bool) -> Self {
        let mut preview = Self::preview(snapshot.clone(), page);
        preview.readiness = readiness.map(|readiness|(readiness, Instant::now(), "preview".into()));
        preview.overview = Some(CurrentOverview {
            schema: 1, operator_schema: crate::model::OPERATOR_SCHEMA,
            scope: "current_only".into(), current_generation: "preview".into(),
            current: snapshot.clone(),
        });
        preview.overview_fresh = !stale;
        preview.library.expand_details = expand_details;
        if product_controls {
            if let Some(product) = snapshot.products.first() {
                preview.library.focus_product(ProductKey::from(product));
                preview.product_detail = Some(CurrentProductDetail {
                    schema:1,operator_schema:crate::model::OPERATOR_SCHEMA,
                    state_token:snapshot.state_token.clone(),current_generation:"preview".into(),
                    system:snapshot.system.clone(),product:product.clone(),
                    environments:snapshot.environments.clone(),
                    vendor_applications:snapshot.vendor_applications.clone(),
                });
            }
        }
        if pending {
            preview.pending = true;
            preview.origin = Some(RequestOrigin::UserAction);
            let mut feedback = RequestFeedback::captured(Action::RuntimeInstall {});
            feedback.transition("running");
            preview.feedback = Some(feedback);
        }
        preview
    }
    fn preparation_details(ui: &mut egui::Ui, v: &serde_json::Value) {
        if v["publication"] == "ordinary" {
            ui.strong("Ordinarily published");
        }
        if let Some(history) = v["candidates"].as_array() {
            egui::CollapsingHeader::new("Candidate generations and their own results").id_salt((v["selection"].as_str(),"candidate-history")).default_open(history.len()>1).show(ui,|ui|{
                for c in history {ui.group(|ui|{ui.label(format!("Candidate {} · {} · {}",c["id"].as_str().unwrap_or("?"),c["disposition"].as_str().unwrap_or("?"),c["publication"].as_str().unwrap_or("?")));if c["publication_acceptance_sealed"]==true && c["unmet_requirements"].as_array().is_some_and(|r|!r.is_empty()){ui.colored_label(egui::Color32::YELLOW,"This publication retains its original acceptance. New evidence needs review; use Withdraw to disable it.");}Self::value(ui,c);});}
            });
        }
        if let Some(reason) = v["recovery"].as_str() {
            ui.colored_label(egui::Color32::YELLOW, reason);
        }
        if let Some(reason) = v["build_prerequisite"].as_str() {
            ui.label(reason);
        }
        egui::CollapsingHeader::new("Inspection generations")
            .id_salt((v["selection"].as_str(), "inspection-history"))
            .show(ui, |ui| Self::value(ui, &v["inspections"]));
        ui.label(format!(
            "Inspection: {} · Preparation: {} · Publication: {}",
            v["inspection"].as_str().unwrap_or("unknown"),
            v["preparation"].as_str().unwrap_or("unknown"),
            v["publication"].as_str().unwrap_or("unknown")
        ));
        ui.small("Preliminary inspection is not a guarantee of DAW audio, editor, automation, or state recall.");
        if v["origin"] == "retained_sv1" {
            ui.small("Adopted exact SV1 work; originally prepared through the engineering CLI.");
        }
        if let Some(reason) = v["controller"]["reason"].as_str() {
            ui.small(reason);
        }
        if !v["preliminary"].is_null() {
            egui::CollapsingHeader::new("Preliminary compatibility details")
                .id_salt(("inspection", v["selection"].as_str()))
                .show(ui, |ui| {
                    let f = &v["preliminary"];
                    ui.label(format!(
                        "Parameters: {} · Positive inspection cleanup: {}",
                        f["parameter_count"], f["cleanup_confirmed"]
                    ));
                    if f["editor_interface"].is_null() {
                        ui.label("Editor interface: no assessment recorded");
                    } else {
                        ui.label(format!(
                            "Editor created: {} · HWND interface result: {} · Not attached",
                            f["editor_interface"]["created"], f["editor_interface"]["hwnd_result"]
                        ));
                    }
                    ui.monospace(serde_json::to_string_pretty(f).unwrap_or_default());
                });
        }
        if let Some(op) = v.get("operation").filter(|o| !o.is_null()) {
            ui.label(format!(
                "This product's operation: {}",
                op["state"].as_str().unwrap_or("unknown")
            ));
            if let Some(stage) = op["preparation_failure"]["attempted_stage"].as_str() {
                ui.label(format!("Stopped during {}", stage.replace('_', " ")));
            }
            if let Some(reason) = op["reason"].as_str() {
                ui.colored_label(egui::Color32::YELLOW, reason);
            }
            if let Some(recovery) = op["preparation_failure"]["recovery"].as_str() {
                ui.small(recovery);
            }
        }
        egui::CollapsingHeader::new("Recorded tests and remaining qualification")
            .id_salt(v["selection"].as_str())
            .default_open(false)
            .show(ui, |ui| {
                for area in [
                    "daw_load",
                    "midi",
                    "audio",
                    "editor",
                    "parameters",
                    "automation",
                    "state_recall",
                    "processing_restart",
                    "retirement",
                ] {
                    let latest = v["evidence"]
                        .as_array()
                        .and_then(|rows| rows.iter().rev().find(|r| r["area"] == area));
                    ui.label(format!(
                        "{}: {}",
                        area.replace('_', " "),
                        latest
                            .and_then(|r| r["status"].as_str())
                            .unwrap_or("not tested")
                    ));
                    if let Some(row) = latest {
                        ui.small(format!(
                            "{} · {}",
                            row["witness"].as_str().unwrap_or("unavailable"),
                            row["detail"].as_str().unwrap_or("")
                        ));
                    }
                }
                if let Some(rows) = v["unmet_requirements"].as_array() {
                    for row in rows {
                        if let Some(text) = row.as_str() {
                            ui.small(text);
                        }
                    }
                }
            });
    }
    fn request(&mut self, q: Query, origin: RequestOrigin, ctx: &egui::Context) {
        if self.frontend_superseded { return; }
        self.pending = true;
        self.origin = Some(origin);
        self.background_poll = origin == RequestOrigin::BackgroundActivity;
        self.action_inflight = origin == RequestOrigin::UserAction;
        if self.action_inflight {
            if let Some(f) = &mut self.feedback {
                f.text = "Submitting this request to the manager…".into();
            }
        }
        self.last_poll = Instant::now();
        client::send(q, self.sender.clone(), ctx.clone(), self.window_wake.clone());
    }
    fn controls_pending(&self) -> bool {
        self.frontend_superseded || self.queued_action.is_some()
            || self.queued_import
            || self.action_inflight
            || self.feedback.as_ref().is_some_and(|f| f.blocking)
            || (self.pending && self.origin.is_some_and(RequestOrigin::foreground))
    }
    fn capture_action(&mut self, request: Request) -> bool {
        if self.controls_pending() {
            return false;
        }
        for form in [&mut self.product_form, &mut self.workspace_select_form,
            &mut self.installer_rename_form, &mut self.software_install_form,
            &mut self.removal_form] {
            if form.as_ref() == Some(&request.action) {
                *form = None;
            }
        }
        // The confirmed request differs from its offer only by the musician's answers.
        if self.removal_form.as_ref().is_some_and(|form| offer_identity(form) == offer_identity(&request.action)) {
            self.removal_form = None;
        }
        self.feedback = Some(RequestFeedback::captured(request.action.clone()));
        self.queued_action = Some(request);
        true
    }
    fn capture_chosen_action(&mut self, action: Action) -> bool {
        let Some(snapshot) = self.current_action_snapshot() else {
            return false;
        };
        self.capture_action(Request {
            schema: crate::model::OPERATOR_SCHEMA,
            state_token: snapshot.state_token.clone(),
            action,
        })
    }
    fn current_action_snapshot(&self) -> Option<&Snapshot> {
        // Age alone cannot change the displayed target or consequences. Capture
        // its original token; current backend admission remains authoritative.
        self.overview.as_ref().filter(|_| self.overview_fresh || self.overview_renewing)
            .map(|o| &o.current)
    }
    fn current_readiness(&self) -> Option<&ReadinessAssessment> {
        if !self.overview_fresh || self.readiness.as_ref()?.1.elapsed() >= Duration::from_secs(60) {
            return None;
        }
        self.last_readiness()
    }
    fn last_readiness(&self) -> Option<&ReadinessAssessment> {
        let overview = self.overview.as_ref()?;
        let (assessment, _, generation) = self.readiness.as_ref()?;
        (*generation == overview.current_generation
            && assessment.state_token == overview.current.state_token
            && assessment.system == overview.current.system).then_some(assessment)
    }
    fn overview_query(&self) -> Query {
        // Only a requested check, first readback, or an owned user workflow
        // collects platform facts. Time expiry and passive change signals
        // renew the current record projection without capability helpers.
        if self.explicit_refresh_queued || self.overview.is_none()
            || self.pending_import_route.is_some()
            || self.feedback.as_ref().is_some_and(|feedback| feedback.release_after_snapshot) {
            Query::Overview
        } else { Query::CurrentOverview }
    }
    fn apply_current_overview(&mut self, mut bundle: CurrentOverview) {
        if bundle.current.system.capacity_available() {
            self.cleanup_failure_observed = bundle.current.system.cleanup_unconfirmed;
        }
        veto_owned_pause(&mut bundle.current.operation, self.cleanup_failure_observed);
        if self.software_install_daw.is_none() {
            let mut choices = bundle.current.installer_setups.iter()
                .filter_map(|setup| setup.journey.as_ref().map(|journey| journey.daw));
            if let Some(first) = choices.next() {
                if choices.all(|choice| choice == first) { self.software_install_daw = Some(first); }
            }
        }
        self.readback_failures = 0;
        self.retry_after = None;
        if let Some(f) = &mut self.feedback { f.reconcile_snapshot(&bundle.current); }
        self.resolve_import_route(&bundle.current);
        if self.snapshot.as_ref().is_some_and(|old| old.state_token != bundle.current.state_token) {
            self.snapshot = None;
            self.details_attempted = false;
        }
        self.overview = Some(bundle);
        if self.selected_product_snapshot().is_none()
            || self.product_detail.as_ref().is_some_and(|detail|
                self.overview.as_ref().is_some_and(|overview|
                    detail.system != overview.current.system)) {
            self.product_detail = None;
            self.product_attempted = None;
        }
        self.overview_fresh = true;
        self.overview_renewing = false;
        self.last_overview = Instant::now();
    }
    fn selected_product_snapshot(&self) -> Option<Snapshot> {
        // Retain the exact card during status refresh so navigation does not
        // collapse. Freshness still controls action admission separately.
        let overview = self.overview.as_ref()?;
        let detail = self.product_detail.as_ref().filter(|detail|
            detail.state_token == overview.current.state_token
                && detail.current_generation == overview.current_generation)?;
        let focus = self.library.focused_product()?;
        if ProductKey::from(&detail.product) != *focus { return None; }
        let mut current = overview.current.clone();
        let mut matches = current.products.iter_mut().filter(|product|
            ProductKey::from(&**product) == *focus);
        let product = matches.next()?;
        *product = detail.product.clone();
        if matches.next().is_some() { return None; }
        current.environments = detail.environments.clone();
        current.vendor_applications = detail.vendor_applications.clone();
        current.system = detail.system.clone();
        Some(current)
    }
    fn current_health_system(&self) -> Option<&System> {
        if !self.overview_fresh && !self.overview_renewing { return None; }
        self.retained_health_system()
    }
    /// Whether a DAW is using bridged plug-ins now; None while unobserved.
    fn plugins_in_use(&self) -> Option<bool> {
        if let Some(dsp) = self.last_pulse.as_ref()
            .filter(|_| self.last_poll.elapsed() < Duration::from_secs(15))
            .and_then(|pulse| pulse.dsp) {
            return Some(dsp > 0);
        }
        self.current_health_system().map(|system| system.dsp > 0)
    }
    fn current_owned_pause(&self) -> bool {
        if self.cleanup_failure_observed { return false; }
        let operation = if let Some(pulse) = self.last_pulse.as_ref()
            .filter(|_| self.last_poll.elapsed() < Duration::from_secs(15)) {
            if pulse.pending_transactions > 0 || pulse.cleanup_unconfirmed == Some(true) {
                return false;
            }
            pulse.operation.as_ref()
        } else if self.overview_fresh || self.overview_renewing {
            self.overview.as_ref().and_then(|overview| overview.current.operation.as_ref())
        } else { None };
        presentation::owned_service_pause(operation)
    }
    fn retained_health_system(&self) -> Option<&System> {
        let overview = self.overview.as_ref()?;
        if self.page == Page::Plugins {
            if let Some(detail) = self.product_detail.as_ref().filter(|detail|
                detail.state_token == overview.current.state_token
                    && detail.current_generation == overview.current_generation
                    && self.library.focused_product().is_some_and(|focus|
                        ProductKey::from(&detail.product) == *focus)) {
                return Some(&detail.system);
            }
        }
        Some(&overview.current.system)
    }
    fn next_action(&mut self) -> Option<Request> {
        if self.pending || self.frontend_superseded {
            return None;
        }
        let request = self.queued_action.take()?;
        let current = self.current_action_snapshot();
        if current.is_some_and(|snapshot| snapshot.state_token == request.state_token) {
            return Some(request);
        }
        // A readback cannot upgrade a click into a different action, target or
        // consequence. Nothing was dispatched, so refuse locally without replay.
        let changed = current.is_some();
        if let Some(feedback) = &mut self.feedback {
            feedback.transition("not_submitted");
            feedback.terminal = true;
            feedback.blocking = false;
            feedback.release_after_snapshot = false;
            feedback.text = if changed {
                "The setup changed during status refresh. Review its current actions and try again; this request was not submitted."
            } else {
                "Current status is unavailable. Refresh and try again; this request was not submitted."
            }.into();
        }
        self.refresh_after |= !changed;
        None
    }
    fn buttons(
        ui: &mut egui::Ui,
        actions: &[AvailableAction],
        busy: Option<&str>,
        pending: bool,
        chosen: &mut Option<Action>,
    ) {
        crate::library::action_buttons(ui, actions, busy, pending, chosen);
    }
    fn resolve_import_route(&mut self, snapshot: &Snapshot) {
        let Some((installer, newly_imported, label)) = self.pending_import_route.take() else { return };
        match imported_destination(snapshot, &installer) {
            Some(Page::Setup) => {
                self.page = Page::Setup;
                self.focus.setup = None;
                self.focus.setup_installer = Some(installer);
                self.focus.setup_scroll = true;
                self.message = if newly_imported {
                    format!("Added {label}. No native plug-in setup has started.")
                } else {
                    "This installer was already added. Showing its existing plug-in setup.".into()
                };
            }
            Some(Page::Workspaces) => {
                self.page = Page::Workspaces;
                self.focus.setup = None;
                self.focus.setup_installer = None;
                self.focus.setup_scroll = false;
                self.message = "This installer belongs to the managed FL Studio application. Showing Workspaces.".into();
            }
            _ => {
                self.pending_import_route = Some((installer, newly_imported, label));
                self.message = "The installer was added, but its setup could not be loaded. Refresh before continuing.".into();
            }
        }
    }
    fn handle_reply(&mut self, reply: Reply) {
        let origin = self.origin.take().unwrap_or(if self.action_inflight {
            RequestOrigin::UserAction
        } else if self.background_poll {
            RequestOrigin::BackgroundActivity
        } else { RequestOrigin::InitialSnapshot });
        let was_action = origin == RequestOrigin::UserAction;
        self.pending = false;
        self.background_poll = false;
        self.action_inflight = false;
        match reply {
            Reply::FrontendSuperseded => {
                // Keep drafts, queued actions and operation feedback intact.
                // Nothing is resubmitted or cancelled during the handoff.
                self.frontend_superseded = true;
                self.overview_fresh = false;
                self.overview_renewing = false;
                self.readiness = None;
            }
            Reply::Overview(bundle) => {
                self.readiness = Some((bundle.readiness.clone(), Instant::now(),
                    bundle.current_generation.clone()));
                self.apply_current_overview((*bundle).into());
            }
            Reply::CurrentOverview(bundle) => self.apply_current_overview(*bundle),
            Reply::Product(detail) => {
                if self.overview.as_ref().is_some_and(|overview|
                    self.overview_fresh
                        && detail.state_token == overview.current.state_token
                        && detail.current_generation == overview.current_generation
                        && overview.current.products.iter().filter(|product|
                            ProductKey::from(*product) == ProductKey::from(&detail.product))
                            .count() == 1) {
                    self.product_detail = Some(*detail);
                } else {
                    self.product_detail = None;
                    self.product_attempted = None;
                    self.overview_fresh = false;
                    self.overview_renewing = false;
                    self.refresh_after = true;
                }
            }
            Reply::Snapshot(mut s) => {
                if s.system.capacity_available() {self.cleanup_failure_observed = s.system.cleanup_unconfirmed;}
                veto_owned_pause(&mut s.operation, self.cleanup_failure_observed);
                if s.schema != crate::model::OPERATOR_SCHEMA {
                    self.message = format!("Update the frontend and manager together: operator model {} required",
                        crate::model::OPERATOR_SCHEMA);
                } else {
                    if let Some(f) = &mut self.feedback {
                        f.reconcile_snapshot(&s);
                    }
                    self.resolve_import_route(&s);
                    if self.overview.as_ref().is_some_and(|old|
                        old.current.state_token != s.state_token) {
                        self.overview_fresh = false;
                        self.overview_renewing = false;
                        self.refresh_after = true;
                    }
                    self.snapshot = Some(*s);
                    self.details_attempted = true;
                    // Initial and post-mutation readback are quiet; a durable
                    // import/action notice is never replaced by polling.
                }
            }
            Reply::Receipt(r) => {
                if r.accepted {
                    self.operation_live = true;
                    self.prompt_pulse = true;
                }
                if let Some(f) = &mut self.feedback {
                    f.receipt(&r);
                }
                self.message = if r.accepted {
                    format!("Operation accepted: {}", r.operation.unwrap_or_default())
                } else {
                    r.refusal.unwrap_or_else(|| "Operation refused".into())
                };
                // Do not race a newly dispatched mutation for registry.lock.
                // The lightweight receipt poll selects the completed readback.
                self.refresh_after = !r.accepted;
            }
            Reply::Activity(mut a) => {
                retain_projected_handoff(self.overview.as_ref().and_then(|overview|
                    overview.current.operation.as_ref()), &mut a.operation);
                if a.system.capacity_available() {self.cleanup_failure_observed = a.system.cleanup_unconfirmed;}
                veto_owned_pause(&mut a.operation, self.cleanup_failure_observed);
                if let Some(op) = &a.operation {
                    if let Some(f) = &mut self.feedback {
                        f.observe(op);
                    }
                }
                if let Some(s) = &mut self.snapshot {
                    if s.system.capacity_available() != a.system.capacity_available()
                        || s.system.dsp != a.system.dsp
                        || s.system.maintenance != a.system.maintenance
                        || s.system.cleanup_unconfirmed != a.system.cleanup_unconfirmed
                        || refresh_for_receipt(&s.operation, &a.operation)
                        || refresh_for_workspace_product(s, a.workspace_product_install_ready)
                    {
                        self.refresh_after = true;
                    }
                    s.system = a.system.clone();
                    s.capture = a.capture.clone();
                    s.operation = a.operation.clone();
                }
                if let Some(overview) = &mut self.overview {
                    if overview.current.system != a.system
                        || refresh_for_receipt(&overview.current.operation, &a.operation) {
                        self.overview_fresh = false;
                        self.overview_renewing = false;
                        self.refresh_after = true;
                    }
                    overview.current.system = a.system;
                    overview.current.capture = a.capture;
                    overview.current.operation = a.operation;
                }
            }
            Reply::Pulse(mut p) => {
                retain_projected_handoff(self.overview.as_ref().and_then(|overview|
                    overview.current.operation.as_ref()), &mut p.operation);
                if let Some(failed) = p.cleanup_unconfirmed {self.cleanup_failure_observed = failed;}
                veto_owned_pause(&mut p.operation, self.cleanup_failure_observed);
                self.prompt_pulse = false;
                self.operation_live = p.operation_live;
                if let Some(op) = &p.operation {
                    if let Some(feedback) = &mut self.feedback { feedback.observe(op); }
                }
                let changed = self.last_pulse.as_ref().map(|previous|
                    pulse_posture_changed(previous, &p));
                if let Some(overview) = &mut self.overview {
                    if overview.current_generation != p.current_generation
                        || changed.unwrap_or_else(|| pulse_system_changed(&p, &overview.current.system))
                        || refresh_for_receipt(&overview.current.operation, &p.operation) {
                        self.overview_fresh = false;
                        self.overview_renewing = false;
                        self.refresh_after = true;
                    }
                    overview.current.operation = p.operation.clone();
                }
                if let Some(snapshot) = &mut self.snapshot {
                    if changed.unwrap_or_else(|| pulse_system_changed(&p, &snapshot.system))
                        || refresh_for_receipt(&snapshot.operation, &p.operation) {
                        self.refresh_after = true;
                    }
                    snapshot.operation = p.operation.clone();
                }
                self.last_pulse = Some(p);
                if self.overview.is_none() || !self.overview_fresh {
                    self.refresh_after = true;
                }
            }
            Reply::Imported(imported) => {
                if let Some(draft) = &mut self.focus.replacement {
                    draft.selected = Some(imported.installer_sha256);
                    self.message = format!("Selected {}. Review it in this setup before running it.", imported.display_label);
                } else {
                    self.pending_import_route = Some((imported.installer_sha256,
                        imported.newly_imported, imported.display_label.clone()));
                    self.message = format!("{} was added. Opening its setup…", imported.display_label);
                }
                self.feedback = None;
                self.refresh_after = true;
            }
            Reply::Cancelled => {
                if origin == RequestOrigin::InstallerImport {
                    self.message = "No installer selected.".into();
                    self.focus.replacement = None;
                }
            }
            Reply::Error(e) => {
                if origin == RequestOrigin::ProductDetail {
                    self.product_detail = None;
                    return;
                }
                if was_action {
                    self.prompt_pulse = true;
                } else {
                    self.readback_failures = self.readback_failures.saturating_add(1);
                    let delay = 1_u64 << self.readback_failures.min(5);
                    self.retry_after = Some(Instant::now() + Duration::from_secs(delay));
                    self.prompt_pulse = false;
                }
                if was_action {
                    if let Some(f) = &mut self.feedback {
                        f.acknowledgment_uncertain = true;
                        f.transition("acknowledgment_unconfirmed");
                        f.text=format!("Request acknowledgment is unavailable: {e}. Reconciling manager ownership; no automatic resubmission.");
                        self.refresh_after = true;
                    }
                }
                if origin.foreground() || matches!(origin, RequestOrigin::InitialSnapshot | RequestOrigin::ExplicitRefresh) {
                    self.message = e;
                }
                // Background readback errors stay quiet as requests, but the
                // last healthy service state cannot remain current.
                if let Some(s) = &mut self.snapshot {
                    s.system.service = "capacity unavailable".into();
                }
                if let Some(overview) = &mut self.overview {
                    overview.current.system.service = "capacity unavailable".into();
                }
                self.overview_fresh = false;
                self.overview_renewing = false;
            }
        }
    }
    fn value(ui: &mut egui::Ui, v: &serde_json::Value) {
        ui.add(
            egui::Label::new(
                egui::RichText::new(serde_json::to_string_pretty(v).unwrap_or_default())
                    .monospace(),
            )
            .wrap(),
        );
    }
}

fn warning_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(245, 190, 100)
    } else {
        egui::Color32::from_rgb(140, 77, 0)
    }
}

fn navigation_button(
    ui: &mut egui::Ui,
    page: &mut Page,
    item: Page,
    width: f32,
) -> (Page, bool, egui::Rect) {
    let selected = *page == item;
    let button = egui::Button::new(item.label())
        .min_size(egui::vec2(width, 44.0))
        .selected(selected);
    let response = ui.add(button);
    if response.clicked() {
        *page = item;
    }
    (item, selected, response.rect)
}

fn navigation(ui: &mut egui::Ui, page: &mut Page) -> Vec<(Page, bool, egui::Rect)> {
    let items = [
        Page::Home,
        Page::Plugins,
        Page::Workspaces,
        Page::Activity,
        Page::Setup,
        Page::Diagnostics,
    ];
    let mut bounds = Vec::new();
    if ui.available_width() < 760.0 {
        let width = ((ui.available_width() - 24.0) / 3.0).max(94.0);
        egui::Grid::new("navigation-narrow")
            .num_columns(3)
            .spacing(egui::vec2(12.0, 12.0))
            .show(ui, |ui| {
                for (index, item) in items.into_iter().enumerate() {
                    bounds.push(navigation_button(ui, page, item, width));
                    if index % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
    } else {
        ui.horizontal_wrapped(|ui| {
            for item in items {
                bounds.push(navigation_button(ui, page, item, 94.0));
            }
        });
    }
    bounds
}

fn navigate(
    destination: &Destination,
    page: &mut Page,
    library: &mut crate::library::Library,
    focus: &mut RouteFocus,
) {
    match destination {
        Destination::Product(key) => {
            library.focus_product(key.clone());
            *page = Page::Plugins;
        }
        Destination::Attempt(key) => {
            focus.setup = Some(key.clone());
            focus.setup_installer = Some(key.installer.clone());
            focus.setup_scroll = true;
            *page = Page::Setup;
        }
        Destination::Session(session) => {
            focus.activity = Some(session.clone());
            *page = Page::Activity;
        }
        Destination::Activity => *page = Page::Activity,
        Destination::Setup => *page = Page::Setup,
        Destination::Diagnostics => *page = Page::Diagnostics,
    }
}


fn existing_setup_installer(ui: &mut egui::Ui, snapshot: &Snapshot,
    fresh: bool, pending: bool, chosen: &mut Option<Action>) {
    if snapshot.environment_installers.is_empty() {return;}
    ui.separator();
    ui.strong("Manage an existing setup");
    let key = egui::Id::new("companion_environment");
    let mut selected = ui.data_mut(|data|data.get_temp::<String>(key)).unwrap_or_default();
    egui::ComboBox::from_id_salt("companion_environment_choice")
        .selected_text(snapshot.environment_installers.iter().find(|target|target.environment == selected)
            .map(|target|target.name.as_str()).unwrap_or("Choose the existing setup or plug-in"))
        .show_ui(ui,|ui| {
            for target in &snapshot.environment_installers {
                ui.selectable_value(&mut selected,target.environment.clone(),&target.name);
            }
        });
    ui.data_mut(|data|data.insert_temp(key,selected.clone()));
    if let Some(target) = snapshot.environment_installers.iter().find(|target|target.environment == selected) {
        ui.push_id(&target.environment,|ui|ui.add_enabled_ui(fresh, |ui|
            crate::library::environment_installer_controls(ui,target,pending,chosen)));
        for app in snapshot.vendor_applications.iter().filter(|app|
            app.details["user_selected"] == true && app.details["environment"] == target.environment) {
            ui.strong(&app.name);
            ui.label(&app.version);
            if let Some(path) = app.details["executable"]["path"].as_str() {ui.small(path);}
            ui.add_enabled_ui(fresh,|ui|
                crate::library::action_buttons(ui,&app.actions,snapshot.system.inactive_reason(),pending,chosen));
        }
    }
    // An exact running owner's Stop remains visible even if no setup is selected.
    for target in snapshot.environment_installers.iter().filter(|target|target.environment != selected) {
        if target.actions.iter().any(|a|matches!(a.action,Action::EnvironmentInstallerStop { .. })) {
            ui.strong(&target.name);
            ui.add_enabled_ui(fresh, |ui|
                crate::library::action_buttons(ui,&target.actions,None,pending,chosen));
        }
    }
}

fn show_discovered_products(setup: &InstallerSetup,
    library: &mut crate::library::Library, page: &mut Page) {
    library.focus_products(setup.discovered.iter().map(|product| ProductKey {
        environment: product.environment.clone(),
        module_sha256: product.module_sha256.clone(),
        class_id: product.class_id.clone(),
    }).collect());
    *page = Page::Plugins;
}

fn setup_phase_label(phase: &SetupPhase) -> &'static str {
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

fn setup_priority(phase: &SetupPhase) -> u8 {
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

fn journey_phase_label(journey: &InstallationJourney) -> &'static str {
    if journey.phase == InstallationPhase::Available && journey.plugins.is_empty() {
        return "Application session complete";
    }
    match journey.phase {
        InstallationPhase::PreparingEnvironment => "Getting ready to install",
        InstallationPhase::VendorRunning => "Complete the vendor's requests",
        InstallationPhase::Discovering => "Finding installed plug-ins",
        InstallationPhase::PreparingPlugins => "Connecting plug-ins to your DAW",
        InstallationPhase::AwaitingApplication => "Continue in the installed app",
        InstallationPhase::Available => "DAW connection complete",
        InstallationPhase::NeedsAttention => "Installation or DAW connection needs attention",
    }
}

fn veto_owned_pause(operation: &mut Option<serde_json::Value>, cleanup_failed: bool) {
    if cleanup_failed {
        if let Some(operation) = operation.as_mut().and_then(serde_json::Value::as_object_mut) {
            operation.remove("owned_service_pause");
        }
    }
}

fn setup_attention_priority(setup: &InstallerSetup) -> u8 {
    setup.journey.as_ref().map(|journey| match journey.phase {
        InstallationPhase::VendorRunning => 0,
        InstallationPhase::NeedsAttention => 2,
        InstallationPhase::AwaitingApplication => 2,
        InstallationPhase::PreparingEnvironment | InstallationPhase::Discovering
            | InstallationPhase::PreparingPlugins => 3,
        InstallationPhase::Available => 6,
    }).unwrap_or_else(|| setup_priority(&setup.phase))
}

/// A finished journey is installation history. What is in the DAW now comes
/// from each plug-in's current state, so a later removal reads as removed.
fn current_plugin_state<'a>(journey: &InstallationJourney, plugin: &InstallationPlugin,
    current: Option<(&'a Snapshot, &str)>) -> Option<&'a str> {
    if journey.phase != InstallationPhase::Available || plugin.stage != InstallationPluginStage::Available {
        return None;
    }
    let (snapshot, environment) = current?;
    snapshot.products.iter().find(|product| product.environment == environment
        && product.class_id.eq_ignore_ascii_case(&plugin.class_id))
        .map(|product| crate::library::status(product).0)
}
fn current_journey_label(journey: &InstallationJourney, current: Option<(&Snapshot, &str)>) -> &'static str {
    if journey.phase == InstallationPhase::Available && !journey.plugins.is_empty() {
        let states: Vec<_> = journey.plugins.iter()
            .filter_map(|plugin| current_plugin_state(journey, plugin, current)).collect();
        let available = states.iter().filter(|state| **state == "Available in your DAW").count();
        if !states.is_empty() && available == 0 { return "Not in your DAW"; }
        if available < states.len() { return "Partly in your DAW"; }
    }
    journey_phase_label(journey)
}
fn journey_results(ui: &mut egui::Ui, journey: &InstallationJourney, current: Option<(&Snapshot, &str)>) {
    ui.small(format!("Your DAW: {}", presentation::daw_name(journey.daw)));
    if journey.phase == InstallationPhase::Available && journey.plugins.is_empty() {
        ui.label("No plug-ins were added during this visit.");
    }
    if let Some(failure) = &journey.failure {
        ui.colored_label(warning_color(ui), installation_failure_text(failure));
    }
    for plugin in &journey.plugins {
        ui.horizontal_wrapped(|ui| {
            ui.strong(&plugin.name);
            if let Some(state) = current_plugin_state(journey, plugin, current) { ui.label(state); return; }
            ui.label(match plugin.stage {
                InstallationPluginStage::Pending => "Waiting to connect",
                InstallationPluginStage::Inspecting => "Checking plug-in interfaces",
                InstallationPluginStage::Preparing => "Preparing the native connection",
                InstallationPluginStage::Publishing => "Making the connection available",
                InstallationPluginStage::Available => "Available in your DAW",
                InstallationPluginStage::NeedsAttention => "Connection needs attention",
                InstallationPluginStage::ReplacementRequired => "A different setup is already available",
            });
        });
        if let Some(failure) = &plugin.failure {
            ui.colored_label(warning_color(ui), installation_failure_text(failure));
        }
        if plugin.stage == InstallationPluginStage::ReplacementRequired {
            ui.small("Open this plug-in to review changing its existing DAW setup.");
        }
    }
}

fn installation_failure_text(failure: &InstallationFailure) -> String {
    let phase = match failure.stage {
        InstallationFailureStage::EnvironmentPreparation => "Installation setup",
        InstallationFailureStage::VendorInstallation => "Vendor installation",
        InstallationFailureStage::Discovery => "Plug-in discovery",
        InstallationFailureStage::Inspection => "Plug-in inspection",
        InstallationFailureStage::Preparation => "Native connection preparation",
        InstallationFailureStage::Publication => "DAW connection",
    };
    format!("{phase}: {}", failure.reason)
}

/// Removal choices sit on every settled card; they are not pending setup work.
fn no_setup_work(setup: &InstallerSetup) -> bool {
    setup.primary.is_none() && setup.secondary.iter().all(|offer| removal_offer(&offer.action))
}
fn retained_setup_records_need_refresh(setup: &InstallerSetup, snapshot: &Snapshot) -> bool {
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

fn setup_display_name(setup: &InstallerSetup, snapshot: &Snapshot) -> String {
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
        [first, rest @ ..] => format!("{first} and {} other plug-ins",rest.len()),
    }
}

fn partial_installation_needs_review(setup: &InstallerSetup, snapshot: &Snapshot) -> bool {
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

fn setup_history_settled_by_current_products(setup: &InstallerSetup, snapshot: &Snapshot) -> bool {
    setup.journey.is_none() && setup.phase == SetupPhase::SetupNeedsAttention
        && no_setup_work(setup)
        && setup.environment.is_some() && !setup.discovered.is_empty()
        && (retained_setup_records_need_refresh(setup, snapshot)
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

struct ApplicationGroup<'a> {
    environment: &'a str,
    setup: Option<&'a InstallerSetup>,
    applications: Vec<&'a VendorApplication>,
}

fn application_groups<'a>(snapshot: &'a Snapshot, focus: &RouteFocus) -> Vec<ApplicationGroup<'a>> {
    let mut groups: Vec<ApplicationGroup<'a>> = Vec::new();
    for app in &snapshot.vendor_applications {
        let Some(environment) = app.details["environment"].as_str() else { continue; };
        let setup = snapshot.installer_setups.iter().find(|setup|
            setup.environment.as_deref() == Some(environment));
        if focus.setup.as_ref().and_then(|key| key.environment.as_deref())
            .is_some_and(|selected| selected != environment)
            || focus.setup_installer.as_ref().is_some_and(|installer|
                setup.is_none_or(|setup| &setup.installer != installer)) { continue; }
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

fn setup_offer_uses_program_choice(snapshot: &Snapshot, setup: &InstallerSetup,
    offer: &AvailableAction) -> bool {
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

fn obsolete_initial_installer_offer(offer: &AvailableAction) -> bool {
    offer.action.software_install().is_some() || matches!(offer.action, Action::InstallerNewAttempt {..}
        | Action::InstallerStart {..} | Action::InstallerStartWithPolicy {..}
        | Action::InstallerFocus {..} | Action::InstallerStop {..} | Action::InstallerScan {..})
}

impl Operator {
    fn readiness_details(ui: &mut egui::Ui, assessment: Option<&ReadinessAssessment>,
        fresh: bool, pending: bool, refresh: &mut bool, chosen: &mut Option<Action>) {
        egui::CollapsingHeader::new("Computer setup and support").show(ui, |ui| {
            ui.label(if !fresh {
                "Refreshing current status. Actions will return when it finishes."
            } else if assessment.is_none() {
                "Computer setup has not been checked in this window. This does not change which plug-ins are available in your DAW."
            } else {
                "This check describes the current computer setup. It does not test a plug-in's sound or saved-project recall."
            });
            if ui.add_enabled(!pending, egui::Button::new("Check computer setup")
                .min_size(egui::vec2(180.0, 44.0))).clicked() { *refresh = true; }
            if let Some(assessment) = assessment {
                for blocker in &assessment.blockers {
                    ui.colored_label(warning_color(ui), &blocker.explanation);
                }
                Self::buttons(ui, std::slice::from_ref(&assessment.support_export_action),
                    None, pending || !fresh, chosen);
                ui.small("The support report stays on this computer. Nothing is uploaded.");
                egui::CollapsingHeader::new("Technical observations").show(ui, |ui| {
                    for facts in [&assessment.platform, &assessment.daw, &assessment.audio,
                        &assessment.graphics, &assessment.runtime] {
                        for fact in facts {
                            ui.small(format!("{}: {} · {} · {:?}", fact.name,
                                fact.value.as_deref().unwrap_or("unknown"), fact.source,
                                fact.certainty));
                        }
                    }
                });
            }
        });
    }
    fn fast_home(ui: &mut egui::Ui, overview: &CurrentOverview,
        assessment: Option<&ReadinessAssessment>, fresh: bool,
        pending: bool, controls: (&mut Page, &mut crate::library::Library, &mut RouteFocus,
            &mut bool, &mut Option<Action>)) {
        let (page, library, focus, refresh, chosen) = controls;
        ui.horizontal_wrapped(|ui| {
            ui.heading("Your plug-ins");
            if ui.add(egui::Button::new("Add a plug-in")
                .min_size(egui::vec2(150.0, 44.0))).clicked() { *page = Page::Setup; }
        });
        if !fresh {
            ui.colored_label(warning_color(ui),
                "Current status is unavailable. These are the last listed plug-ins; setup actions wait for current readback.");
        }
        let unfinished = overview.current.installer_setups.iter().filter(|setup|
            setup.journey.as_ref().map(|journey| journey.phase != InstallationPhase::Available)
                .unwrap_or(setup.phase != SetupPhase::DiscoveryComplete)
                && !setup_history_settled_by_current_products(setup, &overview.current))
            .min_by_key(|setup| setup_attention_priority(setup));
        // A plug-in the musician removed from the DAW is not unfinished setup.
        let needs_setup = overview.current.products.iter().find(|product|
            crate::library::status(product).1 != "published"
                && !matches!(product.disposition.as_str(), "removed_from_daw" | "uninstalled"));
        let recovery = overview.current.products.iter().find(|product|
            crate::library::recovery_pending(product));
        let owned_pause = presentation::owned_service_pause(overview.current.operation.as_ref())
            && overview.current.system.pending_transactions == 0
            && overview.current.system.stale_transports == 0
            && (!overview.current.system.capacity_available() || !overview.current.system.cleanup_unconfirmed);
        egui::Frame::group(ui.style()).fill(ui.visuals().faint_bg_color)
            .inner_margin(16.0).show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            if (!overview.current.system.capacity_available()
                || overview.current.system.cleanup_unconfirmed
                || overview.current.system.stale_transports > 0
                || overview.current.system.pending_transactions > 0) && !owned_pause {
                ui.strong("Next: resolve the bridge service problem");
                ui.label(if !overview.current.system.capacity_available() {
                    "The bridge service could not report its current status."
                } else if overview.current.system.pending_transactions > 0 {
                    "A previous setup change has not finished."
                } else {"Could not confirm that an earlier plug-in or installer closed."});
                let recoveries: Vec<_> = overview.current.actions.iter().filter(|offer|
                    matches!(offer.action,Action::TransactionReconcile {})).cloned().collect();
                Self::buttons(ui,&recoveries,overview.current.system.inactive_reason(),pending || !fresh,chosen);
                if ui.button("Review recovery details").clicked() {*page = Page::Diagnostics;}
            } else if let Some(setup) = unfinished {
                let partial = partial_installation_needs_review(setup, &overview.current);
                let name = setup_display_name(setup, &overview.current);
                let current = setup.environment.as_deref().map(|environment| (&overview.current, environment));
                ui.strong(if let Some(journey) = &setup.journey {
                    format!("{name} · {}", current_journey_label(journey, current))
                } else if partial {format!("Next: review the installation for {name}")}
                    else {format!("Next: continue {name}")});
                if partial {
                    ui.label("A complete installation was not confirmed. Review this setup before making installation changes. Plug-ins already available in your DAW remain available.");
                } else {ui.label(&setup.status);}
                if let Some(journey) = &setup.journey { journey_results(ui, journey, current); }
                if let Some(action) = &setup.primary {
                    crate::library::primary_action(ui, action,
                        overview.current.system.inactive_reason(), pending || !fresh, chosen);
                }
                let review = if partial && setup.primary.is_none() {
                    ui.add(presentation::primary_button("Review this installation",ui)).clicked()
                } else {ui.button("Open this setup").clicked()};
                if review {
                    navigate(&Destination::Attempt(AttemptKey {installer:setup.installer.clone(),
                        environment:setup.environment.clone()}), page, library, focus);
                }
            } else if let Some(product) = recovery {
                ui.strong(format!("Next: recover {}",product.name));
                if let Some(workflow) = &product.compatibility { ui.label(&workflow.summary); }
                if ui.add(presentation::primary_button("Open plug-in recovery",ui)).clicked() {
                    library.focus_product(ProductKey::from(product));
                    *page=Page::Plugins;
                }
            } else if let Some(assessment) = assessment.filter(|assessment|
                assessment.overall_status != ReadinessOutcome::Ready
                    && (!assessment.ordered_steps.is_empty() || !assessment.blockers.is_empty())) {
                ui.small("Computer setup recommendations reflect the last explicit setup check.");
                if let Some(step) = assessment.ordered_steps.first() {
                    ui.strong(format!("Next: {}", step.title));
                    ui.label(&step.detail);
                    if let Some(action) = &step.action {
                        crate::library::primary_action(ui, action,
                            overview.current.system.inactive_reason(), pending || !fresh, chosen);
                    }
                } else {
                    ui.strong(match assessment.overall_status {
                        ReadinessOutcome::Unsupported => "A required computer capability is unavailable",
                        ReadinessOutcome::Unknown => "A required setup check is incomplete",
                        _ => "Computer setup needs attention",
                    });
                }
                if let Some(blocker) = assessment.blockers.first() {
                    ui.colored_label(warning_color(ui), &blocker.explanation);
                }
                if ui.button("Review setup details").clicked() {*page = Page::Diagnostics;}
            } else if let Some(product) = needs_setup {
                ui.strong(format!("Next: set up {} for your DAW", product.name));
                ui.label(crate::library::status(product).0);
                if ui.add(presentation::primary_button("Continue plug-in setup", ui)).clicked() {
                    library.focus_product(ProductKey::from(product));
                    *page = Page::Plugins;
                }
            } else if overview.current.products.is_empty() {
                ui.strong("Start with a Windows plug-in");
                ui.label("Choose your vendor's Windows installer and the Linux DAW you use. Complete the vendor's requests when they appear; the manager connects the installed plug-ins to your DAW.");
                if ui.add(presentation::primary_button("Add your first plug-in", ui)).clicked() {
                    *page = Page::Setup;
                }
            } else {
                let mut daws = overview.current.products.iter()
                    .map(|product| presentation::product_daw(&overview.current, product));
                let daw = daws.next().flatten()
                    .filter(|first| daws.all(|other| other == Some(*first)));
                ui.strong(match daw {
                    Some(daw @ (DawChoice::Bitwig | DawChoice::Reaper)) =>
                        format!("Plug-ins available in {}", presentation::daw_name(daw)),
                    _ => "Plug-ins available in your DAW".into(),
                });
                egui::CollapsingHeader::new("Using your plug-ins").show(ui, |ui| {
                    ui.label("Available plug-ins appear as native VST3s. Add them to a track in your DAW; if one is missing, rescan VST3 plug-ins.");
                });
            }
        });
        if !overview.current.products.is_empty() {
            ui.add_space(8.0);
            ui.small(format!("{} managed plug-ins · open one to manage its setup, updates and settings",
                overview.current.products.len()));
        }
        for current in &overview.current.products {
            ui.push_id((&current.environment, &current.class_id, &current.module_sha256), |ui| {
                egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
                    ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(&current.name).size(18.0).strong());
                        if ui.add(egui::Button::new("Manage plug-in")
                            .min_size(egui::vec2(145.0, 44.0))).clicked() {
                            library.focus_product(ProductKey::from(current));
                            *page = Page::Plugins;
                        }
                    });
                    let progress = presentation::installation_progress(&overview.current, current);
                    if fresh { ui.label(progress.unwrap_or_else(|| crate::library::status(current).0)); }
                    if let Some(product) = assessment.and_then(|assessment| assessment.products.iter()
                        .find(|product| product.class_id == current.class_id
                            && product.module_sha256 == current.module_sha256)) {
                        if progress.is_none() && product.installation_health == InstallationHealth::ActionRequired {
                            ui.colored_label(warning_color(ui), &product.reason);
                        }
                        egui::CollapsingHeader::new("Setup check details").show(ui, |ui| {
                            ui.label(installation_label(product));
                            ui.label(match product.support_qualification {
                                SupportQualification::Verified => "Support qualification: exact profile verified",
                                SupportQualification::Unsupported => "Support qualification: exact profile withdrawn",
                                SupportQualification::NotYetQualified => "Support qualification: not yet qualified",
                            });
                            ui.label(&product.reason);
                        });
                    }
                });
            });
        }
        ui.add_space(8.0);
        Self::readiness_details(ui, assessment, fresh, pending, refresh, chosen);
    }
    fn fast_setup(ui: &mut egui::Ui, overview: &CurrentOverview, fresh: bool,
        pending: bool, controls: (&mut bool, &mut Option<Action>, &mut bool,
            &mut Page, &mut crate::library::Library, &mut RouteFocus, &mut Option<DawChoice>)) {
        let (refresh, chosen, pick, page, library, focus, daw) = controls;
        // Exact-operation offers still control a running installer without DSP capacity.
        // Disable stale actions without adding waiting rows to the retained layout.
        // The DAW and program choices are local drafts, not manager requests.
        let _ = refresh;
        ui.heading("Setup");
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(fresh && !pending, presentation::primary_button("Choose Windows installer", ui)).clicked() {
                focus.replacement = None;
                *pick = true;
            }
            ui.label("Add a plug-in");
            daw_choice_control(ui, daw);
        });
        ui.label(egui::RichText::new(if fresh {
            "Choose software and your DAW · Complete vendor requests · Make music"
        } else { "Refreshing setup · actions wait for current status" }).small()
            .color(if fresh { ui.visuals().weak_text_color() } else { warning_color(ui) }));
        existing_setup_installer(ui,&overview.current,fresh,pending,chosen);
        if focus.setup_installer.is_some() {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Showing the selected installer");
                if ui.add_sized([180.0, 44.0], egui::Button::new("Show all setup")).clicked() {
                    focus.setup_installer = None;
                    focus.setup = None;
                    focus.setup_scroll = false;
                }
            });
        }
        let mut shown = 0;
        let mut setups: Vec<_> = overview.current.installer_setups.iter().filter(|setup|
            focus.setup_installer.as_ref().is_none_or(|id| id == &setup.installer)).collect();
        setups.sort_by_key(|setup| setup_attention_priority(setup));
        for setup in setups {
            shown += 1;
            if focus.setup_scroll && focus.setup_installer.as_ref() == Some(&setup.installer) {
                ui.scroll_to_cursor(Some(egui::Align::Center));
                focus.setup_scroll = false;
            }
            ui.push_id(&setup.installer, |ui| {
                egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
                    ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                    ui.label(egui::RichText::new(setup_display_name(setup,&overview.current)).size(18.0).strong());
                    let retained_refresh = retained_setup_records_need_refresh(setup, &overview.current);
                    let partial = partial_installation_needs_review(setup,&overview.current);
                    let settled = setup_history_settled_by_current_products(setup, &overview.current);
                    let current = setup.environment.as_deref().map(|environment| (&overview.current, environment));
                    ui.strong(if let Some(journey) = &setup.journey { current_journey_label(journey, current) }
                        else if settled {"Plug-ins available in your DAW"}
                        else if retained_refresh {"Installation records need attention"}
                        else if partial {"Review earlier installation"}
                        else {setup_phase_label(&setup.phase)});
                    if let Some(journey) = &setup.journey {
                        ui.label(&setup.status);
                        journey_results(ui, journey, current);
                    } else if settled {
                        ui.label(format!("{} plug-ins available in your DAW.",setup.discovered.len()));
                    } else if retained_refresh {
                        ui.label("Installation records need refreshing before making setup changes.");
                    } else if partial {
                        ui.label("A complete installation was not confirmed. Review the installation result before making setup changes. Plug-ins already available in your DAW remain available.");
                    } else if setup.phase == SetupPhase::DiscoveryComplete {
                        ui.label(format!("{} plug-ins found. Open them to see their current DAW connection.",setup.discovered.len()));
                    } else if setup.phase == SetupPhase::Imported && setup.primary.is_none() {
                        ui.colored_label(warning_color(ui), &setup.status);
                    } else { ui.label(&setup.status); }
                    let continuation = overview.current.environment_installers.iter().find(|target|
                        setup.environment.as_ref() == Some(&target.environment) && target.operation.is_some());
                    let reviewing_replacement = focus.replacement.as_ref().is_some_and(|d|d.original == setup.installer);
                    if let Some(primary) = setup.primary.as_ref().filter(|offer| (continuation.is_none()
                        || !obsolete_initial_installer_offer(offer))
                        && !reviewing_replacement && !setup_offer_uses_program_choice(&overview.current,setup,offer)) {
                        ui.add_enabled_ui(fresh, |ui| crate::library::primary_action(ui, primary,
                            overview.current.system.inactive_reason(), pending, chosen));
                    }
                    if !setup.discovered.is_empty()
                        && ui.add_enabled(fresh, egui::Button::new("View discovered plug-ins")
                            .min_size(egui::vec2(240.0, 44.0))).clicked() {
                        show_discovered_products(setup, library, page);
                    }
                    let secondary: Vec<_> = setup.secondary.iter().filter(|offer| !removal_offer(&offer.action)
                        && (continuation.is_none() || !obsolete_initial_installer_offer(offer)) && !reviewing_replacement
                            && offer.action.software_install().is_none_or(|(_,_,policy)|policy.is_none())
                            && !setup_offer_uses_program_choice(&overview.current,setup,offer)).cloned().collect();
                    ui.add_enabled_ui(fresh, |ui| Self::buttons(ui, &secondary,
                        overview.current.system.inactive_reason(), pending, chosen));
                    if let Some(target) = continuation {
                        ui.label(format!("Latest installer in this setup: {}",
                            target.result["state"].as_str().unwrap_or("status unavailable").replace('_', " ")));
                        let actions: Vec<_> = target.actions.iter().filter(|offer| crate::library::installer_retry(offer)
                            || matches!(offer.action,
                            Action::EnvironmentInstallerFocus {..} | Action::EnvironmentInstallerStop {..}
                                | Action::EnvironmentInstallerScan {..}))
                            .cloned().collect();
                        ui.add_enabled_ui(fresh, |ui| Self::buttons(ui, &actions,
                            overview.current.system.inactive_reason(), pending, chosen));
                    }
                    if let Some(context) = replacement_context(&overview.current, setup) {
                        if ui.add_enabled(fresh && !pending,
                            egui::Button::new("Choose another installer").min_size(egui::vec2(240.0,44.0))).clicked() {
                            focus.replacement = Some(context);
                            focus.setup_installer = Some(setup.installer.clone());
                            *pick = true;
                        }
                    } else {
                        ui.small("To choose another installer, finish or stop the current operation and wait for cleanup.");
                    }
                    if let Some(draft) = focus.replacement.as_ref().filter(|d|d.original == setup.installer) {
                        let mut close = false;
                        if let Some(selected) = &draft.selected {
                            ui.separator();
                            let name = overview.current.installer_setups.iter().find(|s| &s.installer == selected)
                                .map(|s|s.name.as_str()).unwrap_or("Selected Windows installer");
                            ui.strong(format!("Selected installer: {name}"));
                            if let Some(target) = overview.current.environment_installers.iter().find(|t|
                                draft.environment.as_ref() == Some(&t.environment)) {
                                ui.label("Continue in this same setup.");
                                ui.label(&target.consequence);
                                if !target.affected.is_empty() {
                                    ui.strong(format!("Affected plug-ins: {}",target.affected.join(", ")));
                                }
                            }
                            match replacement_offer(&overview.current, draft) {
                                Ok(offer) => {
                                    let before = chosen.clone();
                                    ui.add_enabled_ui(fresh, |ui| crate::library::primary_action(ui, &offer,
                                        overview.current.system.inactive_reason(),pending,chosen));
                                    if *chosen != before {
                                        if draft.environment.is_none() {focus.setup_installer = Some(selected.clone());}
                                        close = true;
                                    }
                                }
                                Err(reason) => {ui.colored_label(warning_color(ui),reason);}
                            }
                        }
                        close |= ui.add_enabled(!pending,egui::Button::new("Keep current installer")
                            .min_size(egui::vec2(160.0,44.0))).clicked();
                        if close {focus.replacement = None;}
                    }
                    egui::CollapsingHeader::new("Setup details and options").show(ui, |ui| {
                        ui.small(format!("Recorded installer label: {}",setup.name));
                        if let Some(environment) = &setup.environment {ui.small(format!("Setup environment: {environment}"));}
                        ui.label(&setup.status);
                        if let Some(compatibility) = &setup.compatibility { ui.small(compatibility); }
                        if continuation.is_none() && !reviewing_replacement {
                            let advanced:Vec<_>=setup.primary.iter().chain(&setup.secondary)
                                .filter(|offer|offer.action.software_install().is_some_and(|(_,_,policy)|policy.is_some())).cloned().collect();
                            ui.add_enabled_ui(fresh,|ui|Self::buttons(ui,&advanced,
                                overview.current.system.inactive_reason(),pending,chosen));
                        }
                        for attempt in overview.current.onboarding.iter().filter(|attempt|
                            attempt.installer == setup.installer
                                && attempt.environment == setup.environment) {
                            for line in installer_lines(&attempt.details["installation"]) { ui.label(line); }
                            for line in installer_policy_lines(&attempt.details["installation"]) { ui.small(line); }
                            if let Some(failure) = &attempt.failure {
                                for line in failure_lines(failure) { ui.label(line); }
                            }
                            if continuation.is_none() && !reviewing_replacement {
                                let advanced:Vec<_>=attempt.actions.iter().filter(|offer|
                                    matches!(offer.action,Action::InstallerStartWithPolicy {..})).cloned().collect();
                                ui.add_enabled_ui(fresh,|ui|Self::buttons(ui,&advanced,
                                    overview.current.system.inactive_reason(),pending,chosen));
                            }
                        }
                        ui.add_enabled_ui(fresh, |ui| Self::buttons(ui, std::slice::from_ref(&setup.rename),
                            overview.current.system.inactive_reason(), pending, chosen));
                        ui.small(format!("Installer SHA-256: {}",setup.installer));
                    });
                    let removal: Vec<_> = setup.secondary.iter().filter(|offer| removal_offer(&offer.action))
                        .cloned().collect();
                    let uninstallers: Vec<_> = overview.current.uninstallers.iter().filter(|u|
                        setup.environment.as_deref() == Some(u.environment.as_str())).collect();
                    if !removal.is_empty() || !uninstallers.is_empty() {
                        egui::CollapsingHeader::new("Uninstall, remove or delete").show(ui, |ui| {
                            for uninstaller in &uninstallers {
                                ui.add_enabled_ui(fresh, |ui| Self::buttons(ui, std::slice::from_ref(&uninstaller.action),
                                    overview.current.system.inactive_reason(), pending, chosen));
                                if !uninstaller.affected.is_empty() {
                                    ui.small(format!("Plug-ins it removes: {}", uninstaller.affected.join(", ")));
                                }
                            }
                            ui.label("Remove its plug-ins from your DAW, or delete the whole Windows setup. Deleting also deletes everything installed in it, files saved inside it such as presets, and any licences or sign-ins stored there. Your project files are not changed.");
                            ui.add_enabled_ui(fresh, |ui| Self::buttons(ui, &removal,
                                overview.current.system.inactive_reason(), pending, chosen));
                        });
                    }
                });
            });
        }
        if overview.current.installer_setups.is_empty() {
            ui.add_space(8.0);
            ui.label("Choose the installer you downloaded from your plug-in vendor. Selecting it adds a managed copy; the installer runs only when you choose to start it.");
            ui.label("Choose your DAW when starting installation. Complete any requests in the vendor's window. After installation finishes, the manager finds and connects the installed plug-ins automatically.");
        } else if shown == 0 {
            ui.colored_label(warning_color(ui),
                "The selected installer is unavailable in this readback. Refresh or show all setup.");
        }
        for group in application_groups(&overview.current, focus) {
            ui.push_id(("application-setup", group.environment), |ui| {
                egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
                    ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                    let names = application_group_names(&overview.current, group.environment);
                    let setup_name = group.setup.map(|setup|setup_display_name(setup,&overview.current))
                        .unwrap_or_else(|| names.first().cloned().unwrap_or_else(||group.applications[0].name.clone()));
                    ui.label(egui::RichText::new(format!("Apps for {setup_name}")).size(18.0).strong());
                    if !names.is_empty() { ui.label(format!("Plug-ins in this setup: {}",names.join(", "))); }
                    for app in group.applications.iter().filter(|app| app.details["choose_application"] != true) {
                        ui.push_id(&app.id, |ui| {
                            ui.strong(&app.name);
                            if let Ok(journey) = serde_json::from_value::<InstallationJourney>(
                                app.details["journey"].clone()) {
                                let current = Some((&overview.current, group.environment));
                                ui.strong(current_journey_label(&journey, current));
                                journey_results(ui, &journey, current);
                            } else { ui.label("Open this app to install or manage its plug-ins."); }
                            if let Some(notice) = presentation::vendor_default_notice(app) { ui.label(notice); }
                            if let Some(consequence) = app.actions.iter().find_map(|offer|
                                presentation::vendor_install_consequence(&overview.current, &offer.action)) {
                                ui.label(consequence);
                            }
                            let actions: Vec<_> = app.actions.iter().filter(|offer|match offer.action {
                                Action::VendorApplicationFocus { .. } => app.state == "running",
                                Action::VendorApplicationStop { .. } => app.state != "closed",
                                _ => true,
                            }).cloned().collect();
                            ui.add_enabled_ui(fresh, |ui| Self::buttons(ui, &actions,
                                overview.current.system.inactive_reason(), pending, chosen));
                        });
                    }
                    let choices: Vec<_> = group.applications.iter()
                        .filter(|app|app.details["choose_application"] == true).collect();
                    if !choices.is_empty() {
                        ui.strong("Choose the app to open");
                        ui.label("Select the installed program you want to use in this setup.");
                        let id = ui.make_persistent_id("installed-program-choice");
                        let mut selected = ui.data_mut(|data|data.get_temp::<String>(id));
                        if fresh && selected.as_ref().is_some_and(|id|!choices.iter().any(|app|&app.id == id)) { selected = None; }
                        egui::ComboBox::from_id_salt("installed-program")
                            .selected_text(selected.as_ref().and_then(|id|choices.iter().find(|app|&app.id == id))
                                .map_or("Choose installed program",|app|app.name.as_str()))
                            .show_ui(ui, |ui|for app in &choices {
                                ui.selectable_value(&mut selected,Some(app.id.clone()),&app.name);
                            });
                        let app = selected.as_ref().and_then(|id|choices.iter().find(|app|&app.id == id));
                        if let Some(app) = app {
                            if let Some(offer) = app.actions.iter().find(|offer|
                                matches!(offer.action,Action::VendorSoftwareInstall { .. })) {
                                if let Some(consequence) = presentation::vendor_install_consequence(&overview.current,&offer.action) {
                                    ui.label(consequence);
                                }
                                ui.add_enabled_ui(fresh, |ui| crate::library::primary_action(ui,offer,
                                    overview.current.system.inactive_reason(),pending,chosen));
                            }
                        } else { ui.add_enabled(false,presentation::primary_button("Open selected app",ui)); }
                        ui.data_mut(|data|match selected {Some(value)=>{data.insert_temp(id,value);},None=>{data.remove::<String>(id);}});
                    }
                });
            });
        }
        if !overview.current.deleted_setups.is_empty() {
            egui::CollapsingHeader::new(format!("Deleted setups ({})", overview.current.deleted_setups.len()))
                .default_open(overview.current.deleted_setups.iter().any(|d| d.state == "deleting"))
                .show(ui, |ui| {
                    for deleted in &overview.current.deleted_setups {
                        ui.push_id(("deleted-setup", &deleted.environment), |ui| {
                            ui.strong(&deleted.name);
                            ui.label(match deleted.state.as_str() {
                                "deleting" => "Deleting didn't finish. Finish it to remove the rest of its Windows files.",
                                "unreadable" => "Its history record can't be read. The setup stays hidden; send Setup details.",
                                _ => "Deleted. Its Windows files are gone; its history is kept.",
                            });
                            if !deleted.plugins.is_empty() {
                                ui.small(format!("Plug-ins it held: {}", deleted.plugins.join(", ")));
                            }
                            ui.add_enabled_ui(fresh, |ui| Self::buttons(ui, &deleted.actions,
                                overview.current.system.inactive_reason(), pending, chosen));
                        });
                    }
                });
        }
        ui.small("Vendor accounts and authorization stay with you and the vendor.");
        if ui.button("Technical setup history").clicked() { *page = Page::Diagnostics; }
    }
    fn health_bar(ui: &mut egui::Ui, system: Option<&System>, owned_pause: bool,
        renewing: bool) {
        egui::Frame::group(ui.style()).fill(ui.visuals().faint_bg_color)
            .inner_margin(10.0).show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            let details_id = ui.make_persistent_id("service_details_open");
            let mut expanded = ui.data_mut(|data|data.get_temp::<bool>(details_id)).unwrap_or(false);
            ui.horizontal_wrapped(|ui| {
                // The same row shape is retained through readback transitions.
                let paused = owned_pause && system.is_none_or(|system|
                    system.pending_transactions == 0 && system.stale_transports == 0
                        && (!system.capacity_available() || !system.cleanup_unconfirmed));
                let title = if paused {"Bridge temporarily paused for installation"} else {
                    system.map(presentation::health).unwrap_or(presentation::Health::Unavailable).title()
                };
                let warning = !paused && system.is_none_or(|system|matches!(presentation::health(system),
                    presentation::Health::Unavailable | presentation::Health::NeedsAttention));
                let text = egui::RichText::new(title).size(16.0).strong();
                ui.label(if warning {text.color(warning_color(ui))} else {text});
                if ui.add(egui::Button::new(if expanded {"Hide service details"} else {"Service details"})
                    .min_size(egui::vec2(155.0, 32.0))).clicked() {expanded = !expanded;}
            });
            // Keep this row present so renewal cannot move navigation or an open chooser.
            ui.small(if renewing { "Last observed status · refreshing…" }
                else { "Service status updates automatically." });
            ui.data_mut(|data|data.insert_temp(details_id,expanded));
            if expanded {
                if let Some(system) = system {
                    ui.horizontal_wrapped(|ui| {
                        if system.capacity_available() {
                            ui.small(format!("DSP {} / {}",system.dsp,system.ceiling));
                            ui.small(format!("Install / scan {}",system.maintenance));
                            ui.small(format!("Stale transports {}",system.stale_transports));
                        } else { ui.small("DSP and capacity unknown"); }
                        ui.small(format!("Pending transactions {}",system.pending_transactions));
                        ui.small(presentation::activity_certainty(system).cleanup_label());
                    });
                } else {ui.small("Current service capacity and cleanup are unknown.");}
                ui.small("Plug-in availability is shown separately.");
            }
        });
    }

    fn request_bar(&mut self, ui: &mut egui::Ui) {
        let foreground_pending = self.queued_import || (self.pending && self.origin.is_some_and(RequestOrigin::foreground));
        if self.feedback.is_none() && self.message.is_empty() && !foreground_pending {
            return;
        }
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            ui.spacing_mut().item_spacing.y = 3.0;
            ui.horizontal_wrapped(|ui| {
                ui.strong("Current request");
                if foreground_pending {
                    ui.spinner();
                }
                ui.label(
                    self.feedback
                        .as_ref()
                        .map_or(if foreground_pending && self.message.is_empty() { "Working…" } else { self.message.as_str() }, |feedback| {
                            if feedback.text == "Click received. Waiting for the current readback; no operation submitted yet." {
                                "Checking current status before starting your request…"
                            } else {feedback.text.as_str()}
                        }),
                );
            });
            if let Some(feedback) = &self.feedback {
                if let Some(journey) = &feedback.journey {
                    let available = journey.plugins.iter().filter(|plugin|
                        plugin.stage == InstallationPluginStage::Available).count();
                    let attention = journey.plugins.iter().filter(|plugin| matches!(plugin.stage,
                        InstallationPluginStage::NeedsAttention | InstallationPluginStage::ReplacementRequired)).count();
                    if !journey.plugins.is_empty() {
                        ui.small(format!("{available} plug-ins connected · {attention} need attention"));
                    }
                    if let Some(failure) = &journey.failure {
                        ui.colored_label(warning_color(ui), installation_failure_text(failure));
                    }
                    egui::CollapsingHeader::new("Installation progress").show(ui, |ui| {
                        ui.strong(journey_phase_label(journey));
                        journey_results(ui, journey, None);
                    });
                }
                egui::CollapsingHeader::new("Request details").show(ui, |ui| {
                    ui.label(&feedback.text);
                    ui.label(
                        feedback
                            .operation
                            .as_deref()
                            .unwrap_or("Operation identity not yet confirmed"),
                    );
                    ui.label(feedback.transitions.join(" / "));
                });
            }
            if !self.feedback.as_ref().is_some_and(|f| f.blocking)
                && ui.add_sized([100.0, 44.0], egui::Button::new("Dismiss")).clicked() {
                self.feedback = None;
                self.message.clear();
            }
        });
    }

    fn attention_list(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        limit: usize,
        page: &mut Page,
        library: &mut crate::library::Library,
        focus: &mut RouteFocus,
    ) {
        let items = presentation::attentions(snapshot);
        if items.is_empty() {
            ui.label("Nothing needs attention right now.");
            return;
        }
        for (index, item) in items.iter().take(limit).enumerate() {
            ui.group(|ui| {
                ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                ui.horizontal_wrapped(|ui| {
                    ui.strong(&item.title);
                    let button = egui::Button::new(if index == 0 {
                        "Review this"
                    } else {
                        "Open item"
                    })
                    .min_size(egui::vec2(128.0, 44.0));
                    if ui.add(button).clicked() {
                        navigate(&item.destination, page, library, focus);
                    }
                });
                ui.label(&item.detail);
            });
        }
        if items.len() > limit
            && ui
                .add_sized([180.0, 44.0], egui::Button::new("See all attention items"))
                .clicked()
        {
            *page = Page::Activity;
        }
    }

    fn session_row(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        row: &serde_json::Value,
        index: usize,
        page: &mut Page,
        library: &mut crate::library::Library,
        activity_focus: &mut Option<String>,
    ) {
        let class_id = row["class_id"].as_str().unwrap_or("");
        let product = presentation::session_product(snapshot, class_id);
        let frame = ui.push_id((row["recent"] == true, index), |ui| {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                ui.strong(presentation::session_display_name(snapshot, class_id));
                let state = if row["recent"] == true {
                    format!(
                        "Most recent failed session · {}",
                        presentation::terminal_text(row["terminal"].as_str().unwrap_or(""))
                    )
                } else if row["state"] == "cleanup_unconfirmed" {
                    "Cleanup unconfirmed · this owner is not proof of live audio".into()
                } else if let Some(terminal) = row["terminal"].as_str() {
                    format!(
                        "Session unavailable · {}",
                        presentation::terminal_text(terminal)
                    )
                } else {
                    "Running bridge session".into()
                };
                ui.label(state);
                if row["recent"] == true {
                    ui.small(
                        if row["cleanup_confirmed"] == true && row["transport_retired"] == true {
                            "Cleanup and transport retirement confirmed"
                        } else {
                            "Cleanup or transport retirement is unconfirmed"
                        },
                    );
                }
                if let Some(product) = product {
                    if ui
                        .add_sized([150.0, 44.0], egui::Button::new("View plug-in"))
                        .clicked()
                    {
                        library.focus_product(ProductKey::from(product));
                        *page = Page::Plugins;
                    }
                }
                egui::CollapsingHeader::new("Technical session details")
                    .show(ui, |ui| Self::value(ui, row));
            })
        });
        if activity_focus.as_deref() == row["session"].as_str() && activity_focus.is_some() {
            ui.scroll_to_rect(frame.response.rect, Some(egui::Align::Center));
            *activity_focus = None;
        }
    }

    fn activity(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        pending: bool,
        page: &mut Page,
        library: &mut crate::library::Library,
        focus: &mut RouteFocus,
        chosen: &mut Option<Action>,
    ) {
        ui.heading("Activity");
        ui.small("Bridge sessions, manager work and retained incidents. Managed DAW workspaces will have separate application activity when the manager reports it.");
        ui.heading("Live bridge sessions");
        if !snapshot.system.capacity_available() {
            ui.colored_label(warning_color(ui), "Current session status is unavailable. Rows below are the last successful snapshot, not a live confirmation.");
        }
        let live: Vec<_> = snapshot
            .active_sessions
            .iter()
            .filter(|row| row["recent"] != true)
            .collect();
        if live.is_empty() {
            ui.label(if snapshot.system.capacity_available() {
                "No bridge sessions are reported."
            } else {
                "No current session list is available."
            });
        }
        for (index, row) in live.into_iter().enumerate() {
            Self::session_row(ui, snapshot, row, index, page, library, &mut focus.activity);
        }
        ui.separator();
        ui.heading("Recent session failures");
        let recent: Vec<_> = snapshot
            .active_sessions
            .iter()
            .filter(|row| row["recent"] == true)
            .collect();
        if recent.is_empty() {
            ui.label("No recent terminal session is retained.");
        }
        for (index, row) in recent.into_iter().enumerate() {
            Self::session_row(ui, snapshot, row, index, page, library, &mut focus.activity);
        }
        ui.separator();
        ui.heading("Needs attention");
        Self::attention_list(ui, snapshot, usize::MAX, page, library, focus);
        ui.separator();
        ui.heading("Incidents");
        if snapshot.recent_incidents.is_empty() {
            ui.label("No sanitized incident is retained.");
        }
        for incident in snapshot.recent_incidents.iter().rev() {
            ui.push_id(&incident.id, |ui| {
                egui::CollapsingHeader::new(format!(
                    "{} · {}",
                    incident.state.replace('_', " "),
                    incident.id
                ))
                .show(ui, |ui| {
                    let lines = incident_lines(&incident.summary);
                    for line in lines.iter().take(2) {
                        ui.label(line);
                    }
                    if let Some(offer) = &incident.export {
                        Self::buttons(
                            ui,
                            std::slice::from_ref(offer),
                            snapshot.system.inactive_reason(),
                            pending,
                            chosen,
                        );
                    }
                    egui::CollapsingHeader::new("Sanitized technical report").show(ui, |ui| {
                        for line in lines.iter().skip(2) {
                            ui.small(line);
                        }
                        Self::value(ui, &incident.summary);
                    });
                });
            });
        }
    }

    fn diagnostics(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        pending: bool,
        chosen: &mut Option<Action>,
    ) {
        ui.heading("Diagnostics");
        crate::library::diagnostics(ui, snapshot, pending, chosen, true);
        ui.heading("Manager actions");
        for offer in &snapshot.actions {
            if snapshot.capture["armed"] == true && matches!(offer.action, Action::CaptureDisarm {})
            {
                continue; // already shown next to the armed state above
            }
            Self::buttons(
                ui,
                std::slice::from_ref(offer),
                snapshot.system.inactive_reason(),
                pending,
                chosen,
            );
        }
        egui::CollapsingHeader::new("Vendor applications and managed setup details").show(ui, |ui| {
            for app in &snapshot.vendor_applications {
                ui.strong(&app.name);
                Self::buttons(ui,&app.actions,snapshot.system.inactive_reason(),pending,chosen);
                for value in [&app.details["dependency_operation"],&app.details] {
                    if value["dependency"].is_object() {
                        for line in dependency_retirement_lines(value) {ui.small(line);}
                    }
                }
                Self::value(ui,&app.details);
            }
            for environment in &snapshot.environments {
                ui.strong(&environment.family);
                Self::buttons(ui,&environment.actions,snapshot.system.inactive_reason(),pending,chosen);
                ui.small(format!("Environment: {} · revision {} · runner {}",
                    environment.id,environment.revision,environment.runner));
                Self::value(ui,&environment.last_scan);
            }
        });
        egui::CollapsingHeader::new("Technical system details").show(ui, |ui| {
            if let Ok(value) = serde_json::to_value(&snapshot.system) {
                Self::value(ui, &value);
            }
            Self::value(ui, &snapshot.capture);
            for runtime in &snapshot.managed_runtime_records { Self::value(ui,runtime); }
        });
    }

    fn workspaces(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        pending: bool,
        chosen: &mut Option<Action>,
        pick: &mut bool,
        focus: &mut RouteFocus,
    ) {
        ui.heading("Workspaces");
        ui.small("Managed DAWs have their own workspace and installation history. Native DAW plug-in publications remain in Plug-ins.");
        if snapshot.workspaces.is_empty() {
            ui.label("No managed DAW workspace is available here yet.");
            return;
        }
        if ui
            .add_enabled(
                !pending,
                egui::Button::new("Import Windows installer").min_size(egui::vec2(240.0, 44.0)),
            )
            .clicked()
        {
            focus.replacement = None;
            *pick = true;
        }
        ui.small("The manager copies and verifies the selected file in private installer custody. Choose its exact release under the application or product below.");
        for workspace in &snapshot.workspaces {
            ui.push_id(&workspace.id, |ui| {
                ui.group(|ui| {
                    ui.heading(&workspace.name);
                    ui.strong(workspace_state_label(&workspace.state));
                    ui.label(format!("Selected version: {}", workspace.selected_release));
                    if let Some(release) = &workspace.installed_advertised_release {
                        ui.label(format!("Installed from declared release: {release}"));
                        if let Some(version) = &workspace.observed_file_version {
                            ui.label(format!("Observed executable version: {version}"));
                        }
                    }
                    if let Some(operation) = &workspace.active_installation_operation {
                        ui.label(format!("Installation in progress: {operation}"));
                    }
                    if workspace.cleanup != "confirmed" {
                        ui.colored_label(warning_color(ui), format!("Workspace cleanup: {}", workspace.cleanup.replace('_', " ")));
                    }
                    if let Some(failure) = &workspace.first_useful_failure {
                        ui.colored_label(warning_color(ui), failure.replace('_', " "));
                    }
                    if let Some(primary_index) = workspace_primary_action(&workspace.actions) {
                        let primary = &workspace.actions[primary_index];
                        let response = ui.add_enabled(
                            !pending && primary.disabled_reason.is_none(),
                            egui::Button::new(egui::RichText::new(&primary.label).strong())
                                .min_size(egui::vec2(240.0, 48.0)),
                        );
                        if response.clicked() {
                            *chosen = Some(primary.action.clone());
                        }
                        if let Some(reason) = primary.disabled_reason.as_deref() {
                            ui.small(reason);
                        } else if pending {
                            ui.small("Waiting for the current request or manager readback");
                        }
                        for (index, offer) in workspace.actions.iter().enumerate() {
                            if index != primary_index {
                                Self::buttons(ui, std::slice::from_ref(offer), None, pending, chosen);
                            }
                        }
                    }
                    egui::CollapsingHeader::new("Choose an imported version")
                        .show(ui, |ui| {
                            ui.label("Enter the exact release shown by the installer. Selection does not run Windows code or change workspace files.");
                            if workspace.installer_choices.is_empty() {
                                ui.label("No admitted Windows installer is available in manager custody.");
                            }
                            Self::buttons(ui, &workspace.installer_choices, None, pending, chosen);
                        });
                    for product in &workspace.products {
                        ui.separator();
                        ui.push_id(product.id, |ui| {
                            ui.heading(&product.name);
                            ui.strong(workspace_product_state_label(&product.state));
                            if let Some(release) = &product.selected_release {
                                ui.label(format!("Selected plug-in version: {release}"));
                            }
                            if let Some(failure) = &product.current_failure {
                                ui.colored_label(warning_color(ui), failure.replace('_', " "));
                            }
                            if let Some(index) = workspace_primary_action(&product.actions) {
                                let primary = &product.actions[index];
                                let response = ui.add_enabled(
                                    !pending && primary.disabled_reason.is_none(),
                                    egui::Button::new(egui::RichText::new(&primary.label).strong())
                                        .min_size(egui::vec2(240.0, 48.0)),
                                );
                                if response.clicked() {
                                    *chosen = Some(primary.action.clone());
                                }
                                if let Some(reason) = primary.disabled_reason.as_deref() {
                                    ui.small(reason.replace('_', " "));
                                }
                                for (other, offer) in product.actions.iter().enumerate() {
                                    if other != index {
                                        Self::buttons(ui, std::slice::from_ref(offer), None, pending, chosen);
                                    }
                                }
                            }
                            egui::CollapsingHeader::new("Choose an imported plug-in version")
                                .show(ui, |ui| {
                                    if product.installer_choices.is_empty() {
                                        ui.label("No admitted installer for this plug-in is in manager custody.");
                                    }
                                    Self::buttons(ui, &product.installer_choices, None, pending, chosen);
                                });
                            egui::CollapsingHeader::new("Technical plug-in installation history")
                                .show(ui, |ui| {
                                    if let Some(sha) = &product.module_sha256 {
                                        ui.label(format!("Installed module SHA-256: {sha}"));
                                    }
                                    Self::value(ui, &product.details);
                                });
                        });
                    }
                    egui::CollapsingHeader::new("Technical workspace details and history")
                        .show(ui, |ui| {
                            ui.label(format!("Workspace: {}", workspace.id));
                            ui.label(format!("Selected installer SHA-256: {}", workspace.selected_installer));
                            if let Some(image) = &workspace.installed_image_sha256 {
                                ui.label(format!("Installed image SHA-256: {image}"));
                            }
                            Self::value(ui, &workspace.details);
                        });
                });
            });
        }
    }
    fn expire_overview(&mut self) {
        if self.overview_fresh && self.last_overview.elapsed() >= Duration::from_secs(60) {
            self.overview_fresh = false;
            self.overview_renewing = true;
            self.refresh_after = true;
        }
    }
    fn product_details(ui: &mut egui::Ui, product: &Product) {
        for limit in &product.limitations { ui.label(limit.replace('_', " ")); }
        if let Some(preparation) = product.details.get("preparation") {
            Self::preparation_details(ui, preparation);
        }
        if let Some(revision) = product.active_revision {
            ui.label(format!("Published revision: {revision} · Recommended: {}",
                product.recommended_revision.map(|value| value.to_string())
                    .unwrap_or_else(|| "none".into())));
        }
        for history in &product.history {
            ui.label(format!("{} · {}{}", history.description, history.claim,
                if history.active { " · active" } else { "" }));
            if let Some(reason) = &history.rollback_unavailable { ui.label(reason); }
        }
        egui::CollapsingHeader::new("Technical product details").show(ui, |ui| {
            ui.label(format!("Status: {}\nClass: {}\nModule SHA-256: {}\nEnvironment: {}\nRunner: {}",
                product.disposition, product.class_id, product.module_sha256,
                product.environment, product.runner));
            Self::value(ui, &product.details);
        });
    }
    fn repaint_delay(&self) -> Duration {
        if self.frontend_superseded { return Duration::from_secs(60); }
        if self.prompt_pulse { return Duration::from_millis(50); }
        let interval = if self.pending && !self.background_poll {
            Duration::from_millis(500)
        } else if self.operation_live {
            Duration::from_secs(2)
        } else {
            Duration::from_secs(10)
        };
        interval.saturating_sub(self.last_poll.elapsed()).max(Duration::from_millis(50))
    }
}
impl eframe::App for Operator {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(reply) = self.receiver.try_recv() {
            self.handle_reply(reply);
        }
        self.expire_overview();
        // The new version's Setup continues the update; this window closes.
        if self.updates.poll(ui.ctx(), self.plugins_in_use()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        let readiness_visible = self.current_readiness().or_else(|| self.last_readiness()).is_some();
        let controls_pending = self.controls_pending();
        let actions_current = self.current_action_snapshot().is_some();
        let action_controls_pending = controls_pending || (!self.preview && !actions_current);
        let mut refresh = false;
        let mut pick = false;
        let mut chosen = None;
        let mut retry_details = false;
        let selected_product_snapshot = self.selected_product_snapshot();
        egui::CentralPanel::default().frame(egui::Frame::central_panel(ui.style()).inner_margin(16.0)).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Linux VST Bridge");
                let refresh_label = if self.overview_fresh { "Refresh" } else { "Check again" };
                if ui.add_enabled(!controls_pending, egui::Button::new(refresh_label).min_size(egui::vec2(92.0, 44.0))).clicked() {
                    refresh = true;
                }
                if self.pending && self.origin == Some(RequestOrigin::ExplicitRefresh) {
                    ui.small("Refreshing…");
                }
            });
            if self.frontend_superseded {
                ui.strong("Bridge updated — this window is paused");
                ui.label("Unfinished inputs remain in this window. Continue in the current manager to see accepted operations and use the updated bridge.");
                if ui.button("Open current manager").clicked() {
                    if let Err(error) = crate::bootstrap::launch_selected() { self.message = error; }
                }
            }
            if self.preview { ui.small("LOCAL DESIGN PREVIEW · sample records · actions do not execute"); }
            Self::health_bar(ui, self.current_health_system(), self.current_owned_pause(),
                self.overview_renewing);
            if let Some(line) = self.updates.headline().filter(|_| self.page != Page::Setup) {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(line);
                    if ui.add(egui::Button::new("View update").min_size(egui::vec2(0.0, 36.0))).clicked() {
                        self.page = Page::Setup;
                    }
                });
            }
            navigation(ui, &mut self.page);
            self.request_bar(ui);
            ui.separator();
            egui::ScrollArea::vertical().id_salt(("manager-page", self.page)).show(ui, |ui| {
                if self.page == Page::Setup && !self.preview {
                    self.updates.show(ui);
                    ui.add_space(8.0);
                }
                if self.overview.is_some() || !self.preview {
                    if let Some(overview) = &self.overview {
                        match self.page {
                            Page::Home => {
                                Self::fast_home(ui, overview, self.readiness.as_ref()
                                    .filter(|_| readiness_visible).map(|(assessment, _, _)| assessment), actions_current,
                                    controls_pending, (&mut self.page, &mut self.library, &mut self.focus, &mut refresh, &mut chosen));
                                return;
                            }
                            Page::Setup => {
                                Self::fast_setup(ui, overview, actions_current,
                                    controls_pending, (&mut refresh, &mut chosen, &mut pick,
                                        &mut self.page, &mut self.library, &mut self.focus,
                                        &mut self.software_install_daw));
                                return;
                            }
                            Page::Plugins => {
                                if let Some(selected) = &selected_product_snapshot {
                                    if !self.overview_fresh {
                                        ui.colored_label(warning_color(ui),
                                            if self.overview_renewing {"Last observed plug-in status · refreshing…"}
                                            else {"Current status is unavailable. Setup actions wait for current readback."});
                                    }
                                    self.library.show(ui, selected, action_controls_pending,
                                        &mut chosen, Self::product_details);
                                } else {
                                    let attempted = self.library.focused_product().is_some_and(|key|
                                        self.product_attempted.as_ref() == Some(key));
                                    self.library.show_current(ui, &overview.current,
                                        actions_current, attempted && !self.pending,
                                        &mut retry_details);
                                }
                                return;
                            }
                            _ => {}
                        }
                    } else if matches!(self.page, Page::Home | Page::Setup) {
                        ui.heading(self.page.label());
                        ui.label("Checking current manager status. Readiness and actions will appear after readback.");
                        return;
                    }
                }
                let Some(snapshot) = &self.snapshot else {
                    ui.label("Loading technical details. Home and Setup remain available.");
                    if self.details_attempted && !self.pending
                        && ui.button("Try loading details again").clicked() {
                        self.details_attempted = false;
                    }
                    return;
                };
                let page = self.page;
                match page {
                    Page::Home | Page::Setup => {ui.label("Loading current plug-ins and setup…");},
                    Page::Plugins => self.library.show(ui, snapshot,
                        action_controls_pending, &mut chosen, Self::product_details),
                    Page::Workspaces => Self::workspaces(ui, snapshot, action_controls_pending, &mut chosen, &mut pick, &mut self.focus),
                    Page::Activity => Self::activity(ui, snapshot, action_controls_pending, &mut self.page,
                        &mut self.library, &mut self.focus, &mut chosen),
                    Page::Diagnostics => Self::diagnostics(ui, snapshot, action_controls_pending, &mut chosen),
                }
            });
        });
        if let Some(a) = chosen.take() {
            if let Some(installation) = installation_with_daw(&a,
                self.software_install_daw.unwrap_or(DawChoice::Other)) {
                if self.software_install_daw.is_some() {
                    chosen = Some(installation);
                } else {
                    self.software_install_form = Some(installation);
                }
            } else if matches!(
                a,
                Action::CandidateObserve { .. } | Action::CandidateReview { .. }
                    | Action::CompatibilityResult { .. }
            ) {
                self.product_form = Some(a);
            } else if matches!(
                a,
                Action::WorkspaceSelectInstaller { .. }
                    | Action::WorkspaceSelectProductInstaller { .. }
            ) {
                self.workspace_select_form = Some(a);
            } else if matches!(a, Action::InstallerRename { .. }) {
                self.installer_rename_form = Some(a);
            } else if needs_confirmation(&a) {
                self.removal_form = Some(a);
            } else {
                chosen = Some(a);
            }
        }
        if let Some(form) = &self.software_install_form {
            let offered = installation_form_offer(self.current_action_snapshot(), form);
            if let Some((submit, cancel)) = show_software_install_form(ui.ctx(),
                &mut self.software_install_daw, &offered, action_controls_pending,
                matches!(form,Action::EnvironmentRefresh {..})) {
                if submit.clicked() {
                    if let Some(daw) = self.software_install_daw {
                        chosen = installation_with_daw(form, daw);
                        self.software_install_form = None;
                    }
                } else if cancel.clicked() {
                    self.software_install_form = None;
                }
            }
        }
        if let Some(form) = self.workspace_select_form.as_mut() {
            let (title, installer, release) = match form {
                Action::WorkspaceSelectInstaller { installer, release } => {
                    ("Choose exact FL Studio version", installer, release)
                }
                Action::WorkspaceSelectProductInstaller {
                    installer, release, ..
                } => ("Choose exact plug-in version", installer, release),
                _ => unreachable!("only workspace selections open this form"),
            };
            let mut submit = false;
            let mut cancel = false;
            egui::Window::new(title)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.label(format!("Imported installer SHA-256: {installer}"));
                    ui.label("Exact installer release");
                    ui.add(egui::TextEdit::singleline(release).char_limit(32).min_size(egui::vec2(220.0, 44.0)));
                    ui.small("The manager verifies this installer in canonical custody before changing the selection.");
                    submit = ui.add_enabled(!release.is_empty() && !action_controls_pending, egui::Button::new("Select version").min_size(egui::vec2(180.0, 44.0))).clicked();
                    cancel = ui.add_sized([100.0, 44.0], egui::Button::new("Cancel")).clicked();
                });
            if submit {
                chosen = self.workspace_select_form.clone();
            } else if cancel {
                self.workspace_select_form = None;
            }
        }
        if let Some(Action::InstallerRename { installer, label }) = self.installer_rename_form.as_mut() {
            let mut submit = false;
            let mut cancel = false;
            egui::Window::new("Name installer").collapsible(false).show(ui.ctx(), |ui| {
                ui.label("This name is for display only. It cannot change the installer, environment or plug-ins.");
                ui.small(format!("Exact installer ID: {}…{}", &installer[..8], &installer[60..]));
                ui.add(egui::TextEdit::singleline(label).char_limit(96)
                    .min_size(egui::vec2(240.0, 44.0)));
                let plausible = plausible_installer_label(label);
                submit = ui.add_enabled(plausible && !action_controls_pending,
                    egui::Button::new("Save name").min_size(egui::vec2(180.0, 44.0))).clicked();
                cancel = ui.add_sized([100.0, 44.0], egui::Button::new("Cancel")).clicked();
            });
            if submit { chosen = self.installer_rename_form.clone(); }
            else if cancel { self.installer_rename_form = None; }
        }
        if let Some(form) = self.removal_form.clone() {
            let snapshot = if matches!(form, Action::PluginRemoveFromDaw { .. }) {
                selected_product_snapshot.as_ref().or_else(|| self.current_action_snapshot())
            } else { self.current_action_snapshot() }.or(if self.preview { self.snapshot.as_ref() } else { None });
            let confirmation = removal_confirmation(snapshot, &form);
            let mut submit = false;
            let mut cancel = false;
            // Optional, default off: also forget the imported installer file.
            let finishing = matches!(&form, Action::SetupDelete { environment, .. } if snapshot.is_some_and(|s|
                s.deleted_setups.iter().any(|d| &d.environment == environment)));
            let forget_id = egui::Id::new(("forget-installer", format!("{form:?}")));
            let mut forget = ui.ctx().data(|d| d.get_temp::<bool>(forget_id)).unwrap_or(false);
            egui::Window::new(confirmation.title).collapsible(false).resizable(false).show(ui.ctx(), |ui| {
                for line in &confirmation.body { ui.label(line); }
                if matches!(form, Action::SetupDelete { .. }) && !finishing {
                    ui.checkbox(&mut forget, "Also forget the installer file");
                    ui.small("You would need to add the installer again to set this up later.");
                }
                if let Err(reason) = &confirmation.offer { ui.colored_label(warning_color(ui), reason); }
                ui.horizontal_wrapped(|ui| {
                    submit = ui.add_enabled(confirmation.offer.is_ok() && !action_controls_pending,
                        egui::Button::new(egui::RichText::new(confirmation.confirm).strong())
                            .min_size(egui::vec2(200.0, 44.0))).clicked();
                    cancel = ui.add_sized([120.0, 44.0], egui::Button::new(confirmation.cancel)).clicked();
                });
            });
            ui.ctx().data_mut(|d| d.insert_temp(forget_id, forget));
            if submit {
                chosen = Some(match confirmed(form) {
                    Action::SetupDelete { installer, environment, confirmed, .. } if !finishing =>
                        Action::SetupDelete { installer, environment, confirmed, forget_installer: forget },
                    other => other,
                });
            }
            else if cancel {
                // A cancelled confirmation forgets the musician's unsent choice.
                ui.ctx().data_mut(|d| d.remove::<bool>(forget_id));
                self.removal_form = None;
            }
        }
        let result_offer = self.product_form.as_ref().and_then(|form| {
            let Action::CompatibilityResult { candidate, expected_current, .. } = form else { return None };
            let snapshot = if self.current_action_snapshot().is_some() {
                selected_product_snapshot.as_ref().or_else(|| self.current_action_snapshot())
            } else if self.preview { self.snapshot.as_ref() } else { None };
            Some(compatibility_result_offer(snapshot, candidate, expected_current))
        });
        if let Some(form) = self.product_form.as_mut() {
            let (submit, cancel) = show_product_form(ui.ctx(), form, result_offer.as_ref(), action_controls_pending)
                .map(|(submit, cancel)| (submit.clicked(), cancel.clicked())).unwrap_or_default();
            if submit {
                chosen = self.product_form.clone();
            } else if cancel {
                self.product_form = None;
            }
        }
        if self.preview {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
            return;
        }
        if self.frontend_superseded { return; }
        if retry_details {
            if self.page == Page::Plugins { self.product_attempted = None; }
            else { self.details_attempted = false; }
        }
        if pick { self.queued_import = true; }
        if refresh { self.explicit_refresh_queued = true; }
        if let Some(a) = chosen {
            self.capture_chosen_action(a);
        }
        if !self.pending && self.queued_import {
            self.queued_import = false;
            self.request(Query::PickInstaller, RequestOrigin::InstallerImport, ui.ctx());
        } else if let Some(request) = self.next_action() {
            self.request(Query::Action(request), RequestOrigin::UserAction, ui.ctx());
        } else if !self.pending && self.prompt_pulse {
            self.prompt_pulse = false;
            self.request(Query::Pulse, RequestOrigin::BackgroundActivity, ui.ctx());
        } else if !self.pending && (self.explicit_refresh_queued || self.refresh_after)
            && (self.explicit_refresh_queued || self.retry_after.is_none_or(|at| Instant::now() >= at)) {
            let origin = if self.explicit_refresh_queued { RequestOrigin::ExplicitRefresh }
                else { RequestOrigin::SilentPostMutationRefresh };
            let query = self.overview_query();
            self.explicit_refresh_queued = false;
            self.refresh_after = false;
            self.request(query, origin, ui.ctx());
        } else if !self.pending && self.page == Page::Plugins && self.overview_fresh
            && self.selected_product_snapshot().is_none()
            && self.library.focused_product().is_some_and(|key|
                self.product_attempted.as_ref() != Some(key)) {
            let key = self.library.focused_product().expect("selected product").clone();
            self.product_attempted = Some(key.clone());
            self.request(Query::Product(key), RequestOrigin::ProductDetail, ui.ctx());
        } else if !self.pending && self.snapshot.is_none() && !self.details_attempted
            && !matches!(self.page, Page::Home | Page::Setup | Page::Plugins) {
            self.details_attempted = true;
            self.request(Query::Snapshot, RequestOrigin::DetailSnapshot, ui.ctx());
        } else if !self.pending && self.page == Page::Activity
            && self.last_poll.elapsed() > Duration::from_secs(10) {
            self.request(Query::Activity, RequestOrigin::BackgroundActivity, ui.ctx());
        } else if !self.pending && self.last_poll.elapsed() >
            Duration::from_secs(if self.operation_live { 2 } else { 10 })
            && self.retry_after.is_none_or(|at| Instant::now() >= at) {
            self.request(Query::Pulse, RequestOrigin::BackgroundActivity, ui.ctx());
        }
        ui.ctx().request_repaint_after(self.repaint_delay());
    }
}
fn software_install_offer(snapshot: Option<&Snapshot>, installer: &str, policy: Option<crate::model::Powershell>)
    -> Result<Option<String>, &'static str> {
    let snapshot = snapshot.ok_or("Current manager status is unavailable. Refresh before installing.")?;
    snapshot.installer_setups.iter().flat_map(|setup|
        setup.primary.iter().chain(setup.secondary.iter()))
        .find(|offer| offer.action.software_install().is_some_and(|(offered,_,choice)|
            offered==installer && choice==policy))
        .map(|offer| offer.disabled_reason.clone().or_else(||
            offer.action.requires_global_inactive().then(||snapshot.system.inactive_reason())
                .flatten().map(str::to_owned)))
        .ok_or("This installation offer changed. Refresh Setup before continuing.")
}

fn installation_with_daw(action: &Action, daw: DawChoice) -> Option<Action> {
    if let Some(action)=action.software_install_with_daw(daw) {return Some(action);}
    match action {
        Action::VendorSoftwareInstall { application, .. } =>
            Some(Action::VendorSoftwareInstall { application: application.clone(), daw }),
        Action::EnvironmentRefresh {environment,..}=>
            Some(Action::EnvironmentRefresh {environment:environment.clone(),daw}),
        Action::RendererSoftwareInstall {application,policy,..}=>
            Some(Action::RendererSoftwareInstall {application:application.clone(),policy:*policy,daw}),
        _ => None,
    }
}

fn installation_form_offer(snapshot: Option<&Snapshot>, action: &Action)
    -> Result<Option<String>, &'static str> {
    if let Some((installer,_,policy))=action.software_install() {
        return software_install_offer(snapshot,installer,policy);
    }
    match action {
        Action::RendererSoftwareInstall {application,policy,..}=> {
            let snapshot=snapshot.ok_or("Current manager status is unavailable. Refresh before continuing.")?;
            snapshot.vendor_applications.iter().flat_map(|app|&app.actions).find(|offer|
                matches!(&offer.action,Action::RendererSoftwareInstall {application:offered,policy:offered_policy,..}
                    if offered==application && offered_policy==policy))
                .map(|offer|offer.disabled_reason.clone().or_else(||snapshot.system.inactive_reason().map(str::to_owned)))
                .ok_or("This app's offered rendering choice changed. Refresh Setup before continuing.")
        },
        Action::EnvironmentRefresh {environment,..}=> {
            let snapshot=snapshot.ok_or("Current manager status is unavailable. Refresh before continuing.")?;
            snapshot.installer_setups.iter().flat_map(|setup|setup.primary.iter().chain(setup.secondary.iter()))
                .find(|offer|matches!(&offer.action,Action::EnvironmentRefresh {environment:offered,..} if offered==environment))
                .map(|offer|offer.disabled_reason.clone().or_else(||snapshot.system.inactive_reason().map(str::to_owned)))
                .ok_or("This setup refresh offer changed. Refresh Setup before continuing.")
        },
        Action::VendorSoftwareInstall { application, .. } => {
            let snapshot = snapshot.ok_or("Current manager status is unavailable. Refresh before installing.")?;
            snapshot.vendor_applications.iter().flat_map(|app| &app.actions)
                .find(|offer| matches!(&offer.action,
                    Action::VendorSoftwareInstall { application: offered, .. } if offered == application))
                .map(|offer| offer.disabled_reason.clone().or_else(||
                    offer.action.requires_global_inactive().then(||snapshot.system.inactive_reason())
                        .flatten().map(str::to_owned)))
                .ok_or("This vendor installation offer changed. Refresh Setup before continuing.")
        }
        _ => Err("This installation offer changed. Refresh Setup before continuing."),
    }
}

/// Removing from the DAW or deleting a setup always asks first.
fn needs_confirmation(action: &Action) -> bool {
    matches!(action, Action::PluginRemoveFromDaw { .. } | Action::SetupRemoveFromDaw { .. }
        | Action::SetupDelete { .. } | Action::ProductUninstall { .. })
}
/// Setup-level removal offers live in their own section of the setup card.
fn removal_offer(action: &Action) -> bool {
    matches!(action, Action::SetupRemoveFromDaw { .. } | Action::SetupDelete { .. })
}
/// The musician's answer to the confirmation. Delete and Uninstall carry it.
fn confirmed(action: Action) -> Action {
    match action {
        Action::SetupDelete { installer, environment, forget_installer, .. } =>
            Action::SetupDelete { installer, environment, confirmed: true, forget_installer },
        Action::ProductUninstall { environment, registration, command_sha256, .. } =>
            Action::ProductUninstall { environment, registration, command_sha256, confirmed: true },
        other => other,
    }
}
/// The offer a confirmed request answers: without the musician's own choices.
fn offer_identity(action: &Action) -> Action {
    match action.clone() {
        Action::SetupDelete { installer, environment, .. } =>
            Action::SetupDelete { installer, environment, confirmed: false, forget_installer: false },
        Action::ProductUninstall { environment, registration, command_sha256, .. } =>
            Action::ProductUninstall { environment, registration, command_sha256, confirmed: false },
        other => other,
    }
}
struct Confirmation {
    title: &'static str,
    body: Vec<String>,
    confirm: &'static str,
    cancel: &'static str,
    /// The exact current offer, or why it can't be confirmed now.
    offer: Result<AvailableAction, String>,
}
fn removal_confirmation(snapshot: Option<&Snapshot>, action: &Action) -> Confirmation {
    const CHANGED: &str = "This choice is no longer offered. Close this and check the current status.";
    // The musician's own answers are not part of the offer.
    let offered = offer_identity(action);
    let current = |offers: Vec<&AvailableAction>| -> Result<AvailableAction, String> {
        let offer = offers.into_iter().find(|offer| offer.action == offered).cloned().ok_or(CHANGED)?;
        match &offer.disabled_reason { Some(reason) => Err(reason.clone()), None => Ok(offer) }
    };
    let Some(snapshot) = snapshot else {
        return Confirmation { title: "Check current status", body: vec![], confirm: "Continue",
            cancel: "Close", offer: Err("Current manager status is unavailable. Refresh before continuing.".into()) };
    };
    match action {
        Action::PluginRemoveFromDaw { class_id, .. } => {
            let product = snapshot.products.iter().find(|p| p.class_id.eq_ignore_ascii_case(class_id)
                && p.actions.iter().any(|offer| offer.action == *action));
            let name = product.map_or("This plug-in", |p| p.name.as_str());
            Confirmation { title: "Remove from your DAW?",
                body: vec![format!("{name} will no longer appear in your DAW's plug-in list."),
                    "Projects that use it will open with it missing until you add it back.".into(),
                    "Its Windows installation and settings stay in its setup, so you can add it back later, unless you delete that setup.".into()],
                confirm: "Remove from my DAW", cancel: "Keep it",
                offer: current(product.into_iter().flat_map(|p| p.actions.iter()).collect()) }
        }
        Action::SetupRemoveFromDaw { environment } => {
            let names: Vec<_> = snapshot.products.iter().filter(|p| p.environment == *environment
                && p.availability.as_ref().is_some_and(|a| a.available)).map(|p| p.name.as_str()).collect();
            let names = if names.is_empty() { "This setup's plug-ins".into() } else { names.join(", ") };
            Confirmation { title: "Remove from your DAW?",
                body: vec![format!("{names} will no longer appear in your DAW's plug-in list."),
                    "Projects that use them will open with them missing until you add them back.".into(),
                    "Their Windows installation stays in this setup. If you delete this setup afterwards, they can't be added back; you would set it up again.".into()],
                confirm: "Remove from my DAW", cancel: "Keep them",
                offer: current(snapshot.installer_setups.iter().flat_map(|s| s.secondary.iter()).collect()) }
        }
        Action::SetupDelete { installer, environment, .. } => {
            if let Some(deleted) = snapshot.deleted_setups.iter().find(|d| d.environment == *environment) {
                return Confirmation { title: "Finish deleting this setup?",
                    body: vec![format!("Deleting the Windows setup for {} didn't finish. This removes the rest of its files.", deleted.name),
                        "Your projects are not changed.".into()],
                    confirm: "Finish deleting", cancel: "Not now",
                    offer: current(deleted.actions.iter().collect()) };
            }
            let setup = snapshot.installer_setups.iter().find(|s| s.installer == *installer);
            let name = setup.map_or_else(|| "this setup".into(), |s| setup_display_name(s, snapshot));
            // Every attempt of the setup, not only the current one.
            let attempts: std::collections::BTreeSet<&str> = std::iter::once(environment.as_str())
                .chain(setup.into_iter().flat_map(|s| s.history.iter().filter_map(|h| h.environment.as_deref())))
                .collect();
            let names: Vec<_> = snapshot.products.iter().filter(|p| attempts.contains(p.environment.as_str()))
                .map(|p| p.name.as_str()).collect();
            let (plugins, them) = match names.len() {
                0 => ("its plug-ins".into(), "them"),
                1 => (names[0].to_owned(), "it"),
                _ => (names.join(", "), "them"),
            };
            Confirmation { title: "Delete this setup?",
                body: vec![format!("This deletes the Windows setup for {name}: everything installed in it, and any licences or sign-ins stored there."),
                    "Files you saved from inside its plug-ins or apps and kept in this setup, such as presets or downloaded content, are deleted too.".into(),
                    "If the vendor limits how many computers can be activated, deactivate the licence in the vendor's app first. Deleting the files doesn't give the activation back.".into(),
                    format!("Projects that use {plugins} will open with {them} missing until you set this setup up again. Add back won't be possible."),
                    "The installer you added stays in the manager, so you can set it up again later, unless you choose to forget it below. A record of this setup is kept in its history. Your project files are not changed.".into()],
                confirm: "Delete setup", cancel: "Cancel",
                offer: current(setup.into_iter().flat_map(|s| s.secondary.iter()).collect()) }
        }
        Action::ProductUninstall { .. } => {
            let uninstaller = snapshot.uninstallers.iter().find(|u| u.action.action == offered);
            let name = uninstaller.map_or("this software", |u| u.name.as_str());
            let mut body = vec![format!("This runs {name}'s own uninstaller in its setup. Follow its window to finish.")];
            if uninstaller.is_some_and(|u| u.coverage != "installed_with_this") {
                body.push("The manager can't tell exactly what this removes: it was installed together with other software, or what it installed wasn't fully recorded. It might be a shared part, such as a runtime or a licence manager, that other software in this setup needs. Uninstall it only if you're sure.".into());
            }
            if let Some(u) = uninstaller.filter(|u| !u.affected.is_empty()) {
                let one = u.affected.len() == 1;
                body.push(format!("First, {} will be removed from your DAW. Projects that use {} will open with {} missing.",
                    u.affected.join(", "), if one { "it" } else { "them" }, if one { "it" } else { "them" }));
            }
            body.push("Afterwards, this setup's plug-ins are checked again one at a time, and any it didn't remove go back into your DAW. The bridge is paused for all your setups until that finishes, which can take a few minutes for a large setup.".into());
            body.push("The uninstaller may also remove licences or settings it stored there. Your projects are not changed.".into());
            Confirmation { title: "Uninstall?", body, confirm: "Uninstall", cancel: "Cancel",
                offer: current(uninstaller.map(|u| &u.action).into_iter().collect()) }
        }
        _ => Confirmation { title: "Check current status", body: vec![], confirm: "Continue", cancel: "Close",
            offer: Err(CHANGED.into()) },
    }
}

fn daw_choice_control(ui: &mut egui::Ui, daw: &mut Option<DawChoice>) {
    egui::ComboBox::from_id_salt("software-install-daw")
        .selected_text(daw.map(presentation::daw_name).unwrap_or("Choose your DAW"))
        .show_ui(ui, |ui| {
            for choice in [DawChoice::Bitwig, DawChoice::Reaper, DawChoice::Other] {
                ui.selectable_value(daw, Some(choice), presentation::daw_name(choice));
            }
        });
}

fn show_software_install_form(ctx: &egui::Context, daw: &mut Option<DawChoice>,
    offer: &Result<Option<String>, &'static str>, pending: bool, refresh: bool)
    -> Option<(egui::Response, egui::Response)> {
    egui::Window::new(if refresh {"Find and connect"} else {"Install and connect"})
        .collapsible(false)
        .max_width((ctx.content_rect().width() - 40.0).clamp(300.0, 460.0))
        .max_height((ctx.content_rect().height() - 40.0).max(200.0))
        .show(ctx, |ui| {
            let refusal = match offer { Ok(reason) => reason.as_deref(), Err(reason) => Some(*reason) };
            let footer_height = 2.0 * (44.0 + ui.spacing().item_spacing.y)
                + refusal.map(|reason| ui.painter().layout(reason.to_owned(),
                    egui::TextStyle::Small.resolve(ui.style()), ui.visuals().text_color(),
                    ui.available_width()).size().y + ui.spacing().item_spacing.y).unwrap_or(0.0);
            egui::ScrollArea::vertical().id_salt("software-install-fields")
                .max_height((ui.available_height() - footer_height).max(0.0))
                .min_scrolled_height(0.0).auto_shrink([false, true]).show(ui, |ui| {
                    ui.label("Which Linux DAW do you use?");
                    daw_choice_control(ui, daw);
                    ui.label(if refresh {"The manager finds installed instruments and effects and connects missing plug-ins automatically. Your working plug-ins keep their current setup."}
                        else {"Complete requests in the vendor's window. The manager then finds and connects all newly installed VST3 plug-ins automatically."});
                    ui.small("Native VST3 plug-ins may also appear in other compatible Linux DAWs.");
                });
            let submit = ui.add_enabled(!pending && daw.is_some() && refusal.is_none(),
                presentation::primary_button(if refresh {"Find and connect"} else {"Start installation"}, ui));
            if let Some(reason) = refusal { ui.small(reason); }
            let cancel = ui.add(egui::Button::new("Cancel").min_size(egui::vec2(100.0, 44.0)));
            (submit, cancel)
        }).and_then(|window| window.inner)
}

fn show_product_form(ctx: &egui::Context, form: &mut Action,
    result_offer: Option<&Result<Option<String>, &'static str>>,
    action_controls_pending: bool) -> Option<(egui::Response, egui::Response)> {
    egui::Window::new(if matches!(form, Action::CompatibilityResult { .. }) {
        "Record plug-in test result"
    } else { "Exact candidate observation / review" })
        .collapsible(false)
        .max_height((ctx.content_rect().height() - 40.0).max(200.0))
        .max_width((ctx.content_rect().width() - 40.0).max(300.0))
        .show(ctx, |ui| {
            // Keep both actions visible; only the fields and their explanations scroll.
            let refusal_height = if matches!(form, Action::CompatibilityResult { .. }) {
                compatibility_result_refusal(result_offer, &TestResultKind::Worked).map(|reason| {
                    ui.painter().layout(reason.to_owned(), egui::TextStyle::Small.resolve(ui.style()),
                        ui.visuals().text_color(), ui.available_width()).size().y
                        + ui.spacing().item_spacing.y
                }).unwrap_or(0.0)
            } else { 0.0 };
            let footer_height = 2.0 * (44.0 + ui.spacing().item_spacing.y) + refusal_height;
            let (valid, label, refusal) = egui::ScrollArea::vertical()
                .id_salt("product-form-fields")
                .max_height((ui.available_height() - footer_height).max(0.0))
                .min_scrolled_height(0.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    match form {
                        Action::CompatibilityResult { result, passed, failed_area, note, .. } => {
                            ui.label("Mark only what you actually observed. The result records only the checks you observed with this setup; it does not establish support for every configuration.");
                            ui.horizontal_wrapped(|ui| {
                                ui.selectable_value(result, TestResultKind::Worked, "It worked");
                                ui.selectable_value(result, TestResultKind::Problem {
                                    category: ProblemCategory::Other }, "Report a problem");
                            });
                            if let TestResultKind::Problem { category } = result {
                                egui::ComboBox::from_id_salt("guided-problem-category")
                                    .selected_text(problem_category_label(*category))
                                    .show_ui(ui, |ui| {
                                        for choice in [ProblemCategory::FailedToLoad,
                                            ProblemCategory::BlankEditor, ProblemCategory::EditorFroze,
                                            ProblemCategory::EditorDisappeared, ProblemCategory::NoAudio,
                                            ProblemCategory::IncorrectAudio, ProblemCategory::ControlsUnresponsive,
                                            ProblemCategory::HostCrashed, ProblemCategory::CleanupIncomplete,
                                            ProblemCategory::Other] {
                                            ui.selectable_value(category, choice, problem_category_label(choice));
                                        }
                                    });
                                if *category == ProblemCategory::Other {
                                    ui.label("Which part had the problem?");
                                    egui::ComboBox::from_id_salt("guided-problem-area")
                                        .selected_text(failed_area.map(test_area_label).unwrap_or("Choose an area"))
                                        .show_ui(ui, |ui| {
                                            for area in test_areas() {
                                                ui.selectable_value(failed_area, Some(area), test_area_label(area));
                                            }
                                        });
                                } else { *failed_area = None; }
                            } else { *failed_area = None; }
                            ui.label("Checks you confirmed");
                            for area in test_areas() {
                                let mut checked = passed.contains(&area);
                                if ui.checkbox(&mut checked, test_area_label(area)).changed() {
                                    if checked { passed.push(area); } else { passed.retain(|item| *item != area); }
                                }
                            }
                            ui.label("Optional detail for a successful test; describe a problem when reporting one");
                            ui.add(egui::TextEdit::singleline(note).char_limit(512)
                                .desired_width(360.0));
                            let valid = match result {
                                TestResultKind::Worked => !passed.is_empty(),
                                TestResultKind::Problem { category } => !note.trim().is_empty()
                                    && (*category != ProblemCategory::Other || failed_area.is_some()),
                            };
                            let refusal = compatibility_result_refusal(result_offer, result);
                            if !valid { ui.small("Select an observed check, or choose a problem and describe it."); }
                            if refusal.is_some() && matches!(result_offer, Some(Ok(_))) {
                                ui.small("You can report a problem now. A successful result needs this plug-in's clean retirement.");
                            }
                            (valid && refusal.is_none(), "Record this test result", refusal)
                        }
                        Action::CandidateObserve{area,status,note,..}=>{
                            ui.label("This records your observation. It does not create a machine measurement or qualification decision.");
                            egui::ComboBox::from_id_salt("area").selected_text(area.as_str()).show_ui(ui,|ui|{for a in ["daw_load","midi","audio","editor","parameters","automation","state_recall","processing_restart","retirement"]{ui.selectable_value(area,a.into(),a.replace('_'," "));}});
                            egui::ComboBox::from_id_salt("status").selected_text(status.as_str()).show_ui(ui,|ui|{for a in ["passed","failed","not_tested","unavailable","not_applicable"]{ui.selectable_value(status,a.into(),a.replace('_'," "));}});
                            ui.add(egui::TextEdit::multiline(note).char_limit(512));
                            (!note.trim().is_empty(), "Record operator observation", None)
                        }
                        Action::CandidateReview{accept,rationale,..}=>{
                            ui.label(if *accept{"Maintainer review: accept this exact local configuration against the retained results. This is not a project-wide support claim. Publication remains a separate action."}else{"Record why this exact configuration needs more work. Publication is unchanged."});
                            ui.add(egui::TextEdit::multiline(rationale).char_limit(512));
                            (!rationale.trim().is_empty(), "Record explicit review decision", None)
                        }
                        _ => (false, "Record result", None),
                    }
                }).inner;
            let submit = ui.add_enabled(valid && !action_controls_pending,
                egui::Button::new(label).min_size(egui::vec2(240.0, 44.0)));
            if let Some(reason) = refusal { ui.small(reason); }
            let cancel = ui.add_sized([100.0, 44.0], egui::Button::new("Cancel"));
            (submit, cancel)
        }).and_then(|window| window.inner)
}

fn compatibility_result_offer(snapshot: Option<&Snapshot>, candidate: &str,
    expected_current: &PublicationIdentity) -> Result<Option<String>, &'static str> {
    let snapshot = snapshot.ok_or("Current manager readback unavailable")?;
    let offer = snapshot.products.iter().flat_map(|product| {
        product.actions.iter().chain(product.compatibility.iter().flat_map(|workflow|
            workflow.primary.iter().chain(workflow.alternatives.iter())))
    }).find(|offer| matches!(&offer.action, Action::CompatibilityResult {
        candidate: offered_candidate, expected_current: offered_current, ..
    } if offered_candidate == candidate && offered_current == expected_current))
        .ok_or("This exact test-result offer is no longer current. Refresh the plug-in's controls.")?;
    Ok(offer.disabled_reason.clone())
}

fn compatibility_result_refusal<'a>(offer: Option<&'a Result<Option<String>, &'static str>>,
    result: &TestResultKind) -> Option<&'a str> {
    match offer {
        Some(Err(reason)) => Some(*reason),
        Some(Ok(reason)) if matches!(result, TestResultKind::Worked) => reason.as_deref(),
        Some(Ok(_)) => None,
        None => Some("Current test-result offer unavailable"),
    }
}

fn installer_lines(v: &serde_json::Value) -> Vec<String> { crate::presentation::installer_lines(v) }

fn dependency_retirement_lines(v: &serde_json::Value) -> Vec<String> {
    if v.is_null() {
        return Vec::new();
    }
    let r = if v["dependency_retirement"].is_object() {
        &v["dependency_retirement"]
    } else {
        &v["dependency"]
    };
    let mut lines = vec![
        format!(
            "Dependency service stop: {}",
            if r["service_retirement_confirmed"] == true {
                "confirmed"
            } else {
                "not confirmed"
            }
        ),
        format!(
            "Dependency process cleanup: {}",
            if r["process_cleanup_confirmed"] == true {
                "confirmed"
            } else {
                "not confirmed"
            }
        ),
        format!(
            "Forced cleanup: {}",
            if r["forced_cleanup_used"] == true {
                "used; does not confirm a clean service stop"
            } else {
                "not reported"
            }
        ),
    ];
    lines.push(match r["dependency_cleanup_disposition"].as_str() {
        Some("graceful_service_retirement")=>"Native Access closed with graceful dependency retirement.".into(),
        Some("exact_owned_session_cleanup")=>"Native Access closed with exact-owned compatibility cleanup; graceful service shutdown was not confirmed.".into(),
        Some("reverified_after_unconfirmed_cleanup")=>"The last session did not confirm it closed; the bridge later confirmed nothing from it was still running.".into(),
        _=>"Native Access or dependency cleanup is not confirmed.".into(),
    });
    if let Some(error) = v["error"].as_str() {
        let explanation=match error {
            "dependency_running_not_ready"=>"The service did not establish exact process and listener readiness; Native Access was not opened.",
            "dependency_recovery_registration_missing"=>"The verified daemon exists, but its service registration is missing. Recovery did not reinstall it.",
            "dependency_recovery_payload_changed"|"dependency_generation_changed"=>"The installed daemon differs from the admitted bundle payload. No replacement was attempted.",
            "dependency_foreign_conflict"|"dependency_same_prefix_unowned"=>"A conflicting or unowned daemon prevents preparation. It was not adopted or stopped.",
            "dependency_command_timeout"=>"A service or installer command did not provide its required acknowledgment in time.",
            "dependency_installer_nonzero"=>"The installer returned a nonzero result. Installed files and service readiness are separate facts.",
            _=>"The dependency operation did not complete; inspect the retained step and retirement results.",
        };
        lines.push(format!("Dependency failure: {error}. {explanation}"));
    }
    if v["dependency"]["recovery"]["authority"]
        == "qualified_bundle_payload_and_retired_installation"
    {
        lines.push("Recovering the verified installed dependency without reinstalling; the earlier installer failure remains recorded.".into());
        lines.push(
            if v["dependency"]["ready_tested"] == true {
                "Fresh service readiness: verified."
            } else {
                "Fresh service readiness: not established."
            }
            .into(),
        );
    }
    if let Some(stages) = v["dependency"]["stages"].as_array() {
        for stage in stages.iter().filter(|s| s["action"] == "install") {
            let result = &stage["installer_result"];
            lines.push(match result["installer_exit"].as_u64().filter(|n| *n<=u32::MAX as u64) {
                Some(code) if result["authority"]=="exact_windows_child_handle_exit"=>format!("Dependency installer exit: {code}. This alone does not establish readiness."),
                _=>"Dependency installer exit: unavailable.".into(),
            });
            lines.push(match stage["exit"].as_i64() {
                Some(code) => format!("Dependency runner exit: {code}."),
                None => "Dependency runner retirement was not observed when the command returned."
                    .into(),
            });
        }
    }
    let observation = &r["stop_observation"];
    if observation["schema"] == 3 {
        let explanation=match observation["classification"].as_str() {
            Some("NAD2_STOP_CONFIRMED")=>"Exact service, process-generation, and listener retirement was confirmed.",
            Some("NAD2_STOP_SUBMITTED_PROGRESSING")=>"A stop was submitted or already pending and bounded progress was observed; retirement was not confirmed.",
            Some("NAD2_STOP_SUBMITTED_NO_TRANSITION")=>"The stop request was submitted, but no transition toward service retirement was observed before the bound.",
            Some("NAD2_STOP_NOT_SUBMITTED")=>"The stop request was not submitted; the retained control error is authoritative.",
            Some("NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS")=>"SCM reported stopped while the exact process or an owned listener remained.",
            Some("NAD2_STOP_OBSERVATION_UNAVAILABLE")=>"The bounded stop observation was unavailable.",
            _=>"The bounded stop classification was unavailable.",
        };
        lines.push(format!(
            "Bounded stop response: {} {explanation}",
            observation["classification"]
                .as_str()
                .unwrap_or("unavailable")
        ));
        lines.push(format!("SCM state: {} -> {}; controls masks: {} -> {}; checkpoint: {} -> {}; wait hint: {} -> {} ms.",
            observation["initial_state"],observation["final_state"],observation["initial_controls_accepted"],
            observation["final_controls_accepted"],observation["initial_checkpoint"],observation["checkpoint"],
            observation["initial_wait_hint_ms"],observation["wait_hint_ms"]));
        match observation["control_count"].as_u64() {
            Some(0)=>lines.push(match observation["initial_state"].as_u64() {
                Some(1)=>"No ControlService request was issued; the service was already stopped.".into(),
                Some(3)=>"No ControlService request was issued; the service was already stopping.".into(),
                _=>"No ControlService request was issued; the service state could not be evaluated.".into(),
            }),
            Some(1) if observation["control_submitted"]==true=>lines.push(format!("ControlService result: submitted in {} ms; returned state {}, controls mask {}, checkpoint {}, wait hint {} ms.",
                observation["control_elapsed_ms"],observation["control_return"]["state"],
                observation["control_return"]["controls_accepted"],observation["control_return"]["checkpoint"],
                observation["control_return"]["wait_hint_ms"])),
            Some(1)=>lines.push(format!("The ControlService request was not submitted; error {}.",observation["control_error"])),
            _=>{},
        }
        lines.push(format!("Exact process wait: {} ({}); listener mask: {} -> {}; distinct transitions: {} total, {} retained, {} dropped.",
            observation["process_wait_class"],observation["process_wait"],observation["initial_listener_mask"],
            observation["listener_mask"],observation["transition_count_total"],observation["transition_count_retained"],
            observation["transition_count_dropped"]));
    }
    lines
}

fn installer_policy_lines(v: &serde_json::Value) -> Vec<&'static str> {
    let Some(policy) = v.get("installer_capability") else {
        return vec![];
    };
    let requested = policy["requested"]["powershell"].as_str();
    let applied = policy["effective"]["effective"]["windows_scripting"]["powershell"].as_str();
    vec![match requested {
        Some("intentionally_unavailable")=>"Requested: PowerShell intentionally unavailable for this installer only.",
        Some("inherited")=>"Requested: inherited scripting behavior.",
        _=>"Scripting policy request could not be verified.",
    }, match applied {
        Some("intentionally_unavailable")=>"Applied at target launch: PowerShell intentionally unavailable. Installer fallback success is not yet established.",
        Some("inherited")=>"Applied at target launch: inherited scripting behavior.",
        _=>"Target-launch policy has not been confirmed applied.",
    }]
}

fn failure_lines(f: &crate::model::OperationFailure) -> Vec<String> {
    let mut lines = vec![match f.stage {
        crate::model::FailureStage::OperatorValidationReadback => {
            "Manager could not finish validating state."
        }
        crate::model::FailureStage::EnvironmentCreationAdmission => {
            "Manager was busy before environment creation could start."
        }
    }
    .into()];
    if !f.mutation_started && !f.environment_created {
        lines.push("Environment creation did not start.".into());
    }
    if !f.installer_launched {
        lines.push("The installer was not launched; this is not an installer failure.".into());
    }
    lines.push(
        if f.retryable {
            "Retry is safe once the manager refreshes and canonical state is healthy."
        } else {
            "Manager access needs attention before retrying."
        }
        .into(),
    );
    lines.push(format!(
        "Validation lock: {:?}; {:?}; {} attempts, {:.1} ms elapsed; holder unknown.",
        f.lock.name,
        f.lock.outcome,
        f.lock.attempts,
        f.lock.elapsed_wait_us as f64 / 1000.0
    ));
    lines
}
fn incident_lines(v: &serde_json::Value) -> Vec<String> {
    let word = |key: &str| v[key].as_str().unwrap_or("unavailable").replace('_', " ");
    let confirmed = |key: &str| match v[key].as_bool() {
        Some(true) => "confirmed",
        Some(false) => "not confirmed",
        None => "unavailable",
    };
    vec![
        format!("Outcome: {}", word("outcome")),
        format!(
            "Cleanup: {} · Transport retirement: {}",
            confirmed("cleanup_confirmed"),
            confirmed("transport_retired")
        ),
        format!(
            "Outer runner exit (separate from Windows): {}",
            v["outer_exit"]
        ),
        format!(
            "Windows reported exit codes: {}",
            v["windows_self_exit_codes"]
        ),
        format!(
            "Retained exception observations: {}",
            v["exceptions"]
                .as_array()
                .map(|items| items.len().to_string())
                .unwrap_or_else(|| "unavailable".into())
        ),
    ]
}
/// A current overview has already verified this app-only handoff. Cheap polls
/// still carry the unchanged historical empty-discovery result. Preserve that
/// one projection while comparing every other field; polls grant no authority.
fn retain_projected_handoff(current: Option<&serde_json::Value>, incoming: &mut Option<serde_json::Value>) {
    let (Some(current), Some(raw)) = (current, incoming.as_ref()) else { return; };
    let Ok(failure) = serde_json::from_value::<InstallationFailure>(raw["journey"]["failure"].clone()) else { return; };
    if current["operation"].as_str().is_none_or(|id| id.is_empty())
        || current["operation"] != raw["operation"]
        || current["journey"]["phase"] != "awaiting_application"
        || !current["journey"]["failure"].is_null()
        || !current["journey"]["plugins"].as_array().is_some_and(Vec::is_empty)
        || raw["journey"]["phase"] != "needs_attention"
        || failure.stage != InstallationFailureStage::Discovery
        || failure.code != "no_supported_instruments_or_effects_discovered"
        || !failure.retryable { return; }
    let mut projected = raw.clone();
    projected["journey"]["phase"] = current["journey"]["phase"].clone();
    projected["journey"]["failure"] = serde_json::Value::Null;
    if projected == *current { *incoming = Some(projected); }
}

fn refresh_for_receipt(old: &Option<serde_json::Value>, new: &Option<serde_json::Value>) -> bool {
    old != new
        && new
            .as_ref()
            .and_then(|v| v["state"].as_str())
            .is_some_and(|s| matches!(s, "vendor_running" | "completed" | "refused"))
}

fn refresh_for_workspace_product(snapshot: &Snapshot, install_finish_ready: bool) -> bool {
    install_finish_ready
        && snapshot
            .workspaces
            .iter()
            .flat_map(|workspace| &workspace.products)
            .any(|product| {
                product.id == WorkspaceProductId::Serum2
                    && !product.actions.iter().any(|offer| {
                        matches!(
                            offer.action,
                            Action::WorkspaceFinishProductInstall {
                                product: WorkspaceProductId::Serum2
                            }
                        ) && offer.disabled_reason.is_none()
                    })
            })
}
#[cfg(test)]
mod tests {
    fn automatic_setup(journey: InstallationJourney, action: Option<Action>) -> InstallerSetup {
        InstallerSetup {
            installer:"ab".repeat(32), name:"Chosen software".into(), label_source:"operator_named".into(),
            byte_size:1024, format:"pe_executable".into(), imported_at:1,
            phase:SetupPhase::DiscoveryComplete, journey:Some(journey),
            status:"Installation results are retained in the same setup.".into(),
            environment:Some("exact-environment".into()), compatibility:None, discovered:vec![],
            primary:action.map(|action|AvailableAction {label:"Continue".into(),action,disabled_reason:None}),
            secondary:vec![],rename:AvailableAction {label:"Rename".into(),
                action:Action::InstallerRename {installer:"ab".repeat(32),label:String::new()},disabled_reason:None},
            history:vec![],
        }
    }

    fn automatic_journey(phase: InstallationPhase) -> InstallationJourney {
        InstallationJourney {
            daw:DawChoice::Bitwig, phase, failure:None,
            plugins:vec![InstallationPlugin {selection:"instrument-selection".into(),
                class_id:"instrument-class".into(),name:"Installed instrument".into(),
                stage:InstallationPluginStage::Available,failure:None},
                InstallationPlugin {selection:"effect-selection".into(),class_id:"effect-class".into(),
                    name:"Installed effect".into(),stage:InstallationPluginStage::NeedsAttention,
                    failure:Some(InstallationFailure {stage:InstallationFailureStage::Inspection,
                        reason:"The plug-in did not load during inspection.".into(),
                        code:"fixture_load_failed".into(),retryable:true})}],
        }
    }

    #[test]
    fn automatic_journey_keeps_successful_siblings_and_exact_failed_phase_continuation() {
        for (width,pending) in [(560.0,false),(960.0,false),(560.0,true)] {
            let mut overview:CurrentOverview = overview_fixture().into();
            overview.current.products.clear();
            let resume = Action::SoftwareInstallResume {onboarding:"exact-environment".into()};
            overview.current.installer_setups = vec![automatic_setup(
                automatic_journey(InstallationPhase::NeedsAttention),Some(resume.clone()))];
            let ctx = result_form_context();
            let mut chosen = None;
            let mut point = egui::Pos2::ZERO;
            for frame in 0..5 {
                let events = if frame >= 3 {vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed:frame == 3,modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output = ctx.run_ui(egui::RawInput {events,screen_rect:Some(
                    egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,1000.0))),
                    ..Default::default()},|ui| {
                    Operator::fast_home(ui,&overview,None,true,pending,
                        (&mut Page::Home,&mut crate::library::Library::default(),
                            &mut RouteFocus::default(),&mut false,&mut chosen));
                });
                if frame == 2 {
                    let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
                    assert!(labels.iter().any(|(label,_)|label=="Installed instrument"));
                    assert!(labels.iter().any(|(label,_)|label=="Available in your DAW"));
                    assert!(labels.iter().any(|(label,_)|label=="Installed effect"));
                    assert!(labels.iter().any(|(label,_)|label==
                        "Plug-in inspection: The plug-in did not load during inspection."));
                    assert!(!labels.iter().any(|(label,_)|label.contains("activation failed")
                        || label=="Prepare for your DAW" || label=="Make available in your DAW"));
                    point=labels.iter().find(|(label,_)|label=="Continue installation and DAW connection").unwrap().1;
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen,(!pending).then_some(resume));
        }
    }

    #[test]
    fn missing_support_keeps_install_as_the_ordinary_action_and_manual_download_in_diagnostics() {
        let mut overview = wrong_installer_overview(None);
        overview.current.installer_setups.remove(0);
        overview.current.managed_runtime_records.clear();
        overview.current.actions = vec![AvailableAction {
            label:"Install compatibility runtime (785 MB download)".into(),
            action:Action::RuntimeInstall {},disabled_reason:None,
        }];
        let installer = overview.current.installer_setups[0].installer.clone();
        let (_,chosen) = wrong_installer_click(&overview,&mut RouteFocus::default(),
            "Install and connect to your DAW",960.0,true);
        assert_eq!(installation_with_daw(&chosen.unwrap(),DawChoice::Bitwig),
            Some(Action::SoftwareInstall {installer,daw:DawChoice::Bitwig}));
        for page in [Page::Setup,Page::Diagnostics] {
            let ctx = result_form_context();
            let mut output = ctx.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,egui::vec2(960.0,1200.0))),..Default::default()},|ui| {
                if page == Page::Setup {
                    Operator::fast_setup(ui,&overview,true,false,(&mut false,&mut None,&mut false,
                        &mut Page::Setup,&mut crate::library::Library::default(),
                        &mut RouteFocus::default(),&mut Some(DawChoice::Bitwig)));
                } else { Operator::diagnostics(ui,&overview.current,false,&mut None); }
            });
            let mut labels = vec![];
            for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            output.textures_delta.clear();
            assert_eq!(labels.iter().any(|(label,_)|label.starts_with("Download Windows plug-in support")),
                page == Page::Diagnostics);
            assert!(!labels.iter().any(|(label,_)|label == "Windows plug-in support"));
        }
    }

    #[test]
    fn automatic_pre_environment_progress_is_visible_on_home_and_setup_without_duplicate_start() {
        let mut overview: CurrentOverview = overview_fixture().into();
        overview.current.products.clear();
        overview.current.environments.clear();
        overview.current.onboarding.clear();
        let mut journey = automatic_journey(InstallationPhase::PreparingEnvironment);
        journey.daw = DawChoice::Reaper;
        journey.plugins.clear();
        let mut setup = automatic_setup(journey.clone(), None);
        setup.phase = SetupPhase::Imported;
        setup.environment = None;
        setup.status = "Getting ready to install.".into();
        overview.current.operation = Some(serde_json::json!({
            "operation":"exact-automatic-install", "state":"running", "journey":journey,
            "action":{"kind":"software_install", "installer":setup.installer,"daw":"reaper"}
        }));
        overview.current.installer_setups = vec![setup];
        for page in [Page::Home, Page::Setup] {
            let ctx = result_form_context();
            let mut current_page = page;
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO,
                    egui::vec2(960.0, 720.0))), ..Default::default()
            }, |ui| {
                if page == Page::Home {
                    Operator::fast_home(ui, &overview, None, true, false,
                        (&mut current_page, &mut crate::library::Library::default(),
                            &mut RouteFocus::default(), &mut false, &mut chosen));
                } else {
                    Operator::fast_setup(ui, &overview, true, false,
                        (&mut false, &mut chosen, &mut false, &mut current_page,
                            &mut crate::library::Library::default(), &mut RouteFocus::default(),
                            &mut Some(DawChoice::Reaper)));
                }
            });
            let mut labels = vec![];
            for shape in &output.shapes { text_centers(&shape.shape, &mut labels); }
            output.textures_delta.clear();
            assert!(labels.iter().any(|(label, _)| label.contains("Getting ready to install")));
            assert!(labels.iter().any(|(label, _)| label == "Your DAW: REAPER"));
            assert!(!labels.iter().any(|(label, _)| label.contains("Install compatibility runtime in Setup")
                || label == "Install and connect to your DAW" || label == "Continue installation and DAW connection"));
            assert!(chosen.is_none());
        }
    }

    #[test]
    fn owned_installation_pause_does_not_turn_normal_progress_into_service_repair() {
        for case in ["owned", "missing", "other", "refused", "failed", "pending", "stale"] {
            let mut overview:CurrentOverview = overview_fixture().into();
            overview.current.products.clear();
            overview.current.system.service = "capacity unavailable".into();
            overview.current.system.cleanup_unconfirmed = true;
            overview.current.system.pending_transactions = usize::from(case == "pending");
            overview.current.system.stale_transports = usize::from(case == "stale");
            let mut journey = automatic_journey(InstallationPhase::PreparingPlugins);
            journey.plugins.clear();
            if case == "failed" {journey.phase = InstallationPhase::NeedsAttention;}
            overview.current.installer_setups = vec![automatic_setup(journey.clone(), None)];
            let mut operation = serde_json::json!({"operation":"active-installation",
                "state":if case == "refused" {"refused"} else {"running"},
                "journey":journey,"owned_service_pause":{"schema":1,"owner_operation":"exact-child",
                    "automatic_operation":if case == "other" {"another-installation"} else {"active-installation"}}});
            if case == "missing" {operation.as_object_mut().unwrap().remove("owned_service_pause");}
            overview.current.operation = Some(operation);
            overview.current.actions = vec![AvailableAction {label:"Reconcile interrupted transaction".into(),
                action:Action::TransactionReconcile {},disabled_reason:Some("Service capacity unavailable".into())}];
            let ctx = result_form_context();
            let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,egui::vec2(960.0,900.0))),..Default::default()}, |ui| {
                Operator::health_bar(ui,Some(&overview.current.system),
                    presentation::owned_service_pause(overview.current.operation.as_ref()), false);
                Operator::fast_home(ui,&overview,None,true,false,
                    (&mut Page::Home,&mut crate::library::Library::default(),
                        &mut RouteFocus::default(),&mut false,&mut chosen));
            });
            let mut labels = vec![];
            for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            output.textures_delta.clear();
            let paused = case == "owned";
            assert_eq!(labels.iter().any(|(label,_)|label == "Bridge temporarily paused for installation"),paused,"{case}");
            assert_eq!(labels.iter().any(|(label,_)|label == "Next: resolve the bridge service problem"),!paused,"{case}");
            assert_eq!(labels.iter().any(|(label,_)|label == "Reconcile interrupted transaction"),!paused,"{case}");
            if paused {assert!(labels.iter().any(|(label,_)|label.contains("Connecting plug-ins to your DAW")));}
            assert!(!overview.current.system.capacity_available());
            assert!(chosen.is_none());
        }
    }

    #[test]
    fn observed_cleanup_failure_vetoes_owned_pause_on_home_after_pulse_and_refresh() {
        let mut overview:CurrentOverview = overview_fixture().into();
        overview.current.products.clear();
        overview.current.system.service = "capacity unavailable".into();
        overview.current.system.cleanup_unconfirmed = true;
        let mut journey = automatic_journey(InstallationPhase::PreparingPlugins);journey.plugins.clear();
        overview.current.installer_setups = vec![automatic_setup(journey.clone(),None)];
        overview.current.operation = Some(serde_json::json!({"operation":"current-op","state":"running",
            "journey":journey,"owned_service_pause":{"schema":1,"owner_operation":"child-op",
                "automatic_operation":"current-op"}}));
        let mut state = state_fixture();state.apply_current_overview(overview.clone());
        let mut pulse = pulse_fixture(true);pulse.service_state = "inactive".into();
        pulse.operation = overview.current.operation.clone();pulse.cleanup_unconfirmed = Some(true);
        state.handle_reply(Reply::Pulse(pulse));
        assert!(!state.current_owned_pause());
        let mut unavailable = pulse_fixture(true);unavailable.operation = overview.current.operation.clone();
        unavailable.cleanup_unconfirmed = None;
        state.handle_reply(Reply::Pulse(unavailable));
        assert!(!state.current_owned_pause(), "An unavailable observation must not erase a known cleanup failure");
        for refresh in [false,true] {
            if refresh {state.apply_current_overview(overview.clone());}
            let current = state.overview.as_ref().unwrap();
            assert!(!presentation::owned_service_pause(current.current.operation.as_ref()));
            let ctx = result_form_context();let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,egui::vec2(960.0,720.0))),..Default::default()}, |ui| {
                Operator::fast_home(ui,current,None,true,false,(&mut Page::Home,
                    &mut crate::library::Library::default(),&mut RouteFocus::default(),&mut false,&mut chosen));
            });
            let mut labels = vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            output.textures_delta.clear();
            assert!(labels.iter().any(|(label,_)|label == "Next: resolve the bridge service problem"));
            assert!(chosen.is_none());
        }
        let mut recovered = pulse_fixture(true);recovered.operation = overview.current.operation.clone();
        recovered.cleanup_unconfirmed = Some(false);
        state.handle_reply(Reply::Pulse(recovered));state.apply_current_overview(overview);
        assert!(state.current_owned_pause(), "A fresh recovery observation permits normal owned progress again");
    }

    #[test]
    fn installation_rows_use_exact_progress_and_keep_available_or_failed_siblings_truthful() {
        let mut overview:CurrentOverview = overview_fixture().into();
        overview.current.products.truncate(1);
        let first = &mut overview.current.products[0];
        first.disposition = "installed_unqualified".into();
        first.availability = Some(NativeAvailability {available:false,reason:Some("No native connection yet".into())});
        first.details = serde_json::json!({});first.compatibility = None;first.actions.clear();
        let mut sibling = first.clone();
        sibling.class_id = "different-class".into();sibling.name = "Already available sibling".into();
        sibling.disposition = "ready".into();
        sibling.availability = Some(NativeAvailability {available:true,reason:None});
        let environment = first.environment.clone();
        let mut journey = automatic_journey(InstallationPhase::PreparingPlugins);
        journey.plugins = vec![InstallationPlugin {selection:"exact-selection".into(),class_id:first.class_id.clone(),
            name:first.name.clone(),stage:InstallationPluginStage::Inspecting,failure:None}];
        overview.current.operation = Some(serde_json::json!({"operation":"exact-operation","state":"running",
            "onboarding":environment,"journey":journey}));
        let mut setup = automatic_setup(journey,None);setup.environment = Some(environment);
        setup.discovered = vec![DiscoveredProduct {environment:first.environment.clone(),
            module_sha256:first.module_sha256.clone(),class_id:first.class_id.clone(),name:first.name.clone()}];
        overview.current.installer_setups = vec![setup];overview.current.products.push(sibling);
        let first = &overview.current.products[0];
        assert_eq!(presentation::installation_progress(&overview.current,first),Some("Checking this plug-in"));
        assert!(presentation::installation_progress(&overview.current,&overview.current.products[1]).is_none());
        let mut another = first.clone();another.module_sha256 = "different-module".into();
        assert!(presentation::installation_progress(&overview.current,&another).is_none());
        another = first.clone();another.disposition = "needs_attention".into();
        assert!(presentation::installation_progress(&overview.current,&another).is_none());
        let mut ambiguous = overview.current.clone();
        let mut duplicate = ambiguous.installer_setups[0].discovered[0].clone();duplicate.module_sha256 = "other-module".into();
        ambiguous.installer_setups[0].discovered.push(duplicate);
        assert!(presentation::installation_progress(&ambiguous,first).is_none());
        let mut failed = overview.current.clone();
        failed.operation.as_mut().unwrap()["journey"]["plugins"][0]["stage"] = serde_json::json!("needs_attention");
        assert!(presentation::installation_progress(&failed,first).is_none());
        for home in [true,false] {
            let ctx = result_form_context();let mut chosen = None;
            let mut output = ctx.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,egui::vec2(960.0,1000.0))),..Default::default()}, |ui| {
                if home {Operator::fast_home(ui,&overview,None,true,true,
                    (&mut Page::Home,&mut crate::library::Library::default(),&mut RouteFocus::default(),&mut false,&mut chosen));}
                else {crate::library::Library::default().show(ui,&overview.current,true,&mut chosen,|_,_|{});}
            });
            let mut labels = vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            output.textures_delta.clear();
            assert!(labels.iter().any(|(label,_)|label == "Checking this plug-in"));
            assert!(labels.iter().any(|(label,_)|label == "Available in your DAW"));
            assert!(!labels.iter().any(|(label,_)|label == "DAW connection needs attention"));
            assert!(chosen.is_none());
        }
    }

    #[test]
    fn installation_form_requires_an_explicit_daw_and_preserves_authoritative_refusals() {
        for size in [egui::vec2(560.0,360.0),egui::vec2(960.0,720.0)] {
            for (daw,offer,pending,enabled) in [(None,Ok(None),false,false),
                (Some(DawChoice::Other),Ok(None),false,true),
                (Some(DawChoice::Reaper),Ok(None),true,false),
                (Some(DawChoice::Bitwig),Ok(Some("This shared setup is in use.".into())),false,false),
                (Some(DawChoice::Bitwig),Err("Current manager status is unavailable."),false,false)] {
                let ctx=result_form_context();
                let mut choice=daw;
                for frame in 0..4 {
                    let mut output=ctx.run_ui(egui::RawInput {screen_rect:Some(
                        egui::Rect::from_min_size(egui::Pos2::ZERO,size)),..Default::default()},|_ui| {
                        let (submit,cancel)=show_software_install_form(&ctx,&mut choice,&offer,pending,false).unwrap();
                        if frame == 3 {
                            assert_eq!(submit.enabled(),enabled);
                            assert!(cancel.enabled());
                            assert!(ctx.content_rect().contains_rect(submit.rect));
                            assert!(ctx.content_rect().contains_rect(cancel.rect));
                        }
                    });
                    output.textures_delta.clear();
                }
                assert_eq!(choice,daw);
            }
        }
    }

    #[test]
    fn automatic_progress_uses_only_the_exact_operation_checkpoint() {
        let mut feedback=RequestFeedback::captured(Action::SoftwareInstall {
            installer:"ab".repeat(32),daw:DawChoice::Bitwig});
        feedback.operation=Some("exact-operation".into());
        let mut observation=serde_json::json!({"operation":"unrelated-operation","state":"running",
            "journey":automatic_journey(InstallationPhase::Discovering)});
        feedback.observe(&observation);
        assert!(feedback.journey.is_none());
        observation["operation"]="exact-operation".into();
        feedback.observe(&observation);
        assert_eq!(feedback.text,"Finding installed plug-ins");
        assert_eq!(feedback.journey.as_ref().unwrap().phase,InstallationPhase::Discovering);
        observation["journey"]=serde_json::to_value(automatic_journey(InstallationPhase::PreparingPlugins)).unwrap();
        feedback.observe(&observation);
        assert_eq!(feedback.text,"Connecting plug-ins to your DAW");
        assert!(!feedback.terminal);
        observation["state"]="completed".into();
        observation["journey"]=serde_json::to_value(automatic_journey(InstallationPhase::NeedsAttention)).unwrap();
        feedback.observe(&observation);
        feedback.refreshed();
        assert!(feedback.terminal);
        assert_eq!(feedback.text,
            "Installation or DAW connection needs attention. Its results and continuation are shown below.");
    }

    #[test]
    fn available_plugin_guidance_follows_its_exact_chosen_daw() {
        for daw in [DawChoice::Bitwig,DawChoice::Reaper,DawChoice::Other] {
            let mut snapshot=overview_fixture().current;
            snapshot.products.truncate(1);
            snapshot.environments.clear();snapshot.vendor_applications.clear();
            let product=&mut snapshot.products[0];
            product.availability=Some(NativeAvailability {available:true,reason:None});
            product.compatibility=None;product.actions.clear();
            let exact=ProductKey::from(&*product);
            let found=DiscoveredProduct {environment:product.environment.clone(),
                module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone(),name:product.name.clone()};
            let mut journey=automatic_journey(InstallationPhase::Available);journey.daw=daw;
            let mut setup=automatic_setup(journey,None);setup.discovered=vec![found.clone()];
            let mut unrelated=setup.clone();unrelated.installer="unrelated-installer".into();
            unrelated.discovered[0].environment="different-environment".into();
            unrelated.journey.as_mut().unwrap().daw=DawChoice::Reaper;
            snapshot.installer_setups=vec![setup,unrelated];
            let ctx=result_form_context();
            let mut library=crate::library::Library::default();library.focus_product(exact);
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui|
                library.show(ui,&snapshot,false,&mut None,|_,_|{}));
            let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            assert!(labels.iter().any(|(label,_)|label==presentation::daw_guidance(Some(daw))));
            if daw!=DawChoice::Reaper {
                assert!(!labels.iter().any(|(label,_)|label.starts_with("In REAPER") || label.starts_with("REAPER:")));
            }
            assert!(!labels.iter().any(|(label,_)|label=="Not yet tested by us"),
                "Absent release qualification is not evidence that the plug-in is untested");
            output.textures_delta.clear();
        }
    }

    #[test]
    fn unambiguous_explicit_daw_choice_is_remembered_for_the_next_installation() {
        let mut operator=state_fixture();
        let mut overview:CurrentOverview=overview_fixture().into();
        overview.current.installer_setups=vec![automatic_setup(
            automatic_journey(InstallationPhase::Available),None)];
        operator.apply_current_overview(overview.clone());
        assert_eq!(operator.software_install_daw,Some(DawChoice::Bitwig));
        operator.software_install_daw=Some(DawChoice::Other);
        operator.apply_current_overview(overview);
        assert_eq!(operator.software_install_daw,Some(DawChoice::Other));
    }

    #[test]
    fn vendor_install_is_visible_and_revalidates_the_exact_offer_with_the_chosen_daw() {
        let application = "exact-source-owned-hub".to_owned();
        let action = Action::VendorSoftwareInstall {application:application.clone(),daw:DawChoice::Other};
        let mut overview:CurrentOverview = overview_fixture().into();
        overview.current.installer_setups.clear();
        let affected = overview.current.products[0].name.clone();
        let environment = overview.current.products[0].environment.clone();
        overview.current.products.truncate(1);
        overview.current.vendor_applications = vec![VendorApplication {id:application.clone(),
            name:"Source-owned vendor hub".into(),version:"1.0.0".into(),state:"registered".into(),
            actions:vec![AvailableAction {label:"Install plug-ins through Source-owned vendor hub".into(),
                action:action.clone(),disabled_reason:None}],details:serde_json::json!({"environment":environment})}];
        let ctx=result_form_context();let mut chosen=None;
        let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
            Operator::fast_setup(ui,&overview,true,false,(&mut false,&mut chosen,&mut false,
                &mut Page::Setup,&mut crate::library::Library::default(),&mut RouteFocus::default(),
                &mut Some(DawChoice::Bitwig)));
        });
        let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
        output.textures_delta.clear();
        assert!(labels.iter().any(|(label,_)|label=="Source-owned vendor hub"));
        assert!(labels.iter().any(|(label,_)|label=="Install plug-ins through Source-owned vendor hub"));
        let consequence = format!("{affected} will be unavailable during this work and checked again after you finish here.");
        let consequence_y = labels.iter().find(|(label,_)|label==&consequence).unwrap().1.y;
        let action_y = labels.iter().find(|(label,_)|label=="Install plug-ins through Source-owned vendor hub").unwrap().1.y;
        assert!(consequence_y < action_y);
        overview.current.products[0].disposition = "ready".into();
        overview.current.products[0].availability = Some(NativeAvailability {available:true,reason:None});
        overview.current.products[0].actions.clear();
        overview.current.products[0].compatibility = None;
        let ctx = result_form_context();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            crate::library::Library::default().show(ui, &overview.current, false,
                &mut chosen, Operator::product_details);
        });
        let mut labels = vec![];
        for shape in &output.shapes { text_centers(&shape.shape, &mut labels); }
        output.textures_delta.clear();
        let consequence_y = labels.iter().find(|(label,_)|label==&consequence).unwrap().1.y;
        let action_y = labels.iter().find(|(label,_)|label=="Install plug-ins through Source-owned vendor hub").unwrap().1.y;
        assert!(consequence_y < action_y);
        for daw in [DawChoice::Bitwig,DawChoice::Reaper,DawChoice::Other] {
            let selected=installation_with_daw(&action,daw).unwrap();
            assert_eq!(selected,Action::VendorSoftwareInstall {application:application.clone(),daw});
            assert_eq!(installation_form_offer(Some(&overview.current),&selected),Ok(None));
        }
        overview.current.vendor_applications[0].actions[0].disabled_reason=Some("The installation is still running.".into());
        assert_eq!(installation_form_offer(Some(&overview.current),&action),
            Ok(Some("The installation is still running.".into())));
        overview.current.vendor_applications[0].actions.clear();
        assert!(installation_form_offer(Some(&overview.current),&action).is_err());
        assert!(installation_form_offer(None,&action).is_err());
    }

    #[test]
    fn renderer_installation_daw_draft_preserves_exact_application_and_rendering_policy() {
        let mut overview:CurrentOverview=overview_fixture().into();
        let application="ac".repeat(32);
        let offered=Action::RendererSoftwareInstall {application:application.clone(),
            policy:RendererPolicy::SoftwareRendering,daw:DawChoice::Other};
        overview.current.vendor_applications=vec![VendorApplication {id:"renderer".into(),name:"Installed app".into(),
            version:"1".into(),state:"closed".into(),details:serde_json::Value::Null,actions:vec![AvailableAction {
                label:"Open".into(),action:offered.clone(),disabled_reason:None}]}];
        let selected=installation_with_daw(&offered,DawChoice::Bitwig).unwrap();
        assert_eq!(selected,Action::RendererSoftwareInstall {application:application.clone(),
            policy:RendererPolicy::SoftwareRendering,daw:DawChoice::Bitwig});
        assert_eq!(installation_form_offer(Some(&overview.current),&selected),Ok(None));
        let foreign_policy=Action::RendererSoftwareInstall {application,
            policy:RendererPolicy::Inherited,daw:DawChoice::Bitwig};
        assert!(installation_form_offer(Some(&overview.current),&foreign_policy).is_err());
    }
    #[test]
    fn returning_vendor_session_with_no_new_plugins_shows_completion_without_installation_failure() {
        let mut overview:CurrentOverview=overview_fixture().into();
        let mut journey=automatic_journey(InstallationPhase::Available);journey.plugins.clear();
        overview.current.installer_setups=vec![automatic_setup(journey.clone(),None)];
        overview.current.installer_setups[0].status="Application session finished. No plug-ins were added.".into();
        overview.current.vendor_applications=vec![VendorApplication {id:"same-hub".into(),
            name:"Existing vendor hub".into(),version:"1.0.0".into(),state:"registered".into(),
            actions:vec![],details:serde_json::json!({"journey":journey})}];
        for product in &mut overview.current.products {
            product.availability=Some(NativeAvailability {available:true,reason:None});
        }
        let ctx=result_form_context();let mut chosen=None;
        let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
            Operator::fast_setup(ui,&overview,true,false,(&mut false,&mut chosen,&mut false,
                &mut Page::Setup,&mut crate::library::Library::default(),&mut RouteFocus::default(),
                &mut Some(DawChoice::Bitwig)));
        });
        let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
        output.textures_delta.clear();
        assert!(labels.iter().any(|(label,_)|label=="Application session complete"));
        assert!(labels.iter().any(|(label,_)|label=="Application session finished. No plug-ins were added."));
        assert!(labels.iter().any(|(label,_)|label=="No plug-ins were added during this visit."));
        assert!(!labels.iter().any(|(label,_)|label=="DAW connection complete"));
        assert!(labels.iter().any(|(label,_)|label=="Your DAW: Bitwig"));
        assert!(!labels.iter().any(|(label,_)|label.contains("needs attention") || label.contains("Report")));
        assert!(chosen.is_none());
    }

    #[test]
    fn retired_vendor_failure_keeps_its_result_visible_and_offers_a_new_visit() {
        let mut overview: CurrentOverview = overview_fixture().into();
        overview.current.installer_setups.clear();
        let mut journey = automatic_journey(InstallationPhase::NeedsAttention);
        journey.plugins.clear();
        journey.failure = Some(InstallationFailure {stage:InstallationFailureStage::VendorInstallation,
            reason:"The vendor application did not finish successfully.".into(),
            code:"vendor_cancelled".into(),retryable:false});
        let action = Action::VendorSoftwareInstall {application:"same-retired-hub".into(),daw:DawChoice::Other};
        overview.current.vendor_applications = vec![VendorApplication {id:"same-retired-hub".into(),
            name:"Existing vendor hub".into(),version:"1.0.0".into(),state:"registered".into(),
            actions:vec![AvailableAction {label:"Try installation again through Existing vendor hub".into(),
                action:action.clone(),disabled_reason:None}],details:serde_json::json!({
                    "environment":overview.current.products[0].environment,"journey":journey})}];
        let ctx=result_form_context();let mut chosen=None;
        let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
            Operator::fast_setup(ui,&overview,true,false,(&mut false,&mut chosen,&mut false,
                &mut Page::Setup,&mut crate::library::Library::default(),&mut RouteFocus::default(),
                &mut Some(DawChoice::Bitwig)));
        });
        let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
        output.textures_delta.clear();
        assert!(labels.iter().any(|(label,_)|label=="Installation or DAW connection needs attention"));
        assert!(labels.iter().any(|(label,_)|label=="Vendor installation: The vendor application did not finish successfully."));
        assert!(labels.iter().any(|(label,_)|label=="Try installation again through Existing vendor hub"));
        assert!(!labels.iter().any(|(label,_)|label.contains("Continue installation") || label=="DAW connection complete"));
        let selected=installation_with_daw(&action,DawChoice::Bitwig).unwrap();
        assert_eq!(installation_form_offer(Some(&overview.current),&selected),Ok(None));
        assert!(chosen.is_none());
    }

    #[test]
    fn unchanged_sibling_revalidation_progress_does_not_mask_real_check_failure() {
        for case in ["checking","other-operation","failed"] {
            let mut snapshot=overview_fixture().current;snapshot.products.truncate(1);
            let product=&mut snapshot.products[0];product.disposition="experimental".into();
            product.availability=Some(NativeAvailability {available:false,reason:Some(
                if case=="failed" {"The required companion could not be loaded."}
                else {"The installation changed; fresh checks are required."}.into())});
            product.details=serde_json::json!({"refusal":"Previous environment authority is unavailable",
                "automatic_revalidation":{"operation":if case=="other-operation" {"unrelated"} else {"exact-check"},
                    "predecessor":{"id":"same-selected-predecessor","sha256":"ab".repeat(32)}}});
            product.actions.clear();product.compatibility=None;
            let mut journey=automatic_journey(InstallationPhase::PreparingPlugins);
            journey.plugins=vec![InstallationPlugin {selection:"same-selection".into(),
                class_id:product.class_id.clone(),name:product.name.clone(),
                stage:if case=="failed" {InstallationPluginStage::NeedsAttention} else {InstallationPluginStage::Inspecting},
                failure:(case=="failed").then(||InstallationFailure {stage:InstallationFailureStage::Inspection,
                    reason:"The required companion could not be loaded.".into(),code:"dependency_failed".into(),retryable:true})}];
            let mut setup=automatic_setup(journey.clone(),None);setup.environment=Some(product.environment.clone());
            setup.discovered=vec![DiscoveredProduct {environment:product.environment.clone(),
                module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone(),name:product.name.clone()}];
            snapshot.operation=Some(serde_json::json!({"operation":"exact-check","state":"running",
                "onboarding":product.environment,"journey":journey}));snapshot.installer_setups=vec![setup];
            let ctx=result_form_context();let mut chosen=None;
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
                crate::library::Library::default().show(ui,&snapshot,false,&mut chosen,Operator::product_details);
            });
            let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            output.textures_delta.clear();
            assert_eq!(labels.iter().any(|(label,_)|label=="Checking this plug-in"),case=="checking","{case}");
            assert_eq!(labels.iter().any(|(label,_)|label=="DAW connection needs attention"),case!="checking","{case}");
            if case=="failed" {assert!(labels.iter().any(|(label,_)|label.contains("required companion could not be loaded")));}
            assert!(!snapshot.products[0].availability.as_ref().unwrap().available);
        }
    }

    #[test]
    fn install_form_revalidates_the_exact_software_offer() {
        let mut snapshot=overview_fixture().current;
        let installer="ab".repeat(32);
        snapshot.installer_setups=vec![automatic_setup(automatic_journey(InstallationPhase::Available),
            Some(Action::SoftwareInstall {installer:installer.clone(),daw:DawChoice::Other}))];
        assert_eq!(software_install_offer(Some(&snapshot),&installer,None),Ok(None));
        snapshot.installer_setups[0].primary.as_mut().unwrap().disabled_reason=
            Some("The shared installation is still running.".into());
        assert_eq!(software_install_offer(Some(&snapshot),&installer,None),
            Ok(Some("The shared installation is still running.".into())));
        assert!(software_install_offer(Some(&snapshot),"different-installer",None).is_err());
        assert!(software_install_offer(None,&installer,None).is_err());
    }

    #[test]
    fn home_prioritizes_reported_problem_recovery_without_erasing_current_availability() {
        let mut overview:CurrentOverview=overview_fixture().into();
        overview.current.products.truncate(1);
        let product=&mut overview.current.products[0];
        product.availability=Some(NativeAvailability {available:true,reason:None});
        product.compatibility=Some(CompatibilityWorkflow {phase:CompatibilityPhase::AwaitingRetirement,
            summary:"The failed setup is still selected. Close the plug-in, then finish recovery.".into(),
            established:vec![],remaining:vec![],current_inspection:None,current_candidate:None,
            primary:Some(AvailableAction {label:"Finish test result".into(),
                action:Action::CompatibilityFinishResult {operation:"exact-recovery".into()},
                disabled_reason:Some("Close this plug-in first.".into())}),alternatives:vec![]});
        let exact=ProductKey::from(&*product);
        let ctx=result_form_context();
        let mut library=crate::library::Library::default();
        let mut page=Page::Home;
        let mut point=egui::Pos2::ZERO;
        for frame in 0..5 {
            let events=if frame>=3 {vec![egui::Event::PointerMoved(point),
                egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                    pressed:frame==3,modifiers:egui::Modifiers::NONE}]} else {vec![]};
            let mut output=ctx.run_ui(egui::RawInput {events,..Default::default()},|ui|
                Operator::fast_home(ui,&overview,None,true,false,
                    (&mut page,&mut library,&mut RouteFocus::default(),&mut false,&mut None)));
            if frame==2 {
                let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
                assert!(labels.iter().any(|(label,_)|label=="Available in your DAW"));
                assert!(labels.iter().any(|(label,_)|label.starts_with("Next: recover")));
                assert!(!labels.iter().any(|(label,_)|label.starts_with("Plug-ins available in ")));
                point=labels.iter().find(|(label,_)|label=="Open plug-in recovery").unwrap().1;
            }
            output.textures_delta.clear();
        }
        assert_eq!(page,Page::Plugins);
        assert_eq!(library.focused_product(),Some(&exact));
    }

    #[test]
    fn ordinary_configuration_label_preserves_deferred_execution_verification() {
        let mut product = crate::model::ReadinessProduct {
            name:"Exact product".into(),class_id:"01".repeat(16),module_sha256:"02".repeat(32),
            profile:None,status:ReadinessOutcome::Ready,installation_health:InstallationHealth::Healthy,
            support_qualification:SupportQualification::NotYetQualified,reason:"Records agree".into(),
            failure_code:None,facts:vec![ReadinessFact {name:"Validation scope".into(),
                value:Some("control_records".into()),source:"Canonical product records".into(),
                observed_at:1,certainty:FactCertainty::Observed}],
        };
        assert_eq!(installation_label(&product), "Current configuration: records and publication agree");
        product.installation_health = InstallationHealth::Unknown;
        assert_eq!(installation_label(&product), "Current configuration: records incomplete");
        product.installation_health = InstallationHealth::ActionRequired;
        assert_eq!(installation_label(&product), "Current configuration: needs attention");
        product.facts.clear();
        product.installation_health = InstallationHealth::Healthy;
        assert_eq!(installation_label(&product), "Current installation: healthy");
    }
    #[test]
    fn recovery_does_not_claim_readiness_or_erase_prior_failure() {
        let mut value = serde_json::json!({"dependency":{"recovery":{"authority":"qualified_bundle_payload_and_retired_installation"},"ready_tested":false}});
        value["error"] = serde_json::json!("dependency_running_not_ready");
        let lines = dependency_retirement_lines(&value);
        assert!(lines
            .iter()
            .any(|s| s.contains("Native Access was not opened")));
        assert!(lines
            .iter()
            .any(|s| s.contains("earlier installer failure remains")));
        assert!(lines
            .iter()
            .any(|s| s == "Fresh service readiness: not established."));
        value["dependency"]["ready_tested"] = serde_json::json!(true);
        assert!(dependency_retirement_lines(&value)
            .iter()
            .any(|s| s == "Fresh service readiness: verified."));
    }
    #[test]
    fn dependency_installer_exit_survives_runner_timeout_and_cleanup() {
        let mut value = serde_json::json!({"error":"dependency_runner_retirement_timeout","dependency":{
            "process_cleanup_confirmed":true,"stages":[{"action":"install","exit":null,
            "installer_result":{"installer_exit":100,"authority":"exact_windows_child_handle_exit"}}]}});
        let lines = dependency_retirement_lines(&value);
        assert!(lines.iter().any(|l| l.contains("installer exit: 100")));
        assert!(lines
            .iter()
            .any(|l| l.contains("runner retirement was not observed")));
        value["dependency"]["stages"][0]["installer_result"]["authority"] =
            serde_json::json!("legacy_adapter_wait_or_exit_ambiguous");
        assert!(dependency_retirement_lines(&value)
            .iter()
            .any(|l| l == "Dependency installer exit: unavailable."));
    }
    #[test]
    fn process_cleanup_never_projects_clean_service_retirement() {
        let value = serde_json::json!({"dependency":{"service_retirement_confirmed":false,"process_cleanup_confirmed":true,"forced_cleanup_used":true}});
        let lines = dependency_retirement_lines(&value);
        assert!(lines[0].ends_with("not confirmed"));
        assert!(lines[1].ends_with("confirmed"));
        assert!(lines[2].contains("does not confirm"));
        let recovered = serde_json::json!({"dependency":{"service_retirement_confirmed":true},"dependency_retirement":{"service_retirement_confirmed":false,"process_cleanup_confirmed":true}});
        assert!(dependency_retirement_lines(&recovered)[0].ends_with("not confirmed"));
    }
    #[test]
    fn native_access_cleanup_dispositions_do_not_conflate_owned_cleanup_with_graceful_stop() {
        let graceful = serde_json::json!({"dependency":{"dependency_cleanup_disposition":"graceful_service_retirement",
            "service_retirement_confirmed":true,"process_cleanup_confirmed":true,"forced_cleanup_used":false}});
        let exact = serde_json::json!({"dependency":{"dependency_cleanup_disposition":"exact_owned_session_cleanup",
            "service_retirement_confirmed":false,"process_cleanup_confirmed":true,"forced_cleanup_used":true}});
        let failed = serde_json::json!({"dependency":{"dependency_cleanup_disposition":"cleanup_unconfirmed",
            "service_retirement_confirmed":false,"process_cleanup_confirmed":false,"forced_cleanup_used":true}});
        assert!(dependency_retirement_lines(&graceful)
            .iter()
            .any(|s| s == "Native Access closed with graceful dependency retirement."));
        let exact_text = dependency_retirement_lines(&exact).join(" ");
        assert!(exact_text.contains("exact-owned compatibility cleanup"));
        assert!(exact_text.contains("graceful service shutdown was not confirmed"));
        assert!(dependency_retirement_lines(&failed)
            .iter()
            .any(|s| s == "Native Access or dependency cleanup is not confirmed."));
        let reverified = serde_json::json!({"dependency":{"dependency_cleanup_disposition":"reverified_after_unconfirmed_cleanup",
            "service_retirement_confirmed":false,"process_cleanup_confirmed":true}});
        let reverified_text = dependency_retirement_lines(&reverified).join(" ");
        assert!(reverified_text.contains("did not confirm it closed; the bridge later confirmed nothing from it was still running"));
        assert!(reverified_text.contains("Dependency service stop: not confirmed"));
    }
    #[test]
    fn nad2_stop_characterizations_are_bounded_and_never_claim_acceptance() {
        let classes = [
            "NAD2_STOP_CONFIRMED",
            "NAD2_STOP_SUBMITTED_PROGRESSING",
            "NAD2_STOP_SUBMITTED_NO_TRANSITION",
            "NAD2_STOP_NOT_SUBMITTED",
            "NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS",
            "NAD2_STOP_OBSERVATION_UNAVAILABLE",
        ];
        for class in classes {
            let submitted = class != "NAD2_STOP_NOT_SUBMITTED";
            let value = serde_json::json!({"dependency":{"service_retirement_confirmed":class=="NAD2_STOP_CONFIRMED",
                "process_cleanup_confirmed":false,"forced_cleanup_used":class!="NAD2_STOP_CONFIRMED","stop_observation":{
                    "schema":3,"classification":class,"initial_state":4,"final_state":4,"initial_controls_accepted":1,
                    "final_controls_accepted":1,"initial_checkpoint":0,"checkpoint":0,"initial_wait_hint_ms":0,
                    "wait_hint_ms":0,"control_count":1,"control_submitted":submitted,
                    "control_error":if submitted {0}else{5},"control_elapsed_ms":3,
                    "control_return":{"state":4,"controls_accepted":1,"checkpoint":0,"wait_hint_ms":0},
                    "process_wait_class":"timeout","process_wait":258,"initial_listener_mask":3,"listener_mask":3,
                    "transition_count_total":1,"transition_count_retained":1,"transition_count_dropped":0}}});
            let lines = dependency_retirement_lines(&value);
            let text = lines.join(" ");
            assert!(text.contains(class));
            if submitted {
                assert!(text.contains("ControlService result: submitted"));
            } else {
                assert!(text.contains("The ControlService request was not submitted; error 5."));
            }
            assert!(!text.contains("request accepted") && !text.contains("service accepted"));
            if class != "NAD2_STOP_CONFIRMED" {
                assert!(lines[0].ends_with("not confirmed"));
            }
        }
    }
    #[test]
    fn pending_stop_without_control_call_reports_observation_not_error_zero() {
        let value = serde_json::json!({"dependency":{"service_retirement_confirmed":false,
            "process_cleanup_confirmed":false,"forced_cleanup_used":false,"stop_observation":{
                "schema":3,"classification":"NAD2_STOP_SUBMITTED_PROGRESSING","initial_state":3,
                "final_state":3,"initial_controls_accepted":1,"final_controls_accepted":1,
                "initial_checkpoint":1,"checkpoint":2,"initial_wait_hint_ms":1000,"wait_hint_ms":900,
                "control_count":0,"control_submitted":false,"control_error":0,"control_elapsed_ms":0,
                "process_wait_class":"timeout","process_wait":258,"initial_listener_mask":3,"listener_mask":3,
                "transition_count_total":2,"transition_count_retained":2,"transition_count_dropped":0}}});
        let text = dependency_retirement_lines(&value).join(" ");
        assert!(text
            .contains("No ControlService request was issued; the service was already stopping."));
        assert!(!text.contains("error 0"));
    }
    #[test]
    fn stopped_service_without_control_call_reports_no_request() {
        let value = serde_json::json!({"dependency":{"service_retirement_confirmed":false,
            "process_cleanup_confirmed":false,"forced_cleanup_used":false,"stop_observation":{
                "schema":3,"classification":"NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS","initial_state":1,
                "final_state":1,"initial_controls_accepted":0,"final_controls_accepted":0,
                "initial_checkpoint":0,"checkpoint":0,"initial_wait_hint_ms":0,"wait_hint_ms":0,
                "control_count":0,"control_submitted":false,"control_error":0,"control_elapsed_ms":0,
                "process_wait_class":"timeout","process_wait":258,"initial_listener_mask":3,"listener_mask":3,
                "transition_count_total":1,"transition_count_retained":1,"transition_count_dropped":0}}});
        let text = dependency_retirement_lines(&value).join(" ");
        assert!(
            text.contains("No ControlService request was issued; the service was already stopped.")
        );
        assert!(!text.contains("error 0"));
    }
    #[test]
    fn unavailable_initial_observation_without_control_call_reports_no_request() {
        let value = serde_json::json!({"dependency":{"service_retirement_confirmed":false,
            "process_cleanup_confirmed":false,"forced_cleanup_used":false,"stop_observation":{
                "schema":3,"classification":"NAD2_STOP_OBSERVATION_UNAVAILABLE","initial_state":0,
                "final_state":0,"initial_controls_accepted":0,"final_controls_accepted":0,
                "initial_checkpoint":0,"checkpoint":0,"initial_wait_hint_ms":0,"wait_hint_ms":0,
                "control_count":0,"control_submitted":false,"control_error":0,"control_elapsed_ms":0,
                "process_wait_class":"unavailable","process_wait":4294967295_u64,
                "initial_listener_mask":4,"listener_mask":4,"transition_count_total":1,
                "transition_count_retained":1,"transition_count_dropped":0}}});
        let text = dependency_retirement_lines(&value).join(" ");
        assert!(text.contains(
            "No ControlService request was issued; the service state could not be evaluated."
        ));
        assert!(!text.contains("error 0"));
    }
    #[test]
    fn refused_control_call_reports_retained_nonzero_error() {
        let value = serde_json::json!({"dependency":{"service_retirement_confirmed":false,
            "process_cleanup_confirmed":false,"forced_cleanup_used":false,"stop_observation":{
                "schema":3,"classification":"NAD2_STOP_NOT_SUBMITTED","initial_state":4,
                "final_state":4,"initial_controls_accepted":1,"final_controls_accepted":1,
                "initial_checkpoint":0,"checkpoint":0,"initial_wait_hint_ms":0,"wait_hint_ms":0,
                "control_count":1,"control_submitted":false,"control_error":5,"control_elapsed_ms":3,
                "process_wait_class":"timeout","process_wait":258,"initial_listener_mask":3,"listener_mask":3,
                "transition_count_total":1,"transition_count_retained":1,"transition_count_dropped":0}}});
        let text = dependency_retirement_lines(&value).join(" ");
        assert!(text.contains("The ControlService request was not submitted; error 5."));
        assert!(!text.contains("No ControlService request was issued"));
    }
    #[test]
    fn installer_root_and_presence_do_not_claim_close_from_helper_success() {
        let mut v = serde_json::json!({"operation":"exact","cleanup_confirmed":true,"transaction":{
            "schema":1,"operation":"exact","outcome":"outer_nonzero_stage_unknown","durable_installation":"partial_installation",
            "launch_binding":{"schema":1,"operation":"exact","status":"bound"},
            "presence_close":{"schema":1,"observation_count":3,"operation_classes":["presence_query","close_request","presence_query"]}}});
        let text = super::installer_lines(&v).join(" ");
        assert!(text.contains("exact operation and launch generation"));
        assert!(
            text.contains("Helper success does not prove")
                && text.contains("results remain unavailable")
        );
        assert!(
            text.contains("PowerShell/CIM")
                && text.contains("matched object and actual close outcome are unavailable")
        );
        v["transaction"]["launch_binding"]["operation"] = "unrelated".into();
        assert!(!super::installer_lines(&v)
            .join(" ")
            .contains("exact operation and launch generation"));
        v["transaction"]["launch_binding"]["operation"] = "exact".into();
        v["transaction"]["launch_binding"]["status"] = "unavailable".into();
        assert!(super::installer_lines(&v)
            .join(" ")
            .contains("Child attribution may be incomplete"));
    }
    #[test]
    fn first_failure_presents_exact_stage_role_relationship_domain_without_guessing() {
        for role in [Some("service_dependency"), None] {
            for (domain, status) in [("wine_self_exit_observation", 3010), ("linux_wait", 2)] {
                let v = serde_json::json!({"operation":"exact","cleanup_confirmed":true,
                    "transaction":{"schema":1,"operation":"exact","outcome":"cancelled",
                        "outer_launcher_exit":-15,"durable_installation":"partial_installation",
                        "first_failure":{"phase":"target_runner","role":role,
                            "relationship":"descendant","domain":domain,"status":status}}});
                let lines = super::installer_lines(&v);
                let failure = lines
                    .iter()
                    .find(|l| l.starts_with("First retained"))
                    .unwrap();
                assert_eq!(failure, &format!("First retained process result: phase target_runner · role {} · relationship descendant · domain {domain} · status {status} · cause unestablished", role.unwrap_or("unknown")));
                assert!(lines.iter().any(|l| l.contains("Outer launcher exit: -15")));
                assert!(lines.iter().any(|l| l.contains("cleanup confirmed")));
            }
        }
    }
    #[test]
    fn installer_partial_nonzero_cancellation_and_cleanup_stay_separate() {
        let mut v = serde_json::json!({"operation":"exact","cleanup_confirmed":true,"startup":{"first_problem":{"code":"runtime_assertion_observed"}},"transaction":{"schema":1,"operation":"exact","outcome":"cancelled","outer_launcher_exit":-15,"durable_installation":"partial_installation","first_failure":{"domain":"linux_wait","status":37}}});
        let lines = super::installer_lines(&v).join(" ");
        assert!(
            lines.contains("runtime assertion observed")
                && lines.contains("cancelled")
                && lines.contains("status 37")
                && lines.contains("partial installation")
                && lines.contains("cleanup confirmed")
        );
        v["transaction"]["outcome"] = "in_progress".into();
        let ongoing = super::installer_lines(&v).join(" ");
        assert!(ongoing.contains("Exact Focus and Stop"));
        assert!(!ongoing.contains("Further work is blocked"));
        v["transaction"]["operation"] = "other".into();
        assert!(super::installer_lines(&v).is_empty());
        let legacy = serde_json::json!({"error":"installer_launcher_failed"});
        assert!(super::installer_lines(&legacy)[0].contains("failing later stage"));
    }

    use super::*;
    #[test]
    fn installer_rename_local_check_matches_manager_utf8_byte_bound() {
        assert!(plausible_installer_label("Lunacy Audio"));
        assert!(plausible_installer_label(&"é".repeat(48)));
        assert!(!plausible_installer_label(&"é".repeat(49)));
        for invalid in [" leading", "trailing ", ".", "..", "path/name", "a:b", "a\\b", "a\u{202e}b"] {
            assert!(!plausible_installer_label(invalid));
        }
    }
    fn state_fixture() -> Operator {
        let (sender, receiver) = mpsc::channel();
        Operator {
            snapshot: None,
            overview: None,
            readiness: None,
            product_detail: None,
            product_attempted: None,
            overview_fresh: false,
            overview_renewing: false,
            details_attempted: false,
            sender,
            receiver,
            window_wake: None,
            pending: false,
            background_poll: false,
            action_inflight: false,
            frontend_superseded: false,
            origin: None,
            queued_import: false,
            explicit_refresh_queued: false,
            queued_action: None,
            feedback: None,
            last_poll: Instant::now(),
            last_overview: Instant::now(),
            operation_live: false,
            prompt_pulse: false,
            last_pulse: None,
            cleanup_failure_observed: false,
            readback_failures: 0,
            retry_after: None,
            message: String::new(),
            library: crate::library::Library::default(),
            refresh_after: false,
            product_form: None,
            workspace_select_form: None,
            installer_rename_form: None,
            removal_form: None,
            software_install_form: None,
            software_install_daw: None,
            pending_import_route: None,
            page: Page::Home,
            focus: RouteFocus::default(),
            preview: false,
            updates: crate::updates::Updates::disabled(),
        }
    }
    #[test]
    fn superseded_frontend_pauses_without_losing_drafts_or_operation_feedback() {
        let mut state = state_fixture();
        state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        let draft = Action::InstallerRename { installer:"ab".repeat(32), label:"Unfinished name".into() };
        state.installer_rename_form = Some(draft.clone());
        let request = Request { schema:crate::model::OPERATOR_SCHEMA,
            state_token:"previous".into(), action:draft.clone() };
        state.queued_action = Some(request.clone());
        let mut feedback = RequestFeedback::captured(draft.clone());
        feedback.operation = Some("accepted-operation".into());
        state.feedback = Some(feedback);
        state.pending = true;
        state.origin = Some(RequestOrigin::BackgroundActivity);
        state.handle_reply(Reply::FrontendSuperseded);
        assert!(state.frontend_superseded && !state.pending && !state.overview_fresh);
        assert!(state.readiness.is_none() && state.controls_pending());
        assert_eq!(state.installer_rename_form.as_ref(), Some(&draft));
        assert_eq!(state.feedback.as_ref().unwrap().operation.as_deref(), Some("accepted-operation"));
        assert!(state.next_action().is_none());
        assert_eq!(state.queued_action.as_ref().map(|r| &r.action), Some(&request.action));
        state.request(Query::Overview, RequestOrigin::ExplicitRefresh, &egui::Context::default());
        assert!(!state.pending, "paused window must not launch a readback or readiness probe");
        assert!(!state.capture_action(request));
    }

    fn overview_fixture() -> InteractiveOverview {
        let snapshot: Snapshot = serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap();
        let readiness = ReadinessAssessment {
            schema:1,state_token:snapshot.state_token.clone(),observed_at:1,
            overall_status:crate::model::ReadinessOutcome::Unknown,
            system:snapshot.system.clone(),platform:vec![],daw:vec![],audio:vec![],
            graphics:vec![],runtime:vec![],products:vec![],blockers:vec![],ordered_steps:vec![],
            support_export_action:AvailableAction {label:"Export".into(),
                action:Action::SupportExport {},disabled_reason:None},
        };
        InteractiveOverview {schema:1,operator_schema:crate::model::OPERATOR_SCHEMA,
            scope:"current_only".into(),current_generation:"same".into(),current:snapshot,readiness}
    }
    fn result_form_fixture() -> Action {
        Action::CompatibilityResult {
            candidate: "ab".repeat(32),
            expected_current: PublicationIdentity { id: "cd".repeat(16), sha256: "ef".repeat(32) },
            result: TestResultKind::Worked, passed: vec![TestArea::DawLoad],
            failed_area: None, note: "Layout test detail".into(),
        }
    }

    fn result_form_context() -> egui::Context {
        let ctx = egui::Context::default();
        let mut style = (*ctx.global_style()).clone();
        // Match the ordinary native frontend's touch-sized controls.
        style.spacing.interact_size.y = 42.0;
        style.spacing.item_spacing = egui::vec2(12.0, 12.0);
        ctx.set_global_style(style);
        ctx
    }

    #[test]
    fn status_readback_changes_keep_navigation_and_action_targets_in_place() {
        for width in [560.0, 960.0, 1280.0] {
            for ready_on_press in [true, false] {
                for navigation_target in [true, false] {
                    let ctx = result_form_context();
                    let mut state = state_fixture();
                    state.overview = Some(overview_fixture().into());
                    state.page = Page::Setup;
                    let mut point = egui::Pos2::ZERO;
                    let mut initial = None;
                    let mut action_clicked = false;
                    for frame in 0..6 {
                        let ready = if frame < 4 { ready_on_press } else { !ready_on_press };
                        state.overview_fresh = ready;
                        let events = if frame == 3 || frame == 4 {
                            vec![egui::Event::PointerMoved(point),
                                egui::Event::PointerButton { pos: point,
                                    button: egui::PointerButton::Primary, pressed: frame == 3,
                                    modifiers: egui::Modifiers::NONE }]
                        } else { vec![] };
                        let mut output = ctx.run_ui(egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO,
                                egui::vec2(width, 800.0))), events, ..Default::default()
                        }, |ui| {
                            Operator::health_bar(ui, state.current_health_system(), state.current_owned_pause(),
                                state.overview_renewing);
                            let bounds = navigation(ui, &mut state.page);
                            let nav = bounds.iter().find(|(p, _, _)| *p == Page::Plugins).unwrap().2;
                            let action = ui.add_enabled(ready,
                                egui::Button::new("Use the selected compatibility runtime")
                                    .min_size(egui::vec2(360.0, 44.0)));
                            if frame >= 2 {
                                let rects = (nav, action.rect);
                                if let Some(initial) = initial {
                                    assert_eq!(rects, initial,
                                        "width={width}, ready_on_press={ready_on_press}, frame={frame}");
                                } else { initial = Some(rects); }
                                if frame == 2 { point = if navigation_target { nav.center() }
                                    else { action.rect.center() }; }
                            }
                            action_clicked |= action.clicked();
                        });
                        output.textures_delta.clear();
                    }
                    if navigation_target {
                        assert_eq!(state.page, Page::Plugins,
                            "navigation lost its click at width={width}, ready_on_press={ready_on_press}");
                    } else {
                        assert!(!action_clicked, "a readback transition cannot authorize a click");
                    }
                }
            }
        }
    }

    #[test]
    fn daw_and_program_choices_survive_renewal_without_retargeting_a_click() {
        for width in [560.0, 960.0, 1280.0] {
            let ctx = result_form_context();
            let mut style = (*ctx.global_style()).clone();
            style.animation_time = 0.0;
            ctx.set_global_style(style);
            let mut overview = overview_fixture();
            overview.current.products.clear();
            overview.current.installer_setups.clear();
            overview.current.environment_installers.clear();
            overview.current.actions.clear();
            let app = |id: &str, name: &str, choosing: bool| VendorApplication {
                id: id.into(), name: name.into(), version: "1.0".into(), state: "closed".into(),
                details: serde_json::json!({"environment":"owned-setup", "choose_application":choosing}),
                actions: vec![AvailableAction { label: format!("Open {name}"), disabled_reason: None,
                    action: Action::VendorSoftwareInstall { application: id.into(), daw: DawChoice::Other } }],
            };
            overview.current.vendor_applications = vec![app("registered", "Managed app", false),
                app("hub", "Source app (hub)", true), app("helper", "Source app (helper)", true)];
            let mut state = state_fixture();
            state.page = Page::Setup;
            state.handle_reply(Reply::Overview(Box::new(overview.clone())));
            state.software_install_daw = None;
            let render = |state: &mut Operator, events: Vec<egui::Event>, chosen: &mut Option<Action>| {
                let pending = state.controls_pending();
                let actions_current = state.current_action_snapshot().is_some();
                let mut output = ctx.run_ui(egui::RawInput {
                    events, screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO,
                        egui::vec2(width, 1200.0))), ..Default::default()
                }, |ui| {
                    Operator::health_bar(ui, state.current_health_system(), state.current_owned_pause(),
                        state.overview_renewing);
                    navigation(ui, &mut state.page);
                    Operator::fast_setup(ui, state.overview.as_ref().unwrap(), actions_current,
                        pending, (&mut false, chosen, &mut false, &mut state.page,
                            &mut state.library, &mut state.focus, &mut state.software_install_daw));
                });
                let mut labels = vec![];
                for shape in &output.shapes { text_centers(&shape.shape, &mut labels); }
                output.textures_delta.clear();
                labels
            };
            let click = |point: egui::Pos2, pressed| vec![egui::Event::PointerMoved(point),
                egui::Event::PointerButton {pos: point, button: egui::PointerButton::Primary,
                    pressed, modifiers: egui::Modifiers::NONE}];
            let point = |labels: &[(String, egui::Pos2)], text: &str| {
                labels.iter().find(|(label, _)| label == text)
                    .unwrap_or_else(|| panic!("Missing {text} at width {width}: {labels:?}")).1
            };
            let expire = |state: &mut Operator| {
                state.last_overview = Instant::now() - Duration::from_secs(61);
                state.expire_overview();
                state.pending = true;
                state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
            };
            let mut chosen = None;
            render(&mut state, vec![], &mut chosen);
            let labels = render(&mut state, vec![], &mut chosen);
            let daw = point(&labels, "Choose your DAW");
            let program = point(&labels, "Choose installed program");
            render(&mut state, click(daw, true), &mut chosen);
            expire(&mut state);
            let labels = render(&mut state, click(daw, false), &mut chosen);
            assert_eq!(point(&labels, "Choose your DAW"), daw, "DAW control moved at width {width}");
            assert_eq!(point(&labels, "Choose installed program"), program,
                "Program control moved at width {width}");
            assert!(labels.iter().any(|(text, _)| text == "Last observed status · refreshing…"));
            assert!(!labels.iter().any(|(text, _)| text == "Bridge service status unavailable"));
            let labels = render(&mut state, vec![], &mut chosen);
            let reaper = point(&labels, "REAPER");
            render(&mut state, click(reaper, true), &mut chosen);
            state.handle_reply(Reply::CurrentOverview(Box::new(overview.clone().into())));
            render(&mut state, click(reaper, false), &mut chosen);
            assert_eq!(state.software_install_daw, Some(DawChoice::Reaper));
            assert!(chosen.is_none());

            let labels = render(&mut state, vec![], &mut chosen);
            let program = point(&labels, "Choose installed program");
            render(&mut state, click(program, true), &mut chosen);
            expire(&mut state);
            let labels = render(&mut state, click(program, false), &mut chosen);
            assert_eq!(point(&labels, "Choose installed program"), program);
            let labels = render(&mut state, vec![], &mut chosen);
            let helper = point(&labels, "Source app (helper)");
            render(&mut state, click(helper, true), &mut chosen);
            render(&mut state, click(helper, false), &mut chosen);
            let labels = render(&mut state, vec![], &mut chosen);
            let open = point(&labels, "Open Source app (helper)");
            render(&mut state, click(open, true), &mut chosen);
            render(&mut state, click(open, false), &mut chosen);
            let action = installation_with_daw(&chosen.take().unwrap(), state.software_install_daw.unwrap()).unwrap();
            assert_eq!(action, Action::VendorSoftwareInstall {
                application: "helper".into(), daw: DawChoice::Reaper });
            assert!(state.capture_chosen_action(action.clone()));
            let original = state.queued_action.as_ref().unwrap().clone();
            assert_eq!(original.state_token, overview.current.state_token);
            assert!(state.next_action().is_none(), "wait for the pending read, without losing the click");

            overview.current.state_token = "renewed-token".into();
            state.handle_reply(Reply::CurrentOverview(Box::new(overview.into())));
            assert_eq!(state.queued_action.as_ref().unwrap().state_token, original.state_token);
            assert_eq!(state.queued_action.as_ref().unwrap().action, action);
            assert!(state.next_action().is_none(), "changed authority must not upgrade the captured token");
            assert!(state.feedback.as_ref().unwrap().text.contains("request was not submitted"));
            assert!(!state.controls_pending());
            let labels = render(&mut state, vec![], &mut chosen);
            assert_eq!(state.software_install_daw, Some(DawChoice::Reaper));
            assert!(!labels.iter().any(|(text, _)| text == "Choose installed program"));
            let open = point(&labels, "Open Source app (helper)");
            render(&mut state, click(open, true), &mut chosen);
            render(&mut state, click(open, false), &mut chosen);
            let action = installation_with_daw(&chosen.unwrap(), state.software_install_daw.unwrap()).unwrap();
            assert_eq!(action, Action::VendorSoftwareInstall {
                application: "helper".into(), daw: DawChoice::Reaper });
            assert!(state.capture_chosen_action(action));
            assert_eq!(state.queued_action.as_ref().unwrap().state_token, "renewed-token");
        }
    }
    #[test]
    fn passive_expiry_retains_health_but_real_changes_remove_it() {
        for cause in ["read_failure", "generation", "cleanup", "owners", "service", "superseded"] {
            let mut state = state_fixture();
            let mut overview = overview_fixture();
            overview.current.system.dsp = 0;
            overview.current.system.keepers = 0;
            overview.current.system.maintenance = 0;
            overview.current.system.pending_transactions = 0;
            overview.current.system.cleanup_unconfirmed = false;
            state.handle_reply(Reply::Overview(Box::new(overview.clone())));
            state.handle_reply(Reply::Pulse(pulse_fixture(false)));
            state.last_overview = Instant::now() - Duration::from_secs(61);
            state.expire_overview();
            assert_eq!(state.current_action_snapshot().unwrap().state_token, overview.current.state_token);
            assert_eq!(state.current_health_system(), Some(&overview.current.system), "{cause}");
            state.handle_reply(Reply::Pulse(pulse_fixture(false)));
            assert_eq!(state.current_health_system(), Some(&overview.current.system));
            assert!(!state.overview_fresh, "a pulse cannot renew the overview");
            assert_eq!(state.current_action_snapshot().unwrap().state_token, overview.current.state_token);

            match cause {
                "read_failure" => state.handle_reply(Reply::Error("Readback failed".into())),
                "superseded" => state.handle_reply(Reply::FrontendSuperseded),
                _ => {
                    let mut pulse = pulse_fixture(false);
                    match cause {
                        "generation" => pulse.current_generation = "changed".into(),
                        "cleanup" => pulse.cleanup_unconfirmed = Some(true),
                        "owners" => pulse.dsp = Some(1),
                        "service" => pulse.service_state = "inactive".into(),
                        _ => unreachable!(),
                    }
                    state.handle_reply(Reply::Pulse(pulse));
                }
            }
            assert!(state.current_health_system().is_none(), "{cause}");
            assert!(state.current_action_snapshot().is_none(), "{cause}");
        }
    }
    #[test]
    fn passive_renewal_keeps_the_home_task_and_readonly_navigation() {
        for width in [560.0, 960.0] {
            let ctx = result_form_context();
            let mut state = state_fixture();
            let mut overview = overview_fixture();
            overview.current.system.dsp = 0;
            overview.current.system.keepers = 0;
            overview.current.system.cleanup_unconfirmed = false;
            overview.current.installer_setups = vec![InstallerSetup {
                installer:"retained-installer".into(), name:"Unfinished setup".into(),
                label_source:"operator_named".into(), byte_size:1024, format:"pe_executable".into(), imported_at:1,
                phase:SetupPhase::DiscoveryComplete, status:"Connect the missing effect.".into(),
                environment:Some("owned-environment".into()), compatibility:None, discovered:vec![],
                primary:Some(AvailableAction {label:"Find and connect installed plug-ins".into(), disabled_reason:None,
                    action:Action::EnvironmentRefresh {environment:"owned-environment".into(), daw:DawChoice::Bitwig}}),
                secondary:vec![], rename:AvailableAction {label:"Rename".into(), disabled_reason:None,
                    action:Action::InstallerRename {installer:"retained-installer".into(), label:String::new()}},
                journey:Some(InstallationJourney {daw:DawChoice::Bitwig, phase:InstallationPhase::NeedsAttention,
                    plugins:vec![], failure:None}), history:vec![],
            }];
            state.handle_reply(Reply::Overview(Box::new(overview)));
            let mut target = egui::Pos2::ZERO;
            let mut before = None;
            let mut chosen = None;
            for frame in 0..5 {
                if frame == 3 {
                    state.last_overview = Instant::now() - Duration::from_secs(61);
                    state.expire_overview();
                    state.pending = true;
                    state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
                }
                let current = state.current_action_snapshot().is_some();
                let pending = state.controls_pending();
                let events = if frame >= 3 {vec![egui::Event::PointerMoved(target),
                    egui::Event::PointerButton {pos:target, button:egui::PointerButton::Primary,
                        pressed:frame == 3, modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output = ctx.run_ui(egui::RawInput {events,
                    screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,1200.0))),
                    ..Default::default()}, |ui| {
                    Operator::health_bar(ui,state.current_health_system(),state.current_owned_pause(),state.overview_renewing);
                    Operator::fast_home(ui,state.overview.as_ref().unwrap(),None,current,pending,
                        (&mut state.page,&mut state.library,&mut state.focus,&mut false,&mut chosen));
                });
                let mut labels = vec![];
                for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
                let positions = |labels:&[(String,egui::Pos2)]| labels.iter()
                    .filter(|(label,_)| label.starts_with("Unfinished setup ·")
                        || label == "Find and connect installed plug-ins" || label == "Open this setup")
                    .cloned().collect::<Vec<_>>();
                if frame == 2 {
                    target = labels.iter().find(|(label,_)|label == "Open this setup").unwrap().1;
                    before = Some(positions(&labels));
                }
                if frame >= 3 {
                    assert_eq!(positions(&labels),*before.as_ref().unwrap(),"task/control reflow at width {width}");
                    assert!(labels.iter().any(|(label,_)|label == "Last observed status · refreshing…"));
                    assert!(!labels.iter().any(|(label,_)|label == "Refreshing plug-in status"
                        || label == "Bridge service status unavailable"));
                }
                output.textures_delta.clear();
            }
            assert_eq!(state.page,Page::Setup);
            assert_eq!(state.focus.setup_installer.as_deref(),Some("retained-installer"));
            assert!(chosen.is_none() && state.queued_action.is_none(),"browsing must not submit a mutation");
        }
    }
    #[test]
    fn readonly_queries_leave_choices_usable_but_mutation_owners_stay_busy() {
        for origin in [RequestOrigin::ProductDetail,RequestOrigin::DetailSnapshot,
            RequestOrigin::ExplicitRefresh,RequestOrigin::SilentPostMutationRefresh,RequestOrigin::BackgroundActivity] {
            let mut state = state_fixture();
            state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
            state.pending = true;
            state.origin = Some(origin);
            assert!(!state.controls_pending(),"{origin:?}");
            assert!(state.capture_chosen_action(Action::EnvironmentRescan {environment:"exact-owned-environment".into()}));
            assert!(state.controls_pending() && state.next_action().is_none());
        }
        for origin in [RequestOrigin::UserAction,RequestOrigin::InstallerImport] {
            let mut state = state_fixture();
            state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
            state.pending = true;
            state.origin = Some(origin);
            assert!(state.controls_pending());
            assert!(!state.capture_chosen_action(Action::RuntimeInstall {}));
        }
    }
    #[test]
    fn genuine_read_failure_refuses_a_queued_click_without_replay() {
        let mut state = state_fixture();
        state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        state.pending = true;
        state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        assert!(state.capture_chosen_action(Action::EnvironmentRescan {environment:"exact-owned-environment".into()}));
        state.handle_reply(Reply::Error("Current readback failed".into()));
        assert!(state.next_action().is_none());
        assert!(state.queued_action.is_none() && state.current_health_system().is_none());
        assert!(state.feedback.as_ref().unwrap().text.contains("Current status is unavailable"));
        state.handle_reply(Reply::CurrentOverview(Box::new(overview_fixture().into())));
        assert!(state.next_action().is_none(),"recovery must not replay a refused click");
    }
    #[test]
    fn same_token_offer_changes_preserve_intent_and_display_manager_refusal() {
        for change in ["service","cleanup","disabled","removed","target"] {
            let mut state = state_fixture();
            let action = Action::EnvironmentRefresh {environment:"exact-owned-environment".into(),daw:DawChoice::Bitwig};
            let mut overview = overview_fixture();
            overview.current.actions.push(AvailableAction {label:"Find and connect".into(),
                action:action.clone(),disabled_reason:None});
            state.handle_reply(Reply::Overview(Box::new(overview.clone())));
            state.snapshot = Some(overview.current.clone());
            state.pending = true;
            state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
            assert!(state.capture_chosen_action(action.clone()));
            let original = state.queued_action.as_ref().unwrap().clone();
            match change {
                "service" => overview.current.system.service = "capacity unavailable".into(),
                "cleanup" => overview.current.system.cleanup_unconfirmed = true,
                "disabled" => overview.current.actions.last_mut().unwrap().disabled_reason = Some("Affected owner is busy".into()),
                "removed" => {overview.current.actions.pop();},
                "target" => overview.current.actions.last_mut().unwrap().action = Action::EnvironmentRefresh {
                    environment:"different-environment".into(),daw:DawChoice::Reaper},
                _ => unreachable!(),
            }
            assert_eq!(overview.current.state_token,original.state_token);
            state.handle_reply(Reply::CurrentOverview(Box::new(overview.into())));
            assert_eq!(state.next_action(),Some(original),"never retarget the captured request, {change}");
            assert!(state.next_action().is_none(),"a submitted request is never replayed");
            state.pending = true;
            state.origin = Some(RequestOrigin::UserAction);
            state.handle_reply(Reply::Receipt(Receipt {schema:crate::model::OPERATOR_SCHEMA,
                accepted:false,operation:Some("exact-refused-request".into()),
                refusal:Some("The current setup offer is unavailable; refresh before trying again.".into())}));
            let feedback = state.feedback.as_ref().unwrap();
            assert_eq!(feedback.action,action);
            assert!(feedback.text.contains("current setup offer is unavailable"));
            assert!(!feedback.text.contains("not submitted"),"this request reached manager admission");
            assert!(state.next_action().is_none(),"a refused request is never replayed");
        }
    }
    #[test]
    fn age_renewal_captures_the_original_runtime_intent_without_upgrading_it() {
        let mut state = state_fixture();
        state.overview = Some(overview_fixture().into());
        state.overview_fresh = true;
        assert!(state.current_action_snapshot().is_some());
        state.last_overview = Instant::now() - Duration::from_secs(61);
        state.expire_overview();
        let token = state.current_action_snapshot().unwrap().state_token.clone();
        state.pending = true;
        state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        let action = Action::EnvironmentRuntimeSelect {
            environment: "exact-existing-setup".into(), runner: "selected-installed-runtime".into(),
            expected_environment: "exact-environment-identity".into(),
        };
        assert!(state.capture_chosen_action(action.clone()));
        assert_eq!(state.queued_action.as_ref().unwrap().state_token, token);
        assert!(state.next_action().is_none());
        let mut changed = overview_fixture();
        changed.current.state_token = "different-environment-or-consequences".into();
        state.handle_reply(Reply::CurrentOverview(Box::new(changed.into())));
        assert!(state.next_action().is_none());
        assert!(state.queued_action.is_none());
        let feedback = state.feedback.as_ref().unwrap();
        assert_eq!(feedback.action, action);
        assert!(feedback.terminal && !feedback.blocking && feedback.text.contains("setup changed"));
        assert!(state.next_action().is_none(), "never automatically replay the click");
    }
    #[test]
    fn passive_refresh_renews_records_without_renewing_readiness() {
        let mut state = state_fixture();
        state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        assert!(state.current_readiness().is_some());
        let received = Instant::now() - Duration::from_secs(61);
        state.readiness.as_mut().unwrap().1 = received;
        state.last_overview = received;
        state.expire_overview();
        assert_eq!(state.current_action_snapshot().unwrap().state_token,
            state.overview.as_ref().unwrap().current.state_token);
        assert!(state.last_readiness().is_some(), "retain the matching last observed recommendations");
        assert!(matches!(state.overview_query(), Query::CurrentOverview));
        let original_facts = serde_json::to_value(&state.readiness.as_ref().unwrap().0).unwrap();
        state.handle_reply(Reply::CurrentOverview(Box::new(overview_fixture().into())));
        assert!(state.current_action_snapshot().is_some());
        assert!(state.current_readiness().is_none());
        assert!(state.last_readiness().is_some(), "a passive read must not erase an unfinished setup recommendation");
        assert_eq!(state.readiness.as_ref().unwrap().1, received);
        assert_eq!(serde_json::to_value(&state.readiness.as_ref().unwrap().0).unwrap(), original_facts);
        state.explicit_refresh_queued = true;
        assert!(matches!(state.overview_query(), Query::Overview));
        state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        assert!(state.current_readiness().is_some());
    }
    #[test]
    fn passive_changes_do_not_collect_platform_facts_but_user_workflows_do() {
        let mut state = state_fixture();
        assert!(matches!(state.overview_query(), Query::Overview));
        state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        let mut changed = pulse_fixture(false);
        changed.current_generation = "changed".into();
        state.handle_reply(Reply::Pulse(changed));
        assert!(state.refresh_after && !state.overview_fresh);
        assert!(matches!(state.overview_query(), Query::CurrentOverview));
        let mut current: CurrentOverview = overview_fixture().into();
        current.current.state_token = "changed-token".into();
        state.handle_reply(Reply::CurrentOverview(Box::new(current)));
        assert!(state.current_action_snapshot().is_some());
        assert!(state.current_readiness().is_none());
        assert!(state.last_readiness().is_none(), "changed bindings cannot reuse old recommendations");
        state.feedback = Some(RequestFeedback::captured(Action::RuntimeInstall {}));
        state.feedback.as_mut().unwrap().receipt(&Receipt {schema:crate::model::OPERATOR_SCHEMA,
            accepted:true,operation:Some("owned-operation".into()),refusal:None});
        state.feedback.as_mut().unwrap().observe(&serde_json::json!({
            "operation":"owned-operation","state":"completed"}));
        assert!(matches!(state.overview_query(), Query::Overview));
        state.feedback = None;
        state.pending_import_route = Some(("exact-installer".into(), true, "Installer".into()));
        assert!(matches!(state.overview_query(), Query::Overview));
    }
    #[test]
    fn changed_performance_generation_cannot_reuse_readiness_with_the_same_token_and_system() {
        let mut state = state_fixture();
        let assessed = overview_fixture();
        state.handle_reply(Reply::Overview(Box::new(assessed.clone())));
        assert!(state.current_readiness().is_some());
        let received = state.readiness.as_ref().unwrap().1;
        let mut changed: CurrentOverview = assessed.clone().into();
        changed.current_generation = "new-performance-generation".into();
        changed.current.products[0].details["added_frames"] = serde_json::json!(1024);
        assert_eq!(changed.current.state_token, assessed.current.state_token);
        assert_eq!(changed.current.system, assessed.current.system);
        state.handle_reply(Reply::CurrentOverview(Box::new(changed)));
        assert!(state.current_action_snapshot().is_some());
        assert!(state.current_readiness().is_none());
        assert!(state.last_readiness().is_none());
        assert_eq!(state.readiness.as_ref().unwrap().1, received);
        assert!(matches!(state.overview_query(), Query::CurrentOverview));
        assert_eq!(state.overview.as_ref().unwrap().current.products[0].details["added_frames"], 1024);
    }

    fn visible_form_text(shape: &egui::epaint::Shape, clip: egui::Rect,
        visible: &mut std::collections::BTreeSet<String>) {
        match shape {
            egui::epaint::Shape::Text(text) if clip.contains_rect(text.visual_bounding_rect()) => {
                visible.insert(text.galley.text().into());
            }
            egui::epaint::Shape::Vec(shapes) => for shape in shapes {
                visible_form_text(shape, clip, visible);
            },
            _ => {},
        }
    }

    #[test]
    fn stale_health_values_do_not_appear_as_current_service_facts() {
        for retained in [false, true] {
            let ctx = result_form_context();
            let mut state = state_fixture();
            state.overview = retained.then(|| overview_fixture().into());
            state.overview_fresh = false;
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(560.0, 360.0));
            let mut visible = std::collections::BTreeSet::new();
            for _ in 0..3 {
                let mut output = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| {
                    Operator::health_bar(ui, state.current_health_system(), state.current_owned_pause(),
                                state.overview_renewing);
                });
                for shape in &output.shapes {
                    visible_form_text(&shape.shape, shape.clip_rect, &mut visible);
                }
                output.textures_delta.clear();
            }
            assert!(visible.contains("Bridge service status unavailable"));
            assert!(visible.contains("Service details"));
            assert!(!visible.iter().any(|text| text.contains("DSP") || text.contains("Service ready")));
        }
    }

    #[test]
    fn result_form_actions_stay_visible_and_every_field_can_be_scrolled_into_view() {
        for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 720.0), egui::vec2(560.0, 360.0)] {
            for problem in [false, true] {
                let ctx = result_form_context();
                let mut form = result_form_fixture();
                if problem {
                    if let Action::CompatibilityResult { result, failed_area, .. } = &mut form {
                        *result = TestResultKind::Problem { category: ProblemCategory::Other };
                        *failed_area = Some(TestArea::Editor);
                    }
                }
                let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
                let mut visible = std::collections::BTreeSet::new();
                let mut pointer = egui::pos2(200.0, 100.0);
                let mut initial_actions = None;
                for frame in 0..36 {
                    let events = if frame > 3 { vec![egui::Event::PointerMoved(pointer),
                        egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(0.0, -60.0), phase: egui::TouchPhase::Move,
                            modifiers: egui::Modifiers::NONE }] } else { vec![] };
                    let mut output = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), events,
                        ..Default::default() }, |_ui| {
                        let (submit, cancel) = show_product_form(&ctx, &mut form, Some(&Ok(None)), false).unwrap();
                        if frame >= 3 {
                            for response in [&submit, &cancel] {
                                assert!(screen.contains_rect(response.rect), "{size:?}: {:?}", response.rect);
                                assert!(response.rect.height() >= 44.0);
                            }
                            let actions = (submit.rect, cancel.rect);
                            if let Some(initial) = initial_actions { assert_eq!(actions, initial); }
                            else { initial_actions = Some(actions); }
                            assert!(submit.enabled());
                            pointer = egui::pos2(submit.rect.center().x, (submit.rect.top() + 57.0) / 2.0);
                        }
                    });
                    if frame >= 3 {
                        for clipped in &output.shapes {
                            visible_form_text(&clipped.shape, clipped.clip_rect.intersect(screen), &mut visible);
                        }
                    }
                    output.textures_delta.clear();
                }
                for area in test_areas() {
                    assert!(visible.contains(test_area_label(area)), "{size:?}, problem={problem}: {}", test_area_label(area));
                }
                assert!(visible.contains("Layout test detail"), "note field is inaccessible at {size:?}");
                if problem { assert!(visible.contains("Which part had the problem?")); }
                assert!(visible.contains("Record this test result"));
                assert!(visible.contains("Cancel"));
            }
        }
    }

    #[test]
    fn result_form_visible_submit_still_enforces_readback_validation_and_pending_authority() {
        for (offer, pending, problem, expected) in [
            (Ok(None), false, false, true),
            (Ok(None), true, false, false),
            (Err("Current manager readback unavailable"), false, false, false),
            (Err("Current manager readback unavailable"), false, true, false),
            (Ok(Some("This plug-in has not retired".into())), false, false, false),
            (Ok(Some("This plug-in has not retired".into())), false, true, true),
        ] {
            let ctx = result_form_context();
            let mut form = result_form_fixture();
            if problem {
                if let Action::CompatibilityResult { result, .. } = &mut form {
                    *result = TestResultKind::Problem { category: ProblemCategory::BlankEditor };
                }
            }
            let mut point = egui::Pos2::ZERO;
            let mut clicked = false;
            for frame in 0..6 {
                let events = if frame == 4 || frame == 5 { vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton { pos: point, button: egui::PointerButton::Primary,
                        pressed: frame == 4, modifiers: egui::Modifiers::NONE }] } else { vec![] };
                let mut output = ctx.run_ui(egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(560.0, 360.0))),
                    events, ..Default::default()
                }, |_ui| {
                    let (submit, cancel) = show_product_form(&ctx, &mut form, Some(&offer), pending).unwrap();
                    if frame >= 3 {
                        assert_eq!(submit.enabled(), expected);
                        assert!(cancel.enabled());
                        assert!(ctx.content_rect().contains_rect(submit.rect));
                        assert!(ctx.content_rect().contains_rect(cancel.rect));
                    }
                    point = submit.rect.center();
                    clicked |= submit.clicked();
                });
                output.textures_delta.clear();
            }
            assert_eq!(clicked, expected, "pending={pending}, problem={problem}, offer={offer:?}");
        }
        for problem in [false, true] {
            let ctx = result_form_context();
            let mut form = result_form_fixture();
            if let Action::CompatibilityResult { result, passed, note, .. } = &mut form {
                passed.clear();
                note.clear();
                if problem { *result = TestResultKind::Problem { category: ProblemCategory::Other }; }
            }
            let mut output = ctx.run_ui(egui::RawInput::default(), |_ui| {
                let (submit, _) = show_product_form(&ctx, &mut form, Some(&Ok(None)), false).unwrap();
                assert!(!submit.enabled(), "an incomplete result cannot be submitted");
            });
            output.textures_delta.clear();
        }
    }
    #[test]
    fn test_result_uses_exact_class_offer_and_keeps_problem_reporting_available() {
        let mut snapshot = overview_fixture().current;
        snapshot.system.dsp = 1;
        let candidate = "exact-candidate".to_string();
        let current = PublicationIdentity {id:"exact-publication".into(),sha256:"exact-digest".into()};
        snapshot.products[0].actions = vec![AvailableAction {label:"Record test result".into(),
            action:Action::CompatibilityResult {candidate:candidate.clone(),expected_current:current.clone(),
                result:TestResultKind::Worked,passed:vec![],failed_area:None,note:String::new()},
            disabled_reason:None}];
        assert!(snapshot.system.inactive_reason().is_some());
        let worked = TestResultKind::Worked;
        let problem = TestResultKind::Problem {category:ProblemCategory::NoAudio};
        let offer = compatibility_result_offer(Some(&snapshot),&candidate,&current);
        assert_eq!(compatibility_result_refusal(Some(&offer),&worked),None);
        snapshot.products[0].actions[0].disabled_reason = Some("This plug-in has not retired".into());
        let offer = compatibility_result_offer(Some(&snapshot),&candidate,&current);
        assert_eq!(compatibility_result_refusal(Some(&offer),&worked),Some("This plug-in has not retired"));
        assert_eq!(compatibility_result_refusal(Some(&offer),&problem),None);
        let changed = PublicationIdentity {id:"changed-publication".into(),..current.clone()};
        let stale = compatibility_result_offer(Some(&snapshot),&candidate,&changed);
        assert!(compatibility_result_refusal(Some(&stale),&worked).is_some());
        assert!(compatibility_result_refusal(Some(&stale),&problem).is_some());
        let unavailable = compatibility_result_offer(None,&candidate,&current);
        assert!(compatibility_result_refusal(Some(&unavailable),&problem).is_some());
    }
    #[test]
    fn home_separates_operational_readiness_from_support_qualification() {
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(items) => for item in items { texts(item, out); },
                _ => {}
            }
        }
        for status in [ReadinessOutcome::Ready, ReadinessOutcome::Unknown] {
            let mut overview = overview_fixture();
            overview.readiness.overall_status = status;
            overview.current.products.truncate(1);
            overview.current.products[0].class_id = "exact-class".into();
            overview.current.products[0].module_sha256 = "exact-module".into();
            overview.readiness.products = vec![crate::model::ReadinessProduct {
                name:"Unfamiliar plug-in".into(),class_id:"exact-class".into(),module_sha256:"exact-module".into(),
                profile:None,status,installation_health:InstallationHealth::Healthy,
                support_qualification:SupportQualification::NotYetQualified,
                reason:"No published support claim for this installation".into(),failure_code:None,facts:vec![],
            }];
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                Operator::fast_home(ui,&overview.clone().into(),Some(&overview.readiness),true,false,
                    (&mut Page::Home,&mut crate::library::Library::default(),&mut RouteFocus::default(),&mut false,&mut None));
            });
            let mut labels = vec![];
            for clipped in &output.shapes { texts(&clipped.shape,&mut labels); }
            output.textures_delta.clear();
            // Missing historical qualification does not erase actual DAW availability.
            assert!(labels.iter().any(|line| line == "Available in your DAW"));
            assert!(!labels.iter().any(|line| line.starts_with("Support qualification:")));
            assert!(!labels.iter().any(|line| line.contains("Setup check incomplete")));
            assert!(labels.iter().any(|line| line == "Plug-ins available in your DAW"));
            assert!(labels.iter().any(|line| line == "Using your plug-ins"));
            assert!(!labels.iter().any(|line| line.starts_with("Next:") || line.contains("add it to a track")));
            assert!(!labels.iter().any(|line| line.contains("Operational facts")));
        }
    }
    #[test]
    fn completed_home_states_availability_and_keeps_usage_help_optional() {
        for (choices, expected) in [
            ([Some(DawChoice::Bitwig),Some(DawChoice::Bitwig)], "Plug-ins available in Bitwig"),
            ([Some(DawChoice::Reaper),Some(DawChoice::Reaper)], "Plug-ins available in REAPER"),
            ([Some(DawChoice::Other),Some(DawChoice::Other)], "Plug-ins available in your DAW"),
            ([None,None], "Plug-ins available in your DAW"),
            ([Some(DawChoice::Bitwig),Some(DawChoice::Reaper)], "Plug-ins available in your DAW"),
            ([Some(DawChoice::Bitwig),None], "Plug-ins available in your DAW"),
        ] {
            let mut overview:CurrentOverview=overview_fixture().into();
            overview.current.products.truncate(2);
            overview.current.installer_setups.clear();
            for (product, choice) in overview.current.products.iter_mut().zip(choices) {
                product.availability=Some(NativeAvailability {available:true,reason:None});
                product.compatibility=None;
                if let Some(daw)=choice {
                    let mut setup=automatic_setup(InstallationJourney {daw,
                        phase:InstallationPhase::Available,failure:None,
                        plugins:vec![InstallationPlugin {selection:"connected-selection".into(),
                            class_id:product.class_id.clone(),name:product.name.clone(),
                            stage:InstallationPluginStage::Available,failure:None}]},None);
                    setup.environment=Some(product.environment.clone());
                    setup.discovered=vec![DiscoveredProduct {environment:product.environment.clone(),
                        module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone(),
                        name:product.name.clone()}];
                    overview.current.installer_setups.push(setup);
                }
            }
            let ctx=result_form_context();
            let mut chosen=None;let mut point=egui::Pos2::ZERO;
            for frame in 0..8 {
                let events=if matches!(frame,3|4) {vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed:frame==3,modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(
                    egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(560.0,1000.0))),
                    ..Default::default()},|ui| {
                    Operator::fast_home(ui,&overview,None,true,false,
                        (&mut Page::Home,&mut crate::library::Library::default(),
                            &mut RouteFocus::default(),&mut false,&mut chosen));
                });
                let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
                assert!(labels.iter().any(|(label,_)|label==expected));
                assert!(!labels.iter().any(|(label,_)|label.starts_with("Next:")
                    || label.contains("activation works") || label.contains("audio works")));
                if frame==2 {
                    assert!(!labels.iter().any(|(label,_)|label.starts_with("Available plug-ins appear")));
                    point=labels.iter().find(|(label,_)|label=="Using your plug-ins").unwrap().1;
                }
                if frame==7 {assert!(labels.iter().any(|(label,_)|label.starts_with("Available plug-ins appear")));}
                output.textures_delta.clear();
            }
            assert!(chosen.is_none(), "Reading usage help must not submit a mutation or success report");
        }
    }
    fn text_centers(shape: &egui::epaint::Shape, out: &mut Vec<(String,egui::Pos2)>) {
        match shape {
            egui::epaint::Shape::Text(text) => out.push((text.galley.text().into(),text.pos+text.galley.size()*0.5)),
            egui::epaint::Shape::Vec(items) => for item in items {text_centers(item,out);},
            _ => {},
        }
    }
    #[test]
    fn home_manage_routes_same_named_class_to_the_exact_environment_even_when_stale() {
        for (width,fresh) in [(560.0,true),(960.0,true),(560.0,false)] {
            let mut overview:CurrentOverview=overview_fixture().into();
            let mut first=overview.current.products[0].clone();
            first.name="Same plug-in name".into();first.environment="first-space".into();
            let mut second=first.clone();second.environment="second-space".into();
            let exact=ProductKey::from(&second);
            overview.current.products=vec![first,second];
            let ctx=result_form_context();
            presentation::configure_style(&ctx);
            let mut page=Page::Home;let mut library=crate::library::Library::default();
            let mut focus=RouteFocus::default();let mut chosen=None;let mut point=egui::Pos2::ZERO;
            for frame in 0..5 {
                let events=if frame>=3 {vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed:frame==3,modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,egui::vec2(width,1000.0))),..Default::default()},|ui| {
                    Operator::fast_home(ui,&overview,None,fresh,false,
                        (&mut page,&mut library,&mut focus,&mut false,&mut chosen));
                });
                if frame==2 {
                    let mut text=vec![];
                    for shape in &output.shapes {text_centers(&shape.shape,&mut text);}
                    point=text.iter().filter(|(label,_)|label=="Manage plug-in").nth(1).unwrap().1;
                }
                output.textures_delta.clear();
            }
            assert_eq!(page,Page::Plugins);
            assert_eq!(library.focused_product(),Some(&exact));
            assert!(chosen.is_none(),"Browsing must not submit a setup mutation");
        }
    }
    #[test]
    fn home_specific_blocker_without_an_ordered_step_never_recommends_starting_the_daw() {
        for outcome in [ReadinessOutcome::Unsupported,ReadinessOutcome::Unknown] {
            let mut overview=overview_fixture();overview.current.products.truncate(1);
            overview.readiness.overall_status=outcome;
            overview.readiness.blockers=vec![ReadinessBlocker {category:"engine_architecture".into(),
                status:outcome,explanation:"This native engine requires x86-64 Linux.".into()}];
            let ctx=result_form_context();
            let mut output=ctx.run_ui(egui::RawInput::default(),|ui| {
                Operator::fast_home(ui,&overview.clone().into(),Some(&overview.readiness),true,false,
                    (&mut Page::Home,&mut crate::library::Library::default(),&mut RouteFocus::default(),&mut false,&mut None));
            });
            let mut text=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut text);}
            assert!(text.iter().any(|(label,_)|label=="This native engine requires x86-64 Linux."));
            assert!(!text.iter().any(|(label,_)|label.starts_with("Plug-ins available in ")));
            assert!(text.iter().any(|(label,_)|label=="Review setup details"));
            // A computer limitation does not erase the separately recorded publication.
            assert!(text.iter().any(|(label,_)|label=="Available in your DAW"));
            output.textures_delta.clear();
        }
    }
    #[test]
    fn retained_installation_maintenance_does_not_displace_available_plugins_but_failures_do() {
        for failed in [false, true] {
            let mut overview: CurrentOverview = overview_fixture().into();
            overview.current.products.truncate(1);
            let product = &mut overview.current.products[0];
            product.disposition = "ready".into();
            let installer = "retained-installer".to_string();
            let setup = InstallerSetup {installer:installer.clone(),name:"Retained installer".into(),
                label_source:"operator_named".into(),byte_size:1024,format:"pe_executable".into(),imported_at:1,
                phase:SetupPhase::SetupNeedsAttention,status:"Review retained installation records.".into(),
                environment:Some(product.environment.clone()),compatibility:None,
                discovered:vec![DiscoveredProduct {environment:product.environment.clone(),
                    module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone(),name:product.name.clone()}],
                primary:None,secondary:vec![],rename:AvailableAction {label:"Rename".into(),
                    action:Action::InstallerRename {installer:installer.clone(),label:String::new()},disabled_reason:None},
                journey:None,history:vec![]};
            overview.current.onboarding=vec![crate::model::Onboarding {failure:None,installer,
                name:setup.name.clone(),byte_size:1024,format:setup.format.clone(),environment:setup.environment.clone(),
                state:"needs_attention".into(),required_human_action:setup.status.clone(),actions:vec![],
                details:serde_json::json!({"managed_product":true,"scan":{},"installation":{
                    "state":if failed {"failed"} else {"completed"},"cleanup_confirmed":true,"owned_live":0}})}];
            overview.current.installer_setups=vec![setup];
            let ctx=result_form_context();presentation::configure_style(&ctx);
            let mut page=Page::Home;let mut library=crate::library::Library::default();
            let mut focus=RouteFocus::default();let mut point=egui::Pos2::ZERO;
            for frame in 0..5 {
                let events=if frame>=3 {vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                    pos:point,button:egui::PointerButton::Primary,pressed:frame==3,modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,egui::vec2(560.0,1000.0))),..Default::default()},|ui| {
                    Operator::fast_home(ui,&overview,None,true,false,
                        (&mut page,&mut library,&mut focus,&mut false,&mut None));
                });
                if frame==2 {
                    let mut text=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut text);}
                    assert_eq!(text.iter().any(|(label,_)|label=="Next: continue Retained installer"),failed);
                    point=text.iter().find(|(label,_)|label==if failed {"Open this setup"} else {"Manage plug-in"}).unwrap().1;
                }
                output.textures_delta.clear();
            }
            assert_eq!(page,if failed {Page::Setup} else {Page::Plugins});
            if failed {assert_eq!(focus.setup_installer.as_deref(),Some("retained-installer"));}
            else {assert_eq!(library.focused_product(),Some(&ProductKey::from(&overview.current.products[0])));}
        }
    }
    #[test]
    fn current_available_products_settle_history_without_rewriting_it() {
        let mut overview: CurrentOverview = overview_fixture().into();
        overview.current.products.truncate(1);
        let product = &mut overview.current.products[0];
        product.disposition = "ready".into();
        product.compatibility = None;
        product.availability = Some(NativeAvailability {available:true,reason:None});
        let environment = product.environment.clone();
        let discovered = DiscoveredProduct {environment:environment.clone(),
            module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone(),name:product.name.clone()};
        let installer = "retained-partial-installer".to_owned();
        overview.current.installer_setups = vec![InstallerSetup {installer:installer.clone(),name:"Earlier setup".into(),
            label_source:"operator_named".into(),byte_size:1024,format:"pe_executable".into(),imported_at:1,
            phase:SetupPhase::SetupNeedsAttention,status:"Earlier installation completeness was unproved.".into(),
            environment:Some(environment.clone()),compatibility:None,discovered:vec![discovered],primary:None,secondary:vec![],
            rename:AvailableAction {label:"Rename".into(),action:Action::InstallerRename {installer:installer.clone(),label:String::new()},disabled_reason:None},
            journey:None,history:vec![]}];
        overview.current.onboarding = vec![crate::model::Onboarding {installer,name:"Earlier setup".into(),byte_size:1024,
            format:"pe_executable".into(),environment:Some(environment),state:"needs_attention".into(),failure:None,
            required_human_action:"Review retained history before installation changes".into(),actions:vec![],
            details:serde_json::json!({"installation":{"state":"completed","cleanup_confirmed":true,"owned_live":0,
                "error":null,"failure":null,"transaction":{"outcome":"partial_installation",
                    "installation_completeness":"unproved","first_failure":null}}})}];
        let retained = serde_json::to_value(&overview.current.onboarding).unwrap();
        assert!(setup_history_settled_by_current_products(&overview.current.installer_setups[0],&overview.current));
        for outcome in ["partial_installation","installed"] {
            for available in [true,false] {
                let mut readback=overview.clone();
                readback.current.onboarding[0].details["managed_product"]=serde_json::json!(true);
                readback.current.onboarding[0].details["scan"]=serde_json::json!({});
                readback.current.onboarding[0].details["installation"]["transaction"]["outcome"]=serde_json::json!(outcome);
                readback.current.products[0].availability.as_mut().unwrap().available=available;
                let ctx = result_form_context();
                let mut output = ctx.run_ui(egui::RawInput::default(),|ui| {
                    Operator::fast_home(ui,&readback,None,true,false,(&mut Page::Home,
                        &mut crate::library::Library::default(),&mut RouteFocus::default(),&mut false,&mut None));
                });
                let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
                assert_eq!(labels.iter().any(|(label,_)|label.contains("Earlier setup")
                    && label.starts_with("Next:")),!available,"{outcome}, available={available}");
                output.textures_delta.clear();
            }
        }
        assert_eq!(serde_json::to_value(&overview.current.onboarding).unwrap(),retained);
        for case in 0..12 {
            let mut changed=overview.clone();
            match case {
                0=>changed.current.installer_setups[0].environment=Some("different-environment".into()),
                1=>changed.current.products[0].module_sha256="changed-module".into(),
                2=>changed.current.products[0].environment="different-environment".into(),
                3=>changed.current.products.clear(),
                4=>changed.current.products[0].availability=Some(NativeAvailability {available:false,reason:Some("Current connection failed".into())}),
                5=>changed.current.products[0].availability=None,
                6=>{let environment=changed.current.installer_setups[0].environment.clone().unwrap();
                    changed.current.installer_setups[0].discovered.push(DiscoveredProduct {environment,
                        module_sha256:"missing-module".into(),class_id:"missing-sibling".into(),name:"Missing sibling".into()});},
                7|8=>changed.current.installer_setups[0].journey=Some(InstallationJourney {daw:DawChoice::Bitwig,
                    phase:if case==7 {InstallationPhase::VendorRunning}else{InstallationPhase::NeedsAttention},plugins:vec![],
                    failure:if case==7 {None}else{Some(InstallationFailure {stage:InstallationFailureStage::Inspection,
                        reason:"Current inspection failed".into(),code:"inspection_failed".into(),retryable:true})}}),
                9=>changed.current.installer_setups[0].primary=Some(AvailableAction {label:"Resume".into(),
                    action:Action::SoftwareInstallResume {onboarding:"exact-owner".into()},disabled_reason:None}),
                10=>changed.current.onboarding[0].actions.push(AvailableAction {label:"Repair".into(),
                    action:Action::SoftwareInstallResume {onboarding:"exact-owner".into()},disabled_reason:None}),
                _=>changed.current.onboarding[0].details["installation"]["transaction"]["first_failure"]=serde_json::json!("retained failure"),
            }
            assert!(!setup_history_settled_by_current_products(&changed.current.installer_setups[0],&changed.current),"case {case}");
        }
    }

    #[test]
    fn setup_details_show_retained_partial_receipt_while_current_plugin_is_available() {
        let mut overview:CurrentOverview=overview_fixture().into();
        overview.current.vendor_applications.clear();
        let product=&mut overview.current.products[0];
        product.availability=Some(NativeAvailability {available:true,reason:None});
        overview.current.installer_setups=vec![InstallerSetup {installer:"retained-installer".into(),
            name:"Earlier setup".into(),label_source:"operator_named".into(),byte_size:1024,
            format:"pe_executable".into(),imported_at:1,phase:SetupPhase::DiscoveryComplete,
            journey:None,status:"Current plug-in available".into(),environment:Some(product.environment.clone()),
            compatibility:None,discovered:vec![],primary:None,secondary:vec![],history:vec![],
            rename:AvailableAction {label:"Rename".into(),action:Action::InstallerRename {
                installer:"retained-installer".into(),label:String::new()},disabled_reason:None}}];
        let setup=&overview.current.installer_setups[0];
        let installer=setup.installer.clone();let environment=setup.environment.clone();
        overview.current.onboarding=vec![crate::model::Onboarding {failure:None,installer:String::new(),
            name:"Earlier setup".into(),byte_size:1024,format:"pe_executable".into(),environment:None,
            state:"completed".into(),required_human_action:String::new(),details:serde_json::Value::Null,actions:vec![]}];
        let attempt=&mut overview.current.onboarding[0];
        attempt.installer=installer;attempt.environment=environment;
        attempt.details=serde_json::json!({"installation":{"operation":"retained-operation",
            "state":"completed","transaction":{"schema":1,"operation":"retained-operation",
                "outcome":"partial_installation","durable_installation":"partial_installation",
                "installation_completeness":"unproved"},
            "installer_capability":{"requested":{"powershell":"inherited"},
                "effective":{"effective":{"windows_scripting":{"powershell":"inherited"}}}}}});
        let original=attempt.details.clone();
        let ctx=result_form_context();
        let mut style=(*ctx.global_style()).clone();style.animation_time=0.0;ctx.set_global_style(style);
        let mut chosen=None;
        let render=|events:Vec<egui::Event>,chosen:&mut Option<Action>| {
            let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,egui::vec2(1000.0,1600.0))),..Default::default()},|ui| {
                Operator::fast_setup(ui,&overview,true,false,(&mut false,chosen,&mut false,&mut Page::Setup,
                    &mut crate::library::Library::default(),&mut RouteFocus::default(),&mut Some(DawChoice::Bitwig)));
            });
            let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            output.textures_delta.clear();labels
        };
        render(vec![],&mut chosen);
        let labels=render(vec![],&mut chosen);
        let point=labels.iter().find(|(label,_)|label=="Setup details and options").unwrap().1;
        for pressed in [true,false] {
            render(vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {pos:point,
                button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}],&mut chosen);
        }
        let labels=render(vec![],&mut chosen);
        let text=labels.iter().map(|(label,_)|label.as_str()).collect::<Vec<_>>().join(" ");
        assert!(text.contains("Installation changes observed; completion not verified"),"{text}");
        assert!(text.contains("Requested: inherited scripting behavior."),"{text}");
        assert!(text.contains("Applied at target launch: inherited scripting behavior."),"{text}");
        assert_eq!(overview.current.onboarding[0].details,original);
        assert!(chosen.is_none());
    }
    #[test]
    fn installed_program_choices_and_registered_apps_keep_their_owned_setup_context() {
        let mut overview:CurrentOverview=overview_fixture().into();
        overview.current.installer_setups.clear();
        overview.current.products.truncate(1);
        overview.current.products[0].environment="older-owned-environment".into();
        overview.current.products[0].name="Older instrument".into();
        let mut second=overview.current.products[0].clone();second.environment="newer-owned-environment".into();
        second.name="Newer effect".into();second.class_id="other-class".into();overview.current.products.push(second);
        let row=|id:&str,environment:&str,name:&str,choosing:bool|VendorApplication {id:id.into(),name:name.into(),
            version:"1.0".into(),state:"closed".into(),details:serde_json::json!({"environment":environment,"choose_application":choosing}),
            actions:vec![AvailableAction {label:format!("Open {name}"),disabled_reason:None,
                action:Action::VendorSoftwareInstall {application:id.into(),daw:DawChoice::Other}}]};
        overview.current.vendor_applications=vec![row("older-app","older-owned-environment","Same app",false),
            row("newer-app","newer-owned-environment","Same app",false),
            row("hub-choice","fresh-owned-environment","Source hub (hub)",true),
            row("helper-choice","fresh-owned-environment","Source hub (helper)",true)];
        let groups=application_groups(&overview.current,&RouteFocus::default());
        assert_eq!(groups.len(),3);assert_eq!(groups[2].applications.len(),2);
        let focus=RouteFocus {setup:Some(AttemptKey {installer:"exact-import".into(),environment:Some("older-owned-environment".into())}),..Default::default()};
        let focused=application_groups(&overview.current,&focus);assert_eq!(focused.len(),1);
        assert_eq!(focused[0].applications[0].id,"older-app");
        let ctx=result_form_context();let mut chosen=None;
        let mut output=ctx.run_ui(egui::RawInput {screen_rect:Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,egui::vec2(1000.0,1400.0))),..Default::default()},|ui| {
            Operator::fast_setup(ui,&overview,true,false,(&mut false,&mut chosen,&mut false,&mut Page::Setup,
                &mut crate::library::Library::default(),&mut RouteFocus::default(),&mut Some(DawChoice::Bitwig)));
        });
        let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
        assert!(labels.iter().any(|(label,_)|label=="Plug-ins in this setup: Older instrument"));
        assert!(labels.iter().any(|(label,_)|label=="Plug-ins in this setup: Newer effect"));
        assert_eq!(labels.iter().filter(|(label,_)|label=="Choose installed program").count(),1);
        assert!(labels.iter().any(|(label,_)|label=="Open selected app"));
        assert!(!labels.iter().any(|(label,_)|label=="Open Source hub (helper)" || label=="Open Source hub (hub)"));
        assert!(!labels.iter().any(|(label,_)|label.contains("owned-environment")));
        assert!(chosen.is_none());output.textures_delta.clear();
    }

    #[test]
    fn explicit_program_selection_opens_only_that_program_in_its_own_setup() {
        let mut overview:CurrentOverview=overview_fixture().into();
        overview.current.installer_setups.clear();overview.current.products.clear();
        let row=|id:&str,environment:&str,name:&str|VendorApplication {id:id.into(),name:name.into(),
            version:"1.0".into(),state:"closed".into(),details:serde_json::json!({"environment":environment,"choose_application":true}),
            actions:vec![AvailableAction {label:format!("Open {name}"),disabled_reason:None,
                action:Action::VendorSoftwareInstall {application:id.into(),daw:DawChoice::Other}}]};
        overview.current.vendor_applications=vec![row("left-hub","left-setup","Source app (hub)"),
            row("left-helper","left-setup","Source app (helper)"),row("right-hub","right-setup","Source app (hub)"),
            row("right-helper","right-setup","Source app (helper)")];
        let ctx=result_form_context();let mut chosen=None;
        let render=|overview:&CurrentOverview,events:Vec<egui::Event>,chosen:&mut Option<Action>| {
            let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,egui::vec2(1000.0,1000.0))),..Default::default()},|ui| {
                Operator::fast_setup(ui,overview,true,false,(&mut false,chosen,&mut false,&mut Page::Setup,
                    &mut crate::library::Library::default(),&mut RouteFocus::default(),&mut Some(DawChoice::Bitwig)));
            });
            let mut labels=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut labels);}
            output.textures_delta.clear();labels
        };
        let click=|point:egui::Pos2,pressed|vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
            pos:point,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}];
        render(&overview,vec![],&mut chosen);
        let labels=render(&overview,vec![],&mut chosen);
        let combo=labels.iter().find(|(label,_)|label=="Choose installed program").unwrap().1;
        render(&overview,click(combo,true),&mut chosen);render(&overview,click(combo,false),&mut chosen);
        let labels=render(&overview,vec![],&mut chosen);
        let helper=labels.iter().find(|(label,_)|label=="Source app (helper)").unwrap().1;
        render(&overview,click(helper,true),&mut chosen);render(&overview,click(helper,false),&mut chosen);
        let labels=render(&overview,vec![],&mut chosen);
        assert_eq!(labels.iter().filter(|(label,_)|label=="Choose installed program").count(),1);
        let open=labels.iter().find(|(label,_)|label=="Open Source app (helper)").unwrap().1;
        assert!(chosen.is_none());
        render(&overview,click(open,true),&mut chosen);render(&overview,click(open,false),&mut chosen);
        assert_eq!(chosen,Some(Action::VendorSoftwareInstall {application:"left-helper".into(),daw:DawChoice::Other}));
        chosen=None;
        overview.current.vendor_applications.retain(|app|app.id!="left-helper");
        let labels=render(&overview,vec![],&mut chosen);
        assert_eq!(labels.iter().filter(|(label,_)|label=="Choose installed program").count(),2);
        assert!(!labels.iter().any(|(label,_)|label=="Open Source app (helper)"));assert!(chosen.is_none());
    }

    #[test]
    fn mixed_retained_installations_review_the_partial_setup_without_current_availability() {
        let mut overview:CurrentOverview=overview_fixture().into();
        let base=overview.current.products[0].clone();
        overview.current.products.clear();overview.current.installer_setups.clear();overview.current.onboarding.clear();
        // The retained Deck shape: six maintenance records and two partial outcomes.
        for (index,(count,outcome)) in [(2,None),(1,Some("partial_installation")),(1,Some("installed")),
            (9,Some("installed")),(1,Some("partial_installation")),(2,Some("installed")),
            (1,Some("installed")),(2,Some("installed"))].into_iter().enumerate() {
            let installer=format!("installer-{index}");let environment=format!("environment-{index}");
            let mut discovered=vec![];
            for class in 0..count {
                let mut product=base.clone();product.environment=environment.clone();
                product.module_sha256=format!("module-{index}-{class}");product.class_id=format!("class-{index}-{class}");
                product.name=format!("Plug-in {index}.{class}");product.disposition="ready".into();product.compatibility=None;
                product.availability=Some(NativeAvailability {available:!matches!(index,1|4),
                    reason:matches!(index,1|4).then(||"Current connection not confirmed".into())});
                discovered.push(DiscoveredProduct {environment:environment.clone(),module_sha256:product.module_sha256.clone(),
                    class_id:product.class_id.clone(),name:product.name.clone()});
                overview.current.products.push(product);
            }
            let name=format!("Unknown Windows installer · ID {index}");
            let status="Installation retained; use the exact managed-environment refresh below before preparing or replacing a publication";
            overview.current.installer_setups.push(InstallerSetup {installer:installer.clone(),name:name.clone(),
                label_source:"generated".into(),byte_size:1024,format:"pe_executable".into(),imported_at:index as u64,
                phase:SetupPhase::SetupNeedsAttention,status:status.into(),environment:Some(environment.clone()),
                compatibility:None,discovered,primary:None,secondary:vec![],
                rename:AvailableAction {label:"Rename".into(),action:Action::InstallerRename {installer:installer.clone(),label:String::new()},disabled_reason:None},journey:None,history:vec![]});
            overview.current.onboarding.push(crate::model::Onboarding {installer,name,byte_size:1024,format:"pe_executable".into(),
                environment:Some(environment),state:"needs_attention".into(),failure:None,required_human_action:status.into(),actions:vec![],
                details:serde_json::json!({"managed_product":true,"scan":{},"installation":{"state":"completed",
                    "cleanup_confirmed":true,"owned_live":0,"error":null,"failure":null,
                    "transaction":{"outcome":outcome,"durable_installation":outcome,"installation_completeness":"unproved",
                        "safe_next_action":"review_retained_outcome_before_retry","outer_launcher_exit":0,"first_failure":null}}})});
        }
        for (index,setup) in overview.current.installer_setups.iter().enumerate() {
            assert_eq!(retained_setup_records_need_refresh(setup,&overview.current),!matches!(index,1|4));
        }
        let intended=&overview.current.installer_setups[1];
        let mut unmatched=intended.clone();unmatched.discovered[0].module_sha256="different-module".into();
        assert_eq!(setup_display_name(&unmatched,&overview.current),unmatched.name);
        let ctx=result_form_context();presentation::configure_style(&ctx);
        let mut page=Page::Home;let mut library=crate::library::Library::default();let mut focus=RouteFocus::default();
        let mut point=egui::Pos2::ZERO;
        for frame in 0..5 {
            let events=if frame>=3 {vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                pos:point,button:egui::PointerButton::Primary,pressed:frame==3,modifiers:egui::Modifiers::NONE}]} else {vec![]};
            let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,egui::vec2(560.0,1200.0))),..Default::default()},|ui| {
                Operator::fast_home(ui,&overview,None,true,false,
                    (&mut page,&mut library,&mut focus,&mut false,&mut None));
            });
            if frame==2 {
                let mut text=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut text);}
                assert!(text.iter().any(|(label,_)|label=="Next: review the installation for Plug-in 1.0"));
                point=text.iter().find(|(label,_)|label=="Review this installation").unwrap().1;
                assert!(!text.iter().any(|(label,_)|label.starts_with("Plug-ins available in ")));
            }
            output.textures_delta.clear();
        }
        assert_eq!(page,Page::Setup);
        assert_eq!(focus.setup,Some(AttemptKey {installer:intended.installer.clone(),environment:intended.environment.clone()}));
    }
    #[test]
    fn live_installer_beats_an_older_import_and_keeps_pending_controls_closed() {
        for page in [Page::Home, Page::Setup] {
        for pending in [false,true] {
            let mut overview:CurrentOverview=overview_fixture().into();
            let action=Action::InstallerFocus {onboarding:"exact-space".into(),operation:"exact-operation".into()};
            let make=|name:&str,phase,primary|InstallerSetup {
                installer:name.into(),name:name.into(),label_source:"operator_named".into(),byte_size:1024,
                format:"pe_executable".into(),imported_at:1,phase,status:"Continue in the vendor window.".into(),
                environment:Some("exact-space".into()),compatibility:None,discovered:vec![],primary,
                secondary:vec![],rename:AvailableAction {label:"Rename".into(),
                    action:Action::InstallerRename {installer:name.into(),label:String::new()},disabled_reason:None},journey:None,history:vec![],
            };
            overview.current.installer_setups=vec![make("Older import",SetupPhase::Imported,None),
                make("Current installation",SetupPhase::InstallerRunning,Some(AvailableAction {
                    label:"Show vendor installer".into(),action:action.clone(),disabled_reason:None}))];
            let ctx=result_form_context();presentation::configure_style(&ctx);
            let mut point=egui::Pos2::ZERO;let mut chosen=None;
            for frame in 0..5 {
                let events=if frame>=3 {vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed:frame==3,modifiers:egui::Modifiers::NONE}]} else {vec![]};
                let mut output=ctx.run_ui(egui::RawInput {events,screen_rect:Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,egui::vec2(560.0,1000.0))),..Default::default()},|ui| {
                    ui.add_space(285.0);
                    if page == Page::Home {
                        Operator::fast_home(ui,&overview,None,true,pending,
                            (&mut Page::Home,&mut crate::library::Library::default(),&mut RouteFocus::default(),&mut false,&mut chosen));
                    } else {
                        Operator::fast_setup(ui,&overview,true,pending,
                            (&mut false,&mut chosen,&mut false,&mut Page::Setup,
                                &mut crate::library::Library::default(),&mut RouteFocus::default(),&mut None));
                    }
                });
                if frame==2 {
                    let mut text=vec![];for shape in &output.shapes {text_centers(&shape.shape,&mut text);}
                    if page == Page::Home {
                        assert!(text.iter().any(|(label,_)|label=="Next: continue Current installation"));
                        assert!(!text.iter().any(|(label,_)|label=="Next: continue Older import"));
                    } else {
                        let current=text.iter().find(|(label,_)|label=="Current installation").unwrap().1;
                        let older=text.iter().find(|(label,_)|label=="Older import").unwrap().1;
                        assert!(current.y < older.y,"The running setup must precede a dormant import");
                    }
                    point=text.iter().find(|(label,_)|label=="Show vendor installer").unwrap().1;
                    assert!(point.y < 700.0,"The current action must fit the first narrow screen");
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen,(!pending).then_some(action));
        }
        }
    }
    #[test]
    fn retained_product_card_requires_matching_identity_and_fresh_actions() {
        let mut state = state_fixture();
        let overview = overview_fixture();
        let product = overview.current.products[0].clone();
        let key = ProductKey::from(&product);
        state.library.focus_product(key);
        state.overview_fresh = true;
        state.overview = Some(overview.clone().into());
        let mut detailed = product;
        detailed.name = "Exact scoped detail".into();
        state.product_detail = Some(CurrentProductDetail {
            schema: 1, operator_schema: crate::model::OPERATOR_SCHEMA,
            state_token: overview.current.state_token.clone(),
            current_generation: overview.current_generation.clone(), product: detailed,
            system: overview.current.system.clone(),
            environments: vec![], vendor_applications: vec![],
        });
        let selected = state.selected_product_snapshot().unwrap();
        assert_eq!(selected.products[0].name, "Exact scoped detail");
        assert_eq!(selected.products[1].name, overview.current.products[1].name);
        state.overview.as_mut().unwrap().current.state_token = "changed".into();
        assert!(state.selected_product_snapshot().is_none());
        state.overview.as_mut().unwrap().current.state_token = overview.current.state_token;
        state.overview_fresh = false;
        assert!(state.selected_product_snapshot().is_some());
        assert!(state.current_action_snapshot().is_none());
        state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        state.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        assert_eq!(state.selected_product_snapshot().unwrap().products[0].name,
            "Exact scoped detail");
        assert!(state.current_action_snapshot().is_some());
        let mut changed = overview_fixture();
        changed.current.state_token = "changed".into();
        state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        state.handle_reply(Reply::Overview(Box::new(changed)));
        assert!(state.product_detail.is_none());
        assert!(state.selected_product_snapshot().is_none());
    }
    fn pulse_fixture(live: bool) -> crate::model::Pulse {
        crate::model::Pulse {schema:crate::model::OPERATOR_SCHEMA,service_state:"active".into(),
            dsp:Some(0),keepers:Some(0),maintenance:Some(0),pending_transactions:0,
            cleanup_unconfirmed:Some(false),current_generation:"same".into(),
            operation:None,operation_live:live}
    }
    fn handoff_poll_fixture() -> (Operator, serde_json::Value) {
        let mut state = state_fixture();
        let mut overview = overview_fixture();
        overview.current.system.dsp = 0;
        overview.current.system.keepers = 0;
        overview.current.system.maintenance = 0;
        overview.current.system.pending_transactions = 0;
        overview.current.system.cleanup_unconfirmed = false;
        let raw = serde_json::json!({"operation":"aa".repeat(16),"state":"refused",
            "reason":"no_supported_instruments_or_effects_discovered",
            "journey":{"daw":"reaper","phase":"needs_attention","plugins":[],
                "failure":{"stage":"discovery","reason":"The installed instruments or effects could not be found or read.",
                    "code":"no_supported_instruments_or_effects_discovered",
                    "retryable":true}}});
        let mut projected = raw.clone();
        projected["journey"]["phase"] = "awaiting_application".into();
        projected["journey"]["failure"] = serde_json::Value::Null;
        overview.current.operation = Some(projected);
        let mut feedback = RequestFeedback::captured(Action::SoftwareInstall {
            installer:"bb".repeat(32), daw:DawChoice::Reaper });
        feedback.operation = Some("aa".repeat(16));
        state.feedback = Some(feedback);
        state.snapshot = Some(overview.current.clone());
        state.handle_reply(Reply::Overview(Box::new(overview)));
        (state, raw)
    }
    fn handoff_poll_reply(state: &Operator, raw: &serde_json::Value, activity: bool) -> Reply {
        if activity {
            Reply::Activity(crate::model::Activity {schema:crate::model::OPERATOR_SCHEMA,
                system:state.overview.as_ref().unwrap().current.system.clone(),
                capture:serde_json::Value::Null, operation:Some(raw.clone()),
                workspace_product_install_ready:false})
        } else {
            let mut pulse = pulse_fixture(false);
            pulse.operation = Some(raw.clone());
            Reply::Pulse(pulse)
        }
    }
    #[test]
    fn unchanged_raw_polls_preserve_verified_handoff_health_and_feedback() {
        for activity in [false, true] {
            let (mut state, raw) = handoff_poll_fixture();
            let original_raw = raw.clone();
            let projected = state.overview.as_ref().unwrap().current.operation.clone();
            for _ in 0..2 {
                state.handle_reply(handoff_poll_reply(&state, &raw, activity));
                assert!(state.overview_fresh && !state.refresh_after);
                assert!(state.current_health_system().unwrap().capacity_available());
                assert_eq!(state.overview.as_ref().unwrap().current.operation, projected);
                assert_eq!(state.snapshot.as_ref().unwrap().operation, projected);
                let feedback = state.feedback.as_ref().unwrap();
                assert_eq!(feedback.journey.as_ref().unwrap().phase, InstallationPhase::AwaitingApplication);
                assert_eq!(feedback.text, "The app is installed. Open it below to install your instruments and effects.");
            }
            assert_eq!(raw, original_raw, "raw history is unchanged");
            state.last_overview = Instant::now() - Duration::from_secs(61);
            state.expire_overview();
            state.handle_reply(handoff_poll_reply(&state, &raw, activity));
            assert!(state.current_health_system().unwrap().capacity_available());
            assert!(!state.overview_fresh, "polls cannot renew the overview");
            assert_eq!(state.current_action_snapshot().unwrap().state_token,
                state.overview.as_ref().unwrap().current.state_token);
        }
    }
    #[test]
    fn meaningful_receipt_changes_still_invalidate_projected_handoff() {
        for activity in [false, true] {
            for cause in ["owner", "state", "code", "stage", "retryable", "daw", "plugins", "reason", "malformed_failure"] {
                let (mut state, mut raw) = handoff_poll_fixture();
                match cause {
                    "owner" => raw["operation"] = "cc".repeat(16).into(),
                    "state" => raw["state"] = "completed".into(),
                    "code" => raw["journey"]["failure"]["code"] = "scanner_failed".into(),
                    "stage" => raw["journey"]["failure"]["stage"] = "connection".into(),
                    "retryable" => raw["journey"]["failure"]["retryable"] = false.into(),
                    "daw" => raw["journey"]["daw"] = "bitwig".into(),
                    "plugins" => raw["journey"]["plugins"] = serde_json::json!([{"class_id":"changed"}]),
                    "reason" => raw["reason"] = "changed_failure".into(),
                    "malformed_failure" => raw["journey"]["failure"]["unknown"] = true.into(),
                    _ => unreachable!(),
                }
                state.handle_reply(handoff_poll_reply(&state, &raw, activity));
                assert!(!state.overview_fresh && state.refresh_after, "{activity}/{cause}");
                assert!(state.current_health_system().is_none(), "{activity}/{cause}");
                assert_eq!(state.overview.as_ref().unwrap().current.operation.as_ref(), Some(&raw));
            }
        }
        for cause in ["empty_owner", "unverified", "current_failure"] {
            let (mut state, mut raw) = handoff_poll_fixture();
            let current = state.overview.as_mut().unwrap().current.operation.as_mut().unwrap();
            match cause {
                "empty_owner" => {current["operation"]="".into(); raw["operation"]="".into();}
                "unverified" => current["journey"]["phase"]="needs_attention".into(),
                "current_failure" => current["journey"]["failure"]=raw["journey"]["failure"].clone(),
                _ => unreachable!(),
            }
            let mut incoming=Some(raw.clone());
            retain_projected_handoff(Some(current), &mut incoming);
            assert_eq!(incoming, Some(raw), "{cause}");
        }
    }
    #[test]
    fn real_posture_generation_and_read_errors_invalidate_unchanged_handoff() {
        for cause in ["generation", "cleanup", "owners", "service", "read_failure", "superseded"] {
            let (mut state, raw) = handoff_poll_fixture();
            state.handle_reply(handoff_poll_reply(&state, &raw, false));
            let mut pulse=pulse_fixture(false);
            pulse.operation=Some(raw);
            match cause {
                "generation" => pulse.current_generation="changed".into(),
                "cleanup" => pulse.cleanup_unconfirmed=Some(true),
                "owners" => pulse.dsp=Some(1),
                "service" => pulse.service_state="inactive".into(),
                "read_failure" => {state.handle_reply(Reply::Error("Readback timed out".into()));}
                "superseded" => state.handle_reply(Reply::FrontendSuperseded),
                _ => unreachable!(),
            }
            if !matches!(cause, "read_failure" | "superseded") {state.handle_reply(Reply::Pulse(pulse));}
            assert!(state.current_health_system().is_none(), "{cause}");
            assert!(state.current_action_snapshot().is_none(), "{cause}");
        }
    }
    #[test]
    fn product_controls_and_header_use_the_same_capacity_capture() {
        let mut state = state_fixture();
        let overview = overview_fixture();
        let product = overview.current.products[0].clone();
        state.library.focus_product(ProductKey::from(&product));
        state.page = Page::Plugins;
        state.overview_fresh = true;
        state.overview = Some(overview.clone().into());
        let mut unavailable = overview.current.system.clone();
        unavailable.service = "capacity unavailable".into();
        state.origin = Some(RequestOrigin::ProductDetail);
        state.handle_reply(Reply::Product(Box::new(CurrentProductDetail {
            schema: 1, operator_schema: crate::model::OPERATOR_SCHEMA,
            state_token: overview.current.state_token.clone(),
            current_generation: overview.current_generation.clone(),
            system: unavailable.clone(), product, environments: vec![],
            vendor_applications: vec![],
        })));
        assert_eq!(state.current_health_system(), Some(&unavailable));
        assert_eq!(state.selected_product_snapshot().unwrap().system, unavailable);
        assert_eq!(presentation::health(state.current_health_system().unwrap()),
            presentation::Health::Unavailable);
        state.overview_fresh = false;
        assert!(state.current_health_system().is_none());
        assert!(state.current_action_snapshot().is_none());
        state.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        state.handle_reply(Reply::Overview(Box::new(overview.clone())));
        assert!(state.product_detail.is_none());
        assert_eq!(state.current_health_system(), Some(&overview.current.system));
    }
    #[test]
    fn suspended_service_does_not_repeatedly_disable_owned_installer_controls() {
        let mut operator = state_fixture();
        let mut overview = overview_fixture();
        overview.current = running_snapshot("current-op");
        overview.current.system.service = "capacity unavailable".into();
        let mut pulse = pulse_fixture(true);
        pulse.service_state = "inactive".into();
        pulse.cleanup_unconfirmed = None;
        operator.overview = Some(overview.clone().into());
        operator.overview_fresh = true;
        operator.origin = Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Pulse(pulse.clone()));
        assert!(!operator.overview_fresh);
        operator.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        operator.handle_reply(Reply::Overview(Box::new(overview)));
        for _ in 0..3 {
            operator.origin = Some(RequestOrigin::BackgroundActivity);
            operator.handle_reply(Reply::Pulse(pulse.clone()));
            assert!(operator.current_action_snapshot().is_some());
        }
        operator.pending = true;
        operator.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        assert!(!operator.controls_pending());
        operator.origin = Some(RequestOrigin::UserAction);
        assert!(operator.controls_pending());
        operator.pending = false;
        operator.origin = Some(RequestOrigin::BackgroundActivity);
        pulse.service_state = "active".into();
        operator.handle_reply(Reply::Pulse(pulse));
        assert!(operator.current_action_snapshot().is_none());
    }
    #[test]
    fn initial_overview_failure_recovers_after_successful_pulse() {
        let mut operator=state_fixture();
        operator.origin=Some(RequestOrigin::InitialSnapshot);
        operator.handle_reply(Reply::Error("temporary readback failure".into()));
        assert!(!operator.overview_fresh && operator.current_action_snapshot().is_none());
        assert!(operator.retry_after.is_some_and(|at|at>Instant::now()));
        operator.origin=Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Pulse(pulse_fixture(false)));
        assert!(operator.refresh_after);
        operator.origin=Some(RequestOrigin::SilentPostMutationRefresh);
        operator.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        assert!(operator.overview_fresh && operator.current_action_snapshot().is_some());
    }
    #[test]
    fn unchanged_pulse_recovers_stale_overview_without_resubmitting_action() {
        let mut operator=state_fixture();
        operator.overview=Some(overview_fixture().into());
        operator.overview_fresh=true;
        operator.origin=Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Error("temporary pulse failure".into()));
        assert!(!operator.overview_fresh && operator.current_action_snapshot().is_none());
        assert!(!operator.refresh_after);
        operator.origin=Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Pulse(pulse_fixture(false)));
        assert!(operator.refresh_after);
        assert!(operator.queued_action.is_none());
    }
    #[test]
    fn repeated_readback_errors_have_bounded_retry_and_no_action_authority() {
        let mut operator=state_fixture();
        operator.overview=Some(overview_fixture().into());
        operator.overview_fresh=true;
        for _ in 0..12 {
            operator.origin=Some(RequestOrigin::BackgroundActivity);
            operator.handle_reply(Reply::Error("still unavailable".into()));
            assert!(!operator.overview_fresh);
            assert!(operator.current_action_snapshot().is_none());
            assert!(!operator.prompt_pulse);
        }
        let remaining=operator.retry_after.unwrap().saturating_duration_since(Instant::now());
        assert!(remaining>=Duration::from_secs(30) && remaining<=Duration::from_secs(32));
    }
    #[test]
    fn successful_pulse_does_not_reset_failing_overview_backoff() {
        let mut operator=state_fixture();
        operator.overview=Some(overview_fixture().into());
        operator.overview_fresh=false;
        for expected in 1..=5 {
            operator.origin=Some(RequestOrigin::SilentPostMutationRefresh);
            operator.handle_reply(Reply::Error("overview unavailable".into()));
            let deadline=operator.retry_after.unwrap();
            assert_eq!(operator.readback_failures,expected);
            operator.origin=Some(RequestOrigin::BackgroundActivity);
            operator.handle_reply(Reply::Pulse(pulse_fixture(false)));
            assert!(operator.refresh_after);
            assert_eq!(operator.retry_after,Some(deadline));
            assert_eq!(operator.readback_failures,expected);
            assert!(operator.current_action_snapshot().is_none());
        }
        operator.origin=Some(RequestOrigin::SilentPostMutationRefresh);
        operator.handle_reply(Reply::Overview(Box::new(overview_fixture())));
        assert!(operator.overview_fresh);
        assert_eq!(operator.readback_failures,0);
        assert!(operator.retry_after.is_none());
    }
    #[test]
    fn accepted_receipt_prompts_pulse_then_terminal_state_returns_to_idle() {
        let mut operator=state_fixture();
        operator.origin=Some(RequestOrigin::UserAction);
        operator.handle_reply(Reply::Receipt(crate::model::Receipt {schema:crate::model::OPERATOR_SCHEMA,
            accepted:true,operation:Some("ab".repeat(16)),refusal:None}));
        assert!(operator.operation_live && operator.prompt_pulse && !operator.refresh_after);
        assert!(operator.repaint_delay()<=Duration::from_millis(50));
        operator.origin=Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Pulse(pulse_fixture(true)));
        assert!(operator.operation_live && !operator.prompt_pulse);
        assert!(operator.repaint_delay()<=Duration::from_secs(2));
        operator.origin=Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Pulse(pulse_fixture(false)));
        assert!(!operator.operation_live);
        assert!(operator.repaint_delay()>=Duration::from_secs(9));
    }
    #[test]
    fn unchanged_owners_with_new_cleanup_uncertainty_invalidate_overview() {
        let mut operator=state_fixture();
        operator.overview=Some(overview_fixture().into());
        operator.overview_fresh=true;
        let mut pulse=pulse_fixture(false);
        pulse.cleanup_unconfirmed=Some(true);
        operator.origin=Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Pulse(pulse));
        assert!(!operator.overview_fresh && operator.refresh_after);
        assert!(operator.current_action_snapshot().is_none());
    }
    #[test]
    fn current_overview_expires_without_claiming_permanent_ready() {
        let mut operator = state_fixture();
        operator.overview_fresh = true;
        operator.last_overview = Instant::now() - Duration::from_secs(61);
        operator.expire_overview();
        assert!(!operator.overview_fresh);
        assert!(operator.refresh_after);
    }
    #[test]
    fn idle_redraw_does_not_wake_the_frontend_twice_per_second() {
        let mut operator = state_fixture();
        assert!(operator.repaint_delay() >= Duration::from_secs(9));
        operator.operation_live = true;
        assert!(operator.repaint_delay() <= Duration::from_secs(2));
        operator.pending = true;
        operator.origin = Some(RequestOrigin::ExplicitRefresh);
        assert!(operator.repaint_delay() <= Duration::from_millis(500));
    }
    #[test]
    fn ten_background_polls_are_quiet_and_silent_refresh_keeps_import_notice() {
        let mut operator = state_fixture();
        let snapshot: Snapshot = serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap();
        operator.snapshot = Some(snapshot.clone());
        operator.message = "Added Lunacy Audio Installer. No installation has started.".into();
        for _ in 0..10 {
            operator.pending = true;
            operator.background_poll = true;
            operator.origin = Some(RequestOrigin::BackgroundActivity);
            operator.handle_reply(Reply::Activity(Activity {
                schema: crate::model::OPERATOR_SCHEMA, system: snapshot.system.clone(), capture: snapshot.capture.clone(),
                operation: snapshot.operation.clone(),
                workspace_product_install_ready: false,
            }));
            assert!(!operator.controls_pending());
            assert!(operator.feedback.is_none());
            assert!(operator.message.starts_with("Added Lunacy"));
            assert!(operator.origin.is_none());
        }
        operator.pending = true;
        operator.origin = Some(RequestOrigin::SilentPostMutationRefresh);
        operator.handle_reply(Reply::Snapshot(Box::new(snapshot.clone())));
        assert!(operator.message.starts_with("Added Lunacy"));
        operator.pending = true;
        operator.origin = Some(RequestOrigin::ExplicitRefresh);
        operator.handle_reply(Reply::Snapshot(Box::new(snapshot)));
        assert!(operator.message.starts_with("Added Lunacy"));
    }
    #[test]
    fn typed_import_focuses_exact_setup_and_duplicate_does_not_add_another_card() {
        let mut operator = state_fixture();
        let sha = "ab".repeat(32);
        let mut snapshot: Snapshot =
            serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap();
        snapshot.installer_setups.push(crate::model::InstallerSetup {
            installer: sha.clone(), name: "Lunacy Audio Installer".into(),
            label_source: "selected_basename".into(), byte_size: 1024,
            format: "pe_executable".into(), imported_at: 1,
            phase: crate::model::SetupPhase::Imported,
            status: "No native plug-in setup has started from this installer.".into(),
            environment: None, compatibility: None, discovered: vec![],
            primary: None, secondary: vec![],
            rename: AvailableAction { label: "Rename".into(),
                action: Action::InstallerRename { installer: sha.clone(), label: String::new() },
                disabled_reason: None }, journey:None,history: vec![],
        });
        let reply = crate::model::InstallerImportResult { schema: 2,
            installer_sha256: sha.clone(), byte_size: 1024, format: "pe_executable".into(),
            newly_imported: true, display_label: "Lunacy Audio Installer".into(), created_at: 1 };
        operator.pending = true;
        operator.origin = Some(RequestOrigin::InstallerImport);
        operator.handle_reply(Reply::Imported(reply.clone()));
        assert_eq!(operator.pending_import_route.as_ref().map(|x| x.0.as_str()), Some(sha.as_str()));
        operator.handle_reply(Reply::Snapshot(Box::new(snapshot.clone())));
        assert_eq!(operator.page, Page::Setup);
        assert_eq!(operator.focus.setup_installer.as_deref(), Some(sha.as_str()));
        assert!(operator.focus.setup_scroll);
        assert!(operator.message.contains("No native plug-in setup has started"));
        let mut duplicate = reply;
        duplicate.newly_imported = false;
        operator.pending = true;
        operator.origin = Some(RequestOrigin::InstallerImport);
        operator.handle_reply(Reply::Imported(duplicate));
        operator.handle_reply(Reply::Snapshot(Box::new(snapshot)));
        assert_eq!(operator.focus.setup_installer.as_deref(), Some(sha.as_str()));
        assert!(operator.message.contains("already added"));
    }
    fn wrong_installer_overview(environment: Option<&str>) -> CurrentOverview {
        let mut overview: CurrentOverview = overview_fixture().into();
        let wrong = "11".repeat(32);
        let right = "22".repeat(32);
        let mut original = automatic_setup(automatic_journey(InstallationPhase::NeedsAttention),None);
        original.installer = wrong.clone();original.name = "First-party updater fixture".into();
        original.environment = environment.map(str::to_owned);
        original.phase = if environment.is_some() {SetupPhase::SetupNeedsAttention} else {SetupPhase::Imported};
        original.history = vec![SetupHistoryRef {environment:original.environment.clone(),state:"cancelled".into()}];
        original.journey = None;original.discovered.clear();
        original.primary = Some(AvailableAction {label:"Retry".into(),disabled_reason:None,
            action:if let Some(environment) = environment {Action::InstallerNewAttempt {
                previous:environment.into(),runner:"same-runner".into()}}
                else {Action::SoftwareInstall {installer:wrong.clone(),daw:DawChoice::Other}}});
        let mut replacement = original.clone();
        replacement.installer = right.clone();replacement.name = "First-party installer fixture".into();
        replacement.environment = None;replacement.history.clear();replacement.phase = SetupPhase::Imported;
        replacement.primary = Some(AvailableAction {label:"Install and connect".into(),disabled_reason:None,
            action:Action::SoftwareInstall {installer:right.clone(),daw:DawChoice::Other}});
        overview.current.installer_setups = vec![original,replacement];
        overview.current.environment_installers = environment.into_iter().map(|environment|EnvironmentInstaller {
            environment:environment.into(),name:"Intended setup".into(),affected:vec!["Existing sibling".into()],
            consequence:"This installer can change this setup's vendor state. Stopping cannot undo those changes.".into(),
            runtime:Some(EnvironmentRuntime {runner:"same-runner".into(),revision:1,affected:vec![],
                consequence:String::new(),choices:vec![]}),operation:None,installer:None,result:serde_json::Value::Null,
            actions:vec![],choices:vec![AvailableAction {label:"Run selected installer".into(),disabled_reason:None,
                action:Action::EnvironmentInstallerStart {environment:environment.into(),installer:right.clone()}}]
        }).collect();
        overview
    }
    fn wrong_installer_text_position(shape: &egui::epaint::Shape, label: &str) -> Option<egui::Pos2> {
        match shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == label => Some(text.pos+text.galley.size()*0.5),
            egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|s|wrong_installer_text_position(s,label)),
            _ => None,
        }
    }
    fn wrong_installer_click(overview: &CurrentOverview, focus: &mut RouteFocus, label: &str,
        width: f32, fresh: bool) -> (bool,Option<Action>) {
        wrong_installer_click_with_pending(overview,focus,label,width,fresh,false)
    }
    fn wrong_installer_click_with_pending(overview: &CurrentOverview, focus: &mut RouteFocus, label: &str,
        width: f32, fresh: bool, pending: bool) -> (bool,Option<Action>) {
        let ctx = egui::Context::default();
        let mut position = egui::Pos2::ZERO;let mut pick = false;let mut chosen = None;
        for pressed in [None,Some(true),Some(false)] {
            let events = pressed.map(|pressed|vec![egui::Event::PointerMoved(position),
                egui::Event::PointerButton {pos:position,button:egui::PointerButton::Primary,
                    pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
            let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()},|ui| {
                ui.set_max_width(width-32.0);
                Operator::fast_setup(ui,overview,fresh,pending,(&mut false,&mut chosen,&mut pick,
                    &mut Page::Setup,&mut crate::library::Library::default(),focus,&mut Some(DawChoice::Bitwig)));
            });
            if pressed.is_none() {
                let found = output.shapes.iter().find_map(|shape|
                    wrong_installer_text_position(&shape.shape,label));
                output.textures_delta.clear();
                position = found.unwrap_or_else(||panic!("Missing {label}"));
            }
            output.textures_delta.clear();
        }
        (pick,chosen)
    }
    #[test]
    fn wrong_installer_ordinary_controls_keep_the_intended_setup_through_file_selection() {
        for (environment,phase) in [(None,SetupPhase::Imported),
            (Some("intended-environment"),SetupPhase::EnvironmentReady),
            (Some("intended-environment"),SetupPhase::SetupNeedsAttention),
            (Some("intended-environment"),SetupPhase::DiscoveryComplete)] {
            for width in [560.0,960.0] {
                let mut overview = wrong_installer_overview(environment);
                overview.current.installer_setups[0].phase = phase.clone();
                let wrong = overview.current.installer_setups[0].installer.clone();
                let right = overview.current.installer_setups[1].installer.clone();
                let original_retry = overview.current.installer_setups[0].primary.clone().unwrap().action;
                let mut operator = state_fixture();
                operator.focus.setup_installer = Some(wrong.clone());
                let (pick,chosen) = wrong_installer_click(&overview,&mut operator.focus,"Choose another installer",width,true);
                assert!(pick);assert!(chosen.is_none());
                operator.origin = Some(RequestOrigin::InstallerImport);
                operator.handle_reply(Reply::Imported(InstallerImportResult {schema:2,
                    installer_sha256:right.clone(),display_label:"First-party installer fixture".into(),
                    format:"pe_executable".into(),byte_size:1024,newly_imported:false,created_at:1}));
                operator.apply_current_overview(overview.clone());
                assert_eq!(operator.focus.setup_installer.as_deref(),Some(wrong.as_str()));
                assert!(operator.pending_import_route.is_none());
                assert!(operator.queued_action.is_none(),"Import cannot launch an installer");
                assert_eq!(overview.current.installer_setups[0].primary.as_ref().unwrap().action,original_retry);
                let label = if environment.is_some() {"Run installer in this setup"} else {"Install and connect to your DAW"};
                let (_,chosen) = wrong_installer_click(&overview,&mut operator.focus,label,width,true);
                assert_eq!(chosen,Some(if let Some(environment) = environment {Action::EnvironmentInstallerStart {
                    environment:environment.into(),installer:right}}
                    else {Action::SoftwareInstall {installer:right,daw:DawChoice::Other}}));
                assert!(operator.focus.replacement.is_none());
            }
        }
    }
    #[test]
    fn wrong_installer_choice_refuses_changed_setup_same_bytes_and_other_working_setup() {
        let mut overview = wrong_installer_overview(Some("intended-environment"));
        let mut draft = replacement_context(&overview.current,&overview.current.installer_setups[0]).unwrap();
        draft.selected = Some(overview.current.installer_setups[1].installer.clone());
        assert!(replacement_offer(&overview.current,&draft).is_ok());
        overview.current.environment_installers[0].runtime.as_mut().unwrap().revision += 1;
        assert!(replacement_offer(&overview.current,&draft).unwrap_err().contains("setup changed"));
        overview.current.environment_installers[0].runtime.as_mut().unwrap().revision -= 1;
        overview.current.installer_setups[0].environment = Some("another-working-setup".into());
        assert!(replacement_offer(&overview.current,&draft).is_err());
        let mut overview = wrong_installer_overview(None);
        let mut draft = replacement_context(&overview.current,&overview.current.installer_setups[0]).unwrap();
        draft.selected = Some(draft.original.clone());
        assert!(replacement_offer(&overview.current,&draft).unwrap_err().contains("same installer"));
        draft.selected = Some(overview.current.installer_setups[1].installer.clone());
        overview.current.installer_setups[1].environment = Some("another-working-setup".into());
        assert!(replacement_offer(&overview.current,&draft).is_err());
    }
    #[test]
    fn wrong_installer_choice_waits_for_retirement_and_preserves_vendor_app_continuation() {
        let mut overview = wrong_installer_overview(Some("intended-environment"));
        let mut draft = replacement_context(&overview.current,&overview.current.installer_setups[0]).unwrap();
        draft.selected = Some(overview.current.installer_setups[1].installer.clone());
        overview.current.environment_installers[0].operation = Some("exact-replacement-owner".into());
        overview.current.environment_installers[0].result = serde_json::json!({"state":"cancelled","owned_live":null,"cleanup_confirmed":false});
        assert!(replacement_offer(&overview.current,&draft).is_err());
        overview.current.environment_installers[0].result = serde_json::json!({"state":"cancelled","owned_live":0,"cleanup_confirmed":true});
        overview.current.environment_installers[0].actions = vec![AvailableAction {label:"Stop installer".into(),disabled_reason:None,
            action:Action::EnvironmentInstallerStop {operation:"exact-replacement-owner".into()}}];
        assert!(replacement_offer(&overview.current,&draft).is_err(),"A live supervisor still owns the operation");
        overview.current.environment_installers[0].operation = None;
        overview.current.environment_installers[0].actions.clear();
        let open = Action::VendorSoftwareInstall {application:"first-party-launcher".into(),daw:DawChoice::Bitwig};
        let setup = &mut overview.current.installer_setups[0];
        setup.journey = Some(InstallationJourney {phase:InstallationPhase::AwaitingApplication,
            daw:DawChoice::Bitwig,plugins:vec![],failure:None});
        setup.primary = Some(AvailableAction {label:"Open the installed app".into(),action:open.clone(),disabled_reason:None});
        assert!(replacement_offer(&overview.current,&draft).is_ok());
        let mut focus = RouteFocus {setup_installer:Some(draft.original),..Default::default()};
        assert_eq!(wrong_installer_click(&overview,&mut focus,"Open the installed app",960.0,true).1,Some(open));
        let (_,chosen) = wrong_installer_click(&overview,&mut focus,"Choose another installer",560.0,false);
        assert!(chosen.is_none());assert!(focus.replacement.is_none(),"Stale controls cannot start file selection");
    }
    #[test]
    fn wrong_installer_picker_cancel_keeps_the_original_setup_and_retry() {
        let overview = wrong_installer_overview(Some("intended-environment"));
        let mut operator = state_fixture();
        let original = &overview.current.installer_setups[0];
        operator.focus.setup_installer = Some(original.installer.clone());
        operator.focus.replacement = replacement_context(&overview.current,original);
        operator.origin = Some(RequestOrigin::InstallerImport);
        operator.handle_reply(Reply::Cancelled);
        assert!(operator.focus.replacement.is_none());
        assert_eq!(operator.focus.setup_installer.as_ref(),Some(&original.installer));
        assert!(operator.queued_action.is_none());
        let mut overview = overview;
        overview.current.installer_setups[0].phase = SetupPhase::CleanupUnconfirmed;
        assert!(replacement_context(&overview.current,&overview.current.installer_setups[0]).is_none());
    }
    #[test]
    fn completed_replacement_preserves_current_connection_and_recovery_offers() {
        for action in [Action::EnvironmentRefresh {environment:"intended-environment".into(),daw:DawChoice::Bitwig},
            Action::EnvironmentRefreshResume {operation:"connection-owner".into()},
            Action::SoftwareInstallResume {onboarding:"intended-environment".into()}] {
            for secondary in [false,true] {
                let mut overview=wrong_installer_overview(Some("intended-environment"));
                let target=&mut overview.current.environment_installers[0];
                target.operation=Some("retired-replacement".into());
                target.result=serde_json::json!({"state":"completed","cleanup_confirmed":true,"owned_live":0});
                let offer=AvailableAction {label:"Connect installed plug-ins".into(),action:action.clone(),disabled_reason:None};
                let label=presentation::action_label(&offer);
                let setup=&mut overview.current.installer_setups[0];
                if secondary {setup.secondary=vec![offer];} else {setup.primary=Some(offer);}
                let mut focus=RouteFocus {setup_installer:Some(setup.installer.clone()),..Default::default()};
                assert_eq!(wrong_installer_click(&overview,&mut focus,&label,960.0,true).1,Some(action.clone()));
            }
        }
    }
    #[test]
    fn wrong_installer_picker_owner_cannot_be_cleared_while_import_is_pending() {
        let overview = wrong_installer_overview(Some("intended-environment"));
        let original = &overview.current.installer_setups[0];
        let mut focus = RouteFocus {setup_installer:Some(original.installer.clone()),
            replacement:replacement_context(&overview.current,original),..Default::default()};
        wrong_installer_click_with_pending(&overview,&mut focus,"Keep current installer",560.0,true,true);
        assert!(focus.replacement.is_some());
        wrong_installer_click_with_pending(&overview,&mut focus,"Keep current installer",560.0,true,false);
        assert!(focus.replacement.is_none());
    }
    #[test]
    fn duplicate_historical_fl_application_installer_routes_to_workspaces_not_empty_setup() {
        let mut operator = state_fixture();
        let sha = "ab".repeat(32);
        let snapshot: Snapshot = serde_json::from_value(serde_json::json!({
            "schema":crate::model::OPERATOR_SCHEMA,"state_token":"current",
            "system":{"service":"active","keepers":0,"dsp":0,"maintenance":0,
                "ceiling":6,"pending_transactions":0,"stale_transports":0,
                "cleanup_unconfirmed":false},
            "onboarding":[],"installer_setups":[],"environments":[],
            "vendor_applications":[],"products":[],"active_sessions":[],
            "capture":null,"recent_incidents":[],"actions":[],"operation":null,
            "workspaces":[{"id":"cd".repeat(16),"name":"FL Studio","state":"installed",
                "selected_installer":"ef".repeat(32),
                "application_installers":[sha.clone(),"ef".repeat(32)],
                "selected_release":"27.0.0.0","installed_advertised_release":"27.0.0.0",
                "observed_file_version":null,"installed_image_sha256":null,
                "active_installation_operation":null,"cleanup":"confirmed",
                "first_useful_failure":null,"actions":[],"installer_choices":[],
                "products":[],"details":null}]
        })).unwrap();
        operator.pending = true;
        operator.origin = Some(RequestOrigin::InstallerImport);
        operator.handle_reply(Reply::Imported(crate::model::InstallerImportResult {
            schema: 2, installer_sha256: sha, byte_size: 1024,
            format: "pe_executable".into(), newly_imported: false,
            display_label: "FL Studio Installer".into(), created_at: 1,
        }));
        operator.handle_reply(Reply::Snapshot(Box::new(snapshot)));
        assert_eq!(operator.page, Page::Workspaces);
        assert!(operator.focus.setup_installer.is_none());
        assert!(operator.message.contains("FL Studio"));
        assert!(operator.pending_import_route.is_none());
    }
    #[test]
    fn fast_setup_keeps_import_focus_and_routes_exact_discoveries() {
        fn labels(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes { labels(shape, out); }
                }
                _ => {}
            }
        }
        let mut overview = overview_fixture();
        let exact = overview.current.products[0].clone();
        let mut other_build = exact.clone();
        other_build.name = "Other installed build".into();
        other_build.module_sha256 = "ef".repeat(32);
        overview.current.products = vec![exact.clone(), other_build];
        let setup = InstallerSetup {
            installer:"ab".repeat(32),name:"Selected installer".into(),
            label_source:"operator_named".into(),byte_size:1024,
            format:"pe_executable".into(),imported_at:1,
            phase:crate::model::SetupPhase::DiscoveryComplete,
            status:"One exact plug-in found".into(),environment:Some(exact.environment.clone()),
            compatibility:None,
            discovered:vec![crate::model::DiscoveredProduct {
                environment:exact.environment.clone(),module_sha256:exact.module_sha256.clone(),
                class_id:exact.class_id.clone(),name:exact.name.clone(),
            }],
            primary:None,secondary:vec![],
            rename:AvailableAction {label:"Rename".into(),
                action:Action::InstallerRename {installer:"ab".repeat(32),label:String::new()},
                disabled_reason:None},journey:None,history:vec![],
        };
        let mut unrelated = setup.clone();
        unrelated.installer = "cd".repeat(32);
        unrelated.name = "Unrelated installer".into();
        overview.current.installer_setups = vec![setup.clone(), unrelated];
        for width in [960.0, 560.0] {
            let ctx = egui::Context::default();
            let mut page = Page::Setup;
            let mut library = crate::library::Library::default();
            let mut focus = RouteFocus {setup_installer:Some(setup.installer.clone()),
                setup_scroll:true,..Default::default()};
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width-32.0);
                Operator::fast_setup(ui,&overview.clone().into(),true,false,
                    (&mut false,&mut None,&mut false,&mut page,&mut library,&mut focus,&mut None));
            });
            let mut text = Vec::new();
            for clipped in &output.shapes { labels(&clipped.shape,&mut text); }
            output.textures_delta.clear();
            assert!(text.iter().any(|line| line == "Selected installer"));
            assert!(!text.iter().any(|line| line == "Unrelated installer"));
            assert!(text.iter().any(|line| line == "View discovered plug-ins"));
            assert!(text.iter().any(|line| line == "Show all setup"));
            assert!(!focus.setup_scroll);
            show_discovered_products(&setup,&mut library,&mut page);
            assert_eq!(page,Page::Plugins);
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                library.show_current(ui,&overview.current,true,false,&mut false);
            });
            let mut text = Vec::new();
            for clipped in &output.shapes { labels(&clipped.shape,&mut text); }
            output.textures_delta.clear();
            assert!(text.iter().any(|line| line == "1 of 2 current plug-ins"));
            assert!(text.iter().any(|line| line == &exact.name));
            assert!(!text.iter().any(|line| line == "Other installed build"));
            assert!(text.iter().any(|line| line.contains("setup and settings")));
        }
    }
    #[test]
    fn removal_and_deletion_ask_first_with_the_exact_current_offer() {
        fn labels(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(shapes) => for shape in shapes { labels(shape, out); },
                _ => {}
            }
        }
        let mut overview = overview_fixture();
        let product = overview.current.products[0].clone();
        let delete = AvailableAction {label:"Delete this setup".into(),
            action:Action::SetupDelete {installer:"ab".repeat(32),environment:product.environment.clone(),confirmed:false,forget_installer:false},
            disabled_reason:None};
        let remove_all = AvailableAction {label:"Remove its plug-ins from my DAW".into(),
            action:Action::SetupRemoveFromDaw {environment:product.environment.clone()},disabled_reason:None};
        let setup = InstallerSetup {
            installer:"ab".repeat(32),name:"Freq Echo installer".into(),
            label_source:"operator_named".into(),byte_size:1024,format:"pe_executable".into(),imported_at:1,
            phase:crate::model::SetupPhase::DiscoveryComplete,status:"One plug-in found".into(),
            environment:Some(product.environment.clone()),compatibility:None,discovered:vec![],
            primary:None,secondary:vec![remove_all.clone(),delete.clone()],
            rename:AvailableAction {label:"Rename".into(),
                action:Action::InstallerRename {installer:"ab".repeat(32),label:String::new()},
                disabled_reason:None},journey:None,history:vec![],
        };
        overview.current.installer_setups = vec![setup.clone()];
        assert!(needs_confirmation(&delete.action) && needs_confirmation(&remove_all.action));
        assert!(!needs_confirmation(&Action::PluginAddBackToDaw {class_id:product.class_id.clone(),
            expected_removed:PublicationIdentity {id:"ab".repeat(16),sha256:"cd".repeat(32)}}));

        // A plug-in of an earlier attempt of the same setup is named too.
        let mut earlier = product.clone();
        earlier.name = "Earlier attempt plug-in".into();
        earlier.environment = "ee".repeat(16);
        overview.current.products.push(earlier);
        overview.current.installer_setups[0].history = vec![crate::model::SetupHistoryRef {
            environment: Some("ee".repeat(16)), state: "installed".into() }];
        let asked = removal_confirmation(Some(&overview.current), &delete.action);
        assert!(asked.body.iter().any(|line| line.contains(&product.name) && line.contains("Earlier attempt plug-in")),
            "{:?}", asked.body);
        overview.current.products.pop();
        overview.current.installer_setups[0].history.clear();
        let asked = removal_confirmation(Some(&overview.current), &delete.action);
        assert_eq!(asked.title, "Delete this setup?");
        assert_eq!(asked.body[0], "This deletes the Windows setup for Freq Echo installer: everything installed in it, and any licences or sign-ins stored there.");
        assert!(asked.body.iter().any(|line| line.contains("presets")), "{:?}", asked.body);
        assert!(asked.body.iter().any(|line| line.contains("deactivate the licence")), "{:?}", asked.body);
        assert!(asked.body.iter().any(|line| line.contains(&product.name) && line.contains("Add back won't be possible")),
            "{:?}", asked.body);
        // Only the musician's confirmation is sent; the offer itself stays unconfirmed.
        assert_eq!(confirmed(delete.action.clone()), Action::SetupDelete {installer:"ab".repeat(32),
            environment:product.environment.clone(),confirmed:true,forget_installer:false});
        assert_eq!(offer_identity(&confirmed(delete.action.clone())), delete.action);
        assert_eq!(confirmed(remove_all.action.clone()), remove_all.action);
        assert_eq!(asked.offer.as_ref().map(|offer| &offer.action), Ok(&delete.action));
        overview.current.installer_setups[0].secondary[1].disabled_reason = Some("Remove Freq Echo from your DAW first.".into());
        assert_eq!(removal_confirmation(Some(&overview.current), &delete.action).offer.unwrap_err(),
            "Remove Freq Echo from your DAW first.");
        let mut other = delete.action.clone();
        if let Action::SetupDelete {installer,..} = &mut other { *installer = "ef".repeat(32); }
        assert!(removal_confirmation(Some(&overview.current), &other).offer.unwrap_err().contains("no longer offered"));
        assert!(removal_confirmation(None, &delete.action).offer.is_err());

        // Uninstall names what leaves the DAW and runs only once confirmed.
        let uninstall = AvailableAction {label:"Uninstall ValhallaFreqEcho".into(),
            action:Action::ProductUninstall {environment:product.environment.clone(),registration:"ab".repeat(32),
                command_sha256:"cd".repeat(32),confirmed:false},disabled_reason:None};
        overview.current.uninstallers = vec![crate::model::Uninstaller {environment:product.environment.clone(),
            name:"ValhallaFreqEcho".into(),affected:vec![product.name.clone()],
            affected_classes:vec![product.class_id.clone()],coverage:"installed_with_this".into(),
            action:uninstall.clone()}];
        assert!(needs_confirmation(&uninstall.action));
        let asked = removal_confirmation(Some(&overview.current), &uninstall.action);
        assert_eq!(asked.title, "Uninstall?");
        assert!(asked.body.iter().any(|line| line.contains(&product.name) && line.contains("removed from your DAW")),
            "{:?}", asked.body);
        assert_eq!(asked.offer.as_ref().map(|offer| &offer.action), Ok(&uninstall.action));
        let sent = confirmed(uninstall.action.clone());
        assert!(matches!(sent, Action::ProductUninstall {confirmed:true,..}));
        assert_eq!(offer_identity(&sent), uninstall.action);
        assert_eq!(removal_confirmation(Some(&overview.current), &sent).offer.as_ref().map(|o| &o.action),
            Ok(&uninstall.action), "the confirmed request still finds its exact offer");

        // The setup card keeps both choices out of its everyday row, in their own section.
        overview.current.installer_setups[0].secondary[1].disabled_reason = None;
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            Operator::fast_setup(ui,&overview.clone().into(),true,false,
                (&mut false,&mut None,&mut false,&mut Page::Setup,&mut crate::library::Library::default(),
                    &mut RouteFocus::default(),&mut None));
        });
        let mut text = Vec::new();
        for clipped in &output.shapes { labels(&clipped.shape,&mut text); }
        output.textures_delta.clear();
        assert!(text.iter().any(|line| line == "Uninstall, remove or delete"));
        assert!(!text.iter().any(|line| line == "Delete this setup"), "collapsed until the musician opens it");

        // A finished installation reads as what is in the DAW now.
        let mut removed = product.clone();
        removed.disposition = "removed_from_daw".into();
        removed.availability = Some(NativeAvailability {available:false,reason:Some("Removed".into())});
        overview.current.products = vec![removed];
        let journey = InstallationJourney {daw:DawChoice::Reaper,phase:InstallationPhase::Available,
            plugins:vec![InstallationPlugin {selection:"selection".into(),class_id:product.class_id.clone(),
                name:product.name.clone(),stage:InstallationPluginStage::Available,failure:None}],failure:None};
        let current = Some((&overview.current, product.environment.as_str()));
        assert_eq!(current_journey_label(&journey, current), "Not in your DAW");
        assert_eq!(current_plugin_state(&journey, &journey.plugins[0], current), Some("Removed from your DAW"));
        assert_eq!(current_journey_label(&journey, None), "DAW connection complete");
    }
    #[test]
    fn selected_setup_stop_is_visible_and_clickable_without_capacity_readiness() {
        fn stop_position(shape: &egui::epaint::Shape) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == "Stop installer" =>
                    Some(text.pos + text.galley.size() * 0.5),
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(stop_position),
                _ => None,
            }
        }
        for fresh in [true, false] {
            let mut overview = overview_fixture();
            let running = running_snapshot("11");
            overview.current.system = running.system;
            let target = running.onboarding[0].actions[0].clone();
            let setup = InstallerSetup {
                installer:"bb".repeat(32),name:"Held installer".into(),
                label_source:"imported_filename".into(),byte_size:1024,
                format:"pe_executable".into(),imported_at:1,
                phase:crate::model::SetupPhase::InstallerRunning,
                status:"Complete the vendor installer or stop this installation.".into(),
                environment:Some("aa".repeat(16)),compatibility:None,discovered:vec![],
                primary:Some(AvailableAction {label:"Focus installer".into(),
                    action:Action::InstallerFocus {onboarding:"aa".repeat(16),operation:"11".into()},disabled_reason:None}),
                secondary:vec![target.clone()],rename:AvailableAction {label:"Rename".into(),
                    action:Action::InstallerRename {installer:"bb".repeat(32),label:String::new()},disabled_reason:None},journey:None,history:vec![],
            };
            overview.current.installer_setups = vec![setup.clone()];
            let ctx = egui::Context::default();
            let mut focus = RouteFocus {setup_installer:Some(setup.installer),..Default::default()};
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None, Some(true), Some(false)] {
                let events = pressed.map(|pressed| vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,
                        pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output = ctx.run_ui(egui::RawInput {events,..Default::default()}, |ui| {
                    ui.set_max_width(1200.0);
                    // Reserve the real application's header, health, navigation
                    // and request area. Recovery must fit a normal 800px screen.
                    ui.add_space(410.0);
                    Operator::fast_setup(ui,&overview.clone().into(),fresh,false,
                        (&mut false,&mut chosen,&mut false,&mut Page::Setup,
                            &mut crate::library::Library::default(),&mut focus,&mut None));
                });
                if pressed.is_none() {
                    point = output.shapes.iter().find_map(|shape|stop_position(&shape.shape)).unwrap();
                    assert!(point.y < 800.0,"Stop below the ordinary viewport: {point:?}");
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen, fresh.then_some(target.action));
        }
    }
    #[test]
    fn fresh_import_advanced_policy_choice_keeps_the_ordinary_daw_request_before_creation() {
        for width in [560.0,960.0] {
            let mut overview=wrong_installer_overview(None);overview.current.onboarding.clear();
            overview.current.installer_setups.truncate(1);
            let setup=&mut overview.current.installer_setups[0];setup.history.clear();
            let expected=Action::SoftwareInstallWithPolicy {installer:setup.installer.clone(),daw:DawChoice::Other,
                powershell:crate::model::Powershell::Inherited};
            setup.secondary=vec![AvailableAction {label:"Install with normal PowerShell behavior".into(),
                action:expected.clone(),disabled_reason:None}];
            assert!(setup.environment.is_none());assert!(setup.history.is_empty());
            let ctx=egui::Context::default();let mut chosen=None;let mut time=0.0;
            let mut focus=RouteFocus {setup_installer:Some(setup.installer.clone()),..Default::default()};
            for label in ["Setup details and options","Install with normal PowerShell behavior"] {
                let mut point=egui::Pos2::ZERO;
                for pressed in [None,Some(true),Some(false)] {
                    time+=0.3;
                    let events=pressed.map(|pressed|vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                        pos:point,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                    let mut output=ctx.run_ui(egui::RawInput {events,time:Some(time),..Default::default()},|ui| {
                        ui.set_max_width(width-32.0);
                        Operator::fast_setup(ui,&overview,true,false,(&mut false,&mut chosen,&mut false,
                            &mut Page::Setup,&mut crate::library::Library::default(),&mut focus,&mut None));
                    });
                    if pressed.is_none() {point=output.shapes.iter().find_map(|shape|
                        wrong_installer_text_position(&shape.shape,label)).expect("visible fresh import policy choice");}
                    output.textures_delta.clear();
                }
            }
            assert_eq!(chosen,Some(expected));
            let selected=installation_with_daw(&chosen.unwrap(),DawChoice::Reaper).unwrap();
            assert_eq!(installation_form_offer(Some(&overview.current),&selected),Ok(None));
            let wire=serde_json::to_vec(&Request {schema:crate::model::OPERATOR_SCHEMA,
                state_token:overview.current.state_token.clone(),action:selected}).unwrap();
            let manager:linux_vst_bridge::operator_model::Request=serde_json::from_slice(&wire).unwrap();
            let (_,daw,policy)=manager.action.software_install().unwrap();
            assert_eq!(daw,linux_vst_bridge::operator_model::DawChoice::Reaper);
            assert_eq!(policy,Some(linux_vst_bridge::operator_model::Powershell::Inherited));
            assert!(overview.current.installer_setups[0].environment.is_none());
        }
    }
    #[test]
    fn advanced_default_initial_installer_policy_is_clickable_in_setup_options() {
        for width in [560.0,960.0] {
            let mut attempt=running_snapshot("unused").onboarding.remove(0);
            let expected=Action::InstallerStartWithPolicy {onboarding:attempt.environment.clone().unwrap(),
                powershell:crate::model::Powershell::Inherited};
            attempt.state="environment_ready".into();attempt.details=serde_json::Value::Null;
            attempt.actions=vec![AvailableAction {label:"Run installer with normal PowerShell behavior".into(),
                action:expected.clone(),disabled_reason:None}];
            let mut setup=automatic_setup(automatic_journey(InstallationPhase::NeedsAttention),None);
            setup.installer=attempt.installer.clone();setup.environment=attempt.environment.clone();
            setup.journey=None;setup.phase=SetupPhase::EnvironmentReady;
            let mut overview=overview_fixture();overview.current.onboarding=vec![attempt];
            overview.current.installer_setups=vec![setup.clone()];overview.current.environment_installers.clear();
            let ctx=egui::Context::default();let mut chosen=None;let mut time=0.0;
            let mut focus=RouteFocus {setup_installer:Some(setup.installer),..Default::default()};
            for label in ["Setup details and options","Run installer with normal PowerShell behavior"] {
                let mut point=egui::Pos2::ZERO;
                for pressed in [None,Some(true),Some(false)] {
                    time+=0.3;
                    let events=pressed.map(|pressed|vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                        pos:point,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                    let mut output=ctx.run_ui(egui::RawInput {events,time:Some(time),..Default::default()},|ui| {
                        ui.set_max_width(width-32.0);
                        Operator::fast_setup(ui,&overview.clone().into(),true,false,(&mut false,&mut chosen,&mut false,
                            &mut Page::Setup,&mut crate::library::Library::default(),&mut focus,&mut None));
                    });
                    if pressed.is_none() {point=output.shapes.iter().find_map(|shape|
                        wrong_installer_text_position(&shape.shape,label)).expect("visible initial policy control");}
                    output.textures_delta.clear();
                }
            }
            assert_eq!(chosen,Some(expected));
        }
    }
    #[test]
    fn ordinary_card_and_chooser_retry_submit_the_offered_choice_after_reopen() {
        for (powershell,matching_card) in [crate::model::Powershell::Inherited,crate::model::Powershell::IntentionallyUnavailable]
            .into_iter().flat_map(|policy|[true,false].map(move |card|(policy,card))) {
            let environment="ab".repeat(16);let installer="cd".repeat(32);
            let offered=Action::environment_installer_action(environment.clone(),installer.clone(),Some(powershell));
            let mut overview:CurrentOverview=overview_fixture().into();
            overview.current.onboarding.clear();overview.current.installer_setups.clear();
            if matching_card {
                let mut setup=automatic_setup(automatic_journey(InstallationPhase::NeedsAttention),None);
                setup.installer="11".repeat(32);setup.environment=Some(environment.clone());
                setup.journey=None;setup.phase=SetupPhase::SetupNeedsAttention;
                overview.current.installer_setups.push(setup);
            }
            overview.current.environment_installers=vec![crate::model::EnvironmentInstaller {
                environment:environment.clone(),runtime:None,name:"Synth setup".into(),affected:vec![],
                consequence:"The installer can change this setup.".into(),operation:Some("ef".repeat(16)),
                installer:Some(installer.clone()),result:serde_json::json!({"state":"cancelled"}),
                choices:vec![AvailableAction {label:"Run chosen installer".into(),
                    action:Action::environment_installer_action(environment.clone(),installer,None),disabled_reason:None}],
                actions:vec![AvailableAction {label:"Retry".into(),action:offered.clone(),disabled_reason:None}]}];
            let wire=serde_json::to_vec(&overview).unwrap();
            let overview:CurrentOverview=serde_json::from_slice(&wire).unwrap();
            let ctx=egui::Context::default();
            if !matching_card {ctx.data_mut(|data|data.insert_temp(egui::Id::new("companion_environment"),environment));}
            let mut chosen=None;let mut point=egui::Pos2::ZERO;let mut time=0.0;
            for pressed in [None,Some(true),Some(false)] {
                time+=0.3;
                let events=pressed.map(|pressed|vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                    pos:point,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output=ctx.run_ui(egui::RawInput {events,time:Some(time),..Default::default()},|ui| {
                    ui.set_max_width(780.0);
                    Operator::fast_setup(ui,&overview,true,false,(&mut false,&mut chosen,&mut false,
                        &mut Page::Setup,&mut crate::library::Library::default(),&mut RouteFocus::default(),&mut None));
                });
                if pressed.is_none() {point=output.shapes.iter().find_map(|shape|
                    wrong_installer_text_position(&shape.shape,"Retry")).expect("ordinary visible Retry after reopen");}
                output.textures_delta.clear();
            }
            assert_eq!(chosen,Some(offered));
            let request=Request {schema:crate::model::OPERATOR_SCHEMA,state_token:overview.current.state_token.clone(),action:chosen.unwrap()};
            let accepted:linux_vst_bridge::operator_model::Request=serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
            assert_eq!(serde_json::to_value(accepted.action.environment_installer_start().unwrap().2).unwrap(),
                serde_json::to_value(Some(powershell)).unwrap());
        }
    }
    #[test]
    fn advanced_default_installer_policy_is_reachable_after_retry_without_reselecting_file() {
        fn position(shape:&egui::epaint::Shape,label:&str)->Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(text) if text.galley.text()==label => Some(text.pos+text.galley.size()*0.5),
                egui::epaint::Shape::Vec(shapes)=>shapes.iter().find_map(|shape|position(shape,label)),
                _=>None,
            }
        }
        let environment="ab".repeat(16);let installer="cd".repeat(32);
        let expected=Action::EnvironmentInstallerStartWithPolicy { environment:environment.clone(),installer:installer.clone(),
            powershell:crate::model::Powershell::Inherited };
        let mut overview=overview_fixture();
        overview.current.environment_installers=vec![crate::model::EnvironmentInstaller {
            environment:environment.clone(),runtime:None,name:"Synth setup".into(),affected:vec![],
            consequence:"The installer can change this setup.".into(),operation:Some("ef".repeat(16)),installer:Some(installer.clone()),
            result:serde_json::json!({"state":"cancelled"}),choices:vec![AvailableAction {label:"Run chosen installer".into(),
                action:Action::EnvironmentInstallerStart {environment:environment.clone(),installer},disabled_reason:None}],
            actions:vec![AvailableAction {label:"Run with normal PowerShell behavior".into(),action:expected.clone(),disabled_reason:None}]}];
        let ctx=egui::Context::default();
        ctx.data_mut(|data|data.insert_temp(egui::Id::new("companion_environment"),environment));
        let mut chosen=None;let mut time=0.0;
        for label in ["Advanced installer options","Run with normal PowerShell behavior"] {
            let mut point=egui::Pos2::ZERO;
            for pressed in [None,Some(true),Some(false)] {
                time+=0.3;
                let events=pressed.map(|pressed|vec![egui::Event::PointerMoved(point),egui::Event::PointerButton {
                    pos:point,button:egui::PointerButton::Primary,pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output=ctx.run_ui(egui::RawInput {events,time:Some(time),..Default::default()},|ui| {
                    ui.set_max_width(780.0);
                    Operator::fast_setup(ui,&overview.clone().into(),true,false,(&mut false,&mut chosen,&mut false,
                        &mut Page::Setup,&mut crate::library::Library::default(),&mut RouteFocus::default(),&mut None));
                });
                if pressed.is_none() { point=output.shapes.iter().find_map(|shape|position(&shape.shape,label)).expect("visible advanced installer control"); }
                output.textures_delta.clear();
            }
        }
        assert_eq!(chosen,Some(expected));
    }
    #[test]
    fn companion_choice_shows_scope_before_run_and_exact_stop_survives_suspension() {
        fn text_position(shape:&egui::epaint::Shape,label:&str)->Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(text) if text.galley.text()==label =>
                    Some(text.pos+text.galley.size()*0.5),
                egui::epaint::Shape::Vec(shapes)=>shapes.iter().rev().find_map(|shape|text_position(shape,label)),
                _=>None,
            }
        }
        let environment="ab".repeat(16);let installer="cd".repeat(32);let operation="ef".repeat(16);
        let run=Action::EnvironmentInstallerStart {environment:environment.clone(),installer:installer.clone()};
        let stop=Action::EnvironmentInstallerStop {operation:operation.clone()};
        for (width,running) in [(960.0,false),(560.0,false),(960.0,true),(560.0,true)] {
            let ctx=egui::Context::default();
            let target=crate::model::EnvironmentInstaller {environment:environment.clone(),runtime:None,
                name:"Synth setup".into(),affected:vec!["Synth".into(),"Effect sibling".into()],
                consequence:"Stopping cannot undo vendor changes. Both plug-ins need fresh preparation.".into(),
                choices:vec![AvailableAction {label:"Run companion in this setup".into(),action:run.clone(),
                    disabled_reason:running.then(||"Installer is running".into())}],
                operation:running.then(||operation.clone()),installer:running.then(||installer.clone()),
                result:if running {serde_json::json!({"operation":operation,"state":"running"})} else {serde_json::Value::Null},
                actions:if running {vec![AvailableAction {label:"Stop installer".into(),action:stop.clone(),disabled_reason:None}]} else {vec![]}};
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("companion_environment"),environment.clone());
                data.insert_temp(egui::Id::new(("companion_installer",&environment)),installer.clone());
            });
            let mut overview=overview_fixture();overview.current.environment_installers=vec![target];
            if running {overview.current.system.service="capacity unavailable".into();}
            let mut chosen=None;let mut position=egui::Pos2::ZERO;
            let label=if running {"Stop installer"} else {"Run companion in this setup"};
            for pressed in [None,Some(true),Some(false)] {
                let events=pressed.map(|pressed|vec![egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {pos:position,button:egui::PointerButton::Primary,
                        pressed,modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output=ctx.run_ui(egui::RawInput {events,..Default::default()},|ui| {
                    ui.set_max_width(width-32.0);ui.add_space(410.0);
                    Operator::fast_setup(ui,&overview.clone().into(),true,false,(&mut false,&mut chosen,&mut false,
                        &mut Page::Setup,&mut crate::library::Library::default(),&mut RouteFocus::default(),&mut None));
                });
                if pressed.is_none() {
                    position=output.shapes.iter().rev().find_map(|shape|text_position(&shape.shape,label)).unwrap();
                    if running {assert!(position.y<800.0,"Stop below viewport: {position:?}");}
                    assert!(output.shapes.iter().any(|shape|text_position(&shape.shape,
                        "Affected plug-ins: Synth, Effect sibling").is_some()));
                }
                output.textures_delta.clear();
            }
            assert_eq!(chosen,Some(if running {stop.clone()} else {run.clone()}));
        }
    }
    #[test]
    fn ordinary_and_narrow_navigation_keeps_every_page_touch_reachable() {
        for (width, selected_page) in [(960.0, Page::Home), (560.0, Page::Activity)] {
            let ctx = egui::Context::default();
            let mut page = selected_page;
            let mut bounds = Vec::new();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                bounds = navigation(ui, &mut page);
            });
            output.textures_delta.clear();
            assert_eq!(bounds.len(), 6);
            assert_eq!(
                bounds.iter().filter(|(_, selected, _)| *selected).count(),
                1
            );
            assert!(bounds
                .iter()
                .any(|(item, selected, _)| *item == selected_page && *selected));
            let first = bounds[0].2;
            for (_, _, rect) in &bounds {
                assert!(rect.width() >= 94.0 && rect.height() >= 44.0);
                assert!(rect.left() >= first.left());
                assert!(rect.right() <= first.left() + width - 32.0);
                assert!(rect.bottom() <= first.top() + 112.0);
            }
            if width == 560.0 {
                let first_top = bounds[0].2.top();
                let second_top = bounds[3].2.top();
                assert!(second_top > first_top);
                assert!(bounds[..3]
                    .iter()
                    .all(|(_, _, rect)| rect.top() == first_top && rect.width() >= 150.0));
                assert!(bounds[3..]
                    .iter()
                    .all(|(_, _, rect)| rect.top() == second_top && rect.width() >= 150.0));
            } else {
                assert!(bounds.iter().all(|(_, _, rect)| rect.top() == first.top()));
            }
        }
    }
    #[test]
    fn workspace_state_actions_and_history_are_reachable_at_ordinary_and_narrow_width() {
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes {
                        texts(shape, out);
                    }
                }
                _ => {}
            }
        }
        let mut snapshot: Snapshot =
            serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap();
        snapshot.workspaces.push(DawWorkspace {
            id: "ab".repeat(16),
            name: "FL Studio".into(),
            state: "uninstalled".into(),
            selected_installer: "cd".repeat(32),
            application_installers: vec!["cd".repeat(32)],
            selected_release: "26.1.6.0".into(),
            installed_advertised_release: None,
            observed_file_version: None,
            installed_image_sha256: None,
            active_installation_operation: None,
            cleanup: "confirmed".into(),
            first_useful_failure: None,
            actions: vec![AvailableAction {
                label: "Install selected version".into(),
                action: Action::WorkspaceInstall {},
                disabled_reason: Some("Prior owner has not retired".into()),
            }],
            installer_choices: vec![AvailableAction {
                label: "Choose imported installer efefefefefef".into(),
                action: Action::WorkspaceSelectInstaller {
                    installer: "ef".repeat(32),
                    release: String::new(),
                },
                disabled_reason: None,
            }],
            products: vec![DawWorkspaceProduct {
                id: WorkspaceProductId::Serum2,
                name: "Serum 2".into(),
                state: "selected".into(),
                selected_release: Some("2.1.5".into()),
                module_sha256: None,
                current_failure: None,
                actions: vec![AvailableAction {
                    label: "Install Serum 2 in FL Studio".into(),
                    action: Action::WorkspaceInstallProduct {
                        product: WorkspaceProductId::Serum2,
                    },
                    disabled_reason: Some("FL session is active".into()),
                }],
                installer_choices: vec![AvailableAction {
                    label: "Choose official Serum 2 2.1.5 installer".into(),
                    action: Action::WorkspaceSelectProductInstaller {
                        product: WorkspaceProductId::Serum2,
                        installer: "ef".repeat(32),
                        release: String::new(),
                    },
                    disabled_reason: None,
                }],
                details: serde_json::json!({"installation_history":[]}),
            }],
            details: serde_json::json!({"installation_history":[{"operation":"old-exact"}]}),
        });
        for width in [960.0, 560.0] {
            let ctx = egui::Context::default();
            let mut chosen = None;
            let mut pick = false;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                Operator::workspaces(ui, &snapshot, false, &mut chosen, &mut pick, &mut RouteFocus::default());
            });
            let mut labels = Vec::new();
            for clipped in &output.shapes {
                texts(&clipped.shape, &mut labels);
            }
            output.textures_delta.clear();
            assert!(labels.iter().any(|s| s == "FL Studio"));
            assert!(labels.iter().any(|s| s == "Uninstalled"));
            assert!(labels.iter().any(|s| s == "Selected version: 26.1.6.0"));
            assert!(labels.iter().any(|s| s == "Install selected version"));
            assert!(labels.iter().any(|s| s == "Import Windows installer"));
            assert!(labels.iter().any(|s| s == "Prior owner has not retired"));
            assert!(labels.iter().any(|s| s == "Choose an imported version"));
            assert!(labels.iter().any(|s| s == "Serum 2"));
            assert!(labels
                .iter()
                .any(|s| s == "Selected plug-in version: 2.1.5"));
            assert!(labels.iter().any(|s| s == "Install Serum 2 in FL Studio"));
            assert!(labels.iter().any(|s| s == "FL session is active"));
            assert!(labels
                .iter()
                .any(|s| s == "Choose an imported plug-in version"));
            assert!(labels
                .iter()
                .any(|s| s == "Technical workspace details and history"));
            assert!(chosen.is_none());
            assert!(!pick);
            // A selected but unsubmitted replacement must not capture an import
            // initiated from Workspaces. Exercise the actual general picker.
            let replacement=wrong_installer_overview(Some("intended-environment"));
            let mut operator=state_fixture();
            operator.focus.replacement=replacement_context(&replacement.current,&replacement.current.installer_setups[0]);
            operator.focus.replacement.as_mut().unwrap().selected=Some("old-selected-installer".into());
            let mut point=egui::Pos2::ZERO;
            for pressed in [None,Some(true),Some(false)] {
                let events=pressed.map(|pressed|vec![egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {pos:point,button:egui::PointerButton::Primary,pressed,
                        modifiers:egui::Modifiers::NONE}]).unwrap_or_default();
                let mut output=ctx.run_ui(egui::RawInput {events,..Default::default()},|ui| {
                    ui.set_max_width(width-32.0);
                    Operator::workspaces(ui,&snapshot,false,&mut None,&mut pick,&mut operator.focus);
                });
                if pressed.is_none() {point=output.shapes.iter().find_map(|shape|
                    wrong_installer_text_position(&shape.shape,"Import Windows installer")).unwrap();}
                output.textures_delta.clear();
            }
            assert!(pick);assert!(operator.focus.replacement.is_none());
            operator.origin=Some(RequestOrigin::InstallerImport);
            operator.handle_reply(Reply::Imported(InstallerImportResult {schema:2,
                installer_sha256:"34".repeat(32),display_label:"General import".into(),byte_size:1024,
                format:"pe_executable".into(),newly_imported:true,created_at:1}));
            assert_eq!(operator.pending_import_route.as_ref().unwrap().0,"34".repeat(32));
            assert!(operator.queued_action.is_none());
        }
        let workspace = &mut snapshot.workspaces[0];
        workspace.state = "ready".into();
        workspace.selected_release = "27.0.0.0".into();
        workspace.installed_advertised_release = Some("26.1.6.0".into());
        workspace.observed_file_version = Some("26.1.6.5639".into());
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            Operator::workspaces(ui, &snapshot, false, &mut None, &mut false, &mut RouteFocus::default());
        });
        let mut labels = Vec::new();
        for clipped in &output.shapes {
            texts(&clipped.shape, &mut labels);
        }
        output.textures_delta.clear();
        assert!(labels.iter().any(|s| s == "Selected version: 27.0.0.0"));
        assert!(labels
            .iter()
            .any(|s| s == "Installed from declared release: 26.1.6.0"));
        assert!(labels
            .iter()
            .any(|s| s == "Observed executable version: 26.1.6.5639"));

        assert!(!refresh_for_workspace_product(&snapshot, false));
        assert!(refresh_for_workspace_product(&snapshot, true));
        let product = &mut snapshot.workspaces[0].products[0];
        product.state = "installing".into();
        product.actions = vec![AvailableAction {
            label: "Complete Serum installation readback".into(),
            action: Action::WorkspaceFinishProductInstall {
                product: WorkspaceProductId::Serum2,
            },
            disabled_reason: Some("Serum installer is active".into()),
        }];
        assert!(refresh_for_workspace_product(&snapshot, true));
        let mut operator = Operator::preview(snapshot.clone(), Page::Workspaces);
        operator.handle_reply(Reply::Activity(Activity {
            schema: crate::model::OPERATOR_SCHEMA,
            system: snapshot.system.clone(),
            capture: snapshot.capture.clone(),
            operation: snapshot.operation.clone(),
            workspace_product_install_ready: true,
        }));
        assert!(operator.refresh_after);
        snapshot.workspaces[0].products[0].actions[0].disabled_reason = None;
        assert!(!refresh_for_workspace_product(&snapshot, true));
    }
    #[test]
    fn workspace_primary_action_uses_manager_eligibility_without_changing_offers() {
        let actions = vec![
            AvailableAction {
                label: "Launch FL Studio".into(),
                action: Action::WorkspaceLaunch {},
                disabled_reason: Some("FL session already active".into()),
            },
            AvailableAction {
                label: "Focus FL Studio".into(),
                action: Action::WorkspaceFocus {},
                disabled_reason: None,
            },
        ];
        assert_eq!(workspace_primary_action(&actions), Some(1));
        assert_eq!(
            actions[0].disabled_reason.as_deref(),
            Some("FL session already active")
        );
        assert_eq!(workspace_primary_action(&actions[..1]), Some(0));
        assert_eq!(workspace_primary_action(&[]), None);
    }
    #[test]
    fn session_attention_opens_activity_at_exact_session_identity() {
        let mut page = Page::Home;
        let mut library = crate::library::Library::default();
        let mut focus = RouteFocus::default();
        navigate(
            &Destination::Session("exact-session".into()),
            &mut page,
            &mut library,
            &mut focus,
        );
        assert_eq!(page, Page::Activity);
        assert_eq!(focus.activity.as_deref(), Some("exact-session"));
        navigate(
            &Destination::Attempt(AttemptKey {
                installer: "exact-installer".into(),
                environment: None,
            }),
            &mut page,
            &mut library,
            &mut focus,
        );
        assert_eq!(page, Page::Setup);
        assert!(focus.setup_scroll);
    }
    #[test]
    fn lost_readback_marks_capacity_unavailable_without_inventing_cleanup_failure() {
        let mut operator = state_fixture();
        operator.snapshot =
            Some(serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap());
        operator.handle_reply(Reply::Error("readback lost".into()));
        let system = &operator.snapshot.as_ref().unwrap().system;
        assert!(!system.capacity_available());
        assert!(!system.cleanup_unconfirmed);
        assert_eq!(
            presentation::activity_certainty(system).cleanup_label(),
            "Cleanup unknown"
        );
    }
    #[test]
    fn failed_background_readback_is_quiet_and_health_recovers_on_next_poll() {
        let snapshot: Snapshot =
            serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap();
        let mut operator = state_fixture();
        operator.snapshot = Some(snapshot.clone());
        operator.message = "Installer added. No installation has started.".into();
        operator.pending = true;
        operator.background_poll = true;
        operator.origin = Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(Reply::Error("activity readback lost".into()));
        assert_eq!(operator.message, "Installer added. No installation has started.");
        assert!(operator.feedback.is_none());
        let stale = operator.snapshot.as_ref().unwrap();
        assert_eq!(presentation::health(&stale.system), presentation::Health::Unavailable);
        assert!(stale.system.inactive_reason().is_some());
        operator.pending = true;
        operator.background_poll = true;
        operator.origin = Some(RequestOrigin::BackgroundActivity);
        operator.handle_reply(activity_from(&snapshot));
        assert_eq!(presentation::health(&operator.snapshot.as_ref().unwrap().system),
            presentation::Health::Ready);
        assert_eq!(operator.message, "Installer added. No installation has started.");
    }
    fn create_request() -> Request {
        Request {
            schema: crate::model::OPERATOR_SCHEMA,
            state_token: "snapshot".into(),
            action: Action::InstallerEnvironmentCreate {
                installer: "ab".repeat(32),
                runner: "cd".repeat(32),
            },
        }
    }
    fn running_snapshot(operation: &str) -> Snapshot {
        let id = "aa".repeat(16);
        Snapshot {
            managed_runtime_records:vec![],
            schema: crate::model::OPERATOR_SCHEMA,
            state_token: "current".into(),
            installer_setups: vec![],
            environment_installers: vec![],
            system: System {
                service: "capacity unavailable".into(),
                keepers: 0,
                dsp: 0,
                maintenance: 0,
                ceiling: 0,
                pending_transactions: 0,
                stale_transports: 0,
                cleanup_unconfirmed: true,
            },
            onboarding: vec![Onboarding {
                failure: None,
                installer: "bb".repeat(32),
                name: "Installer".into(),
                byte_size: 10,
                format: "pe_executable".into(),
                environment: Some(id.clone()),
                state: "running".into(),
                required_human_action: "installer_ui".into(),
                details: serde_json::json!({"installation":{"operation":operation}}),
                actions: vec![AvailableAction {
                    label: "Stop installer".into(),
                    action: Action::InstallerStop {
                        onboarding: id,
                        operation: operation.into(),
                    },
                    disabled_reason: None,
                }],
            }],
            environments: vec![],
            vendor_applications: vec![],
            products: vec![],
            workspaces: vec![],
            active_sessions: vec![],
            capture: serde_json::Value::Null,
            recent_incidents: vec![],
            actions: vec![],
            operation: Some(serde_json::json!({"operation":operation,"state":"vendor_running"})),
            deleted_setups: vec![],
            uninstallers: vec![],
        }
    }
    fn start_feedback(o: &mut Operator, acknowledged: bool) {
        o.feedback = Some(RequestFeedback::captured(Action::InstallerStart {
            onboarding: "aa".repeat(16),
        }));
        if acknowledged {
            o.handle_reply(Reply::Receipt(Receipt {
                schema: crate::model::OPERATOR_SCHEMA,
                accepted: true,
                operation: Some("current-op".into()),
                refusal: None,
            }));
        } else {
            o.action_inflight = true;
            o.handle_reply(Reply::Error("ack lost".into()));
        }
    }
    fn activity_from(s: &Snapshot) -> Reply {
        Reply::Activity(Activity {
            schema: crate::model::OPERATOR_SCHEMA,
            system: s.system.clone(),
            capture: s.capture.clone(),
            operation: s.operation.clone(),
            workspace_product_install_ready: false,
        })
    }
    #[test]
    fn terminal_retry_offers_survive_snapshot_and_poll_without_frontend_inference() {
        // These are manager-projected offers, not frontend eligibility rules.
        // Cover both eligible completed outcomes and the three explicit refusals.
        for (durable, outcome, linked, offered, disabled) in [
            ("not_installed", "not_installed", false, true, None),
            (
                "partial_installation",
                "partial_installation",
                false,
                true,
                None,
            ),
            ("installed", "installed", false, false, None),
            (
                "partial_installation",
                "cleanup_unconfirmed",
                false,
                false,
                None,
            ),
            (
                "partial_installation",
                "partial_installation",
                true,
                false,
                None,
            ),
            (
                "partial_installation",
                "partial_installation",
                false,
                true,
                Some("active DSP"),
            ),
        ] {
            let mut s = running_snapshot("prior-op");
            s.system.service = "active".into();
            s.system.cleanup_unconfirmed = outcome == "cleanup_unconfirmed";
            s.operation = Some(serde_json::json!({"operation":"prior-op","state":"completed"}));
            let card = &mut s.onboarding[0];
            card.state = "completed".into();
            card.details = serde_json::json!({"installation":{"operation":"prior-op","state":"completed",
                "transaction":{"schema":1,"operation":"prior-op","outcome":outcome,"durable_installation":durable}},"linked_attempt":linked});
            let action = Action::InstallerNewAttempt {
                previous: card.environment.clone().unwrap(),
                runner: "cd".repeat(32),
            };
            card.actions = if offered {
                vec![AvailableAction {
                    label: "New isolated attempt".into(),
                    action: action.clone(),
                    disabled_reason: disabled.map(Into::into),
                }]
            } else {
                vec![]
            };
            let mut o = state_fixture(); // reopening receives canonical snapshot
            o.handle_reply(Reply::Snapshot(Box::new(s.clone())));
            o.handle_reply(activity_from(&s));
            let ctx = egui::Context::default();
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            for pressed in [None, Some(true), Some(false)] {
                o.pending = pressed == Some(false);
                o.background_poll = o.pending;
                let events = pressed
                    .map(|pressed| {
                        vec![
                            egui::Event::PointerMoved(point),
                            egui::Event::PointerButton {
                                pos: point,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ]
                    })
                    .unwrap_or_default();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        point = ui.next_widget_position() + egui::vec2(12.0, 12.0);
                        let projected = o.snapshot.as_ref().unwrap();
                        Operator::buttons(
                            ui,
                            &projected.onboarding[0].actions,
                            projected.system.inactive_reason(),
                            o.controls_pending(),
                            &mut chosen,
                        );
                    },
                );
                output.textures_delta.clear();
            }
            assert_eq!(
                chosen,
                if offered && disabled.is_none() {
                    Some(action)
                } else {
                    None
                },
                "{durable}/{outcome}, linked={linked}"
            );
        }
    }
    #[test]
    fn integrated_snapshot_first_and_activity_first_reconcile_stop() {
        for snapshot_first in [true, false] {
            let mut o = state_fixture();
            start_feedback(&mut o, true);
            assert!(o.controls_pending());
            let s = running_snapshot("current-op");
            if !snapshot_first {
                o.handle_reply(activity_from(&s));
                assert!(o.controls_pending());
            }
            o.handle_reply(Reply::Snapshot(Box::new(s.clone())));
            assert!(!o.controls_pending());
            o.handle_reply(activity_from(&s));
            assert!(
                !o.controls_pending(),
                "equal running must not require another transition"
            );
            let ctx = egui::Context::default();
            let mut chosen = None;
            let mut point = egui::Pos2::ZERO;
            for pressed in [None, Some(true), Some(false)] {
                o.pending = pressed == Some(false);
                o.background_poll = o.pending;
                let events = pressed
                    .map(|pressed| {
                        vec![
                            egui::Event::PointerMoved(point),
                            egui::Event::PointerButton {
                                pos: point,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ]
                    })
                    .unwrap_or_default();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        point = ui.next_widget_position() + egui::vec2(12.0, 12.0);
                        Operator::buttons(
                            ui,
                            &s.onboarding[0].actions,
                            s.system.inactive_reason(),
                            o.controls_pending(),
                            &mut chosen,
                        );
                    },
                );
                output.textures_delta.clear();
            }
            assert!(matches!(chosen, Some(Action::InstallerStop { .. })));
            assert!(!s.onboarding[0]
                .actions
                .iter()
                .any(|a| matches!(a.action, Action::InstallerStart { .. })));
        }
    }
    #[test]
    fn policy_start_keeps_exact_stop_usable_across_reply_orders() {
        for snapshot_first in [true, false] {
            let mut o = state_fixture();
            start_feedback(&mut o, true);
            let feedback = o.feedback.as_mut().unwrap();
            let onboarding = match &feedback.action {
                Action::InstallerStart { onboarding } => onboarding.clone(),
                _ => panic!("fixture"),
            };
            feedback.action = Action::InstallerStartWithPolicy {
                onboarding,
                powershell: Powershell::IntentionallyUnavailable,
            };
            assert!(o.controls_pending());
            let s = running_snapshot("current-op");
            if !snapshot_first {
                o.handle_reply(activity_from(&s));
                assert!(o.controls_pending());
            }
            o.handle_reply(Reply::Snapshot(Box::new(s.clone())));
            assert!(!o.controls_pending());
            o.handle_reply(activity_from(&s));
            assert!(
                !o.controls_pending(),
                "equal running must not require another transition"
            );
            let ctx = egui::Context::default();
            let mut chosen = None;
            let mut point = egui::Pos2::ZERO;
            for pressed in [None, Some(true), Some(false)] {
                o.pending = pressed == Some(false);
                o.background_poll = o.pending;
                let events = pressed
                    .map(|pressed| {
                        vec![
                            egui::Event::PointerMoved(point),
                            egui::Event::PointerButton {
                                pos: point,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ]
                    })
                    .unwrap_or_default();
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        point = ui.next_widget_position() + egui::vec2(12.0, 12.0);
                        Operator::buttons(
                            ui,
                            &s.onboarding[0].actions,
                            s.system.inactive_reason(),
                            o.controls_pending(),
                            &mut chosen,
                        );
                    },
                );
                output.textures_delta.clear();
            }
            assert!(matches!(chosen, Some(Action::InstallerStop { .. })));
            assert!(!s.onboarding[0]
                .actions
                .iter()
                .any(|a| matches!(a.action, Action::InstallerStart { .. })));
        }
    }
    #[test]
    fn integrated_unrelated_snapshot_cannot_unlock_request() {
        let mut o = state_fixture();
        start_feedback(&mut o, true);
        o.handle_reply(Reply::Snapshot(Box::new(running_snapshot("old-op"))));
        assert!(o.controls_pending());
        assert!(!o.feedback.as_ref().unwrap().terminal);
        o.handle_reply(activity_from(&running_snapshot("old-op")));
        assert!(o.controls_pending());
        o.handle_reply(activity_from(&running_snapshot("current-op")));
        o.handle_reply(Reply::Snapshot(Box::new(running_snapshot("old-op"))));
        assert!(
            o.controls_pending(),
            "unrelated snapshot cannot release a prior running transition"
        );
    }
    #[test]
    fn uncertain_ack_reconciles_exact_live_owner_without_resubmission() {
        let mut o = state_fixture();
        start_feedback(&mut o, false);
        assert!(!o.feedback.as_ref().unwrap().terminal);
        assert!(o.controls_pending());
        let mut unrelated = running_snapshot("other-op");
        unrelated.onboarding[0].environment = Some("cc".repeat(16));
        o.handle_reply(Reply::Snapshot(Box::new(unrelated)));
        assert!(o.controls_pending());
        let s = running_snapshot("current-op");
        o.handle_reply(Reply::Snapshot(Box::new(s)));
        assert!(!o.controls_pending());
        assert!(o.next_action().is_none());
        assert_eq!(
            o.feedback.as_ref().unwrap().operation.as_deref(),
            Some("current-op")
        );
        assert!(o
            .feedback
            .as_ref()
            .unwrap()
            .transitions
            .iter()
            .any(|s| s == "live_owner_reconciled"));
        for _ in 0..100 {
            o.feedback.as_mut().unwrap().transition("running");
            o.feedback.as_mut().unwrap().transition("validating");
        }
        assert_eq!(o.feedback.as_ref().unwrap().transitions.len(), 12);
    }
    #[test]
    fn background_poll_click_is_captured_then_submitted_once() {
        let mut o = state_fixture();
        let mut overview = overview_fixture();
        overview.current.state_token = create_request().state_token;
        o.handle_reply(Reply::Overview(Box::new(overview)));
        o.pending = true;
        o.background_poll = true;
        assert!(!o.controls_pending());
        o.capture_action(create_request());
        assert!(o.controls_pending());
        assert!(o.next_action().is_none());
        o.feedback.as_mut().unwrap().observe(
            &serde_json::json!({"operation":"old","state":"refused","reason":"old lock failure"}),
        );
        assert!(o.feedback.as_ref().unwrap().text.contains("Click received"));
        o.pending = false;
        o.background_poll = false;
        assert_eq!(o.next_action().unwrap().action, create_request().action);
        assert!(o.next_action().is_none());
        assert!(o.controls_pending());
    }
    #[test]
    fn competing_expert_form_preserves_recovery_receipt_and_terminal_result() {
        for draft in [
            Action::CandidateObserve { candidate: "ef".repeat(32), area: "editor".into(),
                status: "failed".into(), note: "Observed symptom".into() },
            Action::CandidateReview { candidate: "ef".repeat(32), accept: false,
                rationale: "Retained work is needed".into() },
        ] {
            for accepted in [true, false] {
                let mut o = state_fixture();
                o.overview = Some(overview_fixture().into());
                o.overview_fresh = true;
                let recovery = Action::OrdinaryRollback { class_id: "ab".repeat(16),
                    publication: "cd".repeat(16) };
                o.product_form = Some(draft.clone());
                assert!(o.capture_chosen_action(recovery.clone()));
                let request = o.next_action().unwrap();
                assert_eq!(request.action, recovery);
                assert_eq!(o.product_form.as_ref(), Some(&draft));
                o.pending = true;
                o.action_inflight = true;
                o.origin = Some(RequestOrigin::UserAction);
                assert!(!o.capture_chosen_action(draft.clone()));
                assert!(o.queued_action.is_none());
                assert_eq!(o.feedback.as_ref().unwrap().action, recovery);
                assert_eq!(o.product_form.as_ref(), Some(&draft));

                let operation = "31".repeat(16);
                let reason = "Exact affected owner remains live";
                o.handle_reply(Reply::Receipt(Receipt { schema: crate::model::OPERATOR_SCHEMA,
                    accepted, operation: Some(operation.clone()),
                    refusal: (!accepted).then(|| reason.into()) }));
                assert!(!o.capture_chosen_action(draft.clone()));
                let feedback = o.feedback.as_ref().unwrap();
                assert_eq!(feedback.action, recovery);
                assert_eq!(feedback.operation.as_deref(), Some(operation.as_str()));
                if !accepted { assert!(feedback.text.contains(reason)); }

                let terminal = serde_json::json!({"schema":1,"operation":operation,
                    "state":if accepted {"completed"} else {"refused"},
                    "action":recovery,"reason":reason});
                let mut pulse = pulse_fixture(false);
                pulse.operation = Some(terminal.clone());
                o.origin = Some(RequestOrigin::BackgroundActivity);
                o.handle_reply(Reply::Pulse(pulse));
                assert!(!o.capture_chosen_action(draft.clone()),
                    "terminal receipt still needs its fresh readback");
                let feedback = o.feedback.as_ref().unwrap();
                assert!(feedback.terminal && feedback.blocking);
                assert_eq!(feedback.action, recovery);
                assert_eq!(feedback.operation.as_deref(), Some(operation.as_str()));
                assert_eq!(o.product_form.as_ref(), Some(&draft));

                let mut current = overview_fixture();
                current.current.state_token = "after-recovery".into();
                current.readiness.state_token = current.current.state_token.clone();
                current.current.operation = Some(terminal);
                o.origin = Some(RequestOrigin::SilentPostMutationRefresh);
                o.handle_reply(Reply::Overview(Box::new(current)));
                let feedback = o.feedback.as_ref().unwrap();
                assert!(feedback.terminal && !feedback.blocking);
                assert_eq!(feedback.action, recovery);
                assert_eq!(feedback.operation.as_deref(), Some(operation.as_str()));
                if !accepted { assert!(feedback.text.contains(reason)); }
                assert!(o.queued_action.is_none(), "retained drafts are never resubmitted");
                assert_eq!(o.product_form.as_ref(), Some(&draft));

                assert!(o.capture_chosen_action(draft.clone()));
                assert!(o.product_form.is_none());
                let later = o.next_action().unwrap();
                assert_eq!(later.action, draft);
                assert_eq!(later.state_token, "after-recovery");
                assert!(o.next_action().is_none());
            }
        }
    }
    #[test]
    fn recovery_capture_queues_once_during_background_readback() {
        for changed in [false, true] {
            let mut o = state_fixture();
            let overview = overview_fixture();
            let token = overview.current.state_token.clone();
            o.overview = Some(overview.into());
            o.overview_fresh = true;
            o.pending = true;
            o.background_poll = true;
            o.origin = Some(RequestOrigin::BackgroundActivity);
            let recovery = Action::OrdinaryRollback { class_id: "ab".repeat(16),
                publication: "cd".repeat(16) };
            assert!(o.capture_chosen_action(recovery.clone()));
            assert!(!o.capture_chosen_action(Action::BufferingSet {
                class_id: "ab".repeat(16), added_frames: 512 }));
            assert!(o.next_action().is_none());
            let mut pulse = pulse_fixture(false);
            if changed { pulse.cleanup_unconfirmed = Some(true); }
            o.handle_reply(Reply::Pulse(pulse));
            assert_eq!(o.queued_action.as_ref().unwrap().state_token, token);
            if changed {
                assert!(o.next_action().is_none());
                let feedback = o.feedback.as_ref().unwrap();
                assert_eq!(feedback.action, recovery);
                assert!(feedback.terminal && !feedback.blocking);
                assert!(feedback.text.contains("Current status is unavailable"));
            } else {
                let request = o.next_action().unwrap();
                assert_eq!(request.schema, crate::model::OPERATOR_SCHEMA);
                assert_eq!(request.state_token, token, "never relabel a captured request");
                assert_eq!(request.action, recovery);
            }
            assert!(o.next_action().is_none());
        }
    }
    #[test]
    fn form_drafts_survive_unavailable_authority_and_competing_import() {
        let mut o = state_fixture();
        o.overview = Some(overview_fixture().into());
        let observation = Action::CandidateObserve { candidate: "ef".repeat(32),
            area: "editor".into(), status: "failed".into(), note: "Retained note".into() };
        let workspace = Action::WorkspaceSelectInstaller {
            installer: "ab".repeat(32), release: "Exact release".into() };
        let rename = Action::InstallerRename { installer: "cd".repeat(32),
            label: "Retained display name".into() };
        o.product_form = Some(observation.clone());
        o.workspace_select_form = Some(workspace.clone());
        o.installer_rename_form = Some(rename.clone());
        for draft in [&observation, &workspace, &rename] {
            assert!(!o.capture_chosen_action(draft.clone()));
        }
        o.overview_fresh = true;
        o.queued_import = true;
        for draft in [&observation, &workspace, &rename] {
            assert!(!o.capture_chosen_action(draft.clone()));
        }
        assert!(o.feedback.is_none() && o.queued_action.is_none());
        assert_eq!(o.product_form.as_ref(), Some(&observation));
        assert_eq!(o.workspace_select_form.as_ref(), Some(&workspace));
        assert_eq!(o.installer_rename_form.as_ref(), Some(&rename));
        o.queued_import = false;
        assert!(o.capture_chosen_action(workspace.clone()));
        assert!(o.workspace_select_form.is_none());
        assert_eq!(o.product_form.as_ref(), Some(&observation));
        assert_eq!(o.installer_rename_form.as_ref(), Some(&rename));
        assert_eq!(o.next_action().unwrap().action, workspace);
    }
    #[test]
    fn long_setup_feedback_names_only_the_exact_running_operation() {
        let mut feedback=RequestFeedback::captured(Action::RuntimeInstall {});
        feedback.operation=Some("current".into());
        feedback.observe(&serde_json::json!({"operation":"other","state":"running"}));
        assert!(feedback.text.contains("Click received"));
        feedback.observe(&serde_json::json!({"operation":"current","state":"queued"}));
        assert!(feedback.text.contains("queued"));
        assert!(!feedback.text.contains("Downloading"));
        feedback.observe(&serde_json::json!({"operation":"current","state":"running"}));
        assert!(feedback.text.contains("Downloading and preparing"));
        assert!(feedback.blocking);
        feedback.observe(&serde_json::json!({"operation":"current","state":"completed"}));
        assert!(feedback.terminal);
        assert!(!feedback.text.contains("Downloading"));
        feedback.observe(&serde_json::json!({"operation":"current","state":"running"}));
        assert!(!feedback.text.contains("Downloading"),"terminal work cannot become running again");
        let mut feedback=RequestFeedback::captured(Action::EnvironmentRescan {environment:"ab".repeat(16)});
        feedback.operation=Some("scan".into());
        feedback.observe(&serde_json::json!({"operation":"scan","state":"running"}));
        assert!(feedback.text.contains("Checking the installed products"));
        assert!(!feedback.text.contains("Downloading"));
    }
    #[test]
    fn an_uninstall_shows_its_setup_being_checked_once_the_uninstaller_closed() {
        let progress = "Checking this setup's plug-ins (2 of 3)… The bridge is paused for all your setups until this finishes.";
        for action in [Action::ProductUninstall {environment:"ab".repeat(16),registration:"cd".repeat(32),
            command_sha256:"ef".repeat(32),confirmed:true}, Action::InstallerCloseCheck {operation:"ab".repeat(16)}] {
            let mut feedback=RequestFeedback::captured(action);
            feedback.operation=Some("current".into());
            feedback.observe(&serde_json::json!({"operation":"current","state":"vendor_running"}));
            feedback.observe(&serde_json::json!({"operation":"current","state":"running","progress":progress}));
            assert_eq!(feedback.text, progress);
            feedback.observe(&serde_json::json!({"operation":"current","state":"completed",
                "result":{"message":"Freq Echo was checked again and stays in your DAW."}}));
            assert_eq!(feedback.text, "Freq Echo was checked again and stays in your DAW.");
        }
    }
    #[test]
    fn guided_check_keeps_the_offered_identity_and_queues_once() {
        let mut o = state_fixture();
        let action = Action::CompatibilityCheck {
            selection: "ab".repeat(32), audio_layout: None,
            recipe: "cd".repeat(32), predecessor: None,
        };
        let request = Request { schema: crate::model::OPERATOR_SCHEMA, state_token: "current".into(), action };
        let mut overview = overview_fixture();
        overview.current.state_token = request.state_token.clone();
        o.handle_reply(Reply::Overview(Box::new(overview)));
        o.pending = true;
        o.background_poll = true;
        o.capture_action(request.clone());
        assert!(o.controls_pending());
        assert!(o.next_action().is_none());
        o.pending = false;
        o.background_poll = false;
        assert_eq!(o.next_action().unwrap().action, request.action);
        assert!(o.next_action().is_none());
        assert!(o.controls_pending());
    }
    #[test]
    fn old_poll_cannot_overwrite_current_prequeue_refusal_or_inflight_request() {
        let mut f = RequestFeedback::captured(create_request().action);
        let old =
            serde_json::json!({"operation":"old","state":"refused","reason":"old lock failure"});
        f.receipt(&Receipt {
            schema: crate::model::OPERATOR_SCHEMA,
            accepted: false,
            operation: Some("new".into()),
            refusal: Some("fresh refusal".into()),
        });
        f.observe(&old);
        assert!(f.text.contains("fresh refusal"));
        assert!(!f.text.contains("old lock"));
        f = RequestFeedback::captured(create_request().action);
        f.receipt(&Receipt {
            schema: crate::model::OPERATOR_SCHEMA,
            accepted: true,
            operation: Some("new".into()),
            refusal: None,
        });
        f.observe(&old);
        assert!(!f.terminal);
        f.observe(&serde_json::json!({"operation":"new","state":"completed"}));
        assert!(f.terminal);
        f.observe(&old);
        assert!(f.text.contains("completed"));
    }
    #[test]
    fn vendor_controls_unlock_only_after_current_snapshot_without_losing_feedback() {
        let mut f = RequestFeedback::captured(create_request().action);
        f.receipt(&Receipt {
            schema: crate::model::OPERATOR_SCHEMA,
            accepted: true,
            operation: Some("new".into()),
            refusal: None,
        });
        f.observe(&serde_json::json!({"operation":"new","state":"vendor_running"}));
        assert!(f.blocking);
        f.refreshed();
        assert!(!f.blocking && !f.terminal);
        f.observe(&serde_json::json!({"operation":"new","state":"completed"}));
        assert!(f.terminal);
        let mut f = RequestFeedback::captured(create_request().action);
        f.receipt(&Receipt {
            schema: crate::model::OPERATOR_SCHEMA,
            accepted: false,
            operation: Some("refused".into()),
            refusal: Some("reason".into()),
        });
        assert!(f.blocking);
        f.refreshed();
        assert!(!f.blocking);
        assert!(f.text.contains("reason"));
    }
    #[test]
    fn real_button_release_survives_background_poll_start() {
        fn click(old_gate: bool) -> bool {
            let ctx = egui::Context::default();
            let mut o = state_fixture();
            let actions = [AvailableAction {
                label: "Create isolated environment".into(),
                action: create_request().action,
                disabled_reason: None,
            }];
            let mut point = egui::Pos2::ZERO;
            let mut chosen = None;
            let mut frame = |events: Vec<egui::Event>, pending: bool| {
                o.pending = pending;
                o.background_poll = pending;
                let input = egui::RawInput {
                    events,
                    ..Default::default()
                };
                let mut output = ctx.run_ui(input, |ui| {
                    point = ui.next_widget_position() + egui::vec2(12.0, 12.0);
                    Operator::buttons(
                        ui,
                        &actions,
                        None,
                        if old_gate {
                            o.pending
                        } else {
                            o.controls_pending()
                        },
                        &mut chosen,
                    );
                });
                output.textures_delta.clear();
                point
            };
            let p = frame(vec![], false);
            frame(
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                false,
            );
            frame(
                vec![egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
                true,
            );
            chosen.is_some()
        }
        assert!(
            !click(true),
            "reviewed global-pending gate loses this click"
        );
        assert!(
            click(false),
            "production action control must remain enabled through background polling"
        );
    }
    #[test]
    fn timeout_card_distinguishes_manager_refusal_from_installer_failure() {
        let f:crate::model::OperationFailure=serde_json::from_value(serde_json::json!({
            "layer":"manager_control_plane","stage":"operator_validation_readback","code":"registry_lock_timeout",
            "retryable":true,"mutation_started":false,"environment_created":false,"installer_launched":false,
            "lock":{"name":"registry.lock","mode":"exclusive","purpose":"operator_validation_readback","operation":"ab".repeat(16),"policy":"bounded","elapsed_wait_us":100000,"attempts":11,"timeout_ms":100,"outcome":"timeout","holder":"unknown"}
        })).unwrap();
        let text = failure_lines(&f).join("\n");
        for expected in [
            "Manager",
            "Environment creation did not start",
            "installer was not launched",
            "not an installer failure",
            "Retry is safe",
            "holder unknown",
        ] {
            assert!(text.contains(expected));
        }
        let mut f = f;
        f.stage = crate::model::FailureStage::EnvironmentCreationAdmission;
        f.lock.purpose = crate::model::LockPurpose::EnvironmentCreationAdmission;
        let text = failure_lines(&f).join(" ");
        assert!(text.contains("before environment creation"));
        assert!(text.contains("installer was not launched"));
        assert!(text.contains("Retry is safe"));
        f.retryable = false;
        assert!(!failure_lines(&f).join(" ").contains("Retry is safe"));
    }
    #[test]
    fn incident_summary_keeps_runner_exit_separate_and_missing_cleanup_unknown() {
        let v = serde_json::json!({"outcome":"process_scoped_vendor_retirement","outer_exit":-15,"windows_self_exit_codes":[],"exceptions":[]});
        let lines = incident_lines(&v).join("\n");
        assert!(lines.contains("process scoped vendor retirement"));
        assert!(lines.contains("Cleanup: unavailable"));
        assert!(lines.contains("separate from Windows): -15"));
        assert!(!lines.contains("crash"));
    }
    #[test]
    fn readback_waits_for_mutation_or_vendor_launch_receipt() {
        let initial = None;
        for state in ["queued", "waiting", "running"] {
            assert!(!refresh_for_receipt(
                &initial,
                &Some(serde_json::json!({"state":state}))
            ));
        }
        for state in ["vendor_running", "completed", "refused"] {
            let value = Some(serde_json::json!({"state":state}));
            assert!(refresh_for_receipt(&initial, &value));
            assert!(!refresh_for_receipt(&value, &value));
        }
    }
    #[test]
    fn app_only_handoff_replaces_the_current_refusal_copy_for_its_exact_owner() {
        let owner="11".repeat(16);
        let action=Action::SoftwareInstall {installer:"aa".repeat(32),daw:DawChoice::Other};
        let mut feedback=RequestFeedback::captured(action);
        feedback.receipt(&Receipt {schema:crate::model::OPERATOR_SCHEMA,accepted:true,
            operation:Some(owner.clone()),refusal:None});
        feedback.observe(&serde_json::json!({"operation":owner,"state":"refused",
            "reason":"no_supported_instruments_or_effects_discovered"}));
        assert!(feedback.terminal);assert!(feedback.text.contains("refused"));
        let mut snapshot=running_snapshot(&owner);
        let handoff=InstallationJourney {daw:DawChoice::Other,phase:InstallationPhase::AwaitingApplication,
            plugins:vec![],failure:None};
        snapshot.operation=Some(serde_json::json!({"operation":owner,"state":"refused",
            "reason":"no_supported_instruments_or_effects_discovered","journey":handoff}));
        let mut unrelated=feedback.clone();unrelated.operation=Some("22".repeat(16));
        unrelated.reconcile_snapshot(&snapshot);assert!(unrelated.text.contains("refused"));
        feedback.reconcile_snapshot(&snapshot);
        assert_eq!(feedback.text,"The app is installed. Open it below to install your instruments and effects.");
        assert!(feedback.terminal);assert!(!feedback.blocking);
        assert_eq!(feedback.journey.as_ref().unwrap().phase,InstallationPhase::AwaitingApplication);
        assert_eq!(snapshot.operation.as_ref().unwrap()["reason"],"no_supported_instruments_or_effects_discovered");
        let mut genuine_failure=RequestFeedback::captured(Action::SoftwareInstall {
            installer:"aa".repeat(32),daw:DawChoice::Other});
        genuine_failure.receipt(&Receipt {schema:crate::model::OPERATOR_SCHEMA,accepted:true,
            operation:Some(owner),refusal:None});
        snapshot.operation.as_mut().unwrap().as_object_mut().unwrap().remove("journey");
        genuine_failure.reconcile_snapshot(&snapshot);assert!(genuine_failure.text.contains("refused"));
    }

    #[test]
    fn matching_product_receipt_survives_a_newer_unrelated_latest_operation() {
        let mut f = RequestFeedback::captured(Action::PluginPrepare {
            selection: "ab".repeat(32),
            inspection: "cd".repeat(32),
            recipe: "ef".repeat(32),
            predecessor: None,
        });
        f.receipt(&Receipt {
            schema: 5,
            accepted: true,
            operation: Some("11".repeat(16)),
            refusal: None,
        });
        let mut s = running_snapshot(&"22".repeat(16));
        s.operation = Some(serde_json::json!({"operation":"22".repeat(16),"state":"completed"}));
        s.products.push(Product{class_id:"aa".repeat(16),name:"Generated instrument".into(),vendor:"Fixture".into(),role:"instrument".into(),version:"1".into(),disposition:"prepared".into(),active_revision:None,recommended_revision:None,environment:"ef".repeat(16),runner:"pinned".into(),module_sha256:"ff".repeat(32),limitations:vec![],history:vec![],actions:vec![],availability:None,compatibility:None,details:serde_json::json!({"preparation":{"operation":{"operation":"11".repeat(16),"state":"completed"}}})});
        f.reconcile_snapshot(&s);
        assert!(f.terminal);
        assert!(!f.blocking);
        let mut other = RequestFeedback::captured(Action::CandidateObserve {
            candidate: "bc".repeat(32),
            area: "processing_restart".into(),
            status: "failed".into(),
            note: "Exact candidate observation".into(),
        });
        other.receipt(&Receipt {
            schema: 5,
            accepted: true,
            operation: Some("33".repeat(16)),
            refusal: None,
        });
        s.products[0].details["preparation"]["candidates"] = serde_json::json!([
            {"id":"aa".repeat(32),"operation":{"operation":"11".repeat(16),"state":"completed"}},
            {"id":"bc".repeat(32),"operation":{"operation":"33".repeat(16),"state":"refused"}}
        ]);
        other.reconcile_snapshot(&s);
        assert!(other.terminal);
        assert!(!other.blocking);
        assert_eq!(
            other.transitions.last().map(String::as_str),
            Some("refused")
        );
    }

    #[test]
    fn dependency_uncertain_ack_recovers_only_exact_stop_and_terminal() {
        let op = "cd".repeat(16);
        let mut s = running_snapshot(&"ef".repeat(16));
        s.vendor_applications.push(VendorApplication{id:"native-access".into(),name:"Native Access".into(),version:"3.26.0".into(),state:"dependency".into(),actions:vec![AvailableAction{label:"Stop".into(),action:Action::DependencyStop{operation:op.clone()},disabled_reason:None}],details:serde_json::json!({"dependency_manager_operation":{"operation":op,"state":"submission_uncertain"}})});
        let mut f = RequestFeedback::captured(Action::DependencyPrepare {});
        f.receipt(&Receipt {
            schema: crate::model::OPERATOR_SCHEMA,
            accepted: true,
            operation: None,
            refusal: None,
        });
        f.reconcile_snapshot(&s);
        assert_eq!(f.operation, Some(op.clone()));
        assert!(!f.blocking);
        assert!(!f.terminal);
        s.vendor_applications[0].details["dependency_manager_operation"]["state"] =
            serde_json::json!("completed");
        f.reconcile_snapshot(&s);
        assert!(f.terminal);
        s.vendor_applications[0].actions.clear();
        let mut other = RequestFeedback::captured(Action::DependencyPrepare {});
        other.receipt(&Receipt {
            schema: crate::model::OPERATOR_SCHEMA,
            accepted: true,
            operation: None,
            refusal: None,
        });
        other.reconcile_snapshot(&s);
        assert!(other.operation.is_none());
    }

    #[test]
    fn renderer_retirement_does_not_complete_automatic_scan_or_hide_connection_failure() {
        let application="ab".repeat(32);let owner="cd".repeat(16);
        let mut feedback=RequestFeedback::captured(Action::RendererSoftwareInstall {
            application:application.clone(),policy:RendererPolicy::SoftwareRendering,daw:DawChoice::Bitwig});
        feedback.receipt(&Receipt {schema:crate::model::OPERATOR_SCHEMA,accepted:true,
            operation:Some(owner.clone()),refusal:None});
        let mut snapshot=running_snapshot(&owner);
        snapshot.vendor_applications.push(VendorApplication {id:"native-access".into(),name:"Installed app".into(),
            version:"1".into(),state:"retired".into(),actions:vec![],details:serde_json::json!({
                "application_identity":application,"operation":owner,"requested":"software_rendering",
                "manager_operation":{"operation":owner,"state":"completed"}})});
        let mut journey=automatic_journey(InstallationPhase::Discovering);journey.plugins.clear();
        snapshot.operation=Some(serde_json::json!({"operation":owner,"state":"running","journey":journey}));
        feedback.reconcile_snapshot(&snapshot);
        assert!(!feedback.terminal);assert_eq!(feedback.journey.as_ref().unwrap().phase,InstallationPhase::Discovering);
        assert_eq!(feedback.text,"Finding installed plug-ins");
        snapshot.operation=Some(serde_json::json!({"operation":owner,"state":"completed",
            "journey":automatic_journey(InstallationPhase::NeedsAttention)}));
        feedback.reconcile_snapshot(&snapshot);
        assert!(feedback.terminal);assert!(!feedback.blocking);
        assert!(feedback.text.contains("DAW connection needs attention"));
        assert_eq!(feedback.journey.as_ref().unwrap().plugins[1].failure.as_ref().unwrap().code,"fixture_load_failed");
        // A refused shared stage must also remain visible after child retirement.
        let mut refused=RequestFeedback::captured(feedback.action.clone());
        refused.receipt(&Receipt {schema:crate::model::OPERATOR_SCHEMA,accepted:true,
            operation:Some(owner.clone()),refusal:None});
        snapshot.operation=Some(serde_json::json!({"operation":owner,"state":"refused","reason":"discovery_failed"}));
        refused.reconcile_snapshot(&snapshot);assert!(refused.terminal);assert!(refused.text.contains("discovery_failed"));
    }
    #[test]
    fn empty_renderer_visit_and_connection_resume_keep_the_exact_app_handoff() {
        let original="cd".repeat(16);let resume="ef".repeat(16);
        for (action,owner) in [(Action::RendererSoftwareInstall {application:"ab".repeat(32),
                policy:RendererPolicy::SoftwareRendering,daw:DawChoice::Bitwig},original.clone()),
            (Action::EnvironmentRefreshResume {operation:original.clone()},resume)] {
            let mut feedback=RequestFeedback::captured(action);
            feedback.receipt(&Receipt {schema:crate::model::OPERATOR_SCHEMA,accepted:true,
                operation:Some(owner.clone()),refusal:None});
            feedback.observe(&serde_json::json!({"operation":owner,"state":"completed"}));
            let mut snapshot=running_snapshot(&owner);
            snapshot.operation=Some(serde_json::json!({"operation":owner,"state":"completed","renderer_owner":original,
                "journey":InstallationJourney {daw:DawChoice::Bitwig,phase:InstallationPhase::AwaitingApplication,
                    plugins:vec![],failure:None}}));
            let mut unrelated=feedback.clone();unrelated.operation=Some("12".repeat(16));
            unrelated.reconcile_snapshot(&snapshot);assert!(unrelated.journey.is_none());
            let mut foreign=feedback.clone();snapshot.operation.as_mut().unwrap()["renderer_owner"]=serde_json::json!("34".repeat(16));
            foreign.reconcile_snapshot(&snapshot);
            if matches!(feedback.action,Action::EnvironmentRefreshResume {..}) {assert!(foreign.journey.is_none());}
            snapshot.operation.as_mut().unwrap()["renderer_owner"]=serde_json::json!(original);
            feedback.reconcile_snapshot(&snapshot);
            assert!(feedback.terminal);assert!(!feedback.blocking);
            assert_eq!(feedback.journey.as_ref().unwrap().phase,InstallationPhase::AwaitingApplication);
            assert!(feedback.journey.as_ref().unwrap().plugins.is_empty());
            assert_eq!(feedback.text,"The app is installed. Open it below to install your instruments and effects.");
            assert!(!feedback.text.contains("DAW connection complete"));
        }
    }

    #[test]
    fn renderer_snapshot_releases_exact_recovery_even_with_unrelated_latest_receipt() {
        let id = "ab".repeat(32);
        let op = "cd".repeat(16);
        for (activity_first, state) in [
            (false, "vendor_running"),
            (true, "vendor_running"),
            (false, "submission_uncertain"),
            (true, "submission_uncertain"),
        ] {
            let mut f = RequestFeedback::captured(Action::RendererOpen {
                application: id.clone(),
                policy: RendererPolicy::Inherited,
            });
            f.receipt(&Receipt {
                schema: 6,
                accepted: true,
                operation: Some(op.clone()),
                refusal: None,
            });
            let mut s = running_snapshot(&"ef".repeat(16));
            let operation = serde_json::json!({"operation":op,"state":state});
            s.vendor_applications.push(VendorApplication{id:"native-access".into(),name:"Native Access".into(),version:"3.26.0".into(),state:"supervised".into(),actions:vec![AvailableAction{label:"Stop".into(),action:Action::RendererStop{operation:op.clone()},disabled_reason:None}],details:serde_json::json!({"application_identity":id,"operation":op,"requested":"inherited","manager_operation":operation})});
            if activity_first {
                f.observe(&operation);
            }
            f.reconcile_snapshot(&s);
            assert!(!f.blocking);
            assert!(!f.terminal);
            f.reconcile_snapshot(&s);
            assert!(!f.blocking);
            f.observe(&serde_json::json!({"operation":"ef".repeat(16),"state":"completed"}));
            assert!(!f.terminal);
            let mut uncertain = RequestFeedback::captured(f.action.clone());
            uncertain.receipt(&Receipt {
                schema: 6,
                accepted: true,
                operation: None,
                refusal: None,
            });
            uncertain.reconcile_snapshot(&s);
            assert_eq!(uncertain.operation, Some(op.clone()));
            assert!(!uncertain.blocking);
            let mut wrong = RequestFeedback::captured(Action::RendererOpen {
                application: "ff".repeat(32),
                policy: RendererPolicy::Inherited,
            });
            wrong.receipt(&Receipt {
                schema: 6,
                accepted: true,
                operation: None,
                refusal: None,
            });
            wrong.reconcile_snapshot(&s);
            assert!(wrong.blocking);
            assert!(wrong.operation.is_none());
        }
    }
}

#[cfg(test)]
mod is4_presentation_tests {
    #[test]
    fn requested_does_not_imply_effective_or_vendor_success() {
        let mut v = serde_json::json!({"state":"failed","raw_exit":null,"cleanup_confirmed":true,"owned_live":0,"installer_capability":{"requested":{"powershell":"intentionally_unavailable"},"effective":null}});
        let lines = super::installer_policy_lines(&v);
        assert!(lines[1].contains("not been confirmed"));
        assert!(!lines.iter().any(|s| s.contains("Applied at target launch")));
        v["installer_capability"]["effective"] = serde_json::json!({"effective":{"windows_scripting":{"powershell":"intentionally_unavailable"}}});
        assert!(super::installer_policy_lines(&v)[1].contains("not yet established"));
        assert!(super::installer_policy_lines(&serde_json::json!({})).is_empty());
    }
}

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
            format!(
                "Request accepted · {}. Waiting for its result.",
                r.operation.as_deref().unwrap_or("identity unavailable")
            )
        } else {
            format!(
                "This request was refused: {}",
                r.refusal.as_deref().unwrap_or("reason unavailable")
            )
        };
    }
    fn observe(&mut self, op: &serde_json::Value) {
        if self.terminal
            || self.operation.as_deref() != op["operation"].as_str()
            || self.operation.is_none()
        {
            return;
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
                "This operation completed. Refreshing its result…".into()
            }
            Some("waiting" | "validating") => {
                "This operation is validating manager state. Do not click again.".into()
            }
            Some("submission_uncertain") => {
                self.release_after_snapshot = true;
                "Launch acknowledgment is uncertain. Use the exact Stop / reconcile control; no launch was resubmitted.".into()
            }
            Some("vendor_running") => {
                self.release_after_snapshot = true;
                "Vendor operation is running. Refreshing its controls…".into()
            }
            Some("queued" | "running") => {
                "This operation is in progress. Do not click again.".into()
            }
            _ => return,
        };
    }
    fn refreshed(&mut self) {
        if self.release_after_snapshot {
            self.blocking = false;
            self.release_after_snapshot = false;
            if self.text == "This operation completed. Refreshing its result…" {
                self.text = "This operation completed. Its current result is shown below.".into();
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
        if let Action::RendererOpen {
            application,
            policy,
        } = self.action.clone()
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
                        renderer_recovery = self.operation.is_some()
                            && op["operation"].as_str() == self.operation.as_deref();
                        self.observe(op);
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
        } else {
            false
        };
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
    fn for_installer(&self, id: &str) -> bool {
        matches!(&self.action,Action::InstallerEnvironmentCreate{installer,..} if installer==id)
    }
}
#[derive(Default)]
struct RouteFocus {
    setup: Option<AttemptKey>,
    setup_installer: Option<String>,
    setup_scroll: bool,
    activity: Option<String>,
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
    overview: Option<InteractiveOverview>,
    product_detail: Option<CurrentProductDetail>,
    product_attempted: Option<ProductKey>,
    overview_fresh: bool,
    details_attempted: bool,
    sender: mpsc::Sender<Reply>,
    receiver: mpsc::Receiver<Reply>,
    pending: bool,
    background_poll: bool,
    action_inflight: bool,
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
    readback_failures: u32,
    retry_after: Option<Instant>,
    message: String,
    library: crate::library::Library,
    refresh_after: bool,
    product_form: Option<Action>,
    workspace_select_form: Option<Action>,
    installer_rename_form: Option<Action>,
    pending_import_route: Option<(String, bool, String)>,
    page: Page,
    focus: RouteFocus,
    preview: bool,
}
impl Operator {
    pub fn new(ctx: &egui::Context) -> Self {
        let (sender, receiver) = mpsc::channel();
        client::send(Query::Overview, sender.clone(), ctx.clone());
        Self {
            snapshot: None,
            overview: None,
            product_detail: None,
            product_attempted: None,
            overview_fresh: false,
            details_attempted: false,
            sender,
            receiver,
            pending: true,
            background_poll: false,
            action_inflight: false,
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
            readback_failures: 0,
            retry_after: None,
            message: String::new(),
            library: crate::library::Library::default(),
            refresh_after: false,
            product_form: None,
            workspace_select_form: None,
            installer_rename_form: None,
            pending_import_route: None,
            page: Page::Home,
            focus: RouteFocus::default(),
            preview: false,
        }
    }
    /// The example owns only synthetic state; it never starts the manager client.
    #[allow(dead_code)]
    pub fn preview(snapshot: Snapshot, page: Page) -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            snapshot: Some(snapshot),
            overview: None,
            product_detail: None,
            product_attempted: None,
            overview_fresh: false,
            details_attempted: false,
            sender,
            receiver,
            pending: false,
            background_poll: false,
            action_inflight: false,
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
            readback_failures: 0,
            retry_after: None,
            message: String::new(),
            library: crate::library::Library::default(),
            refresh_after: false,
            product_form: None,
            workspace_select_form: None,
            installer_rename_form: None,
            pending_import_route: None,
            page,
            focus: RouteFocus::default(),
            preview: true,
        }
    }
    /// Synthetic current-only overview for source previews. It never calls the manager.
    #[allow(dead_code)]
    pub fn preview_readiness(snapshot: Snapshot, readiness: ReadinessAssessment,
        page: Page) -> Self {
        let mut preview = Self::preview(snapshot.clone(), page);
        if page == Page::Plugins {
            // Preview the current-only product view before deep compatibility readback.
            preview.snapshot = None;
        }
        preview.overview = Some(InteractiveOverview {
            schema: 1, operator_schema: crate::model::OPERATOR_SCHEMA,
            scope: "current_only".into(), current_generation: "preview".into(),
            current: snapshot, readiness,
        });
        preview.overview_fresh = true;
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
                        ui.label("Editor interface: not retained by the original inspector");
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
        client::send(q, self.sender.clone(), ctx.clone());
    }
    fn controls_pending(&self) -> bool {
        self.queued_action.is_some()
            || self.queued_import
            || self.feedback.as_ref().is_some_and(|f| f.blocking)
            || (self.pending && !self.background_poll
                && self.origin != Some(RequestOrigin::SilentPostMutationRefresh))
    }
    fn capture_action(&mut self, request: Request) {
        self.feedback = Some(RequestFeedback::captured(request.action.clone()));
        self.queued_action = Some(request);
    }
    fn current_action_snapshot(&self) -> Option<&Snapshot> {
        self.overview.as_ref().filter(|_| self.overview_fresh).map(|o| &o.current)
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
        Some(current)
    }
    fn next_action(&mut self) -> Option<Request> {
        if self.pending {
            None
        } else {
            self.queued_action.take()
        }
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
                self.message = "The installer is in manager custody, but its exact setup or workspace is unavailable in this readback. Check again before continuing.".into();
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
            Reply::Overview(bundle) => {
                self.readback_failures = 0;
                self.retry_after = None;
                if let Some(f) = &mut self.feedback {
                    f.reconcile_snapshot(&bundle.current);
                }
                self.resolve_import_route(&bundle.current);
                if self.snapshot.as_ref().is_some_and(|old|
                    old.state_token != bundle.current.state_token) {
                    self.snapshot = None;
                    self.details_attempted = false;
                }
                self.overview = Some(*bundle);
                if self.selected_product_snapshot().is_none() {
                    self.product_detail = None;
                    self.product_attempted = None;
                }
                self.overview_fresh = true;
                self.last_overview = Instant::now();
            }
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
                    self.refresh_after = true;
                }
            }
            Reply::Snapshot(s) => {
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
            Reply::Activity(a) => {
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
                        self.refresh_after = true;
                    }
                    overview.current.system = a.system;
                    overview.current.capture = a.capture;
                    overview.current.operation = a.operation;
                }
            }
            Reply::Pulse(p) => {
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
                self.pending_import_route = Some((imported.installer_sha256,
                    imported.newly_imported, imported.display_label.clone()));
                self.feedback = None;
                self.message = format!("{} is in manager custody. Locating its exact setup or workspace…",
                    imported.display_label);
                self.refresh_after = true;
            }
            Reply::Cancelled => {
                if origin == RequestOrigin::InstallerImport {
                    self.message = "No installer selected.".into();
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

fn show_discovered_products(setup: &InstallerSetup,
    library: &mut crate::library::Library, page: &mut Page) {
    library.focus_products(setup.discovered.iter().map(|product| ProductKey {
        environment: product.environment.clone(),
        module_sha256: product.module_sha256.clone(),
        class_id: product.class_id.clone(),
    }).collect());
    *page = Page::Plugins;
}

impl Operator {
    fn readiness_overview(ui: &mut egui::Ui, overview: &InteractiveOverview,
        fresh: bool, pending: bool, refresh: &mut bool, chosen: &mut Option<Action>) {
        let assessment = &overview.readiness;
        ui.heading(match assessment.overall_status {
            ReadinessOutcome::Ready => "Ready for supported software",
            ReadinessOutcome::ActionRequired if assessment.products.is_empty() => "Setup incomplete",
            ReadinessOutcome::ActionRequired => "Action required",
            ReadinessOutcome::Unsupported => "This configuration is not supported",
            ReadinessOutcome::Unknown => "Compatibility has not been qualified",
        });
        if !fresh {
            ui.colored_label(warning_color(ui), "Checking current status. Actions are unavailable until readback completes.");
            return;
        }
        if let Some(blocker) = assessment.blockers.first() {
            ui.label(&blocker.explanation);
        } else {
            ui.label("The selected supported configuration has no current setup blocker.");
        }
        let ready = assessment.products.iter().filter(|row| row.status == ReadinessOutcome::Ready).count();
        ui.small(format!("{} of {} managed plug-ins currently ready under this exact support envelope",
            ready, assessment.products.len()));
        if let Some(step) = assessment.ordered_steps.first() {
            ui.strong(&step.title);
            ui.label(&step.detail);
            if let Some(action) = &step.action {
                let enabled = !pending && action.disabled_reason.is_none();
                if ui.add_enabled(enabled, egui::Button::new(&action.label)
                    .min_size(egui::vec2(220.0, 44.0))).clicked() {
                    *chosen = Some(action.action.clone());
                }
                if let Some(reason) = &action.disabled_reason { ui.small(reason); }
            }
        }
        if ui.add_enabled(!pending, egui::Button::new("Check again")
            .min_size(egui::vec2(150.0, 44.0))).clicked() { *refresh = true; }
        egui::CollapsingHeader::new("Details and support").show(ui, |ui| {
            for facts in [&assessment.platform, &assessment.daw, &assessment.audio,
                &assessment.graphics, &assessment.runtime] {
                for fact in facts {
                    ui.small(format!("{}: {} · {} · {:?}", fact.name,
                        fact.value.as_deref().unwrap_or("unknown"), fact.source,
                        fact.certainty));
                }
            }
            Self::buttons(ui, std::slice::from_ref(&assessment.support_export_action),
                None, pending, chosen);
            ui.small("The report is stored locally. No information is uploaded.");
        });
    }
    fn fast_home(ui: &mut egui::Ui, overview: &InteractiveOverview, fresh: bool,
        pending: bool, page: &mut Page, refresh: &mut bool, chosen: &mut Option<Action>) {
        Self::readiness_overview(ui, overview, fresh, pending, refresh, chosen);
        ui.add_space(12.0);
        ui.heading("Your managed software");
        for product in &overview.readiness.products {
            ui.group(|ui| {
                ui.strong(&product.name);
                ui.label(match product.installation_health {
                    InstallationHealth::Healthy => "Current installation: healthy",
                    InstallationHealth::ActionRequired => "Current installation: needs repair",
                    InstallationHealth::Unknown => "Current installation: not verified",
                });
                ui.label(match product.support_qualification {
                    SupportQualification::Verified => "Beta support: exact profile verified",
                    SupportQualification::Unsupported => "Beta support: exact profile withdrawn",
                    SupportQualification::NotYetQualified => "Beta support: not yet qualified",
                });
                ui.small(&product.reason);
            });
        }
        if overview.readiness.products.is_empty() { ui.label("No managed native plug-ins yet."); }
        ui.add_space(8.0);
        if ui.button("Open Setup").clicked() { *page = Page::Setup; }
        if ui.button("Technical details and history").clicked() { *page = Page::Diagnostics; }
    }
    fn fast_setup(ui: &mut egui::Ui, overview: &InteractiveOverview, fresh: bool,
        pending: bool, controls: (&mut bool, &mut Option<Action>, &mut bool,
            &mut Page, &mut crate::library::Library, &mut RouteFocus)) {
        let (refresh, chosen, pick, page, library, focus) = controls;
        Self::readiness_overview(ui, overview, fresh, pending, refresh, chosen);
        ui.separator();
        ui.heading("Setup");
        let runtime_actions: Vec<_> = overview.current.actions.iter()
            .filter(|offer| matches!(offer.action, Action::RuntimeInstall {})).cloned().collect();
        Self::buttons(ui, &runtime_actions,
            (!fresh).then_some("Current manager readback unavailable"), pending, chosen);
        ui.small("Compatibility runtime setup downloads verified components into this application's storage. Steam and development tools are not required.");
        if ui.add_enabled(fresh && !pending, egui::Button::new("Choose Windows installer")
            .min_size(egui::vec2(240.0, 44.0))).clicked() { *pick = true; }
        ui.small("Choose an installer for manager custody. Vendor sign-in remains yours.");
        if focus.setup_installer.is_some() {
            ui.strong("Showing the selected installer");
            if ui.add_sized([180.0, 44.0], egui::Button::new("Show all setup")).clicked() {
                focus.setup_installer = None;
                focus.setup = None;
                focus.setup_scroll = false;
            }
        }
        let mut shown = 0;
        for setup in overview.current.installer_setups.iter().filter(|setup|
            focus.setup_installer.as_ref().is_none_or(|id| id == &setup.installer)) {
            shown += 1;
            if focus.setup_scroll && focus.setup_installer.as_ref() == Some(&setup.installer) {
                ui.scroll_to_cursor(Some(egui::Align::Center));
                focus.setup_scroll = false;
            }
            ui.push_id(&setup.installer, |ui| {
            ui.group(|ui| {
                ui.strong(&setup.name);
                ui.label(&setup.status);
                if let Some(compatibility) = &setup.compatibility { ui.small(compatibility); }
                if let Some(primary) = &setup.primary {
                    Self::buttons(ui, std::slice::from_ref(primary),
                        (!fresh).then_some("Current manager readback unavailable"), pending, chosen);
                }
                if !setup.discovered.is_empty() {
                    ui.small(format!("{} exact plug-in product(s) discovered", setup.discovered.len()));
                    if ui.add_enabled(fresh, egui::Button::new("View discovered plug-ins")
                        .min_size(egui::vec2(240.0, 44.0))).clicked() {
                        show_discovered_products(setup, library, page);
                    }
                }
                Self::buttons(ui, &setup.secondary,
                    (!fresh).then_some("Current manager readback unavailable"), pending, chosen);
                Self::buttons(ui, std::slice::from_ref(&setup.rename),
                    (!fresh).then_some("Current manager readback unavailable"), pending, chosen);
            });
            });
        }
        if overview.current.installer_setups.is_empty() {
            ui.label("No native plug-in setup is in progress.");
        } else if shown == 0 {
            ui.colored_label(warning_color(ui),
                "The selected installer is unavailable in this readback. Check again or show all setup.");
        }
        if ui.button("Technical setup history").clicked() { *page = Page::Diagnostics; }
    }
    fn health_bar(ui: &mut egui::Ui, system: &System) {
        egui::Frame::group(ui.style())
            .fill(ui.visuals().faint_bg_color)
            .show(ui, |ui| {
                ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                ui.spacing_mut().item_spacing.y = 3.0;
                let state = presentation::health(system);
                let color = if matches!(
                    state,
                    presentation::Health::Unavailable | presentation::Health::NeedsAttention
                ) {
                    warning_color(ui)
                } else {
                    ui.visuals().text_color()
                };
                ui.label(
                    egui::RichText::new(state.title())
                        .size(17.0)
                        .strong()
                        .color(color),
                );
                ui.horizontal_wrapped(|ui| {
                    let label = |text: String| egui::RichText::new(text).size(13.0);
                    ui.label(label(if system.capacity_available() {
                        "Service ready".into()
                    } else {
                        "Service readback unavailable".into()
                    }));
                    if system.capacity_available() {
                        ui.label(label(format!("DSP {} / {}", system.dsp, system.ceiling)));
                        ui.label(label(format!("Install / scan {}", system.maintenance)));
                        ui.label(label(format!(
                            "Stale transports {}",
                            system.stale_transports
                        )));
                    } else {
                        ui.label(label("DSP and capacity unknown".into()));
                        ui.label(label("Install / scan unknown".into()));
                        ui.label(label("Stale transports unknown".into()));
                    }
                    ui.label(label(format!(
                        "Pending transactions {}",
                        system.pending_transactions
                    )));
                    ui.label(label(
                        presentation::activity_certainty(system)
                            .cleanup_label()
                            .into(),
                    ));
                });
                ui.small("State updates automatically.");
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
                        .map_or(if foreground_pending && self.message.is_empty() { "Working…" } else { self.message.as_str() }, |feedback| feedback.text.as_str()),
                );
            });
            if let Some(feedback) = &self.feedback {
                egui::CollapsingHeader::new("Request details").show(ui, |ui| {
                    ui.label(
                        feedback
                            .operation
                            .as_deref()
                            .unwrap_or("Operation identity not yet confirmed"),
                    );
                    ui.label(feedback.transitions.join(" → "));
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

    fn home_products(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        page: &mut Page,
        library: &mut crate::library::Library,
    ) {
        ui.heading("Published plug-ins");
        ui.small("Manager publication is current. Bitwig may still need its own browser rescan.");
        let mut count = 0;
        for product in snapshot
            .products
            .iter()
            .filter(|product| matches!(product.disposition.as_str(), "ready" | "experimental"))
            .take(6)
        {
            count += 1;
            let activity = presentation::product_activity(snapshot, product);
            let label = format!(
                "{}  ·  {}  ·  {}",
                product.name,
                crate::library::status(product).0,
                activity.home_label()
            );
            if ui
                .add_sized([ui.available_width(), 46.0], egui::Button::new(label))
                .clicked()
            {
                library.focus_product(ProductKey::from(product));
                *page = Page::Plugins;
            }
        }
        if count == 0 {
            ui.label("No plug-ins are currently published by the manager.");
        }
        if ui
            .add_sized([180.0, 44.0], egui::Button::new("Browse all plug-ins"))
            .clicked()
        {
            *page = Page::Plugins;
        }
    }

    fn home_live(ui: &mut egui::Ui, snapshot: &Snapshot, page: &mut Page) {
        ui.heading("Running now");
        match presentation::activity_certainty(&snapshot.system) {
            presentation::ActivityCertainty::Unavailable => {
                ui.label("Live session status cannot be confirmed.");
            }
            presentation::ActivityCertainty::CleanupUncertain => {
                ui.colored_label(
                    warning_color(ui),
                    "Cleanup is unconfirmed. Retained owners are not proof of live audio.",
                );
            }
            presentation::ActivityCertainty::Confirmed => {
                let running: Vec<_> = snapshot
                    .active_sessions
                    .iter()
                    .filter(|row| row["recent"] != true && row["state"] == "active")
                    .collect();
                if running.is_empty() {
                    ui.label("No bridge sessions are running.");
                } else {
                    for row in running.iter().take(3) {
                        let class_id = row["class_id"].as_str().unwrap_or("");
                        ui.label(presentation::session_display_name(snapshot, class_id));
                    }
                    if running.len() > 3 {
                        ui.small(format!("And {} more", running.len() - 3));
                    }
                }
            }
        }
        if ui
            .add_sized([180.0, 44.0], egui::Button::new("Open Activity"))
            .clicked()
        {
            *page = Page::Activity;
        }
    }

    fn home_operation(ui: &mut egui::Ui, snapshot: &Snapshot, page: &mut Page) {
        ui.heading("Setup and operations");
        let active_state = snapshot
            .operation
            .as_ref()
            .and_then(|operation| operation["state"].as_str())
            .filter(|state| {
                matches!(
                    *state,
                    "queued"
                        | "waiting"
                        | "validating"
                        | "running"
                        | "vendor_running"
                        | "submission_uncertain"
                )
            });
        let active_attempt = snapshot.onboarding.iter().find(|attempt| {
            matches!(
                attempt.state.as_str(),
                "queued"
                    | "waiting"
                    | "running"
                    | "vendor_running"
                    | "submission_uncertain"
                    | "cleanup_unconfirmed"
            )
        });
        if let Some(state) = active_state {
            ui.label(format!("Manager operation: {}", state.replace('_', " ")));
        }
        if let Some(attempt) = active_attempt {
            ui.label(format!(
                "{}: {}",
                attempt.name,
                attempt.state.replace('_', " ")
            ));
        }
        if snapshot.system.maintenance > 0 {
            ui.label(format!(
                "{} installation or scan owner(s) active",
                snapshot.system.maintenance
            ));
        }
        if active_state.is_none() && active_attempt.is_none() && snapshot.system.maintenance == 0 {
            ui.label("No installation or manager operation is in progress.");
        }
        let awaiting = snapshot
            .onboarding
            .iter()
            .filter(|attempt| matches!(attempt.state.as_str(), "imported" | "environment_ready"))
            .count();
        if awaiting > 0 {
            ui.small(format!(
                "{awaiting} installer(s) ready for your next setup step"
            ));
        }
        if ui
            .add_sized([180.0, 44.0], egui::Button::new("Open Setup"))
            .clicked()
        {
            *page = Page::Setup;
        }
    }

    fn home(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        page: &mut Page,
        library: &mut crate::library::Library,
        focus: &mut RouteFocus,
    ) {
        ui.heading("Home");
        ui.small("Your bridge at a glance. Closing this manager does not stop audio or vendor applications.");
        if ui.available_width() >= 760.0 {
            ui.columns(2, |columns| {
                Self::home_products(&mut columns[0], snapshot, page, library);
                columns[1].heading("Needs attention");
                Self::attention_list(&mut columns[1], snapshot, 3, page, library, focus);
            });
            ui.add_space(12.0);
            ui.columns(2, |columns| {
                Self::home_live(&mut columns[0], snapshot, page);
                Self::home_operation(&mut columns[1], snapshot, page);
            });
        } else {
            let published = snapshot
                .products
                .iter()
                .filter(|product| matches!(product.disposition.as_str(), "ready" | "experimental"))
                .count();
            let items = presentation::attentions(snapshot);
            let attention = items.len();
            ui.small(format!(
                "{published} published plug-in(s) · {attention} item(s) need attention"
            ));
            if let Some(first) = items.first() {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(format!("Needs attention: {}", first.title));
                    if ui
                        .add_sized([124.0, 44.0], egui::Button::new("Review item"))
                        .clicked()
                    {
                        navigate(&first.destination, page, library, focus);
                    }
                });
            }
            ui.separator();
            Self::home_products(ui, snapshot, page, library);
            if attention > 1 {
                ui.separator();
                if ui
                    .add_sized(
                        [220.0, 44.0],
                        egui::Button::new(format!("See all {attention} attention items")),
                    )
                    .clicked()
                {
                    *page = Page::Activity;
                }
            }
            ui.separator();
            Self::home_live(ui, snapshot, page);
            ui.separator();
            Self::home_operation(ui, snapshot, page);
        }
    }
}

impl Operator {
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

    fn setup(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        pending: bool,
        feedback: Option<&RequestFeedback>,
        navigation: (&mut Page, &mut crate::library::Library, &mut RouteFocus),
        chosen: &mut Option<Action>,
        pick: &mut bool,
    ) {
        let (page, library, focus) = navigation;
        let busy = snapshot.system.inactive_reason();
        ui.heading("Setup");
        ui.small("Installers, vendor applications, environments and exact recovery controls.");
        for offer in &snapshot.actions {
            if matches!(offer.action, Action::TransactionReconcile {})
                && snapshot.system.pending_transactions > 0
            {
                Self::buttons(ui, std::slice::from_ref(offer), busy, pending, chosen);
            }
        }
        ui.group(|ui| {
            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
            ui.strong("Add a plug-in");
            if ui.add_enabled(!pending, egui::Button::new("Choose Windows installer")
                .min_size(egui::vec2(240.0, 48.0))).clicked() { *pick = true; }
            ui.small("Selecting a file copies it into manager custody. It does not run the installer.");
        });
        if focus.setup_installer.is_some() {
            ui.strong("Showing the selected installer");
            if ui.add_sized([180.0, 44.0], egui::Button::new("Show all setup"))
                .clicked() { focus.setup_installer = None; focus.setup = None; }
        }
        let sections = [
            ("Ready to set up", 0),
            ("Ready to install", 1),
            ("In progress / Needs attention", 2),
            ("Installed plug-ins / Ready to continue", 3),
        ];
        for (heading, section) in sections {
            let items: Vec<_> = snapshot.installer_setups.iter().filter(|setup| {
                focus.setup_installer.as_ref().is_none_or(|id| id == &setup.installer)
                    && matches!((section, &setup.phase),
                        (0, crate::model::SetupPhase::Imported)
                        | (1, crate::model::SetupPhase::EnvironmentReady)
                        | (2, crate::model::SetupPhase::InstallerRunning
                            | crate::model::SetupPhase::SetupNeedsAttention
                            | crate::model::SetupPhase::CleanupUnconfirmed)
                        | (3, crate::model::SetupPhase::InstallerRetired
                            | crate::model::SetupPhase::ScanReady
                            | crate::model::SetupPhase::DiscoveryComplete))
            }).collect();
            if items.is_empty() { continue; }
            ui.add_space(12.0);
            ui.heading(heading);
            for setup in items {
                ui.push_id(&setup.installer, |ui| {
                    if focus.setup_scroll && focus.setup_installer.as_ref() == Some(&setup.installer) {
                        ui.scroll_to_cursor(Some(egui::Align::Center));
                        focus.setup_scroll = false;
                    }
                    egui::Frame::group(ui.style()).inner_margin(14.0).show(ui, |ui| {
                        ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                        ui.label(egui::RichText::new(&setup.name).size(19.0).strong());
                        let phase = match setup.phase {
                            crate::model::SetupPhase::Imported => "Imported",
                            crate::model::SetupPhase::EnvironmentReady => "Ready to install",
                            crate::model::SetupPhase::InstallerRunning => "Installer open",
                            crate::model::SetupPhase::SetupNeedsAttention => "Setup needs attention",
                            crate::model::SetupPhase::InstallerRetired => "Installer closed",
                            crate::model::SetupPhase::ScanReady => "Ready to find plug-ins",
                            crate::model::SetupPhase::DiscoveryComplete => "Plug-ins found",
                            crate::model::SetupPhase::CleanupUnconfirmed => "Cleanup unresolved",
                        };
                        ui.strong(phase);
                        ui.label(&setup.status);
                        if let Some(target) = focus.setup.as_ref().filter(|key| key.installer == setup.installer) {
                            ui.small(format!("Attention is linked to setup attempt {}",
                                target.environment.as_deref().unwrap_or("before environment creation")));
                        }
                        if let Some(compatibility) = &setup.compatibility {
                            ui.small(compatibility);
                        }
                        if let Some(offer) = &setup.primary {
                            let reason = offer.disabled_reason.as_deref().or(busy);
                            let response = ui.add_enabled(!pending && reason.is_none(),
                                egui::Button::new(egui::RichText::new(&offer.label).strong())
                                    .min_size(egui::vec2(240.0, 48.0)));
                            if response.clicked() { *chosen = Some(offer.action.clone()); }
                            if let Some(reason) = reason { ui.small(reason); }
                        } else if matches!(setup.phase, crate::model::SetupPhase::Imported) {
                            ui.colored_label(warning_color(ui),
                                "Standard compatibility runtime is unavailable. Repair or update the manager package before continuing.");
                        }
                        if !setup.discovered.is_empty()
                            && ui.add_sized([240.0, 48.0], egui::Button::new("View plug-in details")).clicked() {
                            show_discovered_products(setup, library, page);
                        }
                        Self::buttons(ui, &setup.secondary, None, pending, chosen);
                        if ui.add_enabled(!pending, egui::Button::new("Rename")
                            .min_size(egui::vec2(120.0, 44.0))).clicked() {
                            *chosen = Some(Action::InstallerRename {
                                installer: setup.installer.clone(), label: setup.name.clone() });
                        }
                        if let Some(feedback) = feedback.filter(|f| f.for_installer(&setup.installer)) {
                            ui.small(&feedback.text);
                        }
                        egui::CollapsingHeader::new("Technical details and setup history")
                            .default_open(focus.setup.as_ref().is_some_and(|key| key.installer == setup.installer))
                            .show(ui, |ui| {
                                ui.small(format!("Installer SHA-256: {}", setup.installer));
                                ui.small(format!("{} bytes · {} · label source {}", setup.byte_size,
                                    setup.format, setup.label_source));
                                ui.small(format!("Imported at: {}", setup.imported_at));
                                if let Some(environment) = &setup.environment {
                                    ui.small(format!("Environment: {environment}"));
                                }
                                for reference in &setup.history {
                                    let Some(item) = snapshot.onboarding.iter().find(|item|
                                        item.installer == setup.installer
                                            && item.environment == reference.environment) else {
                                        ui.small("A retained setup row is unavailable in this readback.");
                                        continue;
                                    };
                                    ui.separator();
                                    ui.strong(format!("{} · {}", item.environment.as_deref()
                                        .unwrap_or("Imported artifact"), item.state.replace('_', " ")));
                                    for line in installer_policy_lines(&item.details["installation"]) { ui.small(line); }
                                    for line in installer_lines(&item.details["installation"]) { ui.small(line); }
                                    if let Some(failure) = &item.failure {
                                        for line in failure_lines(failure) { ui.small(line); }
                                    }
                                    Self::value(ui, &item.details);
                                    Self::buttons(ui, &item.actions, busy, pending, chosen);
                                }
                            });
                    });
                });
            }
        }
        if snapshot.installer_setups.is_empty() { ui.label("No plug-in installers added yet."); }
        ui.add_space(10.0);
        egui::CollapsingHeader::new("System setup and recovery").show(ui, |ui| {
            for offer in &snapshot.actions {
                if matches!(offer.action, Action::TransactionReconcile {})
                    && snapshot.system.pending_transactions > 0 {
                    Self::buttons(ui, std::slice::from_ref(offer), busy, pending, chosen);
                }
            }
        });
        ui.separator();
        egui::CollapsingHeader::new("Vendor applications").show(ui, |ui| {
            for app in &snapshot.vendor_applications {
                ui.push_id(&app.id, |ui| {
                    ui.group(|ui| {
                        ui.strong(&app.name);
                        ui.label(format!("{} · {}", app.version, app.state.replace('_', " ")));
                        Self::buttons(ui, &app.actions, busy, pending, chosen);
                        egui::CollapsingHeader::new("Technical application details").show(ui, |ui| {
                            if app.id == "native-access" {
                                ui.label(if app.details["dependency_prepared"] == true { "Historical dependency preparation retained; each launch still requires exact session admission and fresh readiness" } else { "Historical dependency preparation is absent; launch uses exact session admission and fresh readiness" });
                                ui.label(if app.details["effective"].is_object() { "Renderer policy applied to the exact owned application" } else { "Renderer policy application not confirmed" });
                                ui.label(format!("Rendering cause: {}", app.details["renderer"]["cause"].as_str().unwrap_or("unresolved")));
                                for line in dependency_retirement_lines(&app.details["dependency_operation"]) { ui.small(line); }
                                if app.details["dependency"].is_object() { for line in dependency_retirement_lines(&app.details) { ui.small(line); } }
                            }
                            Self::value(ui, &app.details);
                        });
                    });
                });
            }
        });
        egui::CollapsingHeader::new("Managed environments").show(ui, |ui| {
            for environment in &snapshot.environments {
                ui.push_id(&environment.id, |ui| {
                    ui.group(|ui| {
                        ui.strong(&environment.family);
                        ui.label(format!(
                            "Revision {} · pinned runner {}",
                            environment.revision, environment.runner
                        ));
                        Self::buttons(ui, &environment.actions, busy, pending, chosen);
                        egui::CollapsingHeader::new("Technical environment details").show(
                            ui,
                            |ui| {
                                ui.small(&environment.authorization);
                                ui.small(format!("Environment: {}", environment.id));
                                Self::value(ui, &environment.last_scan);
                            },
                        );
                    });
                });
            }
        });
        ui.small("Closing the manager does not stop an installer or vendor application. Sign-in and authorization stay in the vendor interface.");
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
        egui::CollapsingHeader::new("Technical system details").show(ui, |ui| {
            if let Ok(value) = serde_json::to_value(&snapshot.system) {
                Self::value(ui, &value);
            }
            Self::value(ui, &snapshot.capture);
        });
    }

    fn workspaces(
        ui: &mut egui::Ui,
        snapshot: &Snapshot,
        pending: bool,
        chosen: &mut Option<Action>,
        pick: &mut bool,
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
            ui.label(format!("Revision {} · {}{}{}", history.revision, history.claim,
                if history.active { " · active" } else { "" },
                if history.rollback_allowed && !history.active { " · rollback available" } else { "" }));
        }
        egui::CollapsingHeader::new("Technical product details").show(ui, |ui| {
            ui.label(format!("Status: {}\nClass: {}\nModule SHA-256: {}\nEnvironment: {}\nRunner: {}",
                product.disposition, product.class_id, product.module_sha256,
                product.environment, product.runner));
            Self::value(ui, &product.details);
        });
    }
    fn repaint_delay(&self) -> Duration {
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

        let controls_pending = self.controls_pending();
        let action_controls_pending = controls_pending || (!self.preview && !self.overview_fresh);
        let mut refresh = false;
        let mut pick = false;
        let mut chosen = None;
        let mut retry_details = false;
        let selected_product_snapshot = self.selected_product_snapshot();
        egui::CentralPanel::default().show(ui, |ui| {
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
            if self.preview { ui.small("LOCAL DESIGN PREVIEW · synthetic records · actions do not execute"); }
            if let Some(snapshot) = self.overview.as_ref().map(|o| &o.current)
                .or(self.snapshot.as_ref()) {
                Self::health_bar(ui, &snapshot.system);
            }
            navigation(ui, &mut self.page);
            self.request_bar(ui);
            ui.separator();
            egui::ScrollArea::vertical().id_salt(("manager-page", self.page)).show(ui, |ui| {
                if self.overview.is_some() || !self.preview {
                    if let Some(overview) = &self.overview {
                        match self.page {
                            Page::Home => {
                                Self::fast_home(ui, overview, self.overview_fresh,
                                    controls_pending, &mut self.page, &mut refresh, &mut chosen);
                                return;
                            }
                            Page::Setup => {
                                Self::fast_setup(ui, overview, self.overview_fresh,
                                    controls_pending, (&mut refresh, &mut chosen, &mut pick,
                                        &mut self.page, &mut self.library, &mut self.focus));
                                return;
                            }
                            Page::Plugins => {
                                if let Some(selected) = &selected_product_snapshot {
                                    if !self.overview_fresh {
                                        ui.colored_label(warning_color(ui),
                                            "Refreshing current status. Actions remain unavailable until readback completes.");
                                    }
                                    self.library.show(ui, selected, action_controls_pending,
                                        &mut chosen, Self::product_details);
                                } else {
                                    let attempted = self.library.focused_product().is_some_and(|key|
                                        self.product_attempted.as_ref() == Some(key));
                                    self.library.show_current(ui, &overview.current,
                                        self.overview_fresh, attempted && !self.pending,
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
                    Page::Home => Self::home(ui, snapshot, &mut self.page, &mut self.library, &mut self.focus),
                    Page::Plugins => self.library.show(ui, snapshot,
                        action_controls_pending, &mut chosen, Self::product_details),
                    Page::Workspaces => Self::workspaces(ui, snapshot, action_controls_pending, &mut chosen, &mut pick),
                    Page::Activity => Self::activity(ui, snapshot, action_controls_pending, &mut self.page,
                        &mut self.library, &mut self.focus, &mut chosen),
                    Page::Setup => Self::setup(ui, snapshot, action_controls_pending, self.feedback.as_ref(),
                        (&mut self.page, &mut self.library, &mut self.focus), &mut chosen, &mut pick),
                    Page::Diagnostics => Self::diagnostics(ui, snapshot, action_controls_pending, &mut chosen),
                }
            });
        });
        if let Some(a) = chosen.take() {
            if matches!(
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
            } else {
                chosen = Some(a);
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
                    submit = ui.add_enabled(!release.is_empty() && !controls_pending, egui::Button::new("Select version").min_size(egui::vec2(180.0, 44.0))).clicked();
                    cancel = ui.add_sized([100.0, 44.0], egui::Button::new("Cancel")).clicked();
                });
            if submit {
                chosen = self.workspace_select_form.take();
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
                submit = ui.add_enabled(plausible && !controls_pending,
                    egui::Button::new("Save name").min_size(egui::vec2(180.0, 44.0))).clicked();
                cancel = ui.add_sized([100.0, 44.0], egui::Button::new("Cancel")).clicked();
            });
            if submit { chosen = self.installer_rename_form.take(); }
            else if cancel { self.installer_rename_form = None; }
        }
        let result_inactive_reason = self.current_action_snapshot().or(self.snapshot.as_ref())
            .map_or(Some("Current manager readback unavailable"), |snapshot|
                snapshot.system.inactive_reason());
        if let Some(form) = self.product_form.as_mut() {
            let mut submit = false;
            let mut cancel = false;
            egui::Window::new(if matches!(form, Action::CompatibilityResult { .. }) {
                "Record plug-in test result"
            } else { "Exact candidate observation / review" })
                .collapsible(false)
                .max_height((ui.ctx().content_rect().height() - 40.0).max(200.0))
                .max_width((ui.ctx().content_rect().width() - 40.0).max(300.0))
                .vscroll(true)
                .show(ui.ctx(),|ui|{
                match form {
                    Action::CompatibilityResult { result, passed, failed_area, note, .. } => {
                        ui.label("Mark only what you actually observed. The test configuration stays experimental after a successful result.");
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
                        let success_blocked = matches!(result, TestResultKind::Worked)
                            && result_inactive_reason.is_some();
                        submit = ui.add_enabled(valid && !controls_pending && !success_blocked,
                            egui::Button::new("Record this test result")
                                .min_size(egui::vec2(240.0, 44.0))).clicked();
                        if !valid { ui.small("Select an observed check, or choose a problem and describe it."); }
                        if success_blocked {
                            ui.small(result_inactive_reason.unwrap_or_default());
                            ui.small("You can report a problem now. A successful result needs clean retirement.");
                        }
                    }
                    Action::CandidateObserve{area,status,note,..}=>{
                        ui.label("This records your observation. It does not create a machine measurement or qualification decision.");
                        egui::ComboBox::from_id_salt("area").selected_text(area.as_str()).show_ui(ui,|ui|{for a in ["daw_load","midi","audio","editor","parameters","automation","state_recall","processing_restart","retirement"]{ui.selectable_value(area,a.into(),a.replace('_'," "));}});
                        egui::ComboBox::from_id_salt("status").selected_text(status.as_str()).show_ui(ui,|ui|{for a in ["passed","failed","not_tested","unavailable","not_applicable"]{ui.selectable_value(status,a.into(),a.replace('_'," "));}});
                        ui.add(egui::TextEdit::multiline(note).char_limit(512));
                        submit=ui.add_enabled(!note.trim().is_empty(),egui::Button::new("Record operator observation")).clicked();
                    }
                    Action::CandidateReview{accept,rationale,..}=>{
                        ui.label(if *accept{"Maintainer review: accept this exact local configuration against the retained results. This is not a project-wide support claim. Publication remains a separate action."}else{"Record why this exact configuration needs more work. Publication is unchanged."});
                        ui.add(egui::TextEdit::multiline(rationale).char_limit(512));
                        submit=ui.add_enabled(!rationale.trim().is_empty(),egui::Button::new("Record explicit review decision")).clicked();
                    }
                    _=>cancel=true,
                }
                cancel|=ui.button("Cancel").clicked();
            });
            if submit {
                chosen = self.product_form.take();
            } else if cancel {
                self.product_form = None;
            }
        }
        if self.preview {
            ui.ctx().request_repaint_after(Duration::from_millis(500));
            return;
        }
        if retry_details {
            if self.page == Page::Plugins { self.product_attempted = None; }
            else { self.details_attempted = false; }
        }
        if pick { self.queued_import = true; }
        if refresh { self.explicit_refresh_queued = true; }
        if let Some(a) = chosen {
            if let Some(s) = self.current_action_snapshot() {
                self.capture_action(Request {
                    schema: crate::model::OPERATOR_SCHEMA,
                    state_token: s.state_token.clone(),
                    action: a,
                });
            }
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
            self.explicit_refresh_queued = false;
            self.refresh_after = false;
            self.request(Query::Overview, origin, ui.ctx());
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
fn installer_lines(v: &serde_json::Value) -> Vec<String> {
    let t = &v["transaction"];
    if t["schema"] != 1 || t["operation"] != v["operation"] {
        return if v["error"] == "installer_launcher_failed" {
            vec!["Earlier installer result: outer launch route exited nonzero; the failing later stage and installation completeness were not captured.".into()]
        } else {
            vec![]
        };
    }
    let outcome=match t["outcome"].as_str() {
        Some("in_progress")=>"Installer supervision is ongoing. Exact Focus and Stop controls remain available; cleanup will be checked after retirement.",
        Some("outer_nonzero_stage_unknown")=>"The outer installer route exited nonzero. The failing child or stage is not established.",
        Some("installed_dependency_failed")=>"Application files are installed, but a process performing service/dependency work exited nonzero. Review that stage before reinstalling.",
        Some("child_failed")=>"An owned child exited nonzero. Its role and underlying cause may still be unknown.",
        Some("cancelled")=>"This attempt was cancelled. Earlier failure observations remain retained.",
        Some("cleanup_unconfirmed")=>"Installer cleanup is unconfirmed. Further work is blocked.",
        Some("installed")=>"Application files and installation registration were observed. First launch and dependency health remain unproved.",
        Some("partial_installation")=>"Partial installation: durable changes exist, but a complete application installation is not established.",
        Some("not_installed")=>"No durable installation was found in the inspected surfaces.",
        _=>"Review the installer stage record; completion does not qualify or publish a plug-in.",
    };
    let mut lines = vec![outcome.into()];
    if let Some(code) = v["startup"]["first_problem"]["code"].as_str() {
        let observation = match code {
            "native_steamclient_load_failed" => "native runtime dependency load failed",
            "native_steamclient_export_unavailable" => "native runtime export unavailable",
            "runtime_assertion_observed" => "runtime assertion observed",
            "prefix_initialization_failed" | "prefix_initialization_timeout" => {
                "environment initialization did not complete"
            }
            _ => "startup problem retained; inspect bounded details",
        };
        lines.push(format!(
            "Earlier startup observation: {observation}. Cancellation and cleanup do not erase it."
        ));
    }
    if let Some(n) = t["outer_launcher_exit"].as_i64() {
        lines.push(format!(
            "Outer launcher exit: {n} (separate from payload and service exits)"
        ));
    }
    lines.push(format!(
        "Durable installation: {}",
        t["durable_installation"]
            .as_str()
            .unwrap_or("unavailable")
            .replace('_', " ")
    ));
    if !t["first_failure"].is_null() {
        let f = &t["first_failure"];
        lines.push(format!("First retained process result: phase {} · role {} · relationship {} · domain {} · status {} · cause unestablished",
            f["phase"].as_str().unwrap_or("unknown"), f["role"].as_str().unwrap_or("unknown"),
            f["relationship"].as_str().unwrap_or("unknown"), f["domain"].as_str().unwrap_or("unknown"), f["status"]));
    }
    let binding = &t["launch_binding"];
    if binding["schema"] == 1 && binding["operation"] == v["operation"] {
        lines.push(if binding["status"] == "bound" {
            "Installer Windows root: exact operation and launch generation observed.".into()
        } else {
            format!(
                "Installer Windows root unavailable: {}. Child attribution may be incomplete.",
                binding["reason"].as_str().unwrap_or("missing observation")
            )
        });
    }
    if t["presence_close"]["schema"] == 1 {
        lines.push(format!("Application presence/close requests observed: {}. Helper success does not prove a match or successful closure; exact match and recheck results remain unavailable.",t["presence_close"]["observation_count"]));
        if t["presence_close"]["operation_classes"]
            .as_array()
            .is_some_and(|rows| {
                rows.iter()
                    .any(|r| r == "presence_query" || r == "close_request")
            })
        {
            lines.push("Observed request mechanism: PowerShell/CIM process query or script process close. The matched object and actual close outcome are unavailable.".into());
        }
    }
    lines.push(
        if v["cleanup_confirmed"] == true {
            "Owned process cleanup confirmed."
        } else {
            "Owned process cleanup not yet confirmed."
        }
        .into(),
    );
    lines
}

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
            product_detail: None,
            product_attempted: None,
            overview_fresh: false,
            details_attempted: false,
            sender,
            receiver,
            pending: false,
            background_poll: false,
            action_inflight: false,
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
            readback_failures: 0,
            retry_after: None,
            message: String::new(),
            library: crate::library::Library::default(),
            refresh_after: false,
            product_form: None,
            workspace_select_form: None,
            installer_rename_form: None,
            pending_import_route: None,
            page: Page::Home,
            focus: RouteFocus::default(),
            preview: false,
        }
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
    #[test]
    fn retained_product_card_requires_matching_identity_and_fresh_actions() {
        let mut state = state_fixture();
        let overview = overview_fixture();
        let product = overview.current.products[0].clone();
        let key = ProductKey::from(&product);
        state.library.focus_product(key);
        state.overview_fresh = true;
        state.overview = Some(overview.clone());
        let mut detailed = product;
        detailed.name = "Exact scoped detail".into();
        state.product_detail = Some(CurrentProductDetail {
            schema: 1, operator_schema: crate::model::OPERATOR_SCHEMA,
            state_token: overview.current.state_token.clone(),
            current_generation: overview.current_generation.clone(), product: detailed,
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
    #[test]
    fn suspended_service_does_not_repeatedly_disable_owned_installer_controls() {
        let mut operator = state_fixture();
        let mut overview = overview_fixture();
        overview.current = running_snapshot("current-op");
        overview.current.system.service = "capacity unavailable".into();
        let mut pulse = pulse_fixture(true);
        pulse.service_state = "inactive".into();
        pulse.cleanup_unconfirmed = None;
        operator.overview = Some(overview.clone());
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
        operator.overview=Some(overview_fixture());
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
        operator.overview=Some(overview_fixture());
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
        operator.overview=Some(overview_fixture());
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
        operator.overview=Some(overview_fixture());
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
                disabled_reason: None }, history: vec![],
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
    fn setup_identity_status_and_primary_action_render_at_both_widths() {
        fn texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
            match shape {
                egui::epaint::Shape::Text(text) => out.push(text.galley.text().into()),
                egui::epaint::Shape::Vec(shapes) => {
                    for shape in shapes { texts(shape, out); }
                }
                _ => {}
            }
        }
        let mut snapshot: Snapshot =
            serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap();
        let installer = "ab".repeat(32);
        snapshot.installer_setups.push(crate::model::InstallerSetup {
            installer: installer.clone(), name: "Lunacy Audio".into(),
            label_source: "operator_named".into(), byte_size: 1024,
            format: "pe_executable".into(), imported_at: 1,
            phase: crate::model::SetupPhase::Imported,
            status: "No native plug-in setup has started from this installer.".into(), environment: None,
            compatibility: Some("Recommended setup: Standard".into()), discovered: vec![],
            primary: Some(AvailableAction { label: "Continue setup".into(),
                action: Action::InstallerEnvironmentCreate {
                    installer: installer.clone(), runner: "exact-default-key".into() },
                disabled_reason: None }), secondary: vec![],
            rename: AvailableAction { label: "Rename".into(),
                action: Action::InstallerRename { installer, label: String::new() },
                disabled_reason: None }, history: vec![],
        });
        for width in [960.0, 560.0] {
            let ctx = egui::Context::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.set_max_width(width - 32.0);
                Operator::setup(ui, &snapshot, false, None,
                    (&mut Page::Setup, &mut crate::library::Library::default(),
                        &mut RouteFocus::default()), &mut None, &mut false);
            });
            let mut labels = Vec::new();
            for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
            output.textures_delta.clear();
            for expected in ["Add a plug-in", "Ready to set up", "Lunacy Audio",
                "No native plug-in setup has started", "Continue setup", "Rename",
                "Recommended setup: Standard", "Technical details and setup history"] {
                assert!(labels.iter().any(|label| label.contains(expected)),
                    "missing {expected} at width {width}: {labels:?}");
            }
        }
        snapshot.installer_setups[0].phase = crate::model::SetupPhase::EnvironmentReady;
        snapshot.installer_setups[0].environment = Some("cd".repeat(16));
        snapshot.installer_setups[0].compatibility = Some("Existing managed configuration".into());
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            Operator::setup(ui, &snapshot, false, None,
                (&mut Page::Setup, &mut crate::library::Library::default(),
                    &mut RouteFocus::default()), &mut None, &mut false);
        });
        let mut labels = Vec::new();
        for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
        output.textures_delta.clear();
        assert!(labels.iter().any(|label| label.contains("Existing managed configuration")));
        assert!(!labels.iter().any(|label| label.contains("Standard · recommended")));
        snapshot.installer_setups[0].phase = crate::model::SetupPhase::SetupNeedsAttention;
        snapshot.installer_setups[0].status = "The completed scan needs review.".into();
        snapshot.installer_setups[0].discovered = vec![crate::model::DiscoveredProduct {
            environment: "cd".repeat(16), module_sha256: "ef".repeat(32),
            class_id: "ab".repeat(16), name: "Exact plug-in".into(),
        }];
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            Operator::setup(ui, &snapshot, false, None,
                (&mut Page::Setup, &mut crate::library::Library::default(),
                    &mut RouteFocus::default()), &mut None, &mut false);
        });
        let mut labels = Vec::new();
        for clipped in &output.shapes { texts(&clipped.shape, &mut labels); }
        output.textures_delta.clear();
        assert!(labels.iter().any(|label| label.contains("Setup needs attention")));
        assert!(labels.iter().any(|label| label.contains("View plug-in details")));
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
                disabled_reason:None},history:vec![],
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
                Operator::fast_setup(ui,&overview,true,false,
                    (&mut false,&mut None,&mut false,&mut page,&mut library,&mut focus));
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
            assert!(text.iter().any(|line| line.contains("compatibility controls")));
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
                Operator::workspaces(ui, &snapshot, false, &mut chosen, &mut pick);
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
        }
        let workspace = &mut snapshot.workspaces[0];
        workspace.state = "ready".into();
        workspace.selected_release = "27.0.0.0".into();
        workspace.installed_advertised_release = Some("26.1.6.0".into());
        workspace.observed_file_version = Some("26.1.6.5639".into());
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            Operator::workspaces(ui, &snapshot, false, &mut None, &mut false);
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
    fn narrow_home_shows_one_attention_shortcut_and_one_all_items_route() {
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
        snapshot.active_sessions.push(serde_json::json!({
            "session":"exact-session", "class_id":"fixture-kontakt", "state":"failed",
            "recent":true, "terminal":"editor_controller_failed",
            "cleanup_confirmed":true, "transport_retired":true
        }));
        assert_eq!(presentation::attentions(&snapshot).len(), 2);
        let ctx = egui::Context::default();
        let mut page = Page::Home;
        let mut library = crate::library::Library::default();
        let mut focus = RouteFocus::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.set_max_width(528.0);
            Operator::home(ui, &snapshot, &mut page, &mut library, &mut focus);
        });
        let mut labels = Vec::new();
        for clipped in &output.shapes {
            texts(&clipped.shape, &mut labels);
        }
        output.textures_delta.clear();
        assert!(labels.iter().any(|text| text == "Review item"));
        assert!(labels
            .iter()
            .any(|text| text == "See all 2 attention items"));
        assert!(!labels
            .iter()
            .any(|text| text == "Review this" || text == "Open item"));
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
            schema: crate::model::OPERATOR_SCHEMA,
            state_token: "current".into(),
            installer_setups: vec![],
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
    fn guided_check_keeps_the_offered_identity_and_queues_once() {
        let mut o = state_fixture();
        let action = Action::CompatibilityCheck {
            selection: "ab".repeat(32), audio_layout: None,
            recipe: "cd".repeat(32), predecessor: None,
        };
        let request = Request { schema: crate::model::OPERATOR_SCHEMA, state_token: "current".into(), action };
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
        assert!(f.for_installer(&"ab".repeat(32)));
        assert!(!f.for_installer(&"ef".repeat(32)));
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
        s.products.push(Product{class_id:"aa".repeat(16),name:"Generated instrument".into(),vendor:"Fixture".into(),role:"instrument".into(),version:"1".into(),disposition:"prepared".into(),active_revision:None,recommended_revision:None,environment:"ef".repeat(16),runner:"pinned".into(),module_sha256:"ff".repeat(32),limitations:vec![],history:vec![],actions:vec![],compatibility:None,details:serde_json::json!({"preparation":{"operation":{"operation":"11".repeat(16),"state":"completed"}}})});
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

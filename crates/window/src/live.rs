//! Live owner readback, offered actions and package Setup. The manager owns every
//! offer and result; this controller owns navigation and one outstanding request.
use crate::{presentation::{self, After, Built, FormStep, ListStep, LocalUi, Node, Tab, Target, Why}, Outcome};
use lvb_client::{Query, Reply};
use lvb_view_model::{self as vm, forms, model::{Action, CurrentOverview, CurrentProductDetail, ProblemCategory, ReadinessAssessment, Request, Snapshot, TestResultKind},
    screen::{Reading, Records, Window}, state::{Effect, Imported, State}, window::Page, Destination, ProductKey};
use makepad_product_ui::{makepad_widgets::Cx, Availability, InputPath};
use std::collections::BTreeMap;
use vm::request::RequestFeedback as Support;

const LIMITATION: &str = "Sample controls are unavailable in the installed manager.";
const UNKNOWN_ACK: &str = "The request acknowledgment is unconfirmed. It may have been accepted. No request will be repeated automatically.";
const DETAILS_CHANGED: &str = "The records changed while loading. Try again to read the current records.";

#[derive(Clone, Debug, PartialEq, Eq)]
struct Binding { token: String, generation: String }
enum PendingDetails {
    Snapshot(Box<Snapshot>, Binding),
    Changed,
}
#[derive(Clone)]
enum Flight {
    Current,
    Overview,
    Import,
    OpenCurrent,
    Product { key: ProductKey, binding: Binding },
    Details { generation: String },
    Pulse,
    Operation(String),
    Action { binding: Binding, offer: Box<Action>, context: Box<State>, ui: Box<LocalUi> },
    Save,
}

pub struct Live {
    pub state: State,
    pub built: Built,
    ui: LocalUi,
    overview: Option<CurrentOverview>,
    readiness: Option<(ReadinessAssessment, Binding)>,
    pending_import: Option<(String, bool, String)>,
    product: Option<Snapshot>,
    details: Option<Snapshot>,
    pending_details: Option<PendingDetails>,
    details_notice: Option<String>,
    details_retried: bool,
    fresh: bool,
    current_read_error: Option<String>,
    superseded: bool,
    product_attempted: bool,
    details_attempted: bool,
    need_current: bool,
    need_interactive: bool,
    need_details: bool,
    queued: Option<(Query, Flight)>,
    in_flight: Option<Flight>,
    support: Option<Support>,
    message: String,
    reveal: bool,
    epoch: u64,
    shown_binding: Option<Binding>,
    form_origin: Option<(Binding, Action)>,
    form_generation: u64,
    last_current: f64,
    operation_live: Option<bool>,
    drawn: BTreeMap<String, Option<String>>,
    offers: BTreeMap<String, Binding>,
    log: Vec<String>,
    clipboard: Option<String>,
    next_poll: f64,
    render_revision: u64,
    bootstrap_panel: Option<lvb_client::bootstrap::Panel>,
    bootstrap_step: Option<lvb_client::bootstrap::Step>,
    bootstrap_waiting: bool,
    update_panel: Option<vm::updates::Panel>,
    update_headline: Option<String>,
    update_step: Option<(vm::updates::Step, Option<String>)>,
    update_waiting: bool,
    update_available: bool,
    close_requested: bool,
    health_observed: Option<f64>,
}
impl Live {
    pub fn new(_posture: String, reveal: bool) -> Self {
        let mut live = Self { state: State::default(), built: Built::default(), ui: LocalUi::default(),
            overview: None, readiness: None, pending_import: None, product: None, details: None, pending_details: None, details_notice: None, details_retried: false, fresh: false, current_read_error: None, superseded: false,
            product_attempted: false, details_attempted: false, need_current: true, need_interactive: false, need_details: false,
            queued: None, in_flight: None, support: None, message: String::new(), reveal,
            epoch: 0, shown_binding: None, form_origin: None, form_generation: 0, last_current: Cx::monotonic_now(), operation_live: None, drawn: BTreeMap::new(), offers: BTreeMap::new(), log: vec![], clipboard: None,
            next_poll: Cx::monotonic_now() + 3.0, render_revision: 0, bootstrap_panel: None, bootstrap_step: None, bootstrap_waiting: false, update_panel: None, update_headline: None, update_step: None, update_waiting: false, update_available: false, close_requested: false, health_observed: None };
        live.drive();
        live.rebuild();
        live
    }
    fn binding(&self) -> Option<Binding> {
        self.overview.as_ref().map(|o| Binding { token: o.current.state_token.clone(), generation: o.current_generation.clone() })
    }
    fn busy(&self) -> bool { self.queued.is_some() || self.in_flight.is_some() }
    fn foreground_busy(&self) -> bool {
        self.queued.as_ref().map(|(_, flight)| flight).into_iter().chain(self.in_flight.as_ref())
            .any(|flight| matches!(flight, Flight::Action { .. } | Flight::Save | Flight::Import | Flight::OpenCurrent))
    }
    /// Background transport work alone does not replace native nodes or focus.
    pub fn render_revision(&self) -> u64 { self.render_revision }
    fn render_matches(&self, other: &Built) -> bool {
        let built = &self.built;
        built.header == other.header && built.page == other.page && built.preview == other.preview
            && built.form == other.form && built.targets == other.targets
            && built.form_focus == other.form_focus && built.screen_from == other.screen_from
    }
    fn support_waiting(&self) -> bool { self.support.as_ref().is_some_and(|s| s.blocking) }
    fn retained_operation(&self) -> Option<String> {
        self.support.as_ref().filter(|s| !s.terminal).and_then(|s| s.operation.clone())
    }
    fn read_report_operation(&mut self) {
        if let Some(id) = self.retained_operation() {
            self.queue(Query::Operation(id.clone()), Flight::Operation(id));
        } else {
            self.queue(Query::Pulse, Flight::Pulse);
        }
    }
    fn queue(&mut self, query: Query, flight: Flight) {
        if self.queued.is_none() && !self.foreground_busy() && !self.superseded {
            self.queued = Some((query, flight));
        }
    }
    fn drive(&mut self) {
        if self.bootstrap_panel.is_some() || self.busy() || self.superseded { return; }
        if self.need_interactive {
            self.need_interactive = false; self.need_current = false;
            self.queue(Query::Overview, Flight::Overview);
        } else if self.need_current {
            self.need_current = false;
            self.queue(Query::CurrentOverview, Flight::Current);
        } else if self.need_details {
            if let Some(binding) = self.binding() {
                self.need_details = false;
                self.details_attempted = true;
                self.details_notice = None;
                self.queue(Query::Snapshot, Flight::Details { generation: binding.generation });
            }
        } else if self.state.page == Page::Plugins && self.product.is_none() && !self.product_attempted {
            if let (Some(key), Some(binding)) = (self.state.plugins.focused_product().cloned(), self.binding()) {
                self.product_attempted = true;
                self.queue(Query::Product(key.clone()), Flight::Product { key, binding });
            }
        }
    }
    fn queue_foreground(&mut self, query: Query, flight: Flight) {
        if self.foreground_busy() || self.superseded { return; }
        // A background read that has not left the window can yield to an
        // intentional request. Preserve its need; never drop an action.
        if let Some((_, postponed)) = self.queued.take() {
            match postponed {
                Flight::Current => self.need_current = true,
                Flight::Overview => self.need_interactive = true,
                Flight::Details { .. } => self.need_details = true,
                Flight::Product { .. } => self.product_attempted = false,
                _ => {}
            }
        }
        self.queued = Some((query, flight));
    }
    fn offered_in(&self, original: &Action, context: &State, ui: &LocalUi) -> bool {
        let current = self.overview.as_ref().map(|o| &o.current);
        let records = Records { current, actions: current, product: self.product.as_ref(), details: self.details.as_ref(),
            ..Records::default() };
        let window = Window::build(&records, &Reading { fresh: true, ..Reading::default() }, context);
        let rendered = presentation::build(&window, self.product.as_ref().or(current).or(self.details.as_ref()), current,
            context, ui, Why { fresh: true, pending: false, in_flight: false, superseded: false }, None, self.reveal, self.form_generation);
        rendered.targets.values().any(|target| matches!(target,
            Target::Offer { offer, .. } if &offer.intent == original && offer.availability.enabled()))
    }
    fn submit(&mut self, action: Action, binding: Binding, offer: Action) {
        if !self.fresh || self.binding().as_ref() != Some(&binding) || self.foreground_busy() || self.support_waiting() { return; }
        let context = Box::new(self.state.clone());
        self.support = Some(Support::captured(action.clone()));
        self.state.sent(&action);
        self.form_origin = None;
        self.queue_foreground(Query::Action(Request { schema: vm::model::OPERATOR_SCHEMA,
            state_token: binding.token.clone(), action }), Flight::Action { binding, offer: Box::new(offer), context, ui: Box::new(self.ui.clone()) });
    }
    /// The worker takes one request. Until its reply, no other query is returned.
    pub fn take_query(&mut self) -> Option<Query> {
        if self.in_flight.is_some() { return None; }
        let (query, flight) = self.queued.take()?;
        if let Flight::Action { binding, offer: original, context, ui } = &flight {
            // A click may wait behind a read, but it cannot borrow that read's
            // new authority. Revalidate the exact displayed offer before send.
            let offered = self.offered_in(original, context, ui);
            if !self.fresh || self.superseded || self.binding().as_ref() != Some(binding) || !offered {
                if let Some(support) = &mut self.support {
                    support.terminal = true; support.blocking = false;
                    support.text = "The request was not submitted because its current offer changed. Review the current status and try again.".into();
                    support.transitions.push("not submitted".into());
                }
                self.rebuild();
                return None;
            }
        }
        if matches!(flight, Flight::Action { .. }) {
            if let Some(support) = &mut self.support {
                support.text = "Sending the selected request…".into();
                support.transitions.push("submitted".into());
            }
        }
        self.in_flight = Some(flight);
        self.rebuild();
        Some(query)
    }
    pub fn bootstrap(&mut self, panel: lvb_client::bootstrap::Panel, close: bool) {
        self.queued = None; self.in_flight = None; self.fresh = false;
        self.update_step = None; self.form_origin = None; self.state.forms = Default::default();
        self.bootstrap_waiting = false; self.bootstrap_panel = Some(panel);
        self.close_requested |= close; self.rebuild();
    }
    pub fn take_bootstrap_step(&mut self) -> Option<lvb_client::bootstrap::Step> { self.bootstrap_step.take() }
    pub fn bootstrap_failed(&mut self, error: String) {
        self.bootstrap_waiting = false;
        if let Some(panel) = &mut self.bootstrap_panel { panel.result = Some(error); }
        self.rebuild();
    }
    pub fn worker_stopped(&mut self, error: String) {
        if let Some(panel) = &mut self.bootstrap_panel {
            panel.progress = None; panel.controls.clear(); panel.result = Some(error);
            panel.detail = "Reopen Setup to check the retained change.";
            self.bootstrap_waiting = false; self.bootstrap_step = None; self.rebuild();
        } else { self.receive(Reply::Error(error.clone())); self.updater_failed(error); }
    }
    pub fn updater_reading(&self) -> Option<(bool, f64)> {
        if !self.fresh || self.superseded { return None; }
        let system = &self.overview.as_ref()?.current.system;
        system.capacity_available().then_some((system.dsp > 0, self.health_observed?))
    }
    pub fn take_update_step(&mut self) -> Option<(vm::updates::Step, Option<String>)> { self.update_step.take() }
    pub fn update(&mut self, panel: vm::updates::Panel, headline: Option<String>, close: bool, available: bool) {
        if matches!(panel.link, vm::updates::Link::Saved { .. }) { self.ui.link_draft.clear(); }
        self.update_available = available; self.update_panel = Some(panel); self.update_headline = headline; self.update_waiting = false;
        self.close_requested |= close; self.rebuild();
    }
    pub fn pause_superseded(&mut self) {
        self.superseded = true; self.fresh = false; self.queued = None; self.update_step = None;
        self.update_waiting = false;
        self.message = "Bridge updated. This window is paused; open the current manager.".into();
        self.rebuild();
    }
    pub fn updater_failed(&mut self, error: String) { self.update_waiting = false; self.message = error; self.rebuild(); }
    pub fn take_close_request(&mut self) -> bool { std::mem::take(&mut self.close_requested) }
    pub fn take_clipboard(&mut self) -> Option<String> { self.clipboard.take() }
    /// Poll only while idle; reads never replay an action with an uncertain acknowledgment.
    pub fn tick(&mut self) -> bool {
        let now = Cx::monotonic_now();
        if self.bootstrap_panel.is_some() || self.busy() || self.superseded || now < self.next_poll { return false; }
        self.next_poll = now + 3.0;
        if !self.fresh || self.overview.is_none() || now - self.last_current >= 6.0 {
            self.need_current = true;
        }
        // A retained result remains readable even when current status is not.
        // Read it first, then let drive renew the overview when due.
        if self.retained_operation().is_some() || !self.need_current { self.read_report_operation(); }
        self.drive();
        self.rebuild();
        true
    }
    pub fn receive(&mut self, reply: Reply) {
        let Some(flight) = self.in_flight.take() else { return };
        match (flight, reply) {
            (_, Reply::FrontendSuperseded) => {
                self.superseded = true; self.fresh = false; self.queued = None;
                self.message = "Bridge updated. This window is paused; open the current manager.".into();
            }
            (Flight::Overview, Reply::Overview(overview)) => {
                let binding = Binding { token: overview.current.state_token.clone(), generation: overview.current_generation.clone() };
                self.readiness = Some((overview.readiness.clone(), binding));
                self.in_flight = Some(Flight::Current);
                self.receive(Reply::CurrentOverview(Box::new((*overview).into())));
                return;
            }
            (Flight::Import, Reply::Imported(imported)) => {
                if let Some(draft) = &mut self.state.focus.replacement {
                    draft.selected = Some(imported.installer_sha256);
                    self.message = format!("Selected {}. Review it in this setup before running it.", imported.display_label);
                } else {
                    self.pending_import = Some((imported.installer_sha256, imported.newly_imported, imported.display_label.clone()));
                    self.message = format!("{} was added. Opening its setup…", imported.display_label);
                }
                self.fresh = false; self.need_current = true;
            }
            (Flight::Import, Reply::Cancelled) => { self.message = "No installer selected.".into(); self.state.focus.replacement = None; }
            (Flight::OpenCurrent, Reply::CurrentManagerOpened) => { self.message = "The current manager was opened.".into(); }
            (Flight::Current, Reply::CurrentOverview(overview)) => {
                self.current_read_error = None;
                let changed = self.binding().as_ref() != Some(&Binding { token: overview.current.state_token.clone(), generation: overview.current_generation.clone() });
                let system_changed = self.overview.as_ref().is_some_and(|old| old.current.system != overview.current.system);
                self.state.observe(&overview.current);
                if changed || system_changed { self.invalidate_pages(); }
                if let Some(support) = &mut self.support { support.reconcile_snapshot(&overview.current); }
                if let Some((installer, added, label)) = self.pending_import.take() {
                    match self.state.imported(&overview.current, &installer, added, &label) {
                        Imported::Shown(message) => self.message = message,
                        Imported::NotYet(message) => { self.message = message.into(); self.pending_import = Some((installer, added, label)); }
                    }
                }
                self.overview = Some(*overview); self.fresh = true; self.last_current = Cx::monotonic_now(); self.health_observed = Some(self.last_current);
                if let Some(pending) = self.pending_details.take() {
                    match pending {
                        PendingDetails::Snapshot(snapshot, requested) if self.binding().as_ref() == Some(&requested)
                            && self.overview.as_ref().is_some_and(|o| o.current.system == snapshot.system) => {
                            self.details = Some(*snapshot);
                            self.details_notice = None;
                            self.details_attempted = true; self.need_details = false;
                        }
                        _ => self.retry_changed_details(),
                    }
                }
                if matches!(self.state.page, Page::Diagnostics | Page::Activity)
                    && self.details.is_none() && !self.details_attempted { self.need_details = true; }
            }
            (Flight::Product { key, binding }, Reply::Product(detail)) => {
                if self.binding().as_ref() == Some(&binding) && self.state.plugins.focused_product() == Some(&key)
                    && detail.state_token == binding.token && detail.current_generation == binding.generation
                    && ProductKey::from(&detail.product) == key {
                    self.product = self.project_product(*detail);
                } else {
                    self.product = None;
                    self.message = "The opened plug-in changed while its records were loading. Refreshing current status.".into();
                    self.fresh = false; self.need_current = true;
                }
            }
            (Flight::Details { generation }, Reply::Snapshot(snapshot)) => {
                let observed = Binding { token: snapshot.state_token.clone(), generation };
                if self.binding().as_ref() == Some(&observed)
                    && self.overview.as_ref().is_some_and(|o| o.current.system == snapshot.system) {
                    self.details = Some(*snapshot);
                    self.details_notice = None;
                } else {
                    self.pending_details = Some(PendingDetails::Snapshot(snapshot, observed));
                    self.fresh = false;
                    self.need_current = true;
                }
            }
            (Flight::Pulse, Reply::Pulse(pulse)) => {
                let changed = self.overview.as_ref().is_none_or(|o| {
                    let system = &o.current.system;
                    o.current_generation != pulse.current_generation || system.service != pulse.service_state
                        || Some(system.dsp) != pulse.dsp || Some(system.keepers) != pulse.keepers
                        || Some(system.maintenance) != pulse.maintenance
                        || system.pending_transactions != pulse.pending_transactions
                        || Some(system.cleanup_unconfirmed) != pulse.cleanup_unconfirmed
                        || o.current.operation != pulse.operation
                }) || self.operation_live.is_some_and(|live| live != pulse.operation_live);
                self.operation_live = Some(pulse.operation_live);
                if changed || pulse.service_state != "active" || pulse.cleanup_unconfirmed.is_none() {
                    self.fresh = false; self.need_current = true;
                }
            }
            (Flight::Operation(id), Reply::Operation(readback)) => {
                if readback.operation != id || readback.record["operation"].as_str() != Some(id.as_str()) {
                    self.failed(Flight::Operation(id), "The manager returned a different operation's result.".into());
                } else if let Some(support) = self.support.as_mut().filter(|s| s.operation.as_ref() == Some(&id)) {
                    support.observe(&readback.record);
                    if support.terminal || support.release_after_snapshot { self.need_current = true; }
                }
            }
            (Flight::Action { .. }, Reply::Receipt(receipt)) => {
                if let Some(support) = &mut self.support {
                    support.receipt(&receipt);
                    if support.acknowledgment_uncertain { support.text = UNKNOWN_ACK.into(); }
                }
                self.fresh = false; self.need_current = true;
                self.read_report_operation();
            }
            (Flight::Action { .. }, Reply::Cancelled) => {
                if let Some(support) = &mut self.support {
                    support.terminal = true; support.blocking = false;
                    support.text = "No program selected. The request was not submitted.".into();
                }
            }
            (Flight::Save, Reply::ReportSaved(path)) => { self.message = format!("Problem report saved to {}.", path.display()); if let Some(s) = &mut self.support { s.text = self.message.clone(); } }
            (Flight::Save, Reply::Cancelled) => { self.message = "Saving cancelled; the report remains ready to save.".into(); if let Some(s) = &mut self.support { s.text = self.message.clone(); } }
            (flight, Reply::Error(error)) => self.failed(flight, error),
            (flight, _) => self.failed(flight, "The manager returned an unexpected reply. Refresh current status.".into()),
        }
        self.drive();
        self.rebuild();
    }
    fn failed(&mut self, flight: Flight, error: String) {
        if matches!(flight, Flight::Current | Flight::Overview | Flight::Pulse) {
            self.current_read_error = Some(error);
        } else if matches!(flight, Flight::Details { .. }) {
            // The owner refused an optimistic read because its records moved;
            // this is not a page failure. Validate current status before using
            // the same bounded retry as an obsolete successful response.
            let refusal = error.trim().strip_prefix("Error: ").unwrap_or(error.trim()).trim_matches('"');
            self.details_notice = Some(if refusal == "operator_state_changed_refresh" {
                self.fresh = false; self.need_current = true;
                self.pending_details = Some(PendingDetails::Changed);
                DETAILS_CHANGED.into()
            } else { format!("This page could not be loaded. Try again. {}", error.chars().take(1024).collect::<String>()) });
        } else { self.message = error; }
        match flight {
            Flight::Action { .. } => {
                if let Some(s) = &mut self.support { s.acknowledgment_uncertain = true; s.release_after_snapshot = true; s.text = format!("{UNKNOWN_ACK} {}", self.message); }
                self.fresh = false; self.need_current = true;
                self.queue(Query::Pulse, Flight::Pulse);
            }
            Flight::Operation(id) => {
                if let Some(s) = self.support.as_mut().filter(|s| s.operation.as_ref() == Some(&id)) {
                    s.text = format!("The request's retained result could not be read. {} No request was repeated.", self.message);
                }
            }
            Flight::Save => if let Some(s) = &mut self.support { s.text = self.message.clone(); },
            Flight::Current | Flight::Overview | Flight::Pulse => {
                self.fresh = false;
                if self.pending_details.take().is_some() {
                    self.details_attempted = true; self.need_details = false;
                    self.details_notice = Some(format!("Current status could not validate this page. Try again. {}",
                        self.current_read_error.as_deref().unwrap_or_default().chars().take(1024).collect::<String>()));
                }
            },
            Flight::Import | Flight::OpenCurrent => {}
            Flight::Product { .. } => self.product = None,
            Flight::Details { .. } => { self.details = None; }
        }
    }
    fn project_product(&self, detail: CurrentProductDetail) -> Option<Snapshot> {
        let mut snapshot = self.overview.as_ref()?.current.clone();
        let key = ProductKey::from(&detail.product);
        if snapshot.products.iter().filter(|p| ProductKey::from(*p) == key).count() != 1 { return None; }
        let product = snapshot.products.iter_mut().find(|p| ProductKey::from(&**p) == key)?;
        *product = detail.product;
        snapshot.system = detail.system;
        snapshot.environments = detail.environments;
        snapshot.vendor_applications = detail.vendor_applications;
        Some(snapshot)
    }
    pub fn rebuild(&mut self) {
        let previous = std::mem::take(&mut self.built);
        if let Some(panel) = &self.bootstrap_panel {
            self.built = presentation::bootstrap::build(panel, self.bootstrap_waiting);
            self.drawn.clear(); self.offers.clear();
            for nodes in [&mut self.built.header, &mut self.built.page] {
                bind_nodes(nodes, &BTreeMap::new(), &BTreeMap::new(), &mut self.drawn);
            }
            if !self.render_matches(&previous) { self.render_revision = self.render_revision.wrapping_add(1); }
            return;
        }
        // Technical history stays retained, while global offers always come
        // from the current owner's overview, including same-token refusals.
        if let (Some(current), Some(details)) = (&self.overview, &mut self.details) {
            if details.state_token == current.current.state_token {
                details.actions = current.current.actions.clone();
            }
        }
        let binding = self.binding();
        if binding != self.shown_binding {
            self.epoch = self.epoch.wrapping_add(1);
            self.shown_binding = binding.clone();
        }
        let current = self.overview.as_ref().map(|o| &o.current);
        let actions = (self.fresh && !self.superseded).then_some(current).flatten();
        let records = Records { current, details: self.details.as_ref(), product: self.product.as_ref(), actions,
            readiness: self.readiness.as_ref().filter(|(value, requested)| binding.as_ref() == Some(requested) && current.is_some_and(|current| value.system == current.system)).map(|(value, _)| value),
            health: self.current_read_error.is_none().then(|| current.map(|s| &s.system)).flatten() };
        let foreground = self.foreground_busy();
        let blocked = foreground || self.support_waiting();
        let details_pending = self.need_details || self.pending_details.is_some()
            || self.queued.as_ref().map(|(_, flight)| flight).into_iter().chain(self.in_flight.as_ref())
                .any(|flight| matches!(flight, Flight::Details { .. }));
        let product_pending = self.queued.as_ref().map(|(_, flight)| flight).into_iter().chain(self.in_flight.as_ref())
            .any(|flight| matches!(flight, Flight::Product { key, binding: requested }
                if self.state.plugins.focused_product() == Some(key) && binding.as_ref() == Some(requested)));
        let current_pending = self.queued.as_ref().map(|(_, flight)| flight).into_iter().chain(self.in_flight.as_ref())
            .any(|flight| matches!(flight, Flight::Current | Flight::Overview));
        let reading = Reading { fresh: self.fresh, renewing: current_pending,
            current_unavailable: self.current_read_error.is_some(),
            owned_pause: vm::service::owned_service_pause(current.and_then(|s| s.operation.as_ref())),
            health_renewing: !self.fresh && !self.superseded && self.current_read_error.is_none(),
            controls_pending: blocked, in_flight: foreground, working: foreground,
            details_attempted: self.details_attempted && !details_pending,
            details_notice: self.details_notice.as_deref(),
            // The queue guard is set before dispatch; a pending attempt is not
            // the shared screen's completed-but-unavailable detail read.
            product_attempted: self.product_attempted && !product_pending,
            superseded: self.superseded, message: self.current_read_error.as_deref().unwrap_or(&self.message), update_headline: self.update_headline.clone(),
            request: self.support.as_ref().map(|s| vm::window::Request { text: &s.text, journey: s.journey.as_ref(),
                operation: s.operation.as_deref(), transitions: &s.transitions, report: s.report.is_some(), blocking: s.blocking }),
            ..Reading::default() };
        let window = Window::build(&records, &reading, &self.state);
        self.built = presentation::build(&window, self.product.as_ref().or(current).or(self.details.as_ref()), actions,
            &self.state, &self.ui, Why { fresh: self.fresh, pending: blocked || self.form_origin.as_ref().is_some_and(|(origin, _)| !self.fresh || Some(origin) != binding.as_ref()), in_flight: foreground, superseded: self.superseded },
            self.update_panel.as_ref(), self.reveal, self.form_generation);
        self.built.preview.clear();
        let mut renames = BTreeMap::new();
        let mut targets = BTreeMap::new();
        let mut limitations = BTreeMap::new();
        self.offers.clear();
        for (id, target) in std::mem::take(&mut self.built.targets) {
            let new_id = if matches!(target, Target::Offer { .. }) { format!("live.{}.{}", self.epoch, id) } else { id.clone() };
            renames.insert(id, new_id.clone());
            if let Target::Offer { .. } = &target {
                if let Some(binding) = &binding { self.offers.insert(new_id.clone(), binding.clone()); }
            } else if matches!(target, Target::Update(_)) && (self.update_waiting || self.superseded || !self.update_available) {
                limitations.insert(new_id.clone(), if self.superseded { "Open the current manager to update." } else { "Waiting for the installed updater." });
            } else if unavailable(&target) { limitations.insert(new_id.clone(), LIMITATION); }
            targets.insert(new_id, target);
        }
        self.built.targets = targets;
        self.drawn.clear();
        for nodes in [&mut self.built.header, &mut self.built.page, &mut self.built.preview] {
            bind_nodes(nodes, &renames, &limitations, &mut self.drawn);
        }
        if let Some(nodes) = &mut self.built.form { bind_nodes(nodes, &renames, &limitations, &mut self.drawn); }
        if !self.render_matches(&previous) { self.render_revision = self.render_revision.wrapping_add(1); }
    }
    pub fn press(&mut self, id: &str, _via: InputPath) -> Outcome {
        let Some(target) = self.built.targets.get(id).cloned() else { return Outcome::Refused };
        let Some(reason) = self.drawn.get(id) else { return Outcome::Refused };
        if let Some(reason) = reason { self.note(reason.clone()); return Outcome::Refused; }
        let before = (self.state.page, self.state.plugins.focus.clone());
        match target {
            Target::Offer { offer, after } => {
                if self.foreground_busy() || self.support_waiting() || !self.fresh
                    || self.offers.get(id) != self.binding().as_ref() { return Outcome::Refused; }
                let Some(binding) = self.binding() else { return Outcome::Refused };
                let original = offer.intent.clone();
                match self.state.press_offer(&offer) {
                    Some(action) => self.submit(action, binding, original),
                    None => { self.form_generation += 1; self.form_origin = Some((binding, original)); }
                }
                if let After::CloseReplacement { focus_after } = after {
                    if let Some(installer) = focus_after { self.state.focus.setup_installer = Some(installer); }
                    self.state.focus.replacement = None;
                }
            }
            Target::Form(step) => self.form_step(step),
            Target::Daw(choice) => self.state.forms.daw = Some(choice),
            Target::Bootstrap(step) => {
                if self.bootstrap_waiting { return Outcome::Refused; }
                self.bootstrap_step = Some(step); self.bootstrap_waiting = true;
            }
            Target::Update(step) => {
                if self.update_waiting || self.superseded || !self.update_available { return Outcome::Refused; }
                let draft = (step == vm::updates::Step::SaveLink).then(|| self.ui.link_draft.clone());
                self.update_step = Some((step, draft)); self.update_waiting = true;
            }
            Target::Control(control) => if let Some(effect) = self.state.apply(control.does) { self.effect(effect); },
            Target::Tab(tab) => match tab {
                Tab::Home => self.state.page = Page::Home,
                Tab::Plugins => { self.state.page = Page::Plugins; self.state.plugins.focus = None; },
                Tab::Activity => self.state.go(&Destination::Activity),
                Tab::Support => self.state.go(&Destination::Diagnostics),
            },
            Target::List(step) => match step {
                ListStep::ShowAll | ListStep::ClearFilters => self.state.plugins = Default::default(),
                ListStep::ClearSearch => self.state.plugins.search.clear(),
                ListStep::Role(role) => self.state.plugins.role = role,
                ListStep::Status(status) => self.state.plugins.status = status,
                ListStep::Open(key) => self.state.go(&Destination::Product(key)),
            },
            Target::Choose { group, key } => { self.ui.choices.insert(group, key); }
            _ => { self.note(LIMITATION.into()); return Outcome::Refused; }
        }
        let navigated = before != (self.state.page, self.state.plugins.focus.clone());
        if navigated {
            self.product = None; self.product_attempted = false;
            if matches!(self.state.page, Page::Diagnostics | Page::Activity) && !self.details_attempted { self.need_details = true; }
        }
        self.drive(); self.rebuild();
        if navigated { Outcome::Navigated } else { Outcome::Changed }
    }
    fn invalidate_pages(&mut self) {
        // Changed owner state retires loaded pages, but a failed read remains
        // an explicit retry. Unrelated counter changes must not replay it.
        if self.product.take().is_some() { self.product_attempted = false; }
        if self.details.take().is_some() { self.details_attempted = false; self.details_retried = false; }
        self.need_details = !self.details_attempted && matches!(self.state.page, Page::Diagnostics | Page::Activity);
    }
    fn retry_changed_details(&mut self) {
        self.details = None;
        // One obsolete response gets one fresh read after its validating
        // overview. Passive changes cannot renew that budget and spin forever.
        if !self.details_retried {
            self.details_retried = true;
            self.details_attempted = false;
            self.need_details = matches!(self.state.page, Page::Diagnostics | Page::Activity);
            self.details_notice = None;
        } else {
            self.details_attempted = true; self.need_details = false;
            self.details_notice = Some(DETAILS_CHANGED.into());
        }
    }
    fn effect(&mut self, effect: Effect) {
        match effect {
            Effect::Refresh => {
                self.fresh = false; self.need_interactive = true;
                self.product_attempted = false; self.details_attempted = false;
                self.details_retried = false;
                self.pending_details = None; self.details_notice = None; self.message.clear(); self.invalidate_pages();
            }
            Effect::RetryDetails => {
                self.message.clear();
                if self.state.page == Page::Plugins && self.state.plugins.focused_product().is_some() {
                    self.product = None;
                    self.product_attempted = false;
                } else { self.details_notice = None; self.details_retried = false; self.need_details = true; }
            }
            Effect::SaveReport => if !self.foreground_busy() && !self.superseded {
                if let Some(report) = self.support.as_ref().and_then(|s| s.report.clone()) { self.queue_foreground(Query::SaveSupportReport(report), Flight::Save); }
            },
            Effect::ChooseInstaller => if !self.foreground_busy() && !self.support_waiting() && !self.superseded {
                self.queue_foreground(Query::PickInstaller, Flight::Import);
            },
            Effect::OpenCurrentManager => if !self.foreground_busy() {
                // This is the one explicit handoff allowed from a superseded window.
                self.queued = Some((Query::OpenCurrentManager, Flight::OpenCurrent));
            },
            Effect::Copy(words) => self.clipboard = Some(words),
            Effect::Dismiss => if !self.support_waiting() { self.support = None; self.message.clear(); },
        }
    }
    fn form_step(&mut self, step: FormStep) {
        let submitting = matches!(step, FormStep::StartInstall | FormStep::SaveRename | FormStep::Confirm | FormStep::SubmitRecord);
        let origin = self.form_origin.clone();
        if submitting && (self.foreground_busy() || self.support_waiting() || !self.fresh
            || origin.as_ref().is_none_or(|(binding, _)| Some(binding) != self.binding().as_ref())) { return; }
        let forms = &mut self.state.forms;
        match step {
            FormStep::ChooseDaw(choice) => forms.daw = Some(choice),
            FormStep::StartInstall => {
                if let (Some(form), Some(daw)) = (forms.install.clone(), forms.daw) {
                    if let Some(action) = forms::installation_with_daw(&form, daw) {
                        forms.install = None;
                        if let Some((binding, offer)) = origin { self.submit(action, binding, offer); }
                    }
                }
                return;
            }
            FormStep::CancelInstall => forms.install = None,
            FormStep::SaveRename => {
                if let Some(action) = forms.rename.clone() {
                    if let Some((binding, offer)) = origin { self.submit(action, binding, offer); }
                }
                return;
            }
            FormStep::CancelRename => forms.rename = None,
            FormStep::ToggleForget => forms.forget_installer = !forms.forget_installer,
            FormStep::Confirm => {
                let Some(form) = forms.confirm.clone() else { return };
                let confirmation = forms::removal_confirmation(self.product.as_ref().or_else(|| self.overview.as_ref().map(|o| &o.current)), &form);
                if let Some(action) = self.state.forms.confirmed(confirmation.forget.is_some()) {
                    if let Some((binding, offer)) = origin { self.submit(action, binding, offer); }
                }
                return;
            }
            FormStep::CancelConfirm => forms.cancel_confirm(),
            FormStep::ResultWorked | FormStep::ResultProblem | FormStep::Category(_) | FormStep::FailedArea(_)
            | FormStep::Passed(_) => {
                if let Some(Action::CompatibilityResult { result, passed, failed_area, .. }) = forms.record.as_mut() {
                    match step {
                        FormStep::ResultWorked => {
                            *result = TestResultKind::Worked;
                            *failed_area = None;
                        }
                        FormStep::ResultProblem => {
                            if matches!(result, TestResultKind::Worked) {
                                *result = TestResultKind::Problem { category: ProblemCategory::Other };
                            }
                        }
                        FormStep::Category(choice) => {
                            if let TestResultKind::Problem { category } = result {
                                *category = choice;
                                if choice != ProblemCategory::Other { *failed_area = None; }
                            }
                        }
                        FormStep::FailedArea(area) => *failed_area = Some(area),
                        FormStep::Passed(area) => {
                            if passed.contains(&area) { passed.retain(|item| *item != area); }
                            else { passed.push(area); }
                        }
                        _ => {}
                    }
                }
            }
            FormStep::ExpertArea(chosen) | FormStep::ExpertStatus(chosen) => {
                if let Some(Action::CandidateObserve { area, status, .. }) = forms.record.as_mut() {
                    if matches!(step, FormStep::ExpertArea(_)) { *area = chosen.into(); } else { *status = chosen.into(); }
                }
            }
            FormStep::SubmitRecord => {
                if let Some(action) = forms.record.clone() {
                    if let Some((binding, offer)) = origin { self.submit(action, binding, offer); }
                }
                return;
            }
            FormStep::CancelRecord => forms.record = None,
        }
        let forms = &self.state.forms;
        if forms.install.is_none() && forms.rename.is_none() && forms.record.is_none() && forms.confirm.is_none() {
            self.form_origin = None;
        }
    }

    pub fn note(&mut self, line: String) {
        self.message = line.clone();
        if self.log.len() == 64 { self.log.remove(0); }
        self.log.push(line);
        self.rebuild();
    }
    pub fn new_log(&mut self) -> Vec<String> { std::mem::take(&mut self.log) }
    pub fn field(&mut self, id: &str, text: &str) {
        let forms = &mut self.state.forms;
        match id {
            "field.rename" => if let Some(Action::InstallerRename { label, .. }) = forms.rename.as_mut() {
                *label = text.chars().take(96).collect();
            },
            "field.note" => match forms.record.as_mut() {
                Some(Action::CompatibilityResult { note, .. } | Action::CandidateObserve { note, .. }) =>
                    *note = text.chars().take(512).collect(),
                Some(Action::CandidateReview { rationale, .. }) => *rationale = text.chars().take(512).collect(),
                _ => {}
            },
            "field.search" => self.state.plugins.search = text.chars().take(512).collect(),
            "field.beta-link" => self.ui.link_draft = text.chars().take(2048).collect(),
            _ => return,
        }
        self.rebuild();
    }

    pub fn dismiss_form(&mut self, _how: &str) {
        let daw = self.state.forms.daw;
        self.state.forms = Default::default(); self.state.forms.daw = daw;
        self.form_origin = None; self.rebuild();
    }
    pub fn page_first(&self) -> Option<String> {
        fn first(nodes: &[Node]) -> Option<String> { nodes.iter().find_map(|node| match node {
            Node::Actions { actions, .. } | Node::Tabs { tabs: actions, .. } => actions.first().map(|a| a.id.0.clone()),
            Node::Panel { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => first(children),
            Node::LibraryRow { action, .. } => Some(action.id.0.clone()), _ => None,
        }) }
        first(self.built.page.get(self.built.screen_from..).unwrap_or_default())
    }
}
fn unavailable(target: &Target) -> bool {
    matches!(target, Target::Preview(_))
}
fn bind_nodes(nodes: &mut [Node], renames: &BTreeMap<String, String>, limitations: &BTreeMap<String, &str>, drawn: &mut BTreeMap<String, Option<String>>) {
    for node in nodes {
        match node {
            Node::Actions { actions, .. } | Node::Tabs { tabs: actions, .. } => for action in actions {
                if let Some(id) = renames.get(&action.id.0) { action.id.0 = id.clone(); }
                if action.reason().is_none() {
                    if let Some(reason) = limitations.get(&action.id.0) { action.availability = Availability::Disabled { reason: (*reason).into() }; }
                }
                drawn.insert(action.id.0.clone(), action.reason().map(str::to_owned));
            },
            Node::Panel { children, .. } | Node::Fold { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => bind_nodes(children, renames, limitations, drawn),
            Node::LibraryRow { action, .. } => { drawn.insert(action.id.0.clone(), action.reason().map(str::to_owned)); }
            _ => {}
        }
    }
}

#[cfg(all(test, feature = "samples"))]
mod tests {
    use super::*;
    use vm::{model::{AvailableAction, Receipt}, samples::{Case, Sample}, Local};

    fn snapshot() -> Snapshot {
        let mut s = Sample::new(Case::new(Page::Home, "populated")).snapshot;
        s.state_token = "exact-token".into();
        s.actions = vec![AvailableAction { label: "Create problem report".into(), action: Action::SupportExport {}, disabled_reason: None }];
        s.operation = None;
        s
    }
    fn overview(snapshot: Snapshot, generation: &str) -> Reply {
        Reply::CurrentOverview(Box::new(CurrentOverview { schema: 1, operator_schema: vm::model::OPERATOR_SCHEMA,
            scope: "current_only".into(), current_generation: generation.into(), current: snapshot }))
    }
    fn interactive(snapshot: Snapshot, generation: &str) -> Reply {
        let readiness = ReadinessAssessment { schema: 1, state_token: snapshot.state_token.clone(), observed_at: 1,
            overall_status: vm::model::ReadinessOutcome::Unknown, system: snapshot.system.clone(),
            platform: vec![], daw: vec![], audio: vec![], graphics: vec![], runtime: vec![], products: vec![],
            blockers: vec![], ordered_steps: vec![], support_export_action: snapshot.actions[0].clone() };
        Reply::Overview(Box::new(vm::model::InteractiveOverview { schema: 1, operator_schema: vm::model::OPERATOR_SCHEMA,
            scope: "current_only".into(), current_generation: generation.into(), current: snapshot, readiness }))
    }
    fn ready() -> Live {
        let mut live = Live::new("test posture".into(), false);
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        assert!(live.take_query().is_none());
        live.receive(overview(snapshot(), "generation-a"));
        live
    }
    fn ready_support() -> Live {
        let mut live = ready();
        show_page(&mut live, Page::Diagnostics, &snapshot());
        live
    }
    fn refresh(live: &mut Live) {
        live.effect(Effect::Refresh); live.drive(); live.rebuild();
    }
    fn target(live: &Live, pick: impl Fn(&Target) -> bool) -> String {
        live.built.targets.iter().find(|(_, t)| pick(t)).map(|(id, _)| id.clone()).expect("target is drawn")
    }
    fn export(live: &Live) -> String { target(live, |t| matches!(t, Target::Offer { offer, .. } if matches!(offer.intent, Action::SupportExport {}))) }
    fn press(live: &mut Live, id: &str) -> Outcome { live.press(id, InputPath::Keyboard) }
    fn pulse(operation: Option<serde_json::Value>) -> Reply {
        let s = snapshot();
        Reply::Pulse(vm::model::Pulse { schema: vm::model::OPERATOR_SCHEMA, service_state: s.system.service,
            dsp: Some(s.system.dsp), keepers: Some(s.system.keepers), maintenance: Some(s.system.maintenance),
            pending_transactions: s.system.pending_transactions, cleanup_unconfirmed: Some(s.system.cleanup_unconfirmed),
            current_generation: "generation-a".into(), operation, operation_live: false })
    }
    fn completed(id: &str) -> serde_json::Value {
        serde_json::json!({"operation":id,"state":"completed","result":{"export":"ab".repeat(16),"sha256":"cd".repeat(32)}})
    }
    fn start(live: &mut Live) {
        if live.state.page != Page::Diagnostics { show_page(live, Page::Diagnostics, &snapshot()); }
        let id = export(live);
        assert_eq!(press(live, &id), Outcome::Changed);
        let Some(Query::Action(request)) = live.take_query() else { panic!("exact offered request") };
        assert_eq!(request.state_token, "exact-token");
        assert_eq!(request.action, Action::SupportExport {});
        assert!(live.take_query().is_none());
    }
    fn current_if_queued(live: &mut Live, operation: Option<serde_json::Value>) {
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        let mut s = snapshot(); s.operation = operation;
        live.receive(overview(s, "generation-a"));
    }

    fn same_render(a: &Built, b: &Built) -> bool {
        a.header == b.header && a.page == b.page && a.preview == b.preview
            && a.form == b.form && a.targets == b.targets
            && a.form_focus == b.form_focus && a.screen_from == b.screen_from
    }
    #[test]
    fn unchanged_background_reads_preserve_the_render_and_control_availability() {
        for page in [Page::Home, Page::Activity, Page::Diagnostics] {
            let mut live = ready();
            if page != Page::Home { show_page(&mut live, page, &snapshot()); }
            let before = live.built.clone();
            for current in [false, true] {
                live.last_current = Cx::monotonic_now() - if current { 6.0 } else { 0.0 };
                live.next_poll = Cx::monotonic_now(); live.tick();
                assert!(same_render(&before, &live.built), "queuing a background read changed {page:?}");
                assert!(matches!((current, live.take_query()),
                    (false, Some(Query::Pulse)) | (true, Some(Query::CurrentOverview))));
                assert!(same_render(&before, &live.built), "dispatching a background read changed {page:?}");
                live.receive(if current { overview(snapshot(), "generation-a") } else { pulse(None) });
                assert!(same_render(&before, &live.built), "an unchanged reply changed {page:?}");
            }
        }
    }
    #[test]
    fn failed_technical_read_remains_retryable_through_background_status_reads() {
        for page in [Page::Activity, Page::Diagnostics] {
            let mut live = ready(); live.state.page = page; live.need_details = true; live.drive();
            assert!(matches!(live.take_query(), Some(Query::Snapshot)));
            live.receive(Reply::Error("Full records read timed out".into()));
            for current in [false, true, true] {
                live.last_current = Cx::monotonic_now() - if current { 6.0 } else { 0.0 };
                live.next_poll = Cx::monotonic_now(); live.tick();
                let retry = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::RetryDetails));
                assert!(live.drawn[&retry].is_none());
                assert!(matches!((current, live.take_query()),
                    (false, Some(Query::Pulse)) | (true, Some(Query::CurrentOverview))));
                let retry = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::RetryDetails));
                assert!(live.drawn[&retry].is_none());
                live.receive(if current { overview(snapshot(), "generation-a") } else { pulse(None) });
                assert!(live.take_query().is_none(), "background read must not replay a failed technical read");
            }
            let retry = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::RetryDetails));
            assert_eq!(press(&mut live, &retry), Outcome::Changed);
            assert!(matches!(live.take_query(), Some(Query::Snapshot)));
            live.receive(Reply::Snapshot(Box::new(snapshot())));
            assert!(live.details.is_some());
        }
    }
    #[test]
    fn a_newer_full_snapshot_is_validated_without_an_endless_overview_snapshot_loop() {
        for page in [Page::Activity, Page::Diagnostics] {
            let mut live = ready(); live.state.page = page; live.need_details = true; live.drive();
            assert!(matches!(live.take_query(), Some(Query::Snapshot)));
            let mut newer = snapshot(); newer.state_token = "newer-owner-state".into();
            live.receive(Reply::Snapshot(Box::new(newer.clone())));
            assert!(!live.fresh);
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            live.receive(overview(newer.clone(), "generation-a"));
            assert_eq!(live.details.as_ref().map(|s| &s.state_token), Some(&newer.state_token));
            assert!(live.fresh);
            assert!(live.take_query().is_none(), "validated records must not be discarded and fetched again");
        }
    }
    #[test]
    fn failed_snapshot_validation_finishes_with_retry_instead_of_loading_forever() {
        let mut live = ready(); live.state.page = Page::Activity; live.need_details = true; live.drive();
        assert!(matches!(live.take_query(), Some(Query::Snapshot)));
        let mut newer = snapshot(); newer.state_token = "new-owner-state".into();
        live.receive(Reply::Snapshot(Box::new(newer)));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        live.receive(Reply::Error("Current owner status unavailable".into()));
        assert!(live.pending_details.is_none());
        let retry = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::RetryDetails));
        assert!(live.drawn[&retry].is_none());
        assert!(live.take_query().is_none());
    }
    #[test]
    fn a_click_during_a_background_read_is_queued_once_and_revalidated_before_submission() {
        for change in ["none", "token", "generation", "disabled", "read_error"] {
            let mut live = ready_support();
            live.last_current = Cx::monotonic_now() - 6.0;
            live.next_poll = Cx::monotonic_now(); live.tick();
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            let id = export(&live);
            assert_eq!(press(&mut live, &id), Outcome::Changed);
            assert_eq!(press(&mut live, &id), Outcome::Refused);
            assert!(live.take_query().is_none());
            let mut s = snapshot();
            if change == "token" { s.state_token = "new-token".into(); }
            if change == "disabled" { s.actions[0].disabled_reason = Some("Owner refused export".into()); }
            live.receive(if change == "read_error" { Reply::Error("status unavailable".into()) }
                else { overview(s, if change == "generation" { "generation-b" } else { "generation-a" }) });
            if change == "none" {
                let Some(Query::Action(request)) = live.take_query() else { panic!("one intentional report request") };
                assert_eq!(request.state_token, "exact-token");
                assert!(live.take_query().is_none());
            } else {
                assert!(!matches!(live.take_query(), Some(Query::Action(_))), "a changed or unavailable owner offer must not submit");
                let support = live.support.as_ref().unwrap();
                assert!(support.terminal && !support.acknowledgment_uncertain && support.operation.is_none());
            }
        }
    }
    #[test]
    fn bootstrap_replaces_live_authority_and_submits_only_its_exact_offered_step() {
        use lvb_client::bootstrap::{Completion, Operation, State as SetupState};
        let mut live = ready_support();
        let old = export(&live);
        let mut setup = SetupState::checking();
        assert_eq!(setup.take_operation(), Some(Operation::Status));
        setup.receive(Ok(Completion::UpdateActive));
        live.bootstrap(setup.panel(), false);
        assert_eq!(press(&mut live, &old), Outcome::Refused);
        assert!(live.take_query().is_none());
        assert!(!live.tick());
        let expected = setup.panel().controls[0].step;
        let update = target(&live, |t| matches!(t, Target::Bootstrap(step) if *step == expected));
        assert_eq!(press(&mut live, &update), Outcome::Changed);
        assert_eq!(press(&mut live, &update), Outcome::Refused);
        let step = live.take_bootstrap_step().unwrap();
        assert!(live.take_bootstrap_step().is_none());
        assert!(setup.press(step));
        assert_eq!(setup.take_operation(), Some(Operation::Update));
        live.bootstrap(setup.panel(), false);
        assert_eq!(press(&mut live, &update), Outcome::Refused);
        setup.receive(Err("Close the DAW before updating.".into()));
        live.bootstrap(setup.panel(), false);
        assert!(format!("{:?}", live.built.page).contains("Close the DAW"));
        assert_eq!(press(&mut live, &update), Outcome::Refused);
        assert!(live.take_bootstrap_step().is_none());
    }
    #[test]
    fn losing_the_setup_worker_ends_waiting_without_replaying_the_change() {
        let mut live = ready();
        let mut setup = lvb_client::bootstrap::State::checking();
        setup.take_operation();
        live.bootstrap(setup.panel(), false);
        live.worker_stopped("The setup worker stopped.".into());
        let panel = live.bootstrap_panel.as_ref().unwrap();
        assert!(panel.progress.is_none());
        assert!(panel.controls.is_empty());
        assert_eq!(panel.result.as_deref(), Some("The setup worker stopped."));
        assert!(live.take_bootstrap_step().is_none());
        assert!(live.take_query().is_none());
    }
    #[test]
    fn plugins_tab_returns_to_collection_and_preserves_search_filters() {
        let mut live = ready();
        let key = ProductKey::from(&snapshot().products[0]);
        live.state.go(&Destination::Product(key));
        live.state.plugins.search = "kept search".into();
        live.rebuild();
        let tab = target(&live, |target| matches!(target, Target::Tab(Tab::Plugins)));
        assert_eq!(press(&mut live, &tab), Outcome::Navigated);
        assert!(live.state.plugins.focus.is_none());
        assert_eq!(live.state.plugins.search, "kept search");
        assert!(live.product.is_none());
    }
    #[test]
    fn updater_choices_reach_the_worker_once_and_saved_links_never_enter_the_view() {
        let mut live = ready(); live.state.page = Page::Setup;
        live.update(vm::updates::Panel::build(false, false, false, None, &vm::updates::Phase::Idle, None), None, false, true);
        live.field("field.beta-link", "https://fixture.invalid/private/?pw=fixture-only");
        let save = target(&live, |t| matches!(t, Target::Update(vm::updates::Step::SaveLink)));
        assert_eq!(press(&mut live, &save), Outcome::Changed);
        assert_eq!(press(&mut live, &save), Outcome::Refused);
        let Some((vm::updates::Step::SaveLink, Some(draft))) = live.take_update_step() else { panic!("explicit link save") };
        assert_eq!(draft, "https://fixture.invalid/private/?pw=fixture-only");
        assert!(live.take_update_step().is_none());
        live.update(vm::updates::Panel::build(true, false, false, None, &vm::updates::Phase::Idle, None), None, false, true);
        assert!(live.ui.link_draft.is_empty());
        assert!(!format!("{:?}", live.built).contains("fixture-only"));
        live.update(vm::updates::Panel::build(true, false, true, None, &vm::updates::Phase::HandedOver, None), None, true, false);
        assert!(live.take_close_request());
        assert!(!live.take_close_request());
    }
    #[test]
    fn startup_has_no_fabricated_records_and_only_one_current_read() {
        let mut live = Live::new(String::new(), false);
        assert!(live.overview.is_none() && live.details.is_none() && live.product.is_none());
        assert!(live.built.targets.values().all(|t| !matches!(t, Target::Offer { .. })));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        assert!(live.take_query().is_none());
        live.receive(Reply::Error("service unavailable".into()));
        assert!(!live.fresh && live.overview.is_none());
    }
    fn readback(id: &str, record: serde_json::Value) -> Reply {
        Reply::Operation(vm::model::OperationReadback { schema: 1, operator_schema: vm::model::OPERATOR_SCHEMA,
            operation: id.into(), record })
    }
    #[test]
    fn report_completion_survives_an_unrelated_operation_becoming_latest() {
        let mut live = ready(); start(&mut live);
        live.receive(Reply::Receipt(Receipt { schema: vm::model::OPERATOR_SCHEMA, accepted: true,
            operation: Some("exact-operation".into()), refusal: None }));
        assert!(matches!(live.take_query(), Some(Query::Operation(id)) if id == "exact-operation"));
        live.receive(readback("exact-operation", serde_json::json!({"operation":"exact-operation","state":"running"})));
        current_if_queued(&mut live, Some(completed("someone-else")));
        assert!(live.support.as_ref().unwrap().report.is_none());
        live.next_poll = Cx::monotonic_now(); assert!(live.tick());
        assert!(matches!(live.take_query(), Some(Query::Operation(id)) if id == "exact-operation"));
        live.receive(readback("exact-operation", completed("exact-operation")));
        current_if_queued(&mut live, Some(completed("someone-else")));
        let save = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::SaveReport));
        assert_eq!(press(&mut live, &save), Outcome::Changed);
        let Some(Query::SaveSupportReport(report)) = live.take_query() else { panic!("save verified report identity") };
        assert_eq!(report.id, "ab".repeat(16)); assert_eq!(report.sha256, "cd".repeat(32));
    }
    // A completed export remains retained while Support reads its new records.
    fn report_ready_while_details_load() -> Live {
        let mut live = ready_support(); start(&mut live);
        live.receive(Reply::Receipt(Receipt { schema: vm::model::OPERATOR_SCHEMA, accepted: true,
            operation: Some("exact-operation".into()), refusal: None }));
        assert!(matches!(live.take_query(), Some(Query::Operation(id)) if id == "exact-operation"));
        live.receive(readback("exact-operation", completed("exact-operation")));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        let mut current = snapshot(); current.state_token = "export-completed".into();
        current.operation = Some(completed("exact-operation"));
        live.receive(overview(current, "generation-b"));
        assert!(matches!(live.take_query(), Some(Query::Snapshot)));
        assert!(live.support.as_ref().unwrap().report.is_some());
        live
    }
    fn page_says(live: &Live, words: &str) -> bool {
        fn contains(nodes: &[Node], words: &str) -> bool {
            nodes.iter().any(|node| match node {
                Node::Text(_, text) => text.contains(words),
                Node::Panel { children, .. } | Node::Fold { children, .. } | Node::Row(children)
                    | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => contains(children, words),
                _ => false,
            })
        }
        contains(&live.built.page, words)
    }
    #[test]
    fn superseded_support_snapshot_retries_once_without_replaying_the_report() {
        for moves_again in [false, true] {
            let mut live = report_ready_while_details_load();
            let mut older = snapshot(); older.state_token = "during-read".into();
            live.receive(Reply::Snapshot(Box::new(older)));
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            let mut current = snapshot(); current.state_token = "after-read".into();
            current.operation = Some(completed("someone-else"));
            live.receive(overview(current.clone(), "generation-c"));
            assert!(live.details.is_none());
            assert!(!page_says(&live, "could not be loaded"));
            assert!(matches!(live.take_query(), Some(Query::Snapshot)), "one automatic fresh read");
            if moves_again {
                let mut newer = current.clone(); newer.state_token = "moved-again".into();
                live.receive(Reply::Snapshot(Box::new(newer.clone())));
                assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
                live.receive(overview(newer.clone(), "generation-d"));
                assert!(page_says(&live, "records changed while loading"));
                assert!(!page_says(&live, "could not be loaded"));
                assert!(live.take_query().is_none(), "a moving owner must not cause an endless read loop");
                // A passive observation must not renew the automatic retry budget.
                live.need_current = true; live.drive();
                assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
                live.receive(overview(newer.clone(), "generation-e"));
                assert!(live.take_query().is_none());
                let retry = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::RetryDetails));
                assert_eq!(press(&mut live, &retry), Outcome::Changed);
                assert!(matches!(live.take_query(), Some(Query::Snapshot)));
                current = newer;
            }
            live.receive(Reply::Snapshot(Box::new(current)));
            assert!(live.details.is_some());
            assert!(live.take_query().is_none());
            let save = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::SaveReport));
            assert_eq!(press(&mut live, &save), Outcome::Changed);
            assert!(matches!(live.take_query(), Some(Query::SaveSupportReport(report))
                if report.id == "ab".repeat(16) && report.sha256 == "cd".repeat(32)));
        }
    }
    #[test]
    fn export_completion_during_the_full_read_settles_support_without_a_manual_retry() {
        let mut live = ready_support(); start(&mut live);
        live.receive(Reply::Receipt(Receipt { schema: vm::model::OPERATOR_SCHEMA, accepted: true,
            operation: Some("exact-operation".into()), refusal: None }));
        assert!(matches!(live.take_query(), Some(Query::Operation(_))));
        let running = serde_json::json!({"operation":"exact-operation","state":"running"});
        live.receive(readback("exact-operation", running.clone()));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        let mut current = snapshot(); current.state_token = "report-running".into(); current.operation = Some(running);
        live.receive(overview(current.clone(), "generation-b"));
        assert!(matches!(live.take_query(), Some(Query::Snapshot)));
        current.state_token = "report-completed".into(); current.operation = Some(completed("exact-operation"));
        live.receive(Reply::Snapshot(Box::new(current.clone())));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        live.receive(overview(current.clone(), "generation-c"));
        assert!(live.support.as_ref().unwrap().report.is_some());
        assert!(matches!(live.take_query(), Some(Query::Snapshot)));
        live.receive(Reply::Snapshot(Box::new(current)));
        assert!(live.details.is_some());
        assert!(live.take_query().is_none());
        assert!(page_says(&live, "Problem report created"));
        assert!(!page_says(&live, "could not be loaded"));
    }
    #[test]
    fn detail_failure_reason_survives_completed_report_and_save_until_explicit_retry() {
        let mut live = report_ready_while_details_load();
        live.receive(Reply::Error(format!("Full records read timed out: {}", "x".repeat(2048))));
        assert!(page_says(&live, "Full records read timed out"));
        assert!(!page_says(&live, &"x".repeat(1025)), "the displayed reason must be bounded");
        let save = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::SaveReport));
        assert_eq!(press(&mut live, &save), Outcome::Changed);
        assert!(matches!(live.take_query(), Some(Query::SaveSupportReport(_))));
        live.receive(Reply::Cancelled);
        assert!(page_says(&live, "Full records read timed out"));
        assert!(page_says(&live, "Saving cancelled"));
        live.need_current = true; live.drive();
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        let current = live.overview.as_ref().unwrap().current.clone();
        live.receive(overview(current.clone(), "generation-b"));
        assert!(page_says(&live, "Full records read timed out"));
        assert!(live.take_query().is_none(), "a real failed read needs explicit retry");
        let retry = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::RetryDetails));
        assert_eq!(press(&mut live, &retry), Outcome::Changed);
        assert!(matches!(live.take_query(), Some(Query::Snapshot)));
        assert!(!page_says(&live, "Full records read timed out"));
        live.receive(Reply::Snapshot(Box::new(current)));
        assert!(live.details.is_some());
        assert!(live.support.as_ref().unwrap().report.is_some());
    }
    #[test]
    fn only_the_exact_changed_read_refusal_gets_one_validated_retry() {
        for (error, stale) in [("operator_state_changed_refresh", true),
            ("Error: \"operator_state_changed_refresh\"\n", true),
            ("read failed after operator_state_changed_refresh: fixture failure", false)] {
            for validation_fails in [false, true] {
                let mut live = report_ready_while_details_load();
                live.receive(Reply::Error(error.into()));
                assert_eq!(page_says(&live, "could not be loaded"), !stale);
                if stale {
                    assert!(!live.fresh);
                    assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
                    let current = live.overview.as_ref().unwrap().current.clone();
                    if validation_fails {
                        live.receive(Reply::Error("Current status unavailable".into()));
                        assert!(page_says(&live, "Current status could not validate this page"));
                    } else {
                        live.receive(overview(current.clone(), "generation-b"));
                        assert!(matches!(live.take_query(), Some(Query::Snapshot)));
                        live.receive(Reply::Snapshot(Box::new(current)));
                        assert!(live.details.is_some());
                    }
                }
                assert!(live.take_query().is_none(), "no read loop or action replay");
                assert!(live.support.as_ref().unwrap().report.is_some());
                if !stale || validation_fails {
                    let retry = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::RetryDetails));
                    assert!(live.drawn[&retry].is_none());
                }
            }
        }
    }
    #[test]
    fn a_superseded_window_labels_retained_health_as_paused() {
        for reply in [false, true] {
            let mut live = ready_support();
            if reply {
                live.need_current = true; live.drive();
                assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
                live.receive(Reply::FrontendSuperseded);
            } else { live.pause_superseded(); }
            let header = format!("{:?}", live.built.header);
            assert!(header.contains("Last observed status · window paused"));
            assert!(!header.contains("refreshing"));
            assert!(!header.contains("status updates automatically"));
            assert!(live.take_query().is_none());
            assert!(live.built.targets.values().all(|t| !matches!(t, Target::Offer { offer, .. }
                if offer.availability.enabled())));
        }
    }
    #[test]
    fn mutation_refresh_retains_last_observed_health_but_a_failed_read_does_not() {
        let mut live = ready_support();
        let before = format!("{:?}", live.built.header);
        assert!(!before.contains("unavailable"));
        start(&mut live);
        live.receive(Reply::Receipt(Receipt { schema: vm::model::OPERATOR_SCHEMA, accepted: true,
            operation: Some("exact-operation".into()), refusal: None }));
        let refreshing = format!("{:?}", live.built.header);
        assert!(!refreshing.contains("Bridge service status unavailable"));
        assert!(refreshing.contains("Last observed status"));
        assert!(!live.fresh);
        assert!(live.built.targets.values().all(|t| !matches!(t, Target::Offer { offer, .. }
            if offer.availability.enabled())));
        assert!(matches!(live.take_query(), Some(Query::Operation(_))));
        live.receive(readback("exact-operation", completed("exact-operation")));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        live.receive(Reply::Error("service read unavailable".into()));
        assert!(format!("{:?}", live.built.header).contains("Bridge service status unavailable"));
    }
    #[test]
    fn support_draws_the_exact_primary_export_once() {
        fn count(nodes: &[Node], targets: &[&str]) -> usize {
            nodes.iter().map(|node| match node {
                Node::Actions { actions, .. } => actions.iter().filter(|a| targets.contains(&a.id.0.as_str())).count(),
                Node::Panel { children, .. } | Node::Fold { children, .. } | Node::Row(children)
                    | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => count(children, targets),
                _ => 0,
            }).sum()
        }
        let live = ready_support();
        let exports: Vec<_> = live.built.targets.iter().filter_map(|(id, target)|
            matches!(target, Target::Offer { offer, .. } if matches!(offer.intent, Action::SupportExport {}))
                .then_some(id.as_str())).collect();
        assert_eq!(count(&live.built.page, &exports), 1);
    }
    #[test]
    fn wrong_or_missing_retained_result_never_substitutes_another_report() {
        for wrong_identity in [false, true] {
            let mut live = ready(); start(&mut live);
            live.receive(Reply::Receipt(Receipt { schema: vm::model::OPERATOR_SCHEMA, accepted: true,
                operation: Some("exact-operation".into()), refusal: None }));
            assert!(matches!(live.take_query(), Some(Query::Operation(id)) if id == "exact-operation"));
            live.receive(if wrong_identity { readback("other-operation", completed("other-operation")) }
                else { Reply::Error("Retained result unavailable".into()) });
            current_if_queued(&mut live, Some(completed("other-operation")));
            assert!(live.support.as_ref().unwrap().report.is_none());
            assert!(!live.support.as_ref().unwrap().terminal);
            live.next_poll = Cx::monotonic_now(); live.tick();
            assert!(matches!(live.take_query(), Some(Query::Operation(id)) if id == "exact-operation"));
        }
    }
    #[test]
    fn overview_failures_do_not_starve_the_retained_report_or_replay_its_action() {
        let mut live = ready(); start(&mut live);
        live.receive(Reply::Receipt(Receipt { schema: vm::model::OPERATOR_SCHEMA, accepted: true,
            operation: Some("exact-operation".into()), refusal: None }));
        for _ in 0..3 {
            assert!(matches!(live.take_query(), Some(Query::Operation(id)) if id == "exact-operation"));
            live.receive(readback("exact-operation", serde_json::json!({"operation":"exact-operation","state":"running"})));
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            live.receive(Reply::Error("current status unavailable".into()));
            assert!(!live.fresh);
            live.next_poll = Cx::monotonic_now(); assert!(live.tick());
        }
        assert!(matches!(live.take_query(), Some(Query::Operation(id)) if id == "exact-operation"));
        live.receive(readback("exact-operation", completed("exact-operation")));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        live.receive(Reply::Error("current status still unavailable".into()));
        assert!(!live.fresh);
        let save = target(&live, |t| matches!(t, Target::Control(c) if c.does == Local::SaveReport));
        assert_eq!(press(&mut live, &save), Outcome::Changed);
        assert!(matches!(live.take_query(), Some(Query::SaveSupportReport(report)) if report.id == "ab".repeat(16)));
    }
    #[test]
    fn an_unconfirmed_ack_never_adopts_a_similar_operation_or_replays_export() {
        let mut live = ready(); start(&mut live);
        live.receive(Reply::Error("connection closed after submission".into()));
        assert!(live.support.as_ref().unwrap().acknowledgment_uncertain);
        assert!(matches!(live.take_query(), Some(Query::Pulse)));
        live.receive(pulse(Some(completed("other-export"))));
        current_if_queued(&mut live, Some(completed("other-export")));
        assert!(live.support.as_ref().unwrap().report.is_none());
        assert!(live.support.as_ref().unwrap().text.contains("unconfirmed"));
        let id = export(&live);
        assert_eq!(press(&mut live, &id), Outcome::Refused);
        live.next_poll = Cx::monotonic_now(); assert!(live.tick());
        assert!(matches!(live.take_query(), Some(Query::Pulse)));
    }
    #[test]
    fn completed_report_with_malformed_identity_cannot_be_saved() {
        let mut support = Support::captured(Action::SupportExport {}); support.operation = Some("owned".into());
        let mut op = completed("owned"); op["result"]["export"] = "../other".into();
        support.observe(&op);
        assert!(support.terminal && support.report.is_none());
    }
    #[test]
    fn old_display_ids_do_not_retarget_new_offers() {
        let mut live = ready_support(); let old = export(&live);
        refresh(&mut live);
        assert!(matches!(live.take_query(), Some(Query::Overview)));
        let mut s = snapshot(); s.state_token = "replacement-token".into();
        live.receive(interactive(s, "generation-b"));
        assert_eq!(press(&mut live, &old), Outcome::Refused);
        assert!(matches!(live.take_query(), Some(Query::Snapshot)));
        let s = live.overview.as_ref().unwrap().current.clone();
        live.receive(Reply::Snapshot(Box::new(s)));
        let current = export(&live); press(&mut live, &current);
        let Some(Query::Action(request)) = live.take_query() else { panic!("new exact offer") };
        assert_eq!(request.state_token, "replacement-token");
    }
    #[test]
    fn stale_product_reply_cannot_change_the_current_focus() {
        let mut live = ready();
        let a = ProductKey::from(&live.overview.as_ref().unwrap().current.products[0]);
        let b = ProductKey::from(&live.overview.as_ref().unwrap().current.products[1]);
        live.state.go(&Destination::Product(a.clone())); live.drive(); live.rebuild();
        assert!(matches!(live.take_query(), Some(Query::Product(key)) if key == a));
        let s = live.overview.as_ref().unwrap().current.clone();
        live.state.go(&Destination::Product(b.clone())); live.product_attempted = false; live.rebuild();
        live.receive(Reply::Product(Box::new(CurrentProductDetail { schema: 1, operator_schema: vm::model::OPERATOR_SCHEMA,
            state_token: s.state_token.clone(), current_generation: "generation-a".into(), system: s.system.clone(),
            product: s.products[0].clone(), environments: s.environments.clone(), vendor_applications: s.vendor_applications.clone() })));
        assert_eq!(live.state.plugins.focused_product(), Some(&b));
        assert!(live.product.is_none());
        current_if_queued(&mut live, None);
        assert!(matches!(live.take_query(), Some(Query::Product(key)) if key == b));
    }
    #[test]
    fn background_recovery_retires_only_the_read_error_and_keeps_the_open_page() {
        for notice in ["", "A separate updater notice"] {
            let mut live = Live::new("test".into(), false);
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            live.receive(Reply::Error("initial read unavailable".into()));
            let support = target(&live, |target| matches!(target, Target::Tab(Tab::Support)));
            assert_eq!(press(&mut live, &support), Outcome::Navigated);
            assert!(!live.details_attempted, "An absent overview must not consume the details read");
            live.note(notice.into());
            live.queue(Query::Pulse, Flight::Pulse);
            assert!(matches!(live.take_query(), Some(Query::Pulse)));
            live.receive(pulse(None));
            assert!(format!("{:?}", live.built.page).contains("initial read unavailable"));
            live.queue(Query::CurrentOverview, Flight::Current);
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            live.receive(overview(snapshot(), "generation-a"));
            assert!(!format!("{:?}", live.built.page).contains("initial read unavailable"));
            assert_eq!(live.message, notice);
            assert_eq!(live.state.page, Page::Diagnostics);
            assert!(matches!(live.take_query(), Some(Query::Snapshot)));
            live.receive(Reply::Snapshot(Box::new(snapshot())));
            assert!(live.details.is_some());
            if !notice.is_empty() { assert!(format!("{:?}", live.built.page).contains(notice)); }
        }
    }

    #[test]
    fn failed_initial_status_settles_and_retry_reads_without_submitting_an_action() {
        let mut live = Live::new("test".into(), false);
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        live.receive(Reply::Error("artifact missing or changed".into()));
        for page in [Page::Home, Page::Setup, Page::Plugins, Page::Activity, Page::Diagnostics] {
            live.state.page = page; live.rebuild();
            assert!(live.built.page.iter().any(|node| matches!(node,
                Node::Text(_, words) if words == "Current status could not be loaded. Try again.")));
            let retry = target(&live, |target| matches!(target,
                Target::Control(control) if control.does == Local::Refresh));
            assert!(live.drawn[&retry].is_none());
        }
        live.state.page = Page::Home; live.rebuild();
        let retry = target(&live, |target| matches!(target,
            Target::Control(control) if control.does == Local::Refresh));
        assert_eq!(press(&mut live, &retry), Outcome::Changed);
        let retry = target(&live, |target| matches!(target,
            Target::Control(control) if control.does == Local::Refresh));
        assert!(live.drawn[&retry].is_some(), "Only one retry may be queued");
        assert!(matches!(live.take_query(), Some(Query::Overview)));
        assert!(live.take_query().is_none());
        assert!(live.current_read_error.is_some(), "Keep the settled failure while retrying");
        live.receive(interactive(snapshot(), "generation-a"));
        assert!(live.current_read_error.is_none());
        assert!(live.fresh && live.overview.is_some());
        assert!(live.support.is_none(), "Retry must not submit an operation");
    }

    #[test]
    fn pending_product_read_shows_loading_and_only_an_error_offers_exact_retry() {
        fn contains(nodes: &[Node], words: &str) -> bool {
            nodes.iter().any(|node| match node {
                Node::Text(_, text) => text.contains(words),
                Node::Panel { children, .. } | Node::Fold { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => contains(children, words),
                _ => false,
            })
        }
        fn loading(live: &Live) {
            assert!(contains(&live.built.page, "Loading this plug-in's setup and settings"));
            assert!(!contains(&live.built.page, "could not be loaded"));
            assert!(!live.built.targets.values().any(|target| matches!(target,
                Target::Control(control) if control.does == Local::RetryDetails)));
        }
        fn failed(live: &Live) {
            assert!(contains(&live.built.page, "Plug-in controls could not be loaded"));
            assert!(!contains(&live.built.page, "Loading this plug-in's setup and settings"));
        }
        for fail_first in [false, true] {
            let mut live = ready();
            let s = live.overview.as_ref().unwrap().current.clone();
            let key = ProductKey::from(&s.products[0]);
            // Opening the product can happen while a background overview is
            // still pending. The product then queues behind that exact read.
            live.last_current = Cx::monotonic_now() - 6.0;
            live.next_poll = Cx::monotonic_now(); live.tick();
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            live.state.go(&Destination::Product(key.clone())); live.drive(); live.rebuild();
            loading(&live);
            live.receive(overview(s.clone(), "generation-a"));
            loading(&live);
            assert!(matches!(live.take_query(), Some(Query::Product(asked)) if asked == key));
            loading(&live);
            if fail_first {
                live.receive(Reply::Error("Exact product read unavailable".into()));
                failed(&live);
                // Unrelated reads do not retry this product or erase its error,
                // either while queued, in flight, or after the same-state reply.
                for current in [false, true] {
                    live.last_current = Cx::monotonic_now() - if current { 6.0 } else { 0.0 };
                    live.next_poll = Cx::monotonic_now(); assert!(live.tick());
                    failed(&live);
                    let query = live.take_query();
                    assert!(matches!((current, query),
                        (false, Some(Query::Pulse)) | (true, Some(Query::CurrentOverview))));
                    failed(&live);
                    live.receive(if current { overview(s.clone(), "generation-a") } else { pulse(None) });
                    failed(&live);
                    assert!(live.take_query().is_none());
                }
                let retry = target(&live, |target| matches!(target,
                    Target::Control(control) if control.does == Local::RetryDetails));
                assert_eq!(press(&mut live, &retry), Outcome::Changed);
                loading(&live);
                assert!(matches!(live.take_query(), Some(Query::Product(asked)) if asked == key));
                loading(&live);
            }
            live.receive(Reply::Product(Box::new(CurrentProductDetail {
                schema: 1, operator_schema: vm::model::OPERATOR_SCHEMA,
                state_token: s.state_token.clone(), current_generation: "generation-a".into(),
                system: s.system.clone(), product: s.products[0].clone(),
                environments: s.environments.clone(), vendor_applications: s.vendor_applications.clone(),
            })));
            assert!(live.product.is_some());
            assert!(!contains(&live.built.page, "could not be loaded"));
            assert!(!contains(&live.built.page, "Loading this plug-in's setup and settings"));
            assert!(live.take_query().is_none());
        }
    }
    #[test]
    fn disabled_owner_reason_survives_the_frontend_limitation() {
        let mut live = ready_support();
        let mut s = snapshot(); s.actions[0].disabled_reason = Some("Owner export unavailable".into());
        live.overview.as_mut().unwrap().current = s;
        live.rebuild();
        let id = export(&live);
        assert_eq!(live.drawn[&id].as_deref(), Some("Owner export unavailable"));
        assert_eq!(press(&mut live, &id), Outcome::Refused);
        assert!(live.take_query().is_none());
    }
    fn ready_case(page: Page, name: &str) -> Live {
        let sample = Sample::new(Case::new(page, name));
        let mut live = Live::new(String::new(), false);
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        live.receive(overview(sample.snapshot.clone(), "generation-a"));
        live.state = sample.state(); live.reveal = true;
        if page == Page::Plugins { live.drive(); finish_page_read(&mut live, page, &sample.snapshot); }
        live.rebuild(); live
    }
    #[test]
    fn confirmation_cancels_safely_and_only_the_current_form_submits_its_exact_action() {
        let mut live = ready_case(Page::Setup, "setup_removal");
        let offer = target(&live, |t| matches!(t, Target::Offer { offer, .. } if matches!(offer.intent, Action::SetupDelete { .. })));
        assert_eq!(press(&mut live, &offer), Outcome::Changed);
        assert!(live.take_query().is_none());
        let retired = target(&live, |t| matches!(t, Target::Form(FormStep::Confirm)));
        live.dismiss_form("Escape");
        assert!(live.take_query().is_none());
        assert_eq!(press(&mut live, &offer), Outcome::Changed);
        assert_eq!(press(&mut live, &retired), Outcome::Refused);
        let confirm = target(&live, |t| matches!(t, Target::Form(FormStep::Confirm)));
        assert_eq!(press(&mut live, &confirm), Outcome::Changed);
        let Some(Query::Action(request)) = live.take_query() else { panic!("confirmed exact request") };
        assert!(matches!(request.action, Action::SetupDelete { confirmed: true, forget_installer: false, .. }));
        assert!(live.take_query().is_none());
    }
    #[test]
    fn rename_and_new_owner_generation_cannot_retarget_an_open_form() {
        for changed in [false, true] {
            let mut live = ready_case(Page::Setup, "setup_many");
            let rename = target(&live, |t| matches!(t, Target::Offer { offer, .. } if matches!(offer.intent, Action::InstallerRename { .. })));
            assert_eq!(press(&mut live, &rename), Outcome::Changed);
            live.field("field.rename", "My instrument setup");
            if changed {
                live.last_current = Cx::monotonic_now() - 6.0;
                live.next_poll = Cx::monotonic_now(); live.tick();
                assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
                let s = live.overview.as_ref().unwrap().current.clone();
                live.receive(overview(s, "generation-b"));
            }
            let save = target(&live, |t| matches!(t, Target::Form(FormStep::SaveRename)));
            if changed {
                assert_eq!(press(&mut live, &save), Outcome::Refused);
                assert!(live.take_query().is_none());
            } else {
                assert_eq!(press(&mut live, &save), Outcome::Changed);
                assert!(matches!(live.take_query(), Some(Query::Action(Request { action: Action::InstallerRename { label, .. }, .. })) if label == "My instrument setup"));
            }
        }
    }
    #[test]
    fn an_intentional_click_before_background_dispatch_is_not_dropped() {
        let mut live = ready_support();
        live.next_poll = Cx::monotonic_now(); live.tick();
        let id = export(&live); assert_eq!(press(&mut live, &id), Outcome::Changed);
        assert!(matches!(live.take_query(), Some(Query::Action(_))));
        assert!(live.take_query().is_none());
    }
    #[test]
    fn installer_import_readiness_and_selected_manager_handoff_use_native_client_hooks() {
        let mut live = ready_case(Page::Setup, "setup_many");
        let installer = live.overview.as_ref().unwrap().current.installer_setups[0].installer.clone();
        live.effect(Effect::ChooseInstaller);
        assert!(matches!(live.take_query(), Some(Query::PickInstaller)));
        live.receive(Reply::Imported(vm::model::InstallerImportResult { schema: 2, installer_sha256: installer.clone(),
            byte_size: 10, format: "exe".into(), newly_imported: false, display_label: "My installer".into(), created_at: 1 }));
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
        let snapshot = live.overview.as_ref().unwrap().current.clone();
        live.receive(overview(snapshot.clone(), "generation-a"));
        assert_eq!(live.state.page, Page::Setup);
        assert_eq!(live.state.focus.setup_installer.as_ref(), Some(&installer));
        refresh(&mut live);
        assert!(matches!(live.take_query(), Some(Query::Overview)));
        live.receive(interactive(snapshot, "generation-a"));
        assert_eq!(live.readiness.as_ref().map(|(_, bound)| bound), live.binding().as_ref());
        live.superseded = true; live.fresh = false;
        live.effect(Effect::OpenCurrentManager);
        assert!(matches!(live.take_query(), Some(Query::OpenCurrentManager)));
        live.receive(Reply::CurrentManagerOpened);
        assert!(live.superseded && !live.fresh);
    }
    #[test]
    fn superseded_frontend_stops_all_queries_and_preserves_unknown_ack() {
        let mut live = ready(); start(&mut live);
        live.receive(Reply::FrontendSuperseded);
        assert!(live.superseded && !live.fresh);
        assert!(live.support.is_some());
        live.next_poll = Cx::monotonic_now(); assert!(!live.tick());
        assert!(live.take_query().is_none());
    }
    #[test]
    fn navigation_and_same_binding_offers_keep_ids_through_idle_reads() {
        let mut live = ready_support();
        let nav = target(&live, |t| matches!(t, Target::Tab(Tab::Plugins)));
        let offered = export(&live);
        live.next_poll = Cx::monotonic_now(); assert!(live.tick());
        assert!(live.built.targets.contains_key(&nav));
        assert!(matches!(live.take_query(), Some(Query::Pulse)));
        live.receive(pulse(None));
        assert!(live.built.targets.contains_key(&nav));
        assert_eq!(export(&live), offered);
        assert_eq!(press(&mut live, &nav), Outcome::Navigated);
    }
    #[test]
    fn same_generation_counter_change_and_six_second_bound_refresh_current() {
        let mut live = ready();
        live.next_poll = Cx::monotonic_now(); assert!(live.tick());
        assert!(matches!(live.take_query(), Some(Query::Pulse)));
        let Reply::Pulse(mut changed) = pulse(None) else { unreachable!() };
        changed.dsp = Some(9);
        live.receive(Reply::Pulse(changed));
        assert!(!live.fresh);
        current_if_queued(&mut live, None);
        live.last_current = Cx::monotonic_now() - 6.0;
        live.next_poll = Cx::monotonic_now(); assert!(live.tick());
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
    }
    #[test]
    fn duplicate_exact_product_records_never_grant_a_product_projection() {
        let mut live = ready();
        let s = &mut live.overview.as_mut().unwrap().current;
        let product = s.products[0].clone();
        s.products.push(product.clone());
        let detail = CurrentProductDetail { schema: 1, operator_schema: vm::model::OPERATOR_SCHEMA,
            state_token: s.state_token.clone(), current_generation: "generation-a".into(), system: s.system.clone(),
            product, environments: s.environments.clone(), vendor_applications: s.vendor_applications.clone() };
        assert!(live.project_product(detail).is_none());
    }

    #[test]
    fn an_accepted_receipt_without_identity_stays_uncertain() {
        let mut live = ready(); start(&mut live);
        live.receive(Reply::Receipt(Receipt { schema: vm::model::OPERATOR_SCHEMA, accepted: true,
            operation: None, refusal: None }));
        assert!(matches!(live.take_query(), Some(Query::Pulse)));
        live.receive(pulse(Some(completed("unrelated"))));
        assert!(live.support.as_ref().unwrap().acknowledgment_uncertain);
        assert!(live.support.as_ref().unwrap().operation.is_none());
        assert!(live.support.as_ref().unwrap().report.is_none());
        assert!(live.support.as_ref().unwrap().text.contains("unconfirmed"));
    }
    #[test]
    fn failed_status_read_recovers_through_current_overview_not_a_pulse_claim() {
        let mut live = ready();
        live.next_poll = Cx::monotonic_now(); live.tick();
        assert!(matches!(live.take_query(), Some(Query::Pulse)));
        live.receive(Reply::Error("service unavailable".into()));
        assert!(!live.fresh);
        live.next_poll = Cx::monotonic_now(); live.tick();
        assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
    }

    fn show_page(live: &mut Live, page: Page, s: &Snapshot) {
        live.state.page = page;
        if page == Page::Plugins {
            live.state.go(&Destination::Product(ProductKey::from(&s.products[0])));
        } else {
            live.need_details = true;
        }
        live.drive(); live.rebuild();
        finish_page_read(live, page, s);
    }
    fn finish_page_read(live: &mut Live, page: Page, s: &Snapshot) {
        match (page, live.take_query()) {
            (Page::Plugins, Some(Query::Product(key))) => {
                assert_eq!(key, ProductKey::from(&s.products[0]));
                live.receive(Reply::Product(Box::new(CurrentProductDetail {
                    schema: 1, operator_schema: vm::model::OPERATOR_SCHEMA,
                    state_token: s.state_token.clone(), current_generation: "generation-a".into(),
                    system: s.system.clone(), product: s.products[0].clone(),
                    environments: s.environments.clone(), vendor_applications: s.vendor_applications.clone(),
                })));
            }
            (Page::Activity | Page::Diagnostics, Some(Query::Snapshot)) => {
                live.receive(Reply::Snapshot(Box::new(s.clone())));
            }
            _ => panic!("the opened page must read its own current records"),
        }
    }
    #[test]
    fn explicit_refresh_rereads_open_pages_even_when_binding_does_not_change() {
        for page in [Page::Plugins, Page::Activity, Page::Diagnostics] {
            for changed_counters in [false, true] {
                let mut live = ready(); let mut s = snapshot();
                show_page(&mut live, page, &s);
                refresh(&mut live);
                assert!(live.product.is_none() && live.details.is_none());
                assert!(matches!(live.take_query(), Some(Query::Overview)));
                if changed_counters { s.system.dsp += 1; }
                live.receive(interactive(s.clone(), "generation-a"));
                finish_page_read(&mut live, page, &s);
                assert_eq!(live.product.as_ref().or(live.details.as_ref()).unwrap().system, s.system);
            }
        }
    }
    #[test]
    fn observed_system_changes_retire_old_page_readings_without_a_token_change() {
        for page in [Page::Plugins, Page::Activity, Page::Diagnostics] {
            let mut live = ready(); let mut s = snapshot();
            show_page(&mut live, page, &s);
            live.last_current = Cx::monotonic_now() - 6.0;
            live.next_poll = Cx::monotonic_now(); live.tick();
            assert!(matches!(live.take_query(), Some(Query::CurrentOverview)));
            s.system.dsp += 1;
            live.receive(overview(s.clone(), "generation-a"));
            assert!(live.product.is_none() && live.details.is_none());
            finish_page_read(&mut live, page, &s);
        }
    }

}

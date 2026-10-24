//! The preview's controller: one sample's records, the window's `State`, and
//! what every press does. No toolkit and no service: a pressed offer goes
//! through `State::press_offer`, a window control through `State::apply`,
//! and an action that would be sent is captured and logged instead, with
//! `State::sent` closing the forms that asked for it, as a send would.

use {
    crate::presentation::{self, After, Built, FormStep, ListStep, LocalUi, Node, PreviewStep, Tab, Target, Why},
    lvb_view_model::{
        forms,
        model::{Action, ProblemCategory, TestResultKind},
        samples::{self, Case, Sample, REQUESTS, STATES},
        screen::{Records, Window},
        state::{Effect, State},
        updates,
        window::Page,
        Destination, Local, ProductKey,
    },
    makepad_product_ui::{ActionView, InputPath},
    std::collections::BTreeMap,
};

/// The distribution name Diagnostics shows. Sample records: a frontend on a
/// real computer reads its own.
pub const DISTRIBUTION: &str = "Example OS";

/// The pages the sample switcher offers.
pub const PAGES: [Page; 5] = [Page::Home, Page::Setup, Page::Plugins, Page::Activity, Page::Diagnostics];

/// Which sample is loaded: a named state, the page its records were made
/// for, and the request and update samples drawn over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selection {
    pub state: usize,
    pub page: Page,
    pub request: Option<usize>,
    pub update: Option<usize>,
}

impl Default for Selection {
    fn default() -> Self {
        Self { state: STATES.iter().position(|state| *state == "populated").unwrap_or(0), page: Page::Home,
            request: None, update: None }
    }
}

impl Selection {
    /// From `--state=`, `--page=`, `--request=` and `--update=`.
    pub fn from_args(args: &[String]) -> Self {
        let value = |flag: &str| args.iter().find_map(|arg| arg.strip_prefix(flag)).map(str::to_owned);
        let mut selection = Self::default();
        if let Some(state) = value("--state=").and_then(|name| STATES.iter().position(|s| *s == name)) {
            selection.state = state;
        }
        if let Some(page) = value("--page=").and_then(|key| Page::from_key(&key)) {
            selection.page = page;
        }
        selection.request = value("--request=").and_then(|name| REQUESTS.iter().position(|r| *r == name));
        selection.update = value("--update=").and_then(|name| samples::updates().iter().position(|(u, _)| *u == name));
        selection
    }

    pub fn state_name(&self) -> &'static str {
        STATES[self.state]
    }
}

pub use crate::Outcome;

#[derive(Clone)]
pub struct Preview {
    pub selection: Selection,
    pub sample: Sample,
    pub state: State,
    pub ui: LocalUi,
    pub built: Built,
    /// Every control drawn, with its printed reason when it is held.
    pub drawn: BTreeMap<String, Option<String>>,
    /// Every action captured instead of sent, in order, with whether a
    /// confirmation answered it.
    pub captured: Vec<(Action, bool)>,
    pub log: Vec<String>,
    log_read: usize,
    /// Draw every option's content (tests and full-page captures).
    pub reveal: bool,
    /// The window's posture in words, for the preview bar.
    pub posture: String,
    /// Each opened form has its own input lifetime, even if it concerns the
    /// same setup. An event retained from a closed form cannot press a new one.
    form_generation: u64,
    activity_fixture: Option<lvb_view_model::model::Activity>,
}

impl Preview {
    /// Exact synthetic restart metadata and current offer for hidden UI checks.
    pub fn recovery_fixture(&mut self) {
        self.activity_fixture = Some(samples::recovery_fixture(&mut self.sample.snapshot));
        self.state.go(&Destination::Product(ProductKey::from(&self.sample.snapshot.products[0])));
        self.rebuild();
    }
    /// Owner-shaped fixture only; no service or real window is opened.
    pub fn display_fixture(&mut self) {
        samples::display_fixture(&mut self.sample.snapshot);
        self.state.go(&Destination::Product(ProductKey::from(&self.sample.snapshot.products[0])));
        self.rebuild();
    }
    /// Same native page and forms as the installed window, with synthetic folders.
    pub fn library_locations_fixture(&mut self) {
        samples::library_locations_fixture(&mut self.sample.snapshot);
        self.state.plugins.show_page(ProductKey::from(&self.sample.snapshot.products[0]),
            lvb_view_model::state::ProductPage::SampleLocations);
        self.state.page = Page::Plugins;
        self.rebuild();
    }
    /// Sample-only report data for rendered interaction checks; never writes a file.
    pub fn report_fixture(&mut self) {
        self.state.page = Page::Diagnostics;
        self.ui.report = Some(presentation::support::Report::new(vm_report()));
        self.rebuild();
    }
    /// Synthetic display stress case for hidden native layout checks. This
    /// module is sample-only; neither its words nor its actions enter live UI.
    pub fn long_row_fixture(&mut self) {
        if let Some(product) = self.sample.snapshot.products.first_mut() {
            product.name = "Layout fixture: a very long orchestral instrument with multiple microphone positions and an extended performance edition".into();
            product.vendor = "Example Instrument Company with a deliberately long international studio and sound design division name".into();
        }
        self.rebuild();
        let mut rows: Vec<_> = self.built.page.iter().filter(|node|
            matches!(node, Node::LibraryRow { .. })).cloned().collect();
        for (index, node) in rows.iter_mut().enumerate() {
            let Node::LibraryRow { action, status, .. } = node else { continue };
            match index {
                0 => *status = Some(makepad_product_ui::StatusView::new(
                    makepad_product_ui::StatusClass::Unknown,
                    "Not tried with this DAW and operating system; compatibility has not been established")),
                1 => {
                    action.label = "Unavailable fixture row".into();
                    action.availability = makepad_product_ui::Availability::Disabled {
                        reason: "Waiting for an exact owner readback before this action can be used.".into(),
                    };
                    action.consequence = Some("This would open only the selected plug-in.".into());
                }
                2 => {
                    action.label = "Pending fixture row".into();
                    action.pending = Some(makepad_product_ui::Pending::Reported(
                        "Owner is checking the selected plug-in and waiting for its window to finish closing.".into()));
                }
                _ => {}
            }
        }
        self.built.page = rows;
        self.built.screen_from = 0;
        self.drawn.clear();
        collect(&self.built.header, &mut self.drawn);
        collect(&self.built.page, &mut self.drawn);
    }

    pub fn new(selection: Selection) -> Self {
        let sample = Self::sample_for(selection);
        let mut preview = Self { selection, state: sample.state(), sample, ui: LocalUi::default(),
            built: Built::default(), drawn: BTreeMap::new(), captured: vec![], log: vec![], log_read: 0,
            reveal: false, posture: String::new(), form_generation: 0, activity_fixture: None };
        preview.load();
        preview
    }

    fn sample_for(selection: Selection) -> Sample {
        let mut sample = Sample::new(Case::new(selection.page, STATES[selection.state]));
        if let Some(request) = selection.request {
            sample = sample.with_request(REQUESTS[request]);
        }
        if let Some(update) = selection.update {
            sample = sample.with_update(samples::updates()[update].0);
        }
        sample
    }

    /// Loads the selected sample: its records, and the window's state as the
    /// sample sets it. New records go through `State::observe`.
    pub fn load(&mut self) {
        self.form_generation += 1;
        self.sample = Self::sample_for(self.selection);
        self.state = self.sample.state();
        self.state.observe(&self.sample.snapshot);
        self.ui = LocalUi::default();
        self.activity_fixture = None;
        self.rebuild();
    }

    /// The manager's records as this window holds them. The opened plug-in's
    /// own records are the sample's, once exactly one plug-in is open: the
    /// samples stand in for every read the window would make.
    pub fn records(&self) -> Records<'_> {
        let snapshot = &self.sample.snapshot;
        let opened = self.state.plugins.focused_product().filter(|key|
            snapshot.products.iter().filter(|product| ProductKey::from(*product) == **key).count() == 1);
        Records { product: self.sample.records().product.or(opened.map(|_| snapshot)),
            activity: self.activity_fixture.as_ref().map(lvb_view_model::diagnostics::Section::Loaded)
                .or(self.sample.records().activity), ..self.sample.records() }
    }

    pub fn window(&self) -> Window {
        Window::build(&self.records(), &self.sample.reading(Some(DISTRIBUTION)), &self.state)
    }

    fn why(&self) -> Why {
        let reading = self.sample.reading(Some(DISTRIBUTION));
        Why { fresh: reading.fresh, pending: reading.controls_pending, in_flight: reading.in_flight,
            superseded: reading.superseded }
    }

    pub fn rebuild(&mut self) {
        let window = self.window();
        let records = self.records();
        let panel = self.sample.update.as_ref().map(samples::UpdateSample::panel);
        let built = presentation::build(&window, Some(&self.sample.snapshot), records.actions, &self.state, &self.ui,
            self.why(), panel.as_ref(), self.reveal, self.form_generation);
        self.built = built;
        self.built.preview = self.preview_bar();
        self.drawn = BTreeMap::new();
        let lists: Vec<Vec<Node>> = [Some(&self.built.preview), Some(&self.built.header), Some(&self.built.page),
            self.built.form.as_ref()].into_iter().flatten().cloned().collect();
        for list in &lists {
            collect(list, &mut self.drawn);
        }
    }

    fn preview_bar(&mut self) -> Vec<Node> {
        let selection = self.selection;
        let request = selection.request.map_or("none", |r| REQUESTS[r]);
        let update = selection.update.map_or("none".to_string(), |u| samples::updates()[u].0.to_string());
        let mut targets = BTreeMap::new();
        let mut button = |id: &str, label: &str, step: PreviewStep| {
            targets.insert(id.to_string(), Target::Preview(step));
            ActionView::offered(id, label)
        };
        let actions = vec![
            button("preview.previous-sample", "Previous sample", PreviewStep::PreviousSample),
            button("preview.next-sample", "Next sample", PreviewStep::NextSample),
            button("preview.previous-page", "Previous page", PreviewStep::PreviousPage),
            button("preview.next-page", "Next page", PreviewStep::NextPage),
            button("preview.next-request", "Next request sample", PreviewStep::NextRequest),
            button("preview.next-update", "Next update sample", PreviewStep::NextUpdate),
        ];
        self.built.targets.extend(targets);
        let mut last = self.log.last().cloned().unwrap_or_else(|| "Nothing pressed yet.".into());
        if last.chars().count() > 160 {
            last = format!("{}… (full line in the log)", last.chars().take(160).collect::<String>());
        }
        vec![Node::Panel { kind: presentation::PanelKind::Preview, children: vec![
            Node::Label("Preview · sample records · nothing is sent".into()),
            Node::Text(presentation::Role::Code, format!(
                "state {}/{} {} · records for {} · request {request} · update {update}",
                selection.state + 1, STATES.len(), selection.state_name(), selection.page.key())),
            Node::Actions { id: "row.preview".into(), stacked: false, actions, notes: vec![] },
            Node::Text(presentation::Role::Code, if self.posture.is_empty() { last.clone() }
                else { format!("{} · last: {last}", self.posture) }),
        ] }]
    }

    fn say(&mut self, line: String) {
        self.log.push(line);
    }

    /// Something the window says about a press it did not pass on.
    pub fn note(&mut self, line: String) {
        self.say(line);
        self.rebuild();
    }

    /// Log lines added since the last call.
    pub fn new_log(&mut self) -> Vec<String> {
        let lines = self.log[self.log_read..].to_vec();
        self.log_read = self.log.len();
        lines
    }

    /// The control a press reached. Nothing is passed on for a control that is
    /// not drawn now or is drawn held: every path refuses as the button does.
    pub fn press(&mut self, id: &str, via: InputPath) -> Outcome {
        let Some(target) = self.built.targets.get(id).cloned() else {
            self.say(format!("Not passed on: {id} via {via:?} is not on the page"));
            return Outcome::Refused;
        };
        if let Some(Some(reason)) = self.drawn.get(id) {
            let reason = reason.clone();
            self.say(format!("Not passed on: {id} via {via:?}. {reason}"));
            self.rebuild();
            return Outcome::Refused;
        }
        let before = (self.state.page, self.state.plugins.focus.clone(), self.state.plugins.page, self.state.focus.clone());
        let outcome = match target {
            Target::Offer { offer, after } => {
                let pressable = presentation::pressable(&offer.availability);
                if pressable {
                    if let After::CloseReplacement { focus_after } = after {
                        if let Some(installer) = focus_after {
                            self.state.focus.setup_installer = Some(installer);
                        }
                        self.state.focus.replacement = None;
                    }
                }
                match self.state.press_offer(&offer) {
                    Some(action) => self.capture(action, via, false),
                    None if pressable => {
                        self.form_generation += 1;
                        let form = self.open_form_name();
                        self.say(format!("Pressed {id} via {via:?}: {form} opened; nothing sent yet"));
                    }
                    None => self.say(format!("Not passed on: {id} via {via:?}. {}",
                        offer.availability.reason().unwrap_or_default())),
                }
                Outcome::Changed
            }
            Target::Control(control) => {
                match self.state.apply(control.does.clone()) {
                    None => self.say(format!("Control {id} via {via:?}: {:?}", control.does)),
                    Some(effect) => self.effect(effect, id, via),
                }
                Outcome::Changed
            }
            Target::Tab(tab) => {
                match tab {
                    Tab::Home => self.state.page = Page::Home,
                    Tab::Plugins => { self.state.plugins.focus = None; self.state.page = Page::Plugins; },
                    Tab::Activity => { self.state.apply(Local::Go(Destination::Activity)); }
                    Tab::Support => { self.state.apply(Local::Go(Destination::Diagnostics)); }
                }
                self.say(format!("Tab {} via {via:?}", tab.label()));
                Outcome::Changed
            }
            Target::Report { report, step } => {
                use presentation::support::Step;
                let Some(view) = self.ui.report.as_mut().filter(|view| view.identity == report) else { return Outcome::Refused };
                match step {
                    Step::Read => view.contents(&serde_json::to_string_pretty(&serde_json::json!({
                        "schema":2,"package_version":"Example build","setups":[],
                        "module_sha256":"ab".repeat(32), "a_long_token_fixture":format!("{}END-OF-LONG-TOKEN", "0123456789abcdef".repeat(32)),
                        "bridge_records":(0..90).map(|n|serde_json::json!({"fixture_record":n,"category":"example"})).collect::<Vec<_>>()
                    })).unwrap()),
                    Step::Back => view.inspecting = false,
                    Step::Previous => view.page = view.page.saturating_sub(1),
                    Step::Next => if view.pages.as_ref().is_some_and(|pages| view.page+1 < pages.len()) { view.page += 1; },
                    Step::Save => { view.saved = Some("/Example Downloads/bridge-report.json".into()); view.inspecting = false; },
                    Step::Done => self.ui.report = None,
                }
                self.say("Sample report interaction; no file was read or saved.".into());
                Outcome::Changed
            }
            Target::Form(step) => {
                self.form_step(step, via);
                Outcome::Changed
            }
            #[cfg(feature = "live")]
            Target::Bootstrap(_) => Outcome::Refused,
            Target::Update(step) => {
                self.update_step(step, via);
                Outcome::Changed
            }
            Target::Choose { group, key } => {
                self.ui.choices.insert(group.clone(), key.clone());
                self.say(format!("Chose {key} in {group} via {via:?}"));
                Outcome::Changed
            }
            Target::List(step) => {
                self.list_step(step, via);
                Outcome::Changed
            }
            Target::Preview(step) => {
                self.preview_step(step);
                return Outcome::Navigated;
            }
            Target::Daw(choice) => {
                self.state.forms.daw = Some(choice);
                self.say(format!("DAW for installations: {choice:?} via {via:?}"));
                Outcome::Changed
            }
        };
        let navigated = before != (self.state.page, self.state.plugins.focus.clone(), self.state.plugins.page, self.state.focus.clone());
        // The list brings the opened plug-in into view once.
        self.state.plugins.scroll_to_focus = false;
        self.state.focus.setup_scroll = false;
        self.rebuild();
        if navigated { Outcome::Navigated } else { outcome }
    }

    fn open_form_name(&self) -> &'static str {
        let forms = &self.state.forms;
        if forms.confirm.is_some() { "the confirmation" }
        else if forms.library_location.is_some() { "the library folder choice" }
        else if forms.install.is_some() { "the DAW choice" }
        else if forms.rename.is_some() { "the rename form" }
        else if forms.record.is_some() { "the result form" }
        else { "a form" }
    }

    /// An action the window would send now: captured and logged with its
    /// exact wire form, then `State::sent` closes the forms that asked for it.
    fn capture(&mut self, action: Action, via: InputPath, confirmed: bool) {
        let wire = serde_json::to_string(&action).unwrap_or_else(|_| format!("{action:?}"));
        self.say(format!("Captured, not sent (via {via:?}): {wire}"));
        self.state.sent(&action);
        self.captured.push((action, confirmed));
    }

    fn effect(&mut self, effect: Effect, id: &str, via: InputPath) {
        let said = match &effect {
            Effect::Refresh => "would read current status again; the sample records stay as they are".to_string(),
            Effect::ChooseInstaller => "would open the file chooser; the preview chooses no file".to_string(),
            Effect::RetryDetails => "would read the technical records again".to_string(),
            Effect::OpenCurrentManager => "would open the current manager".to_string(),
            Effect::Copy(words) => {
                self.ui.copied = true;
                format!("{} characters, not put on the clipboard", words.chars().count())
            }
            Effect::SaveReport => "would ask where to save the report; nothing is saved".to_string(),
            Effect::Dismiss => {
                self.sample.request = None;
                self.sample.message.clear();
                "cleared the request bar".to_string()
            }
        };
        self.say(format!("Effect {effect:?} from {id} via {via:?}: {said}"));
    }

    fn form_step(&mut self, step: FormStep, via: InputPath) {
        let forms = &mut self.state.forms;
        match step {
            FormStep::ChooseLibraryFolder => {
                if let Some(action) = forms.library_location.clone() {
                    if lvb_view_model::locations::folder_offer(Some(&self.sample.snapshot), &action).is_ok() {
                        self.capture(action, via, false);
                    }
                }
                return;
            }
            FormStep::CancelLibraryFolder => forms.library_location = None,
            FormStep::ChooseDaw(choice) => forms.daw = Some(choice),
            FormStep::StartInstall => {
                if let (Some(form), Some(daw)) = (forms.install.clone(), forms.daw) {
                    if let Some(action) = forms::installation_with_daw(&form, daw) {
                        forms.install = None;
                        self.capture(action, via, false);
                    }
                }
                return;
            }
            FormStep::CancelInstall => forms.install = None,
            FormStep::SaveRename => {
                if let Some(action) = forms.rename.clone() {
                    self.capture(action, via, false);
                }
                return;
            }
            FormStep::CancelRename => forms.rename = None,
            FormStep::ToggleForget => forms.forget_installer = !forms.forget_installer,
            FormStep::Confirm => {
                let Some(form) = forms.confirm.clone() else { return };
                let confirmation = forms::removal_confirmation(Some(&self.sample.snapshot), &form);
                if let Some(action) = self.state.forms.confirmed(confirmation.forget.is_some()) {
                    self.capture(action, via, true);
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
                    self.capture(action, via, false);
                }
                return;
            }
            FormStep::CancelRecord => forms.record = None,
        }
        self.say(format!("Form step {step:?} via {via:?}"));
    }

    /// Escape or a click outside the open form: it closes, nothing is sent.
    pub fn dismiss_form(&mut self, how: &str) {
        let forms = &mut self.state.forms;
        if forms.confirm.is_some() { forms.cancel_confirm(); }
        else if forms.library_location.is_some() { forms.library_location = None; }
        else if forms.install.is_some() { forms.install = None; }
        else if forms.rename.is_some() { forms.rename = None; }
        else if forms.record.is_some() { forms.record = None; }
        else { return; }
        self.say(format!("Form closed {how}; nothing sent"));
        self.rebuild();
    }

    fn update_step(&mut self, step: updates::Step, via: InputPath) {
        // The updater owns these; the preview keeps only its local link state.
        if let Some(update) = self.sample.update.as_mut() {
            match step {
                updates::Step::ChangeLink => update.editing = true,
                updates::Step::KeepLink => update.editing = false,
                updates::Step::SaveLink => {
                    update.linked = true;
                    update.editing = false;
                    self.ui.link_draft.clear();
                }
                _ => {}
            }
        }
        self.say(format!("Updater step {step:?} via {via:?}: the preview has no updater; the link is never stored"));
    }

    fn list_step(&mut self, step: ListStep, via: InputPath) {
        let plugins = &mut self.state.plugins;
        match &step {
            ListStep::ShowAll => plugins.focus = None,
            ListStep::ClearSearch => plugins.search.clear(),
            ListStep::ClearFilters => *plugins = Default::default(),
            ListStep::Role(role) => plugins.role = *role,
            ListStep::Status(status) => plugins.status = *status,
            ListStep::Open(key) => plugins.focus_product(key.clone()),
            ListStep::ProductPage {key,page} => plugins.show_page(key.clone(), *page),
        }
        self.say(format!("List {step:?} via {via:?}"));
    }

    fn preview_step(&mut self, step: PreviewStep) {
        let selection = &mut self.selection;
        let pages = PAGES.len();
        let page = PAGES.iter().position(|page| *page == selection.page).unwrap_or(0);
        match step {
            PreviewStep::NextSample => selection.state = (selection.state + 1) % STATES.len(),
            PreviewStep::PreviousSample => selection.state = (selection.state + STATES.len() - 1) % STATES.len(),
            PreviewStep::NextPage => selection.page = PAGES[(page + 1) % pages],
            PreviewStep::PreviousPage => selection.page = PAGES[(page + pages - 1) % pages],
            PreviewStep::NextRequest => selection.request = match selection.request {
                None => Some(0),
                Some(r) if r + 1 < REQUESTS.len() => Some(r + 1),
                Some(_) => None,
            },
            PreviewStep::NextUpdate => selection.update = match selection.update {
                None => Some(0),
                Some(u) if u + 1 < samples::updates().len() => Some(u + 1),
                Some(_) => None,
            },
        }
        let selection = self.selection;
        self.load();
        self.say(format!("Sample {} for {} (request {}, update {})", selection.state_name(), selection.page.key(),
            selection.request.map_or("none", |r| REQUESTS[r]),
            selection.update.map_or("none", |u| samples::updates()[u].0)));
        self.rebuild();
    }

    /// Text typed into a field of the page or the open form.
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
            "field.search" => self.state.plugins.search = text.into(),
            "field.beta-link" => self.ui.link_draft = text.into(),
            _ => return,
        }
        self.rebuild();
    }

    /// The first control of the page, for focus after a keyboard navigation.
    pub fn page_first(&self) -> Option<String> {
        first_control(self.built.page.get(self.built.screen_from..).unwrap_or_default())
    }
}

/// Every control in `nodes`, with its printed reason when held.
fn collect(nodes: &[Node], out: &mut BTreeMap<String, Option<String>>) {
    for node in nodes {
        match node {
            Node::LibraryRow { action, .. } => {
                out.insert(action.id.0.clone(), action.reason().map(str::to_owned));
            }
            Node::Actions { actions, .. } | Node::Tabs { tabs: actions, .. } => {
                for action in actions {
                    out.insert(action.id.0.clone(), action.reason().map(str::to_owned));
                }
            }
            Node::Panel { children, .. } | Node::Fold { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => collect(children, out),
            _ => {}
        }
    }
}

fn first_control(nodes: &[Node]) -> Option<String> {
    nodes.iter().find_map(|node| match node {
        Node::LibraryRow { action, .. } => Some(action.id.0.clone()),
        Node::Actions { actions, .. } | Node::Tabs { tabs: actions, .. } => actions.first().map(|a| a.id.0.clone()),
        Node::Panel { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => first_control(children),
        _ => None,
    })
}

#[cfg(test)]
mod tests;

fn vm_report() -> lvb_view_model::request::SupportReport {
    lvb_view_model::request::SupportReport { id: "ab".repeat(16), sha256: "cd".repeat(32) }
}

//! From the bridge's view model to what the preview draws.
//!
//! Plain Rust over `lvb-view-model` and the Product UI Foundation's display
//! types (`ActionView`, `StatusView`, fact rows); no Makepad drawing happens
//! here, so the tests check exactly what the window draws. Every pressable
//! control gets an id, and `Built::targets` says what pressing it means: a
//! manager offer (pressed only through `State::press_offer`), a window
//! control (`State::apply`), a form step, or a preview-local choice. Nothing
//! here decides availability: an offer is pressable exactly when the view
//! model says it is enabled, and every other reason is printed.

use {
    lvb_view_model::{
        self as vm,
        activity::{Activity, AttentionItem, Attentions, Incidents, SessionRow, Sessions},
        diagnostics::{Diagnostics, VersionSection},
        forms,
        home::{CheckResult, ComputerCheck, Home, HomePlugin, Next},
        journey::{PluginResult, Results},
        model::{Action, DawChoice, ProblemCategory, Snapshot, TestResultKind},
        offers::{self, Availability, Offer, OfferRow, SetupChoice},
        plugins::{self, LeadOffer, LoadingList, LoadingRow, PluginCard, PluginList, SharedSetup,
            Uninstall, VendorGroup},
        screen::{Frame, Screen, Window},
        setup::{
            AppCard, AppGroup, DeletedSetupView, DeletedSetups, ExistingSetup, ExistingSetups,
            InstallerChoice, LastInstaller, ProgramOpen, ProgramPicker, Replace, ReplaceChoices,
            ReplacementDraft, RequiredApp, RuntimeChoice, SelectedInstaller, Setup, SetupCard,
            SetupControls, UserProgram,
        },
        state::State,
        updates,
        window::{HealthBar, Page, RequestBar, RequestProgress, Superseded, TitleRow},
        Control, Detail, Disclosure, Fact, Local, Text, TextRole,
    },
    makepad_product_ui::{
        ActionView, FactRow, FactSectionView, Importance, NoticeTone, NoticeView, StatusClass,
        StatusView,
    },
    std::collections::{BTreeMap, HashMap},
};

#[cfg(feature = "live")]
pub mod bootstrap;
mod frame;
mod home;
mod setup;
#[path = "presentation/updates.rs"]
mod updates_page;
#[path = "presentation/plugins.rs"]
mod plugins_page;
mod activity;
mod support;
#[path = "presentation/forms.rs"]
mod forms_page;

/// How a line of words is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Body,
    Strong,
    Detail,
    /// Something to notice. The view model gives it no status class, so it
    /// is printed in weight, never in a status colour.
    Caution,
    Alert,
    Hint,
    CautionHint,
    Record,
    Code,
    /// An introduction under a heading.
    Lead,
}

impl From<TextRole> for Role {
    fn from(role: TextRole) -> Self {
        match role {
            TextRole::Body => Role::Body,
            TextRole::Strong => Role::Strong,
            TextRole::Detail => Role::Detail,
            TextRole::Caution => Role::Caution,
            TextRole::Alert => Role::Alert,
            TextRole::Hint => Role::Hint,
            TextRole::CautionHint => Role::CautionHint,
            TextRole::Record => Role::Record,
            TextRole::Code => Role::Code,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelKind {
    /// A card: Panel fill, Line border.
    Card,
    /// Grouped detail on the raised neutral.
    Raised,
    /// A Problem rule on the left: an owner-reported problem and its offers.
    Recovery,
    /// The preview's own bar, set apart from the product window.
    Preview,
}

/// One drawable piece of a page. Containers hold their children in reading order.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Brand(String),
    PluginIdentity { name: String, detail: String },
    Title(String),
    Heading(String),
    /// An engraved section label, drawn in capitals.
    Label(String),
    Text(Role, String),
    Chip { id: String, status: StatusView },
    Actions { id: String, stacked: bool, actions: Vec<ActionView>, notes: Vec<String> },
    /// Navigation tabs: one is selected and marked in the accent.
    Tabs { id: String, tabs: Vec<ActionView>, selected: usize },
    Facts { id: String, section: FactSectionView },
    Notice { id: String, notice: NoticeView },
    Panel { kind: PanelKind, children: Vec<Node> },
    /// A disclosure. Its children are drawn only while it is open.
    Fold { id: String, title: String, start_open: bool, children: Vec<Node> },
    /// Children side by side, wrapping.
    Row(Vec<Node>),
    /// Vertical composition without an extra card surface.
    Stack(Vec<Node>),
    /// Two proof columns, 3:2 above 760 px and stacked below it.
    Columns(Vec<Node>),
    /// Compact application identity, navigation, and right-aligned health.
    Header(Vec<Node>),
    /// The entire collection row opens one exact product.
    LibraryRow { action: ActionView, detail: String, status: Option<StatusView> },
    Readiness { status: StatusView, detail: String },
    /// A text entry the musician fills in for an open form.
    Field { id: String, value: String, hint: String, secret: bool },
    /// Owner-reported progress, 0 to 1.
    Progress(f32),
    Rule,
}

/// The four tabs of the window, as the journeys mock arranges them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Home,
    Plugins,
    Activity,
    Support,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Home, Tab::Plugins, Tab::Activity, Tab::Support];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Home => "Home",
            Tab::Plugins => "Plug-ins",
            Tab::Activity => "Activity",
            Tab::Support => "Support",
        }
    }

    /// Today's pages under the four tabs: Setup and its updates sit under
    /// Home, where the Install and Update journeys start; Diagnostics is
    /// Support.
    pub fn of(page: Page) -> Tab {
        match page {
            Page::Home | Page::Setup => Tab::Home,
            Page::Plugins => Tab::Plugins,
            Page::Activity => Tab::Activity,
            Page::Diagnostics => Tab::Support,
        }
    }
}

/// A step inside an open form.
#[derive(Clone, Debug, PartialEq)]
pub enum FormStep {
    ChooseDaw(DawChoice),
    StartInstall,
    CancelInstall,
    SaveRename,
    CancelRename,
    ToggleForget,
    Confirm,
    CancelConfirm,
    ResultWorked,
    ResultProblem,
    Category(ProblemCategory),
    FailedArea(lvb_view_model::model::TestArea),
    Passed(lvb_view_model::model::TestArea),
    SubmitRecord,
    CancelRecord,
    ExpertArea(&'static str),
    ExpertStatus(&'static str),
}

/// The Plug-ins page's own controls, which change only the window's state.
#[derive(Clone, Debug, PartialEq)]
pub enum ListStep {
    ShowAll,
    ClearSearch,
    ClearFilters,
    Role(plugins::RoleFilter),
    Status(plugins::StatusFilter),
    /// Open one exact plug-in from the list shown while controls load.
    Open(vm::ProductKey),
}

/// The preview's own sample switcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewStep {
    NextSample,
    PreviousSample,
    NextPage,
    PreviousPage,
    NextRequest,
    NextUpdate,
}

/// What pressing a control means.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    /// A manager offer, pressed only through `State::press_offer`.
    /// `after` is what the window does locally once it is pressable.
    Offer { offer: Offer, after: After },
    /// A window control, applied through `State::apply`.
    Control(Control),
    Tab(Tab),
    Form(FormStep),
    /// An updater step; the preview has no updater, so it is logged.
    Update(updates::Step),
    #[cfg(feature = "live")]
    Bootstrap(lvb_client::bootstrap::Step),
    /// One option of a choice on the page (local selection only).
    Choose { group: String, key: String },
    List(ListStep),
    Preview(PreviewStep),
    /// Choose which DAW installations use (Setup's DAW picker).
    Daw(DawChoice),
}

/// A window-local follow-up to pressing an offer.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum After {
    #[default]
    Nothing,
    /// The replacement draft closes, and Setup shows this installer's setup.
    CloseReplacement { focus_after: Option<String> },
}

/// What the window knows when a control is held without a reason of its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct Why {
    pub fresh: bool,
    pub pending: bool,
    pub in_flight: bool,
    pub superseded: bool,
}

impl Why {
    /// A reason for a window control the view model shows disabled without
    /// one. These are the view model's own sentences for the same states;
    /// `Control` carries no reason of its own (a D4 parity gap).
    pub fn control(&self) -> String {
        if self.superseded {
            vm::window::superseded().title.into()
        } else if self.pending || self.in_flight {
            offers::WAITING.into()
        } else if !self.fresh {
            offers::HELD.into()
        } else {
            offers::WAITING.into()
        }
    }
}

/// The musician's local choices on the page: which option of each choice
/// is selected, and whether version details were just copied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LocalUi {
    pub choices: HashMap<String, String>,
    pub copied: bool,
    /// What is typed in the beta link field (preview-local, never saved).
    pub link_draft: String,
}

/// Everything the window draws at one moment, and what each control means.
#[derive(Clone, Debug, Default)]
pub struct Built {
    pub preview: Vec<Node>,
    pub header: Vec<Node>,
    pub page: Vec<Node>,
    /// An open form, drawn over the page.
    pub form: Option<Vec<Node>>,
    pub targets: BTreeMap<String, Target>,
    /// The control a form wants focused when it opens: the safe answer.
    pub form_focus: Option<String>,
    /// Where the open page starts in `page`, after the frame's own parts.
    pub screen_from: usize,
}

pub struct Builder<'a> {
    pub targets: BTreeMap<String, Target>,
    used: HashMap<String, usize>,
    ui: &'a LocalUi,
    why: Why,
    /// Draw every option's content, not only the chosen one (tests).
    reveal: bool,
    form_generation: u64,
}

/// A short, stable id fragment from some words.
pub fn slug(words: &str) -> String {
    let mut out = String::new();
    for c in words.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    let trimmed = out.trim_end_matches('-');
    if trimmed.is_empty() { "x".into() } else { trimmed.into() }
}

fn importance(importance: offers::Importance) -> Importance {
    match importance {
        offers::Importance::Primary => Importance::Primary,
        offers::Importance::Secondary => Importance::Secondary,
        offers::Importance::Quiet => Importance::Quiet,
    }
}

pub fn class(class: vm::StatusClass) -> StatusClass {
    match class {
        vm::StatusClass::Ready => StatusClass::Ready,
        vm::StatusClass::Working => StatusClass::Working,
        vm::StatusClass::NeedsYou => StatusClass::NeedsYou,
        vm::StatusClass::Problem => StatusClass::Problem,
        vm::StatusClass::Unknown => StatusClass::Unknown,
    }
}

fn status(status: &vm::Status) -> StatusView {
    StatusView::new(class(status.class), status.words)
}

fn text(text: &Text) -> Node {
    Node::Text(text.role.into(), text.words.clone())
}

fn words(role: Role, words: impl Into<String>) -> Node {
    Node::Text(role, words.into())
}

fn fact_rows(facts: &[Fact]) -> Vec<FactRow> {
    facts
        .iter()
        .map(|fact| match &fact.value {
            Some(value) => FactRow::text(fact.label, value.clone()),
            None => FactRow::missing(fact.label, fact.missing),
        })
        .collect()
}

impl<'a> Builder<'a> {
    pub fn new(ui: &'a LocalUi, why: Why, reveal: bool) -> Self {
        Self { targets: BTreeMap::new(), used: HashMap::new(), ui, why, reveal, form_generation: 0 }
    }

    /// A unique id: `kind.words`, numbered when the same words repeat.
    pub fn id(&mut self, kind: &str, words: &str) -> String {
        let base = format!("{kind}.{}", slug(words));
        let n = self.used.entry(base.clone()).or_insert(0);
        *n += 1;
        if *n == 1 { base } else { format!("{base}-{n}") }
    }

    fn target(&mut self, kind: &str, label: &str, target: Target) -> String {
        // These are local, opaque widget keys, never product identities or
        // persisted data. Keep the complete typed target in the key so a
        // reordered list cannot give a held click or keyboard focus to a
        // different offer. Debug formatting is local to this pinned build.
        let scope = if matches!(target, Target::Form(_)) {
            format!("form-{}::", self.form_generation)
        } else { String::new() };
        let base = format!("{kind}.{}::{scope}{target:?}", slug(label));
        let n = self.used.entry(base.clone()).or_insert(0);
        *n += 1;
        let id = if *n == 1 { base } else { format!("{base}::{n}") };
        self.targets.insert(id.clone(), target);
        id
    }

    /// One manager offer as a control. A removal or deletion is drawn as
    /// destructive: those are exactly the offers `forms::route` confirms.
    pub fn offer(&mut self, offer: &Offer, after: After) -> ActionView {
        let id = self.target("offer", &offer.label, Target::Offer { offer: offer.clone(), after });
        let weight = if forms::needs_confirmation(&offer.intent) {
            Importance::Destructive
        } else {
            importance(offer.importance)
        };
        let view = match offer.availability.reason() {
            None => ActionView::offered(id, offer.label.clone()),
            Some(reason) => ActionView::disabled(id, offer.label.clone(), reason),
        }
        .importance(weight);
        match offer.consequence {
            Some(consequence) => view.consequence(consequence),
            None => view,
        }
    }

    /// A window control. A disabled one prints why.
    pub fn control(&mut self, control: &Control) -> ActionView {
        let id = self.target("control", control.label, Target::Control(control.clone()));
        let view = if control.enabled {
            ActionView::offered(id, control.label)
        } else {
            ActionView::disabled(id, control.label, self.why.control())
        };
        view.importance(importance(control.importance))
    }

    fn local(&mut self, label: &str, weight: Importance, target: Target, reason: Option<String>)
        -> ActionView {
        self.local_keyed(label, label, weight, target, reason)
    }

    /// A local control whose words change with its state (a toggle, a
    /// choice), keyed by `key` so its id and focus survive the change.
    fn local_keyed(&mut self, key: &str, label: &str, weight: Importance, target: Target,
        reason: Option<String>) -> ActionView {
        let id = self.target("local", key, target);
        match reason {
            None => ActionView::offered(id, label),
            Some(reason) => ActionView::disabled(id, label, reason),
        }
        .importance(weight)
    }

    fn actions(&mut self, kind: &str, actions: Vec<ActionView>) -> Node {
        let id = self.id("row", kind);
        Node::Actions { id, stacked: false, actions, notes: vec![] }
    }

    fn one(&mut self, action: ActionView) -> Node {
        let id = format!("row.{}", action.id.0);
        Node::Actions { id, stacked: false, actions: vec![action], notes: vec![] }
    }

    /// A row of offers with its preface, and every refusal printed. Reasons
    /// the buttons already print are not printed twice; the row's other
    /// notes, such as a refusal an enabled offer still carries, are.
    pub fn row(&mut self, row: &OfferRow) -> Vec<Node> {
        self.row_after(row, After::Nothing)
    }

    fn row_after(&mut self, row: &OfferRow, after: After) -> Vec<Node> {
        let OfferRow { preface, offers, notes, consequences } = row;
        let mut out = Vec::new();
        if let Some(preface) = preface {
            out.push(words(Role::Body, preface.clone()));
        }
        if offers.is_empty() {
            return out;
        }
        let actions: Vec<ActionView> = offers.iter().map(|offer| self.offer(offer, after.clone())).collect();
        let printed: Vec<&str> = actions
            .iter()
            .flat_map(|action| action.reason().into_iter().chain(action.consequence.as_deref()))
            .collect();
        let extra: Vec<String> = notes
            .iter()
            .map(String::as_str)
            .chain(consequences.iter().copied())
            .filter(|note| !printed.contains(note))
            .map(str::to_owned)
            .collect();
        let id = self.id("row", offers.first().map_or("offers", |offer| offer.label.as_str()));
        out.push(Node::Actions { id, stacked: false, actions, notes: extra });
        out
    }

    /// The one main offer of a step, on its own row.
    fn main_offer(&mut self, offer: &Offer, after: After) -> Node {
        let view = self.offer(offer, after);
        self.one(view)
    }

    fn fold(&mut self, disclosure: &Disclosure) -> Node {
        let Disclosure { title, id_salt, start_open, force_open, content } = disclosure;
        let children = self.details(content);
        if *force_open {
            // Held open: a labelled section, nothing to fold.
            let mut inline = vec![Node::Label(title.clone())];
            inline.extend(children);
            return Node::Panel { kind: PanelKind::Raised, children: inline };
        }
        let id = self.id("fold", id_salt.unwrap_or(title));
        Node::Fold { id, title: title.clone(), start_open: *start_open, children }
    }

    fn fold_with(&mut self, disclosure: &Disclosure, more: Vec<Node>) -> Node {
        let mut node = self.fold(disclosure);
        match &mut node {
            Node::Fold { children, .. } | Node::Panel { children, .. } => children.extend(more),
            _ => {}
        }
        node
    }

    pub fn details(&mut self, content: &[Detail]) -> Vec<Node> {
        let mut out = Vec::new();
        for detail in content {
            match detail {
                Detail::Text(line) => out.push(text(line)),
                Detail::Offers(row) => out.extend(self.row(row)),
                // Held offers carry their own `Held` reason.
                Detail::Held { actionable: _, row } => out.extend(self.row(row)),
                Detail::Choice(choice) => out.extend(self.setup_choice(choice)),
                Detail::Disclosure(inner) => out.push(self.fold(inner)),
                Detail::Group(inner) => {
                    let children = self.details(inner);
                    out.push(Node::Panel { kind: PanelKind::Raised, children });
                }
                Detail::Divider => out.push(Node::Rule),
            }
        }
        out
    }

    fn setup_choice(&mut self, choice: &SetupChoice) -> Vec<Node> {
        let SetupChoice { offer, notes, description, exact_action: _, identity } = choice;
        let view = self.offer(offer, After::Nothing);
        let printed: Vec<String> = view.reason().into_iter().map(str::to_owned).collect();
        let id = self.id("row", &offer.label);
        let mut extra: Vec<String> = notes.iter().filter(|note| !printed.contains(note)).cloned().collect();
        extra.extend(description.clone());
        vec![Node::Actions { id, stacked: true, actions: vec![view], notes: extra }, self.fold(identity)]
    }

    /// A choice the musician makes on the page: its options as buttons, the
    /// chosen one named, and the chosen option's content beneath.
    fn choice<T>(&mut self, group: &str, placeholder: &str, choices: &[vm::Choice<T>],
        preselected: Option<&str>, mut content: impl FnMut(&mut Self, &T) -> Vec<Node>) -> Vec<Node> {
        let mut out = vec![Node::Label(placeholder.into())];
        let selected = self.ui.choices.get(group).map(String::as_str).or(preselected)
            .filter(|key| choices.iter().any(|choice| choice.key == *key))
            .map(str::to_owned);
        let options: Vec<ActionView> = choices
            .iter()
            .map(|choice| self.local(&choice.label, Importance::Secondary,
                Target::Choose { group: group.into(), key: choice.key.clone() }, None))
            .collect();
        if !options.is_empty() {
            out.push(self.actions(group, options));
        }
        for choice in choices {
            let chosen = selected.as_deref() == Some(choice.key.as_str());
            if chosen {
                out.push(words(Role::Strong, format!("Chosen: {}", choice.label)));
            }
            if chosen || self.reveal {
                let inner = content(self, &choice.content);
                if !inner.is_empty() {
                    out.push(Node::Panel { kind: PanelKind::Raised, children: inner });
                }
            }
        }
        out
    }

    fn results(&mut self, results: &Results) -> Vec<Node> {
        let Results { daw, empty, problem, plugins } = results;
        let mut out = vec![words(Role::Detail, daw.clone())];
        if let Some(empty) = empty {
            out.push(words(Role::Body, *empty));
        }
        if let Some(problem) = problem {
            out.push(words(Role::Caution, problem.clone()));
        }
        for plugin in plugins {
            let PluginResult { name, status: result, problem, hint } = plugin;
            let chip = self.id("chip", &format!("result {name}"));
            out.push(Node::Row(vec![
                words(Role::Strong, name.clone()),
                Node::Chip { id: chip, status: status(result) },
            ]));
            if let Some(problem) = problem {
                out.push(words(Role::Caution, problem.clone()));
            }
            if let Some(hint) = hint {
                out.push(words(Role::Detail, *hint));
            }
        }
        out
    }

    // ----- the pages -----

    pub fn screen(&mut self, screen: &Screen, frame: &Frame, snapshot: Option<&Snapshot>, state: &State,
        update: Option<&updates::Panel>) -> Vec<Node> {
        match screen {
            Screen::Home(home) => self.home(home),
            Screen::Setup(setup) => self.setup(setup, state, update),
            Screen::Plugins { not_current, list, single } => snapshot.map(|snapshot| self.plugins(not_current.as_ref(), list, *single, snapshot, state))
                .unwrap_or_else(|| vec![words(Role::Body, "Reading plug-ins…")]),
            Screen::PluginsLoading(list) => self.loading(list, state),
            Screen::Activity(activity) => self.activity(activity),
            Screen::Diagnostics(diagnostics) => self.diagnostics(diagnostics, frame),
            Screen::Checking { heading, words: said, retry } => {
                let mut out = vec![Node::Title((*heading).into()), words(Role::Body, *said)];
                if let Some(retry) = retry {
                    let retry = self.control(retry);
                    out.push(self.one(retry));
                }
                out
            },
            Screen::LoadingDetails { words: said, retry } => {
                let mut out = vec![Node::Title(Tab::of(state.page).label().into()), words(Role::Body, if retry.is_some() { "This page could not be loaded. Try again." } else { *said })];
                if let Some(retry) = retry {
                    let view = self.control(retry);
                    out.push(self.one(view));
                }
                out
            }
            Screen::LoadingCurrent(said) => vec![words(Role::Body, *said)],
        }
    }


}

/// Every manager offer drawn, with whether it can be pressed now.
pub fn drawn_offers(targets: &BTreeMap<String, Target>, nodes: &[&[Node]]) -> Vec<(ActionView, Offer)> {
    fn walk(nodes: &[Node], out: &mut Vec<ActionView>) {
        for node in nodes {
            match node {
                Node::Actions { actions, .. } => out.extend(actions.iter().cloned()),
                Node::Tabs { tabs, .. } => out.extend(tabs.iter().cloned()),
                Node::Panel { children, .. } | Node::Fold { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => walk(children, out),
                _ => {}
            }
        }
    }
    let mut views = Vec::new();
    for list in nodes {
        walk(list, &mut views);
    }
    views
        .into_iter()
        .filter_map(|view| match targets.get(&view.id.0) {
            Some(Target::Offer { offer, .. }) => Some((view, offer.clone())),
            _ => None,
        })
        .collect()
}

/// Builds what the window draws for `window` over its records.
#[allow(clippy::too_many_arguments)]
pub fn build(window: &Window, snapshot: Option<&Snapshot>, actions: Option<&Snapshot>, state: &State, ui: &LocalUi,
    why: Why, update: Option<&updates::Panel>, reveal: bool, form_generation: u64) -> Built {
    let mut builder = Builder::new(ui, why, reveal);
    builder.form_generation = form_generation;
    let (header, mut page) = builder.frame(&window.frame, state.page);
    let screen_from = page.len();
    page.extend(builder.screen(&window.screen, &window.frame, snapshot, state, update));
    let form = snapshot.and_then(|snapshot| builder.form(state, snapshot, actions, why.pending));
    let (form, form_focus) = match form {
        Some((nodes, focus)) => (Some(nodes), focus),
        None => (None, None),
    };
    Built { preview: vec![], header, page, form, targets: builder.targets, form_focus, screen_from }
}

/// Whether an offer's availability lets it be pressed: the view model's own rule.
pub fn pressable(availability: &Availability) -> bool {
    availability.enabled()
}

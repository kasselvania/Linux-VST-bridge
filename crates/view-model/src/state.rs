//! The window's own state beside the manager's records: the open page, what
//! the Plug-ins list and Setup are narrowed to, and the form a pressed offer
//! opened with the musician's unsent entries. A frontend keeps one `State`.
//! A pressed control changes it through `apply`, and a pressed offer through
//! `press`, so where navigation leads and which offer asks first are decided
//! here once for every frontend. None of it is sent to the manager.
use crate::model::{Action, DawChoice};
use crate::plugins::{Query, RoleFilter, StatusFilter};
use crate::setup::{self, InstallerReplacement};
use crate::window::Page;
use crate::{forms, AttemptKey, Destination, Local, ProductKey};

/// The Plug-ins page: the search, the filters and the plug-ins opened from
/// elsewhere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PluginsView {
    pub search: String,
    pub role: RoleFilter,
    pub status: StatusFilter,
    /// Exact plug-ins opened from elsewhere; search and filters do not apply.
    pub focus: Option<Vec<ProductKey>>,
    /// Hold every disclosure open (previews and tests).
    pub expanded: bool,
    /// Bring the opened plug-ins into view when the page is next drawn.
    pub scroll_to_focus: bool,
}

impl PluginsView {
    pub fn query(&self) -> Query<'_> {
        Query { search: &self.search, role: self.role, status: self.status, focus: self.focus.as_deref() }
    }

    /// The one plug-in open, if exactly one is.
    pub fn focused_product(&self) -> Option<&ProductKey> {
        self.focus.as_deref().and_then(|keys| (keys.len() == 1).then_some(&keys[0]))
    }

    pub fn focus_product(&mut self, key: ProductKey) {
        self.focus_products(vec![key]);
    }

    /// Open exactly these plug-ins, clearing the search and filters.
    pub fn focus_products(&mut self, keys: Vec<ProductKey>) {
        self.search.clear();
        self.role = RoleFilter::All;
        self.status = StatusFilter::All;
        self.focus = Some(keys);
        self.scroll_to_focus = true;
    }
}

/// What Setup and Activity are narrowed to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Focus {
    /// The exact installer attempt opened from elsewhere.
    pub setup: Option<AttemptKey>,
    /// Only this installer's setup is shown.
    pub setup_installer: Option<String>,
    /// Bring that setup into view when Setup is next drawn.
    pub setup_scroll: bool,
    /// The plug-in session opened on Activity.
    pub activity: Option<String>,
    /// A different installer being chosen for one setup.
    pub replacement: Option<InstallerReplacement>,
}

impl Focus {
    pub fn setup_focus(&self) -> setup::Focus<'_> {
        setup::Focus { attempt: self.setup.as_ref(), installer: self.setup_installer.as_deref(),
            replacement: self.replacement.as_ref() }
    }
}

/// The forms a pressed offer opened (see `forms::route`), each holding the
/// manager's exact action with the musician's entries so far.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Forms {
    /// An installation waiting for its DAW.
    pub install: Option<Action>,
    /// The DAW chosen for installations, kept for the next one.
    pub daw: Option<DawChoice>,
    /// A test result or observation being recorded.
    pub record: Option<Action>,
    /// An exact release being entered (Workspaces).
    pub release: Option<Action>,
    /// An installer being named.
    pub rename: Option<Action>,
    /// A removal or deletion awaiting the musician's confirmation.
    pub confirm: Option<Action>,
}

/// Everything the window holds besides the manager's records.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub page: Page,
    pub plugins: PluginsView,
    pub focus: Focus,
    pub forms: Forms,
}

/// What a pressed control asks of the frontend beyond changing the state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Read current status again.
    Refresh,
    /// Let the musician choose an installer file.
    ChooseInstaller,
}

impl State {
    /// Open a page, or an exact item on it.
    pub fn go(&mut self, destination: &Destination) {
        go(destination, &mut self.page, &mut self.plugins, &mut self.focus);
    }

    /// A window-handled control, applied as soon as it is pressed. The frame
    /// handles retrying details, opening the current manager, copying, saving
    /// a report and dismissing where it draws them; those change nothing here.
    pub fn apply(&mut self, local: Local) -> Option<Effect> {
        apply(local, &mut self.page, &mut self.plugins, &mut self.focus)
    }

    /// A pressed manager offer: the action to send now, or None when it
    /// opened its form first. Every frontend presses through here, so no
    /// confirmation or DAW choice can be skipped.
    pub fn press(&mut self, action: Action) -> Option<Action> {
        match forms::route(action, self.forms.daw) {
            forms::Route::Send(action) => return Some(action),
            forms::Route::ChooseDaw(action) => self.forms.install = Some(action),
            forms::Route::Record(action) => self.forms.record = Some(action),
            forms::Route::ChooseRelease(action) => self.forms.release = Some(action),
            forms::Route::Rename(action) => self.forms.rename = Some(action),
            forms::Route::Confirm(action) => self.forms.confirm = Some(action),
        }
        None
    }
}

/// `State::go` over the state's parts, for a frontend that lends them out
/// separately while it draws.
pub fn go(destination: &Destination, page: &mut Page, plugins: &mut PluginsView, focus: &mut Focus) {
    match destination {
        Destination::Product(key) => {
            plugins.focus_product(key.clone());
            *page = Page::Plugins;
        }
        Destination::Products(keys) => {
            plugins.focus_products(keys.clone());
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

/// `State::apply` over the state's parts.
pub fn apply(local: Local, page: &mut Page, plugins: &mut PluginsView, focus: &mut Focus) -> Option<Effect> {
    match local {
        Local::Go(destination) => go(&destination, page, plugins, focus),
        Local::CheckComputer | Local::Refresh => return Some(Effect::Refresh),
        Local::ChooseInstaller => {
            focus.replacement = None;
            return Some(Effect::ChooseInstaller);
        }
        Local::ChooseReplacement(context) => {
            focus.setup_installer = Some(context.original.clone());
            focus.replacement = Some(context);
            return Some(Effect::ChooseInstaller);
        }
        Local::KeepCurrentInstaller => focus.replacement = None,
        Local::ShowAllSetup => {
            focus.setup_installer = None;
            focus.setup = None;
            focus.setup_scroll = false;
        }
        Local::RetryDetails | Local::OpenCurrentManager | Local::Copy(_) | Local::SaveReport | Local::Dismiss => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(class: &str) -> ProductKey {
        ProductKey { environment: "e".into(), class_id: class.into(), module_sha256: "m".into() }
    }

    #[test]
    fn opening_a_plug_in_clears_the_search_and_filters_and_brings_it_into_view() {
        let mut state = State::default();
        state.plugins.search = "keys".into();
        state.plugins.role = RoleFilter::Effects;
        state.apply(Local::Go(Destination::Product(key("a"))));
        assert_eq!(state.page, Page::Plugins);
        assert_eq!(state.plugins.focused_product(), Some(&key("a")));
        assert!(state.plugins.search.is_empty() && state.plugins.role == RoleFilter::All);
        assert!(state.plugins.scroll_to_focus);
        state.go(&Destination::Products(vec![key("a"), key("b")]));
        assert_eq!(state.plugins.focused_product(), None);
    }

    #[test]
    fn setup_focus_narrows_and_show_all_releases_it() {
        let mut state = State::default();
        let attempt = AttemptKey { installer: "i".into(), environment: Some("e".into()) };
        assert_eq!(state.apply(Local::Go(Destination::Attempt(attempt.clone()))), None);
        assert_eq!(state.page, Page::Setup);
        assert_eq!(state.focus.setup_focus().installer, Some("i"));
        assert!(state.focus.setup_scroll);
        assert_eq!(state.apply(Local::ShowAllSetup), None);
        assert_eq!(state.focus, Focus::default());
    }

    #[test]
    fn controls_that_need_the_frontend_say_so() {
        let mut state = State::default();
        assert_eq!(state.apply(Local::Refresh), Some(Effect::Refresh));
        assert_eq!(state.apply(Local::CheckComputer), Some(Effect::Refresh));
        let replacement = InstallerReplacement { original: "old".into(), environment: None, runtime: None,
            selected: None };
        assert_eq!(state.apply(Local::ChooseReplacement(replacement.clone())), Some(Effect::ChooseInstaller));
        assert_eq!(state.focus.replacement.as_ref(), Some(&replacement));
        assert_eq!(state.focus.setup_installer.as_deref(), Some("old"));
        assert_eq!(state.apply(Local::ChooseInstaller), Some(Effect::ChooseInstaller));
        assert_eq!(state.focus.replacement, None);
        for local in [Local::RetryDetails, Local::OpenCurrentManager, Local::Copy("x".into()), Local::SaveReport,
            Local::Dismiss] {
            let before = state.clone();
            assert_eq!(state.apply(local), None);
            assert_eq!(state, before);
        }
    }

    #[test]
    fn a_pressed_offer_that_asks_first_opens_its_form_and_sends_nothing() {
        let mut state = State::default();
        let install = Action::SoftwareInstall { installer: "ab".repeat(32), daw: DawChoice::Other };
        assert_eq!(state.press(install), None);
        assert!(state.forms.install.is_some());
        state.forms.daw = Some(DawChoice::Reaper);
        let sent = state.press(Action::SoftwareInstall { installer: "ab".repeat(32), daw: DawChoice::Other });
        assert!(matches!(sent, Some(Action::SoftwareInstall { daw: DawChoice::Reaper, .. })));
        assert_eq!(state.press(Action::RuntimeInstall {}), Some(Action::RuntimeInstall {}));
    }
}

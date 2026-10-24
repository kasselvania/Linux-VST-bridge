//! The window's own state beside the manager's records: the open page, what
//! the Plug-ins list and Setup are narrowed to, and the form a pressed offer
//! opened with the musician's unsent entries. A frontend keeps one `State`
//! and changes it only through these methods:
//!
//! - a pressed offer goes through `press_offer`, which sends nothing for an
//!   offer that is held, waiting or refused, and opens the offer's form first
//!   where it needs one;
//! - a pressed control goes through `apply`, which changes the state or says
//!   what the frontend must do (`Effect`);
//! - new records go through `observe`, a sent request through `sent`, and an
//!   added installer through `imported`.
//!
//! So where navigation leads, which offer asks first and which form closes
//! are decided here once for every frontend. None of it is sent to the
//! manager.
use crate::model::{Action, DawChoice, Snapshot};
use crate::offers::Offer;
use crate::plugins::{Query, RoleFilter, StatusFilter};
use crate::setup::{self, InstallerReplacement};
use crate::window::Page;
use crate::{forms, AttemptKey, Destination, Local, ProductKey};

/// Local views of the same exact plug-in; never a service action or new identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProductPage { #[default] Overview, Display, Repair }

/// The Plug-ins page: the search, the filters and the plug-ins opened from
/// elsewhere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PluginsView {
    pub page: ProductPage,
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

    pub fn show_page(&mut self, key: ProductKey, page: ProductPage) {
        self.focus_product(key);
        self.page = page;
    }

    /// Open exactly these plug-ins, clearing the search and filters.
    pub fn focus_products(&mut self, keys: Vec<ProductKey>) {
        self.page = ProductPage::Overview;
        self.search.clear();
        self.role = RoleFilter::All;
        self.status = StatusFilter::All;
        self.focus = Some(keys);
        self.scroll_to_focus = true;
    }
}

/// What Setup and Activity are narrowed to and the installer being chosen
/// for a setup. (`setup::Focus` is the view of it Setup is built from.)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageFocus {
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

impl PageFocus {
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
    /// An installer being named.
    pub rename: Option<Action>,
    /// A removal or deletion awaiting the musician's confirmation.
    pub confirm: Option<Action>,
    /// The confirmation's optional "Also forget the installer file", off
    /// until the musician ticks it (`forms::Confirmation::forget`).
    pub forget_installer: bool,
}

impl Forms {
    /// What the confirmation sends: the confirmed action, with the choice to
    /// forget the installer file where the confirmation offered it.
    pub fn confirmed(&self, offers_forget: bool) -> Option<Action> {
        Some(match forms::confirmed(self.confirm.clone()?) {
            Action::SetupDelete { installer, environment, confirmed, .. } if offers_forget =>
                Action::SetupDelete { installer, environment, confirmed, forget_installer: self.forget_installer },
            other => other,
        })
    }

    /// The musician closed the confirmation; its unsent choice goes with it.
    pub fn cancel_confirm(&mut self) {
        self.confirm = None;
        self.forget_installer = false;
    }
}

/// Everything the window holds besides the manager's records.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    pub page: Page,
    pub plugins: PluginsView,
    pub focus: PageFocus,
    pub forms: Forms,
}

/// What a pressed control asks of the frontend beyond changing the state.
/// Every control that changes nothing here returns one, so a frontend's
/// dispatch cannot drop it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Read current status again.
    Refresh,
    /// Let the musician choose an installer file.
    ChooseInstaller,
    /// Read the technical records again after they failed to load.
    RetryDetails,
    /// Open the manager that replaced this window.
    OpenCurrentManager,
    /// Put these words on the clipboard.
    Copy(String),
    /// Choose where to save the problem report the manager created.
    SaveReport,
    /// Clear the request bar.
    Dismiss,
}

/// Where an added installer's setup was shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Imported {
    /// The page now shows it; the window says this.
    Shown(String),
    /// The records do not show it yet; read again and ask again.
    NotYet(&'static str),
}

impl State {
    /// Open a page, or an exact item on it.
    pub fn go(&mut self, destination: &Destination) {
        match destination {
            Destination::Product(key) => {
                self.plugins.focus_product(key.clone());
                self.page = Page::Plugins;
            }
            Destination::Products(keys) => {
                self.plugins.focus_products(keys.clone());
                self.page = Page::Plugins;
            }
            Destination::Attempt(key) => {
                self.focus.setup = Some(key.clone());
                self.focus.setup_installer = Some(key.installer.clone());
                self.focus.setup_scroll = true;
                self.page = Page::Setup;
            }
            Destination::Session(session) => {
                self.focus.activity = Some(session.clone());
                self.page = Page::Activity;
            }
            Destination::Activity => self.page = Page::Activity,
            Destination::Setup => self.page = Page::Setup,
            Destination::Diagnostics => self.page = Page::Diagnostics,
        }
    }

    /// A window-handled control, applied as soon as it is pressed: it changes
    /// this state, or returns what the frontend must do.
    pub fn apply(&mut self, local: Local) -> Option<Effect> {
        Some(match local {
            Local::Go(destination) => {
                self.go(&destination);
                return None;
            }
            Local::CheckComputer | Local::Refresh => Effect::Refresh,
            Local::ChooseInstaller => {
                self.focus.replacement = None;
                Effect::ChooseInstaller
            }
            Local::ChooseReplacement(context) => {
                self.focus.setup_installer = Some(context.original.clone());
                self.focus.replacement = Some(context);
                Effect::ChooseInstaller
            }
            Local::KeepCurrentInstaller => {
                self.focus.replacement = None;
                return None;
            }
            Local::ShowAllSetup => {
                self.focus.setup_installer = None;
                self.focus.setup = None;
                self.focus.setup_scroll = false;
                return None;
            }
            Local::RetryDetails => Effect::RetryDetails,
            Local::OpenCurrentManager => Effect::OpenCurrentManager,
            Local::Copy(words) => Effect::Copy(words),
            Local::SaveReport => Effect::SaveReport,
            Local::Dismiss => Effect::Dismiss,
        })
    }

    /// A pressed offer: the action to send now, or None when the offer can't
    /// be pressed (held, waiting or refused) or opened its form first.
    /// Frontends press offers only through here, so no frontend can press a
    /// held or refused offer or skip a confirmation or DAW choice.
    ///
    /// An immediate-mode frontend that takes the press while it draws and
    /// routes it at the end of the frame does the same in two steps:
    /// `Offer::pressed` at the click, then `press`.
    pub fn press_offer(&mut self, offer: &Offer) -> Option<Action> {
        self.press(offer.pressed()?)
    }

    /// The routing half of `press_offer`, for an action `Offer::pressed`
    /// gave: send it now, or open its form first.
    pub fn press(&mut self, action: Action) -> Option<Action> {
        match forms::route(action, self.forms.daw) {
            forms::Route::Send(action) => return Some(action),
            forms::Route::ChooseDaw(action) => self.forms.install = Some(action),
            forms::Route::Record(action) => self.forms.record = Some(action),
            forms::Route::Rename(action) => self.forms.rename = Some(action),
            forms::Route::Confirm(action) => {
                // Another confirmation starts with its choice off.
                if self.forms.confirm.as_ref() != Some(&action) { self.forms.forget_installer = false; }
                self.forms.confirm = Some(action);
            }
        }
        None
    }

    /// The window sent this request: every form that asked for it closes,
    /// including the confirmation it answers.
    pub fn sent(&mut self, action: &Action) {
        if matches!(action, Action::CandidateSettingsTest { .. }) {
            self.plugins.page = ProductPage::Overview;
        }
        let forms = &mut self.forms;
        for form in [&mut forms.record, &mut forms.rename, &mut forms.install] {
            if form.as_ref() == Some(action) { *form = None; }
        }
        // The confirmed request differs from its offer only by the musician's answers.
        if forms.confirm.as_ref().is_some_and(|form| form == action
            || forms::offer_identity(form) == forms::offer_identity(action)) {
            forms.cancel_confirm();
        }
    }

    /// New current records. Where no DAW is chosen yet and every installation
    /// on record used the same one, it is remembered for the next.
    pub fn observe(&mut self, snapshot: &Snapshot) {
        if self.forms.daw.is_some() { return; }
        let mut choices = snapshot.installer_setups.iter()
            .filter_map(|setup| setup.journey.as_ref().map(|journey| journey.daw));
        if let Some(first) = choices.next() {
            if choices.all(|choice| choice == first) { self.forms.daw = Some(first); }
        }
    }

    /// An installer was added (`newly_imported`) or found already added, under
    /// `label`. Show the one setup it belongs to.
    pub fn imported(&mut self, snapshot: &Snapshot, installer: &str, newly_imported: bool, label: &str) -> Imported {
        match snapshot.installer_setups.iter().filter(|setup| setup.installer == installer).count() {
            1 => {
                self.page = Page::Setup;
                self.focus.setup = None;
                self.focus.setup_installer = Some(installer.to_owned());
                self.focus.setup_scroll = true;
                Imported::Shown(if newly_imported {
                    format!("Added {label}. No native plug-in setup has started.")
                } else {
                    "This installer was already added. Showing its existing plug-in setup.".into()
                })
            }
            _ => Imported::NotYet("The installer was added, but its setup could not be loaded. Refresh before continuing."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offers::{Availability, Importance};

    fn key(class: &str) -> ProductKey {
        ProductKey { environment: "e".into(), class_id: class.into(), module_sha256: "m".into() }
    }

    fn offer(intent: Action, availability: Availability) -> Offer {
        Offer { label: "Go".into(), importance: Importance::Primary, availability, consequence: None, intent }
    }

    fn delete(confirmed: bool, forget_installer: bool) -> Action {
        Action::SetupDelete { installer: "ab".repeat(32), environment: "cd".repeat(16), confirmed, forget_installer }
    }

    #[test]
    fn opening_a_plug_in_clears_the_search_and_filters_and_brings_it_into_view() {
        let mut state = State::default();
        state.plugins.search = "keys".into();
        state.plugins.role = RoleFilter::Effects;
        assert_eq!(state.apply(Local::Go(Destination::Product(key("a")))), None);
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
        assert_eq!(state.apply(Local::Go(Destination::Attempt(attempt))), None);
        assert_eq!(state.page, Page::Setup);
        assert_eq!(state.focus.setup_focus().installer, Some("i"));
        assert!(state.focus.setup_scroll);
        assert_eq!(state.apply(Local::ShowAllSetup), None);
        assert_eq!(state.focus, PageFocus::default());
    }

    #[test]
    fn every_control_changes_the_state_or_says_what_the_frontend_must_do() {
        let mut state = State::default();
        assert_eq!(state.apply(Local::Refresh), Some(Effect::Refresh));
        assert_eq!(state.apply(Local::CheckComputer), Some(Effect::Refresh));
        let replacement = InstallerReplacement { original: "old".into(), environment: None, runtime: None,
            selected: None };
        assert_eq!(state.apply(Local::ChooseReplacement(replacement.clone())), Some(Effect::ChooseInstaller));
        assert_eq!(state.focus.replacement.as_ref(), Some(&replacement));
        assert_eq!(state.focus.setup_installer.as_deref(), Some("old"));
        assert_eq!(state.apply(Local::KeepCurrentInstaller), None);
        assert_eq!(state.focus.replacement, None);
        assert_eq!(state.apply(Local::ChooseInstaller), Some(Effect::ChooseInstaller));
        for (local, effect) in [(Local::RetryDetails, Effect::RetryDetails),
            (Local::OpenCurrentManager, Effect::OpenCurrentManager), (Local::Copy("x".into()), Effect::Copy("x".into())),
            (Local::SaveReport, Effect::SaveReport), (Local::Dismiss, Effect::Dismiss)] {
            let before = state.clone();
            assert_eq!(state.apply(local), Some(effect));
            assert_eq!(state, before);
        }
    }

    #[test]
    fn only_an_enabled_offer_can_be_pressed() {
        let mut state = State::default();
        for availability in [Availability::Held, Availability::Waiting, Availability::Refused("Busy".into())] {
            assert_eq!(state.press_offer(&offer(Action::RuntimeInstall {}, availability.clone())), None);
            assert_eq!(state.press_offer(&offer(delete(false, false), availability)), None);
            assert_eq!(state, State::default());
        }
        assert_eq!(state.press_offer(&offer(Action::RuntimeInstall {}, Availability::Enabled)),
            Some(Action::RuntimeInstall {}));
    }

    #[test]
    fn a_pressed_offer_that_asks_first_opens_its_form_and_sends_nothing() {
        let mut state = State::default();
        let install = Action::SoftwareInstall { installer: "ab".repeat(32), daw: DawChoice::Other };
        assert_eq!(state.press_offer(&offer(install.clone(), Availability::Enabled)), None);
        assert!(state.forms.install.is_some());
        state.forms.daw = Some(DawChoice::Reaper);
        let sent = state.press_offer(&offer(install, Availability::Enabled));
        assert!(matches!(sent, Some(Action::SoftwareInstall { daw: DawChoice::Reaper, .. })));
    }

    #[test]
    fn the_confirmation_sends_its_choice_and_closes_when_sent() {
        let mut state = State::default();
        assert_eq!(state.press(delete(false, false)), None);
        state.forms.forget_installer = true;
        // Pressing the same offer again keeps the open confirmation's choice.
        state.press(delete(false, false));
        assert!(state.forms.forget_installer);
        assert_eq!(state.forms.confirmed(true), Some(delete(true, true)));
        // Where the confirmation offers no choice, none is sent.
        assert_eq!(state.forms.confirmed(false), Some(delete(true, false)));
        state.sent(&delete(true, true));
        assert_eq!(state.forms, Forms::default());
        state.press(delete(false, false));
        state.forms.forget_installer = true;
        state.forms.cancel_confirm();
        assert_eq!(state.forms, Forms::default());
        assert_eq!(state.forms.confirmed(true), None);
    }

    #[test]
    fn a_sent_request_closes_only_the_form_that_asked_for_it() {
        let mut state = State::default();
        let rename = Action::InstallerRename { installer: "ab".repeat(32), label: "Keys".into() };
        state.press(rename.clone());
        state.press(delete(false, false));
        state.sent(&rename);
        assert_eq!(state.forms.rename, None);
        assert!(state.forms.confirm.is_some());
    }

    fn snapshot_with(setups: &[(&str, DawChoice)]) -> Snapshot {
        let journeyed = crate::samples::snapshot(Page::Setup, Some("install_progress"));
        let template = journeyed.installer_setups.iter().find(|setup| setup.journey.is_some())
            .expect("a sample setup with an installation journey").clone();
        let mut snapshot = crate::samples::base();
        snapshot.installer_setups = setups.iter().map(|(installer, daw)| {
            let mut setup = template.clone();
            setup.installer = (*installer).into();
            if let Some(journey) = &mut setup.journey { journey.daw = *daw; }
            setup
        }).collect();
        snapshot
    }

    #[test]
    fn the_one_daw_every_setup_used_is_remembered_and_a_choice_is_never_replaced() {
        let mut state = State::default();
        state.observe(&snapshot_with(&[("a", DawChoice::Bitwig), ("b", DawChoice::Reaper)]));
        assert_eq!(state.forms.daw, None);
        state.observe(&snapshot_with(&[("a", DawChoice::Bitwig), ("b", DawChoice::Bitwig)]));
        assert_eq!(state.forms.daw, Some(DawChoice::Bitwig));
        state.observe(&snapshot_with(&[("a", DawChoice::Reaper)]));
        assert_eq!(state.forms.daw, Some(DawChoice::Bitwig));
    }

    #[test]
    fn an_added_installer_opens_the_one_setup_it_belongs_to() {
        let mut state = State::default();
        let shown = state.imported(&snapshot_with(&[("a", DawChoice::Bitwig)]), "a", true, "Keys");
        assert_eq!(shown, Imported::Shown("Added Keys. No native plug-in setup has started.".into()));
        assert_eq!((state.page, state.focus.setup_installer.as_deref(), state.focus.setup_scroll),
            (Page::Setup, Some("a"), true));
        let mut state = State::default();
        assert!(matches!(state.imported(&snapshot_with(&[("a", DawChoice::Bitwig)]), "a", false, "Keys"),
            Imported::Shown(words) if words.starts_with("This installer was already added")));
        // Not shown yet, or not one place: nothing moves until the records name one.
        for snapshot in [snapshot_with(&[]),
            snapshot_with(&[("a", DawChoice::Bitwig), ("a", DawChoice::Reaper)])] {
            let mut state = State::default();
            assert!(matches!(state.imported(&snapshot, "a", true, "Keys"), Imported::NotYet(_)));
            assert_eq!(state, State::default());
        }
    }
}

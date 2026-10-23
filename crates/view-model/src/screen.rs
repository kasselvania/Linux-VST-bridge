//! The whole window at one moment: its frame (title row, service health,
//! update line and current request) and its open page. It is built from the
//! manager's records, how current they are, and the window's `State`.
//!
//! A frontend that rebuilds after every event calls `Window::build`. One
//! that applies a click while it draws (immediate mode) builds the `Frame` at
//! the top and the `Screen` where the page starts, so a page opened from the
//! frame shows in the same frame.
use crate::activity::Activity;
use crate::diagnostics::Diagnostics;
use crate::home::Home;
use crate::model::{ReadinessAssessment, Snapshot, System};
use crate::plugins::{self, LoadingList, PluginList};
use crate::setup::Setup;
use crate::state::State;
use crate::window::{self, HealthBar, Page, Request, RequestBar, Superseded, TitleRow};
use crate::{Control, Text};

/// The manager's records as the window last read them.
#[derive(Clone, Copy, Debug, Default)]
pub struct Records<'a> {
    /// Current status: the overview Home, Setup and the list are built from.
    pub current: Option<&'a Snapshot>,
    /// The full records, read for the technical pages.
    pub details: Option<&'a Snapshot>,
    /// The opened plug-in's own records over current status.
    pub product: Option<&'a Snapshot>,
    /// Records whose offers are current.
    pub actions: Option<&'a Snapshot>,
    /// The computer-setup check Home shows, while it matches the records.
    pub readiness: Option<&'a ReadinessAssessment>,
    /// The service status the health row shows.
    pub health: Option<&'a System>,
}

/// How current the records are and what the window is doing.
#[derive(Clone, Debug, Default)]
pub struct Reading<'a> {
    /// The current status is current.
    pub fresh: bool,
    /// It aged and is being read again.
    pub renewing: bool,
    /// The health row's status is being read again.
    pub health_renewing: bool,
    /// The manager paused the bridge for its own installation.
    pub owned_pause: bool,
    /// A request, its readback or the window holds the controls.
    pub controls_pending: bool,
    /// A request or read is in flight.
    pub in_flight: bool,
    /// A foreground request or file choice is in flight.
    pub working: bool,
    /// The musician's own Refresh is running.
    pub explicit_refresh: bool,
    /// The full records were asked for and did not arrive.
    pub details_attempted: bool,
    /// The opened plug-in's records were asked for and did not arrive.
    pub product_attempted: bool,
    /// An updated window replaced this one.
    pub superseded: bool,
    /// Sample records; nothing is sent.
    pub preview: bool,
    /// This window's own build, shown before the manager answers.
    pub own_version: Option<&'a str>,
    /// The distribution's name, for Diagnostics.
    pub distribution: Option<&'a str>,
    /// The updater's one line (`updates::headline`).
    pub update_headline: Option<String>,
    /// The window's last request.
    pub request: Option<Request<'a>>,
    /// The window's last notice.
    pub message: &'a str,
}

impl Reading<'_> {
    /// Offers wait: the controls are held, or (outside previews) no records
    /// with current offers are at hand.
    fn actions_pending(&self, records: &Records) -> bool {
        self.controls_pending || (!self.preview && records.actions.is_none())
    }
}

/// Everything around the page.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub title: TitleRow,
    pub superseded: Option<Superseded>,
    pub preview: Option<&'static str>,
    pub health: HealthBar,
    /// The update line and its "View update", on every page but Setup.
    pub headline: Option<(String, Control)>,
    pub request: Option<RequestBar>,
}

impl Frame {
    pub fn build(records: &Records, reading: &Reading, state: &State) -> Self {
        Self {
            title: TitleRow::build(records.current.or(records.details), reading.own_version, reading.fresh,
                reading.controls_pending, records.actions, reading.explicit_refresh),
            superseded: reading.superseded.then(window::superseded),
            preview: reading.preview.then_some(window::PREVIEW),
            health: HealthBar::build(records.health, reading.owned_pause, reading.health_renewing),
            headline: window::update_headline(reading.update_headline.clone().filter(|_| state.page != Page::Setup)),
            request: RequestBar::build(reading.request, reading.message, reading.working,
                !reading.in_flight && !reading.superseded),
        }
    }
}

/// The open page.
#[derive(Clone, Debug, PartialEq)]
pub enum Screen {
    Home(Box<Home>),
    Setup(Box<Setup>),
    /// The list or the opened plug-in, from records with their offers.
    /// `single`: exactly one plug-in was open when the page was built.
    Plugins { not_current: Option<Text>, list: Box<PluginList>, single: bool },
    /// The list from current status while the opened plug-in's records load.
    PluginsLoading(Box<LoadingList>),
    Activity(Box<Activity>),
    Diagnostics(Box<Diagnostics>),
    /// Home or Setup before the manager's first answer.
    Checking { heading: &'static str, words: &'static str },
    /// A technical page before its records arrive.
    LoadingDetails { words: &'static str, retry: Option<Control> },
    /// Home or Setup with full records but no current status yet.
    LoadingCurrent(&'static str),
}

impl Screen {
    pub fn build(records: &Records, reading: &Reading, state: &State) -> Self {
        let actions_pending = reading.actions_pending(records);
        let fresh = records.actions.is_some();
        if records.current.is_some() || !reading.preview {
            if let Some(current) = records.current {
                match state.page {
                    Page::Home => return Self::Home(Box::new(Home::build(current, records.readiness, fresh,
                        reading.health_renewing, reading.controls_pending))),
                    Page::Setup => return Self::Setup(Box::new(Setup::build(current, fresh,
                        reading.controls_pending, state.focus.setup_focus()))),
                    Page::Plugins => return match records.product {
                        Some(product) => Self::plugins(product, actions_pending,
                            (!reading.fresh).then(|| window::plugins_not_current(reading.renewing)), state),
                        None => Self::PluginsLoading(Box::new(LoadingList::build(current, &state.plugins.query(),
                            fresh, reading.product_attempted))),
                    },
                    _ => {}
                }
            } else if matches!(state.page, Page::Home | Page::Setup) {
                return Self::Checking { heading: state.page.label(), words: window::CHECKING };
            }
        }
        let Some(details) = records.details else {
            return Self::LoadingDetails { words: window::LOADING_DETAILS,
                retry: (reading.details_attempted && !reading.in_flight).then(window::retry_details) };
        };
        match state.page {
            Page::Home | Page::Setup => Self::LoadingCurrent(window::LOADING_CURRENT),
            Page::Plugins => Self::plugins(details, actions_pending, None, state),
            Page::Activity => Self::Activity(Box::new(Activity::build(details, actions_pending))),
            Page::Diagnostics => Self::Diagnostics(Box::new(Diagnostics::build(details, actions_pending,
                reading.distribution))),
        }
    }

    fn plugins(snapshot: &Snapshot, pending: bool, not_current: Option<Text>, state: &State) -> Self {
        let single = state.plugins.focused_product().is_some();
        Self::Plugins { not_current, single, list: Box::new(PluginList::build(snapshot, &state.plugins.query(),
            single, plugins::Context { busy: snapshot.system.inactive_reason(), pending,
                expanded: state.plugins.expanded })) }
    }
}

/// The whole window.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub frame: Frame,
    pub screen: Screen,
}

impl Window {
    pub fn build(records: &Records, reading: &Reading, state: &State) -> Self {
        Self { frame: Frame::build(records, reading, state), screen: Screen::build(records, reading, state) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples::{self, Case, Sample};

    fn window(page: Page, state: &str) -> Window {
        Sample::new(Case::new(page, state)).window(Some("Example OS"))
    }

    #[test]
    fn every_sample_builds_its_page_from_the_records_it_has() {
        for case in samples::cases() {
            let (page, state) = (case.page, case.state.clone());
            let sample = Sample::new(case);
            let opened = sample.case.plugin_open && !sample.snapshot.products.is_empty();
            let screen = sample.window(None).screen;
            let expected = match page {
                Page::Home => matches!(screen, Screen::Home(_)),
                Page::Setup => matches!(screen, Screen::Setup(_)),
                // Without an opened plug-in's own records, the list comes from current status.
                Page::Plugins if !opened => matches!(screen, Screen::PluginsLoading(_)),
                Page::Plugins => matches!(screen, Screen::Plugins { single: true, .. }),
                Page::Activity => matches!(screen, Screen::Activity(_)),
                Page::Diagnostics => matches!(screen, Screen::Diagnostics(_)),
            };
            assert!(expected, "{}/{state}: {screen:?}", page.key());
        }
    }

    #[test]
    fn retired_schema29_workspace_records_create_no_screen_or_controls() {
        assert_eq!(Page::from_key("workspaces"), None);
        let workspace: crate::model::DawWorkspace = serde_json::from_value(serde_json::json!({
            "id":"fl-studio", "name":"FL Studio", "state":"running",
            "selected_installer":"old-installer", "selected_release":"26.1.6.0",
            "application_installers":["old-installer"], "cleanup":"running",
            "actions":[{"label":"Stop FL Studio", "action":{"kind":"workspace_stop"},
                "disabled_reason":null}], "installer_choices":[], "details":{}
        })).unwrap();
        for page in Page::ALL {
            let mut sample = Sample::new(Case::new(page, "populated"));
            let before = sample.window(None);
            sample.snapshot.workspaces.push(workspace.clone());
            assert_eq!(sample.window(None), before, "{}", page.key());
        }
    }

    #[test]
    fn stale_records_hold_offers_and_say_the_plug_in_is_not_current() {
        let Screen::Plugins { not_current, .. } = window(Page::Plugins, "stale").screen else { panic!() };
        assert_eq!(not_current, Some(window::plugins_not_current(false)));
        let Screen::Plugins { not_current, .. } = window(Page::Plugins, "plugin_ready").screen else { panic!() };
        assert_eq!(not_current, None);
        let stale = Sample::new(Case::new(Page::Home, "stale"));
        assert!(stale.records().actions.is_none() && stale.records().health.is_none());
    }

    #[test]
    fn the_frame_says_what_the_request_samples_say() {
        let over = |name| Sample::new(Case::new(Page::Home, "populated")).with_request(name).window(None).frame;
        assert!(over("superseded").superseded.is_some());
        assert!(!over("superseded").title.refresh.enabled);
        assert!(over("importing").request.is_some_and(|bar| bar.working));
        assert!(over("report").request.is_some_and(|bar| bar.report.is_some()));
        assert_eq!(over("message").request.map(|bar| bar.words), Some(format!("Operation accepted: {}", "0a".repeat(16))));
        assert!(over("renewing").health.note.contains("refreshing"));
        let pending = window(Page::Home, "pending");
        assert!(pending.frame.request.is_some_and(|bar| bar.dismiss.is_none()));
        assert!(!pending.frame.title.refresh.enabled);
        assert_eq!(window(Page::Home, "populated").frame.preview, Some(window::PREVIEW));
    }

    #[test]
    fn the_update_line_shows_on_every_page_but_setup() {
        for (page, shown) in [(Page::Home, true), (Page::Setup, false), (Page::Plugins, true)] {
            let sample = Sample::new(Case::new(page, "available")).with_update("downloading");
            assert_eq!(sample.window(None).frame.headline.is_some(), shown, "{}", page.key());
        }
        assert!(Sample::new(Case::new(Page::Home, "available")).with_update("idle").window(None).frame.headline.is_none());
    }

    #[test]
    fn missing_records_say_what_is_loading() {
        let state = State { page: Page::Home, ..Default::default() };
        let live = Reading::default();
        assert!(matches!(Screen::build(&Records::default(), &live, &state), Screen::Checking { heading: "Home", .. }));
        let state = State { page: Page::Activity, ..Default::default() };
        let attempted = Reading { details_attempted: true, ..Default::default() };
        assert!(matches!(Screen::build(&Records::default(), &attempted, &state),
            Screen::LoadingDetails { retry: Some(_), .. }));
        let reading = Reading { details_attempted: true, in_flight: true, ..Default::default() };
        assert!(matches!(Screen::build(&Records::default(), &reading, &state), Screen::LoadingDetails { retry: None, .. }));
        let sample = Sample::new(Case::new(Page::Setup, "populated"));
        let records = Records { current: None, ..sample.records() };
        // Only a preview reaches the full records without current status.
        let preview = Reading { preview: true, ..Default::default() };
        assert!(matches!(Screen::build(&records, &preview, &sample.state()), Screen::LoadingCurrent(_)));
        assert!(matches!(Screen::build(&records, &live, &sample.state()), Screen::Checking { heading: "Setup", .. }));
    }
}

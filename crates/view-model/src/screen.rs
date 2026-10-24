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
    /// Independent Support reads, when this frontend loads sections separately.
    pub support: Option<crate::diagnostics::SupportRead<'a>>,
    /// Ordinary Activity is a separate bounded read, not the deep snapshot.
    pub activity: Option<crate::diagnostics::Section<'a, &'a crate::model::Activity>>,
}

/// How current the records are and what the window is doing.
#[derive(Clone, Debug, Default)]
pub struct Reading<'a> {
    /// The current status is current.
    pub fresh: bool,
    /// It aged and is being read again.
    pub renewing: bool,
    /// The current-status read failed; retain the failure until status arrives.
    pub current_unavailable: bool,
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
    /// Why the last full read is unavailable, separately from any request result.
    pub details_notice: Option<&'a str>,
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
        let mut request = RequestBar::build(reading.request, reading.message, reading.working,
            !reading.in_flight && !reading.superseded);
        if reading.current_unavailable && reading.request.is_none() {
            if let Some(request) = &mut request { request.dismiss = None; }
        }
        let mut health = HealthBar::build(records.health, reading.owned_pause, reading.health_renewing);
        if reading.superseded && records.health.is_some() {
            health.last_observed = true;
            health.note = "Last observed status · window paused";
        }
        Self {
            title: TitleRow::build(records.current.or(records.details), reading.own_version, reading.fresh,
                reading.controls_pending, records.actions, reading.explicit_refresh),
            superseded: reading.superseded.then(window::superseded),
            preview: reading.preview.then_some(window::PREVIEW),
            health,
            headline: window::update_headline(reading.update_headline.clone().filter(|_| state.page != Page::Setup)),
            request,
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
    Checking { heading: &'static str, words: &'static str, retry: Option<Control> },
    /// A technical page before its records arrive.
    LoadingDetails { words: String, retry: Option<Control> },
    /// Home or Setup with full records but no current status yet.
    LoadingCurrent(&'static str),
}

impl Screen {
    pub fn build(records: &Records, reading: &Reading, state: &State) -> Self {
        let actions_pending = reading.actions_pending(records);
        let fresh = records.actions.is_some();
        let activity = if reading.superseded { crate::diagnostics::Section::Failed("Window paused") }
            else { records.activity.unwrap_or(crate::diagnostics::Section::NotRequested) };
        if records.current.is_none() && reading.current_unavailable && !reading.preview {
            return Self::Checking { heading: state.page.label(),
                words: "Current status could not be loaded. Try again.",
                retry: Some(Control { label: "Try again", importance: crate::offers::Importance::Secondary,
                    enabled: !reading.controls_pending && !reading.in_flight && !reading.renewing
                        && !reading.superseded, does: crate::Local::Refresh }) };
        }
        if records.current.is_some() || !reading.preview {
            if let Some(current) = records.current {
                match state.page {
                    Page::Home => return Self::Home(Box::new(Home::build_observed(current, records.readiness, fresh,
                        reading.health_renewing, reading.controls_pending, activity))),
                    Page::Setup => return Self::Setup(Box::new(Setup::build(current, fresh,
                        reading.controls_pending, state.focus.setup_focus()))),
                    Page::Plugins => return match records.product {
                        Some(product) => Self::plugins(product, actions_pending,
                            (!reading.fresh).then(|| window::plugins_not_current(reading.renewing)), state, activity),
                        None => Self::PluginsLoading(Box::new(LoadingList::build(current, &state.plugins.query(),
                            if fresh { plugins::CurrentRead::Ready }
                            else if reading.renewing || reading.health_renewing { plugins::CurrentRead::Refreshing }
                            else { plugins::CurrentRead::Unavailable }, reading.product_attempted))),
                    },
                    Page::Diagnostics if records.support.is_some() => return Self::Diagnostics(Box::new(
                        Diagnostics::from_current(current, records.support.unwrap(), actions_pending,
                            reading.in_flight, reading.superseded, reading.distribution))),
                    Page::Activity if records.activity.is_some() || !reading.preview => return Self::Activity(Box::new(
                        Activity::from_current(current, activity, reading.in_flight, reading.superseded, records.actions, actions_pending))),
                    _ => {}
                }
            } else if matches!(state.page, Page::Home | Page::Setup) {
                return Self::Checking { heading: state.page.label(), words: window::CHECKING, retry: None };
            }
        }
        let Some(details) = records.details else {
            return Self::LoadingDetails { words: if reading.details_attempted {
                    reading.details_notice.unwrap_or("This page could not be loaded. Try again.")
                } else { window::LOADING_DETAILS }.into(),
                retry: (reading.details_attempted && !reading.in_flight).then(window::retry_details) };
        };
        match state.page {
            Page::Home | Page::Setup => Self::LoadingCurrent(window::LOADING_CURRENT),
            Page::Plugins => Self::plugins(details, actions_pending, None, state, activity),
            Page::Activity => Self::Activity(Box::new(Activity::build(details, actions_pending))),
            Page::Diagnostics => Self::Diagnostics(Box::new(Diagnostics::build(details, actions_pending,
                reading.distribution))),
        }
    }

    fn plugins(snapshot: &Snapshot, pending: bool, not_current: Option<Text>, state: &State,
        activity: crate::diagnostics::Section<'_, &crate::model::Activity>) -> Self {
        let single = state.plugins.focused_product().is_some();
        Self::Plugins { not_current, single, list: Box::new(PluginList::build(snapshot, &state.plugins.query(),
            single, plugins::Context { busy: snapshot.system.inactive_reason(), pending,
                expanded: state.plugins.expanded, activity })) }
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
    fn retired_workspace_wire_records_are_not_schema30_views() {
        assert_eq!(Page::from_key("workspaces"), None);
        let mut record = serde_json::to_value(crate::samples::base()).unwrap();
        record["workspaces"] = serde_json::json!([]);
        assert!(serde_json::from_value::<Snapshot>(record).is_err());
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
    fn failed_initial_status_is_retryable_on_every_page_without_inventing_records() {
        for page in [Page::Home, Page::Setup, Page::Plugins, Page::Activity, Page::Diagnostics] {
            let state = State { page, ..Default::default() };
            for pending in [false, true] {
                let reading = Reading { current_unavailable: true, renewing: pending, ..Default::default() };
                let Screen::Checking { words, retry: Some(retry), .. } =
                    Screen::build(&Records::default(), &reading, &state) else { panic!("missing retry") };
                assert_eq!(words, "Current status could not be loaded. Try again.");
                assert_eq!(retry.enabled, !pending);
                assert_eq!(retry.does, crate::Local::Refresh);
            }
        }
        let reading = Reading { current_unavailable: true, message: "Read unavailable", ..Default::default() };
        assert!(Frame::build(&Records::default(), &reading, &State::default()).request.unwrap().dismiss.is_none());
        let request = Request { text: "Report saved", journey: None, operation: None,
            transitions: &[], report: false, blocking: false };
        let reading = Reading { request: Some(request), ..reading };
        assert!(Frame::build(&Records::default(), &reading, &State::default()).request.unwrap().dismiss.is_some());
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
    #[test]
    fn ordinary_activity_uses_explicit_sections_without_a_technical_snapshot() {
        use crate::diagnostics::{Section, SupportRead};
        let sample = Sample::new(Case::new(Page::Diagnostics, "populated"));
        let records = Records { details: None, support: Some(SupportRead {
            status: Section::Loading, technical: Section::NotRequested,
        }), ..sample.records() };
        let reading = Reading { fresh: true, ..Reading::default() };
        let state = State { page: Page::Diagnostics, ..State::default() };
        assert!(matches!(Screen::build(&records, &reading, &state), Screen::Diagnostics(_)));
        let state = State { page: Page::Activity, ..State::default() };
        let Screen::Activity(activity) = Screen::build(&records, &reading, &state) else { panic!() };
        assert!(activity.notice.is_some() && activity.live.empty.is_none(),
            "An unrequested observation cannot fall back to raw session history");
        let records = Records { activity: Some(Section::Loading), ..records };
        let Screen::Activity(activity) = Screen::build(&records, &reading, &state) else { panic!("Activity must not request Snapshot") };
        assert!(activity.notice.is_some() && activity.live.empty.is_none());
        let records = Records { activity: Some(Section::Failed("Disconnected")), ..records };
        let Screen::Activity(activity) = Screen::build(&records, &reading, &state) else { panic!() };
        assert!(activity.retry.is_some() && activity.live.empty.is_none());
    }

    fn live_activity(snapshot: &Snapshot) -> crate::model::Activity {
        use crate::model::{self, Observation as O};
        model::Activity { schema: model::OPERATOR_SCHEMA, system: snapshot.system.clone(),
            capture: serde_json::Value::Null, operation: None, incidents: O::Observed(vec![]),
            recent: O::Observed(vec![]), live: model::LiveActivity { instances: O::Observed(vec![model::LiveInstance { recovery:None,
                kind: model::InstanceKind::Daw,
                session: "exact-session".into(), product: O::Observed(model::ProductIdentity::from(&snapshot.products[0])),
                consumer: O::Unavailable("Fixture has no process".into()),
                daw: O::Observed(model::HostApplication { name: "Example DAW".into(), source: model::HostIdentitySource::Vst3HostApplication }),
                transport: O::Unavailable("Fixture has no mapping".into()), lifecycle: O::Unavailable("No phase read".into()),
                processing: model::ProcessingObservation::Advancing { epoch: 1, completed_end: 512, observed_age_ms: 2 },
                signal: model::SignalObservation::NotMeasured, terminal: O::Observed(None), cleanup: model::CleanupState::NotReported,
            }]) } }
    }

    #[test]
    fn home_and_detail_use_exact_pushed_instances_without_changing_offers() {
        use crate::diagnostics::Section;
        let mut snapshot = samples::base();
        // Neither a digest-only nor an environment-only join may attach the
        // first product's instance to a same-class sibling.
        let mut other_build = snapshot.products[0].clone();
        other_build.module_sha256 = "another-build".into();
        let mut other_setup = snapshot.products[0].clone();
        other_setup.environment = "another-setup".into();
        snapshot.products = vec![snapshot.products[0].clone(), other_build, other_setup];
        let activity = live_activity(&snapshot);
        let records = Records { current: Some(&snapshot), product: Some(&snapshot), actions: Some(&snapshot),
            activity: Some(Section::Loaded(&activity)), ..Default::default() };
        let reading = Reading { fresh: true, ..Default::default() };
        let mut state = State { page: Page::Home, ..Default::default() };
        let Screen::Home(home) = Screen::build(&records, &reading, &state) else { panic!() };
        assert!(home.plugins[0].activity.contains("1 observed instance(s)"));
        assert!(home.plugins[0].activity.contains("Host reports Example DAW (VST3 host name)"));
        assert!(home.plugins[0].activity.contains("audible output is not measured"));
        for sibling in &home.plugins[1..] {
            assert_eq!(sibling.activity, "0 observed instance(s)");
        }
        let Screen::Home(before) = Screen::build(&Records { activity: None, ..records }, &reading, &state) else { panic!() };
        assert_eq!(before.next, home.next, "observation never supplies the next action");
        for (before, after) in before.plugins.iter().zip(&home.plugins) {
            assert_eq!(before.manage, after.manage); assert_eq!(before.status, after.status);
        }
        state.go(&crate::Destination::Product(crate::ProductKey::from(&snapshot.products[0])));
        let Screen::Plugins { list, .. } = Screen::build(&records, &reading, &state) else { panic!() };
        let card = &list.groups[0].cards[0];
        assert!(card.notes.iter().any(|text| text.words.contains("Host reports Example DAW")
            && text.words.contains("audible output is not measured")));
        let Screen::Plugins { list: before, .. } = Screen::build(&Records { activity: None, ..records }, &reading, &state) else { panic!() };
        let before = &before.groups[0].cards[0];
        assert_eq!(before.lead, card.lead); assert_eq!(before.offers, card.offers);
        assert_eq!(before.readiness, card.readiness);
    }

    #[test]
    fn live_absence_and_supersession_do_not_claim_zero_or_use_raw_history() {
        use crate::{diagnostics::Section, model::Observation as O};
        let mut snapshot = samples::base();
        snapshot.active_sessions = vec![serde_json::json!({"class_id":snapshot.products[0].class_id,
            "state":"active","recent":false})];
        let mut activity = live_activity(&snapshot);
        let reading = Reading { fresh: true, ..Default::default() };
        let state = State { page: Page::Home, ..Default::default() };
        for (read, expected) in [(Section::NotRequested, "not been observed"),
            (Section::Loading, "Reading live activity"), (Section::Failed("Disconnected"), "unavailable")] {
            let records = Records { current: Some(&snapshot), details: Some(&snapshot), activity: Some(read), ..Default::default() };
            let Screen::Home(home) = Screen::build(&records, &reading, &state) else { panic!() };
            assert!(home.plugins[0].activity.contains(expected));
            assert!(!home.plugins[0].activity.contains("0 observed"));
        }
        activity.live.instances = O::Unavailable("Registry busy".into());
        let view = crate::product::product_activity(Section::Loaded(&activity), &snapshot.products[0]);
        assert!(view.detail_label().contains("unavailable"));
        activity.live.instances = O::Observed(vec![]);
        let view = crate::product::product_activity(Section::Loaded(&activity), &snapshot.products[0]);
        assert_eq!(view.detail_label(), "0 observed instance(s)");
        let activity = live_activity(&snapshot);
        let records = Records { current: Some(&snapshot), details: Some(&snapshot), product: Some(&snapshot),
            activity: Some(Section::Loaded(&activity)), ..Default::default() };
        let paused = Reading { superseded: true, controls_pending: true, ..Default::default() };
        let Screen::Home(home) = Screen::build(&records, &paused, &state) else { panic!() };
        assert_eq!(home.plugins[0].activity, "Current live activity unavailable");
        let mut state = state;
        state.go(&crate::Destination::Product(crate::ProductKey::from(&snapshot.products[0])));
        let Screen::Plugins { list, .. } = Screen::build(&records, &paused, &state) else { panic!() };
        assert!(list.groups[0].cards[0].notes.iter().any(|text| text.words == "Current live activity unavailable"));
        assert!(!list.groups[0].cards[0].notes.iter().any(|text| text.words.contains("Example DAW")));
        state.page = Page::Activity;
        let Screen::Activity(view) = Screen::build(&records, &paused, &state) else { panic!() };
        assert!(view.live.rows.is_empty() && view.live.empty.is_none());
        assert!(!view.retry.unwrap().enabled);
    }

    #[test]
    fn processing_observation_never_turns_an_unchanged_counter_into_a_stop() {
        use crate::{diagnostics::Section, model::{Observation as O, ProcessingObservation as P}};
        let snapshot = samples::base();
        let mut activity = live_activity(&snapshot);
        for (processing, words) in [
            (P::Advancing { epoch:1, completed_end:512, observed_age_ms:2 }, "Processing is advancing; audible output is not measured"),
            (P::LastReportedStopped { epoch:1, observed_age_ms:2 }, "Processing last reported stopped"),
            (P::NotObservedAdvancing { epoch:1, completed_end:512, observed_age_ms:2 }, "Processing progress not observed; a stop is not established"),
            (P::Unavailable { reason:"Mapping changed".into() }, "Processing observation unavailable"),
        ] {
            let O::Observed(instances) = &mut activity.live.instances else { panic!() };
            instances[0].processing = processing;
            let view = crate::product::product_activity(Section::Loaded(&activity), &snapshot.products[0]);
            assert_eq!(view.lines, vec![format!("Host reports Example DAW (VST3 host name) · {words}")]);
        }
        let O::Observed(instances) = &mut activity.live.instances else { panic!() };
        instances[0].product = O::Unavailable("Retained owner missing".into());
        let view = crate::product::product_activity(Section::Loaded(&activity), &snapshot.products[0]);
        assert_eq!(view.detail_label(), "Some admitted instance identities are unavailable");
        assert!(view.lines.is_empty());
    }

}

//! The frame around every page: the title row with the installed version,
//! the bridge service's health, the pages, the current request, and what a
//! page says while its records load. Service health stays separate from
//! plug-in availability.
use super::journey::{self, Results};
use super::offers::{self, Importance, OfferRow};
use super::product::activity_certainty;
use super::service::{self, Health};
use super::{version, Control, Destination, Disclosure, Fact, Local, Status, StatusClass, Text};
use crate::model::{Action, InstallationJourney, InstallationPluginStage, Snapshot, System};

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
    pub const ALL: [Self; 6] = [Self::Home, Self::Plugins, Self::Workspaces, Self::Activity, Self::Setup,
        Self::Diagnostics];

    pub fn label(self) -> &'static str {
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

/// The title row: the product, its installed build and runtime, and the
/// window's own Refresh and problem report.
#[derive(Clone, Debug)]
pub struct TitleRow {
    pub title: &'static str,
    /// "0.12.0test144 · runtime r5"; before the manager answers, this window's build.
    pub version: Option<String>,
    /// A build Setup holds that can be applied.
    pub note: Option<String>,
    pub refresh: Control,
    pub export: Option<OfferRow>,
    /// Said while an explicit refresh runs.
    pub refreshing: Option<&'static str>,
}

impl TitleRow {
    /// `snapshot`: the manager's last records, if any; `own_version`: this
    /// window's build; `actions`: records whose offers are current.
    pub fn build(snapshot: Option<&Snapshot>, own_version: Option<&str>, fresh: bool, pending: bool,
        actions: Option<&Snapshot>, refreshing: bool) -> Self {
        Self {
            title: "Linux VST Bridge",
            version: match snapshot {
                None => own_version.map(str::to_owned),
                Some(snapshot) => {
                    let installed = snapshot.bridge.installed.as_deref().unwrap_or("version unknown");
                    Some(match version::runtime_summary(snapshot) {
                        Some(runtime) => format!("{installed} · runtime {runtime}"),
                        None => installed.to_owned(),
                    })
                }
            },
            note: snapshot.and_then(|snapshot| version::setup_title_note(&snapshot.bridge)),
            refresh: Control { label: if fresh { "Refresh" } else { "Check again" }, importance: Importance::Secondary,
                enabled: !pending, does: Local::Refresh },
            export: actions.and_then(|snapshot| snapshot.actions.iter()
                .find(|offer| matches!(offer.action, Action::SupportExport {})))
                .map(|offer| offers::row(std::slice::from_ref(offer), Importance::Secondary, None, pending)),
            refreshing: refreshing.then_some("Refreshing…"),
        }
    }
}

/// This window was replaced by an updated one and no longer acts.
#[derive(Clone, Debug)]
pub struct Superseded {
    pub title: &'static str,
    pub explanation: &'static str,
    pub open: Control,
}

pub fn superseded() -> Superseded {
    Superseded {
        title: "Bridge updated — this window is paused",
        explanation: "Unfinished inputs remain in this window. Continue in the current manager to see accepted operations and use the updated bridge.",
        open: Control { label: "Open current manager", importance: Importance::Secondary, enabled: true,
            does: Local::OpenCurrentManager },
    }
}

pub const PREVIEW: &str = "LOCAL DESIGN PREVIEW · sample records · actions do not execute";

/// The bridge service's health, with its counters on request.
#[derive(Clone, Debug)]
pub struct HealthBar {
    pub status: Status,
    pub show: &'static str,
    pub hide: &'static str,
    pub note: &'static str,
    /// The service's counters, or None when there is no current reading.
    pub facts: Option<Vec<Fact>>,
    pub unknown: &'static str,
    pub separate: &'static str,
}

impl HealthBar {
    /// `system`: the current or last observed service status; `owned_pause`:
    /// the manager paused the bridge for its own installation; `renewing`:
    /// a status read is under way after the last one aged.
    pub fn build(system: Option<&System>, owned_pause: bool, renewing: bool) -> Self {
        let unobserved = renewing && system.is_none();
        // The same row shape is retained through readback transitions.
        let paused = owned_pause && system.is_none_or(|system|
            system.pending_transactions == 0 && system.stale_transports == 0
                && (!system.capacity_available() || !system.cleanup_unconfirmed));
        let health = system.map(service::health).unwrap_or(Health::Unavailable);
        let status = if paused {
            Status { words: "Bridge temporarily paused for installation", class: StatusClass::Working }
        } else if unobserved {
            Status { words: "Refreshing current status", class: StatusClass::Working }
        } else {
            Status { words: health.title(), class: match health {
                Health::Ready | Health::InUse => StatusClass::Ready,
                Health::Maintenance => StatusClass::Working,
                Health::NeedsAttention => StatusClass::Problem,
                Health::Unavailable => StatusClass::Unknown,
            } }
        };
        let fact = |label, value: Option<String>, missing| Fact { label, value, missing };
        Self {
            status,
            show: "Service details",
            hide: "Hide service details",
            note: if unobserved { "Waiting for current status." }
                else if renewing { "Last observed status · refreshing…" }
                else { "Service status updates automatically." },
            facts: system.map(|system| {
                let mut facts = if system.capacity_available() { vec![
                    fact("DSP", Some(format!("{} / {}", system.dsp, system.ceiling)), "DSP unknown"),
                    fact("Install / scan", Some(system.maintenance.to_string()), "Install / scan unknown"),
                    fact("Stale transports", Some(system.stale_transports.to_string()), "Stale transports unknown"),
                ] } else { vec![fact("DSP and capacity", None, "DSP and capacity unknown")] };
                facts.push(fact("Pending transactions", Some(system.pending_transactions.to_string()),
                    "Pending transactions unknown"));
                facts.push(activity_certainty(system).cleanup());
                facts
            }),
            unknown: "Current service capacity and cleanup are unknown.",
            separate: "Plug-in availability is shown separately.",
        }
    }
}

/// The one-line update headline, on every page but Setup.
pub fn update_headline(headline: Option<String>) -> Option<(String, Control)> {
    headline.map(|line| (line, Control::go("View update", Importance::Secondary, Destination::Setup)))
}

/// The window's last request as its own feedback records it.
#[derive(Clone, Copy, Debug)]
pub struct Request<'a> {
    pub text: &'a str,
    pub journey: Option<&'a InstallationJourney>,
    pub operation: Option<&'a str>,
    pub transitions: &'a [String],
    /// A problem report is ready to save.
    pub report: bool,
    /// The request still holds the window's controls.
    pub blocking: bool,
}

/// What the window says while a click waits for current status.
pub const CAPTURED: &str = "Click received. Waiting for the current readback; no operation submitted yet.";

/// The current request, its installation progress and its record.
#[derive(Clone, Debug)]
pub struct RequestBar {
    pub title: &'static str,
    pub working: bool,
    pub words: String,
    pub progress: Option<RequestProgress>,
    pub details: Option<Disclosure>,
    pub report: Option<(&'static str, Control)>,
    pub dismiss: Option<Control>,
}

#[derive(Clone, Debug)]
pub struct RequestProgress {
    pub counts: Option<String>,
    pub problem: Option<String>,
    pub title: &'static str,
    pub headline: &'static str,
    pub results: Results,
}

impl RequestBar {
    /// `message`: the window's last notice; `working`: a foreground request
    /// or file choice is in flight; `can_save`: nothing else is pending.
    pub fn build(request: Option<Request>, message: &str, working: bool, can_save: bool) -> Option<Self> {
        if request.is_none() && message.is_empty() && !working { return None; }
        Some(Self {
            title: "Current request",
            working,
            words: match &request {
                None if working && message.is_empty() => "Working…".into(),
                None => message.into(),
                Some(request) if request.text == CAPTURED => "Checking current status before starting your request…".into(),
                Some(request) => request.text.into(),
            },
            progress: request.and_then(|request| request.journey).map(|journey| {
                let available = journey.plugins.iter().filter(|plugin|
                    plugin.stage == InstallationPluginStage::Available).count();
                let attention = journey.plugins.iter().filter(|plugin| matches!(plugin.stage,
                    InstallationPluginStage::NeedsAttention | InstallationPluginStage::ReplacementRequired)).count();
                RequestProgress {
                    counts: (!journey.plugins.is_empty())
                        .then(|| format!("{available} plug-ins connected · {attention} need attention")),
                    problem: journey.failure.as_ref().map(|failure| journey::failure_text(failure, None)),
                    title: "Installation progress",
                    headline: journey::phase_label(journey),
                    results: Results::build(journey, None, None),
                }
            }),
            details: request.map(|request| Disclosure::new("Request details", vec![
                Text::body(request.text).into(),
                Text::body(request.operation.unwrap_or("Operation identity not yet confirmed")).into(),
                Text::body(request.transitions.join(" / ")).into(),
            ])),
            report: request.filter(|request| request.report).map(|_| (
                "Includes versions, setup history and bridge diagnostics. No licences, accounts, plug-in files or prefixes. Nothing is uploaded.",
                Control { label: "Save problem report", importance: Importance::Secondary, enabled: can_save,
                    does: Local::SaveReport })),
            dismiss: (!request.is_some_and(|request| request.blocking)).then_some(Control {
                label: "Dismiss", importance: Importance::Secondary, enabled: true, does: Local::Dismiss }),
        })
    }
}

/// Said above a plug-in's controls when its status is not current.
pub fn plugins_not_current(renewing: bool) -> Text {
    Text::caution(if renewing { "Last observed plug-in status · refreshing…" }
        else { "Current status is unavailable. Setup actions wait for current readback." })
}

/// Home and Setup before the manager's first answer.
pub const CHECKING: &str = "Checking current manager status. Readiness and actions will appear after readback.";
/// The technical pages before their records arrive.
pub const LOADING_DETAILS: &str = "Loading technical details. Home and Setup remain available.";
pub const LOADING_CURRENT: &str = "Loading current plug-ins and setup…";

pub fn retry_details() -> Control {
    Control { label: "Try loading details again", importance: Importance::Secondary, enabled: true,
        does: Local::RetryDetails }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system() -> System {
        serde_json::from_str::<Snapshot>(include_str!("../../examples/library-preview.json")).unwrap().system
    }

    #[test]
    fn health_keeps_its_row_through_readback_and_says_what_it_knows() {
        let mut system = system();
        let ready = HealthBar::build(Some(&system), false, false);
        assert_eq!(ready.status, Status { words: "Bridge service running", class: StatusClass::Ready });
        let lines: Vec<_> = ready.facts.unwrap().iter().map(Fact::labelled).collect();
        assert_eq!(lines, ["DSP 0 / 6", "Install / scan 0", "Stale transports 0", "Pending transactions 0",
            "Cleanup confirmed"]);
        let refreshing = HealthBar::build(None, false, true);
        assert_eq!(refreshing.status.words, "Refreshing current status");
        assert_eq!(refreshing.note, "Waiting for current status.");
        assert!(refreshing.facts.is_none());
        let gone = HealthBar::build(None, false, false);
        assert_eq!(gone.status.class, StatusClass::Unknown);
        system.service = "capacity unavailable".into();
        let lines: Vec<_> = HealthBar::build(Some(&system), false, false).facts.unwrap().iter()
            .map(Fact::labelled).collect();
        assert_eq!(lines, ["DSP and capacity unknown", "Pending transactions 0", "Cleanup unknown"]);
        // The manager's own installation pause is not a fault, unless it left one.
        system.service = "active".into();
        assert_eq!(HealthBar::build(Some(&system), true, false).status.class, StatusClass::Working);
        system.pending_transactions = 1;
        assert_eq!(HealthBar::build(Some(&system), true, false).status.class, StatusClass::Problem);
    }

    #[test]
    fn the_request_bar_names_its_request_and_keeps_blocking_requests_undismissable() {
        assert!(RequestBar::build(None, "", false, true).is_none());
        assert_eq!(RequestBar::build(None, "", true, true).unwrap().words, "Working…");
        let transitions = vec!["captured".to_owned()];
        let captured = Request { text: CAPTURED, journey: None, operation: None, transitions: &transitions,
            report: false, blocking: true };
        let bar = RequestBar::build(Some(captured), "", false, true).unwrap();
        assert_eq!(bar.words, "Checking current status before starting your request…");
        assert!(bar.dismiss.is_none() && bar.report.is_none());
        let report = Request { text: "Problem report created.", report: true, blocking: false, ..captured };
        let bar = RequestBar::build(Some(report), "", false, false).unwrap();
        assert!(!bar.report.as_ref().unwrap().1.enabled);
        assert_eq!(bar.dismiss.unwrap().does, Local::Dismiss);
    }
}

//! Activity: live and recent bridge sessions, what needs attention, and the
//! sanitized incidents the manager kept. Current observations bind exact
//! product identities. Legacy technical snapshots retain their class-only
//! history without supplying current product activity.
use super::offers::{self, Importance};
use super::product::terminal_text;
use super::{record, AttemptKey, Control, Destination, Detail, Disclosure, ProductKey, Text};
use crate::model::{Product, Snapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attention {
    pub title: String,
    pub detail: String,
    pub destination: Destination,
}

pub fn attention_destination(target: &crate::model::AttentionTarget) -> Destination {
    match target {
        crate::model::AttentionTarget::Product { product } => Destination::Product(ProductKey {
            environment: product.environment.clone(), module_sha256: product.module_sha256.clone(),
            class_id: product.class_id.clone() }),
        crate::model::AttentionTarget::Setup { installer, environment } => Destination::Attempt(AttemptKey {
            installer: installer.clone(), environment: environment.clone() }),
        crate::model::AttentionTarget::Service => Destination::Diagnostics,
        crate::model::AttentionTarget::Session { session } => Destination::Session(session.clone()),
    }
}

pub fn attentions(snapshot: &Snapshot) -> Vec<Attention> {
    snapshot.presentation.observed().into_iter().flat_map(|view| &view.attention)
        .map(|item| Attention { title: item.title.clone(), detail: item.detail.clone(),
            destination: attention_destination(&item.target) }).collect()
}

pub fn session_product<'a>(snapshot: &'a Snapshot, class_id: &str) -> Option<&'a Product> {
    let matching: Vec<_> = snapshot
        .products
        .iter()
        .filter(|product| !class_id.is_empty() && product.class_id == class_id)
        .collect();
    (matching.len() == 1)
        .then(|| matching.into_iter().next())
        .flatten()
}

pub fn session_display_name<'a>(snapshot: &'a Snapshot, class_id: &str) -> &'a str {
    let mut matches = snapshot
        .products
        .iter()
        .filter(|product| !class_id.is_empty() && product.class_id == class_id);
    match (matches.next(), matches.next()) {
        (Some(product), None) => &product.name,
        (Some(_), Some(_)) => "Shared plug-in class · exact build unavailable",
        _ => "Plug-in class unavailable",
    }
}

/// The sanitized incident summary, most important first.
pub fn incident_lines(v: &serde_json::Value) -> Vec<String> {
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

/// The Activity page.
#[derive(Clone, Debug, PartialEq)]
pub struct Activity {
    pub heading: &'static str,
    pub intro: &'static str,
    pub notice: Option<Text>,
    pub retry: Option<Control>,
    pub live: Sessions,
    pub recent: Sessions,
    pub attention: Attentions,
    pub incidents: Incidents,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sessions {
    pub title: &'static str,
    /// Said when the rows are not a live confirmation.
    pub warning: Option<&'static str>,
    pub empty: Option<&'static str>,
    pub rows: Vec<SessionRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionRow {
    pub recent: bool,
    /// Position in its list, for a row without a session identity.
    pub index: usize,
    pub session: Option<String>,
    pub name: String,
    pub state: String,
    pub cleanup: Option<&'static str>,
    pub recovery: Option<crate::recovery::Recovery>,
    /// Opens the plug-in, when exactly one installed build has this class.
    pub open: Option<Control>,
    pub record: Disclosure,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Attentions {
    pub title: &'static str,
    pub empty: Option<&'static str>,
    pub items: Vec<AttentionItem>,
    /// Shown when more items exist than are listed.
    pub more: Option<Control>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttentionItem {
    pub title: String,
    pub detail: String,
    pub open: Control,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Incidents {
    pub title: &'static str,
    pub empty: Option<&'static str>,
    /// Newest first, each under its state and identity.
    pub incidents: Vec<(String, Disclosure)>,
}

impl Sessions {
    fn build(snapshot: &Snapshot, recent: bool) -> Self {
        let available = snapshot.system.capacity_available();
        let rows: Vec<_> = snapshot.active_sessions.iter().filter(|row| (row["recent"] == true) == recent).collect();
        Self {
            title: if recent { "Recent session failures" } else { "Live bridge sessions" },
            warning: (!recent && !available).then_some(
                "Current session status is unavailable. Rows below are the last successful snapshot, not a live confirmation."),
            empty: rows.is_empty().then_some(if recent { "No recent terminal session is retained." }
                else if available { "No bridge sessions are reported." }
                else { "No current session list is available." }),
            rows: rows.into_iter().enumerate().map(|(index, row)| {
                let class_id = row["class_id"].as_str().unwrap_or("");
                SessionRow { recovery:None,
                    recent,
                    index,
                    session: row["session"].as_str().map(str::to_owned),
                    name: session_display_name(snapshot, class_id).into(),
                    state: if recent {
                        format!("Most recent failed session · {}", terminal_text(row["terminal"].as_str().unwrap_or("")))
                    } else if row["state"] == "cleanup_unconfirmed" {
                        "Cleanup unconfirmed · this owner is not proof of live audio".into()
                    } else if let Some(terminal) = row["terminal"].as_str() {
                        format!("Session unavailable · {}", terminal_text(terminal))
                    } else { "Running bridge session".into() },
                    cleanup: recent.then_some(if row["cleanup_confirmed"] == true && row["transport_retired"] == true {
                        "Cleanup and transport retirement confirmed"
                    } else { "Cleanup or transport retirement is unconfirmed" }),
                    open: session_product(snapshot, class_id).map(|product| Control::go("View plug-in",
                        Importance::Secondary, Destination::Product(ProductKey::from(product)))),
                    record: Disclosure::new("Technical session details", vec![record(row)]),
                }
            }).collect(),
        }
    }
}

impl Attentions {
    pub fn build(snapshot: &Snapshot, limit: usize) -> Self {
        let items = attentions(snapshot);
        Self {
            title: "Needs attention",
            empty: items.is_empty().then_some(if snapshot.presentation.observed().is_some() { "Nothing needs attention right now." } else { "Current attention items are unavailable." }),
            more: (items.len() > limit).then(|| Control::go("See all attention items", Importance::Secondary,
                Destination::Activity)),
            items: items.into_iter().take(limit).enumerate().map(|(index, item)| AttentionItem {
                open: Control::go(if index == 0 { "Review this" } else { "Open item" }, Importance::Secondary,
                    item.destination),
                title: item.title,
                detail: item.detail,
            }).collect(),
        }
    }
}

impl Activity {
    /// Ordinary Activity uses bounded owner summaries, independently of the
    /// optional deep technical snapshot. These records never supply offers.
    pub fn from_current(current: &Snapshot, read: crate::diagnostics::Section<'_, &crate::model::Activity>,
        in_flight: bool, superseded: bool, actions: Option<&Snapshot>, pending: bool) -> Self {
        use crate::{diagnostics::Section, model::Observation as O};
        let mut view = Self { heading: "Activity", intro: "Observed bridge processing and retained incident summaries.",
            notice: None, retry: None,
            live: Sessions { title: "Live bridge instances", warning: None, empty: None, rows: vec![] },
            recent: Sessions { title: "Retained session failures", warning: None, empty: None, rows: vec![] },
            attention: Attentions::build(current, usize::MAX),
            incidents: Incidents { title: "Incident summaries", empty: None, incidents: vec![] } };
        let mut failures = Vec::new();
        let activity = match read {
            Section::Loaded(activity) => Some(activity),
            Section::Loading => { view.notice = Some(Text::body("Loading current activity…")); None },
            Section::NotRequested => { failures.push("Activity has not been read.".into()); None },
            Section::Failed(reason) => { failures.push(format!("Activity could not be loaded. {reason}")); None },
        };
        if let Some(activity) = activity {
            match &activity.live.instances {
                O::Unavailable(reason) => failures.push(format!("Live instances unavailable: {reason}")),
                O::Observed(instances) => {
                    view.live.empty = instances.is_empty().then_some("No admitted bridge instances are reported.");
                    view.live.rows = instances.iter().enumerate().map(|(index, instance)| {
                        let mut row = exact_row(current, &instance.product, &instance.session, index, false);
                        row.state = format!("{} · {}", crate::product::host_text(instance), crate::product::processing_text(instance));
                        row.cleanup = match &instance.cleanup {
                            crate::model::CleanupState::Confirmed => Some("Cleanup confirmed"),
                            crate::model::CleanupState::Unconfirmed => Some("Cleanup unconfirmed"),
                            _ => None,
                        };
                        let mut details = vec![Text::body(crate::product::host_text(instance)).into(),
                            Text::detail("Processing progress does not establish nonzero signal or audible sound.").into()];
                        details.push(Text::detail(match &instance.consumer {
                            O::Observed(consumer) => format!("Admitted native consumer process: {} · current process {}", consumer.identity.pid,
                                match &consumer.current { O::Observed(true) => "confirmed", O::Observed(false) => "no longer matches", O::Unavailable(_) => "unavailable" }),
                            O::Unavailable(reason) => format!("Native consumer identity unavailable: {reason}"),
                        }).into());
                        row.record = Disclosure::new("Instance observations", details);
                        let product = actions.and_then(|snapshot|instance.product.observed().and_then(|key|
                            snapshot.products.iter().find(|product|key.matches(product))));
                        row.recovery = crate::recovery::for_instance(instance,product,None,pending || superseded);
                        row
                    }).collect();
                }
            }
            match &activity.recent {
                O::Unavailable(reason) => failures.push(format!("Retained failure summaries unavailable: {reason}")),
                O::Observed(instances) => {
                    view.recent.empty = instances.is_empty().then_some("No terminal failure summary is retained.");
                    view.recent.rows = instances.iter().enumerate().map(|(index, instance)| {
                        let mut row = exact_row(current, &instance.product, &instance.session, index, true);
                        row.state = format!("Retained failure · {}", failure_text(instance.failure));
                        row.cleanup = Some(if instance.cleanup_confirmed && instance.transport_retired {
                            "Cleanup and transport retirement confirmed" } else { "Cleanup or transport retirement unconfirmed" });
                        row.record = Disclosure::new("Retained observation", vec![Text::detail(
                            "This is the retained terminal summary, not a complete history of closed sessions.").into()]);
                        row
                    }).collect();
                }
            }
            match &activity.incidents {
                O::Unavailable(reason) => failures.push(format!("Incident summaries unavailable: {reason}")),
                O::Observed(incidents) => {
                    view.incidents.empty = incidents.is_empty().then_some("No incident is retained.");
                    view.incidents.incidents = incidents.iter().rev().map(|incident| {
                        let mut content = vec![Text::body(format!("State: {}", incident.state.replace('_', " "))).into()];
                        if let O::Observed(outcome) = &incident.outcome { content.push(Text::body(format!("Outcome: {outcome}")).into()); }
                        content.push(Text::detail(if incident.report_retained {
                            "A retained report is present. Open Support technical inspection to read its full details and current export offer."
                        } else { "No retained share report is present." }).into());
                        (incident.id.clone(), Disclosure::new(format!("{} · {}", incident.state.replace('_', " "), incident.id), content))
                    }).collect();
                }
            }
        }
        if !failures.is_empty() {
            view.notice = Some(Text::caution(failures.join("\n")));
            view.retry = Some(Control { label: "Refresh activity", importance: Importance::Secondary,
                enabled: !in_flight && !superseded, does: crate::Local::Refresh });
        }
        view
    }
    /// `pending`: a request or readback is in flight.
    pub fn build(snapshot: &Snapshot, pending: bool) -> Self {
        let busy = snapshot.system.inactive_reason();
        Self {
            heading: "Activity",
            intro: "Bridge sessions, manager work and retained incidents.",
            notice: None, retry: None,
            live: Sessions::build(snapshot, false),
            recent: Sessions::build(snapshot, true),
            attention: Attentions::build(snapshot, usize::MAX),
            incidents: Incidents {
                title: "Incidents",
                empty: snapshot.recent_incidents.is_empty().then_some("No sanitized incident is retained."),
                incidents: snapshot.recent_incidents.iter().rev().map(|incident| {
                    let lines = incident_lines(&incident.summary);
                    let mut content: Vec<Detail> = lines.iter().take(2).map(|line| Text::body(line).into()).collect();
                    if let Some(offer) = &incident.export {
                        content.push(Detail::Offers(offers::row(std::slice::from_ref(offer), Importance::Secondary,
                            busy, pending)));
                    }
                    let mut report: Vec<Detail> = lines.iter().skip(2).map(|line| Text::detail(line).into()).collect();
                    report.push(record(&incident.summary));
                    content.push(Detail::Disclosure(Disclosure::new("Sanitized technical report", report)));
                    (incident.id.clone(), Disclosure::new(format!("{} · {}", incident.state.replace('_', " "), incident.id), content))
                }).collect(),
            },
        }
    }
}

fn failure_text(failure: crate::model::InstanceFailure) -> &'static str {
    match failure {
        crate::model::InstanceFailure::WindowsHostExited => "Windows plug-in host exited unexpectedly",
        crate::model::InstanceFailure::EditorControllerFailed => "Editor or controller failed",
        crate::model::InstanceFailure::TransportFailed => "Bridge transport failed",
    }
}
fn exact_row(current: &Snapshot, identity: &crate::model::Observation<crate::model::ProductIdentity>,
    session: &str, index: usize, recent: bool) -> SessionRow {
    let key = identity.observed();
    let product = key.and_then(|key| current.products.iter().find(|product| key.matches(product)));
    SessionRow { recovery:None, recent, index, session: Some(session.into()),
        name: product.map_or_else(|| if key.is_some() { "Exact admitted build is absent from current products".into() }
            else { "Exact admitted build is unavailable".into() }, |product| product.name.clone()),
        state: String::new(), cleanup: None,
        open: product.map(|product| Control::go("View plug-in", Importance::Secondary,
            Destination::Product(ProductKey::from(product)))),
        record: Disclosure::new("Instance identity", vec![Text::detail(session).into()]) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::product::{ActivityCertainty, product_activity, last_load_text};

    fn snapshot() -> Snapshot {
        crate::samples::base()
    }

    #[test]
    fn refused_load_says_how_long_ago_and_reaches_the_attention_list() {
        // The elapsed-time wording itself is tested with the screen model.
        let mut snapshot = snapshot();
        let index = snapshot.products.iter()
            .position(|p| p.availability.as_ref().is_some_and(|a| a.available)
                && !matches!(p.disposition.as_str(), "needs_attention" | "quarantined" | "another_configuration"))
            .expect("an available plug-in in the preview");
        assert_eq!(last_load_text(&snapshot.products[index]), None);
        let name = snapshot.products[index].name.clone();
        snapshot.products[index].details["last_load"] = serde_json::json!({"outcome": "refused",
            "observed_at": 1_000,
            "problem": {"code": "windows_side_stopped", "what_happened": "It stayed empty.",
                "likely_cause": "Its Windows side stopped.", "what_to_try": "Load it again."}});
        let text = last_load_text(&snapshot.products[index]).unwrap();
        assert!(text.starts_with("Load refused ") && text.ends_with("It stayed empty. Its Windows side stopped. Load it again."));
        let crate::model::Observation::Observed(presentation) = &mut snapshot.presentation else { panic!() };
        presentation.attention = vec![crate::model::Attention { purpose: crate::model::AttentionPurpose::Recovery,
            target: crate::model::AttentionTarget::Product { product: crate::model::ProductIdentity::from(&snapshot.products[index]) },
            title: name.clone(), detail: text.clone(), action: None }];
        assert!(attentions(&snapshot).iter().any(|item| item.title == name && item.detail == text));
        // A refused audio start (#403) says so rather than a refused load.
        snapshot.products[index].details["last_load"]["outcome"] = "activation_refused".into();
        let text = last_load_text(&snapshot.products[index]).unwrap();
        assert!(text.starts_with("Audio start refused ") && text.ends_with("Load it again."));
    }

    #[test]
    fn retained_class_rows_are_technical_history_not_current_product_activity() {
        let mut snapshot = snapshot();
        let mut second = snapshot.products[1].clone(); second.module_sha256 = "another-build".into();
        snapshot.products.push(second);
        snapshot.active_sessions = vec![serde_json::json!({"class_id":"fixture-fragments","state":"active"})];
        for product in snapshot.products.iter().filter(|p| p.class_id == "fixture-fragments") {
            let view = product_activity(crate::diagnostics::Section::NotRequested, product);
            assert_eq!(view.active, 0); assert_ne!(view.certainty, ActivityCertainty::Confirmed);
            assert!(view.detail_label().contains("not been observed"));
        }
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        assert_eq!(session_display_name(&snapshot, "fixture-fragments"), "Shared plug-in class · exact build unavailable");
    }

    #[test]
    fn exact_attention_target_and_class_level_session_claims() {
        let mut snapshot = snapshot();
        let p = &snapshot.products[2];
        let key = ProductKey::from(p);
        let owner = crate::model::Attention { purpose: crate::model::AttentionPurpose::Connection,
            target: crate::model::AttentionTarget::Product { product: crate::model::ProductIdentity::from(p) },
            title: p.name.clone(), detail: "Rescan this exact build.".into(), action: None };
        let crate::model::Observation::Observed(presentation) = &mut snapshot.presentation else { panic!() };
        presentation.attention = vec![owner];
        let item = attentions(&snapshot).into_iter().next().unwrap();
        assert_eq!(item.destination, Destination::Product(key));
        assert!(item.detail.contains("Rescan"));
        snapshot.active_sessions = vec![
            serde_json::json!({"class_id":"fixture-fragments","state":"active","recent":false}),
            serde_json::json!({"class_id":"fixture-fragments","state":"failed","recent":true,
                "terminal":"windows_host_exited","cleanup_confirmed":true,"transport_retired":true}),
        ];
        snapshot.products.push(snapshot.products[1].clone());
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        // Legacy class-only session rows never synthesize a product or session attention target.
        let before = attentions(&snapshot);
        snapshot.active_sessions[1]["session"] = serde_json::json!("exact-session");
        let after = attentions(&snapshot);
        assert_eq!(before.len(), after.len());
        assert!(after.iter().all(|item| !matches!(item.destination, Destination::Session(_))));
    }

    #[test]
    fn typed_live_observations_bind_the_exact_build_and_distinguish_unknown_from_empty() {
        use crate::model::{self, Observation as O};
        use crate::diagnostics::Section;
        let mut snapshot = snapshot();
        let original = snapshot.products[0].clone();
        let mut sibling = original.clone(); sibling.module_sha256 = "other-digest".into(); sibling.name = "Sibling build".into();
        snapshot.products.push(sibling.clone());
        let instance = model::LiveInstance { recovery:None, kind: model::InstanceKind::Daw, session: "exact-session".into(), product: O::Observed(model::ProductIdentity::from(&sibling)),
            consumer: O::Unavailable("No current process observation".into()), daw: O::Unavailable("Unrecognized".into()),
            transport: O::Unavailable("Not mapped".into()), lifecycle: O::Unavailable("Not read".into()),
            processing: model::ProcessingObservation::Advancing { epoch: 1, completed_end: 512, observed_age_ms: 2 },
            signal: model::SignalObservation::NotMeasured, terminal: O::Observed(None), cleanup: model::CleanupState::NotReported };
        let mut activity = model::Activity { schema: model::OPERATOR_SCHEMA, live: model::LiveActivity { instances: O::Observed(vec![instance]) },
            recent: O::Observed(vec![]), incidents: O::Observed(vec![]), system: snapshot.system.clone(), capture: serde_json::Value::Null,
            operation: None };
        let view = Activity::from_current(&snapshot, Section::Loaded(&activity), false, false, Some(&snapshot), false);
        assert_eq!(view.live.rows[0].name, "Sibling build");
        assert_eq!(view.live.rows[0].open.as_ref().unwrap().does, crate::Local::Go(Destination::Product(ProductKey::from(&sibling))));
        assert!(view.live.rows[0].state.contains("advancing") && view.live.rows[0].state.contains("audible output is not measured"));
        snapshot.products.pop();
        let view = Activity::from_current(&snapshot, Section::Loaded(&activity), false, false, Some(&snapshot), false);
        assert!(view.live.rows[0].open.is_none(), "same class and environment cannot substitute a digest");
        activity.live.instances = O::Unavailable("Registry is busy".into());
        let view = Activity::from_current(&snapshot, Section::Loaded(&activity), false, false, Some(&snapshot), false);
        assert!(view.live.empty.is_none() && view.notice.is_some() && view.retry.is_some());
        activity.live.instances = O::Observed(vec![]);
        let view = Activity::from_current(&snapshot, Section::Loaded(&activity), false, false, Some(&snapshot), false);
        assert!(view.live.empty.is_some() && view.notice.is_none());
        for pending in [true, false] {
            let view = Activity::from_current(&snapshot, Section::Failed("Disconnected"), pending, false, Some(&snapshot), pending);
            assert!(view.live.rows.is_empty() && view.live.empty.is_none());
            assert_eq!(view.retry.unwrap().enabled, !pending);
        }
    }

    use crate::window::Page;
    use crate::samples::{self, STATES};

    #[test]
    fn sample_states_list_sessions_attention_and_incidents_with_exact_offers() {
        for state in STATES {
            let snapshot = samples::snapshot(Page::Activity, Some(state));
            let view = Activity::build(&snapshot, false);
            let live = snapshot.active_sessions.iter().filter(|row| row["recent"] != true).count();
            assert_eq!(view.live.rows.len(), live, "{state}");
            assert_eq!(view.recent.rows.len(), snapshot.active_sessions.len() - live, "{state}");
            assert_eq!(view.live.warning.is_some(), !snapshot.system.capacity_available(), "{state}");
            assert_eq!(view.attention.items.len(), attentions(&snapshot).len(), "{state}");
            assert!(view.attention.more.is_none(), "{state}: Activity lists every item");
            assert_eq!(view.incidents.incidents.len(), snapshot.recent_incidents.len(), "{state}");
            for row in view.live.rows.iter().chain(&view.recent.rows) {
                // Only a class with one installed build opens a plug-in.
                let class = snapshot.active_sessions.iter().filter(|session| (session["recent"] == true) == row.recent)
                    .nth(row.index).and_then(|session| session["class_id"].as_str()).unwrap_or("");
                assert_eq!(row.open.is_some(), session_product(&snapshot, class).is_some(), "{state}");
            }
        }
        let snapshot = samples::snapshot(Page::Activity, Some("incidents"));
        let view = Activity::build(&snapshot, false);
        let states: Vec<_> = view.live.rows.iter().map(|row| row.state.as_str()).collect();
        assert_eq!(states, ["Running bridge session", "Cleanup unconfirmed · this owner is not proof of live audio",
            "Session unavailable · Windows plug-in host exited unexpectedly"]);
        assert_eq!(view.recent.rows[0].cleanup, Some("Cleanup or transport retirement is unconfirmed"));
        let (id, newest) = &view.incidents.incidents[0];
        assert_eq!(id, &"42".repeat(16), "newest first");
        assert_eq!(newest.title, format!("cleanup unconfirmed · {}", "42".repeat(16)));
        let (_, saved) = &view.incidents.incidents[1];
        assert!(saved.content.iter().any(|detail| matches!(detail, Detail::Offers(row)
            if row.offers[0].label == "Save incident report")));
        assert!(view.attention.items.iter().any(|item| matches!(&item.open.does, crate::Local::Go(Destination::Attempt(_)))));
    }
}

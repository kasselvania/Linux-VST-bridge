//! Activity: live and recent bridge sessions, what needs attention, and the
//! sanitized incidents the manager kept. A session record names a plug-in
//! class, not an installed build; several builds of one class stay
//! class-level everywhere.
use super::offers::{self, Importance};
use super::product::{last_load_text, product_activity, product_issue, terminal_text};
use super::{record, AttemptKey, Control, Destination, Detail, Disclosure, ProductKey, Text};
use crate::model::{Product, Snapshot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attention {
    pub title: String,
    pub detail: String,
    pub destination: Destination,
}

pub fn attentions(snapshot: &Snapshot) -> Vec<Attention> {
    let mut items = Vec::new();
    let mut seen_sessions = std::collections::HashSet::new();
    let system = &snapshot.system;
    if !system.capacity_available() {
        items.push(Attention {
            title: "Bridge readback unavailable".into(),
            detail: "Active plug-ins and safe admission cannot be confirmed. Refresh the manager."
                .into(),
            destination: Destination::Diagnostics,
        });
    }
    if system.capacity_available() && system.cleanup_unconfirmed {
        items.push(Attention {
            title: "Cleanup is unconfirmed".into(),
            detail: "The manager is blocking unsafe new work until ownership is resolved.".into(),
            destination: Destination::Diagnostics,
        });
    }
    if system.pending_transactions > 0 {
        items.push(Attention {
            title: "Publication transaction needs review".into(),
            detail: format!(
                "{} pending transaction(s). Open Setup for the manager's reconcile control.",
                system.pending_transactions
            ),
            destination: Destination::Setup,
        });
    }
    if system.stale_transports > 0 && system.capacity_available() {
        items.push(Attention {
            title: "Old bridge transport remains".into(),
            detail: format!(
                "{} stale transport(s) reported by the manager.",
                system.stale_transports
            ),
            destination: Destination::Diagnostics,
        });
    }
    for (index, row) in snapshot.active_sessions.iter().enumerate() {
        let Some(class_id) = row["class_id"].as_str().filter(|id| !id.is_empty()) else {
            continue;
        };
        let Some(terminal) = row["terminal"].as_str() else {
            continue;
        };
        if snapshot
            .products
            .iter()
            .filter(|product| product.class_id == class_id)
            .count()
            < 2
        {
            continue;
        }
        let session = row["session"].as_str().filter(|id| !id.is_empty());
        if !seen_sessions.insert(session.map_or_else(|| format!("legacy-{index}"), str::to_owned)) {
            continue;
        }
        items.push(Attention {
            title: "Plug-in session needs attention".into(),
            detail: format!("{}. Several installed builds share this class; the exact build is unavailable in the session record.", terminal_text(terminal)),
            destination: session.map_or(Destination::Activity, |id| Destination::Session(id.into())),
        });
    }
    for product in &snapshot.products {
        let activity = product_activity(snapshot, product);
        let problematic = matches!(
            product.disposition.as_str(),
            "needs_attention" | "quarantined" | "another_configuration"
        );
        let detail = if problematic && activity.shared_class {
            Some(
                product_issue(product)
                    .unwrap_or_else(|| "Review this plug-in's manager state.".into()),
            )
        } else if !activity.failed_live.is_empty() {
            (!activity.shared_class).then(|| format!("Live session: {}.", activity.failed_live[0]))
        } else if activity.recent_cleanup_unconfirmed {
            (!activity.shared_class)
                .then(|| "A recent failed session has unconfirmed cleanup.".into())
        } else if let Some(refused) = (!problematic).then(|| last_load_text(product)).flatten() {
            Some(refused)
        } else if problematic {
            Some(
                product_issue(product)
                    .unwrap_or_else(|| "Review this plug-in's manager state.".into()),
            )
        } else {
            activity
                .recent_failure
                .filter(|_| !activity.shared_class)
                .map(|failure| format!("Most recent session: {failure}."))
        };
        if let Some(detail) = detail {
            items.push(Attention {
                title: product.name.clone(),
                detail,
                destination: Destination::Product(ProductKey::from(product)),
            });
        }
    }
    for attempt in &snapshot.onboarding {
        if matches!(
            attempt.state.as_str(),
            "needs_attention" | "cleanup_unconfirmed" | "failed"
        ) {
            items.push(Attention {
                title: "Installation needs attention".into(),
                detail: attempt.required_human_action.clone(),
                destination: Destination::Attempt(AttemptKey {
                    installer: attempt.installer.clone(),
                    environment: attempt.environment.clone(),
                }),
            });
        }
    }
    items
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
#[derive(Clone, Debug)]
pub struct Activity {
    pub heading: &'static str,
    pub intro: &'static str,
    pub live: Sessions,
    pub recent: Sessions,
    pub attention: Attentions,
    pub incidents: Incidents,
}

#[derive(Clone, Debug)]
pub struct Sessions {
    pub title: &'static str,
    /// Said when the rows are not a live confirmation.
    pub warning: Option<&'static str>,
    pub empty: Option<&'static str>,
    pub rows: Vec<SessionRow>,
}

#[derive(Clone, Debug)]
pub struct SessionRow {
    pub recent: bool,
    /// Position in its list, for a row without a session identity.
    pub index: usize,
    pub session: Option<String>,
    pub name: String,
    pub state: String,
    pub cleanup: Option<&'static str>,
    /// Opens the plug-in, when exactly one installed build has this class.
    pub open: Option<Control>,
    pub record: Disclosure,
}

#[derive(Clone, Debug)]
pub struct Attentions {
    pub title: &'static str,
    pub empty: Option<&'static str>,
    pub items: Vec<AttentionItem>,
    /// Shown when more items exist than are listed.
    pub more: Option<Control>,
}

#[derive(Clone, Debug)]
pub struct AttentionItem {
    pub title: String,
    pub detail: String,
    pub open: Control,
}

#[derive(Clone, Debug)]
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
                SessionRow {
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
            empty: items.is_empty().then_some("Nothing needs attention right now."),
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
    /// `pending`: a request or readback is in flight.
    pub fn build(snapshot: &Snapshot, pending: bool) -> Self {
        let busy = snapshot.system.inactive_reason();
        Self {
            heading: "Activity",
            intro: "Bridge sessions, manager work and retained incidents. Managed DAW workspaces will have separate application activity when the manager reports it.",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::product::{activity_certainty, ActivityCertainty};

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
        assert!(attentions(&snapshot).iter().any(|item| item.title == name && item.detail == text));
        // A refused audio start (#403) says so rather than a refused load.
        snapshot.products[index].details["last_load"]["outcome"] = "activation_refused".into();
        let text = last_load_text(&snapshot.products[index]).unwrap();
        assert!(text.starts_with("Audio start refused ") && text.ends_with("Load it again."));
    }

    #[test]
    fn retained_active_row_never_becomes_a_current_product_claim_without_certain_capacity() {
        let mut snapshot = snapshot();
        snapshot.active_sessions = vec![serde_json::json!({
            "session":"exact-session", "class_id":"fixture-fragments", "state":"active", "recent":false
        })];
        let product = &snapshot.products[1];
        let confirmed = product_activity(&snapshot, product);
        assert_eq!(confirmed.certainty, ActivityCertainty::Confirmed);
        assert_eq!(confirmed.active, 1);
        assert_eq!(confirmed.detail_label(), "1 active instance(s)");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup().labelled(),
            "Cleanup confirmed"
        );

        snapshot.system.service = "capacity unavailable".into();
        let unavailable = product_activity(&snapshot, product);
        assert_eq!(unavailable.certainty, ActivityCertainty::Unavailable);
        assert_eq!(unavailable.active, 0);
        assert_eq!(unavailable.detail_label(), "Current activity unavailable");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup().labelled(),
            "Cleanup unknown"
        );
        snapshot.system.service = "active".into();
        snapshot.system.cleanup_unconfirmed = true;
        let uncertain = product_activity(&snapshot, product);
        assert_eq!(uncertain.certainty, ActivityCertainty::CleanupUncertain);
        assert_eq!(uncertain.active, 0);
        assert_eq!(
            uncertain.detail_label(),
            "Cleanup unresolved · current instances not confirmed"
        );
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup().labelled(),
            "Cleanup unconfirmed"
        );
        snapshot.system.service = "capacity unavailable".into();
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup().labelled(),
            "Cleanup unknown"
        );
        let unavailable_with_conservative_flag = product_activity(&snapshot, product);
        assert_eq!(
            unavailable_with_conservative_flag.certainty,
            ActivityCertainty::Unavailable
        );
        assert!(!attentions(&snapshot)
            .iter()
            .any(|item| item.title == "Cleanup is unconfirmed"));
    }

    #[test]
    fn shared_class_session_stays_class_level_on_both_product_surfaces() {
        let mut snapshot = snapshot();
        let mut second_build = snapshot.products[1].clone();
        second_build.module_sha256 = "different-build".into();
        second_build.environment = "different-environment".into();
        snapshot.products.push(second_build);
        snapshot.active_sessions = vec![serde_json::json!({
            "session":"exact-session", "class_id":"fixture-fragments", "state":"active", "recent":false
        })];
        for product in snapshot
            .products
            .iter()
            .filter(|p| p.class_id == "fixture-fragments")
        {
            let activity = product_activity(&snapshot, product);
            assert!(activity.shared_class);
            assert_eq!(activity.active, 1);
            assert_eq!(
                activity.detail_label(),
                "1 active in this plug-in class · exact build unavailable"
            );
        }
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        assert_eq!(
            session_display_name(&snapshot, "fixture-fragments"),
            "Shared plug-in class · exact build unavailable"
        );
        assert_eq!(snapshot.active_sessions[0]["session"], "exact-session");
    }

    #[test]
    fn exact_attention_target_and_class_level_session_claims() {
        let mut snapshot = snapshot();
        let p = &snapshot.products[2];
        let key = ProductKey::from(p);
        let item = attentions(&snapshot)
            .into_iter()
            .find(|item| item.title == p.name)
            .unwrap();
        assert_eq!(item.destination, Destination::Product(key));
        assert!(item.detail.contains("Rescan"));
        snapshot.active_sessions = vec![
            serde_json::json!({"class_id":"fixture-fragments","state":"active","recent":false}),
            serde_json::json!({"class_id":"fixture-fragments","state":"failed","recent":true,
                "terminal":"windows_host_exited","cleanup_confirmed":true,"transport_retired":true}),
        ];
        let state = product_activity(&snapshot, &snapshot.products[1]);
        assert_eq!(state.active, 1);
        assert_eq!(
            state.recent_failure,
            Some("Windows plug-in host exited unexpectedly")
        );
        assert!(!state.recent_cleanup_unconfirmed);
        snapshot.products.push(snapshot.products[1].clone());
        assert!(product_activity(&snapshot, &snapshot.products[1]).shared_class);
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        let class_items: Vec<_> = attentions(&snapshot)
            .into_iter()
            .filter(|item| item.destination == Destination::Activity)
            .collect();
        assert_eq!(class_items.len(), 1);
        assert!(class_items[0].detail.contains("exact build is unavailable"));
        snapshot.active_sessions[1]["session"] = serde_json::json!("exact-session");
        assert!(attentions(&snapshot)
            .iter()
            .any(|item| item.destination == Destination::Session("exact-session".into())));
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
        assert!(view.attention.items.iter().any(|item| item.title == "Installation needs attention"
            && matches!(&item.open.does, crate::Local::Go(Destination::Attempt(_)))));
    }
}

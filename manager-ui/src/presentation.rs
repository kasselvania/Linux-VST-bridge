//! Read-only language and routing over the canonical operator snapshot.
use crate::model::{Product, Snapshot, System};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductKey {
    pub environment: String,
    pub class_id: String,
    pub module_sha256: String,
}

impl From<&Product> for ProductKey {
    fn from(product: &Product) -> Self {
        Self {
            environment: product.environment.clone(),
            class_id: product.class_id.clone(),
            module_sha256: product.module_sha256.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptKey {
    pub installer: String,
    pub environment: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Product(ProductKey),
    Attempt(AttemptKey),
    Session(String),
    Activity,
    Setup,
    Diagnostics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attention {
    pub title: String,
    pub detail: String,
    pub destination: Destination,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Ready,
    InUse,
    Maintenance,
    NeedsAttention,
    Unavailable,
}

impl Health {
    pub fn title(self) -> &'static str {
        match self {
            Self::Ready => "Bridge ready",
            Self::InUse => "Bridge in use",
            Self::Maintenance => "Setup in progress",
            Self::NeedsAttention => "Bridge needs attention",
            Self::Unavailable => "Bridge status unavailable",
        }
    }
}

pub fn health(system: &System) -> Health {
    if !system.capacity_available() {
        Health::Unavailable
    } else if system.cleanup_unconfirmed
        || system.pending_transactions > 0
        || system.stale_transports > 0
    {
        Health::NeedsAttention
    } else if system.maintenance > 0 {
        Health::Maintenance
    } else if system.dsp > 0 {
        Health::InUse
    } else {
        Health::Ready
    }
}

pub fn terminal_text(terminal: &str) -> &'static str {
    match terminal {
        "editor_controller_failed" => "Editor or controller failed",
        "windows_host_exited" => "Windows plug-in host exited unexpectedly",
        "transport_failed" => "Bridge connection failed",
        _ => "Session failed; details are available",
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivityCertainty {
    Confirmed,
    CleanupUncertain,
    #[default]
    Unavailable,
}

pub fn activity_certainty(system: &System) -> ActivityCertainty {
    // The manager conservatively sets cleanup_unconfirmed when capacity
    // readback is absent. That does not establish the current cleanup state.
    if !system.capacity_available() {
        ActivityCertainty::Unavailable
    } else if system.cleanup_unconfirmed {
        ActivityCertainty::CleanupUncertain
    } else {
        ActivityCertainty::Confirmed
    }
}

impl ActivityCertainty {
    pub fn cleanup_label(self) -> &'static str {
        match self {
            Self::Confirmed => "Cleanup confirmed",
            Self::CleanupUncertain => "Cleanup unconfirmed",
            Self::Unavailable => "Cleanup unknown",
        }
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ProductActivity {
    pub certainty: ActivityCertainty,
    pub active: usize,
    pub failed_live: Vec<&'static str>,
    pub recent_failure: Option<&'static str>,
    pub recent_cleanup_unconfirmed: bool,
    /// The manager's session record names a class, not an installed build.
    pub shared_class: bool,
}

impl ProductActivity {
    pub fn detail_label(&self) -> String {
        match self.certainty {
            ActivityCertainty::Confirmed if self.shared_class => format!(
                "{} active in this plug-in class · exact build unavailable",
                self.active
            ),
            ActivityCertainty::Confirmed => format!("{} active instance(s)", self.active),
            ActivityCertainty::CleanupUncertain => {
                "Cleanup unresolved · current instances not confirmed".into()
            }
            ActivityCertainty::Unavailable => "Current activity unavailable".into(),
        }
    }

    pub fn home_label(&self) -> String {
        match self.certainty {
            ActivityCertainty::Confirmed if self.shared_class => {
                "Class activity in Activity".into()
            }
            ActivityCertainty::Confirmed => format!("{} active", self.active),
            ActivityCertainty::CleanupUncertain => {
                "Cleanup unresolved · current instances not confirmed".into()
            }
            ActivityCertainty::Unavailable => "Current activity unavailable".into(),
        }
    }
}

pub fn product_activity(snapshot: &Snapshot, product: &Product) -> ProductActivity {
    let mut result = ProductActivity {
        certainty: activity_certainty(&snapshot.system),
        ..ProductActivity::default()
    };
    if product.class_id.is_empty() {
        return result;
    }
    result.shared_class = snapshot
        .products
        .iter()
        .filter(|other| other.class_id == product.class_id)
        .count()
        > 1;
    for row in snapshot
        .active_sessions
        .iter()
        .filter(|row| row["class_id"].as_str() == Some(product.class_id.as_str()))
    {
        if row["recent"] == true {
            if let Some(terminal) = row["terminal"].as_str() {
                result.recent_failure = Some(terminal_text(terminal));
                result.recent_cleanup_unconfirmed |=
                    row["cleanup_confirmed"] != true || row["transport_retired"] != true;
            }
        } else if result.certainty == ActivityCertainty::Confirmed {
            if let Some(terminal) = row["terminal"].as_str() {
                result.failed_live.push(terminal_text(terminal));
            } else if row["state"] == "active" {
                result.active += 1;
            }
        }
    }
    result
}

pub fn product_issue(product: &Product) -> Option<String> {
    let preparation = &product.details["preparation"];
    let management = [
        product.details["refusal"].as_str(),
        product.details["preparation_failure"].as_str(),
        preparation["operation"]["reason"].as_str(),
        preparation["recovery"].as_str(),
    ]
    .into_iter()
    .flatten()
    .find(|value| !value.trim().is_empty());
    management
        .map(|value| value.replace('_', " "))
        .or_else(|| {
            product.details["inspection_error"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned)
        })
        .or_else(|| {
            product.details["inspection_hint"]
                .as_str()
                .filter(|value| !value.trim().is_empty())
                .map(|value| value.replace('_', " "))
        })
        .or_else(|| {
            product
                .limitations
                .first()
                .map(|value| value.replace('_', " "))
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap()
    }

    #[test]
    fn health_distinguishes_ready_busy_unavailable_and_cleanup_uncertainty() {
        let mut system = snapshot().system;
        assert_eq!(health(&system), Health::Ready);
        system.dsp = 2;
        assert_eq!(health(&system), Health::InUse);
        system.maintenance = 1;
        assert_eq!(health(&system), Health::Maintenance);
        system.pending_transactions = 1;
        assert_eq!(health(&system), Health::NeedsAttention);
        system.pending_transactions = 0;
        system.cleanup_unconfirmed = true;
        assert_eq!(health(&system), Health::NeedsAttention);
        system.service = "capacity unavailable".into();
        assert_eq!(health(&system), Health::Unavailable);
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
        assert_eq!(confirmed.home_label(), "1 active");
        assert_eq!(confirmed.detail_label(), "1 active instance(s)");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup confirmed"
        );

        snapshot.system.service = "capacity unavailable".into();
        let unavailable = product_activity(&snapshot, product);
        assert_eq!(unavailable.certainty, ActivityCertainty::Unavailable);
        assert_eq!(unavailable.active, 0);
        assert_eq!(unavailable.home_label(), "Current activity unavailable");
        assert_eq!(unavailable.detail_label(), "Current activity unavailable");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unknown"
        );
        snapshot.system.service = "active".into();
        snapshot.system.cleanup_unconfirmed = true;
        let uncertain = product_activity(&snapshot, product);
        assert_eq!(uncertain.certainty, ActivityCertainty::CleanupUncertain);
        assert_eq!(uncertain.active, 0);
        assert_eq!(
            uncertain.home_label(),
            "Cleanup unresolved · current instances not confirmed"
        );
        assert_eq!(
            uncertain.detail_label(),
            "Cleanup unresolved · current instances not confirmed"
        );
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unconfirmed"
        );
        snapshot.system.service = "capacity unavailable".into();
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unknown"
        );
        let unavailable_with_conservative_flag = product_activity(&snapshot, product);
        assert_eq!(
            unavailable_with_conservative_flag.certainty,
            ActivityCertainty::Unavailable
        );
        assert_eq!(
            unavailable_with_conservative_flag.home_label(),
            "Current activity unavailable"
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
            assert_eq!(activity.home_label(), "Class activity in Activity");
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
}

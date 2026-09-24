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

#[derive(Default, Debug, PartialEq, Eq)]
pub struct ProductActivity {
    pub active: usize,
    pub failed_live: Vec<&'static str>,
    pub recent_failure: Option<&'static str>,
    pub recent_cleanup_unconfirmed: bool,
    /// The manager's session record names a class, not an installed build.
    pub shared_class: bool,
}

pub fn product_activity(snapshot: &Snapshot, product: &Product) -> ProductActivity {
    let mut result = ProductActivity::default();
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
        } else if let Some(terminal) = row["terminal"].as_str() {
            result.failed_live.push(terminal_text(terminal));
        } else if row["state"] == "active" {
            result.active += 1;
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
    if system.cleanup_unconfirmed {
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

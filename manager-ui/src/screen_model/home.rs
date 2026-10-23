//! Home: the one next step, then each plug-in at a glance, then the computer
//! setup check. Built from the manager's current overview and the last
//! explicit setup check; every control is a manager offer or opens a page.
use super::journey::{self, Results};
use super::offers::{self, Importance, Offer, OfferRow};
use super::{product, service, setup, AttemptKey, Control, Destination, Detail, Disclosure, Local,
    ProductKey, StatusClass, Text};
use crate::model::{Action, DawChoice, InstallationHealth, ReadinessAssessment, ReadinessOutcome,
    ReadinessProduct, Snapshot, SupportQualification};

pub struct Home {
    pub heading: &'static str,
    pub add: Control,
    /// Why the listed plug-ins may be out of date, when they are.
    pub freshness: Option<Text>,
    pub next: Next,
    /// The count above the plug-in rows, when there are plug-ins.
    pub summary: Option<String>,
    pub plugins: Vec<HomePlugin>,
    pub computer: ComputerCheck,
}

/// The one thing to do next, most urgent first.
pub enum Next {
    /// The bridge service itself needs the musician first.
    Service {
        title: &'static str,
        what_happened: &'static str,
        offers: OfferRow,
        review: Control,
    },
    /// An installation is unfinished.
    Setup {
        title: String,
        status: String,
        journey: Option<Results>,
        primary: Option<Offer>,
        open: Control,
    },
    /// The last setup check recommends a computer change.
    Computer {
        note: &'static str,
        title: String,
        detail: Option<String>,
        action: Option<Offer>,
        blocker: Option<String>,
        review: Control,
    },
    /// One plug-in to recover or set up, or the first one to add.
    Go {
        title: String,
        detail: Option<String>,
        open: Control,
    },
    /// Nothing waits: every plug-in is available.
    Ready {
        title: String,
        help: Disclosure,
    },
}

/// One plug-in at a glance.
pub struct HomePlugin {
    pub key: ProductKey,
    pub name: String,
    pub manage: Control,
    /// Current availability or install progress, while status is current.
    pub status: Option<&'static str>,
    /// What the last setup check found wrong with its installation.
    pub problem: Option<String>,
    pub check: Option<Disclosure>,
}

/// The computer setup check and the support report.
pub struct ComputerCheck {
    pub title: &'static str,
    pub intro: &'static str,
    pub check: Control,
    pub result: Option<CheckResult>,
}

pub struct CheckResult {
    pub blockers: Vec<String>,
    pub export: OfferRow,
    pub export_note: &'static str,
    pub observations: Disclosure,
}

pub fn installation_label(product: &ReadinessProduct) -> &'static str {
    let records = product.facts.iter().any(|fact| fact.name == "Validation scope"
        && fact.value.as_deref() == Some("control_records"));
    match (records, product.installation_health) {
        (true, InstallationHealth::Healthy) => "Current configuration: records and publication agree",
        (true, InstallationHealth::ActionRequired) => "Current configuration: needs attention",
        (true, InstallationHealth::Unknown) => "Current configuration: records incomplete",
        (false, InstallationHealth::Healthy) => "Current installation: healthy",
        (false, InstallationHealth::ActionRequired) => "Current installation: needs repair",
        (false, InstallationHealth::Unknown) => "Current installation: not verified",
    }
}

impl Home {
    /// `fresh`: the offers are current; `refreshing`: a status read is under
    /// way after an ordinary change; `pending`: a request is in flight.
    pub fn build(snapshot: &Snapshot, assessment: Option<&ReadinessAssessment>, fresh: bool,
        refreshing: bool, pending: bool) -> Self {
        let busy = snapshot.system.inactive_reason();
        let waiting = pending || !fresh;
        Self {
            heading: "Your plug-ins",
            add: Control::go("Add a plug-in", Importance::Secondary, Destination::Setup),
            freshness: (!fresh).then(|| if refreshing {
                Text::body("Refreshing the listed plug-ins; setup actions wait for current readback.")
            } else {
                Text::caution("Current status is unavailable. These are the last listed plug-ins; setup actions wait for current readback.")
            }),
            next: Next::build(snapshot, assessment, busy, waiting),
            summary: (!snapshot.products.is_empty()).then(|| format!(
                "{} managed plug-ins · open one to manage its setup, updates and settings", snapshot.products.len())),
            plugins: snapshot.products.iter().map(|current| {
                let progress = product::installation_progress(snapshot, current);
                let checked = assessment.and_then(|assessment| assessment.products.iter()
                    .find(|product| product.class_id == current.class_id
                        && product.module_sha256 == current.module_sha256));
                HomePlugin {
                    key: ProductKey::from(current),
                    name: current.name.clone(),
                    manage: Control::go("Manage plug-in", Importance::Secondary,
                        Destination::Product(ProductKey::from(current))),
                    status: fresh.then(|| progress.unwrap_or_else(|| product::status(current).words)),
                    problem: checked.filter(|product| progress.is_none()
                        && product.installation_health == InstallationHealth::ActionRequired)
                        .map(|product| product.reason.clone()),
                    check: checked.map(|product| Disclosure::new("Setup check details", vec![
                        Text::body(installation_label(product)).into(),
                        Text::body(match product.support_qualification {
                            SupportQualification::Verified => "Support qualification: exact profile verified",
                            SupportQualification::Unsupported => "Support qualification: exact profile withdrawn",
                            SupportQualification::NotYetQualified => "Support qualification: not yet qualified",
                        }).into(),
                        Text::body(&product.reason).into(),
                    ])),
                }
            }).collect(),
            computer: ComputerCheck {
                title: "Computer setup and support",
                intro: if !fresh {
                    "Refreshing current status. Actions will return when it finishes."
                } else if assessment.is_none() {
                    "Computer setup has not been checked in this window. This does not change which plug-ins are available in your DAW."
                } else {
                    "This check describes the current computer setup. It does not test a plug-in's sound or saved-project recall."
                },
                check: Control { label: "Check computer setup", importance: Importance::Secondary,
                    enabled: !pending, does: Local::CheckComputer },
                result: assessment.map(|assessment| CheckResult {
                    blockers: assessment.blockers.iter().map(|blocker| blocker.explanation.clone()).collect(),
                    export: offers::row(std::slice::from_ref(&assessment.support_export_action),
                        Importance::Secondary, None, waiting),
                    export_note: "The support report stays on this computer. Nothing is uploaded.",
                    observations: Disclosure::new("Technical observations", [&assessment.platform,
                        &assessment.daw, &assessment.audio, &assessment.graphics, &assessment.runtime]
                        .into_iter().flatten().map(|fact| Detail::from(Text::detail(format!("{}: {} · {} · {:?}",
                            fact.name, fact.value.as_deref().unwrap_or("unknown"), fact.source, fact.certainty))))
                        .collect()),
                }),
            },
        }
    }
}

impl Next {
    /// `waiting`: a request is in flight or status is not current.
    fn build(snapshot: &Snapshot, assessment: Option<&ReadinessAssessment>, busy: Option<&str>,
        waiting: bool) -> Self {
        let primary = |offer| offers::offer(offer, Importance::Primary, busy, waiting);
        if let Some(what_happened) = service::problem(&snapshot.system, snapshot.operation.as_ref()) {
            let recoveries: Vec<_> = snapshot.actions.iter().filter(|offer|
                matches!(offer.action, Action::TransactionReconcile {})).cloned().collect();
            return Self::Service {
                title: "Next: resolve the bridge service problem",
                what_happened,
                offers: offers::row(&recoveries, Importance::Secondary, busy, waiting),
                review: Control::go("Review recovery details", Importance::Secondary, Destination::Diagnostics),
            };
        }
        let unfinished = snapshot.installer_setups.iter().filter(|setup| setup::unfinished(setup, snapshot))
            .min_by_key(|setup| setup::attention_priority(setup));
        if let Some(setup) = unfinished {
            let partial = setup::partial_installation_needs_review(setup, snapshot);
            let name = setup::display_name(setup, snapshot);
            let current = setup.environment.as_deref().map(|environment| (snapshot, environment));
            let review = partial && setup.primary.is_none();
            return Self::Setup {
                title: if let Some(journey) = &setup.journey {
                    format!("{name} · {}", journey::current_label(journey, current))
                } else if partial { format!("Next: review the installation for {name}") }
                    else { format!("Next: continue {name}") },
                status: if partial {
                    "A complete installation was not confirmed. Review this setup before making installation changes. Plug-ins already available in your DAW remain available.".into()
                } else { setup.status.clone() },
                journey: setup.journey.as_ref().map(|journey| Results::build(journey, current, setup.primary.as_ref())),
                primary: setup.primary.as_ref().map(primary),
                open: Control::go(if review { "Review this installation" } else { "Open this setup" },
                    if review { Importance::Primary } else { Importance::Secondary },
                    Destination::Attempt(AttemptKey { installer: setup.installer.clone(),
                        environment: setup.environment.clone() })),
            };
        }
        if let Some(recovering) = snapshot.products.iter().find(|product| product::recovery_pending(product)) {
            return Self::Go {
                title: format!("Next: recover {}", recovering.name),
                detail: recovering.compatibility.as_ref().map(|workflow| workflow.summary.clone()),
                open: Control::go("Open plug-in recovery", Importance::Primary,
                    Destination::Product(ProductKey::from(recovering))),
            };
        }
        if let Some(assessment) = assessment.filter(|assessment|
            assessment.overall_status != ReadinessOutcome::Ready
                && (!assessment.ordered_steps.is_empty() || !assessment.blockers.is_empty())) {
            let step = assessment.ordered_steps.first();
            return Self::Computer {
                note: "Computer setup recommendations reflect the last explicit setup check.",
                title: match step {
                    Some(step) => format!("Next: {}", step.title),
                    None => match assessment.overall_status {
                        ReadinessOutcome::Unsupported => "A required computer capability is unavailable",
                        ReadinessOutcome::Unknown => "A required setup check is incomplete",
                        _ => "Computer setup needs attention",
                    }.into(),
                },
                detail: step.map(|step| step.detail.clone()),
                action: step.and_then(|step| step.action.as_ref()).map(primary),
                blocker: assessment.blockers.first().map(|blocker| blocker.explanation.clone()),
                review: Control::go("Review setup details", Importance::Secondary, Destination::Diagnostics),
            };
        }
        // A plug-in the musician removed from the DAW is not unfinished setup.
        if let Some(waiting) = snapshot.products.iter().find(|product|
            product::status(product).class != StatusClass::Ready
                && !matches!(product.disposition.as_str(), "removed_from_daw" | "uninstalled")) {
            return Self::Go {
                title: format!("Next: set up {} for your DAW", waiting.name),
                detail: Some(product::status(waiting).words.into()),
                open: Control::go("Continue plug-in setup", Importance::Primary,
                    Destination::Product(ProductKey::from(waiting))),
            };
        }
        if snapshot.products.is_empty() {
            return Self::Go {
                title: "Start with a Windows plug-in".into(),
                detail: Some("Choose your vendor's Windows installer and the Linux DAW you use. Complete the vendor's requests when they appear; the manager connects the installed plug-ins to your DAW.".into()),
                open: Control::go("Add your first plug-in", Importance::Primary, Destination::Setup),
            };
        }
        let mut daws = snapshot.products.iter().map(|product| product::product_daw(snapshot, product));
        let daw = daws.next().flatten().filter(|first| daws.all(|other| other == Some(*first)));
        Self::Ready {
            title: match daw {
                Some(daw @ (DawChoice::Bitwig | DawChoice::Reaper)) =>
                    format!("Plug-ins available in {}", product::daw_name(daw)),
                _ => "Plug-ins available in your DAW".into(),
            },
            help: Disclosure::new("Using your plug-ins", vec![Text::body(
                "Available plug-ins appear as native VST3s. Add them to a track in your DAW; if one is missing, rescan VST3 plug-ins.").into()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::Page;
    use crate::preview_states::{self, STATES};
    use crate::screen_model::offers::Availability;

    fn sample(state: &str) -> (Snapshot, Option<ReadinessAssessment>) {
        let snapshot = preview_states::snapshot(Page::Home, Some(state));
        let readiness = preview_states::readiness(Some(state), &snapshot);
        (snapshot, readiness)
    }

    fn home(state: &str, fresh: bool, pending: bool) -> Home {
        let (snapshot, readiness) = sample(state);
        Home::build(&snapshot, readiness.as_ref(), fresh, false, pending)
    }

    /// Every manager offer on Home, with the reasons printed beside it.
    fn row(row: &OfferRow) -> Vec<(&Offer, Vec<&str>)> {
        row.offers.iter().map(|offer| (offer, row.notes.iter().map(String::as_str).collect())).collect()
    }

    fn offers(home: &Home) -> Vec<(&Offer, Vec<&str>)> {
        let mut out: Vec<(&Offer, Vec<&str>)> = Vec::new();
        match &home.next {
            Next::Service { offers, .. } => out.extend(row(offers)),
            Next::Setup { primary, .. } => out.extend(primary.iter().map(|offer|
                (offer, offer.availability.reason().into_iter().collect()))),
            Next::Computer { action, .. } => out.extend(action.iter().map(|offer|
                (offer, offer.availability.reason().into_iter().collect()))),
            Next::Go { .. } | Next::Ready { .. } => {}
        }
        if let Some(result) = &home.computer.result { out.extend(row(&result.export)); }
        out
    }

    fn manager_offers(snapshot: &Snapshot, readiness: Option<&ReadinessAssessment>) -> Vec<Action> {
        let mut offered: Vec<Action> = snapshot.actions.iter().map(|offer| offer.action.clone()).collect();
        offered.extend(snapshot.installer_setups.iter().flat_map(|setup| setup.primary.iter())
            .map(|offer| offer.action.clone()));
        offered.extend(readiness.into_iter().flat_map(|assessment| assessment.ordered_steps.iter()
            .filter_map(|step| step.action.as_ref()).chain([&assessment.support_export_action]))
            .map(|offer| offer.action.clone()));
        offered
    }

    #[test]
    fn sample_states_offer_only_manager_actions_with_every_refusal_visible() {
        let mut checked = 0;
        for state in STATES {
            let (snapshot, readiness) = sample(state);
            let offered = manager_offers(&snapshot, readiness.as_ref());
            for (fresh, pending) in [(true, false), (true, true), (false, false)] {
                let home = Home::build(&snapshot, readiness.as_ref(), fresh, false, pending);
                for (offer, notes) in offers(&home) {
                    assert!(offered.contains(&offer.intent), "{state}: {} is not a manager offer", offer.label);
                    if let Some(reason) = offer.availability.reason() {
                        assert!(notes.contains(&reason), "{state}: {} hides {reason}", offer.label);
                    }
                    if pending || !fresh {
                        assert!(!offer.availability.enabled(), "{state}: {} pressable while waiting", offer.label);
                    }
                    checked += 1;
                }
                assert_eq!(home.computer.check.enabled, !pending, "{state}");
                assert_eq!(home.plugins.len(), snapshot.products.len(), "{state}");
                assert!(home.plugins.iter().all(|row| row.status.is_some() == fresh), "{state}");
            }
        }
        assert!(checked > 20, "only {checked} offers checked");
    }

    #[test]
    fn the_next_step_is_the_most_urgent_one() {
        let title = |state: &str| match home(state, true, false).next {
            Next::Service { title, .. } => format!("service: {title}"),
            Next::Setup { title, .. } => format!("setup: {title}"),
            Next::Computer { title, .. } => format!("computer: {title}"),
            Next::Go { title, .. } => format!("go: {title}"),
            Next::Ready { title, .. } => format!("ready: {title}"),
        };
        for (state, expected) in [
            ("first_use", "go: Start with a Windows plug-in"),
            ("available", "ready: Plug-ins available in your DAW"),
            ("service_problem", "service: Next: resolve the bridge service problem"),
            ("unavailable", "service: Next: resolve the bridge service problem"),
            ("install_progress", "setup: Example Audio Bundle · Connecting plug-ins to your DAW"),
            // The manager paused the bridge for its own installation: not a service problem.
            ("install_paused", "setup: Example Audio Bundle · Connecting plug-ins to your DAW"),
            // Unconfirmed cleanup comes before a journey that needs attention.
            ("install_attention", "setup: Next: continue Example Drums"),
            ("setup_removal", "setup: Next: review the installation for Example Lead installer"),
            ("pb0_step", "computer: Next: Download Windows plug-in support"),
            ("pb0_unsupported", "computer: A required computer capability is unavailable"),
            ("populated", "go: Next: set up Serum 2 FX for your DAW"),
        ] {
            assert_eq!(title(state), expected, "{state}");
        }
        let Next::Service { what_happened, offers, review, .. } = home("service_problem", true, false).next
            else { panic!("a service problem") };
        assert_eq!(what_happened, "A previous setup change has not finished.");
        assert!(matches!(offers.offers[0].intent, Action::TransactionReconcile {}));
        assert_eq!(review.does, Local::Go(Destination::Diagnostics));
        let Next::Setup { primary, open, journey, .. } = home("setup_removal", true, false).next
            else { panic!("a setup step") };
        assert!(primary.is_none() && journey.is_none());
        assert_eq!((open.label, open.importance), ("Review this installation", Importance::Primary));
        let Next::Computer { action, .. } = home("pb0_step", true, false).next else { panic!("a computer step") };
        let action = action.expect("the step's own offer");
        assert_eq!(action.label, "Download Windows plug-in support (310 MB)");
        let Next::Computer { action, .. } = home("pb0_step", false, false).next else { panic!("a computer step") };
        assert_eq!(action.unwrap().availability, Availability::Waiting);
    }

    #[test]
    fn plug_in_rows_say_progress_and_withhold_status_without_current_readback() {
        let progress = home("install_progress", true, false);
        let statuses: Vec<_> = progress.plugins.iter().filter_map(|row| row.status).collect();
        for expected in ["Connecting to your DAW", "Preparing its DAW connection", "Checking this plug-in",
            "Waiting to connect to your DAW"] {
            assert!(statuses.contains(&expected), "{expected}: {statuses:?}");
        }
        let stale = home("install_progress", false, false);
        assert!(stale.plugins.iter().all(|row| row.status.is_none()));
        assert_eq!(stale.freshness, Some(Text::caution(
            "Current status is unavailable. These are the last listed plug-ins; setup actions wait for current readback.")));
        let (snapshot, readiness) = sample("pb0_action");
        let checked = Home::build(&snapshot, readiness.as_ref(), true, false, false);
        assert!(checked.plugins.iter().all(|row| row.check.is_some()));
        let result = checked.computer.result.expect("a setup check");
        assert_eq!(result.blockers, ["Bitwig audio callback maximum still needs confirmation."]);
        assert_eq!(result.observations.title, "Technical observations");
    }
}

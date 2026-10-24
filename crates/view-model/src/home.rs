//! Home: the one next step, then each plug-in at a glance, then the computer
//! setup check. Built from the manager's current overview and the last
//! explicit setup check; every control is a manager offer or opens a page.
use super::journey::Results;
use super::offers::{self, Importance, Offer, OfferRow};
use super::{product, Control, Destination, Detail, Disclosure, Local,
    ProductKey, StatusClass, Text};
use crate::model::{InstallationHealth, ReadinessAssessment, ReadinessOutcome,
    ReadinessProduct, Snapshot, SupportQualification};

#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
pub enum Next {
    /// The bridge service itself needs the musician first.
    Service {
        title: String,
        what_happened: String,
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
    /// One exact plug-in has an unfinished recovery, independent of siblings.
    Recovery {
        title: String,
        detail: Option<String>,
        open: Control,
    },
    /// One plug-in to set up, or the first one to add.
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
#[derive(Clone, Debug, PartialEq)]
pub struct HomePlugin {
    pub key: ProductKey,
    pub name: String,
    pub subtitle: String,
    pub activity: String,
    pub status_class: StatusClass,
    pub manage: Control,
    /// Current availability or install progress, while status is current.
    pub status: Option<&'static str>,
    /// What the last setup check found wrong with its installation.
    pub problem: Option<String>,
    pub check: Option<Disclosure>,
}

/// The computer setup check and the support report.
#[derive(Clone, Debug, PartialEq)]
pub struct ComputerCheck {
    pub title: &'static str,
    pub intro: &'static str,
    pub check: Control,
    pub result: Option<CheckResult>,
}

#[derive(Clone, Debug, PartialEq)]
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
        Self::build_observed(snapshot, assessment, fresh, refreshing, pending, crate::diagnostics::Section::NotRequested)
    }
    pub fn build_observed(snapshot: &Snapshot, assessment: Option<&ReadinessAssessment>, fresh: bool,
        refreshing: bool, pending: bool, activity: crate::diagnostics::Section<'_, &crate::model::Activity>) -> Self {
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
                    subtitle: format!("{} · {}", product::vendor(current), product::role(current)),
                    activity: product::product_activity(activity, current).short(),
                    status_class: if fresh { product::status(current).class } else { StatusClass::Unknown },
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
        let Some(presentation) = snapshot.presentation.observed() else {
            return Self::Go { title: "Current next step unavailable".into(),
                detail: Some("Refresh current status to read the service's next step.".into()),
                open: Control::go("Review service status", Importance::Secondary, Destination::Diagnostics) };
        };
        if let Some(item) = presentation.attention.first() {
            let destination = super::activity::attention_destination(&item.target);
            let offered = item.action.as_ref().and_then(|action| snapshot.actions.iter()
                .chain(snapshot.installer_setups.iter().flat_map(|s| s.primary.iter().chain(&s.secondary)))
                .chain(snapshot.products.iter().flat_map(|p| &p.actions))
                .find(|offer| &offer.action == action));
            return match item.purpose {
                crate::model::AttentionPurpose::Service => Self::Service {
                    title: item.title.clone(), what_happened: item.detail.clone(),
                    offers: offers::row(&offered.cloned().into_iter().collect::<Vec<_>>(), Importance::Secondary, busy, waiting),
                    review: Control::go("Review recovery details", Importance::Secondary, destination),
                },
                crate::model::AttentionPurpose::Installation => Self::Setup {
                    title: item.title.clone(), status: item.detail.clone(), journey: None,
                    primary: offered.map(primary),
                    open: Control::go(if matches!(item.target, crate::model::AttentionTarget::Product { .. }) {
                        "Open this plug-in" } else { "Open this setup" }, Importance::Secondary, destination),
                },
                crate::model::AttentionPurpose::Recovery => Self::Recovery {
                    title: item.title.clone(), detail: Some(item.detail.clone()),
                    open: Control::go("Open plug-in recovery", Importance::Primary, destination),
                },
                crate::model::AttentionPurpose::Connection => Self::Go {
                    title: item.title.clone(), detail: Some(item.detail.clone()),
                    open: Control::go("Continue plug-in setup", Importance::Primary, destination),
                },
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
        if snapshot.products.is_empty() {
            return Self::Go {
                title: "Start with a Windows plug-in".into(),
                detail: Some("Choose your vendor's Windows installer and the Linux DAW you use. Complete the vendor's requests when they appear; the manager connects the installed plug-ins to your DAW.".into()),
                open: Control::go("Add your first plug-in", Importance::Primary, Destination::Setup),
            };
        }
        Self::Ready {
            title: presentation.settled_message.clone(),
            help: Disclosure::new("Using your plug-ins", vec![Text::body(
                "Available plug-ins appear as native VST3s. Add them to a track in your DAW; if one is missing, rescan VST3 plug-ins.").into()]),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Action;
    use super::*;
    use crate::window::Page;
    use crate::samples::{self, STATES};

    fn sample(state: &str) -> (Snapshot, Option<ReadinessAssessment>) {
        let snapshot = samples::snapshot(Page::Home, Some(state));
        let readiness = samples::readiness(Some(state), &snapshot);
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
            Next::Recovery { .. } | Next::Go { .. } | Next::Ready { .. } => {}
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
    fn the_next_step_follows_owner_order_and_keeps_exact_offer_refusals() {
        use crate::model::{Observation as O, Attention, AttentionPurpose as P, AttentionTarget as T};
        let (mut snapshot, _) = sample("service_problem");
        let offered = snapshot.actions.iter().find(|offer| matches!(offer.action, Action::TransactionReconcile {})).unwrap().clone();
        let product = crate::model::ProductIdentity::from(&snapshot.products[0]);
        let O::Observed(presentation) = &mut snapshot.presentation else { panic!() };
        presentation.attention = vec![Attention { purpose: P::Service, target: T::Service, title: "Owner service priority".into(),
            detail: "Owner's exact reason".into(), action: Some(offered.action.clone()) },
            Attention { purpose: P::Recovery, target: T::Product { product: product.clone() }, title: "Owner recovery priority".into(),
                detail: "Exact recovery".into(), action: None }];
        let Next::Service { title, what_happened, offers, .. } = Home::build(&snapshot, None, true, false, false).next else { panic!() };
        assert_eq!(title, "Owner service priority"); assert_eq!(what_happened, "Owner's exact reason");
        assert_eq!(offers.offers[0].intent, offered.action);
        let O::Observed(presentation) = &mut snapshot.presentation else { panic!() };
        presentation.attention.reverse();
        let Next::Recovery { open, .. } = Home::build(&snapshot, None, true, false, false).next else { panic!() };
        assert_eq!(open.does, Local::Go(Destination::Product(ProductKey { environment: product.environment,
            module_sha256: product.module_sha256, class_id: product.class_id })));
        snapshot.presentation = O::Unavailable("Owner composition unavailable".into());
        assert!(matches!(Home::build(&snapshot, None, true, false, false).next, Next::Go { open, .. }
            if open.does == Local::Go(Destination::Diagnostics)));
    }

    #[test]
    fn published_product_recovery_is_distinct_from_routine_setup_navigation() {
        for phase in [crate::model::CompatibilityPhase::AwaitingRetirement,
            crate::model::CompatibilityPhase::PublicationNeedsAttention] {
            let (mut snapshot, _) = sample("available");
            snapshot.installer_setups.clear();
            let product = &mut snapshot.products[0];
            product.active_revision = Some(1);
            product.compatibility = Some(crate::model::CompatibilityWorkflow {
                phase, summary: "Recorded problem needs recovery".into(),
                established: vec![], remaining: vec![], current_inspection: None,
                current_candidate: None, primary: None, alternatives: vec![],
            });
            let expected = ProductKey::from(&*product);
            let name = product.name.clone();
            samples::presentation(&mut snapshot, "available");
            let home = Home::build(&snapshot, None, true, false, false);
            let Next::Recovery { title, detail, open } = home.next else { panic!("exact recovery") };
            assert_eq!(title, format!("Recover {name}"));
            assert_eq!(detail.as_deref(), Some("Recorded problem needs recovery"));
            assert_eq!(open.does, Local::Go(Destination::Product(expected)));
            assert!(snapshot.products[0].active_revision.is_some(), "publication does not hide recovery");
            assert!(home.plugins.len() > 1, "healthy siblings remain separate library rows");
        }
        assert!(matches!(home("populated", true, false).next, Next::Go { .. }));
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

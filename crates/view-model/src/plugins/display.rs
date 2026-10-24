//! Display choices and repair steps supplied by the owning service. This
//! adapter formats observations and rejoins references to exact current offers.
use crate::{model::{self, Action, AvailableAction, Observation, Product},
    offers::{self, Importance, OfferRow}, Detail, Disclosure, Text};

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayView {
    pub notice: Option<Text>,
    pub modes: Vec<DisplayMode>,
    pub repairs: Vec<RepairStep>,
    pub provenance: Option<Disclosure>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayMode {
    pub label: String,
    pub posture: String,
    pub result: Text,
    pub result_record: Option<Disclosure>,
    pub prepare: OfferRow,
    pub trials: Vec<DisplayTrial>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DisplayTrial {
    pub posture: String,
    pub current: bool,
    pub evidence: Text,
    pub result: Text,
    pub actions: OfferRow,
    pub identity: Disclosure,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RepairStep {
    pub kind: model::RepairStepKind,
    pub label: String,
    pub detail: String,
    pub actions: OfferRow,
}

pub(super) fn references(product: &Product) -> Vec<Action> {
    let Some(display) = product.display.observed() else { return vec![]; };
    display.modes.iter().flat_map(|mode| mode.prepare.iter().chain(mode.trials.iter().flat_map(|trial| &trial.actions)))
        .chain(display.repair.steps.iter().filter_map(|step| step.action.as_ref())).cloned().collect()
}

/// Each local page receives the same exact current offers. Navigation never
/// substitutes a prose reference for the control the musician needs there.
pub(super) fn build(product: &Product, busy: Option<&str>, pending: bool) -> DisplayView {
    let Observation::Observed(display) = &product.display else {
        return DisplayView { notice: Some(Text::detail("Display choices and repair steps are unavailable. Refresh the plug-in controls to check again.")),
            modes: vec![], repairs: vec![], provenance: None };
    };
    let mut resolver = Resolver { product, busy, pending };
    let modes = display.modes.iter().map(|mode| DisplayMode {
        label: mode.label.clone(),
        posture: match (mode.selected, mode.suggested) {
            (true, true) => "Current display setting · Suggested",
            (true, false) => "Current display setting",
            (false, true) => "Suggested",
            (false, false) => "Another display setting",
        }.into(),
        result: result(&mode.last_result),
        result_record: result_record(&mode.last_result),
        prepare: if mode.prepare.is_some() { resolver.actions(mode.prepare.iter(), None) }
            else { OfferRow::default() },
        trials: mode.trials.iter().map(|trial| DisplayTrial {
            current: trial.selected || trial.current_inputs,
            posture: match (trial.selected, trial.current_inputs) {
                (true, true) => "Current display setting",
                (true, false) => "Current display setting · needs checking after setup changes",
                (false, true) => "Prepared setting · not selected",
                (false, false) => "Needs checking after setup changes",
            }.into(),
            evidence: match trial.evidence {
                model::DisplayEvidenceState::FailureRetained => Text::caution("An earlier editor problem remains recorded for these settings; a later passing check does not clear it."),
                model::DisplayEvidenceState::PassingObservation => Text::body("The editor passed a check with these settings."),
                model::DisplayEvidenceState::NotEstablished => Text::detail("The editor has not passed a check with these settings."),
            },
            result: result(&trial.last_result),
            actions: resolver.actions(trial.actions.iter(), None),
            identity: Disclosure::new("Exact trial and settings", {
                let mut details = vec![Text::detail(format!("Candidate: {}", trial.candidate)).into(),
                    crate::record(&serde_json::json!({"settings":trial.settings,"predecessor":trial.predecessor,
                        "previous_publication":trial.previous_publication}))];
                if let Some(record) = result_record(&trial.last_result) { details.push(Detail::Disclosure(record)); }
                details
            }),
        }).collect(),
    }).collect();
    let repairs = display.repair.steps.iter().map(|step| RepairStep {
        kind: step.kind,
        label: step.label.clone(), detail: step.detail.clone(),
        actions: resolver.actions(step.action.iter(), step.unavailable_reason.as_deref()),
    }).collect();
    let mut evidence: Vec<Detail> = vec![Text::detail(format!("Profile: {} · revision {}",
        display.repair.profile_id, display.repair.profile_revision)).into(),
        Text::detail(format!("Observed candidate: {}", display.observed_candidate)).into()];
    evidence.extend(display.repair.evidence.iter().map(|line| Text::detail(line).into()));
    DisplayView { notice: None, modes, repairs, provenance: Some(Disclosure::new("About these repair steps", evidence)) }
}

fn result(result: &Observation<Option<model::DisplayResult>>) -> Text {
    let value = match result {
        Observation::Unavailable(reason) => return Text::detail(format!("Last editor check unavailable: {reason}")),
        Observation::Observed(None) => return Text::detail("Not checked yet."),
        Observation::Observed(Some(value)) => value,
    };
    let status = match value.status {
        model::DisplayTestStatus::Passed => "Passed",
        model::DisplayTestStatus::Failed => "Failed",
        model::DisplayTestStatus::NotTested => "Not tested",
        model::DisplayTestStatus::Unavailable => "Unavailable",
        model::DisplayTestStatus::NotApplicable => "Not applicable",
    };
    let witness = match value.witness {
        model::DisplayWitness::Operator => "your report",
        model::DisplayWitness::Machine => "automated check",
        model::DisplayWitness::Generated => "generated regression",
    };
    let words = format!("Last editor check: {status} · {witness} · {}", value.detail);
    if value.status == model::DisplayTestStatus::Failed { Text::caution(words) } else { Text::detail(words) }
}

fn result_record(result: &Observation<Option<model::DisplayResult>>) -> Option<Disclosure> {
    let Observation::Observed(Some(result)) = result else { return None; };
    Some(Disclosure::new("Exact editor result", vec![crate::record(&serde_json::to_value(result).unwrap_or_default())]))
}

struct Resolver<'a> { product: &'a Product, busy: Option<&'a str>, pending: bool }
impl Resolver<'_> {
    fn actions<'a>(&mut self, references: impl Iterator<Item = &'a Action>, unavailable: Option<&str>) -> OfferRow {
        let mut actions: Vec<AvailableAction> = vec![];
        let mut notes = vec![];
        for action in references {
            let current = self.product.actions.iter().chain(self.product.compatibility.iter()
                .flat_map(|workflow| workflow.primary.iter().chain(&workflow.alternatives)))
                .find(|offer| &offer.action == action);
            let Some(current) = current else {
                notes.push("This exact step is no longer offered. Refresh the plug-in controls.".into());
                continue;
            };
            if actions.iter().any(|offer| &offer.action == action) { continue; }
            actions.push(current.clone());
        }
        let mut row = offers::row(&actions, Importance::Secondary, self.busy, self.pending);
        row.notes.extend(notes);
        if let Some(reason) = unavailable { row.notes.push(reason.into()); }
        if row.offers.is_empty() && row.notes.is_empty() { row.notes.push("This step is unavailable right now.".into()); }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> crate::model::Snapshot {
        let mut snapshot = crate::samples::base();
        crate::samples::display_fixture(&mut snapshot);
        snapshot
    }
    #[test]
    fn display_choices_keep_owner_order_results_and_exact_current_actions() {
        let mut snapshot = fixture();
        let p = &mut snapshot.products[0];
        // Old untyped display fields are not an alternate mode list or result.
        p.details = serde_json::json!({"graphics":{"requested":"fastest supported renderer"},
            "configuration":{"assessment":{"editor":{"status":"working"}}}});
        let view = build(p, None, false);
        assert_eq!(view.modes.iter().map(|mode| mode.label.as_str()).collect::<Vec<_>>(),
            ["Runtime defaults", "Wine Direct3D 11"]);
        assert_eq!(view.modes[0].posture, "Current display setting");
        assert!(view.modes[0].prepare.offers.is_empty() && view.modes[0].prepare.notes.is_empty(),
            "a mode with no preparation control is not a failed step");
        assert_eq!(view.modes[1].posture, "Suggested");
        assert!(view.modes[0].result.words.contains("Failed · your report"));
        assert!(view.modes[1].result.words.contains("Passed · automated check"));
        assert!(view.modes[1].trials[0].evidence.words.starts_with("An earlier editor problem remains recorded"),
            "a later pass cannot hide retained failure");
        let model::Observation::Observed(display) = &p.display else { panic!() };
        assert_eq!(view.modes[1].prepare.offers[0].pressed(), display.modes[1].prepare);
        assert_eq!(view.modes[1].trials[0].actions.offers[0].pressed(), Some(display.modes[1].trials[0].actions[0].clone()));
        assert_eq!(view.repairs.iter().map(|step| step.label.as_str()).collect::<Vec<_>>(),
            display.repair.steps.iter().map(|step| step.label.as_str()).collect::<Vec<_>>());
        assert_eq!(view.repairs[3].actions.offers[0].availability,
            offers::Availability::Refused("Close this exact plug-in window before preparing its accessibility trial.".into()));
        assert!(view.repairs[5].actions.offers.is_empty());
        assert!(view.repairs[5].actions.notes[0].contains("No previous settings"));
        assert_eq!(view.repairs[2].actions.offers[0].intent, view.modes[1].prepare.offers[0].intent,
            "the repair page keeps the exact control available on its own");
        assert!(!format!("{view:?}").contains("fastest supported renderer"));
    }

    #[test]
    fn references_cannot_survive_offer_replacement_or_override_its_disabled_reason() {
        let mut snapshot = fixture();
        let p = &mut snapshot.products[0];
        let model::Observation::Observed(display) = &p.display else { panic!() };
        let offered = display.modes[1].prepare.clone().unwrap();
        let index = p.actions.iter().position(|offer| offer.action == offered).unwrap();
        let pending = build(p, None, true);
        assert_eq!(pending.modes[1].prepare.offers[0].availability, offers::Availability::Waiting);
        assert!(pending.modes[1].prepare.offers[0].pressed().is_none());
        assert!(matches!(pending.repairs[3].actions.offers[0].availability, offers::Availability::Refused(_)));
        if let Action::CandidateSettingsPrepare {expected_current:Some(current), ..} = &mut p.actions[index].action {
            current.sha256 = "ef".repeat(32);
        } else { panic!() }
        let changed = build(p, None, false);
        assert!(changed.modes[1].prepare.offers.is_empty());
        assert!(changed.modes[1].prepare.notes.iter().any(|note| note.contains("no longer offered")));
        p.actions.clear();
        let gone = build(p, None, false);
        assert!(gone.modes.iter().all(|mode| mode.prepare.offers.is_empty()
            && mode.trials.iter().all(|trial| trial.actions.offers.is_empty())));
        p.display = Observation::Unavailable("No exact candidate".into());
        let missing = build(p, None, false);
        assert!(missing.modes.is_empty() && missing.repairs.is_empty() && missing.notice.is_some());
    }
}

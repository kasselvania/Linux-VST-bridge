//! Display facts reuse exact preparation records. The recipe only orders the
//! existing mechanisms; every control is joined to a current owner offer.
use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe { schema: u32, steps: Vec<Step> }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step { kind: ui::RepairStepKind, label: String, detail: String }

fn effective(c: &prep::Candidate) -> ui::EffectiveAccessibility {
    match c.profile.capabilities.accessibility {
        profiles::Accessibility::WindowsDefault => ui::EffectiveAccessibility::WindowsDefault,
        profiles::Accessibility::DisabledForVendorProcess => ui::EffectiveAccessibility::DisabledForHost,
    }
}

fn same_inputs(a: &prep::Candidate, b: &prep::Candidate) -> bool {
    a.selection == b.selection && a.host == b.host && a.source_manifest == b.source_manifest
        && a.native == b.native && a.recipe_sha256 == b.recipe_sha256
        && a.preparation_basis == b.preparation_basis
}

fn editor_result(row: Option<&prep::CandidateView>)
    -> (ui::Observation<Option<ui::DisplayResult>>, ui::DisplayEvidenceState) {
    let Some(row) = row else {
        return (ui::Observation::Unavailable("The check results for these settings could not be read.".into()),
            ui::DisplayEvidenceState::NotEstablished);
    };
    let evidence: Vec<_> = row.evidence.iter()
        .filter(|o| o.candidate == row.id && o.area == prep::Area::Editor).collect();
    let last = evidence.iter().max_by_key(|o| (o.ordinal, &o.operation));
    let state = if evidence.iter().any(|o| o.status == prep::TestStatus::Failed) {
        ui::DisplayEvidenceState::FailureRetained
    } else if last.is_some_and(|o| o.status == prep::TestStatus::Passed
        && o.witness != prep::Witness::GeneratedRegression) {
        ui::DisplayEvidenceState::PassingObservation
    } else { ui::DisplayEvidenceState::NotEstablished };
    let result = last.map(|o| ui::DisplayResult {
        candidate: row.id.clone(), ordinal: o.ordinal, recorded_at: o.recorded_at,
        status: match o.status {
            prep::TestStatus::Passed => ui::DisplayTestStatus::Passed,
            prep::TestStatus::Failed => ui::DisplayTestStatus::Failed,
            prep::TestStatus::NotTested => ui::DisplayTestStatus::NotTested,
            prep::TestStatus::Unavailable => ui::DisplayTestStatus::Unavailable,
            prep::TestStatus::NotApplicable => ui::DisplayTestStatus::NotApplicable,
        },
        witness: match o.witness {
            prep::Witness::OperatorObservation => ui::DisplayWitness::Operator,
            prep::Witness::MachineObservation => ui::DisplayWitness::Machine,
            prep::Witness::GeneratedRegression => ui::DisplayWitness::Generated,
        }, detail: o.detail.clone(),
    });
    (ui::Observation::Observed(result), state)
}

pub(super) fn compose(c: &prep::Candidate, view: &prep::View,
    candidates: &[prep::Candidate], configuration: &serde_json::Value) -> Result<ui::DisplayConfiguration> {
    let recipe: Recipe = serde_json::from_str(include_str!("../../../../../compatibility/editor-repair.json"))?;
    require(recipe.schema == 1, "editor_repair_recipe_schema")?;
    let selected = view.candidates.iter().find(|row|
        matches!(row.publication.as_str(), "ordinary" | "experimental"))
        .map(|row| row.id.clone());
    let settings_base = selected.as_ref().and_then(|id| candidates.iter()
        .find(|candidate| candidate.id().is_ok_and(|value| &value == id))).unwrap_or(c);
    let settings = ui::DisplaySettings {
        candidate: settings_base.id()?,
        expected_current: selected.as_ref().and(view.current_revision.as_ref())
            .map(|current| ui::PublicationIdentity {id:current.id.clone(),sha256:current.sha256.clone()}),
        current: prep::configuration::settings(settings_base),
        effective_accessibility: effective(settings_base), choices: vec![],
    };
    let suggested = configuration["assessment"]["recommendation"]["requirement"] == "wine_d3d11";
    let mut modes = Vec::new();
    for (backend, label) in [(None, "Runtime defaults"),
        (Some(ui::GraphicsBackend::WineD3d11), "Wine Direct3D 11")] {
        let mut trials = Vec::new();
        for candidate in candidates.iter().filter(|candidate| same_inputs(c, candidate)
            && candidate.profile.capabilities.graphics == backend) {
            let id = candidate.id()?;
            let row = view.candidates.iter().find(|row| row.id == id);
            let (last_result, evidence) = editor_result(row);
            trials.push(ui::DisplayTrial { selected: selected.as_ref() == Some(&id),
                current_inputs: row.is_some_and(|row| row.current_inputs),
                candidate: id,
                predecessor: candidate.settings_trial.as_ref().map(|t| t.predecessor.clone()),
                previous_publication: candidate.settings_trial.as_ref().and_then(|t| t.baseline.as_ref())
                    .map(|b| ui::PublicationIdentity {id:b.id.clone(),sha256:b.sha256.clone()}),
                settings: prep::configuration::settings(candidate),
                last_result, evidence, actions: Vec::new() });
        }
        trials.sort_by(|a,b| a.candidate.cmp(&b.candidate));
        let last_result = if trials.iter().any(|trial| matches!(trial.last_result, ui::Observation::Unavailable(_))) {
            ui::Observation::Unavailable("Some check results for these settings could not be read.".into())
        } else {
            ui::Observation::Observed(trials.iter().filter_map(|trial| match &trial.last_result {
                ui::Observation::Observed(Some(result)) => Some(result), _ => None,
            }).max_by_key(|r| (r.recorded_at, r.ordinal, &r.candidate)).cloned())
        };
        modes.push(ui::DisplayModeChoice { backend, label: label.into(),
            selected: trials.iter().any(|trial| trial.selected),
            suggested: suggested && backend == Some(ui::GraphicsBackend::WineD3d11),
            prepare: None, last_result, trials });
    }
    let mut steps: Vec<_> = recipe.steps.into_iter().map(|step| ui::RepairStep {
        kind: step.kind, label: step.label, detail: step.detail,
        action: None, unavailable_reason: None,
    }).collect();
    if let Some(step) = steps.iter_mut().find(|s| s.kind == ui::RepairStepKind::ProfileAccessibility) {
        step.label = match settings.effective_accessibility {
            ui::EffectiveAccessibility::WindowsDefault => "Turn off Windows accessibility support",
            ui::EffectiveAccessibility::DisabledForHost => "Use Windows accessibility defaults",
        }.into();
        step.detail.push_str(match settings_base.profile.capabilities.accessibility {
            profiles::Accessibility::DisabledForVendorProcess =>
                " The current settings disable Windows accessibility for this plug-in. Linux input settings are unchanged.",
            profiles::Accessibility::WindowsDefault =>
                " The current settings use the Windows accessibility defaults.",
        });
    }
    Ok(ui::DisplayConfiguration { observed_candidate: c.id()?, selected_candidate: selected, modes, settings,
        repair: ui::RepairLadder { profile_id: c.profile.id.clone(), profile_revision: c.profile.revision,
            evidence: c.profile.evidence.clone(), steps } })
}

fn candidate_action(action: &ui::Action, id: &str) -> bool {
    match action {
        ui::Action::CandidateGraphicsAssess {candidate}
        | ui::Action::ExperimentalDisable {candidate}
        | ui::Action::ExperimentalEnable {candidate}
        | ui::Action::ExperimentalReplace {candidate,..}
        | ui::Action::CompatibilityPublishTest {candidate,..}
        | ui::Action::CompatibilityResult {candidate,..}
        | ui::Action::CandidateReview {candidate,..} => candidate == id,
        _ => false,
    }
}

/// May be called again after final offer projection (including detached Open).
/// Nothing here grants an action absent from that final owner offer set.
pub(super) fn bind(product: &mut ui::Product) {
    let offers: Vec<_> = product.actions.iter().chain(product.compatibility.iter()
        .flat_map(|w| w.primary.iter().chain(w.alternatives.iter()))).cloned().collect();
    let ui::Observation::Observed(display) = &mut product.display else { return };
    display.settings.choices = offers.iter().filter_map(|offer| {
        let ui::Action::CandidateSettingsTest {candidate,settings,expected_current} = &offer.action else { return None };
        if candidate != &display.settings.candidate || expected_current != &display.settings.expected_current { return None; }
        let effective_accessibility = match settings.accessibility {
            ui::AccessibilityChoice::WindowsDefault => ui::EffectiveAccessibility::WindowsDefault,
            ui::AccessibilityChoice::DisabledForHost => ui::EffectiveAccessibility::DisabledForHost,
            // An unresolved default cannot stand in for an explicit choice.
            ui::AccessibilityChoice::ProfileDefault => return None,
        };
        Some(ui::DisplaySettingsChoice {settings:settings.clone(), effective_accessibility, action:offer.action.clone()})
    }).collect();
    let inputs: Vec<_> = display.modes.iter().flat_map(|mode| &mode.trials)
        .map(|trial| (trial.candidate.clone(), trial.settings.clone())).collect();
    for mode in &mut display.modes {
        mode.prepare = offers.iter().find(|offer| matches!(&offer.action,
            ui::Action::CandidateSettingsPrepare {candidate,settings,..} if settings.graphics == mode.backend
                && inputs.iter().find(|(id,_)| id == candidate).map(|(_,settings)|settings).is_some_and(|before|
                    settings.graphics != before.graphics && settings.accessibility == before.accessibility)))
            .map(|offer| offer.action.clone());
        for trial in &mut mode.trials {
            trial.actions = offers.iter().filter(|offer| candidate_action(&offer.action, &trial.candidate))
                .map(|offer| offer.action.clone()).collect();
        }
    }
    for step in &mut display.repair.steps {
        use ui::RepairStepKind as K;
        let offer = offers.iter().find(|offer| match (&step.kind, &offer.action) {
            (K::OpenStandalone, ui::Action::PluginOpen {product: identity,..}) =>
                identity.environment == product.environment && identity.module_sha256 == product.module_sha256
                    && identity.class_id.eq_ignore_ascii_case(&product.class_id),
            (K::AssessDisplay, ui::Action::CandidateGraphicsAssess {candidate}) =>
                candidate == &display.observed_candidate,
            (K::TryDisplayMode, action) => display.modes.iter().any(|mode| mode.prepare.as_ref() == Some(action)),
            (K::ProfileAccessibility, action) => display.settings.choices.iter().any(|choice|
                &choice.action == action && choice.settings.graphics == display.settings.current.graphics
                    && choice.effective_accessibility != display.settings.effective_accessibility),
            (K::RestorePrevious, ui::Action::ExperimentalDisable {candidate}) =>
                display.selected_candidate.as_ref() == Some(candidate)
                    && display.modes.iter().flat_map(|m| &m.trials).any(|trial|
                        trial.candidate == *candidate && trial.previous_publication.is_some()),
            (K::RestorePrevious, ui::Action::OrdinaryRollback {class_id,publication}) =>
                class_id.eq_ignore_ascii_case(&product.class_id)
                    && display.modes.iter().flat_map(|m| &m.trials).any(|trial| trial.selected
                        && trial.previous_publication.as_ref().is_some_and(|prior| prior.id == *publication)),
            (K::ReportProblem, ui::Action::CompatibilityResult {result: ui::TestResultKind::Problem {..},..}
                | ui::Action::CompatibilityFinishResult {..}) => true,
            _ => false,
        });
        step.action = offer.map(|offer| offer.action.clone());
        step.unavailable_reason = offer.and_then(|offer| offer.disabled_reason.clone()).or_else(|| {
            offer.is_none().then(|| match step.kind {
                K::OpenStandalone => "This plug-in cannot be opened on its own right now.",
                K::AssessDisplay => "A display check is not available right now.",
                K::TryDisplayMode => "No other display mode is available to prepare.",
                K::ProfileAccessibility => "Changing Windows accessibility for this plug-in is not available right now.",
                K::RepairInstallation => "Choose the maker's installer to run in this same setup.",
                K::RestorePrevious => "No previous settings are available to restore.",
                K::ReportProblem => "Recording a problem for these settings is not available right now.",
            }.into())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::owner::preparation_cli::tests::{projection_fixture, projection_product, projection_software};

    fn read(m: &Manager, c: &prep::Candidate) -> prep::View {
        prep::view(m, &c.selection, &c.host, &c.source_manifest.sha256).unwrap()
    }
    fn facts(p: &ui::Product) -> &ui::DisplayConfiguration {
        match &p.display { ui::Observation::Observed(value) => value, other => panic!("{other:?}") }
    }
    fn projected(m: &Manager, c: &prep::Candidate) -> ui::Product {
        let mut products = vec![projection_product(c)];
        crate::owner::preparation_cli::project(m, &projection_software(c), &mut products, None).unwrap();
        products.remove(0)
    }

    #[test]
    fn display_results_keep_exact_candidate_failure_precedence_and_generated_provenance() {
        let (f, c) = projection_fixture();
        prep::record_candidate(&f.m, &c).unwrap();
        for status in [prep::TestStatus::Failed, prep::TestStatus::Passed] {
            prep::record_observation(&f.m, &c, &random_id().unwrap(), prep::Area::Editor,
                status, "Exact editor observation").unwrap();
        }
        let view = read(&f.m, &c);
        let row = view.candidates.iter().find(|r| r.id == c.id().unwrap()).unwrap();
        let (last, evidence) = editor_result(Some(row));
        assert_eq!(evidence, ui::DisplayEvidenceState::FailureRetained);
        assert!(matches!(last, ui::Observation::Observed(Some(ui::DisplayResult {
            status:ui::DisplayTestStatus::Passed, witness:ui::DisplayWitness::Operator, .. }))));
        assert!(prep::unmet(&row.evidence, &c.profile.role).iter().any(|s| s.starts_with("Editor:")));
        let mut generated = row.clone();
        generated.evidence.retain(|o| o.status == prep::TestStatus::Passed);
        generated.evidence[0].witness = prep::Witness::GeneratedRegression;
        let (last, state) = editor_result(Some(&generated));
        assert_eq!(state, ui::DisplayEvidenceState::NotEstablished);
        assert!(matches!(last, ui::Observation::Observed(Some(ui::DisplayResult {
            witness:ui::DisplayWitness::Generated, .. }))));
        generated.evidence[0].witness = prep::Witness::MachineObservation;
        assert_eq!(editor_result(Some(&generated)).1, ui::DisplayEvidenceState::PassingObservation);
        generated.evidence[0].candidate = "ff".repeat(32);
        assert_eq!(editor_result(Some(&generated)).0, ui::Observation::Observed(None));
        assert!(matches!(editor_result(None).0, ui::Observation::Unavailable(_)));
    }

    #[test]
    fn display_trials_do_not_borrow_results_from_other_modes_or_builds() {
        let (f, base) = projection_fixture();
        prep::record_candidate(&f.m, &base).unwrap();
        prep::record_observation(&f.m, &base, &random_id().unwrap(), prep::Area::Editor,
            prep::TestStatus::Failed, "Runtime defaults editor did not respond").unwrap();
        let trial = prep::configuration::prepare(&f.m, &base,
            Some(ui::GraphicsBackend::WineD3d11), None).unwrap();
        let view = read(&f.m, &base);
        let mut different_build = trial.clone();
        different_build.native.artifact.sha256 = "fe".repeat(32);
        let mut different_setup = trial.clone();
        different_setup.selection.environment.revision += 1;
        let value = compose(&base, &view, &[base.clone(), trial.clone(), different_build, different_setup],
            &serde_json::json!({"assessment":{"recommendation":{"requirement":"wine_d3d11"}}})).unwrap();
        let defaults = value.modes.iter().find(|m| m.backend.is_none()).unwrap();
        assert!(matches!(&defaults.last_result, ui::Observation::Observed(Some(r))
            if r.candidate == base.id().unwrap() && r.status == ui::DisplayTestStatus::Failed));
        let alternative = value.modes.iter().find(|m| m.backend.is_some()).unwrap();
        assert!(alternative.suggested);
        assert!(!alternative.selected);
        assert_eq!(alternative.trials.len(), 1);
        assert_eq!(alternative.trials[0].candidate, trial.id().unwrap());
        assert_eq!(alternative.last_result, ui::Observation::Observed(None));
        assert_eq!(alternative.trials[0].evidence, ui::DisplayEvidenceState::NotEstablished);
        assert_eq!(value.repair.profile_id, base.profile.id);
        assert_eq!(value.repair.evidence, base.profile.evidence);
    }

    #[test]
    fn effective_accessibility_comes_from_retained_profile_not_default_choice_spelling() {
        let (f, mut base) = projection_fixture();
        base.profile.capabilities.accessibility = profiles::Accessibility::DisabledForVendorProcess;
        base.profile.limitations.push(profiles::Limitation::WindowsAccessibilityUnavailable);
        // Synthetic retained advice, as in the preparation owner regression.
        // No support claim is made for this fixture.
        base.profile.evidence.push("evidence/default-accessibility-advice.json".into());
        // No explicit local override: this candidate resolved profile advice to Off.
        assert_eq!(prep::configuration::settings(&base).accessibility, ui::AccessibilityChoice::ProfileDefault);
        prep::record_candidate(&f.m, &base).unwrap();
        let selected = prep::enable(&f.m, &base, false).unwrap();
        let mut product = projected(&f.m, &base);
        let value = facts(&product);
        assert_eq!(value.settings.effective_accessibility, ui::EffectiveAccessibility::DisabledForHost);
        assert_eq!(value.settings.expected_current.as_ref().unwrap().id, selected.id);
        assert!(value.repair.steps.iter().find(|step| step.kind == ui::RepairStepKind::ProfileAccessibility)
            .unwrap().label.contains("Windows accessibility defaults"));
        assert_eq!(value.settings.choices.len(), 4);
        assert!(value.settings.choices.iter().all(|choice| choice.settings.accessibility != ui::AccessibilityChoice::ProfileDefault));
        let exact = value.settings.choices[0].action.clone();
        let offer = product.actions.iter_mut().find(|offer| offer.action == exact).unwrap();
        offer.disabled_reason = Some("Close the affected instance".into());
        bind(&mut product);
        assert!(facts(&product).settings.choices.iter().any(|choice| choice.action == exact));
        product.actions.retain(|offer| offer.action != exact);
        bind(&mut product);
        assert!(!facts(&product).settings.choices.iter().any(|choice| choice.action == exact));
    }

    #[test]
    fn display_recipe_uses_real_offers_and_exact_previous_selection_only() {
        let (f, base) = projection_fixture();
        prep::record_candidate(&f.m, &base).unwrap();
        let original = prep::enable(&f.m, &base, false).unwrap();
        let mut product = projected(&f.m, &base);
        let initial = facts(&product);
        assert_eq!(initial.modes.iter().filter(|m| m.prepare.is_some()).count(), 1,
            "accessibility-only offers must not be labelled as a display-mode change");
        let accessibility = initial.repair.steps.iter().find(|s| s.kind == ui::RepairStepKind::ProfileAccessibility)
            .unwrap().action.as_ref().unwrap();
        assert!(matches!(accessibility, ui::Action::CandidateSettingsTest {settings,..}
            if settings.graphics == initial.settings.current.graphics),
            "accessibility repair keeps the selected graphics mode");
        product.actions.push(ui::AvailableAction {label:"Resume installation".into(),
            action:ui::Action::SoftwareInstallResume {onboarding:"retained-installation".into()},
            disabled_reason:None});
        bind(&mut product);
        let repair = facts(&product).repair.steps.iter().find(|step|
            step.kind == ui::RepairStepKind::RepairInstallation).unwrap();
        assert!(repair.action.is_none(), "unfinished installation continuation is not installation repair");
        assert_eq!(repair.unavailable_reason.as_deref(),Some("Choose the maker's installer to run in this same setup."));
        let trial = prep::configuration::prepare(&f.m, &base,
            Some(ui::GraphicsBackend::WineD3d11), Some(&original)).unwrap();
        let selected = prep::replace(&f.m, &trial, &original).unwrap();
        product = projected(&f.m, &trial);
        let restore = facts(&product).repair.steps.iter().find(|s| s.kind == ui::RepairStepKind::RestorePrevious)
            .unwrap().action.clone().unwrap();
        assert_eq!(restore, ui::Action::ExperimentalDisable {candidate:trial.id().unwrap()});
        let offers: Vec<_> = product.actions.iter().chain(product.compatibility.iter()
            .flat_map(|w| w.primary.iter().chain(&w.alternatives))).map(|o|o.action.clone()).collect();
        for step in &facts(&product).repair.steps {
            assert!(step.action.as_ref().is_none_or(|a| offers.contains(a)));
            assert!(step.action.is_some() || step.unavailable_reason.is_some());
        }
        // A later projection may remove or disable a control. Rebinding does
        // not retain the earlier permission, nor invent a replacement.
        for offer in product.actions.iter_mut().chain(product.compatibility.iter_mut()
            .flat_map(|w| w.primary.iter_mut().chain(&mut w.alternatives))) {
            if offer.action == restore {offer.disabled_reason = Some("Exact instance is still running".into());}
        }
        bind(&mut product);
        assert_eq!(facts(&product).repair.steps.iter().find(|s| s.kind == ui::RepairStepKind::RestorePrevious)
            .unwrap().unavailable_reason.as_deref(), Some("Exact instance is still running"));
        product.actions.clear();
        product.compatibility = None;
        bind(&mut product);
        assert!(facts(&product).repair.steps.iter().all(|s| s.action.is_none() && s.unavailable_reason.is_some()));
        assert!(facts(&product).modes.iter().all(|m| m.prepare.is_none() && m.trials.iter().all(|t|t.actions.is_empty())));
        prep::disable_exact(&f.m, &trial, &selected).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original));
    }
}

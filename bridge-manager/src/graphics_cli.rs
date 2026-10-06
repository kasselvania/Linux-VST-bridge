//! Explicit diagnostics use ordinary inspected identities and process custody.
use crate::*;
use graphics::{assessment, pe};
use linux_vst_bridge::{operator_model as ui, preparation as prep};

pub(super) fn project(m: &Manager, product: &mut ui::Product,
    workflow: &mut ui::CompatibilityWorkflow, view: &prep::View,
    candidates: &[prep::Candidate], busy: Option<&str>) -> Result<()> {
    let Some(c) = candidates.iter().find(|c|
        c.id().ok().as_deref() == workflow.current_candidate.as_deref()) else { return Ok(()) };
    let id = c.id()?;
    let row = view.candidates.iter().find(|row| row.id == id).ok_or("graphics_candidate_view")?;
    let mut configuration = prep::configuration::view(m, c)?;
    configuration["publication"] = serde_json::json!(row.publication);
    // The product prepares a Wine D3D11 trial itself after an assessment that
    // asks for it. Mark that next to the assessment, on the base's own view and
    // on the "before" view shown with its trial, so no stale "prepare" remains.
    let mark_applied = |assessment: &mut serde_json::Value, trial: &str| {
        if assessment["recommendation"]["requirement"] == "wine_d3d11" {
            let setup = view.candidates.iter().find(|row| row.id == trial).map(|row| row.lineage.ordinal);
            assessment["applied"] = serde_json::json!({"prepared": trial, "setup": setup});
        }
    };
    if let Some(trial) = &c.settings_trial {
        if let Some(base) = candidates.iter().find(|c| c.id().is_ok_and(|id| id == trial.predecessor)) {
            configuration["before"] = prep::configuration::view(m, base)?;
            if c.profile.capabilities.graphics == Some(ui::GraphicsBackend::WineD3d11) {
                mark_applied(&mut configuration["before"]["assessment"], &id);
            }
        }
    }
    if let Some(trial) = candidates.iter().find(|t| t.settings_trial.as_ref()
        .is_some_and(|trial| trial.predecessor == id)
        && t.profile.capabilities.graphics == Some(ui::GraphicsBackend::WineD3d11))
        .and_then(|t| t.id().ok()) {
        mark_applied(&mut configuration["assessment"], &trial);
    }
    // Every trial offer owns its own predecessor. A different proposal cannot
    // supply labels, settings or publication authority for the selected one.
    for prepared in candidates.iter().filter(|candidate|
        prep::same_product(&candidate.selection, &c.selection)) {
        let Some(trial) = &prepared.settings_trial else { continue };
        let prepared_id = prepared.id()?;
        let Some(prepared_row) = view.candidates.iter().find(|row| row.id == prepared_id) else { continue };
        for offer in product.actions.iter_mut().chain(workflow.primary.iter_mut())
            .chain(workflow.alternatives.iter_mut()) {
            match &offer.action {
                ui::Action::ExperimentalDisable { candidate } if candidate == &prepared_id =>
                    offer.label = if trial.baseline.is_some() { "Restore the settings from before this trial" }
                        else { "Remove this trial from the DAW" }.into(),
                ui::Action::CompatibilityResult { candidate, .. } if candidate == &prepared_id =>
                    offer.label = "Keep these settings and record the result".into(),
                ui::Action::ExperimentalReplace { candidate, .. }
                | ui::Action::ExperimentalEnable { candidate }
                | ui::Action::CompatibilityPublishTest { candidate, .. } if candidate == &prepared_id => {
                    let graphics = if prepared.profile.capabilities.graphics.is_some() {
                        "Wine D3D11 graphics"
                    } else { "runtime graphics defaults" };
                    let accessibility = if prepared.profile.capabilities.accessibility
                        == profiles::Accessibility::DisabledForVendorProcess {
                        "Windows accessibility disabled"
                    } else { "Windows accessibility defaults" };
                    offer.label = format!("Try {graphics}, {accessibility} · setup {}", prepared_row.lineage.ordinal);
                    let selected = if prepared_row.publication == "another_configuration" {
                        view.current_revision.as_ref()
                    } else { None };
                    if selected != trial.baseline.as_ref() {
                        offer.disabled_reason = Some("The previous selection changed. Prepare a trial from the current configuration.".into());
                    }
                }
                _ => {}
            }
        }
    }
    if row.ordinary_acceptance_current {
        configuration["qualification"] = serde_json::json!("Accepted exact local configuration against retained observations; not general platform support.");
    }
    product.details["configuration"] = configuration.clone();
    // Compatibility projection for older report consumers; one owner/result.
    product.details["graphics"] = configuration;
    let configuration_blocked = if !row.current_inputs {
        Some("Refresh this plug-in's changed inputs before assessing settings")
    } else if matches!(workflow.phase, ui::CompatibilityPhase::PublicationNeedsAttention
        | ui::CompatibilityPhase::AwaitingRetirement | ui::CompatibilityPhase::TestResultIncomplete) {
        Some("Finish the current recovery before another graphics operation")
    } else { None };
    let blocked = busy.or(configuration_blocked);
    product.actions.push(ui::AvailableAction {
        label: "Assess graphics with these settings".into(),
        action: ui::Action::CandidateGraphicsAssess { candidate: id.clone() },
        disabled_reason: blocked.map(str::to_owned),
    });
    let base = if row.publication == "another_configuration" {
        view.candidates.iter().find(|v| matches!(v.publication.as_str(), "ordinary" | "experimental"))
            .and_then(|v| candidates.iter().find(|c| c.id().is_ok_and(|id| id == v.id)))
    } else { Some(c) };
    if let Some(base) = base.filter(|base| base.origin == prep::Origin::ManagedPreparation) {
            let current = if matches!(row.publication.as_str(), "ordinary" | "experimental" | "another_configuration") {
                view.current_revision.as_ref()
            } else { None };
            let selected = prep::configuration::settings(base);
            let mut choices = Vec::new();
            let mut graphics = selected.clone();
            graphics.graphics = if selected.graphics.is_some() { None }
                else { Some(ui::GraphicsBackend::WineD3d11) };
            choices.push((if graphics.graphics.is_some() { "Prepare Wine D3D11 graphics fallback" }
                else { "Prepare runtime graphics defaults" }, graphics));
            for (accessibility, label) in [
                (ui::AccessibilityChoice::ProfileDefault, "Prepare profile accessibility defaults"),
                (ui::AccessibilityChoice::WindowsDefault, "Prepare Windows accessibility defaults"),
                (ui::AccessibilityChoice::DisabledForHost, "Prepare disabling Windows accessibility for this plug-in"),
            ] {
                if selected.accessibility != accessibility {
                    let mut settings = selected.clone();
                    settings.accessibility = accessibility;
                    choices.push((label, settings));
                }
            }
            for (label, settings) in choices {
                product.actions.push(ui::AvailableAction {
                    label: label.into(),
                    action: ui::Action::CandidateSettingsPrepare { candidate: base.id()?, settings,
                        expected_current: current.map(|current| ui::PublicationIdentity {
                            id: current.id.clone(), sha256: current.sha256.clone() }) },
                    // This records immutable next-launch choices only. Applying
                    // the trial is a separate owner-scoped publication operation.
                    disabled_reason: configuration_blocked.map(str::to_owned),
                });
            }
    }
    Ok(())
}

pub(super) fn run(m: &Manager, request_path: &Path) -> Result<()> {
    let binding = inspection_binding_for(m, read_json(request_path)?, false)?;
    let result = assess(m, binding, || m.lock("registry.lock"))?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

pub(super) fn assess(m: &Manager, binding: HostBinding,
    registry_admission: impl FnOnce() -> Result<Lock>) -> Result<assessment::Assessment> {
    with_launch_verification(|| {
        let selected = software(m)?;
        let sw = package_authority::paired_host_components(m, &selected, &binding.host,
            &binding.host_source_sha256)?;
        let module_stamp = observation::ModuleStamp::read(&binding.module.path)?;
        let imports = pe::inspect(&mut file(&binding.module.path)?)?;
        binding.module.verify()?;
        let context = assessment::Context::for_configuration(&binding.environment,
            &binding.module, &binding.metadata.class_id, &binding.host,
            &binding.host_source_sha256, &binding.compatibility)?;
        context.fingerprint()?;
        let registry = registry_admission()?;
        m.require_inactive(None)?;
        let (mut job, path) = spec(m, binding, true, false, false)?;
        job.graphics_assessment = true;
        atomic_json(&path, &job)?;
        let pending = PendingAdmission::new(job.lease.clone(), Arc::new(AtomicBool::new(false)));
        let child = spawn(m, &sw, &path, None)?;
        drop(registry);
        vendor_product_cli::finish_scan(child, &job, &path, pending)?;
        job.registration.module.verify()?;
        require(
            observation::ModuleStamp::read(&job.registration.module.path)? == module_stamp,
            "graphics_module_changed_during_assessment",
        )?;
        require(
            environment_record(m, &job.registration.environment.id)?
                == job.registration.environment,
            "graphics_environment_changed_during_assessment",
        )?;
        let raw: serde_json::Value = read_json(&job.report)?;
        let result = assessment::assemble(context, imports, &raw, observation::now()?)?;
        let directory = m.root.join("observations/graphics");
        private_dir(&directory)?;
        atomic_json(&directory.join(format!("{}.json", job.session)), &result)?;
        Ok(result)
    })
}

/// The publication a settings trial from this candidate must keep as its
/// baseline: the class's current managed revision when this candidate is the
/// published one, otherwise none. `prepare_settings` rechecks the same facts
/// under the registry lock.
fn trial_baseline(m: &Manager, c: &prep::Candidate) -> Result<Option<publication::RevisionRef>> {
    if !matches!(prep::publication_state(m, c)?.as_str(), "ordinary" | "experimental") {
        return Ok(None);
    }
    Ok(m.registry()?.classes.get(&c.selection.class.id)
        .and_then(|entry| entry.managed_revision.clone()))
}

/// Do what the rule asks without a click. Only Wine's built-in Direct3D 11 has
/// a launch-only trial to prepare here; the DirectComposition runner and the
/// other requirements stay a named recommendation. Preparing changes no
/// publication: trying the trial in the DAW remains the operator's explicit,
/// owner-scoped step. A refusal is reported as text, never as a failed assessment.
pub(super) fn apply_recommendation(m: &Manager, c: &prep::Candidate,
    requirement: assessment::Requirement) -> Result<(serde_json::Value, Option<prep::Candidate>)> {
    let mut applied = serde_json::json!({"requirement": requirement, "prepared": null});
    if requirement != assessment::Requirement::WineD3d11
        || prep::configuration::settings(c).graphics == Some(ui::GraphicsBackend::WineD3d11) {
        return Ok((applied, None));
    }
    let _guard = m.lock("registry.lock")?;
    let baseline = trial_baseline(m, c)?;
    match prep::configuration::prepare(m, c, Some(ui::GraphicsBackend::WineD3d11), baseline.as_ref()) {
        Ok(trial) => {
            applied["prepared"] = serde_json::json!(trial.id()?);
            Ok((applied, Some(trial)))
        }
        Err(error) => {
            applied["refusal"] = serde_json::json!(error.to_string());
            Ok((applied, None))
        }
    }
}

type Assessor<'a> = &'a mut dyn FnMut(&prep::Candidate) -> Result<assessment::Assessment>;

/// Every preparation ends with the editor's graphics requirement decided and
/// acted on. A run retained under this exact launch context is reused; the
/// explicit assessment action still makes a fresh one. A failed assessment
/// never fails the preparation: audio publication does not depend on it.
pub(super) fn follow_preparation(m: &Manager, sw: &Software, c: &prep::Candidate, operation: &str,
    assess: Assessor<'_>) -> serde_json::Value {
    let mut run = || -> Result<serde_json::Value> {
        let (mut value, requirement) = decided(m, sw, c, operation, assess)?;
        value["applied"] = applied(m, sw, c, operation, requirement, assess)?;
        Ok(value)
    };
    run().unwrap_or_else(assessment_failure)
}

/// After a fresh assessment: prepare what it asks for and assess the trial.
pub(super) fn follow_assessment(m: &Manager, sw: &Software, c: &prep::Candidate, operation: &str,
    requirement: assessment::Requirement, assess: Assessor<'_>) -> serde_json::Value {
    applied(m, sw, c, operation, requirement, assess).unwrap_or_else(assessment_failure)
}

fn assessment_failure(error: Box<dyn std::error::Error + Send + Sync>) -> serde_json::Value {
    serde_json::json!({"stage":"assessment_failed",
        "failure": error.to_string().chars().take(512).collect::<String>()})
}

/// Prepare the trial the rule asks for, then assess that trial too, so the
/// musician sees whether Wine's Direct3D 11 passes before trying it in the DAW.
fn applied(m: &Manager, sw: &Software, c: &prep::Candidate, operation: &str,
    requirement: assessment::Requirement, assess: Assessor<'_>) -> Result<serde_json::Value> {
    let (mut value, trial) = apply_recommendation(m, c, requirement)?;
    if let Some(trial) = trial {
        value["trial"] = decided(m, sw, &trial, operation, assess)
            .map(|(assessed, _)| assessed).unwrap_or_else(assessment_failure);
    }
    Ok(value)
}

/// The requirement for this candidate: from a run retained under its exact
/// launch context when one carries a decision, otherwise from one made now.
fn decided(m: &Manager, sw: &Software, c: &prep::Candidate, operation: &str,
    assess: Assessor<'_>) -> Result<(serde_json::Value, assessment::Requirement)> {
    if let Some(retained) = prep::configuration::latest_assessment(m, c)? {
        if let Ok(requirement) = serde_json::from_value::<assessment::Requirement>(
            retained["recommendation"]["requirement"].clone()) {
            return Ok((serde_json::json!({"stage":"retained","candidate":c.id()?,
                "observed_at":retained["observed_at"],
                "recommendation":retained["recommendation"]}), requirement));
        }
    }
    let result = assess(c)?;
    let _guard = m.lock("registry.lock")?;
    prep::verify_candidate(m, c, &sw.host, &sw.source_sha256)?;
    prep::configuration::record_assessment(m, c, operation, &result)?;
    let requirement = result.recommendation.requirement;
    Ok((serde_json::json!({"stage":"assessed","candidate":c.id()?,
        "recommendation":result.recommendation}), requirement))
}

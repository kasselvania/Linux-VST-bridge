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
    if let Some(trial) = &c.settings_trial {
        if let Some(base) = candidates.iter().find(|c| c.id().is_ok_and(|id| id == trial.predecessor)) {
            configuration["before"] = prep::configuration::view(m, base)?;
        }
        for offer in product.actions.iter_mut().chain(workflow.primary.iter_mut()) {
            match &offer.action {
                ui::Action::ExperimentalDisable { candidate } if candidate == &id =>
                    offer.label = if trial.baseline.is_some() { "Restore the settings from before this trial" }
                        else { "Remove this trial from the DAW" }.into(),
                ui::Action::CompatibilityResult { candidate, .. } if candidate == &id =>
                    offer.label = "Keep these settings and record the result".into(),
                ui::Action::ExperimentalReplace { candidate, .. }
                | ui::Action::CompatibilityPublishTest { candidate, .. } if candidate == &id => {
                    offer.label = "Try the prepared compatibility settings".into();
                    let selected = if row.publication == "another_configuration" {
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

//! MF3 operator crossings. The frontend selects offered identities only.
use super::*;
use linux_vst_bridge::{operator_model as ui, preparation as prep};
use serde_json::{json, Value};
pub fn project(
    m: &Manager,
    sw: &Software,
    products: &mut [ui::Product],
    busy: Option<&str>,
) -> Result<()> {
    project_with_excluded_operation(m, sw, products, busy, None)
}

/// Worker validation must not let its own waiting receipt hide the action it
/// is validating. Ordinary snapshots still show that in-flight operation.
pub(super) fn project_for_operation(
    m: &Manager,
    sw: &Software,
    products: &mut [ui::Product],
    busy: Option<&str>,
    operation: &str,
) -> Result<()> {
    project_with_excluded_operation(m, sw, products, busy, Some(operation))
}

fn project_with_excluded_operation(
    m: &Manager,
    sw: &Software,
    products: &mut [ui::Product],
    busy: Option<&str>,
    exclude_operation: Option<&str>,
) -> Result<()> {
    let selections = product_selections(
        prep::selections(m, &sw.host, &sw.source_sha256)?,
        prep::candidates(m, &sw.host, &sw.source_sha256)?,
    )?;
    for s in selections {
        let Some(p) = products.iter_mut().find(|p| {
            p.class_id == s.class.id
                && p.module_sha256 == s.module.sha256
                && p.environment == s.environment.id
        }) else {
            continue;
        };
        let mut v = prep::view(m, &s, &sw.host, &sw.source_sha256)?;
        // Ordinary registry readback remains authority; MF3 augments that card.
        let canonical = matches!(p.disposition.as_str(), "ready" | "needs_attention")
            && (v.candidates.is_empty()
                || p.details["profile"]["claim"] == "verified_exact_fixture");
        if !canonical {
            p.active_revision = v.current_profile_revision;
        }
        p.details["preparation"] = serde_json::to_value(&v)?;
        if canonical && v.candidates.is_empty() {
            p.details["preparation"]["publication"] = json!(if p.disposition == "ready" {
                "ordinary"
            } else {
                "needs_attention"
            });
            p.details["preparation"]["current_profile_revision"] = json!(p.active_revision);
            p.details["preparation"]["canonical_publication"] = p.details["publication"].clone();
        }
        let offer = |label: String, action: ui::Action, reason: Option<&str>| ui::AvailableAction {
            label,
            action,
            disabled_reason: reason.map(str::to_owned),
        };
        let current_selection = prep::verify_selection(m, &s, &sw.host, &sw.source_sha256).is_ok();
        let stale = if current_selection {
            None
        } else {
            Some("Historical selection: current inventory/scanner must be refreshed before preparation or publication")
        };
        let current_layout = v.recommended_audio_layout.clone();
        p.actions.push(offer(
            match (&v.recommended_inspection, &current_layout) {
                (Some(_), Some(profiles::AudioLayoutPolicy::StereoMainPair)) => {
                    "Refresh stereo inspection"
                }
                (Some(_), None) => "Refresh preliminary inspection",
                (None, _) => "Check compatibility",
            }
            .into(),
            ui::Action::PluginReinspect {
                selection: v.selection.clone(),
                audio_layout: current_layout.clone(),
            },
            busy.or(stale),
        ));
        if s.class.role == "effect" {
            let (label, audio_layout) = if current_layout
                == Some(profiles::AudioLayoutPolicy::StereoMainPair)
            {
                ("Check default-layout compatibility", None)
            } else {
                (
                    "Check stereo compatibility",
                    Some(profiles::AudioLayoutPolicy::StereoMainPair),
                )
            };
            p.actions.push(offer(
                label.into(),
                ui::Action::PluginReinspect {
                    selection: v.selection.clone(),
                    audio_layout,
                },
                busy.or(stale),
            ));
        }
        if let Some(inspection) = &v.recommended_inspection {
            let kit = prep::build::recipe(m);
            let controller = matches!(
                v.controller,
                Some(prep::ControllerAssociation::Unavailable { .. })
            );
            if let Ok(kit) = kit {
                p.actions.push(offer(
                    "Prepare a test candidate for this inspection and recipe".into(),
                    ui::Action::PluginPrepare {
                        selection: v.selection.clone(),
                        inspection: inspection.clone(),
                        recipe: kit.sha256,
                        predecessor: v.candidate.clone(),
                    },
                    busy.or(stale).or(if controller {
                        Some("Refresh inspection to retain exact controller association")
                    } else {
                        None
                    }),
                ));
            } else {
                p.details["preparation"]["build_prerequisite"] =
                    json!("Install a current preparation kit to prepare another generation");
            }
        }
        if !canonical {
            p.disposition = match v.publication.as_str() {
                "needs_attention" => "needs_attention",
                "another_configuration" => "another_configuration",
                "experimental" => "experimental",
                "ordinary" => "ready",
                _ => {
                    if v.candidates.is_empty() {
                        "installed_unqualified"
                    } else {
                        "prepared"
                    }
                }
            }
            .into();
        }
        for h in &v.candidates {
            let id = &h.id;
            let label = |name: &str| format!("{} · candidate {}", name, &id[..12]);
            let historical = if h.current_inputs {
                None
            } else {
                Some("Historical candidate inputs: retain evidence; prepare a current generation before enabling")
            };
            if let Some(a) = publication_action(
                id,
                &h.publication,
                v.current_revision.as_ref(),
                if matches!(h.publication.as_str(), "ordinary" | "experimental") {
                    busy
                } else {
                    busy.or(historical)
                },
            ) {
                p.actions.push(a);
            }
            if h.publication == "needs_attention" {
                p.details["preparation"]["recovery"]=json!("Publication and physical state disagree. Reconcile the managed transaction before enabling or replacing a candidate.");
            }
            p.actions.push(offer(
                label("Record an operator observation"),
                ui::Action::CandidateObserve {
                    candidate: id.clone(),
                    area: "editor".into(),
                    status: "not_tested".into(),
                    note: String::new(),
                },
                None,
            ));
            p.actions.push(offer(label("Review this exact configuration"),ui::Action::CandidateReview{candidate:id.clone(),accept:true,rationale:String::new()},busy.or(if h.unmet_requirements.is_empty(){None}else{Some("Qualification incomplete: inspect missing or failed results for this candidate")})));
            p.actions.push(offer(
                label("Record needs-work review"),
                ui::Action::CandidateReview {
                    candidate: id.clone(),
                    accept: false,
                    rationale: String::new(),
                },
                None,
            ));
            if h.publication != "ordinary" {
                p.actions.push(offer(
                label("Publish accepted exact configuration for ordinary use"),
                ui::Action::CandidatePublishOrdinary {
                    candidate: id.clone(),
                },
                busy.or(historical).or(
                    if h.ordinary_acceptance_current
                        && !matches!(
                            h.publication.as_str(),
                            "needs_attention" | "another_configuration"
                        )
                    {
                        None
                    } else {
                        Some("Current exact acceptance and a reconciled publication are required")
                    },
                ),
            ));
            }
        }
        if let Some(rows) = p.details["preparation"]["candidates"].as_array_mut() {
            for row in rows {
                let id = row["id"].as_str().ok_or("candidate_view_id")?.to_owned();
                if let Some(op) = operator_cli::product_receipt(m, &v.selection, Some(&id), exclude_operation)? {
                    row["operation"] = op;
                }
            }
        }
        if let Some(op) = operator_cli::product_receipt(m, &v.selection, v.candidate.as_deref(), exclude_operation)? {
            v.operation = Some(op.clone());
            p.details["preparation"]["operation"] = op;
        }
        let mut workflow = guided_projection(m, &s, &v, busy, stale, canonical,
            p.disposition == "ready")?;
        // A failed test can be removed while its accepted ancestor becomes
        // current, before the result's completion marker is retained. Search
        // every candidate for this exact product, not only the selected one.
        let mut results = Vec::new();
        for candidate in &v.candidates {
            results.extend(prep::guided_results(m, &candidate.id)?);
        }
        let incomplete: Vec<_> = results.iter().filter(|row| row["completed"] != true).collect();
        if incomplete.len() > 1 {
            workflow.phase = ui::CompatibilityPhase::PublicationNeedsAttention;
            workflow.summary = "Several test-result operations need reconciliation. Review their exact history before continuing.".into();
            workflow.primary = None;
        } else if let Some(incomplete) = incomplete.first() {
            let operation = incomplete["operation"].as_str().ok_or("guided_result_operation")?;
            let action: ui::Action = serde_json::from_value(incomplete["action"].clone())?;
            let ui::Action::CompatibilityResult { candidate, result, expected_current, .. } = action
                else { return Err("guided_result_intent_action".into()) };
            let candidate = prep::candidate(m, &candidate, &sw.host, &sw.source_sha256)?;
            let (phase, summary, refusal) = match (result, prep::publication_state(m, &candidate)?.as_str()) {
                (ui::TestResultKind::Problem { .. }, "experimental")
                    if exact_test_publication(m, &candidate, &expected_current).is_ok() => (
                    ui::CompatibilityPhase::AwaitingRetirement,
                    "Problem recorded. The test configuration is still selected. Close the DAW or complete recovery, then finish this result.",
                    busy.map(str::to_owned),
                ),
                (ui::TestResultKind::Problem { .. }, _) => match guided_result_disposition(m, &candidate, &expected_current, operation) {
                    Ok(GuidedDisposition::Restored) => (
                        ui::CompatibilityPhase::TestResultIncomplete,
                        "Test result needs final reconciliation. The failed test configuration was removed and the previous working configuration is selected.",
                        busy.map(str::to_owned),
                    ),
                    Ok(GuidedDisposition::Removed) => (
                        ui::CompatibilityPhase::TestResultIncomplete,
                        "Test result needs final reconciliation. The failed test configuration was removed; finish its exact record.",
                        busy.map(str::to_owned),
                    ),
                    Err(_) => (
                        ui::CompatibilityPhase::PublicationNeedsAttention,
                        "Test result needs reconciliation. Its exact publication disposition cannot be confirmed; the current configuration will be preserved.",
                        Some("Exact publication disposition cannot be confirmed; reconcile it before finishing".into()),
                    ),
                },
                (ui::TestResultKind::Worked, _) => (
                    ui::CompatibilityPhase::TestResultIncomplete,
                    "Test result was only partly recorded. Finish the exact disposition after confirming the plug-in is closed.",
                    busy.map(str::to_owned),
                ),
            };
            workflow.phase = phase;
            workflow.summary = summary.into();
            workflow.primary = Some(ui::AvailableAction {
                label: "Finish recording test result".into(),
                action: ui::Action::CompatibilityFinishResult { operation: operation.into() },
                disabled_reason: refusal,
            });
        }
        p.details["preparation"]["guided_results"] = json!(results);
        let checks = prep::guided_checks(m, &v.selection)?;
        if incomplete.is_empty() {
            let unfinished: Vec<_> = checks.iter().filter(|row| matches!(row["stage"].as_str(),
                Some("started" | "inspection_retained" | "candidate_retained"))).collect();
            if unfinished.len() > 1 {
                workflow.phase = ui::CompatibilityPhase::PublicationNeedsAttention;
                workflow.summary = "Several interrupted compatibility checks need reconciliation. Their exact history is retained.".into();
                workflow.primary = None;
                workflow.alternatives.clear();
            } else if let Some(row) = unfinished.first() {
                let source = row["operation"].as_str().ok_or("guided_check_operation")?;
                let intent = prep::guided_check_stage(m, source, "intent")?
                    .ok_or("guided_check_resume_intent_missing")?;
                let original: ui::Action = serde_json::from_value(intent["action"].clone())?;
                let source_state = operator_cli::resumable_check_source(m, source, &original);
                workflow.alternatives.clear();
                match source_state {
                    Ok(false) => {
                        workflow.phase = ui::CompatibilityPhase::Checking;
                        workflow.summary = "The exact compatibility check is still running or retiring.".into();
                        workflow.primary = None;
                    }
                    state => {
                        let reason = if let Err(e) = state {
                            Some(format!("Original check authority cannot be confirmed: {e}"))
                        } else if let ui::Action::CompatibilityCheck {
                            selection, recipe, predecessor, ..
                        } = &original {
                            if selection != &v.selection || stale.is_some() {
                                Some("Installed selection changed; the interrupted check cannot continue".into())
                            } else if prep::build::recipe(m).ok().as_ref().map(|kit| &kit.sha256)
                                != Some(recipe) {
                                Some("Preparation recipe changed; the interrupted check cannot continue".into())
                            } else if v.candidate != *predecessor
                                && v.candidate.as_deref() != row["candidate"].as_str() {
                                Some("Candidate predecessor changed; the interrupted check cannot continue".into())
                            } else { busy.map(str::to_owned) }
                        } else {
                            Some("Original check action is not an exact compatibility check".into())
                        };
                        workflow.phase = if reason.is_some() && busy.is_none() {
                            ui::CompatibilityPhase::PublicationNeedsAttention
                        } else { ui::CompatibilityPhase::CheckFailed };
                        workflow.summary = "Compatibility check was interrupted. Its completed inspection and candidate stages are retained; finish the exact check to continue.".into();
                        workflow.primary = Some(ui::AvailableAction {
                            label: "Finish compatibility check".into(),
                            action: ui::Action::CompatibilityResumeCheck { operation: source.into() },
                            disabled_reason: reason,
                        });
                    }
                }
            }
        }
        p.details["preparation"]["guided_checks"] = json!(checks);
        p.disposition = match workflow.phase {
            ui::CompatibilityPhase::OrdinarySupported => "ready",
            ui::CompatibilityPhase::AvailableForTest | ui::CompatibilityPhase::PassedExperimental => "experimental",
            ui::CompatibilityPhase::ReadyForTest => "prepared",
            ui::CompatibilityPhase::NotChecked | ui::CompatibilityPhase::Checking => "installed_unqualified",
            ui::CompatibilityPhase::CheckBlocked | ui::CompatibilityPhase::CheckFailed
            | ui::CompatibilityPhase::AwaitingRetirement
            | ui::CompatibilityPhase::TestResultIncomplete | ui::CompatibilityPhase::NeedsWork
            | ui::CompatibilityPhase::PublicationNeedsAttention | ui::CompatibilityPhase::HistoricalOnly => "needs_attention",
        }.into();
        p.compatibility = Some(workflow);
    }
    Ok(())
}

fn guided_projection(
    m: &Manager,
    selection: &prep::Selection,
    view: &prep::View,
    busy: Option<&str>,
    stale: Option<&str>,
    canonical: bool,
    ordinary_ready: bool,
) -> Result<ui::CompatibilityWorkflow> {
    use ui::CompatibilityPhase as Phase;
    let published = view.candidates.iter().find(|row| row.disposition == "current_published");
    let newest = view.candidates.iter().rev().find(|row| row.current_inputs
        && row.review.as_ref().is_none_or(|review| review.choice != prep::ReviewChoice::NeedsWork));
    let selected = match (published, newest) {
        (Some(current), Some(newer)) if newer.lineage.ordinal > current.lineage.ordinal => Some(newer),
        (Some(current), _) => Some(current),
        (None, Some(newest)) => Some(newest),
        (None, None) => view.candidates.last(),
    };
    let mut primary = None;
    let mut alternatives = Vec::new();
    let mut established = Vec::new();
    let mut remaining = Vec::new();
    if let Some(row) = selected {
        for observation in &row.evidence {
            if observation.status == prep::TestStatus::Passed
                && observation.witness != prep::Witness::GeneratedRegression {
                let label = format!("{} observed", area_label(observation.area));
                if !established.contains(&label) { established.push(label); }
            }
        }
        remaining = row.unmet_requirements.iter().map(|requirement|
            requirement.split(':').next().unwrap_or(requirement).replace('_', " ")).collect();
    }
    let exact = selected.filter(|row| row.current_inputs);
    let offer = |label: &str, action: ui::Action, disabled: Option<&str>| ui::AvailableAction {
        label: label.into(), action, disabled_reason: disabled.map(str::to_owned),
    };
    let recommended_host = view.inspections.iter().find(|row| row.recommended)
        .map(|row| row.host_sha256.as_str());
    let layout_ambiguous = selection.class.role == "effect"
        && recommended_host.is_some()
        && view.inspections.iter().any(|row| Some(row.host_sha256.as_str()) == recommended_host
            && row.audio_layout.is_none())
        && view.inspections.iter().any(|row| Some(row.host_sha256.as_str()) == recommended_host
            && row.audio_layout == Some(profiles::AudioLayoutPolicy::StereoMainPair));
    if layout_ambiguous {
        if let Ok(kit) = prep::build::recipe(m) {
            for (label, audio_layout) in [
                ("Stereo effect", Some(profiles::AudioLayoutPolicy::StereoMainPair)),
                ("Use the plug-in's reported default layout", None),
            ] {
                alternatives.push(offer(label, ui::Action::CompatibilityCheck {
                    selection: view.selection.clone(), audio_layout,
                    recipe: kit.sha256.clone(), predecessor: view.candidate.clone(),
                }, busy.or(stale)));
            }
        }
    }
    let check_offer = || -> Result<Option<ui::AvailableAction>> {
        if layout_ambiguous { return Ok(None); }
        let kit = match prep::build::recipe(m) { Ok(kit) => kit, Err(_) => return Ok(None) };
        Ok(Some(offer("Check compatibility", ui::Action::CompatibilityCheck {
            selection: view.selection.clone(),
            audio_layout: view.recommended_audio_layout.clone(),
            recipe: kit.sha256,
            predecessor: view.candidate.clone(),
        }, busy.or(stale))))
    };
    let (phase, summary) = if canonical && view.candidates.is_empty() {
        if ordinary_ready { (Phase::OrdinarySupported, "Supported exact configuration".into()) }
        else { (Phase::PublicationNeedsAttention, "Existing publication needs attention.".into()) }
    } else if stale.is_some() {
        (Phase::HistoricalOnly, "This installed selection is historical. Refresh its managed inventory before checking it again.".into())
    } else if selected.is_some_and(|row| row.publication == "needs_attention") {
        (Phase::PublicationNeedsAttention, "Publication needs reconciliation before testing.".into())
    } else if let Some(row) = exact.filter(|row| row.publication == "ordinary") {
        let _ = row;
        (Phase::OrdinarySupported, "Supported exact configuration".into())
    } else if let Some(row) = exact.filter(|row| row.publication == "experimental") {
        if let Some(expected) = view.current_revision.as_ref() {
            primary = Some(offer("Record test result", ui::Action::CompatibilityResult {
                candidate: row.id.clone(), expected_current: publication_identity(expected),
                result: ui::TestResultKind::Worked, passed: vec![], failed_area: None,
                note: String::new(),
            }, None));
        }
        if row.review.as_ref().is_some_and(|review| review.choice == prep::ReviewChoice::AcceptExactLocal) {
            (Phase::PassedExperimental, "Recorded checks passed. This configuration remains experimental.".into())
        } else if row.evidence.iter().any(|observation|
            observation.status == prep::TestStatus::Passed
                && observation.witness == prep::Witness::OperatorObservation) {
            (Phase::AvailableForTest, "Test result recorded. Additional checks remain before this exact configuration can be accepted.".into())
        } else {
            (Phase::AvailableForTest, "Available temporarily in your native DAW. Test this exact plug-in once, then record what happened.".into())
        }
    } else if let Some(row) = exact.filter(|row| row.review.as_ref().is_some_and(|review| review.choice == prep::ReviewChoice::NeedsWork)) {
        let _ = row;
        (Phase::NeedsWork, "This exact configuration needs work. Its test publication is no longer active; installation and evidence remain.".into())
    } else if let Some(row) = exact {
        let publication = row.publication.as_str();
        if matches!(publication, "unpublished" | "removed" | "another_configuration") {
            let expected_current = if publication == "another_configuration" {
                view.current_revision.as_ref().map(publication_identity)
            } else { None };
            let ordinary_current = if publication == "another_configuration" {
                view.current_revision.as_ref().map(|revision|
                    m.load_revision(&selection.class.id, revision)
                        .map(|record| record.qualification.is_none())).transpose()?.unwrap_or(false)
            } else { false };
            if !ordinary_current
                && (publication != "another_configuration" || expected_current.is_some()) {
                primary = Some(offer(if publication == "another_configuration" {
                    "Replace it with this test configuration"
                } else { "Make available for testing" },
                ui::Action::CompatibilityPublishTest { candidate: row.id.clone(), expected_current }, busy));
            }
            (Phase::ReadyForTest, if ordinary_current {
                "An ordinarily supported configuration is currently selected. This test configuration cannot replace it through the guided flow."
            } else if publication == "another_configuration" {
                "A different configuration is selected. Replacing it for a test requires your explicit choice."
            } else { "Compatibility check completed. An immutable test configuration is ready." }.into())
        } else {
            (Phase::PublicationNeedsAttention, "Publication state needs review before testing.".into())
        }
    } else {
        let operation = &view.operation;
        let failed = matches!(operation.as_ref().and_then(|v| v["state"].as_str()), Some("refused" | "failed"));
        let running = matches!(operation.as_ref().and_then(|v| v["state"].as_str()), Some("queued" | "waiting" | "running"));
        if running {
            (Phase::Checking, "Checking the Windows plug-in and preparing one test configuration…".into())
        } else if failed {
            primary = check_offer()?;
            let reason = operation.as_ref().and_then(|row| row["reason"].as_str())
                .filter(|reason| !reason.is_empty()).unwrap_or("Review the retained check details");
            (Phase::CheckFailed, format!("Compatibility check stopped: {reason}. Installation is preserved; no test configuration was made available."))
        } else if prep::build::recipe(m).is_err() {
            (Phase::CheckBlocked, "Preparation tools are unavailable. Repair or update the manager package before checking compatibility.".into())
        } else {
            primary = check_offer()?;
            (Phase::NotChecked, if layout_ambiguous {
                "Choose one layout for the compatibility check. Both have retained exact inspection evidence."
            } else { "Installed · compatibility not checked" }.into())
        }
    };
    Ok(ui::CompatibilityWorkflow {
        phase, summary, established, remaining,
        current_inspection: view.recommended_inspection.clone(),
        current_candidate: selected.map(|row| row.id.clone()), primary, alternatives,
    })
}

fn area_label(area: prep::Area) -> &'static str {
    match area {
        prep::Area::DawLoad => "DAW loading", prep::Area::Midi => "MIDI",
        prep::Area::Audio => "Audio", prep::Area::Editor => "Editor",
        prep::Area::Parameters => "Controls", prep::Area::Automation => "Automation",
        prep::Area::StateRecall => "State recall", prep::Area::ProcessingRestart => "Processing restart",
        prep::Area::Retirement => "Clean close",
    }
}

fn result_area(area: ui::TestArea) -> prep::Area {
    match area {
        ui::TestArea::DawLoad => prep::Area::DawLoad,
        ui::TestArea::Midi => prep::Area::Midi,
        ui::TestArea::Audio => prep::Area::Audio,
        ui::TestArea::Editor => prep::Area::Editor,
        ui::TestArea::Parameters => prep::Area::Parameters,
        ui::TestArea::Automation => prep::Area::Automation,
        ui::TestArea::StateRecall => prep::Area::StateRecall,
        ui::TestArea::ProcessingRestart => prep::Area::ProcessingRestart,
        ui::TestArea::Retirement => prep::Area::Retirement,
    }
}
fn problem_area(category: ui::ProblemCategory) -> Option<ui::TestArea> {
    use ui::{ProblemCategory as Problem, TestArea as Area};
    Some(match category {
        Problem::FailedToLoad => Area::DawLoad,
        Problem::BlankEditor | Problem::EditorFroze | Problem::EditorDisappeared => Area::Editor,
        Problem::NoAudio | Problem::IncorrectAudio => Area::Audio,
        Problem::ControlsUnresponsive => Area::Parameters,
        Problem::HostCrashed | Problem::CleanupIncomplete => Area::Retirement,
        Problem::Other => return None,
    })
}

fn exact_test_publication(
    m: &Manager,
    candidate: &prep::Candidate,
    expected: &ui::PublicationIdentity,
) -> Result<()> {
    require(prep::publication_state(m, candidate)? == "experimental",
        "guided_test_publication_not_current")?;
    let db = m.registry()?;
    let current = db.classes.get(&candidate.selection.class.id)
        .and_then(|entry| entry.managed_revision.as_ref())
        .ok_or("guided_test_publication_missing")?;
    require(current.id == expected.id && current.sha256 == expected.sha256,
        "guided_test_publication_changed")?;
    let revision = m.load_revision(&candidate.selection.class.id, current)?;
    require(revision.profile == candidate.profile
        && revision.registration.module.sha256 == candidate.selection.module.sha256
        && revision.registration.native.sha256 == candidate.native.artifact.sha256
        && revision.registration.environment.id == candidate.selection.environment.id
        && revision.qualification == Some(publication::Qualification::ManagedExperimental),
        "guided_test_candidate_binding")
}

fn guided_result_entries(action: &ui::Action) -> Result<(Vec<(prep::Area, prep::TestStatus)>, &str)> {
    let ui::Action::CompatibilityResult {
        result, passed, failed_area, note, ..
    } = action else { return Err("guided_result_action".into()) };
    let is_problem = matches!(result, ui::TestResultKind::Problem { .. });
    require(passed.len() <= prep::AREAS.len()
        && (is_problem || !passed.is_empty())
        && (note.is_empty() || text(note)), "guided_result_bound")?;
    let expected_failed = match *result {
        ui::TestResultKind::Worked => {
            require(failed_area.is_none(), "guided_success_failure_area")?;
            None
        }
        ui::TestResultKind::Problem { category } => {
            require(text(note), "guided_problem_note_required")?;
            let area = problem_area(category).or(*failed_area).ok_or("guided_problem_area_required")?;
            require(failed_area.is_none() || *failed_area == Some(area), "guided_problem_area_changed")?;
            Some(area)
        }
    };
    let mut areas = std::collections::BTreeSet::new();
    let mut observations = Vec::new();
    for area in passed {
        require(areas.insert(*area), "guided_result_duplicate_area")?;
        observations.push((result_area(*area), prep::TestStatus::Passed));
    }
    if let Some(area) = expected_failed {
        require(areas.insert(area), "guided_result_conflicting_area")?;
        observations.push((result_area(area), prep::TestStatus::Failed));
    }
    Ok((observations, if note.is_empty() {
        "Operator confirmed only the selected checks in the native DAW"
    } else { note }))
}

fn guided_result(
    m: &Manager,
    sw: &Software,
    action: &ui::Action,
    operation: &str,
    registry_admission: impl FnOnce() -> Result<Lock>,
) -> Result<Value> {
    let ui::Action::CompatibilityResult {
        candidate, expected_current: expected, result, ..
    } = action else { return Err("guided_result_action".into()) };
    let (observations, detail) = guided_result_entries(action)?;
    let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
    let is_problem = matches!(result, ui::TestResultKind::Problem { .. });
    let already = prep::guided_results(m, candidate)?.into_iter()
        .find(|entry| entry["operation"] == operation);
    if let Some(entry) = &already {
        require(entry["action"] == serde_json::to_value(action)?,
            "guided_result_replay_changed")?;
        if entry["completed"] == true {
            return Ok(json!({"candidate":candidate,"result":"already_recorded",
                "publication_changed":false}));
        }
    }
    // An incomplete prior operation may resume only while its exact candidate
    // is still selected. Never write a review against a newer publication.
    exact_test_publication(m, &c, expected)?;
    if !is_problem {
        // A background canonical readback can briefly own registry.lock after
        // worker validation. Keep the final inactivity recheck, but use the
        // same bounded admission as other operator mutation crossings.
        let _guard = registry_admission()?;
        m.require_inactive(Some(&c.selection.class.id))?;
    }
    prep::retain_guided_result(m, operation, &serde_json::to_value(action)?)?;
    prep::record_guided_observations(m, &c, operation, &observations, detail)?;
    exact_test_publication(m, &c, expected)?;
    if is_problem {
        Ok(json!({"candidate":candidate,"result":"awaiting_retirement",
            "publication_changed":false,"evidence_preserved":true}))
    } else {
        let unmet = prep::unmet(&prep::observations(m, &c)?, &c.profile.role);
        if unmet.is_empty() {
            prep::review(m, &c, operation, prep::ReviewChoice::AcceptExactLocal, detail)?;
        }
        exact_test_publication(m, &c, expected)?;
        prep::complete_guided_result(m, operation)?;
        Ok(json!({"candidate":candidate,"result":if unmet.is_empty() {
            "passed_experimental" } else { "partial_experimental" },
            "remaining":unmet,"ordinary_published":false,"publication_changed":false}))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GuidedDisposition {
    Removed,
    Restored,
}

/// Read-only proof of the exact disposition already performed by this result.
/// A newer or foreign publication is never interpreted as its restoration.
fn guided_result_disposition(
    m: &Manager,
    c: &prep::Candidate,
    expected: &ui::PublicationIdentity,
    source_operation: &str,
) -> Result<GuidedDisposition> {
    require(prep::observations(m, c)?.iter().any(|row| row.operation == source_operation
        && row.status == prep::TestStatus::Failed), "guided_result_evidence_incomplete")?;
    require(prep::decisions(m, c)?.iter().any(|decision| decision.operation == source_operation
        && decision.choice == prep::ReviewChoice::NeedsWork), "guided_result_review_incomplete")?;
    let original = m.load_revision(&c.selection.class.id, &publication_reference(expected))?;
    require(original.profile == c.profile, "guided_result_original_publication")?;
    let state = prep::publication_state(m, c)?;
    if state == "removed" {
        let db = m.registry()?;
        require(db.classes.get(&c.selection.class.id).is_some_and(|entry|
            entry.publication == Publication::Removed),
            "guided_result_removed_readback")?;
        return Ok(GuidedDisposition::Removed);
    }
    require(state == "another_configuration", "guided_result_publication_drift")?;
    let current = m.registry()?.classes.get(&c.selection.class.id)
        .and_then(|entry| entry.managed_revision.clone())
        .ok_or("guided_result_restored_revision_missing")?;
    let mut parent = original.parent;
    let mut restored = false;
    for _ in 0..64 {
        let Some(reference) = parent else { break };
        let revision = m.load_revision(&c.selection.class.id, &reference)?;
        if revision.qualification.is_none()
            && revision.profile.claim == profiles::Claim::VerifiedExactFixture {
            restored = current == reference;
            break;
        }
        parent = revision.parent;
    }
    require(restored, "guided_result_restoration_drift")?;
    Ok(GuidedDisposition::Restored)
}

fn finish_guided_result(m: &Manager, sw: &Software, source_operation: &str) -> Result<Value> {
    let action: ui::Action = serde_json::from_value(prep::guided_result_intent(m, source_operation)?)?;
    let ui::Action::CompatibilityResult { candidate, result, expected_current, .. } = &action
        else { return Err("guided_result_intent_action".into()) };
    let (observations, detail) = guided_result_entries(&action)?;
    let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
    if prep::guided_results(m, candidate)?.iter().any(|row|
        row["operation"] == source_operation && row["completed"] == true) {
        return Ok(json!({"candidate":candidate,"result":"already_recorded",
            "publication_changed":false}));
    }
    match result {
        ui::TestResultKind::Worked => {
            exact_test_publication(m, &c, expected_current)?;
            {
                let _guard = m.lock("registry.lock")?;
                m.require_inactive(Some(&c.selection.class.id))?;
            }
            prep::record_guided_observations(m, &c, source_operation, &observations, detail)?;
            let unmet = prep::unmet(&prep::observations(m, &c)?, &c.profile.role);
            if unmet.is_empty() {
                prep::review(m, &c, source_operation, prep::ReviewChoice::AcceptExactLocal, detail)?;
            }
            exact_test_publication(m, &c, expected_current)?;
            prep::complete_guided_result(m, source_operation)?;
            Ok(json!({"candidate":candidate,"result":if unmet.is_empty() {
                "passed_experimental" } else { "partial_experimental" },
                "remaining":unmet,"resumed":true,"ordinary_published":false,
                "publication_changed":false}))
        }
        ui::TestResultKind::Problem { .. } => {
            let mut publication_changed = false;
            if prep::publication_state(m, &c)? == "experimental" {
                exact_test_publication(m, &c, expected_current)?;
                {
                    let _guard = m.lock("registry.lock")?;
                    m.require_inactive(None)?;
                }
                prep::record_guided_observations(m, &c, source_operation, &observations, detail)?;
                prep::review(m, &c, source_operation, prep::ReviewChoice::NeedsWork, detail)?;
                exact_test_publication(m, &c, expected_current)?;
                prep::disable_exact(m, &c, &publication_reference(expected_current))?;
                publication_changed = true;
            }
            guided_result_disposition(m, &c, expected_current, source_operation)?;
            prep::complete_guided_result(m, source_operation)?;
            Ok(json!({"candidate":candidate,"result":"needs_work","resumed":true,
                "publication_changed":publication_changed,"evidence_preserved":true}))
        }
    }
}

/// Inventory wins over candidate-bound history. Conflicting current inventory
/// cannot be resolved by iteration order. Historical selections only supply a
/// card anchor when no current selection exists; view() retains all generations.
fn product_selections(
    current: Vec<prep::Selection>,
    history: Vec<prep::Candidate>,
) -> Result<Vec<prep::Selection>> {
    let key = |s: &prep::Selection| {
        (
            s.environment.id.clone(),
            s.module.sha256.clone(),
            s.class.id.clone(),
        )
    };
    let mut products = std::collections::BTreeMap::new();
    for s in current {
        if let Some(old) = products.insert(key(&s), s.clone()) {
            require(old == s, "preparation_current_selection_ambiguous")?;
        }
    }
    for c in history {
        products.entry(key(&c.selection)).or_insert(c.selection);
    }
    Ok(products.into_values().collect())
}

fn publication_identity(r: &publication::RevisionRef) -> ui::PublicationIdentity {
    ui::PublicationIdentity {
        id: r.id.clone(),
        sha256: r.sha256.clone(),
    }
}
fn publication_reference(r: &ui::PublicationIdentity) -> publication::RevisionRef {
    publication::RevisionRef {
        id: r.id.clone(),
        sha256: r.sha256.clone(),
    }
}
fn publication_action(
    id: &str,
    state: &str,
    current: Option<&publication::RevisionRef>,
    reason: Option<&str>,
) -> Option<ui::AvailableAction> {
    let (label, action) = match state {
        "experimental" => (
            "Disable experimental use / restore previous publication".into(),
            ui::Action::ExperimentalDisable {
                candidate: id.into(),
            },
        ),
        "ordinary" => (
            "Withdraw ordinary publication (retain evidence)".into(),
            ui::Action::CandidateWithdraw {
                candidate: id.into(),
                expected_current: publication_identity(current?),
            },
        ),
        "another_configuration" => (
            format!(
                "Replace current revision {} with this candidate",
                current?.id
            ),
            ui::Action::ExperimentalReplace {
                candidate: id.into(),
                expected_current: publication_identity(current?),
            },
        ),
        "needs_attention" => return None,
        "unpublished" | "removed" => (
            "Enable experimental use in Bitwig".into(),
            ui::Action::ExperimentalEnable {
                candidate: id.into(),
            },
        ),
        _ => return None,
    };
    Some(ui::AvailableAction {
        label: format!("{} · candidate {}", label, &id[..12]),
        action,
        disabled_reason: reason.map(str::to_owned),
    })
}

pub fn is_action(a: &ui::Action) -> bool {
    matches!(
        a,
        ui::Action::PluginReinspect { .. }
            | ui::Action::CompatibilityCheck { .. }
            | ui::Action::CompatibilityResumeCheck { .. }
            | ui::Action::CompatibilityPublishTest { .. }
            | ui::Action::CompatibilityResult { .. }
            | ui::Action::CompatibilityFinishResult { .. }
            | ui::Action::ExperimentalReplace { .. }
            | ui::Action::CandidateWithdraw { .. }
            | ui::Action::PluginInspect { .. }
            | ui::Action::PluginPrepare { .. }
            | ui::Action::ExperimentalEnable { .. }
            | ui::Action::ExperimentalDisable { .. }
            | ui::Action::CandidateObserve { .. }
            | ui::Action::CandidateReview { .. }
            | ui::Action::CandidatePublishOrdinary { .. }
    )
}
pub fn offered(actual: &ui::Action, offer: &ui::Action) -> bool {
    match (actual, offer) {
        (
            ui::Action::CompatibilityResult {
                candidate, expected_current, result, passed, failed_area, note,
            },
            ui::Action::CompatibilityResult {
                candidate: offered_candidate, expected_current: offered_current, ..
            },
        ) => {
            candidate == offered_candidate && expected_current == offered_current
                && passed.len() <= prep::AREAS.len()
                && passed.iter().collect::<std::collections::BTreeSet<_>>().len() == passed.len()
                && match result {
                    ui::TestResultKind::Worked => !passed.is_empty() && failed_area.is_none()
                        && (note.is_empty() || text(note)),
                    ui::TestResultKind::Problem { category } => text(note)
                        && problem_area(*category).or(*failed_area).is_some()
                        && failed_area.is_none_or(|area| problem_area(*category).is_none_or(|known| known == area)),
                }
        }
        (
            ui::Action::InstallerRename { installer, label },
            ui::Action::InstallerRename { installer: offered, .. },
        ) => installer == offered && crate::installer_import::valid_label(label),
        (
            ui::Action::WorkspaceSelectInstaller { installer, release },
            ui::Action::WorkspaceSelectInstaller { installer: offered, .. },
        ) => installer == offered && daw_workspace::release_syntax(release),
        (
            ui::Action::WorkspaceSelectProductInstaller { product, installer, release },
            ui::Action::WorkspaceSelectProductInstaller { product: offered_product, installer: offered_installer, .. },
        ) => product == offered_product && installer == offered_installer
            && daw_workspace::release_syntax(release),
        (
            ui::Action::CandidateObserve {
                candidate,
                area,
                status,
                note,
            },
            ui::Action::CandidateObserve { candidate: c, .. },
        ) => {
            candidate == c
                && parse::<prep::Area>(area).is_ok()
                && parse::<prep::TestStatus>(status).is_ok()
                && text(note)
        }
        (
            ui::Action::CandidateReview {
                candidate,
                accept,
                rationale,
            },
            ui::Action::CandidateReview {
                candidate: c,
                accept: a,
                ..
            },
        ) => candidate == c && accept == a && text(rationale),
        _ => actual == offer,
    }
}
fn parse<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    Ok(serde_json::from_value(json!(s))?)
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 512 && !s.chars().any(char::is_control)
}
// Exact class selection must survive the manager-to-supervisor crossing. Module
// census uses first_audio separately; selecting an instrument may never guess.
fn selected_inspection_spec(m: &Manager, binding: HostBinding) -> Result<(SessionSpec, PathBuf)> {
    require(valid_hex(&binding.metadata.class_id, 32), "inspection_class_selection")?;
    spec(m, binding, true, false, false)
}
pub(super) fn admitted_inspection_spec(
    m: &Manager,
    binding: HostBinding,
    admission: impl FnOnce() -> Result<Lock>,
) -> Result<(SessionSpec, PathBuf)> {
    let guard = admission()?;
    guard.require_registry(m)?;
    m.require_inactive(None)?;
    selected_inspection_spec(m, binding)
}
pub fn inspect(
    m: &Manager,
    s: &prep::Selection,
    sw: &Software,
    audio_layout: Option<profiles::AudioLayoutPolicy>,
    admission: impl FnOnce() -> Result<Lock>,
) -> Result<prep::Inspection> {
    let i = inspect_unretained(m, s, sw, audio_layout, admission)?;
    prep::retain_inspection(m, &i)?;
    Ok(i)
}
fn inspect_unretained(
    m: &Manager,
    s: &prep::Selection,
    sw: &Software,
    audio_layout: Option<profiles::AudioLayoutPolicy>,
    admission: impl FnOnce() -> Result<Lock>,
) -> Result<prep::Inspection> {
    prep::verify_selection(m, s, &sw.host, &sw.source_sha256)?;
    let runtime = prep::build::stage_runtime(m)?;
    let stamp = observation::ModuleStamp::read(&s.module.path)?;
    let (job, path) = admitted_inspection_spec(
        m,
        HostBinding {
            metadata: ClassSelection {
                class_id: s.class.id.clone(),
            },
            environment: s.environment.clone(),
            module: s.module.clone(),
            host: runtime.host.clone(),
            host_source_sha256: runtime.source_manifest.sha256.clone(),
            compatibility: Compatibility {
                audio_layout: audio_layout.clone(),
                ..Compatibility::default()
            },
        },
        admission,
    )?;
    let pending = PendingAdmission::new(job.lease.clone(), Arc::new(AtomicBool::new(false)));
    let child = spawn(sw, &path, None)?;
    vendor_product_cli::finish_scan(child, &job, &path, pending)?;
    require(
        observation::ModuleStamp::read(&s.module.path)? == stamp,
        "inspection_module_changed",
    )?;
    let report = Artifact {
        sha256: digest(&job.report)?,
        path: job.report,
    };
    let i = prep::inspect_record_with_layout(
        s.clone(),
        report,
        prep::Origin::ManagedPreparation,
        runtime.host,
        runtime.source_manifest,
        audio_layout,
    )?;
    Ok(i)
}

fn check_stage(m: &Manager, operation: &str, stage: &str, action: &Value) -> Result<Option<Value>> {
    let record = prep::guided_check_stage(m, operation, stage)?;
    if let Some(record) = &record {
        require(record["schema"] == 1 && record["operation"] == operation
            && record["action"] == *action, "guided_check_stage_binding")?;
    }
    Ok(record.map(|record| record["value"].clone()))
}

fn retry_guided_inspection(
    m: &Manager, action: &Value, selection: &prep::Selection,
) -> Result<Option<prep::Inspection>> {
    let mut found = None;
    for row in prep::guided_checks(m, &selection.id()?)? {
        if row["stage"] != "candidate_preparation_failed" { continue; }
        let operation = row["operation"].as_str().ok_or("guided_check_operation")?;
        let Some(intent) = prep::guided_check_stage(m, operation, "intent")? else { continue };
        if intent["action"] != *action { continue; }
        let Some(stage) = check_stage(m, operation, "inspection", action)? else { continue };
        let inspection: prep::Inspection = serde_json::from_value(stage["inspection"].clone())?;
        if prep::exact_inspection(m, selection, &inspection.id()?).is_err() { continue; }
        if found.as_ref().is_some_and(|prior: &prep::Inspection| prior != &inspection) {
            return Ok(None);
        }
        found = Some(inspection);
    }
    Ok(found)
}

fn verify_guided_inspection(
    m: &Manager, s: &prep::Selection, i: &prep::Inspection,
    recipe: &str, audio_layout: &Option<profiles::AudioLayoutPolicy>,
) -> Result<()> {
    let runtime = prep::build::existing_runtime(m, recipe)?;
    require(i.selection == *s && i.audio_layout == *audio_layout
        && i.host == runtime.host && i.source_manifest == runtime.source_manifest,
        "guided_check_inspection_binding")?;
    require(prep::inspect_record_with_layout(s.clone(), i.report.clone(), i.origin.clone(),
        i.host.clone(), i.source_manifest.clone(), i.audio_layout.clone())? == *i,
        "guided_check_inspection_changed")?;
    i.report.verify()?;
    Ok(())
}

#[cfg(test)]
#[derive(Clone)]
struct GuidedCheckProbe {
    inspection: prep::Inspection,
    candidate: prep::Candidate,
    source_native: PathBuf,
    interrupt_after: Option<&'static str>,
    inspections: std::rc::Rc<std::cell::Cell<usize>>,
    builds: std::rc::Rc<std::cell::Cell<usize>>,
}
#[cfg(test)]
thread_local! {
    static GUIDED_CHECK_PROBE: std::cell::RefCell<Option<GuidedCheckProbe>> = const {
        std::cell::RefCell::new(None)
    };
}
#[cfg(test)]
fn probed_inspection() -> Option<prep::Inspection> {
    GUIDED_CHECK_PROBE.with(|slot| slot.borrow().as_ref().map(|probe| {
        probe.inspections.set(probe.inspections.get() + 1);
        probe.inspection.clone()
    }))
}
#[cfg(test)]
fn probed_candidate() -> Option<prep::Candidate> {
    GUIDED_CHECK_PROBE.with(|slot| slot.borrow().as_ref().map(|probe| {
        probe.builds.set(probe.builds.get() + 1);
        if probe.candidate.native.artifact.path != probe.source_native {
            let dir = probe.candidate.native.artifact.path.parent().unwrap();
            assert!(!dir.exists(), "preparation_operation_already_started");
            private_dir(dir).unwrap();
            fs::write(dir.join("build.log"), b"source-owned candidate construction").unwrap();
            private_dir(&dir.join("scratch")).unwrap();
            fs::write(dir.join("scratch/object.o"), b"compiler scratch").unwrap();
            if probe.interrupt_after == Some("construction") {
                panic!("source-owned interruption during construction");
            }
            fs::copy(&probe.source_native, &probe.candidate.native.artifact.path).unwrap();
            fs::set_permissions(&probe.candidate.native.artifact.path,
                fs::Permissions::from_mode(0o500)).unwrap();
            atomic_json(&dir.join("build.json"), &json!({
                "native_sha256":probe.candidate.native.artifact.sha256,
                "kit_sha256":probe.candidate.recipe_sha256,
            })).unwrap();
        }
        probe.candidate.clone()
    }))
}
#[cfg(test)]
fn probed_checkpoint(stage: &str) {
    let interrupt = GUIDED_CHECK_PROBE.with(|slot| slot.borrow().as_ref()
        .is_some_and(|probe| probe.interrupt_after == Some(stage)));
    if interrupt { panic!("source-owned interruption after {stage}"); }
}

fn guided_check<I, B>(
    m: &Manager, sw: &Software, action: &ui::Action, operation: &str,
    inspector: I, builder: B, mut checkpoint: impl FnMut(&str),
) -> Result<Value>
where
    I: FnOnce(&prep::Selection) -> Result<prep::Inspection>,
    B: FnOnce(&prep::Selection, &prep::Inspection, Option<&prep::Candidate>)
        -> Result<(prep::Candidate, bool)>,
{
    let ui::Action::CompatibilityCheck { selection, audio_layout, recipe, predecessor } = action
        else { return Err("guided_check_action".into()) };
    let s = prep::select(m, selection, &sw.host, &sw.source_sha256)?;
    require(prep::build::recipe(m)?.sha256 == *recipe, "preparation_recipe_changed")?;
    let prior = predecessor.as_ref().map(|id|
        prep::candidate(m, id, &sw.host, &sw.source_sha256)).transpose()?;
    if let Some(prior) = &prior {
        require(prep::same_product(&prior.selection, &s), "candidate_predecessor_selection")?;
    }
    let serialized = serde_json::to_value(action)?;
    if let Some(intent) = prep::guided_check_stage(m, operation, "intent")? {
        require(intent["schema"] == 1 && intent["operation"] == operation
            && intent["action"] == serialized, "guided_check_intent_changed")?;
    } else {
        prep::retain_guided_check_stage(m, operation, "intent", &serialized, &json!({}))?;
    }
    if let Some(done) = check_stage(m, operation, "completed", &serialized)? {
        if let Some(failure) = done["failure"].as_str() { return Err(failure.into()); }
        return Ok(done);
    }
    let run = || -> Result<Value> {
        let current = prep::view(m, &s, &sw.host, &sw.source_sha256)?;
        let owned_candidate = check_stage(m, operation, "candidate", &serialized)?;
        require(current.candidate == *predecessor
            || owned_candidate.as_ref().is_some_and(|stage|
                current.candidate.as_deref() == stage["id"].as_str()),
            "candidate_predecessor_changed")?;
        if owned_candidate.is_none() {
            if let Some(row) = current.candidates.iter().find(|row| row.current_inputs
                && row.id == current.candidate.as_deref().unwrap_or("")
                && row.recipe == *recipe
                && current.recommended_audio_layout == *audio_layout
                && row.review.as_ref().is_none_or(|review| review.choice != prep::ReviewChoice::NeedsWork)) {
                return Ok(json!({"stage":"candidate_reused","selection":selection,
                    "inspection":row.inspection,"candidate":row.id,"publication_changed":false}));
            }
        }
        let saved_inspection = check_stage(m, operation, "inspection", &serialized)?;
        require(owned_candidate.is_none() || saved_inspection.is_some(),
            "guided_check_candidate_without_inspection")?;
        let i = if let Some(stage) = saved_inspection {
            let i: prep::Inspection = serde_json::from_value(stage["inspection"].clone())?;
            require(stage["id"] == i.id()?, "guided_check_inspection_identity")?;
            verify_guided_inspection(m, &s, &i, recipe, audio_layout)?;
            prep::retain_inspection(m, &i)?;
            prep::exact_inspection(m, &s, &i.id()?)?
        } else {
            let reused = retry_guided_inspection(m, &serialized, &s)?;
            let i = match reused { Some(i) => i, None => inspector(&s)? };
            verify_guided_inspection(m, &s, &i, recipe, audio_layout)?;
            let id = i.id()?;
            prep::retain_guided_check_stage(m, operation, "inspection", &serialized,
                &json!({"id":id,"inspection":i}))?;
            checkpoint("inspection");
            prep::retain_inspection(m, &i)?;
            prep::exact_inspection(m, &s, &id)?
        };
        prep::verify_selection(m, &s, &sw.host, &sw.source_sha256)?;
        require(prep::build::recipe(m)?.sha256 == *recipe, "preparation_recipe_changed")?;
        let candidate_stage = check_stage(m, operation, "candidate", &serialized)?;
        let (c, artifact_reused) = if let Some(stage) = candidate_stage {
            let c: prep::Candidate = serde_json::from_value(stage["candidate"].clone())?;
            require(stage["id"] == c.id()? && c.selection == s && c.inspection == i
                && c.recipe_sha256 == *recipe, "guided_check_candidate_binding")?;
            (c, stage["artifact_reused"] == true)
        } else {
            require(prep::view(m, &s, &sw.host, &sw.source_sha256)?.candidate == *predecessor,
                "candidate_predecessor_changed")?;
            let (c, artifact_reused) = builder(&s, &i, prior.as_ref())?;
            require(c.selection == s && c.inspection == i && c.recipe_sha256 == *recipe,
                "guided_check_candidate_binding")?;
            prep::verify_candidate(m, &c, &sw.host, &sw.source_sha256)?;
            prep::retain_guided_check_stage(m, operation, "candidate", &serialized,
                &json!({"id":c.id()?,"candidate":c,"artifact_reused":artifact_reused}))?;
            checkpoint("candidate");
            (c, artifact_reused)
        };
        let basis = prep::preparation_basis(m, prior.as_ref())?;
        require(c.preparation_basis.as_deref() == Some(basis.as_str()),
            "guided_check_preparation_basis")?;
        prep::verify_candidate(m, &c, &sw.host, &sw.source_sha256)?;
        let _guard = m.lock("registry.lock")?;
        m.require_inactive(None)?;
        prep::retain_lineage(m, &c, operation, predecessor.as_deref())?;
        let lineage = prep::lineage(m, &c)?;
        require(lineage.preparation_identity == operation
            && lineage.predecessor == *predecessor, "guided_check_lineage_binding")?;
        let candidate = prep::record_candidate(m, &c)?;
        Ok(json!({"stage":"candidate_ready","selection":selection,
            "inspection":i.id()?,"candidate":candidate,"artifact_reused":artifact_reused,
            "publication_changed":false}))
    };
    let result = run();
    let finished = match &result {
        Ok(value) => {
            let mut record = value.clone();
            record["schema"] = json!(1);
            record["operation"] = json!(operation);
            record
        }
        Err(error) => {
            // Durable successful stages are recoverable; a failed inspector
            // cannot borrow a different historical inspection.
            let inspection = check_stage(m, operation, "inspection", &serialized)?;
            if inspection.is_some() && check_stage(m, operation, "candidate", &serialized)?.is_some() {
                return Err(error.to_string().into());
            }
            json!({"schema":1,"operation":operation,"selection":selection,
                "stage":if inspection.is_some() { "candidate_preparation_failed" } else { "inspection_failed" },
                "inspection":inspection.map(|stage| stage["id"].clone()),
                "candidate":null,"failure":error.to_string().chars().take(512).collect::<String>(),
                "publication_changed":false})
        }
    };
    prep::retain_guided_check_stage(m, operation, "completed", &serialized, &finished)?;
    result.map(|_| finished)
}
pub fn execute(
    m: &Manager,
    a: &ui::Action,
    operation: &str,
    registry_admission: impl FnOnce() -> Result<Lock>,
) -> Result<Value> {
    let sw = software(m)?;
    match a {
        ui::Action::PluginInspect {
            selection,
            audio_layout,
        }
        | ui::Action::PluginReinspect {
            selection,
            audio_layout,
        } => {
            let s = prep::select(m, selection, &sw.host, &sw.source_sha256)?;
            let i = inspect(m, &s, &sw, audio_layout.clone(), registry_admission)?;
            Ok(
                json!({"selection":selection,"inspection":"complete","controller":i.controller,"guarantee":false,"publication_changed":false}),
            )
        }
        ui::Action::CompatibilityCheck { .. }
        | ui::Action::CompatibilityResumeCheck { .. } => {
            let (check_action, source_operation) = match a {
                ui::Action::CompatibilityResumeCheck { operation: source } => {
                    let intent = prep::guided_check_stage(m, source, "intent")?
                        .ok_or("guided_check_resume_intent_missing")?;
                    let original: ui::Action = serde_json::from_value(intent["action"].clone())?;
                    require(operator_cli::resumable_check_source(m, source, &original)?,
                        "guided_check_source_still_running")?;
                    require(prep::guided_check_stage(m, source, "completed")?.is_none(),
                        "guided_check_already_completed")?;
                    (original, source.clone())
                }
                _ => (a.clone(), operation.to_owned()),
            };
            let ui::Action::CompatibilityCheck { audio_layout, recipe, .. } = &check_action
                else { return Err("guided_check_resume_action".into()) };
            guided_check(m, &sw, &check_action, &source_operation,
                |s| {
                    #[cfg(test)]
                    if let Some(inspection) = probed_inspection() { return Ok(inspection); }
                    inspect_unretained(m, s, &sw, audio_layout.clone(), registry_admission)
                },
                |s, i, prior| {
                    #[cfg(test)]
                    if let Some(candidate) = probed_candidate() { return Ok((candidate, false)); }
                    let basis = prep::preparation_basis(m, prior)?;
                    let reusable = prep::build::reusable(m, s, i, recipe)?;
                    let artifact_reused = reusable.is_some();
                    let c = match reusable {
                        Some(c) => c,
                        None => prep::build::construct(m, s.clone(), i.clone(), i.host.clone(),
                            i.source_manifest.clone(), &source_operation)?,
                    };
                    Ok((prep::bind_preparation_basis(c, Some(basis))?, artifact_reused))
                }, |stage| {
                    #[cfg(test)]
                    probed_checkpoint(stage);
                    #[cfg(not(test))]
                    let _ = stage;
                })
        }
        ui::Action::PluginPrepare {
            selection,
            inspection,
            recipe,
            predecessor,
        } => {
            let s = prep::select(m, selection, &sw.host, &sw.source_sha256)?;
            let i = prep::exact_inspection(m, &s, inspection)?;
            require(
                prep::build::recipe(m)?.sha256 == *recipe,
                "preparation_recipe_changed",
            )?;
            let prior = predecessor
                .as_ref()
                .map(|id| prep::candidate(m, id, &sw.host, &sw.source_sha256))
                .transpose()?;
            if let Some(c) = &prior {
                require(
                    prep::same_product(&c.selection, &s),
                    "candidate_predecessor_selection",
                )?;
            }
            let basis = prep::preparation_basis(m, prior.as_ref())?;
            let reusable = prep::build::reusable(m, &s, &i, recipe)?;
            let artifact_reused = reusable.is_some();
            let c = if let Some(c) = reusable {
                c
            } else {
                prep::build::construct(
                    m,
                    s,
                    i.clone(),
                    i.host.clone(),
                    i.source_manifest.clone(),
                    operation,
                )?
            };
            let c = prep::bind_preparation_basis(c, Some(basis))?;
            let candidate_reused = prep::retained_candidates(m)?.iter().any(|old| old == &c);
            prep::verify_candidate(m, &c, &sw.host, &sw.source_sha256)?;
            let _guard = m.lock("registry.lock")?;
            m.require_inactive(None)?;
            prep::retain_lineage(m, &c, operation, predecessor.as_deref())?;
            let id = prep::record_candidate(m, &c)?;
            Ok(
                json!({"selection":selection,"candidate":id,"prepared":true,"candidate_reused":candidate_reused,"artifact_reused":artifact_reused,"publication_changed":false}),
            )
        }
        ui::Action::ExperimentalReplace {
            candidate,
            expected_current,
        } => {
            let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
            let revision = prep::replace(m, &c, &publication_reference(expected_current))?;
            Ok(json!({"candidate":candidate,"replaced":expected_current,"publication":revision}))
        }
        ui::Action::CandidateWithdraw {
            candidate,
            expected_current,
        } => {
            let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
            prep::withdraw(m, &c, &publication_reference(expected_current))?;
            Ok(
                json!({"candidate":candidate,"withdrawn":expected_current,"evidence_preserved":true}),
            )
        }
        ui::Action::ExperimentalEnable { candidate }
        | ui::Action::CandidatePublishOrdinary { candidate } => {
            let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
            let ordinary = matches!(a, ui::Action::CandidatePublishOrdinary { .. });
            let r = prep::enable(m, &c, ordinary)?;
            Ok(
                json!({"candidate":candidate,"publication":r,"ordinary":ordinary,"experimental":!ordinary}),
            )
        }
        ui::Action::ExperimentalDisable { candidate } => {
            let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
            prep::disable(m, &c)?;
            Ok(json!({"candidate":candidate,"experimental":false,"history_preserved":true}))
        }
        ui::Action::CompatibilityPublishTest { candidate, expected_current } => {
            let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
            let state = prep::publication_state(m, &c)?;
            let revision = match (state.as_str(), expected_current) {
                ("experimental", _) => {
                    let current = prep::current_revision(m, &c.selection.class.id)?
                        .ok_or("candidate_publication_missing")?;
                    return Ok(json!({"candidate":candidate,"publication":current.id,
                        "already_available":true,"publication_changed":false}));
                }
                ("unpublished" | "removed", None) => prep::enable(m, &c, false)?,
                ("another_configuration", Some(expected)) =>
                    prep::replace(m, &c, &publication_reference(expected))?,
                _ => return Err("test_publication_requires_explicit_current_configuration".into()),
            };
            Ok(json!({"candidate":candidate,"publication":revision,
                "ordinary":false,"experimental":true}))
        }
        ui::Action::CompatibilityResult { .. } =>
            guided_result(m, &sw, a, operation, registry_admission),
        ui::Action::CompatibilityFinishResult { operation: source_operation } => {
            finish_guided_result(m, &sw, source_operation)
        }
        ui::Action::CandidateObserve {
            candidate,
            area,
            status,
            note,
        } => {
            let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
            prep::record_observation(m, &c, operation, parse(area)?, parse(status)?, note)?;
            Ok(
                json!({"candidate":candidate,"observation":"operator_observation","publication_changed":false}),
            )
        }
        ui::Action::CandidateReview {
            candidate,
            accept,
            rationale,
        } => {
            let c = prep::candidate(m, candidate, &sw.host, &sw.source_sha256)?;
            let decision = prep::review(
                m,
                &c,
                operation,
                if *accept {
                    prep::ReviewChoice::AcceptExactLocal
                } else {
                    prep::ReviewChoice::NeedsWork
                },
                rationale,
            )?;
            Ok(json!({"candidate":candidate,"review":decision,"publication_changed":false}))
        }
        _ => Err("not_preparation_action".into()),
    }
}

pub fn failure(action: &ui::Action) -> Option<Value> {
    let stage = match action {
        ui::Action::PluginInspect { .. } | ui::Action::PluginReinspect { .. } => {
            "preliminary_inspection"
        }
        ui::Action::CompatibilityCheck { .. }
        | ui::Action::CompatibilityResumeCheck { .. } => "guided_compatibility_check",
        ui::Action::PluginPrepare { .. } => "candidate_preparation",
        ui::Action::ExperimentalReplace { .. }
        | ui::Action::CandidateWithdraw { .. }
        | ui::Action::ExperimentalEnable { .. }
        | ui::Action::ExperimentalDisable { .. }
        | ui::Action::CompatibilityPublishTest { .. }
        | ui::Action::CandidatePublishOrdinary { .. } => "publication_transaction",
        ui::Action::CandidateObserve { .. } | ui::Action::CandidateReview { .. }
        | ui::Action::CompatibilityResult { .. }
        | ui::Action::CompatibilityFinishResult { .. } => {
            "qualification_record"
        }
        _ => return None,
    };
    Some(
        json!({"schema":1,"layer":"manager_control_plane","attempted_stage":stage,"result":"refused","automatic_retry":false,"installation_preserved":true,"prior_publication_provenance_preserved":true,"recovery":"Refresh canonical state; reconcile a pending publication before another explicit action. A refusal is not a plug-in crash."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn projection_fixture() -> (test_fixture::Fixture, prep::Candidate) {
        projection_fixture_with_role(false)
    }
    fn projection_fixture_with_role(effect: bool) -> (test_fixture::Fixture, prep::Candidate) {
        use observation::ModuleStamp;
        use prep::*;
        use profiles::Family;
        use test_fixture::{inspection_report, prepared_accessibility};
        let (mut f, _, mut census, mut native) = prepared_accessibility(false);
        f.m.unpublish(&f.r.key()).unwrap();
        atomic_json(&f.m.root.join("registry.json"), &Registry::default()).unwrap();
        let old = f.r.environment.root.clone();
        let id = "13".repeat(16);
        let envroot = f.m.root.join("environments").join(&id);
        fs::rename(&old, &envroot).unwrap();
        f.r.module.path = envroot.join(f.r.module.path.strip_prefix(&old).unwrap());
        f.r.environment.id = id;
        f.r.environment.root = envroot;
        atomic_json(
            &f.r.environment.root.join("environment.json"),
            &f.r.environment,
        )
        .unwrap();
        census.environment.environment = f.r.environment.clone();
        census.environment.family = Family::ManagedInstallerV1;
        census.module = f.r.module.clone();
        census.module_stamp = ModuleStamp::read(&census.module.path).unwrap();
        if effect {
            census.selected.subcategories = "Fx".into();
            native.class.subcategories = "Fx".into();
        }
        let mut raw = inspection_report(&census);
        // Complete non-audio factory class; the production inventory validates all rows.
        let class = raw["records"][1]["classes"][0].clone();
        let mut controller = class;
        controller["raw_tuid_hex"] = json!("02".repeat(16));
        controller["category_hex"] = json!(hex(b"Component Controller Class"));
        raw["records"][1]["classes"][1] = controller;
        raw["records"].as_array_mut().unwrap().push(
        json!({"state":"ap8_controller_association","combined":false,"class_id":"02".repeat(16)}),
    );
        atomic_json(&census.report.path, &raw).unwrap();
        census.report.sha256 = digest(&census.report.path).unwrap();
        let classes = linux_vst_bridge::inventory::classes(&raw).unwrap();
        let scan = linux_vst_bridge::inventory::Scan {
            schema: 1,
            id: random_id().unwrap(),
            environment: f.r.environment.clone(),
            host: f.r.host.clone(),
            host_source_sha256: f.r.host_source_sha256.clone(),
            completed_at: observation::now().unwrap(),
            modules: vec![linux_vst_bridge::inventory::Module {
                artifact: f.r.module.clone(),
                classes,
                report: census.report.clone(),
                inspection_error: None,
                quarantine_reason: None,
            }],
            changes: Default::default(),
        };
        private_dir(&f.m.root.join("inventory")).unwrap();
        atomic_json(
            &f.m.root
                .join("inventory")
                .join(format!("{}.json", f.r.environment.id)),
            &scan,
        )
        .unwrap();
        let s = selections(&f.m, &f.r.host, &f.r.host_source_sha256)
            .unwrap()
            .pop()
            .unwrap();
        let i = inspect_record(s.clone(), census.report, Origin::ManagedPreparation).unwrap();
        let manifest = Artifact {
            path: f.r.host.path.with_file_name("host-source-manifest.json"),
            sha256: f.r.host_source_sha256.clone(),
        };
        let c = prepared(s, i, native, f.r.host.clone(), manifest, "aa".repeat(32)).unwrap();
        (f, c)
    }
    #[test]
    fn effect_offers_explicit_stereo_inspection_without_changing_default_action_shape() {
        let (f, c) = projection_fixture_with_role(true);
        let mut products = vec![projection_product(&c)];
        project(&f.m, &projection_software(&c), &mut products, None).unwrap();
        let inspections: Vec<_> = products[0]
            .actions
            .iter()
            .filter(|action| matches!(action.action, ui::Action::PluginReinspect { .. }))
            .collect();
        assert_eq!(inspections.len(), 2);
        assert!(inspections.iter().any(|action| {
            action.label == "Check stereo compatibility"
                && matches!(
                    action.action,
                    ui::Action::PluginReinspect {
                        audio_layout: Some(profiles::AudioLayoutPolicy::StereoMainPair),
                        ..
                    }
                )
        }));
        let default = inspections
            .iter()
            .find(|action| action.label == "Check compatibility")
            .unwrap();
        assert!(serde_json::to_value(&default.action)
            .unwrap()
            .get("audio_layout")
            .is_none());
    }
    #[test]
    fn selected_inspection_preserves_exact_class_in_supervisor_job() {
        let f = test_fixture::Fixture::new();
        let expected = f.r.metadata.class_id.clone();
        let (job, path) = selected_inspection_spec(&f.m, f.r.clone().into()).unwrap();
        assert!(job.inspect);
        assert!(!job.first_audio, "selected instrument must not use factory-first guessing");
        assert_eq!(job.registration.metadata.class_id, expected);
        let saved: SessionSpec = read_json(&path).unwrap();
        assert!(!saved.first_audio);
        assert_eq!(saved.registration.metadata.class_id, expected);
        let mut missing: HostBinding = f.r.clone().into();
        missing.metadata.class_id.clear();
        assert!(selected_inspection_spec(&f.m, missing).is_err());
    }
    #[test]
    fn attention_and_alternative_never_offer_generic_enable() {
        let id = "ab".repeat(32);
        let r = publication::RevisionRef {
            id: "cd".repeat(16),
            sha256: "ef".repeat(32),
        };
        assert!(publication_action(&id, "needs_attention", Some(&r), None).is_none());
        let offered = publication_action(&id, "another_configuration", Some(&r), None).unwrap();
        assert_eq!(
            offered.action,
            ui::Action::ExperimentalReplace {
                candidate: id.clone(),
                expected_current: publication_identity(&r)
            }
        );
        let mut wrong = r.clone();
        wrong.id = "00".repeat(16);
        assert!(!super::offered(
            &ui::Action::ExperimentalReplace {
                candidate: id.clone(),
                expected_current: publication_identity(&wrong)
            },
            &offered.action
        ));
        assert!(matches!(
            publication_action(&id, "ordinary", Some(&r), None)
                .unwrap()
                .action,
            ui::Action::CandidateWithdraw { .. }
        ));
        assert!(publication_action(&id, "unknown", Some(&r), None).is_none());
    }
    #[test]
    fn observation_is_closed_and_product_bound() {
        let offered = ui::Action::CandidateObserve {
            candidate: "ab".repeat(32),
            area: "editor".into(),
            status: "not_tested".into(),
            note: String::new(),
        };
        let make = |candidate: String, area: &str, status: &str| ui::Action::CandidateObserve {
            candidate,
            area: area.into(),
            status: status.into(),
            note: "I observed the exact candidate editor".into(),
        };
        assert!(super::offered(
            &make("ab".repeat(32), "editor", "passed"),
            &offered
        ));
        assert!(!super::offered(
            &make("cd".repeat(32), "editor", "passed"),
            &offered
        ));
        assert!(!super::offered(
            &make("ab".repeat(32), "shell", "passed"),
            &offered
        ));
        assert!(!super::offered(
            &make("ab".repeat(32), "editor", "supported"),
            &offered
        ));
        assert!(!super::offered(&offered, &offered));
    }
    #[test]
    fn ordinary_review_is_explicit_and_separate() {
        let offer = ui::Action::CandidateReview {
            candidate: "ab".repeat(32),
            accept: false,
            rationale: String::new(),
        };
        assert!(!super::offered(
            &ui::Action::CandidateReview {
                candidate: "ab".repeat(32),
                accept: true,
                rationale: "Reviewed all evidence".into()
            },
            &offer
        ));
        assert!(!super::offered(
            &ui::Action::CandidatePublishOrdinary {
                candidate: "ab".repeat(32)
            },
            &offer
        ));
    }
    #[test]
    fn keeper_and_maintenance_inspector_keep_independent_host_bindings() {
        let (f, _, _, _) = test_fixture::prepared();
        let id = "ac".repeat(16);
        let root = f.m.root.join("environments").join(&id);
        let env = Environment {
            id: id.clone(),
            root,
            revision: 1,
            runner: f.r.environment.runner.clone(),
        };
        let drive = env.root.join("compatdata/pfx/drive_c");
        private_dir(&drive).unwrap();
        atomic_json(&env.root.join("environment.json"), &env).unwrap();
        let module = Artifact {
            path: drive.join("module.vst3"),
            sha256: f.r.module.sha256.clone(),
        };
        fs::copy(&f.r.module.path, &module.path).unwrap();
        private_dir(&onboarding::directory(&f.m, &id).unwrap()).unwrap();
        atomic_json(
            &onboarding::directory(&f.m, &id)
                .unwrap()
                .join("record.json"),
            &onboarding::Record {
                schema: 1,
                id: id.clone(),
                installer: "ab".repeat(32),
                environment: env.clone(),
                created_at: 1,
                creation_operation: "cd".repeat(16),
                installation_operation: None,
                published: false,
                previous_attempt: None,
            },
        )
        .unwrap();
        let a = f.r.host.clone();
        let sw = Software {
            installer_launch: None,
            preparation_kit: None,
            manager: a.clone(),
            operator_frontend: None,
            supervisor: a.clone(),
            ownership: a.clone(),
            host: a.clone(),
            source_manifest: Artifact {
                path: a.path.with_file_name("host-source-manifest.json"),
                sha256: f.r.host_source_sha256.clone(),
            },
            source_sha256: f.r.host_source_sha256.clone(),
            native_catalogue: None,
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let original = HostBinding {
            metadata: ClassSelection {
                class_id: f.r.key(),
            },
            environment: env,
            module,
            host: a,
            host_source_sha256: f.r.host_source_sha256.clone(),
            compatibility: Compatibility::default(),
        };
        assert!(spec(&f.m, original.clone(), true, false, true).is_ok());
        let mut newer = original;
        newer.host.sha256 = "de".repeat(32);
        assert!(spec(&f.m, newer.clone(), true, false, true).is_err());
        assert!(spec(&f.m, newer.clone(), true, true, false).is_ok());
        assert!(spec(&f.m, newer, false, false, false).is_err());
    }
    fn projection_software(c: &prep::Candidate) -> Software {
        Software {
            installer_launch: None,
            preparation_kit: None,
            manager: c.host.clone(),
            operator_frontend: None,
            supervisor: c.host.clone(),
            ownership: c.host.clone(),
            host: c.host.clone(),
            source_manifest: c.source_manifest.clone(),
            source_sha256: c.source_manifest.sha256.clone(),
            native_catalogue: None,
        }
    }
    fn projection_product(c: &prep::Candidate) -> ui::Product {
        let s = &c.selection;
        ui::Product {
            class_id: s.class.id.clone(),
            name: s.class.name.clone(),
            vendor: s.class.vendor.clone(),
            role: s.class.role.clone(),
            version: s.class.version.clone(),
            disposition: "installed_unqualified".into(),
            active_revision: None,
            recommended_revision: None,
            environment: s.environment.id.clone(),
            runner: s.environment.runner.id.clone(),
            module_sha256: s.module.sha256.clone(),
            limitations: vec![],
            history: vec![],
            actions: vec![],
            compatibility: None,
            details: json!({}),
        }
    }
    fn projection_kit(f: &test_fixture::Fixture, c: &prep::Candidate) -> Software {
        let mut sw = projection_software(c);
        let path = f.m.root.join("software/ui2-projection-kit.zip");
        private_dir(path.parent().unwrap()).unwrap();
        fs::write(&path, b"source-owned test kit identity").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        sw.preparation_kit = Some(Artifact { sha256: digest(&path).unwrap(), path });
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        sw
    }
    fn guided_fixture() -> (test_fixture::Fixture, prep::Candidate, Software, ui::Action) {
        let (f, mut c) = projection_fixture_with_role(true);
        let mut sw = projection_kit(&f, &c);
        let recipe = sw.preparation_kit.as_ref().unwrap().sha256.clone();
        let root = f.m.root.join("software/preparation-kits").join(&recipe);
        private_dir(&root).unwrap();
        let host = root.join("host.exe");
        let source = root.join("host-source-manifest.json");
        fs::copy(&c.host.path, &host).unwrap();
        fs::copy(&c.source_manifest.path, &source).unwrap();
        fs::set_permissions(&host, fs::Permissions::from_mode(0o400)).unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o400)).unwrap();
        let runtime = prep::build::Runtime {
            kit: sw.preparation_kit.clone().unwrap(),
            host: Artifact { sha256: digest(&host).unwrap(), path: host },
            source_manifest: Artifact { sha256: digest(&source).unwrap(), path: source },
            builder: None, generator: None,
        };
        atomic_json(&root.join("runtime.json"), &runtime).unwrap();
        c.inspection.host = runtime.host.clone();
        c.inspection.source_manifest = runtime.source_manifest.clone();
        c.host = runtime.host;
        c.source_manifest = runtime.source_manifest;
        c.recipe_sha256 = recipe.clone();
        let basis = prep::preparation_basis(&f.m, None).unwrap();
        c = prep::bind_preparation_basis(c, Some(basis)).unwrap();
        let action = ui::Action::CompatibilityCheck {
            selection: c.selection.id().unwrap(), audio_layout: None,
            recipe, predecessor: None,
        };
        // The operator snapshot discovers this unqualified product through
        // its exact managed environment and inventory, with no publication.
        let catalogue = linux_vst_bridge::catalogue::Catalogue {
            schema: 3, natives: vec![c.native.clone()],
            environments: vec![linux_vst_bridge::catalogue::EnvironmentBinding {
                family: profiles::Family::ManagedInstallerV1,
                environment: c.selection.environment.clone(),
            }],
            hosts: vec![], onboarding_runtime: None,
        };
        let path = f.m.root.join("software/catalogue.json");
        atomic_json(&path, &catalogue).unwrap();
        sw.native_catalogue = Some(Artifact { sha256: digest(&path).unwrap(), path });
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        (f, c, sw, action)
    }
    #[test]
    fn ui2_interrupted_checks_resume_through_fresh_offered_operator_requests() {
        use std::{cell::Cell, panic::{catch_unwind, AssertUnwindSafe}, rc::Rc};
        for boundary in ["inspection", "candidate"] {
            let (f, c, _sw, check) = guided_fixture();
            let before = fs::read(f.m.root.join("registry.json")).unwrap();
            let inspections = Rc::new(Cell::new(0));
            let builds = Rc::new(Cell::new(0));
            let source = operator_cli::test_submit_offered(&f.m, &check).unwrap();
            let mut owned = c.clone();
            owned.native.artifact.path = f.m.root.join("preparation/work")
                .join(&source).join("native.so");
            GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = Some(GuidedCheckProbe {
                inspection: c.inspection.clone(), candidate: owned.clone(),
                source_native: c.native.artifact.path.clone(),
                interrupt_after: Some(boundary), inspections: inspections.clone(),
                builds: builds.clone(),
            }));
            assert!(catch_unwind(AssertUnwindSafe(||
                operator_cli::test_run_offered_worker(&f.m, &source).unwrap())).is_err());
            let old = operator_cli::test_finalize_interrupted_worker(&f.m, &source).unwrap();
            assert_eq!(old["state"], "refused");
            assert_eq!(inspections.get(), 1);
            assert_eq!(builds.get(), usize::from(boundary == "candidate"));
            if boundary == "candidate" {
                // The installed private UI2 generation retained schema-11
                // requests. Only the new continuation uses current schema 10.
                let request_path = f.m.root.join("operator").join(&source).join("request.json");
                let mut historical: ui::Request = read_json(&request_path).unwrap();
                historical.schema = 11;
                atomic_json(&request_path, &historical).unwrap();
            }
            if boundary == "candidate" {
                assert!(owned.native.artifact.path.exists(),
                    "retirement must preserve the checkpointed native output");
            }
            GUIDED_CHECK_PROBE.with(|slot| slot.borrow_mut().as_mut().unwrap().interrupt_after = None);
            let snapshot = operator_cli::snapshot_idle_test(&f.m).unwrap();
            let resume = snapshot.products.iter()
                .find(|p| p.class_id == c.selection.class.id)
                .and_then(|p| p.compatibility.as_ref())
                .and_then(|workflow| workflow.primary.as_ref()).unwrap();
            assert_eq!(resume.action, ui::Action::CompatibilityResumeCheck {
                operation: source.clone(),
            });
            assert!(resume.disabled_reason.is_none());
            let next = operator_cli::test_submit_offered(&f.m, &resume.action).unwrap();
            assert_ne!(next, source);
            let result = operator_cli::test_run_offered_worker(&f.m, &next).unwrap();
            assert_eq!(result["state"], "completed", "{result:?}");
            assert_eq!(result["result"]["candidate"], owned.id().unwrap());
            assert_eq!(inspections.get(), 1);
            assert_eq!(builds.get(), 1);
            assert_eq!(prep::retained_candidates(&f.m).unwrap().len(), 1);
            assert_eq!(prep::publication_state(&f.m, &owned).unwrap(), "unpublished");
            assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
            assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &source).unwrap(), old);
            GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = None);
        }
    }
    fn interrupted_check_fixture() -> (test_fixture::Fixture, prep::Candidate, String, ui::Action) {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let (f, c, _sw, check) = guided_fixture();
        GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = Some(GuidedCheckProbe {
            inspection: c.inspection.clone(), candidate: c.clone(),
            source_native: c.native.artifact.path.clone(),
            interrupt_after: Some("inspection"),
            inspections: std::rc::Rc::new(std::cell::Cell::new(0)),
            builds: std::rc::Rc::new(std::cell::Cell::new(0)),
        }));
        let source = operator_cli::test_submit_offered(&f.m, &check).unwrap();
        assert!(catch_unwind(AssertUnwindSafe(||
            operator_cli::test_run_offered_worker(&f.m, &source).unwrap())).is_err());
        assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &source)
            .unwrap()["state"], "refused");
        GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = None);
        (f, c, source, check)
    }
    fn submit_check_continuation(m: &Manager, c: &prep::Candidate, source: &str) -> String {
        let snapshot = operator_cli::snapshot_idle_test(m).unwrap();
        let offered = snapshot.products.iter().find(|p| p.class_id == c.selection.class.id)
            .and_then(|p| p.compatibility.as_ref())
            .and_then(|workflow| workflow.primary.as_ref()).unwrap();
        assert_eq!(offered.label, "Finish compatibility check");
        assert_eq!(offered.action, ui::Action::CompatibilityResumeCheck { operation: source.into() });
        assert!(offered.disabled_reason.is_none());
        operator_cli::test_submit_offered(m, &offered.action).unwrap()
    }
    #[test]
    fn ui2_successful_check_continuation_finalizer_cleans_original_source_scratch() {
        use std::{cell::Cell, rc::Rc};
        let (f, c, source, _check) = interrupted_check_fixture();
        let original_receipt = f.m.root.join("operator").join(&source).join("result.json");
        let before_receipt = fs::read(&original_receipt).unwrap();
        let before_registry = fs::read(f.m.root.join("registry.json")).unwrap();
        let dir = f.m.root.join("preparation/work").join(&source);
        let mut owned = c.clone();
        owned.native.artifact.path = dir.join("native.so");
        let inspections = Rc::new(Cell::new(1));
        let builds = Rc::new(Cell::new(0));
        GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = Some(GuidedCheckProbe {
            inspection: c.inspection.clone(), candidate: owned.clone(),
            source_native: c.native.artifact.path.clone(), interrupt_after: None,
            inspections: inspections.clone(), builds: builds.clone(),
        }));
        let continuation = submit_check_continuation(&f.m, &c, &source);
        let result = operator_cli::test_run_offered_worker(&f.m, &continuation).unwrap();
        assert_eq!(result["state"], "completed", "{result:?}");
        assert_eq!(result["result"]["candidate"], owned.id().unwrap());
        assert!(dir.join("scratch/object.o").exists());
        let build_before = fs::read(dir.join("build.json")).unwrap();
        let fresh_dir = f.m.root.join("preparation/work").join(&continuation);
        private_dir(&fresh_dir).unwrap();
        fs::write(fresh_dir.join("scratch"), b"fresh worker scratch").unwrap();
        assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &continuation).unwrap(), result);
        assert!(!fresh_dir.exists());
        assert!(!dir.join("scratch").exists());
        assert!(!dir.join("build.log").exists());
        assert_eq!(fs::read(dir.join("build.json")).unwrap(), build_before);
        owned.native.artifact.verify().unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        assert_eq!(inspections.get(), 1);
        assert_eq!(builds.get(), 1);
        assert_eq!(prep::retained_candidates(&f.m).unwrap(), vec![owned.clone()]);
        assert_eq!(prep::publication_state(&f.m, &owned).unwrap(), "unpublished");
        assert_eq!(fs::read(&original_receipt).unwrap(), before_receipt);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before_registry);
        assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &continuation).unwrap(), result);
        GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = None);
    }
    #[test]
    fn ui2_second_interruption_during_check_continuation_remains_recoverable() {
        use std::{cell::Cell, panic::{catch_unwind, AssertUnwindSafe}, rc::Rc};
        let (f, c, source, _check) = interrupted_check_fixture();
        let original_receipt = f.m.root.join("operator").join(&source).join("result.json");
        let before_receipt = fs::read(&original_receipt).unwrap();
        let before_registry = fs::read(f.m.root.join("registry.json")).unwrap();
        let dir = f.m.root.join("preparation/work").join(&source);
        let mut owned = c.clone();
        owned.native.artifact.path = dir.join("native.so");
        let inspections = Rc::new(Cell::new(1));
        let builds = Rc::new(Cell::new(0));
        GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = Some(GuidedCheckProbe {
            inspection: c.inspection.clone(), candidate: owned.clone(),
            source_native: c.native.artifact.path.clone(), interrupt_after: Some("construction"),
            inspections: inspections.clone(), builds: builds.clone(),
        }));
        let interrupted = submit_check_continuation(&f.m, &c, &source);
        assert!(catch_unwind(AssertUnwindSafe(||
            operator_cli::test_run_offered_worker(&f.m, &interrupted).unwrap())).is_err());
        assert!(dir.join("scratch/object.o").exists());
        assert!(prep::guided_check_stage(&f.m, &source, "candidate").unwrap().is_none());
        let refused = operator_cli::test_finalize_interrupted_worker(&f.m, &interrupted).unwrap();
        assert_eq!(refused["state"], "refused");
        assert!(!dir.exists(), "partial source work must not block the next construction");
        let failure: Value = read_json(&f.m.root.join("preparation/failures")
            .join(&source).join("build.log.json")).unwrap();
        assert_eq!(failure, "source-owned candidate construction");
        GUIDED_CHECK_PROBE.with(|slot| slot.borrow_mut().as_mut().unwrap().interrupt_after = None);
        let next = submit_check_continuation(&f.m, &c, &source);
        assert_ne!(next, interrupted);
        let result = operator_cli::test_run_offered_worker(&f.m, &next).unwrap();
        assert_eq!(result["state"], "completed", "{result:?}");
        assert_eq!(result["result"]["candidate"], owned.id().unwrap());
        assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &next).unwrap(), result);
        assert_eq!(inspections.get(), 1);
        assert_eq!(builds.get(), 2, "only the interrupted construction must restart");
        assert_eq!(prep::retained_candidates(&f.m).unwrap(), vec![owned.clone()]);
        owned.native.artifact.verify().unwrap();
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        assert_eq!(prep::publication_state(&f.m, &owned).unwrap(), "unpublished");
        assert_eq!(fs::read(&original_receipt).unwrap(), before_receipt);
        assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &interrupted).unwrap(), refused);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before_registry);
        GUIDED_CHECK_PROBE.with(|slot| *slot.borrow_mut() = None);
    }
    #[test]
    fn ui2_interrupted_check_refuses_changed_recipe_and_selection() {
        let (f, c, source, _check) = interrupted_check_fixture();
        let resume = ui::Action::CompatibilityResumeCheck { operation: source.clone() };
        let mut sw = software(&f.m).unwrap();
        let path = f.m.root.join("software/new-test-recipe.zip");
        fs::write(&path, b"different immutable source-owned recipe").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        sw.preparation_kit = Some(Artifact { sha256: digest(&path).unwrap(), path });
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let snapshot = operator_cli::snapshot_idle_test(&f.m).unwrap();
        let pending = snapshot.products.iter().find(|p| p.class_id == c.selection.class.id)
            .unwrap().compatibility.as_ref().unwrap().primary.as_ref().unwrap();
        assert_eq!(pending.action, resume);
        assert!(pending.disabled_reason.as_deref().unwrap().contains("recipe changed"));
        assert!(operator_cli::test_submit_offered(&f.m, &resume).is_err());
        assert_eq!(execute(&f.m, &resume, &random_id().unwrap(),
            || f.m.lock("registry.lock")).unwrap_err().to_string(),
            "preparation_recipe_changed");

        let (f, c, source, _check) = interrupted_check_fixture();
        let resume = ui::Action::CompatibilityResumeCheck { operation: source };
        let path = f.m.root.join("inventory").join(format!("{}.json", c.selection.environment.id));
        let mut scan: inventory::Scan = read_json(&path).unwrap();
        let exact = scan.modules[0].classes.iter_mut()
            .find(|class| class.id == c.selection.class.id).unwrap();
        exact.version.push_str(".changed");
        atomic_json(&path, &scan).unwrap();
        assert!(operator_cli::test_submit_offered(&f.m, &resume).is_err());
        assert_eq!(execute(&f.m, &resume, &random_id().unwrap(),
            || f.m.lock("registry.lock")).unwrap_err().to_string(),
            "preparation_selection_stale_or_ambiguous");
    }
    #[test]
    fn ui2_multiple_interrupted_checks_refuse_ambiguous_continuation() {
        let (f, c, source, check) = interrupted_check_fixture();
        let second = random_id().unwrap();
        prep::retain_guided_check_stage(&f.m, &second, "intent",
            &serde_json::to_value(check).unwrap(), &json!({})).unwrap();
        let snapshot = operator_cli::snapshot_idle_test(&f.m).unwrap();
        let workflow = snapshot.products.iter().find(|p| p.class_id == c.selection.class.id)
            .unwrap().compatibility.as_ref().unwrap();
        assert_eq!(workflow.phase, ui::CompatibilityPhase::PublicationNeedsAttention);
        assert!(workflow.primary.is_none());
        assert!(operator_cli::test_submit_offered(&f.m,
            &ui::Action::CompatibilityResumeCheck { operation: source }).is_err());
    }
    fn offered_finish(m: &Manager, candidate: &str, source: &str) -> Value {
        let snapshot = operator_cli::snapshot_idle_test(m).unwrap();
        let offered = snapshot.products.iter().find(|p| p.compatibility.as_ref()
            .and_then(|workflow| workflow.current_candidate.as_deref()) == Some(candidate))
            .or_else(|| snapshot.products.iter().find(|p| p.details["preparation"]["guided_results"]
                .as_array().is_some_and(|rows| rows.iter().any(|row|
                    row["operation"] == source))))
            .and_then(|p| p.compatibility.as_ref())
            .and_then(|workflow| workflow.primary.as_ref()).unwrap();
        assert_eq!(offered.action, ui::Action::CompatibilityFinishResult {
            operation: source.into(),
        });
        assert!(offered.disabled_reason.is_none());
        let request = operator_cli::test_submit_offered(m, &offered.action).unwrap();
        let receipt = operator_cli::test_run_offered_worker(m, &request).unwrap();
        assert_eq!(receipt["state"], "completed", "{receipt:?}");
        receipt["result"].clone()
    }
    #[test]
    fn ui2_offered_finish_recovers_success_at_intent_observation_and_review() {
        for stage in ["intent", "observation", "review"] {
            let (f, c, _sw, _) = guided_fixture();
            prep::record_candidate(&f.m, &c).unwrap();
            prep::enable(&f.m, &c, false).unwrap();
            let candidate = c.id().unwrap();
            let current = publication_identity(f.m.registry().unwrap().classes
                [&c.selection.class.id].managed_revision.as_ref().unwrap());
            let action = ui::Action::CompatibilityResult {
                candidate: candidate.clone(), expected_current: current,
                result: ui::TestResultKind::Worked,
                passed: vec![ui::TestArea::DawLoad, ui::TestArea::Midi,
                    ui::TestArea::Audio, ui::TestArea::Editor,
                    ui::TestArea::Parameters, ui::TestArea::Automation,
                    ui::TestArea::StateRecall, ui::TestArea::ProcessingRestart,
                    ui::TestArea::Retirement],
                failed_area: None, note: "Exact DAW test passed".into(),
            };
            let source = operator_cli::test_submit_offered(&f.m, &action).unwrap();
            prep::retain_guided_result(&f.m, &source,
                &serde_json::to_value(&action).unwrap()).unwrap();
            if stage != "intent" {
                let (entries, detail) = guided_result_entries(&action).unwrap();
                prep::record_guided_observations(&f.m, &c, &source, &entries, detail).unwrap();
            }
            if stage == "review" {
                prep::review(&f.m, &c, &source,
                    prep::ReviewChoice::AcceptExactLocal, "Exact DAW test passed").unwrap();
            }
            assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &source)
                .unwrap()["state"], "refused");
            let before_registry = fs::read(f.m.root.join("registry.json")).unwrap();
            let finished = offered_finish(&f.m, &candidate, &source);
            assert_eq!(finished["result"], "passed_experimental");
            assert_eq!(finished["publication_changed"], false);
            assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before_registry);
            assert_eq!(prep::observations(&f.m, &c).unwrap().iter()
                .filter(|row| row.operation == source).count(), 9);
            assert_eq!(prep::decisions(&f.m, &c).unwrap().iter()
                .filter(|row| row.operation == source).count(), 1);
            assert_eq!(prep::guided_results(&f.m, &candidate).unwrap()[0]["completed"], true);
            assert_eq!(finish_guided_result(&f.m, &software(&f.m).unwrap(), &source)
                .unwrap()["result"], "already_recorded");
        }
    }
    #[test]
    fn ui2_offered_finish_keeps_partial_success_experimental() {
        let (f, c, _sw, _) = guided_fixture();
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let candidate = c.id().unwrap();
        let current = publication_identity(f.m.registry().unwrap().classes
            [&c.selection.class.id].managed_revision.as_ref().unwrap());
        let action = ui::Action::CompatibilityResult {
            candidate: candidate.clone(), expected_current: current,
            result: ui::TestResultKind::Worked,
            passed: vec![ui::TestArea::DawLoad], failed_area: None,
            note: "Loaded in Bitwig".into(),
        };
        let source = operator_cli::test_submit_offered(&f.m, &action).unwrap();
        prep::retain_guided_result(&f.m, &source,
            &serde_json::to_value(action).unwrap()).unwrap();
        operator_cli::test_finalize_interrupted_worker(&f.m, &source).unwrap();
        let finished = offered_finish(&f.m, &candidate, &source);
        assert_eq!(finished["result"], "partial_experimental");
        assert_eq!(finished["ordinary_published"], false);
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "experimental");
        assert!(prep::decisions(&f.m, &c).unwrap().is_empty());
    }
    #[test]
    fn ui2_offered_finish_recovers_problem_at_each_retained_stage() {
        for stage in ["intent", "observation", "review", "disposition"] {
            let (f, c, _sw, _) = guided_fixture();
            prep::record_candidate(&f.m, &c).unwrap();
            prep::enable(&f.m, &c, false).unwrap();
            let candidate = c.id().unwrap();
            let current = publication_identity(f.m.registry().unwrap().classes
                [&c.selection.class.id].managed_revision.as_ref().unwrap());
            let action = ui::Action::CompatibilityResult {
                candidate: candidate.clone(), expected_current: current.clone(),
                result: ui::TestResultKind::Problem {
                    category: ui::ProblemCategory::BlankEditor,
                },
                passed: vec![ui::TestArea::DawLoad], failed_area: None,
                note: "White editor while audio continued".into(),
            };
            let source = operator_cli::test_submit_offered(&f.m, &action).unwrap();
            prep::retain_guided_result(&f.m, &source,
                &serde_json::to_value(&action).unwrap()).unwrap();
            if stage != "intent" {
                let (entries, detail) = guided_result_entries(&action).unwrap();
                prep::record_guided_observations(&f.m, &c, &source, &entries, detail).unwrap();
            }
            if matches!(stage, "review" | "disposition") {
                prep::review(&f.m, &c, &source, prep::ReviewChoice::NeedsWork,
                    "White editor while audio continued").unwrap();
            }
            if stage == "disposition" {
                prep::disable_exact(&f.m, &c, &publication_reference(&current)).unwrap();
            }
            assert_eq!(operator_cli::test_finalize_interrupted_worker(&f.m, &source)
                .unwrap()["state"], "refused");
            let before_registry = fs::read(f.m.root.join("registry.json")).unwrap();
            let finished = offered_finish(&f.m, &candidate, &source);
            assert_eq!(finished["result"], "needs_work");
            assert_eq!(finished["publication_changed"], stage != "disposition");
            if stage == "disposition" {
                assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before_registry);
            }
            assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "removed");
            assert_eq!(prep::observations(&f.m, &c).unwrap().iter()
                .filter(|row| row.operation == source).count(), 2);
            assert_eq!(prep::decisions(&f.m, &c).unwrap().iter()
                .filter(|row| row.operation == source).count(), 1);
            assert_eq!(prep::guided_results(&f.m, &candidate).unwrap()[0]["completed"], true);
        }
    }
    #[test]
    fn ui2_offered_finish_after_restoration_preserves_accepted_ancestor() {
        let (f, a, _sw, _) = guided_fixture();
        prep::record_candidate(&f.m, &a).unwrap();
        for area in prep::AREAS {
            prep::record_observation(&f.m, &a, &random_id().unwrap(), area,
                prep::TestStatus::Passed, "Source-owned accepted ancestor").unwrap();
        }
        prep::review(&f.m, &a, &random_id().unwrap(),
            prep::ReviewChoice::AcceptExactLocal, "Exact accepted ancestor").unwrap();
        let ordinary = prep::enable(&f.m, &a, true).unwrap();
        let basis = prep::preparation_basis(&f.m, Some(&a)).unwrap();
        let b = prep::bind_preparation_basis(a.clone(), Some(basis)).unwrap();
        prep::record_candidate_with_predecessor(&f.m, &b, Some(&a.id().unwrap())).unwrap();
        let test_revision = prep::replace(&f.m, &b, &ordinary).unwrap();
        let action = ui::Action::CompatibilityResult {
            candidate: b.id().unwrap(), expected_current: publication_identity(&test_revision),
            result: ui::TestResultKind::Problem {
                category: ui::ProblemCategory::BlankEditor,
            }, passed: vec![], failed_area: None,
            note: "Editor remained blank".into(),
        };
        let source = operator_cli::test_submit_offered(&f.m, &action).unwrap();
        prep::retain_guided_result(&f.m, &source,
            &serde_json::to_value(&action).unwrap()).unwrap();
        let (entries, detail) = guided_result_entries(&action).unwrap();
        prep::record_guided_observations(&f.m, &b, &source, &entries, detail).unwrap();
        prep::review(&f.m, &b, &source, prep::ReviewChoice::NeedsWork, detail).unwrap();
        prep::disable_exact(&f.m, &b, &test_revision).unwrap();
        operator_cli::test_finalize_interrupted_worker(&f.m, &source).unwrap();
        let registry = fs::read(f.m.root.join("registry.json")).unwrap();
        let observations = prep::observations(&f.m, &b).unwrap();
        let reviews = prep::decisions(&f.m, &b).unwrap();
        let finished = offered_finish(&f.m, &b.id().unwrap(), &source);
        assert_eq!(finished["result"], "needs_work");
        assert_eq!(finished["publication_changed"], false);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry);
        assert_eq!(prep::observations(&f.m, &b).unwrap(), observations);
        assert_eq!(prep::decisions(&f.m, &b).unwrap(), reviews);
        assert_eq!(f.m.registry().unwrap().classes[&a.selection.class.id].managed_revision,
            Some(ordinary));
    }
    #[test]
    fn ui2_interrupted_after_inspection_checkpoint_resumes_without_inspector() {
        use std::{cell::Cell, panic::{catch_unwind, AssertUnwindSafe}};
        let (f, c, sw, action) = guided_fixture();
        let operation = random_id().unwrap();
        let inspections = Cell::new(0);
        let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
        let interrupted = catch_unwind(AssertUnwindSafe(|| {
            let _ = guided_check(&f.m, &sw, &action, &operation,
                |_| { inspections.set(inspections.get() + 1); Ok(c.inspection.clone()) },
                |_, _, _| panic!("candidate work must not start before the checkpoint"),
                |stage| if stage == "inspection" { panic!("interrupted after inspection") });
        }));
        assert!(interrupted.is_err());
        assert_eq!(inspections.get(), 1);
        let result = guided_check(&f.m, &sw, &action, &operation,
            |_| panic!("retained inspection must not rerun"),
            |_, _, _| Ok((c.clone(), false)), |_| {}).unwrap();
        assert_eq!(result["stage"], "candidate_ready");
        assert_eq!(result["candidate"], c.id().unwrap());
        assert_eq!(prep::inspections(&f.m, &c.selection).unwrap().len(), 1);
        assert_eq!(prep::retained_candidates(&f.m).unwrap().len(), 1);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry_before);
    }
    #[test]
    fn ui2_interrupted_after_candidate_checkpoint_resumes_without_rebuild() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let (f, c, sw, action) = guided_fixture();
        let operation = random_id().unwrap();
        let interrupted = catch_unwind(AssertUnwindSafe(|| {
            let _ = guided_check(&f.m, &sw, &action, &operation,
                |_| Ok(c.inspection.clone()),
                |_, _, _| Ok((c.clone(), false)),
                |stage| if stage == "candidate" { panic!("interrupted after candidate") });
        }));
        assert!(interrupted.is_err());
        assert!(prep::retained_candidates(&f.m).unwrap().is_empty());
        let result = guided_check(&f.m, &sw, &action, &operation,
            |_| panic!("inspector must not rerun"),
            |_, _, _| panic!("candidate must not rebuild"), |_| {}).unwrap();
        assert_eq!(result["stage"], "candidate_ready");
        assert_eq!(result["candidate"], c.id().unwrap());
        assert_eq!(prep::retained_candidates(&f.m).unwrap().len(), 1);
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "unpublished");
        assert_eq!(prep::lineage(&f.m, &c).unwrap().preparation_identity, operation);
        assert_eq!(prep::guided_checks(&f.m, &c.selection.id().unwrap()).unwrap()[0]["stage"],
            "candidate_ready");
        assert_eq!(prep::guided_checks(&f.m, &c.selection.id().unwrap()).unwrap()[0]["operation"],
            operation);
        fs::remove_file(f.m.root.join("preparation/guided-checks")
            .join(&operation).join("completed.json")).unwrap();
        let replay = guided_check(&f.m, &sw, &action, &operation,
            |_| panic!("inspector must not rerun after candidate retention"),
            |_, _, _| panic!("candidate must not rebuild after candidate retention"), |_| {})
            .unwrap();
        assert_eq!(replay["candidate"], c.id().unwrap());
    }
    #[test]
    fn ui2_failed_candidate_preparation_retries_exact_inspection_once() {
        let (f, c, sw, action) = guided_fixture();
        let first = random_id().unwrap();
        let second = random_id().unwrap();
        assert_eq!(guided_check(&f.m, &sw, &action, &first,
            |_| Ok(c.inspection.clone()),
            |_, _, _| Err("source_owned_candidate_build_failed".into()), |_| {})
            .unwrap_err().to_string(), "source_owned_candidate_build_failed");
        let checks = prep::guided_checks(&f.m, &c.selection.id().unwrap()).unwrap();
        assert_eq!(checks[0]["stage"], "candidate_preparation_failed");
        assert_eq!(checks[0]["inspection"], c.inspection.id().unwrap());
        let result = guided_check(&f.m, &sw, &action, &second,
            |_| panic!("exact retained inspection should be reused"),
            |_, _, _| Ok((c.clone(), false)), |_| {}).unwrap();
        assert_eq!(result["stage"], "candidate_ready");
        assert_eq!(prep::inspections(&f.m, &c.selection).unwrap().len(), 1);
        assert_eq!(prep::retained_candidates(&f.m).unwrap().len(), 1);
    }
    #[test]
    fn ui2_unchecked_product_offers_one_exact_check_without_publication() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_kit(&f, &c);
        let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
        let mut products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        let workflow = products[0].compatibility.as_ref().unwrap();
        assert_eq!(workflow.phase, ui::CompatibilityPhase::NotChecked);
        let primary = workflow.primary.as_ref().unwrap();
        assert_eq!(primary.label, "Check compatibility");
        assert_eq!(primary.action, ui::Action::CompatibilityCheck {
            selection: c.selection.id().unwrap(), audio_layout: None,
            recipe: sw.preparation_kit.unwrap().sha256, predecessor: None,
        });
        assert!(workflow.alternatives.is_empty());
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry_before);
        assert!(prep::candidates(&f.m, &c.host, &c.source_manifest.sha256).unwrap().is_empty());
    }
    #[test]
    fn ui2_worker_validation_does_not_hide_its_own_check_offer() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_kit(&f, &c);
        let mut products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        let offered = products[0].compatibility.as_ref().unwrap().primary.as_ref().unwrap().action.clone();
        let operation = random_id().unwrap();
        let job = f.m.root.join("operator").join(&operation);
        private_dir(&job).unwrap();
        atomic_json(&job.join("request.json"), &json!({"schema":ui::OPERATOR_SCHEMA,
            "state_token":"source-owned-check", "action":offered})).unwrap();
        atomic_json(&job.join("result.json"), &json!({"schema":1,
            "operation":operation,"state":"waiting","stage":"operator_validation_readback"})).unwrap();
        products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products[0].compatibility.as_ref().unwrap().phase,
            ui::CompatibilityPhase::Checking);
        assert!(products[0].compatibility.as_ref().unwrap().primary.is_none());
        products = vec![projection_product(&c)];
        project_for_operation(&f.m, &sw, &mut products, None, &operation).unwrap();
        let validating = products[0].compatibility.as_ref().unwrap();
        assert_eq!(validating.phase, ui::CompatibilityPhase::NotChecked);
        assert_eq!(validating.primary.as_ref().unwrap().action, offered);
        products = vec![projection_product(&c)];
        project_for_operation(&f.m, &sw, &mut products, None, &random_id().unwrap()).unwrap();
        assert_eq!(products[0].compatibility.as_ref().unwrap().phase,
            ui::CompatibilityPhase::Checking);
        assert!(prep::candidates(&f.m, &c.host, &c.source_manifest.sha256).unwrap().is_empty());
    }
    #[test]
    fn ui2_failed_new_check_never_claims_an_older_inspection() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_kit(&f, &c);
        prep::retain_inspection(&f.m, &c.inspection).unwrap();
        let old_inspection = c.inspection.id().unwrap();
        let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
        let operation = random_id().unwrap();
        let action = ui::Action::CompatibilityCheck {
            selection: c.selection.id().unwrap(), audio_layout: None,
            recipe: sw.preparation_kit.unwrap().sha256, predecessor: None,
        };
        assert!(execute(&f.m, &action, &operation,
            || Err("source_owned_inspection_refused".into())).is_err());
        let checks = prep::guided_checks(&f.m, &c.selection.id().unwrap()).unwrap();
        let attempt = checks.iter().find(|row| row["operation"] == operation).unwrap();
        assert_eq!(attempt["stage"], "inspection_failed");
        assert!(attempt["inspection"].is_null());
        assert_ne!(attempt["inspection"], old_inspection);
        assert!(attempt["candidate"].is_null());
        assert_eq!(attempt["publication_changed"], false);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry_before);
    }
    #[test]
    fn ui2_exact_current_candidate_is_reused_without_another_inspector() {
        let (f, mut c) = projection_fixture_with_role(true);
        let sw = projection_kit(&f, &c);
        let recipe = sw.preparation_kit.unwrap().sha256;
        c.recipe_sha256 = recipe.clone();
        prep::record_candidate(&f.m, &c).unwrap();
        let candidate = c.id().unwrap();
        let operation = random_id().unwrap();
        let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
        let action = ui::Action::CompatibilityCheck {
            selection: c.selection.id().unwrap(), audio_layout: None,
            recipe, predecessor: Some(candidate.clone()),
        };
        let result = execute(&f.m, &action, &operation,
            || panic!("an exact current candidate must not rerun inspection")).unwrap();
        assert_eq!(result["stage"], "candidate_reused");
        assert_eq!(result["candidate"], candidate);
        assert_eq!(result["publication_changed"], false);
        assert_eq!(prep::retained_candidates(&f.m).unwrap().len(), 1);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry_before);
    }
    #[test]
    fn ui2_completed_result_marker_must_match_its_exact_intent() {
        let f = test_fixture::Fixture::new();
        let operation = random_id().unwrap();
        let candidate = "ab".repeat(32);
        let action = ui::Action::CompatibilityResult {
            candidate: candidate.clone(),
            expected_current: ui::PublicationIdentity {
                id: "cd".repeat(16), sha256: "ef".repeat(32),
            },
            result: ui::TestResultKind::Worked, passed: vec![ui::TestArea::DawLoad],
            failed_area: None, note: String::new(),
        };
        prep::retain_guided_result(&f.m, &operation, &serde_json::to_value(action).unwrap()).unwrap();
        prep::complete_guided_result(&f.m, &operation).unwrap();
        assert_eq!(prep::guided_results(&f.m, &candidate).unwrap()[0]["completed"], true);
        let marker = f.m.root.join("preparation/guided-results")
            .join(&operation).join("completed.json");
        fs::remove_file(&marker).unwrap();
        fs::write(&marker, b"{\"schema\":1,\"operation\":\"wrong\"}").unwrap();
        assert_eq!(prep::guided_results(&f.m, &candidate).unwrap_err().to_string(),
            "guided_result_completion_binding");
    }
    #[test]
    fn ui2_prepared_test_result_and_needs_work_keep_exact_history() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_software(&c);
        prep::record_candidate(&f.m, &c).unwrap();
        let candidate = c.id().unwrap();
        let mut products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        let workflow = products[0].compatibility.as_ref().unwrap();
        assert_eq!(workflow.phase, ui::CompatibilityPhase::ReadyForTest);
        assert!(matches!(workflow.primary.as_ref().unwrap().action,
            ui::Action::CompatibilityPublishTest { .. }));
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "unpublished");
        prep::enable(&f.m, &c, false).unwrap();
        let expected = publication_identity(f.m.registry().unwrap().classes
            [&c.selection.class.id].managed_revision.as_ref().unwrap());
        let first = random_id().unwrap();
        let worked = ui::Action::CompatibilityResult {
            candidate: candidate.clone(), expected_current: expected.clone(),
            result: ui::TestResultKind::Worked,
            passed: vec![ui::TestArea::DawLoad, ui::TestArea::Editor],
            failed_area: None, note: String::new(),
        };
        let partial = guided_result(&f.m, &sw, &worked, &first,
            || f.m.lock("registry.lock")).unwrap();
        assert_eq!(partial["result"], "partial_experimental");
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "experimental");
        assert!(prep::decisions(&f.m, &c).unwrap().is_empty());
        assert_eq!(prep::observations(&f.m, &c).unwrap().iter()
            .filter(|observation| observation.operation == first).count(), 2);
        assert_eq!(guided_result(&f.m, &sw, &worked, &first,
            || f.m.lock("registry.lock")).unwrap()["result"],
            "already_recorded");
        products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products[0].compatibility.as_ref().unwrap().phase,
            ui::CompatibilityPhase::AvailableForTest);
        let negative = random_id().unwrap();
        let problem = ui::Action::CompatibilityResult {
            candidate: candidate.clone(), expected_current: expected.clone(),
            result: ui::TestResultKind::Problem { category: ui::ProblemCategory::BlankEditor },
            passed: vec![], failed_area: None, note: "Editor was white".into(),
        };
        let result = guided_result(&f.m, &sw, &problem, &negative,
            || f.m.lock("registry.lock")).unwrap();
        assert_eq!(result["result"], "awaiting_retirement");
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "experimental");
        assert!(prep::decisions(&f.m, &c).unwrap().is_empty());
        assert_eq!(prep::observations(&f.m, &c).unwrap().iter()
            .filter(|observation| observation.operation == negative).count(), 1);
        assert!(prep::guided_results(&f.m, &candidate).unwrap().iter()
            .any(|row| row["operation"] == negative && row["completed"] == false));
        products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products[0].compatibility.as_ref().unwrap().phase,
            ui::CompatibilityPhase::AwaitingRetirement);
        let finished = finish_guided_result(&f.m, &sw, &negative).unwrap();
        assert_eq!(finished["result"], "needs_work");
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "removed");
        assert_eq!(prep::decisions(&f.m, &c).unwrap().last().unwrap().choice,
            prep::ReviewChoice::NeedsWork);
        fs::remove_file(f.m.root.join("preparation/guided-results")
            .join(&negative).join("completed.json")).unwrap();
        products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        let pending = products[0].compatibility.as_ref().unwrap();
        assert_eq!(pending.phase, ui::CompatibilityPhase::TestResultIncomplete);
        assert!(matches!(&pending.primary.as_ref().unwrap().action,
            ui::Action::CompatibilityFinishResult { operation } if operation == &negative));
        let resumed = finish_guided_result(&f.m, &sw, &negative).unwrap();
        assert_eq!(resumed["result"], "needs_work");
        assert_eq!(resumed["resumed"], true);
        assert!(prep::guided_results(&f.m, &candidate).unwrap().iter()
            .any(|row| row["operation"] == negative && row["completed"] == true));
        products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products[0].compatibility.as_ref().unwrap().phase,
            ui::CompatibilityPhase::NeedsWork);
        assert_eq!(products[0].details["preparation"]["candidates"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn ui2_success_result_waits_for_a_polling_registry_before_recording() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_software(&c);
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let candidate = c.id().unwrap();
        let expected = publication_identity(f.m.registry().unwrap().classes
            [&c.selection.class.id].managed_revision.as_ref().unwrap());
        let operation = random_id().unwrap();
        let action = ui::Action::CompatibilityResult {
            candidate: candidate.clone(), expected_current: expected,
            result: ui::TestResultKind::Worked,
            passed: vec![ui::TestArea::DawLoad],
            failed_area: None, note: "Exact visible DAW load".into(),
        };
        let mut attempts = 0;
        let held = f.m.lock("registry.lock").unwrap();
        let result = std::thread::scope(|scope| {
            scope.spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(100));
                drop(held);
            });
            guided_result(&f.m, &sw, &action, &operation, || {
                let (guard, facts) = f.m.lock_bounded(
                    ui::OperatorLock::Registry,
                    ui::LockPurpose::OperatorValidationReadback,
                    Some(&operation),
                    std::time::Duration::from_secs(2),
                )?;
                attempts = facts.attempts;
                Ok(guard)
            })
        }).unwrap();
        assert!(attempts > 1, "the final inactivity recheck must wait for readback");
        assert_eq!(result["result"], "partial_experimental");
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "experimental");
        assert!(prep::guided_results(&f.m, &candidate).unwrap().iter()
            .any(|row| row["operation"] == operation && row["completed"] == true));
        assert_eq!(prep::observations(&f.m, &c).unwrap().iter()
            .filter(|row| row.operation == operation).count(), 1);
    }
    #[test]
    fn ui2_restored_ancestor_keeps_historical_pending_finish_visible() {
        let (f, a) = projection_fixture_with_role(true);
        let sw = projection_software(&a);
        prep::record_candidate(&f.m, &a).unwrap();
        for area in prep::AREAS {
            prep::record_observation(&f.m, &a, &random_id().unwrap(), area,
                prep::TestStatus::Passed, "Generated exact accepted ancestor").unwrap();
        }
        prep::review(&f.m, &a, &random_id().unwrap(),
            prep::ReviewChoice::AcceptExactLocal, "Generated exact acceptance").unwrap();
        let ordinary = prep::enable(&f.m, &a, true).unwrap();
        let basis = prep::preparation_basis(&f.m, Some(&a)).unwrap();
        let b = prep::bind_preparation_basis(a.clone(), Some(basis)).unwrap();
        assert_ne!(a.id().unwrap(), b.id().unwrap());
        prep::record_candidate_with_predecessor(&f.m, &b, Some(&a.id().unwrap())).unwrap();
        let test_revision = prep::replace(&f.m, &b, &ordinary).unwrap();
        let expected = publication_identity(&test_revision);
        let operation = random_id().unwrap();
        let problem = ui::Action::CompatibilityResult {
            candidate: b.id().unwrap(), expected_current: expected.clone(),
            result: ui::TestResultKind::Problem { category: ui::ProblemCategory::BlankEditor },
            passed: vec![], failed_area: None, note: "Editor remained blank".into(),
        };
        assert_eq!(guided_result(&f.m, &sw, &problem, &operation,
            || f.m.lock("registry.lock")).unwrap()["result"],
            "awaiting_retirement");
        prep::review(&f.m, &b, &operation, prep::ReviewChoice::NeedsWork,
            "Editor remained blank").unwrap();
        prep::disable_exact(&f.m, &b, &test_revision).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&a.selection.class.id].managed_revision,
            Some(ordinary.clone()));
        assert_eq!(prep::publication_state(&f.m, &b).unwrap(), "another_configuration");
        let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
        let observations_before = prep::observations(&f.m, &b).unwrap();
        let reviews_before = prep::decisions(&f.m, &b).unwrap();
        let mut products = vec![projection_product(&a)];
        project(&f.m, &sw, &mut products, None).unwrap();
        let pending = products[0].compatibility.as_ref().unwrap();
        assert_eq!(pending.current_candidate.as_deref(), Some(a.id().unwrap().as_str()));
        assert_eq!(pending.phase, ui::CompatibilityPhase::TestResultIncomplete);
        assert!(pending.summary.contains("previous working configuration is selected"));
        assert!(matches!(&pending.primary.as_ref().unwrap().action,
            ui::Action::CompatibilityFinishResult { operation: source } if source == &operation));
        assert_eq!(pending.primary.as_ref().unwrap().disabled_reason, None);
        assert!(products[0].details["preparation"]["guided_results"].as_array().unwrap()
            .iter().any(|row| row["operation"] == operation && row["completed"] == false));
        assert_eq!(finish_guided_result(&f.m, &sw, &operation).unwrap()["result"], "needs_work");
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry_before);
        assert_eq!(prep::observations(&f.m, &b).unwrap(), observations_before);
        assert_eq!(prep::decisions(&f.m, &b).unwrap(), reviews_before);
        assert_eq!(f.m.registry().unwrap().classes[&a.selection.class.id].managed_revision,
            Some(ordinary));
        assert_eq!(prep::publication_state(&f.m, &b).unwrap(), "another_configuration");
        assert!(prep::guided_results(&f.m, &b.id().unwrap()).unwrap().iter()
            .any(|row| row["operation"] == operation && row["completed"] == true));
        products = vec![projection_product(&a)];
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products[0].compatibility.as_ref().unwrap().phase,
            ui::CompatibilityPhase::OrdinarySupported);
        assert_eq!(products[0].disposition, "ready");
    }
    #[test]
    fn ui2_active_problem_is_recorded_before_safe_publication_disposition() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_software(&c);
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let expected = publication_identity(f.m.registry().unwrap().classes
            [&c.selection.class.id].managed_revision.as_ref().unwrap());
        let operation = random_id().unwrap();
        let action = ui::Action::CompatibilityResult {
            candidate: c.id().unwrap(), expected_current: expected,
            result: ui::TestResultKind::Problem { category: ui::ProblemCategory::BlankEditor },
            passed: vec![ui::TestArea::DawLoad], failed_area: None,
            note: "Editor remained blank".into(),
        };
        let sid = random_id().unwrap();
        let report = f.m.root.join("runtime/results").join(format!("{sid}.json"));
        let lease = f.m.root.join("runtime/leases").join(format!("{sid}.json"));
        private_dir(report.parent().unwrap()).unwrap();
        private_dir(lease.parent().unwrap()).unwrap();
        atomic_json(&lease, &report).unwrap();
        let owner = c.selection.environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&sid).join("owner.json");
        private_dir(owner.parent().unwrap()).unwrap();
        atomic_json(&owner, &json!({"session":sid,"report":report,
            "keeper":false,"registration":{"metadata":{"class_id":c.selection.class.id}}})).unwrap();
        assert!(f.m.require_inactive(None).is_err());
        let mut products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, Some("Close active bridged instances")).unwrap();
        assert!(products[0].compatibility.as_ref().unwrap().primary.as_ref().unwrap()
            .disabled_reason.is_none());
        let before = fs::read(f.m.root.join("registry.json")).unwrap();
        let result = guided_result(&f.m, &sw, &action, &operation,
            || f.m.lock("registry.lock")).unwrap();
        assert_eq!(result["result"], "awaiting_retirement");
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
        assert!(prep::decisions(&f.m, &c).unwrap().is_empty());
        assert_eq!(prep::observations(&f.m, &c).unwrap().iter()
            .filter(|row| row.operation == operation).count(), 2);
        assert_eq!(finish_guided_result(&f.m, &sw, &operation).unwrap_err().to_string(),
            "active_device_lease");
        fs::remove_file(&lease).unwrap();
        let finished = finish_guided_result(&f.m, &sw, &operation).unwrap();
        assert_eq!(finished["result"], "needs_work");
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "removed");
    }
    #[test]
    fn ui2_cleanup_incomplete_report_retains_only_evidence() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_software(&c);
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let expected = publication_identity(f.m.registry().unwrap().classes
            [&c.selection.class.id].managed_revision.as_ref().unwrap());
        let operation = random_id().unwrap();
        let action = ui::Action::CompatibilityResult {
            candidate: c.id().unwrap(), expected_current: expected,
            result: ui::TestResultKind::Problem {
                category: ui::ProblemCategory::CleanupIncomplete,
            },
            passed: vec![], failed_area: None,
            note: "The host did not retire after closing the DAW".into(),
        };
        let mut products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, Some("Previous instance cleanup is unconfirmed")).unwrap();
        assert!(products[0].compatibility.as_ref().unwrap().primary.as_ref().unwrap()
            .disabled_reason.is_none());
        let before = fs::read(f.m.root.join("registry.json")).unwrap();
        assert_eq!(guided_result(&f.m, &sw, &action, &operation,
            || f.m.lock("registry.lock")).unwrap()["result"],
            "awaiting_retirement");
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
        assert!(prep::guided_result_intent(&f.m, &operation).is_ok());
        assert!(prep::observations(&f.m, &c).unwrap().iter()
            .any(|row| row.operation == operation && row.area == prep::Area::Retirement
                && row.status == prep::TestStatus::Failed));
        assert!(prep::decisions(&f.m, &c).unwrap().is_empty());
        assert_eq!(prep::guided_results(&f.m, &c.id().unwrap()).unwrap()[0]["completed"], false);
    }
    #[test]
    fn ui2_publication_drift_before_finish_preserves_problem_and_newer_state() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_software(&c);
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let expected = publication_identity(f.m.registry().unwrap().classes
            [&c.selection.class.id].managed_revision.as_ref().unwrap());
        let operation = random_id().unwrap();
        let action = ui::Action::CompatibilityResult {
            candidate: c.id().unwrap(), expected_current: expected,
            result: ui::TestResultKind::Problem { category: ui::ProblemCategory::BlankEditor },
            passed: vec![], failed_area: None, note: "Blank editor".into(),
        };
        guided_result(&f.m, &sw, &action, &operation,
            || f.m.lock("registry.lock")).unwrap();
        prep::disable(&f.m, &c).unwrap();
        let newer = prep::enable(&f.m, &c, false).unwrap();
        let ui::Action::CompatibilityResult { expected_current, .. } = &action else { unreachable!() };
        assert_ne!(newer.id, expected_current.id);
        let changed = fs::read(f.m.root.join("registry.json")).unwrap();
        assert!(finish_guided_result(&f.m, &sw, &operation).is_err());
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), changed);
        assert!(prep::decisions(&f.m, &c).unwrap().is_empty());
        assert!(prep::observations(&f.m, &c).unwrap().iter()
            .any(|row| row.operation == operation && row.status == prep::TestStatus::Failed));
        assert_eq!(prep::guided_results(&f.m, &c.id().unwrap()).unwrap()[0]["completed"], false);
        let mut products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        let pending = products[0].compatibility.as_ref().unwrap();
        assert_eq!(pending.phase, ui::CompatibilityPhase::PublicationNeedsAttention);
        assert!(pending.summary.contains("cannot be confirmed"));
        assert!(matches!(&pending.primary.as_ref().unwrap().action,
            ui::Action::CompatibilityFinishResult { operation: source } if source == &operation));
        assert!(pending.primary.as_ref().unwrap().disabled_reason.is_some());
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), changed);
    }
    #[test]
    fn ui2_wrong_publication_refuses_before_observation_or_review() {
        let (f, c) = projection_fixture();
        let sw = projection_software(&c);
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let mut expected = publication_identity(f.m.registry().unwrap().classes
            [&c.selection.class.id].managed_revision.as_ref().unwrap());
        expected.id = "00".repeat(16);
        let operation = random_id().unwrap();
        private_dir(&f.m.root.join("preparation/observations")).unwrap();
        let before = test_fixture::snapshot(&f.m.root.join("preparation/observations"));
        let problem = ui::Action::CompatibilityResult {
            candidate: c.id().unwrap(), expected_current: expected,
            result: ui::TestResultKind::Problem { category: ui::ProblemCategory::BlankEditor },
            passed: vec![], failed_area: None, note: "White editor".into(),
        };
        let error = guided_result(&f.m, &sw, &problem, &operation,
            || f.m.lock("registry.lock")).unwrap_err().to_string();
        assert_eq!(error, "guided_test_publication_changed");
        assert_eq!(test_fixture::snapshot(&f.m.root.join("preparation/observations")), before);
        assert!(prep::guided_results(&f.m, &c.id().unwrap()).unwrap().is_empty());
        assert_eq!(prep::publication_state(&f.m, &c).unwrap(), "experimental");
    }
    #[test]
    fn ui2_current_experimental_candidate_does_not_inherit_older_needs_work() {
        let (f, c) = projection_fixture();
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let sw = projection_software(&c);
        let mut view = prep::view(&f.m, &c.selection, &sw.host, &sw.source_sha256).unwrap();
        let mut prior = view.candidates[0].clone();
        prior.id = "ac".repeat(32);
        prior.publication = "removed".into();
        prior.disposition = "superseded".into();
        prior.review = Some(prep::Decision {
            schema: 1, candidate: prior.id.clone(), operation: random_id().unwrap(),
            evidence_sha256: "ba".repeat(32), choice: prep::ReviewChoice::NeedsWork,
            rationale: "Retained white editor result".into(), authority: "fixture".into(),
            recorded_at: 1, ordinal: 1, ordinary_profile: None,
        });
        view.candidates.insert(0, prior);
        let workflow = guided_projection(&f.m, &c.selection, &view, None, None, false, false).unwrap();
        assert_eq!(workflow.phase, ui::CompatibilityPhase::AvailableForTest);
        assert_eq!(workflow.current_candidate.as_deref(), Some(c.id().unwrap().as_str()));
        assert!(matches!(workflow.primary.unwrap().action, ui::Action::CompatibilityResult { .. }));
    }
    #[test]
    fn ui2_two_exact_effect_layouts_offer_only_bounded_human_choices() {
        let (f, c) = projection_fixture_with_role(true);
        let sw = projection_kit(&f, &c);
        prep::retain_inspection(&f.m, &c.inspection).unwrap();
        let mut view = prep::view(&f.m, &c.selection, &sw.host, &sw.source_sha256).unwrap();
        view.inspections[0].recommended = true;
        view.recommended_inspection = Some(view.inspections[0].id.clone());
        let mut stereo = view.inspections[0].clone();
        stereo.id = "cd".repeat(32);
        stereo.audio_layout = Some(profiles::AudioLayoutPolicy::StereoMainPair);
        stereo.recommended = false;
        view.inspections.push(stereo);
        let workflow = guided_projection(&f.m, &c.selection, &view, None, None, false, false).unwrap();
        assert_eq!(workflow.phase, ui::CompatibilityPhase::NotChecked);
        assert!(workflow.primary.is_none());
        assert_eq!(workflow.alternatives.len(), 2);
        assert_eq!(workflow.alternatives[0].label, "Stereo effect");
        assert_eq!(workflow.alternatives[1].label, "Use the plug-in's reported default layout");
        assert!(matches!(workflow.alternatives[0].action,
            ui::Action::CompatibilityCheck { audio_layout: Some(profiles::AudioLayoutPolicy::StereoMainPair), .. }));
        assert!(matches!(workflow.alternatives[1].action,
            ui::Action::CompatibilityCheck { audio_layout: None, .. }));
    }
    #[test]
    fn ui2_result_payload_cannot_replace_manager_candidate_or_claim_unobserved_checks() {
        let expected = ui::PublicationIdentity { id: "ab".repeat(16), sha256: "cd".repeat(32) };
        let offered = ui::Action::CompatibilityResult {
            candidate: "ef".repeat(32), expected_current: expected.clone(),
            result: ui::TestResultKind::Worked, passed: vec![], failed_area: None,
            note: String::new(),
        };
        let valid = ui::Action::CompatibilityResult {
            candidate: "ef".repeat(32), expected_current: expected.clone(),
            result: ui::TestResultKind::Worked, passed: vec![ui::TestArea::Editor],
            failed_area: None, note: String::new(),
        };
        assert!(super::offered(&valid, &offered));
        let mut wrong = valid.clone();
        if let ui::Action::CompatibilityResult { candidate, .. } = &mut wrong {
            *candidate = "00".repeat(32);
        }
        assert!(!super::offered(&wrong, &offered));
        wrong = valid.clone();
        if let ui::Action::CompatibilityResult { expected_current, .. } = &mut wrong {
            expected_current.id = "00".repeat(16);
        }
        assert!(!super::offered(&wrong, &offered));
        wrong = valid.clone();
        if let ui::Action::CompatibilityResult { passed, .. } = &mut wrong {
            passed.push(ui::TestArea::Editor);
        }
        assert!(!super::offered(&wrong, &offered));
        wrong = valid;
        if let ui::Action::CompatibilityResult { passed, .. } = &mut wrong { passed.clear(); }
        assert!(!super::offered(&wrong, &offered));
    }
    #[test]
    fn ordinary_registry_card_without_mf3_candidate_keeps_its_authority() {
        let (f, c) = projection_fixture();
        let sw = projection_software(&c);
        let mut product = projection_product(&c);
        product.disposition = "ready".into();
        product.active_revision = Some(18);
        let ordinary = ui::AvailableAction {
            label: "Rollback to exact ordinary".into(),
            action: ui::Action::OrdinaryRollback {
                class_id: c.selection.class.id.clone(),
                publication: "cd".repeat(16),
            },
            disabled_reason: None,
        };
        product.actions.push(ordinary.clone());
        let facts = json!({"id":"ef".repeat(16),"revision":18});
        product.details = json!({"profile":{"claim":"verified_exact_fixture"},"publication":facts});
        let mut products = vec![product];
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products.len(), 1);
        let p = &products[0];
        assert_eq!(p.disposition, "ready");
        assert_eq!(p.active_revision, Some(18));
        assert_eq!(p.details["publication"], facts);
        assert_eq!(p.details["preparation"]["publication"], "ordinary");
        assert!(p
            .actions
            .iter()
            .any(|a| serde_json::to_value(a).unwrap() == serde_json::to_value(&ordinary).unwrap()));
        assert!(!p
            .actions
            .iter()
            .any(|a| matches!(a.action, ui::Action::ExperimentalEnable { .. })));
        assert_eq!(p.details["preparation"]["candidates"], json!([]));
        products[0].disposition = "needs_attention".into();
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products[0].disposition, "needs_attention");
        assert_eq!(products[0].active_revision, Some(18));
    }
    #[test]
    fn removed_experimental_candidate_does_not_inherit_ordinary_admission_warning() {
        let (f, c) = projection_fixture();
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        prep::disable(&f.m, &c).unwrap();
        let mut product = projection_product(&c);
        // Ordinary readback correctly refuses a removed review candidate. It is
        // history, not a currently active ordinary product needing recovery.
        product.disposition = "needs_attention".into();
        product.active_revision = Some(c.profile.revision);
        product.details = json!({"profile":{"claim":"review_candidate"}});
        let mut products = vec![product];
        project(&f.m, &projection_software(&c), &mut products, None).unwrap();
        assert_eq!(products[0].disposition, "prepared");
        assert_eq!(products[0].active_revision, None);
        assert_eq!(products[0].details["preparation"]["publication"], "removed");
        assert!(!f.m.publications.join(format!("LVB_{}.vst3", c.selection.class.id)).exists());
        assert!(products[0].actions.iter().any(|a| matches!(&a.action,
            ui::Action::ExperimentalEnable { candidate } if *candidate == c.id().unwrap())));
        // An actual physical/publication disagreement still needs attention and
        // must not expose Enable merely because this is an experimental product.
        std::os::unix::fs::symlink(f.outer.join("foreign-target"), f.m.publications.join(format!("LVB_{}.vst3", c.selection.class.id))).unwrap();
        let mut products = vec![projection_product(&c)];
        project(&f.m, &projection_software(&c), &mut products, None).unwrap();
        assert_eq!(products[0].disposition, "needs_attention");
        assert!(!products[0].actions.iter().any(|a| matches!(&a.action,
            ui::Action::ExperimentalEnable { .. })));
    }
    #[test]
    fn current_selection_projects_once_with_old_candidate_and_inspection_history() {
        let (f, c) = projection_fixture();
        prep::record_candidate(&f.m, &c).unwrap();
        prep::retain_inspection(&f.m, &c.inspection).unwrap();
        let original = test_fixture::snapshot(&f.m.root.join("preparation/candidates"));
        let mut sw = projection_software(&c);
        // A current kit host with exact runtime identities for preparation offers.
        let kitpath = f.m.root.join("software/projection-kit.zip");
        fs::write(&kitpath, b"generated kit identity").unwrap();
        fs::set_permissions(&kitpath, fs::Permissions::from_mode(0o400)).unwrap();
        let kit = Artifact {
            sha256: digest(&kitpath).unwrap(),
            path: kitpath,
        };
        let dir = f.m.root.join("software/preparation-kits").join(&kit.sha256);
        private_dir(&dir).unwrap();
        let copy = |a: &Artifact, name: &str| {
            let path = dir.join(name);
            fs::copy(&a.path, &path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            Artifact {
                path,
                sha256: a.sha256.clone(),
            }
        };
        let host = copy(&c.host, "host.exe");
        let source = copy(&c.source_manifest, "host-source-manifest.json");
        atomic_json(
            &dir.join("runtime.json"),
            &prep::build::Runtime {
                kit: kit.clone(),
                host: host.clone(),
                source_manifest: source.clone(),
                builder: None,
                generator: None,
            },
        )
        .unwrap();
        sw.preparation_kit = Some(kit.clone());
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let mut raw: Value = read_json(&c.selection.factory_report.path).unwrap();
        raw["inspection_generation"] = json!(2);
        let report_path = c
            .selection
            .factory_report
            .path
            .with_file_name("inspection-s2.json");
        atomic_json(&report_path, &raw).unwrap();
        let report = Artifact {
            sha256: digest(&report_path).unwrap(),
            path: report_path,
        };
        let scanpath =
            f.m.root
                .join("inventory")
                .join(format!("{}.json", c.selection.environment.id));
        let mut scan: inventory::Scan = read_json(&scanpath).unwrap();
        scan.modules[0].report = report.clone();
        atomic_json(&scanpath, &scan).unwrap();
        let s2 = prep::selections(&f.m, &sw.host, &sw.source_sha256)
            .unwrap()
            .pop()
            .unwrap();
        assert_ne!(s2, c.selection);
        let i2 = prep::inspect_record_with(
            s2.clone(),
            report,
            prep::Origin::ManagedPreparation,
            host,
            source,
        )
        .unwrap();
        prep::retain_inspection(&f.m, &i2).unwrap();
        assert_eq!(
            product_selections(vec![s2.clone(), s2.clone()], vec![c.clone(), c.clone()]).unwrap(),
            vec![s2.clone()]
        );
        assert!(product_selections(vec![s2.clone(), c.selection.clone()], vec![]).is_err());
        let mut products = vec![projection_product(&c)];
        project(&f.m, &sw, &mut products, None).unwrap();
        assert_eq!(products.len(), 1);
        let p = &products[0];
        let v = &p.details["preparation"];
        assert_eq!(v["selection"], s2.id().unwrap());
        assert_eq!(v["recommended_inspection"], i2.id().unwrap());
        assert_eq!(v["candidates"].as_array().unwrap().len(), 1);
        assert!(v["inspections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == c.inspection.id().unwrap()));
        assert_eq!(
            p.actions
                .iter()
                .filter(|a| matches!(a.action, ui::Action::PluginReinspect { .. }))
                .count(),
            1
        );
        let prepare = p
            .actions
            .iter()
            .find(|a| matches!(a.action, ui::Action::PluginPrepare { .. }))
            .unwrap();
        assert_eq!(
            prepare.action,
            ui::Action::PluginPrepare {
                selection: s2.id().unwrap(),
                inspection: i2.id().unwrap(),
                recipe: kit.sha256,
                predecessor: Some(c.id().unwrap())
            }
        );
        for (n, a) in p.actions.iter().enumerate() {
            assert!(!p.actions[..n].iter().any(|b| b.action == a.action));
        }
        assert_eq!(
            test_fixture::snapshot(&f.m.root.join("preparation/candidates")),
            original
        );
    }
    #[test]
    fn ordinarily_published_candidate_offers_withdraw_and_not_publish() {
        let (f, c) = projection_fixture();
        prep::record_candidate(&f.m, &c).unwrap();
        for area in prep::AREAS {
            prep::record_observation(
                &f.m,
                &c,
                &random_id().unwrap(),
                area,
                prep::TestStatus::Passed,
                "Generated exact evidence",
            )
            .unwrap();
        }
        prep::review(
            &f.m,
            &c,
            &random_id().unwrap(),
            prep::ReviewChoice::AcceptExactLocal,
            "Generated exact review",
        )
        .unwrap();
        prep::enable(&f.m, &c, true).unwrap();
        let mut products = vec![projection_product(&c)];
        project(&f.m, &projection_software(&c), &mut products, None).unwrap();
        assert_eq!(products[0].disposition, "ready");
        assert_eq!(
            products[0].details["preparation"]["publication"],
            "ordinary"
        );
        assert!(products[0]
            .actions
            .iter()
            .any(|a| matches!(a.action, ui::Action::CandidateWithdraw { .. })));
        assert!(!products[0]
            .actions
            .iter()
            .any(|a| matches!(a.action, ui::Action::CandidatePublishOrdinary { .. })));
    }
}

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
    let mut selections = prep::selections(m, &sw.host, &sw.source_sha256)?;
    for c in prep::candidates(m, &sw.host, &sw.source_sha256)? {
        if !selections.iter().any(|s| {
            s.environment.id == c.selection.environment.id
                && s.module.sha256 == c.selection.module.sha256
                && s.class.id == c.selection.class.id
        }) {
            selections.push(c.selection);
        }
    }
    for s in selections {
        let Some(p) = products.iter_mut().find(|p| {
            p.class_id == s.class.id
                && p.module_sha256 == s.module.sha256
                && p.environment == s.environment.id
        }) else {
            continue;
        };
        let v = prep::view(m, &s, &sw.host, &sw.source_sha256)?;
        p.active_revision = v.current_profile_revision;
        p.details["preparation"] = serde_json::to_value(&v)?;
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
        p.actions.push(offer(
            if v.recommended_inspection.is_some() {
                "Refresh preliminary inspection"
            } else {
                "Check compatibility"
            }
            .into(),
            ui::Action::PluginReinspect {
                selection: v.selection.clone(),
            },
            busy.or(stale),
        ));
        if let Some(inspection) = &v.recommended_inspection {
            let kit = prep::build::recipe_available(m);
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
        if let Some(rows) = p.details["preparation"]["candidates"].as_array_mut() {
            for row in rows {
                let id = row["id"].as_str().ok_or("candidate_view_id")?.to_owned();
                if let Some(op) = operator_cli::product_receipt(m, &v.selection, Some(&id))? {
                    row["operation"] = op;
                }
            }
        }
        if let Some(op) = operator_cli::product_receipt(m, &v.selection, v.candidate.as_deref())? {
            p.details["preparation"]["operation"] = op;
        }
    }
    Ok(())
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
pub fn inspect(m: &Manager, s: &prep::Selection, sw: &Software) -> Result<prep::Inspection> {
    prep::verify_selection(m, s, &sw.host, &sw.source_sha256)?;
    let runtime = prep::build::stage_runtime(m)?;
    let stamp = observation::ModuleStamp::read(&s.module.path)?;
    let (job, path) = {
        let _guard = m.lock("registry.lock")?;
        m.require_inactive(None)?;
        spec(
            m,
            HostBinding {
                metadata: ClassSelection {
                    class_id: s.class.id.clone(),
                },
                environment: s.environment.clone(),
                module: s.module.clone(),
                host: runtime.host.clone(),
                host_source_sha256: runtime.source_manifest.sha256.clone(),
                compatibility: Compatibility::default(),
            },
            true,
            true,
            false,
        )?
    };
    let mut pending = PendingAdmission::new(job.lease.clone(), Arc::new(AtomicBool::new(false)));
    let child = spawn(sw, &path, None)?;
    pending.expose();
    vendor_product_cli::finish_scan(child, &job, pending)?;
    require(
        observation::ModuleStamp::read(&s.module.path)? == stamp,
        "inspection_module_changed",
    )?;
    let report = Artifact {
        sha256: digest(&job.report)?,
        path: job.report,
    };
    let i = prep::inspect_record_with(
        s.clone(),
        report,
        prep::Origin::ManagedPreparation,
        runtime.host,
        runtime.source_manifest,
    )?;
    prep::retain_inspection(m, &i)?;
    Ok(i)
}
pub fn execute(m: &Manager, a: &ui::Action, operation: &str) -> Result<Value> {
    let sw = software(m)?;
    match a {
        ui::Action::PluginInspect { selection } | ui::Action::PluginReinspect { selection } => {
            let s = prep::select(m, selection, &sw.host, &sw.source_sha256)?;
            let i = inspect(m, &s, &sw)?;
            Ok(
                json!({"selection":selection,"inspection":"complete","controller":i.controller,"guarantee":false,"publication_changed":false}),
            )
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
        ui::Action::PluginPrepare { .. } => "candidate_preparation",
        ui::Action::ExperimentalReplace { .. }
        | ui::Action::CandidateWithdraw { .. }
        | ui::Action::ExperimentalEnable { .. }
        | ui::Action::ExperimentalDisable { .. }
        | ui::Action::CandidatePublishOrdinary { .. } => "publication_transaction",
        ui::Action::CandidateObserve { .. } | ui::Action::CandidateReview { .. } => {
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
}

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
        let v = prep::view(m, &s, &sw.host, &sw.source_sha256)?;
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
    fn projection_fixture() -> (test_fixture::Fixture, prep::Candidate) {
        use observation::ModuleStamp;
        use prep::*;
        use profiles::Family;
        use test_fixture::{inspection_report, prepared_accessibility};
        let (mut f, _, mut census, native) = prepared_accessibility(false);
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
    fn projection_software(c: &prep::Candidate) -> Software {
        Software {
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
            details: json!({}),
        }
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

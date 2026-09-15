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
    for s in prep::selections(m, &sw.host, &sw.source_sha256)? {
        let Some(p) = products.iter_mut().find(|p| {
            p.class_id == s.class.id
                && p.module_sha256 == s.module.sha256
                && p.environment == s.environment.id
        }) else {
            continue;
        };
        let v = match prep::view(m, &s, &sw.host, &sw.source_sha256) {
            Ok(v) => v,
            Err(e) => {
                p.details["preparation_failure"] = json!(e.to_string());
                p.disposition = "needs_attention".into();
                continue;
            }
        };
        let offer = |label: &str, a: ui::Action, reason: Option<&str>| ui::AvailableAction {
            label: label.into(),
            action: a,
            disabled_reason: reason.map(str::to_owned),
        };
        p.details["preparation"] = serde_json::to_value(&v)?;
        if !matches!(v.publication.as_str(), "experimental" | "ordinary") {
            p.active_revision = None;
        }
        if let Some(id) = v.candidate.as_ref() {
            let enabled = v.publication == "experimental";
            if enabled {
                p.disposition = "experimental".into()
            } else if v.publication != "ordinary" {
                p.disposition = "prepared".into()
            }
            p.actions.push(if enabled {
                offer(
                    "Disable experimental use / restore previous publication",
                    ui::Action::ExperimentalDisable {
                        candidate: id.clone(),
                    },
                    busy,
                )
            } else {
                offer(
                    "Enable experimental use in Bitwig",
                    ui::Action::ExperimentalEnable {
                        candidate: id.clone(),
                    },
                    busy.or(if v.publication == "ordinary" {
                        Some("This exact product is already ordinarily published")
                    } else {
                        None
                    }),
                )
            });
            p.actions.push(offer(
                "Record an operator observation",
                ui::Action::CandidateObserve {
                    candidate: id.clone(),
                    area: "editor".into(),
                    status: "not_tested".into(),
                    note: String::new(),
                },
                None,
            ));
            p.actions.push(offer(
                "Review this exact configuration",
                ui::Action::CandidateReview {
                    candidate: id.clone(),
                    accept: true,
                    rationale: String::new(),
                },
                if v.unmet_requirements.is_empty() {
                    busy
                } else {
                    Some("Qualification incomplete: review the missing or failed results below")
                },
            ));
            p.actions.push(offer(
                "Record needs-work review",
                ui::Action::CandidateReview {
                    candidate: id.clone(),
                    accept: false,
                    rationale: String::new(),
                },
                None,
            ));
            let approved = v.ordinary_acceptance_current;
            p.actions.push(offer("Publish accepted exact configuration for ordinary use",ui::Action::CandidatePublishOrdinary{candidate:id.clone()},busy.or(if approved{None}else{Some("An explicit current qualification decision and complete product evidence are required")})));
        } else if v.inspection == "complete" {
            let controller = matches!(
                v.controller,
                Some(prep::ControllerAssociation::Unavailable { .. })
            );
            let kit = prep::build::recipe_available(m)
                .err()
                .map(|e| e.to_string());
            p.actions.push(offer("Prepare test instance",ui::Action::PluginPrepare{selection:v.selection.clone()},busy.or(if controller{Some("Exact controller association unavailable; inspection needs updated source-owned metadata")}else{kit.as_deref()})));
        } else {
            p.actions.push(offer(
                "Check compatibility",
                ui::Action::PluginInspect {
                    selection: v.selection.clone(),
                },
                busy,
            ));
        }
        if let Some(op) = operator_cli::product_receipt(m, &v.selection, v.candidate.as_deref())? {
            p.details["preparation"]["operation"] = op;
        }
    }
    Ok(())
}
pub fn is_action(a: &ui::Action) -> bool {
    matches!(
        a,
        ui::Action::PluginInspect { .. }
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
    if let Some(i) = prep::inspection(m, s)? {
        return Ok(i);
    }
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
        ui::Action::PluginInspect { selection } => {
            let s = prep::select(m, selection, &sw.host, &sw.source_sha256)?;
            let i = inspect(m, &s, &sw)?;
            Ok(
                json!({"selection":selection,"inspection":"complete","controller":i.controller,"guarantee":false,"publication_changed":false}),
            )
        }
        ui::Action::PluginPrepare { selection } => {
            let s = prep::select(m, selection, &sw.host, &sw.source_sha256)?;
            if let Some(c) = prep::candidates(m, &sw.host, &sw.source_sha256)?
                .into_iter()
                .find(|c| c.selection == s)
            {
                prep::verify_candidate(m, &c, &sw.host, &sw.source_sha256)?;
                return Ok(
                    json!({"selection":selection,"candidate":c.id()?,"reused":true,"publication_changed":false}),
                );
            }
            let i = prep::inspection(m, &s)?.ok_or("preliminary_inspection_required")?;
            let c = prep::build::construct(
                m,
                s,
                i.clone(),
                i.host.clone(),
                i.source_manifest.clone(),
                operation,
            )?;
            prep::verify_candidate(m, &c, &sw.host, &sw.source_sha256)?;
            let _guard = m.lock("registry.lock")?;
            m.require_inactive(None)?;
            let id = prep::record_candidate(m, &c)?;
            Ok(
                json!({"selection":selection,"candidate":id,"prepared":true,"publication_changed":false}),
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
        ui::Action::PluginInspect { .. } => "preliminary_inspection",
        ui::Action::PluginPrepare { .. } => "candidate_preparation",
        ui::Action::ExperimentalEnable { .. }
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
}

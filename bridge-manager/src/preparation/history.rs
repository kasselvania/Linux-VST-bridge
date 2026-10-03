//! Historical identity is readable independently of current installation authority.
use super::*;

pub fn retained_candidates(m: &Manager) -> Result<Vec<Candidate>> {
    let mut out = vec![];
    for p in list(&root(m).join("candidates"))? {
        if !p.is_dir() {
            continue;
        }
        let record = p.join("candidate.json");
        // A crash before atomic installation leaves only an uncommitted directory.
        if !record.try_exists()? {
            continue;
        }
        let c: Candidate = bounded(&record)?;
        require(
            c.schema == 1 && p.file_name().and_then(|n| n.to_str()) == Some(c.id()?.as_str()),
            "candidate_identity",
        )?;
        out.push(c);
    }
    Ok(out)
}
pub fn retain_lineage(
    m: &Manager,
    c: &Candidate,
    identity: &str,
    predecessor: Option<&str>,
) -> Result<()> {
    let _owner = m.lock("preparation-lineage.lock")?;
    let id = c.id()?;
    let path = object(m, "lineage", &id)?.join("record.json");
    if path.exists() {
        return lineage_record(m, c).map(|_| ());
    }
    if let Some(prior) = predecessor {
        let old = retained_candidates(m)?
            .into_iter()
            .find(|p| p.id().is_ok_and(|x| x == prior))
            .ok_or("candidate_predecessor_absent")?;
        require(
            same_product(&old.selection, &c.selection) && prior != id,
            "candidate_predecessor_selection",
        )?;
    }
    let ordinal = list(&root(m).join("lineage"))?.len() as u64 + 1;
    immutable(
        &path,
        &CandidateLineage {
            schema: 1,
            candidate: id,
            preparation_identity: identity.into(),
            ordinal,
            predecessor: predecessor.map(str::to_owned),
        },
    )
}
pub fn lineage(m: &Manager, c: &Candidate) -> Result<CandidateLineage> {
    let path = object(m, "lineage", &c.id()?)?.join("record.json");
    if !path.exists() {
        retain_lineage(m, c, &c.id()?, None)?;
    }
    lineage_record(m, c)
}
pub(super) fn lineage_record(m: &Manager, c: &Candidate) -> Result<CandidateLineage> {
    let path = object(m, "lineage", &c.id()?)?.join("record.json");
    let v: CandidateLineage = bounded(&path)?;
    require(
        v.schema == 1 && v.candidate == c.id()?,
        "candidate_lineage_identity",
    )?;
    Ok(v)
}
pub fn legacy_provenance(m: &Manager, c: &Candidate) -> Result<LegacyProvenance> {
    let v: LegacyProvenance = bounded(&object(m, "legacy", &c.id()?)?.join("provenance.json"))?;
    require(
        v.schema == 1
            && v.candidate == c.id()?
            && v.profile == c.profile
            && v.native == c.native
            && v.host == c.host
            && v.source_manifest == c.source_manifest
            && v.inspection == c.inspection
            && v.authority == "sv1_instrument",
        "legacy_provenance_binding",
    )?;
    Ok(v)
}
pub(super) fn retained_history_paths(m: &Manager, c: &Candidate) -> Result<Vec<(&'static str, PathBuf)>> {
    let dir = object(m, "legacy", &c.id()?)?;
    Ok(vec![
        ("provenance", dir.join("provenance.json")),
        ("environment", dir.join("environment.json")),
        ("onboarding", dir.join("onboarding.json")),
        ("inventory", dir.join("inventory.json")),
        ("lineage", object(m, "lineage", &c.id()?)?.join("record.json")),
        ("inspection", object(m, "inspections", &c.selection.id()?)?
            .join(format!("{}.json", c.inspection.id()?))),
        ("inspection_order", object(m, "inspection-order", &c.inspection.id()?)?.join("record.json")),
    ])
}
fn retained_binding() -> Result<Value> {
    Ok(serde_json::from_slice(include_bytes!("../../../compatibility/sv1/binding.json"))?)
}
#[derive(Clone, Debug, Serialize)]
pub struct RetainedHistoryRecovery {
    pub candidate: String,
    pub expected_history: String,
    pub missing: Vec<String>,
    pub unavailable_reason: Option<String>,
}
/// Missing migration records are visible without inventing lineage or granting
/// executable authority. Existing malformed or conflicting records still refuse.
pub fn retained_history_recovery(m: &Manager, c: &Candidate) -> Result<Option<RetainedHistoryRecovery>> {
    retained_history_recovery_with_binding(m, c, &retained_binding()?)
}
pub(super) fn retained_history_recovery_with_binding(m: &Manager, c: &Candidate, binding: &Value)
    -> Result<Option<RetainedHistoryRecovery>> {
    if c.origin != Origin::RetainedSv1 { return Ok(None); }
    let paths = retained_history_paths(m, c)?;
    let missing = paths.iter().filter_map(|(name, path)| match fs::symlink_metadata(path) {
        Ok(_) => None,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(Ok((*name).to_owned())),
        Err(error) => Some(Err(error)),
    }).collect::<std::io::Result<Vec<_>>>()?;
    if missing.is_empty() { return Ok(None); }
    let provenance = if missing.iter().any(|name| name == "provenance") { None }
        else {
            let provenance = legacy_provenance(m, c)?;
            require(provenance.binding == *binding, "legacy_input_binding")?;
            Some(provenance)
        };
    let mut records = Vec::new();
    for (name, path) in paths {
        if missing.iter().any(|slot| slot == name) {
            records.push((name, None));
            continue;
        }
        require(path.canonicalize()? == path, "preparation_record_bound_or_alias")?;
        let (value, hash): (Value, String) = read_json_identity(&path)?;
        match name {
            "environment" | "onboarding" | "inventory" => {
                require(hash == binding[format!("{name}_sha256")], "legacy_input_binding")?;
                if let Some(provenance) = &provenance {
                    let expected = match name {
                        "environment" => &provenance.environment,
                        "onboarding" => &provenance.onboarding,
                        _ => &provenance.inventory,
                    };
                    let artifact = provenance.inputs.get(name).ok_or("legacy_input_absent")?;
                    require(artifact.path == path && artifact.sha256 == hash && value == *expected,
                        "legacy_input_identity")?;
                }
            }
            "lineage" => { lineage_record(m, c)?; }
            "inspection" => require(serde_json::from_value::<Inspection>(value)? == c.inspection,
                "legacy_inspection_history_changed")?,
            "inspection_order" => { serde_json::from_value::<u64>(value)?; }
            _ => (),
        }
        records.push((name, Some(hash)));
    }
    // Bind any current input that the existing migration owner would need to
    // snapshot. Its exact original digest is still enforced by that owner.
    let mut unavailable_inputs = Vec::new();
    for (name, path) in [
        ("environment", c.selection.environment.root.join("environment.json")),
        ("onboarding", m.root.join("onboarding").join(&c.selection.environment.id).join("record.json")),
        ("inventory", m.root.join("inventory").join(format!("{}.json", c.selection.environment.id))),
    ] {
        if missing.iter().any(|slot| slot == name) {
            if let Err(error) = fs::symlink_metadata(&path) {
                if error.kind() != std::io::ErrorKind::NotFound { return Err(error.into()); }
                unavailable_inputs.push(name);
                records.push((name, None));
                continue;
            }
            require(path.canonicalize()? == path, "preparation_record_bound_or_alias")?;
            let (_, hash): (Value, String) = read_json_identity(&path)?;
            records.push((name, Some(hash)));
        }
    }
    Ok(Some(RetainedHistoryRecovery {candidate:c.id()?,
        expected_history:key(&(c.id()?, binding, records))?,missing,
        unavailable_reason:(!unavailable_inputs.is_empty()).then(||
            "Saved setup history cannot be completed because original setup records are missing.".into())}))
}
/// The same migration owner, scoped to the exact offered candidate and records.
pub fn complete_candidate_history(m: &Manager, candidate: &str, expected_history: &str) -> Result<()> {
    complete_candidate_history_with_binding(m, candidate, expected_history, &retained_binding()?)
}
pub(super) fn complete_candidate_history_with_binding(m: &Manager, candidate: &str,
    expected_history: &str, binding: &Value) -> Result<()> {
    require(valid_hex(expected_history, 64), "candidate_history_identity")?;
    let c = candidate_record(m, candidate)?;
    let recovery = retained_history_recovery_with_binding(m, &c, binding)?
        .ok_or("candidate_history_not_incomplete")?;
    require(recovery.expected_history == expected_history, "candidate_history_changed")?;
    require(recovery.unavailable_reason.is_none(), "candidate_history_source_unavailable")?;
    complete_legacy_history(m, &c, binding)
}
/// A candidate record predates generation history in the first installed MF3.
/// Presence of that record is not proof that its provenance transition completed.
pub(super) fn complete_legacy_history(m: &Manager, c: &Candidate, binding: &Value) -> Result<()> {
    let mut complete = true;
    for (_, path) in retained_history_paths(m, c)? {
        complete &= path.try_exists()?;
    }
    if !complete {
        // Existing snapshots win over mutable inventory. The original binding
        // still verifies each missing snapshot, and no conflicting file is replaced.
        materialize_legacy(m, c, binding)?;
    }
    verify_legacy(m, c)?;
    let inspection: Inspection = bounded(
        &object(m, "inspections", &c.selection.id()?)?.join(format!("{}.json", c.inspection.id()?)),
    )?;
    require(inspection == c.inspection, "legacy_inspection_history_changed")?;
    let _ordinal: u64 = bounded(&object(m, "inspection-order", &c.inspection.id()?)?.join("record.json"))?;
    let _lineage = lineage(m, c)?;
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MaterializeBoundary {
    Environment,
    Onboarding,
    Inventory,
    ProvenanceStaged,
    ProvenanceInstalled,
    Lineage,
    Candidate,
}
pub(super) fn materialize_legacy(m: &Manager, c: &Candidate, b: &Value) -> Result<()> {
    materialize_legacy_with(m, c, b, None)
}
pub(super) fn materialize_legacy_with(
    m: &Manager,
    c: &Candidate,
    b: &Value,
    fail: Option<MaterializeBoundary>,
) -> Result<()> {
    let stop = |at| require(fail != Some(at), "legacy_materialization_interrupted");
    let dir = object(m, "legacy", &c.id()?)?;
    let seal = dir.join("provenance.json");
    let mut inputs = std::collections::BTreeMap::new();
    for (name, path, boundary) in [
        (
            "environment",
            c.selection.environment.root.join("environment.json"),
            MaterializeBoundary::Environment,
        ),
        (
            "onboarding",
            m.root
                .join("onboarding")
                .join(&c.selection.environment.id)
                .join("record.json"),
            MaterializeBoundary::Onboarding,
        ),
        (
            "inventory",
            m.root
                .join("inventory")
                .join(format!("{}.json", c.selection.environment.id)),
            MaterializeBoundary::Inventory,
        ),
    ] {
        let target = dir.join(format!("{name}.json"));
        // Previously committed exact snapshots survive mutable inventory drift.
        let source = if fs::symlink_metadata(&target).is_ok() {
            &target
        } else {
            &path
        };
        let mut bytes = vec![];
        file(source)?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        require(
            bytes.len() <= 8 * 1024 * 1024
                && hex(&Sha256::digest(&bytes)) == b[format!("{name}_sha256")],
            if source == &target {
                "preparation_immutable_conflict"
            } else {
                "legacy_input_binding"
            },
        )?;
        immutable_bytes(&target, &bytes)?;
        inputs.insert(
            name.to_owned(),
            Artifact {
                path: target,
                sha256: hex(&Sha256::digest(&bytes)),
            },
        );
        stop(boundary)?;
    }
    let v = LegacyProvenance {
        schema: 1,
        candidate: c.id()?,
        authority: "sv1_instrument".into(),
        profile: c.profile.clone(),
        native: c.native.clone(),
        host: c.host.clone(),
        source_manifest: c.source_manifest.clone(),
        inspection: c.inspection.clone(),
        binding: b.clone(),
        environment: bounded(&inputs["environment"].path)?,
        onboarding: bounded(&inputs["onboarding"].path)?,
        inventory: bounded(&inputs["inventory"].path)?,
        inputs,
        original_evidence: serde_json::from_slice(include_bytes!(
            "../../../evidence/sv1/first-operator-session.json"
        ))?,
    };
    immutable_bytes_staged(&seal, &serde_json::to_vec(&v)?, &mut || {
        stop(MaterializeBoundary::ProvenanceStaged)
    })?;
    stop(MaterializeBoundary::ProvenanceInstalled)?;
    // The seal alone is not completion: finish all derived history after a crash.
    verify_legacy(m, c)?;
    retain_inspection(m, &c.inspection)?;
    retain_lineage(
        m,
        c,
        &format!(
            "sv1:{}:{}",
            v.original_evidence["source_head_at_publication"]
                .as_str()
                .unwrap_or("unavailable"),
            c.profile.fingerprint()?
        ),
        None,
    )?;
    stop(MaterializeBoundary::Lineage)?;
    record_candidate(m, c)?;
    stop(MaterializeBoundary::Candidate)?;
    Ok(())
}
pub fn verify_legacy(m: &Manager, c: &Candidate) -> Result<()> {
    validate_legacy_record(m, c)?;
    for a in [&c.host, &c.source_manifest, &c.native.artifact,
        &c.inspection.report, &c.selection.factory_report] { a.verify()?; }
    Ok(())
}
pub(super) fn validate_legacy_record(m: &Manager, c: &Candidate) -> Result<()> {
    require(c.origin == Origin::RetainedSv1, "not_legacy_provenance")?;
    let v = legacy_provenance(m, c)?;
    for (name, value) in [
        ("environment", &v.environment),
        ("onboarding", &v.onboarding),
        ("inventory", &v.inventory),
    ] {
        let a = v.inputs.get(name).ok_or("legacy_input_absent")?;
        require(
            a.path == object(m, "legacy", &c.id()?)?.join(format!("{name}.json"))
                && a.sha256 == v.binding[format!("{name}_sha256")],
            "legacy_input_identity",
        )?;
        require(a.read_record::<Value>(8 * 1024 * 1024)? == *value, "legacy_input_content")?;
    }
    require(
        v.binding["inspection_sha256"] == c.inspection.report.sha256
            && serde_json::from_value::<Environment>(v.environment)? == c.selection.environment,
        "legacy_inspection_environment",
    )?;
    for a in [
        &c.host,
        &c.source_manifest,
        &c.native.artifact,
        &c.inspection.report,
        &c.selection.factory_report,
    ] {
        a.validate_record()?;
    }
    c.native.matches(&v.profile)
}

pub fn inspections(m: &Manager, s: &Selection) -> Result<Vec<Inspection>> {
    inspections_with_candidates(m, s, &retained_candidates(m)?)
}
fn inspections_with_candidates(m: &Manager, s: &Selection, candidates: &[Candidate]) -> Result<Vec<Inspection>> {
    let mut out = vec![];
    for p in list(&object(m, "inspections", &s.id()?)?)? {
        if p.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let i: Inspection = bounded(&p)?;
        require(i.schema == 1 && i.selection == *s, "inspection_identity")?;
        if !out.iter().any(|x: &Inspection| x.id().ok() == i.id().ok()) {
            out.push(i);
        }
    }
    for c in candidates.iter()
        .filter(|c| c.selection == *s)
    {
        if !out.contains(&c.inspection) {
            out.push(c.inspection.clone());
        }
    }
    let mut ordered = out
        .into_iter()
        .map(|i| Ok((inspection_ordinal(m, &i)?, i)))
        .collect::<Result<Vec<_>>>()?;
    ordered.sort_by_key(|(ordinal, _)| *ordinal);
    Ok(ordered.into_iter().map(|(_, i)| i).collect())
}
pub(super) fn retain_inspection_order(m: &Manager, i: &Inspection) -> Result<()> {
    let _owner = m.lock("preparation-inspection.lock")?;
    let path = object(m, "inspection-order", &i.id()?)?.join("record.json");
    if !path.exists() {
        immutable(
            &path,
            &(list(&root(m).join("inspection-order"))?.len() as u64 + 1),
        )?;
    }
    Ok(())
}
fn inspection_ordinal(m: &Manager, i: &Inspection) -> Result<u64> {
    let p = object(m, "inspection-order", &i.id()?)?.join("record.json");
    if p.exists() {
        bounded(&p)
    } else {
        Ok(0)
    }
}
pub fn recommended_inspection(m: &Manager, s: &Selection) -> Result<Option<Inspection>> {
    recommended_inspection_with(m, s, &retained_candidates(m)?, true)
}
fn recommended_inspection_with(m: &Manager, s: &Selection, candidates: &[Candidate],
    execution: bool) -> Result<Option<Inspection>> {
    let current = if m.root.join("software.json").exists() {
        let sw: crate::catalogue::Software = bounded(&m.root.join("software.json"))?;
        if execution { verify_selection(m, s, &sw.host, &sw.source_sha256) }
        else { validate_selection_record(m, s, &sw.host, &sw.source_sha256) }
    } else {
        if execution { verify_selection(m, s, &s.scanner, &s.scanner_source) }
        else { validate_selection_record(m, s, &s.scanner, &s.scanner_source) }
    };
    if current.is_err() {
        return Ok(None);
    }

    let expected = match build::recipe_available(m) {
        Ok(a) => match if execution { build::existing_runtime(m, &a.sha256) }
            else { build::existing_runtime_record(m, &a.sha256) } {
            Ok(r) => Some(r),
            Err(_) => return Ok(None),
        },
        Err(_) => None,
    };
    let mut matches = vec![];
    for i in inspections_with_candidates(m, s, candidates)? {
        let current = if let Some(r) = &expected {
            i.host == r.host && i.source_manifest == r.source_manifest
        } else {
            i.host.sha256 == s.scanner.sha256 && i.source_manifest.sha256 == s.scanner_source
        };
        let host_valid = if execution { i.host.verify().is_ok() && i.source_manifest.verify().is_ok() }
            else { i.host.validate_record().is_ok() && i.source_manifest.validate_record().is_ok() };
        if current
            && i.report.read_record::<Value>(8 * 1024 * 1024).is_ok()
            && host_valid
        {
            matches.push(i);
        }
    }
    Ok(matches.pop())
}
pub fn exact_inspection(m: &Manager, s: &Selection, id: &str) -> Result<Inspection> {
    let i = inspections(m, s)?
        .into_iter()
        .find(|i| i.id().is_ok_and(|v| v == id))
        .ok_or("inspection_absent")?;
    require(
        recommended_inspection(m, s)?.as_ref() == Some(&i),
        "inspection_superseded_refresh_required",
    )?;
    require(
        inspect_record_with_layout(
            s.clone(),
            i.report.clone(),
            i.origin.clone(),
            i.host.clone(),
            i.source_manifest.clone(),
            i.audio_layout.clone(),
        )? == i,
        "inspection_changed",
    )?;
    Ok(i)
}
/// Policy A: a review bound into an ordinary profile is immutable authority for
/// that publication. New observations govern new acceptance, not a hidden unload.
pub fn accepted_profile(m: &Manager, c: &Candidate, p: &Profile) -> Result<bool> {
    for d in decisions(m, c)? {
        if d.choice == ReviewChoice::AcceptExactLocal && d.ordinary_profile.as_ref() == Some(p) {
            let retained: Decision = bounded(
                &m.root
                    .join("evidence/mf3/reviews")
                    .join(format!("{}.json", d.operation)),
            )?;
            require(retained == d, "publication_acceptance_seal_changed")?;
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn current_revision(m: &Manager, class: &str) -> Result<Option<Revision>> {
    let db = m.registry()?;
    db.classes
        .get(class)
        .and_then(|e| e.managed_revision.as_ref())
        .map(|r| m.load_revision(class, r))
        .transpose()
}
pub fn view(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<View> {
    let all = candidates(m, host, source)?;
    view_with_candidates(m, s, host, source, &all)
}

/// Project several products from one retained candidate census rather than
/// rereading and materializing the same history for every product card.
pub fn view_with_candidates(
    m: &Manager, s: &Selection, host: &Artifact, source: &str, all: &[Candidate],
) -> Result<View> {
    view_with_evidence(m, s, host, source, all, true)
}
pub fn view_records(m: &Manager, s: &Selection, host: &Artifact, source: &str,
    records: &RecordReadback) -> Result<View> {
    for c in records.candidates.iter().filter(|c| same_product(&c.selection, s)) {
        records.watch_candidate(m, c)?;
    }
    view_with_evidence(m, s, host, source, &records.candidates, false)
}
fn view_with_evidence(m: &Manager, s: &Selection, host: &Artifact, source: &str,
    all: &[Candidate], execution: bool) -> Result<View> {
    let found: Vec<_> = all.iter()
        .filter(|c| {
            c.selection.environment.id == s.environment.id
                && c.selection.module.sha256 == s.module.sha256
                && c.selection.class.id == s.class.id
        })
        .cloned()
        .collect();
    let i = recommended_inspection_with(m, s, all, execution)?;
    let recommended = i.as_ref().map(Inspection::id).transpose()?;
    let mut history = vec![];
    for c in &found {
        let evidence = observations(m, c)?;
        let publication = if execution { publication_state(m, c)? } else { publication_state_record(m, c)? };
        let lineage = if execution { lineage(m, c)? } else { lineage_record(m, c)? };
        let current = if execution { verify_candidate(m, c, host, source).is_ok() }
            else { validate_current_candidate_record(m, c, host, source).is_ok() };
        let sealed = publication == "ordinary";
        let legacy = if c.origin == Origin::RetainedSv1 {
            Some(legacy_provenance(m, c)?)
        } else {
            None
        };
        let terminal_cleanup=legacy.as_ref().map(|p|{
            let r=&p.original_evidence;
            serde_json::json!({"disposition":r["retirement_disposition"],"cleanup_confirmed":r["cleanup_confirmed"],"transport_retired":r["transport_retired"],"source":"retained_sv1","normal_retirement_proved":false})
        }).unwrap_or(Value::Null);
        history.push(CandidateView {
            id: c.id()?,
            origin: c.origin.clone(),
            lineage,
            inspection: c.inspection.id()?,
            host_sha256: c.host.sha256.clone(),
            source_sha256: c.source_manifest.sha256.clone(),
            native_sha256: c.native.artifact.sha256.clone(),
            descriptor_sha256: c.native.descriptor_sha256.clone(),
            recipe: legacy
                .as_ref()
                .map(key)
                .transpose()?
                .unwrap_or_else(|| c.recipe_sha256.clone()),
            disposition: if matches!(publication.as_str(), "experimental" | "ordinary") {
                "current_published"
            } else if !current {
                "historical"
            } else {
                "prepared"
            }
            .into(),
            current_inputs: current,
            publication,
            evidence: evidence.clone(),
            unmet_requirements: unmet(&evidence, &c.profile.role),
            review: decisions(m, c)?.pop(),
            terminal_cleanup,
            publication_acceptance_sealed: sealed,
            ordinary_acceptance_current: accepted(m, c).is_ok(),
        });
    }
    let predecessors: Vec<_> = history
        .iter()
        .filter_map(|h| h.lineage.predecessor.clone())
        .collect();
    for h in &mut history {
        if h.disposition == "prepared" && predecessors.contains(&h.id) {
            h.disposition = "superseded".into();
        }
    }
    history.sort_by_key(|h| h.lineage.ordinal);
    let selected = history
        .iter()
        .find(|h| h.disposition == "current_published")
        .or_else(|| history.last());
    let mut inspection_history = inspections_with_candidates(m, s, all)?;
    for c in &found {
        if !inspection_history.contains(&c.inspection) {
            inspection_history.push(c.inspection.clone());
        }
    }
    let all = inspection_history
        .into_iter()
        .map(|i| -> Result<InspectionView> {
            let id = i.id()?;
            let bound = found
                .iter()
                .filter(|c| c.inspection == i)
                .map(Candidate::id)
                .collect::<Result<Vec<_>>>()?;
            let rec = recommended.as_ref() == Some(&id);
            Ok(InspectionView {
                id,
                report_sha256: i.report.sha256,
                host_sha256: i.host.sha256,
                source_sha256: i.source_manifest.sha256,
                origin: i.origin,
                audio_layout: i.audio_layout,
                recommended: rec,
                candidate_bound: bound,
                disposition: if rec {
                    "recommended"
                } else {
                    "historical_or_superseded"
                }
                .into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let db = m.registry()?;
    let reference = db
        .classes
        .get(&s.class.id)
        .and_then(|e| e.managed_revision.clone());
    let active = db
        .classes
        .get(&s.class.id)
        .filter(|e| e.publication == Publication::Published)
        .and_then(|_| current_revision_with(m, &s.class.id, execution).ok().flatten())
        .map(|r| r.profile.revision);
    let view = View {
        selection: s.id()?,
        inspection: if i.is_some() {
            "complete"
        } else {
            "not_checked"
        }
        .into(),
        preliminary: preliminary(i.as_ref())?,
        controller: i.as_ref().map(|i| i.controller.clone()),
        candidate: selected.map(|h| h.id.clone()),
        preparation: if history.is_empty() {
            "not_prepared"
        } else {
            "prepared"
        }
        .into(),
        publication: selected
            .map(|h| h.publication.clone())
            .unwrap_or("unpublished".into()),
        origin: selected.map(|h| h.origin.clone()),
        evidence: selected.map(|h| h.evidence.clone()).unwrap_or_default(),
        unmet_requirements: selected
            .map(|h| h.unmet_requirements.clone())
            .unwrap_or_default(),
        review: selected.and_then(|h| h.review.clone()),
        ordinary_acceptance_current: selected.is_some_and(|h| h.ordinary_acceptance_current),
        operation: None,
        inspections: all,
        recommended_inspection: recommended,
        recommended_audio_layout: i.as_ref().and_then(|i| i.audio_layout.clone()),
        current_revision: reference,
        current_profile_revision: active,
        publication_facts: publication_facts_with(m, &s.class.id, execution)?,
        candidates: history,
    };
    Ok(view)
}

pub fn withdraw(m: &Manager, c: &Candidate, expected: &RevisionRef) -> Result<()> {
    require(
        publication_state(m, c)? == "ordinary",
        "not_selected_ordinary_publication",
    )?;
    m.unpublish_exact(&c.selection.class.id, expected)
}

pub fn publication_facts(m: &Manager, class: &str) -> Result<Value> {
    publication_facts_with(m, class, true)
}
fn current_revision_with(m: &Manager, class: &str, execution: bool) -> Result<Option<Revision>> {
    let db = m.registry()?;
    db.classes.get(class).and_then(|entry| entry.managed_revision.as_ref())
        .map(|reference| if execution { m.load_revision(class, reference) }
            else { m.load_revision_record(class, reference) }).transpose()
}
fn publication_facts_with(m: &Manager, class: &str, execution: bool) -> Result<Value> {
    let db = m.registry()?;
    let e = db.classes.get(class);
    let r = current_revision_with(m, class, execution)?;
    let pointer = physical(&m.link(class))?;
    Ok(
        serde_json::json!({"registry_publication":e.map(|e|&e.publication),"current_revision":e.and_then(|e|e.managed_revision.as_ref()),"current_profile_sha256":r.as_ref().map(|r|&r.profile_sha256),"qualification":r.as_ref().and_then(|r|r.qualification),"physical_present":pointer.is_some(),"physical_matches_retained":r.as_ref().is_some_and(|r|pointer.as_ref()==Some(&r.target)),"pending":m.publication_pending(class)?,"acceptance_policy":"publication_time_until_explicit_withdrawal"}),
    )
}

/// A new evidence/review basis is a new metadata generation, never a rewrite.
pub fn preparation_basis(m: &Manager, predecessor: Option<&Candidate>) -> Result<String> {
    if let Some(c) = predecessor {
        if observations(m, c)?.is_empty() && decisions(m, c)?.is_empty() {
            if let Some(basis) = &c.preparation_basis {
                return Ok(basis.clone());
            }
        }
    }

    key(&predecessor
        .map(|c| -> Result<_> { Ok((c.id()?, observations(m, c)?, decisions(m, c)?)) })
        .transpose()?)
}
pub fn bind_preparation_basis(c: Candidate, basis: Option<String>) -> Result<Candidate> {
    let graphics = c.profile.capabilities.graphics;
    let settings = c.local_settings.clone();
    let accessibility = c.profile.capabilities.accessibility.clone();
    // Preparation/refresh already resolved defaults and carried explicit
    // choices. Binding review metadata must preserve that configuration.
    let mut result = prepared_with_advice(
        c.selection,
        c.inspection,
        c.native,
        c.host,
        c.source_manifest,
        c.recipe_sha256,
        (accessibility.clone(), c.profile.evidence),
    )?;
    if let Some(settings) = settings {
        configuration::apply_resolved_settings(&mut result, settings.clone(), accessibility)?;
        result.profile.id = format!("managed.{}", key(&(&result.profile.id, &settings))?);
    } else if graphics.is_some() {
        result.profile.capabilities.graphics = graphics;
        result.profile.id = format!("managed.{}", key(&(&result.profile.id, graphics))?);
    }
    if let Some(basis) = basis {
        require(valid_hex(&basis, 64), "preparation_evidence_basis")?;
        result.profile.id = format!("managed.{}", key(&(&result.profile.id, &basis))?);
        result.preparation_basis = Some(basis);
    }
    Ok(result)
}

pub fn same_product(a: &Selection, b: &Selection) -> bool {
    a.environment.id == b.environment.id
        && a.module.sha256 == b.module.sha256
        && a.class.id == b.class.id
}

//! MF3: canonical selection, immutable preparation and explicit local review.
//! No frontend path, process, profile or compiler authority enters this owner.
pub mod build;
mod history;
mod model;
use crate::{
    catalogue::NativeArtifact,
    observation::{Census, ModuleStamp},
    profiles::*,
    publication::*,
    *,
};
pub use history::*;
pub use model::*;
use serde_json::Value;

pub fn key<T: Serialize>(v: &T) -> Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(v)?)))
}
fn bounded<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T> {
    require(
        p.canonicalize()? == p && file(p)?.metadata()?.len() <= 8 * 1024 * 1024,
        "preparation_record_bound_or_alias",
    )?;
    read_json(p)
}
fn root(m: &Manager) -> PathBuf {
    m.root.join("preparation")
}
fn object(m: &Manager, kind: &str, id: &str) -> Result<PathBuf> {
    require(valid_hex(id, 64), "preparation_identity")?;
    Ok(root(m).join(kind).join(id))
}
fn list(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.try_exists()? {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for e in fs::read_dir(dir)?.take(257) {
        require(out.len() < 256, "preparation_count_bound")?;
        out.push(e?.path());
    }
    out.sort();
    Ok(out)
}
fn immutable<T: Serialize>(p: &Path, v: &T) -> Result<()> {
    immutable_bytes(p, &serde_json::to_vec(v)?)
}
fn immutable_bytes(p: &Path, bytes: &[u8]) -> Result<()> {
    immutable_bytes_staged(p, bytes, &mut || Ok(()))
}
fn immutable_bytes_staged(
    p: &Path,
    bytes: &[u8],
    staged: &mut dyn FnMut() -> Result<()>,
) -> Result<()> {
    require(bytes.len() <= 8 * 1024 * 1024, "preparation_record_bound")?;
    let verify = || -> Result<()> {
        let f = file(p)?;
        require(
            f.metadata()?.len() == bytes.len() as u64 && p.canonicalize()? == p,
            "preparation_immutable_conflict",
        )?;
        let mut prior = vec![];
        f.take(8 * 1024 * 1024 + 1).read_to_end(&mut prior)?;
        require(
            prior == bytes && Sha256::digest(&prior) == Sha256::digest(bytes),
            "preparation_immutable_conflict",
        )
    };
    let parent = p.parent().ok_or("record_parent")?;
    if fs::symlink_metadata(p).is_ok() {
        verify()?;
        File::open(parent)?.sync_all()?;
        return Ok(());
    }
    private_dir(parent)?;
    let tmp = parent.join(format!(".writing-{}", random_id()?));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o400)
            .open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        staged()?;
        match crate::publication::rename_link(&tmp, p, false) {
            Ok(()) => (),
            Err(e)
                if e.downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) =>
            {
                verify()?
            }
            Err(e) => return Err(e),
        }
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    // A killed process can leave only a private temporary file, never a partial
    // authoritative target. Normal error/retry paths also retire their temporary.
    let _ = fs::remove_file(&tmp);
    result
}
/// Factory and environment identities, never a friendly-name dispatch table.
pub fn selections(m: &Manager, host: &Artifact, source: &str) -> Result<Vec<Selection>> {
    let mut out = vec![];
    for path in list(&m.root.join("inventory"))? {
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let scan: crate::inventory::Scan = bounded(&path)?;
        if scan.schema != 1 || scan.host.sha256 != host.sha256 || scan.host_source_sha256 != source
        {
            continue;
        }
        let envpath = m
            .root
            .join("environments")
            .join(&scan.environment.id)
            .join("environment.json");
        if !valid_hex(&scan.environment.id, 32)
            || scan.environment.root != envpath.parent().unwrap()
            || bounded::<Environment>(&envpath)? != scan.environment
        {
            continue;
        }
        for module in scan.modules {
            if module.quarantine_reason.is_some() {
                continue;
            }
            for class in module.classes {
                if class.category != "Audio Module Class"
                    || !matches!(class.role.as_str(), "instrument" | "effect")
                {
                    continue;
                }
                out.push(Selection {
                    schema: 1,
                    environment: scan.environment.clone(),
                    module: module.artifact.clone(),
                    class,
                    scanner: scan.host.clone(),
                    scanner_source: scan.host_source_sha256.clone(),
                    factory_report: module.report.clone(),
                });
            }
        }
    }
    require(out.len() <= 256, "preparation_selection_bound")?;
    Ok(out)
}
pub fn select(m: &Manager, id: &str, host: &Artifact, source: &str) -> Result<Selection> {
    require(valid_hex(id, 64), "preparation_selection_identity")?;
    let matches: Vec<_> = selections(m, host, source)?
        .into_iter()
        .filter(|s| s.id().is_ok_and(|v| v == id))
        .collect();
    require(
        matches.len() == 1,
        "preparation_selection_stale_or_ambiguous",
    )?;
    let s = matches.into_iter().next().unwrap();
    verify_selection(m, &s, host, source)?;
    Ok(s)
}
pub fn verify_selection(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<()> {
    require(
        selections(m, host, source)?
            .iter()
            .any(|current| current == s),
        "preparation_inventory_superseded",
    )?;
    verify_selection_data(m, s, host, source)
}
fn verify_selection_data(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<()> {
    require(
        s.schema == 1 && s.scanner.sha256 == host.sha256 && s.scanner_source == source,
        "preparation_scanner_changed",
    )?;
    require(
        valid_hex(&s.environment.id, 32)
            && s.environment.root == m.root.join("environments").join(&s.environment.id),
        "preparation_environment_location",
    )?;
    require(
        bounded::<Environment>(&s.environment.root.join("environment.json"))? == s.environment,
        "preparation_environment_changed",
    )?;
    require(
        s.module
            .path
            .starts_with(s.environment.root.join("compatdata/pfx/drive_c"))
            && s.module.path.canonicalize()? == s.module.path,
        "preparation_module_location",
    )?;
    s.module.verify()?;
    s.scanner.verify()?;
    s.environment.runner.verify()?;
    s.factory_report.verify()?;
    let classes = crate::inventory::classes(&bounded::<Value>(&s.factory_report.path)?)?;
    require(
        classes.iter().filter(|c| **c == s.class).count() == 1
            && s.class.category == "Audio Module Class",
        "preparation_class_changed",
    )
}
fn association(raw: &Value) -> Result<ControllerAssociation> {
    let records = raw["records"].as_array().ok_or("inspection_records")?;
    let rows: Vec<_> = records
        .iter()
        .filter(|r| r["state"] == "ap8_controller_association")
        .collect();
    if rows.is_empty() {
        return Ok(ControllerAssociation::Unavailable{reason:"Retained inspector initialized the controller but did not retain its returned class identity".into()});
    }
    require(rows.len() == 1, "controller_association_ambiguous")?;
    if rows[0]["combined"] == true {
        return Ok(ControllerAssociation::Combined);
    }
    let cid = rows[0]["class_id"]
        .as_str()
        .ok_or("controller_association_missing")?
        .to_uppercase();
    require(valid_hex(&cid, 32), "controller_association_invalid")?;
    require(
        crate::inventory::classes(raw)?
            .iter()
            .any(|c| c.id == cid && c.category != "Audio Module Class"),
        "controller_association_not_in_factory",
    )?;
    Ok(ControllerAssociation::Separate { class_id: cid })
}
pub fn inspect_record(s: Selection, report: Artifact, origin: Origin) -> Result<Inspection> {
    let host = s.scanner.clone();
    let source_manifest = Artifact {
        path: host.path.with_file_name("host-source-manifest.json"),
        sha256: s.scanner_source.clone(),
    };
    inspect_record_with(s, report, origin, host, source_manifest)
}
pub fn inspect_record_with(
    s: Selection,
    report: Artifact,
    origin: Origin,
    host: Artifact,
    source_manifest: Artifact,
) -> Result<Inspection> {
    report.verify()?;
    host.verify()?;
    source_manifest.verify()?;
    let raw: Value = bounded(&report.path)?;
    Census::from_report(
        crate::catalogue::EnvironmentBinding {
            family: Family::ManagedInstallerV1,
            environment: s.environment.clone(),
        },
        s.module.clone(),
        ModuleStamp::read(&s.module.path)?,
        host.clone(),
        source_manifest.sha256.clone(),
        report.clone(),
        &s.class.id,
    )?;
    Ok(Inspection {
        schema: 1,
        controller: association(&raw)?,
        selection: s,
        report,
        host,
        source_manifest,
        origin,
    })
}
pub fn retain_inspection(m: &Manager, i: &Inspection) -> Result<()> {
    let dir = object(m, "inspections", &i.selection.id()?)?;
    immutable(&dir.join(format!("{}.json", i.id()?)), i)?;
    retain_inspection_order(m, i)?;
    changed(m)
}
pub fn inspection(m: &Manager, s: &Selection) -> Result<Option<Inspection>> {
    recommended_inspection(m, s)
}
/// Import original authority into durable generic history; no product action.
fn adopt_sv1(m: &Manager, s: &Selection) -> Result<Option<Candidate>> {
    let p = crate::managed_candidate::candidate()?;
    if p.class.class_id != s.class.id || p.module_sha256 != s.module.sha256 {
        return Ok(None);
    }
    let b: Value =
        serde_json::from_slice(include_bytes!("../../../compatibility/sv1/binding.json"))?;
    if b["environment"] != s.environment.id {
        return Ok(None);
    }
    let dir = m
        .root
        .join("software/sv1-qualification")
        .join(p.fingerprint()?);
    if !dir.exists() {
        return Ok(None);
    }
    for (path, field) in [
        (
            s.environment.root.join("environment.json"),
            "environment_sha256",
        ),
        (
            m.root
                .join("onboarding")
                .join(&s.environment.id)
                .join("record.json"),
            "onboarding_sha256",
        ),
        (
            m.root
                .join("inventory")
                .join(format!("{}.json", s.environment.id)),
            "inventory_sha256",
        ),
    ] {
        require(
            digest(&path)? == b[field].as_str().ok_or("sv1_binding_field")?,
            "sv1_provenance_changed",
        )?;
    }
    let c = crate::qualification::load_for(m, p.clone(), Qualification::Sv1Instrument)?;
    let report = Artifact {
        path: dir.join("inspection.json"),
        sha256: b["inspection_sha256"]
            .as_str()
            .ok_or("sv1_inspection_identity")?
            .into(),
    };
    let i = inspect_record_with(
        s.clone(),
        report,
        Origin::RetainedSv1,
        c.host.clone(),
        c.source_manifest.clone(),
    )?;
    let candidate = Candidate {
        schema: 1,
        selection: s.clone(),
        inspection: i,
        profile: p,
        native: c.native,
        host: c.host,
        source_manifest: c.source_manifest,
        origin: Origin::RetainedSv1,
        recipe_sha256: "retained-sv1".into(),
        preparation_basis: None,
    };
    materialize_legacy(m, &candidate, &b)?;
    Ok(Some(candidate))
}
pub fn candidates(m: &Manager, _host: &Artifact, _source: &str) -> Result<Vec<Candidate>> {
    let mut out = retained_candidates(m)?;
    // Once adopted, mutable inventory is never consulted to reconstruct history.
    let legacy = crate::managed_candidate::candidate()?;
    if !out.iter().any(|c| c.origin == Origin::RetainedSv1) {
        for path in list(&m.root.join("inventory"))? {
            if path.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let scan: crate::inventory::Scan = bounded(&path)?;
            for module in scan.modules {
                for class in module.classes {
                    if class.id != legacy.class.class_id
                        || module.artifact.sha256 != legacy.module_sha256
                    {
                        continue;
                    }
                    let s = Selection {
                        schema: 1,
                        environment: scan.environment.clone(),
                        module: module.artifact.clone(),
                        class,
                        scanner: scan.host.clone(),
                        scanner_source: scan.host_source_sha256.clone(),
                        factory_report: module.report.clone(),
                    };
                    if let Some(c) = adopt_sv1(m, &s)? {
                        out.push(c);
                    }
                }
            }
        }
    }
    Ok(out)
}
pub fn candidate(m: &Manager, id: &str, host: &Artifact, source: &str) -> Result<Candidate> {
    require(valid_hex(id, 64), "candidate_identity")?;
    candidates(m, host, source)?
        .into_iter()
        .find(|c| c.id().is_ok_and(|v| v == id))
        .ok_or_else(|| "candidate_absent".into())
}
pub fn verify_candidate(m: &Manager, c: &Candidate, host: &Artifact, source: &str) -> Result<()> {
    verify_selection(m, &c.selection, host, source)?;
    verify_retained_candidate(m, c)
}
pub fn verify_retained_candidate(m: &Manager, c: &Candidate) -> Result<()> {
    let host = &c.selection.scanner;
    let source = c.selection.scanner_source.as_str();
    verify_selection_data(m, &c.selection, host, source)?;
    require(
        c.schema == 1
            && c.inspection.selection == c.selection
            && c.profile.claim == Claim::ReviewCandidate
            && c.profile.class.class_id == c.selection.class.id,
        "candidate_binding",
    )?;
    require(
        c.profile.capabilities.compatibility() == Compatibility::default(),
        "candidate_policy_requires_explicit_support",
    )?;
    require(
        c.host == c.inspection.host && c.source_manifest == c.inspection.source_manifest,
        "candidate_host_changed",
    )?;
    if c.origin == Origin::RetainedSv1 {
        verify_legacy(m, c)?;
    } else if c.host.sha256 != host.sha256 || c.source_manifest.sha256 != source {
        build::verify_runtime(m, c)?;
    }
    c.source_manifest.verify()?;
    c.host.verify()?;
    c.native.matches(&c.profile)?;
    c.native.artifact.verify()?;
    require(
        inspect_record_with(
            c.selection.clone(),
            c.inspection.report.clone(),
            c.inspection.origin.clone(),
            c.host.clone(),
            c.source_manifest.clone(),
        )? == c.inspection,
        "candidate_inspection_changed",
    )?;
    let census = c.census()?;
    let reg = crate::observation::derive_for(
        &c.profile,
        &census,
        &c.native,
        SelectionPurpose::Qualification,
    )?;
    reg.verify(&m.root)
}
pub fn record_candidate(m: &Manager, c: &Candidate) -> Result<String> {
    let id = c.id()?;
    let d = object(m, "candidates", &id)?;
    immutable(&d.join("candidate.json"), c)?;
    retain_lineage(m, c, &id, None)?;
    changed(m)?;
    Ok(id)
}
/// Reuse is by complete identity. Native bytes must come from the fixed builder,
/// never from an operator-supplied path or manifest.
pub fn prepared(
    s: Selection,
    i: Inspection,
    native: NativeArtifact,
    host: Artifact,
    manifest: Artifact,
    recipe: String,
) -> Result<Candidate> {
    require(
        s == i.selection
            && native.module_sha256 == s.module.sha256
            && native.class.class_id == s.class.id,
        "preparation_inputs_changed",
    )?;
    let raw: Value = bounded(&i.report.path)?;
    require(raw["error"].is_null(), "inspection_failed")?;
    let profile = Profile {
        schema: 1,
        id: format!(
            "managed.{}",
            key(&(&s, &i, &native, &host, &manifest, &recipe))?
        ),
        revision: 1,
        claim: Claim::ReviewCandidate,
        module_sha256: s.module.sha256.clone(),
        factory_vendor: native.class.vendor.clone(),
        class: native.class.clone(),
        role: role(&native.class)?,
        requirements: Requirements {
            runner: runner_match(&s.environment.runner)?,
            environment_family: Family::ManagedInstallerV1,
            environment_revision: s.environment.revision,
            host_sha256: host.sha256.clone(),
            host_source_sha256: manifest.sha256.clone(),
            native_sha256: native.artifact.sha256.clone(),
            native_source_commit: native.source_commit.clone(),
            descriptor_sha256: native.descriptor_sha256.clone(),
        },
        capabilities: Capabilities {
            accessibility: Accessibility::WindowsDefault,
            editor: Editor::DetachedDirectVendorLifecycle,
            state: State::ConcurrentReadOnlyCaptureV12,
            precision: Precision::Float32Only,
            performance: PerformancePolicy::Frames512Recommended256Unqualified,
            vendor_retirement: None,
            editor_lifetime: None,
            event_output: None,
        },
        limitations: vec![
            Limitation::DirectEditorUnderQualification,
            Limitation::Unqualified256,
            Limitation::DetachedFocusRefusal,
        ],
        evidence: vec!["docs/MF3.md".into()],
    };
    profile.validate()?;
    Ok(Candidate {
        schema: 1,
        selection: s,
        inspection: i,
        profile,
        native,
        host,
        source_manifest: manifest,
        origin: Origin::ManagedPreparation,
        recipe_sha256: recipe,
        preparation_basis: None,
    })
}
fn evidence_dir(m: &Manager, id: &str) -> Result<PathBuf> {
    object(m, "observations", id)
}
pub fn observations(m: &Manager, c: &Candidate) -> Result<Vec<Observation>> {
    let mut out = vec![];
    // Original observations remain labelled with their original provenance.
    if c.origin == Origin::RetainedSv1 {
        let report = legacy_provenance(m, c)?.original_evidence;
        out.extend(retained_sv1_observations(c, &report)?);
    }
    for p in list(&evidence_dir(m, &c.id()?)?)? {
        let o: Observation = bounded(&p)?;
        require(
            o.schema == 1 && o.candidate == c.id()?,
            "observation_binding",
        )?;
        out.push(o)
    }
    out.sort_by_key(|o| (o.ordinal, o.operation.clone()));
    Ok(out)
}
fn retained_sv1_observations(c: &Candidate, report: &Value) -> Result<Vec<Observation>> {
    let mut out = vec![];
    if report["module_sha256"] == c.selection.module.sha256
        && report["class_id"] == c.selection.class.id
        && report["host_sha256"] == c.host.sha256
        && report["host_source_manifest_sha256"] == c.source_manifest.sha256
    {
        for (area,status,witness,detail) in [(Area::DawLoad,TestStatus::Passed,Witness::MachineObservation,"SV1 retained exact instrument processing session"),(Area::Editor,TestStatus::Passed,Witness::OperatorObservation,"Operator reported responsive editor before failure"),(Area::Parameters,TestStatus::Passed,Witness::MachineObservation,"SV1 retained two complete gestures and 24 value events"),(Area::ProcessingRestart,TestStatus::Failed,Witness::MachineObservation,"Terminal lifecycle correlation failure on processing restart"),(Area::Retirement,TestStatus::NotTested,Witness::MachineObservation,"Normal retirement was not observed; positive terminal cleanup is retained separately")]{out.push(Observation{schema:1,candidate:c.id()?,operation:"retained-sv1".into(),area,status,witness,detail:detail.into(),recorded_at:0,ordinal:0})}
    }
    Ok(out)
}

fn detail(s: &str) -> Result<()> {
    require(
        !s.trim().is_empty() && s.len() <= 512 && !s.chars().any(char::is_control),
        "observation_detail_bound",
    )
}
pub fn record_observation(
    m: &Manager,
    c: &Candidate,
    operation: &str,
    area: Area,
    status: TestStatus,
    note: &str,
) -> Result<()> {
    require(valid_hex(operation, 32), "observation_operation")?;
    detail(note)?;
    let _owner = m.lock("preparation-evidence.lock")?;
    let ordinal = observations(m, c)?
        .iter()
        .map(|o| o.ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("observation_ordinal")?;
    let o = Observation {
        schema: 1,
        candidate: c.id()?,
        operation: operation.into(),
        area,
        status,
        witness: Witness::OperatorObservation,
        detail: note.into(),
        recorded_at: crate::observation::now()?,
        ordinal,
    };
    immutable(
        &evidence_dir(m, &c.id()?)?.join(format!("{operation}.json")),
        &o,
    )?;
    changed(m)
}
pub fn unmet(observations: &[Observation], role: &Role) -> Vec<String> {
    AREAS
        .iter()
        .filter(|a| !(**a == Area::Midi && *role == Role::Effect))
        .filter_map(|a| {
            let latest = observations.iter().rev().find(|o| o.area == *a);
            if !observations
                .iter()
                .any(|o| o.area == *a && o.status == TestStatus::Failed)
                && latest.is_some_and(|o| {
                    o.status == TestStatus::Passed && o.witness != Witness::GeneratedRegression
                })
            {
                None
            } else {
                Some(format!(
                    "{a:?}: a retained passing product observation is required"
                ))
            }
        })
        .collect()
}
pub fn decisions(m: &Manager, c: &Candidate) -> Result<Vec<Decision>> {
    let mut out = vec![];
    for p in list(&object(m, "reviews", &c.id()?)?)? {
        let d: Decision = bounded(&p)?;
        require(d.schema == 1 && d.candidate == c.id()?, "review_binding")?;
        out.push(d)
    }
    out.sort_by_key(|d| (d.ordinal, d.operation.clone()));
    Ok(out)
}
pub fn review(
    m: &Manager,
    c: &Candidate,
    operation: &str,
    choice: ReviewChoice,
    rationale: &str,
) -> Result<Decision> {
    detail(rationale)?;
    require(valid_hex(operation, 32), "review_operation")?;
    let _owner = m.lock("preparation-evidence.lock")?;
    let ordinal = decisions(m, c)?
        .iter()
        .map(|d| d.ordinal)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("review_ordinal")?;
    let observations = observations(m, c)?;
    let profile = if choice == ReviewChoice::AcceptExactLocal {
        require(
            unmet(&observations, &c.profile.role).is_empty(),
            "qualification_requirements_unmet",
        )?;
        let mut p = c.profile.clone();
        p.revision = p
            .revision
            .checked_add(1)
            .ok_or("profile_revision_overflow")?;
        p.claim = Claim::VerifiedExactFixture;
        p.evidence
            .push(format!("evidence/mf3/reviews/{operation}.json"));
        p.validate()?;
        Some(p)
    } else {
        None
    };
    let d = Decision {
        schema: 1,
        candidate: c.id()?,
        operation: operation.into(),
        evidence_sha256: key(&observations)?,
        choice,
        rationale: rationale.into(),
        authority: "explicit local maintainer review; not project-wide support certification"
            .into(),
        recorded_at: crate::observation::now()?,
        ordinary_profile: profile,
        ordinal,
    };
    immutable(
        &m.root
            .join("evidence/mf3/reviews")
            .join(format!("{operation}.json")),
        &d,
    )?;
    immutable(
        &object(m, "reviews", &c.id()?)?.join(format!("{operation}.json")),
        &d,
    )?;
    changed(m)?;
    Ok(d)
}
pub fn accepted(m: &Manager, c: &Candidate) -> Result<Profile> {
    let d = decisions(m, c)?
        .pop()
        .ok_or("qualification_review_required")?;
    require(
        d.choice == ReviewChoice::AcceptExactLocal
            && d.evidence_sha256 == key(&observations(m, c)?)?
            && unmet(&observations(m, c)?, &c.profile.role).is_empty(),
        "qualification_review_stale_or_unmet",
    )?;
    d.ordinary_profile
        .ok_or_else(|| "qualification_review_required".into())
}
pub fn publication_state(m: &Manager, c: &Candidate) -> Result<String> {
    if m.publication_pending(&c.selection.class.id)? {
        return Ok("needs_attention".into());
    }
    let db = m.registry()?;
    let Some(e) = db.classes.get(&c.selection.class.id) else {
        return Ok(if physical(&m.link(&c.selection.class.id))?.is_some() {
            "needs_attention"
        } else {
            "unpublished"
        }
        .into());
    };
    if e.publication != Publication::Published {
        return Ok(if physical(&m.link(&c.selection.class.id))?.is_none() {
            "removed"
        } else {
            "needs_attention"
        }
        .into());
    }
    let r = m.load_revision(
        &c.selection.class.id,
        e.managed_revision
            .as_ref()
            .ok_or("publication_revision_missing")?,
    )?;
    if physical(&m.link(&c.selection.class.id))? != Some(r.target.clone())
        || m.publication_pending(&c.selection.class.id)?
    {
        return Ok("needs_attention".into());
    }
    if r.profile != c.profile && !accepted_profile(m, c, &r.profile)? {
        return Ok("another_configuration".into());
    }
    Ok(if r.qualification.is_some() {
        "experimental"
    } else {
        "ordinary"
    }
    .into())
}
fn preliminary(i: Option<&Inspection>) -> Result<Value> {
    let Some(i) = i else { return Ok(Value::Null) };
    let raw: Value = bounded(&i.report.path)?;
    let rows = raw["records"].as_array().ok_or("inspection_records")?;
    let one = |state: &str| {
        rows.iter()
            .find(|r| r["state"] == state)
            .cloned()
            .unwrap_or(Value::Null)
    };
    let buses: Vec<Value>=rows.iter().filter(|r|r["state"]=="ap8_bus").map(|r|serde_json::json!({"media":r["media"],"direction":r["direction"],"index":r["index"],"channels":r["channels"],"type":r["type"],"arrangement":r["arrangement"]})).collect();
    Ok(
        serde_json::json!({"component_initialized":true,"controller_initialized":true,"buses":buses,"parameter_count":one("ap8_parameter_count")["count"],"precision":one("ap12_capabilities"),"state":one("ap12_persistence"),"editor_interface":one("ap8_editor_interface"),"editor_attached":false,"cleanup_confirmed":raw["cleanup_confirmed"]}),
    )
}

fn runner_match(r: &Runner) -> Result<RunnerMatch> {
    let hash = |path: &Path| -> Result<String> {
        r.files
            .iter()
            .find(|a| a.path == path)
            .map(|a| a.sha256.clone())
            .ok_or_else(|| "runner_entry_missing".into())
    };
    let mut files: Vec<_> = r.files.iter().map(|a| a.sha256.clone()).collect();
    files.sort();
    files.dedup();
    Ok(RunnerMatch {
        id: r.id.clone(),
        version: r.version.clone(),
        proton_sha256: hash(&r.proton)?,
        entry_point_sha256: hash(&r.entry_point)?,
        file_sha256: files,
    })
}

fn for_profile(m: &Manager, p: &Profile) -> Result<Option<Candidate>> {
    let mut found = vec![];
    for path in list(&root(m).join("candidates"))? {
        let c: Candidate = bounded(&path.join("candidate.json"))?;
        require(
            path.file_name().and_then(|n| n.to_str()) == Some(c.id()?.as_str()),
            "candidate_identity",
        )?;
        if c.profile == *p
            || (p.claim == Claim::VerifiedExactFixture && accepted_profile(m, &c, p)?)
        {
            found.push(c)
        }
    }
    require(found.len() <= 1, "candidate_identity_ambiguous")?;
    Ok(found.pop())
}
pub(crate) fn owns_profile(m: &Manager, p: &Profile) -> Result<bool> {
    Ok(for_profile(m, p)?.is_some())
}
/// Used after service admission to keep per-class runtime identity separate from
/// the environment keeper and the maintenance inspector.
pub fn session_binding(
    m: &Manager,
    class: &str,
    environment: &Environment,
    module: &Artifact,
    host: &Artifact,
    source: &str,
) -> Result<bool> {
    let db = m.registry()?;
    let Some(e) = db.classes.get(class) else {
        return Ok(false);
    };
    let Some(reference) = &e.managed_revision else {
        return Ok(false);
    };
    let r = m.load_revision(class, reference)?;
    if !owns_profile(m, &r.profile)? {
        return Ok(false);
    }
    require(
        r.registration.environment == *environment
            && r.registration.module == *module
            && r.registration.host == *host
            && r.registration.host_source_sha256 == source,
        "managed_session_binding_changed",
    )?;
    Ok(true)
}
pub(crate) fn check_publication(m: &Manager, p: &Profile, r: &Registration) -> Result<()> {
    let c = for_profile(m, p)?.ok_or("candidate_preparation_required")?;
    verify_retained_candidate(m, &c)?;
    let mut expected = crate::observation::derive_for(
        p,
        &c.census()?,
        &c.native,
        if p.claim == Claim::ReviewCandidate {
            SelectionPurpose::Qualification
        } else {
            SelectionPurpose::Activation
        },
    )?;
    expected.native.path = r.native.path.clone();
    require(expected == *r, "candidate_registration_changed")
}
pub(crate) fn retained(m: &Manager, r: &Revision) -> Result<()> {
    check_publication(m, &r.profile, &r.registration)?;
    require(
        r.performance.added_frames == 512 && r.external_ids == external_ids(&r.class_id)?,
        "candidate_runtime_contract",
    )?;
    require(
        (r.profile.claim == Claim::ReviewCandidate
            && r.qualification == Some(Qualification::ManagedExperimental))
            || (r.profile.claim == Claim::VerifiedExactFixture && r.qualification.is_none()),
        "candidate_authority_kind",
    )?;
    let db = m.registry()?;
    let e = db
        .classes
        .get(&r.class_id)
        .ok_or("candidate_not_published")?;
    let reference = e
        .managed_revision
        .as_ref()
        .ok_or("candidate_not_published")?;
    require(
        reference.id == r.id
            && e.registration == r.registration
            && e.publication == Publication::Published
            && physical(&m.link(&r.class_id))? == Some(r.target.clone())
            && !m.publication_pending(&r.class_id)?,
        "candidate_not_published",
    )?;
    m.verify_completed_publication(r, reference)
}
pub(crate) fn permits_transition(
    m: &Manager,
    p: &Profile,
    prior: &Revision,
    q: Option<Qualification>,
) -> Result<bool> {
    if q != Some(Qualification::ManagedExperimental) && q.is_some() {
        return Ok(false);
    }
    let Some(c) = for_profile(m, p)? else {
        return Ok(false);
    };
    // A prior experimental publication must be this exact prepared candidate.
    // Ordinary rollback parents retain their own independent accepted authority.
    Ok(prior.profile == c.profile
        && matches!(
            prior.qualification,
            Some(Qualification::ManagedExperimental | Qualification::Sv1Instrument)
        )
        && (p == &c.profile || accepted(m, &c).is_ok_and(|a| a == *p)))
}
pub fn enable(m: &Manager, c: &Candidate, ordinary: bool) -> Result<RevisionRef> {
    enable_exact(m, c, ordinary, None)
}
pub fn replace(m: &Manager, c: &Candidate, expected: &RevisionRef) -> Result<RevisionRef> {
    require(
        publication_state(m, c)? == "another_configuration",
        "replacement_not_required",
    )?;
    enable_exact(m, c, false, Some(expected))
}
fn enable_exact(
    m: &Manager,
    c: &Candidate,
    ordinary: bool,
    expected: Option<&RevisionRef>,
) -> Result<RevisionRef> {
    require(
        !ordinary || publication_state(m, c)? != "ordinary",
        "candidate_already_ordinary",
    )?;
    verify_candidate(m, c, &c.selection.scanner, &c.selection.scanner_source)?;
    require(
        publication_state(m, c)? != "needs_attention"
            && (publication_state(m, c)? != "another_configuration" || expected.is_some()),
        "explicit_replacement_or_reconciliation_required",
    )?;
    // Adopting retained CLI provenance grants no publication by itself.
    record_candidate(m, c)?;
    let p = if ordinary {
        accepted(m, c)?
    } else {
        c.profile.clone()
    };
    let census = c.census()?;
    let r = crate::observation::derive_for(
        &p,
        &census,
        &c.native,
        if ordinary {
            SelectionPurpose::Activation
        } else {
            SelectionPurpose::Qualification
        },
    )?;
    m.publish_with_expected(
        &p,
        &census,
        r,
        (&c.host, &c.source_manifest.sha256),
        (
            if ordinary {
                None
            } else {
                Some(Qualification::ManagedExperimental)
            },
            true,
        ),
        None,
        expected,
    )
}
pub fn disable(m: &Manager, c: &Candidate) -> Result<()> {
    let db = m.registry()?;
    let e = db
        .classes
        .get(&c.selection.class.id)
        .ok_or("candidate_not_published")?;
    let r = m.load_revision(
        &c.selection.class.id,
        e.managed_revision
            .as_ref()
            .ok_or("candidate_not_published")?,
    )?;
    require(
        r.profile == c.profile
            && matches!(
                r.qualification,
                Some(Qualification::ManagedExperimental | Qualification::Sv1Instrument)
            ),
        "not_selected_experimental_publication",
    )?;
    let mut parent = r.parent.clone();
    let mut visited = std::collections::BTreeSet::new();
    while let Some(reference) = parent {
        require(
            visited.len() < 64 && visited.insert(reference.id.clone()),
            "candidate_ancestry_cycle",
        )?;
        let prior = m.load_revision(&r.class_id, &reference)?;
        if prior.qualification.is_none() && prior.profile.claim == Claim::VerifiedExactFixture {
            m.rollback_exact_inactive(
                &r.class_id,
                &reference.id,
                e.managed_revision
                    .as_ref()
                    .ok_or("candidate_not_published")?,
            )?;
            return Ok(());
        }
        parent = prior.parent;
    }
    m.unpublish_exact_inactive(
        &r.class_id,
        e.managed_revision
            .as_ref()
            .ok_or("candidate_not_published")?,
    )
}

fn changed(m: &Manager) -> Result<()> {
    private_dir(&root(m))?;
    atomic_json(&root(m).join("revision.json"), &random_id()?)
}
#[cfg(test)]
mod tests;

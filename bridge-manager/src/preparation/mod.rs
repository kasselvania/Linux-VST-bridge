//! MF3: canonical selection, immutable preparation and explicit local review.
//! No frontend path, process, profile or compiler authority enters this owner.
pub mod build;
pub mod configuration;
mod accessibility;
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
fn guided_check_path(m: &Manager, operation: &str, stage: &str) -> Result<PathBuf> {
    require(valid_hex(operation, 32), "guided_check_operation")?;
    require(matches!(stage, "intent" | "inspection" | "candidate" | "completed"),
        "guided_check_stage")?;
    Ok(root(m).join("guided-checks").join(operation).join(format!("{stage}.json")))
}
pub fn guided_check_stage(m: &Manager, operation: &str, stage: &str) -> Result<Option<Value>> {
    let path = guided_check_path(m, operation, stage)?;
    if path.exists() { Ok(Some(bounded(&path)?)) } else { Ok(None) }
}
pub fn retain_guided_check_stage(
    m: &Manager, operation: &str, stage: &str, action: &Value, value: &Value,
) -> Result<()> {
    let path = guided_check_path(m, operation, stage)?;
    let record = serde_json::json!({"schema":1,"operation":operation,"action":action,"value":value});
    immutable(&path, &record)?;
    changed(m)
}
pub fn guided_checks(m: &Manager, selection: &str) -> Result<Vec<Value>> {
    let mut rows = vec![];
    for path in list(&root(m).join("guided-checks"))? {
        let operation = path.file_name().and_then(|name| name.to_str())
            .ok_or("guided_check_history_path")?;
        require(path.is_dir() && valid_hex(operation, 32), "guided_check_history_identity")?;
        let intent = guided_check_stage(m, operation, "intent")?
            .ok_or("guided_check_intent_missing")?;
        require(intent["schema"] == 1 && intent["operation"] == operation
            && intent["action"]["kind"] == "compatibility_check",
            "guided_check_history_identity")?;
        if intent["action"]["selection"] != selection { continue; }
        let completed = guided_check_stage(m, operation, "completed")?;
        let inspection = guided_check_stage(m, operation, "inspection")?;
        let candidate = guided_check_stage(m, operation, "candidate")?;
        for stage in [&completed, &inspection, &candidate].into_iter().flatten() {
            require(stage["schema"] == 1 && stage["operation"] == operation
                && stage["action"] == intent["action"], "guided_check_stage_binding")?;
        }
        if let Some(done) = &completed {
            require(done["value"]["schema"] == 1
                && done["value"]["operation"] == operation,
                "guided_check_completion_binding")?;
        }
        let row = completed.map_or_else(|| serde_json::json!({
            "schema":1,"operation":operation,"selection":selection,
            "stage": if candidate.is_some() { "candidate_retained" }
                else if inspection.is_some() { "inspection_retained" } else { "started" },
            "inspection":inspection.as_ref().map(|s| s["value"]["id"].clone()),
            "candidate":candidate.as_ref().map(|s| s["value"]["id"].clone()),
            "publication_changed":false,
        }), |done| done["value"].clone());
        rows.push(row);
    }
    Ok(rows)
}
pub fn retain_guided_result(m: &Manager, operation: &str, action: &Value) -> Result<()> {
    require(valid_hex(operation, 32), "guided_result_operation")?;
    let value = serde_json::json!({"schema":1,"operation":operation,"action":action});
    immutable(&root(m).join("guided-results").join(operation).join("intent.json"), &value)?;
    changed(m)
}
pub fn guided_result_intent(m: &Manager, operation: &str) -> Result<Value> {
    require(valid_hex(operation, 32), "guided_result_operation")?;
    let value: Value = bounded(&root(m).join("guided-results").join(operation).join("intent.json"))?;
    require(value["schema"] == 1 && value["operation"] == operation
        && value["action"]["kind"] == "compatibility_result", "guided_result_intent_binding")?;
    Ok(value["action"].clone())
}
pub fn complete_guided_result(m: &Manager, operation: &str) -> Result<()> {
    let action = guided_result_intent(m, operation)?;
    immutable(&root(m).join("guided-results").join(operation).join("completed.json"),
        &serde_json::json!({"schema":1,"operation":operation,"action":action}))?;
    changed(m)
}
pub fn guided_results(m: &Manager, candidate: &str) -> Result<Vec<Value>> {
    require(valid_hex(candidate, 64), "guided_result_candidate")?;
    let mut result = vec![];
    for path in list(&root(m).join("guided-results"))? {
        let operation = path.file_name().and_then(|name| name.to_str()).ok_or("guided_result_path")?;
        let action = guided_result_intent(m, operation)?;
        if action["candidate"] != candidate { continue; }
        let completed = path.join("completed.json");
        if completed.exists() {
            let marker: Value = bounded(&completed)?;
            require(marker["schema"] == 1 && marker["operation"] == operation
                && marker["action"] == action, "guided_result_completion_binding")?;
        }
        result.push(serde_json::json!({"operation":operation,"action":action,
            "completed":completed.exists()}));
    }
    Ok(result)
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
    for e in fs::read_dir(dir)? {
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
// Environment identifiers are opaque. Retained installations use UUIDs while
// managed installer environments use compact identifiers. The record and exact
// direct-child location supply authority, as in Registration::verify; spelling
// must not exclude an existing installation from preparing an update.
fn environment_location(m: &Manager, environment: &Environment) -> bool {
    let parent = m.root.join("environments");
    environment.revision > 0
        && !environment.id.is_empty()
        && environment.root.parent() == Some(parent.as_path())
        && environment.root.file_name().and_then(|name| name.to_str())
            == Some(environment.id.as_str())
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
        if !environment_location(m, &scan.environment)
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
    let s = select_record(m, id, host, source)?;
    verify_selection_data(m, &s, host, source)?;
    Ok(s)
}
pub fn select_record(m: &Manager, id: &str, host: &Artifact, source: &str) -> Result<Selection> {
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
    validate_selection_data(m, &s, host, source)?;
    Ok(s)
}
pub fn verify_selection(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<()> {
    validate_selection_record(m, s, host, source)?;
    verify_selection_data(m, s, host, source)
}
pub fn validate_selection_record(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<()> {
    require(
        selections(m, host, source)?
            .iter()
            .any(|current| current == s),
        "preparation_inventory_superseded",
    )?;
    validate_selection_data(m, s, host, source)
}
fn verify_selection_data(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<()> {
    verify_selection_artifacts(m, s, host, source)?;
    s.environment.runner.verify()
}
fn verify_selection_artifacts(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<()> {
    validate_selection_data(m, s, host, source)?;
    s.module.verify()?;
    s.scanner.verify()
}
fn validate_selection_data(m: &Manager, s: &Selection, host: &Artifact, source: &str) -> Result<()> {
    validate_selection_data_with_current(m,s,host,source,true)
}
fn validate_selection_data_with_current(m: &Manager, s: &Selection, host: &Artifact,
    source: &str, current: bool) -> Result<()> {
    require(
        s.schema == 1 && s.scanner.sha256 == host.sha256 && s.scanner_source == source,
        "preparation_scanner_changed",
    )?;
    require(
        environment_location(m, &s.environment),
        "preparation_environment_location",
    )?;
    require(
        !current || bounded::<Environment>(&s.environment.root.join("environment.json"))? == s.environment,
        "preparation_environment_changed",
    )?;
    require(
        s.module
            .path
            .starts_with(s.environment.root.join("compatdata/pfx/drive_c"))
            && (!current || s.module.path.canonicalize()? == s.module.path),
        "preparation_module_location",
    )?;
    if current {
        s.module.validate_record()?;
        s.scanner.validate_record()?;
        s.environment.runner.validate_record()?;
    } else {
        require(s.module.path.is_absolute() && valid_hex(&s.module.sha256,64)
            && s.scanner.path.is_absolute() && valid_hex(&s.scanner.sha256,64),
            "preparation_artifact_identity")?;
    }
    let raw: Value = s.factory_report.read_record(8 * 1024 * 1024)?;
    let classes = crate::inventory::classes(&raw)?;
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
    inspect_record_with_layout(s, report, origin, host, source_manifest, None)
}
pub fn inspect_record_with_layout(
    s: Selection,
    report: Artifact,
    origin: Origin,
    host: Artifact,
    source_manifest: Artifact,
    audio_layout: Option<AudioLayoutPolicy>,
) -> Result<Inspection> {
    host.verify()?;
    source_manifest.verify()?;
    inspection_record_with_layout(s, report, origin, host, source_manifest, audio_layout)
}
fn inspection_record_with_layout(
    s: Selection, report: Artifact, origin: Origin, host: Artifact,
    source_manifest: Artifact, audio_layout: Option<AudioLayoutPolicy>,
) -> Result<Inspection> {
    let stamp=Some(ModuleStamp::read(&s.module.path)?);
    inspection_record_with_stamp(s,report,origin,host,source_manifest,audio_layout,stamp)
}
fn inspection_record_with_stamp(s: Selection, report: Artifact, origin: Origin, host: Artifact,
    source_manifest: Artifact, audio_layout: Option<AudioLayoutPolicy>, stamp:Option<ModuleStamp>)
    -> Result<Inspection> {
    host.validate_record()?;
    source_manifest.validate_record()?;
    let raw: Value = report.read_record(8 * 1024 * 1024)?;
    Census::from_report(
        crate::catalogue::EnvironmentBinding {
            family: Family::ManagedInstallerV1,
            environment: s.environment.clone(),
        },
        s.module.clone(),
        stamp,
        host.clone(),
        source_manifest.sha256.clone(),
        report.clone(),
        &s.class.id,
    )?;
    verify_audio_layout(&raw, audio_layout.as_ref())?;
    Ok(Inspection {
        schema: 1,
        controller: association(&raw)?,
        selection: s,
        report,
        host,
        source_manifest,
        origin,
        audio_layout,
    })
}

fn verify_audio_layout(raw: &Value, policy: Option<&AudioLayoutPolicy>) -> Result<()> {
    let records = raw["records"].as_array().ok_or("inspection_records")?;
    let applied: Vec<_> = records
        .iter()
        .filter(|row| row["state"] == "ap18_audio_layout")
        .collect();
    let Some(AudioLayoutPolicy::StereoMainPair) = policy else {
        return require(applied.is_empty(), "unexpected_audio_layout_application");
    };
    require(
        applied.len() == 1
            && applied[0]["policy"] == "stereo_main_pair"
            && applied[0]["input_count"] == 1
            && applied[0]["output_count"] == 1
            && applied[0]["result"] == 0
            && applied[0]["verified"] == true,
        "stereo_main_pair_application_missing",
    )?;
    let audio: Vec<_> = records
        .iter()
        .filter(|row| row["state"] == "ap8_bus" && row["media"] == 0)
        .collect();
    require(audio.len() == 2, "stereo_main_pair_bus_count")?;
    for direction in 0..=1 {
        let rows: Vec<_> = audio
            .iter()
            .filter(|row| row["direction"] == direction)
            .collect();
        require(
            rows.len() == 1
                && rows[0]["index"] == 0
                && rows[0]["type"] == 0
                && rows[0]["channels"] == 2
                && rows[0]["arrangement"] == 3,
            "stereo_main_pair_readback",
        )?;
    }
    Ok(())
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
        local_settings: None,
        settings_trial: None,
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
        launch_configuration_intent: None,
        touch_carry_forward: None,
    };
    materialize_legacy(m, &candidate, &b)?;
    Ok(Some(candidate))
}
/// Upgrade older candidate-only records before taking an operator state token.
/// This only completes retained history; it does not inspect, build or publish.
pub fn materialize_retained_history(m: &Manager) -> Result<Vec<Candidate>> {
    let binding = serde_json::from_slice(include_bytes!("../../../compatibility/sv1/binding.json"))?;
    retained_history_with_binding(m, &binding)
}
fn retained_history_with_binding(m: &Manager, binding: &Value) -> Result<Vec<Candidate>> {
    let out = retained_candidates(m)?;
    for c in out.iter().filter(|c| c.origin == Origin::RetainedSv1) {
        complete_legacy_history(m, c, binding)?;
    }
    Ok(out)
}
pub fn candidates(m: &Manager, _host: &Artifact, _source: &str) -> Result<Vec<Candidate>> {
    let mut out = materialize_retained_history(m)?;
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
/// Exact immutable ID lookup, without enumerating unrelated preparations.
pub fn candidate_record(m: &Manager, id: &str) -> Result<Candidate> {
    require(valid_hex(id, 64), "candidate_identity")?;
    let path = object(m, "candidates", id)?.join("candidate.json");
    require(path.try_exists()?, "candidate_absent")?;
    let c: Candidate = bounded(&path)?;
    require(c.schema == 1 && c.id()? == id, "candidate_identity")?;
    Ok(c)
}
pub fn verify_candidate(m: &Manager, c: &Candidate, host: &Artifact, source: &str) -> Result<()> {
    if matches!(c.origin, Origin::X11TouchReleaseV1 | Origin::X11TouchRoutingV2) {
        require(
            (c.selection.scanner.sha256 == host.sha256 && c.selection.scanner_source == source)
                || (c.host.sha256 == host.sha256 && c.source_manifest.sha256 == source),
            "touch_current_runtime",
        )?;
        verify_touch_carry_forward(m, c)?;
    } else {
        require(c.touch_carry_forward.is_none(), "touch_carry_forward_origin")?;
        // Keep the current inventory and caller binding. Retained verification
        // below checks these artifacts and fully verifies the same runner.
        validate_selection_record(m, &c.selection, host, source)?;
    }
    verify_retained_candidate(m, c)
}
pub fn validate_current_candidate_record(m: &Manager, c: &Candidate,
    host: &Artifact, source: &str) -> Result<()> {
    validate_selection_data(m,&c.selection,&c.selection.scanner,&c.selection.scanner_source)?;
    if matches!(c.origin, Origin::X11TouchReleaseV1 | Origin::X11TouchRoutingV2) {
        require((c.selection.scanner.sha256 == host.sha256 && c.selection.scanner_source == source)
            || (c.host.sha256 == host.sha256 && c.source_manifest.sha256 == source),
            "touch_current_runtime")?;
        validate_touch_record(m, c)?;
    } else {
        require(c.touch_carry_forward.is_none(), "touch_carry_forward_origin")?;
        validate_selection_record(m, &c.selection, host, source)?;
    }
    validate_candidate_record(m, c)
}
pub const SERUM_TOUCH_PREDECESSOR: &str =
    "f6af02eba109d3632b2ecc786f2c9006ec6bc1d433772001d24d2969fb944b44";
pub const SERUM_TOUCH_ROUTING_PREDECESSOR: &str =
    "91b699291eb7b7d1ff6e725d5e6fd1abed88ea721dbd81a621dd2fb39d38d207";

pub fn touch_successor(
    predecessor: &Candidate,
    after: &Environment,
    provenance: TouchCarryForward,
) -> Result<Candidate> {
    require(
        predecessor.id()? == SERUM_TOUCH_PREDECESSOR
            && predecessor.origin == Origin::ManagedPreparation
            && predecessor.selection.environment.id == after.id
            && predecessor.selection.environment.root == after.root
            && predecessor.selection.environment.revision.checked_add(1) == Some(after.revision)
            && after.runner.policy == Some(RunnerPolicy::X11TouchReleaseV1)
            && after.runner.id == "proton-11.0-2c-x11-touch-release-v1"
            && provenance.predecessor == SERUM_TOUCH_PREDECESSOR
            && predecessor.selection.class.id == "56534558667350736572756D20320000",
        "touch_carry_forward_predecessor",
    )?;
    carry_forward_touch_fields(predecessor, after, provenance, Origin::X11TouchReleaseV1)
}

pub fn touch_routing_successor(
    predecessor: &Candidate,
    after: &Environment,
    provenance: TouchCarryForward,
) -> Result<Candidate> {
    require(
        predecessor.id()? == SERUM_TOUCH_ROUTING_PREDECESSOR
            && predecessor.origin == Origin::X11TouchReleaseV1
            && predecessor.selection.environment.id == after.id
            && predecessor.selection.environment.root == after.root
            && predecessor.selection.environment.revision.checked_add(1) == Some(after.revision)
            && after.runner.policy == Some(RunnerPolicy::X11TouchRoutingV2)
            && after.runner.id == "proton-11.0-2c-x11-touch-routing-v2"
            && provenance.predecessor == SERUM_TOUCH_ROUTING_PREDECESSOR
            && predecessor.selection.class.id == "56534558667350736572756D20320000",
        "touch_routing_carry_forward_predecessor",
    )?;
    carry_forward_touch_fields(predecessor, after, provenance, Origin::X11TouchRoutingV2)
}

fn carry_forward_touch_fields(
    predecessor: &Candidate,
    after: &Environment,
    provenance: TouchCarryForward,
    origin: Origin,
) -> Result<Candidate> {
    let mut candidate = predecessor.clone();
    candidate.selection.environment = after.clone();
    candidate.inspection.selection = candidate.selection.clone();
    candidate.inspection.origin = origin.clone();
    candidate.origin = origin;
    candidate.profile.revision = candidate.profile.revision.checked_add(1).ok_or("touch_profile_revision")?;
    candidate.profile.requirements.environment_revision = after.revision;
    candidate.profile.requirements.runner = runner_match(&after.runner)?;
    candidate.profile.evidence.push("docs/PLUGIN_RELIABILITY_FOLLOWUP.md".into());
    candidate.touch_carry_forward = Some(provenance);
    candidate.profile.validate()?;
    Ok(candidate)
}

fn verify_touch_carry_forward(m: &Manager, c: &Candidate) -> Result<()> {
    validate_touch_record(m, c)?;
    c.touch_carry_forward.as_ref().ok_or("touch_carry_forward_absent")?.runner_manifest.verify()?;
    let prepared: Value = bounded(&c.touch_carry_forward.as_ref()
        .ok_or("touch_carry_forward_absent")?.transition.path.parent()
        .ok_or("touch_carry_forward_transition")?.join("transition.json"))?;
    let retired: RevisionRef = serde_json::from_value(prepared["removed_publication"].clone())?;
    m.load_revision(&c.selection.class.id, &retired)?;
    Ok(())
}
fn validate_touch_record(m: &Manager, c: &Candidate) -> Result<()> {
    let provenance = c.touch_carry_forward.as_ref().ok_or("touch_carry_forward_absent")?;
    let expected_predecessor = match c.origin {
        Origin::X11TouchReleaseV1 => SERUM_TOUCH_PREDECESSOR,
        Origin::X11TouchRoutingV2 => SERUM_TOUCH_ROUTING_PREDECESSOR,
        _ => return Err("touch_carry_forward_origin".into()),
    };
    require(
        provenance.predecessor == expected_predecessor,
        "touch_carry_forward_predecessor",
    )?;
    provenance.runner_manifest.validate_record()?;
    let predecessor = candidate_record(m, &provenance.predecessor)?;
    let transition: Value = provenance.transition.read_record(8 * 1024 * 1024)?;
    let prepared_path = provenance.transition.path.parent()
        .ok_or("touch_carry_forward_transition")?.join("transition.json");
    let prepared: Value = bounded(&prepared_path)?;
    require(
        prepared["schema"] == 1
            && prepared["state"] == "prepared"
            && prepared["environment"] == c.selection.environment.id
            && prepared["before_revision"] == predecessor.selection.environment.revision
            && prepared["after_revision"] == c.selection.environment.revision
            && prepared["before_runner_sha256"] == hex(&Sha256::digest(serde_json::to_vec(&predecessor.selection.environment.runner)?))
            && prepared["after_runner_sha256"] == hex(&Sha256::digest(serde_json::to_vec(&c.selection.environment.runner)?))
            && prepared["candidate_manifest_sha256"] == provenance.runner_manifest.sha256
            && prepared["retired_class"] == c.selection.class.id
            && prepared["prefix_recreated"] == false
            && prepared["installation_changed"] == false
            && prepared["historical_results_rewritten"] == false
            && prepared["publication_carried_forward"] == false,
        "touch_carry_forward_prepared",
    )?;
    let retired: crate::publication::RevisionRef = serde_json::from_value(prepared["removed_publication"].clone())?;
    let retired_revision = m.load_revision_record(&c.selection.class.id, &retired)?;
    require(
        retired_revision.profile == predecessor.profile
            && retired_revision.registration.module == predecessor.selection.module
            && retired_revision.registration.host == predecessor.host
            && retired_revision.registration.native.sha256 == predecessor.native.artifact.sha256,
        "touch_carry_forward_retired_revision",
    )?;
    require(
        transition["schema"] == 1
            && transition["state"] == "completed"
            && transition["environment"] == c.selection.environment.id
            && transition["revision"] == c.selection.environment.revision
            && transition["runner_sha256"] == hex(&Sha256::digest(serde_json::to_vec(&c.selection.environment.runner)?))
            && transition["candidate_manifest_sha256"] == provenance.runner_manifest.sha256
            && transition["carried_predecessor"] == provenance.predecessor,
        "touch_carry_forward_transition",
    )?;
    let expected = match c.origin {
        Origin::X11TouchReleaseV1 =>
            touch_successor(&predecessor, &c.selection.environment, provenance.clone())?,
        Origin::X11TouchRoutingV2 =>
            touch_routing_successor(&predecessor, &c.selection.environment, provenance.clone())?,
        _ => return Err("touch_carry_forward_origin".into()),
    };
    require(*c == expected, "touch_carry_forward_changed")
}
pub fn verify_retained_candidate(m: &Manager, c: &Candidate) -> Result<()> {
    validate_candidate_record(m, c)?;
    configuration::verify_trial(m, c)?;
    let host = &c.selection.scanner;
    let source = c.selection.scanner_source.as_str();
    // Registration::verify below performs the full runner verification for
    // this candidate. Verify the selection artifacts here without hashing
    // the same runtime tree twice in one verification call.
    verify_selection_artifacts(m, &c.selection, host, source)?;
    if c.origin == Origin::RetainedSv1 {
        verify_legacy(m, c)?;
    } else if c.host.sha256 != host.sha256 || c.source_manifest.sha256 != source {
        build::verify_runtime(m, c)?;
    }
    c.source_manifest.verify()?;
    c.host.verify()?;
    c.native.artifact.verify()?;
    configuration::registration(c)?.verify(&m.root)
}
/// Candidate consistency and retained control evidence only. A readable
/// candidate never replaces verify_candidate/verify_retained_candidate.
pub fn validate_candidate_record(m: &Manager, c: &Candidate) -> Result<()> {
    let host = &c.selection.scanner;
    let source = c.selection.scanner_source.as_str();
    // Historical configuration is readable after a supervised dependency mutation.
    // Current preparation/publication and native admission check the exact current
    // environment separately; this record consistency check grants none of them.
    validate_selection_data_with_current(m, &c.selection, host, source, false)?;
    require(
        c.schema == 1
            && c.inspection.selection == c.selection
            && c.profile.claim == Claim::ReviewCandidate
            && c.profile.class.class_id == c.selection.class.id,
        "candidate_binding",
    )?;
    require((c.profile.capabilities.accessibility == Accessibility::DisabledForVendorProcess)
        == c.profile.limitations.contains(&Limitation::WindowsAccessibilityUnavailable),
        "candidate_accessibility_limitation_changed")?;
    if c.local_settings.is_none() && c.profile.capabilities.accessibility != Accessibility::WindowsDefault {
        // The exact retained profile owns its resolved advice. Loading,
        // publication and rollback must not consult a newer recommendation.
        require(c.origin == Origin::ManagedPreparation
            && c.profile.evidence.iter().any(|reference| reference.starts_with("evidence/")),
            "candidate_policy_requires_explicit_support")?;
    }
    configuration::verify_settings(c)?;
    if let Some(reference) = &c.launch_configuration_intent {
        launch_configuration_source(m, c, reference)?;
    }
    let expected_compatibility = Compatibility {
        graphics: c.profile.capabilities.graphics,
        disable_windows_accessibility: c.profile.capabilities.accessibility == Accessibility::DisabledForVendorProcess,
        audio_layout: c.inspection.audio_layout.clone(),
        ..Compatibility::default()
    };
    let expected_compatibility = if c.profile.capabilities.compatibility()
        != expected_compatibility
    {
        retained_revision_configuration(m, c)?
            .map(|revision| revision.registration.compatibility)
            .unwrap_or(expected_compatibility)
    } else {
        expected_compatibility
    };
    require(
        c.profile.capabilities.compatibility() == expected_compatibility,
        "candidate_policy_requires_explicit_support",
    )?;
    configuration::validate_trial_record(m, c)?;
    require(
        c.host == c.inspection.host && c.source_manifest == c.inspection.source_manifest,
        "candidate_host_changed",
    )?;
    if c.origin == Origin::RetainedSv1 {
        validate_legacy_record(m, c)?;
    } else if c.host.sha256 != host.sha256 || c.source_manifest.sha256 != source {
        let runtime = build::existing_runtime_record(m, &c.recipe_sha256)?;
        require(c.host == runtime.host && c.source_manifest == runtime.source_manifest,
            "candidate_runtime_changed")?;
    }
    c.source_manifest.validate_record()?;
    c.host.validate_record()?;
    c.native.matches_record(&c.profile)?;
    c.native.artifact.validate_record()?;
    require(
        inspection_record_with_stamp(
            c.selection.clone(),
            c.inspection.report.clone(),
            c.inspection.origin.clone(),
            c.host.clone(),
            c.source_manifest.clone(),
            c.inspection.audio_layout.clone(),
            None,
        )? == c.inspection,
        "candidate_inspection_changed",
    )?;
    let census = c.census_record()?;
    let reg = configuration::registration_record_for(c, &c.profile, &census,
        SelectionPurpose::Qualification)?;
    reg.metadata.verify()?;
    require(reg.host.path.starts_with(m.root.join("software"))
        && reg.environment.root.canonicalize()? == reg.environment.root,
        "candidate_retained_location")?;
    reg.verify_descriptor()
}
pub fn record_candidate(m: &Manager, c: &Candidate) -> Result<String> {
    record_candidate_with_predecessor(m, c, None)
}
pub fn record_candidate_with_predecessor(
    m: &Manager,
    c: &Candidate,
    predecessor: Option<&str>,
) -> Result<String> {
    let id = c.id()?;
    let d = object(m, "candidates", &id)?;
    // The candidate is the visible commit. Retain its exact ancestry first;
    // an interrupted lineage write may leave no committed candidate.
    retain_lineage(m, c, &id, predecessor)?;
    immutable(&d.join("candidate.json"), c)?;
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
    let (accessibility, reference) = accessibility::selected(&s)?;
    let mut evidence = vec!["docs/MF3.md".into()];
    if let Some(reference) = reference { evidence.push(reference); }
    prepared_with_advice(s, i, native, host, manifest, recipe, (accessibility, evidence))
}

// Build one profile from advice already resolved by preparation, or retained
// in the exact configuration when binding metadata for a new review generation.
fn prepared_with_advice(
    s: Selection,
    i: Inspection,
    native: NativeArtifact,
    host: Artifact,
    manifest: Artifact,
    recipe: String,
    advice: (Accessibility, Vec<String>),
) -> Result<Candidate> {
    require(
        s == i.selection
            && native.module_sha256 == s.module.sha256
            && native.class.class_id == s.class.id,
        "preparation_inputs_changed",
    )?;
    let raw: Value = bounded(&i.report.path)?;
    require(raw["error"].is_null(), "inspection_failed")?;
    let (accessibility, evidence) = advice;
    let mut limitations=vec![Limitation::DirectEditorUnderQualification,
        Limitation::Unqualified256,Limitation::DetachedFocusRefusal];
    if accessibility == Accessibility::DisabledForVendorProcess {
        limitations.push(Limitation::WindowsAccessibilityUnavailable);
    }
    let identity = if accessibility == Accessibility::WindowsDefault {
        key(&(&s, &i, &native, &host, &manifest, &recipe))?
    } else {
        key(&(&s, &i, &native, &host, &manifest, &recipe, &accessibility))?
    };
    let profile = Profile {
        schema: 1,
        id: format!(
            "managed.{}",
            identity
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
            graphics: None,
            accessibility,
            editor: Editor::DetachedDirectVendorLifecycle,
            state: State::ConcurrentReadOnlyCaptureV12,
            precision: Precision::Float32Only,
            performance: PerformancePolicy::Frames512Recommended256Unqualified,
            vendor_retirement: None,
            editor_lifetime: None,
            event_output: None,
            audio_layout: i.audio_layout.clone(),
        },
        limitations,
        evidence,
    };
    profile.validate()?;
    Ok(Candidate {
        local_settings: None,
        settings_trial: None,
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
        launch_configuration_intent: None,
        touch_carry_forward: None,
    })
}
fn evidence_dir(m: &Manager, id: &str) -> Result<PathBuf> {
    object(m, "observations", id)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GuidedObservationBatch {
    schema: u32,
    candidate: String,
    operation: String,
    observations: Vec<Observation>,
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
    for p in list(&object(m, "observation-batches", &c.id()?)?)? {
        let batch: GuidedObservationBatch = bounded(&p)?;
        require(batch.schema == 1 && batch.candidate == c.id()?
            && valid_hex(&batch.operation, 32) && batch.observations.len() <= AREAS.len()
            && p.file_name().and_then(|name| name.to_str())
                == Some(format!("{}.json", batch.operation).as_str()),
            "guided_observation_batch_binding")?;
        for row in batch.observations {
            require(row.candidate == batch.candidate && row.operation == batch.operation
                && row.witness == Witness::OperatorObservation,
                "guided_observation_binding")?;
            out.push(row);
        }
    }
    out.sort_by_key(|o| (o.ordinal, o.operation.clone()));
    Ok(out)
}
pub fn record_guided_observations(
    m: &Manager,
    c: &Candidate,
    operation: &str,
    entries: &[(Area, TestStatus)],
    note: &str,
) -> Result<()> {
    require(valid_hex(operation, 32) && !entries.is_empty() && entries.len() <= AREAS.len(),
        "guided_observation_bound")?;
    detail(note)?;
    let mut areas = std::collections::BTreeSet::new();
    for (area, _) in entries {
        require(areas.insert(format!("{area:?}")), "guided_observation_duplicate_area")?;
    }
    let _owner = m.lock("preparation-evidence.lock")?;
    let path = object(m, "observation-batches", &c.id()?)?.join(format!("{operation}.json"));
    if path.exists() {
        let prior: GuidedObservationBatch = bounded(&path)?;
        require(prior.schema == 1 && prior.candidate == c.id()? && prior.operation == operation
            && prior.observations.len() == entries.len()
            && prior.observations.iter().zip(entries).all(|(row, (area, status))|
                row.area == *area && row.status == *status && row.detail == note),
            "guided_observation_replay_changed")?;
        return Ok(());
    }
    let first = observations(m, c)?.iter().map(|row| row.ordinal).max().unwrap_or(0)
        .checked_add(1).ok_or("observation_ordinal")?;
    let batch = GuidedObservationBatch {
        schema: 1, candidate: c.id()?, operation: operation.into(),
        observations: entries.iter().enumerate().map(|(index, (area, status))| Ok(Observation {
            schema: 1, candidate: c.id()?, operation: operation.into(), area: *area,
            status: *status, witness: Witness::OperatorObservation, detail: note.into(),
            recorded_at: crate::observation::now()?,
            ordinal: first.checked_add(index as u64).ok_or("observation_ordinal")?,
        })).collect::<Result<Vec<_>>>()?,
    };
    immutable(&path, &batch)?;
    changed(m)
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
    if let Some(prior) = decisions(m, c)?.into_iter().find(|decision| decision.operation == operation) {
        require(prior.choice == choice && prior.rationale == rationale
            && prior.evidence_sha256 == key(&observations(m, c)?)?,
            "review_replay_changed")?;
        return Ok(prior);
    }
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
    publication_state_with(m, c, Manager::load_revision)
}
pub fn publication_state_record(m: &Manager, c: &Candidate) -> Result<String> {
    publication_state_with(m, c, Manager::load_revision_record)
}
/// Bounded control-record proof for a selected candidate whose discovery row
/// is historical.  This does not grant preparation or execution authority; it
/// only prevents the operator projection from hiding an exact publication
/// after a coordinated package refresh.
pub fn selected_publication_candidate_record(m: &Manager, c: &Candidate) -> Result<bool> {
    if bounded::<Environment>(&c.selection.environment.root.join("environment.json"))?
        != c.selection.environment { return Ok(false); }
    if !matches!(publication_state_record(m, c)?.as_str(), "ordinary" | "experimental") {
        return Ok(false);
    }
    let db = m.registry()?;
    let entry = db.classes.get(&c.selection.class.id)
        .ok_or("publication_revision_missing")?;
    let reference = entry.managed_revision.as_ref()
        .ok_or("publication_revision_missing")?;
    let revision = m.load_revision_record(&c.selection.class.id, reference)?;
    require(entry.registration == revision.registration,
        "candidate_registration_changed")?;
    m.verify_completed_publication(&revision, reference)?;
    m.performance(&c.selection.class.id)?;
    let records = RecordReadback::capture(m)?;
    let Some(selected) = records.publication_candidate_record(
        m, &revision.profile, &revision.registration)? else {
        return Ok(false);
    };
    Ok(selected.id()? == c.id()?)
}
fn publication_state_with(m: &Manager, c: &Candidate,
    load: fn(&Manager, &str, &RevisionRef) -> Result<Revision>) -> Result<String> {
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
    let r = load(m,
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
    if bounded::<Environment>(&r.registration.environment.root.join("environment.json"))?
        != r.registration.environment {
        return Ok("environment_changed".into());
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
        policy: r.policy.clone(),
    })
}

fn for_profile(m: &Manager, p: &Profile) -> Result<Option<Candidate>> {
    let records = RecordReadback::capture(m)?;
    Ok(records.for_profile(m, p)?.cloned())
}
/// One census of bounded immutable metadata for a control projection.
/// It has no execution-verification result and never survives that projection.
pub struct RecordReadback {
    pub candidates: Vec<Candidate>,
    watched: std::sync::Mutex<std::collections::BTreeSet<PathBuf>>,
}
impl RecordReadback {
    pub fn capture(m: &Manager) -> Result<Self> {
        let candidates = retained_candidates(m)?;
        let mut watched = std::collections::BTreeSet::new();
        watched.insert(root(m).join("candidates"));
        Ok(Self { candidates, watched:std::sync::Mutex::new(watched) })
    }
    fn for_profile<'a>(&'a self, m: &Manager, p: &Profile) -> Result<Option<&'a Candidate>> {
        let mut found = None;
        for c in &self.candidates {
            if c.selection.class.id != p.class.class_id || c.selection.module.sha256 != p.module_sha256 {
                continue;
            }
            if c.profile == *p || (p.claim == Claim::VerifiedExactFixture && accepted_profile(m, c, p)?) {
                require(found.is_none(), "candidate_identity_ambiguous")?;
                found = Some(c);
            }
        }
        Ok(found)
    }
    /// Exact control-record reconstruction for status. It grants no execution
    /// or mutation authority.
    pub fn publication_candidate_record<'a>(
        &'a self,
        m: &Manager,
        p: &Profile,
        r: &Registration,
    ) -> Result<Option<&'a Candidate>> {
        let Some(c) = self.for_profile(m, p)? else {
            return Ok(None);
        };
        validate_candidate_record(m, c)?;
        let mut expected = configuration::registration_record_for(
            c,
            p,
            &c.census_record()?,
            if p.claim == Claim::ReviewCandidate {
                SelectionPurpose::Qualification
            } else {
                SelectionPurpose::Activation
            },
        )?;
        expected.relocate_native(r.native.path.clone());
        require(expected == *r, "candidate_registration_changed")?;
        Ok(Some(c))
    }
    pub fn watched_paths(&self) -> Result<Vec<PathBuf>> {
        Ok(self.watched.lock().map_err(|_| "preparation_record_watch_poisoned")?.iter().cloned().collect())
    }
    pub(super) fn watch_candidate(&self, m: &Manager, c: &Candidate) -> Result<()> {
        let mut paths = self.watched.lock().map_err(|_| "preparation_record_watch_poisoned")?;
        let id = c.id()?;
        paths.insert(object(m, "candidates", &id)?.join("candidate.json"));
        paths.insert(object(m, "lineage", &id)?.join("record.json"));
        paths.insert(object(m, "inspections", &c.selection.id()?)?);
        for artifact in [&c.selection.module, &c.selection.scanner, &c.selection.factory_report,
            &c.inspection.report, &c.host, &c.source_manifest, &c.native.artifact]
            .into_iter().chain(c.native.descriptor.iter()) {
            paths.insert(artifact.path.clone());
        }
        if valid_hex(&c.recipe_sha256, 64) {
            paths.insert(m.root.join("software/preparation-kits").join(&c.recipe_sha256).join("runtime.json"));
        }
        if c.origin == Origin::RetainedSv1 {
            paths.extend(history::retained_history_paths(m, c)?.into_iter().map(|(_, path)| path));
            paths.insert(c.selection.environment.root.join("environment.json"));
            paths.insert(m.root.join("onboarding").join(&c.selection.environment.id).join("record.json"));
            paths.insert(m.root.join("inventory").join(format!("{}.json", c.selection.environment.id)));
        }
        require(paths.len() <= 4096, "preparation_record_watch_bound")
    }
    pub fn incomplete_history(&self, m: &Manager, s: &Selection) -> Result<Vec<RetainedHistoryRecovery>> {
        let mut incomplete = Vec::new();
        for c in self.candidates.iter().filter(|c| same_product(&c.selection, s)) {
            self.watch_candidate(m, c)?;
            if let Some(recovery) = retained_history_recovery(m, c)? { incomplete.push(recovery); }
        }
        Ok(incomplete)
    }
    /// Missing modern lineage is an authority gap, not permission to infer a
    /// predecessor or reuse the legacy migration owner.
    pub fn missing_lineage(&self, m: &Manager, s: &Selection) -> Result<Vec<String>> {
        let mut missing = Vec::new();
        for c in self.candidates.iter().filter(|c| same_product(&c.selection, s)
            && c.origin != Origin::RetainedSv1) {
            self.watch_candidate(m, c)?;
            let path = object(m, "lineage", &c.id()?)?.join("record.json");
            match fs::symlink_metadata(&path) {
                Ok(_) => { history::lineage_record(m, c)?; }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => missing.push(c.id()?),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(missing)
    }
    pub fn catalogue_free_registry(&self, m: &Manager, registry: &Registry) -> Result<bool> {
        for (class, entry) in &registry.classes {
            let Some(reference) = &entry.managed_revision else { return Ok(false); };
            let r = m.load_revision_record(class, reference)?;
            let Some(c) = self.for_profile(m, &r.profile)? else { return Ok(false); };
            self.watch_candidate(m, c)?;
            validate_candidate_record(m, c)?;
            let mut expected = configuration::registration_record_for(c, &r.profile, &c.census_record()?,
                if r.profile.claim == Claim::ReviewCandidate { SelectionPurpose::Qualification }
                else { SelectionPurpose::Activation })?;
            expected.relocate_native(r.registration.native.path.clone());
            require(expected == r.registration && r.registration == entry.registration,
                "candidate_catalogue_free_identity")?;
            require(r.external_ids == external_ids(class)?
                && ((r.profile.claim == Claim::ReviewCandidate
                    && r.qualification == Some(Qualification::ManagedExperimental))
                    || (r.profile.claim == Claim::VerifiedExactFixture && r.qualification.is_none())),
                "candidate_authority_kind")?;
        }
        Ok(true)
    }
}

/// Read-only package status classification from retained control records.
/// Execution and mutation owners must still perform full artifact verification.
pub fn registry_requires_loaded_engine_refresh_record(m: &Manager) -> Result<bool> {
    registry_requires_loaded_engine_refresh_record_with_registry(m, || m.lock("registry.lock"))
}
#[doc(hidden)]
pub fn registry_requires_loaded_engine_refresh_record_with_registry(m: &Manager,
    acquire_registry: impl FnOnce() -> Result<Lock>) -> Result<bool> {
    let records = RecordReadback::capture(m)?;
    let snapshot = m.package_publication_record_snapshot_locked(&acquire_registry()?)?;
    for state in snapshot {
        if state.entry.publication != Publication::Published {
            continue;
        }
        let Some(reference) = &state.entry.managed_revision else {
            return Ok(true);
        };
        let revision = m.load_revision_record(&state.class_id, reference)?;
        if crate::environment_revision::current_environment(m, &revision.registration.environment)?
            != revision.registration.environment { continue; }
        let Some(candidate) = records.publication_candidate_record(
            m,
            &revision.profile,
            &revision.registration,
        )? else {
            return Ok(true);
        };
        if !build::supports_loaded_engine_admission_record(m, candidate)? {
            return Ok(true);
        }
    }
    Ok(false)
}
/// Mutation classification preserves full candidate/runtime byte verification
/// while sharing the same explicit retained admission provenance as status.
pub fn registry_requires_loaded_engine_refresh(m: &Manager) -> Result<bool> {
    registry_requires_loaded_engine_refresh_with_registry(m, || m.lock("registry.lock"))
}
#[doc(hidden)]
pub fn registry_requires_loaded_engine_refresh_with_registry(m: &Manager,
    acquire_registry: impl FnOnce() -> Result<Lock>) -> Result<bool> {
    let snapshot = m.package_publication_snapshot_locked(&acquire_registry()?)?;
    for state in snapshot {
        if state.entry.publication != Publication::Published {
            continue;
        }
        let Some(reference) = &state.entry.managed_revision else {
            return Ok(true);
        };
        let revision = m.load_revision(&state.class_id, reference)?;
        if crate::environment_revision::current_environment(m, &revision.registration.environment)?
            != revision.registration.environment { continue; }
        let modern = publication_candidate(m, &revision.profile, &revision.registration)
            .and_then(|candidate| build::supports_loaded_engine_admission(m, &candidate))
            .unwrap_or(false);
        if !modern {
            return Ok(true);
        }
    }
    Ok(false)
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
    publication_candidate(m, p, r).map(|_| ())
}
#[doc(hidden)]
pub fn publication_candidate(m: &Manager, p: &Profile, r: &Registration) -> Result<Candidate> {
    let c = for_profile(m, p)?.ok_or("candidate_preparation_required")?;
    verify_publication_candidate(m, p, r, c)
}
fn verify_publication_candidate(
    m: &Manager,
    p: &Profile,
    r: &Registration,
    c: Candidate,
) -> Result<Candidate> {
    verify_retained_candidate(m, &c)?;
    let mut expected = configuration::registration_for(
        &c,
        p,
        &c.census()?,
        if p.claim == Claim::ReviewCandidate {
            SelectionPurpose::Qualification
        } else {
            SelectionPurpose::Activation
        },
    )?;
    expected.relocate_native(r.native.path.clone());
    require(expected == *r, "candidate_registration_changed")?;
    Ok(c)
}
#[derive(Debug)]
struct RefreshSource {
    selection: Selection,
    candidate: Option<Candidate>,
    retained_configuration: Option<Revision>,
    basis: String,
}
fn retained_revision_basis(predecessor: &Revision) -> Result<String> {
    let mut bytes = serde_json::to_vec(predecessor)?;
    bytes.push(b'\n');
    Ok(hex(&Sha256::digest(bytes)))
}
fn retained_revision_identity(predecessor: &Revision) -> Result<String> {
    Ok(format!(
        "retained-revision-v1:{}:{}:{}",
        predecessor.class_id,
        predecessor.id,
        retained_revision_basis(predecessor)?,
    ))
}
fn retained_revision_configuration(m: &Manager, candidate: &Candidate) -> Result<Option<Revision>> {
    if let Some(reference) = &candidate.launch_configuration_intent {
        return launch_configuration_source(m, candidate, reference).map(Some);
    }
    let Some(basis) = candidate.preparation_basis.as_deref() else {
        return Ok(None);
    };
    let path = object(m, "lineage", &candidate.id()?)?.join("record.json");
    if !path.try_exists()? {
        return Ok(None);
    }
    let lineage = history::lineage_record(m, candidate)?;
    let Some(binding) = lineage
        .preparation_identity
        .strip_prefix("retained-revision-v1:")
    else {
        return Ok(None);
    };
    let fields: Vec<_> = binding.split(':').collect();
    require(
        fields.len() == 3
            && fields[0] == candidate.selection.class.id
            && valid_hex(fields[0], 32)
            && valid_hex(fields[1], 32)
            && valid_hex(fields[2], 64)
            && fields[2] == basis,
        "bridge_refresh_predecessor_binding",
    )?;
    let reference = RevisionRef {
        id: fields[1].into(),
        sha256: fields[2].into(),
    };
    let revision = m.load_revision_record(fields[0], &reference)?;
    let profiles = retained_refresh_profiles()?;
    require(
        retained_revision_basis(&revision)? == basis
            && revision.qualification.is_none()
            && revision.profile.claim == Claim::VerifiedExactFixture
            && profiles
                .iter()
                .filter(|profile| **profile == revision.profile)
                .count()
                == 1
            && revision.profile.capabilities.compatibility()
                == revision.registration.compatibility
            && revision.census.environment.environment == candidate.selection.environment
            && revision.census.module == candidate.selection.module
            && revision.census.host == candidate.selection.scanner
            && revision.census.host_source_sha256 == candidate.selection.scanner_source
            && revision.census.report == candidate.selection.factory_report
            && revision.registration.metadata.class_id == candidate.selection.class.id
            && revision.registration.metadata.name == candidate.selection.class.name
            && revision.registration.metadata.vendor == candidate.selection.class.vendor
            && revision.registration.metadata.version == candidate.selection.class.version
            && revision.registration.metadata.subcategories
                == candidate.selection.class.subcategories
            && revision.registration.compatibility.audio_layout
                == candidate.inspection.audio_layout
            && revision.registration.compatibility
                == candidate.profile.capabilities.compatibility()
            && candidate.local_settings.is_some(),
        "bridge_refresh_predecessor_binding",
    )?;
    Ok(Some(revision))
}

/// The retained revision owns these launch policies, not support on a different
/// runner. Fresh preparation binds them to current content and a new inspection.
fn launch_configuration_source(m: &Manager, candidate: &Candidate,
    reference: &RevisionRef) -> Result<Revision> {
    let revision = m.load_revision_record(&candidate.selection.class.id, reference)?;
    let profiles = retained_refresh_profiles()?;
    require(revision.qualification.is_none()
        && revision.profile.claim == Claim::VerifiedExactFixture
        && profiles.iter().filter(|profile| **profile == revision.profile).count() == 1
        && revision.profile.capabilities.compatibility() == revision.registration.compatibility
        && (revision.profile.capabilities.vendor_retirement.is_some()
            || revision.profile.capabilities.editor_lifetime.is_some()
            || revision.profile.capabilities.event_output.is_some())
        && revision.registration.module == candidate.selection.module
        && revision.registration.metadata.class_id == candidate.selection.class.id
        && revision.registration.metadata.name == candidate.selection.class.name
        && revision.registration.metadata.vendor == candidate.selection.class.vendor
        && revision.registration.metadata.version == candidate.selection.class.version
        && revision.registration.metadata.subcategories == candidate.selection.class.subcategories
        && revision.registration.compatibility.audio_layout == candidate.inspection.audio_layout
        && revision.profile.capabilities.vendor_retirement == candidate.profile.capabilities.vendor_retirement
        && revision.profile.capabilities.editor_lifetime == candidate.profile.capabilities.editor_lifetime
        && revision.profile.capabilities.event_output == candidate.profile.capabilities.event_output
        && candidate.origin == Origin::ManagedPreparation
        && candidate.local_settings.is_some(), "candidate_launch_configuration_intent")?;
    crate::environment_revision::require_owned_revision(m,
        &revision.registration.environment, &candidate.selection.environment)?;
    Ok(revision)
}

pub fn carry_launch_configuration(m: &Manager, mut next: Candidate,
    prior: Option<&Candidate>) -> Result<Candidate> {
    let Some(prior) = prior else { return Ok(next) };
    validate_candidate_record(m, prior)?;
    let Some(source) = retained_revision_configuration(m, prior)? else { return Ok(next) };
    if source.profile.capabilities.vendor_retirement.is_none()
        && source.profile.capabilities.editor_lifetime.is_none()
        && source.profile.capabilities.event_output.is_none() { return Ok(next); }
    require(same_product(&prior.selection, &next.selection), "candidate_predecessor_selection")?;
    let reference = RevisionRef { id: source.id.clone(), sha256: retained_revision_basis(&source)? };
    if next.launch_configuration_intent.as_ref() == Some(&reference) {
        launch_configuration_source(m, &next, &reference)?;
        return Ok(next);
    }
    next.profile.capabilities.vendor_retirement = source.profile.capabilities.vendor_retirement.clone();
    next.profile.capabilities.editor_lifetime = source.profile.capabilities.editor_lifetime.clone();
    next.profile.capabilities.event_output = source.profile.capabilities.event_output.clone();
    next.launch_configuration_intent = Some(reference);
    next.profile.id = format!("managed.{}", key(&(&next.profile.id,
        "retained_launch_configuration_v1", &next.launch_configuration_intent))?);
    next.profile.validate()?;
    launch_configuration_source(m, &next, next.launch_configuration_intent.as_ref().unwrap())?;
    Ok(next)
}
fn retain_refresh_lineage(
    m: &Manager,
    candidate: &Candidate,
    predecessor: &Revision,
    prior_candidate: Option<&str>,
) -> Result<()> {
    let identity = retained_revision_identity(predecessor)?;
    retain_lineage(m, candidate, &identity, prior_candidate)?;
    let retained = history::lineage_record(m, candidate)?;
    require(
        retained.preparation_identity == identity
            && retained.predecessor.as_deref() == prior_candidate,
        "bridge_refresh_predecessor_binding",
    )
}
#[cfg(test)]
thread_local! {
    static RETAINED_REFRESH_PROFILE_FIXTURE:
        std::cell::RefCell<Option<Vec<Profile>>> = const { std::cell::RefCell::new(None) };
}
fn retained_refresh_profiles() -> Result<Vec<Profile>> {
    #[cfg(test)]
    if let Some(profiles) = RETAINED_REFRESH_PROFILE_FIXTURE.with(|slot| slot.borrow().clone()) {
        crate::profiles::validate_set(&profiles)?;
        return Ok(profiles);
    }
    crate::profiles::installed_profiles()
}
#[cfg(test)]
fn with_retained_refresh_profiles<T>(profiles: Vec<Profile>, run: impl FnOnce() -> T) -> T {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            RETAINED_REFRESH_PROFILE_FIXTURE.with(|slot| *slot.borrow_mut() = None);
        }
    }
    RETAINED_REFRESH_PROFILE_FIXTURE.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(profiles);
    });
    let reset = Reset;
    let result = run();
    drop(reset);
    result
}
fn retained_revision_selection(m: &Manager, predecessor: &Revision) -> Result<Selection> {
    require(
        predecessor.qualification.is_none()
            && predecessor.profile.claim == Claim::VerifiedExactFixture,
        "bridge_refresh_predecessor_claim",
    )?;
    let profiles: Vec<_> = retained_refresh_profiles()?
        .into_iter()
        .filter(|profile| *profile == predecessor.profile)
        .collect();
    require(profiles.len() == 1, "bridge_refresh_predecessor_profile")?;
    m.verify_retained_authority(predecessor, &profiles)?;
    crate::observation::select_for(&profiles, &predecessor.census, SelectionPurpose::Activation)?;
    predecessor.registration.verify(&m.root)?;
    predecessor.census.verify_current(
        &m.root,
        &predecessor.census.host,
        &predecessor.census.host_source_sha256,
        predecessor.census.captured_at,
    )?;
    let raw: Value = predecessor.census.report.read_record(8 * 1024 * 1024)?;
    let classes: Vec<_> = crate::inventory::classes(&raw)?
        .into_iter()
        .filter(|class| {
            class.id == predecessor.class_id
                && class.id == predecessor.registration.metadata.class_id
                && class.name == predecessor.registration.metadata.name
                && class.vendor == predecessor.registration.metadata.vendor
                && class.version == predecessor.registration.metadata.version
                && class.subcategories == predecessor.registration.metadata.subcategories
        })
        .collect();
    require(classes.len() == 1, "bridge_refresh_predecessor_class")?;
    require(
        predecessor.census.environment.environment == predecessor.registration.environment
            && predecessor.census.module == predecessor.registration.module
            && predecessor.census.host == predecessor.registration.host
            && predecessor.census.host_source_sha256 == predecessor.registration.host_source_sha256
            && predecessor.census.selected == predecessor.registration.metadata
            && predecessor.census.module_stamp
                == Some(ModuleStamp::read(&predecessor.census.module.path)?)
            && predecessor.profile.requirements.native_sha256
                == predecessor.registration.native.sha256
            && predecessor.profile.capabilities.compatibility()
                == predecessor.registration.compatibility,
        "bridge_refresh_predecessor_changed",
    )?;
    let selection = Selection {
        schema: 1,
        environment: predecessor.census.environment.environment.clone(),
        module: predecessor.census.module.clone(),
        class: classes.into_iter().next().unwrap(),
        scanner: predecessor.census.host.clone(),
        scanner_source: predecessor.census.host_source_sha256.clone(),
        factory_report: predecessor.census.report.clone(),
    };
    verify_selection_data(m, &selection, &selection.scanner, &selection.scanner_source)?;
    Ok(selection)
}
fn refresh_source(m: &Manager, predecessor: &Revision) -> Result<RefreshSource> {
    if let Some(candidate) = for_profile(m, &predecessor.profile)? {
        require(
            object(m, "lineage", &candidate.id()?)?
                .join("record.json")
                .try_exists()?,
            "candidate_lineage_absent",
        )?;
        history::lineage_record(m, &candidate)?;
        let candidate = verify_publication_candidate(
            m,
            &predecessor.profile,
            &predecessor.registration,
            candidate,
        )?;
        let retained_configuration = retained_revision_configuration(m, &candidate)?;
        let basis = if retained_configuration.is_some() {
            candidate
                .preparation_basis
                .clone()
                .ok_or("bridge_refresh_predecessor_binding")?
        } else {
            preparation_basis(m, Some(&candidate))?
        };
        return Ok(RefreshSource {
            selection: candidate.selection.clone(),
            candidate: Some(candidate),
            retained_configuration,
            basis,
        });
    }
    let selection = retained_revision_selection(m, predecessor)?;
    Ok(RefreshSource {
        selection,
        candidate: None,
        retained_configuration: Some(predecessor.clone()),
        basis: retained_revision_basis(predecessor)?,
    })
}
#[doc(hidden)]
pub fn publication_requires_refresh(
    m: &Manager, predecessor: &Revision, target_recipe: &str,
) -> Result<bool> {
    let source = refresh_source(m, predecessor)?;
    source.candidate.as_ref().map_or(Ok(true), |candidate| {
        // Admission support describes the retained bridge, not the bridge
        // supplied by this update. Both halves belong to the target kit.
        if candidate.recipe_sha256 != target_recipe { return Ok(true); }
        build::supports_loaded_engine_admission(m, candidate).map(|supported| !supported)
    })
}
fn carry_retained_revision_configuration(
    mut candidate: Candidate,
    predecessor: &Revision,
) -> Result<Candidate> {
    require(
        candidate.inspection.audio_layout == predecessor.registration.compatibility.audio_layout,
        "bridge_refresh_predecessor_configuration",
    )?;
    let accessibility = predecessor.profile.capabilities.accessibility.clone();
    let settings = crate::operator_model::LocalSettings {
        graphics: predecessor.profile.capabilities.graphics,
        accessibility: match accessibility {
            Accessibility::WindowsDefault => {
                crate::operator_model::AccessibilityChoice::WindowsDefault
            }
            Accessibility::DisabledForVendorProcess => {
                crate::operator_model::AccessibilityChoice::DisabledForHost
            }
        },
    };
    configuration::apply_resolved_settings(&mut candidate, settings, accessibility)?;
    candidate.profile.capabilities.vendor_retirement =
        predecessor.profile.capabilities.vendor_retirement.clone();
    candidate.profile.capabilities.editor_lifetime =
        predecessor.profile.capabilities.editor_lifetime.clone();
    candidate.profile.capabilities.event_output =
        predecessor.profile.capabilities.event_output.clone();
    require(
        candidate.profile.capabilities.compatibility() == predecessor.registration.compatibility,
        "bridge_refresh_predecessor_configuration",
    )?;
    candidate.profile.id = format!(
        "managed.{}",
        key(&(
            &candidate.profile.id,
            "retained_publication_configuration_v1",
            &predecessor.profile_sha256,
            &predecessor.registration.compatibility,
        ))?
    );
    candidate.profile.validate()?;
    Ok(candidate)
}
/// A package refresh starts from an exact retained publication, whose original
/// discovery row may have been superseded by a later inventory.  The retained
/// revision and candidate are the authority for its class/module/environment;
/// the new inspection and runtime still undergo complete byte verification.
/// Ordinary preparation continues to require a current inventory row through
/// `verify_candidate`.
pub(crate) fn verify_refresh_candidate(
    m: &Manager,
    candidate: &Candidate,
    predecessor: &Revision,
) -> Result<()> {
    let source = refresh_source(m, predecessor)?;
    require(
        predecessor.class_id == candidate.selection.class.id
            && predecessor.registration.metadata.class_id == candidate.selection.class.id
            && predecessor.registration.environment == candidate.selection.environment
            && predecessor.registration.module == candidate.selection.module
            && source.selection == candidate.selection,
        "bridge_refresh_predecessor_changed",
    )?;
    require(
        candidate.origin == Origin::ManagedPreparation
            && candidate.inspection.origin == Origin::ManagedPreparation,
        "bridge_refresh_candidate_origin",
    )?;
    if source.retained_configuration.is_some() {
        require(
            candidate.preparation_basis.as_deref() == Some(source.basis.as_str())
                && candidate.local_settings.is_some()
                && candidate.profile.capabilities.compatibility()
                    == predecessor.registration.compatibility,
            "bridge_refresh_predecessor_configuration",
        )?;
    }
    verify_retained_candidate(m, candidate)
}
#[doc(hidden)]
pub fn refresh_candidate(
    m: &Manager,
    predecessor: &Revision,
    runtime: build::Runtime,
    report: Artifact,
    operation: &str,
) -> Result<Candidate> {
    let source = refresh_source(m, predecessor)?;
    require(
        source.selection.environment == predecessor.registration.environment
            && source.selection.module == predecessor.registration.module,
        "bridge_refresh_predecessor_changed",
    )?;
    let inspection = inspect_record_with_layout(
        source.selection.clone(),
        report,
        Origin::ManagedPreparation,
        runtime.host.clone(),
        runtime.source_manifest.clone(),
        predecessor.registration.compatibility.audio_layout.clone(),
    )?;
    let next =
        build::construct_with_runtime(m, source.selection.clone(), inspection, runtime, operation)?;
    let next = if let Some(prior) = source.candidate.as_ref() {
        let next = bind_preparation_basis(
            configuration::carry_settings(next, Some(prior))?,
            Some(source.basis.clone()),
        )?;
        if source.retained_configuration.is_some() {
            carry_retained_revision_configuration(next, predecessor)?
        } else {
            next
        }
    } else {
        carry_retained_revision_configuration(
            bind_preparation_basis(next, Some(source.basis.clone()))?,
            predecessor,
        )?
    };
    let prior_candidate = source
        .candidate
        .as_ref()
        .map(Candidate::id)
        .transpose()?;
    if let Some(retained) = source.retained_configuration.as_ref() {
        retain_refresh_lineage(m, &next, retained, prior_candidate.as_deref())?;
    }
    verify_refresh_candidate(m, &next, predecessor)?;
    require(
        build::supports_loaded_engine_admission(m, &next)?,
        "loaded_engine_admission_contract_missing",
    )?;
    crate::operator_lock::timing::measure(
        crate::operator_lock::timing::Stage::CandidateMutation,
        || {
            if let Some(prior) = source.candidate.as_ref() {
                record_candidate_with_predecessor(m, &next, Some(&prior.id()?))
            } else {
                record_candidate(m, &next)
            }
        },
    )?;
    Ok(next)
}
fn verify_retained_revision(m: &Manager, r: &Revision) -> Result<()> {
    check_publication(m, &r.profile, &r.registration)?;
    // Publication retains the explicitly selected buffering configuration.
    // A larger snapshot needs the same exact proxy capability as selection;
    // a new manager cannot enlarge a historical native binary's envelope.
    r.performance.verify()?;
    let buffering = r.performance.added_frames != 1024
        || build::maximum_bridge_frames(m, &r.registration)? == Some(1024);
    require(
        buffering && (r.performance.delivery_mode != DeliveryMode::SameCallback
            || build::supports_audio_completion(m, &r.registration)?) && r.external_ids == external_ids(&r.class_id)?,
        "candidate_runtime_contract",
    )?;
    require(
        (r.profile.claim == Claim::ReviewCandidate
            && r.qualification == Some(Qualification::ManagedExperimental))
            || (r.profile.claim == Claim::VerifiedExactFixture && r.qualification.is_none()),
        "candidate_authority_kind",
    )?;
    Ok(())
}
fn catalogue_free_revision<'a>(m: &Manager, class: &str, entry: &'a Entry)
    -> Result<Option<(Revision, &'a RevisionRef)>> {
    let Some(reference) = &entry.managed_revision else { return Ok(None); };
    let revision = m.load_revision(class, reference)?;
    if !owns_profile(m, &revision.profile)? { return Ok(None); }
    verify_retained_revision(m, &revision)?;
    require(revision.registration == entry.registration,
        "candidate_catalogue_free_identity")?;
    Ok(Some((revision, reference)))
}
/// Readback retains exact managed ownership even when publication needs
/// reconciliation. It does not authorize setup, mutation or execution.
pub(crate) fn catalogue_free_registry_readback(m: &Manager, registry: &Registry) -> Result<bool> {
    RecordReadback::capture(m)?.catalogue_free_registry(m, registry)
}
/// Managed preparation owns its exact generated publication independently of
/// the static catalogue shipped for previously qualified fixtures. A completed
/// transaction and the physical selected/removed disposition remain required.
pub(crate) fn catalogue_free_registry(m: &Manager, registry: &Registry) -> Result<bool> {
    for (class, entry) in &registry.classes {
        let Some((revision, reference)) = catalogue_free_revision(m, class, entry)? else {
            return Ok(false);
        };
        require(!m.publication_pending(class)?, "candidate_catalogue_free_identity")?;
        m.verify_completed_publication(&revision, reference)?;
        let physical = physical(&m.link(class))?;
        require(match entry.publication {
            Publication::Published => physical == Some(revision.target),
            Publication::Removed => physical.is_none(),
            Publication::Pending => false,
        }, "candidate_catalogue_free_publication")?;
    }
    Ok(true)
}
pub(crate) fn retained(m: &Manager, r: &Revision) -> Result<()> {
    verify_retained_revision(m, r)?;
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
    enable_with_registry(m, c, ordinary, || m.lock("registry.lock"))
}
pub fn enable_with_registry(
    m: &Manager, c: &Candidate, ordinary: bool,
    acquire_registry: impl FnMut() -> Result<Lock>,
) -> Result<RevisionRef> {
    enable_exact(m, c, ordinary, None, None, acquire_registry)
}
pub fn replace(m: &Manager, c: &Candidate, expected: &RevisionRef) -> Result<RevisionRef> {
    replace_with_registry(m, c, expected, || m.lock("registry.lock"))
}
pub fn replace_with_registry(
    m: &Manager, c: &Candidate, expected: &RevisionRef,
    acquire_registry: impl FnMut() -> Result<Lock>,
) -> Result<RevisionRef> {
    require(
        publication_state(m, c)? == "another_configuration",
        "replacement_not_required",
    )?;
    enable_exact(m, c, false, Some(expected), None, acquire_registry)
}
#[doc(hidden)]
pub fn replace_refreshed(
    m: &Manager,
    c: &Candidate,
    predecessor: &Revision,
    expected: &RevisionRef,
) -> Result<RevisionRef> {
    require(
        publication_state(m, c)? == "another_configuration",
        "replacement_not_required",
    )?;
    enable_exact(m, c, false, Some(expected), Some(predecessor), || m.lock("registry.lock"))
}
fn enable_exact(
    m: &Manager,
    c: &Candidate,
    ordinary: bool,
    expected: Option<&RevisionRef>,
    refresh_predecessor: Option<&Revision>,
    acquire_registry: impl FnMut() -> Result<Lock>,
) -> Result<RevisionRef> {
    use crate::operator_lock::timing::{self, Stage};
    require(
        !ordinary || publication_state(m, c)? != "ordinary",
        "candidate_already_ordinary",
    )?;
    timing::measure(Stage::CandidateVerification, || match refresh_predecessor {
        Some(predecessor) => verify_refresh_candidate(m, c, predecessor),
        None => verify_candidate(m, c, &c.selection.scanner, &c.selection.scanner_source),
    })?;
    if refresh_predecessor.is_some() {
        require(
            build::supports_loaded_engine_admission(m, c)?,
            "loaded_engine_admission_contract_missing",
        )?;
    }
    if !ordinary {
        if let Some(trial) = &c.settings_trial {
            require(expected == trial.baseline.as_ref(), "settings_trial_baseline_changed")?;
        }
    }
    require(
        publication_state(m, c)? != "needs_attention"
            && (publication_state(m, c)? != "another_configuration" || expected.is_some()),
        "explicit_replacement_or_reconciliation_required",
    )?;
    // Adopting retained CLI provenance grants no publication by itself.
    timing::measure(Stage::CandidateMutation, ||
        record_candidate(m, c))?;
    let p = if ordinary {
        accepted(m, c)?
    } else {
        c.profile.clone()
    };
    let census = c.census()?;
    let r = configuration::registration_for(
        c, &p, &census,
        if ordinary {
            SelectionPurpose::Activation
        } else {
            SelectionPurpose::Qualification
        },
    )?;
    m.publish_with_expected_registry(
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
            false,
        ),
        None,
        expected,
        acquire_registry,
    )
}
pub fn disable(m: &Manager, c: &Candidate) -> Result<()> {
    let current = m.registry()?.classes.get(&c.selection.class.id)
        .and_then(|entry| entry.managed_revision.clone())
        .ok_or("candidate_not_published")?;
    disable_exact(m, c, &current)
}
/// A guided negative result may only retire the publication that was offered
/// when the result was captured, including through the final registry lock.
pub fn disable_exact(m: &Manager, c: &Candidate, expected: &RevisionRef) -> Result<()> {
    let db = m.registry()?;
    let e = db
        .classes
        .get(&c.selection.class.id)
        .ok_or("candidate_not_published")?;
    require(e.managed_revision.as_ref() == Some(expected),
        "guided_test_publication_changed")?;
    let r = m.load_revision(
        &c.selection.class.id,
        expected,
    )?;
    require(
        r.profile == c.profile
            && matches!(
                r.qualification,
                Some(Qualification::ManagedExperimental | Qualification::Sv1Instrument)
            ),
        "not_selected_experimental_publication",
    )?;
    if let Some(trial) = &c.settings_trial {
        configuration::verify_trial(m, c)?;
        if let Some(baseline) = &trial.baseline {
            require(r.parent.as_ref() == Some(baseline), "settings_trial_parent_changed")?;
            m.rollback_exact(&r.class_id, &baseline.id, expected)?;
        } else {
            m.unpublish_exact(&r.class_id, expected)?;
        }
        return Ok(());
    }
    let mut parent = r.parent.clone();
    let mut visited = std::collections::BTreeSet::new();
    while let Some(reference) = parent {
        require(
            visited.len() < 64 && visited.insert(reference.id.clone()),
            "candidate_ancestry_cycle",
        )?;
        let prior = m.load_revision(&r.class_id, &reference)?;
        if prior.qualification.is_none() && prior.profile.claim == Claim::VerifiedExactFixture {
            m.rollback_exact(
                &r.class_id,
                &reference.id,
                expected,
            )?;
            return Ok(());
        }
        parent = prior.parent;
    }
    m.unpublish_exact(
        &r.class_id,
        expected,
    )
}

fn changed(m: &Manager) -> Result<()> {
    private_dir(&root(m))?;
    atomic_json(&root(m).join("revision.json"), &random_id()?)
}
#[cfg(test)]
pub(crate) mod tests;

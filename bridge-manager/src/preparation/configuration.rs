//! Settings live in the existing candidate/profile and publication history.
//! A launch-only trial never revises a runner, prefix or native descriptor.
use super::*;
use crate::{graphics::assessment, operator_model::GraphicsBackend};

pub fn registration(c: &Candidate) -> Result<Registration> {
    crate::observation::derive_for(&c.profile, &c.census()?, &c.native,
        SelectionPurpose::Qualification)
}

fn successor(base: &Candidate, backend: Option<GraphicsBackend>, baseline: Option<&RevisionRef>) -> Result<Candidate> {
    require(base.origin == Origin::ManagedPreparation, "settings_require_managed_preparation")?;
    require(base.profile.capabilities.graphics != backend, "settings_trial_unchanged")?;
    let mut next = base.clone();
    next.settings_trial = Some(SettingsTrial { predecessor: base.id()?, baseline: baseline.cloned() });
    next.profile.capabilities.graphics = backend;
    next.profile.id = format!("managed.{}", key(&(&next.settings_trial, backend))?);
    next.profile.revision = 1;
    next.profile.claim = Claim::ReviewCandidate;
    next.profile.validate()?;
    Ok(next)
}

/// Caller owns registry admission. Preparing changes no publication; the
/// existing explicit test-publication action will recheck the same baseline.
pub fn prepare(m: &Manager, base: &Candidate, backend: Option<GraphicsBackend>,
    expected: Option<&RevisionRef>) -> Result<Candidate> {
    m.require_inactive(None)?;
    verify_candidate(m, base, &base.selection.scanner, &base.selection.scanner_source)?;
    let state = publication_state(m, base)?;
    require(matches!((state.as_str(), expected), ("ordinary" | "experimental", Some(_))
        | ("unpublished" | "removed", None)), "settings_trial_requires_current_configuration")?;
    require(m.registry()?.classes.get(&base.selection.class.id)
        .filter(|entry| entry.publication == Publication::Published)
        .and_then(|entry| entry.managed_revision.as_ref()) == expected,
        "settings_trial_baseline_changed")?;
    let next = successor(base, backend, expected)?;
    verify_candidate(m, &next, &base.selection.scanner, &base.selection.scanner_source)?;
    // Retain ancestry before exposing the candidate, as ordinary preparation
    // does. A crash must not let readback invent a predecessor-free lineage.
    retain_lineage(m, &next, &next.id()?, Some(&base.id()?))?;
    record_candidate_with_predecessor(m, &next, Some(&base.id()?))?;
    Ok(next)
}

pub(super) fn verify_trial(m: &Manager, c: &Candidate) -> Result<()> {
    let Some(trial) = &c.settings_trial else {
        return Ok(());
    };
    let base = retained_candidates(m)?.into_iter()
        .find(|old| old.id().is_ok_and(|id| id == trial.predecessor))
        .ok_or("settings_trial_predecessor_absent")?;
    require(successor(&base, c.profile.capabilities.graphics, trial.baseline.as_ref())? == *c,
        "settings_trial_configuration_changed")?;
    let Some(baseline) = &trial.baseline else { return Ok(()) };
    let prior = m.load_revision(&c.selection.class.id, baseline)?;
    require((prior.profile == base.profile || accepted_profile(m, &base, &prior.profile)?)
        && prior.registration.environment == c.selection.environment
        && prior.registration.module == c.selection.module
        && prior.registration.host == c.host,
        "settings_trial_baseline_binding")
}

/// Refreshing discovery/native data keeps explicit local launch settings. It
/// does not inherit results or a trial baseline from a different generation.
pub fn carry_settings(mut next: Candidate, prior: Option<&Candidate>) -> Result<Candidate> {
    if let Some(prior) = prior {
        if next.profile.capabilities.graphics != prior.profile.capabilities.graphics {
            next.profile.capabilities.graphics = prior.profile.capabilities.graphics;
            next.profile.id = format!("managed.{}", key(&(&next.profile.id, next.profile.capabilities.graphics))?);
            next.profile.validate()?;
        }
    }
    Ok(next)
}

pub fn context(c: &Candidate) -> Result<assessment::Context> {
    assessment::Context::for_configuration(&c.selection.environment, &c.selection.module,
        &c.selection.class.id, &c.host, &c.source_manifest.sha256,
        &c.profile.capabilities.compatibility())
}

pub fn record_assessment(m: &Manager, c: &Candidate, operation: &str, result: &assessment::Assessment) -> Result<()> {
    require(valid_hex(operation, 32) && result.context == context(c)?
        && result.context_fingerprint == context(c)?.fingerprint()?,
        "candidate_graphics_assessment_binding")?;
    let value = serde_json::json!({"candidate":c.id()?,"assessment":result});
    immutable(&object(m, "graphics", &c.id()?)?.join(format!("{operation}.json")), &value)?;
    changed(m)
}

/// Retained observations describe that run, not the current driver/device.
/// Ordinary projection never launches a probe or infers accelerated rendering.
pub fn view(m: &Manager, c: &Candidate) -> Result<Value> {
    let expected = context(c)?;
    let mut reports = Vec::new();
    for path in list(&root(m).join("graphics").join(c.id()?))? {
        if path.extension().and_then(|s| s.to_str()) != Some("json") { continue; }
        require(path.file_stem().and_then(|s| s.to_str()).is_some_and(|s| valid_hex(s, 32)),
            "candidate_graphics_operation_identity")?;
        let row: Value = bounded(&path)?;
        require(row["candidate"] == c.id()?, "candidate_graphics_identity")?;
        let observed: assessment::Context = serde_json::from_value(row["assessment"]["context"].clone())?;
        if observed == expected && row["assessment"]["context_fingerprint"] == expected.fingerprint()? {
            reports.push(row["assessment"].clone());
        }
    }
    reports.sort_by_key(|row| row["observed_at"].as_u64().unwrap_or(0));
    Ok(serde_json::json!({
        "candidate":c.id()?, "backend":c.profile.capabilities.graphics,
        "requested":expected.requested_graphics,
        "reason":if c.profile.capabilities.graphics.is_some() || c.settings_trial.is_some() { "Explicit local choice" } else { "Selected runtime defaults" },
        "scope":"This plug-in class's new Windows host processes and their children; other classes keep their own settings",
        "change":c.settings_trial, "assessment":reports.pop(),
        "observation_scope":"Recorded assessment only; current driver/device freshness and the editor's actual rendering device are not established",
        "qualification":"unqualified by graphics settings or capability probes"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preparation::tests::fixture;

    #[test]
    fn graphics_trial_reuses_engine_keeps_results_separate_and_restores_exact_experimental_baseline() {
        let (f, base) = fixture();
        record_candidate(&f.m, &base).unwrap();
        let original = enable(&f.m, &base, false).unwrap();
        let environment = fs::read(base.selection.environment.root.join("environment.json")).unwrap();
        let before = serde_json::to_value(f.m.registry().unwrap()).unwrap();
        let next = prepare(&f.m, &base, Some(GraphicsBackend::WineD3d11), Some(&original)).unwrap();
        assert_eq!(serde_json::to_value(f.m.registry().unwrap()).unwrap(), before, "preparation must not select the trial");
        assert_eq!(next.native, base.native);
        assert_eq!(next.selection, base.selection);
        assert_eq!(lineage(&f.m, &next).unwrap().predecessor, Some(base.id().unwrap()));
        assert!(observations(&f.m, &next).unwrap().is_empty());
        assert!(view(&f.m, &next).unwrap()["assessment"].is_null());
        assert_ne!(context(&base).unwrap().fingerprint().unwrap(), context(&next).unwrap().fingerprint().unwrap());
        let selected = replace(&f.m, &next, &original).unwrap();
        let revision = f.m.load_revision(&base.selection.class.id, &selected).unwrap();
        assert_eq!(revision.registration.compatibility.graphics, Some(GraphicsBackend::WineD3d11));
        assert_eq!(revision.external_ids, f.m.load_revision(&base.selection.class.id, &original).unwrap().external_ids);
        check_publication(&f.m, &next.profile, &revision.registration).unwrap();
        assert!(catalogue_free_registry(&f.m, &f.m.registry().unwrap()).unwrap());
        disable_exact(&f.m, &next, &selected).unwrap();
        let after = serde_json::to_value(f.m.registry().unwrap()).unwrap();
        assert_eq!(after["classes"], before["classes"], "restore the experimental predecessor, do not remove it");
        assert!(after["revision"].as_u64() > before["revision"].as_u64());
        assert_eq!(fs::read(base.selection.environment.root.join("environment.json")).unwrap(), environment);
    }

    #[test]
    fn graphics_trial_refuses_stale_baselines_and_changed_configuration() {
        let (f, base) = fixture();
        record_candidate(&f.m, &base).unwrap();
        let original = enable(&f.m, &base, false).unwrap();
        let mut stale = original.clone(); stale.sha256 = "ff".repeat(32);
        assert!(prepare(&f.m, &base, Some(GraphicsBackend::WineD3d11), Some(&stale)).is_err());
        let next = prepare(&f.m, &base, Some(GraphicsBackend::WineD3d11), Some(&original)).unwrap();
        let mut changed = next.clone(); changed.profile.capabilities.graphics = None;
        assert!(verify_candidate(&f.m, &changed, &base.host, &base.source_manifest.sha256).is_err());
        let selected = replace(&f.m, &next, &original).unwrap();
        let reset = prepare(&f.m, &next, None, Some(&selected)).unwrap();
        assert!(replace(&f.m, &reset, &original).is_err());
        let reset_ref = replace(&f.m, &reset, &selected).unwrap();
        assert!(f.m.load_revision(&base.selection.class.id, &reset_ref).unwrap().registration.compatibility.graphics.is_none());
        assert!(disable_exact(&f.m, &next, &selected).is_err(), "stale restore must not withdraw the new trial");
        disable_exact(&f.m, &reset, &reset_ref).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(selected));
        let refreshed = carry_settings(base.clone(), Some(&next)).unwrap();
        let refreshed = bind_preparation_basis(refreshed, Some("aa".repeat(32))).unwrap();
        assert_eq!(refreshed.profile.capabilities.graphics, next.profile.capabilities.graphics);
        assert!(refreshed.settings_trial.is_none());
        assert!(observations(&f.m, &refreshed).unwrap().is_empty());
    }

    #[test]
    fn graphics_trial_publication_interruptions_reconcile_to_complete_baseline_or_trial() {
        for boundary in BOUNDARIES {
            let (f, base) = fixture();
            record_candidate(&f.m, &base).unwrap();
            let original = enable(&f.m, &base, false).unwrap();
            let next = prepare(&f.m, &base, Some(GraphicsBackend::WineD3d11), Some(&original)).unwrap();
            assert!(f.m.publish_with_expected(&next.profile, &next.census().unwrap(),
                registration(&next).unwrap(), (&next.host, &next.source_manifest.sha256),
                (Some(Qualification::ManagedExperimental), true), Some(boundary), Some(&original)).is_err());
            f.m.reconcile().unwrap();
            let db = f.m.registry().unwrap();
            let entry = &db.classes[&base.selection.class.id];
            assert_eq!(entry.publication, Publication::Published);
            let reference = entry.managed_revision.as_ref().unwrap();
            let revision = f.m.load_revision(&base.selection.class.id, reference).unwrap();
            assert!(revision.profile == base.profile || revision.profile == next.profile);
            check_publication(&f.m, &revision.profile, &entry.registration).unwrap();
            if revision.profile == next.profile {
                disable_exact(&f.m, &next, reference).unwrap();
                assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original));
            }
        }
    }

    #[test]
    fn graphics_can_be_tried_before_first_publication_without_a_support_claim() {
        let (f, base) = fixture();
        record_candidate(&f.m, &base).unwrap();
        let next = prepare(&f.m, &base, Some(GraphicsBackend::WineD3d11), None).unwrap();
        assert!(f.m.registry().unwrap().classes.is_empty());
        assert!(next.settings_trial.as_ref().unwrap().baseline.is_none());
        let selected = enable(&f.m, &next, false).unwrap();
        disable_exact(&f.m, &next, &selected).unwrap();
        assert_eq!(publication_state(&f.m, &next).unwrap(), "removed");
        assert!(next.selection.module.verify().is_ok());
        assert_eq!(next.profile.claim, Claim::ReviewCandidate);
    }
}

//! Settings live in the existing candidate/profile and publication history.
//! A launch-only trial never revises a runner, prefix or native descriptor.
use super::*;
use crate::{graphics::assessment, operator_model::{GraphicsBackend, LocalSettings, AccessibilityChoice}};

pub fn registration(c: &Candidate) -> Result<Registration> {
    registration_for(c, &c.profile, &c.census()?, SelectionPurpose::Qualification)
}

fn legacy_successor(base: &Candidate, backend: Option<GraphicsBackend>, baseline: Option<&RevisionRef>) -> Result<Candidate> {
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

/// The same resolved choices feed publication, current registration validation,
/// assessment and ultimately HostBinding at launch. Support changes cannot alter
/// the effective launch options or choose a different module/runtime.
pub(super) fn registration_for(c: &Candidate, profile: &Profile, census: &crate::observation::Census,
    purpose: SelectionPurpose) -> Result<Registration> {
    verify_settings(c)?;
    require(profile.capabilities.compatibility() == c.profile.capabilities.compatibility(),
        "configuration_profile_changed")?;
    crate::observation::derive_for(profile, census, &c.native, purpose)
}

pub fn settings(c: &Candidate) -> LocalSettings {
    c.local_settings.clone().unwrap_or(LocalSettings {
        graphics: c.profile.capabilities.graphics,
        accessibility: AccessibilityChoice::ProfileDefault,
    })
}

fn selected_accessibility(c: &Candidate, settings: &LocalSettings) -> Result<Accessibility> {
    Ok(match settings.accessibility {
        AccessibilityChoice::ProfileDefault => accessibility::selected(&c.selection)?.0,
        AccessibilityChoice::WindowsDefault => Accessibility::WindowsDefault,
        AccessibilityChoice::DisabledForHost => Accessibility::DisabledForVendorProcess,
    })
}

fn accessibility_matches_choice(settings: &LocalSettings, resolved: &Accessibility) -> bool {
    match settings.accessibility {
        // The candidate's exact profile retains the advice resolved when it
        // was prepared. A later advice update cannot reinterpret that record.
        AccessibilityChoice::ProfileDefault => true,
        AccessibilityChoice::WindowsDefault => *resolved == Accessibility::WindowsDefault,
        AccessibilityChoice::DisabledForHost => *resolved == Accessibility::DisabledForVendorProcess,
    }
}

pub(super) fn verify_settings(c: &Candidate) -> Result<()> {
    if let Some(settings) = &c.local_settings {
        require(c.origin == Origin::ManagedPreparation,
            "local_settings_require_managed_preparation")?;
        require(c.profile.capabilities.graphics == settings.graphics
            && accessibility_matches_choice(settings, &c.profile.capabilities.accessibility),
            "configuration_choices_changed")?;
    }
    Ok(())
}

fn apply_settings(c: &mut Candidate, settings: LocalSettings) -> Result<()> {
    let resolved = selected_accessibility(c, &settings)?;
    apply_resolved_settings(c, settings, resolved)
}

fn apply_resolved_settings(c: &mut Candidate, settings: LocalSettings,
    resolved: Accessibility) -> Result<()> {
    require(accessibility_matches_choice(&settings, &resolved), "configuration_choices_changed")?;
    c.profile.capabilities.graphics = settings.graphics;
    c.profile.capabilities.accessibility = resolved;
    c.profile.limitations.retain(|l| *l != Limitation::WindowsAccessibilityUnavailable);
    if c.profile.capabilities.accessibility == Accessibility::DisabledForVendorProcess {
        c.profile.limitations.push(Limitation::WindowsAccessibilityUnavailable);
    }
    c.local_settings = Some(settings);
    Ok(())
}

fn successor(base: &Candidate, requested: &LocalSettings, baseline: Option<&RevisionRef>) -> Result<Candidate> {
    let resolved = selected_accessibility(base, requested)?;
    successor_with_resolved(base, requested, baseline, resolved)
}

fn successor_with_resolved(base: &Candidate, requested: &LocalSettings,
    baseline: Option<&RevisionRef>, resolved: Accessibility) -> Result<Candidate> {
    require(base.origin == Origin::ManagedPreparation, "settings_require_managed_preparation")?;
    require(settings(base) != *requested, "settings_trial_unchanged")?;
    let mut next = base.clone();
    next.settings_trial = Some(SettingsTrial { predecessor: base.id()?, baseline: baseline.cloned() });
    apply_resolved_settings(&mut next, requested.clone(), resolved)?;
    next.profile.id = format!("managed.{}", key(&(&next.settings_trial, requested, &next.profile.capabilities.accessibility))?);
    next.profile.revision = 1;
    next.profile.claim = Claim::ReviewCandidate;
    next.profile.validate()?;
    Ok(next)
}

/// Caller owns registry admission. Preparing changes no publication; the
/// existing explicit test-publication action will recheck the same baseline.
pub fn prepare(m: &Manager, base: &Candidate, backend: Option<GraphicsBackend>,
    expected: Option<&RevisionRef>) -> Result<Candidate> {
    let mut requested = settings(base);
    requested.graphics = backend;
    prepare_settings(m, base, &requested, expected)
}

/// Preparing immutable choices does not mutate the prefix, publication or an
/// active instance. Publication later rechecks and quiesces the affected class.
pub fn prepare_settings(m: &Manager, base: &Candidate, requested: &LocalSettings,
    expected: Option<&RevisionRef>) -> Result<Candidate> {
    verify_candidate(m, base, &base.selection.scanner, &base.selection.scanner_source)?;
    let state = publication_state(m, base)?;
    require(matches!((state.as_str(), expected), ("ordinary" | "experimental", Some(_))
        | ("unpublished" | "removed", None)), "settings_trial_requires_current_configuration")?;
    require(m.registry()?.classes.get(&base.selection.class.id)
        .filter(|entry| entry.publication == Publication::Published)
        .and_then(|entry| entry.managed_revision.as_ref()) == expected,
        "settings_trial_baseline_changed")?;
    let next = successor(base, requested, expected)?;
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
    let expected = if let Some(settings) = &c.local_settings {
        successor_with_resolved(&base, settings, trial.baseline.as_ref(),
            c.profile.capabilities.accessibility.clone())?
    } else {
        legacy_successor(&base, c.profile.capabilities.graphics, trial.baseline.as_ref())?
    };
    require(expected == *c, "settings_trial_configuration_changed")?;
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
        if let Some(settings) = &prior.local_settings {
            apply_settings(&mut next, settings.clone())?;
            next.profile.id = format!("managed.{}", key(&(&next.profile.id, settings))?);
            next.profile.validate()?;
            return Ok(next);
        }
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
        &registration(c)?.compatibility)
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
    let registration = registration(c)?;
    let selected = settings(c);
    let explicit_graphics = c.local_settings.is_some() || c.settings_trial.is_some();
    let accessibility_source = match selected.accessibility {
        AccessibilityChoice::ProfileDefault => "Applicable profile advice or Windows defaults",
        _ => "Explicit local choice",
    };
    let buffering = m.performance(&c.selection.class.id)?;
    Ok(serde_json::json!({
        "schema":1,
        "launch":{
            "module":registration.module.sha256,
            "class_id":registration.metadata.class_id,
            "runtime":format!("{} · {}",registration.environment.runner.id,registration.environment.runner.version),
            "environment":registration.environment.id,
            "environment_revision":registration.environment.revision,
            "host":registration.host.sha256,
            "native":registration.native.sha256,
            "descriptor":c.native.descriptor_sha256,
            "display":"X11/XWayland (selected Windows host route)"
        },
        "settings":selected,
        "choices":[
            {"name":"Graphics libraries", "value":expected.requested_graphics,
             "source":if explicit_graphics { "Explicit local choice" } else { "Selected runtime defaults" },
             "scope":"This plug-in class's new Windows host processes and their children",
             "change":"Prepare now; stop affected instances before applying. Prefix and system driver remain unchanged."},
            {"name":"Windows accessibility", "value":if registration.compatibility.disable_windows_accessibility {
                "Disabled for this plug-in host" } else { "Windows defaults" },
             "source":accessibility_source,
             "scope":"This plug-in class's new Windows host processes and their children",
             "change":"Disabling removes Windows accessibility support for these processes. Linux/Deck input settings are not changed."}
        ],
        "buffering":{"added_frames":buffering.added_frames,
            "source":"Current explicit class preference, or 512-frame default. Independent of settings restoration."},
        "candidate":c.id()?, "backend":c.profile.capabilities.graphics,
        "requested":expected.requested_graphics,
        "reason":if c.profile.capabilities.graphics.is_some() || c.settings_trial.is_some() { "Explicit local choice" } else { "Selected runtime defaults" },
        "scope":"This plug-in class's new Windows host processes and their children; other classes keep their own settings",
        "change":c.settings_trial, "assessment":reports.pop(),
        "observation_scope":"Recorded assessment only; current driver/device freshness and the editor's actual rendering device are not established",
        "qualification":"Not yet qualified. Configuration choices and capability probes do not establish musical use."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preparation::tests::fixture;

    #[test]
    fn changed_profile_advice_does_not_reinterpret_retained_settings_or_restoration() {
        let (f, base) = fixture();
        record_candidate(&f.m, &base).unwrap();
        let original = enable(&f.m, &base, false).unwrap();
        let requested = LocalSettings {graphics:Some(GraphicsBackend::WineD3d11),
            accessibility:AccessibilityChoice::ProfileDefault};
        // Model a retained trial prepared when the applicable advice disabled
        // accessibility. Today's advice for this exact fixture is WindowsDefault.
        let retained = successor_with_resolved(&base, &requested, Some(&original),
            Accessibility::DisabledForVendorProcess).unwrap();
        assert_eq!(selected_accessibility(&retained, &requested).unwrap(), Accessibility::WindowsDefault);
        let saved = serde_json::to_vec(&retained).unwrap();
        let saved_id = retained.id().unwrap();
        record_candidate_with_predecessor(&f.m, &retained, Some(&base.id().unwrap())).unwrap();
        verify_candidate(&f.m, &retained, &base.host, &base.source_manifest.sha256).unwrap();
        let selected = replace(&f.m, &retained, &original).unwrap();
        let revision = f.m.load_revision(&base.selection.class.id, &selected).unwrap();
        assert!(revision.registration.compatibility.disable_windows_accessibility);
        check_publication(&f.m, &revision.profile, &revision.registration).unwrap();

        // Refresh and a newly requested trial resolve today's advice. Restoring
        // the retained predecessor must still recover its old effective choice.
        let refreshed = carry_settings(base.clone(), Some(&retained)).unwrap();
        assert_eq!(refreshed.local_settings.as_ref(), Some(&requested));
        assert!(!registration(&refreshed).unwrap().compatibility.disable_windows_accessibility);
        let next_requested = LocalSettings {graphics:None,accessibility:AccessibilityChoice::ProfileDefault};
        let next = prepare_settings(&f.m, &retained, &next_requested, Some(&selected)).unwrap();
        let next_selected = replace(&f.m, &next, &selected).unwrap();
        assert!(!f.m.registry().unwrap().classes[&base.selection.class.id]
            .registration.compatibility.disable_windows_accessibility);
        disable_exact(&f.m, &next, &next_selected).unwrap();
        let restored = f.m.registry().unwrap().classes[&base.selection.class.id].clone();
        assert_eq!(restored.managed_revision, Some(selected.clone()));
        assert!(restored.registration.compatibility.disable_windows_accessibility);
        check_publication(&f.m, &revision.profile, &restored.registration).unwrap();
        let loaded = candidate(&f.m, &saved_id, &base.host, &base.source_manifest.sha256).unwrap();
        assert_eq!(loaded.id().unwrap(), saved_id);
        assert_eq!(serde_json::to_vec(&loaded).unwrap(), saved);

        let mut mismatch = retained.clone();
        mismatch.local_settings.as_mut().unwrap().accessibility = AccessibilityChoice::WindowsDefault;
        assert!(registration(&mismatch).is_err(), "an explicit override must match its resolved value");
        let mut changed = retained.clone();
        changed.profile.evidence.push("evidence/unrelated.json".into());
        assert!(verify_trial(&f.m, &changed).is_err(), "other retained trial fields remain exact");
        disable_exact(&f.m, &retained, &selected).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original));
    }

    #[test]
    fn applying_and_restoring_choices_quiesces_only_the_affected_class() {
        let (f, base) = fixture();
        record_candidate(&f.m, &base).unwrap();
        let original = enable(&f.m, &base, false).unwrap();
        let sibling = crate::managed_tests::lease(&f, &"FE".repeat(16), false);
        let sibling_bytes = fs::read(&sibling).unwrap();
        let active = crate::managed_tests::lease(&f, &base.selection.class.id, false);
        let requested = LocalSettings { graphics: None, accessibility: AccessibilityChoice::DisabledForHost };
        // Preparing future choices may proceed while this very class is active.
        let next = prepare_settings(&f.m, &base, &requested, Some(&original)).unwrap();
        assert!(replace(&f.m, &next, &original).is_err());
        assert!(f.m.select_delay(&base.selection.class.id, 256).is_err());
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original.clone()));
        fs::remove_file(active).unwrap(); // simulated confirmed owner retirement
        let selected = replace(&f.m, &next, &original).unwrap();
        f.m.select_delay(&base.selection.class.id, 256).unwrap();
        let active = crate::managed_tests::lease(&f, &base.selection.class.id, false);
        assert!(disable_exact(&f.m, &next, &selected).is_err());
        fs::remove_file(active).unwrap();
        disable_exact(&f.m, &next, &selected).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original));
        assert_eq!(f.m.performance(&base.selection.class.id).unwrap().added_frames, 256);
        assert_eq!(fs::read(&sibling).unwrap(), sibling_bytes);
        // Unattributable custody is never treated as an independent sibling.
        fs::write(&sibling, b"unresolved owner").unwrap();
        assert!(replace(&f.m, &next, &f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision.clone().unwrap()).is_err());
    }

    #[test]
    fn shared_choices_reach_registration_survive_refresh_and_restore_exactly() {
        let (f, base) = fixture();
        record_candidate(&f.m, &base).unwrap();
        let original = enable(&f.m, &base, false).unwrap();
        let environment = fs::read(base.selection.environment.root.join("environment.json")).unwrap();
        let requested = LocalSettings { graphics: Some(GraphicsBackend::WineD3d11),
            accessibility: AccessibilityChoice::DisabledForHost };
        let next = prepare_settings(&f.m, &base, &requested, Some(&original)).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original.clone()));
        assert_eq!(next.local_settings.as_ref(), Some(&requested));
        assert_eq!(next.native, base.native);
        assert!(observations(&f.m, &next).unwrap().is_empty());
        let selected = replace(&f.m, &next, &original).unwrap();
        let current = f.m.load_revision(&base.selection.class.id, &selected).unwrap();
        assert_eq!(current.registration.compatibility.graphics, requested.graphics);
        assert!(current.registration.compatibility.disable_windows_accessibility);
        check_publication(&f.m, &current.profile, &current.registration).unwrap();
        let shown = view(&f.m, &next).unwrap();
        assert_eq!(shown["launch"]["module"], current.registration.module.sha256);
        assert_eq!(shown["launch"]["native"], current.registration.native.sha256);
        assert_eq!(shown["settings"], serde_json::to_value(&requested).unwrap());
        assert!(shown["assessment"].is_null());
        assert_eq!(shown["buffering"]["added_frames"], 512);
        let refreshed = carry_settings(base.clone(), Some(&next)).unwrap();
        assert_eq!(refreshed.local_settings.as_ref(), Some(&requested));
        assert_eq!(registration(&refreshed).unwrap().compatibility, current.registration.compatibility);
        assert!(refreshed.settings_trial.is_none());
        assert!(observations(&f.m, &refreshed).unwrap().is_empty());
        let mut corrupted = next.clone();
        corrupted.profile.capabilities.accessibility = Accessibility::WindowsDefault;
        assert!(registration(&corrupted).is_err());
        assert!(verify_candidate(&f.m, &corrupted, &base.host, &base.source_manifest.sha256).is_err());
        disable_exact(&f.m, &next, &selected).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original));
        assert_eq!(fs::read(base.selection.environment.root.join("environment.json")).unwrap(), environment);
        assert_eq!(f.m.performance(&base.selection.class.id).unwrap().added_frames, 512);
    }

    #[test]
    fn legacy_records_keep_identity_and_can_restore_then_adopt_explicit_choices() {
        let (f, base) = fixture();
        let old_json = serde_json::to_vec(&base).unwrap();
        assert!(!String::from_utf8_lossy(&old_json).contains("local_settings"));
        let loaded: Candidate = serde_json::from_slice(&old_json).unwrap();
        assert_eq!(loaded.id().unwrap(), base.id().unwrap());
        record_candidate(&f.m, &base).unwrap();
        let original = enable(&f.m, &base, false).unwrap();
        let legacy = legacy_successor(&base, Some(GraphicsBackend::WineD3d11), Some(&original)).unwrap();
        record_candidate_with_predecessor(&f.m, &legacy, Some(&base.id().unwrap())).unwrap();
        verify_candidate(&f.m, &legacy, &base.host, &base.source_manifest.sha256).unwrap();
        let legacy_ref = replace(&f.m, &legacy, &original).unwrap();
        let requested = LocalSettings { graphics: None, accessibility: AccessibilityChoice::WindowsDefault };
        let next = prepare_settings(&f.m, &legacy, &requested, Some(&legacy_ref)).unwrap();
        let selected = replace(&f.m, &next, &legacy_ref).unwrap();
        assert!(!f.m.load_revision(&base.selection.class.id, &selected).unwrap().registration.compatibility.disable_windows_accessibility);
        disable_exact(&f.m, &next, &selected).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(legacy_ref.clone()));
        disable_exact(&f.m, &legacy, &legacy_ref).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&base.selection.class.id].managed_revision, Some(original));
        assert_eq!(serde_json::to_vec(&loaded).unwrap(), old_json);
    }

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

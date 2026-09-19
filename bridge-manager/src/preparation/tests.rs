use super::*;
use crate::test_fixture::{inspection_report, prepared_accessibility, snapshot, Fixture};
use serde_json::json;
fn fixture() -> (Fixture, Candidate) {
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
    let classes = crate::inventory::classes(&raw).unwrap();
    let scan = crate::inventory::Scan {
        schema: 1,
        id: random_id().unwrap(),
        environment: f.r.environment.clone(),
        host: f.r.host.clone(),
        host_source_sha256: f.r.host_source_sha256.clone(),
        completed_at: observation::now().unwrap(),
        modules: vec![crate::inventory::Module {
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
fn generic_selection_controller_and_exact_reuse() {
    let (f, c) = fixture();
    assert_ne!(
        c.profile.class.class_id,
        crate::managed_candidate::candidate()
            .unwrap()
            .class
            .class_id
    );
    assert_eq!(
        selections(&f.m, &c.host, &c.source_manifest.sha256)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        c.inspection.controller,
        ControllerAssociation::Separate {
            class_id: "02".repeat(16)
        }
    );
    verify_candidate(&f.m, &c, &c.host, &c.source_manifest.sha256).unwrap();
    retain_inspection(&f.m, &c.inspection).unwrap();
    let id = record_candidate(&f.m, &c).unwrap();
    assert_eq!(record_candidate(&f.m, &c).unwrap(), id);
    assert_eq!(
        candidate(&f.m, &id, &c.host, &c.source_manifest.sha256).unwrap(),
        c
    );
    assert!(f.m.registry().unwrap().classes.is_empty());
    assert_eq!(
        candidates(&f.m, &c.host, &c.source_manifest.sha256)
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn stereo_layout_is_bidirectionally_bound_to_inspection_and_candidate() {
    let (_f, c) = fixture();
    assert!(serde_json::to_value(&c.inspection).unwrap()["audio_layout"].is_null());
    assert!(inspect_record_with_layout(
        c.selection.clone(),
        c.inspection.report.clone(),
        c.inspection.origin.clone(),
        c.host.clone(),
        c.source_manifest.clone(),
        Some(AudioLayoutPolicy::StereoMainPair),
    )
    .is_err());

    let mut raw: Value = read_json(&c.inspection.report.path).unwrap();
    let records = raw["records"].as_array_mut().unwrap();
    records.push(json!({"state":"ap18_audio_layout","policy":"stereo_main_pair","input_count":1,"output_count":1,"result":0,"verified":true}));
    records.push(json!({"state":"ap8_bus","media":0,"direction":0,"index":0,"channels":2,"type":0,"flags":1,"arrangement":3,"name":"Input"}));
    records.push(json!({"state":"ap8_bus","media":0,"direction":1,"index":0,"channels":2,"type":0,"flags":1,"arrangement":3,"name":"Output"}));
    let path = c.inspection.report.path.with_file_name("stereo-inspection.json");
    atomic_json(&path, &raw).unwrap();
    let report = Artifact {
        sha256: digest(&path).unwrap(),
        path,
    };
    assert!(inspect_record_with(
        c.selection.clone(),
        report.clone(),
        c.inspection.origin.clone(),
        c.host.clone(),
        c.source_manifest.clone(),
    )
    .is_err());
    let inspection = inspect_record_with_layout(
        c.selection.clone(),
        report,
        c.inspection.origin.clone(),
        c.host.clone(),
        c.source_manifest.clone(),
        Some(AudioLayoutPolicy::StereoMainPair),
    )
    .unwrap();
    assert_ne!(inspection.id().unwrap(), c.inspection.id().unwrap());
    let candidate = prepared(
        c.selection.clone(),
        inspection,
        c.native.clone(),
        c.host.clone(),
        c.source_manifest.clone(),
        c.recipe_sha256.clone(),
    )
    .unwrap();
    assert_eq!(
        candidate.profile.capabilities.compatibility().audio_layout,
        Some(AudioLayoutPolicy::StereoMainPair)
    );
}
#[test]
fn stale_inventory_inspection_and_scanner_refused() {
    let (f, c) = fixture();
    let s = &c.selection;
    assert!(select(&f.m, &s.id().unwrap(), &s.scanner, "aa").is_err());
    fs::write(&s.factory_report.path, b"changed").unwrap();
    assert!(verify_selection(&f.m, s, &s.scanner, &s.scanner_source).is_err());
}
#[test]
fn experimental_enable_disable_preserves_installation_and_history() {
    let (f, c) = fixture();
    let env = snapshot(&f.r.environment.root);
    let inv = fs::read(
        f.m.root
            .join("inventory")
            .join(format!("{}.json", f.r.environment.id)),
    )
    .unwrap();
    let reference = enable(&f.m, &c, false).unwrap();
    let revision =
        f.m.load_revision(&c.selection.class.id, &reference)
            .unwrap();
    assert_eq!(
        revision.qualification,
        Some(Qualification::ManagedExperimental)
    );
    assert!(revision.parent.is_none());
    f.m.verify_served_host(
        &revision.registration,
        &c.host,
        &c.source_manifest.sha256,
        &crate::profiles::installed_profiles().unwrap(),
    )
    .unwrap();
    f.m.restore_editor_qualifications().unwrap();
    assert_eq!(publication_state(&f.m, &c).unwrap(), "experimental");
    assert!(enable(&f.m, &c, true).is_err());
    disable(&f.m, &c).unwrap();
    assert_eq!(publication_state(&f.m, &c).unwrap(), "removed");
    assert_eq!(snapshot(&f.r.environment.root), env);
    assert_eq!(
        fs::read(
            f.m.root
                .join("inventory")
                .join(format!("{}.json", f.r.environment.id))
        )
        .unwrap(),
        inv
    );
    enable(&f.m, &c, false).unwrap();
    assert_eq!(publication_state(&f.m, &c).unwrap(), "experimental");
}
#[test]
fn explicit_review_needs_complete_attributed_results() {
    let (f, c) = fixture();
    record_candidate(&f.m, &c).unwrap();
    assert!(review(
        &f.m,
        &c,
        &random_id().unwrap(),
        ReviewChoice::AcceptExactLocal,
        "Exact local review"
    )
    .is_err());
    for area in AREAS {
        record_observation(
            &f.m,
            &c,
            &random_id().unwrap(),
            area,
            TestStatus::Passed,
            "Operator observed this outcome on the exact candidate",
        )
        .unwrap();
    }
    let decision = review(
        &f.m,
        &c,
        &random_id().unwrap(),
        ReviewChoice::AcceptExactLocal,
        "Reviewed each retained result and limitations for this local fixture",
    )
    .unwrap();
    assert!(decision.ordinary_profile.is_some());
    assert!(f.m.registry().unwrap().classes.is_empty());
    let reference = enable(&f.m, &c, true).unwrap();
    let rev =
        f.m.load_revision(&c.selection.class.id, &reference)
            .unwrap();
    assert_eq!(rev.profile.claim, Claim::VerifiedExactFixture);
    assert!(rev.qualification.is_none());
    f.m.verify_served_host(
        &rev.registration,
        &c.host,
        &c.source_manifest.sha256,
        &crate::profiles::installed_profiles().unwrap(),
    )
    .unwrap();
    record_observation(
        &f.m,
        &c,
        &random_id().unwrap(),
        Area::Retirement,
        TestStatus::Failed,
        "Later cleanup failure observed",
    )
    .unwrap();
    assert!(accepted(&f.m, &c).is_err());
}
#[test]
fn controller_order_or_names_never_authorize_pairing() {
    let (f, c) = fixture();
    let mut raw: Value = read_json(&c.inspection.report.path).unwrap();
    raw["records"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["state"] != "ap8_controller_association");
    assert!(matches!(
        association(&raw).unwrap(),
        ControllerAssociation::Unavailable { .. }
    ));
    raw["records"].as_array_mut().unwrap().push(
        json!({"state":"ap8_controller_association","combined":false,"class_id":"ff".repeat(16)}),
    );
    assert!(association(&raw).is_err());
    drop(f);
}
#[test]
fn publication_copy_releases_registry_and_rechecks_before_commit() {
    let (f, c) = fixture();
    record_candidate(&f.m, &c).unwrap();
    std::thread::scope(|scope| {
        let (ready, seen) = std::sync::mpsc::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let m = &f.m;
        let c = &c;
        let worker = scope.spawn(move || {
            crate::publication::COPY_OBSERVER.with(|h| {
                *h.borrow_mut() = Some(Box::new(move || {
                    ready.send(()).unwrap();
                    wait.recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                }))
            });
            let result = enable(m, c, false);
            crate::publication::COPY_OBSERVER.with(|h| *h.borrow_mut() = None);
            result
        });
        seen.recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let guard =
            f.m.lock("registry.lock")
                .expect("native copy must not retain registry authority");
        drop(guard);
        release.send(()).unwrap();
        worker.join().unwrap().unwrap();
    });
}
#[test]
fn interrupted_publication_never_exposes_partial_native() {
    for boundary in [
        Boundary::CandidateCreated,
        Boundary::NativeCopied,
        Boundary::Intent,
        Boundary::PointerExchanged,
        Boundary::RegistryCommitted,
    ] {
        let (f, c) = fixture();
        record_candidate(&f.m, &c).unwrap();
        let census = c.census().unwrap();
        let r = crate::observation::derive_for(
            &c.profile,
            &census,
            &c.native,
            SelectionPurpose::Qualification,
        )
        .unwrap();
        assert!(f
            .m
            .publish_with_scope(
                &c.profile,
                &census,
                r,
                (&c.host, &c.source_manifest.sha256),
                (Some(Qualification::ManagedExperimental), true),
                Some(boundary)
            )
            .is_err());
        f.m.reconcile().unwrap();
        if let Some(e) =
            f.m.registry()
                .unwrap()
                .classes
                .get(&c.selection.class.id)
                .filter(|e| e.publication == Publication::Published)
        {
            f.m.verify_served_host(
                &e.registration,
                &c.host,
                &c.source_manifest.sha256,
                &crate::profiles::installed_profiles().unwrap(),
            )
            .unwrap();
        } else {
            assert!(physical(&f.m.link(&c.selection.class.id))
                .unwrap()
                .is_none());
        }
    }
}
#[test]
fn ordinary_parent_restored_exactly_without_sibling_or_installation_change() {
    let (f, c) = fixture();
    for area in AREAS {
        record_observation(
            &f.m,
            &c,
            &random_id().unwrap(),
            area,
            TestStatus::Passed,
            "Exact generated setup used to exercise publication law",
        )
        .unwrap();
    }
    record_candidate(&f.m, &c).unwrap();
    review(
        &f.m,
        &c,
        &random_id().unwrap(),
        ReviewChoice::AcceptExactLocal,
        "Generated acceptance fixture; no real product qualification claimed",
    )
    .unwrap();
    let ordinary = enable(&f.m, &c, true).unwrap();
    let original = f.m.registry().unwrap().classes[&c.selection.class.id].clone();
    enable(&f.m, &c, false).unwrap();
    disable(&f.m, &c).unwrap();
    let restored = f.m.registry().unwrap().classes[&c.selection.class.id].clone();
    assert_eq!(restored.managed_revision, Some(ordinary));
    assert_eq!(restored, original);
}
#[test]
fn evidence_order_is_durable_and_failure_cannot_be_checked_away() {
    let (f, c) = fixture();
    record_observation(
        &f.m,
        &c,
        &"ff".repeat(16),
        Area::Audio,
        TestStatus::Failed,
        "Observed failure",
    )
    .unwrap();
    record_observation(
        &f.m,
        &c,
        &"00".repeat(16),
        Area::Audio,
        TestStatus::Passed,
        "Later positive observation",
    )
    .unwrap();
    let rows = observations(&f.m, &c).unwrap();
    assert_eq!(rows[1].operation, "00".repeat(16));
    assert_eq!(rows[1].ordinal, 2);
    assert!(unmet(&rows, &c.profile.role)
        .iter()
        .any(|s| s.starts_with("Audio")));
}

#[test]
fn effect_is_a_distinct_selection_and_never_inherits_instrument_inspection() {
    let (f, c) = fixture();
    let path =
        f.m.root
            .join("inventory")
            .join(format!("{}.json", c.selection.environment.id));
    let mut scan: crate::inventory::Scan = read_json(&path).unwrap();
    let mut raw: Value = read_json(&c.selection.factory_report.path).unwrap();
    let mut fx = raw["records"][1]["classes"][0].clone();
    fx["raw_tuid_hex"] = json!("03".repeat(16));
    fx["subcategories_hex"] = json!(hex(b"Fx"));
    raw["records"][1]["classes"]
        .as_array_mut()
        .unwrap()
        .push(fx);
    raw["records"][1]["class_count"] = json!(3);
    atomic_json(&c.selection.factory_report.path, &raw).unwrap();
    scan.modules[0].classes = crate::inventory::classes(&raw).unwrap();
    scan.modules[0].report.sha256 = digest(&scan.modules[0].report.path).unwrap();
    atomic_json(&path, &scan).unwrap();
    let all = selections(&f.m, &c.host, &c.source_manifest.sha256).unwrap();
    assert_eq!(all.len(), 2);
    let effect = all.iter().find(|s| s.class.role == "effect").unwrap();
    assert_ne!(
        effect.id().unwrap(),
        all.iter()
            .find(|s| s.class.role == "instrument")
            .unwrap()
            .id()
            .unwrap()
    );
    assert!(inspect_record(
        effect.clone(),
        scan.modules[0].report.clone(),
        Origin::ManagedPreparation
    )
    .is_err());
    assert!(unmet(&[], &Role::Effect)
        .iter()
        .all(|s| !s.starts_with("Midi:")));
    assert!(f.m.registry().unwrap().classes.is_empty());
}

#[test]
fn failed_build_cleanup_retains_bounded_diagnostic_without_partial_candidate() {
    let (f, c) = fixture();
    let operation = "de".repeat(16);
    let dir = f.m.root.join("preparation/work").join(&operation);
    private_dir(&dir).unwrap();
    fs::write(dir.join("native.so"), b"half-built").unwrap();
    fs::write(dir.join("build.log"), b"source-owned compiler failure").unwrap();
    fs::write(
        dir.join("failure.json"),
        b"{\"error\":\"native_build_failed_1\"}",
    )
    .unwrap();
    build::cleanup_work(&f.m, &operation).unwrap();
    assert!(!dir.exists());
    assert!(f
        .m
        .root
        .join("preparation/failures")
        .join(operation)
        .join("build.log.json")
        .exists());
    assert!(candidates(&f.m, &c.host, &c.source_manifest.sha256)
        .unwrap()
        .is_empty());
    assert!(f.m.registry().unwrap().classes.is_empty());
}

#[test]
fn preparation_runtime_is_exact_immutable_software_and_bad_package_retires_stage() {
    for good in [true, false] {
        let (f, c) = fixture();
        let path = f.m.root.join("software/preparation-kit.zip");
        let script = r#"import json,zipfile,hashlib,sys
path,good=sys.argv[1:];files={'runtime/host.exe':b'host','runtime/host-source-manifest.json':b'{}'}
if good=='false':del files['runtime/host.exe']
with zipfile.ZipFile(path,'w') as z:
 z.writestr('recipe.json',json.dumps({'schema':1,'files':{k:hashlib.sha256(v).hexdigest() for k,v in files.items()}}))
 for k,v in files.items():z.writestr(k,v)
"#;
        assert!(std::process::Command::new("python3")
            .args(["-I", "-c", script])
            .arg(&path)
            .arg(if good { "true" } else { "false" })
            .status()
            .unwrap()
            .success());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        let a = c.host.clone();
        let sw = crate::catalogue::Software {
            manager: a.clone(),
            operator_frontend: None,
            installer_launch: None,
            preparation_kit: Some(Artifact {
                sha256: digest(&path).unwrap(),
                path,
            }),
            supervisor: a.clone(),
            ownership: a.clone(),
            host: a,
            source_manifest: c.source_manifest.clone(),
            source_sha256: c.source_manifest.sha256.clone(),
            native_catalogue: None,
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let result = build::stage_runtime(&f.m);
        if good {
            let r = result.unwrap();
            assert!(r.host.path.starts_with(f.m.root.join("software")));
            assert_eq!(
                file(&r.host.path).unwrap().metadata().unwrap().mode() & 0o222,
                0
            );
            assert_eq!(build::stage_runtime(&f.m).unwrap().host, r.host);
        } else {
            assert!(result.is_err());
            assert_eq!(
                fs::read_dir(f.m.root.join("software/preparation-kits"))
                    .unwrap()
                    .count(),
                0
            );
        }
    }
}

fn next_generation(m: &Manager, a: &Candidate) -> Candidate {
    let path = m.root.join("reports/new-inspection.json");
    private_dir(path.parent().unwrap()).unwrap();
    let mut raw: Value = read_json(&a.inspection.report.path).unwrap();
    raw["generation_fixture"] = json!(2);
    atomic_json(&path, &raw).unwrap();
    let i = inspect_record_with(
        a.selection.clone(),
        Artifact {
            sha256: digest(&path).unwrap(),
            path,
        },
        Origin::ManagedPreparation,
        a.host.clone(),
        a.source_manifest.clone(),
    )
    .unwrap();
    prepared(
        a.selection.clone(),
        i,
        a.native.clone(),
        a.host.clone(),
        a.source_manifest.clone(),
        "bc".repeat(32),
    )
    .unwrap()
}
#[test]
fn candidate_and_inspection_generations_coexist_and_actions_are_exact() {
    let (f, a) = fixture();
    record_candidate(&f.m, &a).unwrap();
    retain_inspection(&f.m, &a.inspection).unwrap();
    record_observation(
        &f.m,
        &a,
        &random_id().unwrap(),
        Area::ProcessingRestart,
        TestStatus::Failed,
        "Generated restart failure A",
    )
    .unwrap();
    let b = next_generation(&f.m, &a);
    retain_inspection(&f.m, &b.inspection).unwrap();
    retain_lineage(&f.m, &b, "generated-preparation-B", Some(&a.id().unwrap())).unwrap();
    record_candidate(&f.m, &b).unwrap();
    assert_eq!(record_candidate(&f.m, &b).unwrap(), b.id().unwrap());
    let history = view(&f.m, &a.selection, &a.host, &a.source_manifest.sha256).unwrap();
    assert_eq!(history.candidates.len(), 2);
    assert_eq!(history.inspections.len(), 2);
    assert_eq!(
        history.recommended_inspection,
        Some(b.inspection.id().unwrap())
    );
    assert_eq!(history.candidates[0].disposition, "superseded");
    assert_eq!(
        history.candidates[1].lineage.predecessor,
        Some(a.id().unwrap())
    );
    assert_eq!(
        candidate(&f.m, &b.id().unwrap(), &b.host, &b.source_manifest.sha256).unwrap(),
        b
    );
    assert!(observations(&f.m, &b).unwrap().is_empty());
    assert!(exact_inspection(&f.m, &a.selection, &a.inspection.id().unwrap()).is_err());
    assert_eq!(
        inspection(&f.m, &a.selection).unwrap(),
        Some(b.inspection.clone())
    );
    let published = enable(&f.m, &b, false).unwrap();
    assert_eq!(
        publication_state(&f.m, &a).unwrap(),
        "another_configuration"
    );
    assert_eq!(publication_state(&f.m, &b).unwrap(), "experimental");
    assert!(disable(&f.m, &a).is_err());
    assert_eq!(
        f.m.registry().unwrap().classes[&b.selection.class.id].managed_revision,
        Some(published)
    );
    disable(&f.m, &b).unwrap();
}
#[test]
fn exact_replacement_preserves_current_revision_and_refuses_stale_expectation() {
    let (f, a) = fixture();
    let current = enable(&f.m, &a, false).unwrap();
    let b = next_generation(&f.m, &a);
    record_candidate(&f.m, &b).unwrap();
    let v = view(&f.m, &b.selection, &b.host, &b.source_manifest.sha256).unwrap();
    assert_eq!(v.current_revision, Some(current.clone()));
    assert_eq!(v.current_profile_revision, Some(1));
    assert!(enable(&f.m, &b, false).is_err());
    let mut stale = current.clone();
    stale.id = "00".repeat(16);
    assert!(replace(&f.m, &b, &stale).is_err());
    assert_eq!(
        f.m.registry().unwrap().classes[&a.selection.class.id].managed_revision,
        Some(current.clone())
    );
    let new = replace(&f.m, &b, &current).unwrap();
    assert_ne!(new, current);
    assert_eq!(publication_state(&f.m, &b).unwrap(), "experimental");
    disable(&f.m, &b).unwrap();
}
#[test]
fn needs_attention_preserves_physical_facts_and_refuses_enable() {
    let (f, c) = fixture();
    let r = enable(&f.m, &c, false).unwrap();
    fs::remove_file(f.m.link(&c.selection.class.id)).unwrap();
    assert_eq!(publication_state(&f.m, &c).unwrap(), "needs_attention");
    let v = view(&f.m, &c.selection, &c.host, &c.source_manifest.sha256).unwrap();
    assert_eq!(v.current_revision, Some(r));
    assert_eq!(v.current_profile_revision, Some(1));
    assert_eq!(v.publication_facts["physical_present"], false);
    assert!(enable(&f.m, &c, false).is_err());
}
#[test]
fn ordinary_publication_acceptance_is_immutable_until_explicit_withdrawal() {
    let (f, c) = fixture();
    record_candidate(&f.m, &c).unwrap();
    for area in AREAS {
        record_observation(
            &f.m,
            &c,
            &random_id().unwrap(),
            area,
            TestStatus::Passed,
            "Generated product review fixture",
        )
        .unwrap();
    }
    review(
        &f.m,
        &c,
        &random_id().unwrap(),
        ReviewChoice::AcceptExactLocal,
        "Generated exact local review",
    )
    .unwrap();
    let reference = enable(&f.m, &c, true).unwrap();
    record_observation(
        &f.m,
        &c,
        &random_id().unwrap(),
        Area::ProcessingRestart,
        TestStatus::Failed,
        "Later exact failure; explicit withdrawal remains available",
    )
    .unwrap();
    assert!(accepted(&f.m, &c).is_err()); // No new acceptance/publication.
    assert_eq!(publication_state(&f.m, &c).unwrap(), "ordinary");
    let r =
        f.m.load_revision(&c.selection.class.id, &reference)
            .unwrap();
    retained(&f.m, &r).unwrap(); // Same authority used by service admission.
    let v = view(&f.m, &c.selection, &c.host, &c.source_manifest.sha256).unwrap();
    assert!(v.candidates[0].publication_acceptance_sealed);
    assert!(!v.ordinary_acceptance_current);
    assert!(v.evidence.iter().any(|o| o.status == TestStatus::Failed));
    withdraw(&f.m, &c, &reference).unwrap();
    assert_eq!(publication_state(&f.m, &c).unwrap(), "removed");
    assert!(retained(&f.m, &r).is_err());
}
#[test]
fn durable_legacy_provenance_survives_current_host_inventory_advance() {
    let (f, mut c) = fixture();
    c.origin = Origin::RetainedSv1;
    c.inspection.origin = Origin::RetainedSv1;
    c.recipe_sha256 = "retained-sv1".into();
    let on =
        f.m.root
            .join("onboarding")
            .join(&c.selection.environment.id)
            .join("record.json");
    private_dir(on.parent().unwrap()).unwrap();
    atomic_json(&on, &json!({"test":"original onboarding"})).unwrap();
    let inv =
        f.m.root
            .join("inventory")
            .join(format!("{}.json", c.selection.environment.id));
    let b = json!({"environment_sha256":digest(&c.selection.environment.root.join("environment.json")).unwrap(),"onboarding_sha256":digest(&on).unwrap(),"inventory_sha256":digest(&inv).unwrap(),"inspection_sha256":c.inspection.report.sha256});
    history::materialize_legacy(&f.m, &c, &b).unwrap();
    let id = c.id().unwrap();
    let before = fs::read(object(&f.m, "legacy", &id).unwrap().join("provenance.json")).unwrap();
    atomic_json(&inv, &json!({"new":"scanner generation"})).unwrap();
    fs::remove_file(&on).unwrap();
    verify_legacy(&f.m, &c).unwrap(); // Never kit verification for retained-sv1.
    assert_eq!(candidate(&f.m, &id, &c.host, &"ff".repeat(32)).unwrap(), c);
    assert_eq!(
        before,
        fs::read(object(&f.m, "legacy", &id).unwrap().join("provenance.json")).unwrap()
    );
    let v = view(&f.m, &c.selection, &c.host, &"ff".repeat(32)).unwrap();
    assert_eq!(v.candidates[0].disposition, "historical");
    assert_eq!(v.candidates[0].origin, Origin::RetainedSv1);
}
#[test]
fn processing_restart_is_required_and_is_not_normal_retirement() {
    let (f, c) = fixture();
    record_observation(
        &f.m,
        &c,
        &random_id().unwrap(),
        Area::ProcessingRestart,
        TestStatus::Failed,
        "Restart failed; terminal cleanup was positive",
    )
    .unwrap();
    let observations = observations(&f.m, &c).unwrap();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].area, Area::ProcessingRestart);
    assert!(!observations.iter().any(|o| o.area == Area::Retirement));
    let missing = unmet(&observations, &Role::Instrument);
    assert!(missing.iter().any(|s| s.starts_with("ProcessingRestart")));
    assert!(missing.iter().any(|s| s.starts_with("Retirement")));
}

#[test]
fn complete_build_identity_reuses_only_the_exact_generation() {
    let (f, a) = fixture();
    let kitpath = f.m.root.join("software/generated-kit.zip");
    fs::write(&kitpath, b"generated immutable kit identity").unwrap();
    fs::set_permissions(&kitpath, fs::Permissions::from_mode(0o400)).unwrap();
    let kit = Artifact {
        sha256: digest(&kitpath).unwrap(),
        path: kitpath,
    };
    let dir = f.m.root.join("software/preparation-kits").join(&kit.sha256);
    private_dir(&dir).unwrap();
    let make = |name: &str, bytes: &[u8]| {
        let path = dir.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        Artifact {
            sha256: digest(&path).unwrap(),
            path,
        }
    };
    let host = make("host.exe", &fs::read(&a.host.path).unwrap());
    let manifest = make(
        "host-source-manifest.json",
        &fs::read(&a.source_manifest.path).unwrap(),
    );
    let builder = make("native_builder.py", b"exact source-owned builder");
    let generator = make("ap8_descriptor.py", b"exact source-owned descriptor");
    let runtime = build::Runtime {
        kit: kit.clone(),
        host: host.clone(),
        source_manifest: manifest.clone(),
        builder: Some(builder.clone()),
        generator: Some(generator.clone()),
    };
    immutable(&dir.join("runtime.json"), &runtime).unwrap();
    let i = inspect_record_with(
        a.selection.clone(),
        a.inspection.report.clone(),
        Origin::ManagedPreparation,
        host.clone(),
        manifest.clone(),
    )
    .unwrap();
    let c = prepared(
        a.selection.clone(),
        i.clone(),
        a.native.clone(),
        host,
        manifest,
        kit.sha256.clone(),
    )
    .unwrap();
    let build = c.native.artifact.path.with_file_name("build.json");
    atomic_json(&build,&json!({"kit_sha256":kit.sha256,"builder_sha256":builder.sha256,"generator_sha256":generator.sha256,"native_sha256":c.native.artifact.sha256,"descriptor_sha256":c.native.descriptor_sha256,"source_commit":c.native.source_commit})).unwrap();
    record_candidate(&f.m, &c).unwrap();
    assert_eq!(
        build::reusable(&f.m, &c.selection, &i, &kit.sha256).unwrap(),
        Some(c.clone())
    );
    assert!(build::reusable(&f.m, &c.selection, &i, &"ff".repeat(32))
        .unwrap()
        .is_none());
    let different = next_generation(&f.m, &c);
    assert!(
        build::reusable(&f.m, &c.selection, &different.inspection, &kit.sha256)
            .unwrap()
            .is_none()
    );
    let mut row: Value = read_json(&build).unwrap();
    row["generator_sha256"] = json!("00".repeat(32));
    atomic_json(&build, &row).unwrap();
    assert!(build::reusable(&f.m, &c.selection, &i, &kit.sha256).is_err());
}

#[test]
fn metadata_generation_binds_review_basis_without_rebuilding_identical_artifact() {
    let (f, a) = fixture();
    record_candidate(&f.m, &a).unwrap();
    let basis = preparation_basis(&f.m, Some(&a)).unwrap();
    let b = bind_preparation_basis(a.clone(), Some(basis.clone())).unwrap();
    assert_ne!(a.id().unwrap(), b.id().unwrap());
    assert_eq!(a.native, b.native);
    assert_eq!(preparation_basis(&f.m, Some(&b)).unwrap(), basis);
    assert_eq!(bind_preparation_basis(b.clone(), Some(basis)).unwrap(), b);
    record_candidate(&f.m, &b).unwrap();
    record_observation(
        &f.m,
        &b,
        &random_id().unwrap(),
        Area::Audio,
        TestStatus::Unavailable,
        "A new review basis retains this observation",
    )
    .unwrap();
    let changed = preparation_basis(&f.m, Some(&b)).unwrap();
    let next = bind_preparation_basis(b.clone(), Some(changed)).unwrap();
    assert_ne!(next.id().unwrap(), b.id().unwrap());
    assert_eq!(next.native, b.native);
}

#[test]
fn original_sv1_projection_separates_restart_failure_from_terminal_cleanup() {
    let (_f, mut c) = fixture();
    let report: Value = serde_json::from_slice(include_bytes!(
        "../../../evidence/sv1/first-operator-session.json"
    ))
    .unwrap();
    c.selection.module.sha256 = report["module_sha256"].as_str().unwrap().into();
    c.selection.class.id = report["class_id"].as_str().unwrap().into();
    c.host.sha256 = report["host_sha256"].as_str().unwrap().into();
    c.source_manifest.sha256 = report["host_source_manifest_sha256"]
        .as_str()
        .unwrap()
        .into();
    let rows = retained_sv1_observations(&c, &report).unwrap();
    assert_eq!(
        rows.iter()
            .find(|r| r.area == Area::ProcessingRestart)
            .unwrap()
            .status,
        TestStatus::Failed
    );
    assert_eq!(
        rows.iter()
            .find(|r| r.area == Area::Retirement)
            .unwrap()
            .status,
        TestStatus::NotTested
    );
    assert_eq!(report["cleanup_confirmed"], true);
    assert_eq!(report["transport_retired"], true);
    c.selection.class.id = "00".repeat(16);
    assert!(retained_sv1_observations(&c, &report).unwrap().is_empty());
}

#[test]
fn repeated_ordinary_publication_is_nonmutating_at_both_owners() {
    let (f, c) = fixture();
    record_candidate(&f.m, &c).unwrap();
    for area in AREAS {
        record_observation(
            &f.m,
            &c,
            &random_id().unwrap(),
            area,
            TestStatus::Passed,
            "Generated exact review",
        )
        .unwrap();
    }
    review(
        &f.m,
        &c,
        &random_id().unwrap(),
        ReviewChoice::AcceptExactLocal,
        "Generated explicit acceptance",
    )
    .unwrap();
    let current = enable(&f.m, &c, true).unwrap();
    let revision = f.m.load_revision(&c.selection.class.id, &current).unwrap();
    let before = snapshot(&f.m.root); // Registry, revisions, pointers, transactions and all evidence.
    assert_eq!(
        enable(&f.m, &c, true).unwrap_err().to_string(),
        "candidate_already_ordinary"
    );
    assert_eq!(snapshot(&f.m.root), before);
    assert_eq!(
        f.m.publish_with_expected(
            &revision.profile,
            &c.census().unwrap(),
            revision.registration.clone(),
            (&c.host, &c.source_manifest.sha256),
            (None, true),
            None,
            None
        )
        .unwrap_err()
        .to_string(),
        "candidate_already_ordinary"
    );
    assert_eq!(snapshot(&f.m.root), before);
    assert_eq!(
        f.m.registry().unwrap().classes[&c.selection.class.id].managed_revision,
        Some(current)
    );
}

fn legacy_fixture() -> (Fixture, Candidate, Value) {
    let (f, mut c) = fixture();
    c.origin = Origin::RetainedSv1;
    c.inspection.origin = Origin::RetainedSv1;
    c.recipe_sha256 = "retained-sv1".into();
    let on =
        f.m.root
            .join("onboarding")
            .join(&c.selection.environment.id)
            .join("record.json");
    private_dir(on.parent().unwrap()).unwrap();
    atomic_json(&on, &json!({"test":"original onboarding"})).unwrap();
    let inv =
        f.m.root
            .join("inventory")
            .join(format!("{}.json", c.selection.environment.id));
    let b = json!({"environment_sha256":digest(&c.selection.environment.root.join("environment.json")).unwrap(),"onboarding_sha256":digest(&on).unwrap(),"inventory_sha256":digest(&inv).unwrap(),"inspection_sha256":c.inspection.report.sha256});
    (f, c, b)
}
#[test]
fn every_legacy_materialization_boundary_recovers_without_partial_final_files() {
    use history::MaterializeBoundary::*;
    for boundary in [
        Environment,
        Onboarding,
        Inventory,
        ProvenanceStaged,
        ProvenanceInstalled,
        Lineage,
        Candidate,
    ] {
        let (f, c, b) = legacy_fixture();
        let dir = object(&f.m, "legacy", &c.id().unwrap()).unwrap();
        assert_eq!(
            history::materialize_legacy_with(&f.m, &c, &b, Some(boundary))
                .unwrap_err()
                .to_string(),
            "legacy_materialization_interrupted"
        );
        for name in ["environment", "onboarding", "inventory"] {
            let p = dir.join(format!("{name}.json"));
            if p.exists() {
                assert_eq!(digest(&p).unwrap(), b[format!("{name}_sha256")]);
            }
        }
        if boundary == ProvenanceStaged {
            assert!(!dir.join("provenance.json").exists());
            let uncommitted = object(&f.m, "candidates", &c.id().unwrap()).unwrap();
            private_dir(&uncommitted).unwrap();
            fs::write(uncommitted.join(".writing-interrupted"), b"partial").unwrap();
            assert!(retained_candidates(&f.m).unwrap().is_empty());
        }
        // A true killed writer may leave a partial *temporary*, which has no authority.
        fs::write(
            dir.join(".writing-interrupted-fixture"),
            b"partial temporary",
        )
        .unwrap();
        history::materialize_legacy(&f.m, &c, &b).unwrap();
        verify_legacy(&f.m, &c).unwrap();
        assert_eq!(retained_candidates(&f.m).unwrap(), vec![c.clone()]);
        assert_eq!(
            inspections(&f.m, &c.selection).unwrap(),
            vec![c.inspection.clone()]
        );
        let line = lineage(&f.m, &c).unwrap();
        assert!(line.preparation_identity.starts_with("sv1:"));
        let seal = fs::read(dir.join("provenance.json")).unwrap();
        history::materialize_legacy(&f.m, &c, &b).unwrap();
        assert_eq!(lineage(&f.m, &c).unwrap(), line);
        assert_eq!(fs::read(dir.join("provenance.json")).unwrap(), seal);
    }
}
#[test]
fn immutable_raw_custody_accepts_exact_and_never_replaces_conflicts() {
    let (f, c, b) = legacy_fixture();
    let dir = object(&f.m, "legacy", &c.id().unwrap()).unwrap();
    private_dir(&dir).unwrap();
    let target = dir.join("environment.json");
    let bytes = fs::read(c.selection.environment.root.join("environment.json")).unwrap();
    immutable_bytes(&target, &bytes).unwrap();
    immutable_bytes(&target, &bytes).unwrap();
    assert_eq!(
        immutable_bytes(&target, b"different")
            .unwrap_err()
            .to_string(),
        "preparation_immutable_conflict"
    );
    assert_eq!(fs::read(&target).unwrap(), bytes);
    let conflict = dir.join("onboarding.json");
    immutable_bytes(&conflict, b"not the bound snapshot").unwrap();
    assert_eq!(
        history::materialize_legacy(&f.m, &c, &b)
            .unwrap_err()
            .to_string(),
        "preparation_immutable_conflict"
    );
    assert_eq!(fs::read(&conflict).unwrap(), b"not the bound snapshot");
    assert!(!dir.join("provenance.json").exists());
    // Atomic no-replace also handles a concurrent exact completion.
    let race = dir.join("race.json");
    immutable_bytes_staged(&race, b"complete", &mut || {
        immutable_bytes(&race, b"complete")
    })
    .unwrap();
    assert_eq!(fs::read(&race).unwrap(), b"complete");
}

#[test]
fn legacy_provenance_conflict_is_not_silently_reused_or_replaced() {
    let (f, c, b) = legacy_fixture();
    history::materialize_legacy(&f.m, &c, &b).unwrap();
    let seal = object(&f.m, "legacy", &c.id().unwrap())
        .unwrap()
        .join("provenance.json");
    let mut v: Value = read_json(&seal).unwrap();
    v["original_evidence"]["foreign_change"] = json!(true);
    atomic_json(&seal, &v).unwrap();
    let before = fs::read(&seal).unwrap();
    assert_eq!(
        history::materialize_legacy(&f.m, &c, &b)
            .unwrap_err()
            .to_string(),
        "preparation_immutable_conflict"
    );
    assert_eq!(fs::read(&seal).unwrap(), before);
}

#[test]
fn installed_candidate_only_upgrade_materializes_history_without_replacing_candidate() {
    let (f, c, b) = legacy_fixture();
    let record = object(&f.m, "candidates", &c.id().unwrap()).unwrap().join("candidate.json");
    immutable(&record, &c).unwrap(); // Actual pre-generation installed state.
    let before = fs::read(&record).unwrap();
    assert_eq!(retained_history_with_binding(&f.m, &b).unwrap(), vec![c.clone()]);
    verify_legacy(&f.m, &c).unwrap();
    assert!(lineage(&f.m, &c).unwrap().preparation_identity.starts_with("sv1:"));
    assert_eq!(inspections(&f.m, &c.selection).unwrap(), vec![c.inspection.clone()]);
    let revision = fs::read(root(&f.m).join("revision.json")).unwrap();
    // Complete history is read-only, including after current inventory advances.
    fs::remove_file(f.m.root.join("inventory").join(format!("{}.json", c.selection.environment.id))).unwrap();
    assert_eq!(retained_history_with_binding(&f.m, &b).unwrap(), vec![c.clone()]);
    assert_eq!(fs::read(root(&f.m).join("revision.json")).unwrap(), revision);
    assert_eq!(fs::read(&record).unwrap(), before);
}

#[test]
fn candidate_only_upgrade_retries_each_interruption_through_readback_owner() {
    use history::MaterializeBoundary::*;
    for boundary in [Environment, Onboarding, Inventory, ProvenanceStaged,
        ProvenanceInstalled, Lineage, Candidate] {
        let (f, c, b) = legacy_fixture();
        let record = object(&f.m, "candidates", &c.id().unwrap()).unwrap().join("candidate.json");
        immutable(&record, &c).unwrap();
        let before = fs::read(&record).unwrap();
        let registry = fs::read(f.m.root.join("registry.json")).unwrap();
        assert_eq!(history::materialize_legacy_with(&f.m, &c, &b, Some(boundary))
            .unwrap_err().to_string(), "legacy_materialization_interrupted");
        retained_history_with_binding(&f.m, &b).unwrap();
        verify_legacy(&f.m, &c).unwrap();
        let revision = fs::read(root(&f.m).join("revision.json")).unwrap();
        retained_history_with_binding(&f.m, &b).unwrap();
        assert_eq!(fs::read(root(&f.m).join("revision.json")).unwrap(), revision);
        assert_eq!(fs::read(&record).unwrap(), before);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry);
        assert!(lineage(&f.m, &c).unwrap().preparation_identity.starts_with("sv1:"));
    }
}
#[test]
fn candidate_only_upgrade_refuses_bound_snapshot_conflict_without_rewriting_it() {
    let (f, c, b) = legacy_fixture();
    let record = object(&f.m, "candidates", &c.id().unwrap()).unwrap().join("candidate.json");
    immutable(&record, &c).unwrap();
    let before = fs::read(&record).unwrap();
    let conflict = object(&f.m, "legacy", &c.id().unwrap()).unwrap().join("environment.json");
    immutable_bytes(&conflict, b"conflicting immutable material").unwrap();
    assert_eq!(retained_history_with_binding(&f.m, &b).unwrap_err().to_string(),
        "preparation_immutable_conflict");
    assert_eq!(fs::read(&conflict).unwrap(), b"conflicting immutable material");
    assert_eq!(fs::read(&record).unwrap(), before);
    assert!(!conflict.with_file_name("provenance.json").exists());
}

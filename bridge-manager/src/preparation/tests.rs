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

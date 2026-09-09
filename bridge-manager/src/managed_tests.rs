use crate::{catalogue::*, observation::*, profiles::*, publication::*, test_fixture::*, *};

fn publish(
    f: &Fixture,
    p: &Profile,
    c: &Census,
    n: &NativeArtifact,
    fail: Option<Boundary>,
) -> Result<RevisionRef> {
    f.m.managed_publish(p, c, derive(p, c, n)?, &c.host, &c.host_source_sha256, fail)
}
fn reason<T>(r: Result<T>, expected: &str) {
    assert_eq!(r.err().unwrap().to_string(), expected);
}

#[test]
fn explicit_host_update_preserves_exact_prior_host_for_rollback_only() {
    let (f, mut p, c, n) = prepared();
    p.revision = 3;
    let key = f.r.key();
    let first = publish(&f, &p, &c, &n, None).unwrap();
    let prior = f.m.load_revision(&key, &first).unwrap();
    let mut next = c.clone();
    let dir = f.m.root.join("software/host-update");
    private_dir(&dir).unwrap();
    fs::write(dir.join("host.exe"), b"updated host same protocol").unwrap();
    fs::write(
        dir.join("host-source-manifest.json"),
        b"updated host source",
    )
    .unwrap();
    next.host = Artifact {
        path: dir.join("host.exe"),
        sha256: digest(&dir.join("host.exe")).unwrap(),
    };
    next.host_source_sha256 = digest(&dir.join("host-source-manifest.json")).unwrap();
    next.report.path = f.outer.join("updated-inspection.json");
    atomic_json(&next.report.path, &inspection_report(&next)).unwrap();
    next.report.sha256 = digest(&next.report.path).unwrap();
    let mut updated = p.clone();
    updated.revision += 1;
    updated.claim = Claim::VerifiedExactFixture;
    updated.requirements.host_sha256 = next.host.sha256.clone();
    updated.requirements.host_source_sha256 = next.host_source_sha256.clone();
    // Product setup may adopt the exact native while the old host/publication
    // remains in use. The subsequent observation still binds the new host.
    adoption(&f.m, std::slice::from_ref(&updated)).unwrap();
    for claim in [Claim::ReviewCandidate, Claim::Withdrawn] {
        let mut denied = updated.clone();
        denied.claim = claim;
        reason(
            f.m.verify_served_host(
                &prior.registration,
                &next.host,
                &next.host_source_sha256,
                &[denied],
            ),
            "installed_host_mismatch",
        );
    }
    let permitted = std::slice::from_ref(&updated);
    f.m.verify_served_host(
        &prior.registration,
        &next.host,
        &next.host_source_sha256,
        permitted,
    )
    .unwrap();
    reason(
        f.m.verify_served_host(
            &prior.registration,
            &next.host,
            &next.host_source_sha256,
            std::slice::from_ref(&p),
        ),
        "installed_host_mismatch",
    );
    let second = publish(&f, &updated, &next, &n, None).unwrap();
    let candidate = f.m.load_revision(&key, &second).unwrap();
    assert_eq!(candidate.external_ids, prior.external_ids);
    assert_eq!(candidate.parent.as_ref(), Some(&first));
    f.m.rollback(&key, &first.id, None).unwrap();
    let identity = f.identity();
    let restored = f.m.resolve(&identity).unwrap();
    assert_eq!(restored, prior.registration);
    f.m.verify_served_host(&restored, &next.host, &next.host_source_sha256, permitted)
        .unwrap();
    fs::write(
        restored
            .host
            .path
            .with_file_name("host-source-manifest.json"),
        b"changed source",
    )
    .unwrap();
    assert!(
        f.m.verify_served_host(&restored, &next.host, &next.host_source_sha256, permitted)
            .is_err()
    );
    // No retained revision means no cross-version host admission.
    let raw = Fixture::new();
    raw.m.register(raw.r.clone()).unwrap();
    reason(
        raw.m
            .verify_served_host(&raw.r, &next.host, &next.host_source_sha256, permitted),
        "installed_host_mismatch",
    );
}

#[test]
fn shipped_profiles_are_closed_and_separate_from_local_bindings() {
    let p = installed_profiles().unwrap();
    assert_eq!(p.len(), 2);
    for p in p {
        p.validate().unwrap();
        assert_eq!(p.claim, Claim::VerifiedExactFixture);
    }
    let (_, p, _, _) = prepared();
    let bytes = serde_json::to_vec(&p).unwrap();
    assert!(Profile::parse(&bytes).is_ok());
    for (key, value) in [
        ("command", serde_json::json!("echo forbidden")),
        ("extra", serde_json::json!({})),
        ("path", serde_json::json!("/machine/local")),
    ] {
        let mut bad = serde_json::to_value(&p).unwrap();
        bad[key] = value;
        assert!(Profile::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    for bad in [
        serde_json::json!({"arbitrary_hook":"anything"}),
        serde_json::json!("unknown_capability"),
    ] {
        let mut v = serde_json::to_value(&p).unwrap();
        v["capabilities"]["state"] = bad;
        assert!(Profile::parse(&serde_json::to_vec(&v).unwrap()).is_err());
    }
    let mut bad = p.clone();
    bad.schema = 2;
    assert!(bad.validate().is_err());
    bad = p.clone();
    bad.id = "../escape".into();
    assert!(bad.validate().is_err());
    bad = p.clone();
    bad.module_sha256 = "bad".into();
    assert!(bad.validate().is_err());
    bad = p.clone();
    bad.evidence = vec!["docs/x".into(); 17];
    assert!(bad.validate().is_err());
    bad = p.clone();
    bad.evidence = vec!["docs/../../x".into()];
    assert!(bad.validate().is_err());
    assert!(Profile::parse(&vec![b' '; PROFILE_LIMIT + 1]).is_err());
    reason(
        validate_set(&[p.clone(), p.clone()]),
        "duplicate_profile_revision",
    );
    assert!(validate_set(&vec![p; PROFILE_COUNT + 1]).is_err());
}
#[test]
fn exact_matching_refuses_ambiguity_and_incompatible_observations() {
    let (f, p, c, n) = prepared();
    assert_eq!(select(std::slice::from_ref(&p), &c).unwrap(), &p);
    let mut other = p.clone();
    other.id = "another.profile".into();
    reason(select(&[p.clone(), other], &c), "profile_ambiguous");
    let mut bad = c.clone();
    bad.selected.class_id = "02".repeat(16);
    reason(select(std::slice::from_ref(&p), &bad), "profile_no_match");
    bad = c.clone();
    bad.module.sha256 = "00".repeat(32);
    reason(
        select(std::slice::from_ref(&p), &bad),
        "module_digest_changed",
    );
    bad = c.clone();
    bad.classes.clear();
    reason(select(std::slice::from_ref(&p), &bad), "class_absent");
    bad = c.clone();
    bad.selected.subcategories = "Fx|Tools".into();
    reason(select(std::slice::from_ref(&p), &bad), "role_mismatch");
    bad = c.clone();
    bad.selected.version = "wrong".into();
    reason(select(std::slice::from_ref(&p), &bad), "metadata_mismatch");
    bad = c.clone();
    bad.environment.environment.revision = 2;
    reason(
        select(std::slice::from_ref(&p), &bad),
        "environment_mismatch",
    );
    bad = c.clone();
    bad.environment.environment.runner.id = "other".into();
    reason(select(std::slice::from_ref(&p), &bad), "runner_mismatch");
    bad = c.clone();
    bad.host.sha256 = "00".repeat(32);
    reason(
        select(std::slice::from_ref(&p), &bad),
        "installed_host_mismatch",
    );
    bad = c.clone();
    bad.float32 = false;
    reason(select(std::slice::from_ref(&p), &bad), "precision_mismatch");
    reason(
        c.verify_current(
            &f.m.root,
            &c.host,
            &c.host_source_sha256,
            c.captured_at + 601,
        ),
        "stale_census",
    );
    let mut wrong = n.clone();
    wrong.artifact.sha256 = "00".repeat(32);
    reason(derive(&p, &c, &wrong), "native_artifact_mismatch");
    let r = derive(&p, &c, &n).unwrap();
    assert_eq!(r.metadata, c.selected);
    assert_eq!(r.native, n.artifact);
    assert!(r.compatibility.disable_windows_accessibility);
    fs::write(&c.module.path, b"changed").unwrap();
    assert!(
        c.verify_current(&f.m.root, &c.host, &c.host_source_sha256, now().unwrap())
            .is_err()
    );
}
#[test]
fn publication_is_idempotent_and_exact_rollback_keeps_external_identity() {
    let (f, p, c, n) = prepared();
    let key = f.r.key();
    let old = fs::read_link(f.m.link(&key)).unwrap();
    let before = fs::read(&f.r.module.path).unwrap();
    let one = publish(&f, &p, &c, &n, None).unwrap();
    let first = f.m.load_revision(&key, &one).unwrap();
    assert_ne!(first.target, old);
    assert!(old.exists());
    assert_eq!(publish(&f, &p, &c, &n, None).unwrap(), one);
    let mut next = p.clone();
    next.revision = 2;
    let two = publish(&f, &next, &c, &n, None).unwrap();
    let second = f.m.load_revision(&key, &two).unwrap();
    assert_eq!(second.parent, Some(one.clone()));
    assert_eq!(first.external_ids, second.external_ids);
    f.m.rollback(&key, &one.id, None).unwrap();
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), first.target);
    f.m.unpublish(&key).unwrap();
    assert!(!f.m.link(&key).exists());
    f.m.rollback(&key, &one.id, None).unwrap();
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), first.target);
    assert_eq!(fs::read(&f.r.module.path).unwrap(), before);
    // Raw register still refuses a changed/managed binding.
    assert!(f.m.register(f.r.clone()).is_err());
}
#[test]
fn every_publication_boundary_recovers_using_physical_links_and_preserves_prior() {
    for point in BOUNDARIES {
        let (f, p, c, n) = prepared();
        let key = f.r.key();
        let old = fs::read_link(f.m.link(&key)).unwrap();
        let old_hash = digest(&f.m.native_path(&f.r)).unwrap();
        assert!(publish(&f, &p, &c, &n, Some(point)).is_err(), "{point:?}");
        let activated = matches!(
            point,
            Boundary::PointerExchanged
                | Boundary::PointerSynced
                | Boundary::RegistryCommitted
                | Boundary::ResultWritten
                | Boundary::Cleanup
        );
        f.m.reconcile().unwrap();
        f.m.reconcile().unwrap();
        let active = fs::read_link(f.m.link(&key)).unwrap();
        assert_eq!(active != old, activated, "{point:?}");
        assert_eq!(digest(&f.m.native_path(&f.r)).unwrap(), old_hash);
        let e = f.m.registry().unwrap().classes.remove(&key).unwrap();
        assert_eq!(f.m.entry_target(&e).unwrap(), active);
        assert!(!f.m.publication_pending(&key).unwrap());
    }
}
#[test]
fn candidate_corruption_after_activation_restores_exact_prior() {
    let (f, p, c, n) = prepared();
    let key = f.r.key();
    let old = fs::read_link(f.m.link(&key)).unwrap();
    assert!(publish(&f, &p, &c, &n, Some(Boundary::PointerExchanged)).is_err());
    let target = fs::read_link(f.m.link(&key)).unwrap();
    let native = target
        .join("Contents/x86_64-linux")
        .join(format!("LVB_{key}.so"));
    fs::set_permissions(&native, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&native, b"corrupt candidate").unwrap();
    f.m.reconcile().unwrap();
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), old);
}
#[test]
fn foreign_publication_is_never_overwritten() {
    let (f, p, c, n) = prepared();
    let link = f.m.link(&f.r.key());
    fs::remove_file(&link).unwrap();
    fs::create_dir(&link).unwrap();
    fs::write(link.join("foreign"), b"keep").unwrap();
    assert!(publish(&f, &p, &c, &n, None).is_err());
    assert!(f.m.unpublish(&f.r.key()).is_err());
    assert_eq!(fs::read(link.join("foreign")).unwrap(), b"keep");
}

fn lease(f: &Fixture, key: &str, keeper: bool) -> PathBuf {
    let sid = random_id().unwrap();
    let results = f.m.root.join("runtime/results");
    let leases = f.m.root.join("runtime/leases");
    private_dir(&results).unwrap();
    private_dir(&leases).unwrap();
    let report = results.join(format!(
        "{}-{sid}.json",
        if keeper { "environment" } else { "windows" }
    ));
    let directory =
        f.r.environment
            .root
            .join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&sid);
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("owner.json"),&serde_json::json!({"session":sid,"report":report,"keeper":keeper,"registration":{"metadata":{"class_id":key}}})).unwrap();
    let path = leases.join(format!("{sid}.json"));
    atomic_json(&path, &report).unwrap();
    path
}
#[test]
fn active_target_refuses_mutation_but_keeper_and_healthy_sibling_survive() {
    let (f, p, c, n) = prepared();
    let key = f.r.key();
    let one = publish(&f, &p, &c, &n, None).unwrap();
    let keeper = lease(&f, &key, true);
    let mut second = p.clone();
    second.revision = 2;
    let two = publish(&f, &second, &c, &n, None).unwrap();
    let active = lease(&f, &key, false);
    reason(f.m.rollback(&key, &one.id, None), "active_device_lease");
    assert!(f.m.unpublish(&key).is_err());
    assert!(publish(&f, &p, &c, &n, None).is_err());
    fs::remove_file(active).unwrap();
    let sibling = lease(&f, &"02".repeat(16), false);
    f.m.rollback(&key, &one.id, None).unwrap();
    assert!(keeper.exists() && sibling.exists());
    assert_ne!(one, two);
}
#[test]
fn failed_updates_and_profile_revision_reuse_keep_the_prior_exact() {
    let (f, p, c, n) = prepared();
    let one = publish(&f, &p, &c, &n, None).unwrap();
    let key = f.r.key();
    let before = fs::read_link(f.m.link(&key)).unwrap();
    let mut bad = p.clone();
    bad.limitations.push(Limitation::DetachedFocusRefusal);
    reason(publish(&f, &bad, &c, &n, None), "immutable_record_conflict");
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), before);
    let mut updated = p.clone();
    updated.revision = 2;
    let mut native = n.clone();
    native.artifact.path = f.m.root.join("software/new-native.so");
    fs::write(&native.artifact.path, b"new native revision").unwrap();
    native.artifact.sha256 = digest(&native.artifact.path).unwrap();
    updated.requirements.native_sha256 = native.artifact.sha256.clone();
    let two = publish(&f, &updated, &c, &native, None).unwrap();
    assert_ne!(one, two);
    f.m.rollback(&key, &one.id, None).unwrap();
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), before);
    assert_eq!(
        digest(
            &f.m.load_revision(&key, &one)
                .unwrap()
                .registration
                .native
                .path
        )
        .unwrap(),
        n.artifact.sha256
    );
}
#[test]
fn rollback_and_unpublish_interruptions_reconcile_without_touching_siblings() {
    for remove in [false, true] {
        for point in [
            Boundary::Intent,
            Boundary::PointerStaged,
            Boundary::PointerExchanged,
            Boundary::PointerSynced,
            Boundary::RegistryCommitted,
            Boundary::ResultWritten,
            Boundary::Cleanup,
        ] {
            let (f, p, c, n) = prepared();
            let one = publish(&f, &p, &c, &n, None).unwrap();
            let key = f.r.key();
            let mut next = p.clone();
            next.revision = 2;
            let two = publish(&f, &next, &c, &n, None).unwrap();
            let newer = f.m.load_revision(&key, &two).unwrap().target;
            let mut sibling = f.r.clone();
            sibling.metadata.class_id = "02".repeat(16);
            f.m.register(sibling.clone()).unwrap();
            let sibling_before = fs::read_link(f.m.link(&sibling.key())).unwrap();
            if remove {
                let lock = f.m.lock("registry.lock").unwrap();
                let mut db = f.m.registry().unwrap();
                assert!(f.m.remove_revision(&mut db, &key, Some(point)).is_err());
                drop(lock);
            } else {
                assert!(f.m.rollback(&key, &one.id, Some(point)).is_err());
            }
            f.m.reconcile().unwrap();
            let activated = !matches!(point, Boundary::Intent | Boundary::PointerStaged);
            let actual = fs::read_link(f.m.link(&key)).ok();
            let expected = if activated {
                if remove {
                    None
                } else {
                    Some(f.m.load_revision(&key, &one).unwrap().target)
                }
            } else {
                Some(newer)
            };
            assert_eq!(actual, expected, "remove={remove} point={point:?}");
            assert_eq!(
                fs::read_link(f.m.link(&sibling.key())).unwrap(),
                sibling_before
            );
        }
    }
}

#[test]
fn status_requires_real_target_and_provenance_and_reports_pending() {
    use crate::readback::RefusalCode;
    let (f, p, c, n) = prepared();
    let key = f.r.key();
    let read = || {
        f.m.managed_status_for_policy(&c.host, &c.host_source_sha256, std::slice::from_ref(&p))
            .unwrap()
            .products
            .remove(0)
    };
    assert!(read().publication_valid);
    let one = publish(&f, &p, &c, &n, None).unwrap();
    let row = read();
    assert_eq!(row.active_revision, Some(one));
    assert!(row.prior_rollback.unwrap().valid);
    assert!(row.refusal.is_none());
    let target = fs::read_link(f.m.link(&key)).unwrap();
    fs::remove_file(f.m.link(&key)).unwrap();
    let row = read();
    assert!(!row.publication_valid);
    assert!(row.active_revision.is_none());
    assert_eq!(row.refusal.unwrap().code, RefusalCode::ForeignPublication);
    symlink(&target, f.m.link(&key)).unwrap();
    assert!(publish(&f, &p, &c, &n, Some(Boundary::CandidateReady)).is_err());
    assert_eq!(read().refusal.unwrap().code, RefusalCode::RecoveryPending);
    f.m.reconcile().unwrap();
    let native = target
        .join("Contents/x86_64-linux")
        .join(format!("LVB_{key}.so"));
    fs::remove_file(&native).unwrap();
    fs::remove_file(f.m.link(&key)).unwrap();
    let row = read();
    assert!(!row.publication_valid);
    assert!(!row.native_artifact_valid);
    assert!(row.active_revision.is_none());
    assert_eq!(row.refusal.unwrap().code, RefusalCode::NativeMismatch);
}
#[test]
fn exact_native_catalogue_adopts_owned_copy_without_build_tree_dependency() {
    let (f, p, c, n) = prepared();
    let mut catalogue = adoption(&f.m, std::slice::from_ref(&p)).unwrap();
    assert_eq!(catalogue.natives[0].artifact.path, f.m.native_path(&f.r));
    assert_ne!(catalogue.natives[0].artifact.path, f.r.native.path);
    // This is the copy setup installs with product software.
    catalogue.natives[0].artifact = n.artifact;
    catalogue.validate(&f.m.root).unwrap();
    fs::remove_file(&f.r.native.path).unwrap();
    let derived = derive(&p, &c, catalogue.native(&p).unwrap()).unwrap();
    f.m.managed_publish(&p, &c, derived, &c.host, &c.host_source_sha256, None)
        .unwrap();
    assert!(f.m.resolve(&f.identity()).is_ok());
    catalogue.natives[0].external_ids.swap(0, 1);
    assert!(catalogue.validate(&f.m.root).is_err());
    assert_eq!(
        external_ids("417274754156495350724C4650726F63").unwrap(),
        [
            "ac7f2418f2915958b5b67620593b9a2d",
            "0c80f2acc39f5975a38d177102555683"
        ]
    );
    assert_eq!(
        external_ids("41727475415649536772616E50726F63").unwrap(),
        [
            "96bd361810b3568a9671700732903d6e",
            "d3181e9d37e75871ba5ffbcae0775861"
        ]
    );
}
#[test]
fn production_inspection_consumer_requires_completed_correlated_exact_sdk_census() {
    let (f, mut p, mut c, _) = prepared();
    c.selected.metadata_tier = "factory_3_unicode".into();
    p.class = c.selected.clone();
    let original = inspection_report(&c);
    let parse = |v: &serde_json::Value| {
        atomic_json(&c.report.path, v).unwrap();
        let report = Artifact {
            path: c.report.path.clone(),
            sha256: digest(&c.report.path).unwrap(),
        };
        Census::from_report(
            c.environment.clone(),
            c.module.clone(),
            c.module_stamp.clone(),
            c.host.clone(),
            c.host_source_sha256.clone(),
            report,
            &c.selected.class_id,
        )
    };
    let actual = parse(&original).unwrap();
    assert_eq!(actual.selected, c.selected);
    assert_eq!(actual.classes.len(), 2);
    assert_eq!(select(std::slice::from_ref(&p), &actual).unwrap(), &p);
    // Optional parameter readback stays unavailable; opaque state success is
    // neither required nor invented by publication inspection.
    assert_eq!(actual.parameter_count, 1);
    actual
        .verify_current(&f.m.root, &c.host, &c.host_source_sha256, now().unwrap())
        .unwrap();
    for (index, field, value, expected) in [
        (
            0,
            "scanner_sha256",
            serde_json::json!("00".repeat(32)),
            "inspection_binding_mismatch",
        ),
        (
            2,
            "class_id",
            serde_json::json!("02".repeat(16)),
            "class_mismatch",
        ),
        (
            3,
            "count",
            serde_json::json!(2),
            "parameter_metadata_incomplete",
        ),
        (
            6,
            "exit_code",
            serde_json::json!(90),
            "inspection_incomplete",
        ),
    ] {
        let mut bad = original.clone();
        bad["records"][index][field] = value;
        reason(parse(&bad), expected);
    }
    let mut bad = original.clone();
    bad["records"][1]["classes"][0]["raw_tuid_hex"] = serde_json::json!("00".repeat(16));
    reason(parse(&bad), "class_absent");
    bad = original.clone();
    bad["records"][1]["classes"][0]["version_hex"] = serde_json::json!("31002e003900");
    reason(parse(&bad), "metadata_mismatch");
    bad = original.clone();
    bad["cleanup_confirmed"] = serde_json::json!(false);
    reason(parse(&bad), "inspection_failed");
    bad = original.clone();
    bad["records"][5]["float64_result"] = serde_json::json!(0);
    reason(select(&[p], &parse(&bad).unwrap()), "precision_mismatch");
}
#[test]
fn installed_operator_command_preserves_foreign_entries_and_updates_only_owned_link() {
    let (f, _, _, _) = prepared();
    let dir = f.outer.join("bin");
    let link = dir.join("linux-vst-bridge");
    let old = f.m.root.join("software/old-manager");
    let new = f.m.root.join("software/new-manager");
    install_command(&link, &old, None).unwrap();
    install_command(&link, &old, None).unwrap();
    install_command(&link, &new, Some(&old)).unwrap();
    assert_eq!(fs::read_link(&link).unwrap(), new);
    fs::remove_file(&link).unwrap();
    fs::write(&link, b"foreign command").unwrap();
    assert!(install_command(&link, &old, Some(&new)).is_err());
    assert_eq!(fs::read(&link).unwrap(), b"foreign command");
}

#[test]
fn two_managed_classes_update_remove_and_roll_back_independently() {
    let (f, p, c, n) = prepared();
    let instrument = publish(&f, &p, &c, &n, None).unwrap();
    let mut effect = p.clone();
    effect.id = "fixture.effect".into();
    effect.class.class_id = "02".repeat(16);
    effect.class.name = "Observed effect".into();
    effect.class.subcategories = "Fx|Tools".into();
    effect.role = Role::Effect;
    let mut facts = c.clone();
    facts.selected = effect.class.clone();
    facts.report.path = f.outer.join("effect-report.json");
    atomic_json(&facts.report.path, &inspection_report(&facts)).unwrap();
    facts.report.sha256 = digest(&facts.report.path).unwrap();
    facts = Census::from_report(
        facts.environment,
        facts.module,
        facts.module_stamp,
        facts.host,
        facts.host_source_sha256,
        facts.report,
        &effect.class.class_id,
    )
    .unwrap();
    let mut native = n.clone();
    native.class = effect.class.clone();
    native.external_ids = external_ids(&effect.class.class_id).unwrap();
    let one = publish(&f, &effect, &facts, &native, None).unwrap();
    let key = effect.class.class_id.clone();
    assert!(f.m.load_revision(&key, &one).unwrap().parent.is_none());
    let instrument_target = fs::read_link(f.m.link(&p.class.class_id)).unwrap();
    let state = f.r.environment.root.join("operator-state");
    fs::write(&state, b"local fixture state sentinel").unwrap();
    effect.revision = 2;
    publish(&f, &effect, &facts, &native, None).unwrap();
    f.m.unpublish(&key).unwrap();
    assert_eq!(
        fs::read_link(f.m.link(&p.class.class_id)).unwrap(),
        instrument_target
    );
    f.m.rollback(&key, &one.id, None).unwrap();
    assert_eq!(
        f.m.registry().unwrap().classes[&p.class.class_id].managed_revision,
        Some(instrument)
    );
    assert_eq!(fs::read(state).unwrap(), b"local fixture state sentinel");
    assert_eq!(
        f.m.managed_status_for_policy(&c.host, &c.host_source_sha256, std::slice::from_ref(&p))
            .unwrap()
            .products
            .len(),
        2
    );
}
#[test]
fn pending_status_reads_the_activated_candidate_before_registry_commit() {
    let (f, p, c, n) = prepared();
    let old = publish(&f, &p, &c, &n, None).unwrap();
    assert!(publish(&f, &p, &c, &n, Some(Boundary::PointerExchanged)).is_err());
    let row =
        f.m.managed_status_for_policy(&c.host, &c.host_source_sha256, std::slice::from_ref(&p))
            .unwrap()
            .products
            .remove(0);
    assert!(row.recovery_pending);
    assert_eq!(row.recorded_revision, Some(old.clone()));
    assert_ne!(row.active_revision, Some(old));
    assert!(row.active_revision.is_some());
    let physical =
        f.m.load_revision(&f.r.key(), row.active_revision.as_ref().unwrap())
            .unwrap();
    assert_eq!(row.physical_target, Some(physical.target));
    f.m.reconcile().unwrap();
}

#[test]
fn claim_eligibility_separates_qualification_new_activation_and_current_host_policy() {
    let (f, mut p, c, n) = prepared();
    p.claim = Claim::ReviewCandidate;
    let p = Profile::parse(&serde_json::to_vec(&p).unwrap()).unwrap();
    let before = snapshot(&f.outer);
    let qualified = select_for(
        std::slice::from_ref(&p),
        &c,
        SelectionPurpose::Qualification,
    )
    .unwrap();
    assert_eq!(qualified, &p);
    let registration = derive_for(&p, &c, &n, SelectionPurpose::Qualification).unwrap();
    for (claim, error, code) in [
        (
            Claim::ReviewCandidate,
            "profile_review_candidate_not_activatable",
            readback::RefusalCode::ReviewCandidateNotActivatable,
        ),
        (
            Claim::Withdrawn,
            "profile_withdrawn",
            readback::RefusalCode::ProfileWithdrawn,
        ),
    ] {
        let mut denied = p.clone();
        denied.claim = claim;
        reason(select(std::slice::from_ref(&denied), &c), error);
        let e =
            f.m.managed_publish(
                &denied,
                &c,
                registration.clone(),
                &c.host,
                &c.host_source_sha256,
                None,
            )
            .unwrap_err();
        assert_eq!(readback::refusal(e.as_ref()).code, code);
        reason(
            f.m.verify_served_host(&registration, &c.host, &c.host_source_sha256, &[denied]),
            "installed_host_mismatch",
        );
        assert_eq!(snapshot(&f.outer), before);
    }
    let mut verified = p.clone();
    verified.claim = Claim::VerifiedExactFixture;
    f.m.verify_served_host(
        &registration,
        &c.host,
        &c.host_source_sha256,
        std::slice::from_ref(&verified),
    )
    .unwrap();
    let mut duplicate = verified.clone();
    duplicate.id = "fixture.duplicate".into();
    reason(
        f.m.verify_served_host(
            &registration,
            &c.host,
            &c.host_source_sha256,
            &[verified.clone(), duplicate],
        ),
        "installed_host_mismatch",
    );
    publish(&f, &verified, &c, &n, None).unwrap();
    let row =
        f.m.managed_status_for_policy(&c.host, &c.host_source_sha256, &[verified])
            .unwrap()
            .products
            .remove(0);
    assert_eq!(
        serde_json::to_value(row.profile).unwrap()["claim"],
        "verified_exact_fixture"
    );
    assert!(row.refusal.is_none());
    // Refusal must not reconcile an unrelated already pending transaction.
    let mut next = p.clone();
    next.claim = Claim::VerifiedExactFixture;
    next.revision += 1;
    assert!(publish(&f, &next, &c, &n, Some(Boundary::CandidateReady)).is_err());
    let pending = snapshot(&f.outer);
    reason(
        f.m.managed_publish(&p, &c, registration, &c.host, &c.host_source_sha256, None),
        "profile_review_candidate_not_activatable",
    );
    assert_eq!(snapshot(&f.outer), pending);
}

#[test]
fn exact_retained_candidate_and_legacy_rollback_survive_verified_transition() {
    let (f, mut p, c, n) = prepared();
    p.revision = 2;
    p.claim = Claim::ReviewCandidate;
    let reg = derive_for(&p, &c, &n, SelectionPurpose::Qualification).unwrap();
    let two = retained_candidate_fixture(&f.m, &p, &c, reg);
    let old = f.m.load_revision(&f.r.key(), &two).unwrap();
    let legacy = old.parent.clone().unwrap();
    let old_bytes = snapshot(old.target.parent().unwrap());
    let row =
        f.m.managed_status_for_policy(&c.host, &c.host_source_sha256, std::slice::from_ref(&p))
            .unwrap()
            .products
            .remove(0);
    assert_eq!(
        serde_json::to_value(row.profile).unwrap()["claim"],
        "review_candidate"
    );
    assert_eq!(
        row.refusal.unwrap().code,
        readback::RefusalCode::HostMismatch
    );
    let mut verified = p.clone();
    verified.revision = 3;
    verified.claim = Claim::VerifiedExactFixture;
    let three = publish(&f, &verified, &c, &n, None).unwrap();
    assert_eq!(
        f.m.load_revision(&f.r.key(), &three).unwrap().parent,
        Some(two.clone())
    );
    f.m.rollback(&f.r.key(), &two.id, None).unwrap();
    let restored = f.m.resolve(&f.identity()).unwrap();
    reason(
        f.m.verify_served_host(
            &restored,
            &c.host,
            &c.host_source_sha256,
            std::slice::from_ref(&verified),
        ),
        "qualification_candidate_contract",
    ); // retained rollback is not serving authority
    assert_eq!(fs::read_link(f.m.link(&f.r.key())).unwrap(), old.target);
    assert_eq!(snapshot(old.target.parent().unwrap()), old_bytes);
    f.m.rollback(&f.r.key(), &legacy.id, None).unwrap();
    assert!(
        f.m.load_revision(&f.r.key(), &legacy)
            .unwrap()
            .adopted_legacy
    );
    assert!(f.m.resolve(&f.identity()).is_ok());
}

#[test]
fn revision_three_preserves_exact_revision_two_constraints_and_external_ids() {
    let old = [
        (
            include_bytes!("../tests/fixtures/arturia-pure-lofi-revision-2.json").as_slice(),
            "044482ca6edd1c4faa7d85869b4149d877b4bcc2b993d70b3eed14b5186408dc",
        ),
        (
            include_bytes!("../tests/fixtures/arturia-efx-fragments-revision-2.json").as_slice(),
            "11e19dde31ea185a86a340b8a8007cf672d514c2c2a055a33d4dbefa9e52dfeb",
        ),
    ];
    let current = installed_profiles().unwrap();
    for (bytes, fingerprint) in old {
        let prior = Profile::parse(bytes).unwrap();
        assert_eq!(prior.revision, 2);
        assert_eq!(prior.claim, Claim::ReviewCandidate);
        assert_eq!(prior.fingerprint().unwrap(), fingerprint);
        let new = current.iter().find(|p| p.id == prior.id).unwrap();
        assert_eq!(new.revision, 3);
        assert_eq!(new.claim, Claim::VerifiedExactFixture);
        assert_eq!(
            external_ids(&prior.class.class_id).unwrap(),
            external_ids(&new.class.class_id).unwrap()
        );
        assert!(prior.evidence.iter().all(|e| new.evidence.contains(e)));
        for evidence in [
            "evidence/ap14/transactions.json",
            "evidence/ap14/installed-identities.json",
        ] {
            assert!(new.evidence.iter().any(|e| e == evidence));
        }
        let mut normalized = new.clone();
        normalized.revision = prior.revision;
        normalized.claim = prior.claim.clone();
        normalized.evidence = prior.evidence.clone();
        assert_eq!(normalized, prior);
    }
}

#[test]
fn canonical_claim_names_preserve_all_lifecycle_states() {
    for (claim, name) in [
        (Claim::ReviewCandidate, "review_candidate"),
        (Claim::VerifiedExactFixture, "verified_exact_fixture"),
        (Claim::Withdrawn, "withdrawn"),
    ] {
        let selected = readback::ProfileSelection {
            id: "fixture.instrument".into(),
            revision: 2,
            activation_permitted: claim.permits(SelectionPurpose::Activation),
            claim,
        };
        assert_eq!(serde_json::to_value(selected).unwrap()["claim"], name);
    }
}

fn editor_candidate(
    f: &Fixture,
    prior: &Profile,
    census: &Census,
    native: &NativeArtifact,
) -> (Profile, Census, NativeArtifact) {
    let mut p = prior.clone();
    p.revision = 4;
    p.claim = Claim::ReviewCandidate;
    p.capabilities.editor = Editor::DetachedDirectVendorLifecycle;
    let mut c = census.clone();
    let mut n = native.clone();
    let dir = f.m.root.join("software/ap15-fixture");
    private_dir(&dir).unwrap();
    fs::write(dir.join("host.exe"), b"candidate owner with GUI v4").unwrap();
    fs::write(
        dir.join("host-source-manifest.json"),
        b"candidate host source",
    )
    .unwrap();
    fs::write(dir.join("native.so"), b"candidate direct native").unwrap();
    c.host = Artifact {
        path: dir.join("host.exe"),
        sha256: digest(&dir.join("host.exe")).unwrap(),
    };
    c.host_source_sha256 = digest(&dir.join("host-source-manifest.json")).unwrap();
    c.report.path = dir.join("inspection.json");
    atomic_json(&c.report.path, &inspection_report(&c)).unwrap();
    c.report.sha256 = digest(&c.report.path).unwrap();
    n.artifact = Artifact {
        path: dir.join("native.so"),
        sha256: digest(&dir.join("native.so")).unwrap(),
    };
    n.source_commit = "ce".repeat(20);
    p.requirements.host_sha256 = c.host.sha256.clone();
    p.requirements.host_source_sha256 = c.host_source_sha256.clone();
    p.requirements.native_sha256 = n.artifact.sha256.clone();
    p.requirements.native_source_commit = n.source_commit.clone();
    let sealed =
        f.m.root
            .join("software/ap15-qualification")
            .join(p.fingerprint().unwrap());
    private_dir(&sealed).unwrap();
    for name in ["host.exe", "host-source-manifest.json", "native.so"] {
        fs::copy(dir.join(name), sealed.join(name)).unwrap();
        fs::set_permissions(sealed.join(name), fs::Permissions::from_mode(0o400)).unwrap();
    }
    c.host.path = sealed.join("host.exe");
    n.artifact.path = sealed.join("native.so");
    atomic_json(&c.report.path, &inspection_report(&c)).unwrap();
    c.report.sha256 = digest(&c.report.path).unwrap();
    (p, c, n)
}
fn qualify(
    f: &Fixture,
    p: &Profile,
    c: &Census,
    n: &NativeArtifact,
    fail: Option<Boundary>,
) -> Result<RevisionRef> {
    f.m.publish_selected(
        p,
        c,
        derive_for(p, c, n, SelectionPurpose::Qualification)?,
        (&c.host, &c.host_source_sha256),
        Some(Qualification::Ap15Editor),
        fail,
    )
}
#[test]
fn qualification_boundaries_restore_exact_verified_parent() {
    for point in BOUNDARIES {
        let (f, mut p, c, n) = prepared();
        p.revision = 3;
        let parent = publish(&f, &p, &c, &n, None).unwrap();
        let key = p.class.class_id.clone();
        let prior = f.m.load_revision(&key, &parent).unwrap();
        let original = snapshot(prior.target.parent().unwrap());
        let vendor = snapshot(&f.r.environment.root);
        let (candidate, census, native) = editor_candidate(&f, &p, &c, &n);
        let result = qualify(&f, &candidate, &census, &native, Some(point));
        reason(result, &format!("injected_{point:?}"));
        f.m.reconcile().unwrap();
        f.m.reconcile().unwrap();
        assert_eq!(
            f.m.registry().unwrap().classes[&key]
                .managed_revision
                .as_ref(),
            Some(&parent),
            "{point:?}"
        );
        assert_eq!(
            fs::read_link(f.m.link(&key)).unwrap(),
            prior.target,
            "{point:?}"
        );
        assert_eq!(
            snapshot(prior.target.parent().unwrap()),
            original,
            "{point:?}"
        );
        assert_eq!(snapshot(&f.r.environment.root), vendor);
        assert!(!f.m.publication_pending(&key).unwrap());
    }
}
#[test]
fn qualification_is_visible_bounded_inactive_and_preserves_ordinary_authority() {
    let (f, mut p, c, n) = prepared();
    p.revision = 3;
    let parent = publish(&f, &p, &c, &n, None).unwrap();
    let key = p.class.class_id.clone();
    let prior = f.m.load_revision(&key, &parent).unwrap();
    // AP14 records encode byte-identically: absent AP15 field is not serialized.
    assert!(
        serde_json::to_value(&prior)
            .unwrap()
            .get("qualification")
            .is_none()
    );
    let (candidate, census, native) = editor_candidate(&f, &p, &c, &n);
    let before = snapshot(&f.outer);
    reason(
        f.m.managed_publish(
            &candidate,
            &census,
            derive_for(
                &candidate,
                &census,
                &native,
                SelectionPurpose::Qualification,
            )
            .unwrap(),
            &census.host,
            &census.host_source_sha256,
            None,
        ),
        "profile_review_candidate_not_activatable",
    );
    assert_eq!(snapshot(&f.outer), before);
    // Missing production Windows/native identities cannot acquire authority.
    reason(
        f.m.qualify_editor(&census, None),
        "ap15_candidate_artifacts_pending",
    );
    reason(
        qualification::stage(&f.m, &f.outer),
        "ap15_candidate_artifacts_pending",
    );
    assert_eq!(snapshot(&f.outer), before);
    let active = lease(&f, &key, false);
    reason(
        qualify(&f, &candidate, &census, &native, None),
        "active_device_lease",
    );
    fs::remove_file(active).unwrap();
    let keeper = lease(&f, &key, true);
    let selected = qualify(&f, &candidate, &census, &native, None).unwrap();
    let r = f.m.load_revision(&key, &selected).unwrap();
    assert_eq!(r.qualification, Some(Qualification::Ap15Editor));
    assert_eq!(r.parent, Some(parent.clone()));
    assert_eq!(r.external_ids, prior.external_ids);
    f.m.verify_retained_authority(&r, std::slice::from_ref(&candidate))
        .unwrap();
    reason(
        f.m.verify_served_host(
            &r.registration,
            &c.host,
            &c.host_source_sha256,
            std::slice::from_ref(&p),
        ),
        "qualification_exact_candidate_required",
    );
    assert!(
        f.m.verify_served_host(
            &r.registration,
            &c.host,
            &c.host_source_sha256,
            std::slice::from_ref(&candidate)
        )
        .is_err()
    );
    let status =
        f.m.managed_status_for_policy(&c.host, &c.host_source_sha256, std::slice::from_ref(&p))
            .unwrap();
    assert_eq!(
        status.products[0].qualification,
        Some(Qualification::Ap15Editor)
    );
    assert!(
        !status.products[0]
            .profile
            .as_ref()
            .unwrap()
            .activation_permitted
    );
    assert!(!status.products[0].installed_host_valid); // compiled candidate roster is absent
    let active = lease(&f, &key, false);
    reason(f.m.reconcile(), "active_device_lease");
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), r.target);
    fs::remove_file(active).unwrap();
    let foreign = f.outer.join("foreign");
    private_dir(&foreign).unwrap();
    fs::remove_file(f.m.link(&key)).unwrap();
    symlink(&foreign, f.m.link(&key)).unwrap();
    assert!(f.m.reconcile().is_err());
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), foreign);
    fs::remove_file(f.m.link(&key)).unwrap();
    symlink(&r.target, f.m.link(&key)).unwrap();
    f.m.reconcile().unwrap();
    assert_eq!(fs::read_link(f.m.link(&key)).unwrap(), prior.target);
    assert!(keeper.exists());
    // Qualification cannot be rebased onto an unverified or later ordinary parent.
    let mut next = p.clone();
    next.revision = 5;
    publish(&f, &next, &c, &n, None).unwrap();
    reason(
        qualify(&f, &candidate, &census, &native, None),
        "qualification_verified_parent_mismatch",
    );
}

#[test]
fn ap15_capability_is_closed_and_verified_revision_three_bytes_are_immutable() {
    use sha2::Digest;
    for (bytes, expected) in [
        (
            include_bytes!("../../compatibility/arturia-pure-lofi.json").as_slice(),
            "52c66718ce8aaa0c4bbfb9f395d515636e5628e779e69aac403635349133653e",
        ),
        (
            include_bytes!("../../compatibility/arturia-efx-fragments.json").as_slice(),
            "c0d6daf0f3c30c995429ec75afded4e5c70323a0a8858ec8a10c667f3ee3c350",
        ),
    ] {
        assert_eq!(hex(&sha2::Sha256::digest(bytes)), expected);
        let prior = Profile::parse(bytes).unwrap();
        assert_eq!(prior.revision, 3);
        assert_eq!(prior.claim, Claim::VerifiedExactFixture);
        let mut candidate = prior.clone();
        candidate.revision = 4;
        candidate.claim = Claim::ReviewCandidate;
        candidate.capabilities.editor = Editor::DetachedDirectVendorLifecycle;
        let mut value = serde_json::to_value(&candidate).unwrap();
        assert_eq!(
            value["capabilities"]["editor"],
            "detached_direct_vendor_lifecycle"
        );
        assert_eq!(
            Profile::parse(&serde_json::to_vec(&value).unwrap()).unwrap(),
            candidate
        );
        assert!(!candidate.claim.permits(SelectionPurpose::Activation));
        assert_eq!(
            external_ids(&prior.class.class_id).unwrap(),
            external_ids(&candidate.class.class_id).unwrap()
        );
        value["capabilities"]["editor"] = serde_json::json!("arbitrary_editor_hook");
        assert!(Profile::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

#[test]
fn retained_candidate_serving_requires_exact_qualification_marker_and_parent() {
    let (f, mut p, c, n) = prepared();
    p.revision = 3;
    let parent = publish(&f, &p, &c, &n, None).unwrap();
    let (candidate, census, native) = editor_candidate(&f, &p, &c, &n);
    let selected = qualify(&f, &candidate, &census, &native, None).unwrap();
    let r = f.m.load_revision(&p.class.class_id, &selected).unwrap();
    let roster = std::slice::from_ref(&candidate);
    f.m.verify_retained_authority(&r, roster).unwrap();
    let mut bad = r.clone();
    bad.qualification = None;
    reason(
        f.m.verify_retained_authority(&bad, roster),
        "qualification_candidate_contract",
    );
    // No second/unknown marker can even enter the typed retained record.
    let mut json = serde_json::to_value(&r).unwrap();
    json["qualification"] = serde_json::json!("another_qualification");
    assert!(serde_json::from_value::<Revision>(json).is_err());
    bad = r.clone();
    bad.profile.capabilities.editor = Editor::DetachedOwnerThreadWithNativePanel;
    reason(
        f.m.verify_retained_authority(&bad, roster),
        "qualification_candidate_contract",
    );
    bad = r.clone();
    bad.parent = None;
    reason(
        f.m.verify_retained_authority(&bad, roster),
        "qualification_verified_parent_required",
    );
    bad = r.clone();
    bad.parent = Some(selected.clone());
    assert!(f.m.verify_retained_authority(&bad, roster).is_err());
    bad = r.clone();
    bad.external_ids.swap(0, 1);
    reason(
        f.m.verify_retained_authority(&bad, roster),
        "qualification_exact_candidate_required",
    );
    bad = r.clone();
    bad.profile.claim = Claim::Withdrawn;
    reason(
        f.m.verify_retained_authority(&bad, roster),
        "qualification_candidate_contract",
    );
    reason(
        f.m.verify_retained_authority(&r, &[]),
        "qualification_exact_candidate_required",
    );
    assert!(qualification::candidates().is_err());
    f.m.rollback(&p.class.class_id, &parent.id, None).unwrap();
    let restored = f.m.load_revision(&p.class.class_id, &parent).unwrap();
    f.m.verify_served_host(&restored.registration, &c.host, &c.host_source_sha256, &[p])
        .unwrap();
}

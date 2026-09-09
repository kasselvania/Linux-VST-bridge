use crate::{catalogue::*, observation::*, profiles::*, publication::*, tests::Fixture, *};

fn prepared() -> (Fixture, Profile, Census, NativeArtifact) {
    let mut f = Fixture::new();
    f.r.compatibility.disable_windows_accessibility = true;
    f.r.metadata.metadata_tier = "factory_3_unicode".into();
    f.m.register(f.r.clone()).unwrap();
    let native_path = f.m.root.join("software/native.so");
    fs::copy(&f.r.native.path, &native_path).unwrap();
    let mut hashes: Vec<_> =
        f.r.environment
            .runner
            .files
            .iter()
            .map(|a| a.sha256.clone())
            .collect();
    hashes.sort();
    let p = Profile {
        schema: 1,
        id: "fixture.instrument".into(),
        revision: 1,
        claim: Claim::ReviewCandidate,
        module_sha256: f.r.module.sha256.clone(),
        factory_vendor: f.r.metadata.vendor.clone(),
        class: f.r.metadata.clone(),
        role: Role::Instrument,
        requirements: Requirements {
            runner: RunnerMatch {
                id: f.r.environment.runner.id.clone(),
                version: f.r.environment.runner.version.clone(),
                proton_sha256: f.r.environment.runner.files[0].sha256.clone(),
                entry_point_sha256: f.r.environment.runner.files[1].sha256.clone(),
                file_sha256: hashes,
            },
            environment_family: Family::ArturiaPersistentV1,
            environment_revision: 1,
            host_sha256: f.r.host.sha256.clone(),
            host_source_sha256: f.r.host_source_sha256.clone(),
            native_sha256: f.r.native.sha256.clone(),
            native_source_commit: "ab".repeat(20),
            descriptor_sha256: "cd".repeat(32),
        },
        capabilities: Capabilities {
            accessibility: Accessibility::DisabledForVendorProcess,
            editor: Editor::DetachedOwnerThreadWithNativePanel,
            state: State::ConcurrentReadOnlyCaptureV12,
            precision: Precision::Float32Only,
            performance: PerformancePolicy::Frames512Recommended256Unqualified,
        },
        limitations: vec![Limitation::ShortDeliveryGaps],
        evidence: vec!["docs/AP13.md".into()],
    };
    let report = f.outer.join("inspection.json");
    fs::write(&report, b"{}").unwrap();
    let c = Census {
        schema: 1,
        id: random_id().unwrap(),
        captured_at: now().unwrap(),
        environment: EnvironmentBinding {
            family: Family::ArturiaPersistentV1,
            environment: f.r.environment.clone(),
        },
        module: f.r.module.clone(),
        module_stamp: ModuleStamp::read(&f.r.module.path).unwrap(),
        host: f.r.host.clone(),
        host_source_sha256: f.r.host_source_sha256.clone(),
        report: Artifact {
            sha256: digest(&report).unwrap(),
            path: report,
        },
        factory_vendor: p.factory_vendor.clone(),
        classes: vec![f.r.key()],
        selected: f.r.metadata.clone(),
        parameter_count: 1,
        float32: true,
        float64: false,
    };
    atomic_json(&c.report.path, &inspection_report(&c)).unwrap();
    let c = Census::from_report(
        c.environment,
        c.module,
        c.module_stamp,
        c.host,
        c.host_source_sha256,
        Artifact {
            sha256: digest(&c.report.path).unwrap(),
            path: c.report.path,
        },
        &f.r.key(),
    )
    .unwrap();
    let n = NativeArtifact {
        class: f.r.metadata.clone(),
        module_sha256: f.r.module.sha256.clone(),
        artifact: Artifact {
            path: native_path,
            sha256: f.r.native.sha256.clone(),
        },
        source_commit: p.requirements.native_source_commit.clone(),
        descriptor_sha256: p.requirements.descriptor_sha256.clone(),
        external_ids: external_ids(&f.r.key()).unwrap(),
    };
    (f, p, c, n)
}
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
fn shipped_profiles_are_closed_and_separate_from_local_bindings() {
    let p = installed_profiles().unwrap();
    assert_eq!(p.len(), 2);
    for p in p {
        p.validate().unwrap();
        assert_eq!(p.claim, Claim::ReviewCandidate);
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
    assert!(c
        .verify_current(&f.m.root, &c.host, &c.host_source_sha256, now().unwrap())
        .is_err());
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
        f.m.managed_status(&c.host, &c.host_source_sha256)
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
fn inspection_report(c: &Census) -> serde_json::Value {
    // Exact production fields from factory_census.cpp and inspect_module.cpp.
    // Raw Windows TUID uses GUID byte order, not the portable FUID string.
    let mut raw = (0..32)
        .step_by(2)
        .map(|i| u8::from_str_radix(&c.selected.class_id[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    raw[..4].reverse();
    raw[4..6].reverse();
    raw[6..8].reverse();
    let utf16 = |s: &str| {
        hex(&s
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>())
    };
    let sdk = serde_json::json!({"raw_tuid_hex":hex(&raw),"tier":"IPluginFactory3.PClassInfoW",
        "name_hex":utf16(&c.selected.name),"vendor_hex":utf16(&c.selected.vendor),"version_hex":utf16(&c.selected.version),
        "category_hex":hex(b"Audio Module Class"),"subcategories_hex":hex(c.selected.subcategories.as_bytes())});
    serde_json::json!({"cleanup_confirmed":true,"transport_retired":true,"gated":true,"error":null,"records":[
        {"state":"readiness_announced","module_sha256":c.module.sha256,"scanner_sha256":c.host.sha256,"implementation_source_manifest_sha256":c.host_source_sha256},
        {"state":"ap8_factory","factory":{"vendor_hex":hex(c.factory_vendor.as_bytes())},"class_count":2,"classes":[sdk,{"raw_tuid_hex":"0123456789ABCDEF0123456789ABCDEF"}]},
        {"state":"ap12_class","class_id":c.selected.class_id,"name":c.selected.name,"vendor":c.selected.vendor,"version":c.selected.version,"subcategories":c.selected.subcategories,"metadata_tier":"factory_3_unicode"},
        {"state":"ap8_parameter_count","count":1},
        {"state":"ap8_parameters","parameters":[[42,"Parameter","",0,1,0.5,null]]},
        {"state":"ap12_capabilities","float32_result":0,"float64_result":1},
        {"state":"ap8_inspection_closed","exit_code":0},{"state":"scanner_completed","inspection_complete":true}]})
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
        f.m.managed_status(&c.host, &c.host_source_sha256)
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
        f.m.managed_status(&c.host, &c.host_source_sha256)
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

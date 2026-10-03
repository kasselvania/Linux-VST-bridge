use super::*;
use crate::test_fixture::{inspection_report, prepared_accessibility, snapshot, Fixture};
use serde_json::json;
pub(crate) fn fixture() -> (Fixture, Candidate) {
    fixture_with_environment(&"13".repeat(16))
}
#[test]
fn candidate_commit_requires_valid_retained_lineage() {
    let (f, c) = fixture();
    let id = c.id().unwrap();
    let record = object(&f.m, "candidates", &id).unwrap().join("candidate.json");
    assert_eq!(record_candidate_with_predecessor(&f.m, &c, Some(&"ff".repeat(32)))
        .unwrap_err().to_string(), "candidate_predecessor_absent");
    assert!(!record.exists());
    assert!(retained_candidates(&f.m).unwrap().is_empty());
    let lineage_path = object(&f.m, "lineage", &id).unwrap().join("record.json");
    immutable(&lineage_path, &CandidateLineage {schema:1,candidate:"ff".repeat(32),
        preparation_identity:"original".into(),ordinal:7,predecessor:None}).unwrap();
    let original = fs::read(&lineage_path).unwrap();
    assert_eq!(record_candidate(&f.m, &c).unwrap_err().to_string(), "candidate_lineage_identity");
    assert!(!record.exists());
    assert!(retained_candidates(&f.m).unwrap().is_empty());
    assert_eq!(fs::read(&lineage_path).unwrap(), original);
    assert!(!root(&f.m).join("revision.json").exists());
}
fn fixture_with_environment(id: &str) -> (Fixture, Candidate) {
    let (mut f, _, mut census, native) = prepared_accessibility(false);
    f.m.unpublish(&f.r.key()).unwrap();
    atomic_json(&f.m.root.join("registry.json"), &Registry::default()).unwrap();
    let old = f.r.environment.root.clone();
    let envroot = f.m.root.join("environments").join(id);
    fs::rename(&old, &envroot).unwrap();
    f.r.module.path = envroot.join(f.r.module.path.strip_prefix(&old).unwrap());
    f.r.environment.id = id.into();
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
fn record_readback_watches_only_used_product_records_not_unrelated_history_payloads() {
    let (f, current) = fixture();
    record_candidate(&f.m, &current).unwrap();
    enable(&f.m, &current, false).unwrap();
    let mut unrelated = current.clone();
    unrelated.selection.class.id = "ef".repeat(16);
    unrelated.inspection.selection = unrelated.selection.clone();
    unrelated.profile.class.class_id = unrelated.selection.class.id.clone();
    unrelated.native.class.class_id = unrelated.selection.class.id.clone();
    unrelated.native.artifact.path = f.outer.join("unrelated-history-payload");
    let id = unrelated.id().unwrap();
    let directory = object(&f.m, "candidates", &id).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("candidate.json"), &unrelated).unwrap();
    let records = RecordReadback::capture(&f.m).unwrap();
    assert_eq!(records.candidates.len(), 2);
    assert_eq!(records.watched_paths().unwrap(), vec![root(&f.m).join("candidates")]);
    assert!(records.catalogue_free_registry(&f.m, &f.m.registry().unwrap()).unwrap());
    let watches = records.watched_paths().unwrap();
    assert!(watches.contains(&current.native.artifact.path));
    assert!(!watches.contains(&unrelated.native.artifact.path));
    assert!(!watches.contains(&directory.join("candidate.json")));
    assert!(!watches.contains(&object(&f.m, "lineage", &id).unwrap().join("record.json")));
}
#[test]
fn retained_uuid_environment_can_prepare_from_current_exact_inventory() {
    let (f, c) = fixture_with_environment("11111111-1111-4111-8111-111111111111");
    let before = snapshot(&f.r.environment.root);
    let current = select(&f.m, &c.selection.id().unwrap(), &f.r.host,
        &f.r.host_source_sha256).unwrap();
    assert_eq!(current, c.selection);
    verify_selection(&f.m, &current, &f.r.host, &f.r.host_source_sha256).unwrap();
    retain_inspection(&f.m, &c.inspection).unwrap();
    record_candidate(&f.m, &c).unwrap();
    let reference = enable(&f.m, &c, false).unwrap();
    let published = f.m.load_revision(&c.selection.class.id, &reference).unwrap();
    assert_eq!(published.registration.environment, f.r.environment);
    assert_eq!(snapshot(&f.r.environment.root), before);
    // A current source/host census is still required; preserving the identifier
    // grants no authority to use stale scanner evidence or another location.
    assert!(select(&f.m, &current.id().unwrap(), &f.r.host, &"ff".repeat(32)).is_err());
    let mut wrong = current.clone();
    wrong.environment.id = "../11111111-1111-4111-8111-111111111111".into();
    assert!(verify_selection_data(&f.m, &wrong, &f.r.host,
        &f.r.host_source_sha256).is_err());
    wrong = current.clone();
    wrong.environment.root = f.m.root.join("outside").join(&wrong.environment.id);
    assert!(verify_selection_data(&f.m, &wrong, &f.r.host,
        &f.r.host_source_sha256).is_err());
}
#[test]
fn retained_accessibility_configuration_requires_consistent_advice_and_limits() {
    let (f, mut candidate) = fixture();
    candidate.profile.capabilities.accessibility = Accessibility::DisabledForVendorProcess;
    assert!(verify_retained_candidate(&f.m, &candidate).unwrap_err().to_string()
        .contains("candidate_accessibility_limitation_changed"));
    candidate.profile.limitations.push(Limitation::WindowsAccessibilityUnavailable);
    assert!(verify_retained_candidate(&f.m, &candidate).unwrap_err().to_string()
        .contains("candidate_policy_requires_explicit_support"));
    candidate.profile.evidence.push("evidence/default-accessibility-advice.json".into());
    verify_retained_candidate(&f.m, &candidate).unwrap();
    candidate.origin = Origin::RetainedSv1;
    assert!(verify_retained_candidate(&f.m, &candidate).is_err());
    candidate.origin = Origin::ManagedPreparation;
    candidate.profile.capabilities.accessibility = Accessibility::WindowsDefault;
    assert!(verify_retained_candidate(&f.m, &candidate).unwrap_err().to_string()
        .contains("candidate_accessibility_limitation_changed"));
}
#[test]
fn managed_publication_needs_no_static_catalogue_but_keeps_exact_authority() {
    let (f, c) = fixture();
    retain_inspection(&f.m, &c.inspection).unwrap();
    record_candidate(&f.m, &c).unwrap();
    let reference = enable(&f.m, &c, false).unwrap();
    assert!(crate::catalogue::catalogue_free_registry(&f.m, &f.m.registry().unwrap()).unwrap());
    assert!(crate::catalogue::catalogue_free_registry_readback(&f.m, &f.m.registry().unwrap()).unwrap());
    let mut foreign = f.m.registry().unwrap();
    foreign.classes.get_mut(&c.selection.class.id).unwrap().registration.native.sha256 = "ff".repeat(32);
    assert!(catalogue_free_registry(&f.m, &foreign).is_err());
    assert!(catalogue_free_registry_readback(&f.m, &foreign).is_err());
    let mut legacy = f.m.registry().unwrap();
    legacy.classes.get_mut(&c.selection.class.id).unwrap().managed_revision = None;
    assert!(!catalogue_free_registry(&f.m, &legacy).unwrap());
    assert!(!catalogue_free_registry_readback(&f.m, &legacy).unwrap());
    let revision = f.m.load_revision(&c.selection.class.id, &reference).unwrap();
    let completion = f.m.root.join("transactions").join(format!("{}.result.json",revision.transaction));
    let saved = completion.with_extension("saved");
    fs::rename(&completion, &saved).unwrap();
    assert!(catalogue_free_registry(&f.m, &f.m.registry().unwrap()).is_err());
    assert!(catalogue_free_registry_readback(&f.m, &f.m.registry().unwrap()).unwrap());
    fs::rename(&saved, &completion).unwrap();
    let link = f.m.link(&c.selection.class.id);
    let target = fs::read_link(&link).unwrap();
    fs::remove_file(&link).unwrap();
    symlink(&f.m.root, &link).unwrap();
    assert!(catalogue_free_registry(&f.m, &f.m.registry().unwrap()).is_err());
    assert!(catalogue_free_registry_readback(&f.m, &f.m.registry().unwrap()).unwrap());
    fs::remove_file(&link).unwrap();
    symlink(target, &link).unwrap();
    disable(&f.m, &c).unwrap();
    assert!(crate::catalogue::catalogue_free_registry(&f.m, &f.m.registry().unwrap()).unwrap());
    assert!(crate::catalogue::catalogue_free_registry_readback(&f.m, &f.m.registry().unwrap()).unwrap());
}
#[test]
fn successor_publication_keeps_explicit_buffering_and_requires_exact_proxy_capacity() {
    let (f, c) = fixture();
    retain_inspection(&f.m, &c.inspection).unwrap();
    record_candidate(&f.m, &c).unwrap();
    let original = enable(&f.m, &c, false).unwrap();
    fn kit(f: &Fixture, c: &Candidate, schema: u32) -> Artifact {
        let path = f.m.root.join("software").join(format!("envelope-{schema}.zip"));
        private_dir(path.parent().unwrap()).unwrap();
        let row = json!({"class_id":c.selection.class.id,"module_sha256":c.selection.module.sha256,
            "native_sha256":c.native.artifact.sha256,"file":"prebuilt/proxy.so",
            "maximum_bridge_frames":1024});
        let index = json!({"schema":schema,"proxies":[row]}).to_string();
        let status = std::process::Command::new("python3").args(["-I", "-c", r#"
import hashlib,json,sys,zipfile
index=sys.argv[2].encode()
with zipfile.ZipFile(sys.argv[1],'w') as archive:
 archive.writestr('prebuilt/index.json',index)
 archive.writestr('recipe.json',json.dumps({'schema':3,'files':{
  'prebuilt/index.json':hashlib.sha256(index).hexdigest(),'prebuilt/proxy.so':sys.argv[3]}}))
"#]).arg(&path).arg(index).arg(&c.native.artifact.sha256).status().unwrap();
        assert!(status.success());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        Artifact {sha256:digest(&path).unwrap(),path}
    }
    let mut software = crate::catalogue::Software {
        installer_launch: None, preparation_kit: None,
        manager: c.host.clone(), operator_frontend: None,
        supervisor: c.host.clone(), ownership: c.host.clone(), host: c.host.clone(),
        source_manifest: c.source_manifest.clone(),
        source_sha256: c.source_manifest.sha256.clone(), native_catalogue: None,
    };
    software.preparation_kit = Some(kit(&f, &c, 2));
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    f.m.select_delay(&c.selection.class.id, 1024).unwrap();
    // Buffering is an independent, explicit preference. A later graphics trial
    // must restore the original publication without reverting that preference
    // to the publication-time snapshot (512).
    let trial = configuration::prepare(&f.m, &c,
        Some(crate::operator_model::GraphicsBackend::WineD3d11), Some(&original)).unwrap();
    let trial_ref = replace(&f.m, &trial, &original).unwrap();
    let before_restore = fs::read(f.m.root.join("registry.json")).unwrap();
    let capable_kit = software.preparation_kit.take();
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    assert!(disable_exact(&f.m, &trial, &trial_ref).is_err(), "unknown target capacity must still refuse");
    assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before_restore);
    assert_eq!(f.m.performance(&c.selection.class.id).unwrap().added_frames, 1024);
    software.preparation_kit = capable_kit;
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    disable_exact(&f.m, &trial, &trial_ref).unwrap();
    assert_eq!(f.m.registry().unwrap().classes[&c.selection.class.id].managed_revision, Some(original.clone()));
    assert_eq!(f.m.performance(&c.selection.class.id).unwrap().added_frames, 1024);
    let next = prepared(c.selection.clone(), c.inspection.clone(), c.native.clone(),
        c.host.clone(), c.source_manifest.clone(), software.preparation_kit.as_ref().unwrap().sha256.clone()).unwrap();
    record_candidate(&f.m, &next).unwrap();
    let reference = replace(&f.m, &next, &original).unwrap();
    let revision = f.m.load_revision(&c.selection.class.id, &reference).unwrap();
    assert_eq!(revision.performance.added_frames, 1024);
    retained(&f.m, &revision).unwrap();
    assert!(catalogue_free_registry(&f.m, &f.m.registry().unwrap()).unwrap());
    // An older exact kit or absent capability cannot authorize this snapshot.
    software.preparation_kit = Some(kit(&f, &c, 1));
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    assert!(retained(&f.m, &revision).unwrap_err().to_string().contains("candidate_runtime_contract"));
    software.preparation_kit = None;
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    assert!(retained(&f.m, &revision).is_err());
    let prior = f.m.load_revision(&c.selection.class.id, &original).unwrap();
    verify_retained_revision(&f.m, &prior).unwrap();
}
#[test]
fn populated_kit_update_retains_exact_published_proxy_capacity() {
    let (f, c) = fixture();
    retain_inspection(&f.m, &c.inspection).unwrap();
    record_candidate(&f.m, &c).unwrap();
    let original = enable(&f.m, &c, false).unwrap();
    fn kit(f: &Fixture, c: &Candidate, name: &str, successor: bool) -> Artifact {
        let path = f.m.root.join("software").join(format!("capacity-{name}.zip"));
        private_dir(path.parent().unwrap()).unwrap();
        let status = std::process::Command::new("python3").args(["-I", "-c", r#"
import hashlib,json,pathlib,sys,zipfile
out,host,source,native,klass,module,successor=sys.argv[1:]
proxy=b'successor proxy fixture' if successor=='true' else pathlib.Path(native).read_bytes()
digest=lambda data:hashlib.sha256(data).hexdigest()
index={'schema':2,'proxies':[{'class_id':klass,'module_sha256':module,
 'native_sha256':digest(proxy),'file':'prebuilt/proxy.so','maximum_bridge_frames':1024}]}
files={'prebuilt/index.json':json.dumps(index).encode(),'prebuilt/proxy.so':proxy,
 'runtime/host.exe':pathlib.Path(host).read_bytes(),
 'runtime/host-source-manifest.json':pathlib.Path(source).read_bytes(),
 'tools/mf3/native_builder.py':b'# test instrumentation',
 'tools/ap8_descriptor.py':b'# test instrumentation'}
with zipfile.ZipFile(out,'w') as archive:
 for key,data in files.items():archive.writestr(key,data)
 archive.writestr('recipe.json',json.dumps({'schema':3,'files':{key:digest(data) for key,data in files.items()}}))
"#]).arg(&path).arg(&c.host.path).arg(&c.source_manifest.path)
            .arg(&c.native.artifact.path).arg(&c.selection.class.id)
            .arg(&c.selection.module.sha256).arg(successor.to_string()).status().unwrap();
        assert!(status.success());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        Artifact {sha256:digest(&path).unwrap(),path}
    }
    let old_kit = kit(&f, &c, "old", false);
    let mut software = crate::catalogue::Software {
        installer_launch: None, preparation_kit: Some(old_kit.clone()),
        manager: c.host.clone(), operator_frontend: None,
        supervisor: c.host.clone(), ownership: c.host.clone(), host: c.host.clone(),
        source_manifest: c.source_manifest.clone(),
        source_sha256: c.source_manifest.sha256.clone(), native_catalogue: None,
    };
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    f.m.select_delay(&c.selection.class.id, 1024).unwrap();
    let runtime = build::stage_runtime(&f.m).unwrap();
    let mut inspection = c.inspection.clone();
    inspection.host = runtime.host.clone();
    inspection.source_manifest = runtime.source_manifest.clone();
    let retained_candidate = prepared(c.selection.clone(), inspection, c.native.clone(),
        runtime.host, runtime.source_manifest, old_kit.sha256.clone()).unwrap();
    retain_inspection(&f.m, &retained_candidate.inspection).unwrap();
    record_candidate(&f.m, &retained_candidate).unwrap();
    let reference = replace(&f.m, &retained_candidate, &original).unwrap();
    let revision = f.m.load_revision(&c.selection.class.id, &reference).unwrap();
    assert_eq!(revision.performance.added_frames, 1024);
    let registry_before = fs::read(f.m.root.join("registry.json")).unwrap();
    software.preparation_kit = Some(kit(&f, &c, "new", true));
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    retained(&f.m, &revision).unwrap();
    assert!(catalogue_free_registry(&f.m, &f.m.registry().unwrap()).unwrap());
    f.m.select_delay(&c.selection.class.id, 512).unwrap();
    f.m.select_delay(&c.selection.class.id, 1024).unwrap();
    assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry_before);
    let trial = configuration::prepare(&f.m, &retained_candidate,
        Some(crate::operator_model::GraphicsBackend::WineD3d11), Some(&reference)).unwrap();
    let trial_ref = replace(&f.m, &trial, &reference).unwrap();
    disable_exact(&f.m, &trial, &trial_ref).unwrap();
    assert_eq!(f.m.registry().unwrap().classes[&c.selection.class.id].managed_revision, Some(reference.clone()));
    assert_eq!(f.m.performance(&c.selection.class.id).unwrap().added_frames, 1024);
    let mut foreign = revision.registration.clone();
    foreign.module.sha256 = "ff".repeat(32);
    assert_eq!(build::maximum_bridge_frames(&f.m, &foreign).unwrap(), None);
    software.preparation_kit = None;
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();
    retained(&f.m, &revision).unwrap();
    let record = f.m.root.join("software/preparation-kits")
        .join(&old_kit.sha256).join("runtime.json");
    let saved = fs::read(&record).unwrap();
    fs::remove_file(&record).unwrap();
    assert!(retained(&f.m, &revision).is_err());
    immutable(&record, &serde_json::from_slice::<build::Runtime>(&saved).unwrap()).unwrap();
    fs::set_permissions(&old_kit.path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&old_kit.path, b"changed retained recipe").unwrap();
    assert!(retained(&f.m, &revision).is_err());
}
#[test]
fn touch_carry_forward_keeps_factory_and_selected_class_without_claiming_a_new_scan() {
    let (fixture, before) = fixture();
    let mut after = before.selection.environment.clone();
    after.revision += 1;
    after.runner.id = "proton-11.0-2c-x11-touch-release-v1".into();
    after.runner.policy = Some(RunnerPolicy::X11TouchReleaseV1);
    let provenance = TouchCarryForward {
        predecessor: SERUM_TOUCH_PREDECESSOR.into(),
        transition: Artifact { path: "/unused/result.json".into(), sha256: "aa".repeat(32) },
        runner_manifest: Artifact { path: "/unused/manifest.json".into(), sha256: "bb".repeat(32) },
    };
    let next = carry_forward_touch_fields(&before, &after, provenance.clone(), Origin::X11TouchReleaseV1).unwrap();
    assert_eq!(next.selection.module, before.selection.module);
    assert_eq!(next.selection.class, before.selection.class);
    assert_eq!(next.selection.factory_report, before.selection.factory_report);
    assert_eq!(next.inspection.report, before.inspection.report);
    assert_eq!(next.native, before.native);
    assert_eq!(next.host, before.host);
    assert_eq!(next.source_manifest, before.source_manifest);
    assert_eq!(next.profile.capabilities, before.profile.capabilities);
    assert_eq!(next.profile.revision, before.profile.revision + 1);
    assert_eq!(next.profile.requirements.environment_revision, after.revision);
    assert_eq!(next.touch_carry_forward, Some(provenance));
    assert_eq!(next.origin, Origin::X11TouchReleaseV1);
    assert_eq!(next.inspection.origin, Origin::X11TouchReleaseV1);
    let inventory_path = fixture.m.root.join("inventory").join(format!("{}.json", before.selection.environment.id));
    let inventory_bytes = fs::read(&inventory_path).unwrap();
    atomic_json(&after.root.join("environment.json"), &after).unwrap();
    assert!(selections(&fixture.m, &before.selection.scanner, &before.selection.scanner_source).unwrap().is_empty());
    assert_eq!(fs::read(&inventory_path).unwrap(), inventory_bytes);
    assert_eq!(
        touch_successor(&before, &after, next.touch_carry_forward.unwrap())
            .unwrap_err().to_string(),
        "touch_carry_forward_predecessor"
    );
}
#[test]
fn touch_routing_successor_carries_only_the_exact_candidate_c_authority() {
    let (_fixture, mut predecessor) = fixture();
    let mut after = predecessor.selection.environment.clone();
    after.revision += 1;
    after.runner.id = "proton-11.0-2c-x11-touch-routing-v2".into();
    after.runner.policy = Some(RunnerPolicy::X11TouchRoutingV2);
    predecessor.origin = Origin::X11TouchReleaseV1;
    let provenance = TouchCarryForward {
        predecessor: SERUM_TOUCH_ROUTING_PREDECESSOR.into(),
        transition: Artifact { path: "/unused/result.json".into(), sha256: "aa".repeat(32) },
        runner_manifest: Artifact { path: "/unused/manifest.json".into(), sha256: "bb".repeat(32) },
    };
    // An arbitrary earlier candidate is not candidate C, even if its fields
    // resemble the physical Serum profile. The transition owner must select C.
    assert_eq!(
        touch_routing_successor(&predecessor, &after, provenance).unwrap_err().to_string(),
        "touch_routing_carry_forward_predecessor"
    );
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
        candidate_record(&f.m, &id).unwrap(),
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
fn managed_observation_requires_exact_current_retained_publication() {
    let (f, c) = fixture();
    enable(&f.m, &c, false).unwrap();
    let admitted =
        crate::ui_observation::admit_managed_observation(&f.m, &c.selection.class.id).unwrap();
    assert_eq!(
        admitted.profile_fingerprint,
        c.profile.fingerprint().unwrap()
    );
    assert_eq!(admitted.registration.module, c.selection.module);
    assert_eq!(admitted.registration.host, c.host);
    assert_eq!(
        admitted.registration.host_source_sha256,
        c.source_manifest.sha256
    );
    assert_eq!(admitted.maximum_seconds, 30);
    assert!(crate::ui_observation::admit(&f.m, &c.selection.class.id).is_err());

    fs::write(&c.inspection.report.path, b"changed retained inspection").unwrap();
    assert!(crate::ui_observation::admit_managed_observation(&f.m, &c.selection.class.id).is_err());

    for changed in 0..4 {
        let (f, c) = fixture();
        enable(&f.m, &c, false).unwrap();
        let path = match changed {
            0 => &c.selection.module.path,
            1 => &c.native.artifact.path,
            2 => &c.host.path,
            _ => &c.source_manifest.path,
        };
        fs::write(path, b"changed retained artifact").unwrap();
        assert!(crate::ui_observation::admit_managed_observation(&f.m, &c.selection.class.id).is_err());
    }

    let (f, c) = fixture();
    enable(&f.m, &c, false).unwrap();
    fs::remove_file(f.m.link(&c.selection.class.id)).unwrap();
    assert!(crate::ui_observation::admit_managed_observation(&f.m, &c.selection.class.id).is_err());
    assert!(crate::ui_observation::admit_managed_observation(&f.m, &"ff".repeat(16)).is_err());
}

#[test]
fn managed_experimental_publication_can_arm_one_exact_crash_capture() {
    let (f, c) = fixture();
    enable(&f.m, &c, false).unwrap();
    atomic_json(
        &f.m.root.join("software.json"),
        &json!({"fixture":true,"product":"managed-experimental"}),
    )
    .unwrap();

    crate::crash_capture::arm(&f.m, Some(&c.selection.class.id)).unwrap();
    let registration = f.m.registry().unwrap().classes[&c.selection.class.id]
        .registration
        .clone();
    let capture = crate::crash_capture::claim(&f.m, &registration, &"ab".repeat(16))
        .unwrap()
        .unwrap();
    let request: serde_json::Value = read_json(&capture.directory.join("request.json")).unwrap();
    assert_eq!(request["session"], "abababababababababababababababab");
    assert_eq!(request["registration"]["metadata"]["class_id"], registration.key());
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
        candidate_record(&f.m, &b.id().unwrap()).unwrap(),
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
    assert_eq!(candidate_record(&f.m, &id).unwrap(), c);
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
    // Several settings candidates share one compiled artifact. Refresh keeps
    // the selected override without mistaking the candidates for two engines.
    let trial = configuration::prepare(&f.m, &c,
        Some(crate::operator_model::GraphicsBackend::WineD3d11), None).unwrap();
    let reusable = build::reusable(&f.m, &c.selection, &i, &kit.sha256).unwrap().unwrap();
    let refreshed = configuration::carry_settings(reusable, Some(&trial)).unwrap();
    let refreshed = bind_preparation_basis(refreshed, Some("bc".repeat(32))).unwrap();
    record_candidate_with_predecessor(&f.m, &refreshed, Some(&trial.id().unwrap())).unwrap();
    assert_eq!(refreshed.profile.capabilities.graphics, trial.profile.capabilities.graphics);
    assert_eq!(build::reusable(&f.m, &c.selection, &i, &kit.sha256).unwrap(), Some(c.clone()));
    // A historical default recommendation does not select native code or
    // reinterpret its configuration when today's advice changes.
    let advised = prepared_with_advice(c.selection.clone(), i.clone(), c.native.clone(),
        c.host.clone(), c.source_manifest.clone(), kit.sha256.clone(),
        (Accessibility::DisabledForVendorProcess,
            vec!["docs/MF3.md".into(), "evidence/default-accessibility-advice.json".into()])).unwrap();
    record_candidate(&f.m, &advised).unwrap();
    assert_eq!(build::reusable(&f.m, &c.selection, &i, &kit.sha256).unwrap(), Some(c.clone()));
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
fn scoped_history_recovery_rechecks_records_preserves_lineage_and_ignores_unrelated_candidates() {
    let (f, c, binding) = legacy_fixture();
    let id = c.id().unwrap();
    let record = object(&f.m, "candidates", &id).unwrap().join("candidate.json");
    immutable(&record, &c).unwrap();
    let candidate_bytes = fs::read(&record).unwrap();
    let mut unrelated = c.clone();
    unrelated.selection.environment.id = "14".repeat(16);
    unrelated.selection.environment.root = f.m.root.join("environments").join(&unrelated.selection.environment.id);
    let unrelated_record = object(&f.m, "candidates", &unrelated.id().unwrap()).unwrap().join("candidate.json");
    immutable(&unrelated_record, &unrelated).unwrap();
    let unrelated_bytes = fs::read(&unrelated_record).unwrap();
    let line = CandidateLineage {schema:1,candidate:id.clone(),preparation_identity:"original-generation".into(),
        ordinal:17,predecessor:None};
    let lineage_path = object(&f.m, "lineage", &id).unwrap().join("record.json");
    immutable(&lineage_path, &line).unwrap();
    let lineage_bytes = fs::read(&lineage_path).unwrap();
    let recovery = history::retained_history_recovery_with_binding(&f.m, &c, &binding).unwrap().unwrap();
    assert!(!recovery.missing.iter().any(|slot| slot == "lineage"));
    let inventory_path = f.m.root.join("inventory").join(format!("{}.json", c.selection.environment.id));
    let inventory_bytes = fs::read(&inventory_path).unwrap();
    let mut inventory: Value = read_json(&inventory_path).unwrap();
    inventory["completed_at"] = json!(999);
    atomic_json(&inventory_path, &inventory).unwrap();
    assert_eq!(history::complete_candidate_history_with_binding(&f.m, &id,
        &recovery.expected_history, &binding).unwrap_err().to_string(), "candidate_history_changed");
    assert!(!object(&f.m, "legacy", &id).unwrap().exists());
    fs::write(&inventory_path, &inventory_bytes).unwrap();
    history::complete_candidate_history_with_binding(&f.m, &id, &recovery.expected_history, &binding).unwrap();
    assert!(history::retained_history_recovery_with_binding(&f.m, &c, &binding).unwrap().is_none());
    verify_legacy(&f.m, &c).unwrap();
    assert_eq!(lineage(&f.m, &c).unwrap(), line);
    assert_eq!(fs::read(&lineage_path).unwrap(), lineage_bytes);
    assert_eq!(fs::read(&record).unwrap(), candidate_bytes);
    assert_eq!(fs::read(&unrelated_record).unwrap(), unrelated_bytes);
    assert!(!object(&f.m, "legacy", &unrelated.id().unwrap()).unwrap().exists());
}
#[test]
fn scoped_history_recovery_reports_absent_source_without_materializing_history() {
    let (f, c, binding) = legacy_fixture();
    let id = c.id().unwrap();
    let record = object(&f.m, "candidates", &id).unwrap().join("candidate.json");
    immutable(&record, &c).unwrap();
    let original = fs::read(&record).unwrap();
    fs::remove_file(f.m.root.join("inventory").join(format!("{}.json", c.selection.environment.id))).unwrap();
    let recovery = history::retained_history_recovery_with_binding(&f.m, &c, &binding).unwrap().unwrap();
    assert_eq!(recovery.unavailable_reason.as_deref(), Some(
        "Saved setup history cannot be completed because original setup records are missing."));
    assert_eq!(history::complete_candidate_history_with_binding(&f.m, &id,
        &recovery.expected_history, &binding).unwrap_err().to_string(), "candidate_history_source_unavailable");
    assert_eq!(fs::read(&record).unwrap(), original);
    assert!(!object(&f.m, "legacy", &id).unwrap().exists());
    assert!(!object(&f.m, "lineage", &id).unwrap().exists());
}
#[test]
fn scoped_history_recovery_finishes_each_partial_snapshot_without_false_corruption_fallback() {
    use history::MaterializeBoundary::*;
    for boundary in [Environment, Onboarding, Inventory, ProvenanceStaged, ProvenanceInstalled] {
        let (f, c, binding) = legacy_fixture();
        let id = c.id().unwrap();
        let record = object(&f.m, "candidates", &id).unwrap().join("candidate.json");
        immutable(&record, &c).unwrap();
        let candidate_bytes = fs::read(&record).unwrap();
        assert_eq!(history::materialize_legacy_with(&f.m, &c, &binding, Some(boundary))
            .unwrap_err().to_string(), "legacy_materialization_interrupted");
        let recovery = history::retained_history_recovery_with_binding(&f.m, &c, &binding).unwrap().unwrap();
        let snapshots = snapshot(&object(&f.m, "legacy", &id).unwrap());
        history::complete_candidate_history_with_binding(&f.m, &id, &recovery.expected_history, &binding).unwrap();
        let completed = snapshot(&object(&f.m, "legacy", &id).unwrap());
        for (path, bytes) in snapshots {
            assert_eq!(completed.get(&path), Some(&bytes), "existing immutable snapshots must win");
        }
        verify_legacy(&f.m, &c).unwrap();
        assert_eq!(fs::read(&record).unwrap(), candidate_bytes);
        assert!(history::retained_history_recovery_with_binding(&f.m, &c, &binding).unwrap().is_none());
    }
    let (f, c, binding) = legacy_fixture();
    let conflict = object(&f.m, "legacy", &c.id().unwrap()).unwrap().join("environment.json");
    immutable(&conflict, &json!({"conflicting":"snapshot"})).unwrap();
    let bytes = fs::read(&conflict).unwrap();
    assert_eq!(history::retained_history_recovery_with_binding(&f.m, &c, &binding)
        .unwrap_err().to_string(), "legacy_input_binding");
    assert_eq!(fs::read(&conflict).unwrap(), bytes);
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

#[test]
fn reusable_engine_actual_preparation_retains_and_publishes_descriptor() {
    let (f, c) = fixture();
    let path = f.m.root.join("software/reusable-kit.zip");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let script = r#"import json,zipfile,hashlib,sys,pathlib
path,root,host,manifest=sys.argv[1:];root=pathlib.Path(root)
sha=lambda b:hashlib.sha256(b).hexdigest()
files={'prebuilt/engine.so':b'\x7fELFreusable-engine-fixture',
 'runtime/host.exe':pathlib.Path(host).read_bytes(),
 'runtime/host-source-manifest.json':pathlib.Path(manifest).read_bytes()}
for name in ('tools/mf3/native_builder.py','tools/ap8_descriptor.py'):files[name]=(root/name).read_bytes()
files['prebuilt/index.json']=json.dumps(dict(schema=3,engine='prebuilt/engine.so',
 engine_sha256=sha(files['prebuilt/engine.so']),descriptor_schema=1,maximum_bridge_frames=1024,native_sources={})).encode()
recipe=dict(schema=4,source_commit='ab'*20,sdk='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96',
 sdk_runtime='b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8',files={k:sha(v) for k,v in files.items()})
with zipfile.ZipFile(path,'w') as z:
 z.writestr('recipe.json',json.dumps(recipe))
 for k,v in files.items():z.writestr(k,v)
"#;
    assert!(std::process::Command::new("python3")
        .args(["-I", "-c", script])
        .arg(&path)
        .arg(root)
        .arg(&c.host.path)
        .arg(&c.source_manifest.path)
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
    let runtime = build::stage_runtime(&f.m).unwrap();
    let mut raw: Value = read_json(&c.inspection.report.path).unwrap();
    raw["records"]
        .as_array_mut()
        .unwrap()
        .push(json!({"state":"ap8_bus","media":0,
        "direction":1,"index":0,"channels":2,"type":0,"flags":1,"arrangement":3,"name":"Output"}));
    let report = f.outer.join("preparation-inspection.json");
    atomic_json(&report, &raw).unwrap();
    let inspection = inspect_record_with(
        c.selection.clone(),
        Artifact {
            sha256: digest(&report).unwrap(),
            path: report,
        },
        Origin::ManagedPreparation,
        runtime.host.clone(),
        runtime.source_manifest.clone(),
    )
    .unwrap();
    let operation = random_id().unwrap();
    let prepared = build::construct(
        &f.m,
        c.selection,
        inspection,
        runtime.host,
        runtime.source_manifest,
        &operation,
    )
    .unwrap();
    let descriptor = prepared.native.descriptor.as_ref().unwrap();
    assert_eq!(prepared.profile.claim, Claim::ReviewCandidate);
    assert_eq!(
        fs::read(&prepared.native.artifact.path).unwrap(),
        b"\x7fELFreusable-engine-fixture"
    );
    let data: lvb_plugin_descriptor::Descriptor = read_json(&descriptor.path).unwrap();
    assert_eq!(data.parameters[0].initial, 0.5);
    assert!(!data.parameters[0].available);
    verify_candidate(
        &f.m,
        &prepared,
        &prepared.selection.scanner,
        &prepared.selection.scanner_source,
    )
    .unwrap();
    retain_inspection(&f.m, &prepared.inspection).unwrap();
    record_candidate(&f.m, &prepared).unwrap();
    build::cleanup_work(&f.m, &operation).unwrap();
    descriptor.verify().unwrap();
    let revision = enable(&f.m, &prepared, false).unwrap();
    let installed =
        f.m.load_revision(&prepared.selection.class.id, &revision)
            .unwrap();
    assert_eq!(
        installed.registration.descriptor.as_ref().unwrap().sha256,
        descriptor.sha256
    );
    assert_eq!(
        fs::read_link(f.m.link(&prepared.selection.class.id)).unwrap(),
        installed.target
    );
    check_publication(&f.m, &installed.profile, &installed.registration).unwrap();
    assert!(catalogue_free_registry(&f.m, &f.m.registry().unwrap()).unwrap());
    assert_eq!(f.m.resolve(&f.identity()).unwrap(), installed.registration);
}

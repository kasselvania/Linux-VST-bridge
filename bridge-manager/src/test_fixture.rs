//! Shared existing manager/publication fixture for library and installed CLI tests.
use crate::{catalogue::*, observation::*, profiles::*, *};
pub(crate) struct Fixture {
    pub(crate) m: Manager,
    pub(crate) r: Registration,
    pub(crate) outer: PathBuf,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        unsafe {
            libc::umask(0o077);
        }
        // A real owner.sock must also fit macOS SUN_LEN. The system per-app
        // temporary directory can exceed that before the fixture suffix.
        let outer = PathBuf::from("/tmp")
            .canonicalize()
            .unwrap()
            .join(format!("lvb-manager-{}", random_id().unwrap()));
        private_dir(&outer).unwrap();
        let m = Manager {
            root: outer.join("managed"),
            publications: outer.join("vst3"),
        };
        private_dir(&m.root).unwrap();
        let env = m
            .root
            .join("environments")
            .join("11111111-1111-4111-8111-111111111111");
        private_dir(&env).unwrap();
        private_dir(&env.join("compatdata/pfx/drive_c/Program Files/Common Files/VST3")).unwrap();
        fn artifact(p: PathBuf, bytes: &[u8]) -> Artifact {
            fs::write(&p, bytes).unwrap();
            Artifact {
                sha256: digest(&p).unwrap(),
                path: p,
            }
        }
        let proton = artifact(outer.join("proton"), b"runner");
        let entry = artifact(outer.join("entry"), b"runtime");
        private_dir(&m.root.join("software")).unwrap();
        let r = Registration {
            descriptor: None,
            metadata: Metadata {
                class_id: "01".repeat(16),
                name: "Actual class".into(),
                vendor: "Vendor".into(),
                version: "1.0".into(),
                subcategories: "Instrument|Sampler".into(),
                metadata_tier: "factory_2".into(),
            },
            environment: Environment {
                id: "11111111-1111-4111-8111-111111111111".into(),
                root: env.clone(),
                revision: 1,
                runner: Runner {
                    id: "exact-runner-1".into(),
                    version: "pinned".into(),
                    proton: proton.path.clone(),
                    entry_point: entry.path.clone(),
                    files: vec![proton, entry],
                    policy: None,
                },
            },
            module: artifact(
                env.join("compatdata/pfx/drive_c/Program Files/Common Files/VST3/a.vst3"),
                b"vendor module",
            ),
            host: artifact(m.root.join("software/host"), b"host"),
            host_source_sha256: "ab".repeat(32),
            native: artifact(outer.join("native"), b"native"),
            compatibility: Compatibility::default(),
        };
        atomic_json(&env.join("environment.json"), &r.environment).unwrap();
        Self { m, r, outer }
    }
    pub(crate) fn identity(&self) -> Vec<u8> {
        (self.r.metadata.class_id.clone() + &self.r.module.sha256)
            .as_bytes()
            .chunks(2)
            .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.outer).unwrap();
    }
}
pub(crate) fn prepared() -> (Fixture, Profile, Census, NativeArtifact) {
    prepared_accessibility(true)
}
pub(crate) fn prepared_accessibility(disabled: bool) -> (Fixture, Profile, Census, NativeArtifact) {
    let mut f = Fixture::new();
    f.r.compatibility.disable_windows_accessibility = disabled;
    f.r.metadata.metadata_tier = "factory_3_unicode".into();
    let source = f.r.host.path.with_file_name("host-source-manifest.json");
    fs::write(&source, b"fixture host source").unwrap();
    f.r.host_source_sha256 = digest(&source).unwrap();
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
        claim: Claim::VerifiedExactFixture,
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
                policy: f.r.environment.runner.policy.clone(),
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
            graphics: None,
            editor_lifetime: None,
            vendor_retirement: None,
            event_output: None,
            audio_layout: None,
            accessibility: if disabled { Accessibility::DisabledForVendorProcess } else { Accessibility::WindowsDefault },
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
        module_stamp: Some(ModuleStamp::read(&f.r.module.path).unwrap()),
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
            descriptor: None,
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
pub(crate) fn inspection_report(c: &Census) -> serde_json::Value {
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

pub(crate) fn reusable_kit(path: &Path, host: &Artifact, source: &Artifact, engine: &str) {
    let script = r#"import json,zipfile,hashlib,sys,pathlib
path,root,host,manifest,engine=sys.argv[1:];root=pathlib.Path(root)
sha=lambda b:hashlib.sha256(b).hexdigest()
files={'prebuilt/engine.so':b'\x7fELF'+engine.encode(),
 'runtime/host.exe':pathlib.Path(host).read_bytes(),
 'runtime/host-source-manifest.json':pathlib.Path(manifest).read_bytes()}
helper_names=['lvb-direct-wait.dll','x86_64-windows/lvb-direct-wait.dll','x86_64-unix/lvb-direct-wait.so']
for name in helper_names:files['runtime/'+name]=name.encode()
files['runtime/direct-audio-helper.json']=json.dumps(dict(schema=1,abi=1,
 host_sha256=sha(files['runtime/host.exe']),runner_wine_revision='46b29104e3741fe23bf5e2547196a253aab88c89',
 files={name:sha(files['runtime/'+name]) for name in helper_names})).encode()
for name in ('tools/mf3/native_builder.py','tools/ap8_descriptor.py'):files[name]=(root/name).read_bytes()
files['prebuilt/index.json']=json.dumps(dict(schema=3,engine='prebuilt/engine.so',
 engine_sha256=sha(files['prebuilt/engine.so']),descriptor_schema=1,maximum_bridge_frames=1024,
 audio_completion_contract=1,loaded_engine_admission_contract=1,native_sources={})).encode()
recipe=dict(schema=4,source_commit='ab'*20,sdk='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96',
 sdk_runtime='b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8',files={k:sha(v) for k,v in files.items()})
with zipfile.ZipFile(path,'w') as z:
 z.writestr('recipe.json',json.dumps(recipe))
 for k,v in files.items():z.writestr(k,v)
"#;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    assert!(std::process::Command::new("python3").args(["-I", "-c", script])
        .arg(path).arg(root).arg(&host.path).arg(&source.path).arg(engine)
        .status().unwrap().success());
    fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
}

/// Compare every file byte, directory and symlink, including publication and
/// transaction records. Reading a candidate cannot leave hidden durable work.
pub(crate) fn snapshot(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(path: &Path, out: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>) {
        let md = fs::symlink_metadata(path).unwrap();
        let value = if md.file_type().is_symlink() {
            format!("link:{}", fs::read_link(path).unwrap().display()).into_bytes()
        } else if md.is_dir() {
            for item in fs::read_dir(path).unwrap() {
                visit(&item.unwrap().path(), out);
            }
            b"directory".to_vec()
        } else {
            fs::read(path).unwrap()
        };
        out.insert(path.to_path_buf(), value);
    }
    let mut result = std::collections::BTreeMap::new();
    visit(root, &mut result);
    result
}

pub(crate) fn prepared_candidate(id: &str) -> (Fixture, preparation::Candidate) {
    use preparation::{selections, inspect_record, Origin};
    use serde_json::json;
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
    census.module_stamp = Some(ModuleStamp::read(&census.module.path).unwrap());
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
    let c = preparation::prepared(s, i, native, f.r.host.clone(), manifest, "aa".repeat(32)).unwrap();
    (f, c)
}

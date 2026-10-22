//! Shared existing manager/publication fixture for library and installed CLI tests.
use crate::{catalogue::*, observation::*, profiles::*, *};
/// Give a scanner stub a complete source pair for the same isolated launch path.
#[allow(dead_code)] // This shared fixture is also compiled by library-only tests.
pub(crate) fn set_source_supervisor(software: &mut catalogue::Software, script: &Path) {
    let ownership=script.with_file_name(format!("{}-ownership.py",
        script.file_stem().unwrap().to_str().unwrap()));
    fs::write(&ownership,b"# first-party scanner fixture ownership module\n").unwrap();
    software.supervisor=Artifact {sha256:digest(script).unwrap(),path:script.to_path_buf()};
    software.ownership=Artifact {sha256:digest(&ownership).unwrap(),path:ownership};
}

pub(crate) struct Fixture {
    pub(crate) m: Manager,
    pub(crate) r: Registration,
    pub(crate) outer: PathBuf,
}
impl Fixture {
    pub(crate) fn new() -> Self {
        Self::with_layout("lvb-manager", |outer| Manager {
            root: outer.join("managed"),
            publications: outer.join("vst3"),
        })
    }
    /// The same fixture at the layout `Manager::installed()` resolves when
    /// HOME is `outer`, so a real manager process started with that HOME owns
    /// it. Used by the real-service tests under `tests/`.
    #[allow(dead_code)] // This shared fixture is also compiled by library-only tests.
    pub(crate) fn installed() -> Self {
        Self::with_layout("lvb-home", |home| Manager {
            root: home.join(".local/share/linux-vst-bridge/managed"),
            publications: home.join(".vst3"),
        })
    }
    fn with_layout(prefix: &str, layout: impl FnOnce(&Path) -> Manager) -> Self {
        unsafe {
            libc::umask(0o077);
        }
        // A real owner.sock must also fit macOS SUN_LEN. The system per-app
        // temporary directory can exceed that before the fixture suffix.
        let outer = PathBuf::from("/tmp")
            .canonicalize()
            .unwrap()
            .join(format!("{prefix}-{}", random_id().unwrap()));
        private_dir(&outer).unwrap();
        let m = layout(&outer);
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
    prepared_in(Fixture::new(), disabled)
}
pub(crate) fn prepared_in(mut f: Fixture, disabled: bool) -> (Fixture, Profile, Census, NativeArtifact) {
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

/// A managed-installer candidate built through the current reusable preparation
/// kit, so its native image carries the loaded-engine descriptor contract.
/// Records `software.json` with the scanner host and that kit.
#[allow(dead_code)] // This shared fixture is also compiled by library-only tests.
pub(crate) fn projection_candidate_in(f: Fixture, effect: bool) -> (Fixture, preparation::Candidate) {
    let (f, mut candidates) = projection_candidates_in(f, effect, 1);
    (f, candidates.pop().unwrap())
}
/// As `projection_candidate_in`, with `count` classes in one setup, each in
/// its own module: class `01…` in the fixture module, then `03…`, `05…`.
#[allow(dead_code)] // This shared fixture is also compiled by library-only tests.
pub(crate) fn projection_candidates_in(f: Fixture, effect: bool, count: usize)
    -> (Fixture, Vec<preparation::Candidate>) {
    use preparation::{inspect_record_with, selections, Origin};
    use serde_json::json;
    let (mut f, _, mut census, _) = prepared_in(f, false);
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
    census.module_stamp = Some(ModuleStamp::read(&census.module.path).unwrap());
    if effect {
        census.selected.subcategories = "Fx".into();
    }
    let mut modules = Vec::new();
    let mut classes = Vec::new();
    for index in 0..count {
        let mut census = census.clone();
        let controller = format!("{:02}", 2 * index + 2).repeat(16);
        if index > 0 {
            let module = f.r.module.path.with_file_name(format!("module-{index}.vst3"));
            fs::write(&module, format!("vendor module {index}")).unwrap();
            census.module = Artifact { sha256: digest(&module).unwrap(), path: module };
            census.module_stamp = Some(ModuleStamp::read(&census.module.path).unwrap());
            census.selected.class_id = format!("{:02}", 2 * index + 1).repeat(16);
            census.selected.name = format!("Actual class {}", index + 1);
            census.report.path = census.report.path.with_file_name(format!("inspection-{index}.json"));
        }
        let mut raw = inspection_report(&census);
        // Complete non-audio factory class; the production inventory validates all rows.
        let class = raw["records"][1]["classes"][0].clone();
        let mut associated = class;
        associated["raw_tuid_hex"] = json!(controller);
        associated["category_hex"] = json!(hex(b"Component Controller Class"));
        raw["records"][1]["classes"][1] = associated;
        raw["records"].as_array_mut().unwrap().push(
            json!({"state":"ap8_controller_association","combined":false,"class_id":controller}),
        );
        if effect {
            raw["records"].as_array_mut().unwrap().push(json!({"state":"ap8_bus",
                "media":0,"direction":0,"index":0,"channels":2,"type":0,"flags":1,
                "arrangement":3,"name":"Input"}));
        }
        raw["records"].as_array_mut().unwrap().push(json!({"state":"ap8_bus",
            "media":0,"direction":1,"index":0,"channels":2,"type":0,"flags":1,
            "arrangement":3,"name":"Output"}));
        atomic_json(&census.report.path, &raw).unwrap();
        census.report.sha256 = digest(&census.report.path).unwrap();
        modules.push(crate::inventory::Module {
            artifact: census.module.clone(),
            classes: crate::inventory::classes(&raw).unwrap(),
            report: census.report.clone(),
            inspection_error: None,
            quarantine_reason: None,
        });
        classes.push((census.selected.class_id.clone(), census.report));
    }
    let scan = crate::inventory::Scan {
        schema: 1,
        id: random_id().unwrap(),
        environment: f.r.environment.clone(),
        host: f.r.host.clone(),
        host_source_sha256: f.r.host_source_sha256.clone(),
        completed_at: observation::now().unwrap(),
        modules,
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
    let mut offered = selections(&f.m, &f.r.host, &f.r.host_source_sha256).unwrap();
    assert_eq!(offered.len(), count);
    let manifest = Artifact {
        path: f.r.host.path.with_file_name("host-source-manifest.json"),
        sha256: f.r.host_source_sha256.clone(),
    };
    let sw = projection_kit_software(&f, &manifest);
    let runtime = preparation::build::stage_runtime_for_software(&f.m, &sw).unwrap();
    let candidates = classes.into_iter().map(|(class, report)| {
        let index = offered.iter().position(|s| s.class.id == class).unwrap();
        let s = offered.remove(index);
        let i = inspect_record_with(s.clone(), report, Origin::ManagedPreparation,
            runtime.host.clone(), runtime.source_manifest.clone()).unwrap();
        preparation::build::construct_with_runtime(&f.m, s, i, runtime.clone(),
            &random_id().unwrap(), None).unwrap()
    }).collect();
    atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
    (f, candidates)
}
/// How the setups of a retained history were changed.
#[allow(dead_code)] // This shared fixture is also compiled by library-only tests.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Changes {
    /// Installer runs in each setup, after the installation that created it.
    /// Their units are asked about; the caller reports them closed.
    Installers,
    /// Runtime changes back and forth, in setups whose installer never ran.
    /// Nothing about them is asked of systemd.
    Runtimes,
}
/// What a long-used installation retains, in the shape of the maintainer's
/// Deck (#504): `setups` setups created from one imported installer, each
/// changed `edges` times, so every record bound when a setup was created
/// reaches its current revision through the whole chain; and `finished`
/// other finished operations. Records only: no prefix contents. Returns the
/// installer operations whose units are asked about.
#[allow(dead_code)] // This shared fixture is also compiled by library-only tests.
pub(crate) fn retained_history(f: &Fixture, changes: Changes, setups: usize, edges: u64,
    finished: usize) -> Vec<String> {
    use serde_json::json;
    let m = &f.m;
    let installers = m.root.join("installers");
    private_dir(&installers).unwrap();
    let image = installers.join("retained.exe");
    fs::write(&image, b"MZ retained history installer").unwrap();
    let installer = digest(&image).unwrap();
    let image = {
        let named = installers.join(format!("{installer}.exe"));
        fs::rename(&image, &named).unwrap();
        fs::set_permissions(&named, fs::Permissions::from_mode(0o400)).unwrap();
        named
    };
    atomic_json(&installers.join(format!("{installer}.json")), &json!({"schema":1,"id":installer,
        "artifact":{"path":image,"sha256":installer},"byte_size":29,"format":"pe_executable",
        "name_hint":"Retained history installer","source_class":"operator_selected_file",
        "import_result":"imported","created_at":1,"vendor":null,"product":null})).unwrap();
    // A second runtime to change to and back.
    let runners = {
        let first = f.r.environment.runner.clone();
        let pinned = |name: &str| {
            let path = f.outer.join(name);
            fs::write(&path, name.as_bytes()).unwrap();
            Artifact { sha256: digest(&path).unwrap(), path }
        };
        let (proton, entry) = (pinned("retained-proton"), pinned("retained-entry"));
        [first, Runner { id: "exact-runner-2".into(), version: "pinned".into(),
            proton: proton.path.clone(), entry_point: entry.path.clone(), files: vec![proton, entry],
            policy: None }]
    };
    let closed = |operation: &str| json!({"schema":2,"operation":operation,"state":"completed",
        "raw_exit":0,"cleanup_confirmed":true,"owned_live":0,"error":null});
    let job = |operation: &str, request: operator_model::Action| {
        let directory = m.root.join("operator").join(operation);
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("request.json"), &operator_model::Request {
            schema: operator_model::OPERATOR_SCHEMA, state_token: "retained".into(), action: request }).unwrap();
        atomic_json(&directory.join("result.json"), &json!({"schema":1,"operation":operation,
            "state":"completed","result":{}})).unwrap();
        directory
    };
    let mut units = Vec::new();
    for setup in 0..setups {
        let id = format!("{:032x}", 0xa11_0000 + setup);
        let root = m.root.join("environments").join(&id);
        private_dir(&root.join("compatdata/pfx/drive_c")).unwrap();
        let at = |revision: u64| Environment { id: id.clone(), root: root.clone(), revision,
            runner: runners[if changes == Changes::Runtimes { (revision as usize + 1) % 2 } else { 0 }].clone() };
        let onboarding = m.root.join("onboarding").join(&id);
        private_dir(&onboarding).unwrap();
        let installed = (changes == Changes::Installers).then(|| random_id().unwrap());
        if let Some(installed) = &installed {
            atomic_json(&onboarding.join(format!("{installed}-result.json")), &closed(installed)).unwrap();
            units.push(installed.clone());
        }
        let record = onboarding.join("record.json");
        atomic_json(&record, &json!({"schema":1,"id":id,"installer":installer,"environment":at(1),
            "created_at":1 + setup,"creation_operation":random_id().unwrap(),
            "installation_operation":installed,"published":false})).unwrap();
        for revision in 1..=edges {
            let operation = random_id().unwrap();
            let (before, environment) = (at(revision), at(revision + 1));
            if changes == Changes::Installers {
                let directory = job(&operation, operator_model::Action::EnvironmentInstallerStart {
                    environment: id.clone(), installer: installer.clone() });
                atomic_json(&directory.join("installation.json"), &environment_revision::InstallerBinding {
                    schema: 1, operation: operation.clone(), installer: installer.clone(), before, environment,
                    created_at: 1000 * (setup as u64 + 1) + revision, preserve_existing_prefix: Some(true),
                    automatic_request: None }).unwrap();
                atomic_json(&directory.join("installation-result.json"), &closed(&operation)).unwrap();
                units.push(operation);
            } else {
                let directory = job(&operation, operator_model::Action::EnvironmentRuntimeSelect {
                    environment: id.clone(), runner: catalogue::runner_key(&environment.runner).unwrap(),
                    expected_environment: environment_revision::identity(&before).unwrap() });
                atomic_json(&directory.join("runtime-transition.json"), &environment_revision::RuntimeBinding {
                    schema: 1, operation, before, environment,
                    affected: vec![environment_revision::RuntimeOwner::Setup {
                        record: Artifact { sha256: digest(&record).unwrap(), path: record.clone() },
                        environment: at(1) }],
                    state: environment_revision::RuntimeState::Scanned }).unwrap();
            }
        }
        atomic_json(&root.join("environment.json"), &at(edges + 1)).unwrap();
    }
    for _ in 0..finished {
        job(&random_id().unwrap(), operator_model::Action::BufferingSet { class_id: f.r.key(),
            added_frames: 256 });
    }
    units
}

/// The scanner host with a reusable preparation kit declaring the current
/// loaded-engine admission and audio completion contracts.
fn projection_kit_software(f: &Fixture, manifest: &Artifact) -> Software {
    let path = f.m.root.join("software/ui2-projection-kit.zip");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap().to_path_buf();
    let script = r#"import json,zipfile,hashlib,sys,pathlib
path,root,host,manifest=sys.argv[1:];root=pathlib.Path(root)
sha=lambda b:hashlib.sha256(b).hexdigest()
files={'prebuilt/engine.so':b'\x7fELFprojection-reusable-engine',
 'runtime/host.exe':pathlib.Path(host).read_bytes(),
 'runtime/host-source-manifest.json':pathlib.Path(manifest).read_bytes()}
for name in ('tools/mf3/native_builder.py','tools/ap8_descriptor.py'):
 files[name]=(root/name).read_bytes()
files['prebuilt/index.json']=json.dumps(dict(schema=3,engine='prebuilt/engine.so',
 engine_sha256=sha(files['prebuilt/engine.so']),descriptor_schema=1,maximum_bridge_frames=1024,
 audio_completion_contract=1,loaded_engine_admission_contract=1,native_sources={})).encode()
recipe=dict(schema=4,source_commit='ab'*20,sdk='3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96',
 sdk_runtime='b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8',
 files={k:sha(v) for k,v in files.items()})
with zipfile.ZipFile(path,'w') as z:
 z.writestr('recipe.json',json.dumps(recipe))
 for name,data in files.items():z.writestr(name,data)
"#;
    assert!(std::process::Command::new("python3")
        .args(["-I", "-c", script]).arg(&path).arg(root)
        .arg(&f.r.host.path).arg(&manifest.path)
        .status().unwrap().success());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    let host = f.r.host.clone();
    Software {
        installer_launch: None,
        preparation_kit: Some(Artifact { sha256: digest(&path).unwrap(), path }),
        manager: host.clone(),
        operator_frontend: None,
        supervisor: host.clone(),
        ownership: host.clone(),
        host,
        source_manifest: manifest.clone(),
        source_sha256: manifest.sha256.clone(),
        native_catalogue: None,
    }
}

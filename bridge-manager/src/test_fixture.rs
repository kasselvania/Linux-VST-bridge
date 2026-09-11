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
        let outer = std::env::temp_dir()
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
    let mut f = Fixture::new();
    f.r.compatibility.disable_windows_accessibility = true;
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
            event_output: None,
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

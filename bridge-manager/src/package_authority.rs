//! PKG0: fixed installed-package intake into immutable user-owned software.
//! Package-manager ownership authenticates /usr inputs; the release signature
//! is checked by the package/release tooling before these files are installed.
use super::*;
use std::os::unix::fs::MetadataExt;

const PACKAGE_ROOT: &str = "/usr";
const NAMES: [&str; 6] = [
    "linux-vst-bridge", "linux-audio-compatibility-manager", "session.pyc",
    "ownership.pyc", "host.exe", "host-source-manifest.json",
];

#[derive(Clone)]
struct Inputs { root: PathBuf }
impl Inputs {
    fn system() -> Self { Self { root: PathBuf::from(PACKAGE_ROOT) } }
    #[cfg(test)]
    fn under(root: &Path) -> Self { Self { root: root.to_path_buf() } }
    fn manifest(&self) -> PathBuf {
        self.root.join("share/linux-vst-bridge/pkg0-manifest.json")
    }
    fn path(&self, name: &str) -> Result<PathBuf> {
        Ok(self.root.join(match name {
            "linux-vst-bridge" => "bin/linux-vst-bridge",
            "linux-audio-compatibility-manager" => "bin/linux-audio-compatibility-manager",
            "session.pyc" => "lib/linux-vst-bridge/supervisor/session.pyc",
            "ownership.pyc" => "lib/linux-vst-bridge/supervisor/ownership.pyc",
            "host.exe" => "lib/linux-vst-bridge/host/bridge-host.exe",
            "host-source-manifest.json" => "lib/linux-vst-bridge/host/source-manifest.json",
            _ => return Err("package_artifact_name".into()),
        }))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PackageFile { name: String, sha256: String, size: u64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExternalRuntime { id: String, manifest_sha256: String }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PackageManifest {
    schema: u32,
    source_head: String,
    source_tree: String,
    operator_schema: u32,
    files: Vec<PackageFile>,
    external_runtime: ExternalRuntime,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Generation {
    schema: u32,
    manifest_sha256: String,
    manifest: PackageManifest,
    predecessor: Option<Software>,
    retained_installer_launch: Option<Artifact>,
    retained_preparation_kit: Option<Artifact>,
    catalogue_sha256: Option<String>,
}

fn checked_source(path: &Path, owner: u32, max: u64) -> Result<(String, u64)> {
    let mut source = fs::OpenOptions::new().read(true)
        .custom_flags(libc::O_NOFOLLOW).open(path)?;
    let metadata = source.metadata()?;
    require(path.is_absolute() && metadata.is_file() && metadata.uid() == owner
        && metadata.mode() & 0o022 == 0 && metadata.len() <= max,
        "package_artifact_owner_or_extent")?;
    let mut hash = sha2::Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let size = source.read(&mut buffer)?;
        if size == 0 { break; }
        hash.update(&buffer[..size]);
    }
    Ok((hex(&hash.finalize()), metadata.len()))
}
fn read_manifest(inputs: &Inputs, owner: u32) -> Result<(PackageManifest, String)> {
    let path = inputs.manifest();
    let (sha, _) = checked_source(&path, owner, 64 * 1024)?;
    let manifest: PackageManifest = read_json(&path)?;
    require(digest(&path)? == sha, "package_manifest_changed")?;
    require(manifest.schema == 1 && manifest.operator_schema == operator_model::OPERATOR_SCHEMA
        && valid_hex(&manifest.source_head, 40) && valid_hex(&manifest.source_tree, 40)
        && !manifest.external_runtime.id.is_empty()
        && manifest.external_runtime.id.len() <= 128
        && manifest.external_runtime.id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        && valid_hex(&manifest.external_runtime.manifest_sha256, 64)
        && manifest.files.len() == NAMES.len(), "package_manifest_schema")?;
    for (entry, name) in manifest.files.iter().zip(NAMES) {
        require(entry.name == name && valid_hex(&entry.sha256, 64)
            && entry.size <= 128 * 1024 * 1024,
            "package_manifest_roster")?;
        let (actual, size) = checked_source(&inputs.path(name)?, owner, 128 * 1024 * 1024)?;
        require(actual == entry.sha256 && size == entry.size,
            "package_artifact_changed")?;
    }
    Ok((manifest, sha))
}
fn id(record: &Generation) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(record)?)))
}
fn artifact(dir: &Path, name: &str) -> Result<Artifact> {
    let path = dir.join(name);
    Ok(Artifact { sha256: digest(&path)?, path })
}
fn verify_software_identity(m: &Manager, old: &Software) -> Result<()> {
    require(old.operator_frontend.as_ref().is_some_and(|a|
        a.path.parent() == old.manager.path.parent()), "package_prior_pairing")?;
    for artifact in [&old.manager, &old.supervisor, &old.ownership,
        &old.host, &old.source_manifest].into_iter()
        .chain([&old.operator_frontend, &old.installer_launch,
            &old.preparation_kit, &old.native_catalogue].into_iter().flatten()) {
        artifact.verify()?;
        require(artifact.path.starts_with(m.root.join("software"))
            && artifact.path.canonicalize()? == artifact.path
            && fs::metadata(&artifact.path)?.permissions().mode() & 0o222 == 0,
            "package_prior_software_identity")?;
    }
    if old.native_catalogue.is_some() { old.catalogue(m)?; }
    Ok(())
}
fn old_software(m: &Manager) -> Result<Option<Software>> {
    let path = m.root.join("software.json");
    if !path.try_exists()? { return Ok(None); }
    let old = software(m)?;
    verify_software_identity(m, &old)?;
    Ok(Some(old))
}
fn verify_generation(m: &Manager, current: &Software) -> Result<Generation> {
    let dir = current.manager.path.parent().ok_or("package_generation_path")?;
    require(dir.parent() == Some(m.root.join("software").as_path()),
        "package_generation_path")?;
    private_dir(dir)?;
    require(fs::metadata(dir.join("package-generation.json"))?.permissions().mode() & 0o222 == 0,
        "package_generation_record_writable")?;
    let record: Generation = read_json(&dir.join("package-generation.json"))?;
    require(record.schema == 1 && id(&record)? == dir.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "package_generation_identity")?;
    require(record.manifest.schema == 1 && record.manifest.files.len() == NAMES.len()
        && record.manifest.files.iter().zip(NAMES).all(|(entry, name)| entry.name == name
            && valid_hex(&entry.sha256, 64)), "package_generation_manifest")?;
    let expected = [
        (&current.manager, NAMES[0]),
        (current.operator_frontend.as_ref().ok_or("package_frontend_absent")?, NAMES[1]),
        (&current.supervisor, NAMES[2]), (&current.ownership, NAMES[3]),
        (&current.host, NAMES[4]), (&current.source_manifest, NAMES[5]),
    ];
    for (index, (actual, name)) in expected.iter().enumerate() {
        require(actual.path == dir.join(name)
            && actual.sha256 == record.manifest.files[index].sha256,
            "package_generation_artifact_binding")?;
        actual.verify()?;
    }
    require(current.source_sha256 == current.source_manifest.sha256
        && current.installer_launch == record.retained_installer_launch
        && current.preparation_kit == record.retained_preparation_kit
        && current.native_catalogue.as_ref().map(|a| a.sha256.as_str())
            == record.catalogue_sha256.as_deref(),
        "package_generation_software_binding")?;
    if let Some(a) = &current.native_catalogue {
        require(a.path == dir.join("native-catalogue.json"),
            "package_generation_catalogue_binding")?;
        a.verify()?;
    }
    Ok(record)
}
fn stage(m: &Manager, inputs: &Inputs, manifest: PackageManifest,
    manifest_sha256: String, predecessor: Option<Software>) -> Result<Software> {
    let catalogue = predecessor.as_ref().and_then(|s| s.native_catalogue.clone());
    if let Some(a) = &catalogue { a.verify()?; }
    let record = Generation {
        schema: 1, manifest_sha256, manifest,
        retained_installer_launch: predecessor.as_ref().and_then(|s| s.installer_launch.clone()),
        retained_preparation_kit: predecessor.as_ref().and_then(|s| s.preparation_kit.clone()),
        catalogue_sha256: catalogue.as_ref().map(|a| a.sha256.clone()),
        predecessor,
    };
    let generation = id(&record)?;
    let base = m.root.join("software");
    let dest = base.join(&generation);
    catalogue::verify_host_path(&dest.join("host.exe"))?;
    private_dir(&base)?;
    if dest.try_exists()? {
        private_dir(&dest)?;
        let saved: Generation = read_json(&dest.join("package-generation.json"))?;
        require(id(&saved)? == generation && serde_json::to_vec(&saved)? == serde_json::to_vec(&record)?,
            "package_generation_collision")?;
    } else {
        let scratch = base.join(format!("stage-{}", random_id()?));
        private_dir(&scratch)?;
        let result = (|| -> Result<()> {
            for file in &record.manifest.files {
                let source = inputs.path(&file.name)?;
                let target = scratch.join(&file.name);
                fs::copy(&source, &target)?;
                require(digest(&target)? == file.sha256, "package_copy_changed")?;
                fs::set_permissions(&target, fs::Permissions::from_mode(
                    if matches!(file.name.as_str(), "linux-vst-bridge" | "linux-audio-compatibility-manager") {0o500} else {0o400}))?;
                fs::File::open(target)?.sync_all()?;
            }
            if let Some(a) = &catalogue {
                let target = scratch.join("native-catalogue.json");
                fs::copy(&a.path, &target)?;
                require(digest(&target)? == a.sha256, "package_catalogue_copy_changed")?;
                fs::set_permissions(&target, fs::Permissions::from_mode(0o400))?;
                fs::File::open(target)?.sync_all()?;
            }
            atomic_json(&scratch.join("package-generation.json"), &record)?;
            fs::set_permissions(scratch.join("package-generation.json"),
                fs::Permissions::from_mode(0o400))?;
            fs::File::open(scratch.join("package-generation.json"))?.sync_all()?;
            fs::File::open(&scratch)?.sync_all()?;
            fs::rename(&scratch, &dest)?;
            fs::File::open(&base)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() && scratch.exists() { fs::remove_dir_all(&scratch)?; }
        result?;
    }
    for file in &record.manifest.files {
        let path = dest.join(&file.name);
        require(digest(&path)? == file.sha256
            && fs::metadata(&path)?.permissions().mode() & 0o222 == 0,
            "package_generation_changed")?;
    }
    require(fs::metadata(dest.join("package-generation.json"))?.permissions().mode() & 0o222 == 0,
        "package_generation_record_writable")?;
    let native_catalogue = if let Some(expected) = &record.catalogue_sha256 {
        let a = artifact(&dest, "native-catalogue.json")?;
        require(&a.sha256 == expected, "package_generation_catalogue_changed")?;
        Some(a)
    } else { None };
    let result = Software {
        installer_launch: record.retained_installer_launch,
        preparation_kit: record.retained_preparation_kit,
        operator_frontend: Some(artifact(&dest, "linux-audio-compatibility-manager")?),
        manager: artifact(&dest, "linux-vst-bridge")?,
        supervisor: artifact(&dest, "session.pyc")?,
        ownership: artifact(&dest, "ownership.pyc")?,
        host: artifact(&dest, "host.exe")?,
        source_manifest: artifact(&dest, "host-source-manifest.json")?,
        source_sha256: digest(&dest.join("host-source-manifest.json"))?,
        native_catalogue,
    };
    if result.native_catalogue.is_some() { result.catalogue(m)?; }
    Ok(result)
}
fn preflight(m: &Manager) -> Result<()> {
    require(!reconcile_leases(m)?, "package_cleanup_unconfirmed")?;
    m.require_inactive(None)?;
    require(capacity::owners(m)?.is_empty(), "package_owner_active")?;
    require(operator_cli::pending_transactions(m)? == 0,
        "package_transaction_pending")?;
    require(onboarding::all_retired(m)?, "package_installer_owner_active")?;
    require(operator_cli::vendor_retired(m)?, "package_vendor_owner_active")?;
    daw_workspace::package_idle(m)?;
    let transports = transport_storage::root();
    if transports.try_exists()? {
        require(fs::read_dir(&transports)?.next().is_none(), "package_stale_transport")?;
    }
    Ok(())
}
fn with_locks<T>(m: &Manager, work: impl FnOnce() -> Result<T>) -> Result<T> {
    let _service = m.lock("service.lock")?;
    let _setup = m.lock("setup.lock")?;
    let _registry = m.lock("registry.lock")?;
    work()
}
fn adopt_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32) -> Result<()> {
    with_locks(m, || {
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m)?;
        let (manifest, sha) = read_manifest(inputs, owner)?;
        let predecessor = old_software(m)?;
        if let Some(old) = &predecessor {
            if old.native_catalogue.is_some() || !m.registry()?.classes.is_empty() {
                let host = &manifest.files[4];
                let source = &manifest.files[5];
                require(old.host.sha256 == host.sha256
                    && old.source_manifest.sha256 == source.sha256,
                    "package_existing_product_host_pair_changed")?;
            }
        }
        if let Some(old) = &predecessor {
            if old.manager.path.parent()
                .is_some_and(|dir| dir.join("package-generation.json").exists()) {
                let old_record = verify_generation(m, old)?;
                if old_record.manifest_sha256 == sha {
                    require(old_record.manifest == manifest, "package_manifest_changed")?;
                    return Ok(());
                }
            }
        }
        let installed = stage(m, inputs, manifest, sha, predecessor.clone())?;
        setup_install::commit_journaled(m, home, &installed, predecessor.as_ref())
    })
}
fn rollback_from(m: &Manager, home: &Path) -> Result<()> {
    with_locks(m, || {
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m)?;
        let current = old_software(m)?.ok_or("package_not_installed")?;
        let record = verify_generation(m, &current)?;
        let old = record.predecessor.ok_or("package_no_predecessor")?;
        verify_software_identity(m, &old)?;
        setup_install::commit_journaled(m, home, &old, Some(&current))
    })
}
fn recover_from(m: &Manager, home: &Path) -> Result<bool> {
    with_locks(m, || {
        preflight(m)?;
        setup_install::recover_journaled(m, home)
    })
}
fn require_service_stopped() -> Result<()> {
    let state = Command::new("systemctl").args([
        "--user", "show", "--property=ActiveState", "--value",
        "linux-vst-bridge.service",
    ]).output()?;
    require(state.status.success()
        && matches!(std::str::from_utf8(&state.stdout)?.trim(), "inactive" | "failed"),
        "package_stop_service_first")
}
pub(super) fn adopt(m: &Manager) -> Result<()> {
    require_service_stopped()?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    adopt_from(m, &home, &Inputs::system(), 0)
}
pub(super) fn rollback(m: &Manager) -> Result<()> {
    require_service_stopped()?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    rollback_from(m, &home)
}
pub(super) fn recover(m: &Manager) -> Result<bool> {
    require_service_stopped()?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    recover_from(m, &home)
}

#[cfg(test)]
mod tests {
    use super::*;
    use linux_vst_bridge::catalogue::{Catalogue, NativeArtifact};
    struct Fixture {
        base: test_fixture::Fixture,
        home: PathBuf,
        inputs: Inputs,
        owner: u32,
    }
    impl Fixture {
        fn new() -> Self {
            let base = test_fixture::Fixture::new();
            let home = base.outer.join("home");
            let inputs = Inputs::under(&base.outer.join("package/usr"));
            for name in NAMES {
                let path = inputs.path(name).unwrap();
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, match name {
                    "host.exe" => b"host".as_slice(),
                    "host-source-manifest.json" => b"fixture host source".as_slice(),
                    _ => name.as_bytes(),
                }).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
            }
            let result = Self { base, home, inputs, owner: unsafe { libc::getuid() } };
            result.write_manifest();
            result
        }
        fn manifest(&self) -> PackageManifest {
            PackageManifest {
                schema: 1, source_head: "ab".repeat(20), source_tree: "cd".repeat(20),
                operator_schema: operator_model::OPERATOR_SCHEMA,
                files: NAMES.iter().map(|name| {
                    let path = self.inputs.path(name).unwrap();
                    PackageFile { name: (*name).into(), sha256: digest(&path).unwrap(),
                        size: fs::metadata(path).unwrap().len() }
                }).collect(),
                external_runtime: ExternalRuntime { id: "exact-proton-slr".into(),
                    manifest_sha256: "ef".repeat(32) },
            }
        }
        fn write_manifest(&self) {
            let path = self.inputs.manifest();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            atomic_json(&path, &self.manifest()).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();
        }
        fn replace(&self, name: &str, data: &[u8], refresh_manifest: bool) {
            let path = self.inputs.path(name).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            fs::write(&path, data).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
            if refresh_manifest { self.write_manifest(); }
        }
        fn current(&self) -> Software { software(&self.base.m).unwrap() }
        fn adopt(&self) -> Result<()> {
            adopt_from(&self.base.m, &self.home, &self.inputs, self.owner)
        }
        fn setup_legacy_with_catalogue(&self, native: NativeArtifact) {
            let m = &self.base.m;
            let dir = m.root.join("software/legacy");
            private_dir(&dir).unwrap();
            for name in NAMES {
                let path = dir.join(name);
                fs::copy(self.inputs.path(name).unwrap(), &path).unwrap();
                fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();
            }
            let catalog = Catalogue { schema: 3, natives: vec![native],
                environments: vec![linux_vst_bridge::catalogue::EnvironmentBinding {
                    family: linux_vst_bridge::profiles::Family::ArturiaPersistentV1,
                    environment: self.base.r.environment.clone(),
                }], hosts: vec![], onboarding_runtime: None };
            atomic_json(&dir.join("native-catalogue.json"), &catalog).unwrap();
            fs::set_permissions(dir.join("native-catalogue.json"), fs::Permissions::from_mode(0o444)).unwrap();
            let old = Software {
                installer_launch: None, preparation_kit: None,
                operator_frontend: Some(artifact(&dir, NAMES[1]).unwrap()),
                manager: artifact(&dir, NAMES[0]).unwrap(),
                supervisor: artifact(&dir, NAMES[2]).unwrap(),
                ownership: artifact(&dir, NAMES[3]).unwrap(),
                host: artifact(&dir, NAMES[4]).unwrap(),
                source_manifest: artifact(&dir, NAMES[5]).unwrap(),
                source_sha256: digest(&dir.join(NAMES[5])).unwrap(),
                native_catalogue: Some(artifact(&dir, "native-catalogue.json").unwrap()),
            };
            old.catalogue(m).unwrap();
            setup_install::commit(m, &self.home, &old, None, None).unwrap();
        }
    }

    #[test]
    fn fresh_adoption_pairing_and_exact_rollback() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let first = f.current();
        assert_eq!(first.manager.path.parent(), first.operator_frontend.as_ref().and_then(|a| a.path.parent()));
        let original = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.adopt().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), original);
        f.replace("linux-vst-bridge", b"successor manager", true);
        f.replace("linux-audio-compatibility-manager", b"successor frontend", true);
        f.adopt().unwrap();
        let second = f.current();
        assert_ne!(first.manager.sha256, second.manager.sha256);
        first.manager.verify().unwrap();
        rollback_from(&f.base.m, &f.home).unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), original);
        assert_eq!(fs::read_link(f.home.join(".local/bin/linux-vst-bridge")).unwrap(), first.manager.path);
        assert_eq!(fs::read_link(f.home.join(".local/bin/linux-audio-compatibility-manager")).unwrap(), first.operator_frontend.unwrap().path);
        second.manager.verify().unwrap();
    }

    #[test]
    fn package_manifest_artifact_and_foreign_generation_refuse() {
        let f = Fixture::new();
        f.replace("linux-vst-bridge", b"tampered", false);
        assert!(f.adopt().is_err());
        assert!(!f.base.m.root.join("software.json").exists());
        f.write_manifest();
        let path = f.inputs.manifest();
        let mut manifest = f.manifest();
        manifest.operator_schema -= 1;
        atomic_json(&path, &manifest).unwrap();
        assert!(f.adopt().is_err());
        f.write_manifest();
        f.adopt().unwrap();
        let first = f.current();
        let record = first.manager.path.parent().unwrap().join("package-generation.json");
        let before = fs::read(&record).unwrap();
        fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&record, b"foreign").unwrap();
        assert!(f.adopt().is_err());
        fs::write(&record, before).unwrap();
        fs::set_permissions(&record, fs::Permissions::from_mode(0o400)).unwrap();
        f.replace("linux-audio-compatibility-manager", b"changed without manifest", false);
        assert!(f.adopt().is_err());
    }

    #[test]
    fn pending_and_cleanup_uncertainty_refuse_without_selection_change() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.replace("linux-vst-bridge", b"new manager", true);
        let pending = f.base.m.root.join("transactions/a.pending.json");
        fs::create_dir_all(pending.parent().unwrap()).unwrap();
        fs::write(&pending, b"{}").unwrap();
        assert!(f.adopt().is_err());
        fs::remove_file(pending).unwrap();
        let lease = f.base.m.root.join("runtime/leases/").join("ab".repeat(16) + ".json");
        fs::write(&lease, b"\"/tmp/unknown-report.json\"").unwrap();
        assert!(f.adopt().is_err());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
    }

    #[test]
    fn live_owner_refuses_package_update() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.replace("linux-vst-bridge", b"new manager", true);
        let session = "ab".repeat(16);
        let results = f.base.m.root.join("runtime/results");
        private_dir(&results).unwrap();
        let report = results.join(format!("{session}.json"));
        fs::write(&report, b"{\"ready\":true,\"cleanup_confirmed\":false}").unwrap();
        atomic_json(&f.base.m.root.join("runtime/leases").join(format!("{session}.json")), &report).unwrap();
        let owner = f.base.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&session).join("owner.json");
        private_dir(owner.parent().unwrap()).unwrap();
        atomic_json(&owner, &serde_json::json!({"session":session,"report":report})).unwrap();
        assert!(f.adopt().is_err());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
    }

    #[test]
    fn package_removal_and_reinstall_reuse_retained_user_generation() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let retained = fs::read(f.base.m.root.join("software.json")).unwrap();
        let user = f.base.m.root.join("projects/retained.flp");
        fs::create_dir_all(user.parent().unwrap()).unwrap();
        fs::write(&user, b"private user state").unwrap();
        let removed = f.base.outer.join("removed-package");
        fs::rename(&f.inputs.root, &removed).unwrap();
        assert!(f.adopt().is_err());
        f.current().manager.verify().unwrap();
        assert_eq!(fs::read(&user).unwrap(), b"private user state");
        fs::rename(removed, &f.inputs.root).unwrap();
        f.adopt().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), retained);
        assert_eq!(fs::read(user).unwrap(), b"private user state");
    }

    #[test]
    fn populated_catalogue_and_user_records_survive_update_and_rollback() {
        let (base, _, _, native) = test_fixture::prepared();
        let f = Fixture { home: base.outer.join("home"),
            inputs: Inputs::under(&base.outer.join("package/usr")),
            owner: unsafe { libc::getuid() }, base };
        for name in NAMES {
            let path = f.inputs.path(name).unwrap();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, match name {
                "host.exe" => b"host".as_slice(),
                "host-source-manifest.json" => b"fixture host source".as_slice(),
                _ => name.as_bytes(),
            }).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();
        }
        f.write_manifest();
        f.setup_legacy_with_catalogue(native);
        let old = f.current();
        let catalogue_bytes = fs::read(old.native_catalogue.as_ref().unwrap().path.clone()).unwrap();
        let registry_bytes = fs::read(f.base.m.root.join("registry.json")).unwrap();
        let mut retained = Vec::new();
        for name in ["daw-workspaces/fl/workspace.json", "installers/a.json",
            "compatibility/history/a.json", "vendor/app/a.json", "projects/a.flp"] {
            let path = f.base.m.root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, name.as_bytes()).unwrap();
            retained.push((path, name.as_bytes().to_vec()));
        }
        f.adopt().unwrap();
        let first = f.current();
        assert_eq!(fs::read(first.native_catalogue.as_ref().unwrap().path.clone()).unwrap(), catalogue_bytes);
        f.replace("linux-vst-bridge", b"next manager", true);
        f.replace("linux-audio-compatibility-manager", b"next frontend", true);
        f.adopt().unwrap();
        let next = f.current();
        assert_eq!(fs::read(next.native_catalogue.as_ref().unwrap().path.clone()).unwrap(), catalogue_bytes);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
        for (path, bytes) in &retained { assert_eq!(&fs::read(path).unwrap(), bytes); }
        let selected = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.replace("host.exe", b"different host cannot inherit existing catalogue", true);
        assert!(f.adopt().is_err());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected);
        rollback_from(&f.base.m, &f.home).unwrap();
        assert_eq!(f.current().manager.sha256, first.manager.sha256);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
        for (path, bytes) in &retained { assert_eq!(&fs::read(path).unwrap(), bytes); }
        old.native_catalogue.unwrap().verify().unwrap();
    }

    #[test]
    fn interrupted_switch_recovers_predecessor_or_completed_pair() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let first = f.current();
        f.replace("linux-vst-bridge", b"next manager", true);
        f.replace("linux-audio-compatibility-manager", b"next frontend", true);
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        let next = stage(&f.base.m, &f.inputs, manifest, sha, Some(first.clone())).unwrap();
        setup_install::interrupt_journaled_for_test(&f.base.m, &f.home, &next, Some(&first), false).unwrap();
        assert!(f.adopt().is_err());
        assert!(recover_from(&f.base.m, &f.home).unwrap());
        assert_eq!(f.current().manager.sha256, first.manager.sha256);
        f.adopt().unwrap();
        let next = f.current();
        rollback_from(&f.base.m, &f.home).unwrap();
        setup_install::interrupt_journaled_for_test(&f.base.m, &f.home, &next, Some(&first), true).unwrap();
        assert!(recover_from(&f.base.m, &f.home).unwrap());
        assert_eq!(f.current().manager.sha256, next.manager.sha256);
        assert_eq!(fs::read_link(f.home.join(".local/bin/linux-audio-compatibility-manager")).unwrap(), next.operator_frontend.unwrap().path);
    }

    #[test]
    fn interrupted_switch_refuses_foreign_route_without_erasing_journal() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let first = f.current();
        f.replace("linux-vst-bridge", b"next manager", true);
        f.replace("linux-audio-compatibility-manager", b"next frontend", true);
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        let next = stage(&f.base.m, &f.inputs, manifest, sha, Some(first.clone())).unwrap();
        setup_install::interrupt_journaled_for_test(&f.base.m, &f.home, &next, Some(&first), false).unwrap();
        let link = f.home.join(".local/bin/linux-vst-bridge");
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink("/usr/bin/foreign", &link).unwrap();
        assert!(recover_from(&f.base.m, &f.home).is_err());
        assert!(f.base.m.root.join("package-transition.json").exists());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(),
            serde_json::to_vec(&first).unwrap());
    }
}

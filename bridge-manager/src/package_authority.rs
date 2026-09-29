//! PKG0: fixed installed-package intake into immutable user-owned software.
//! Package-manager ownership authenticates /usr inputs; the release signature
//! is checked by the package/release tooling before these files are installed.
use super::*;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;

const PACKAGE_ROOT: &str = "/usr";
const NAMES: [&str; 6] = [
    "linux-vst-bridge", "linux-audio-compatibility-manager", "session.pyc",
    "ownership.pyc", "host.exe", "host-source-manifest.json",
];
const NAMES_WITH_KIT: [&str; 7] = [
    "linux-vst-bridge", "linux-audio-compatibility-manager", "session.pyc",
    "ownership.pyc", "host.exe", "host-source-manifest.json", "preparation-kit.zip",
];
fn names(schema: u32) -> Result<&'static [&'static str]> {
    match schema {
        1 => Ok(&NAMES),
        2 => Ok(&NAMES_WITH_KIT),
        _ => Err("package_manifest_schema".into()),
    }
}

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
            "preparation-kit.zip" => "lib/linux-vst-bridge/preparation/preparation-kit.zip",
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
    package: String,
    version: String,
    pkgrel: u32,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    packaged_preparation_kit_sha256: Option<String>,
}

fn package_source(path: &Path, owner: u32, max: u64, capture: bool) -> Result<(String, u64, Vec<u8>)> {
    let mut source = fs::OpenOptions::new().read(true)
        .custom_flags(libc::O_NOFOLLOW).open(path)?;
    let before = source.metadata()?;
    require(path.is_absolute() && before.is_file() && before.uid() == owner
        && before.mode() & 0o022 == 0 && before.len() <= max,
        "package_artifact_owner_or_extent")?;
    let mut hash = sha2::Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut bytes = Vec::new();
    let mut length = 0u64;
    loop {
        let size = source.read(&mut buffer)?;
        if size == 0 { break; }
        length = length.checked_add(size as u64).ok_or("package_artifact_extent")?;
        require(length <= max && length <= before.len(), "package_artifact_extent")?;
        hash.update(&buffer[..size]);
        if capture { bytes.extend_from_slice(&buffer[..size]); }
    }
    let after = source.metadata()?;
    let path_after = fs::symlink_metadata(path)?;
    require(length == before.len() && before.dev() == after.dev()
        && before.ino() == after.ino() && before.len() == after.len()
        && before.dev() == path_after.dev() && before.ino() == path_after.ino()
        && before.mtime() == after.mtime() && before.mtime_nsec() == after.mtime_nsec()
        && before.ctime() == after.ctime() && before.ctime_nsec() == after.ctime_nsec()
        && before.uid() == after.uid() && before.mode() == after.mode(),
        "package_artifact_changed_during_read")?;
    Ok((hex(&hash.finalize()), length, bytes))
}
fn read_manifest(inputs: &Inputs, owner: u32) -> Result<(PackageManifest, String)> {
    let path = inputs.manifest();
    let (sha, _, bytes) = package_source(&path, owner, 64 * 1024, true)?;
    let manifest: PackageManifest = serde_json::from_slice(&bytes)?;
    validate_manifest_identity(&manifest)?;
    for (entry, name) in manifest.files.iter().zip(names(manifest.schema)?) {
        let maximum = if *name == "preparation-kit.zip" { 256 } else { 128 } * 1024 * 1024;
        let (actual, size, _) = package_source(&inputs.path(name)?, owner, maximum, false)?;
        require(actual == entry.sha256 && size == entry.size,
            "package_artifact_changed")?;
    }
    Ok((manifest, sha))
}
fn validate_manifest_identity(manifest: &PackageManifest) -> Result<()> {
    let expected = names(manifest.schema)?;
    require(manifest.operator_schema == operator_model::OPERATOR_SCHEMA
        && manifest.package == "linux-vst-bridge-beta"
        && valid_package_version(&manifest.version) && manifest.pkgrel == 1
        && valid_hex(&manifest.source_head, 40) && valid_hex(&manifest.source_tree, 40)
        && !manifest.external_runtime.id.is_empty()
        && manifest.external_runtime.id.len() <= 128
        && manifest.external_runtime.id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        && valid_hex(&manifest.external_runtime.manifest_sha256, 64)
        && manifest.files.len() == expected.len(), "package_manifest_schema")?;
    for (entry, name) in manifest.files.iter().zip(expected) {
        let maximum = if *name == "preparation-kit.zip" { 256 } else { 128 } * 1024 * 1024;
        require(entry.name == *name && valid_hex(&entry.sha256, 64)
            && entry.size <= maximum,
            "package_manifest_roster")?;
    }
    Ok(())
}
fn valid_package_version(value: &str) -> bool {
    !value.is_empty() && value.len() <= 80 && value.as_bytes()[0].is_ascii_digit()
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.')
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
fn require_retained_host_pair(m: &Manager, old: &Software,
    manifest: &PackageManifest) -> Result<()> {
    if old.native_catalogue.is_some() || !m.registry()?.classes.is_empty() {
        require(old.host.sha256 == manifest.files[4].sha256
            && old.source_manifest.sha256 == manifest.files[5].sha256,
            "package_existing_product_host_pair_changed")?;
    }
    Ok(())
}
fn generation_record(manifest: PackageManifest, manifest_sha256: String,
    predecessor: Option<Software>) -> Generation {
    let catalogue = predecessor.as_ref().and_then(|s| s.native_catalogue.as_ref());
    let packaged_preparation_kit_sha256 = (manifest.schema == 2)
        .then(|| manifest.files[NAMES.len()].sha256.clone());
    Generation {
        schema: if packaged_preparation_kit_sha256.is_some() { 2 } else { 1 },
        manifest_sha256, manifest,
        retained_installer_launch: predecessor.as_ref().and_then(|s| s.installer_launch.clone()),
        retained_preparation_kit: if packaged_preparation_kit_sha256.is_some() { None } else {
            predecessor.as_ref().and_then(|s| s.preparation_kit.clone())
        },
        catalogue_sha256: catalogue.map(|a| a.sha256.clone()),
        packaged_preparation_kit_sha256,
        predecessor,
    }
}

/// A read-only predecessor decision. The caller separately verifies the
/// candidate package bytes; this method reads selected user authority and
/// computes the same immutable generation record that adoption would stage.
fn predecessor_plan(m: &Manager, home: &Path, manifest: PackageManifest,
    manifest_sha256: String) -> Result<(Generation, Vec<setup_install::RouteStatus>)> {
    validate_manifest_identity(&manifest)?;
    require(valid_hex(&manifest_sha256, 64), "package_manifest_digest")?;
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let old = old_software(m)?.ok_or("package_predecessor_absent")?;
    require_retained_host_pair(m, &old, &manifest)?;
    let routes = setup_install::current_route_statuses(m, home, &old)?;
    if old.manager.path.parent().is_some_and(|dir|
        dir.join("package-generation.json").exists()) {
        let selected = verify_generation(m, &old)?;
        if selected.manifest_sha256 == manifest_sha256 {
            require(selected.manifest == manifest, "package_manifest_changed")?;
            return Ok((selected, routes));
        }
    }
    let record = generation_record(manifest, manifest_sha256, Some(old.clone()));
    let planned_id = id(&record)?;
    require(old.manager.path.parent().and_then(|p| p.file_name())
        .and_then(|p| p.to_str()) != Some(planned_id.as_str()),
        "package_self_predecessor")?;
    Ok((record, routes))
}
/// Source-owned PB0-R3 audit entry point. The input is only an exact bounded
/// package manifest, never a path or command. It cannot stage or select it.
#[cfg(feature = "pb0-r3-audit")]
pub(super) fn audit_predecessor_from_bytes(m: &Manager, home: &Path,
    bytes: &[u8]) -> Result<serde_json::Value> {
    require(bytes.len() <= 64 * 1024, "package_manifest_extent")?;
    let manifest: PackageManifest = serde_json::from_slice(bytes)?;
    let sha = hex(&sha2::Sha256::digest(bytes));
    let (record, routes) = predecessor_plan(m, home, manifest, sha)?;
    let old = old_software(m)?.ok_or("package_predecessor_absent")?;
    let selected_generation = old.manager.path.parent()
        .and_then(|p| p.file_name()).and_then(|p| p.to_str())
        .ok_or("package_predecessor_identity")?;
    let planned_generation = id(&record)?;
    let rollback_predecessor = record.predecessor.as_ref().and_then(|s|
        s.manager.path.parent()?.file_name()?.to_str());
    Ok(serde_json::json!({
        "schema": 1,
        "candidate_source_head": record.manifest.source_head,
        "candidate_source_tree": record.manifest.source_tree,
        "candidate_version": record.manifest.version,
        "candidate_manifest_sha256": record.manifest_sha256,
        "planned_generation": planned_generation,
        "selected_generation": selected_generation,
        "reuses_selected_generation": planned_generation == selected_generation,
        "predecessor_generation": rollback_predecessor,
        "predecessor_manager_sha256": old.manager.sha256,
        "predecessor_frontend_sha256": old.operator_frontend.as_ref().map(|a| &a.sha256),
        "retained_catalogue_sha256": record.catalogue_sha256,
        "retained_host_sha256": old.host.sha256,
        "retained_source_sha256": old.source_manifest.sha256,
        "candidate_preparation_kit_sha256": record.packaged_preparation_kit_sha256,
        "predecessor_preparation_kit_sha256": old.preparation_kit.as_ref().map(|a| &a.sha256),
        "external_runtime_id": record.manifest.external_runtime.id,
        "external_runtime_manifest_sha256": record.manifest.external_runtime.manifest_sha256,
        "routes": routes,
        "writes": false,
    }))
}
fn verify_generation(m: &Manager, current: &Software) -> Result<Generation> {
    let dir = current.manager.path.parent().ok_or("package_generation_path")?;
    require(dir.parent() == Some(m.root.join("software").as_path()),
        "package_generation_path")?;
    // Verification and predecessor planning are read-only. In particular an
    // absent selected generation must never be recreated as a side effect.
    existing_generation_dir(dir)?;
    require(fs::metadata(dir.join("package-generation.json"))?.permissions().mode() & 0o222 == 0,
        "package_generation_record_writable")?;
    let record: Generation = read_json(&dir.join("package-generation.json"))?;
    require(matches!((record.schema, record.manifest.schema), (1, 1) | (2, 2))
        && id(&record)? == dir.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "package_generation_identity")?;
    let expected_names = names(record.manifest.schema)?;
    require(record.manifest.package == "linux-vst-bridge-beta"
        && valid_package_version(&record.manifest.version) && record.manifest.pkgrel == 1
        && record.manifest.files.len() == expected_names.len()
        && record.manifest.files.iter().zip(expected_names).all(|(entry, name)| entry.name == *name
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
    let packaged_kit = if record.schema == 2 {
        let sha = &record.manifest.files[NAMES.len()].sha256;
        require(record.packaged_preparation_kit_sha256.as_ref() == Some(sha)
            && record.retained_preparation_kit.is_none(), "package_generation_kit_binding")?;
        let kit = Artifact { path: dir.join("preparation-kit.zip"), sha256: sha.clone() };
        kit.verify()?;
        require(fs::metadata(&kit.path)?.permissions().mode() & 0o222 == 0,
            "package_generation_kit_writable")?;
        Some(kit)
    } else {
        require(record.packaged_preparation_kit_sha256.is_none(),
            "package_generation_kit_binding")?;
        record.retained_preparation_kit.clone()
    };
    require(current.source_sha256 == current.source_manifest.sha256
        && current.installer_launch == record.retained_installer_launch
        && current.preparation_kit == packaged_kit
        && current.native_catalogue.as_ref().map(|a| a.sha256.as_str())
            == record.catalogue_sha256.as_deref(),
        "package_generation_software_binding")?;
    if let Some(a) = &current.native_catalogue {
        require(a.path == dir.join("native-catalogue.json"),
            "package_generation_catalogue_binding")?;
        a.verify()?;
        require(fs::metadata(&a.path)?.permissions().mode() & 0o222 == 0,
            "package_generation_catalogue_writable")?;
    }
    Ok(record)
}
pub(super) fn selected_package_version(m: &Manager, current: &Software) -> Result<Option<String>> {
    let Some(dir) = current.manager.path.parent() else { return Ok(None); };
    if !dir.join("package-generation.json").try_exists()? { return Ok(None); }
    Ok(Some(verify_generation(m, current)?.manifest.version))
}
fn existing_generation_dir(dir: &Path) -> Result<()> {
    existing_generation_dir_with(dir, || {})
}
fn existing_generation_dir_with(dir: &Path, after_open: impl FnOnce()) -> Result<()> {
    let opened = fs::OpenOptions::new().read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW).open(dir)?;
    after_open();
    let held = opened.metadata()?;
    let path = fs::symlink_metadata(dir)?;
    require(held.is_dir() && path.is_dir() && held.uid() == unsafe { libc::getuid() }
        && held.mode() & 0o077 == 0 && held.dev() == path.dev()
        && held.ino() == path.ino(), "package_generation_directory_changed")?;
    Ok(())
}
fn stage(m: &Manager, inputs: &Inputs, manifest: PackageManifest,
    manifest_sha256: String, predecessor: Option<Software>) -> Result<Software> {
    let catalogue = predecessor.as_ref().and_then(|s| s.native_catalogue.clone());
    if let Some(a) = &catalogue { a.verify()?; }
    let record = generation_record(manifest, manifest_sha256, predecessor);
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
        require(fs::metadata(&a.path)?.permissions().mode() & 0o222 == 0,
            "package_generation_catalogue_writable")?;
        Some(a)
    } else { None };
    let result = Software {
        installer_launch: record.retained_installer_launch,
        preparation_kit: if record.schema == 2 {
            Some(artifact(&dest, "preparation-kit.zip")?)
        } else { record.retained_preparation_kit },
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
    let _package = m.lock("package.lock")?;
    let _service = m.lock("service.lock")?;
    let _setup = m.lock("setup.lock")?;
    let _registry = m.lock("registry.lock")?;
    work()
}
fn adopt_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    with_locks(m, || {
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m)?;
        let (manifest, sha) = read_manifest(inputs, owner)?;
        let predecessor = old_software(m)?;
        if let Some(old) = &predecessor {
            require_retained_host_pair(m, old, &manifest)?;
            // The same read-only predecessor and route checks used by the
            // PB0-R3 gate run before any successor files are staged.
            let _ = predecessor_plan(m, home, manifest.clone(), sha.clone())?;
        }
        if let Some(old) = &predecessor {
            if old.manager.path.parent()
                .is_some_and(|dir| dir.join("package-generation.json").exists()) {
                let old_record = verify_generation(m, old)?;
                if old_record.manifest_sha256 == sha {
                    require(old_record.manifest == manifest, "package_manifest_changed")?;
                    return setup_install::commit_journaled(m, home, old, Some(old),
                        |selected| reload_and_verify(service, home, selected));
                }
            }
        }
        let installed = stage(m, inputs, manifest, sha, predecessor.clone())?;
        setup_install::commit_journaled(m, home, &installed, predecessor.as_ref(),
            |selected| reload_and_verify(service, home, selected))
    })
}
fn rollback_from(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<()> {
    with_locks(m, || {
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m)?;
        let current = old_software(m)?.ok_or("package_not_installed")?;
        let record = verify_generation(m, &current)?;
        let old = record.predecessor.ok_or("package_no_predecessor")?;
        verify_software_identity(m, &old)?;
        setup_install::commit_journaled(m, home, &old, Some(&current),
            |selected| reload_and_verify(service, home, selected))
    })
}
fn recover_from(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<bool> {
    with_locks(m, || {
        require_service_stopped(service)?;
        preflight(m)?;
        setup_install::recover_journaled(m, home,
            |selected| reload_and_verify(service, home, selected))
    })
}
#[derive(Clone, Debug)]
struct UnitReadback { load: String, active: String, fragment: String, exec: String }
trait ServiceControl {
    fn show(&self) -> Result<UnitReadback>;
    fn reload(&self) -> Result<()>;
    fn enable_start(&self) -> Result<()>;
    fn healthy(&self, m: &Manager) -> Result<()>;
}
struct SystemctlService;
fn systemctl_bounded(args: &[&str], capture: bool) -> Result<Vec<u8>> {
    let mut child = Command::new("systemctl").args(args)
        .stdin(std::process::Stdio::null())
        .stdout(if capture { std::process::Stdio::piped() } else { std::process::Stdio::null() })
        .stderr(std::process::Stdio::null()).spawn()?;
    for _ in 0..100 {
        if let Some(status) = child.try_wait()? {
            require(status.success(), "package_user_service_unavailable")?;
            if !capture { return Ok(Vec::new()); }
            let mut stdout = child.stdout.take().ok_or("package_user_service_readback")?;
            let fd = stdout.as_raw_fd();
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            require(flags >= 0 && unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
                "package_user_service_readback")?;
            let mut bytes = Vec::new();
            let mut ended = false;
            for _ in 0..100 {
                let mut buffer = [0u8; 8193];
                match stdout.read(&mut buffer) {
                    Ok(0) => { ended = true; break; }
                    Ok(count) => bytes.extend_from_slice(&buffer[..count]),
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                    Err(error) => return Err(error.into()),
                }
                require(bytes.len() <= 8192, "package_user_service_readback")?;
            }
            require(ended && bytes.len() <= 8192, "package_user_service_readback")?;
            return Ok(bytes);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    child.kill()?;
    child.wait()?;
    Err("package_user_service_timeout".into())
}
impl ServiceControl for SystemctlService {
    fn show(&self) -> Result<UnitReadback> {
        let output = systemctl_bounded(&[
            "--user", "show", "--no-pager",
            "--property=LoadState,ActiveState,FragmentPath,ExecStart",
            "linux-vst-bridge.service",
        ], true)?;
        parse_unit_readback(std::str::from_utf8(&output)?)
    }
    fn reload(&self) -> Result<()> {
        systemctl_bounded(&["--user", "daemon-reload"], false).map(|_| ())
    }
    fn enable_start(&self) -> Result<()> {
        systemctl_bounded(&["--user", "enable", "--now", "linux-vst-bridge.service"], false).map(|_| ())
    }
    fn healthy(&self, m: &Manager) -> Result<()> {
        require(capacity_reply(m)?["ok"] == true, "package_service_health_unavailable")
    }
}
fn parse_unit_readback(text: &str) -> Result<UnitReadback> {
    let mut fields = std::collections::BTreeMap::new();
    for line in text.lines() {
        let (key, value) = line.split_once('=').ok_or("package_user_service_readback")?;
        require(matches!(key, "LoadState" | "ActiveState" | "FragmentPath" | "ExecStart")
            && fields.insert(key, value).is_none(), "package_user_service_readback")?;
    }
    let get = |key| fields.get(key).copied().ok_or("package_user_service_readback");
    Ok(UnitReadback { load: get("LoadState")?.into(), active: get("ActiveState")?.into(),
        fragment: get("FragmentPath")?.into(), exec: get("ExecStart")?.into() })
}
fn require_service_stopped(service: &impl ServiceControl) -> Result<()> {
    let state = service.show()?;
    require((state.load == "not-found" && state.active == "inactive"
        && state.fragment.is_empty() && state.exec.is_empty())
        || (state.load == "loaded" && matches!(state.active.as_str(), "inactive" | "failed")
            && !state.fragment.is_empty() && !state.exec.is_empty()),
        "package_stop_service_first")
}
fn effective_exec_is(raw: &str, executable: &Path) -> bool {
    if raw.matches("{ path=").count() != 1 || !raw.ends_with('}') { return false; }
    let Some(expected) = executable.to_str() else { return false; };
    let Some(tail) = raw.strip_prefix("{ path=") else { return false; };
    let Some((path, tail)) = tail.split_once(" ; argv[]=") else { return false; };
    let Some((argv, _)) = tail.split_once(" ; ") else { return false; };
    path == expected && argv == format!("{expected} serve")
}
fn reload_and_verify(service: &impl ServiceControl, home: &Path,
    selected: Option<&Software>) -> Result<()> {
    service.reload()?;
    let state = service.show()?;
    match selected {
        Some(software) => require(state.load == "loaded"
            && matches!(state.active.as_str(), "inactive" | "failed")
            && state.fragment == home.join(".config/systemd/user/linux-vst-bridge.service")
                .to_str().ok_or("package_service_path")?
            && effective_exec_is(&state.exec, &software.manager.path),
            "package_service_effective_route_mismatch"),
        None => require(state.load == "not-found" && state.active == "inactive"
            && state.fragment.is_empty() && state.exec.is_empty(),
            "package_service_effective_route_mismatch"),
    }
}
#[derive(Serialize)]
struct ActivationStatus {
    schema: u32,
    state: &'static str,
    package_version: String,
}
fn selected_activation(m: &Manager, home: &Path,
    service: &impl ServiceControl) -> Result<(Software, Generation, UnitReadback)> {
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let selected = old_software(m)?.ok_or("package_not_installed")?;
    let generation = verify_generation(m, &selected)?;
    let routes = setup_install::current_route_statuses(m, home, &selected)?;
    require(routes.len() == 6 && routes.iter().all(|route| route.status == "exact"),
        "package_routes_need_repair")?;
    let state = service.show()?;
    require(state.load == "loaded"
        && state.fragment == home.join(".config/systemd/user/linux-vst-bridge.service")
            .to_str().ok_or("package_service_path")?
        && effective_exec_is(&state.exec, &selected.manager.path)
        && matches!(state.active.as_str(), "inactive" | "failed" | "active"),
        "package_service_effective_route_mismatch")?;
    Ok((selected, generation, state))
}
fn activation_status_from(m: &Manager, home: &Path,
    service: &impl ServiceControl) -> Result<ActivationStatus> {
    let (selected, generation, state) = selected_activation(m, home, service)?;
    if state.active == "active" { service.healthy(m)?; }
    let (again, second, after) = selected_activation(m, home, service)?;
    require(selected.manager == again.manager && generation.manifest_sha256 == second.manifest_sha256
        && state.active == after.active, "package_activation_status_changed")?;
    Ok(ActivationStatus { schema: 1,
        state: if state.active == "active" { "active" } else { "inactive" },
        package_version: generation.manifest.version })
}
fn activate_from(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<()> {
    // A package action gate excludes a concurrent package switch. setup.lock
    // excludes legacy route changes. The service must acquire service.lock on
    // startup, so neither service.lock nor registry.lock spans systemctl.
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    let (selected, generation, state) = selected_activation(m, home, service)?;
    if state.active != "active" {
        preflight(m)?;
        let (checked, checked_generation, checked_state) = selected_activation(m, home, service)?;
        require(checked.manager == selected.manager
            && checked_generation.manifest_sha256 == generation.manifest_sha256
            && checked_state.active != "active", "package_activation_state_changed")?;
        service.enable_start()?;
    }
    let (after, after_generation, readback) = selected_activation(m, home, service)?;
    require(after.manager == selected.manager
        && after_generation.manifest_sha256 == generation.manifest_sha256
        && readback.active == "active", "package_service_did_not_start")?;
    service.healthy(m)?;
    Ok(())
}
pub(super) fn adopt(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    adopt_from(m, &home, &Inputs::system(), 0, &SystemctlService)
}
pub(super) fn rollback(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    rollback_from(m, &home, &SystemctlService)
}
pub(super) fn recover(m: &Manager) -> Result<bool> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    recover_from(m, &home, &SystemctlService)
}
pub(super) fn activation_status(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    println!("{}", serde_json::to_string(&activation_status_from(m, &home, &SystemctlService)?)?);
    Ok(())
}
pub(super) fn activate(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    activate_from(m, &home, &SystemctlService)
}

#[cfg(test)]
mod tests {
    use super::*;
    use linux_vst_bridge::catalogue::{Catalogue, NativeArtifact};
    use std::cell::{Cell, RefCell};
    struct FakeService {
        home: PathBuf,
        loaded: RefCell<UnitReadback>,
        fail_reload: Cell<bool>,
        fail_show: Cell<bool>,
        stale_reload: Cell<bool>,
        fail_start: Cell<bool>,
        fail_health: Cell<bool>,
        starts: Cell<usize>,
    }
    impl FakeService {
        fn new(home: &Path) -> Self {
            Self { home: home.into(), loaded: RefCell::new(UnitReadback {
                load: "not-found".into(), active: "inactive".into(),
                fragment: String::new(), exec: String::new(),
            }), fail_reload: Cell::new(false), fail_show: Cell::new(false),
                stale_reload: Cell::new(false), fail_start: Cell::new(false),
                fail_health: Cell::new(false), starts: Cell::new(0) }
        }
    }
    impl ServiceControl for FakeService {
        fn show(&self) -> Result<UnitReadback> {
            require(!self.fail_show.get(), "package_user_service_unavailable")?;
            Ok(self.loaded.borrow().clone())
        }
        fn reload(&self) -> Result<()> {
            require(!self.fail_reload.get(), "package_user_service_reload_failed")?;
            if self.stale_reload.get() { return Ok(()); }
            let path = self.home.join(".config/systemd/user/linux-vst-bridge.service");
            let next = if path.exists() {
                let unit = fs::read_to_string(&path)?;
                let command = unit.lines().find_map(|line| line.strip_prefix("ExecStart=\""))
                    .ok_or("test_unit_exec")?;
                let (executable, _) = command.split_once("\" serve").ok_or("test_unit_exec")?;
                UnitReadback { load: "loaded".into(), active: "inactive".into(),
                    fragment: path.to_str().ok_or("test_unit_path")?.into(),
                    exec: format!("{{ path={executable} ; argv[]={executable} serve ; ignore_errors=no }}"), }
            } else {
                UnitReadback { load: "not-found".into(), active: "inactive".into(),
                    fragment: String::new(), exec: String::new() }
            };
            *self.loaded.borrow_mut() = next;
            Ok(())
        }
        fn enable_start(&self) -> Result<()> {
            require(!self.fail_start.get(), "package_user_service_unavailable")?;
            let mut state = self.loaded.borrow_mut();
            require(state.load == "loaded" && matches!(state.active.as_str(), "inactive" | "failed"),
                "package_service_effective_route_mismatch")?;
            state.active = "active".into();
            self.starts.set(self.starts.get() + 1);
            Ok(())
        }
        fn healthy(&self, _: &Manager) -> Result<()> {
            require(!self.fail_health.get(), "package_service_health_unavailable")
        }
    }
    struct Fixture {
        base: test_fixture::Fixture,
        home: PathBuf,
        inputs: Inputs,
        owner: u32,
        service: FakeService,
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
            let service = FakeService::new(&home);
            let result = Self { base, home, inputs, owner: unsafe { libc::getuid() }, service };
            result.write_manifest();
            result
        }
        fn manifest(&self) -> PackageManifest {
            let schema = if self.inputs.path("preparation-kit.zip").unwrap().exists() { 2 } else { 1 };
            PackageManifest {
                schema, package: "linux-vst-bridge-beta".into(), version: "0.1.0beta1".into(),
                pkgrel: 1,
                source_head: "ab".repeat(20), source_tree: "cd".repeat(20),
                operator_schema: operator_model::OPERATOR_SCHEMA,
                files: names(schema).unwrap().iter().map(|name| {
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
        fn add_kit(&self, data: &[u8]) {
            let path = self.inputs.path("preparation-kit.zip").unwrap();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, data).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
            self.write_manifest();
        }
        fn current(&self) -> Software { software(&self.base.m).unwrap() }
        fn adopt(&self) -> Result<()> {
            adopt_from(&self.base.m, &self.home, &self.inputs, self.owner, &self.service)
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
            self.service.reload().unwrap();
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
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), original);
        assert_eq!(fs::read_link(f.home.join(".local/bin/linux-vst-bridge")).unwrap(), first.manager.path);
        assert_eq!(fs::read_link(f.home.join(".local/bin/linux-audio-compatibility-manager")).unwrap(), first.operator_frontend.unwrap().path);
        second.manager.verify().unwrap();
    }

    #[test]
    fn explicit_activation_requires_exact_selected_routes_and_is_idempotent() {
        let f = Fixture::new();
        assert!(activation_status_from(&f.base.m, &f.home, &f.service).is_err());
        f.adopt().unwrap();
        let software = fs::read(f.base.m.root.join("software.json")).unwrap();
        let registry = fs::read(f.base.m.root.join("registry.json")).ok();
        let unit = fs::read(f.home.join(".config/systemd/user/linux-vst-bridge.service")).unwrap();
        let status = activation_status_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!((status.schema, status.state, status.package_version.as_str()),
                   (1, "inactive", "0.1.0beta1"));
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(activation_status_from(&f.base.m, &f.home, &f.service).unwrap().state,
                   "active");
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(f.service.starts.get(), 1);
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), software);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).ok(), registry);
        assert_eq!(fs::read(f.home.join(".config/systemd/user/linux-vst-bridge.service")).unwrap(), unit);
    }

    #[test]
    fn activation_refuses_foreign_route_unavailable_bus_and_changed_exec() {
        let f = Fixture::new();
        f.adopt().unwrap();
        f.service.fail_show.set(true);
        assert!(activation_status_from(&f.base.m, &f.home, &f.service).is_err());
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        f.service.fail_show.set(false);
        f.service.loaded.borrow_mut().exec =
            "{ path=/usr/bin/foreign ; argv[]=/usr/bin/foreign serve ; ignore_errors=no }".into();
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        f.service.reload().unwrap();
        let desktop = f.home.join(".local/share/applications/linux-audio-compatibility-manager.desktop");
        fs::write(&desktop, b"foreign desktop\n").unwrap();
        assert!(activation_status_from(&f.base.m, &f.home, &f.service).is_err());
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        assert_eq!(f.service.starts.get(), 0);
    }

    #[test]
    fn failed_start_and_health_readback_do_not_claim_activation() {
        let f = Fixture::new();
        f.adopt().unwrap();
        f.service.fail_start.set(true);
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        assert_eq!(activation_status_from(&f.base.m, &f.home, &f.service).unwrap().state,
                   "inactive");
        f.service.fail_start.set(false);
        f.service.fail_health.set(true);
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        assert_eq!(f.service.starts.get(), 1);
        f.service.fail_health.set(false);
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(f.service.starts.get(), 1);
    }

    #[test]
    fn activation_refuses_pending_or_uncertain_owner_before_start() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let pending = f.base.m.root.join("transactions/a.pending.json");
        fs::create_dir_all(pending.parent().unwrap()).unwrap();
        fs::write(&pending, b"{}").unwrap();
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        assert_eq!(f.service.starts.get(), 0);
        fs::remove_file(pending).unwrap();
        let lease = f.base.m.root.join("runtime/leases/").join("ab".repeat(16) + ".json");
        fs::write(&lease, b"\"/tmp/unknown-report.json\"").unwrap();
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        assert_eq!(f.service.starts.get(), 0);
    }

    #[test]
    fn predecessor_plan_is_read_only_and_matches_adoption_identity() {
        let (base, _, _, native) = test_fixture::prepared();
        let home = base.outer.join("home");
        let f = Fixture { service: FakeService::new(&home), home,
            inputs: Inputs::under(&base.outer.join("package/usr")),
            owner: unsafe { libc::getuid() }, base };
        for name in NAMES {
            let path = f.inputs.path(name).unwrap();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, if name == "host.exe" { b"host".as_slice() }
                else if name == "host-source-manifest.json" { b"fixture host source".as_slice() }
                else { name.as_bytes() }).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();
        }
        f.write_manifest();
        f.setup_legacy_with_catalogue(native);
        let old = f.current();
        let software_before = fs::read(f.base.m.root.join("software.json")).unwrap();
        let catalogue_before = fs::read(old.native_catalogue.as_ref().unwrap().path.clone()).unwrap();
        let registry_before = fs::read(f.base.m.root.join("registry.json")).unwrap();
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        let (planned, routes) = predecessor_plan(&f.base.m, &f.home, manifest.clone(), sha.clone()).unwrap();
        assert!(routes.iter().all(|r| r.status == "exact"));
        assert_eq!(planned.predecessor.as_ref().unwrap().manager.sha256, old.manager.sha256);
        assert_eq!(planned.catalogue_sha256.as_deref(), old.native_catalogue.as_ref().map(|a| a.sha256.as_str()));
        assert!(!f.base.m.root.join("package-transition.json").exists());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), software_before);
        assert_eq!(fs::read(old.native_catalogue.as_ref().unwrap().path.clone()).unwrap(), catalogue_before);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_before);
        assert!(!f.base.m.root.join("software").join(id(&planned).unwrap()).exists());
        f.adopt().unwrap();
        assert_eq!(f.current().manager.path.parent().unwrap().file_name().unwrap().to_str(),
            Some(id(&planned).unwrap().as_str()));
        let selected_before = fs::read(f.base.m.root.join("software.json")).unwrap();
        let (same, _) = predecessor_plan(&f.base.m, &f.home, manifest, sha).unwrap();
        assert_eq!(id(&same).unwrap(), id(&planned).unwrap());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected_before);
    }

    #[test]
    fn missing_selected_generation_is_never_created_by_predecessor_plan() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let current = f.current();
        let generation = current.manager.path.parent().unwrap().to_path_buf();
        let selected = fs::read(f.base.m.root.join("software.json")).unwrap();
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        fs::remove_dir_all(&generation).unwrap();
        assert!(predecessor_plan(&f.base.m, &f.home, manifest, sha).is_err());
        assert!(!generation.exists());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected);
    }

    #[test]
    fn replaced_generation_directory_refuses_without_creation() {
        let f = Fixture::new();
        let selected = f.base.m.root.join("software/race-generation");
        let replacement = f.base.m.root.join("software/replacement-generation");
        let held_name = f.base.m.root.join("software/held-generation");
        private_dir(&selected).unwrap();
        private_dir(&replacement).unwrap();
        let result = existing_generation_dir_with(&selected, || {
            fs::rename(&selected, &held_name).unwrap();
            std::os::unix::fs::symlink(&replacement, &selected).unwrap();
        });
        assert!(result.is_err());
        assert!(fs::symlink_metadata(&selected).unwrap().file_type().is_symlink());
        assert!(held_name.is_dir() && replacement.is_dir());
    }

    #[test]
    fn predecessor_plan_classifies_missing_owned_routes_and_refuses_foreign_or_changed_host() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        let frontend = f.home.join(".local/bin/linux-audio-compatibility-manager");
        fs::remove_file(&frontend).unwrap();
        let (same, routes) = predecessor_plan(&f.base.m, &f.home, manifest.clone(), sha.clone()).unwrap();
        assert_eq!(routes.iter().find(|r| r.route == "frontend_command").unwrap().status, "missing");
        assert_eq!(same.manifest_sha256, sha);
        std::os::unix::fs::symlink("/tmp/foreign-manager", &frontend).unwrap();
        assert!(predecessor_plan(&f.base.m, &f.home, manifest.clone(), sha.clone()).is_err());
        fs::remove_file(&frontend).unwrap();
        let unit = f.home.join(".config/systemd/user/linux-vst-bridge.service");
        let original = fs::read(&unit).unwrap();
        let stale = String::from_utf8(original.clone()).unwrap()
            .replace("RestartSec=2", "RestartSec=3");
        fs::write(&unit, stale.as_bytes()).unwrap();
        let (_, routes) = predecessor_plan(&f.base.m, &f.home, manifest.clone(), sha.clone()).unwrap();
        assert_eq!(routes.iter().find(|r| r.route == "service").unwrap().status,
            "stale_package_owned");
        assert_eq!(fs::read(&unit).unwrap(), stale.as_bytes());
        fs::write(&unit, original).unwrap();
        let mut changed_host = manifest;
        changed_host.files[4].sha256 = "ab".repeat(32);
        assert!(predecessor_plan(&f.base.m, &f.home, changed_host, sha).is_err());
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
    fn packaged_kit_has_exact_generation_ownership_and_rollback() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let original = f.current();
        let original_record = original.manager.path.parent().unwrap().join("package-generation.json");
        let original_record_bytes = fs::read(&original_record).unwrap();
        assert!(!serde_json::to_value(verify_generation(&f.base.m, &original).unwrap()).unwrap()
            .as_object().unwrap().contains_key("packaged_preparation_kit_sha256"));
        f.add_kit(b"source-owned native kit revision two");
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        assert_eq!(manifest.schema, 2);
        let (planned, _) = predecessor_plan(&f.base.m, &f.home, manifest, sha).unwrap();
        let selected_before = fs::read(f.base.m.root.join("software.json")).unwrap();
        assert_eq!(planned.packaged_preparation_kit_sha256.as_deref(),
            Some(digest(&f.inputs.path("preparation-kit.zip").unwrap()).unwrap().as_str()));
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected_before);
        assert!(!f.base.m.root.join("software").join(id(&planned).unwrap()).exists());

        f.adopt().unwrap();
        let selected = f.current();
        let kit = selected.preparation_kit.as_ref().unwrap();
        assert_eq!(kit.path, selected.manager.path.parent().unwrap().join("preparation-kit.zip"));
        assert_eq!(fs::read(&kit.path).unwrap(), b"source-owned native kit revision two");
        assert_eq!(fs::metadata(&kit.path).unwrap().permissions().mode() & 0o222, 0);
        assert_eq!(verify_generation(&f.base.m, &selected).unwrap().schema, 2);
        let selected_bytes = fs::read(f.base.m.root.join("software.json")).unwrap();
        let selected_record = selected.manager.path.parent().unwrap().join("package-generation.json");
        let selected_record_bytes = fs::read(&selected_record).unwrap();
        f.adopt().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected_bytes);
        assert_eq!(fs::read(&selected_record).unwrap(), selected_record_bytes);
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected_before);
        assert!(f.current().preparation_kit.is_none());
        assert_eq!(fs::read(&original_record).unwrap(), original_record_bytes);
        f.adopt().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected_bytes);
    }

    #[test]
    fn packaged_kit_changed_input_or_retained_bytes_refuse() {
        let f = Fixture::new();
        f.adopt().unwrap();
        f.add_kit(b"kit-one");
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.replace("preparation-kit.zip", b"changed without manifest", false);
        assert!(f.adopt().is_err());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
        f.write_manifest();
        f.adopt().unwrap();
        let selected = f.current();
        let kit = selected.preparation_kit.unwrap();
        fs::set_permissions(&kit.path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(f.adopt().is_err());
        fs::set_permissions(&kit.path, fs::Permissions::from_mode(0o400)).unwrap();
        f.adopt().unwrap();
        fs::set_permissions(&kit.path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&kit.path, b"changed retained kit").unwrap();
        fs::set_permissions(&kit.path, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(f.adopt().is_err());
    }

    #[test]
    fn populated_legacy_kit_and_catalogue_remain_exact_rollback_authority() {
        let (base, _, _, native) = test_fixture::prepared();
        let home = base.outer.join("home");
        let f = Fixture { service: FakeService::new(&home), home,
            inputs: Inputs::under(&base.outer.join("package/usr")),
            owner: unsafe { libc::getuid() }, base };
        for name in NAMES {
            let path = f.inputs.path(name).unwrap();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, if name == "host.exe" { b"host".as_slice() }
                else if name == "host-source-manifest.json" { b"fixture host source".as_slice() }
                else { name.as_bytes() }).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o444)).unwrap();
        }
        f.write_manifest();
        f.setup_legacy_with_catalogue(native);
        let mut old = f.current();
        let legacy_kit = old.manager.path.parent().unwrap().join("preparation-kit.zip");
        fs::write(&legacy_kit, b"retained legacy preparation kit").unwrap();
        fs::set_permissions(&legacy_kit, fs::Permissions::from_mode(0o444)).unwrap();
        old.preparation_kit = Some(artifact(legacy_kit.parent().unwrap(), "preparation-kit.zip").unwrap());
        atomic_json(&f.base.m.root.join("software.json"), &old).unwrap();
        let old_bytes = fs::read(f.base.m.root.join("software.json")).unwrap();
        let catalogue_bytes = fs::read(&old.native_catalogue.as_ref().unwrap().path).unwrap();
        let registry_bytes = fs::read(f.base.m.root.join("registry.json")).unwrap();
        f.add_kit(b"new source-owned native kit");
        f.adopt().unwrap();
        let new = f.current();
        assert_ne!(new.preparation_kit, old.preparation_kit);
        assert_eq!(fs::read(new.native_catalogue.as_ref().unwrap().path.clone()).unwrap(), catalogue_bytes);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&fs::read(f.base.m.root.join("software.json")).unwrap()).unwrap(),
            serde_json::from_slice::<serde_json::Value>(&old_bytes).unwrap());
        old.preparation_kit.unwrap().verify().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
    }

    #[test]
    fn interrupted_kit_selection_restores_exact_predecessor() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let original = f.current();
        let old_bytes = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.add_kit(b"kit for interrupted generation");
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        let successor = stage(&f.base.m, &f.inputs, manifest, sha, Some(original.clone())).unwrap();
        assert!(successor.preparation_kit.is_some());
        setup_install::interrupt_journaled_for_test(&f.base.m, &f.home,
            &successor, Some(&original), false).unwrap();
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), old_bytes);
        assert!(f.current().preparation_kit.is_none());
        f.adopt().unwrap();
        assert_eq!(f.current().preparation_kit.unwrap().sha256,
            digest(&f.inputs.path("preparation-kit.zip").unwrap()).unwrap());
    }

    #[test]
    fn populated_catalogue_and_user_records_survive_update_and_rollback() {
        let (base, _, _, native) = test_fixture::prepared();
        let home = base.outer.join("home");
        let f = Fixture { service: FakeService::new(&home), home,
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
            fs::write(&path, match name {
                "host.exe" => b"host".as_slice(),
                "host-source-manifest.json" => b"fixture host source".as_slice(),
                _ => name.as_bytes(),
            }).unwrap();
            retained.push((path, name.as_bytes().to_vec()));
        }
        f.adopt().unwrap();
        let first = f.current();
        assert_eq!(fs::read(first.native_catalogue.as_ref().unwrap().path.clone()).unwrap(), catalogue_bytes);
        let first_record = first.manager.path.parent().unwrap().join("package-generation.json");
        let first_record_bytes = fs::read(&first_record).unwrap();
        fs::remove_file(f.home.join(".local/bin/linux-audio-compatibility-manager")).unwrap();
        f.adopt().unwrap();
        assert_eq!(fs::read(&first_record).unwrap(), first_record_bytes);
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
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(f.current().manager.sha256, first.manager.sha256);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
        for (path, bytes) in &retained { assert_eq!(&fs::read(path).unwrap(), bytes); }
        old.native_catalogue.unwrap().verify().unwrap();
    }

    #[test]
    fn copied_native_catalogue_must_remain_nonwritable() {
        let (base, _, _, native) = test_fixture::prepared();
        let home = base.outer.join("home");
        let f = Fixture { service: FakeService::new(&home), home,
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
        f.adopt().unwrap();
        let selected = f.current();
        let catalogue = selected.native_catalogue.unwrap().path;
        let exact = fs::read(&catalogue).unwrap();
        fs::set_permissions(&catalogue, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(f.adopt().is_err());
        assert_eq!(fs::read(&catalogue).unwrap(), exact);
        fs::set_permissions(&catalogue, fs::Permissions::from_mode(0o400)).unwrap();
        f.adopt().unwrap();
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
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert_eq!(f.current().manager.sha256, first.manager.sha256);
        f.adopt().unwrap();
        let next = f.current();
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        setup_install::interrupt_journaled_for_test(&f.base.m, &f.home, &next, Some(&first), true).unwrap();
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
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
        assert!(recover_from(&f.base.m, &f.home, &f.service).is_err());
        assert!(f.base.m.root.join("package-transition.json").exists());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(),
            serde_json::to_vec(&first).unwrap());
    }

    #[test]
    fn same_generation_repairs_owned_routes_without_new_generation() {
        for route in ["manager", "frontend", "desktop", "callback_desktop", "service", "association"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let current = f.current();
            let generation = current.manager.path.parent().unwrap().join("package-generation.json");
            let generation_before = fs::read(&generation).unwrap();
            let software = f.base.m.root.join("software.json");
            let software_before = fs::read(&software).unwrap();
            let paths = [
                f.home.join(".local/bin/linux-vst-bridge"),
                f.home.join(".local/bin/linux-audio-compatibility-manager"),
                f.home.join(".local/share/applications/linux-audio-compatibility-manager.desktop"),
                f.home.join(".local/share/applications/linux-vst-bridge-native-access.desktop"),
                f.home.join(".config/systemd/user/linux-vst-bridge.service"),
                f.home.join(".config/mimeapps.list"),
            ];
            let before: Vec<_> = paths.iter().map(|p| fs::read_link(p)
                .map(|target| target.into_os_string().into_encoded_bytes())
                .or_else(|_| fs::read(p))).collect::<std::io::Result<_>>().unwrap();
            let index = match route {
                "manager" => 0, "frontend" => 1, "desktop" => 2,
                "callback_desktop" => 3, "service" => 4, _ => 5,
            };
            if route == "service" {
                fs::write(&paths[index], b"[Unit]\nDescription=Linux VST Bridge registered host\n# stale package-owned unit\n").unwrap();
            } else if route == "association" {
                fs::write(&paths[index], b"[Default Applications]\n").unwrap();
            } else {
                fs::remove_file(&paths[index]).unwrap();
            }
            f.adopt().unwrap();
            let after: Vec<_> = paths.iter().map(|p| fs::read_link(p)
                .map(|target| target.into_os_string().into_encoded_bytes())
                .or_else(|_| fs::read(p))).collect::<std::io::Result<_>>().unwrap();
            assert_eq!(after, before, "{route}");
            assert_eq!(fs::read(&software).unwrap(), software_before, "{route}");
            assert_eq!(fs::read(&generation).unwrap(), generation_before, "{route}");
            assert_eq!(f.current().manager.path, current.manager.path);
            assert!(!f.base.m.root.join("package-transition.json").exists());
        }
    }

    #[test]
    fn same_generation_refuses_foreign_routes() {
        for route in ["manager", "desktop", "service"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let before = fs::read(f.base.m.root.join("software.json")).unwrap();
            let path = match route {
                "manager" => f.home.join(".local/bin/linux-vst-bridge"),
                "desktop" => f.home.join(".local/share/applications/linux-audio-compatibility-manager.desktop"),
                _ => f.home.join(".config/systemd/user/linux-vst-bridge.service"),
            };
            fs::remove_file(&path).unwrap();
            if route == "manager" {
                std::os::unix::fs::symlink("/usr/bin/foreign", &path).unwrap();
            } else { fs::write(&path, b"foreign route").unwrap(); }
            assert!(f.adopt().is_err(), "{route}");
            assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
        }
    }

    #[test]
    fn service_state_and_effective_reload_are_exact() {
        assert!(parse_unit_readback("LoadState=loaded\nActiveState=inactive\nFragmentPath=/x\nExecStart={ path=/x ; argv[]=/x serve ; ignore_errors=no }\n").is_ok());
        assert!(parse_unit_readback("LoadState=loaded\nActiveState=inactive\n").is_err());
        assert!(parse_unit_readback("LoadState=loaded\nLoadState=loaded\nActiveState=inactive\nFragmentPath=/x\nExecStart=/x\n").is_err());
        let f = Fixture::new();
        assert!(require_service_stopped(&f.service).is_ok());
        for active in ["active", "activating", "deactivating", "reloading", "unknown"] {
            f.service.loaded.borrow_mut().active = active.into();
            assert!(require_service_stopped(&f.service).is_err(), "{active}");
        }
        f.service.loaded.borrow_mut().active = "inactive".into();
        f.service.loaded.borrow_mut().load = "error".into();
        assert!(require_service_stopped(&f.service).is_err());
        f.service.loaded.borrow_mut().load = "not-found".into();
        f.service.fail_show.set(true);
        assert!(require_service_stopped(&f.service).is_err());
        f.service.fail_show.set(false);
        f.adopt().unwrap();
        assert_eq!(f.service.show().unwrap().load, "loaded");
        assert!(require_service_stopped(&f.service).is_ok());
        f.service.loaded.borrow_mut().active = "failed".into();
        assert!(require_service_stopped(&f.service).is_ok());
        f.service.loaded.borrow_mut().active = "inactive".into();
        f.replace("linux-vst-bridge", b"successor", true);
        f.service.fail_reload.set(true);
        assert!(f.adopt().is_err());
        assert!(f.base.m.root.join("package-transition.json").exists());
        f.service.fail_reload.set(false);
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        let selected = f.current();
        assert!(effective_exec_is(&f.service.show().unwrap().exec, &selected.manager.path));
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        let predecessor = f.current();
        assert!(effective_exec_is(&f.service.show().unwrap().exec, &predecessor.manager.path));
        assert_ne!(selected.manager.path, predecessor.manager.path);
        f.replace("linux-vst-bridge", b"third generation", true);
        f.service.stale_reload.set(true);
        assert!(f.adopt().is_err());
        assert!(f.base.m.root.join("package-transition.json").exists());
        f.service.stale_reload.set(false);
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert!(effective_exec_is(&f.service.show().unwrap().exec, &f.current().manager.path));
    }

    #[test]
    fn exact_noop_reload_failure_retains_recoverable_journal() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.service.fail_reload.set(true);
        assert!(f.adopt().is_err());
        assert!(f.base.m.root.join("package-transition.json").exists());
        f.service.fail_reload.set(false);
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
        assert!(!f.base.m.root.join("package-transition.json").exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "tools/pkg0/root_intake_fixture.py prepares the root-owned input"]
    fn root_owned_package_intake() {
        let path = PathBuf::from(std::env::var_os("PKG0_ROOT_INPUT_FIXTURE").unwrap());
        let expected = std::env::var("PKG0_ROOT_EXPECT").unwrap();
        let f = Fixture::new();
        let result = adopt_from(&f.base.m, &f.home, &Inputs::under(&path), 0, &f.service);
        assert_eq!(result.is_ok(), expected == "ok", "{result:?}");
        if expected == "ok" {
            assert!(f.current().manager.path.starts_with(f.base.m.root.join("software")));
            assert_eq!(f.current().preparation_kit.is_some(),
                std::env::var("PKG0_ROOT_EXPECT_KIT").unwrap() == "yes");
        } else {
            assert!(!f.base.m.root.join("software.json").exists());
        }
    }
}

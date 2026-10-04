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
    fn installed() -> Result<(Self, u32)> {
        let executable = std::env::current_exe()?;
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
        match linux_vst_bridge::portable_package::input_root(&executable, &home)? {
            Some(root) => Ok((Self {root}, unsafe{libc::getuid()})),
            None => Ok((Self {root:PathBuf::from(PACKAGE_ROOT)}, 0)),
        }
    }
    fn installed_record() -> Result<(Self, u32)> {
        let executable = std::env::current_exe()?;
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
        match linux_vst_bridge::portable_package::input_root_record(&executable, &home)? {
            Some(root) => Ok((Self { root }, unsafe { libc::getuid() })),
            None => Ok((Self { root: PathBuf::from(PACKAGE_ROOT) }, 0)),
        }
    }
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TransitionKind { Update, Restore }

const CONTROL_RECORD_MAX: usize = 8 * 1024 * 1024;

fn require_json_extent(value: &impl Serialize, reason: &str) -> Result<()> {
    let length = serde_json::to_vec(value)?.len()
        .checked_add(1).ok_or(reason)?;
    require(length <= CONTROL_RECORD_MAX, reason)
}

/// Immutable provenance for one completed managed package refresh. It records
/// the exact coherent pair on both sides; current selection remains owned by
/// software.json and the publication registry.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshReceipt {
    schema: u32,
    predecessor: Software,
    successor: Software,
    before_publications: Vec<publication::PublicationState>,
    after_publications: Vec<publication::PublicationState>,
    transitions: Vec<publication::PreparedTransition>,
}

/// The sole crash journal for a package/publication switch. The setup plan is
/// the existing route owner; publication transitions remain the existing
/// per-class owner. This record coordinates their safe order and recovery.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinatedTransition {
    schema: u32,
    id: String,
    kind: TransitionKind,
    recovery_before: Software,
    setup: setup_install::Plan,
    receipt: RefreshReceipt,
}

fn ordered_publications(states: &[publication::PublicationState]) -> bool {
    states.len() <= 256
        && states.windows(2).all(|pair| pair[0].class_id < pair[1].class_id)
}

fn validate_receipt(receipt: &RefreshReceipt) -> Result<()> {
    require(receipt.schema == 1
        && ordered_publications(&receipt.before_publications)
        && ordered_publications(&receipt.after_publications)
        && receipt.before_publications.len() == receipt.after_publications.len()
        && receipt.transitions.len() <= receipt.before_publications.len()
        && receipt.transitions.windows(2)
            .all(|pair| pair[0].before.class_id < pair[1].before.class_id),
        "package_refresh_receipt")?;
    let mut expected = receipt.before_publications.clone();
    for transition in &receipt.transitions {
        require(transition.schema == 2
            && transition.before.class_id == transition.after.class_id,
            "package_refresh_receipt")?;
        let state = expected.iter_mut().find(|state|
            state.class_id == transition.before.class_id)
            .ok_or("package_refresh_receipt")?;
        require(*state == transition.before, "package_refresh_receipt")?;
        *state = transition.after.clone();
    }
    require(expected == receipt.after_publications,
        "package_refresh_receipt")
}

fn validate_coordinated(m: &Manager, home: &Path,
    transition: &CoordinatedTransition) -> Result<()> {
    require(transition.schema == 2 && valid_hex(&transition.id, 32),
        "package_transition_schema")?;
    validate_receipt(&transition.receipt)?;
    setup_install::validate_plan(&transition.setup, m, home)?;
    let (before, after) = setup_install::plan_software(&transition.setup)?;
    let before = before.ok_or("package_transition_software")?;
    match transition.kind {
        TransitionKind::Update => require(before == transition.recovery_before
            && after == transition.receipt.successor,
            "package_transition_software"),
        TransitionKind::Restore => require(before == transition.receipt.successor
            && after == transition.receipt.predecessor
            && transition.recovery_before == transition.receipt.successor,
            "package_transition_software"),
    }?;
    require_json_extent(&transition.receipt, "package_refresh_receipt_bound")?;
    require_json_extent(transition, "package_transition_bound")
}

fn receipt_directory(m: &Manager, software: &Software) -> Result<PathBuf> {
    let generation = software.manager.path.parent().ok_or("package_generation_path")?;
    require(generation.parent() == Some(m.root.join("software").as_path()),
        "package_generation_path")?;
    Ok(generation.join("package-refreshes"))
}

fn retain_refresh_receipt(m: &Manager, receipt: &RefreshReceipt) -> Result<()> {
    validate_receipt(receipt)?;
    let mut bytes = serde_json::to_vec(receipt)?;
    bytes.push(b'\n');
    require(bytes.len() <= CONTROL_RECORD_MAX,
        "package_refresh_receipt_bound")?;
    let id = hex(&sha2::Sha256::digest(&bytes));
    let directory = receipt_directory(m, &receipt.successor)?;
    private_dir(&directory)?;
    let path = directory.join(format!("{id}.json"));
    if path.try_exists()? {
        require(fs::read(&path)? == bytes, "package_refresh_receipt_conflict")?;
        return Ok(());
    }
    atomic_json(&path, receipt)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o400))?;
    fs::File::open(&path)?.sync_all()?;
    fs::File::open(&directory)?.sync_all()?;
    Ok(())
}

fn matching_refresh_receipts(m: &Manager, selected: &Software,
    publications: &[publication::PublicationState]) -> Result<Vec<RefreshReceipt>> {
    let directory = receipt_directory(m, selected)?;
    let mut matching = Vec::new();
    if directory.try_exists()? {
        let mut count = 0usize;
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            count += 1;
            require(count <= 256 && entry.file_type()?.is_file(),
                "package_refresh_receipt_bound")?;
            let metadata = entry.metadata()?;
            require(metadata.uid() == unsafe { libc::getuid() }
                && metadata.mode() & 0o022 == 0
                && metadata.len() <= 8 * 1024 * 1024,
                "package_refresh_receipt_bound")?;
            let name = entry.file_name();
            let name = name.to_str().ok_or("package_refresh_receipt_identity")?;
            let digest_name = name.strip_suffix(".json")
                .ok_or("package_refresh_receipt_identity")?;
            require(valid_hex(digest_name, 64)
                && digest(&entry.path())? == digest_name,
                "package_refresh_receipt_identity")?;
            let receipt: RefreshReceipt = read_json(&entry.path())?;
            validate_receipt(&receipt)?;
            if receipt.successor == *selected
                && receipt.after_publications == publications {
                matching.push(receipt);
            }
        }
    }
    Ok(matching)
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
fn package_artifact_record(path: &Path, owner: u32, maximum: u64, expected: u64) -> Result<()> {
    let held = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let before = held.metadata()?;
    require(
        path.is_absolute()
            && before.is_file()
            && before.uid() == owner
            && before.mode() & 0o022 == 0
            && before.len() <= maximum
            && before.len() == expected,
        "package_artifact_owner_or_extent",
    )?;
    let after = held.metadata()?;
    let path_after = fs::symlink_metadata(path)?;
    require(
        before.dev() == after.dev()
            && before.ino() == after.ino()
            && before.len() == after.len()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
            && before.ctime() == after.ctime()
            && before.ctime_nsec() == after.ctime_nsec()
            && before.uid() == after.uid()
            && before.mode() == after.mode()
            && before.dev() == path_after.dev()
            && before.ino() == path_after.ino()
            && before.len() == path_after.len()
            && before.mtime() == path_after.mtime()
            && before.mtime_nsec() == path_after.mtime_nsec()
            && before.ctime() == path_after.ctime()
            && before.ctime_nsec() == path_after.ctime_nsec()
            && before.uid() == path_after.uid()
            && before.mode() == path_after.mode(),
        "package_artifact_changed_during_read",
    )
}
fn read_manifest_record(inputs: &Inputs, owner: u32) -> Result<(PackageManifest, String)> {
    let path = inputs.manifest();
    let (sha, _, bytes) = package_source(&path, owner, 64 * 1024, true)?;
    let manifest: PackageManifest = serde_json::from_slice(&bytes)?;
    validate_manifest_identity(&manifest)?;
    for (entry, name) in manifest.files.iter().zip(names(manifest.schema)?) {
        let maximum = if *name == "preparation-kit.zip" {
            256
        } else {
            128
        } * 1024
            * 1024;
        package_artifact_record(&inputs.path(name)?, owner, maximum, entry.size)?;
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
fn validate_software_record(m: &Manager, old: &Software) -> Result<()> {
    require(
        old.operator_frontend
            .as_ref()
            .is_some_and(|a| a.path.parent() == old.manager.path.parent()),
        "package_prior_pairing",
    )?;
    old.validate_record()?;
    for artifact in [
        &old.manager,
        &old.supervisor,
        &old.ownership,
        &old.host,
        &old.source_manifest,
    ]
    .into_iter()
    .chain(
        [
            &old.operator_frontend,
            &old.installer_launch,
            &old.preparation_kit,
            &old.native_catalogue,
        ]
        .into_iter()
        .flatten(),
    ) {
        require(
            artifact.path.starts_with(m.root.join("software"))
                && artifact.path.canonicalize()? == artifact.path
                && fs::metadata(&artifact.path)?.permissions().mode() & 0o222 == 0,
            "package_prior_software_identity",
        )?;
    }
    if old.native_catalogue.is_some() {
        old.catalogue_record(m)?;
    }
    Ok(())
}
fn old_software(m: &Manager) -> Result<Option<Software>> {
    let path = m.root.join("software.json");
    if !path.try_exists()? { return Ok(None); }
    let old = software(m)?;
    verify_software_identity(m, &old)?;
    Ok(Some(old))
}
fn old_software_record(m: &Manager) -> Result<Option<Software>> {
    let path = m.root.join("software.json");
    if !path.try_exists()? {
        return Ok(None);
    }
    let old = software_record(m)?;
    validate_software_record(m, &old)?;
    Ok(Some(old))
}
fn require_retained_host_pair(m: &Manager, old: &Software,
    manifest: &PackageManifest) -> Result<()> {
    if old.host.sha256 != manifest.files[4].sha256
        || old.source_manifest.sha256 != manifest.files[5].sha256 {
        // New preparation uses the successor's pair. Existing publications
        // retain the exact supervisor/ownership/host set that served them.
        for entry in m.registry()?.classes.values() {
            if entry.publication == Publication::Published {
                paired_components(m, old, &entry.registration)?;
            }
        }
    }
    Ok(())
}
fn require_retained_host_pair_record(
    m: &Manager,
    old: &Software,
    manifest: &PackageManifest,
) -> Result<()> {
    if old.host.sha256 != manifest.files[4].sha256
        || old.source_manifest.sha256 != manifest.files[5].sha256
    {
        for entry in m.registry()?.classes.values() {
            if entry.publication == Publication::Published {
                paired_components_record(m, old, &entry.registration)?;
            }
        }
    }
    Ok(())
}
fn retained_catalogue(m: &Manager, predecessor: Option<&Software>,
    manifest: &PackageManifest) -> Result<Option<Vec<u8>>> {
    let Some(old) = predecessor else { return Ok(None) };
    let Some(artifact) = &old.native_catalogue else { return Ok(None) };
    artifact.verify()?;
    let bytes = fs::read(&artifact.path)?;
    if old.host.sha256 == manifest.files[4].sha256
        && old.source_manifest.sha256 == manifest.files[5].sha256 {
        return Ok(Some(bytes));
    }
    let mut catalogue = old.catalogue(m)?;
    let retained = catalogue::HostArtifact {
        host: old.host.clone(), source_manifest: old.source_manifest.clone() };
    if !catalogue.hosts.iter().any(|host| host.host.sha256 == retained.host.sha256
        && host.source_manifest.sha256 == retained.source_manifest.sha256) {
        catalogue.hosts.push(retained);
    }
    catalogue.schema = catalogue.schema.max(2);
    catalogue.hosts.retain(|host| host.host.sha256 != manifest.files[4].sha256
        || host.source_manifest.sha256 != manifest.files[5].sha256);
    catalogue.validate(&m.root)?;
    Ok(Some(serde_json::to_vec(&catalogue)?))
}
fn generation_record(m: &Manager, manifest: PackageManifest, manifest_sha256: String,
    predecessor: Option<Software>) -> Result<Generation> {
    let catalogue = retained_catalogue(m, predecessor.as_ref(), &manifest)?;
    let packaged_preparation_kit_sha256 = (manifest.schema == 2)
        .then(|| manifest.files[NAMES.len()].sha256.clone());
    Ok(Generation {
        schema: if packaged_preparation_kit_sha256.is_some() { 2 } else { 1 },
        manifest_sha256, manifest,
        retained_installer_launch: predecessor.as_ref().and_then(|s| s.installer_launch.clone()),
        retained_preparation_kit: if packaged_preparation_kit_sha256.is_some() { None } else {
            predecessor.as_ref().and_then(|s| s.preparation_kit.clone())
        },
        catalogue_sha256: catalogue.map(|bytes| hex(&sha2::Sha256::digest(bytes))),
        packaged_preparation_kit_sha256,
        predecessor,
    })
}

/// Resolve a publication's complete retained execution set. A changed
/// package never silently pairs its new supervisor with an older host.
pub(super) fn paired_components(m: &Manager, selected: &Software,
    registration: &Registration) -> Result<Software> {
    paired_host_components(m, selected, &registration.host, &registration.host_source_sha256)
}
fn paired_components_record(
    m: &Manager,
    selected: &Software,
    registration: &Registration,
) -> Result<Software> {
    paired_host_components_record(
        m,
        selected,
        &registration.host,
        &registration.host_source_sha256,
    )
}
pub(super) fn paired_host_components(m: &Manager, selected: &Software,
    host: &Artifact, source_sha256: &str) -> Result<Software> {
    let mut candidate = selected.clone();
    let mut seen = std::collections::BTreeSet::new();
    let mut supplemental = None;
    for depth in 0..32 {
        verify_software_identity(m, &candidate)?;
        if candidate.host.sha256 == host.sha256 && candidate.source_sha256 == source_sha256 {
            return Ok(candidate);
        }
        // A copied catalogue preserves discovery authority, not permission to
        // combine a successor supervisor with an older host. Prefer the actual
        // predecessor set. A supplemental host belongs to the oldest retained
        // set that introduced it, including pre-package fixture installations.
        if let Some(retained) = candidate.native_catalogue.as_ref()
            .map(|_| candidate.catalogue(m)).transpose()?.and_then(|catalogue|
                catalogue.hosts.into_iter().find(|entry|
                    entry.host.sha256 == host.sha256
                        && entry.source_manifest.sha256 == source_sha256)) {
            let mut set = candidate.clone();
            set.host = retained.host;
            set.source_manifest = retained.source_manifest;
            set.source_sha256 = set.source_manifest.sha256.clone();
            supplemental = Some(set);
        }
        // Managed preparation hosts are selected by the immutable kit, not
        // necessarily listed in the static fixture catalogue. Follow only this
        // generation's exact kit and its verified staged runtime. Arbitrary
        // historical preparation directories provide no execution authority.
        if let Some(kit) = &candidate.preparation_kit {
            require(valid_hex(&kit.sha256, 64), "preparation_kit_identity")?;
            let record = m.root.join("software/preparation-kits")
                .join(&kit.sha256).join("runtime.json");
            if record.try_exists()? {
                let runtime = linux_vst_bridge::preparation::build::existing_runtime(m, &kit.sha256)?;
                if runtime.host.sha256 == host.sha256
                    && runtime.source_manifest.sha256 == source_sha256 {
                    let mut set = candidate.clone();
                    set.host = runtime.host;
                    set.source_manifest = runtime.source_manifest;
                    set.source_sha256 = set.source_manifest.sha256.clone();
                    supplemental = Some(set);
                }
            }
        }
        require(seen.insert(candidate.manager.path.clone()), "package_predecessor_cycle")?;
        let dir = candidate.manager.path.parent().ok_or("package_generation_path")?;
        if !dir.join("package-generation.json").try_exists()? { break; }
        let Some(predecessor) = verify_generation(m, &candidate)?.predecessor else { break; };
        require(depth < 31, "package_predecessor_bound")?;
        candidate = predecessor;
    }
    supplemental.ok_or_else(|| "publication_component_generation_unavailable".into())
}
fn paired_host_components_record(
    m: &Manager,
    selected: &Software,
    host: &Artifact,
    source_sha256: &str,
) -> Result<Software> {
    let mut candidate = selected.clone();
    let mut seen = std::collections::BTreeSet::new();
    let mut supplemental = None;
    for depth in 0..32 {
        validate_software_record(m, &candidate)?;
        if candidate.host.sha256 == host.sha256 && candidate.source_sha256 == source_sha256 {
            return Ok(candidate);
        }
        if let Some(retained) = candidate
            .native_catalogue
            .as_ref()
            .map(|_| candidate.catalogue_record(m))
            .transpose()?
            .and_then(|catalogue| {
                catalogue.hosts.into_iter().find(|entry| {
                    entry.host.sha256 == host.sha256
                        && entry.source_manifest.sha256 == source_sha256
                })
            })
        {
            let mut set = candidate.clone();
            set.host = retained.host;
            set.source_manifest = retained.source_manifest;
            set.source_sha256 = set.source_manifest.sha256.clone();
            supplemental = Some(set);
        }
        if let Some(kit) = &candidate.preparation_kit {
            require(valid_hex(&kit.sha256, 64), "preparation_kit_identity")?;
            let record = m
                .root
                .join("software/preparation-kits")
                .join(&kit.sha256)
                .join("runtime.json");
            if record.try_exists()? {
                let runtime =
                    linux_vst_bridge::preparation::build::existing_runtime_record(m, &kit.sha256)?;
                if runtime.host.sha256 == host.sha256
                    && runtime.source_manifest.sha256 == source_sha256
                {
                    let mut set = candidate.clone();
                    set.host = runtime.host;
                    set.source_manifest = runtime.source_manifest;
                    set.source_sha256 = set.source_manifest.sha256.clone();
                    supplemental = Some(set);
                }
            }
        }
        require(
            seen.insert(candidate.manager.path.clone()),
            "package_predecessor_cycle",
        )?;
        let dir = candidate
            .manager
            .path
            .parent()
            .ok_or("package_generation_path")?;
        if !dir.join("package-generation.json").try_exists()? {
            break;
        }
        let Some(predecessor) = validate_generation_record(m, &candidate)?.predecessor else {
            break;
        };
        require(depth < 31, "package_predecessor_bound")?;
        candidate = predecessor;
    }
    supplemental.ok_or_else(|| "publication_component_generation_unavailable".into())
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
    let record = generation_record(m, manifest, manifest_sha256, Some(old.clone()))?;
    let planned_id = id(&record)?;
    require(old.manager.path.parent().and_then(|p| p.file_name())
        .and_then(|p| p.to_str()) != Some(planned_id.as_str()),
        "package_self_predecessor")?;
    Ok((record, routes))
}
fn validate_predecessor_status(m: &Manager, manifest: &PackageManifest,
    manifest_sha256: &str) -> Result<()> {
    validate_manifest_identity(manifest)?;
    require(valid_hex(manifest_sha256, 64), "package_manifest_digest")?;
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let old = old_software_record(m)?.ok_or("package_predecessor_absent")?;
    require_retained_host_pair_record(m, &old, manifest)?;
    if old.manager.path.parent().is_some_and(|dir|
        dir.join("package-generation.json").exists()) {
        let selected = validate_generation_record(m, &old)?;
        if selected.manifest_sha256 == manifest_sha256 {
            require(selected.manifest == *manifest, "package_manifest_changed")?;
        }
    }
    Ok(())
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
fn validate_generation_record(m: &Manager, current: &Software) -> Result<Generation> {
    let dir = current
        .manager
        .path
        .parent()
        .ok_or("package_generation_path")?;
    require(
        dir.parent() == Some(m.root.join("software").as_path()),
        "package_generation_path",
    )?;
    existing_generation_dir(dir)?;
    let record_path = dir.join("package-generation.json");
    require(
        record_path.canonicalize()? == record_path
            && fs::metadata(&record_path)?.permissions().mode() & 0o222 == 0,
        "package_generation_record_writable",
    )?;
    let record: Generation = read_json(&record_path)?;
    require(
        matches!((record.schema, record.manifest.schema), (1, 1) | (2, 2))
            && id(&record)? == dir.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "package_generation_identity",
    )?;
    let expected_names = names(record.manifest.schema)?;
    require(
        record.manifest.package == "linux-vst-bridge-beta"
            && valid_package_version(&record.manifest.version)
            && record.manifest.pkgrel == 1
            && record.manifest.files.len() == expected_names.len()
            && record
                .manifest
                .files
                .iter()
                .zip(expected_names)
                .all(|(entry, name)| {
                    entry.name == *name
                        && valid_hex(&entry.sha256, 64)
                        && entry.size
                            <= (if *name == "preparation-kit.zip" {
                                256
                            } else {
                                128
                            }) * 1024
                                * 1024
                }),
        "package_generation_manifest",
    )?;
    let expected = [
        (&current.manager, NAMES[0]),
        (
            current
                .operator_frontend
                .as_ref()
                .ok_or("package_frontend_absent")?,
            NAMES[1],
        ),
        (&current.supervisor, NAMES[2]),
        (&current.ownership, NAMES[3]),
        (&current.host, NAMES[4]),
        (&current.source_manifest, NAMES[5]),
    ];
    for (index, (actual, name)) in expected.iter().enumerate() {
        require(
            actual.path == dir.join(name)
                && actual.sha256 == record.manifest.files[index].sha256
                && file(&actual.path)?.metadata()?.len() == record.manifest.files[index].size,
            "package_generation_artifact_binding",
        )?;
        actual.validate_record()?;
    }
    let packaged_kit = if record.schema == 2 {
        let entry = &record.manifest.files[NAMES.len()];
        require(
            record.packaged_preparation_kit_sha256.as_ref() == Some(&entry.sha256)
                && record.retained_preparation_kit.is_none(),
            "package_generation_kit_binding",
        )?;
        let kit = Artifact {
            path: dir.join("preparation-kit.zip"),
            sha256: entry.sha256.clone(),
        };
        kit.validate_record()?;
        require(
            file(&kit.path)?.metadata()?.len() == entry.size
                && fs::metadata(&kit.path)?.permissions().mode() & 0o222 == 0,
            "package_generation_kit_writable",
        )?;
        Some(kit)
    } else {
        require(
            record.packaged_preparation_kit_sha256.is_none(),
            "package_generation_kit_binding",
        )?;
        record.retained_preparation_kit.clone()
    };
    require(
        current.source_sha256 == current.source_manifest.sha256
            && current.installer_launch == record.retained_installer_launch
            && current.preparation_kit == packaged_kit
            && current.native_catalogue.as_ref().map(|a| a.sha256.as_str())
                == record.catalogue_sha256.as_deref(),
        "package_generation_software_binding",
    )?;
    if let Some(a) = &current.native_catalogue {
        require(
            a.path == dir.join("native-catalogue.json")
                && fs::metadata(&a.path)?.permissions().mode() & 0o222 == 0,
            "package_generation_catalogue_binding",
        )?;
        current.catalogue_record(m)?;
    }
    Ok(record)
}
pub(super) fn selected_package_version(m: &Manager, current: &Software) -> Result<Option<String>> {
    Ok(selected_generation(m, current)?.map(|record| record.manifest.version))
}
fn selected_generation(m: &Manager, current: &Software) -> Result<Option<Generation>> {
    let dir = current.manager.path.parent().ok_or("package_generation_path")?;
    match fs::symlink_metadata(dir.join("package-generation.json")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Only the retained pre-package helper layout has no package record.
            // A missing or damaged modern record must never become legacy.
            require(current.supervisor.path == dir.join("session.py")
                && current.ownership.path == dir.join("ownership.py"),
                "package_generation_record_missing")?;
            existing_generation_dir(dir)?;
            verify_software_identity(m, current)?;
            Ok(None)
        }
        Err(error) => Err(error.into()),
        Ok(metadata) => {
            require(metadata.is_file() && !metadata.file_type().is_symlink()
                && metadata.uid() == unsafe { libc::getuid() }
                && metadata.mode() & 0o222 == 0,
                "package_generation_record_changed")?;
            Ok(Some(verify_generation(m, current)?))
        }
    }
}
fn selected_generation_record(m: &Manager, current: &Software) -> Result<Option<Generation>> {
    let dir = current
        .manager
        .path
        .parent()
        .ok_or("package_generation_path")?;
    match fs::symlink_metadata(dir.join("package-generation.json")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            require(
                current.supervisor.path == dir.join("session.py")
                    && current.ownership.path == dir.join("ownership.py"),
                "package_generation_record_missing",
            )?;
            existing_generation_dir(dir)?;
            validate_software_record(m, current)?;
            Ok(None)
        }
        Err(error) => Err(error.into()),
        Ok(metadata) => {
            require(
                metadata.is_file()
                    && !metadata.file_type().is_symlink()
                    && metadata.uid() == unsafe { libc::getuid() }
                    && metadata.mode() & 0o222 == 0,
                "package_generation_record_changed",
            )?;
            Ok(Some(validate_generation_record(m, current)?))
        }
    }
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
    let catalogue = retained_catalogue(m, predecessor.as_ref(), &manifest)?;
    let record = generation_record(m, manifest, manifest_sha256, predecessor)?;
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
            if let Some(bytes) = &catalogue {
                let target = scratch.join("native-catalogue.json");
                fs::write(&target, bytes)?;
                require(Some(digest(&target)?) == record.catalogue_sha256,
                    "package_catalogue_copy_changed")?;
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
    transport_storage::require_no_sessions()?;
    Ok(())
}
fn recovery_preflight(m: &Manager,
    transition: &CoordinatedTransition) -> Result<()> {
    require(!reconcile_leases(m)?, "package_cleanup_unconfirmed")?;
    m.require_inactive(None)?;
    require(capacity::owners(m)?.is_empty(), "package_owner_active")?;
    m.verify_package_pending_intents(&transition.receipt.transitions)?;
    require(onboarding::all_retired(m)?, "package_installer_owner_active")?;
    require(operator_cli::vendor_retired(m)?, "package_vendor_owner_active")?;
    daw_workspace::package_idle(m)?;
    transport_storage::require_no_sessions()?;
    Ok(())
}
fn with_locks<T>(m: &Manager, work: impl FnOnce() -> Result<T>) -> Result<T> {
    let _package = m.lock("package.lock")?;
    let _service = m.lock("service.lock")?;
    let _setup = m.lock("setup.lock")?;
    let _registry = m.lock("registry.lock")?;
    work()
}

fn publications_need_refresh(m: &Manager) -> Result<bool> {
    preparation::registry_requires_loaded_engine_refresh(m)
}
fn retained_publication_ancestor(m: &Manager, key: &str, id: &str)
    -> Result<(publication::RevisionRef, publication::Revision)> {
    require(valid_hex(key, 32) && valid_hex(id, 32), "rollback_identity")?;
    let entry = m.registry()?.classes.get(key).cloned()
        .ok_or("registration_absent")?;
    require(entry.publication == Publication::Published,
        "rollback_publication_not_selected")?;
    let current = entry.managed_revision.ok_or("no_managed_revision")?;
    let mut reference = Some(current.clone());
    let mut selected = None;
    for _ in 0..256 {
        let Some(candidate) = reference else { break };
        let revision = m.load_revision(key, &candidate)?;
        if revision.id == id {
            selected = Some(revision);
            break;
        }
        reference = revision.parent.clone();
    }
    Ok((current, selected.ok_or("rollback_revision_not_retained_ancestor")?))
}

pub(super) fn publication_restore_needs_refresh(m: &Manager, key: &str,
    id: &str) -> Result<bool> {
    let (_, target) = retained_publication_ancestor(m, key, id)?;
    Ok(!preparation::build::revision_supports_loaded_engine_admission(m, &target)
        .unwrap_or(false))
}

/// Ordinary history restoration keeps the retained vendor/configuration but
/// never republishes an LVB1-4 image under the repaired manager. The existing
/// preparation and publication owners create a fresh managed revision whose
/// parent is the currently selected revision; the historical target remains
/// immutable evidence.
pub(super) fn restore_publication(m: &Manager, key: &str, id: &str,
    operation: &str) -> Result<publication::RevisionRef> {
    let _package = m.lock("package.lock")?;
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let (current, target) = retained_publication_ancestor(m, key, id)?;
    if preparation::build::revision_supports_loaded_engine_admission(m, &target)
        .unwrap_or(false) {
        return m.rollback_managed_expected(key, id, &current);
    }
    m.require_inactive(None)?;
    let selected = software(m)?;
    verify_software_identity(m, &selected)?;
    let runtime = preparation::build::stage_runtime_for_software(m, &selected)?;
    require(preparation::build::runtime_declares_loaded_engine(&runtime)?,
        "loaded_engine_admission_contract_missing")?;
    let report = inspect_for_package_refresh(m, &selected, &runtime,
        &target.registration)?;
    let prepared = preparation::refresh_candidate(m, &target, runtime,
        report, operation);
    let cleanup = preparation::build::cleanup_work(m, operation);
    let candidate = prepared?;
    cleanup?;
    preparation::replace_refreshed(m, &candidate, &target, &current)
}

fn prepare_refresh_receipt(m: &Manager, target: &Software,
    predecessor: Software) -> Result<RefreshReceipt> {
    let before = m.package_publication_snapshot()?;
    // The fixed population bound is 256. Each class gets the supervisor's
    // 190-second outer inspection bound plus the existing 1,250-second build
    // owner and retirement allowance; setup/staging gets a separate base.
    // Schema-4 reusable engines normally only copy bytes, but the authority
    // bound remains valid if that helper fails to return promptly.
    let published = before.iter().filter(|state|
        state.entry.publication == Publication::Published).count() as u64;
    let refresh_seconds = 120u64.checked_add(published.checked_mul(1_500)
        .ok_or("package_refresh_deadline")?).ok_or("package_refresh_deadline")?;
    let deadline = std::time::Instant::now().checked_add(
        std::time::Duration::from_secs(refresh_seconds))
        .ok_or("package_refresh_deadline")?;
    let runtime = preparation::build::stage_runtime_for_software(m, target)?;
    require(preparation::build::runtime_declares_loaded_engine(&runtime)?,
        "loaded_engine_admission_contract_missing")?;
    let mut transitions = Vec::new();
    for state in &before {
        require(std::time::Instant::now() < deadline,
            "package_refresh_deadline")?;
        if state.entry.publication != Publication::Published { continue; }
        let reference = state.entry.managed_revision.as_ref()
            .ok_or("bridge_refresh_managed_predecessor_missing")?;
        let revision = m.load_revision(&state.class_id, reference)?;
        if !preparation::publication_requires_refresh(m, &revision)? {
            continue;
        }
        let report = inspect_for_package_refresh(m, target, &runtime,
            &revision.registration)?;
        let operation = random_id()?;
        let prepared = preparation::refresh_candidate(m, &revision,
            runtime.clone(), report, &operation);
        let cleanup = preparation::build::cleanup_work(m, &operation);
        let candidate = prepared?;
        cleanup?;
        transitions.push(m.prepare_package_refresh(&candidate, reference)?);
        require(std::time::Instant::now() < deadline,
            "package_refresh_deadline")?;
    }
    transitions.sort_by(|left, right| left.before.class_id.cmp(&right.before.class_id));
    let mut after = before.clone();
    for transition in &transitions {
        let state = after.iter_mut().find(|state|
            state.class_id == transition.before.class_id)
            .ok_or("package_publication_set_changed")?;
        require(*state == transition.before, "package_publication_set_changed")?;
        *state = transition.after.clone();
    }
    m.verify_package_publication_snapshot(&before)?;
    let receipt = RefreshReceipt { schema:1, predecessor,
        successor:target.clone(), before_publications:before,
        after_publications:after, transitions };
    validate_receipt(&receipt)?;
    Ok(receipt)
}

fn start_and_health(service: &impl ServiceControl, m: &Manager, home: &Path,
    selected: &Software) -> Result<()> {
    reload_and_verify(service, home, Some(selected))?;
    service.enable_start()?;
    let state = service.show()?;
    require(state.load == "loaded" && state.active == "active"
        && effective_exec_is(&state.exec, &selected.manager.path),
        "package_service_did_not_start")?;
    for _ in 0..20 {
        if service.healthy(m).is_ok() { return Ok(()); }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    service.healthy(m)
}

fn stop_transition_service(m: &Manager, home: &Path,
    service: &impl ServiceControl, software: &[&Software],
    publications: &[publication::PreparedTransition]) -> Result<()> {
    let state = service.show()?;
    if matches!(state.active.as_str(), "active" | "activating") {
        require(state.load == "loaded"
            && state.fragment == home.join(".config/systemd/user/linux-vst-bridge.service")
                .to_str().ok_or("package_service_path")?
            && software.iter().any(|selected|
                effective_exec_is(&state.exec, &selected.manager.path)),
            "package_active_service_identity_changed")?;
        // An active service has a live capacity endpoint whose readback must
        // agree with durable custody. A manager refusing a mixed journal can
        // remain in systemd's activating/auto-restart state before creating
        // that socket; only exact durable absence can authorize stopping it.
        let observed = (state.active == "active").then(|| service.idle(m)).transpose()?;
        {
            let _registry = m.lock("registry.lock")?;
            m.require_inactive(None)?;
            let owners = capacity::owners(m)?;
            if let Some(observed) = observed {
                require(owners == observed, "package_owners_changed")?;
            } else {
                require(owners.is_empty(), "package_service_not_clean_idle")?;
            }
            m.verify_package_pending_intents(publications)?;
            let current = service.show()?;
            require(current.load == state.load && current.active == state.active
                && current.fragment == state.fragment && current.exec == state.exec,
                "package_active_service_identity_changed")?;
            service.stop()?;
        }
    }
    reconcile_stopped_service(m, service)
}

fn apply_coordinated(m: &Manager, transition: &CoordinatedTransition) -> Result<()> {
    match transition.kind {
        TransitionKind::Update => {
            setup_install::apply_plan(&transition.setup)?;
            for publication in &transition.receipt.transitions {
                m.commit_package_publication(publication)?;
            }
            require(transition.setup.is_after()?, "package_transition_incomplete")?;
            m.verify_package_publication_snapshot(
                &transition.receipt.after_publications)
        }
        TransitionKind::Restore => {
            for publication in transition.receipt.transitions.iter().rev() {
                m.restore_package_publication(publication)?;
            }
            m.verify_package_publication_snapshot(
                &transition.receipt.before_publications)?;
            setup_install::apply_restore_plan(&transition.setup)
        }
    }
}

fn restore_coordinated_origin(m: &Manager,
    transition: &CoordinatedTransition) -> Result<()> {
    match transition.kind {
        TransitionKind::Update => {
            for publication in transition.receipt.transitions.iter().rev() {
                m.restore_package_publication(publication)?;
            }
            m.verify_package_publication_snapshot(
                &transition.receipt.before_publications)?;
            setup_install::restore_plan(&transition.setup)?;
            require(transition.setup.is_before()?, "package_transition_recovery_incomplete")
        }
        TransitionKind::Restore => {
            setup_install::restore_successor_plan(&transition.setup)?;
            for publication in &transition.receipt.transitions {
                m.commit_package_publication(publication)?;
            }
            require(transition.setup.is_before()?, "package_transition_recovery_incomplete")?;
            m.verify_package_publication_snapshot(
                &transition.receipt.after_publications)
        }
    }
}

fn coordinated_final_software(transition: &CoordinatedTransition) -> &Software {
    match transition.kind {
        TransitionKind::Update => &transition.receipt.successor,
        TransitionKind::Restore => &transition.receipt.predecessor,
    }
}

fn coordinated_origin_publications(transition: &CoordinatedTransition)
    -> &[publication::PublicationState] {
    match transition.kind {
        TransitionKind::Update => &transition.receipt.before_publications,
        TransitionKind::Restore => &transition.receipt.after_publications,
    }
}

fn require_service_transition_coherent_at(m: &Manager, selected: &Software,
    home: &Path) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    if !journal.try_exists()? { return Ok(()); }
    let value: serde_json::Value = read_json(&journal)?;
    require(value.get("schema").and_then(serde_json::Value::as_u64) == Some(2),
        "package_transition_needs_recovery")?;
    let transition: CoordinatedTransition = serde_json::from_value(value)?;
    validate_coordinated(m, home, &transition)?;
    let publications = m.package_publication_snapshot()?;
    let coherent = match transition.kind {
        TransitionKind::Update => (transition.setup.is_before()?
            && *selected == transition.recovery_before
            && publications == transition.receipt.before_publications)
            || (transition.setup.is_after()?
                && *selected == transition.receipt.successor
                && publications == transition.receipt.after_publications),
        TransitionKind::Restore => (transition.setup.is_before()?
            && *selected == transition.receipt.successor
            && publications == transition.receipt.after_publications)
            || (transition.setup.is_after()?
                && *selected == transition.receipt.predecessor
                && publications == transition.receipt.before_publications),
    };
    require(coherent, "package_transition_needs_recovery")
}

pub(super) fn require_service_transition_coherent(m: &Manager,
    selected: &Software) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    require_service_transition_coherent_at(m, selected, &home)
}

fn complete_coordinated(m: &Manager, home: &Path, service: &impl ServiceControl,
    transition: &CoordinatedTransition) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    apply_coordinated(m, transition)?;
    let selected = coordinated_final_software(transition);
    start_and_health(service, m, home, selected)?;
    if transition.kind == TransitionKind::Update {
        retain_refresh_receipt(m, &transition.receipt)?;
    }
    fs::remove_file(&journal)?;
    fs::File::open(&m.root)?.sync_all()?;
    Ok(())
}

fn recover_coordinated_error(m: &Manager, home: &Path,
    service: &impl ServiceControl, transition: &CoordinatedTransition,
    original: Box<dyn std::error::Error + Send + Sync>) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    let recovery = (|| -> Result<()> {
        stop_transition_service(m, home, service,
            &[&transition.recovery_before, coordinated_final_software(transition)],
            &transition.receipt.transitions)?;
        restore_coordinated_origin(m, transition)?;
        start_and_health(service, m, home, &transition.recovery_before)?;
        m.verify_package_publication_snapshot(
            coordinated_origin_publications(transition))?;
        fs::remove_file(&journal)?;
        fs::File::open(&m.root)?.sync_all()?;
        Ok(())
    })();
    match recovery {
        Ok(()) => Err(original),
        Err(recovery) => Err(format!(
            "{original}; package recovery failed: {recovery}").into()),
    }
}

fn run_coordinated(m: &Manager, home: &Path, service: &impl ServiceControl,
    transition: CoordinatedTransition, restart_prior: bool) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    let prepared = (|| -> Result<()> {
        validate_coordinated(m, home, &transition)?;
        require(!journal.try_exists()?, "package_transition_needs_recovery")?;
        Ok(())
    })();
    if let Err(original) = prepared {
        return restart_prior_after_error(m, home, service,
            &transition.recovery_before, original, restart_prior);
    }
    if let Err(original) = atomic_json(&journal, &transition) {
        if journal.try_exists()? {
            return recover_coordinated_error(m, home, service,
                &transition, original);
        }
        return restart_prior_after_error(m, home, service,
            &transition.recovery_before, original, restart_prior);
    }
    match complete_coordinated(m, home, service, &transition) {
        Ok(()) => Ok(()),
        Err(original) => recover_coordinated_error(m, home, service,
            &transition, original),
    }
}

fn restart_prior_after_error<T>(m: &Manager, home: &Path,
    service: &impl ServiceControl, prior: &Software,
    original: Box<dyn std::error::Error + Send + Sync>,
    required: bool) -> Result<T> {
    if !required { return Err(original); }
    match start_and_health(service, m, home, prior) {
        Ok(()) => Err(original),
        Err(recovery) => Err(format!(
            "{original}; prior service restart failed: {recovery}").into()),
    }
}

fn stop_for_coordinated(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl, prior: &Software) -> Result<bool> {
    let before = service.show()?;
    let was_active = before.active == "active";
    match stop_for_repair_locked(m, home, inputs, owner, service) {
        Ok(()) => Ok(was_active),
        Err(original) => {
            let after = service.show().map_err(|observation| format!(
                "{original}; prior service state unavailable: {observation}"))?;
            let stopped = matches!(after.active.as_str(), "inactive" | "failed");
            restart_prior_after_error(m, home, service, prior, original,
                was_active && stopped)
        }
    }
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
            let _ = predecessor_plan(m, home, manifest.clone(), sha.clone())?;
            if old.manager.path.parent()
                .is_some_and(|dir| dir.join("package-generation.json").exists()) {
                let old_record = verify_generation(m, old)?;
                if old_record.manifest_sha256 == sha {
                    require(old_record.manifest == manifest, "package_manifest_changed")?;
                    return setup_install::commit_journaled(m, home, old, Some(old),
                        |selected| reload_and_verify(service, home, selected));
                }
            }
            require(!m.registry()?.classes.values().any(|entry|
                entry.publication == Publication::Published),
                "package_update_required: use Update Bridge")?;
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
        require(!m.registry()?.classes.values().any(|entry|
            entry.publication == Publication::Published),
            "package_restore_required: use Restore previous setup")?;
        let current = old_software(m)?.ok_or("package_not_installed")?;
        let record = verify_generation(m, &current)?;
        let old = record.predecessor.ok_or("package_no_predecessor")?;
        verify_software_identity(m, &old)?;
        setup_install::commit_journaled(m, home, &old, Some(&current),
            |selected| reload_and_verify(service, home, selected))
    })
}
fn update_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    let current = old_software(m)?.ok_or("package_not_installed")?;
    let restart_prior = stop_for_coordinated(m, home, inputs, owner, service, &current)?;
    let prepared = (|| -> Result<CoordinatedTransition> {
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m)?;
        let (manifest, sha) = read_manifest(inputs, owner)?;
        require_retained_host_pair(m, &current, &manifest)?;
        let target = if current.manager.path.parent()
            .is_some_and(|dir| dir.join("package-generation.json").exists()) {
            let selected = verify_generation(m, &current)?;
            if selected.manifest_sha256 == sha {
                require(selected.manifest == manifest, "package_manifest_changed")?;
                current.clone()
            } else {
                let _ = predecessor_plan(m, home, manifest.clone(), sha.clone())?;
                stage(m, inputs, manifest, sha, Some(current.clone()))?
            }
        } else {
            let _ = predecessor_plan(m, home, manifest.clone(), sha.clone())?;
            stage(m, inputs, manifest, sha, Some(current.clone()))?
        };
        let needs_refresh = publications_need_refresh(m)?;
        require(target != current || needs_refresh,
            "package_update_not_required")?;
        let predecessor = if target == current && needs_refresh {
            verify_generation(m, &current)?.predecessor
                .ok_or("package_restore_predecessor_unavailable")?
        } else { current.clone() };
        verify_software_identity(m, &predecessor)?;
        let receipt = prepare_refresh_receipt(m, &target, predecessor)?;
        for state in &receipt.before_publications {
            if state.entry.publication == Publication::Published {
                let _ = paired_components(m, &receipt.predecessor,
                    &state.entry.registration)?;
            }
        }
        let setup = setup_install::plan(m, home, &target, Some(&current))?;
        Ok(CoordinatedTransition { schema:2, id:random_id()?,
            kind:TransitionKind::Update, recovery_before:current.clone(),
            setup, receipt })
    })();
    let transition = match prepared {
        Ok(transition) => transition,
        Err(original) => {
            return restart_prior_after_error(m, home, service, &current,
                original, restart_prior);
        }
    };
    run_coordinated(m, home, service, transition, restart_prior)
}

fn restore_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    let current = old_software(m)?.ok_or("package_not_installed")?;
    let restart_prior = stop_for_coordinated(m, home, inputs, owner, service, &current)?;
    let prepared = (|| -> Result<CoordinatedTransition> {
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m)?;
        let snapshot = m.package_publication_snapshot()?;
        let matching = matching_refresh_receipts(m, &current, &snapshot)?;
        require(matching.len() <= 1, "package_restore_receipt_ambiguous")?;
        let receipt = if let Some(receipt) = matching.into_iter().next() {
            receipt
        } else {
            require(!snapshot.iter().any(|state|
                state.entry.publication == Publication::Published),
                "package_restore_receipt_unavailable")?;
            let predecessor = verify_generation(m, &current)?.predecessor
                .ok_or("package_no_predecessor")?;
            RefreshReceipt { schema:1, predecessor, successor:current.clone(),
                before_publications:snapshot.clone(), after_publications:snapshot,
                transitions:Vec::new() }
        };
        verify_software_identity(m, &receipt.predecessor)?;
        let setup = setup_install::plan(m, home, &receipt.predecessor, Some(&current))?;
        Ok(CoordinatedTransition { schema:2, id:random_id()?,
            kind:TransitionKind::Restore, recovery_before:current.clone(),
            setup, receipt })
    })();
    let transition = match prepared {
        Ok(transition) => transition,
        Err(original) => {
            return restart_prior_after_error(m, home, service, &current,
                original, restart_prior);
        }
    };
    run_coordinated(m, home, service, transition, restart_prior)
}

fn recover_from(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<bool> {
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    let journal = m.root.join("package-transition.json");
    if !journal.try_exists()? { return Ok(false); }
    let value: serde_json::Value = read_json(&journal)?;
    if value.get("schema").and_then(serde_json::Value::as_u64) != Some(2) {
        let plan = setup_install::pending_plan(m, home)?;
        let (before, after) = setup_install::plan_software(&plan)?;
        let mut allowed = vec![&after];
        if let Some(before) = before.as_ref() { allowed.push(before); }
        stop_transition_service(m, home, service, &allowed, &[])?;
        preflight(m)?;
        let recovered = setup_install::recover_journaled(m, home,
            |selected| reload_and_verify(service, home, selected))?;
        if let Some(selected) = old_software(m)? {
            start_and_health(service, m, home, &selected)?;
        }
        return Ok(recovered);
    }
    let transition: CoordinatedTransition = serde_json::from_value(value)?;
    validate_coordinated(m, home, &transition)?;
    stop_transition_service(m, home, service,
        &[&transition.recovery_before, coordinated_final_software(&transition)],
        &transition.receipt.transitions)?;
    recovery_preflight(m, &transition)?;
    for publication in &transition.receipt.transitions {
        m.reconcile_package_pending_intent(publication)?;
    }
    require(operator_cli::pending_transactions(m)? == 0,
        "package_transaction_pending")?;
    let final_publications = match transition.kind {
        TransitionKind::Update => &transition.receipt.after_publications,
        TransitionKind::Restore => &transition.receipt.before_publications,
    };
    let final_state = transition.setup.is_after()?
        && m.package_publication_snapshot()? == *final_publications;
    if final_state {
        start_and_health(service, m, home, coordinated_final_software(&transition))?;
        if transition.kind == TransitionKind::Update {
            retain_refresh_receipt(m, &transition.receipt)?;
        }
    } else {
        restore_coordinated_origin(m, &transition)?;
        start_and_health(service, m, home, &transition.recovery_before)?;
    }
    fs::remove_file(&journal)?;
    fs::File::open(&m.root)?.sync_all()?;
    Ok(true)
}
#[derive(Clone, Debug)]
struct UnitReadback { load: String, active: String, fragment: String, exec: String }
trait ServiceControl {
    fn show(&self) -> Result<UnitReadback>;
    fn reload(&self) -> Result<()>;
    fn enable_start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn healthy(&self, m: &Manager) -> Result<()>;
    fn idle(&self, m: &Manager) -> Result<Vec<capacity::Owner>>;
}
fn stoppable_keepers(reply: &serde_json::Value) -> Result<Vec<capacity::Owner>> {
    require(reply["ok"] == true, "package_service_not_clean_idle")?;
    let c = &reply["capacity"];
    let owners: Vec<capacity::Owner> = serde_json::from_value(c["owners"].clone())?;
    let mut sessions = std::collections::BTreeSet::new();
    require(c["schema"] == 1 && c["dsp"] == 0 && c["maintenance"] == 0
        // Unconfirmed retirement blocks adoption, not stopping the exact
        // selected service. DSP, maintenance, transactions and other owners
        // still prohibit this idle-service stop.
        && c["cleanup_unconfirmed"].is_boolean()
        && c["keepers"].as_u64() == Some(owners.len() as u64)
        && owners.iter().all(|owner| owner.kind == capacity::Kind::Keeper
            && owner.terminal.is_none()
            && valid_hex(&owner.session, 32) && valid_hex(&owner.class_id, 32)
            && sessions.insert(&owner.session)),
        "package_service_not_clean_idle")?;
    Ok(owners)
}
struct SystemctlService;
fn systemctl_bounded(args: &[&str], capture: bool) -> Result<Vec<u8>> {
    systemctl_bounded_for(args, capture, 100)
}
fn systemctl_bounded_for(args: &[&str], capture: bool, attempts: usize) -> Result<Vec<u8>> {
    let mut child = Command::new("systemctl").args(args)
        .stdin(std::process::Stdio::null())
        .stdout(if capture { std::process::Stdio::piped() } else { std::process::Stdio::null() })
        .stderr(std::process::Stdio::null()).spawn()?;
    for _ in 0..attempts {
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
    fn stop(&self) -> Result<()> {
        systemctl_bounded_for(&["--user", "stop", "linux-vst-bridge.service"], false, 800).map(|_| ())
    }
    fn healthy(&self, m: &Manager) -> Result<()> {
        require(capacity_reply(m)?["ok"] == true, "package_service_health_unavailable")
    }
    fn idle(&self, m: &Manager) -> Result<Vec<capacity::Owner>> {
        stoppable_keepers(&capacity_reply(m)?)
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
    let load = get("LoadState")?;
    let active = get("ActiveState")?;
    let fragment = get("FragmentPath")?;
    // systemd omits ExecStart altogether for a genuinely absent user unit.
    // Admit that exact first-adoption readback without allowing a loaded unit
    // to lose its executable identity.
    let exec = match fields.get("ExecStart") {
        Some(value) => *value,
        None if load == "not-found" && active == "inactive" && fragment.is_empty() => "",
        None => return Err("package_user_service_readback".into()),
    };
    Ok(UnitReadback { load: load.into(), active: active.into(),
        fragment: fragment.into(), exec: exec.into() })
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
    service: &impl ServiceControl) -> Result<(Software, String, UnitReadback)> {
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let selected = old_software(m)?.ok_or("package_not_installed")?;
    let version = selected_generation(m, &selected)?.map(|record| record.manifest.version)
        .unwrap_or_else(|| "retained-installation".into());
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
    Ok((selected, version, state))
}
fn selected_activation_record(m: &Manager, home: &Path,
    service: &impl ServiceControl) -> Result<(Software, String, UnitReadback)> {
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let selected = old_software_record(m)?.ok_or("package_not_installed")?;
    let version = selected_generation_record(m, &selected)?
        .map(|record| record.manifest.version)
        .unwrap_or_else(|| "retained-installation".into());
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
    Ok((selected, version, state))
}
fn activation_status_from(m: &Manager, home: &Path,
    service: &impl ServiceControl) -> Result<ActivationStatus> {
    let (selected, version, state) = selected_activation_record(m, home, service)?;
    if state.active == "active" { service.healthy(m)?; }
    let (again, second, after) = selected_activation_record(m, home, service)?;
    require(selected == again && version == second
        && state.active == after.active, "package_activation_status_changed")?;
    Ok(ActivationStatus { schema: 1,
        state: if state.active == "active" { "active" } else { "inactive" },
        package_version: version })
}
/// First-run is a read-only classification. In particular an absent package
/// record is admitted as legacy only for the older selected helper layout;
/// a damaged package generation cannot become an adoptable predecessor.
fn bootstrap_status_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<ActivationStatus> {
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let Some(selected) = old_software_record(m)? else {
        let (manifest, _) = read_manifest_record(inputs, owner)?;
        let state = service.show()?;
        require(state.load == "not-found" && state.active == "inactive"
            && state.fragment.is_empty() && state.exec.is_empty(),
            "package_unselected_service_ambiguous")?;
        return Ok(ActivationStatus { schema: 3, state: "fresh_adoptable",
            package_version: manifest.version });
    };
    let generation = selected_generation_record(m, &selected)?;
    let legacy = generation.is_none();
    let refresh_required = preparation::registry_requires_loaded_engine_refresh_record(m)?;
    let (manifest, sha) = read_manifest_record(inputs, owner)?;
    let (version, update_available, rollback_available) = if legacy {
        require_retained_host_pair_record(m, &selected, &manifest)?;
        validate_predecessor_status(m, &manifest, &sha)?;
        (manifest.version, false, false)
    } else {
        let generation = generation.ok_or("package_generation_record_missing")?;
        if let Some(predecessor) = &generation.predecessor {
            validate_software_record(m, predecessor)?;
        }
        let current_publications = m.package_publication_record_snapshot()?;
        let receipts = matching_refresh_receipts(m, &selected, &current_publications)?;
        require(receipts.len() <= 1, "package_restore_receipt_ambiguous")?;
        let has_published = current_publications.iter().any(|state|
            state.entry.publication == Publication::Published);
        let rollback_available = receipts.len() == 1
            || (!has_published && generation.predecessor.is_some());
        // Status binds the owned fixed roster to this bounded manifest and
        // leaves payload certification to the selected mutation.
        if sha == generation.manifest_sha256 {
            require(manifest == generation.manifest, "package_manifest_changed")?;
            (generation.manifest.version, refresh_required, rollback_available)
        } else {
            validate_predecessor_status(m, &manifest, &sha)?;
            (manifest.version, true, rollback_available)
        }
    };
    let routes = setup_install::current_route_statuses(m, home, &selected)?;
    require(routes.len() == 6, "package_route_plan_shape")?;
    let routes_exact = routes.iter().all(|route| route.status == "exact");
    let state = service.show()?;
    let effective_exact = state.load == "loaded"
        && state.fragment == home.join(".config/systemd/user/linux-vst-bridge.service")
            .to_str().ok_or("package_service_path")?
        && effective_exec_is(&state.exec, &selected.manager.path);
    let posture = match state.active.as_str() {
        "active" => {
            require(effective_exact, "package_active_service_identity_changed")?;
            if legacy || update_available || rollback_available || !routes_exact {
                stop_gate(m, service)?;
            }
            if legacy { "legacy_active" }
            else if update_available { "update_active" }
            else if !routes_exact { "repair_active" }
            else if rollback_available { "rollback_active" }
            else { "active" }
        }
        "inactive" | "failed" if (state.load == "loaded"
            && !state.fragment.is_empty() && !state.exec.is_empty())
            || (state.load == "not-found" && state.active == "inactive"
                && state.fragment.is_empty() && state.exec.is_empty()) => {
            let keeper_retirement_pending = {
                let _registry = m.lock("registry.lock")?;
                let _unconfirmed = reconcile_leases(m)?;
                m.require_inactive(None)?;
                let owners = capacity::owners(m)?;
                require(owners.iter().all(|owner| owner.kind == capacity::Kind::Keeper),
                    "package_owner_active")?;
                !owners.is_empty()
            };
            if legacy && keeper_retirement_pending { "legacy_retirement_pending" }
            else if update_available && keeper_retirement_pending { "update_retirement_pending" }
            else if routes_exact && rollback_available && keeper_retirement_pending {
                "rollback_retirement_pending"
            }
            else if keeper_retirement_pending { "repair_retirement_pending" }
            else if legacy { "legacy_adoptable" }
            else if update_available { "update_adoptable" }
            else if routes_exact && effective_exact && rollback_available { "rollback_inactive" }
            else if routes_exact && effective_exact { "inactive" }
            else { "repair_inactive" }
        }
        _ => return Err("package_user_service_ambiguous".into()),
    };
    Ok(ActivationStatus { schema: 3, state: posture, package_version: version })
}
fn stop_gate(m: &Manager, service: &impl ServiceControl) -> Result<Vec<capacity::Owner>> {
    let before = service.idle(m)?;
    let latest = m.root.join("operator/latest.json");
    if latest.try_exists()? {
        let operation: serde_json::Value = read_json(&latest)?;
        require(matches!(operation["state"].as_str(), Some("completed" | "refused")),
            "package_operator_active_or_ambiguous")?;
    }
    let operations = m.root.join("operator");
    if operations.try_exists()? {
        let mut count = 0usize;
        for entry in fs::read_dir(&operations)? {
            let entry = entry?;
            count += 1;
            require(count <= 2048, "package_operator_inventory_bound")?;
            let name = entry.file_name();
            let Some(id) = name.to_str() else { return Err("package_operator_identity".into()); };
            if !valid_hex(id, 32) { continue; }
            require(entry.file_type()?.is_dir(), "package_operator_identity")?;
            let operation: serde_json::Value = read_json(&entry.path().join("result.json"))?;
            require(matches!(operation["state"].as_str(), Some("completed" | "refused"))
                && operation["operation"] == id, "package_operator_active_or_ambiguous")?;
        }
    }
    {
        let _registry = m.lock("registry.lock")?;
        m.require_inactive(None)?;
        require(capacity::owners(m)? == before, "package_owners_changed")?;
        require(operator_cli::pending_transactions(m)? == 0,
            "package_transaction_pending")?;
    }
    require(onboarding::all_retired(m)?, "package_installer_owner_active")?;
    require(operator_cli::vendor_retired(m)?, "package_vendor_owner_active")?;
    daw_workspace::package_idle(m)?;
    transport_storage::require_no_sessions()?;
    let after = service.idle(m)?;
    require(after == before, "package_owners_changed")?;
    Ok(after)
}
fn stop_selected_service(m: &Manager, home: &Path, selected: &Software,
    service: &impl ServiceControl,
    observed_keepers: &[capacity::Owner]) -> Result<()> {
    // Bounded systemctl stop intentionally runs under the admission lock,
    // matching operator suspension. Read-only status probes never do this.
    let _registry = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(capacity::owners(m)? == observed_keepers, "package_owners_changed")?;
    require(operator_cli::pending_transactions(m)? == 0,
        "package_transaction_pending")?;
    let unit = service.show()?;
    require(unit.load == "loaded" && unit.active == "active"
        && unit.fragment == home.join(".config/systemd/user/linux-vst-bridge.service")
            .to_str().ok_or("package_service_path")?
        && effective_exec_is(&unit.exec, &selected.manager.path),
        "package_active_service_identity_changed")?;
    service.stop()?;
    // A stopped unit is not enough: each keeper needs exact retirement proof
    // before its lease can be removed or adoption offered.
    reconcile_stopped_service(m, service)?;
    require(capacity::owners(m)?.is_empty(), "package_owner_active")
}
fn reconcile_stopped_service(m: &Manager, service: &impl ServiceControl) -> Result<()> {
    require_service_stopped(service)?;
    if reconcile_leases(m)? {
        if observe_stopped_leases(m, kernel_boot()?.as_deref())? {
            return Err("package_restart_required".into());
        }
        return Err("package_cleanup_unconfirmed".into());
    }
    Ok(())
}
fn stop_for_repair_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    // The selected service owns service.lock until it stops. Hold package and
    // setup selection steady without taking service.lock. The final bounded
    // stop alone holds registry.lock to exclude a new DSP admission.
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    stop_for_repair_locked(m, home, inputs, owner, service)
}
fn stop_for_repair_locked(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    let (_, package_sha) = read_manifest(inputs, owner)?;
    let status = bootstrap_status_from(m, home, inputs, owner, service)?;
    require(matches!(status.state, "legacy_active" | "repair_active" | "update_active"
        | "rollback_active"
        | "legacy_adoptable" | "repair_inactive" | "update_adoptable"
        | "rollback_inactive" | "rollback_retirement_pending"
        | "legacy_retirement_pending" | "repair_retirement_pending"
        | "update_retirement_pending"), "package_stop_not_offered")?;
    if matches!(status.state, "legacy_adoptable" | "repair_inactive" | "update_adoptable"
        | "rollback_inactive") { return Ok(()); }
    if matches!(status.state, "legacy_retirement_pending" | "repair_retirement_pending"
        | "update_retirement_pending" | "rollback_retirement_pending") {
        let _registry = m.lock("registry.lock")?;
        m.require_inactive(None)?;
        reconcile_stopped_service(m, service)?;
        require(capacity::owners(m)?.is_empty(), "package_owner_active")?;
        drop(_registry);
        let after = bootstrap_status_from(m, home, inputs, owner, service)?;
        require(read_manifest(inputs, owner)?.1 == package_sha,
            "package_input_changed_during_stop")?;
        return require(after.package_version == status.package_version
            && matches!((status.state, after.state),
                ("legacy_retirement_pending", "legacy_adoptable")
                | ("repair_retirement_pending", "repair_inactive")
                | ("repair_retirement_pending", "inactive")
                | ("update_retirement_pending", "update_adoptable")
                | ("rollback_retirement_pending", "rollback_inactive")),
            "package_service_did_not_stop_cleanly");
    }
    let before = bootstrap_status_from(m, home, inputs, owner, service)?;
    require(read_manifest(inputs, owner)?.1 == package_sha
        && before.state == status.state && before.package_version == status.package_version,
        "package_stop_state_changed")?;
    let observed_keepers = service.idle(m)?;
    let selected = old_software(m)?.ok_or("package_not_installed")?;
    stop_selected_service(m, home, &selected, service, &observed_keepers)?;
    let after = bootstrap_status_from(m, home, inputs, owner, service)?;
    require(read_manifest(inputs, owner)?.1 == package_sha,
        "package_input_changed_during_stop")?;
    require(after.package_version == status.package_version
        && matches!((status.state, after.state),
        ("legacy_active", "legacy_adoptable") | ("repair_active", "repair_inactive")
        | ("update_active", "update_adoptable")
        | ("rollback_active", "rollback_inactive")),
        "package_service_did_not_stop_cleanly")
}
fn activate_from(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<()> {
    // A package action gate excludes a concurrent package switch. setup.lock
    // excludes legacy route changes. The service must acquire service.lock on
    // startup, so neither service.lock nor registry.lock spans systemctl.
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    let (selected, version, state) = selected_activation(m, home, service)?;
    let starting = state.active != "active";
    if starting {
        preflight(m)?;
        let (checked, checked_version, checked_state) = selected_activation(m, home, service)?;
        require(checked == selected && checked_version == version
            && checked_state.active != "active", "package_activation_state_changed")?;
        service.enable_start()?;
    }
    let (after, after_version, readback) = selected_activation(m, home, service)?;
    require(after == selected && after_version == version
        && readback.active == "active", "package_service_did_not_start")?;
    if starting {
        for _ in 0..20 {
            if service.healthy(m).is_ok() { return Ok(()); }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    service.healthy(m)?;
    Ok(())
}
pub(super) fn adopt(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    let (inputs, owner) = Inputs::installed()?;
    adopt_from(m, &home, &inputs, owner, &SystemctlService)
}
pub(super) fn rollback(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    rollback_from(m, &home, &SystemctlService)
}
pub(super) fn update(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    let (inputs, owner) = Inputs::installed()?;
    update_from(m, &home, &inputs, owner, &SystemctlService)
}
pub(super) fn restore(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    let (inputs, owner) = Inputs::installed()?;
    restore_from(m, &home, &inputs, owner, &SystemctlService)
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
pub(super) fn bootstrap_status(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    let (inputs, owner) = Inputs::installed_record()?;
    println!("{}", serde_json::to_string(&bootstrap_status_from(m, &home,
        &inputs, owner, &SystemctlService)?)?);
    Ok(())
}
pub(super) fn stop_for_repair(m: &Manager) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    let (inputs, owner) = Inputs::installed()?;
    stop_for_repair_from(m, &home, &inputs, owner, &SystemctlService)
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
        fail_starts: Cell<usize>,
        fail_health: Cell<bool>,
        fail_idle: Cell<bool>,
        fail_stop: Cell<bool>,
        retire_keepers: Cell<bool>,
        starts: Cell<usize>,
        stops: Cell<usize>,
    }
    impl FakeService {
        fn new(home: &Path) -> Self {
            Self { home: home.into(), loaded: RefCell::new(UnitReadback {
                load: "not-found".into(), active: "inactive".into(),
                fragment: String::new(), exec: String::new(),
            }), fail_reload: Cell::new(false), fail_show: Cell::new(false),
                stale_reload: Cell::new(false), fail_start: Cell::new(false),
                fail_starts: Cell::new(0),
                fail_health: Cell::new(false), fail_idle: Cell::new(false),
                fail_stop: Cell::new(false), retire_keepers: Cell::new(true),
                starts: Cell::new(0), stops: Cell::new(0) }
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
            if self.fail_starts.get() > 0 {
                self.fail_starts.set(self.fail_starts.get() - 1);
                return Err("package_user_service_unavailable".into());
            }
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
        fn idle(&self, m: &Manager) -> Result<Vec<capacity::Owner>> {
            require(!self.fail_idle.get(), "package_service_not_clean_idle")?;
            let _registry = m.lock("registry.lock")?;
            let owners = capacity::owners(m)?;
            require(owners.iter().all(|owner| owner.kind == capacity::Kind::Keeper),
                "package_service_not_clean_idle")?;
            Ok(owners)
        }
        fn stop(&self) -> Result<()> {
            require(!self.fail_stop.get(), "package_user_service_unavailable")?;
            require(matches!(self.loaded.borrow().active.as_str(), "active" | "activating"),
                "package_stop_state_changed")?;
            self.loaded.borrow_mut().active = "inactive".into();
            self.stops.set(self.stops.get() + 1);
            if self.retire_keepers.get() {
                let leases = self.home.parent().ok_or("test_home")?.join("managed/runtime/leases");
                if leases.try_exists()? {
                    for entry in fs::read_dir(leases)? {
                        let report: PathBuf = read_json(&entry?.path())?;
                        atomic_json(&report, &serde_json::json!({"ready":false,
                            "cleanup_confirmed":true}))?;
                    }
                }
            }
            Ok(())
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
            Self::with_base(test_fixture::Fixture::new())
        }
        fn with_base(base: test_fixture::Fixture) -> Self {
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
                let old_name = match name {
                    "session.pyc" => "session.py",
                    "ownership.pyc" => "ownership.py",
                    _ => name,
                };
                let path = dir.join(old_name);
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
                supervisor: artifact(&dir, "session.py").unwrap(),
                ownership: artifact(&dir, "ownership.py").unwrap(),
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

    fn owned_lease(f: &Fixture, session: &str, keeper: bool) -> PathBuf {
        let report = f.base.m.root.join("runtime/results").join(format!(
            "{}-{session}.json", if keeper { "environment" } else { "windows" }));
        private_dir(report.parent().unwrap()).unwrap();
        atomic_json(&report, &serde_json::json!({"ready":true,
            "environment":f.base.r.environment.id})).unwrap();
        let owner = f.base.r.environment.root
            .join("compatdata/pfx/drive_c/bridge/sessions")
            .join(session).join("owner.json");
        private_dir(owner.parent().unwrap()).unwrap();
        atomic_json(&owner, &serde_json::json!({"session":session,"report":report,
            "keeper":keeper,"inspect":keeper,"vendor_access":false,
            "registration":{"metadata":{"class_id":f.base.r.metadata.class_id}}})).unwrap();
        let lease = f.base.m.root.join("runtime/leases").join(format!("{session}.json"));
        private_dir(lease.parent().unwrap()).unwrap();
        atomic_json(&lease, &report).unwrap();
        lease
    }

    #[test]
    fn capacity_classifier_accepts_only_exact_stoppable_keepers() {
        let keeper = serde_json::json!({"session":"ab".repeat(16),
            "class_id":"01".repeat(16),"kind":"keeper"});
        let second = serde_json::json!({"session":"cd".repeat(16),
            "class_id":"01".repeat(16),"kind":"keeper"});
        let reply = serde_json::json!({"ok":true,"capacity":{"schema":1,
            "dsp":0,"maintenance":0,"keepers":2,"cleanup_unconfirmed":false,
            "owners":[keeper.clone(),second.clone()]}});
        assert_eq!(stoppable_keepers(&reply).unwrap().len(), 2);
        let mut interrupted = reply.clone();
        interrupted["capacity"]["cleanup_unconfirmed"] = true.into();
        assert_eq!(stoppable_keepers(&interrupted).unwrap().len(), 2);
        for changed in [
            serde_json::json!({"capacity":{"schema":1,"dsp":0,"maintenance":0,
                "keepers":2,"cleanup_unconfirmed":false,"owners":[keeper.clone(),second.clone()]}}),
            serde_json::json!({"ok":true,"capacity":{"schema":1,"dsp":1,"maintenance":0,
                "keepers":2,"cleanup_unconfirmed":false,"owners":[keeper.clone(),second.clone()]}}),
            serde_json::json!({"ok":true,"capacity":{"schema":1,"dsp":0,"maintenance":0,
                "keepers":1,"cleanup_unconfirmed":false,"owners":[keeper.clone(),second.clone()]}}),
            serde_json::json!({"ok":true,"capacity":{"schema":1,"dsp":0,"maintenance":0,
                "keepers":2,"cleanup_unconfirmed":"unknown","owners":[keeper.clone(),second.clone()]}}),
            serde_json::json!({"ok":true,"capacity":{"schema":1,"dsp":0,"maintenance":0,
                "keepers":2,"cleanup_unconfirmed":false,"owners":[keeper.clone(),keeper.clone()]}}),
            serde_json::json!({"ok":true,"capacity":{"schema":1,"dsp":0,"maintenance":0,
                "keepers":1,"cleanup_unconfirmed":false,
                "owners":[{"session":"cd".repeat(16),"class_id":"01".repeat(16),"kind":"dsp"}]}}),
        ] { assert!(stoppable_keepers(&changed).is_err()); }
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
    fn first_run_classifies_verified_legacy_and_explicitly_stops_its_idle_service() {
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
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "legacy_adoptable");
        let manifest = f.inputs.manifest();
        fs::set_permissions(&manifest, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        fs::set_permissions(&manifest, fs::Permissions::from_mode(0o444)).unwrap();
        f.service.loaded.borrow_mut().active = "active".into();
        let first_keeper = owned_lease(&f, &"ab".repeat(16), true);
        let second_keeper = owned_lease(&f, &"cd".repeat(16), true);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "legacy_active");
        stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
        assert_eq!(f.service.stops.get(), 1);
        assert!(!first_keeper.exists() && !second_keeper.exists());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "legacy_adoptable");
        assert!(f.adopt().unwrap_err().to_string().contains("package_update_required"));
        assert!(update_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service)
            .unwrap_err().to_string().contains("Preparation support is missing"));
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
    }

    #[test]
    fn active_selected_generation_repairs_only_after_explicit_clean_stop() {
        let f = Fixture::new();
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "fresh_adoptable");
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let first_keeper = owned_lease(&f, &"ab".repeat(16), true);
        let second_keeper = owned_lease(&f, &"cd".repeat(16), true);
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        let route = f.home.join(".local/bin/linux-vst-bridge");
        fs::remove_file(&route).unwrap();
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "repair_active");
        f.service.fail_idle.set(true);
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
        f.service.fail_idle.set(false);
        stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
        assert_eq!(f.service.stops.get(), 1);
        assert!(!first_keeper.exists() && !second_keeper.exists());
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "repair_inactive");
        f.adopt().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "inactive");
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "active");
    }

    #[test]
    fn installed_successor_is_offered_and_adopted_only_after_exact_clean_stop() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let predecessor = f.current();
        let predecessor_bytes = fs::read(f.base.m.root.join("software.json")).unwrap();
        let registry = fs::read(f.base.m.root.join("registry.json")).ok();
        let old_unit = fs::read(f.home.join(".config/systemd/user/linux-vst-bridge.service")).unwrap();
        f.replace("linux-vst-bridge", b"successor manager", true);
        f.replace("linux-audio-compatibility-manager", b"successor frontend", true);
        let mut manifest = f.manifest();
        manifest.version = "0.2.0beta1".into();
        atomic_json(&f.inputs.manifest(), &manifest).unwrap();
        fs::set_permissions(f.inputs.manifest(), fs::Permissions::from_mode(0o444)).unwrap();

        let status = bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap();
        assert_eq!((status.schema, status.state, status.package_version.as_str()),
            (3, "update_active", "0.2.0beta1"));
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), predecessor_bytes);
        assert_eq!(fs::read(f.home.join(".config/systemd/user/linux-vst-bridge.service")).unwrap(), old_unit);
        assert!(f.adopt().is_err());
        f.service.fail_idle.set(true);
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
        f.service.fail_idle.set(false);
        stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap();
        assert_eq!(f.service.stops.get(), 1);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "update_adoptable");
        f.adopt().unwrap();
        let successor = f.current();
        assert_ne!(successor.manager.path.parent(), predecessor.manager.path.parent());
        assert_eq!(verify_generation(&f.base.m, &successor).unwrap()
            .predecessor.unwrap().manager, predecessor.manager);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).ok(), registry);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "rollback_inactive");
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "rollback_active");
        f.service.fail_idle.set(true);
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 1);
        f.service.fail_idle.set(false);
        predecessor.manager.verify().unwrap();
        successor.manager.verify().unwrap();
        // A package update and its exact software rollback remain separate
        // from the still-installed /usr package bytes.
        stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap();
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "rollback_inactive");
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), predecessor_bytes);
        assert_eq!(activation_status_from(&f.base.m, &f.home, &f.service).unwrap().state,
            "inactive");
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(activation_status_from(&f.base.m, &f.home, &f.service).unwrap().state,
            "active");
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).ok(), registry);
    }

    #[test]
    fn rollback_offer_refuses_changed_exact_predecessor() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let predecessor = f.current();
        f.replace("linux-vst-bridge", b"successor manager", true);
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let selected = fs::read(f.base.m.root.join("software.json")).unwrap();
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "rollback_active");
        fs::set_permissions(&predecessor.manager.path,
            fs::Permissions::from_mode(0o644)).unwrap();
        fs::write(&predecessor.manager.path, b"changed predecessor").unwrap();
        assert!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected);
        assert_eq!(f.service.stops.get(), 0);
    }

    #[test]
    fn changed_installed_package_artifact_refuses_status_without_stopping_service() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        f.replace("linux-audio-compatibility-manager", b"changed without manifest", false);
        assert!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
    }

    #[test]
    fn bootstrap_status_defers_bulk_hash_while_mutation_refuses_changed_payload() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let selected = f.current();
        let bytes = fs::read(&selected.manager.path).unwrap();
        fs::set_permissions(&selected.manager.path,
            fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(&selected.manager.path, vec![b'X'; bytes.len()]).unwrap();
        fs::set_permissions(&selected.manager.path,
            fs::Permissions::from_mode(0o500)).unwrap();

        let status = bootstrap_status_from(&f.base.m, &f.home, &f.inputs,
            f.owner, &f.service).unwrap();
        assert_eq!((status.state, status.package_version.as_str()),
            ("inactive", "0.1.0beta1"));
        let selected_status = activation_status_from(&f.base.m, &f.home,
            &f.service).unwrap();
        assert_eq!((selected_status.state, selected_status.package_version.as_str()),
            ("inactive", "0.1.0beta1"));
        assert!(activate_from(&f.base.m, &f.home, &f.service).unwrap_err()
            .to_string().contains("artifact missing or changed"));
        assert!(f.adopt().unwrap_err().to_string()
            .contains("artifact missing or changed"));
        assert_eq!(f.service.starts.get(), 0);
        assert_eq!(f.service.stops.get(), 0);
    }

    #[test]
    fn installed_successor_waits_for_exact_keeper_retirement() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let predecessor = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.replace("linux-vst-bridge", b"successor manager", true);
        let keeper = owned_lease(&f, &"ab".repeat(16), true);
        f.service.retire_keepers.set(false);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "update_active");
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 1);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "update_retirement_pending");
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), predecessor);
        assert!(f.adopt().is_err());
        let report: PathBuf = read_json(&keeper).unwrap();
        atomic_json(&report, &serde_json::json!({"ready":false,
            "cleanup_confirmed":true})).unwrap();
        stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap();
        assert_eq!(f.service.stops.get(), 1);
        assert!(!keeper.exists());
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "update_adoptable");
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), predecessor);
    }

    #[test]
    #[cfg(target_os="linux")]
    fn interrupted_keeper_restart_observation_preserves_populated_predecessor() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let selected = fs::read(f.base.m.root.join("software.json")).unwrap();
        f.replace("linux-vst-bridge", b"successor manager", true);
        let session = "ab".repeat(16);
        let lease = owned_lease(&f, &session, true);
        let report: PathBuf = read_json(&lease).unwrap();
        let original = fs::read(&report).unwrap();
        let generation = f.base.m.root.join("runtime/lease-generations")
            .join(format!("{session}.json"));
        // A running service cannot acquire restart observation authority.
        assert!(reconcile_stopped_service(&f.base.m, &f.service).is_err());
        assert!(!generation.exists());
        f.service.retire_keepers.set(false);
        assert_eq!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap_err().to_string(), "package_restart_required");
        let record: serde_json::Value = read_json(&generation).unwrap();
        assert_eq!(record["basis"], "stopped_service_observation");
        assert!(lease.exists());
        assert!(f.adopt().is_err());
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected);
        let other_boot = if record["kernel_boot"] == "11111111-1111-1111-1111-111111111111" {
            "22222222-2222-2222-2222-222222222222"
        } else { "11111111-1111-1111-1111-111111111111" };
        assert!(!reconcile_leases_in_kernel(&f.base.m, Some(other_boot)).unwrap());
        assert!(!lease.exists());
        assert_eq!(fs::read(&report).unwrap(), original);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "update_adoptable");
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected);
    }

    #[test]
    fn stopped_keeper_cleanup_remains_pending_until_exact_positive_retirement() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let selected = fs::read(f.base.m.root.join("software.json")).unwrap();
        let route = f.home.join(".local/bin/linux-vst-bridge");
        fs::remove_file(&route).unwrap();
        let keeper = owned_lease(&f, &"ab".repeat(16), true);
        f.service.retire_keepers.set(false);
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 1);
        assert!(keeper.exists());
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "repair_retirement_pending");
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected);
        assert!(!route.exists());
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        let report: PathBuf = read_json(&keeper).unwrap();
        atomic_json(&report, &serde_json::json!({"ready":false,
            "cleanup_confirmed":true})).unwrap();
        stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap();
        assert!(!keeper.exists());
        assert_eq!(f.service.stops.get(), 1);
        assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap().state, "repair_inactive");
        f.adopt().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), selected);
    }

    #[test]
    fn fresh_dsp_admission_after_idle_readback_refuses_before_service_stop() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let keeper = owned_lease(&f, &"ab".repeat(16), true);
        let observed = f.service.idle(&f.base.m).unwrap();
        assert_eq!(observed.len(), 1);
        let dsp = owned_lease(&f, &"cd".repeat(16), false);
        assert!(stop_selected_service(&f.base.m, &f.home, &f.current(),
            &f.service, &observed).is_err());
        assert_eq!(f.service.stops.get(), 0);
        assert_eq!(f.service.loaded.borrow().active, "active");
        assert!(keeper.exists() && dsp.exists());
    }

    #[test]
    fn bootstrap_refuses_broken_generation_foreign_route_and_foreign_live_service() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let selected = f.current();
        let record = selected.manager.path.parent().unwrap().join("package-generation.json");
        fs::remove_file(&record).unwrap();
        assert!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let route = f.home.join(".local/bin/linux-vst-bridge");
        fs::remove_file(&route).unwrap();
        std::os::unix::fs::symlink("/usr/bin/foreign", &route).unwrap();
        assert!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
        fs::remove_file(&route).unwrap();
        f.service.loaded.borrow_mut().exec =
            "{ path=/usr/bin/foreign ; argv[]=/usr/bin/foreign serve ; ignore_errors=no }".into();
        assert!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
    }

    #[test]
    fn active_desktop_and_service_route_repairs_are_explicit_and_exact() {
        for route in ["desktop", "service"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            activate_from(&f.base.m, &f.home, &f.service).unwrap();
            let path = if route == "desktop" {
                f.home.join(".local/share/applications/linux-audio-compatibility-manager.desktop")
            } else { f.home.join(".config/systemd/user/linux-vst-bridge.service") };
            let before = fs::read(&path).unwrap();
            if route == "desktop" { fs::remove_file(&path).unwrap(); }
            else { fs::write(&path, b"[Unit]\nDescription=Linux VST Bridge registered host\n# stale owned unit\n").unwrap(); }
            assert_eq!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
                &f.service).unwrap().state, "repair_active", "{route}");
            assert!(f.adopt().is_err(), "{route}");
            stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
            f.adopt().unwrap();
            assert_eq!(fs::read(&path).unwrap(), before, "{route}");
            assert_eq!(f.service.stops.get(), 1, "{route}");
        }
    }

    #[test]
    fn stop_for_repair_refuses_live_operation_without_stopping() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        fs::remove_file(f.home.join(".local/bin/linux-vst-bridge")).unwrap();
        let latest = f.base.m.root.join("operator/latest.json");
        fs::create_dir_all(latest.parent().unwrap()).unwrap();
        atomic_json(&latest, &serde_json::json!({"schema":1,"operation":"ab".repeat(16),
            "state":"vendor_running"})).unwrap();
        assert!(bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
        assert_eq!(f.service.loaded.borrow().active, "active");
        atomic_json(&latest, &serde_json::json!({"schema":1,"operation":"cd".repeat(16),
            "state":"completed"})).unwrap();
        let older = f.base.m.root.join("operator").join("ab".repeat(16));
        fs::create_dir_all(&older).unwrap();
        atomic_json(&older.join("result.json"), &serde_json::json!({"schema":1,
            "operation":"ab".repeat(16),"state":"running"})).unwrap();
        assert!(stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).is_err());
        assert_eq!(f.service.stops.get(), 0);
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
        assert!(activation_status_from(&f.base.m, &f.home, &f.service).is_err());
        assert_eq!(f.service.starts.get(), 1);
        f.service.fail_health.set(false);
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(f.service.starts.get(), 1);
    }

    #[test]
    fn missing_modern_generation_record_cannot_authorize_legacy_restart() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        let current = f.current();
        fs::remove_file(current.manager.path.parent().unwrap()
            .join("package-generation.json")).unwrap();
        assert!(selected_package_version(&f.base.m, &current).is_err());
        assert!(activation_status_from(&f.base.m, &f.home, &f.service).is_err());
        assert!(activate_from(&f.base.m, &f.home, &f.service).is_err());
        assert_eq!(f.service.starts.get(), 0);
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
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
        assert!(f.adopt().unwrap_err().to_string().contains("package_update_required"));
        let selected_before = fs::read(f.base.m.root.join("software.json")).unwrap();
        assert_eq!(selected_before, software_before);
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
    fn predecessor_plan_classifies_owned_routes_and_plans_changed_host() {
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
        let changed_sha = hex(&sha2::Sha256::digest(serde_json::to_vec(&changed_host).unwrap()));
        let (planned, _) = predecessor_plan(&f.base.m, &f.home, changed_host, changed_sha).unwrap();
        assert_eq!(planned.predecessor.unwrap().host, f.current().host);
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
    fn populated_legacy_kit_and_catalogue_cannot_bypass_managed_update() {
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
        assert!(f.adopt().unwrap_err().to_string().contains("package_update_required"));
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), old_bytes);
        old.preparation_kit.as_ref().unwrap().verify().unwrap();
        assert_eq!(fs::read(old.native_catalogue.as_ref().unwrap().path.clone()).unwrap(),
            catalogue_bytes);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
        assert!(!old.manager.path.parent().unwrap().join("package-generation.json").exists());
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
        assert_eq!(f.service.show().unwrap().active, "active");
        f.service.stop().unwrap();
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
        atomic_json(&f.base.m.root.join("registry.json"), &Registry::default()).unwrap();
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
        f.replace("host.exe", b"successor host", true);
        f.replace("host-source-manifest.json", b"successor host source", true);
        f.replace("session.pyc", b"successor supervisor", true);
        f.replace("ownership.pyc", b"successor ownership", true);
        f.adopt().unwrap();
        let changed = f.current();
        assert_ne!(changed.host.sha256, next.host.sha256);
        let execution = paired_components(&f.base.m, &changed, &f.base.r).unwrap();
        assert_eq!(execution.host, next.host);
        assert_eq!(execution.supervisor, next.supervisor);
        assert_eq!(execution.ownership, next.ownership);
        assert_ne!(execution.supervisor, changed.supervisor);
        assert_eq!(changed.catalogue(&f.base.m).unwrap().hosts[0].host, next.host);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
        for (path, bytes) in &retained { assert_eq!(&fs::read(path).unwrap(), bytes); }
        // Projects created after an update belong to the customer as well.
        let later = f.base.m.root.join("projects/later.bwproject");
        fs::write(&later, b"later music").unwrap();
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(f.current().supervisor, next.supervisor);
        assert_eq!(fs::read(later).unwrap(), b"later music");
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(f.current().manager.sha256, first.manager.sha256);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry_bytes);
        for (path, bytes) in &retained { assert_eq!(&fs::read(path).unwrap(), bytes); }
        old.native_catalogue.unwrap().verify().unwrap();
    }

    #[test]
    fn retained_execution_refuses_missing_pair_and_tampered_predecessor() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let old = f.current();
        let mut registration = f.base.r.clone();
        registration.host = old.host.clone();
        registration.host_source_sha256 = old.source_sha256.clone();
        f.replace("host.exe", b"successor host", true);
        f.replace("host-source-manifest.json", b"successor source", true);
        f.replace("session.pyc", b"successor supervisor", true);
        f.adopt().unwrap();
        let next = f.current();
        assert_eq!(paired_components(&f.base.m, &next, &registration).unwrap().supervisor, old.supervisor);
        let mut unavailable = registration.clone();
        unavailable.host.sha256 = "fe".repeat(32);
        assert!(paired_components(&f.base.m, &next, &unavailable).is_err());
        fs::set_permissions(&old.supervisor.path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&old.supervisor.path, b"tampered supervisor").unwrap();
        assert!(paired_components(&f.base.m, &next, &registration).is_err());
        assert_eq!(f.current().host, next.host);
    }

    #[test]
    fn retained_preparation_pair_keeps_its_original_components_across_package_update() {
        let f = Fixture::new();
        f.add_kit(b"original selected preparation kit");
        f.adopt().unwrap();
        let old = f.current();
        let kit = old.preparation_kit.clone().unwrap();
        let dir = f.base.m.root.join("software/preparation-kits").join(&kit.sha256);
        private_dir(&dir).unwrap();
        for (name, bytes) in [("host.exe", b"prepared host".as_slice()),
            ("host-source-manifest.json", b"prepared host source".as_slice())] {
            fs::write(dir.join(name), bytes).unwrap();
            fs::set_permissions(dir.join(name), fs::Permissions::from_mode(0o444)).unwrap();
        }
        let runtime = linux_vst_bridge::preparation::build::Runtime { kit,
            host: artifact(&dir, "host.exe").unwrap(),
            source_manifest: artifact(&dir, "host-source-manifest.json").unwrap(),
            builder: None, generator: None };
        atomic_json(&dir.join("runtime.json"), &runtime).unwrap();
        fs::set_permissions(dir.join("runtime.json"), fs::Permissions::from_mode(0o444)).unwrap();
        let mut registration = f.base.r.clone();
        registration.host = runtime.host.clone();
        registration.host_source_sha256 = runtime.source_manifest.sha256.clone();
        let execution = paired_components(&f.base.m, &old, &registration).unwrap();
        assert_eq!(execution.supervisor, old.supervisor);
        assert_eq!(execution.host, runtime.host);
        assert!(old.native_catalogue.is_none());

        f.replace("host.exe", b"successor host", true);
        f.replace("host-source-manifest.json", b"successor source", true);
        f.replace("session.pyc", b"successor supervisor", true);
        f.replace("ownership.pyc", b"successor ownership", true);
        f.replace("preparation-kit.zip", b"successor preparation kit", true);
        f.adopt().unwrap();
        let next = f.current();
        let retained = paired_components(&f.base.m, &next, &registration).unwrap();
        assert_eq!(retained.supervisor, old.supervisor);
        assert_eq!(retained.ownership, old.ownership);
        assert_eq!(retained.host, runtime.host);
        assert_ne!(retained.supervisor, next.supervisor);
        let mut foreign = registration.clone();
        foreign.host_source_sha256 = "fe".repeat(32);
        assert!(paired_components(&f.base.m, &next, &foreign).is_err());
        let record = dir.join("runtime.json");
        fs::rename(&record, dir.join("runtime.saved")).unwrap();
        assert!(paired_components(&f.base.m, &next, &registration).is_err());
        fs::rename(dir.join("runtime.saved"), &record).unwrap();
        fs::set_permissions(&runtime.host.path, fs::Permissions::from_mode(0o644)).unwrap();
        fs::write(&runtime.host.path, b"changed prepared host").unwrap();
        assert!(paired_components(&f.base.m, &next, &registration).is_err());
        assert_eq!(f.current().host, next.host);
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
        atomic_json(&f.base.m.root.join("registry.json"), &Registry::default()).unwrap();
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
        assert_eq!(f.service.show().unwrap().active, "active");
        f.service.stop().unwrap();
        f.adopt().unwrap();
        let next = f.current();
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        setup_install::interrupt_journaled_for_test(&f.base.m, &f.home, &next, Some(&first), true).unwrap();
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert_eq!(f.service.show().unwrap().active, "active");
        assert_eq!(f.current().manager.sha256, next.manager.sha256);
        assert_eq!(fs::read_link(f.home.join(".local/bin/linux-audio-compatibility-manager")).unwrap(), next.operator_frontend.unwrap().path);
    }

    #[test]
    fn interrupted_first_install_recovers_absence_or_completed_selection() {
        for complete in [false, true] {
            let f = Fixture::new();
            let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
            let installed = stage(&f.base.m, &f.inputs, manifest, sha, None).unwrap();
            setup_install::interrupt_journaled_for_test(&f.base.m, &f.home,
                &installed, None, complete).unwrap();
            assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
            assert!(!f.base.m.root.join("package-transition.json").exists());
            if complete {
                assert!(f.current() == installed);
                assert_eq!(f.service.show().unwrap().active, "active");
                assert_eq!(f.service.starts.get(), 1);
            } else {
                assert!(old_software(&f.base.m).unwrap().is_none());
                let state = f.service.show().unwrap();
                assert_eq!(state.load, "not-found");
                assert_eq!(state.active, "inactive");
                assert_eq!(f.service.starts.get(), 0);
                assert!(!f.home.join(".local/bin/linux-vst-bridge").exists());
            }
        }
    }

    #[test]
    fn first_install_auto_restart_recovers_after_unit_before_selection() {
        let f = Fixture::new();
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        let installed = stage(&f.base.m, &f.inputs, manifest, sha, None).unwrap();
        setup_install::interrupt_first_install_after_service_for_test(
            &f.base.m, &f.home, &installed).unwrap();
        f.service.reload().unwrap();
        f.service.loaded.borrow_mut().active = "activating".into();
        f.service.fail_idle.set(true);
        assert!(old_software(&f.base.m).unwrap().is_none());
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert_eq!(f.service.stops.get(), 1);
        assert_eq!(f.service.starts.get(), 0);
        let state = f.service.show().unwrap();
        assert_eq!(state.load, "not-found");
        assert_eq!(state.active, "inactive");
        assert!(!f.base.m.root.join("package-transition.json").exists());
    }

    #[test]
    fn auto_restart_recovery_refuses_foreign_exec_and_durable_owner() {
        for refusal in ["foreign_exec", "durable_owner"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let selected = f.current();
            f.service.loaded.borrow_mut().active = "activating".into();
            f.service.fail_idle.set(true);
            if refusal == "foreign_exec" {
                f.service.loaded.borrow_mut().exec =
                    "{ path=/usr/bin/foreign ; argv[]=/usr/bin/foreign serve ; ignore_errors=no }".into();
            } else {
                owned_lease(&f, &"ab".repeat(16), false);
            }
            assert!(stop_transition_service(&f.base.m, &f.home, &f.service,
                &[&selected], &[]).is_err(), "{refusal}");
            assert_eq!(f.service.stops.get(), 0, "{refusal}");
            assert_eq!(f.service.show().unwrap().active, "activating", "{refusal}");
        }
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
        let absent = parse_unit_readback(
            "LoadState=not-found\nActiveState=inactive\nFragmentPath=\n").unwrap();
        assert_eq!(absent.load, "not-found");
        assert_eq!(absent.active, "inactive");
        assert!(absent.fragment.is_empty() && absent.exec.is_empty());
        assert!(parse_unit_readback(
            "LoadState=not-found\nActiveState=active\nFragmentPath=\n").is_err());
        assert!(parse_unit_readback(
            "LoadState=not-found\nActiveState=inactive\nFragmentPath=/x\n").is_err());
        assert!(parse_unit_readback(
            "LoadState=loaded\nActiveState=inactive\nFragmentPath=/x\n").is_err());
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
        let exact = f.service.show().unwrap();
        for changed_exec in ["", "{ path=/usr/bin/foreign ; argv[]=/usr/bin/foreign serve ; ignore_errors=no }"] {
            f.service.loaded.borrow_mut().exec = changed_exec.into();
            assert!(activation_status_from(&f.base.m, &f.home, &f.service).is_err());
        }
        *f.service.loaded.borrow_mut() = exact;
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
        f.service.stop().unwrap();
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

    fn staged_transition(f: &Fixture) -> (Software, Software, RefreshReceipt,
        setup_install::Plan) {
        let first = f.current();
        f.replace("linux-vst-bridge", b"coordinated next manager", true);
        f.replace("linux-audio-compatibility-manager",
            b"coordinated next frontend", true);
        let (manifest, sha) = read_manifest(&f.inputs, f.owner).unwrap();
        let next = stage(&f.base.m, &f.inputs, manifest, sha,
            Some(first.clone())).unwrap();
        let publications = f.base.m.package_publication_snapshot().unwrap();
        let receipt = RefreshReceipt { schema:1, predecessor:first.clone(),
            successor:next.clone(), before_publications:publications.clone(),
            after_publications:publications, transitions:Vec::new() };
        let setup = setup_install::plan(&f.base.m, &f.home, &next,
            Some(&first)).unwrap();
        (first, next, receipt, setup)
    }

    fn real_publication_transition() -> (Fixture, publication::PreparedTransition) {
        let (base, profile, census, native) = test_fixture::prepared();
        let f = Fixture::with_base(base);
        f.adopt().unwrap();
        let registration = observation::derive(&profile, &census, &native).unwrap();
        let first = f.base.m.managed_publish(&profile, &census, registration.clone(),
            &census.host, &census.host_source_sha256, None).unwrap();
        let before = f.base.m.package_publication_snapshot().unwrap().remove(0);
        let mut next_profile = profile.clone();
        next_profile.revision += 1;
        let second = f.base.m.managed_publish(&next_profile, &census, registration,
            &census.host, &census.host_source_sha256, None).unwrap();
        let after = f.base.m.package_publication_snapshot().unwrap().remove(0);
        let second_revision = f.base.m.load_revision(&after.class_id, &second).unwrap();
        let transition = serde_json::from_value(serde_json::json!({
            "schema":2,
            "before":before,
            "after":after,
            "intent":{
                "schema":1,
                "id":second_revision.transaction,
                "class_id":before.class_id,
                "prior":{
                    "entry":before.entry,
                    "revision":first,
                    "target":before.target,
                },
                "candidate":second,
                "candidate_target":after.target,
            },
            "reverse_intent":{
                "schema":1,
                "id":random_id().unwrap(),
                "class_id":after.class_id,
                "prior":{
                    "entry":after.entry,
                    "revision":second,
                    "target":after.target,
                },
                "candidate":first,
                "candidate_target":before.target,
            },
        })).unwrap();
        // Arrange the retained baseline through the same exact package
        // transition authority used by recovery. Ordinary rollback correctly
        // refuses these deliberately legacy fixture publications.
        f.base.m.restore_package_publication(&transition).unwrap();
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(), vec![before.clone()]);
        (f, transition)
    }

    fn interrupt_publication(transition: &publication::PreparedTransition,
        m: &Manager, reverse: bool) {
        let value = serde_json::to_value(transition).unwrap();
        let intent = &value[if reverse { "reverse_intent" } else { "intent" }];
        let class_id = intent["class_id"].as_str().unwrap();
        let id = intent["id"].as_str().unwrap();
        let directory = m.root.join("transactions");
        private_dir(&directory).unwrap();
        let archived = directory.join(format!("{id}.json"));
        if !archived.exists() { atomic_json(&archived, intent).unwrap(); }
        assert_eq!(read_json::<serde_json::Value>(&archived).unwrap(), *intent);
        // Production binds the pending marker to the exact archived bytes,
        // including their hash. Preserve those bytes when arranging the
        // interrupted state instead of reserializing the same JSON value.
        fs::copy(&archived,
            directory.join(format!("{class_id}.pending.json"))).unwrap();
        let target = intent["candidate_target"].as_str().unwrap();
        let link = m.link(class_id);
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(target, &link).unwrap();
    }

    #[test]
    fn coordinated_update_and_restore_recover_after_old_routes_are_selected() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let (first, next, receipt, setup) = staged_transition(&f);
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone() };
        run_coordinated(&f.base.m, &f.home, &f.service, update, false).unwrap();
        assert!(f.current() == next);
        assert_eq!(f.service.show().unwrap().active, "active");
        assert_eq!(matching_refresh_receipts(&f.base.m, &next,
            &receipt.after_publications).unwrap().len(), 1);

        f.service.stop().unwrap();
        let restore = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Restore, recovery_before:next.clone(),
            setup:setup_install::plan(&f.base.m, &f.home, &first,
                Some(&next)).unwrap(), receipt };
        let journal = f.base.m.root.join("package-transition.json");
        atomic_json(&journal, &restore).unwrap();
        // Simulate interruption after the retained old package routes have
        // been selected. Recovery still runs in this fixed new manager.
        setup_install::apply_plan(&restore.setup).unwrap();
        assert!(f.current() == first);
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert!(f.current() == first);
        assert_eq!(f.service.show().unwrap().active, "active");
        assert!(!journal.exists());
    }

    #[test]
    fn restore_keeps_successor_service_bootable_until_predecessor_is_coherent() {
        let (f, publication) = real_publication_transition();
        let (first, next, _, setup) = staged_transition(&f);
        let receipt = RefreshReceipt { schema:1, predecessor:first.clone(),
            successor:next.clone(),
            before_publications:vec![publication.before.clone()],
            after_publications:vec![publication.after.clone()],
            transitions:vec![publication.clone()] };
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone() };
        run_coordinated(&f.base.m, &f.home, &f.service, update, false).unwrap();
        f.service.stop().unwrap();
        let restore = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Restore, recovery_before:next.clone(),
            setup:setup_install::plan(&f.base.m, &f.home, &first,
                Some(&next)).unwrap(), receipt };
        let journal = f.base.m.root.join("package-transition.json");
        atomic_json(&journal, &restore).unwrap();
        f.base.m.restore_package_publication(&publication).unwrap();
        setup_install::interrupt_restore_before_service_for_test(&restore.setup).unwrap();
        assert!(f.current() == first);
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(),
            restore.receipt.before_publications);
        f.service.reload().unwrap();
        let boot = f.service.show().unwrap();
        assert!(effective_exec_is(&boot.exec, &next.manager.path));
        assert!(require_service_transition_coherent_at(
            &f.base.m, &first, &f.home).is_err());
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert!(f.current() == next);
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(),
            restore.receipt.after_publications);
        assert_eq!(f.service.show().unwrap().active, "active");
        assert!(!journal.exists());
    }

    #[test]
    fn failed_restore_activation_recovers_successor_service_before_selection() {
        let (f, publication) = real_publication_transition();
        let (first, next, _, setup) = staged_transition(&f);
        let receipt = RefreshReceipt { schema:1, predecessor:first.clone(),
            successor:next.clone(),
            before_publications:vec![publication.before.clone()],
            after_publications:vec![publication.after.clone()],
            transitions:vec![publication] };
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone() };
        run_coordinated(&f.base.m, &f.home, &f.service, update, false).unwrap();
        f.service.stop().unwrap();
        let restore = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Restore, recovery_before:next.clone(),
            setup:setup_install::plan(&f.base.m, &f.home, &first,
                Some(&next)).unwrap(), receipt };
        let journal = f.base.m.root.join("package-transition.json");
        atomic_json(&journal, &restore).unwrap();
        apply_coordinated(&f.base.m, &restore).unwrap();
        assert!(f.current() == first);
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(),
            restore.receipt.before_publications);
        f.service.fail_start.set(true);
        assert!(start_and_health(&f.service, &f.base.m, &f.home, &first).is_err());
        f.service.fail_start.set(false);

        // Simulate interruption during failure recovery immediately after the
        // journal-aware successor unit has replaced the predecessor unit.
        setup_install::interrupt_restore_origin_after_service_for_test(
            &restore.setup).unwrap();
        f.service.reload().unwrap();
        let boot = f.service.show().unwrap();
        assert!(effective_exec_is(&boot.exec, &next.manager.path));
        assert!(f.current() == first);
        assert!(require_service_transition_coherent_at(
            &f.base.m, &first, &f.home).is_err());
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert!(f.current() == next);
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(),
            restore.receipt.after_publications);
        assert_eq!(f.service.show().unwrap().active, "active");
        assert!(!journal.exists());
    }

    #[test]
    fn coordinated_recovery_owns_interrupted_forward_and_reverse_publications() {
        let (f, publication) = real_publication_transition();
        let (first, next, _, setup) = staged_transition(&f);
        let receipt = RefreshReceipt { schema:1, predecessor:first.clone(),
            successor:next.clone(),
            before_publications:vec![publication.before.clone()],
            after_publications:vec![publication.after.clone()],
            transitions:vec![publication.clone()] };
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone() };
        let journal = f.base.m.root.join("package-transition.json");
        atomic_json(&journal, &update).unwrap();
        interrupt_publication(&publication, &f.base.m, false);
        assert!(f.base.m.publication_pending(&publication.before.class_id).unwrap());
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(),
            receipt.before_publications);
        assert!(!journal.exists());

        f.service.stop().unwrap();
        run_coordinated(&f.base.m, &f.home, &f.service, update, false).unwrap();
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(),
            receipt.after_publications);
        f.service.stop().unwrap();
        let restore = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Restore, recovery_before:next.clone(),
            setup:setup_install::plan(&f.base.m, &f.home, &first,
                Some(&next)).unwrap(), receipt:receipt.clone() };
        atomic_json(&journal, &restore).unwrap();
        interrupt_publication(&publication, &f.base.m, true);
        assert!(f.base.m.publication_pending(&publication.before.class_id).unwrap());
        assert!(recover_from(&f.base.m, &f.home, &f.service).unwrap());
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(),
            receipt.after_publications);
        assert!(f.current() == next);
        assert!(!journal.exists());
    }

    #[test]
    fn all_final_recovery_does_not_stop_an_active_dsp() {
        let (f, publication) = real_publication_transition();
        let (first, next, _, setup) = staged_transition(&f);
        let receipt = RefreshReceipt { schema:1, predecessor:first.clone(),
            successor:next.clone(),
            before_publications:vec![publication.before.clone()],
            after_publications:vec![publication.after.clone()],
            transitions:vec![publication] };
        let transition = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first, setup, receipt };
        let journal = f.base.m.root.join("package-transition.json");
        atomic_json(&journal, &transition).unwrap();
        apply_coordinated(&f.base.m, &transition).unwrap();
        start_and_health(&f.service, &f.base.m, &f.home, &next).unwrap();
        owned_lease(&f, &"ab".repeat(16), false);
        let stops = f.service.stops.get();
        assert_eq!(recover_from(&f.base.m, &f.home, &f.service)
            .unwrap_err().to_string(), "package_service_not_clean_idle");
        assert_eq!(f.service.stops.get(), stops);
        assert_eq!(f.service.show().unwrap().active, "active");
        assert!(journal.exists());
    }

    #[test]
    fn coordinated_activation_failure_restores_prior_or_retains_recovery() {
        for failed_starts in [1, 2] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let (first, _, receipt, setup) = staged_transition(&f);
            let transition = CoordinatedTransition { schema:2,
                id:random_id().unwrap(), kind:TransitionKind::Update,
                recovery_before:first.clone(), setup, receipt };
            f.service.fail_starts.set(failed_starts);
            let error = run_coordinated(&f.base.m, &f.home, &f.service,
                transition, false).unwrap_err().to_string();
            assert!(error.contains("package_user_service_unavailable"));
            assert!(f.current() == first);
            assert_eq!(f.base.m.root.join("package-transition.json").exists(),
                failed_starts == 2);
            if failed_starts == 1 {
                assert_eq!(f.service.show().unwrap().active, "active");
                assert!(!error.contains("package recovery failed"));
            } else {
                assert!(error.contains("package recovery failed"));
            }
        }
    }

    #[test]
    fn service_admission_refuses_mixed_package_and_publication_journal() {
        let (f, prepared) = real_publication_transition();
        let (first, next, _, setup) = staged_transition(&f);
        let before = vec![prepared.before.clone()];
        let after = vec![prepared.after.clone()];
        let receipt = RefreshReceipt { schema:1, predecessor:first.clone(),
            successor:next.clone(), before_publications:before,
            after_publications:after, transitions:vec![prepared] };
        let transition = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt };
        let journal = f.base.m.root.join("package-transition.json");
        atomic_json(&journal, &transition).unwrap();
        require_service_transition_coherent_at(&f.base.m, &first, &f.home).unwrap();
        setup_install::apply_plan(&transition.setup).unwrap();
        assert!(require_service_transition_coherent_at(
            &f.base.m, &next, &f.home).is_err());
        assert!(journal.exists());
    }

    #[test]
    fn failed_preparation_after_owned_stop_restarts_selected_service() {
        let (f, _) = real_publication_transition();
        let selected = f.current();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        f.replace("linux-vst-bridge", b"target requiring managed refresh", true);
        let error = update_from(&f.base.m, &f.home, &f.inputs, f.owner,
            &f.service).unwrap_err().to_string();
        assert!(error.contains("Preparation support is missing"));
        assert!(f.current() == selected);
        assert_eq!(f.service.show().unwrap().active, "active");
        assert_eq!((f.service.stops.get(), f.service.starts.get()), (1, 2));
        assert!(!f.base.m.root.join("package-transition.json").exists());
    }

    #[test]
    fn oversized_coordinator_is_refused_before_journal_and_restarts_prior_service() {
        let f = Fixture::new();
        f.adopt().unwrap();
        f.base.m.register(f.base.r.clone()).unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        f.service.stop().unwrap();
        let (first, _, mut receipt, setup) = staged_transition(&f);
        let mut state = f.base.m.package_publication_snapshot().unwrap().remove(0);
        state.entry.registration.metadata.name = "x".repeat(CONTROL_RECORD_MAX);
        receipt.before_publications = vec![state.clone()];
        receipt.after_publications = vec![state];
        let transition = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first, setup, receipt };
        let error = run_coordinated(&f.base.m, &f.home, &f.service,
            transition, true).unwrap_err().to_string();
        assert!(error.contains("package_refresh_receipt_bound"));
        assert!(!f.base.m.root.join("package-transition.json").exists());
        assert_eq!(f.service.show().unwrap().active, "active");
    }

    #[test]
    fn raw_package_switches_cannot_bypass_a_published_bridge_refresh() {
        let (base, profile, census, _) = test_fixture::prepared();
        let home = base.outer.join("home");
        let f = Fixture { service:FakeService::new(&home), home,
            inputs:Inputs::under(&base.outer.join("package/usr")),
            owner:unsafe { libc::getuid() }, base };
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
        f.adopt().unwrap();
        // Establish a package predecessor before this fixture publishes. Once
        // a publication exists, raw generation switches are no longer legal.
        f.base.m.unpublish(&f.base.r.key()).unwrap();
        atomic_json(&f.base.m.root.join("registry.json"), &Registry::default()).unwrap();
        f.replace("linux-vst-bridge", b"second generation", true);
        f.adopt().unwrap();
        let selected = f.current();
        f.base.m.managed_publish(&profile, &census, f.base.r.clone(),
            &f.base.r.host, &f.base.r.host_source_sha256, None).unwrap();
        let registry = fs::read(f.base.m.root.join("registry.json")).unwrap();
        let frontend = f.home.join(".local/bin/linux-audio-compatibility-manager");
        fs::remove_file(&frontend).unwrap();
        f.adopt().unwrap();
        assert_eq!(fs::read_link(&frontend).unwrap(),
            selected.operator_frontend.as_ref().unwrap().path);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry);
        f.replace("linux-vst-bridge", b"third generation", true);
        let adopt_error = f.adopt().unwrap_err().to_string();
        assert!(adopt_error.contains("package_update_required"));
        let restore_error = rollback_from(&f.base.m, &f.home,
            &f.service).unwrap_err().to_string();
        assert!(restore_error.contains("package_restore_required"));
        assert!(f.current() == selected);
        assert_eq!(fs::read(f.base.m.root.join("registry.json")).unwrap(), registry);
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

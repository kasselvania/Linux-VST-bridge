//! PKG0: fixed installed-package intake into immutable user-owned software.
//! Package-manager ownership authenticates /usr inputs; the release signature
//! is checked by the package/release tooling before these files are installed.
use super::*;
use linux_vst_bridge::publication::{Revision, RevisionRef};
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
const SOURCE_NAMES_WITH_KIT: [&str; 7] = [
    "linux-vst-bridge", "linux-audio-compatibility-manager", "session.py",
    "ownership.py", "host.exe", "host-source-manifest.json", "preparation-kit.zip",
];
fn names(schema: u32) -> Result<&'static [&'static str]> {
    match schema {
        1 => Ok(&NAMES),
        2 => Ok(&NAMES_WITH_KIT),
        3 => Ok(&SOURCE_NAMES_WITH_KIT),
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
            "session.py" => "lib/linux-vst-bridge/supervisor/session.py",
            "ownership.pyc" => "lib/linux-vst-bridge/supervisor/ownership.pyc",
            "ownership.py" => "lib/linux-vst-bridge/supervisor/ownership.py",
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resume_history: Option<ResumeHistory>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumeHistory {
    source: Software,
    captured: operator_cli::PackageResume,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResumeHandoff {
    captured: operator_cli::PackageResume,
    origin_active: bool,
    settling: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resume_handoff: Option<ResumeHandoff>,
}

fn ordered_publications(states: &[publication::PublicationState]) -> bool {
    states.len() <= 256
        && states.windows(2).all(|pair| pair[0].class_id < pair[1].class_id)
}

fn validate_receipt(receipt: &RefreshReceipt) -> Result<()> {
    require(matches!((receipt.schema,receipt.resume_history.is_some()),(1,false) | (2,true))
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
    require(expected == receipt.after_publications,"package_refresh_receipt")?;
    if let Some(history) = &receipt.resume_history {history.captured.validate_source(&history.source)?;}
    Ok(())
}

fn validate_coordinated(m: &Manager, home: &Path,
    transition: &CoordinatedTransition) -> Result<()> {
    require(matches!((transition.schema,transition.resume_handoff.is_some()),(2,false) | (3,true))
        && valid_hex(&transition.id, 32),
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
    if let Some(handoff) = &transition.resume_handoff {
        handoff.captured.validate_source(&transition.recovery_before)?;
    }
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
    Ok(refresh_receipts(m,selected)?.into_iter()
        .filter(|receipt|receipt.after_publications==publications).collect())
}
fn refresh_receipts(m:&Manager,selected:&Software)->Result<Vec<RefreshReceipt>> {
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
            if receipt.successor == *selected {matching.push(receipt);}
        }
    }
    Ok(matching)
}

/// Only a real package transition can renew an unfinished job's predecessor.
/// Discovery/module baselines stay with that job; arbitrary replacements refuse.
pub(super) fn renewed_predecessor(m:&Manager,prior:&Revision)->Result<Revision> {
    let db=m.registry()?;let entry=db.classes.get(&prior.class_id).ok_or("package_renewal_class_missing")?;
    let wanted=entry.managed_revision.as_ref().ok_or("package_renewal_publication_missing")?;
    let prior_path=m.root.join("publications").join(&prior.class_id).join("revisions").join(&prior.id).join("revision.json");
    let original=RevisionRef {id:prior.id.clone(),sha256:digest(&prior_path)?};
    require(m.load_revision(&prior.class_id,&original)?==*prior,"package_renewal_predecessor_changed")?;
    let mut cursor=wanted.clone();let mut selected=software(m)?;
    for depth in 0..32 {
        if cursor==original {break;}
        let edges=refresh_receipts(m,&selected)?.into_iter().flat_map(|receipt|receipt.transitions)
            .filter(|edge|edge.after.class_id==prior.class_id
                && edge.after.entry.managed_revision.as_ref()==Some(&cursor)).collect::<Vec<_>>();
        require(edges.len()<=1,"package_renewal_ownership_conflict")?;
        if let Some(edge)=edges.first() {
            require(edge.before.class_id==prior.class_id && edge.before.performance==edge.after.performance,
                "package_renewal_settings_changed")?;
            cursor=edge.before.entry.managed_revision.clone().ok_or("package_renewal_predecessor_missing")?;
        }
        if cursor==original {break;}
        selected=verify_generation(m,&selected)?.predecessor.ok_or("package_renewal_authority_missing")?;
        require(depth<31,"package_renewal_history_bound")?;
    }
    require(cursor==original,"package_renewal_authority_missing")?;
    let current=m.load_revision(&prior.class_id,wanted)?;
    require(current.registration.module==prior.registration.module
        && current.registration.metadata==prior.registration.metadata
        && current.registration.environment==prior.registration.environment
        && current.registration.compatibility==prior.registration.compatibility
        && current.performance==prior.performance && current.external_ids==prior.external_ids
        && current.qualification==prior.qualification,"package_renewal_content_or_settings_changed")?;
    Ok(current)
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
    if manifest.schema == 3 { linux_vst_bridge::supervisor::verify_host_python()?; }
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
    linux_vst_bridge::package_version::valid_version(value)
}
fn id(record: &Generation) -> Result<String> {
    // Source generations are distinct from the retained loose-source layout,
    // even if their package record is later missing. Schema 1/2 IDs stay exact.
    let hash = hex(&sha2::Sha256::digest(serde_json::to_vec(record)?));
    Ok(if record.schema == 3 { format!("source-{hash}") } else { hash })
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
    let packaged_preparation_kit_sha256 = (manifest.schema >= 2)
        .then(|| manifest.files[NAMES.len()].sha256.clone());
    Ok(Generation {
        schema: manifest.schema,
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
fn generation_identity(m: &Manager, dir: &Path) -> Result<Generation> {
    require(dir.parent() == Some(m.root.join("software").as_path()),
        "package_generation_path")?;
    // Verification and predecessor planning are read-only. In particular an
    // absent selected generation must never be recreated as a side effect.
    existing_generation_dir(dir)?;
    let record_path = dir.join("package-generation.json");
    require(record_path.canonicalize()? == record_path
        && fs::metadata(&record_path)?.permissions().mode() & 0o222 == 0,
        "package_generation_record_writable")?;
    let record: Generation = read_json(&record_path)?;
    require(matches!((record.schema, record.manifest.schema), (1, 1) | (2, 2) | (3, 3))
        && id(&record)? == dir.file_name().and_then(|n| n.to_str()).unwrap_or(""),
        "package_generation_identity")?;
    let expected_names = names(record.manifest.schema)?;
    require(record.manifest.package == "linux-vst-bridge-beta"
        && valid_package_version(&record.manifest.version) && record.manifest.pkgrel == 1
        && record.manifest.files.len() == expected_names.len()
        && record.manifest.files.iter().zip(expected_names).all(|(entry, name)| entry.name == *name
            && valid_hex(&entry.sha256, 64)
            && entry.size <= (if *name == "preparation-kit.zip" { 256 } else { 128 })
                * 1024 * 1024), "package_generation_manifest")?;
    Ok(record)
}

fn verify_generation(m: &Manager, current: &Software) -> Result<Generation> {
    let dir = current.manager.path.parent().ok_or("package_generation_path")?;
    let record = generation_identity(m, dir)?;
    let generation_names = names(record.manifest.schema)?;
    let expected = [
        (&current.manager, generation_names[0]),
        (current.operator_frontend.as_ref().ok_or("package_frontend_absent")?, generation_names[1]),
        (&current.supervisor, generation_names[2]), (&current.ownership, generation_names[3]),
        (&current.host, generation_names[4]), (&current.source_manifest, generation_names[5]),
    ];
    for (index, (actual, name)) in expected.iter().enumerate() {
        require(actual.path == dir.join(name)
            && actual.sha256 == record.manifest.files[index].sha256,
            "package_generation_artifact_binding")?;
        actual.verify()?;
    }
    let packaged_kit = if record.schema >= 2 {
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
    if record.schema>=2 {preparation::build::verify_package_host(current,true)?;}
    Ok(record)
}
fn validate_generation_record(m: &Manager, current: &Software) -> Result<Generation> {
    let dir = current
        .manager
        .path
        .parent()
        .ok_or("package_generation_path")?;
    let record = generation_identity(m, dir)?;
    let generation_names = names(record.manifest.schema)?;
    let expected = [
        (&current.manager, generation_names[0]),
        (
            current
                .operator_frontend
                .as_ref()
                .ok_or("package_frontend_absent")?,
            generation_names[1],
        ),
        (&current.supervisor, generation_names[2]),
        (&current.ownership, generation_names[3]),
        (&current.host, generation_names[4]),
        (&current.source_manifest, generation_names[5]),
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
    let packaged_kit = if record.schema >= 2 {
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
    if record.schema>=2 {preparation::build::verify_package_host(current,false)?;}
    Ok(record)
}
/// Only an executable in the canonical managed frontend slot is subject to
/// package selection. Standalone CLI callers and other clients retain their
/// existing behavior. Use the already verified immutable generation's content
/// identity, not a process name or a recurring binary hash/directory scan.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn require_selected_frontend(m: &Manager, executable: &Path,
    running: &fs::Metadata) -> Result<()> {
    let Some(dir) = executable.parent().filter(|dir|
        dir.parent() == Some(m.root.join("software").as_path())
        && executable.file_name() == Some(std::ffi::OsStr::new(NAMES[1]))) else {
        return Ok(());
    };
    let record = generation_identity(m, dir)?;
    let frontend = &record.manifest.files[1];
    let held = file(executable)?;
    let on_disk = held.metadata()?;
    require(executable.canonicalize()? == executable
        && on_disk.permissions().mode() & 0o222 == 0
        && on_disk.dev() == running.dev() && on_disk.ino() == running.ino()
        && on_disk.len() == frontend.size,
        "package_frontend_executable_changed")?;
    let selected = software_record(m)?;
    let current = selected.operator_frontend.as_ref().ok_or("package_frontend_absent")?;
    require(current.path.parent() == selected.manager.path.parent(), "package_prior_pairing")?;
    let selected_record = generation_identity(m,
        current.path.parent().ok_or("package_generation_path")?)?;
    require(current.path.file_name() == Some(std::ffi::OsStr::new(NAMES[1]))
        && current.sha256 == selected_record.manifest.files[1].sha256,
        "package_generation_artifact_binding")?;
    require(frontend.sha256 == current.sha256, linux_vst_bridge::service::contract::FRONTEND_SUPERSEDED)
}

pub(super) fn require_current_frontend_caller(_m: &Manager) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let process = PathBuf::from(format!("/proc/{}", unsafe { libc::getppid() }));
        // An unreadable caller is not a managed-frontend ownership claim.
        let Ok(executable) = fs::read_link(process.join("exe")) else { return Ok(()); };
        let running = fs::metadata(process.join("exe"))?;
        require_selected_frontend(_m, &executable, &running)?;
    }
    Ok(())
}

/// The service socket's counterpart: there the window is the connecting
/// process itself. Its identity is read while that same process generation
/// holds the connection, so a reused pid lends no other program's identity.
pub(super) fn require_current_frontend_peer(_m: &Manager, _peer: &std::os::unix::net::UnixStream) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        let caller = transport_storage::peer_process(_peer)?;
        let process = PathBuf::from(format!("/proc/{}", caller.pid));
        let Ok(executable) = fs::read_link(process.join("exe")) else { return Ok(()); };
        let running = fs::metadata(process.join("exe"))?;
        require(transport_storage::peer_process(_peer)? == caller, "peer_process_generation_changed")?;
        require_selected_frontend(_m, &executable, &running)?;
    }
    Ok(())
}

pub(super) fn selected_package_version(m: &Manager, current: &Software) -> Result<Option<String>> {
    Ok(selected_generation(m, current)?.map(|record| record.manifest.version))
}
/// A package's version and source tree.
type PackageBuild = (String, String);
/// The installed bridge build and what Setup holds beside it, for display.
/// Setup opens the user's staged package when one is staged, otherwise the
/// system package. Nothing here fails the readback: an unreadable installed
/// record leaves the version unknown, and an unverifiable Setup package is
/// reported as unchecked rather than replaced by another package.
pub(super) fn bridge_build(m: &Manager, current: &Software) -> operator_model::BridgeBuild {
    let staged = match std::env::var_os("HOME") {
        Some(home) => linux_vst_bridge::portable_package::setup_release(Path::new(&home))
            .map(|release| release.map(|release| (release.version, release.source_tree))),
        None => Ok(None),
    };
    bridge_build_from(m, current, staged, || Inputs::installed_record().map(Some))
}
fn bridge_build_from(m: &Manager, current: &Software, staged: Result<Option<PackageBuild>>,
    system: impl FnOnce() -> Result<Option<(Inputs, u32)>>) -> operator_model::BridgeBuild {
    use linux_vst_bridge::package_version::compare_versions;
    use operator_model::SetupPackage;
    // The record alone names the version; its artifacts are not revisited here.
    let installed = current.manager.path.parent()
        .and_then(|dir| generation_identity(m, dir).ok()).map(|record| record.manifest);
    // Only the absence of a staged package consults the system package.
    let setup = match staged {
        Ok(None) => system_package(system),
        other => other,
    };
    let setup = match (setup, &installed) {
        (Err(_), _) => SetupPackage::Unchecked,
        (Ok(Some((version, tree))), Some(installed)) =>
            match compare_versions(&version, &installed.version) {
                std::cmp::Ordering::Greater => SetupPackage::Newer(version),
                std::cmp::Ordering::Less => SetupPackage::Older(version),
                std::cmp::Ordering::Equal if tree != installed.source_tree =>
                    SetupPackage::DifferentBuild(version),
                std::cmp::Ordering::Equal => SetupPackage::Nothing,
            },
        _ => SetupPackage::Nothing,
    };
    operator_model::BridgeBuild { installed: installed.map(|manifest| manifest.version), setup }
}
fn system_package(system: impl FnOnce() -> Result<Option<(Inputs, u32)>>)
    -> Result<Option<PackageBuild>> {
    let Some((inputs, owner)) = system()? else { return Ok(None) };
    if !inputs.manifest().try_exists()? { return Ok(None); }
    let (manifest, _) = read_manifest_record(&inputs, owner)?;
    Ok(Some((manifest.version, manifest.source_tree)))
}
fn selected_generation(m: &Manager, current: &Software) -> Result<Option<Generation>> {
    let dir = current.manager.path.parent().ok_or("package_generation_path")?;
    match fs::symlink_metadata(dir.join("package-generation.json")) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Only the retained pre-package helper layout has no package record.
            // A missing or damaged modern record must never become legacy.
            require(!dir.file_name().and_then(|name| name.to_str()).unwrap_or("").starts_with("source-")
                && current.supervisor.path == dir.join("session.py")
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
                !dir.file_name().and_then(|name| name.to_str()).unwrap_or("").starts_with("source-")
                    && current.supervisor.path == dir.join("session.py")
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
            if record.schema>=2 {
                let kit=Artifact {path:scratch.join("preparation-kit.zip"),
                    sha256:record.manifest.files[NAMES.len()].sha256.clone()};
                preparation::build::stage_package_host(&kit,&scratch,&dest,
                    &record.manifest.files[4].sha256,&record.manifest.files[5].sha256)?;
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
        preparation_kit: if record.schema >= 2 {
            Some(artifact(&dest, "preparation-kit.zip")?)
        } else { record.retained_preparation_kit },
        operator_frontend: Some(artifact(&dest, "linux-audio-compatibility-manager")?),
        manager: artifact(&dest, "linux-vst-bridge")?,
        supervisor: artifact(&dest, names(record.manifest.schema)?[2])?,
        ownership: artifact(&dest, names(record.manifest.schema)?[3])?,
        host: artifact(&dest, "host.exe")?,
        source_manifest: artifact(&dest, "host-source-manifest.json")?,
        source_sha256: digest(&dest.join("host-source-manifest.json"))?,
        native_catalogue,
    };
    if result.native_catalogue.is_some() { result.catalogue(m)?; }
    if record.schema>=2 {preparation::build::verify_package_host(&result,true)?;}
    Ok(result)
}
fn preflight(m: &Manager, service: &impl ServiceControl) -> Result<()> {
    require(operator_cli::all_operations_finished(m)?,"package_operator_active_or_ambiguous")?;
    require(!reconcile_leases(m)?, "package_cleanup_unconfirmed")?;
    m.require_inactive(None)?;
    require(capacity::owners(m)?.is_empty(), "package_owner_active")?;
    require(operator_cli::pending_transactions(m)? == 0,
        "package_transaction_pending")?;
    require(onboarding::all_retired(m)?, "package_installer_owner_active")?;
    require(service.vendor_retired(m)?, "package_vendor_owner_active")?;
    daw_workspace::package_idle(m)?;
    transport_storage::require_no_sessions()?;
    Ok(())
}
/// Settling stays inside Setup's 90 s status wait: the census uses what is left
/// of this budget, dead-task finishing waits at most the bounded registry
/// readback, and no service restoration is waited on.
const SETTLE_BUDGET: Duration = Duration::from_secs(60);
/// A freeze or power loss can end an operator worker or an app session without
/// the owner that records its end. Settle exactly those before a gate reads
/// them, so they cannot block the update that would fix them. Live or
/// unobservable owners are untouched and still refuse.
///
/// Runs once, at a package entry point, holding neither the registry nor the
/// resume lock. Finishing a dead task does what its ExecStopPost would have
/// done except restoring a service: every reservation it or its
/// automatic-install checks leave goes to the package handoff, activation,
/// the service start or Recover.
fn settle_interrupted(m: &Manager, service: &impl ServiceControl) -> Result<()> {
    let deadline = Instant::now() + SETTLE_BUDGET;
    (|| -> Result<()> {
        for failure in operator_cli::settle_dead_workers(m, |operation| service.worker_live(operation))? {
            eprintln!("An interrupted bridge task was not completely finished: {failure}");
        }
        // A live task may own its app session and that session's writer.
        if !operator_cli::all_workers_finished(m)? { return Ok(()); }
        service.reverify_vendor(m, deadline.saturating_duration_since(Instant::now()))?;
        let recovery=operator_cli::vendor_maintenance::reconcile(m, |operation| service.worker_live(operation))?;
        require(recovery.failures.is_empty(),&recovery.failures.join("; "))
    })().map_err(gate_refusal)
}
/// Setup has no per-app Recover; its refusals end with Setup's own next step.
/// The full cause goes to Setup's error output first; the code and census
/// also stay in the app's recover-check record.
fn gate_refusal(error: Box<dyn std::error::Error + Send + Sync>) -> Box<dyn std::error::Error + Send + Sync> {
    let mut cause = error.to_string();
    let mut source = error.source();
    while let Some(inner) = source { cause.push_str(&format!(": {inner}")); source = inner.source(); }
    let Some(refusal) = error.downcast_ref::<vendor_cli::RecoverRefusal>() else {
        eprintln!("Interrupted work was not settled: {cause}");
        return error;
    };
    eprintln!("An app session re-check refused ({}): {cause}", refusal.code);
    match refusal.code {
        "vendor_recover_service_running" => "The app's support service is still running. Restart the computer, then choose Check again.",
        "vendor_application_custody_uncertain" => "Part of an app opened through the bridge is still running. Close it or wait a minute, then choose Check again.",
        "vendor_recover_census_timeout" | "vendor_recover_observation_unavailable" => "The bridge could not confirm that an app opened through it has closed. Restart the computer, then choose Check again; if this stays, send Setup details.",
        "vendor_recover_unit_unavailable" => "The bridge could not check whether an app opened through it is still running. Choose Check again; if this stays, send Setup details.",
        "vendor_recover_daemon_name_unobservable" => "The bridge can't yet check this app's support service, so it can't close the app's session itself. Send Setup details.",
        // The check itself could not run: the supervisor or its request.
        _ => "The bridge could not run its check of an app opened through it. Download the bridge package again, then choose Check again; if it still fails, send Setup details.",
    }.into()
}
/// A settling refusal explains a gate's own refusal of the same app session.
/// Settling never decides on its own: every gate still applies its checks.
fn settled_then<T>(settled: Result<()>, gated: Result<T>) -> Result<T> {
    match (settled, gated) {
        (Err(refusal), Err(error)) if matches!(error.to_string().as_str(), "package_vendor_owner_active"
            | "package_resume_vendor_binding" | "package_resume_vendor_not_retired"
            | "operator_vendor_cleanup_unconfirmed") => Err(refusal),
        (_, gated) => gated,
    }
}
/// The census supervisor and ownership modules shipped in the package whose
/// manager runs this gate; the installed ones may predate the census. Only
/// these two files and the manifest naming them are read and verified.
fn package_census_tools(inputs: &Inputs, owner: u32) -> Result<(Artifact, Artifact)> {
    let (_, _, bytes) = package_source(&inputs.manifest(), owner, 64 * 1024, true)?;
    let manifest: PackageManifest = serde_json::from_slice(&bytes)?;
    validate_manifest_identity(&manifest)?;
    let tool = |name: &str| -> Result<Artifact> {
        let entry = manifest.files.iter().find(|file| file.name == name).ok_or("package_manifest_roster")?;
        let path = inputs.path(name)?;
        let (sha256, size, _) = package_source(&path, owner, 128 * 1024 * 1024, false)?;
        require(sha256 == entry.sha256 && size == entry.size, "package_artifact_changed")?;
        Ok(Artifact { path, sha256 })
    };
    let roster = names(manifest.schema)?;
    Ok((tool(roster[2])?, tool(roster[3])?))
}
/// The running manager's own package: a signed source package verifies just
/// the two supervisor rows of its release; the system package is root-owned.
fn running_package_census_tools() -> Result<(Artifact, Artifact)> {
    let executable = std::env::current_exe()?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    if let Some([(supervisor, supervisor_sha256), (ownership, ownership_sha256)]) =
        linux_vst_bridge::portable_package::input_supervisor(&executable, &home)? {
        return Ok((Artifact { path: supervisor, sha256: supervisor_sha256 },
            Artifact { path: ownership, sha256: ownership_sha256 }));
    }
    package_census_tools(&Inputs { root: PathBuf::from(PACKAGE_ROOT) }, 0)
}
fn recovery_preflight(m: &Manager,
    transition: &CoordinatedTransition, service: &impl ServiceControl) -> Result<()> {
    require(operator_cli::all_operations_finished(m)?,"package_operator_active_or_ambiguous")?;
    require(!reconcile_leases(m)?, "package_cleanup_unconfirmed")?;
    m.require_inactive(None)?;
    require(capacity::owners(m)?.is_empty(), "package_owner_active")?;
    m.verify_package_pending_intents(&transition.receipt.transitions)?;
    require(onboarding::all_retired(m)?, "package_installer_owner_active")?;
    require(service.vendor_retired(m)?, "package_vendor_owner_active")?;
    daw_workspace::package_idle(m)?;
    transport_storage::require_no_sessions()?;
    Ok(())
}
fn with_locks<T>(m: &Manager, work: impl FnOnce() -> Result<T>) -> Result<T> {
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    let _resume = operator_cli::resume_lock(m)?;
    let _service = m.lock("service.lock")?;
    let _registry = m.lock("registry.lock")?;
    work()
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

/// Whether adding back a removed publication needs a fresh check first: it was
/// prepared with another bridge kit than the one installed now.
pub(super) fn add_back_needs_refresh(m: &Manager, key: &str, removed: &RevisionRef) -> Result<bool> {
    let revision = m.load_revision_record(key, removed)?;
    let kit = preparation::build::recipe_available(m)?;
    preparation::publication_requires_refresh(m, &revision, &kit.sha256)
}

/// "Add back to my DAW" after a bridge update, as one action: inspect the same
/// Windows module and class with the current bridge, prepare it with the
/// removed publication's settings, and publish that. Caller has suspended the
/// service under the environment lock, as for a refreshed restore.
pub(super) fn add_back_publication(m: &Manager, key: &str, removed: &RevisionRef,
    operation: &str) -> Result<RevisionRef> {
    add_back_publication_with(m, key, removed, operation, inspect_for_package_refresh)
}
/// `inspect` is the supervised inspection of the same module and class with
/// the current bridge; tests supply its report.
pub(super) fn add_back_publication_with(m: &Manager, key: &str, removed: &RevisionRef,
    operation: &str, inspect: impl FnOnce(&Manager, &Software, &preparation::build::Runtime,
        &Registration) -> Result<Artifact>) -> Result<RevisionRef> {
    let _package = m.lock("package.lock")?;
    require(!m.root.join("package-transition.json").try_exists()?,
        "package_transition_needs_recovery")?;
    let entry = m.registry()?.classes.get(key).cloned().ok_or("registration_absent")?;
    require(entry.publication == Publication::Removed
        && entry.managed_revision.as_ref() == Some(removed)
        && !m.publication_pending(key)?, "add_back_publication_changed")?;
    let predecessor = m.load_revision(key, removed)?;
    m.require_inactive(None)?;
    let selected = software(m)?;
    verify_software_identity(m, &selected)?;
    let runtime = preparation::build::stage_runtime_for_software(m, &selected)?;
    require(preparation::build::runtime_declares_loaded_engine(&runtime)?,
        "loaded_engine_admission_contract_missing")?;
    let report = inspect(m, &selected, &runtime, &predecessor.registration)?;
    let prepared = preparation::refresh_candidate(m, &predecessor, runtime, report, operation);
    let cleanup = preparation::build::cleanup_work(m, operation);
    let candidate = prepared?;
    cleanup?;
    preparation::enable_refreshed(m, &candidate, &predecessor)
}

fn prepare_refresh_receipt(m: &Manager, target: &Software,
    predecessor: Software,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<RefreshReceipt> {
    let before = m.package_publication_snapshot_locked(&acquire_registry()?)?;
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
        // A runtime change already made this publication historical. Update
        // the manager without inventing a new-runner publication; ordinary
        // fresh preparation owns that separate musician choice.
        if environment_revision::current_environment(m, &revision.registration.environment)?
            != revision.registration.environment { continue; }
        // A runtime that changed outside the product, such as Steam updating
        // the Proton build a setup sits on, makes this publication historical
        // for the refresh as well: the setup's own launch admission reports
        // the condition. The manager update continues without it.
        if revision.registration.environment.runner.verify_pinned_files().is_err() { continue; }
        if !preparation::publication_requires_refresh(m, &revision, &runtime.kit.sha256)? {
            continue;
        }
        let report = inspect_for_package_refresh_with_registry(m, target, &runtime,
            &revision.registration, &mut *acquire_registry)?;
        let operation = random_id()?;
        let prepared = preparation::refresh_candidate(m, &revision,
            runtime.clone(), report, &operation);
        let cleanup = preparation::build::cleanup_work(m, &operation);
        let candidate = prepared?;
        cleanup?;
        transitions.push(m.prepare_package_refresh_with_registry(
            &candidate, reference, &mut *acquire_registry)?);
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
    m.verify_package_publication_snapshot_locked(&before, &acquire_registry()?)?;
    let receipt = RefreshReceipt { schema:1, predecessor,
        successor:target.clone(), before_publications:before,
        after_publications:after, transitions, resume_history:None };
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
    if !m.root.join("package-transition.json").try_exists()? {
        for _ in 0..20 {
            if service.healthy(m).is_ok() { return Ok(()); }
            std::thread::sleep(Duration::from_millis(100));
        }
        return service.healthy(m);
    }
    // The transition's new service may spend the existing registry allowance
    // before creating its endpoint, plus systemd's two-second restart delay.
    let deadline = Instant::now() + operator_cli::OPERATOR_WAIT + Duration::from_secs(4);
    loop {
        let result = service.healthy(m);
        if result.is_ok() || Instant::now() >= deadline { return result; }
        std::thread::sleep(Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())));
    }
}

fn stop_transition_service(m: &Manager, home: &Path,
    service: &impl ServiceControl, software: &[&Software],
    publications: &[publication::PreparedTransition]) -> Result<()> {
    stop_transition_service_with_registry(m, home, service, software, publications,
        &mut || m.lock("registry.lock"))
}
fn stop_transition_service_with_registry(m: &Manager, home: &Path,
    service: &impl ServiceControl, software: &[&Software],
    publications: &[publication::PreparedTransition],
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
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
            let registry = acquire_registry()?;
            registry.require_registry(m)?;
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

#[cfg(test)]
fn apply_coordinated(m: &Manager, transition: &CoordinatedTransition) -> Result<()> {
    apply_coordinated_with_registry(m, transition, &mut || m.lock("registry.lock"))
}
fn apply_coordinated_with_registry(m: &Manager, transition: &CoordinatedTransition,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    match transition.kind {
        TransitionKind::Update => {
            setup_install::apply_plan(&transition.setup)?;
            for publication in &transition.receipt.transitions {
                m.commit_package_publication_with_registry(publication, &mut *acquire_registry)?;
            }
            require(transition.setup.is_after()?, "package_transition_incomplete")?;
            m.verify_package_publication_snapshot_locked(
                &transition.receipt.after_publications, &acquire_registry()?)
        }
        TransitionKind::Restore => {
            for publication in transition.receipt.transitions.iter().rev() {
                m.restore_package_publication_with_registry(publication, &mut *acquire_registry)?;
            }
            m.verify_package_publication_snapshot_locked(
                &transition.receipt.before_publications, &acquire_registry()?)?;
            setup_install::apply_restore_plan(&transition.setup)
        }
    }
}

fn restore_coordinated_origin(m: &Manager,
    transition: &CoordinatedTransition) -> Result<()> {
    restore_coordinated_origin_with_registry(m, transition, &mut || m.lock("registry.lock"))
}
fn restore_coordinated_origin_with_registry(m: &Manager,
    transition: &CoordinatedTransition,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    match transition.kind {
        TransitionKind::Update => {
            for publication in transition.receipt.transitions.iter().rev() {
                m.restore_package_publication_with_registry(publication, &mut *acquire_registry)?;
            }
            m.verify_package_publication_snapshot_locked(
                &transition.receipt.before_publications, &acquire_registry()?)?;
            setup_install::restore_plan(&transition.setup)?;
            require(transition.setup.is_before()?, "package_transition_recovery_incomplete")
        }
        TransitionKind::Restore => {
            setup_install::restore_successor_plan(&transition.setup)?;
            for publication in &transition.receipt.transitions {
                m.commit_package_publication_with_registry(publication, &mut *acquire_registry)?;
            }
            require(transition.setup.is_before()?, "package_transition_recovery_incomplete")?;
            m.verify_package_publication_snapshot_locked(
                &transition.receipt.after_publications, &acquire_registry()?)
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

#[cfg(test)]
fn require_service_transition_coherent_at(m: &Manager, selected: &Software,
    home: &Path) -> Result<()> {
    require_service_transition_coherent_with_registry(m, selected, home,
        &mut || m.lock("registry.lock"))
}
fn require_service_transition_coherent_with_registry(m: &Manager, selected: &Software,
    home: &Path, acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    if !journal.try_exists()? { return Ok(()); }
    let value: serde_json::Value = read_json(&journal)?;
    require(matches!(value.get("schema").and_then(serde_json::Value::as_u64),Some(2 | 3)),
        "package_transition_needs_recovery")?;
    let transition: CoordinatedTransition = serde_json::from_value(value)?;
    validate_coordinated(m, home, &transition)?;
    let publications = m.package_publication_snapshot_locked(&acquire_registry()?)?;
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

pub(super) fn reconcile_service_startup(m: &Manager,
    selected: &Software) -> Result<()> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    reconcile_service_startup_at(m, selected, &home, bounded_registry(m))
}
fn reconcile_service_startup_at(m: &Manager, selected: &Software,
    home: &Path, mut acquire_registry: impl FnMut() -> Result<Lock>) -> Result<()> {
    if !m.root.join("package-transition.json").try_exists()? { return m.reconcile(); }
    require_service_transition_coherent_with_registry(m, selected, home, &mut acquire_registry)?;
    m.reconcile_with_registry(acquire_registry)
}

fn complete_coordinated(m: &Manager, home: &Path, service: &impl ServiceControl,
    transition: &CoordinatedTransition,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    apply_coordinated_with_registry(m, transition, acquire_registry)?;
    let selected = coordinated_final_software(transition);
    start_and_health(service, m, home, selected)?;
    settle_resume_handoff_with_registry(m,service,transition,acquire_registry)?;
    if transition.kind == TransitionKind::Update {
        retain_transition_receipt(m,transition)?;
    }
    fs::remove_file(&journal)?;
    fs::File::open(&m.root)?.sync_all()?;
    Ok(())
}

fn retain_transition_receipt(m: &Manager, transition: &CoordinatedTransition) -> Result<()> {
    let mut receipt = transition.receipt.clone();
    if let Some(handoff) = &transition.resume_handoff {
        receipt.schema = 2;
        receipt.resume_history = Some(ResumeHistory {source:transition.recovery_before.clone(),
            captured:handoff.captured.clone()});
    }
    retain_refresh_receipt(m,&receipt)
}
fn validate_resume_handoff(m: &Manager, service: &impl ServiceControl,
    transition: &CoordinatedTransition) -> Result<()> {
    if let Some(handoff) = &transition.resume_handoff {
        require(service.vendor_retired(m)? && onboarding::all_retired(m)?,
            "package_resume_vendor_not_retired")?;
        operator_cli::validate_package_resume_with(m,&handoff.captured,
            &transition.recovery_before,handoff.settling,|op|service.worker_live(op))?;
    }
    Ok(())
}
fn settle_resume_handoff(m: &Manager, service: &impl ServiceControl,
    transition: &CoordinatedTransition) -> Result<()> {
    settle_resume_handoff_with_registry(m, service, transition, &mut || m.lock("registry.lock"))
}
fn settle_resume_handoff_with_registry(m: &Manager, service: &impl ServiceControl,
    transition: &CoordinatedTransition,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    let Some(handoff) = &transition.resume_handoff else {return Ok(())};
    let owners = service.clean_idle(m)?;
    {
        let registry = acquire_registry()?;
        registry.require_registry(m)?;
        require(operator_cli::all_operations_finished(m)?,"package_operator_active_or_ambiguous")?;
        m.require_inactive(None)?;
        require(owners == capacity::owners(m)?,"package_owners_changed")?;
        require(transition.setup.is_after()?
            && old_software(m)?.as_ref() == Some(coordinated_final_software(transition)),
            "package_resume_target_changed")?;
        m.verify_package_publication_snapshot_locked(match transition.kind {
            TransitionKind::Update => &transition.receipt.after_publications,
            TransitionKind::Restore => &transition.receipt.before_publications,
        }, &registry)?;
        require(operator_cli::pending_transactions(m)? == 0,"package_transaction_pending")?;
        require(onboarding::all_retired(m)? && service.vendor_retired(m)?,
            "package_resume_vendor_not_retired")?;
        validate_resume_handoff(m,service,transition)?;
        // Persist ownership of the compare-and-consume crash window first.
        let mut settling = transition.clone();
        settling.resume_handoff.as_mut().ok_or("package_resume_missing")?.settling = true;
        atomic_json(&m.root.join("package-transition.json"),&settling)?;
        operator_cli::consume_package_resume(m,&handoff.captured)?;
    }
    Ok(())
}
fn restore_resume_handoff(m: &Manager, transition: &CoordinatedTransition) -> Result<()> {
    if let Some(handoff) = &transition.resume_handoff {
        operator_cli::restore_package_resume(m,&handoff.captured)?;
    }
    Ok(())
}
fn predecessor_readable_transition(transition: &CoordinatedTransition) -> CoordinatedTransition {
    let mut old = transition.clone();
    old.schema = 2;
    old.resume_handoff = None;
    old.receipt.schema = 1;
    old.receipt.resume_history = None;
    old
}
fn restore_origin_service(m: &Manager, home: &Path, service: &impl ServiceControl,
    transition: &CoordinatedTransition) -> Result<()> {
    restore_resume_handoff(m,transition)?;
    let Some(handoff) = &transition.resume_handoff else {
        return start_and_health(service,m,home,&transition.recovery_before);
    };
    if handoff.origin_active {
        // Old strict decoders see only their original journal/receipt contract.
        // A paused origin keeps its handoff journal until verified retirement.
        atomic_json(&m.root.join("package-transition.json"),&predecessor_readable_transition(transition))?;
        start_and_health(service,m,home,&transition.recovery_before)
    } else {
        reload_and_verify(service,home,Some(&transition.recovery_before))?;
        require_service_stopped(service)
    }
}

fn recover_coordinated_error(m: &Manager, home: &Path,
    service: &impl ServiceControl, transition: &CoordinatedTransition,
    original: Box<dyn std::error::Error + Send + Sync>,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    let recovery = (|| -> Result<()> {
        let observed: CoordinatedTransition = read_json(&journal)?;
        let mut expected = transition.clone();
        if let (Some(a),Some(b)) = (&mut expected.resume_handoff,&observed.resume_handoff) {
            a.settling = b.settling;
        }
        require(serde_json::to_value(&expected)? == serde_json::to_value(&observed)?,
            "package_transition_changed")?;
        validate_resume_handoff(m,service,&observed)?;
        stop_transition_service_with_registry(m, home, service,
            &[&transition.recovery_before, coordinated_final_software(transition)],
            &transition.receipt.transitions, acquire_registry)?;
        restore_coordinated_origin_with_registry(m, transition, acquire_registry)?;
        restore_origin_service(m,home,service,transition)?;
        m.verify_package_publication_snapshot_locked(
            coordinated_origin_publications(transition), &acquire_registry()?)?;
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

#[cfg(test)]
fn run_coordinated(m: &Manager, home: &Path, service: &impl ServiceControl,
    transition: CoordinatedTransition, restart_prior: bool) -> Result<()> {
    run_coordinated_with_registry(m, home, service, transition, restart_prior,
        &mut || m.lock("registry.lock"), &mut || m.lock("registry.lock"))
}
fn run_coordinated_with_registry(m: &Manager, home: &Path, service: &impl ServiceControl,
    mut transition: CoordinatedTransition, restart_prior: bool,
    acquire_registry: &mut impl FnMut() -> Result<Lock>,
    acquire_recovery_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    let journal = m.root.join("package-transition.json");
    let prepared = (|| -> Result<()> {
        if transition.resume_handoff.is_none() {
            if transition.kind == TransitionKind::Restore {
                operator_cli::require_no_package_resume(m)?;
            }
            if let Some(captured) = operator_cli::capture_package_resume_with(m,
                &transition.recovery_before,|op|service.worker_live(op))? {
                transition.schema = 3;
                transition.resume_handoff = Some(ResumeHandoff {captured,
                    origin_active:restart_prior,settling:false});
            }
        }
        validate_coordinated(m, home, &transition)?;
        validate_resume_handoff(m,service,&transition)?;
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
                &transition, original, acquire_recovery_registry);
        }
        return restart_prior_after_error(m, home, service,
            &transition.recovery_before, original, restart_prior);
    }
    match complete_coordinated(m, home, service, &transition, acquire_registry) {
        Ok(()) => Ok(()),
        Err(original) => recover_coordinated_error(m, home, service,
            &transition, original, acquire_recovery_registry),
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

fn stop_for_coordinated_with_registry(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl, prior: &Software,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<bool> {
    let before = service.show()?;
    let was_active = before.active == "active";
    match stop_for_repair_locked_with_registry(m, home, inputs, owner, service, acquire_registry) {
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
    // A pending service reservation refuses anyway; settle only if it may help.
    operator_cli::require_no_package_resume(m)?;
    let settled = settle_interrupted(m, service);
    settled_then(settled, adopt_settled(m, home, inputs, owner, service))
}
fn adopt_settled(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    with_locks(m, || {
        operator_cli::require_no_package_resume(m)?;
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m,service)?;
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
    operator_cli::require_no_package_resume(m)?;
    let settled = settle_interrupted(m, service);
    settled_then(settled, rollback_settled(m, home, service))
}
fn rollback_settled(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<()> {
    with_locks(m, || {
        operator_cli::require_no_package_resume(m)?;
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m,service)?;
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
    update_from_with_registry(m, home, inputs, owner, service,
        bounded_registry(m), bounded_registry(m))
}
fn bounded_registry(m: &Manager) -> impl FnMut() -> Result<Lock> + '_ {
    registry_wait_budget(m, operator_cli::OPERATOR_WAIT)
}
fn registry_wait_budget(m: &Manager, mut remaining_wait: Duration)
    -> impl FnMut() -> Result<Lock> + '_ {
    move || {
        let started = Instant::now();
        let result = m.lock_bounded(operator_model::OperatorLock::Registry,
            operator_model::LockPurpose::OperatorValidationReadback, None, remaining_wait);
        remaining_wait = remaining_wait.saturating_sub(started.elapsed());
        result.map(|(guard, _)| guard)
    }
}
fn update_from_with_registry(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl,
    acquire_registry: impl FnMut() -> Result<Lock>,
    acquire_recovery_registry: impl FnMut() -> Result<Lock>) -> Result<()> {
    // A package transition is one verification lifetime, including recovery.
    // Each reused digest still reopens and checks the exact file identity.
    linux_vst_bridge::with_launch_verification(|| {
        let _package = m.lock("package.lock")?;
        let _setup = m.lock("setup.lock")?;
        // Before capture: an interrupted owner must be terminal and its app
        // session retired before its service reservation can be handed over.
        // Settling restores no service, so it needs no resume lock.
        let settled = settle_interrupted(m, service);
        settled_then(settled, update_settled(m, home, inputs, owner, service,
            acquire_registry, acquire_recovery_registry))
    })
}
fn update_settled(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl,
    mut acquire_registry: impl FnMut() -> Result<Lock>,
    mut acquire_recovery_registry: impl FnMut() -> Result<Lock>) -> Result<()> {
    let _resume = operator_cli::resume_lock(m)?;
    let current = old_software(m)?.ok_or("package_not_installed")?;
    let captured = operator_cli::capture_package_resume_with(m,&current,|op|service.worker_live(op))?;
    let restart_prior = stop_for_coordinated_with_registry(m, home, inputs, owner, service,
        &current, &mut acquire_registry)?;
    let prepared = (|| -> Result<CoordinatedTransition> {
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m,service)?;
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
        let needs_refresh = preparation::registry_requires_loaded_engine_refresh_with_registry(
            m, &mut acquire_registry)?;
        require(target != current || needs_refresh,
            "package_update_not_required")?;
        let predecessor = if target == current && needs_refresh {
            verify_generation(m, &current)?.predecessor
                .ok_or("package_restore_predecessor_unavailable")?
        } else { current.clone() };
        verify_software_identity(m, &predecessor)?;
        let receipt = prepare_refresh_receipt(m, &target, predecessor,
            &mut acquire_registry)?;
        for state in &receipt.before_publications {
            if state.entry.publication == Publication::Published {
                let _ = paired_components(m, &receipt.predecessor,
                    &state.entry.registration)?;
            }
        }
        let setup = setup_install::plan(m, home, &target, Some(&current))?;
        Ok(CoordinatedTransition { schema:if captured.is_some(){3}else{2}, id:random_id()?,
            kind:TransitionKind::Update, recovery_before:current.clone(),
            setup, receipt, resume_handoff:captured.clone().map(|captured|ResumeHandoff {
                captured,origin_active:restart_prior,settling:false}) })
    })();
    let transition = match prepared {
        Ok(transition) => transition,
        Err(original) => {
            return restart_prior_after_error(m, home, service, &current,
                original, restart_prior);
        }
    };
    run_coordinated_with_registry(m, home, service, transition, restart_prior,
        &mut acquire_registry, &mut acquire_recovery_registry)
}

fn restore_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    restore_from_with_registry(m, home, inputs, owner, service,
        bounded_registry(m), bounded_registry(m))
}
fn restore_from_with_registry(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl,
    acquire_registry: impl FnMut() -> Result<Lock>,
    acquire_recovery_registry: impl FnMut() -> Result<Lock>) -> Result<()> {
    // A package transition is one verification lifetime, including recovery.
    // Each reused digest still reopens and checks the exact file identity.
    linux_vst_bridge::with_launch_verification(|| {
        let _package = m.lock("package.lock")?;
        let _setup = m.lock("setup.lock")?;
        let settled = settle_interrupted(m, service);
        settled_then(settled, restore_settled(m, home, inputs, owner, service,
            acquire_registry, acquire_recovery_registry))
    })
}
fn restore_settled(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl,
    mut acquire_registry: impl FnMut() -> Result<Lock>,
    mut acquire_recovery_registry: impl FnMut() -> Result<Lock>) -> Result<()> {
    let _resume = operator_cli::resume_lock(m)?;
    // Restore carries no service reservation; the interrupted owner's own
    // restoration is completed first, as its worker would have.
    operator_cli::restore_terminated_owner_locked(m, |operation| service.worker_live(operation))?;
    operator_cli::require_no_package_resume(m)?;
    let current = old_software(m)?.ok_or("package_not_installed")?;
    let restart_prior = stop_for_coordinated_with_registry(m, home, inputs, owner, service,
        &current, &mut acquire_registry)?;
    let prepared = (|| -> Result<CoordinatedTransition> {
        require_service_stopped(service)?;
        require(!m.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        preflight(m,service)?;
        let snapshot = m.package_publication_snapshot_locked(&acquire_registry()?)?;
        let matching = matching_refresh_receipts(m, &current, &snapshot)?;
        require(matching.len() <= 1, "package_restore_receipt_ambiguous")?;
        let mut receipt = if let Some(receipt) = matching.into_iter().next() {
            receipt
        } else {
            require(!snapshot.iter().any(|state|
                state.entry.publication == Publication::Published),
                "package_restore_receipt_unavailable")?;
            let predecessor = verify_generation(m, &current)?.predecessor
                .ok_or("package_no_predecessor")?;
            RefreshReceipt { schema:1, predecessor, successor:current.clone(),
                before_publications:snapshot.clone(), after_publications:snapshot,
                transitions:Vec::new(), resume_history:None }
        };
        // Completed handoff history is provenance, never a new obligation.
        // A retained predecessor must see its original nested receipt schema.
        receipt.schema = 1;
        receipt.resume_history = None;
        verify_software_identity(m, &receipt.predecessor)?;
        let setup = setup_install::plan(m, home, &receipt.predecessor, Some(&current))?;
        Ok(CoordinatedTransition { schema:2, id:random_id()?,
            kind:TransitionKind::Restore, recovery_before:current.clone(),
            setup, receipt, resume_handoff:None })
    })();
    let transition = match prepared {
        Ok(transition) => transition,
        Err(original) => {
            return restart_prior_after_error(m, home, service, &current,
                original, restart_prior);
        }
    };
    run_coordinated_with_registry(m, home, service, transition, restart_prior,
        &mut acquire_registry, &mut acquire_recovery_registry)
}

fn recover_from(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<bool> {
    let _package = m.lock("package.lock")?;
    let _setup = m.lock("setup.lock")?;
    let _resume = operator_cli::resume_lock(m)?;
    let journal = m.root.join("package-transition.json");
    if !journal.try_exists()? { return Ok(false); }
    let value: serde_json::Value = read_json(&journal)?;
    if !matches!(value.get("schema").and_then(serde_json::Value::as_u64),Some(2 | 3)) {
        let plan = setup_install::pending_plan(m, home)?;
        let (before, after) = setup_install::plan_software(&plan)?;
        let mut allowed = vec![&after];
        if let Some(before) = before.as_ref() { allowed.push(before); }
        stop_transition_service(m, home, service, &allowed, &[])?;
        preflight(m,service)?;
        let recovered = setup_install::recover_journaled(m, home,
            |selected| reload_and_verify(service, home, selected))?;
        if let Some(selected) = old_software(m)? {
            start_and_health(service, m, home, &selected)?;
        }
        return Ok(recovered);
    }
    let transition: CoordinatedTransition = serde_json::from_value(value)?;
    validate_coordinated(m, home, &transition)?;
    validate_resume_handoff(m,service,&transition)?;
    stop_transition_service(m, home, service,
        &[&transition.recovery_before, coordinated_final_software(&transition)],
        &transition.receipt.transitions)?;
    recovery_preflight(m, &transition,service)?;
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
        settle_resume_handoff(m,service,&transition)?;
        if transition.kind == TransitionKind::Update {
            retain_transition_receipt(m,&transition)?;
        }
    } else {
        restore_coordinated_origin(m, &transition)?;
        restore_origin_service(m,home,service,&transition)?;
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
    fn clean_idle(&self, m: &Manager) -> Result<Vec<capacity::Owner>>;
    fn vendor_retired(&self, m: &Manager) -> Result<bool> {operator_cli::vendor_retired(m)}
    fn reverify_vendor(&self, m: &Manager, limit: Duration) -> Result<()> {
        vendor_cli::reverify_interrupted(m, limit, running_package_census_tools)
    }
    fn restore_reservation(&self, m: &Manager) -> Result<()> {
        operator_cli::restore_terminated_owner(m, |operation| self.worker_live(operation))
    }
    fn worker_live(&self, operation: &str) -> Result<bool> {
        require(valid_hex(operation,32),"package_resume_identity")?;
        match readiness::bounded_user_unit_state(&format!("linux-vst-bridge-operator-{operation}.service"))?.as_str() {
            "inactive" | "failed" => Ok(false),
            "active" | "activating" | "deactivating" | "reloading" => Ok(true),
            _ => Err("package_resume_worker_state_unavailable".into()),
        }
    }
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
fn clean_keepers(reply: &serde_json::Value) -> Result<Vec<capacity::Owner>> {
    require(reply["capacity"]["cleanup_unconfirmed"] == false,
        "package_resume_cleanup_unconfirmed")?;
    stoppable_keepers(reply)
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
    fn clean_idle(&self, m: &Manager) -> Result<Vec<capacity::Owner>> {
        clean_keepers(&capacity_reply(m)?)
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
/// First-run classification. It changes nothing except settling work whose
/// owner ended with the machine (see `settle_interrupted`); without a package
/// record it settles nothing. An absent package record is admitted as legacy
/// only for the older selected helper layout; a damaged package generation
/// cannot become an adoptable predecessor.
fn bootstrap_status_from(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<ActivationStatus> {
    let settled = if !m.root.join("package-transition.json").try_exists()?
        && m.root.join("software.json").try_exists()? {
        settle_interrupted(m, service)
    } else { Ok(()) };
    // Like service readback, allow two seconds of registry waiting, cumulative
    // across guards. The frontend allows status 90 s, covering the settling.
    let mut acquire_registry = registry_wait_budget(m, Duration::from_secs(2));
    settled_then(settled, bootstrap_status_with_registry(m, home, inputs, owner, service, &mut acquire_registry))
}
fn bootstrap_status_with_registry(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<ActivationStatus> {
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
    let refresh_required = preparation::registry_requires_loaded_engine_refresh_record_with_registry(
        m, &mut *acquire_registry)?;
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
        let current_publications = m.package_publication_record_snapshot_locked(&acquire_registry()?)?;
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
                stop_gate(m, service, acquire_registry)?;
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
                let registry = acquire_registry()?;
                registry.require_registry(m)?;
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
fn stop_gate(m: &Manager, service: &impl ServiceControl,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<Vec<capacity::Owner>> {
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
        let registry = acquire_registry()?;
        registry.require_registry(m)?;
        require(operator_cli::all_operations_finished(m)?,"package_operator_active_or_ambiguous")?;
        m.require_inactive(None)?;
        require(capacity::owners(m)? == before, "package_owners_changed")?;
        require(operator_cli::pending_transactions(m)? == 0,
            "package_transaction_pending")?;
    }
    require(onboarding::all_retired(m)?, "package_installer_owner_active")?;
    require(service.vendor_retired(m)?, "package_vendor_owner_active")?;
    daw_workspace::package_idle(m)?;
    transport_storage::require_no_sessions()?;
    let after = service.idle(m)?;
    require(after == before, "package_owners_changed")?;
    Ok(after)
}
#[cfg(test)]
fn stop_selected_service(m: &Manager, home: &Path, selected: &Software,
    service: &impl ServiceControl,
    observed_keepers: &[capacity::Owner]) -> Result<()> {
    stop_selected_service_with_registry(m, home, selected, service, observed_keepers,
        &mut || m.lock("registry.lock"))
}
fn stop_selected_service_with_registry(m: &Manager, home: &Path, selected: &Software,
    service: &impl ServiceControl, observed_keepers: &[capacity::Owner],
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    // Bounded systemctl stop intentionally runs under the admission lock,
    // matching operator suspension. Read-only status probes never do this.
    let registry = acquire_registry()?;
    registry.require_registry(m)?;
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
    let settled = settle_interrupted(m, service);
    settled_then(settled, stop_for_repair_locked(m, home, inputs, owner, service))
}
fn stop_for_repair_locked(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl) -> Result<()> {
    stop_for_repair_locked_with_registry(m, home, inputs, owner, service,
        &mut || m.lock("registry.lock"))
}
fn stop_for_repair_locked_with_registry(m: &Manager, home: &Path, inputs: &Inputs, owner: u32,
    service: &impl ServiceControl,
    acquire_registry: &mut impl FnMut() -> Result<Lock>) -> Result<()> {
    let (_, package_sha) = read_manifest(inputs, owner)?;
    let status = bootstrap_status_with_registry(m, home, inputs, owner, service, acquire_registry)?;
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
        let registry = acquire_registry()?;
        registry.require_registry(m)?;
        m.require_inactive(None)?;
        reconcile_stopped_service(m, service)?;
        require(capacity::owners(m)?.is_empty(), "package_owner_active")?;
        drop(registry);
        let after = bootstrap_status_with_registry(m, home, inputs, owner, service, acquire_registry)?;
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
    let before = bootstrap_status_with_registry(m, home, inputs, owner, service, acquire_registry)?;
    require(read_manifest(inputs, owner)?.1 == package_sha
        && before.state == status.state && before.package_version == status.package_version,
        "package_stop_state_changed")?;
    let observed_keepers = service.idle(m)?;
    let selected = old_software(m)?.ok_or("package_not_installed")?;
    stop_selected_service_with_registry(m, home, &selected, service, &observed_keepers, acquire_registry)?;
    let after = bootstrap_status_with_registry(m, home, inputs, owner, service, acquire_registry)?;
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
    // Settling precedes the activation readback; it restores no service.
    let settled = settle_interrupted(m, service);
    settled_then(settled, activate_settled(m, home, service))?;
    // A task that ended with the machine may still hold this service's
    // reservation; its owner would have released it once the service ran.
    if let Err(error) = service.restore_reservation(m) {
        eprintln!("An interrupted bridge task's service reservation was not released: {error}");
    }
    Ok(())
}
fn activate_settled(m: &Manager, home: &Path, service: &impl ServiceControl) -> Result<()> {
    let (selected, version, state) = selected_activation(m, home, service)?;
    let starting = state.active != "active";
    if starting {
        preflight(m,service)?;
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
    // Test30's closed wire shapes, independent of the new optional fields.
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LegacyReceipt {
        schema:u32, predecessor:Software, successor:Software,
        before_publications:Vec<publication::PublicationState>,
        after_publications:Vec<publication::PublicationState>,
        transitions:Vec<publication::PreparedTransition>,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LegacyTransition {
        schema:u32,id:String,kind:TransitionKind,recovery_before:Software,
        setup:setup_install::Plan,receipt:LegacyReceipt,
    }
    type ReadbackHook = Box<dyn FnOnce(&Manager)>;
    type VendorHook = Box<dyn Fn(&Manager) -> Result<()>>;
    struct FakeService {
        home: PathBuf,
        loaded: RefCell<UnitReadback>,
        fail_reload: Cell<bool>,
        fail_show: Cell<bool>,
        stale_reload: Cell<bool>,
        fail_start: Cell<bool>,
        fail_starts: Cell<usize>,
        fail_health: Cell<bool>,
        fail_health_reads: Cell<usize>,
        fail_idle: Cell<bool>,
        fail_clean: Cell<bool>,
        worker_active: Cell<bool>,
        worker_unavailable: Cell<bool>,
        reverify: RefCell<Option<VendorHook>>,
        reverifications: Cell<usize>,
        reservations_restored: Cell<usize>,
        before_clean: RefCell<Option<ReadbackHook>>,
        legacy_start_manager: RefCell<Option<PathBuf>>,
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
                fail_health: Cell::new(false), fail_health_reads: Cell::new(0), fail_idle: Cell::new(false),
                fail_clean: Cell::new(false),worker_active:Cell::new(false),
                worker_unavailable:Cell::new(false),reverify:RefCell::new(None),
                reverifications:Cell::new(0),reservations_restored:Cell::new(0),
                before_clean:RefCell::new(None),
                legacy_start_manager:RefCell::new(None),
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
            if let Some(manager) = self.legacy_start_manager.borrow().as_ref() {
                if effective_exec_is(&self.loaded.borrow().exec,manager) {
                    let journal = self.home.parent().ok_or("test_home")?
                        .join("managed/package-transition.json");
                    if journal.try_exists()? {
                        let old: LegacyTransition = read_json(&journal)?;
                        require(old.schema == 2 && old.receipt.schema == 1,
                            "legacy_package_transition_schema")?;
                    }
                }
            }
            let mut state = self.loaded.borrow_mut();
            require(state.load == "loaded" && matches!(state.active.as_str(), "inactive" | "failed"),
                "package_service_effective_route_mismatch")?;
            state.active = "active".into();
            self.starts.set(self.starts.get() + 1);
            Ok(())
        }
        fn healthy(&self, _: &Manager) -> Result<()> {
            if self.fail_health_reads.get() > 0 {
                self.fail_health_reads.set(self.fail_health_reads.get() - 1);
                return Err("package_service_health_unavailable".into());
            }
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
        fn clean_idle(&self, m: &Manager) -> Result<Vec<capacity::Owner>> {
            if let Some(change) = self.before_clean.borrow_mut().take() {change(m);}
            require(!self.fail_clean.get(),"package_resume_cleanup_unconfirmed")?;
            self.idle(m)
        }
        fn worker_live(&self, _: &str) -> Result<bool> {
            require(!self.worker_unavailable.get(),"package_resume_worker_state_unavailable")?;
            Ok(self.worker_active.get())
        }
        fn reverify_vendor(&self, m: &Manager, limit: Duration) -> Result<()> {
            self.reverifications.set(self.reverifications.get() + 1);
            match self.reverify.borrow().as_ref() {
                Some(hook) => hook(m),
                None => vendor_cli::reverify_interrupted(m, limit, || Err("test_package_census_tools".into())),
            }
        }
        fn restore_reservation(&self, m: &Manager) -> Result<()> {
            operator_cli::restore_terminated_owner_with(m, |operation| self.worker_live(operation),
                || { self.reservations_restored.set(self.reservations_restored.get() + 1); Ok(()) })
        }
        fn vendor_retired(&self, m: &Manager) -> Result<bool> {
            operator_cli::vendor_retired_with(m,||Ok(false))
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
            let schema = if self.inputs.path("session.py").unwrap().exists() { 3 }
                else if self.inputs.path("preparation-kit.zip").unwrap().exists() { 2 } else { 1 };
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
            if path.exists() {fs::set_permissions(&path,fs::Permissions::from_mode(0o600)).unwrap();}
            let host_path=self.inputs.path("host.exe").unwrap();
            let source_path=self.inputs.path("host-source-manifest.json").unwrap();
            test_fixture::reusable_kit(&path,&Artifact {sha256:digest(&host_path).unwrap(),path:host_path},
                &Artifact {sha256:digest(&source_path).unwrap(),path:source_path},std::str::from_utf8(data).unwrap());
            self.write_manifest();
        }
        fn common_kit(&self, variant:&str) -> PathBuf {
            self.add_kit(b"common package fixture");
            let build=self.base.outer.join("wine-build.json");
            atomic_json(&build,&serde_json::json!({"schema":1,"kind":"wine_build",
                "helper_abi":1,"wine_revision":"22".repeat(20)})).unwrap();
            let kit=self.inputs.path("preparation-kit.zip").unwrap();
            fs::set_permissions(&kit,fs::Permissions::from_mode(0o600)).unwrap();
            let script=r#"import sys,json,zipfile,hashlib,pathlib
kit,build,variant=sys.argv[1:];sha=lambda b:hashlib.sha256(b).hexdigest()
with zipfile.ZipFile(kit) as z:files={n:z.read(n) for n in z.namelist()}
helper=json.loads(files['runtime/direct-audio-helper.json'])
helper.update(schema=2,runner_build_sha256=sha(pathlib.Path(build).read_bytes()),runner_wine_revision='22'*20)
if variant=='host':
 files['runtime/host.exe']=b'other inspection host';helper['host_sha256']=sha(files['runtime/host.exe'])
if variant=='source':files['runtime/host-source-manifest.json']=b'other inspection source'
if variant=='legacy':
 helper.pop('runner_build_sha256');helper['schema']=1;helper['runner_wine_revision']='46b29104e3741fe23bf5e2547196a253aab88c89'
 files['runtime/host.exe']=b'legacy separate inspection host';helper['host_sha256']=sha(files['runtime/host.exe'])
files['runtime/direct-audio-helper.json']=json.dumps(helper).encode()
if variant=='partial':del files['runtime/x86_64-unix/lvb-direct-wait.so']
recipe=json.loads(files['recipe.json']);recipe['files']={n:sha(b) for n,b in files.items() if n!='recipe.json'}
files['recipe.json']=json.dumps(recipe).encode()
for name in ['host','source']:
 key='runtime/'+('host.exe' if name=='host' else 'host-source-manifest.json')
 if variant=='missing-'+name:del files[key]
 if variant=='changed-'+name:files[key]=b'actual member differs from claimed recipe'
with zipfile.ZipFile(kit,'w') as z:
 for n,b in files.items():z.writestr(n,b)
"#;
            assert!(Command::new("python3").args(["-I","-c",script]).arg(&kit).arg(&build)
                .arg(variant).status().unwrap().success());
            fs::set_permissions(&kit,fs::Permissions::from_mode(0o400)).unwrap();
            self.write_manifest();build
        }
        fn add_kit_archive(&self, data:&[u8]) {
            let path=self.inputs.path("preparation-kit.zip").unwrap();
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if path.exists() {fs::set_permissions(&path,fs::Permissions::from_mode(0o600)).unwrap();}
            fs::write(&path,data).unwrap();
            fs::set_permissions(&path,fs::Permissions::from_mode(0o400)).unwrap();
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

    #[test]
    fn initial_common_package_host_passes_real_pair_preflight_and_never_repairs() {
        let f=Fixture::new();let build=f.common_kit("valid");
        f.adopt().unwrap();let sw=f.current();
        let dir=sw.manager.path.parent().unwrap();
        assert_eq!(sw.host.path,dir.join("host.exe"));
        assert_eq!(sw.source_manifest.path,dir.join("host-source-manifest.json"));
        assert_eq!(digest(&f.inputs.path("host.exe").unwrap()).unwrap(),sw.host.sha256);
        let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
        let script=r#"import sys,json,pathlib,hashlib
sys.path.insert(0,sys.argv[1]);import session
sw=json.loads(pathlib.Path(sys.argv[2]).read_text());build=pathlib.Path(sys.argv[3]);root=build.parent/'common';root.mkdir(mode=0o700)
data=build.read_bytes();declaration=root/'wine-build.json';declaration.write_bytes(data);declaration.chmod(0o400)
marker=root/'native-command-session.json';marker.write_text(json.dumps(dict(schema=2,kind='native_proton_command_session',client_sha256='33'*32,service_sha256='44'*32,wine_build_sha256=hashlib.sha256(data).hexdigest())));marker.chmod(0o400)
artifact=lambda p:dict(path=str(p),sha256=hashlib.sha256(p.read_bytes()).hexdigest())
reg=dict(host=sw['host'],environment=dict(runner=dict(proton=str(root/'proton'),files=[artifact(marker),artifact(declaration)])))
env=session.direct_audio_environment(reg,dict(WINEDLLOVERRIDES='winebus.sys=d'))
assert env['WINEDLLPATH']==str(pathlib.Path(sw['host']['path']).parent)
assert env['WINEDLLOVERRIDES']=='winebus.sys=d;lvb-direct-wait=b'
"#;
        assert!(Command::new("python3").args(["-I","-c",script]).arg(root)
            .arg(f.base.m.root.join("software.json")).arg(build).status().unwrap().success());
        let before=test_fixture::snapshot(&f.base.m.root);
        validate_generation_record(&f.base.m,&sw).unwrap();verify_generation(&f.base.m,&sw).unwrap();
        f.adopt().unwrap();assert_eq!(test_fixture::snapshot(&f.base.m.root),before);
        fs::remove_file(dir.join("x86_64-unix/lvb-direct-wait.so")).unwrap();
        let damaged=test_fixture::snapshot(&f.base.m.root);
        assert!(f.adopt().is_err());assert!(validate_generation_record(&f.base.m,&sw).is_err());
        assert_eq!(test_fixture::snapshot(&f.base.m.root),damaged);
    }

    #[test]
    fn common_package_host_mismatch_and_partial_support_leave_current_software_intact() {
        for variant in ["host","source","partial","missing-host","missing-source","changed-host","changed-source"] {
            let f=Fixture::new();f.adopt().unwrap();
            let before=fs::read(f.base.m.root.join("software.json")).unwrap();
            f.common_kit(variant);assert!(f.adopt().is_err(),"{variant}");
            assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(),before);
            assert!(!fs::read_dir(f.base.m.root.join("software")).unwrap()
                .any(|e|e.unwrap().file_name().to_string_lossy().starts_with("stage-")));
        }
        let f=Fixture::new();f.common_kit("legacy");f.adopt().unwrap();
        let sw=f.current();assert_eq!(fs::read(&sw.host.path).unwrap(),b"host");
        assert!(!sw.host.path.with_file_name("runtime.json").exists());
        verify_generation(&f.base.m,&sw).unwrap();
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
    fn managed_frontend_requests_follow_selected_content_after_update_and_restore() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let first = f.current().operator_frontend.unwrap();
        let first_meta = fs::metadata(&first.path).unwrap();
        require_selected_frontend(&f.base.m, &first.path, &first_meta).unwrap();
        // A manager-only update still uses the same frontend content.
        f.replace(NAMES[0], b"manager-only update", true);
        f.adopt().unwrap();
        require_selected_frontend(&f.base.m, &first.path, &first_meta).unwrap();
        f.replace(NAMES[1], b"updated frontend", true);
        f.adopt().unwrap();
        let second = f.current().operator_frontend.unwrap();
        let second_meta = fs::metadata(&second.path).unwrap();
        let mut work_started = false;
        let result = require_selected_frontend(&f.base.m, &first.path, &first_meta)
            .map(|()| { work_started = true; });
        assert!(result.unwrap_err().to_string().starts_with("operator_frontend_superseded: "));
        assert!(!work_started, "obsolete requests must stop before readiness or mutation");
        require_selected_frontend(&f.base.m, &second.path, &second_meta).unwrap();
        rollback_from(&f.base.m, &f.home, &f.service).unwrap();
        require_selected_frontend(&f.base.m, &first.path, &first_meta).unwrap();
        assert!(require_selected_frontend(&f.base.m, &second.path, &second_meta).is_err());
        first.verify().unwrap();
        second.verify().unwrap();
    }

    #[test]
    fn frontend_binding_keeps_standalone_clients_and_refuses_changed_owned_identity() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let frontend = f.current().operator_frontend.unwrap();
        let running = fs::metadata(&frontend.path).unwrap();
        for path in [f.home.join(NAMES[1]), f.base.m.root.join("software/other-client")]
        {
            require_selected_frontend(&f.base.m, &path, &running).unwrap();
        }
        let foreign = fs::metadata(f.inputs.path(NAMES[1]).unwrap()).unwrap();
        assert!(require_selected_frontend(&f.base.m, &frontend.path, &foreign)
            .unwrap_err().to_string().contains("package_frontend_executable_changed"));
        let record = frontend.path.parent().unwrap().join("package-generation.json");
        fs::set_permissions(&record, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(require_selected_frontend(&f.base.m, &frontend.path, &running).is_err());
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
    fn operation_interrupted_with_its_worker_unit_no_longer_blocks_update() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        f.replace("linux-vst-bridge", b"successor manager", true);
        // The host froze during an app session: the worker unit and its
        // ExecStopPost vanished, leaving the receipt at vendor_running.
        let id = "7d".repeat(16);
        let directory = f.base.m.root.join("operator").join(&id);
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("request.json"), &serde_json::json!({
            "schema":operator_model::OPERATOR_SCHEMA,"state_token":"00".repeat(32),
            "action":{"kind":"capture_disarm"}})).unwrap();
        let interrupted = serde_json::json!({"schema":1,"operation":id,"state":"vendor_running"});
        atomic_json(&directory.join("result.json"), &interrupted).unwrap();
        let latest = f.base.m.root.join("operator/latest.json");
        atomic_json(&latest, &interrupted).unwrap();
        let settled = f.service.reverifications.get();
        let status = || bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service);
        f.service.worker_unavailable.set(true);
        assert_eq!(status().err().unwrap().to_string(), "package_operator_active_or_ambiguous");
        f.service.worker_unavailable.set(false);
        f.service.worker_active.set(true);
        assert_eq!(status().err().unwrap().to_string(), "package_operator_active_or_ambiguous");
        assert_eq!(read_json::<serde_json::Value>(&directory.join("result.json")).unwrap(), interrupted);
        // A live task may own its app session's writer: no app re-check under it.
        assert_eq!(f.service.reverifications.get(), settled);
        f.service.worker_active.set(false);
        assert_eq!(status().unwrap().state, "update_active");
        let finished: serde_json::Value = read_json(&directory.join("result.json")).unwrap();
        assert_eq!((finished["state"].as_str(), finished["reason"].as_str()),
            (Some("refused"), Some("operator_worker_terminated")));
        assert_eq!(read_json::<serde_json::Value>(&latest).unwrap(), finished);
        assert_eq!(f.service.reverifications.get(), settled + 1);
        stop_for_repair_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
        assert_eq!(f.service.stops.get(), 1);
        // Settling runs once per entry point, never in its nested status checks.
        assert_eq!(f.service.reverifications.get(), settled + 2);
    }

    #[test]
    fn activation_releases_the_reservation_of_a_task_that_ended_with_the_machine() {
        let f = Fixture::new();
        f.adopt().unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        let owner = "7d".repeat(16);
        let directory = f.base.m.root.join("operator").join(&owner);
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("request.json"), &serde_json::json!({
            "schema":operator_model::OPERATOR_SCHEMA,"state_token":"00".repeat(32),
            "action":{"kind":"capture_disarm"}})).unwrap();
        let reservation = f.base.m.root.join("operator/resume.json");
        atomic_json(&reservation, &serde_json::json!({"schema":2,"owner_operation":owner,
            "vendor_operation":null,"resume":true,"software":f.current().manager.sha256})).unwrap();
        // An owner that ended for another reason keeps its reservation.
        atomic_json(&directory.join("result.json"), &serde_json::json!({"schema":1,
            "operation":owner,"state":"refused","reason":"operator_action_failed"})).unwrap();
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert!(reservation.exists());
        atomic_json(&directory.join("result.json"), &serde_json::json!({"schema":1,
            "operation":owner,"state":"refused","reason":"operator_worker_terminated"})).unwrap();
        f.service.worker_active.set(true);
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert!(reservation.exists(), "A running worker keeps its reservation");
        f.service.worker_active.set(false);
        activate_from(&f.base.m, &f.home, &f.service).unwrap();
        assert_eq!(f.service.reservations_restored.get(), 1);
        assert!(!reservation.exists());
    }

    #[test]
    fn gate_census_tools_are_the_verified_files_of_the_running_package() {
        let f = Fixture::new();
        let (supervisor, ownership) = package_census_tools(&f.inputs, f.owner).unwrap();
        for (tool, name) in [(&supervisor, "session.pyc"), (&ownership, "ownership.pyc")] {
            assert_eq!(tool.path, f.inputs.path(name).unwrap());
            assert_eq!(tool.sha256, digest(&tool.path).unwrap());
        }
        // A changed package file is never offered to run.
        f.replace("session.pyc", b"substituted supervisor", false);
        assert!(package_census_tools(&f.inputs, f.owner).is_err());
    }

    #[test]
    fn update_gate_reverifies_an_interrupted_app_session() {
        use super::vendor_cli::interruption_tests as interruption;
        for running in [true, false] {
            let (base, selector, job) = interruption::interrupted_session();
            let f = Fixture::with_base(base);
            *f.service.reverify.borrow_mut() = Some(Box::new(move |m|
                interruption::reverify_with_census(m, interruption::census(running))));
            let unit = vec![format!("linux-vst-bridge-vendor-{selector}.service")];
            let before = fs::read(&job.report).unwrap();
            // An entry point settles, then its gate decides.
            let gate = readiness::with_unit_state_fixture(&unit, "inactive", || {
                let settled = settle_interrupted(&f.base.m, &f.service);
                settled_then(settled, stop_gate(&f.base.m, &f.service, &mut || f.base.m.lock("registry.lock")))
            });
            let retired = readiness::with_unit_state_fixture(&unit, "inactive", ||
                operator_cli::vendor_retired_with(&f.base.m, || Ok(false))).unwrap();
            if running {
                // A plain sentence with Setup's next step, not package_vendor_owner_active.
                assert_eq!(gate.unwrap_err().to_string(),
                    "The app's support service is still running. Restart the computer, then choose Check again.");
                assert_eq!(fs::read(&job.report).unwrap(), before);
                assert!(!retired);
            } else {
                gate.unwrap();
                assert!(retired);
            }
        }
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
    fn bridge_build_names_the_installed_version_and_what_setup_holds() {
        use operator_model::{BridgeBuild, SetupPackage};
        fn build(f: &Fixture, staged: Result<Option<PackageBuild>>,
            system: impl FnOnce() -> Result<Option<(Inputs, u32)>>) -> BridgeBuild {
            bridge_build_from(&f.base.m, &f.current(), staged, system)
        }
        let f = Fixture::new();
        let tree = "cd".repeat(20);
        let staged = |version: &str, tree: &str| -> Result<Option<PackageBuild>> {
            Ok(Some((version.to_owned(), tree.to_owned())))
        };
        let system = || Ok(Some((f.inputs.clone(), f.owner)));
        let unused = || -> Result<Option<(Inputs, u32)>> { panic!("staged package answered") };
        f.adopt().unwrap();
        // Setup holds the installed build itself.
        assert_eq!(build(&f, Ok(None), system), BridgeBuild {
            installed: Some("0.1.0beta1".into()), setup: SetupPackage::Nothing });
        // Only a strictly newer version is an update; numbers compare as numbers.
        let mut manifest = f.manifest();
        manifest.version = "0.1.0beta10".into();
        atomic_json(&f.inputs.manifest(), &manifest).unwrap();
        fs::set_permissions(f.inputs.manifest(), fs::Permissions::from_mode(0o444)).unwrap();
        assert_eq!(build(&f, Ok(None), system).setup, SetupPackage::Newer("0.1.0beta10".into()));
        // The user's staged package is what Setup opens; the system one is not read.
        assert_eq!(build(&f, staged("0.1.0beta0", &tree), unused).setup,
            SetupPackage::Older("0.1.0beta0".into()));
        assert_eq!(build(&f, staged("0.1.0beta1", &tree), unused).setup, SetupPackage::Nothing);
        assert_eq!(build(&f, staged("0.1.0beta1", &"ef".repeat(20)), unused).setup,
            SetupPackage::DifferentBuild("0.1.0beta1".into()));
        // A staged package that cannot be verified is not replaced by the system one.
        assert_eq!(build(&f, Err("user_package_signature".into()), unused).setup,
            SetupPackage::Unchecked);
        // An unknown installed version offers nothing.
        fs::remove_file(f.current().manager.path.parent().unwrap()
            .join("package-generation.json")).unwrap();
        assert_eq!(build(&f, Ok(None), system), BridgeBuild::default());
        assert_eq!(build(&f, staged("9.0", &tree), unused), BridgeBuild::default());
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
        assert_eq!(fs::read(&kit.path).unwrap(), fs::read(f.inputs.path("preparation-kit.zip").unwrap()).unwrap());
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
    fn descriptive_legacy_source_layout_remains_selected_without_a_package_record() {
        let (base,_,_,native)=test_fixture::prepared();
        let f=Fixture::with_base(base);
        f.setup_legacy_with_catalogue(native);
        let retained=f.current();
        assert_eq!(retained.manager.path.parent().unwrap().file_name().unwrap(),"legacy");
        assert!(!retained.manager.path.parent().unwrap().join("package-generation.json").exists());
        assert!(selected_generation(&f.base.m,&retained).unwrap().is_none());
        assert!(selected_generation_record(&f.base.m,&retained).unwrap().is_none());
    }

    #[test]
    fn source_generation_preserves_bytecode_predecessor_and_exact_restore() {
        let f = Fixture::new();
        f.add_kit(b"same reusable host across source migration");
        f.adopt().unwrap();
        let old = f.current();
        let old_selection = fs::read(f.base.m.root.join("software.json")).unwrap();
        let old_record = old.manager.path.parent().unwrap().join("package-generation.json");
        let old_bytes = fs::read(&old_record).unwrap();
        assert_eq!(verify_generation(&f.base.m, &old).unwrap().schema, 2);
        assert_eq!(linux_vst_bridge::supervisor::arguments(&old.supervisor,&old.ownership).unwrap(),
            vec![old.supervisor.path.clone().into_os_string()]);
        for (name, bytes) in [("session.py", include_bytes!("../runtime/session.py").as_slice()),
            ("ownership.py", include_bytes!("../runtime/ownership.py").as_slice())] {
            let path = f.inputs.path(name).unwrap();
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path,fs::Permissions::from_mode(0o444)).unwrap();
        }
        f.write_manifest();
        f.adopt().unwrap();
        let source = f.current();
        let source_selection = fs::read(f.base.m.root.join("software.json")).unwrap();
        assert_eq!(verify_generation(&f.base.m,&source).unwrap().schema,3);
        assert_eq!(source.supervisor.path.file_name().unwrap(),"session.py");
        assert_eq!(source.ownership.path.file_name().unwrap(),"ownership.py");
        assert_eq!(package_census_tools(&f.inputs,f.owner).unwrap(),
            (Artifact {path:f.inputs.path("session.py").unwrap(),sha256:source.supervisor.sha256.clone()},
             Artifact {path:f.inputs.path("ownership.py").unwrap(),sha256:source.ownership.sha256.clone()}));
        rollback_from(&f.base.m,&f.home,&f.service).unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(),old_selection);
        assert_eq!(fs::read(&old_record).unwrap(),old_bytes);
        f.adopt().unwrap();
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(),source_selection);
        assert_eq!(fs::read(&old_record).unwrap(),old_bytes);
        let source_record=source.manager.path.parent().unwrap().join("package-generation.json");
        let saved=source_record.with_extension("saved");
        fs::rename(&source_record,&saved).unwrap();
        assert!(selected_generation(&f.base.m,&source).is_err());
        assert!(selected_generation_record(&f.base.m,&source).is_err());
        fs::rename(&saved,&source_record).unwrap();
        fs::set_permissions(&source.ownership.path,fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&source.ownership.path,b"changed source").unwrap();
        assert!(verify_generation(&f.base.m,&source).is_err());
        assert!(linux_vst_bridge::supervisor::arguments(&source.supervisor,&source.ownership).is_err());
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
        f.add_kit(b"valid replacement kit");
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
            builder: None, generator: None, direct_audio_helpers:vec![] };
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
        f.add_kit(b"successor preparation kit");
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
            after_publications:publications, transitions:Vec::new(), resume_history:None };
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

    #[test]
    fn genuine_package_renewal_resolves_a_retained_predecessor_without_adopting_replacement() {
        let (f,publication)=real_publication_transition();
        let before=publication.before.entry.managed_revision.as_ref().unwrap();
        let prior=f.base.m.load_revision(&publication.before.class_id,before).unwrap();
        let checkpoint=f.base.m.root.join("retained-prelaunch-baseline.json");
        atomic_json(&checkpoint,&serde_json::json!({"prior_modules":[prior.registration.module.sha256],
            "prior_selections":[format!("{}:{}",prior.registration.module.sha256,prior.class_id)],"predecessors":[prior]})).unwrap();
        let baseline=fs::read(&checkpoint).unwrap();
        let (first,next,_,setup)=staged_transition(&f);
        let receipt=RefreshReceipt {schema:1,predecessor:first.clone(),successor:next.clone(),
            before_publications:vec![publication.before.clone()],after_publications:vec![publication.after.clone()],
            transitions:vec![publication.clone()],resume_history:None};
        let update=CoordinatedTransition {schema:2,id:random_id().unwrap(),kind:TransitionKind::Update,
            recovery_before:first,setup,receipt,resume_handoff:None};
        run_coordinated(&f.base.m,&f.home,&f.service,update,false).unwrap();
        let renewed=renewed_predecessor(&f.base.m,&prior).unwrap();
        assert_eq!(renewed.id,publication.after.entry.managed_revision.as_ref().unwrap().id);
        assert_eq!(renewed.registration.module,prior.registration.module);
        assert_eq!(fs::read(&checkpoint).unwrap(),baseline);
        // A later arbitrary publication is not a package renewal of this baseline.
        let mut profile=renewed.profile.clone();profile.revision+=1;
        f.base.m.managed_publish(&profile,&renewed.census,renewed.registration.clone(),
            &renewed.census.host,&renewed.census.host_source_sha256,None).unwrap();
        assert!(renewed_predecessor(&f.base.m,&prior).is_err());
        assert_eq!(fs::read(&checkpoint).unwrap(),baseline);
    }

    fn refreshing_update_fixture() -> Fixture {
        let (base, candidate) = test_fixture::prepared_candidate(&"13".repeat(16));
        let f = Fixture::with_base(base);
        let mut report: serde_json::Value = read_json(&candidate.inspection.report.path).unwrap();
        report["records"].as_array_mut().unwrap().push(serde_json::json!({"state":"ap8_bus",
            "media":0,"direction":1,"index":0,"channels":2,"type":0,"flags":1,
            "arrangement":3,"name":"Output"}));
        let report_path = f.base.outer.join("baseline-inspection.json");
        atomic_json(&report_path, &report).unwrap();
        let kit = f.base.m.root.join("software/baseline-kit.zip");
        test_fixture::reusable_kit(&kit, &candidate.host, &candidate.source_manifest, "retained-engine");
        let temporary = Software { manager:candidate.host.clone(),operator_frontend:None,
            installer_launch:None,preparation_kit:Some(Artifact {sha256:digest(&kit).unwrap(),path:kit}),
            supervisor:candidate.host.clone(),ownership:candidate.host.clone(),host:candidate.host.clone(),
            source_manifest:candidate.source_manifest.clone(),source_sha256:candidate.source_manifest.sha256.clone(),
            native_catalogue:None };
        let runtime = preparation::build::stage_runtime_for_software(&f.base.m, &temporary).unwrap();
        let inspection = preparation::inspect_record_with(candidate.selection.clone(),
            Artifact {sha256:digest(&report_path).unwrap(),path:report_path},
            preparation::Origin::ManagedPreparation,runtime.host.clone(),runtime.source_manifest.clone()).unwrap();
        let operation = random_id().unwrap();
        let retained = preparation::build::construct_with_runtime(&f.base.m, candidate.selection,
            inspection, runtime, &operation, None).unwrap();
        preparation::retain_inspection(&f.base.m, &retained.inspection).unwrap();
        preparation::record_candidate(&f.base.m, &retained).unwrap();
        preparation::build::cleanup_work(&f.base.m, &operation).unwrap();
        preparation::enable(&f.base.m, &retained, false).unwrap();
        f.adopt().unwrap();
        let kit = f.base.outer.join("update-kit.zip");
        test_fixture::reusable_kit(&kit, &retained.host, &retained.source_manifest, "updated-engine");
        f.add_kit_archive(&fs::read(kit).unwrap());
        let template = serde_json::to_string(&serde_json::to_string(&report).unwrap()).unwrap();
        let supervisor = f.base.outer.join("update-supervisor.py");
        fs::write(&supervisor, format!(r#"import json,sys,pathlib
j=json.loads(pathlib.Path(sys.argv[1]).read_text())
r=json.loads({template})
p=pathlib.Path(j['report']);p.write_text(json.dumps(r))
p.with_suffix('.ownership.json').write_text(json.dumps(dict(session=j['session'],cleanup_confirmed=True,transport_retired=True)))
print('LVO0 '+j['session']+' ready',flush=True)
"#)).unwrap();
        let compiled = f.base.outer.join("update-supervisor.pyc");
        assert!(Command::new("python3").args(["-I", "-c",
            "import py_compile,sys;py_compile.compile(sys.argv[1],cfile=sys.argv[2],doraise=True)"])
            .arg(supervisor).arg(&compiled).status().unwrap().success());
        f.replace("session.pyc", &fs::read(compiled).unwrap(), true);
        f.replace("linux-vst-bridge", b"updated manager", true);
        f.service.enable_start().unwrap();
        f
    }

    #[test]
    fn adding_back_after_an_update_rechecks_the_same_plugin_with_the_installed_bridge() {
        let f = refreshing_update_fixture();
        let m = &f.base.m;
        let key = f.base.r.key();
        let removed = m.registry().unwrap().classes[&key].managed_revision.clone().unwrap();
        // The musician took it out of the DAW before the bridge update.
        m.unpublish_exact(&key, &removed).unwrap();
        update_from(m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
        assert!(add_back_needs_refresh(m, &key, &removed).unwrap(), "prepared with the old bridge");
        // The real supervised inspection, preparation and publication.
        let fresh = add_back_publication(m, &key, &removed, &random_id().unwrap()).unwrap();
        let entry = m.registry().unwrap().classes[&key].clone();
        assert_eq!(entry.publication, Publication::Published);
        assert_eq!(entry.managed_revision.as_ref(), Some(&fresh));
        assert_ne!(fresh, removed);
        let prior = m.load_revision(&key, &removed).unwrap();
        let current = m.load_revision(&key, &fresh).unwrap();
        assert_eq!(current.registration.module, prior.registration.module, "the same Windows module");
        assert_eq!(current.registration.metadata.class_id, prior.registration.metadata.class_id);
        assert_eq!(current.registration.environment.id, prior.registration.environment.id);
        assert!(!add_back_needs_refresh(m, &key, &fresh).unwrap(), "built with the installed bridge");
        assert_eq!(linux_vst_bridge::publication::physical(&m.link(&key)).unwrap(), Some(current.target));
        // The removed publication stays in history; a replay refuses without changing anything.
        let registry = fs::read(m.root.join("registry.json")).unwrap();
        assert!(add_back_publication(m, &key, &removed, &random_id().unwrap()).is_err());
        assert_eq!(fs::read(m.root.join("registry.json")).unwrap(), registry);
    }
    /// Steam updated the Proton build that a published plug-in's setup sits on
    /// (the Deck, 2026-10-18). The bridge update keeps that publication as it
    /// is and completes; the setup's own launch admission reports the runtime.
    #[test]
    fn software_update_continues_past_a_setup_whose_runtime_changed_outside_the_product() {
        use std::os::unix::fs::PermissionsExt;
        let f = refreshing_update_fixture();
        let m = &f.base.m;
        let key = f.base.r.key();
        let before = m.registry().unwrap().classes[&key].clone();
        let previous = f.current();
        let pinned = f.base.r.environment.runner.files[0].path.clone();
        let mode = fs::metadata(&pinned).unwrap().permissions();
        fs::set_permissions(&pinned, fs::Permissions::from_mode(0o600)).unwrap();
        let mut changed = fs::read(&pinned).unwrap();
        changed.extend_from_slice(b"\n# updated by Steam\n");
        fs::write(&pinned, &changed).unwrap();
        fs::set_permissions(&pinned, mode).unwrap();
        assert!(f.base.r.environment.runner.verify_pinned_files().is_err());
        update_from(m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
        assert_ne!(f.current().manager.sha256, previous.manager.sha256, "the bridge was updated");
        let after = m.registry().unwrap().classes[&key].clone();
        assert_eq!(after.publication, Publication::Published);
        assert_eq!(after.managed_revision, before.managed_revision,
            "the publication on the changed runtime was kept, not refreshed");
        // Re-adding that plug-in would run it: the setup's own operation
        // reports the changed runtime rather than the update.
        assert!(add_back_needs_refresh(m, &key, before.managed_revision.as_ref().unwrap())
            .unwrap_err().to_string().contains("artifact missing or changed"));
    }
    #[test]
    fn software_update_preserves_owned_stale_publications_until_fresh_preparation() {
        for broken_edge in [false, true] {
            let f = refreshing_update_fixture();
            let m = &f.base.m;
            let original_entry = m.registry().unwrap().classes[&f.base.r.key()].clone();
            let original_ref = original_entry.managed_revision.as_ref().unwrap();
            let original = m.load_revision(&f.base.r.key(), original_ref).unwrap();
            let current = preparation::publication_candidate(m, &original.profile,
                &original.registration).unwrap();
            let sibling_id = "03".repeat(16);
            let mut factory:serde_json::Value = read_json(&current.selection.factory_report.path).unwrap();
            let classes = factory["records"][1]["classes"].as_array_mut().unwrap();
            let mut sibling = classes[0].clone();
            sibling["raw_tuid_hex"] = serde_json::json!(sibling_id);
            classes.push(sibling);
            let count = classes.len();
            factory["records"][1]["class_count"] = serde_json::json!(count);
            let factory_path = f.base.outer.join("two-class-factory.json");
            atomic_json(&factory_path, &factory).unwrap();
            let factory = Artifact {sha256:digest(&factory_path).unwrap(),path:factory_path};
            let mut scan:inventory::Scan = read_json(&m.root.join("inventory")
                .join(format!("{}.json",current.selection.environment.id))).unwrap();
            scan.modules[0].classes = inventory::classes(&read_json(&factory.path).unwrap()).unwrap();
            scan.modules[0].report = factory.clone();
            atomic_json(&m.root.join("inventory").join(format!("{}.json",scan.environment.id)),&scan).unwrap();
            let sibling_class = scan.modules[0].classes.iter().find(|c| c.id==sibling_id).unwrap().clone();
            let mut selection = current.selection.clone();
            selection.class = sibling_class;
            selection.factory_report = factory.clone();
            let sibling_report = f.base.outer.join("sibling-inspection.json");
            let report = fs::read_to_string(&current.inspection.report.path).unwrap()
                .replace(&current.selection.class.id,&sibling_id);
            fs::write(&sibling_report, report).unwrap();
            let sibling_report = Artifact {sha256:digest(&sibling_report).unwrap(),path:sibling_report};
            let inspection = preparation::inspect_record_with(selection.clone(),sibling_report,
                preparation::Origin::ManagedPreparation,current.host.clone(),current.source_manifest.clone()).unwrap();
            let operation = random_id().unwrap();
            let sibling = preparation::build::construct_with_runtime(m,selection,inspection,
                preparation::build::existing_runtime(m,&current.recipe_sha256).unwrap(),&operation, None).unwrap();
            preparation::record_candidate(m,&sibling).unwrap();
            preparation::enable(m,&sibling,false).unwrap();
            preparation::build::cleanup_work(m,&operation).unwrap();
            let stale = m.registry().unwrap().classes[&sibling_id].clone();
            let before = current.selection.environment.clone();
            let mut after = before.clone();after.revision+=1;
            after.runner.id="different-fixture-runtime".into();
            let operation = random_id().unwrap();
            let directory=m.root.join("operator").join(&operation);private_dir(&directory).unwrap();
            let request=directory.join("request.json");
            atomic_json(&request,&operator_model::Request {schema:operator_model::OPERATOR_SCHEMA,
                state_token:"fixture".into(),action:operator_model::Action::EnvironmentRuntimeSelect {
                    environment:before.id.clone(),runner:catalogue::runner_key(&after.runner).unwrap(),
                    expected_environment:environment_revision::identity(&before).unwrap()}}).unwrap();
            atomic_json(&directory.join("runtime-transition.json"),&environment_revision::RuntimeBinding {
                schema:1,operation:operation.clone(),before:before.clone(),environment:after.clone(),
                affected:vec![environment_revision::RuntimeOwner::Class {registration:Box::new(original.registration.clone())}],
                state:environment_revision::RuntimeState::Scanned}).unwrap();
            atomic_json(&directory.join("result.json"), &serde_json::json!({
                "schema":1,"operation":operation,"state":"completed","result":{}})).unwrap();
            atomic_json(&after.root.join("environment.json"),&after).unwrap();
            scan.environment=after.clone();
            atomic_json(&m.root.join("inventory").join(format!("{}.json",after.id)),&scan).unwrap();
            let mut selection=current.selection.clone();selection.environment=after.clone();
            selection.factory_report=factory;
            let inspection=preparation::inspect_record_with(selection.clone(),current.inspection.report.clone(),
                preparation::Origin::ManagedPreparation,current.host.clone(),current.source_manifest.clone()).unwrap();
            let fresh=preparation::prepared(selection,inspection,current.native.clone(),current.host.clone(),
                current.source_manifest.clone(),current.recipe_sha256.clone()).unwrap();
            preparation::record_candidate_with_predecessor(m,&fresh,Some(&current.id().unwrap())).unwrap();
            preparation::replace(m,&fresh,original_ref).unwrap();
            let selected=f.current();let registry=fs::read(m.root.join("registry.json")).unwrap();
            if broken_edge {fs::remove_file(&request).unwrap();}
            let result=update_from(m,&f.home,&f.inputs,f.owner,&f.service);
            if broken_edge {
                assert!(result.is_err(),"missing ownership was treated as harmless stale state");
                assert!(f.current()==selected);
                assert_eq!(fs::read(m.root.join("registry.json")).unwrap(),registry);
            } else {
                result.unwrap();assert!(f.current()!=selected);
                let registry=m.registry().unwrap();
                assert_eq!(registry.classes[&sibling_id],stale);
                assert_eq!(registry.classes[&f.base.r.key()].registration.environment,after);
                assert_ne!(registry.classes[&f.base.r.key()].registration.native.sha256,
                    fresh.native.artifact.sha256);
                assert_eq!(preparation::publication_state_record(m,&sibling).unwrap(),"environment_changed");
                let identity = (sibling_id.clone() + &sibling.selection.module.sha256).as_bytes()
                    .chunks(2).map(|pair|u8::from_str_radix(std::str::from_utf8(pair).unwrap(),16).unwrap())
                    .collect::<Vec<_>>();
                assert!(m.resolve(&identity).is_err());
                assert!(!preparation::registry_requires_loaded_engine_refresh_record(m).unwrap());
                assert!(!preparation::registry_requires_loaded_engine_refresh(m).unwrap());
                assert_eq!(verify_generation(m,&f.current()).unwrap().predecessor.unwrap().manager,selected.manager);
            }
            assert!(!m.root.join("package-transition.json").exists());
        }
    }

    #[test]
    fn bootstrap_status_waits_for_brief_readback_contention_without_switching_setup() {
        let f = Fixture::new();
        f.adopt().unwrap();
        f.service.enable_start().unwrap();
        f.replace("linux-vst-bridge", b"updated manager", true);
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        let publications = f.base.m.package_publication_snapshot().unwrap();
        let stops = f.service.stops.get();
        let held = f.base.m.lock("registry.lock").unwrap();
        let result = std::thread::scope(|scope| {
            scope.spawn(move || {
                std::thread::sleep(Duration::from_millis(80));
                drop(held);
            });
            bootstrap_status_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service)
        });
        assert_eq!(result.unwrap().state, "update_active");
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(), publications);
        assert_eq!(f.service.stops.get(), stops);
        assert!(!f.base.m.root.join("package-transition.json").exists());
    }

    #[test]
    fn bootstrap_status_wait_is_bounded_and_rechecks_the_selected_service() {
        for timeout in [true, false] {
            let f = Fixture::new();
            f.adopt().unwrap();
            f.service.enable_start().unwrap();
            f.replace("linux-vst-bridge", b"updated manager", true);
            let before = fs::read(f.base.m.root.join("software.json")).unwrap();
            let publications = f.base.m.package_publication_snapshot().unwrap();
            let stops = f.service.stops.get();
            let mut waited = false;
            let mut acquire = registry_wait_budget(&f.base.m, Duration::from_millis(20));
            let result = bootstrap_status_with_registry(&f.base.m, &f.home, &f.inputs,
                f.owner, &f.service, &mut || {
                    if waited { return acquire(); }
                    waited = true;
                    let held = f.base.m.lock("registry.lock")?;
                    if timeout {
                        let result = acquire();
                        drop(held);
                        result
                    } else {
                        f.service.loaded.borrow_mut().exec = "foreign service".into();
                        drop(held);
                        acquire()
                    }
                });
            let error = result.err().expect("changed or contended status must refuse");
            if timeout {
                let facts = &error.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap().facts;
                assert!(facts.attempts > 1 && facts.outcome == operator_model::LockOutcome::Timeout);
            } else {
                assert_eq!(error.to_string(), "package_active_service_identity_changed");
            }
            assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(), before);
            assert_eq!(f.base.m.package_publication_snapshot().unwrap(), publications);
            assert_eq!(f.service.stops.get(), stops);
            assert!(!f.base.m.root.join("package-transition.json").exists());
        }
    }

    #[test]
    fn restorer_waits_for_a_brief_registry_readback_without_changing_custody() {
        let f = refreshing_update_fixture();
        let predecessor = f.current();
        let publications = f.base.m.package_publication_snapshot().unwrap();
        let preferences = f.base.m.performance(&f.base.r.key()).unwrap();
        let project = f.base.outer.join("saved-project");
        fs::write(&project, b"saved musician project").unwrap();
        update_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
        let held = f.base.m.lock("registry.lock").unwrap();
        let result = std::thread::scope(|scope| {
            scope.spawn(move || {
                std::thread::sleep(Duration::from_millis(80));
                drop(held);
            });
            restore_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service)
        });
        result.unwrap();
        assert!(f.current() == predecessor);
        assert_eq!(f.base.m.package_publication_snapshot().unwrap(), publications);
        assert_eq!(f.base.m.performance(&f.base.r.key()).unwrap(), preferences);
        assert_eq!(fs::read(project).unwrap(), b"saved musician project");
        assert_eq!(f.service.show().unwrap().active, "active");
        assert!(!f.base.m.root.join("package-transition.json").exists());
    }

    #[test]
    fn restorer_bounds_contention_and_recovers_a_partially_restored_publication() {
        use operator_model::LockOutcome;
        // Refuse before stopping, refuse after a publication has changed, or finish.
        for fail_phase in [Some(false), Some(true), None] {
            let f = refreshing_update_fixture();
            let predecessor = f.current();
            let original_publications = f.base.m.package_publication_snapshot().unwrap();
            update_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
            let before = f.current();
            let publications = f.base.m.package_publication_snapshot().unwrap();
            let registry = fs::read(f.base.m.root.join("registry.json")).unwrap();
            let preferences = f.base.m.performance(&f.base.r.key()).unwrap();
            let project = f.base.outer.join("saved-project");
            fs::write(&project, b"saved musician project").unwrap();
            let stops = f.service.stops.get();
            let mut calls = 0;
            let mut recovery_calls = 0;
            let mut failed = false;
            let mut forward = registry_wait_budget(&f.base.m, Duration::from_millis(20));
            let mut recovery = registry_wait_budget(&f.base.m, Duration::from_millis(20));
            let result = restore_from_with_registry(&f.base.m, &f.home, &f.inputs, f.owner,
                &f.service, || {
                    calls += 1;
                    let held = f.base.m.lock("registry.lock")?;
                    assert!(matches!(f.base.m.try_lock("registry.lock")?, operator_lock::LockAttempt::Busy));
                    let changed = f.base.m.root.join("package-transition.json").exists()
                        && fs::read(f.base.m.root.join("registry.json"))? != registry;
                    let refuse = !failed && match fail_phase {
                        Some(false) => calls == 1,
                        Some(true) => changed,
                        None => false,
                    };
                    let held = if refuse { failed = true; Some(held) }
                        else { drop(held); None };
                    let result = forward();
                    drop(held);
                    result
                }, || {
                    recovery_calls += 1;
                    let held = f.base.m.lock("registry.lock")?;
                    assert!(matches!(f.base.m.try_lock("registry.lock")?, operator_lock::LockAttempt::Busy));
                    drop(held);
                    recovery()
                });
            if fail_phase.is_some() {
                assert!(failed);
                let error = result.unwrap_err();
                let facts = &error.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap().facts;
                assert!(facts.attempts > 1 && facts.outcome == LockOutcome::Timeout);
                assert!(f.current() == before);
                assert_eq!(f.base.m.package_publication_snapshot().unwrap(), publications);
                if fail_phase == Some(false) {
                    assert_eq!(f.service.stops.get(), stops);
                    assert_eq!(recovery_calls, 0);
                } else {
                    assert!(recovery_calls > 0, "partial Restore did not use its recovery allowance");
                }
            } else {
                result.unwrap();
                assert!(f.current() == predecessor);
                assert_eq!(f.base.m.package_publication_snapshot().unwrap(), original_publications);
                assert_eq!(recovery_calls, 0);
            }
            assert_eq!(f.base.m.performance(&f.base.r.key()).unwrap(), preferences);
            assert_eq!(fs::read(project).unwrap(), b"saved musician project");
            assert_eq!(f.service.show().unwrap().active, "active");
            assert!(!f.base.m.root.join("package-transition.json").exists());
            assert_eq!(operator_cli::pending_transactions(&f.base.m).unwrap(), 0);
        }
    }

    #[test]
    fn restorer_wait_rechecks_service_identity_and_restore_snapshot() {
        for change_at in [7, 11] {
            let f = refreshing_update_fixture();
            update_from(&f.base.m, &f.home, &f.inputs, f.owner, &f.service).unwrap();
            let before = f.current();
            let stops = f.service.stops.get();
            let mut calls = 0;
            let result = restore_from_with_registry(&f.base.m, &f.home, &f.inputs, f.owner,
                &f.service, || {
                    calls += 1;
                    let held = f.base.m.lock("registry.lock")?;
                    if calls == 7 && change_at == 7 {
                        f.service.loaded.borrow_mut().exec = "foreign service".into();
                    }
                    drop(held);
                    if calls == 11 && change_at == 11 {
                        f.base.m.select_delay(&f.base.r.key(), 1024)?;
                    }
                    bounded_registry(&f.base.m)()
                }, bounded_registry(&f.base.m));
            let error = result.unwrap_err().to_string();
            assert_eq!(calls, change_at);
            assert!(error.contains(if change_at == 7 { "package_active_service_identity_changed" }
                else { "package_restore_receipt_unavailable" }), "{error}");
            assert!(f.current() == before);
            assert!(!f.base.m.root.join("package-transition.json").exists());
            if change_at == 7 { assert_eq!(f.service.stops.get(), stops); }
            else { assert_eq!(f.base.m.performance(&f.base.r.key()).unwrap().added_frames, 1024); }
        }
        for name in ["package.lock", "setup.lock"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let before = f.current();
            let _held = f.base.m.lock(name).unwrap();
            let error = restore_from_with_registry(&f.base.m, &f.home, &f.inputs, f.owner,
                &f.service, || panic!("a concurrent mutation cannot wait on registry"),
                || panic!("a concurrent mutation cannot enter recovery")).unwrap_err();
            assert!(error.downcast_ref::<operator_lock::LockBusy>().is_some());
            assert!(f.current() == before);
        }
    }

    #[test]
    fn updater_registry_admission_covers_stop_refresh_commit_and_contended_recovery() {
        readiness::with_unit_state_fixture(&[vendor_cli::unit("arturia-software-center").unwrap()],"inactive",|| {
        use operator_model::LockOutcome;
        // Cleanup refusal (13), and refusal during resumed target settlement (19).
        // The success case also settles the retained suspension (19).
        for fail_at in [Some(13), Some(19), None] {
            let f = refreshing_update_fixture();
            let before = f.current();
            let publications = f.base.m.package_publication_snapshot().unwrap();
            let preferences = f.base.m.performance(&f.base.r.key()).unwrap();
            let (reservation, bytes, failed, failed_bytes) = retired_suspension(&f, &before);
            let project = f.base.outer.join("saved-project");
            fs::write(&project, b"saved musician project").unwrap();
            let mut calls = 0;
            let mut waits = vec![];
            let mut forward = registry_wait_budget(&f.base.m, Duration::from_millis(20));
            let mut recovery_calls = 0;
            let mut recovery = registry_wait_budget(&f.base.m, Duration::from_millis(20));
            let mut initial_contention = Some(f.base.m.lock("registry.lock").unwrap());
            let result = update_from_with_registry(&f.base.m, &f.home, &f.inputs, f.owner,
                &f.service, || {
                    calls += 1;
                    let held = match initial_contention.take() {
                        Some(held) => held,
                        None => f.base.m.lock("registry.lock")?,
                    };
                    assert!(matches!(f.base.m.try_lock("registry.lock")?, operator_lock::LockAttempt::Busy));
                    // Every reachable crossing sees a real busy registry then
                    // release. The chosen failure keeps that exact guard held.
                    let held = if fail_at == Some(calls) { Some(held) } else { drop(held); None };
                    let acquired = forward();
                    drop(held);
                    match acquired {
                        Ok(guard) => Ok(guard),
                        Err(error) => {
                            waits.push(error.downcast_ref::<operator_lock::AcquisitionFailure>()
                                .unwrap().facts.clone());
                            Err(error)
                        }
                    }
                }, || {
                    recovery_calls += 1;
                    let held = f.base.m.lock("registry.lock")?;
                    assert!(matches!(f.base.m.try_lock("registry.lock")?, operator_lock::LockAttempt::Busy));
                    drop(held);
                    recovery()
                });
            assert_eq!(calls, fail_at.unwrap_or(19), "ordinary updater admission bypassed: {result:?}");
            assert_eq!(recovery_calls, if fail_at == Some(19) { 5 } else { 0 });
            if fail_at.is_some() {
                let error = result.unwrap_err();
                let facts = &error.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap().facts;
                assert!(facts.attempts > 1 && facts.outcome == LockOutcome::Timeout);
                assert!(f.current() == before);
                assert_eq!(f.base.m.package_publication_snapshot().unwrap(), publications);
                assert_eq!(fs::read(&reservation).unwrap(), bytes);
            } else {
                result.unwrap();
                assert!(f.current() != before);
                let current = f.base.m.package_publication_snapshot().unwrap();
                assert_eq!(current[0].entry.registration.environment,
                    publications[0].entry.registration.environment);
                assert_eq!(current[0].entry.registration.module,
                    publications[0].entry.registration.module);
                assert!(!reservation.exists());
            }
            assert_eq!(fs::read(failed).unwrap(), failed_bytes);
            assert_eq!(waits.len(), usize::from(fail_at.is_some()));
            assert_eq!(f.base.m.performance(&f.base.r.key()).unwrap(), preferences);
            assert_eq!(fs::read(project).unwrap(), b"saved musician project");
            assert_eq!(f.service.show().unwrap().active, "active");
            assert!(!f.base.m.root.join("package-transition.json").exists());
            assert!(!f.base.m.publication_pending(&f.base.r.key()).unwrap());
        }
        });
    }

    #[test]
    fn updater_wait_rechecks_stop_identity_and_final_refresh_snapshot() {
        for change_at in [7, 15] {
            let f = refreshing_update_fixture();
            let before = f.current();
            let mut calls = 0;
            let result = update_from_with_registry(&f.base.m, &f.home, &f.inputs, f.owner,
                &f.service, || {
                    calls += 1;
                    let held = f.base.m.lock("registry.lock")?;
                    assert!(matches!(f.base.m.try_lock("registry.lock")?, operator_lock::LockAttempt::Busy));
                    if calls == 7 && change_at == 7 {
                        f.service.loaded.borrow_mut().exec = "foreign service".into();
                    }
                    drop(held);
                    if calls == 15 && change_at == 15 {
                        f.base.m.select_delay(&f.base.r.key(), 1024)?;
                    }
                    bounded_registry(&f.base.m)()
                }, bounded_registry(&f.base.m));
            let error = result.unwrap_err().to_string();
            assert_eq!(calls, change_at);
            assert!(error.contains(if change_at == 7 { "package_active_service_identity_changed" }
                else { "package_publication_set_changed" }), "{error}");
            assert!(f.current() == before);
            assert!(!f.base.m.root.join("package-transition.json").exists());
            assert!(!f.base.m.publication_pending(&f.base.r.key()).unwrap());
            if change_at == 7 { assert_eq!(f.service.stops.get(), 0); }
            else {
                assert_eq!(f.base.m.performance(&f.base.r.key()).unwrap().added_frames, 1024);
                assert_eq!(f.service.show().unwrap().active, "active");
            }
        }
    }

    #[test]
    fn updater_wait_budget_is_shared_and_recovery_gets_a_finite_fresh_allowance() {
        let f = Fixture::new();
        let held = f.base.m.lock("registry.lock").unwrap();
        let mut forward = registry_wait_budget(&f.base.m, Duration::from_millis(20));
        let exhausted = forward().err().unwrap();
        let facts = &exhausted.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap().facts;
        assert!(facts.attempts > 1 && facts.timeout_ms == 20);
        let refused = forward().err().unwrap();
        let facts = &refused.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap().facts;
        assert_eq!((facts.attempts, facts.timeout_ms), (1, 0));
        let mut recovery = registry_wait_budget(&f.base.m, Duration::from_millis(20));
        let refused = recovery().err().unwrap();
        let facts = &refused.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap().facts;
        assert!(facts.attempts > 1 && facts.timeout_ms == 20);
        drop(held);
        assert!(forward().is_ok());
        assert!(recovery().is_ok());
    }

    #[test]
    fn coordinated_service_startup_waits_for_both_guards_and_health_endpoint() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let (first, _, receipt, setup) = staged_transition(&f);
        let transition = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup, receipt, resume_handoff:None };
        let journal = f.base.m.root.join("package-transition.json");
        atomic_json(&journal, &transition).unwrap();
        let mut calls = 0;
        let mut initial_contention = Some(f.base.m.lock("registry.lock").unwrap());
        reconcile_service_startup_at(&f.base.m, &first, &f.home, || {
            calls += 1;
            let held = match initial_contention.take() {
                Some(held) => held,
                None => f.base.m.lock("registry.lock")?,
            };
            assert!(matches!(f.base.m.try_lock("registry.lock")?, operator_lock::LockAttempt::Busy));
            drop(held);
            bounded_registry(&f.base.m)()
        }).unwrap();
        assert_eq!(calls, 2, "coherence and reconciliation must both use the startup allowance");
        // Twenty-one refused probes cross the former two-second window.
        f.service.fail_health_reads.set(21);
        start_and_health(&f.service, &f.base.m, &f.home, &first).unwrap();
        assert_eq!(f.service.fail_health_reads.get(), 0);
        fs::remove_file(journal).unwrap();
        let held = f.base.m.lock("registry.lock").unwrap();
        let error = reconcile_service_startup_at(&f.base.m, &first, &f.home,
            || panic!("ordinary startup must retain fail-fast admission")).unwrap_err();
        assert!(error.downcast_ref::<operator_lock::LockBusy>().is_some());
        drop(held);
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

    fn retired_suspension(f: &Fixture, source: &Software) -> (PathBuf,Vec<u8>,PathBuf,Vec<u8>) {
        let owner = "ab".repeat(16);
        let vendor = "cd".repeat(16);
        let directory = f.base.m.root.join("operator").join(&owner);
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("request.json"),&serde_json::json!({
            "schema":operator_model::OPERATOR_SCHEMA,"state_token":"00".repeat(32),
            "action":{"kind":"vendor_application_open","application":"arturia-software-center"}
        })).unwrap();
        let failed = serde_json::json!({"schema":1,"operation":owner,"state":"refused",
            "error":"unknown field close_result"});
        atomic_json(&directory.join("result.json"),&failed).unwrap();
        let failed_bytes = fs::read(directory.join("result.json")).unwrap();
        let reservation = f.base.m.root.join("operator/resume.json");
        atomic_json(&reservation,&serde_json::json!({"schema":2,"owner_operation":owner,
            "vendor_operation":vendor,"resume":true,"software":source.manager.sha256})).unwrap();
        let original:serde_json::Value = read_json(&reservation).unwrap();
        fs::write(&reservation,format!("{}\n ",serde_json::to_string_pretty(&original).unwrap())).unwrap();
        let application = vendor_application::Application {schema:1,
            id:vendor_application::ApplicationId::ArturiaSoftwareCenter,
            environment:f.base.r.environment.clone(),executable:f.base.r.module.clone(),
            helpers:vec![],installer_sha256:Some("00".repeat(32)),installer_result:Some(f.base.r.host.clone()),
            observed_installer_version:"fixture".into(),user_selection:None};
        let vendor_dir = f.base.m.root.join("vendor-applications/arturia-software-center");
        private_dir(&vendor_dir).unwrap();
        atomic_json(&vendor_dir.join("application.json"),&application).unwrap();
        atomic_json(&vendor_dir.join("operation.json"),&vendor_application::Operation {
            application,report:vendor_dir.join("operation-result.json"),
            mode:vendor_application::LaunchMode::Normal,operation_id:vendor.clone(),launcher:None, managed:None,capability_notice:None}).unwrap();
        atomic_json(&vendor_dir.join("operation-result.json"),&serde_json::json!({
            "schema":3,"operation_id":vendor,"state":"completed","launcher_exit":0,
            "owned_live":0,"cleanup_confirmed":true,"close_result":null,
            "focus_result":null,"discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        let bytes = fs::read(&reservation).unwrap();
        (reservation,bytes,directory.join("result.json"),failed_bytes)
    }

    #[test]
    fn package_update_settles_retired_suspension_after_clean_target_readback() {
        readiness::with_unit_state_fixture(&[vendor_cli::unit("arturia-software-center").unwrap()],"inactive",|| {
        let f = Fixture::new();
        f.adopt().unwrap();
        let (first, next, receipt, setup) = staged_transition(&f);
        let (reservation,original,failed,failed_bytes) = retired_suspension(&f,&first);
        let transition = CoordinatedTransition {schema:2,id:random_id().unwrap(),
            kind:TransitionKind::Update,recovery_before:first.clone(),setup,receipt, resume_handoff:None};
        run_coordinated(&f.base.m,&f.home,&f.service,transition,false).unwrap();
        assert!(f.current() == next);
        assert!(!reservation.exists(),"completed update left the old-software suspension pending");
        assert_eq!(fs::read(&failed).unwrap(),failed_bytes);
        let receipts = matching_refresh_receipts(&f.base.m,&next,
            &f.base.m.package_publication_snapshot().unwrap()).unwrap();
        let history = receipts[0].resume_history.as_ref().unwrap();
        assert!(history.source == first);
        assert_eq!(serde_json::to_value(&history.captured).unwrap()["record"],
            String::from_utf8(original).unwrap());
        // Restoring the retained package does not reactivate this historical obligation.
        f.service.stop().unwrap();
        restore_from(&f.base.m,&f.home,&f.inputs,f.owner,&f.service).unwrap();
        assert!(f.current() == first);
        assert!(!reservation.exists());
        assert_eq!(fs::read(&failed).unwrap(),failed_bytes);
        });
    }
    #[test]
    fn package_resume_snapshot_verification_requires_exact_registry_guard() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let expected = f.base.m.package_publication_snapshot().unwrap();
        let wrong_lock = f.base.m.lock("package.lock").unwrap();
        assert_eq!(f.base.m.package_publication_snapshot_locked(&wrong_lock)
            .unwrap_err().to_string(), "registry_guard_identity");
        assert_eq!(f.base.m.package_publication_record_snapshot_locked(&wrong_lock)
            .unwrap_err().to_string(), "registry_guard_identity");
        assert_eq!(f.base.m.verify_package_publication_snapshot_locked(&expected, &wrong_lock)
            .unwrap_err().to_string(), "registry_guard_identity");
        let other = Fixture::new();
        let foreign_lock = other.base.m.lock("registry.lock").unwrap();
        assert_eq!(f.base.m.verify_package_publication_snapshot_locked(&expected, &foreign_lock)
            .unwrap_err().to_string(), "registry_guard_identity");
        let registry = f.base.m.lock("registry.lock").unwrap();
        f.base.m.verify_package_publication_snapshot_locked(&expected, &registry).unwrap();
    }
    #[test]
    fn package_resume_crash_boundaries_preserve_or_settle_exact_owner() {
        readiness::with_unit_state_fixture(&[vendor_cli::unit("arturia-software-center").unwrap()],"inactive",|| {
        for boundary in ["before_switch","after_switch","settling_present","consumed"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let (first,next,receipt,setup) = staged_transition(&f);
            let (reservation,original,failed,failed_bytes) = retired_suspension(&f,&first);
            let captured = operator_cli::capture_package_resume_with(&f.base.m,&first,
                |_|Ok(false)).unwrap().unwrap();
            let transition = CoordinatedTransition {schema:3,id:random_id().unwrap(),
                kind:TransitionKind::Update,recovery_before:first.clone(),setup,receipt,
                resume_handoff:Some(ResumeHandoff {captured:captured.clone(),origin_active:false,
                    settling:matches!(boundary,"settling_present" | "consumed")})};
            let journal = f.base.m.root.join("package-transition.json");
            atomic_json(&journal,&transition).unwrap();
            if boundary != "before_switch" {apply_coordinated(&f.base.m,&transition).unwrap();}
            if boundary == "consumed" {operator_cli::consume_package_resume(&f.base.m,&captured).unwrap();}
            assert!(recover_from(&f.base.m,&f.home,&f.service).unwrap());
            if boundary == "before_switch" {
                assert!(f.current() == first);
                assert_eq!(fs::read(&reservation).unwrap(),original);
                assert_eq!(f.service.show().unwrap().active,"inactive");
                assert_eq!(f.service.starts.get(),0);
            } else {
                assert!(f.current() == next);
                assert!(!reservation.exists());
                assert_eq!(f.service.show().unwrap().active,"active");
            }
            assert_eq!(fs::read(&failed).unwrap(),failed_bytes);
            assert!(!journal.exists());
        }
        });
    }
    #[test]
    fn package_resume_target_failure_restores_paused_origin_and_raw_reservation() {
        readiness::with_unit_state_fixture(&[vendor_cli::unit("arturia-software-center").unwrap()],"inactive",|| {
        for failure in ["start","clean"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let (first,_,receipt,setup) = staged_transition(&f);
            let (reservation,original,failed,failed_bytes) = retired_suspension(&f,&first);
            if failure == "start" {f.service.fail_start.set(true);}
            else {f.service.fail_clean.set(true);}
            let transition = CoordinatedTransition {schema:2,id:random_id().unwrap(),
                kind:TransitionKind::Update,recovery_before:first.clone(),setup,receipt,resume_handoff:None};
            assert!(run_coordinated(&f.base.m,&f.home,&f.service,transition,false).is_err());
            assert!(f.current() == first);
            assert_eq!(fs::read(&reservation).unwrap(),original);
            assert_eq!(fs::read(&failed).unwrap(),failed_bytes);
            assert_eq!(f.service.show().unwrap().active,"inactive");
            assert!(!f.base.m.root.join("package-transition.json").exists());
        }
        });
    }
    #[test]
    fn package_resume_rejects_missing_or_replaced_owner_without_stopping_service() {
        readiness::with_unit_state_fixture(&[vendor_cli::unit("arturia-software-center").unwrap()],"inactive",|| {
        for change in ["missing","replacement","result","vendor","worker"] {
            let f = Fixture::new();
            f.adopt().unwrap();
            let (first,_,receipt,setup) = staged_transition(&f);
            let (reservation,_,failed,_) = retired_suspension(&f,&first);
            let captured = operator_cli::capture_package_resume_with(&f.base.m,&first,
                |_|Ok(false)).unwrap().unwrap();
            let transition = CoordinatedTransition {schema:3,id:random_id().unwrap(),
                kind:TransitionKind::Update,recovery_before:first,setup,receipt,
                resume_handoff:Some(ResumeHandoff {captured,origin_active:false,settling:false})};
            let journal = f.base.m.root.join("package-transition.json");
            atomic_json(&journal,&transition).unwrap();
            apply_coordinated(&f.base.m,&transition).unwrap();
            match change {
                "missing" => fs::remove_file(&reservation).unwrap(),
                "replacement" => {
                    let mut replacement:serde_json::Value = read_json(&reservation).unwrap();
                    replacement["owner_operation"] = "ef".repeat(16).into();
                    atomic_json(&reservation,&replacement).unwrap();
                }
                "result" => atomic_json(&failed,&serde_json::json!({"operation":"ab".repeat(16),"state":"running"})).unwrap(),
                "vendor" => {
                    let path = f.base.m.root.join("vendor-applications/arturia-software-center/operation-result.json");
                    let mut result:serde_json::Value = read_json(&path).unwrap();
                    result["operation_id"] = "ef".repeat(16).into();
                    atomic_json(&path,&result).unwrap();
                }
                "worker" => f.service.worker_active.set(true),
                _ => unreachable!(),
            }
            let retained = fs::read(&reservation).ok();
            let stops = f.service.stops.get();
            assert!(recover_from(&f.base.m,&f.home,&f.service).is_err(),"accepted {change}");
            assert_eq!(f.service.stops.get(),stops);
            assert_eq!(fs::read(&reservation).ok(),retained);
            assert!(journal.exists());
        }
        });
    }
    #[test]
    fn package_resume_clean_readback_rechecks_replacement_owner_before_consuming() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let (first,next,receipt,setup) = staged_transition(&f);
        let (reservation,_,failed,failed_bytes) = retired_suspension(&f,&first);
        *f.service.before_clean.borrow_mut() = Some(Box::new(|m| {
            let path = m.root.join("operator/resume.json");
            let mut changed:serde_json::Value = read_json(&path).unwrap();
            changed["owner_operation"] = "ef".repeat(16).into();
            atomic_json(&path,&changed).unwrap();
        }));
        let transition = CoordinatedTransition {schema:2,id:random_id().unwrap(),
            kind:TransitionKind::Update,recovery_before:first,setup,receipt,resume_handoff:None};
        // This synthetic retired operation has no real user systemd service.
        // Model its exact unit, rather than depending on the test host's bus.
        assert!(readiness::with_unit_state_fixture(
            &["linux-vst-bridge-vendor-arturia-software-center.service".into()],"inactive",
            ||run_coordinated(&f.base.m,&f.home,&f.service,transition,false)).is_err());
        assert!(f.current() == next);
        assert_eq!(f.service.stops.get(),0);
        assert_eq!(read_json::<serde_json::Value>(&reservation).unwrap()["owner_operation"],"ef".repeat(16));
        assert_eq!(fs::read(&failed).unwrap(),failed_bytes);
        assert!(f.base.m.root.join("package-transition.json").exists());
    }
    #[test]
    fn package_resume_active_origin_rollback_is_readable_by_the_old_decoder() {
        readiness::with_unit_state_fixture(&[vendor_cli::unit("arturia-software-center").unwrap()],"inactive",|| {
        for starts in [1,2] {
            let f = Fixture::new();
            f.adopt().unwrap();
            activate_from(&f.base.m,&f.home,&f.service).unwrap();
            let (first,_,receipt,setup) = staged_transition(&f);
            let (reservation,original,_,_) = retired_suspension(&f,&first);
            *f.service.legacy_start_manager.borrow_mut() = Some(first.manager.path.clone());
            f.service.stop().unwrap();
            f.service.fail_starts.set(starts);
            let transition = CoordinatedTransition {schema:2,id:random_id().unwrap(),
                kind:TransitionKind::Update,recovery_before:first.clone(),setup,receipt,resume_handoff:None};
            assert!(run_coordinated(&f.base.m,&f.home,&f.service,transition,true).is_err());
            assert!(f.current() == first);
            assert_eq!(fs::read(&reservation).unwrap(),original);
            let journal = f.base.m.root.join("package-transition.json");
            if starts == 2 {
                let legacy:LegacyTransition = read_json(&journal).unwrap();
                assert_eq!((legacy.schema,legacy.receipt.schema),(2,1));
                assert!(recover_from(&f.base.m,&f.home,&f.service).unwrap());
                assert_eq!(fs::read(&reservation).unwrap(),original);
            }
            assert!(!journal.exists());
            assert_eq!(f.service.show().unwrap().active,"active");
        }
        });
    }
    #[test]
    fn package_resume_cannot_be_bypassed_by_another_generation_switch() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let first = f.current();
        let (reservation,original,failed,failed_bytes) = retired_suspension(&f,&first);
        let before = fs::read(f.base.m.root.join("software.json")).unwrap();
        // Restore first re-checks interrupted sessions. Model the retired
        // app's exact unit rather than depending on the test host's bus.
        let results = readiness::with_unit_state_fixture(
            &["linux-vst-bridge-vendor-arturia-software-center.service".into()],"inactive",
            || [adopt_from(&f.base.m,&f.home,&f.inputs,f.owner,&f.service),
                rollback_from(&f.base.m,&f.home,&f.service),
                restore_from(&f.base.m,&f.home,&f.inputs,f.owner,&f.service)]);
        for result in results {
            assert_eq!(result.unwrap_err().to_string(),"package_resume_requires_coordinated_update");
        }
        assert_eq!(fs::read(f.base.m.root.join("software.json")).unwrap(),before);
        assert_eq!(fs::read(&reservation).unwrap(),original);
        assert_eq!(fs::read(&failed).unwrap(),failed_bytes);
        assert!(!f.base.m.root.join("package-transition.json").exists());
        assert_eq!(f.service.starts.get(),0);
    }
    #[test]
    fn package_resume_settlement_requires_positive_clean_capacity() {
        let mut reply = serde_json::json!({"ok":true,"capacity":{"schema":1,
            "dsp":0,"maintenance":0,"keepers":0,"owners":[],"cleanup_unconfirmed":false}});
        assert!(clean_keepers(&reply).unwrap().is_empty());
        reply["capacity"]["cleanup_unconfirmed"] = true.into();
        assert!(stoppable_keepers(&reply).is_ok());
        assert!(clean_keepers(&reply).is_err());
        reply["capacity"]["cleanup_unconfirmed"] = false.into();
        reply["capacity"]["maintenance"] = 1.into();
        assert!(clean_keepers(&reply).is_err());
        reply["capacity"]["maintenance"] = 0.into();
        reply["ok"] = false.into();
        assert!(clean_keepers(&reply).is_err());
    }
    #[test]
    fn coordinated_update_and_restore_recover_after_old_routes_are_selected() {
        let f = Fixture::new();
        f.adopt().unwrap();
        let (first, next, receipt, setup) = staged_transition(&f);
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone(), resume_handoff:None };
        run_coordinated(&f.base.m, &f.home, &f.service, update, false).unwrap();
        assert!(f.current() == next);
        assert_eq!(f.service.show().unwrap().active, "active");
        assert_eq!(matching_refresh_receipts(&f.base.m, &next,
            &receipt.after_publications).unwrap().len(), 1);

        f.service.stop().unwrap();
        let restore = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Restore, recovery_before:next.clone(),
            setup:setup_install::plan(&f.base.m, &f.home, &first,
                Some(&next)).unwrap(), receipt, resume_handoff:None };
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
            transitions:vec![publication.clone()], resume_history:None };
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone(), resume_handoff:None };
        run_coordinated(&f.base.m, &f.home, &f.service, update, false).unwrap();
        f.service.stop().unwrap();
        let restore = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Restore, recovery_before:next.clone(),
            setup:setup_install::plan(&f.base.m, &f.home, &first,
                Some(&next)).unwrap(), receipt, resume_handoff:None };
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
            transitions:vec![publication], resume_history:None };
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone(), resume_handoff:None };
        run_coordinated(&f.base.m, &f.home, &f.service, update, false).unwrap();
        f.service.stop().unwrap();
        let restore = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Restore, recovery_before:next.clone(),
            setup:setup_install::plan(&f.base.m, &f.home, &first,
                Some(&next)).unwrap(), receipt, resume_handoff:None };
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
            transitions:vec![publication.clone()], resume_history:None };
        let update = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt:receipt.clone(), resume_handoff:None };
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
                Some(&next)).unwrap(), receipt:receipt.clone(), resume_handoff:None };
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
            transitions:vec![publication], resume_history:None };
        let transition = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first, setup, receipt, resume_handoff:None };
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
                recovery_before:first.clone(), setup, receipt, resume_handoff:None };
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
            after_publications:after, transitions:vec![prepared], resume_history:None };
        let transition = CoordinatedTransition { schema:2, id:random_id().unwrap(),
            kind:TransitionKind::Update, recovery_before:first.clone(), setup,
            receipt, resume_handoff:None };
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
            kind:TransitionKind::Update, recovery_before:first, setup, receipt, resume_handoff:None };
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

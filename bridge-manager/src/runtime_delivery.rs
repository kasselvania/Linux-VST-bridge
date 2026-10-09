//! Product-owned runtime acquisition. No Steam installation or caller-selected URL.
//! Runs only on the control plane through an explicit operator setup action.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Component;
use std::os::unix::fs::DirBuilderExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const ID: &str = "managed-ge-proton11-7-slr4-20260805-r3";
const PLATFORM_FILES: &str = "SteamLinuxRuntime_4/steamrt4_platform_4.0.20260805.254769/files";
const LIFETIME_LOCK: &str = "SteamLinuxRuntime_4/steamrt4_platform_4.0.20260805.254769/files/.ref";
const GE: &str = "GE-Proton11-7-x86_64";
const SLR: &str = "SteamLinuxRuntime_4";
const TREE: &str = "runtime-tree.json";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Download {
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub root: String,
    pub compression: String,
}
pub fn downloads() -> Vec<Download> {
    vec![
        Download {
            url: "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/GE-Proton11-7/GE-Proton11-7-x86_64.tar.gz".into(),
            sha256: "c5448b76a230384e2d7bc6beb5ccb97bafb7e2c3b6c527cb03a1a546bbcb00a0".into(),
            size: 563784602, root: GE.into(), compression: "gzip".into(),
        },
        Download {
            url: "https://repo.steampowered.com/steamrt4/images/4.0.20260805.254769/SteamLinuxRuntime_4.tar.xz".into(),
            sha256: "3226d8234e7c0542ee767837832bfb1dad5e5e2dc944ec97eb221b437f6b9349".into(),
            size: 164895128, root: SLR.into(), compression: "xz".into(),
        },
    ]
}
#[derive(Clone)]
struct Revision {
    id: &'static str,
    version: &'static str,
    downloads: Vec<Download>,
    uia_guard: bool,
}
fn revisions() -> Vec<Revision> {
    vec![Revision { id:ID,
        version:"GE-Proton11-7; SLR 4.0.20260805.254769; managed download v3",
        downloads:downloads(),uia_guard:false }]
}
fn recommended() -> Revision {
    // Recommendations apply to acquisition and new environments. Retained
    // environment/publication runner identities never consult this selection.
    revisions().pop().expect("declared runtime revision")
}
fn revision_record_path(m: &Manager, revision: &Revision) -> PathBuf {
    m.root.join("runners").join(revision.id).join("runtime.json")
}
pub fn record_paths(m: &Manager) -> Vec<PathBuf> {
    revisions().iter().map(|revision| revision_record_path(m,revision)).collect()
}
pub fn selected_record_path(m: &Manager, runner: &Runner) -> Result<Option<PathBuf>> {
    let Some(revision) = revisions().into_iter().find(|revision| revision.id==runner.id)
        else {return Ok(None)};
    let path=revision_record_path(m,&revision);
    let base=path.parent().ok_or("runtime_parent")?;
    if runner.proton!=base.join(GE).join("proton")
        || runner.entry_point!=base.join(SLR).join("_v2-entry-point") {return Ok(None)}
    let retained=identity_record(m,&revision)?.ok_or("managed_runtime_selected_record_missing")?;
    require(&retained==runner,"managed_runtime_selected_record_changed")?;
    Ok(Some(path))
}
#[derive(Clone, Debug, Serialize)]
pub struct RecordState {
    pub id: String,
    pub record_path: PathBuf,
    pub sha256: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub runner: Option<Runner>,
    pub failure: Option<String>,
}
fn record_state(m: &Manager, revision: &Revision) -> Result<Option<RecordState>> {
    let path=revision_record_path(m,revision);
    let mut state=RecordState {id:revision.id.into(),record_path:path.clone(),sha256:None,
        metadata:None,runner:None,failure:None};
    let metadata=match fs::symlink_metadata(&path) {
        Ok(metadata)=>metadata,
        Err(error) if error.kind()==std::io::ErrorKind::NotFound=>return Ok(None),
        Err(error)=>{state.failure=Some(failure_text(&error));return Ok(Some(state))},
    };
    state.metadata=Some(serde_json::to_value(DigestFileIdentity::from(&metadata))?);
    let result=(|| -> Result<Runner> {
        let bytes=read_control_bytes(&path,8*1024*1024)?;
        state.sha256=Some(hex(&Sha256::digest(&bytes)));
        let record:Record=serde_json::from_slice(&bytes)?;
        validate_identity_record(&path,revision,&record)?;
        Ok(record.runner)
    })();
    match result {
        Ok(runner)=>{
            state.failure=runner.validate_record().err().map(|error|failure_text(&error));
            state.runner=Some(runner);
        }
        Err(error)=>state.failure=Some(failure_text(&error)),
    }
    Ok(Some(state))
}
fn failure_text(error: &dyn std::fmt::Display) -> String {
    error.to_string().chars().take(2048).collect()
}
pub fn record_states(m: &Manager) -> Result<Vec<RecordState>> {
    states_for(m,&revisions())
}
fn states_for(m: &Manager, revisions: &[Revision]) -> Result<Vec<RecordState>> {
    revisions.iter().filter_map(|revision|match record_state(m,revision) {
        Ok(Some(state))=>Some(Ok(state)),Ok(None)=>None,Err(error)=>Some(Err(error)),
    }).collect()
}
pub fn recommended_record_state(m: &Manager) -> Result<Option<RecordState>> {
    record_state(m,&recommended())
}
pub fn readback_records(m: &Manager) -> Result<Vec<serde_json::Value>> {
    record_states(m)?.into_iter().map(|state|serde_json::to_value(state).map_err(Into::into)).collect()
}
pub fn installation_disabled_reason(m: &Manager) -> Result<Option<String>> {
    Ok(recommended_record_state(m)?.map(|state|match state.failure {
        Some(failure)=>format!("The retained compatibility runtime is unavailable: {failure}. Restore its original files before starting new work."),
        None=>"The selected compatibility runtime is already installed".into(),
    }))
}
pub fn installation_label() -> String {
    let megabytes=recommended().downloads.iter().map(|download|download.size).sum::<u64>()/1_000_000;
    format!("Install compatibility runtime ({megabytes} MB download)")
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record { schema: u32, id: String, downloads: Vec<Download>, runner: Runner }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TreeEntry {
    path: PathBuf, sha256: Option<String>, target: Option<PathBuf>,
    size: u64, mode: u32, directory: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardFile {
    path: PathBuf, original_sha256: String, corrected_sha256: String,
    source: PathBuf, size: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardArtifact { path: PathBuf, sha256: String, size: u64 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardArtifacts {
    test: GuardArtifact, source: GuardArtifact, recipe: GuardArtifact,
    patch: GuardArtifact, guard_source: GuardArtifact, notices: GuardArtifact,
    license: GuardArtifact,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GuardManifest {
    schema: u32, base_ge_sha256: String, files: Vec<GuardFile>, artifacts: GuardArtifacts,
}
fn verify_guard_artifact(root: &Path, artifact: &GuardArtifact) -> Result<()> {
    safe_path(&artifact.path)?;
    require(valid_hex(&artifact.sha256,64) && artifact.size>0
        && artifact.size<=512*1024*1024,"runtime_guard_artifact_identity")?;
    let path=root.join(&artifact.path);
    require(file(&path)?.metadata()?.len()==artifact.size && digest(&path)?==artifact.sha256,
        "runtime_guard_artifact_changed")
}
/// Compose only an unpublished revision. The component archive binds the exact
/// upstream preimage and retains corresponding source, build recipe and notices.
fn compose_guard(stage: &Path, revision: &Revision, rows: &mut Vec<TreeEntry>) -> Result<PathBuf> {
    let root=stage.join("uia-guard");
    let manifest:GuardManifest=read_json(&root.join("manifest.json"))?;
    require(manifest.schema==1 && revision.downloads.first().is_some_and(|base|
        base.root==GE && base.sha256==manifest.base_ge_sha256)
        && (1..=16).contains(&manifest.files.len()),"runtime_guard_base_binding")?;
    for artifact in [&manifest.artifacts.test,&manifest.artifacts.source,&manifest.artifacts.recipe,
        &manifest.artifacts.patch,&manifest.artifacts.guard_source,&manifest.artifacts.notices,
        &manifest.artifacts.license] {verify_guard_artifact(&root,artifact)?;}
    let mut selected=BTreeSet::new();
    // Establish every preimage before changing any staged byte.
    for replacement in &manifest.files {
        safe_path(&replacement.path)?;safe_path(&replacement.source)?;
        require(replacement.path.starts_with("files") && selected.insert(replacement.path.clone())
            && valid_hex(&replacement.original_sha256,64),"runtime_guard_replacement_identity")?;
        let relative=Path::new(GE).join(&replacement.path);
        let declared=rows.iter().find(|row|row.path==relative)
            .ok_or("runtime_guard_preimage_missing")?;
        require(!declared.directory && declared.target.is_none()
            && declared.sha256.as_deref()==Some(replacement.original_sha256.as_str())
            && digest(&stage.join(&relative))?==replacement.original_sha256,
            "runtime_guard_preimage_changed")?;
        verify_guard_artifact(&root,&GuardArtifact {path:replacement.source.clone(),
            sha256:replacement.corrected_sha256.clone(),size:replacement.size})?;
    }
    for replacement in &manifest.files {
        let relative=Path::new(GE).join(&replacement.path);
        let path=stage.join(&relative);
        let row=rows.iter_mut().find(|row|row.path==relative).ok_or("runtime_guard_preimage_missing")?;
        fs::set_permissions(&path,fs::Permissions::from_mode(0o600))?;
        fs::copy(root.join(&replacement.source),&path)?;
        fs::set_permissions(&path,fs::Permissions::from_mode(row.mode))?;
        File::open(&path)?.sync_all()?;
        row.sha256=Some(digest(&path)?);row.size=file(&path)?.metadata()?.len();
        require(row.sha256.as_deref()==Some(replacement.corrected_sha256.as_str())
            && row.size==replacement.size,"runtime_guard_replacement_changed")?;
    }
    let client=stage.join(SLR).join("pressure-vessel/bin/steam-runtime-launch-client");
    let service=stage.join(SLR).join("pressure-vessel/libexec/steam-runtime-tools-0/x86_64-linux-gnu-srt-launcher-service");
    let relative=Path::new(GE).join("native-command-session.json");
    require(!stage.join(&relative).try_exists()?,"runtime_command_component_already_present")?;
    atomic_json(&stage.join(&relative),&serde_json::json!({"schema":1,
        "kind":"native_proton_command_session","client_sha256":digest(&client)?,
        "service_sha256":digest(&service)?}))?;
    fs::set_permissions(stage.join(&relative),fs::Permissions::from_mode(0o400))?;
    rows.push(TreeEntry {path:relative.clone(),sha256:Some(digest(&stage.join(&relative))?),
        target:None,size:file(&stage.join(&relative))?.metadata()?.len(),mode:0o400,directory:false});
    Ok(relative)
}
/// An observation cache, never launch authority. Only a completed full byte
/// verification writes it; readback matches every file's inode and change times.
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct VerifiedTree {
    schema: u32, manifest_sha256: String, files: BTreeMap<PathBuf, DigestFileIdentity>,
}
fn cache_path(manifest: &Artifact) -> Result<PathBuf> {
    require(valid_hex(&manifest.sha256,64), "managed_runtime_manifest_digest")?;
    let runtime = manifest.path.parent().ok_or("managed_runtime_parent")?;
    Ok(runtime.parent().ok_or("managed_runtime_cache_parent")?
        .join(format!(".readback-{}.json",manifest.sha256)))
}
fn cached_tree(manifest: &Artifact) -> Option<VerifiedTree> {
    let path = cache_path(manifest).ok()?;
    let f = file(&path).ok()?;
    if f.metadata().ok()?.mode() & 0o077 != 0 { return None; }
    let cached: VerifiedTree = read_json(&path).ok()?;
    (cached.schema==1 && cached.manifest_sha256==manifest.sha256
        && !cached.files.is_empty() && cached.files.len()<=40000).then_some(cached)
}
/// Warm a missing observation cache before taking projection/registry locks.
/// This changes only cached observations, never the runtime or its identity.
pub fn prepare_readback(m: &Manager) -> Result<()> {
    for runner in installed_identity_records(m)? {
        let manifest = runner.files.iter().find(|a|
            a.path.file_name().is_some_and(|name| name==TREE))
            .ok_or("managed_runtime_tree_missing")?;
        if cached_tree(manifest).is_none() {
            // An unavailable retained payload remains readable for recovery;
            // warming observations cannot make another revision unavailable.
            let _ = runner.verify();
        }
    }
    Ok(())
}
pub fn record_path(m: &Manager) -> PathBuf { revision_record_path(m,&recommended()) }
/// All retained managed ownership records, without admitting any payload.
pub fn installed_identity_records(m: &Manager) -> Result<Vec<Runner>> {
    Ok(record_states(m)?.into_iter().filter_map(|state|state.runner).collect())
}
#[cfg(test)]
fn identity_records(m: &Manager, revisions: &[Revision]) -> Result<Vec<Runner>> {
    Ok(states_for(m,revisions)?.into_iter().filter_map(|state|state.runner).collect())
}
pub fn installed(m: &Manager) -> Result<Option<Runner>> {
    let runner = installed_record(m)?;
    if let Some(runner) = &runner { runner.verify()?; }
    Ok(runner)
}
/// Current owned runtime selection, without walking or verifying its payload.
pub fn installed_record(m: &Manager) -> Result<Option<Runner>> {
    let runner = installed_identity_record(m)?;
    if let Some(runner) = &runner { runner.validate_record()?; }
    Ok(runner)
}
/// Retained ownership identity for recovery readback. This grants no runtime
/// admission: even entry-point metadata is checked separately before selection.
pub fn installed_identity_record(m: &Manager) -> Result<Option<Runner>> {
    identity_record(m,&recommended())
}
fn identity_record(m: &Manager, revision: &Revision) -> Result<Option<Runner>> {
    let path = revision_record_path(m,revision);
    if !path.try_exists()? { return Ok(None); }
    let record: Record = read_json(&path)?;
    validate_identity_record(&path,revision,&record)?;
    Ok(Some(record.runner))
}
fn validate_identity_record(path: &Path, revision: &Revision, record: &Record) -> Result<()> {
    let dir = path.parent().ok_or("runtime_parent")?;
    require(record.schema == 1 && record.id == revision.id && record.downloads == revision.downloads
        && record.runner.id == revision.id
        && record.runner.proton == dir.join(GE).join("proton")
        && record.runner.entry_point == dir.join(SLR).join("_v2-entry-point")
        && record.runner.policy.is_none(), "managed_runtime_binding")?;
    require(record.runner.files.iter().filter(|artifact|artifact.path==dir.join(TREE)).count()==1
        && record.runner.files.iter().all(|artifact|artifact.path.starts_with(dir)),
        "managed_runtime_tree_binding")?;
    if revision.uia_guard {
        require(record.runner.files.iter().filter(|artifact|
            artifact.path==dir.join(GE).join("native-command-session.json")).count()==1,
            "managed_runtime_command_component_binding")?;
    }
    require(file(path)?.metadata()?.mode() & 0o222 == 0,
        "managed_runtime_record_writable")?;
    record.runner.validate_identity()?;
    Ok(())
}
/// Install and each launch admission hash every byte. Repeated checks within
/// one admission reuse its freshly hashed, unchanged file identities. Read-only
/// projections may reuse prior byte observations for unchanged file metadata.
/// All calls check the exact
/// tree, ownership, modes and symlink targets. No verification occurs in DSP.
pub fn verify_tree(runner: &Runner) -> Result<()> {
    verify_tree_mode(runner,readback_digests_active())
}
fn verify_tree_mode(runner: &Runner, readback: bool) -> Result<()> {
    let Some(manifest) = runner.files.iter().find(|a|
        a.path.file_name().is_some_and(|name| name == TREE)) else { return Ok(()); };
    require(file(&manifest.path)?.metadata()?.len() <= 16 * 1024 * 1024,
        "managed_runtime_tree_extent")?;
    let rows: Vec<TreeEntry> = read_json(&manifest.path)?;
    let base = manifest.path.parent().ok_or("managed_runtime_parent")?;
    require(!rows.is_empty() && rows.len() <= 40000, "managed_runtime_tree_count")?;
    let mut seen = BTreeSet::new();
    let cached = cached_tree(manifest);
    let mut verified = VerifiedTree { schema:1, manifest_sha256:manifest.sha256.clone(),
        files:BTreeMap::new() };
    for row in rows {
        safe_path(&row.path)?;
        require(seen.insert(row.path.clone()), "managed_runtime_tree_duplicate")?;
        let path = base.join(&row.path);
        let meta = fs::symlink_metadata(&path)?;
        require(meta.uid() == unsafe { libc::getuid() }, "managed_runtime_tree_owner")?;
        if row.directory {
            require(row.target.is_none() && row.sha256.is_none() && meta.is_dir()
                && meta.mode() & 0o077 == 0, "managed_runtime_directory_changed")?;
        } else if let Some(target) = row.target {
            safe_link(&row.path, &target)?;
            require(row.sha256.is_none() && meta.file_type().is_symlink()
                && fs::read_link(&path)? == target, "managed_runtime_link_changed")?;
        } else {
            require(meta.is_file() && meta.len() == row.size && meta.mode() & 0o777 == row.mode,
                "managed_runtime_file_changed")?;
            let identity = DigestFileIdentity::from(&meta);
            if !readback || cached.as_ref().and_then(|c|c.files.get(&row.path))!=Some(&identity) {
                require(row.sha256.as_deref() == Some(digest(&path)?.as_str()),
                    "managed_runtime_file_changed")?;
                require(DigestFileIdentity::from(&fs::symlink_metadata(&path)?)==identity,
                    "managed_runtime_file_changed_during_verification")?;
            }
            verified.files.insert(row.path,identity);
        }
    }
    let actual = tree_inventory(base)?;
    let actual: BTreeSet<_> = actual.into_iter().filter(|(p,_)|
        p != Path::new(TREE) && p != Path::new("runtime.json")).map(|(p,_)|p).collect();
    require(actual == seen, "managed_runtime_roster_changed")?;
    if !readback && cached.as_ref()!=Some(&verified) {
        atomic_json(&cache_path(manifest)?,&verified)?;
    }
    Ok(())
}
fn tree_inventory(base: &Path) -> Result<Vec<(PathBuf, bool)>> {
    let mut todo = vec![PathBuf::new()];
    let mut rows = Vec::new();
    while let Some(dir) = todo.pop() {
        for entry in fs::read_dir(base.join(dir))? {
            let entry = entry?;
            let relative = entry.path().strip_prefix(base)?.to_owned();
            let directory = entry.file_type()?.is_dir();
            require(rows.len() < 40000, "managed_runtime_tree_count")?;
            if directory { todo.push(relative.clone()); }
            rows.push((relative, directory));
        }
    }
    Ok(rows)
}
fn safe_path(path: &Path) -> Result<()> {
    require(!path.as_os_str().is_empty() && path.components().all(|c|
        matches!(c, Component::Normal(_))), "runtime_archive_path")
}
fn safe_link(path: &Path, target: &Path) -> Result<()> {
    require(!target.is_absolute(), "runtime_archive_link_escape")?;
    let mut depth = path.parent().ok_or("runtime_archive_link_parent")?.components().count();
    for part in target.components() {
        match part {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => {
                require(depth > 1, "runtime_archive_link_escape")?;
                depth -= 1;
            }
            _ => return Err("runtime_archive_link_escape".into()),
        }
    }
    Ok(())
}
/// Pressure-vessel hard-links its platform payload, then normalizes the shared
/// inode to 0644/0755 while constructing a mutable sysroot. Use those canonical
/// modes inside the private runtime directory; verify bytes at every admission.
/// The separate exact empty lifetime lock must remain write-openable.
fn payload_mode(path: &Path, size: u64, upstream_mode: u32) -> Result<u32> {
    if path == Path::new(LIFETIME_LOCK) {
        require(size == 0, "runtime_lifetime_lock_nonempty")?;
        Ok(0o600)
    } else if path.starts_with(PLATFORM_FILES) {
        Ok(if upstream_mode & 0o111 != 0 { 0o755 } else { 0o644 })
    } else {
        Ok(if upstream_mode & 0o111 != 0 { 0o500 } else { 0o400 })
    }
}
fn unpack(input: &Path, spec: &Download, stage: &Path) -> Result<Vec<TreeEntry>> {
    require(file(input)?.metadata()?.len() == spec.size && digest(input)? == spec.sha256,
        "runtime_download_changed")?;
    let input = file(input)?;
    let reader: Box<dyn Read> = match spec.compression.as_str() {
        "gzip" => Box::new(flate2::read::GzDecoder::new(input)),
        "xz" => Box::new(xz2::read::XzDecoder::new(input)),
        _ => return Err("runtime_archive_compression".into()),
    };
    let mut archive = tar::Archive::new(reader);
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    let mut links = Vec::new();
    let mut total = 0_u64;
    let deadline = Instant::now() + Duration::from_secs(600);
    for item in archive.entries()? {
        require(Instant::now() < deadline && seen.len() < 40000, "runtime_archive_bound")?;
        let mut item = item?;
        let path = item.path()?.into_owned();
        safe_path(&path)?;
        require(path.components().next().is_some_and(|c| c.as_os_str() == spec.root.as_str())
            && seen.insert(path.clone()), "runtime_archive_roster")?;
        let kind = item.header().entry_type();
        let out = stage.join(&path);
        if kind.is_dir() {
            fs::DirBuilder::new().recursive(true).mode(0o700).create(out)?;
        } else if kind.is_symlink() {
            let target = item.link_name()?.ok_or("runtime_archive_link")?.into_owned();
            safe_link(&path, &target)?;
            links.push((path, target));
        } else if kind.is_file() {
            let size = item.size();
            total = total.checked_add(size).ok_or("runtime_archive_extent")?;
            require(size <= 512 * 1024 * 1024 && total <= 8 * 1024 * 1024 * 1024,
                "runtime_archive_extent")?;
            fs::DirBuilder::new().recursive(true).mode(0o700)
                .create(out.parent().ok_or("runtime_archive_parent")?)?;
            let mut output = OpenOptions::new().create_new(true).write(true).mode(0o600).open(&out)?;
            require(std::io::copy(&mut item, &mut output)? == size, "runtime_archive_truncated")?;
            output.sync_all()?;
            let mode = payload_mode(&path, size, item.header().mode()?)?;
            fs::set_permissions(&out, fs::Permissions::from_mode(mode))?;
            rows.push(TreeEntry {path, sha256:Some(digest(&out)?), target:None, size, mode, directory:false});
        } else { return Err("runtime_archive_entry_type".into()); }
    }
    // Never extract through a symlink. Links are created only after regular files.
    for (path, target) in links {
        let out = stage.join(&path);
        fs::DirBuilder::new().recursive(true).mode(0o700)
            .create(out.parent().ok_or("runtime_archive_parent")?)?;
        symlink(&target, out)?;
        rows.push(TreeEntry {path, sha256:None, target:Some(target), size:0, mode:0, directory:false});
    }
    rows.sort_by(|a,b| a.path.cmp(&b.path));
    Ok(rows)
}
fn fetch(spec: &Download, path: &Path) -> Result<()> {
    let mut child = Command::new("curl").args(["--fail", "--location", "--silent",
        "--show-error", "--proto", "=https", "--proto-redir", "=https",
        "--connect-timeout", "30", "--max-time", "900", "--max-filesize"])
        .arg(spec.size.to_string()).arg("--output").arg(path).arg(&spec.url)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    let deadline = Instant::now() + Duration::from_secs(920);
    loop {
        if let Some(status) = child.try_wait()? {
            return require(status.success(), "runtime_download_failed_check_connection");
        }
        if Instant::now() >= deadline {
            child.kill()?; child.wait()?; return Err("runtime_download_deadline".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
/// Selected immutable version only; failed staging never replaces a runtime.
pub fn install(m: &Manager) -> Result<Runner> {
    require(cfg!(all(target_os="linux", target_arch="x86_64")),
        "managed_runtime_requires_x86_64_linux")?;
    let _lock = m.lock("runtime-delivery.lock")?;
    if let Some(runner) = installed(m)? { return Ok(runner); }
    let revision = recommended();
    let base = m.root.join("runners");
    private_dir(&base)?;
    let dest = base.join(revision.id);
    require(!dest.try_exists()?, "managed_runtime_incomplete_requires_attention")?;
    let stage = base.join(format!(".stage-{}", random_id()?));
    private_dir(&stage)?;
    let result = (|| -> Result<Runner> {
        let mut rows = Vec::new();
        for (n, spec) in revision.downloads.iter().enumerate() {
            let archive = stage.join(format!("download-{n}"));
            fetch(spec, &archive)?;
            rows.extend(unpack(&archive, spec, &stage)?);
            fs::remove_file(archive)?;
        }
        let command_component=if revision.uia_guard {Some(compose_guard(&stage,&revision,&mut rows)?)} else {None};
        for (path,directory) in tree_inventory(&stage)? {
            if directory { rows.push(TreeEntry {path,sha256:None,target:None,size:0,mode:0o700,directory:true}); }
        }
        atomic_json(&stage.join(TREE), &rows)?;
        fs::set_permissions(stage.join(TREE), fs::Permissions::from_mode(0o400))?;
        let mut files = Vec::new();
        for relative in [TREE, "GE-Proton11-7-x86_64/proton",
            "GE-Proton11-7-x86_64/files/bin/wine", "SteamLinuxRuntime_4/_v2-entry-point",
            "SteamLinuxRuntime_4/pressure-vessel/bin/pressure-vessel-wrap"] {
            files.push(Artifact {path:dest.join(relative), sha256:digest(&stage.join(relative))?});
        }
        if let Some(relative)=command_component {
            files.push(Artifact {path:dest.join(&relative),sha256:digest(&stage.join(relative))?});
        }
        let runner = Runner {id:revision.id.into(),
            version:revision.version.into(),
            proton:dest.join(GE).join("proton"), entry_point:dest.join(SLR).join("_v2-entry-point"),
            files, policy:None};
        let mut staged_runner=runner.clone();
        staged_runner.proton=stage.join(runner.proton.strip_prefix(&dest)?);
        staged_runner.entry_point=stage.join(runner.entry_point.strip_prefix(&dest)?);
        for artifact in &mut staged_runner.files {artifact.path=stage.join(artifact.path.strip_prefix(&dest)?);}
        staged_runner.verify()?;
        atomic_json(&stage.join("runtime.json"), &Record {schema:1,id:revision.id.into(),
            downloads:revision.downloads.clone(),runner:runner.clone()})?;
        fs::set_permissions(stage.join("runtime.json"), fs::Permissions::from_mode(0o400))?;
        fs::rename(&stage, &dest)?;
        File::open(&base)?.sync_all()?;
        installed(m)?.ok_or_else(|| "managed_runtime_install_missing".into())
    })();
    if stage.exists() { fs::remove_dir_all(&stage)?; }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::DirBuilderExt;
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("lvb-runtime-{}",random_id().unwrap()));
            fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
            Self(p)
        }
        fn archive(&self, entries: &[(&str, &[u8])]) -> (PathBuf,Download) {
            let encoder=flate2::write::GzEncoder::new(Vec::new(),flate2::Compression::default());
            let mut tar=tar::Builder::new(encoder);
            for (name,data) in entries {
                let mut header=tar::Header::new_gnu();
                header.set_size(data.len() as u64);header.set_mode(0o755);header.set_cksum();
                tar.append_data(&mut header,name,*data).unwrap();
            }
            let data=tar.into_inner().unwrap().finish().unwrap();
            let path=self.0.join("archive");fs::write(&path,&data).unwrap();
            let spec=Download {url:"https://test.invalid/fixed".into(),sha256:digest(&path).unwrap(),
                size:data.len() as u64,root:"GE".into(),compression:"gzip".into()};
            (path,spec)
        }
    }
    impl Drop for Scratch { fn drop(&mut self) { let _=fs::remove_dir_all(&self.0); } }
    fn retained_revision(m: &Manager, revision: &Revision) -> Runner {
        let base = revision_record_path(m,revision).parent().unwrap().to_owned();
        fs::DirBuilder::new().recursive(true).mode(0o700).create(&base).unwrap();
        let mut files = Vec::new();
        for relative in [format!("{GE}/proton"),format!("{SLR}/_v2-entry-point")] {
            let path=base.join(relative);
            fs::DirBuilder::new().recursive(true).mode(0o700).create(path.parent().unwrap()).unwrap();
            fs::write(&path,revision.id).unwrap();
            fs::set_permissions(&path,fs::Permissions::from_mode(0o500)).unwrap();
            files.push(Artifact {sha256:digest(&path).unwrap(),path});
        }
        let rows=tree_inventory(&base).unwrap().into_iter().map(|(path,directory)| {
            let absolute=base.join(&path);
            TreeEntry {path,sha256:(!directory).then(||digest(&absolute).unwrap()),
                target:None,size:if directory {0} else {fs::metadata(&absolute).unwrap().len()},
                mode:if directory {0o700} else {0o500},directory}
        }).collect::<Vec<_>>();
        let manifest=base.join(TREE);
        atomic_json(&manifest,&rows).unwrap();
        fs::set_permissions(&manifest,fs::Permissions::from_mode(0o400)).unwrap();
        files.push(Artifact {sha256:digest(&manifest).unwrap(),path:manifest});
        let runner=Runner {id:revision.id.into(),version:revision.version.into(),
            proton:base.join(GE).join("proton"),entry_point:base.join(SLR).join("_v2-entry-point"),
            files,policy:None};
        let record=revision_record_path(m,revision);
        atomic_json(&record,&Record {schema:1,id:revision.id.into(),
            downloads:revision.downloads.clone(),runner:runner.clone()}).unwrap();
        fs::set_permissions(record,fs::Permissions::from_mode(0o400)).unwrap();
        runner
    }
    #[test]
    fn retained_revisions_coexist_without_substitution_when_old_payload_is_missing() {
        let tmp=Scratch::new();
        let m=Manager {root:tmp.0.clone(),publications:tmp.0.join("publications")};
        let old=recommended();
        let new=Revision {id:"managed-next-test",version:"next-test",downloads:downloads(),uia_guard:false};
        let prior=retained_revision(&m,&old);
        let next=retained_revision(&m,&new);
        let previous_record=fs::read(revision_record_path(&m,&old)).unwrap();
        assert_eq!(identity_records(&m,&[old.clone(),new.clone()]).unwrap(),[prior.clone(),next.clone()]);
        prior.verify().unwrap();next.verify().unwrap();
        fs::remove_file(&prior.proton).unwrap();
        assert_eq!(identity_record(&m,&old).unwrap(),Some(prior.clone()));
        assert!(prior.verify().is_err());
        assert_eq!(identity_record(&m,&new).unwrap(),Some(next.clone()));
        next.verify().unwrap();
        assert_eq!(fs::read(revision_record_path(&m,&old)).unwrap(),previous_record);
        assert_eq!(installed_identity_record(&m).unwrap(),Some(prior));
        assert!(installed(&m).is_err(),"another retained revision cannot repair the selected identity");
    }
    #[test]
    fn incomplete_acquisition_is_never_an_installed_revision() {
        let tmp=Scratch::new();
        let m=Manager {root:tmp.0.clone(),publications:tmp.0.join("publications")};
        let old=recommended();
        let new=Revision {id:"managed-next-test",version:"next-test",downloads:downloads(),uia_guard:false};
        let prior=retained_revision(&m,&old);
        let incomplete=revision_record_path(&m,&new).parent().unwrap().to_owned();
        fs::DirBuilder::new().recursive(true).mode(0o700).create(&incomplete).unwrap();
        fs::write(incomplete.join("download-0"),b"partial").unwrap();
        assert!(identity_record(&m,&new).unwrap().is_none());
        assert_eq!(identity_records(&m,&[old,new]).unwrap(),[prior]);
        let stage=m.root.join("runners/.stage-unpublished");
        fs::DirBuilder::new().recursive(true).mode(0o700).create(&stage).unwrap();
        fs::write(stage.join("runtime.json"),b"partial").unwrap();
        assert_eq!(installed_identity_records(&m).unwrap().len(),1);
    }
    #[test]
    fn malformed_old_record_preserves_failure_and_does_not_hide_an_independent_revision() {
        let tmp=Scratch::new();
        let m=Manager {root:tmp.0.clone(),publications:tmp.0.join("publications")};
        let old=recommended();
        let new=Revision {id:"managed-next-test",version:"next-test",downloads:downloads(),uia_guard:false};
        retained_revision(&m,&old);
        let next=retained_revision(&m,&new);
        let path=revision_record_path(&m,&old);
        fs::set_permissions(&path,fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&path,b"invalid owned record").unwrap();
        fs::set_permissions(&path,fs::Permissions::from_mode(0o400)).unwrap();
        let before=fs::read(&path).unwrap();
        let states=states_for(&m,&[old.clone(),new.clone()]).unwrap();
        assert_eq!(states[0].id,old.id);assert_eq!(states[0].record_path,path);
        assert!(states[0].runner.is_none());assert!(states[0].failure.is_some());
        assert_eq!(states[0].sha256,Some(digest(&path).unwrap()));
        assert_eq!(states[1].runner,Some(next.clone()));assert!(states[1].failure.is_none());
        assert_eq!(identity_records(&m,&[old.clone(),new.clone()]).unwrap().as_slice(),
            std::slice::from_ref(&next));
        assert!(identity_record(&m,&old).is_err());
        next.verify().unwrap();
        assert_eq!(fs::read(path).unwrap(),before);
        assert_eq!(failure_text(&"x".repeat(8192)).len(),2048);
    }
    #[test]
    fn unreadable_old_record_does_not_hide_an_independent_owned_revision() {
        if unsafe {libc::getuid()}==0 {return;}
        let tmp=Scratch::new();
        let m=Manager {root:tmp.0.clone(),publications:tmp.0.join("publications")};
        let old=recommended();
        let new=Revision {id:"managed-next-test",version:"next-test",downloads:downloads(),uia_guard:false};
        retained_revision(&m,&old);let next=retained_revision(&m,&new);
        let directory=revision_record_path(&m,&old).parent().unwrap().to_owned();
        fs::set_permissions(&directory,fs::Permissions::from_mode(0o000)).unwrap();
        let states=states_for(&m,&[old,new]);
        fs::set_permissions(&directory,fs::Permissions::from_mode(0o700)).unwrap();
        let states=states.unwrap();
        assert!(states[0].runner.is_none() && states[0].metadata.is_none() && states[0].sha256.is_none());
        assert!(states[0].failure.is_some());assert_eq!(states[1].runner,Some(next.clone()));
        next.verify().unwrap();
    }
    #[test]
    fn managed_record_stamp_requires_exact_owned_runner_and_does_not_claim_another_root() {
        let tmp=Scratch::new();
        let m=Manager {root:tmp.0.clone(),publications:tmp.0.join("publications")};
        let revision=recommended();let runner=retained_revision(&m,&revision);
        assert_eq!(selected_record_path(&m,&runner).unwrap(),Some(record_path(&m)));
        let mut changed=runner.clone();changed.version="different declaration".into();
        assert!(selected_record_path(&m,&changed).unwrap_err().to_string().contains("selected_record_changed"));
        changed=runner.clone();changed.proton=tmp.0.join("another-root/proton");
        assert!(selected_record_path(&m,&changed).unwrap().is_none());
        fs::remove_file(record_path(&m)).unwrap();
        assert!(selected_record_path(&m,&runner).unwrap_err().to_string().contains("selected_record_missing"));
    }
    fn guard_fixture(tmp: &Scratch) -> (PathBuf,Revision,Vec<TreeEntry>) {
        let stage=tmp.0.join("guard-stage");
        fs::DirBuilder::new().mode(0o700).create(&stage).unwrap();
        let (archive,mut ge)=tmp.archive(&[
            ("GE-Proton11-7-x86_64/proton",b"proton"),
            ("GE-Proton11-7-x86_64/files/bin/wine",b"wine"),
            ("GE-Proton11-7-x86_64/files/lib/wine/x86_64-windows/uiautomationcore.dll",b"original DLL")]);
        ge.root=GE.into();let mut rows=unpack(&archive,&ge,&stage).unwrap();
        let (archive,mut slr)=tmp.archive(&[
            ("SteamLinuxRuntime_4/_v2-entry-point",b"entry"),
            ("SteamLinuxRuntime_4/pressure-vessel/bin/pressure-vessel-wrap",b"wrap"),
            ("SteamLinuxRuntime_4/pressure-vessel/bin/steam-runtime-launch-client",b"client"),
            ("SteamLinuxRuntime_4/pressure-vessel/libexec/steam-runtime-tools-0/x86_64-linux-gnu-srt-launcher-service",b"service")]);
        slr.root=SLR.into();rows.extend(unpack(&archive,&slr,&stage).unwrap());
        let names=[("test","uiautomationcore_test.exe"),("source","wine-source.tar.gz"),
            ("recipe","build.py"),("patch","uia-null-provider.patch"),
            ("guard_source","guard_provider.c"),("notices","THIRD_PARTY_NOTICES.txt"),("license","COPYING.LIB")];
        let mut artifacts=serde_json::Map::new();
        let mut payloads=vec![("uia-guard/uiautomationcore.dll".to_owned(),b"corrected DLL".to_vec())];
        for (key,name) in names {
            let data=format!("{name} fixture").into_bytes();
            artifacts.insert(key.into(),serde_json::json!({"path":name,"sha256":hex(&Sha256::digest(&data)),"size":data.len()}));
            payloads.push((format!("uia-guard/{name}"),data));
        }
        let manifest=serde_json::json!({"schema":1,"base_ge_sha256":ge.sha256,
            "files":[{"path":"files/lib/wine/x86_64-windows/uiautomationcore.dll",
                "original_sha256":hex(&Sha256::digest(b"original DLL")),
                "corrected_sha256":hex(&Sha256::digest(b"corrected DLL")),
                "source":"uiautomationcore.dll","size":b"corrected DLL".len()}],"artifacts":artifacts});
        payloads.push(("uia-guard/manifest.json".into(),serde_json::to_vec(&manifest).unwrap()));
        let entries=payloads.iter().map(|(name,data)|(name.as_str(),data.as_slice())).collect::<Vec<_>>();
        let (archive,mut component)=tmp.archive(&entries);component.root="uia-guard".into();
        rows.extend(unpack(&archive,&component,&stage).unwrap());
        (stage,Revision {id:"managed-uia-test",version:"uia fixture",downloads:vec![ge,slr,component],uia_guard:true},rows)
    }
    #[test]
    fn corrected_composition_seals_new_bytes_source_notices_and_declared_command_adapter() {
        let tmp=Scratch::new();let (stage,revision,mut rows)=guard_fixture(&tmp);
        let component=compose_guard(&stage,&revision,&mut rows).unwrap();
        let relative=Path::new(GE).join("files/lib/wine/x86_64-windows/uiautomationcore.dll");
        assert_eq!(fs::read(stage.join(&relative)).unwrap(),b"corrected DLL");
        assert_eq!(rows.iter().find(|row|row.path==relative).unwrap().sha256,
            Some(hex(&Sha256::digest(b"corrected DLL"))));
        assert!(rows.iter().any(|row|row.path==Path::new("uia-guard/wine-source.tar.gz")));
        assert!(rows.iter().any(|row|row.path==Path::new("uia-guard/THIRD_PARTY_NOTICES.txt")));
        for (path,directory) in tree_inventory(&stage).unwrap() {
            if directory {rows.push(TreeEntry {path,sha256:None,target:None,size:0,mode:0o700,directory:true});}
        }
        atomic_json(&stage.join(TREE),&rows).unwrap();
        fs::set_permissions(stage.join(TREE),fs::Permissions::from_mode(0o400)).unwrap();
        let runner=Runner {id:revision.id.into(),version:revision.version.into(),
            proton:stage.join(GE).join("proton"),entry_point:stage.join(SLR).join("_v2-entry-point"),policy:None,
            files:[PathBuf::from(TREE),Path::new(GE).join("proton"),Path::new(SLR).join("_v2-entry-point"),component.clone()]
                .into_iter().map(|relative|Artifact {path:stage.join(&relative),sha256:digest(&stage.join(relative)).unwrap()}).collect()};
        runner.verify().unwrap();
        let declared:serde_json::Value=read_json(&stage.join(component)).unwrap();
        assert_eq!(declared["kind"],"native_proton_command_session");
        assert_eq!(declared["client_sha256"],hex(&Sha256::digest(b"client")));
        fs::set_permissions(stage.join(&relative),fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(stage.join(&relative),b"different DLL").unwrap();
        fs::set_permissions(stage.join(&relative),fs::Permissions::from_mode(0o500)).unwrap();
        assert!(runner.verify().unwrap_err().to_string().contains("managed_runtime_file_changed"));
    }
    #[test]
    fn wrong_preimage_or_corresponding_source_refuses_before_composition() {
        for damage_source in [false,true] {
            let tmp=Scratch::new();let (stage,revision,mut rows)=guard_fixture(&tmp);
            let path=if damage_source {stage.join("uia-guard/wine-source.tar.gz")}
                else {stage.join(GE).join("files/lib/wine/x86_64-windows/uiautomationcore.dll")};
            fs::set_permissions(&path,fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(&path,b"changed").unwrap();
            fs::set_permissions(&path,fs::Permissions::from_mode(0o500)).unwrap();
            let error=compose_guard(&stage,&revision,&mut rows).unwrap_err().to_string();
            assert!(error.contains(if damage_source {"guard_artifact_changed"} else {"guard_preimage_changed"}),"{error}");
            assert!(!stage.join(GE).join("native-command-session.json").exists());
            if damage_source {assert_eq!(fs::read(stage.join(GE).join("files/lib/wine/x86_64-windows/uiautomationcore.dll")).unwrap(),b"original DLL");}
        }
    }
    #[test]
    fn corrupt_download_and_duplicate_entries_never_install() {
        let tmp=Scratch::new();
        let stage=tmp.0.join("stage");fs::DirBuilder::new().mode(0o700).create(&stage).unwrap();
        let (archive,spec)=tmp.archive(&[("GE/proton",b"original")]);
        fs::write(&archive,b"changed").unwrap();
        assert!(unpack(&archive,&spec,&stage).unwrap_err().to_string().contains("runtime_download_changed"));
        assert_eq!(fs::read_dir(&stage).unwrap().count(),0);
        let (archive,spec)=tmp.archive(&[("GE/proton",b"one"),("GE/proton",b"two")]);
        assert!(unpack(&archive,&spec,&stage).unwrap_err().to_string().contains("runtime_archive_roster"));
    }
    #[test]
    fn runtime_tree_refuses_changed_bytes_and_added_files() {
        let tmp=Scratch::new();
        let stage=tmp.0.join("stage");fs::DirBuilder::new().mode(0o700).create(&stage).unwrap();
        let (archive,spec)=tmp.archive(&[("GE/proton",b"original")]);
        let mut rows=unpack(&archive,&spec,&stage).unwrap();
        for (path,directory) in tree_inventory(&stage).unwrap() {
            if directory { rows.push(TreeEntry {path,sha256:None,target:None,size:0,mode:0o700,directory:true}); }
        }
        atomic_json(&stage.join(TREE),&rows).unwrap();
        let runner=Runner {id:ID.into(),version:"test".into(),proton:stage.join("GE/proton"),
            entry_point:stage.join("GE/proton"),policy:None,
            files:vec![Artifact {path:stage.join(TREE),sha256:digest(&stage.join(TREE)).unwrap()}]};
        verify_tree(&runner).unwrap();
        assert!(cached_tree(&runner.files[0]).is_some());
        with_readback_digests(|| {
            verify_tree(&runner).unwrap();
            assert!(SCOPED_DIGESTS.with(|cache|cache.borrow().as_ref().unwrap().is_empty()),
                "unchanged cached readback must not hash runtime payloads");
        });
        // Execution ignores the persisted readback cache and a surrounding
        // readback scope. Only bytes read in this fresh admission are reusable.
        with_readback_digests(|| with_launch_verification(|| {
            assert!(!readback_digests_active());
            verify_tree(&runner).unwrap();
            assert_eq!(SCOPED_DIGESTS.with(|cache| cache.borrow().as_ref().unwrap().len()),1);
            verify_tree(&runner).unwrap();
            assert_eq!(SCOPED_DIGESTS.with(|cache| cache.borrow().as_ref().unwrap().len()),1);
            let payload=stage.join("GE/proton");
            fs::set_permissions(&payload,fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(&payload,b"modified").unwrap();
            fs::set_permissions(&payload,fs::Permissions::from_mode(0o500)).unwrap();
            assert!(verify_tree(&runner).unwrap_err().to_string().contains("file_changed"));
            fs::set_permissions(&payload,fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(&payload,b"original").unwrap();
            fs::set_permissions(&payload,fs::Permissions::from_mode(0o500)).unwrap();
            verify_tree(&runner).unwrap();
            with_readback_digests(|| assert!(!readback_digests_active()));
        }));
        assert!(SCOPED_DIGESTS.with(|cache|cache.borrow().is_none()));
        fs::write(stage.join("GE/injected.dll"),b"extra").unwrap();
        assert!(with_readback_digests(||verify_tree(&runner)).unwrap_err().to_string().contains("roster_changed"));
        fs::remove_file(stage.join("GE/injected.dll")).unwrap();
        fs::set_permissions(stage.join("GE/proton"),fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(stage.join("GE/proton"),b"modified").unwrap();
        fs::set_permissions(stage.join("GE/proton"),fs::Permissions::from_mode(0o500)).unwrap();
        assert!(with_readback_digests(||verify_tree(&runner)).unwrap_err().to_string().contains("file_changed"));
        // Even a forged observation stamp cannot authorize launch: the ordinary
        // verification path ignores cached payload observations and reads bytes.
        let mut forged = cached_tree(&runner.files[0]).unwrap();
        forged.files.insert(PathBuf::from("GE/proton"),
            DigestFileIdentity::from(&fs::symlink_metadata(stage.join("GE/proton")).unwrap()));
        atomic_json(&cache_path(&runner.files[0]).unwrap(),&forged).unwrap();
        assert!(verify_tree(&runner).unwrap_err().to_string().contains("file_changed"));
        assert!(with_launch_verification(||verify_tree(&runner)).unwrap_err().to_string().contains("file_changed"));
        let shared = LaunchVerification::default();
        assert!(shared.prepare(Instant::now()+Duration::from_secs(2),
            || verify_tree(&runner)).err().unwrap().to_string().contains("file_changed"));
        assert!(shared.records.lock().unwrap().is_empty(),
            "a forged persisted observation cannot populate process launch authority");
    }
    #[test]
    fn lifetime_lock_is_exact_empty_and_write_openable() {
        let tmp=Scratch::new();
        let stage=tmp.0.join("stage");fs::DirBuilder::new().mode(0o700).create(&stage).unwrap();
        let (archive,mut spec)=tmp.archive(&[(LIFETIME_LOCK,b"")]);spec.root=SLR.into();
        let rows=unpack(&archive,&spec,&stage).unwrap();
        assert_eq!(rows[0].mode,0o600);
        assert_eq!(rows[0].sha256.as_deref(),Some(hex(&Sha256::digest(b"")).as_str()));
        OpenOptions::new().read(true).write(true).open(stage.join(LIFETIME_LOCK)).unwrap();
        assert_eq!(payload_mode(Path::new("GE/.ref"),0,0o644).unwrap(),0o400);
        assert_eq!(payload_mode(Path::new("GE/proton"),0,0o755).unwrap(),0o500);
        assert!(payload_mode(Path::new(LIFETIME_LOCK),1,0o644).is_err());
    }
    #[test]
    fn platform_hard_link_normalization_preserves_verified_source() {
        let tmp=Scratch::new();
        let stage=tmp.0.join("stage");fs::DirBuilder::new().mode(0o700).create(&stage).unwrap();
        let relative=format!("{PLATFORM_FILES}/bin/reference");
        let (archive,mut spec)=tmp.archive(&[(&relative,b"original")]);spec.root=SLR.into();
        let mut rows=unpack(&archive,&spec,&stage).unwrap();
        assert_eq!(rows[0].mode,0o755);
        assert_eq!(payload_mode(Path::new(&format!("{PLATFORM_FILES}/lib/data")),1,0o644).unwrap(),0o644);
        for (path,directory) in tree_inventory(&stage).unwrap() {
            if directory { rows.push(TreeEntry {path,sha256:None,target:None,size:0,mode:0o700,directory:true}); }
        }
        atomic_json(&stage.join(TREE),&rows).unwrap();
        let source=stage.join(relative);
        let runner=Runner {id:ID.into(),version:"test".into(),proton:source.clone(),
            entry_point:source.clone(),policy:None,
            files:vec![Artifact {path:stage.join(TREE),sha256:digest(&stage.join(TREE)).unwrap()}]};
        verify_tree(&runner).unwrap();
        let linked=tmp.0.join("mutable-reference");
        fs::hard_link(&source,&linked).unwrap();
        // The upstream copy normalizes the shared inode, not a separate file.
        fs::set_permissions(&linked,fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(fs::metadata(&source).unwrap().ino(),fs::metadata(&linked).unwrap().ino());
        verify_tree(&runner).unwrap();
        fs::write(&linked,b"modified").unwrap();
        assert!(with_readback_digests(||verify_tree(&runner)).unwrap_err().to_string().contains("file_changed"));
        assert!(verify_tree(&runner).unwrap_err().to_string().contains("file_changed"));
    }
    #[test]
    fn archive_paths_and_links_cannot_escape() {
        for bad in ["/tmp/escape", "../escape", "GE/../../escape", ""] {
            assert!(safe_path(Path::new(bad)).is_err());
        }
        assert!(safe_link(Path::new("GE/files/default/a"),Path::new("../../lib/a")).is_ok());
        for bad in ["/tmp/a","../../../../a"] {
            assert!(safe_link(Path::new("GE/files/a"),Path::new(bad)).is_err());
        }
    }
    #[test]
    fn sources_are_exact_and_do_not_follow_latest() {
        let inputs=downloads();
        assert_eq!(inputs.len(),2);
        assert!(inputs.iter().all(|d| d.url.starts_with("https://") &&
            !d.url.contains("latest") && valid_hex(&d.sha256,64) && d.size>0));
    }
}

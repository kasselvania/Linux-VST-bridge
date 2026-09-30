//! Product-owned runtime acquisition. No Steam installation or caller-selected URL.
//! Runs only on the control plane through an explicit operator setup action.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Component;
use std::os::unix::fs::DirBuilderExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const ID: &str = "managed-ge-proton11-7-slr4-20260805-r2";
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
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record { schema: u32, id: String, downloads: Vec<Download>, runner: Runner }
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TreeEntry {
    path: PathBuf, sha256: Option<String>, target: Option<PathBuf>,
    size: u64, mode: u32, directory: bool,
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
    let path = record_path(m);
    if !path.try_exists()? { return Ok(()); }
    let record: Record = read_json(&path)?;
    let manifest = record.runner.files.iter().find(|a|
        a.path.file_name().is_some_and(|name| name==TREE))
        .ok_or("managed_runtime_tree_missing")?;
    if cached_tree(manifest).is_none() {
        let runner = installed(m)?.ok_or("managed_runtime_install_missing")?;
        verify_tree_mode(&runner,false)?;
    }
    Ok(())
}
pub fn record_path(m: &Manager) -> PathBuf { m.root.join("runners").join(ID).join("runtime.json") }
pub fn installed(m: &Manager) -> Result<Option<Runner>> {
    let path = record_path(m);
    if !path.try_exists()? { return Ok(None); }
    let record: Record = read_json(&path)?;
    let dir = path.parent().ok_or("runtime_parent")?;
    require(record.schema == 1 && record.id == ID && record.downloads == downloads()
        && record.runner.id == ID
        && record.runner.proton == dir.join(GE).join("proton")
        && record.runner.entry_point == dir.join(SLR).join("_v2-entry-point")
        && record.runner.policy.is_none(), "managed_runtime_binding")?;
    require(file(&path)?.metadata()?.mode() & 0o222 == 0,
        "managed_runtime_record_writable")?;
    record.runner.verify()?;
    Ok(Some(record.runner))
}
/// Install and launch hash every byte. Read-only projections may reuse a prior
/// byte observation only for unchanged file metadata. All calls check the exact
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
    let actual = inventory(base)?;
    let actual: BTreeSet<_> = actual.into_iter().filter(|(p,_)|
        p != Path::new(TREE) && p != Path::new("runtime.json")).map(|(p,_)|p).collect();
    require(actual == seen, "managed_runtime_roster_changed")?;
    if !readback && cached.as_ref()!=Some(&verified) {
        atomic_json(&cache_path(manifest)?,&verified)?;
    }
    Ok(())
}
fn inventory(base: &Path) -> Result<Vec<(PathBuf, bool)>> {
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
/// Pressure-vessel opens this exact empty lifetime lock read/write. Its bytes
/// remain pinned to empty; no executable or general runtime path is writable.
fn payload_mode(path: &Path, size: u64, upstream_mode: u32) -> Result<u32> {
    if path == Path::new(LIFETIME_LOCK) {
        require(size == 0, "runtime_lifetime_lock_nonempty")?;
        Ok(0o600)
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
    let base = m.root.join("runners");
    private_dir(&base)?;
    let dest = base.join(ID);
    require(!dest.try_exists()?, "managed_runtime_incomplete_requires_attention")?;
    let stage = base.join(format!(".stage-{}", random_id()?));
    private_dir(&stage)?;
    let result = (|| -> Result<Runner> {
        let mut rows = Vec::new();
        for (n, spec) in downloads().iter().enumerate() {
            let archive = stage.join(format!("download-{n}"));
            fetch(spec, &archive)?;
            rows.extend(unpack(&archive, spec, &stage)?);
            fs::remove_file(archive)?;
        }
        for (path,directory) in inventory(&stage)? {
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
        let runner = Runner {id:ID.into(),
            version:"GE-Proton11-7; SLR 4.0.20260805.254769; managed download v2".into(),
            proton:dest.join(GE).join("proton"), entry_point:dest.join(SLR).join("_v2-entry-point"),
            files, policy:None};
        atomic_json(&stage.join("runtime.json"), &Record {schema:1,id:ID.into(),
            downloads:downloads(),runner:runner.clone()})?;
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
        for (path,directory) in inventory(&stage).unwrap() {
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
            assert!(READBACK_DIGESTS.with(|cache|cache.borrow().as_ref().unwrap().is_empty()),
                "unchanged cached readback must not hash runtime payloads");
        });
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
    }
    #[test]
    fn only_exact_empty_lifetime_lock_is_write_openable() {
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

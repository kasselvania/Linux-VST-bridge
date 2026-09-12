//! Canonical, inactive-only registration and atomic publication. No SDK or DSP here.
pub mod acceptance;
pub mod capacity;
pub mod catalogue;
#[cfg(test)]
mod managed_tests;
pub mod observation;
pub mod profiles;
pub mod pigments;
pub mod publication;
pub mod qualification;
pub mod readback;
#[cfg(test)]
mod test_fixture;
pub mod vendor_application;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{symlink, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(why.into())
    }
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn valid_hex(s: &str, n: usize) -> bool {
    s.len() == n && s.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn random_id() -> Result<String> {
    let mut b = [0u8; 16];
    File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(hex(&b))
}
pub fn private_dir(p: &Path) -> Result<()> {
    fs::DirBuilder::new().recursive(true).create(p)?;
    let m = fs::symlink_metadata(p)?;
    require(
        m.is_dir() && m.uid() == unsafe { libc::getuid() },
        "directory ownership/type differs",
    )?;
    // Creation callers use umask 077; never chmod an existing public directory.
    require(m.mode() & 0o077 == 0, "directory must be private")
}
pub fn file(p: &Path) -> Result<File> {
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(p)?;
    let m = f.metadata()?;
    require(
        m.is_file() && m.uid() == unsafe { libc::getuid() },
        "file ownership/type differs",
    )?;
    Ok(f)
}
pub fn digest(p: &Path) -> Result<String> {
    let mut f = file(p)?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(hex(&h.finalize()))
}
pub fn read_json<T: for<'de> Deserialize<'de>>(p: &Path) -> Result<T> {
    let f = file(p)?;
    require(f.metadata()?.len() <= 8 * 1024 * 1024, "JSON size limit")?;
    Ok(serde_json::from_reader(f.take(8 * 1024 * 1024 + 1))?)
}
pub fn atomic_json<T: Serialize>(p: &Path, data: &T) -> Result<()> {
    let temp = p.with_extension(format!("tmp-{}", random_id()?));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    let result = (|| {
        serde_json::to_writer(&mut f, data)?;
        f.write_all(b"\n")?;
        f.sync_all()?;
        fs::rename(&temp, p)?;
        File::open(p.parent().ok_or("parent absent")?)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: PathBuf,
    pub sha256: String,
}
impl Artifact {
    pub fn verify(&self) -> Result<()> {
        require(
            self.path.is_absolute() && valid_hex(&self.sha256, 64),
            "artifact identity syntax",
        )?;
        require(
            digest(&self.path)? == self.sha256,
            "artifact missing or changed",
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Runner {
    pub id: String,
    pub version: String,
    pub proton: PathBuf,
    pub entry_point: PathBuf,
    pub files: Vec<Artifact>,
}
impl Runner {
    pub fn verify(&self) -> Result<()> {
        require(
            !self.id.is_empty()
                && !self.version.is_empty()
                && (2..=128).contains(&self.files.len()),
            "runner identity incomplete",
        )?;
        require(
            self.files.iter().any(|f| f.path == self.proton)
                && self.files.iter().any(|f| f.path == self.entry_point),
            "runner entry points not pinned",
        )?;
        for f in &self.files {
            f.verify()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub id: String,
    pub root: PathBuf,
    pub runner: Runner,
    pub revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub class_id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub subcategories: String,
    pub metadata_tier: String,
}
impl Metadata {
    pub fn verify(&self) -> Result<()> {
        require(valid_hex(&self.class_id, 32), "class ID syntax")?;
        for (v, n) in [
            (&self.name, 63),
            (&self.vendor, 63),
            (&self.version, 63),
            (&self.subcategories, 127),
        ] {
            require(
                !v.is_empty() && v.len() <= n && !v.chars().any(char::is_control),
                "SDK metadata bound",
            )?;
        }
        require(
            matches!(
                self.metadata_tier.as_str(),
                "factory_2" | "factory_3_unicode"
            ),
            "declared category unavailable",
        )?;
        let c: Vec<_> = self.subcategories.split('|').collect();
        require(
            c.contains(&"Fx") != c.contains(&"Instrument"),
            "ambiguous or absent SDK role",
        )
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_lifetime: Option<profiles::EditorLifetime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_output: Option<profiles::EventOutputPolicy>,
    pub disable_windows_accessibility: bool,
}
/// Installed performance preference, independently versioned from vendor state
/// and the class registry. Missing records preserve the accepted 512-frame path.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Performance {
    pub schema: u32,
    pub added_frames: u32,
}
impl Default for Performance {
    fn default() -> Self {
        Self {
            schema: 1,
            added_frames: 512,
        }
    }
}
impl Performance {
    pub fn verify(&self) -> Result<()> {
        require(
            self.schema == 1 && matches!(self.added_frames, 256 | 512),
            "unsupported performance schema or delay (use 256 or 512 frames)",
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub metadata: Metadata,
    pub environment: Environment,
    pub module: Artifact,
    pub host: Artifact,
    pub host_source_sha256: String,
    pub native: Artifact,
    pub compatibility: Compatibility,
}
impl Registration {
    pub fn key(&self) -> String {
        self.metadata.class_id.to_uppercase()
    }
    pub fn verify(&self, root: &Path) -> Result<()> {
        self.metadata.verify()?;
        require(
            self.environment.revision > 0 && !self.environment.id.is_empty(),
            "environment revision absent",
        )?;
        let parent = root.join("environments");
        require(
            self.environment.root.parent() == Some(parent.as_path())
                && self.environment.root.file_name().and_then(|s| s.to_str())
                    == Some(&self.environment.id),
            "environment identity/path differs",
        )?;
        private_dir(&self.environment.root)?;
        require(
            self.environment.root.canonicalize()? == self.environment.root,
            "environment path traverses symlink",
        )?;
        let installed: Environment = read_json(&self.environment.root.join("environment.json"))?;
        require(
            installed == self.environment,
            "registered environment revision differs",
        )?;
        require(
            self.host.path.starts_with(root.join("software")),
            "host is not installed product software",
        )?;
        require(
            self.module
                .path
                .starts_with(self.environment.root.join("compatdata/pfx/drive_c")),
            "module outside registered environment",
        )?;
        require(
            self.module.path.canonicalize()? == self.module.path,
            "module path traverses a symlink",
        )?;
        require(
            valid_hex(&self.host_source_sha256, 64),
            "host source identity syntax",
        )?;
        self.environment.runner.verify()?;
        self.module.verify()?;
        self.host.verify()?;
        self.native.verify()?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Publication {
    Pending,
    Published,
    Removed,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub registration: Registration,
    pub publication: Publication,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_revision: Option<publication::RevisionRef>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub schema: u32,
    pub revision: u64,
    pub classes: BTreeMap<String, Entry>,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            schema: 1,
            revision: 0,
            classes: BTreeMap::new(),
        }
    }
}
pub struct Manager {
    pub root: PathBuf,
    pub publications: PathBuf,
}
pub struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(std::os::fd::AsRawFd::as_raw_fd(&self.0), libc::LOCK_UN);
        }
    }
}
impl Manager {
    /// Must be called while holding registry.lock, the same lock as admission.
    pub fn require_inactive(&self, class: Option<&str>) -> Result<()> {
        let leases = self.root.join("runtime/leases");
        if !leases.try_exists()? {
            return Ok(());
        }
        for entry in fs::read_dir(leases)? {
            let path = entry?.path();
            let report: PathBuf = read_json(&path)?;
            require(
                report.parent() == Some(self.root.join("runtime/results").as_path()),
                "lease_identity",
            )?;
            let sid = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("lease_identity")?;
            require(valid_hex(sid, 32), "active_lease_unresolved")?;
            let mut owner = None;
            for env in fs::read_dir(self.root.join("environments"))? {
                let spec = env?
                    .path()
                    .join("compatdata/pfx/drive_c/bridge/sessions")
                    .join(sid)
                    .join("owner.json");
                if spec.try_exists()? {
                    require(owner.is_none(), "duplicate_lease_identity")?;
                    owner = Some(read_json::<serde_json::Value>(&spec)?);
                }
            }
            let owner = owner.ok_or("active_lease_unresolved")?;
            require(
                owner["session"].as_str() == Some(sid)
                    && owner["report"].as_str() == report.to_str(),
                "lease_identity",
            )?;
            if owner["keeper"] == true {
                require(
                    report
                        .file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.starts_with("environment-")),
                    "lease_identity",
                )?;
                continue;
            }
            let active = owner["registration"]["metadata"]["class_id"]
                .as_str()
                .ok_or("active_lease_unresolved")?;
            require(valid_hex(active, 32), "active_lease_unresolved")?;
            require(
                class.is_some_and(|c| !c.eq_ignore_ascii_case(active)),
                "active_device_lease",
            )?;
        }
        Ok(())
    }
    pub fn performance(&self, key: &str) -> Result<Performance> {
        require(valid_hex(key, 32), "class ID syntax")?;
        let path = self
            .root
            .join("performance")
            .join(format!("{}.json", key.to_uppercase()));
        let value = match fs::symlink_metadata(&path) {
            Ok(_) => read_json(&path)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Performance::default(),
            Err(e) => return Err(e.into()),
        };
        value.verify()?;
        Ok(value)
    }
    pub fn select_delay(&self, key: &str, frames: u32) -> Result<()> {
        let value = Performance {
            schema: 1,
            added_frames: frames,
        };
        value.verify()?;
        require(valid_hex(key, 32), "class ID syntax")?;
        // Admission holds this same lock until its lease is published. No
        // instance can race a preference change into its startup binding.
        let _lock = self.lock("registry.lock")?;
        let key = key.to_uppercase();
        require(
            self.registry()?.classes.contains_key(&key),
            "class not registered",
        )?;
        let leases = self.root.join("runtime/leases");
        if leases.try_exists()? {
            for entry in fs::read_dir(leases)? {
                let report: PathBuf = read_json(&entry?.path())?;
                require(
                    report
                        .file_name()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.starts_with("environment-")),
                    "close all bridged devices before changing delay; an instance lease remains",
                )?;
            }
        }
        private_dir(&self.root.join("performance"))?;
        atomic_json(
            &self.root.join("performance").join(format!("{key}.json")),
            &value,
        )
    }
    pub fn installed() -> Result<Self> {
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
        Ok(Self {
            root: home.join(".local/share/linux-vst-bridge/managed"),
            publications: home.join(".vst3"),
        })
    }
    pub fn lock(&self, name: &str) -> Result<Lock> {
        private_dir(&self.root)?;
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.root.join(name))?;
        require(
            f.metadata()?.uid() == unsafe { libc::getuid() } && f.metadata()?.is_file(),
            "lock ownership differs",
        )?;
        require(
            unsafe {
                libc::flock(
                    std::os::fd::AsRawFd::as_raw_fd(&f),
                    libc::LOCK_EX | libc::LOCK_NB,
                )
            } == 0,
            "operation already running",
        )?;
        Ok(Lock(f))
    }
    pub fn registry(&self) -> Result<Registry> {
        let p = self.root.join("registry.json");
        if !p.try_exists()? {
            return Ok(Registry::default());
        }
        let db: Registry = read_json(&p)?;
        require(db.schema == 1, "unsupported registry schema")?;
        Ok(db)
    }
    fn save(&self, r: &mut Registry) -> Result<()> {
        r.revision = r.revision.checked_add(1).ok_or("revision exhausted")?;
        atomic_json(&self.root.join("registry.json"), r)
    }
    pub fn link(&self, key: &str) -> PathBuf {
        self.publications.join(format!("LVB_{key}.vst3"))
    }
    pub fn target(&self, r: &Registration) -> PathBuf {
        self.root
            .join("publications")
            .join(r.key())
            .join(&r.native.sha256)
            .join(format!("LVB_{}.vst3", r.key()))
    }
    pub fn native_path(&self, r: &Registration) -> PathBuf {
        self.target(r)
            .join("Contents/x86_64-linux")
            .join(format!("LVB_{}.so", r.key()))
    }
    pub fn register(&self, r: Registration) -> Result<()> {
        let _lock = self.lock("registry.lock")?;
        r.verify(&self.root)?;
        let mut db = self.registry()?;
        let key = r.key();
        let mut installed = r.clone();
        installed.native.path = self.native_path(&r);
        if let Some(old) = db.classes.get(&key) {
            require(
                old.managed_revision.is_none(),
                "managed binding requires explicit publication transaction",
            )?;
            let mut previous = old.registration.clone();
            previous.native.path = self.native_path(&previous);
            require(
                previous == installed,
                "existing binding differs; explicit update transaction required",
            )?;
        }
        db.classes.insert(
            key.clone(),
            Entry {
                registration: r,
                publication: Publication::Pending,
                managed_revision: None,
            },
        );
        self.save(&mut db)?;
        self.publish_entry(&db.classes[&key].registration)?;
        let entry = db.classes.get_mut(&key).unwrap();
        entry.registration = installed;
        entry.publication = Publication::Published;
        self.save(&mut db)
    }
    fn publish_entry(&self, r: &Registration) -> Result<()> {
        let target = self.target(r);
        let so = target
            .join("Contents/x86_64-linux")
            .join(format!("LVB_{}.so", r.key()));
        if !target.try_exists()? {
            let parent = target.parent().ok_or("publication parent")?;
            private_dir(parent)?;
            let stage = parent.join(format!("stage-{}", random_id()?));
            private_dir(&stage)?;
            let dir = stage.join("Contents/x86_64-linux");
            private_dir(&dir)?;
            let copy = dir.join(so.file_name().unwrap());
            fs::copy(&r.native.path, &copy)?;
            require(
                digest(&copy)? == r.native.sha256,
                "publication copy changed",
            )?;
            fs::set_permissions(&copy, fs::Permissions::from_mode(0o500))?;
            File::open(&copy)?.sync_all()?;
            atomic_json(&stage.join("bridge-provenance.json"), r)?;
            fs::rename(stage, &target)?;
            File::open(parent)?.sync_all()?;
        }
        require(
            digest(&so)? == r.native.sha256,
            "installed publication changed",
        )?;
        fs::create_dir_all(&self.publications)?;
        let parent_meta = fs::symlink_metadata(&self.publications)?;
        require(
            parent_meta.is_dir() && parent_meta.uid() == unsafe { libc::getuid() },
            "publication directory ownership differs",
        )?;
        let link = self.link(&r.key());
        if fs::symlink_metadata(&link).is_ok() {
            require(
                fs::read_link(&link)? == target,
                "publication name occupied by another owner",
            )?;
            return Ok(());
        }
        let tmp = self.publications.join(format!(".lvb-{}", random_id()?));
        symlink(&target, &tmp)?;
        fs::rename(&tmp, &link)?;
        File::open(&self.publications)?.sync_all()?;
        Ok(())
    }
    pub fn reconcile(&self) -> Result<()> {
        let _lock = self.lock("registry.lock")?;
        let mut db = self.registry()?;
        self.reconcile_revisions(&mut db)?;
        let mut changed = false;
        for e in db.classes.values_mut() {
            if e.publication == Publication::Pending {
                let target = self.native_path(&e.registration);
                if target.try_exists()? {
                    e.registration.native.path = target.clone();
                }
                e.registration.verify(&self.root)?;
                self.publish_entry(&e.registration)?;
                e.registration.native.path = target;
                e.publication = Publication::Published;
                changed = true;
            }
        }
        if changed {
            self.save(&mut db)?;
        }
        drop(_lock);
        self.restore_editor_qualifications()
    }
    pub fn unpublish(&self, key: &str) -> Result<()> {
        require(valid_hex(key, 32), "class ID syntax")?;
        let key = key.to_uppercase();
        let _lock = self.lock("registry.lock")?;
        self.require_inactive(Some(&key))?;
        let mut db = self.registry()?;
        if db
            .classes
            .get(&key)
            .is_some_and(|e| e.managed_revision.is_some())
        {
            return self.remove_revision(&mut db, &key, None);
        }
        let entry = db.classes.get_mut(&key).ok_or("registration absent")?;
        let link = self.link(&key);
        let target = self.target(&entry.registration);
        if fs::symlink_metadata(&link).is_ok() {
            require(fs::read_link(&link)? == target, "publication owner differs")?;
            fs::remove_file(link)?;
            File::open(&self.publications)?.sync_all()?;
        }
        entry.publication = Publication::Removed;
        self.save(&mut db)
    }
    pub fn resolve(&self, identity: &[u8]) -> Result<Registration> {
        require(identity.len() == 48, "identity extent")?;
        let key = hex(&identity[..16]).to_uppercase();
        let db = self.registry()?;
        let e = db.classes.get(&key).ok_or("class not registered")?;
        require(
            !self.publication_pending(&key)?,
            "publication_recovery_pending",
        )?;
        require(
            e.publication == Publication::Published
                && e.registration.module.sha256 == hex(&identity[16..]),
            "mapping inactive or module binding differs",
        )?;
        e.registration.verify(&self.root)?;
        require(
            fs::read_link(self.link(&key))? == self.entry_target(e)?,
            "publication unavailable",
        )?;
        Ok(e.registration.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    #[test]
    fn installed_delay_is_inactive_versioned_and_separate_from_identity() {
        let f = Fixture::new();
        f.m.register(f.r.clone()).unwrap();
        let key = f.r.key();
        let before = f.m.resolve(&f.identity()).unwrap();
        assert_eq!(f.m.performance(&key).unwrap().added_frames, 512);
        f.m.select_delay(&key, 256).unwrap();
        assert_eq!(f.m.performance(&key).unwrap().added_frames, 256);
        assert_eq!(before, f.m.resolve(&f.identity()).unwrap());
        assert!(f.m.select_delay(&key, 128).is_err());
        private_dir(&f.m.root.join("runtime/leases")).unwrap();
        let lease = f.m.root.join("runtime/leases/active.json");
        atomic_json(
            &lease,
            &f.m.root.join("runtime/results/windows-active.json"),
        )
        .unwrap();
        assert!(f.m.select_delay(&key, 512).is_err());
        assert_eq!(f.m.performance(&key).unwrap().added_frames, 256);
        fs::remove_file(&lease).unwrap();
        // The environment keeper is not a DSP instance.
        atomic_json(
            &lease,
            &f.m.root.join("runtime/results/environment-keeper.json"),
        )
        .unwrap();
        f.m.select_delay(&key, 512).unwrap();
        assert_eq!(f.m.performance(&key).unwrap().added_frames, 512);
        let path = f.m.root.join("performance").join(format!("{key}.json"));
        atomic_json(
            &path,
            &Performance {
                schema: 2,
                added_frames: 256,
            },
        )
        .unwrap();
        assert!(f.m.performance(&key).is_err());
    }
    #[test]
    fn exact_metadata_and_roles() {
        let f = Fixture::new();
        assert!(f.r.metadata.verify().is_ok());
        for c in ["", "Fx|Instrument", "Instrumental"] {
            let mut r = f.r.clone();
            r.metadata.subcategories = c.into();
            assert!(r.metadata.verify().is_err());
        }
        let mut r = f.r.clone();
        r.metadata.metadata_tier = "factory_1".into();
        assert!(r.metadata.verify().is_err());
    }
    #[test]
    fn idempotent_publication_and_removal_preserve_vendor_files() {
        let f = Fixture::new();
        f.m.register(f.r.clone()).unwrap();
        let first = fs::read_link(f.m.link(&f.r.key())).unwrap();
        f.m.register(f.r.clone()).unwrap();
        assert_eq!(first, fs::read_link(f.m.link(&f.r.key())).unwrap());
        assert_eq!(fs::read_dir(&f.m.publications).unwrap().count(), 1);
        let resolved = f.m.resolve(&f.identity()).unwrap();
        assert_eq!(resolved.metadata, f.r.metadata);
        fs::remove_file(&f.r.native.path).unwrap();
        assert!(f.m.resolve(&f.identity()).is_ok());
        f.m.unpublish(&f.r.key()).unwrap();
        assert!(!f.m.link(&f.r.key()).exists());
        assert!(f.r.module.path.exists());
        assert!(f.m.resolve(&f.identity()).is_err());
    }
    #[test]
    fn changed_or_missing_artifacts_refuse_startup() {
        for which in 0..3 {
            let f = Fixture::new();
            f.m.register(f.r.clone()).unwrap();
            let p = match which {
                0 => &f.r.module.path,
                1 => &f.r.environment.runner.proton,
                _ => &f.m.native_path(&f.r),
            };
            fs::set_permissions(p, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(p, b"changed").unwrap();
            assert!(f.m.resolve(&f.identity()).is_err());
            fs::remove_file(p).unwrap();
            assert!(f.m.resolve(&f.identity()).is_err());
        }
    }
    #[test]
    fn interrupted_publication_recovers_and_never_overwrites_foreign_entry() {
        let f = Fixture::new();
        let mut db = Registry::default();
        db.classes.insert(
            f.r.key(),
            Entry {
                registration: f.r.clone(),
                publication: Publication::Pending,
                managed_revision: None,
            },
        );
        f.m.save(&mut db).unwrap();
        f.m.reconcile().unwrap();
        assert!(f.m.resolve(&f.identity()).is_ok());
        fs::remove_file(f.m.link(&f.r.key())).unwrap();
        fs::create_dir(f.m.link(&f.r.key())).unwrap();
        assert!(f.m.register(f.r.clone()).is_err());
        assert!(f.m.unpublish(&f.r.key()).is_err());
        assert!(f.m.link(&f.r.key()).is_dir());
    }
    #[test]
    fn duplicate_service_start_and_changed_binding_are_refused() {
        let f = Fixture::new();
        let lock = f.m.lock("service.lock").unwrap();
        assert!(f.m.lock("service.lock").is_err());
        drop(lock);
        assert!(f.m.lock("service.lock").is_ok());
        f.m.register(f.r.clone()).unwrap();
        let mut r = f.r.clone();
        r.metadata.name = "different".into();
        assert!(f.m.register(r).is_err());
    }
    #[test]
    fn removing_one_mapping_preserves_the_other() {
        let f = Fixture::new();
        f.m.register(f.r.clone()).unwrap();
        let mut other = f.r.clone();
        other.metadata.class_id = "02".repeat(16);
        other.metadata.name = "Effect".into();
        other.metadata.subcategories = "Fx|Delay".into();
        f.m.register(other.clone()).unwrap();
        f.m.unpublish(&f.r.key()).unwrap();
        assert_eq!(
            fs::read_link(f.m.link(&other.key())).unwrap(),
            f.m.target(&other)
        );
        assert_eq!(
            f.m.registry().unwrap().classes[&other.key()].publication,
            Publication::Published
        );
    }
}

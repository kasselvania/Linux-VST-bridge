//! User preset custody, separate from setup snapshots and diagnostic exports.
//! Bundled profiles qualify exact builds and file formats. No vendor is inferred
//! from a display name, and an unknown profile never prevents ordinary use.
use crate::{hex, require, valid_hex, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
mod configured;
mod fs;
#[cfg(test)]
mod tests;
const MAX_ENTRIES: usize = 2048;
const MAX_FILES: usize = 512;
const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_TOTAL: u64 = 256 * 1024 * 1024;

pub use lvb_model::operator_model::PresetProduct as Product;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Root {
    /// Exact profile-relative user-only directory beneath the setup root.
    Setup { relative: String },
    /// A qualified known-folder link in the setup plus a user-only suffix.
    /// Only this declared link may resolve outside the setup.
    KnownFolder { link: String, suffix: String },
    /// A single qualified JSON preference, interpreted as an absolute C: path.
    /// No other preference is retained; only a user-only child is eligible.
    ConfiguredJson {
        file: String,
        key: String,
        drive_c: String,
        suffix: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub module_sha256: String,
    pub class_id: String,
    pub root: Root,
    /// Qualified preset-only extensions. Never databases, archives or content.
    pub extensions: Vec<String>,
    /// An explicit compatibility group for byte-preserving maintenance recovery.
    pub compatibility: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema: u32,
    profiles: Vec<Profile>,
}
#[cfg(test)]
thread_local! {static TEST_PROFILES: std::cell::RefCell<Option<Vec<Profile>>> = const {std::cell::RefCell::new(None)};}
#[cfg(test)]
pub(crate) fn with_profiles<T>(profiles: Vec<Profile>, run: impl FnOnce() -> T) -> T {
    struct Reset(Option<Vec<Profile>>);
    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_PROFILES.with(|p| *p.borrow_mut() = self.0.take());
        }
    }
    let _reset = Reset(TEST_PROFILES.with(|p| p.replace(Some(profiles))));
    run()
}
pub fn profiles() -> Result<Vec<Profile>> {
    #[cfg(test)]
    if let Some(profiles) = TEST_PROFILES.with(|p| p.borrow().clone()) {
        return Ok(profiles);
    }
    let catalog: Catalog = serde_json::from_slice(include_bytes!(
        "../../../../compatibility/user-presets.json"
    ))?;
    require(
        catalog.schema == 1 && catalog.profiles.len() <= 128,
        "preset_profile_catalog",
    )?;
    for profile in &catalog.profiles {
        profile.validate()?;
    }
    Ok(catalog.profiles)
}
impl Profile {
    fn validate(&self) -> Result<()> {
        fs::component(&self.id)?;
        fs::component(&self.compatibility)?;
        require(
            valid_hex(&self.module_sha256, 64)
                && valid_hex(&self.class_id, 32)
                && !self.extensions.is_empty()
                && self.extensions.len() <= 8,
            "preset_profile_identity",
        )?;
        for ext in &self.extensions {
            require(
                !ext.is_empty()
                    && ext.len() <= 16
                    && ext.bytes().all(|b| b.is_ascii_alphanumeric())
                    && ![
                        "exe", "dll", "db", "sqlite", "json", "xml", "reg", "zip", "dat", "key",
                        "lic",
                    ]
                    .contains(&ext.to_ascii_lowercase().as_str()),
                "preset_profile_format",
            )?;
        }
        match &self.root {
            Root::Setup { relative } => {
                fs::relative(relative)?;
            }
            Root::KnownFolder { link, suffix } => {
                fs::relative(link)?;
                fs::relative(suffix)?;
            }
            Root::ConfiguredJson {
                file,
                key,
                drive_c,
                suffix,
            } => {
                fs::relative(file)?;
                fs::relative(drive_c)?;
                fs::relative(suffix)?;
                require(
                    !key.is_empty() && key.len() <= 128 && !key.chars().any(char::is_control),
                    "preset_preference_key",
                )?;
            }
        }
        Ok(())
    }
    fn accepts(&self, name: &str) -> bool {
        Path::new(name)
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|ext| {
                self.extensions
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(ext))
            })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KnownFolderSource {
    link: PathBuf,
    target: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub product: Product,
    pub profile: Profile,
    pub root: PathBuf,
    pub device: u64,
    pub inode: u64,
    pub external_to_setup: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured_source: Option<configured::Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_folder_source: Option<KnownFolderSource>,
}
impl Binding {
    pub fn observe(product: Product, setup: &Path, profile: &Profile) -> Result<Self> {
        profile.validate()?;
        require(
            product.module_sha256 == profile.module_sha256 && product.class_id == profile.class_id,
            "preset_profile_exact_build",
        )?;
        let mut configured_source = None;
        let mut known_folder_source = None;
        let root = match &profile.root {
            Root::Setup { relative } => setup.join(relative),
            Root::KnownFolder { link, suffix } => {
                let link_path = setup.join(link);
                fs::directory(link_path.parent().ok_or("preset_known_folder_parent")?)?;
                let meta = std::fs::symlink_metadata(&link_path)?;
                require(
                    meta.file_type().is_symlink() || meta.is_dir(),
                    "preset_known_folder_kind",
                )?;
                let target = link_path.canonicalize()?;
                let root = target.join(suffix);
                known_folder_source = Some(KnownFolderSource {
                    link: link_path,
                    target,
                });
                root
            }
            Root::ConfiguredJson {
                file,
                key,
                drive_c,
                suffix,
            } => {
                let source = configured::Source::observe(setup, file, key, drive_c, suffix)?;
                let root = source.root()?;
                configured_source = Some(source);
                root
            }
        };
        let directory = fs::directory(&root)?;
        let (device, inode) = fs::identity(&directory)?;
        Ok(Self {
            product,
            profile: profile.clone(),
            external_to_setup: !root.starts_with(setup),
            root,
            device,
            inode,
            configured_source,
            known_folder_source,
        })
    }
    pub fn token(&self) -> Result<String> {
        Ok(hash(&serde_json::to_vec(self)?))
    }
    pub fn revalidate(&self) -> Result<()> {
        self.open().map(|_| ())
    }
    fn open(&self) -> Result<File> {
        if let Some(source) = &self.known_folder_source {
            fs::directory(source.link.parent().ok_or("preset_known_folder_parent")?)?;
            require(
                source.link.canonicalize()? == source.target,
                "preset_known_folder_changed_refresh",
            )?;
        }
        if let Some(source) = &self.configured_source {
            source.revalidate()?;
            require(
                source.root()? == self.root,
                "preset_configured_root_changed_refresh",
            )?;
        }
        let dir = fs::directory(&self.root)?;
        require(
            fs::identity(&dir)? == (self.device, self.inode),
            "preset_root_replaced_refresh",
        )?;
        Ok(dir)
    }
}
fn hash(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub relative: String,
    pub bytes: u64,
    pub sha256: String,
    pub source_identity: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    pub schema: u32,
    pub source: Binding,
    pub operation: String,
    pub files: Vec<Entry>,
    pub total_bytes: u64,
}
impl Backup {
    pub fn digest(&self) -> Result<String> {
        Ok(hash(&serde_json::to_vec(self)?))
    }
}
fn walk(
    dir: &File,
    prefix: &str,
    profile: &Profile,
    entries: &mut usize,
    total: &mut u64,
    files: &mut Vec<Entry>,
) -> Result<()> {
    for name in fs::names(dir)? {
        *entries += 1;
        require(*entries <= MAX_ENTRIES, "preset_traversal_bound")?;
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        fs::relative(&relative)?;
        let file = fs::at(dir, &name, libc::O_RDONLY, 0)?;
        let meta = file.metadata()?;
        if meta.is_dir() {
            fs::identity(&file)?;
            walk(&file, &relative, profile, entries, total, files)?;
        } else if profile.accepts(&name) {
            let (bytes, source_identity) = fs::read_observed(dir, &name)?;
            *total += bytes.len() as u64;
            require(
                files.len() < MAX_FILES && *total <= MAX_TOTAL,
                "preset_backup_size_bound",
            )?;
            files.push(Entry {
                relative,
                bytes: bytes.len() as u64,
                sha256: hash(&bytes),
                source_identity,
            });
        } else {
            require(meta.is_file(), "preset_special_file_refused")?;
        }
    }
    Ok(())
}
pub fn inspect(binding: &Binding) -> Result<Vec<Entry>> {
    let dir = binding.open()?;
    let mut files = vec![];
    walk(&dir, "", &binding.profile, &mut 0, &mut 0, &mut files)?;
    binding.open()?;
    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(files)
}
#[cfg(test)]
thread_local! {pub(crate) static COPY_CHECKPOINT: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const {std::cell::RefCell::new(None)};}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicationBoundary {
    Prepared,
    Admitted,
    Published,
}
#[cfg(test)]
thread_local! {pub(crate) static PUBLICATION_CHECKPOINT: std::cell::RefCell<Option<Box<dyn FnMut(bool,PublicationBoundary)>>> = const {std::cell::RefCell::new(None)};}
#[cfg(test)]
fn publication_checkpoint(alias: bool, boundary: PublicationBoundary) {
    PUBLICATION_CHECKPOINT.with(|hook| {
        if let Some(run) = hook.borrow_mut().as_mut() {
            run(alias, boundary)
        }
    });
}
/// A complete backup is published only after source and copied bytes agree.
/// Partial directories have no manifest and are never recovery sources.
pub fn backup(storage: &Path, binding: &Binding, operation: &str) -> Result<Backup> {
    backup_with(storage, binding, operation, || Ok(()))
}
fn backup_with(
    storage: &Path,
    binding: &Binding,
    operation: &str,
    after_snapshot: impl FnOnce() -> Result<()>,
) -> Result<Backup> {
    require(valid_hex(operation, 32), "preset_operation_identity")?;
    let source = binding.open()?;
    let files = inspect(binding)?;
    let record = Backup {
        schema: 1,
        source: binding.clone(),
        operation: operation.into(),
        total_bytes: files.iter().map(|f| f.bytes).sum(),
        files,
    };
    let digest = record.digest()?;
    let store = fs::directory(storage)?;
    let pending_name = digest.clone();
    // Never reuse or delete an interrupted backup.
    let pending_c = std::ffi::CString::new(pending_name.as_str())?;
    use std::os::fd::AsRawFd;
    require(
        unsafe { libc::mkdirat(store.as_raw_fd(), pending_c.as_ptr(), 0o700) } == 0,
        "preset_backup_attempt_exists",
    )?;
    let pending = fs::at(&store, &pending_name, libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    let payload = fs::mkdir(&pending, "files")?;
    after_snapshot()?;
    #[cfg(test)]
    COPY_CHECKPOINT.with(|hook| {
        if let Some(run) = hook.borrow_mut().take() {
            run();
        }
    });
    for entry in &record.files {
        let (data, identity) = fs::read_observed(&source, &entry.relative)?;
        require(
            identity == entry.source_identity
                && data.len() as u64 == entry.bytes
                && hash(&data) == entry.sha256,
            "preset_source_changed",
        )?;
        fs::write_new(&payload, &entry.relative, &data)?;
        require(
            hash(&fs::read(&payload, &entry.relative)?) == entry.sha256,
            "preset_backup_copy_changed",
        )?;
    }
    require(inspect(binding)? == record.files, "preset_source_changed")?;
    fs::write_new(&pending, "manifest.json", &serde_json::to_vec(&record)?)?;
    fs::seal(&pending)?;
    store.sync_all()?;
    verify(storage, &digest)
}
pub fn verify(storage: &Path, digest: &str) -> Result<Backup> {
    require(valid_hex(digest, 64), "preset_backup_identity")?;
    let dir = fs::directory(&storage.join(digest))?;
    let data = fs::read(&dir, "manifest.json")?;
    require(
        data.len() <= 1024 * 1024 && hash(&data) == digest,
        "preset_backup_manifest_changed",
    )?;
    let backup: Backup = serde_json::from_slice(&data)?;
    backup.source.profile.validate()?;
    require(
        backup.source.product.module_sha256 == backup.source.profile.module_sha256
            && backup.source.product.class_id == backup.source.profile.class_id
            && crate::environment_revision::valid_environment_id(
                &backup.source.product.environment,
            )
            && valid_hex(&backup.operation, 32)
            && backup.source.root.is_absolute(),
        "preset_backup_source_binding",
    )?;
    require(
        backup.schema == 1 && backup.files.len() <= MAX_FILES && backup.total_bytes <= MAX_TOTAL,
        "preset_backup_extent",
    )?;
    let payload = fs::at(&dir, "files", libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0;
    for entry in &backup.files {
        fs::relative(&entry.relative)?;
        require(
            seen.insert(&entry.relative)
                && backup.source.profile.accepts(&entry.relative)
                && valid_hex(&entry.sha256, 64)
                && valid_hex(&entry.source_identity, 64),
            "preset_backup_file_binding",
        )?;
        let data = fs::read(&payload, &entry.relative)?;
        require(
            data.len() as u64 == entry.bytes && hash(&data) == entry.sha256,
            "preset_backup_bytes_changed",
        )?;
        total += entry.bytes;
    }
    require(total == backup.total_bytes, "preset_backup_total")?;
    Ok(backup)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Recovery {
    pub missing: Vec<Entry>,
    pub unchanged: usize,
    pub conflicts: usize,
}
pub fn recovery(binding: &Binding, backup: &Backup) -> Result<Recovery> {
    // Cross-build recovery additionally needs the operation's transition proof
    // at the owner boundary. This primitive never infers one from names.
    require(
        binding.product.environment == backup.source.product.environment
            && binding.product.class_id == backup.source.product.class_id
            && binding.root == backup.source.root
            && binding.profile.compatibility == backup.source.profile.compatibility,
        "preset_recovery_scope",
    )?;
    require(
        backup
            .files
            .iter()
            .all(|entry| binding.profile.accepts(&entry.relative)),
        "preset_recovery_format_not_qualified",
    )?;
    let current = inspect(binding)?;
    let mut result = Recovery::default();
    for entry in &backup.files {
        match current.iter().find(|f| f.relative == entry.relative) {
            None => result.missing.push(entry.clone()),
            Some(other) if other.bytes == entry.bytes && other.sha256 == entry.sha256 => {
                result.unchanged += 1
            }
            Some(_) => result.conflicts += 1,
        }
    }
    Ok(result)
}
pub fn restore_missing(
    storage: &Path,
    binding: &Binding,
    digest: &str,
    expected: &Recovery,
) -> Result<Recovery> {
    restore_missing_checked(storage, binding, digest, expected, || Ok(()))
}
pub fn restore_missing_checked<G>(
    storage: &Path,
    binding: &Binding,
    digest: &str,
    expected: &Recovery,
    mut admitted: impl FnMut() -> Result<G>,
) -> Result<Recovery> {
    let backup = verify(storage, digest)?;
    require(
        recovery(binding, &backup)? == *expected,
        "preset_recovery_offer_changed_refresh",
    )?;
    let source = fs::directory(&storage.join(digest).join("files"))?;
    let target = binding.open()?;
    for entry in &expected.missing {
        binding.open()?;
        let data = fs::read(&source, &entry.relative)?;
        require(hash(&data) == entry.sha256, "preset_backup_bytes_changed")?;
        fs::write_new_checked(&target, &entry.relative, &data, || {
            let admission = admitted()?;
            binding.revalidate()?;
            Ok(admission)
        })?;
        require(
            hash(&fs::read(&target, &entry.relative)?) == entry.sha256,
            "preset_restore_destination_changed",
        )?;
    }
    binding.open()?;
    recovery(binding, &backup)
}
pub fn import(binding: &Binding, name: &str, selected: &File) -> Result<()> {
    import_with(binding, name, selected, || Ok(()))
}
pub fn import_with<G>(
    binding: &Binding,
    name: &str,
    selected: &File,
    before_publish: impl FnOnce() -> Result<G>,
) -> Result<()> {
    fs::component(name)?;
    require(
        binding.profile.accepts(name),
        "preset_selected_format_not_qualified",
    )?;
    let identity = fs::stamp(&selected.metadata()?);
    let data = fs::bytes(selected)?;
    let target = binding.open()?;
    fs::write_new_checked(&target, name, &data, || {
        let admission = before_publish()?;
        require(
            fs::stamp(&selected.metadata()?) == identity,
            "preset_selected_source_changed",
        )?;
        binding.revalidate()?;
        Ok(admission)
    })?;
    binding.open()?;
    require(
        hash(&fs::read(&target, name)?) == hash(&data),
        "preset_import_verify",
    )
}
pub fn link(binding: &Binding, linux_parent: &Path, name: &str) -> Result<PathBuf> {
    link_checked(binding, linux_parent, name, || Ok(()))
}
pub fn link_checked<G>(
    binding: &Binding,
    linux_parent: &Path,
    name: &str,
    admitted: impl FnOnce() -> Result<G>,
) -> Result<PathBuf> {
    binding.open()?;
    let parent = fs::directory(linux_parent)?;
    fs::link(&parent, name, &binding.root, || {
        let admission = admitted()?;
        binding.revalidate()?;
        Ok(admission)
    })?;
    binding.open()?;
    Ok(linux_parent.join(name))
}
pub fn linux_parent(home: &Path) -> Result<PathBuf> {
    let home_dir = fs::directory(home)?;
    fs::identity(&home_dir)?;
    let music = fs::mkdir(&home_dir, "Music")?;
    fs::mkdir(&music, "Plug-in Presets")?;
    music.sync_all()?;
    home_dir.sync_all()?;
    Ok(home.join("Music/Plug-in Presets"))
}

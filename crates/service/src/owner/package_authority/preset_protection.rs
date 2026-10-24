//! Immutable package-owned preparation. This is not another operation journal
//! and does not authorize cross-build preset recovery.
use super::*;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

const MAX_BYTES: usize = 2 * 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Protection {
    schema: u32,
    transition: String,
    direction: TransitionKind,
    before: Software,
    after: Software,
    setups: Vec<operator_cli::presets::Protection>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    device: u64,
    inode: u64,
    digest: String,
    protection: Protection,
}
fn path(m: &Manager, successor: &Software, id: &str) -> Result<PathBuf> {
    require(
        id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "package_preset_transition_id",
    )?;
    Ok(receipt_directory(m, successor)?
        .parent()
        .ok_or("package_generation_path")?
        .join("preset-protection")
        .join(format!("{id}.json")))
}
fn digest(protection: &Protection) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(protection)?)))
}
fn verify_at(
    m: &Manager,
    path: &Path,
    id: &str,
    direction: TransitionKind,
    before: &Software,
    after: &Software,
    current: bool,
) -> Result<()> {
    let stored: Stored = serde_json::from_slice(&crate::read_control_bytes(path, MAX_BYTES)?)?;
    let meta = fs::symlink_metadata(path)?;
    require(
        meta.is_file()
            && meta.nlink() == 1
            && meta.uid() == unsafe { libc::geteuid() }
            && meta.mode() & 0o777 == 0o400
            && stored.device == meta.dev()
            && stored.inode == meta.ino(),
        "package_preset_sidecar_replaced",
    )?;
    let p = &stored.protection;
    require(
        p.schema == 1
            && p.transition == id
            && p.direction == direction
            && p.before == *before
            && p.after == *after
            && p.setups.len() <= 128
            && stored.digest == digest(p)?,
        "package_preset_sidecar_mismatch",
    )?;
    for setup in &p.setups {
        operator_cli::presets::verify_protection(m, setup, id)?;
        if current {
            operator_cli::presets::verify_package_sources(m, setup)?;
        }
    }
    require(
        crate::DigestFileIdentity::from(&fs::symlink_metadata(path)?)
            == crate::DigestFileIdentity::from(&meta),
        "package_preset_sidecar_replaced",
    )?;
    Ok(())
}
pub(super) fn prepare(
    m: &Manager,
    successor: &Software,
    id: &str,
    direction: TransitionKind,
    before: &Software,
    after: &Software,
) -> Result<()> {
    let path = path(m, successor, id)?;
    if fs::symlink_metadata(&path).is_ok() {
        return verify_at(m, &path, id, direction, before, after, true);
    }
    // Orphan preparation is harmless and never enters operator/ or the sole
    // package journal. A partial sidecar refuses; it is never silently replaced.
    let protection = Protection {
        schema: 1,
        transition: id.into(),
        direction,
        before: before.clone(),
        after: after.clone(),
        setups: operator_cli::presets::protect_package(m, id)?,
    };
    let directory = path.parent().ok_or("package_preset_parent")?;
    private_dir(directory)?;
    let mut held = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)?;
    let meta = held.metadata()?;
    let stored = Stored {
        device: meta.dev(),
        inode: meta.ino(),
        digest: digest(&protection)?,
        protection,
    };
    let bytes = serde_json::to_vec(&stored)?;
    require(bytes.len() <= MAX_BYTES, "package_preset_sidecar_bound")?;
    held.write_all(&bytes)?;
    held.sync_all()?;
    fs::File::open(directory)?.sync_all()?;
    verify_at(m, &path, id, direction, before, after, true)
}
pub(super) fn prepare_transition(m: &Manager, t: &CoordinatedTransition) -> Result<()> {
    prepare(
        m,
        &t.receipt.successor,
        &t.id,
        t.kind,
        &t.recovery_before,
        coordinated_final_software(t),
    )
}
pub(super) fn verify_required(m: &Manager, t: &CoordinatedTransition) -> Result<()> {
    let path = path(m, &t.receipt.successor, &t.id)?;
    verify_at(
        m,
        &path,
        &t.id,
        t.kind,
        &t.recovery_before,
        coordinated_final_software(t),
        true,
    )
}
/// The unchanged package journal cannot distinguish predecessor operation from
/// lost protection metadata. Absence is unknown, never evidence of protection;
/// it must not prevent bounded package recovery. Present invalid data refuses.
pub(super) fn verify(m: &Manager, t: &CoordinatedTransition) -> Result<()> {
    let path = path(m, &t.receipt.successor, &t.id)?;
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => verify_at(
            m,
            &path,
            &t.id,
            t.kind,
            &t.recovery_before,
            coordinated_final_software(t),
            false,
        ),
    }
}

/// Verification of payload bytes happens before global registry custody. This
/// bounded metadata witness rejoins the exact sidecar immediately before the
/// package owner rewrites its journal.
pub(super) struct Readback {
    path: PathBuf,
    identity: Option<crate::DigestFileIdentity>,
}
impl Readback {
    pub(super) fn recheck(&self) -> Result<()> {
        require(
            identity(&self.path)? == self.identity,
            "package_preset_sidecar_replaced",
        )
    }
}
fn identity(path: &Path) -> Result<Option<crate::DigestFileIdentity>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(crate::DigestFileIdentity::from(&meta))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
pub(super) fn readback(m: &Manager, t: &CoordinatedTransition) -> Result<Readback> {
    let path = path(m, &t.receipt.successor, &t.id)?;
    let witness = Readback {
        identity: identity(&path)?,
        path,
    };
    verify(m, t)?;
    witness.recheck()?;
    Ok(witness)
}

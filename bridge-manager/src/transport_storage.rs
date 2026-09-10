//! Session transport belongs to volatile memory, not the environment's journaled
//! filesystem. This owner runs during service/admission only, never in a DAW.
use linux_vst_bridge::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

const MARKER: &[u8] = b"linux-vst-bridge volatile transport v1\n";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemoryTransport {
    pub schema: u32,
    pub device: u64,
    pub inode: u64,
}
pub fn root() -> PathBuf {
    PathBuf::from(format!("/run/user/{}/linux-vst-bridge", unsafe {
        libc::getuid()
    }))
}
fn private(path: &Path) -> Result<fs::Metadata> {
    let m = fs::symlink_metadata(path)?;
    require(
        m.is_dir() && m.uid() == unsafe { libc::getuid() } && m.mode() & 0o077 == 0,
        "transport_directory_ownership",
    )?;
    require(path.canonicalize()? == path, "transport_directory_alias")?;
    Ok(m)
}
#[cfg(target_os = "linux")]
fn memory_filesystem(path: &Path) -> Result<()> {
    use std::os::fd::AsRawFd;
    let f = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(path)?;
    let mut s = std::mem::MaybeUninit::<libc::statfs>::uninit();
    require(
        unsafe { libc::fstatfs(f.as_raw_fd(), s.as_mut_ptr()) } == 0,
        "transport_statfs_failed",
    )?;
    require(
        unsafe { s.assume_init() }.f_type == libc::TMPFS_MAGIC,
        "transport_requires_tmpfs",
    )
}
#[cfg(not(target_os = "linux"))]
fn memory_filesystem(_path: &Path) -> Result<()> {
    Err("transport_requires_linux_tmpfs".into())
}

pub fn initialize() -> Result<()> {
    initialize_at(&root())
}
fn initialize_at(path: &Path) -> Result<()> {
    // Check before creating anything. No fallback to a disk-backed directory.
    memory_filesystem(path.parent().ok_or("transport_root_parent")?)?;
    let created = match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
        Err(e) => return Err(e.into()),
    };
    private(path)?;
    memory_filesystem(path)?;
    let marker = path.join("storage-v1");
    if created {
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&marker)?;
        f.write_all(MARKER)?;
    }
    let mut f = file(&marker)?;
    let m = f.metadata()?;
    require(
        m.mode() & 0o077 == 0 && m.len() == MARKER.len() as u64,
        "transport_root_foreign",
    )?;
    let mut bytes = [0u8; MARKER.len()];
    f.read_exact(&mut bytes)?;
    require(bytes == MARKER, "transport_root_foreign")
}
use std::os::unix::fs::DirBuilderExt;

pub fn create(session: &str) -> Result<(PathBuf, MemoryTransport)> {
    create_at(&root(), session)
}
fn create_at(root: &Path, session: &str) -> Result<(PathBuf, MemoryTransport)> {
    require(
        session.len() == 32
            && session
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)),
        "transport_session_identity",
    )?;
    initialize_at(root)?;
    let path = root.join(session);
    fs::DirBuilder::new().mode(0o700).create(&path)?; // never adopt a prior/foreign session
    let m = private(&path)?;
    Ok((
        path,
        MemoryTransport {
            schema: 1,
            device: m.dev(),
            inode: m.ino(),
        },
    ))
}

/// SO_PEERCRED identifies the actual connecting DAW process in this manager's
/// PID namespace. Refuse a hidden/mismatched mount before returning its binding.
#[cfg(target_os = "linux")]
pub fn visible_to_peer(peer: &std::os::unix::net::UnixStream, directory: &Path) -> Result<()> {
    use std::os::fd::AsRawFd;
    let mut cred = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    require(
        unsafe {
            libc::getsockopt(
                peer.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                cred.as_mut_ptr().cast(),
                &mut length,
            )
        } == 0
            && length as usize == std::mem::size_of::<libc::ucred>(),
        "transport_peer_credentials",
    )?;
    let cred = unsafe { cred.assume_init() };
    require(
        cred.uid == unsafe { libc::getuid() } && cred.pid > 0,
        "transport_peer_owner",
    )?;
    let visible =
        PathBuf::from(format!("/proc/{}/root", cred.pid)).join(directory.strip_prefix("/")?);
    same_directory(directory, &visible)
}
#[cfg(not(target_os = "linux"))]
pub fn visible_to_peer(_peer: &std::os::unix::net::UnixStream, _directory: &Path) -> Result<()> {
    Err("transport_requires_linux_tmpfs".into())
}
#[cfg(any(target_os = "linux", test))]
fn same_directory(host: &Path, peer: &Path) -> Result<()> {
    let a = private(host)?;
    let b = fs::symlink_metadata(peer)?;
    require(
        b.is_dir()
            && b.uid() == a.uid()
            && b.mode() & 0o077 == 0
            && (a.dev(), a.ino()) == (b.dev(), b.ino()),
        "transport_not_visible_to_daw",
    )
}

/// Admission owns an empty memory directory until its binding is exposed. Once
/// exposed only the supervisor's positive native/Windows retirement may remove
/// it, including when the later launcher fails.
pub struct PendingTransport {
    pub directory: PathBuf,
    pub identity: MemoryTransport,
    exposed: bool,
}
impl PendingTransport {
    pub fn new(session: &str) -> Result<Self> {
        let (directory, identity) = create(session)?;
        Ok(Self {
            directory,
            identity,
            exposed: false,
        })
    }
    pub fn expose(&mut self) {
        self.exposed = true;
    }
}
impl Drop for PendingTransport {
    fn drop(&mut self) {
        if !self.exposed {
            if let Ok(m) = private(&self.directory) {
                if (m.dev(), m.ino()) == (self.identity.device, self.identity.inode) {
                    let _ = fs::remove_dir(&self.directory); // empty, never recursive
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hidden_mount_is_not_a_shared_transport() {
        let a = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("ap16-visible-{}", random_id().unwrap()));
        private_dir(&a).unwrap();
        private_dir(&a.join("other")).unwrap();
        assert!(same_directory(&a, &a).is_ok());
        assert!(same_directory(&a, &a.join("other")).is_err());
        fs::remove_dir_all(a).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn memory_storage_is_private_exact_and_never_adopts_foreign_sessions() {
        let r = PathBuf::from("/dev/shm").join(format!("ap16-storage-{}", random_id().unwrap()));
        initialize_at(&r).unwrap();
        let sid = "ab".repeat(16);
        let (p, id) = create_at(&r, &sid).unwrap();
        assert_eq!(id.schema, 1);
        assert_eq!(id.inode, fs::metadata(&p).unwrap().ino());
        memory_filesystem(&p).unwrap();
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        visible_to_peer(&a, &p).unwrap();
        assert!(create_at(&r, &sid).is_err());
        for invalid in ["../escape", "", &"AB".repeat(16)] {
            assert!(create_at(&r, invalid).is_err());
        }
        fs::write(r.join("storage-v1"), b"foreign").unwrap();
        assert!(create_at(&r, &"cd".repeat(16)).is_err());
        assert!(p.exists());
        fs::remove_dir_all(r).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn unexposed_memory_is_removed_but_exposed_ownership_is_retained() {
        let r = PathBuf::from("/dev/shm").join(format!("ap16-guard-{}", random_id().unwrap()));
        initialize_at(&r).unwrap();
        let (directory, identity) = create_at(&r, &"de".repeat(16)).unwrap();
        drop(PendingTransport {
            directory: directory.clone(),
            identity,
            exposed: false,
        });
        assert!(!directory.exists());
        let (directory, identity) = create_at(&r, &"de".repeat(16)).unwrap();
        let mut guard = PendingTransport {
            directory: directory.clone(),
            identity,
            exposed: false,
        };
        guard.expose();
        drop(guard);
        assert!(directory.exists());
        fs::remove_dir_all(r).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn journaled_storage_is_refused_before_directory_creation() {
        // Linux CI checkout is disk/overlay; it must never be an IPC fallback.
        let r = std::env::current_dir()
            .unwrap()
            .join(format!("ap16-disk-{}", random_id().unwrap()));
        if memory_filesystem(r.parent().unwrap()).is_ok() {
            return;
        }
        assert!(initialize_at(&r)
            .unwrap_err()
            .to_string()
            .contains("transport_requires_tmpfs"));
        assert!(!r.exists());
    }
}

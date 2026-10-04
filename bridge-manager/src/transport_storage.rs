//! Session transport belongs to volatile memory, not the environment's journaled
//! filesystem. This owner runs during service/admission only, never in a DAW.
use crate::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    os::unix::{
        ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
};

const MARKER: &[u8] = b"linux-vst-bridge volatile transport v1\n";
const GRAPHICAL_DENIAL_SOCKETS: [&str; 2] = [".lvb-denied-dbus", ".lvb-denied-wayland"];
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemoryTransport {
    pub schema: u32,
    pub device: u64,
    pub inode: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GraphicalSession {
    pub schema: u32,
    pub peer_pid: i32,
    pub peer_start_ticks: u64,
    pub display: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wayland_display: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xauthority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dbus_session_bus_address: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerProcess {
    pub pid: i32,
    pub uid: u32,
    pub start_ticks: u64,
}

impl GraphicalSession {
    pub fn same_display_context(&self, other: &Self) -> bool {
        self.display == other.display
            && self.wayland_display == other.wayland_display
            && self.xauthority == other.xauthority
            && self.dbus_session_bus_address == other.dbus_session_bus_address
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_graphical_environment(
    peer_pid: i32,
    peer_start_ticks: u64,
    bytes: &[u8],
) -> Result<GraphicalSession> {
    require(
        peer_pid > 0 && peer_start_ticks > 0,
        "graphical_peer_generation",
    )?;
    require(
        bytes.len() <= 1024 * 1024,
        "graphical_peer_environment_extent",
    )?;
    let mut selected = std::collections::BTreeMap::new();
    for item in bytes
        .split(|byte| *byte == 0)
        .filter(|item| !item.is_empty())
    {
        let text = std::str::from_utf8(item)?;
        let Some((key, value)) = text.split_once('=') else {
            continue;
        };
        if matches!(
            key,
            "DISPLAY" | "WAYLAND_DISPLAY" | "XAUTHORITY" | "DBUS_SESSION_BUS_ADDRESS"
        ) {
            require(
                !value.is_empty()
                    && value.len() <= 4096
                    && !value.bytes().any(|byte| byte == b'\n' || byte == b'\r')
                    && selected.insert(key, value.to_owned()).is_none(),
                "graphical_peer_environment",
            )?;
        }
    }
    let display = selected
        .remove("DISPLAY")
        .ok_or("graphical_peer_display_absent")?;
    Ok(GraphicalSession {
        schema: 1,
        peer_pid,
        peer_start_ticks,
        display,
        wayland_display: selected.remove("WAYLAND_DISPLAY"),
        xauthority: selected.remove("XAUTHORITY"),
        dbus_session_bus_address: selected.remove("DBUS_SESSION_BUS_ADDRESS"),
    })
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

fn exact_session_directory(
    root: &Path,
    session: &str,
    identity: &MemoryTransport,
    require_memory: bool,
) -> Result<PathBuf> {
    require(
        session.len() == 32
            && session
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            && identity.schema == 1
            && identity.device != 0
            && identity.inode != 0,
        "transport_session_identity",
    )?;
    let path = root.join(session);
    let metadata = private(&path)?;
    require(
        (metadata.dev(), metadata.ino()) == (identity.device, identity.inode),
        "transport_directory_replaced",
    )?;
    if require_memory {
        memory_filesystem(&path)?;
    }
    Ok(path)
}

/// Resolve only the session directory retained by the exact admission record.
/// Callers must still open individual files with O_NOFOLLOW.
pub fn session_directory(session: &str, identity: &MemoryTransport) -> Result<PathBuf> {
    exact_session_directory(&root(), session, identity, true)
}

#[cfg(test)]
pub(crate) fn fixture_session_directory(
    root: &Path,
    session: &str,
    identity: &MemoryTransport,
) -> Result<PathBuf> {
    exact_session_directory(root, session, identity, false)
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
    require(bytes == MARKER, "transport_root_foreign")?;
    for name in GRAPHICAL_DENIAL_SOCKETS {
        initialize_graphical_denial(path, name)?;
    }
    Ok(())
}
use std::os::unix::fs::DirBuilderExt;

fn initialize_graphical_denial(root: &Path, name: &str) -> Result<()> {
    initialize_graphical_denial_with(root, name, || {})
}

fn initialize_graphical_denial_with(root: &Path, name: &str, bound: impl FnOnce()) -> Result<()> {
    let path = root.join(name);
    match fs::symlink_metadata(&path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // A listening descriptor can survive in a concurrently forked
            // child until that child execs, even when it has FD_CLOEXEC. The
            // denial inode must therefore never enter the listening state.
            #[cfg(target_os = "linux")]
            let socket_type = libc::SOCK_STREAM | libc::SOCK_CLOEXEC;
            #[cfg(not(target_os = "linux"))]
            let socket_type = libc::SOCK_STREAM;
            let raw = unsafe { libc::socket(libc::AF_UNIX, socket_type, 0) };
            if raw < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let socket = unsafe { OwnedFd::from_raw_fd(raw) };
            #[cfg(target_os = "macos")]
            if unsafe { libc::fcntl(socket.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let bytes = path.as_os_str().as_bytes();
            let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
            require(!bytes.contains(&0) && bytes.len() < address.sun_path.len(),
                "graphical_denial_path")?;
            address.sun_family = libc::AF_UNIX as libc::sa_family_t;
            for (target, source) in address.sun_path.iter_mut().zip(bytes) {
                *target = *source as libc::c_char;
            }
            let length = std::mem::offset_of!(libc::sockaddr_un, sun_path)
                .checked_add(bytes.len() + 1).ok_or("graphical_denial_path")?;
            #[cfg(target_os = "macos")]
            {
                address.sun_len = length.try_into()?;
            }
            if unsafe { libc::bind(socket.as_raw_fd(),
                std::ptr::from_ref(&address).cast::<libc::sockaddr>(), length.try_into()?) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
            bound();
            drop(socket); // retain a bound stream inode that was never listening
        }
        Err(error) => return Err(error.into()),
    }
    verify_graphical_denial(root, name)
}

fn verify_graphical_denial(root: &Path, name: &str) -> Result<()> {
    let path = root.join(name);
    let before = fs::symlink_metadata(&path)?;
    require(
        before.file_type().is_socket()
            && before.uid() == unsafe { libc::getuid() }
            && before.mode() & 0o077 == 0,
        "graphical_denial_ownership",
    )?;
    match UnixStream::connect(&path) {
        Ok(stream) => {
            drop(stream);
            return Err("graphical_denial_listening".into());
        }
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {}
        Err(error) => return Err(error.into()),
    }
    let after = fs::symlink_metadata(&path)?;
    require(
        after.file_type().is_socket()
            && after.uid() == unsafe { libc::getuid() }
            && after.mode() & 0o077 == 0
            && (
                before.dev(),
                before.ino(),
                before.ctime(),
                before.ctime_nsec(),
            ) == (after.dev(), after.ino(), after.ctime(), after.ctime_nsec()),
        "graphical_denial_replaced",
    )
}

/// A package transition may proceed only when the volatile transport root
/// contains its exact persistent marker and denial sockets, with no session.
/// This inspection never initializes or repairs the root.
pub fn require_no_sessions() -> Result<()> {
    let path = root();
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => require_no_sessions_at(&path),
    }
}

fn require_no_sessions_at(path: &Path) -> Result<()> {
    let before = private(path)?;
    memory_filesystem(path)?;
    let marker = path.join("storage-v1");
    let mut source = file(&marker)?;
    let first = source.metadata()?;
    require(first.mode() & 0o077 == 0 && first.len() == MARKER.len() as u64,
        "transport_root_foreign")?;
    let mut bytes = [0u8; MARKER.len()];
    source.read_exact(&mut bytes)?;
    let last = source.metadata()?;
    let named = fs::symlink_metadata(&marker)?;
    require(bytes == MARKER && named.is_file() && !named.file_type().is_symlink()
        && (first.dev(), first.ino(), first.len(), first.mtime(), first.mtime_nsec(),
            first.ctime(), first.ctime_nsec())
            == (last.dev(), last.ino(), last.len(), last.mtime(), last.mtime_nsec(),
                last.ctime(), last.ctime_nsec())
        && (first.dev(), first.ino()) == (named.dev(), named.ino()),
        "transport_root_foreign")?;
    for name in GRAPHICAL_DENIAL_SOCKETS {
        verify_graphical_denial(path, name)?;
    }
    let mut count = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        count += 1;
        require(count <= 3 && matches!(entry.file_name().to_str(),
            Some("storage-v1" | ".lvb-denied-dbus" | ".lvb-denied-wayland")),
            "package_stale_transport")?;
    }
    require(count == 3, "transport_root_foreign")?;
    let after = private(path)?;
    require((before.dev(), before.ino()) == (after.dev(), after.ino()),
        "transport_directory_replaced")
}

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
fn peer_credentials(peer: &std::os::unix::net::UnixStream) -> Result<libc::ucred> {
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
    Ok(cred)
}
#[cfg(target_os = "linux")]
fn process_start_ticks(pid: i32) -> Result<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let (_, fields) = stat.rsplit_once(") ").ok_or("peer_process_stat")?;
    let start = fields
        .split_ascii_whitespace()
        .nth(19)
        .ok_or("peer_process_stat")?
        .parse()?;
    require(start > 0, "peer_process_generation")?;
    Ok(start)
}
#[cfg(target_os = "linux")]
impl PeerProcess {
    fn verify(&self, peer: &std::os::unix::net::UnixStream) -> Result<()> {
        let credentials = peer_credentials(peer)?;
        require(
            credentials.pid == self.pid
                && credentials.uid == self.uid
                && process_start_ticks(self.pid)? == self.start_ticks,
            "peer_process_generation_changed",
        )
    }
}
#[cfg(target_os = "linux")]
pub fn peer_process(peer: &std::os::unix::net::UnixStream) -> Result<PeerProcess> {
    let credentials = peer_credentials(peer)?;
    let process = PeerProcess {
        pid: credentials.pid,
        uid: credentials.uid,
        start_ticks: process_start_ticks(credentials.pid)?,
    };
    process.verify(peer)?;
    Ok(process)
}
#[cfg(any(target_os = "linux", test))]
fn maps_artifact(bytes: &[u8], metadata: &fs::Metadata) -> Result<bool> {
    for line in std::str::from_utf8(bytes)?.lines() {
        let fields: Vec<_> = line.split_whitespace().take(5).collect();
        require(fields.len() == 5, "native_caller_maps_format")?;
        let (major, minor) = fields[3].split_once(':').ok_or("native_caller_maps_device")?;
        if fields[1].starts_with("r-x")
            && fields[4].parse::<u64>()? == metadata.ino()
            && u64::from_str_radix(major, 16)? == libc::major(metadata.dev() as libc::dev_t) as u64
            && u64::from_str_radix(minor, 16)? == libc::minor(metadata.dev() as libc::dev_t) as u64 {
            return Ok(true);
        }
    }
    Ok(false)
}
#[cfg(target_os = "linux")]
pub fn peer_maps_artifact(
    peer: &std::os::unix::net::UnixStream,
    process: &PeerProcess,
    artifact: &Path,
) -> Result<()> {
    process.verify(peer)?;
    let file = crate::file(artifact)?;
    let metadata = file.metadata()?;
    require(
        metadata.is_file() && metadata.uid() == process.uid,
        "native_caller_artifact_owner",
    )?;
    let mut bytes = Vec::new();
    fs::File::open(format!("/proc/{}/maps", process.pid))?.take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    require(bytes.len() <= 2 * 1024 * 1024, "native_caller_maps_extent")?;
    require(maps_artifact(&bytes, &metadata)?, "native_caller_image_not_mapped")?;
    process.verify(peer)
}
#[cfg(not(target_os = "linux"))]
pub fn peer_process(_peer: &std::os::unix::net::UnixStream) -> Result<PeerProcess> {
    Err("peer_process_requires_linux".into())
}
#[cfg(not(target_os = "linux"))]
pub fn peer_maps_artifact(
    _peer: &std::os::unix::net::UnixStream,
    _process: &PeerProcess,
    _artifact: &Path,
) -> Result<()> {
    Err("native_caller_mapping_requires_linux".into())
}
#[cfg(target_os = "linux")]
pub fn visible_to_peer(
    peer: &std::os::unix::net::UnixStream,
    process: &PeerProcess,
    directory: &Path,
) -> Result<()> {
    process.verify(peer)?;
    let visible =
        PathBuf::from(format!("/proc/{}/root", process.pid)).join(directory.strip_prefix("/")?);
    same_directory(directory, &visible)?;
    process.verify(peer)
}
#[cfg(target_os = "linux")]
pub fn graphical_session(
    peer: &std::os::unix::net::UnixStream,
    process: &PeerProcess,
) -> Result<GraphicalSession> {
    process.verify(peer)?;
    let environment = fs::read(format!("/proc/{}/environ", process.pid))?;
    process.verify(peer)?;
    parse_graphical_environment(process.pid, process.start_ticks, &environment)
}
#[cfg(not(target_os = "linux"))]
pub fn visible_to_peer(
    _peer: &std::os::unix::net::UnixStream,
    _process: &PeerProcess,
    _directory: &Path,
) -> Result<()> {
    Err("transport_requires_linux_tmpfs".into())
}
#[cfg(not(target_os = "linux"))]
pub fn graphical_session(
    _peer: &std::os::unix::net::UnixStream,
    _process: &PeerProcess,
) -> Result<GraphicalSession> {
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
    #[cfg(target_os = "linux")]
    use std::os::unix::net::UnixListener;

    #[cfg(target_os = "linux")]
    struct ForkedChild {
        pid: libc::pid_t,
        release: OwnedFd,
    }
    #[cfg(target_os = "linux")]
    impl ForkedChild {
        fn wait_bounded(&mut self) -> Option<i32> {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                let mut status = 0;
                let result = unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) };
                if result == self.pid {
                    self.pid = -1;
                    return Some(status);
                }
                if result < 0 && std::io::Error::last_os_error().kind()
                    != std::io::ErrorKind::Interrupted {
                    return None;
                }
                if std::time::Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        fn release_and_wait(mut self) -> i32 {
            let byte = [1u8];
            assert_eq!(unsafe { libc::write(self.release.as_raw_fd(), byte.as_ptr().cast(), 1) }, 1);
            self.wait_bounded().expect("forked child did not exit")
        }
    }
    #[cfg(target_os = "linux")]
    impl Drop for ForkedChild {
        fn drop(&mut self) {
            if self.pid <= 0 {
                return;
            }
            unsafe { libc::kill(self.pid, libc::SIGKILL); }
            let mut status = 0;
            loop {
                let result = unsafe { libc::waitpid(self.pid, &mut status, 0) };
                if result == self.pid || (result < 0
                    && std::io::Error::last_os_error().kind()
                        != std::io::ErrorKind::Interrupted) {
                    break;
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    struct TestDirectory(PathBuf);
    #[cfg(target_os = "linux")]
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn graphical_context_uses_only_the_authenticated_peer_allowlist() {
        let context = parse_graphical_environment(
            41,
            9001,
            b"DISPLAY=:7\0WAYLAND_DISPLAY=gamescope-1\0XAUTHORITY=/run/user/1000/xauth\0DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus\0HOME=/secret\0TOKEN=private\0",
        )
        .unwrap();
        assert_eq!(context.display, ":7");
        assert_eq!(context.wayland_display.as_deref(), Some("gamescope-1"));
        assert_eq!(context.peer_pid, 41);
        assert_eq!(context.peer_start_ticks, 9001);
        let value = serde_json::to_value(context).unwrap();
        assert!(value.get("HOME").is_none() && value.get("TOKEN").is_none());
        assert!(parse_graphical_environment(41, 9001, b"WAYLAND_DISPLAY=x\0").is_err());
    }
    #[test]
    fn hidden_mount_is_not_a_shared_transport() {
        let a = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("ap16-visible-{}", random_id().unwrap()));
        fs::DirBuilder::new().mode(0o700).create(&a).unwrap();
        private_dir(&a).unwrap();
        let other = a.join("other");
        fs::DirBuilder::new().mode(0o700).create(&other).unwrap();
        private_dir(&other).unwrap();
        assert!(same_directory(&a, &a).is_ok());
        assert!(same_directory(&a, &other).is_err());
        fs::remove_dir_all(a).unwrap();
    }
    #[test]
    fn mapped_native_identity_uses_device_and_inode_not_path_text() {
        let path = std::env::temp_dir().join(format!("mapped-native-{}", random_id().unwrap()));
        fs::write(&path, b"native image").unwrap();
        let metadata = fs::metadata(&path).unwrap();
        let matching = format!("1000-2000 r-xp 0 {:x}:{:x} {} /different/mount/name\n",
            libc::major(metadata.dev() as libc::dev_t), libc::minor(metadata.dev() as libc::dev_t), metadata.ino());
        assert!(maps_artifact(matching.as_bytes(), &metadata).unwrap());
        let readonly = matching.replacen("r-xp", "r--p", 1);
        assert!(!maps_artifact(readonly.as_bytes(), &metadata).unwrap());
        let wrong = format!("1000-2000 r-xp 0 {:x}:{:x} {} {}\n",
            libc::major(metadata.dev() as libc::dev_t), libc::minor(metadata.dev() as libc::dev_t),
            metadata.ino()+1, path.display());
        assert!(!maps_artifact(wrong.as_bytes(), &metadata).unwrap());
        assert!(maps_artifact(b"malformed\n", &metadata).is_err());
        fs::remove_file(path).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn memory_storage_is_private_exact_and_never_adopts_foreign_sessions() {
        let r = PathBuf::from("/dev/shm").join(format!("ap16-storage-{}", random_id().unwrap()));
        initialize_at(&r).unwrap();
        for name in GRAPHICAL_DENIAL_SOCKETS {
            let denial = r.join(name);
            let metadata = fs::symlink_metadata(&denial).unwrap();
            assert!(metadata.file_type().is_socket());
            assert_eq!(metadata.mode() & 0o077, 0);
            assert_eq!(
                UnixStream::connect(&denial).unwrap_err().kind(),
                std::io::ErrorKind::ConnectionRefused
            );
        }
        let sid = "ab".repeat(16);
        let (p, id) = create_at(&r, &sid).unwrap();
        assert_eq!(id.schema, 1);
        assert_eq!(id.inode, fs::metadata(&p).unwrap().ino());
        memory_filesystem(&p).unwrap();
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let process = peer_process(&a).unwrap();
        visible_to_peer(&a, &process, &p).unwrap();
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
    fn package_idle_readback_accepts_only_exact_static_transport_entries() {
        let r = PathBuf::from("/dev/shm")
            .join(format!("package-transport-{}", random_id().unwrap()));
        require_no_sessions_at(&r).unwrap_err();
        assert!(!r.exists());
        initialize_at(&r).unwrap();
        require_no_sessions_at(&r).unwrap();
        let session = "cd".repeat(16);
        let (directory, _) = create_at(&r, &session).unwrap();
        assert!(require_no_sessions_at(&r).unwrap_err().to_string()
            .contains("package_stale_transport"));
        fs::remove_dir(&directory).unwrap();
        require_no_sessions_at(&r).unwrap();
        fs::write(r.join("storage-v1"), b"foreign").unwrap();
        assert!(require_no_sessions_at(&r).is_err());
        fs::write(r.join("storage-v1"), MARKER).unwrap();
        fs::remove_file(r.join(GRAPHICAL_DENIAL_SOCKETS[0])).unwrap();
        assert!(require_no_sessions_at(&r).is_err());
        assert!(!r.join(GRAPHICAL_DENIAL_SOCKETS[0]).exists());
        fs::remove_dir_all(r).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn graphical_denials_refuse_foreign_listeners_types_permissions_and_aliases() {
        for case in ["listening", "regular", "public", "symlink"] {
            let r = PathBuf::from("/dev/shm")
                .join(format!("graphical-denial-{case}-{}", random_id().unwrap()));
            initialize_at(&r).unwrap();
            let path = r.join(GRAPHICAL_DENIAL_SOCKETS[0]);
            fs::remove_file(&path).unwrap();
            let mut listener = None;
            match case {
                "listening" => listener = Some(UnixListener::bind(&path).unwrap()),
                "regular" => fs::write(&path, b"foreign").unwrap(),
                "public" => {
                    let socket = UnixListener::bind(&path).unwrap();
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
                    drop(socket);
                }
                "symlink" => std::os::unix::fs::symlink(r.join("storage-v1"), &path).unwrap(),
                _ => unreachable!(),
            }
            assert!(initialize_at(&r).is_err(), "accepted {case} denial object");
            drop(listener);
            fs::remove_dir_all(r).unwrap();
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn forked_pre_exec_child_cannot_keep_a_denial_socket_listening() {
        let r = TestDirectory(PathBuf::from("/dev/shm")
            .join(format!("graphical-denial-fork-{}", random_id().unwrap())));
        fs::DirBuilder::new().mode(0o700).create(&r.0).unwrap();
        let mut ready = [-1; 2];
        let mut release = [-1; 2];
        assert_eq!(unsafe { libc::pipe2(ready.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
        assert_eq!(unsafe { libc::pipe2(release.as_mut_ptr(), libc::O_CLOEXEC) }, 0);
        let ready_read = unsafe { OwnedFd::from_raw_fd(ready[0]) };
        let ready_write = unsafe { OwnedFd::from_raw_fd(ready[1]) };
        let release_read = unsafe { OwnedFd::from_raw_fd(release[0]) };
        let release_write = unsafe { OwnedFd::from_raw_fd(release[1]) };
        let mut child = None;
        let result = initialize_graphical_denial_with(&r.0, GRAPHICAL_DENIAL_SOCKETS[0], || {
            let pid = unsafe { libc::fork() };
            if pid == 0 {
                unsafe {
                    libc::close(ready_read.as_raw_fd());
                    libc::close(release_write.as_raw_fd());
                    let byte = [1u8];
                    let _ = libc::write(ready_write.as_raw_fd(), byte.as_ptr().cast(), 1);
                    libc::close(ready_write.as_raw_fd());
                    let mut release_byte = 0u8;
                    let _ = libc::read(release_read.as_raw_fd(),
                        std::ptr::from_mut(&mut release_byte).cast(), 1);
                    libc::_exit(0);
                }
            }
            assert!(pid > 0);
            drop(ready_write);
            drop(release_read);
            child = Some(ForkedChild { pid, release: release_write });
            let mut pollfd = libc::pollfd {
                fd: ready_read.as_raw_fd(), events: libc::POLLIN, revents: 0
            };
            assert_eq!(unsafe { libc::poll(&mut pollfd, 1, 5_000) }, 1);
            let mut byte = 0u8;
            assert_eq!(unsafe { libc::read(ready_read.as_raw_fd(),
                std::ptr::from_mut(&mut byte).cast(), 1) }, 1);
        });
        result.unwrap();
        let status = child.take().unwrap().release_and_wait();
        assert!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0);
        verify_graphical_denial(&r.0, GRAPHICAL_DENIAL_SOCKETS[0]).unwrap();
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

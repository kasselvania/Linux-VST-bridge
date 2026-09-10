//! Per-instance preview discovery. Never called from the audio callback.
//! The owner launches only the pinned reference host; it does not launch a DAW.
use crate::{invalid, need};
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{
        fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

unsafe extern "C" {
    fn getuid() -> u32;
}

/// Best-effort terminal worker diagnostic, before its owned stage is retired.
/// The SDK creates the private sink; never follow a replacement symlink.
pub fn append_report(path: &Path, bytes: &[u8]) {
    #[cfg(target_os = "linux")]
    const NOFOLLOW: i32 = 0x20000;
    #[cfg(target_os = "macos")]
    const NOFOLLOW: i32 = 0x100;
    let Ok(mut file) = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .custom_flags(NOFOLLOW)
        .open(path)
    else {
        return;
    };
    let Ok(meta) = file.metadata() else {
        return;
    };
    if meta.is_file()
        && meta.uid() == unsafe { getuid() }
        && meta.mode() & 0o077 == 0
        && bytes.len() <= 2048
        && meta.len() <= 131072 - bytes.len() as u64
    {
        let _ = file.write_all(bytes);
    }
}

/// Emit bounded JSONL records individually; the sink retains its per-record
/// and total-file caps even when a report contains several gap traces.
pub fn append_records(path: &Path, jsonl: &str) {
    for record in jsonl.split_inclusive('\n') {
        append_report(path, record.as_bytes());
    }
}

pub struct Binding {
    pub directory: PathBuf,
    pub session: [u8; 16],
    pub installed_delay: Option<u32>,
    // Held through Session::close. EOF tells the owner to clean up after a crash.
    pub owner: Option<Owner>,
}

pub struct Owner {
    stream: UnixStream,
    retired: bool,
}
impl Owner {
    fn new(stream: UnixStream) -> Self {
        Self {
            stream,
            retired: false,
        }
    }
    pub fn finish(mut self) -> io::Result<()> {
        if self.retired {
            return Ok(());
        }
        self.stream.shutdown(std::net::Shutdown::Write)?;
        self.stream.set_nonblocking(false)?;
        self.stream
            .set_read_timeout(Some(Duration::from_secs(60)))?;
        let mut reply = [0];
        self.stream.read_exact(&mut reply)?;
        need(reply == [b'R'], "owner did not confirm complete retirement")
    }
}

fn private(path: &Path, directory: bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    need(
        meta.uid() == unsafe { getuid() }
            && meta.mode() & 0o077 == 0
            && if directory {
                meta.is_dir()
            } else {
                meta.file_type().is_socket()
            },
        "preview path is not private",
    )
}

pub fn connect(root: &Path) -> io::Result<Binding> {
    connect_greeting(root, b"AP4\n")
}
pub fn connect_greeting(root: &Path, greeting: &[u8]) -> io::Result<Binding> {
    if !greeting.starts_with(ap1_native_client::admission::GREETING) {
        return connect_once(root, greeting, None);
    }
    // Concurrent project loads can briefly contend on registry.lock. Only a
    // typed refusal that grants no ownership may be retried. EOF, a partial
    // binding, protocol failure or an actual capacity refusal is never retried.
    // This runs on non-RT instance startup, with one deadline and a finite count.
    let end = Instant::now() + Duration::from_secs(10);
    for attempt in 0..64 {
        match connect_once(root, greeting, Some(end)) {
            Err(e)
                if e.get_ref()
                    .and_then(|e| e.downcast_ref::<ap1_native_client::admission::Refusal>())
                    == Some(&ap1_native_client::admission::Refusal::ServiceBusy)
                    && attempt < 63
                    && end.saturating_duration_since(Instant::now())
                        > Duration::from_millis(20) =>
            {
                std::thread::sleep(Duration::from_millis(20));
            }
            result => return result,
        }
    }
    unreachable!("last attempt always returns")
}
fn connect_once(root: &Path, greeting: &[u8], deadline: Option<Instant>) -> io::Result<Binding> {
    private(root, true)?;
    let address = root.join("owner.sock");
    private(&address, false)?;
    let mut owner = UnixStream::connect(address)?;
    owner.set_write_timeout(Some(match deadline {
        Some(end) => end
            .checked_duration_since(Instant::now())
            .ok_or_else(|| invalid("preview startup timeout"))?
            .min(Duration::from_secs(5)),
        None => Duration::from_secs(5),
    }))?;
    let version3 = greeting.starts_with(ap1_native_client::admission::GREETING);
    let mut request = [0u8; 16];
    if version3 {
        need(greeting.len() == 53, "admission greeting extent")?;
        std::fs::File::open("/dev/urandom")?.read_exact(&mut request)?;
        need(request != [0; 16], "admission request identity")?;
    }
    owner.write_all(greeting)?;
    if version3 {
        owner.write_all(&request)?;
    }
    // One absolute startup-message deadline, including fragmented replies.
    let end = deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(10));
    let mut read = |bytes: &mut [u8]| -> io::Result<()> {
        let mut offset = 0;
        while offset < bytes.len() {
            owner.set_read_timeout(Some(
                end.checked_duration_since(Instant::now())
                    .ok_or_else(|| invalid("preview startup timeout"))?,
            ))?;
            let n = owner.read(&mut bytes[offset..])?;
            need(n != 0, "preview owner disconnected")?;
            offset += n;
        }
        Ok(())
    };
    let mut length = [0; 2];
    read(&mut length)?;
    let n = u16::from_le_bytes(length) as usize;
    let minimum = if version3 {
        ap1_native_client::admission::HEADER
    } else {
        34
    };
    need((minimum..=1024).contains(&n), "preview startup length")?;
    let mut bytes = vec![0; n];
    read(&mut bytes)?;
    if version3 {
        let binding =
            ap1_native_client::admission::decode(&bytes, request)?.map_err(io::Error::other)?;
        let directory = PathBuf::from(binding.directory);
        private(&directory, true)?;
        owner.set_nonblocking(true)?;
        return Ok(Binding {
            directory,
            session: binding.session,
            installed_delay: Some(binding.added_frames),
            owner: Some(Owner::new(owner)),
        });
    }
    let reply = std::str::from_utf8(&bytes).map_err(|_| invalid("preview startup encoding"))?;
    let (id, directory) = reply
        .split_once('\n')
        .ok_or_else(|| invalid("preview startup shape"))?;
    let (installed_delay, directory) = if greeting.starts_with(b"LVB2\n") {
        let (delay, path) = directory
            .split_once('\n')
            .ok_or_else(|| invalid("installed performance binding absent"))?;
        let delay = delay
            .parse::<u32>()
            .map_err(|_| invalid("installed delay encoding"))?;
        need(matches!(delay, 256 | 512), "unsupported installed delay")?;
        (Some(delay), path)
    } else {
        (None, directory)
    };
    need(
        id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit()),
        "preview session syntax",
    )?;
    let directory = PathBuf::from(directory);
    need(
        directory.is_absolute(),
        "preview directory must be absolute",
    )?;
    private(&directory, true)?;
    let session = std::array::from_fn(|i| u8::from_str_radix(&id[i * 2..i * 2 + 2], 16).unwrap());
    owner.set_nonblocking(true)?;
    Ok(Binding {
        directory,
        session,
        installed_delay,
        owner: Some(Owner::new(owner)),
    })
}

pub(crate) fn performance_root(commercial: bool) -> PathBuf {
    if cfg!(feature = "registered") && commercial {
        return PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
            .join(".local/share/linux-vst-bridge/managed/runtime");
    }
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(if commercial {
        "AP9-Performance/serum"
    } else {
        "AP9-Performance/reference"
    })
}
pub fn discover_performance(identity: Option<crate::state::Identity>) -> io::Result<Binding> {
    let mut greeting = if cfg!(feature = "registered") && identity.is_some() {
        ap1_native_client::admission::GREETING.to_vec()
    } else {
        b"AP9\n".to_vec()
    };
    if let Some(i) = identity {
        greeting.extend_from_slice(&i.class);
        greeting.extend_from_slice(&i.module);
    }
    connect_greeting(&performance_root(identity.is_some()), &greeting)
}
pub fn discover() -> io::Result<Binding> {
    let home = std::env::var_os("HOME").ok_or_else(|| invalid("preview home absent"))?;
    connect(&PathBuf::from(home).join("AP4-State-Test/preview"))
}

// Faults outlive disposable Windows stages, including a peer that exits before
// the native worker can publish its first useful explanation.
pub fn report_path(session: [u8; 16]) -> PathBuf {
    let name: String = session.iter().map(|b| format!("{b:02x}")).collect();
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("AP4-State-Test/preview/results")
        .join(format!("native-{name}.jsonl"))
}

/// Owner loss is checked by the transport worker while idle, never by audio.
pub fn check_owner(owner: &mut Option<Owner>) -> io::Result<()> {
    if let Some(owner) = owner {
        let mut reply = [0];
        match owner.stream.read(&mut reply) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(e) => return Err(e),
            result => {
                owner.retired = result.ok() == Some(1) && reply == [b'R'];
                return Err(invalid(
                    "preview owner disconnected or sent unsolicited data",
                ));
            }
        }
    }
    Ok(())
}

pub fn discover_commercial(identity: crate::state::Identity) -> io::Result<Binding> {
    let home = std::env::var_os("HOME").ok_or_else(|| invalid("preview home absent"))?;
    let mut greeting = b"AP8\n".to_vec();
    greeting.extend_from_slice(&identity.class);
    greeting.extend_from_slice(&identity.module);
    connect_greeting(
        &PathBuf::from(home).join("AP8-Commercial-Test/preview"),
        &greeting,
    )
}
pub fn commercial_report_path(session: [u8; 16]) -> PathBuf {
    let name: String = session.iter().map(|b| format!("{b:02x}")).collect();
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("AP8-Commercial-Test/preview/results")
        .join(format!("native-{name}.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::{
        fs::{DirBuilderExt, PermissionsExt},
        net::UnixListener,
    };
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = PathBuf::from(format!(
                "/tmp/ap4-{}",
                u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
            ));
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn registered_v3_refusal_is_correlated_and_never_creates_a_binding() {
        use ap1_native_client::admission::{self, Refusal};
        for stale in [false, true] {
            for reason in [
                Refusal::ServiceBusy,
                Refusal::GlobalCapacity,
                Refusal::ClassCapacity,
                Refusal::CleanupUnconfirmed,
            ] {
                let dir = Directory::new();
                let socket = dir.0.join("owner.sock");
                let listener = UnixListener::bind(&socket).unwrap();
                fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
                let peer = std::thread::spawn(move || {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut hello = [0; 69];
                    stream.read_exact(&mut hello).unwrap();
                    assert_eq!(&hello[..5], admission::GREETING);
                    let mut request: [u8; 16] = hello[53..].try_into().unwrap();
                    assert_ne!(request, [0; 16]);
                    if stale {
                        request[0] ^= 1;
                    }
                    let reply = admission::refused(request, reason);
                    stream
                        .write_all(&(reply.len() as u16).to_le_bytes())
                        .unwrap();
                    // Fragmentation remains inside the single startup deadline.
                    for byte in reply {
                        stream.write_all(&[byte]).unwrap();
                    }
                    let mut end = Vec::new();
                    stream.read_to_end(&mut end).unwrap();
                    assert!(end.is_empty());
                });
                let mut greeting = admission::GREETING.to_vec();
                greeting.extend([7; 48]);
                let error = connect_once(&dir.0, &greeting, None)
                    .err()
                    .unwrap()
                    .to_string();
                assert_eq!(
                    error,
                    if stale {
                        "admission stale reply"
                    } else {
                        reason.code()
                    }
                );
                peer.join().unwrap();
                assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
            }
        }
    }
    #[test]
    fn only_unowned_busy_can_retry_and_retry_count_is_bounded() {
        use ap1_native_client::admission::{self, Refusal};
        for succeeds in [true, false] {
            let dir = Directory::new();
            let socket = dir.0.join("owner.sock");
            let listener = UnixListener::bind(&socket).unwrap();
            fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
            let path = dir.0.clone();
            let peer = std::thread::spawn(move || {
                let count = if succeeds { 2 } else { 64 };
                let mut identities = std::collections::BTreeSet::new();
                for i in 0..count {
                    let (mut stream, _) = listener.accept().unwrap();
                    let mut hello = [0; 69];
                    stream.read_exact(&mut hello).unwrap();
                    let request = hello[53..].try_into().unwrap();
                    assert!(identities.insert(request));
                    let bytes = if succeeds && i == 1 {
                        admission::accepted(
                            request,
                            &admission::Binding {
                                session: [9; 16],
                                added_frames: 512,
                                directory: path.to_str().unwrap().into(),
                            },
                        )
                        .unwrap()
                    } else {
                        admission::refused(request, Refusal::ServiceBusy)
                    };
                    stream
                        .write_all(&(bytes.len() as u16).to_le_bytes())
                        .unwrap();
                    stream.write_all(&bytes).unwrap();
                    assert_eq!(stream.read(&mut [0]).unwrap(), 0);
                }
                count
            });
            let mut greeting = admission::GREETING.to_vec();
            greeting.extend([7; 48]);
            let result = connect_greeting(&dir.0, &greeting);
            if succeeds {
                assert_eq!(result.unwrap().session, [9; 16]);
            } else {
                assert_eq!(
                    result.err().unwrap().to_string(),
                    Refusal::ServiceBusy.code()
                );
            }
            assert_eq!(peer.join().unwrap(), if succeeds { 2 } else { 64 });
            assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
        }
    }
    #[test]
    fn registered_v3_success_and_legacy_success_keep_distinct_encodings() {
        use ap1_native_client::admission;
        let dir = Directory::new();
        let socket = dir.0.join("owner.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).unwrap();
        let path = dir.0.clone();
        let peer = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut hello = [0; 69];
            stream.read_exact(&mut hello).unwrap();
            let reply = admission::accepted(
                hello[53..].try_into().unwrap(),
                &admission::Binding {
                    session: [9; 16],
                    added_frames: 512,
                    directory: path.to_str().unwrap().into(),
                },
            )
            .unwrap();
            stream
                .write_all(&(reply.len() as u16).to_le_bytes())
                .unwrap();
            stream.write_all(&reply).unwrap();
            let mut end = Vec::new();
            stream.read_to_end(&mut end).unwrap();
        });
        let mut greeting = admission::GREETING.to_vec();
        greeting.extend([7; 48]);
        let b = connect_greeting(&dir.0, &greeting).unwrap();
        assert_eq!(b.session, [9; 16]);
        assert_eq!(b.installed_delay, Some(512));
        drop(b);
        peer.join().unwrap();
    }
    #[test]
    fn installed_binding_transfers_a_validated_inactive_delay() {
        for value in ["256", "512", "128", "512\nextra"] {
            let dir = Directory::new();
            let listener = UnixListener::bind(dir.0.join("owner.sock")).unwrap();
            fs::set_permissions(dir.0.join("owner.sock"), fs::Permissions::from_mode(0o600))
                .unwrap();
            let path = dir.0.clone();
            let sent_value = value.to_string();
            let peer = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut hello = [0; 5];
                stream.read_exact(&mut hello).unwrap();
                assert_eq!(&hello, b"LVB2\n");
                let reply = format!("{}\n{}\n{}", "aa".repeat(16), sent_value, path.display());
                stream
                    .write_all(&(reply.len() as u16).to_le_bytes())
                    .unwrap();
                stream.write_all(reply.as_bytes()).unwrap();
                let mut end = Vec::new();
                stream.read_to_end(&mut end).unwrap();
            });
            let result = connect_greeting(&dir.0, b"LVB2\n");
            if matches!(value, "256" | "512") {
                assert_eq!(
                    result.unwrap().installed_delay,
                    Some(value.parse().unwrap())
                );
            } else {
                assert!(result.is_err());
            }
            peer.join().unwrap();
        }
    }
    #[test]
    fn retirement_requires_positive_owner_disposition() {
        for reply in [b"R".as_slice(), b"".as_slice(), b"F".as_slice()] {
            let (mut peer, socket) = UnixStream::pair().unwrap();
            let t = std::thread::spawn(move || {
                let mut byte = [0];
                assert_eq!(peer.read(&mut byte).unwrap(), 0);
                peer.write_all(reply).unwrap();
            });
            assert_eq!(Owner::new(socket).finish().is_ok(), reply == b"R");
            t.join().unwrap();
        }
        let (mut peer, socket) = UnixStream::pair().unwrap();
        socket.set_nonblocking(true).unwrap();
        let mut owner = Some(Owner::new(socket));
        peer.write_all(b"R").unwrap();
        assert!(check_owner(&mut owner).is_err());
        owner.unwrap().finish().unwrap(); // acknowledged before the fault was noticed
    }
    #[test]
    fn private_discovery_fragmented_reply_and_owner_disconnect() {
        let dir = Directory::new();
        let path = dir.0.join("owner.sock");
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let reply = format!("{}\n{}", "a".repeat(32), dir.0.display()).into_bytes();
        let t = std::thread::spawn(move || {
            let (mut peer, _) = listener.accept().unwrap();
            let mut greeting = [0; 4];
            peer.read_exact(&mut greeting).unwrap();
            assert_eq!(&greeting, b"AP4\n");
            for b in (reply.len() as u16).to_le_bytes().into_iter().chain(reply) {
                peer.write_all(&[b]).unwrap();
            }
            peer
        });
        let mut binding = connect(&dir.0).unwrap();
        let peer = t.join().unwrap();
        assert_eq!(binding.directory, dir.0);
        assert_eq!(binding.session, [0xaa; 16]);
        check_owner(&mut binding.owner).unwrap();
        // A normal pause has no heartbeat requirement or hidden expiry.
        std::thread::sleep(Duration::from_millis(20));
        check_owner(&mut binding.owner).unwrap();
        drop(peer);
        assert!(check_owner(&mut binding.owner).is_err());
    }
    #[test]
    fn startup_owner_loss_cancels_transport_accept() {
        let dir = Directory::new();
        let (peer, owner) = UnixStream::pair().unwrap();
        owner.set_nonblocking(true).unwrap();
        drop(peer);
        let started = Instant::now();
        assert!(crate::Session::open(
            Binding {
                installed_delay: None,
                directory: dir.0.clone(),
                session: [3; 16],
                owner: Some(Owner::new(owner))
            },
            256,
            4
        )
        .is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn absent_stale_and_public_endpoints_refuse_without_fallback() {
        let dir = Directory::new();
        assert!(connect(&dir.0).is_err());
        let path = dir.0.join("owner.sock");
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        assert!(connect(&dir.0).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        drop(listener);
        assert!(connect(&dir.0).is_err());
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("absent", path).unwrap();
        assert!(connect(&dir.0).is_err());
    }
}

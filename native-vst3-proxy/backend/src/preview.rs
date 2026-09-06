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
        && meta.len() <= 65536 - bytes.len() as u64
    {
        let _ = file.write_all(bytes);
    }
}

pub struct Binding {
    pub directory: PathBuf,
    pub session: [u8; 16],
    // Held through Session::close. EOF tells the owner to clean up after a crash.
    pub owner: Option<UnixStream>,
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
    private(root, true)?;
    let address = root.join("owner.sock");
    private(&address, false)?;
    let mut owner = UnixStream::connect(address)?;
    owner.set_write_timeout(Some(Duration::from_secs(5)))?;
    owner.write_all(b"AP4\n")?;
    // One absolute startup-message deadline, including fragmented replies.
    let end = Instant::now() + Duration::from_secs(10);
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
    need((34..=1024).contains(&n), "preview startup length")?;
    let mut bytes = vec![0; n];
    read(&mut bytes)?;
    let reply = std::str::from_utf8(&bytes).map_err(|_| invalid("preview startup encoding"))?;
    let (id, directory) = reply
        .split_once('\n')
        .ok_or_else(|| invalid("preview startup shape"))?;
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
        owner: Some(owner),
    })
}

pub fn discover() -> io::Result<Binding> {
    let home = std::env::var_os("HOME").ok_or_else(|| invalid("preview home absent"))?;
    connect(&PathBuf::from(home).join("AP4-State-Test/preview"))
}

/// Owner loss is checked by the transport worker while idle, never by audio.
pub fn check_owner(owner: &mut Option<UnixStream>) -> io::Result<()> {
    if let Some(owner) = owner {
        match owner.read(&mut [0]) {
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(e) => return Err(e),
            _ => {
                return Err(invalid(
                    "preview owner disconnected or sent unsolicited data",
                ))
            }
        }
    }
    Ok(())
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
                directory: dir.0.clone(),
                session: [3; 16],
                owner: Some(owner)
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

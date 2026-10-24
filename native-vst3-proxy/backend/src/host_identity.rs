//! Optional VST3 host self-report. Initialization/recovery only, never audio.
use std::{io, path::Path};

pub fn from_utf16(value: &[u16]) -> Option<String> {
    if value.is_empty() || value.len() > 127 { return None; }
    let name = String::from_utf16(value).ok()?;
    ap1_native_client::host_identity::validate_name(&name).ok()?;
    Some(name)
}

pub fn report(root: &Path, session: [u8; 16], name: &str) {
    // Reporting is not launch admission. A missing/old/busy service simply
    // leaves host observation unavailable, with no retry or launch failure.
    let _ = send(root, session, name);
}

#[cfg(not(target_os = "linux"))]
fn send(_root: &Path, _session: [u8; 16], _name: &str) -> io::Result<()> { Ok(()) }

#[cfg(target_os = "linux")]
fn send(root: &Path, session: [u8; 16], name: &str) -> io::Result<()> {
    use std::{io::{Read, Write}, os::{fd::{AsRawFd, FromRawFd}, unix::{ffi::OsStrExt,
        fs::{FileTypeExt, MetadataExt}, net::UnixStream}}, time::{Duration, Instant}};
    let bytes = ap1_native_client::host_identity::encode(session, name)?;
    let path = root.join("owner.sock");
    for (path, directory) in [(root, true), (path.as_path(), false)] {
        let meta = std::fs::symlink_metadata(path)?;
        crate::need(meta.uid() == unsafe { libc::getuid() } && meta.mode() & 0o077 == 0
            && if directory { meta.is_dir() } else { meta.file_type().is_socket() }, "host_report_custody")?;
    }
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as _;
    let raw = path.as_os_str().as_bytes();
    crate::need(raw.len() < address.sun_path.len(), "host_report_address")?;
    for (target, byte) in address.sun_path.iter_mut().zip(raw) { *target = *byte as _; }
    let end = Instant::now() + Duration::from_millis(250);
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 { return Err(io::Error::last_os_error()); }
    let socket = unsafe { UnixStream::from_raw_fd(fd) };
    let connected = unsafe { libc::connect(fd, (&address as *const libc::sockaddr_un).cast(),
        std::mem::size_of_val(&address) as _) };
    if connected != 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) { return Err(error); }
    }
    let wait = |events| -> io::Result<()> {
        loop {
            let remaining = end.checked_duration_since(Instant::now())
                .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "host report deadline"))?;
            let mut poll = libc::pollfd { fd:socket.as_raw_fd(), events, revents:0 };
            let result = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
            if result > 0 { return Ok(()); }
            if result == 0 { return Err(io::Error::new(io::ErrorKind::TimedOut, "host report deadline")); }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted { return Err(error); }
        }
    };
    wait(libc::POLLOUT)?;
    if let Some(error) = socket.take_error()? { return Err(error); }
    socket.peer_addr()?;
    let mut written = 0;
    while written < bytes.len() {
        wait(libc::POLLOUT)?;
        match (&socket).write(&bytes[written..]) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::WriteZero, "host report closed")),
            Ok(count) => written += count,
            Err(error) if matches!(error.kind(), io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock) => (),
            Err(error) => return Err(error),
        }
    }
    loop {
        wait(libc::POLLIN)?;
        let mut reply = [0];
        match (&socket).read(&mut reply) {
            Ok(1) if reply == [b'H'] => return Ok(()),
            Ok(_) => return Err(crate::invalid("host report not acknowledged")),
            Err(error) if matches!(error.kind(), io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock) => (),
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_text_is_bounded_strict_utf16_not_lossy_process_identification() {
        for name in ["REAPER", "Other host 🎹"] {
            assert_eq!(from_utf16(&name.encode_utf16().collect::<Vec<_>>()).as_deref(), Some(name));
        }
        for invalid in [vec![], vec![0xd800], vec![0xdc00], vec![65, 0, 66], vec![65, 10], vec![32], vec![65;128]] {
            assert!(from_utf16(&invalid).is_none());
        }
        assert_eq!(from_utf16(&[65;127]).unwrap().len(), 127);
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn bounded_sender_delivers_exact_frame_and_old_service_refusal_is_optional() {
        use std::{io::{Read, Write}, os::unix::{fs::PermissionsExt, net::UnixListener},
            sync::atomic::{AtomicU64, Ordering}, time::Duration};
        static NEXT: AtomicU64=AtomicU64::new(0);
        let root=std::env::temp_dir().join(format!("lvb-host-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root,std::fs::Permissions::from_mode(0o700)).unwrap();
        for accepted in [true,false] {
            let path=root.join("owner.sock");let listener=UnixListener::bind(&path).unwrap();
            std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o600)).unwrap();
            listener.set_nonblocking(true).unwrap();
            let server=std::thread::spawn(move || {
                let end=std::time::Instant::now()+Duration::from_secs(2);
                let (mut stream,_)=loop {
                    match listener.accept() {
                        Ok(pair)=>break pair,
                        Err(error) if error.kind()==io::ErrorKind::WouldBlock && std::time::Instant::now()<end
                            =>std::thread::sleep(Duration::from_millis(1)),
                        Err(error)=>panic!("bounded test accept: {error}"),
                    }
                };
                stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                let mut greeting=[0;5];stream.read_exact(&mut greeting).unwrap();
                assert_eq!(&greeting,ap1_native_client::host_identity::GREETING);
                assert_eq!(ap1_native_client::host_identity::read(&mut stream).unwrap(),([9;16],"Unknown host 🎹".into()));
                stream.write_all(if accepted {b"H"} else {b"?"}).unwrap();
            });
            assert_eq!(send(&root,[9;16],"Unknown host 🎹").is_ok(),accepted);
            server.join().unwrap();std::fs::remove_file(path).unwrap();
        }
        // No service endpoint: optional reporting has no execution outcome.
        report(&root,[9;16],"Unknown host 🎹");
        std::fs::remove_dir(root).unwrap();
    }

}

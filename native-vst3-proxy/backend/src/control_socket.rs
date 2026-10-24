//! Bounded local control connection; never used by the audio callback.
use std::{io, os::unix::net::UnixStream, path::Path, time::Duration};
#[cfg(target_os = "linux")]
use std::{
    os::{fd::FromRawFd, unix::ffi::OsStrExt},
    time::Instant,
};
#[cfg(target_os = "linux")]
pub fn connect(path: &Path, limit: Duration) -> io::Result<UnixStream> {
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as _;
    let raw = path.as_os_str().as_bytes();
    crate::need(raw.len() < address.sun_path.len(), "control socket path")?;
    for (to, from) in address.sun_path.iter_mut().zip(raw) {
        *to = *from as _;
    }
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let socket = unsafe { UnixStream::from_raw_fd(fd) };
    if unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&address) as _,
        )
    } != 0
    {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(error);
        }
    }
    let deadline = Instant::now() + limit;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "control connect deadline"))?;
        let mut poll = libc::pollfd {
            fd,
            events: libc::POLLOUT,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
        if result > 0 {
            break;
        }
        if result == 0 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "control connect deadline",
            ));
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
    if let Some(error) = socket.take_error()? {
        return Err(error);
    }
    socket.peer_addr()?;
    Ok(socket)
}
#[cfg(not(target_os = "linux"))]
pub fn connect(_path: &Path, _limit: Duration) -> io::Result<UnixStream> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Bridge proxy control requires Linux",
    ))
}

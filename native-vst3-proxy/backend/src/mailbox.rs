//! Delivery mailbox v3 (v2 audio layout plus correlated timing). Only the non-RT transport worker touches this view.
//! The existing authenticated socket owns lifecycle/state; one request is in flight.
use ap1_native_client::{invalid, need, Frame};
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io,
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::Path,
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
    time::{Duration, Instant},
};
const BYTES: usize = 33024;
const REQUEST: usize = 256;
const REPLY: usize = 16640;
const REQUEST_CAP: usize = 16384;
const REPLY_CAP: usize = 16384;
unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
pub struct Mailbox {
    pointer: NonNull<u8>,
    _file: File,
    pub diagnostic: [u64; 15],
}
// A view is moved into the single transport worker. No borrowed data escapes.
unsafe impl Send for Mailbox {}
impl Mailbox {
    pub fn create(path: &Path, session: [u8; 16]) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.set_len(BYTES as u64)?;
        let pointer = unsafe { mmap(std::ptr::null_mut(), BYTES, 3, 1, file.as_raw_fd(), 0) };
        need(pointer as isize != -1, "delivery mapping failed")?;
        let mut view = Self {
            pointer: NonNull::new(pointer.cast())
                .ok_or_else(|| invalid("null delivery mapping"))?,
            _file: file,
            diagnostic: [0; 15],
        };
        view.write(0, b"LVBM");
        view.write(4, &3u32.to_le_bytes());
        view.write(8, &(BYTES as u32).to_le_bytes());
        view.write(16, &session);
        view.flag(64).store(0, Ordering::Release);
        view.flag(128).store(0, Ordering::Release);
        Ok(view)
    }
    fn flag(&self, offset: usize) -> &AtomicU32 {
        unsafe { &*self.pointer.as_ptr().add(offset).cast::<AtomicU32>() }
    }
    fn write(&mut self, offset: usize, bytes: &[u8]) {
        assert!(offset + bytes.len() <= BYTES);
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.pointer.as_ptr().add(offset),
                bytes.len(),
            )
        }
    }
    fn read(&self, offset: usize, size: usize) -> Vec<u8> {
        assert!(offset + size <= BYTES);
        let mut bytes = vec![0; size];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.pointer.as_ptr().add(offset),
                bytes.as_mut_ptr(),
                size,
            )
        };
        bytes
    }
    fn idle(&self) -> io::Result<()> {
        need(
            self.flag(64).load(Ordering::Acquire) == 0
                && self.flag(128).load(Ordering::Acquire) == 0,
            "delivery slot is not free",
        )
    }
    // Control notification is consumed before the Windows delivery thread
    // resumes mailbox requests. The worker yields until this handoff is done.
    pub fn control_handoff_pending(&self) -> bool {
        self.flag(64).load(Ordering::Acquire) == 2
    }
    pub fn send(&mut self, frame: &Frame, minor: u64) -> io::Result<()> {
        self.idle()?;
        let bytes = frame.encode_version(minor)?;
        need(
            frame.kind == ap1_native_client::PROCESS && bytes.len() <= REQUEST_CAP,
            "delivery request extent/kind",
        )?;
        self.write(REQUEST, &bytes);
        self.write(68, &(bytes.len() as u32).to_le_bytes());
        self.flag(64).store(1, Ordering::Release);
        Ok(())
    }
    /// Called after a complete control frame has been sent on the socket. This
    /// avoids polling Winsock while there is no control work to consume.
    pub fn control(&mut self) -> io::Result<()> {
        self.idle()?;
        self.flag(64).store(2, Ordering::Release);
        Ok(())
    }
    pub fn receive(&mut self, minor: u64, end: Instant) -> io::Result<Frame> {
        self.receive_while(minor, end, || Ok(()))
    }
    pub fn receive_while(&mut self, minor: u64, end: Instant, mut healthy: impl FnMut() -> io::Result<()>) -> io::Result<Frame> {
        let mut next_health = Instant::now();
        loop {
            match self.flag(128).load(Ordering::Acquire) {
                0 => {}
                1 => break,
                _ => return Err(invalid("invalid delivery response flag")),
            }
            need(Instant::now() < end, "delivery response deadline")?;
            if Instant::now() >= next_health {
                healthy()?;
                next_health = Instant::now() + Duration::from_millis(10);
            }
            std::thread::sleep(Duration::from_micros(50));
        }
        let size = u32::from_le_bytes(self.read(132, 4).try_into().unwrap()) as usize;
        need(
            (ap1_native_client::HEADER..=REPLY_CAP).contains(&size),
            "delivery response extent",
        )?;
        let result = Frame::decode_version(&self.read(REPLY, size), minor);
        for i in 0..15 {
            let mut bytes = [0; 8];
            unsafe { std::ptr::copy_nonoverlapping(self.pointer.as_ptr().add(136+i*8), bytes.as_mut_ptr(), 8); }
            self.diagnostic[i] = u64::from_le_bytes(bytes);
        }
        self.flag(128).store(0, Ordering::Release);
        result
    }
}

/// Non-consuming endpoint check on the transport worker. Do not turn a known
/// dead socket into a five-second wait for an impossible mailbox response.
pub fn peer_alive(socket: &std::net::TcpStream) -> io::Result<()> {
    peer_status(socket, false)
}
pub fn peer_status(socket: &std::net::TcpStream, capture_pending: bool) -> io::Result<()> {
    unsafe extern "C" { fn recv(fd: i32, p: *mut u8, n: usize, flags: i32) -> isize; }
    #[cfg(target_os = "linux")] const DONTWAIT: i32 = 0x40;
    #[cfg(target_os = "macos")] const DONTWAIT: i32 = 0x80;
    let mut byte = 0;
    match unsafe { recv(socket.as_raw_fd(), &mut byte, 1, DONTWAIT | 2) } {
        0 => Err(invalid("delivery endpoint disconnected")),
        n if n > 0 => need(capture_pending, "unexpected socket data during mailbox delivery"),
        _ => {
            let error = io::Error::last_os_error();
            if matches!(error.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) { Ok(()) } else { Err(error) }
        }
    }
}
impl Drop for Mailbox {
    fn drop(&mut self) {
        unsafe {
            munmap(self.pointer.as_ptr().cast(), BYTES);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_endpoint_loss_interrupts_an_unfinished_request() {
        let path = std::env::temp_dir().join(format!("ap13-loss-{:032x}", u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
        let mut mailbox = Mailbox::create(&path, [13; 16]).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (peer, _) = listener.accept().unwrap();
        peer_alive(&client).unwrap();
        mailbox.send(&Frame { kind: 3, session: [13;16], sequence: 9, payload: vec![0;56] }, 7).unwrap();
        let killer = std::thread::spawn(move || { std::thread::sleep(Duration::from_millis(20)); drop(peer); });
        let start = Instant::now();
        let error = mailbox.receive_while(7, start + Duration::from_secs(5), || peer_alive(&client)).unwrap_err();
        assert!(error.to_string().contains("endpoint disconnected"));
        assert!(start.elapsed() < Duration::from_millis(500));
        killer.join().unwrap();
        assert_eq!(mailbox.flag(64).load(Ordering::Acquire), 1);
        drop(mailbox);std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn ownership_control_and_invalid_reply_are_bounded() {
        let path = std::env::temp_dir().join(format!(
            "ap10-mailbox-{:032x}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        let mut m = Mailbox::create(&path, [3; 16]).unwrap();
        let f = Frame {
            kind: 3,
            session: [3; 16],
            sequence: 9,
            payload: vec![0; 56],
        };
        m.send(&f, 7).unwrap();
        assert!(m.send(&f, 7).is_err());
        assert!(m.control().is_err());
        assert_eq!(m.read(16, 16), [3; 16]);
        assert_eq!(
            Frame::decode_version(&m.read(REQUEST, 112), 7)
                .unwrap()
                .sequence,
            9
        );
        // Counterpart consumes the request before it publishes a response. The wire
        // preserves the original session/sequence, including reordered/invalid replies.
        m.flag(64).store(0, Ordering::Release);
        let reply = Frame {
            kind: 4,
            session: [3; 16],
            sequence: 9,
            payload: vec![0; 40],
        }
        .encode_version(7)
        .unwrap();
        m.write(REPLY, &reply);
        m.write(132, &(reply.len() as u32).to_le_bytes());
        m.flag(128).store(1, Ordering::Release);
        assert_eq!(m.receive(7, Instant::now()).unwrap().sequence, 9);
        m.control().unwrap();
        assert_eq!(m.flag(64).load(Ordering::Acquire), 2);
        m.flag(64).store(0, Ordering::Release);
        assert!(m.receive(7, Instant::now()).is_err());
        m.write(132, &16385u32.to_le_bytes());
        m.flag(128).store(1, Ordering::Release);
        assert!(m.receive(7, Instant::now()).is_err());
        drop(m);
        std::fs::remove_file(path).unwrap();
    }
}

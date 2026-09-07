//! AP10 delivery mailbox v1. Only the non-RT transport worker touches this view.
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
const BYTES: usize = 16896;
const REQUEST: usize = 256;
const REPLY: usize = 16640;
const REQUEST_CAP: usize = 16384;
const REPLY_CAP: usize = 256;
unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
pub struct Mailbox {
    pointer: NonNull<u8>,
    _file: File,
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
        };
        view.write(0, b"LVBM");
        view.write(4, &1u32.to_le_bytes());
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
        loop {
            match self.flag(128).load(Ordering::Acquire) {
                0 => {}
                1 => break,
                _ => return Err(invalid("invalid delivery response flag")),
            }
            need(Instant::now() < end, "delivery response deadline")?;
            std::thread::sleep(Duration::from_micros(50));
        }
        let size = u32::from_le_bytes(self.read(132, 4).try_into().unwrap()) as usize;
        need(
            (ap1_native_client::HEADER..=REPLY_CAP).contains(&size),
            "delivery response extent",
        )?;
        let result = Frame::decode_version(&self.read(REPLY, size), minor);
        self.flag(128).store(0, Ordering::Release);
        result
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
        m.write(132, &257u32.to_le_bytes());
        m.flag(128).store(1, Ordering::Release);
        assert!(m.receive(7, Instant::now()).is_err());
        drop(m);
        std::fs::remove_file(path).unwrap();
    }
}

//! Independent fault visibility, never called by the DAW audio callback.
//! v2: 1024 bytes; immutable session header, three single-writer lanes. Each
//! lane has an atomic publication counter and two atomic-word slots. A writer
//! killed during publication leaves the previous slot readable. Readers retry
//! at most three times; all words are atomic (no seqlock data races).
//! Native words 10..16 carry sampled cumulative presentation counters.
use ap1_native_client::{invalid, need};
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io,
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::Path,
    ptr::NonNull,
    sync::atomic::{AtomicU64, Ordering},
};
const BYTES: usize = 1024;
unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
pub struct Status {
    pointer: NonNull<u8>,
    _file: File,
    counter: u64,
    pub generation: u64,
    pub delivery: [u64; 6],
}
unsafe impl Send for Status {}
impl Status {
    pub fn create(path: &Path, session: [u8; 16]) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.set_len(BYTES as u64)?;
        let p = unsafe { mmap(std::ptr::null_mut(), BYTES, 3, 1, file.as_raw_fd(), 0) };
        need(p as isize != -1, "fault status mapping failed")?;
        let value = Self {
            pointer: NonNull::new(p.cast()).ok_or_else(|| invalid("null fault status"))?,
            _file: file,
            counter: 0,
            generation: 1,
            delivery: [0; 6],
        };
        let mut header = [0u8; 64];
        header[..4].copy_from_slice(b"LVFS");
        header[4..8].copy_from_slice(&2u32.to_le_bytes());
        header[8..12].copy_from_slice(&(BYTES as u32).to_le_bytes());
        header[16..32].copy_from_slice(&session);
        unsafe {
            std::ptr::copy_nonoverlapping(header.as_ptr(), value.pointer.as_ptr(), 64);
        }
        Ok(value)
    }
    fn word(&self, offset: usize) -> &AtomicU64 {
        unsafe { &*self.pointer.as_ptr().add(offset).cast::<AtomicU64>() }
    }
    pub fn publish(&mut self, epoch: u64, sequence: u64, position: u64, stage: u64, detail: u64) {
        // Overflow would require centuries at the maximum processing rate.
        let next = self
            .counter
            .checked_add(1)
            .expect("fault status sequence exhausted");
        let base = 128 + (next as usize & 1) * 128;
        let row = [
            self.generation,
            epoch,
            sequence,
            position,
            stage,
            detail,
            crate::observer::monotonic_ns(),
            1_000_000_000,
            0,
            u64::from(std::process::id()),
        ];
        for (i, v) in row.iter().enumerate() {
            self.word(base + i * 8).store(*v, Ordering::SeqCst);
        }
        for (i, v) in self.delivery.iter().enumerate() {
            self.word(base + (10 + i) * 8).store(*v, Ordering::SeqCst);
        }
        self.word(64).store(next, Ordering::SeqCst);
        self.counter = next;
    }
}
impl Drop for Status {
    fn drop(&mut self) {
        unsafe {
            munmap(self.pointer.as_ptr().cast(), BYTES);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    #[test]
    fn first_idle_request_remains_visible_after_real_transport_deadline() {
        let dir=std::env::temp_dir().join(format!("ap12-fault-{:032x}",u128::from_le_bytes(mapping::random().unwrap())));
        std::fs::create_dir(&dir).unwrap();
        let id=[12;16];
        let status=Status::create(&dir.join("ap12.status"),id).unwrap();
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let socket=TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (_peer,_)=listener.accept().unwrap();
        let mut session=Session {
            gui:None,gui_revision:0,mailbox:Some(mailbox::Mailbox::create(&dir.join("ap10.delivery"),id).unwrap()),mailbox_enabled:true,fault_status:Some(status),
            notices:(0,0),returned:Default::default(),mapping:Some(Mapping::new(&dir.join("ap1.audio")).unwrap()),socket,
            state:ClientState { session:id,next:9,slot:Slot::Writable },phase:11,max:256,minor:11,epoch:1,position:0,witness:None,identity:None,trace:Default::default(),sample_rate:48000,armed:false,owner:None,
        };
        // An admitted peer deliberately never consumes the first silent block.
        // Production Session::process_events -> Mailbox::receive must keep the
        // original five-second terminal deadline and leave an independent row.
        let start=std::time::Instant::now();
        let error=session.process_events(256,f64::NAN,3,[&[0.;256];2],&[],context::Context::default()).unwrap_err();
        assert!(error.to_string().contains("delivery response deadline"));
        assert!(start.elapsed()>=std::time::Duration::from_secs(5));
        assert!(start.elapsed()<std::time::Duration::from_secs(7));
        assert!(!session.armed);
        let status=session.fault_status.as_ref().unwrap();let c=status.word(64).load(Ordering::SeqCst);let slot=128+(c as usize&1)*128;
        assert_eq!((0..6).map(|i|status.word(slot+i*8).load(Ordering::SeqCst)).collect::<Vec<_>>(),[1,1,9,0,2,0]);
        // Do not let partially overwritten inactive data replace the last row.
        status.word(128+((c as usize+1)&1)*128).store(999,Ordering::SeqCst);
        assert_eq!(status.word(slot).load(Ordering::SeqCst),1);
        drop(session);std::fs::remove_dir_all(dir).unwrap();
    }
}

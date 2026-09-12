//! IF1 terminal custody v1. Independent from audio/result protocol versions.
//! Three single-writer producer slots publish before a shared first-winner CAS.
//! A producer killed before commit cannot obstruct another producer. No writer
//! can replace the first complete record. All mutable shared words are atomic.
//! Native progress has two slots and a bounded reader. Only the transport owner
//! updates it; never the DAW callback. No payload, pointer, or text is retained.
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
pub const BYTES: usize = 2048;
pub const WORDS: usize = 24;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Record {
    pub words: [u64; WORDS],
}
// Record: schema, session low/high, generation, epoch, request sequence,
// last completed end position, state revision/generation/through, digest[4],
// class (1 root exit, 2 editor/controller fatal, 3 transport), status,
// failed request position, native lifecycle phase, producer, status domain, reserved[4].
pub struct Status {
    pointer: NonNull<u8>,
    _file: File,
    session: [u8; 16],
}
unsafe impl Send for Status {}
unsafe impl Sync for Status {}
unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
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
        need(p as isize != -1, "terminal status mmap")?;
        let value = Self {
            pointer: NonNull::new(p.cast()).ok_or_else(|| invalid("terminal mapping null"))?,
            _file: file,
            session,
        };
        let mut header = [0u8; 64];
        header[..4].copy_from_slice(b"LVIF");
        header[4..8].copy_from_slice(&1u32.to_le_bytes());
        header[8..12].copy_from_slice(&(BYTES as u32).to_le_bytes());
        header[16..32].copy_from_slice(&session);
        unsafe { std::ptr::copy_nonoverlapping(header.as_ptr(), value.pointer.as_ptr(), 64) };
        value.progress([1, 0, 1, 0, 0], Some(0), None);
        Ok(value)
    }
    fn word(&self, offset: usize) -> &AtomicU64 {
        unsafe { &*self.pointer.as_ptr().add(offset).cast::<AtomicU64>() }
    }
    pub fn progress(
        &self,
        context: [u64; 5],
        complete: Option<u64>,
        snapshot: Option<&crate::recovery::Snapshot>,
    ) {
        let [generation, epoch, sequence, position, phase] = context;
        let counter = self.word(1024).load(Ordering::SeqCst);
        // This method has one progress writer. Its previous complete context
        // cannot race itself; failure producers never write these slots.
        let previous = self.context().unwrap_or_default();
        let mut row = previous;
        row.words[0] = 1;
        row.words[1] = u64::from_le_bytes(self.session[..8].try_into().unwrap());
        row.words[2] = u64::from_le_bytes(self.session[8..].try_into().unwrap());
        row.words[3] = generation;
        row.words[4] = epoch;
        row.words[5] = sequence;
        row.words[16] = position;
        row.words[17] = phase;
        if let Some(end) = complete {
            row.words[6] = end;
        }
        if let Some(s) = snapshot {
            row.words[7] = s.revision;
            row.words[8] = s.generation;
            row.words[9] = s.through;
            for (i, b) in s.digest.chunks_exact(8).enumerate() {
                row.words[10 + i] = u64::from_le_bytes(b.try_into().unwrap());
            }
        }
        if counter != 0 && row == previous {
            return;
        }
        let next = counter.checked_add(1).expect("terminal progress exhausted");
        let at = 1088 + (next as usize & 1) * 256;
        for (i, v) in row.words.iter().enumerate() {
            self.word(at + i * 8).store(*v, Ordering::SeqCst);
        }
        self.word(1024).store(next, Ordering::SeqCst);
    }
    pub fn context(&self) -> Option<Record> {
        self.context_after_copy(|| {})
    }
    // Empty in production; tests force publication precisely during a read.
    fn context_after_copy(&self, mut after_copy: impl FnMut()) -> Option<Record> {
        for _ in 0..3 {
            let c = self.word(1024).load(Ordering::SeqCst);
            if c == 0 {
                return None;
            }
            let at = 1088 + (c as usize & 1) * 256;
            let r = Record {
                words: std::array::from_fn(|i| self.word(at + i * 8).load(Ordering::SeqCst)),
            };
            after_copy();
            if self.word(1024).load(Ordering::SeqCst) == c {
                return Some(r);
            }
        }
        None
    }
    // Called once by the native worker. Windows and supervisor own other slots.
    pub fn fail_native(&self, status: u64) {
        if self.word(64).load(Ordering::SeqCst) != 0 {
            return;
        }
        let Some(mut r) = self.context() else { return };
        r.words[14] = 3;
        r.words[15] = status;
        r.words[18] = 1;
        r.words[19] = 1;
        for (i, v) in r.words.iter().enumerate() {
            self.word(128 + i * 8).store(*v, Ordering::SeqCst);
        }
        let _ = self
            .word(64)
            .compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst);
    }
    pub fn read(&self) -> Option<Record> {
        let c = self.word(64).load(Ordering::SeqCst);
        if !(1..=3).contains(&c) {
            return None;
        }
        let at = 128 + (c as usize - 1) * 256;
        let r = Record {
            words: std::array::from_fn(|i| self.word(at + i * 8).load(Ordering::SeqCst)),
        };
        (r.words[0] == 1
            && r.words[18] == c
            && (1..=3).contains(&r.words[14])
            && (1..=5).contains(&r.words[19])
            && r.words[20..].iter().all(|v| *v == 0)
            && r.words[1].to_le_bytes() == self.session[..8]
            && r.words[2].to_le_bytes() == self.session[8..])
            .then_some(r)
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
    fn fixture() -> (std::path::PathBuf, std::sync::Arc<Status>) {
        let path = std::env::temp_dir().join(format!(
            "if1-{}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        let status = std::sync::Arc::new(Status::create(&path, [17; 16]).unwrap());
        (path, status)
    }
    #[test]
    fn unchanged_idle_context_does_not_publish_but_confirmed_state_does() {
        let (path, s) = fixture();
        let initial: Vec<_> = (1024..BYTES)
            .step_by(8)
            .map(|i| s.word(i).load(Ordering::SeqCst))
            .collect();
        for _ in 0..1000 {
            s.progress([1, 0, 1, 0, 0], Some(0), None);
        }
        assert_eq!(
            initial,
            (1024..BYTES)
                .step_by(8)
                .map(|i| s.word(i).load(Ordering::SeqCst))
                .collect::<Vec<_>>()
        );
        s.progress([7, 2, 104687, 512, 11], Some(512), None);
        assert_eq!(s.word(1024).load(Ordering::SeqCst), 2);
        let snapshot = crate::recovery::Snapshot {
            bytes: vec![],
            revision: 9,
            source: 1,
            generation: 7,
            through: 104687,
            digest: [23; 32],
        };
        s.progress([7, 2, 104687, 512, 11], None, Some(&snapshot));
        assert_eq!(s.word(1024).load(Ordering::SeqCst), 3);
        for _ in 0..1000 {
            s.progress([7, 2, 104687, 512, 11], Some(512), Some(&snapshot));
        }
        assert_eq!(s.word(1024).load(Ordering::SeqCst), 3);
        assert_eq!(&s.context().unwrap().words[7..10], &[9, 7, 104687]);
        drop(s);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn concurrent_progress_read_exhaustion_is_retryable() {
        use std::{sync::mpsc, time::Duration};
        let (path, s) = fixture();
        let (request, requests) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let writer = s.clone();
        let task = std::thread::spawn(move || {
            for round in 0..3 {
                requests.recv_timeout(Duration::from_secs(5)).unwrap();
                // Recycle both slots between copy and validation, not just one.
                for step in 1..=2 {
                    let n = round * 2 + step;
                    writer.progress([7, 2, 104687 + n, n * 256, 11], Some(n * 256), None);
                }
                done.send(()).unwrap();
            }
        });
        assert_eq!(
            s.context_after_copy(|| {
                request.send(()).unwrap();
                completed.recv_timeout(Duration::from_secs(5)).unwrap();
            }),
            None
        );
        task.join().unwrap();
        assert!(s.read().is_none());
        s.fail_native(3);
        let first = s.read().unwrap();
        assert_eq!(&first.words[3..7], &[7, 2, 104693, 1536]);
        assert_eq!(first.words[16], 1536);
        s.fail_native(99);
        assert_eq!(s.read(), Some(first));
        drop(s);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn first_complete_survives_interrupted_other_producer() {
        let p = std::env::temp_dir().join(format!(
            "if1-{}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        let s = Status::create(&p, [17; 16]).unwrap();
        assert_eq!(s.read(), None);
        s.progress([2, 3, 104687, 512, 11], Some(512), None);
        // A killed Windows writer partially filled its own slot but never committed.
        s.word(384).store(999, Ordering::SeqCst);
        s.fail_native(3);
        let first = s.read().unwrap();
        assert_eq!(&first.words[3..7], &[2, 3, 104687, 512]);
        assert_eq!(first.words[14], 3);
        s.fail_native(99);
        assert_eq!(s.read(), Some(first));
        // Partial future progress is not a complete state or context publication.
        let c = s.word(1024).load(Ordering::SeqCst);
        s.word(1088 + ((c as usize + 1) & 1) * 256)
            .store(999, Ordering::SeqCst);
        assert_eq!(s.context().unwrap().words[0], 1);
        drop(s);
        std::fs::remove_file(p).unwrap();
    }
}

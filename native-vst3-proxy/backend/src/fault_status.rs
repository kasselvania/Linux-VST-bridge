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
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
const BYTES: usize = 1024;
const LANES: usize = 3;
const LANE_BYTES: usize = 320;
const SLOT_BYTES: usize = 128;
const MAX_WORDS: usize = SLOT_BYTES / 8;
unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
struct View {
    pointer: NonNull<u8>,
    _file: File,
}
unsafe impl Send for View {}
unsafe impl Sync for View {}
impl View {
    fn word(&self, offset: usize) -> &AtomicU64 {
        unsafe { &*self.pointer.as_ptr().add(offset).cast::<AtomicU64>() }
    }
}
impl Drop for View {
    fn drop(&mut self) {
        unsafe {
            munmap(self.pointer.as_ptr().cast(), BYTES);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stability {
    Absent,
    Stable,
    Unstable,
}
impl Stability {
    pub fn name(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Stable => "stable",
            Self::Unstable => "unstable",
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Lane {
    pub stability: Stability,
    pub attempts: u32,
    pub publication_before: u64,
    pub publication_after: u64,
    pub words: [u64; MAX_WORDS],
    pub word_count: usize,
}
impl Lane {
    const fn empty(word_count: usize) -> Self {
        Self {
            stability: Stability::Absent,
            attempts: 0,
            publication_before: 0,
            publication_after: 0,
            words: [0; MAX_WORDS],
            word_count,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub lanes: [Lane; LANES],
}

pub struct Reader {
    view: Arc<View>,
}
unsafe impl Send for Reader {}
unsafe impl Sync for Reader {}
impl Reader {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            lanes: std::array::from_fn(|index| {
                let words = if index == 0 { 16 } else { 10 };
                read_lane(index, words, |offset| {
                    self.view.word(offset).load(Ordering::SeqCst)
                })
            }),
        }
    }
}

fn read_lane(
    index: usize,
    word_count: usize,
    mut load: impl FnMut(usize) -> u64,
) -> Lane {
    let lane = 64 + index * LANE_BYTES;
    let mut observed = Lane::empty(word_count);
    for attempt in 1..=3 {
        let before = load(lane);
        if before == 0 {
            let after = load(lane);
            observed.attempts = attempt;
            observed.publication_before = before;
            observed.publication_after = after;
            if after == 0 {
                return observed;
            }
            continue;
        }
        let slot = lane + 64 + (before as usize & 1) * SLOT_BYTES;
        let mut words = [0; MAX_WORDS];
        for (offset, word) in words[..word_count].iter_mut().enumerate() {
            *word = load(slot + offset * 8);
        }
        let after = load(lane);
        observed.attempts = attempt;
        observed.publication_before = before;
        observed.publication_after = after;
        if before == after {
            observed.stability = Stability::Stable;
            observed.words = words;
            return observed;
        }
    }
    observed.stability = Stability::Unstable;
    observed
}

pub struct Status {
    pub terminal: std::sync::Arc<crate::terminal::Status>,
    view: Arc<View>,
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
        let Some(pointer) = NonNull::new(p.cast()) else {
            unsafe { munmap(p, BYTES); }
            return Err(invalid("null fault status"));
        };
        // Own the mapping immediately. Any later terminal-status setup error
        // now drops the map instead of leaking it.
        let view = Arc::new(View { pointer, _file: file });
        let mut header = [0u8; 64];
        header[..4].copy_from_slice(b"LVFS");
        header[4..8].copy_from_slice(&2u32.to_le_bytes());
        header[8..12].copy_from_slice(&(BYTES as u32).to_le_bytes());
        header[16..32].copy_from_slice(&session);
        unsafe {
            std::ptr::copy_nonoverlapping(header.as_ptr(), view.pointer.as_ptr(), 64);
        }
        // The immutable header is complete before any Reader can be exposed.
        let terminal = std::sync::Arc::new(crate::terminal::Status::create(
            &path.with_file_name("if1.terminal"),
            session,
        )?);
        Ok(Self {
            terminal,
            view,
            counter: 0,
            generation: 1,
            delivery: [0; 6],
        })
    }
    fn word(&self, offset: usize) -> &AtomicU64 {
        self.view.word(offset)
    }
    pub fn reader(&self) -> Reader {
        Reader {
            view: self.view.clone(),
        }
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
            gui:None,gui_revision:0,mailbox:Some(mailbox::Mailbox::create(&dir.join("ap10.delivery"),id).unwrap()),mailbox_enabled:true,
            #[cfg(target_os="linux")] direct_requested:false,notifications:None,configured_mode:0,capture:None,fault_status:Some(status),
            notices:(0,0),returned:Default::default(),processing:crate::ProcessingScratch::new(),mapping:Some(Mapping::new(&dir.join("ap1.audio")).unwrap()),socket,
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

    #[test]
    fn retained_reader_outlives_writer_and_distinguishes_lane_states() {
        let dir=std::env::temp_dir().join(format!("ap12-view-{:032x}",u128::from_le_bytes(mapping::random().unwrap())));
        std::fs::create_dir(&dir).unwrap();
        let path=dir.join("ap12.status");
        let mut status=Status::create(&path,[13;16]).unwrap();
        status.publish(7,11,19,3,23);
        let reader=status.reader();
        drop(status);
        std::fs::remove_file(&path).unwrap();
        let snapshot=reader.snapshot();
        assert_eq!(snapshot.lanes[0].stability,Stability::Stable);
        assert_eq!(&snapshot.lanes[0].words[..6],&[1,7,11,19,3,23]);
        assert_eq!(snapshot.lanes[1].stability,Stability::Absent);
        drop(reader);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bounded_reader_never_presents_an_advancing_lane_as_stable() {
        let lane=64+LANE_BYTES;
        let mut publication=0;
        let unstable=read_lane(1,10,|offset| {
            if offset==lane { publication+=1; publication } else { 99 }
        });
        assert_eq!(unstable.stability,Stability::Unstable);
        assert_eq!(unstable.attempts,3);
        assert_ne!(unstable.publication_before,unstable.publication_after);

        let absent=read_lane(2,10,|_|0);
        assert_eq!(absent.stability,Stability::Absent);
        assert_eq!(absent.attempts,1);
    }
}

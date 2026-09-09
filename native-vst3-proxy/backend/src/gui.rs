//! AP11 UI companion: two bounded owner-thread queues, independent of delivery.
//! Audio only obtains an atomic revision when admitting parameter changes.
use ap1_native_client::{invalid, need};
use std::{
    ffi::c_void,
    fs::{File, OpenOptions},
    io,
    os::unix::{fs::OpenOptionsExt, io::AsRawFd},
    path::Path,
    ptr::NonNull,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Mutex,
    },
};

pub const CAPACITY: u64 = 512;
const HEADER: usize = 320;
const MESSAGE: usize = 608;
const BYTES: usize = HEADER + 2 * CAPACITY as usize * MESSAGE;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Message {
    pub abi_version: u32,
    pub extent: u32,
    pub kind: u32,
    pub id: u32,
    pub revision: u64,
    pub value: f64,
    pub flags: i32,
    pub steps: i32,
    pub count: u32,
    pub result: u32,
    pub title: [u16; 128],
    pub units: [u16; 128],
    pub activation: u64,
    pub user_time: u32,
    pub requestor_x11: u32,
    pub target_x11: u32,
    pub view_epoch: u32,
    pub focus_result: u32,
    pub focus_flags: u32,
    pub native_view: u64,
    pub lifecycle: u32,
    pub reserved: u32,
}
impl Default for Message {
    fn default() -> Self {
        Self {
            abi_version: 4,
            extent: MESSAGE as u32,
            kind: 0,
            id: 0,
            revision: 0,
            value: 0.,
            flags: 0,
            steps: 0,
            count: 0,
            result: 0,
            title: [0; 128],
            units: [0; 128],
            activation: 0,
            user_time: 0,
            requestor_x11: 0,
            target_x11: 0,
            view_epoch: 0,
            focus_result: 0,
            focus_flags: 0,
            native_view: 0,
            lifecycle: 0,
            reserved: 0,
        }
    }
}
impl Message {
    // The caller must supply at least this fixed eight-byte prefix. Reject an
    // older/short extent before forming a reference to the new full structure.
    pub unsafe fn valid_prefix(message: *const Self) -> bool {
        !message.is_null()
            && message.cast::<u32>().read_unaligned() == 4
            && message.cast::<u32>().add(1).read_unaligned() == MESSAGE as u32
    }
    fn valid(&self) -> bool {
        self.abi_version == 4 && self.extent == MESSAGE as u32 && self.reserved == 0
    }
}
const _: () = assert!(std::mem::size_of::<Message>() == MESSAGE);
const _: () = assert!(std::mem::offset_of!(Message, extent) == 4);
const _: () = assert!(std::mem::offset_of!(Message, activation) == 560);
const _: () = assert!(std::mem::offset_of!(Message, native_view) == 592);
const _: () = assert!(std::mem::offset_of!(Message, lifecycle) == 600);
unsafe extern "C" {
    fn mmap(a: *mut c_void, n: usize, p: i32, f: i32, fd: i32, o: i64) -> *mut c_void;
    fn munmap(a: *mut c_void, n: usize) -> i32;
}
pub struct Gui {
    pointer: NonNull<u8>,
    _file: File,
    ui: Mutex<()>,
}
// Each direction has one producer/consumer. Native UI operations are serialized
// independently of audio; slots are published/reclaimed with release/acquire.
unsafe impl Send for Gui {}
unsafe impl Sync for Gui {}
impl Gui {
    pub fn create(path: &Path, session: [u8; 16]) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        file.set_len(BYTES as u64)?;
        let p = unsafe { mmap(std::ptr::null_mut(), BYTES, 3, 1, file.as_raw_fd(), 0) };
        need(p as isize != -1, "GUI mapping failed")?;
        let out = Self {
            pointer: NonNull::new(p.cast()).ok_or_else(|| invalid("null GUI mapping"))?,
            _file: file,
            ui: Mutex::new(()),
        };
        unsafe {
            std::ptr::write_bytes(out.pointer.as_ptr(), 0, BYTES);
        }
        out.write(0, b"LVBU");
        out.write(4, &5u32.to_le_bytes());
        out.write(8, &(BYTES as u32).to_le_bytes());
        out.write(12, &(MESSAGE as u32).to_le_bytes());
        out.write(16, &session);
        out.write(32, &(CAPACITY as u32).to_le_bytes());
        out.word(96).store(1, Ordering::Release);
        Ok(out)
    }
    fn flag(&self, n: usize) -> &AtomicU32 {
        unsafe { &*self.pointer.as_ptr().add(n).cast() }
    }
    fn word(&self, n: usize) -> &AtomicU64 {
        unsafe { &*self.pointer.as_ptr().add(n).cast() }
    }
    fn write(&self, n: usize, b: &[u8]) {
        assert!(n + b.len() <= BYTES);
        unsafe {
            std::ptr::copy_nonoverlapping(b.as_ptr(), self.pointer.as_ptr().add(n), b.len());
        }
    }
    pub fn revision(&self) -> u64 {
        let n = self.word(96).fetch_add(1, Ordering::AcqRel);
        if n == u64::MAX {
            self.fail(2);
            0
        } else {
            n
        }
    }
    pub fn shutdown(&self) {
        self.flag(104).store(1, Ordering::Release);
    }
    pub fn fail(&self, code: u32) -> u32 {
        if code != 0 {
            let _ = self
                .flag(108)
                .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire);
        }
        let fault = self.flag(108).load(Ordering::Acquire);
        if fault == 0 && self.flag(104).load(Ordering::Acquire) != 0 {
            11
        } else {
            fault
        }
    }
    pub fn capabilities(&self, caps: u32) -> u32 {
        if caps & !7 != 0 {
            return 4;
        }
        self.flag(136).store(caps, Ordering::Release);
        0
    }
    pub fn send(&self, message: &mut Message) -> u32 {
        let Ok(_guard) = self.ui.try_lock() else {
            return 3;
        };
        if self.flag(104).load(Ordering::Acquire) != 0 {
            return 1;
        }
        if !message.valid() {
            return 4;
        }
        // A separate bounded close mailbox survives command backlog. Its
        // seqlock snapshot binds the exact native view and expected vendor
        // epoch; epoch zero cancels an opening owned by that native view only.
        if message.kind == 2 {
            if message.native_view == 0 {
                return 4;
            }
            if message.native_view < self.word(256).load(Ordering::Acquire) {
                return 0; // a retired native view cannot supersede a newer close
            }
            let sequence = self.word(120).load(Ordering::Acquire);
            if sequence > u64::MAX - 2 || sequence & 1 != 0 {
                self.fail(2);
                return 2;
            }
            self.word(120).store(sequence + 1, Ordering::SeqCst);
            self.word(256).store(message.native_view, Ordering::SeqCst);
            self.flag(264).store(message.view_epoch, Ordering::SeqCst);
            self.word(152)
                .store(self.word(64).load(Ordering::Acquire), Ordering::SeqCst);
            self.word(120).store(sequence + 2, Ordering::SeqCst);
            return 0;
        }
        if self.fail(0) != 0 {
            return 2;
        }
        if !matches!(message.kind, 1 | 3 | 4 | 5)
            || (message.kind == 3
                && (!message.value.is_finite() || !(0.0..=1.0).contains(&message.value)))
        {
            return 4;
        }
        if message.kind == 3 {
            message.revision = self.revision();
        }
        self.push(64, 72, HEADER, message)
    }
    fn push(&self, published: usize, consumed: usize, base: usize, message: &Message) -> u32 {
        let p = self.word(published).load(Ordering::Relaxed);
        let c = self.word(consumed).load(Ordering::Acquire);
        if c > p || p - c > CAPACITY || p == u64::MAX {
            self.fail(2);
            return 2;
        }
        if p - c == CAPACITY {
            self.fail(1);
            return 3;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                message as *const Message,
                self.pointer
                    .as_ptr()
                    .add(base + (p % CAPACITY) as usize * MESSAGE)
                    .cast(),
                1,
            );
        }
        self.word(published).store(p + 1, Ordering::Release);
        0
    }
    fn pop(&self, published: usize, consumed: usize, base: usize, message: &mut Message) -> u32 {
        let c = self.word(consumed).load(Ordering::Relaxed);
        let p = self.word(published).load(Ordering::Acquire);
        if c > p || p - c > CAPACITY || c == u64::MAX {
            self.fail(2);
            return 2;
        }
        if p == c {
            message.kind = 0;
            return 0;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.pointer
                    .as_ptr()
                    .add(base + (c % CAPACITY) as usize * MESSAGE)
                    .cast(),
                message as *mut Message,
                1,
            );
        }
        self.word(consumed).store(c + 1, Ordering::Release);
        if !message.valid() {
            self.fail(2);
            return 2;
        }
        0
    }
    pub fn take(&self, message: &mut Message) -> u32 {
        let Ok(_guard) = self.ui.try_lock() else {
            return 3;
        };
        self.pop(80, 88, HEADER + CAPACITY as usize * MESSAGE, message)
    }
}
impl Drop for Gui {
    fn drop(&mut self) {
        self.shutdown();
        unsafe {
            munmap(self.pointer.as_ptr().cast(), BYTES);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Gui, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "ap11-gui-{}-{}",
            std::process::id(),
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        (Gui::create(&path, [17; 16]).unwrap(), path)
    }
    #[test]
    fn bidirectional_bounded_queues_and_close() {
        let (gui, path) = fixture();
        let mut m = Message::default();
        for i in 0..CAPACITY {
            assert_eq!(
                gui.send(&mut Message {
                    kind: 3,
                    id: i as u32,
                    value: 0.25,
                    ..Message::default()
                }),
                0
            );
        }
        for i in 0..CAPACITY {
            assert_eq!(gui.pop(64, 72, HEADER, &mut m), 0);
            assert_eq!(m.id, i as u32);
            assert_eq!(m.revision, i + 1);
        }
        for kind in [101, 102, 103] {
            assert_eq!(
                gui.push(
                    80,
                    88,
                    HEADER + CAPACITY as usize * MESSAGE,
                    &Message {
                        kind,
                        id: 42,
                        value: 0.75,
                        ..Message::default()
                    }
                ),
                0
            );
        }
        for kind in [101, 102, 103] {
            assert_eq!(gui.take(&mut m), 0);
            assert_eq!(m.kind, kind);
            assert_eq!(m.id, 42);
        }
        for _ in 0..CAPACITY {
            assert_eq!(
                gui.send(&mut Message {
                    kind: 1,
                    ..Message::default()
                }),
                0
            );
        }
        assert_eq!(
            gui.send(&mut Message {
                kind: 1,
                ..Message::default()
            }),
            3
        );
        assert_eq!(gui.fail(0), 1);
        assert_eq!(
            gui.send(&mut Message {
                kind: 2,
                native_view: 1,
                ..Message::default()
            }),
            0
        );
        assert_eq!(gui.word(120).load(Ordering::Acquire), 2);
        // Backlog affects this UI channel, not revision admission/audio queues.
        assert!(gui.revision() > 0);
        gui.shutdown();
        assert_eq!(
            gui.send(&mut Message {
                kind: 1,
                ..Message::default()
            }),
            1
        );
        drop(gui);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn invalid_counter_never_reuses_a_live_slot() {
        let (gui, path) = fixture();
        gui.word(72).store(9, Ordering::Release);
        assert_eq!(
            gui.send(&mut Message {
                kind: 1,
                ..Message::default()
            }),
            2
        );
        assert_eq!(gui.word(64).load(Ordering::Acquire), 0);
        drop(gui);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn versioned_close_mailbox_preserves_owner_and_refuses_exhaustion() {
        let (gui, path) = fixture();
        let mut close = Message {
            kind: 2,
            native_view: 7,
            view_epoch: 3,
            ..Message::default()
        };
        close.abi_version = 2;
        assert_eq!(gui.send(&mut close), 4);
        assert_eq!(gui.word(120).load(Ordering::Acquire), 0);
        close.abi_version = 4;
        assert_eq!(gui.send(&mut close), 0);
        assert_eq!(gui.word(120).load(Ordering::Acquire), 2);
        assert_eq!(gui.word(256).load(Ordering::Acquire), 7);
        assert_eq!(gui.flag(264).load(Ordering::Acquire), 3);
        close.native_view = 6;
        close.view_epoch = 2;
        assert_eq!(gui.send(&mut close), 0);
        assert_eq!(gui.word(120).load(Ordering::Acquire), 2);
        assert_eq!(gui.word(256).load(Ordering::Acquire), 7);
        assert_eq!(gui.flag(264).load(Ordering::Acquire), 3);
        gui.word(120).store(u64::MAX - 1, Ordering::Release);
        close.native_view = 8;
        assert_eq!(gui.send(&mut close), 2);
        assert_eq!(gui.word(120).load(Ordering::Acquire), u64::MAX - 1);
        assert_eq!(gui.word(256).load(Ordering::Acquire), 7);
        drop(gui);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn malformed_return_layout_fails_only_gui_channel() {
        let (gui, path) = fixture();
        let m = Message {
            abi_version: 2,
            kind: 108,
            ..Message::default()
        };
        assert_eq!(
            gui.push(80, 88, HEADER + CAPACITY as usize * MESSAGE, &m),
            0
        );
        assert_eq!(gui.take(&mut Message::default()), 2);
        assert_eq!(gui.fail(0), 2);
        drop(gui);
        std::fs::remove_file(path).unwrap();
    }
}

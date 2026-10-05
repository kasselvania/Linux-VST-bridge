//! Offline AP2 session. The caller supplies owned buffers; no DSP exists here.
#[cfg(test)]
mod commercial_tests;
mod context;
#[cfg(target_os = "linux")]
mod descriptor;
mod completion_wait;
mod fault_status;
mod terminal;
mod gui;
mod instances;
mod mailbox;
mod observer;
mod input_observation;
mod output_pool;
mod performance;
mod parameter_curves;
mod preview;
mod process_results;
mod queue;
mod queued;
mod recovery;
mod scheduling;
mod state;
#[cfg(feature = "rpi0")]
pub mod rpi0;
use ap1_native_client::{
    endpoint::{receive_version, send_version, Prepared},
    mapping::{barrier, Mapping},
    *,
};
use std::{
    cell::RefCell,
    io,
    net::TcpStream,
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
// Production callback tests share one finite registry. Serialize its owners,
// while registry contention tests continue using their independent registries.
#[cfg(test)]
pub(crate) fn registry_test() -> std::sync::MutexGuard<'static, ()> {
    static OWNERS: Mutex<()> = Mutex::new(());
    OWNERS.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
#[cfg(test)]
mod allocation_test {
    use std::{alloc::{GlobalAlloc, Layout, System}, cell::Cell};
    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static COUNTS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
    }
    struct Counted;
    #[global_allocator]
    static ALLOCATOR: Counted = Counted;
    unsafe impl GlobalAlloc for Counted {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ACTIVE.try_with(|a| { if a.get() { COUNTS.with(|c| { let mut v=c.get();v[0]+=1;c.set(v); }); } }).ok();
            unsafe { System.alloc(layout) }
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ACTIVE.try_with(|a| { if a.get() { COUNTS.with(|c| { let mut v=c.get();v[2]+=1;c.set(v); }); } }).ok();
            unsafe { System.dealloc(ptr, layout) }
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            ACTIVE.try_with(|a| { if a.get() { COUNTS.with(|c| { let mut v=c.get();v[1]+=1;c.set(v); }); } }).ok();
            unsafe { System.realloc(ptr, layout, size) }
        }
    }
    pub fn measure<T>(f: impl FnOnce() -> T) -> (T, [usize; 3]) {
        ACTIVE.with(|_| {});
        COUNTS.with(|c| c.set([0; 3]));
        ACTIVE.with(|a| a.set(true));
        let result=f();
        ACTIVE.with(|a| a.set(false));
        (result, COUNTS.with(Cell::get))
    }
}
#[cfg(test)]
mod sample_plane_test {
    use std::cell::Cell;
    thread_local! {
        static ACCESSES: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
    }
    pub fn reset() { ACCESSES.with(|value| value.set((0, 0))); }
    pub fn write() { ACCESSES.with(|value| { let (writes, reads) = value.get(); value.set((writes + 1, reads)); }); }
    pub fn read() { ACCESSES.with(|value| { let (writes, reads) = value.get(); value.set((writes, reads + 1)); }); }
    pub fn accesses() -> (usize, usize) { ACCESSES.with(Cell::get) }
}
struct Session {
    gui: Option<std::sync::Arc<gui::Gui>>,
    gui_revision: u64,
    mapping: Option<Mapping>,
    mailbox: Option<mailbox::Mailbox>,
    mailbox_enabled: bool,
    notifications: Option<ap1_native_client::notification::Channel>,
    configured_mode: u32,
    capture: Option<state::Capture>,
    fault_status: Option<fault_status::Status>,
    notices: (u32, u64),
    returned: process_results::Packet,
    processing: ProcessingScratch,
    socket: TcpStream,
    state: ClientState,
    phase: u16,
    max: usize,
    minor: u64,
    epoch: u64,
    position: u64,
    witness: Option<observer::Observer>,
    identity: Option<state::Identity>,
    trace: observer::Trace,
    sample_rate: u32,
    armed: bool,
    owner: Option<preview::Owner>,
}
struct ProcessingScratch {
    transition_request: Frame,
    transition_reply: Frame,
    transition_reader: ap1_native_client::endpoint::IncrementalFrame,
    request: Frame,
    reply: Frame,
    wire: Vec<u8>,
}
impl ProcessingScratch {
    fn new() -> Self {
        Self {
            transition_request: Frame { kind: 0, session: [0; 16], sequence: 0, payload: Vec::with_capacity(8) },
            transition_reply: Frame { kind: 0, session: [0; 16], sequence: 0, payload: Vec::with_capacity(8) },
            transition_reader: ap1_native_client::endpoint::IncrementalFrame::new(),
            request: Frame { kind: PROCESS, session: [0; 16], sequence: 0, payload: Vec::with_capacity(8360) },
            reply: Frame { kind: 0, session: [0; 16], sequence: 0, payload: Vec::with_capacity(10312) },
            wire: Vec::with_capacity(HEADER + 8360),
        }
    }
}
#[derive(Clone, Copy)]
struct PendingStart {
    session: [u8; 16],
    sequence: u64,
    epoch: u64,
    receive_deadline: std::time::Instant,
}
impl PendingStart {
    fn bounded_by(self, deadline: std::time::Instant) -> Self {
        Self { receive_deadline: self.receive_deadline.min(deadline), ..self }
    }
}
#[derive(Clone, Copy)]
enum ProcessAuthority {
    Running,
    PendingStart(PendingStart),
}
// Mapping has no escaping references; the registry serializes every access.
unsafe impl Send for Session {}
static SESSION: Mutex<Option<(u64, Session)>> = Mutex::new(None);
static NEXT: AtomicU64 = AtomicU64::new(1);
thread_local! {static ERROR_DETAIL:RefCell<String>=const{RefCell::new(String::new())};}
fn retain(error: &io::Error) -> i32 {
    ERROR_DETAIL.with(|s| {
        *s.borrow_mut() = format!("{:?}: {}", error.kind(), error)
            .chars()
            .take(384)
            .collect()
    });
    if state::save_refused(error) {
        5
    } else {
        2
    }
}
fn ffi(f: impl FnOnce() -> i32) -> i32 {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| {
        ERROR_DETAIL.with(|s| *s.borrow_mut() = "Rust panic caught at C ABI".into());
        4
    })
}
fn binding(preview: bool) -> io::Result<preview::Binding> {
    if preview
        && std::env::var_os("LVB_AP2_SESSION_DIR").is_none()
        && std::env::var_os("LVB_AP2_SESSION").is_none()
    {
        return preview::discover();
    }
    let path = std::env::var_os("LVB_AP2_SESSION_DIR")
        .ok_or_else(|| invalid("missing private binding"))?;
    let path = PathBuf::from(path);
    need(
        path.is_absolute() && path.is_dir() && !path.is_symlink(),
        "invalid private directory",
    )?;
    let id = std::env::var("LVB_AP2_SESSION").map_err(|_| invalid("missing session"))?;
    need(
        id.len() == 32 && id.bytes().all(|c| c.is_ascii_hexdigit()),
        "session syntax",
    )?;
    let session = std::array::from_fn(|i| u8::from_str_radix(&id[i * 2..i * 2 + 2], 16).unwrap());
    Ok(preview::Binding {
        directory: path,
        session,
        installed_delay: None,
        delivery_mode: performance::DeliveryMode::Buffered,
        owner: None,
    })
}
impl Session {
    fn open(binding: preview::Binding, max: usize, minor: u64) -> io::Result<Self> {
        Self::open_bound(
            &binding.directory,
            binding.session,
            max,
            minor,
            binding.owner,
        )
    }
    fn open_bound(
        path: &std::path::Path,
        id: [u8; 16],
        max: usize,
        minor: u64,
        mut owner: Option<preview::Owner>,
    ) -> io::Result<Self> {
        let gui = if minor >= 10 {
            Some(std::sync::Arc::new(gui::Gui::create(
                &path.join("ap11.ui"),
                id,
            )?))
        } else {
            None
        };
        let mailbox = if minor >= 15 || minor >= 6 && performance::use_mailbox()? {
            Some(mailbox::Mailbox::create(&path.join("ap10.delivery"), id)?)
        } else {
            None
        };
        let fault_status = if minor >= 6 {
            Some(fault_status::Status::create(&path.join("ap12.status"), id)?)
        } else {
            None
        };
        let prepared = Prepared::with_protocol(path, id, if minor >= 13 { MULTI_CHANNELS } else { 2 }, minor)?;
        let (mapping, socket, notifications) =
            prepared.accept_notified_while(minor, || preview::check_owner(&mut owner))?;
        let mut s = Self {
            gui,
            gui_revision: 0,
            mapping: Some(mapping),
            mailbox,
            mailbox_enabled: false,
            notifications,
            configured_mode: 0,
            capture: None,
            fault_status,
            notices: (0, 0),
            returned: process_results::Packet::default(),
            processing: ProcessingScratch::new(),
            socket,
            state: ClientState {
                session: id,
                next: 1,
                slot: Slot::Writable,
            },
            phase: 8,
            max,
            minor,
            epoch: 0,
            position: 0,
            witness: if matches!(minor, 5 | 7 | 8 | 9 | 10 | 11 | 12 | 13)
                || (minor >= 14 && observer::delivery_enabled())
            {
                observer::Observer::commercial().ok()
            } else if matches!(minor, 4 | 6)
                && (owner.is_some() || std::env::var("LVB_AP4_COMPARE").as_deref() == Ok("1"))
            {
                observer::Observer::new().ok()
            } else {
                None
            },
            owner,
            trace: observer::Trace::default(),
            sample_rate: 48000,
            armed: false,
            identity: None,
        };
        if minor >= 4 {
            s.phase = 17;
            return Ok(s);
        }
        let mut payload = (max as u32).to_le_bytes().to_vec();
        if minor == 3 {
            payload.extend_from_slice(&0u32.to_le_bytes());
        } // SDK kRealtime
        s.exchange(8, payload)?;
        s.phase = 9;
        Ok(s)
    }
    fn exchange(&mut self, kind: u16, payload: Vec<u8>) -> io::Result<Frame> {
        need(self.phase != ERROR, "failed session")?;
        let f = Frame {
            kind,
            session: self.state.session,
            sequence: self.state.next,
            payload,
        };
        let result = self
            .send_control(&f)
            .and_then(|_| receive_version(&mut self.socket, 10, self.minor));
        match result {
            Ok(reply)
                if reply.kind == kind + 1
                    && reply.session == f.session
                    && reply.sequence == f.sequence
                    && (kind == 20 && self.minor >= 6 && reply.payload.len() == 16
                        || reply.payload.is_empty()
                            && kind != 20
                            && !(self.minor >= 3 && matches!(kind, 10 | 12))
                        || self.minor >= 3
                            && matches!(kind, 10 | 12)
                            && reply.payload == f.payload) =>
            {
                Ok(reply)
            }
            Ok(_reply) => {
                #[cfg(test)]
                eprintln!("LC1 response expected_kind={} actual_kind={} expected_sequence={} actual_sequence={} expected_epoch={:?} actual_epoch={:?} session_match={} native_next={} phase={}",kind+1,_reply.kind,f.sequence,_reply.sequence,matches!(kind,10|12).then(||f.payload.get(..8).map(ap1_native_client::get)).flatten(),matches!(_reply.kind,11|13).then(||_reply.payload.get(..8).map(ap1_native_client::get)).flatten(),_reply.session==f.session,self.state.next,self.phase);
                self.phase = ERROR;
                self.state.failed();
                Err(invalid("wrong lifecycle response"))
            }
            Err(error) => {
                self.phase = ERROR;
                self.state.failed();
                Err(error)
            }
        }
    }
    fn send_control(&mut self, frame: &Frame) -> io::Result<()> {
        send_version(&mut self.socket, frame, 5, self.minor)?;
        if self.mailbox_enabled && self.phase == 11 {
            self.mailbox
                .as_mut()
                .ok_or_else(|| invalid("delivery mapping absent"))?
                .control()?;
            if let Some(channel) = &mut self.notifications { channel.notify()?; }
        }
        Ok(())
    }
    fn wait_control_handoff(&mut self, end: std::time::Instant,
        mut cancelled: impl FnMut() -> bool) -> io::Result<()> {
        while self.mailbox.as_ref().is_some_and(|mailbox| mailbox.control_handoff_pending()) {
            need(!cancelled(), "audio operation cancelled")?;
            need(std::time::Instant::now() < end, "control handoff deadline")?;
            preview::check_owner(&mut self.owner)?;
            mailbox::peer_status(&self.socket, self.capture.is_some())?;
            if let Some(capture) = &mut self.capture { capture.service(&self.socket, self.minor)?; }
            if let Some(channel) = &mut self.notifications { channel.service(end)?; }
            else { std::thread::sleep(std::time::Duration::from_micros(50)); }
        }
        Ok(())
    }
    fn worker_wait_fds(&self) -> [(i32, bool); 2] {
        use std::os::unix::io::AsRawFd;
        [(if self.capture.as_ref().is_some_and(|capture| capture.is_unread()) { self.socket.as_raw_fd() } else { -1 }, false),
            self.notifications.as_ref().map_or((-1, false), |channel| (channel.raw_fd(), channel.pending_wake()))]
    }
    fn service_worker_wakes(&mut self, _end: std::time::Instant, mut cancelled: impl FnMut() -> bool) -> io::Result<()> {
        need(!cancelled(), "audio operation cancelled")?;
        preview::check_owner(&mut self.owner)?;
        mailbox::peer_status(&self.socket, self.capture.is_some())?;
        if let Some(channel) = &mut self.notifications { channel.service(std::time::Instant::now())?; }
        Ok(())
    }
    fn configure(&mut self, mut bytes: Vec<u8>) -> io::Result<Vec<u8>> {
        need(
            self.minor >= 6 && matches!(self.phase, 17 | 15),
            "setup requires inactive session",
        )?;
        performance::validate_wire(&bytes)?;
        need(get(&bytes[..4]) <= if self.minor >= 14 { BLOCK_CAP as u64 } else { CAP as u64 },
             "setup exceeds negotiated mapping")?;
        self.sample_rate = f64::from_le_bytes(bytes[8..16].try_into().unwrap()) as u32;
        if self.minor >= 13 {
            let channels = performance::output_channels(&bytes)?;
            self.mapping.as_mut().ok_or_else(|| invalid("mapping absent"))?.output_channels = channels;
        }
        let mailbox_version = 3 * u64::from(self.mailbox.is_some());
        put(&mut bytes[16..20], mailbox_version);
        let configured_mode = get(&bytes[4..8]) as u32;
        let reply = self.exchange(20, bytes)?;
        need(
            get(&reply.payload[12..16]) == mailbox_version
                && matches!(get(&reply.payload[8..12]), 1 | 3),
            "invalid setup response",
        )?;
        self.mailbox_enabled = mailbox_version == 3;
        self.configured_mode = configured_mode;
        Ok(reply.payload)
    }
    fn activate(&mut self, maximum: usize, mode: u32) -> io::Result<()> {
        need(
            self.minor >= 4
                && matches!(self.phase, 17 | 15)
                && (1..=if self.minor >= 14 { BLOCK_CAP } else { CAP }).contains(&maximum)
                && if self.minor >= 6 {
                    if self.minor >= 15 { mode <= 2 } else { matches!(mode, 0 | 2) }
                } else {
                    mode <= 1
                },
            "state session activation",
        )?;
        self.exchange(
            8,
            [(maximum as u32).to_le_bytes(), mode.to_le_bytes()].concat(),
        )?;
        self.max = maximum;
        self.phase = 9;
        Ok(())
    }
    fn transition(&mut self, op: u16) -> io::Result<()> {
        need(
            matches!((self.phase, op), (9, 10) | (11, 12) | (13, 14) | (9, 14)),
            "lifecycle order",
        )?;
        self.exchange(op, vec![])?;
        self.phase = op + 1;
        Ok(())
    }
    fn transition_epoch(&mut self, op: u16, epoch: u64) -> io::Result<()> {
        need(
            self.minor >= 3
                && match op {
                    10 => (self.phase == 9 || self.phase == 13) && epoch == self.epoch + 1,
                    12 => self.phase == 11 && epoch == self.epoch,
                    _ => false,
                },
            "queued lifecycle epoch/order",
        )?;
        self.exchange(op, epoch.to_le_bytes().to_vec())?;
        if op == 10 {
            self.epoch = epoch;
            self.position = 0;
        }
        self.phase = op + 1;
        Ok(())
    }
    fn can_overlap_start(&self) -> bool {
        self.minor == 15
            && self.mailbox_enabled
            && self.mailbox.is_some()
            && self.notifications.is_some()
    }
    fn begin_start(&mut self, epoch: u64) -> io::Result<PendingStart> {
        need(
            self.can_overlap_start()
                && matches!(self.phase, 9 | 13)
                && epoch == self.epoch + 1
                && self.state.slot == Slot::Writable
                && self.capture.is_none()
                && self.mailbox.as_ref().is_some_and(|mailbox| !mailbox.control_handoff_pending()),
            "pending Start ownership/order",
        )?;
        self.processing.transition_reader.reset();
        let request = &mut self.processing.transition_request;
        request.kind = 10;
        request.session = self.state.session;
        request.sequence = self.state.next;
        request.payload.clear();
        request.payload.extend_from_slice(&epoch.to_le_bytes());
        ap1_native_client::endpoint::send_version_with(
            &mut self.socket,
            request,
            5,
            self.minor,
            &mut self.processing.wire,
        )?;
        Ok(PendingStart {
            session: request.session,
            sequence: request.sequence,
            epoch,
            receive_deadline: std::time::Instant::now() + std::time::Duration::from_secs(10),
        })
    }
    fn pending_start_wait_fds(&self) -> [(i32, bool); 2] {
        use std::os::fd::AsRawFd;
        [(self.socket.as_raw_fd(), false), (-1, false)]
    }
    fn finish_start(
        &mut self,
        pending: PendingStart,
        mut cancelled: impl FnMut() -> bool,
    ) -> io::Result<()> {
        let result = ap1_native_client::endpoint::receive_incremental_until_while(
            &mut self.socket,
            pending.receive_deadline,
            self.minor,
            &mut self.processing.transition_reader,
            &mut self.processing.transition_reply,
            || need(!cancelled(), "pending Start cancelled"),
        );
        let valid = result.and_then(|_| self.commit_start(pending));
        if let Err(error) = valid {
            self.phase = ERROR;
            self.state.failed();
            return Err(error);
        }
        Ok(())
    }
    fn commit_start(&mut self, pending: PendingStart) -> io::Result<()> {
        {
            let reply = &self.processing.transition_reply;
            need(
                reply.kind == 11
                    && reply.session == pending.session
                    && reply.sequence == pending.sequence
                    && reply.payload == pending.epoch.to_le_bytes(),
                "wrong pending Started response",
            )?;
        }
        self.epoch = pending.epoch;
        self.position = 0;
        self.phase = 11;
        Ok(())
    }
    fn poll_start(&mut self, pending: PendingStart) -> io::Result<bool> {
        let result = (|| {
            need(std::time::Instant::now() < pending.receive_deadline,
                "pending Start deadline")?;
            let complete = self.processing.transition_reader.read_available(
                &self.socket, self.minor, &mut self.processing.transition_reply)?;
            if !complete { return Ok(false); }
            need(std::time::Instant::now() < pending.receive_deadline,
                "pending Start deadline")?;
            self.commit_start(pending)?;
            Ok(true)
        })();
        if let Err(error) = result {
            self.phase = ERROR;
            self.state.failed();
            return Err(error);
        }
        result
    }
    #[allow(clippy::too_many_arguments)]
    fn process_positioned(
        &mut self,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
        timeline: (u64, u64),
        events: &[events::Event],
        context: context::Context,
        process_mode: u32,
        deadline: std::time::Instant,
        cancelled: impl FnMut() -> bool,
        capture_completed: impl FnMut(io::Result<Vec<u8>>) -> io::Result<()>,
    ) -> io::Result<([[u32; BLOCK_CAP + 2]; 2], u64)> {
        need(
            self.minor >= 3 && timeline == (self.epoch, self.position),
            "queued audio epoch/position",
        )?;
        self.process_events_with(n, gain, silence, input, events, context, process_mode, deadline, cancelled, capture_completed)
    }
    #[allow(clippy::too_many_arguments)]
    fn process_positioned_pending_start(
        &mut self,
        pending: PendingStart,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
        timeline: (u64, u64),
        events: &[events::Event],
        context: context::Context,
        process_mode: u32,
        deadline: std::time::Instant,
        cancelled: impl FnMut() -> bool,
        capture_completed: impl FnMut(io::Result<Vec<u8>>) -> io::Result<()>,
        started: impl FnMut(),
    ) -> io::Result<([[u32; BLOCK_CAP + 2]; 2], u64)> {
        need(timeline == (pending.epoch, 0), "pending Start audio epoch/position")?;
        self.process_events_with_authority(
            ProcessAuthority::PendingStart(pending),
            n,
            gain,
            silence,
            input,
            events,
            context,
            process_mode,
            deadline,
            cancelled,
            capture_completed,
            started,
        )
    }
    fn process(
        &mut self,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
    ) -> io::Result<([[u32; BLOCK_CAP + 2]; 2], u64)> {
        self.process_events(n, gain, silence, input, &[], context::Context::default())
    }
    fn process_events(
        &mut self,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
        events: &[events::Event],
        context: context::Context,
    ) -> io::Result<([[u32; BLOCK_CAP + 2]; 2], u64)> {
        self.process_events_with(n, gain, silence, input, events, context, self.configured_mode,
            std::time::Instant::now() + std::time::Duration::from_secs(5), || false, |_| Ok(()))
    }
    #[allow(clippy::too_many_arguments)]
    fn process_events_with(
        &mut self, n: usize, gain: f64, silence: u64, input: [&[f32]; 2],
        events: &[events::Event], context: context::Context, process_mode: u32,
        deadline: std::time::Instant, cancelled: impl FnMut() -> bool,
        capture_completed: impl FnMut(io::Result<Vec<u8>>) -> io::Result<()>,
    ) -> io::Result<([[u32; BLOCK_CAP + 2]; 2], u64)> {
        self.process_events_with_authority(
            ProcessAuthority::Running,
            n,
            gain,
            silence,
            input,
            events,
            context,
            process_mode,
            deadline,
            cancelled,
            capture_completed,
            || {},
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn process_events_with_authority(
        &mut self, authority: ProcessAuthority,
        n: usize, gain: f64, silence: u64, input: [&[f32]; 2],
        events: &[events::Event], context: context::Context, process_mode: u32,
        deadline: std::time::Instant, mut cancelled: impl FnMut() -> bool,
        mut capture_completed: impl FnMut(io::Result<Vec<u8>>) -> io::Result<()>,
        mut started: impl FnMut(),
    ) -> io::Result<([[u32; BLOCK_CAP + 2]; 2], u64)> {
        need(!cancelled(), "audio operation cancelled")?;
        need(self.minor < 15 || process_mode <= 2 && (process_mode == 2) == (self.configured_mode == 2),
            "process mode requires inactive setup")?;
        need(
            matches!(self.minor, 5 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15) || events.is_empty(),
            "events require negotiated protocol",
        )?;
        need(
            !matches!(self.minor, 5 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15) || gain.is_nan(),
            "commercial legacy gain refused",
        )?;
        let (operation_epoch, operation_position) = match authority {
            ProcessAuthority::Running => {
                need(self.phase == 11, "process requires Started")?;
                (self.epoch, self.position)
            }
            ProcessAuthority::PendingStart(pending) => {
                need(
                    self.can_overlap_start()
                        && matches!(self.phase, 9 | 13)
                        && pending.session == self.state.session
                        && pending.sequence == self.state.next
                        && pending.epoch == self.epoch + 1
                        && self.capture.is_none(),
                    "pending Start process authority",
                )?;
                (pending.epoch, 0)
            }
        };
        need(
            self.state.slot == Slot::Writable
                && (self.minor >= 3 || self.state.next <= 64)
                && (n > 0 || self.minor >= 4)
                && n <= self.max
                && silence <= 3,
            "process state/extent",
        )?;
        need(
            (self.minor >= 4 && gain.is_nan()) || gain.is_finite() && (0.0..=1.0).contains(&gain),
            "gain",
        )?;
        self.armed |= self.minor == 6
            || input.iter().any(|p| p.iter().any(|&v| v != 0.))
            || events.iter().any(|e| e.kind == events::NOTE_ON);
        let first_note = events.iter().find(|e| e.kind == events::NOTE_ON);
        self.trace = observer::Trace {
            sample_rate: self.sample_rate,
            armed: self.armed,
            epoch: operation_epoch,
            position: operation_position,
            frames: n as u64,
            event_count: events.len() as u32,
            note_on_count: events.iter().filter(|e| e.kind == events::NOTE_ON).count() as u32,
            note_offset: first_note.map_or(0, |e| e.offset),
            note_id: first_note.map_or(0, |e| e.id),
            note_pitch: first_note.map_or(-1, |e| e.pitch),
            note_channel: first_note.map_or(-1, |e| e.channel),
            sequence: self.state.next,
            started: Some(std::time::Instant::now()),
            ..Default::default()
        };
        let capacity = self.mapping.as_ref().ok_or_else(|| invalid("mapping absent"))?.capacity;
        need(n <= capacity, "process exceeds negotiated mapping")?;
        // A zero-frame operation still carries events, parameters, identity and
        // an exact completion. It exposes no sample planes to the processor, so
        // do not prepare or touch the mapped sample payload for that operation.
        let sample_planes = if n == 0 {
            None
        } else {
            let mut snapshot = [[0u32; BLOCK_CAP + 2]; 2];
            let mut poison = [POISON; BLOCK_CAP + 2];
            poison[0] = GUARD;
            poison[capacity + 1] = GUARD;
            for ch in 0..2 {
                snapshot[ch][0] = GUARD;
                snapshot[ch][capacity + 1] = GUARD;
                for i in 0..n {
                    need(input[ch][i].is_finite(), "nonfinite input")?;
                    snapshot[ch][i + 1] = input[ch][i].to_bits();
                }
            }
            Some((snapshot, poison))
        };
        self.returned.events = 0;
        self.returned.points = 0;
        self.returned.bytes = 0;
        let (mapping_output, mapping_stride, output_channels) = {
            let map = self.mapping.as_ref().ok_or_else(|| invalid("mapping absent"))?;
            (map.output, map.stride, map.output_channels)
        };
        let result = (|| {
            if let Some((snapshot, poison)) = &sample_planes {
                let map = self
                    .mapping
                    .as_mut()
                    .ok_or_else(|| invalid("mapping absent"))?;
                for (ch, plane) in snapshot.iter().enumerate() {
                    #[cfg(test)]
                    sample_plane_test::write();
                    map.write_plane(INPUT, ch, plane)?;
                }
                for ch in 0..output_channels {
                    #[cfg(test)]
                    sample_plane_test::write();
                    map.write_plane(map.output, ch, poison)?;
                }
                barrier();
            }
            let request = &mut self.processing.request;
            if self.minor >= 4 {
                request.payload.clear();
                request.payload.resize(32, 0);
                let payload = &mut request.payload;
                for (offset, value) in [
                    (0, n as u64),
                    (4, INPUT as u64),
                    (8, mapping_output as u64),
                    (12, mapping_stride as u64),
                    (24, silence),
                    (28, u64::from(!gain.is_nan())),
                ] {
                    put(&mut payload[offset..offset + 4], value);
                }
                payload[16..24]
                    .copy_from_slice(&if gain.is_nan() { 0f64 } else { gain }.to_le_bytes());
                self.state.slot = Slot::Outstanding {
                    sequence: self.state.next,
                    frames: n,
                };
                request.kind = PROCESS;
                request.session = self.state.session;
                request.sequence = self.state.next;
            } else if self.minor >= 3 {
                *request = self.state.process_sustained(n, gain, silence as u32)?;
            } else {
                *request = self.state.process(n, gain, silence as u32)?;
            }
            if self.minor >= 3 {
                request.payload.extend_from_slice(&operation_epoch.to_le_bytes());
                request
                    .payload
                    .extend_from_slice(&operation_position.to_le_bytes());
            }
            if matches!(self.minor, 5 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15) {
                events::encode_for_block(events, n, &mut request.payload, self.minor >= 14)?;
            }
            if self.minor >= 8 {
                context.encode_into(&mut request.payload);
            }
            if self.minor >= 10 {
                request.payload.extend(self.gui_revision.to_le_bytes());
            }
            if self.minor >= 15 {
                request.payload.extend(process_mode.to_le_bytes());
                request.payload.extend(0u32.to_le_bytes());
            }
            self.trace.prepared = Some(std::time::Instant::now());
            if let Some(s) = &mut self.fault_status {
                s.publish(operation_epoch, self.state.next, operation_position, 1, 0);
            }
            if self.mailbox_enabled {
                need(!cancelled(), "audio operation cancelled before publication")?;
                need(std::time::Instant::now() < deadline, "audio containment expired before publication")?;
                self.mailbox
                    .as_mut()
                    .ok_or_else(|| invalid("delivery mapping absent"))?
                    .send(request, self.minor)?;
                if let Some(channel) = &mut self.notifications { channel.notify()?; }
                self.trace.sent = Some(std::time::Instant::now());
                if let Some(s) = &mut self.fault_status {
                    s.publish(operation_epoch, self.state.next, operation_position, 2, 0);
                }
                if let ProcessAuthority::PendingStart(pending) = authority {
                    self.finish_start(pending, &mut cancelled)?;
                    started();
                }
                let mailbox = self
                    .mailbox
                    .as_mut()
                    .ok_or_else(|| invalid("delivery mapping absent"))?;
                let mut healthy = || {
                    need(!cancelled(), "audio operation cancelled")?;
                    preview::check_owner(&mut self.owner)?;
                    mailbox::peer_status(&self.socket, self.capture.is_some())?;
                    if let Some(capture) = &mut self.capture {
                        if let Some(reply) = capture.poll(&self.socket, self.minor)? {
                            let request = Frame { kind: 16, session: self.state.session, sequence: capture.sequence, payload: vec![] };
                            let result = state::validate_reply(self.minor, &mut self.witness, &request, reply)
                                .and_then(|payload| state::bound_envelope(self.identity, &payload));
                            let fatal = result.as_ref().is_err_and(|error| !state::save_refused(error));
                            self.capture = None;
                            capture_completed(result)?;
                            need(!fatal, "state capture failed; original result retained")?;
                        }
                    }
                    Ok(())
                };
                if let Some(channel) = &mut self.notifications {
                    mailbox.receive_notified_into(
                        self.minor, deadline, &mut healthy, &mut self.processing.reply, channel)?;
                } else {
                    need(self.minor < 15, "paired notification channel absent")?;
                    mailbox.receive_while_into(
                        self.minor, deadline, &mut healthy, &mut self.processing.reply)?;
                }
                self.trace.windows = mailbox.diagnostic;
            } else {
                need(!cancelled(), "audio operation cancelled before publication")?;
                need(std::time::Instant::now() < deadline, "audio containment expired before publication")?;
                ap1_native_client::endpoint::send_version_with(&mut self.socket, request, 5, self.minor, &mut self.processing.wire)?;
                self.trace.sent = Some(std::time::Instant::now());
                if let Some(s) = &mut self.fault_status {
                    s.publish(operation_epoch, self.state.next, operation_position, 2, 0);
                }
                ap1_native_client::endpoint::receive_version_into(
                    &mut self.socket, 5, self.minor, &mut self.processing.reply)?;
            }
            let reply = &mut self.processing.reply;
            if let Some(s) = &mut self.fault_status {
                s.publish(operation_epoch, self.state.next, operation_position, 3, 0);
            }
            self.trace.replied = Some(std::time::Instant::now());
            if self.minor >= 3 {
                need(
                    (if self.minor >= 9 {
                        reply.payload.len() >= 72
                    } else {
                        reply.payload.len()
                            == if self.minor == 8 {
                                56
                            } else if self.minor >= 6 {
                                40
                            } else {
                                32
                            }
                    }) && get(&reply.payload[16..24]) == operation_epoch
                        && get(&reply.payload[24..32]) == operation_position,
                    "Done epoch/position differs",
                )?;
                if self.minor >= 6 {
                    self.trace.process_ns = Some(get(&reply.payload[32..40]));
                }
                if self.minor >= 8 {
                    let flags = get(&reply.payload[40..44]) as u32;
                    need(
                        flags & !10 == 0 && get(&reply.payload[52..56]) == 0,
                        "restart notification fields",
                    )?;
                    self.notices = (flags, get(&reply.payload[44..52]));
                    if self.minor >= 9 {
                        self.returned = process_results::Packet::decode(&reply.payload[56..], n)?;
                    }
                }
                reply.payload.truncate(16);
            }
            let flags = self.state.done_at(reply, mapping_output)?;
            need(output_channels == 64 || flags >> output_channels == 0, "output flags")?;
            let output = if let Some((snapshot, _)) = &sample_planes {
                let map = self
                    .mapping
                    .as_mut()
                    .ok_or_else(|| invalid("mapping absent"))?;
                barrier();
                #[cfg(test)]
                sample_plane_test::read();
                let left = map.plane(map.output, 0)?;
                #[cfg(test)]
                sample_plane_test::read();
                let right = map.plane(map.output, 1)?;
                let output: [[u32; BLOCK_CAP + 2]; 2] = [left, right];
                for ch in 0..2 {
                    #[cfg(test)]
                    sample_plane_test::read();
                    let mapped_input = map.plane(INPUT, ch)?;
                    need(mapped_input == snapshot[ch], "input changed")?;
                    need(
                        output[ch][0] == GUARD
                            && output[ch][capacity + 1] == GUARD
                            && output[ch][n + 1..capacity + 1].iter().all(|&x| x == POISON),
                        "output bounds",
                    )?;
                    for &bits in &output[ch][1..n + 1] {
                        let sample = f32::from_bits(bits);
                        need(
                            sample.is_finite() && ((flags & (1 << ch)) == 0 || sample == 0.0),
                            "invalid output claim",
                        )?;
                    }
                }
                for ch in 2..output_channels {
                    #[cfg(test)]
                    sample_plane_test::read();
                    let plane: [u32; BLOCK_CAP + 2] = map.plane(map.output, ch)?;
                    need(plane[0] == GUARD && plane[capacity+1] == GUARD
                        && plane[n+1..capacity+1].iter().all(|&x| x == POISON), "extra output bounds")?;
                    for i in 0..n {
                        let sample = f32::from_bits(plane[i+1]);
                        need(sample.is_finite() && (flags & (1u64 << ch) == 0 || sample == 0.), "extra output claim")?;
                        map.extra[ch-2][i] = sample;
                    }
                }
                output
            } else { [[0; BLOCK_CAP + 2]; 2] };
            self.trace.validated = Some(std::time::Instant::now());
            self.position += n as u64;
            Ok((output, flags))
        })();
        if result.is_err() {
            self.phase = ERROR;
            self.state.failed();
        }
        result
    }
    fn close(mut self) -> io::Result<()> {
        if let Some(gui) = &self.gui {
            gui.shutdown();
        }
        let result = if self.phase == 15 || self.minor >= 4 && self.phase == 17 {
            let f = self.state.close()?;
            send_version(&mut self.socket, &f, 5, self.minor)
                .and_then(|_| receive_version(&mut self.socket, 10, self.minor))
                .and_then(|f| self.state.closed(&f))
        } else {
            Err(invalid("unclean session close"))
        };
        // No native asynchronous worker exists. Shutdown prevents replay; the external
        // supervisor owns the independently mapped Windows endpoint on failure.
        let _ = self.socket.shutdown(std::net::Shutdown::Both);
        self.notifications.take();
        let unmap = self
            .mapping
            .take()
            .ok_or_else(|| invalid("mapping absent"))?
            .close();
        result.and(unmap)
    }
}
#[no_mangle]
pub extern "C" fn ap2_abi_version() -> u32 {
    1
}
/// # Safety
/// `handle` is writable for one u64. Call only from a serialized owner thread.
#[no_mangle]
pub unsafe extern "C" fn ap2_open(max: u32, handle: *mut u64) -> i32 {
    ffi(|| {
        if handle.is_null() || !(1..=256).contains(&max) {
            return 1;
        }
        let Ok(mut slot) = SESSION.try_lock() else {
            return 3;
        };
        if slot.is_some() {
            return 3;
        }
        let result = binding(false).and_then(|b| Session::open(b, max as usize, 2));
        match result {
            Ok(s) => {
                let id = NEXT.fetch_add(1, Ordering::Relaxed);
                *handle = id;
                *slot = Some((id, s));
                0
            }
            Err(error) => retain(&error),
        }
    })
}
#[no_mangle]
pub extern "C" fn ap2_transition(handle: u64, op: u32) -> i32 {
    ffi(|| {
        let Ok(mut slot) = SESSION.try_lock() else {
            return 3;
        };
        let Some((id, s)) = slot.as_mut() else {
            return 1;
        };
        if *id != handle || !matches!(op, 10 | 12 | 14) {
            return 1;
        }
        match s.transition(op as u16) {
            Ok(()) => 0,
            Err(error) => retain(&error),
        }
    })
}
/// # Safety
/// Buffers must cover `n` floats, flags one u64; no concurrent lifecycle calls.
#[no_mangle]
pub unsafe extern "C" fn ap2_process(
    handle: u64,
    n: u32,
    gain: f64,
    silence: u64,
    left: *const f32,
    right: *const f32,
    out_left: *mut f32,
    out_right: *mut f32,
    flags: *mut u64,
) -> i32 {
    ffi(|| {
        if n == 0
            || n > 256
            || left.is_null()
            || right.is_null()
            || out_left.is_null()
            || out_right.is_null()
            || flags.is_null()
        {
            return 1;
        }
        let Ok(mut slot) = SESSION.try_lock() else {
            return 3;
        };
        let Some((id, s)) = slot.as_mut() else {
            return 1;
        };
        if *id != handle {
            return 1;
        }
        let result = s.process(
            n as usize,
            gain,
            silence,
            [
                std::slice::from_raw_parts(left, n as usize),
                std::slice::from_raw_parts(right, n as usize),
            ],
        );
        match result {
            Ok((output, f)) => {
                for i in 0..n as usize {
                    *out_left.add(i) = f32::from_bits(output[0][i + 1]);
                    *out_right.add(i) = f32::from_bits(output[1][i + 1]);
                }
                *flags = f;
                0
            }
            Err(error) => retain(&error),
        }
    })
}
#[no_mangle]
pub extern "C" fn ap2_close(handle: u64) -> i32 {
    ffi(|| {
        let Ok(mut slot) = SESSION.try_lock() else {
            return 3;
        };
        if slot.as_ref().map(|v| v.0) != Some(handle) {
            return 1;
        }
        let (_, s) = slot.take().unwrap();
        match s.close() {
            Ok(()) => 0,
            Err(error) => retain(&error),
        }
    })
}

/// Copies a bounded explanation from the calling thread. No private binding data.
/// # Safety
/// `out` is writable for `capacity` bytes when non-null.
#[no_mangle]
pub unsafe extern "C" fn ap2_error(out: *mut u8, capacity: u32) -> u32 {
    if out.is_null() || capacity == 0 {
        return 0;
    }
    ERROR_DETAIL.with(|s| {
        let s = s.borrow();
        let n = s.len().min(capacity as usize - 1);
        std::ptr::copy_nonoverlapping(s.as_ptr(), out, n);
        *out.add(n) = 0;
        n as u32
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{Duration, Instant};
    fn pending_session(socket: TcpStream, path: &std::path::Path, session: [u8; 16]) -> Session {
        Session {
            gui: None,
            gui_revision: 0,
            mapping: Some(Mapping::new(path).unwrap()),
            mailbox: None,
            mailbox_enabled: false,
            notifications: None,
            configured_mode: 0,
            capture: None,
            fault_status: None,
            notices: (0, 0),
            returned: Default::default(),
            processing: ProcessingScratch::new(),
            socket,
            state: ClientState { session, next: 17, slot: Slot::Writable },
            phase: 9,
            max: 256,
            minor: 15,
            epoch: 0,
            position: 0,
            witness: None,
            identity: None,
            trace: Default::default(),
            sample_rate: 48000,
            armed: false,
            owner: None,
        }
    }
    fn socket_pair() -> (TcpStream, TcpStream) {
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }
    #[test]
    fn invalid_ownership_has_no_effect() {
        assert_eq!(ap2_abi_version(), 1);
        assert_eq!(ap2_close(77), 1);
        assert_eq!(ap2_transition(77, 10), 1);
        unsafe {
            let mut handle = 0;
            assert_eq!(ap2_open(0, &mut handle), 1);
            assert_eq!(ap2_open(257, &mut handle), 1);
            assert_eq!(ap2_open(256, std::ptr::null_mut()), 1);
        }
        assert!(SESSION.lock().unwrap().is_none());
    }
    #[test]
    fn successor_does_not_change_ap1_codec() {
        let f = Frame {
            kind: 8,
            session: [0; 16],
            sequence: 1,
            payload: vec![0; 4],
        };
        assert!(f.encode().is_err());
        let raw = f.encode_version(2).unwrap();
        assert!(Frame::decode(&raw).is_err());
        assert_eq!(Frame::decode_version(&raw, 2).unwrap(), f);
    }
    #[test]
    fn pending_start_refusal_fails_exact_session_and_fresh_session_can_restart() {
        let directory = std::env::temp_dir().join(format!(
            "lvb-pending-start-{}-{}",
            std::process::id(),
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        std::fs::create_dir(&directory).unwrap();
        let identity = [45; 16];
        let pending = PendingStart {
            session: identity,
            sequence: 17,
            epoch: 1,
            receive_deadline: Instant::now() + Duration::from_secs(1),
        };
        let (client, mut peer) = socket_pair();
        let mut refused = pending_session(client, &directory.join("refused.audio"), identity);
        let writer = std::thread::spawn(move || {
            send_version(&mut peer, &Frame {
                kind: 7,
                session: identity,
                sequence: 17,
                payload: vec![1, 0, 0, 0],
            }, 1, 15).unwrap();
        });
        assert_eq!(refused.finish_start(pending, || false).unwrap_err().to_string(),
            "wrong pending Started response");
        writer.join().unwrap();
        assert_eq!(refused.phase, ERROR);
        assert_eq!(refused.state.slot, Slot::Failed);

        let (client, mut peer) = socket_pair();
        let mut restarted = pending_session(client, &directory.join("restarted.audio"), identity);
        let writer = std::thread::spawn(move || {
            send_version(&mut peer, &Frame {
                kind: 11,
                session: identity,
                sequence: 17,
                payload: 1u64.to_le_bytes().to_vec(),
            }, 1, 15).unwrap();
        });
        restarted.finish_start(pending, || false).unwrap();
        writer.join().unwrap();
        assert_eq!((restarted.phase, restarted.epoch, restarted.position), (11, 1, 0));
        assert_eq!(restarted.state.slot, Slot::Writable);
        drop(refused);
        drop(restarted);
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn pending_start_partial_ack_cannot_outlive_the_audio_deadline() {
        let directory = std::env::temp_dir().join(format!(
            "lvb-pending-start-audio-deadline-{}-{}",
            std::process::id(),
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        std::fs::create_dir(&directory).unwrap();
        let identity = [46; 16];
        let (client, mut peer) = socket_pair();
        let mut session = pending_session(client, &directory.join("deadline.audio"), identity);
        let pending = PendingStart {
            session: identity,
            sequence: 17,
            epoch: 1,
            receive_deadline: Instant::now() + Duration::from_secs(10),
        };
        let reply = Frame {
            kind: 11,
            session: identity,
            sequence: 17,
            payload: 1u64.to_le_bytes().to_vec(),
        }
        .encode_version(15)
        .unwrap();
        peer.write_all(&reply[..13]).unwrap();
        let audio_deadline = Instant::now() - Duration::from_nanos(1);
        let bounded = pending.bounded_by(audio_deadline);
        assert_eq!(bounded.receive_deadline, audio_deadline);
        assert_eq!(session.finish_start(bounded, || false).unwrap_err().kind(),
            io::ErrorKind::TimedOut);
        assert_eq!(session.phase, ERROR);
        assert_eq!(session.state.slot, Slot::Failed);
        drop(session);
        drop(peer);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

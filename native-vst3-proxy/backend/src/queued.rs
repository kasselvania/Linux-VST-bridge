//! Callback operations contain bounded copies, scalar checks, atomics and
//! monotonic timestamp reads. No allocation, waiting, logging or transport I/O.
//! All mapping/socket/session work and error formatting belong to the worker.
use crate::{binding, queue::Queue, retain, state, Session};
use ap1_native_client::{
    events::{Event, MAX_EVENTS},
    invalid, CAP, ERROR,
};
use std::{
    cell::UnsafeCell,
    io,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
pub const DELAY: u64 = 1024;
pub const DESCRIPTORS: usize = 2048;
const START: u32 = 10;
const STOP: u32 = 12;
const DEACTIVATE: u32 = 14;
const CLOSE: u32 = 5;
const AUDIO: u32 = 3;
// Fault code 1 was the retired terminal-underrun policy. Never reuse it.
const OVERFLOW: u64 = 2;
const WORKER: u64 = 3;
const CORRELATION: u64 = 4;
#[derive(Clone, Copy)]
pub struct Item {
    pub gui_revision: u64,
    pub kind: u32,
    pub n: u32,
    pub epoch: u64,
    pub position: u64,
    pub gain: f64,
    pub flags: u64,
    pub queued: Option<Instant>,
    pub parent: [u64; 4],
    pub context: crate::context::Context,
    pub event_count: u32,
    pub events: [Event; MAX_EVENTS],
    pub data: [[f32; CAP]; 2],
}
impl Item {
    fn control(kind: u32, epoch: u64) -> Self {
        Self {
            gui_revision: 0,
            kind,
            n: 0,
            epoch,
            position: 0,
            gain: 0.,
            flags: 0,
            queued: None,
            parent: [0; 4],
            context: crate::context::Context::default(),
            event_count: 0,
            events: [Event::default(); MAX_EVENTS],
            data: [[0.; CAP]; 2],
        }
    }
}
#[derive(Clone, Copy)]
struct AudioResult {
    n: u32,
    position: u64,
    flags: u64,
    data: [[f32; CAP]; 2],
}
impl AudioResult {
    fn empty() -> Self {
        Self {
            n: 0,
            position: 0,
            flags: 0,
            data: [[0.; CAP]; 2],
        }
    }
}
#[derive(Clone, Copy)]
struct Completion {
    audio: AudioResult,
    epoch: u64,
    returned: crate::process_results::Packet,
}
impl From<Item> for Completion {
    fn from(i: Item) -> Self {
        Self {
            audio: AudioResult {
                n: i.n,
                position: i.position,
                flags: i.flags,
                data: i.data,
            },
            epoch: i.epoch,
            returned: crate::process_results::Packet::default(),
        }
    }
}
struct Shared {
    gui: Option<Arc<crate::gui::Gui>>,
    snapshots: Arc<std::sync::Mutex<crate::recovery::Store>>,
    generation: u64,
    identity: Option<state::Identity>,
    notices: AtomicU64,
    notice_traits: AtomicU64,
    last_edit: AtomicU64,
    retired: AtomicBool,
    requests: Queue<Item>,
    results: Queue<Completion>,
    wanted: AtomicU64,
    fault: AtomicU64,
    ack: AtomicU64,
    quit: AtomicBool,
    processed: AtomicU64,
    first_position: AtomicU64,
    detail: std::sync::Mutex<String>,
    // Separate owner-thread mailbox. It never writes the SPSC audio queue.
    control: std::sync::Mutex<Option<Control>>,
    pending_control: AtomicBool,
    state_capable: AtomicBool,
    observer: Option<Arc<crate::observer::Shared>>,
    worker_op: AtomicU64,
    worker_epoch: AtomicU64,
    worker_position: AtomicU64,
    service_us_max: AtomicU64,
    // Callback writes counters only; the transport publishes them through the
    // existing independent status lane. No callback mapping or diagnostic I/O.
    delivery_totals: [AtomicU64; 6],
    first_context_ready: AtomicBool,
    first_epoch: AtomicU64,
    first_worker_op: AtomicU64,
    first_worker_epoch: AtomicU64,
    first_worker_position: AtomicU64,
    first_requests: [AtomicU64; 2],
    first_results: [AtomicU64; 2],
}
impl Shared {
    fn new() -> Self {
        Self {
            gui: None,
            snapshots: Arc::new(std::sync::Mutex::new(crate::recovery::Store::default())),
            generation: 1,
            identity: None,
            notices: AtomicU64::new(0),
            notice_traits: AtomicU64::new(0),
            last_edit: AtomicU64::new(0),
            retired: AtomicBool::new(false),
            requests: Queue::new(DESCRIPTORS),
            results: Queue::new(DESCRIPTORS),
            wanted: AtomicU64::new(0),
            fault: AtomicU64::new(0),
            ack: AtomicU64::new(9),
            quit: AtomicBool::new(false),
            processed: AtomicU64::new(0),
            first_position: AtomicU64::new(u64::MAX),
            detail: std::sync::Mutex::new(String::new()),
            control: std::sync::Mutex::new(None),
            pending_control: AtomicBool::new(false),
            state_capable: AtomicBool::new(false),
            observer: None,
            worker_op: AtomicU64::new(0),
            worker_epoch: AtomicU64::new(0),
            worker_position: AtomicU64::new(0),
            service_us_max: AtomicU64::new(0),
            delivery_totals: std::array::from_fn(|_| AtomicU64::new(0)),
            first_context_ready: AtomicBool::new(false),
            first_epoch: AtomicU64::new(0),
            first_worker_op: AtomicU64::new(0),
            first_worker_epoch: AtomicU64::new(0),
            first_worker_position: AtomicU64::new(0),
            first_requests: std::array::from_fn(|_| AtomicU64::new(0)),
            first_results: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }
    fn fail(&self, code: u64, position: u64) {
        if self
            .fault
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            self.first_position.store(position, Ordering::Relaxed);
            self.first_epoch
                .store(self.wanted.load(Ordering::Acquire), Ordering::Relaxed);
            self.first_worker_op
                .store(self.worker_op.load(Ordering::Acquire), Ordering::Relaxed);
            self.first_worker_epoch
                .store(self.worker_epoch.load(Ordering::Acquire), Ordering::Relaxed);
            self.first_worker_position.store(
                self.worker_position.load(Ordering::Acquire),
                Ordering::Relaxed,
            );
            // Independent progress counters: bounded observations, not an
            // atomic queue snapshot or a wall-clock claim about the peer.
            self.first_requests[0].store(self.requests.published(), Ordering::Relaxed);
            self.first_requests[1].store(self.requests.consumed(), Ordering::Relaxed);
            self.first_results[0].store(self.results.published(), Ordering::Relaxed);
            self.first_results[1].store(self.results.consumed(), Ordering::Relaxed);
            self.first_context_ready.store(true, Ordering::Release);
        }
    }
}
struct Control {
    barrier: u64,
    op: u32,
    bytes: Vec<u8>,
    result: Option<io::Result<Vec<u8>>>,
}
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Delivery {
    pub missing_frames: u64,
    pub gaps: u64,
    pub expired_frames: u64,
    pub delivered_frames: u64,
    pub priming_frames: u64,
}
struct Callback {
    host_call: u64,
    delay: u64,
    epoch: u64,
    position: u64,
    running: bool,
    have: bool,
    offset: usize,
    current: AudioResult,
    audio: std::collections::VecDeque<AudioResult>,
    returned: crate::process_results::Pending,
    next_result: u64,
    in_gap: bool,
    delivery: Delivery,
}
impl Callback {
    fn new() -> Self {
        let mut audio = std::collections::VecDeque::from(vec![AudioResult::empty(); DESCRIPTORS]);
        for item in &mut audio {
            unsafe { std::ptr::write_volatile(&mut item.n, 0); }
        }
        audio.clear();
        Self {
            host_call: 0,
            delay: DELAY,
            epoch: 0,
            position: 0,
            running: false,
            have: false,
            offset: 0,
            current: AudioResult::empty(),
            audio,
            returned: crate::process_results::Pending::new(),
            next_result: 0,
            in_gap: false,
            delivery: Delivery::default(),
        }
    }
    fn transition(&mut self, s: &Shared, op: u32) -> u32 {
        if s.fault.load(Ordering::Acquire) != 0 {
            return 2;
        }
        match op {
            START if !self.running && self.epoch < u64::MAX => {
                self.epoch += 1;
                self.position = 0;
                self.have = false;
                self.offset = 0;
                self.next_result = 0;
                self.in_gap = false;
                s.results.discard_published();
                self.audio.clear();
                self.returned.reset();
                s.wanted.store(self.epoch, Ordering::Release);
                self.running = true;
            }
            STOP if self.running => {
                self.running = false;
                self.audio.clear();
                self.returned.reset();
                s.wanted.store(0, Ordering::Release);
            }
            _ => return 1,
        }
        if !s.requests.push(Item::control(op, self.epoch)) {
            s.fail(OVERFLOW, self.position);
            return 2;
        }
        0
    }
    fn process(
        &mut self,
        s: &Shared,
        mut request: Item,
        out: &mut [[f32; CAP]; 2],
    ) -> Result<u64, u32> {
        if !self.running || request.n as usize > CAP {
            return Err(1);
        }
        if s.fault.load(Ordering::Acquire) != 0 {
            return Err(2);
        }
        request.epoch = self.epoch;
        request.position = self.position;
        request.queued = Some(Instant::now());
        if request.events[..request.event_count as usize]
            .iter()
            .any(|e| e.kind == 2)
        {
            request.gui_revision = s.gui.as_ref().map_or(0, |gui| gui.revision());
        }
        if !s.requests.push(request) {
            s.fail(OVERFLOW, self.position);
            return Err(2);
        }
        if !request.gain.is_nan() || request.event_count > 0 {
            s.last_edit.store(s.requests.published(), Ordering::Release);
        }
        self.delivery = Delivery::default();
        let n = request.n as usize;
        let mut flags = 3;
        // Consume whole completions independently of audio presentation. This
        // admits zero-frame results and preserves late events before audio expiry.
        for _ in 0..DESCRIPTORS {
            let Some(item) = s.results.pop() else {
                break;
            };
            if item.epoch < self.epoch {
                continue;
            }
            let a = item.audio;
            if item.epoch != self.epoch
                || a.n as usize > CAP
                || a.position != self.next_result
                || a.position.checked_add(a.n as u64).is_none()
            {
                s.fail(CORRELATION, self.position);
                return Err(2);
            }
            self.next_result += a.n as u64;
            if !self
                .returned
                .append(&item.returned, a.position, a.n as usize, self.delay)
            {
                s.fail(OVERFLOW, self.position);
                return Err(2);
            }
            if a.n > 0 {
                if self.audio.len() == DESCRIPTORS {
                    s.fail(OVERFLOW, self.position);
                    return Err(2);
                }
                self.audio.push_back(a);
            }
        }
        let mut i = 0;
        while i < n {
            let position = self.position + i as u64;
            if position < self.delay {
                let count = (self.delay - position).min((n - i) as u64) as usize;
                for plane in out.iter_mut() {
                    plane[i..i + count].fill(0.);
                }
                self.delivery.priming_frames += count as u64;
                i += count;
                continue;
            }
            let expected = position - self.delay;
            if !self.have {
                match self.audio.pop_front() {
                    Some(item) => {
                        self.current = item;
                        self.offset = 0;
                        self.have = true;
                    }
                    None => {
                        // The buffer is filled successfully, with a counted
                        // missing presentation span. Continue the same epoch.
                        let count = n - i;
                        for plane in out.iter_mut() {
                            plane[i..n].fill(0.);
                        }
                        if let Some(observer) = &s.observer {
                            if !observer.gaps.push(crate::observer::Gap {
                                epoch: self.epoch,
                                position: expected,
                                frames: count as u64,
                                at: Instant::now(),
                                parent: request.parent,
                            }) {
                                observer.gap_drops.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        self.delivery.missing_frames += count as u64;
                        self.delivery.gaps += u64::from(!self.in_gap);
                        self.in_gap = true;
                        break;
                    }
                }
            }
            let expired = expected
                .saturating_sub(self.current.position)
                .min(self.current.n as u64) as usize;
            if expired > self.offset {
                self.delivery.expired_frames += (expired - self.offset) as u64;
                self.offset = expired;
            }
            if self.offset == self.current.n as usize {
                self.have = false;
                continue;
            }
            if self.current.position + self.offset as u64 != expected {
                s.fail(CORRELATION, position);
                return Err(2);
            }
            let count = (n - i).min(self.current.n as usize - self.offset);
            flags &= self.current.flags;
            for (out, input) in out.iter_mut().zip(&self.current.data) {
                out[i..i + count].copy_from_slice(&input[self.offset..self.offset + count]);
            }
            self.delivery.delivered_frames += count as u64;
            self.in_gap = false;
            self.offset += count;
            i += count;
            if self.offset == self.current.n as usize {
                self.have = false;
            }
        }
        self.position += request.n as u64;
        Ok(flags)
    }
}
struct Live {
    shared: Arc<Shared>,
    callback: UnsafeCell<Callback>,
    busy: AtomicBool,
    worker: Option<JoinHandle<()>>,
    report: Option<std::path::PathBuf>,
    max: usize,
    recovery_blocked: bool,
    installed_delay: Option<u32>,
    minor: u64,
    setup: Option<Vec<u8>>,
}
// Callback interior state is accessed only under this instance's nonblocking
// guard. The worker owns Shared/Session; removal excludes every live lease.
unsafe impl Sync for Live {}
static INSTANCES: crate::instances::Registry<Live> = crate::instances::Registry::new();
struct Guard<'a>(&'a AtomicBool);
impl<'a> Guard<'a> {
    fn acquire(live: &'a Live) -> Option<Self> {
        live.busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Self(&live.busy))
    }
}
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
fn worker(mut session: Session, s: Arc<Shared>, report: Option<std::path::PathBuf>) {
    let mut previous_control = [0u64; 4];
    let mut deferred = None;
    if let Some(status) = &mut session.fault_status {
        status.generation = s.generation;
    }
    let run = (|| -> io::Result<()> {
        loop {
            crate::preview::check_owner(&mut session.owner)?;
            if s.quit.load(Ordering::Acquire) || s.fault.load(Ordering::Acquire) != 0 {
                return Err(invalid("queued session fault or cancelled"));
            }
            if s.pending_control.load(Ordering::Acquire) {
                let mut mailbox = s
                    .control
                    .lock()
                    .map_err(|_| invalid("state mailbox poisoned"))?;
                if let Some(c) = mailbox.as_mut() {
                    if c.result.is_none() && s.requests.consumed() >= c.barrier {
                        s.worker_op.store(c.op as u64, Ordering::Release);
                        if session.capture.is_none() {
                            previous_control = [c.op as u64, crate::observer::monotonic_ns(), 0, c.barrier];
                            if c.op == 16 && session.can_capture_during_audio() { session.begin_capture()?; }
                        }
                        let completed = if session.capture.is_some() {
                            session.poll_capture().map(|r| r.and_then(|p| state::bound_envelope(session.identity, &p)))
                        } else { Some(match c.op {
                            16 => session
                                .component_state(None)
                                .and_then(|p| state::bound_envelope(session.identity, &p)),
                            18 => session
                                .component_state(Some(&c.bytes))
                                .and_then(|p| state::bound_envelope(session.identity, &p)),
                            8 => {
                                // Observation waits for a real state operation;
                                // it cannot add a prerequisite transport request.
                                session
                                    .activate(
                                        ap1_native_client::get(&c.bytes[..4]) as usize,
                                        ap1_native_client::get(&c.bytes[4..]) as u32,
                                    )
                                    .map(|_| vec![])
                            }
                            20 => session.configure(c.bytes.clone()),
                            14 => session.transition(14).map(|_| {
                                s.ack.store(15, Ordering::Release);
                                vec![]
                            }),
                            _ => Err(invalid("unknown owner operation")),
                        }) };
                        if let Some(result) = completed {
                        previous_control[2] = crate::observer::monotonic_ns();
                        let failed = result.as_ref().is_err_and(|e| !state::save_refused(e));
                        if let Ok(bytes) = &result {
                            if matches!(c.op, 16 | 18) {
                                s.snapshots
                                    .lock()
                                    .map_err(|_| invalid("snapshot store poisoned"))?
                                    .confirm(
                                        bytes.clone(),
                                        c.op,
                                        s.generation,
                                        c.barrier,
                                    )?;
                            }
                        } else if let Err(error) = &result {
                            if !state::save_refused(error) {
                                if let Ok(mut detail) = s.detail.lock() {
                                    if detail.is_empty() {
                                        *detail = bounded_detail(error);
                                    }
                                }
                            }
                        }
                        c.result = Some(result);
                        s.pending_control.store(false, Ordering::Release);
                        if failed {
                            return Err(invalid("component state/control failed; original detail retained in response"));
                        }
                        }
                    }
                }
            }
            s.worker_op.store(0, Ordering::Release);
            if session.mailbox.as_ref().is_some_and(|m| m.control_handoff_pending()) {
                thread::sleep(Duration::from_micros(50));
                continue;
            }
            let Some(mut item) = deferred.take().or_else(|| s.requests.pop()) else {
                thread::sleep(Duration::from_micros(50));
                continue;
            };
            // Restore/lifecycle keep exclusive control-stream ownership. Only
            // ordered audio can pass an admitted read-only capture.
            if session.capture.is_some() && item.kind != AUDIO {
                deferred = Some(item);
                thread::sleep(Duration::from_micros(50));
                continue;
            }
            s.worker_epoch.store(item.epoch, Ordering::Relaxed);
            s.worker_position.store(item.position, Ordering::Relaxed);
            s.worker_op.store(item.kind as u64, Ordering::Release);
            match item.kind {
                AUDIO => {
                    let started = Instant::now();
                    if let Some(status) = &mut session.fault_status {
                        status.delivery = std::array::from_fn(|i| s.delivery_totals[i].load(Ordering::Acquire));
                    }
                    let n = item.n as usize;
                    let original = item.data;
                    session.gui_revision = item.gui_revision;
                    let (words, flags) = session.process_positioned(
                        n,
                        item.gain,
                        item.flags,
                        [&item.data[0][..n], &item.data[1][..n]],
                        (item.epoch, item.position),
                        &item.events[..item.event_count as usize],
                        item.context,
                    )?;
                    for (ch, word) in words.iter().enumerate() {
                        for i in 0..n {
                            item.data[ch][i] = f32::from_bits(word[i + 1]);
                        }
                    }
                    if session.notices.0 != 0 {
                        s.notice_traits.store(session.notices.1, Ordering::Relaxed);
                        s.notices
                            .fetch_or(u64::from(session.notices.0), Ordering::Release);
                        session.notices.0 = 0;
                    }
                    item.flags = flags;
                    s.service_us_max.fetch_max(
                        started.elapsed().as_micros().min(u64::MAX as u128) as u64,
                        Ordering::Relaxed,
                    );
                    s.processed.fetch_add(1, Ordering::Relaxed);
                    let publish = s.wanted.load(Ordering::Acquire) == item.epoch;
                    let mut completion = Completion::from(item);
                    completion.returned = session.returned;
                    if publish && !s.results.push(completion) {
                        s.fail(OVERFLOW, item.position);
                        return Err(invalid("completed output capacity"));
                    }
                    session.trace.queued = item.queued;
                    session.trace.parent = item.parent;
                    session.trace.previous_control = previous_control;
                    session.trace.published = publish.then(Instant::now);
                    // Only a bounded copy after output publication. The next request
                    // never waits for comparison, hashing or report readers.
                    if let Some(observer) = &mut session.witness {
                        observer.audio(
                            n,
                            item.gain,
                            [&original[0][..n], &original[1][..n]],
                            words,
                            session.trace,
                        );
                    }
                }
                START | STOP => {
                    session.transition_epoch(item.kind as u16, item.epoch)?;
                    s.ack.store(
                        ((item.epoch) << 8) | u64::from(item.kind + 1),
                        Ordering::Release,
                    );
                }
                DEACTIVATE => {
                    session.transition(14)?;
                    s.ack.store(15, Ordering::Release);
                }
                CLOSE => return Ok(()),
                _ => return Err(invalid("queued control kind")),
            }
        }
    })();
    if let Err(ref error) = run {
        s.fail(WORKER, session.position);
        if let Ok(mut d) = s.detail.lock() {
            if d.is_empty() {
                *d = bounded_detail(error);
            }
        }
        session.phase = ERROR;
        // Retain the first fault before Session::close permits the preview
        // owner to retire this instance's stage. No callback performs I/O.
        if let Some(path) = &report {
            let text = format!(
                "{{\"event\":\"ap5_worker_fault\",\"generation\":{},\"fault\":{},\"first_position\":{},\"processed\":{},\"request_high\":{},\"result_high\":{},\"detail\":\"{}\"}}\n",
                s.generation,
                s.fault.load(Ordering::Acquire),
                s.first_position.load(Ordering::Acquire),
                s.processed.load(Ordering::Acquire),
                s.requests.high_water(), s.results.high_water(),
                json_text(&s.detail.lock().map(|d| d.clone()).unwrap_or_default())
            );
            crate::preview::append_report(path, text.as_bytes());
            crate::preview::append_report(path, progress_text(&s).as_bytes());
        }
    }
    if let Some(observer) = &mut session.witness {
        observer.finish();
        if let Some(path) = &report {
            crate::preview::append_records(path, &crate::observer::report_text(&observer.shared));
        }
    }
    if session.witness.is_none() {
        if let Some(path) = &report {
            crate::preview::append_report(path, b"{\"event\":\"ap7_observation\",\"enabled\":false,\"verified_returned_samples\":0}\n");
        }
    }
    let owner = session.owner.take();
    if let Err(error) = session.close() {
        s.fail(WORKER, u64::MAX);
        if let Ok(mut d) = s.detail.lock() {
            if d.is_empty() {
                *d = format!("{:?}: {}", error.kind(), error)
                    .chars()
                    .take(384)
                    .collect();
            }
        }
    }
    // Positive owner acknowledgement includes process containment AND stage
    // retirement. EOF, timeout or a missing owner cannot authorize recovery.
    if let Some(owner) = owner {
        s.retired.store(owner.finish().is_ok(), Ordering::Release);
    }
    s.ack.store(6, Ordering::Release);
}
fn progress_text(s: &Shared) -> String {
    let ready = s.first_context_ready.load(Ordering::Acquire);
    format!(
        "{{\"event\":\"ap7_fault_progress\",\"context_ready\":{},\"epoch\":{},\"worker_op\":{},\"worker_epoch\":{},\"worker_position\":{},\"request_published\":{},\"request_consumed\":{},\"result_published\":{},\"result_consumed\":{},\"service_us_max_at_report\":{},\"observation_skips\":{}}}\n",
        ready, s.first_epoch.load(Ordering::Relaxed), s.first_worker_op.load(Ordering::Relaxed),
        s.first_worker_epoch.load(Ordering::Relaxed), s.first_worker_position.load(Ordering::Relaxed),
        s.first_requests[0].load(Ordering::Relaxed), s.first_requests[1].load(Ordering::Relaxed),
        s.first_results[0].load(Ordering::Relaxed), s.first_results[1].load(Ordering::Relaxed),
        s.service_us_max.load(Ordering::Relaxed), s.observer.as_ref().map_or(0, |o| o.dropped.load(Ordering::Relaxed)))
}
fn bounded_detail(error: &io::Error) -> String {
    format!("{:?}: {}", error.kind(), error)
        .chars()
        .take(384)
        .collect()
}
fn json_text(detail: &str) -> String {
    detail
        .chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            c if c.is_control() => vec!['?'],
            c => vec![c],
        })
        .collect()
}
#[no_mangle]
pub extern "C" fn ap3_abi_version() -> u32 {
    1
}
#[no_mangle]
pub unsafe extern "C" fn ap3_open(max: u32, handle: *mut u64) -> u32 {
    open(max, handle, 3, None)
}
#[no_mangle]
pub unsafe extern "C" fn ap4_open(handle: *mut u64) -> u32 {
    open(256, handle, 4, None)
}
pub(crate) unsafe fn open(
    max: u32,
    handle: *mut u64,
    minor: u64,
    identity: Option<state::Identity>,
) -> u32 {
    if handle.is_null() || !(1..=256).contains(&max) {
        return 1;
    }
    crate::ffi(|| {
        match INSTANCES.insert(|| {
            let binding = if minor >= 6 {
                crate::preview::discover_performance(identity)?
            } else if let Some(identity) = identity {
                crate::preview::discover_commercial(identity)?
            } else {
                binding(minor == 4)?
            };
            let report = binding.owner.as_ref().map(|_| {
                if minor >= 6 {
                    crate::preview::performance_root(identity.is_some())
                        .join("results")
                        .join(format!(
                            "native-{:032x}.jsonl",
                            u128::from_be_bytes(binding.session)
                        ))
                } else if identity.is_some() {
                    crate::preview::commercial_report_path(binding.session)
                } else {
                    crate::preview::report_path(binding.session)
                }
            });
            let installed_delay = binding.installed_delay;
            let mut session = Session::open(binding, max as usize, minor)?;
            session.identity = identity;
            let mut shared = Shared::new();
            shared.gui = session.gui.clone();
            shared.identity = identity;
            shared.observer = session.witness.as_ref().map(|w| w.shared.clone());
            let shared = Arc::new(shared);
            shared.state_capable.store(minor >= 4, Ordering::Release);
            if minor >= 4 {
                shared.ack.store(17, Ordering::Release);
            }
            let peer = shared.clone();
            let worker_report = report.clone();
            let t = thread::Builder::new()
                .name("ap3-transport".into())
                .spawn(move || worker(session, peer, worker_report))?;
            Ok::<_, io::Error>(Live {
                shared,
                callback: UnsafeCell::new(Callback::new()),
                busy: AtomicBool::new(false),
                worker: Some(t),
                report,
                max: max as usize,
                recovery_blocked: false,
                installed_delay,
                minor,
                setup: None,
            })
        }) {
            Ok(Ok(id)) => {
                *handle = id;
                0
            }
            Ok(Err(e)) => retain(&e),
            Err(_) => retain(&io::Error::other(
                ap1_native_client::admission::Refusal::NativeImageCapacity,
            )),
        }
    }) as u32
}
#[repr(C)]
#[derive(Default)]
pub struct RecoveryInfo {
    revision: u64,
    generation: u64,
    source: u32,
    uncaptured: u32,
    digest: [u8; 32],
}
#[no_mangle]
pub unsafe extern "C" fn ap6_snapshot(id: u64, out: *mut RecoveryInfo) -> u32 {
    crate::ffi(|| {
        if out.is_null() {
            return 1;
        }
        let Some(l) = INSTANCES.lease(id) else {
            return 1;
        };
        let Ok(store) = l.shared.snapshots.lock() else {
            return 2;
        };
        let Some(snapshot) = store.latest() else {
            return 2;
        };
        *out = RecoveryInfo {
            revision: snapshot.revision,
            generation: l.shared.generation,
            source: snapshot.source,
            digest: snapshot.digest,
            uncaptured: u32::from(
                snapshot.generation != l.shared.generation
                    || l.shared.last_edit.load(Ordering::Acquire) > snapshot.through,
            ),
        };
        0
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap6_recover(
    id: u64,
    revision: u64,
    out: *mut u8,
    capacity: u32,
    size: *mut u32,
) -> u32 {
    crate::ffi(|| {
        if out.is_null()
            || size.is_null()
            || (capacity as usize) < state::HEADER_SIZE + state::LIMIT
        {
            return 1;
        }
        match INSTANCES.update(id, |l| -> io::Result<()> {
            if l.recovery_blocked || l.shared.fault.load(Ordering::Acquire) == 0 {
                return Err(invalid("recovery requires a failed, contained instance"));
            }
            let snapshot = l
                .shared
                .snapshots
                .lock()
                .map_err(|_| invalid("snapshot store poisoned"))?
                .select(revision)?;
            let payload = state::bound_payload(l.shared.identity, &snapshot.bytes)?;
            let end = Instant::now() + Duration::from_secs(80);
            while l.worker.as_ref().is_some_and(|t| !t.is_finished()) {
                if Instant::now() >= end {
                    return Err(invalid("failed worker containment deadline"));
                }
                thread::sleep(Duration::from_millis(1));
            }
            if let Some(t) = l.worker.take() {
                t.join().map_err(|_| invalid("failed worker panicked"))?;
            }
            if !l.shared.retired.load(Ordering::Acquire) {
                return Err(invalid(
                    "owner has not confirmed failed endpoint retirement",
                ));
            }
            // A failed new startup/restore cannot silently authorize another
            // replacement. The owner still contains any newly owned endpoint.
            l.recovery_blocked = true;
            let binding = if l.minor >= 6 {
                crate::preview::discover_performance(l.shared.identity)?
            } else {
                binding(true)?
            };
            if binding.installed_delay != l.installed_delay {
                return Err(invalid("installed delay changed; reopen the device to renegotiate latency"));
            }
            if binding.owner.is_none() {
                return Err(invalid("recovery requires the private owner"));
            }
            let report = Some(if l.minor >= 6 {
                crate::preview::performance_root(l.shared.identity.is_some())
                    .join("results")
                    .join(format!(
                        "native-{:032x}.jsonl",
                        u128::from_be_bytes(binding.session)
                    ))
            } else {
                crate::preview::report_path(binding.session)
            });
            let mut session = Session::open(binding, l.max.min(CAP), l.minor)?;
            session.identity = l.shared.identity;
            if let Err(error) = session.component_state(Some(payload)).and_then(|_| {
                if let Some(setup) = &l.setup {
                    session.configure(setup.clone())?;
                }
                Ok(())
            }) {
                let owner = session.owner.take();
                let _ = session.close();
                if let Some(owner) = owner {
                    let _ = owner.finish();
                }
                return Err(error);
            }
            let mut shared = Shared::new();
            shared.gui = session.gui.clone();
            shared.generation = l
                .shared
                .generation
                .checked_add(1)
                .ok_or_else(|| invalid("transport generation exhausted"))?;
            shared.snapshots = l.shared.snapshots.clone();
            shared.identity = l.shared.identity;
            shared.state_capable.store(true, Ordering::Release);
            shared.ack.store(17, Ordering::Release);
            shared.observer = session.witness.as_ref().map(|w| w.shared.clone());
            let shared = Arc::new(shared);
            let peer = shared.clone();
            let worker_report = report.clone();
            let t = thread::Builder::new()
                .name("ap6-transport".into())
                .spawn(move || worker(session, peer, worker_report))?;
            l.shared = shared;
            l.worker = Some(t);
            l.report = report;
            let delay = l.callback.get_mut().delay;
            l.callback = UnsafeCell::new(Callback::new());
            l.callback.get_mut().delay = delay;
            l.recovery_blocked = false;
            std::ptr::copy_nonoverlapping(snapshot.bytes.as_ptr(), out, snapshot.bytes.len());
            *size = snapshot.bytes.len() as u32;
            Ok(())
        }) {
            Ok(Ok(())) => 0,
            Ok(Err(error)) => retain(&error),
            Err(code) => code as i32,
        }
    }) as u32
}
// Copy a diagnostic location once to its SDK instance. It remains usable after
// backend close for the final lifecycle report, without thread-local routing.
#[no_mangle]
pub unsafe extern "C" fn ap5_report_path(id: u64, out: *mut u8, capacity: u32) -> u32 {
    crate::ffi(|| {
        use std::os::unix::ffi::OsStrExt;
        if out.is_null() || capacity == 0 {
            return 1;
        }
        let Some(live) = INSTANCES.lease(id) else {
            return 1;
        };
        let bytes = live
            .report
            .as_ref()
            .map(|p| p.as_os_str().as_bytes())
            .unwrap_or(&[]);
        if bytes.len() >= capacity as usize {
            return 1;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
        *out.add(bytes.len()) = 0;
        0
    }) as u32
}
// The lifetime lease is released before any blocking control work. Arc retains
// this instance's mailbox through I/O; audio never takes the control mutex.
unsafe fn control(id: u64, op: u32, bytes: Vec<u8>) -> io::Result<Vec<u8>> {
    let s = INSTANCES
        .lease(id)
        .ok_or_else(|| invalid("state handle"))?
        .shared
        .clone();
    if !s.state_capable.load(Ordering::Acquire) || s.fault.load(Ordering::Acquire) != 0 {
        return Err(invalid("state unavailable"));
    }
    let barrier = s.requests.published();
    {
        let mut c = s
            .control
            .lock()
            .map_err(|_| invalid("state mailbox poisoned"))?;
        if c.is_some() {
            return Err(invalid("state operation already pending"));
        }
        *c = Some(Control {
            barrier,
            op,
            bytes,
            result: None,
        });
        s.pending_control.store(true, Ordering::Release);
    }
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        {
            let mut c = s
                .control
                .lock()
                .map_err(|_| invalid("state mailbox poisoned"))?;
            if let Some(result) = c.as_mut().and_then(|v| v.result.take()) {
                *c = None;
                return result;
            }
        }
        if Instant::now() >= until || s.fault.load(Ordering::Acquire) != 0 {
            s.fail(WORKER, u64::MAX);
            return Err(invalid(
                "state acknowledgement missing; instance failed, no retry",
            ));
        }
        thread::sleep(Duration::from_micros(50));
    }
}
#[no_mangle]
pub unsafe extern "C" fn ap9_setup(
    id: u64,
    maximum: u32,
    mode: u32,
    rate: f64,
    out: *mut u32,
) -> u32 {
    setup(
        id,
        maximum,
        mode,
        rate,
        &[],
        false,
        out,
        std::ptr::null_mut(),
    )
}
#[no_mangle]
pub unsafe extern "C" fn ap10_setup(
    id: u64,
    maximum: u32,
    mode: u32,
    rate: f64,
    io: *const u8,
    len: u32,
    notifications: u32,
    out: *mut u32,
) -> u32 {
    if io.is_null() || out.is_null() || notifications > 1 || !(4..=1028).contains(&len) {
        return 1;
    }
    setup(
        id,
        maximum,
        mode,
        rate,
        std::slice::from_raw_parts(io, len as usize),
        notifications == 1,
        out,
        out.add(2),
    )
}
#[allow(clippy::too_many_arguments)]
unsafe fn setup(
    id: u64,
    maximum: u32,
    mode: u32,
    rate: f64,
    io: &[u8],
    notifications: bool,
    out: *mut u32,
    vendor_out: *mut u32,
) -> u32 {
    crate::ffi(|| {
        if out.is_null() {
            return 1;
        }
        let result = (|| -> io::Result<()> {
            let installed = INSTANCES.lease(id).ok_or_else(|| invalid("setup instance absent"))?.installed_delay;
            let delay = if let Some(delay) = installed {
                crate::performance::validate_delay(maximum, delay)?;
                delay
            } else {
                crate::performance::selected_delay(maximum)?
            };
            let mut bytes = crate::performance::wire(maximum, mode, rate)?;
            if !io.is_empty() {
                bytes[20..24]
                    .copy_from_slice(&(if notifications { 3u32 } else { 1u32 }).to_le_bytes());
                bytes.extend(io);
            }
            crate::performance::validate_wire(&bytes)?;
            let reply = control(id, 20, bytes.clone())?;
            let vendor = ap1_native_client::get(&reply[..4]) as u32;
            let total = vendor
                .checked_add(delay)
                .ok_or_else(|| invalid("latency overflow"))?;
            INSTANCES.update(id,|l| -> io::Result<()> {
                let callback=l.callback.get_mut();
                if callback.running {return Err(invalid("configuration during playback"));}
                callback.delay=u64::from(delay);l.max=maximum as usize;
                l.setup=Some(bytes);
                if let Some(path)=&l.report {
                    crate::preview::append_report(path,format!("{{\"event\":\"ap9_setup\",\"host_maximum\":{maximum},\"transport_maximum\":{},\"sample_rate\":{rate},\"bridge_frames\":{delay},\"vendor_frames\":{vendor},\"total_frames\":{total},\"vendor_precision_bits\":{}}}\n",maximum.min(256),ap1_native_client::get(&reply[8..12])).as_bytes());
                }
                Ok(())
            }).map_err(|_|invalid("setup instance ownership"))??;
            if !vendor_out.is_null() {
                *vendor_out = vendor;
            }
            *out = total;
            *out.add(1) = ap1_native_client::get(&reply[4..8]) as u32;
            Ok(())
        })();
        match result {
            Ok(()) => 0,
            Err(e) => retain(&e),
        }
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap4_activate(id: u64, max: u32, mode: u32) -> u32 {
    crate::ffi(
        || match control(id, 8, [max.to_le_bytes(), mode.to_le_bytes()].concat()) {
            Ok(_) => 0,
            Err(e) => retain(&e),
        },
    ) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap4_deactivate(id: u64) -> u32 {
    crate::ffi(|| match control(id, 14, vec![]) {
        Ok(_) => 0,
        Err(e) => retain(&e),
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap4_state(
    id: u64,
    restore: *const u8,
    n: u32,
    out: *mut u8,
    capacity: u32,
    size: *mut u32,
) -> u32 {
    crate::ffi(|| {
        if out.is_null()
            || size.is_null()
            || (capacity as usize) < state::HEADER_SIZE + state::LIMIT
            || n as usize > state::HEADER_SIZE + state::LIMIT
        {
            return 1;
        }
        let request = if restore.is_null() {
            if n != 0 {
                return 1;
            }
            Ok((16, vec![]))
        } else {
            let Some(l) = INSTANCES.lease(id) else {
                return 1;
            };
            state::bound_payload(
                l.shared.identity,
                std::slice::from_raw_parts(restore, n as usize),
            )
            .map(|p| (18, p.to_vec()))
        };
        match request.and_then(|(op, b)| control(id, op, b)) {
            Ok(b) => {
                std::ptr::copy_nonoverlapping(b.as_ptr(), out, b.len());
                *size = b.len() as u32;
                0
            }
            Err(e) => retain(&e),
        }
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap3_transition(id: u64, op: u32) -> u32 {
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let Some(_guard) = Guard::acquire(&l) else {
        return 3;
    };
    if matches!(op, START | STOP) {
        return (&mut *l.callback.get()).transition(&l.shared, op);
    }
    if op != DEACTIVATE || (*l.callback.get()).running {
        return 1;
    }
    if l.shared.fault.load(Ordering::Acquire) != 0 {
        return 2;
    }
    if !l.shared.requests.push(Item::control(DEACTIVATE, 0)) {
        return 2;
    }
    let end = Instant::now() + Duration::from_secs(20);
    while l.shared.ack.load(Ordering::Acquire) != 15 {
        if l.shared.fault.load(Ordering::Acquire) != 0 || Instant::now() > end {
            return 2;
        }
        thread::sleep(Duration::from_millis(1));
    }
    0
}
#[no_mangle]
pub unsafe extern "C" fn ap3_process(
    id: u64,
    n: u32,
    gain: f64,
    flags: u64,
    left: *const f32,
    right: *const f32,
    out_left: *mut f32,
    out_right: *mut f32,
    out_flags: *mut u64,
) -> u32 {
    ap7_process(
        id,
        n,
        gain,
        flags,
        left,
        right,
        out_left,
        out_right,
        out_flags,
        std::ptr::null_mut(),
    )
}
#[no_mangle]
pub unsafe extern "C" fn ap7_process(
    id: u64,
    n: u32,
    gain: f64,
    flags: u64,
    left: *const f32,
    right: *const f32,
    out_left: *mut f32,
    out_right: *mut f32,
    out_flags: *mut u64,
    delivery: *mut Delivery,
) -> u32 {
    process_events(
        id,
        n,
        gain,
        flags,
        left,
        right,
        out_left,
        out_right,
        out_flags,
        delivery,
        &[],
        crate::context::Context::default(),
        false,
        0,
    )
}
// Mirrors the fixed C ABI and adds a borrowed bounded event span.
#[allow(clippy::too_many_arguments)]
unsafe fn process_events(
    id: u64,
    n: u32,
    gain: f64,
    flags: u64,
    left: *const f32,
    right: *const f32,
    out_left: *mut f32,
    out_right: *mut f32,
    out_flags: *mut u64,
    delivery: *mut Delivery,
    events: &[Event],
    context: crate::context::Context,
    detailed: bool,
    entered_ns: u64,
) -> u32 {
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let Some(_guard) = Guard::acquire(&l) else {
        return 3;
    };
    let n = n as usize;
    if (n == 0 && !l.shared.state_capable.load(Ordering::Acquire))
        || n > l.max
        || flags > 3
        || !(gain.is_finite() && (0.0..=1.0).contains(&gain)
            || gain.is_nan() && l.shared.state_capable.load(Ordering::Acquire))
        || left.is_null()
        || right.is_null()
        || out_left.is_null()
        || out_right.is_null()
        || out_flags.is_null()
    {
        return if detailed { 0x101 } else { 1 };
    }
    if events.len() > MAX_EVENTS
        || (!events.is_empty() && l.shared.identity.is_none())
        || events.iter().any(|e| !e.valid_host(n))
    {
        return if detailed { 0x102 } else { 1 };
    }
    // Validate all host input before admitting any subblock. In-place output
    // can replace earlier samples only after their input has been copied.
    for (ch, p) in [left, right].into_iter().enumerate() {
        for v in std::slice::from_raw_parts(p, n) {
            if !v.is_finite() {
                return if detailed { 0x103 } else { 1 };
            }
            if flags & (1 << ch) != 0 && *v != 0. {
                return if detailed { 0x104 } else { 1 };
            }
        }
    }
    if context.chunk(n).is_none() {
        return if detailed { 0x105 } else { 1 };
    }
    {
        let callback = &mut *l.callback.get();
        callback.returned.window(callback.position, n);
        let Some(next) = callback.host_call.checked_add(1) else { return 2; };
        callback.host_call = next;
    }
    let entered_ns = if entered_ns == 0 { crate::observer::monotonic_ns() } else { entered_ns };
    let mut total = Delivery::default();
    let mut combined = 3;
    let mut offset = 0;
    loop {
        let count = (n - offset).min(CAP);
        let mut item = Item::control(AUDIO, 0);
        item.n = count as u32;
        item.parent = [(*l.callback.get()).host_call, n as u64, offset as u64, entered_ns];
        item.context = context.chunk(offset).unwrap();
        item.flags = flags;
        item.gain = if offset == 0 { gain } else { f64::NAN };
        for e in events {
            if n == 0 || e.offset as usize >= offset && (e.offset as usize) < offset + count {
                let mut local = *e;
                local.offset -= offset as u32;
                item.events[item.event_count as usize] = local;
                item.event_count += 1;
            }
        }
        for (ch, p) in [left, right].into_iter().enumerate() {
            item.data[ch][..count]
                .copy_from_slice(std::slice::from_raw_parts(p.add(offset), count));
        }
        let mut out = [[0.; CAP]; 2];
        let callback = &mut *l.callback.get();
        match callback.process(&l.shared, item, &mut out) {
            Ok(f) => {
                combined &= f;
                for (ch, p) in [out_left, out_right].into_iter().enumerate() {
                    std::ptr::copy_nonoverlapping(out[ch].as_ptr(), p.add(offset), count);
                }
                let d = callback.delivery;
                total.missing_frames += d.missing_frames;
                total.gaps += d.gaps;
                total.expired_frames += d.expired_frames;
                total.delivered_frames += d.delivered_frames;
                total.priming_frames += d.priming_frames;
            }
            Err(code) => return code,
        }
        offset += count;
        if offset == n {
            break;
        }
    }
    *out_flags = combined;
    for (counter, delta) in l.shared.delivery_totals.iter().zip([
        n as u64, total.missing_frames, total.gaps, total.expired_frames,
        total.delivered_frames, total.priming_frames,
    ]) {
        counter.store(counter.load(Ordering::Relaxed) + delta, Ordering::Release);
    }
    if !delivery.is_null() {
        *delivery = total;
    }
    0
}

#[repr(C)]
#[derive(Default)]
pub struct Stats {
    fault: u64,
    first_position: u64,
    processed: u64,
    request_high: u64,
    result_high: u64,
    position: u64,
    epoch: u64,
}
#[no_mangle]
pub unsafe extern "C" fn ap3_stats(id: u64, out: *mut Stats) -> u32 {
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let Some(_guard) = Guard::acquire(&l) else {
        return 3;
    };
    if out.is_null() {
        return 1;
    }
    *out = Stats {
        fault: l.shared.fault.load(Ordering::Acquire),
        first_position: l.shared.first_position.load(Ordering::Acquire),
        processed: l.shared.processed.load(Ordering::Acquire),
        request_high: l.shared.requests.high_water(),
        result_high: l.shared.results.high_water(),
        position: (*l.callback.get()).position,
        epoch: (*l.callback.get()).epoch,
    };
    0
}
#[no_mangle]
pub unsafe extern "C" fn ap3_close(id: u64) -> u32 {
    crate::ffi(|| {
        match INSTANCES.remove(id, |l| {
            let clean = matches!(l.shared.ack.load(Ordering::Acquire), 15 | 17)
                && l.shared.fault.load(Ordering::Acquire) == 0;
            if clean {
                if !l.shared.requests.push(Item::control(CLOSE, 0)) {
                    l.shared.quit.store(true, Ordering::Release);
                }
            } else {
                l.shared.quit.store(true, Ordering::Release);
            }
            if let Some(t) = l.worker.take() {
                if t.join().is_err() {
                    l.shared.fail(WORKER, u64::MAX);
                }
            }
            let ok = clean && l.shared.fault.load(Ordering::Acquire) == 0;
            if !ok {
                if let Ok(d) = l.shared.detail.lock() {
                    if !d.is_empty() {
                        retain(&invalid(d.as_str()));
                    }
                }
            }
            if ok {
                0
            } else {
                2
            }
        }) {
            Ok(code) => code,
            Err(code) => code as i32,
        }
    }) as u32
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Observation {
    pub comparison: state::WitnessReport,
    pub input_hash: u64,
    pub output_hash: u64,
}
#[no_mangle]
pub unsafe extern "C" fn ap5_observation(id: u64, out: *mut Observation) -> u32 {
    crate::ffi(|| {
        if out.is_null() {
            return 1;
        }
        let Some(live) = INSTANCES.lease(id) else {
            return 1;
        };
        let shared = live.shared.clone();
        drop(live);
        let Some(observer) = &shared.observer else {
            return 2;
        };
        let result = match observer.report.lock() {
            Ok(v) => {
                *out = v.observation;
                0
            }
            Err(_) => 2,
        };
        result
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap4_witness(id: u64, out: *mut state::WitnessReport) -> u32 {
    crate::ffi(|| {
        if out.is_null() {
            return 1;
        }
        let Some(live) = INSTANCES.lease(id) else {
            return 1;
        };
        let shared = live.shared.clone();
        drop(live);
        let Some(observer) = &shared.observer else {
            return 2;
        };
        let result = match observer.report.lock() {
            Ok(v) => {
                *out = v.observation.comparison;
                0
            }
            Err(_) => 2,
        };
        result
    }) as u32
}

#[repr(C)]
pub struct Failure {
    fault: u64,
    first_position: u64,
    processed: u64,
    detail: [u8; 385],
}
fn failure_snapshot(s: &Shared) -> Failure {
    let mut out = Failure {
        fault: s.fault.load(Ordering::Acquire),
        first_position: s.first_position.load(Ordering::Acquire),
        processed: s.processed.load(Ordering::Acquire),
        detail: [0; 385],
    };
    if let Ok(detail) = s.detail.lock() {
        let n = detail.len().min(out.detail.len() - 1);
        out.detail[..n].copy_from_slice(&detail.as_bytes()[..n]);
    }
    out
}
#[no_mangle]
pub unsafe extern "C" fn ap4_failure(id: u64, out: *mut Failure) -> u32 {
    crate::ffi(|| {
        if out.is_null() {
            return 1;
        }
        let Some(live) = INSTANCES.lease(id) else {
            return 1;
        };
        let shared = live.shared.clone();
        drop(live);
        *out = failure_snapshot(&shared);
        0
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap9_open(identity: *const u8, handle: *mut u64) -> u32 {
    let identity = if identity.is_null() {
        None
    } else {
        let b = std::slice::from_raw_parts(identity, 48);
        Some(state::Identity {
            class: b[..16].try_into().unwrap(),
            module: b[16..].try_into().unwrap(),
        })
    };
    open(
        256,
        handle,
        if identity.is_some() { 12 } else { 6 },
        identity,
    )
}
#[no_mangle]
pub unsafe extern "C" fn ap8_open(identity: *const u8, handle: *mut u64) -> u32 {
    if identity.is_null() {
        return 1;
    }
    let b = std::slice::from_raw_parts(identity, 48);
    open(
        256,
        handle,
        5,
        Some(state::Identity {
            class: b[..16].try_into().unwrap(),
            module: b[16..].try_into().unwrap(),
        }),
    )
}
#[no_mangle]
pub unsafe extern "C" fn ap8_process(
    id: u64,
    n: u32,
    events: *const Event,
    count: u32,
    left: *const f32,
    right: *const f32,
    out_left: *mut f32,
    out_right: *mut f32,
    out_flags: *mut u64,
    delivery: *mut Delivery,
) -> u32 {
    if count as usize > MAX_EVENTS || (count > 0 && events.is_null()) {
        return 1;
    }
    let events = if count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(events, count as usize)
    };
    process_events(
        id,
        n,
        f64::NAN,
        0,
        left,
        right,
        out_left,
        out_right,
        out_flags,
        delivery,
        events,
        crate::context::Context::default(),
        false,
        0,
    )
}

#[no_mangle]
pub unsafe extern "C" fn ap10_process(
    id: u64,
    n: u32,
    events: *const Event,
    count: u32,
    context: *const crate::context::Context,
    flags: u64,
    left: *const f32,
    right: *const f32,
    out_left: *mut f32,
    out_right: *mut f32,
    out_flags: *mut u64,
    delivery: *mut Delivery,
) -> u32 {
    ap13_process(id,n,events,count,context,flags,left,right,out_left,out_right,out_flags,delivery,0)
}
#[no_mangle]
pub unsafe extern "C" fn ap13_process(
    id: u64,
    n: u32,
    events: *const Event,
    count: u32,
    context: *const crate::context::Context,
    flags: u64,
    left: *const f32,
    right: *const f32,
    out_left: *mut f32,
    out_right: *mut f32,
    out_flags: *mut u64,
    delivery: *mut Delivery,
    entered_ns: u64,
) -> u32 {
    if count as usize > MAX_EVENTS || (count > 0 && events.is_null()) || context.is_null() {
        return 1;
    }
    let events = if count == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(events, count as usize)
    };
    process_events(
        id,
        n,
        f64::NAN,
        flags,
        left,
        right,
        out_left,
        out_right,
        out_flags,
        delivery,
        events,
        *context,
        true,
        entered_ns,
    )
}

#[no_mangle]
pub unsafe extern "C" fn ap10_notices(id: u64, out: *mut u32) -> u32 {
    if out.is_null() {
        return 1;
    }
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let flags = l.shared.notices.swap(0, Ordering::AcqRel);
    let traits = l.shared.notice_traits.load(Ordering::Acquire);
    *out = flags as u32;
    *out.add(1) = traits as u32;
    *out.add(2) = (traits >> 32) as u32;
    0
}

#[no_mangle]
pub extern "C" fn ap10_results_abi_version() -> u32 {
    1
}
// UI access leases the same instance/generation but never takes the callback
// guard, state mailbox or transport-worker lock. Retirement invalidates it.
#[no_mangle]
pub unsafe extern "C" fn ap11_gui_generation(id: u64, out: *mut u64) -> u32 {
    if out.is_null() {
        return 4;
    }
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    if l.shared.gui.is_none() {
        return 1;
    }
    *out = l.shared.generation;
    0
}
fn gui_call(id: u64, generation: u64, f: impl FnOnce(&crate::gui::Gui) -> u32) -> u32 {
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    if generation != l.shared.generation {
        return 5;
    }
    let Some(gui) = &l.shared.gui else {
        return 1;
    };
    f(gui)
}
#[no_mangle]
pub unsafe extern "C" fn ap11_gui_command(
    id: u64,
    generation: u64,
    m: *mut crate::gui::Message,
) -> u32 {
    if !crate::gui::Message::valid_prefix(m) {
        return 4;
    }
    gui_call(id, generation, |gui| gui.send(&mut *m))
}
#[no_mangle]
pub unsafe extern "C" fn ap11_gui_take(
    id: u64,
    generation: u64,
    m: *mut crate::gui::Message,
) -> u32 {
    if !crate::gui::Message::valid_prefix(m) {
        return 4;
    }
    gui_call(id, generation, |gui| gui.take(&mut *m))
}
#[no_mangle]
pub extern "C" fn ap11_gui_capabilities(id: u64, generation: u64, caps: u32) -> u32 {
    gui_call(id, generation, |gui| gui.capabilities(caps))
}
#[no_mangle]
pub extern "C" fn ap11_gui_failure(id: u64, generation: u64, code: u32) -> u32 {
    gui_call(id, generation, |gui| gui.fail(code))
}
#[no_mangle]
pub unsafe extern "C" fn ap10_take_results(
    id: u64,
    out: *mut crate::process_results::Packet,
) -> u32 {
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let Some(_g) = Guard::acquire(&l) else {
        return 3;
    };
    if out.is_null() {
        return 1;
    }
    if l.shared.fault.load(Ordering::Acquire) != 0 {
        return 2;
    }
    (*l.callback.get()).returned.take(&mut *out);
    0
}
#[no_mangle]
pub unsafe extern "C" fn ap10_result_stats(
    id: u64,
    out: *mut crate::process_results::Stats,
) -> u32 {
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let Some(_g) = Guard::acquire(&l) else {
        return 3;
    };
    if out.is_null() {
        return 1;
    }
    *out = (*l.callback.get()).returned.stats();
    0
}
#[no_mangle]
pub unsafe extern "C" fn ap10_fail_results(id: u64) -> u32 {
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let Some(_g) = Guard::acquire(&l) else {
        return 3;
    };
    l.shared.fail(5, (*l.callback.get()).position);
    (*l.callback.get()).returned.reset();
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gui_abi_rejects_short_prefix_before_forming_full_message() {
        for mut prefix in [[2u32, 584u32], [4, 8], [3, 608], [5, 608], [0, 0]] {
            let pointer = prefix.as_mut_ptr().cast::<crate::gui::Message>();
            unsafe {
                assert_eq!(ap11_gui_command(0, 0, pointer), 4);
                assert_eq!(ap11_gui_take(0, 0, pointer), 4);
            }
        }
        unsafe {
            assert_eq!(ap11_gui_command(0, 0, std::ptr::null_mut()), 4);
            assert_eq!(ap11_gui_take(0, 0, std::ptr::null_mut()), 4);
        }
    }
    #[test]
    fn parent_callbacks_preserve_exact_one_and_two_proxy_delay() {
        // The consumer runs only after the complete parent host callback. A
        // 512-frame parent must not acquire an artificial wait between chunks.
        for (maximum, delay) in [(512, 512), (256, 512), (256, 256), (128, 256)] {
            let mut ids = Vec::new();
            let mut peers = Vec::new();
            for _ in 0..2 {
                let shared = Arc::new(Shared::new());
                shared.state_capable.store(true, Ordering::Release);
                let mut callback = Callback::new();
                callback.delay = delay as u64;
                assert_eq!(callback.transition(&shared, START), 0);
                shared.requests.pop().unwrap();
                ids.push(INSTANCES.insert(|| Ok::<_, ()>(Live {
                    shared: shared.clone(), callback: UnsafeCell::new(callback),
                    busy: AtomicBool::new(false), worker: None, report: None,
                    max: maximum, recovery_blocked: false, installed_delay: Some(delay as u32),
                    minor: 11, setup: None,
                })).unwrap().unwrap());
                peers.push(shared);
            }
            let mut position = 0;
            for parent in 1..65 {
                let n = if parent % 3 == 0 { 64 } else { maximum };
                let input: Vec<f32> = (position..position+n).map(|i| (i+1) as f32).collect();
                let mut previous = input;
                for (device, id) in ids.iter().enumerate() {
                    let mut left = vec![0.; n]; let mut right = vec![0.; n];
                    let mut flags = 0; let mut delivery = Delivery::default();
                    assert_eq!(unsafe { ap13_process(*id,n as u32,std::ptr::null(),0,
                        &crate::context::Context::default(),0,previous.as_ptr(),previous.as_ptr(),
                        left.as_mut_ptr(),right.as_mut_ptr(),&mut flags,&mut delivery,parent*1000) },0);
                    for i in 0..n {
                        let lag = delay*(device+1);
                        let expected = if position+i < lag { 0. } else { (position+i-lag+1) as f32 };
                        assert_eq!(left[i],expected); assert_eq!(right[i],expected);
                    }
                    assert_eq!(delivery.missing_frames,0);
                    assert_eq!(delivery.expired_frames,0);
                    previous = left;
                }
                for peer in &peers {
                    let mut offset = 0;
                    while let Some(item) = peer.requests.pop() {
                        assert_eq!(item.parent,[parent,n as u64,offset as u64,parent*1000]);
                        assert_eq!(item.position, (position+offset) as u64);
                        offset += item.n as usize;
                        assert!(peer.results.push(Completion::from(item)));
                    }
                    assert_eq!(offset,n);
                }
                position += n;
            }
            for id in ids { INSTANCES.remove(id, |_| ()).unwrap(); }
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn parent_host_blocks_do_not_repeatedly_fault_in_queue_storage() {
        // Exercise every queue slot through the real chunking callback. Query
        // thread-local counters in this host consumer, never in process().
        #[repr(C)] struct Usage { times: [i64; 4], counters: [i64; 14] }
        unsafe extern "C" { fn getrusage(who: i32, out: *mut Usage) -> i32; }
        fn faults() -> i64 {
            let mut u = Usage { times: [0; 4], counters: [0; 14] };
            assert_eq!(unsafe { getrusage(1, &mut u) }, 0);
            u.counters[4]
        }
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared: shared.clone(), callback: UnsafeCell::new(Callback::new()),
            busy: AtomicBool::new(false), worker: None, report: None,
            max: 512, recovery_blocked: false, installed_delay: None,
            minor: 11, setup: None,
        })).unwrap().unwrap();
        let input = [0f32;512]; let mut output = [[0f32;512];2];
        assert_eq!(unsafe { ap3_transition(id, START) }, 0);
        shared.requests.pop().unwrap();
        let mut total=0; let mut maximum=0; let mut first=0;
        for block in 0..1100 {
            let mut flags=0; let mut d=Delivery::default();
            let before=faults();
            let rc=unsafe { ap10_process(id,512,std::ptr::null(),0,
                &crate::context::Context::default(),3,input.as_ptr(),input.as_ptr(),
                output[0].as_mut_ptr(),output[1].as_mut_ptr(),&mut flags,&mut d) };
            let delta=faults()-before;
            assert_eq!(rc,0); total+=delta; maximum=maximum.max(delta);if block==0 {first=delta;}
            // Same parent 512-frame host block: both chunks have already been
            // admitted before its off-thread consumer can return completions.
            for _ in 0..2 {
                let item=shared.requests.pop().unwrap();
                assert!(shared.results.push(Completion::from(item)));
            }
        }
        eprintln!("AP13 parent512 callback minor faults: first={first} total={total} max={maximum}");
        INSTANCES.remove(id, |_| ()).unwrap();
        // A host's fresh callback thread can fault in its code/stack on the
        // first invocation (12 pages in the unoptimized CI host). It is not
        // memory owned by this instance. The following calls still traverse
        // every newly used queue page; the old implementation faults throughout.
        assert_eq!(total-first, 0, "queue pages must be touched before callback use");
    }
    #[test]
    fn acknowledged_setup_is_retained_for_recovery() {
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = INSTANCES
            .insert(|| {
                Ok::<_, ()>(Live {
                    shared: shared.clone(),
                    callback: UnsafeCell::new(Callback::new()),
                    busy: AtomicBool::new(false),
                    worker: None,
                    report: None,
                    max: 256,
                    recovery_blocked: false,
                installed_delay: None,
                    minor: 6,
                    setup: None,
                })
            })
            .unwrap()
            .unwrap();
        let peer = shared.clone();
        let responder = thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(5);
            loop {
                if peer.pending_control.load(Ordering::Acquire) {
                    let mut mailbox = peer.control.lock().unwrap();
                    let c = mailbox.as_mut().unwrap();
                    assert_eq!(c.op, 20);
                    assert_eq!(c.bytes, crate::performance::wire(128, 0, 96000.).unwrap());
                    c.result = Some(Ok([
                        7u32.to_le_bytes(),
                        18u32.to_le_bytes(),
                        1u32.to_le_bytes(),
                        0u32.to_le_bytes(),
                    ]
                    .concat()));
                    peer.pending_control.store(false, Ordering::Release);
                    break;
                }
                assert!(Instant::now() < until);
                thread::yield_now();
            }
        });
        let mut traits = [0u32; 2];
        assert_eq!(
            unsafe { ap9_setup(id, 128, 0, 96000., traits.as_mut_ptr()) },
            0
        );
        responder.join().unwrap();
        INSTANCES
            .remove(id, |l| {
                assert_eq!(
                    l.setup,
                    Some(crate::performance::wire(128, 0, 96000.).unwrap())
                );
                assert_eq!(l.max, 128);
                assert_eq!(u64::from(traits[0]), l.callback.get_mut().delay + 7);
                assert_eq!(traits[1], 18);
            })
            .unwrap();
    }
    #[test]
    fn large_host_blocks_preserve_notes_and_parameter_offsets() {
        let mut shared = Shared::new();
        shared.state_capable.store(true, Ordering::Relaxed);
        shared.identity = Some(state::Identity {
            class: [1; 16],
            module: [2; 32],
        });
        let shared = Arc::new(shared);
        let mut cb = Callback::new();
        assert_eq!(cb.transition(&shared, START), 0);
        let id = INSTANCES
            .insert(|| {
                Ok::<_, ()>(Live {
                    shared: shared.clone(),
                    callback: UnsafeCell::new(cb),
                    busy: AtomicBool::new(false),
                    worker: None,
                    report: None,
                    max: 1024,
                    recovery_blocked: false,
                installed_delay: None,
                    minor: 7,
                    setup: None,
                })
            })
            .unwrap()
            .unwrap();
        let input = [0f32; 1024];
        let mut left = [0f32; 1024];
        let mut right = [0f32; 1024];
        let mut flags = 0;
        let mut delivery = Delivery::default();
        let events = [
            Event {
                offset: 7,
                kind: 0,
                id: 1,
                pitch: 60,
                value: 0.7,
                ..Default::default()
            },
            Event {
                offset: 511,
                kind: 2,
                id: 900,
                value: 0.25,
                ..Default::default()
            },
            Event {
                offset: 1023,
                kind: 1,
                id: 1,
                pitch: 60,
                value: 0.2,
                ..Default::default()
            },
        ];
        unsafe {
            assert_eq!(
                ap8_process(
                    id,
                    1024,
                    events.as_ptr(),
                    3,
                    input.as_ptr(),
                    input.as_ptr(),
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    &mut flags,
                    &mut delivery
                ),
                0
            );
        }
        assert_eq!(delivery.priming_frames, 1024);
        assert_eq!(shared.requests.pop().unwrap().kind, START);
        let mut observed = Vec::new();
        for position in [0, 256, 512, 768] {
            let item = shared.requests.pop().unwrap();
            assert_eq!(item.position, position);
            assert_eq!(item.n, 256);
            for e in &item.events[..item.event_count as usize] {
                assert!(e.valid(256));
                let mut e = *e;
                e.offset += position as u32;
                observed.push(e);
            }
        }
        assert_eq!(observed, events);
        let count = shared.requests.published();
        let bad = Event {
            offset: 1024,
            ..events[0]
        };
        unsafe {
            assert_eq!(
                ap8_process(
                    id,
                    1024,
                    &bad,
                    1,
                    input.as_ptr(),
                    input.as_ptr(),
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    &mut flags,
                    &mut delivery
                ),
                1
            );
        }
        assert_eq!(shared.requests.published(), count);
        // AP10 must reject an invalid last sample/context before admitting the
        // first chunk, and keep the legacy return codes unchanged above.
        let valid_context = crate::context::Context {
            present: 1,
            state: 2,
            rate: 48000.,
            project: 100,
            ..Default::default()
        };
        for (last, input_flags, context, expected) in [
            (f32::NAN, 0, valid_context, 0x103),
            (1., 1, valid_context, 0x104),
            (
                0.,
                0,
                crate::context::Context {
                    rate: 0.,
                    ..valid_context
                },
                0x105,
            ),
        ] {
            let mut bad_input = [0f32; 1024];
            bad_input[1023] = last;
            left.fill(7.);
            right.fill(8.);
            let result = unsafe {
                ap10_process(
                    id,
                    1024,
                    std::ptr::null(),
                    0,
                    &context,
                    input_flags,
                    bad_input.as_ptr(),
                    input.as_ptr(),
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    &mut flags,
                    &mut delivery,
                )
            };
            assert_eq!(result, expected);
            assert_eq!(shared.requests.published(), count);
            assert_eq!(left, [7.; 1024]);
            assert_eq!(right, [8.; 1024]);
        }
        // A valid in-place effect block preserves both inputs and the actual
        // host context before delayed output replaces its buffers.
        left.fill(0.25);
        right.fill(-0.5);
        assert_eq!(
            unsafe {
                ap10_process(
                    id,
                    512,
                    events.as_ptr(),
                    2,
                    &valid_context,
                    0,
                    left.as_ptr(),
                    right.as_ptr(),
                    left.as_mut_ptr(),
                    right.as_mut_ptr(),
                    &mut flags,
                    &mut delivery,
                )
            },
            0
        );
        for offset in [0, 256] {
            let item = shared.requests.pop().unwrap();
            assert_eq!(item.context.project, 100 + offset);
            assert_eq!(item.data[0], [0.25; CAP]);
            assert_eq!(item.data[1], [-0.5; CAP]);
            assert_eq!(item.event_count, 1);
            assert_eq!(item.events[0].offset, if offset == 0 { 7 } else { 255 });
        }
        INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn actual_transport_progresses_with_paused_observation_consumer_and_reader() {
        use ap1_native_client::{
            endpoint::{receive_version, send_version},
            mapping::Mapping,
            ClientState, Frame, Slot, INPUT, OUTPUT, STRIDE,
        };
        use std::os::unix::fs::FileExt;
        let path = std::env::temp_dir().join(format!(
            "ap7-observer-{}.audio",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        let mapping = Mapping::new(&path).unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let socket = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        socket.set_nodelay(true).unwrap();
        let (mut remote, _) = listener.accept().unwrap();
        remote.set_nodelay(true).unwrap();
        let peer = thread::spawn(move || {
            let mut processed = 0;
            loop {
                let f = receive_version(&mut remote, 5, 4).unwrap();
                let payload = if f.kind == 3 {
                    assert_eq!(ap1_native_client::get(&f.payload[40..48]), processed * 256);
                    let mut bytes = [0u8; CAP * 4];
                    for ch in 0..2 {
                        file.read_exact_at(&mut bytes, (INPUT + ch * STRIDE + 4) as u64)
                            .unwrap();
                        for value in bytes.chunks_exact_mut(4) {
                            let sample = f32::from_le_bytes(value.try_into().unwrap()) * 0.5;
                            value.copy_from_slice(&sample.to_le_bytes());
                        }
                        file.write_all_at(&bytes, (OUTPUT + ch * STRIDE + 4) as u64)
                            .unwrap();
                    }
                    processed += 1;
                    [
                        256u32.to_le_bytes().as_slice(),
                        (OUTPUT as u32).to_le_bytes().as_slice(),
                        0u64.to_le_bytes().as_slice(),
                        &f.payload[32..48],
                    ]
                    .concat()
                } else if matches!(f.kind, 10 | 12) {
                    f.payload.clone()
                } else {
                    vec![]
                };
                send_version(
                    &mut remote,
                    &Frame {
                        kind: f.kind + 1,
                        session: f.session,
                        sequence: f.sequence,
                        payload,
                    },
                    5,
                    4,
                )
                .unwrap();
                if f.kind == 5 {
                    return processed;
                }
            }
        });
        let mut observer = crate::observer::Observer::paused();
        observer.state(
            &[0.5f32.to_le_bytes(), 0f32.to_le_bytes(), 0u32.to_le_bytes()].concat(),
            true,
        );
        let observation = observer.shared.clone();
        let session = Session {
            gui: None,
            gui_revision: 0,
            mailbox: None,
            mailbox_enabled: false,
                capture: None,
        fault_status: None,
            notices: (0, 0),
            returned: crate::process_results::Packet::default(),
            mapping: Some(mapping),
            socket,
            state: ClientState {
                session: [1; 16],
                next: 1,
                slot: Slot::Writable,
            },
            phase: 9,
            max: CAP,
            minor: 4,
            identity: None,
            epoch: 0,
            position: 0,
            witness: Some(observer),
            trace: Default::default(),
            sample_rate: 48000,
            armed: false,
            owner: None,
        };
        let mut shared = Shared::new();
        shared.observer = Some(observation.clone());
        let shared = Arc::new(shared);
        let service = shared.clone();
        // Both diagnostic endpoints are unavailable throughout real mapped/TCP
        // processing. This catches waits added anywhere in the worker loop.
        let reader = observation.report.lock().unwrap();
        let transport = thread::spawn(move || worker(session, service, None));
        let mut callback = Callback::new();
        assert_eq!(callback.transition(&shared, START), 0);
        let mut item = Item::control(AUDIO, 0);
        item.n = 256;
        item.gain = 0.5;
        item.data = [[0.25; CAP]; 2];
        let mut output = [[0.; CAP]; 2];
        for n in 0..128 {
            callback.process(&shared, item, &mut output).unwrap();
            assert_eq!(callback.delivery.missing_frames, 0);
            assert_eq!(output, [[if n < 4 { 0. } else { 0.125 }; CAP]; 2]);
            let end = Instant::now() + Duration::from_secs(2);
            while shared.processed.load(Ordering::Acquire) <= n {
                assert!(Instant::now() < end, "observer stalled actual transport");
                assert_eq!(shared.fault.load(Ordering::Acquire), 0);
                thread::sleep(Duration::from_millis(1));
            }
        }
        assert!(observation.dropped.load(Ordering::Relaxed) > 0);
        assert_eq!(callback.transition(&shared, STOP), 0);
        assert!(shared.requests.push(Item::control(DEACTIVATE, 0)));
        assert!(shared.requests.push(Item::control(CLOSE, 0)));
        transport.join().unwrap();
        assert_eq!(peer.join().unwrap(), 128);
        assert_eq!(shared.fault.load(Ordering::Acquire), 0);
        assert_eq!(observation.offered.load(Ordering::Relaxed), 128 * 512);
        assert_eq!(reader.observation.comparison.samples, 0); // none checked
        drop(reader);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn correlated_save_refusal_keeps_worker_audio_snapshot_and_sibling() {
        use ap1_native_client::{
            endpoint::{receive_version, send_version},
            mapping::Mapping,
            ClientState, Frame, Slot, OUTPUT, STRIDE,
        };
        use std::os::unix::fs::FileExt;
        fn fixture() -> (u64, Arc<Shared>, thread::JoinHandle<()>, std::path::PathBuf) {
            let path = std::env::temp_dir().join(format!(
                "ap12-state-{}",
                u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
            ));
            let mapping = Mapping::new(&path).unwrap();
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let socket = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
            let (mut remote, _) = listener.accept().unwrap();
            let peer = thread::spawn(move || {
                let mut saves = 0;
                let mut next = 1;
                while let Ok(f) = receive_version(&mut remote, 5, 11) {
                    assert_eq!(f.sequence, next);
                    let mut kind = f.kind + 1;
                    let payload = match f.kind {
                        16 => {
                            saves += 1;
                            next += 1;
                            if saves == 2 {
                                kind = 7;
                                [
                                    1u32.to_le_bytes(),
                                    16u32.to_le_bytes(),
                                    1u32.to_le_bytes(),
                                    1u32.to_le_bytes(),
                                ]
                                .concat()
                            } else {
                                // v2 opaque state, one explicitly unavailable parameter.
                                let mut b = vec![0; 35];
                                b[0] = 3;
                                b[8] = 1;
                                b[12] = 2;
                                b[16..19].copy_from_slice(&[0xde, 0xad, 0xff]);
                                b[19..23].copy_from_slice(&42u32.to_le_bytes());
                                b
                            }
                        }
                        3 => {
                            next += 1;
                            for ch in 0..2 {
                                file.write_all_at(
                                    &0.25f32.to_le_bytes(),
                                    (OUTPUT + ch * STRIDE + 4) as u64,
                                )
                                .unwrap();
                            }
                            let mut b = vec![0; 72];
                            b[..4].copy_from_slice(&1u32.to_le_bytes());
                            b[4..8].copy_from_slice(&(OUTPUT as u32).to_le_bytes());
                            b[16..32].copy_from_slice(&f.payload[32..48]);

                            b
                        }
                        12 => f.payload.clone(),
                        _ => vec![],
                    };
                    send_version(
                        &mut remote,
                        &Frame {
                            kind,
                            session: f.session,
                            sequence: f.sequence,
                            payload,
                        },
                        5,
                        11,
                    )
                    .unwrap();
                    if f.kind == 5 {
                        break;
                    }
                }
            });
            let identity = Some(state::Identity {
                class: [4; 16],
                module: [5; 32],
            });
            let session = Session {
                gui: None,
                gui_revision: 0,
                mailbox: None,
                mailbox_enabled: false,
                capture: None,
        fault_status: None,
                notices: (0, 0),
                returned: Default::default(),
                mapping: Some(mapping),
                socket,
                state: ClientState {
                    session: [4; 16],
                    next: 1,
                    slot: Slot::Writable,
                },
                phase: 11,
                max: CAP,
                minor: 11,
                identity,
                epoch: 1,
                position: 0,
                witness: None,
                trace: Default::default(),
                sample_rate: 48000,
                armed: false,
                owner: None,
            };
            let mut s = Shared::new();
            s.identity = identity;
            let shared = Arc::new(s);
            shared.state_capable.store(true, Ordering::Release);
            shared.wanted.store(1, Ordering::Release);
            let service = shared.clone();
            let worker = thread::spawn(move || super::worker(session, service, None));
            let id = INSTANCES
                .insert(|| {
                    Ok::<_, ()>(Live {
                        shared: shared.clone(),
                        callback: UnsafeCell::new(Callback::new()),
                        busy: AtomicBool::new(false),
                        worker: Some(worker),
                        report: None,
                        max: CAP,
                        recovery_blocked: false,
                installed_delay: None,
                        minor: 11,
                        setup: None,
                    })
                })
                .unwrap()
                .unwrap();
            (id, shared, peer, path)
        }
        let a = fixture();
        let b = fixture();
        let mut bytes = vec![0; state::LIMIT + state::HEADER_SIZE];
        let mut n = 0;
        unsafe {
            assert_eq!(
                ap4_state(
                    a.0,
                    std::ptr::null(),
                    0,
                    bytes.as_mut_ptr(),
                    bytes.len() as u32,
                    &mut n
                ),
                0
            );
        }
        let snapshot = a.1.snapshots.lock().unwrap().latest().unwrap().clone();
        assert_eq!(snapshot.revision, 1);
        assert_eq!(&snapshot.bytes[8..12], &3u32.to_le_bytes());
        bytes.fill(0xA5);
        n = 123;
        unsafe {
            assert_eq!(
                ap4_state(
                    a.0,
                    std::ptr::null(),
                    0,
                    bytes.as_mut_ptr(),
                    bytes.len() as u32,
                    &mut n
                ),
                5
            );
        }
        assert_eq!(n, 123);
        assert!(bytes.iter().all(|v| *v == 0xA5));
        let prior = a.1.snapshots.lock().unwrap().latest().unwrap().clone();
        assert_eq!(prior.revision, 1);
        assert_eq!(prior.digest, snapshot.digest);
        assert_eq!(a.1.fault.load(Ordering::Acquire), 0);
        for s in [&a.1, &b.1] {
            let mut item = Item::control(AUDIO, 1);
            item.n = 1;
            item.gain = f64::NAN;
            assert!(s.requests.push(item));
            let end = Instant::now() + Duration::from_secs(3);
            while s.processed.load(Ordering::Acquire) == 0 {
                if s.fault.load(Ordering::Acquire) != 0 {
                    thread::sleep(Duration::from_millis(10));
                    panic!("{:?}", s.detail.lock().unwrap());
                }
                assert!(Instant::now() < end);
                thread::yield_now();
            }
            let output = loop {
                if let Some(v) = s.results.pop() {
                    break v;
                }
                assert!(Instant::now() < end);
                thread::yield_now();
            };
            assert_eq!(output.audio.data[0][0], 0.25);
            assert_eq!(s.fault.load(Ordering::Acquire), 0);
        }
        unsafe {
            assert_eq!(
                ap4_state(
                    a.0,
                    std::ptr::null(),
                    0,
                    bytes.as_mut_ptr(),
                    bytes.len() as u32,
                    &mut n
                ),
                0
            );
        }
        assert_eq!(a.1.snapshots.lock().unwrap().latest().unwrap().revision, 2);
        for (id, s, peer, path) in [a, b] {
            assert!(s.requests.push(Item::control(STOP, 1)));
            assert!(s.requests.push(Item::control(DEACTIVATE, 1)));
            assert!(s.requests.push(Item::control(CLOSE, 1)));
            INSTANCES
                .remove(id, |live| live.worker.take().unwrap().join().unwrap())
                .unwrap();
            peer.join().unwrap();
            assert_eq!(s.fault.load(Ordering::Acquire), 0);
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn first_callback_fault_survives_later_worker_failure() {
        let s = Shared::new();
        s.wanted.store(2, Ordering::Release);
        s.worker_op.store(AUDIO as u64, Ordering::Release);
        s.worker_epoch.store(2, Ordering::Release);
        s.worker_position.store(0, Ordering::Release);
        s.fail(CORRELATION, 1024);
        s.worker_op.store(18, Ordering::Release);
        s.fail(WORKER, 2048);
        *s.detail.lock().unwrap() = "queued session fault or cancelled".into();
        let report = failure_snapshot(&s);
        assert_eq!(report.fault, CORRELATION);
        assert_eq!(report.first_position, 1024);
        assert_eq!(report.processed, 0);
        assert!(s.first_context_ready.load(Ordering::Acquire));
        assert_eq!(s.first_epoch.load(Ordering::Relaxed), 2);
        assert_eq!(s.first_worker_op.load(Ordering::Relaxed), AUDIO as u64);
        assert!(report.detail.starts_with(b"queued session fault"));
    }
    fn pump(s: &Shared) {
        while let Some(mut r) = s.requests.pop() {
            if r.kind == AUDIO {
                for ch in 0..2 {
                    for i in 0..r.n as usize {
                        r.data[ch][i] *= r.gain as f32;
                    }
                }
                r.flags = if r.gain == 0. { 3 } else { r.flags };
                assert!(s.results.push(r.into()));
            }
        }
    }
    #[test]
    fn stalled_gui_and_stale_generation_do_not_hold_audio_delivery() {
        let path = std::env::temp_dir().join(format!(
            "ap11-queued-{}-{}",
            std::process::id(),
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        let gui = Arc::new(crate::gui::Gui::create(&path, [19; 16]).unwrap());
        let mut shared = Shared::new();
        shared.gui = Some(gui.clone());
        shared.generation = 8;
        let shared = Arc::new(shared);
        let id = INSTANCES
            .insert(|| {
                Ok::<_, ()>(Live {
                    shared: shared.clone(),
                    callback: UnsafeCell::new(Callback::new()),
                    busy: AtomicBool::new(false),
                    worker: None,
                    report: None,
                    max: 256,
                    recovery_blocked: false,
                installed_delay: None,
                    minor: 10,
                    setup: None,
                })
            })
            .unwrap()
            .unwrap();
        let mut message = crate::gui::Message {
            kind: 1,
            ..Default::default()
        };
        assert_eq!(unsafe { ap11_gui_command(id, 7, &mut message) }, 5);
        assert_eq!(ap11_gui_failure(id, 8, 0), 0);
        for _ in 0..crate::gui::CAPACITY {
            assert_eq!(unsafe { ap11_gui_command(id, 8, &mut message) }, 0);
        }
        assert_eq!(unsafe { ap11_gui_command(id, 8, &mut message) }, 3);
        assert_eq!(ap11_gui_failure(id, 8, 0), 1);
        let mut cb = Callback::new();
        cb.delay = 512;
        assert_eq!(cb.transition(&shared, START), 0);
        let mut request = Item::control(AUDIO, 0);
        request.n = 256;
        request.gain = 0.5;
        request.data = [[0.25; CAP]; 2];
        request.event_count = 1;
        request.events[0].kind = 2;
        request.events[0].value = 0.5;
        let mut out = [[0.; CAP]; 2];
        for i in 0..64 {
            cb.process(&shared, request, &mut out).unwrap();
            assert_eq!(cb.delivery.missing_frames, 0);
            assert_eq!(out, [[if i < 2 { 0. } else { 0.125 }; CAP]; 2]);
            while let Some(mut admitted) = shared.requests.pop() {
                if admitted.kind == AUDIO {
                    assert!(admitted.gui_revision > 0);
                    for channel in &mut admitted.data {
                        for value in channel {
                            *value *= 0.5;
                        }
                    }
                    assert!(shared.results.push(admitted.into()));
                }
            }
        }
        assert_eq!(shared.fault.load(Ordering::Acquire), 0);
        message.kind = 2;
        message.native_view = 1;
        assert_eq!(unsafe { ap11_gui_command(id, 8, &mut message) }, 0);
        INSTANCES.remove(id, |_| ()).unwrap();
        assert_eq!(unsafe { ap11_gui_command(id, 8, &mut message) }, 1);
        drop(shared);
        drop(gui);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn variable_lengths_delayed_gain_and_silence_are_exact() {
        let s = Shared::new();
        let mut cb = Callback::new();
        assert_eq!(cb.transition(&s, START), 0);
        pump(&s);
        let mut reference = Vec::new();
        let mut count = 0;
        for b in 0..4000 {
            let n = [1, 16, 63, 256][b % 4];
            let gain = [0.5, 0.25, 0., 0.75][b % 4];
            let mut r = Item::control(AUDIO, 0);
            r.n = n as u32;
            r.gain = gain;
            r.flags = if b % 7 == 0 { 1 } else { 0 };
            for i in 0..n {
                r.data[0][i] = if r.flags == 1 {
                    0.
                } else {
                    ((count + i) % 17) as f32 / 32. - 0.25
                };
                r.data[1][i] = ((count + i) % 13) as f32 / 32. - 0.125;
                reference.push([r.data[0][i] * gain as f32, r.data[1][i] * gain as f32]);
            }
            let mut out = [[999.; CAP]; 2];
            cb.process(&s, r, &mut out).unwrap();
            for i in 0..n {
                for (ch, plane) in out.iter().enumerate() {
                    assert_eq!(
                        plane[i],
                        if count + i < DELAY as usize {
                            0.
                        } else {
                            reference[count + i - DELAY as usize][ch]
                        }
                    );
                }
            }
            count += n;
            pump(&s);
        }
        assert_eq!(s.fault.load(Ordering::Relaxed), 0);
        assert_eq!(cb.transition(&s, STOP), 0);
    }
    #[test]
    fn late_output_expires_only_past_samples_and_preserves_ordered_inputs() {
        let s = Shared::new();
        let mut cb = Callback::new();
        cb.transition(&s, START);
        let mut request = Item::control(AUDIO, 0);
        request.n = 256;
        request.gain = 0.5;
        request.data = [[0.25; CAP]; 2];
        let mut out = [[99.; CAP]; 2];
        for _ in 0..4 {
            cb.process(&s, request, &mut out).unwrap();
        }
        // First half of source block zero misses its presentation deadline.
        request.n = 128;
        cb.process(&s, request, &mut out).unwrap();
        assert_eq!(cb.delivery.missing_frames, 128);
        assert_eq!(cb.delivery.gaps, 1);
        assert_eq!(s.fault.load(Ordering::Acquire), 0);
        assert_eq!(cb.position, 1152);
        // All five admitted requests, including the gap callback, run once.
        assert_eq!(s.requests.published(), 6); // includes START
        pump(&s);
        cb.process(&s, request, &mut out).unwrap();
        assert_eq!(cb.delivery.expired_frames, 128);
        assert_eq!(cb.delivery.delivered_frames, 128);
        assert_eq!(cb.delivery.missing_frames, 0);
        assert_eq!(&out[0][..128], &[0.125; 128]);
        assert_eq!(cb.epoch, 1);
        assert_eq!(cb.next_result, 1152); // all five completions decoded before presentation
        pump(&s);
        // A second gap can outlive several complete returned blocks.
        let s = Shared::new();
        let mut cb = Callback::new();
        cb.transition(&s, START);
        request.n = 256;
        for _ in 0..7 {
            cb.process(&s, request, &mut out).unwrap();
        }
        assert_eq!(cb.delivery.missing_frames, 256);
        assert_eq!(cb.delivery.gaps, 0); // one contiguous gap, not three
        pump(&s);
        cb.process(&s, request, &mut out).unwrap();
        assert_eq!(cb.delivery.expired_frames, 768);
        assert_eq!(out, [[0.125; CAP]; 2]);
        assert_eq!(cb.delivery.missing_frames, 0);
        assert_eq!(cb.epoch, 1);
    }
    #[test]
    fn production_audio_expiry_keeps_required_returned_note_off_and_zero_flush() {
        let s = Shared::new();
        let mut cb = Callback::new();
        cb.delay = 128;
        cb.transition(&s, START);
        let mut request = Item::control(AUDIO, 0);
        request.n = 128;
        let mut out = [[0.; CAP]; 2];
        for _ in 0..4 {
            cb.returned.window(cb.position, 128);
            cb.process(&s, request, &mut out).unwrap();
        }
        assert_eq!(cb.delivery.missing_frames, 128);
        let mut first = Completion::from(Item::control(AUDIO, 1));
        first.audio.n = 128;
        first.returned.events = 1;
        first.returned.event[0] = crate::process_results::Event {
            kind: 1,
            a: -77,
            offset: 7,
            ..Default::default()
        };
        assert!(s.results.push(first));
        let mut flush = Completion::from(Item::control(AUDIO, 1));
        flush.audio.position = 128;
        flush.returned.points = 1;
        flush.returned.point[0] = crate::process_results::Point {
            offset: 0,
            id: 42,
            value: 0.25,
        };
        assert!(s.results.push(flush));
        cb.returned.window(cb.position, 128);
        cb.process(&s, request, &mut out).unwrap();
        assert_eq!(cb.delivery.expired_frames, 128);
        assert_eq!(s.fault.load(Ordering::Acquire), 0);
        let mut packet = crate::process_results::Packet::default();
        cb.returned.take(&mut packet);
        assert_eq!((packet.events, packet.points), (1, 1));
        assert_eq!(
            (
                packet.event[0].kind,
                packet.event[0].a,
                packet.event[0].offset
            ),
            (1, -77, 0)
        );
        assert_eq!(packet.point[0].offset, 0);
        assert_eq!(cb.returned.stats().late_events, 1);
        cb.returned.take(&mut packet);
        assert_eq!((packet.events, packet.points), (0, 0));
    }
    #[test]
    fn stop_restart_discards_old_epoch_and_resets_delay() {
        let s = Shared::new();
        let mut cb = Callback::new();
        cb.transition(&s, START);
        let mut r = Item::control(AUDIO, 0);
        r.n = 256;
        r.gain = 0.5;
        r.data = [[0.25; CAP]; 2];
        let mut out = [[0.; CAP]; 2];
        for _ in 0..6 {
            cb.process(&s, r, &mut out).unwrap();
            pump(&s);
        }
        cb.transition(&s, STOP);
        assert_eq!(cb.process(&s, r, &mut out), Err(1));
        cb.transition(&s, START);
        pump(&s);
        // Late old producer result is discarded; it cannot enter epoch two.
        let mut old = r;
        old.epoch = 1;
        old.position = 999;
        assert!(s.results.push(old.into()));
        for _ in 0..4 {
            cb.process(&s, r, &mut out).unwrap();
            assert_eq!(out, [[0.; CAP]; 2]);
            pump(&s);
        }
        cb.process(&s, r, &mut out).unwrap();
        assert_eq!(out, [[0.125; CAP]; 2]);
        assert_eq!(cb.epoch, 2);
    }
    #[test]
    fn descriptor_overflow_and_wrong_position_fail() {
        let s = Shared::new();
        let mut cb = Callback::new();
        for _ in 0..DESCRIPTORS / 2 {
            assert_eq!(cb.transition(&s, START), 0);
            assert_eq!(cb.transition(&s, STOP), 0);
        }
        assert_eq!(cb.transition(&s, START), 2);
        assert_eq!(s.fault.load(Ordering::Acquire), OVERFLOW);
        let s = Shared::new();
        let mut cb = Callback::new();
        cb.transition(&s, START);
        let mut r = Item::control(AUDIO, 0);
        r.n = 256;
        r.gain = 0.5;
        let mut out = [[0.; CAP]; 2];
        for _ in 0..4 {
            cb.process(&s, r, &mut out).unwrap();
        }
        r.epoch = 1;
        r.position = 1;
        assert!(s.results.push(r.into()));
        assert_eq!(cb.process(&s, r, &mut out), Err(2));
        assert_eq!(s.fault.load(Ordering::Acquire), CORRELATION);
    }
    #[test]
    fn invalid_ffi_handle_refuses_without_work() {
        unsafe {
            assert_eq!(ap3_transition(0, START), 1);
            assert_eq!(ap3_close(0), 1);
            assert_eq!(ap3_stats(0, std::ptr::null_mut()), 1);
        }
    }
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod capture_tests;

//! Callback operations contain bounded copies, scalar checks, atomics and
//! monotonic timestamp reads and a declared, bounded completion wait when needed.
//! No allocation, logging, control wait or transport I/O.
//! All mapping/socket/session work and error formatting belong to the worker.
use crate::{binding, queue::Queue, retain, state, Session};
use ap1_native_client::{
    events::{Event, MAX_EVENTS},
    invalid, need, BLOCK_CAP as CAP, CAP as LEGACY_CAP, ERROR,
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
// The mapping-3 block is four times the old transport extent. Keep the
// admitted sample capacity bounded at the prior 524,288-frame ceiling.
pub const DESCRIPTORS: usize = 512;
const START: u32 = 10;
const STOP: u32 = 12;
const DEACTIVATE: u32 = 14;
const CLOSE: u32 = 5;
const AUDIO: u32 = 3;
// Fault code 1 was the retired terminal-underrun policy. Never reuse it.
const OVERFLOW: u64 = 2;
const WORKER: u64 = 3;
const CORRELATION: u64 = 4;
// 5 is retained for malformed returned process results.
const COMPLETION_DEADLINE: u64 = 6;
const COMPLETION_CANCELLED: u32 = 0x109;
const COMPLETION_EXPIRED: u32 = 0x108;
#[derive(Clone, Copy)]
pub struct Item {
    pub gui_revision: u64,
    pub kind: u32,
    pub n: u32,
    pub epoch: u64,
    pub position: u64,
    pub ticket: u64,
    pub process_mode: u32,
    pub(crate) completion: Option<crate::performance::CompletionPolicy>,
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
            ticket: 0,
            process_mode: 0,
            completion: None,
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
    extra_slot: usize,
    data: [[f32; CAP]; 2],
}
#[derive(Clone, Copy)]
struct Completion {
    audio: AudioResult,
    epoch: u64,
    ticket: u64,
    returned: crate::process_results::Packet,
    windows_process_ns: Option<u64>,
    windows_sequence: u64,
}
impl From<Item> for Completion {
    fn from(i: Item) -> Self {
        Self {
            audio: AudioResult {
                n: i.n,
                position: i.position,
                flags: i.flags,
                extra_slot: crate::output_pool::NONE,
                data: i.data,
            },
            epoch: i.epoch,
            ticket: i.ticket,
            returned: crate::process_results::Packet::default(),
            windows_process_ns: None,
            windows_sequence: 0,
        }
    }
}
#[derive(Clone, Copy, Default)]
struct DeadlineIdentity {
    generation: u64,
    epoch: u64,
    position: u64,
    host_call: u64,
    operation_ticket: u64,
    operation_submitted: bool,
}
const WORKER_BINDING_WORDS: usize = 6;
#[derive(Clone, Copy)]
struct WorkerBindingObservation {
    stability: crate::fault_status::Stability,
    attempts: u32,
    publication_before: u64,
    publication_after: u64,
    words: [u64; WORKER_BINDING_WORDS],
}
impl WorkerBindingObservation {
    const fn absent() -> Self {
        Self {
            stability: crate::fault_status::Stability::Absent,
            attempts: 0,
            publication_before: 0,
            publication_after: 0,
            words: [0; WORKER_BINDING_WORDS],
        }
    }
}
struct WorkerBinding {
    publication: AtomicU64,
    slots: [[AtomicU64; WORKER_BINDING_WORDS]; 2],
}
impl WorkerBinding {
    fn new() -> Self {
        Self {
            publication: AtomicU64::new(0),
            slots: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU64::new(0))),
        }
    }
    fn publish(&self, words: [u64; WORKER_BINDING_WORDS]) {
        let Some(next) = self.publication.load(Ordering::Relaxed).checked_add(1) else {
            return;
        };
        for (destination, value) in self.slots[next as usize & 1].iter().zip(words) {
            destination.store(value, Ordering::SeqCst);
        }
        self.publication.store(next, Ordering::SeqCst);
    }
    fn read(&self) -> WorkerBindingObservation {
        let mut observed = WorkerBindingObservation::absent();
        for attempt in 1..=3 {
            let before = self.publication.load(Ordering::SeqCst);
            if before == 0 {
                let after = self.publication.load(Ordering::SeqCst);
                observed.attempts = attempt;
                observed.publication_before = before;
                observed.publication_after = after;
                if after == 0 {
                    return observed;
                }
                continue;
            }
            let words = std::array::from_fn(|index| {
                self.slots[before as usize & 1][index].load(Ordering::SeqCst)
            });
            let after = self.publication.load(Ordering::SeqCst);
            observed.attempts = attempt;
            observed.publication_before = before;
            observed.publication_after = after;
            if before == after {
                observed.stability = crate::fault_status::Stability::Stable;
                observed.words = words;
                return observed;
            }
        }
        observed.stability = crate::fault_status::Stability::Unstable;
        observed
    }
}
#[derive(Clone, Copy)]
struct RefusalSnapshot {
    identity: DeadlineIdentity,
    worker_binding: [WorkerBindingObservation; 2],
    status: crate::fault_status::Snapshot,
}
struct FirstRefusal {
    ready: AtomicBool,
    exported: AtomicBool,
    value: UnsafeCell<Option<RefusalSnapshot>>,
}
// Exactly one first-fault CAS winner writes the fixed snapshot. Registry
// removal waits for all callback leases before the close owner reads it.
unsafe impl Sync for FirstRefusal {}
impl FirstRefusal {
    fn new() -> Self {
        Self {
            ready: AtomicBool::new(false),
            exported: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        }
    }
    fn capture(
        &self,
        reader: &crate::fault_status::Reader,
        binding: &WorkerBinding,
        identity: DeadlineIdentity,
    ) {
        let before = binding.read();
        let status = reader.snapshot();
        let after = binding.read();
        unsafe {
            *self.value.get() = Some(RefusalSnapshot {
                identity,
                worker_binding: [before, after],
                status,
            });
        }
        self.ready.store(true, Ordering::Release);
    }
    #[cfg(test)]
    fn read(&self) -> Option<RefusalSnapshot> {
        self.ready
            .load(Ordering::Acquire)
            .then(|| unsafe { *self.value.get() })
            .flatten()
    }
    fn take_for_export(&self) -> Option<RefusalSnapshot> {
        if !self.ready.load(Ordering::Acquire)
            || self.exported.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err() {
            return None;
        }
        unsafe { *self.value.get() }
    }
}
#[cfg(target_os="linux")]
#[derive(Default)]
struct DirectProgress {
    revision:AtomicU64,
    words:[AtomicU64;4], // epoch, AUDIO ticket, request position, completed end
}
#[cfg(target_os="linux")]
impl DirectProgress {
    // The guarded callback is the sole writer. Off-path readers never wait
    // for it; an interrupted publication is explicitly unknown.
    fn publish(&self,item:&Item,completed:bool) {
        self.revision.fetch_add(1,Ordering::SeqCst);
        if self.words[0].load(Ordering::SeqCst)!=item.epoch {self.words[3].store(0,Ordering::SeqCst);}
        self.words[0].store(item.epoch,Ordering::SeqCst);
        self.words[1].store(item.ticket,Ordering::SeqCst);
        self.words[2].store(item.position,Ordering::SeqCst);
        if completed {self.words[3].store(item.position+u64::from(item.n),Ordering::SeqCst);}
        self.revision.fetch_add(1,Ordering::SeqCst);
    }
    fn read(&self)->Option<[u64;4]> {
        for _ in 0..3 {
            let before=self.revision.load(Ordering::SeqCst);
            if before==0 || before&1!=0 {continue;}
            let words=std::array::from_fn(|i|self.words[i].load(Ordering::SeqCst));
            if self.revision.load(Ordering::SeqCst)==before {return Some(words);}
        }
        None
    }
}
struct Shared {
    #[cfg(target_os="linux")]
    direct: std::sync::OnceLock<crate::direct_audio::Endpoint>,
    #[cfg(target_os="linux")]
    direct_submitted:AtomicU64,
    #[cfg(target_os="linux")]
    direct_completed:AtomicU64,
    #[cfg(target_os="linux")]
    direct_progress:DirectProgress,
    #[cfg(target_os="linux")]
    prepared_audio: std::sync::Mutex<Option<Box<crate::direct_audio_session::AudioSession>>>,
    terminal: Option<Arc<crate::terminal::Status>>,
    // Set only after a complete, session/generation-bound terminal record.
    // The callback reads one atomic; it never reads the mapped custody record.
    terminal_latched: AtomicBool,
    gui: Option<Arc<crate::gui::Gui>>,
    snapshots: Arc<std::sync::Mutex<crate::recovery::Store>>,
    generation: u64,
    identity: Option<state::Identity>,
    execution: Option<ap1_native_client::admission::ExecutionIdentity>,
    notices: AtomicU64,
    notice_traits: AtomicU64,
    last_edit: AtomicU64,
    retired: AtomicBool,
    requests: Queue<Item>,
    results: Queue<Completion>,
    completion: crate::completion_wait::Signal,
    // The non-RT control caller owns this wait. The mailbox result remains the
    // predicate; the sequence only prevents a publication/check race from
    // losing the wake and is isolated from audio completion traffic.
    control_acknowledgement: crate::completion_wait::Signal,
    work: crate::completion_wait::WorkSignal,
    extra: std::sync::OnceLock<crate::output_pool::Pool>,
    wanted: AtomicU64,
    fault: AtomicU64,
    ack: AtomicU64,
    quit: AtomicBool,
    cancelled: AtomicBool,
    processed: AtomicU64,
    #[cfg(feature = "rpi0")]
    processed_frames: AtomicU64,
    first_position: AtomicU64,
    detail: std::sync::Mutex<String>,
    // Separate owner-thread mailbox. It never writes the SPSC audio queue.
    control: std::sync::Mutex<Option<Control>>,
    pending_control: AtomicBool,
    #[cfg(test)]
    control_waits: AtomicU64,
    #[cfg(test)]
    phase_waits: AtomicU64,
    // Test-only scheduling boundary after Session validated an owned reply,
    // before the worker publishes it. No production branch or storage exists.
    #[cfg(test)]
    publication_hold_ticket: AtomicU64,
    #[cfg(test)]
    publication_held_ticket: AtomicU64,
    #[cfg(test)]
    final_publication_hold_ticket: AtomicU64,
    #[cfg(test)]
    final_publication_held_ticket: AtomicU64,
    #[cfg(test)]
    pending_start_wakes: AtomicU64,
    #[cfg(test)]
    pending_start_control_fences: AtomicU64,
    state_capable: AtomicBool,
    curve_state_revision: AtomicU64,
    observer: Option<Arc<crate::observer::Shared>>,
    phase_diagnostics: bool,
    fault_reader: Option<crate::fault_status::Reader>,
    worker_binding: WorkerBinding,
    first_refusal: FirstRefusal,
    worker_op: AtomicU64,
    worker_epoch: AtomicU64,
    worker_position: AtomicU64,
    worker_thread: AtomicU64,
    // Distinct DAW threads seen calling process(); the worker requests a
    // real-time policy for each through the supervisor, off the callback.
    callers: [AtomicU64; crate::scheduling::CALLER_SLOTS],
    service_us_max: AtomicU64,
    // Callback writes counters only; the transport publishes them through the
    // existing independent status lane. No callback mapping or diagnostic I/O.
    delivery_totals: [AtomicU64; 6],
    // The callback classifies the entire host block once, against a successful
    // Windows setProcessing acknowledgement for that exact epoch. Counters
    // remain separate from terminal/queue snapshots and never perform I/O.
    processing_ready_epoch: AtomicU64,
    delivery_phases: [[AtomicU64; 6]; 2],
    first_context_ready: AtomicBool,
    first_epoch: AtomicU64,
    first_worker_op: AtomicU64,
    first_worker_epoch: AtomicU64,
    first_worker_position: AtomicU64,
    first_requests: [AtomicU64; 2],
    first_results: [AtomicU64; 2],
}
impl Shared {
    fn publish_result(&self, value: Completion) -> Option<Instant> {
        if !self.results.push(value) { return None; }
        let published = Instant::now();
        self.completion.notify();
        Some(published)
    }
    fn try_new() -> io::Result<Self> {
        Ok(Self {
            #[cfg(target_os="linux")]
            direct: std::sync::OnceLock::new(),
            #[cfg(target_os="linux")]
            direct_submitted:AtomicU64::new(0),
            #[cfg(target_os="linux")]
            direct_completed:AtomicU64::new(0),
            #[cfg(target_os="linux")]
            direct_progress:DirectProgress::default(),
            #[cfg(target_os="linux")]
            prepared_audio: std::sync::Mutex::new(None),
            gui: None,
            terminal: None,
            terminal_latched: AtomicBool::new(false),
            snapshots: Arc::new(std::sync::Mutex::new(crate::recovery::Store::default())),
            generation: 1,
            identity: None,
            execution: None,
            notices: AtomicU64::new(0),
            notice_traits: AtomicU64::new(0),
            last_edit: AtomicU64::new(0),
            retired: AtomicBool::new(false),
            requests: Queue::new(DESCRIPTORS),
            results: Queue::new(DESCRIPTORS),
            completion: crate::completion_wait::Signal::new(),
            control_acknowledgement: crate::completion_wait::Signal::new(),
            work: crate::completion_wait::WorkSignal::new()?,
            extra: std::sync::OnceLock::new(),
            wanted: AtomicU64::new(0),
            fault: AtomicU64::new(0),
            ack: AtomicU64::new(9),
            quit: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            processed: AtomicU64::new(0),
            #[cfg(feature = "rpi0")]
            processed_frames: AtomicU64::new(0),
            first_position: AtomicU64::new(u64::MAX),
            detail: std::sync::Mutex::new(String::new()),
            control: std::sync::Mutex::new(None),
            pending_control: AtomicBool::new(false),
            #[cfg(test)]
            control_waits: AtomicU64::new(0),
            #[cfg(test)]
            phase_waits: AtomicU64::new(0),
            #[cfg(test)]
            publication_hold_ticket: AtomicU64::new(0),
            #[cfg(test)]
            publication_held_ticket: AtomicU64::new(0),
            #[cfg(test)]
            final_publication_hold_ticket: AtomicU64::new(0),
            #[cfg(test)]
            final_publication_held_ticket: AtomicU64::new(0),
            #[cfg(test)]
            pending_start_wakes: AtomicU64::new(0),
            #[cfg(test)]
            pending_start_control_fences: AtomicU64::new(0),
            state_capable: AtomicBool::new(false),
            curve_state_revision: AtomicU64::new(0),
            observer: None,
            phase_diagnostics: false,
            fault_reader: None,
            worker_binding: WorkerBinding::new(),
            first_refusal: FirstRefusal::new(),
            worker_op: AtomicU64::new(0),
            worker_epoch: AtomicU64::new(0),
            worker_position: AtomicU64::new(0),
            worker_thread: AtomicU64::new(0),
            callers: std::array::from_fn(|_| AtomicU64::new(0)),
            service_us_max: AtomicU64::new(0),
            delivery_totals: std::array::from_fn(|_| AtomicU64::new(0)),
            processing_ready_epoch: AtomicU64::new(0),
            delivery_phases: std::array::from_fn(|_| std::array::from_fn(|_| AtomicU64::new(0))),
            first_context_ready: AtomicBool::new(false),
            first_epoch: AtomicU64::new(0),
            first_worker_op: AtomicU64::new(0),
            first_worker_epoch: AtomicU64::new(0),
            first_worker_position: AtomicU64::new(0),
            first_requests: std::array::from_fn(|_| AtomicU64::new(0)),
            first_results: std::array::from_fn(|_| AtomicU64::new(0)),
        })
    }
    #[cfg(test)]
    fn new() -> Self { Self::try_new().expect("prepared worker wake") }
    fn prepare_outputs(&self, bytes: &[u8]) -> io::Result<()> {
        let channels=crate::performance::output_channels(bytes)? - 2;
        if let Some(pool)=self.extra.get() {
            ap1_native_client::need(pool.channels==channels,"output layout changed")?;
        } else if channels>0 {
            self.extra.set(crate::output_pool::Pool::new(channels,DESCRIPTORS + 1))
                .map_err(|_| invalid("output pool already prepared"))?;
        }
        Ok(())
    }
    fn release_output(&self, audio: &AudioResult) {
        if let Some(pool)=self.extra.get() {pool.release(audio.extra_slot);}
    }
    fn audio_progress(&self,session:&Session)->([u64;5],Option<u64>) {
        #[cfg(target_os="linux")]
        if self.direct.get().is_some() {
            return self.direct_progress.read().map_or(
                ([self.generation,session.epoch,u64::MAX,u64::MAX,u64::from(session.phase)],None),
                |[epoch,ticket,position,end]|([self.generation,epoch,ticket,position,u64::from(session.phase)],Some(end)));
        }
        ([self.generation,session.epoch,session.state.next,session.position,u64::from(session.phase)],Some(session.position))
    }
    fn terminal_record(&self) -> Option<crate::terminal::Record> {
        let record = self.terminal.as_ref()?.read()?;
        if record.words[3] != self.generation || self.generation == 0 { return None; }
        self.terminal_latched.store(true, Ordering::Release);
        Some(record)
    }
    fn begin_failure(&self, code: u64, position: u64) -> bool {
        let first = self
            .fault
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        if first {
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
        first
    }
    fn notify_failure(&self) {
        #[cfg(target_os="linux")]
        if let Some(channel)=self.direct.get() {channel.cancel(3);}
        self.completion.notify();
        self.control_acknowledgement.notify();
        self.work.notify();
    }
    fn fail(&self, code: u64, position: u64) {
        self.begin_failure(code, position);
        self.notify_failure();
    }
    fn fail_deadline(&self, position: u64, identity: DeadlineIdentity) {
        if self.begin_failure(COMPLETION_DEADLINE, position) {
            if let Some(reader) = &self.fault_reader {
                self.first_refusal.capture(reader, &self.worker_binding, identity);
            }
        }
        self.notify_failure();
    }
    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        #[cfg(target_os="linux")]
        if let Some(channel)=self.direct.get() {channel.cancel(1);}
        self.completion.notify();
        self.control_acknowledgement.notify();
        self.work.notify();
    }
}
// A direct completion is local until handed to the existing consumer. Any
// intervening cancellation/fault return must release its prepared extra slot.
struct DirectOutputGuard<'a> {shared:&'a Shared,slot:usize}
impl Drop for DirectOutputGuard<'_> {
    fn drop(&mut self) {if let Some(pool)=self.shared.extra.get(){pool.release(self.slot);}}
}
struct Control {
    barrier: u64,
    #[cfg(target_os="linux")]
    direct_barrier:Option<u64>,
    op: u32,
    bytes: Vec<u8>,
    result: Option<io::Result<Vec<u8>>>,
}
fn admitted_control_precedes(s: &Shared, consumed_includes_candidate: bool) -> io::Result<bool> {
    if !s.pending_control.load(Ordering::Acquire) { return Ok(false); }
    let mailbox = s.control.lock().map_err(|_| invalid("state mailbox poisoned"))?;
    let consumed = s.requests.consumed();
    Ok(mailbox.as_ref().is_some_and(|control| {
        control.result.is_none()
            && if consumed_includes_candidate {
                control.barrier < consumed
            } else {
                control.barrier <= consumed
            }
    }))
}
fn pending_start_cancelled(s: &Shared) -> bool {
    s.cancelled.load(Ordering::Acquire)
        || s.quit.load(Ordering::Acquire)
        || s.fault.load(Ordering::Acquire) != 0
}
fn pending_start_wait_after_snapshot(s: &Shared, observed: u32) -> io::Result<u32> {
    ap1_native_client::need(!pending_start_cancelled(s), "pending Start cancelled")?;
    Ok(observed)
}
fn audio_transport_deadline(item: &Item, now: Instant) -> Instant {
    item.completion.map_or_else(|| now + crate::performance::AUDIO_CONTAINMENT,
        |policy| policy.deadline)
}
// Both normal owner service and an admitted read-only capture finish here.
// This is transport-worker code, never a DAW callback. Capture completion can
// therefore acknowledge its owner while a slower offline AUDIO is still pending.
fn complete_control(s: &Shared, c: &mut Control, result: io::Result<Vec<u8>>,
    progress: ([u64; 5],Option<u64>)) -> io::Result<()> {
    let failed = result.as_ref().is_err_and(|e| !state::save_refused(e));
    if let Ok(bytes) = &result {
        if matches!(c.op, 16 | 18) {
            let mut store = s.snapshots.lock().map_err(|_| invalid("snapshot store poisoned"))?;
            let mut through=c.barrier;
            #[cfg(target_os="linux")]
            if let Some(direct)=c.direct_barrier {through=direct;}
            store.confirm(bytes.clone(), c.op, s.generation, through)?;
            if let Some(t) = &s.terminal { t.progress(progress.0, progress.1, store.latest()); }
        }
    } else if let Err(error) = &result {
        if !state::save_refused(error) {
            if let Ok(mut detail) = s.detail.lock() {
                if detail.is_empty() { *detail = bounded_detail(error); }
            }
        }
    }
    c.result = Some(result);
    s.pending_control.store(false, Ordering::Release);
    // Publish only after the authoritative result and pending predicate. The
    // waiter snapshots before inspecting both, so an acknowledgement racing
    // with wait entry cannot be lost.
    s.control_acknowledgement.notify();
    if failed { Err(invalid("component state/control failed; original detail retained in response")) }
    else { Ok(()) }
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
// Keep a small failure witness even with the optional sample/SDK observer off.
// Only the guarded callback writes these records; close reads them after all
// instance leases have retired. Progress fields are independent observations,
// not an atomic snapshot or proof of why the peer missed this deadline.
#[derive(Clone, Copy, Default)]
struct PresentationGap {
    clock: [u64; 2],
    generation: u64,
    epoch: u64,
    position: u64,
    frames: u64,
    parent: [u64; 4],
    worker_thread: u64,
    worker: [u64; 3],
    requests: [u64; 2],
    results: [u64; 2],
    control_pending: bool,
    ready_epoch: u64,
}
#[derive(Default)]
struct PresentationGaps {
    records: [PresentationGap; 16],
    length: usize,
    omitted: u64,
}
impl PresentationGaps {
    fn record(&mut self, s: &Shared, epoch: u64, position: u64, frames: u64, parent: [u64; 4]) {
        if self.length == self.records.len() {
            self.omitted = self.omitted.saturating_add(1);
            return;
        }
        let mut r = PresentationGap {
            clock: [crate::observer::monotonic_ns(), 0],
            generation: s.generation, epoch, position, frames, parent,
            worker_thread: s.worker_thread.load(Ordering::Acquire),
            worker: [s.worker_op.load(Ordering::Acquire), s.worker_epoch.load(Ordering::Acquire),
                s.worker_position.load(Ordering::Acquire)],
            requests: [s.requests.published(), s.requests.consumed()],
            results: [s.results.published(), s.results.consumed()],
            control_pending: s.pending_control.load(Ordering::Acquire),
            ready_epoch: s.processing_ready_epoch.load(Ordering::Acquire),
        };
        r.clock[1] = crate::observer::monotonic_ns();
        self.records[self.length] = r;
        self.length += 1;
    }
    fn report(&self, path: &std::path::Path) {
        for r in &self.records[..self.length] {
            let text = format!(concat!("{{\"event\":\"audio_presentation_gap\",\"schema\":1,",
                "\"clock_monotonic_ns\":[{},{}],\"generation\":{},\"epoch\":{},",
                "\"position\":{},\"frames\":{},\"parent_callback\":[{},{},{},{}],",
                "\"worker_thread\":{},\"worker\":[{},{},{}],\"requests\":[{},{}],",
                "\"results\":[{},{}],\"control_pending\":{},\"ready_epoch\":{},",
                "\"progress_is_atomic_snapshot\":false}}\n"),
                r.clock[0], r.clock[1], r.generation, r.epoch, r.position, r.frames,
                r.parent[0], r.parent[1], r.parent[2], r.parent[3], r.worker_thread,
                r.worker[0], r.worker[1], r.worker[2], r.requests[0], r.requests[1],
                r.results[0], r.results[1], r.control_pending, r.ready_epoch);
            crate::preview::append_report(path, text.as_bytes());
        }
        crate::preview::append_report(path, format!(
            "{{\"event\":\"audio_presentation_gap_summary\",\"retained_spans\":{},\"omitted_spans\":{}}}\n",
            self.length, self.omitted).as_bytes());
    }
}
struct Callback {
    #[cfg(target_os="linux")]
    direct_audio: Option<Box<crate::direct_audio_session::AudioSession>>,
    windows_timing: WindowsProcessTiming,
    // One originating call policy remains owned through final SDK sink work.
    // Each queued Item owns its copy after Buffered presentation returns.
    completion_policy: Option<crate::performance::CompletionPolicy>,
    completion_waits: u64,
    completion_wait_misses: u64,
    // Longest a realtime callback waits for the Windows side to acknowledge a
    // start before it answers with silence. None keeps the full wait.
    unready_bound: Option<Duration>,
    unready_callbacks: u64,
    unready_frames: u64,
    callback_ns_max: u64,
    callback_us_buckets: [u64; 10],
    curve_plan: crate::parameter_curves::Plan,
    curve_carry: crate::parameter_curves::Carry,
    curve_values: crate::parameter_curves::Values,
    curve_continuation: Option<crate::context::Context>,
    curve_gui_revision: u64,
    curve_state_revision: u64,
    host_call: u64,
    submitted_operation: u64,
    completed_operation: u64,
    last_operation_position: u64,
    last_operation_host_call: u64,
    last_operation_ticket: u64,
    delay: u64,
    epoch: u64,
    position: u64,
    running: bool,
    audio: crate::output_pool::Retained,
    returned: crate::process_results::Pending,
    next_result: u64,
    in_gap: bool,
    delivery: Delivery,
    gaps: PresentationGaps,
}
impl Callback {
    fn new() -> Self {
        Self {
            #[cfg(target_os="linux")]
            direct_audio: None,
            windows_timing: WindowsProcessTiming::default(),
            completion_policy: None,
            completion_waits: 0,
            unready_bound: None,
            unready_callbacks: 0,
            unready_frames: 0,
            completion_wait_misses: 0,
            callback_ns_max: 0,
            callback_us_buckets: [0; 10],
            curve_plan: crate::parameter_curves::Plan::empty(),
            curve_carry: crate::parameter_curves::Carry::empty(),
            curve_values: crate::parameter_curves::Values::empty(),
            curve_continuation: None,
            curve_gui_revision: 0,
            curve_state_revision: 0,
            host_call: 0,
            submitted_operation: 0,
            completed_operation: 0,
            last_operation_position: 0,
            last_operation_host_call: 0,
            last_operation_ticket: 0,
            delay: DELAY,
            epoch: 0,
            position: 0,
            running: false,
            audio: crate::output_pool::Retained::new(2, CAP, DELAY as usize),
            returned: crate::process_results::Pending::with_capacity(DELAY as usize + DESCRIPTORS + 1),
            next_result: 0,
            in_gap: false,
            delivery: Delivery::default(),
            gaps: PresentationGaps::default(),
        }
    }
    fn prepare(&mut self, maximum: usize, channels: usize, delay: usize) {
        // Inactive owner only. One positive operation can occur per retained
        // frame (N=1), plus bounded queued completions and the current operation.
        // D=0 exact delivery has no delayed or queued operation backlog.
        self.audio = crate::output_pool::Retained::new(channels, maximum, delay);
        self.returned = crate::process_results::Pending::with_capacity(
            if delay == 0 { 1 } else { delay + DESCRIPTORS + 1 });
        self.delay = delay as u64;
    }
    fn deadline_identity(&self, s: &Shared, submitted: bool) -> DeadlineIdentity {
        DeadlineIdentity {
            generation: s.generation,
            epoch: self.epoch,
            position: if submitted { self.last_operation_position } else { self.position },
            host_call: if submitted { self.last_operation_host_call } else { self.host_call },
            operation_ticket: if submitted { self.last_operation_ticket } else { 0 },
            operation_submitted: submitted,
        }
    }
    fn clear_audio(&mut self, s: &Shared) {
        self.curve_carry = crate::parameter_curves::Carry::empty();
        self.curve_values.invalidate();
        self.curve_continuation = None;
        self.audio.clear();
        for _ in 0..DESCRIPTORS {
            let Some(c)=s.results.pop() else {break;};
            s.release_output(&c.audio);
        }
    }
    // Recovery replaces the transport, not the exact SDK parameter census.
    // Move its preconfigured storage and discard values from the failed peer.
    fn replacement(&mut self) -> Self {
        let mut next = Self::new();
        next.prepare(self.audio.maximum, self.audio.channels, self.delay as usize);
        next.gaps = std::mem::take(&mut self.gaps);
        next.completion_waits = std::mem::take(&mut self.completion_waits);
        next.unready_bound = self.unready_bound;
        next.unready_callbacks = std::mem::take(&mut self.unready_callbacks);
        next.unready_frames = std::mem::take(&mut self.unready_frames);
        next.completion_wait_misses = std::mem::take(&mut self.completion_wait_misses);
        next.callback_ns_max = std::mem::take(&mut self.callback_ns_max);
        next.callback_us_buckets = std::mem::take(&mut self.callback_us_buckets);
        next.curve_values = std::mem::replace(
            &mut self.curve_values, crate::parameter_curves::Values::empty());
        next.curve_values.invalidate();
        next
    }
    fn transition(&mut self, s: &Shared, op: u32) -> u32 {
        self.completion_policy = None;
        if s.fault.load(Ordering::Acquire) != 0 {
            return 2;
        }
        match op {
            START if !self.running && self.epoch < u64::MAX => {
                self.epoch += 1;
                self.position = 0;
                self.clear_audio(s);
                self.next_result = 0;
                self.submitted_operation = 0;
                self.completed_operation = 0;
                self.in_gap = false;

                self.returned.reset();
                s.wanted.store(self.epoch, Ordering::Release);
                self.running = true;
            }
            STOP if self.running => {
                self.running = false;
                self.clear_audio(s);
                self.returned.reset();
                s.wanted.store(0, Ordering::Release);
            }
            _ => return 1,
        }
        if !s.requests.push(Item::control(op, self.epoch)) {
            s.fail(OVERFLOW, self.position);
            return 2;
        }
        s.work.notify();
        0
    }
    #[cfg(test)]
    fn process(
        &mut self,
        s: &Shared,
        request: Item,
        out: &mut [[f32; CAP]; 2],
    ) -> Result<u64, u32> { self.process_outputs(s,request,out,&[],0) }
    #[cfg(test)]
    fn process_outputs(&mut self, s: &Shared, request: Item,
        out: &mut [[f32; CAP]; 2], extra: &[*mut f32], destination: usize,
    ) -> Result<u64,u32> {
        self.process_outputs_until(s, request, out, extra, destination, None)
    }
    #[cfg(test)]
    fn process_outputs_until(&mut self, s: &Shared, request: Item,
        out: &mut [[f32; CAP]; 2], extra: &[*mut f32], destination: usize,
        deadline: Option<Instant>,
    ) -> Result<u64,u32> {
        self.process_outputs_until_traced(s, request, out, extra, destination, deadline, None)
    }
    // Keep the callback's already-bounded audio, deadline, and optional trace
    // borrows explicit at this real-time boundary.
    #[allow(clippy::too_many_arguments)]
    fn process_outputs_until_traced(&mut self, s: &Shared, mut request: Item,
        out: &mut [[f32; CAP]; 2], extra: &[*mut f32], destination: usize,
        deadline: Option<Instant>, mut trace: Option<&mut PhaseTrace>,
    ) -> Result<u64,u32> {
        if !self.running || request.n as usize > CAP {
            return Err(1);
        }
        if s.fault.load(Ordering::Acquire) != 0 || s.terminal_latched.load(Ordering::Acquire) {
            return Err(2);
        }
        if s.cancelled.load(Ordering::Acquire) || s.quit.load(Ordering::Acquire) {
            return Err(COMPLETION_CANCELLED);
        }
        let exact = request.completion.is_some_and(|policy| policy.exact);
        if request.completion.is_some() && deadline.is_some_and(|end| Instant::now() >= end) {
            s.fail_deadline(self.position, self.deadline_identity(s, false));
            return Err(COMPLETION_EXPIRED);
        }
        for &p in extra { if !p.is_null() { unsafe {
            std::ptr::write_bytes(p.add(destination),0,request.n as usize);
        } } }
        // The Windows side acknowledges a start from its owner thread, which
        // may still be opening an editor. That gets a short stated bound.
        // Past it the plug-in is not processing yet: this block is silence and
        // is not part of its stream. Nothing is queued, its events are not
        // delivered, and the DAW's thread does not wait for an editor.
        if let Some(bound) = self.unready_bound.filter(|_| request.process_mode != 2) {
            let mut until = Instant::now() + bound;
            if let Some(end) = deadline { until = until.min(end); }
            while s.processing_ready_epoch.load(Ordering::Acquire) != self.epoch {
                let observed = s.completion.snapshot();
                if s.processing_ready_epoch.load(Ordering::Acquire) == self.epoch { break; }
                if s.cancelled.load(Ordering::Acquire) || s.quit.load(Ordering::Acquire) {
                    return Err(COMPLETION_CANCELLED);
                }
                if s.fault.load(Ordering::Acquire) != 0 || s.terminal_latched.load(Ordering::Acquire) {
                    return Err(2);
                }
                if Instant::now() >= until {
                    let n = request.n as usize;
                    for plane in out.iter_mut() { plane[..n].fill(0.); }
                    self.unready_callbacks += 1;
                    self.unready_frames += n as u64;
                    self.delivery = Delivery::default();
                    return Ok(channel_mask(2 + extra.len()));
                }
                s.completion.wait(observed, until);
            }
        }
        request.epoch = self.epoch;
        request.position = self.position;
        if request.completion.is_some() {
            let Some(ticket) = self.submitted_operation.checked_add(1) else {
                s.fail(OVERFLOW, self.position); return Err(2);
            };
            request.ticket = ticket;
            self.submitted_operation = ticket;
            self.last_operation_position = request.position;
            self.last_operation_host_call = request.parent[0];
            self.last_operation_ticket = ticket;
        }
        request.queued = Some(Instant::now());
        if request.events[..request.event_count as usize]
            .iter()
            .any(|e| e.kind == 2)
        {
            request.gui_revision = s.gui.as_ref().map_or(0, |gui| gui.revision());
            self.curve_gui_revision = s.gui.as_ref().map_or(0, |_| request.gui_revision + 1);
        }
        let mut direct_result:Option<Completion>=None;
        let mut direct_slot=DirectOutputGuard {shared:s,slot:crate::output_pool::NONE};
        #[cfg(target_os="linux")]
        if let Some(audio)=self.direct_audio.as_mut() {
            if !exact || self.delay!=0 {s.fail(CORRELATION,self.position);return Err(2);}
            let until=deadline.unwrap_or_else(||Instant::now()+crate::performance::AUDIO_CONTAINMENT);
            // START acknowledgement is the control fence. Once ready, neither
            // AUDIO publication nor reply waits for the mutable Session worker.
            while s.processing_ready_epoch.load(Ordering::Acquire)!=self.epoch {
                let observed=s.completion.snapshot();
                if s.processing_ready_epoch.load(Ordering::Acquire)==self.epoch {break;}
                if s.cancelled.load(Ordering::Acquire)||s.quit.load(Ordering::Acquire) {
                    return Err(COMPLETION_CANCELLED);
                }
                if s.fault.load(Ordering::Acquire)!=0 {return Err(2);}
                if Instant::now()>=until {s.fail_deadline(self.position,self.deadline_identity(s,false));return Err(COMPLETION_EXPIRED);}
                s.completion.wait(observed,until);
            }
            let submitted=s.direct_submitted.fetch_add(1,Ordering::AcqRel);
            let Some(submitted)=submitted.checked_add(1) else {s.fail(OVERFLOW,self.position);return Err(2);};
            if !request.gain.is_nan() || request.event_count>0 {s.last_edit.store(submitted,Ordering::Release);}
            s.direct_progress.publish(&request,false);
            let done=match audio.process(&mut request,until) {
                Ok(done)=>done,
                Err(crate::direct_audio::Failure::Expired)=>{
                    s.fail_deadline(self.position,self.deadline_identity(s,true));return Err(COMPLETION_EXPIRED);
                }
                Err(crate::direct_audio::Failure::Cancelled)=>return Err(COMPLETION_CANCELLED),
                Err(_)=>{s.fail(CORRELATION,self.position);return Err(2);}
            };
            let mut result=Completion::from(request);result.audio.flags=done.flags;
            result.returned=done.returned;result.windows_sequence=done.sequence;
            result.windows_process_ns=Some(done.process_ns);
            if request.n>0 {if let Some(pool)=s.extra.get() {
                let Some(slot)=pool.publish_available(&audio.mapping.extra,request.n as usize) else {
                    s.fail(OVERFLOW,self.position);return Err(2);
                };result.audio.extra_slot=slot;direct_slot.slot=slot;
            }}
            s.notice_traits.store(done.traits,Ordering::Release);
            s.notices.fetch_or(done.notices as u64,Ordering::Release);
            s.processed.fetch_add(1,Ordering::Relaxed);
            #[cfg(feature="rpi0")]
            s.processed_frames.fetch_add(request.n as u64,Ordering::Relaxed);
            s.direct_completed.store(submitted,Ordering::Release);
            s.direct_progress.publish(&request,true);
            direct_result=Some(result);
        }
        if direct_result.is_none() {
            if !s.requests.push(request) {s.fail(OVERFLOW,self.position);return Err(2);}
            s.work.notify();
        }
        if direct_result.is_none() && (!request.gain.is_nan() || request.event_count > 0) {
            s.last_edit.store(s.requests.published(), Ordering::Release);
        }
        self.delivery = Delivery::default();
        let n = request.n as usize;
        let mut flags = channel_mask(2+extra.len());
        let required_end = self.position.saturating_add(n as u64).saturating_sub(self.delay);
        if let Some(t) = trace.as_deref_mut() {
            t.predicate_kind = u64::from(exact);
            t.predicate_required = if exact { request.ticket } else { required_end };
            t.predicate_before = if exact { self.completed_operation } else { self.next_result };
            t.predicate_initial_satisfied = u64::from(if exact {
                self.completed_operation >= request.ticket
            } else {
                n == 0 || self.next_result >= required_end
            });
            t.phase_reached |= PHASE_PREDICATE_BEFORE | PHASE_WAIT;
        }
        // Consume whole completions independently of audio presentation. This
        // admits zero-frame results and preserves late events before audio expiry.
        let presentation_start = self.position.saturating_sub(self.delay);
        self.delivery.expired_frames += self.audio.discard_before(presentation_start);
        for _ in 0..=DESCRIPTORS {
            let item = loop {
                let observed = s.completion.snapshot();
                if s.cancelled.load(Ordering::Acquire) || s.quit.load(Ordering::Acquire) {
                    return Err(COMPLETION_CANCELLED);
                }
                if s.fault.load(Ordering::Acquire) != 0 || s.terminal_latched.load(Ordering::Acquire) {
                    return Err(2);
                }
                if let Some(item)=direct_result.take() {direct_slot.slot=crate::output_pool::NONE;break Some(item);}
                if let Some(item)=s.results.pop() {break Some(item);}
                let Some(until) = deadline else { break None; };
                let satisfied = if exact { self.completed_operation >= request.ticket }
                    else { n == 0 || self.next_result >= required_end };
                if satisfied || (request.completion.is_none() && !exact && s.pending_control.load(Ordering::Acquire))
                    || s.fault.load(Ordering::Acquire) != 0
                    || s.terminal_latched.load(Ordering::Acquire)
                    || s.quit.load(Ordering::Acquire)
                { break None; }
                self.completion_waits += 1;
                #[cfg(test)]
                s.phase_waits.fetch_add(1, Ordering::Release);
                let interrupt = Instant::now() + crate::performance::INTERRUPT_INTERVAL;
                let wait_begin = trace.as_ref().map(|_| crate::observer::monotonic_ns());
                let signalled = s.completion.wait(observed, until.min(interrupt));
                if let (Some(t), Some(begin)) = (trace.as_deref_mut(), wait_begin) {
                    retain_wait_clock(t, begin, crate::observer::monotonic_ns());
                }
                if !signalled {
                    // A publication can precede timeout return while its wake
                    // races. Inspect once more before declaring missing audio.
                    let item = s.results.pop();
                    if item.is_some() { break item; }
                    if Instant::now() < until { continue; }
                    self.completion_wait_misses += 1;
                    break None;
                }
            };
            let Some(item) = item else { break; };
            if item.epoch < self.epoch {
                s.release_output(&item.audio);
                continue;
            }
            let a = item.audio;
            if item.epoch != self.epoch
                || a.n as usize > CAP
                || a.position != self.next_result
                || a.position.checked_add(a.n as u64).is_none()
            {
                s.release_output(&a);
                s.fail(CORRELATION, self.position);
                return Err(2);
            }
            if item.ticket != 0 {
                if self.completed_operation.checked_add(1) != Some(item.ticket) {
                    s.release_output(&a); s.fail(CORRELATION, self.position); return Err(2);
                }
                if exact && item.ticket == request.ticket {
                    self.windows_timing.add(item.epoch, item.windows_sequence, item.windows_process_ns);
                }
                self.completed_operation = item.ticket;
            } else if request.completion.is_some() {
                s.release_output(&a); s.fail(CORRELATION, self.position); return Err(2);
            }
            self.next_result += a.n as u64;
            if !self
                .returned
                .append(&item.returned, a.position, a.n as usize,
                    if a.n == 0 { 0 } else { self.delay })
            {
                s.release_output(&a);
                s.fail(OVERFLOW, self.position);
                return Err(2);
            }
            if a.n > 0 {
                let extra = s.extra.get().map(|pool| (pool, a.extra_slot));
                let retained = self.audio.append(a.position, a.n as usize, a.flags, &a.data,
                    extra, presentation_start);
                // The frame ring owns the copied samples now. The worker slot
                // can be reused even while those samples remain delayed by D.
                s.release_output(&a);
                let Some(expired) = retained else {
                    s.fail(OVERFLOW, self.position);
                    return Err(2);
                };
                self.delivery.expired_frames += expired;
            }
        }
        if let Some(t) = trace {
            t.predicate_after = if exact { self.completed_operation } else { self.next_result };
            t.predicate_final_satisfied = u64::from(if exact {
                self.completed_operation >= request.ticket
            } else {
                n == 0 || self.next_result >= required_end
            });
            t.phase_reached |= PHASE_PREDICATE_AFTER;
        }
        let unsatisfied = if exact { self.completed_operation < request.ticket }
            else { self.next_result < required_end };
        if request.completion.is_some() && (unsatisfied
            || deadline.is_some_and(|end| Instant::now() >= end)) {
            s.fail_deadline(self.position, self.deadline_identity(s, true));
            return Err(COMPLETION_EXPIRED);
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
            if self.audio.is_empty() {
                // Retain the gap observation and failure output. Modern
                // ordered completion cannot report successful missing spans;
                // legacy policy keeps its previously declared counted silence.
                let count = n - i;
                self.gaps.record(s, self.epoch, expected, count as u64, request.parent);
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
                if request.completion.is_some() {
                    s.fail(CORRELATION, position);
                    return Err(2);
                }
                break;
            }
            self.delivery.expired_frames += self.audio.discard_before(expected);
            if self.audio.is_empty() { continue; }
            if self.audio.position() != expected || self.audio.channels != extra.len() + 2 {
                s.fail(CORRELATION, position);
                return Err(2);
            }
            let (count, copied_flags) = self.audio.copy(n - i, out, extra, i, destination + i);
            flags &= copied_flags;
            self.delivery.delivered_frames += count as u64;
            self.in_gap = false;
            i += count;
        }
        self.position += request.n as u64;
        Ok(flags)
    }
}

const PHASE_TRACE_SCHEMA: u32 = 1;
// Optional diagnostics: exact own-ticket vendor elapsed time, already present
// in IPC15 replies. No additional timer or observer thread on the callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct WindowsProcessTiming {
    pub schema: u32,
    pub size: u32,
    pub epoch: u64,
    pub host_call: u64,
    pub requests: u64,
    pub process_ns: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub valid: u32,
    pub reserved: u32,
}
impl Default for WindowsProcessTiming {
    fn default() -> Self {
        Self {
            schema: 1,
            size: 64,
            epoch: 0,
            host_call: 0,
            requests: 0,
            process_ns: 0,
            first_sequence: 0,
            last_sequence: 0,
            valid: 0,
            reserved: 0,
        }
    }
}
impl WindowsProcessTiming {
    fn add(&mut self, epoch: u64, sequence: u64, ns: Option<u64>) {
        if self.requests == 0 {
            self.epoch = epoch;
            self.first_sequence = sequence;
            self.valid = 1;
        } else if self.epoch != epoch || sequence <= self.last_sequence {
            self.valid = 0;
        }
        self.requests += 1;
        self.last_sequence = sequence;
        match ns.and_then(|n| self.process_ns.checked_add(n)) {
            Some(total) if sequence != 0 => self.process_ns = total,
            _ => self.valid = 0,
        }
    }
}
const _: () = assert!(std::mem::size_of::<WindowsProcessTiming>() == 64);
const PHASE_RUST_ENTRY: u64 = 1 << 1;
const PHASE_POLICY: u64 = 1 << 2;
const PHASE_IDENTITY: u64 = 1 << 3;
const PHASE_PREDICATE_BEFORE: u64 = 1 << 4;
const PHASE_WAIT: u64 = 1 << 5;
const PHASE_PREDICATE_AFTER: u64 = 1 << 6;
const PHASE_PRESENTATION_DONE: u64 = 1 << 7;
const PHASE_RUST_PRE_RETURN: u64 = 1 << 8;
const PHASE_BACKEND_RESULT: u64 = 1 << 9;

#[inline]
fn retain_wait_clock(t: &mut PhaseTrace, begin: u64, end: u64) {
    let prior = t.wait_count;
    t.wait_count = t.wait_count.saturating_add(1);
    if begin == 0 || end == 0 || end < begin {
        t.clock_valid &= !PHASE_WAIT;
        return;
    }
    // Once any interval is unavailable, later intervals cannot make the
    // aggregate complete. Retain only the valid prefix and the whole count.
    if prior != 0 && t.clock_valid & PHASE_WAIT == 0 {
        return;
    }
    if prior == 0 {
        t.wait_begin_ns = begin;
    }
    t.wait_end_ns = end;
    t.wait_total_ns = t.wait_total_ns.saturating_add(end - begin);
    t.clock_valid |= PHASE_WAIT;
}

#[repr(C)]
#[derive(Default)]
pub struct PhaseTrace {
    pub schema: u32,
    pub size: u32,
    pub phase_reached: u64,
    pub clock_valid: u64,
    pub ordinal: u64,
    pub generation: u64,
    pub epoch: u64,
    pub host_call: u64,
    pub position: u64,
    pub frames: u64,
    pub mode: u64,
    pub delivery_mode: u64,
    pub exact: u64,
    pub allowance_ns: u64,
    pub wait_deadline_lower_ns: u64,
    pub wait_deadline_upper_ns: u64,
    pub predicate_kind: u64,
    pub predicate_required: u64,
    pub predicate_before: u64,
    pub predicate_after: u64,
    pub predicate_initial_satisfied: u64,
    pub predicate_final_satisfied: u64,
    pub cpp_entry_ns: u64,
    pub rust_entry_ns: u64,
    pub wait_begin_ns: u64,
    pub wait_end_ns: u64,
    pub wait_total_ns: u64,
    pub wait_count: u64,
    pub presentation_done_ns: u64,
    pub rust_pre_return_ns: u64,
    pub result_delivery_begin_ns: u64,
    pub result_delivery_done_ns: u64,
    pub cpp_pre_return_ns: u64,
    pub backend_result: u32,
    pub sdk_result: i32,
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
    delivery_mode: crate::performance::DeliveryMode,
    minor: u64,
    setup: Option<Vec<u8>>,
}
// Callback interior state is accessed only under this instance's nonblocking
// guard. The worker owns Shared/Session; removal excludes every live lease.
unsafe impl Sync for Live {}
static INSTANCES: crate::instances::Registry<Live> = crate::instances::Registry::new();
#[cfg(target_os = "linux")]
pub(crate) fn process_call_report(id: u64) -> Option<std::path::PathBuf> {
    INSTANCES.lease(id)?.report.clone()
}
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
#[cfg(target_os="linux")]
struct DeathWatch { stop:Arc<AtomicBool>, thread:Option<JoinHandle<()>> }
#[cfg(target_os="linux")]
impl DeathWatch {
    fn start(socket:std::net::TcpStream,s:Arc<Shared>)->io::Result<Self> {
        use std::os::fd::AsRawFd;
        let stop=Arc::new(AtomicBool::new(false));let stopping=stop.clone();
        let thread=thread::Builder::new().name("lvb-peer-health".into()).spawn(move||{
            let mut fd=libc::pollfd {fd:socket.as_raw_fd(),events:libc::POLLRDHUP,revents:0};
            while !stopping.load(Ordering::Acquire) {
                let result=unsafe {libc::poll(&mut fd,1,20)};
                if stopping.load(Ordering::Acquire) {break;}
                if result<0 {
                    if io::Error::last_os_error().kind()==io::ErrorKind::Interrupted {continue;}
                    s.fail(WORKER,u64::MAX);break;
                }
                if fd.revents&(libc::POLLRDHUP|libc::POLLHUP|libc::POLLERR|libc::POLLNVAL)!=0 {
                    s.fail(WORKER,u64::MAX);break;
                }
            }
        })?;
        Ok(Self {stop,thread:Some(thread)})
    }
}
#[cfg(target_os="linux")]
impl Drop for DeathWatch {
    fn drop(&mut self) {
        self.stop.store(true,Ordering::Release);
        if let Some(thread)=self.thread.take() {let _=thread.join();}
    }
}
impl Shared {
    /// Real-time path: one gettid and at most eight atomic loads. A thread is
    /// recorded the first time it calls; the worker is woken once to request
    /// its policy. The guard serializes callers, so a lost race retries on
    /// the next call and nothing here allocates, waits or logs.
    fn note_caller(&self) {
        #[cfg(target_os = "linux")]
        {
            let tid = unsafe { libc::gettid() } as u64;
            for slot in &self.callers {
                match slot.load(Ordering::Acquire) {
                    0 => {
                        if slot.compare_exchange(0, tid, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                            self.work.notify();
                        }
                        return;
                    }
                    seen if seen == tid => return,
                    _ => {}
                }
            }
        }
    }
}
fn worker(mut session: Session, s: Arc<Shared>, report: Option<std::path::PathBuf>, preparation: Option<crate::scheduling::Preparation>) {
    #[cfg(target_os = "linux")]
    {
        unsafe extern "C" { fn gettid() -> i32; }
        s.worker_thread.store(unsafe { gettid() } as u64, Ordering::Release);
    }
    let scheduling = crate::scheduling::prepare(preparation.clone());
    let mut caller_grants = crate::scheduling::CallerGrants::new(preparation);
    let mut input_observation = crate::input_observation::InputObservation::new(crate::observer::delivery_enabled());
    let mut previous_control = [0u64; 4];
    let mut deferred = None;
    let mut terminal_context = None;
    let mut phases = PhaseRecords::default();
    #[cfg(target_os="linux")]
    let mut death_watch:Option<DeathWatch>=None;
    if let Some(status) = &mut session.fault_status {
        status.generation = s.generation;
    }
    let run = (|| -> io::Result<()> {
        'work: loop {
            let work_observed = s.work.snapshot();
            s.work.check()?;
            if let Some(t) = &s.terminal {
                let (context,complete)=s.audio_progress(&session);
                if terminal_context != Some((context,complete)) {
                    t.progress(context,complete,None);
                    terminal_context = Some((context,complete));
                }
                if t.read().is_some() { return Err(invalid("terminal instance failure")); }
            }
            crate::preview::check_owner(&mut session.owner)?;
            if s.quit.load(Ordering::Acquire) || s.fault.load(Ordering::Acquire) != 0 {
                return Err(invalid("queued session fault or cancelled"));
            }
            session.service_worker_wakes(Instant::now(),
                || s.quit.load(Ordering::Acquire) || s.fault.load(Ordering::Acquire) != 0)?;
            caller_grants.service(&s.callers);
            if s.pending_control.load(Ordering::Acquire) {
                let mut mailbox = s
                    .control
                    .lock()
                    .map_err(|_| invalid("state mailbox poisoned"))?;
                if let Some(c) = mailbox.as_mut() {
                    let mut audio_fenced=true;
                    #[cfg(target_os="linux")]
                    if let Some(barrier)=c.direct_barrier {audio_fenced=s.direct_completed.load(Ordering::Acquire)>=barrier;}
                    if c.result.is_none() && s.requests.consumed() >= c.barrier && audio_fenced {
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
                            20 => session.configure(c.bytes.clone()).and_then(|reply| {
                                #[cfg(target_os="linux")]
                                if let Some(audio)=session.prepare_direct_audio()? {
                                    if s.direct.get().is_none() {
                                        s.direct.set(audio.channel.clone()).map_err(|_|invalid("direct channel already prepared"))?;
                                    }
                                    *s.prepared_audio.lock().map_err(|_|invalid("prepared audio owner poisoned"))?=Some(Box::new(audio));
                                    if death_watch.is_none() {death_watch=Some(DeathWatch::start(session.socket.try_clone()?,s.clone())?);}
                                }
                                Ok(reply)
                            }),
                            14 => session.transition(14).map(|_| {
                                s.processing_ready_epoch.store(0, Ordering::Release);
                                phases.record(&s, "deactivated", session.epoch);
                                s.ack.store(15, Ordering::Release);
                                vec![]
                            }),
                            _ => Err(invalid("unknown owner operation")),
                        }) };
                        if let Some(result) = completed {
                        previous_control[2] = crate::observer::monotonic_ns();
                        complete_control(&s, c, result,s.audio_progress(&session))?;
                        }
                    }
                }
            }
            s.worker_op.store(0, Ordering::Release);
            if session.mailbox.as_ref().is_some_and(|m| m.control_handoff_pending()) {
                session.wait_control_handoff(Instant::now() + Duration::from_secs(10),
                    || s.cancelled.load(Ordering::Acquire) || s.quit.load(Ordering::Acquire)
                        || s.fault.load(Ordering::Acquire) != 0)?;
                continue;
            }
            let Some(mut item) = deferred.take().or_else(|| s.requests.pop()) else {
                s.work.wait(work_observed, Instant::now() + crate::performance::INTERRUPT_INTERVAL,
                    session.worker_wait_fds())?;
                continue;
            };
            // Restore/lifecycle keep exclusive control-stream ownership. Only
            // ordered audio can pass an admitted read-only capture.
            if session.capture.is_some() && item.kind != AUDIO {
                deferred = Some(item);
                s.work.wait(work_observed, Instant::now() + crate::performance::INTERRUPT_INTERVAL,
                    session.worker_wait_fds())?;
                continue;
            }
            let mut pending_start = None;
            if item.kind == START && session.can_overlap_start() {
                s.worker_epoch.store(item.epoch, Ordering::Relaxed);
                s.worker_position.store(item.position, Ordering::Relaxed);
                s.worker_op.store(START as u64, Ordering::Release);
                let pending = session.begin_start(item.epoch)?;
                let next = loop {
                    if pending_start_cancelled(&s) {
                        return Err(invalid("pending Start cancelled"));
                    }
                    if admitted_control_precedes(&s, false)? {
                        #[cfg(test)]
                        s.pending_start_control_fences.fetch_add(1, Ordering::Release);
                        session.finish_start(pending, || {
                            s.cancelled.load(Ordering::Acquire)
                                || s.quit.load(Ordering::Acquire)
                                || s.fault.load(Ordering::Acquire) != 0
                        })?;
                        break None;
                    }
                    if let Some(next) = s.requests.pop() {
                        if admitted_control_precedes(&s, true)? {
                            deferred = Some(next);
                            #[cfg(test)]
                            s.pending_start_control_fences.fetch_add(1, Ordering::Release);
                            session.finish_start(pending, || {
                                s.cancelled.load(Ordering::Acquire)
                                    || s.quit.load(Ordering::Acquire)
                                    || s.fault.load(Ordering::Acquire) != 0
                            })?;
                            break None;
                        }
                        break Some(next);
                    }
                    if session.poll_start(pending)? { break None; }
                    let observed = s.work.snapshot();
                    // Close the publication/snapshot race before blocking. A
                    // socket wake may contain only a partial Started frame;
                    // consuming that prefix must not hide later local AUDIO.
                    if let Some(next) = s.requests.pop() {
                        if admitted_control_precedes(&s, true)? {
                            deferred = Some(next);
                            #[cfg(test)]
                            s.pending_start_control_fences.fetch_add(1, Ordering::Release);
                            session.finish_start(pending, || {
                                s.cancelled.load(Ordering::Acquire)
                                    || s.quit.load(Ordering::Acquire)
                                    || s.fault.load(Ordering::Acquire) != 0
                            })?;
                            break None;
                        }
                        break Some(next);
                    }
                    let observed = pending_start_wait_after_snapshot(&s, observed)?;
                    s.work.wait(observed, pending.receive_deadline,
                        session.pending_start_wait_fds())?;
                    #[cfg(test)]
                    s.pending_start_wakes.fetch_add(1, Ordering::Release);
                };
                match next {
                    Some(next) if next.kind == AUDIO => {
                        item = next;
                        pending_start = Some(pending);
                    }
                    Some(next) if next.kind == STOP => {
                        session.finish_start(pending, || {
                            s.cancelled.load(Ordering::Acquire)
                                || s.quit.load(Ordering::Acquire)
                                || s.fault.load(Ordering::Acquire) != 0
                        })?;
                        s.processing_ready_epoch.store(pending.epoch, Ordering::Release);
                        phases.record(&s, "processing_ready", pending.epoch);
                        s.ack.store((pending.epoch << 8) | u64::from(START + 1), Ordering::Release);
                        s.worker_epoch.store(next.epoch, Ordering::Relaxed);
                        s.worker_position.store(next.position, Ordering::Relaxed);
                        s.worker_op.store(STOP as u64, Ordering::Release);
                        session.transition_epoch(STOP as u16, next.epoch)?;
                        s.processing_ready_epoch.store(0, Ordering::Release);
                        phases.record(&s, "processing_stopped", next.epoch);
                        s.ack.store((next.epoch << 8) | u64::from(STOP + 1), Ordering::Release);
                        continue 'work;
                    }
                    Some(_) => return Err(invalid("pending Start ordered request kind")),
                    None => {
                        s.processing_ready_epoch.store(pending.epoch, Ordering::Release);
                        phases.record(&s, "processing_ready", pending.epoch);
                        s.ack.store((pending.epoch << 8) | u64::from(START + 1), Ordering::Release);
                        continue 'work;
                    }
                }
            }
            s.worker_epoch.store(item.epoch, Ordering::Relaxed);
            s.worker_position.store(item.position, Ordering::Relaxed);
            s.worker_op.store(item.kind as u64, Ordering::Release);
            match item.kind {
                AUDIO => {
                    let started = Instant::now();
                    if s.phase_diagnostics {
                        s.worker_binding.publish([
                            s.generation,
                            item.epoch,
                            item.position,
                            item.ticket,
                            item.parent[0],
                            session.state.next,
                        ]);
                    }
                    if let Some(status) = &mut session.fault_status {
                        status.delivery = std::array::from_fn(|i| s.delivery_totals[i].load(Ordering::Acquire));
                    }
                    let n = item.n as usize;
                    let original = item.data;
                    input_observation.observe(item.epoch,session.state.next,item.position,item.flags,[&item.data[0][..n],&item.data[1][..n]]);
                    session.gui_revision = item.gui_revision;
                    let capture_context = [s.generation,session.epoch,session.state.next,
                        session.position,u64::from(session.phase)];
                    let deadline = audio_transport_deadline(&item, Instant::now());
                    let deadline_identity = DeadlineIdentity {
                        generation: s.generation, epoch: item.epoch,
                        position: item.position, host_call: item.parent[0],
                        operation_ticket: item.ticket, operation_submitted: true,
                    };
                    if Instant::now() >= deadline {
                        s.fail_deadline(item.position, deadline_identity);
                        return Err(invalid("originating audio containment expired before worker service"));
                    }
                    let mut capture_completed = |result| {
                            let mut mailbox = s.control.lock().map_err(|_| invalid("state mailbox poisoned"))?;
                            let c = mailbox.as_mut().ok_or_else(|| invalid("admitted capture owner absent"))?;
                            ap1_native_client::need(c.op == 16 && c.result.is_none()
                                && s.pending_control.load(Ordering::Acquire), "admitted capture owner changed")?;
                            previous_control[2] = crate::observer::monotonic_ns();
                            complete_control(&s, c, result, (capture_context,Some(capture_context[3])))
                        };
                    let cancelled = || s.cancelled.load(Ordering::Acquire)
                        || s.quit.load(Ordering::Acquire)
                        || s.fault.load(Ordering::Acquire) != 0;
                    let processed = if let Some(pending) = pending_start {
                        let pending = pending.bounded_by(deadline);
                        session.process_positioned_pending_start(
                            pending, n, item.gain, item.flags,
                            [&item.data[0][..n], &item.data[1][..n]],
                            (item.epoch, item.position),
                            &item.events[..item.event_count as usize], item.context,
                            item.process_mode, deadline, cancelled, &mut capture_completed,
                            || {
                                s.processing_ready_epoch.store(pending.epoch, Ordering::Release);
                                phases.record(&s, "processing_ready", pending.epoch);
                                s.ack.store((pending.epoch << 8) | u64::from(START + 1), Ordering::Release);
                            },
                        )
                    } else {
                        session.process_positioned(
                            n, item.gain, item.flags,
                            [&item.data[0][..n], &item.data[1][..n]],
                            (item.epoch, item.position),
                            &item.events[..item.event_count as usize], item.context,
                            item.process_mode, deadline, cancelled, &mut capture_completed,
                        )
                    };
                    if item.completion.is_some() && !cancelled() && Instant::now() >= deadline {
                        s.fail_deadline(item.position, deadline_identity);
                    }
                    let (words, flags) = processed?;
                    #[cfg(test)]
                    if item.ticket != 0 && s.publication_hold_ticket.load(Ordering::Acquire) == item.ticket {
                        assert!(session.trace.validated.is_some());
                        s.publication_held_ticket.store(item.ticket, Ordering::Release);
                        let until = Instant::now() + Duration::from_secs(3);
                        while s.publication_hold_ticket.load(Ordering::Acquire) == item.ticket
                            && !cancelled() {
                            assert!(Instant::now() < until, "test publication hold was not released");
                            thread::yield_now();
                        }
                    }
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
                    #[cfg(feature = "rpi0")]
                    s.processed_frames.fetch_add(n as u64, Ordering::Relaxed);
                    let publish = s.wanted.load(Ordering::Acquire) == item.epoch;
                    let mut completion = Completion::from(item);
                    completion.returned = session.returned;
                    completion.windows_process_ns = session.trace.process_ns;
                    completion.windows_sequence = session.trace.sequence;
                    if publish && n>0 {
                        if let Some(pool)=s.extra.get() {
                            let map=session.mapping.as_ref().ok_or_else(||invalid("output mapping absent"))?;
                            let slot=pool.publish_available(&map.extra,n)
                                .ok_or_else(||invalid("extra output capacity"))?;
                            completion.audio.extra_slot=slot;
                        }
                    }
                    #[cfg(test)]
                    if item.ticket != 0 && s.final_publication_hold_ticket.load(Ordering::Acquire) == item.ticket {
                        assert!(session.trace.validated.is_some());
                        s.final_publication_held_ticket.store(item.ticket, Ordering::Release);
                        let until = Instant::now() + Duration::from_secs(3);
                        while s.final_publication_hold_ticket.load(Ordering::Acquire) == item.ticket
                            && !cancelled() {
                            assert!(Instant::now() < until, "test final publication hold was not released");
                            thread::yield_now();
                        }
                    }
                    // The owned completion and any extra slot are fully prepared.
                    // Check at publication, not before the intervening copy work.
                    if cancelled() {
                        s.release_output(&completion.audio);
                        return Err(invalid("audio completion cancelled before publication"));
                    }
                    if item.completion.is_some() && Instant::now() >= deadline {
                        s.release_output(&completion.audio);
                        s.fail_deadline(item.position, deadline_identity);
                        return Err(invalid("originating audio containment expired before result publication"));
                    }
                    let published = if publish { s.publish_result(completion) } else { None };
                    if publish && published.is_none() {
                        s.release_output(&completion.audio);
                        s.fail(OVERFLOW, item.position);
                        return Err(invalid("completed output capacity"));
                    }
                    session.trace.queued = item.queued;
                    session.trace.parent = item.parent;
                    session.trace.previous_control = previous_control;
                    session.trace.published = published;
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
                    s.processing_ready_epoch.store(if item.kind == START { item.epoch } else { 0 }, Ordering::Release);
                    phases.record(&s, if item.kind == START { "processing_ready" } else { "processing_stopped" }, item.epoch);
                    s.completion.notify();
                    s.ack.store(
                        ((item.epoch) << 8) | u64::from(item.kind + 1),
                        Ordering::Release,
                    );
                }
                DEACTIVATE => {
                    session.transition(14)?;
                    s.processing_ready_epoch.store(0, Ordering::Release);
                    phases.record(&s, "deactivated", session.epoch);
                    s.ack.store(15, Ordering::Release);
                }
                CLOSE => return Ok(()),
                _ => return Err(invalid("queued control kind")),
            }
        }
    })();
    #[cfg(target_os="linux")]
    drop(death_watch);
    s.processing_ready_epoch.store(0, Ordering::Release);
    phases.record(&s, if run.is_ok() { "closing" } else { "failed" }, session.epoch);
    if let Err(ref error) = run {
        // Ordinary Close returns Ok. Cancellation during teardown is not a
        // new terminal incident unless the worker already holds a fault.
        if !s.quit.load(Ordering::Acquire) || s.fault.load(Ordering::Acquire) != 0 {
            if let Some(t) = &s.terminal {
                let (context,complete)=s.audio_progress(&session);t.progress(context,complete,None);
                t.fail_native(match s.fault.load(Ordering::Acquire) {0=>WORKER, code=>code});
            }
        }
        // Publish containment before exposing worker failure to the callback.
        // Unclassified failures retain their existing error behavior.
        s.terminal_record();
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
    if let Some(path) = &report {
        scheduling.report(path);
        caller_grants.report(path);
        crate::preview::append_report(path, input_observation.report().as_bytes());
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
    let retired_epoch = session.epoch;
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
    // Write phase measurements only after processing and ownership retirement.
    // Even a slow diagnostic disk cannot delay steady-state audio delivery.
    if let Some(path) = &report {
        phases.write(path);
        crate::preview::append_report(path, phase_text(&PhaseRecord::read(&s,
            if s.retired.load(Ordering::Acquire) { "retired" } else { "retirement_unconfirmed" },
            retired_epoch), phases.omitted).as_bytes());
    }
    s.ack.store(6, Ordering::Release);
}
#[derive(Clone, Copy)]
struct PhaseRecord {
    phase: &'static str,
    epoch: u64,
    monotonic_ns: u64,
    delivery: [[u64; 6]; 2],
}
impl PhaseRecord {
    const EMPTY: Self = Self { phase: "", epoch: 0, monotonic_ns: 0, delivery: [[0; 6]; 2] };
    fn read(s: &Shared, phase: &'static str, epoch: u64) -> Self {
        Self { phase, epoch, monotonic_ns: crate::observer::monotonic_ns(),
            delivery: std::array::from_fn(|p| std::array::from_fn(|i|
                s.delivery_phases[p][i].load(Ordering::Acquire))) }
    }
}
struct PhaseRecords { records: [PhaseRecord; 128], length: usize, omitted: u64 }
impl Default for PhaseRecords {
    fn default() -> Self { Self { records: [PhaseRecord::EMPTY; 128], length: 0, omitted: 0 } }
}
impl PhaseRecords {
    fn record(&mut self, s: &Shared, phase: &'static str, epoch: u64) {
        if self.length == self.records.len() { self.omitted += 1; return; }
        self.records[self.length] = PhaseRecord::read(s, phase, epoch);
        self.length += 1;
    }
    fn write(&self, path: &std::path::Path) {
        for record in &self.records[..self.length] {
            crate::preview::append_report(path, phase_text(record, self.omitted).as_bytes());
        }
    }
}
fn phase_text(record: &PhaseRecord, omitted: u64) -> String {
    let counters = |d: [u64; 6]| format!(
        "{{\"admitted_frames\":{},\"missing_frames\":{},\"gaps\":{},\"expired_frames\":{},\"delivered_frames\":{},\"priming_frames\":{}}}",
        d[0], d[1], d[2], d[3], d[4], d[5]);
    format!("{{\"event\":\"ap7_audio_phase\",\"schema\":1,\"phase\":\"{}\",\"epoch\":{},\"monotonic_ns\":{},\"startup\":{},\"processing\":{},\"omitted_phase_markers\":{}}}\n",
        record.phase, record.epoch, record.monotonic_ns, counters(record.delivery[0]), counters(record.delivery[1]), omitted)
}
fn processing_phase(epoch: u64, acknowledged_epoch: u64) -> usize {
    usize::from(epoch != 0 && acknowledged_epoch == epoch)
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
fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".into(), |value| value.to_string())
}
fn optional_bool(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "null",
    }
}
fn worker_binding_text(observation: WorkerBindingObservation) -> String {
    let stable = observation.stability == crate::fault_status::Stability::Stable;
    let value = |word| stable.then_some(observation.words[word]);
    format!(concat!("{{\"stability\":\"{}\",\"attempts\":{},\"publication\":[{},{}],",
        "\"generation\":{},\"epoch\":{},\"position\":{},\"operation_ticket\":{},",
        "\"host_call\":{},\"protocol_sequence\":{}}}"),
        observation.stability.name(), observation.attempts,
        observation.publication_before, observation.publication_after,
        optional_u64(value(0)), optional_u64(value(1)), optional_u64(value(2)),
        optional_u64(value(3)), optional_u64(value(4)), optional_u64(value(5)))
}
fn refusal_lane_text(
    index: usize,
    lane: crate::fault_status::Lane,
    identity: DeadlineIdentity,
    native: Option<[u64; 4]>,
    worker: Option<[u64; WORKER_BINDING_WORDS]>,
    worker_matches_refusal: bool,
) -> String {
    let names = ["native_transport", "windows_delivery", "windows_ui_owner"];
    let clocks = [
        "linux_clock_monotonic_ns",
        "windows_query_performance_counter",
        "windows_query_performance_counter",
    ];
    let stable = lane.stability == crate::fault_status::Stability::Stable;
    let value = |word| stable.then_some(lane.words[word]);
    let match_fields = |reference: Option<[u64; 4]>| {
        reference.map(|reference| [
            lane.words[0] == reference[0],
            lane.words[1] == reference[1],
            lane.words[3] == reference[2],
            lane.words[2] == reference[3],
        ])
    };
    let native_matches = (stable && index != 0).then(|| match_fields(native)).flatten();
    let worker_matches = (stable).then(|| worker.map(|binding| [
        lane.words[0] == binding[0],
        lane.words[1] == binding[1],
        lane.words[3] == binding[2],
        lane.words[2] == binding[5],
    ])).flatten();
    let correlated = |matches: Option<[bool; 4]>| matches.map(|matches| matches.into_iter().all(|v|v));
    let correlates_worker = correlated(worker_matches);
    let match_text = |matches: Option<[bool; 4]>| match matches {
        Some(matches) => format!("{{\"generation\":{},\"epoch\":{},\"position\":{},\"sequence\":{}}}",
            matches[0],matches[1],matches[2],matches[3]),
        None => "{\"generation\":null,\"epoch\":null,\"position\":null,\"sequence\":null}".into(),
    };
    let delivery = if stable && lane.word_count == 16 {
        format!(concat!(",\"delivery\":{{\"admitted_frames\":{},\"missing_frames\":{},",
            "\"gaps\":{},\"expired_frames\":{},\"delivered_frames\":{},\"priming_frames\":{}}}"),
            lane.words[10], lane.words[11], lane.words[12], lane.words[13], lane.words[14], lane.words[15])
    } else {
        String::new()
    };
    format!(concat!("{{\"lane\":\"{}\",\"stability\":\"{}\",\"attempts\":{},",
        "\"publication\":[{},{}],\"generation\":{},\"epoch\":{},\"sequence\":{},",
        "\"position\":{},\"stage\":{},\"detail\":{},\"thread_id\":{},\"process_id\":{},",
        "\"clock\":{{\"domain\":\"{}\",\"ticks\":{},\"frequency\":{}}},",
        "\"matches_refusal\":{{\"generation\":{},\"epoch\":{},\"position\":{}}},",
        "\"matches_native\":{},\"correlates_native\":{},",
        "\"matches_worker\":{},\"correlates_worker\":{},\"correlates_refusal\":{}{} }}"),
        names[index], lane.stability.name(), lane.attempts,
        lane.publication_before, lane.publication_after,
        optional_u64(value(0)), optional_u64(value(1)), optional_u64(value(2)),
        optional_u64(value(3)), optional_u64(value(4)), optional_u64(value(5)),
        optional_u64(value(8)), optional_u64(value(9)), clocks[index],
        optional_u64(value(6)), optional_u64(value(7)),
        optional_bool(value(0).map(|v| v == identity.generation)),
        optional_bool(value(1).map(|v| v == identity.epoch)),
        optional_bool(value(3).map(|v| v == identity.position)),
        match_text(native_matches), optional_bool(correlated(native_matches)),
        match_text(worker_matches), optional_bool(correlates_worker),
        optional_bool(correlates_worker.map(|matched| matched && worker_matches_refusal)), delivery)
}
fn refusal_text(snapshot: RefusalSnapshot) -> String {
    let native = snapshot.status.lanes[0];
    let binding_before = snapshot.worker_binding[0];
    let binding_after = snapshot.worker_binding[1];
    let binding_unchanged = binding_before.stability == crate::fault_status::Stability::Stable
        && binding_after.stability == crate::fault_status::Stability::Stable
        && binding_before.publication_before == binding_after.publication_before
        && binding_before.words == binding_after.words;
    let binding_matches_refusal = binding_unchanged
        && snapshot.identity.operation_submitted
        && binding_before.words[..5] == [
            snapshot.identity.generation,
            snapshot.identity.epoch,
            snapshot.identity.position,
            snapshot.identity.operation_ticket,
            snapshot.identity.host_call,
        ];
    let native_matches_binding = binding_unchanged
        && native.stability == crate::fault_status::Stability::Stable
        && native.words[0] == binding_before.words[0]
        && native.words[1] == binding_before.words[1]
        && native.words[2] == binding_before.words[5]
        && native.words[3] == binding_before.words[2];
    let callback_binding = if binding_matches_refusal && native_matches_binding { "matched" } else { "unknown" };
    // A stable native row can correlate AP12 lanes by their published tuple.
    // Callback binding additionally requires one unchanged, coherent worker
    // publication containing the operation ticket and host call.
    let native_tuple = (native.stability == crate::fault_status::Stability::Stable)
        .then_some([native.words[0],native.words[1],native.words[3],native.words[2]]);
    let native_sequence = native_tuple.map(|tuple|tuple[3]);
    let worker_tuple = binding_unchanged.then_some(binding_before.words);
    let mut records = format!(concat!("{{\"event\":\"ap23_exact_deadline_status\",\"schema\":1,\"ap12_schema\":2,",
        "\"refusal\":{{\"generation\":{},\"epoch\":{},\"position\":{},\"host_call\":{},",
        "\"operation_ticket\":{},\"operation_submitted\":{}}},",
        "\"worker_binding\":{{\"before\":{},\"after\":{},\"unchanged\":{},",
        "\"matches_refusal\":{},\"native_status_matches_binding\":{}}},",
        "\"native_sequence_reference\":{},\"callback_binding\":\"{}\",",
        "\"progress_is_atomic_snapshot\":false}}\n"),
        snapshot.identity.generation, snapshot.identity.epoch, snapshot.identity.position,
        snapshot.identity.host_call, snapshot.identity.operation_ticket,
        snapshot.identity.operation_submitted,
        worker_binding_text(binding_before), worker_binding_text(binding_after), binding_unchanged,
        binding_matches_refusal, native_matches_binding, optional_u64(native_sequence), callback_binding);
    // The existing report sink caps each JSONL record at 2048 bytes. Keep one
    // captured snapshot, but export its three independently sampled AP12 lanes
    // as correlated bounded records instead of silently dropping one oversized
    // aggregate record.
    for index in 0..3 {
        let lane = refusal_lane_text(index, snapshot.status.lanes[index], snapshot.identity,
            native_tuple, worker_tuple, binding_matches_refusal);
        records.push_str(&format!(concat!("{{\"event\":\"ap23_exact_deadline_lane\",\"schema\":1,",
            "\"ap12_schema\":2,\"refusal\":{{\"generation\":{},\"epoch\":{},",
            "\"position\":{},\"host_call\":{},\"operation_ticket\":{}}},\"status\":{}}}\n"),
            snapshot.identity.generation, snapshot.identity.epoch, snapshot.identity.position,
            snapshot.identity.host_call, snapshot.identity.operation_ticket, lane));
    }
    records
}
fn export_refusal(s: &Shared, path: &std::path::Path) {
    if let Some(snapshot) = s.first_refusal.take_for_export() {
        crate::preview::append_records(path, &refusal_text(snapshot));
    }
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
    open_execution(max, handle, minor, identity, None)
}
unsafe fn open_execution(
    max: u32,
    handle: *mut u64,
    minor: u64,
    identity: Option<state::Identity>,
    execution: Option<ap1_native_client::admission::ExecutionIdentity>,
) -> u32 {
    open_with(max, handle, minor, identity, execution, None, || {
        if minor >= 6 {
            crate::preview::discover_performance(identity, execution)
        } else if let Some(identity) = identity {
            crate::preview::discover_commercial(identity)
        } else {
            binding(minor == 4)
        }
    })
}

#[cfg(feature = "rpi0")]
pub(crate) unsafe fn open_bound(
    max: u32,
    handle: *mut u64,
    identity: state::Identity,
    binding: crate::preview::Binding,
) -> u32 {
    let report = binding.directory.join("rpi0.performance.jsonl");
    open_with(max, handle, 12, Some(identity), None, Some(report), || Ok(binding))
}

unsafe fn open_with(
    max: u32,
    handle: *mut u64,
    minor: u64,
    identity: Option<state::Identity>,
    execution: Option<ap1_native_client::admission::ExecutionIdentity>,
    report_override: Option<std::path::PathBuf>,
    binding_source: impl FnOnce() -> io::Result<crate::preview::Binding>,
) -> u32 {
    if handle.is_null() || !(1..=256).contains(&max) {
        return 1;
    }
    crate::ffi(|| {
        match INSTANCES.insert(|| {
            let binding = binding_source()?;
            let report = report_override.or_else(|| binding.owner.as_ref().map(|_| {
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
            }));
            let installed_delay = binding.installed_delay;
            let delivery_mode = binding.delivery_mode;
            let preparation = crate::scheduling::Preparation::from_binding(&binding);
            let mut shared = Shared::try_new()?;
            let mut session = Session::open(binding, max as usize, minor)?;
            session.identity = identity;
            shared.phase_diagnostics = crate::observer::phase_delivery_enabled();
            if shared.phase_diagnostics {
                shared.fault_reader = session.fault_status.as_ref().map(|status| status.reader());
            }
            shared.gui = session.gui.clone();
            shared.terminal = session.fault_status.as_ref().map(|f| f.terminal.clone());
            shared.identity = identity;
            shared.execution = execution;
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
                .spawn(move || worker(session, peer, worker_report, preparation))?;
            Ok::<_, io::Error>(Live {
                shared,
                callback: UnsafeCell::new(Callback::new()),
                busy: AtomicBool::new(false),
                worker: Some(t),
                report,
                max: max as usize,
                recovery_blocked: false,
                installed_delay,
                delivery_mode,
                minor,
                setup: None,
            })
        }) {
            Ok(Ok(id)) => {
                *handle = id;
                0
            }
            Ok(Err(e)) => retain(&e),
            // Preserve Full, temporary Busy, and terminal generation exhaustion.
            // This startup-only refusal creates no manager/session ownership.
            Err(e) => retain(&io::Error::other(e)),
        }
    }) as u32
}
// RT-safe classification only. Zero means not latched, not proof of health.
// Registry::lease uses one indexed slot, one fetch_add/fetch_sub, one pointer
// load and an identity comparison, with no allocation, lock, wait or retry.
// This query adds exactly one AtomicBool load and never reads the mapping.
#[no_mangle]
pub extern "C" fn if2_terminal_status(id: u64) -> u32 {
    INSTANCES.lease(id).is_some_and(|l| l.shared.terminal_latched.load(Ordering::Acquire)) as u32
}
/// Bounded non-RT query. The Arc retains mapped custody after session unlink.
#[no_mangle]
pub unsafe extern "C" fn if1_terminal(id:u64,out:*mut crate::terminal::Record)->u32 {
    crate::ffi(|| {
        if out.is_null(){return 1;}
        let Some(l)=INSTANCES.lease(id) else{return 1;};
        *out=crate::terminal::Record::default();
        if let Some(record)=l.shared.terminal_record(){*out=record;}
        0
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
unsafe fn recover_state(id: u64, revision: u64, capacity: usize) -> Result<Vec<u8>, i32> {
        match INSTANCES.update(id, |l| -> io::Result<Vec<u8>> {
            if l.recovery_blocked || l.shared.fault.load(Ordering::Acquire) == 0 {
                return Err(invalid("recovery requires a failed, contained instance"));
            }
            let snapshot = l
                .shared
                .snapshots
                .lock()
                .map_err(|_| invalid("snapshot store poisoned"))?
                .select(revision)?;
            need(snapshot.bytes.len() <= capacity, "recovery output capacity")?;
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
            // Registry update has excluded every callback lease and the old
            // worker is joined. Preserve its one refusal record before either
            // Shared ownership or the report path can be replaced.
            if let Some(path) = &l.report {
                export_refusal(&l.shared,path);
            }
            // A failed new startup/restore cannot silently authorize another
            // replacement. The owner still contains any newly owned endpoint.
            l.recovery_blocked = true;
            let binding = if l.minor >= 6 {
                crate::preview::discover_performance(l.shared.identity, l.shared.execution)?
            } else {
                binding(true)?
            };
            if binding.installed_delay != l.installed_delay || binding.delivery_mode != l.delivery_mode {
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
            let preparation = crate::scheduling::Preparation::from_binding(&binding);
            let mut shared = Shared::try_new()?;
            let mut session = Session::open(binding, l.max.min(if crate::performance::whole_block(l.minor) { CAP } else { LEGACY_CAP }), l.minor)?;
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
            if l.minor>=13 {if let Some(bytes)=&l.setup {shared.prepare_outputs(bytes)?;}}
            shared.phase_diagnostics = l.shared.phase_diagnostics;
            if shared.phase_diagnostics {
                shared.fault_reader = session.fault_status.as_ref().map(|status| status.reader());
            }
            shared.gui = session.gui.clone();
            shared.terminal = session.fault_status.as_ref().map(|f| f.terminal.clone());
            shared.generation = l
                .shared
                .generation
                .checked_add(1)
                .ok_or_else(|| invalid("transport generation exhausted"))?;
            shared.snapshots = l.shared.snapshots.clone();
            shared.identity = l.shared.identity;
            shared.execution = l.shared.execution;
            shared.state_capable.store(true, Ordering::Release);
            shared.ack.store(17, Ordering::Release);
            shared.observer = session.witness.as_ref().map(|w| w.shared.clone());
            let shared = Arc::new(shared);
            let peer = shared.clone();
            let worker_report = report.clone();
            let t = thread::Builder::new()
                .name("ap6-transport".into())
                .spawn(move || worker(session, peer, worker_report, preparation))?;
            l.shared = shared;
            l.worker = Some(t);
            l.report = report;
            l.callback = UnsafeCell::new(l.callback.get_mut().replacement());
            l.recovery_blocked = false;
            Ok(snapshot.bytes)
        }) {
            Ok(Ok(bytes)) => Ok(bytes),
            Ok(Err(error)) => Err(retain(&error)),
            Err(code) => Err(code as i32),
        }
}
#[no_mangle]
pub unsafe extern "C" fn ap6_recover(
    id: u64, revision: u64, out: *mut u8, capacity: u32, size: *mut u32,
) -> u32 {
    crate::ffi(|| {
        if out.is_null() || size.is_null() { return 1; }
        match recover_state(id, revision, capacity as usize) {
            Ok(bytes) => {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
                *size = bytes.len() as u32;
                0
            }
            Err(code) => code,
        }
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap6_recover_owned_v1(
    id: u64, revision: u64, out: *mut state::OwnedState,
) -> u32 {
    crate::ffi(|| {
        if !state::owned_output_ready(out) { return 1; }
        match recover_state(id, revision, state::HEADER_SIZE + state::LIMIT) {
            Ok(bytes) => { state::give_owned(out, bytes); 0 }
            Err(code) => code,
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
        if op == 18 {
            let revision = s.curve_state_revision.load(Ordering::Relaxed)
                .checked_add(1).ok_or_else(|| invalid("parameter state revision exhausted"))?;
            s.curve_state_revision.store(revision, Ordering::Release);
        }
        *c = Some(Control {
            barrier,
            #[cfg(target_os="linux")]
            direct_barrier:s.direct.get().map(|_|s.direct_submitted.load(Ordering::Acquire)),
            op,
            bytes,
            result: None,
        });
        s.pending_control.store(true, Ordering::Release);
    }
    s.work.notify();
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        let observed = s.control_acknowledgement.snapshot();
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
        if s.cancelled.load(Ordering::Acquire) || s.quit.load(Ordering::Acquire) {
            return Err(invalid(
                "state acknowledgement cancelled; no retry",
            ));
        }
        if Instant::now() >= until || s.fault.load(Ordering::Acquire) != 0 {
            s.fail(WORKER, u64::MAX);
            return Err(invalid(
                "state acknowledgement missing; instance failed, no retry",
            ));
        }
        #[cfg(test)]
        s.control_waits.fetch_add(1, Ordering::Release);
        s.control_acknowledgement.wait(observed, until);
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
pub unsafe extern "C" fn ap22_curve_parameters(id: u64, ids: *const u32, count: u32) -> u32 {
    crate::ffi(|| {
        if count > 8192 || (count > 0 && ids.is_null()) { return 1; }
        let Some(l) = INSTANCES.lease(id) else { return 1; };
        let Some(_guard) = Guard::acquire(&l) else { return 3; };
        let callback = &mut *l.callback.get();
        if callback.running || callback.epoch != 0 || l.shared.fault.load(Ordering::Acquire) != 0 { return 2; }
        let ids = if count == 0 { &[] } else { std::slice::from_raw_parts(ids, count as usize) };
        if callback.curve_values.configure(ids).is_err() { return 1; }
        0
    }) as u32
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
    if io.is_null() || out.is_null() || notifications > 1
        || !(4..=crate::performance::MAX_BUS_CONTRACT_BYTES).contains(&len) {
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
            let live = INSTANCES.lease(id).ok_or_else(|| invalid("setup instance absent"))?;
            let installed = live.installed_delay;
            let delivery_mode = live.delivery_mode;
            drop(live);
            let remembered = if let Some(delay) = installed {
                delay
            } else {
                crate::performance::selected_delay(maximum)?
            };
            let delay = delivery_mode.effective_delay(maximum, remembered)?;
            let minor = INSTANCES.lease(id).ok_or_else(|| invalid("setup instance absent"))?.minor;
            let mut bytes = crate::performance::wire_version(maximum, mode, rate, crate::performance::whole_block(minor))?;
            if !io.is_empty() {
                bytes[20..24]
                    .copy_from_slice(&(if notifications { 3u32 } else { 1u32 }).to_le_bytes());
                bytes.extend(io);
            }
            crate::performance::validate_wire(&bytes)?;
            {
                let live=INSTANCES.lease(id).ok_or_else(||invalid("setup instance absent"))?;
                if live.minor>=13 {live.shared.prepare_outputs(&bytes)?;}
            }
            #[cfg(target_os="linux")]
            INSTANCES.update(id,|l|->io::Result<()> {
                if l.callback.get_mut().running {return Err(invalid("configuration during playback"));}
                l.callback.get_mut().direct_audio.take();Ok(())
            }).map_err(|_|invalid("setup instance ownership"))??;
            let reply = control(id, 20, bytes.clone())?;
            let vendor = ap1_native_client::get(&reply[..4]) as u32;
            let total = vendor
                .checked_add(delay)
                .ok_or_else(|| invalid("latency overflow"))?;
            INSTANCES.update(id,|l| -> io::Result<()> {
                let callback=l.callback.get_mut();
                if callback.running {return Err(invalid("configuration during playback"));}
                let block_maximum = if crate::performance::whole_block(minor) { maximum as usize }
                    else { maximum.min(256) as usize };
                let channels = l.shared.extra.get().map_or(2, |pool| pool.channels + 2);
                callback.prepare(block_maximum, channels, delay as usize); l.max=maximum as usize;
                #[cfg(target_os="linux")]
                {callback.direct_audio=l.shared.prepared_audio.lock().map_err(|_|invalid("prepared audio owner poisoned"))?.take();}
                l.setup=Some(bytes);
                if let Some(path)=&l.report {
                    crate::preview::append_report(path,format!("{{\"event\":\"ap9_setup\",\"protocol_minor\":{minor},\"host_maximum\":{maximum},\"transport_maximum\":{},\"sample_rate\":{rate},\"bridge_frames\":{delay},\"vendor_frames\":{vendor},\"total_frames\":{total},\"vendor_precision_bits\":{}}}\n",if crate::performance::whole_block(minor) {maximum} else {maximum.min(256)},ap1_native_client::get(&reply[8..12])).as_bytes());
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
unsafe fn read_state(id: u64, restore: *const u8, n: u32) -> io::Result<Vec<u8>> {
    need(n as usize <= state::HEADER_SIZE + state::LIMIT, "restore state capacity")?;
    let request = if restore.is_null() {
        need(n == 0, "capture has no input bytes")?;
        Ok((16, vec![]))
    } else {
        let live = INSTANCES.lease(id).ok_or_else(|| invalid("state handle"))?;
        state::restore_payload(live.shared.identity, std::slice::from_raw_parts(restore, n as usize))
            .map(|bytes| (18, bytes.to_vec()))
    };
    request.and_then(|(op, bytes)| control(id, op, bytes))
}
#[no_mangle]
pub unsafe extern "C" fn ap4_state(
    id: u64, restore: *const u8, n: u32, out: *mut u8, capacity: u32, size: *mut u32,
) -> u32 {
    crate::ffi(|| {
        if out.is_null() || size.is_null() { return 1; }
        match read_state(id, restore, n).and_then(|bytes| {
            need(bytes.len() <= capacity as usize, "state output capacity")?;
            Ok(bytes)
        }) {
            Ok(bytes) => {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
                *size = bytes.len() as u32;
                0
            }
            Err(error) => retain(&error),
        }
    }) as u32
}
#[no_mangle]
pub unsafe extern "C" fn ap4_state_owned_v1(
    id: u64, restore: *const u8, n: u32, out: *mut state::OwnedState,
) -> u32 {
    crate::ffi(|| {
        if !state::owned_output_ready(out) { return 1; }
        match read_state(id, restore, n) {
            Ok(bytes) => { state::give_owned(out, bytes); 0 }
            Err(error) => retain(&error),
        }
    }) as u32
}
#[cfg(feature = "rpi0")]
pub(crate) fn wait_started(id: u64) -> io::Result<()> {
    let shared = INSTANCES.lease(id).ok_or_else(|| invalid("start acknowledgement handle"))?.shared.clone();
    let epoch = shared.wanted.load(Ordering::Acquire);
    let expected = (epoch != 0 && epoch <= (u64::MAX >> 8))
        .then(|| (epoch << 8) | u64::from(START + 1))
        .ok_or_else(|| invalid("start acknowledgement epoch"))?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if shared.ack.load(Ordering::Acquire) == expected { return Ok(()); }
        if shared.fault.load(Ordering::Acquire) != 0 || Instant::now() >= deadline {
            return Err(invalid("start acknowledgement missing"));
        }
        thread::sleep(Duration::from_micros(50));
    }
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
    l.shared.work.notify();
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
        false,
        &[],
        None,
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
    contain_terminal: bool,
    extra: &[*mut f32],
    actual_mode: Option<u32>,
) -> u32 {
    process_events_traced(id,n,gain,flags,left,right,out_left,out_right,out_flags,delivery,
        events,context,detailed,entered_ns,contain_terminal,extra,actual_mode,None)
}
#[allow(clippy::too_many_arguments)]
unsafe fn process_events_traced(
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
    contain_terminal: bool,
    extra: &[*mut f32],
    actual_mode: Option<u32>,
    mut trace: Option<&mut PhaseTrace>,
) -> u32 {
    let callback_entered = Instant::now();
    let Some(l) = INSTANCES.lease(id) else { return 1; };
    let Some(_guard) = Guard::acquire(&l) else { return 3; };
    if let Some(t) = trace.as_deref_mut() {
        t.rust_entry_ns = crate::observer::monotonic_ns();
        t.phase_reached |= PHASE_RUST_ENTRY;
        if t.rust_entry_ns != 0 { t.clock_valid |= PHASE_RUST_ENTRY; }
    }
    let result = process_events_guarded(&l,callback_entered,n,gain,flags,left,right,out_left,out_right,out_flags,
        delivery,events,context,detailed,entered_ns,contain_terminal,extra,actual_mode,
        trace.as_deref_mut());
    if result != 0 { (*l.callback.get()).completion_policy = None; }
    if let Some(t) = trace {
        t.backend_result = result;
        t.phase_reached |= PHASE_BACKEND_RESULT;
        t.rust_pre_return_ns = crate::observer::monotonic_ns();
        t.phase_reached |= PHASE_RUST_PRE_RETURN;
        if t.rust_pre_return_ns != 0 { t.clock_valid |= PHASE_RUST_PRE_RETURN; }
    }
    result
}
#[allow(clippy::too_many_arguments)]
unsafe fn process_events_guarded(
    l: &Live,
    callback_entered: Instant,
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
    contain_terminal: bool,
    extra: &[*mut f32],
    actual_mode: Option<u32>,
    mut trace: Option<&mut PhaseTrace>,
) -> u32 {
    (*l.callback.get()).completion_policy = None;
    (*l.callback.get()).windows_timing = WindowsProcessTiming::default();
    l.shared.note_caller();
    let n = n as usize;
    if extra.len()>62 || (n>0 && extra.len()!=l.shared.extra.get().map_or(0,|p|p.channels)) {
        return if detailed {0x101} else {1};
    }
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
    let configured_mode = l.setup.as_ref().map_or(0, |setup|
        u32::from_le_bytes(setup[4..8].try_into().unwrap()));
    let mode = actual_mode.unwrap_or(configured_mode);
    if !crate::performance::valid_process_mode(configured_mode, mode)
        || (actual_mode.is_some() && l.minor < 15 && mode != configured_mode) {
        return if detailed { 0x10A } else { 1 };
    }
    let offline = mode == 2;
    let exact = offline || n == 0 || l.delivery_mode == crate::performance::DeliveryMode::SameCallback;
    if contain_terminal && l.shared.terminal_latched.load(Ordering::Acquire) {
        return if exact { 2 } else { CONTAINED_TERMINAL };
    }
    let whole_block = crate::performance::whole_block(l.minor);
    // One original callback-entry containment bound includes all ordered debt
    // and presentation. It is independent of actual N, maximum M and Fs.
    // Offline remains separate; legacy/ARM retain their existing policy.
    // Sample Instant before CLOCK_MONOTONIC, so a scheduling interruption
    // between reads can shorten the remaining wait, never renew the allowance.
    let phase_budget_before_ns = trace.as_ref().map(|_| crate::observer::monotonic_ns());
    let budget_now = Instant::now();
    let budget_now_ns = crate::observer::monotonic_ns();
    let phase_budget_after_ns = trace.as_ref().map(|_| crate::observer::monotonic_ns());
    let completion = if whole_block {
        l.setup.as_ref().map(|_| crate::performance::CompletionPolicy::new(
            mode, l.delivery_mode, n, entered_ns,
            budget_now_ns, budget_now))
    } else { None };
    (*l.callback.get()).completion_policy = completion;
    // Half of this block's duration. A zero-frame flush carries parameter
    // changes that must reach the plug-in in order, so it keeps the full wait.
    (*l.callback.get()).unready_bound = match (&l.setup, completion) {
        (Some(setup), Some(_)) if !offline && n > 0 => {
            let rate = f64::from_le_bytes(setup[8..16].try_into().unwrap());
            (rate.is_finite() && rate > 0.).then(|| Duration::from_secs_f64(n as f64 / rate / 2.))
        }
        _ => None,
    };
    let completion_deadline = completion.map(|p| p.deadline);
    if let (Some(t), Some(policy)) = (trace.as_deref_mut(), completion) {
        t.frames = n as u64;
        t.mode = mode as u64;
        t.delivery_mode = l.delivery_mode as u64;
        t.exact = u64::from(policy.exact);
        t.allowance_ns = policy.allowance.as_nanos().min(u64::MAX as u128) as u64;
        let remaining = policy.deadline.saturating_duration_since(budget_now)
            .as_nanos().min(u64::MAX as u128) as u64;
        let before = phase_budget_before_ns.unwrap_or(0);
        let after = phase_budget_after_ns.unwrap_or(0);
        if before != 0 && after != 0 {
            t.wait_deadline_lower_ns = before.saturating_add(remaining);
            t.wait_deadline_upper_ns = after.saturating_add(remaining);
            t.clock_valid |= PHASE_POLICY;
        }
        t.phase_reached |= PHASE_POLICY;
    }
    // The current product transports the DAW's queue unchanged. Its vendor
    // processor owns implicit parameter values, including after state/GUI
    // changes and transport seeks. Curve reconstruction is legacy-only.
    if !whole_block {
    let callback = &mut *l.callback.get();
    let gui_revision = l.shared.gui.as_ref().map_or(0, |gui| gui.revision_cursor());
    let gui_unchanged = gui_revision == callback.curve_gui_revision;
    let state_revision = l.shared.curve_state_revision.load(Ordering::Acquire);
    let state_unchanged = state_revision == callback.curve_state_revision;
    let continuation = callback.curve_continuation.is_some_and(|expected| {
        expected.present == context.present && (context.present == 0 ||
            expected.rate == context.rate && expected.state & 0x21006 == context.state & 0x21006 &&
            expected.project == context.project &&
            (context.state & 0x20000 == 0 || expected.continuous == context.continuous) &&
            (context.state & 0x1004 != 0x1004 ||
                expected.cycle_start == context.cycle_start && expected.cycle_end == context.cycle_end))
    }) && gui_unchanged && state_unchanged;
    if !continuation { callback.curve_carry.count = 0; }
    // A transport seek discards the future endpoint but does not change the
    // parameter value established by the previous accepted processing block.
    // An external edit does change it; require a new explicit/observed anchor.
    if !gui_unchanged || !state_unchanged {
        callback.curve_values.invalidate(); callback.curve_gui_revision = gui_revision;
        callback.curve_state_revision = state_revision;
    }
    if callback.curve_plan.prepare(events, n, &callback.curve_carry, &callback.curve_values).is_err() {
        // Unknown implicit curve baseline or synthesized queue capacity. Never
        // guess a descriptor default or partly admit an otherwise invalid plan.
        return if detailed { PARAMETER_CURVE_UNAVAILABLE } else { 1 };
    }
    }
    {
        let callback = &mut *l.callback.get();
        callback.returned.window(callback.position, n);
        let Some(next) = callback.host_call.checked_add(1) else { return 2; };
        callback.host_call = next;
        if let Some(t) = trace.as_deref_mut() {
            t.generation = l.shared.generation;
            t.epoch = callback.epoch;
            t.host_call = callback.host_call;
            t.position = callback.position;
            t.phase_reached |= PHASE_IDENTITY;
        }
    }
    let entered_ns = if entered_ns == 0 { crate::observer::monotonic_ns() } else { entered_ns };
    let mut total = Delivery::default();
    let phase = processing_phase((*l.callback.get()).epoch,
        l.shared.processing_ready_epoch.load(Ordering::Acquire));
    let mut combined = channel_mask(2+extra.len());
    let mut offset = 0;
    let mut timing_requests = 0;
    loop {
        let count = (n - offset).min(if whole_block { CAP } else { LEGACY_CAP });
        let mut item = Item::control(AUDIO, 0);
        item.n = count as u32;
        item.process_mode = mode;
        item.completion = completion;
        item.parent = [(*l.callback.get()).host_call, n as u64, offset as u64, entered_ns];
        item.context = context.chunk(offset).unwrap();
        item.flags = flags;
        item.gain = if offset == 0 { gain } else { f64::NAN };
        if whole_block {
            item.events[..events.len()].copy_from_slice(events);
            item.event_count = events.len() as u32;
        } else {
            let block = &(*l.callback.get()).curve_plan.blocks[offset / LEGACY_CAP];
            item.events[..block.count].copy_from_slice(&block.events[..block.count]);
            item.event_count = block.count as u32;
        }
        for (ch, p) in [left, right].into_iter().enumerate() {
            item.data[ch][..count]
                .copy_from_slice(std::slice::from_raw_parts(p.add(offset), count));
        }
        let mut out = [[0.; CAP]; 2];
        let callback = &mut *l.callback.get();
        match callback.process_outputs_until_traced(&l.shared, item, &mut out,extra,offset,
            completion_deadline,trace.as_deref_mut()) {
            Ok(f) => {
                timing_requests += 1;
                combined &= f;
                for (ch, p) in [out_left, out_right].into_iter().enumerate() {
                    std::ptr::copy_nonoverlapping(out[ch].as_ptr(), p.add(offset), count);
                }
                if let Some(t) = trace.as_deref_mut() {
                    t.presentation_done_ns = crate::observer::monotonic_ns();
                    t.phase_reached |= PHASE_PRESENTATION_DONE;
                    if t.presentation_done_ns != 0 { t.clock_valid |= PHASE_PRESENTATION_DONE; }
                }
                let d = callback.delivery;
                total.missing_frames += d.missing_frames;
                total.gaps += d.gaps;
                total.expired_frames += d.expired_frames;
                total.delivered_frames += d.delivered_frames;
                total.priming_frames += d.priming_frames;
            }
            Err(code) => return if contain_terminal && !exact && l.shared.terminal_latched.load(Ordering::Acquire) {
                CONTAINED_TERMINAL
            } else { code },
        }
        offset += count;
        if offset == n {
            break;
        }
    }
    // The result and local presentation must finish within the original entry
    // allowance. A result racing the timeout cannot authorize late success.
    if completion.is_some_and(|policy| Instant::now() >= policy.deadline) {
        let callback = &*l.callback.get();
        l.shared.fail_deadline(
            callback.position,
            callback.deadline_identity(&l.shared, true),
        );
        return COMPLETION_EXPIRED;
    }
    *out_flags = combined;
    let callback = &mut *l.callback.get();
    callback.windows_timing.host_call = callback.host_call;
    callback.windows_timing.valid = u32::from(exact &&
        callback.windows_timing.requests == timing_requests && callback.windows_timing.valid == 1);
    if !whole_block {
    let callback = &mut *l.callback.get();
    callback.curve_carry.count = callback.curve_plan.next.count;
    callback.curve_carry.events[..callback.curve_carry.count]
        .copy_from_slice(&callback.curve_plan.next.events[..callback.curve_carry.count]);
    callback.curve_continuation = (n > 0).then(|| context.chunk(n).unwrap());
    callback.curve_values.commit(&callback.curve_plan.last);
    }
    for (counter, delta) in l.shared.delivery_totals.iter().zip([
        n as u64, total.missing_frames, total.gaps, total.expired_frames,
        total.delivered_frames, total.priming_frames,
    ]) {
        counter.store(counter.load(Ordering::Relaxed) + delta, Ordering::Release);
    }
    for (counter, delta) in l.shared.delivery_phases[phase].iter().zip([
        n as u64, total.missing_frames, total.gaps, total.expired_frames,
        total.delivered_frames, total.priming_frames,
    ]) {
        counter.store(counter.load(Ordering::Relaxed) + delta, Ordering::Release);
    }
    if !delivery.is_null() {
        *delivery = total;
    }
    let elapsed = callback_entered.elapsed().as_nanos().min(u64::MAX as u128) as u64;
    let callback = &mut *l.callback.get();
    callback.callback_ns_max = callback.callback_ns_max.max(elapsed);
    let bounds = [50, 100, 250, 500, 1000, 2000, 5000, 10000, 25000, u64::MAX];
    let bucket = bounds.iter().position(|limit: &u64| elapsed <= limit.saturating_mul(1000)).unwrap();
    callback.callback_us_buckets[bucket] += 1;
    0
}

#[cfg(feature = "rpi0")]
pub(crate) fn processed_frames(id: u64) -> Option<u64> {
    Some(INSTANCES.lease(id)?.shared.processed_frames.load(Ordering::Acquire))
}

#[repr(C)]
#[derive(Default)]
pub struct Stats {
    pub fault: u64,
    pub first_position: u64,
    pub processed: u64,
    pub request_high: u64,
    pub result_high: u64,
    pub position: u64,
    pub epoch: u64,
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
pub unsafe extern "C" fn ap3_close(id: u64) -> u32 { close_instance(id, false) }
#[no_mangle]
pub unsafe extern "C" fn if2_close(id: u64) -> u32 { close_instance(id, true) }
// Cancellation retains all instance/mapping ownership. The non-RT close owner
// publishes it before waiting for any callback lease, then joins/contains work.
#[no_mangle]
pub extern "C" fn ap23_cancel(id: u64) -> u32 {
    let Some(l) = INSTANCES.lease(id) else { return 1; };
    l.shared.cancel();
    0
}
#[no_mangle]
pub extern "C" fn ap23_deadline_failed(id: u64) -> u32 {
    let Some(l) = INSTANCES.lease(id) else { return 1; };
    let Some(_guard) = Guard::acquire(&l) else { return 3; };
    let callback = unsafe { &*l.callback.get() };
    l.shared.fail_deadline(
        l.shared.worker_position.load(Ordering::Acquire),
        callback.deadline_identity(&l.shared, callback.last_operation_ticket != 0),
    );
    0
}
// Mandatory C ABI v2 final check, after the SDK sinks return. The C++ instance
// guard spans processing and this check; it cannot finish another call's policy.
#[no_mangle]
pub extern "C" fn ap23_finish_callback(id: u64) -> u32 {
    let Some(l) = INSTANCES.lease(id) else { return 1; };
    let Some(_guard) = Guard::acquire(&l) else { return 3; };
    let callback = unsafe { &mut *l.callback.get() };
    let policy = callback.completion_policy.take();
    if l.shared.cancelled.load(Ordering::Acquire) || l.shared.quit.load(Ordering::Acquire) {
        return COMPLETION_CANCELLED;
    }
    if l.shared.fault.load(Ordering::Acquire) != 0 || l.shared.terminal_latched.load(Ordering::Acquire) {
        return 2;
    }
    let Some(policy) = policy else { return 1; };
    if Instant::now() >= policy.deadline {
        l.shared.fail_deadline(callback.position, callback.deadline_identity(&l.shared, true));
        return COMPLETION_EXPIRED;
    }
    0
}
unsafe fn close_instance(id: u64, contain_terminal: bool) -> u32 {
    let cancellation = ap23_cancel(id);
    if cancellation != 0 { return cancellation; }
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
            l.shared.work.notify();
            l.shared.completion.notify();
            let mut joined = true;
            if let Some(t) = l.worker.take() {
                if t.join().is_err() {
                    joined = false;
                    l.shared.fail(WORKER, u64::MAX);
                }
            }
            let contained = contain_terminal && joined && l.shared.terminal_record().is_some()
                && l.shared.retired.load(Ordering::Acquire)
                && !l.shared.pending_control.load(Ordering::Acquire);
            let ok = contained || (clean && joined && l.shared.fault.load(Ordering::Acquire) == 0);
            if let Some(path) = &l.report {
                let callback = l.callback.get_mut();
                callback.gaps.report(path);
                export_refusal(&l.shared,path);
                crate::preview::append_report(path, format!(
                    "{{\"event\":\"native_callback_completion\",\"scope\":\"successful Rust processing calls including completion wait\",\"waits\":{},\"waits_without_result\":{},\"unready_callbacks\":{},\"unready_frames\":{},\"maximum_ns\":{},\"bucket_upper_us\":[50,100,250,500,1000,2000,5000,10000,25000,null],\"buckets\":{:?}}}\n",
                    callback.completion_waits, callback.completion_wait_misses,
                    callback.unready_callbacks, callback.unready_frames,
                    callback.callback_ns_max, callback.callback_us_buckets).as_bytes());
            }
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
        if identity.is_some() { 15 } else { 6 },
        identity,
    )
}
#[no_mangle]
pub unsafe extern "C" fn ap24_open(identity: *const u8, engine: *const u8,
    descriptor: *const u8, handle: *mut u64) -> u32 {
    if identity.is_null() || engine.is_null() || descriptor.is_null() { return 1; }
    let value = std::slice::from_raw_parts(identity, 48);
    open_execution(256, handle, 15, Some(state::Identity {
        class: value[..16].try_into().unwrap(),
        module: value[16..].try_into().unwrap(),
    }), Some(ap1_native_client::admission::ExecutionIdentity {
        engine: std::slice::from_raw_parts(engine, 32).try_into().unwrap(),
        descriptor: std::slice::from_raw_parts(descriptor, 32).try_into().unwrap(),
    }))
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
        false,
        &[],
        None,
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
// Native C ABI extension; no Windows wire or packet layout change.
const CONTAINED_TERMINAL: u32 = 0x106;
// A refused input cannot authorize terminal silence or dead-instance custody.
const PARAMETER_CURVE_UNAVAILABLE: u32 = 0x107;
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
    process_current(id,n,events,count,context,flags,left,right,out_left,out_right,out_flags,delivery,entered_ns,false)
}
#[no_mangle]
pub unsafe extern "C" fn if2_process(
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
    process_current(id,n,events,count,context,flags,left,right,out_left,out_right,out_flags,delivery,entered_ns,true)
}
#[allow(clippy::too_many_arguments)]
unsafe fn process_current(
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
    contain_terminal: bool,
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
        contain_terminal,
        &[],
        None,
    )
}

fn channel_mask(channels: usize) -> u64 { if channels==64 {u64::MAX} else {(1u64<<channels)-1} }
#[no_mangle]
pub unsafe extern "C" fn ap19_process_outputs(
    id:u64,n:u32,events:*const Event,count:u32,context:*const crate::context::Context,
    flags:u64,left:*const f32,right:*const f32,outputs:*const *mut f32,channels:u32,
    out_flags:*mut u64,delivery:*mut Delivery,entered_ns:u64,
)->u32 {
    if count as usize>MAX_EVENTS || (count>0&&events.is_null()) || context.is_null()
        || outputs.is_null() || !(2..=64).contains(&channels) || !channels.is_multiple_of(2) {return 1;}
    let outputs=std::slice::from_raw_parts(outputs,channels as usize);
    let events=if count==0 {&[]} else {std::slice::from_raw_parts(events,count as usize)};
    process_events(id,n,f64::NAN,flags,left,right,outputs[0],outputs[1],out_flags,delivery,
        events,*context,true,entered_ns,true,&outputs[2..],None)
}
#[no_mangle]
pub extern "C" fn ap23_abi_version() -> u32 { 2 }
#[no_mangle]
pub unsafe extern "C" fn ap23_phase_trace_enabled(id: u64, out: *mut u32) -> u32 {
    if out.is_null() { return 0x101; }
    let Some(l) = INSTANCES.lease(id) else { return 1; };
    let Some(_guard) = Guard::acquire(&l) else { return 3; };
    *out = u32::from(l.shared.phase_diagnostics);
    0
}
#[no_mangle]
pub unsafe extern "C" fn ap23_process_outputs(
    id:u64,n:u32,mode:u32,events:*const Event,count:u32,context:*const crate::context::Context,
    flags:u64,left:*const f32,right:*const f32,outputs:*const *mut f32,channels:u32,
    out_flags:*mut u64,delivery:*mut Delivery,entered_ns:u64,
)->u32 {
    if count as usize>MAX_EVENTS || (count>0&&events.is_null()) || context.is_null()
        || outputs.is_null() || !(2..=64).contains(&channels) || !channels.is_multiple_of(2) {
        return 0x101;
    }
    let outputs=std::slice::from_raw_parts(outputs,channels as usize);
    let events=if count==0 {&[]} else {std::slice::from_raw_parts(events,count as usize)};
    process_events(id,n,f64::NAN,flags,left,right,outputs[0],outputs[1],out_flags,delivery,
        events,*context,true,entered_ns,true,&outputs[2..],Some(mode))
}
#[no_mangle]
pub unsafe extern "C" fn ap23_process_windows_timing(
    id: u64,
    out: *mut WindowsProcessTiming,
) -> u32 {
    if out.is_null() || (*out).schema != 1 || (*out).size != 64 {
        return 2;
    }
    let Some(l) = INSTANCES.lease(id) else {
        return 1;
    };
    let Some(_guard) = Guard::acquire(&l) else {
        return 3;
    };
    // The SDK busy guard spans process -> this one-shot copy -> final sinks.
    *out = std::mem::take(&mut (*l.callback.get()).windows_timing);
    0
}
#[no_mangle]
pub unsafe extern "C" fn ap23_process_outputs_trace(
    id:u64,n:u32,mode:u32,events:*const Event,count:u32,context:*const crate::context::Context,
    flags:u64,left:*const f32,right:*const f32,outputs:*const *mut f32,channels:u32,
    out_flags:*mut u64,delivery:*mut Delivery,entered_ns:u64,trace:*mut PhaseTrace,
)->u32 {
    if trace.is_null() || (*trace).schema != PHASE_TRACE_SCHEMA
        || (*trace).size as usize != std::mem::size_of::<PhaseTrace>()
        || count as usize>MAX_EVENTS || (count>0&&events.is_null()) || context.is_null()
        || outputs.is_null() || !(2..=64).contains(&channels) || !channels.is_multiple_of(2) {
        return 0x101;
    }
    let outputs=std::slice::from_raw_parts(outputs,channels as usize);
    let events=if count==0 {&[]} else {std::slice::from_raw_parts(events,count as usize)};
    process_events_traced(id,n,f64::NAN,flags,left,right,outputs[0],outputs[1],out_flags,
        delivery,events,*context,true,entered_ns,true,&outputs[2..],Some(mode),Some(&mut *trace))
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
#[path = "completion_contract_tests.rs"]
mod completion_contract_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os="linux")]
    fn direct_fixture(epoch:u64,channels:usize)->(Callback,Arc<Shared>,crate::direct_audio::Endpoint,
        std::fs::File,std::path::PathBuf) {
        use std::fs::OpenOptions;
        let root=std::env::temp_dir().join(format!("lvb-direct-route-{}-{}",std::process::id(),crate::observer::monotonic_ns()));
        std::fs::create_dir(&root).unwrap();
        let file=OpenOptions::new().read(true).write(true).create_new(true).open(root.join("delivery")).unwrap();
        file.set_len(crate::direct_audio::BYTES as u64).unwrap();
        let channel=crate::direct_audio::Endpoint::prepare(&file).unwrap();
        let mut mapping=ap1_native_client::mapping::Mapping::with_layout(&root.join("samples"),64,true).unwrap();
        mapping.output_channels=channels;
        let render=OpenOptions::new().read(true).write(true).open(root.join("samples")).unwrap();
        let audio=mapping.transfer_audio().unwrap();
        assert!(mapping.transfer_audio().is_err());
        assert!(mapping.write(ap1_native_client::INPUT,&[0]).is_err());
        assert!(mapping.read(ap1_native_client::INPUT,1).is_err());
        let shared=Arc::new(Shared::new());shared.direct.set(channel.clone()).ok().unwrap();
        if channels>2 {shared.extra.set(crate::output_pool::Pool::new(channels-2,2)).ok().unwrap();}
        shared.processing_ready_epoch.store(epoch+1,Ordering::Release);
        let mut callback=Callback::new();callback.prepare(128,channels,0);callback.running=true;callback.epoch=epoch+1;
        callback.direct_audio=Some(Box::new(crate::direct_audio_session::AudioSession::prepare(channel.clone(),audio,[7;16],0,epoch).unwrap()));
        (callback,shared,channel,render,root)
    }
    #[cfg(target_os="linux")]
    fn direct_reply(channel:&crate::direct_audio::Endpoint,mapping:&mut std::fs::File,
        expected:(u64,u64,u64,u32)) {
        use ap1_native_client::{Frame,get,put,INPUT,BLOCK_STRIDE,BLOCK_OUTPUT};
        use std::io::{Seek,SeekFrom,Read,Write};
        let bytes=channel.render_receive(Instant::now()+Duration::from_secs(2)).unwrap();
        let frame=Frame::decode_version(&bytes,15).unwrap();let n=get(&frame.payload[..4]) as usize;
        assert_eq!((get(&frame.payload[32..40]),frame.sequence,get(&frame.payload[40..48]),n as u32),expected);
        for ch in 0..2 {
            let mut bytes=vec![0;n*4];mapping.seek(SeekFrom::Start((INPUT+ch*BLOCK_STRIDE+4) as u64)).unwrap();mapping.read_exact(&mut bytes).unwrap();
            mapping.seek(SeekFrom::Start((BLOCK_OUTPUT+ch*BLOCK_STRIDE+4) as u64)).unwrap();mapping.write_all(&bytes).unwrap();
        }
        let mut payload=vec![0;72];put(&mut payload[..4],n as u64);put(&mut payload[4..8],BLOCK_OUTPUT as u64);
        put(&mut payload[16..24],expected.0);put(&mut payload[24..32],expected.2);put(&mut payload[32..40],123);
        channel.render_reply(&Frame {kind:4,session:[7;16],sequence:frame.sequence,payload}.encode_version(15).unwrap());
    }
    #[cfg(target_os="linux")]
    #[test]
    fn direct_callback_completes_while_control_store_is_stalled_and_reprepares_epoch() {
        for epoch in [0,1] {
            let (mut callback,shared,channel,mut mapping,root)=direct_fixture(epoch,2);
            // These exact control locks are deliberately held throughout all AUDIO calls.
            let control=shared.control.lock().unwrap();let snapshots=shared.snapshots.lock().unwrap();
            shared.pending_control.store(true,Ordering::Release);
            let render=thread::spawn(move||for (ticket,position,n) in [(1,0,64),(2,64,0),(3,64,128)] {
                direct_reply(&channel,&mut mapping,(epoch+1,ticket,position,n));
            });
            for n in [64,0,128] {
                let mut item=Item::control(AUDIO,epoch+1);item.n=n;item.gain=f64::NAN;
                if n==128 {item.event_count=1;item.events[0]=ap1_native_client::events::Event {kind:2,value:0.5,..Default::default()};}
                item.data[0][..n as usize].fill(0.25);item.data[1][..n as usize].fill(-0.5);
                item.completion=Some(crate::performance::CompletionPolicy {allowance:Duration::from_secs(2),deadline:Instant::now()+Duration::from_secs(2),exact:true});
                let mut out=[[0.;CAP];2];let end=item.completion.unwrap().deadline;
                let (result,allocations)=crate::allocation_test::measure(||callback.process_outputs_until(&shared,item,&mut out,&[],0,Some(end)));
                assert_eq!(result,Ok(if n==0 {3} else {0}));assert_eq!(allocations,[0;3]);
                assert_eq!(&out[0][..n as usize],&item.data[0][..n as usize]);
                assert_eq!(&out[1][..n as usize],&item.data[1][..n as usize]);
            }
            assert_eq!(shared.requests.published(),0);assert_eq!(shared.processed.load(Ordering::Acquire),3);
            assert_eq!(shared.direct_submitted.load(Ordering::Acquire),3);assert_eq!(shared.direct_completed.load(Ordering::Acquire),3);
            assert_eq!(shared.direct_progress.read(),Some([epoch+1,3,64,192]));
            render.join().unwrap();drop(control);drop(snapshots);
            let mut capture=Control {barrier:0,direct_barrier:Some(1),op:16,bytes:vec![],result:None};
            complete_control(&shared,&mut capture,Ok(vec![9,7]),([1,epoch+1,3,64,11],Some(192))).unwrap();
            assert_eq!(shared.snapshots.lock().unwrap().latest().unwrap().through,1);
            assert_eq!(shared.last_edit.load(Ordering::Acquire),3);
            assert!(shared.last_edit.load(Ordering::Acquire)>shared.snapshots.lock().unwrap().latest().unwrap().through,
                "later direct parameter edit cannot reuse an older admitted capture");
            drop(callback);
            std::fs::remove_dir_all(root).unwrap();
        }
    }
    #[cfg(target_os="linux")]
    #[test]
    fn direct_peer_fin_with_unread_state_wakes_callback_while_store_is_stalled() {
        use std::io::Write;
        let (mut callback,shared,channel,_mapping,root)=direct_fixture(0,2);
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut peer=std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (socket,_)=listener.accept().unwrap();let watch=DeathWatch::start(socket,shared.clone()).unwrap();
        let snapshots=shared.snapshots.lock().unwrap();let control=shared.control.lock().unwrap();
        let (published,wait)=std::sync::mpsc::channel();
        let render=thread::spawn(move||{
            channel.render_receive(Instant::now()+Duration::from_secs(2)).unwrap();published.send(()).unwrap();
        });
        let callback_shared=shared.clone();
        let callback=thread::spawn(move||{
            let mut item=Item::control(AUDIO,1);item.n=64;item.gain=f64::NAN;
            item.completion=Some(crate::performance::CompletionPolicy {allowance:Duration::from_secs(5),deadline:Instant::now()+Duration::from_secs(5),exact:true});
            callback.process_outputs_until(&callback_shared,item,&mut [[0.;CAP];2],&[],0,Some(item.completion.unwrap().deadline))
        });
        wait.recv_timeout(Duration::from_secs(2)).unwrap();thread::sleep(Duration::from_millis(20));
        let begin=Instant::now();peer.write_all(b"unread state bytes").unwrap();peer.shutdown(std::net::Shutdown::Write).unwrap();
        assert_eq!(callback.join().unwrap(),Err(COMPLETION_CANCELLED));assert!(begin.elapsed()<Duration::from_millis(200));
        assert_eq!(shared.fault.load(Ordering::Acquire),WORKER);
        render.join().unwrap();drop(watch);drop(snapshots);drop(control);std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(target_os="linux")]
    #[test]
    fn direct_zero_calls_do_not_consume_extra_slots_and_cancelled_local_slot_releases() {
        let (mut callback,shared,channel,mut mapping,root)=direct_fixture(0,4);
        let render=thread::spawn(move||for ticket in 1..=16 {direct_reply(&channel,&mut mapping,(1,ticket,0,0));});
        for _ in 0..16 {
            let mut item=Item::control(AUDIO,1);item.gain=f64::NAN;
            item.completion=Some(crate::performance::CompletionPolicy {allowance:Duration::from_secs(2),deadline:Instant::now()+Duration::from_secs(2),exact:true});
            let (result,allocations)=crate::allocation_test::measure(||callback.process_outputs_until(&shared,item,&mut [[0.;CAP];2],&[std::ptr::null_mut();2],0,Some(item.completion.unwrap().deadline)));
            assert_eq!(result,Ok(15));assert_eq!(allocations,[0;3]);
        }
        render.join().unwrap();let pool=shared.extra.get().unwrap();let planes=[[0.;CAP];2];
        assert!(pool.publish(0,&planes,1));assert!(pool.publish(1,&planes,1));pool.release(0);pool.release(1);
        {let _guard=DirectOutputGuard {shared:&shared,slot:pool.publish_available(&planes,1).unwrap()};}
        assert!(pool.publish(0,&planes,1));pool.release(0);drop(callback);std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn pending_start_control_barrier_preserves_publication_order_across_pop() {
        let shared = Shared::new();
        assert!(shared.requests.push(Item::control(START, 1)));
        assert_eq!(shared.requests.pop().unwrap().kind, START);
        assert!(!admitted_control_precedes(&shared, false).unwrap());

        *shared.control.lock().unwrap() = Some(Control {
            barrier: shared.requests.published(),
            #[cfg(target_os="linux")]
            direct_barrier:None,
            op: 16,
            bytes: vec![],
            result: None,
        });
        shared.pending_control.store(true, Ordering::Release);
        assert!(admitted_control_precedes(&shared, false).unwrap());

        assert!(shared.requests.push(Item::control(AUDIO, 1)));
        assert_eq!(shared.requests.pop().unwrap().kind, AUDIO);
        assert!(admitted_control_precedes(&shared, true).unwrap(),
            "control admitted before AUDIO must survive the pop race");

        shared.control.lock().unwrap().as_mut().unwrap().barrier =
            shared.requests.published();
        assert!(!admitted_control_precedes(&shared, true).unwrap(),
            "control whose barrier includes AUDIO must remain after it");
    }
    #[test]
    fn pending_start_cancellation_included_in_snapshot_is_rechecked_before_wait() {
        let shared = Shared::new();
        assert!(!pending_start_cancelled(&shared));
        shared.cancelled.store(true, Ordering::Release);
        shared.work.notify();
        let observed = shared.work.snapshot();
        assert_eq!(pending_start_wait_after_snapshot(&shared, observed)
            .unwrap_err().to_string(), "pending Start cancelled");
    }
    #[test]
    fn buffered_work_keeps_its_originating_absolute_containment_bound() {
        let now = Instant::now();
        let local_deadline = now - Duration::from_nanos(1);
        let mut item = Item::control(AUDIO, 1);
        item.n = 128;
        item.completion = Some(crate::performance::CompletionPolicy {
            allowance: Duration::from_nanos(128),
            deadline: local_deadline,
            exact: false,
        });
        let transport = audio_transport_deadline(&item, now);
        assert_eq!(transport, local_deadline);
        let pending = crate::PendingStart {
            session: [0; 16],
            sequence: 1,
            epoch: 1,
            receive_deadline: now + Duration::from_secs(10),
        }
        .bounded_by(transport);
        assert_eq!(pending.receive_deadline, local_deadline);
    }
    fn control_live(shared: Arc<Shared>) -> u64 {
        INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared, callback: UnsafeCell::new(Callback::new()), busy: AtomicBool::new(false),
            worker: None, report: None, max: 256, recovery_blocked: false,
            installed_delay: Some(256),
            delivery_mode: crate::performance::DeliveryMode::Buffered,
            minor: 15, setup: None,
        })).unwrap().unwrap()
    }
    fn await_control_wait(shared: &Shared) {
        let until = Instant::now() + Duration::from_secs(3);
        while shared.control_waits.load(Ordering::Acquire) == 0 {
            assert!(Instant::now() < until, "control caller did not enter its acknowledgement wait");
            thread::yield_now();
        }
    }
    fn prepared_live(shared: Arc<Shared>, maximum: u32, mode: u32,
        delivery: crate::performance::DeliveryMode) -> u64 {
        let mut callback = Callback::new();
        callback.prepare(maximum as usize, shared.extra.get().map_or(2, |pool| pool.channels + 2),
            delivery.effective_delay(maximum, 256).unwrap() as usize);
        assert_eq!(callback.transition(&shared, START), 0);
        shared.requests.pop().unwrap();
        // This helper stands in for the peer: it has taken the start, so it
        // acknowledges it as the transport worker would.
        shared.processing_ready_epoch.store(callback.epoch, Ordering::Release);
        INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared, callback: UnsafeCell::new(callback), busy: AtomicBool::new(false),
            worker: None, report: None, max: maximum as usize, recovery_blocked: false,
            installed_delay: Some(256), delivery_mode: delivery, minor: 15,
            setup: Some(crate::performance::wire_version(maximum, mode, 48000., true).unwrap()),
        })).unwrap().unwrap()
    }
    #[test]
    fn current_control_acknowledgement_delivers_a_result_across_racing_wait_entry() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = control_live(shared.clone());
        let caller = thread::spawn(move || unsafe { control(id, 20, vec![1, 2, 3]) });
        await_control_wait(&shared);
        // The counter establishes arrival at the wait boundary, not which side
        // of wait entry wins this race. Signal's deterministic
        // publication_between_inspection_and_sleep_cannot_lose_wake test
        // separately forces publication before entering the primitive wait.
        let started = Instant::now();
        {
            let mut mailbox = shared.control.lock().unwrap();
            let pending = mailbox.as_mut().expect("published control predicate");
            assert_eq!((pending.op, pending.bytes.as_slice()), (20, [1, 2, 3].as_slice()));
            complete_control(&shared, pending, Ok(vec![4, 5, 6]), ([0; 5],None)).unwrap();
        }
        assert_eq!(caller.join().unwrap().unwrap(), vec![4, 5, 6]);
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(!shared.pending_control.load(Ordering::Acquire));
        assert!(shared.control.lock().unwrap().is_none());
        INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn current_control_wait_is_woken_by_cancellation_without_fabricating_an_acknowledgement() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = control_live(shared.clone());
        let caller = thread::spawn(move || unsafe { control(id, 16, vec![]) });
        await_control_wait(&shared);
        let started = Instant::now();
        shared.cancel();
        let error = caller.join().unwrap().unwrap_err();
        assert_eq!(error.to_string(), "state acknowledgement cancelled; no retry");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(shared.fault.load(Ordering::Acquire), 0);
        assert!(shared.pending_control.load(Ordering::Acquire));
        let mailbox = shared.control.lock().unwrap();
        assert!(mailbox.as_ref().is_some_and(|pending| pending.result.is_none()));
        drop(mailbox);
        INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn current_control_wait_is_woken_by_fault_without_replacing_the_first_failure() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = control_live(shared.clone());
        let caller = thread::spawn(move || unsafe { control(id, 8, vec![0; 8]) });
        await_control_wait(&shared);
        let started = Instant::now();
        shared.fail(CORRELATION, 99);
        let error = caller.join().unwrap().unwrap_err();
        assert_eq!(error.to_string(), "state acknowledgement missing; instance failed, no retry");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(shared.fault.load(Ordering::Acquire), CORRELATION);
        assert_eq!(shared.first_position.load(Ordering::Acquire), 99);
        assert!(shared.pending_control.load(Ordering::Acquire));
        assert!(shared.control.lock().unwrap().as_ref().is_some_and(|pending| pending.result.is_none()));
        INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn consecutive_zero_frame_operations_return_their_exact_results_without_advancing_audio() {
        let _registry_owner = crate::registry_test();
        for delivery in [crate::performance::DeliveryMode::Buffered,
            crate::performance::DeliveryMode::SameCallback] {
            let mut shared = Shared::new();
            shared.identity = Some(state::Identity { class: [1; 16], module: [2; 32] });
            let shared = Arc::new(shared);
            shared.state_capable.store(true, Ordering::Release);
            let id = prepared_live(shared.clone(), 256, 0, delivery);
            for (ticket, mode) in [(1, 0), (2, 1), (3, 0)] {
                // The source seam supplies an already completed ordered result;
                // the installed SDK consumer qualifies transport timing.
                let mut item = Item::control(AUDIO, 1);
                item.ticket = ticket;
                item.process_mode = mode;
                let mut completion = Completion::from(item);
                completion.returned.events = 1;
                completion.returned.event[0] = crate::process_results::Event {
                    kind: 65535, a: 64, c: ticket as i32, ..Default::default()
                };
                completion.returned.points = 1;
                completion.returned.point[0] = crate::process_results::Point {
                    offset: 0, id: 17, value: ticket as f64 / 4.,
                };
                assert!(shared.publish_result(completion).is_some());
                let mut dummy = 0.; let mut flags = 0;
                let planes = [std::ptr::addr_of_mut!(dummy); 2];
                let (result, allocations) = crate::allocation_test::measure(|| unsafe {
                    ap23_process_outputs(id, 0, mode, std::ptr::null(), 0,
                        &crate::context::Context::default(), 0, &dummy, &dummy, planes.as_ptr(),
                        2, &mut flags, std::ptr::null_mut(), 0)
                });
                assert_eq!(result, 0); assert_eq!(allocations, [0; 3]);
                let request = shared.requests.pop().unwrap();
                assert_eq!((request.ticket, request.position, request.n, request.process_mode),
                    (ticket, 0, 0, mode));
                let mut packet = crate::process_results::Packet::default();
                unsafe { assert_eq!(ap10_take_results(id, &mut packet), 0); }
                assert_eq!((packet.events, packet.points), (1, 1));
                assert_eq!((packet.event[0].offset, packet.event[0].c), (0, ticket as i32));
                assert_eq!((packet.point[0].offset, packet.point[0].value), (0, ticket as f64 / 4.));
                unsafe { assert_eq!(ap10_take_results(id, &mut packet), 0); }
                assert_eq!((packet.events, packet.points), (0, 0));
            }
            INSTANCES.remove(id, |_| ()).unwrap();
        }
    }
    #[test]
    fn legal_start_immediately_precedes_zero_frame_audio_without_readiness_pacing() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let mut callback = Callback::new();
        callback.prepare(256, 2, 0);
        assert_eq!(callback.transition(&shared, START), 0);
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared: shared.clone(), callback: UnsafeCell::new(callback),
            busy: AtomicBool::new(false), worker: None, report: None, max: 256,
            recovery_blocked: false, installed_delay: Some(256),
            delivery_mode: crate::performance::DeliveryMode::SameCallback, minor: 15,
            setup: Some(crate::performance::wire_version(256, 1, 48000., true).unwrap()),
        })).unwrap().unwrap();

        // Supply the exact completion at the source seam so this test controls
        // lifecycle/request ordering without turning worker timing into a pace.
        // The START request remains queued and no processing-ready observation
        // is published before the legal first callback.
        let mut completed = Item::control(AUDIO, 1);
        completed.ticket = 1;
        completed.process_mode = 1;
        let mut completion = Completion::from(completed);
        completion.returned.events = 1;
        completion.returned.event[0] = crate::process_results::Event {
            kind: 65535, a: 64, c: 71, ..Default::default()
        };
        completion.returned.points = 1;
        completion.returned.point[0] = crate::process_results::Point {
            offset: 0, id: 31, value: 0.25,
        };
        assert!(shared.publish_result(completion).is_some());
        assert_eq!(shared.processing_ready_epoch.load(Ordering::Acquire), 0);

        let mut sample = 0.;
        let planes = [std::ptr::addr_of_mut!(sample); 2];
        let mut flags = u64::MAX;
        let (result, allocations) = crate::allocation_test::measure(|| unsafe {
            ap23_process_outputs(id, 0, 1, std::ptr::null(), 0,
                &crate::context::Context::default(), 0, &sample, &sample,
                planes.as_ptr(), 2, &mut flags, std::ptr::null_mut(), 0)
        });
        assert_eq!(result, 0);
        assert_eq!(allocations, [0; 3]);
        assert_eq!(flags, 3);
        assert_eq!(shared.processing_ready_epoch.load(Ordering::Acquire), 0);

        let start = shared.requests.pop().unwrap();
        let audio = shared.requests.pop().unwrap();
        assert_eq!((start.kind, start.epoch), (START, 1));
        assert_eq!((audio.kind, audio.epoch, audio.ticket, audio.position, audio.n,
            audio.process_mode, audio.parent[0]), (AUDIO, 1, 1, 0, 0, 1, 1));
        assert!(shared.requests.pop().is_none());
        let mut packet = crate::process_results::Packet::default();
        unsafe { assert_eq!(ap10_take_results(id, &mut packet), 0); }
        assert_eq!((packet.events, packet.points), (1, 1));
        assert_eq!((packet.event[0].offset, packet.event[0].c), (0, 71));
        assert_eq!((packet.point[0].offset, packet.point[0].id, packet.point[0].value),
            (0, 31, 0.25));
        INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn zero_frame_buffered_timeout_is_explicit_and_never_left_for_the_next_callback() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let mut callback = Callback::new(); callback.prepare(256, 2, 256);
        callback.running = true; callback.epoch = 1;
        let mut request = Item::control(AUDIO, 1);
        let now = Instant::now();
        let mut policy = crate::performance::CompletionPolicy::new(0,
            crate::performance::DeliveryMode::Buffered, 0, 0, 0, now);
        // Inject an expired absolute bound; production uses the full 5s ceiling.
        policy.deadline = now - Duration::from_nanos(1); request.completion = Some(policy);
        let (result, allocations) = crate::allocation_test::measure(||
            callback.process_outputs_until(&shared, request, &mut [[0.; CAP]; 2], &[], 0,
                Some(policy.deadline)));
        assert!(shared.requests.pop().is_none(), "expired call cannot admit new work");
        assert_eq!(result, Err(COMPLETION_EXPIRED)); assert_eq!(allocations, [0; 3]);
        assert_eq!(shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
    }
    #[test]
    fn final_sdk_check_consumes_the_same_queued_policy_and_refuses_stale_calls() {
        let _registry_owner = crate::registry_test();
        assert_eq!(ap23_abi_version(), 2);
        for mode in [crate::performance::DeliveryMode::SameCallback,
            crate::performance::DeliveryMode::Buffered] {
            let shared = Arc::new(Shared::new());
            shared.state_capable.store(true, Ordering::Release);
            let id = prepared_live(shared.clone(), 256, 0, mode);
            assert_eq!(ap23_finish_callback(id), 1, "no originating call exists");
            let mut result = Item::control(AUDIO, 1); result.ticket = 1;
            assert!(shared.publish_result(Completion::from(result)).is_some());
            let mut dummy = 0.; let planes = [std::ptr::addr_of_mut!(dummy); 2]; let mut flags = 9;
            unsafe { assert_eq!(ap23_process_outputs(id, 0, 0, std::ptr::null(), 0,
                &crate::context::Context::default(), 0, &dummy, &dummy, planes.as_ptr(),
                2, &mut flags, std::ptr::null_mut(), 0), 0); }
            let request = shared.requests.pop().unwrap();
            let l = INSTANCES.lease(id).unwrap();
            let callback = unsafe { &*l.callback.get() };
            assert_eq!(callback.completion_policy.unwrap().deadline,
                request.completion.unwrap().deadline);
            drop(l);
            let (finish, allocations) = crate::allocation_test::measure(|| ap23_finish_callback(id));
            assert_eq!(finish, 0); assert_eq!(allocations, [0; 3]);
            assert_eq!(ap23_finish_callback(id), 1, "success cannot reuse consumed policy");
            unsafe { assert_eq!(ap23_process_outputs(id, 257, 0, std::ptr::null(), 0,
                &crate::context::Context::default(), 0, &dummy, &dummy, planes.as_ptr(),
                2, &mut flags, std::ptr::null_mut(), 0), 0x101); }
            assert_eq!(ap23_finish_callback(id), 1, "failed new call clears old policy");
            INSTANCES.remove(id, |_| ()).unwrap();
        }
    }
    #[test]
    fn buffered_final_sdk_sink_crossing_original_five_second_bound_refuses_success() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new()); shared.state_capable.store(true, Ordering::Release);
        let id = prepared_live(shared.clone(), 256, 0, crate::performance::DeliveryMode::Buffered);
        let input = [0.; 64]; let mut output = [[0.; 64]; 2];
        let planes = [output[0].as_mut_ptr(), output[1].as_mut_ptr()]; let mut flags = 9;
        unsafe { assert_eq!(ap23_process_outputs(id, 64, 0, std::ptr::null(), 0,
            &crate::context::Context::default(), 3, input.as_ptr(), input.as_ptr(),
            planes.as_ptr(), 2, &mut flags, std::ptr::null_mut(), 0), 0); }
        let request = shared.requests.pop().unwrap(); let policy = request.completion.unwrap();
        assert_eq!(policy.allowance, crate::performance::AUDIO_CONTAINMENT);
        // Model a host SDK sink that returns after the real original bound.
        // The bridge cannot preempt that sink, but must refuse subsequent success.
        thread::sleep(policy.deadline.saturating_duration_since(Instant::now())
            + Duration::from_millis(1));
        let (finish, allocations) = crate::allocation_test::measure(|| ap23_finish_callback(id));
        assert_eq!(finish, COMPLETION_EXPIRED); assert_eq!(allocations, [0; 3]);
        assert_eq!(shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
        assert_eq!(shared.first_requests[0].load(Ordering::Acquire), 2, "START and AUDIO were published");
        assert_eq!(ap23_finish_callback(id), 2, "first expiry remains primary");
        INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn actual_mode_switches_are_carried_and_offline_crossing_is_refused_before_admission() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = prepared_live(shared.clone(), 256, 1, crate::performance::DeliveryMode::Buffered);
        let input = [0.; 64]; let mut out = [[9.; 64]; 2];
        let planes = [out[0].as_mut_ptr(), out[1].as_mut_ptr()]; let mut flags = 0;
        for mode in [1, 0, 1] {
            let result = unsafe { ap23_process_outputs(id, 64, mode, std::ptr::null(), 0,
                &crate::context::Context::default(), 3, input.as_ptr(), input.as_ptr(),
                planes.as_ptr(), 2, &mut flags, std::ptr::null_mut(), 0) };
            assert_eq!(result, 0);
            let request = shared.requests.pop().unwrap(); assert_eq!(request.process_mode, mode);
        }
        let published = shared.requests.published();
        let result = unsafe { ap23_process_outputs(id, 64, 2, std::ptr::null(), 0,
            &crate::context::Context::default(), 3, input.as_ptr(), input.as_ptr(),
            planes.as_ptr(), 2, &mut flags, std::ptr::null_mut(), 0) };
        assert_eq!(result, 0x10A); assert_eq!(shared.requests.published(), published);
        let live = INSTANCES.lease(id).unwrap();
        assert_eq!(unsafe { (*live.callback.get()).position }, 192);
        drop(live); INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn same_callback_offline_covers_every_actual_length_in_place_and_multiple_outputs() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        shared.extra.set(crate::output_pool::Pool::new(2, DESCRIPTORS)).ok().unwrap();
        let id = prepared_live(shared.clone(), 1024, 2, crate::performance::DeliveryMode::SameCallback);
        let pool = shared.extra.get().unwrap();
        let mut position = 0u64;
        for n in 0..=1024 {
            let mut input = [[12345.; CAP]; 2]; let mut extra = [[12345.; CAP]; 2];
            let mut expected_extra = [[0.; CAP]; 2];
            let mut item = Item::control(AUDIO, 1);
            item.ticket = n as u64 + 1; item.n = n as u32; item.position = position;
            for i in 0..n {
                let value = ((position + i as u64) % 997) as f32 / 1000.;
                for ch in 0..2 {
                    input[ch][i] = value;
                    item.data[ch][i] = value * (ch as f32 + 1.);
                    expected_extra[ch][i] = value * (ch as f32 + 3.);
                }
            }
            let mut completion = Completion::from(item);
            completion.audio.flags = 0;
            if n > 0 {
                let slot = shared.results.published() as usize % DESCRIPTORS;
                assert!(pool.publish(slot, &expected_extra, n)); completion.audio.extra_slot = slot;
            }
            assert!(shared.publish_result(completion).is_some());
            let planes = [input[0].as_mut_ptr(), input[1].as_mut_ptr(),
                extra[0].as_mut_ptr(), extra[1].as_mut_ptr()];
            let mut flags = 0; let mut delivery = Delivery::default();
            let (result, allocations) = crate::allocation_test::measure(|| unsafe {
                ap23_process_outputs(id, n as u32, 2, std::ptr::null(), 0,
                    &crate::context::Context::default(), 0, input[0].as_ptr(), input[1].as_ptr(),
                    planes.as_ptr(), 4, &mut flags, &mut delivery, 0)
            });
            assert_eq!(result, 0); assert_eq!(allocations, [0; 3]);
            assert_eq!((delivery.delivered_frames, delivery.missing_frames,
                delivery.priming_frames, delivery.expired_frames), (n as u64, 0, 0, 0));
            for ch in 0..2 {
                assert_eq!(&input[ch][..n], &item.data[ch][..n]);
                assert_eq!(&extra[ch][..n], &expected_extra[ch][..n]);
                assert!(input[ch][n..].iter().chain(&extra[ch][n..]).all(|&v| v == 12345.));
            }
            let request = shared.requests.pop().unwrap();
            assert_eq!((request.ticket, request.position, request.n), (item.ticket, position, n as u32));
            position += n as u64;
        }
        let live = INSTANCES.lease(id).unwrap();
        assert_eq!(unsafe { (*live.callback.get()).position }, position);
        assert_eq!(unsafe { (*live.callback.get()).audio.len() }, 0);
        drop(live); INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn buffered_offline_one_frame_calls_preserve_all_outputs_results_and_the_host_drain() {
        buffered_one_frame_oracle(true);
    }
    #[test]
    fn buffered_offline_one_frame_calls_preserve_audio_at_every_selected_delay() {
        buffered_one_frame_oracle(false);
    }
    #[test]
    fn buffered_offline_one_frame_primary_audio_survives_the_maximum_delay() {
        let shared = Shared::new(); let mut callback = Callback::new();
        assert_eq!(callback.transition(&shared, START), 0); shared.requests.pop().unwrap();
        let mut out = [[0.; CAP]; 2];
        for position in 0..(DELAY as usize * 2 + 17) {
            let mut request = Item::control(AUDIO, 1); request.n = 1;
            request.position = position as u64; request.ticket = position as u64 + 1;
            let end = Instant::now() + Duration::from_secs(60);
            request.completion = Some(crate::performance::CompletionPolicy {
                allowance: Duration::from_millis(50), deadline: end, exact: true });
            let mut completion = Completion::from(request);
            completion.audio.data[0][0] = position as f32;
            completion.audio.data[1][0] = position as f32 + 0.5;
            assert!(shared.publish_result(completion).is_some());
            let (result, allocations) = crate::allocation_test::measure(||
                callback.process_outputs_until(&shared, request, &mut out, &[], 0, Some(end)));
            assert_eq!(result, Ok(if position < DELAY as usize { 3 } else { 0 }), "D=1024, frame {position}"); assert_eq!(allocations, [0; 3]);
            shared.requests.pop().unwrap();
            if position >= DELAY as usize {
                assert_eq!(out[0][0], (position - DELAY as usize) as f32);
                assert_eq!(out[1][0], (position - DELAY as usize) as f32 + 0.5);
            }
        }
    }
    fn buffered_one_frame_oracle(dense: bool) {
        let _registry_owner = crate::registry_test();
        for delay in [256usize, 512, 1024] {
            let events = if dense { 64usize } else { 1 };
            let points = if dense { 128usize } else { 1 };
            let shared = Arc::new(Shared::new());
            shared.state_capable.store(true, Ordering::Release);
            shared.extra.set(crate::output_pool::Pool::new(2, DESCRIPTORS)).ok().unwrap();
            let mut callback = Callback::new(); callback.prepare(delay, 4, delay);
            assert_eq!(callback.transition(&shared, START), 0);
            shared.requests.pop().unwrap();
            let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
                shared: shared.clone(), callback: UnsafeCell::new(callback),
                busy: AtomicBool::new(false), worker: None, report: None,
                max: delay, recovery_blocked: false, installed_delay: Some(delay as u32),
                delivery_mode: crate::performance::DeliveryMode::Buffered, minor: 15,
                setup: Some(crate::performance::wire_version(delay as u32, 2, 48000., true).unwrap()),
            })).unwrap().unwrap();
            let program = delay + 17;
            let mut ticket = 0u64;
            let mut out = [[12345.; CAP]; 4]; let mut input = [0.; CAP];
            for position in 0..program + delay {
                if position == delay / 2 {
                    ticket += 1;
                    let mut flush = Completion::from(Item::control(AUDIO, 1));
                    flush.audio.position = position as u64; flush.ticket = ticket;
                    flush.returned.events = 1; flush.returned.points = 1;
                    flush.returned.event[0] = crate::process_results::Event {
                        kind: 65535, a: 74, c: 127, ..Default::default()
                    };
                    flush.returned.point[0] = crate::process_results::Point {
                        id: 999, value: 0.25, ..Default::default()
                    };
                    assert!(shared.publish_result(flush).is_some());
                    let planes = out.each_mut().map(|p| p.as_mut_ptr());
                    let mut flags = 0;
                    let (result, allocations) = crate::allocation_test::measure(|| unsafe {
                        ap23_process_outputs(id, 0, 2, std::ptr::null(), 0,
                            &crate::context::Context::default(), 0, input.as_ptr(), input.as_ptr(),
                            planes.as_ptr(), 4, &mut flags, std::ptr::null_mut(), 0)
                    });
                    assert_eq!(result, 0, "D={delay} zero-frame call"); assert_eq!(allocations, [0; 3]);
                    assert_eq!(shared.requests.pop().unwrap().ticket, ticket);
                    let mut packet = crate::process_results::Packet::default();
                    unsafe { assert_eq!(ap10_take_results(id, &mut packet), 0); }
                    assert_eq!((packet.events, packet.points), (1, 1));
                    assert_eq!((packet.event[0].c, packet.point[0].id), (127, 999));
                }
                ticket += 1;
                let mut item = Item::control(AUDIO, 1);
                item.n = 1; item.position = position as u64; item.ticket = ticket;
                let active = position < program;
                item.flags = if !active { 15 } else if position % 2 == 0 { 10 } else { 5 };
                let sample = |channel: usize, at: usize| {
                    if at >= program || (at + channel) % 2 == 1 { 0. }
                    else { (at as f32 + 1.) * (channel as f32 + 1.) / 4096. }
                };
                for ch in 0..2 { item.data[ch][0] = sample(ch, position); }
                let mut completion = Completion::from(item);
                let mut extra = [[0.; CAP]; 2];
                for (ch, plane) in extra.iter_mut().enumerate() { plane[0] = sample(ch + 2, position); }
                let slot = shared.results.published() as usize % DESCRIPTORS;
                assert!(shared.extra.get().unwrap().publish(slot, &extra, 1), "D={delay} slot at {position}");
                completion.audio.extra_slot = slot;
                if active {
                    completion.returned.events = events as u32;
                    completion.returned.points = points as u32;
                    completion.returned.bytes = (events * 64) as u32;
                    for e in 0..events {
                        completion.returned.event[e] = crate::process_results::Event {
                            kind: 2, payload_offset: (e * 64) as u32, payload_size: 64,
                            ..Default::default()
                        };
                        completion.returned.payload[e * 64..(e + 1) * 64].fill(position as u8);
                        completion.returned.payload[e * 64 + 1] = e as u8;
                    }
                    for p in 0..points {
                        completion.returned.point[p] = crate::process_results::Point {
                            id: p as u32, value: position as f64 / program as f64,
                            ..Default::default()
                        };
                    }
                }
                assert!(shared.publish_result(completion).is_some());
                input[0] = position as f32; let planes = out.each_mut().map(|p| p.as_mut_ptr());
                let mut flags = 99; let mut delivery = Delivery::default();
                let (result, allocations) = crate::allocation_test::measure(|| unsafe {
                    ap23_process_outputs(id, 1, 2, std::ptr::null(), 0,
                        &crate::context::Context::default(), 0, input.as_ptr(), input.as_ptr(),
                        planes.as_ptr(), 4, &mut flags, &mut delivery, 0)
                });
                assert_eq!(result, 0, "D={delay}, frame {position}"); assert_eq!(allocations, [0; 3]);
                assert_eq!(shared.requests.pop().unwrap().ticket, ticket);
                assert_eq!((delivery.missing_frames, delivery.expired_frames), (0, 0));
                for (ch, plane) in out.iter().enumerate() {
                    let expected = if position < delay { 0. } else { sample(ch, position - delay) };
                    assert_eq!(plane[0], expected, "D={delay}, frame {position}, channel {ch}");
                    assert!(plane[1..].iter().all(|&v| v == 12345.));
                }
                let mut packet = crate::process_results::Packet::default();
                let (result, allocations) = crate::allocation_test::measure(|| unsafe { ap10_take_results(id, &mut packet) });
                assert_eq!(result, 0); assert_eq!(allocations, [0; 3]);
                let due = position >= delay && position - delay < program;
                assert_eq!((packet.events, packet.points), if due { (events as u32, points as u32) } else { (0, 0) });
                if due {
                    let source = position - delay;
                    assert_eq!(flags, if source % 2 == 0 { 10 } else { 5 });
                    for e in 0..events {
                        assert_eq!((packet.event[e].offset, packet.event[e].payload_size), (0, 64));
                        let offset = packet.event[e].payload_offset as usize;
                        assert_eq!(&packet.payload[offset..offset + 2], &[source as u8, e as u8]);
                        assert!(packet.payload[offset + 2..offset + 64].iter().all(|&b| b == source as u8));
                    }
                    for p in 0..points {
                        assert_eq!((packet.point[p].offset, packet.point[p].id), (0, p as u32));
                        assert_eq!(packet.point[p].value, source as f64 / program as f64);
                    }
                } else { assert_eq!(flags, 15); }
                unsafe { assert_eq!(ap10_take_results(id, &mut packet), 0); }
                assert_eq!((packet.events, packet.points), (0, 0));
            }
            assert_eq!(shared.fault.load(Ordering::Acquire), 0);
            INSTANCES.remove(id, |_| ()).unwrap();
        }
    }
    #[test]
    fn buffered_offline_every_actual_length_preserves_the_absolute_audio_timeline() {
        let _registry_owner = crate::registry_test();
        for maximum in [256usize, 512, 1024] {
            let shared = Arc::new(Shared::new());
            shared.state_capable.store(true, Ordering::Release);
            shared.extra.set(crate::output_pool::Pool::new(2, DESCRIPTORS + 1)).ok().unwrap();
            let mut callback = Callback::new(); callback.prepare(maximum, 4, maximum);
            assert_eq!(callback.transition(&shared, START), 0); shared.requests.pop().unwrap();
            let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
                shared: shared.clone(), callback: UnsafeCell::new(callback),
                busy: AtomicBool::new(false), worker: None, report: None,
                max: maximum, recovery_blocked: false, installed_delay: Some(maximum as u32),
                delivery_mode: crate::performance::DeliveryMode::Buffered, minor: 15,
                setup: Some(crate::performance::wire_version(maximum as u32, 2, 48000., true).unwrap()),
            })).unwrap().unwrap();
            let total = maximum * (maximum + 1) / 2 + maximum;
            let mut history: [Vec<f32>; 4] = std::array::from_fn(|_| Vec::with_capacity(total));
            let mut history_flags = Vec::with_capacity(total);
            let mut out = [[12345.; CAP]; 4]; let input = [0.; CAP]; let mut position = 0usize;
            for (call, n) in (0..=maximum).chain(std::iter::once(maximum)).enumerate() {
                let draining = call == maximum + 1;
                let silent = if draining { 15 } else if call % 2 == 0 { 10 } else { 5 };
                let mut item = Item::control(AUDIO, 1);
                item.n = n as u32; item.position = position as u64; item.ticket = call as u64 + 1;
                item.flags = silent;
                let mut extra = [[0.; CAP]; 2];
                for (ch, plane) in item.data.iter_mut().chain(extra.iter_mut()).enumerate() {
                    for (i, sample) in plane[..n].iter_mut().enumerate() {
                        let value = if silent & (1 << ch) != 0 { 0. }
                            else { ((position + i) % 997) as f32 * (ch as f32 + 1.) / 4096. };
                        history[ch].push(value);
                        *sample = value;
                    }
                }
                history_flags.extend(std::iter::repeat_n(silent, n));
                let mut completion = Completion::from(item);
                if n > 0 {
                    let pool = shared.extra.get().unwrap();
                    let slot = pool.publish_available(&extra, n).unwrap();
                    completion.audio.extra_slot = slot;
                }
                assert!(shared.publish_result(completion).is_some());
                let planes = out.each_mut().map(|p| p.as_mut_ptr());
                let mut flags = 99; let mut delivery = Delivery::default();
                let (result, allocations) = crate::allocation_test::measure(|| unsafe {
                    ap23_process_outputs(id, n as u32, 2, std::ptr::null(), 0,
                        &crate::context::Context::default(), 0, input.as_ptr(), input.as_ptr(),
                        planes.as_ptr(), 4, &mut flags, &mut delivery, 0)
                });
                assert_eq!(result, 0, "M={maximum}, N={n}, call={call}"); assert_eq!(allocations, [0; 3]);
                assert_eq!((delivery.missing_frames, delivery.expired_frames), (0, 0));
                let expected_flags = history_flags[position.saturating_sub(maximum)..(position + n).saturating_sub(maximum)]
                    .iter().fold(15, |mask, &flag| mask & flag);
                for (ch, plane) in out.iter().enumerate() {
                    for (i, &sample) in plane[..n].iter().enumerate() {
                        let absolute = position + i;
                        let expected = if absolute < maximum { 0. } else { history[ch][absolute - maximum] };
                        assert_eq!(sample, expected, "M={maximum}, N={n}, frame={absolute}, channel={ch}");
                    }
                }
                assert_eq!(flags, expected_flags);
                shared.requests.pop().unwrap(); position += n;
            }
            assert_eq!(position, total);
            assert_eq!(shared.fault.load(Ordering::Acquire), 0);
            INSTANCES.remove(id, |_| ()).unwrap();
        }
    }
    #[test]
    fn inactive_maximum_growth_and_shrink_reprepare_history_and_retain_the_extra_pool_extent() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared: shared.clone(), callback: UnsafeCell::new(Callback::new()),
            busy: AtomicBool::new(false), worker: None, report: None,
            max: 64, recovery_blocked: false, installed_delay: Some(1024),
            delivery_mode: crate::performance::DeliveryMode::Buffered, minor: 15, setup: None,
        })).unwrap().unwrap();
        let mut contract = 2u32.to_le_bytes().to_vec();
        for index in 0u32..2 {
            for value in [0u32, 1, index, 2, 0, 1] { contract.extend(value.to_le_bytes()); }
            contract.extend(3u64.to_le_bytes());
        }
        let worker_shared = shared.clone();
        let peer = thread::spawn(move || {
            for maximum in [64u64, 1024, 128] {
                let end = Instant::now() + Duration::from_secs(3);
                loop {
                    let mut control = worker_shared.control.lock().unwrap();
                    if let Some(c) = control.as_mut() {
                        if c.result.is_none() && ap1_native_client::get(&c.bytes[..4]) == maximum {
                            assert_eq!(c.op, 20);
                            let mut reply = vec![0; 16]; reply[8] = 1;
                            complete_control(&worker_shared, c, Ok(reply), ([0; 5],None)).unwrap();
                            break;
                        }
                    }
                    drop(control); assert!(Instant::now() < end); thread::yield_now();
                }
            }
        });
        let mut pool_identity: *const crate::output_pool::Pool = std::ptr::null();
        for maximum in [64usize, 1024, 128] {
            let mut traits = [0; 3];
            assert_eq!(unsafe { ap10_setup(id, maximum as u32, 2, 48000., contract.as_ptr(),
                contract.len() as u32, 1, traits.as_mut_ptr()) }, 0);
            assert_eq!(traits, [1024, 0, 0]);
            let pool = shared.extra.get().unwrap();
            if pool_identity.is_null() { pool_identity = pool as *const _; }
            else { assert_eq!(pool_identity, pool as *const _); }
            let live = INSTANCES.lease(id).unwrap();
            let callback = unsafe { &*live.callback.get() };
            assert_eq!((callback.audio.maximum, callback.audio.channels), (maximum, 4));
            assert_eq!(callback.audio.storage_bytes(), (1024 + maximum) * (4 * 4 + 8));
            drop(live);
            assert_eq!(unsafe { ap3_transition(id, START) }, 0); shared.requests.pop().unwrap();
            let input = [0.; CAP]; let mut output = [[12345.; CAP]; 4];
            for (ticket, position) in (0..1024 + maximum).step_by(maximum).enumerate() {
                let mut item = Item::control(AUDIO, 1);
                let epoch = INSTANCES.lease(id).unwrap().shared.wanted.load(Ordering::Acquire);
                item.epoch = epoch; item.n = maximum as u32; item.ticket = ticket as u64 + 1;
                item.position = position as u64; item.data = [[0.25; CAP], [0.5; CAP]];
                let mut completion = Completion::from(item);
                completion.audio.extra_slot = pool.publish_available(&[[0.75; CAP], [1.; CAP]], maximum).unwrap();
                assert!(shared.publish_result(completion).is_some());
                let planes = output.each_mut().map(|p| p.as_mut_ptr()); let mut flags = 0;
                let (result, allocations) = crate::allocation_test::measure(|| unsafe {
                    ap23_process_outputs(id, maximum as u32, 2, std::ptr::null(), 0,
                        &crate::context::Context::default(), 0, input.as_ptr(), input.as_ptr(),
                        planes.as_ptr(), 4, &mut flags, std::ptr::null_mut(), 0)
                });
                assert_eq!(result, 0); assert_eq!(allocations, [0; 3]);
                shared.requests.pop().unwrap();
                for (ch, plane) in output.iter().enumerate() {
                    let expected = if position < 1024 { 0. } else { (ch as f32 + 1.) / 4. };
                    assert!(plane[..maximum].iter().all(|&value| value == expected));
                    assert!(plane[maximum..].iter().all(|&value| value == 12345.));
                }
            }
            assert_eq!(unsafe { ap3_transition(id, STOP) }, 0); shared.requests.pop().unwrap();
            // CAP remains the immutable transport slot extent across legal
            // inactive M changes; source history alone is prepared to D+M.
            assert!(pool.publish(0, &[[0.; CAP]; 2], CAP)); pool.release(0);
        }
        peer.join().unwrap(); INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn close_publishes_offline_cancellation_before_waiting_for_the_callback_lease() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = prepared_live(shared.clone(), 64, 2, crate::performance::DeliveryMode::SameCallback);
        let joined = Arc::new(AtomicBool::new(false));
        let worker_shared = shared.clone(); let worker_joined = joined.clone();
        let worker = thread::spawn(move || {
            while !worker_shared.quit.load(Ordering::Acquire) {
                thread::sleep(crate::performance::INTERRUPT_INTERVAL);
            }
            // Retained worker storage outlives the callback's cancellation wake.
            thread::sleep(Duration::from_millis(20));
            worker_joined.store(true, Ordering::Release);
        });
        INSTANCES.update(id, |l| l.worker = Some(worker)).unwrap();
        let callback = thread::spawn(move || {
            let input = [0.; 64]; let mut out = [[9.; 64]; 2]; let mut flags = 0;
            let planes = [out[0].as_mut_ptr(), out[1].as_mut_ptr()];
            let (result, allocations) = crate::allocation_test::measure(|| unsafe {
                ap23_process_outputs(id, 64, 2, std::ptr::null(), 0,
                    &crate::context::Context::default(), 3, input.as_ptr(), input.as_ptr(),
                    planes.as_ptr(), 2, &mut flags, std::ptr::null_mut(), 0)
            });
            (result, allocations, out)
        });
        let admission = Instant::now() + Duration::from_secs(1);
        while shared.requests.published() <= 1 {
            assert!(Instant::now() < admission); thread::yield_now();
        }
        let started = Instant::now();
        let close = unsafe { if2_close(id) };
        let elapsed = started.elapsed();
        let (result, allocations, output) = callback.join().unwrap();
        assert_eq!(result, COMPLETION_CANCELLED); assert_eq!(allocations, [0; 3]);
        assert_eq!(output, [[9.; 64]; 2]);
        assert_eq!(close, 2, "unclean cancellation is not normal successful retirement");
        assert!(joined.load(Ordering::Acquire)); assert!(INSTANCES.lease(id).is_none());
        assert!(elapsed < Duration::from_millis(250), "close waited for the 60-second callback bound");
    }
    #[test]
    fn offline_completes_slow_valid_work_beyond_the_old_transport_timeout() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let mut callback = Callback::new();
        callback.delay = 0;
        assert_eq!(callback.transition(&shared, START), 0);
        shared.requests.pop().unwrap();
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared: shared.clone(), callback: UnsafeCell::new(callback),
            busy: AtomicBool::new(false), worker: None, report: None,
            max: 64, recovery_blocked: false, installed_delay: Some(256),
            delivery_mode: crate::performance::DeliveryMode::Buffered,
            minor: 14, setup: Some(crate::performance::wire_version(64, 2, 48000., true).unwrap()),
        })).unwrap().unwrap();
        let peer = shared.clone();
        let worker = thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(1);
            let item = loop {
                if let Some(item) = peer.requests.pop() { break item; }
                assert!(Instant::now() < until);
                thread::yield_now();
            };
            thread::sleep(Duration::from_millis(5100));
            assert!(peer.publish_result(item.into()).is_some());
        });
        let input = [0.375; 64]; let mut output = [[9.; 64]; 2];
        let mut flags = 0; let mut delivery = Delivery::default();
        let started = Instant::now();
        let (result, allocations) = crate::allocation_test::measure(|| unsafe {
            if2_process(id, 64, std::ptr::null(), 0, &crate::context::Context::default(),
                0, input.as_ptr(), input.as_ptr(), output[0].as_mut_ptr(), output[1].as_mut_ptr(),
                &mut flags, &mut delivery, 0)
        });
        let elapsed = started.elapsed();
        worker.join().unwrap();
        INSTANCES.remove(id, |_| ()).unwrap();
        assert_eq!(result, 0);
        assert_eq!(allocations, [0; 3]);
        assert!(elapsed >= Duration::from_secs(5), "offline returned before its operation completed");
        assert_eq!(delivery.missing_frames, 0, "timeout silence is not successful offline audio");
        assert_eq!(output, [[0.375; 64]; 2]);
    }
    #[test]
    fn offline_terminal_state_is_an_explicit_failure() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        shared.terminal_latched.store(true, Ordering::Release);
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared, callback: UnsafeCell::new(Callback::new()), busy: AtomicBool::new(false),
            worker: None, report: None, max: 64, recovery_blocked: false,
            installed_delay: Some(256), delivery_mode: crate::performance::DeliveryMode::Buffered, minor: 14,
            setup: Some(crate::performance::wire_version(64, 2, 48000., true).unwrap()),
        })).unwrap().unwrap();
        let input = [0.; 64]; let mut output = [[9.; 64]; 2]; let mut flags = 0;
        let result = unsafe {if2_process(id, 64, std::ptr::null(), 0,
            &crate::context::Context::default(), 3, input.as_ptr(), input.as_ptr(),
            output[0].as_mut_ptr(), output[1].as_mut_ptr(), &mut flags, std::ptr::null_mut(), 0)};
        INSTANCES.remove(id, |_| ()).unwrap();
        assert_eq!(result, 2, "offline must never authorize contained success silence");
        assert_eq!(output, [[9.; 64]; 2], "the SDK edge owns failed-output silence");
    }
    #[test]
    fn exact_realtime_terminal_state_cannot_substitute_contained_success_silence() {
        let _registry_owner = crate::registry_test();
        for delivery in [crate::performance::DeliveryMode::Buffered,
            crate::performance::DeliveryMode::SameCallback] {
            let shared = Arc::new(Shared::new());
            shared.state_capable.store(true, Ordering::Release);
            shared.terminal_latched.store(true, Ordering::Release);
            let id = prepared_live(shared.clone(), 64, 0, delivery);
            let input = [0.; 64]; let mut output = [[9.; 64]; 2]; let mut flags = 99;
            let planes = [output[0].as_mut_ptr(), output[1].as_mut_ptr()];
            for n in [0, 64] {
                let result = unsafe { ap23_process_outputs(id, n, 1, std::ptr::null(), 0,
                    &crate::context::Context::default(), 3, input.as_ptr(), input.as_ptr(),
                    planes.as_ptr(), 2, &mut flags, std::ptr::null_mut(), 0) };
                assert_eq!(result, if n == 0 || delivery == crate::performance::DeliveryMode::SameCallback {
                    2
                } else { CONTAINED_TERMINAL });
            }
            assert_eq!(shared.requests.published(), 1);
            assert_eq!(output, [[9.; 64]; 2]); assert_eq!(flags, 99);
            INSTANCES.remove(id, |_| ()).unwrap();
        }
    }
    #[test]
    fn whole_daw_block_preserves_sparse_queues_after_gui_state_and_seek_without_allocating() {
        let _registry_owner = crate::registry_test();
        let path = std::env::temp_dir().join(format!("whole-block-{}", u128::from_le_bytes(
            ap1_native_client::mapping::random().unwrap())));
        let gui = Arc::new(crate::gui::Gui::create(&path, [14; 16]).unwrap());
        let mut shared = Shared::new();
        shared.gui = Some(gui.clone());
        shared.identity = Some(state::Identity { class: [14; 16], module: [15; 32] });
        shared.state_capable.store(true, Ordering::Release);
        let shared = Arc::new(shared);
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live { shared: shared.clone(),
            callback: UnsafeCell::new(Callback::new()), busy: AtomicBool::new(false),
            worker: None, report: None, max: 1024, recovery_blocked: false,
            installed_delay: Some(1024), delivery_mode: crate::performance::DeliveryMode::Buffered, minor: 14, setup: None })).unwrap().unwrap();
        assert_eq!(unsafe { ap3_transition(id, START) }, 0);
        shared.requests.pop().unwrap();
        let input = [0.25; 1024]; let mut output = [[0.; 1024]; 2];
        let mut flags = 0; let mut delivery = Delivery::default();
        // No bridge parameter values are established. The vendor owns the
        // implicit -1 value; even an exact end anchor passes through unchanged.
        for (call, n, project) in [(1, 1008, 500), (2, 1024, 1508), (3, 1008, 42), (4, 0, 42)] {
            assert_eq!(gui.send(&mut crate::gui::Message { kind: 3, id: 7, value: 0.8,
                ..Default::default() }), 0);
            shared.curve_state_revision.store(call, Ordering::Release);
            let events = [Event { offset: n, kind: 2, id: 7, value: 0.6, ..Default::default() },
                Event { offset: n.saturating_sub(1), kind: 2, id: 99, value: 0.25, ..Default::default() }];
            let context = crate::context::Context { present: 1, rate: 48000., state: 0x21006,
                project, continuous: call as i64 * 1024, cycle_start: 0.125, cycle_end: 0.5,
                ..Default::default() };
            let before = shared.requests.published();
            let (rc, allocations) = crate::allocation_test::measure(|| unsafe { if2_process(id, n,
                events.as_ptr(), 2, &context, 0, input.as_ptr(), input.as_ptr(),
                output[0].as_mut_ptr(), output[1].as_mut_ptr(), &mut flags, &mut delivery, call * 1000) });
            assert_eq!(rc, 0); assert_eq!(allocations, [0; 3]);
            assert_eq!(shared.requests.published(), before + 1);
            let request = shared.requests.pop().unwrap();
            assert_eq!(request.n, n); assert_eq!(request.parent, [call, n as u64, 0, call * 1000]);
            assert_eq!(request.event_count, 2); assert_eq!(request.events[..2], events);
            assert_eq!(request.context.encode(), context.encode());
            assert_eq!(request.data[0][..n as usize], input[..n as usize]);
            assert!(shared.requests.pop().is_none());
        }
        let before = shared.requests.published();
        let bad = Event { kind: 0, offset: 1008, value: 0.5, pitch: 60, ..Default::default() };
        let (rc, allocations) = crate::allocation_test::measure(|| unsafe { if2_process(id, 1008,
            &bad, 1, &crate::context::Context::default(), 0, input.as_ptr(), input.as_ptr(),
            output[0].as_mut_ptr(), output[1].as_mut_ptr(), &mut flags, &mut delivery, 9000) });
        assert_eq!(rc, 0x102); assert_eq!(allocations, [0; 3]);
        assert_eq!(shared.requests.published(), before); assert_eq!(if2_terminal_status(id), 0);
        assert_eq!(unsafe { ap3_transition(id, STOP) }, 0);
        INSTANCES.remove(id, |_| ()).unwrap(); std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn all_output_planes_share_timeline_and_release_on_stop() {
        let shared=Shared::new();
        shared.extra.set(crate::output_pool::Pool::new(62,DESCRIPTORS)).ok().unwrap();
        let pool=shared.extra.get().unwrap();
        let mut callback=Callback::new();
        callback.prepare(CAP, 64, DELAY as usize);
        assert_eq!(callback.transition(&shared,START),0);
        assert_eq!(shared.requests.pop().unwrap().kind,START);
        let mut main=[[0.;CAP];2];
        let mut extra=vec![[0.;CAP];62];
        let pointers:Vec<_>=extra.iter_mut().map(|p|p.as_mut_ptr()).collect();
        for block in 0..8 {
            let mut request=Item::control(AUDIO,0);request.n=CAP as u32;
            callback.process_outputs(&shared,request,&mut main,&pointers,0).unwrap();
            if block>=1 {
                assert_eq!(main[0],[block as f32-1.;CAP]);
                for (ch, plane) in extra.iter().enumerate() {assert_eq!(*plane,[100.+ch as f32+block as f32-1.;CAP]);}
            } else {assert!(extra.iter().flatten().all(|x|*x==0.));}
            let request=shared.requests.pop().unwrap();
            let mut completion=Completion::from(request);
            completion.audio.data=[[block as f32;CAP];2];
            completion.audio.flags=0;
            let planes:Vec<_>=(0..62).map(|ch|[100.+ch as f32+block as f32;CAP]).collect();
            let slot=shared.results.published() as usize % DESCRIPTORS;
            assert!(pool.publish(slot,&planes,CAP));
            completion.audio.extra_slot=slot;
            assert!(shared.results.push(completion));
        }
        assert_eq!(callback.transition(&shared,STOP),0);
        let planes=vec![[0.;CAP];62];
        for slot in 0..8 {assert!(pool.publish(slot,&planes,CAP));pool.release(slot);}
    }

    #[test]
    fn setup_abi_delivers_complete_multi_output_contract() {
        let _registry_owner = crate::registry_test();
        for maximum in [512, 1024] {
            let shared = Arc::new(Shared::new());
            shared.state_capable.store(true, Ordering::Release);
            let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
                shared: shared.clone(), callback: UnsafeCell::new(Callback::new()),
                busy: AtomicBool::new(false), worker: None, report: None,
                max: maximum as usize, recovery_blocked: false, installed_delay: Some(maximum),
                delivery_mode: crate::performance::DeliveryMode::Buffered,
                minor: 12, setup: None,
            })).unwrap().unwrap();
            let mut contract = 34u32.to_le_bytes().to_vec();
            for (media, direction, index, channels) in (0u32..32)
                .map(|i| (0u32, 1u32, i, 2u32))
                .chain([(1, 0, 0, 16), (1, 1, 0, 16)]) {
                for value in [media, direction, index, channels, 0, u32::from(index == 0)] {
                    contract.extend(value.to_le_bytes());
                }
                contract.extend((if media == 0 { 3u64 } else { 0 }).to_le_bytes());
            }
            let expected = contract.clone();
            let peer = thread::spawn(move || {
                let until = Instant::now() + Duration::from_secs(3);
                loop {
                    if let Some(c) = shared.control.lock().unwrap().as_mut() {
                        assert_eq!(c.op, 20);
                        assert_eq!(&c.bytes[24..], expected);
                        let mut reply = vec![0; 16];
                        reply[8] = 1;
                        complete_control(&shared, c, Ok(reply), ([0; 5],None)).unwrap();
                        break;
                    }
                    assert!(Instant::now() < until, "setup ABI did not deliver bus contract");
                    thread::yield_now();
                }
            });
            let mut traits = [0u32; 3];
            assert_eq!(unsafe { ap10_setup(id, maximum, 0, 48000., contract.as_ptr(),
                contract.len() as u32, 1, traits.as_mut_ptr()) }, 0);
            assert_eq!(traits, [maximum, 0, 0]);
            peer.join().unwrap();
            assert_eq!(unsafe { ap10_setup(id, maximum, 0, 48000., contract.as_ptr(),
                crate::performance::MAX_BUS_CONTRACT_BYTES + 1, 1, traits.as_mut_ptr()) }, 1);
            INSTANCES.remove(id, |_| ()).unwrap();
        }
    }
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
    fn unacknowledged_start_is_answered_with_silence_within_its_bound() {
        let shared = Shared::new();
        let mut callback = Callback::new();
        callback.delay = 512;
        callback.unready_bound = Some(Duration::from_millis(2));
        callback.transition(&shared, START);
        shared.requests.pop().unwrap();
        let mut request = Item::control(AUDIO, 0);
        request.n = 512;
        request.data = [[0.25; CAP]; 2];
        request.event_count = 0;
        // The Windows side has not acknowledged the start: the callback
        // returns silence inside its bound and queues nothing.
        let mut output = [[9.; CAP]; 2];
        let started = Instant::now();
        let (result, allocations) = crate::allocation_test::measure(||
            callback.process_outputs_until(&shared, request, &mut output, &[], 0,
                Some(started + Duration::from_secs(5))));
        assert_eq!(result, Ok(3));
        assert_eq!(allocations, [0; 3]);
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(output[0][..512].iter().chain(&output[1][..512]).all(|v| *v == 0.));
        assert!(shared.requests.pop().is_none(), "an unready block is not part of the stream");
        assert_eq!((callback.position, callback.unready_callbacks, callback.unready_frames,
            callback.completion_waits), (0, 1, 512, 0));
        assert_eq!(callback.delivery, Delivery::default());
        assert_eq!(shared.fault.load(Ordering::Acquire), 0);
        // An offline render keeps the full wait: its block is queued.
        let mut offline = request;
        offline.process_mode = 2;
        callback.process_outputs_until(&shared, offline, &mut output, &[], 0, None).unwrap();
        assert_eq!(shared.requests.pop().unwrap().position, 0);
        assert_eq!(callback.unready_callbacks, 1);
        // Once the start is acknowledged the next realtime block is queued at
        // the position the stream has reached.
        shared.processing_ready_epoch.store(callback.epoch, Ordering::Release);
        callback.process_outputs_until(&shared, request, &mut output, &[], 0, None).unwrap();
        assert_eq!(shared.requests.pop().unwrap().position, 512);
        assert_eq!((callback.unready_callbacks, callback.unready_frames), (1, 512));
    }
    #[test]
    fn completion_deadline_keeps_silence_expiry_and_control_nonwaiting() {
        let shared = Shared::new();
        let mut callback = Callback::new();
        callback.delay = 512;
        callback.transition(&shared, START);
        shared.requests.pop().unwrap();
        let mut request = Item::control(AUDIO, 0);
        request.n = 512;
        request.data = [[0.25; CAP]; 2];
        let mut output = [[9.; CAP]; 2];
        callback.process_outputs_until(&shared, request, &mut output, &[], 0,
            Some(Instant::now() + Duration::from_millis(10))).unwrap();
        assert_eq!(callback.completion_waits, 0, "priming cannot wait");
        let late = shared.requests.pop().unwrap();
        let started = Instant::now();
        let (result, allocations) = crate::allocation_test::measure(||
            callback.process_outputs_until(&shared, request, &mut output, &[], 0,
                Some(started + Duration::from_millis(2))));
        result.unwrap();
        assert_eq!(allocations, [0; 3]);
        assert!(started.elapsed() < Duration::from_millis(100), "dead peer exceeded the local budget by 98 ms");
        assert_eq!(callback.delivery.missing_frames, 512);
        assert!(output.iter().all(|plane| plane[..512].iter().all(|v| *v == 0.)));
        assert_eq!(callback.completion_wait_misses, 1);
        assert_eq!(shared.fault.load(Ordering::Acquire), 0);
        shared.requests.pop().unwrap();
        assert!(shared.publish_result(late.into()).is_some());
        shared.pending_control.store(true, Ordering::Release);
        let waits = callback.completion_waits;
        callback.process_outputs_until(&shared, request, &mut output, &[], 0,
            Some(Instant::now() + Duration::from_millis(10))).unwrap();
        assert_eq!(callback.completion_waits, waits, "control work cannot extend the callback");
        assert_eq!(callback.delivery.expired_frames, 512, "late audio must never shift forward");
        assert_eq!(callback.delivery.missing_frames, 512);
        shared.requests.pop().unwrap();
        shared.pending_control.store(false, Ordering::Release);
        request.n = 0;
        callback.process_outputs_until(&shared, request, &mut output, &[], 0,
            Some(Instant::now() + Duration::from_millis(10))).unwrap();
        assert_eq!(callback.completion_waits, waits, "zero-frame flush cannot wait");
        assert_eq!(callback.delivery.missing_frames, 0);
    }

    #[test]
    fn phase_trace_wait_clock_requires_every_interval() {
        let mut trace = PhaseTrace::default();
        retain_wait_clock(&mut trace, 10, 14);
        assert_eq!((trace.wait_count,trace.wait_begin_ns,trace.wait_end_ns,
            trace.wait_total_ns,trace.clock_valid&PHASE_WAIT),(1,10,14,4,PHASE_WAIT));
        retain_wait_clock(&mut trace, 0, 20);
        assert_eq!((trace.wait_count,trace.wait_begin_ns,trace.wait_end_ns,
            trace.wait_total_ns,trace.clock_valid&PHASE_WAIT),(2,10,14,4,0));
        retain_wait_clock(&mut trace, 30, 35);
        assert_eq!((trace.wait_count,trace.wait_begin_ns,trace.wait_end_ns,
            trace.wait_total_ns,trace.clock_valid&PHASE_WAIT),(3,10,14,4,0));

        let mut unavailable_first = PhaseTrace::default();
        retain_wait_clock(&mut unavailable_first, 0, 0);
        retain_wait_clock(&mut unavailable_first, 40, 50);
        assert_eq!((unavailable_first.wait_count,unavailable_first.wait_begin_ns,
            unavailable_first.wait_end_ns,unavailable_first.wait_total_ns,
            unavailable_first.clock_valid&PHASE_WAIT),(2,0,0,0,0));
    }

    #[test]
    fn phase_trace_separates_buffered_completion_wait_from_host_presentation() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let mut callback = Callback::new();
        callback.prepare(512, 2, 512);
        callback.running = true;
        callback.epoch = 1;
        callback.position = 1024;
        callback.next_result = 512;
        callback.submitted_operation = 1;
        // The stream is two blocks in: its start was acknowledged long ago.
        shared.processing_ready_epoch.store(1, Ordering::Release);
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared: shared.clone(), callback: UnsafeCell::new(callback),
            busy: AtomicBool::new(false), worker: None, report: None,
            max: 512, recovery_blocked: false, installed_delay: Some(512),
            delivery_mode: crate::performance::DeliveryMode::Buffered,
            minor: 15, setup: Some(crate::performance::wire_version(512, 0, 48000., true).unwrap()),
        })).unwrap().unwrap();
        let peer = shared.clone();
        let worker = thread::spawn(move || {
            let until = Instant::now() + Duration::from_secs(1);
            let current = loop {
                if let Some(item) = peer.requests.pop() { break item; }
                assert!(Instant::now() < until);
                thread::yield_now();
            };
            assert_eq!((current.position,current.n,current.ticket),(1024,512,2));
            while peer.phase_waits.load(Ordering::Acquire)==0 {
                assert!(Instant::now()<until);
                thread::yield_now();
            }
            let mut prior = Item::control(AUDIO, 1);
            prior.n = 512;
            prior.position = 512;
            prior.ticket = 1;
            prior.data = [[0.25; CAP]; 2];
            assert!(peer.publish_result(prior.into()).is_some());
        });
        let input = [0.5; 512];
        let mut output = [[9.; 512]; 2];
        let planes = [output[0].as_mut_ptr(),output[1].as_mut_ptr()];
        let mut flags = 0;
        let mut delivery = Delivery::default();
        let mut trace = PhaseTrace { schema: PHASE_TRACE_SCHEMA,
            size: std::mem::size_of::<PhaseTrace>() as u32, ..Default::default() };
        let entered = crate::observer::monotonic_ns();
        let (result, allocations) = crate::allocation_test::measure(|| unsafe {
            ap23_process_outputs_trace(id,512,0,std::ptr::null(),0,
                &crate::context::Context::default(),0,input.as_ptr(),input.as_ptr(),
                planes.as_ptr(),2,&mut flags,&mut delivery,entered,&mut trace)
        });
        worker.join().unwrap();
        INSTANCES.remove(id, |_| ()).unwrap();
        assert_eq!(result,0);assert_eq!(allocations,[0;3]);
        assert_eq!(output,[[0.25;512];2]);assert_eq!(delivery.delivered_frames,512);
        assert_eq!((trace.generation,trace.epoch,trace.host_call,trace.position),(1,1,1,1024));
        assert_eq!((trace.frames,trace.mode,trace.delivery_mode,trace.exact),(512,0,0,0));
        assert_eq!(trace.allowance_ns,5_000_000_000);
        assert!(trace.wait_deadline_lower_ns<=trace.wait_deadline_upper_ns);
        assert_eq!((trace.predicate_kind,trace.predicate_required,
            trace.predicate_before,trace.predicate_after),(0,1024,512,1024));
        assert_eq!((trace.predicate_initial_satisfied,trace.predicate_final_satisfied),(0,1));
        assert!(trace.wait_count>=1);
        #[cfg(target_os="linux")]
        assert!(trace.wait_begin_ns>0&&trace.wait_end_ns>=trace.wait_begin_ns&&trace.wait_total_ns>0,
            "wait trace {:?}",(trace.wait_count,trace.wait_begin_ns,trace.wait_end_ns,trace.wait_total_ns));
        let complete = PHASE_RUST_ENTRY|PHASE_POLICY|PHASE_IDENTITY|PHASE_PREDICATE_BEFORE|
            PHASE_WAIT|PHASE_PREDICATE_AFTER|PHASE_PRESENTATION_DONE|
            PHASE_RUST_PRE_RETURN|PHASE_BACKEND_RESULT;
        assert_eq!(trace.phase_reached&complete,complete);
        let clocked = PHASE_RUST_ENTRY|PHASE_POLICY|PHASE_WAIT|
            PHASE_PRESENTATION_DONE|PHASE_RUST_PRE_RETURN;
        #[cfg(target_os="linux")]
        {
            assert_eq!(trace.clock_valid&clocked,clocked);
            assert!(trace.rust_entry_ns<=trace.wait_begin_ns&&
                trace.wait_end_ns<=trace.presentation_done_ns&&
                trace.presentation_done_ns<=trace.rust_pre_return_ns);
        }
        #[cfg(not(target_os="linux"))]
        {
            assert_eq!(trace.clock_valid&clocked,0);
            assert_eq!((trace.rust_entry_ns,trace.wait_deadline_lower_ns,
                trace.wait_deadline_upper_ns,trace.wait_begin_ns,trace.wait_end_ns,
                trace.wait_total_ns,trace.presentation_done_ns,trace.rust_pre_return_ns),
                (0,0,0,0,0,0,0,0));
        }
    }

    #[test]
    fn phase_trace_does_not_write_outside_the_existing_instance_guard() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let id = prepared_live(shared,64,0,crate::performance::DeliveryMode::Buffered);
        let lease = INSTANCES.lease(id).unwrap();
        let guard = Guard::acquire(&lease).unwrap();
        let input = [0.;64];let mut output=[[9.;64];2];
        let planes=[output[0].as_mut_ptr(),output[1].as_mut_ptr()];let mut flags=0;
        let mut trace=PhaseTrace {schema:PHASE_TRACE_SCHEMA,
            size:std::mem::size_of::<PhaseTrace>() as u32,phase_reached:1,ordinal:7,
            cpp_entry_ns:23,..Default::default()};
        let result=unsafe {ap23_process_outputs_trace(id,64,0,std::ptr::null(),0,
            &crate::context::Context::default(),3,input.as_ptr(),input.as_ptr(),planes.as_ptr(),2,
            &mut flags,std::ptr::null_mut(),23,&mut trace)};
        assert_eq!(result,3);
        assert_eq!((trace.phase_reached,trace.clock_valid,trace.ordinal,trace.cpp_entry_ns),(1,0,7,23));
        assert_eq!((trace.rust_entry_ns,trace.backend_result,trace.rust_pre_return_ns),(0,0,0));
        assert_eq!(output,[[9.;64];2]);
        drop(guard);drop(lease);INSTANCES.remove(id, |_| ()).unwrap();
    }

    #[test]
    fn burst_callbacks_deliver_completed_audio_within_the_block_budget() {
        let _registry_owner = crate::registry_test();
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        let mut callback = Callback::new();
        callback.delay = 512;
        assert_eq!(callback.transition(&shared, START), 0);
        shared.requests.pop().unwrap();
        // The peer below has taken the start; acknowledge it as the worker would.
        shared.processing_ready_epoch.store(1, Ordering::Release);
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared: shared.clone(), callback: UnsafeCell::new(callback),
            busy: AtomicBool::new(false), worker: None, report: None,
            max: 512, recovery_blocked: false, installed_delay: Some(512),
            delivery_mode: crate::performance::DeliveryMode::Buffered,
            minor: 14, setup: Some(crate::performance::wire_version(512, 0, 48000., true).unwrap()),
        })).unwrap().unwrap();
        let peer = shared.clone();
        let worker = thread::spawn(move || {
            for _ in 0..16 {
                let until = Instant::now() + Duration::from_secs(1);
                let item = loop {
                    if let Some(item) = peer.requests.pop() { break item; }
                    assert!(Instant::now() < until);
                    thread::yield_now();
                };
                // A measured Deck request needed about 2 ms, but the DAW
                // presented it after only 1.34 ms. Sample delay is not pacing.
                thread::sleep(Duration::from_millis(2));
                assert!(peer.publish_result(Completion::from(item)).is_some());
            }
        });
        let mut missing = 0;
        let mut correct = true;
        for block in 0..16 {
            let input = [0.25 + block as f32 / 64.; 512];
            let mut output = [[9.; 512]; 2];
            let mut flags = 0;
            let mut delivery = Delivery::default();
            let (rc, allocations) = crate::allocation_test::measure(|| unsafe {
                if2_process(id, 512, std::ptr::null(), 0, &crate::context::Context::default(),
                    0, input.as_ptr(), input.as_ptr(), output[0].as_mut_ptr(), output[1].as_mut_ptr(),
                    &mut flags, &mut delivery, 0)
            });
            assert_eq!(rc, 0);
            assert_eq!(allocations, [0; 3]);
            missing += delivery.missing_frames;
            let expected = if block == 0 { 0. } else { 0.25 + (block - 1) as f32 / 64. };
            correct &= output.iter().flatten().all(|v| *v == expected);
        }
        worker.join().unwrap();
        INSTANCES.remove(id, |_| ()).unwrap();
        assert_eq!(missing, 0, "bursty callbacks discarded deliverable audio");
        assert!(correct, "sample positions or fixed bridge delay changed");
    }

    #[test]
    fn parent_callbacks_preserve_exact_one_and_two_proxy_delay() {
        let _registry_owner = crate::registry_test();
        // The consumer runs only after the complete parent host callback. A
        // 512-frame parent must not acquire an artificial wait between chunks.
        for (maximum, delay) in [(1024, 1024), (512, 1024), (512, 512), (256, 512), (256, 256), (128, 256)] {
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
                    delivery_mode: crate::performance::DeliveryMode::Buffered,
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
        let _registry_owner = crate::registry_test();
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
            delivery_mode: crate::performance::DeliveryMode::Buffered,
            minor: 11, setup: None,
        })).unwrap().unwrap();
        let input = [0f32;512]; let mut output = [[0f32;512];2];
        assert_eq!(unsafe { ap3_transition(id, START) }, 0);
        shared.requests.pop().unwrap();
        let mut total=0; let mut maximum=0; let mut first=0;
        for block in 0..1100 {
            // An acknowledgement for another epoch cannot classify this block
            // as processing. First two blocks precede the exact readiness.
            if block == 1 { shared.processing_ready_epoch.store(2, Ordering::Release); }
            if block == 2 { shared.processing_ready_epoch.store(1, Ordering::Release); }
            let mut flags=0; let mut d=Delivery::default();
            let before=faults();
            let (rc, allocations)=crate::allocation_test::measure(|| unsafe { ap10_process(id,512,std::ptr::null(),0,
                &crate::context::Context::default(),3,input.as_ptr(),input.as_ptr(),
                output[0].as_mut_ptr(),output[1].as_mut_ptr(),&mut flags,&mut d) });
            let delta=faults()-before;
            assert_eq!(allocations, [0; 3], "phase attribution allocated/reallocated/freed in the callback");
            assert_eq!(rc,0); total+=delta; maximum=maximum.max(delta);if block==0 {first=delta;}
            // Same parent 512-frame host block: both chunks have already been
            // admitted before its off-thread consumer can return completions.
            for _ in 0..2 {
                let item=shared.requests.pop().unwrap();
                assert!(shared.results.push(Completion::from(item)));
            }
        }
        eprintln!("AP13 parent512 callback minor faults: first={first} total={total} max={maximum}");
        assert_eq!(processing_phase(0, 0), 0, "unstarted epochs cannot report processing readiness");
        assert_eq!(shared.delivery_phases[0][0].load(Ordering::Acquire), 1024);
        assert_eq!(shared.delivery_phases[1][0].load(Ordering::Acquire), 1098*512);
        for i in 0..6 {
            assert_eq!(shared.delivery_totals[i].load(Ordering::Acquire),
                shared.delivery_phases[0][i].load(Ordering::Acquire)+shared.delivery_phases[1][i].load(Ordering::Acquire));
        }
        let mut phases = PhaseRecords::default();
        phases.record(&shared, "processing_stopped", 1);
        assert_eq!(phases.records[0].delivery[0][0], 1024);
        for _ in 0..129 { phases.record(&shared, "processing_stopped", 1); }
        assert_eq!(phases.length, 128);
        assert_eq!(phases.omitted, 2);
        INSTANCES.remove(id, |_| ()).unwrap();
        // A host's fresh callback thread can fault in its code/stack on the
        // first invocation (12 pages in the unoptimized CI host). It is not
        // memory owned by this instance. The following calls still traverse
        // every newly used queue page; the old implementation faults throughout.
        assert_eq!(total-first, 0, "queue pages must be touched before callback use");
    }
    #[test]
    fn acknowledged_setup_is_retained_for_recovery() {
        let _registry_owner = crate::registry_test();
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
                    delivery_mode: crate::performance::DeliveryMode::Buffered,
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
                    let reply = [
                        7u32.to_le_bytes(),
                        18u32.to_le_bytes(),
                        1u32.to_le_bytes(),
                        0u32.to_le_bytes(),
                    ]
                    .concat();
                    complete_control(&peer, c, Ok(reply), ([0; 5],None)).unwrap();
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
    fn recovery_keeps_exact_parameter_census_but_discards_failed_peer_values() {
        use crate::parameter_curves::Carry;
        let point = |id, offset, value| Event { id, offset, value, kind: 2, ..Event::default() };
        let mut callback = Callback::new();
        callback.delay = 512;
        callback.curve_values.configure(&[7, 99]).unwrap();
        callback.curve_plan.prepare(&[point(7, 0, 0.3)], 512,
            &Carry::empty(), &callback.curve_values).unwrap();
        callback.curve_values.commit(&callback.curve_plan.last);
        callback.curve_plan.prepare(&[point(7, 512, 0.6)], 512,
            &Carry::empty(), &callback.curve_values).unwrap();
        let mut old = Shared::new(); old.generation = 7;
        callback.gaps.record(&old,3,512,128,[9,512,0,1234]);
        let mut next = callback.replacement();
        assert_eq!(next.delay, 512);
        assert_eq!(next.gaps.length,1);
        assert_eq!(next.gaps.records[0].generation,7);
        assert_eq!(next.gaps.records[0].epoch,3);
        assert_eq!(callback.gaps.length,0);
        assert!(next.curve_values.configure(&[7, 99]).is_err());
        assert!(next.curve_plan.prepare(&[point(7, 512, 0.6)], 512,
            &Carry::empty(), &next.curve_values).is_err());
        assert!(next.curve_plan.prepare(&[point(123, 0, 0.6)], 512,
            &Carry::empty(), &next.curve_values).is_err());
        next.curve_plan.prepare(&[point(99, 0, 0.4)], 512,
            &Carry::empty(), &next.curve_values).unwrap();
    }
    #[test]
    fn production_curve_endpoint_carry_respects_edits_seeks_stop_and_capacity() {
        let _registry_owner = crate::registry_test();
        use crate::parameter_curves::Carry;
        use crate::gui::{Gui, Message};
        let path = std::env::temp_dir().join(format!("lvb-curves-{}", u128::from_le_bytes(
            ap1_native_client::mapping::random().unwrap())));
        let gui = Arc::new(Gui::create(&path, [7; 16]).unwrap());
        let mut shared = Shared::new();
        shared.gui = Some(gui.clone());
        shared.state_capable.store(true, Ordering::Relaxed);
        shared.identity = Some(state::Identity { class: [1; 16], module: [2; 32] });
        let shared = Arc::new(shared);
        let callback = Callback::new();
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live { shared: shared.clone(),
            callback: UnsafeCell::new(callback), busy: AtomicBool::new(false), worker: None,
            report: None, max: 1024, recovery_blocked: false, installed_delay: None,
            delivery_mode: crate::performance::DeliveryMode::Buffered,
            minor: 7, setup: None })).unwrap().unwrap();
        assert_eq!(unsafe { ap22_curve_parameters(id,[7].as_ptr(),1) },0);
        assert_eq!(unsafe { ap22_curve_parameters(id,[7].as_ptr(),1) },1);
        assert_eq!(unsafe { ap3_transition(id,START) },0);
        assert_eq!(unsafe { ap22_curve_parameters(id,[7].as_ptr(),1) },2);
        assert_eq!(shared.requests.pop().unwrap().kind, START);
        let input = [0.; 1024];
        let mut left = [0.; 1024];
        let mut right = [0.; 1024];
        let mut flags = 0;
        let mut delivery = Delivery::default();
        let point = |offset, value| Event { offset, kind: 2, id: 7, value, ..Event::default() };
        let run = |n, project, events: &[Event], left: &mut [f32; 1024], right: &mut [f32; 1024], flags: &mut u64,
            delivery: &mut Delivery| unsafe { ap10_process(id, n, events.as_ptr(), events.len() as u32,
            &crate::context::Context { present: 1, state: 2, rate: 48000., project, ..Default::default() },
            0, input.as_ptr(), input.as_ptr(), left.as_mut_ptr(), right.as_mut_ptr(), flags, delivery) };
        let (result, allocations) = crate::allocation_test::measure(|| run(1024, 500,
            &[point(0, 0.125), point(1024, 0.75)], &mut left, &mut right, &mut flags, &mut delivery));
        assert_eq!(result, 0); assert_eq!(allocations, [0; 3]);
        for start in [0, 256, 512, 768] {
            let item = shared.requests.pop().unwrap();
            assert_eq!(item.event_count, 2);
            for event in &item.events[..2] {
                assert!(event.valid(256));
                assert!((event.value - (0.125 + 0.625 * (start + event.offset) as f64 / 1024.)).abs() < 1e-14);
            }
        }
        assert_eq!(run(512, 1524, &[], &mut left, &mut right, &mut flags, &mut delivery), 0);
        let first = shared.requests.pop().unwrap();
        assert_eq!(first.event_count, 1); assert_eq!(first.events[0], point(0, 0.75));
        assert_eq!(shared.requests.pop().unwrap().event_count, 0);
        assert_eq!(run(512, 2036, &[point(0, 0.3), point(512, 0.7)], &mut left, &mut right, &mut flags, &mut delivery), 0);
        for _ in 0..2 { shared.requests.pop().unwrap(); }
        assert_eq!(gui.send(&mut Message { kind: 3, id: 7, value: 0.8, ..Default::default() }), 0);
        assert_eq!(run(512, 2548, &[], &mut left, &mut right, &mut flags, &mut delivery), 0);
        for _ in 0..2 { assert_eq!(shared.requests.pop().unwrap().event_count, 0); }
        assert_eq!(run(512, 3060, &[point(0, 0.2), point(512, 0.6)], &mut left, &mut right, &mut flags, &mut delivery), 0);
        for _ in 0..2 { shared.requests.pop().unwrap(); }
        assert_eq!(run(512, 42, &[], &mut left, &mut right, &mut flags, &mut delivery), 0);
        for _ in 0..2 { assert_eq!(shared.requests.pop().unwrap().event_count, 0); }
        let previous = 0.2 + 0.4 * 511. / 512.;
        let (result, allocations) = crate::allocation_test::measure(|| run(512, 554,
            &[point(512, 0.6)], &mut left, &mut right, &mut flags, &mut delivery));
        assert_eq!(result, 0); assert_eq!(allocations, [0; 3]);
        for start in [0, 256] {
            let item = shared.requests.pop().unwrap();
            for event in &item.events[..item.event_count as usize] {
                assert!(event.valid(256));
                let value = previous + (0.6 - previous) * (start + event.offset as usize + 1) as f64 / 513.;
                assert!((event.value - value).abs() < 1e-14);
            }
        }
        assert_eq!(gui.send(&mut Message { kind: 3, id: 7, value: 0.8, ..Default::default() }), 0);
        let before = shared.requests.published();
        let refused = unsafe { if2_process(id, 512, [point(512, 0.6)].as_ptr(), 1,
            &crate::context::Context { present: 1, state: 2, rate: 48000., project: 1066, ..Default::default() },
            0, input.as_ptr(), input.as_ptr(), left.as_mut_ptr(), right.as_mut_ptr(), &mut flags, &mut delivery, 1) };
        assert_eq!(refused, PARAMETER_CURVE_UNAVAILABLE);
        assert_ne!(refused, CONTAINED_TERMINAL);
        assert_eq!(if2_terminal_status(id), 0);
        assert_eq!(shared.requests.published(), before);
        let lease = INSTANCES.lease(id).unwrap();
        let callback = unsafe { &mut *lease.callback.get() };
        callback.curve_carry = Carry::empty(); callback.curve_carry.count = 1;
        callback.curve_carry.events[0] = point(0, 0.9);
        assert_eq!(callback.transition(&shared, STOP), 0);
        assert_eq!(callback.curve_carry.count, 0); assert!(callback.curve_continuation.is_none());
        drop(lease); INSTANCES.remove(id, |_| ()).unwrap();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn large_host_blocks_preserve_notes_and_parameter_offsets() {
        let _registry_owner = crate::registry_test();
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
                    delivery_mode: crate::performance::DeliveryMode::Buffered,
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
                offset: 0,
                kind: 2,
                id: 900,
                value: 0.25,
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
                    events.len() as u32,
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
        assert_eq!(observed, [events[0], events[1], Event { offset: 255, ..events[1] },
            Event { offset: 256, ..events[1] }, events[2], events[3]]);
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
                    3,
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
            assert_eq!(item.data[0][..LEGACY_CAP], [0.25; LEGACY_CAP]);
            assert!(item.data[0][LEGACY_CAP..].iter().all(|v| *v == 0.));
            assert_eq!(item.data[1][..LEGACY_CAP], [-0.5; LEGACY_CAP]);
            assert!(item.data[1][LEGACY_CAP..].iter().all(|v| *v == 0.));
            assert_eq!(item.event_count, if offset == 0 { 3 } else { 2 });
            assert_eq!(item.events[0].offset, if offset == 0 { 7 } else { 0 });
            assert_eq!(item.events[item.event_count as usize - 1].offset, 255);
        }
        INSTANCES.remove(id, |_| ()).unwrap();
    }
    #[test]
    fn actual_transport_progresses_with_paused_observation_consumer_and_reader() {
        use ap1_native_client::{
            endpoint::{receive_version, send_version},
            mapping::Mapping,
            ClientState, Frame, Slot, INPUT,
        };
        use std::os::unix::fs::FileExt;
        for (minor, frames) in [(4, 256usize), (14, 1024usize)] {
        let path = std::env::temp_dir().join(format!(
            "ap7-observer-{}.audio",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        let mapping = if minor == 14 {
            Mapping::with_layout(&path, 64, true).unwrap()
        } else { Mapping::new(&path).unwrap() };
        let output_offset = mapping.output;
        let stride = mapping.stride;
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
                let f = receive_version(&mut remote, 5, minor).unwrap();
                let payload = if f.kind == 3 {
                    assert_eq!(ap1_native_client::get(&f.payload[40..48]), processed * frames as u64);
                    let mut bytes = vec![0u8; frames * 4];
                    for ch in 0..2 {
                        file.read_exact_at(&mut bytes, (INPUT + ch * stride + 4) as u64)
                            .unwrap();
                        for value in bytes.chunks_exact_mut(4) {
                            let sample = f32::from_le_bytes(value.try_into().unwrap()) * 0.5;
                            value.copy_from_slice(&sample.to_le_bytes());
                        }
                        file.write_all_at(&bytes, (output_offset + ch * stride + 4) as u64)
                            .unwrap();
                    }
                    processed += 1;
                    let mut payload = [
                        (frames as u32).to_le_bytes().as_slice(),
                        (output_offset as u32).to_le_bytes().as_slice(),
                        0u64.to_le_bytes().as_slice(),
                        &f.payload[32..48],
                    ]
                    .concat();
                    if minor == 14 { payload.resize(72, 0); }
                    payload
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
                    minor,
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
            notifications: None, configured_mode: 0,
            gui: None,
            gui_revision: 0,
            mailbox: None,
            mailbox_enabled: false,
            #[cfg(target_os="linux")] direct_requested:false,
                capture: None,
        fault_status: None,
            notices: (0, 0),
            returned: crate::process_results::Packet::default(),
            processing: crate::ProcessingScratch::new(),
            mapping: Some(mapping),
            socket,
            state: ClientState {
                session: [1; 16],
                next: 1,
                slot: Slot::Writable,
            },
            phase: 9,
            max: CAP,
            minor,
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
        let transport = thread::spawn(move || worker(session, service, None, None));
        let mut callback = Callback::new();
        assert_eq!(callback.transition(&shared, START), 0);
        let mut item = Item::control(AUDIO, 0);
        item.n = frames as u32;
        item.gain = if minor == 14 { f64::NAN } else { 0.5 };
        item.data = [[0.25; CAP]; 2];
        let mut output = [[0.; CAP]; 2];
        for n in 0..128 {
            callback.process(&shared, item, &mut output).unwrap();
            assert_eq!(callback.delivery.missing_frames, 0);
            for plane in &output { assert!(plane[..frames].iter().all(|&v|
                v == if n < (DELAY as usize / frames) { 0. } else { 0.125 })); }
            let end = Instant::now() + Duration::from_secs(2);
            while shared.results.published() <= n as u64 {
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
        assert_eq!(observation.offered.load(Ordering::Relaxed), 128 * 2 * frames as u64);
        assert_eq!(reader.observation.comparison.samples, 0); // none checked
        drop(reader);
        std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn terminal_peer_exit_reaches_bounded_query_after_mapping_unlink() {
        let _registry_owner = crate::registry_test();
        use ap1_native_client::{ClientState,Slot};
        let dir=std::env::temp_dir().join(format!("if1-worker-{}",u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
        std::fs::create_dir(&dir).unwrap();
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let socket=std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (remote,_)=listener.accept().unwrap();drop(remote); // unexpected peer exit
        let status=crate::fault_status::Status::create(&dir.join("ap12.status"),[31;16]).unwrap();
        let terminal=status.terminal.clone();
        let session=Session {
            notifications:None,configured_mode:0,
            gui:None,gui_revision:0,mapping:Some(ap1_native_client::mapping::Mapping::new(&dir.join("ap1.audio")).unwrap()),
            mailbox:None,mailbox_enabled:false,
            #[cfg(target_os="linux")] direct_requested:false,capture:None,fault_status:Some(status),notices:(0,0),returned:Default::default(),processing:crate::ProcessingScratch::new(),socket,
            state:ClientState{session:[31;16],next:104687,slot:Slot::Writable},phase:11,max:CAP,minor:11,epoch:2,position:768,
            witness:None,identity:None,trace:Default::default(),sample_rate:48000,armed:false,owner:None,
        };
        let mut shared=Shared::new();shared.generation=7;shared.terminal=Some(terminal.clone());
        let shared=Arc::new(shared);shared.wanted.store(2,Ordering::Release);
        let mut item=Item::control(AUDIO,2);item.n=256;item.position=768;item.data=[[0.;CAP];2];
        assert!(shared.requests.push(item));
        let peer=shared.clone();let thread=thread::spawn(move||worker(session,peer,None,None));
        thread.join().unwrap();
        assert_eq!(shared.fault.load(Ordering::Acquire),WORKER);
        assert!(shared.terminal_latched.load(Ordering::Acquire));
        shared.state_capable.store(true,Ordering::Release);
        let id=INSTANCES.insert(||Ok::<_,()>(Live{shared,callback:UnsafeCell::new(Callback::new()),busy:AtomicBool::new(false),worker:None,report:None,max:CAP,recovery_blocked:false,installed_delay:None,delivery_mode:crate::performance::DeliveryMode::Buffered,minor:11,setup:None})).unwrap().unwrap();
        std::fs::remove_dir_all(&dir).unwrap(); // the UI query outlives physical transport cleanup
        let mut r=crate::terminal::Record::default();
        let input=[0f32;256];let mut left=[0f32;256];let mut right=[0f32;256];let mut flags=0;let mut delivery=Delivery::default();
        unsafe{assert_eq!(if2_process(id,256,std::ptr::null(),0,&crate::context::Context::default(),3,input.as_ptr(),input.as_ptr(),left.as_mut_ptr(),right.as_mut_ptr(),&mut flags,&mut delivery,1),CONTAINED_TERMINAL);}
        unsafe{assert_eq!(if1_terminal(id,&mut r),0);}
        assert_eq!(&r.words[3..7],&[7,2,104687,768]);assert_eq!(&r.words[14..16],&[3,WORKER]);
        let first=r;terminal.fail_native(99);
        unsafe{assert_eq!(if1_terminal(id,&mut r),0);assert_eq!(r,first);ap3_close(id);assert_ne!(if1_terminal(id,&mut r),0);}
    }
    #[test]
    fn contained_terminal_keeps_validation_and_never_admits_more_work() {
        let _registry_owner = crate::registry_test();
        use std::os::unix::fs::FileExt;
        // Simulate each external committed producer, then exercise the actual
        // public native ABI. Only the non-RT query reads the complete mapping.
        for (class, producer, domain) in [(1u64,3u64,4u64),(2,2,2),(3,1,1)] {
            let path=std::env::temp_dir().join(format!("if2-custody-{}",u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
            let status=Arc::new(crate::terminal::Status::create(&path,[42;16]).unwrap());
            status.progress([7,2,104687,512,11],Some(512),None);
            let mut record=status.context().unwrap();record.words[14]=class;record.words[15]=215;
            record.words[18]=producer;record.words[19]=domain;
            let file=std::fs::OpenOptions::new().write(true).open(&path).unwrap();
            let mut shared=Shared::new();shared.generation=7;shared.terminal=Some(status);
            shared.identity=Some(state::Identity{class:[1;16],module:[2;32]});
            shared.state_capable.store(true,Ordering::Release);
            let shared=Arc::new(shared);
            let mut callback=Callback::new();callback.running=true;callback.epoch=2;callback.position=512;
            let id=INSTANCES.insert(||Ok::<_,()>(Live{shared:shared.clone(),callback:UnsafeCell::new(callback),busy:AtomicBool::new(false),worker:None,report:None,max:512,recovery_blocked:false,installed_delay:None,delivery_mode:crate::performance::DeliveryMode::Buffered,minor:11,setup:None})).unwrap().unwrap();
            let input=[0f32;512];let mut left=[9f32;512];let mut right=[9f32;512];
            let context=crate::context::Context::default();let mut flags=99;let mut delivery=Delivery::default();
            // No complete custody: a local fault is not relabeled contained.
            shared.fail(WORKER,512);
            unsafe {assert_eq!(if2_process(id,256,std::ptr::null(),0,&context,3,input.as_ptr(),input.as_ptr(),left.as_mut_ptr(),right.as_mut_ptr(),&mut flags,&mut delivery,1),2);}
            for (i,v) in record.words.iter().enumerate(){file.write_all_at(&v.to_le_bytes(),128+(producer-1)*256+i as u64*8).unwrap();}
            file.write_all_at(&producer.to_le_bytes(),64).unwrap();
            let mut read=crate::terminal::Record::default();
            unsafe{assert_eq!(if1_terminal(id,&mut read),0);}assert_eq!(read,record);
            let queued=shared.requests.published();
            // The UI can see external custody before the worker sees EOF.
            // A subblock admission must also honor the latch in that window.
            shared.fault.store(0,Ordering::Release);
            let lease=INSTANCES.lease(id).unwrap();let mut out=[[0f32;CAP];2];
            unsafe{assert_eq!((*lease.callback.get()).process(&shared,Item::control(AUDIO,2),&mut out),Err(2));}
            drop(lease);shared.fault.store(WORKER,Ordering::Release);
            let call=|n,events:&[Event],ctx:&crate::context::Context,input_flags,left:&mut [f32;512],right:&mut [f32;512],flags:&mut u64,delivery:&mut Delivery|unsafe{
                if2_process(id,n,events.as_ptr(),events.len() as u32,ctx,input_flags,input.as_ptr(),input.as_ptr(),left.as_mut_ptr(),right.as_mut_ptr(),flags,delivery,1)
            };
            for n in [0,1,64,256,512] {for _ in 0..1000 {assert_eq!(call(n,&[],&context,3,&mut left,&mut right,&mut flags,&mut delivery),if n==0 {2} else {CONTAINED_TERMINAL});}}
            assert_eq!(shared.requests.published(),queued);
            let lease=INSTANCES.lease(id).unwrap();unsafe{assert_eq!((*lease.callback.get()).position,512);assert_eq!((*lease.callback.get()).epoch,2);assert_eq!((*lease.callback.get()).host_call,1);}drop(lease);
            assert!(left.iter().chain(&right).all(|x|*x==9.)); // SDK owner fills silence.
            assert_eq!(flags,99);
            let good=Event{offset:144,kind:0,id:17,channel:0,pitch:60,value:0.75,..Default::default()};
            assert_eq!(call(256,&[good],&context,3,&mut left,&mut right,&mut flags,&mut delivery),CONTAINED_TERMINAL);
            for bad in [Event{offset:256,..good},Event{channel:16,..good},Event{value:f64::NAN,..good},Event{value:1.1,..good}] {
                assert_eq!(call(256,&[bad],&context,3,&mut left,&mut right,&mut flags,&mut delivery),0x102);
            }
            assert_eq!(call(513,&[],&context,3,&mut left,&mut right,&mut flags,&mut delivery),0x101);
            let bad_context=crate::context::Context{present:1,rate:f64::NAN,..Default::default()};
            assert_eq!(call(256,&[],&bad_context,3,&mut left,&mut right,&mut flags,&mut delivery),0x105);
            unsafe {assert_eq!(if2_process(id,256,std::ptr::null(),0,&context,3,std::ptr::null(),input.as_ptr(),left.as_mut_ptr(),right.as_mut_ptr(),&mut flags,&mut delivery,1),0x101);}
            // Old ABI keeps its old failure contract.
            unsafe {assert_eq!(ap13_process(id,256,std::ptr::null(),0,&context,3,input.as_ptr(),input.as_ptr(),left.as_mut_ptr(),right.as_mut_ptr(),&mut flags,&mut delivery,1),2);}
            assert_eq!(shared.requests.published(),queued);
            // Custody alone cannot authorize positive containment cleanup.
            if class==1 {unsafe{assert_eq!(if2_close(id),2);}}
            else {shared.retired.store(true,Ordering::Release);unsafe{assert_eq!(if2_close(id),0);assert_ne!(if2_close(id),0);}}
            unsafe {assert_ne!(if1_terminal(id,&mut read),0);}
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn correlated_save_refusal_keeps_worker_audio_snapshot_and_sibling() {
        let _registry_owner = crate::registry_test();
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
            notifications: None, configured_mode: 0,
                gui: None,
                gui_revision: 0,
                mailbox: None,
                mailbox_enabled: false,
            #[cfg(target_os="linux")] direct_requested:false,
                capture: None,
        fault_status: None,
                notices: (0, 0),
                returned: Default::default(),
                processing: crate::ProcessingScratch::new(),
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
            let worker = thread::spawn(move || super::worker(session, service, None, None));
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
                        delivery_mode: crate::performance::DeliveryMode::Buffered,
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
    #[test]
    fn exact_deadline_retains_the_first_bounded_status_before_live_progress_advances() {
        let dir=std::env::temp_dir().join(format!("ap12-refusal-{:032x}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
        std::fs::create_dir(&dir).unwrap();
        let mut status=crate::fault_status::Status::create(&dir.join("ap12.status"),[31;16]).unwrap();
        status.publish(2,17,0,2,0);
        let mut shared=Shared::new();
        shared.generation=1;
        shared.phase_diagnostics=true;
        shared.fault_reader=Some(status.reader());
        shared.worker_binding.publish([1,2,0,1,4,17]);
        let identity=DeadlineIdentity {generation:1,epoch:2,position:0,host_call:4,
            operation_ticket:1,operation_submitted:true};
        let (_,allocations)=crate::allocation_test::measure(||shared.fail_deadline(0,identity));
        assert_eq!(allocations,[0;3]);
        shared.worker_binding.publish([1,2,0,2,5,18]);
        status.publish(2,18,0,3,0);
        shared.fail_deadline(99,DeadlineIdentity {host_call:5,..identity});
        let retained=shared.first_refusal.read().unwrap();
        assert_eq!(retained.status.lanes[0].words[2],17);
        assert_eq!(retained.status.lanes[0].words[4],2);
        assert_eq!(shared.fault_reader.as_ref().unwrap().snapshot().lanes[0].words[4],3);
        assert_eq!(shared.first_position.load(Ordering::Acquire),0);
        let text=refusal_text(retained);
        assert!(text.contains("\"stage\":2"));
        assert!(text.contains("\"native_sequence_reference\":17"));
        assert!(text.contains("\"native_status_matches_binding\":true"));
        assert!(text.contains("\"callback_binding\":\"matched\""));
        assert!(text.contains("\"progress_is_atomic_snapshot\":false"));
        drop(shared);drop(status);std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn recovery_export_retains_the_old_refusal_once_before_shared_replacement() {
        let dir=std::env::temp_dir().join(format!("ap12-recovery-export-{:032x}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
        std::fs::create_dir(&dir).unwrap();
        let mut status=crate::fault_status::Status::create(&dir.join("ap12.status"),[32;16]).unwrap();
        status.publish(3,21,0,2,0);
        let mut shared=Shared::new();shared.generation=1;shared.phase_diagnostics=true;
        shared.fault_reader=Some(status.reader());shared.worker_binding.publish([1,3,0,1,7,21]);
        shared.fail_deadline(0,DeadlineIdentity {generation:1,epoch:3,position:0,host_call:7,
            operation_ticket:1,operation_submitted:true});
        let old=Arc::new(shared);let report=dir.join("old.jsonl");
        export_refusal(&old,&report);
        // Both a later failed recovery cleanup and the ordinary close path may
        // reach this helper; neither can duplicate the old generation row.
        export_refusal(&old,&report);
        let replacement=Arc::new(Shared::new());
        assert!(replacement.first_refusal.read().is_none());
        drop(replacement);
        let text=std::fs::read_to_string(&report).unwrap();
        assert_eq!(text.matches("\"event\":\"ap23_exact_deadline_status\"").count(),1);
        assert_eq!(text.matches("\"event\":\"ap23_exact_deadline_lane\"").count(),3);
        assert!(text.lines().all(|record|record.len()<=2048));
        assert!(text.contains("\"callback_binding\":\"matched\""));
        drop(old);drop(status);std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn refusal_report_keeps_stability_and_identity_matches_independent() {
        let mut native=[0;16];native[..10].copy_from_slice(&[1,2,17,0,2,0,100,1_000_000_000,7,8]);
        let mut stale=[0;16];stale[..10].copy_from_slice(&[9,2,17,1,4,0,200,10_000_000,10,11]);
        let snapshot=RefusalSnapshot {
            identity:DeadlineIdentity {generation:1,epoch:2,position:0,host_call:4,
                operation_ticket:1,operation_submitted:true},
            worker_binding:[
                WorkerBindingObservation {stability:crate::fault_status::Stability::Stable,
                    attempts:1,publication_before:2,publication_after:2,words:[1,2,0,1,4,17]},
                WorkerBindingObservation {stability:crate::fault_status::Stability::Stable,
                    attempts:1,publication_before:2,publication_after:2,words:[1,2,0,1,4,17]},
            ],
            status:crate::fault_status::Snapshot {lanes:[
                crate::fault_status::Lane {stability:crate::fault_status::Stability::Stable,
                    attempts:1,publication_before:3,publication_after:3,words:native,word_count:16},
                crate::fault_status::Lane {stability:crate::fault_status::Stability::Stable,
                    attempts:1,publication_before:4,publication_after:4,words:stale,word_count:10},
                crate::fault_status::Lane {stability:crate::fault_status::Stability::Unstable,
                    attempts:3,publication_before:5,publication_after:6,words:[0;16],word_count:10},
            ]},
        };
        let text=refusal_text(snapshot);
        assert!(text.contains("\"lane\":\"windows_delivery\",\"stability\":\"stable\""));
        assert!(text.contains("\"matches_refusal\":{\"generation\":false,\"epoch\":true,\"position\":false}"));
        assert!(text.contains("\"matches_native\":{\"generation\":false,\"epoch\":true,\"position\":false,\"sequence\":true}"));
        assert!(text.contains("\"correlates_native\":false"));
        assert!(text.contains("\"callback_binding\":\"matched\""));
        assert!(text.contains("\"lane\":\"windows_ui_owner\",\"stability\":\"unstable\",\"attempts\":3"));
        assert!(text.contains("\"generation\":null"));
    }
    #[test]
    fn changed_or_predecessor_worker_binding_cannot_bind_repeated_zero_frame_status() {
        let mut native=[0;16];native[..10].copy_from_slice(&[1,2,18,0,2,0,100,1_000_000_000,7,8]);
        let lane=crate::fault_status::Lane {stability:crate::fault_status::Stability::Stable,
            attempts:1,publication_before:4,publication_after:4,words:native,word_count:16};
        let observed=|publication,ticket,host_call,sequence| WorkerBindingObservation {
            stability:crate::fault_status::Stability::Stable,attempts:1,
            publication_before:publication,publication_after:publication,
            words:[1,2,0,ticket,host_call,sequence],
        };
        let identity=DeadlineIdentity {generation:1,epoch:2,position:0,host_call:5,
            operation_ticket:2,operation_submitted:true};
        let snapshot=|bindings| RefusalSnapshot {identity,worker_binding:bindings,
            status:crate::fault_status::Snapshot {lanes:[lane,
                crate::fault_status::Lane {stability:crate::fault_status::Stability::Absent,
                    attempts:1,publication_before:0,publication_after:0,words:[0;16],word_count:10},
                crate::fault_status::Lane {stability:crate::fault_status::Stability::Absent,
                    attempts:1,publication_before:0,publication_after:0,words:[0;16],word_count:10}]}};
        let predecessor=refusal_text(snapshot([observed(3,1,4,18),observed(3,1,4,18)]));
        assert!(predecessor.contains("\"worker_binding\":{\"before\":"));
        assert!(predecessor.contains("\"matches_refusal\":false"));
        assert!(predecessor.contains("\"native_status_matches_binding\":true"));
        assert!(predecessor.contains("\"callback_binding\":\"unknown\""));
        let changed=refusal_text(snapshot([observed(3,2,5,18),observed(4,3,6,19)]));
        assert!(changed.contains("\"unchanged\":false"));
        assert!(changed.contains("\"callback_binding\":\"unknown\""));
    }
    #[test]
    fn diagnostics_disabled_deadline_has_no_status_snapshot_or_callback_allocation() {
        let shared=Shared::new();
        assert!(!shared.phase_diagnostics);assert!(shared.fault_reader.is_none());
        let (_,allocations)=crate::allocation_test::measure(||shared.fail_deadline(0,
            DeadlineIdentity {generation:1,epoch:1,position:0,host_call:1,
                operation_ticket:1,operation_submitted:true}));
        assert_eq!(allocations,[0;3]);
        assert!(shared.first_refusal.read().is_none());
        assert_eq!(shared.fault.load(Ordering::Acquire),COMPLETION_DEADLINE);
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
        let _registry_owner = crate::registry_test();
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
                    delivery_mode: crate::performance::DeliveryMode::Buffered,
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
            for plane in &out { assert_eq!(plane[..256], [if i < 2 { 0. } else { 0.125 }; 256]); }
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
    fn untraced_missing_span_retains_bounded_context_until_close() {
        let _registry_owner = crate::registry_test();
        let path = std::env::temp_dir().join(format!("audio-gap-{}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(true, Ordering::Release);
        assert!(shared.observer.is_none());
        let mut callback = Callback::new(); callback.delay = 512;
        let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
            shared: shared.clone(), callback: UnsafeCell::new(callback),
            busy: AtomicBool::new(false), worker: None, report: Some(path.clone()),
            max: 512, recovery_blocked: false, installed_delay: Some(512),
            delivery_mode: crate::performance::DeliveryMode::Buffered,
            minor: 14, setup: None,
        })).unwrap().unwrap();
        let input = [0.; 512]; let mut output = [[9.; 512]; 2];
        let mut missing = 0;
        for epoch in 1..=2 {
            assert_eq!(unsafe { ap3_transition(id, START) }, 0);
            shared.requests.pop().unwrap();
            shared.processing_ready_epoch.store(epoch, Ordering::Release);
            shared.worker_op.store(AUDIO as u64, Ordering::Release);
            shared.worker_epoch.store(epoch, Ordering::Release);
            shared.worker_position.store(0, Ordering::Release);
            for block in 0..10 {
                let mut flags = 0; let mut delivery = Delivery::default();
                let (rc, allocations) = crate::allocation_test::measure(|| unsafe {
                    if2_process(id,512,std::ptr::null(),0,&crate::context::Context::default(),
                        3,input.as_ptr(),input.as_ptr(),output[0].as_mut_ptr(),output[1].as_mut_ptr(),
                        &mut flags,&mut delivery,1234)
                });
                assert_eq!(rc,0); assert_eq!(allocations,[0;3]);
                assert_eq!(delivery.missing_frames,if block == 0 {0} else {512});
                assert!(output.iter().flatten().all(|v| *v == 0.));
                missing += delivery.missing_frames;
                // The peer consumes requests but deliberately supplies no output.
                shared.requests.pop().unwrap();
            }
            assert_eq!(unsafe { ap3_transition(id, STOP) },0);
            shared.requests.pop().unwrap();
        }
        assert_eq!(missing,18*512);
        assert_eq!(shared.fault.load(Ordering::Acquire),0);
        assert!(!path.exists(),"no callback file output");
        shared.ack.store(15,Ordering::Release);
        assert_eq!(unsafe { if2_close(id) },0);
        let report = std::fs::read_to_string(&path).unwrap();
        assert_eq!(report.lines().filter(|l| l.contains("\"event\":\"audio_presentation_gap\"")).count(),16);
        assert!(report.contains("\"omitted_spans\":2"));
        assert!(report.contains("\"epoch\":2"));
        assert!(report.contains("\"position\":0,\"frames\":512"));
        assert!(report.contains("\"parent_callback\":[2,512,0,1234]"));
        assert!(report.contains("\"worker\":[3,1,0]"));
        std::fs::remove_file(path).unwrap();
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
        for plane in &out { assert_eq!(plane[..256], [0.125; 256]); }
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
        for plane in &out { assert_eq!(plane[..256], [0.125; 256]); }
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

#[cfg(test)]
#[path = "lc1_tests.rs"]
mod lc1_tests;

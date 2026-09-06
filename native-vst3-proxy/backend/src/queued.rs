//! AP3 callback operations contain only bounded copies, scalar checks and atomics.
//! All mapping/socket/session work and error formatting belong to the worker.
use crate::{binding, queue::Queue, retain, state, Session};
use ap1_native_client::{invalid, CAP, ERROR};
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
pub const UNDERFLOW: u64 = 1;
const OVERFLOW: u64 = 2;
const WORKER: u64 = 3;
const CORRELATION: u64 = 4;
#[derive(Clone, Copy)]
pub struct Item {
    pub kind: u32,
    pub n: u32,
    pub epoch: u64,
    pub position: u64,
    pub gain: f64,
    pub flags: u64,
    pub data: [[f32; CAP]; 2],
}
impl Item {
    fn control(kind: u32, epoch: u64) -> Self {
        Self {
            kind,
            n: 0,
            epoch,
            position: 0,
            gain: 0.,
            flags: 0,
            data: [[0.; CAP]; 2],
        }
    }
}
struct Shared {
    snapshots: Arc<std::sync::Mutex<crate::recovery::Store>>,
    generation: u64,
    last_edit: AtomicU64,
    retired: AtomicBool,
    requests: Queue<Item>,
    results: Queue<Item>,
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
    witness: std::sync::Mutex<Observation>,
    observation_skips: AtomicU64,
    worker_op: AtomicU64,
    worker_epoch: AtomicU64,
    worker_position: AtomicU64,
    service_us_max: AtomicU64,
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
            snapshots: Arc::new(std::sync::Mutex::new(crate::recovery::Store::default())),
            generation: 1,
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
            witness: std::sync::Mutex::new(Observation::default()),
            observation_skips: AtomicU64::new(0),
            worker_op: AtomicU64::new(0),
            worker_epoch: AtomicU64::new(0),
            worker_position: AtomicU64::new(0),
            service_us_max: AtomicU64::new(0),
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
struct Callback {
    epoch: u64,
    position: u64,
    running: bool,
    have: bool,
    offset: usize,
    current: Item,
}
impl Callback {
    fn new() -> Self {
        Self {
            epoch: 0,
            position: 0,
            running: false,
            have: false,
            offset: 0,
            current: Item::control(0, 0),
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
                s.results.discard_published();
                s.wanted.store(self.epoch, Ordering::Release);
                self.running = true;
            }
            STOP if self.running => {
                self.running = false;
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
        if !s.requests.push(request) {
            s.fail(OVERFLOW, self.position);
            return Err(2);
        }
        if !request.gain.is_nan() {
            s.last_edit.store(s.requests.published(), Ordering::Release);
        }
        let mut flags = 3;
        // Two planar channels share one sample cursor; indexing expresses the layout.
        #[allow(clippy::needless_range_loop)]
        for i in 0..request.n as usize {
            let position = self.position + i as u64;
            if position < DELAY {
                out[0][i] = 0.;
                out[1][i] = 0.;
                continue;
            }
            let expected = position - DELAY;
            if !self.have {
                // At most one old epoch result can race discard_published at restart. A
                // bounded loop also rejects a violated queue/epoch invariant explicitly.
                for _ in 0..DESCRIPTORS {
                    match s.results.pop() {
                        Some(item) if item.epoch < self.epoch => continue,
                        Some(item) => {
                            self.current = item;
                            self.offset = 0;
                            self.have = true;
                            break;
                        }
                        None => break,
                    }
                }
                if !self.have {
                    s.fail(UNDERFLOW, position);
                    return Err(2);
                }
            }
            if self.current.epoch != self.epoch
                || self.current.n == 0
                || self.current.n as usize > CAP
                || self.current.position + self.offset as u64 != expected
            {
                s.fail(CORRELATION, position);
                return Err(2);
            }
            flags &= self.current.flags;
            for ch in 0..2 {
                out[ch][i] = self.current.data[ch][self.offset];
            }
            self.offset += 1;
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
                        let result = match c.op {
                            16 => session
                                .component_state(None)
                                .and_then(|p| state::envelope(&p)),
                            18 => session
                                .component_state(Some(&c.bytes))
                                .and_then(|p| state::envelope(&p)),
                            8 => (|| {
                                if session.witness.as_ref().is_some_and(|w| !w.ready) {
                                    session.component_state(None)?;
                                }
                                session
                                    .activate(
                                        ap1_native_client::get(&c.bytes[..4]) as usize,
                                        ap1_native_client::get(&c.bytes[4..]) as u32,
                                    )
                                    .map(|_| vec![])
                            })(),
                            14 => session.transition(14).map(|_| {
                                s.ack.store(15, Ordering::Release);
                                vec![]
                            }),
                            _ => Err(invalid("unknown owner operation")),
                        };
                        let failed = result.is_err();
                        if let Ok(bytes) = &result {
                            if matches!(c.op, 16 | 18) {
                                s.snapshots
                                    .lock()
                                    .map_err(|_| invalid("snapshot store poisoned"))?
                                    .confirm(
                                        bytes.clone(),
                                        c.op,
                                        s.generation,
                                        s.requests.consumed(),
                                    )?;
                            }
                        } else if let Err(error) = &result {
                            if let Ok(mut detail) = s.detail.lock() {
                                if detail.is_empty() {
                                    *detail = bounded_detail(error);
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
            s.worker_op.store(0, Ordering::Release);
            if let Some(w) = &session.witness {
                publish_observation(
                    &s,
                    Observation {
                        comparison: w.report,
                        input_hash: w.input_hash,
                        output_hash: w.output_hash,
                    },
                )?;
            }
            let Some(mut item) = s.requests.pop() else {
                thread::sleep(Duration::from_micros(50));
                continue;
            };
            s.worker_epoch.store(item.epoch, Ordering::Relaxed);
            s.worker_position.store(item.position, Ordering::Relaxed);
            s.worker_op.store(item.kind as u64, Ordering::Release);
            match item.kind {
                AUDIO => {
                    let started = Instant::now();
                    let n = item.n as usize;
                    let (words, flags) = session.process_positioned(
                        n,
                        item.gain,
                        item.flags,
                        [&item.data[0][..n], &item.data[1][..n]],
                        item.epoch,
                        item.position,
                    )?;
                    for (ch, word) in words.iter().enumerate() {
                        for i in 0..n {
                            item.data[ch][i] = f32::from_bits(word[i + 1]);
                        }
                    }
                    item.flags = flags;
                    s.service_us_max.fetch_max(
                        started.elapsed().as_micros().min(u64::MAX as u128) as u64,
                        Ordering::Relaxed,
                    );
                    s.processed.fetch_add(1, Ordering::Relaxed);
                    if n > 0
                        && s.wanted.load(Ordering::Acquire) == item.epoch
                        && !s.results.push(item)
                    {
                        s.fail(OVERFLOW, item.position);
                        return Err(invalid("completed output capacity"));
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
fn publish_observation(s: &Shared, observation: Observation) -> io::Result<()> {
    // Diagnostic readers may be descheduled with the lock held. Publishing a
    // cumulative observation is optional; audio service never waits for it.
    match s.witness.try_lock() {
        Ok(mut slot) => *slot = observation,
        Err(std::sync::TryLockError::WouldBlock) => {
            s.observation_skips.fetch_add(1, Ordering::Relaxed);
        }
        Err(std::sync::TryLockError::Poisoned(_)) => {
            return Err(invalid("fixture observer poisoned"));
        }
    }
    Ok(())
}
fn progress_text(s: &Shared) -> String {
    let ready = s.first_context_ready.load(Ordering::Acquire);
    format!(
        "{{\"event\":\"ap7_fault_progress\",\"context_ready\":{},\"epoch\":{},\"worker_op\":{},\"worker_epoch\":{},\"worker_position\":{},\"request_published\":{},\"request_consumed\":{},\"result_published\":{},\"result_consumed\":{},\"service_us_max_at_report\":{},\"observation_skips\":{}}}\n",
        ready, s.first_epoch.load(Ordering::Relaxed), s.first_worker_op.load(Ordering::Relaxed),
        s.first_worker_epoch.load(Ordering::Relaxed), s.first_worker_position.load(Ordering::Relaxed),
        s.first_requests[0].load(Ordering::Relaxed), s.first_requests[1].load(Ordering::Relaxed),
        s.first_results[0].load(Ordering::Relaxed), s.first_results[1].load(Ordering::Relaxed),
        s.service_us_max.load(Ordering::Relaxed), s.observation_skips.load(Ordering::Relaxed))
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
    open(max, handle, 3)
}
#[no_mangle]
pub unsafe extern "C" fn ap4_open(handle: *mut u64) -> u32 {
    open(256, handle, 4)
}
unsafe fn open(max: u32, handle: *mut u64, minor: u64) -> u32 {
    if handle.is_null() || !(1..=256).contains(&max) {
        return 1;
    }
    crate::ffi(|| {
        match INSTANCES.insert(|| {
            let binding = binding(minor == 4)?;
            let report = binding
                .owner
                .as_ref()
                .map(|_| crate::preview::report_path(binding.session));
            let session = Session::open(binding, max as usize, minor)?;
            let shared = Arc::new(Shared::new());
            shared.state_capable.store(minor == 4, Ordering::Release);
            if minor == 4 {
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
            })
        }) {
            Ok(Ok(id)) => {
                *handle = id;
                0
            }
            Ok(Err(e)) => retain(&e),
            Err(code) => code as i32,
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
            let payload = state::payload(&snapshot.bytes)?;
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
            let binding = binding(true)?;
            if binding.owner.is_none() {
                return Err(invalid("recovery requires the private owner"));
            }
            let report = Some(crate::preview::report_path(binding.session));
            let mut session = Session::open(binding, l.max, 4)?;
            if let Err(error) = session.component_state(Some(payload)) {
                let owner = session.owner.take();
                let _ = session.close();
                if let Some(owner) = owner {
                    let _ = owner.finish();
                }
                return Err(error);
            }
            let mut shared = Shared::new();
            shared.generation = l
                .shared
                .generation
                .checked_add(1)
                .ok_or_else(|| invalid("transport generation exhausted"))?;
            shared.snapshots = l.shared.snapshots.clone();
            shared.state_capable.store(true, Ordering::Release);
            shared.ack.store(17, Ordering::Release);
            if let Some(w) = &session.witness {
                *shared.witness.lock().unwrap() = Observation {
                    comparison: w.report,
                    input_hash: w.input_hash,
                    output_hash: w.output_hash,
                };
            }
            let shared = Arc::new(shared);
            let peer = shared.clone();
            let worker_report = report.clone();
            let t = thread::Builder::new()
                .name("ap6-transport".into())
                .spawn(move || worker(session, peer, worker_report))?;
            l.shared = shared;
            l.worker = Some(t);
            l.report = report;
            l.callback = UnsafeCell::new(Callback::new());
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
            state::payload(std::slice::from_raw_parts(restore, n as usize))
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
        return 1;
    }
    let mut item = Item::control(AUDIO, 0);
    item.n = n as u32;
    item.gain = gain;
    item.flags = flags;
    for (ch, p) in [left, right].into_iter().enumerate() {
        for (i, &v) in std::slice::from_raw_parts(p, n).iter().enumerate() {
            if !v.is_finite() || (flags & (1 << ch) != 0 && v != 0.) {
                return 1;
            }
            item.data[ch][i] = v;
        }
    }
    let mut out = [[0.; CAP]; 2];
    match (&mut *l.callback.get()).process(&l.shared, item, &mut out) {
        Ok(f) => {
            for (ch, p) in [out_left, out_right].into_iter().enumerate() {
                std::ptr::copy_nonoverlapping(out[ch].as_ptr(), p, n);
            }
            *out_flags = f;
            0
        }
        Err(code) => code,
    }
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
    comparison: state::WitnessReport,
    input_hash: u64,
    output_hash: u64,
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
        let result = match shared.witness.lock() {
            Ok(v) => {
                *out = *v;
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
        let result = match shared.witness.lock() {
            Ok(v) => {
                *out = v.comparison;
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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paused_diagnostic_reader_cannot_starve_due_audio() {
        let s = Arc::new(Shared::new());
        let mut cb = Callback::new();
        cb.transition(&s, START);
        let mut r = Item::control(AUDIO, 0);
        r.n = 256;
        r.gain = 0.5;
        r.data = [[0.25; CAP]; 2];
        let mut out = [[0.; CAP]; 2];
        // A diagnostic reader is descheduled while holding its short lock.
        // The same publication operation used by worker must not delay audio.
        let reader = s.witness.lock().unwrap();
        for _ in 0..4 {
            cb.process(&s, r, &mut out).unwrap();
        }
        let peer = s.clone();
        let (done, progress) = std::sync::mpsc::channel();
        let t = thread::spawn(move || {
            publish_observation(&peer, Observation::default()).unwrap();
            pump(&peer);
            done.send(()).unwrap();
        });
        let progressed = progress.recv_timeout(Duration::from_millis(200)).is_ok();
        let result = cb.process(&s, r, &mut out);
        drop(reader);
        t.join().unwrap();
        assert_eq!(
            result,
            Ok(0),
            "diagnostic contention caused a due-audio underflow"
        );
        assert!(progressed);
        assert_eq!(s.observation_skips.load(Ordering::Relaxed), 1);
        assert_eq!(out, [[0.125; CAP]; 2]);
        assert_eq!(s.fault.load(Ordering::Acquire), 0);
    }
    #[test]
    fn first_callback_fault_survives_later_worker_failure() {
        let s = Shared::new();
        s.wanted.store(2, Ordering::Release);
        s.worker_op.store(AUDIO as u64, Ordering::Release);
        s.worker_epoch.store(2, Ordering::Release);
        s.worker_position.store(0, Ordering::Release);
        s.fail(UNDERFLOW, 1024);
        s.worker_op.store(18, Ordering::Release);
        s.fail(WORKER, 2048);
        *s.detail.lock().unwrap() = "queued session fault or cancelled".into();
        let report = failure_snapshot(&s);
        assert_eq!(report.fault, UNDERFLOW);
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
                assert!(s.results.push(r));
            }
        }
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
                for ch in 0..2 {
                    assert_eq!(
                        out[ch][i],
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
    fn absent_due_output_latches_and_never_replays() {
        let s = Shared::new();
        let mut cb = Callback::new();
        cb.transition(&s, START);
        let mut r = Item::control(AUDIO, 0);
        r.n = 256;
        r.gain = 0.5;
        let mut out = [[0.; CAP]; 2];
        for _ in 0..4 {
            assert!(cb.process(&s, r, &mut out).is_ok());
        }
        assert_eq!(cb.process(&s, r, &mut out), Err(2));
        let w = s.requests.high_water();
        assert_eq!(cb.process(&s, r, &mut out), Err(2));
        assert_eq!(s.requests.high_water(), w);
        assert_eq!(s.fault.load(Ordering::Acquire), UNDERFLOW);
        assert_eq!(cb.transition(&s, START), 2);
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
        assert!(s.results.push(old));
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
        assert!(s.results.push(r));
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

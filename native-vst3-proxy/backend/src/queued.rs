//! AP3 callback operations contain only bounded copies, scalar checks and atomics.
//! All mapping/socket/session work and error formatting belong to the worker.
use crate::{binding, queue::Queue, retain, state, Session};
use ap1_native_client::{invalid, CAP, ERROR};
use std::{
    cell::UnsafeCell,
    io,
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicU64, Ordering},
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
}
impl Shared {
    fn new() -> Self {
        Self {
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
        }
    }
    fn fail(&self, code: u64, position: u64) {
        if self
            .fault
            .compare_exchange(0, code, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
        {
            self.first_position.store(position, Ordering::Release);
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
    id: u64,
    shared: Arc<Shared>,
    callback: UnsafeCell<Callback>,
    worker: UnsafeCell<Option<JoinHandle<()>>>,
    max: usize,
}
static ACTIVE: AtomicPtr<Live> = AtomicPtr::new(std::ptr::null_mut());
static BUSY: AtomicBool = AtomicBool::new(false);
static NEXT: AtomicU64 = AtomicU64::new(1);
struct Guard;
impl Guard {
    fn acquire() -> Option<Self> {
        BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| Self)
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}
// The nonblocking registry guard serializes lifecycle/callback/close, including
// invalid handles. Worker only owns Shared and Session, never Live/host buffers.
unsafe fn live(id: u64) -> Option<&'static Live> {
    let p = ACTIVE.load(Ordering::Acquire);
    if p.is_null() {
        None
    } else if (*p).id == id {
        Some(&*p)
    } else {
        None
    }
}
fn worker(mut session: Session, s: Arc<Shared>) {
    let run = (|| -> io::Result<()> {
        loop {
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
                        let result = match c.op {
                            16 => session
                                .component_state(None)
                                .and_then(|p| state::envelope(&p)),
                            18 => session
                                .component_state(Some(&c.bytes))
                                .and_then(|p| state::envelope(&p)),
                            8 => session
                                .activate(
                                    ap1_native_client::get(&c.bytes[..4]) as usize,
                                    ap1_native_client::get(&c.bytes[4..]) as u32,
                                )
                                .map(|_| vec![]),
                            14 => session.transition(14).map(|_| {
                                s.ack.store(15, Ordering::Release);
                                vec![]
                            }),
                            _ => Err(invalid("unknown owner operation")),
                        };
                        let failed = result.is_err();
                        c.result = Some(result);
                        s.pending_control.store(false, Ordering::Release);
                        if failed {
                            return Err(invalid("component state/control failed; original detail retained in response"));
                        }
                    }
                }
            }
            let Some(mut item) = s.requests.pop() else {
                thread::sleep(Duration::from_micros(50));
                continue;
            };
            match item.kind {
                AUDIO => {
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
            *d = format!("{:?}: {}", error.kind(), error)
                .chars()
                .take(384)
                .collect();
        }
        session.phase = ERROR;
    }
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
    s.ack.store(6, Ordering::Release);
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
    let Some(_guard) = Guard::acquire() else {
        return 3;
    };
    if handle.is_null() || !(1..=256).contains(&max) {
        return 1;
    }
    if !ACTIVE.load(Ordering::Acquire).is_null() {
        return 3;
    }
    let result = std::panic::catch_unwind(|| {
        let (path, id) = binding()?;
        let session = Session::open_at(&path, id, max as usize, minor)?;
        let shared = Arc::new(Shared::new());
        shared.state_capable.store(minor == 4, Ordering::Release);
        if minor == 4 {
            shared.ack.store(17, Ordering::Release);
        }
        let peer = shared.clone();
        let t = thread::Builder::new()
            .name("ap3-transport".into())
            .spawn(move || worker(session, peer))?;
        Ok::<_, io::Error>(Box::new(Live {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            shared,
            callback: UnsafeCell::new(Callback::new()),
            worker: UnsafeCell::new(Some(t)),
            max: max as usize,
        }))
    });
    match result {
        Ok(Ok(value)) => {
            *handle = value.id;
            ACTIVE.store(Box::into_raw(value), Ordering::Release);
            0
        }
        Ok(Err(e)) => retain(&e) as u32,
        Err(_) => 4,
    }
}
// Caller is the SDK owner thread; it must not close concurrently. Arc retains
// worker storage throughout I/O. This does not touch Live's callback state or BUSY.
unsafe fn control(id: u64, op: u32, bytes: Vec<u8>) -> io::Result<Vec<u8>> {
    let p = ACTIVE.load(Ordering::Acquire);
    if p.is_null() || (*p).id != id {
        return Err(invalid("state handle"));
    }
    let s = (*p).shared.clone();
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
    let Some(_guard) = Guard::acquire() else {
        return 3;
    };
    let Some(l) = live(id) else {
        return 1;
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
    let Some(_guard) = Guard::acquire() else {
        return 3;
    };
    let Some(l) = live(id) else {
        return 1;
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
    let Some(_g) = Guard::acquire() else {
        return 3;
    };
    let Some(l) = live(id) else {
        return 1;
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
    let Some(_guard) = Guard::acquire() else {
        return 3;
    };
    let Some(l) = live(id) else {
        return 1;
    };
    let clean = matches!(l.shared.ack.load(Ordering::Acquire), 15 | 17)
        && l.shared.fault.load(Ordering::Acquire) == 0;
    if clean {
        if !l.shared.requests.push(Item::control(CLOSE, 0)) {
            l.shared.quit.store(true, Ordering::Release);
        }
    } else {
        l.shared.quit.store(true, Ordering::Release);
    }
    if let Some(t) = (&mut *l.worker.get()).take() {
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
    let p = ACTIVE.swap(std::ptr::null_mut(), Ordering::AcqRel);
    drop(Box::from_raw(p));
    if ok {
        0
    } else {
        2
    }
}
#[cfg(test)]
mod tests {
    use super::*;
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

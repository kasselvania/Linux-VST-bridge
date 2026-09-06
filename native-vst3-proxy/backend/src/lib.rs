//! Offline AP2 session. The caller supplies owned buffers; no DSP exists here.
#[cfg(test)]
mod commercial_tests;
mod instances;
mod observer;
mod performance;
mod preview;
mod queue;
mod queued;
mod recovery;
mod state;
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
struct Session {
    mapping: Option<Mapping>,
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
    2
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
        let prepared = Prepared::create(path, id)?;
        let (mapping, socket) =
            prepared.accept_while(minor, || preview::check_owner(&mut owner))?;
        let mut s = Self {
            mapping: Some(mapping),
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
            witness: if matches!(minor, 5 | 7) {
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
        let result = send_version(&mut self.socket, &f, 5, self.minor)
            .and_then(|_| receive_version(&mut self.socket, 10, self.minor));
        match result {
            Ok(reply)
                if reply.kind == kind + 1
                    && reply.session == f.session
                    && reply.sequence == f.sequence
                    && (kind == 20 && self.minor >= 6 && reply.payload.len() == 16
                        || reply.payload.is_empty()
                            && !(self.minor >= 3 && matches!(kind, 10 | 12))
                        || self.minor >= 3
                            && matches!(kind, 10 | 12)
                            && reply.payload == f.payload) =>
            {
                Ok(reply)
            }
            Ok(_) => {
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
    fn configure(&mut self, bytes: Vec<u8>) -> io::Result<Vec<u8>> {
        need(
            self.minor >= 6 && matches!(self.phase, 17 | 15),
            "setup requires inactive session",
        )?;
        performance::validate_wire(&bytes)?;
        self.sample_rate = f64::from_le_bytes(bytes[8..16].try_into().unwrap()) as u32;
        let reply = self.exchange(20, bytes)?;
        need(
            get(&reply.payload[12..16]) == 0 && get(&reply.payload[8..12]) & 1 == 1,
            "invalid setup response",
        )?;
        Ok(reply.payload)
    }
    fn activate(&mut self, maximum: usize, mode: u32) -> io::Result<()> {
        need(
            self.minor >= 4
                && matches!(self.phase, 17 | 15)
                && (1..=CAP).contains(&maximum)
                && if self.minor >= 6 {
                    matches!(mode, 0 | 2)
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
    fn process_positioned(
        &mut self,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
        timeline: (u64, u64),
        events: &[events::Event],
    ) -> io::Result<([[u32; CAP + 2]; 2], u64)> {
        need(
            self.minor >= 3 && timeline == (self.epoch, self.position),
            "queued audio epoch/position",
        )?;
        self.process_events(n, gain, silence, input, events)
    }
    fn process(
        &mut self,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
    ) -> io::Result<([[u32; CAP + 2]; 2], u64)> {
        self.process_events(n, gain, silence, input, &[])
    }
    fn process_events(
        &mut self,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
        events: &[events::Event],
    ) -> io::Result<([[u32; CAP + 2]; 2], u64)> {
        need(
            matches!(self.minor, 5 | 7) || events.is_empty(),
            "events require negotiated protocol",
        )?;
        need(
            !matches!(self.minor, 5 | 7) || gain.is_nan(),
            "commercial legacy gain refused",
        )?;
        need(
            self.phase == 11
                && self.state.slot == Slot::Writable
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
        self.armed |= self.minor == 6 || events.iter().any(|e| e.kind == events::NOTE_ON);
        self.trace = observer::Trace {
            sample_rate: self.sample_rate,
            armed: self.armed,
            epoch: self.epoch,
            position: self.position,
            frames: n as u64,
            sequence: self.state.next,
            started: Some(std::time::Instant::now()),
            ..Default::default()
        };
        let mut snapshot = [[0u32; CAP + 2]; 2];
        let mut poison = [POISON; CAP + 2];
        poison[0] = GUARD;
        poison[CAP + 1] = GUARD;
        for ch in 0..2 {
            snapshot[ch][0] = GUARD;
            snapshot[ch][CAP + 1] = GUARD;
            for i in 0..n {
                need(input[ch][i].is_finite(), "nonfinite input")?;
                snapshot[ch][i + 1] = input[ch][i].to_bits();
            }
        }
        let result = (|| {
            let map = self
                .mapping
                .as_mut()
                .ok_or_else(|| invalid("mapping absent"))?;
            for (ch, plane) in snapshot.iter().enumerate() {
                map.write_plane(INPUT, ch, plane)?;
                map.write_plane(OUTPUT, ch, &poison)?;
            }
            barrier();
            let mut request = if self.minor >= 4 {
                let mut payload = vec![0; 32];
                for (offset, value) in [
                    (0, n as u64),
                    (4, INPUT as u64),
                    (8, OUTPUT as u64),
                    (12, STRIDE as u64),
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
                Frame {
                    kind: PROCESS,
                    session: self.state.session,
                    sequence: self.state.next,
                    payload,
                }
            } else if self.minor >= 3 {
                self.state.process_sustained(n, gain, silence as u32)?
            } else {
                self.state.process(n, gain, silence as u32)?
            };
            if self.minor >= 3 {
                request.payload.extend_from_slice(&self.epoch.to_le_bytes());
                request
                    .payload
                    .extend_from_slice(&self.position.to_le_bytes());
            }
            if matches!(self.minor, 5 | 7) {
                request
                    .payload
                    .extend_from_slice(&events::encode(events, n)?);
            }
            self.trace.prepared = Some(std::time::Instant::now());
            send_version(&mut self.socket, &request, 5, self.minor)?;
            self.trace.sent = Some(std::time::Instant::now());
            let mut reply = receive_version(&mut self.socket, 5, self.minor)?;
            self.trace.replied = Some(std::time::Instant::now());
            if self.minor >= 3 {
                need(
                    reply.payload.len() == if self.minor >= 6 { 40 } else { 32 }
                        && get(&reply.payload[16..24]) == self.epoch
                        && get(&reply.payload[24..32]) == self.position,
                    "Done epoch/position differs",
                )?;
                if self.minor >= 6 {
                    self.trace.process_ns = Some(get(&reply.payload[32..40]));
                }
                reply.payload.truncate(16);
            }
            let flags = self.state.done(&reply)?;
            barrier();
            let output = [map.plane(OUTPUT, 0)?, map.plane(OUTPUT, 1)?];
            need(flags & !3 == 0, "output flags")?;
            for ch in 0..2 {
                need(map.plane(INPUT, ch)? == snapshot[ch], "input changed")?;
                need(
                    output[ch][0] == GUARD
                        && output[ch][CAP + 1] == GUARD
                        && output[ch][n + 1..CAP + 1].iter().all(|&x| x == POISON),
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
}

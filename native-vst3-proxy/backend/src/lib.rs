//! Offline AP2 session. The caller supplies owned buffers; no DSP exists here.
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
const MINOR: u64 = 2;
struct Session {
    mapping: Option<Mapping>,
    socket: TcpStream,
    state: ClientState,
    phase: u16,
    max: usize,
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
fn binding() -> io::Result<(PathBuf, [u8; 16])> {
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
    Ok((path, session))
}
impl Session {
    fn exchange(&mut self, kind: u16, payload: Vec<u8>) -> io::Result<Frame> {
        need(self.phase != ERROR, "failed session")?;
        let f = Frame {
            kind,
            session: self.state.session,
            sequence: self.state.next,
            payload,
        };
        let result = send_version(&mut self.socket, &f, 5, MINOR)
            .and_then(|_| receive_version(&mut self.socket, 10, MINOR));
        match result {
            Ok(reply)
                if reply.kind == kind + 1
                    && reply.session == f.session
                    && reply.sequence == f.sequence
                    && reply.payload.is_empty() =>
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
    fn transition(&mut self, op: u16) -> io::Result<()> {
        need(
            matches!((self.phase, op), (9, 10) | (11, 12) | (13, 14)),
            "lifecycle order",
        )?;
        self.exchange(op, vec![])?;
        self.phase = op + 1;
        Ok(())
    }
    fn process(
        &mut self,
        n: usize,
        gain: f64,
        silence: u64,
        input: [&[f32]; 2],
    ) -> io::Result<([[u32; CAP + 2]; 2], u64)> {
        need(
            self.phase == 11
                && self.state.slot == Slot::Writable
                && self.state.next <= 64
                && n > 0
                && n <= self.max
                && silence <= 3,
            "process state/extent",
        )?;
        need(gain.is_finite() && (0.0..=1.0).contains(&gain), "gain")?;
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
            let request = self.state.process(n, gain, silence as u32)?;
            send_version(&mut self.socket, &request, 5, MINOR)?;
            let reply = receive_version(&mut self.socket, 5, MINOR)?;
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
                for &bits in &output[ch][1..=n] {
                    let sample = f32::from_bits(bits);
                    need(
                        sample.is_finite() && ((flags & (1 << ch)) == 0 || sample == 0.0),
                        "invalid output claim",
                    )?;
                }
            }
            Ok((output, flags))
        })();
        if result.is_err() {
            self.phase = ERROR;
            self.state.failed();
        }
        result
    }
    fn close(mut self) -> io::Result<()> {
        let result = if self.phase == 15 {
            let f = self.state.close()?;
            send_version(&mut self.socket, &f, 5, MINOR)
                .and_then(|_| receive_version(&mut self.socket, 10, MINOR))
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
        let result = (|| {
            let (path, id) = binding()?;
            let prepared = Prepared::create(&path, id)?;
            let (mapping, socket) = prepared.accept(MINOR)?;
            let mut s = Session {
                mapping: Some(mapping),
                socket,
                state: ClientState {
                    session: id,
                    next: 1,
                    slot: Slot::Writable,
                },
                phase: 8,
                max: max as usize,
            };
            s.exchange(8, max.to_le_bytes().to_vec())?;
            s.phase = 9;
            Ok::<_, io::Error>(s)
        })();
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

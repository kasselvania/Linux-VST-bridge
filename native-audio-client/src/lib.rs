pub mod endpoint;
pub mod events;
pub mod mapping;
// AP1 bounded wire codec and Linux-owned slot state. No external dependencies.
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};
pub const HEADER: usize = 56;
pub const CAP: usize = 256;
pub const STRIDE: usize = 1032;
pub const INPUT: usize = 64;
pub const OUTPUT: usize = 2128;
pub const MAP_BYTES: usize = 4192;
pub const GUARD: u32 = 0x4b123456;
pub const POISON: u32 = 0x7fc12345;
pub const WITNESS: u64 = 0x8d396b274e105ac3;
pub const HELLO: u16 = 1;
pub const READY: u16 = 2;
pub const PROCESS: u16 = 3;
pub const DONE: u16 = 4;
pub const CLOSE: u16 = 5;
pub const CLOSED: u16 = 6;
pub const ERROR: u16 = 7;
pub fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
pub fn need(b: bool, s: &str) -> io::Result<()> {
    if b {
        Ok(())
    } else {
        Err(invalid(s))
    }
}
pub fn get(b: &[u8]) -> u64 {
    b.iter()
        .enumerate()
        .fold(0, |n, (i, v)| n | ((*v as u64) << (i * 8)))
}
pub fn put(b: &mut [u8], v: u64) {
    for (i, x) in b.iter_mut().enumerate() {
        *x = (v >> (i * 8)) as u8;
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub kind: u16,
    pub session: [u8; 16],
    pub sequence: u64,
    pub payload: Vec<u8>,
}
impl Frame {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        self.encode_version(1)
    }
    pub fn encode_version(&self, minor: u64) -> io::Result<Vec<u8>> {
        need(
            (1..=if minor >= 6 {
                21
            } else if minor >= 4 {
                19
            } else if minor >= 2 {
                15
            } else {
                7
            })
                .contains(&self.kind)
                && (1..=7).contains(&minor)
                && self.payload.len()
                    <= if minor >= 4 && matches!(self.kind, 17..=19) {
                        1 << 20
                    } else if matches!(minor, 5 | 7) && self.kind == PROCESS {
                        8248
                    } else {
                        4040
                    },
            "frame kind/length",
        )?;
        let mut b = vec![0; HEADER + self.payload.len()];
        put(&mut b[0..4], 0x3141504c);
        put(&mut b[4..6], 1);
        put(&mut b[6..8], minor);
        put(&mut b[8..10], self.kind as u64);
        put(&mut b[12..16], self.payload.len() as u64);
        b[16..32].copy_from_slice(&self.session);
        put(&mut b[32..40], 1);
        put(&mut b[40..48], self.sequence);
        b[56..].copy_from_slice(&self.payload);
        Ok(b)
    }
    pub fn decode(b: &[u8]) -> io::Result<Self> {
        Self::decode_version(b, 1)
    }
    pub fn decode_version(b: &[u8], minor: u64) -> io::Result<Self> {
        need(b.len() >= HEADER, "truncated header")?;
        let n = payload_length_version(&b[..HEADER], minor)?;
        need(b.len() == HEADER + n, "truncated/extra payload")?;
        Ok(Self {
            kind: get(&b[8..10]) as u16,
            session: b[16..32].try_into().unwrap(),
            sequence: get(&b[40..48]),
            payload: b[56..].to_vec(),
        })
    }
}
pub fn payload_length(b: &[u8]) -> io::Result<usize> {
    payload_length_version(b, 1)
}
pub fn payload_length_version(b: &[u8], minor: u64) -> io::Result<usize> {
    need(
        b.len() == HEADER
            && get(&b[0..4]) == 0x3141504c
            && get(&b[4..6]) == 1
            && get(&b[6..8]) == minor
            && (1..=7).contains(&minor)
            && get(&b[10..12]) == 0,
        "protocol version/header",
    )?;
    need(
        (1..=if minor >= 6 {
            21
        } else if minor >= 4 {
            19
        } else if minor >= 2 {
            15
        } else {
            7
        })
            .contains(&get(&b[8..10]))
            && get(&b[32..40]) == 1
            && get(&b[48..56]) == 0,
        "kind/instance/parent",
    )?;
    let n = get(&b[12..16]);
    need(
        n <= if minor >= 4 && matches!(get(&b[8..10]), 17..=19) {
            1 << 20
        } else if matches!(minor, 5 | 7) && get(&b[8..10]) == PROCESS as u64 {
            8248
        } else {
            4040
        },
        "frame length",
    )?;
    Ok(n as usize)
}
pub fn read_frame<R: Read>(r: &mut R) -> io::Result<Frame> {
    read_frame_version(r, 1)
}
pub fn read_frame_version<R: Read>(r: &mut R, minor: u64) -> io::Result<Frame> {
    let mut b = vec![0; HEADER];
    r.read_exact(&mut b)?;
    let n = payload_length_version(&b, minor)?;
    b.resize(HEADER + n, 0);
    r.read_exact(&mut b[HEADER..])?;
    Frame::decode_version(&b, minor)
}
// Each socket operation shares one absolute deadline, including fragmentation.
pub struct Deadline<'a> {
    pub stream: &'a mut TcpStream,
    pub end: Instant,
}
impl Read for Deadline<'_> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let left = self
            .end
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "control deadline"))?;
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(b)
    }
}
impl Write for Deadline<'_> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let left = self
            .end
            .checked_duration_since(Instant::now())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "control deadline"))?;
        self.stream.set_write_timeout(Some(left))?;
        self.stream.write(b)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
pub fn receive(s: &mut TcpStream, seconds: u64) -> io::Result<Frame> {
    read_frame(&mut Deadline {
        stream: s,
        end: Instant::now() + Duration::from_secs(seconds),
    })
}
pub fn send(s: &mut TcpStream, f: &Frame, seconds: u64) -> io::Result<()> {
    Deadline {
        stream: s,
        end: Instant::now() + Duration::from_secs(seconds),
    }
    .write_all(&f.encode()?)
}
#[derive(Debug, PartialEq)]
pub enum Slot {
    Writable,
    Outstanding { sequence: u64, frames: usize },
    Failed,
    Closed,
}
pub struct ClientState {
    pub session: [u8; 16],
    pub next: u64,
    pub slot: Slot,
}
impl ClientState {
    pub fn process(&mut self, frames: usize, gain: f64, silence: u32) -> io::Result<Frame> {
        self.process_limited(frames, gain, silence, 64)
    }
    pub fn process_sustained(
        &mut self,
        frames: usize,
        gain: f64,
        silence: u32,
    ) -> io::Result<Frame> {
        self.process_limited(frames, gain, silence, u64::MAX - 1)
    }
    fn process_limited(
        &mut self,
        frames: usize,
        gain: f64,
        silence: u32,
        limit: u64,
    ) -> io::Result<Frame> {
        need(
            self.slot == Slot::Writable
                && self.next <= limit
                && (1..=CAP).contains(&frames)
                && silence <= 3
                && gain.is_finite()
                && (0.0..=1.0).contains(&gain),
            "request ownership/extent",
        )?;
        let mut p = vec![0; 32];
        for (o, v) in [
            (0, frames as u64),
            (4, INPUT as u64),
            (8, OUTPUT as u64),
            (12, STRIDE as u64),
            (24, silence as u64),
        ] {
            put(&mut p[o..o + 4], v);
        }
        put(&mut p[16..24], gain.to_bits());
        self.slot = Slot::Outstanding {
            sequence: self.next,
            frames,
        };
        Ok(Frame {
            kind: PROCESS,
            session: self.session,
            sequence: self.next,
            payload: p,
        })
    }
    pub fn done(&mut self, f: &Frame) -> io::Result<u64> {
        let ok = match self.slot {
            Slot::Outstanding { sequence, frames } => {
                f.kind == DONE
                    && f.session == self.session
                    && f.sequence == sequence
                    && f.payload.len() == 16
                    && get(&f.payload[..4]) == frames as u64
                    && get(&f.payload[4..8]) == OUTPUT as u64
            }
            _ => false,
        };
        if !ok {
            self.slot = Slot::Failed;
            return Err(invalid("stale/wrong Done; slot unusable"));
        }
        self.slot = Slot::Writable;
        self.next += 1;
        Ok(get(&f.payload[8..16]))
    }
    pub fn failed(&mut self) {
        self.slot = Slot::Failed;
    }
    pub fn close(&mut self) -> io::Result<Frame> {
        need(
            self.slot == Slot::Writable,
            "close while outstanding/failed",
        )?;
        self.slot = Slot::Closed;
        Ok(Frame {
            kind: CLOSE,
            session: self.session,
            sequence: self.next,
            payload: vec![],
        })
    }
    pub fn closed(&self, f: &Frame) -> io::Result<()> {
        need(
            self.slot == Slot::Closed
                && f.kind == CLOSED
                && f.session == self.session
                && f.sequence == self.next
                && f.payload.is_empty(),
            "Closed differs",
        )
    }
}
pub const LENGTHS: [usize; 10] = [1, 16, 63, 256, 16, 63, 1, 256, 16, 63];
pub const GAINS: [f64; 10] = [0.5, 0.25, 0.75, 0.5, 0.75, 0.25, 0.5, 0.75, 0.0, 0.5];
pub const SILENCE: [u32; 10] = [0, 0, 0, 0, 0, 3, 0, 0, 0, 1];
pub fn recipe(seed: u64, b: usize, ch: usize) -> [u32; CAP + 2] {
    let mut a = [0u32; CAP + 2];
    a[0] = GUARD;
    a[CAP + 1] = GUARD;
    let mut x = seed ^ ((b as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15)) ^ ((ch as u64 + 1) << 48);
    for v in &mut a[1..=LENGTHS[b]] {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let numerator = (x % 129) as i32 - 64;
        *v = if SILENCE[b] & (1 << ch) != 0 {
            0
        } else {
            (numerator as f32 / 128.0).to_bits()
        };
    }
    // Make the added valid cases discriminating for every post-Ready seed.
    if b == 8 {
        a[1] = ((ch + 1) as f32 / 4.0).to_bits();
    }
    if b == 9 && ch == 1 {
        a[1] = (-0.5f32).to_bits();
    }
    a
}
pub fn compare(
    input: &[[u32; CAP + 2]; 2],
    output: &[[u32; CAP + 2]; 2],
    frames: usize,
    gain: f64,
    output_silence: u64,
) -> io::Result<f64> {
    need(output_silence & !3 == 0, "invalid output silence bits")?;
    let mut max: f64 = 0.0;
    for ch in 0..2 {
        need(
            output[ch][0] == GUARD && output[ch][CAP + 1] == GUARD,
            "output guard",
        )?;
        need(
            output[ch][frames + 1..CAP + 1].iter().all(|&x| x == POISON),
            "unused output modified",
        )?;
        for i in 1..=frames {
            let actual = f32::from_bits(output[ch][i]) as f64;
            let expected = f32::from_bits(input[ch][i]) as f64 * gain;
            need(actual.is_finite(), "nonfinite/unwritten output")?;
            need(
                output_silence & (1 << ch) == 0 || actual == 0.0,
                "output silence claim has nonzero sample",
            )?;
            max = max.max((actual - expected).abs());
        }
    }
    need(max == 0.0, "mapped sample mismatch")?;
    Ok(max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn state() -> ClientState {
        ClientState {
            session: [7; 16],
            next: 1,
            slot: Slot::Writable,
        }
    }
    struct Fragments {
        data: Cursor<Vec<u8>>,
    }
    impl Read for Fragments {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            let n = b.len().min(3);
            self.data.read(&mut b[..n])
        }
    }
    #[test]
    fn fragmented_coalesced_and_truncated() {
        let mut s = state();
        let frame = s.process(63, 0.75, 0).unwrap();
        let bytes = frame.encode().unwrap();
        let mut r = Fragments {
            data: Cursor::new([bytes.clone(), bytes.clone()].concat()),
        };
        assert_eq!(read_frame(&mut r).unwrap(), frame);
        assert_eq!(read_frame(&mut r).unwrap(), frame);
        for n in 0..bytes.len() {
            assert!(read_frame(&mut Cursor::new(&bytes[..n])).is_err());
        }
    }
    #[test]
    fn bad_header() {
        let b = state().process(16, 0.5, 0).unwrap().encode().unwrap();
        for offset in [0, 4, 6, 10, 12, 32, 48] {
            let mut bad = b.clone();
            bad[offset] = 255;
            assert!(Frame::decode(&bad).is_err());
        }
    }
    #[test]
    fn wrong_done_never_reuses() {
        for fault in 0..4 {
            let mut s = state();
            s.process(16, 0.5, 0).unwrap();
            assert!(s.process(1, 0.5, 0).is_err());
            let mut p = vec![0; 16];
            put(&mut p[..4], 16);
            put(&mut p[4..8], OUTPUT as u64);
            let mut done = Frame {
                kind: DONE,
                session: s.session,
                sequence: 1,
                payload: p,
            };
            match fault {
                0 => done.sequence = 2,
                1 => done.session[0] = 8,
                2 => done.payload[0] = 1,
                _ => done.kind = ERROR,
            };
            assert!(s.done(&done).is_err());
            assert_eq!(s.slot, Slot::Failed);
            assert!(s.process(16, 0.5, 0).is_err());
            assert!(s.close().is_err());
        }
    }
    #[test]
    fn duplicate_done_and_lost_response() {
        let mut s = state();
        s.process(1, 0.5, 0).unwrap();
        let mut p = vec![0; 16];
        put(&mut p[..4], 1);
        put(&mut p[4..8], OUTPUT as u64);
        let done = Frame {
            kind: DONE,
            session: s.session,
            sequence: 1,
            payload: p,
        };
        s.done(&done).unwrap();
        assert!(s.done(&done).is_err());
        assert_eq!(s.slot, Slot::Failed);
        let mut s = state();
        s.process(1, 0.5, 0).unwrap();
        s.failed();
        assert!(s.close().is_err());
    }
    #[test]
    fn numerical_corruption() {
        let input = [recipe(7381, 3, 0), recipe(7381, 3, 1)];
        let mut output = [[POISON; CAP + 2]; 2];
        for ch in 0..2 {
            output[ch][0] = GUARD;
            output[ch][CAP + 1] = GUARD;
            for i in 1..=CAP {
                output[ch][i] = (f32::from_bits(input[ch][i]) * 0.5).to_bits();
            }
        }
        assert_eq!(compare(&input, &output, 256, 0.5, 0).unwrap(), 0.0);
        for ch in 0..2 {
            for i in 1..=CAP {
                let mut bad = output;
                bad[ch][i] = POISON;
                assert!(compare(&input, &bad, 256, 0.5, 0).is_err());
            }
        }
        output.swap(0, 1);
        assert!(compare(&input, &output, 256, 0.5, 0).is_err());
    }
    #[test]
    fn partial_write_error() {
        struct Broken(usize);
        impl Write for Broken {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                if self.0 == 0 {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "injected"));
                }
                self.0 -= 1;
                Ok(b.len().min(2))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut s = state();
        let frame = s.process(16, 0.5, 0).unwrap();
        assert!(Broken(2).write_all(&frame.encode().unwrap()).is_err());
        s.failed();
        assert!(s.process(16, 0.5, 0).is_err());
    }
    #[test]
    fn real_peer_disconnect_and_deadline() {
        use std::net::TcpListener;
        let l = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let mut client = TcpStream::connect(l.local_addr().unwrap()).unwrap();
        let (peer, _) = l.accept().unwrap();
        let mut deadline = Deadline {
            stream: &mut client,
            end: Instant::now() + Duration::from_millis(20),
        };
        assert!(read_frame(&mut deadline).is_err());
        drop(peer);
        assert!(receive(&mut client, 1).is_err());
    }
}

#[cfg(test)]
#[test]
fn cross_language_vector() {
    let hex = include_str!("../../tools/ap1-tests/golden.txt").trim();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let mut state = ClientState {
        session: std::array::from_fn(|i| i as u8),
        next: 7,
        slot: Slot::Writable,
    };
    let frame = state.process(63, 0.75, 0).unwrap();
    assert_eq!(frame.encode().unwrap(), bytes);
    assert_eq!(Frame::decode(&bytes).unwrap(), frame);
}

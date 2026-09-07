//! Owned, bounded VST3 process results. No SDK pointers cross the worker boundary.
use ap1_native_client::{get, need};
use std::io;
pub const EVENTS: usize = 64;
pub const POINTS: usize = 128;
pub const PAYLOAD: usize = 4096;
pub const EVENT_PAYLOAD: usize = 512;
const PENDING_EVENTS: usize = 512;
const PENDING_POINTS: usize = 1024;
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Event {
    pub offset: i32,
    pub bus: i32,
    pub ppq: f64,
    pub flags: u16,
    pub kind: u16,
    pub payload_offset: u32,
    pub payload_size: u32,
    pub a: i32,
    pub b: i32,
    pub c: i32,
    pub d: u32,
    pub reserved: u32,
    pub value: u64,
    pub extra: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Point {
    pub offset: i32,
    pub id: u32,
    pub value: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Packet {
    pub events: u32,
    pub points: u32,
    pub bytes: u32,
    pub reserved: u32,
    pub event: [Event; EVENTS],
    pub point: [Point; POINTS],
    pub payload: [u8; PAYLOAD],
}
impl Default for Packet {
    fn default() -> Self {
        Self {
            events: 0,
            points: 0,
            bytes: 0,
            reserved: 0,
            event: [Event::default(); EVENTS],
            point: [Point::default(); POINTS],
            payload: [0; PAYLOAD],
        }
    }
}
impl Packet {
    pub fn decode(b: &[u8], frames: usize) -> io::Result<Self> {
        need(b.len() >= 16, "process result header")?;
        let (ne, np, nb) = (
            get(&b[..4]) as usize,
            get(&b[4..8]) as usize,
            get(&b[8..12]) as usize,
        );
        need(
            ne <= EVENTS
                && np <= POINTS
                && nb <= PAYLOAD
                && get(&b[12..16]) == 0
                && b.len() == 16 + ne * 64 + np * 16 + nb,
            "process result capacities",
        )?;
        let mut p = Self {
            events: ne as u32,
            points: np as u32,
            bytes: nb as u32,
            ..Self::default()
        };
        p.payload[..nb].copy_from_slice(&b[16 + ne * 64 + np * 16..]);
        for i in 0..ne {
            let r = &b[16 + i * 64..16 + (i + 1) * 64];
            let e = Event {
                offset: get(&r[..4]) as i32,
                bus: get(&r[4..8]) as i32,
                ppq: f64::from_bits(get(&r[8..16])),
                flags: get(&r[16..18]) as u16,
                kind: get(&r[18..20]) as u16,
                payload_offset: get(&r[20..24]) as u32,
                payload_size: get(&r[24..28]) as u32,
                a: get(&r[28..32]) as i32,
                b: get(&r[32..36]) as i32,
                c: get(&r[36..40]) as i32,
                d: get(&r[40..44]) as u32,
                reserved: get(&r[44..48]) as u32,
                value: get(&r[48..56]),
                extra: get(&r[56..64]),
            };
            need(
                e.valid(frames, &p.payload[..nb]),
                "malformed returned event",
            )?;
            p.event[i] = e;
        }
        for i in 0..np {
            let r = &b[16 + ne * 64 + i * 16..16 + ne * 64 + (i + 1) * 16];
            let q = Point {
                offset: get(&r[..4]) as i32,
                id: get(&r[4..8]) as u32,
                value: f64::from_bits(get(&r[8..16])),
            };
            need(
                q.offset >= 0 && (q.offset as usize) < frames.max(1) && normal(q.value),
                "malformed returned parameter",
            )?;
            p.point[i] = q;
        }
        Ok(p)
    }
}
fn normal(v: f64) -> bool {
    v.is_finite() && (0.0..=1.0).contains(&v)
}
impl Event {
    fn valid(&self, frames: usize, payload: &[u8]) -> bool {
        let e = self;
        let start = e.payload_offset as usize;
        let size = e.payload_size as usize;
        if e.offset < 0
            || e.offset as usize >= frames.max(1)
            || !(0..8).contains(&e.bus)
            || !e.ppq.is_finite()
            || e.reserved != 0
            || size > EVENT_PAYLOAD
            || start > payload.len()
            || size > payload.len() - start
            || !start.is_multiple_of(2)
        {
            return false;
        }
        let f = f32::from_bits(e.value as u32);
        let t = f32::from_bits(e.extra as u32);
        let note = (0..16).contains(&e.b)
            && (0..128).contains(&e.c)
            && e.value <= u32::MAX as u64
            && normal(f as f64)
            && e.extra <= u32::MAX as u64
            && t.is_finite();
        let text = size >= 2
            && size.is_multiple_of(2)
            && payload[start + size - 2..start + size] == [0, 0];
        match e.kind {
            0 => size == 0 && note,
            1 => size == 0 && note && e.d == 0,
            2 => e.d == 0 && e.a == 0 && e.b == 0 && e.c == 0 && e.value == 0 && e.extra == 0,
            3 => size == 0 && note && e.d == 0 && e.extra == 0,
            4 => {
                size == 0 && e.b == 0 && e.c == 0 && e.extra == 0 && normal(f64::from_bits(e.value))
            }
            5 => text && e.b == 0 && e.c == 0 && e.value == 0 && e.extra == 0,
            6 => {
                text && (0..128).contains(&e.a)
                    && (0..128).contains(&e.b)
                    && (i16::MIN as i32..=i16::MAX as i32).contains(&e.c)
                    && e.d == 0
                    && e.value == 0
                    && e.extra == 0
            }
            7 => {
                text && (0..128).contains(&e.a)
                    && e.b == 0
                    && (i16::MIN as i32..=i16::MAX as i32).contains(&e.c)
                    && e.d == 0
                    && e.value == 0
                    && e.extra == 0
            }
            8 => size == 0 && e.b == 0 && e.c == 0 && e.extra == 0,
            65535 => {
                size == 0
                    && (0..256).contains(&e.a)
                    && (0..16).contains(&e.b)
                    && (0..128).contains(&e.c)
                    && e.d <= 127
                    && e.value == 0
                    && e.extra == 0
            }
            _ => false,
        }
    }
}
#[derive(Clone, Copy)]
struct PendingEvent {
    due: u64,
    flush: bool,
    value: Event,
    payload: [u8; EVENT_PAYLOAD],
}
#[derive(Clone, Copy)]
struct PendingPoint {
    due: u64,
    flush: bool,
    value: Point,
}
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub late_events: u64,
    pub late_points: u64,
    pub discarded_on_reset: u64,
    pub pending_events: u64,
    pub pending_points: u64,
}
pub struct Pending {
    events: Vec<PendingEvent>,
    points: Vec<PendingPoint>,
    pub stats: Stats,
    start: u64,
    end: u64,
}
impl Pending {
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(PENDING_EVENTS),
            points: Vec::with_capacity(PENDING_POINTS),
            stats: Stats::default(),
            start: 0,
            end: 0,
        }
    }
    pub fn reset(&mut self) {
        self.stats.discarded_on_reset += (self.events.len() + self.points.len()) as u64;
        self.events.clear();
        self.points.clear();
        self.start = 0;
        self.end = 0;
    }
    pub fn window(&mut self, start: u64, n: usize) {
        self.start = start;
        self.end = start.saturating_add(n as u64);
    }
    pub fn append(&mut self, p: &Packet, position: u64, frames: usize, delay: u64) -> bool {
        if self.events.len() + p.events as usize > PENDING_EVENTS
            || self.points.len() + p.points as usize > PENDING_POINTS
        {
            return false;
        }
        // Stable insertion by presentation time retains producer order at equal offsets.
        for e in &p.event[..p.events as usize] {
            let Some(due) = position
                .checked_add(e.offset as u64)
                .and_then(|p| p.checked_add(delay))
            else {
                return false;
            };
            let mut event = PendingEvent {
                due,
                flush: frames == 0,
                value: *e,
                payload: [0; EVENT_PAYLOAD],
            };
            event.payload[..e.payload_size as usize].copy_from_slice(
                &p.payload[e.payload_offset as usize..(e.payload_offset + e.payload_size) as usize],
            );
            event.value.payload_offset = 0;
            let i = self.events.partition_point(|x| x.due <= due);
            self.events.insert(i, event);
        }
        for p in &p.point[..p.points as usize] {
            let Some(due) = position
                .checked_add(p.offset as u64)
                .and_then(|p| p.checked_add(delay))
            else {
                return false;
            };
            let i = self.points.partition_point(|x| x.due <= due);
            self.points.insert(
                i,
                PendingPoint {
                    due,
                    flush: frames == 0,
                    value: *p,
                },
            );
        }
        true
    }
    fn eligible(&self, due: u64, flush: bool) -> bool {
        flush || due < self.end || (self.start == self.end && due <= self.start)
    }
    pub fn take(&mut self, out: &mut Packet) {
        out.events = 0;
        out.points = 0;
        out.bytes = 0;
        out.reserved = 0;
        // Flush packets can be eligible without advancing the sample clock.
        let mut i = 0;
        while i < self.events.len() && out.events < EVENTS as u32 {
            let e = &self.events[i];
            if !self.eligible(e.due, e.flush) {
                i += 1;
                continue;
            }
            let start = (out.bytes + 1) & !1;
            let size = e.value.payload_size;
            if start + size > PAYLOAD as u32 {
                break;
            }
            let mut e = self.events.remove(i);
            if e.due < self.start && !e.flush {
                self.stats.late_events += 1;
            }
            e.value.offset = if e.flush {
                0
            } else {
                e.due.saturating_sub(self.start) as i32
            };
            e.value.payload_offset = start;
            out.event[out.events as usize] = e.value;
            out.events += 1;
            if start > out.bytes {
                out.payload[out.bytes as usize] = 0;
            }
            out.payload[start as usize..(start + size) as usize]
                .copy_from_slice(&e.payload[..size as usize]);
            out.bytes = start + size;
        }
        let mut i = 0;
        while i < self.points.len() && out.points < POINTS as u32 {
            let p = &self.points[i];
            if !self.eligible(p.due, p.flush) {
                i += 1;
                continue;
            }
            let mut p = self.points.remove(i);
            if p.due < self.start && !p.flush {
                self.stats.late_points += 1;
            }
            p.value.offset = if p.flush {
                0
            } else {
                p.due.saturating_sub(self.start) as i32
            };
            out.point[out.points as usize] = p.value;
            out.points += 1;
        }
    }
    pub fn stats(&self) -> Stats {
        Stats {
            pending_events: self.events.len() as u64,
            pending_points: self.points.len() as u64,
            ..self.stats
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn abi_and_boundaries() {
        assert_eq!(std::mem::size_of::<Event>(), 64);
        assert_eq!(std::mem::size_of::<Packet>(), 10256);
        assert!(Packet::decode(&[0; 16], 0).is_ok());
        let mut b = [0; 16];
        b[0] = 65;
        assert!(Packet::decode(&b, 0).is_err());
    }
    #[test]
    fn late_note_off_survives_expired_audio_and_preserves_order() {
        let mut p = Packet {
            events: 2,
            ..Default::default()
        };
        p.event[0] = Event {
            kind: 1,
            offset: 7,
            a: -7,
            ..Default::default()
        };
        p.event[1] = Event {
            kind: 65535,
            offset: 7,
            a: 64,
            ..Default::default()
        };
        let mut q = Pending::new();
        assert!(q.append(&p, 100, 64, 256));
        let mut out = Packet::default();
        q.window(300, 64);
        q.take(&mut out);
        assert_eq!(out.events, 2);
        assert_eq!(out.event[0].offset, 63);
        assert_eq!(out.event[1].kind, 65535);
        q.take(&mut out);
        assert_eq!(out.events, 0);
        assert!(q.append(&p, 100, 64, 256));
        q.window(600, 128);
        q.take(&mut out);
        assert_eq!(out.event[0].offset, 0);
        assert_eq!(out.event[0].a, -7);
        assert_eq!(q.stats.late_events, 2);
    }
    #[test]
    fn flush_owned_payload_capacity_and_reset() {
        let mut p = Packet {
            events: 1,
            ..Default::default()
        };
        p.event[0] = Event {
            kind: 2,
            payload_size: 3,
            ..Default::default()
        };
        p.payload[..3].copy_from_slice(&[0xf0, 1, 0xf7]);
        p.points = 1;
        p.point[0] = Point {
            id: 9,
            value: 0.5,
            ..Default::default()
        };
        let mut q = Pending::new();
        assert!(q.append(&p, 0, 0, 512));
        p.payload.fill(0);
        q.window(0, 0);
        let mut out = Packet::default();
        q.take(&mut out);
        assert_eq!(out.points, 1);
        assert_eq!(&out.payload[..3], &[0xf0, 1, 0xf7]);
        for _ in 0..512 {
            assert!(q.append(&p, 0, 32, 512));
        }
        assert!(!q.append(&p, 0, 32, 512));
        q.reset();
        assert_eq!(q.stats.discarded_on_reset, 1024);
        q.window(1000, 32);
        q.take(&mut out);
        assert_eq!(out.events, 0);
    }
}

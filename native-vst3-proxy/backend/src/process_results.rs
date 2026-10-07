//! Owned, bounded VST3 process results. No SDK pointers cross the worker boundary.
use ap1_native_client::get;
fn rt_need(ok: bool, reason: &'static str) -> Result<(), &'static str> {
    if ok { Ok(()) } else { Err(reason) }
}
use std::io;
pub const EVENTS: usize = 64;
pub const POINTS: usize = 128;
pub const PAYLOAD: usize = 4096;
pub const EVENT_PAYLOAD: usize = 512;
#[cfg(test)]
const PENDING_EVENTS: usize = 512;
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
        Self::decode_rt(b, frames).map_err(ap1_native_client::invalid)
    }
    pub(crate) fn decode_rt(b: &[u8], frames: usize) -> Result<Self, &'static str> {
        rt_need(b.len() >= 16, "process result header")?;
        let (ne, np, nb) = (
            get(&b[..4]) as usize,
            get(&b[4..8]) as usize,
            get(&b[8..12]) as usize,
        );
        rt_need(
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
            rt_need(
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
            rt_need(
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
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Stats {
    pub late_events: u64,
    pub late_points: u64,
    pub discarded_on_reset: u64,
    pub pending_events: u64,
    pub pending_points: u64,
}
#[derive(Clone, Copy)]
struct PendingPacket {
    packet: Packet,
    position: u64,
    delay: u64,
    serial: u64,
    flush: bool,
    event_order: [u8; EVENTS],
    point_order: [u8; POINTS],
    event_next: usize,
    point_next: usize,
}
impl PendingPacket {
    fn empty() -> Self {
        Self { packet: Packet::default(), position: 0, delay: 0, serial: 0, flush: false,
            event_order: [0; EVENTS], point_order: [0; POINTS], event_next: 0, point_next: 0 }
    }
    fn due(&self, kind: Kind) -> u64 {
        let offset = match kind {
            Kind::Event => self.packet.event[self.event_order[self.event_next] as usize].offset,
            Kind::Point => self.packet.point[self.point_order[self.point_next] as usize].offset,
        };
        self.position + self.delay + offset as u64
    }
}
#[derive(Clone, Copy)]
enum Kind { Event, Point }
struct Heap { indices: Vec<usize> }
impl Heap {
    fn new(capacity: usize) -> Self { Self { indices: crate::queue::preallocated(capacity) } }
    fn less(a: usize, b: usize, slots: &[PendingPacket], kind: Kind) -> bool {
        (slots[a].due(kind), slots[a].serial) < (slots[b].due(kind), slots[b].serial)
    }
    fn peek(&self) -> Option<usize> { self.indices.first().copied() }
    fn push(&mut self, index: usize, slots: &[PendingPacket], kind: Kind) {
        debug_assert!(self.indices.len() < self.indices.capacity());
        self.indices.push(index);
        let mut at = self.indices.len() - 1;
        while at > 0 {
            let parent = (at - 1) / 2;
            if !Self::less(self.indices[at], self.indices[parent], slots, kind) { break; }
            self.indices.swap(at, parent); at = parent;
        }
    }
    fn pop(&mut self, slots: &[PendingPacket], kind: Kind) -> usize {
        let result = self.indices[0];
        let last = self.indices.pop().unwrap();
        if !self.indices.is_empty() {
            self.indices[0] = last;
            let mut at = 0;
            loop {
                let left = at * 2 + 1;
                if left >= self.indices.len() { break; }
                let right = left + 1;
                let child = if right < self.indices.len()
                    && Self::less(self.indices[right], self.indices[left], slots, kind) { right } else { left };
                if !Self::less(self.indices[child], self.indices[at], slots, kind) { break; }
                self.indices.swap(at, child); at = child;
            }
        }
        result
    }
}
/// One owned bounded packet per retained operation. This shares its 4096-byte
/// payload among that operation's <=64 events instead of reserving 512 bytes
/// for every event across every delayed N=1 operation. The owner prepares
/// capacity D+queue_depth+1 for Buffered or 1 for exact D=0 presentation,
/// plus the bounded internal startup synchronization operations.
/// Storage bytes = capacity*(sizeof(PendingPacket)+4*sizeof(usize)); the heaps
/// contain one next-result index per packet, never one node per event/point.
/// Sorting <=64/128 indices and heap depth <=ceil(log2(capacity)) bound work.
/// Event and point order remains stable at equal presentation positions.
pub struct Pending {
    slots: Box<[PendingPacket]>,
    free: Vec<usize>,
    events: Heap,
    points: Heap,
    flush_points: Heap,
    serial: u64,
    pub stats: Stats,
    start: u64,
    end: u64,
}
impl Pending {
    #[cfg(test)]
    pub fn new() -> Self { Self::with_capacity(PENDING_EVENTS) }
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0);
        let mut slots = crate::queue::preallocated(capacity);
        slots.resize_with(capacity, PendingPacket::empty);
        let mut free = crate::queue::preallocated(capacity);
        free.extend((0..capacity).rev());
        Self { slots: slots.into_boxed_slice(), free,
            events: Heap::new(capacity), points: Heap::new(capacity), flush_points: Heap::new(capacity),
            serial: 0, stats: Stats::default(), start: 0, end: 0 }
    }
    pub fn reset(&mut self) {
        self.stats.discarded_on_reset += self.stats.pending_events + self.stats.pending_points;
        self.stats.pending_events = 0; self.stats.pending_points = 0;
        self.events.indices.clear(); self.points.indices.clear(); self.flush_points.indices.clear();
        self.free.clear(); self.free.extend((0..self.slots.len()).rev());
        self.serial = 0; self.start = 0; self.end = 0;
    }
    pub fn window(&mut self, start: u64, n: usize) {
        self.start = start; self.end = start.saturating_add(n as u64);
    }
    pub fn append(&mut self, p: &Packet, position: u64, frames: usize, delay: u64) -> bool {
        if p.events as usize > EVENTS || p.points as usize > POINTS || p.bytes as usize > PAYLOAD
            || p.reserved != 0 { return false; }
        if p.events == 0 && p.points == 0 { return true; }
        if self.free.is_empty() { return false; }
        let Some(base) = position.checked_add(delay) else { return false; };
        for event in &p.event[..p.events as usize] {
            if event.offset < 0 || event.offset as usize >= frames.max(1)
                || event.payload_size as usize > EVENT_PAYLOAD
                || (event.payload_offset as usize).checked_add(event.payload_size as usize)
                    .is_none_or(|end| end > p.bytes as usize)
                || base.checked_add(event.offset as u64).is_none() { return false; }
        }
        for point in &p.point[..p.points as usize] {
            if point.offset < 0 || point.offset as usize >= frames.max(1)
                || base.checked_add(point.offset as u64).is_none() { return false; }
        }
        let Some(serial) = self.serial.checked_add(1) else { return false; };
        let index = self.free.pop().unwrap();
        let slot = &mut self.slots[index];
        slot.packet = *p; slot.position = position; slot.delay = delay;
        slot.serial = serial; slot.flush = frames == 0; slot.event_next = 0; slot.point_next = 0;
        // Sort small scalar indices, preserving producer order at equal offsets.
        for at in 0..p.events as usize {
            slot.event_order[at] = at as u8;
            let mut i = at;
            while i > 0 && p.event[slot.event_order[i - 1] as usize].offset
                > p.event[slot.event_order[i] as usize].offset {
                slot.event_order.swap(i - 1, i); i -= 1;
            }
        }
        for at in 0..p.points as usize {
            slot.point_order[at] = at as u8;
            let mut i = at;
            while i > 0 && p.point[slot.point_order[i - 1] as usize].offset
                > p.point[slot.point_order[i] as usize].offset {
                slot.point_order.swap(i - 1, i); i -= 1;
            }
        }
        self.serial = serial;
        self.stats.pending_events += u64::from(p.events);
        self.stats.pending_points += u64::from(p.points);
        if p.events > 0 { self.events.push(index, &self.slots, Kind::Event); }
        if p.points > 0 {
            if frames == 0 { self.flush_points.push(index, &self.slots, Kind::Point); }
            else { self.points.push(index, &self.slots, Kind::Point); }
        }
        true
    }
    fn eligible(&self, due: u64) -> bool {
        due < self.end || (self.start == self.end && due <= self.start)
    }
    fn retire_if_empty(&mut self, index: usize) {
        let slot = &self.slots[index];
        if slot.event_next == slot.packet.events as usize && slot.point_next == slot.packet.points as usize {
            self.free.push(index);
        }
    }
    pub fn take(&mut self, out: &mut Packet) {
        out.events = 0; out.points = 0; out.bytes = 0; out.reserved = 0;
        while out.events < EVENTS as u32 {
            let Some(index) = self.events.peek() else { break; };
            let due = self.slots[index].due(Kind::Event);
            if !self.eligible(due) { break; }
            let slot = &self.slots[index];
            let mut event = slot.packet.event[slot.event_order[slot.event_next] as usize];
            let start = (out.bytes + 1) & !1;
            if start + event.payload_size > PAYLOAD as u32 { break; }
            self.events.pop(&self.slots, Kind::Event);
            let slot = &mut self.slots[index];
            let source = event.payload_offset as usize;
            out.payload[start as usize..(start + event.payload_size) as usize]
                .copy_from_slice(&slot.packet.payload[source..source + event.payload_size as usize]);
            if start > out.bytes { out.payload[out.bytes as usize] = 0; }
            out.bytes = start + event.payload_size;
            if due < self.start { self.stats.late_events += 1; }
            event.offset = due.saturating_sub(self.start) as i32; event.payload_offset = start;
            out.event[out.events as usize] = event; out.events += 1;
            self.stats.pending_events -= 1; slot.event_next += 1;
            if slot.event_next < slot.packet.events as usize { self.events.push(index, &self.slots, Kind::Event); }
            self.retire_if_empty(index);
        }
        while out.points < POINTS as u32 {
            let normal = self.points.peek().filter(|&i| self.eligible(self.slots[i].due(Kind::Point)));
            let flush = self.flush_points.peek();
            let (index, flushing) = match (normal, flush) {
                (Some(a), Some(b)) => if Heap::less(a, b, &self.slots, Kind::Point) { (a, false) } else { (b, true) },
                (Some(a), None) => (a, false), (None, Some(b)) => (b, true), (None, None) => break,
            };
            if flushing { self.flush_points.pop(&self.slots, Kind::Point); }
            else { self.points.pop(&self.slots, Kind::Point); }
            let slot = &mut self.slots[index];
            let due = slot.due(Kind::Point);
            let mut point = slot.packet.point[slot.point_order[slot.point_next] as usize];
            if due < self.start && !slot.flush { self.stats.late_points += 1; }
            point.offset = if slot.flush { 0 } else { due.saturating_sub(self.start) as i32 };
            out.point[out.points as usize] = point; out.points += 1;
            self.stats.pending_points -= 1; slot.point_next += 1;
            if slot.point_next < slot.packet.points as usize {
                if flushing { self.flush_points.push(index, &self.slots, Kind::Point); }
                else { self.points.push(index, &self.slots, Kind::Point); }
            }
            self.retire_if_empty(index);
        }
    }
    pub fn stats(&self) -> Stats { self.stats }
    #[cfg(test)]
    pub fn storage_bytes(&self) -> usize {
        self.slots.len() * std::mem::size_of::<PendingPacket>()
            + (self.free.capacity() + self.events.indices.capacity() + self.points.indices.capacity()
                + self.flush_points.indices.capacity()) * std::mem::size_of::<usize>()
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
    fn compact_packet_storage_has_a_prepared_bound_without_a_same_callback_delay_tax() {
        let same = Pending::with_capacity(1);
        let buffered = Pending::with_capacity(1024 + 512 + 1);
        let unit = std::mem::size_of::<PendingPacket>() + 4 * std::mem::size_of::<usize>();
        assert_eq!(same.storage_bytes(), unit);
        assert_eq!(buffered.storage_bytes(), 1537 * unit);
        assert!(same.storage_bytes() < 11 * 1024);
        assert!(buffered.storage_bytes() < 17 * 1024 * 1024);
        eprintln!("prepared process results: same={} bytes, buffered D1024={} bytes", same.storage_bytes(), buffered.storage_bytes());
    }
    #[test]
    fn packet_heap_preserves_cross_packet_time_order_equal_offsets_and_flush_eligibility() {
        let mut pending = Pending::with_capacity(3);
        let mut packet = Packet { events: 3, points: 3, ..Default::default() };
        for (i, offset) in [3, 1, 1].into_iter().enumerate() {
            packet.event[i] = Event { offset, kind: 65535, a: i as i32 + 1, ..Default::default() };
            packet.point[i] = Point { offset, id: i as u32 + 1, value: 0.25 };
        }
        assert!(pending.append(&packet, 0, 4, 256));
        packet.events = 2; packet.points = 2;
        for (i, offset) in [1, 0].into_iter().enumerate() {
            packet.event[i].offset = offset; packet.event[i].a = 10 + i as i32;
            packet.point[i].offset = offset; packet.point[i].id = 10 + i as u32;
        }
        assert!(pending.append(&packet, 0, 4, 256));
        let mut out = Packet::default();
        pending.window(256, 4);
        let (_, allocations) = crate::allocation_test::measure(|| pending.take(&mut out));
        assert_eq!(allocations, [0; 3]);
        assert_eq!(out.event[..5].iter().map(|e| e.a).collect::<Vec<_>>(), [11, 2, 3, 10, 1]);
        assert_eq!(out.point[..5].iter().map(|p| p.id).collect::<Vec<_>>(), [11, 2, 3, 10, 1]);
        assert_eq!((pending.stats().pending_events, pending.stats().pending_points), (0, 0));
        packet.events = 0; packet.points = 1; packet.point[0].offset = 0;
        assert!(pending.append(&packet, 1000, 0, 0));
        pending.window(0, 0); pending.take(&mut out);
        assert_eq!((out.points, out.point[0].offset), (1, 0));
    }
    #[test]
    fn overlapping_maximum_payload_slices_repack_into_eight_owned_drain_packets() {
        let mut pending = Pending::with_capacity(1);
        let mut packet = Packet { events: 64, bytes: 512, ..Default::default() };
        packet.payload[..512].fill(0xab);
        for event in &mut packet.event {
            *event = Event { kind: 2, payload_size: 512, ..Default::default() };
        }
        assert!(pending.append(&packet, 0, 1, 0));
        packet.payload.fill(0); pending.window(0, 1);
        let mut out = Packet::default();
        for _ in 0..8 {
            let (_, allocations) = crate::allocation_test::measure(|| pending.take(&mut out));
            assert_eq!(allocations, [0; 3]);
            assert_eq!((out.events, out.points, out.bytes), (8, 0, 4096));
            for (i, event) in out.event[..8].iter().enumerate() {
                assert_eq!((event.payload_offset, event.payload_size), ((i * 512) as u32, 512));
            }
            assert!(out.payload.iter().all(|&byte| byte == 0xab));
        }
        pending.take(&mut out); assert_eq!((out.events, out.points), (0, 0));
        assert_eq!(pending.stats().pending_events, 0);
        assert!(pending.append(&packet, 0, 1, 0), "the final drain releases the source packet slot");
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
            bytes: 3,
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
        assert_eq!(out.events, 0); // Parameter flush does not advance event time.
        q.window(512, 0); // A zero-frame callback can still deliver a due event.
        q.take(&mut out);
        assert_eq!(out.events, 1);
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

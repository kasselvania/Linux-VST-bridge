//! Prepared start-up audio and parameter storage. The callback owns both; no
//! allocation or control owner is involved after activation.
use ap1_native_client::{events::{Event, MAX_EVENTS, NOTE_OFF, NOTE_ON, PARAMETER}, get, invalid, need, BLOCK_CAP};
use std::io;

/// Longest a note may be held through an unacknowledged processing start, in
/// audio frames of that start. Past it, held and further note-ons of that start
/// are discarded rather than fired together late. Note-offs are still
/// delivered, except those releasing a note-on discarded in the same start.
pub(crate) const NOTE_HOLD: std::time::Duration = std::time::Duration::from_millis(250);

// A note-off releases a note-on with the same VST3 note id when both carry one,
// otherwise the same channel and pitch.
fn releases(on: &Event, off: &Event) -> bool {
    if on.id != u32::MAX && off.id != u32::MAX { on.id == off.id }
    else { on.channel == off.channel && on.pitch == off.pitch }
}
// Admitted note events always carry channel 0..16 and pitch 0..128.
fn key(e: &Event) -> Option<(usize, usize)> {
    let (c, p) = (usize::try_from(e.channel).ok()?, usize::try_from(e.pitch).ok()?);
    (c < 16 && p < 128).then_some((c, p))
}

/// Note events the DAW sent while the Windows side had not yet acknowledged a
/// processing start. Those blocks are outside the vendor's stream, so the
/// events are played late rather than lost: in arrival order, at the first
/// sample of the first rendered block. A DAW that restarts processing on
/// every transport start (REAPER) otherwise loses its first notes. A note
/// that starts and ends inside the held window arrives as a zero-length note.
/// Fixed storage, owned by the callback; nothing allocates.
pub(crate) struct Notes {
    events: [Event; MAX_EVENTS],
    count: usize,
    // Note-ons discarded in this start without a held release. Their later
    // note-offs are discarded too instead of being delivered. Note-ons with a
    // VST3 note id are remembered by id (pairing uses ids); id-less ones, and
    // any beyond the id list, by channel and pitch.
    discarded: [[u16; 128]; 16],
    discarded_ids: [u32; MAX_EVENTS],
    discarded_id_count: usize,
    // Set after a search found no two identical held releases; cleared by any
    // change to storage, so a full store of distinct releases is searched once.
    releases_unique: bool,
    #[cfg(test)]
    pub(crate) duplicate_scans: u64,
}
impl Notes {
    pub fn empty() -> Self {
        Self { events: [Event::default(); MAX_EVENTS], count: 0, discarded: [[0; 128]; 16],
            discarded_ids: [0; MAX_EVENTS], discarded_id_count: 0, releases_unique: false,
            #[cfg(test)] duplicate_scans: 0 }
    }
    pub fn pending(&self) -> usize { self.count }
    /// A new processing start: earlier discards no longer pair with releases.
    pub fn begin_start(&mut self) { self.discarded = [[0; 128]; 16]; self.discarded_id_count = 0; }
    fn note_discarded(&mut self, on: &Event) {
        if on.id != u32::MAX && self.discarded_id_count < MAX_EVENTS {
            self.discarded_ids[self.discarded_id_count] = on.id;
            self.discarded_id_count += 1;
        } else if let Some((c, p)) = key(on) {
            self.discarded[c][p] = self.discarded[c][p].saturating_add(1);
        }
    }
    // A release of a note-on discarded in this start consumes that record.
    fn releases_discarded(&mut self, off: &Event) -> bool {
        if off.id != u32::MAX {
            if let Some(i) = self.discarded_ids[..self.discarded_id_count].iter().position(|&id| id == off.id) {
                self.discarded_id_count -= 1;
                self.discarded_ids[i] = self.discarded_ids[self.discarded_id_count];
                return true;
            }
        }
        match key(off) {
            Some((c, p)) if self.discarded[c][p] > 0 => { self.discarded[c][p] -= 1; true }
            _ => false,
        }
    }
    // A held note-on this release would pair with (one not already released).
    fn releases_held(&self, off: &Event) -> bool {
        let held = &self.events[..self.count];
        let ons = held.iter().filter(|e| e.kind == NOTE_ON && releases(e, off)).count();
        let offs = held.iter().filter(|e| e.kind == NOTE_OFF && releases(off, e)).count();
        ons > offs
    }
    fn store(&mut self, e: &Event) {
        self.events[self.count] = Event { offset: 0, ..*e };
        self.count += 1;
        self.releases_unique = false;
    }
    fn remove(&mut self, drop: &[bool; MAX_EVENTS]) -> u64 {
        self.releases_unique = false;
        let mut kept = 0;
        for (i, &dropped) in drop.iter().enumerate().take(self.count) {
            if !dropped { self.events[kept] = self.events[i]; kept += 1; }
        }
        let dropped = (self.count - kept) as u64;
        self.count = kept;
        dropped
    }
    /// Discard every held note-on with its paired held note-off (a stop, a new
    /// start, or the hold limit). Note-offs for notes already playing are kept
    /// for the next rendered block. Returns how many events were discarded.
    pub fn discard_note_ons(&mut self) -> u64 {
        if !self.events[..self.count].iter().any(|e| e.kind == NOTE_ON) { return 0; }
        let mut drop = [false; MAX_EVENTS];
        for j in 0..self.count {
            if self.events[j].kind != NOTE_OFF { continue; }
            // Its pair is the oldest still-unpaired held note-on it releases.
            if let Some(i) = (0..j).find(|&i| self.events[i].kind == NOTE_ON && !drop[i]
                && releases(&self.events[i], &self.events[j])) {
                drop[i] = true; drop[j] = true;
            }
        }
        for (i, dropped) in drop.iter_mut().enumerate().take(self.count) {
            let on = self.events[i];
            if on.kind == NOTE_ON {
                // An unpaired note-on's release is still to come.
                if !*dropped { self.note_discarded(&on); }
                *dropped = true;
            }
        }
        self.remove(&drop)
    }
    // The oldest held note-on is paired with the first later note-off releasing it.
    fn evict_oldest_note_on(&mut self) -> u64 {
        let Some(first) = self.events[..self.count].iter().position(|e| e.kind == NOTE_ON) else { return 0; };
        let mut drop = [false; MAX_EVENTS];
        drop[first] = true;
        match (first + 1..self.count).find(|&j| self.events[j].kind == NOTE_OFF
            && releases(&self.events[first], &self.events[j])) {
            Some(j) => drop[j] = true,
            None => { let on = self.events[first]; self.note_discarded(&on); }
        }
        self.remove(&drop)
    }
    #[cfg(test)]
    fn count_scan(&mut self) { self.duplicate_scans += 1; }
    #[cfg(not(test))]
    fn count_scan(&mut self) {}
    // Two identical held note-offs carry one release; drop the later copy. One
    // pass with the latest held release per channel and pitch; a release with
    // the same key but another note id is not a copy. Unchanged storage is not
    // searched again after a search that found none.
    fn evict_duplicate_note_off(&mut self) -> u64 {
        if self.releases_unique { return 0; }
        self.count_scan();
        let mut seen = [0u128; 16];
        let mut latest = [[0u8; 128]; 16];
        for j in 0..self.count {
            let e = self.events[j];
            if e.kind != NOTE_OFF { continue; }
            let Some((c, p)) = key(&e) else { continue; };
            if seen[c] & (1u128 << p) != 0 && self.events[latest[c][p] as usize].id == e.id {
                let mut drop = [false; MAX_EVENTS];
                drop[j] = true;
                return self.remove(&drop);
            }
            seen[c] |= 1u128 << p;
            latest[c][p] = j as u8;
        }
        self.releases_unique = true;
        0
    }
    /// Hold this block's note events; returns how many events will not be
    /// played. With `hold_note_ons` false (past the hold limit) note-ons are
    /// discarded and only note-offs are held. A note-off whose note-on was
    /// discarded in this start is discarded with it. Full storage evicts the
    /// oldest held note-on (with its release), then a duplicate held note-off,
    /// before it refuses an incoming release.
    pub fn retain(&mut self, events: &[Event], hold_note_ons: bool) -> u64 {
        let mut dropped = 0;
        for e in events.iter().filter(|e| e.kind == NOTE_ON || e.kind == NOTE_OFF) {
            if e.kind == NOTE_ON && !hold_note_ons { self.note_discarded(e); dropped += 1; continue; }
            if e.kind == NOTE_OFF && !self.releases_held(e) && self.releases_discarded(e) {
                dropped += 1; continue;
            }
            if self.count == MAX_EVENTS {
                if e.kind == NOTE_ON { self.note_discarded(e); dropped += 1; continue; }
                dropped += self.evict_oldest_note_on();
                // The evicted note-on may be the one this release ends.
                if self.count < MAX_EVENTS && !self.releases_held(e) && self.releases_discarded(e) {
                    dropped += 1; continue;
                }
                if self.count == MAX_EVENTS { dropped += self.evict_duplicate_note_off(); }
                if self.count == MAX_EVENTS { dropped += 1; continue; }
            }
            self.store(e);
        }
        dropped
    }
    /// Put held notes ahead of a rendered block's own notes, at sample 0. The
    /// block's parameter points keep their offsets. When the block cannot carry
    /// every note, the remainder (held notes, then the block's own notes) stays
    /// held in order for the next block, so no note-off overtakes its note-on.
    /// Returns how many previously held events this block carries.
    pub fn deliver(&mut self, events: &mut [Event; MAX_EVENTS], count: &mut u32) -> usize {
        if self.count == 0 { return 0; }
        let own = (*count as usize).min(MAX_EVENTS);
        let mut block = [Event::default(); MAX_EVENTS];
        block[..own].copy_from_slice(&events[..own]);
        let mut n = 0;
        for e in block[..own].iter().filter(|e| e.kind == PARAMETER) { events[n] = *e; n += 1; }
        let delivered = self.count.min(MAX_EVENTS - n);
        events[n..n + delivered].copy_from_slice(&self.events[..delivered]);
        n += delivered;
        self.events.copy_within(delivered..self.count, 0);
        self.count -= delivered;
        self.releases_unique = false;
        for e in block[..own].iter().filter(|e| e.kind != PARAMETER) {
            if self.count == 0 && n < MAX_EVENTS { events[n] = *e; n += 1; }
            // Bounded by construction: the remainder never exceeds what was held.
            else if self.count < MAX_EVENTS { self.store(e); }
        }
        *count = n as u32;
        delivered
    }
}

pub(crate) const MAX_PARAMETERS: usize = 8192;
pub(crate) const MAX_PARAMETER_PACKETS: usize = MAX_PARAMETERS.div_ceil(MAX_EVENTS);
// The existing transport admits at most this many retained sample frames.
pub(crate) const MAX_DELAY: usize = crate::queued::DESCRIPTORS * BLOCK_CAP;
struct Value { id: u32, value: Option<f64> }
pub(crate) struct Parameters { values: Vec<Value>, configured: bool }
impl Parameters {
    pub fn empty() -> Self { Self { values: Vec::new(), configured: false } }
    pub fn configure(&mut self, ids: &[u32]) -> Result<(), ()> {
        if self.configured || ids.len() > MAX_PARAMETERS { return Err(()); }
        let mut values: Vec<_> = ids.iter().map(|&id| Value { id, value: None }).collect();
        values.sort_unstable_by_key(|v| v.id);
        if values.windows(2).any(|p| p[0].id == p[1].id) { return Err(()); }
        self.values = values; self.configured = true; Ok(())
    }
    pub fn clear(&mut self) { for v in &mut self.values { v.value = None; } }
    pub fn pending(&self) -> bool { self.values.iter().any(|v| v.value.is_some()) }
    pub fn retain(&mut self, events: &[Event]) -> Result<(), ()> {
        for event in events.iter().filter(|e| e.kind == PARAMETER) {
            let index = self.values.binary_search_by_key(&event.id, |v| v.id).map_err(|_| ())?;
            self.values[index].value = Some(event.value);
        }
        Ok(())
    }
    pub fn packet(&mut self, events: &mut [Event; MAX_EVENTS]) -> usize {
        let mut n = 0;
        for v in &mut self.values {
            if let Some(value) = v.value.take() {
                events[n] = Event { kind: PARAMETER, id: v.id, value, ..Event::default() };
                n += 1;
                if n == MAX_EVENTS { break; }
            }
        }
        n
    }
}

pub(crate) struct Dry {
    samples: Box<[f32]>,
    delayed: [[f32; BLOCK_CAP]; 2],
    delay: usize,
    fade_frames: usize,
    output: usize,
    cursor: usize,
    populated: usize,
    fade: usize,
}
impl Dry {
    pub fn prepare(effect: bool, setup: &[u8], vendor: u32, bridge: usize) -> io::Result<Option<Self>> {
        if !effect || setup.len() == 24 { return Ok(None); }
        let input = setup[28..].chunks_exact(32).any(|r| get(&r[..4]) == 0
            && get(&r[4..8]) == 0 && get(&r[8..12]) == 0 && get(&r[12..16]) == 2
            && get(&r[16..20]) == 0 && get(&r[20..24]) == 1 && get(&r[24..32]) == 3);
        let output = setup[28..].chunks_exact(32).find(|r| get(&r[..4]) == 0
            && get(&r[4..8]) == 1 && get(&r[12..16]) == 2
            && get(&r[16..20]) == 0 && get(&r[20..24]) == 1 && get(&r[24..32]) == 3)
            .map(|r| get(&r[8..12]) as usize * 2);
        let Some(output) = output.filter(|_| input) else { return Ok(None); };
        let delay = bridge.checked_add(vendor as usize).ok_or_else(|| invalid("start-up latency overflow"))?;
        need(delay <= MAX_DELAY, "start-up dry latency exceeds retained sample capacity")?;
        let maximum = get(&setup[..4]) as usize;
        let rate = f64::from_le_bytes(setup[8..16].try_into().unwrap());
        need(rate.is_finite() && rate > 0. && rate <= u32::MAX as f64,
            "start-up crossfade sample rate")?;
        let fade_frames = (rate * 0.05).ceil() as usize;
        // A latency-changing parameter can change L while starting. Reserve the
        // existing finite frame ceiling now, then adjust the read distance only.
        let capacity = MAX_DELAY + maximum;
        let mut samples = crate::queue::preallocated(2 * capacity);
        samples.resize(2 * capacity, 0.);
        Ok(Some(Self { samples: samples.into_boxed_slice(), delayed: [[0.; BLOCK_CAP]; 2],
            delay, fade_frames, output, cursor: 0, populated: 0, fade: 0 }))
    }
    pub fn reset(&mut self) { self.cursor = 0; self.populated = 0; self.fade = 0; }
    pub fn active(&self) -> bool { self.fade < self.fade_frames }
    pub fn committed(&self) -> bool { self.fade != 0 }
    pub fn delay(&mut self, delay: usize) -> Result<(), ()> {
        if delay > MAX_DELAY { return Err(()); }
        self.delay = delay; Ok(())
    }
    pub fn feed(&mut self, input: &[[f32; BLOCK_CAP]; 2], n: usize) {
        if !self.active() { return; }
        let capacity = self.samples.len() / 2;
        for i in 0..n {
            for (ch, plane) in input.iter().enumerate() {
                self.samples[ch * capacity + self.cursor] = plane[i];
            }
            self.cursor = (self.cursor + 1) % capacity;
            self.populated = (self.populated + 1).min(capacity);
        }
    }
    pub unsafe fn present(&mut self, main: &mut [[f32; BLOCK_CAP]; 2], extra: &[*mut f32],
        destination: usize, n: usize, wet_position: Option<u64>, wet_delay: u64,
    ) -> u64 {
        let channels = extra.len() + 2;
        // N0 parameter replay may have changed L after this block was copied.
        // Read the prepared history at the latest delay without feeding twice.
        if self.active() {
            let capacity = self.samples.len() / 2;
            for i in 0..n {
                for ch in 0..2 {
                    self.delayed[ch][i] = if self.populated.saturating_sub(n - i) >= self.delay {
                        self.samples[ch * capacity + (self.cursor + 2 * capacity - n + i - self.delay) % capacity]
                    } else { 0. };
                }
            }
        }
        let mut flags = if channels == 64 { u64::MAX } else { (1 << channels) - 1 };
        for i in 0..n {
            let ready = wet_position.is_some_and(|p| p.saturating_add(i as u64) >= wet_delay);
            let mix = if ready {
                self.fade = (self.fade + 1).min(self.fade_frames);
                self.fade as f32 / self.fade_frames as f32
            } else { 0. };
            for ch in 0..channels {
                let pointer = if ch < 2 { main[ch].as_mut_ptr().add(i) }
                    else if extra[ch - 2].is_null() { continue; }
                    else { extra[ch - 2].add(destination + i) };
                let dry = if (self.output..self.output + 2).contains(&ch) {
                    self.delayed[ch - self.output][i]
                } else { 0. };
                let value = if mix == 1. { *pointer } else if mix == 0. { dry }
                    else { dry * (1. - mix) + *pointer * mix };
                *pointer = value;
                if value != 0. { flags &= !(1 << ch); }
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup(auxiliary_main: bool) -> Vec<u8> {
        let mut bytes = crate::performance::wire_version(4, 0, 48000., true).unwrap();
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes.extend((if auxiliary_main { 3u32 } else { 2u32 }).to_le_bytes());
        let outputs = if auxiliary_main { 2 } else { 1 };
        for (direction,index,kind) in std::iter::once((0u32,0u32,0u32))
            .chain((0..outputs).map(|index| (1,index,if auxiliary_main {u32::from(index==0)} else {0}))) {
            for value in [0u32,direction,index,2,kind,1] { bytes.extend(value.to_le_bytes()); }
            bytes.extend(3u64.to_le_bytes());
        }
        bytes
    }
    #[test]
    fn startup_values_survive_empty_callbacks_and_coalesce_across_full_census() {
        let ids: Vec<_> = (0..MAX_PARAMETERS as u32).rev().collect();
        let mut values = Parameters::empty(); values.configure(&ids).unwrap();
        let mut packet = [Event::default(); MAX_EVENTS];
        for id in 0..MAX_PARAMETERS as u32 {
            values.retain(&[Event { kind: PARAMETER, id, value: 0.25, ..Event::default() }]).unwrap();
        }
        values.retain(&[Event { kind: PARAMETER, id: 7, value: 0.75, ..Event::default() }]).unwrap();
        values.retain(&[]).unwrap();
        let ((count, seven), allocations) = crate::allocation_test::measure(|| {
            let mut count = 0; let mut seven = None;
            for _ in 0..MAX_PARAMETERS.div_ceil(MAX_EVENTS) {
                let n = values.packet(&mut packet);
                count += n;
                if let Some(e) = packet[..n].iter().find(|e| e.id == 7) { seven = Some(e.value); }
                assert!(packet[..n].iter().all(|e| e.offset == 0 && e.valid_host(0)));
            }
            (count,seven)
        });
        assert_eq!((count,seven), (MAX_PARAMETERS,Some(0.75))); assert_eq!(allocations,[0;3]);
        assert_eq!(values.packet(&mut packet),0);
        assert!(values.retain(&[Event { kind: PARAMETER, id: MAX_PARAMETERS as u32, ..Event::default() }]).is_err());
        values.retain(&[Event { kind: PARAMETER, id: 7, ..Event::default() }]).unwrap();
        values.clear(); assert_eq!(values.packet(&mut packet),0);
    }
    #[test]
    fn held_notes_pair_releases_by_note_id_and_deliver_without_allocating() {
        let note = |kind, pitch, id| Event { kind, pitch, id, value: 0.8, offset: 9, ..Event::default() };
        let mut notes = Notes::empty();
        // Two voices of pitch 60 told apart by note id; 61 carries none.
        let (dropped, allocations) = crate::allocation_test::measure(|| notes.retain(&[
            note(NOTE_ON, 60, 1), note(NOTE_ON, 60, 2),
            Event { kind: PARAMETER, id: 7, value: 0.5, ..Event::default() },
            note(NOTE_OFF, 60, 2), note(NOTE_ON, 61, u32::MAX), note(NOTE_OFF, 61, u32::MAX),
            note(NOTE_OFF, 59, 9)], true));
        assert_eq!((dropped, notes.pending(), allocations), (0, 6, [0; 3]));
        // Discarding note-ons removes each with its own release: voice 2 and
        // 61 go with their note-offs; voice 1 goes alone; 59's release stays.
        assert_eq!(notes.discard_note_ons(), 5);
        let mut events = [Event::default(); MAX_EVENTS];
        events[0] = Event { kind: PARAMETER, id: 7, value: 0.25, offset: 3, ..Event::default() };
        events[1] = note(NOTE_ON, 64, u32::MAX);
        let mut count = 2;
        let (delivered, allocations) = crate::allocation_test::measure(|| notes.deliver(&mut events, &mut count));
        assert_eq!((delivered, count, allocations), (1, 3, [0; 3]));
        assert_eq!(events[..3].iter().map(|e| (e.kind, e.pitch, e.offset)).collect::<Vec<_>>(),
            [(PARAMETER, 0, 3), (NOTE_OFF, 59, 0), (NOTE_ON, 64, 9)]);
        assert!(events[..3].iter().all(|e| e.valid_host(512)));
        assert_eq!(notes.deliver(&mut events, &mut count), 0);
        // Past the hold limit note-ons are discarded, and so are their releases;
        // a release of a note already playing is kept.
        assert_eq!(notes.retain(&[note(NOTE_ON, 70, 4), note(NOTE_OFF, 70, 4), note(NOTE_OFF, 71, 5)], false), 2);
        assert_eq!(notes.pending(), 1);
    }
    #[test]
    fn full_storage_keeps_one_copy_of_a_release_and_a_new_start_forgets_discards() {
        let off = |channel: i16, pitch: i16| Event { kind: NOTE_OFF, channel, pitch, id: u32::MAX, value: 0.5, ..Event::default() };
        let mut notes = Notes::empty();
        // 255 releases of notes already playing plus one duplicate fill storage.
        let mut releases: Vec<_> = (0..255).map(|i| off((i / 128) as i16, (i % 128) as i16)).collect();
        releases.push(off(0, 0));
        assert_eq!(notes.retain(&releases, true), 0);
        assert_eq!(notes.pending(), MAX_EVENTS);
        // A new release takes the duplicate's place instead of being refused;
        // without a duplicate a further release is refused, and counted.
        let (dropped, allocations) = crate::allocation_test::measure(|| notes.retain(&[off(2, 5)], true));
        assert_eq!((dropped, notes.pending(), allocations), (1, MAX_EVENTS, [0; 3]));
        assert_eq!(notes.retain(&[off(2, 6)], true), 1);
        let scans = notes.duplicate_scans;
        // Unchanged storage is not searched again, however many releases arrive.
        let flood: Vec<_> = (0..2048).map(|i| off(3 + (i / 128) as i16 % 13, (i % 128) as i16)).collect();
        let (dropped, allocations) = crate::allocation_test::measure(|| notes.retain(&flood, true));
        assert_eq!((dropped, notes.duplicate_scans, allocations), (2048, scans, [0; 3]));
        let mut events = [Event::default(); MAX_EVENTS];
        let mut count = 0;
        assert_eq!(notes.deliver(&mut events, &mut count), MAX_EVENTS);
        let held = &events[..count as usize];
        assert_eq!(held.iter().filter(|e| (e.channel, e.pitch) == (0, 0)).count(), 1);
        assert_eq!(held.iter().filter(|e| (e.channel, e.pitch) == (2, 5)).count(), 1);
        assert!(!held.iter().any(|e| (e.channel, e.pitch) == (2, 6)));
        // A release that evicts its own note-on from full storage goes with it,
        // and does not leave a count that would swallow another note's release.
        let on = |pitch: i16, id: u32| Event { kind: NOTE_ON, pitch, id, value: 0.5, ..Event::default() };
        let id_off = |pitch: i16, id: u32| Event { kind: NOTE_OFF, pitch, id, value: 0.5, ..Event::default() };
        let mut notes = Notes::empty();
        let mut fill = vec![on(60, 7)];
        fill.extend((1..MAX_EVENTS).map(|i| off(1 + (i / 128) as i16, (i % 128) as i16)));
        assert_eq!(notes.retain(&fill, true), 0);
        assert_eq!(notes.retain(&[id_off(60, 7)], true), 2);
        assert_eq!(notes.retain(&[id_off(60, 5)], true), 0);
        let mut count = 0;
        notes.deliver(&mut events, &mut count);
        let held = &events[..count as usize];
        assert!(!held.iter().any(|e| e.id == 7));
        assert_eq!(held.iter().filter(|e| e.kind == NOTE_OFF && e.id == 5).count(), 1);
        // Discards pair with releases by note id when the note-on carried one.
        let mut notes = Notes::empty();
        notes.retain(&[on(60, 1), on(60, 2)], false);
        assert_eq!(notes.retain(&[id_off(60, 2), id_off(60, 9), id_off(60, 1)], false), 2);
        assert_eq!(notes.pending(), 1);
        // A note-on discarded in one start does not swallow a release after a new start.
        let mut notes = Notes::empty();
        notes.retain(&[Event { kind: NOTE_ON, pitch: 60, id: u32::MAX, value: 0.5, ..Event::default() }], false);
        notes.begin_start();
        assert_eq!(notes.retain(&[off(0, 60)], true), 0);
        assert_eq!(notes.pending(), 1);
    }
    #[test]
    fn startup_dry_matches_reported_delay_with_variable_blocks_and_main_output_mapping() {
        let mut dry = Dry::prepare(true,&setup(true),1,2).unwrap().unwrap();
        let mut output = [[0.;BLOCK_CAP];2]; let mut auxiliary = [[0.;BLOCK_CAP];2];
        let extra = [auxiliary[0].as_mut_ptr(),auxiliary[1].as_mut_ptr()];
        let mut position = 0;
        for n in [1,2,4,1] {
            let mut input = [[0.;BLOCK_CAP];2];
            for i in 0..n { input[0][i] = (position+i+1) as f32; input[1][i] = input[0][i] * 2.; }
            let (flags,allocations) = crate::allocation_test::measure(|| {
                dry.feed(&input,n);
                unsafe { dry.present(&mut output,&extra,0,n,None,0) }
            });
            assert_eq!(allocations,[0;3]); assert_eq!(flags & 3,3);
            for i in 0..n {
                let expected = if position+i < 3 { 0. } else { (position+i-2) as f32 };
                assert_eq!((output[0][i],output[1][i]),(0.,0.));
                assert_eq!((auxiliary[0][i],auxiliary[1][i]),(expected,expected*2.));
            }
            position += n;
        }
        dry.reset(); dry.feed(&[[9.;BLOCK_CAP];2],2);
        unsafe { dry.present(&mut output,&extra,0,2,None,0); }
        assert_eq!(&auxiliary[0][..2],&[0.,0.]);
        assert!(dry.delay(MAX_DELAY+1).is_err());
        assert!(Dry::prepare(false,&setup(false),0,0).unwrap().is_none());
    }
    #[test]
    fn startup_dry_uses_latest_latency_and_waits_through_vendor_priming_before_fading() {
        let mut dry = Dry::prepare(true,&setup(false),0,0).unwrap().unwrap();
        let mut input = [[10.;BLOCK_CAP];2];
        dry.feed(&input,4); let mut output = [[0.;BLOCK_CAP];2];
        unsafe { dry.present(&mut output,&[],0,4,None,0); }
        // Current input is already captured when N0 replay changes L to 2.
        input[0][..4].copy_from_slice(&[20.,30.,40.,50.]); input[1]=input[0];
        dry.feed(&input,4); dry.delay(2).unwrap();
        output[0][..4].fill(1.); output[1][..4].fill(1.);
        unsafe { dry.present(&mut output,&[],0,4,Some(0),2); }
        let blend = |a,frame| a*(1.-frame as f32/2400.)+frame as f32/2400.;
        assert_eq!(&output[0][..4],&[10.,10.,blend(20.,1),blend(30.,2)]);
        dry.feed(&input,2); output[0][..2].fill(1.); output[1][..2].fill(1.);
        unsafe { dry.present(&mut output,&[],0,2,Some(4),2); }
        assert_eq!(&output[0][..2],&[blend(40.,3),blend(50.,4)]);
        assert!(dry.active()); assert!(dry.committed());
    }
    #[test]
    fn crossfade_is_fifty_milliseconds_across_rates_variable_blocks_and_silent_wet() {
        for rate in [44100u32,48000,96000,192000] {
            let mut bytes=setup(false);
            bytes[8..16].copy_from_slice(&(rate as f64).to_le_bytes());
            let mut dry=Dry::prepare(true,&bytes,0,0).unwrap().unwrap();
            let frames=rate as usize/20;let mut position=0;let input=[[1.;BLOCK_CAP];2];
            let mut output=[[0.;BLOCK_CAP];2];
            while position<frames+7 {
                let n=[1,3,4,2][position%4].min(frames+7-position);
                for plane in &mut output {plane[..n].fill(0.);} // valid silent effect output
                let (_,allocations)=crate::allocation_test::measure(|| {
                    dry.feed(&input,n);
                    unsafe {dry.present(&mut output,&[],0,n,Some(position as u64),0)}
                });
                assert_eq!(allocations,[0;3]);
                for i in 0..n {
                    let mix=((position+i+1).min(frames)) as f32/frames as f32;
                    assert_eq!(output[0][i],1.-mix);
                    assert_eq!(output[1][i],output[0][i]);
                }
                position+=n;assert_eq!(dry.active(),position<frames);
            }
            dry.reset();assert!(dry.active());assert!(!dry.committed());
            dry.feed(&input,4);output=[[0.;BLOCK_CAP];2];
            unsafe {dry.present(&mut output,&[],0,4,None,0);}
            assert_eq!(&output[0][..4],&[1.;4]);assert!(!dry.committed());
        }
    }
}

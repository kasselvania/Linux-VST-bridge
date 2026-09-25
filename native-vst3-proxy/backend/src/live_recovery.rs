//! Opt-in, fixed-capacity player intent for same-instance live recovery.
//! The callback alone owns this state. No allocation occurs after selection.
use ap1_native_client::events::{Event, NOTE_OFF, NOTE_ON, PARAMETER};

pub const LEDGER_NOTES: usize = 256;
pub const LEDGER_PARAMETERS: usize = 256;
pub const PENDING_EVENTS: usize = 256;
const JOURNAL_REQUESTS: usize = 64;
const JOURNAL_EVENTS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Policy {
    pub horizon_frames: u32,
    pub worker_deadline_ms: u32,
    pub max_recoveries: u32,
}
impl Policy {
    pub fn valid(self) -> bool {
        (256..=8192).contains(&self.horizon_frames)
            && (20..=5000).contains(&self.worker_deadline_ms)
            && self.max_recoveries == 1
    }
}

#[derive(Clone)]
pub struct View {
    pub notes: [Option<Event>; LEDGER_NOTES],
    pub parameters: [Option<Event>; LEDGER_PARAMETERS],
}
impl Default for View {
    fn default() -> Self {
        Self {
            notes: [None; LEDGER_NOTES],
            parameters: [None; LEDGER_PARAMETERS],
        }
    }
}
impl View {
    pub fn parameter(&self, id: u32) -> Option<Event> {
        self.parameters.iter().flatten().find(|p| p.id == id).copied()
    }
    pub fn apply(&mut self, event: Event) -> Result<(), ()> {
        match event.kind {
            NOTE_ON => {
                if let Some(slot) = self.notes.iter_mut()
                    .find(|slot| slot.is_some_and(|v| v.id == event.id)) {
                    *slot = Some(event);
                } else {
                    *self.notes.iter_mut().find(|slot| slot.is_none()).ok_or(())? = Some(event);
                }
            }
            NOTE_OFF => {
                if let Some(slot) = self.notes.iter_mut()
                    .find(|slot| slot.is_some_and(|v| v.id == event.id)) {
                    *slot = None;
                }
            }
            PARAMETER => {
                if let Some(slot) = self.parameters.iter_mut()
                    .find(|slot| slot.is_some_and(|v| v.id == event.id)) {
                    *slot = Some(event);
                } else {
                    *self.parameters.iter_mut().find(|slot| slot.is_none()).ok_or(())? = Some(event);
                }
            }
            _ => return Err(()),
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
struct Record {
    epoch: u64,
    position: u64,
    sequence: u64,
    n: u32,
    event_count: usize,
}

// One callback-owned FIFO. Empty audio requests also have records, so each
// completion advances an exact frontier. Events occupy a separate fixed ring.
struct Journal {
    records: [Record; JOURNAL_REQUESTS],
    head: usize,
    len: usize,
    events: [Event; JOURNAL_EVENTS],
    event_head: usize,
    event_len: usize,
}
impl Default for Journal {
    fn default() -> Self {
        Self {
            records: [Record::default(); JOURNAL_REQUESTS],
            head: 0,
            len: 0,
            events: [Event::default(); JOURNAL_EVENTS],
            event_head: 0,
            event_len: 0,
        }
    }
}
impl Journal {
    fn capacity_for(&self, count: usize) -> bool {
        self.len < JOURNAL_REQUESTS && self.event_len + count <= JOURNAL_EVENTS
    }
    fn push(&mut self, record: Record, events: &[Event]) {
        debug_assert!(self.capacity_for(events.len()));
        debug_assert_eq!(record.event_count, events.len());
        self.records[(self.head + self.len) % JOURNAL_REQUESTS] = record;
        for (i, event) in events.iter().enumerate() {
            self.events[(self.event_head + self.event_len + i) % JOURNAL_EVENTS] = *event;
        }
        self.event_len += events.len();
        self.len += 1;
    }
    fn front(&self) -> Option<Record> {
        (self.len > 0).then(|| self.records[self.head])
    }
    fn pop_confirmed(&mut self, confirmed: &mut View) -> Result<Record, ()> {
        let record = self.front().ok_or(())?;
        for i in 0..record.event_count {
            confirmed.apply(self.events[(self.event_head + i) % JOURNAL_EVENTS])?;
        }
        self.head = (self.head + 1) % JOURNAL_REQUESTS;
        self.len -= 1;
        self.event_head = (self.event_head + record.event_count) % JOURNAL_EVENTS;
        self.event_len -= record.event_count;
        Ok(record)
    }
    fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
        self.event_head = 0;
        self.event_len = 0;
    }
}

#[derive(Clone, Copy)]
pub enum Observation {
    Normal,
    AdmissionClosed,
    AfterAcknowledge,
}

pub struct Ledger {
    // Confirmed advances on completion, never on request publication.
    pub confirmed: View,
    pub current: View,
    old_at_ack: View,
    held_at_ack: View,
    replay_parameters: View,
    journal: Journal,
    next_sequence: u64,
    completed_sequence: u64,
    completed_position: u64,
    pub pending: [Event; PENDING_EVENTS],
    pub pending_len: usize,
    pub pending_cursor: usize,
    pub reconcile_cursor: usize,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            confirmed: View::default(),
            current: View::default(),
            old_at_ack: View::default(),
            held_at_ack: View::default(),
            replay_parameters: View::default(),
            journal: Journal::default(),
            next_sequence: 1,
            completed_sequence: 0,
            completed_position: 0,
            pending: [Event::default(); PENDING_EVENTS],
            pending_len: 0,
            pending_cursor: 0,
            reconcile_cursor: 0,
        }
    }
}
impl Ledger {
    pub fn observe(&mut self, event: Event, phase: Observation) -> Result<(), ()> {
        self.current.apply(event)?;
        match phase {
            Observation::Normal => {}
            Observation::AdmissionClosed => {
                if event.kind == PARAMETER {
                    self.replay_parameters.apply(event)?;
                }
            }
            Observation::AfterAcknowledge => {
                if event.kind == PARAMETER {
                    if let Some(existing) = self.pending[self.pending_cursor..self.pending_len]
                        .iter_mut().find(|old| old.kind == PARAMETER && old.id == event.id) {
                        *existing = event;
                        return Ok(());
                    }
                }
                if self.pending_len == PENDING_EVENTS { return Err(()); }
                self.pending[self.pending_len] = event;
                self.pending_len += 1;
            }
        }
        Ok(())
    }
    pub fn can_publish(&self, count: usize) -> bool {
        self.next_sequence < u64::MAX && self.journal.capacity_for(count)
    }
    pub fn next_sequence(&self) -> u64 { self.next_sequence }
    pub fn published(&mut self, epoch: u64, position: u64, n: u32, events: &[Event]) {
        debug_assert!(self.can_publish(events.len()));
        self.journal.push(Record {
            epoch, position, sequence: self.next_sequence, n, event_count: events.len(),
        }, events);
        self.next_sequence += 1;
    }
    pub fn completed(&mut self, epoch: u64, position: u64, n: u32, sequence: u64) -> Result<(), ()> {
        let record = self.journal.front().ok_or(())?;
        if (record.epoch, record.position, record.n, record.sequence)
            != (epoch, position, n, sequence) || sequence <= self.completed_sequence {
            return Err(());
        }
        self.journal.pop_confirmed(&mut self.confirmed)?;
        self.completed_sequence = sequence;
        self.completed_position = position.checked_add(n as u64).ok_or(())?;
        Ok(())
    }
    pub fn acknowledge(&mut self, old_epoch: u64, final_sequence: u64, final_position: u64) -> Result<(), ()> {
        if final_sequence < self.completed_sequence || final_sequence >= self.next_sequence {
            return Err(());
        }
        while self.journal.front().is_some_and(|r| r.sequence <= final_sequence) {
            let record = self.journal.front().ok_or(())?;
            if record.epoch != old_epoch || record.sequence <= self.completed_sequence {
                return Err(());
            }
            self.journal.pop_confirmed(&mut self.confirmed)?;
            self.completed_sequence = record.sequence;
            self.completed_position = record.position.checked_add(record.n as u64).ok_or(())?;
        }
        if self.completed_sequence != final_sequence || self.completed_position != final_position {
            return Err(());
        }
        self.old_at_ack = self.confirmed.clone();
        self.held_at_ack = self.current.clone();
        // Only discarded or closed-admission parameter intent is replayed.
        // A historical completed value may have been superseded by a preset.
        let mut offset = self.journal.event_head;
        for i in 0..self.journal.len {
            let record = self.journal.records[(self.journal.head + i) % JOURNAL_REQUESTS];
            if record.epoch != old_epoch { return Err(()); }
            for j in 0..record.event_count {
                let event = self.journal.events[(offset + j) % JOURNAL_EVENTS];
                if event.kind == PARAMETER {
                    self.replay_parameters.apply(self.current.parameter(event.id).ok_or(())?)?;
                }
            }
            offset = (offset + record.event_count) % JOURNAL_EVENTS;
        }
        self.journal.clear();
        self.completed_position = 0;
        self.pending_len = 0;
        self.pending_cursor = 0;
        self.reconcile_cursor = 0;
        Ok(())
    }
    pub fn next_reconcile(&mut self) -> Option<Event> {
        // Clear confirmed old notes before reissuing currently held notes.
        while self.reconcile_cursor < LEDGER_NOTES * 2 + LEDGER_PARAMETERS {
            let i = self.reconcile_cursor;
            self.reconcile_cursor += 1;
            let event = if i < LEDGER_NOTES {
                self.old_at_ack.notes[i].map(|old| Event {
                    kind: NOTE_OFF, offset: 0, value: 0., ..old
                })
            } else if i < LEDGER_NOTES * 2 {
                self.held_at_ack.notes[i - LEDGER_NOTES].map(|mut held| {
                    held.offset = 0; held
                })
            } else {
                self.replay_parameters.parameters[i - LEDGER_NOTES * 2].map(|mut p| {
                    p.offset = 0; p
                })
            };
            if event.is_some() { return event; }
        }
        if self.pending_cursor < self.pending_len {
            let mut event = self.pending[self.pending_cursor];
            self.pending_cursor += 1;
            event.offset = 0;
            return Some(event);
        }
        None
    }
    pub fn finished(&self) -> bool {
        self.reconcile_cursor == LEDGER_NOTES * 2 + LEDGER_PARAMETERS
            && self.pending_cursor == self.pending_len
    }
    pub fn finish(&mut self) {
        self.pending_len = 0;
        self.pending_cursor = 0;
        self.replay_parameters = View::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn note(kind: u32, id: u32) -> Event {
        Event { kind, id, pitch: 60, value: if kind == NOTE_ON { 0.8 } else { 0. },
            ..Event::default() }
    }
    fn parameter(value: f64) -> Event {
        Event { kind: PARAMETER, id: 99, value, ..Event::default() }
    }
    #[test]
    fn published_note_off_is_not_confirmed_and_historical_parameter_is_not_replayed() {
        let mut l = Ledger::default();
        let on = note(NOTE_ON, 7);
        l.observe(on, Observation::Normal).unwrap();
        l.published(1, 0, 256, &[on, parameter(0.2)]);
        l.completed(1, 0, 256, 1).unwrap();
        let off = note(NOTE_OFF, 7);
        l.observe(off, Observation::Normal).unwrap();
        l.published(1, 256, 256, &[off]);
        assert!(l.confirmed.notes.iter().flatten().any(|n| n.id == 7));
        l.acknowledge(1, 1, 256).unwrap();
        assert_eq!(l.next_reconcile().unwrap().kind, NOTE_OFF);
        assert!(l.next_reconcile().is_none());
    }
    #[test]
    fn discarded_and_closed_parameters_replay_latest_only() {
        let mut l = Ledger::default();
        l.observe(parameter(0.2), Observation::Normal).unwrap();
        l.published(1, 0, 256, &[parameter(0.2)]);
        l.completed(1, 0, 256, 1).unwrap();
        l.observe(parameter(0.9), Observation::Normal).unwrap();
        l.published(1, 256, 256, &[parameter(0.9)]);
        l.observe(parameter(0.7), Observation::AdmissionClosed).unwrap();
        l.acknowledge(1, 1, 256).unwrap();
        assert_eq!(l.next_reconcile().unwrap().value, 0.7);
        assert!(l.next_reconcile().is_none());
    }
    #[test]
    fn pending_parameter_movements_keep_only_latest_value() {
        let mut l = Ledger::default();
        l.acknowledge(1, 0, 0).unwrap();
        l.observe(parameter(0.1), Observation::AfterAcknowledge).unwrap();
        l.observe(parameter(0.9), Observation::AfterAcknowledge).unwrap();
        assert_eq!(l.pending_len, 1);
        assert_eq!(l.next_reconcile().unwrap().value, 0.9);
    }
}

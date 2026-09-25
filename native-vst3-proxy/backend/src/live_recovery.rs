//! Opt-in, fixed-capacity player intent for same-instance live recovery.
//! The callback alone owns this state. No allocation occurs after selection.
use ap1_native_client::events::{Event, NOTE_OFF, NOTE_ON, PARAMETER};

pub const LEDGER_NOTES: usize = 256;
pub const LEDGER_PARAMETERS: usize = 256;
pub const PENDING_EVENTS: usize = 256;

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
    pub fn apply(&mut self, event: Event) -> Result<(), ()> {
        match event.kind {
            NOTE_ON => {
                if let Some(slot) = self
                    .notes
                    .iter_mut()
                    .find(|slot| slot.is_some_and(|v| v.id == event.id))
                {
                    *slot = Some(event);
                } else {
                    *self
                        .notes
                        .iter_mut()
                        .find(|slot| slot.is_none())
                        .ok_or(())? = Some(event);
                }
            }
            NOTE_OFF => {
                if let Some(slot) = self
                    .notes
                    .iter_mut()
                    .find(|slot| slot.is_some_and(|v| v.id == event.id))
                {
                    *slot = None;
                }
            }
            PARAMETER => {
                if let Some(slot) = self
                    .parameters
                    .iter_mut()
                    .find(|slot| slot.is_some_and(|v| v.id == event.id))
                {
                    *slot = Some(event);
                } else {
                    *self
                        .parameters
                        .iter_mut()
                        .find(|slot| slot.is_none())
                        .ok_or(())? = Some(event);
                }
            }
            _ => return Err(()),
        }
        Ok(())
    }
}

pub struct Ledger {
    pub admitted: View,
    pub current: View,
    pub held_at_ack: View,
    pub pending: [Event; PENDING_EVENTS],
    pub pending_len: usize,
    pub pending_cursor: usize,
    pub reconcile_cursor: usize,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            admitted: View::default(),
            current: View::default(),
            held_at_ack: View::default(),
            pending: [Event::default(); PENDING_EVENTS],
            pending_len: 0,
            pending_cursor: 0,
            reconcile_cursor: 0,
        }
    }
}
impl Ledger {
    pub fn observe(&mut self, event: Event, after_ack: bool) -> Result<(), ()> {
        self.current.apply(event)?;
        if after_ack {
            if event.kind == PARAMETER {
                if let Some(existing) = self.pending[self.pending_cursor..self.pending_len]
                    .iter_mut()
                    .find(|old| old.kind == PARAMETER && old.id == event.id)
                {
                    *existing = event;
                    return Ok(());
                }
            }
            if self.pending_len == PENDING_EVENTS {
                return Err(());
            }
            self.pending[self.pending_len] = event;
            self.pending_len += 1;
        }
        Ok(())
    }
    pub fn admitted(&mut self, events: &[Event]) -> Result<(), ()> {
        for event in events {
            self.admitted.apply(*event)?;
        }
        Ok(())
    }
    pub fn acknowledge(&mut self) {
        self.held_at_ack = self.current.clone();
        self.pending_len = 0;
        self.pending_cursor = 0;
        self.reconcile_cursor = 0;
    }
    pub fn next_reconcile(&mut self) -> Option<Event> {
        // Clear every old note ID first, then reissue only notes held at ack,
        // then the latest value per parameter. Later input is appended below.
        while self.reconcile_cursor < LEDGER_NOTES * 2 + LEDGER_PARAMETERS {
            let i = self.reconcile_cursor;
            self.reconcile_cursor += 1;
            let event = if i < LEDGER_NOTES {
                self.admitted.notes[i].map(|old| Event {
                    kind: NOTE_OFF,
                    offset: 0,
                    value: 0.,
                    ..old
                })
            } else if i < LEDGER_NOTES * 2 {
                self.held_at_ack.notes[i - LEDGER_NOTES].map(|mut held| {
                    held.offset = 0;
                    held
                })
            } else {
                self.held_at_ack.parameters[i - LEDGER_NOTES * 2].map(|mut p| {
                    p.offset = 0;
                    p
                })
            };
            if event.is_some() {
                return event;
            }
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
        self.admitted = self.current.clone();
        self.pending_len = 0;
        self.pending_cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_off_held_on_latest_parameter_and_post_ack_edges() {
        let mut l = Ledger::default();
        let old = Event {
            kind: NOTE_ON,
            id: 7,
            channel: 1,
            pitch: 60,
            value: 0.8,
            ..Event::default()
        };
        l.admitted(&[old]).unwrap();
        l.observe(old, false).unwrap();
        l.observe(
            Event {
                kind: NOTE_OFF,
                value: 0.,
                ..old
            },
            false,
        )
        .unwrap();
        let held = Event {
            kind: NOTE_ON,
            id: 8,
            channel: 1,
            pitch: 64,
            value: 0.7,
            ..Event::default()
        };
        l.observe(held, false).unwrap();
        l.observe(
            Event {
                kind: PARAMETER,
                id: 99,
                value: 0.2,
                ..Event::default()
            },
            false,
        )
        .unwrap();
        l.observe(
            Event {
                kind: PARAMETER,
                id: 99,
                value: 0.9,
                ..Event::default()
            },
            false,
        )
        .unwrap();
        l.acknowledge();
        let later = Event {
            kind: NOTE_ON,
            id: 9,
            channel: 1,
            pitch: 67,
            value: 0.6,
            ..Event::default()
        };
        l.observe(later, true).unwrap();
        let mut seen = Vec::new();
        while let Some(e) = l.next_reconcile() {
            seen.push(e);
        }
        assert_eq!(
            seen.iter().map(|e| (e.kind, e.id)).collect::<Vec<_>>(),
            vec![(NOTE_OFF, 7), (NOTE_ON, 8), (PARAMETER, 99), (NOTE_ON, 9)]
        );
        assert_eq!(seen[2].value, 0.9);
        assert!(l.finished());
    }
    #[test]
    fn pending_parameter_movements_keep_only_the_latest_value() {
        let mut l = Ledger::default();
        l.acknowledge();
        l.observe(
            Event {
                kind: PARAMETER,
                id: 42,
                value: 0.1,
                ..Event::default()
            },
            true,
        )
        .unwrap();
        l.observe(
            Event {
                kind: PARAMETER,
                id: 42,
                value: 0.9,
                ..Event::default()
            },
            true,
        )
        .unwrap();
        assert_eq!(l.pending_len, 1);
        assert_eq!(l.next_reconcile().unwrap().value, 0.9);
        assert!(l.next_reconcile().is_none());
    }
}

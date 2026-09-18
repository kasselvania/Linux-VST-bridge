use crate::{ALL_NOTES_OFF_PARAMETER_BASE, SUSTAIN_PARAMETER_BASE};
use ap2_backend::rpi0::{Event, NOTE_OFF, NOTE_ON, PARAMETER};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counters {
    pub accepted: u64,
    pub unsupported: u64,
    pub malformed: u64,
    pub overflow: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Refusal {
    Malformed,
    Unsupported,
    Overflow,
}

/// Stateful parser needed only to retain exact note identities. MIDI channels
/// remain distinct in every note and CC-derived parameter identity.
pub struct Parser {
    notes: [[u32; 128]; 16],
    next_note: u32,
    pub counters: Counters,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            notes: [[0; 128]; 16],
            next_note: 1,
            counters: Counters::default(),
        }
    }
}

impl Parser {
    pub fn parse(&mut self, offset: u32, bytes: &[u8], frames: u32) -> Result<Event, Refusal> {
        if offset >= frames || bytes.len() != 3 || bytes[0] < 0x80 || bytes[0] >= 0xf0 {
            self.counters.malformed += 1;
            return Err(Refusal::Malformed);
        }
        let status = bytes[0] & 0xf0;
        let channel = (bytes[0] & 0x0f) as usize;
        let data1 = bytes[1];
        let data2 = bytes[2];
        if data1 >= 128 || data2 >= 128 {
            self.counters.malformed += 1;
            return Err(Refusal::Malformed);
        }
        let event = match status {
            0x90 if data2 != 0 => {
                let id = self.next_note;
                self.next_note = self.next_note.checked_add(1).ok_or_else(|| {
                    self.counters.overflow += 1;
                    Refusal::Overflow
                })?;
                self.notes[channel][data1 as usize] = id;
                Event {
                    offset,
                    kind: NOTE_ON,
                    id,
                    channel: channel as i16,
                    pitch: data1 as i16,
                    value: f64::from(data2) / 127.0,
                    ..Event::default()
                }
            }
            0x80 | 0x90 => {
                let slot = &mut self.notes[channel][data1 as usize];
                if *slot == 0 {
                    self.counters.unsupported += 1;
                    return Err(Refusal::Unsupported);
                }
                let id = *slot;
                *slot = 0;
                Event {
                    offset,
                    kind: NOTE_OFF,
                    id,
                    channel: channel as i16,
                    pitch: data1 as i16,
                    value: f64::from(data2) / 127.0,
                    ..Event::default()
                }
            }
            0xb0 if data1 == 64 => Event {
                offset,
                kind: PARAMETER,
                id: SUSTAIN_PARAMETER_BASE + channel as u32,
                value: if data2 >= 64 { 1.0 } else { 0.0 },
                ..Event::default()
            },
            0xb0 if data1 == 123 => {
                self.notes[channel].fill(0);
                Event {
                    offset,
                    kind: PARAMETER,
                    id: ALL_NOTES_OFF_PARAMETER_BASE + channel as u32,
                    value: 1.0,
                    ..Event::default()
                }
            }
            _ => {
                self.counters.unsupported += 1;
                return Err(Refusal::Unsupported);
            }
        };
        self.counters.accepted += 1;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_messages_preserve_channel_offset_velocity_and_identity() {
        let mut parser = Parser::default();
        let on = parser.parse(17, &[0x92, 60, 96], 128).unwrap();
        assert_eq!(
            (on.kind, on.channel, on.pitch, on.offset),
            (NOTE_ON, 2, 60, 17)
        );
        assert!((on.value - 96.0 / 127.0).abs() < 1e-12);
        let sustain = parser.parse(31, &[0xb2, 64, 127], 128).unwrap();
        assert_eq!(sustain.id, SUSTAIN_PARAMETER_BASE + 2);
        let off = parser.parse(63, &[0x82, 60, 23], 128).unwrap();
        assert_eq!(off.id, on.id);
        assert_eq!(off.channel, 2);
    }

    #[test]
    fn velocity_zero_all_notes_off_and_refusals_are_explicit() {
        let mut parser = Parser::default();
        parser.parse(0, &[0x90, 48, 100], 64).unwrap();
        assert_eq!(parser.parse(9, &[0x90, 48, 0], 64).unwrap().kind, NOTE_OFF);
        parser.parse(0, &[0x9f, 72, 1], 64).unwrap();
        let all = parser.parse(10, &[0xbf, 123, 0], 64).unwrap();
        assert_eq!(all.id, ALL_NOTES_OFF_PARAMETER_BASE + 15);
        assert_eq!(
            parser.parse(64, &[0x90, 60, 1], 64),
            Err(Refusal::Malformed)
        );
        assert_eq!(
            parser.parse(0, &[0xe0, 0, 64], 64),
            Err(Refusal::Unsupported)
        );
        assert_eq!(parser.parse(0, &[0x90, 60], 64), Err(Refusal::Malformed));
    }

    #[test]
    fn channels_are_never_collapsed() {
        let mut parser = Parser::default();
        let a = parser.parse(0, &[0x90, 60, 10], 64).unwrap();
        let b = parser.parse(0, &[0x91, 60, 10], 64).unwrap();
        assert_ne!(a.id, b.id);
        assert_eq!(a.channel, 0);
        assert_eq!(b.channel, 1);
    }
}

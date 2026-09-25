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

/// The reference instrument owns synthetic CC parameter IDs. Commercial note
/// input must never inherit those identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControllerPolicy {
    ReferenceInstrument,
    TrackedNoteOffs,
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
    /// Writes a complete message translation into caller-owned storage. CC123
    /// expands to at most 128 note-offs on its own channel. Capacity refusal
    /// leaves tracked notes intact; an empty channel is an accepted no-op.
    pub fn parse_into(
        &mut self,
        policy: ControllerPolicy,
        offset: u32,
        bytes: &[u8],
        frames: u32,
        output: &mut [Event],
    ) -> Result<usize, Refusal> {
        if policy == ControllerPolicy::TrackedNoteOffs {
            if offset >= frames
                || bytes.len() != 3
                || bytes[0] < 0x80
                || bytes[0] >= 0xf0
                || bytes[1] >= 128
                || bytes[2] >= 128
            {
                self.counters.malformed += 1;
                return Err(Refusal::Malformed);
            }
            let status = bytes[0] & 0xf0;
            let channel = usize::from(bytes[0] & 0x0f);
            if status == 0xb0 && bytes[1] == 123 {
                let notes = &mut self.notes[channel];
                let count = notes.iter().filter(|&&id| id != 0).count();
                if output.len() < count {
                    self.counters.overflow += 1;
                    return Err(Refusal::Overflow);
                }
                let mut index = 0;
                for (pitch, id) in notes.iter_mut().enumerate() {
                    if *id != 0 {
                        output[index] = Event {
                            offset,
                            kind: NOTE_OFF,
                            id: *id,
                            channel: channel as i16,
                            pitch: pitch as i16,
                            ..Event::default()
                        };
                        *id = 0;
                        index += 1;
                    }
                }
                self.counters.accepted += 1;
                return Ok(count);
            }
            // No commercial controller assignment has been authenticated.
            // Refuse unsupported CCs (including sustain), never invent ParamIDs.
            // One outstanding note per channel/pitch is the bounded contract;
            // refuse a duplicate note-on rather than lose its older note ID.
            if !matches!(status, 0x80 | 0x90)
                || (status == 0x90
                    && bytes[2] != 0
                    && self.notes[channel][usize::from(bytes[1])] != 0)
            {
                self.counters.unsupported += 1;
                return Err(Refusal::Unsupported);
            }
        }
        if output.is_empty() {
            self.counters.overflow += 1;
            return Err(Refusal::Overflow);
        }
        output[0] = self.parse(offset, bytes, frames)?;
        Ok(1)
    }

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
    fn commercial_all_notes_off_preserves_channel_pitch_id_and_offset() {
        let mut parser = Parser::default();
        let mut output = [Event::default(); 128];
        let policy = ControllerPolicy::TrackedNoteOffs;
        parser
            .parse_into(policy, 1, &[0x92, 60, 96], 512, &mut output)
            .unwrap();
        let first = output[0];
        parser
            .parse_into(policy, 2, &[0x92, 64, 96], 512, &mut output)
            .unwrap();
        let second = output[0];
        parser
            .parse_into(policy, 3, &[0x93, 60, 96], 512, &mut output)
            .unwrap();
        let other = output[0];
        assert_eq!(
            parser.parse_into(policy, 17, &[0xb2, 123, 0], 512, &mut output),
            Ok(2)
        );
        for (event, original) in output[..2].iter().zip([first, second]) {
            assert_eq!(
                (
                    event.kind,
                    event.channel,
                    event.pitch,
                    event.id,
                    event.offset
                ),
                (NOTE_OFF, 2, original.pitch, original.id, 17)
            );
        }
        assert_eq!(
            parser.parse_into(policy, 18, &[0xb2, 123, 0], 512, &mut []),
            Ok(0)
        );
        parser
            .parse_into(policy, 19, &[0x83, 60, 0], 512, &mut output)
            .unwrap();
        assert_eq!(output[0].id, other.id);
        assert_eq!(parser.counters.accepted, 6);
    }

    #[test]
    fn all_notes_off_capacity_refusal_is_atomic_and_retriable() {
        let mut parser = Parser::default();
        let mut output = [Event::default(); 128];
        let policy = ControllerPolicy::TrackedNoteOffs;
        for pitch in 0..128 {
            parser
                .parse_into(policy, 0, &[0x90, pitch, 96], 512, &mut output)
                .unwrap();
        }
        output.fill(Event::default());
        assert_eq!(
            parser.parse_into(policy, 8, &[0xb0, 123, 0], 512, &mut output[..127]),
            Err(Refusal::Overflow)
        );
        assert!(output.iter().all(|e| e.id == 0));
        assert_eq!(parser.counters.accepted, 128);
        assert_eq!(
            parser.parse_into(policy, 8, &[0xb0, 123, 0], 512, &mut output),
            Ok(128)
        );
        for (pitch, event) in output.iter().enumerate() {
            assert_eq!(
                (event.kind, event.pitch, event.id),
                (NOTE_OFF, pitch as i16, pitch as u32 + 1)
            );
        }
        assert_eq!(
            parser.parse_into(policy, 9, &[0xb0, 123, 0], 512, &mut []),
            Ok(0)
        );
        assert_eq!(parser.counters.overflow, 1);
    }

    #[test]
    fn exact_qualification_sequence_never_emits_a_reference_parameter() {
        let mut parser = Parser::default();
        let mut output = [Event::default(); 128];
        let policy = ControllerPolicy::TrackedNoteOffs;
        for (bytes, count) in [([0x90, 60, 96], 1), ([0x80, 60, 0], 1), ([0xb0, 123, 0], 0)] {
            assert_eq!(
                parser.parse_into(policy, 0, &bytes, 512, &mut output),
                Ok(count)
            );
            assert!(output[..count].iter().all(|e| e.kind != PARAMETER));
        }
        assert_eq!(
            parser.counters,
            Counters {
                accepted: 3,
                ..Counters::default()
            }
        );
        assert_eq!(
            parser.parse_into(policy, 0, &[0xb0, 64, 127], 512, &mut output),
            Err(Refusal::Unsupported)
        );
    }

    #[test]
    fn commercial_duplicate_and_invalid_messages_preserve_the_live_note() {
        let mut parser = Parser::default();
        let mut output = [Event::default(); 128];
        let policy = ControllerPolicy::TrackedNoteOffs;
        parser
            .parse_into(policy, 0, &[0x9f, 72, 96], 512, &mut output)
            .unwrap();
        let id = output[0].id;
        assert_eq!(
            parser.parse_into(policy, 1, &[0x9f, 72, 127], 512, &mut output),
            Err(Refusal::Unsupported)
        );
        for (offset, bytes) in [
            (512, &[0xbf, 123, 0][..]),
            (0, &[0xbf, 123][..]),
            (0, &[0xbf, 123, 128][..]),
        ] {
            assert_eq!(
                parser.parse_into(policy, offset, bytes, 512, &mut output),
                Err(Refusal::Malformed)
            );
        }
        assert_eq!(
            parser.parse_into(policy, 9, &[0x9f, 72, 0], 512, &mut []),
            Err(Refusal::Overflow)
        );
        parser
            .parse_into(policy, 9, &[0x9f, 72, 0], 512, &mut output)
            .unwrap();
        assert_eq!((output[0].kind, output[0].id), (NOTE_OFF, id));
    }

    #[test]
    fn reference_policy_keeps_its_owned_controller_identity() {
        let mut parser = Parser::default();
        let mut output = [Event::default(); 1];
        assert_eq!(
            parser.parse_into(
                ControllerPolicy::ReferenceInstrument,
                3,
                &[0xbf, 123, 0],
                512,
                &mut output
            ),
            Ok(1)
        );
        assert_eq!(
            (output[0].kind, output[0].id),
            (PARAMETER, ALL_NOTES_OFF_PARAMETER_BASE + 15)
        );
    }

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

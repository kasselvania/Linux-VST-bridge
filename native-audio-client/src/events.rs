//! Bounded VST3 note/parameter input carried with its admitted audio block.
//! These are value records, not SDK objects or opaque MIDI byte messages.
use crate::{get, invalid, need, put, CAP};
use std::io;
pub const MAX_EVENTS: usize = 256;
pub const EVENT_BYTES: usize = 32;
pub const NOTE_ON: u32 = 0;
pub const NOTE_OFF: u32 = 1;
pub const PARAMETER: u32 = 2;
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Event {
    pub offset: u32,
    pub kind: u32,
    pub id: u32,
    pub channel: i16,
    pub pitch: i16,
    pub value: f64,
    pub tuning: f32,
    pub reserved: u32,
}
impl Event {
    pub fn valid(&self, frames: usize) -> bool {
        frames <= CAP
            && (self.offset as usize) < frames.max(1)
            && self.value.is_finite()
            && (0.0..=1.0).contains(&self.value)
            && self.reserved == 0
            && match self.kind {
                NOTE_ON | NOTE_OFF => {
                    frames > 0
                        && (0..16).contains(&self.channel)
                        && (0..128).contains(&self.pitch)
                        && self.tuning.is_finite()
                }
                PARAMETER => self.channel == 0 && self.pitch == 0 && self.tuning == 0.,
                _ => false,
            }
    }
}
pub fn encode(events: &[Event], frames: usize) -> io::Result<Vec<u8>> {
    need(events.len() <= MAX_EVENTS, "event capacity")?;
    let mut bytes = vec![0; 8 + EVENT_BYTES * events.len()];
    put(&mut bytes[..4], events.len() as u64);
    for (e, b) in events.iter().zip(bytes[8..].chunks_exact_mut(EVENT_BYTES)) {
        need(e.valid(frames), "event value/extent")?;
        put(&mut b[..4], e.offset as u64);
        put(&mut b[4..8], e.kind as u64);
        put(&mut b[8..12], e.id as u64);
        b[12..14].copy_from_slice(&e.channel.to_le_bytes());
        b[14..16].copy_from_slice(&e.pitch.to_le_bytes());
        b[16..24].copy_from_slice(&e.value.to_le_bytes());
        b[24..28].copy_from_slice(&e.tuning.to_le_bytes());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8], frames: usize) -> io::Result<Vec<Event>> {
    need(bytes.len() >= 8, "event header")?;
    let n = get(&bytes[..4]) as usize;
    need(
        n <= MAX_EVENTS && get(&bytes[4..8]) == 0 && bytes.len() == 8 + n * EVENT_BYTES,
        "event count/length/reserved",
    )?;
    bytes[8..]
        .chunks_exact(EVENT_BYTES)
        .map(|b| {
            let e = Event {
                offset: get(&b[..4]) as u32,
                kind: get(&b[4..8]) as u32,
                id: get(&b[8..12]) as u32,
                channel: i16::from_le_bytes(b[12..14].try_into().unwrap()),
                pitch: i16::from_le_bytes(b[14..16].try_into().unwrap()),
                value: f64::from_le_bytes(b[16..24].try_into().unwrap()),
                tuning: f32::from_le_bytes(b[24..28].try_into().unwrap()),
                reserved: get(&b[28..32]) as u32,
            };
            if e.valid(frames) {
                Ok(e)
            } else {
                Err(invalid("event value/extent"))
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_notes_and_real_parameter_ids_roundtrip_without_coalescing() {
        let events = [
            Event {
                kind: NOTE_ON,
                offset: 7,
                id: 123,
                pitch: 60,
                channel: 3,
                value: 0.75,
                tuning: -3.5,
                ..Event::default()
            },
            Event {
                kind: PARAMETER,
                offset: 19,
                id: 0xa0051001,
                value: 0.125,
                ..Event::default()
            },
            Event {
                kind: PARAMETER,
                offset: 31,
                id: 0xa0051001,
                value: 0.875,
                ..Event::default()
            },
            Event {
                kind: NOTE_OFF,
                offset: 93,
                id: 123,
                pitch: 60,
                channel: 3,
                value: 0.25,
                ..Event::default()
            },
        ];
        let wire = encode(&events, 128).unwrap();
        assert_eq!(decode(&wire, 128).unwrap(), events);
        assert!(decode(&wire, 64).is_err());
        assert!(decode(&wire[..wire.len() - 1], 128).is_err());
        let mut corrupt = wire.clone();
        corrupt[8 + 28] = 1;
        assert!(decode(&corrupt, 128).is_err());
        assert!(encode(&[events[0]; MAX_EVENTS + 1], 128).is_err());
        assert!(encode(&[events[0]], 0).is_err());
        let flush = Event {
            kind: PARAMETER,
            id: 900,
            value: 0.5,
            ..Event::default()
        };
        assert!(encode(&[flush], 0).is_ok());
    }
}

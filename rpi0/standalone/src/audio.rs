use crate::{midi::Parser, spsc::Queue, MAX_MIDI_EVENTS};
use ap2_backend::rpi0::{Event, PARAMETER};

/// Bounded, allocation-free observation of samples actually returned to JACK.
#[derive(Default, Debug, PartialEq)]
pub struct OutputObservation {
    pub nonzero: u64,
    pub nonfinite: u64,
    pub peak: f32,
}

impl OutputObservation {
    pub fn measure(samples: &[f32]) -> Self {
        let mut result = Self::default();
        for &sample in samples {
            if !sample.is_finite() {
                result.nonfinite += 1;
            } else {
                result.nonzero += u64::from(sample != 0.0);
                result.peak = result.peak.max(sample.abs());
            }
        }
        result
    }
}

/// Apply the host's final output level. Bypass selects current stereo input
/// while processing continues, so returning to wet output does not revive old
/// queued work. Both routes receive the same output trim.
pub fn present_output(inputs: [&[f32]; 2], outputs: [&mut [f32]; 2], bypass: bool, trim: f32) {
    let [left, right] = outputs;
    for (input, output) in inputs.into_iter().zip([left, right]) {
        debug_assert_eq!(input.len(), output.len());
        if bypass {
            for (destination, source) in output.iter_mut().zip(input) {
                *destination = *source * trim;
            }
        } else if trim != 1.0 {
            for sample in output {
                *sample *= trim;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParameterUpdate {
    pub id: u32,
    pub value: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MidiPacket {
    pub offset: u32,
    pub size: u8,
    pub bytes: [u8; 3],
}

pub struct BlockEvents {
    values: [Event; MAX_MIDI_EVENTS],
    count: usize,
}

impl Default for BlockEvents {
    fn default() -> Self {
        Self {
            values: [Event::default(); MAX_MIDI_EVENTS],
            count: 0,
        }
    }
}

impl BlockEvents {
    pub fn prepare<const N: usize>(
        &mut self,
        parser: &mut Parser,
        updates: &Queue<ParameterUpdate, N>,
        midi: &[MidiPacket],
        frames: u32,
    ) -> usize {
        self.count = 0;
        while let Some(update) = updates.pop() {
            if self.count == self.values.len() {
                break;
            }
            if update.value.is_finite() && (0.0..=1.0).contains(&update.value) {
                self.values[self.count] = Event {
                    kind: PARAMETER,
                    id: update.id,
                    value: update.value,
                    ..Event::default()
                };
                self.count += 1;
            }
        }
        for packet in midi {
            if self.count == self.values.len() {
                break;
            }
            if packet.size == 3 {
                if let Ok(event) = parser.parse(packet.offset, &packet.bytes, frames) {
                    self.values[self.count] = event;
                    self.count += 1;
                }
            } else {
                let _ = parser.parse(
                    packet.offset,
                    &packet.bytes[..packet.size.min(3) as usize],
                    frames,
                );
            }
        }
        self.count
    }

    pub fn as_slice(&self) -> &[Event] {
        &self.values[..self.count]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GAIN_PARAMETER, SUSTAIN_PARAMETER_BASE};
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct Audit;
    static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
    thread_local! { static TRACK: Cell<bool> = const { Cell::new(false) }; }
    unsafe impl GlobalAlloc for Audit {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            TRACK.with(|track| {
                if track.get() {
                    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
                }
            });
            System.alloc(layout)
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
            TRACK.with(|track| {
                if track.get() {
                    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
                }
            });
            System.realloc(ptr, layout, size)
        }
    }
    #[global_allocator]
    static AUDIT: Audit = Audit;

    #[test]
    fn full_channel_note_off_expansion_allocates_nothing() {
        let mut parser = Parser::default();
        let mut output = [Event::default(); 128];
        let policy = crate::midi::ControllerPolicy::TrackedNoteOffs;
        for pitch in 0..128 {
            parser
                .parse_into(policy, 0, &[0x90, pitch, 96], 512, &mut output)
                .unwrap();
        }
        ALLOCATIONS.store(0, Ordering::Relaxed);
        TRACK.with(|track| track.set(true));
        let count = parser.parse_into(policy, 17, &[0xb0, 123, 0], 512, &mut output);
        TRACK.with(|track| track.set(false));
        assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
        assert_eq!(count, Ok(128));
    }

    #[test]
    fn output_observation_distinguishes_silence_signed_audio_and_nonfinite_without_allocation() {
        ALLOCATIONS.store(0, Ordering::Relaxed);
        TRACK.with(|track| track.set(true));
        let observed =
            OutputObservation::measure(&[0.0, -0.0, -0.75, 0.25, f32::NAN, f32::INFINITY]);
        TRACK.with(|track| track.set(false));
        assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
        assert_eq!(
            observed,
            OutputObservation {
                nonzero: 2,
                nonfinite: 2,
                peak: 0.75
            }
        );
        assert_eq!(
            OutputObservation::measure(&[0.0; 512]),
            OutputObservation::default()
        );
    }

    #[test]
    fn output_presentation_selects_current_dry_input_and_trims_both_routes_without_allocation() {
        let dry_left = [0.25, -0.5];
        let dry_right = [0.75, -0.25];
        let mut wet_left = [1.0, -1.0];
        let mut wet_right = [0.5, -0.5];
        ALLOCATIONS.store(0, Ordering::Relaxed);
        TRACK.with(|track| track.set(true));
        present_output(
            [&dry_left, &dry_right],
            [&mut wet_left, &mut wet_right],
            false,
            0.5,
        );
        assert_eq!(wet_left, [0.5, -0.5]);
        assert_eq!(wet_right, [0.25, -0.25]);
        present_output(
            [&dry_left, &dry_right],
            [&mut wet_left, &mut wet_right],
            true,
            0.5,
        );
        TRACK.with(|track| track.set(false));
        assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
        assert_eq!(wet_left, [0.125, -0.25]);
        assert_eq!(wet_right, [0.375, -0.125]);
    }

    #[test]
    fn callback_event_preparation_is_allocation_free_and_bounded() {
        let queue = Queue::<ParameterUpdate, 8>::new();
        queue
            .push(ParameterUpdate {
                id: GAIN_PARAMETER,
                value: 0.25,
            })
            .unwrap();
        let packets = [
            MidiPacket {
                offset: 3,
                size: 3,
                bytes: [0x91, 64, 100],
            },
            MidiPacket {
                offset: 7,
                size: 3,
                bytes: [0xb1, 64, 127],
            },
        ];
        let mut block = BlockEvents::default();
        let mut parser = Parser::default();
        ALLOCATIONS.store(0, Ordering::Relaxed);
        TRACK.with(|track| track.set(true));
        let count = block.prepare(&mut parser, &queue, &packets, 64);
        TRACK.with(|track| track.set(false));
        assert_eq!(ALLOCATIONS.load(Ordering::Relaxed), 0);
        assert_eq!(count, 3);
        assert_eq!(block.as_slice()[0].id, GAIN_PARAMETER);
        assert_eq!(block.as_slice()[2].id, SUSTAIN_PARAMETER_BASE + 1);
    }
}

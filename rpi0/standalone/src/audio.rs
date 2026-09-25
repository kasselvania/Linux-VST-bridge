use crate::{midi::Parser, spsc::Queue, MAX_MIDI_EVENTS};
use ap2_backend::rpi0::{Event, PARAMETER};

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

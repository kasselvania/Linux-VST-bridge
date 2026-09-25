//! Deterministic, DAW-free ARM appliance edge for RPI0.
pub mod audio;
pub mod callback_gate;
#[cfg(all(target_os = "linux", feature = "jack-runtime"))]
pub mod jack;
pub mod midi;
pub mod protocol;
pub mod spsc;

pub const SAMPLE_RATE: u32 = 48_000;
pub const MAX_JACK_FRAMES: usize = 1024;
pub const MAX_MIDI_EVENTS: usize = 256;
pub const GAIN_PARAMETER: u32 = 0x5250_0001;
pub const SUSTAIN_PARAMETER_BASE: u32 = 0x5250_0100;
pub const ALL_NOTES_OFF_PARAMETER_BASE: u32 = 0x5250_0200;

const _: () = assert!(cfg!(target_endian = "little"));
const _: () = assert!(cfg!(target_pointer_width = "64"));

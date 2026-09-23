//! Bounded JACK qualification fixture; no bridge or vendor-runtime implementation.
pub const RATE: u64 = 48_000;
pub const NOTE_EVENTS: [(u64, [u8; 3]); 3] = [
    (RATE / 2, [0x90, 60, 96]),
    (RATE * 2, [0x80, 60, 0]),
    (RATE * 5 / 2, [0xb0, 123, 0]),
];

pub const VOICE_STEPS: [usize; 5] = [1, 2, 4, 6, 8];
pub const CHORD_NOTES: [u8; 8] = [48, 52, 55, 59, 62, 65, 69, 72];

/// Identical two-key stimulus for editor/headless comparison: C4/G4 at velocity
/// 96, held from second 2 through second 14 of a 20-second capture.
pub fn two_note_events() -> Vec<(u64, [u8; 3])> {
    held_note_events(&[60, 67])
}

/// Adds E4/B4 to the two-note comparison, preserving its timing and velocity.
pub fn four_note_events() -> Vec<(u64, [u8; 3])> {
    held_note_events(&[60, 64, 67, 71])
}

fn held_note_events(notes: &[u8]) -> Vec<(u64, [u8; 3])> {
    let mut events = Vec::with_capacity(notes.len() * 2 + 1);
    for &pitch in notes { events.push((RATE * 2, [0x90, pitch, 96])); }
    for &pitch in notes { events.push((RATE * 14, [0x80, pitch, 0])); }
    events.push((RATE * 18, [0xb0, 123, 0]));
    events
}

/// A fixed channel-1 chord ladder. Each six-second window has three seconds
/// of held notes followed by note-offs, CC123 and a quiet tail.
pub fn polyphony_events() -> Vec<(u64, [u8; 3])> {
    let mut events = Vec::with_capacity(47);
    for (step, voices) in VOICE_STEPS.into_iter().enumerate() {
        let start = step as u64 * RATE * 6;
        for &pitch in &CHORD_NOTES[..voices] {
            events.push((start + RATE / 2, [0x90, pitch, 96]));
        }
        for &pitch in &CHORD_NOTES[..voices] {
            events.push((start + RATE * 7 / 2, [0x80, pitch, 0]));
        }
        events.push((start + RATE * 4, [0xb0, 123, 0]));
    }
    events
}

/// Two minutes of repeated eight-key chords. Releases may overlap the next
/// chord; only the final CC123 clears remaining notes. Allocate before activation.
pub fn stress_events() -> Vec<(u64, [u8; 3])> {
    let mut events = Vec::with_capacity(481);
    for cycle in 0..30 {
        let start = cycle * RATE * 4;
        for pitch in CHORD_NOTES {
            events.push((start + RATE / 4, [0x90, pitch, 96]));
        }
        for pitch in CHORD_NOTES {
            events.push((start + RATE * 7 / 2, [0x80, pitch, 0]));
        }
    }
    events.push((RATE * 120 - RATE / 4, [0xb0, 123, 0]));
    events
}

/// Optional fixture recording, allocated and touched before JACK activation.
/// The callback only copies into this fixed extent; file I/O happens after retirement.
pub struct Capture {
    frames: Vec<[f32; 2]>,
}
impl Capture {
    pub fn new(frames: usize) -> Self {
        let mut frames = vec![[0.0; 2]; frames];
        // Force writable backing pages now; a zero-filled allocation alone may
        // otherwise defer its first physical page writes into the callback.
        for frame in &mut frames {
            unsafe {
                std::ptr::write_volatile(frame, [0.0; 2]);
            }
        }
        Self { frames }
    }
    pub fn store_channel(&mut self, start: usize, channel: usize, samples: &[f32]) -> bool {
        let Some(end) = start.checked_add(samples.len()) else {
            return false;
        };
        if channel >= 2 || end > self.frames.len() {
            return false;
        }
        for (frame, &sample) in self.frames[start..end].iter_mut().zip(samples) {
            frame[channel] = sample;
        }
        true
    }
    pub fn write_interleaved(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        for frame in &self.frames {
            for sample in frame {
                output.write_all(&sample.to_le_bytes())?;
            }
        }
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct Meter {
    pub samples: u64,
    pub nonzero: u64,
    pub nonzero_windows: u64,
    pub nonfinite: u64,
    pub peak: f32,
    pub squares: f64,
}
impl Meter {
    pub fn observe(&mut self, samples: &[f32]) {
        let before = self.nonzero;
        self.samples += samples.len() as u64;
        for &sample in samples {
            if !sample.is_finite() {
                self.nonfinite += 1;
                continue;
            }
            self.nonzero += u64::from(sample != 0.0);
            self.peak = self.peak.max(sample.abs());
            self.squares += f64::from(sample) * f64::from(sample);
        }
        self.nonzero_windows += u64::from(self.nonzero != before);
    }
    pub fn rms(&self) -> f64 {
        if self.samples == 0 {
            0.0
        } else {
            (self.squares / self.samples as f64).sqrt()
        }
    }
}

pub fn note_offset(event: u64, start: u64, frames: u32) -> Option<u32> {
    if event >= start && event < start + u64::from(frames) {
        Some((event - start) as u32)
    } else {
        None
    }
}

pub fn tone(frame: u64, channel: usize) -> f32 {
    let Some(at) = frame.checked_sub(RATE / 4) else {
        return 0.0;
    };
    if at >= RATE * 2 {
        return 0.0;
    }
    let ramp = (at.min(RATE * 2 - 1 - at) as f64 / 480.0).min(1.0);
    let hz = if channel == 0 { 440.0 } else { 660.0 };
    (0.05 * ramp * (std::f64::consts::TAU * hz * at as f64 / RATE as f64).sin()) as f32
}

#[cfg(all(target_os = "linux", not(test)))]
#[path = "qualification_jack.rs"]
mod live;

fn main() {
    #[cfg(all(target_os = "linux", not(test)))]
    if let Err(error) = live::run() {
        eprintln!("RPI1_QUALIFICATION_REFUSED {error}");
        std::process::exit(1);
    }
    #[cfg(not(target_os = "linux"))]
    panic!("physical qualification requires Linux JACK");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_notes_extend_comparison_without_changing_hold_or_velocity() {
        let events = four_note_events();
        assert_eq!(events.len(), 9);
        let mut held = [false; 128];
        for &(at, bytes) in &events {
            match bytes[0] {
                0x90 => {
                    assert_eq!(at, RATE * 2);
                    assert_eq!(bytes[2], 96);
                    assert!(!held[bytes[1] as usize]);
                    held[bytes[1] as usize] = true;
                }
                0x80 => {
                    assert_eq!(at, RATE * 14);
                    assert!(held[bytes[1] as usize]);
                    held[bytes[1] as usize] = false;
                }
                0xb0 => assert!(held.iter().all(|&v| !v)),
                _ => panic!("unexpected event"),
            }
        }
        assert!(held.iter().all(|&v| !v));
        for event in two_note_events() { assert!(events.contains(&event)); }
    }
    #[test]
    fn two_notes_are_balanced_and_keep_offsets_across_periods() {
        let events = two_note_events();
        let mut held = [false; 128];
        for &(at, bytes) in &events {
            assert!(at < RATE * 20);
            match bytes[0] {
                0x90 => { assert!(!held[bytes[1] as usize]); held[bytes[1] as usize] = true; }
                0x80 => { assert!(held[bytes[1] as usize]); held[bytes[1] as usize] = false; }
                0xb0 => assert!(held.iter().all(|&v| !v)),
                _ => panic!("unexpected event"),
            }
            assert!(held.iter().filter(|&&v| v).count() <= 2);
        }
        assert!(held.iter().all(|&v| !v));
        assert_eq!(events[2].0 - events[0].0, RATE * 12);
        for period in [127, 512] {
            let mut emitted = Vec::new();
            for start in (0..RATE * 20).step_by(period) {
                for &(at, bytes) in &events {
                    if let Some(offset) = note_offset(at, start, period as u32) {
                        emitted.push((start + u64::from(offset), bytes));
                    }
                }
            }
            assert_eq!(emitted, events);
        }
    }
    #[test]
    fn two_minute_stress_has_eight_keys_and_no_unreleased_notes() {
        let events = stress_events();
        assert_eq!(events.len(), 481);
        assert!(events.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert!(events.iter().all(|(at, _)| *at < RATE * 120));
        let mut held = [false; 128];
        let mut peaks = 0;
        for &(_, bytes) in &events {
            match bytes[0] {
                0x90 => {
                    assert!(!held[bytes[1] as usize]);
                    held[bytes[1] as usize] = true;
                    if held.iter().filter(|&&v| v).count() == 8 { peaks += 1; }
                }
                0x80 => {
                    assert!(held[bytes[1] as usize]);
                    held[bytes[1] as usize] = false;
                }
                0xb0 => assert!(held.iter().all(|&v| !v)),
                _ => panic!("unexpected stress event"),
            }
        }
        assert_eq!(peaks, 30);
        assert!(held.iter().all(|&v| !v));
        for period in [127, 512] {
            let mut emitted = Vec::new();
            for start in (0..RATE * 120).step_by(period) {
                for &(at, bytes) in &events {
                    if let Some(offset) = note_offset(at, start, period as u32) {
                        emitted.push((start + u64::from(offset), bytes));
                    }
                }
            }
            assert_eq!(emitted, events);
        }
    }
    #[test]
    fn chord_ladder_preserves_offsets_balances_notes_and_stays_bounded() {
        let expected = polyphony_events();
        assert_eq!(expected.len(), 47);
        for period in [64, 128, 256, 512, 1024] {
            let mut emitted = Vec::new();
            for start in (0..RATE * 30).step_by(period) {
                for &(at, bytes) in &expected {
                    if let Some(offset) = note_offset(at, start, period as u32) {
                        emitted.push((start + u64::from(offset), bytes));
                    }
                }
            }
            assert_eq!(emitted, expected);
        }
        let mut held = [false; 128];
        for &(_, bytes) in &expected {
            match bytes[0] {
                0x90 => {
                    assert!(!held[bytes[1] as usize]);
                    held[bytes[1] as usize] = true;
                }
                0x80 => {
                    assert!(held[bytes[1] as usize]);
                    held[bytes[1] as usize] = false;
                }
                0xb0 => assert!(held.iter().all(|&v| !v)),
                _ => panic!("unexpected event"),
            }
        }
        assert!(held.iter().all(|&v| !v));
    }
    #[test]
    fn capture_preserves_stereo_bits_and_refuses_out_of_bounds_writes() {
        let mut capture = Capture::new(3);
        assert!(capture.store_channel(0, 0, &[0.25, -0.5]));
        assert!(capture.store_channel(2, 0, &[-0.0]));
        assert!(capture.store_channel(0, 1, &[1.0, 0.0, -1.0]));
        assert!(!capture.store_channel(2, 0, &[8.0, 9.0]));
        assert!(!capture.store_channel(0, 2, &[8.0]));
        assert!(!capture.store_channel(usize::MAX, 0, &[8.0]));
        let mut bytes = Vec::new();
        capture.write_interleaved(&mut bytes).unwrap();
        let expected = [0.25_f32, 1.0, -0.5, 0.0, -0.0, -1.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(bytes, expected);
    }
    #[test]
    fn silence_nonfinite_and_stereo_statistics_are_distinct() {
        let mut meter = Meter::default();
        meter.observe(&[0.0, -0.0]);
        meter.observe(&[-0.5, 0.5, f32::NAN, f32::INFINITY]);
        assert_eq!(
            (
                meter.samples,
                meter.nonzero,
                meter.nonzero_windows,
                meter.nonfinite
            ),
            (6, 2, 1, 2)
        );
        assert_eq!(meter.peak, 0.5);
        assert!((meter.rms() - (0.5_f64 / 6.0).sqrt()).abs() < 1e-12);
    }
    #[test]
    fn exact_sequence_is_emitted_once_across_callback_boundaries() {
        for period in [64, 128, 256, 512, 1024] {
            let mut emitted = Vec::new();
            for start in (0..RATE * 5).step_by(period) {
                for (time, bytes) in NOTE_EVENTS {
                    if let Some(offset) = note_offset(time, start, period as u32) {
                        emitted.push((start + u64::from(offset), bytes));
                    }
                }
            }
            assert_eq!(emitted, NOTE_EVENTS);
        }
    }
    #[test]
    fn tone_has_bounded_duration_level_and_two_nonzero_channels() {
        for ch in 0..2 {
            let values = (0..RATE * 3)
                .map(|frame| tone(frame, ch))
                .collect::<Vec<_>>();
            let mut meter = Meter::default();
            meter.observe(&values);
            assert!(meter.nonzero > 90_000 && meter.peak <= 0.05);
            assert!(meter.rms() > 0.02);
            assert!(values[..12_000].iter().all(|v| *v == 0.0));
            assert!(values[108_000..].iter().all(|v| *v == 0.0));
        }
    }
}

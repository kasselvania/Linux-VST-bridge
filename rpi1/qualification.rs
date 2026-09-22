//! Bounded JACK qualification fixture; no bridge or vendor-runtime implementation.
pub const RATE: u64 = 48_000;
pub const NOTE_EVENTS: [(u64, [u8; 3]); 3] = [
    (RATE / 2, [0x90, 60, 96]),
    (RATE * 2, [0x80, 60, 0]),
    (RATE * 5 / 2, [0xb0, 123, 0]),
];

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

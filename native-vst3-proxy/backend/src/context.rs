//! AP10 context v1: fixed C ABI and explicit little-endian wire fields.
//! Unavailable SDK fields have their validity flags cleared by the native edge.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Context {
    pub present: u32,
    pub state: u32,
    pub rate: f64,
    pub project: i64,
    pub system: i64,
    pub continuous: i64,
    pub music: f64,
    pub bar: f64,
    pub cycle_start: f64,
    pub cycle_end: f64,
    pub tempo: f64,
    pub numerator: i32,
    pub denominator: i32,
    pub clock: i32,
    pub reserved: u32,
}
impl Context {
    pub fn valid(&self) -> bool {
        self.present <= 1
            && self.reserved == 0
            && self.state & !0x2bf0e == 0
            && (self.present == 0
                || (self.rate.is_finite()
                    && self.rate > 0.
                    && (self.state & 0x200 == 0 || self.music.is_finite())
                    && (self.state & 0x800 == 0 || self.bar.is_finite())
                    && (self.state & 0x1000 == 0
                        || self.cycle_start.is_finite()
                            && self.cycle_end.is_finite()
                            && self.cycle_end > self.cycle_start)
                    && (self.state & 0x400 == 0 || self.tempo.is_finite() && self.tempo > 0.)
                    && (self.state & 0x2000 == 0 || self.numerator > 0 && self.denominator > 0)))
    }
    pub fn chunk(mut self, offset: usize) -> Option<Self> {
        if !self.valid() {
            return None;
        }
        if self.present == 0 || offset == 0 {
            return Some(self);
        }
        if self.state & 2 != 0 {
            self.project = self.project.checked_add(offset as i64)?;
            if self.state & 0x600 == 0x600 {
                self.music += offset as f64 * self.tempo / (60. * self.rate);
                if self.state & 0x1004 == 0x1004 && self.music >= self.cycle_end {
                    let span = self.cycle_end - self.cycle_start;
                    let loops = ((self.music - self.cycle_start) / span).floor();
                    self.music -= loops * span;
                    self.project = self
                        .project
                        .checked_sub((loops * span * 60. * self.rate / self.tempo).round() as i64)?;
                }
            } else {
                self.state &= !0x200;
            }
            // The original bar may no longer contain this chunk.
            self.state &= !0x800;
        }
        if self.state & 0x20000 != 0 {
            self.continuous = self.continuous.checked_add(offset as i64)?;
        }
        if self.state & 0x100 != 0 {
            self.system = self
                .system
                .checked_add((offset as f64 * 1e9 / self.rate).round() as i64)?;
        }
        if self.state & 0x8000 != 0 {
            self.clock = self.clock.checked_sub(offset as i32)?;
        }
        Some(self)
    }
    pub fn encode(self) -> Vec<u8> {
        let mut b = Vec::with_capacity(96);
        b.extend(self.present.to_le_bytes());
        b.extend(self.state.to_le_bytes());
        b.extend(self.rate.to_le_bytes());
        for v in [self.project, self.system, self.continuous] {
            b.extend(v.to_le_bytes());
        }
        for v in [
            self.music,
            self.bar,
            self.cycle_start,
            self.cycle_end,
            self.tempo,
        ] {
            b.extend(v.to_le_bytes());
        }
        for v in [self.numerator, self.denominator, self.clock] {
            b.extend(v.to_le_bytes());
        }
        b.extend(self.reserved.to_le_bytes());
        b
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_context_chunking_and_absence() {
        assert_eq!(std::mem::size_of::<Context>(), 96);
        let c = Context {
            present: 1,
            state: 0x20f02,
            rate: 48000.,
            project: 48000,
            continuous: 96000,
            system: 1_000_000_000,
            music: 2.,
            bar: 0.,
            tempo: 120.,
            ..Default::default()
        };
        let d = c.chunk(256).unwrap();
        assert_eq!(d.project, 48256);
        assert_eq!(d.continuous, 96256);
        assert!((d.music - (2. + 256. / 24000.)).abs() < 1e-12);
        assert_eq!(d.state & 0x800, 0);
        assert_eq!(d.encode().len(), 96);
        assert_eq!(Context::default().chunk(256).unwrap().present, 0);
        assert!(Context {
            rate: f64::NAN,
            ..c
        }
        .chunk(0)
        .is_none());
        assert!(Context {
            project: i64::MAX,
            ..c
        }
        .chunk(1)
        .is_none());
        let stopped = Context {
            state: c.state & !2,
            ..c
        }
        .chunk(128)
        .unwrap();
        assert_eq!(stopped.project, c.project);
        assert_eq!(stopped.music, c.music);
    }
}

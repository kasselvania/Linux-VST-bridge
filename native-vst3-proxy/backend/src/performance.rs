//! Inactive-only settings and versioned setup bytes. No callback file reads.
use ap1_native_client::{get, invalid, need};
use std::{
    io,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};
pub fn wire(max: u32, mode: u32, rate: f64) -> io::Result<Vec<u8>> {
    need((1..=1024).contains(&max), "host maximum outside 1..1024")?;
    let mut bytes = Vec::with_capacity(24);
    bytes.extend_from_slice(&max.min(256).to_le_bytes());
    bytes.extend_from_slice(&mode.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&[0; 8]);
    validate_wire(&bytes)?;
    Ok(bytes)
}
pub fn validate_wire(b: &[u8]) -> io::Result<()> {
    need(b.len() == 24, "setup extent")?;
    let rate = f64::from_le_bytes(b[8..16].try_into().unwrap());
    need(
        (1..=256).contains(&get(&b[..4]))
            && matches!(get(&b[4..8]), 0 | 2)
            && [44100., 48000., 88200., 96000., 192000.].contains(&rate)
            && get(&b[16..24]) == 0,
        "unsupported setup",
    )
}
pub fn selected_delay(max: u32) -> io::Result<u32> {
    need((1..=1024).contains(&max), "host maximum outside 1..1024")?;
    let path = PathBuf::from(std::env::var_os("HOME").ok_or_else(|| invalid("home absent"))?)
        .join("AP9-Performance/delay-frames");
    read_delay(&path, max)
}
fn read_delay(path: &std::path::Path, max: u32) -> io::Result<u32> {
    need((1..=1024).contains(&max), "host maximum outside 1..1024")?;
    #[cfg(target_os = "linux")]
    const NOFOLLOW: i32 = 0x20000;
    #[cfg(target_os = "macos")]
    const NOFOLLOW: i32 = 0x100;
    let delay = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(NOFOLLOW)
        .open(path)
    {
        Err(e) if e.kind() == io::ErrorKind::NotFound => 512.max(max.next_power_of_two()),
        Err(e) => return Err(e),
        Ok(f) => {
            let m = f.metadata()?;
            unsafe extern "C" {
                fn getuid() -> u32;
            }
            need(
                m.is_file()
                    && m.uid() == unsafe { getuid() }
                    && m.mode() & 0o077 == 0
                    && m.len() <= 16,
                "delay setting must be a private small file",
            )?;
            let mut text = String::new();
            f.take(17).read_to_string(&mut text)?;
            text.trim()
                .parse::<u32>()
                .map_err(|_| invalid("invalid delay frames"))?
        }
    };
    need(
        [64, 128, 256, 512, 1024, 2048].contains(&delay)
            && (1..=1024).contains(&max)
            && delay >= max,
        "delay must cover at least one host block",
    )?;
    Ok(delay)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_settings_default_floor_and_invalid_values() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "ap9-setting-{:032x}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        std::fs::create_dir(&dir).unwrap();
        let file = dir.join("delay");
        assert_eq!(read_delay(&file, 128).unwrap(), 512);
        assert_eq!(read_delay(&file, 1024).unwrap(), 1024);
        assert!(read_delay(&file, 1025).is_err());
        std::fs::write(&file, "128\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(read_delay(&file, 128).unwrap(), 128);
        assert!(read_delay(&file, 256).is_err());
        for value in ["0", "127", "128.0", "4096", "01234567890123456789"] {
            std::fs::write(&file, value).unwrap();
            assert!(read_delay(&file, 64).is_err());
        }
        std::fs::write(&file, "128").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_delay(&file, 64).is_err());
        let link = dir.join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(read_delay(&link, 64).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn setup_bounds_and_no_precision_conversion() {
        for r in [44100., 48000., 88200., 96000., 192000.] {
            assert!(wire(1024, 0, r).is_ok());
        }
        assert!(wire(1025, 0, 48000.).is_err());
        assert!(wire(64, 0, 47999.).is_err());
        assert!(wire(64, 1, 48000.).is_err());
        let mut b = wire(64, 0, 48000.).unwrap();
        b[16] = 1;
        assert!(validate_wire(&b).is_err());
    }
}

// Fixed logarithmic histograms (eight buckets per octave) on the optional
// observer thread. Quantiles are bucket upper bounds; maxima remain exact.
#[derive(Clone)]
struct Distribution {
    bins: [u64; 192],
    count: u64,
    max: u64,
    sum: u128,
}
impl Default for Distribution {
    fn default() -> Self {
        Self {
            bins: [0; 192],
            count: 0,
            max: 0,
            sum: 0,
        }
    }
}
impl Distribution {
    fn upper(bin: usize) -> u64 {
        let exponent = bin / 8;
        let fraction = bin % 8;
        ((9 + fraction) as u64 * (1u64 << exponent)) * 1000 / 8
    }
    fn add(&mut self, ns: u64) {
        let bin = (0..191).find(|b| ns <= Self::upper(*b)).unwrap_or(191);
        self.bins[bin] += 1;
        self.count += 1;
        self.max = self.max.max(ns);
        self.sum += ns as u128;
    }
    fn quantile(&self, p: u64) -> u64 {
        let target = (self.count * p).div_ceil(100);
        let mut count = 0;
        for (i, n) in self.bins.iter().enumerate() {
            count += n;
            if count >= target {
                return Self::upper(i);
            }
        }
        self.max
    }
    fn json(&self, name: &str) -> String {
        format!("{{\"event\":\"ap9_timing\",\"stage\":\"{name}\",\"requests\":{},\"mean_ns\":{},\"p50_upper_ns\":{},\"p95_upper_ns\":{},\"p99_upper_ns\":{},\"max_ns\":{}}}\n",self.count,self.sum/self.count.max(1)as u128,self.quantile(50),self.quantile(95),self.quantile(99),self.max)
    }
}
#[derive(Default, Clone)]
pub struct Timings {
    stages: [Distribution; 9],
    slow: Vec<(u64, u64, u64, u64, u64)>,
    negative_residuals: u64,
    unpublished: u64,
}
impl Timings {
    pub fn add(&mut self, t: crate::observer::Trace) {
        fn elapsed(a: Option<std::time::Instant>, b: Option<std::time::Instant>) -> Option<u64> {
            Some(
                b?.checked_duration_since(a?)?
                    .as_nanos()
                    .min(u64::MAX as u128) as u64,
            )
        }
        let Some(win) = t.process_ns else {
            return;
        };
        let durations = [
            elapsed(t.queued, t.started),
            elapsed(t.started, t.prepared),
            elapsed(t.prepared, t.sent),
            elapsed(t.sent, t.replied),
            elapsed(t.replied, t.validated),
            elapsed(t.validated, t.published),
            elapsed(t.queued, t.published),
            Some(win),
            elapsed(t.queued, t.published).and_then(|n| n.checked_sub(win)),
        ];
        if durations[6].is_none() {
            self.unpublished += 1;
        } else if durations[8].is_none() {
            self.negative_residuals += 1;
        }
        for (s, n) in self.stages.iter_mut().zip(durations) {
            if let Some(n) = n {
                s.add(n);
            }
        }
        if let Some(total) = durations[6] {
            self.slow
                .push((total, t.epoch, t.sequence, t.position, win));
            self.slow.sort_unstable_by_key(|b| std::cmp::Reverse(b.0));
            self.slow.truncate(8);
        }
    }
    pub fn json(&self) -> String {
        if self.stages[6].count == 0 {
            return String::new();
        }
        let mut text = String::new();
        for (d, n) in self.stages.iter().zip([
            "queue",
            "prepare",
            "send",
            "reply",
            "validation",
            "publication",
            "admission_to_publication",
            "windows_process",
            "correlated_non_plugin",
        ]) {
            text.push_str(&d.json(n));
        }
        text.push_str(&format!(
            "{{\"event\":\"ap9_clock_resolution\",\"negative_correlated_residuals\":{}}}\n",
            self.negative_residuals
        ));
        text.push_str(&format!(
            "{{\"event\":\"ap9_unpublished\",\"requests\":{}}}\n",
            self.unpublished
        ));
        for (total, epoch, seq, pos, win) in &self.slow {
            text.push_str(&format!("{{\"event\":\"ap9_slow_request\",\"epoch\":{epoch},\"sequence\":{seq},\"position\":{pos},\"service_ns\":{total},\"windows_process_ns\":{win}}}\n"));
        }
        text
    }
}

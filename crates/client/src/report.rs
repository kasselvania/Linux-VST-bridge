//! Presentation of the same bounded JSON bytes used by the report saver.
use crate::SupportReport;

pub const REPORT_LIMIT: usize = 8 * 1024 * 1024;

/// Format only the validated owner value. Bound output while serializing so
/// indentation cannot expand a legal input into an unbounded allocation.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn encode(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    struct Bounded(Vec<u8>);
    impl std::io::Write for Bounded {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > REPORT_LIMIT.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("report size limit"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    let mut output = Bounded(Vec::new());
    if serde_json::to_writer_pretty(&mut output, value).is_err() {
        // A valid compact owner report must remain savable when whitespace
        // alone exceeds the bound. Preview uses these same fallback bytes.
        output.0.clear();
        serde_json::to_writer(&mut output, value)
            .map_err(|_| "Problem report is too large to read.".to_owned())?;
    }
    Ok(output.0)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contents {
    pub report: SupportReport,
    /// These are exactly the bytes the save path writes, decoded as UTF-8.
    pub json: String,
}

impl Contents {
    #[cfg(any(target_os = "linux", test))]
    pub(crate) fn validated(report: &SupportReport, bytes: Vec<u8>) -> Result<Self, String> {
        if bytes.len() > REPORT_LIMIT {
            return Err("Problem report is too large to read.".into());
        }
        let json = String::from_utf8(bytes).map_err(|_| "Problem report could not be read.")?;
        Ok(Self { report: report.clone(), json })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contents_preserve_the_saved_bytes_and_bound_the_payload() {
        let report = SupportReport { id: "ab".repeat(16), sha256: "cd".repeat(32) };
        let bytes = "{\"schema\":2,\"text\":\"Échos 🎹\"}".as_bytes().to_vec();
        let contents = Contents::validated(&report, bytes.clone()).unwrap();
        assert_eq!(contents.report, report);
        assert_eq!(contents.json.as_bytes(), bytes);
        assert!(Contents::validated(&report, vec![b' '; REPORT_LIMIT + 1]).is_err());
        assert!(Contents::validated(&report, vec![0xff]).is_err());
    }
    #[test]
    fn shared_formatting_is_readable_lossless_and_bounded_during_expansion() {
        let value = serde_json::json!({"schema":2,"records":[{"hash":"ab".repeat(32)}]});
        let bytes = encode(&value).unwrap();
        assert!(bytes.contains(&b'\n'));
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(), value);
        assert!(encode(&serde_json::json!({"large":"x".repeat(REPORT_LIMIT)})).is_err());
        let mut nested = serde_json::Value::Array(vec![serde_json::Value::Null; 60_000]);
        for _ in 0..80 { nested = serde_json::Value::Array(vec![nested]); }
        let compact = serde_json::to_vec(&nested).unwrap();
        assert!(compact.len() < REPORT_LIMIT);
        assert_eq!(encode(&nested).unwrap(), compact, "formatting cannot make a valid report unsavable");
    }
}

//! Bridge package versions: ordering, and the version a generation records.
//! Display and comparison only; nothing here selects or certifies a package.
use serde::Deserialize;
use std::{cmp::Ordering, path::Path};

/// Dotted versions with numeric parts compared as numbers:
/// `0.12.0test99 < 0.12.0test100 < 0.12.1`.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    #[derive(PartialEq, Eq)]
    enum Part<'a> { Number(&'a str), Word(&'a str) }
    fn parts(version: &str) -> Vec<Part<'_>> {
        let mut out = Vec::new();
        let bytes = version.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'.' { i += 1; continue; }
            let digit = bytes[i].is_ascii_digit();
            let start = i;
            while i < bytes.len() && bytes[i] != b'.' && bytes[i].is_ascii_digit() == digit { i += 1; }
            let text = &version[start..i];
            out.push(if digit {
                let trimmed = text.trim_start_matches('0');
                Part::Number(if trimmed.is_empty() { "0" } else { trimmed })
            } else { Part::Word(text) });
        }
        out
    }
    let (a, b) = (parts(a), parts(b));
    for (x, y) in a.iter().zip(&b) {
        let order = match (x, y) {
            (Part::Number(x), Part::Number(y)) => x.len().cmp(&y.len()).then_with(|| x.cmp(y)),
            (Part::Word(x), Part::Word(y)) => x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase()),
            (Part::Number(_), Part::Word(_)) => Ordering::Greater,
            (Part::Word(_), Part::Number(_)) => Ordering::Less,
        };
        if order != Ordering::Equal { return order; }
    }
    a.len().cmp(&b.len())
}

/// A package version as packages carry it: a digit, then letters, digits and dots.
pub fn valid_version(value: &str) -> bool {
    !value.is_empty() && value.len() <= 80 && value.as_bytes()[0].is_ascii_digit()
        && value.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.')
}

/// The version a package generation directory records for itself, for a
/// window that must name its own build before the manager answers.
pub fn generation_version(dir: &Path) -> Option<String> {
    #[derive(Deserialize)]
    struct Manifest { version: String }
    #[derive(Deserialize)]
    struct Record { manifest: Manifest }
    // Bounded, regular, own file only: this runs on the UI thread at start.
    let record: Record = crate::read_json(&dir.join("package-generation.json")).ok()?;
    valid_version(&record.manifest.version).then_some(record.manifest.version)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_order_their_numbers_as_numbers() {
        assert_eq!(compare_versions("0.12.0test148", "0.12.0test146"), Ordering::Greater);
        assert_eq!(compare_versions("0.12.0test99", "0.12.0test100"), Ordering::Less);
        assert_eq!(compare_versions("0.12.0test144", "0.12.1"), Ordering::Less);
        assert_eq!(compare_versions("0.12.0test144", "0.12.0test144"), Ordering::Equal);
        assert!(valid_version("0.12.0test144"));
        assert!(!valid_version("test144") && !valid_version("0.12/../x") && !valid_version(""));
    }
    #[test]
    fn generation_version_reads_only_a_well_formed_version() {
        let dir = std::env::temp_dir().join(format!("lvb-generation-version-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(generation_version(&dir), None);
        std::fs::write(dir.join("package-generation.json"),
            br#"{"schema":2,"manifest":{"version":"0.12.0test144","files":[]}}"#).unwrap();
        assert_eq!(generation_version(&dir).as_deref(), Some("0.12.0test144"));
        std::fs::write(dir.join("package-generation.json"),
            br#"{"manifest":{"version":"/home/someone"}}"#).unwrap();
        assert_eq!(generation_version(&dir), None);
        // A FIFO or an oversized record is never read.
        std::fs::remove_file(dir.join("package-generation.json")).unwrap();
        let fifo = std::ffi::CString::new(dir.join("package-generation.json").to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert_eq!(generation_version(&dir), None);
        std::fs::remove_file(dir.join("package-generation.json")).unwrap();
        std::fs::write(dir.join("package-generation.json"), vec![b' '; 9 * 1024 * 1024]).unwrap();
        assert_eq!(generation_version(&dir), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

//! The return's code must not reach any file, journal or output. Every code
//! the test sends is a fresh fake; the search reports where, never what.
use std::{
    collections::HashSet,
    fs,
    io::Read,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
    process::Command,
};

/// Every fake code is this marker and a fresh random part. A URL carries its
/// code, so finding either the marker or any random part finds every copy,
/// including one with the marker cut off, in ASCII or UTF-16LE (Windows'
/// own text and registry encoding).
const MARKER: &[u8] = b"lvbfakecode";
const RANDOM: usize = 32;
/// The codes sent, and a control value the test itself writes to files and
/// the journal so that the search shows it can find what it looks for.
pub struct Secrets { codes: HashSet<Vec<u8>>, control: Vec<u8> }
impl Default for Secrets {
    fn default() -> Self {
        Self { codes: HashSet::new(), control: linux_vst_bridge::random_id().unwrap().into_bytes() }
    }
}

/// What one search pass found: any code, and the control in each encoding.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Matched { code: bool, control: bool, wide_control: bool }
impl Matched {
    fn or(self, other: Self) -> Self {
        Self { code: self.code || other.code, control: self.control || other.control,
            wide_control: self.wide_control || other.wide_control }
    }
    fn all(self) -> bool { self.code && self.control && self.wide_control }
}

impl Secrets {
    pub fn control(&self) -> &str { std::str::from_utf8(&self.control).unwrap() }
    /// The control as Windows writes text: UTF-16LE.
    pub fn wide_control(&self) -> Vec<u8> { self.control.iter().flat_map(|byte| [*byte, 0]).collect() }
    /// A sign-in return in Native Access's shape with a fresh fake code.
    pub fn url(&mut self, shape: impl FnOnce(&str) -> String) -> String {
        let random = linux_vst_bridge::random_id().unwrap();
        assert_eq!(random.len(), RANDOM);
        let url = shape(&format!("{}{random}", std::str::from_utf8(MARKER).unwrap()));
        self.codes.insert(random.into_bytes());
        url
    }
    pub fn found_in(&self, bytes: &[u8]) -> bool { self.matches(bytes).code }
    /// One pass: the marker, and every run of lowercase hex long enough to
    /// hold a random part, each in ASCII and UTF-16LE.
    fn matches(&self, bytes: &[u8]) -> Matched {
        let wide: Vec<u8> = MARKER.iter().flat_map(|byte| [*byte, 0]).collect();
        let mut matched = Matched { code: bytes.windows(MARKER.len()).any(|window| window == MARKER)
            || bytes.windows(wide.len()).any(|window| window == wide.as_slice()), ..Matched::default() };
        let hex = |byte: u8| matches!(byte, b'0'..=b'9' | b'a'..=b'f');
        let (mut run, mut wide_runs) = (0, [0; 2]);
        let mut narrowed = [0u8; RANDOM];
        for index in 0..bytes.len() {
            if hex(bytes[index]) {
                run += 1;
                if run >= RANDOM {
                    let window = &bytes[index + 1 - RANDOM..=index];
                    matched.code |= self.codes.contains(window);
                    matched.control |= window == self.control.as_slice();
                }
            } else { run = 0; }
            // A UTF-16LE character starting here: a hex byte, then zero.
            let parity = index % 2;
            if index + 1 < bytes.len() && hex(bytes[index]) && bytes[index + 1] == 0 {
                wide_runs[parity] += 1;
                if wide_runs[parity] >= RANDOM {
                    let first = index + 2 - 2 * RANDOM;
                    for (slot, byte) in narrowed.iter_mut().zip(bytes[first..=index].iter().step_by(2)) { *slot = *byte; }
                    matched.code |= self.codes.contains(narrowed.as_slice());
                    matched.wide_control |= narrowed == self.control.as_slice();
                }
            } else { wide_runs[parity] = 0; }
            if matched.all() { break; }
        }
        matched
    }
}

pub struct Search {
    pub files: usize,
    pub bytes: u64,
    pub unreadable: usize,
    /// Listed, then gone before it was read: a live session's atomic record
    /// writes rename their temporary file into place.
    pub vanished: usize,
    pub found: Vec<String>,
    /// Where the control was found in ASCII, and where in UTF-16LE.
    pub controls: Vec<String>,
    pub wide_controls: Vec<String>,
    /// Anything that could not be read inside `strict`: there, nothing may be.
    pub unreadable_strict: Vec<String>,
}

/// Search every regular file under `roots`, except below `skip`, and the
/// account's and the system's journals. While a session is `live`, a file
/// under `strict` may be renamed away between listing and reading; once
/// nothing runs, a missing file there is as unreadable as any other.
pub fn search(secrets: &Secrets, roots: &[PathBuf], skip: &[PathBuf], strict: &Path, live: bool) -> Search {
    let mut search = Search { files: 0, bytes: 0, unreadable: 0, vanished: 0, found: vec![], controls: vec![],
        wide_controls: vec![], unreadable_strict: vec![] };
    let longest = 2 * RANDOM.max(MARKER.len());
    let unreadable = |search: &mut Search, path: &Path, error: std::io::Error| {
        let bridge = path.starts_with(strict);
        if error.kind() == std::io::ErrorKind::NotFound && (live || !bridge) { search.vanished += 1; return; }
        search.unreadable += 1;
        if bridge { search.unreadable_strict.push(path.display().to_string()); }
    };
    let mut pending: Vec<PathBuf> = roots.iter().filter(|root| root.exists()).cloned().collect();
    while let Some(path) = pending.pop() {
        if skip.iter().any(|skipped| path.starts_with(skipped)) { continue; }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => { unreadable(&mut search, &path, error); continue }
        };
        let kind = metadata.file_type();
        if kind.is_dir() {
            match fs::read_dir(&path) {
                Ok(entries) => pending.extend(entries.flatten().map(|entry| entry.path())),
                Err(error) => unreadable(&mut search, &path, error),
            }
        } else if kind.is_file() && !kind.is_fifo() {
            match file_contains(&path, secrets, longest) {
                Ok((matched, bytes)) => {
                    search.files += 1;
                    search.bytes += bytes;
                    search.record(matched, &path.display().to_string());
                }
                Err(error) => unreadable(&mut search, &path, error),
            }
        }
    }
    for (journal, args) in [("user journal", &["--user", "-o", "export", "--no-pager"][..]),
        ("system journal", &["-o", "export", "--no-pager"][..])] {
        let output = Command::new("journalctl").args(args).output();
        match output {
            Ok(output) if output.status.success() => {
                search.bytes += output.stdout.len() as u64;
                search.record(secrets.matches(&output.stdout), journal);
            }
            _ => search.unreadable += 1,
        }
    }
    search
}
impl Search {
    fn record(&mut self, matched: Matched, place: &str) {
        if matched.code { self.found.push(place.into()); }
        if matched.control { self.controls.push(place.into()); }
        if matched.wide_control { self.wide_controls.push(place.into()); }
    }
}

fn file_contains(path: &Path, secrets: &Secrets, longest: usize) -> std::io::Result<(Matched, u64)> {
    let mut file = fs::File::open(path)?;
    let mut carried: Vec<u8> = vec![];
    let mut buffer = vec![0; 8 * 1024 * 1024];
    let (mut total, mut seen) = (0, Matched::default());
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { return Ok((seen, total)); }
        total += n as u64;
        carried.extend_from_slice(&buffer[..n]);
        seen = seen.or(secrets.matches(&carried));
        if seen.all() { return Ok((seen, total)); }
        let keep = carried.len().saturating_sub(longest);
        carried.drain(..keep);
    }
}

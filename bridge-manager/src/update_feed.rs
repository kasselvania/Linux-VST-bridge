//! Signed beta update feed: find, download and stage a newer bridge package.
//!
//! Trust: the feed and the package are both verified with the release key
//! compiled into this application. The feed has its own signing domain, so a
//! package signature can never stand in for it and the reverse. A downloaded
//! installer is never executed: once its SHA-256 and its own signature verify,
//! it is staged exactly as the installer's "Install application files" stages
//! it, and the existing Update Bridge step applies it.
//!
//! Privacy: the tester's beta key is kept in a private file and sent only as a
//! `PW:` request header, written to curl's standard input. It never appears in
//! a URL, a command line, a log line, an error or a support export.
use crate::portable_package::{Bundle, Trust};
use crate::{atomic_json, digest, random_id, require, Result};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt, fs,
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const FEED_DOMAIN: &[u8] = b"Linux VST Bridge update feed v1\0";
pub const FEED_FILE: &str = "latest.json";
pub const SIGNATURE_FILE: &str = "latest.json.sig";
pub const CHANNEL: &str = "beta";
const LINK_FILE: &str = "beta-link.json";
const FEED_MAX: u64 = 64 * 1024;
const NOTES_MAX: usize = 16 * 1024;
/// The installer's own bound: payload plus launcher and manifest.
const PACKAGE_MAX: u64 = 2_000_000_000 + 256 * 1024 * 1024;
const LINK_MAX: usize = 2048;
const NOT_A_LINK: &str = "That doesn't look like a beta link. Paste the whole link you were sent; it starts with https://.";

/// What went wrong, in words a musician can act on. None of these carry the
/// beta key, a URL or a server response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateError {
    /// Offline, the server is down, a timeout or a server error.
    Offline,
    /// The server refused the beta key (401/403).
    LinkRejected,
    /// The beta folder has no update feed.
    NoFeed,
    /// The feed's signature or format did not verify.
    FeedUnverified,
    /// The feed names an installer the beta folder does not have.
    PackageMissing,
    /// The download stopped part-way; the next attempt resumes it.
    DownloadStopped,
    /// The download's size or SHA-256 differs from the signed feed.
    Damaged,
    /// The downloaded installer's own signature or version did not verify.
    PackageUnverified,
    /// The package is not newer than the running version.
    Downgrade,
    /// This system lacks the curl command updates are fetched with.
    NoCurl,
    /// A local step (disk, staging) refused; the text is the manager's.
    Local(String),
}
impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => f.write_str("Couldn't check for updates right now."),
            Self::LinkRejected => f.write_str("Your beta link no longer works; paste the new link."),
            Self::NoFeed => f.write_str("No update has been published at your beta link yet."),
            Self::FeedUnverified => f.write_str("The update information could not be verified, so nothing was downloaded. Try again later; if this stays, tell the maintainer."),
            Self::PackageMissing => f.write_str("The update file is missing from the beta folder. Try again later; if this stays, tell the maintainer."),
            Self::DownloadStopped => f.write_str("The download stopped before it finished. Choose Download and update again to continue where it left off."),
            Self::Damaged => f.write_str("The downloaded update did not match what was published, so it was deleted and nothing was installed. Choose Download and update to try again."),
            Self::PackageUnverified => f.write_str("The downloaded update's signature did not verify, so nothing was installed. Tell the maintainer."),
            Self::Downgrade => f.write_str("This update is not newer than your current version, so it was not installed. To go back to an earlier version, use Restore previous setup."),
            Self::NoCurl => f.write_str("Updates need the curl command, which this system does not have."),
            Self::Local(reason) => {
                if reason.contains(' ') && reason.starts_with(|c: char| c.is_ascii_uppercase()) {
                    write!(f, "The update could not be installed: {reason}{}",
                        if reason.ends_with('.') { "" } else { "." })
                } else {
                    write!(f, "The update could not be installed ({reason}). Your current setup is unchanged.")
                }
            }
        }
    }
}
impl std::error::Error for UpdateError {}
fn local(error: impl fmt::Display) -> UpdateError { UpdateError::Local(error.to_string()) }

/// The tester's beta folder and its key. The key is never shown, logged or
/// placed in a URL; `Debug` hides it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BetaLink {
    schema: u32,
    /// `https://host:port`, lower-case host, explicit port.
    origin: String,
    /// The folder, beginning and ending with `/`.
    path: String,
    key: Option<String>,
}
impl fmt::Debug for BetaLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BetaLink").field("origin", &self.origin).field("path", &self.path)
            .field("key", &self.key.as_ref().map(|_| "[private]")).finish()
    }
}
fn valid_host(host: &str) -> bool {
    if let Some(inner) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        return !inner.is_empty() && inner.len() <= 45
            && inner.bytes().all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
    }
    !host.is_empty() && host.len() <= 253 && host.split('.').all(|label| !label.is_empty()
        && label.len() <= 63 && !label.starts_with('-') && !label.ends_with('-')
        && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}
fn loopback(host: &str) -> bool {
    host == "localhost" || host == "[::1]"
        || host.strip_prefix("127.").is_some_and(|rest| rest.split('.').count() == 3
            && rest.split('.').all(|part| part.parse::<u8>().is_ok()))
}
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else { out.push(bytes[i]); i += 1; }
    }
    String::from_utf8(out).ok()
}
fn valid_key(key: &str) -> bool {
    (1..=256).contains(&key.len()) && key.bytes().all(|b| (0x20..=0x7e).contains(&b))
        && !key.starts_with(' ') && !key.ends_with(' ')
}
fn valid_path(path: &str) -> bool {
    path.starts_with('/') && path.ends_with('/') && path.len() <= 512
        && path.bytes().all(|b| b.is_ascii_alphanumeric()
            || b"-._~%!$&()*+,;=:@/".contains(&b))
        && path.split('/').all(|segment| percent_decode(segment)
            .is_some_and(|s| s != "." && s != ".." && !s.contains(['/', '\\'])))
}
impl BetaLink {
    /// Read a pasted beta link. A link to a file in the folder is accepted as
    /// the folder. Plain `http://` is accepted only for this computer.
    pub fn parse(text: &str) -> std::result::Result<Self, &'static str> {
        let text = text.trim();
        if text.is_empty() || text.len() > LINK_MAX || !text.is_ascii()
            || text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(NOT_A_LINK);
        }
        let lower = text.to_ascii_lowercase();
        let (scheme, rest) = if lower.starts_with("https://") { ("https", &text[8..]) }
            else if lower.starts_with("http://") { ("http", &text[7..]) }
            else { return Err(NOT_A_LINK) };
        let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let (authority, rest) = rest.split_at(end);
        if authority.contains('@') { return Err(NOT_A_LINK); }
        let (host, port) = match authority.rfind(':') {
            Some(i) if !authority[i..].contains(']') => (&authority[..i], Some(&authority[i + 1..])),
            _ => (authority, None),
        };
        let host = host.to_ascii_lowercase();
        if !valid_host(&host) { return Err(NOT_A_LINK); }
        let port = match port {
            Some(text) => match text.parse::<u16>() { Ok(port) if port != 0 => port, _ => return Err(NOT_A_LINK) },
            None => if scheme == "https" { 443 } else { 80 },
        };
        if scheme == "http" && !loopback(&host) {
            return Err("Beta links must start with https://. Paste the link exactly as you were sent it.");
        }
        let (before_fragment, fragment) = rest.split_once('#').unwrap_or((rest, ""));
        let (path, query) = before_fragment.split_once('?').unwrap_or((before_fragment, ""));
        let path = if path.is_empty() { "/" } else { path };
        let folder = &path[..=path.rfind('/').ok_or(NOT_A_LINK)?];
        if !valid_path(folder) { return Err(NOT_A_LINK); }
        let mut key = None;
        for pair in query.split('&').chain(fragment.split('&')) {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            if name.eq_ignore_ascii_case("pw") {
                let value = percent_decode(value).filter(|v| valid_key(v))
                    .ok_or("The key in this beta link can't be used. Paste the link exactly as you were sent it.")?;
                key = Some(value);
            }
        }
        Ok(Self { schema: 1, origin: format!("{scheme}://{host}:{port}"), path: folder.into(), key })
    }
    fn validate(&self) -> Result<()> {
        let reparsed = Self::parse(&format!("{}{}", self.origin, self.path))
            .map_err(|_| "update_link_record")?;
        require(self.schema == 1 && reparsed.origin == self.origin && reparsed.path == self.path
            && self.key.as_deref().is_none_or(valid_key), "update_link_record")
    }
    /// The host name shown to the tester; never the key.
    pub fn site(&self) -> &str {
        let host = self.origin.split_once("://").map(|(_, rest)| rest).unwrap_or(&self.origin);
        host.rsplit_once(':').map(|(host, _)| host).unwrap_or(host)
    }
    pub fn has_key(&self) -> bool { self.key.is_some() }
    fn plain_loopback(&self) -> bool { self.origin.starts_with("http://") }
    /// A file in the beta folder. File names are checked by the feed, so no
    /// encoding is needed, and the key is never part of it.
    fn url(&self, file: &str) -> String { format!("{}{}{}", self.origin, self.path, file) }
}

pub fn updates_dir(home: &Path) -> PathBuf { home.join(".local/share/linux-vst-bridge/updates") }
fn private_directory(path: &Path) -> Result<()> {
    fs::DirBuilder::new().recursive(true).mode(0o700).create(path)?;
    crate::private_dir(path)
}
/// Store the link privately (0600 in a 0700 folder). Replacing it is how a
/// rotated key is taken up.
pub fn save_link(home: &Path, link: &BetaLink) -> Result<()> {
    link.validate()?;
    let directory = updates_dir(home);
    private_directory(&directory)?;
    atomic_json(&directory.join(LINK_FILE), link)
}
pub fn load_link(home: &Path) -> Result<Option<BetaLink>> {
    let path = updates_dir(home).join(LINK_FILE);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
        Ok(metadata) => {
            require(metadata.is_file() && metadata.uid() == unsafe { libc::getuid() }
                && metadata.mode() & 0o077 == 0, "update_link_custody")?;
            let link: BetaLink = crate::read_json(&path)?;
            link.validate()?;
            Ok(Some(link))
        }
    }
}
pub fn forget_link(home: &Path) -> Result<()> {
    match fs::remove_file(updates_dir(home).join(LINK_FILE)) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Feed {
    pub schema: u32,
    pub channel: String,
    pub version: String,
    pub published: String,
    pub min_version: String,
    pub notes: String,
    pub packages: BTreeMap<String, FeedPackage>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FeedPackage { pub file: String, pub size: u64, pub sha256: String }

/// The same version rule as a signed package release.
pub fn valid_version(version: &str) -> bool {
    !version.is_empty() && version.len() <= 80 && version.as_bytes()[0].is_ascii_digit()
        && version.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
}
fn valid_system(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}
fn valid_file(name: &str) -> bool {
    (1..=160).contains(&name.len()) && !name.starts_with(['.', '-']) && name.ends_with(".install")
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// The feed's signature, by the compiled release key, comes before any field
/// of it is used.
pub fn verify_feed(bytes: &[u8], signature: &[u8], trust: &Trust) -> Result<Feed> {
    require(bytes.len() as u64 <= FEED_MAX, "update_feed_extent")?;
    trust.verify_detached(FEED_DOMAIN, bytes, signature)?;
    let feed: Feed = serde_json::from_slice(bytes)?;
    require(feed.schema == 1 && feed.channel == CHANNEL
        && valid_version(&feed.version) && valid_version(&feed.min_version)
        && compare_versions(&feed.min_version, &feed.version) != Ordering::Greater
        && (1..=64).contains(&feed.published.len())
        && feed.published.bytes().all(|b| b.is_ascii_graphic())
        && feed.notes.len() <= NOTES_MAX
        && feed.notes.chars().all(|c| matches!(c, '\n' | '\r' | '\t') || !c.is_control())
        && (1..=16).contains(&feed.packages.len()), "update_feed_format")?;
    for (system, package) in &feed.packages {
        require(valid_system(system) && valid_file(&package.file) && package.size > 0
            && package.size <= PACKAGE_MAX && valid_sha256(&package.sha256), "update_feed_format")?;
    }
    Ok(feed)
}

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
/// `0.12.0test148` is shown as `Test148`, the build number testers know.
pub fn display_version(version: &str) -> String {
    match version.rsplit_once("test") {
        Some((_, number)) if !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()) =>
            format!("Test{number}"),
        _ => version.to_owned(),
    }
}

/// The system's own name first, then the systems it is like, from os-release.
pub fn system_names(os_release: &str) -> Vec<String> {
    let mut names = Vec::new();
    for key in ["ID", "ID_LIKE"] {
        for word in crate::os_release_value(os_release, key).unwrap_or_default().split_whitespace() {
            let word = word.to_ascii_lowercase();
            if valid_system(&word) && !names.contains(&word) { names.push(word); }
        }
    }
    names
}
pub fn read_os_release() -> String {
    for path in ["/etc/os-release", "/usr/lib/os-release"] {
        let mut bytes = Vec::new();
        if fs::File::open(path).and_then(|f| f.take(8193).read_to_end(&mut bytes)).is_ok()
            && bytes.len() <= 8192 {
            if let Ok(text) = String::from_utf8(bytes) { return text; }
        }
    }
    String::new()
}

/// The running application's package version, from its selected generation
/// record. None for the older layout without a package record.
pub fn installed_version(home: &Path) -> Result<Option<String>> {
    let root = home.join(".local/share/linux-vst-bridge/managed");
    let software: crate::catalogue::Software = crate::read_json(&root.join("software.json"))?;
    let directory = software.manager.path.parent().ok_or("update_installed_version")?;
    require(directory.parent() == Some(root.join("software").as_path()), "update_installed_version")?;
    let record = directory.join("package-generation.json");
    match fs::symlink_metadata(&record) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    let value: serde_json::Value = crate::read_json(&record)?;
    let version = value["manifest"]["version"].as_str().filter(|v| valid_version(v))
        .ok_or("update_installed_version")?;
    Ok(Some(version.to_owned()))
}

#[derive(Debug, PartialEq, Eq)]
enum Fetched { Complete, NotFound, Rejected, RangeRefused }
fn quoted(text: &str) -> String { text.replace('\\', "\\\\").replace('"', "\\\"") }
/// Read by curl from standard input: the only place the key is written.
fn curl_config(link: &BetaLink, file: &str) -> String {
    let mut config = format!("url = \"{}\"\n", quoted(&link.url(file)));
    if let Some(key) = &link.key { config.push_str(&format!("header = \"PW: {}\"\n", quoted(key))); }
    config
}
fn curl_command(link: &BetaLink, output: &Path, limit: u64, resume: bool, max_time: Duration) -> Command {
    let mut command = Command::new("curl");
    // `--disable` must come first: no curlrc can add options or redirect output.
    command.arg("--disable")
        .args(["--config", "-", "--silent", "--fail", "--max-redirs", "0", "--proto"])
        .arg(if link.plain_loopback() { "=http" } else { "=https" })
        .args(["--connect-timeout", "15", "--speed-limit", "1024", "--speed-time", "60",
            "--write-out", "%{http_code}", "--max-time"])
        .arg(max_time.as_secs().max(1).to_string())
        .arg("--max-filesize").arg(limit.to_string())
        .arg("--output").arg(output);
    if resume { command.args(["--continue-at", "-"]); }
    command.env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    command
}
/// One bounded GET into `output`. Errors carry no server text.
fn transfer(link: &BetaLink, file: &str, output: &Path, limit: u64, resume: bool,
    max_time: Duration, progress: &mut dyn FnMut(u64)) -> std::result::Result<Fetched, UpdateError> {
    let mut child = curl_command(link, output, limit, resume, max_time).spawn()
        .map_err(|_| UpdateError::NoCurl)?;
    let written = child.stdin.take().map(|mut input| input.write_all(curl_config(link, file).as_bytes()));
    if !matches!(written, Some(Ok(()))) {
        let _ = child.kill(); let _ = child.wait();
        return Err(UpdateError::Offline);
    }
    let deadline = Instant::now() + max_time + Duration::from_secs(20);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(_) => { let _ = child.kill(); let _ = child.wait(); return Err(UpdateError::Offline); }
        }
        let size = fs::metadata(output).map(|m| m.len()).unwrap_or(0);
        if size > limit {
            let _ = child.kill(); let _ = child.wait();
            return Err(UpdateError::Damaged);
        }
        progress(size);
        if Instant::now() >= deadline {
            let _ = child.kill(); let _ = child.wait();
            return Err(UpdateError::Offline);
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut code = String::new();
    if let Some(stdout) = child.stdout.take() { let _ = stdout.take(8).read_to_string(&mut code); }
    let code = code.trim().parse::<u16>().unwrap_or(0);
    match (status.code(), code) {
        (Some(0), 200 | 206) => Ok(Fetched::Complete),
        (Some(0 | 22), 401 | 403) => Ok(Fetched::Rejected),
        (Some(0 | 22), 404 | 410) => Ok(Fetched::NotFound),
        (Some(0 | 22), 416) | (Some(33), _) => Ok(Fetched::RangeRefused),
        (Some(63), _) => Err(UpdateError::Damaged),
        _ => Err(UpdateError::Offline),
    }
}
fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let file = crate::file(path)?;
    require(file.metadata()?.len() <= limit, "update_feed_extent")?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    require(bytes.len() as u64 <= limit, "update_feed_extent")?;
    Ok(bytes)
}
fn fetch_small(link: &BetaLink, file: &str, scratch: &Path, limit: u64)
    -> std::result::Result<Vec<u8>, UpdateError> {
    let path = scratch.join(format!("{}.download", random_id().map_err(local)?));
    let fetched = transfer(link, file, &path, limit, false, Duration::from_secs(30), &mut |_| {});
    let bytes = match fetched {
        Ok(Fetched::Complete) => read_bounded(&path, limit).map_err(|_| UpdateError::FeedUnverified),
        Ok(Fetched::Rejected) => Err(UpdateError::LinkRejected),
        Ok(Fetched::NotFound) => Err(UpdateError::NoFeed),
        Ok(Fetched::RangeRefused) | Err(UpdateError::Damaged) => Err(UpdateError::FeedUnverified),
        Err(error) => Err(error),
    };
    let _ = fs::remove_file(&path);
    bytes
}
/// Fetch and verify the feed. A feed and signature replaced between the two
/// requests are retried once.
pub fn fetch_feed(home: &Path, link: &BetaLink, trust: &Trust) -> std::result::Result<Feed, UpdateError> {
    let scratch = updates_dir(home).join("check");
    private_directory(&scratch).map_err(local)?;
    for attempt in 0..2 {
        let bytes = fetch_small(link, FEED_FILE, &scratch, FEED_MAX)?;
        let signature = fetch_small(link, SIGNATURE_FILE, &scratch, 64)
            .map_err(|e| if e == UpdateError::NoFeed { UpdateError::FeedUnverified } else { e })?;
        match verify_feed(&bytes, &signature, trust) {
            Ok(feed) => return Ok(feed),
            Err(_) if attempt == 0 => std::thread::sleep(Duration::from_secs(2)),
            Err(_) => {}
        }
    }
    Err(UpdateError::FeedUnverified)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub version: String,
    pub notes: String,
    /// The feed entry chosen for this system.
    pub system: String,
    pub package: FeedPackage,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    Available(Offer),
    /// The published version is not newer. Never offers a downgrade.
    UpToDate { latest: String },
    /// No package in the feed for this system.
    NoPackage { latest: String, systems: Vec<String> },
    /// The published build cannot be applied from this version in the app.
    ManualUpdate { latest: String },
}
/// The decision after a verified feed: newer, applicable, for this system.
pub fn decide(feed: Feed, current: &str, systems: &[String]) -> Check {
    if compare_versions(&feed.version, current) != Ordering::Greater {
        return Check::UpToDate { latest: feed.version };
    }
    if compare_versions(current, &feed.min_version) == Ordering::Less {
        return Check::ManualUpdate { latest: feed.version };
    }
    let Some((system, package)) = systems.iter()
        .find_map(|name| feed.packages.get(name).map(|package| (name.clone(), package.clone()))) else {
        return Check::NoPackage { latest: feed.version, systems: systems.to_vec() };
    };
    Check::Available(Offer { version: feed.version, notes: feed.notes, system, package })
}
pub fn check(home: &Path, link: &BetaLink, trust: &Trust, current: &str, systems: &[String])
    -> std::result::Result<Check, UpdateError> {
    Ok(decide(fetch_feed(home, link, trust)?, current, systems))
}

/// Download the offered installer (resuming a partial download), verify its
/// size and SHA-256 against the signed feed and its own signature exactly as
/// the installer does, refuse anything not newer, then stage it and point the
/// Setup entry at it. Returns the staged Setup frontend; nothing is executed.
pub fn download_and_stage(home: &Path, link: &BetaLink, trust: &Trust, current: &str,
    offer: &Offer, progress: &mut dyn FnMut(u64, u64)) -> std::result::Result<PathBuf, UpdateError> {
    download_and_stage_with(home, link, trust, current, offer, progress,
        crate::portable_package::expose_setup)
}
fn download_time(size: u64) -> Duration {
    Duration::from_secs((300 + size / 50_000).min(4 * 3600))
}
fn download_and_stage_with(home: &Path, link: &BetaLink, trust: &Trust, current: &str,
    offer: &Offer, progress: &mut dyn FnMut(u64, u64),
    expose: impl FnOnce(&Path, &Path) -> Result<()>) -> std::result::Result<PathBuf, UpdateError> {
    let package = &offer.package;
    if !valid_file(&package.file) || !valid_sha256(&package.sha256) || package.size == 0
        || package.size > PACKAGE_MAX || !valid_version(&offer.version) {
        return Err(UpdateError::FeedUnverified);
    }
    if compare_versions(&offer.version, current) != Ordering::Greater {
        return Err(UpdateError::Downgrade);
    }
    let downloads = updates_dir(home).join("downloads");
    private_directory(&downloads).map_err(local)?;
    let part = downloads.join(format!("{}.part", package.sha256));
    let done = downloads.join(format!("{}.install", package.sha256));
    // Only the offered download is kept; an older partial one is removed.
    for entry in fs::read_dir(&downloads).map_err(local)? {
        let path = entry.map_err(local)?.path();
        if path != part && path != done { let _ = fs::remove_file(&path); }
    }
    let size_of = |path: &Path| fs::symlink_metadata(path).map(|m| m.len()).ok();
    if size_of(&done).is_none() {
        let mut restarted = false;
        loop {
            let have = size_of(&part).unwrap_or(0);
            if have > package.size {
                let _ = fs::remove_file(&part);
                return Err(UpdateError::Damaged);
            }
            if have == package.size { break; }
            let fetched = transfer(link, &package.file, &part, package.size, have > 0,
                download_time(package.size), &mut |n| progress(n, package.size));
            if size_of(&part) == Some(0) { let _ = fs::remove_file(&part); }
            match fetched {
                Ok(Fetched::Complete) => break,
                Ok(Fetched::Rejected) => return Err(UpdateError::LinkRejected),
                Ok(Fetched::NotFound) => return Err(UpdateError::PackageMissing),
                Ok(Fetched::RangeRefused) if !restarted => {
                    let _ = fs::remove_file(&part);
                    restarted = true;
                }
                Ok(Fetched::RangeRefused) => return Err(UpdateError::DownloadStopped),
                Err(UpdateError::Offline) => return Err(UpdateError::DownloadStopped),
                Err(error) => {
                    if error == UpdateError::Damaged { let _ = fs::remove_file(&part); }
                    return Err(error);
                }
            }
        }
        let have = size_of(&part).unwrap_or(0);
        if have < package.size { return Err(UpdateError::DownloadStopped); }
        if have != package.size || digest(&part).ok().as_deref() != Some(package.sha256.as_str()) {
            let _ = fs::remove_file(&part);
            return Err(UpdateError::Damaged);
        }
        fs::set_permissions(&part, fs::Permissions::from_mode(0o400)).map_err(local)?;
        fs::rename(&part, &done).map_err(local)?;
    } else if digest(&done).ok().as_deref() != Some(package.sha256.as_str()) {
        let _ = fs::remove_file(&done);
        return Err(UpdateError::Damaged);
    }
    progress(package.size, package.size);
    let refused = |error: UpdateError| { let _ = fs::remove_file(&done); error };
    let mut bundle = Bundle::open(&done, trust.clone()).map_err(|_| refused(UpdateError::PackageUnverified))?;
    if bundle.release().version != offer.version {
        return Err(refused(UpdateError::PackageUnverified));
    }
    if compare_versions(&bundle.release().version, current) != Ordering::Greater {
        return Err(refused(UpdateError::Downgrade));
    }
    let frontend = bundle.stage(home).map_err(local)?;
    expose(&frontend, home).map_err(local)?;
    let _ = fs::remove_file(&done);
    Ok(frontend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable_package::tests::Fixture;
    use ed25519_dalek::Signer;
    use std::{io::{BufRead, BufReader}, net::{TcpListener, TcpStream},
        sync::{Arc, Mutex}};

    const KEY: &str = "s3cret+Key/42";

    fn curl_available() -> bool {
        Command::new("curl").arg("--version").stdout(Stdio::null()).stderr(Stdio::null())
            .status().is_ok_and(|s| s.success())
    }

    #[derive(Default)]
    struct Shared {
        files: BTreeMap<String, Vec<u8>>,
        key: Option<String>,
        /// Send only this many body bytes of the next response, then close.
        cut: Option<usize>,
        log: Vec<String>,
    }
    /// A small HTTP/1.1 file server for the beta folder, on loopback only.
    struct Server { port: u16, shared: Arc<Mutex<Shared>> }
    impl Server {
        fn start(key: Option<&str>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let shared = Arc::new(Mutex::new(Shared { key: key.map(Into::into), ..Default::default() }));
            let state = shared.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let _ = Self::serve(stream, &state);
                }
            });
            Self { port, shared }
        }
        fn put(&self, name: &str, bytes: Vec<u8>) {
            self.shared.lock().unwrap().files.insert(name.into(), bytes);
        }
        fn link(&self) -> BetaLink {
            BetaLink::parse(&format!("http://127.0.0.1:{}/private-beta/?pw={}", self.port,
                KEY.replace('+', "%2B").replace('/', "%2F"))).unwrap()
        }
        fn log(&self) -> Vec<String> { self.shared.lock().unwrap().log.clone() }
        fn serve(stream: TcpStream, state: &Arc<Mutex<Shared>>) -> std::io::Result<()> {
            stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            let mut reader = BufReader::new(stream.try_clone()?);
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line)? == 0 { break; }
                let line = line.trim_end().to_owned();
                if line.is_empty() { break; }
                head.push(line);
            }
            let mut shared = state.lock().unwrap();
            shared.log.push(head.join("\n"));
            let header = |name: &str| head.iter().skip(1).find_map(|line| line.split_once(':')
                .filter(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_owned()));
            let target = head.first().and_then(|l| l.split(' ').nth(1)).unwrap_or("").to_owned();
            let mut out = stream;
            if shared.key.is_some() && header("PW") != shared.key {
                return out.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            }
            let Some(data) = target.strip_prefix("/private-beta/").and_then(|n| shared.files.get(n)).cloned() else {
                return out.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            };
            let start = header("Range").and_then(|r| r.strip_prefix("bytes=")
                .and_then(|r| r.strip_suffix('-')).and_then(|n| n.parse::<usize>().ok()));
            let (status, body, range) = match start {
                Some(n) if n < data.len() => ("206 Partial Content", data[n..].to_vec(),
                    format!("Content-Range: bytes {n}-{}/{}\r\n", data.len() - 1, data.len())),
                Some(_) => return out.write_all(b"HTTP/1.1 416 Range Not Satisfiable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"),
                None => ("200 OK", data, String::new()),
            };
            let cut = shared.cut.take();
            drop(shared);
            out.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\n{range}Connection: close\r\n\r\n",
                body.len()).as_bytes())?;
            out.write_all(&body[..cut.unwrap_or(body.len()).min(body.len())])?;
            out.flush()?;
            if cut.is_some() { out.shutdown(std::net::Shutdown::Both)?; }
            Ok(())
        }
    }
    fn signed_feed(f: &Fixture, feed: &Feed, domain: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let bytes = serde_json::to_vec(feed).unwrap();
        let message = [domain, b"internal_test", b"\0", bytes.as_slice()].concat();
        (bytes.clone(), f.key.sign(&message).to_bytes().to_vec())
    }
    fn feed_for(version: &str, packages: &[(&str, &Path)]) -> Feed {
        Feed { schema: 1, channel: "beta".into(), version: version.into(),
            published: "2026-10-16T12:00:00Z".into(), min_version: "0.12.0test146".into(),
            notes: "What's new\n- In-app updates.".into(),
            packages: packages.iter().map(|(system, path)| {
                let bytes = fs::read(path).unwrap();
                (system.to_string(), FeedPackage {
                    file: format!("Linux-VST-Bridge-{version}-{system}-internal.install"),
                    size: bytes.len() as u64, sha256: crate::hex(&sha2::Sha256::digest(&bytes)) })
            }).collect() }
    }
    use sha2::Digest as _;
    fn publish(server: &Server, f: &Fixture, feed: &Feed, installer: &Path) {
        let (bytes, signature) = signed_feed(f, feed, FEED_DOMAIN);
        server.put(FEED_FILE, bytes);
        server.put(SIGNATURE_FILE, signature);
        for package in feed.packages.values() { server.put(&package.file, fs::read(installer).unwrap()); }
    }
    fn stage(f: &Fixture, link: &BetaLink, offer: &Offer, current: &str)
        -> std::result::Result<PathBuf, UpdateError> {
        let trust = f.trust();
        download_and_stage_with(&f.home, link, &f.trust(), current, offer, &mut |_, _| {},
            |frontend, home| crate::portable_package::expose_setup_with_trust(frontend, home, &trust))
    }
    fn staged_packages(f: &Fixture) -> usize {
        fs::read_dir(f.home.join(".local/share/linux-vst-bridge/packages")).map(|entries| entries
            .flatten().filter(|e| e.file_name().len() == 64).count()).unwrap_or(0)
    }
    fn downloads(f: &Fixture) -> usize {
        fs::read_dir(updates_dir(&f.home).join("downloads")).map(|e| e.count()).unwrap_or(0)
    }

    #[test]
    fn pasted_links_keep_the_key_out_of_urls_and_debug_output() {
        let link = BetaLink::parse("  https://Beta.Example.ts.net:8443/private-beta/?pw=s3cret%2BKey%2F42  ").unwrap();
        assert_eq!(link.url(FEED_FILE), "https://beta.example.ts.net:8443/private-beta/latest.json");
        assert_eq!(link.site(), "beta.example.ts.net");
        assert_eq!(link.key.as_deref(), Some(KEY));
        assert!(!format!("{link:?}").contains("s3cret"));
        // A link to one file of the folder means the folder.
        let file = BetaLink::parse("https://beta.example.ts.net:8443/private-beta/Linux-VST-Bridge-0.12.0test144-cachyos-internal.install?pw=k").unwrap();
        assert_eq!(file.url(FEED_FILE), "https://beta.example.ts.net:8443/private-beta/latest.json");
        assert_eq!(BetaLink::parse("https://h.example/b/#pw=k").unwrap().key.as_deref(), Some("k"));
        assert_eq!(BetaLink::parse("https://h.example").unwrap().url("a.install"), "https://h.example:443/a.install");
        for bad in ["", "beta link", "ftp://h/x/", "https://user:pw@h/x/", "https://h:0/x/",
            "https://h/x/?pw=a%0Db", "https://h/../x/", "https://h/x/%2e%2e/", "https://h/x\"/",
            "https://-h/x/", "https://h/x/ y"] {
            assert!(BetaLink::parse(bad).is_err(), "{bad}");
        }
        // Plain http only for this computer: the key must not cross a network unencrypted.
        assert!(BetaLink::parse("http://beta.example.ts.net:8443/private-beta/?pw=k").is_err());
        assert!(BetaLink::parse("http://127.0.0.1:8000/private-beta/?pw=k").is_ok());
        assert!(BetaLink::parse("http://localhost:8000/private-beta/?pw=k").is_ok());
    }

    #[test]
    fn link_is_stored_privately_and_replaced_by_a_new_paste() {
        let f = Fixture::new();
        assert_eq!(load_link(&f.home).unwrap(), None);
        let first = BetaLink::parse("https://h.example:8443/private-beta/?pw=old").unwrap();
        save_link(&f.home, &first).unwrap();
        let path = updates_dir(&f.home).join(LINK_FILE);
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
        assert_eq!(fs::metadata(updates_dir(&f.home)).unwrap().mode() & 0o777, 0o700);
        assert_eq!(load_link(&f.home).unwrap(), Some(first));
        let second = BetaLink::parse("https://h.example:8443/private-beta/?pw=new").unwrap();
        save_link(&f.home, &second).unwrap();
        assert_eq!(load_link(&f.home).unwrap(), Some(second));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load_link(&f.home).is_err(), "a readable key file is not used");
        forget_link(&f.home).unwrap();
        assert_eq!(load_link(&f.home).unwrap(), None);
    }

    #[test]
    fn curl_command_line_never_carries_the_key_or_the_url() {
        let link = BetaLink::parse("https://h.example:8443/private-beta/?pw=s3cret%22quote").unwrap();
        let command = curl_command(&link, Path::new("/tmp/out"), 100, true, Duration::from_secs(30));
        let args: Vec<_> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args[0], "--disable");
        assert!(args.iter().all(|a| !a.contains("s3cret") && !a.contains("h.example")), "{args:?}");
        assert!(args.windows(2).any(|w| w == ["--proto", "=https"]));
        let config = curl_config(&link, FEED_FILE);
        assert_eq!(config, "url = \"https://h.example:8443/private-beta/latest.json\"\nheader = \"PW: s3cret\\\"quote\"\n");
        assert!(!config.lines().next().unwrap().contains("s3cret"));
    }

    #[test]
    fn feed_signature_must_be_the_release_key_over_the_feed_domain() {
        let f = Fixture::new();
        let installer = f.bundle("0.12.0test148");
        let feed = feed_for("0.12.0test148", &[("cachyos", &installer)]);
        let (bytes, signature) = signed_feed(&f, &feed, FEED_DOMAIN);
        assert_eq!(verify_feed(&bytes, &signature, &f.trust()).unwrap(), feed);
        let mut changed = bytes.clone();
        let at = changed.iter().position(|b| *b == b'4').unwrap();
        changed[at] = b'9';
        assert!(verify_feed(&changed, &signature, &f.trust()).is_err());
        // A signature for another purpose (the package domain) never verifies a feed.
        let (_, package_domain) = signed_feed(&f, &feed, b"Linux VST Bridge user package v1\0");
        assert!(verify_feed(&bytes, &package_domain, &f.trust()).is_err());
        assert!(f.trust().verify_detached(b"Linux VST Bridge user package v1\0", &bytes, &package_domain).is_err());
        // The same key in the release class does not accept an internal-test feed.
        let release = Trust::new(f.key.verifying_key().to_bytes(), "release").unwrap();
        assert!(verify_feed(&bytes, &signature, &release).is_err());
        // Signed but malformed: unsafe file name.
        let mut unsafe_name = feed.clone();
        unsafe_name.packages.get_mut("cachyos").unwrap().file = "../evil.install".into();
        let (bytes, signature) = signed_feed(&f, &unsafe_name, FEED_DOMAIN);
        assert!(verify_feed(&bytes, &signature, &f.trust()).is_err());
    }

    #[test]
    fn versions_compare_numerically_and_never_offer_a_downgrade() {
        assert_eq!(compare_versions("0.12.0test148", "0.12.0test146"), Ordering::Greater);
        assert_eq!(compare_versions("0.12.0test99", "0.12.0test100"), Ordering::Less);
        assert_eq!(compare_versions("0.12.0test144", "0.12.1"), Ordering::Less);
        assert_eq!(compare_versions("0.12.0test144", "0.12.0test144"), Ordering::Equal);
        assert_eq!(display_version("0.12.0test148"), "Test148");
        assert_eq!(display_version("1.0.0"), "1.0.0");
        let package = FeedPackage { file: "a.install".into(), size: 1, sha256: "a".repeat(64) };
        let feed = |version: &str| Feed { schema: 1, channel: "beta".into(), version: version.into(),
            published: "now".into(), min_version: "0.12.0test146".into(), notes: String::new(),
            packages: [("cachyos".to_string(), package.clone())].into() };
        let systems = system_names("NAME=\"CachyOS Linux\"\nID=cachyos\nID_LIKE=\"arch\"\n");
        assert_eq!(systems, ["cachyos", "arch"]);
        assert!(matches!(decide(feed("0.12.0test148"), "0.12.0test146", &systems), Check::Available(o) if o.system == "cachyos"));
        assert_eq!(decide(feed("0.12.0test146"), "0.12.0test146", &systems), Check::UpToDate { latest: "0.12.0test146".into() });
        assert_eq!(decide(feed("0.12.0test140"), "0.12.0test146", &systems), Check::UpToDate { latest: "0.12.0test140".into() });
        assert_eq!(decide(feed("0.12.0test148"), "0.12.0test144", &systems), Check::ManualUpdate { latest: "0.12.0test148".into() });
        let steamos = system_names("ID=steamos\nID_LIKE=arch\n");
        assert!(matches!(decide(feed("0.12.0test148"), "0.12.0test146", &steamos), Check::NoPackage { .. }));
        // A feed may name a family: an Arch-like system takes its "arch" entry.
        let mut family = feed("0.12.0test148");
        family.packages = [("arch".to_string(), package.clone())].into();
        assert!(matches!(decide(family, "0.12.0test146", &steamos), Check::Available(o) if o.system == "arch"));
    }

    #[test]
    fn check_finds_the_newer_build_and_sends_the_key_only_as_a_header() {
        if !curl_available() { return; }
        let f = Fixture::new();
        let server = Server::start(Some(KEY));
        let installer = f.bundle("0.12.0test148");
        let feed = feed_for("0.12.0test148", &[("cachyos", &installer), ("steamos", &installer)]);
        publish(&server, &f, &feed, &installer);
        let systems = system_names("ID=cachyos\nID_LIKE=arch\n");
        let result = check(&f.home, &server.link(), &f.trust(), "0.12.0test146", &systems).unwrap();
        let Check::Available(offer) = result else { panic!("{result:?}") };
        assert_eq!(offer.version, "0.12.0test148");
        assert_eq!(offer.package, feed.packages["cachyos"]);
        assert!(offer.notes.contains("In-app updates"));
        let log = server.log();
        assert_eq!(log.len(), 2);
        for request in &log {
            let line = request.lines().next().unwrap();
            assert!(!line.contains("s3cret") && !line.to_ascii_lowercase().contains("pw"), "{line}");
            assert!(request.lines().any(|h| h == format!("PW: {KEY}")), "{request}");
        }
        assert_eq!(check(&f.home, &server.link(), &f.trust(), "0.12.0test148", &systems).unwrap(),
            Check::UpToDate { latest: "0.12.0test148".into() });
        // A changed feed with the old signature is refused after one retry.
        server.put(FEED_FILE, serde_json::to_vec(&Feed { version: "0.12.0test150".into(), ..feed.clone() }).unwrap());
        assert_eq!(check(&f.home, &server.link(), &f.trust(), "0.12.0test146", &systems),
            Err(UpdateError::FeedUnverified));
        // Scratch downloads never remain.
        assert_eq!(fs::read_dir(updates_dir(&f.home).join("check")).unwrap().count(), 0);
    }

    #[test]
    fn wrong_key_offline_and_missing_feed_are_one_plain_line() {
        if !curl_available() { return; }
        let f = Fixture::new();
        let server = Server::start(Some("rotated"));
        let systems = vec!["cachyos".to_string()];
        let rejected = check(&f.home, &server.link(), &f.trust(), "0.12.0test146", &systems).unwrap_err();
        assert_eq!(rejected, UpdateError::LinkRejected);
        assert_eq!(rejected.to_string(), "Your beta link no longer works; paste the new link.");
        let open = Server::start(None);
        assert_eq!(check(&f.home, &open.link(), &f.trust(), "0.12.0test146", &systems), Err(UpdateError::NoFeed));
        let closed = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);
        let offline = BetaLink::parse(&format!("http://127.0.0.1:{port}/private-beta/?pw={KEY}")).unwrap();
        let error = check(&f.home, &offline, &f.trust(), "0.12.0test146", &systems).unwrap_err();
        assert_eq!(error, UpdateError::Offline);
        assert_eq!(error.to_string(), "Couldn't check for updates right now.");
        for error in [rejected, error, UpdateError::Damaged, UpdateError::PackageUnverified,
            UpdateError::Local("user_package_intake_busy".into())] {
            assert!(!error.to_string().contains("s3cret") && !format!("{error:?}").contains("s3cret"));
        }
    }

    #[test]
    fn download_verifies_then_stages_like_the_installer() {
        if !curl_available() { return; }
        let f = Fixture::new();
        let server = Server::start(Some(KEY));
        let installer = f.bundle("0.12.0test148");
        let feed = feed_for("0.12.0test148", &[("cachyos", &installer)]);
        publish(&server, &f, &feed, &installer);
        let Check::Available(offer) = check(&f.home, &server.link(), &f.trust(), "0.12.0test146",
            &["cachyos".into()]).unwrap() else { panic!() };
        let mut seen = Vec::new();
        let trust = f.trust();
        let frontend = download_and_stage_with(&f.home, &server.link(), &f.trust(), "0.12.0test146",
            &offer, &mut |done, total| seen.push((done, total)),
            |frontend, home| crate::portable_package::expose_setup_with_trust(frontend, home, &trust)).unwrap();
        assert!(frontend.ends_with("usr/bin/linux-audio-compatibility-manager") && frontend.exists());
        assert_eq!(seen.last(), Some(&(offer.package.size, offer.package.size)));
        let setup = fs::read_to_string(f.home.join(".local/share/applications/linux-vst-bridge-setup.desktop")).unwrap();
        assert!(setup.contains(&format!("Exec=\"{}\"", frontend.display())));
        assert_eq!(downloads(&f), 0, "the verified installer is removed once staged");
        // The downloaded installer was never made executable or run: only staged files exist.
        assert_eq!(staged_packages(&f), 1);
    }

    #[test]
    fn interrupted_download_resumes_where_it_stopped() {
        if !curl_available() { return; }
        let f = Fixture::new();
        let server = Server::start(Some(KEY));
        let installer = f.bundle("0.12.0test148");
        let feed = feed_for("0.12.0test148", &[("cachyos", &installer)]);
        publish(&server, &f, &feed, &installer);
        let offer = Offer { version: feed.version.clone(), notes: String::new(), system: "cachyos".into(),
            package: feed.packages["cachyos"].clone() };
        server.shared.lock().unwrap().cut = Some(offer.package.size as usize / 2);
        assert_eq!(stage(&f, &server.link(), &offer, "0.12.0test146"), Err(UpdateError::DownloadStopped));
        assert_eq!(staged_packages(&f), 0);
        stage(&f, &server.link(), &offer, "0.12.0test146").unwrap();
        assert!(server.log().iter().any(|r| r.lines().any(|h| h.starts_with("Range: bytes="))));
        assert_eq!(staged_packages(&f), 1);
    }

    #[test]
    fn mismatched_size_hash_signature_or_version_installs_nothing() {
        if !curl_available() { return; }
        let f = Fixture::new();
        let server = Server::start(Some(KEY));
        let installer = f.bundle("0.12.0test148");
        let feed = feed_for("0.12.0test148", &[("cachyos", &installer)]);
        publish(&server, &f, &feed, &installer);
        let good = Offer { version: feed.version.clone(), notes: String::new(), system: "cachyos".into(),
            package: feed.packages["cachyos"].clone() };
        let link = server.link();
        // SHA-256 differs from the signed feed.
        let mut wrong_hash = good.clone();
        wrong_hash.package.sha256 = "0".repeat(64);
        assert_eq!(stage(&f, &link, &wrong_hash, "0.12.0test146"), Err(UpdateError::Damaged));
        // Larger than announced: the size cap stops it.
        let mut smaller = good.clone();
        smaller.package.size -= 100;
        assert_eq!(stage(&f, &link, &smaller, "0.12.0test146"), Err(UpdateError::Damaged));
        assert_eq!((staged_packages(&f), downloads(&f)), (0, 0));
        // Published bytes whose own signature fails: the feed hash matches, the package does not verify.
        let mut tampered = fs::read(&installer).unwrap();
        tampered[60] ^= 1;
        let mut forged = good.clone();
        forged.package.sha256 = crate::hex(&sha2::Sha256::digest(&tampered));
        server.put(&good.package.file, tampered);
        assert_eq!(stage(&f, &link, &forged, "0.12.0test146"), Err(UpdateError::PackageUnverified));
        // A correctly signed older package offered as newer is refused.
        let older = f.bundle("0.12.0test140");
        let bytes = fs::read(&older).unwrap();
        let mut relabelled = good.clone();
        relabelled.package.size = bytes.len() as u64;
        relabelled.package.sha256 = crate::hex(&sha2::Sha256::digest(&bytes));
        server.put(&good.package.file, bytes);
        assert_eq!(stage(&f, &link, &relabelled, "0.12.0test146"), Err(UpdateError::PackageUnverified));
        // Not newer than the running version.
        assert_eq!(stage(&f, &link, &good, "0.12.0test148"), Err(UpdateError::Downgrade));
        // Missing file and rotated key.
        let mut missing = good.clone();
        missing.package.file = "Gone.install".into();
        assert_eq!(stage(&f, &link, &missing, "0.12.0test146"), Err(UpdateError::PackageMissing));
        server.shared.lock().unwrap().key = Some("rotated".into());
        assert_eq!(stage(&f, &link, &good, "0.12.0test146"), Err(UpdateError::LinkRejected));
        assert_eq!((staged_packages(&f), downloads(&f)), (0, 0));
    }

    #[test]
    fn installed_version_reads_the_selected_generation_record() {
        let f = Fixture::new();
        let root = f.home.join(".local/share/linux-vst-bridge/managed");
        let generation = root.join("software/generation");
        fs::create_dir_all(&generation).unwrap();
        let artifact = |name: &str| crate::Artifact { path: generation.join(name), sha256: "0".repeat(64) };
        let software = crate::catalogue::Software { installer_launch: None, preparation_kit: None,
            operator_frontend: None, manager: artifact("linux-vst-bridge"), supervisor: artifact("session.pyc"),
            ownership: artifact("ownership.pyc"), host: artifact("host.exe"),
            source_manifest: artifact("host-source-manifest.json"), source_sha256: "0".repeat(64),
            native_catalogue: None };
        atomic_json(&root.join("software.json"), &software).unwrap();
        assert_eq!(installed_version(&f.home).unwrap(), None);
        atomic_json(&generation.join("package-generation.json"),
            &serde_json::json!({"schema":1,"manifest":{"version":"0.12.0test146"}})).unwrap();
        assert_eq!(installed_version(&f.home).unwrap().as_deref(), Some("0.12.0test146"));
    }
}

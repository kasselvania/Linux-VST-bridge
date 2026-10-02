//! Worker preparation only. Never called from process/setProcessing.
//! The DAW owns its process limits; this code does not change them.
use std::{io::{self, Read, Write}, path::{Path, PathBuf}, time::{Duration, Instant}};

const RR: i32 = 2;
const RESET_ON_FORK: i32 = 0x4000_0000;
const PRIORITY: i32 = 5;
const RTTIME_US: u64 = 200_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Policy { kind: i32, priority: i32 }
impl Policy {
    fn ordinary(self) -> bool { self.kind & !RESET_ON_FORK == 0 && self.priority == 0 }
    fn requested(self) -> bool { self.kind == (RR | RESET_ON_FORK) && self.priority == PRIORITY }
}

pub(crate) struct Preparation { directory: PathBuf, session: [u8; 16] }
impl Preparation {
    pub(crate) fn from_binding(binding: &crate::preview::Binding) -> Option<Self> {
        binding.owner.as_ref().map(|_| Self { directory: binding.directory.clone(), session: binding.session })
    }
    fn header(&self) -> Vec<u8> {
        [b"LVNS".as_slice(), &1u32.to_le_bytes(), &self.session].concat()
    }
    fn request(&self, pid: u32, tid: u32, start: u64, timeout: Duration) -> io::Result<bool> {
        use std::os::unix::fs::OpenOptionsExt;
        let header = self.header();
        // An older supervisor advertises nothing. Do not send new bytes on the
        // existing ownership socket or wait for an unsupported operation.
        if private_read(&self.directory.join("native-scheduling.supported"), 24)? != header {
            return Ok(false);
        }
        let request = [header.as_slice(), &pid.to_le_bytes(), &tid.to_le_bytes(), &start.to_le_bytes()].concat();
        let temp = self.directory.join("native-scheduling.request.tmp");
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp)?;
        file.write_all(&request)?;
        drop(file);
        std::fs::rename(temp, self.directory.join("native-scheduling.request"))?;
        let until = Instant::now() + timeout;
        loop {
            match private_read(&self.directory.join("native-scheduling.reply"), 28) {
                Ok(bytes) => return Ok(bytes[..24] == header && matches!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 1 | 2)),
                Err(e) if e.kind() == io::ErrorKind::NotFound && Instant::now() < until => std::thread::sleep(Duration::from_millis(5)),
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
                Err(e) => return Err(e),
            }
        }
    }
}
fn private_read(path: &Path, length: usize) -> io::Result<Vec<u8>> {
    use std::os::unix::fs::MetadataExt;
    let before = std::fs::symlink_metadata(path)?;
    if !before.is_file() || before.mode() & 0o077 != 0 || before.len() != length as u64 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "scheduling record custody"));
    }
    let mut file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    if (before.dev(), before.ino(), before.uid()) != (opened.dev(), opened.ino(), opened.uid()) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "scheduling record changed"));
    }
    let mut bytes = vec![0; length]; file.read_exact(&mut bytes)?; Ok(bytes)
}

#[derive(Debug)]
pub(crate) struct Observation {
    outcome: &'static str,
    route: &'static str,
    before: Option<Policy>,
    after: Option<Policy>,
    clock: [u64; 2],
}
impl Observation {
    fn skipped(reason: &'static str) -> Self {
        Self { outcome: reason, route: "none", before: None, after: None, clock: [0; 2] }
    }
    pub(crate) fn report(&self, path: &Path) {
        let policy = |p: Option<Policy>| p.map_or("null".to_owned(), |p|
            format!("{{\"policy\":{},\"priority\":{}}}", p.kind, p.priority));
        let text = format!(concat!("{{\"event\":\"native_audio_scheduling\",\"schema\":1,",
            "\"outcome\":\"{}\",\"route\":\"{}\",\"before\":{},\"effective\":{},",
            "\"clock_monotonic_ns\":[{},{}],\"requested_priority\":5,",
            "\"host_limits_changed\":false}}\n"),
            self.outcome, self.route, policy(self.before), policy(self.after), self.clock[0], self.clock[1]);
        crate::preview::append_report(path, text.as_bytes());
    }
}

// A denied, unavailable or ineffectual capability request is never a success.
// Existing policies and the enclosing DAW's resource budget are preserved.
fn acquire(before: Option<Policy>, hard_rttime: Option<u64>,
    request: impl FnOnce() -> (bool, Option<Policy>)) -> Observation {
    let mut result = Observation { before, after: before, ..Observation::skipped("policy_unavailable") };
    let Some(policy) = before else { return result; };
    if !policy.ordinary() {
        result.outcome = "existing_policy_preserved";
    } else if !hard_rttime.is_some_and(|v| v > 0 && v <= RTTIME_US) {
        result.outcome = "host_realtime_budget_unavailable";
    } else {
        let (accepted, effective) = request();
        result.after = effective;
        result.outcome = if accepted && effective.is_some_and(Policy::requested) {
            "effective"
        } else { "unavailable" };
    }
    result
}

pub(crate) fn prepare(preparation: Option<Preparation>) -> Observation {
    let Some(preparation) = preparation else { return Observation::skipped("unmanaged_fixture"); };
    #[cfg(target_os = "linux")]
    { linux::prepare(preparation) }
    #[cfg(not(target_os = "linux"))]
    { let _ = preparation; Observation::skipped("unsupported_platform") }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::os::raw::{c_int, c_ulong};
    #[repr(C)] struct Param { priority: c_int }
    #[repr(C)] struct Limit { soft: c_ulong, hard: c_ulong }
    unsafe extern "C" {
        fn gettid() -> c_int;
        fn sched_getscheduler(pid: c_int) -> c_int;
        fn sched_getparam(pid: c_int, param: *mut Param) -> c_int;
        fn getrlimit(resource: c_int, limit: *mut Limit) -> c_int;
    }
    fn current() -> Option<Policy> {
        let kind = unsafe { sched_getscheduler(0) };
        let mut param = Param { priority: 0 };
        (kind >= 0 && unsafe { sched_getparam(0, &mut param) } == 0)
            .then_some(Policy { kind, priority: param.priority })
    }
    pub(super) fn prepare(preparation: Preparation) -> Observation {
        let began = crate::observer::monotonic_ns();
        let mut limit = Limit { soft: 0, hard: 0 };
        let hard = (unsafe { getrlimit(15, &mut limit) } == 0).then_some(limit.hard as u64);
        let mut result = acquire(current(), hard, || {
            let accepted = (|| -> std::io::Result<bool> {
                let tid = unsafe { gettid() } as u32;
                let mut text = String::new();
                std::fs::File::open(format!("/proc/self/task/{tid}/stat"))?.take(4097).read_to_string(&mut text)?;
                let start = (text.len() <= 4096).then(|| text.rsplit_once(')')?.1.split_whitespace().nth(19)?.parse().ok()).flatten()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "worker identity"))?;
                preparation.request(std::process::id(), tid, start, Duration::from_secs(3))
            })().unwrap_or(false);
            (accepted, current())
        });
        result.route = "owned_supervisor_realtimekit";
        result.clock = [began, crate::observer::monotonic_ns()];
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const OTHER: Policy = Policy { kind: 0, priority: 0 };
    const REQUESTED: Policy = Policy { kind: RR | RESET_ON_FORK, priority: PRIORITY };
    #[test]
    fn preparation_is_negotiated_bounded_and_checks_session_reply() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("native-scheduling-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let preparation = Preparation { directory: root.clone(), session: [0x42; 16] };
        // Old supervisors have no advertisement and receive no new request.
        assert!(preparation.request(10, 11, 12, Duration::from_millis(10)).is_err());
        assert!(!root.join("native-scheduling.request").exists());
        let write = |name: &str, data: &[u8]| {
            let path = root.join(name); std::fs::write(&path, data).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        };
        write("native-scheduling.supported", &preparation.header());
        let began = Instant::now();
        assert!(!preparation.request(10, 11, 12, Duration::from_millis(10)).unwrap());
        assert!(began.elapsed() < Duration::from_secs(1));
        let bytes = std::fs::read(root.join("native-scheduling.request")).unwrap();
        assert_eq!(&bytes[24..], &[10u32.to_le_bytes().as_slice(), &11u32.to_le_bytes(), &12u64.to_le_bytes()].concat());
        let mut reply = preparation.header(); reply.extend(1u32.to_le_bytes());
        write("native-scheduling.reply", &reply);
        assert!(preparation.request(10, 11, 12, Duration::ZERO).unwrap());
        reply[8] ^= 1; write("native-scheduling.reply", &reply);
        assert!(!preparation.request(10, 11, 12, Duration::ZERO).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn ordinary_worker_requests_and_verifies_realtime() {
        let mut called = false;
        let r = acquire(Some(OTHER), Some(RTTIME_US), || { called = true; (true, Some(REQUESTED)) });
        assert!(called);
        assert_eq!(r.outcome, "effective");
        assert_eq!(r.before, Some(OTHER));
        assert_eq!(r.after, Some(REQUESTED));
    }
    #[test]
    fn host_policy_and_process_budget_are_not_overridden() {
        for p in [Policy { kind: 1, priority: 20 }, REQUESTED, Policy { kind: 3, priority: 0 }] {
            let r = acquire(Some(p), Some(RTTIME_US), || panic!("must preserve host policy"));
            assert_eq!(r.outcome, "existing_policy_preserved");
            assert_eq!(r.after, Some(p));
        }
        for budget in [None, Some(0), Some(RTTIME_US + 1), Some(u64::MAX)] {
            let r = acquire(Some(OTHER), budget, || panic!("must preserve host budget"));
            assert_eq!(r.outcome, "host_realtime_budget_unavailable");
        }
    }
    #[test]
    fn denial_or_bad_readback_never_claims_effective_policy() {
        for reply in [(false, Some(OTHER)), (true, Some(OTHER)), (true, None),
            (true, Some(Policy { kind: RR, priority: PRIORITY })), (false, Some(REQUESTED))] {
            assert_eq!(acquire(Some(OTHER), Some(RTTIME_US), || reply).outcome, "unavailable");
        }
        assert_eq!(acquire(None, Some(RTTIME_US), || panic!("unknown policy")).outcome, "policy_unavailable");
    }
}

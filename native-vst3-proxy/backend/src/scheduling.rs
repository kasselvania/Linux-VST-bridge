//! Worker preparation and off-path caller-thread requests. Nothing here runs
//! on process/setProcessing. The DAW owns its process limits; this code does
//! not change them.
use std::{io::{self, Read, Write}, path::{Path, PathBuf}, time::{Duration, Instant}};

const RR: i32 = 2;
const RESET_ON_FORK: i32 = 0x4000_0000;
const PRIORITY: i32 = 5;
const RTTIME_US: u64 = 200_000;
const WORKER_SUPPORTED: &str = "native-scheduling.supported";
const WORKER_REQUEST: &str = "native-scheduling.request";
const WORKER_REPLY: &str = "native-scheduling.reply";
const CALLER_SUPPORTED: &str = "caller-scheduling.supported";
const CALLER_REQUEST: &str = "caller-scheduling.request";
const CALLER_REPLY: &str = "caller-scheduling.reply";
const CALLER_TIMEOUT: Duration = Duration::from_secs(3);
/// Distinct DAW threads whose process() calls are recorded for a policy request.
pub(crate) const CALLER_SLOTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Policy { kind: i32, priority: i32 }
impl Policy {
    fn ordinary(self) -> bool { self.kind & !RESET_ON_FORK == 0 && self.priority == 0 }
    fn requested(self) -> bool { self.kind == (RR | RESET_ON_FORK) && self.priority == PRIORITY }
}

#[derive(Clone)]
pub(crate) struct Preparation { directory: PathBuf, session: [u8; 16] }
impl Preparation {
    pub(crate) fn from_binding(binding: &crate::preview::Binding) -> Option<Self> {
        binding.owner.as_ref().map(|_| Self { directory: binding.directory.clone(), session: binding.session })
    }
    fn header(&self) -> Vec<u8> {
        [b"LVNS".as_slice(), &1u32.to_le_bytes(), &self.session].concat()
    }
    /// Publishes one 40-byte request if the supervisor advertises the role.
    /// An older supervisor advertises nothing; no new bytes go on the existing
    /// ownership socket and no unsupported operation is waited for.
    fn write_request(&self, supported: &str, name: &str, pid: u32, tid: u32, start: u64) -> io::Result<bool> {
        use std::os::unix::fs::OpenOptionsExt;
        let header = self.header();
        if private_read(&self.directory.join(supported), 24)? != header {
            return Ok(false);
        }
        let request = [header.as_slice(), &pid.to_le_bytes(), &tid.to_le_bytes(), &start.to_le_bytes()].concat();
        let temp = self.directory.join(format!("{name}.tmp"));
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temp)?;
        file.write_all(&request)?;
        drop(file);
        std::fs::rename(temp, self.directory.join(name))?;
        Ok(true)
    }
    fn request(&self, pid: u32, tid: u32, start: u64, timeout: Duration) -> io::Result<bool> {
        if !self.write_request(WORKER_SUPPORTED, WORKER_REQUEST, pid, tid, start)? { return Ok(false); }
        let header = self.header();
        let until = Instant::now() + timeout;
        loop {
            match private_read(&self.directory.join(WORKER_REPLY), 28) {
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
    role: &'static str,
    thread: u32,
}
impl Observation {
    fn skipped(reason: &'static str) -> Self {
        Self { outcome: reason, route: "none", before: None, after: None, clock: [0; 2], role: "worker", thread: 0 }
    }
    pub(crate) fn report(&self, path: &Path) {
        let policy = |p: Option<Policy>| p.map_or("null".to_owned(), |p|
            format!("{{\"policy\":{},\"priority\":{}}}", p.kind, p.priority));
        let text = format!(concat!("{{\"event\":\"native_audio_scheduling\",\"schema\":1,\"role\":\"{}\",\"thread\":{},",
            "\"outcome\":\"{}\",\"route\":\"{}\",\"before\":{},\"effective\":{},",
            "\"clock_monotonic_ns\":[{},{}],\"requested_priority\":5,",
            "\"host_limits_changed\":false}}\n"),
            self.role, self.thread, self.outcome, self.route, policy(self.before), policy(self.after), self.clock[0], self.clock[1]);
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
mod platform {
    use super::Policy;
    use std::io::Read;
    use std::os::raw::{c_int, c_ulong};
    #[repr(C)] struct Param { priority: c_int }
    #[repr(C)] struct Limit { soft: c_ulong, hard: c_ulong }
    unsafe extern "C" {
        pub(super) fn gettid() -> c_int;
        fn sched_getscheduler(pid: c_int) -> c_int;
        fn sched_getparam(pid: c_int, param: *mut Param) -> c_int;
        fn getrlimit(resource: c_int, limit: *mut Limit) -> c_int;
    }
    /// Policy of one thread of this process; 0 names the calling thread.
    pub(super) fn policy_of(tid: u32) -> Option<Policy> {
        let kind = unsafe { sched_getscheduler(tid as c_int) };
        let mut param = Param { priority: 0 };
        (kind >= 0 && unsafe { sched_getparam(tid as c_int, &mut param) } == 0)
            .then_some(Policy { kind, priority: param.priority })
    }
    pub(super) fn hard_rttime() -> Option<u64> {
        let mut limit = Limit { soft: 0, hard: 0 };
        // c_ulong is target-width dependent; keep this observation's wire width fixed.
        #[allow(clippy::unnecessary_cast)]
        (unsafe { getrlimit(15, &mut limit) } == 0).then_some(limit.hard as u64)
    }
    /// Kernel start tick of one thread of this process, as the supervisor
    /// verifies it. A reused thread ID cannot carry the same start.
    pub(super) fn thread_start(tid: u32) -> Option<u64> {
        let mut text = String::new();
        std::fs::File::open(format!("/proc/self/task/{tid}/stat")).ok()?.take(4097).read_to_string(&mut text).ok()?;
        if text.len() > 4096 { return None; }
        text.rsplit_once(')')?.1.split_whitespace().nth(19)?.parse().ok()
    }
}
#[cfg(not(target_os = "linux"))]
mod platform {
    use super::Policy;
    pub(super) fn policy_of(_tid: u32) -> Option<Policy> { None }
    pub(super) fn hard_rttime() -> Option<u64> { None }
    pub(super) fn thread_start(_tid: u32) -> Option<u64> { None }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    pub(super) fn prepare(preparation: Preparation) -> Observation {
        let began = crate::observer::monotonic_ns();
        let tid = unsafe { platform::gettid() } as u32;
        let mut result = acquire(platform::policy_of(0), platform::hard_rttime(), || {
            let accepted = platform::thread_start(tid)
                .map(|start| preparation.request(std::process::id(), tid, start, Duration::from_secs(3)).unwrap_or(false))
                .unwrap_or(false);
            (accepted, platform::policy_of(0))
        });
        result.route = "owned_supervisor_realtimekit";
        result.clock = [began, crate::observer::monotonic_ns()];
        result.thread = tid;
        result
    }
}

/// One off-path real-time request for a DAW thread that called process().
/// The worker begins it and polls it between its ordinary wakes. No thread
/// sleeps or blocks on the reply, and the request never runs on the callback.
pub(crate) struct CallerGrant {
    preparation: Preparation,
    tid: u32,
    before: Option<Policy>,
    began: u64,
    deadline: Instant,
}
pub(crate) enum CallerStep { Done(Observation), Pending(CallerGrant) }
impl CallerGrant {
    fn observation(tid: u32, began: u64, before: Option<Policy>, after: Option<Policy>, outcome: &'static str, route: &'static str) -> Observation {
        Observation { outcome, route, before, after, clock: [began, crate::observer::monotonic_ns()], role: "caller", thread: tid }
    }
    pub(crate) fn begin(preparation: &Preparation, tid: u32) -> CallerStep {
        let began = crate::observer::monotonic_ns();
        let before = platform::policy_of(tid);
        let done = |outcome, route| CallerStep::Done(Self::observation(tid, began, before, before, outcome, route));
        let Some(policy) = before else { return done("policy_unavailable", "none"); };
        if !policy.ordinary() { return done("existing_policy_preserved", "none"); }
        if !platform::hard_rttime().is_some_and(|v| v > 0 && v <= RTTIME_US) {
            return done("host_realtime_budget_unavailable", "none");
        }
        let Some(start) = platform::thread_start(tid) else { return done("caller_identity_unavailable", "none"); };
        match preparation.write_request(CALLER_SUPPORTED, CALLER_REQUEST, std::process::id(), tid, start) {
            Ok(true) => CallerStep::Pending(Self { preparation: preparation.clone(), tid, before, began, deadline: Instant::now() + CALLER_TIMEOUT }),
            // No advertisement at all is an older supervisor, not a failure.
            Ok(false) => done("unsupported_supervisor", "none"),
            Err(e) if e.kind() == io::ErrorKind::NotFound => done("unsupported_supervisor", "none"),
            Err(_) => done("request_unavailable", "owned_supervisor_realtimekit"),
        }
    }
    pub(crate) fn poll(self) -> CallerStep {
        const ROUTE: &str = "owned_supervisor_realtimekit";
        let reply = self.preparation.directory.join(CALLER_REPLY);
        let expired = Instant::now() >= self.deadline;
        match private_read(&reply, 32) {
            Ok(bytes) => {
                let _ = std::fs::remove_file(&reply);
                let tid = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
                let code = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
                if bytes[..24] != self.preparation.header()[..] || tid != self.tid {
                    // A reply left over from another thread's request is discarded.
                    return if expired { self.finish("reply_timeout", ROUTE) } else { CallerStep::Pending(self) };
                }
                let after = platform::policy_of(self.tid);
                let outcome = if matches!(code, 1 | 2) && after.is_some_and(Policy::requested) { "effective" } else { "unavailable" };
                CallerStep::Done(Self::observation(self.tid, self.began, self.before, after, outcome, ROUTE))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound && !expired => CallerStep::Pending(self),
            Err(e) if e.kind() == io::ErrorKind::NotFound => self.finish("reply_timeout", ROUTE),
            Err(_) => self.finish("reply_unavailable", ROUTE),
        }
    }
    fn finish(self, outcome: &'static str, route: &'static str) -> CallerStep {
        CallerStep::Done(Self::observation(self.tid, self.began, self.before, platform::policy_of(self.tid), outcome, route))
    }
}

/// Worker-owned sequence of caller grants: one outstanding request at a time,
/// each recorded thread requested once, at most one small private file
/// operation per service call.
pub(crate) struct CallerGrants {
    preparation: Option<Preparation>,
    next: usize,
    pending: Option<CallerGrant>,
    observations: Vec<Observation>,
}
impl CallerGrants {
    pub(crate) fn new(preparation: Option<Preparation>) -> Self {
        Self { preparation, next: 0, pending: None, observations: Vec::with_capacity(CALLER_SLOTS) }
    }
    pub(crate) fn service(&mut self, callers: &[std::sync::atomic::AtomicU64]) {
        if let Some(grant) = self.pending.take() {
            match grant.poll() {
                CallerStep::Pending(grant) => { self.pending = Some(grant); return; }
                CallerStep::Done(observation) => self.observations.push(observation),
            }
        }
        let Some(preparation) = &self.preparation else { return; };
        while self.next < callers.len() {
            let tid = callers[self.next].load(std::sync::atomic::Ordering::Acquire);
            if tid == 0 { return; }
            self.next += 1;
            match CallerGrant::begin(preparation, tid as u32) {
                CallerStep::Pending(grant) => { self.pending = Some(grant); return; }
                CallerStep::Done(observation) => self.observations.push(observation),
            }
        }
    }
    pub(crate) fn report(&self, path: &Path) {
        for observation in &self.observations { observation.report(path); }
    }
    #[cfg(test)]
    fn outcomes(&self) -> Vec<&'static str> { self.observations.iter().map(|o| o.outcome).collect() }
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
    #[test]
    fn caller_grants_request_each_recorded_thread_once_and_never_block() {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};
        let root = std::env::temp_dir().join(format!("caller-scheduling-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let preparation = Preparation { directory: root.clone(), session: [0x42; 16] };
        let write = |name: &str, data: &[u8]| {
            let path = root.join(name); std::fs::write(&path, data).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
        };
        let callers: [AtomicU64; CALLER_SLOTS] = std::array::from_fn(|_| AtomicU64::new(0));
        // The grant refuses without a finite real-time budget. Lowering this
        // process's own RLIMIT_RTTIME is always permitted and affects no RT thread here.
        let budget = libc::rlimit { rlim_cur: RTTIME_US as libc::rlim_t, rlim_max: RTTIME_US as libc::rlim_t };
        assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_RTTIME, &budget) }, 0);
        assert_eq!(platform::hard_rttime(), Some(RTTIME_US));
        // Nothing recorded: no request, no observation.
        let mut grants = CallerGrants::new(Some(preparation.clone()));
        grants.service(&callers);
        assert!(grants.outcomes().is_empty() && !root.join(CALLER_REQUEST).exists());
        // This test thread is an ordinary thread of this process; its policy
        // and start are readable. An old supervisor advertises nothing.
        let me = unsafe { platform::gettid() } as u64;
        callers[0].store(me, Ordering::Release);
        grants.service(&callers);
        assert_eq!(grants.outcomes(), vec!["unsupported_supervisor"]);
        assert!(!root.join(CALLER_REQUEST).exists());
        // A supporting supervisor: the request carries this process/thread and
        // the worker keeps going while no reply exists.
        write(CALLER_SUPPORTED, &preparation.header());
        let mut grants = CallerGrants::new(Some(preparation.clone()));
        let began = Instant::now();
        grants.service(&callers);
        assert!(began.elapsed() < Duration::from_millis(500));
        let bytes = std::fs::read(root.join(CALLER_REQUEST)).unwrap();
        assert_eq!(&bytes[..24], &preparation.header()[..]);
        assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), std::process::id());
        assert_eq!(u32::from_le_bytes(bytes[28..32].try_into().unwrap()), me as u32);
        assert!(u64::from_le_bytes(bytes[32..40].try_into().unwrap()) > 0);
        grants.service(&callers);
        assert!(grants.outcomes().is_empty() && grants.pending.is_some());
        // A reply for another thread is discarded; ours completes the grant.
        // The policy did not actually change here, so acceptance reads unavailable.
        let mut other = preparation.header(); other.extend(7u32.to_le_bytes()); other.extend(1u32.to_le_bytes());
        write(CALLER_REPLY, &other);
        grants.service(&callers);
        assert!(grants.outcomes().is_empty() && !root.join(CALLER_REPLY).exists());
        let mut reply = preparation.header(); reply.extend((me as u32).to_le_bytes()); reply.extend(1u32.to_le_bytes());
        write(CALLER_REPLY, &reply);
        grants.service(&callers);
        assert_eq!(grants.outcomes(), vec!["unavailable"]);
        assert!(!root.join(CALLER_REPLY).exists());
        // The same thread is never requested twice; a second recorded thread is.
        std::fs::remove_file(root.join(CALLER_REQUEST)).unwrap();
        grants.service(&callers);
        assert!(!root.join(CALLER_REQUEST).exists());
        callers[1].store(u64::MAX >> 1, Ordering::Release); // no such thread
        grants.service(&callers);
        assert_eq!(grants.outcomes(), vec!["unavailable", "policy_unavailable"]);
        // Unmanaged fixtures request nothing.
        let mut none = CallerGrants::new(None);
        none.service(&callers);
        assert!(none.outcomes().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}

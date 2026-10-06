//! An owned render-thread scheduling request, run by the supervisor, never by
//! the DAW callback. RealtimeKit remains the permission and resource authority.
use linux_vst_bridge::{require, Result};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, os::unix::fs::MetadataExt, path::{Path, PathBuf},
    process::{Command, Stdio}, time::{Duration, Instant}};

const PRIORITY: i32 = 5;
const RTTIME_US: u64 = 200_000;
const MAX_OWNERS: usize = 256;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    schema: u32,
    session: String,
    status: PathBuf,
    owned: Vec<(i32, u64)>,
    // Supplied only by the supervisor from the authenticated owner socket's
    // retained peer identity. Never supplied by the native request file.
    native_peer: Option<(i32, u64)>,
    // "worker" (default) names the proxy's own transport thread; "caller"
    // names a DAW thread the proxy recorded calling process(). Each role has
    // its own request file; the DAW's thread name is not a selection input.
    #[serde(default)]
    native_role: Option<String>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Target { pid: i32, process_start: u64, tid: i32, thread_start: u64 }
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Policy { policy: i32, priority: i32 }

// Observation of the supervisor-supplied cohort, never new ownership authority.
// Keep the validated session directory pinned; neither replacement paths nor
// symlinks may redirect the audit write. Normal builds perform none of this work.
#[cfg(feature = "pb0-c0-audit")]
mod audit {
    use super::*;
    use std::{os::fd::AsRawFd, os::unix::fs::OpenOptionsExt};
    const FILENAME: &str = "windows-scheduling.audit.json";
    pub(super) struct Owned<'a> {
        request: &'a Request,
        directory: fs::File,
        parent: PathBuf,
        device: u64,
        inode: u64,
        version: u32,
        header: [u8;32],
        pub(super) render_matches: Option<usize>,
    }
    #[derive(Serialize)]
    struct Record<'a> {
        schema: u32, request_schema: u32, session: &'a str,
        basis: &'static str, clock: &'static str, status_device: u64, status_inode: u64,
        status_version: u32, status_extent: u64, owned: &'a [(i32,u64)],
        monotonic_ns: Option<u64>, render_matches: Option<usize>,
    }
    impl<'a> Owned<'a> {
        pub(super) fn new(request: &'a Request, status: &fs::Metadata, header: [u8;32]) -> Option<Self> {
            if request.status.file_name()? != "ap12.status" {return None;}
            let parent = request.status.parent()?.to_owned();
            let directory = fs::OpenOptions::new().read(true)
                .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW).open(&parent).ok()?;
            let meta = directory.metadata().ok()?;
            if !meta.is_dir() || meta.uid() != status.uid() || meta.mode() & 0o077 != 0 {return None;}
            let observed = Self {request,directory,parent,device:status.dev(),inode:status.ino(),
                version:u32::from_le_bytes(header[4..8].try_into().ok()?),header,render_matches:None};
            observed.revalidate().ok()?;
            Some(observed)
        }
        fn pinned(&self) -> PathBuf {
            PathBuf::from(format!("/proc/self/fd/{}",self.directory.as_raw_fd()))
        }
        fn revalidate(&self) -> Result<()> {
            let expected = self.directory.metadata()?;
            let current = fs::symlink_metadata(&self.parent)?;
            require(current.is_dir() && current.dev() == expected.dev() && current.ino() == expected.ino()
                && current.uid() == expected.uid() && current.mode() & 0o077 == 0,
                "scheduling_audit_directory_changed")?;
            let mut file = linux_vst_bridge::file(&self.pinned().join("ap12.status"))?;
            let status = file.metadata()?;
            require(status.dev() == self.device && status.ino() == self.inode
                && status.uid() == expected.uid() && status.mode() & 0o077 == 0 && status.len() == 1024,
                "scheduling_audit_status_changed")?;
            let mut header = [0;32];file.read_exact(&mut header)?;
            require(header == self.header,"scheduling_audit_header_changed")
        }
        fn export(&self) -> Result<()> {
            self.revalidate()?;
            let mut time = libc::timespec {tv_sec:0,tv_nsec:0};
            let monotonic_ns = if unsafe {libc::clock_gettime(libc::CLOCK_MONOTONIC,&mut time)} == 0 {
                u64::try_from(time.tv_sec).ok().and_then(|seconds|seconds.checked_mul(1_000_000_000))
                    .and_then(|seconds|u64::try_from(time.tv_nsec).ok().and_then(|ns|seconds.checked_add(ns)))
            } else {None};
            linux_vst_bridge::atomic_json(&self.pinned().join(FILENAME),&Record {
                schema:1,request_schema:self.request.schema,session:&self.request.session,
                basis:"supervisor_supplied_cohort",clock:"clock_monotonic",status_device:self.device,status_inode:self.inode,
                status_version:self.version,status_extent:1024,owned:&self.request.owned,
                monotonic_ns,render_matches:self.render_matches,
            })
        }
    }
    impl Drop for Owned<'_> {
        fn drop(&mut self) {let _ = self.export();}
    }
}

fn bounded(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?.take(maximum + 1).read_to_end(&mut bytes)?;
    require(bytes.len() as u64 <= maximum, "scheduling_read_extent")?;
    Ok(bytes)
}
fn start(proc: &Path, id: i32) -> Result<u64> {
    let bytes = bounded(&proc.join("stat"), 4096)?;
    let text = std::str::from_utf8(&bytes)?;
    let (number, _) = text.split_once(" (").ok_or("scheduling_stat")?;
    require(number.parse::<i32>()? == id, "scheduling_pid_changed")?;
    Ok(text.rsplit_once(')').ok_or("scheduling_stat")?.1.split_whitespace()
        .nth(19).ok_or("scheduling_stat")?.parse()?)
}
fn map_inodes(bytes: &[u8], inode: u64, device: u64) -> Result<bool> {
    // /proc maps gives the backing inode and device even across a mount alias.
    // A name, Windows PID or path string alone never selects a process.
    for line in std::str::from_utf8(bytes)?.lines() {
        let fields: Vec<_> = line.split_whitespace().take(5).collect();
        require(fields.len() == 5, "scheduling_maps_format")?;
        let (major, minor) = fields[3].split_once(':').ok_or("scheduling_maps_device")?;
        if fields[4].parse::<u64>()? == inode
            && u64::from_str_radix(major, 16)? == u64::from(libc::major(device))
            && u64::from_str_radix(minor, 16)? == u64::from(libc::minor(device)) {
            return Ok(true);
        }
    }
    Ok(false)
}
fn select(request: &Request, proc_root: &Path, uid: u32) -> Result<Target> {
    require(request.schema == (if request.native_peer.is_some() { 2 } else { 1 })
        && request.session.len() == 32
        && request.session.bytes().all(|b| b.is_ascii_hexdigit())
        && (request.native_peer.is_some() || !request.owned.is_empty()) && request.owned.len() <= MAX_OWNERS,
        "scheduling_request")?;
    let mut status_file = linux_vst_bridge::file(&request.status)?;
    let status = status_file.metadata()?;
    require(status.is_file() && status.uid() == uid && status.mode() & 0o077 == 0
        && status.len() == 1024, "scheduling_status_custody")?;
    // Only the immutable header is read here; concurrently published counters
    // require atomic loads and are not part of target selection.
    let mut header = [0u8;32]; status_file.read_exact(&mut header)?;
    let version = u32::from_le_bytes(header[4..8].try_into()?);
    require(&header[..4] == b"LVFS" && (1..=2).contains(&version)
        && header[8..16] == [0, 4, 0, 0, 0, 0, 0, 0]
        && header[16..32].iter().map(|b| format!("{b:02x}")).collect::<String>()
            == request.session.to_ascii_lowercase(), "scheduling_status_binding")?;
    if let Some((pid, process_start)) = request.native_peer {
        require(request.owned.is_empty() && pid > 0, "scheduling_native_peer")?;
        let caller = match request.native_role.as_deref() {
            None | Some("worker") => false,
            Some("caller") => true,
            Some(_) => return Err("scheduling_request".into()),
        };
        let path = request.status.parent().ok_or("scheduling_native_directory")?
            .join(if caller { "caller-scheduling.request" } else { "native-scheduling.request" });
        let mut file = linux_vst_bridge::file(&path)?;
        let meta = file.metadata()?;
        require(meta.uid() == uid && meta.mode() & 0o077 == 0 && meta.len() == 40,
            "scheduling_native_request_custody")?;
        let mut bytes = [0; 40]; file.read_exact(&mut bytes)?;
        require(&bytes[..4] == b"LVNS" && bytes[4..8] == 1u32.to_le_bytes()
            && bytes[8..24] == header[16..32], "scheduling_native_request_binding")?;
        let namespace_pid = u32::from_le_bytes(bytes[24..28].try_into()?);
        let namespace_tid = u32::from_le_bytes(bytes[28..32].try_into()?);
        let thread_start = u64::from_le_bytes(bytes[32..40].try_into()?);
        require(namespace_pid > 0 && namespace_tid > 0 && thread_start > 0, "scheduling_native_identity")?;
        let process = proc_root.join(pid.to_string());
        require(process.metadata()?.uid() == uid && start(&process, pid)? == process_start
            && namespace_id(&process)? == namespace_pid, "scheduling_native_peer_changed")?;
        require(map_inodes(&bounded(&process.join("maps"), 2 * 1024 * 1024)?, status.ino(), status.dev())?,
            "scheduling_native_session_mapping")?;
        let mut found = None;
        for (count, entry) in fs::read_dir(process.join("task"))?.enumerate() {
            require(count < 1024, "scheduling_task_extent")?;
            let task = entry?;
            if namespace_id(&task.path())? != namespace_tid { continue; }
            let tid: i32 = task.file_name().to_str().ok_or("scheduling_tid")?.parse()?;
            let comm = bounded(&task.path().join("comm"), 32)?;
            require(found.is_none() && task.metadata()?.uid() == uid
                && (caller || matches!(comm.as_slice(), b"ap3-transport\n" | b"ap6-transport\n"))
                && start(&task.path(), tid)? == thread_start, "scheduling_native_worker_changed")?;
            found = Some(Target { pid, process_start, tid, thread_start });
        }
        require(start(&process, pid)? == process_start, "scheduling_native_peer_changed")?;
        return found.ok_or_else(|| "scheduling_native_worker_absent".into());
    }
    #[cfg(feature = "pb0-c0-audit")]
    let mut observed = audit::Owned::new(request,&status,header);
    let mut targets = Vec::new();
    for &(pid, process_start) in &request.owned {
        if pid <= 0 { return Err("scheduling_owner_pid".into()); }
        let process = proc_root.join(pid.to_string());
        let candidate = (|| -> Result<()> {
            require(process.metadata()?.uid() == uid && start(&process, pid)? == process_start,
                "scheduling_owner_changed")?;
            let mut threads = Vec::new();
            for (count, entry) in fs::read_dir(process.join("task"))?.enumerate() {
                require(count < 1024, "scheduling_task_extent")?;
                let task = entry?;
                let tid: i32 = task.file_name().to_str().ok_or("scheduling_tid")?.parse()?;
                if bounded(&task.path().join("comm"), 32)? != b"lvb-audio\n" { continue; }
                require(task.metadata()?.uid() == uid, "scheduling_thread_owner")?;
                threads.push(Target { pid, process_start, tid, thread_start: start(&task.path(), tid)? });
            }
            if !threads.is_empty() && map_inodes(&bounded(&process.join("maps"), 2 * 1024 * 1024)?,
                status.ino(), status.dev())? { targets.extend(threads); }
            require(start(&process, pid)? == process_start, "scheduling_owner_changed")
        })();
        // An exited sibling is normal. Other malformed/denied observations
        // refuse this request rather than widening the process search.
        if let Err(error) = candidate {
            if error.downcast_ref::<std::io::Error>().is_some_and(|e|
                matches!(e.raw_os_error(), Some(libc::ENOENT | libc::ESRCH))) { continue; }
            return Err(error);
        }
    }
    targets.sort_by_key(|t| t.tid); targets.dedup();
    #[cfg(feature = "pb0-c0-audit")]
    if let Some(observed) = observed.as_mut() {observed.render_matches = Some(targets.len());}
    require(targets.len() == 1, "scheduling_unique_owned_render_thread")?;
    Ok(targets.remove(0))
}
fn namespace_id(proc: &Path) -> Result<u32> {
    let bytes = bounded(&proc.join("status"), 16384)?;
    let text = std::str::from_utf8(&bytes)?;
    let mut rows = text.lines().filter_map(|line| line.strip_prefix("NSpid:"));
    let row = rows.next().ok_or("scheduling_namespace_identity")?;
    require(rows.next().is_none(), "scheduling_namespace_identity")?;
    Ok(row.split_whitespace().last().ok_or("scheduling_namespace_identity")?.parse()?)
}
fn current(target: &Target) -> Result<Policy> {
    let process = PathBuf::from(format!("/proc/{}", target.pid));
    require(start(&process, target.pid)? == target.process_start
        && start(&process.join(format!("task/{}", target.tid)), target.tid)? == target.thread_start,
        "scheduling_target_changed")?;
    let mut param = libc::sched_param { sched_priority: 0 };
    let policy = unsafe { libc::sched_getscheduler(target.tid) };
    require(policy >= 0 && unsafe { libc::sched_getparam(target.tid, &mut param) } == 0,
        "scheduling_readback")?;
    Ok(Policy { policy, priority: param.sched_priority })
}
fn requested(policy: &Policy) -> bool {
    policy.policy == (libc::SCHED_RR | libc::SCHED_RESET_ON_FORK) && policy.priority == PRIORITY
}
fn ordinary(policy: &Policy) -> bool {
    policy.policy & !libc::SCHED_RESET_ON_FORK == libc::SCHED_OTHER && policy.priority == 0
}
fn fixed_reason(error: &dyn std::fmt::Display) -> String {
    let message = error.to_string();
    if message.starts_with("scheduling_") { message }
    else { "scheduling_capability_unavailable".into() }
}
#[derive(Clone, Default)]
struct RequestObservation {
    attempted: bool,
    accepted: Option<bool>,
    command_exited: bool,
    command_exit_code: Option<i32>,
    command_timed_out: bool,
    // The bus client's bounded first error line. A failed call that names its
    // D-Bus error is positive refusal evidence; a silent failure or timeout is not.
    refusal: Option<String>,
}
impl RequestObservation {
    fn refused(&self) -> Option<bool> { self.accepted.map(|accepted| !accepted) }
    fn unknown(&self) -> bool { self.attempted && self.accepted.is_none() }
}
const REFUSAL_CHARS: usize = 160;
/// One bounded printable line of the bus client's diagnostic output. The text
/// is a fixed-width observation for the record, never parsed for policy.
fn refusal_text(bytes: &[u8]) -> Option<String> {
    let line = bytes.split(|&b| b == b'\n').map(|l| l.trim_ascii()).find(|l| !l.is_empty())?;
    let mut text: String = line.iter().take(REFUSAL_CHARS)
        .map(|&b| if (0x20..0x7f).contains(&b) { b as char } else { '?' }).collect();
    if line.len() > REFUSAL_CHARS { text.push_str("..."); }
    Some(text)
}
fn unavailable_after_before(
    target: &Target,
    before: Policy,
    error: &dyn std::fmt::Display,
    request: RequestObservation,
    readback: impl FnOnce(&Target) -> Result<Policy>,
) -> serde_json::Value {
    let reason = fixed_reason(error);
    match readback(target) {
        Ok(effective) => serde_json::json!({"outcome":"unavailable","reason":reason,
            "target":target,"before":before,"effective":effective,"effective_readback":"observed",
            "request_attempted":request.attempted,"request_accepted":request.accepted,
            "request_refused":request.refused(),"request_unknown":request.unknown(),"request_refusal":request.refusal,
            "command_exited":request.command_exited,"command_exit_code":request.command_exit_code,
            "command_timed_out":request.command_timed_out}),
        Err(readback_error) => serde_json::json!({"outcome":"unavailable","reason":reason,
            "target":target,"before":before,"effective":null,"effective_readback":"unavailable",
            "effective_readback_reason":fixed_reason(readback_error.as_ref()),
            "request_attempted":request.attempted,"request_accepted":request.accepted,
            "request_refused":request.refused(),"request_unknown":request.unknown(),"request_refusal":request.refusal,
            "command_exited":request.command_exited,"command_exit_code":request.command_exit_code,
            "command_timed_out":request.command_timed_out}),
    }
}
fn apply(target: &Target, native: bool) -> Result<serde_json::Value> {
    let before = current(target)?;
    if requested(&before) { return Ok(serde_json::json!({"outcome":"already_effective","target":target,"effective":before,
        "request_attempted":false,"request_accepted":null,"request_refused":null,"request_unknown":false,"request_refusal":null,
        "command_exited":false,"command_exit_code":null,"command_timed_out":false})); }
    let mut request = RequestObservation::default();
    let attempt = (|| -> Result<serde_json::Value> {
        require(ordinary(&before), "scheduling_existing_policy_preserved")?;
        // Best effort capability: no privilege changes and no RT budget increase.
        let mut limits = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        require(unsafe { libc::prlimit(target.pid, libc::RLIMIT_RTTIME, std::ptr::null(), &mut limits) } == 0,
            "scheduling_limits_read")?;
        // A native worker belongs to the DAW process. Never change a DAW-wide limit.
        let bounded_limits = if native {
            require(limits.rlim_max > 0 && limits.rlim_max <= RTTIME_US, "scheduling_native_budget_unavailable")?;
            limits
        } else { libc::rlimit { rlim_cur: limits.rlim_cur.min(RTTIME_US), rlim_max: limits.rlim_max.min(RTTIME_US) } };
        current(target)?;
        if !native {
            require(unsafe { libc::prlimit(target.pid, libc::RLIMIT_RTTIME, &bounded_limits, std::ptr::null_mut()) } == 0,
                "scheduling_limits_set")?;
        }
        // RealtimeKit sets RESET_ON_FORK itself and checks membership of the
        // supplied thread in the supplied process. Do not preemptively change a
        // numeric TID's policy or silently downgrade an existing RT policy.
        current(target)?;
        request.attempted = true;
        let mut child = Command::new("/usr/bin/busctl")
            .args(["--system", "--timeout=1", "--allow-interactive-authorization=no", "call",
                "org.freedesktop.RealtimeKit1", "/org/freedesktop/RealtimeKit1", "org.freedesktop.RealtimeKit1",
                "MakeThreadRealtimeWithPID", "ttu"])
            .args([target.pid.to_string(), target.tid.to_string(), PRIORITY.to_string()])
            .env_remove("DBUS_SYSTEM_BUS_ADDRESS").stdin(Stdio::null())
            .stdout(Stdio::null()).stderr(Stdio::piped()).spawn()?;
        let deadline = Instant::now() + Duration::from_millis(1200);
        loop {
            if let Some(status) = child.try_wait()? {
                request.command_exited=true;
                request.command_exit_code=status.code();
                if status.success() { request.accepted=Some(true); }
                break;
            }
            if Instant::now() >= deadline {
                request.command_timed_out=true;
                let _ = child.kill();let _ = child.wait();break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        // The client has exited, so this bounded read cannot block on a live writer.
        if let Some(stderr) = child.stderr.take() {
            let mut bytes = Vec::new();
            if stderr.take(4096).read_to_end(&mut bytes).is_ok() { request.refusal = refusal_text(&bytes); }
        }
        if request.command_exited && request.accepted.is_none() && request.refusal.is_some() {
            request.accepted = Some(false);
        }
        let effective = current(target)?;
        Ok(serde_json::json!({"outcome":if request.accepted==Some(true) && requested(&effective) {"effective"} else {"unavailable"},
            "target":target,"before":before,"effective":effective,"requested_priority":PRIORITY,
            "rttime_soft_us":bounded_limits.rlim_cur,"rttime_hard_us":bounded_limits.rlim_max,
            "request_attempted":request.attempted,"request_accepted":request.accepted,
            "request_refused":request.refused(),"request_unknown":request.unknown(),"request_refusal":request.refusal,
            "command_exited":request.command_exited,"command_exit_code":request.command_exit_code,
            "command_timed_out":request.command_timed_out}))
    })();
    Ok(match attempt {
        Ok(value) => value,
        Err(error) => unavailable_after_before(target, before, error.as_ref(), request, current),
    })
}

pub fn run() -> Result<()> {
    let mut bytes = Vec::new(); std::io::stdin().take(32_769).read_to_end(&mut bytes)?;
    require(bytes.len() <= 32_768, "scheduling_request_extent")?;
    let result = (|| -> Result<_> {
        let request: Request = serde_json::from_slice(&bytes)?;
        let target = select(&request, Path::new("/proc"), unsafe { libc::getuid() })?;
        apply(&target, request.native_peer.is_some())
    })();
    // Fixed failure classes only: raw errors can contain private paths.
    let value = result.unwrap_or_else(|error| {
        let reason = fixed_reason(error.as_ref());
        serde_json::json!({"outcome":"unavailable","reason":reason,
            "request_attempted":false,"request_accepted":null,"request_refused":null,"request_unknown":false,"request_refusal":null,
            "command_exited":false,"command_exit_code":null,"command_timed_out":false})
    });
    println!("{}", serde_json::to_string(&value)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, sync::atomic::{AtomicUsize, Ordering}};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Fixture { root: PathBuf, request: Request }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("lvb-scheduling-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&root).unwrap();
            let status = root.join("ap12.status");
            let mut bytes = [0u8; 1024]; bytes[..4].copy_from_slice(b"LVFS");
            bytes[4..8].copy_from_slice(&2u32.to_le_bytes()); bytes[8..12].copy_from_slice(&1024u32.to_le_bytes());
            bytes[16..32].fill(0x42); fs::write(&status, bytes).unwrap();
            fs::set_permissions(&status, fs::Permissions::from_mode(0o600)).unwrap();
            Self { root, request: Request {schema:1,session:"42".repeat(16),status,owned:vec![(100,900)],native_peer:None,native_role:None} }
        }
        fn process(&self, pid: i32, tid: i32, mapped: bool) {
            let p = self.root.join(pid.to_string()); let task = p.join(format!("task/{tid}"));
            fs::create_dir_all(&task).unwrap();
            Self::stat(&p, pid, 900); Self::stat(&task, tid, 901);
            fs::write(task.join("comm"), b"lvb-audio\n").unwrap();
            let m = fs::metadata(&self.request.status).unwrap();
            let inode = if mapped {m.ino()} else {m.ino()+1};
            fs::write(p.join("maps"), format!("1000-2000 rw-s 00000000 {:x}:{:x} {inode} /alias/ap12.status\n",
                libc::major(m.dev()),libc::minor(m.dev()))).unwrap();
        }
        fn stat(p: &Path, id: i32, start: u64) {
            let mut fields = vec!["0".to_owned();20]; fields[0]="S".into(); fields[19]=start.to_string();
            fs::write(p.join("stat"),format!("{id} (name with ) parens) {}\n",fields.join(" "))).unwrap();
        }
        fn select(&self) -> Result<Target> { select(&self.request,&self.root,unsafe {libc::getuid()}) }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _=fs::remove_dir_all(&self.root); } }
    #[test]
    fn scheduling_requires_owned_process_and_exact_session_mapping() {
        let mut f=Fixture::new(); f.process(100,101,true); f.process(200,201,false); f.process(300,301,true);
        f.request.owned.push((200,900));
        assert_eq!(f.select().unwrap(),Target {pid:100,process_start:900,tid:101,thread_start:901});
        // A matching thread outside the supervisor's custody must be ignored.
        f.request.owned=vec![(200,900)]; assert!(f.select().is_err());
    }
    #[test]
    fn scheduling_refuses_ambiguous_and_recycled_owners() {
        let mut f=Fixture::new();f.process(100,101,true);f.process(200,201,true);
        f.request.owned.push((200,900)); assert!(f.select().is_err());
        f.request.owned.pop(); Fixture::stat(&f.root.join("100"),100,902); assert!(f.select().is_err());
    }
    #[test]
    fn scheduling_refuses_another_session_or_public_status() {
        let mut f=Fixture::new();f.process(100,101,true);
        f.request.session="43".repeat(16);assert!(f.select().is_err());
        f.request.session="42".repeat(16);
        fs::set_permissions(&f.request.status,fs::Permissions::from_mode(0o644)).unwrap();assert!(f.select().is_err());
    }
    #[cfg(feature = "pb0-c0-audit")]
    fn private_audit_fixture() -> Fixture {
        let f = Fixture::new();
        fs::set_permissions(&f.root,fs::Permissions::from_mode(0o700)).unwrap();
        f
    }
    #[cfg(feature = "pb0-c0-audit")]
    fn scheduling_audit(f: &Fixture) -> serde_json::Value {
        linux_vst_bridge::read_json(&f.root.join("windows-scheduling.audit.json")).unwrap()
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_owned_cohort_is_exact_private_bound_and_selection_result_is_unchanged() {
        let mut f = private_audit_fixture();
        f.process(100,101,true);f.process(200,201,false);f.process(300,301,true);
        f.request.owned.push((200,900));
        assert_eq!(f.select().unwrap(),Target {pid:100,process_start:900,tid:101,thread_start:901});
        let observed = scheduling_audit(&f);
        assert_eq!(observed.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["basis","clock","monotonic_ns","owned","render_matches","request_schema","schema","session",
                "status_device","status_extent","status_inode","status_version"]);
        assert_eq!(observed["owned"],serde_json::json!([[100,900],[200,900]]));
        assert_eq!(observed["render_matches"],1);
        assert_eq!(observed["basis"],"supervisor_supplied_cohort");
        assert_eq!(observed["session"],f.request.session);
        let status = fs::metadata(&f.request.status).unwrap();
        assert_eq!(observed["status_device"],status.dev());assert_eq!(observed["status_inode"],status.ino());
        assert_eq!(observed["status_extent"],1024);assert_eq!(observed["status_version"],2);
        assert!(observed["monotonic_ns"].as_u64().is_some_and(|value|value > 0));
        let path = f.root.join("windows-scheduling.audit.json");
        assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777,0o600);
        assert!(!fs::read_to_string(path).unwrap().contains(f.root.to_str().unwrap()));
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_render_zero_multiple_and_incomplete_are_distinct_without_widening_custody() {
        let mut f = private_audit_fixture();f.process(100,101,false);f.process(300,301,true);
        assert_eq!(f.select().unwrap_err().to_string(),"scheduling_unique_owned_render_thread");
        assert_eq!(scheduling_audit(&f)["render_matches"],0);
        f.process(100,101,true);f.process(200,201,true);f.request.owned.push((200,900));
        assert_eq!(f.select().unwrap_err().to_string(),"scheduling_unique_owned_render_thread");
        assert_eq!(scheduling_audit(&f)["render_matches"],2);
        Fixture::stat(&f.root.join("100"),100,902);
        assert_eq!(f.select().unwrap_err().to_string(),"scheduling_owner_changed");
        assert!(scheduling_audit(&f)["render_matches"].is_null());
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_refuses_status_drift_and_export_error_cannot_change_selection() {
        let mut f = private_audit_fixture();f.process(100,101,true);
        f.request.session = "43".repeat(16);
        assert_eq!(f.select().unwrap_err().to_string(),"scheduling_status_binding");
        assert!(!f.root.join("windows-scheduling.audit.json").exists());
        f.request.session = "42".repeat(16);
        f.request.owned.resize(MAX_OWNERS+1,(100,900));
        assert_eq!(f.select().unwrap_err().to_string(),"scheduling_request");
        assert!(!f.root.join("windows-scheduling.audit.json").exists());
        f.request.owned.truncate(1);
        fs::create_dir(f.root.join("windows-scheduling.audit.json")).unwrap();
        assert_eq!(f.select().unwrap().tid,101);
        assert!(f.root.join("windows-scheduling.audit.json").is_dir());
        assert!(fs::read_dir(&f.root).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().contains(".tmp-")));
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_native_request_cannot_overwrite_the_windows_cohort() {
        let mut f = private_audit_fixture();f.process(100,101,true);f.select().unwrap();
        let path = f.root.join("windows-scheduling.audit.json");
        let before = fs::read(&path).unwrap();
        f.request.schema = 2;f.request.native_peer = Some((100,900));f.request.owned.clear();
        assert!(f.select().is_err()); // Missing native startup record is still refused.
        assert_eq!(fs::read(path).unwrap(),before);
    }
    #[cfg(feature = "pb0-c0-audit")]
    #[test]
    fn audit_write_cannot_follow_replacement_directory_symlink_or_status() {
        let f = private_audit_fixture();
        let header = [b"LVFS".as_slice(),&2u32.to_le_bytes(),&[0,4,0,0,0,0,0,0],&[0x42;16]].concat();
        let observed = audit::Owned::new(&f.request,&fs::metadata(&f.request.status).unwrap(),header.clone().try_into().unwrap()).unwrap();
        let retained = f.root.with_extension("retained");
        fs::rename(&f.root,&retained).unwrap();fs::create_dir(&f.root).unwrap();
        drop(observed);
        assert!(!f.root.join("windows-scheduling.audit.json").exists());
        assert!(!retained.join("windows-scheduling.audit.json").exists());
        fs::remove_dir(&f.root).unwrap();std::os::unix::fs::symlink(&retained,&f.root).unwrap();
        assert!(audit::Owned::new(&f.request,&fs::metadata(&f.request.status).unwrap(),header.try_into().unwrap()).is_none());
        fs::remove_file(&f.root).unwrap();fs::rename(&retained,&f.root).unwrap();
        let mut status = linux_vst_bridge::file(&f.request.status).unwrap();
        let mut header = [0;32];status.read_exact(&mut header).unwrap();
        let observed = audit::Owned::new(&f.request,&status.metadata().unwrap(),header).unwrap();
        fs::rename(&f.request.status,f.root.join("old.status")).unwrap();
        fs::copy(f.root.join("old.status"),&f.request.status).unwrap();
        drop(observed);
        assert!(!f.root.join("windows-scheduling.audit.json").exists());
    }
    #[cfg(not(feature = "pb0-c0-audit"))]
    #[test]
    fn audit_disabled_selection_never_exports_a_cohort() {
        let f = Fixture::new();f.process(100,101,true);assert_eq!(f.select().unwrap().tid,101);
        assert!(!f.root.join("windows-scheduling.audit.json").exists());
    }
    #[test]
    fn refusal_text_is_one_bounded_printable_line_and_failure_evidence_is_explicit() {
        assert_eq!(refusal_text(b""),None);
        assert_eq!(refusal_text(b"\n  \n"),None);
        assert_eq!(refusal_text(b"\nCall failed: Access denied\nsecond line\n").as_deref(),Some("Call failed: Access denied"));
        assert_eq!(refusal_text(b"tab\there \xff\x01end").as_deref(),Some("tab?here ??end"));
        let long=vec![b'x';REFUSAL_CHARS+40];
        let text=refusal_text(&long).unwrap();
        assert_eq!(text.len(),REFUSAL_CHARS+3);assert!(text.ends_with("..."));
        let mut request=RequestObservation {attempted:true,..RequestObservation::default()};
        assert_eq!((request.refused(),request.unknown()),(None,true));
        request.accepted=Some(false);request.refusal=Some("Call failed: Access denied".into());
        assert_eq!((request.refused(),request.unknown()),(Some(true),false));
        request.accepted=Some(true);
        assert_eq!((request.refused(),request.unknown()),(Some(false),false));
    }
    #[test]
    fn scheduling_preserves_existing_policy_and_requires_effective_reset_flag() {
        assert!(ordinary(&Policy{policy:libc::SCHED_OTHER,priority:0}));
        assert!(!ordinary(&Policy{policy:libc::SCHED_FIFO,priority:20}));
        assert!(!ordinary(&Policy{policy:libc::SCHED_RR|libc::SCHED_RESET_ON_FORK,priority:20}));
        assert!(!requested(&Policy{policy:libc::SCHED_RR,priority:5}));
        assert!(!requested(&Policy{policy:libc::SCHED_OTHER,priority:0}));
        assert!(requested(&Policy{policy:libc::SCHED_RR|libc::SCHED_RESET_ON_FORK,priority:5}));
    }
    #[test]
    fn scheduling_error_retains_before_and_fresh_identity_checked_effective_policy() {
        let target=Target {pid:10,process_start:20,tid:30,thread_start:40};
        let before=Policy {policy:libc::SCHED_OTHER,priority:0};
        let effective=Policy {policy:libc::SCHED_RR|libc::SCHED_RESET_ON_FORK,priority:PRIORITY};
        let calls=AtomicUsize::new(0);
        let value=unavailable_after_before(&target,before.clone(),
            &"scheduling_native_budget_unavailable",RequestObservation::default(),|observed| {
                calls.fetch_add(1,Ordering::Relaxed);
                assert_eq!(observed,&target);
                Ok(effective.clone())
            });
        assert_eq!(calls.load(Ordering::Relaxed),1);
        assert_eq!(value["outcome"],"unavailable");
        assert_eq!(value["reason"],"scheduling_native_budget_unavailable");
        assert_eq!(value["target"],serde_json::to_value(&target).unwrap());
        assert_eq!(value["before"],serde_json::to_value(&before).unwrap());
        assert_eq!(value["effective"],serde_json::to_value(&effective).unwrap());
        assert_eq!(value["effective_readback"],"observed");
        assert_eq!(value["request_attempted"],false);
        assert!(value["request_accepted"].is_null());
        assert!(value["request_refused"].is_null());
        assert_eq!(value["request_unknown"],false);
    }
    #[test]
    fn scheduling_error_retains_before_and_null_when_fresh_identity_readback_refuses() {
        let target=Target {pid:10,process_start:20,tid:30,thread_start:40};
        let before=Policy {policy:libc::SCHED_OTHER,priority:0};
        let calls=AtomicUsize::new(0);
        let request=RequestObservation {attempted:true,..Default::default()};
        let value=unavailable_after_before(&target,before.clone(),&std::io::Error::other("private path"),request,|_| {
            calls.fetch_add(1,Ordering::Relaxed);
            Err("scheduling_target_changed".into())
        });
        assert_eq!(calls.load(Ordering::Relaxed),1);
        assert_eq!(value["outcome"],"unavailable");
        assert_eq!(value["reason"],"scheduling_capability_unavailable");
        assert_eq!(value["before"],serde_json::to_value(&before).unwrap());
        assert!(value["effective"].is_null());
        assert_eq!(value["effective_readback"],"unavailable");
        assert_eq!(value["effective_readback_reason"],"scheduling_target_changed");
        assert_eq!(value["request_attempted"],true);
        assert!(value["request_accepted"].is_null());
        assert!(value["request_refused"].is_null());
        assert_eq!(value["request_unknown"],true);
        let nonzero=unavailable_after_before(&target,before.clone(),&"scheduling_capability_unavailable",
            RequestObservation {attempted:true,command_exited:true,command_exit_code:Some(1),
                ..Default::default()},|_|Ok(Policy {policy:0,priority:0}));
        assert!(nonzero["request_accepted"].is_null());
        assert!(nonzero["request_refused"].is_null());
        assert_eq!(nonzero["request_unknown"],true);
        assert_eq!(nonzero["command_exited"],true);
        assert_eq!(nonzero["command_exit_code"],1);
        assert_eq!(nonzero["command_timed_out"],false);
        let accepted=unavailable_after_before(&target,before,&"scheduling_capability_unavailable",
            RequestObservation {attempted:true,accepted:Some(true),command_exited:true,
                command_exit_code:Some(0),command_timed_out:false,refusal:None},|_|Ok(Policy {policy:0,priority:0}));
        assert_eq!(accepted["request_accepted"],true);
        assert_eq!(accepted["request_refused"],false);
        assert_eq!(accepted["request_unknown"],false);
    }
    #[test]
    fn native_worker_requires_socket_peer_namespace_generation_and_session_mapping() {
        let mut f=Fixture::new();f.process(100,101,true);f.process(100,102,true);
        f.request.schema=2;f.request.owned.clear();f.request.native_peer=Some((100,900));
        let process=f.root.join("100");
        fs::write(process.join("status"), "NSpid:\t100\t40\n").unwrap();
        for (tid,nsid) in [(101,41),(102,42)] {
            let task=process.join(format!("task/{tid}"));
            fs::write(task.join("status"),format!("NSpid:\t{tid}\t{nsid}\n")).unwrap();
            fs::write(task.join("comm"),b"ap3-transport\n").unwrap();
        }
        let file=f.root.join("native-scheduling.request");
        let mut bytes=[b"LVNS".as_slice(),&1u32.to_le_bytes(),&[0x42;16],
            &40u32.to_le_bytes(),&42u32.to_le_bytes(),&901u64.to_le_bytes()].concat();
        fs::write(&file,&bytes).unwrap();fs::set_permissions(&file,fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(f.select().unwrap(),Target{pid:100,process_start:900,tid:102,thread_start:901});
        // Two identical thread names in one DAW: only the namespace ID granted
        // through this session's startup record selects the target.
        bytes[28..32].copy_from_slice(&41u32.to_le_bytes());fs::write(&file,&bytes).unwrap();
        assert_eq!(f.select().unwrap().tid,101);
        bytes[32..40].copy_from_slice(&902u64.to_le_bytes());fs::write(&file,&bytes).unwrap();
        assert!(f.select().is_err());
        bytes[32..40].copy_from_slice(&901u64.to_le_bytes());bytes[8]^=1;fs::write(&file,&bytes).unwrap();
        assert!(f.select().is_err());
        bytes[8]^=1;fs::write(&file,&bytes).unwrap();
        f.request.native_peer=Some((100,902));assert!(f.select().is_err());
        f.request.native_peer=Some((100,900));fs::write(process.join("maps"),b"").unwrap();assert!(f.select().is_err());
    }
    #[test]
    fn native_caller_role_reads_its_own_request_and_selects_any_thread_name() {
        let mut f=Fixture::new();f.process(100,101,true);
        f.request.schema=2;f.request.owned.clear();f.request.native_peer=Some((100,900));
        let process=f.root.join("100");
        fs::write(process.join("status"), "NSpid:\t100\t40\n").unwrap();
        let task=process.join("task/101");
        fs::write(task.join("status"),"NSpid:\t101\t41\n").unwrap();
        fs::write(task.join("comm"),b"BitwigPluginHos\n").unwrap();
        let bytes=[b"LVNS".as_slice(),&1u32.to_le_bytes(),&[0x42;16],
            &40u32.to_le_bytes(),&41u32.to_le_bytes(),&901u64.to_le_bytes()].concat();
        let write=|name:&str|{let file=f.root.join(name);fs::write(&file,&bytes).unwrap();
            fs::set_permissions(&file,fs::Permissions::from_mode(0o600)).unwrap();};
        // The worker role still requires the transport thread name and its own file.
        write("native-scheduling.request");
        assert!(f.select().is_err());
        f.request.native_role=Some("caller".into());
        assert!(f.select().is_err()); // no caller request file yet
        write("caller-scheduling.request");
        assert_eq!(f.select().unwrap(),Target{pid:100,process_start:900,tid:101,thread_start:901});
        f.request.native_role=Some("worker".into());assert!(f.select().is_err());
        f.request.native_role=Some("other".into());assert!(f.select().is_err());
    }
}

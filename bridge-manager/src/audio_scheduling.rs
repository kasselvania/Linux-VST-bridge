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
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Target { pid: i32, process_start: u64, tid: i32, thread_start: u64 }
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Policy { policy: i32, priority: i32 }

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
        let path = request.status.parent().ok_or("scheduling_native_directory")?.join("native-scheduling.request");
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
                && matches!(comm.as_slice(), b"ap3-transport\n" | b"ap6-transport\n")
                && start(&task.path(), tid)? == thread_start, "scheduling_native_worker_changed")?;
            found = Some(Target { pid, process_start, tid, thread_start });
        }
        require(start(&process, pid)? == process_start, "scheduling_native_peer_changed")?;
        return found.ok_or_else(|| "scheduling_native_worker_absent".into());
    }
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
fn apply(target: &Target, native: bool) -> Result<serde_json::Value> {
    let before = current(target)?;
    if requested(&before) { return Ok(serde_json::json!({"outcome":"already_effective","target":target,"effective":before})); }
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
    let mut child = Command::new("/usr/bin/busctl")
        .args(["--system", "--timeout=1", "--allow-interactive-authorization=no", "call",
            "org.freedesktop.RealtimeKit1", "/org/freedesktop/RealtimeKit1", "org.freedesktop.RealtimeKit1",
            "MakeThreadRealtimeWithPID", "ttu"])
        .args([target.pid.to_string(), target.tid.to_string(), PRIORITY.to_string()])
        .env_remove("DBUS_SYSTEM_BUS_ADDRESS").stdin(Stdio::null())
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn()?;
    let deadline = Instant::now() + Duration::from_millis(1200);
    let accepted = loop {
        if let Some(status) = child.try_wait()? { break status.success(); }
        if Instant::now() >= deadline { let _ = child.kill(); let _ = child.wait(); break false; }
        std::thread::sleep(Duration::from_millis(5));
    };
    let effective = current(target)?;
    Ok(serde_json::json!({"outcome":if accepted && requested(&effective) {"effective"} else {"unavailable"},
        "target":target,"before":before,"effective":effective,"requested_priority":PRIORITY,
        "rttime_soft_us":bounded_limits.rlim_cur,"rttime_hard_us":bounded_limits.rlim_max}))
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
        let message = error.to_string();
        let reason = if message.starts_with("scheduling_") { message }
            else { "scheduling_capability_unavailable".into() };
        serde_json::json!({"outcome":"unavailable","reason":reason})
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
            Self { root, request: Request {schema:1,session:"42".repeat(16),status,owned:vec![(100,900)],native_peer:None} }
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
}

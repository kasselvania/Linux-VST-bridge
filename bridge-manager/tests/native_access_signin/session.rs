//! A managed Native Access session and browser returns sent to it.
//!
//! `Session::open` submits the session as the window's Open does
//! (`vendor_cli::run_selected`): it verifies the selection, observes the
//! profile capabilities, writes the same launch records and starts the unit
//! from the same `vendor_application::session_command`. Everything after
//! that is the product. Returns go through the real URL handler.
use crate::setup::Setup;
use linux_vst_bridge::{
    vendor_application::{self, Application, LaunchMode, Operation, OperationResult},
    *,
};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    os::unix::net::{SocketAddr, UnixStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// From submission to a bound app, a ready service and an open return
/// listener. A first open also installs the bundled service.
const READY: Duration = Duration::from_secs(240);
/// The handler waits at most 25 s for the listener and 70 s for the result.
const HANDLER: Duration = Duration::from_secs(120);
/// `systemctl stop` allows 90 s (TimeoutStopSec); the record follows.
const RETIREMENT: Duration = Duration::from_secs(120);
/// The supervisor publishes its record every half second.
const PUBLICATION: Duration = Duration::from_secs(5);

pub fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
}

pub struct Session<'a> {
    setup: &'a Setup,
    pub operation: String,
    pub unit: String,
    stopped: bool,
}

/// One run of `linux-vst-bridge native-access-callback URL`.
pub struct Return {
    pub success: bool,
    pub stderr: Vec<u8>,
    pub stdout: Vec<u8>,
    pub started_ms: u64,
    /// When the handler reached the supervisor's socket, after its checks.
    /// Sampled every 5 ms, so a connection closed within one sample can be
    /// missed; the supervisor's own record catches that (see main.rs).
    pub connected_ms: Option<u64>,
    pub exited_ms: u64,
    pub elapsed: Duration,
    /// Read from /proc while the handler ran, once it had protected itself.
    pub handler: Option<HandlerFacts>,
}
#[derive(Debug)]
pub struct HandlerFacts {
    pub core_limit_zero: bool,
    /// Not dumpable: /proc/PID/environ is no longer readable by its own user.
    pub environment_unreadable: bool,
}

impl Setup {
    pub fn unit(&self) -> String {
        format!("linux-vst-bridge-vendor-{}.service", self.selector)
    }

    /// Run the URL handler as the browser's desktop entry does.
    pub fn browser_return(&self, url: &str, socket: Option<&str>) -> Return {
        let started = Instant::now();
        let started_ms = unix_ms();
        let mut child = Command::new(&self.software.manager.path)
            .arg("native-access-callback").arg(url)
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().unwrap();
        let pid = child.id();
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let out = std::thread::spawn(move || { let mut v = vec![]; stdout.read_to_end(&mut v).unwrap(); v });
        let err = std::thread::spawn(move || { let mut v = vec![]; stderr.read_to_end(&mut v).unwrap(); v });
        let baseline = socket.map(socket_entries);
        let (mut connected_ms, mut handler) = (None, None);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() { break status; }
            assert!(started.elapsed() < HANDLER, "the URL handler did not finish within {HANDLER:?}");
            if handler.is_none() { handler = handler_facts(pid); }
            if let (None, Some(name), Some(before)) = (connected_ms, socket, baseline) {
                if socket_entries(name) > before { connected_ms = Some(unix_ms()); }
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        Return { success: status.success(), stdout: out.join().unwrap(), stderr: err.join().unwrap(),
            started_ms, connected_ms, exited_ms: unix_ms(), elapsed: started.elapsed(), handler }
    }

    pub fn received(&self) -> Vec<Received> {
        self.stand_in_lines("received.log").iter().map(|line| Received::parse(line)).collect()
    }
    /// `secondary start` and `secondary end` lines at or after `since_ms`.
    pub fn secondary_events(&self, since_ms: u64) -> Vec<(String, u64)> {
        self.stand_in_lines("secondary.log").iter().filter_map(|line| {
            let kind = if line.starts_with("secondary start") { "start" }
                else if line.starts_with("secondary end") { "end" } else { return None };
            let at = field(line, "at_ms")?.parse().ok()?;
            (at >= since_ms).then(|| (format!("{kind} {}", field(line, "code").unwrap_or_default()).trim().to_owned(), at))
        }).collect()
    }
}

/// One `received` line from the stand-in's first copy.
#[derive(Debug)]
pub struct Received {
    pub args: u32,
    pub gpu_flag: u32,
    pub urls: u32,
    pub other: u32,
    pub url_sha256: String,
    pub url_bytes: usize,
    pub at_ms: u64,
}
impl Received {
    fn parse(line: &str) -> Self {
        let number = |name: &str| field(line, name).unwrap_or_else(|| panic!("{name} missing in {line}"));
        assert!(line.starts_with("received "), "malformed hand-over: {line}");
        Self { args: number("args").parse().unwrap(), gpu_flag: number("gpu_flag").parse().unwrap(),
            urls: number("urls").parse().unwrap(), other: number("other").parse().unwrap(),
            url_sha256: number("url_sha256").into(), url_bytes: number("url_bytes").parse().unwrap(),
            at_ms: number("at_ms").parse().unwrap() }
    }
}
fn field<'l>(line: &'l str, name: &str) -> Option<&'l str> {
    line.split_whitespace().find_map(|part| part.strip_prefix(name)?.strip_prefix('='))
}

impl<'a> Session<'a> {
    /// The window's Open, as `vendor_cli::run_selected("launch")` performs it.
    pub fn open(setup: &'a Setup) -> Self {
        let m = &setup.m;
        let directory = &setup.directory;
        let unit = setup.unit();
        let app: Application = read_json(&directory.join("application.json")).unwrap();
        app.verify(&m.root).unwrap();
        let inspected = vendor_profile::inspect(&app, &m.root).unwrap();
        let capabilities = inspected.capabilities.unwrap_or_else(|| panic!(
            "the bundled profile did not recognise the stand-in: {:?}", inspected.missing_capability));
        let software = preparation::build::launch_software(m).unwrap();
        let managed = vendor_profile::ManagedLaunch::new(capabilities, software.clone()).unwrap();
        let job = directory.join("operation.json");
        let report = directory.join("operation-result.json");
        let prior = job.exists();
        if prior {
            let old: OperationResult = read_json(&report).unwrap();
            assert!(old.retired(), "the previous session is not retired: {old:?}");
            let history = directory.join(format!("retired-{}", random_id().unwrap()));
            private_dir(&history).unwrap();
            fs::rename(&job, history.join("operation.json")).unwrap();
            fs::rename(&report, history.join("result.json")).unwrap();
        }
        let operation = random_id().unwrap();
        atomic_json(&directory.join(vendor_profile::LAUNCH_SETTINGS),
            &vendor_profile::launch_settings(&managed.capabilities.profile, &operation).unwrap()).unwrap();
        atomic_json(&job, &Operation { application: app, report: report.clone(), mode: LaunchMode::Normal,
            operation_id: operation.clone(), launcher: software.installer_launch.clone(),
            managed: Some(managed), capability_notice: inspected.missing_capability }).unwrap();
        atomic_json(&report, &json!({"schema":3,"operation_id":operation,"state":"unknown","launcher_exit":null,
            "owned_live":null,"cleanup_confirmed":false,"error":null,"discarded_diagnostic_bytes":0,
            "account_posture":"unknown"})).unwrap();
        if prior {
            let _ = Command::new("systemctl").args(["--user", "reset-failed", &unit]).stderr(Stdio::null()).status();
        }
        let status = vendor_application::session_command(&unit, true, &software, &job).unwrap().status().unwrap();
        assert!(status.success(), "systemd-run refused the session unit: {status}");
        Self { setup, operation, unit, stopped: false }
    }

    pub fn setup(&self) -> &'a Setup { self.setup }
    /// The supervisor's own record of this session (managed-session.json).
    pub fn record(&self) -> Value {
        let value: Value = read_json(&self.setup.directory.join("managed-session.json")).unwrap_or(Value::Null);
        if value["operation"] == self.operation { value } else { Value::Null }
    }
    /// The product's result for this operation (operation-result.json). The
    /// supervisor replaces the file by rename while the session runs, and a
    /// read that overlaps a replacement is refused; it is read again.
    pub fn result(&self) -> Value {
        let path = self.setup.directory.join("operation-result.json");
        for _ in 0..20 {
            match read_json(&path) {
                Err(error) if error.to_string() == "control_record_changed_during_read" =>
                    std::thread::sleep(Duration::from_millis(10)),
                read => return read.unwrap(),
            }
        }
        panic!("operation-result.json changed during every one of 20 reads");
    }
    pub fn socket_name(&self) -> String {
        format!("lvb-native-access-{}-{}", unsafe { libc::getuid() }, self.operation)
    }

    /// Wait until the app is bound, its service is ready, the return listener
    /// is open and the stand-in's first copy has its single-instance window.
    pub fn wait_ready(&self) -> Duration {
        let started = Instant::now();
        let ready_before = self.primaries();
        loop {
            let record = self.record();
            if record["state"] == "running" && record["effective"]["operation"] == self.operation
                && record["renderer"]["launch_binding"]["status"] == "bound"
                && record["dependency"]["ready_tested"] == true && record["browser_return"].is_object()
                && self.primaries() > ready_before && socket_entries(&self.socket_name()) > 0 {
                return started.elapsed();
            }
            let result = self.result();
            assert!(!matches!(result["state"].as_str(), Some("completed" | "failed" | "cancelled" | "cleanup_unconfirmed")),
                "the session ended before it was ready: {} {}", result["state"], result["error_code"]);
            assert!(started.elapsed() < READY, "not ready within {READY:?}: {:#}", summary(&record));
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    fn primaries(&self) -> usize {
        self.setup.stand_in_lines("primary.log").iter().filter(|l| l.starts_with("primary ready")).count()
    }

    pub fn browser_return(&self, url: &str) -> Return {
        self.setup.browser_return(url, Some(&self.socket_name()))
    }

    /// The published `browser_return` once it shows these counts.
    pub fn wait_browser_return(&self, attempts: u64, dispatched: u64, last: &[&str]) -> Value {
        let started = Instant::now();
        loop {
            let value = self.record()["browser_return"].clone();
            if value["attempts"] == attempts && value["dispatched"] == dispatched
                && last.iter().any(|result| value["last_result"] == *result) {
                return value;
            }
            assert!(started.elapsed() < PUBLICATION,
                "expected attempts={attempts} dispatched={dispatched} last_result in {last:?}, got {value}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Processes in the session's own unit cgroup.
    pub fn members(&self) -> Vec<u32> {
        let output = Command::new("systemctl").args(["--user", "show", "-p", "ControlGroup", "--value", &self.unit])
            .output().unwrap();
        let group = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if group.is_empty() { return vec![]; }
        let mut pids = vec![];
        let mut pending = vec![PathBuf::from("/sys/fs/cgroup").join(group.trim_start_matches('/'))];
        while let Some(directory) = pending.pop() {
            let Ok(text) = fs::read_to_string(directory.join("cgroup.procs")) else { continue };
            pids.extend(text.split_whitespace().filter_map(|pid| pid.parse::<u32>().ok()));
            for entry in fs::read_dir(&directory).into_iter().flatten().flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) { pending.push(entry.path()); }
            }
        }
        pids
    }
    /// Unit members with the stand-in's app file mapped: copies of the app.
    pub fn app_copies(&self) -> usize {
        let image = self.setup.app_executable().display().to_string();
        self.members().into_iter().filter(|pid| fs::read_to_string(format!("/proc/{pid}/maps"))
            .is_ok_and(|maps| maps.lines().any(|line| line.ends_with(&image)))).count()
    }
    /// Every unit member may not write a core file.
    pub fn core_limits(&self) -> (usize, Vec<u32>) {
        let members = self.members();
        let open = members.iter().copied().filter(|pid| core_limit_zero(*pid) == Some(false)).collect();
        (members.len(), open)
    }

    /// The window's Stop: `systemctl --user --no-block stop`, then the record.
    pub fn stop(&mut self) -> (Value, Duration) {
        let started = Instant::now();
        let status = Command::new("systemctl").args(["--user", "--no-block", "stop", &self.unit]).status().unwrap();
        assert!(status.success(), "systemctl stop: {status}");
        self.stopped = true;
        (self.wait_ended(), started.elapsed())
    }
    /// The unit has gone and the product's result for this operation is final.
    pub fn wait_ended(&mut self) -> Value {
        let started = Instant::now();
        loop {
            let result = self.result();
            let terminal = result["operation_id"] == self.operation && matches!(result["state"].as_str(),
                Some("completed" | "failed" | "cancelled" | "cleanup_unconfirmed"));
            if terminal && !unit_present(&self.unit) {
                self.stopped = true;
                return result;
            }
            assert!(started.elapsed() < RETIREMENT, "the session did not retire within {RETIREMENT:?}: {}",
                result["state"]);
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Talk to the return listener directly, as any same-user program can,
    /// and send one frame; returns the supervisor's one-byte answer.
    pub fn raw_return(&self, frame: &[u8]) -> Option<u8> {
        let mut peer = UnixStream::connect_addr(&abstract_address(&self.socket_name())).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        peer.write_all(self.operation.as_bytes()).unwrap();
        let mut ready = [0; 4];
        peer.read_exact(&mut ready).ok()?;
        assert_eq!(&ready, b"NAC1");
        peer.write_all(&(frame.len() as u32).to_le_bytes()).unwrap();
        peer.write_all(frame).unwrap();
        let mut result = [0; 1];
        peer.read_exact(&mut result).ok().map(|_| result[0])
    }
}
impl Drop for Session<'_> {
    fn drop(&mut self) {
        // A failed assertion must not leave the app running in the account.
        if !self.stopped {
            let _ = Command::new("systemctl").args(["--user", "--no-block", "stop", &self.unit]).status();
        }
    }
}

fn summary(record: &Value) -> Value {
    json!({"state":record["state"],"error":record["error"],"effective":record["effective"].is_object(),
        "binding":record["renderer"]["launch_binding"]["status"],
        "dependency_ready":record["dependency"]["ready_tested"],"browser_return":record["browser_return"]})
}

pub fn unit_present(unit: &str) -> bool {
    let output = Command::new("systemctl").args(["--user", "show", "-p", "LoadState,ActiveState", unit])
        .output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    !(text.contains("LoadState=not-found") || text.contains("ActiveState=inactive") || text.contains("ActiveState=failed"))
}

fn abstract_address(name: &str) -> SocketAddr {
    use std::os::linux::net::SocketAddrExt;
    SocketAddr::from_abstract_name(name).unwrap()
}
/// Abstract sockets bound to this name: the listener plus queued connections.
pub fn socket_entries(name: &str) -> usize {
    let wanted = format!("@{name}");
    fs::read_to_string("/proc/net/unix").unwrap().lines()
        .filter(|line| line.split_whitespace().nth(7) == Some(wanted.as_str())).count()
}

fn core_limit_zero(pid: u32) -> Option<bool> {
    let limits = fs::read_to_string(format!("/proc/{pid}/limits")).ok()?;
    let line = limits.lines().find(|line| line.starts_with("Max core file size"))?;
    let fields: Vec<_> = line["Max core file size".len()..].split_whitespace().collect();
    Some(fields.first() == Some(&"0") && fields.get(1) == Some(&"0"))
}
/// Facts only once the handler has made itself non-dumpable; it does so first.
fn handler_facts(pid: u32) -> Option<HandlerFacts> {
    let environment = fs::read(format!("/proc/{pid}/environ"));
    let unreadable = matches!(&environment, Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied);
    if !unreadable { return None; }
    Some(HandlerFacts { core_limit_zero: core_limit_zero(pid)?, environment_unreadable: true })
}

/// Processes anywhere that run in this prefix, as the dependency census finds them.
pub fn prefix_processes(prefix: &Path) -> Vec<u32> {
    let wanted = format!("WINEPREFIX={}", prefix.display());
    fs::read_dir("/proc").unwrap().flatten().filter_map(|entry| {
        let pid: u32 = entry.file_name().to_str()?.parse().ok()?;
        let environment = fs::read(entry.path().join("environ")).ok()?;
        environment.split(|byte| *byte == 0).any(|item| item == wanted.as_bytes()).then_some(pid)
    }).collect()
}
/// Listening TCP sockets on these ports, IPv4 or IPv6, from any program.
pub fn listeners(ports: &[u16]) -> usize {
    ["/proc/net/tcp", "/proc/net/tcp6"].iter().filter_map(|table| fs::read_to_string(table).ok())
        .flat_map(|text| text.lines().skip(1).map(str::to_owned).collect::<Vec<_>>())
        .filter(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            let port = fields.get(1).and_then(|local| local.rsplit(':').next())
                .and_then(|hex| u16::from_str_radix(hex, 16).ok());
            fields.get(3) == Some(&"0A") && port.is_some_and(|port| ports.contains(&port))
        }).count()
}

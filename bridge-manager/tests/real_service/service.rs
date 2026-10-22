//! Harness: a fixture state root served by the real `linux-vst-bridge serve`.
//!
//! The manager resolves its state from `HOME`, so each fixture is its own
//! home (`Fixture::installed`). The service binary is the one Cargo built for
//! this test, installed into the fixture's package generation exactly as the
//! product records it (read-only, beside a frontend, supervisor and ownership
//! module). Nothing in the product is switched into a test mode.
//!
//! Running without systemd. `serve` itself needs no unit: it is started here
//! as an ordinary child in its own process group, and stopped with SIGTERM to
//! that group, which is what the unit's `KillMode=control-group` does.
//! Operator requests start their worker with `systemd-run --user` and finish
//! it with the unit's `ExecStopPost`. The test puts a stand-in `systemd-run`
//! and `systemctl` first on PATH (the existing seam the manager's unit tests
//! use for `systemctl`): the stand-in runs the worker and then its
//! `ExecStopPost` as a detached session with the caller's environment, and
//! refuses any other use, so an action that would stop or start the service
//! fails visibly instead of passing. Readiness probes that call
//! `/usr/bin/systemctl` directly still see the real user manager, or none.
//!
//! The Windows side is a stand-in supervisor that keeps the service's
//! supervisor contract (readiness and retirement receipts, result and
//! ownership records, transport cleanup) without starting Wine. The native
//! side is this test binary re-executed as a separate DAW process: it maps
//! the published engine as a loader would and speaks the proxy's LVB5
//! admission protocol through `ap1_native_client::admission`.
//!
//! The memory transport root is the fixed per-user `/run/user/UID`
//! directory, which must exist on tmpfs. As in the product, one service per
//! user uses it: the tests take turns, and every session they create is
//! retired before the service stops. A bridge service the same user already
//! runs shares that root; do not run these tests during its package update.
use crate::test_fixture::{self, Fixture};
use linux_vst_bridge::{catalogue::Software, service::contract, *};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixStream, process::{CommandExt, ExitStatusExt}},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};

const MANAGER: &str = env!("CARGO_BIN_EXE_linux-vst-bridge");
const STARTUP: Duration = Duration::from_secs(60);
const OPERATION: Duration = Duration::from_secs(90);
/// From the DAW's acknowledged removal to the owner leaving the census.
const RETIREMENT: Duration = Duration::from_secs(5);

/// Published managed plug-ins in a fixture home, ready to be served.
pub struct Published {
    pub fixture: Fixture,
    pub plugins: Vec<Plugin>,
    manager: PathBuf,
    stubs: PathBuf,
}
/// One published class, as its native proxy identifies it.
pub struct Plugin {
    pub class_id: String,
    pub module_sha256: String,
    pub environment: String,
    pub engine: PathBuf,
    pub engine_sha256: String,
    pub descriptor_sha256: String,
}
impl Plugin {
    /// Its plug-in page, as the window names it.
    pub fn key(&self) -> contract::ProductKey {
        contract::ProductKey { environment: self.environment.clone(),
            module_sha256: self.module_sha256.clone(), class_id: self.class_id.clone() }
    }
}

/// The preparation fixture's candidate, built by the current reusable kit,
/// recorded and published through preparation, then installed beside the
/// real manager binary as the selected package generation.
pub fn published() -> Published {
    published_classes(1)
}
/// As `published`, with `count` classes from separate modules of one setup.
pub fn published_classes(count: usize) -> Published {
    require_transport_root();
    let (f, candidates) = test_fixture::projection_candidates_in(Fixture::installed(), false, count);
    for candidate in &candidates {
        preparation::record_candidate(&f.m, candidate).unwrap();
        preparation::enable(&f.m, candidate, false).unwrap();
    }
    let registry = f.m.registry().unwrap();
    let plugins: Vec<_> = candidates.iter().map(|candidate| {
        let entry = &registry.classes[&candidate.selection.class.id];
        assert_eq!(entry.publication, Publication::Published);
        let registration = &entry.registration;
        Plugin {
            class_id: registration.key(),
            module_sha256: registration.module.sha256.clone(),
            environment: registration.environment.id.clone(),
            engine: registration.native.path.clone(),
            engine_sha256: registration.native.sha256.clone(),
            descriptor_sha256: registration.descriptor.as_ref().expect("loaded-engine descriptor").sha256.clone(),
        }
    }).collect();
    assert_eq!(registry.classes.len(), count);

    // The selected package generation: every artifact read-only under software/.
    let mut software: Software = read_json(&f.m.root.join("software.json")).unwrap();
    let generation = f.m.root.join("software").join(format!("generation-{}", random_id().unwrap()));
    private_dir(&generation).unwrap();
    let install = |name: &str, bytes: &[u8], mode: u32| {
        let path = generation.join(name);
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        Artifact { sha256: digest(&path).unwrap(), path }
    };
    software.manager = install("linux-vst-bridge", &fs::read(MANAGER).unwrap(), 0o500);
    software.operator_frontend = Some(install("linux-audio-compatibility-manager",
        b"real-service fixture frontend; never executed\n", 0o500));
    software.supervisor = install("session.py", SUPERVISOR.as_bytes(), 0o400);
    software.ownership = install("ownership.py", b"# real-service fixture ownership module\n", 0o400);
    for artifact in [&software.host, &software.source_manifest] {
        fs::set_permissions(&artifact.path, fs::Permissions::from_mode(0o400)).unwrap();
    }
    atomic_json(&f.m.root.join("software.json"), &software).unwrap();

    let stubs = f.outer.join("stubs");
    private_dir(&stubs.join("units")).unwrap();
    for (name, script) in [("systemd-run", SYSTEMD_RUN), ("systemctl", SYSTEMCTL)] {
        let path = stubs.join(name);
        fs::write(&path, script.replace("@UNITS@", stubs.join("units").to_str().unwrap())).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o500)).unwrap();
    }
    Published { fixture: f, plugins, manager: software.manager.path, stubs }
}

/// A retained managed runtime (the r3 generation) beside the published
/// plug-ins, as an update leaves the previous one. The current overview
/// lists it and checks its pinned files, which lie below the runtime records
/// the change watch covers and are named by no stamp. Returns its `proton`.
pub fn retained_runtime(m: &Manager) -> PathBuf {
    use linux_vst_bridge::runtime_delivery;
    use std::os::unix::fs::MetadataExt;
    let base = m.root.join("runners").join(runtime_delivery::ID);
    let mut artifacts = Vec::new();
    for (relative, bytes) in [("GE-Proton11-7-x86_64/proton", b"fixture proton".as_slice()),
        ("SteamLinuxRuntime_4/_v2-entry-point", b"fixture entry".as_slice())] {
        let path = base.join(relative);
        private_dir(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o500)).unwrap();
        artifacts.push(Artifact { sha256: digest(&path).unwrap(), path });
    }
    let mut rows = std::collections::BTreeMap::new();
    for artifact in &artifacts {
        let relative = artifact.path.strip_prefix(&base).unwrap();
        let metadata = fs::metadata(&artifact.path).unwrap();
        rows.insert(relative.to_owned(), json!({"path":relative,"sha256":artifact.sha256,
            "target":null,"size":metadata.len(),"mode":metadata.mode() & 0o777,"directory":false}));
        for parent in relative.ancestors().skip(1).filter(|parent| !parent.as_os_str().is_empty()) {
            rows.insert(parent.to_owned(), json!({"path":parent,"sha256":null,"target":null,
                "size":0,"mode":0o700,"directory":true}));
        }
    }
    let tree = base.join("runtime-tree.json");
    atomic_json(&tree, &rows.into_values().collect::<Vec<_>>()).unwrap();
    fs::set_permissions(&tree, fs::Permissions::from_mode(0o400)).unwrap();
    let files = vec![artifacts[0].clone(), artifacts[1].clone(), Artifact { sha256: digest(&tree).unwrap(), path: tree }];
    let runner = Runner { id: runtime_delivery::ID.into(), version: "retained fixture".into(),
        proton: artifacts[0].path.clone(), entry_point: artifacts[1].path.clone(), files, policy: None };
    runner.verify().unwrap();
    let record = base.join("runtime.json");
    atomic_json(&record, &json!({"schema":1,"id":runtime_delivery::ID,
        "downloads":runtime_delivery::downloads(),"runner":runner})).unwrap();
    fs::set_permissions(&record, fs::Permissions::from_mode(0o400)).unwrap();
    artifacts[0].path.clone()
}

fn require_transport_root() {
    let runtime = PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() }));
    assert!(runtime.is_dir(), "{} is required: the service's memory transport root lives there. \
        Log in normally, or create it as a private tmpfs-backed directory owned by this user.",
        runtime.display());
    // Services started in parallel would otherwise race to create its marker.
    static INITIALIZED: std::sync::Once = std::sync::Once::new();
    INITIALIZED.call_once(|| transport_storage::initialize().unwrap());
}

pub struct RealService {
    pub published: Published,
    child: Option<Child>,
    // One user runs one bridge service: services share the per-user memory
    // transport root, so the tests run them one at a time.
    _turn: std::sync::MutexGuard<'static, ()>,
}
static ONE_SERVICE_PER_USER: std::sync::Mutex<()> = std::sync::Mutex::new(());
impl RealService {
    pub fn start(published: Published) -> Self {
        let turn = ONE_SERVICE_PER_USER.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut service = Self { published, child: None, _turn: turn };
        service.spawn();
        service
    }
    fn spawn(&mut self) {
        let log = self.published.fixture.outer.join(format!("service-{}.log", random_id().unwrap()));
        let output = fs::File::create(&log).unwrap();
        self.child = Some(command(&self.published, &self.published.manager)
            .arg("serve")
            .stdin(Stdio::null())
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .process_group(0)
            .spawn()
            .unwrap());
        let deadline = Instant::now() + STARTUP;
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                panic!("serve exited during startup: {status}");
            }
            if self.try_pulse().is_some() {
                return;
            }
            assert!(Instant::now() < deadline, "serve did not listen within {STARTUP:?}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    /// The `serve` process.
    pub fn pid(&self) -> u32 {
        self.child.as_ref().expect("a running service").id()
    }
    pub fn socket(&self) -> PathBuf {
        self.published.fixture.m.root.join("runtime/owner.sock")
    }
    /// LVP1: one byte, whether unconfirmed cleanup blocks new admissions.
    fn try_pulse(&self) -> Option<u8> {
        let mut peer = UnixStream::connect(self.socket()).ok()?;
        peer.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
        peer.write_all(b"LVP1\n").ok()?;
        let mut state = [0];
        peer.read_exact(&mut state).ok()?;
        Some(state[0])
    }
    pub fn pulse(&self) -> u8 {
        self.try_pulse().expect("LVP1 pulse")
    }
    /// LVC1: the service's own capacity census.
    pub fn capacity(&self) -> Value {
        self.census()["capacity"].clone()
    }
    /// LVC1: how the service answers status reads.
    pub fn views(&self) -> Value {
        self.census()["views"].clone()
    }
    fn census(&self) -> Value {
        let mut peer = UnixStream::connect(self.socket()).unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        peer.write_all(b"LVC1\n").unwrap();
        let mut length = [0; 4];
        peer.read_exact(&mut length).unwrap();
        let mut bytes = vec![0; u32::from_le_bytes(length) as usize];
        peer.read_exact(&mut bytes).unwrap();
        let reply: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(reply["ok"], true, "{reply:#}");
        reply
    }
    /// LVC1 until `settled` holds, within the service's retirement bound.
    pub fn capacity_when(&self, settled: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + RETIREMENT;
        loop {
            let capacity = self.capacity();
            if settled(&capacity) {
                return capacity;
            }
            assert!(Instant::now() < deadline, "capacity did not settle within {RETIREMENT:?}: {capacity:#}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    /// The installed manager command as the window runs it, in this fixture.
    pub fn manager_command(&self, program: Option<&Path>) -> contract::Subprocess {
        contract::Subprocess::new(program.unwrap_or(&self.published.manager))
            .with_environment(environment(&self.published))
    }
    /// The running service's socket, as the window reaches it.
    pub fn service(&self) -> contract::Service {
        contract::Service::new(self.socket())
    }
    /// One window exchange through the client contract. The reply must pass
    /// the contract's checks, and it is returned exactly as the service or
    /// command wrote it, so nothing the typed model would drop goes unseen.
    pub fn call(&self, transport: impl contract::Transport, request: &contract::Request) -> Value {
        let written = contract::Transport::exchange(&transport, request, None)
            .unwrap_or_else(|failure| panic!("{request:?}: {failure:?}"));
        contract::decode(request, &written).unwrap_or_else(|refused| panic!("{request:?}: {refused}"));
        serde_json::from_slice(&written).unwrap()
    }
    /// A status read as the window makes it: from the running service, or
    /// for the overview, through the manager command.
    pub fn read(&self, read: contract::Read) -> Value {
        if read.served() {
            self.call(self.service(), &contract::Request::Read(read))
        } else {
            self.call(self.manager_command(None), &contract::Request::Read(read))
        }
    }
    /// An action request; `request` is the window's request document.
    pub fn request(&self, request: &Value) -> Value {
        self.call(self.manager_command(None),
            &contract::Request::Action(serde_json::from_value(request.clone()).unwrap()))
    }
    /// The first published plug-in.
    pub fn plugin(&self) -> &Plugin {
        &self.published.plugins[0]
    }
    /// The first published plug-in's row in a snapshot.
    pub fn product<'a>(&self, snapshot: &'a Value) -> &'a Value {
        let products = snapshot["products"].as_array().unwrap();
        let matching: Vec<_> = products.iter()
            .filter(|product| product["class_id"] == self.plugin().class_id.as_str()).collect();
        assert_eq!(matching.len(), 1, "one product row for the published class: {products:#?}");
        matching[0]
    }
    /// The first published plug-in's page.
    pub fn product_detail(&self) -> Value {
        self.read(contract::Read::Product(self.plugin().key()))
    }
    /// The window's activity read until the operation is final, then the end
    /// of its worker unit (including ExecStopPost).
    pub fn finished_operation(&self, operation: &str) -> Value {
        let deadline = Instant::now() + OPERATION;
        let result = loop {
            let activity = self.read(contract::Read::Activity);
            let current = &activity["operation"];
            if current["operation"] == operation
                && matches!(current["state"].as_str(), Some("completed" | "refused")) {
                break current.clone();
            }
            assert!(Instant::now() < deadline, "operation {operation} unfinished: {activity:#}");
            std::thread::sleep(Duration::from_millis(100));
        };
        let unit = self.published.stubs.join("units")
            .join(format!("linux-vst-bridge-operator-{operation}.done"));
        while !unit.exists() {
            assert!(Instant::now() < deadline, "worker unit for {operation} did not end");
            std::thread::sleep(Duration::from_millis(50));
        }
        let ended: Value = read_json(&unit).unwrap();
        assert_eq!(ended, json!({"main":0,"stop_post":0}), "worker unit exit");
        result
    }
    /// A new native instance of `p` loaded by a separate DAW process.
    pub fn admit(&self, p: &Plugin) -> Instance {
        let config = [self.socket().to_str().unwrap(), &p.class_id, &p.module_sha256,
            &p.engine_sha256, &p.descriptor_sha256, p.engine.to_str().unwrap()].join(" ");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "native_admission_client", "--nocapture", "--test-threads=1", "-q"])
            .env_clear()
            .env("LVB_REAL_SERVICE_ADMISSION", config)
            // A DAW runs in a graphical session; the keeper binds to it.
            .env("DISPLAY", ":99")
            .env("HOME", &self.published.fixture.outer)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
            .spawn().unwrap();
        let stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        let admitted = match client_line(&mut stdout).split_once(' ') {
            Some(("accepted", binding)) => {
                let fields: Vec<_> = binding.splitn(4, ' ').collect();
                Admitted::Accepted {
                    session: fields[0].into(),
                    added_frames: fields[1].parse().unwrap(),
                    delivery_mode: fields[2].parse().unwrap(),
                    directory: fields[3].into(),
                }
            }
            Some(("refused", code)) => Admitted::Refused(code.into()),
            other => panic!("admission client: {other:?}"),
        };
        Instance { child, stdin: Some(stdin), stdout, admitted }
    }
    /// Stop the service as its unit does, then start it again on the same state.
    pub fn restart(&mut self) {
        self.terminate();
        self.spawn();
    }
    pub fn stop(mut self) {
        self.terminate();
    }
    fn terminate(&mut self) {
        let Some(mut child) = self.child.take() else { return };
        let group = child.id() as i32;
        assert_eq!(unsafe { libc::kill(-group, libc::SIGTERM) }, 0);
        let deadline = Instant::now() + Duration::from_secs(10);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() { break status; }
            assert!(Instant::now() < deadline, "serve did not stop on SIGTERM");
            std::thread::sleep(Duration::from_millis(20));
        };
        assert_eq!(status.signal(), Some(libc::SIGTERM), "serve ended by its stop signal: {status}");
        // Every process the service owned (its setup keepers) stops with it.
        loop {
            let remaining = group_members(group);
            if remaining.is_empty() { break; }
            assert!(Instant::now() < deadline, "service processes outlived the stop: {remaining:?}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
/// Live (not yet exited) processes in a process group. An orphan that has
/// exited but awaits its reaper is not running.
fn group_members(group: i32) -> Vec<String> {
    fs::read_dir("/proc").unwrap().flatten().filter_map(|entry| {
        entry.file_name().to_str()?.parse::<u32>().ok()?;
        let stat = fs::read_to_string(entry.path().join("stat")).ok()?;
        // pid (comm) state ppid pgrp ...
        let (head, tail) = stat.rsplit_once(')')?;
        let fields: Vec<_> = tail.split_whitespace().collect();
        (fields.get(2)?.parse::<i32>().ok()? == group && fields[0] != "Z").then(|| format!("{head})"))
    }).collect()
}
impl Drop for RealService {
    fn drop(&mut self) {
        if let Some(child) = &self.child {
            unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL); }
        }
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
        if std::thread::panicking() {
            let outer = &self.published.fixture.outer;
            let logs = fs::read_dir(outer).into_iter().flatten().flatten().map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|x| x == "log"));
            let units = fs::read_dir(self.published.stubs.join("units")).into_iter().flatten().flatten()
                .map(|entry| entry.path());
            for path in logs.chain(units) {
                eprintln!("--- {} ---\n{}", path.display(), fs::read_to_string(&path).unwrap_or_default());
            }
        }
    }
}

/// The product's own launch environment, reduced to what the service reads.
fn environment(p: &Published) -> [(&'static str, std::ffi::OsString); 4] {
    [("HOME", p.fixture.outer.clone().into()),
        ("PATH", format!("{}:/usr/local/bin:/usr/bin:/bin", p.stubs.display()).into()),
        ("LANG", "C.UTF-8".into()),
        ("RUST_BACKTRACE", "1".into())]
}
fn command(p: &Published, program: &Path) -> Command {
    let mut command = Command::new(program);
    command.env_clear().envs(environment(p));
    command
}

#[derive(Debug, PartialEq, Eq)]
pub enum Admitted {
    Accepted { session: String, added_frames: u32, delivery_mode: u32, directory: String },
    Refused(String),
}
/// One native instance held by its DAW process until released.
pub struct Instance {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
    admitted: Admitted,
}
impl Instance {
    pub fn admitted(&self) -> &Admitted {
        &self.admitted
    }
    pub fn session(&self) -> &str {
        match &self.admitted {
            Admitted::Accepted { session, .. } => session,
            Admitted::Refused(code) => panic!("refused instance has no session: {code}"),
        }
    }
    /// Remove the instance: the DAW closes its owner connection and waits
    /// for the supervisor's retirement acknowledgement.
    pub fn release(mut self) {
        if let Admitted::Accepted { .. } = self.admitted {
            let mut stdin = self.stdin.take().unwrap();
            stdin.write_all(b"release\n").unwrap();
            drop(stdin);
            assert_eq!(client_line(&mut self.stdout), "retired");
        }
        let status = self.child.wait().unwrap();
        assert!(status.success(), "admission client: {status}");
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        if self.stdin.is_some() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
fn client_line(stdout: &mut BufReader<ChildStdout>) -> String {
    let mut line = String::new();
    loop {
        line.clear();
        assert_ne!(stdout.read_line(&mut line).unwrap(), 0, "admission client ended without a result");
        if let Some(value) = line.trim_end().strip_prefix("lvb-admission ") {
            return value.into();
        }
    }
}

/// The admission client process. It behaves as the proxy's registered
/// startup: map the engine, send LVB5 with a fresh request identity, retry
/// only the typed busy refusal within the proxy's 65-second budget, then hold
/// the owner connection until told to release it.
pub fn admission_client() {
    use ap1_native_client::admission;
    let config = std::env::var("LVB_REAL_SERVICE_ADMISSION").expect("admission configuration");
    let fields: Vec<_> = config.split(' ').collect();
    let [socket, class, module, engine, descriptor, engine_path] = fields[..] else {
        panic!("admission configuration: {config}")
    };
    let bytes = |text: &str| -> Vec<u8> {
        (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap()).collect()
    };
    let image = fs::File::open(engine_path).unwrap();
    let length = image.metadata().unwrap().len() as usize;
    let mapped = unsafe { libc::mmap(std::ptr::null_mut(), length, libc::PROT_READ | libc::PROT_EXEC,
        libc::MAP_PRIVATE, std::os::fd::AsRawFd::as_raw_fd(&image), 0) };
    assert_ne!(mapped, libc::MAP_FAILED, "map the published engine: {}", std::io::Error::last_os_error());
    let mut greeting = admission::GREETING_V5.to_vec();
    for field in [class, module, engine, descriptor] {
        greeting.extend(bytes(field));
    }
    assert_eq!(greeting.len(), 117);
    let end = Instant::now() + Duration::from_secs(65);
    let (mut owner, result) = loop {
        let mut request = [0u8; 16];
        fs::File::open("/dev/urandom").unwrap().read_exact(&mut request).unwrap();
        let mut owner = UnixStream::connect(socket).unwrap();
        owner.set_write_timeout(Some(Duration::from_secs(5))).unwrap();
        owner.write_all(&greeting).unwrap();
        owner.write_all(&request).unwrap();
        owner.set_read_timeout(Some(end.saturating_duration_since(Instant::now()))).unwrap();
        let mut length = [0; 2];
        owner.read_exact(&mut length).unwrap();
        let mut reply = vec![0; u16::from_le_bytes(length) as usize];
        owner.read_exact(&mut reply).unwrap();
        let result = admission::decode_v5(&reply, request).unwrap();
        if result == Err(admission::Refusal::ServiceBusy) && Instant::now() + Duration::from_millis(20) < end {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        break (owner, result);
    };
    match result {
        Ok(binding) => println!("lvb-admission accepted {} {} {} {}", hex(&binding.session),
            binding.added_frames, binding.delivery_mode, binding.directory),
        Err(refusal) => {
            println!("lvb-admission refused {}", refusal.code());
            return;
        }
    }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
    assert_eq!(line, "release\n");
    owner.shutdown(std::net::Shutdown::Write).unwrap();
    owner.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    let mut acknowledgement = [0];
    owner.read_exact(&mut acknowledgement).unwrap();
    assert_eq!(&acknowledgement, b"R", "supervisor retirement acknowledgement");
    println!("lvb-admission retired");
}

/// Stand-in supervisor source, run through the product's own bootstrap.
/// A keeper publishes readiness until SIGTERM; an instance owns its DAW
/// connection (stdin) until the DAW closes it, then retires its transport.
const SUPERVISOR: &str = r#""""Real-service test supervisor: the service contract without Wine."""
import json, os, pathlib, shutil, signal, socket, sys, threading

job = json.loads(pathlib.Path(sys.argv[-1]).read_text())
session = job["session"]
report = pathlib.Path(job["report"])
environment = job["registration"]["environment"]["id"]

def record(path, value):
    temporary = path.with_name(path.name + ".tmp")
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(descriptor, "w") as output:
        json.dump(value, output)
    os.replace(temporary, path)

def retired(extra):
    record(report.with_suffix(".ownership.json"),
        {"session": session, "cleanup_confirmed": True, "transport_retired": True})
    record(report, {"session": session, "ownership_schema": 1, "error": None,
        "cleanup_confirmed": True, "transport_retired": True, **extra})

if job["keeper"]:
    stop = threading.Event()
    signal.signal(signal.SIGTERM, lambda *_: stop.set())
    record(report, {"session": session, "environment": environment, "ready": True})
    while not stop.wait(0.2):
        pass
    shutil.rmtree(job["directory"], ignore_errors=True)
    retired({"environment": environment, "ready": False})
    sys.exit(0)

peer = socket.socket(fileno=os.dup(0))
print("LVO0 " + session + " ready", flush=True)
while peer.recv(4096):
    pass
shutil.rmtree(job["directory"])
retired({})
peer.sendall(b"R")
peer.close()
print("LVO1 " + session + " retired", flush=True)
"#;

/// `systemd-run --user` for transient operator units: run the command, then
/// its ExecStopPost, detached, with the caller's environment and umask 077.
const SYSTEMD_RUN: &str = r#"#!/usr/bin/python3 -I
import json, os, pathlib, shlex, subprocess, sys
units = pathlib.Path("@UNITS@")
known = {"UMask", "KillMode", "StandardOutput", "StandardError"}
unit, stop_post, command = None, None, []
arguments = sys.argv[1:]
for index, argument in enumerate(arguments):
    if argument in ("--user", "--collect"):
        continue
    if argument.startswith("--property="):
        key, _, value = argument[len("--property="):].partition("=")
        if key == "ExecStopPost":
            stop_post = shlex.split(value.replace("%%", "%"))
        elif key not in known:
            sys.exit("real-service systemd-run: unsupported property " + key)
        continue
    if argument.startswith("--unit="):
        unit = argument[len("--unit="):]
        continue
    if argument.startswith("-"):
        sys.exit("real-service systemd-run: unsupported option " + argument)
    command = arguments[index:]
    break
if not unit or not command:
    sys.exit("real-service systemd-run: unit and command required")
(units / (unit + ".started")).write_text(json.dumps(command))
if os.fork():
    sys.exit(0)
# The unit belongs to the user manager, not to its caller: a new session with
# none of the caller's descriptors.
os.setsid()
os.umask(0o077)
null = os.open(os.devnull, os.O_RDWR)
log = os.open(units / (unit + ".log"), os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
os.dup2(null, 0)
os.dup2(log, 1)
os.dup2(log, 2)
main = subprocess.run(command).returncode
post = subprocess.run(stop_post).returncode if stop_post else 0
(units / (unit + ".done")).write_text(json.dumps({"main": main, "stop_post": post}))
"#;

/// `systemctl --user is-active` for the stand-in units; anything else fails.
const SYSTEMCTL: &str = r#"#!/usr/bin/python3 -I
import pathlib, sys
units = pathlib.Path("@UNITS@")
with open(units / "systemctl.log", "a") as log:
    log.write(" ".join(sys.argv[1:]) + "\n")
arguments = sys.argv[1:]
if arguments[:3] == ["--user", "is-active", "--quiet"] and len(arguments) == 4:
    unit = arguments[3].removesuffix(".service")
    live = (units / (unit + ".started")).exists() and not (units / (unit + ".done")).exists()
    sys.exit(0 if live else 3)
sys.exit("real-service systemctl: unsupported " + " ".join(arguments))
"#;

/// Whether `kind` for `class` with `fields` is offered: `None` when absent,
/// `Some(None)` when enabled, `Some(Some(reason))` when shown disabled.
pub fn offer_state(offered: &[&Value], kind: &str, class: &str, fields: &Value) -> Option<Option<String>> {
    let matching: Vec<_> = offered.iter().filter(|offer| {
        let action = &offer["action"];
        action["kind"] == kind && action["class_id"] == class
            && fields.as_object().unwrap().iter().all(|(key, value)| &action[key] == value)
    }).collect();
    assert!(matching.len() <= 1, "one {kind} offer: {matching:#?}");
    matching.first().map(|offer| offer["disabled_reason"].as_str().map(str::to_owned))
}

/// Every offer on a product: its own actions and its compatibility workflow.
pub fn offers(product: &Value) -> Vec<&Value> {
    product["actions"].as_array().unwrap().iter()
        .chain(product["compatibility"]["primary"].as_array().into_iter().flatten())
        .collect()
}

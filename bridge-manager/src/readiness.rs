//! Bounded local facts, execution requirements and separate support qualification.
//! This module never executes a profile or interprets SUPPORT_MATRIX.md.
use super::*;
use linux_vst_bridge::{operator_model as ui, profiles};
use serde_json::{json, Value};
use std::{io::Read, os::{fd::AsRawFd, unix::{fs::FileTypeExt, process::CommandExt}}, process::{Child, Stdio}, thread, time::Duration};

const BITWIG: &str = "com.bitwig.BitwigStudio";
const DECK_MODEL: &str = "Galileo";
const DECK_DISTRO: &str = "steamos";
const DECK_VERSION: &str = "3.8.16";
const BITWIG_VERSION: &str = "6.1";
const BITWIG_REF: &str = "app/com.bitwig.BitwigStudio/x86_64/stable";
const BETA_PROFILE_IDS: [&str; 3] = [
    "arturia-pure-lofi",
    "arturia-efx-fragments",
    "arturia-pigments",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Probe<T> { Observed(T), Absent, Unavailable, Malformed }
impl<T> Probe<T> {
    fn observed(&self) -> Option<&T> {
        if let Self::Observed(value) = self { Some(value) } else { None }
    }
    fn is_absent(&self) -> bool { matches!(self, Self::Absent) }
}
impl<T: Clone> Probe<T> {
    fn value(&self) -> Option<T> { self.observed().cloned() }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct PlatformReadback {
    pub operating_system: Probe<String>,
    pub arch: Probe<String>,
    pub model: Option<String>,
    pub distro: Option<String>,
    pub distro_version: Option<String>,
    pub kernel: Option<String>,
    pub session: Option<String>,
    pub display: Option<String>,
    pub runtime_dir: Probe<bool>,
    pub pipewire_socket: Probe<bool>,
    pub jack_available: Probe<bool>,
    pub pipewire_graph_rate: Option<u32>,
    pub pipewire_quantum: Option<u32>,
    /// Only a trusted live DAW/device adapter may populate these. Idle PB0
    /// collection deliberately leaves them unknown.
    pub device_sample_rate: Option<u32>,
    pub daw_callback_maximum: Option<u32>,
    pub bitwig_ref: Probe<String>,
    pub bitwig_version: Probe<String>,
    pub bitwig_runtime: Probe<String>,
    pub bitwig_permissions: Probe<String>,
    pub publication_directory: Probe<bool>,
    /// Observations belong to the consuming host context. Manager-side paths
    /// and Flatpak permission declarations cannot populate these as available.
    pub hosts: Vec<HostReadback>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HostReadback {
    pub name: String,
    pub source: String,
    pub adapter: Probe<bool>,
    pub publication: Probe<bool>,
    pub ipc: Probe<bool>,
    pub pipewire: Probe<bool>,
    pub jack: Probe<bool>,
    pub display: Probe<bool>,
}

fn unavailable_host(name: &str, source: &str, adapter: Probe<bool>) -> HostReadback {
    HostReadback { name:name.into(), source:source.into(), adapter,
        publication:Probe::Unavailable, ipc:Probe::Unavailable,
        pipewire:Probe::Unavailable, jack:Probe::Unavailable, display:Probe::Unavailable }
}

fn available(probe: &Probe<bool>) -> bool { probe.observed() == Some(&true) }
fn absent(probe: &Probe<bool>) -> bool {
    probe.is_absent() || probe.observed() == Some(&false)
}
fn either(left: &Probe<bool>, right: &Probe<bool>) -> Probe<bool> {
    if available(left) || available(right) { Probe::Observed(true) }
    else if absent(left) && absent(right) { Probe::Absent }
    else { Probe::Unavailable }
}

fn host_requirements(host: &HostReadback, requires_display: bool) -> Vec<ui::ReadinessBlocker> {
    use ui::ReadinessOutcome as O;
    let mut missing = Vec::new();
    let audio = either(&host.pipewire, &host.jack);
    for (category, probe, description) in [
        ("host_adapter", &host.adapter, "consuming-host adapter"),
        ("publication_path", &host.publication, "managed publication access"),
        ("host_ipc", &host.ipc, "bridge IPC access"),
        ("host_audio", &audio, "PipeWire or JACK access"),
    ].into_iter().chain(requires_display.then_some(
        ("host_display", &host.display, "X11/XWayland access for the selected Windows host route"))) {
        if !available(probe) {
            missing.push(issue(category, if absent(probe) { O::ActionRequired } else { O::Unknown },
                &format!("{}: {description} {}.", host.name,
                    if absent(probe) { "is unavailable" } else { "has not been observed in this consuming context" })));
        }
    }
    missing
}

fn capability_fact(name: &str, probe: &Probe<bool>, source: &str, at: u64) -> ui::ReadinessFact {
    observed(name, if available(probe) { Some("available".into()) }
        else if absent(probe) { Some("unavailable".into()) } else { None }, source, at)
}

fn bounded_file(path: &Path, limit: u64) -> Option<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= limit as usize)
        .then(|| String::from_utf8(bytes).ok())
        .flatten()
}

fn path_probe(path: &Path, socket: bool) -> Probe<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if (socket && metadata.file_type().is_socket())
            || (!socket && metadata.file_type().is_dir()) => Probe::Observed(true),
        Ok(_) => Probe::Malformed,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Probe::Absent,
        Err(_) => Probe::Unavailable,
    }
}

/// Only fixed, read-only platform verbs. The 3-second deadline and 4-KiB
/// output limit prevent a broken helper from hanging manager readback.
fn bounded_command(program: &str, args: &[&str]) -> Probe<String> {
    let executable = match program {
        "flatpak" => "/usr/bin/flatpak",
        "pw-metadata" => "/usr/bin/pw-metadata",
        "jack_lsp" => "/usr/bin/jack_lsp",
        "uname" => "/usr/bin/uname",
        "systemctl" => "/usr/bin/systemctl",
        "glxinfo" if args == ["-B"] => "/usr/bin/glxinfo",
        _ => return Probe::Unavailable,
    };
    let mut command = Command::new(executable);
    command
        .args(args)
        .env("LC_ALL", "C")
        .env(
            "XDG_RUNTIME_DIR",
            format!("/run/user/{}", unsafe { libc::getuid() }),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // Each fixed helper owns a private session. A timed-out parent may have
    // descendants holding the output pipe; retire the entire owned cohort.
    unsafe { command.pre_exec(|| {
        if libc::setsid() < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
    }); }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Probe::Unavailable,
        Err(_) => return Probe::Unavailable,
    };
    bounded_child(child, Duration::from_secs(3))
}

/// A fixed, read-only systemd query with the same process-cohort and output
/// bounds as platform probes. Callers supply only manager-derived unit names.
pub(super) fn bounded_user_unit_state(unit: &str) -> Result<String> {
    require(unit.len() <= 128 && unit.starts_with("linux-vst-bridge-")
        && unit.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.')),
        "readiness_unit_identity")?;
    admit_unit_state(bounded_command("systemctl", &["--user", "show", unit,
        "-p", "ActiveState", "--value"]))
}
fn admit_unit_state(probe: Probe<String>) -> Result<String> {
    match probe {
        Probe::Observed(state) if matches!(state.as_str(),
            "active" | "activating" | "deactivating" | "reloading" | "inactive" | "failed") => Ok(state),
        _ => Err("readiness_unit_state_unavailable".into()),
    }
}

pub(super) fn service_state() -> &'static str {
    match bounded_command("systemctl", &["--user", "show", "-p", "ActiveState",
        "--value", "linux-vst-bridge.service"]) {
        Probe::Observed(value) if value == "active" => "active",
        Probe::Observed(value) if matches!(value.as_str(), "inactive" | "failed") => "inactive",
        _ => "unknown",
    }
}

fn retire_helper(child: &mut Child) {
    // The setsid leader remains unreaped until its owned group is signalled.
    // A numeric group is never signalled after this owner has been reaped.
    unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL); }
    let _ = child.wait();
}
#[cfg(target_os = "linux")]
fn completed_helper(child: &mut Child) -> std::io::Result<Option<std::process::ExitStatus>> {
    let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::waitid(libc::P_PID, child.id() as libc::id_t, &mut info,
        libc::WEXITED | libc::WNOHANG | libc::WNOWAIT) };
    if result < 0 { return Err(std::io::Error::last_os_error()); }
    if info.si_signo == 0 { return Ok(None); }
    // WNOWAIT preserves the leader PID and group identity until after all
    // fixed-helper descendants in that session receive retirement.
    unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL); }
    child.wait().map(Some)
}
fn bounded_child(mut child: Child, limit: Duration) -> Probe<String> {
    let Some(mut stdout) = child.stdout.take() else {
        retire_helper(&mut child);
        return Probe::Unavailable;
    };
    let flags = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        retire_helper(&mut child);
        return Probe::Unavailable;
    }
    let deadline = Instant::now() + limit;
    let mut bytes = Vec::new();
    let mut exited = None;
    let mut eof = false;
    loop {
        let mut chunk = [0u8; 512];
        loop {
            match stdout.read(&mut chunk) {
                Ok(0) => { eof = true; break; }
                Ok(n) if bytes.len() + n <= 4096 => bytes.extend_from_slice(&chunk[..n]),
                Ok(_) => { retire_helper(&mut child); return Probe::Malformed; }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => { retire_helper(&mut child); return Probe::Unavailable; }
            }
        }
        if eof && exited.is_none() {
            #[cfg(target_os = "linux")]
            let completion = completed_helper(&mut child);
            #[cfg(not(target_os = "linux"))]
            let completion = child.try_wait();
            match completion {
                Ok(status) => exited = status,
                Err(_) => { retire_helper(&mut child); return Probe::Unavailable; }
            }
        }
        if eof && exited.is_some() { break; }
        if Instant::now() >= deadline {
            // A descendant can retain stdout after the fixed helper exits.
            // Drop the descriptor without leaving a blocked reader thread.
            retire_helper(&mut child);
            return Probe::Unavailable;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !exited.is_some_and(|status| status.success()) { return Probe::Absent; }
    match String::from_utf8(bytes) {
        Ok(text) => Probe::Observed(text.trim().to_owned()),
        Err(_) => Probe::Malformed,
    }
}

fn key_value(text: &str, key: &str) -> Option<String> {
    let value = text
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))?;
    let value = value.trim().trim_matches('"');
    (!value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

fn pipewire_setting(text: &str, key: &str) -> Option<u32> {
    // pw-metadata prints key:'clock.rate' value:'48000'. This is graph
    // configuration, never a device rate or Bitwig callback maximum.
    text.lines().find_map(|line| {
        (line.contains(&format!("key:'{key}'")) || line.contains(&format!("key: '{key}'")))
            .then(|| line.split("value:").nth(1))
            .flatten()
            .and_then(|value| value.trim().trim_matches('\'').split('\'').next())
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| (32..=384000).contains(value))
    })
}

fn local_display_number(raw: &str) -> Option<u8> {
    let mut parts = raw.strip_prefix(':')?.split('.');
    let number = parts.next()?;
    if let Some(screen) = parts.next() {
        if screen.is_empty() || screen.len() > 2 || !screen.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    if parts.next().is_some() {
        return None;
    }
    (!number.is_empty() && number.len() <= 2 && number.bytes().all(|b| b.is_ascii_digit()))
        .then(|| number.parse::<u8>().ok())
        .flatten()
}

fn process_start(process: &Path, pid: i32) -> Option<u64> {
    let stat = bounded_file(&process.join("stat"), 4096)?;
    let (number, _) = stat.split_once(" (")?;
    if number.parse::<i32>().ok()? != pid { return None; }
    stat.rsplit_once(')')?.1.split_whitespace().nth(19)?.parse().ok()
}

fn same_visible_object(path: &Path, process_root: &Path) -> Probe<bool> {
    use std::os::unix::fs::MetadataExt;
    let Ok(relative) = path.strip_prefix("/") else { return Probe::Malformed; };
    match (fs::metadata(path), consuming_metadata(process_root, relative)) {
        (Ok(host), Ok(peer)) => Probe::Observed(host.dev() == peer.dev() && host.ino() == peer.ino()),
        (Err(e), _) | (_, Err(e)) if e.kind() == std::io::ErrorKind::NotFound => Probe::Absent,
        _ => Probe::Unavailable,
    }
}

#[cfg(target_os = "linux")]
fn consuming_metadata(process_root: &Path, relative: &Path) -> std::io::Result<fs::Metadata> {
    use std::os::unix::ffi::OsStrExt;
    let root = fs::File::open(process_root)?;
    let path = std::ffi::CString::new(relative.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags = (libc::O_PATH | libc::O_CLOEXEC) as u64;
    // Absolute publication symlinks must resolve inside the consumer's root.
    // Ordinary /proc/PID/root/path metadata can follow one back into our root.
    how.resolve = libc::RESOLVE_IN_ROOT | libc::RESOLVE_NO_MAGICLINKS;
    let fd = unsafe { libc::syscall(libc::SYS_openat2, root.as_raw_fd(), path.as_ptr(),
        &how, std::mem::size_of::<libc::open_how>()) };
    if fd < 0 { return Err(std::io::Error::last_os_error()); }
    let file = unsafe { fs::File::from_raw_fd(fd as i32) };
    file.metadata()
}
#[cfg(not(target_os = "linux"))]
fn consuming_metadata(_process_root: &Path, _relative: &Path) -> std::io::Result<fs::Metadata> {
    Err(std::io::ErrorKind::Unsupported.into())
}

fn mapped_artifact(process: &Path, artifact: &Path) -> Option<bool> {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::metadata(artifact).ok()?;
    let maps = bounded_file(&process.join("maps"), 2 * 1024 * 1024)?;
    for line in maps.lines() {
        let fields: Vec<_> = line.split_whitespace().take(5).collect();
        if fields.len() != 5 { return None; }
        let (major, minor) = fields[3].split_once(':')?;
        if fields[4].parse::<u64>().ok()? == meta.ino()
            && u64::from_str_radix(major,16).ok()? == libc::major(meta.dev() as libc::dev_t) as u64
            && u64::from_str_radix(minor,16).ok()? == libc::minor(meta.dev() as libc::dev_t) as u64 {
            return Some(true);
        }
    }
    Some(false)
}

/// An existing authenticated admission supplies the peer; no executable name,
/// desktop-file guess or manager-side path can impersonate a consuming DAW.
fn observed_consumer(m: &Manager, registration: &Registration,
    graphical: &transport_storage::GraphicalSession, proc_root: &Path,
    transport_root: &Path, pipewire: &Probe<bool>, jack: &Probe<bool>) -> Result<HostReadback> {
    use std::os::unix::fs::MetadataExt;
    let process = proc_root.join(graphical.peer_pid.to_string());
    require(graphical.schema == 1 && graphical.peer_pid > 0 && graphical.peer_start_ticks > 0
        && process.metadata()?.uid() == unsafe { libc::getuid() }
        && process_start(&process, graphical.peer_pid) == Some(graphical.peer_start_ticks),
        "readiness_consumer_generation")?;
    require(mapped_artifact(&process, &registration.native.path) == Some(true),
        "readiness_consumer_proxy_mapping")?;
    let root = process.join("root");
    let root_identity = root.metadata()?;
    let mount_identity = fs::read_link(process.join("ns/mnt")).ok();
    let sandbox = match consuming_metadata(&root, Path::new(".flatpak-info")) {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.into()),
    };
    // Only identical mount namespaces can inherit host-side audio observation.
    // A sandbox needs its own visible socket; a host JACK query cannot prove it.
    let same_mount = mount_identity.clone()
        .zip(fs::read_link(proc_root.join("self/ns/mnt")).ok())
        .map(|(peer, ours)| peer == ours);
    let ipc_identity = fs::read_link(process.join("ns/ipc")).ok();
    let same_ipc = ipc_identity.clone()
        .zip(fs::read_link(proc_root.join("self/ns/ipc")).ok())
        .map(|(peer, ours)| peer == ours);
    let runtime = PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() }));
    let display = local_display_number(&graphical.display).map(|number| {
        let socket = PathBuf::from(format!("/tmp/.X11-unix/X{number}"));
        if available(&path_probe(&socket, true)) { same_visible_object(&socket, &root) }
        else { path_probe(&socket, true) }
    }).unwrap_or(Probe::Unavailable);
    let result = HostReadback {
        name: if sandbox || same_mount == Some(false) { "Active sandboxed DAW" }
            else if same_mount == Some(true) { "Active native DAW" }
            else { "Active DAW (mount context unknown)" }.into(),
        source: format!("authenticated consumer, mapped proxy and mount view for class {}; not DAW or audio qualification",
            registration.metadata.class_id),
        adapter: if same_mount.is_some() { Probe::Observed(true) } else { Probe::Unavailable },
        publication: same_visible_object(&m.link(&registration.metadata.class_id), &root),
        ipc: same_visible_object(transport_root, &root),
        pipewire: if available(pipewire) { same_visible_object(&runtime.join("pipewire-0"), &root) }
            else { pipewire.clone() },
        jack: if same_mount == Some(true) && same_ipc == Some(true) { jack.clone() } else { Probe::Unavailable },
        display,
    };
    let final_root = root.metadata()?;
    require(process_start(&process, graphical.peer_pid) == Some(graphical.peer_start_ticks)
        && mount_identity == fs::read_link(process.join("ns/mnt")).ok()
        && ipc_identity == fs::read_link(process.join("ns/ipc")).ok()
        && (root_identity.dev(), root_identity.ino()) == (final_root.dev(), final_root.ino()),
        "readiness_consumer_generation_changed")?;
    Ok(result)
}

fn collect_consumers(m: &Manager, pipewire: &Probe<bool>, jack: &Probe<bool>) -> Result<Vec<HostReadback>> {
    collect_consumers_from(m, pipewire, jack, Path::new("/proc"), &transport_storage::root())
}

fn collect_consumers_from(m: &Manager, pipewire: &Probe<bool>, jack: &Probe<bool>,
    proc_root: &Path, transport_root: &Path) -> Result<Vec<HostReadback>> {
    let _guard = match m.try_lock("registry.lock")? {
        operator_lock::LockAttempt::Acquired(guard) => guard,
        operator_lock::LockAttempt::Busy => return Err("readiness_consumers_busy".into()),
    };
    let registry = m.registry()?;
    let mut result = Vec::new();
    for owner in capacity::owners(m)?.into_iter()
        .filter(|owner| owner.kind == capacity::Kind::Dsp && owner.terminal.is_none()) {
        let report: PathBuf = read_json(&m.root.join("runtime/leases").join(format!("{}.json", owner.session)))?;
        // A stale lease from an earlier boot can otherwise collide with a new
        // PID and start tick. Historical ownership is never a current peer.
        let generation: LeaseGeneration = read_json(&m.root.join("runtime/lease-generations")
            .join(format!("{}.json", owner.session)))?;
        let boot = bounded_file(&proc_root.join("sys/kernel/random/boot_id"), 64)
            .ok_or("readiness_consumer_boot_unavailable")?;
        require(generation.schema == 1 && generation.session == owner.session
            && generation.report == report && valid_kernel_boot(boot.trim())
            && generation.kernel_boot == boot.trim()
            && matches!(generation.basis, LeaseGenerationBasis::BeforeLaunch),
            "readiness_consumer_boot_changed")?;
        let (retained, _) = m.lease_owner(&owner.session, &report)?;
        let registration = &registry.classes.get(&owner.class_id)
            .ok_or("readiness_consumer_registration_absent")?.registration;
        let bound: HostBinding = serde_json::from_value(retained["registration"].clone())?;
        require(bound == registration.clone().into(), "readiness_consumer_registration_changed")?;
        let graphical = serde_json::from_value(retained["graphical_session"].clone())?;
        result.push(observed_consumer(m, registration, &graphical, proc_root,
            transport_root, pipewire, jack)?);
    }
    Ok(result)
}

pub(super) fn collect(m: &Manager) -> PlatformReadback {
    let os = bounded_file(Path::new("/etc/os-release"), 8192).unwrap_or_default();
    let env = |key: &str| {
        std::env::var(key).ok().filter(|value| {
            !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
        })
    };
    let display = env("DISPLAY")
        .and_then(|raw| local_display_number(&raw))
        .and_then(|number| {
            Path::new("/tmp/.X11-unix")
                .join(format!("X{number}"))
                .exists()
                .then(|| "socket available".into())
        });
    // Independent fixed readbacks run together, so one unavailable desktop
    // helper cannot add its full timeout to every other helper's latency.
    let (info, metadata, bitwig_ref, bitwig_runtime, bitwig_permissions, arch, jack, flatpak_list) =
        thread::scope(|scope| {
            let info = scope.spawn(|| bounded_command("flatpak", &["info", BITWIG]));
            let metadata = scope.spawn(|| bounded_command("pw-metadata", &["-n", "settings", "0"]));
            let reference = scope.spawn(|| bounded_command("flatpak", &["info", "--show-ref", BITWIG]));
            let runtime = scope.spawn(|| bounded_command("flatpak", &["info", "--show-runtime", BITWIG]));
            let permissions = scope.spawn(|| bounded_command("flatpak", &["info", "--show-permissions", BITWIG]));
            let arch = scope.spawn(|| bounded_command("uname", &["-m"]));
            let jack = scope.spawn(|| bounded_command("jack_lsp", &[]));
            let flatpak_list = scope.spawn(|| bounded_command("flatpak", &["list", "--app", "--columns=application"]));
            (info.join().unwrap_or(Probe::Unavailable),
                metadata.join().unwrap_or(Probe::Unavailable),
                reference.join().unwrap_or(Probe::Unavailable),
                runtime.join().unwrap_or(Probe::Unavailable),
                permissions.join().unwrap_or(Probe::Unavailable),
                arch.join().unwrap_or(Probe::Unavailable),
                jack.join().unwrap_or(Probe::Unavailable),
                flatpak_list.join().unwrap_or(Probe::Unavailable))
        });
    let runtime = PathBuf::from(format!("/run/user/{}", unsafe { libc::getuid() }));
    let runtime_dir = path_probe(&runtime, false);
    let version = match info {
        Probe::Observed(info) => info.lines().find_map(|line| {
            line.trim().strip_prefix("Version:").map(str::trim)
                .filter(|value| !value.is_empty() && value.len() <= 64)
                .map(str::to_owned)
        }).map_or(Probe::Malformed, Probe::Observed),
        Probe::Absent => Probe::Absent,
        Probe::Unavailable => Probe::Unavailable,
        Probe::Malformed => Probe::Malformed,
    };
    let nonempty = |probe: Probe<String>| match probe {
        Probe::Observed(value) if !value.is_empty() && value.len() <= 4096 => Probe::Observed(value),
        Probe::Observed(_) => Probe::Malformed,
        other => other,
    };
    let arch = match nonempty(arch) {
        Probe::Absent => Probe::Unavailable,
        other => other,
    };
    let bitwig_ref = match bitwig_ref {
        Probe::Absent => match flatpak_list.observed() {
            Some(list) if !list.lines().any(|line| line.trim() == BITWIG) => Probe::Absent,
            _ => Probe::Unavailable,
        },
        other => nonempty(other),
    };
    let pipewire_socket = path_probe(&runtime.join("pipewire-0"), true);
    let jack_available = match jack {
        Probe::Observed(_) => Probe::Observed(true),
        Probe::Absent => Probe::Absent,
        Probe::Unavailable => Probe::Unavailable,
        Probe::Malformed => Probe::Malformed,
    };
    let mut hosts = vec![unavailable_host("Native DAW", "no current authenticated native consumer observation", Probe::Unavailable),
        unavailable_host("Bitwig Flatpak", "installation metadata only; consuming paths require an authenticated consumer",
            match &bitwig_ref { Probe::Observed(_) => Probe::Observed(true), Probe::Absent => Probe::Absent, _ => Probe::Unavailable })];
    match collect_consumers(m, &pipewire_socket, &jack_available) {
        Ok(consumers) => hosts.extend(consumers),
        Err(_) => hosts.push(unavailable_host("Active DAW", "current consumer ownership could not be verified", Probe::Unavailable)),
    }
    PlatformReadback {
        operating_system: Probe::Observed(std::env::consts::OS.into()),
        arch,
        model: bounded_file(Path::new("/sys/devices/virtual/dmi/id/product_name"), 128)
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty()),
        distro: key_value(&os, "ID"),
        distro_version: key_value(&os, "VERSION_ID"),
        kernel: bounded_file(Path::new("/proc/sys/kernel/osrelease"), 256)
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty()),
        session: env("XDG_SESSION_TYPE"),
        display,
        runtime_dir,
        pipewire_socket,
        jack_available,
        pipewire_graph_rate: metadata.observed().and_then(|v| pipewire_setting(v, "clock.rate")),
        pipewire_quantum: metadata.observed().and_then(|v| pipewire_setting(v, "clock.quantum")),
        device_sample_rate: None,
        daw_callback_maximum: None,
        bitwig_ref,
        bitwig_version: version,
        bitwig_runtime: nonempty(bitwig_runtime),
        bitwig_permissions: nonempty(bitwig_permissions),
        publication_directory: path_probe(&m.publications, false),
        hosts,
    }
}

fn fact(
    name: &str,
    value: Option<String>,
    source: &str,
    at: u64,
    certainty: ui::FactCertainty,
) -> ui::ReadinessFact {
    ui::ReadinessFact {
        name: name.into(),
        value,
        source: source.into(),
        observed_at: at,
        certainty,
    }
}
fn observed(name: &str, value: Option<String>, source: &str, at: u64) -> ui::ReadinessFact {
    fact(
        name,
        value.clone(),
        source,
        at,
        if value.is_some() {
            ui::FactCertainty::Observed
        } else {
            ui::FactCertainty::Unknown
        },
    )
}
fn issue(category: &str, status: ui::ReadinessOutcome, explanation: &str) -> ui::ReadinessBlocker {
    ui::ReadinessBlocker {
        category: category.into(),
        status,
        explanation: explanation.into(),
    }
}
fn step(title: &str, detail: &str, action: Option<ui::AvailableAction>) -> ui::ReadinessStep {
    ui::ReadinessStep {
        title: title.into(),
        detail: detail.into(),
        action,
    }
}
fn relevant_repair(product: &ui::Product) -> Option<ui::AvailableAction> {
    product
        .actions
        .iter()
        .find(|offer| {
            matches!(
                offer.action,
                ui::Action::OrdinaryRollback { .. }
                    | ui::Action::OrdinaryRestoreRecommended { .. }
                    | ui::Action::QuarantinedModuleRetry { .. }
                    | ui::Action::EnvironmentRescan { .. }
            )
        })
        .cloned()
}
fn product_facts(product: &ui::Product, at: u64) -> Vec<ui::ReadinessFact> {
    let exact = |name: &str, value: Option<String>| {
        observed(name, value, "canonical manager product readback", at)
    };
    let valid = |name: &str, key: &str| {
        exact(
            name,
            product.details[key].as_bool().map(|valid| {
                if valid {
                    "verified"
                } else {
                    "missing_or_changed"
                }
                .into()
            }),
        )
    };
    vec![
        exact("Environment ID", Some(product.environment.clone())),
        exact(
            "Environment revision",
            product.details["environment_revision"]
                .as_u64()
                .map(|n| n.to_string()),
        ),
        valid("Environment verification", "environment_valid"),
        exact("Runner ID", Some(product.runner.clone())),
        valid("Runner verification", "runner_valid"),
        exact("Requested Windows graphics", product.details["requested_graphics"].as_str().map(str::to_owned)),
        observed("Plug-in renderer", None,
            "requires observation in this exact Windows runtime and editor session", at),
        observed("Plug-in hardware acceleration", None,
            "requested policy and native host diagnostics cannot establish this", at),
        exact("Module SHA-256", Some(product.module_sha256.clone())),
        valid("Module verification", "module_valid"),
        exact("VST3 class ID", Some(product.class_id.clone())),
        exact(
            "Profile ID",
            product.details["profile"]["id"].as_str().map(str::to_owned),
        ),
        exact(
            "Profile revision",
            product.details["profile"]["revision"]
                .as_u64()
                .map(|n| n.to_string()),
        ),
        exact(
            "Windows host SHA-256",
            product.details["host_sha256"].as_str().map(str::to_owned),
        ),
        exact(
            "Windows host source SHA-256",
            product.details["host_source_sha256"]
                .as_str()
                .map(str::to_owned),
        ),
        valid("Windows host verification", "host_valid"),
        exact(
            "Native proxy SHA-256",
            product.details["native_sha256"].as_str().map(str::to_owned),
        ),
        valid("Native proxy verification", "native_valid"),
        exact(
            "Publication ID",
            product.details["publication"]["id"]
                .as_str()
                .map(str::to_owned),
        ),
        exact(
            "Publication SHA-256",
            product.details["publication"]["sha256"]
                .as_str()
                .map(str::to_owned),
        ),
        valid("Publication verification", "publication_valid"),
        valid("Completed publication", "publication_complete"),
        exact("Bridge presentation reserve",product.details["added_frames"].as_u64()
            .map(|frames|format!("{frames} frames"))),
        exact("Publication selected",
            product.details["publication_selected"].as_bool().map(|selected|
                if selected {"yes"} else {"no"}.into())),
    ]
}
fn matches_deck(p: &PlatformReadback) -> bool {
    p.operating_system.observed().map(String::as_str) == Some("linux")
        && p.arch.observed().map(String::as_str) == Some("x86_64")
        && p.model.as_deref() == Some(DECK_MODEL)
        && p.distro.as_deref() == Some(DECK_DISTRO)
        && p.distro_version.as_deref() == Some(DECK_VERSION)
        && p.bitwig_ref.observed().map(String::as_str) == Some(BITWIG_REF)
        && p.bitwig_version.observed().map(String::as_str) == Some(BITWIG_VERSION)
}

/// A reviewed source-owned support envelope, bound to the revision captured
/// with the current registry and installed profiles. It cannot extend support
/// to sibling builds.
fn accepted_profile_captured(installed: &[profiles::Profile],
    revision: &publication::Revision, product: &ui::Product) -> bool {
    installed.iter().any(|profile| {
        BETA_PROFILE_IDS.contains(&profile.id.as_str())
            && revision.profile == *profile
            && profile.claim == profiles::Claim::VerifiedExactFixture
            && product.details["qualification"].is_null()
            && product.module_sha256 == profile.module_sha256
            && product.runner == profile.requirements.runner.id
            && product.details["environment_revision"] == profile.requirements.environment_revision
            && product.details["host_sha256"] == profile.requirements.host_sha256
            && product.details["host_source_sha256"] == profile.requirements.host_source_sha256
            && product.details["native_sha256"] == profile.requirements.native_sha256
            && product.details["publication_complete"] == true
            && product.details["added_frames"] == 512
            && product.version == profile.class.version
            && product.details["profile"]["id"] == profile.id
            && product.details["profile"]["revision"] == profile.revision
    })
}

pub(super) fn resolve_captured(snapshot: &ui::Snapshot, p: &PlatformReadback,
    installed: &[profiles::Profile],
    revisions: &std::collections::BTreeMap<String,
        std::result::Result<Option<publication::Revision>, &'static str>>,
    at: u64) -> ui::ReadinessAssessment {
    resolve_with(snapshot, p, at, &[], |product| match revisions.get(&product.class_id) {
        Some(Ok(Some(revision))) => Ok(accepted_profile_captured(installed, revision, product)),
        Some(Err(code)) => Err(*code),
        _ => Ok(false),
    })
}

fn resolve_with(
    snapshot: &ui::Snapshot,
    p: &PlatformReadback,
    at: u64,
    authority_failures: &[&'static str],
    accepted: impl Fn(&ui::Product) -> std::result::Result<bool, &'static str>,
) -> ui::ReadinessAssessment {
    use ui::ReadinessOutcome as O;
    let mut blockers = Vec::new();
    let mut steps = Vec::new();
    let has_deck = matches_deck(p);
    for code in authority_failures {
        blockers.push(issue(
            code,
            O::Unknown,
            "Installed compatibility authority could not be verified. Open Diagnostics or create a support export.",
        ));
    }
    if p.operating_system.observed().map(String::as_str) != Some("linux") {
        blockers.push(issue("operating_system", if p.operating_system.observed().is_some() {
            O::Unsupported
        } else { O::Unknown }, "The selected native engine requires the Linux platform ABI."));
    }
    if p.arch.observed().map(String::as_str) != Some("x86_64") {
        blockers.push(issue(
            "architecture",
            if p.arch.observed().is_some() {
                O::Unsupported
            } else {
                O::Unknown
            },
            "This native bridge lane requires x86-64 Linux.",
        ));
    }
    // These are prerequisites of the implemented Windows-host route, not a
    // machine/product support list. A live DAW format is negotiated at launch.
    let audio = either(&p.pipewire_socket, &p.jack_available);
    let display = p.display.as_ref().map(|_| Probe::Observed(true)).unwrap_or(Probe::Unavailable);
    for (category, probe, explanation) in [
        ("runtime_ipc", &p.runtime_dir, "The user runtime directory is required for bridge IPC."),
        ("publication_path", &p.publication_directory, "The managed publication directory is required."),
        ("audio_ipc", &audio, "At least one observed PipeWire or JACK audio route is required."),
        ("graphics", &display, "X11/XWayland is required by the selected Windows host launch route."),
    ] {
        if !available(probe) {
            blockers.push(issue(category, if absent(probe) { O::ActionRequired } else { O::Unknown }, explanation));
            steps.push(step("Review required capability", explanation, None));
        }
    }
    if !snapshot.system.capacity_available() {
        blockers.push(issue(
            "service",
            O::ActionRequired,
            "Bridge service readback is unavailable; current capacity and cleanup are unknown.",
        ));
        steps.insert(
            0,
            step(
                "Restore bridge readback",
                "Open Diagnostics and Check again after service recovery.",
                None,
            ),
        );
    } else if snapshot.system.cleanup_unconfirmed || snapshot.system.stale_transports > 0 {
        blockers.push(issue(
            "cleanup",
            O::ActionRequired,
            "Cleanup is unresolved; new work is unsafe.",
        ));
        steps.insert(
            0,
            step(
                "Review cleanup",
                "Open Diagnostics and use only manager-offered recovery.",
                None,
            ),
        );
    }
    if snapshot.system.pending_transactions > 0 {
        blockers.push(issue(
            "transaction",
            O::ActionRequired,
            "A publication transaction needs reconciliation.",
        ));
        let offered = snapshot
            .actions
            .iter()
            .find(|a| matches!(a.action, ui::Action::TransactionReconcile {}))
            .cloned();
        steps.insert(
            0,
            step(
                "Reconcile the publication",
                "Use the manager's exact recovery action when enabled.",
                offered,
            ),
        );
    }
    for attempt in &snapshot.onboarding {
        if attempt.state == "needs_user_action" {
            blockers.push(issue(
                "vendor_action",
                O::ActionRequired,
                "A vendor or user-owned installation step is waiting in Setup.",
            ));
            steps.push(step(
                "Continue lawful vendor setup",
                &attempt.required_human_action,
                attempt
                    .actions
                    .iter()
                    .find(|offer| offer.disabled_reason.is_none())
                    .cloned(),
            ));
        }
    }
    let mut products = Vec::new();
    for product in &snapshot.products {
        let acceptance = accepted(product);
        let authority_failure = acceptance.as_ref().err().copied();
        let exact = acceptance == Ok(true);
        let current_checks = [
            "module_valid",
            "environment_valid",
            "runner_valid",
            "native_valid",
            "host_valid",
            "publication_valid",
            "performance_valid",
        ];
        let health = if product.disposition == "quarantined"
            || product.details["current"] == false
            || current_checks.iter().any(|key| product.details[*key] == false) {
            ui::InstallationHealth::ActionRequired
        } else if current_checks.iter().all(|key| product.details[*key] == true) {
            ui::InstallationHealth::Healthy
        } else {
            ui::InstallationHealth::Unknown
        };
        let support = if product.details["profile"]["claim"] == "withdrawn" {
            ui::SupportQualification::Unsupported
        } else if exact && has_deck {
            ui::SupportQualification::Verified
        } else {
            ui::SupportQualification::NotYetQualified
        };
        let (status, reason) = if product.disposition == "quarantined" {
            (O::ActionRequired, "The managed scan quarantined this exact module.")
        } else if product.details["current"] == false {
            (O::ActionRequired,
                "The installed plug-in inventory is stale. Check Setup for the exact offered refresh or rescan action.")
        } else if health == ui::InstallationHealth::ActionRequired {
            (O::ActionRequired,
                "The current module, environment, runner, host, proxy or publication needs repair.")
        } else if product.details["publication_selected"] == false {
            (O::ActionRequired,
                "This product is not currently made available to native DAWs.")
        } else if authority_failure.is_some() {
            (
                O::Unknown,
                "Installed compatibility authority could not be verified for this product.",
            )
        } else if health == ui::InstallationHealth::Healthy
            && product.details["publication_selected"] == true
            && product.disposition == "ready" {
            (O::Ready, "Selected execution bindings verify. The publication is eligible to try; live host behavior and support qualification are separate.")
        } else {
            (O::Unknown, "Required execution bindings or the selected publication have not been verified.")
        };
        if let Some(code) = authority_failure {
            if !blockers.iter().any(|blocker| blocker.category == code) {
                blockers.push(issue(code, O::Unknown,
                    "Installed compatibility authority could not be verified. Open Diagnostics or create a support export."));
            }
        }
        if status == O::ActionRequired {
            blockers.push(issue(
                "product",
                status,
                &format!("{}: {reason}", product.name),
            ));
            steps.push(step(
                &format!("Review {}", product.name),
                reason,
                relevant_repair(product),
            ));
        } else if status == O::Unsupported {
            blockers.push(issue(
                "product",
                status,
                &format!("{}: {reason}", product.name),
            ));
        }
        products.push(ui::ReadinessProduct {
            name: product.name.clone(),
            class_id: product.class_id.clone(),
            module_sha256: product.module_sha256.clone(),
            profile: product.details["profile"]["id"].as_str().map(str::to_owned),
            status,
            installation_health: health,
            support_qualification: support,
            reason: reason.into(),
            failure_code: authority_failure.map(str::to_owned).or_else(|| {
                product.details["refusal"]["code"]
                    .as_str()
                    .map(str::to_owned)
            }),
            facts: product_facts(product, at),
        });
    }
    let any_ready = products.iter().any(|product| product.status == O::Ready);
    if !any_ready && products.is_empty() {
        blockers.push(issue(
            "product",
            O::ActionRequired,
            "No managed plug-in has been installed and published.",
        ));
        steps.push(step(
            "Add a plug-in",
            "Open Setup to import the lawful Windows installer.",
            None,
        ));
    }
    // Current DAW rate/block size are live negotiation facts. An idle manager
    // must neither invent them nor use their absence as a publication refusal.
    // Product failures remain scoped to their product.
    let system_blockers = blockers.iter().filter(|b| b.category != "product");
    let system_statuses: Vec<_> = system_blockers.map(|b| b.status).collect();
    let overall = if system_statuses.contains(&O::Unsupported) {
        O::Unsupported
    } else if system_statuses.contains(&O::Unknown) {
        O::Unknown
    } else if !system_statuses.is_empty() {
        O::ActionRequired
    } else if any_ready {
        O::Ready
    } else if products.is_empty()
        || products
            .iter()
            .any(|product| product.status == O::ActionRequired)
    {
        O::ActionRequired
    } else {
        O::Unknown
    };
    if overall == O::Unsupported {
        steps=vec![step("Review required architecture",
            "The selected native engine requires x86-64 Linux.",None)];
    } else if overall == O::Unknown {
        steps.truncate(1);
        if steps.is_empty() { steps.push(step("Review missing observations",
            "A required execution capability could not be verified. Review Details or create a support export.",None)); }
    } else {
        steps.truncate(1);
    }
    if !system_statuses.is_empty() {
        for product in &mut products {
            if product.status == O::Ready {
                product.status = overall;
                product.reason =
                    "Selected execution bindings verify; a required system capability needs attention."
                        .into();
            }
        }
    }
    let source = "historical bounded support envelope; not an execution requirement";
    let qualified = |name: &str, value: &str| {
        fact(
            name,
            Some(value.into()),
            source,
            at,
            ui::FactCertainty::Observed,
        )
    };
    ui::ReadinessAssessment {
        schema: 1,
        state_token: snapshot.state_token.clone(),
        observed_at: at,
        overall_status: overall,
        system: snapshot.system.clone(),
        platform: vec![
            observed("Operating system", p.operating_system.value(), "manager native platform ABI", at),
            observed("CPU architecture", p.arch.value(), "uname -m", at),
            observed("Hardware model", p.model.clone(), "DMI product name", at),
            observed("Distribution", p.distro.clone(), "/etc/os-release", at),
            observed(
                "Distribution version",
                p.distro_version.clone(),
                "/etc/os-release",
                at,
            ),
            observed("Kernel", p.kernel.clone(), "/proc/sys/kernel/osrelease", at),
            qualified("Historical qualified distribution", "SteamOS 3.8.16"),
        ],
        daw: {
            let mut facts = vec![
                observed("DAW Flatpak ref", p.bitwig_ref.value(), "flatpak info; installation only", at),
                observed("DAW version", p.bitwig_version.value(), "flatpak info; installation only", at),
                observed("DAW runtime", p.bitwig_runtime.value(), "flatpak info; installation only", at),
                observed("Sandbox permission metadata", p.bitwig_permissions.observed().map(|_| "available; not consuming-path proof".into()), "flatpak info", at),
                qualified("Historical qualified DAW", "Bitwig Studio 6.1 Flatpak"),
            ];
            for host in &p.hosts {
                let missing = host_requirements(host, true);
                let status = if missing.is_empty() { "required access observed" }
                    else if missing.iter().any(|item| item.status == O::ActionRequired) { "required access unavailable" }
                    else { "consuming-context observation unavailable" };
                facts.push(observed(&format!("{} context", host.name), Some(status.into()), &host.source, at));
                for (name, probe) in [("adapter", &host.adapter), ("publication access", &host.publication),
                    ("bridge IPC access", &host.ipc), ("PipeWire access", &host.pipewire),
                    ("JACK access", &host.jack), ("route display access", &host.display)] {
                    facts.push(capability_fact(&format!("{} {name}", host.name), probe, &host.source, at));
                }
            }
            facts
        },
        audio: vec![
            observed(
                "PipeWire availability",
                p.pipewire_socket.observed()
                    .map(|value| if *value { "available" } else { "unavailable" }.into()),
                "user runtime socket",
                at,
            ),
            observed(
                "JACK availability",
                p.jack_available.observed()
                    .map(|v| if *v { "available" } else { "unavailable" }.into()),
                "jack_lsp readback",
                at,
            ),
            observed(
                "PipeWire graph rate",
                p.pipewire_graph_rate.map(|n| n.to_string()),
                "pw-metadata settings",
                at,
            ),
            observed(
                "PipeWire graph quantum",
                p.pipewire_quantum.map(|n| n.to_string()),
                "pw-metadata settings",
                at,
            ),
            observed(
                "Audio device sample rate",
                p.device_sample_rate.map(|n| n.to_string()),
                "trusted live DAW/device readback; unavailable when idle",
                at,
            ),
            observed(
                "DAW callback maximum",
                p.daw_callback_maximum.map(|n| n.to_string()),
                "trusted live DAW readback; unavailable when idle",
                at,
            ),
            qualified("Historical qualified device sample rate", "48000 Hz"),
            qualified("Historical qualified host maximum", "at most 512 frames"),
            observed(
                "Bridge processing quantum",
                None,
                "not exposed in current operator snapshot",
                at,
            ),
            qualified("Historical presentation reserve", "512 added frames"),
            observed(
                "Vendor algorithmic latency",
                None,
                "requires live vendor report",
                at,
            ),
        ],
        graphics: vec![
            observed(
                "Desktop session",
                p.session.clone(),
                "user session environment",
                at,
            ),
            fact(
                "X11/XWayland socket",
                p.display.clone(),
                "DISPLAY and local X11 socket",
                at,
                if p.display.is_some() {
                    ui::FactCertainty::Inferred
                } else {
                    ui::FactCertainty::Unknown
                },
            ),
            observed(
                "Native host graphics diagnostic",
                None,
                "collected only by an explicit support export; does not identify the Windows editor renderer",
                at,
            ),
        ],
        runtime: vec![
            observed(
                "User runtime directory",
                p.runtime_dir.observed()
                    .map(|value| if *value { "available" } else { "unavailable" }.into()),
                "/run/user/<uid>",
                at,
            ),
            observed(
                "PipeWire socket",
                p.pipewire_socket.observed()
                    .map(|value| if *value { "available" } else { "unavailable" }.into()),
                "user runtime directory",
                at,
            ),
            observed(
                "Manager service",
                Some(snapshot.system.service.clone()),
                "canonical capacity readback",
                at,
            ),
        ],
        products,
        blockers,
        ordered_steps: steps,
        support_export_action: ui::AvailableAction {
            label: "Create sanitized support export".into(),
            action: ui::Action::SupportExport {},
            disabled_reason: None,
        },
    }
}

fn safe_export_text(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    if value.len() > 160
        || value.chars().any(char::is_control)
        || lower.contains("/home/")
        || lower.contains("/users/")
        || lower.contains("http:")
        || lower.contains("https:")
        || value.contains(['?', '@', '~', '\\'])
    {
        "[redacted]".into()
    } else {
        value.into()
    }
}
#[derive(serde::Serialize)]
struct SupportReadiness {
    schema: u32,
    observed_at: u64,
    overall_status: ui::ReadinessOutcome,
    platform: Vec<ui::ReadinessFact>,
    daw: Vec<ui::ReadinessFact>,
    audio: Vec<ui::ReadinessFact>,
    graphics: Vec<ui::ReadinessFact>,
    runtime: Vec<ui::ReadinessFact>,
    products: Vec<ui::ReadinessProduct>,
    blockers: Vec<ui::ReadinessBlocker>,
    suggested_step_title: Option<String>,
}
fn sanitized(mut assessment: ui::ReadinessAssessment) -> SupportReadiness {
    for facts in [
        &mut assessment.platform,
        &mut assessment.daw,
        &mut assessment.audio,
        &mut assessment.graphics,
        &mut assessment.runtime,
    ] {
        for fact in facts {
            fact.value = fact.value.as_deref().map(safe_export_text);
        }
    }
    for product in &mut assessment.products {
        product.name = safe_export_text(&product.name);
        product.profile = product.profile.as_deref().map(safe_export_text);
        product.reason = safe_export_text(&product.reason);
        product.failure_code = product.failure_code.as_deref().map(safe_export_text);
        for fact in &mut product.facts {
            fact.value = fact.value.as_deref().map(safe_export_text);
        }
    }
    for blocker in &mut assessment.blockers {
        blocker.explanation = safe_export_text(&blocker.explanation);
    }
    SupportReadiness {
        schema: 1,
        observed_at: assessment.observed_at,
        overall_status: assessment.overall_status,
        platform: assessment.platform,
        daw: assessment.daw,
        audio: assessment.audio,
        graphics: assessment.graphics,
        runtime: assessment.runtime,
        products: assessment.products,
        blockers: assessment.blockers,
        suggested_step_title: assessment.ordered_steps.first()
            .map(|step| safe_export_text(&step.title)),
    }
}

/// The export is an allowlist built from the typed assessment. No raw logs,
/// paths, environment dump, license state, or vendor payload can enter it.
pub(super) fn export(m: &Manager, overview: &ui::InteractiveOverview) -> Result<Value> {
    let snapshot = &overview.current;
    let mut assessment = overview.readiness.clone();
    // A separate native context is useful support evidence, never a Windows
    // rendering qualification. Do not start it during ordinary UI readback.
    let graphics = if std::env::var("DISPLAY").ok().and_then(|s| local_display_number(&s)).is_some() {
        bounded_command("glxinfo", &["-B"])
    } else { Probe::Unavailable };
    append_host_graphics(&mut assessment, graphics, observation::now()?);
    let refused_step = assessment
        .ordered_steps
        .first()
        .and_then(|step| step.action.as_ref())
        .and_then(|action| action.disabled_reason.as_deref())
        .map(safe_export_text);
    let assessment = sanitized(assessment);
    let sw: Software = read_json(&m.root.join("software.json"))?;
    write_export(m, snapshot, assessment, &sw, refused_step)
}
fn append_host_graphics(assessment: &mut ui::ReadinessAssessment, probe: Probe<String>, at: u64) {
    let parsed = probe.observed().and_then(|text| linux_vst_bridge::graphics::parse_host_glx(text));
    let status = match (&probe, &parsed) {
        (_, Some(_)) => "observed",
        (Probe::Observed(_) | Probe::Malformed, _) => "malformed or ambiguous",
        (Probe::Absent, _) => "probe failed",
        (Probe::Unavailable, _) => "unavailable or timed out",
    };
    let source = "explicit native glxinfo -B diagnostic; separate from the Windows plug-in context";
    assessment.graphics.push(observed("Native GLX probe status", Some(status.into()), source, at));
    for (name, value) in [
        ("Native GLX vendor", parsed.as_ref().map(|p| p.vendor.clone())),
        ("Native GLX renderer", parsed.as_ref().map(|p| p.renderer.clone())),
        ("Native GLX version", parsed.as_ref().map(|p| p.version.clone())),
        ("Native GLX rendering", parsed.as_ref().map(|p| p.rendering().to_owned())),
    ] { assessment.graphics.push(observed(name, value, source, at)); }
}
#[cfg(feature = "pb0-c0-audit")]
pub(super) fn audit_export(m: &Manager, overview: &ui::InteractiveOverview) -> Result<Value> {
    let snapshot = &overview.current;
    let assessment = sanitized(overview.readiness.clone());
    let sw: Software = read_json(&m.root.join("software.json"))?;
    report_value(m, snapshot, assessment, &sw, None)
}
fn write_export(
    m: &Manager,
    snapshot: &ui::Snapshot,
    assessment: SupportReadiness,
    software: &Software,
    refused_step: Option<String>,
) -> Result<Value> {
    let report = report_value(m, snapshot, assessment, software, refused_step)?;
    let id = random_id()?;
    let directory = m.root.join("support-exports");
    private_dir(&directory)?;
    atomic_json(&directory.join(format!("{id}.json")), &report)?;
    Ok(json!({"export":id,"file":format!("{id}.json"),"result":"saved_locally"}))
}
fn report_value(m: &Manager, snapshot: &ui::Snapshot,
    assessment: SupportReadiness, software: &Software,
    refused_step: Option<String>) -> Result<Value> {
    let current_error = assessment
        .blockers
        .iter()
        .find(|blocker| blocker.category.starts_with("READINESS_"))
        .map(|blocker| blocker.category.clone())
        .or_else(|| {
            assessment
                .products
                .iter()
                .find_map(|product| product.failure_code.clone())
        })
        .or_else(|| {
            assessment
                .blockers
                .first()
                .map(|blocker| blocker.category.clone())
        });
    let package_version = crate::package_authority::selected_package_version(m, software)?;
    let selected_software_generation = software.manager.path.parent()
        .and_then(|parent| parent.file_name()).and_then(|name| name.to_str())
        .filter(|id| valid_hex(id, 64)).map(str::to_owned);
    let incident_rows = if snapshot.recent_incidents.is_empty() {
        let mut rows = Vec::new();
        for path in crash_capture::incidents(m)?.into_iter().take(16) {
            let id = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let status_path = path.join("status.json");
            let share_path = path.join("share.json");
            let status: Value = if status_path.try_exists()? { read_json(&status_path)? }
                else { Value::Null };
            let share: Value = if share_path.try_exists()? { read_json(&share_path)? }
                else { Value::Null };
            rows.push(json!({"state":safe_export_text(status["state"].as_str().unwrap_or("unknown")),
                "id":if valid_hex(id,32){id.to_owned()}else{"[redacted]".into()},
                "category":share["first_failure"]["code"].as_str()
                    .or_else(||share["category"].as_str()).map(safe_export_text)
                    .unwrap_or_else(||"unclassified".into())}));
        }
        rows
    } else {
        snapshot.recent_incidents.iter().take(16).map(|incident|json!({
            "state":safe_export_text(&incident.state),
            "id":if valid_hex(&incident.id,32){incident.id.clone()}else{"[redacted]".into()},
            "category":incident.summary["first_failure"]["code"].as_str()
                .or_else(||incident.summary["category"].as_str())
                .map(safe_export_text).unwrap_or_else(||"unclassified".into())
        })).collect()
    };
    Ok(json!({"schema":2,"assessment":assessment,
        "selected_software_generation":selected_software_generation,
        "manager_sha256":software.manager.sha256,
        "frontend_sha256":software.operator_frontend.as_ref().map(|a| &a.sha256),
        "package_version":package_version,
        "manager_source_version":env!("CARGO_PKG_VERSION"),
        "transaction_cleanup":snapshot.system,
        "refused_step":refused_step,
        "current_error":current_error,
        "incidents":incident_rows}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graphics_host_observation_does_not_qualify_the_windows_renderer() {
        let (mut snapshot, platform) = fixture();
        snapshot.products[0].details["requested_graphics"] = json!(linux_vst_bridge::graphics::requested_backend(
            Some(&linux_vst_bridge::RunnerPolicy::DcompWineBuiltinsReferenceV1)));
        let mut assessment = result(&snapshot, &platform);
        let prior = assessment.clone();
        append_host_graphics(&mut assessment, Probe::Observed(
            "name of display: secret-host\nOpenGL vendor string: Mesa\nOpenGL renderer string: llvmpipe (LLVM test)\nOpenGL version string: 4.5 Test\nAccelerated: no\n".into()), 456);
        assert_eq!(assessment.overall_status, prior.overall_status);
        assert_eq!(assessment.products, prior.products);
        let facts = &assessment.products[0].facts;
        assert!(facts.iter().find(|f| f.name == "Requested Windows graphics").unwrap().value.as_ref().unwrap().contains("WineD3D"));
        for name in ["Plug-in renderer", "Plug-in hardware acceleration"] {
            let f = facts.iter().find(|f| f.name == name).unwrap();
            assert!(f.value.is_none()); assert_eq!(f.certainty, ui::FactCertainty::Unknown);
        }
        assert_eq!(assessment.graphics.iter().find(|f| f.name == "Native GLX rendering").unwrap().value.as_deref(), Some("software"));
        assert!(!serde_json::to_string(&sanitized(assessment)).unwrap().contains("secret-host"));
    }
    #[test]
    fn graphics_probe_failure_keeps_observations_unknown() {
        assert!(matches!(bounded_command("glxinfo", &["-display", "remote:0"]), Probe::Unavailable));
        let (snapshot, platform) = fixture();
        for probe in [Probe::Absent, Probe::Unavailable, Probe::Malformed,
            Probe::Observed("OpenGL renderer string: incomplete".into())] {
            let mut assessment = result(&snapshot, &platform);
            append_host_graphics(&mut assessment, probe, 456);
            assert!(assessment.graphics.iter().filter(|f| f.name.starts_with("Native GLX")
                && f.name != "Native GLX probe status").all(|f| f.value.is_none()));
        }
    }
    #[test]
    fn bounded_unit_readback_admits_only_known_exact_states() {
        for state in ["active","activating","deactivating","reloading","inactive","failed"] {
            assert_eq!(admit_unit_state(Probe::Observed(state.into())).unwrap(),state);
        }
        for probe in [Probe::Absent,Probe::Unavailable,Probe::Malformed,
            Probe::Observed("unknown".into())] {
            assert!(admit_unit_state(probe).is_err());
        }
        for unit in ["other.service","linux-vst-bridge-../escape.service",
            "linux-vst-bridge-unit with spaces.service"] {
            assert!(bounded_user_unit_state(unit).is_err());
        }
    }
    fn support_software(f: &crate::test_fixture::Fixture) -> Software {
        let artifact = f.r.host.clone();
        // A support export uses an actual retained software layout. Synthetic
        // aliases for the helpers cannot stand in for a missing package record.
        let helper = |name: &str| {
            let path = artifact.path.with_file_name(name);
            fs::write(&path, name.as_bytes()).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            Artifact { sha256: digest(&path).unwrap(), path }
        };
        let supervisor = helper("session.py");
        let ownership = helper("ownership.py");
        fs::set_permissions(&artifact.path, fs::Permissions::from_mode(0o400)).unwrap();
        Software { installer_launch: None, preparation_kit: None,
            operator_frontend: Some(artifact.clone()), manager: artifact.clone(),
            supervisor, ownership,
            host: artifact.clone(), source_manifest: artifact.clone(),
            source_sha256: artifact.sha256.clone(), native_catalogue: None }
    }

    #[test]
    fn bounded_platform_parsers_keep_graph_values_separate_from_device_and_host_values() {
        assert!(matches!(bounded_command("sh", &["-c", "true"]), Probe::Unavailable));
        assert_eq!(local_display_number(":0"), Some(0));
        assert_eq!(local_display_number(":12.0"), Some(12));
        assert_eq!(local_display_number(":0.bad"), None);
        assert_eq!(local_display_number("localhost:10"), None);
        let os = "ID=steamos\nVERSION_ID=\"3.8.16\"\n";
        assert_eq!(key_value(os, "ID").as_deref(), Some("steamos"));
        assert_eq!(key_value(os, "VERSION_ID").as_deref(), Some("3.8.16"));
        assert_eq!(key_value("ID=bad\nOTHER=value\n", "DEVICE_RATE"), None);
        let graph="subject:0 key:'clock.rate' value:'48000' type:'Spa:String:JSON'\nsubject:0 key:'clock.quantum' value:'512' type:'Spa:String:JSON'";
        assert_eq!(pipewire_setting(graph, "clock.rate"), Some(48000));
        assert_eq!(pipewire_setting(graph, "clock.quantum"), Some(512));
        assert_eq!(pipewire_setting(graph, "device.rate"), None);
    }

    #[test]
    fn platform_helper_output_and_inherited_pipe_have_a_bounded_lifetime() {
        let spawn = |script| {
            let mut command = Command::new("/bin/sh");
            command.args(["-c", script]).stdout(Stdio::piped());
            unsafe { command.pre_exec(|| {
                if libc::setsid() < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
            }); }
            command.spawn().unwrap()
        };
        let child = spawn("printf ready");
        assert_eq!(bounded_child(child, Duration::from_secs(2)), Probe::Observed("ready".into()));
        let child = spawn("printf ready; sleep 0.3 &");
        let started = Instant::now();
        assert_eq!(bounded_child(child, Duration::from_millis(50)), Probe::Unavailable);
        assert!(started.elapsed() < Duration::from_secs(2));
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn timed_out_fixed_helper_retires_its_descendant() {
        let marker = std::env::temp_dir().join(format!("lvb-probe-{}", random_id().unwrap()));
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 30 & printf '%s' \"$!\" > \"$PID_FILE\"; wait"])
            .env("PID_FILE", &marker).stdout(Stdio::piped());
        unsafe { command.pre_exec(|| {
            if libc::setsid() < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
        }); }
        let child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !marker.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        let descendant: i32 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
        assert_eq!(bounded_child(child, Duration::from_millis(30)), Probe::Unavailable);
        let _ = std::fs::remove_file(&marker);
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let live = unsafe { libc::kill(descendant, 0) } == 0;
            let zombie = std::fs::read_to_string(format!("/proc/{descendant}/stat"))
                .ok().and_then(|s|s.split_whitespace().nth(2).map(str::to_owned))
                .is_some_and(|state|state == "Z");
            if !live || zombie { break; }
            assert!(Instant::now() < deadline, "fixed-helper descendant survived timeout");
            thread::sleep(Duration::from_millis(5));
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn completed_fixed_helper_retires_descendant_that_closed_stdout() {
        let marker = std::env::temp_dir().join(format!("lvb-probe-{}", random_id().unwrap()));
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 30 >/dev/null & printf '%s' \"$!\" > \"$PID_FILE\"; printf ready"])
            .env("PID_FILE", &marker).stdout(Stdio::piped());
        unsafe { command.pre_exec(|| {
            if libc::setsid() < 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
        }); }
        let child = command.spawn().unwrap();
        assert_eq!(bounded_child(child, Duration::from_secs(2)), Probe::Observed("ready".into()));
        let descendant: i32 = std::fs::read_to_string(&marker).unwrap().parse().unwrap();
        let _ = std::fs::remove_file(&marker);
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            let live = unsafe { libc::kill(descendant, 0) } == 0;
            let zombie = std::fs::read_to_string(format!("/proc/{descendant}/stat"))
                .ok().and_then(|s|s.split_whitespace().nth(2).map(str::to_owned))
                .is_some_and(|state|state == "Z");
            if !live || zombie { break; }
            assert!(Instant::now() < deadline, "fixed-helper descendant survived completion");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn fixture() -> (ui::Snapshot, PlatformReadback) {
        let mut snapshot: ui::Snapshot = serde_json::from_str(include_str!(
            "../../manager-ui/examples/library-preview.json"
        ))
        .unwrap();
        snapshot.products.truncate(1);
        let p = profiles::ap17_profiles().unwrap().remove(0);
        let product = &mut snapshot.products[0];
        product.class_id = p.class.class_id;
        product.name = p.class.name;
        product.version = p.class.version;
        product.module_sha256 = p.module_sha256;
        product.runner = p.requirements.runner.id;
        product.disposition = "ready".into();
        product.details = json!({"profile":{"id":p.id,"revision":p.revision,"claim":"verified_exact_fixture"},
            "module_valid":true,"environment_valid":true,"runner_valid":true,
            "native_valid":true,"host_valid":true,"publication_valid":true,
            "publication_complete":true,"publication_selected":true,"performance_valid":true,"added_frames":512,
            "qualification":null});
        snapshot.onboarding.clear();
        snapshot.system.service = "active".into();
        snapshot.system.dsp = 0;
        snapshot.system.maintenance = 0;
        snapshot.system.pending_transactions = 0;
        snapshot.system.stale_transports = 0;
        snapshot.system.cleanup_unconfirmed = false;
        let platform = PlatformReadback {
            operating_system: Probe::Observed("linux".into()),
            arch: Probe::Observed("x86_64".into()),
            model: Some(DECK_MODEL.into()),
            distro: Some(DECK_DISTRO.into()),
            distro_version: Some(DECK_VERSION.into()),
            kernel: Some("6.11".into()),
            session: Some("wayland".into()),
            display: Some("available".into()),
            runtime_dir: Probe::Observed(true),
            pipewire_socket: Probe::Observed(true),
            jack_available: Probe::Observed(true),
            pipewire_graph_rate: Some(48000),
            pipewire_quantum: Some(512),
            device_sample_rate: Some(48000),
            daw_callback_maximum: Some(512),
            bitwig_ref: Probe::Observed(BITWIG_REF.into()),
            bitwig_version: Probe::Observed(BITWIG_VERSION.into()),
            bitwig_runtime: Probe::Observed("org.freedesktop.Platform/x86_64/24.08".into()),
            bitwig_permissions: Probe::Observed(
                "[Context]\nsockets=x11;pulseaudio;\nfilesystems=host;\n".into(),
            ),
            publication_directory: Probe::Observed(true),
            hosts:vec![unavailable_host("Native DAW", "idle fixture", Probe::Unavailable)],
        };
        (snapshot, platform)
    }
    fn result(snapshot: &ui::Snapshot, platform: &PlatformReadback) -> ui::ReadinessAssessment {
        let accepted = profiles::ap17_profiles().unwrap();
        resolve_with(snapshot, platform, 123, &[], |product| {
            Ok(accepted.iter().any(|profile| {
                product.class_id == profile.class.class_id
                    && product.module_sha256 == profile.module_sha256
                    && product.details["profile"]["id"] == profile.id
                    && product.details["profile"]["revision"] == profile.revision
            }))
        })
    }
    #[test]
    fn exact_ready_fixture_and_unobservable_audio_are_distinct() {
        let (s, mut p) = fixture();
        let ready = result(&s, &p);
        assert_eq!(ready.overall_status, ui::ReadinessOutcome::Ready);
        assert_eq!(ready.products[0].status, ui::ReadinessOutcome::Ready);
        assert!(ready.ordered_steps.is_empty());
        p.device_sample_rate = None;
        p.daw_callback_maximum = None;
        let idle = result(&s, &p);
        assert_eq!(idle.overall_status, ui::ReadinessOutcome::Ready);
        assert_eq!(idle.products[0].status, ui::ReadinessOutcome::Ready);
        assert!(idle.ordered_steps.is_empty());
        assert!(idle
            .audio
            .iter()
            .find(|f| f.name == "DAW callback maximum")
            .unwrap()
            .value
            .is_none());
        assert_eq!(
            idle.audio
                .iter()
                .find(|f| f.name == "PipeWire graph rate")
                .unwrap()
                .value
                .as_deref(),
            Some("48000")
        );
    }
    #[test]
    fn platform_daw_audio_and_graphics_refuse_false_ready() {
        let (s, p) = fixture();
        type PlatformCase = (
            &'static str,
            fn(&mut PlatformReadback),
            ui::ReadinessOutcome,
        );
        let cases: Vec<PlatformCase> = vec![
            ("incompatible platform ABI", |p| p.operating_system = Probe::Observed("macos".into()), ui::ReadinessOutcome::Unsupported),
            ("unobserved platform ABI", |p| p.operating_system = Probe::Unavailable, ui::ReadinessOutcome::Unknown),
            ("unfamiliar distro", |p| p.distro = Some("new-linux".into()), ui::ReadinessOutcome::Ready),
            ("unobserved distro", |p| {p.distro = None; p.model = None;}, ui::ReadinessOutcome::Ready),
            ("wrong architecture", |p| p.arch = Probe::Observed("aarch64".into()), ui::ReadinessOutcome::Unsupported),
            ("missing optional Bitwig", |p| p.bitwig_ref = Probe::Absent, ui::ReadinessOutcome::Ready),
            ("unavailable optional Bitwig", |p| p.bitwig_ref = Probe::Unavailable, ui::ReadinessOutcome::Ready),
            ("permission metadata is not access", |p| p.bitwig_permissions = Probe::Malformed, ui::ReadinessOutcome::Ready),
            ("unqualified DAW version", |p| p.bitwig_version = Probe::Observed("7.0".into()), ui::ReadinessOutcome::Ready),
            ("publication directory absent", |p| p.publication_directory = Probe::Absent, ui::ReadinessOutcome::ActionRequired),
            ("publication observation unavailable", |p| p.publication_directory = Probe::Unavailable, ui::ReadinessOutcome::Unknown),
            ("live callback negotiation", |p| p.daw_callback_maximum = Some(1024), ui::ReadinessOutcome::Ready),
            ("live device negotiation", |p| p.device_sample_rate = Some(44100), ui::ReadinessOutcome::Ready),
            ("selected display route", |p| p.display = None, ui::ReadinessOutcome::Unknown),
            ("JACK only", |p| p.pipewire_socket = Probe::Absent, ui::ReadinessOutcome::Ready),
            ("unavailable PipeWire with JACK", |p| p.pipewire_socket = Probe::Unavailable, ui::ReadinessOutcome::Ready),
            ("no audio route", |p| {p.pipewire_socket = Probe::Absent; p.jack_available = Probe::Absent;}, ui::ReadinessOutcome::ActionRequired),
            ("unavailable audio observations", |p| {p.pipewire_socket = Probe::Unavailable; p.jack_available = Probe::Absent;}, ui::ReadinessOutcome::Unknown),
            ("IPC runtime absent", |p| p.runtime_dir = Probe::Absent, ui::ReadinessOutcome::ActionRequired),
        ];
        for (name, change, expected) in cases {
            let mut p = p.clone();
            change(&mut p);
            let r = result(&s, &p);
            assert_eq!(r.overall_status, expected, "{name}");
            assert_eq!(r.products[0].status, expected, "{name}");
        }
    }
    #[test]
    fn unfamiliar_valid_publication_is_eligible_without_a_support_or_idle_daw_claim() {
        let (mut snapshot, mut platform) = fixture();
        let product = &mut snapshot.products[0];
        product.name = "Unfamiliar instrument".into();
        product.vendor = "Unfamiliar vendor".into();
        product.class_id = "ce".repeat(16);
        product.module_sha256 = "de".repeat(32);
        product.details["profile"] = Value::Null;
        platform.model = None;
        platform.distro = Some("unfamiliar-linux".into());
        platform.bitwig_ref = Probe::Absent;
        platform.device_sample_rate = None;
        platform.daw_callback_maximum = None;
        let assessment = result(&snapshot, &platform);
        assert_eq!(assessment.overall_status, ui::ReadinessOutcome::Ready);
        assert_eq!(assessment.products[0].installation_health, ui::InstallationHealth::Healthy);
        assert_eq!(assessment.products[0].support_qualification, ui::SupportQualification::NotYetQualified);
        assert!(assessment.daw.iter().find(|fact| fact.name == "Native DAW publication access")
            .unwrap().value.is_none());
        snapshot.products[0].details["host_valid"] = Value::Null;
        assert_eq!(result(&snapshot, &platform).products[0].status, ui::ReadinessOutcome::Unknown);
        snapshot.products[0].details["host_valid"] = json!(false);
        assert_eq!(result(&snapshot, &platform).products[0].status, ui::ReadinessOutcome::ActionRequired);
    }

    #[test]
    fn host_requirements_are_scoped_and_optional_display_is_not_an_audio_requirement() {
        let native = HostReadback { name:"Native fixture".into(), source:"test observation".into(),
            adapter:Probe::Observed(true), publication:Probe::Observed(true), ipc:Probe::Observed(true),
            pipewire:Probe::Absent, jack:Probe::Observed(true), display:Probe::Unavailable };
        assert!(host_requirements(&native, false).is_empty());
        let interactive = host_requirements(&native, true);
        assert_eq!(interactive.len(), 1);
        assert_eq!(interactive[0].category, "host_display");
        assert_eq!(interactive[0].status, ui::ReadinessOutcome::Unknown);
        let mut sandbox = native.clone();
        sandbox.name = "Sandbox fixture".into();
        sandbox.publication = Probe::Absent;
        sandbox.ipc = Probe::Unavailable;
        sandbox.jack = Probe::Absent;
        let missing = host_requirements(&sandbox, false);
        assert!(missing.iter().any(|item| item.category == "publication_path"
            && item.status == ui::ReadinessOutcome::ActionRequired));
        assert!(missing.iter().any(|item| item.category == "host_ipc"
            && item.status == ui::ReadinessOutcome::Unknown));
        assert!(missing.iter().any(|item| item.category == "host_audio"
            && item.status == ui::ReadinessOutcome::ActionRequired));
        let (snapshot, mut platform) = fixture();
        platform.hosts = vec![native, sandbox];
        let assessment = result(&snapshot, &platform);
        // There is no selected host in this idle overview; an unused sandbox's
        // missing access must not block preparing or trying the valid publication.
        assert_eq!(assessment.overall_status, ui::ReadinessOutcome::Ready);
        assert_eq!(assessment.daw.iter().find(|fact| fact.name == "Sandbox fixture publication access")
            .unwrap().value.as_deref(), Some("unavailable"));
    }

    #[cfg(target_os = "linux")]
    fn consumer_fixture() -> (crate::test_fixture::Fixture, PathBuf, PathBuf, String) {
        use std::os::unix::{fs::{symlink, MetadataExt}, net::UnixListener};
        let f = crate::test_fixture::Fixture::new();
        let sid = "42".repeat(16);
        let proc_root = f.outer.join("proc");
        let transport = f.outer.join("transport");
        let process = proc_root.join("41");
        for directory in [process.join("ns"), proc_root.join("self/ns"),
            proc_root.join("sys/kernel/random"), transport.clone(), f.m.publications.clone(),
            f.m.root.join("runtime/leases"), f.m.root.join("runtime/results"),
            f.m.root.join("runtime/lease-generations")] { private_dir(&directory).unwrap(); }
        symlink("/", process.join("root")).unwrap();
        symlink("mnt:[1]", process.join("ns/mnt")).unwrap();
        symlink("mnt:[1]", proc_root.join("self/ns/mnt")).unwrap();
        symlink("ipc:[1]", process.join("ns/ipc")).unwrap();
        symlink("ipc:[1]", proc_root.join("self/ns/ipc")).unwrap();
        let mut stat = vec!["0"; 20]; stat[0] = "S"; stat[19] = "900";
        fs::write(process.join("stat"), format!("41 (unknown native consumer) {}", stat.join(" "))).unwrap();
        let meta = fs::metadata(&f.r.native.path).unwrap();
        fs::write(process.join("maps"), format!("10-20 r-xp 0 {:x}:{:x} {} unknown\n",
            libc::major(meta.dev()), libc::minor(meta.dev()), meta.ino())).unwrap();
        fs::write(proc_root.join("sys/kernel/random/boot_id"), "11111111-1111-4111-8111-111111111111\n").unwrap();
        let bundle = f.outer.join("bundle"); private_dir(&bundle).unwrap();
        symlink(&bundle, f.m.link(&f.r.metadata.class_id)).unwrap();
        let mut registry = Registry::default();
        registry.classes.insert(f.r.key(), Entry {registration:f.r.clone(), publication:Publication::Published, managed_revision:None});
        atomic_json(&f.m.root.join("registry.json"), &registry).unwrap();
        let report = f.m.root.join("runtime/results").join(format!("windows-{sid}.json"));
        let owner_dir = f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions").join(&sid);
        private_dir(&owner_dir).unwrap();
        let lease = f.m.root.join("runtime/leases").join(format!("{sid}.json"));
        atomic_json(&lease, &report).unwrap();
        let owner = owner_dir.join("owner.json");
        atomic_json(&owner, &json!({"session":sid,"report":report,"lease":lease,
            "registration":HostBinding::from(f.r.clone()),"keeper":false,"inspect":false,
            "graphical_session":{"schema":1,"peer_pid":41,"peer_start_ticks":900,"display":":99"}})).unwrap();
        f.m.retain_lease_owner(&owner).unwrap();
        atomic_json(&f.m.root.join("runtime/lease-generations").join(format!("{sid}.json")),
            &json!({"schema":1,"session":sid,"report":report,"kernel_boot":"11111111-1111-4111-8111-111111111111","basis":"before_launch"})).unwrap();
        // A real socket under fixture custody distinguishes IPC visibility from
        // an arbitrary regular file. It is not a DAW/audio success receipt.
        let _socket = UnixListener::bind(transport.join("fixture.sock")).unwrap();
        (f, proc_root, transport, sid)
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn consumer_collection_requires_current_boot_process_and_exact_proxy_binding() {
        let (f, proc_root, transport, _) = consumer_fixture();
        let collect = || collect_consumers_from(&f.m, &Probe::Absent, &Probe::Observed(true), &proc_root, &transport);
        let hosts = collect().unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].name, "Active native DAW");
        assert_eq!(hosts[0].publication, Probe::Observed(true));
        assert_eq!(hosts[0].ipc, Probe::Observed(true));
        assert_eq!(hosts[0].jack, Probe::Observed(true));
        let stat_path = proc_root.join("41/stat");
        let original = fs::read_to_string(&stat_path).unwrap();
        fs::write(&stat_path, original.replace("900", "901")).unwrap();
        assert!(collect().unwrap_err().to_string().contains("consumer_generation"));
        fs::write(&stat_path, original).unwrap();
        fs::write(proc_root.join("41/maps"), "10-20 r-xp 0 00:00 1 foreign\n").unwrap();
        assert!(collect().unwrap_err().to_string().contains("consumer_proxy_mapping"));
        fs::write(proc_root.join("sys/kernel/random/boot_id"), "22222222-2222-4222-8222-222222222222\n").unwrap();
        assert!(collect().unwrap_err().to_string().contains("consumer_boot_changed"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn sandbox_consumer_cannot_inherit_host_path_or_jack_access() {
        let (f, proc_root, transport, _) = consumer_fixture();
        let root = proc_root.join("41/root");
        fs::remove_file(&root).unwrap(); private_dir(&root).unwrap();
        fs::write(root.join(".flatpak-info"), "[Application]\nname=unknown.fixture\n").unwrap();
        fs::remove_file(proc_root.join("41/ns/mnt")).unwrap();
        std::os::unix::fs::symlink("mnt:[2]", proc_root.join("41/ns/mnt")).unwrap();
        let hosts = collect_consumers_from(&f.m, &Probe::Absent, &Probe::Observed(true), &proc_root, &transport).unwrap();
        assert_eq!(hosts[0].name, "Active sandboxed DAW");
        assert_eq!(hosts[0].publication, Probe::Absent);
        assert_eq!(hosts[0].ipc, Probe::Absent);
        assert_eq!(hosts[0].jack, Probe::Unavailable);
        fs::remove_file(proc_root.join("41/ns/mnt")).unwrap();
        let hosts = collect_consumers_from(&f.m, &Probe::Absent, &Probe::Observed(true), &proc_root, &transport).unwrap();
        assert_eq!(hosts[0].adapter, Probe::Unavailable);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn consuming_path_resolves_absolute_symlinks_inside_consumer_root() {
        let f = crate::test_fixture::Fixture::new();
        let root = f.outer.join("isolated-root"); private_dir(&root).unwrap();
        let host_path = f.outer.join("exposed");
        std::os::unix::fs::symlink(&f.r.native.path, &host_path).unwrap();
        let peer_path = root.join(host_path.strip_prefix("/").unwrap());
        private_dir(peer_path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&f.r.native.path, &peer_path).unwrap();
        // Naive metadata follows the absolute target into the observer's root.
        assert!(fs::metadata(&peer_path).is_ok());
        assert_eq!(same_visible_object(&host_path, &root), Probe::Absent);
        let visible_target = root.join(f.r.native.path.strip_prefix("/").unwrap());
        fs::hard_link(&f.r.native.path, &visible_target).unwrap();
        assert_eq!(same_visible_object(&host_path, &root), Probe::Observed(true));
        fs::remove_file(&visible_target).unwrap(); fs::write(&visible_target, "different object").unwrap();
        assert_eq!(same_visible_object(&host_path, &root), Probe::Observed(false));
    }
    #[test]
    fn exact_artifacts_transactions_and_cleanup_are_requirements_but_withdrawal_is_qualification() {
        let (s, p) = fixture();
        for key in [
            "module_valid",
            "environment_valid",
            "runner_valid",
            "native_valid",
            "host_valid",
            "publication_valid",
            "performance_valid",
        ] {
            let mut s = s.clone();
            s.products[0].details[key] = json!(false);
            let r = result(&s, &p);
            assert_eq!(
                r.overall_status,
                ui::ReadinessOutcome::ActionRequired,
                "{key}"
            );
        }
        let mut s = s.clone();
        s.products[0].details["profile"]["claim"] = json!("withdrawn");
        assert_eq!(
            result(&s, &p).products[0].support_qualification,
            ui::SupportQualification::Unsupported
        );
        assert_eq!(result(&s, &p).overall_status, ui::ReadinessOutcome::Ready);
        s.products[0].details["profile"]["claim"] = json!("verified_exact_fixture");
        s.system.pending_transactions = 1;
        assert_eq!(
            result(&s, &p).overall_status,
            ui::ReadinessOutcome::ActionRequired
        );
        s.system.pending_transactions = 0;
        s.system.cleanup_unconfirmed = true;
        assert_eq!(
            result(&s, &p).overall_status,
            ui::ReadinessOutcome::ActionRequired
        );
        s.system.cleanup_unconfirmed = false;
        s.products[0].disposition = "quarantined".into();
        assert_eq!(
            result(&s, &p).overall_status,
            ui::ReadinessOutcome::ActionRequired
        );
        s.products[0].details = json!({"quarantine_reason":"scanner refusal"});
        assert_eq!(
            result(&s, &p).overall_status,
            ui::ReadinessOutcome::ActionRequired
        );
    }
    #[test]
    fn support_profile_identity_does_not_replace_current_execution_validation() {
        let (mut s, p) = fixture();
        assert_eq!(result(&s, &p).overall_status, ui::ReadinessOutcome::Ready);
        s.products[0].module_sha256 = "00".repeat(32);
        assert_eq!(result(&s, &p).products[0].support_qualification, ui::SupportQualification::NotYetQualified);
        assert_eq!(result(&s, &p).overall_status, ui::ReadinessOutcome::Ready);
        s.products[0].details["module_valid"] = json!(false);
        assert_eq!(result(&s, &p).overall_status, ui::ReadinessOutcome::ActionRequired);
    }
    #[test]
    fn physically_absent_publication_can_be_valid_without_being_ready() {
        let (mut snapshot, platform) = fixture();
        snapshot.products[0].disposition = "needs_attention".into();
        snapshot.products[0].details["publication_selected"] = json!(false);
        let assessment = result(&snapshot, &platform);
        assert_eq!(assessment.overall_status, ui::ReadinessOutcome::ActionRequired);
        assert_eq!(assessment.products[0].status, ui::ReadinessOutcome::ActionRequired);
        assert_eq!(assessment.products[0].installation_health,
            ui::InstallationHealth::Healthy);
        assert_eq!(assessment.products[0].support_qualification,
            ui::SupportQualification::Verified);
        assert!(assessment.products[0].reason.contains("not currently made available"));
    }
    #[test]
    fn mixed_roster_keeps_product_outcomes_separate_from_machine_readiness() {
        use ui::ReadinessOutcome as O;
        let (mut s, mut p) = fixture();
        let ready = s.products[0].clone();
        let mut other = ready.clone();
        other.name = "Another build".into();
        other.class_id = "ab".repeat(16);
        other.module_sha256 = "cd".repeat(32);
        other.details["profile"]["id"] = json!("unqualified-profile");
        s.products.push(other.clone());
        let unknown = result(&s, &p);
        assert_eq!(unknown.overall_status, O::Ready);
        assert_eq!(unknown.products[0].status, O::Ready);
        assert_eq!(unknown.products[1].status, O::Ready);
        s.products[1].details["module_valid"] = json!(false);
        let broken_unqualified = result(&s, &p);
        assert_eq!(broken_unqualified.products[1].status, O::ActionRequired);
        assert_eq!(broken_unqualified.products[1].installation_health,
            ui::InstallationHealth::ActionRequired);
        assert_eq!(broken_unqualified.products[1].support_qualification,
            ui::SupportQualification::NotYetQualified);
        assert_eq!(broken_unqualified.overall_status, O::Ready);
        s.products[1].details["module_valid"] = json!(true);

        s.products[1].details["profile"]["id"] = ready.details["profile"]["id"].clone();
        s.products[1].details["profile"]["claim"] = json!("withdrawn");
        let unsupported = result(&s, &p);
        assert_eq!(unsupported.overall_status, O::Ready);
        assert_eq!(unsupported.products[0].status, O::Ready);
        assert_eq!(unsupported.products[1].support_qualification, ui::SupportQualification::Unsupported);
        assert_eq!(unsupported.products[1].status, O::Ready);

        s.products[1].disposition = "quarantined".into();
        let quarantined = result(&s, &p);
        assert_eq!(quarantined.overall_status, O::Ready);
        assert_eq!(quarantined.products[0].status, O::Ready);
        assert_eq!(quarantined.products[1].status, O::ActionRequired);

        let second = profiles::ap17_profiles().unwrap().remove(1);
        let mut second_ready = ready.clone();
        second_ready.name = second.class.name;
        second_ready.class_id = second.class.class_id;
        second_ready.version = second.class.version;
        second_ready.module_sha256 = second.module_sha256;
        second_ready.details["profile"]["id"] = json!(second.id);
        second_ready.details["profile"]["revision"] = json!(second.revision);
        s.products.insert(1, second_ready);
        s.products[2].disposition = "ready".into();
        let two_and_withdrawn = result(&s, &p);
        assert_eq!(two_and_withdrawn.overall_status, O::Ready);
        assert_eq!(
            two_and_withdrawn
                .products
                .iter()
                .filter(|p| p.status == O::Ready)
                .count(),
            3
        );
        assert_eq!(two_and_withdrawn.products[2].support_qualification, ui::SupportQualification::Unsupported);

        s.products.retain(|product| product.name == "Another build");
        s.products[0].details["profile"]["id"] = json!("unqualified-profile");
        s.products[0].details["profile"]["claim"] = json!("verified_exact_fixture");
        let only_unknown = result(&s, &p);
        assert_eq!(only_unknown.overall_status, O::Ready);
        assert_eq!(only_unknown.products[0].status, O::Ready);
        assert_eq!(only_unknown.products[0].support_qualification, ui::SupportQualification::NotYetQualified);

        s.products = vec![ready];
        p.arch = Probe::Observed("aarch64".into());
        let wrong_arch = result(&s, &p);
        assert_eq!(wrong_arch.overall_status, O::Unsupported);
        assert_ne!(wrong_arch.products[0].status, O::Ready);
    }
    #[test]
    fn stale_discovered_product_needs_attention_even_outside_beta_envelope() {
        use ui::ReadinessOutcome as O;
        let (mut snapshot, platform)=fixture();
        let mut discovered=snapshot.products[0].clone();
        discovered.class_id="ab".repeat(16);
        discovered.name="Discovered fixture".into();
        discovered.disposition="installed_unqualified".into();
        discovered.details=json!({"scan":"cd".repeat(16),"current":true});
        snapshot.products.push(discovered);
        let current=result(&snapshot,&platform);
        assert_eq!(current.products[1].installation_health,ui::InstallationHealth::Unknown);
        assert_eq!(current.products[1].status,O::Unknown);
        assert_eq!(current.overall_status,O::Ready);
        snapshot.products[1].details["current"]=json!(false);
        snapshot.products[1].disposition="needs_attention".into();
        let stale=result(&snapshot,&platform);
        assert_eq!(stale.products[1].installation_health,ui::InstallationHealth::ActionRequired);
        assert_eq!(stale.products[1].status,O::ActionRequired);
        assert!(stale.products[1].reason.contains("inventory is stale"));
        assert_eq!(stale.overall_status,O::Ready);
        snapshot.products[1].disposition="quarantined".into();
        let quarantined=result(&snapshot,&platform);
        assert_eq!(quarantined.products[1].status,O::ActionRequired);
        assert!(quarantined.products[1].reason.contains("quarantined"));
    }
    #[test]
    fn captured_beta_profile_requires_completed_publication_and_512_reserve() {
        let (prepared_fixture,mut profile,census,native)=test_fixture::prepared();
        profile.id="arturia-pure-lofi".into();
        let revision=publication::Revision {schema:1,id:"ab".repeat(16),
            class_id:profile.class.class_id.clone(),external_ids:native.external_ids,
            profile:profile.clone(),profile_sha256:"cd".repeat(32),census,
            registration:prepared_fixture.r.clone(),performance:Performance::default(),
            target:prepared_fixture.outer.join("target"),parent:None,transaction:"ef".repeat(16),
            adopted_legacy:false,qualification:None};
        let (snapshot,_)=fixture();
        let mut product=snapshot.products[0].clone();
        product.class_id=profile.class.class_id.clone();
        product.version=profile.class.version.clone();
        product.module_sha256=profile.module_sha256.clone();
        product.runner=profile.requirements.runner.id.clone();
        product.details=json!({"profile":{"id":profile.id,"revision":profile.revision},
            "qualification":null,"environment_revision":profile.requirements.environment_revision,
            "host_sha256":profile.requirements.host_sha256,
            "host_source_sha256":profile.requirements.host_source_sha256,
            "native_sha256":profile.requirements.native_sha256,
            "publication_complete":true,"added_frames":512});
        assert!(accepted_profile_captured(std::slice::from_ref(&profile),&revision,&product));
        product.details["added_frames"]=json!(256);
        assert!(!accepted_profile_captured(std::slice::from_ref(&profile),&revision,&product));
        product.details["added_frames"]=json!(512);
        product.details["publication_complete"]=json!(false);
        assert!(!accepted_profile_captured(std::slice::from_ref(&profile),&revision,&product));
    }
    #[test]
    fn authority_read_failure_is_distinct_from_unqualified_product() {
        use ui::ReadinessOutcome as O;
        let (s, p) = fixture();
        for code in [
            "READINESS_REGISTRY_UNAVAILABLE",
            "READINESS_PROFILE_AUTHORITY_UNAVAILABLE",
            "READINESS_REVISION_UNAVAILABLE",
        ] {
            let r = resolve_with(&s, &p, 123, &[code], |_| Err(code));
            assert_eq!(r.overall_status, O::Unknown);
            assert_eq!(r.products[0].status, O::Unknown);
            assert_eq!(r.products[0].failure_code.as_deref(), Some(code));
            assert!(r.blockers.iter().any(|b| b.category == code));
            assert!(r.ordered_steps[0].title.contains("observations"));
        }
        let f = crate::test_fixture::Fixture::new();
        let assessment = resolve_with(&s, &p, 123,
            &["READINESS_REGISTRY_UNAVAILABLE"],
            |_| Err("READINESS_REGISTRY_UNAVAILABLE"));
        let receipt = write_export(&f.m, &s, sanitized(assessment), &support_software(&f), None)
            .unwrap();
        let report: Value = read_json(&f.m.root.join("support-exports")
            .join(format!("{}.json",receipt["export"].as_str().unwrap()))).unwrap();
        assert_eq!(report["current_error"], "READINESS_REGISTRY_UNAVAILABLE");
    }
    #[test]
    fn vendor_attention_and_old_wire_shape_remain_truthful() {
        let (mut s, p) = fixture();
        s.onboarding.push(ui::Onboarding {
            failure: None,
            installer: "ab".repeat(32),
            name: "Vendor setup".into(),
            byte_size: 0,
            format: "exe".into(),
            environment: None,
            state: "needs_user_action".into(),
            required_human_action: "Sign in with the vendor".into(),
            details: Value::Null,
            actions: vec![],
        });
        let assessment = result(&s, &p);
        assert_eq!(
            assessment.overall_status,
            ui::ReadinessOutcome::ActionRequired
        );
        assert!(assessment
            .blockers
            .iter()
            .any(|b| b.category == "vendor_action"));
        assert_eq!(s.schema, ui::OPERATOR_SCHEMA);
        let raw = serde_json::to_value(&s).unwrap();
        assert!(raw.get("readiness").is_none());
        assert!(s
            .actions
            .iter()
            .all(|offer| !matches!(offer.action, ui::Action::SupportExport {})));
        let _: ui::Snapshot = serde_json::from_value(raw).unwrap();
    }
    #[test]
    fn support_export_removes_sensitive_free_text_and_action_arguments() {
        let (mut s, p) = fixture();
        s.products[0].name = "https://example.test/?token=secret".into();
        let mut assessment = result(&s, &p);
        assessment.platform[0].value = Some("/home/private/person".into());
        assessment.ordered_steps.push(step(
            "user@example.com",
            "token?secret",
            Some(ui::AvailableAction {
                label: "Run".into(),
                action: ui::Action::EnvironmentRescan {
                    environment: "exact".into(),
                },
                disabled_reason: None,
            }),
        ));
        let sanitized = sanitized(assessment);
        let raw = serde_json::to_string(&sanitized).unwrap();
        for secret in [
            "token=secret",
            "/home/private",
            "user@example.com",
            "token?secret",
            "\"environment\":\"exact\"",
        ] {
            assert!(!raw.contains(secret));
        }
        assert_eq!(sanitized.products[0].name, "[redacted]");
        assert!(sanitized.suggested_step_title.is_some());
        assert!(!raw.contains("state_token"));
        assert!(!raw.contains("support_export_action"));
        assert!(!raw.contains("\"kind\""));
    }
    #[test]
    fn local_export_is_parseable_and_contains_only_allowlisted_incident_fields() {
        let f = crate::test_fixture::Fixture::new();
        let (mut s, p) = fixture();
        s.recent_incidents.push(ui::Incident {
            id: "ab".repeat(16),
            state: "retired".into(),
            summary: json!({"category":"host_exit","credential":"top-secret",
                "log":"/home/operator/private"}),
            export: None,
        });
        let assessment = sanitized(result(&s, &p));
        let receipt = write_export(&f.m, &s, assessment, &support_software(&f), None).unwrap();
        let id = receipt["export"].as_str().unwrap();
        let file = f.m.root.join("support-exports").join(format!("{id}.json"));
        let report: Value = read_json(&file).unwrap();
        assert_eq!(report["schema"], 2);
        assert_eq!(report["manager_sha256"], f.r.host.sha256);
        assert_eq!(report["frontend_sha256"], f.r.host.sha256);
        assert_eq!(report["refused_step"], Value::Null);
        assert_eq!(report["current_error"], Value::Null);
        assert_eq!(report["incidents"][0]["category"], "host_exit");
        let raw = std::fs::read_to_string(&file).unwrap();
        assert!(!raw.contains("top-secret"));
        assert!(!raw.contains("/home/operator"));
        assert!(!raw.contains("credential"));
        for forbidden in ["state_token", "support_export_action", "\"kind\"",
            "\"action\"", "\"disabled_reason\"", "/Users/", "/home/",
            "compatdata/pfx", "token?", "token="] {
            assert!(!raw.contains(forbidden), "support export leaked {forbidden}");
        }
    }
}

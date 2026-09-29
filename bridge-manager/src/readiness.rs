//! PB0: bounded local facts and exact, read-only supported-system resolution.
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
    PlatformReadback {
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
        pipewire_socket: path_probe(&runtime.join("pipewire-0"), true),
        jack_available: match jack {
            Probe::Observed(_) => Probe::Observed(true),
            Probe::Absent => Probe::Absent,
            Probe::Unavailable => Probe::Unavailable,
            Probe::Malformed => Probe::Malformed,
        },
        pipewire_graph_rate: metadata.observed().and_then(|v| pipewire_setting(v, "clock.rate")),
        pipewire_quantum: metadata.observed().and_then(|v| pipewire_setting(v, "clock.quantum")),
        device_sample_rate: None,
        daw_callback_maximum: None,
        bitwig_ref,
        bitwig_version: version,
        bitwig_runtime: nonempty(bitwig_runtime),
        bitwig_permissions: nonempty(bitwig_permissions),
        publication_directory: path_probe(&m.publications, false),
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
    p.arch.observed().map(String::as_str) == Some("x86_64")
        && p.model.as_deref() == Some(DECK_MODEL)
        && p.distro.as_deref() == Some(DECK_DISTRO)
        && p.distro_version.as_deref() == Some(DECK_VERSION)
        && p.bitwig_ref.observed().map(String::as_str) == Some(BITWIG_REF)
        && p.bitwig_version.observed().map(String::as_str) == Some(BITWIG_VERSION)
}
fn sandbox_path(p: &PlatformReadback) -> Option<bool> {
    p.bitwig_permissions.observed().and_then(|permissions| {
        if !permissions.lines().any(|line| line.trim() == "[Context]") { return None; }
        let filesystems = key_value(permissions, "filesystems")?;
        let sockets = key_value(permissions, "sockets")?;
        let publication_directory = match &p.publication_directory {
            Probe::Observed(value) => *value,
            Probe::Absent => false,
            Probe::Unavailable | Probe::Malformed => return None,
        };
        Some(publication_directory
            && filesystems.split(';').any(|entry| entry == "host")
            && sockets.split(';').any(|entry| entry == "x11"))
    })
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
    } else if p.distro.is_none() || p.distro_version.is_none() || p.model.is_none() {
        blockers.push(issue(
            "platform",
            O::Unknown,
            "Machine or distribution identity is unavailable.",
        ));
    } else if p.distro.as_deref() != Some(DECK_DISTRO)
        || p.distro_version.as_deref() != Some(DECK_VERSION)
        || p.model.as_deref() != Some(DECK_MODEL)
    {
        blockers.push(issue("platform", O::Unknown,
            "This exact distribution, version and hardware combination has not been qualified for these profiles."));
    }
    if p.bitwig_ref.is_absent() {
        blockers.push(issue(
            "daw",
            O::ActionRequired,
            "Bitwig Studio Flatpak is not visible to the manager.",
        ));
        steps.push(step(
            "Install or verify Bitwig Studio",
            "Use the supported Bitwig Flatpak route, then Check again.",
            None,
        ));
    } else if p.bitwig_ref.observed().map(String::as_str).is_none()
        || p.bitwig_version.observed().map(String::as_str).is_none() {
        blockers.push(issue("daw", O::Unknown,
            "Bitwig installation or version could not be observed reliably."));
    } else if p.bitwig_ref.observed().map(String::as_str) != Some(BITWIG_REF)
        || p.bitwig_version.observed().map(String::as_str) != Some(BITWIG_VERSION)
    {
        blockers.push(issue(
            "daw",
            O::Unknown,
            "The installed DAW build or Flatpak branch is outside the accepted Bitwig 6.1 fixture.",
        ));
    }
    match sandbox_path(p) {
        Some(true) => {}
        Some(false) => {
            blockers.push(issue("publication_path", O::ActionRequired,
                "Bitwig's current Flatpak permissions do not establish access to the managed VST3 path and X11 editor socket."));
            steps.push(step("Review Bitwig Flatpak access",
                "Use the supported application permissions; the manager will not widen the sandbox automatically.", None));
        }
        None => blockers.push(issue(
            "publication_path",
            O::Unknown,
            "Bitwig sandbox permissions could not be read.",
        )),
    }
    if p.display.is_none() || p.session.is_none() {
        blockers.push(issue(
            "graphics",
            O::Unknown,
            "The current graphical session or X11 display cannot be confirmed.",
        ));
    } else if !matches!(p.session.as_deref(), Some("wayland" | "x11")) {
        blockers.push(issue(
            "graphics",
            O::ActionRequired,
            "The selected profiles require a Desktop session with X11/XWayland editor access.",
        ));
        steps.push(step(
            "Use a compatible Desktop session",
            "Return to SteamOS Desktop Mode, then Check again.",
            None,
        ));
    }
    if p.runtime_dir.is_absent() || p.pipewire_socket.is_absent() {
        blockers.push(issue(
            "audio_ipc",
            O::ActionRequired,
            "The user runtime directory or PipeWire socket is unavailable.",
        ));
        steps.push(step(
            "Restore the desktop audio session",
            "Sign in to the normal desktop session, then Check again.",
            None,
        ));
    } else if p.runtime_dir.observed() != Some(&true)
        || p.pipewire_socket.observed() != Some(&true) {
        blockers.push(issue("audio_ipc", O::Unknown,
            "The user runtime directory or PipeWire socket could not be verified."));
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
        let beta_profile = product.details["profile"]["id"]
            .as_str()
            .is_some_and(|id| BETA_PROFILE_IDS.contains(&id));
        let acceptance = beta_profile.then(|| accepted(product));
        let authority_failure = acceptance
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .copied();
        let exact = acceptance.is_some_and(|result| result == Ok(true));
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
                "This product is not currently made available to Bitwig.")
        } else if authority_failure.is_some() {
            (
                O::Unknown,
                "Installed compatibility authority could not be verified for this product.",
            )
        } else if support == ui::SupportQualification::Unsupported {
            (O::Unsupported, "This exact profile has been withdrawn.")
        } else if !beta_profile {
            (O::Unknown,"This product has no accepted PB0 support envelope; its current installation health is shown separately.")
        } else if exact && product.disposition == "ready" && has_deck {
            (O::Ready,"Exact ordinary Arturia profile and managed artifacts verify on the accepted Deck fixture.")
        } else {
            (
                O::Unknown,
                "This build and machine combination has no accepted PB0 support envelope.",
            )
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
            "Add a supported plug-in",
            "Open Setup to import the lawful Windows installer.",
            None,
        ));
    }
    // The device rate and Bitwig's internal callback maximum are not reliably
    // readable from an idle manager. Graph metadata never substitutes for them.
    if any_ready
        && (p.device_sample_rate != Some(48000)
            || p.daw_callback_maximum.is_none_or(|maximum| maximum > 512))
    {
        let unknown = p.device_sample_rate.is_none() || p.daw_callback_maximum.is_none();
        blockers.push(issue("host_audio", O::ActionRequired,if unknown {
            "Bitwig's current device rate or callback maximum is not confirmed outside a live session."
        } else {
            "The observed device rate or Bitwig callback maximum does not match the accepted 48 kHz / at-most-512-frame profile."
        }));
        steps.push(step("Verify Bitwig audio settings",
            if unknown {
                "In Bitwig Audio settings, confirm 48 kHz and a maximum of at most 512 frames. The idle manager cannot verify these values yet, so this assessment remains action required. PipeWire graph values are not a substitute."
            } else {
                "Set the exact device rate and Bitwig callback maximum to the qualified values, then Check again. The manager does not change DAW settings."
            },None));
        for product in &mut products {
            if product.status == O::Ready {
                product.status = O::ActionRequired;
                product.reason="Exact profile and artifacts verify; current Bitwig audio settings still need confirmation.".into();
            }
        }
    }
    // Product-specific outcomes remain on their product. A withdrawn profile,
    // quarantine, or unqualified sibling cannot invalidate a verified system
    // or another exact supported product.
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
        steps=vec![step("Review supported configurations",
            "This exact configuration crosses an accepted negative support boundary. No setup mutation is offered.",None)];
    } else if overall == O::Unknown {
        steps=vec![step("Review exact compatibility",
            "The manager has no accepted support claim for this exact combination. Review Details or create a support export.",None)];
    } else {
        steps.truncate(1);
    }
    if !system_statuses.is_empty() {
        for product in &mut products {
            if product.status == O::Ready {
                product.status = overall;
                product.reason =
                    "Exact profile and artifacts verify; the current system is not fully ready."
                        .into();
            }
        }
    }
    let source = "accepted SteamOS 3.8.16 / Bitwig 6.1 Arturia profile envelope";
    let required = |name: &str, value: &str| {
        fact(
            name,
            Some(value.into()),
            source,
            at,
            ui::FactCertainty::ProfileRequired,
        )
    };
    ui::ReadinessAssessment {
        schema: 1,
        state_token: snapshot.state_token.clone(),
        observed_at: at,
        overall_status: overall,
        system: snapshot.system.clone(),
        platform: vec![
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
            required("Qualified distribution", "SteamOS 3.8.16"),
        ],
        daw: vec![
            observed("DAW Flatpak ref", p.bitwig_ref.value(), "flatpak info", at),
            observed("DAW version", p.bitwig_version.value(), "flatpak info", at),
            observed("DAW runtime", p.bitwig_runtime.value(), "flatpak info", at),
            observed(
                "Sandbox publication access",
                sandbox_path(p).map(|b| if b { "available" } else { "unavailable" }.into()),
                "Flatpak permissions and managed publication directory",
                at,
            ),
            required("Qualified DAW", "Bitwig Studio 6.1 Flatpak"),
        ],
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
                "Bitwig callback maximum",
                p.daw_callback_maximum.map(|n| n.to_string()),
                "trusted live DAW readback; unavailable when idle",
                at,
            ),
            required("Qualified device sample rate", "48000 Hz"),
            required("Qualified host maximum", "at most 512 frames"),
            observed(
                "Bridge processing quantum",
                None,
                "not exposed in current operator snapshot",
                at,
            ),
            required("Bridge presentation reserve", "512 added frames"),
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
                "Graphics driver",
                None,
                "no profile-required driver probe",
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
    let assessment = overview.readiness.clone();
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
        Software { installer_launch: None, preparation_kit: None,
            operator_frontend: Some(artifact.clone()), manager: artifact.clone(),
            supervisor: artifact.clone(), ownership: artifact.clone(),
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
            "publication_complete":true,"performance_valid":true,"added_frames":512,
            "qualification":null});
        snapshot.onboarding.clear();
        snapshot.system.service = "active".into();
        snapshot.system.dsp = 0;
        snapshot.system.maintenance = 0;
        snapshot.system.pending_transactions = 0;
        snapshot.system.stale_transports = 0;
        snapshot.system.cleanup_unconfirmed = false;
        let platform = PlatformReadback {
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
        assert_eq!(idle.overall_status, ui::ReadinessOutcome::ActionRequired);
        assert_eq!(
            idle.products[0].status,
            ui::ReadinessOutcome::ActionRequired
        );
        assert_eq!(idle.ordered_steps.len(), 1);
        assert!(idle.ordered_steps[0].title.contains("Bitwig audio"));
        assert!(idle
            .audio
            .iter()
            .find(|f| f.name == "Bitwig callback maximum")
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
            (
                "unknown distro",
                |p| p.distro = Some("new-linux".into()),
                ui::ReadinessOutcome::Unknown,
            ),
            (
                "wrong architecture",
                |p| p.arch = Probe::Observed("aarch64".into()),
                ui::ReadinessOutcome::Unsupported,
            ),
            (
                "missing daw",
                |p| p.bitwig_ref = Probe::Absent,
                ui::ReadinessOutcome::ActionRequired,
            ),
            (
                "unavailable daw probe",
                |p| p.bitwig_ref = Probe::Unavailable,
                ui::ReadinessOutcome::Unknown,
            ),
            (
                "malformed permissions",
                |p| p.bitwig_permissions = Probe::Malformed,
                ui::ReadinessOutcome::Unknown,
            ),
            (
                "incomplete permission observation",
                |p| p.bitwig_permissions = Probe::Observed("[Context]\nfilesystems=host;".into()),
                ui::ReadinessOutcome::Unknown,
            ),
            (
                "wrong daw version",
                |p| p.bitwig_version = Probe::Observed("7.0".into()),
                ui::ReadinessOutcome::Unknown,
            ),
            (
                "sandbox path",
                |p| p.publication_directory = Probe::Absent,
                ui::ReadinessOutcome::ActionRequired,
            ),
            (
                "callback maximum",
                |p| p.daw_callback_maximum = Some(1024),
                ui::ReadinessOutcome::ActionRequired,
            ),
            (
                "device rate",
                |p| p.device_sample_rate = Some(44100),
                ui::ReadinessOutcome::ActionRequired,
            ),
            (
                "graphics",
                |p| p.display = None,
                ui::ReadinessOutcome::Unknown,
            ),
            (
                "ipc",
                |p| p.pipewire_socket = Probe::Absent,
                ui::ReadinessOutcome::ActionRequired,
            ),
            (
                "unavailable ipc observation",
                |p| p.pipewire_socket = Probe::Unavailable,
                ui::ReadinessOutcome::Unknown,
            ),
        ];
        for (name, change, expected) in cases {
            let mut p = p.clone();
            change(&mut p);
            let r = result(&s, &p);
            assert_eq!(r.overall_status, expected, "{name}");
            assert_ne!(r.products[0].status, ui::ReadinessOutcome::Ready, "{name}");
        }
    }
    #[test]
    fn exact_artifacts_transactions_cleanup_and_withdrawal_are_not_ready() {
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
            result(&s, &p).products[0].status,
            ui::ReadinessOutcome::Unsupported
        );
        assert_eq!(result(&s, &p).overall_status, ui::ReadinessOutcome::Unknown);
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
    fn source_profile_and_current_exact_identity_are_both_required() {
        let (mut s, p) = fixture();
        assert_eq!(result(&s, &p).overall_status, ui::ReadinessOutcome::Ready);
        s.products[0].module_sha256 = "00".repeat(32);
        assert_eq!(
            result(&s, &p).products[0].status,
            ui::ReadinessOutcome::Unknown
        );
        assert_eq!(result(&s, &p).overall_status, ui::ReadinessOutcome::Unknown);
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
        assert_eq!(unknown.products[1].status, O::Unknown);
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
        assert_eq!(unsupported.products[1].status, O::Unsupported);

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
            2
        );
        assert_eq!(two_and_withdrawn.products[2].status, O::Unsupported);

        s.products.retain(|product| product.name == "Another build");
        s.products[0].details["profile"]["id"] = json!("unqualified-profile");
        s.products[0].details["profile"]["claim"] = json!("verified_exact_fixture");
        let only_unknown = result(&s, &p);
        assert_eq!(only_unknown.overall_status, O::Unknown);
        assert_eq!(only_unknown.products[0].status, O::Unknown);

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
            assert!(r.ordered_steps[0].title.contains("compatibility"));
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

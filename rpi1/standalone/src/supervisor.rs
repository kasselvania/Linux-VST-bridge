use crate::config::{Config, Runner};
use std::{
    collections::BTreeMap,
    ffi::{CStr, OsString},
    fs, io,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct Identity {
    pub unit: String,
    pub cgroup: PathBuf,
    pub main_pid: u32,
    pub main_start_ticks: u64,
}

#[derive(Clone, Debug)]
pub struct ProcessIdentity {
    pub pid: u32,
    pub start_ticks: u64,
    pub executable: PathBuf,
}

pub struct Cohort {
    identity: Identity,
    retired: bool,
    native_leader: Option<PathBuf>,
}

impl Cohort {
    pub fn launch(config: &Config, session: &str, host_arguments: &[OsString]) -> io::Result<Self> {
        validate_session(session)?;
        config.verify_files()?;
        let unit = format!("lvb-rpi1-{session}.service");
        if systemctl(&["is-active", &unit])?.success() {
            return Err(invalid("RPI1 unit already active"));
        }
        let mut command = Command::new("/usr/bin/systemd-run");
        command.args([
            "--user",
            "--collect",
            "--quiet",
            "--service-type=exec",
            "--property=KillMode=control-group",
            "--property=TimeoutStopSec=10s",
            "--property=SendSIGKILL=yes",
            &format!("--unit={unit}"),
        ]);
        let mut selected_environment = environment(config)?;
        let diagnostics = std::env::var("LVB_RPI1_DIAGNOSTICS").unwrap_or_default();
        selected_environment.extend(diagnostic_environment(config, session, &diagnostics)?);
        for (key, value) in selected_environment {
            command.arg(
                OsString::from(format!("--setenv={key}="))
                    .into_string()
                    .unwrap()
                    + &value,
            );
        }
        if matches!(config.runner, Runner::Native { .. }) {
            command.args([
                "--property=RuntimeMaxSec=300",
                "--property=MemoryMax=3G",
                "--property=TasksMax=512",
            ]);
        }
        command.args(launch_vector(config, host_arguments));
        require(
            command.status()?.success(),
            "systemd refused exact RPI1 cohort",
        )?;
        let end = Instant::now() + Duration::from_secs(10);
        let identity = loop {
            match inspect(&unit).and_then(|identity| {
                if let Runner::Native { leader, .. } = &config.runner {
                    let actual = fs::read_link(format!("/proc/{}/exe", identity.main_pid))?;
                    require(
                        fs::canonicalize(actual)? == fs::canonicalize(&leader.path)?,
                        "native launcher has not reached its pinned leader",
                    )?;
                }
                Ok(identity)
            }) {
                Ok(identity) => break identity,
                Err(_) if Instant::now() < end => thread::sleep(Duration::from_millis(20)),
                Err(error) => {
                    let _ = systemctl(&["stop", &unit]);
                    return Err(error);
                }
            }
        };
        let cohort = Self {
            identity,
            retired: false,
            native_leader: match &config.runner {
                Runner::Native { leader, .. } => Some(leader.path.clone()),
                Runner::Box64(_) => None,
            },
        };
        cohort.verify_startup_leader()?;
        Ok(cohort)
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// Bounded local liveness check during initialization; no systemctl child
    /// can stretch the startup observation deadline.
    pub(crate) fn verify_startup_leader(&self) -> io::Result<()> {
        require(
            process_start(self.identity.main_pid)? == self.identity.main_start_ticks,
            "RPI1 startup cohort leader changed",
        )?;
        if let Some(expected) = &self.native_leader {
            let actual = fs::read_link(format!("/proc/{}/exe", self.identity.main_pid))?;
            require(
                fs::canonicalize(actual)? == fs::canonicalize(expected)?,
                "native launcher leader executable changed",
            )?;
        }
        let members = fs::read_to_string(self.identity.cgroup.join("cgroup.procs"))?;
        require(
            members
                .lines()
                .any(|line| line.parse::<u32>().ok() == Some(self.identity.main_pid)),
            "RPI1 startup cohort leader exited",
        )
    }

    pub fn verify(&self) -> io::Result<Vec<ProcessIdentity>> {
        self.verify_startup_leader()?;
        let current = inspect(&self.identity.unit)?;
        if current.main_pid != self.identity.main_pid
            || current.main_start_ticks != self.identity.main_start_ticks
            || current.cgroup != self.identity.cgroup
        {
            return Err(invalid("RPI1 cohort identity changed"));
        }
        let text = fs::read_to_string(current.cgroup.join("cgroup.procs"))?;
        let mut processes = Vec::new();
        for line in text.lines() {
            if processes.len() >= 4096 {
                return Err(invalid("RPI1 cohort process bound"));
            }
            let pid = line
                .parse::<u32>()
                .map_err(|_| invalid("cgroup PID syntax"))?;
            let start_ticks = match process_start(pid) {
                Ok(value) => value,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            let executable = match fs::read_link(format!("/proc/{pid}/exe")) {
                Ok(value) => value,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            };
            match process_start(pid) {
                Ok(after) if after == start_ticks => {}
                Ok(_) => return Err(invalid("RPI1 cohort PID was recycled")),
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
            processes.push(ProcessIdentity {
                pid,
                start_ticks,
                executable,
            });
        }
        require(
            !processes.is_empty()
                && processes.iter().any(|process| {
                    process.pid == self.identity.main_pid
                        && process.start_ticks == self.identity.main_start_ticks
                }),
            "RPI1 cgroup lost main process",
        )?;
        Ok(processes)
    }

    pub fn retire(mut self) -> io::Result<()> {
        let status = systemctl(&["stop", &self.identity.unit])?;
        require(status.success(), "systemd stop failed")?;
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            let active = systemctl(&["is-active", &self.identity.unit])?;
            let empty = fs::read_to_string(self.identity.cgroup.join("cgroup.procs"))
                .map(|value| value.trim().is_empty())
                .unwrap_or(true);
            if !active.success() && empty {
                self.retired = true;
                return Ok(());
            }
            if Instant::now() >= end {
                return Err(invalid("RPI1 cgroup cleanup incomplete"));
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Cohort {
    fn drop(&mut self) {
        if !self.retired {
            // No name matching, wineserver-wide operation or foreign prefix action.
            let _ = systemctl(&["stop", &self.identity.unit]);
        }
    }
}

pub fn launch_vector(config: &Config, host_arguments: &[OsString]) -> Vec<OsString> {
    let Runner::Box64(runtime) = &config.runner else {
        let Runner::Native { launcher, .. } = &config.runner else {
            unreachable!()
        };
        let mut vector = vec![launcher.path.as_os_str().to_owned()];
        vector.extend_from_slice(host_arguments);
        return vector;
    };
    let mut command = vec![
        runtime.slr_entry.path.as_os_str().to_owned(),
        "--verb=run".into(),
        "--".into(),
        runtime.proton.path.as_os_str().to_owned(),
        "runinprefix".into(),
    ];
    command.extend_from_slice(host_arguments);
    command
}

pub fn environment(config: &Config) -> io::Result<BTreeMap<String, String>> {
    if matches!(config.runner, Runner::Native { .. }) {
        let mut values = BTreeMap::new();
        for (key, value) in [
            ("WINEPREFIX", config.prefix().to_string_lossy().into_owned()),
            ("DISPLAY", config.display.clone()),
            (
                "XAUTHORITY",
                config.xauthority.path.to_string_lossy().into_owned(),
            ),
            ("LANG", "C.UTF-8".into()),
            ("WINEDLLOVERRIDES", "uiautomationcore=".into()),
            (
                "LVB_EVENT_OUTPUT_POLICY",
                "reported_zero_event_channels_unspecified".into(),
            ),
            ("LVB_RPI0_ARCHITECTURE", "required".into()),
            (
                "LVB_EDITOR_LIFETIME",
                "retain_editor_view_until_instance_retirement".into(),
            ),
            (
                "LVB_VENDOR_RETIREMENT",
                "process_scoped_vendor_retirement".into(),
            ),
        ] {
            if value.bytes().any(|b| matches!(b, 0 | b'\n' | b'\r')) {
                return Err(invalid("native environment value"));
            }
            values.insert(key.to_owned(), value);
        }
        return Ok(values);
    }
    let Runner::Box64(runtime) = &config.runner else {
        unreachable!()
    };
    let (user, _) = user_identity()?;
    let root = &config.environment_root;
    let prefix = config.prefix();
    let path_value = |path: PathBuf| {
        path.into_os_string()
            .into_string()
            .map_err(|_| invalid("environment path encoding"))
    };
    let mut values = BTreeMap::new();
    for (key, value) in [
        ("HOME", path_value(root.join("home"))?),
        ("USER", user.clone()),
        ("LOGNAME", user),
        ("PATH", "/usr/bin:/bin".to_owned()),
        ("LANG", "C.UTF-8".to_owned()),
        (
            "XDG_RUNTIME_DIR",
            format!("/run/user/{}", unsafe { libc::getuid() }),
        ),
        ("DISPLAY", config.display.clone()),
        ("XAUTHORITY", path_value(config.xauthority.path.clone())?),
        ("PYTHONHOME", "/usr".to_owned()),
        ("WINEDLLOVERRIDES", "uiautomationcore=".to_owned()),
        ("STEAM_ZENITY", String::new()),
        ("STEAM_COMPAT_APP_ID", "0".to_owned()),
        ("SteamAppId", "0".to_owned()),
        ("SteamGameId", "0".to_owned()),
        (
            "STEAM_COMPAT_MACHINE_ARCHITECTURE",
            "aarch64-linux-gnu".to_owned(),
        ),
        ("XDG_CACHE_HOME", path_value(root.join("cache"))?),
        ("XDG_CONFIG_HOME", path_value(root.join("config"))?),
        ("XDG_DATA_HOME", path_value(root.join("data"))?),
        ("TMPDIR", path_value(root.join("tmp"))?),
        (
            "STEAM_COMPAT_CLIENT_INSTALL_PATH",
            path_value(root.join("steam-root"))?,
        ),
        (
            "STEAM_COMPAT_DATA_PATH",
            path_value(root.join("compatdata"))?,
        ),
        (
            "STEAM_COMPAT_INSTALL_PATH",
            path_value(prefix.join("drive_c/Program Files/Common Files/VST3"))?,
        ),
        (
            "STEAM_COMPAT_EMULATOR",
            path_value(runtime.emulator_manifest.path.clone())?,
        ),
        (
            "STEAM_COMPAT_GRAPHICS_PROVIDER",
            path_value(runtime.graphics_manifest.path.clone())?,
        ),
        (
            "PRESSURE_VESSEL_FILESYSTEMS_RO",
            path_value(config.xauthority.path.clone())?,
        ),
        ("PRESSURE_VESSEL_FILESYSTEMS_RW", path_value(root.clone())?),
        (
            "PRESSURE_VESSEL_VARIABLE_DIR",
            path_value(runtime.runtime_variable_dir.clone())?,
        ),
        (
            "LVB_EVENT_OUTPUT_POLICY",
            "reported_zero_event_channels_unspecified".to_owned(),
        ),
        ("LVB_RPI0_ARCHITECTURE", "required".to_owned()),
        (
            "LVB_EDITOR_LIFETIME",
            "retain_editor_view_until_instance_retirement".to_owned(),
        ),
        (
            "LVB_VENDOR_RETIREMENT",
            "process_scoped_vendor_retirement".to_owned(),
        ),
    ] {
        if value
            .bytes()
            .any(|byte| byte == 0 || byte == b'\n' || byte == b'\r')
        {
            return Err(invalid(format!("{key} environment value")));
        }
        values.insert(key.to_owned(), value);
    }
    Ok(values)
}

/// Optional, bounded per-session diagnostics. The native owner consumes this
/// selector; it is never forwarded to Proton or interpreted as a shell string.
/// Exact pinned Box64/Proton support is recorded in RPI1 observability evidence.
fn diagnostic_environment(
    config: &Config,
    session: &str,
    raw: &str,
) -> io::Result<BTreeMap<String, String>> {
    let mut selected = BTreeMap::new();
    if raw.is_empty() {
        return Ok(selected);
    }
    if matches!(config.runner, Runner::Native { .. }) {
        return Err(invalid("Box64 diagnostic selection on native runner"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for option in raw.split(',') {
        if !seen.insert(option) {
            return Err(invalid("duplicate RPI1 diagnostic option"));
        }
        match option {
            "box64-crash" => {
                selected.insert("BOX64_ROLLING_LOG".into(), "64".into());
                selected.insert("BOX64_SHOWSEGV".into(), "1".into());
                selected.insert("BOX64_SHOWBT".into(), "1".into());
            }
            "box64-perfmap" => {
                selected.insert("BOX64_DYNAREC_PERFMAP".into(), "1".into());
            }
            "proton-log" => {
                let path = config.evidence_directory.join(format!("{session}-proton"));
                use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
                fs::DirBuilder::new().mode(0o700).create(&path)?;
                let metadata = fs::symlink_metadata(&path)?;
                if !metadata.is_dir()
                    || metadata.file_type().is_symlink()
                    || metadata.uid() != unsafe { libc::getuid() }
                    || metadata.permissions().mode() & 0o077 != 0
                {
                    return Err(invalid("private Proton diagnostic directory required"));
                }
                selected.insert("PROTON_LOG".into(), "1".into());
                selected.insert(
                    "PROTON_LOG_DIR".into(),
                    path.into_os_string()
                        .into_string()
                        .map_err(|_| invalid("Proton diagnostic path encoding"))?,
                );
            }
            _ => return Err(invalid("unknown RPI1 diagnostic option")),
        }
    }
    Ok(selected)
}

pub fn read_journal(unit: &str) -> io::Result<Vec<u8>> {
    if !unit.starts_with("lvb-rpi1-")
        || !unit.ends_with(".service")
        || unit
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.')))
    {
        return Err(invalid("RPI1 journal unit identity"));
    }
    let output = Command::new("/usr/bin/journalctl")
        .args(["--user-unit", unit, "--output=cat", "--no-pager"])
        .output()?;
    require(output.status.success(), "RPI1 journal read failed")?;
    require(
        !output.stdout.is_empty() && output.stdout.len() <= 2 * 1024 * 1024,
        "RPI1 journal extent",
    )?;
    Ok(output.stdout)
}

fn inspect(unit: &str) -> io::Result<Identity> {
    let output = Command::new("/usr/bin/systemctl")
        .args([
            "--user",
            "show",
            unit,
            "--property=ActiveState,MainPID,ControlGroup",
            "--no-pager",
        ])
        .output()?;
    require(output.status.success(), "systemd unit unavailable")?;
    let text =
        std::str::from_utf8(&output.stdout).map_err(|_| invalid("systemd output encoding"))?;
    let mut active = None;
    let mut pid = None;
    let mut group = None;
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| invalid("systemd property shape"))?;
        match key {
            "ActiveState" => active = Some(value),
            "MainPID" => {
                pid = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| invalid("main PID syntax"))?,
                )
            }
            "ControlGroup" => group = Some(value),
            _ => return Err(invalid("unexpected systemd property")),
        }
    }
    require(active == Some("active"), "RPI1 unit not active")?;
    let main_pid = pid
        .filter(|value| *value > 0)
        .ok_or_else(|| invalid("RPI1 main PID absent"))?;
    let group = group
        .filter(|value| value.starts_with('/') && !value.contains(".."))
        .ok_or_else(|| invalid("RPI1 cgroup identity"))?;
    Ok(Identity {
        unit: unit.to_owned(),
        cgroup: PathBuf::from("/sys/fs/cgroup").join(group.trim_start_matches('/')),
        main_pid,
        main_start_ticks: process_start(main_pid)?,
    })
}

fn process_start(pid: u32) -> io::Result<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))?;
    let end = stat.rfind(')').ok_or_else(|| invalid("proc stat shape"))?;
    stat[end + 2..]
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| invalid("proc start identity absent"))?
        .parse::<u64>()
        .map_err(|_| invalid("proc start identity syntax"))
}

fn user_identity() -> io::Result<(String, PathBuf)> {
    let record = unsafe { libc::getpwuid(libc::getuid()) };
    if record.is_null() {
        return Err(io::Error::last_os_error());
    }
    let record = unsafe { &*record };
    let user = unsafe { CStr::from_ptr(record.pw_name) }
        .to_str()
        .map_err(|_| invalid("user name encoding"))?
        .to_owned();
    let home = unsafe { CStr::from_ptr(record.pw_dir) };
    Ok((
        user,
        PathBuf::from(std::ffi::OsStr::from_bytes(home.to_bytes())),
    ))
}

fn validate_session(session: &str) -> io::Result<()> {
    if session.len() != 32 || !session.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Err(invalid("session identity"))
    } else {
        Ok(())
    }
}

fn systemctl(arguments: &[&str]) -> io::Result<ExitStatus> {
    Command::new("/usr/bin/systemctl")
        .arg("--user")
        .args(arguments)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}

fn require(value: bool, message: &str) -> io::Result<()> {
    if value {
        Ok(())
    } else {
        Err(invalid(message))
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Box64Runner, PinnedFile, Runner};

    #[test]
    fn launch_vector_is_exact_slr_proton_runinprefix_chain() {
        let file = |path: &str| PinnedFile {
            path: PathBuf::from(path),
            sha256: [0; 32],
        };
        let mut config = Config {
            runner: Runner::Box64(Box64Runner {
                box64: file("/root/box64"),
                adapter: file("/root/adapter"),
                emulator_manifest: file("/root/emulator.json"),
                graphics_manifest: file("/root/graphics.json"),
                slr_entry: file("/root/SteamLinuxRuntime_4/_v2-entry-point"),
                proton: file("/root/Proton 11.0/proton"),
                runtime_variable_dir: "/root/runtime-var-pyhome-exact".into(),
            }),
            windows_host: file("/root/pfx/drive_c/host.exe"),
            plugin: file("/root/pfx/drive_c/Pigments.vst3"),
            environment_root: "/root".into(),
            display: ":1".into(),
            xauthority: file("/home/user/.Xauthority"),
            source_manifest_sha256: [0; 32],
            bridge_frames: 2048,
            jack_client: "lvb-arm-pigments".into(),
            evidence_directory: "/root/evidence".into(),
        };
        let vector = launch_vector(&config, &["C:\\host.exe".into(), "--session".into()]);
        assert_eq!(vector[0], "/root/SteamLinuxRuntime_4/_v2-entry-point");
        assert_eq!(vector[1], "--verb=run");
        assert_eq!(vector[2], "--");
        assert_eq!(vector[3], "/root/Proton 11.0/proton");
        assert_eq!(vector[4], "runinprefix");
        assert_eq!(vector[5], "C:\\host.exe");
        let values = environment(&config).unwrap();
        assert_eq!(values["PYTHONHOME"], "/usr");
        assert_eq!(values["LVB_RPI0_ARCHITECTURE"], "required");
        assert_eq!(
            values["LVB_VENDOR_RETIREMENT"],
            "process_scoped_vendor_retirement"
        );
        assert!(
            diagnostic_environment(&config, "00000000000000000000000000000000", "")
                .unwrap()
                .is_empty()
        );
        assert!(diagnostic_environment(
            &config,
            "00000000000000000000000000000000",
            "box64-crash,box64-crash"
        )
        .is_err());
        assert!(
            diagnostic_environment(&config, "00000000000000000000000000000000", "full-trace")
                .is_err()
        );
        let diagnostic = diagnostic_environment(
            &config,
            "00000000000000000000000000000000",
            "box64-crash,box64-perfmap",
        )
        .unwrap();
        assert_eq!(diagnostic["BOX64_ROLLING_LOG"], "64");
        assert_eq!(diagnostic["BOX64_SHOWBT"], "1");
        assert_eq!(diagnostic["BOX64_SHOWSEGV"], "1");
        assert_eq!(diagnostic["BOX64_DYNAREC_PERFMAP"], "1");
        config.runner = Runner::Native {
            launcher: file("/native/launch"),
            leader: file("/usr/bin/python3"),
        };
        assert_eq!(
            launch_vector(&config, &["C:\\host.exe".into()]),
            vec![
                OsString::from("/native/launch"),
                OsString::from("C:\\host.exe")
            ]
        );
        let env = environment(&config).unwrap();
        assert_eq!(env["WINEPREFIX"], "/root/compatdata/pfx");
        assert_eq!(
            env["LVB_EVENT_OUTPUT_POLICY"],
            "reported_zero_event_channels_unspecified"
        );
        assert!(!env.contains_key("STEAM_COMPAT_EMULATOR"));
        assert!(!env.contains_key("PYTHONHOME"));
        assert!(
            diagnostic_environment(&config, "00000000000000000000000000000000", "box64-crash")
                .is_err()
        );
    }

    #[test]
    fn proc_start_identity_parser_uses_field_twenty_two_after_comm() {
        let stat =
            "101 (name with spaces) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 424242 20";
        let end = stat.rfind(')').unwrap();
        assert_eq!(
            stat[end + 2..].split_whitespace().nth(19).unwrap(),
            "424242"
        );
    }
}

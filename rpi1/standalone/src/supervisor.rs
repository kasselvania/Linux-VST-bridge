use crate::config::Config;
use std::{
    collections::BTreeMap,
    ffi::{CStr, OsString},
    fs, io,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    process::{Command, ExitStatus},
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
        for (key, value) in environment(config)? {
            command.arg(
                OsString::from(format!("--setenv={key}="))
                    .into_string()
                    .unwrap()
                    + &value,
            );
        }
        command.args(launch_vector(config, host_arguments));
        require(
            command.status()?.success(),
            "systemd refused exact RPI1 cohort",
        )?;
        let end = Instant::now() + Duration::from_secs(10);
        let identity = loop {
            match inspect(&unit) {
                Ok(identity) => break identity,
                Err(_) if Instant::now() < end => thread::sleep(Duration::from_millis(20)),
                Err(error) => return Err(error),
            }
        };
        Ok(Self {
            identity,
            retired: false,
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn verify(&self) -> io::Result<Vec<ProcessIdentity>> {
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
            let start_ticks = process_start(pid)?;
            let executable = fs::read_link(format!("/proc/{pid}/exe"))?;
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
    let mut command = vec![
        config.slr_entry.path.as_os_str().to_owned(),
        "--verb=run".into(),
        "--".into(),
        config.proton.path.as_os_str().to_owned(),
        "runinprefix".into(),
    ];
    command.extend_from_slice(host_arguments);
    command
}

pub fn environment(config: &Config) -> io::Result<BTreeMap<String, String>> {
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
            path_value(config.emulator_manifest.path.clone())?,
        ),
        (
            "STEAM_COMPAT_GRAPHICS_PROVIDER",
            path_value(config.graphics_manifest.path.clone())?,
        ),
        (
            "PRESSURE_VESSEL_FILESYSTEMS_RO",
            path_value(config.xauthority.path.clone())?,
        ),
        ("PRESSURE_VESSEL_FILESYSTEMS_RW", path_value(root.clone())?),
        (
            "PRESSURE_VESSEL_VARIABLE_DIR",
            path_value(config.runtime_variable_dir.clone())?,
        ),
        (
            "LVB_EVENT_OUTPUT_POLICY",
            "reported_zero_event_channels_unspecified".to_owned(),
        ),
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
    use crate::config::PinnedFile;

    #[test]
    fn launch_vector_is_exact_slr_proton_runinprefix_chain() {
        let file = |path: &str| PinnedFile {
            path: PathBuf::from(path),
            sha256: [0; 32],
        };
        let config = Config {
            box64: file("/root/box64"),
            adapter: file("/root/adapter"),
            emulator_manifest: file("/root/emulator.json"),
            graphics_manifest: file("/root/graphics.json"),
            slr_entry: file("/root/SteamLinuxRuntime_4/_v2-entry-point"),
            proton: file("/root/Proton 11.0/proton"),
            windows_host: file("/root/pfx/drive_c/host.exe"),
            plugin: file("/root/pfx/drive_c/Pigments.vst3"),
            environment_root: "/root".into(),
            runtime_variable_dir: "/root/runtime-var-pyhome-exact".into(),
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

use crate::config::{Config, PinnedFile};
use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
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
    pub main_executable: PathBuf,
}

pub struct Cohort {
    identity: Identity,
    box64: PinnedFile,
    retired: bool,
}

impl Cohort {
    pub fn launch(
        config: &Config,
        session_hex: &str,
        windows_arguments: &[OsString],
    ) -> io::Result<Self> {
        if session_hex.len() != 32 || !session_hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("session identity"));
        }
        let unit = format!("lvb-rpi0-{session_hex}.service");
        let active = systemctl(&["is-active", &unit])?;
        if active.success() {
            return Err(invalid("RPI0 unit already active"));
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
        command.arg(format!("--setenv=WINEPREFIX={}", config.prefix.display()));
        command.arg(format!(
            "--setenv=BOX64_RCFILE={}",
            config.box64_rc.path.display()
        ));
        for setting in [
            "BOX64_NOBANNER=1",
            "BOX64_DYNACACHE=0",
            "BOX64_DYNAREC=1",
            "BOX64_DYNAREC_STRONGMEM=1",
        ] {
            command.arg(format!("--setenv={setting}"));
        }
        command
            .arg(&config.box64.path)
            .arg(&config.wine.path)
            .args(windows_arguments);
        let status = command.status()?;
        require(status.success(), "systemd refused exact RPI0 cohort")?;
        let end = Instant::now() + Duration::from_secs(10);
        let identity = loop {
            match inspect(&unit, &config.box64.path) {
                Ok(identity) => break identity,
                Err(error) if Instant::now() < end => {
                    let _ = error;
                    thread::sleep(Duration::from_millis(20));
                }
                Err(error) => return Err(error),
            }
        };
        Ok(Self {
            identity,
            box64: config.box64.clone(),
            retired: false,
        })
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    pub fn verify(&self) -> io::Result<Vec<u32>> {
        let current = inspect(&self.identity.unit, &self.box64.path)?;
        if current.main_pid != self.identity.main_pid
            || current.main_start_ticks != self.identity.main_start_ticks
            || current.cgroup != self.identity.cgroup
            || current.main_executable != self.identity.main_executable
        {
            return Err(invalid("RPI0 cohort identity changed"));
        }
        let text = fs::read_to_string(current.cgroup.join("cgroup.procs"))?;
        let mut pids = Vec::new();
        for line in text.lines() {
            let pid = line
                .parse::<u32>()
                .map_err(|_| invalid("cgroup PID syntax"))?;
            let executable = fs::read_link(format!("/proc/{pid}/exe"))?;
            if fs::canonicalize(executable)? != fs::canonicalize(&self.box64.path)? {
                return Err(invalid(format!(
                    "unrecognized executable in RPI0 cgroup: {pid}"
                )));
            }
            pids.push(pid);
        }
        require(
            !pids.is_empty() && pids.contains(&self.identity.main_pid),
            "RPI0 cgroup lost main process",
        )?;
        Ok(pids)
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
                break;
            }
            if Instant::now() >= end {
                return Err(invalid("RPI0 cgroup cleanup incomplete"));
            }
            thread::sleep(Duration::from_millis(50));
        }
        self.retired = true;
        Ok(())
    }

    pub fn await_retired(mut self, timeout: Duration) -> io::Result<()> {
        let end = Instant::now() + timeout;
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
                return Err(invalid("RPI0 cohort did not retire naturally"));
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Cohort {
    fn drop(&mut self) {
        if !self.retired {
            // Never fall back to name matching or a global Wine operation.
            let _ = systemctl(&["stop", &self.identity.unit]);
        }
    }
}

pub fn run_owned_probe(config: &Config, session_hex: &str, windows: bool) -> io::Result<()> {
    let argument = if windows {
        config.windows_probe.path.as_os_str()
    } else {
        config.linux_probe.path.as_os_str()
    };
    let mut command = Command::new("/usr/bin/systemd-run");
    let unit = format!(
        "lvb-rpi0-preflight-{session_hex}-{}.service",
        if windows { "windows" } else { "linux" }
    );
    command.args([
        "--user",
        "--wait",
        "--collect",
        "--quiet",
        "--service-type=exec",
        "--property=KillMode=control-group",
        &format!("--unit={unit}"),
    ]);
    command.arg(format!(
        "--setenv=BOX64_RCFILE={}",
        config.box64_rc.path.display()
    ));
    command.arg("--setenv=BOX64_NOBANNER=1");
    if windows {
        command.arg(format!("--setenv=WINEPREFIX={}", config.prefix.display()));
    }
    command.arg(&config.box64.path);
    if windows {
        command.arg(&config.wine.path);
    }
    let status = command.arg(argument).arg("--self-test").status()?;
    require(
        status.success(),
        if windows {
            "Windows preflight failed"
        } else {
            "Linux preflight failed"
        },
    )?;
    let active = systemctl(&["is-active", &unit])?;
    require(!active.success(), "preflight unit remained active")
}

pub fn read_journal(unit: &str) -> io::Result<Vec<u8>> {
    if !unit.starts_with("lvb-rpi0-")
        || !unit.ends_with(".service")
        || unit
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.')))
    {
        return Err(invalid("RPI0 journal unit identity"));
    }
    let output = Command::new("/usr/bin/journalctl")
        .args(["--user-unit", unit, "--output=cat", "--no-pager"])
        .output()?;
    require(output.status.success(), "RPI0 journal read failed")?;
    require(
        !output.stdout.is_empty() && output.stdout.len() <= 2 * 1024 * 1024,
        "RPI0 journal extent",
    )?;
    Ok(output.stdout)
}

fn inspect(unit: &str, expected_executable: &Path) -> io::Result<Identity> {
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
    require(active == Some("active"), "RPI0 unit not active")?;
    let main_pid = pid
        .filter(|value| *value > 0)
        .ok_or_else(|| invalid("RPI0 main PID absent"))?;
    let group = group
        .filter(|value| value.starts_with('/') && !value.contains(".."))
        .ok_or_else(|| invalid("RPI0 cgroup identity"))?;
    let cgroup = PathBuf::from("/sys/fs/cgroup").join(group.trim_start_matches('/'));
    let executable = fs::canonicalize(fs::read_link(format!("/proc/{main_pid}/exe"))?)?;
    if executable != fs::canonicalize(expected_executable)? {
        return Err(invalid("RPI0 main executable differs"));
    }
    let stat = fs::read_to_string(format!("/proc/{main_pid}/stat"))?;
    let end = stat.rfind(')').ok_or_else(|| invalid("proc stat shape"))?;
    let main_start_ticks = stat[end + 2..]
        .split_whitespace()
        .nth(19)
        .ok_or_else(|| invalid("proc start identity absent"))?
        .parse::<u64>()
        .map_err(|_| invalid("proc start identity syntax"))?;
    Ok(Identity {
        unit: unit.to_owned(),
        cgroup,
        main_pid,
        main_start_ticks,
        main_executable: executable,
    })
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
    #[test]
    fn proc_start_identity_parser_uses_field_twenty_two_after_comm() {
        let stat =
            "101 (name with spaces) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 424242 21";
        let end = stat.rfind(')').unwrap();
        assert_eq!(
            stat[end + 2..].split_whitespace().nth(19).unwrap(),
            "424242"
        );
    }
    #[test]
    fn pinned_runtime_values_are_reportable_without_paths() {
        assert_eq!(crate::config::hex(&[0xab; 2]), "abab");
    }
}

//! One host launch shape for exact signed source, retaining bytecode history.
use crate::{require, Artifact, Result};
use std::{ffi::OsString, io::Read, process::{Command, Stdio}, time::{Duration, Instant}};

pub const BOOTSTRAP: &str = include_str!("../runtime/bootstrap.py");
pub const SOURCE_PYTHON_RANGE: &str = "CPython >=3.13,<3.16";

/// These arguments follow the interpreter, including behind systemd-run.
/// Retained bytecode keeps its historical direct-script invocation. Source
/// never imports an adjacent or ambient ownership module and writes no pyc.
pub fn arguments(session: &Artifact, ownership: &Artifact) -> Result<Vec<OsString>> {
    session.verify()?;
    ownership.verify()?;
    let source = session.path.extension().is_some_and(|x| x == "py");
    require(source == ownership.path.extension().is_some_and(|x| x == "py"),
        "supervisor_payload_format_mismatch")?;
    if !source { return Ok(vec![session.path.clone().into_os_string()]); }
    Ok(vec!["-I".into(), "-S".into(), "-B".into(), "-c".into(), BOOTSTRAP.into(),
        session.path.clone().into_os_string(), session.sha256.clone().into(),
        ownership.path.clone().into_os_string(), ownership.sha256.clone().into()])
}

/// Source generation admission checks the same range as every actual launch.
pub fn verify_host_python() -> Result<()> {
    let mut child = Command::new("/usr/bin/python3")
        .args(["-I", "-S", "-B", "-c", BOOTSTRAP, "--probe"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.try_wait()? { break status; }
        if Instant::now() >= deadline {
            let _ = child.kill(); let _ = child.wait();
            return Err("supervisor_python_probe_timeout".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut output = String::new();
    child.stdout.take().ok_or("supervisor_python_probe")?.take(64).read_to_string(&mut output)?;
    require(status.success() && output.trim() == "cpython-source-v1",
        "This build requires host CPython >=3.13,<3.16. Your selected application was retained")
}

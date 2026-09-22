//! Pre-activation observation of the exact host's existing lifecycle records.
//! Inspection completion permits sending Configure; its response remains the
//! authority for successful setup. No journal work runs in the audio callback.
use crate::supervisor::Cohort;
use serde_json::Value;
use std::{
    io::{self, Read},
    os::fd::AsRawFd,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const BYTE_LIMIT: usize = 2 * 1024 * 1024;
const LINE_LIMIT: usize = 64 * 1024;

struct Journal(Child);
impl Drop for Journal {
    fn drop(&mut self) {
        // Only the observer child is killed; the cohort retains its existing
        // process owner and failure cleanup. Reap on success and every error.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub fn await_initialization(cohort: &Cohort, session: &str, timeout: Duration) -> io::Result<()> {
    if session.len() != 32
        || !session.bytes().all(|b| b.is_ascii_hexdigit())
        || cohort.identity().unit != format!("lvb-rpi1-{session}.service")
        || timeout.is_zero()
        || timeout > Duration::from_secs(120)
    {
        return Err(invalid("RPI1 initialization binding or deadline"));
    }
    cohort.verify_startup_leader()?;
    let mut child = Journal(
        Command::new("/usr/bin/journalctl")
            .args([
                "--user-unit",
                &cohort.identity().unit,
                "--output=cat",
                "--no-pager",
                "--follow",
                "--lines=all",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?,
    );
    let mut stdout = child
        .0
        .stdout
        .take()
        .ok_or_else(|| invalid("startup journal pipe"))?;
    let fd = stdout.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    wait(&mut stdout, session, timeout, || {
        cohort.verify_startup_leader()?;
        if child.0.try_wait()?.is_some() {
            return Err(invalid("initialization journal observer exited"));
        }
        Ok(())
    })
}

fn wait(
    reader: &mut impl Read,
    session: &str,
    timeout: Duration,
    mut alive: impl FnMut() -> io::Result<()>,
) -> io::Result<()> {
    let end = Instant::now() + timeout;
    let mut records = Records::new(session);
    let mut bytes = [0; 4096];
    loop {
        if Instant::now() >= end {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Pigments initialization deadline",
            ));
        }
        alive()?;
        match reader.read(&mut bytes) {
            Ok(0) => return Err(invalid("initialization journal stream closed")),
            Ok(n) => {
                if records.push(&bytes[..n])? {
                    alive()?;
                    if Instant::now() >= end {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "Pigments initialization deadline",
                        ));
                    }
                    return Ok(());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(
                    Duration::from_millis(20).min(end.saturating_duration_since(Instant::now())),
                );
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
}

struct Records<'a> {
    session: &'a str,
    bound: bool,
    capture: bool,
    total: usize,
    line: Vec<u8>,
}
impl<'a> Records<'a> {
    fn new(session: &'a str) -> Self {
        Self {
            session,
            bound: false,
            capture: false,
            total: 0,
            line: Vec::new(),
        }
    }
    fn push(&mut self, bytes: &[u8]) -> io::Result<bool> {
        self.total = self
            .total
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("initialization journal extent"))?;
        if self.total > BYTE_LIMIT {
            return Err(invalid("initialization journal byte bound"));
        }
        for &byte in bytes {
            if byte == b'\n' {
                let ready = self.record()?;
                self.line.clear();
                if ready {
                    return Ok(true);
                }
            } else {
                if self.line.len() == LINE_LIMIT {
                    return Err(invalid("initialization journal line bound"));
                }
                self.line.push(byte);
            }
        }
        Ok(false)
    }
    fn record(&mut self) -> io::Result<bool> {
        // The pinned source host emits compact JSON. Wine/systemd prose is not
        // an initialization receipt and is ignored without interpreting it.
        if !self.line.starts_with(b"{\"event\":") {
            return Ok(false);
        }
        let value: Value = serde_json::from_slice(&self.line)
            .map_err(|_| invalid("malformed host lifecycle record"))?;
        if value["event"] != "lifecycle" {
            return Ok(false);
        }
        match value["state"].as_str() {
            Some("readiness_announced") => {
                if value["session"] != self.session || value["mode"] != "ap9-commercial" {
                    return Err(invalid("initialization journal session differs"));
                }
                self.bound = true;
            }
            Some("ap8_failure" | "ap1_transport_error" | "ap3_processing_error") => {
                return Err(invalid("Windows host failed during initialization"));
            }
            Some("ap12_persistence") => {
                if !self.bound || value["capture_available"] != true || value["sdk_result"] != 0 {
                    return Err(invalid("initial component state unavailable"));
                }
                self.capture = true;
            }
            Some("ap8_inspected") => {
                if !self.bound
                    || !self.capture
                    || value["float32_result"] != 0
                    || !value["state_bytes"]
                        .as_u64()
                        .is_some_and(|n| n > 0 && n <= 16 * 1024 * 1024)
                {
                    return Err(invalid("incomplete Pigments initialization record"));
                }
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, os::unix::net::UnixStream};
    const SESSION: &str = "0123456789abcdef0123456789abcdef";
    fn receipt() -> Vec<u8> {
        format!("{{\"event\":\"lifecycle\",\"state\":\"readiness_announced\",\"session\":\"{SESSION}\",\"mode\":\"ap9-commercial\"}}\n{{\"event\":\"lifecycle\",\"state\":\"ap12_persistence\",\"capture_available\":true,\"sdk_result\":0}}\n{{\"event\":\"lifecycle\",\"state\":\"ap8_inspected\",\"float32_result\":0,\"state_bytes\":155035}}\n").into_bytes()
    }
    #[test]
    fn fragmented_receipt_requires_matching_session_and_complete_inspection() {
        let bytes = receipt();
        let mut records = Records::new(SESSION);
        assert!(!records.push(b"Wine startup text\n").unwrap());
        for byte in &bytes[..bytes.len() - 1] {
            assert!(!records.push(&[*byte]).unwrap());
        }
        assert!(records.push(b"\n").unwrap());
        assert!(Records::new("different").push(&bytes).is_err());
        let mut unbound = Records::new(SESSION);
        assert!(unbound.push(b"{\"event\":\"lifecycle\",\"state\":\"ap8_inspected\",\"float32_result\":0,\"state_bytes\":1}\n").is_err());
    }
    #[test]
    fn failures_and_stream_bounds_never_become_ready() {
        for state in ["ap8_failure", "ap1_transport_error", "ap3_processing_error"] {
            let line = format!("{{\"event\":\"lifecycle\",\"state\":\"{state}\"}}\n");
            assert!(Records::new(SESSION).push(line.as_bytes()).is_err());
        }
        assert!(Records::new(SESSION).push(b"{\"event\":bad}\n").is_err());
        assert!(Records::new(SESSION)
            .push(&vec![b'x'; LINE_LIMIT + 1])
            .is_err());
        assert!(Records::new(SESSION)
            .push(&vec![b'\n'; BYTE_LIMIT + 1])
            .is_err());
        let absent = String::from_utf8(receipt())
            .unwrap()
            .replace("\"capture_available\":true", "\"capture_available\":false");
        assert!(Records::new(SESSION).push(absent.as_bytes()).is_err());
    }
    #[test]
    fn startup_can_outlast_configure_budget_without_sending_control() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        let peer = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10_100));
            writer.write_all(&receipt()).unwrap();
        });
        let start = Instant::now();
        wait(&mut reader, SESSION, Duration::from_secs(12), || Ok(())).unwrap();
        assert!(start.elapsed() >= Duration::from_secs(10));
        peer.join().unwrap();
    }
    #[test]
    fn missing_receipt_deadline_and_cohort_exit_are_bounded() {
        let (mut reader, _writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        assert_eq!(
            wait(&mut reader, SESSION, Duration::from_millis(25), || Ok(()))
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert!(wait(&mut reader, SESSION, Duration::from_secs(1), || Err(
            invalid("cohort exited")
        ))
        .unwrap_err()
        .to_string()
        .contains("cohort exited"));
    }

    #[test]
    fn observer_child_is_reaped_when_its_guard_leaves_scope() {
        let child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        drop(Journal(child));
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}

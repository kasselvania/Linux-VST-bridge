//! The actual control service owns every browser-return dispatch in this test.
//! Its output stays under the fixture root, inside both privacy searches.
use crate::setup::Setup;
use std::{
    fs,
    io::{self, Read, Write},
    os::unix::{fs::OpenOptionsExt, net::UnixStream, process::ExitStatusExt},
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const STARTUP: Duration = Duration::from_secs(60);
const RETIREMENT: Duration = Duration::from_secs(10);

pub struct BridgeService { child: Option<Child> }

impl BridgeService {
    pub fn start(setup: &Setup) -> Self {
        let log = fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
            .open(setup.m.root.join("signin-service.log")).unwrap();
        let child = Command::new(&setup.software.manager.path).arg("serve")
            .stdin(Stdio::null()).stdout(log.try_clone().unwrap()).stderr(log)
            .spawn().unwrap();
        // Own the child before waiting, so startup failures also retire it.
        let mut service = Self { child: Some(child) };
        let started = Instant::now();
        let socket = setup.m.root.join("runtime/owner.sock");
        loop {
            assert!(service.child.as_mut().unwrap().try_wait().unwrap().is_none(),
                "the bridge service exited during startup; its output is in signin-service.log");
            if let Some(pulse) = pulse(&socket) {
                assert_eq!(pulse, 0, "a fresh fixture has no unconfirmed cleanup");
                service.assert_private();
                return service;
            }
            assert!(started.elapsed() < STARTUP, "the bridge service did not become ready");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn stop(mut self) {
        assert!(self.child.as_mut().unwrap().try_wait().unwrap().is_none(),
            "the bridge service stopped before the fixture retired it");
        let status = self.terminate().expect("retire the bridge service normally");
        assert_eq!(status.signal(), Some(libc::SIGTERM), "bridge service exit: {status}");
    }

    /// The callback's long-lived owner has the same privacy protections as
    /// the CLI handler. Never print or retain any process environment bytes.
    pub fn assert_private(&self) {
        let pid = self.child.as_ref().expect("an owned bridge service").id();
        let process = std::path::PathBuf::from(format!("/proc/{pid}"));
        let environment = fs::File::open(process.join("environ"));
        assert!(matches!(environment, Err(error) if error.kind() == io::ErrorKind::PermissionDenied),
            "the bridge service must be non-dumpable to its own user");
        let limits = fs::read_to_string(process.join("limits")).unwrap();
        let core = limits.lines().find_map(|line| line.strip_prefix("Max core file size"))
            .expect("core limit is observable");
        let fields: Vec<_> = core.split_whitespace().collect();
        assert_eq!(&fields[..2], &["0", "0"], "bridge service core limits");
        // Exact operation ownership still has a readable process generation;
        // disabling ptrace access must not hide the retained executor identity.
        let stat = fs::read_to_string(process.join("stat")).unwrap();
        let start: u64 = stat.rsplit_once(") ").unwrap().1.split_whitespace().nth(19)
            .unwrap().parse().unwrap();
        assert!(start > 0, "bridge service start ticks stay readable");
    }

    fn terminate(&mut self) -> io::Result<ExitStatus> {
        let child = self.child.as_mut().expect("an owned bridge service");
        if child.try_wait()?.is_none() && unsafe { libc::kill(child.id() as i32, libc::SIGTERM) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                self.child.take();
                return Ok(status);
            }
            if started.elapsed() >= RETIREMENT {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "bridge service did not retire after SIGTERM"));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for BridgeService {
    fn drop(&mut self) {
        if self.child.is_some() {
            if let Err(error) = self.terminate() {
                eprintln!("bridge service fixture cleanup failed: {error}");
            }
        }
    }
}

fn pulse(socket: &Path) -> Option<u8> {
    let mut peer = UnixStream::connect(socket).ok()?;
    peer.set_read_timeout(Some(Duration::from_millis(250))).ok()?;
    peer.set_write_timeout(Some(Duration::from_millis(250))).ok()?;
    peer.write_all(b"LVP1\n").ok()?;
    let mut state = [0];
    peer.read_exact(&mut state).ok()?;
    Some(state[0])
}

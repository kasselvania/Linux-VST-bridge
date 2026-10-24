//! Linux executable custody retaining disk-backed `/proc/self/exe` semantics.
//!
//! An open descriptor prevents rename substitution, but not writes to its inode.
//! Exec the retained ELF under ptrace and verify it at the exec stop, before any
//! instruction (including the ELF interpreter) runs. Linux holds deny_write_access
//! on that inode from exec until exit. A concurrent writable fd/mapping refuses
//! exec; a completed earlier write fails the signed digest check. No pathname is
//! reopened for execution. Without renames, startup sees the staged package path;
//! renaming the inode or its parents changes the proc-derived path and may cause
//! package-relative bootstrap to refuse it.
use super::*;
use std::{
    ffi::CString,
    os::unix::ffi::OsStrExt,
    time::{Duration, Instant},
};

pub struct VerifiedExecutable {
    file: fs::File,
    path: PathBuf,
    row: ReleaseFile,
}
impl VerifiedExecutable {
    pub(super) fn open(path: &Path, row: &ReleaseFile) -> Result<Self> {
        require(path.canonicalize()? == path, "user_package_link")?;
        let mut executable = Self {
            file: file(path)?,
            path: path.into(),
            row: row.clone(),
        };
        executable.verify()?;
        Ok(executable)
    }
    fn verify(&mut self) -> Result<()> {
        let meta = self.file.metadata()?;
        require(
            meta.is_file()
                && meta.uid() == unsafe { libc::getuid() }
                && meta.mode() & 0o7777 == 0o555
                && meta.len() == self.row.size,
            "user_package_executable_custody",
        )?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut magic = [0; 4];
        self.file.read_exact(&mut magic)?;
        require(magic == *b"\x7fELF", "user_package_executable_format")?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut hash = Sha256::new();
        let mut remaining = self.row.size;
        let mut block = [0; 65536];
        while remaining != 0 {
            let bound = block.len().min(remaining as usize);
            self.file.read_exact(&mut block[..bound])?;
            hash.update(&block[..bound]);
            remaining -= bound as u64;
        }
        require(
            hex(&hash.finalize()) == self.row.sha256,
            "user_package_executable_changed",
        )
    }
    pub fn spawn(self) -> Result<()> {
        let mut child = self.start()?;
        // The window outlives the installer; reap it without blocking its worker.
        std::thread::Builder::new()
            .name("bridge-setup-reaper".into())
            .spawn(move || {
                let _ = child.wait();
            })?;
        Ok(())
    }
    fn start(mut self) -> Result<Child> {
        let executable = CString::new(format!("/proc/self/fd/{}", self.file.as_raw_fd()))?;
        let name = CString::new(self.path.as_os_str().as_bytes())?;
        let argv = [name.as_ptr(), std::ptr::null()];
        let environment: Vec<CString> = std::env::vars_os()
            .map(|(key, value)| {
                let mut bytes = key.as_bytes().to_vec();
                bytes.push(b'=');
                bytes.extend_from_slice(value.as_bytes());
                CString::new(bytes)
            })
            .collect::<std::result::Result<_, _>>()?;
        let mut envp: Vec<_> = environment.iter().map(|value| value.as_ptr()).collect();
        envp.push(std::ptr::null());
        let parent = unsafe { libc::getpid() };
        let parent_thread = unsafe { libc::syscall(libc::SYS_gettid) as libc::pid_t };
        // All allocation and Rust locks precede fork. The child uses only raw
        // syscalls/async-signal-safe libc until exec or _exit, even on failure.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if pid == 0 {
            unsafe {
                // Cover tracer death before EXITKILL can be installed. Linux
                // ties PDEATHSIG to the creating thread, not just the process.
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0) != 0
                    || libc::getppid() != parent
                    || libc::syscall(libc::SYS_tgkill, parent, parent_thread, 0) != 0
                    || libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) != 0
                {
                    libc::_exit(127);
                }
                libc::raise(libc::SIGSTOP);
                // Parent installed EXITKILL before continuing us. Clear the
                // temporary death signal so Setup survives installer closure.
                if libc::prctl(libc::PR_SET_PDEATHSIG, 0, 0, 0, 0) != 0 {
                    libc::_exit(127);
                }
                libc::execve(executable.as_ptr(), argv.as_ptr(), envp.as_ptr());
                libc::_exit(127);
            }
        }
        let mut child = Child { pid, reaped: false };
        child.stopped(libc::SIGSTOP, 0)?;
        ptrace(
            libc::PTRACE_SETOPTIONS,
            pid,
            (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_EXITKILL) as usize,
        )?;
        ptrace(libc::PTRACE_CONT, pid, 0)?;
        // TRACEEXEC uses ptrace_notify, independent of the inherited signal mask.
        // The legacy exec SIGTRAP is only send_sig and may remain blocked.
        // Linux v6.12 include/linux/ptrace.h: ptrace_event().
        child.stopped(libc::SIGTRAP, libc::PTRACE_EVENT_EXEC)?;
        // At this stop no new program instruction has run, and the kernel now
        // excludes writes. Also prove this is our retained ELF, not an interpreter
        // selected through a script/binfmt handler. Refuse any unexpected object.
        let running = fs::File::open(format!("/proc/{pid}/exe"))?.metadata()?;
        let retained = self.file.metadata()?;
        require(
            running.dev() == retained.dev() && running.ino() == retained.ino(),
            "user_package_executable_identity",
        )?;
        self.verify()?;
        ptrace(libc::PTRACE_DETACH, pid, 0)?;
        Ok(child)
    }
}
fn ptrace(request: libc::c_uint, pid: libc::pid_t, data: usize) -> Result<()> {
    if unsafe { libc::ptrace(request, pid, std::ptr::null_mut::<libc::c_void>(), data) } < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}
struct Child {
    pid: libc::pid_t,
    reaped: bool,
}
impl Child {
    fn stopped(&mut self, signal: i32, event: i32) -> Result<()> {
        // Bounds only stop observation, not filesystem verification or reaping.
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let mut status = 0;
            let result =
                unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG | libc::__WALL) };
            if result == self.pid {
                self.reaped = libc::WIFEXITED(status) || libc::WIFSIGNALED(status);
                return require(
                    libc::WIFSTOPPED(status)
                        && libc::WSTOPSIG(status) == signal
                        && status >> 16 == event,
                    "user_package_executable_stop_failed",
                );
            }
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::ECHILD) {
                    // We no longer own a waitable PID; never signal a reused PID.
                    self.reaped = true;
                }
                if error.kind() != std::io::ErrorKind::Interrupted {
                    return Err(error.into());
                }
            }
            require(
                Instant::now() < deadline,
                "user_package_executable_stop_timeout",
            )?;
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    fn wait(&mut self) -> Result<i32> {
        loop {
            let mut status = 0;
            if unsafe { libc::waitpid(self.pid, &mut status, 0) } == self.pid {
                self.reaped = true;
                return Ok(status);
            }
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ECHILD) {
                self.reaped = true;
            }
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error.into());
            }
        }
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        if !self.reaped {
            // Refusal, panic or failed reaper creation cannot release unverified
            // code. Never detach on an error. Reaping is blocking: uninterruptible
            // kernel I/O can delay cleanup beyond the stop-observation deadline.
            unsafe {
                libc::kill(self.pid, libc::SIGKILL);
            }
            let _ = self.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable_package::tests::Fixture;
    use std::process::{Command, Stdio};

    fn probe(f: &Fixture, label: &str) -> Vec<u8> {
        // Tiny source-owned ELF, no display/runtime or installed package needed.
        // It records both kernel path and argv[0], plus the live write exclusion
        // and cleared death signal. The replacement leaves a distinct marker.
        let source = format!(
            r#"
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/prctl.h>
int main(int argc, char **argv) {{
    char path[4096]; int death = -1;
    ssize_t n = readlink("/proc/self/exe", path, sizeof(path)-1);
    if (n < 0) return 10;
    path[n] = 0;
    int fd = open("/proc/self/exe", O_WRONLY);
    int busy = fd < 0 && errno == ETXTBSY;
    if (fd >= 0) close(fd);
    if (prctl(PR_GET_PDEATHSIG, &death) != 0) return 11;
    FILE *f = fopen({}, "w");
    if (!f) return 12;
    fprintf(f, "{}\n%s\n%s\n%d %d\n", path, argv[0], busy, death);
    return fclose(f) != 0;
}}
"#,
            serde_json::to_string(&f.home.join("executed").to_str().unwrap()).unwrap(),
            label
        );
        let output = f.home.join(format!("probe-{label}"));
        let mut cc = Command::new("cc")
            .args(["-x", "c", "-o"])
            .arg(&output)
            .arg("-")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        cc.stdin
            .take()
            .unwrap()
            .write_all(source.as_bytes())
            .unwrap();
        assert!(
            cc.wait().unwrap().success(),
            "Linux custody tests require a C compiler"
        );
        fs::read(output).unwrap()
    }
    fn staged() -> (Fixture, PathBuf, Vec<u8>) {
        let mut f = Fixture::new();
        let original = probe(&f, "original");
        let replacement = probe(&f, "replacement");
        let (row, bytes) = f
            .rows
            .iter_mut()
            .find(|(row, _)| row.kind == "frontend")
            .unwrap();
        row.size = original.len() as u64;
        row.sha256 = hex(&Sha256::digest(&original));
        *bytes = original;
        let frontend = Bundle::open(&f.bundle("1.0"), f.trust())
            .unwrap()
            .stage(&f.home)
            .unwrap();
        (f, frontend, replacement)
    }
    fn retain(f: &Fixture, path: &Path) -> VerifiedExecutable {
        retained_input_executable_with_trust(
            path,
            &f.home,
            "linux-audio-compatibility-manager",
            &f.trust(),
        )
        .unwrap()
    }
    fn run(executable: VerifiedExecutable) {
        let mut child = executable.start().unwrap();
        let status = child.wait().unwrap();
        assert!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0);
    }
    #[test]
    fn ordinary_launch_preserves_package_path_and_releases_tracing() {
        let (f, path, _) = staged();
        run(retain(&f, &path));
        assert_eq!(
            fs::read_to_string(f.home.join("executed")).unwrap(),
            format!("original\n{}\n{}\n1 0\n", path.display(), path.display())
        );
        assert!(input_root_record_with_trust(&path, &f.home, &f.trust())
            .unwrap()
            .is_some());
    }
    // Thread-local mask ownership; restore even if a test assertion unwinds.
    struct BlockedTrap {
        previous: libc::sigset_t,
        _thread: std::marker::PhantomData<std::rc::Rc<()>>,
    }
    impl BlockedTrap {
        fn new() -> Self {
            unsafe {
                let mut mask = std::mem::zeroed();
                let mut previous = std::mem::zeroed();
                assert_eq!(libc::sigemptyset(&mut mask), 0);
                assert_eq!(libc::sigaddset(&mut mask, libc::SIGTRAP), 0);
                assert_eq!(
                    libc::pthread_sigmask(libc::SIG_BLOCK, &mask, &mut previous),
                    0
                );
                Self {
                    previous,
                    _thread: std::marker::PhantomData,
                }
            }
        }
    }
    impl Drop for BlockedTrap {
        fn drop(&mut self) {
            assert_eq!(
                unsafe {
                    libc::pthread_sigmask(libc::SIG_SETMASK, &self.previous, std::ptr::null_mut())
                },
                0
            );
        }
    }
    #[test]
    fn blocked_sigtrap_unchanged_launch_preserves_package_path() {
        let (f, path, _) = staged();
        let executable = retain(&f, &path);
        let _mask = BlockedTrap::new();
        run(executable);
        assert_eq!(
            fs::read_to_string(f.home.join("executed")).unwrap(),
            format!("original\n{}\n{}\n1 0\n", path.display(), path.display())
        );
    }
    #[test]
    fn blocked_sigtrap_in_place_replacement_never_writes_marker() {
        let (f, path, replacement) = staged();
        let executable = retain(&f, &path);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(&path, replacement).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        let _mask = BlockedTrap::new();
        assert!(executable.start().is_err());
        // start's refusal kills and reaps before returning: this checks absence
        // of any replacement side effect, not merely eventual rejection.
        assert!(!f.home.join("executed").exists());
    }
    #[test]
    fn unrelated_signal_stops_are_refused_and_reaped() {
        for signal in [libc::SIGTRAP, libc::SIGSTOP, libc::SIGUSR1] {
            let pid = unsafe { libc::fork() };
            assert!(pid >= 0);
            if pid == 0 {
                unsafe {
                    if libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) != 0 {
                        libc::_exit(127);
                    }
                    libc::raise(libc::SIGSTOP);
                    libc::raise(signal);
                    libc::_exit(99);
                }
            }
            let mut child = Child { pid, reaped: false };
            child.stopped(libc::SIGSTOP, 0).unwrap();
            ptrace(
                libc::PTRACE_SETOPTIONS,
                pid,
                (libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_EXITKILL) as usize,
            )
            .unwrap();
            ptrace(libc::PTRACE_CONT, pid, 0).unwrap();
            assert_eq!(
                child
                    .stopped(libc::SIGTRAP, libc::PTRACE_EVENT_EXEC)
                    .unwrap_err()
                    .to_string(),
                "user_package_executable_stop_failed"
            );
            assert!(
                !child.reaped,
                "must observe the unrelated stop, not child exit"
            );
            drop(child);
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
        }
    }
    #[test]
    fn replacing_verified_entry_never_executes_replacement() {
        let (f, path, replacement) = staged();
        let executable = retain(&f, &path);
        fs::rename(&path, path.with_extension("old")).unwrap();
        fs::write(&path, replacement).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        run(executable);
        assert!(fs::read_to_string(f.home.join("executed"))
            .unwrap()
            .starts_with("original\n"));
    }
    #[test]
    fn replacing_verified_parent_never_executes_replacement() {
        let (f, path, replacement) = staged();
        let executable = retain(&f, &path);
        let parent = path.parent().unwrap();
        fs::rename(parent, parent.with_extension("old")).unwrap();
        directory(parent).unwrap();
        fs::write(&path, replacement).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        run(executable);
        assert!(fs::read_to_string(f.home.join("executed"))
            .unwrap()
            .starts_with("original\n"));
    }
    #[test]
    fn completed_in_place_write_is_refused_before_any_instruction_runs() {
        let (f, path, replacement) = staged();
        let executable = retain(&f, &path);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(&path, replacement).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(executable.start().is_err());
        assert!(!f.home.join("executed").exists());
    }
    #[test]
    fn same_size_in_place_change_is_refused_by_digest_at_exec_stop() {
        let (f, path, _) = staged();
        let executable = retain(&f, &path);
        let mut bytes = fs::read(&path).unwrap();
        // Change ELF trailing metadata, preserving the loadable program and size.
        *bytes.last_mut().unwrap() ^= 1;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        let error = executable
            .start()
            .err()
            .expect("modified ELF must be refused");
        assert_eq!(error.to_string(), "user_package_executable_changed");
        assert!(!f.home.join("executed").exists());
    }
    #[test]
    fn open_writer_refuses_exec() {
        let (f, path, _) = staged();
        let executable = retain(&f, &path);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let writer = fs::OpenOptions::new().write(true).open(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        assert!(executable.start().is_err());
        assert!(!f.home.join("executed").exists());
        drop(writer);
    }
    #[test]
    fn writable_mapping_refuses_exec_even_after_writer_fd_is_closed() {
        let (f, path, _) = staged();
        let executable = retain(&f, &path);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let writer = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mapping = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                writer.as_raw_fd(),
                0,
            )
        };
        assert_ne!(mapping, libc::MAP_FAILED);
        drop(writer);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        let result = executable.start();
        unsafe {
            libc::munmap(mapping, 4096);
        }
        assert!(result.is_err());
        assert!(!f.home.join("executed").exists());
    }
}

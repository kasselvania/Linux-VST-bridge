//! Execute a sealed, signed snapshot; the staged inode is never executed.
//!
//! Linux deny_write_access is not a portable freeze of shared writable mappings.
//! Copy from the retained input, seal against writes/growth/shrinkage, then hash
//! the sealed bytes. Neither a racing copy nor later source mutation can authorize
//! unsigned bytes. No ptrace or host security policy changes are required.
//! `/proc/self/exe` identifies the snapshot. argv[0] is only a package locator;
//! current_frontend_path validates it against the running sealed signed bytes.
use super::*;
use std::{
    ffi::CString,
    os::fd::FromRawFd,
    os::unix::process::CommandExt,
    process::{Child, Command},
};

const SEALS: i32 = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;

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
        verify_bytes(&mut executable.file, row)?;
        Ok(executable)
    }
    fn snapshot(&mut self) -> Result<fs::File> {
        let name = CString::new("bridge-setup")?;
        // Explicit executable memfd on Linux >=6.3; older kernels only accept
        // the original flags. EINVAL is the only fallback, never EACCES/EPERM.
        const MFD_EXEC: u32 = 0x0010;
        let flags = libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING;
        let mut fd = unsafe { libc::memfd_create(name.as_ptr(), flags | MFD_EXEC) };
        if fd < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINVAL) {
            fd = unsafe { libc::memfd_create(name.as_ptr(), flags) };
        }
        if fd < 0 {
            return Err(format!(
                "Setup requires executable sealed memory: {}",
                std::io::Error::last_os_error()
            )
            .into());
        }
        let mut snapshot = unsafe { fs::File::from_raw_fd(fd) };
        self.file.seek(SeekFrom::Start(0))?;
        require(
            std::io::copy(&mut (&mut self.file).take(self.row.size), &mut snapshot)?
                == self.row.size,
            "user_package_executable_changed",
        )?;
        snapshot.set_permissions(fs::Permissions::from_mode(0o555))?;
        if unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, SEALS) } < 0 {
            return Err(format!(
                "Setup requires immutable executable seals: {}",
                std::io::Error::last_os_error()
            )
            .into());
        }
        require_sealed(&snapshot)?;
        // Only this post-seal hash authorizes execution, never the source hash.
        verify_bytes(&mut snapshot, &self.row)?;
        Ok(snapshot)
    }
    pub fn spawn(self) -> Result<()> {
        let child = self.start()?;
        // Keep owned cleanup even if creation of the reaper thread fails.
        let mut child = Reap(Some(child));
        std::thread::Builder::new()
            .name("bridge-setup-reaper".into())
            .spawn(move || {
                if let Some(mut process) = child.0.take() {
                    let _ = process.wait();
                }
            })?;
        Ok(())
    }
    fn start(mut self) -> Result<Child> {
        let snapshot = self.snapshot()?;
        // CLOEXEC keeps the fd available until the kernel resolves this name,
        // then closes it in the new ELF. Command reports exec errors synchronously.
        Ok(
            Command::new(format!("/proc/self/fd/{}", snapshot.as_raw_fd()))
                .arg0(&self.path)
                .spawn()?,
        )
    }
}
struct Reap(Option<Child>);
impl Drop for Reap {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn require_sealed(file: &fs::File) -> Result<()> {
    let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
    require(
        seals >= 0 && seals & SEALS == SEALS,
        "user_package_executable_unsealed",
    )
}
fn verify_bytes(file: &mut fs::File, row: &ReleaseFile) -> Result<()> {
    let meta = file.metadata()?;
    require(
        meta.is_file()
            && meta.uid() == unsafe { libc::getuid() }
            && meta.mode() & 0o7777 == 0o555
            && meta.len() == row.size,
        "user_package_executable_custody",
    )?;
    file.seek(SeekFrom::Start(0))?;
    let mut magic = [0; 4];
    file.read_exact(&mut magic)?;
    require(magic == *b"\x7fELF", "user_package_executable_format")?;
    file.seek(SeekFrom::Start(0))?;
    let mut hash = Sha256::new();
    let mut remaining = row.size;
    let mut block = [0; 65536];
    while remaining != 0 {
        let bound = block.len().min(remaining as usize);
        file.read_exact(&mut block[..bound])?;
        hash.update(&block[..bound]);
        remaining -= bound as u64;
    }
    require(
        hex(&hash.finalize()) == row.sha256,
        "user_package_executable_changed",
    )
}

/// Logical frontend location for package/bootstrap and selected-client routing.
/// Disk launches retain current_exe semantics. A sealed launch must prove its
/// argv[0] locator names a signed frontend with exactly the running bytes.
/// No inherited environment variable can redirect this resolution.
pub fn current_frontend_path() -> Result<PathBuf> {
    current_frontend_path_with_trust(Trust::compiled)
}
fn current_frontend_path_with_trust(trust: impl FnOnce() -> Result<Trust>) -> Result<PathBuf> {
    let current = std::env::current_exe()?;
    let mut running = fs::File::open("/proc/self/exe")?;
    // Ordinary on-disk frontends (including tmpfs) keep their existing route.
    if let Ok(named) = fs::metadata(&current) {
        let actual = running.metadata()?;
        require(
            named.is_file() && named.dev() == actual.dev() && named.ino() == actual.ino(),
            "user_package_executable_identity",
        )?;
        return Ok(current);
    }
    require_sealed(&running)?;
    let locator = PathBuf::from(
        std::env::args_os()
            .next()
            .ok_or("user_package_source_required")?,
    );
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("Home directory unavailable")?);
    sealed_frontend_path(&mut running, &locator, &home, &trust()?)
}
fn sealed_frontend_path(
    running: &mut fs::File,
    locator: &Path,
    home: &Path,
    trust: &Trust,
) -> Result<PathBuf> {
    require_sealed(running)?;
    let (root, release) = input_release_record_with_trust(locator, home, trust)?
        .ok_or("user_package_source_required")?;
    let row = release
        .files
        .iter()
        .find(|row| row.kind == "frontend" && root.join(&row.destination) == locator)
        .ok_or("user_package_executable_identity")?;
    verify_bytes(running, row)?;
    Ok(locator.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portable_package::tests::Fixture;
    use std::process::{Command, Stdio};

    fn probe(f: &Fixture, label: &str) -> Vec<u8> {
        // Tiny source-owned ELF, no display/runtime or installed package needed.
        // It records both kernel path and argv[0], plus the immutable seals
        // and cleared death signal. The replacement leaves a distinct marker.
        let source = format!(
            r#"
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/prctl.h>
#include <signal.h>
int main(int argc, char **argv) {{
    if (access({}, F_OK) == 0) raise(SIGSTOP);
    char path[4096]; int death = -1;
    ssize_t n = readlink("/proc/self/exe", path, sizeof(path)-1);
    if (n < 0) return 10;
    path[n] = 0;
    int fd = open("/proc/self/exe", O_RDONLY);
    int seals = fd < 0 ? -1 : fcntl(fd, F_GET_SEALS);
    int busy = seals >= 0 && (seals & 15) == 15;
    if (fd >= 0) close(fd);
    if (prctl(PR_GET_PDEATHSIG, &death) != 0) return 11;
    FILE *f = fopen({}, "w");
    if (!f) return 12;
    fprintf(f, "{}\n%s\n%s\n%d %d\n", path, argv[0], busy, death);
    return fclose(f) != 0;
}}
"#,
            serde_json::to_string(&f.home.join("pause").to_str().unwrap()).unwrap(),
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
        assert!(status.success());
    }
    #[test]
    fn ordinary_launch_preserves_argv_and_uses_sealed_executable() {
        let (f, path, _) = staged();
        run(retain(&f, &path));
        assert_eq!(
            fs::read_to_string(f.home.join("executed")).unwrap(),
            format!(
                "original\n/memfd:bridge-setup (deleted)\n{}\n1 0\n",
                path.display()
            )
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
            format!(
                "original\n/memfd:bridge-setup (deleted)\n{}\n1 0\n",
                path.display()
            )
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
    fn same_size_in_place_change_is_refused_by_post_seal_digest() {
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
    struct Mapping {
        address: *mut libc::c_void,
        len: usize,
    }
    impl Mapping {
        fn new(path: &Path) -> Self {
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
            let writer = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .unwrap();
            let len = writer.metadata().unwrap().len() as usize;
            let address = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    writer.as_raw_fd(),
                    0,
                )
            };
            assert_ne!(address, libc::MAP_FAILED);
            drop(writer);
            fs::set_permissions(path, fs::Permissions::from_mode(0o555)).unwrap();
            Self { address, len }
        }
        fn tamper(&self, offset: usize) {
            assert!(offset + 8 <= self.len);
            unsafe {
                std::ptr::copy_nonoverlapping(
                    b"tampered".as_ptr(),
                    (self.address as *mut u8).add(offset),
                    8,
                );
                assert_eq!(libc::msync(self.address, self.len, libc::MS_SYNC), 0);
            }
        }
    }
    impl Drop for Mapping {
        fn drop(&mut self) {
            unsafe {
                libc::munmap(self.address, self.len);
            }
        }
    }
    fn marker_offset(path: &Path) -> usize {
        let bytes = fs::read(path).unwrap();
        let offsets: Vec<_> = bytes
            .windows(9)
            .enumerate()
            .filter_map(|(i, bytes)| (bytes == b"original\n").then_some(i))
            .collect();
        assert_eq!(offsets.len(), 1);
        offsets[0]
    }
    fn stopped(child: &mut Reap) {
        let pid = child.0.as_ref().unwrap().id() as i32;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let mut status = 0;
            let result =
                unsafe { libc::waitpid(pid, &mut status, libc::WUNTRACED | libc::WNOHANG) };
            if result == pid {
                assert!(libc::WIFSTOPPED(status) && libc::WSTOPSIG(status) == libc::SIGSTOP);
                return;
            }
            assert_eq!(result, 0);
            assert!(std::time::Instant::now() < deadline, "probe did not stop");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    fn resume(mut child: Reap) {
        assert_eq!(
            unsafe { libc::kill(child.0.as_ref().unwrap().id() as i32, libc::SIGCONT) },
            0
        );
        assert!(child.0.as_mut().unwrap().wait().unwrap().success());
        child.0.take();
    }
    #[test]
    fn disk_exec_mapping_demonstrates_custody_boundary() {
        // Counterexample to the old write-exclusion premise: hash AFTER exec,
        // then mutate a live ELF string through the preexisting closed-fd mmap.
        let (f, path, _) = staged();
        let mapping = Mapping::new(&path);
        let offset = marker_offset(&path);
        let original_hash = digest(&path).unwrap();
        fs::write(f.home.join("pause"), b"").unwrap();
        let process = match Command::new(&path).spawn() {
            Ok(child) => child,
            Err(error) => {
                // Some filesystems retain the write exclusion through the mmap.
                // That is the refusal control, not evidence of a mutation exploit.
                assert_eq!(error.raw_os_error(), Some(libc::ETXTBSY));
                assert!(!f.home.join("executed").exists());
                eprintln!("disk mapping control: kernel refused exec with ETXTBSY");
                return;
            }
        };
        let mut child = Reap(Some(process));
        stopped(&mut child);
        let pid = child.0.as_ref().unwrap().id();
        assert_eq!(
            hex(&Sha256::digest(
                fs::read(format!("/proc/{pid}/exe")).unwrap()
            )),
            original_hash
        );
        mapping.tamper(offset);
        assert_ne!(digest(&path).unwrap(), original_hash);
        resume(child);
        assert!(fs::read_to_string(f.home.join("executed"))
            .unwrap()
            .starts_with("tampered\n"));
        eprintln!("disk mapping counterexample: post-exec hash passed, then mapped mutation executed tampered marker");
    }
    #[test]
    fn writable_mapping_after_fd_close_cannot_mutate_verified_running_snapshot() {
        let (f, path, _) = staged();
        let executable = retain(&f, &path);
        let mapping = Mapping::new(&path);
        let offset = marker_offset(&path);
        fs::write(f.home.join("pause"), b"").unwrap();
        let mut child = Reap(Some(executable.start().unwrap()));
        stopped(&mut child);
        let running = PathBuf::from(format!("/proc/{}/exe", child.0.as_ref().unwrap().id()));
        let original_hash = hex(&Sha256::digest(fs::read(&running).unwrap()));
        assert_eq!(digest(&path).unwrap(), original_hash);
        mapping.tamper(offset);
        assert_ne!(digest(&path).unwrap(), original_hash);
        assert_eq!(
            hex(&Sha256::digest(fs::read(&running).unwrap())),
            original_hash
        );
        resume(child);
        assert!(fs::read_to_string(f.home.join("executed"))
            .unwrap()
            .starts_with("original\n"));
    }
    #[test]
    fn mapping_mutation_before_snapshot_is_refused_without_marker() {
        let (f, path, _) = staged();
        let executable = retain(&f, &path);
        let mapping = Mapping::new(&path);
        mapping.tamper(marker_offset(&path));
        assert!(executable.start().is_err());
        assert!(!f.home.join("executed").exists());
    }
    #[test]
    fn sealed_snapshot_refuses_writes_resize_and_shared_writable_mapping() {
        use std::os::unix::fs::FileExt;
        let (f, path, _) = staged();
        let mut executable = retain(&f, &path);
        let snapshot = executable.snapshot().unwrap();
        require_sealed(&snapshot).unwrap();
        assert_eq!(
            snapshot.write_at(b"bad", 0).unwrap_err().raw_os_error(),
            Some(libc::EPERM)
        );
        assert_eq!(
            snapshot.set_len(0).unwrap_err().raw_os_error(),
            Some(libc::EPERM)
        );
        assert_eq!(
            snapshot
                .set_len(executable.row.size + 1)
                .unwrap_err()
                .raw_os_error(),
            Some(libc::EPERM)
        );
        let mapping = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                snapshot.as_raw_fd(),
                0,
            )
        };
        assert_eq!(mapping, libc::MAP_FAILED);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
    }
    #[test]
    fn open_source_writer_cannot_change_snapshot_after_verification() {
        use std::os::unix::fs::FileExt;
        let (f, path, _) = staged();
        let executable = retain(&f, &path);
        let offset = marker_offset(&path);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        let writer = fs::OpenOptions::new().write(true).open(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o555)).unwrap();
        fs::write(f.home.join("pause"), b"").unwrap();
        let mut child = Reap(Some(executable.start().unwrap()));
        stopped(&mut child);
        writer.write_all_at(b"tampered", offset as u64).unwrap();
        resume(child);
        assert!(fs::read_to_string(f.home.join("executed"))
            .unwrap()
            .starts_with("original\n"));
    }
    #[test]
    fn sealed_bootstrap_requires_exact_signed_frontend_and_seals() {
        let (f, path, replacement) = staged();
        let mut executable = retain(&f, &path);
        let mut snapshot = executable.snapshot().unwrap();
        assert_eq!(
            sealed_frontend_path(&mut snapshot, &path, &f.home, &f.trust()).unwrap(),
            path
        );
        assert!(sealed_frontend_path(&mut executable.file, &path, &f.home, &f.trust()).is_err());
        assert!(sealed_frontend_path(
            &mut snapshot,
            Path::new("/usr/bin/linux-audio-compatibility-manager"),
            &f.home,
            &f.trust()
        )
        .is_err());
        let manager = path.with_file_name("linux-vst-bridge");
        assert!(sealed_frontend_path(&mut snapshot, &manager, &f.home, &f.trust()).is_err());
        // A different, validly signed package is not authority for this snapshot.
        let mut other = Fixture::new();
        let (row, bytes) = other
            .rows
            .iter_mut()
            .find(|(r, _)| r.kind == "frontend")
            .unwrap();
        row.size = replacement.len() as u64;
        row.sha256 = hex(&Sha256::digest(&replacement));
        *bytes = replacement;
        let other_path = Bundle::open(&other.bundle("2.0"), other.trust())
            .unwrap()
            .stage(&other.home)
            .unwrap();
        assert!(
            sealed_frontend_path(&mut snapshot, &other_path, &other.home, &other.trust()).is_err()
        );
        fs::rename(&path, path.with_extension("old")).unwrap();
        assert!(sealed_frontend_path(&mut snapshot, &path, &f.home, &f.trust()).is_err());
    }
    #[test]
    fn write_seal_refuses_preexisting_writable_mapping() {
        let name = CString::new("seal-control").unwrap();
        let fd = unsafe {
            libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING)
        };
        assert!(fd >= 0);
        let file = unsafe { fs::File::from_raw_fd(fd) };
        file.set_len(4096).unwrap();
        let address = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        assert_ne!(address, libc::MAP_FAILED);
        let mapping = Mapping { address, len: 4096 };
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, SEALS) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBUSY)
        );
        drop(mapping);
        assert_eq!(unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, SEALS) }, 0);
        require_sealed(&file).unwrap();
    }
    #[test]
    fn running_frontend_probe() {
        // Subprocess helper; standalone invocation does not count as a scenario.
        let Some(home) = std::env::var_os("LVB_SEALED_FRONTEND_PROBE") else {
            return;
        };
        let home = PathBuf::from(home);
        let key = ed25519_dalek::SigningKey::from_bytes(&[0x51; 32]);
        let trust = Trust::new(key.verifying_key().to_bytes(), "internal_test").unwrap();
        let path = current_frontend_path_with_trust(|| Ok(trust.clone())).unwrap();
        assert_eq!(path, PathBuf::from(std::env::args_os().next().unwrap()));
        let root = input_root_record_with_trust(&path, &home, &trust)
            .unwrap()
            .unwrap();
        assert_eq!(root.join("bin/linux-audio-compatibility-manager"), path);
        assert!(
            input_executable_with_trust(&path, &home, "linux-vst-bridge", &trust)
                .unwrap()
                .is_some()
        );
        fs::write(home.join("bootstrap-complete"), b"verified").unwrap();
    }
    #[test]
    fn sealed_exec_resolves_running_bootstrap_root_and_manager() {
        let mut f = Fixture::new();
        let bytes = fs::read("/proc/self/exe").unwrap();
        let (row, payload) = f
            .rows
            .iter_mut()
            .find(|(r, _)| r.kind == "frontend")
            .unwrap();
        row.size = bytes.len() as u64;
        row.sha256 = hex(&Sha256::digest(&bytes));
        *payload = bytes;
        let path = Bundle::open(&f.bundle("1.0"), f.trust())
            .unwrap()
            .stage(&f.home)
            .unwrap();
        let mut executable = retain(&f, &path);
        let snapshot = executable.snapshot().unwrap();
        let status = Command::new(format!("/proc/self/fd/{}", snapshot.as_raw_fd()))
            .arg0(&path)
            .args([
                "--exact",
                "portable_package::executable::tests::running_frontend_probe",
                "--nocapture",
            ])
            .env("HOME", &f.home)
            .env("LVB_SEALED_FRONTEND_PROBE", &f.home)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            fs::read(f.home.join("bootstrap-complete")).unwrap(),
            b"verified"
        );
    }
}

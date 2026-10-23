//! Explicit first-run handoff for the fixed system package frontend.
//! PKG0 owns every adoption, refusal, route and rollback decision.
use super::{Completion, Operation};
use std::{
    io::Read,
    os::fd::AsRawFd,
    os::unix::fs::{FileExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const SYSTEM_FRONTEND: &str = "/usr/bin/linux-audio-compatibility-manager";
const SYSTEM_MANAGER: &str = "/usr/bin/linux-vst-bridge";

#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    Checking,
    Ordinary,
}

#[cfg(test)]
fn selected_frontend(home: &Path) -> PathBuf {
    home.join(".local/bin/linux-audio-compatibility-manager")
}

fn select_entry(executable: &Path) -> Entry {
    if executable != Path::new(SYSTEM_FRONTEND) {
        Entry::Ordinary
    } else {
        Entry::Checking
    }
}

pub fn entry() -> Result<Entry, String> {
    let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("Home directory unavailable")?);
    if linux_vst_bridge::portable_package::input_root_record(&executable, &home)
        .map_err(|e| e.to_string())?.is_some() {
        return Ok(Entry::Checking);
    }
    if executable != Path::new(SYSTEM_FRONTEND) {
        return Ok(Entry::Ordinary);
    }
    Ok(select_entry(&executable))
}

use crate::launch_selected_with;

pub fn launch_selected() -> Result<(), String> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("Home directory unavailable")?);
    let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
    launch_selected_with(&home, &executable, |frontend| {
        Command::new(frontend).spawn().map(|_| ())
            .map_err(|_| "Selected application could not be opened".into())
    })
}

fn fixed_command(operation: Operation) -> Result<Command, String> {
    let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("Home directory unavailable")?);
    let manager = linux_vst_bridge::portable_package::input_executable(&executable,&home,
        "linux-vst-bridge")
        .map_err(|e|e.to_string())?
        .unwrap_or_else(|| PathBuf::from(SYSTEM_MANAGER));
    let mut command = Command::new(manager);
    command.arg(match operation {
        Operation::Adopt => "package-adopt",
        Operation::Activate => "package-activate",
        Operation::Update => "package-update",
        Operation::Restore => "package-restore",
        Operation::Recover => "package-recover",
        Operation::StopForRepair => "package-stop-for-repair",
        Operation::Status => "package-bootstrap-status",
        Operation::SelectedStatus => "package-activation-status",
    });
    Ok(command)
}

fn nonblocking(fd: i32) -> Result<(), String> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err("Package result stream unavailable".into());
    }
    Ok(())
}

fn collect_pipe<R: Read>(
    pipe: &mut Option<R>,
    data: &mut Vec<u8>,
    limit: usize,
) -> Result<(), String> {
    let Some(reader) = pipe.as_mut() else {
        return Ok(());
    };
    loop {
        let mut buffer = [0u8; 1024];
        match reader.read(&mut buffer) {
            Ok(0) => {
                *pipe = None;
                return Ok(());
            }
            Ok(count) => {
                if data.len().saturating_add(count) > limit {
                    return Err("Package result exceeded its bound".into());
                }
                data.extend_from_slice(&buffer[..count]);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(_) => return Err("Package result stream failed".into()),
        }
    }
}

fn stop_observing(
    operation: Operation,
    mut child: std::process::Child,
    mut stdout: Option<std::process::ChildStdout>,
    mut stderr: Option<std::process::ChildStderr>,
) {
    if !operation.owns_transition() {
        let _ = child.kill();
        let _ = child.wait();
        return;
    }
    // A lost UI observation must not kill the transaction owner. Drain its
    // pipes and reap it while the backend completes its own bounded operation.
    {
        let mut discarded = Vec::new();
        loop {
            let _ = collect_pipe(&mut stdout, &mut discarded, 4096);
            discarded.clear();
            let _ = collect_pipe(&mut stderr, &mut discarded, 4096);
            discarded.clear();
            match child.try_wait() {
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                _ => break,
            }
        }
    }
}

fn transition_output() -> Result<std::fs::File, String> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Package output identity unavailable")?.as_nanos();
    let path = std::env::temp_dir().join(format!("lvb-package-output-{}-{stamp}-{}",
        std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let file = std::fs::OpenOptions::new().read(true).write(true).create_new(true)
        .mode(0o600).open(&path).map_err(|_| "Private package output unavailable")?;
    std::fs::remove_file(&path).map_err(|_| "Private package output cleanup failed")?;
    // The backend inherits the open file, so GUI exit cannot break its output.
    // The unlinked private file lives until both owners close it. Durable change
    // status belongs to the manager's existing transition record.
    Ok(file)
}

fn execute(operation: Operation) -> Result<Vec<u8>, String> {
    let retained_error = operation.owns_transition().then(transition_output).transpose()?;
    let error_output = match retained_error.as_ref() {
        Some(file) => Stdio::from(file.try_clone().map_err(|_| "Package output unavailable")?),
        None => Stdio::piped(),
    };
    let mut child = fixed_command(operation)?
        .stdin(Stdio::null())
        .stdout(if matches!(operation, Operation::Status | Operation::SelectedStatus) {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(error_output)
        .spawn()
        .map_err(|_| "Installed package manager is unavailable".to_owned())?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    for fd in [
        stdout.as_ref().map(AsRawFd::as_raw_fd),
        stderr.as_ref().map(AsRawFd::as_raw_fd),
    ]
    .into_iter()
    .flatten()
    {
        if let Err(reason) = nonblocking(fd) {
            stop_observing(operation, child, stdout, stderr);
            return Err(reason);
        }
    }
    let mut output = Vec::new();
    let mut error = Vec::new();
    // The coordinated owner bounds preparation and cleanup. Killing that owner
    // at a separate UI deadline could strand its supervised children. Keep the
    // window responsive while awaiting its final result; closing the window
    // does not cancel or replace the managed transition.
    let deadline = (!operation.owns_transition()).then(|| Instant::now()
        // Setup status may first finish work whose owner ended with the
        // machine and re-check an app session; the manager keeps that within
        // its 60-second settling budget and waits on no service restoration.
        + Duration::from_secs(if operation == Operation::Status {
            90
        } else if operation == Operation::SelectedStatus {
            10
        } else {
            300
        }));
    let mut exited_at = None;
    let status = loop {
        let readback = collect_pipe(&mut stdout, &mut output, 4096)
            .and_then(|_| collect_pipe(&mut stderr, &mut error, 4096));
        if let Err(reason) = readback {
            stop_observing(operation, child, stdout, stderr);
            return Err(reason);
        }
        let child_status = match child.try_wait() {
            Ok(status) => status,
            Err(_) => {
                stop_observing(operation, child, stdout, stderr);
                return Err("Package wait failed; reopen setup to check the retained operation".into());
            }
        };
        if let Some(status) = child_status {
            if stdout.is_none() && stderr.is_none() {
                break status;
            }
            let since = exited_at.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(1) {
                return Err("Package result stream remained open after the manager exited".into());
            }
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if matches!(operation, Operation::Status | Operation::SelectedStatus) {
                "The bridge status check took too long. Choose Check again; no update or repair was started.".into()
            } else {
                "Package setup timed out. Reopen the application to inspect or recover its retained transition; no second request was submitted.".into()
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if let Some(file) = retained_error {
        let length = file.metadata().map_err(|_| "Package result unavailable")?.len();
        if length > 4096 { return Err("Package result exceeded its bound; reopen setup to check the completed operation".into()); }
        error.resize(length as usize, 0);
        file.read_exact_at(&mut error, 0).map_err(|_| "Package result unavailable")?;
    }
    if !status.success() {
        let refusal = String::from_utf8_lossy(&error);
        let bounded = refusal.chars().take(4096).collect::<String>();
        return Err(if bounded.trim().is_empty() {
            "Package setup was refused; reason unavailable".into()
        } else {
            bounded
        });
    }
    Ok(output)
}

fn confirmation_readback(operation: Operation) -> Operation {
    if matches!(operation, Operation::Adopt | Operation::Activate | Operation::Update | Operation::Restore) {
        Operation::SelectedStatus
    } else {
        Operation::Status
    }
}

pub fn run(operation: Operation) -> Result<Completion, String> {
    if !matches!(operation, Operation::Status | Operation::SelectedStatus) {
        execute(operation)?;
    }
    // A completed adoption or rollback may leave a verified predecessor, so
    // bootstrap status can offer another switch. Finish the current step from
    // the selected generation's service state instead.
    let state = match confirmation_readback(operation) {
        Operation::SelectedStatus => selected_activation_status()?,
        Operation::Status => activation_status()?,
        _ => return Err("Package confirmation route is invalid".into()),
    };
    completed_status(operation, state)
}

fn completed_status(operation: Operation, state: &str) -> Result<Completion, String> {
    if matches!(operation, Operation::Update | Operation::Restore) {
        return if state == "active" { Ok(Completion::Selected) }
            else { Err("The bridge change has not completed. Reopen setup to recover the retained change.".into()) };
    }
    match state {
        "active" => Ok(Completion::Selected),
        "inactive" if operation != Operation::Activate => Ok(Completion::NeedsActivation),
        "inactive" => Err("The bridge service did not become active after setup".into()),
        "fresh_adoptable" => Ok(Completion::FreshAdoptable),
        "legacy_adoptable" => Ok(Completion::LegacyAdoptable),
        "legacy_active" => Ok(Completion::LegacyActive),
        "update_active" => Ok(Completion::UpdateActive),
        "update_adoptable" => Ok(Completion::UpdateAdoptable),
        "update_retirement_pending" => Ok(Completion::UpdateRetirementPending),
        "rollback_active" => Ok(Completion::RollbackActive),
        "rollback_inactive" => Ok(Completion::RollbackInactive),
        "rollback_retirement_pending" => Ok(Completion::RollbackRetirementPending),
        "repair_active" => Ok(Completion::RepairActive),
        "repair_inactive" => Ok(Completion::RepairInactive),
        "legacy_retirement_pending" => Ok(Completion::LegacyRetirementPending),
        "repair_retirement_pending" => Ok(Completion::RetirementPending),
        _ => Err("Package activation status is invalid".into()),
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationStatus {
    schema: u32,
    state: String,
    package_version: String,
}

fn activation_status() -> Result<&'static str, String> {
    let data = execute(Operation::Status)?;
    parse_activation_status(&data)
}

fn selected_activation_status() -> Result<&'static str, String> {
    let data = execute(Operation::SelectedStatus)?;
    parse_selected_activation_status(&data)
}

fn parse_activation_status(data: &[u8]) -> Result<&'static str, String> {
    if data.len() > 4096 {
        return Err("Package activation status exceeded its bound".into());
    }
    let result: ActivationStatus = serde_json::from_slice(data)
        .map_err(|_| "Package activation status is malformed".to_owned())?;
    if result.schema != 3 || result.package_version.is_empty() || result.package_version.len() > 80
    {
        return Err("Package activation status is incompatible".into());
    }
    match result.state.as_str() {
        "active" => Ok("active"),
        "inactive" => Ok("inactive"),
        "fresh_adoptable" => Ok("fresh_adoptable"),
        "legacy_adoptable" => Ok("legacy_adoptable"),
        "legacy_active" => Ok("legacy_active"),
        "update_active" => Ok("update_active"),
        "update_adoptable" => Ok("update_adoptable"),
        "update_retirement_pending" => Ok("update_retirement_pending"),
        "rollback_active" => Ok("rollback_active"),
        "rollback_inactive" => Ok("rollback_inactive"),
        "rollback_retirement_pending" => Ok("rollback_retirement_pending"),
        "repair_active" => Ok("repair_active"),
        "repair_inactive" => Ok("repair_inactive"),
        "legacy_retirement_pending" => Ok("legacy_retirement_pending"),
        "repair_retirement_pending" => Ok("repair_retirement_pending"),
        _ => Err("Package activation status is unknown".into()),
    }
}

fn parse_selected_activation_status(data: &[u8]) -> Result<&'static str, String> {
    if data.len() > 4096 {
        return Err("Selected application status exceeded its bound".into());
    }
    let result: ActivationStatus = serde_json::from_slice(data)
        .map_err(|_| "Selected application status is malformed".to_owned())?;
    if result.schema != 1 || result.package_version.is_empty() || result.package_version.len() > 80 {
        return Err("Selected application status is incompatible".into());
    }
    match result.state.as_str() {
        "active" => Ok("active"),
        "inactive" => Ok("inactive"),
        _ => Err("Selected application status is unknown".into()),
    }
}


/// Open only after the owner confirms the selected generation's service.
pub fn run_and_open(operation: Operation) -> Result<Completion, String> {
    let completion = run(operation)?;
    if completion == Completion::Selected { launch_selected()?; }
    Ok(completion)
}

/// One persistent command worker. Closing a window drops observation; it never
/// sends a cancellation to an accepted package transition.
pub struct Worker {
    jobs: std::sync::mpsc::SyncSender<Operation>,
    replies: std::sync::mpsc::Receiver<Result<Completion, String>>,
}
impl Default for Worker { fn default() -> Self { Self::new() } }
impl Worker {
    pub fn new() -> Self {
        let (jobs, incoming) = std::sync::mpsc::sync_channel(1);
        let (outgoing, replies) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new().name("bridge-package-setup".into()).spawn(move || {
            while let Ok(operation) = incoming.recv() {
                if outgoing.send(run_and_open(operation)).is_err() { break; }
            }
        }).expect("start package setup worker");
        Self { jobs, replies }
    }
    pub fn send(&self, operation: Operation) -> Result<(), String> {
        self.jobs.try_send(operation).map_err(|_| "Setup could not receive this choice. Check again.".into())
    }
    pub fn receive(&self) -> Option<Result<Completion, String>> {
        match self.replies.try_recv() {
            Ok(reply) => Some(reply),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(_) => Some(Err("Setup stopped. Reopen it to check the retained change.".into())),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use crate::bootstrap::{State as Bootstrap, plain_refusal};

    #[test]
    fn selected_handoff_verifies_exact_frontend_before_launch() {
        let home = std::env::temp_dir().join(format!("lvb-ui-launch-{}-{}",
            std::process::id(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&home).unwrap();
        let home = std::fs::canonicalize(home).unwrap();
        let generation = home.join(".local/share/linux-vst-bridge/managed/software/generation");
        std::fs::create_dir_all(&generation).unwrap();
        let artifact = |name: &str, bytes: &[u8], mode: u32| {
            let path = generation.join(name);
            std::fs::write(&path, bytes).unwrap();
            std::fs::set_permissions(&path,
                std::fs::Permissions::from_mode(mode)).unwrap();
            linux_vst_bridge::Artifact { sha256:linux_vst_bridge::digest(&path).unwrap(), path }
        };
        let manager = artifact("linux-vst-bridge", b"manager", 0o500);
        let frontend = artifact("linux-audio-compatibility-manager", b"frontend", 0o500);
        let supervisor = artifact("session.pyc", b"supervisor", 0o400);
        let ownership = artifact("ownership.pyc", b"ownership", 0o400);
        let host = artifact("host.exe", b"host", 0o400);
        let source_manifest = artifact("host-source-manifest.json", b"source", 0o400);
        let software = linux_vst_bridge::catalogue::Software {
            installer_launch:None, preparation_kit:None,
            operator_frontend:Some(frontend.clone()), manager,
            supervisor, ownership, host, source_sha256:source_manifest.sha256.clone(),
            source_manifest, native_catalogue:None,
        };
        linux_vst_bridge::atomic_json(
            &home.join(".local/share/linux-vst-bridge/managed/software.json"),
            &software).unwrap();
        std::fs::create_dir_all(home.join(".local/bin")).unwrap();
        std::os::unix::fs::symlink(&frontend.path,selected_frontend(&home)).unwrap();
        let launches=std::cell::Cell::new(0);
        launch_selected_with(&home,Path::new("/usr/bin/source-setup"),|path| {
            assert_eq!(path,&frontend.path);launches.set(launches.get()+1);Ok(())
        }).unwrap();
        assert_eq!(launches.get(),1);
        let software_path=home.join(".local/share/linux-vst-bridge/managed/software.json");
        let software_bytes=std::fs::read(&software_path).unwrap();
        std::fs::write(&software_path,vec![b'X';8*1024*1024+1]).unwrap();
        assert!(launch_selected_with(&home,Path::new("/usr/bin/source-setup"),|_| {
            launches.set(launches.get()+1);Ok(())
        }).is_err());
        std::fs::remove_file(&software_path).unwrap();
        let fifo=std::ffi::CString::new(
            std::os::unix::ffi::OsStrExt::as_bytes(software_path.as_os_str())).unwrap();
        assert_eq!(unsafe{libc::mkfifo(fifo.as_ptr(),0o600)},0);
        assert!(launch_selected_with(&home,Path::new("/usr/bin/source-setup"),|_| {
            launches.set(launches.get()+1);Ok(())
        }).is_err());
        std::fs::remove_file(&software_path).unwrap();
        let saved=software_path.with_extension("saved");
        std::fs::write(&saved,&software_bytes).unwrap();
        std::os::unix::fs::symlink(&saved,&software_path).unwrap();
        assert!(launch_selected_with(&home,Path::new("/usr/bin/source-setup"),|_| {
            launches.set(launches.get()+1);Ok(())
        }).is_err());
        std::fs::remove_file(&software_path).unwrap();
        std::fs::write(&software_path,software_bytes).unwrap();
        assert_eq!(launches.get(),1);
        std::fs::set_permissions(&frontend.path,
            std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(&frontend.path,b"changed!").unwrap();
        std::fs::set_permissions(&frontend.path,
            std::fs::Permissions::from_mode(0o500)).unwrap();
        assert!(launch_selected_with(&home,Path::new("/usr/bin/source-setup"),|_| {
            launches.set(launches.get()+1);Ok(())
        }).is_err());
        assert_eq!(launches.get(),1);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn transition_output_survives_observer_process_exit() {
        let marker = std::env::temp_dir().join(format!("lvb-ui-exit-{}-{}",
            std::process::id(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let output = transition_output().unwrap();
        let mut observer = Command::new("sh").args(["-c", r#"
            ( set -e; sleep 0.1; i=0
              while [ "$i" -lt 2000 ]; do
                printf 'owned transition output after observer exit\n' >&2
                i=$((i+1))
              done
              printf complete > "$1"
            ) &
            exit 0
        "#, "observer"]).arg(&marker).stdin(Stdio::null())
            .stdout(Stdio::null()).stderr(Stdio::from(output.try_clone().unwrap()))
            .spawn().unwrap();
        drop(output);
        assert!(observer.wait().unwrap().success());
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if std::fs::read(&marker).is_ok_and(|bytes| bytes == b"complete") {
                std::fs::remove_file(marker).unwrap();
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("owned transition must finish writing after its observer exits");
    }

    #[test]
    fn lost_observation_drains_and_reaps_the_transition_without_killing_it() {
        let marker = std::env::temp_dir().join(format!("lvb-ui-owner-{}-{}",
            std::process::id(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut child = Command::new("sh").args(["-c",
            "i=0; while [ \"$i\" -lt 2000 ]; do printf 'transition output exceeds the UI observation limit\\n' >&2; i=$((i+1)); done; printf complete > \"$1\"",
            "owned-transition"]).arg(&marker).stdout(Stdio::piped())
            .stderr(Stdio::piped()).spawn().unwrap();
        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        nonblocking(stdout.as_ref().unwrap().as_raw_fd()).unwrap();
        nonblocking(stderr.as_ref().unwrap().as_raw_fd()).unwrap();
        stop_observing(Operation::Update, child, stdout, stderr);
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if marker.exists() && unsafe { libc::kill(pid as i32, 0) } == -1
                && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                assert_eq!(std::fs::read(&marker).unwrap(), b"complete");
                std::fs::remove_file(marker).unwrap();
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("transition must finish and be reaped after its UI observer stops");
    }
    #[test]
    fn interrupted_shutdown_explains_restart_without_offering_adoption() {
        let mut screen = Bootstrap::new();
        assert!(!screen.apply_completion(Err("Error: \"package_restart_required\"\n".into())));
        assert_eq!(screen.primary_action(), (Operation::Status, "Check again"));
        assert!(!screen.package_adopt_offered && !screen.stop_offered && !screen.adopted);
        assert!(screen.result.as_deref().unwrap().contains("Restart this computer"));
        assert!(screen.result.as_deref().unwrap().contains("retained"));
    }
    #[test]
    fn first_run_commands_have_only_fixed_package_owner_and_closed_verb() {
        for (operation, verb) in [
            (Operation::Adopt, "package-adopt"),
            (Operation::Activate, "package-activate"),
            (Operation::Update, "package-update"),
            (Operation::Restore, "package-restore"),
            (Operation::Recover, "package-recover"),
            (Operation::StopForRepair, "package-stop-for-repair"),
            (Operation::Status, "package-bootstrap-status"),
            (Operation::SelectedStatus, "package-activation-status"),
        ] {
            let command = fixed_command(operation).unwrap();
            assert_eq!(command.get_program(), SYSTEM_MANAGER);
            assert_eq!(command.get_args().collect::<Vec<_>>(), vec![verb]);
        }
    }
    #[test]
    fn system_entry_checks_exact_manager_posture_before_offering_a_button() {
        assert_eq!(
            select_entry(Path::new(SYSTEM_FRONTEND)),
            Entry::Checking
        );
        assert_eq!(
            select_entry(Path::new("/tmp/development-frontend")),
            Entry::Ordinary
        );
        let checking = Bootstrap::checking();
        assert!(checking.check_queued);
        assert!(checking.running.is_none());
    }
    #[test]
    fn activation_readback_distinguishes_exact_active_inactive_and_invalid() {
        for state in ["active", "inactive", "fresh_adoptable", "legacy_adoptable",
            "legacy_active", "repair_active", "repair_inactive",
            "update_active", "update_adoptable", "update_retirement_pending",
            "rollback_active", "rollback_inactive", "rollback_retirement_pending",
            "legacy_retirement_pending", "repair_retirement_pending"] {
            let data = serde_json::json!({"schema":3,"state":state,"package_version":"0.1.0"});
            assert_eq!(
                parse_activation_status(data.to_string().as_bytes()).unwrap(),
                state
            );
        }
        for value in [
            serde_json::json!({"schema":2,"state":"active","package_version":"0.1.0"}),
            serde_json::json!({"schema":3,"state":"unknown","package_version":"0.1.0"}),
            serde_json::json!({"schema":3,"state":"active","package_version":""}),
            serde_json::json!({"schema":3,"state":"active","package_version":"0.1.0","extra":true}),
        ] {
            assert!(parse_activation_status(value.to_string().as_bytes()).is_err());
        }
        for state in ["active", "inactive"] {
            let data = serde_json::json!({"schema":1,"state":state,"package_version":"0.1.0"});
            assert_eq!(parse_selected_activation_status(data.to_string().as_bytes()).unwrap(),
                state);
        }
        for value in [
            serde_json::json!({"schema":3,"state":"active","package_version":"0.1.0"}),
            serde_json::json!({"schema":1,"state":"rollback_active","package_version":"0.1.0"}),
            serde_json::json!({"schema":1,"state":"active","package_version":"0.1.0","extra":true}),
        ] {
            assert!(parse_selected_activation_status(value.to_string().as_bytes()).is_err());
        }
    }
    #[test]
    fn setup_verifies_current_status_before_handoff() {
        for operation in [Operation::Adopt, Operation::Activate, Operation::Update, Operation::Restore] {
            assert_eq!(confirmation_readback(operation), Operation::SelectedStatus);
        }
        for operation in [Operation::Status, Operation::StopForRepair, Operation::Recover] {
            assert_eq!(confirmation_readback(operation), Operation::Status);
        }
        assert_eq!(
            completed_status(Operation::Adopt, "inactive").unwrap(),
            Completion::NeedsActivation
        );
        assert_eq!(
            completed_status(Operation::Recover, "inactive").unwrap(),
            Completion::NeedsActivation
        );
        assert_eq!(
            completed_status(Operation::Activate, "active").unwrap(),
            Completion::Selected
        );
        for operation in [Operation::Update, Operation::Restore] {
            assert_eq!(completed_status(operation, "active").unwrap(), Completion::Selected);
            for state in ["inactive", "update_adoptable", "rollback_inactive", "unknown"] {
                assert!(completed_status(operation, state).is_err(), "incomplete change cannot open Library");
            }
        }
        assert!(completed_status(Operation::Activate, "inactive").is_err());
        assert!(completed_status(Operation::Adopt, "unknown").is_err());
    }
    #[test]
    fn package_result_stream_is_bounded() {
        let mut small = Some(std::io::Cursor::new(b"ok".to_vec()));
        let mut bytes = Vec::new();
        collect_pipe(&mut small, &mut bytes, 2).unwrap();
        assert!(small.is_none());
        assert_eq!(bytes, b"ok");
        let mut long = Some(std::io::Cursor::new(b"too long".to_vec()));
        assert!(collect_pipe(&mut long, &mut Vec::new(), 2).is_err());
    }
    #[test]
    fn only_manager_owned_postures_offer_normal_setup_buttons() {
        for raw in [
            "package_transition_needs_recovery",
            "Error: \"package_transition_needs_recovery\"\n",
        ] {
            let screen = Bootstrap::attention(raw.into());
            assert!(screen.recovery_offered);
            assert!(!screen.package_adopt_offered);
        }
        for (state, action, label) in [
            ("fresh_adoptable", Operation::Adopt, "Set up application"),
            ("legacy_adoptable", Operation::Update, "Update Bridge"),
            ("legacy_active", Operation::Update, "Update Bridge"),
            ("repair_active", Operation::StopForRepair, "Stop bridge service for setup"),
            ("update_active", Operation::Update, "Update Bridge"),
            ("update_retirement_pending", Operation::Update, "Update Bridge"),
            ("update_adoptable", Operation::Update, "Update Bridge"),
            ("rollback_active", Operation::Restore, "Restore previous setup"),
            ("rollback_retirement_pending", Operation::Restore, "Restore previous setup"),
            ("rollback_inactive", Operation::Restore, "Restore previous setup"),
            ("legacy_retirement_pending", Operation::Update, "Update Bridge"),
            ("repair_retirement_pending", Operation::StopForRepair, "Finish bridge shutdown"),
            ("repair_inactive", Operation::Adopt, "Apply installed package"),
            ("inactive", Operation::Activate, "Start bridge service"),
        ] {
            let mut screen = Bootstrap::checking();
            assert!(!screen.apply_completion(completed_status(Operation::Status, state)));
            assert_eq!(screen.primary_action(), (action, label));
        }
        for (initial, operation) in [("update_active", Operation::Update),
            ("rollback_active", Operation::Restore)] {
            let mut screen = Bootstrap::checking();
            assert!(!screen.apply_completion(completed_status(Operation::Status, initial)));
            assert_eq!(screen.primary_action().0, operation);
            assert!(screen.apply_completion(completed_status(operation, "active")),
                "one completed operation hands off without a manual activation step");
            assert!(!screen.apply_completion(completed_status(operation, "inactive")));
            assert!(screen.attention && !screen.adopted);
            assert_eq!(screen.primary_action(), (Operation::Status, "Check again"));
        }
        for raw in [
            "prefix_package_routes_need_repair",
            "package_stop_service_first",
            "Error: \"package_transition_needs_recovery\": stale",
            "package_routes_need_repair",
            "Error: \"package_service_effective_route_mismatch\"\n",
        ] {
            let screen = Bootstrap::attention(raw.into());
            assert!(!screen.package_adopt_offered && !screen.recovery_offered);
            assert_eq!(screen.primary_action(), (Operation::Status, "Check again"));
        }
    }
    #[test]
    fn continued_update_starts_only_the_offered_update_once() {
        for (state, expected) in [("update_active", Some(Operation::Update)),
            ("update_adoptable", Some(Operation::Update)),
            ("update_retirement_pending", Some(Operation::Update)),
            ("rollback_active", None), ("repair_active", None), ("inactive", None)] {
            let mut screen = Bootstrap::continuing_update();
            assert!(screen.check_queued);
            assert!(!screen.apply_completion(completed_status(Operation::Status, state)));
            assert_eq!(screen.continue_after_status(), expected, "{state}");
            assert_eq!(screen.continue_after_status(), None, "never a second time");
        }
        let mut recovery = Bootstrap::continuing_update();
        assert!(!recovery.apply_completion(Err("package_transition_needs_recovery".into())));
        assert_eq!(recovery.continue_after_status(), None);
        let mut ordinary = Bootstrap::checking();
        assert!(!ordinary.apply_completion(completed_status(Operation::Status, "update_active")));
        assert_eq!(ordinary.continue_after_status(), None, "plain Setup still waits for a click");
    }
    #[test]
    fn gate_refusals_are_shown_in_plain_words() {
        for raw in ["Error: \"package_operator_active_or_ambiguous\"\n", "package_operator_active_or_ambiguous"] {
            let mut screen = Bootstrap::checking();
            assert!(!screen.apply_completion(Err(raw.into())));
            let shown = screen.result.as_deref().unwrap();
            assert!(shown.starts_with("A bridge task is still running") && !shown.contains('_'), "{shown}");
            assert_eq!(screen.primary_action(), (Operation::Status, "Check again"));
        }
        for code in ["package_vendor_owner_active", "package_installer_owner_active",
            "package_owner_active", "package_service_not_clean_idle"] {
            let shown = plain_refusal(&format!("Error: {code:?}\n"));
            assert!(!shown.contains('_') && shown.ends_with('.'), "{shown}");
        }
        // Notes before the final line (interrupted work not finished) are not
        // the refusal; the manager's own sentence is shown without its wrapper.
        let manager = "An interrupted bridge task was not completely finished: ab: busy\n\
            Error: \"The app's support service is still running. Restart the computer, then choose Check again.\"\n";
        let sentence = "The app's support service is still running. Restart the computer, then choose Check again.";
        assert_eq!(plain_refusal(manager), sentence);
        assert_eq!(Bootstrap::attention(manager.into()).result.as_deref(), Some(sentence));
        let noted = "An interrupted bridge task was not completely finished: ab: busy\nError: \"package_transition_needs_recovery\"\n";
        assert!(Bootstrap::attention(noted.into()).recovery_offered);
        // Unknown codes stay exact for Setup details.
        for raw in ["Error: \"package_owners_changed\"\n", "package_update_required: use Update Bridge"] {
            assert_eq!(plain_refusal(raw), raw);
        }
    }
}

//! Explicit first-run handoff for the fixed system package frontend.
//! PKG0 owns every adoption, refusal, route and rollback decision.
use eframe::egui;
use std::{
    io::Read,
    os::fd::AsRawFd,
    os::unix::fs::{FileExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

const SYSTEM_FRONTEND: &str = "/usr/bin/linux-audio-compatibility-manager";
const SYSTEM_MANAGER: &str = "/usr/bin/linux-vst-bridge";

#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    Checking,
    Ordinary,
}

fn selected_frontend() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
    Ok(PathBuf::from(home).join(".local/bin/linux-audio-compatibility-manager"))
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
    if linux_vst_bridge::portable_package::input_root(&executable, &home)
        .map_err(|e| e.to_string())?.is_some() {
        return Ok(Entry::Checking);
    }
    if executable != Path::new(SYSTEM_FRONTEND) {
        return Ok(Entry::Ordinary);
    }
    Ok(select_entry(&executable))
}

pub fn launch_selected() -> Result<(), String> {
    let frontend = selected_frontend()?;
    let target = std::fs::canonicalize(&frontend)
        .map_err(|_| "Selected application route is unavailable".to_owned())?;
    if target == Path::new(SYSTEM_FRONTEND)
        || std::env::current_exe().is_ok_and(|executable|executable==target) {
        return Err("Selected application route points back to the package launcher".into());
    }
    Command::new(frontend)
        .spawn()
        .map(|_| ())
        .map_err(|_| "Selected application could not be opened".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Adopt,
    Activate,
    Update,
    Restore,
    Recover,
    StopForRepair,
    Status,
    SelectedStatus,
}

impl Operation {
    fn owns_transition(self) -> bool {
        matches!(self, Self::Update | Self::Restore | Self::Recover)
    }
}

fn fixed_command(operation: Operation) -> Result<Command, String> {
    let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("Home directory unavailable")?);
    let manager = linux_vst_bridge::portable_package::input_root(&executable,&home)
        .map_err(|e|e.to_string())?.map(|root|root.join("bin/linux-vst-bridge"))
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
    std::thread::spawn(move || {
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
    });
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
        + Duration::from_secs(if matches!(operation, Operation::Status | Operation::SelectedStatus) {
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
                "Package status readback timed out; no setup action was submitted".into()
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Completion {
    Selected,
    NeedsActivation,
    FreshAdoptable,
    LegacyAdoptable,
    LegacyActive,
    UpdateActive,
    UpdateAdoptable,
    UpdateRetirementPending,
    RollbackActive,
    RollbackInactive,
    RollbackRetirementPending,
    RepairActive,
    RepairInactive,
    RetirementPending,
    LegacyRetirementPending,
}

fn confirmation_readback(operation: Operation) -> Operation {
    if matches!(operation, Operation::Adopt | Operation::Activate | Operation::Update | Operation::Restore) {
        Operation::SelectedStatus
    } else {
        Operation::Status
    }
}

fn run(operation: Operation) -> Result<Completion, String> {
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

fn exact_refusal(error: &str, code: &str) -> bool {
    error.trim() == code || error.trim() == format!("Error: {code:?}")
}

pub struct Bootstrap {
    receiver: Option<Receiver<Result<Completion, String>>>,
    running: Option<Operation>,
    result: Option<String>,
    recovery_offered: bool,
    package_adopt_offered: bool,
    stop_offered: bool,
    retirement_pending: bool,
    legacy_update: bool,
    update_available: bool,
    rollback_available: bool,
    adopted: bool,
    attention: bool,
    check_queued: bool,
}

impl Bootstrap {
    pub fn new() -> Self {
        Self {
            receiver: None,
            running: None,
            result: None,
            recovery_offered: false,
            package_adopt_offered: false,
            stop_offered: false,
            retirement_pending: false,
            legacy_update: false,
            update_available: false,
            rollback_available: false,
            adopted: false,
            attention: false,
            check_queued: false,
        }
    }

    pub fn checking() -> Self {
        Self {
            check_queued: true,
            ..Self::new()
        }
    }

    /// Source preview of the second first-run step.
    #[allow(dead_code)]
    pub fn needs_activation() -> Self {
        Self {
            adopted: true,
            ..Self::new()
        }
    }

    #[allow(dead_code)]
    pub fn legacy_adoptable_preview() -> Self {
        let mut screen = Self::new();
        screen.apply_completion(Ok(Completion::LegacyAdoptable));
        screen
    }

    #[allow(dead_code)]
    pub fn repair_active_preview() -> Self {
        let mut screen = Self::new();
        screen.apply_completion(Ok(Completion::RepairActive));
        screen
    }

    pub fn attention(error: String) -> Self {
        Self {
            receiver: None,
            running: None,
            recovery_offered: exact_refusal(&error, "package_transition_needs_recovery"),
            package_adopt_offered: false,
            stop_offered: false,
            retirement_pending: false,
            legacy_update: false,
            update_available: false,
            rollback_available: false,
            result: Some(error),
            adopted: false,
            attention: true,
            check_queued: false,
        }
    }

    fn submit(&mut self, operation: Operation, ctx: egui::Context) {
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.running = Some(operation);
        self.result = None;
        std::thread::spawn(move || {
            let result = run(operation);
            let _ = sender.send(result);
            ctx.request_repaint();
        });
    }

    fn apply_completion(&mut self, result: Result<Completion, String>) -> bool {
        self.recovery_offered = false;
        self.package_adopt_offered = false;
        self.stop_offered = false;
        self.retirement_pending = false;
        self.legacy_update = false;
        self.update_available = false;
        self.rollback_available = false;
        self.adopted = false;
        self.attention = false;
        self.result = None;
        match result {
            Ok(Completion::Selected) => return true,
            Ok(Completion::NeedsActivation) => self.adopted = true,
            Ok(Completion::FreshAdoptable) => {},
            Ok(Completion::LegacyAdoptable) => {
                self.package_adopt_offered = true;
                self.legacy_update = true;
            }
            Ok(Completion::LegacyActive) => {
                self.stop_offered = true;
                self.legacy_update = true;
            }
            Ok(Completion::RepairActive) => self.stop_offered = true,
            Ok(Completion::LegacyRetirementPending) => {
                self.stop_offered = true;
                self.retirement_pending = true;
                self.legacy_update = true;
            }
            Ok(Completion::UpdateActive) => {
                self.stop_offered = true;
                self.update_available = true;
            }
            Ok(Completion::UpdateAdoptable) => {
                self.package_adopt_offered = true;
                self.update_available = true;
            }
            Ok(Completion::UpdateRetirementPending) => {
                self.stop_offered = true;
                self.retirement_pending = true;
                self.update_available = true;
            }
            Ok(Completion::RollbackActive) => {
                self.stop_offered = true;
                self.rollback_available = true;
            }
            Ok(Completion::RollbackInactive) => {
                self.rollback_available = true;
            }
            Ok(Completion::RollbackRetirementPending) => {
                self.stop_offered = true;
                self.retirement_pending = true;
                self.rollback_available = true;
            }
            Ok(Completion::RetirementPending) => {
                self.stop_offered = true;
                self.retirement_pending = true;
            }
            Ok(Completion::RepairInactive) => self.package_adopt_offered = true,
            Err(error) => {
                self.recovery_offered = exact_refusal(&error, "package_transition_needs_recovery");
                self.attention = true;
                self.result = Some(if exact_refusal(&error, "package_restart_required") {
                    "The bridge service is stopped, but an interrupted session could not confirm its shutdown. Restart this computer, then reopen Setup to continue. Your projects, installed plug-ins and current application version are retained.".into()
                } else { error });
            }
        }
        false
    }

    fn primary_action(&self) -> (Operation, &'static str) {
        if self.recovery_offered { (Operation::Recover, "Recover interrupted setup") }
        else if self.update_available || self.legacy_update { (Operation::Update, "Update Bridge") }
        else if self.rollback_available { (Operation::Restore, "Restore previous setup") }
        else if self.stop_offered { (Operation::StopForRepair,
            if self.retirement_pending { "Finish bridge shutdown" }
            else { "Stop bridge service for setup" }) }
        else if self.package_adopt_offered { (Operation::Adopt, "Apply installed package") }
        else if self.attention { (Operation::Status, "Check again") }
        else if self.adopted { (Operation::Activate, "Start bridge service") }
        else { (Operation::Adopt, "Set up application") }
    }

}

impl eframe::App for Bootstrap {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.check_queued {
            self.check_queued = false;
            self.submit(Operation::Status, ui.ctx().clone());
        }
        if let Some(result) = self
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok())
        {
            self.receiver = None;
            self.running = None;
            if self.apply_completion(result) {
                match launch_selected() {
                    Ok(()) => {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                    Err(error) => {
                        self.attention = true;
                        self.result = Some(error);
                    }
                }
            }
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(if self.update_available || self.legacy_update {
                "Update Linux VST Bridge"
            } else if self.rollback_available {
                "Restore previous setup"
            } else if self.stop_offered || (self.package_adopt_offered && !self.legacy_update) {
                "Repair Linux VST Bridge"
            } else { "Set up Linux VST Bridge" });
            ui.label(if self.update_available || self.legacy_update {
                "The manager prepares compatible bridge components for your installed plug-ins, then applies the update. Your plug-ins, licenses, projects and settings are retained."
            } else if self.rollback_available {
                "Restore the retained application and its matching plug-in bridges together. Your vendor installations, projects and settings are retained."
            } else if self.stop_offered || self.package_adopt_offered {
                "An existing managed installation is selected. The manager will keep its plug-ins and rollback history while checking application routes."
            } else {
                "The application package is installed. Set up your private, managed copy before opening your Library."
            });
            ui.label("The manager checks existing managed plug-ins and installation records before changing application files. Vendor authorization remains yours.");
            if let Some(operation) = self.running {
                ui.strong(match operation {
                    Operation::Adopt => "Setting up the application…",
                    Operation::Activate => "Starting the bridge service…",
                    Operation::Update => "Preparing and applying the bridge update…",
                    Operation::Restore => "Restoring the previous setup…",
                    Operation::Recover => "Finishing interrupted setup…",
                    Operation::StopForRepair => "Stopping the idle bridge service…",
                    Operation::Status => "Checking package status…",
                    Operation::SelectedStatus => "Checking selected application…",
                });
                ui.small(if operation.owns_transition() {
                    "The manager handles preparation and recovery. Closing this window leaves the operation running; reopen setup to check its result."
                } else { "Keep this window open while setup completes." });
            } else {
                if self.recovery_offered { ui.label("An earlier setup stopped before it finished. Finish that saved change before opening your Library."); }
                else if self.update_available || self.legacy_update {
                    ui.label("Close your DAW before updating. Preparation, verification and service restart are handled together. If preparation fails, the current setup stays selected.");
                }
                else if self.rollback_available {
                    ui.label("Close your DAW before restoring. The manager verifies the retained components and restarts the restored setup.");
                }
                else if self.stop_offered && self.retirement_pending { ui.label("The bridge service stopped, but its cleanup has not been confirmed. Finish shutdown before continuing."); }
                else if self.stop_offered { ui.label("Close your DAW first. The manager will confirm that no plug-ins or setup tasks are active before repairing the application."); }
                else if self.package_adopt_offered { ui.label("Application routes need attention. Applying the installed package rechecks the exact generation and refuses a route it does not own."); }
                else if self.attention { ui.label("Setup needs attention. Check again to confirm the current application and next step."); }
                else if self.adopted { ui.label("Application files and routes are selected. Start the bridge service to finish first-run setup."); }
                let (operation, label) = self.primary_action();
                if ui.add_sized([250.0, 48.0], egui::Button::new(label)).clicked() {
                    self.submit(operation, ui.ctx().clone());
                }
                if (self.update_available || self.legacy_update || self.rollback_available)
                    && !self.retirement_pending && ui.button("Keep current setup").clicked() {
                    self.submit(Operation::Activate, ui.ctx().clone());
                }
            }
            if let Some(result) = &self.result {
                if self.recovery_offered || self.package_adopt_offered {
                    egui::CollapsingHeader::new("Technical reason").show(ui, |ui| {
                        ui.colored_label(egui::Color32::YELLOW, result);
                    });
                } else {
                    ui.colored_label(egui::Color32::YELLOW, result);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

//! Explicit first-run handoff for the fixed system package frontend.
//! PKG0 owns every adoption, refusal, route and rollback decision.
use eframe::egui;
use std::{
    io::Read,
    os::fd::AsRawFd,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

const SYSTEM_FRONTEND: &str = "/usr/bin/linux-audio-compatibility-manager";
const SYSTEM_MANAGER: &str = "/usr/bin/linux-vst-bridge";

#[derive(Debug, PartialEq, Eq)]
pub enum Entry {
    Selected,
    FirstRun,
    NeedsActivation,
    Attention(String),
    Ordinary,
}

fn selected_frontend() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
    Ok(PathBuf::from(home).join(".local/bin/linux-audio-compatibility-manager"))
}

fn selected_manager() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("Home directory unavailable")?;
    Ok(PathBuf::from(home).join(".local/bin/linux-vst-bridge"))
}

fn select_entry(executable: &Path, route_present: bool) -> Entry {
    if executable != Path::new(SYSTEM_FRONTEND) {
        Entry::Ordinary
    } else if route_present {
        Entry::Selected
    } else {
        Entry::FirstRun
    }
}

pub fn entry() -> Result<Entry, String> {
    let executable = std::env::current_exe().map_err(|_| "Frontend identity unavailable")?;
    if executable != Path::new(SYSTEM_FRONTEND) {
        return Ok(Entry::Ordinary);
    }
    match std::fs::symlink_metadata(selected_manager()?) {
        Ok(_) => match activation_status() {
            Ok("active") => Ok(select_entry(&executable, true)),
            Ok("inactive") => Ok(Entry::NeedsActivation),
            Ok(_) => Ok(Entry::Attention(
                "Package activation status is invalid".into(),
            )),
            Err(error) => Ok(Entry::Attention(error)),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(select_entry(&executable, false))
        }
        Err(_) => Err("Selected manager route could not be inspected".into()),
    }
}

pub fn launch_selected() -> Result<(), String> {
    let frontend = selected_frontend()?;
    let target = std::fs::canonicalize(&frontend)
        .map_err(|_| "Selected application route is unavailable".to_owned())?;
    if target == Path::new(SYSTEM_FRONTEND) {
        return Err("Selected application route points back to the package launcher".into());
    }
    Command::new(frontend)
        .spawn()
        .map(|_| ())
        .map_err(|_| "Selected application could not be opened".into())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    Adopt,
    Activate,
    Recover,
    Status,
}

fn fixed_command(operation: Operation) -> Command {
    let mut command = Command::new(SYSTEM_MANAGER);
    command.arg(match operation {
        Operation::Adopt => "package-adopt",
        Operation::Activate => "package-activate",
        Operation::Recover => "package-recover",
        Operation::Status => "package-activation-status",
    });
    command
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

fn execute(operation: Operation) -> Result<Vec<u8>, String> {
    let mut child = fixed_command(operation)
        .stdin(Stdio::null())
        .stdout(if operation == Operation::Status {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::piped())
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
            let _ = child.kill();
            let _ = child.wait();
            return Err(reason);
        }
    }
    let mut output = Vec::new();
    let mut error = Vec::new();
    let deadline = Instant::now()
        + Duration::from_secs(if operation == Operation::Status {
            10
        } else {
            300
        });
    let mut exited_at = None;
    let status = loop {
        let readback = collect_pipe(&mut stdout, &mut output, 4096)
            .and_then(|_| collect_pipe(&mut stderr, &mut error, 4096));
        if let Err(reason) = readback {
            let _ = child.kill();
            let _ = child.wait();
            return Err(reason);
        }
        if let Some(status) = child.try_wait().map_err(|_| "Package wait failed")? {
            if stdout.is_none() && stderr.is_none() {
                break status;
            }
            let since = exited_at.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(1) {
                return Err("Package result stream remained open after the manager exited".into());
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(if operation == Operation::Status {
                "Package status readback timed out; no setup action was submitted".into()
            } else {
                "Package setup timed out. Reopen the application to inspect or recover its retained transition; no second request was submitted.".into()
            });
        }
        std::thread::sleep(Duration::from_millis(20));
    };
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
}

fn run(operation: Operation) -> Result<Completion, String> {
    if operation != Operation::Status {
        execute(operation)?;
    }
    completed_status(operation, activation_status()?)
}

fn completed_status(operation: Operation, state: &str) -> Result<Completion, String> {
    match state {
        "active" => Ok(Completion::Selected),
        "inactive" if operation != Operation::Activate => Ok(Completion::NeedsActivation),
        "inactive" => Err("The bridge service did not become active after setup".into()),
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

fn parse_activation_status(data: &[u8]) -> Result<&'static str, String> {
    if data.len() > 4096 {
        return Err("Package activation status exceeded its bound".into());
    }
    let result: ActivationStatus = serde_json::from_slice(data)
        .map_err(|_| "Package activation status is malformed".to_owned())?;
    if result.schema != 1 || result.package_version.is_empty() || result.package_version.len() > 80
    {
        return Err("Package activation status is incompatible".into());
    }
    match result.state.as_str() {
        "active" => Ok("active"),
        "inactive" => Ok("inactive"),
        _ => Err("Package activation status is unknown".into()),
    }
}

pub struct Bootstrap {
    receiver: Option<Receiver<Result<Completion, String>>>,
    running: Option<Operation>,
    result: Option<String>,
    recovery_offered: bool,
    adopted: bool,
    attention: bool,
}

impl Bootstrap {
    pub fn new() -> Self {
        Self {
            receiver: None,
            running: None,
            result: None,
            recovery_offered: false,
            adopted: false,
            attention: false,
        }
    }

    pub fn needs_activation() -> Self {
        Self {
            adopted: true,
            ..Self::new()
        }
    }

    pub fn attention(error: String) -> Self {
        Self {
            receiver: None,
            running: None,
            recovery_offered: error.contains("package_transition_needs_recovery"),
            result: Some(error),
            adopted: false,
            attention: true,
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
}

impl eframe::App for Bootstrap {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(result) = self
            .receiver
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok())
        {
            self.receiver = None;
            self.running = None;
            match result {
                Ok(Completion::Selected) => match launch_selected() {
                    Ok(()) => {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        return;
                    }
                    Err(error) => self.result = Some(error),
                },
                Ok(Completion::NeedsActivation) => {
                    self.adopted = true;
                    self.recovery_offered = false;
                    self.attention = false;
                    self.result = None;
                }
                Err(error) => {
                    self.recovery_offered = error.contains("package_transition_needs_recovery");
                    if self.recovery_offered {
                        self.attention = true;
                    }
                    self.result = Some(error);
                }
            }
        }
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Set up Linux VST Bridge");
            ui.label("The application package is installed. Set up your private, managed copy before opening your Library.");
            ui.label("Your existing plug-ins, installations and authorizations remain in your account. The manager checks whether setup is safe before changing anything.");
            if let Some(operation) = self.running {
                ui.strong(match operation {
                    Operation::Adopt => "Setting up the application…",
                    Operation::Activate => "Starting the bridge service…",
                    Operation::Recover => "Finishing interrupted setup…",
                    Operation::Status => "Checking package status…",
                });
                ui.small("Keep this window open while the package transition completes.");
            } else if self.recovery_offered {
                if ui.add_sized([250.0, 48.0], egui::Button::new("Finish interrupted setup")).clicked() {
                    self.submit(Operation::Recover, ui.ctx().clone());
                }
            } else if self.attention {
                ui.label("The selected application needs attention. Its state has not been changed by this screen.");
                if ui.add_sized([250.0, 48.0], egui::Button::new("Check again")).clicked() {
                    match Command::new(SYSTEM_FRONTEND).spawn() {
                        Ok(_) => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(_) => self.result = Some("Package launcher could not be reopened".into()),
                    }
                }
            } else if self.adopted {
                ui.label("Application files and routes are selected. Start the bridge service to finish first-run setup.");
                if ui.add_sized([250.0, 48.0], egui::Button::new("Start bridge service")).clicked() {
                    self.submit(Operation::Activate, ui.ctx().clone());
                }
            } else if ui.add_sized([250.0, 48.0], egui::Button::new("Set up application")).clicked() {
                self.submit(Operation::Adopt, ui.ctx().clone());
            }
            if let Some(result) = &self.result {
                ui.colored_label(egui::Color32::YELLOW, result);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_run_commands_have_only_fixed_package_owner_and_closed_verb() {
        for (operation, verb) in [
            (Operation::Adopt, "package-adopt"),
            (Operation::Activate, "package-activate"),
            (Operation::Recover, "package-recover"),
            (Operation::Status, "package-activation-status"),
        ] {
            let command = fixed_command(operation);
            assert_eq!(command.get_program(), SYSTEM_MANAGER);
            assert_eq!(command.get_args().collect::<Vec<_>>(), vec![verb]);
        }
    }
    #[test]
    fn system_entry_requires_explicit_adoption_only_without_user_route() {
        assert_eq!(
            select_entry(Path::new(SYSTEM_FRONTEND), false),
            Entry::FirstRun
        );
        assert_eq!(
            select_entry(Path::new(SYSTEM_FRONTEND), true),
            Entry::Selected
        );
        assert_eq!(
            select_entry(Path::new("/tmp/development-frontend"), false),
            Entry::Ordinary
        );
    }
    #[test]
    fn activation_readback_distinguishes_exact_active_inactive_and_invalid() {
        for state in ["active", "inactive"] {
            let data = serde_json::json!({"schema":1,"state":state,"package_version":"0.1.0"});
            assert_eq!(
                parse_activation_status(data.to_string().as_bytes()).unwrap(),
                state
            );
        }
        for value in [
            serde_json::json!({"schema":2,"state":"active","package_version":"0.1.0"}),
            serde_json::json!({"schema":1,"state":"unknown","package_version":"0.1.0"}),
            serde_json::json!({"schema":1,"state":"active","package_version":""}),
            serde_json::json!({"schema":1,"state":"active","package_version":"0.1.0","extra":true}),
        ] {
            assert!(parse_activation_status(value.to_string().as_bytes()).is_err());
        }
    }
    #[test]
    fn setup_verifies_current_status_before_handoff() {
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
}

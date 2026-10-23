//! Package setup presentation and exact owner-offered steps, shared by native frontends.
//! Only the installed adapter executes the fixed package commands.
#[cfg(target_os = "linux")]
mod installed;
#[cfg(target_os = "linux")]
pub use installed::{entry, launch_selected, run, run_and_open, Entry, Worker};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
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

/// The manager's final line; earlier lines are notes such as interrupted work
/// it could not completely finish.
fn final_line(error: &str) -> &str {
    error.lines().rev().map(str::trim).find(|line| !line.is_empty()).unwrap_or("")
}
fn exact_refusal(error: &str, code: &str) -> bool {
    let error = final_line(error);
    error == code || error == format!("Error: {code:?}")
}

/// The manager's refusal in words a musician can act on. A sentence the
/// manager already wrote loses only the process wrapper; a known gate code
/// gets a sentence; anything else stays exact for Setup details.
fn plain_refusal(error: &str) -> String {
    let last = final_line(error);
    let inner = last.strip_prefix("Error: \"").and_then(|rest| rest.strip_suffix('"'))
        .map(|text| text.replace("\\\"", "\"").replace("\\\\", "\\"))
        .unwrap_or_else(|| last.to_owned());
    // A restart does not settle what these still find, so none promises it.
    match inner.as_str() {
        "package_operator_active_or_ambiguous" => "A bridge task is still running or did not record how it ended. Let it finish, then choose Check again; if this stays, send Setup details.".into(),
        "package_vendor_owner_active" => "An app session opened through the bridge has not finished closing. Close the app, then choose Check again; if this stays, send Setup details.".into(),
        "package_installer_owner_active" => "An installer started through the bridge has not finished. Let it finish, then choose Check again; if this stays, send Setup details.".into(),
        "package_owner_active" | "package_service_not_clean_idle" | "active_device_lease" => "The bridge is still in use, usually by an open DAW. Close your DAW, then choose Check again; if this stays, send Setup details.".into(),
        text if text.ends_with('.') && text.contains(' ')
            && text.starts_with(|c: char| c.is_ascii_uppercase()) => text.into(),
        _ => error.to_owned(),
    }
}

pub struct State {
    epoch: u64,
    queued: Option<Operation>,
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
    /// Start Update Bridge once the first status check offers it.
    continue_update: bool,
}

impl State {
    pub fn new() -> Self {
        Self {
            epoch: 0, queued: None,
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
            continue_update: false,
        }
    }

    pub fn checking() -> Self {
        Self {
            check_queued: true,
            ..Self::new()
        }
    }

    /// Opened by the in-app updater after it staged this package: check, and
    /// when the check offers Update Bridge, start it. Any other state is shown
    /// as usual, with its usual buttons.
    pub fn continuing_update() -> Self {
        Self {
            continue_update: true,
            ..Self::checking()
        }
    }

    /// After the first status check of a continued update.
    fn continue_after_status(&mut self) -> Option<Operation> {
        if !std::mem::take(&mut self.continue_update) { return None; }
        (self.primary_action().0 == Operation::Update).then_some(Operation::Update)
    }

    pub fn attention(error: String) -> Self {
        Self {
            epoch: 0, queued: None,
            running: None,
            recovery_offered: exact_refusal(&error, "package_transition_needs_recovery"),
            package_adopt_offered: false,
            stop_offered: false,
            retirement_pending: false,
            legacy_update: false,
            update_available: false,
            rollback_available: false,
            result: Some(plain_refusal(&error)),
            adopted: false,
            attention: true,
            check_queued: false,
            continue_update: false,
        }
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
                } else { plain_refusal(&error) });
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


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step { epoch: u64, operation: Operation }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control { pub label: &'static str, pub step: Step }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panel {
    pub title: &'static str,
    pub introduction: &'static str,
    pub advice: &'static str,
    pub progress: Option<&'static str>,
    pub detail: &'static str,
    pub controls: Vec<Control>,
    pub result: Option<String>,
}
impl Default for State { fn default() -> Self { Self::new() } }
impl State {
    /// A rendered step is valid only for the exact state that offered it.
    pub fn press(&mut self, step: Step) -> bool {
        if !self.panel().controls.iter().any(|control| control.step == step) { return false; }
        self.queued = Some(step.operation);
        self.epoch += 1;
        true
    }
    pub fn take_operation(&mut self) -> Option<Operation> {
        if self.running.is_some() { return None; }
        let operation = if std::mem::take(&mut self.check_queued) { Some(Operation::Status) }
            else { self.queued.take() }?;
        self.running = Some(operation);
        self.result = None;
        self.epoch += 1;
        Some(operation)
    }
    /// True means the owner confirmed selection; launch uses the verified route.
    pub fn receive(&mut self, result: Result<Completion, String>) -> bool {
        let finished = self.running.take();
        self.epoch += 1;
        if self.apply_completion(result) { return true; }
        if finished == Some(Operation::Status) { self.queued = self.continue_after_status(); }
        false
    }
    pub fn open_failed(&mut self, error: String) {
        self.attention = true;
        self.result = Some(error);
        self.epoch += 1;
    }
    pub fn panel(&self) -> Panel {
        let updating = self.update_available || self.legacy_update;
        let operation = self.running.or(self.queued).or(self.check_queued.then_some(Operation::Status));
        let progress = operation.map(|operation| match operation {
            Operation::Adopt => "Setting up the application…",
            Operation::Activate => "Starting the bridge service…",
            Operation::Update => "Preparing and applying the bridge update…",
            Operation::Restore => "Restoring the previous setup…",
            Operation::Recover => "Finishing interrupted setup…",
            Operation::StopForRepair => "Stopping the idle bridge service…",
            Operation::Status => "Checking package status…",
            Operation::SelectedStatus => "Checking selected application…",
        });
        let mut controls = Vec::new();
        if operation.is_none() {
            let (operation, label) = self.primary_action();
            controls.push(Control { label, step: Step { epoch: self.epoch, operation } });
            if (updating || self.rollback_available) && !self.retirement_pending {
                controls.push(Control { label: "Keep current setup", step: Step { epoch: self.epoch, operation: Operation::Activate } });
            }
        }
        Panel {
            title: if updating { "Update Linux VST Bridge" } else if self.rollback_available { "Restore previous setup" }
                else if self.stop_offered || (self.package_adopt_offered && !self.legacy_update) { "Repair Linux VST Bridge" }
                else { "Set up Linux VST Bridge" },
            introduction: if updating { "The manager updates compatible bridge components, then applies the update. Your plug-ins, licenses, projects and settings are retained. Plug-ins needing a fresh compatibility check remain unchanged." }
                else if self.rollback_available { "Restore the retained application and its matching plug-in bridges together. Your vendor installations, projects and settings are retained." }
                else if self.stop_offered || self.package_adopt_offered { "An existing managed installation is selected. The manager will keep its plug-ins and rollback history while checking application routes." }
                else { "The application package is installed. Set up your private, managed copy before opening your Library." },
            advice: if self.recovery_offered { "An earlier setup stopped before it finished. Finish that saved change before opening your Library." }
                else if updating { "Close your DAW before updating. Preparation, verification and service restart are handled together. If preparation fails, the current setup stays selected." }
                else if self.rollback_available { "Close your DAW before restoring. The manager verifies the retained components and restarts the restored setup." }
                else if self.stop_offered && self.retirement_pending { "The bridge service stopped, but its cleanup has not been confirmed. Finish shutdown before continuing." }
                else if self.stop_offered { "Close your DAW first. The manager will confirm that no plug-ins or setup tasks are active before repairing the application." }
                else if self.package_adopt_offered { "Application routes need attention. Applying the installed package rechecks the exact generation and refuses a route it does not own." }
                else if self.attention { "Setup needs attention. Check again to confirm the current application and next step." }
                else if self.adopted { "Application files and routes are selected. Start the bridge service to finish first-run setup." }
                else { "The manager checks existing managed plug-ins and installation records before changing application files. Vendor authorization remains yours." },
            detail: if operation.is_some_and(Operation::owns_transition) { "The manager handles preparation and recovery. Closing this window leaves the operation running; reopen setup to check its result." }
                else if operation.is_some() { "Keep this window open while setup completes." } else { "" },
            progress, controls, result: self.result.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_steps_are_once_only_and_do_not_borrow_new_owner_state() {
        let mut state = State::checking();
        assert!(state.panel().controls.is_empty());
        assert_eq!(state.take_operation(), Some(Operation::Status));
        assert!(!state.receive(Ok(Completion::UpdateActive)));
        let update = state.panel().controls[0].step;
        assert!(state.press(update));
        assert!(!state.press(update));
        assert_eq!(state.take_operation(), Some(Operation::Update));
        assert!(state.take_operation().is_none());
        assert!(!state.receive(Err("Owner refused the update".into())));
        assert!(!state.press(update));
        assert_eq!(state.panel().controls[0].label, "Check again");
        assert!(state.take_operation().is_none(), "a refusal cannot replay the update");
    }
    #[test]
    fn continued_update_requires_the_exact_offered_state_and_selected_readback() {
        for completion in [Completion::FreshAdoptable, Completion::NeedsActivation,
            Completion::RollbackActive, Completion::RepairActive, Completion::UpdateActive] {
            let mut state = State::continuing_update();
            assert_eq!(state.take_operation(), Some(Operation::Status));
            assert!(!state.receive(Ok(completion)));
            let next = state.take_operation();
            assert_eq!(next, (completion == Completion::UpdateActive).then_some(Operation::Update));
            assert!(state.take_operation().is_none());
            if next.is_some() { assert!(state.receive(Ok(Completion::Selected))); }
        }
    }
}

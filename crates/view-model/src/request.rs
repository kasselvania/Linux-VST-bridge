//! Retained request feedback shared by every native frontend.
//! The manager owns operation identity and outcomes. A window retains the exact
//! accepted operation and never turns an uncertain acknowledgment into a replay.
use crate::{journey, model::*};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportReport {
    pub id: String,
    pub sha256: String,
}
fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn runtime_verification_label(op: &serde_json::Value) -> Option<String> {
    let checked = op["runtime_verification"]["checked"].as_u64()?;
    let total = op["runtime_verification"]["total"].as_u64()?;
    (total > 0 && total <= 40000 && checked <= total).then(|| format!(
        "Checking compatibility files: {checked} of {total}. This can take longer after a restart."))
}

#[derive(Clone, Debug)]
pub struct RequestFeedback {
    pub action: Action,
    pub operation: Option<String>,
    pub text: String,
    pub terminal: bool,
    pub blocking: bool,
    pub release_after_snapshot: bool,
    pub acknowledgment_uncertain: bool,
    pub journey: Option<InstallationJourney>,
    pub transitions: Vec<String>,
    pub report: Option<SupportReport>,
    pub report_save_started: bool,
}
impl RequestFeedback {
    pub fn captured(action: Action) -> Self {
        Self {
            action,
            operation: None,
            text: crate::window::CAPTURED.into(),
            terminal: false,
            blocking: true,
            release_after_snapshot: false,
            acknowledgment_uncertain: false,
            journey: None,
            transitions: vec!["captured".into()],
            report: None,
            report_save_started: false,
        }
    }
    pub fn transition(&mut self, state: &str) {
        if self.transitions.last().is_none_or(|s| s != state) {
            if self.transitions.len() == 12 {
                self.transitions.remove(0);
            }
            self.transitions.push(state.into());
        }
    }
    pub fn receipt(&mut self, r: &Receipt) {
        self.acknowledgment_uncertain = r.accepted && r.operation.is_none();
        self.transition(if r.accepted { "accepted" } else { "refused" });
        self.operation = r.operation.clone();
        self.terminal = !r.accepted;
        self.release_after_snapshot = !r.accepted;
        self.text = if r.accepted {
            "Request accepted. Waiting for its result.".into()
        } else {
            format!(
                "This request was refused: {}",
                r.refusal.as_deref().unwrap_or("reason unavailable")
            )
        };
    }
    pub fn observe(&mut self, op: &serde_json::Value) {
        if self.operation.is_some() && self.operation.as_deref() == op["operation"].as_str()
            && (matches!(self.action,Action::SoftwareInstall { .. } | Action::SoftwareInstallWithPolicy {..} | Action::VendorSoftwareInstall { .. }
                | Action::SoftwareInstallResume { .. } | Action::RendererSoftwareInstall { .. })
                || matches!(&self.action,Action::EnvironmentRefreshResume {operation}
                    if op["renderer_owner"].as_str()==Some(operation.as_str()))) {
            if let Ok(journey)=serde_json::from_value::<InstallationJourney>(op["journey"].clone()) {
                if journey.phase == InstallationPhase::AwaitingApplication && journey.failure.is_none() {
                    self.journey=Some(journey);self.transition("awaiting_application");
                    self.terminal=true;self.blocking=false;self.release_after_snapshot=false;
                    self.text="The app is installed. Open it below to install your instruments and effects.".into();
                    return;
                }
            }
        }
        if self.terminal
            || self.operation.as_deref() != op["operation"].as_str()
            || self.operation.is_none()
        {
            return;
        }
        if let Ok(journey) = serde_json::from_value::<InstallationJourney>(op["journey"].clone()) {
            self.journey = Some(journey);
        }
        // A terminal executor receipt is not proof that the plug-in restarted.
        // Keep the exact operation and uncertainty through a subsequent refresh.
        if op["state"] == "completed" && op["outcome"] == "unconfirmed" {
            self.terminal = true;
            self.release_after_snapshot = true;
            self.transition("outcome_unconfirmed");
            self.text = if matches!(self.action, Action::PluginRestart { .. }) {
                "The restart result could not be confirmed. Check this instance in your DAW. The request will not be sent again.".into()
            } else {
                "The result could not be confirmed. Check its current status. The request will not be sent again.".into()
            };
            return;
        }
        if let Some(
            state @ ("refused"
            | "completed"
            | "waiting"
            | "validating"
            | "vendor_running"
            | "queued"
            | "running"
            | "submission_uncertain"),
        ) = op["state"].as_str()
        {
            self.transition(state);
        }
        let verification = runtime_verification_label(op);
        self.text = match op["state"].as_str() {
            Some("refused") => {
                self.terminal = true;
                self.release_after_snapshot = true;
                format!(
                    "This request was refused: {}",
                    op["reason"].as_str().unwrap_or("reason unavailable")
                )
            }
            Some("completed") => {
                self.terminal = true;
                self.release_after_snapshot = true;
                if matches!(self.action,Action::SupportExport {}) {
                    self.report = op["result"]["export"].as_str().zip(op["result"]["sha256"].as_str())
                        .filter(|(id,sha)|valid_hex(id,32)
                            && valid_hex(sha,64))
                        .map(|(id,sha256)|SupportReport {id:id.into(),sha256:sha256.into()});
                    if self.report.is_some() {
                        "Problem report created. Choose where to save the file to send.".into()
                    } else { "The report's file identity is unavailable. Create a new problem report.".into() }
                } else if let Some(message) = op["result"]["message"].as_str().filter(|_| matches!(self.action,
                    Action::PluginRemoveFromDaw { .. } | Action::PluginAddBackToDaw { .. }
                    | Action::SetupRemoveFromDaw { .. } | Action::ProductUninstall { .. }
                    | Action::InstallerCloseCheck { .. })) {
                    message.into()
                } else if self.journey.as_ref().is_some_and(|journey|
                    journey.phase == InstallationPhase::NeedsAttention) {
                    "Installation or DAW connection needs attention. Refreshing its results…".into()
                } else { "This operation completed. Refreshing its result…".into() }
            }
            Some("waiting" | "running") if verification.is_some() =>
                verification.unwrap_or_default(),
            Some("waiting" | "validating") => {
                "This operation is validating manager state. Do not click again.".into()
            }
            Some("submission_uncertain") => {
                self.release_after_snapshot = true;
                "Launch acknowledgment is uncertain. Use the exact Stop / reconcile control; no launch was resubmitted.".into()
            }
            Some("vendor_running") if matches!(self.action, Action::PluginFinishSetup { .. }) => {
                self.release_after_snapshot = true;
                "Finish setup in the plug-in window, then close it. The bridge will check it again automatically.".into()
            }
            Some("vendor_running") if matches!(self.action, Action::ProductUninstall { .. }) => {
                self.release_after_snapshot = true;
                "The vendor's uninstaller is running. Follow its window to finish.".into()
            }
            Some("vendor_running") => {
                if self.blocking {
                    self.release_after_snapshot = true;
                    "Vendor operation is running. Refreshing its controls…".into()
                } else {
                    "The operation is supervised and running. Use its exact Focus or Stop control below.".into()
                }
            }
            Some("queued") => "This operation is queued. Do not click again.".into(),
            Some("running") => match &self.action {
                Action::SoftwareInstall { .. } | Action::SoftwareInstallWithPolicy {..} | Action::VendorSoftwareInstall { .. }
                    | Action::SoftwareInstallResume { .. } | Action::RendererSoftwareInstall {..} | Action::EnvironmentRefresh { .. }
                    | Action::EnvironmentRefreshResume { .. } =>
                    self.journey.as_ref().map(journey::phase_label)
                        .unwrap_or("Installing and connecting your plug-ins").into(),
                Action::RuntimeInstall {} =>
                    "Downloading and preparing the compatibility runtime. This may take several minutes.".into(),
                Action::EnvironmentRescan { .. } =>
                    "Checking this setup's installed plug-ins.".into(),
                Action::EnvironmentRuntimeSelect { .. } =>
                    "Changing the setup's runtime and checking its installed plug-ins. Next, choose Prepare for your DAW for each plug-in.".into(),
                Action::CompatibilityCheck { .. } | Action::CompatibilityResumeCheck { .. } =>
                    "Checking this plug-in and preparing its matching bridge.".into(),
                Action::PluginPrepare { .. } => "Preparing this plug-in's matching bridge.".into(),
                Action::CandidateGraphicsAssess { .. } => "Assessing graphics in this plug-in's selected runtime.".into(),
                Action::CandidateGraphicsPrepare { .. } => "Preparing a reversible graphics settings trial.".into(),
                Action::CandidateSettingsPrepare { .. } => "Preparing a reversible compatibility settings trial. Your current selection stays in place.".into(),
                Action::CompatibilityPublishTest { .. } => "Selecting this plug-in's prepared bridge.".into(),
                Action::PluginRemoveFromDaw { .. } => "Removing this plug-in from your DAW.".into(),
                Action::PluginAddBackToDaw { .. } => "Adding this plug-in back to your DAW.".into(),
                Action::SetupRemoveFromDaw { .. } => "Removing this setup's plug-ins from your DAW.".into(),
                Action::SetupDelete { .. } => "Deleting this setup's Windows files. A large setup can take a minute.".into(),
                // Once the uninstaller closed, the setup's own progress.
                Action::ProductUninstall { .. } | Action::InstallerCloseCheck { .. }
                    if op["progress"].as_str().is_some() => op["progress"].as_str().unwrap_or_default().into(),
                Action::ProductUninstall { .. } =>
                    "Taking the affected plug-ins out of your DAW and starting the uninstaller.".into(),
                Action::InstallerCloseCheck { .. } => "Checking that it closed.".into(),
                _ => "This operation is in progress. Do not click again.".into(),
            },
            _ => return,
        };
    }
    pub fn refreshed(&mut self) {
        if self.release_after_snapshot {
            self.blocking = false;
            self.release_after_snapshot = false;
            if self.text == "This operation completed. Refreshing its result…" {
                self.text = "This operation completed. Its current result is shown below.".into();
            } else if self.text == "Installation or DAW connection needs attention. Refreshing its results…" {
                self.text = "Installation or DAW connection needs attention. Its results and continuation are shown below.".into();
            } else if self.text == "Vendor operation is running. Refreshing its controls…" {
                self.text = "The operation is supervised and running. Use its exact Focus or Stop control below.".into();
            }
        }
    }
    pub fn reconcile_snapshot(&mut self, s: &Snapshot) {
        // A lost acknowledgment is not a failed operation. Only exact canonical
        // onboarding ownership plus its offered recovery action can resolve it.
        if self.acknowledgment_uncertain && self.operation.is_none() {
            if let Action::InstallerStart { onboarding }
            | Action::InstallerStartWithPolicy { onboarding, .. } = &self.action
            {
                let owners: Vec<_> = s.onboarding.iter().filter(|r| r.environment.as_ref() == Some(onboarding))
                    .filter_map(|r| {
                        let op = r.details["installation"]["operation"].as_str()?;
                        let exact = r.actions.iter().any(|a| matches!(&a.action,
                            Action::InstallerStop { onboarding: id, operation } if id == onboarding && operation == op));
                        (exact && matches!(r.details["installation"]["state"].as_str().unwrap_or(r.state.as_str()), "starting" | "running" | "unknown")).then_some(op.to_owned())
                    }).collect();
                if owners.len() == 1 {
                    self.operation = Some(owners[0].clone());
                    self.transition("live_owner_reconciled");
                    self.release_after_snapshot = true;
                    self.text = "Submission acknowledgment was unavailable. The manager confirms this exact live installer; recovery controls are available. No request was resubmitted.".into();
                }
            }
        }
        let mut renderer_recovery = false;
        if let Action::RendererOpen {application,policy}
            | Action::RendererSoftwareInstall {application,policy,..}=self.action.clone()
        {
            for app in &s.vendor_applications {
                if app.details["application_identity"].as_str() == Some(application.as_str()) {
                    if self.acknowledgment_uncertain
                        && self.operation.is_none()
                        && app.details["requested"] == serde_json::to_value(policy).unwrap()
                        && matches!(
                            app.details["manager_operation"]["state"].as_str(),
                            Some("vendor_running" | "submission_uncertain")
                        )
                    {
                        if let Some(op) = app.details["operation"].as_str() {
                            if app.actions.iter().any(|a|matches!(&a.action,Action::RendererStop{operation} if operation==op)) {
                                self.operation=Some(op.into());self.release_after_snapshot=true;
                            }
                        }
                    }
                    if let Some(op) = app.details.get("manager_operation") {
                        // RendererOpen ends with its session; automatic installation
                        // continues into discovery/connection under the operator owner.
                        // A retired renderer child cannot complete that larger job.
                        if matches!(self.action,Action::RendererOpen { .. })
                            || matches!(op["state"].as_str(),Some("vendor_running"|"submission_uncertain")) {
                            renderer_recovery = self.operation.is_some()
                                && op["operation"].as_str() == self.operation.as_deref();
                            self.observe(op);
                        }
                    }
                }
            }
        }
        if matches!(self.action, Action::DependencyPrepare {}) {
            for app in &s.vendor_applications {
                if app.id != "native-access" {
                    continue;
                }
                let record = &app.details["dependency_manager_operation"];
                if let Some(op) = record["operation"].as_str() {
                    let exact = app.actions.iter().any(
                        |a| matches!(&a.action,Action::DependencyStop{operation} if operation==op),
                    );
                    if exact
                        && self.acknowledgment_uncertain
                        && self.operation.is_none()
                        && matches!(
                            record["state"].as_str(),
                            Some("submission_uncertain" | "vendor_running")
                        )
                    {
                        self.operation = Some(op.into());
                        self.release_after_snapshot = true;
                    }
                    if exact && self.operation.as_deref() == Some(op) {
                        renderer_recovery = true;
                        self.observe(record);
                    }
                }
            }
        }
        let matching_operation = self.operation.is_some()
            && s.operation
                .as_ref()
                .is_some_and(|op| op["operation"].as_str() == self.operation.as_deref());
        if let Some((environment,installer,_)) = self.action.environment_installer_start() {
            if let Some(target) = s.environment_installers.iter().find(|row|
                row.environment == environment && row.operation.as_deref() == self.operation.as_deref()) {
                self.observe(&target.result);
            } else if self.acknowledgment_uncertain && self.operation.is_none() {
                let owners:Vec<_> = s.environment_installers.iter().filter(|row|row.environment == environment
                    && row.installer.as_deref() == Some(installer))
                    .filter_map(|row|row.operation.as_ref().filter(|op|row.actions.iter().any(|offer|
                        matches!(&offer.action,Action::EnvironmentInstallerStop {operation} if operation == *op))))
                    .cloned().collect();
                if owners.len() == 1 {
                    // The exact start request in the live owner is checked by the
                    // manager. Never infer ownership from a display name.
                    self.operation = Some(owners[0].clone());
                    self.release_after_snapshot = true;
                }
            }
        }
        let exact_recovery = if let Action::InstallerStart { onboarding }
        | Action::InstallerStartWithPolicy { onboarding, .. } = &self.action
        {
            s.onboarding.iter().any(|r| {
                r.environment.as_ref() == Some(onboarding)
                    && r.actions.iter().any(|a| {
                        matches!(&a.action, Action::InstallerStop{onboarding:id,operation}
                    if id==onboarding && Some(operation.as_str())==self.operation.as_deref())
                    })
            })
        } else if let Some((environment,_,_)) = self.action.environment_installer_start() {
            s.environment_installers.iter().any(|row|row.environment == environment
                && row.actions.iter().any(|offer|matches!(&offer.action,
                    Action::EnvironmentInstallerStop {operation} if Some(operation.as_str()) == self.operation.as_deref())))
        } else {
            false
        };
        if exact_recovery && self.action.environment_installer_start().is_some() {
            self.release_after_snapshot = true;
            self.text = "The installer is running in this setup. Show or stop its window below.".into();
        }
        for product in &s.products {
            if let Some(op) = product.details["preparation"].get("operation") {
                self.observe(op);
            }
            if let Some(rows) = product.details["preparation"]["candidates"].as_array() {
                for row in rows {
                    if let Some(op) = row.get("operation") {
                        self.observe(op);
                    }
                }
            }
        }
        if let Some(op) = &s.operation {
            self.observe(op);
        }
        if self.terminal || matching_operation || exact_recovery || renderer_recovery {
            self.refreshed();
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    fn accepted() -> RequestFeedback {
        let mut feedback = RequestFeedback::captured(Action::SupportExport {});
        feedback.receipt(&Receipt { schema: OPERATOR_SCHEMA, accepted: true,
            operation: Some("exact-operation".into()), refusal: None });
        feedback
    }
    #[test]
    fn only_the_exact_accepted_operation_can_complete_a_report() {
        let mut feedback = accepted();
        let mut record = serde_json::json!({"operation":"other-operation","state":"completed",
            "result":{"export":"ab".repeat(16),"sha256":"cd".repeat(32)}});
        feedback.observe(&record);
        assert!(!feedback.terminal && feedback.report.is_none());
        record["operation"] = "exact-operation".into();
        feedback.observe(&record);
        assert!(feedback.terminal && feedback.blocking);
        let report = feedback.report.as_ref().unwrap();
        assert_eq!(report.id, "ab".repeat(16));
        assert_eq!(report.sha256, "cd".repeat(32));
        feedback.refreshed();
        assert!(!feedback.blocking);
    }
    #[test]
    fn unknown_ack_and_malformed_report_do_not_grant_a_file() {
        let mut feedback = RequestFeedback::captured(Action::SupportExport {});
        feedback.receipt(&Receipt { schema: OPERATOR_SCHEMA, accepted: true,
            operation: None, refusal: None });
        feedback.observe(&serde_json::json!({"operation":"unrelated","state":"completed",
            "result":{"export":"ab".repeat(16),"sha256":"cd".repeat(32)}}));
        assert!(feedback.acknowledgment_uncertain && feedback.report.is_none() && !feedback.terminal);
        let mut feedback = accepted();
        feedback.observe(&serde_json::json!({"operation":"exact-operation","state":"completed",
            "result":{"export":"../unowned","sha256":"cd".repeat(32)}}));
        assert!(feedback.terminal && feedback.report.is_none());
    }
    #[test]
    fn running_vendor_controls_resume_only_after_current_records_arrive() {
        let mut feedback = RequestFeedback::captured(Action::RuntimeInstall {});
        feedback.receipt(&Receipt { schema: OPERATOR_SCHEMA, accepted: true,
            operation: Some("owned-operation".into()), refusal: None });
        feedback.observe(&serde_json::json!({"operation":"owned-operation","state":"vendor_running"}));
        assert!(feedback.blocking && feedback.release_after_snapshot && !feedback.terminal);
        feedback.refreshed();
        assert!(!feedback.blocking && !feedback.release_after_snapshot && !feedback.terminal);
        assert_eq!(feedback.operation.as_deref(), Some("owned-operation"));
    }
    #[test]
    fn unconfirmed_restart_never_becomes_success_after_refresh() {
        let mut feedback = RequestFeedback::captured(Action::PluginRestart {
            instance: "ab".repeat(16), generation: 7,
            snapshot: RecoverySnapshot { revision: 3, sha256: "cd".repeat(32) },
        });
        feedback.receipt(&Receipt { schema: OPERATOR_SCHEMA, accepted: true,
            operation: Some("exact-operation".into()), refusal: None });
        let mut record = serde_json::json!({"operation":"other-operation", "state":"completed",
            "outcome":"unconfirmed", "result":{"restart":"unconfirmed"}});
        feedback.observe(&record);
        assert!(!feedback.terminal);
        record["operation"] = "exact-operation".into();
        feedback.observe(&record);
        assert!(feedback.terminal && feedback.blocking && feedback.release_after_snapshot);
        assert_eq!(feedback.transitions.last().map(String::as_str), Some("outcome_unconfirmed"));
        assert!(!feedback.transitions.iter().any(|state| state == "completed"));
        let warning = feedback.text.clone();
        assert!(warning.contains("could not be confirmed"));
        feedback.refreshed();
        assert!(!feedback.blocking);
        assert_eq!(feedback.text, warning);
        assert_eq!(feedback.operation.as_deref(), Some("exact-operation"));
        assert!(feedback.report.is_none());
    }
}

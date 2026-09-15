//! Versioned operator projection and closed requests. No filesystem or launch authority.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: u32,
    pub state_token: String,
    pub action: Action,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    PluginInspect { selection: String },
    PluginPrepare { selection: String },
    ExperimentalEnable { candidate: String },
    ExperimentalDisable { candidate: String },
    CandidateObserve { candidate: String, area: String, status: String, note: String },
    CandidateReview { candidate: String, accept: bool, rationale: String },
    CandidatePublishOrdinary { candidate: String },
    InstallerEnvironmentCreate {
        installer: String,
        runner: String,
    },
    InstallerNewAttempt {
        previous: String,
        runner: String,
    },
    InstallerStart {
        onboarding: String,
    },
    InstallerFocus {
        onboarding: String,
        operation: String,
    },
    InstallerStop {
        onboarding: String,
        operation: String,
    },
    InstallerScan {
        onboarding: String,
    },
    VendorApplicationOpen {
        application: String,
    },
    VendorApplicationFocus {
        application: String,
    },
    VendorApplicationStop {
        application: String,
    },
    EnvironmentRescan {
        environment: String,
    },
    OrdinaryRollback {
        class_id: String,
        publication: String,
    },
    OrdinaryRestoreRecommended {
        class_id: String,
    },
    TransactionReconcile {},
    CaptureArm {
        class_id: String,
    },
    CaptureDisarm {},
    IncidentExport {
        incident: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AvailableAction {
    pub label: String,
    pub action: Action,
    pub disabled_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    pub revision: u32,
    pub claim: String,
    pub publication: String,
    pub active: bool,
    pub rollback_allowed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub class_id: String,
    pub name: String,
    pub vendor: String,
    pub role: String,
    pub version: String,
    pub disposition: String,
    pub active_revision: Option<u32>,
    pub recommended_revision: Option<u32>,
    pub environment: String,
    pub runner: String,
    pub module_sha256: String,
    pub limitations: Vec<String>,
    pub history: Vec<History>,
    pub actions: Vec<AvailableAction>,
    pub details: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub id: String,
    pub family: String,
    pub runner: String,
    pub revision: u64,
    pub authorization: String,
    pub last_scan: serde_json::Value,
    pub actions: Vec<AvailableAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VendorApplication {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: String,
    pub actions: Vec<AvailableAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Incident {
    pub id: String,
    pub state: String,
    pub summary: serde_json::Value,
    pub export: Option<AvailableAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct System {
    pub service: String,
    pub keepers: usize,
    pub dsp: usize,
    pub maintenance: usize,
    pub ceiling: usize,
    pub pending_transactions: usize,
    pub stale_transports: usize,
    pub cleanup_unconfirmed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub onboarding: Vec<Onboarding>,
    pub schema: u32,
    pub state_token: String,
    pub system: System,
    pub environments: Vec<Environment>,
    pub vendor_applications: Vec<VendorApplication>,
    pub products: Vec<Product>,
    pub active_sessions: Vec<serde_json::Value>,
    pub capture: serde_json::Value,
    pub recent_incidents: Vec<Incident>,
    pub actions: Vec<AvailableAction>,
    pub operation: Option<serde_json::Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Onboarding {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<OperationFailure>,
    pub installer: String,
    pub name: String,
    pub byte_size: u64,
    pub format: String,
    pub environment: Option<String>,
    pub state: String,
    pub required_human_action: String,
    pub details: serde_json::Value,
    pub actions: Vec<AvailableAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: u32,
    pub accepted: bool,
    pub operation: Option<String>,
    pub refusal: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Activity {
    pub schema: u32,
    pub system: System,
    pub capture: serde_json::Value,
    pub operation: Option<serde_json::Value>,
}
impl System {
    /// Preserve the version-1 wire shape so an exactly retained frontend can
    /// still parse new manager readback. Only a successful LVC1 reply is active.
    pub fn capacity_available(&self) -> bool {
        self.service == "active"
    }
    pub fn inactive_reason(&self) -> Option<&'static str> {
        if !self.capacity_available() {
            Some("Service capacity unavailable; actions requiring inactivity are unsafe")
        } else if self.cleanup_unconfirmed {
            Some("Previous instance cleanup is unconfirmed")
        } else if self.dsp > 0 || self.maintenance > 0 {
            Some("Close active bridged instances before this action")
        } else {
            None
        }
    }
}
impl Action {
    pub fn requires_inactive(&self) -> bool {
        matches!(
            self,
            Self::PluginInspect { .. } | Self::PluginPrepare { .. } | Self::ExperimentalEnable { .. } | Self::ExperimentalDisable { .. } | Self::CandidatePublishOrdinary { .. } | Self::InstallerEnvironmentCreate { .. }
                | Self::InstallerNewAttempt { .. }
                | Self::InstallerStart { .. }
                | Self::InstallerScan { .. }
                | Self::VendorApplicationOpen { .. }
                | Self::EnvironmentRescan { .. }
                | Self::OrdinaryRollback { .. }
                | Self::OrdinaryRestoreRecommended { .. }
                | Self::TransactionReconcile {}
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unavailable_and_blocked_capacity_are_not_presented_as_active_dsp() {
        let mut s = System {
            service: "unavailable".into(),
            keepers: 0,
            dsp: 0,
            maintenance: 0,
            ceiling: 0,
            pending_transactions: 0,
            stale_transports: 0,
            cleanup_unconfirmed: true,
        };
        assert!(s.inactive_reason().unwrap().contains("unavailable"));
        s.service = "active".into();
        s.dsp = 1;
        assert_eq!(
            s.inactive_reason(),
            Some("Previous instance cleanup is unconfirmed")
        );
        s.cleanup_unconfirmed = false;
        assert!(s.inactive_reason().unwrap().contains("Close active"));
        s.dsp = 0;
        assert_eq!(s.inactive_reason(), None);
    }
    #[test]
    fn action_vocabulary_cannot_carry_a_command_path_or_pid() {
        for raw in [
            r#"{"kind":"run","command":"anything"}"#,
            r#"{"kind":"installer_start","onboarding":"a","path":"/tmp/x"}"#,
            r#"{"kind":"installer_environment_create","installer":"a","runner":"b","environment":".wine"}"#,
            r#"{"kind":"inspect_and_publish","class_id":"a"}"#,
            r#"{"kind":"capture_disarm","path":"/tmp/other"}"#,
            r#"{"kind":"vendor_application_focus","application":"asc","pid":42}"#,
            r#"{"kind":"ordinary_activate_candidate","class_id":"a"}"#,
        ] {
            assert!(serde_json::from_str::<Action>(raw).is_err());
        }
        let a = Action::CaptureDisarm {};
        assert_eq!(
            serde_json::from_slice::<Action>(&serde_json::to_vec(&a).unwrap()).unwrap(),
            a
        );
    }
}

// Additive schema-2 failure/readback projection. Closed fields carry no paths or PID claims.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperatorLock {
    #[serde(rename = "registry.lock")]
    Registry,
    #[serde(rename = "operator-canonical.lock")]
    Canonical,
    #[serde(rename = "operator-receipt.lock")]
    Receipt,
    #[serde(rename = "operator-resume.lock")]
    Resume,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockMode {
    Exclusive,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockPurpose {
    EnvironmentCreationAdmission,
    OperatorReadback,
    OperatorValidationReadback,
    ActionSerialization,
    OperationReceipt,
    ServiceRecovery,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WaitPolicy {
    Bounded,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockOutcome {
    Acquired,
    Timeout,
    Error,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LockHolder {
    Unknown,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureLayer {
    ManagerControlPlane,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureStage {
    EnvironmentCreationAdmission,
    OperatorValidationReadback,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    RegistryLockTimeout,
    SerializationLockTimeout,
    LockAccessError,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LockFacts {
    pub name: OperatorLock,
    pub mode: LockMode,
    pub purpose: LockPurpose,
    pub operation: Option<String>,
    pub policy: WaitPolicy,
    pub elapsed_wait_us: u64,
    pub attempts: u32,
    pub timeout_ms: u64,
    pub outcome: LockOutcome,
    pub holder: LockHolder,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperationFailure {
    pub layer: FailureLayer,
    pub stage: FailureStage,
    pub code: FailureCode,
    pub retryable: bool,
    pub mutation_started: bool,
    pub environment_created: bool,
    pub installer_launched: bool,
    pub lock: LockFacts,
}

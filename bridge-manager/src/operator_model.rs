//! Versioned operator projection and closed requests. No filesystem or launch authority.
use serde::{Deserialize, Serialize};
/// Manager/frontend wire generation. Durable installer, workspace and operation
/// records keep their own owner-defined schema versions.
pub const OPERATOR_SCHEMA: u32 = 16;
/// A launch-only DLL selection, never a Proton prefix conversion or driver install.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GraphicsBackend {
    WineD3d11,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioLayoutPolicy {
    StereoMainPair,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RendererPolicy {
    Inherited,
    SoftwareRendering,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Presentation {
    BlankWhite,
    RenderedNonblank,
    Unavailable,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Powershell {
    Inherited,
    IntentionallyUnavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: u32,
    pub state_token: String,
    pub action: Action,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PublicationIdentity {
    pub id: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    CandidateGraphicsAssess { candidate: String },
    CandidateGraphicsPrepare {
        candidate: String,
        backend: Option<GraphicsBackend>,
        expected_current: Option<PublicationIdentity>,
    },
    /// Explicit fixed-source runtime download; no caller path or URL.
    RuntimeInstall {},
    /// Explicit local allowlisted support report; no caller-selected path.
    SupportExport {},
    DependencyPrepare {},
    DependencyStop {
        operation: String,
    },
    RendererDiscover {},
    RendererOpen {
        application: String,
        policy: RendererPolicy,
    },
    RendererFocus {
        operation: String,
    },
    RendererStop {
        operation: String,
    },
    RendererObserve {
        operation: String,
        presentation: Presentation,
    },
    PluginInspect {
        selection: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        audio_layout: Option<AudioLayoutPolicy>,
    },
    PluginPrepare {
        selection: String,
        inspection: String,
        recipe: String,
        predecessor: Option<String>,
    },
    PluginReinspect {
        selection: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        audio_layout: Option<AudioLayoutPolicy>,
    },
    CompatibilityCheck {
        selection: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        audio_layout: Option<AudioLayoutPolicy>,
        recipe: String,
        predecessor: Option<String>,
    },
    /// Continue the exact retained check using a new request and its original
    /// immutable inspection/candidate checkpoints.
    CompatibilityResumeCheck { operation: String },
    CompatibilityPublishTest {
        candidate: String,
        expected_current: Option<PublicationIdentity>,
    },
    CompatibilityResult {
        candidate: String,
        expected_current: PublicationIdentity,
        result: TestResultKind,
        passed: Vec<TestArea>,
        failed_area: Option<TestArea>,
        note: String,
    },
    CompatibilityFinishResult { operation: String },
    ExperimentalReplace {
        candidate: String,
        expected_current: PublicationIdentity,
    },
    CandidateWithdraw {
        candidate: String,
        expected_current: PublicationIdentity,
    },
    ExperimentalEnable {
        candidate: String,
    },
    ExperimentalDisable {
        candidate: String,
    },
    CandidateObserve {
        candidate: String,
        area: String,
        status: String,
        note: String,
    },
    CandidateReview {
        candidate: String,
        accept: bool,
        rationale: String,
    },
    CandidatePublishOrdinary {
        candidate: String,
    },
    InstallerEnvironmentCreate {
        installer: String,
        runner: String,
    },
    InstallerRename {
        installer: String,
        label: String,
    },
    InstallerNewAttempt {
        previous: String,
        runner: String,
    },
    InstallerStart {
        onboarding: String,
    },
    InstallerStartWithPolicy {
        onboarding: String,
        powershell: Powershell,
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
    QuarantinedModuleRetry {
        environment: String,
        scan: String,
        module_index: usize,
        module_sha256: String,
        report_sha256: String,
    },
    OrdinaryRollback {
        class_id: String,
        publication: String,
    },
    OrdinaryRestoreRecommended {
        class_id: String,
    },
    BufferingSet {
        class_id: String,
        added_frames: u32,
    },
    TransactionReconcile {},
    CaptureArm {
        class_id: String,
    },
    CaptureDisarm {},
    IncidentExport {
        incident: String,
    },
    WorkspaceSelectInstaller {
        installer: String,
        release: String,
    },
    WorkspaceInstall {},
    WorkspaceFinishInstall {},
    WorkspaceLaunch {},
    WorkspaceUninstall {},
    WorkspaceFinishUninstall {},
    WorkspaceFocus {},
    WorkspaceStop {},
    WorkspaceSelectProductInstaller {
        product: WorkspaceProductId,
        installer: String,
        release: String,
    },
    WorkspaceInstallProduct {
        product: WorkspaceProductId,
    },
    WorkspaceFinishProductInstall {
        product: WorkspaceProductId,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceProductId {
    Serum2,
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
    #[serde(default)]
    pub compatibility: Option<CompatibilityWorkflow>,
    pub details: serde_json::Value,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TestArea {
    DawLoad, Midi, Audio, Editor, Parameters, Automation, StateRecall,
    ProcessingRestart, Retirement,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProblemCategory {
    FailedToLoad, BlankEditor, EditorFroze, EditorDisappeared,
    NoAudio, IncorrectAudio, ControlsUnresponsive, HostCrashed,
    CleanupIncomplete, Other,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TestResultKind {
    Worked,
    Problem { category: ProblemCategory },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityPhase {
    NotChecked, CheckBlocked, Checking, CheckFailed, ReadyForTest,
    AvailableForTest, AwaitingRetirement, TestResultIncomplete, PassedExperimental,
    NeedsWork, OrdinarySupported, PublicationNeedsAttention, HistoricalOnly,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityWorkflow {
    pub phase: CompatibilityPhase,
    pub summary: String,
    pub established: Vec<String>,
    pub remaining: Vec<String>,
    pub current_inspection: Option<String>,
    pub current_candidate: Option<String>,
    pub primary: Option<AvailableAction>,
    pub alternatives: Vec<AvailableAction>,
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
    #[serde(default)]
    pub details: serde_json::Value,
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
pub struct DawWorkspace {
    pub id: String,
    pub name: String,
    pub state: String,
    pub selected_installer: String,
    #[serde(default)]
    pub application_installers: Vec<String>,
    pub selected_release: String,
    pub installed_advertised_release: Option<String>,
    pub observed_file_version: Option<String>,
    pub installed_image_sha256: Option<String>,
    pub active_installation_operation: Option<String>,
    pub cleanup: String,
    pub first_useful_failure: Option<String>,
    pub actions: Vec<AvailableAction>,
    pub installer_choices: Vec<AvailableAction>,
    #[serde(default)]
    pub products: Vec<DawWorkspaceProduct>,
    pub details: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DawWorkspaceProduct {
    pub id: WorkspaceProductId,
    pub name: String,
    pub state: String,
    pub selected_release: Option<String>,
    pub module_sha256: Option<String>,
    pub current_failure: Option<String>,
    pub actions: Vec<AvailableAction>,
    pub installer_choices: Vec<AvailableAction>,
    pub details: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
    #[serde(default)]
    pub installer_setups: Vec<InstallerSetup>,
    pub schema: u32,
    pub state_token: String,
    pub system: System,
    pub environments: Vec<Environment>,
    pub vendor_applications: Vec<VendorApplication>,
    pub products: Vec<Product>,
    #[serde(default)]
    pub workspaces: Vec<DawWorkspace>,
    pub active_sessions: Vec<serde_json::Value>,
    pub capture: serde_json::Value,
    pub recent_incidents: Vec<Incident>,
    pub actions: Vec<AvailableAction>,
    pub operation: Option<serde_json::Value>,
}
/// Current presentation only. The nested snapshot is deliberately scoped to
/// current products and setup: histories and diagnostic detail are absent.
/// The full `operator snapshot` endpoint retains its separate deep contract.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractiveOverview {
    pub schema: u32,
    pub operator_schema: u32,
    pub scope: String,
    pub current_generation: String,
    pub current: Snapshot,
    pub readiness: ReadinessAssessment,
}
/// One current product with its exact manager-offered compatibility controls.
/// Deep histories for unrelated products remain in the Diagnostics snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrentProductDetail {
    pub schema: u32,
    pub operator_schema: u32,
    pub state_token: String,
    pub current_generation: String,
    /// Capacity from the same capture that supplied this product's offers.
    pub system: System,
    pub product: Product,
    pub environments: Vec<Environment>,
    pub vendor_applications: Vec<VendorApplication>,
}
/// Cheap change signal for the ordinary idle frontend. It never grants an
/// action or certifies product bytes.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pulse {
    pub schema: u32,
    /// A change detector, not a capacity or cleanup-certainty receipt.
    pub service_state: String,
    pub dsp: Option<usize>,
    pub keepers: Option<usize>,
    pub maintenance: Option<usize>,
    pub pending_transactions: usize,
    /// Lightweight service change signal; absence invalidates current health.
    pub cleanup_unconfirmed: Option<bool>,
    pub current_generation: String,
    pub operation: Option<serde_json::Value>,
    pub operation_live: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessOutcome { Ready, ActionRequired, Unsupported, Unknown }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstallationHealth { Healthy, ActionRequired, Unknown }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SupportQualification { Verified, Unsupported, NotYetQualified }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactCertainty { Observed, ProfileRequired, Inferred, Unknown }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadinessFact {
    pub name: String,
    pub value: Option<String>,
    pub source: String,
    pub observed_at: u64,
    pub certainty: FactCertainty,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadinessProduct {
    pub name: String,
    pub class_id: String,
    pub module_sha256: String,
    pub profile: Option<String>,
    pub status: ReadinessOutcome,
    pub installation_health: InstallationHealth,
    pub support_qualification: SupportQualification,
    pub reason: String,
    pub failure_code: Option<String>,
    pub facts: Vec<ReadinessFact>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadinessBlocker {
    pub category: String,
    pub status: ReadinessOutcome,
    pub explanation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessStep {
    pub title: String,
    pub detail: String,
    pub action: Option<AvailableAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessAssessment {
    pub schema: u32,
    pub state_token: String,
    pub observed_at: u64,
    pub overall_status: ReadinessOutcome,
    pub system: System,
    pub platform: Vec<ReadinessFact>,
    pub daw: Vec<ReadinessFact>,
    pub audio: Vec<ReadinessFact>,
    pub graphics: Vec<ReadinessFact>,
    pub runtime: Vec<ReadinessFact>,
    pub products: Vec<ReadinessProduct>,
    pub blockers: Vec<ReadinessBlocker>,
    pub ordered_steps: Vec<ReadinessStep>,
    pub support_export_action: AvailableAction,
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SetupPhase {
    Imported,
    EnvironmentReady,
    InstallerRunning,
    SetupNeedsAttention,
    InstallerRetired,
    ScanReady,
    DiscoveryComplete,
    CleanupUnconfirmed,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveredProduct {
    pub environment: String,
    pub module_sha256: String,
    pub class_id: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallerSetup {
    pub installer: String,
    pub name: String,
    pub label_source: String,
    pub byte_size: u64,
    pub format: String,
    pub imported_at: u64,
    pub phase: SetupPhase,
    pub status: String,
    pub environment: Option<String>,
    pub compatibility: Option<String>,
    pub discovered: Vec<DiscoveredProduct>,
    pub primary: Option<AvailableAction>,
    pub secondary: Vec<AvailableAction>,
    pub rename: AvailableAction,
    pub history: Vec<SetupHistoryRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupHistoryRef {
    pub environment: Option<String>,
    pub state: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InstallerImportResult {
    pub schema: u32,
    pub installer_sha256: String,
    pub byte_size: u64,
    pub format: String,
    pub newly_imported: bool,
    pub display_label: String,
    pub created_at: u64,
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
    #[serde(default)]
    pub workspace_product_install_ready: bool,
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
            Self::RuntimeInstall {}
                | Self::CandidateGraphicsAssess { .. }
                | Self::CandidateGraphicsPrepare { .. }
                | Self::DependencyPrepare {}
                | Self::RendererDiscover {}
                | Self::RendererOpen { .. }
                | Self::PluginReinspect { .. }
                | Self::CompatibilityCheck { .. }
                | Self::CompatibilityResumeCheck { .. }
                | Self::CompatibilityPublishTest { .. }
                | Self::CompatibilityResult { result: TestResultKind::Worked, .. }
                | Self::CompatibilityFinishResult { .. }
                | Self::ExperimentalReplace { .. }
                | Self::CandidateWithdraw { .. }
                | Self::PluginInspect { .. }
                | Self::PluginPrepare { .. }
                | Self::ExperimentalEnable { .. }
                | Self::ExperimentalDisable { .. }
                | Self::CandidatePublishOrdinary { .. }
                | Self::InstallerEnvironmentCreate { .. }
                | Self::InstallerNewAttempt { .. }
                | Self::InstallerStart { .. }
                | Self::InstallerStartWithPolicy { .. }
                | Self::InstallerScan { .. }
                | Self::VendorApplicationOpen { .. }
                | Self::EnvironmentRescan { .. }
                | Self::QuarantinedModuleRetry { .. }
                | Self::OrdinaryRollback { .. }
                | Self::OrdinaryRestoreRecommended { .. }
                | Self::BufferingSet { .. }
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
            r#"{"kind":"quarantined_module_retry","environment":"a","scan":"b","module_index":0,"module_sha256":"c","report_sha256":"d","path":"/tmp/plugin.vst3"}"#,
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

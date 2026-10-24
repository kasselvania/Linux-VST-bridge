//! Owner-composed display trials and ordered repair guidance. Action references
//! must still resolve to current offers; none of these facts authorizes work.
use super::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum DisplayTestStatus { Passed, Failed, NotTested, Unavailable, NotApplicable }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum DisplayWitness { Operator, Machine, Generated }
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum DisplayEvidenceState { FailureRetained, PassingObservation, NotEstablished }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DisplayResult {
    pub candidate: String,
    pub ordinal: u64,
    pub recorded_at: u64,
    pub status: DisplayTestStatus,
    pub witness: DisplayWitness,
    pub detail: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DisplayTrial {
    pub candidate: String,
    pub predecessor: Option<String>,
    pub previous_publication: Option<PublicationIdentity>,
    pub settings: LocalSettings,
    pub current_inputs: bool,
    pub selected: bool,
    pub last_result: Observation<Option<DisplayResult>>,
    /// Any retained failure on this exact candidate wins, even after a later pass.
    pub evidence: DisplayEvidenceState,
    pub actions: Vec<Action>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DisplayModeChoice {
    pub backend: Option<GraphicsBackend>,
    pub label: String,
    pub selected: bool,
    pub suggested: bool,
    pub prepare: Option<Action>,
    /// Latest recorded editor result among the listed exact-context trials.
    /// Candidate identity remains attached; this is not a mode-wide support claim.
    pub last_result: Observation<Option<DisplayResult>>,
    pub trials: Vec<DisplayTrial>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all="snake_case")]
pub enum RepairStepKind {
    OpenStandalone, AssessDisplay, TryDisplayMode, ProfileAccessibility,
    RepairInstallation, RestorePrevious, ReportProblem,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepairStep {
    pub kind: RepairStepKind,
    pub label: String,
    pub detail: String,
    pub action: Option<Action>,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepairLadder {
    pub profile_id: String,
    pub profile_revision: u32,
    pub evidence: Vec<String>,
    pub steps: Vec<RepairStep>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DisplayConfiguration {
    pub observed_candidate: String,
    pub selected_candidate: Option<String>,
    pub modes: Vec<DisplayModeChoice>,
    pub repair: RepairLadder,
}
impl DisplayConfiguration {
    pub fn unavailable() -> Observation<Self> {
        Observation::Unavailable("Exact display configuration has not been observed".into())
    }
}

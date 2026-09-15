//! Durable preparation records, distinct from registry/publication authority.
use crate::{catalogue::NativeArtifact, observation::Census, profiles::Profile, *};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub schema: u32,
    pub environment: Environment,
    pub module: Artifact,
    pub class: crate::inventory::Class,
    pub scanner: Artifact,
    pub scanner_source: String,
    pub factory_report: Artifact,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    ManagedPreparation,
    RetainedSv1,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Inspection {
    pub schema: u32,
    pub selection: Selection,
    pub report: Artifact,
    pub host: Artifact,
    pub source_manifest: Artifact,
    pub controller: ControllerAssociation,
    pub origin: Origin,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ControllerAssociation {
    Combined,
    Separate { class_id: String },
    Unavailable { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub schema: u32,
    pub selection: Selection,
    pub inspection: Inspection,
    pub profile: Profile,
    pub native: NativeArtifact,
    pub host: Artifact,
    pub source_manifest: Artifact,
    pub origin: Origin,
    pub recipe_sha256: String,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Area {
    DawLoad,
    Midi,
    Audio,
    Editor,
    Parameters,
    Automation,
    StateRecall,
    Retirement,
}
pub const AREAS: [Area; 8] = [
    Area::DawLoad,
    Area::Midi,
    Area::Audio,
    Area::Editor,
    Area::Parameters,
    Area::Automation,
    Area::StateRecall,
    Area::Retirement,
];
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    Passed,
    Failed,
    NotTested,
    Unavailable,
    NotApplicable,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Witness {
    OperatorObservation,
    MachineObservation,
    GeneratedRegression,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: u32,
    pub candidate: String,
    pub operation: String,
    pub area: Area,
    pub status: TestStatus,
    pub witness: Witness,
    pub detail: String,
    pub recorded_at: u64,
    pub ordinal: u64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewChoice {
    AcceptExactLocal,
    NeedsWork,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub schema: u32,
    pub candidate: String,
    pub operation: String,
    pub evidence_sha256: String,
    pub choice: ReviewChoice,
    pub rationale: String,
    pub authority: String,
    pub recorded_at: u64,
    pub ordinal: u64,
    pub ordinary_profile: Option<Profile>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub selection: String,
    pub inspection: String,
    pub controller: Option<ControllerAssociation>,
    pub preliminary: serde_json::Value,
    pub candidate: Option<String>,
    pub preparation: String,
    pub publication: String,
    pub origin: Option<Origin>,
    pub evidence: Vec<Observation>,
    pub unmet_requirements: Vec<String>,
    pub review: Option<Decision>,
    pub operation: Option<serde_json::Value>,
}
impl Selection {
    pub fn id(&self) -> Result<String> {
        super::key(self)
    }
}
impl Candidate {
    pub fn id(&self) -> Result<String> {
        super::key(self)
    }
    pub fn census(&self) -> Result<Census> {
        Census::from_report(
            crate::catalogue::EnvironmentBinding {
                family: crate::profiles::Family::ManagedInstallerV1,
                environment: self.selection.environment.clone(),
            },
            self.selection.module.clone(),
            crate::observation::ModuleStamp::read(&self.selection.module.path)?,
            self.inspection.host.clone(),
            self.inspection.source_manifest.sha256.clone(),
            self.inspection.report.clone(),
            &self.selection.class.id,
        )
    }
}

impl Inspection {
    pub fn census(&self) -> Result<Census> {
        Census::from_report(
            crate::catalogue::EnvironmentBinding {
                family: crate::profiles::Family::ManagedInstallerV1,
                environment: self.selection.environment.clone(),
            },
            self.selection.module.clone(),
            crate::observation::ModuleStamp::read(&self.selection.module.path)?,
            self.host.clone(),
            self.source_manifest.sha256.clone(),
            self.report.clone(),
            &self.selection.class.id,
        )
    }
}

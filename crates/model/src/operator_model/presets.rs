//! Preset facts and closed commands. File lists and proprietary bytes remain
//! private to the service; the window shows counts and acts on exact offers.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresetProduct {
    pub environment: String,
    pub module_sha256: String,
    pub class_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresetCommand {
    Backup,
    Link,
    Open,
    Import,
    Restore {
        backup: String,
        missing_digest: String,
        confirmed: bool,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresetLocation {
    Known,
    Unknown,
    Unavailable,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PresetFiles {
    pub location: PresetLocation,
    pub explanation: String,
    pub user_path: Option<std::path::PathBuf>,
    pub linux_path: Option<std::path::PathBuf>,
    pub files: Option<usize>,
    pub bytes: Option<u64>,
    pub backup: Option<String>,
    pub missing: usize,
    pub conflicts: usize,
    pub actions: Vec<AvailableAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetResult {
    pub schema: u32,
    pub receipt: Receipt,
    /// Present only for a successful current owner-validated Open command.
    pub folder: Option<std::path::PathBuf>,
}

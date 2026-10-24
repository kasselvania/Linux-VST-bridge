//! Exact proxy recovery. State bytes remain inside the admitted DAW process.
use super::{AvailableAction, Observation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecoverySnapshot {
    pub revision: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InstanceRecovery {
    /// Original admitted session, retained by the live proxy across host replacement.
    pub instance: String,
    pub generation: u64,
    pub snapshot: Observation<RecoverySnapshot>,
    pub uncaptured_changes_possible: bool,
    pub detail: String,
    pub restart: Option<AvailableAction>,
}

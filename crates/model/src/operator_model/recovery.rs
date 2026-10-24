//! Exact proxy recovery. State bytes remain inside the admitted DAW process.
use super::{AvailableAction, Observation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecoverySnapshot {
    pub revision: u64,
    pub sha256: String,
}

/// Current authenticated proxy observation, never a command acknowledgment.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryPhase {
    #[default]
    Unknown,
    Connected,
    Failed,
    Restarting,
    Restored,
    Refused,
    Uncertain,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InstanceRecovery {
    #[serde(default)]
    pub phase: RecoveryPhase,
    /// Original admitted session, retained by the live proxy across host replacement.
    pub instance: String,
    pub generation: u64,
    pub snapshot: Observation<RecoverySnapshot>,
    pub uncaptured_changes_possible: bool,
    pub detail: String,
    pub restart: Option<AvailableAction>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_phase_is_unknown_and_unknown_phase_is_refused() {
        let record = serde_json::json!({"instance":"instance","generation":1,
            "snapshot":{"state":"unavailable","value":"missing"},
            "uncaptured_changes_possible":false,"detail":"Restarted","restart":null});
        let decoded: InstanceRecovery = serde_json::from_value(record.clone()).unwrap();
        assert_eq!(decoded.phase, RecoveryPhase::Unknown);
        let mut unknown = record;
        unknown["phase"] = "future_success".into();
        assert!(serde_json::from_value::<InstanceRecovery>(unknown).is_err());
    }
}

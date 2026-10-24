//! Read-only observations of exact admitted native instances. Never action authority.
use super::{Observation, ProductIdentity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsumerIdentity {
    pub boot_id: String,
    pub pid: i32,
    pub uid: u32,
    pub start_ticks: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConsumerObservation {
    pub identity: ConsumerIdentity,
    /// A kernel identity check, independent of process names or DAW guesses.
    pub current: Observation<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransportIdentity {
    pub device: u64,
    pub inode: u64,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportedLifecycle {
    Initializing,
    Activated,
    Started,
    Stopped,
    Deactivated,
    Created,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProcessingObservation {
    /// Completed frame positions increased within the same transport and epoch.
    /// This includes silent processing and does not establish audible output.
    Advancing { epoch: u64, completed_end: u64, observed_age_ms: u64 },
    LastReportedStopped { epoch: u64, observed_age_ms: u64 },
    /// A first or unchanged counter is not proof of a stopped or failed instance.
    NotObservedAdvancing { epoch: u64, completed_end: u64, observed_age_ms: u64 },
    Unavailable { reason: String },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalObservation { NotMeasured }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceFailure { WindowsHostExited, EditorControllerFailed, TransportFailed }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupState {
    NotReported,
    Unconfirmed,
    Confirmed,
    Unavailable(String),
}

/// A name returned by the VST3 host interface, reported by the exact admitted
/// consumer. It is display provenance, never verification of a DAW or authority.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostIdentitySource { Vst3HostApplication }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostApplication {
    pub name: String,
    pub source: HostIdentitySource,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceKind { Daw, Standalone }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LiveInstance {
    pub kind: InstanceKind,
    pub session: String,
    pub product: Observation<ProductIdentity>,
    pub consumer: Observation<ConsumerObservation>,
    /// Host-reported name; a sandbox host may report its enclosing application.
    pub daw: Observation<HostApplication>,
    pub transport: Observation<TransportIdentity>,
    pub lifecycle: Observation<ReportedLifecycle>,
    pub processing: ProcessingObservation,
    pub signal: SignalObservation,
    pub terminal: Observation<Option<InstanceFailure>>,
    pub cleanup: CleanupState,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LiveActivity {
    /// Unresolved leases invalidate coverage; an empty observed census is explicit.
    pub instances: Observation<Vec<LiveInstance>>,
}

impl LiveActivity {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self { instances: Observation::Unavailable(reason.into()) }
    }
}

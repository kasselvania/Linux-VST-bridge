//! Read-only facts composed by the service. None of these records grants an action.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", content = "value", rename_all = "snake_case")]
pub enum Observation<T> {
    Observed(T),
    Unavailable(String),
}
impl<T> Observation<T> {
    pub fn observed(&self) -> Option<&T> {
        match self { Self::Observed(value) => Some(value), Self::Unavailable(_) => None }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct ProductIdentity {
    pub environment: String,
    pub module_sha256: String,
    pub class_id: String,
}
impl From<&Product> for ProductIdentity {
    fn from(product: &Product) -> Self {
        Self { environment: product.environment.clone(), module_sha256: product.module_sha256.clone(),
            class_id: product.class_id.clone() }
    }
}
impl ProductIdentity {
    pub fn matches(&self, product: &Product) -> bool {
        self.environment == product.environment && self.module_sha256 == product.module_sha256
            && self.class_id.eq_ignore_ascii_case(&product.class_id)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AttentionTarget {
    Product { product: ProductIdentity },
    Setup { installer: String, environment: Option<String> },
    Service,
    Session { session: String },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Attention {
    pub purpose: AttentionPurpose,
    pub target: AttentionTarget,
    pub title: String,
    pub detail: String,
    /// References an existing current offer; this record never authorizes execution.
    pub action: Option<Action>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttentionPurpose { Service, Installation, Recovery, Connection }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SetupDisposition {
    InProgress,
    NeedsAttention,
    RecordsNeedRefresh,
    PartialInstallation,
    SettledHistory,
    Complete,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionDisposition { Available, Removed, NeedsAttention, NotAvailable }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JourneyConnection {
    pub selection: String,
    pub product: Observation<ProductIdentity>,
    pub disposition: Observation<ConnectionDisposition>,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupState {
    pub installer: String,
    pub environment: Option<String>,
    pub name: String,
    pub disposition: SetupDisposition,
    pub headline: String,
    pub status: String,
    /// Exact current connection facts; missing selections remain unobserved.
    pub connections: Vec<JourneyConnection>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionProgress { Waiting, Inspecting, Preparing, Publishing }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProductProgress {
    pub product: ProductIdentity,
    pub stage: ConnectionProgress,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CurrentPresentation {
    /// Owner order: running/uncertain/failed setups before settled history.
    pub setups: Vec<SetupState>,
    /// Owner order and exact subjects, shared by Home and Activity.
    pub attention: Vec<Attention>,
    /// Current operation progress positively bound to the exact selected build.
    pub progress: Vec<ProductProgress>,
    pub change_previews: Vec<ChangePreview>,
    pub package_update: PackageUpdatePreview,
    pub settled_message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IncidentSummary {
    pub id: String,
    pub state: String,
    pub outcome: Observation<String>,
    pub cleanup_confirmed: Observation<bool>,
    pub transport_retired: Observation<bool>,
    /// A retained allowlisted report exists; it is not read on the ordinary page.
    pub report_retained: bool,
}

/// Retained terminal summaries, not a complete history of every closed session.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecentInstance {
    pub session: String,
    pub class_id: String,
    pub product: Observation<ProductIdentity>,
    pub failure: InstanceFailure,
    pub cleanup_confirmed: bool,
    pub transport_retired: bool,
    pub observed_at: u64,
}

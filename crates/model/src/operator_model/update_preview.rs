//! Service-composed impact of a currently offered change. These facts grant no action.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SetupSummary {
    pub environment: String,
    pub name: String,
    pub products: Vec<ProductIdentity>,
    pub product_names: Vec<String>,
    pub runtime: Observation<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChangePreview {
    /// References the exact current offer, including its expected setup revision.
    pub action: Action,
    pub affected: Vec<SetupSummary>,
    pub unchanged: Vec<SetupSummary>,
    pub consequence: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PackageUpdatePreview {
    /// A bridge package update preserves every setup's runtime and installation.
    /// Its bridge executables and native publications are updated separately.
    pub preserved_setups: Vec<SetupSummary>,
}

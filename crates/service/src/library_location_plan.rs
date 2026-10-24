//! Closed launch input. Folder selection and availability policy belong to the
//! location owner; supervisors only verify and apply this exact snapshot.
use crate::{catalogue::Software, Environment, Manager, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocationPlan {
    pub schema: u32,
    pub digest: String,
    pub locations: Vec<LocationGrant>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LocationGrant {
    pub path: PathBuf,
    pub drive: String,
    pub read_only: bool,
    pub available: bool,
    pub device: u64,
    pub inode: u64,
    pub volume: String,
    pub handle_type: i32,
    pub handle: Vec<u8>,
}

pub fn for_launch(m: &Manager, environment: &Environment, software: &Software)
    -> Result<Option<LocationPlan>> {
    let plan = crate::library_locations::launch_plan(m, environment)?;
    if plan.is_some() {
        crate::supervisor::require_library_locations(&software.supervisor)?;
    }
    Ok(plan)
}

pub fn bind(m: &Manager, environment: &Environment, software: &Software,
    spec: &mut serde_json::Value) -> Result<()> {
    if let Some(plan) = for_launch(m, environment, software)? {
        spec["library_locations"] = serde_json::to_value(plan)?;
    }
    Ok(())
}

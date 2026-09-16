//! IS4 selection is scoped to one installer operation, never an environment default.
use crate::{catalogue::Software, *};
use serde_json::{json, Value};

pub use crate::operator_model::Powershell;

/// Called by the existing manager launch owner after canonical inactivity and
/// onboarding admission. No caller environment strings or paths are accepted.
/// The generated qualification helper uses this identical construction owner.
pub fn bind(spec: &mut Value, software: &Software, powershell: Powershell) -> Result<()> {
    require(spec["schema"] == 2 && spec.get("installer_capability").is_none(), "installer_policy_spec")?;
    require(spec["operation"].as_str().is_some_and(|v| valid_hex(v,32)), "installer_policy_operation")?;
    let environment: Environment = serde_json::from_value(spec["environment"].clone())?;
    require(valid_hex(&environment.id,32) && environment.revision > 0, "installer_policy_environment")?;
    let installer: Artifact = serde_json::from_value(spec["installer"].clone())?;
    installer.verify()?;
    for artifact in [&software.manager, &software.supervisor, &software.ownership] { artifact.verify()?; }
    // Full software digest binds also host, adapter and generation metadata.
    let software_record = serde_json::to_value(software)?;
    let software_sha256 = hex(&sha2::Sha256::digest(serde_json::to_vec(&software_record)?));
    spec["installer_capability"] = json!({"schema":1,"operation":spec["operation"],
        "environment":environment,"installer":installer,
        "software_sha256":software_sha256,"software":software_record,
        "owners":{"manager":software.manager,"supervisor":software.supervisor,"ownership":software.ownership},
        "windows_scripting":{"powershell":powershell}});
    spec["schema"] = json!(3);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_closed_capabilities_and_actions_deserialize() {
        for mode in ["inherited","intentionally_unavailable"] {
            let a = json!({"kind":"installer_start_with_policy","onboarding":"ab".repeat(16),"powershell":mode});
            let parsed: crate::operator_model::Action = serde_json::from_value(a.clone()).unwrap();
            assert!(parsed.requires_inactive());
            for key in ["environment","override","command","path","pid","arguments","WINEDLLOVERRIDES"] {
                let mut bad=a.clone();bad[key]=json!("powershell.exe=");
                assert!(serde_json::from_value::<crate::operator_model::Action>(bad).is_err());
            }
        }
        for mode in ["required_interpreter","powershell.exe=","unavailable",""] {
            assert!(serde_json::from_value::<Powershell>(json!(mode)).is_err());
        }
    }
}

//! Environment inventory is factory discovery, never compatibility authority.
use crate::{
    observation::{decode, windows_class_id},
    *,
};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Class {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub category: String,
    pub subcategories: String,
    pub role: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub artifact: Artifact,
    pub classes: Vec<Class>,
    pub report: Artifact,
    pub inspection_error: Option<String>,
    pub quarantine_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scan {
    pub schema: u32,
    pub id: String,
    pub environment: Environment,
    pub host: Artifact,
    pub host_source_sha256: String,
    pub completed_at: u64,
    pub modules: Vec<Module>,
    #[serde(default)]
    pub changes: Changes,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Changes {
    pub initial_scan: bool,
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
    pub unchanged: usize,
}
/// Locations correlate an installation delta; module digests remain byte identity.
pub fn changes(prior: Option<&[Module]>, current: &[Module]) -> Changes {
    let previous = prior.unwrap_or(&[]);
    let mut result = Changes {
        initial_scan: prior.is_none(),
        ..Changes::default()
    };
    for module in current {
        match previous
            .iter()
            .find(|old| old.artifact.path == module.artifact.path)
        {
            None => result.added += 1,
            Some(old) if old.artifact.sha256 == module.artifact.sha256 => result.unchanged += 1,
            Some(_) => result.changed += 1,
        }
    }
    result.removed = previous
        .iter()
        .filter(|old| {
            !current
                .iter()
                .any(|module| module.artifact.path == old.artifact.path)
        })
        .count();
    result
}
/// Consume only the complete SDK factory record. Follow-on component inspection
/// may refuse a multi-class module; that distinct outcome is retained by Module.
pub fn classes(report: &Value) -> Result<Vec<Class>> {
    require(
        report["cleanup_confirmed"] == true && report["transport_retired"] == true,
        "inventory_cleanup_unconfirmed",
    )?;
    let records = report["records"].as_array().ok_or("inventory_records")?;
    require(records.len() <= 4096, "inventory_record_bound")?;
    let found: Vec<_> = records
        .iter()
        .filter(|v| v["state"] == "ap8_factory")
        .collect();
    require(found.len() == 1, "inventory_factory_absent_or_duplicate")?;
    let factory = found[0];
    let list = factory["classes"].as_array().ok_or("inventory_classes")?;
    require(
        !list.is_empty()
            && list.len() <= 256
            && factory["class_count"].as_u64() == Some(list.len() as u64),
        "inventory_class_bound",
    )?;
    let vendor = decode(&factory["factory"], "vendor_hex", false)?;
    let mut ids = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    for c in list {
        let id = windows_class_id(c["raw_tuid_hex"].as_str().ok_or("inventory_class_id")?)?;
        require(ids.insert(id.clone()), "inventory_duplicate_class")?;
        let tier = c["tier"].as_str().ok_or("inventory_tier")?;
        let unicode = tier == "IPluginFactory3.PClassInfoW";
        require(
            unicode
                || matches!(
                    tier,
                    "IPluginFactory2.PClassInfo2" | "IPluginFactory.PClassInfo"
                ),
            "inventory_tier",
        )?;
        let extended = unicode || tier == "IPluginFactory2.PClassInfo2";
        let name = decode(c, "name_hex", unicode)?;
        let category = decode(c, "category_hex", false)?;
        let subcategories = if extended {
            decode(c, "subcategories_hex", false)?
        } else {
            String::new()
        };
        let class_vendor = if extended {
            decode(c, "vendor_hex", unicode)?
        } else {
            String::new()
        };
        let version = if extended {
            decode(c, "version_hex", unicode)?
        } else {
            String::new()
        };
        let role = if category != "Audio Module Class" {
            "other"
        } else if subcategories.split('|').any(|s| s == "Instrument") {
            "instrument"
        } else if subcategories.split('|').any(|s| s == "Fx") {
            "effect"
        } else {
            "unknown"
        };
        require(
            [
                &name,
                &category,
                &subcategories,
                &class_vendor,
                &version,
                &vendor,
            ]
            .iter()
            .all(|s| !s.chars().any(char::is_control)),
            "inventory_metadata_control",
        )?;
        result.push(Class {
            id,
            name,
            vendor: if class_vendor.is_empty() {
                vendor.clone()
            } else {
                class_vendor
            },
            version,
            category,
            subcategories,
            role: role.into(),
        });
    }
    Ok(result)
}

/// Retained scan facts are current only under the same exact scanner and environment.
pub fn stale_reason(
    module: &Module,
    observed_environment: &Environment,
    observed_host: &Artifact,
    observed_source: &str,
    environment: &Environment,
    host: &Artifact,
    source: &str,
) -> Option<&'static str> {
    if observed_host.sha256 != host.sha256 || observed_source != source || observed_host.verify().is_err() || host.verify().is_err() {
        Some("Scanner host changed — rescan under current scanner host")
    } else if observed_environment != environment {
        Some("Environment changed — rescan required")
    } else if module.artifact.verify().is_err() {
        Some("Module changed or missing — rescan required")
    } else {
        None
    }
}

/// A specific complete factory result, never an exit-code-only reinterpretation.
pub fn inspection_hint(module: &Module) -> Result<Option<&'static str>> {
    module.report.verify()?;
    let raw:Value=read_json(&module.report.path)?;
    let all=classes(&raw)?;
    let explicit=raw["records"].as_array().is_some_and(|rows|rows.iter().any(|r|r["state"]=="ap8_failure" && r["reason"]=="multiple audio classes require explicit selection"));
    Ok(if all.iter().filter(|c|c.category=="Audio Module Class").count()>1 && explicit {Some("Discovery succeeded. Select an audio class for deeper inspection.")}else{None})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> Value {
        serde_json::json!({"cleanup_confirmed":true,"transport_retired":true,"records":[{"state":"ap8_factory","factory":{"vendor_hex":hex(b"New Vendor")},"class_count":1,"classes":[{"raw_tuid_hex":"01000000020003000400000000000000","tier":"IPluginFactory2.PClassInfo2","name_hex":hex(b"Unknown synth"),"category_hex":hex(b"Audio Module Class"),"subcategories_hex":hex(b"Instrument|Synth"),"vendor_hex":"","version_hex":hex(b"1.0")}]}]})
    }
    #[test]
    fn inventory_requires_current_module_environment_host_and_source() {
        let f = crate::test_fixture::Fixture::new();
        let module = Module {
            artifact: f.r.module.clone(),
            classes: vec![],
            report: f.r.module.clone(),
            quarantine_reason: None,
            inspection_error: None,
        };
        let env = &f.r.environment;
        let host = &f.r.host;
        let source = &f.r.host_source_sha256;
        assert_eq!(
            stale_reason(&module, env, host, source, env, host, source),
            None
        );
        let mut changed = host.clone();
        changed.sha256 = "bb".repeat(32);
        assert!(
            stale_reason(&module, env, host, source, env, &changed, source)
                .unwrap()
                .contains("Scanner host changed")
        );
        assert!(
            stale_reason(&module, env, host, source, env, host, &"cc".repeat(32))
                .unwrap()
                .contains("Scanner host changed")
        );
        let mut changed = env.clone();
        changed.revision += 1;
        assert!(
            stale_reason(&module, env, host, source, &changed, host, source)
                .unwrap()
                .contains("Environment changed")
        );
        fs::write(&module.artifact.path, b"changed module").unwrap();
        assert!(
            stale_reason(&module, env, host, source, env, host, source)
                .unwrap()
                .contains("Module changed")
        );
    }
    #[test]
    fn scan_delta_separates_location_and_bytes() {
        let module = |path: &str, hash: &str| Module {
            artifact: Artifact {
                path: path.into(),
                sha256: hash.into(),
            },
            classes: vec![],
            report: Artifact {
                path: "report".into(),
                sha256: "report".into(),
            },
            inspection_error: None,
            quarantine_reason: None,
        };
        let prior = vec![module("a", "1"), module("b", "2"), module("c", "3")];
        let current = vec![module("a", "1"), module("b", "4"), module("d", "3")];
        assert_eq!(
            changes(Some(&prior), &current),
            Changes {
                initial_scan: false,
                added: 1,
                changed: 1,
                removed: 1,
                unchanged: 1
            }
        );
        assert_eq!(changes(None, &current).added, 3);
        assert!(changes(None, &current).initial_scan);
    }
    #[test]
    fn unknown_factory_is_inventory_not_a_profile() {
        let mut r = report();
        r["error"] = "multiple audio classes require explicit selection".into();
        let c = classes(&r).unwrap();
        assert_eq!(c[0].role, "instrument");
        assert_eq!(c[0].vendor, "New Vendor");
        assert_eq!(c[0].id, "00000001000200030400000000000000");
        assert!(!serde_json::to_string(&c).unwrap().contains("activation"));
        r["cleanup_confirmed"] = false.into();
        assert!(classes(&r).is_err());
    }
    #[test]
    fn incomplete_duplicate_or_unbounded_census_is_refused() {
        let mut r = report();
        r["records"][0]["class_count"] = 2.into();
        assert!(classes(&r).is_err());
        let mut r = report();
        let c = r["records"][0]["classes"][0].clone();
        r["records"][0]["classes"].as_array_mut().unwrap().push(c);
        r["records"][0]["class_count"] = 2.into();
        assert!(classes(&r).is_err());
    }
}

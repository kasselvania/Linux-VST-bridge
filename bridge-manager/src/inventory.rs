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

#[cfg(test)]
mod tests {
    use super::*;
    fn report() -> Value {
        serde_json::json!({"cleanup_confirmed":true,"transport_retired":true,"records":[{"state":"ap8_factory","factory":{"vendor_hex":hex(b"New Vendor")},"class_count":1,"classes":[{"raw_tuid_hex":"01000000020003000400000000000000","tier":"IPluginFactory2.PClassInfo2","name_hex":hex(b"Unknown synth"),"category_hex":hex(b"Audio Module Class"),"subcategories_hex":hex(b"Instrument|Synth"),"vendor_hex":"","version_hex":hex(b"1.0")}]}]})
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

//! Bounded supervised SDK facts and exact profile selection, outside the DAW.
use crate::{catalogue::*, profiles::*, *};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModuleStamp {
    pub device: u64,
    pub inode: u64,
    pub bytes: u64,
    pub modified: i64,
    pub modified_ns: i64,
}
impl ModuleStamp {
    pub fn read(path: &Path) -> Result<Self> {
        let m = file(path)?.metadata()?;
        Ok(Self {
            device: m.dev(),
            inode: m.ino(),
            bytes: m.len(),
            modified: m.mtime(),
            modified_ns: m.mtime_nsec(),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Census {
    pub schema: u32,
    pub id: String,
    pub captured_at: u64,
    pub environment: EnvironmentBinding,
    pub module: Artifact,
    pub module_stamp: ModuleStamp,
    pub host: Artifact,
    pub host_source_sha256: String,
    pub report: Artifact,
    pub factory_vendor: String,
    pub classes: Vec<String>,
    pub selected: Metadata,
    pub parameter_count: u32,
    pub float32: bool,
    pub float64: bool,
}
fn one<'a>(records: &'a [Value], state: &str) -> Result<&'a Value> {
    let found: Vec<_> = records.iter().filter(|r| r["state"] == state).collect();
    require(found.len() == 1, "census_record_missing_or_duplicate")?;
    Ok(found[0])
}
fn string<'a>(v: &'a Value, key: &str, bound: usize) -> Result<&'a str> {
    let text = v[key].as_str().ok_or("census_field")?;
    require(
        text.len() <= bound && !text.chars().any(char::is_control),
        "census_field_bound",
    )?;
    Ok(text)
}
fn bytes(s: &str) -> Result<Vec<u8>> {
    require(
        s.len().is_multiple_of(2) && s.len() <= 1024 && s.bytes().all(|b| b.is_ascii_hexdigit()),
        "census_hex",
    )?;
    Ok((0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect())
}
fn decode(v: &Value, key: &str, unicode: bool) -> Result<String> {
    let b = bytes(string(v, key, 1024)?)?;
    if unicode {
        require(b.len().is_multiple_of(2), "census_utf16")?;
        Ok(String::from_utf16(
            &b.chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )?)
    } else {
        Ok(String::from_utf8(b)?)
    }
}
fn windows_class_id(raw: &str) -> Result<String> {
    require(valid_hex(raw, 32), "census_class_identity")?;
    let mut b = bytes(raw)?;
    b[..4].reverse();
    b[4..6].reverse();
    b[6..8].reverse();
    Ok(hex(&b).to_uppercase())
}
impl Census {
    pub fn from_report(
        environment: EnvironmentBinding,
        module: Artifact,
        stamp: ModuleStamp,
        host: Artifact,
        source: String,
        report: Artifact,
        class_id: &str,
    ) -> Result<Self> {
        report.verify()?;
        let value: Value = read_json(&report.path)?;
        require(
            value["cleanup_confirmed"] == true
                && value["transport_retired"] == true
                && value["gated"] == true
                && value["error"].is_null(),
            "inspection_failed",
        )?;
        let records = value["records"].as_array().ok_or("inspection_records")?;
        require(records.len() <= 4096, "inspection_record_bound")?;
        require(
            one(records, "ap8_inspection_closed")?["exit_code"] == 0
                && one(records, "scanner_completed")?["inspection_complete"] == true,
            "inspection_incomplete",
        )?;
        let ready = one(records, "readiness_announced")?;
        require(
            ready["module_sha256"] == module.sha256
                && ready["scanner_sha256"] == host.sha256
                && ready["implementation_source_manifest_sha256"] == source,
            "inspection_binding_mismatch",
        )?;
        let sdk = one(records, "ap12_class")?;
        let selected = Metadata {
            class_id: string(sdk, "class_id", 32)?.to_uppercase(),
            name: string(sdk, "name", 63)?.into(),
            vendor: string(sdk, "vendor", 63)?.into(),
            version: string(sdk, "version", 63)?.into(),
            subcategories: string(sdk, "subcategories", 127)?.into(),
            metadata_tier: string(sdk, "metadata_tier", 32)?.into(),
        };
        selected.verify()?;
        require(selected.class_id == class_id, "class_mismatch")?;
        let factory = one(records, "ap8_factory")?;
        let factory_vendor = decode(&factory["factory"], "vendor_hex", false)?;
        let classes = factory["classes"].as_array().ok_or("census_classes")?;
        require(
            !classes.is_empty()
                && classes.len() <= 256
                && factory["class_count"].as_u64() == Some(classes.len() as u64),
            "census_class_bound",
        )?;
        let mut ids = std::collections::BTreeSet::new();
        let mut selected_seen = false;
        for c in classes {
            let id = windows_class_id(string(c, "raw_tuid_hex", 32)?)?;
            require(ids.insert(id.clone()), "duplicate_census_class")?;
            if id != class_id {
                continue;
            }
            selected_seen = true;
            let tier = match string(c, "tier", 32)? {
                "IPluginFactory3.PClassInfoW" => "factory_3_unicode",
                "IPluginFactory2.PClassInfo2" => "factory_2",
                _ => return Err("metadata_tier_unsupported".into()),
            };
            let unicode = tier == "factory_3_unicode";
            require(
                tier == selected.metadata_tier
                    && decode(c, "category_hex", false)? == "Audio Module Class",
                "metadata_mismatch",
            )?;
            let vendor = decode(c, "vendor_hex", unicode)?;
            require(
                decode(c, "name_hex", unicode)? == selected.name
                    && (if vendor.is_empty() {
                        &factory_vendor
                    } else {
                        &vendor
                    }) == &selected.vendor
                    && decode(c, "version_hex", unicode)? == selected.version
                    && decode(c, "subcategories_hex", false)? == selected.subcategories,
                "metadata_mismatch",
            )?;
        }
        require(selected_seen, "class_absent")?;
        let count = one(records, "ap8_parameter_count")?["count"]
            .as_u64()
            .ok_or("parameter_count")?;
        require(count <= 8192, "parameter_count_bound")?;
        let mut parameters = std::collections::BTreeSet::new();
        for row in records.iter().filter(|r| r["state"] == "ap8_parameters") {
            let list = row["parameters"].as_array().ok_or("parameter_records")?;
            require(list.len() <= 32, "parameter_chunk_bound")?;
            for p in list {
                let id = p[0].as_u64().ok_or("parameter_id")?;
                require(
                    id <= u32::MAX as u64 && parameters.insert(id),
                    "duplicate_parameter_id",
                )?;
            }
        }
        require(
            parameters.len() == count as usize,
            "parameter_metadata_incomplete",
        )?;
        let caps = one(records, "ap12_capabilities")?;
        require(
            caps["float32_result"].as_i64().is_some() && caps["float64_result"].as_i64().is_some(),
            "census_precision_absent",
        )?;
        Ok(Self {
            schema: 1,
            id: random_id()?,
            captured_at: now()?,
            environment,
            module,
            module_stamp: stamp,
            host,
            host_source_sha256: source,
            report,
            factory_vendor,
            classes: ids.into_iter().collect(),
            selected,
            parameter_count: count as u32,
            float32: caps["float32_result"] == 0,
            float64: caps["float64_result"] == 0,
        })
    }
    pub fn verify_current(
        &self,
        root: &Path,
        host: &Artifact,
        source: &str,
        at: u64,
    ) -> Result<()> {
        require(
            self.schema == 1
                && valid_hex(&self.id, 32)
                && at >= self.captured_at
                && at - self.captured_at <= 600,
            "stale_census",
        )?;
        require(
            self.module_stamp == ModuleStamp::read(&self.module.path)?,
            "stale_census",
        )?;
        self.module.verify().map_err(|_| "module_digest_changed")?;
        self.report.verify().map_err(|_| "census_report_changed")?;
        // A cached projection is not authority by itself. Reconsume its exact
        // retained supervised report before trusting any normalized facts.
        let mut observed = Self::from_report(
            self.environment.clone(),
            self.module.clone(),
            self.module_stamp.clone(),
            self.host.clone(),
            self.host_source_sha256.clone(),
            self.report.clone(),
            &self.selected.class_id,
        )?;
        observed.id = self.id.clone();
        observed.captured_at = self.captured_at;
        require(observed == *self, "census_projection_changed")?;
        require(
            self.host == *host && self.host_source_sha256 == source,
            "installed_host_mismatch",
        )?;
        self.host.verify()?;
        let e = &self.environment.environment;
        require(
            e.root == root.join("environments").join(&e.id)
                && read_json::<Environment>(&e.root.join("environment.json"))? == *e,
            "environment_mismatch",
        )?;
        e.runner.verify().map_err(|_| "runner_mismatch")?;
        require(
            self.module
                .path
                .starts_with(e.root.join("compatdata/pfx/drive_c"))
                && self.module.path.canonicalize()? == self.module.path,
            "module_location",
        )?;
        Ok(())
    }
}

pub fn select<'a>(profiles: &'a [Profile], facts: &Census) -> Result<&'a Profile> {
    validate_set(profiles)?;
    let eligible: Vec<_> = profiles
        .iter()
        .filter(|p| p.class.class_id == facts.selected.class_id && p.claim != Claim::Withdrawn)
        .collect();
    require(!eligible.is_empty(), "profile_no_match")?;
    let exact: Vec<_> = eligible
        .iter()
        .copied()
        .filter(|p| p.module_sha256 == facts.module.sha256)
        .collect();
    require(!exact.is_empty(), "module_digest_changed")?;
    let mut matched = Vec::new();
    let mut refusal = "profile_no_match".to_string();
    for p in exact {
        let result = (|| -> Result<()> {
            require(facts.classes.contains(&p.class.class_id), "class_absent")?;
            require(role(&facts.selected)? == p.role, "role_mismatch")?;
            require(
                facts.selected == p.class && facts.factory_vendor == p.factory_vendor,
                "metadata_mismatch",
            )?;
            p.verify_environment(&facts.environment.environment, &facts.environment.family)?;
            require(
                facts.host.sha256 == p.requirements.host_sha256
                    && facts.host_source_sha256 == p.requirements.host_source_sha256,
                "installed_host_mismatch",
            )?;
            require(facts.float32 && !facts.float64, "precision_mismatch")?;
            Ok(())
        })();
        match result {
            Ok(()) => matched.push(p),
            Err(e) => refusal = e.to_string(),
        }
    }
    require(!matched.is_empty(), &refusal)?;
    require(matched.len() == 1, "profile_ambiguous")?;
    Ok(matched[0])
}

pub fn derive(profile: &Profile, facts: &Census, native: &NativeArtifact) -> Result<Registration> {
    select(std::slice::from_ref(profile), facts)?;
    native.matches(profile)?;
    native.artifact.verify()?;
    Ok(Registration {
        metadata: facts.selected.clone(),
        environment: facts.environment.environment.clone(),
        module: facts.module.clone(),
        host: facts.host.clone(),
        host_source_sha256: facts.host_source_sha256.clone(),
        native: native.artifact.clone(),
        compatibility: profile.capabilities.compatibility(),
    })
}

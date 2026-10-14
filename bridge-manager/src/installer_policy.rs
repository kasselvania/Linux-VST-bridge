//! IS4 selection is scoped to one installer operation, never an environment default.
use crate::{catalogue::Software, *};
use serde_json::{json, Value};

mod metadata;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstallerMatch {
    container: String,
    company_name: String,
    product_name: String,
    product_version: String,
    file_version: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchProfile {
    schema: u32,
    id: String,
    installer: InstallerMatch,
    managed_runtime_revision: String,
    powershell: Powershell,
    reason: String,
}

/// Reviewed installer advice is optional. Unknown metadata, versions, formats
/// and runtimes keep the ordinary launch; they are not an admission failure.
/// An explicit operation choice always overrides automatic advice.
pub fn bind_for_launch(m: &Manager, spec: &mut Value, software: &Software,
    explicit: Option<Powershell>) -> Result<()> {
    bind_for_launch_with(spec, software, explicit, |runner|
        Ok(runtime_delivery::selected_record_path(m, runner)?.is_some()))
}
fn bind_for_launch_with(spec: &mut Value, software: &Software, explicit: Option<Powershell>,
    retained_runtime: impl FnOnce(&Runner) -> Result<bool>) -> Result<()> {
    if let Some(policy) = explicit { return bind(spec, software, policy); }
    if eligible_adapter_record(spec["format"].as_str().unwrap_or(""), software).is_err() { return Ok(()); }
    let profile: LaunchProfile = serde_json::from_slice(include_bytes!("installer_policy/native_access_close_check.json"))?;
    require(profile.schema == 1 && !profile.id.is_empty() && !profile.reason.is_empty(), "installer_profile_record")?;
    let environment: Environment = serde_json::from_value(spec["environment"].clone())?;
    if environment.runner.id != profile.managed_runtime_revision { return Ok(()); }
    let installer: Artifact = serde_json::from_value(spec["installer"].clone())?;
    // Classification failure only declines advice. Existing import and launch
    // owners still verify and supervise the exact selected executable.
    let inspected = file(&installer.path).and_then(|mut source| metadata::inspect(&mut source));
    let Ok(inspected) = inspected else { return Ok(()); };
    let expected = &profile.installer;
    if inspected.container != expected.container || [
        ("CompanyName", &expected.company_name), ("ProductName", &expected.product_name),
        ("ProductVersion", &expected.product_version), ("FileVersion", &expected.file_version),
    ].iter().any(|(key, value)| inspected.strings.get(*key) != Some(*value)) { return Ok(()); }
    // A missing or mismatched retained qualification declines automatic advice;
    // it cannot turn an otherwise ordinary installer into a forbidden one.
    if !retained_runtime(&environment.runner).unwrap_or(false) { return Ok(()); }
    bind(spec, software, profile.powershell)
}

pub use crate::operator_model::Powershell;

/// The production policy is qualified only for the exact PE/IS2 adapter route.
pub fn eligible_adapter<'a>(format: &str, software: &'a Software) -> Result<&'a Artifact> {
    let adapter = eligible_adapter_record(format, software)?;
    adapter.verify()?;
    Ok(adapter)
}
pub fn eligible_adapter_record<'a>(format: &str, software: &'a Software) -> Result<&'a Artifact> {
    require(format == "pe_executable", "installer_policy_format")?;
    let adapter = software.installer_launch.as_ref().ok_or("installer_policy_adapter_missing")?;
    adapter.validate_record()?;
    Ok(adapter)
}

/// Called by the existing manager launch owner after canonical inactivity and
/// onboarding admission. No caller environment strings or paths are accepted.
/// The generated qualification helper uses this identical construction owner.
pub fn bind(spec: &mut Value, software: &Software, powershell: Powershell) -> Result<()> {
    require(spec["schema"] == 2 && spec.get("installer_capability").is_none(), "installer_policy_spec")?;
    require(spec["operation"].as_str().is_some_and(|v| valid_hex(v,32)), "installer_policy_operation")?;
    let adapter = eligible_adapter(spec["format"].as_str().unwrap_or(""), software)?;
    require(spec["installer_launch"] == serde_json::to_value(adapter)?, "installer_policy_adapter_mismatch")?;
    let environment: Environment = serde_json::from_value(spec["environment"].clone())?;
    require(environment_revision::valid_environment_id(&environment.id) && environment.revision > 0, "installer_policy_environment")?;
    let installer: Artifact = serde_json::from_value(spec["installer"].clone())?;
    installer.verify()?;
    for artifact in [&software.manager, &software.supervisor, &software.ownership] { artifact.verify()?; }
    // Full software digest binds also host, adapter and generation metadata.
    let software_record = serde_json::to_value(software)?;
    let software_sha256 = hex(&sha2::Sha256::digest(serde_json::to_vec(&software_record)?));
    spec["installer_capability"] = json!({"schema":1,"operation":spec["operation"],
        "environment":environment,"installer":installer,"format":"pe_executable","installer_launch":adapter,
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
        let old = json!({"kind":"environment_installer_start","environment":"ab".repeat(16),"installer":"cd".repeat(32)});
        let parsed: crate::operator_model::Action = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(),old);
        assert_eq!(parsed.environment_installer_start().unwrap().2,None);
        let explicit = json!({"kind":"environment_installer_start_with_policy","environment":"ab".repeat(16),
            "installer":"cd".repeat(32),"powershell":"inherited"});
        let parsed: crate::operator_model::Action = serde_json::from_value(explicit).unwrap();
        assert!(parsed.requires_inactive());
        assert_eq!(parsed.environment_installer_start().unwrap().2,Some(Powershell::Inherited));
        let old=json!({"kind":"software_install","installer":"ab".repeat(32),"daw":"reaper"});
        let parsed:crate::operator_model::Action=serde_json::from_value(old.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(),old);
    }

    fn fixture() -> (crate::test_fixture::Fixture, Software, Value) {
        let f = crate::test_fixture::Fixture::new();
        let path = f.outer.join("arbitrary-editable-name.exe");
        fs::write(&path, metadata::fixture(None)).unwrap();
        let installer = Artifact { sha256: digest(&path).unwrap(), path };
        let owner = f.r.host.clone();
        let software = Software { manager: owner.clone(), supervisor: owner.clone(), ownership: owner.clone(),
            host: owner.clone(), source_manifest: owner.clone(), source_sha256: f.r.host_source_sha256.clone(),
            operator_frontend: None, native_catalogue: None, preparation_kit: None, installer_launch: Some(owner) };
        let mut environment = f.r.environment.clone();
        environment.id = "ab".repeat(16);
        environment.runner.id = "managed-ge-proton11-7-slr4-20260805-r4".into();
        let spec = json!({"schema":2,"operation":"cd".repeat(16),"environment":environment,
            "installer":installer,"format":"pe_executable","installer_launch":software.installer_launch});
        (f, software, spec)
    }
    #[test]
    fn policy_binding_uses_the_shared_environment_id_contract() {
        for id in ["11111111111141118111111111111111","11111111-1111-4111-8111-111111111111"] {
            for policy in [Powershell::Inherited,Powershell::IntentionallyUnavailable] {
                let (_f,software,mut spec)=fixture();spec["environment"]["id"]=json!(id);
                bind_for_launch_with(&mut spec,&software,Some(policy),|_|panic!("explicit policy")).unwrap();
                assert_eq!(spec["installer_capability"]["environment"]["id"],id);
            }
        }
    }
    #[test]
    fn ordinary_launch_selects_content_class_advice_and_binds_exact_operation() {
        let (_f, software, mut spec) = fixture();
        let before = spec.clone();
        let environment = serde_json::to_vec(&spec["environment"]).unwrap();
        bind_for_launch_with(&mut spec, &software, None, |runner| {
            assert_eq!(runner, &serde_json::from_value::<Environment>(before["environment"].clone()).unwrap().runner);
            Ok(true)
        }).unwrap();
        assert_eq!(spec["schema"], 3);
        assert_eq!(spec["installer_capability"]["windows_scripting"]["powershell"], "intentionally_unavailable");
        for key in ["operation", "installer", "environment", "installer_launch"] {
            assert_eq!(spec["installer_capability"][key], before[key]);
        }
        assert_eq!(serde_json::to_vec(&spec["environment"]).unwrap(), environment);
        assert_eq!(spec["installer_capability"]["owners"]["manager"], serde_json::to_value(&software.manager).unwrap());
    }
    #[test]
    fn unknown_versions_vendor_class_and_runtime_keep_ordinary_attempts() {
        for (key, value) in [("ProductVersion", "3.27.0"), ("FileVersion", "3.26.0.963"),
            ("CompanyName", "Unrelated vendor"), ("ProductName", "Another application")] {
            let (_f, software, mut spec) = fixture();
            let artifact: Artifact = serde_json::from_value(spec["installer"].clone()).unwrap();
            fs::write(&artifact.path, metadata::fixture(Some((key, value)))).unwrap();
            spec["installer"]["sha256"] = json!(digest(&artifact.path).unwrap());
            let before = spec.clone();
            bind_for_launch_with(&mut spec, &software, None, |_| panic!("metadata mismatch must not select advice")).unwrap();
            assert_eq!(spec, before);
        }
        let (f, software, spec) = fixture();
        for qualified in [false, true] {
            let mut unknown = spec.clone();
            if qualified { unknown["environment"]["runner"]["id"] = json!("operator-imported-runtime"); }
            let before = unknown.clone();
            bind_for_launch_with(&mut unknown, &software, None, |_| Ok(false)).unwrap();
            assert_eq!(unknown, before);
        }
        let mut missing_record = spec.clone();
        bind_for_launch(&f.m, &mut missing_record, &software, None).unwrap();
        assert_eq!(missing_record, spec);
        let mut mismatched_record = spec.clone();
        bind_for_launch_with(&mut mismatched_record, &software, None, |_| Err("managed_runtime_selected_record_changed".into())).unwrap();
        assert_eq!(mismatched_record, spec);
        for format in ["msi_compound", "unknown"] {
            let mut ordinary = spec.clone(); ordinary["format"] = json!(format);
            let before = ordinary.clone();
            bind_for_launch_with(&mut ordinary, &software, None, |_| panic!("unrelated format")).unwrap();
            assert_eq!(ordinary, before);
        }
        let artifact: Artifact = serde_json::from_value(spec["installer"].clone()).unwrap();
        for (from,to) in [("StringFileInfo","XtringFileInfo"),("040904b0","notlang!")] {
            let mut bytes = metadata::fixture(None);
            let from:Vec<_> = from.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let to:Vec<_> = to.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let at = bytes.windows(from.len()).position(|value|value == from).unwrap();
            bytes[at..at+to.len()].copy_from_slice(&to);
            fs::write(&artifact.path,bytes).unwrap();
            let mut malformed = spec.clone(); malformed["installer"]["sha256"] = json!(digest(&artifact.path).unwrap());
            let before = malformed.clone();
            bind_for_launch_with(&mut malformed,&software,None,|_|panic!("malformed resource must decline advice")).unwrap();
            assert_eq!(malformed,before);
        }
        fs::write(&artifact.path, b"unclassified executable metadata").unwrap();
        let mut unclassified = spec.clone();
        bind_for_launch_with(&mut unclassified, &software, None, |_| panic!("no installer class")).unwrap();
        assert_eq!(unclassified, spec);
    }
    #[test]
    fn explicit_operation_choice_wins_and_changed_content_cannot_inherit_advice() {
        let (_f, software, spec) = fixture();
        let mut inherited = spec.clone();
        bind_for_launch_with(&mut inherited, &software, Some(Powershell::Inherited), |_| panic!("explicit choice")).unwrap();
        assert_eq!(inherited["installer_capability"]["windows_scripting"]["powershell"], "inherited");
        let mut changed = spec.clone();
        changed["installer"]["sha256"] = json!("00".repeat(32));
        assert!(bind_for_launch_with(&mut changed, &software, None, |_| Ok(true)).is_err());
        assert_eq!(changed["schema"], 2);
        assert!(changed.get("installer_capability").is_none());
    }
}

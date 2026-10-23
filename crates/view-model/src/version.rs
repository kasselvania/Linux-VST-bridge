//! The installed bridge, what Setup holds beside it, and the runtimes in use,
//! for the title row, Diagnostics and bug reports. Versions and names only:
//! never an account, licence, token or path.
use super::product::daw_name;
use crate::model::{BridgeBuild, SetupPackage, Snapshot};

/// "Installed: Linux VST Bridge 0.12.0test144", from the installed generation record.
pub fn installed_line(build: &BridgeBuild) -> String {
    match &build.installed {
        Some(version) => format!("Installed: Linux VST Bridge {version}"),
        None => "Installed: Linux VST Bridge (version unknown)".into(),
    }
}

/// Title-row note, only for a package the musician can apply.
pub fn setup_title_note(build: &BridgeBuild) -> Option<String> {
    match &build.setup {
        SetupPackage::Newer(version) => Some(format!("Update ready: {version}")),
        SetupPackage::DifferentBuild(_) => Some("Different build ready in Setup".into()),
        _ => None,
    }
}

/// What Setup holds beside the installed bridge, for Diagnostics and reports.
pub fn setup_line(build: &BridgeBuild) -> Option<String> {
    match &build.setup {
        SetupPackage::Newer(version) => Some(format!("Update ready in Setup: {version}")),
        SetupPackage::DifferentBuild(version) =>
            Some(format!("A different build of {version} is ready in Setup")),
        SetupPackage::Older(version) => Some(format!("Setup holds an older version ({version})")),
        SetupPackage::Unchecked => Some("Setup's package could not be checked".into()),
        SetupPackage::Nothing => None,
    }
}

/// A product-delivered runtime's revision ("r5"). Any other runner id may name
/// a person or machine, so it is never shown.
fn runtime_name(id: &str) -> String {
    match id.strip_prefix("managed-").and_then(|rest| rest.rsplit_once("-r")) {
        Some((_, n)) if !n.is_empty() && n.len() <= 4 && n.bytes().all(|b| b.is_ascii_digit()) =>
            format!("r{n}"),
        _ => "custom runtime".into(),
    }
}

/// Compatibility runtimes the plug-in setups use, else the ones installed.
pub fn runtime_summary(snapshot: &Snapshot) -> Option<String> {
    let mut names: Vec<_> = snapshot.environments.iter().map(|e| runtime_name(&e.runner)).collect();
    if names.is_empty() {
        names = snapshot.managed_runtime_records.iter()
            .filter(|record| record["failure"].is_null())
            .filter_map(|record| record["id"].as_str().map(runtime_name)).collect();
    }
    names.sort();
    names.dedup();
    (!names.is_empty()).then(|| names.join(", "))
}

/// DAWs chosen for installations, when any was chosen.
fn chosen_daws(snapshot: &Snapshot) -> Option<String> {
    let mut daws: Vec<_> = snapshot.installer_setups.iter()
        .filter_map(|setup| setup.journey.as_ref().map(|journey| daw_name(journey.daw))).collect();
    daws.sort();
    daws.dedup();
    (!daws.is_empty()).then(|| daws.join(", "))
}

/// Plain text for a bug report. Versions and names only: no account, licence,
/// token or path.
pub fn version_details(snapshot: &Snapshot, distribution: Option<&str>) -> String {
    let mut lines = vec![installed_line(&snapshot.bridge)];
    lines.extend(setup_line(&snapshot.bridge));
    lines.push(format!("Compatibility runtime: {}",
        runtime_summary(snapshot).as_deref().unwrap_or("not installed")));
    lines.push(format!("Distribution: {}", distribution.unwrap_or("unknown")));
    if let Some(daws) = chosen_daws(snapshot) {
        lines.push(format!("DAW chosen in Setup: {daws}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        crate::samples::base()
    }

    #[test]
    fn version_details_name_build_runtime_and_distribution_only() {
        let mut s = snapshot();
        s.bridge = BridgeBuild { installed: Some("0.12.0test144".into()), setup: SetupPackage::Nothing };
        for environment in &mut s.environments {
            environment.runner = "managed-ge-proton11-7-slr4-20260805-r5".into();
        }
        assert_eq!(setup_title_note(&s.bridge), None);
        assert_eq!(version_details(&s, Some("CachyOS Linux")), "Installed: Linux VST Bridge 0.12.0test144\n\
            Compatibility runtime: r5\nDistribution: CachyOS Linux");
        s.bridge.setup = SetupPackage::Newer("0.12.0test145".into());
        s.environments[0].runner = "managed-ge-proton11-7-slr4-20260805-r4".into();
        assert_eq!(setup_title_note(&s.bridge).as_deref(), Some("Update ready: 0.12.0test145"));
        assert_eq!(version_details(&s, None), "Installed: Linux VST Bridge 0.12.0test144\n\
            Update ready in Setup: 0.12.0test145\nCompatibility runtime: r4, r5\nDistribution: unknown");
        // Only a newer or different build is offered in the title row.
        for (setup, line) in [
            (SetupPackage::DifferentBuild("0.12.0test144".into()),
                "A different build of 0.12.0test144 is ready in Setup"),
            (SetupPackage::Older("0.12.0test140".into()), "Setup holds an older version (0.12.0test140)"),
            (SetupPackage::Unchecked, "Setup's package could not be checked"),
        ] {
            s.bridge.setup = setup.clone();
            assert_eq!(setup_line(&s.bridge).as_deref(), Some(line));
            assert_eq!(setup_title_note(&s.bridge).is_some(),
                matches!(setup, SetupPackage::DifferentBuild(_)));
            assert!(version_details(&s, None).contains(line));
        }
        // Without plug-in setups, the installed runtime is the one named.
        s.bridge = BridgeBuild::default();
        s.environments.clear();
        s.managed_runtime_records = vec![
            serde_json::json!({"id":"managed-ge-proton11-7-slr4-20260805-r3","failure":"changed"}),
            serde_json::json!({"id":"managed-ge-proton11-7-slr4-20260805-r5","failure":null})];
        assert_eq!(installed_line(&s.bridge), "Installed: Linux VST Bridge (version unknown)");
        assert_eq!(setup_line(&s.bridge), None);
        assert_eq!(runtime_summary(&s).as_deref(), Some("r5"));
        s.managed_runtime_records.clear();
        assert!(version_details(&s, None).contains("Compatibility runtime: not installed"));
    }

    #[test]
    fn only_a_delivered_runtime_is_named_and_any_other_runner_stays_private() {
        assert_eq!(runtime_name("managed-ge-proton11-7-slr4-20260805-r5"), "r5");
        for private in ["pk-deck-wine", "/home/someone/.steam/proton", "proton-9.0-r2",
            "managed-ge-proton-rx", "managed-pk-deck-r"] {
            assert_eq!(runtime_name(private), "custom runtime", "{private}");
        }
        let mut s = snapshot();
        s.environments[0].runner = "pk-deck-wine".into();
        s.environments[1].runner = "managed-ge-proton11-7-slr4-20260805-r5".into();
        s.environments.truncate(2);
        let details = version_details(&s, None);
        assert!(details.contains("Compatibility runtime: custom runtime, r5"));
        assert!(!details.contains("pk") && !details.contains("deck"));
    }
}

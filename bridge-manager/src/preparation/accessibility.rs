//! Exact preparation settings; a proposed profile remains a review candidate.
//! No environment-wide override or vendor/name-based compatibility inference.
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rules {
    schema: u32,
    rules: Vec<Rule>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    module_sha256: String,
    class_id: String,
    runner_id: String,
    runner_version: String,
    proton_sha256: String,
    entry_point_sha256: String,
    uiautomationcore_sha256: String,
    accessibility: Accessibility,
    evidence: String,
}
impl Rule {
    fn matches(&self, selection: &Selection) -> bool {
        let runner = &selection.environment.runner;
        selection.module.sha256 == self.module_sha256
            && selection.class.id == self.class_id
            && runner.id == self.runner_id
            && runner.version == self.runner_version
            && runner.files.iter().any(|a| a.path == runner.proton && a.sha256 == self.proton_sha256)
            && runner.files.iter().any(|a| a.path == runner.entry_point && a.sha256 == self.entry_point_sha256)
    }
}
fn rules() -> Result<Rules> {
    let rules: Rules = serde_json::from_slice(include_bytes!("../../../compatibility/preparation-accessibility.json"))?;
    require(rules.schema == 1 && rules.rules.len() <= 16, "preparation_accessibility_schema")?;
    let mut identities = std::collections::BTreeSet::new();
    for rule in &rules.rules {
        require(valid_hex(&rule.module_sha256, 64) && valid_hex(&rule.class_id, 32)
            && valid_hex(&rule.proton_sha256, 64) && valid_hex(&rule.entry_point_sha256, 64)
            && valid_hex(&rule.uiautomationcore_sha256, 64)
            && rule.accessibility == Accessibility::DisabledForVendorProcess
            && !rule.runner_id.is_empty() && rule.runner_id.len() <= 128
            && !rule.runner_version.is_empty() && rule.runner_version.len() <= 256
            && rule.evidence.starts_with("evidence/") && !rule.evidence.contains("..")
            && identities.insert((&rule.module_sha256, &rule.class_id, &rule.runner_id, &rule.runner_version)),
            "preparation_accessibility_rule")?;
    }
    Ok(rules)
}
pub(super) fn selected(selection: &Selection) -> Result<(Accessibility, Option<String>)> {
    let rules = rules()?;
    let matching: Vec<_> = rules.rules.iter().filter(|r| r.matches(selection)).collect();
    require(matching.len() <= 1, "preparation_accessibility_ambiguous")?;
    let Some(rule) = matching.first() else { return Ok((Accessibility::WindowsDefault, None)); };
    // Verify the faulting component itself. A runtime name or entry script alone
    // cannot establish the identity of its Wine DLLs.
    let dll = selection.environment.runner.proton.parent().ok_or("runner_parent")?
        .join("files/lib/wine/x86_64-windows/uiautomationcore.dll");
    require(digest(&dll)? == rule.uiautomationcore_sha256, "preparation_accessibility_runtime_changed")?;
    Ok((rule.accessibility.clone(), Some(rule.evidence.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_policy_matches_exact_module_class_and_runtime_only() {
        let rule = rules().unwrap().rules.remove(0);
        let (_fixture, candidate) = crate::preparation::tests::fixture();
        let mut selection = candidate.selection;
        selection.module.sha256 = rule.module_sha256.clone();
        selection.class.id = rule.class_id.clone();
        let runner = &mut selection.environment.runner;
        runner.id = rule.runner_id.clone();
        runner.version = rule.runner_version.clone();
        runner.files = vec![Artifact { path: runner.proton.clone(), sha256: rule.proton_sha256.clone() },
            Artifact { path: runner.entry_point.clone(), sha256: rule.entry_point_sha256.clone() }];
        assert!(rule.matches(&selection));
        let dll = selection.environment.runner.proton.parent().unwrap()
            .join("files/lib/wine/x86_64-windows/uiautomationcore.dll");
        fs::create_dir_all(dll.parent().unwrap()).unwrap();
        fs::write(&dll, b"different runtime DLL").unwrap();
        assert!(selected(&selection).is_err(), "a matching name cannot authorize another DLL");
        let mut changed = selection.clone(); changed.module.sha256 = "00".repeat(32);
        assert!(!rule.matches(&changed));
        changed = selection.clone(); changed.class.id = "00".repeat(16); assert!(!rule.matches(&changed));
        changed = selection.clone(); changed.environment.runner.version.push_str(" successor"); assert!(!rule.matches(&changed));
        changed = selection.clone(); changed.environment.runner.files[0].sha256 = "00".repeat(32); assert!(!rule.matches(&changed));
        changed = selection; changed.environment.runner.id.push_str(" successor"); assert!(!rule.matches(&changed));
        assert_eq!(selected(&changed).unwrap(), (Accessibility::WindowsDefault, None));
    }
}

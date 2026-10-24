//! Update impact follows exact setup membership, never a vendor name or class.
use super::*;

pub(super) fn compose(snapshot: &ui::Snapshot) -> (Vec<ui::ChangePreview>, ui::PackageUpdatePreview) {
    let mut ids = std::collections::BTreeSet::new();
    ids.extend(snapshot.environments.iter().map(|e| e.id.clone()));
    ids.extend(snapshot.environment_installers.iter().map(|e| e.environment.clone()));
    ids.extend(snapshot.installer_setups.iter().filter_map(|s| s.environment.clone()));
    ids.extend(snapshot.products.iter().filter(|p| p.disposition != "uninstalled").map(|p| p.environment.clone()));
    ids.retain(|id| !snapshot.deleted_setups.iter().any(|setup| &setup.environment == id));
    let setups: Vec<_> = ids.into_iter().map(|id| {
        let environment = snapshot.environments.iter().find(|e| e.id == id);
        let installed = snapshot.installer_setups.iter().find(|s| s.environment.as_ref() == Some(&id));
        let target = snapshot.environment_installers.iter().find(|e| e.environment == id);
        let products: Vec<_> = snapshot.products.iter().filter(|p| p.environment == id && p.disposition != "uninstalled").collect();
        let name = target.map(|s| s.name.clone()).or_else(|| installed.map(|s| s.name.clone()))
            .or_else(|| environment.map(|e| e.family.clone())).filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "Installed setup".into());
        let runtime = target.and_then(|t| t.runtime.as_ref()).map(|r| r.runner.clone())
            .or_else(|| environment.map(|e| e.runner.clone()));
        ui::SetupSummary { environment: id, name,
            products: products.iter().map(|p| ui::ProductIdentity::from(*p)).collect(),
            product_names: products.iter().map(|p| p.name.clone()).collect(),
            runtime: runtime.map(ui::Observation::Observed)
                .unwrap_or_else(|| ui::Observation::Unavailable("Current runtime was not read".into())) }
    }).collect();
    let changes = snapshot.environment_installers.iter().flat_map(|target| target.runtime.iter())
        .flat_map(|runtime| runtime.choices.iter()).filter_map(|offer| {
            let ui::Action::EnvironmentRuntimeSelect { environment, .. } = &offer.action else { return None; };
            let (affected, unchanged): (Vec<_>, Vec<_>) = setups.iter().cloned().partition(|s| &s.environment == environment);
            if affected.len() != 1 { return None; }
            Some(ui::ChangePreview { action: offer.action.clone(), affected, unchanged,
                consequence: "Close the plug-ins and applications in the affected setup before continuing. Its runtime changes and its plug-ins are checked again. Other setups keep their current runtime and can keep playing. Returning to the previous runtime does not undo changes made by vendor software.".into() })
        }).collect();
    (changes, ui::PackageUpdatePreview { preserved_setups: setups })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_preview_binds_the_whole_setup_and_preserves_same_vendor_siblings() {
        let mut snapshot: ui::Snapshot = serde_json::from_str(include_str!("../../../../../manager-ui/examples/library-preview.json")).unwrap();
        let mut product = snapshot.products[0].clone();
        snapshot.products.clear(); snapshot.environments.clear(); snapshot.environment_installers.clear();
        snapshot.installer_setups.clear();
        product.environment = "ab".repeat(16); product.disposition = "ready".into();
        let mut sibling = product.clone(); sibling.environment = "cd".repeat(16);
        snapshot.products = vec![product.clone(), sibling];
        let action = ui::Action::EnvironmentRuntimeSelect { environment: product.environment.clone(),
            runner: "new-runtime".into(), expected_environment: "ef".repeat(32) };
        snapshot.environment_installers.push(ui::EnvironmentInstaller {
            environment: product.environment.clone(), name: "Affected setup".into(), affected: vec![],
            consequence: String::new(), dependency_help: vec![], choices: vec![], operation: None, installer: None,
            result: json!({}), actions: vec![], runtime: Some(ui::EnvironmentRuntime { runner: "old-runtime".into(),
                revision: 3, affected: vec![], consequence: String::new(), choices: vec![super::super::action("Choose runtime", action.clone(), None)] }) });
        let (changes, package) = compose(&snapshot);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].action, action);
        assert_eq!(changes[0].affected[0].products, vec![ui::ProductIdentity::from(&product)]);
        assert_eq!(changes[0].unchanged.len(), 1);
        assert_ne!(changes[0].unchanged[0].environment, product.environment);
        assert!(matches!(changes[0].unchanged[0].runtime, ui::Observation::Unavailable(_)));
        assert_eq!(package.preserved_setups.len(), 2, "package updates preserve both setups");
        assert_eq!(package.preserved_setups[0].runtime, ui::Observation::Observed("old-runtime".into()));
        snapshot.environment_installers[0].runtime.as_mut().unwrap().choices.clear();
        assert!(compose(&snapshot).0.is_empty(), "withdrawn choices have no stale preview");
        snapshot.deleted_setups.push(ui::DeletedSetup {installer:"removed".into(),
            environment:product.environment.clone(),name:"Deleted setup".into(),state:"deleted".into(),
            deleted_at:Some(1),plugins:vec![],actions:vec![]});
        let (_, package) = compose(&snapshot);
        assert_eq!(package.preserved_setups.len(), 1);
        assert_ne!(package.preserved_setups[0].environment, product.environment,
            "retained deletion history is not an installed setup");
    }
}

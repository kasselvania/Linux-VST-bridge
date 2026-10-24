//! Compact preset facts and owner actions; deliberately no preset browser.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct Presets {
    pub facts: Vec<Fact>,
    pub explanation: String,
    pub recovery: Option<String>,
    pub actions: OfferRow,
}
pub fn build(product: &Product, busy: Option<&str>, pending: bool) -> Presets {
    let view = serde_json::from_value::<crate::model::PresetFiles>(
        product.details["preset_files"].clone(),
    )
    .ok();
    let facts = vec![
        Fact {
            label: "Kept in",
            value: view
                .as_ref()
                .and_then(|v| v.user_path.as_ref())
                .map(|p| p.display().to_string()),
            missing: "User preset location has not been checked",
        },
        Fact {
            label: "Your files",
            value: view
                .as_ref()
                .and_then(|v| v.files.zip(v.bytes))
                .map(|(count, bytes)| format!("{count} files · {bytes} bytes")),
            missing: "Not counted",
        },
        Fact {
            label: "On Linux",
            value: view
                .as_ref()
                .and_then(|v| v.linux_path.as_ref())
                .map(|p| p.display().to_string()),
            missing: "No linked folder",
        },
        Fact {
            label: "Last backup",
            value: view
                .as_ref()
                .and_then(|v| v.backup.as_ref())
                .map(|_| "Completed backup retained outside the setup".into()),
            missing: "No completed backup",
        },
    ];
    let actions: Vec<_> = product
        .actions
        .iter()
        .filter(|a| matches!(a.action, Action::PresetFiles { .. }))
        .cloned()
        .collect();
    Presets {facts,explanation:format!("{} User presets stay out of support reports. Factory content stays with the vendor.",view.as_ref().map(|v|v.explanation.as_str()).unwrap_or("User preset location is not known for this build.")),
        recovery:view.as_ref().filter(|v|v.missing>0 || v.conflicts>0).map(|v|format!("{} missing files can be recovered. {} existing files differ and will stay unchanged.",v.missing,v.conflicts)),
        actions:offers::row(&actions,Importance::Secondary,busy,pending)}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_unknown_and_recovery_pages_show_only_owner_facts_and_offers() {
        let mut snapshot = crate::samples::base();
        let product = &mut snapshot.products[0];
        assert!(build(product, None, false).actions.offers.is_empty());
        let action = Action::PresetFiles {
            product: crate::model::PresetProduct {
                environment: product.environment.clone(),
                module_sha256: product.module_sha256.clone(),
                class_id: product.class_id.clone(),
            },
            expected_root: "root".into(),
            command: crate::model::PresetCommand::Restore {
                backup: "backup".into(),
                missing_digest: "missing".into(),
                confirmed: false,
            },
        };
        product.actions.push(AvailableAction {
            label: "Restore missing presets…".into(),
            action: action.clone(),
            disabled_reason: None,
        });
        product.details["preset_files"] = serde_json::to_value(crate::model::PresetFiles {
            location: crate::model::PresetLocation::Known,
            explanation: "Qualified user folder".into(),
            user_path: Some("/vendor/User Presets".into()),
            linux_path: Some("/music/User Presets".into()),
            files: Some(2),
            bytes: Some(128),
            backup: Some("backup".into()),
            missing: 1,
            conflicts: 1,
            actions: vec![],
        })
        .unwrap();
        let page = build(product, None, false);
        assert_eq!(page.actions.offers.len(), 1);
        assert_eq!(page.actions.offers[0].intent, action);
        assert!(page.recovery.unwrap().contains("1 existing files differ"));
        assert!(page
            .facts
            .iter()
            .any(|f| f.value.as_deref() == Some("2 files · 128 bytes")));
        assert!(crate::forms::needs_confirmation(&action));
        assert!(matches!(
            crate::forms::confirmed(action),
            Action::PresetFiles {
                command: crate::model::PresetCommand::Restore {
                    confirmed: true,
                    ..
                },
                ..
            }
        ));
    }
}

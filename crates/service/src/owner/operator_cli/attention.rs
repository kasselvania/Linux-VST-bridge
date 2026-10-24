//! Current setup and attention composition. The window formats these facts;
//! it does not reconcile retained installation history with current products.
use super::*;
use ui::{Attention, AttentionTarget, ConnectionDisposition as Connection, CurrentPresentation,
    InstallationPhase as Phase, InstallationPluginStage as Stage, Observation, ProductIdentity,
    SetupDisposition as Disposition, SetupState};

pub(super) fn compose(m: &Manager, sw: &Software, snapshot: &mut ui::Snapshot) -> Result<()> {
    let selections = if snapshot.installer_setups.iter().any(|s| s.journey.as_ref().is_some_and(|j| !j.plugins.is_empty())) {
        linux_vst_bridge::preparation::selections(m, &sw.host, &sw.source_sha256)?
    } else { vec![] };
    let bindings = selections.iter().map(|selection| Ok((selection.id()?, ProductIdentity {
        environment: selection.environment.id.clone(), module_sha256: selection.module.sha256.clone(),
        class_id: selection.class.id.clone(),
    }))).collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    snapshot.presentation = Observation::Observed(project(snapshot, &bindings));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ui::Snapshot {
        let mut snapshot: ui::Snapshot = serde_json::from_str(include_str!("../../../../../manager-ui/examples/library-preview.json")).unwrap();
        snapshot.products.truncate(1);
        let product = &mut snapshot.products[0];
        product.availability = Some(ui::NativeAvailability { available: true, reason: None });
        product.compatibility = None;
        let setup: ui::InstallerSetup = serde_json::from_value(json!({
            "installer":"ab".repeat(32),"name":"Earlier setup","label_source":"operator_named",
            "byte_size":1,"format":"pe_executable","imported_at":1,"phase":"setup_needs_attention",
            "journey":null,"status":"Review retained history","environment":product.environment,
            "compatibility":null,"discovered":[{"environment":product.environment,"module_sha256":product.module_sha256,
                "class_id":product.class_id,"name":product.name}],"primary":null,"secondary":[],
            "rename":{"label":"Rename","action":{"kind":"installer_rename","installer":"ab".repeat(32),"label":""},"disabled_reason":null},"history":[]
        })).unwrap();
        snapshot.onboarding = vec![ui::Onboarding { installer: setup.installer.clone(), name: setup.name.clone(),
            byte_size: 1, format: setup.format.clone(), environment: setup.environment.clone(), state: "needs_attention".into(),
            failure: None, required_human_action: "Review retained history".into(), actions: vec![],
            details: json!({"managed_product":true,"scan":{},"installation":{"state":"completed","cleanup_confirmed":true,
                "owned_live":0,"error":null,"failure":null,"transaction":{"outcome":"partial_installation","first_failure":null}}}) }];
        snapshot.installer_setups = vec![setup];
        snapshot.system.service = "active".into(); snapshot.system.cleanup_unconfirmed = false; snapshot.system.pending_transactions = 0;
        snapshot
    }
    #[test]
    fn exact_current_products_settle_history_without_rewriting_it() {
        let snapshot = fixture();
        let before = serde_json::to_value(&snapshot.onboarding).unwrap();
        assert_eq!(project(&snapshot, &Default::default()).setups[0].disposition, Disposition::SettledHistory);
        for case in 0..12 {
            let mut changed = snapshot.clone();
            match case {
                0 => changed.installer_setups[0].environment = Some("different-environment".into()),
                1 => changed.products[0].module_sha256 = "changed-module".into(),
                2 => changed.products[0].environment = "different-environment".into(),
                3 => changed.products.clear(),
                4 => changed.products[0].availability = Some(ui::NativeAvailability { available: false, reason: Some("Current connection failed".into()) }),
                5 => changed.products[0].availability = None,
                6 => { let mut missing = changed.installer_setups[0].discovered[0].clone();
                    missing.module_sha256 = "missing-module".into(); changed.installer_setups[0].discovered.push(missing); },
                7 | 8 => changed.installer_setups[0].journey = Some(ui::InstallationJourney { daw: ui::DawChoice::Bitwig,
                    phase: if case == 7 { Phase::VendorRunning } else { Phase::NeedsAttention }, plugins: vec![], failure: None }),
                9 => changed.installer_setups[0].primary = Some(action("Continue", ui::Action::SoftwareInstallResume { onboarding: "exact-owner".into() }, None)),
                10 => changed.onboarding[0].actions.push(action("Repair", ui::Action::SoftwareInstallResume { onboarding: "exact-owner".into() }, None)),
                _ => changed.onboarding[0].details["installation"]["transaction"]["first_failure"] = json!("retained failure"),
            }
            assert_ne!(project(&changed, &Default::default()).setups[0].disposition, Disposition::SettledHistory, "case {case}");
        }
        assert_eq!(serde_json::to_value(&snapshot.onboarding).unwrap(), before);
    }
    #[test]
    fn one_failed_connection_names_and_routes_only_its_exact_build() {
        let mut snapshot = fixture();
        let mut healthy = snapshot.products[0].clone(); healthy.name = "Healthy sibling".into();
        healthy.module_sha256 = "cd".repeat(32);
        let failed = &mut snapshot.products[0]; failed.name = "Affected instrument".into();
        failed.availability = Some(ui::NativeAvailability { available: false, reason: Some("Publication failed".into()) });
        let key = ProductIdentity::from(&*failed);
        let class = failed.class_id.clone();
        snapshot.products.push(healthy);
        snapshot.installer_setups[0].label_source = "generated".into();
        snapshot.installer_setups[0].journey = Some(ui::InstallationJourney { daw: ui::DawChoice::Reaper,
            phase: Phase::NeedsAttention, failure: None, plugins: vec![ui::InstallationPlugin {
                selection: "exact-selection".into(), class_id: class, name: "Affected instrument".into(), stage: Stage::NeedsAttention,
                failure: Some(ui::InstallationFailure { stage: ui::InstallationFailureStage::Publication,
                    reason: "Publication failed".into(), code: "fixture".into(), retryable: true }) }] });
        let bindings = [("exact-selection".into(), key.clone())].into_iter().collect();
        let view = project(&snapshot, &bindings);
        assert_eq!(view.attention.len(), 1);
        assert_eq!(view.attention[0].target, AttentionTarget::Product { product: key.clone() });
        assert!(view.attention[0].title.starts_with("Affected instrument"));
        assert!(!view.attention[0].title.contains("other"));
        assert_eq!(view.setups[0].connections[0].product, Observation::Observed(key));
        assert_eq!(view.setups[0].connections[0].disposition, Observation::Observed(Connection::NeedsAttention));
        // A same-class sibling is not evidence when the original selection is gone.
        let unknown = project(&snapshot, &Default::default());
        assert!(matches!(unknown.setups[0].connections[0].product, Observation::Unavailable(_)));
        assert!(matches!(unknown.attention[0].target, AttentionTarget::Setup { .. }));
    }
    #[test]
    fn current_progress_requires_exact_selection_and_never_masks_failure() {
        let mut snapshot = fixture();
        let product = &mut snapshot.products[0];
        product.availability = Some(ui::NativeAvailability { available: false, reason: Some("Not connected".into()) });
        product.disposition = "installed_unqualified".into(); product.details = json!({});
        let key = ProductIdentity::from(&*product);
        let journey = ui::InstallationJourney { daw: ui::DawChoice::Reaper, phase: Phase::PreparingPlugins,
            failure: None, plugins: vec![ui::InstallationPlugin { selection: "exact-selection".into(), class_id: key.class_id.clone(),
                name: product.name.clone(), stage: Stage::Inspecting, failure: None }] };
        snapshot.installer_setups[0].journey = Some(journey.clone());
        snapshot.operation = Some(json!({"operation":"exact-operation","onboarding":key.environment,
            "state":"running","journey":journey}));
        let bindings: std::collections::BTreeMap<_,_> = [("exact-selection".into(), key.clone())].into_iter().collect();
        assert_eq!(progress(&snapshot, &bindings), vec![ui::ProductProgress { product: key.clone(), stage: ui::ConnectionProgress::Inspecting }]);
        for case in 0..14 {
            let mut changed = snapshot.clone(); let mut selected = bindings.clone();
            match case {
                0 => changed.products[0].availability.as_mut().unwrap().available = true,
                1 => changed.products[0].disposition = "quarantined".into(),
                2 => changed.products[0].details["current"] = json!(false),
                3 => changed.products[0].details["refusal"] = json!("Actual refusal"),
                4 => changed.products[0].environment = "different".into(),
                5 => changed.installer_setups[0].discovered.clear(),
                6 => selected.get_mut("exact-selection").unwrap().module_sha256 = "another-build".into(),
                7 => changed.operation.as_mut().unwrap()["journey"]["plugins"][0]["selection"] = json!("another-selection"),
                8 => { let plugin = changed.operation.as_ref().unwrap()["journey"]["plugins"][0].clone();
                    changed.operation.as_mut().unwrap()["journey"]["plugins"].as_array_mut().unwrap().push(plugin); },
                9 => changed.operation.as_mut().unwrap()["state"] = json!("completed"),
                10 => changed.operation.as_mut().unwrap()["journey"]["phase"] = json!("needs_attention"),
                11 => changed.operation.as_mut().unwrap()["journey"]["plugins"][0]["stage"] = json!("needs_attention"),
                12 => changed.operation.as_mut().unwrap()["journey"]["plugins"][0]["class_id"] = json!("other-class"),
                _ => { changed.products[0].disposition = "experimental".into();
                    changed.products[0].details = json!({"refusal":"Old authority", "automatic_revalidation": {
                        "operation":"other-operation", "predecessor":{"id":"retained", "sha256":"ab".repeat(32)}}}); },
            }
            assert!(progress(&changed, &selected).is_empty(), "case {case}");
        }
        snapshot.products[0].disposition = "experimental".into();
        snapshot.products[0].details = json!({"refusal":"Old authority", "automatic_revalidation": {
            "operation":"exact-operation", "predecessor":{"id":"retained", "sha256":"ab".repeat(32)}}});
        assert_eq!(progress(&snapshot, &bindings).len(), 1, "exact revalidation still has visible progress");
        snapshot.operation.as_mut().unwrap()["journey"]["plugins"][0]["stage"] = json!("needs_attention");
        assert!(progress(&snapshot, &bindings).is_empty(), "actual failed recheck must stay visible");
    }

    #[test]
    fn owned_pause_is_not_service_failure_and_cannot_hide_independent_problems() {
        let mut snapshot = fixture();
        snapshot.system.service = "capacity unavailable".into(); snapshot.system.cleanup_unconfirmed = true;
        snapshot.operation = Some(json!({"operation":"exact", "state":"running", "journey":{"phase":"preparing_plugins","failure":null},
            "owned_service_pause":{"schema":1,"owner_operation":"child","automatic_operation":"exact"}}));
        assert!(service_problem(&snapshot).is_none());
        for case in 0..7 {
            let mut changed = snapshot.clone();
            match case {
                0 => changed.operation.as_mut().unwrap().as_object_mut().unwrap().remove("owned_service_pause").map(|_| ()).unwrap(),
                1 => changed.operation.as_mut().unwrap()["owned_service_pause"]["automatic_operation"] = json!("other"),
                2 => changed.operation.as_mut().unwrap()["state"] = json!("refused"),
                3 => changed.operation.as_mut().unwrap()["journey"]["phase"] = json!("needs_attention"),
                4 => changed.system.pending_transactions = 1,
                5 => changed.system.stale_transports = 1,
                _ => changed.system.service = "active".into(), // observed cleanup failure
            }
            assert!(service_problem(&changed).is_some(), "case {case}");
        }
    }

}

fn priority(setup: &ui::InstallerSetup) -> u8 {
    use ui::SetupPhase as S;
    match setup.journey.as_ref().map(|journey| journey.phase) {
        Some(Phase::VendorRunning) => 0,
        Some(Phase::NeedsAttention | Phase::AwaitingApplication) => 2,
        Some(Phase::PreparingEnvironment | Phase::Discovering | Phase::PreparingPlugins) => 3,
        Some(Phase::Available) => 6,
        None => match setup.phase {
            S::InstallerRunning => 0, S::CleanupUnconfirmed => 1, S::SetupNeedsAttention => 2,
            S::EnvironmentReady => 3, S::InstallerRetired | S::ScanReady => 4,
            S::Imported => 5, S::DiscoveryComplete => 6,
        },
    }
}
fn name(setup: &ui::InstallerSetup, snapshot: &ui::Snapshot) -> String {
    if setup.label_source != "generated" { return setup.name.clone(); }
    let mut names = Vec::new();
    for found in &setup.discovered {
        if snapshot.products.iter().any(|product| product.environment == found.environment
            && product.module_sha256 == found.module_sha256
            && product.class_id.eq_ignore_ascii_case(&found.class_id)) && !names.contains(&found.name) {
            names.push(found.name.clone());
        }
    }
    match names.as_slice() { [] => setup.name.clone(), [one] => one.clone(),
        [first, rest @ ..] => format!("{first} and {} other plug-ins", rest.len()) }
}
fn connection(product: &ui::Product) -> Observation<Connection> {
    let Some(availability) = &product.availability else {
        return Observation::Unavailable("Current DAW connection status was not observed.".into());
    };
    Observation::Observed(if availability.available { Connection::Available }
        else if matches!(product.disposition.as_str(), "removed_from_daw" | "uninstalled") { Connection::Removed }
        else if availability.reason.is_some() { Connection::NeedsAttention }
        else { Connection::NotAvailable })
}
fn phase_title(setup: &ui::InstallerSetup) -> &'static str {
    if let Some(journey) = &setup.journey {
        return match journey.phase {
            Phase::PreparingEnvironment => "Getting ready to install",
            Phase::VendorRunning => "Complete the vendor's requests",
            Phase::Discovering => "Finding installed plug-ins",
            Phase::PreparingPlugins => "Connecting plug-ins to your DAW",
            Phase::AwaitingApplication => "Continue in the installed app",
            Phase::Available if journey.plugins.is_empty() => "Application session complete",
            Phase::Available => "DAW connection complete",
            Phase::NeedsAttention => "Installation or DAW connection needs attention",
        };
    }
    use ui::SetupPhase as S;
    match setup.phase {
        S::Imported => "Ready to set up", S::EnvironmentReady => "Ready to install",
        S::InstallerRunning => "Vendor installer is open", S::SetupNeedsAttention => "Installation needs your attention",
        S::InstallerRetired | S::ScanReady => "Ready to find installed plug-ins",
        S::DiscoveryComplete => "Plug-ins found", S::CleanupUnconfirmed => "Could not confirm the installer closed",
    }
}
fn setup_state(setup: &ui::InstallerSetup, snapshot: &ui::Snapshot,
    bindings: &std::collections::BTreeMap<String, ProductIdentity>) -> SetupState {
    let no_work = setup.primary.is_none() && setup.secondary.iter().all(|offer|
        matches!(offer.action, ui::Action::SetupRemoveFromDaw {..} | ui::Action::SetupDelete {..}));
    let attempts: Vec<_> = snapshot.onboarding.iter().filter(|attempt|
        attempt.installer == setup.installer && attempt.environment == setup.environment).collect();
    let retired_installation = |attempt: &&ui::Onboarding| {
        let installed = &attempt.details["installation"];
        attempt.state == "needs_attention" && attempt.failure.is_none()
            && installed["state"] == "completed" && installed["cleanup_confirmed"] == true
            && installed["owned_live"] == 0 && installed["error"].is_null() && installed["failure"].is_null()
    };
    let partial = setup.phase == ui::SetupPhase::SetupNeedsAttention && attempts.iter().any(|attempt|
        retired_installation(attempt) && attempt.details["installation"]["transaction"]["outcome"] == "partial_installation");
    let records_need_refresh = setup.journey.is_none() && setup.phase == ui::SetupPhase::SetupNeedsAttention
        && no_work && setup.environment.is_some() && !setup.discovered.is_empty()
        && attempts.iter().any(|attempt| retired_installation(attempt) && attempt.actions.is_empty()
            && attempt.details["managed_product"] == true && attempt.details["scan"].is_object()
            && matches!(attempt.details["installation"]["transaction"]["outcome"].as_str(), None | Some("installed")));
    let settled = setup.journey.is_none() && no_work && (records_need_refresh || partial)
        && !setup.discovered.is_empty() && !attempts.iter().any(|attempt| attempt.failure.is_some()
            || !attempt.actions.is_empty() || !attempt.details["installation"]["error"].is_null()
            || !attempt.details["installation"]["failure"].is_null()
            || !attempt.details["installation"]["transaction"]["first_failure"].is_null())
        && setup.discovered.iter().all(|found| setup.environment.as_ref() == Some(&found.environment)
            && snapshot.products.iter().any(|product| product.environment == found.environment
                && product.module_sha256 == found.module_sha256 && product.class_id.eq_ignore_ascii_case(&found.class_id)
                && product.availability.as_ref().is_some_and(|a| a.available)));
    let complete = setup.journey.as_ref().map_or(setup.phase == ui::SetupPhase::DiscoveryComplete,
        |journey| journey.phase == Phase::Available);
    let disposition = if settled { Disposition::SettledHistory } else if complete { Disposition::Complete }
        else if records_need_refresh { Disposition::RecordsNeedRefresh } else if partial { Disposition::PartialInstallation }
        else if matches!(setup.phase, ui::SetupPhase::SetupNeedsAttention | ui::SetupPhase::CleanupUnconfirmed)
            || setup.journey.as_ref().is_some_and(|j| j.phase == Phase::NeedsAttention)
            || (setup.phase == ui::SetupPhase::Imported && setup.primary.is_none()) { Disposition::NeedsAttention }
        else { Disposition::InProgress };
    let connections: Vec<_> = setup.journey.iter().flat_map(|j| &j.plugins).map(|plugin| {
        let key = bindings.get(&plugin.selection).filter(|key| setup.environment.as_ref() == Some(&key.environment)
            && key.class_id.eq_ignore_ascii_case(&plugin.class_id));
        let product = key.and_then(|key| snapshot.products.iter().find(|product| key.matches(product)));
        ui::JourneyConnection { selection: plugin.selection.clone(),
            product: key.cloned().map(Observation::Observed).unwrap_or_else(||
                Observation::Unavailable("The exact retained selection is unavailable.".into())),
            disposition: product.map(connection).unwrap_or_else(||
                Observation::Unavailable("The exact installed build has no current connection observation.".into())),
            reason: product.and_then(|p| p.availability.as_ref()).and_then(|a| a.reason.clone()) }
    }).collect();
    let mut headline = match disposition {
        Disposition::SettledHistory => "Plug-ins available in your DAW",
        Disposition::RecordsNeedRefresh => "Installation records need attention",
        Disposition::PartialInstallation => "Review earlier installation",
        _ => phase_title(setup),
    }.to_owned();
    if complete && !connections.is_empty() {
        let available = connections.iter().filter(|c| c.disposition == Observation::Observed(Connection::Available)).count();
        headline = if connections.iter().any(|c| matches!(c.disposition, Observation::Unavailable(_))) {
            "Current DAW connection status is incomplete"
        } else if available == 0 { "Not in your DAW" }
            else if available < connections.len() { "Partly in your DAW" } else { "DAW connection complete" }.into();
    }
    let status = match disposition {
        Disposition::SettledHistory => format!("{} plug-ins available in your DAW.", setup.discovered.len()),
        Disposition::RecordsNeedRefresh => "Installation records need refreshing before making setup changes.".into(),
        Disposition::PartialInstallation => "A complete installation was not confirmed. Review the installation result before making setup changes. Plug-ins already available in your DAW remain available.".into(),
        _ => setup.status.clone(),
    };
    SetupState { installer: setup.installer.clone(), environment: setup.environment.clone(), name: name(setup, snapshot),
        disposition, headline, status, connections }
}

fn project(snapshot: &ui::Snapshot, bindings: &std::collections::BTreeMap<String, ProductIdentity>) -> CurrentPresentation {
    let mut setups: Vec<_> = snapshot.installer_setups.iter().collect();
    setups.sort_by_key(|setup| priority(setup));
    let mut attention = Vec::new();
    if let Some(reason) = service_problem(snapshot) {
        attention.push(Attention { purpose: ui::AttentionPurpose::Service, target: AttentionTarget::Service, title: "Review bridge service status".into(),
            detail: reason.into(),
            action: snapshot.actions.iter().find(|offer| matches!(offer.action, ui::Action::TransactionReconcile {})).map(|a| a.action.clone()) });
    }
    let mut covered = std::collections::HashSet::new();
    let states = setups.into_iter().map(|setup| {
        let state = setup_state(setup, snapshot, bindings);
        if !matches!(state.disposition, Disposition::Complete | Disposition::SettledHistory) {
            let failed: Vec<_> = setup.journey.iter().flat_map(|journey| &journey.plugins)
                .filter(|p| p.failure.is_some() || matches!(p.stage, Stage::NeedsAttention | Stage::ReplacementRequired)).collect();
            let exact = if failed.len() == 1 { state.connections.iter().find(|c| c.selection == failed[0].selection)
                .and_then(|c| c.product.observed()).cloned() } else { None };
            if let Some(key) = &exact { covered.insert(key.clone()); }
            let subject = match failed.as_slice() { [] => state.name.clone(), [one] => one.name.clone(),
                [first, rest @ ..] => format!("{} and {} other affected plug-ins", first.name, rest.len()) };
            attention.push(Attention { purpose: ui::AttentionPurpose::Installation, target: exact.map_or_else(|| AttentionTarget::Setup {
                installer: setup.installer.clone(), environment: setup.environment.clone() }, |product| AttentionTarget::Product { product }),
                title: format!("{subject} · {}", state.headline),
                detail: failed.first().and_then(|p| p.failure.as_ref()).map_or_else(|| state.status.clone(), |f| f.reason.clone()),
                action: setup.primary.as_ref().map(|offer| offer.action.clone()) });
        }
        state
    }).collect();
    for product in &snapshot.products {
        let key = ProductIdentity::from(product);
        if covered.contains(&key) || matches!(product.disposition.as_str(), "removed_from_daw" | "uninstalled") { continue; }
        let recovering = product.compatibility.as_ref().is_some_and(|workflow| matches!(workflow.phase,
            ui::CompatibilityPhase::AwaitingRetirement | ui::CompatibilityPhase::PublicationNeedsAttention));
        let refusal = explanation(&product.details["last_load"]["problem"]);
        if !recovering && refusal.is_none() && product.availability.as_ref().is_some_and(|a| a.available) { continue; }
        attention.push(Attention { purpose: if recovering { ui::AttentionPurpose::Recovery } else { ui::AttentionPurpose::Connection },
            target: AttentionTarget::Product { product: key },
            title: if recovering { format!("Recover {}", product.name) } else if refusal.is_some() { format!("Review the refused load of {}", product.name) }
                else { format!("Set up {} for your DAW", product.name) },
            detail: if recovering { product.compatibility.as_ref().unwrap().summary.clone() }
                else { refusal.or_else(|| product.availability.as_ref().and_then(|a| a.reason.clone()))
                    .unwrap_or_else(|| "Review this plug-in's current connection and next step.".into()) },
            action: None });
    }
    let available = snapshot.products.iter().filter(|product| product.availability.as_ref().is_some_and(|a| a.available)).count();
    let (change_previews, package_update) = super::update_preview::compose(snapshot);
    CurrentPresentation { setups: states, attention, progress: progress(snapshot, bindings), change_previews, package_update, settled_message: if available == 0 {
        "No plug-ins are currently available in your DAW".into()
    } else { format!("{available} plug-in{} available in your DAW", if available == 1 { "" } else { "s" }) } }
}

fn explanation(value: &Value) -> Option<String> {
    let what = value["what_happened"].as_str().filter(|text| !text.trim().is_empty())?;
    Some([Some(what), value["likely_cause"].as_str(), value["what_to_try"].as_str()]
        .into_iter().flatten().filter(|text| !text.trim().is_empty()).collect::<Vec<_>>().join(" "))
}

fn progress(snapshot: &ui::Snapshot, bindings: &std::collections::BTreeMap<String, ProductIdentity>) -> Vec<ui::ProductProgress> {
    use ui::ConnectionProgress as P;
    let Some(operation) = &snapshot.operation else { return vec![]; };
    if !matches!(operation["state"].as_str(), Some("waiting" | "running")) { return vec![]; }
    let Ok(journey) = serde_json::from_value::<ui::InstallationJourney>(operation["journey"].clone()) else { return vec![]; };
    if journey.phase != Phase::PreparingPlugins || journey.failure.is_some() { return vec![]; }
    let Some(environment) = operation["onboarding"].as_str() else { return vec![]; };
    let mut setups = snapshot.installer_setups.iter().filter(|setup|
        setup.environment.as_deref() == Some(environment) && setup.journey.is_some());
    let Some(setup) = setups.next() else { return vec![]; };
    if setups.next().is_some() { return vec![]; }
    snapshot.products.iter().filter_map(|product| {
        let key = ProductIdentity::from(product);
        let revalidation = &product.details["automatic_revalidation"];
        let rechecking = revalidation["operation"] == operation["operation"]
            && operation["operation"].as_str().is_some_and(|id| !id.is_empty())
            && revalidation["predecessor"]["id"].as_str().is_some_and(|id| !id.is_empty())
            && revalidation["predecessor"]["sha256"].as_str().is_some_and(|sha| valid_hex(sha, 64));
        if product.availability.as_ref().is_some_and(|a| a.available)
            || (!rechecking && (!matches!(product.disposition.as_str(), "installed_unqualified" | "prepared")
                || !product.details["refusal"].is_null()))
            || product.disposition == "quarantined" || product.details["current"] == false
            || product.compatibility.as_ref().is_some_and(|c| c.phase == ui::CompatibilityPhase::AwaitingRetirement) {
            return None;
        }
        if product.environment != environment || !setup.discovered.iter().any(|found|
            found.environment == key.environment && found.module_sha256 == key.module_sha256
                && found.class_id.eq_ignore_ascii_case(&key.class_id)) { return None; }
        let mut plugins = journey.plugins.iter().filter(|plugin| plugin.failure.is_none()
            && bindings.get(&plugin.selection).is_some_and(|selected| selected == &key)
            && plugin.class_id.eq_ignore_ascii_case(&key.class_id));
        let plugin = plugins.next()?;
        if plugins.next().is_some() { return None; }
        let stage = match plugin.stage { Stage::Pending => P::Waiting, Stage::Inspecting => P::Inspecting,
            Stage::Preparing => P::Preparing, Stage::Publishing => P::Publishing, _ => return None };
        Some(ui::ProductProgress { product: key, stage })
    }).collect()
}

fn service_problem(snapshot: &ui::Snapshot) -> Option<&'static str> {
    let system = &snapshot.system;
    // This derived marker is constructed by current_operation only after the
    // exact live operation/retirement checks; it is never accepted from disk.
    let owned_pause = snapshot.operation.as_ref().is_some_and(|operation| {
        let pause = &operation["owned_service_pause"];
        pause["schema"] == 1 && pause["automatic_operation"] == operation["operation"]
            && operation["operation"].as_str().is_some_and(|id| !id.is_empty())
            && pause["owner_operation"].as_str().is_some_and(|id| !id.is_empty())
            && matches!(operation["state"].as_str(), Some("waiting" | "running" | "vendor_running"))
            && matches!(operation["journey"]["phase"].as_str(), Some("preparing_environment" | "vendor_running" | "discovering" | "preparing_plugins"))
            && operation["journey"]["failure"].is_null()
    }) && system.pending_transactions == 0 && system.stale_transports == 0
        && (!system.capacity_available() || !system.cleanup_unconfirmed);
    if owned_pause { return None; }
    if !system.capacity_available() { Some("Current bridge service capacity is unavailable.") }
    else if system.pending_transactions > 0 { Some("A retained publication transaction needs review.") }
    else if system.cleanup_unconfirmed || system.stale_transports > 0 { Some("Previous session cleanup is unconfirmed.") }
    else { None }
}

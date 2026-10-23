//! Named sample states for previews, galleries and the screen-model tests.
//! Clearly marked synthetic records; nothing here contacts the manager. Built
//! for tests and with the `samples` feature only, so no release build of a
//! window carries them.
use crate::{model, window::Page};

/// The recorded preview records every sample state starts from.
pub fn base() -> model::Snapshot {
    serde_json::from_str(include_str!("../../../manager-ui/examples/library-preview.json")).expect("preview fixture")
}

/// Every named state the preview accepts, other than recorded snapshots.
pub const STATES: [&str; 39] = ["busy", "idle", "guided", "guided_new", "guided_pending", "pb0_ready",
    "pb0_action", "pb0_unsupported", "pb0_unknown", "populated", "available", "first_use", "setup_many",
    "install_running", "pending", "recoverable", "plugin_ready", "plugin_action", "plugin_details",
    "retained_installation", "plugin_history", "plugin_history_open", "current_loading", "stale",
    "unavailable", "cleanup", "shared", "install_progress", "install_paused", "install_attention",
    "install_app", "install_complete", "existing_setup", "setup_removal", "service_problem",
    "incidents", "diagnostics_full", "pb0_step", "plugin_technical"];

/// States that add records for the screens beyond the plug-in list: install
/// progress, apps, existing setups, removal, service problems and incidents.
const SCREEN_STATES: [&str; 11] = ["install_progress", "install_paused", "install_attention",
    "install_app", "install_complete", "existing_setup", "setup_removal", "service_problem",
    "incidents", "diagnostics_full", "pb0_step"];

/// The sample records for one page and named state (`None` is the preview default).
pub fn snapshot(page: Page, requested: Option<&str>) -> model::Snapshot {
    let state = requested.unwrap_or("populated");
    let mut snapshot = base();
    snapshot.bridge = model::BridgeBuild { installed: Some("0.12.0preview".into()),
        setup: model::SetupPackage::Nothing };
    if page == Page::Plugins
        && matches!(requested, Some("guided" | "guided_new" | "guided_pending")) {
        let mut beam = snapshot.products[0].clone();
        beam.name = "BEAM 2.3.1".into();
        beam.version = "2.3.1".into();
        beam.vendor = "Lunacy Audio".into();
        beam.role = "effect".into();
        beam.disposition = "experimental".into();
        beam.environment = "cd".repeat(16);
        beam.module_sha256 = "ef".repeat(32);
        beam.class_id = "01".repeat(16);
        beam.actions.clear();
        beam.compatibility = Some(model::CompatibilityWorkflow {
            phase: model::CompatibilityPhase::AvailableForTest,
            summary: "Available temporarily in Bitwig. Record only what you observed.".into(),
            established: vec!["Editor observed".into(), "Controls observed".into(),
                "Clean close observed".into()],
            remaining: vec!["Audio".into(), "State recall".into()],
            current_inspection: Some("aa".repeat(32)),
            current_candidate: Some("bb".repeat(32)),
            primary: Some(model::AvailableAction {
                label: "Record test result".into(),
                action: model::Action::CompatibilityResult {
                    candidate: "bb".repeat(32),
                    expected_current: model::PublicationIdentity {
                        id: "cc".repeat(16), sha256: "dd".repeat(32),
                    },
                    result: model::TestResultKind::Worked, passed: vec![],
                    failed_area: None, note: String::new(),
                }, disabled_reason: None,
            }), alternatives: vec![],
        });
        if requested == Some("guided_pending") {
            beam.compatibility = Some(model::CompatibilityWorkflow {
                phase: model::CompatibilityPhase::AwaitingRetirement,
                summary: "Problem recorded. The test configuration is still selected. Close the DAW or complete recovery, then finish this result.".into(),
                established: vec!["Loaded in the DAW".into()],
                remaining: vec!["Editor".into(), "Clean close".into()],
                current_inspection: Some("aa".repeat(32)),
                current_candidate: Some("bb".repeat(32)),
                primary: Some(model::AvailableAction {
                    label: "Finish recording test result".into(),
                    action: model::Action::CompatibilityFinishResult {
                        operation: "ee".repeat(16),
                    },
                    disabled_reason: Some("Previous instance cleanup is unconfirmed".into()),
                }), alternatives: vec![],
            });
            snapshot.system.cleanup_unconfirmed = true;
        }
        let mut bismuth = beam.clone();
        bismuth.name = "BEAM Bismuth".into();
        bismuth.class_id = "02".repeat(16);
        bismuth.disposition = "installed_unqualified".into();
        bismuth.availability = Some(model::NativeAvailability {available:false,reason:None});
        bismuth.compatibility = Some(model::CompatibilityWorkflow {
            phase: model::CompatibilityPhase::NotChecked,
            summary: "Installed · compatibility not checked".into(),
            established: vec![], remaining: vec![], current_inspection: None,
            current_candidate: None,
            primary: Some(model::AvailableAction {
                label: "Check compatibility".into(),
                action: model::Action::CompatibilityCheck {
                    selection: "ee".repeat(32), audio_layout: None,
                    recipe: "ff".repeat(32), predecessor: None,
                }, disabled_reason: None,
            }), alternatives: vec![],
        });
        snapshot.products = if requested == Some("guided_new") {
            vec![bismuth]
        } else { vec![beam, bismuth] };
    }
    if page == Page::Setup {
        let installer = "ab".repeat(32);
        let environment = "cd".repeat(16);
        let mut beam = snapshot.products[0].clone();
        beam.name = "BEAM".into();
        beam.vendor = "Lunacy Audio".into();
        beam.environment = environment.clone();
        beam.class_id = "01".repeat(16);
        beam.module_sha256 = "ef".repeat(32);
        beam.disposition = "installed_unqualified".into();
        beam.availability = Some(model::NativeAvailability {available:false,reason:None});
        beam.actions.clear();
        let mut sibling = beam.clone();
        sibling.name = "BEAM Bismuth".into();
        sibling.class_id = "02".repeat(16);
        snapshot.products.extend([beam, sibling]);
        snapshot.installer_setups.push(model::InstallerSetup {
            installer: installer.clone(), name: "Lunacy Audio".into(),
            label_source: "operator_named".into(), byte_size: 246_000_000,
            format: "pe_executable".into(), imported_at: 1,
            phase: model::SetupPhase::DiscoveryComplete,
            status: "2 plug-ins found · discovery did not publish them; see each plug-in's current status".into(),
            environment: Some(environment.clone()), compatibility: Some("Existing managed configuration".into()),
            discovered: snapshot.products.iter().filter(|p| p.environment == environment)
                .map(|p| model::DiscoveredProduct { environment: p.environment.clone(),
                    module_sha256: p.module_sha256.clone(), class_id: p.class_id.clone(),
                    name: p.name.clone() }).collect(),
            primary: None, secondary: vec![],
            rename: model::AvailableAction { label: "Rename".into(),
                action: model::Action::InstallerRename { installer: installer.clone(), label: String::new() },
                disabled_reason: None },
            journey:None,history: vec![model::SetupHistoryRef { environment: Some(environment.clone()),
                state: "installed_unqualified".into() }],
        });
        snapshot.onboarding.push(model::Onboarding { failure: None, installer,
            name: "Windows installer".into(), byte_size: 246_000_000,
            format: "pe_executable".into(), environment: Some(environment),
            state: "installed_unqualified".into(),
            required_human_action: "Review discovery".into(),
            details: serde_json::json!({"installation":{"state":"completed","cleanup_confirmed":true},
                "scan":{"id":"synthetic-scan","modules":2}}), actions: vec![] });
    }
    snapshot.system.dsp = 1;
    snapshot.active_sessions = vec![
        serde_json::json!({"session":"11".repeat(16),"class_id":"fixture-fragments",
            "state":"active","terminal":null,"recent":false}),
        serde_json::json!({"session":"22".repeat(16),"class_id":"fixture-kontakt",
            "state":"failed","terminal":"editor_controller_failed","recent":true,
            "cleanup_confirmed":true,"transport_retired":true,"observed_at":1}),
    ];
    snapshot.operation =
        Some(serde_json::json!({"operation":"preview-operation","state":"running"}));
    match requested.unwrap_or("busy") {
        "busy" => {}
        "idle" | "guided" | "guided_new" | "guided_pending"
        | "pb0_ready" | "pb0_action" | "pb0_unsupported" | "pb0_unknown"
        | "populated" | "available" | "first_use" | "setup_many" | "install_running"
        | "pending" | "recoverable" | "plugin_ready" | "plugin_action" | "plugin_details"
        | "retained_installation" | "plugin_history" | "plugin_history_open" | "current_loading" | "stale"
        | "plugin_technical" => {
            snapshot.system.dsp = 0;
            snapshot.active_sessions.retain(|row| row["recent"] == true);
            snapshot.operation = None;
        }
        state if SCREEN_STATES.contains(&state) => {
            snapshot.system.dsp = 0;
            snapshot.active_sessions.retain(|row| row["recent"] == true);
            snapshot.operation = None;
            screen_records(&mut snapshot, state);
        }
        "unavailable" => {
            snapshot.system.service = "capacity unavailable".into();
            snapshot.system.cleanup_unconfirmed = true; // canonical unavailable readback is conservative
            snapshot.system.ceiling = 0;
            snapshot.system.dsp = 0;
        }
        "cleanup" => snapshot.system.cleanup_unconfirmed = true,
        "shared" => {
            let mut other_build = snapshot.products[1].clone();
            other_build.name = "Efx FRAGMENTS (other build)".into();
            other_build.module_sha256 = "fixture-fragments-other-build".into();
            other_build.environment = "fixture-arturia-other".into();
            snapshot.products.push(other_build);
        }
        "recorded_current" | "recorded_expanded" => {},
        other => panic!("unknown state: {other}"),
    }
    if state == "first_use" {
        snapshot.products.clear();
        snapshot.environments.clear();
        snapshot.vendor_applications.clear();
        snapshot.installer_setups.clear();
        snapshot.onboarding.clear();
        snapshot.active_sessions.clear();
        snapshot.actions = vec![model::AvailableAction {label:"Download compatibility runtime".into(),
            action:model::Action::RuntimeInstall {},disabled_reason:None}];
    }
    if state == "available" {
        for product in &mut snapshot.products {
            product.disposition = "ready".into();
            product.availability = Some(model::NativeAvailability {available:true,reason:None});
            product.details = serde_json::json!({});
        }
    }
    if state == "retained_installation" {
        snapshot.products.truncate(1);
        let product=&mut snapshot.products[0];product.disposition="ready".into();
        let installer="af".repeat(32);
        snapshot.installer_setups=vec![model::InstallerSetup {
            installer:installer.clone(),name:"Unknown Windows installer · ID afafafaf…afaf".into(),
            label_source:"generated".into(),byte_size:1_000_000,format:"pe_executable".into(),imported_at:1,
            phase:model::SetupPhase::SetupNeedsAttention,
            status:"Installation retained; use the exact managed-environment refresh before preparing or replacing a publication".into(),
            environment:Some(product.environment.clone()),compatibility:None,
            discovered:vec![model::DiscoveredProduct {environment:product.environment.clone(),
                module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone(),name:product.name.clone()}],
            primary:None,secondary:vec![],rename:model::AvailableAction {label:"Rename".into(),
                action:model::Action::InstallerRename {installer:installer.clone(),label:String::new()},disabled_reason:None},journey:None,history:vec![],
        }];
        snapshot.onboarding=vec![model::Onboarding {failure:None,installer,name:"Retained installation".into(),
            byte_size:1_000_000,format:"pe_executable".into(),environment:Some(product.environment.clone()),
            state:"needs_attention".into(),required_human_action:snapshot.installer_setups[0].status.clone(),actions:vec![],
            details:serde_json::json!({"managed_product":true,"scan":{},"installation":{
                "state":"completed","cleanup_confirmed":true,"owned_live":0}})}];
    }
    if matches!(state,"setup_many" | "install_running" | "recoverable") {
        for index in 1..=5 {
            let installer = format!("{index:064x}");
            let product = &snapshot.products[(index - 1) % snapshot.products.len()];
            snapshot.installer_setups.push(model::InstallerSetup {
                installer:installer.clone(),name:format!("Example Audio installer {index}"),
                label_source:"operator_named".into(),byte_size:50_000_000,format:"pe_executable".into(),
                imported_at:index as u64,phase:model::SetupPhase::DiscoveryComplete,
                status:"Installation finished and plug-ins discovered.".into(),
                environment:Some(product.environment.clone()),compatibility:None,
                discovered:vec![model::DiscoveredProduct {environment:product.environment.clone(),
                    module_sha256:product.module_sha256.clone(),class_id:product.class_id.clone(),name:product.name.clone()}],
                primary:None,secondary:vec![],rename:model::AvailableAction {label:"Rename".into(),
                    action:model::Action::InstallerRename {installer,label:String::new()},disabled_reason:None},journey:None,history:vec![],
            });
        }
        if matches!(state,"install_running" | "recoverable") {
            let installer="ac".repeat(32);let onboarding="ba".repeat(16);let operation="ca".repeat(16);
            let running=state == "install_running";
            let mut dormant=snapshot.installer_setups[0].clone();
            dormant.installer="ad".repeat(32);dormant.name="Older installer added for later".into();
            dormant.phase=model::SetupPhase::Imported;dormant.status="Ready to start setup when needed.".into();
            dormant.discovered.clear();
            snapshot.installer_setups.insert(0,dormant);
            snapshot.installer_setups.insert(1,model::InstallerSetup {
                installer:installer.clone(),name:"Example Synth".into(),label_source:"operator_named".into(),
                byte_size:50_000_000,format:"pe_executable".into(),imported_at:6,
                phase:if running {model::SetupPhase::InstallerRunning} else {model::SetupPhase::SetupNeedsAttention},
                status:if running {"Complete installation and any vendor sign-in in the open installer."}
                    else {"The installer exited with an error. Installed files may remain; find them before deciding whether to retry."}.into(),
                environment:Some(onboarding.clone()),compatibility:None,discovered:vec![],
                primary:Some(model::AvailableAction {label:if running {"Show vendor installer"} else {"Find installed plug-ins"}.into(),
                    action:if running {model::Action::InstallerFocus {onboarding:onboarding.clone(),operation:operation.clone()}}
                        else {model::Action::InstallerScan {onboarding:onboarding.clone()}},disabled_reason:None}),
                secondary:if running {vec![model::AvailableAction {label:"Stop installer".into(),
                    action:model::Action::InstallerStop {onboarding,operation},disabled_reason:None}]} else {vec![]},
                rename:model::AvailableAction {label:"Rename".into(),action:model::Action::InstallerRename {installer,label:String::new()},
                    disabled_reason:None},journey:None,history:vec![],
            });
            snapshot.system.maintenance=usize::from(running);
        }
    }
    if matches!(state,"plugin_ready" | "plugin_action" | "plugin_details" | "plugin_history" | "plugin_history_open") {
        let product=&mut snapshot.products[0];
        product.availability = Some(model::NativeAvailability {available:state != "plugin_action",reason:None});
        product.compatibility=Some(model::CompatibilityWorkflow {
            phase:if state == "plugin_action" {model::CompatibilityPhase::ReadyForTest}
                else {model::CompatibilityPhase::OrdinarySupported},
            summary:if state == "plugin_action" {"Preparation complete; publication is not selected."}
                else {"The selected native publication matches this configuration."}.into(),
            established:vec!["Audio playback".into(),"Saved-project recall".into()],
            remaining:vec!["Custom editor on this computer".into()],current_inspection:None,current_candidate:None,
            primary:if state == "plugin_action" {Some(model::AvailableAction {label:"Make available for testing".into(),
                action:model::Action::CompatibilityPublishTest {candidate:"fa".repeat(32),expected_current:None},disabled_reason:None})} else {None},
            alternatives:vec![],
        });
        product.details=serde_json::json!({"verification_scope":"control_records","publication_selected":state != "plugin_action"});
        if matches!(state,"plugin_history" | "plugin_history_open") {
            product.disposition="ready".into();
            product.actions=vec![model::AvailableAction {label:"Check current setup".into(),
                action:model::Action::PluginReinspect {selection:"aa".repeat(32),audio_layout:None},disabled_reason:None}];
            product.compatibility.as_mut().unwrap().primary=Some(product.actions[0].clone());
            for index in 0..12 {
                product.actions.push(model::AvailableAction {label:format!("Restore previous setup {index} · 512-frame bridge buffering"),
                    action:model::Action::OrdinaryRollback {class_id:product.class_id.clone(),publication:format!("{index:032x}")},
                    disabled_reason:(index==0).then(||"Previous files are unavailable".into())});
            }
            product.details["configuration"]=serde_json::json!({"choices":[],"launch":{"runtime":"managed runtime", "environment":"exact installed setup"},
                "qualification":"Recorded tests remain unchanged","buffering":{"added_frames":512}});
        }
    }
    if state == "plugin_technical" {
        let product = &mut snapshot.products[0];
        product.limitations = vec!["editor_not_yet_assessed".into()];
        product.active_revision = Some(3);
        product.recommended_revision = Some(2);
        product.history = vec![model::History { revision: 2, claim: "ordinary".into(), publication: "61".repeat(16),
            active: false, rollback_allowed: true, description: "Version 8.13 · default graphics".into(),
            rollback_unavailable: Some("Installed files changed since this setup".into()) }];
        product.details = serde_json::json!({"preparation": {
            "selection": "62".repeat(32), "publication": "ordinary", "inspection": "inspected",
            "preparation": "prepared", "origin": "retained_sv1",
            "recovery": "The previous preparation stopped; prepare again.",
            "build_prerequisite": "The native bridge for this layout is built.",
            "controller": {"reason": "Controller separated from processor."},
            "candidates": [{"id": "63".repeat(16), "disposition": "accepted", "publication": "selected",
                "publication_acceptance_sealed": true, "unmet_requirements": ["Saved-project recall on this computer"]},
                {"id": "64".repeat(16), "disposition": "superseded", "publication": "retired"}],
            "inspections": [{"id": "65".repeat(16), "result": "passed"}],
            "preliminary": {"parameter_count": 412, "cleanup_confirmed": true,
                "editor_interface": {"created": true, "hwnd_result": 0}},
            "operation": {"state": "failed", "reason": "The bridge build was interrupted.",
                "preparation_failure": {"attempted_stage": "native_build", "recovery": "Prepare this setup again."}},
            "evidence": [{"area": "audio", "status": "passed", "witness": "operator", "detail": "Played a chord"},
                {"area": "editor", "status": "failed", "witness": "operator", "detail": "Blank editor"}],
            "unmet_requirements": ["Saved-project recall on this computer"]}});
    }
    snapshot
}

/// Records for the screens beyond the plug-in list: install progress, required
/// apps, existing setups and their runtime, removal, service problems,
/// incidents and diagnostics. Synthetic sample identities only; every control
/// is an offer the manager can make.
fn screen_records(snapshot: &mut model::Snapshot, state: &str) {
    use serde_json::json;
    fn offer(label: &str, action: serde_json::Value, refused: Option<&str>) -> model::AvailableAction {
        serde_json::from_value(json!({"label": label, "action": action, "disabled_reason": refused}))
            .expect("sample offer")
    }
    fn journey(daw: &str, phase: &str, plugins: serde_json::Value, failure: serde_json::Value)
        -> model::InstallationJourney {
        serde_json::from_value(json!({"daw": daw, "phase": phase, "plugins": plugins, "failure": failure}))
            .expect("sample journey")
    }
    fn found(product: &model::Product) -> model::DiscoveredProduct {
        model::DiscoveredProduct { environment: product.environment.clone(),
            module_sha256: product.module_sha256.clone(), class_id: product.class_id.clone(),
            name: product.name.clone() }
    }
    fn stage(product: &model::Product, stage: &str, failure: serde_json::Value) -> serde_json::Value {
        json!({"selection": product.module_sha256, "class_id": product.class_id, "name": product.name,
            "stage": stage, "failure": failure})
    }
    let base = snapshot.products[0].clone();
    let product = |name: &str, role: &str, environment: &str, index: u8, disposition: &str, available: bool| {
        let mut product = base.clone();
        product.name = name.into();
        product.vendor = "Example Audio".into();
        product.role = role.into();
        product.version = "1.0.0".into();
        product.environment = environment.into();
        product.class_id = format!("{index:02x}").repeat(16);
        product.module_sha256 = format!("{:02x}", index ^ 0x80).repeat(32);
        product.disposition = disposition.into();
        product.availability = Some(model::NativeAvailability { available, reason: None });
        product.compatibility = None;
        product.actions.clear();
        product.history.clear();
        product.limitations.clear();
        product.active_revision = None;
        product.details = json!({});
        product
    };
    let setup = |installer: &str, name: &str, phase: model::SetupPhase, status: &str,
        environment: Option<&str>, discovered: Vec<model::DiscoveredProduct>| model::InstallerSetup {
        installer: installer.into(), name: name.into(), label_source: "operator_named".into(),
        byte_size: 120_000_000, format: "pe_executable".into(), imported_at: 7, phase,
        journey: None, status: status.into(), environment: environment.map(str::to_owned),
        compatibility: None, discovered, primary: None, secondary: vec![],
        rename: offer("Rename", json!({"kind": "installer_rename", "installer": installer, "label": ""}), None),
        history: environment.into_iter().map(|environment| model::SetupHistoryRef {
            environment: Some(environment.into()), state: "installed".into() }).collect(),
    };
    match state {
        "install_progress" | "install_paused" => {
            let environment = "e1".repeat(16);
            let products = [product("Example Keys", "instrument", &environment, 0xa1, "installed_unqualified", false),
                product("Example Pads", "instrument", &environment, 0xa2, "installed_unqualified", false),
                product("Example Echo", "effect", &environment, 0xa3, "installed_unqualified", false),
                product("Example Strings", "instrument", &environment, 0xa4, "prepared", false)];
            let plugins = json!([stage(&products[0], "publishing", json!(null)),
                stage(&products[1], "preparing", json!(null)), stage(&products[2], "inspecting", json!(null)),
                stage(&products[3], "pending", json!(null))]);
            let mut bundle = setup(&"a1".repeat(32), "Example Audio Bundle", model::SetupPhase::DiscoveryComplete,
                "Connecting the installed plug-ins to REAPER.", Some(&environment),
                products.iter().map(found).collect());
            bundle.journey = Some(journey("reaper", "preparing_plugins", plugins.clone(), json!(null)));
            snapshot.installer_setups.push(bundle);
            let operation = "0a".repeat(16);
            let mut running = json!({"operation": operation, "state": "running", "onboarding": environment,
                "journey": {"daw": "reaper", "phase": "preparing_plugins", "plugins": plugins, "failure": null}});
            if state == "install_paused" {
                running["owned_service_pause"] = json!({"schema": 1, "automatic_operation": operation,
                    "owner_operation": "0b".repeat(16)});
                snapshot.system.maintenance = 1;
            }
            snapshot.operation = Some(running);
            snapshot.products.extend(products);
        }
        "install_attention" => {
            let environment = "e2".repeat(16);
            let keys = product("Example Keys", "instrument", &environment, 0xb1, "ready", true);
            let mut pads = product("Example Pads", "instrument", &environment, 0xb2, "installed_unqualified", false);
            pads.details = json!({"problem": {"code": "sdk_component_start",
                "what_happened": "The plug-in loaded, but it refused to start its audio engine.",
                "likely_cause": "It is usually missing something it needs at start-up.",
                "what_to_try": "Use the vendor's installer or app to repair it, then run the check again."}});
            let echo = product("Example Echo", "effect", &environment, 0xb3, "another_configuration", false);
            let plugins = json!([stage(&keys, "available", json!(null)),
                stage(&pads, "needs_attention", json!({"stage": "preparation",
                    "reason": "The native connection could not be prepared.", "code": "preparation_failed",
                    "retryable": true})),
                stage(&echo, "replacement_required", json!(null))]);
            let mut bundle = setup(&"a2".repeat(32), "Example Audio Bundle", model::SetupPhase::DiscoveryComplete,
                "Some plug-ins could not be connected to Bitwig.", Some(&environment),
                [&keys, &pads, &echo].into_iter().map(found).collect());
            bundle.journey = Some(journey("bitwig", "needs_attention", plugins, json!({"stage": "publication",
                "reason": "operation already running", "code": "operation already running", "retryable": true})));
            bundle.primary = Some(offer("Continue installation",
                json!({"kind": "software_install_resume", "onboarding": environment}), None));
            snapshot.installer_setups.push(bundle);
            let drums = "e3".repeat(16);
            let mut unclosed = setup(&"a3".repeat(32), "Example Drums", model::SetupPhase::CleanupUnconfirmed,
                "The bridge could not confirm that the installer closed.", Some(&drums), vec![]);
            unclosed.primary = Some(offer("Check that it closed",
                json!({"kind": "installer_close_check", "operation": "0c".repeat(16)}), None));
            snapshot.installer_setups.push(unclosed);
            snapshot.products.extend([keys, pads, echo]);
        }
        "install_app" => {
            let environment = "e4".repeat(16);
            let synth = product("Hub Synth", "instrument", &environment, 0xc1, "ready", true);
            let waiting = json!({"daw": "bitwig", "phase": "awaiting_application", "plugins": [], "failure": null});
            let mut hub = setup(&"a4".repeat(32), "Example Hub installer", model::SetupPhase::InstallerRetired,
                "The app is installed. Open it below to install your instruments and effects.",
                Some(&environment), vec![found(&synth)]);
            hub.journey = Some(journey("bitwig", "awaiting_application", json!([]), json!(null)));
            hub.primary = Some(offer("Install plug-ins with Example Hub",
                json!({"kind": "vendor_software_install", "application": "example-hub", "daw": "other"}), None));
            snapshot.installer_setups.push(hub);
            let app = |id: &str, name: &str, state: &str, details: serde_json::Value,
                actions: Vec<model::AvailableAction>| model::VendorApplication {
                id: id.into(), name: name.into(), version: "3.1".into(), state: state.into(), details, actions };
            snapshot.vendor_applications.extend([
                app("example-hub", "Example Hub", "running", json!({"environment": environment, "journey": waiting,
                    "dependency": {"registry_defaults": [{"known_folder": "Downloads", "result": "applied", "reason": null}]}}),
                    vec![offer("Show Example Hub", json!({"kind": "vendor_application_focus", "application": "example-hub"}), None),
                        offer("Stop Example Hub", json!({"kind": "vendor_application_stop", "application": "example-hub"}), None),
                        offer("Install plug-ins with Example Hub", json!({"kind": "vendor_software_install",
                            "application": "example-hub", "daw": "other"}), None)]),
                app("example-player", "Example Player", "closed", json!({"environment": environment, "choose_application": true}),
                    vec![offer("Open Example Player", json!({"kind": "vendor_software_install",
                        "application": "example-player", "daw": "other"}), None)]),
                app("example-library", "Example Library", "closed", json!({"environment": environment, "choose_application": true}),
                    vec![offer("Open Example Library", json!({"kind": "vendor_software_install",
                        "application": "example-library", "daw": "other"}), Some("Close Example Hub first"))]),
                app("example-tool", "Example Tool", "closed", json!({"environment": environment,
                    "journey": {"daw": "reaper", "phase": "available", "plugins": [], "failure": null}}),
                    vec![offer("Open Example Tool", json!({"kind": "vendor_application_open", "application": "example-tool"}), None),
                        offer("Stop Example Tool", json!({"kind": "vendor_application_stop", "application": "example-tool"}),
                            Some("Example Tool is not running"))]),
            ]);
            snapshot.products.push(synth);
        }
        "install_complete" => {
            let environment = "e5".repeat(16);
            let keys = product("Example Keys", "instrument", &environment, 0xd1, "ready", true);
            let mut pads = product("Example Pads", "instrument", &environment, 0xd2, "removed_from_daw", false);
            pads.availability = Some(model::NativeAvailability { available: false, reason: Some(
                "Removed from your DAW. Projects that use it open with it missing until you add it back.".into()) });
            let mut done = setup(&"a5".repeat(32), "Example Audio Bundle", model::SetupPhase::DiscoveryComplete,
                "Installation finished and plug-ins are connected.", Some(&environment), vec![found(&keys), found(&pads)]);
            done.journey = Some(journey("reaper", "available",
                json!([stage(&keys, "available", json!(null)), stage(&pads, "available", json!(null))]), json!(null)));
            done.secondary = vec![offer("Remove from my DAW", json!({"kind": "setup_remove_from_daw", "environment": environment}), None)];
            let visit = "e6".repeat(16);
            let mut empty = setup(&"a6".repeat(32), "Example Hub visit", model::SetupPhase::DiscoveryComplete,
                "The app session ended without new plug-ins.", Some(&visit), vec![]);
            empty.journey = Some(journey("bitwig", "available", json!([]), json!(null)));
            let found_only = "e7".repeat(16);
            let echo = product("Example Echo", "effect", &found_only, 0xd3, "ready", true);
            let mut plain = setup(&"a7".repeat(32), "Example Effects", model::SetupPhase::DiscoveryComplete,
                "Installation finished and plug-ins discovered.", Some(&found_only), vec![found(&echo)]);
            plain.label_source = "generated".into();
            snapshot.installer_setups.extend([done, empty, plain]);
            snapshot.products.extend([keys, pads, echo]);
        }
        "existing_setup" => {
            let ni = base.environment.clone();
            let helper = "b1".repeat(32);
            let start = |label: &str, powershell: Option<&str>, refused: Option<&str>| match powershell {
                Some(powershell) => offer(label, json!({"kind": "environment_installer_start_with_policy",
                    "environment": ni, "installer": helper, "powershell": powershell}), refused),
                None => offer(label, json!({"kind": "environment_installer_start", "environment": ni,
                    "installer": helper}), refused),
            };
            let installers: Vec<model::EnvironmentInstaller> = serde_json::from_value(json!([{
                "environment": ni, "name": "Native Instruments setup", "affected": ["Kontakt 8"],
                "consequence": "Kontakt 8 is unavailable in your DAW while the installer runs and is checked again afterwards.",
                "dependency_help": ["Kontakt asks for its licence service on first start. Install it here when asked."],
                "choices": [start("Example Licence Helper", None, None),
                    start("Example Licence Helper · PowerShell unavailable", Some("intentionally_unavailable"), None)],
                "operation": "0d".repeat(16), "installer": helper,
                "result": {"operation": "0d".repeat(16), "state": "failed", "cleanup_confirmed": true, "owned_live": 0,
                    "transaction": {"schema": 1, "operation": "0d".repeat(16), "outcome": "child_failed",
                        "durable_installation": "partial_installation", "installation_completeness": "unproved"}},
                "actions": [
                    offer("Choose a program", json!({"kind": "program_choose", "environment": ni,
                        "expected_environment": "rev-3", "executable": {"path": "C:/Program Files/Example/editor.exe",
                        "sha256": "b2".repeat(32)}}), None),
                    start("Retry", None, None),
                    start("Retry · PowerShell unavailable", Some("intentionally_unavailable"), None),
                    offer("Find installed plug-ins", json!({"kind": "environment_installer_scan", "environment": ni}), None)],
                "runtime": {"runner": "managed-ge-proton11-7-slr4-20260805-r5", "revision": 3,
                    "affected": ["Kontakt 8", "Native Access"],
                    "consequence": "Changing the runtime stops this setup's plug-ins and checks them again.",
                    "choices": [offer("Runtime r7", json!({"kind": "environment_runtime_select", "environment": ni,
                        "runner": "managed-ge-proton11-7-slr4-20260805-r7", "expected_environment": "rev-3"}), None),
                        offer("Runtime r4", json!({"kind": "environment_runtime_select", "environment": ni,
                        "runner": "managed-ge-proton11-7-slr4-20260805-r4", "expected_environment": "rev-3"}),
                        Some("Runtime r4 is older than this setup"))]}
            }, {
                "environment": "fixture-arturia", "name": "Arturia setup", "affected": [],
                "consequence": "Efx FRAGMENTS is unavailable in your DAW while the installer runs.",
                "choices": [], "operation": "0e".repeat(16), "installer": "b3".repeat(32),
                "result": {"operation": "0e".repeat(16), "state": "running"},
                "actions": [offer("Show installer", json!({"kind": "environment_installer_focus", "operation": "0e".repeat(16)}), None),
                    offer("Stop installer", json!({"kind": "environment_installer_stop", "operation": "0e".repeat(16)}), None)],
                "runtime": {"runner": "fixture-runner", "revision": 1, "affected": ["Efx FRAGMENTS"],
                    "consequence": "No other runtime is installed.", "choices": []}
            }])).expect("sample existing setups");
            snapshot.environment_installers = installers;
            snapshot.vendor_applications.push(model::VendorApplication { id: "user-program".into(),
                name: "Example Editor".into(), version: "2.0".into(), state: "closed".into(),
                details: json!({"environment": ni, "user_selected": true,
                    "executable": {"path": "C:/Program Files/Example/editor.exe"}}),
                actions: vec![offer("Open Example Editor", json!({"kind": "vendor_application_open",
                    "application": "user-program"}), None)] });
            let mut kontakt = setup(&"a8".repeat(32), "Native Instruments", model::SetupPhase::DiscoveryComplete,
                "Installation finished and plug-ins discovered.", Some(&ni), vec![found(&base)]);
            kontakt.compatibility = Some("Existing managed configuration".into());
            snapshot.installer_setups.push(kontakt);
        }
        "setup_removal" => {
            let xfer = snapshot.products[2].clone();
            let mut serum = setup(&"a9".repeat(32), "Xfer Serum 2", model::SetupPhase::DiscoveryComplete,
                "Installation finished and plug-ins discovered.", Some(&xfer.environment), vec![found(&xfer)]);
            serum.secondary = vec![
                offer("Remove from my DAW", json!({"kind": "setup_remove_from_daw", "environment": xfer.environment}), None),
                offer("Delete setup", json!({"kind": "setup_delete", "installer": "a9".repeat(32),
                    "environment": xfer.environment}), None)];
            let mut unreadable = setup(&"01".repeat(32), "Unknown Windows installer · ID 01010101…0101",
                model::SetupPhase::Imported, "This file could not be read as a Windows installer.", None, vec![]);
            unreadable.label_source = "generated".into();
            unreadable.history.clear();
            let partial_environment = "e8".repeat(16);
            let lead = product("Example Lead", "instrument", &partial_environment, 0xe1, "installed_unqualified", false);
            let partial = setup(&"aa".repeat(32), "Example Lead installer", model::SetupPhase::SetupNeedsAttention,
                "The installer finished, but a complete installation was not confirmed.", Some(&partial_environment),
                vec![found(&lead)]);
            let fresh = "ab".repeat(32);
            let mut policy = setup(&fresh, "Example Strings installer", model::SetupPhase::Imported,
                "Ready to install.", None, vec![]);
            policy.primary = Some(offer("Install", json!({"kind": "software_install", "installer": fresh, "daw": "other"}), None));
            policy.secondary = vec![offer("Install · PowerShell unavailable", json!({"kind": "software_install_with_policy",
                "installer": fresh, "daw": "other", "powershell": "intentionally_unavailable"}), None)];
            snapshot.installer_setups.extend([serum, unreadable, partial, policy]);
            let onboarding: Vec<model::Onboarding> = serde_json::from_value(json!([{
                "installer": "aa".repeat(32), "name": "Example Lead installer", "byte_size": 120_000_000,
                "format": "pe_executable", "environment": partial_environment, "state": "needs_attention",
                "required_human_action": "Review the installation result",
                "details": {"installation": {"operation": "0f".repeat(16), "state": "completed",
                    "cleanup_confirmed": true, "owned_live": 0, "error": null, "failure": null,
                    "transaction": {"schema": 1, "operation": "0f".repeat(16), "outcome": "partial_installation",
                        "durable_installation": "partial_installation", "installation_completeness": "unproved",
                        "outer_launcher_exit": 1,
                        "first_failure": {"phase": "install", "role": "dependency", "relationship": "child",
                            "domain": "windows", "status": 3},
                        "launch_binding": {"schema": 1, "operation": "0f".repeat(16), "status": "unavailable",
                            "reason": "launch generation was not observed"},
                        "presence_close": {"schema": 1, "observation_count": 2, "operation_classes": ["presence_query"]}},
                    "startup": {"first_problem": {"code": "runtime_assertion_observed"}}}},
                "actions": []
            }, {
                "installer": fresh, "name": "Example Strings installer", "byte_size": 120_000_000,
                "format": "pe_executable", "environment": null, "state": "imported",
                "required_human_action": "Start installation",
                "failure": {"layer": "manager_control_plane", "stage": "environment_creation_admission",
                    "code": "registry_lock_timeout", "retryable": true, "mutation_started": false,
                    "environment_created": false, "installer_launched": false,
                    "lock": {"name": "registry.lock", "mode": "exclusive", "purpose": "environment_creation_admission",
                        "operation": null, "policy": "bounded", "elapsed_wait_us": 2_500_000, "attempts": 3,
                        "timeout_ms": 2500, "outcome": "timeout", "holder": "unknown"}},
                "details": {"installation": {"installer_capability": {
                    "requested": {"powershell": "intentionally_unavailable"},
                    "effective": {"effective": {"windows_scripting": {"powershell": "intentionally_unavailable"}}}}}},
                "actions": [offer("Start · PowerShell unavailable", json!({"kind": "installer_start_with_policy",
                    "onboarding": "ac".repeat(16), "powershell": "intentionally_unavailable"}), None)]
            }])).expect("sample onboarding");
            snapshot.onboarding.extend(onboarding);
            let uninstall = |name: &str, coverage: &str, affected: Vec<String>, refused: Option<&str>| model::Uninstaller {
                environment: xfer.environment.clone(), name: name.into(), affected,
                affected_classes: vec![xfer.class_id.clone()], coverage: coverage.into(),
                action: offer(&format!("Uninstall {name}"), json!({"kind": "product_uninstall",
                    "environment": xfer.environment, "registration": name, "command_sha256": "c1".repeat(32)}), refused),
            };
            snapshot.uninstallers = vec![uninstall("Serum 2", "installed_with_this", vec![xfer.name.clone()], None),
                uninstall("Xfer Shared Runtime", "unknown", vec![xfer.name.clone()], Some("Close Serum 2 FX first"))];
            snapshot.deleted_setups = serde_json::from_value(json!([
                {"installer": "ad".repeat(32), "environment": "e9".repeat(16), "name": "Old Synth", "state": "deleting",
                    "deleted_at": null, "plugins": ["Old Synth"], "actions": [offer("Finish deleting",
                        json!({"kind": "setup_delete", "installer": "ad".repeat(32), "environment": "e9".repeat(16)}), None)]},
                {"installer": "ae".repeat(32), "environment": "ea".repeat(16), "name": "Unreadable setup",
                    "state": "unreadable", "deleted_at": null, "plugins": [], "actions": []},
                {"installer": "af".repeat(32), "environment": "eb".repeat(16), "name": "Old Effects", "state": "deleted",
                    "deleted_at": 1, "plugins": ["Old Echo", "Old Reverb"], "actions": []}])).expect("sample deleted setups");
            snapshot.products.push(lead);
        }
        "service_problem" => {
            snapshot.system.pending_transactions = 1;
            snapshot.system.stale_transports = 1;
            snapshot.actions.push(offer("Finish the interrupted change", json!({"kind": "transaction_reconcile"}), None));
        }
        "incidents" => {
            snapshot.system.dsp = 2;
            snapshot.active_sessions = vec![
                json!({"session": "31".repeat(16), "class_id": "fixture-fragments", "state": "active", "terminal": null, "recent": false}),
                json!({"session": "32".repeat(16), "class_id": "fixture-serumfx", "state": "cleanup_unconfirmed",
                    "terminal": null, "recent": false}),
                json!({"session": "33".repeat(16), "class_id": "fixture-kontakt", "state": "failed",
                    "terminal": "windows_host_exited", "recent": false}),
                json!({"session": "34".repeat(16), "class_id": "fixture-kontakt", "state": "failed",
                    "terminal": "transport_failed", "recent": true, "cleanup_confirmed": false,
                    "transport_retired": true, "observed_at": 1}),
            ];
            snapshot.recent_incidents = serde_json::from_value(json!([
                {"id": "41".repeat(16), "state": "cleanup_confirmed", "summary": {"outcome": "windows_host_exited",
                    "cleanup_confirmed": true, "transport_retired": true, "outer_exit": 0,
                    "windows_self_exit_codes": [3221225477u64], "exceptions": [{"code": "access_violation"}]},
                    "export": offer("Save incident report", json!({"kind": "incident_export", "incident": "41".repeat(16)}), None)},
                {"id": "42".repeat(16), "state": "cleanup_unconfirmed", "summary": {"outcome": "transport_failed",
                    "cleanup_confirmed": false}, "export": null}])).expect("sample incidents");
            snapshot.onboarding.push(serde_json::from_value(json!({"installer": "b4".repeat(32),
                "name": "Example Drums installer", "byte_size": 1, "format": "pe_executable",
                "environment": "ec".repeat(16), "state": "failed",
                "required_human_action": "The installer exited with an error. Open Setup to review it.",
                "details": {}, "actions": []})).expect("sample attempt"));
            snapshot.products[1].disposition = "needs_attention".into();
            snapshot.products[1].availability = Some(model::NativeAvailability { available: false,
                reason: Some("The selected native connection is missing.".into()) });
        }
        "diagnostics_full" => {
            snapshot.capture = json!({"armed": true, "active_retention": 0});
            snapshot.actions.push(offer("Arm crash capture for Kontakt 8",
                json!({"kind": "capture_arm", "class_id": base.class_id}), None));
            snapshot.operation = Some(json!({"operation": "51".repeat(16), "state": "completed",
                "result": {"message": "Finished"}}));
            snapshot.managed_runtime_records = vec![json!({"id": "managed-ge-proton11-7-slr4-20260805-r7",
                "failure": null}), json!({"id": "managed-ge-proton11-7-slr4-20260805-r4", "failure": "changed"})];
            for environment in &mut snapshot.environments {
                environment.runner = "managed-ge-proton11-7-slr4-20260805-r7".into();
                environment.last_scan = json!({"module_count": 3, "stale": false});
            }
            snapshot.bridge.setup = model::SetupPackage::Newer("0.12.0preview2".into());
            snapshot.vendor_applications[1].details["dependency_operation"] = json!({
                "error": "dependency_command_timeout",
                "dependency": {"service_retirement_confirmed": false, "process_cleanup_confirmed": true,
                    "forced_cleanup_used": true, "dependency_cleanup_disposition": "exact_owned_session_cleanup",
                    "recovery": {"authority": "qualified_bundle_payload_and_retired_installation"}, "ready_tested": false,
                    "stages": [{"action": "install", "exit": 0, "installer_result": {"installer_exit": 0,
                        "authority": "exact_windows_child_handle_exit"}}],
                    "stop_observation": {"schema": 3, "classification": "NAD2_STOP_SUBMITTED_PROGRESSING",
                        "initial_state": 4, "final_state": 3, "initial_controls_accepted": 1, "final_controls_accepted": 0,
                        "initial_checkpoint": 0, "checkpoint": 2, "initial_wait_hint_ms": 0, "wait_hint_ms": 3000,
                        "control_count": 1, "control_submitted": true, "control_elapsed_ms": 4,
                        "control_return": {"state": 3, "controls_accepted": 0, "checkpoint": 1, "wait_hint_ms": 3000},
                        "process_wait_class": "timeout", "process_wait": 258, "initial_listener_mask": 1,
                        "listener_mask": 1, "transition_count_total": 2, "transition_count_retained": 2,
                        "transition_count_dropped": 0}}});
        }
        "pb0_step" => {}
        other => panic!("no screen records for {other}"),
    }
}

/// Sample setup-check results for the pb0 states, over the final records.
pub fn readiness(requested: Option<&str>, snapshot: &model::Snapshot) -> Option<model::ReadinessAssessment> {
    match requested {
        Some(mode @ ("pb0_ready" | "pb0_action" | "pb0_unsupported" | "pb0_unknown" | "pb0_step")) => {
            use model::ReadinessOutcome as O;
            let (outcome, reason, step) = match mode {
                "pb0_ready" => (O::Ready, "Current setup records match the selected configuration.", None),
                "pb0_action" => (O::ActionRequired,
                    "Bitwig audio callback maximum still needs confirmation.",
                    Some("Check Bitwig audio settings")),
                "pb0_step" => (O::ActionRequired,
                    "Windows plug-in support is not installed yet.",
                    Some("Download Windows plug-in support")),
                "pb0_unsupported" => (O::Unsupported,
                    "This native engine requires x86-64 Linux.", None),
                _ => (O::Unknown,
                    "Current desktop display access could not be checked.", None),
            };
            let product_status = if outcome == O::Unsupported { O::Unknown } else { outcome };
            Some(model::ReadinessAssessment {
                schema: 1,
                state_token: snapshot.state_token.clone(),
                observed_at: 1,
                overall_status: outcome,
                system: snapshot.system.clone(),
                platform: vec![model::ReadinessFact {
                    name: "System".into(), value: Some("SteamOS 3.8.16".into()),
                    source: "fixed source preview".into(), observed_at: 1,
                    certainty: model::FactCertainty::Observed,
                }],
                daw: if mode == "pb0_step" { vec![model::ReadinessFact { name: "Bitwig".into(),
                    value: Some("5.3".into()), source: "fixed source preview".into(), observed_at: 1,
                    certainty: model::FactCertainty::Inferred }] } else { vec![] },
                audio: if mode == "pb0_step" { vec![model::ReadinessFact { name: "Audio server".into(),
                    value: None, source: "fixed source preview".into(), observed_at: 1,
                    certainty: model::FactCertainty::Unknown }] } else { vec![] },
                graphics: vec![], runtime: vec![],
                products: snapshot.products.iter().map(|p| model::ReadinessProduct {
                    name: p.name.clone(), class_id: p.class_id.clone(),
                    module_sha256: p.module_sha256.clone(), profile: None,
                    status: product_status,
                    installation_health: model::InstallationHealth::Healthy,
                    support_qualification: if product_status == O::Ready {
                        model::SupportQualification::Verified
                    } else { model::SupportQualification::NotYetQualified },
                    reason: reason.into(), failure_code: None,
                    facts: vec![],
                }).collect(),
                blockers: if outcome == O::Ready { vec![] } else {
                    vec![model::ReadinessBlocker {
                        category: "preview".into(), status: outcome,
                        explanation: reason.into(),
                    }]
                },
                ordered_steps: step.into_iter().map(|title| model::ReadinessStep {
                    title: title.into(), detail: "Verify the value in Bitwig, then Check again.".into(),
                    action: (mode == "pb0_step").then(|| model::AvailableAction {
                        label: "Install compatibility runtime (310 MB)".into(),
                        action: model::Action::RuntimeInstall {}, disabled_reason: None }),
                }).collect(),
                support_export_action: model::AvailableAction {
                    label: "Create sanitized support export".into(),
                    action: model::Action::SupportExport {}, disabled_reason: None,
                },
            })
        }
        _ => None,
    }
}

// The sample table: each named state as a window shows it, the request and
// window samples over it, and every state of Setup's update panel. The
// current window's previews and render checks are built from these, so a
// gallery that draws `Sample::window` shows what that window draws.

/// One named state on one page, with the window state previews show it in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Case {
    pub page: Page,
    pub state: String,
    /// The first plug-in is open, with its own records read.
    pub plugin_open: bool,
    /// The musician's last request is still in flight.
    pub pending: bool,
    /// Every disclosure is held open.
    pub expanded: bool,
    /// The records are no longer current.
    pub stale: bool,
}

impl Case {
    pub fn new(page: Page, state: &str) -> Self {
        Self {
            page,
            state: state.to_owned(),
            plugin_open: page == Page::Plugins && state != "current_loading",
            pending: state == "pending",
            expanded: matches!(state, "plugin_details" | "plugin_history_open" | "plugin_technical"
                | "recorded_expanded"),
            stale: state == "stale",
        }
    }
}

/// Every page in every named state, in the order the render checks draw them.
pub fn cases() -> impl Iterator<Item = Case> {
    Page::ALL.into_iter().flat_map(|page| STATES.into_iter().map(move |state| Case::new(page, state)))
}

/// A request as the window's own feedback records it.
#[derive(Clone, Debug)]
pub struct RequestRecord {
    pub action: model::Action,
    pub text: String,
    pub operation: Option<String>,
    pub transitions: Vec<String>,
    pub journey: Option<model::InstallationJourney>,
    /// A problem report ready to save: its id and SHA-256.
    pub report: Option<(String, String)>,
    /// The manager reported its result.
    pub terminal: bool,
    /// The request still holds the window's controls.
    pub blocking: bool,
}

impl RequestRecord {
    pub fn view(&self) -> crate::window::Request<'_> {
        crate::window::Request { text: &self.text, journey: self.journey.as_ref(),
            operation: self.operation.as_deref(), transitions: &self.transitions,
            report: self.report.is_some(), blocking: self.blocking }
    }

    /// A click waiting for current status, then started.
    fn running(action: model::Action) -> Self {
        Self { action, text: crate::window::CAPTURED.into(), operation: None,
            transitions: vec!["captured".into(), "running".into()], journey: None, report: None,
            terminal: false, blocking: true }
    }

    /// A request the manager accepted and finished.
    fn settled(action: model::Action, text: &str) -> Self {
        Self { action, text: text.into(), operation: Some("0a".repeat(16)),
            transitions: vec!["captured".into(), "accepted".into(), "completed".into()], journey: None,
            report: None, terminal: true, blocking: false }
    }
}

/// The request bar and window samples, drawn over Home and the plug-in page.
pub const REQUESTS: [&str; 7] = ["journey", "report", "refused", "message", "importing", "superseded", "renewing"];

/// Setup's update panel in one state.
#[derive(Clone, Debug, PartialEq)]
pub struct UpdateSample {
    /// A beta link is saved.
    pub linked: bool,
    /// The musician is changing it.
    pub editing: bool,
    /// What is typed in the link field.
    pub draft: String,
    pub link_message: Option<String>,
    pub phase: crate::updates::Phase,
    pub notice: Option<crate::updates::UpdateNotice>,
}

impl UpdateSample {
    /// A check, download or hand-over runs.
    pub fn busy(&self) -> bool {
        use crate::updates::Phase;
        matches!(self.phase, Phase::Checking | Phase::Downloading { .. } | Phase::HandingOver | Phase::HandedOver)
    }

    pub fn panel(&self) -> crate::updates::Panel {
        crate::updates::Panel::build(self.linked, self.editing, self.busy(), self.link_message.as_deref(),
            &self.phase, self.notice.as_ref())
    }
}

/// Every state Setup's update panel draws, in the order the render checks draw them.
pub fn updates() -> Vec<(&'static str, UpdateSample)> {
    use crate::updates::{Phase, UpdateNotice};
    let version = || "0.12.0test148".to_owned();
    let linked = |phase: Phase| UpdateSample { linked: true, editing: false, draft: String::new(), link_message: None,
        phase, notice: None };
    let available = || Phase::Available { version: version(), notes: "Setup → Beta updates\r\nFaster plug-in list".into() };
    vec![
        ("no_link", UpdateSample { linked: false, ..linked(Phase::Idle) }),
        ("unreadable_link", UpdateSample { linked: false,
            link_message: Some("Your saved beta link could not be read. Paste it again.".into()), ..linked(Phase::Idle) }),
        ("idle", linked(Phase::Idle)),
        ("editing", UpdateSample { editing: true, draft: "https://example".into(),
            link_message: Some("This is not a beta link.".into()), ..linked(Phase::Idle) }),
        ("checking", linked(Phase::Checking)),
        ("failed", linked(Phase::Failed(UpdateNotice {
            text: "Setup is already installing something; try again when it finishes.".into(),
            technical: Some("user_package_intake_busy".into()) }))),
        ("up_to_date", linked(Phase::UpToDate("0.12.0test146".into()))),
        ("no_package", linked(Phase::NoPackage(version()))),
        ("manual", linked(Phase::Manual(Some(version())))),
        ("manual_unknown", linked(Phase::Manual(None))),
        ("available", linked(available())),
        ("available_notice", UpdateSample { notice: Some(UpdateNotice {
            text: "The download stopped before it finished. Choose Download and update again to continue where it left off.".into(),
            technical: None }), ..linked(available()) }),
        ("downloading", linked(Phase::Downloading { version: version(), done: 4_000_000, total: 10_000_000 })),
        ("waiting", linked(Phase::Waiting { version: version(), observed: false, long: false })),
        ("waiting_long", linked(Phase::Waiting { version: version(), observed: false, long: true })),
        ("handing_over", linked(Phase::HandingOver)),
        ("handed_over", linked(Phase::HandedOver)),
    ]
}

/// One sample as a window holds it: the records, the window's own state, and
/// what its last request, notice and updater say.
#[derive(Clone, Debug)]
pub struct Sample {
    pub case: Case,
    pub snapshot: model::Snapshot,
    pub readiness: Option<model::ReadinessAssessment>,
    pub request: Option<RequestRecord>,
    pub message: String,
    /// An installer file is being chosen.
    pub importing: bool,
    /// An updated window replaced this one.
    pub superseded: bool,
    /// The records aged and are being read again.
    pub renewing: bool,
    pub update: Option<UpdateSample>,
}

impl Sample {
    /// The case's own sample records.
    pub fn new(case: Case) -> Self {
        let snapshot = snapshot(case.page, Some(&case.state));
        let readiness = readiness(Some(&case.state), &snapshot);
        Self::with_records(case, snapshot, readiness)
    }

    /// The case shown over other records, such as a recorded snapshot.
    pub fn with_records(case: Case, snapshot: model::Snapshot, readiness: Option<model::ReadinessAssessment>) -> Self {
        let request = case.pending.then(|| RequestRecord::running(model::Action::RuntimeInstall {}));
        Self { case, snapshot, readiness, request, message: String::new(), importing: false, superseded: false,
            renewing: false, update: None }
    }

    /// One of `REQUESTS` over this sample.
    pub fn with_request(mut self, name: &str) -> Self {
        use model::Action;
        match name {
            "journey" => {
                let mut request = RequestRecord::settled(Action::SoftwareInstall { installer: "ab".repeat(32),
                    daw: model::DawChoice::Reaper },
                    "Installation or DAW connection needs attention. Its results and continuation are shown below.");
                request.journey = serde_json::from_value(serde_json::json!({"daw": "reaper", "phase": "needs_attention",
                    "failure": {"stage": "publication", "reason": "Run the same step again.", "code": "x", "retryable": true},
                    "plugins": [{"selection": "s1", "class_id": "c1", "name": "Example Keys", "stage": "available", "failure": null},
                        {"selection": "s2", "class_id": "c2", "name": "Example Pads", "stage": "needs_attention",
                            "failure": {"stage": "preparation", "reason": "The native connection could not be prepared.",
                                "code": "y", "retryable": true}},
                        {"selection": "s3", "class_id": "c3", "name": "Example Echo", "stage": "replacement_required",
                            "failure": null}]})).ok();
                self.request = Some(request);
            }
            "report" => {
                let mut request = RequestRecord::settled(Action::SupportExport {},
                    "Problem report created. Choose where to save the file to send.");
                request.report = Some(("ab".repeat(16), "cd".repeat(32)));
                self.request = Some(request);
            }
            "refused" => self.request = Some(RequestRecord::settled(Action::RuntimeInstall {},
                "This request was refused: Close active bridged instances before this action")),
            "message" => self.message = format!("Operation accepted: {}", "0a".repeat(16)),
            "importing" => self.importing = true,
            "superseded" => self.superseded = true,
            "renewing" => self.renewing = true,
            other => panic!("unknown request sample: {other}"),
        }
        self
    }

    /// One of `updates()` as the updater's state.
    pub fn with_update(mut self, name: &str) -> Self {
        self.update = Some(updates().into_iter().find(|(sample, _)| *sample == name)
            .unwrap_or_else(|| panic!("unknown update sample: {name}")).1);
        self
    }

    /// The window's own state for this sample.
    pub fn state(&self) -> crate::state::State {
        let mut state = crate::state::State { page: self.case.page, ..Default::default() };
        state.plugins.expanded = self.case.expanded;
        if self.case.plugin_open {
            if let Some(product) = self.snapshot.products.first() {
                state.plugins.focus_product(product.into());
            }
        }
        state
    }

    fn fresh(&self) -> bool { !self.case.stale && !self.renewing }

    /// Current status is at hand: fresh, or aged and being read again.
    fn current(&self) -> bool { self.fresh() || self.renewing }

    /// The opened plug-in's records, when exactly one plug-in matches it.
    fn product(&self) -> Option<&model::Snapshot> {
        let product = self.snapshot.products.first().filter(|_| self.case.plugin_open)?;
        let key = crate::ProductKey::from(product);
        (self.snapshot.products.iter().filter(|other| crate::ProductKey::from(*other) == key).count() == 1)
            .then_some(&self.snapshot)
    }

    pub fn records(&self) -> crate::screen::Records<'_> {
        let current = self.current().then_some(&self.snapshot);
        crate::screen::Records {
            current: Some(&self.snapshot),
            details: Some(&self.snapshot),
            product: self.product(),
            actions: current,
            readiness: self.readiness.as_ref().filter(|readiness| readiness.state_token == self.snapshot.state_token
                && readiness.system == self.snapshot.system),
            health: current.map(|snapshot| &snapshot.system),
            support: None,
        }
    }

    /// `distribution`: the name Diagnostics shows; a frontend reads its own.
    pub fn reading<'a>(&'a self, distribution: Option<&'a str>) -> crate::screen::Reading<'a> {
        let in_flight = self.case.pending;
        crate::screen::Reading {
            fresh: self.fresh(),
            renewing: self.renewing,
            current_unavailable: false,
            health_renewing: !self.superseded && self.renewing,
            owned_pause: self.current() && crate::service::owned_service_pause(self.snapshot.operation.as_ref()),
            controls_pending: self.superseded || self.importing || in_flight
                || self.request.as_ref().is_some_and(|request| request.blocking),
            in_flight,
            working: self.importing || in_flight,
            explicit_refresh: false,
            details_attempted: false,
            details_notice: None,
            product_attempted: false,
            superseded: self.superseded,
            preview: true,
            own_version: None,
            distribution,
            update_headline: self.update.as_ref().and_then(|update| crate::updates::headline(&update.phase)),
            request: self.request.as_ref().map(RequestRecord::view),
            message: &self.message,
        }
    }

    pub fn window(&self, distribution: Option<&str>) -> crate::screen::Window {
        crate::screen::Window::build(&self.records(), &self.reading(distribution), &self.state())
    }
}

//! Source-owned preview of the production manager views with synthetic records.
//! Usage: OUTPUT.png [WIDTH] [PAGE] [STATE] [light|dark] [HEIGHT] [SNAPSHOT.json] [OPEN_LABEL]
//! States: populated, available, first_use, setup_many, install_running, pending,
//! recoverable, plugin_ready, plugin_action, plugin_details; historical states also work.
#[path = "../src/bootstrap.rs"]
#[allow(dead_code)]
mod bootstrap;
#[path = "../src/client.rs"]
mod client;
#[path = "../src/library.rs"]
mod library;
#[allow(dead_code)]
#[path = "../../bridge-manager/src/operator_model.rs"]
mod model;
#[path = "../src/operator.rs"]
#[allow(dead_code)]
mod operator;
#[path = "../src/presentation.rs"]
mod presentation;

use eframe::egui;

struct Preview {
    operator: operator::Operator,
    output: std::path::PathBuf,
    frame: usize,
    open_label: Option<String>,
    click_at: Option<egui::Pos2>,
}

impl eframe::App for Preview {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if matches!(self.frame, 3 | 4) {
            if let Some(pos) = self.click_at {
                input.events.extend([egui::Event::PointerMoved(pos),egui::Event::PointerButton {
                    pos, button:egui::PointerButton::Primary, pressed:self.frame==3,
                    modifiers:egui::Modifiers::NONE}]);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let screenshot = ui.ctx().input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot {
            let file = std::fs::File::create(&self.output).expect("create preview image");
            let mut encoder = png::Encoder::new(file, image.width() as u32, image.height() as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("PNG header")
                .write_image_data(image.as_raw())
                .expect("PNG image");
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.operator.ui(ui, frame);
        if self.frame == 2 {
            if let Some(label) = &self.open_label {
                fn center(shape:&egui::epaint::Shape,label:&str)->Option<egui::Pos2> {
                    match shape {
                        egui::epaint::Shape::Text(text) if text.galley.text().starts_with(label) =>
                            Some(text.pos+text.galley.size()*0.5),
                        egui::epaint::Shape::Vec(items)=>items.iter().find_map(|shape|center(shape,label)),
                        _=>None,
                    }
                }
                self.click_at=ui.ctx().graphics(|graphics|graphics.get(ui.layer_id())
                    .and_then(|list|list.all_entries().find_map(|entry|center(&entry.shape,label))));
                assert!(self.click_at.is_some(),"Preview disclosure label was not rendered: {label}");
            }
        }
        self.frame += 1;
        if self.frame == if self.open_label.is_some() { 8 } else { 3 } {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        if self.frame > 300 {
            panic!("preview screenshot was not returned");
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(30));
    }
}

fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let output = std::path::PathBuf::from(args.first().expect("OUTPUT.png required"));
    let width = args
        .get(1)
        .map(|value| value.parse::<f32>().expect("WIDTH must be a number"))
        .unwrap_or(960.0);
    let page = match args.get(2).map(String::as_str).unwrap_or("home") {
        "home" => operator::Page::Home,
        "plugins" => operator::Page::Plugins,
        "workspaces" => operator::Page::Workspaces,
        "activity" => operator::Page::Activity,
        "setup" => operator::Page::Setup,
        "diagnostics" => operator::Page::Diagnostics,
        other => panic!("unknown page: {other}"),
    };
    let state = args.get(3).map(String::as_str).unwrap_or("populated");
    let mut snapshot: model::Snapshot =
        serde_json::from_str(include_str!("library-preview.json")).expect("preview fixture");
    snapshot.bridge = model::BridgeBuild { installed: Some("0.12.0preview".into()),
        setup: model::SetupPackage::Nothing };
    if page == operator::Page::Plugins
        && matches!(args.get(3).map(String::as_str), Some("guided" | "guided_new" | "guided_pending")) {
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
        if args.get(3).map(String::as_str) == Some("guided_pending") {
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
        snapshot.products = if args.get(3).map(String::as_str) == Some("guided_new") {
            vec![bismuth]
        } else { vec![beam, bismuth] };
    }
    if page == operator::Page::Setup {
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
    if page == operator::Page::Workspaces {
        snapshot.workspaces.push(model::DawWorkspace {
            id: "aa".repeat(16),
            name: "FL Studio".into(),
            state: "uninstalled".into(),
            selected_installer: "bb".repeat(32),
            application_installers: vec!["bb".repeat(32)],
            selected_release: "26.1.6.0".into(),
            installed_advertised_release: None,
            observed_file_version: None,
            installed_image_sha256: None,
            active_installation_operation: None,
            cleanup: "confirmed".into(),
            first_useful_failure: None,
            actions: vec![model::AvailableAction {
                label: "Install selected version".into(),
                action: model::Action::WorkspaceInstall {},
                disabled_reason: None,
            }],
            installer_choices: vec![model::AvailableAction {
                label: "Choose imported installer cccccccccccc".into(),
                action: model::Action::WorkspaceSelectInstaller {
                    installer: "cc".repeat(32),
                    release: String::new(),
                },
                disabled_reason: None,
            }],
            products: vec![model::DawWorkspaceProduct {
                id: model::WorkspaceProductId::Serum2,
                name: "Serum 2".into(),
                state: "selected".into(),
                selected_release: Some("2.1.5".into()),
                module_sha256: None,
                current_failure: None,
                actions: vec![model::AvailableAction {
                    label: "Install Serum 2 in FL Studio".into(),
                    action: model::Action::WorkspaceInstallProduct { product: model::WorkspaceProductId::Serum2 },
                    disabled_reason: None,
                }],
                installer_choices: vec![],
                details: serde_json::json!({"installation_history":[]}),
            }],
            details: serde_json::json!({
                "installation_history":[{"record":{"operation":"first-install","outcome":"installed"}}],
                "uninstall_history":[{"record":{"operation":"first-uninstall","outcome":"completed"}}]
            }),
        });
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
    match args.get(3).map(String::as_str).unwrap_or("busy") {
        "busy" => {}
        "idle" | "guided" | "guided_new" | "guided_pending"
        | "pb0_ready" | "pb0_action" | "pb0_unsupported" | "pb0_unknown"
        | "populated" | "available" | "first_use" | "setup_many" | "install_running"
        | "pending" | "recoverable" | "plugin_ready" | "plugin_action" | "plugin_details"
        | "retained_installation" | "plugin_history" | "plugin_history_open" | "current_loading" | "stale" => {
            snapshot.system.dsp = 0;
            snapshot.active_sessions.retain(|row| row["recent"] == true);
            snapshot.operation = None;
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
    if let Some(path) = args.get(6) {
        assert!(matches!(state,"recorded_current" | "recorded_expanded"),
            "External snapshots require a recorded preview state");
        snapshot = serde_json::from_slice(&std::fs::read(path).expect("read allowlisted preview snapshot"))
            .expect("decode recorded preview snapshot");
    }
    let theme = match args.get(4).map(String::as_str).unwrap_or("light") {
        "light" => egui::Theme::Light,
        "dark" => egui::Theme::Dark,
        other => panic!("unknown theme: {other}"),
    };
    let readiness = match args.get(3).map(String::as_str) {
        Some(mode @ ("pb0_ready" | "pb0_action" | "pb0_unsupported" | "pb0_unknown")) => {
            use model::ReadinessOutcome as O;
            let (outcome, reason, step) = match mode {
                "pb0_ready" => (O::Ready, "Current setup records match the selected configuration.", None),
                "pb0_action" => (O::ActionRequired,
                    "Bitwig audio callback maximum still needs confirmation.",
                    Some("Check Bitwig audio settings")),
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
                daw: vec![], audio: vec![], graphics: vec![], runtime: vec![],
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
                    action: None,
                }).collect(),
                support_export_action: model::AvailableAction {
                    label: "Create sanitized support export".into(),
                    action: model::Action::SupportExport {}, disabled_reason: None,
                },
            })
        }
        _ => None,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width,args.get(5).map(|height|height.parse::<f32>().expect("HEIGHT must be a number")).unwrap_or(720.0)])
            .with_active(false),
        ..Default::default()
    };
    let open_label = args.get(7).cloned();
    eframe::run_native(
        "Manager preview",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_theme(theme);
            presentation::configure_style(&cc.egui_ctx);
            // Capture the settled disclosure state, not an animation frame.
            cc.egui_ctx.global_style_mut(|style| style.animation_time = 0.0);
            Ok(Box::new(Preview {
                operator: operator::Operator::preview_current(snapshot, readiness, page,
                    page == operator::Page::Plugins && state != "current_loading",
                    state == "pending", matches!(state,"plugin_details" | "plugin_history_open" | "recorded_expanded"), state == "stale"),
                output,
                frame: 0,
                open_label,
                click_at: None,
            }))
        }),
    )
}

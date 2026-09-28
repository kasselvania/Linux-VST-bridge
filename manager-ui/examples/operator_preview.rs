//! Source-owned preview of the production manager views with synthetic records.
//! Usage: OUTPUT.png [WIDTH] [PAGE] [busy|idle|unavailable|cleanup|shared|guided|guided_new|guided_pending] [light|dark]
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
}

impl eframe::App for Preview {
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
        self.frame += 1;
        if self.frame == 3 {
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
    let mut snapshot: model::Snapshot =
        serde_json::from_str(include_str!("library-preview.json")).expect("preview fixture");
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
            history: vec![model::SetupHistoryRef { environment: Some(environment.clone()),
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
        "idle" | "guided" | "guided_new" | "guided_pending" => {
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
        other => panic!("unknown state: {other}"),
    }
    let theme = match args.get(4).map(String::as_str).unwrap_or("light") {
        "light" => egui::Theme::Light,
        "dark" => egui::Theme::Dark,
        other => panic!("unknown theme: {other}"),
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width, 720.0])
            .with_active(false),
        ..Default::default()
    };
    eframe::run_native(
        "Manager preview",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_theme(theme);
            let mut style = (*cc.egui_ctx.global_style()).clone();
            style.spacing.interact_size.y = 42.0;
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            cc.egui_ctx.set_global_style(style);
            Ok(Box::new(Preview {
                operator: operator::Operator::preview(snapshot, page),
                output,
                frame: 0,
            }))
        }),
    )
}

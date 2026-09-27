//! Source-owned preview of the production manager views with synthetic records.
//! Usage: OUTPUT.png [WIDTH] [PAGE] [busy|idle|unavailable|cleanup|shared|readiness_ready|readiness_action|readiness_unsupported|readiness_unknown] [light|dark]
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
    let preview_state=args.get(3).map(String::as_str).unwrap_or("busy");
    let mut snapshot: model::Snapshot = if preview_state=="real" {
        serde_json::from_slice(&std::fs::read(args.get(5).expect("snapshot JSON required")).expect("read snapshot"))
            .expect("parse exact manager snapshot")
    } else {
        serde_json::from_str(include_str!("library-preview.json")).expect("preview fixture")
    };
    if page == operator::Page::Workspaces && preview_state!="real" {
        snapshot.workspaces.push(model::DawWorkspace {
            id: "aa".repeat(16),
            name: "FL Studio".into(),
            state: "uninstalled".into(),
            selected_installer: "bb".repeat(32),
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
            details: serde_json::json!({
                "installation_history":[{"record":{"operation":"first-install","outcome":"installed"}}],
                "uninstall_history":[{"record":{"operation":"first-uninstall","outcome":"completed"}}]
            }),
        });
    }
    if preview_state!="real" {
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
    }
    match preview_state {
        "real" => {},
        "busy" => {}
        "idle" => {
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
        "readiness_ready"|"readiness_action"|"readiness_unsupported"|"readiness_unknown" => {
            snapshot.system.dsp=0;
            snapshot.active_sessions.clear();
            snapshot.operation=None;
        }
        other => panic!("unknown state: {other}"),
    }
    let readiness=if preview_state=="real" {
        Some(serde_json::from_slice::<model::ReadinessAssessment>(
            &std::fs::read(args.get(6).expect("readiness JSON required")).expect("read readiness"))
            .expect("parse exact manager readiness"))
    } else if preview_state.starts_with("readiness_") {
        let status=match preview_state {
            "readiness_ready"=>model::ReadinessOutcome::Ready,
            "readiness_action"=>model::ReadinessOutcome::ActionRequired,
            "readiness_unsupported"=>model::ReadinessOutcome::Unsupported,
            _=>model::ReadinessOutcome::Unknown,
        };
        let reason=match status {
            model::ReadinessOutcome::Ready=>"",
            model::ReadinessOutcome::ActionRequired=>"Bitwig's callback maximum has not been confirmed from the current session.",
            model::ReadinessOutcome::Unsupported=>"This execution lane requires x86-64 Linux.",
            model::ReadinessOutcome::Unknown=>"This distribution and DAW combination has not been qualified.",
        };
        Some(model::ReadinessAssessment {
            schema:1,state_token:snapshot.state_token.clone(),observed_at:123,
            overall_status:status,system:snapshot.system.clone(),
            platform:vec![],daw:vec![],audio:vec![],graphics:vec![],runtime:vec![],
            products:vec![model::ReadinessProduct {name:"Pure LoFi".into(),
                class_id:"fixture-pure-lofi".into(),module_sha256:"fixture-module".into(),
                profile:Some("arturia-pure-lofi".into()),status,
                reason:if status==model::ReadinessOutcome::Ready {
                    "Exact supported fixture verified.".into()
                } else {reason.into()},failure_code:None,facts:vec![]}],
            blockers:if reason.is_empty(){vec![]}else{vec![model::ReadinessBlocker {
                category:"fixture".into(),status,explanation:reason.into()}]},
            ordered_steps:if status==model::ReadinessOutcome::ActionRequired {
                vec![model::ReadinessStep {title:"Verify Bitwig audio settings".into(),
                    detail:"Confirm 48 kHz and a maximum of at most 512 frames. The idle manager cannot yet verify these values.".into(),action:None}]
            } else {vec![]},
            support_export_action:model::AvailableAction {label:"Create sanitized support export".into(),
                action:model::Action::SupportExport {},disabled_reason:None},
        })
    } else {None};
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
                operator: if let Some(readiness)=readiness {
                    operator::Operator::preview_with_readiness(snapshot,readiness,page,preview_state=="real")
                } else {operator::Operator::preview(snapshot,page)},
                output,
                frame: 0,
            }))
        }),
    )
}

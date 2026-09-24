//! Source-owned preview of the production manager views with synthetic records.
//! Usage: OUTPUT.png [WIDTH] [PAGE] [busy|idle|unavailable|cleanup] [light|dark]
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
        "idle" => {
            snapshot.system.dsp = 0;
            snapshot.active_sessions.retain(|row| row["recent"] == true);
            snapshot.operation = None;
        }
        "unavailable" => {
            snapshot.system.service = "capacity unavailable".into();
            snapshot.system.cleanup_unconfirmed = true;
            snapshot.system.ceiling = 0;
            snapshot.system.dsp = 0;
        }
        "cleanup" => snapshot.system.cleanup_unconfirmed = true,
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

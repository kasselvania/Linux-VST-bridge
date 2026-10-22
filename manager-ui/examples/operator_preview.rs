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
use linux_vst_bridge::operator_model as model;
#[path = "../src/operator.rs"]
#[allow(dead_code)]
mod operator;
#[path = "../src/presentation.rs"]
mod presentation;
#[path = "../src/preview_states.rs"]
#[allow(dead_code)]
mod preview_states;
#[path = "../src/screen_model/mod.rs"]
#[allow(dead_code)]
mod screen_model;
#[path = "../src/updates.rs"]
#[allow(dead_code)]
mod updates;
#[path = "../src/widgets.rs"]
mod widgets;

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
    let mut snapshot = preview_states::snapshot(page, args.get(3).map(String::as_str));
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
    let readiness = preview_states::readiness(args.get(3).map(String::as_str), &snapshot);
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

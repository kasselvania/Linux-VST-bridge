//! Local visual preview of the production library widget. No manager client or OS input.
//! With OUTPUT.png [WIDTH] [SEARCH] [--diagnostics], saves its own rendered frame and exits.
#[path = "../src/library.rs"]
mod library;
#[allow(dead_code)]
#[path = "../../bridge-manager/src/operator_model.rs"]
mod model;
use eframe::egui;

struct Preview {
    snapshot: model::Snapshot,
    library: library::Library,
    output: Option<std::path::PathBuf>,
    frame: usize,
    diagnostics: bool,
}

impl eframe::App for Preview {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let screenshot = ui.ctx().input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let (Some(path), Some(image)) = (&self.output, screenshot) {
            let file = std::fs::File::create(path).expect("create preview image");
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
        egui::CentralPanel::default().show(ui, |ui| {
            ui.small("LOCAL DESIGN PREVIEW · synthetic records · actions do not execute");
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut selected = None;
                self.library
                    .show(ui, &self.snapshot, false, &mut selected, |ui, p| {
                        ui.label(format!(
                            "Example metadata for {}. No installed product was inspected.",
                            p.name
                        ));
                    });
                if self.diagnostics {
                    ui.add_space(16.0);
                    library::diagnostics(ui, &self.snapshot, false, &mut selected, true);
                }
            });
        });
        self.frame += 1;
        if self.output.is_some() && self.frame == 3 {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        if self.frame > 300 && self.output.is_some() {
            panic!("preview screenshot was not returned");
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(30));
    }
}

fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let output = args.first().map(std::path::PathBuf::from);
    let width = args
        .get(1)
        .map(|s| s.parse::<f32>().expect("width"))
        .unwrap_or(960.0);
    let mut library = library::Library::default();
    library.search = args.get(2).cloned().unwrap_or_default();
    let diagnostics = args.get(3).is_some_and(|arg| arg == "--diagnostics");
    library.expand_details = diagnostics;
    let mut snapshot: model::Snapshot =
        serde_json::from_str(include_str!("library-preview.json")).expect("preview data");
    if diagnostics {
        snapshot.capture = serde_json::json!({"armed": true, "active_retention": 0});
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([width, 800.0])
            .with_active(false),
        ..Default::default()
    };
    eframe::run_native(
        "Plug-in library preview",
        options,
        Box::new(move |cc| {
            let mut style = (*cc.egui_ctx.global_style()).clone();
            style.spacing.interact_size.y = 42.0;
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            cc.egui_ctx.set_global_style(style);
            Ok(Box::new(Preview {
                snapshot,
                library,
                output,
                frame: 0,
                diagnostics,
            }))
        }),
    )
}

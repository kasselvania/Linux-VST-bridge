//! Native frontend. Desktop fixture is explicit and has no management authority.
use eframe::egui;
mod client;
mod library;
#[path = "../../bridge-manager/src/operator_model.rs"]
mod model;
mod operator;
mod presentation;
struct DesktopFixture {
    count: u32,
    text: String,
}
impl eframe::App for DesktopFixture {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Linux Audio Compatibility Manager");
            ui.label("MF1 desktop fixture — no product operations");
            ui.separator();
            ui.label("Keyboard input");
            ui.text_edit_singleline(&mut self.text);
            if ui
                .add_sized([240.0, 48.0], egui::Button::new("Test pointer / touch"))
                .clicked()
            {
                self.count += 1;
            }
            ui.label(format!("Input receipts: {}", self.count));
            ui.label(format!("Display scale: {:.2}", ui.ctx().pixels_per_point()));
            if ui
                .add_sized([240.0, 48.0], egui::Button::new("Close fixture"))
                .clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
fn main() -> eframe::Result {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let fixture = args == ["--desktop-fixture"];
    if !args.is_empty() && !fixture {
        eprintln!("This frontend accepts no command, path or operation arguments.");
        std::process::exit(64);
    }
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([560.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Linux Audio Compatibility Manager",
        options,
        Box::new(|cc| {
            let mut style = (*cc.egui_ctx.global_style()).clone();
            style.spacing.interact_size.y = 42.0;
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            cc.egui_ctx.set_global_style(style);
            if fixture {
                Ok(Box::new(DesktopFixture {
                    count: 0,
                    text: String::new(),
                }))
            } else {
                Ok(Box::new(operator::Operator::new(&cc.egui_ctx)))
            }
        }),
    )
}

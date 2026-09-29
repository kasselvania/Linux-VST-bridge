//! Source preview of the explicit system-package first-run screen.
//! Usage: OUTPUT.png [WIDTH] [first|legacy|stop|activate|attention]
#[path = "../src/bootstrap.rs"]
#[allow(dead_code)]
mod bootstrap;

use eframe::egui;

struct Preview {
    bootstrap: bootstrap::Bootstrap,
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
        self.bootstrap.ui(ui, frame);
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
    let bootstrap = match args.get(2).map(String::as_str).unwrap_or("first") {
        "first" => bootstrap::Bootstrap::new(),
        "legacy" => bootstrap::Bootstrap::legacy_adoptable_preview(),
        "stop" => bootstrap::Bootstrap::repair_active_preview(),
        "activate" => bootstrap::Bootstrap::needs_activation(),
        "attention" => bootstrap::Bootstrap::attention("package_transition_needs_recovery".into()),
        other => panic!("unknown state: {other}"),
    };
    eframe::run_native(
        "First-run source preview",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([width, 720.0])
                .with_title("First-run source preview"),
            ..Default::default()
        },
        Box::new(move |_cc| {
            Ok(Box::new(Preview {
                bootstrap,
                output,
                frame: 0,
            }))
        }),
    )
}

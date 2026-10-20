//! The screen model stays toolkit-free and reads only the manager's typed view.
const SOURCES: [(&str, &str); 5] = [
    ("mod.rs", include_str!("mod.rs")),
    ("offers.rs", include_str!("offers.rs")),
    ("plugins.rs", include_str!("plugins.rs")),
    ("product.rs", include_str!("product.rs")),
    ("settings.rs", include_str!("settings.rs")),
];

#[test]
fn screen_model_names_no_ui_toolkit_and_no_window_module() {
    for (file, source) in SOURCES {
        let model = source;
        for toolkit in ["egui", "eframe", "epaint", "winit", "glow", "makepad", "rfd"] {
            assert!(!model.to_lowercase().contains(toolkit), "{file} names {toolkit}");
        }
        for (index, _) in model.match_indices("crate::") {
            let path = &model[index + "crate::".len()..];
            assert!(path.starts_with("model") || path.starts_with("screen_model"),
                "{file} reaches outside the manager model: crate::{}", &path[..path.len().min(30)]);
        }
    }
}

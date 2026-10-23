//! The window's shared theme and the facts it reads from this computer: its
//! own build and the distribution. Every screen's words live in the screen model.
use eframe::egui;

/// Shared readable, touch-sized presentation for the ordinary app and previews.
pub fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.spacing.interact_size.y = 44.0;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    for (kind, size) in [(egui::TextStyle::Heading, 23.0), (egui::TextStyle::Body, 15.0),
        (egui::TextStyle::Button, 14.0), (egui::TextStyle::Small, 13.0)] {
        style.text_styles.insert(kind, egui::FontId::proportional(size));
    }
    ctx.set_global_style(style);
    let mut visuals = ctx.global_style().visuals.clone();
    if visuals.dark_mode {
        visuals.override_text_color = Some(egui::Color32::from_rgb(224, 227, 232));
        visuals.panel_fill = egui::Color32::from_rgb(24, 27, 32);
        visuals.faint_bg_color = egui::Color32::from_rgb(32, 37, 44);
        visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(71, 79, 91);
    } else {
        visuals.override_text_color = Some(egui::Color32::from_rgb(34, 42, 54));
        visuals.panel_fill = egui::Color32::from_rgb(248, 250, 253);
        visuals.faint_bg_color = egui::Color32::from_rgb(236, 241, 248);
    }
    ctx.set_visuals(visuals);
}

pub fn primary_button(label: impl Into<egui::WidgetText>, ui: &egui::Ui) -> egui::Button<'static> {
    egui::Button::new(label).fill(ui.visuals().selection.bg_fill)
        .min_size(egui::vec2(0.0, 44.0))
}

/// This window's own build, from its generation record, read once. It names
/// a version before the manager answers.
pub fn own_version() -> Option<&'static str> {
    static VERSION: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| std::env::current_exe().ok()?.parent()
        .and_then(linux_vst_bridge::package_version::generation_version)).as_deref()
}

/// The distribution's own name from os-release, without user or path detail.
pub fn os_release_name(text: &str) -> Option<String> {
    let value = |key: &str| text.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .map(|v| v.trim().trim_matches(['"', '\'']).to_owned())
        .filter(|v| !v.is_empty() && v.len() <= 80 && !v.chars().any(char::is_control));
    value("PRETTY_NAME").or_else(|| Some(match (value("NAME")?, value("VERSION_ID")) {
        (name, Some(version)) => format!("{name} {version}"),
        (name, None) => name,
    }))
}

/// This computer's distribution, read once.
pub fn distribution() -> Option<&'static str> {
    static NAME: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    NAME.get_or_init(|| os_release_name(&linux_vst_bridge::read_os_release())).as_deref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_name_prefers_the_pretty_name() {
        assert_eq!(os_release_name("NAME=\"CachyOS Linux\"\nPRETTY_NAME=\"CachyOS\"\nID=cachyos\n").as_deref(),
            Some("CachyOS"));
        assert_eq!(os_release_name("NAME=\"SteamOS\"\nVERSION_ID=3.8\n").as_deref(), Some("SteamOS 3.8"));
        assert_eq!(os_release_name("ID=unknown\n"), None);
    }
}

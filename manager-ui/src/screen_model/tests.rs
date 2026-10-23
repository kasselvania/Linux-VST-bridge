//! The screen model stays toolkit-free and reads only the manager's typed view.
use std::path::Path;

/// The source without its `#[cfg(test)] mod tests { ... }` block. Code after
/// that block is still checked.
fn without_test_module(source: &str) -> String {
    const START: &str = "#[cfg(test)]\nmod tests {";
    let Some(start) = source.find(START) else { return source.to_owned() };
    let mut depth = 0usize;
    for (offset, c) in source[start + START.len() - 1..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = start + START.len() - 1 + offset + 1;
                    return format!("{}{}", &source[..start], without_test_module(&source[end..]));
                }
            }
            _ => {}
        }
    }
    panic!("unterminated test module");
}

/// What the screen model may use from the manager crate.
const MANAGER_PATHS: [&str; 4] = ["operator_model", "operator_lock::is_busy_text",
    "operator_lock::busy_sentence_naming", "update_feed::display_version"];

/// A path after `linux_vst_bridge::` names exactly one allowed item.
fn allowed_manager_path(path: &str) -> bool {
    MANAGER_PATHS.iter().any(|allowed| path.starts_with(allowed)
        && !path[allowed.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

#[test]
fn screen_model_names_no_ui_toolkit_and_no_window_module() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/screen_model");
    let mut checked = 0;
    for entry in std::fs::read_dir(&directory).unwrap() {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().is_none_or(|extension| extension != "rs") || file == "tests.rs" { continue; }
        // Unit tests may use the preview's sample states; the model may not.
        let model = without_test_module(&std::fs::read_to_string(&path).unwrap());
        for toolkit in ["egui", "eframe", "epaint", "winit", "glow", "makepad", "rfd"] {
            assert!(!model.to_lowercase().contains(toolkit), "{file} names {toolkit}");
        }
        assert!(!model.contains("super::super"), "{file} reaches outside the screen model through super::super");
        for (index, _) in model.match_indices("crate::") {
            let path = &model[index + "crate::".len()..];
            assert!(path.starts_with("model") || path.starts_with("screen_model"),
                "{file} reaches outside the manager model: crate::{}", &path[..path.len().min(30)]);
        }
        // The manager crate only for its wire model and two wording helpers,
        // which move to the model crate later (#504).
        for (index, _) in model.match_indices("linux_vst_bridge::") {
            let path = &model[index + "linux_vst_bridge::".len()..];
            assert!(allowed_manager_path(path),
                "{file} reaches into the manager: linux_vst_bridge::{}", &path[..path.len().min(40)]);
        }
        checked += 1;
    }
    assert!(checked >= 5, "only {checked} screen-model files checked");
}

#[test]
fn the_guard_sees_code_after_a_test_module() {
    let source = "fn a() {}\n#[cfg(test)]\nmod tests {\n fn b() { if true { egui(); } }\n}\nfn c() { egui(); }\n";
    let model = without_test_module(source);
    assert!(model.contains("fn c()") && !model.contains("fn b()"));
}

#[test]
fn the_guard_allows_only_the_listed_manager_paths() {
    let allowed = allowed_manager_path;
    for path in ["operator_model::Snapshot", "operator_lock::is_busy_text(code)", "update_feed::display_version;"] {
        assert!(allowed(path), "{path}");
    }
    for path in ["operator_lock::LockBusy", "operator_lock::is_busy_texts", "update_feed::check(", "valid_hex(", "{operator_model}"] {
        assert!(!allowed(path), "{path}");
    }
}

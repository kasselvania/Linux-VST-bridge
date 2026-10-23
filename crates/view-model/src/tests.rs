//! The view model stays toolkit-free and apart from the manager service: it
//! reads only the wire model, and only tests and previews use its samples.
use std::collections::BTreeSet;
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

/// What the view model may use from the wire-model crate: the model and the
/// shared words, not the transports that reach the service.
const MODEL_PATHS: [&str; 2] = ["operator_model", "wording"];

/// A path after `lvb_model::` names exactly one allowed module.
fn allowed_model_path(path: &str) -> bool {
    MODEL_PATHS.iter().any(|allowed| path.starts_with(allowed)
        && !path[allowed.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn view_model_names_no_ui_toolkit_and_not_the_manager() {
    let mut checked = 0;
    for entry in std::fs::read_dir(manifest_dir().join("src")).unwrap() {
        let path = entry.unwrap().path();
        let file = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.extension().is_none_or(|extension| extension != "rs") || file == "tests.rs" { continue; }
        // Tests included: nothing here may name a toolkit or the manager crate.
        let source = std::fs::read_to_string(&path).unwrap();
        for toolkit in ["egui", "eframe", "epaint", "winit", "glow", "makepad", "rfd"] {
            assert!(!source.to_lowercase().contains(toolkit), "{file} names {toolkit}");
        }
        for manager in ["linux_vst_bridge", "linux-vst-bridge", "bridge-manager", "bridge_manager"] {
            assert!(!source.contains(manager), "{file} names the manager: {manager}");
        }
        let model = without_test_module(&source);
        for (index, _) in model.match_indices("lvb_model::") {
            let path = &model[index + "lvb_model::".len()..];
            assert!(allowed_model_path(path),
                "{file} reaches past the wire model: lvb_model::{}", &path[..path.len().min(40)]);
        }
        // Unit tests may use the sample states; the screens may not.
        if file != "samples.rs" {
            assert!(!model.contains("samples::"), "{file} builds a screen from sample states");
        }
        checked += 1;
    }
    assert!(checked >= 5, "only {checked} view-model files checked");
    let lib = std::fs::read_to_string(manifest_dir().join("src/lib.rs")).unwrap();
    assert!(lib.contains("#[cfg(any(test, feature = \"samples\"))]\npub mod samples;"),
        "the sample states are built only for tests and the samples feature");
}

/// The crate names in one lockfile.
fn locked(path: &Path) -> BTreeSet<String> {
    std::fs::read_to_string(path).unwrap().lines()
        .filter_map(|line| line.strip_prefix("name = \"")?.strip_suffix('"').map(str::to_owned))
        .collect()
}

#[test]
fn view_model_depends_on_the_wire_model_alone() {
    let manifest = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    let mut table = None;
    let mut dependencies = BTreeSet::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            assert!(!line.contains("dependencies") || line == "[dependencies]",
                "only [dependencies] may name crates: {line}");
            table = Some(line.to_owned());
        } else if table.as_deref() == Some("[dependencies]") && !line.is_empty() && !line.starts_with('#') {
            dependencies.insert(line.split(['=', ' ']).next().unwrap().to_owned());
        }
    }
    assert_eq!(dependencies, BTreeSet::from(["lvb-model".to_owned(), "serde_json".to_owned()]));
    // Everything this crate builds with, the wire model already builds with:
    // no service, no toolkit and nothing else arrives through a dependency.
    let mut expected = locked(&manifest_dir().join("../model/Cargo.lock"));
    expected.insert("lvb-view-model".into());
    assert_eq!(locked(&manifest_dir().join("Cargo.lock")), expected);
}

#[test]
fn the_guard_sees_code_after_a_test_module() {
    let source = "fn a() {}\n#[cfg(test)]\nmod tests {\n fn b() { if true { egui(); } }\n}\nfn c() { egui(); }\n";
    let model = without_test_module(source);
    assert!(model.contains("fn c()") && !model.contains("fn b()"));
}

#[test]
fn the_guard_allows_only_the_listed_model_paths() {
    let allowed = allowed_model_path;
    for path in ["operator_model::Snapshot", "operator_model as model;", "wording::is_busy_text(code)",
        "wording`)"] {
        assert!(allowed(path), "{path}");
    }
    for path in ["contract::Service", "operator_models::Snapshot", "wordings::x", "{operator_model}"] {
        assert!(!allowed(path), "{path}");
    }
}

//! Test instrumentation: join real Windows fixture observations and PE bytes
//! using the production parser and report. Never an activation/import route.
use linux_vst_bridge::{
    graphics::{assessment, pe},
    *,
};
use std::path::Path;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    require(
        args.len() == 3,
        "usage: graphics_assessment_report MODULE WINDOWS_REFERENCE_REPORT",
    )?;
    let raw: serde_json::Value = read_json(Path::new(&args[2]))?;
    let context: assessment::Context = serde_json::from_value(raw["context"].clone())?;
    require(
        digest(Path::new(&args[1]))? == context.module_sha256,
        "fixture_module_identity",
    )?;
    let imports = pe::inspect(&mut file(Path::new(&args[1]))?)?;
    let mut result = assessment::assemble(context, imports, &raw, 0)?;
    result.scope = "native Windows CI reference; no Proton or commercial qualification";
    // Reference 1 imports Direct3D 11, so it is loaded with the module;
    // reference 2 delay-loads OpenGL, so it first appears with the editor.
    let library = if args[1].ends_with("-1.vst3") {
        Some((assessment::Library::D3d11, "loaded_before_editor"))
    } else if args[1].ends_with("-2.vst3") {
        Some((assessment::Library::OpenGl, "loaded_during_editor"))
    } else {
        None
    };
    if let Some((library, loaded)) = library {
        require(
            result.findings.iter().any(|f| {
                f.library == library
                    && f.probe_status == "passed"
                    && f.sources.iter().any(|s| s.ends_with("import_hint"))
                    && f.sources.contains(&loaded)
                    && result.recommendation.editor_loaded.contains(&library)
            }),
            "fixture_joined_graphics",
        )?;
    }
    if args[1].ends_with("-5.vst3") {
        require(!result.editor.closed
            && result.editor_retirement == assessment::EditorRetirement::TimedOut
            && result.qualification == "unqualified", "fixture_failed_editor_retirement")?;
    }
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

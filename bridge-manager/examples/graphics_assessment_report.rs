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
    let library = if args[1].ends_with("-1.vst3") {
        Some(assessment::Library::D3d11)
    } else if args[1].ends_with("-2.vst3") {
        Some(assessment::Library::OpenGl)
    } else {
        None
    };
    if let Some(library) = library {
        require(
            result.findings.iter().any(|f| {
                f.library == library
                    && f.capability == "passed"
                    && f.sources.iter().any(|s| s.ends_with("import_hint"))
                    && f.sources.contains(&"loaded_during_editor")
            }),
            "fixture_joined_graphics",
        )?;
    }
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

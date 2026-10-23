//! Shared observation vocabulary. No product/distro selector and no activation authority.
use crate::{operator_model::GraphicsBackend, require, valid_hex, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Library {
    D3d9,
    D3d11,
    D3d12,
    Dxgi,
    Direct2d,
    DirectComposition,
    OpenGl,
    Vulkan,
    WebView2,
    AngleEgl,
    AngleGles,
}
impl Library {
    pub fn from_dll(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "d3d9.dll" => Self::D3d9,
            "d3d11.dll" => Self::D3d11,
            "d3d12.dll" => Self::D3d12,
            "dxgi.dll" => Self::Dxgi,
            "d2d1.dll" => Self::Direct2d,
            "dcomp.dll" => Self::DirectComposition,
            "opengl32.dll" => Self::OpenGl,
            "vulkan-1.dll" => Self::Vulkan,
            "webview2loader.dll" => Self::WebView2,
            "libegl.dll" => Self::AngleEgl,
            "libglesv2.dll" => Self::AngleGles,
            _ => return None,
        })
    }
    fn probe(self) -> Option<&'static str> {
        match self {
            Self::D3d11 | Self::Dxgi => Some("d3d11_default"),
            Self::OpenGl => Some("opengl"),
            Self::DirectComposition => Some("direct_composition"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Context {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_backend: Option<crate::operator_model::GraphicsBackend>,
    pub module_sha256: String,
    pub class_id: String,
    pub runner_fingerprint: String,
    pub environment_fingerprint: String,
    pub environment_revision: u64,
    pub host_sha256: String,
    pub host_source_sha256: String,
    pub requested_graphics: String,
}
impl Context {
    pub fn for_configuration(environment: &crate::Environment, module: &crate::Artifact,
        class_id: &str, host: &crate::Artifact, source: &str,
        compatibility: &crate::Compatibility) -> Result<Self> {
        Ok(Self {
            configuration_fingerprint: Some(crate::preparation::key(compatibility)?),
            requested_backend: compatibility.graphics,
            module_sha256: module.sha256.clone(), class_id: class_id.into(),
            runner_fingerprint: crate::catalogue::runner_key(&environment.runner)?,
            environment_fingerprint: crate::preparation::key(environment)?,
            environment_revision: environment.revision, host_sha256: host.sha256.clone(),
            host_source_sha256: source.into(),
            requested_graphics: super::requested_configuration(environment.runner.policy.as_ref(),
                compatibility.graphics).into(),
        })
    }
    pub fn fingerprint(&self) -> Result<String> {
        require(
            [
                &self.module_sha256,
                &self.runner_fingerprint,
                &self.environment_fingerprint,
                &self.host_sha256,
                &self.host_source_sha256,
            ]
            .iter()
            .all(|s| valid_hex(s, 64))
                && valid_hex(&self.class_id, 32)
                && self.configuration_fingerprint.as_ref().is_none_or(|v| valid_hex(v, 64)),
            "graphics_context_identity",
        )?;
        Ok(crate::hex(&Sha256::digest(serde_json::to_vec(self)?)))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    pub api: String,
    pub status: String,
    pub rendering: String,
    pub renderer: Option<String>,
    pub vendor_id: Option<u32>,
    pub device_id: Option<u32>,
    pub driver_version: Option<String>,
    pub feature_level: Option<u32>,
    /// Which `d3d11.dll` the Direct3D probes ran on: Wine's built-in or another
    /// provider. Observed from the loaded module, never from the requested
    /// launch settings. Absent on other probes and from hosts that predate it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
}
impl Probe {
    fn wine_builtin(&self) -> bool { self.implementation.as_deref() == Some("wine_builtin") }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Editor {
    pub status: String,
    pub before: Vec<Library>,
    /// Every graphics library in the process while the editor was open. This
    /// includes everything in `before`; use `opened_with_editor` for the change.
    pub during: Vec<Library>,
    pub closed: bool,
}
impl Editor {
    /// Libraries that appeared while the editor opened.
    pub fn opened_with_editor(&self) -> impl Iterator<Item = Library> + '_ {
        self.during.iter().copied().filter(|library| !self.before.contains(library))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorRetirement {
    Closed,
    TimedOut,
    Failed,
}
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub library: Library,
    pub sources: Vec<&'static str>,
    pub probe_api: Option<String>,
    pub probe_status: String,
}
/// What the editor needs from the runner, decided by rule from the libraries
/// the plug-in actually loaded and this launch's probes. Import hints alone
/// never decide; a plug-in without an editor decides nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Requirement {
    /// The available observations indicate no graphics change. Visual rendering remains untested.
    None,
    /// Direct3D 11 is used, the default path failed its probe and is not already Wine's built-in:
    /// Wine's built-in D3D11/DXGI is the next launch.
    WineD3d11,
    /// The editor's own module uses DirectComposition and this launch cannot create a
    /// composition device: the blank-editor class.
    DirectCompositionRunner,
    /// The plug-in opened no editor in this inspection.
    EditorAbsent,
    /// An editor dependency is unassessed, or its path failed a probe with no further launch choice.
    Undetermined,
}
#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub requirement: Requirement,
    /// Libraries attributable to the plug-in: first loaded while its editor
    /// opened, or loaded earlier and named in its module's import table.
    pub editor_loaded: Vec<Library>,
    pub reason: &'static str,
    pub action: &'static str,
}
pub fn recommend(editor: &Editor, probes: &[Probe], imports: &super::pe::Imports,
    requested_backend: Option<GraphicsBackend>) -> Recommendation {
    let passed = |api: &str| probes.iter().any(|p| p.api == api && p.status == "passed");
    let imported = |l: &Library| imports.ordinary.contains(l) || imports.delayed.contains(l);
    // `during` repeats everything already loaded before the editor opened: the
    // host's own libraries, and whatever those pulled in. Only the change is the
    // editor's; an earlier library is the plug-in's when its module imports it.
    let loaded: Vec<Library> = editor.opened_with_editor()
        .chain(editor.before.iter().copied().filter(imported))
        .collect::<BTreeSet<_>>().into_iter().collect();
    let builtin_d3d11 = probes.iter().any(|p| p.api == "d3d11_default" && p.wine_builtin());
    let has = |l: Library| loaded.contains(&l);
    let finish = |requirement, reason, action| Recommendation { requirement, editor_loaded: loaded.clone(), reason, action };
    if editor.status != "opened" {
        return finish(Requirement::EditorAbsent,
            "The plug-in opened no editor in this inspection.",
            "Nothing to change for graphics.");
    }
    if has(Library::DirectComposition) && !passed("direct_composition") {
        return finish(Requirement::DirectCompositionRunner,
            "The editor's module uses DirectComposition and this launch cannot create a composition device. An editor that presents through it shows blank or white.",
            "Select the DirectComposition-capable runner for this plug-in and prepare again.");
    }
    if (has(Library::D3d11) || has(Library::Dxgi)) && !passed("d3d11_default") {
        return if requested_backend == Some(GraphicsBackend::WineD3d11) {
            finish(Requirement::Undetermined,
                "The editor draws through Direct3D 11 and it failed its rendering probe with Wine's built-in path too.",
                "Keep the recorded observations and report the editor as not displaying on this runner.")
        } else if builtin_d3d11 {
            // The default launch is already Wine's built-in Direct3D 11, so
            // preparing that trial would repeat this exact launch.
            finish(Requirement::Undetermined,
                "The editor draws through Direct3D 11, this launch already uses Wine's built-in Direct3D 11, and it failed its rendering probe.",
                "Keep the recorded observations and report the editor as not displaying on this runner.")
        } else {
            finish(Requirement::WineD3d11,
                "The editor draws through Direct3D 11 and the default path failed its rendering probe on this launch.",
                "Prepare with Wine's built-in Direct3D 11 and DXGI for this plug-in host, then assess again.")
        };
    }
    if has(Library::OpenGl) && !passed("opengl") {
        return finish(Requirement::Undetermined,
            "The editor draws through OpenGL and it failed its rendering probe on this launch.",
            "Keep the recorded observations and report the editor as not displaying on this runner.");
    }
    if has(Library::WebView2) {
        return finish(Requirement::Undetermined,
            "A WebView2-related library loaded with the editor. This inspection does not establish whether the editor requires WebView2 or whether it is available.",
            "Check the actual editor before changing settings. Audio and state recall need their own tests.");
    }
    finish(Requirement::None,
        "The available runtime probes indicate no graphics change. Opening the editor does not establish that it rendered correctly.",
        "Check the actual editor; audio and state recall need their own tests.")
}
#[derive(Debug, Clone, Serialize)]
pub struct Assessment {
    pub applied_launch: Value,
    pub schema: u32,
    pub context: Context,
    pub context_fingerprint: String,
    pub observations_fingerprint: String,
    pub observed_at: u64,
    pub scope: &'static str,
    pub imports: super::pe::Imports,
    pub editor: Editor,
    pub editor_retirement: EditorRetirement,
    pub runtime_probes: Vec<Probe>,
    pub findings: Vec<Finding>,
    pub recommendation: Recommendation,
    pub editor_device: Option<String>,
    pub child_renderers: &'static str,
    pub qualification: &'static str,
    pub next_action: &'static str,
}
/// Reapply current advice to retained facts without probing or rewriting the run.
pub fn project_recommendation(value: &mut Value) -> Result<()> {
    let context: Context = serde_json::from_value(value["context"].clone())?;
    let editor: Editor = serde_json::from_value(value["editor"].clone())?;
    let probes: Vec<Probe> = serde_json::from_value(value["runtime_probes"].clone())?;
    let imports: super::pe::Imports = serde_json::from_value(value["imports"].clone())?;
    let recommendation = recommend(&editor, &probes, &imports, context.requested_backend);
    value["next_action"] = Value::from(recommendation.action);
    value["recommendation"] = serde_json::to_value(recommendation)?;
    Ok(())
}

fn bounded_text(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|s| {
        !s.is_empty()
            && s.len() <= 256
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || " ._()-:+,".contains(c))
    })
}
fn one<'a>(rows: &'a [Value], state: &str) -> Result<&'a Value> {
    let found: Vec<_> = rows.iter().filter(|r| r["state"] == state).collect();
    require(found.len() == 1, "graphics_observation_absent_or_duplicate")?;
    Ok(found[0])
}

// The supervisor composes graphics, accessibility and its builtin audio helper.
// Check the effective graphics assignments, not one serialization of the list.
// Conflicting assignments are refused rather than guessing their precedence.
fn wine_d3d11_builtins(value: &str) -> bool {
    let mut selected = [false; 2];
    for entry in value.split(';').filter(|entry| !entry.trim().is_empty()) {
        let Some((names, order)) = entry.split_once('=') else { return false };
        for name in names.split(',') {
            let name = name.trim().to_ascii_lowercase();
            let name = name.rsplit(['\\', '/']).next().unwrap_or("");
            let name = name.strip_suffix(".dll").unwrap_or(name);
            let name = if name == "*" { name } else { name.strip_prefix('*').unwrap_or(name) };
            if name.is_empty() || (name != "*" && name.contains('*')) { return false; }
            if name == "*" {
                if order.trim() != "b" { return false; }
            } else if let Some(index) = ["d3d11", "dxgi"].iter().position(|target| *target == name) {
                if order.trim() != "b" { return false; }
                selected[index] = true;
            }
        }
    }
    selected.into_iter().all(|present| present)
}

fn editor_retirement(rows: &[Value], graphics: &Value, report: &Value,
    editor: &mut Editor) -> Result<EditorRetirement> {
    let closes: Vec<_> = rows.iter().filter(|row| row["state"] == "graphics_editor_closed").collect();
    if graphics["schema"] == 1 {
        require(editor.closed && closes.is_empty(), "graphics_editor_observation")?;
        return Ok(EditorRetirement::Closed);
    }
    require(graphics["schema"] == 2 && !editor.closed && closes.len() <= 1,
        "graphics_editor_observation")?;
    if let Some(close) = closes.first() {
        require(close["schema"] == 1 && close["closed"].is_boolean(), "graphics_editor_retirement")?;
        editor.closed = close["closed"] == true;
        return Ok(if editor.closed { EditorRetirement::Closed } else { EditorRetirement::Failed });
    }
    require(report["error"] == "TimeoutError: Windows call deadline: closeGraphicsEditor"
        && rows.iter().rev().find(|row| row["state"] == "ap8_call")
            .is_some_and(|row| row["operation"] == "closeGraphicsEditor"),
        "graphics_editor_retirement")?;
    Ok(EditorRetirement::TimedOut)
}
pub fn assemble(
    context: Context,
    imports: super::pe::Imports,
    report: &Value,
    at: u64,
) -> Result<Assessment> {
    let applied_launch = report["graphics_configuration"].clone();
    if context.configuration_fingerprint.is_some() {
        require(applied_launch["requested_backend"] == serde_json::to_value(context.requested_backend)?
            && applied_launch["scope"] == "host_process_and_children"
            && applied_launch["renderer_observed"] == false,
            "graphics_launch_configuration_missing_or_changed")?;
        if context.requested_backend == Some(crate::operator_model::GraphicsBackend::WineD3d11) {
            require(applied_launch["dll_overrides"].as_str().is_some_and(wine_d3d11_builtins),
                "graphics_launch_override_not_applied")?;
        }
    }
    require(
        report["cleanup_confirmed"] == true
            && report["transport_retired"] == true
            && report["gated"] == true,
        "graphics_assessment_incomplete",
    )?;
    let rows = report["records"]
        .as_array()
        .ok_or("graphics_assessment_records")?;
    require(rows.len() <= 4096, "graphics_assessment_record_bound")?;
    let binding = one(rows, "readiness_announced")?;
    require(
        binding["module_sha256"] == context.module_sha256
            && binding["scanner_sha256"] == context.host_sha256
            && binding["implementation_source_manifest_sha256"] == context.host_source_sha256
            && binding["mode"] == "graphics-assessment",
        "graphics_assessment_binding",
    )?;
    require(
        one(rows, "ap11_class")?["class_id"]
            .as_str()
            .is_some_and(|s| s.eq_ignore_ascii_case(&context.class_id)),
        "graphics_assessment_class",
    )?;
    let graphics = one(rows, "graphics_assessment")?;
    let mut editor: Editor = serde_json::from_value(graphics["editor"].clone())?;
    let editor_retirement = editor_retirement(rows, graphics, report, &mut editor)?;
    if editor_retirement == EditorRetirement::Closed {
        require(report["error"].is_null()
            && one(rows, "ap8_inspection_closed")?["exit_code"] == 0
            && one(rows, "scanner_completed")?["inspection_complete"] == true,
            "graphics_assessment_terminal")?;
    } else {
        // A contained editor-close failure does not erase observations already
        // made on this exact launch. It also cannot become a successful scan.
        require(report["error"].as_str().is_some_and(|error| !error.is_empty())
            && !rows.iter().any(|row| matches!(row["state"].as_str(),
                Some("ap8_inspection_closed" | "scanner_completed"))),
            "graphics_assessment_terminal")?;
    }
    require(
        matches!(editor.status.as_str(), "opened" | "unavailable")
            && editor.before.len() <= 11
            && editor.during.len() <= 11
            && editor.before.iter().collect::<BTreeSet<_>>().len() == editor.before.len()
            && editor.during.iter().collect::<BTreeSet<_>>().len() == editor.during.len(),
        "graphics_editor_observation",
    )?;
    let runtime = one(rows, "graphics_runtime")?;
    // Schema 2 hosts report the Direct3D 11 provider and classify software
    // rendering from the driver's own renderer string.
    let reports_provider = runtime["schema"] == 2;
    require(runtime["schema"] == 1 || reports_provider, "graphics_runtime_schema")?;
    let mut probes: Vec<Probe> = serde_json::from_value(runtime["probes"].clone())?;
    require(probes.len() == 4, "graphics_probe_count")?;
    let expected = BTreeSet::from([
        "d3d11_default",
        "d3d11_warp",
        "opengl",
        "direct_composition",
    ]);
    require(
        probes
            .iter()
            .map(|p| p.api.as_str())
            .collect::<BTreeSet<_>>()
            == expected,
        "graphics_probe_apis",
    )?;
    for p in &mut probes {
        require(
            matches!(
                p.status.as_str(),
                "passed" | "unavailable" | "readback_failed"
            ) && matches!(
                p.rendering.as_str(),
                "reported_hardware" | "reported_software" | "unknown"
            ) && bounded_text(&p.renderer)
                && bounded_text(&p.driver_version),
            "graphics_probe_fields",
        )?;
        require(
            match p.implementation.as_deref() {
                None => !reports_provider || !p.api.starts_with("d3d11_") || p.status == "unavailable",
                Some("wine_builtin" | "other") => reports_provider && p.api.starts_with("d3d11_"),
                Some(_) => false,
            },
            "graphics_probe_implementation",
        )?;
        // An earlier host lost any renderer name containing a comma, which is
        // every Mesa driver, and so could not have checked it for software.
        if !reports_provider && p.api == "opengl" && p.renderer.is_none() && p.rendering == "reported_hardware" {
            p.rendering = "unknown".into();
        }
        if p.status == "passed"
            && p.renderer
                .as_ref()
                .is_some_and(|r| super::software_renderer(r))
        {
            p.rendering = "reported_software".into();
        }
        require(
            p.status == "passed" || p.rendering == "unknown",
            "graphics_probe_failure_claim",
        )?;
        // A WARP request that the provider did not honour is not a software
        // result, and Wine's built-in Direct3D reports a stand-in adapter, not
        // the device that drew. Neither may be called hardware.
        require(
            (p.api != "d3d11_warp" && !p.wine_builtin()) || p.rendering != "reported_hardware",
            "graphics_unfounded_hardware_claim",
        )?;
    }
    let all: BTreeSet<_> = imports
        .ordinary
        .iter()
        .chain(&imports.delayed)
        .chain(&editor.before)
        .chain(&editor.during)
        .copied()
        .collect();
    let findings = all
        .into_iter()
        .map(|library| {
            let mut sources = Vec::new();
            if imports.ordinary.contains(&library) {
                sources.push("ordinary_import_hint");
            }
            if imports.delayed.contains(&library) {
                sources.push("delayed_import_hint");
            }
            if editor.before.contains(&library) {
                sources.push("loaded_before_editor");
            }
            if editor.opened_with_editor().any(|opened| opened == library) {
                sources.push("loaded_during_editor");
            }
            let capability = library
                .probe()
                .and_then(|api| probes.iter().find(|p| p.api == api))
                .map(|p| p.status.clone())
                .unwrap_or_else(|| "not_tested".into());
            Finding {
                library,
                sources,
                probe_api: library.probe().map(str::to_owned),
                probe_status: capability,
            }
        })
        .collect();
    let recommendation = recommend(&editor, &probes, &imports, context.requested_backend);
    let next_action = recommendation.action;
    let context_fingerprint = context.fingerprint()?;
    let observations_fingerprint = crate::hex(&Sha256::digest(serde_json::to_vec(&(
        &context_fingerprint,
        &imports,
        &editor,
        &editor_retirement,
        &probes,
        &applied_launch,
    ))?));
    Ok(Assessment {schema:1,applied_launch,context_fingerprint,observations_fingerprint,context,observed_at:at,
        scope:"isolated inspection in selected environment; independent probe contexts are not the plug-in rendering context",
        imports,editor,editor_retirement,runtime_probes:probes,findings,recommendation,editor_device:None,child_renderers:"not_observed",
        qualification:"unqualified",next_action})
}

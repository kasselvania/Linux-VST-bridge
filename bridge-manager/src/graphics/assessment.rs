//! Shared observation vocabulary. No product/distro selector and no activation authority.
use crate::{require, valid_hex, Result};
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
                && valid_hex(&self.class_id, 32),
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
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Editor {
    pub status: String,
    pub before: Vec<Library>,
    pub during: Vec<Library>,
    pub closed: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub library: Library,
    pub sources: Vec<&'static str>,
    pub probe_api: Option<String>,
    pub probe_status: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Assessment {
    pub schema: u32,
    pub context: Context,
    pub context_fingerprint: String,
    pub observations_fingerprint: String,
    pub observed_at: u64,
    pub scope: &'static str,
    pub imports: super::pe::Imports,
    pub editor: Editor,
    pub runtime_probes: Vec<Probe>,
    pub findings: Vec<Finding>,
    pub editor_device: Option<String>,
    pub child_renderers: &'static str,
    pub qualification: &'static str,
    pub next_action: &'static str,
}
fn bounded_text(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|s| {
        !s.is_empty()
            && s.len() <= 256
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || " ._()-:+".contains(c))
    })
}
fn one<'a>(rows: &'a [Value], state: &str) -> Result<&'a Value> {
    let found: Vec<_> = rows.iter().filter(|r| r["state"] == state).collect();
    require(found.len() == 1, "graphics_observation_absent_or_duplicate")?;
    Ok(found[0])
}
pub fn assemble(
    context: Context,
    imports: super::pe::Imports,
    report: &Value,
    at: u64,
) -> Result<Assessment> {
    require(
        report["cleanup_confirmed"] == true
            && report["transport_retired"] == true
            && report["gated"] == true
            && report["error"].is_null(),
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
    require(
        one(rows, "ap8_inspection_closed")?["exit_code"] == 0
            && one(rows, "scanner_completed")?["inspection_complete"] == true,
        "graphics_assessment_terminal",
    )?;
    let graphics = one(rows, "graphics_assessment")?;
    require(graphics["schema"] == 1, "graphics_assessment_schema")?;
    let editor: Editor = serde_json::from_value(graphics["editor"].clone())?;
    require(
        matches!(editor.status.as_str(), "opened" | "unavailable")
            && editor.closed
            && editor.before.len() <= 11
            && editor.during.len() <= 11
            && editor.before.iter().collect::<BTreeSet<_>>().len() == editor.before.len()
            && editor.during.iter().collect::<BTreeSet<_>>().len() == editor.during.len(),
        "graphics_editor_observation",
    )?;
    let runtime = one(rows, "graphics_runtime")?;
    require(runtime["schema"] == 1, "graphics_runtime_schema")?;
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
        require(
            p.api != "d3d11_warp" || p.status != "passed" || p.rendering == "reported_software",
            "graphics_warp_claim",
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
            if editor.during.contains(&library) {
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
    let context_fingerprint = context.fingerprint()?;
    let observations_fingerprint = crate::hex(&Sha256::digest(serde_json::to_vec(&(
        &context_fingerprint,
        &imports,
        &editor,
        &probes,
    ))?));
    Ok(Assessment {schema:1,context_fingerprint,observations_fingerprint,context,observed_at:at,
        scope:"isolated inspection in selected environment; independent probe contexts are not the plug-in rendering context",
        imports,editor,runtime_probes:probes,findings,editor_device:None,child_renderers:"not_observed",
        qualification:"unqualified",next_action:"Review candidate API observations, then run editor/audio coexistence and recall tests; no settings were selected"})
}

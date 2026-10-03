//! Graphics facts have a context. A host GLX observation is never promoted to
//! evidence about a Windows editor, its browser child, or a Vulkan context.
use crate::RunnerPolicy;
pub mod assessment;
pub mod pe;
#[cfg(test)]
mod tests_assessment {
    include!("graphics/tests.rs");
}

pub fn requested_backend(policy: Option<&RunnerPolicy>) -> &'static str {
    match policy {
        Some(RunnerPolicy::DcompWineBuiltinsReferenceV1) => "WineD3D / OpenGL; Wine built-in DComp, D2D, D3D11 and DXGI",
        Some(RunnerPolicy::X11TouchReleaseV1 | RunnerPolicy::X11TouchRoutingV2) | None =>
            "Runner defaults; no manager graphics override",
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct HostGlx {
    pub vendor: String,
    pub renderer: String,
    pub version: String,
    /// None means the helper did not report acceleration. Direct rendering
    /// alone is deliberately not used as evidence of hardware acceleration.
    pub accelerated: Option<bool>,
}
impl HostGlx {
    pub fn rendering(&self) -> &'static str {
        if self.accelerated == Some(false) || software_renderer(&self.renderer) {
            "software"
        } else if self.accelerated == Some(true) {
            "hardware reported by host GLX"
        } else {
            "unknown"
        }
    }
}
fn software_renderer(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["llvmpipe", "softpipe", "swrast", "software rasterizer"].iter().any(|name|
        lower == *name || lower.strip_prefix(name).is_some_and(|rest| rest.starts_with(' ')))
}

/// Accept only bounded, unambiguous fields from `glxinfo -B`. Ignore display
/// names, paths and extension lists; never export raw helper output.
pub fn parse_host_glx(text: &str) -> Option<HostGlx> {
    if text.len() > 4096 || text.lines().count() > 128 { return None; }
    let (mut vendor, mut renderer, mut version, mut accelerated) = (None, None, None, None);
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once(':') else { continue; };
        let slot = match key {
            "OpenGL vendor string" => &mut vendor,
            "OpenGL renderer string" => &mut renderer,
            "OpenGL version string" => &mut version,
            "Accelerated" => {
                if accelerated.is_some() { return None; }
                accelerated = Some(match value.trim() { "yes" => true, "no" => false, _ => return None });
                continue;
            }
            _ => continue,
        };
        let value = value.trim();
        if slot.is_some() || value.is_empty() || value.len() > 256
            || value.chars().any(char::is_control) { return None; }
        *slot = Some(value.to_owned());
    }
    let result = HostGlx { vendor: vendor?, renderer: renderer?, version: version?, accelerated };
    if result.accelerated == Some(true) && software_renderer(&result.renderer) { return None; }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn output(renderer: &str, extra: &str) -> String {
        format!("name of display: private-display\nOpenGL vendor string: Mesa\nOpenGL renderer string: {renderer}\nOpenGL version string: 4.6 Test\n{extra}")
    }
    #[test]
    fn graphics_policy_is_requested_not_observed() {
        assert!(requested_backend(Some(&RunnerPolicy::DcompWineBuiltinsReferenceV1)).starts_with("WineD3D / OpenGL"));
        assert_eq!(requested_backend(None), requested_backend(Some(&RunnerPolicy::X11TouchRoutingV2)));
    }
    #[test]
    fn graphics_host_hardware_and_software_are_explicit() {
        assert_eq!(parse_host_glx(&output("AMD Test", "Accelerated: yes\n")).unwrap().rendering(), "hardware reported by host GLX");
        for renderer in ["llvmpipe (LLVM test)", "softpipe", "Software Rasterizer"] {
            assert_eq!(parse_host_glx(&output(renderer, "direct rendering: Yes\n")).unwrap().rendering(), "software");
        }
        assert_eq!(parse_host_glx(&output("Unknown device", "Accelerated: no\n")).unwrap().rendering(), "software");
        assert_eq!(parse_host_glx(&output("Unknown device", "direct rendering: Yes\n")).unwrap().rendering(), "unknown");
    }
    #[test]
    fn graphics_missing_ambiguous_and_oversized_facts_are_refused() {
        for text in [String::new(), "OpenGL renderer string: AMD".into(),
            output("llvmpipe", "Accelerated: yes\n"),
            output("AMD", "OpenGL renderer string: Other\n"),
            output("AMD", "Accelerated: yes\nAccelerated: no\n"),
            output("AMD", "Accelerated: maybe\n"),
            output(&"x".repeat(257), ""), output("AMD\u{0}", ""), "x".repeat(4097)] {
            assert!(parse_host_glx(&text).is_none());
        }
    }
}

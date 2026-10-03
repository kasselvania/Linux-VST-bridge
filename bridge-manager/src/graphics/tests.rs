use super::{assessment::*, pe};
use serde_json::{json, Value};
use std::io::Cursor;

fn put16(b: &mut [u8], p: usize, v: u16) {
    b[p..p + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(b: &mut [u8], p: usize, v: u32) {
    b[p..p + 4].copy_from_slice(&v.to_le_bytes());
}
fn image(ordinary: &str, delayed: &str) -> Vec<u8> {
    let mut b = vec![0; 2048];
    b[..2].copy_from_slice(b"MZ");
    put32(&mut b, 60, 128);
    b[128..132].copy_from_slice(b"PE\0\0");
    put16(&mut b, 132, 0x8664);
    put16(&mut b, 134, 1);
    put16(&mut b, 148, 240);
    put16(&mut b, 152, 0x20b);
    put32(&mut b, 152 + 60, 512);
    put32(&mut b, 152 + 108, 16);
    put32(&mut b, 152 + 112 + 8, 0x1000);
    put32(&mut b, 152 + 112 + 12, 40);
    put32(&mut b, 152 + 112 + 13 * 8, 0x1100);
    put32(&mut b, 152 + 112 + 13 * 8 + 4, 64);
    put32(&mut b, 392 + 12, 0x1000);
    put32(&mut b, 392 + 16, 1536);
    put32(&mut b, 392 + 20, 512);
    put32(&mut b, 512 + 12, 0x1200);
    put32(&mut b, 768, 1);
    put32(&mut b, 768 + 4, 0x1300);
    b[1024..1024 + ordinary.len()].copy_from_slice(ordinary.as_bytes());
    b[1280..1280 + delayed.len()].copy_from_slice(delayed.as_bytes());
    b
}
#[test]
fn graphics_pe_ordinary_and_delayed_dependencies_are_hints() {
    let result = pe::inspect(&mut Cursor::new(image("D3D11.dll", "OpenGL32.DLL"))).unwrap();
    assert_eq!(result.ordinary, vec![Library::D3d11]);
    assert_eq!(result.delayed, vec![Library::OpenGl]);
    let result = pe::inspect(&mut Cursor::new(image(
        "private-vendor.dll",
        "WebView2Loader.dll",
    )))
    .unwrap();
    assert!(result.ordinary.is_empty());
    assert_eq!(result.delayed, vec![Library::WebView2]);
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains("private-vendor"));
}
#[test]
fn graphics_pe_rejects_truncation_ambiguous_rvas_and_malformed_directories() {
    let source = image("d3d11.dll", "opengl32.dll");
    for len in 0..source.len() {
        assert!(
            pe::inspect(&mut Cursor::new(&source[..len])).is_err(),
            "{len}"
        );
    }
    for (offset, value) in [
        (512 + 12, 0xfffffff0),
        (152 + 112 + 12, 20),
        (768, 4),
        (392 + 20, 2040),
        (152 + 108, 17),
        (152 + 60, 32),
    ] {
        let mut b = source.clone();
        put32(&mut b, offset, value);
        assert!(pe::inspect(&mut Cursor::new(b)).is_err());
    }
    let mut b = source.clone();
    put16(&mut b, 134, 2);
    let duplicate = b[392..432].to_vec();
    b[432..472].copy_from_slice(&duplicate);
    assert!(pe::inspect(&mut Cursor::new(b)).is_err());
    assert!(pe::inspect(&mut Cursor::new(image("C:\\private.dll", "opengl32.dll"))).is_err());
}
fn context() -> Context {
    Context {
        module_sha256: "11".repeat(32),
        class_id: "AA".repeat(16),
        runner_fingerprint: "22".repeat(32),
        environment_fingerprint: "33".repeat(32),
        environment_revision: 1,
        host_sha256: "44".repeat(32),
        host_source_sha256: "55".repeat(32),
        requested_graphics: "runner defaults".into(),
        configuration_fingerprint: None,
        requested_backend: None,
    }
}
fn report() -> Value {
    let c = context();
    let mut probes = Vec::new();
    for api in [
        "d3d11_default",
        "d3d11_warp",
        "opengl",
        "direct_composition",
    ] {
        probes.push(json!({"api":api,"status":"passed","rendering":if api=="direct_composition" {"unknown"}else{"reported_software"},
            "renderer":null,"vendor_id":null,"device_id":null,"driver_version":null,"feature_level":null}));
    }
    json!({"gated":true,"cleanup_confirmed":true,"transport_retired":true,"error":null,"records":[
        {"state":"readiness_announced","module_sha256":c.module_sha256,"scanner_sha256":c.host_sha256,
            "implementation_source_manifest_sha256":c.host_source_sha256,"mode":"graphics-assessment"},
        {"state":"ap11_class","class_id":c.class_id},
        {"state":"graphics_assessment","schema":1,"editor":{"status":"opened","before":["d3d11"],"during":["d3d11","web_view2"],"closed":true}},
        {"state":"graphics_runtime","schema":1,"probes":probes},
        {"state":"ap8_inspection_closed","exit_code":0},{"state":"scanner_completed","inspection_complete":true}
    ]})
}
#[test]
fn graphics_assessment_reuses_rules_without_promoting_hints_or_probe_devices() {
    let imports = pe::inspect(&mut Cursor::new(image("d3d11.dll", "opengl32.dll"))).unwrap();
    let a = assemble(context(), imports, &report(), 100).unwrap();
    assert_eq!(a.findings.len(), 3);
    assert_eq!(a.qualification, "unqualified");
    assert!(a.editor_device.is_none());
    assert_eq!(
        a.findings
            .iter()
            .find(|f| f.library == Library::WebView2)
            .unwrap()
            .probe_status,
        "not_tested"
    );
    assert_eq!(
        a.findings
            .iter()
            .find(|f| f.library == Library::D3d11)
            .unwrap()
            .sources
            .len(),
        3
    );
    let original = context().fingerprint().unwrap();
    for field in [
        "module_sha256",
        "runner_fingerprint",
        "environment_fingerprint",
        "host_sha256",
        "host_source_sha256",
    ] {
        let mut c = serde_json::to_value(context()).unwrap();
        c[field] = json!("66".repeat(32));
        assert_ne!(
            serde_json::from_value::<Context>(c)
                .unwrap()
                .fingerprint()
                .unwrap(),
            original
        );
    }
}

#[test]
fn candidate_assessment_binds_launch_settings_and_keeps_missing_editor_optional() {
    use crate::preparation::{self as prep, configuration};
    let (f, base) = prep::tests::fixture();
    prep::record_candidate(&f.m, &base).unwrap();
    let original = prep::enable(&f.m, &base, false).unwrap();
    let next = configuration::prepare(&f.m, &base,
        Some(crate::operator_model::GraphicsBackend::WineD3d11), Some(&original)).unwrap();
    let context = configuration::context(&next).unwrap();
    let imports = pe::Imports { machine: 0x8664, ordinary: vec![Library::D3d11], delayed: vec![] };
    let mut raw = report();
    raw["records"][0]["module_sha256"] = json!(context.module_sha256);
    raw["records"][0]["scanner_sha256"] = json!(context.host_sha256);
    raw["records"][0]["implementation_source_manifest_sha256"] = json!(context.host_source_sha256);
    raw["records"][1]["class_id"] = json!(context.class_id);
    raw["records"][2]["editor"]["status"] = json!("unavailable");
    assert!(assemble(context.clone(), imports.clone(), &raw, 100).is_err());
    raw["graphics_configuration"] = json!({"requested_backend":"wine_d3d11", "dll_overrides":"d3d11,dxgi=b",
        "scope":"host_process_and_children","renderer_observed":false});
    let assessment = assemble(context.clone(), imports.clone(), &raw, 100).unwrap();
    assert_eq!(assessment.editor.status, "unavailable");
    assert_eq!(assessment.qualification, "unqualified");
    configuration::record_assessment(&f.m, &next, &"ac".repeat(16), &assessment).unwrap();
    assert_eq!(configuration::view(&f.m, &next).unwrap()["assessment"]["editor"]["status"], "unavailable");
    assert!(configuration::view(&f.m, &base).unwrap()["assessment"].is_null());
    assert!(configuration::record_assessment(&f.m, &base, &"ad".repeat(16), &assessment).is_err());
    raw["graphics_configuration"]["dll_overrides"] = json!("unrelated=b");
    assert!(assemble(context, imports, &raw, 101).is_err());
    assert!(prep::observations(&f.m, &next).unwrap().is_empty(), "graphics probes cannot qualify audio/editor/state");
}
#[test]
fn graphics_assessment_refuses_wrong_identity_incomplete_and_false_claims() {
    let imports = pe::inspect(&mut Cursor::new(image("d3d11.dll", "opengl32.dll"))).unwrap();
    for index in 0..8 {
        let mut r = report();
        match index {
            0 => r["cleanup_confirmed"] = json!(false),
            1 => r["records"][0]["module_sha256"] = json!("00".repeat(32)),
            2 => r["records"][1]["class_id"] = json!("00".repeat(16)),
            3 => {
                r["records"].as_array_mut().unwrap().pop();
            }
            4 => {
                let row = r["records"][3].clone();
                r["records"].as_array_mut().unwrap().push(row);
            }
            5 => r["records"][3]["probes"][1]["rendering"] = json!("reported_hardware"),
            6 => r["records"][3]["probes"][0]["renderer"] = json!("/home/private/device"),
            _ => r["records"][3]["probes"][0]["status"] = json!("unavailable"),
        };
        assert!(
            assemble(context(), imports.clone(), &r, 100).is_err(),
            "case {index}"
        );
    }
    let mut r = report();
    r["records"][2]["editor"]["status"] = json!("unavailable");
    assert_eq!(
        assemble(context(), imports, &r, 100).unwrap().editor.status,
        "unavailable"
    );
}

#[test]
fn graphics_driver_observation_changes_without_rekeying_environment() {
    let imports = pe::inspect(&mut Cursor::new(image("d3d11.dll", "opengl32.dll"))).unwrap();
    let original = assemble(context(), imports.clone(), &report(), 100).unwrap();
    let mut changed = report();
    changed["records"][3]["probes"][0]["renderer"] = json!("llvmpipe (LLVM test)");
    changed["records"][3]["probes"][0]["rendering"] = json!("reported_hardware");
    changed["records"][3]["probes"][0]["driver_version"] = json!("2.0");
    let next = assemble(context(), imports, &changed, 101).unwrap();
    assert_eq!(next.context, original.context);
    assert_eq!(next.context_fingerprint, original.context_fingerprint);
    assert_ne!(
        next.observations_fingerprint,
        original.observations_fingerprint
    );
    assert_eq!(next.runtime_probes[0].rendering, "reported_software");
    assert!(next.editor_device.is_none());
}

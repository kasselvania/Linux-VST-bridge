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
            .sources,
        vec!["ordinary_import_hint", "loaded_before_editor"]
    );
    assert_eq!(
        a.findings.iter().find(|f| f.library == Library::WebView2).unwrap().sources,
        vec!["loaded_during_editor"]
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
fn graphics_recommendation_follows_editor_loads_and_this_launch_probes() {
    use super::assessment::{recommend, Editor, Probe, Requirement};
    use crate::operator_model::GraphicsBackend;
    let probe = |api: &str, status: &str| Probe { api: api.into(), status: status.into(), rendering: "unknown".into(),
        renderer: None, vendor_id: None, device_id: None, driver_version: None, feature_level: None, implementation: None };
    let probes = |failed: &[&str]| ["d3d11_default", "d3d11_warp", "opengl", "direct_composition"].iter()
        .map(|api| probe(api, if failed.contains(api) { "readback_failed" } else { "passed" })).collect::<Vec<_>>();
    let editor = |status: &str, before: &[Library], during: &[Library]| Editor {
        status: status.into(), before: before.to_vec(), during: during.to_vec(), closed: true };
    let imports = |ordinary: &[Library]| pe::Imports { machine: 0x8664, ordinary: ordinary.to_vec(), delayed: vec![] };
    let none = imports(&[]);
    // A plain editor on a healthy launch needs nothing.
    let r = recommend(&editor("opened", &[], &[]), &probes(&[]), &none, None);
    assert_eq!((r.requirement, r.editor_loaded.len()), (Requirement::None, 0));
    // No editor decides nothing, whatever the probes say.
    assert_eq!(recommend(&editor("unavailable", &[], &[]), &probes(&["d3d11_default"]), &none, None).requirement,
        Requirement::EditorAbsent);
    // An unassessed WebView2 dependency cannot hide a failed Direct3D probe.
    assert_eq!(recommend(&editor("opened", &[], &[Library::D3d11, Library::WebView2]), &probes(&["d3d11_default"]), &none, None)
        .requirement, Requirement::WineD3d11);
    // DirectComposition: the blank-editor class, decided by this launch's probe.
    assert_eq!(recommend(&editor("opened", &[], &[Library::DirectComposition, Library::D3d11]), &probes(&["direct_composition"]), &none, None)
        .requirement, Requirement::DirectCompositionRunner);
    assert_eq!(recommend(&editor("opened", &[], &[Library::DirectComposition, Library::D3d11]), &probes(&[]), &none, None)
        .requirement, Requirement::None);
    // Direct3D 11 failing on the default path asks for Wine's built-in path once.
    assert_eq!(recommend(&editor("opened", &[], &[Library::Dxgi]), &probes(&["d3d11_default"]), &none, None).requirement,
        Requirement::WineD3d11);
    assert_eq!(recommend(&editor("opened", &[], &[Library::Dxgi]), &probes(&["d3d11_default"]), &none, Some(GraphicsBackend::WineD3d11))
        .requirement, Requirement::Undetermined);
    // OpenGL failing has no further launch choice here.
    assert_eq!(recommend(&editor("opened", &[], &[Library::OpenGl]), &probes(&["opengl"]), &none, None).requirement,
        Requirement::Undetermined);
    // A library loaded with the module counts only when the module imports it;
    // an import that was never loaded counts for nothing.
    let r = recommend(&editor("opened", &[Library::D3d11], &[]), &probes(&["d3d11_default"]), &imports(&[Library::D3d11]), None);
    assert_eq!((r.requirement, r.editor_loaded.clone()), (Requirement::WineD3d11, vec![Library::D3d11]));
    assert_eq!(recommend(&editor("opened", &[Library::D3d11], &[]), &probes(&["d3d11_default"]), &none, None).requirement,
        Requirement::None);
    assert_eq!(recommend(&editor("opened", &[], &[]), &probes(&["d3d11_default"]), &imports(&[Library::D3d11]), None).requirement,
        Requirement::None);
    // `during` repeats what was loaded before the editor. A library the module
    // does not import, loaded by something else before the editor opened, is
    // not the editor's: a working Direct2D editor with DirectComposition in the
    // process is left alone, while a module that imports it is still decided.
    let process = [Library::D3d11, Library::Dxgi, Library::Direct2d, Library::DirectComposition, Library::OpenGl];
    let r = recommend(&editor("opened", &process, &process), &probes(&["direct_composition"]),
        &imports(&[Library::D3d11, Library::Direct2d]), None);
    assert_eq!((r.requirement, r.editor_loaded), (Requirement::None, vec![Library::D3d11, Library::Direct2d]));
    let r = recommend(&editor("opened", &process, &process), &probes(&["direct_composition"]),
        &imports(&[Library::D3d11, Library::Dxgi, Library::Direct2d, Library::DirectComposition]), None);
    assert_eq!(r.requirement, Requirement::DirectCompositionRunner);
    assert!(!r.editor_loaded.contains(&Library::OpenGl), "a dependency of the runner's Direct3D is not the editor's");
    assert!(r.reason.contains("cannot create a composition device"));
    // A default launch that is already Wine's built-in Direct3D 11 has no Wine
    // trial left to offer; another provider still gets the one trial.
    let with_provider = |provider: &str| { let mut all = probes(&["d3d11_default"]);
        all[0].implementation = Some(provider.into()); all };
    assert_eq!(recommend(&editor("opened", &[], &[Library::Dxgi]), &with_provider("wine_builtin"), &none, None).requirement,
        Requirement::Undetermined);
    assert_eq!(recommend(&editor("opened", &[], &[Library::Dxgi]), &with_provider("other"), &none, None).requirement,
        Requirement::WineD3d11);
    // The assembled assessment carries the recommendation and uses its action.
    let a = assemble(context(), pe::inspect(&mut Cursor::new(image("d3d11.dll", "opengl32.dll"))).unwrap(), &report(), 100).unwrap();
    assert_eq!(a.recommendation.requirement, Requirement::Undetermined);
    assert_eq!(a.next_action, a.recommendation.action);
    assert_eq!(serde_json::to_value(&a).unwrap()["recommendation"]["requirement"], json!("undetermined"));
}

#[test]
fn detected_webview2_stays_unassessed_and_does_not_mask_unavailable_capabilities() {
    let imports = pe::Imports { machine: 0x8664, ordinary: vec![], delayed: vec![Library::WebView2] };
    for (library, failed_api, expected) in [
        (Library::OpenGl, None, Requirement::Undetermined),
        (Library::DirectComposition, Some("direct_composition"), Requirement::DirectCompositionRunner),
        (Library::D3d11, Some("d3d11_default"), Requirement::WineD3d11),
        (Library::OpenGl, Some("opengl"), Requirement::Undetermined),
    ] {
        let mut raw = report();
        raw["records"][2]["editor"]["before"] = json!([]);
        raw["records"][2]["editor"]["during"] = json!([library, Library::WebView2]);
        if let Some(api) = failed_api {
            let probe = raw["records"][3]["probes"].as_array_mut().unwrap().iter_mut()
                .find(|probe| probe["api"] == api).unwrap();
            probe["status"] = json!("unavailable");
            probe["rendering"] = json!("unknown");
        }
        let assessment = assemble(context(), imports.clone(), &raw, 100).unwrap();
        assert_eq!(assessment.recommendation.requirement, expected);
        let webview = assessment.findings.iter().find(|finding| finding.library == Library::WebView2).unwrap();
        assert_eq!(webview.probe_status, "not_tested");
        assert!(webview.probe_api.is_none());
        assert!(assessment.recommendation.editor_loaded.contains(&Library::WebView2));
        assert!(!assessment.recommendation.reason.contains("will not display"));
        assert!(!assessment.recommendation.action.contains("still work"));
        if failed_api.is_none() {
            assert!(assessment.recommendation.reason.contains("does not establish whether"));
            assert!(assessment.recommendation.action.contains("Audio and state recall need their own tests"));
        } else {
            assert!(!assessment.recommendation.reason.contains("WebView2"), "a dependency hint cannot mask the actual failed probe");
        }
    }
    // An import hint that never loaded also cannot invent a runner requirement.
    let mut raw = report();
    raw["records"][2]["editor"]["during"] = json!(["d3d11"]);
    let assessment = assemble(context(), imports, &raw, 100).unwrap();
    assert_eq!(assessment.recommendation.requirement, Requirement::None);
    assert!(!assessment.recommendation.reason.contains("Every graphics library"));
    assert!(assessment.recommendation.reason.contains("does not establish that it rendered"));
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
fn graphics_launch_accepts_composed_builtins_and_refuses_conflicting_overrides() {
    let mut c = context();
    c.configuration_fingerprint = Some("77".repeat(32));
    c.requested_backend = Some(crate::operator_model::GraphicsBackend::WineD3d11);
    let imports = pe::Imports { machine: 0x8664, ordinary: vec![], delayed: vec![] };
    let mut raw = report();
    raw["graphics_configuration"] = json!({"requested_backend":"wine_d3d11",
        "scope":"host_process_and_children","renderer_observed":false});
    for overrides in ["d3d11,dxgi=b;lvb-direct-wait=b",
        "d2d1,d3d11,dxgi,dcomp=b;uiautomationcore=;lvb-direct-wait=b",
        "lvb-direct-wait=b; DXGI.dll = b ; D3D11 = b", "d3d11=b;dxgi=b;other=n,b"] {
        raw["graphics_configuration"]["dll_overrides"] = json!(overrides);
        assert!(assemble(c.clone(), imports.clone(), &raw, 100).is_ok(), "{overrides}");
    }
    for overrides in ["lvb-direct-wait=b", "d3d11=b", "dxgi=b", "d3d11,dxgi=n,b",
        "d3d11,dxgi=b;dxgi=n", "dxgi=n;d3d11,dxgi=b", "d3d11,dxgi=b;*=n",
        "d3d11,dxgi=b;*dxgi=n", "d3d11,dxgi=b;C:\\windows\\system32\\d3d11.dll=n",
        "d3d11,dxgi=b;d3d*=n", "d3d11,dxgi=b=invalid"] {
        raw["graphics_configuration"]["dll_overrides"] = json!(overrides);
        assert!(assemble(c.clone(), imports.clone(), &raw, 100).is_err(), "{overrides}");
    }
}

fn close_timeout_report() -> Value {
    let mut raw = report();
    raw["records"].as_array_mut().unwrap().truncate(4);
    raw["records"][2]["schema"] = json!(2);
    raw["records"][2]["editor"]["closed"] = json!(false);
    raw["records"].as_array_mut().unwrap().push(json!({"state":"ap8_call","operation":"closeGraphicsEditor"}));
    raw["error"] = json!("TimeoutError: Windows call deadline: closeGraphicsEditor");
    raw
}

#[test]
fn graphics_observations_survive_contained_close_failure_without_qualifying_editor() {
    let imports = pe::Imports { machine: 0x8664, ordinary: vec![], delayed: vec![] };
    let mut raw = close_timeout_report();
    let assessment = assemble(context(), imports.clone(), &raw, 100).unwrap();
    assert!(!assessment.editor.closed);
    assert_eq!(assessment.editor_retirement, EditorRetirement::TimedOut);
    assert_eq!(assessment.recommendation.requirement, Requirement::Undetermined);
    assert_eq!(assessment.qualification, "unqualified");
    assert!(assessment.editor_device.is_none());
    raw["records"].as_array_mut().unwrap().push(json!({"state":"graphics_editor_closed","schema":1,"closed":false}));
    raw["error"] = json!("Windows host exited without successful close");
    assert_eq!(assemble(context(), imports.clone(), &raw, 100).unwrap().editor_retirement, EditorRetirement::Failed);
    raw["records"].as_array_mut().unwrap().last_mut().unwrap()["closed"] = json!(true);
    assert!(assemble(context(), imports.clone(), &raw, 100).is_err(), "closed editor does not waive inspection failure");
    raw["error"] = Value::Null;
    raw["records"].as_array_mut().unwrap().extend([
        json!({"state":"ap8_inspection_closed","exit_code":0}),
        json!({"state":"scanner_completed","inspection_complete":true})]);
    let assessment = assemble(context(), imports, &raw, 100).unwrap();
    assert!(assessment.editor.closed);
    assert_eq!(assessment.editor_retirement, EditorRetirement::Closed);
}

#[test]
fn graphics_close_timeout_does_not_waive_binding_probes_or_cleanup() {
    let imports = pe::Imports { machine: 0x8664, ordinary: vec![], delayed: vec![] };
    for case in 0..10 {
        let mut raw = close_timeout_report();
        match case {
            0 => raw["cleanup_confirmed"] = json!(false),
            1 => raw["transport_retired"] = json!(false),
            2 => raw["gated"] = json!(false),
            3 => raw["records"][0]["scanner_sha256"] = json!("00".repeat(32)),
            4 => raw["records"][1]["class_id"] = json!("00".repeat(16)),
            5 => raw["records"][3]["probes"].as_array_mut().unwrap().clear(),
            6 => raw["error"] = json!("TimeoutError: Windows call deadline: assessGraphicsRuntime"),
            7 => raw["records"].as_array_mut().unwrap().last_mut().unwrap()["operation"] = json!("getComponentState"),
            8 => raw["records"].as_array_mut().unwrap().push(json!({"state":"scanner_completed","inspection_complete":true})),
            _ => raw["records"][2]["editor"]["closed"] = json!(true),
        }
        assert!(assemble(context(), imports.clone(), &raw, 100).is_err(), "case {case}");
    }
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

fn provider_report(default: Value, warp: Value) -> Value {
    let mut raw = report();
    raw["records"][3]["schema"] = json!(2);
    raw["records"][3]["probes"][0]["implementation"] = default;
    raw["records"][3]["probes"][1]["implementation"] = warp;
    raw
}

#[test]
fn graphics_probes_report_the_observed_direct3d_provider_and_never_an_unfounded_device() {
    let imports = pe::Imports { machine: 0x8664, ordinary: vec![], delayed: vec![] };
    let check = |raw: &Value| assemble(context(), imports.clone(), raw, 100);
    // Wine's built-in Direct3D names a stand-in adapter and ignores the WARP
    // request: the name is kept as reported, the device claim is not.
    let mut raw = provider_report(json!("wine_builtin"), json!("wine_builtin"));
    for index in [0, 1] {
        raw["records"][3]["probes"][index]["renderer"] = json!("ATI Radeon HD 5600 Series");
        raw["records"][3]["probes"][index]["rendering"] = json!("unknown");
    }
    raw["records"][3]["probes"][2]["renderer"] = json!("AMD Custom GPU 0405 (radeonsi, vangogh, LLVM 20.1.8, DRM 3.64, 6.16.12-valve24.5-1-neptune-616)");
    raw["records"][3]["probes"][2]["rendering"] = json!("reported_hardware");
    let a = check(&raw).unwrap();
    assert_eq!(a.runtime_probes[0].implementation.as_deref(), Some("wine_builtin"));
    assert_eq!(a.runtime_probes[0].rendering, "unknown");
    assert!(a.runtime_probes[2].renderer.as_deref().unwrap().starts_with("AMD Custom GPU"));
    assert_eq!(a.runtime_probes[2].rendering, "reported_hardware");
    assert_eq!(serde_json::to_value(&a).unwrap()["runtime_probes"][0]["implementation"], json!("wine_builtin"));
    assert!(serde_json::to_value(&a).unwrap()["runtime_probes"][2].get("implementation").is_none());
    for index in [0, 1] {
        let mut claimed = raw.clone();
        claimed["records"][3]["probes"][index]["rendering"] = json!("reported_hardware");
        assert!(check(&claimed).is_err(), "probe {index}");
    }
    // A software Mesa driver keeps its name and is software whatever the host said.
    raw["records"][3]["probes"][2]["renderer"] = json!("llvmpipe (LLVM 20.1.8, 256 bits)");
    assert_eq!(check(&raw).unwrap().runtime_probes[2].rendering, "reported_software");
    // The provider is required from a host that reports it, bounded to the two
    // known values, and belongs only to the Direct3D probes.
    assert!(check(&provider_report(json!("other"), json!("other"))).is_ok());
    assert!(check(&provider_report(Value::Null, json!("other"))).is_err());
    assert!(check(&provider_report(json!("dxvk"), json!("other"))).is_err());
    let mut misplaced = provider_report(json!("other"), json!("other"));
    misplaced["records"][3]["probes"][2]["implementation"] = json!("other");
    assert!(check(&misplaced).is_err());
    let mut unloaded = provider_report(Value::Null, json!("other"));
    unloaded["records"][3]["probes"][0]["status"] = json!("unavailable");
    unloaded["records"][3]["probes"][0]["rendering"] = json!("unknown");
    assert!(check(&unloaded).is_ok(), "a Direct3D library that did not load has no provider");
    // An earlier host cannot name a provider, and its hardware claim for an
    // OpenGL renderer it could not retain is not kept.
    let mut earlier = report();
    earlier["records"][3]["probes"][0]["implementation"] = json!("other");
    assert!(check(&earlier).is_err());
    let mut earlier = report();
    earlier["records"][3]["probes"][2]["rendering"] = json!("reported_hardware");
    assert_eq!(check(&earlier).unwrap().runtime_probes[2].rendering, "unknown");
}

//! Behaviour oracle through the real manager service.
//!
//! Each test builds a fixture state root from the shared manager fixtures,
//! starts the real `linux-vst-bridge serve` binary on it as a child process,
//! and observes it only through the entry points a real client uses: the
//! `operator` command line the window runs, and the service socket the
//! native proxy and status readers use. These behaviours must stay identical
//! while the manager is restructured around one long-running service (#489).
//!
//! How the service runs without systemd is documented in `service.rs`.
#![cfg(target_os = "linux")]

use linux_vst_bridge::*;
// The shared fixture resolves these through `crate::*`, as in the manager binary.
#[allow(unused_imports)]
use std::{
    fs,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[allow(dead_code, unused_imports)]
#[path = "../../src/test_fixture.rs"]
mod test_fixture;

mod service;

use serde_json::{json, Value};
use service::{Admitted, RealService};

/// Oracle 1. What the window reads from the running service: the wire schema,
/// the published plug-in in the overview, the service's capacity ceiling, and
/// the actions the plug-in page offers.
#[test]
fn overview_reports_schema_products_ceiling_and_offers() {
    let service = RealService::start(service::published());
    let p = service.plugin();
    let overview = service.operator(&["overview"], None);
    assert_eq!(overview["schema"], 1, "{overview:#}");
    assert_eq!(overview["operator_schema"], operator_model::OPERATOR_SCHEMA);
    assert_eq!(operator_model::OPERATOR_SCHEMA, 29, "a wire change must be deliberate");
    assert_eq!(overview["scope"], "current_only");
    let current = &overview["current"];
    assert_eq!(current["schema"], 29);
    let system = &current["system"];
    assert_eq!(system["ceiling"], 8, "{system:#}");
    assert_eq!((system["dsp"].as_u64(), system["maintenance"].as_u64()), (Some(0), Some(0)));
    assert_eq!(system["cleanup_unconfirmed"], false);
    assert_eq!(system["pending_transactions"], 0);

    let row = service.product(current);
    for (field, value) in [("name", "Actual class"), ("vendor", "Vendor"), ("role", "instrument"),
        ("version", "1.0"), ("disposition", "ready"), ("runner", "exact-runner-1")] {
        assert_eq!(row[field], value, "{field}: {row:#}");
    }
    assert_eq!(row["environment"], p.environment.as_str());
    assert_eq!(row["module_sha256"], p.module_sha256.as_str());
    assert_eq!(row["active_revision"], 1);
    assert_eq!(row["details"]["publication_valid"], true);
    assert_eq!(row["details"]["publication_selected"], true);

    // The plug-in page reads the product's own offers from the same state.
    let detail = service.product_detail();
    assert_eq!(detail["operator_schema"], 29, "{detail:#}");
    assert_eq!(detail["state_token"], current["state_token"]);
    assert_eq!(detail["system"]["ceiling"], 8);
    let offered = service::offers(&detail["product"]);
    let class = p.class_id.as_str();
    let state = |kind: &str, fields: Value| service::offer_state(&offered, kind, class, &fields);
    assert_eq!(state("buffering_set", json!({"added_frames":256})), Some(None), "{offered:#?}");
    assert_eq!(state("buffering_set", json!({"added_frames":1024})), Some(None));
    assert_eq!(state("buffering_set", json!({"added_frames":512})), None, "the current setting");
    assert_eq!(state("delivery_set", json!({"mode":"same_callback"})), Some(None));
    assert_eq!(state("plugin_remove_from_daw", json!({})), Some(None));
    assert!(matches!(state("capture_arm", json!({})), Some(Some(_))),
        "crash capture needs an ordinary publication");
    service.stop();
}

/// Oracle 2. One offered, harmless action round-trips: the request is
/// accepted against the page's state token, its worker runs (reading the
/// running service's capacity before it may write), the result reaches the
/// window's activity read, the offer moves, and the next native activation
/// uses the new setting.
#[test]
fn offered_buffering_change_round_trips_through_request_worker_and_result() {
    let service = RealService::start(service::published());
    let class = service.plugin().class_id.clone();
    let detail = service.product_detail();
    let action = json!({"kind":"buffering_set","class_id":class,"added_frames":256});
    let request = json!({"schema":29,"state_token":detail["state_token"],"action":action});
    let receipt = service.operator(&["request"], Some(&request));
    assert_eq!(receipt["schema"], 29, "{receipt:#}");
    assert_eq!(receipt["accepted"], true, "{receipt:#}");
    assert!(receipt["refusal"].is_null());
    let operation = receipt["operation"].as_str().unwrap().to_owned();
    let result = service.finished_operation(&operation);
    assert_eq!(result["state"], "completed", "{result:#}");
    assert_eq!(result["result"], json!({"remembered_buffered_frames":256,"added_bridge_frames":256,
        "applies_to":"next_activation","compatibility_qualified":false}));

    // The offer has moved: replaying it is refused, and 512 is offered back.
    let replay = service.operator(&["request"], Some(&request));
    assert_eq!(replay["accepted"], false, "{replay:#}");
    assert_eq!(replay["refusal"], "operator_action_not_available");
    let after = service.product_detail();
    let offered = service::offers(&after["product"]);
    let state = |frames: u32| service::offer_state(&offered, "buffering_set", &class,
        &json!({"added_frames":frames}));
    assert_eq!((state(256), state(512), state(1024)), (None, Some(None), Some(None)));

    // The next activation uses it: a native admission is bound at 256 frames.
    let instance = service.admit(service.plugin());
    let Admitted::Accepted { added_frames, .. } = instance.admitted() else {
        panic!("admission refused: {:?}", instance.admitted())
    };
    assert_eq!(*added_frames, 256);
    // While it plays, the page reads the service's census and withholds
    // configuration changes for that plug-in.
    let playing = service.product_detail();
    assert_eq!(playing["system"]["dsp"], 1, "{:#}", playing["system"]);
    let offered = service::offers(&playing["product"]);
    assert_eq!(service::offer_state(&offered, "buffering_set", &class, &json!({"added_frames":512})),
        Some(Some("Close instances of this plug-in before changing its configuration".into())));
    instance.release();
    service.capacity_when(|capacity| capacity["dsp"] == 0);
    service.stop();
}

/// Oracle 3. Native-proxy admission over the service socket, in the proxy's
/// own wire format, for two published plug-ins of one setup: every instance
/// is admitted with its own session and counted; a plug-in is refused at its
/// four native slots and the service at its ceiling of eight, neither refusal
/// taking capacity; a removed instance is retired from the census and its slot
/// admits again. A restarted service finds no owners left behind.
#[test]
fn native_admission_counts_releases_and_refuses_at_the_limit() {
    let mut service = RealService::start(service::published_classes(2));
    let [first, second] = &service.published.plugins[..] else { unreachable!() };
    let idle = service.capacity();
    assert_eq!(idle["limits"]["global_dsp"], 8, "{idle:#}");
    assert_eq!(idle["limits"]["native_image_hard"], 4);
    assert_eq!((idle["dsp"].as_u64(), idle["available_dsp"].as_u64()), (Some(0), Some(8)));
    for plugin in [first, second] {
        assert!(idle["class_ceilings"].as_array().unwrap()
            .contains(&json!({"class_id":plugin.class_id,"dsp":4})), "{idle:#}");
    }

    let mut live = Vec::new();
    let admit = |plugin: &service::Plugin, live: &mut Vec<service::Instance>| {
        let instance = service.admit(plugin);
        let Admitted::Accepted { added_frames, delivery_mode, .. } = instance.admitted() else {
            panic!("instance {} refused: {:?}", live.len() + 1, instance.admitted())
        };
        assert_eq!((*added_frames, *delivery_mode), (512, 0), "installed defaults");
        live.push(instance);
    };
    for admitted in 1..=4 {
        admit(first, &mut live);
        let capacity = service.capacity();
        assert_eq!(capacity["dsp"], admitted, "{capacity:#}");
        assert_eq!(capacity["per_class"][&first.class_id], admitted);
        assert_eq!(capacity["available_dsp"], 8 - admitted);
        assert_eq!(capacity["keepers"], 1, "one keeper for the setup");
    }
    let refused = service.admit(first);
    assert_eq!(refused.admitted(), &Admitted::Refused("admission_native_image_capacity".into()));
    assert_eq!(service.capacity()["dsp"], 4, "a refusal takes no capacity");

    for admitted in 5..=8 {
        admit(second, &mut live);
        let capacity = service.capacity();
        assert_eq!(capacity["dsp"], admitted, "{capacity:#}");
        assert_eq!(capacity["per_class"][&second.class_id], admitted - 4);
        assert_eq!(capacity["keepers"], 1, "both plug-ins share the setup's keeper");
    }
    assert_eq!(service.capacity()["available_dsp"], 0);
    let sessions: std::collections::BTreeSet<_> = live.iter().map(|i| i.session().to_owned()).collect();
    assert_eq!(sessions.len(), 8, "each instance has its own session");
    for plugin in [first, second] {
        let refused = service.admit(plugin);
        assert_eq!(refused.admitted(), &Admitted::Refused("admission_global_capacity".into()));
    }
    assert_eq!(service.capacity()["dsp"], 8, "a refusal takes no capacity");

    // The DAW's removal is acknowledged first; the service then retires the
    // owner from its census within a bound, and its slot admits again.
    live.remove(0).release();
    let capacity = service.capacity_when(|capacity| capacity["dsp"] == 7);
    assert_eq!(capacity["per_class"][&first.class_id], 3);
    assert_eq!(capacity["cleanup_unconfirmed"], false);
    admit(first, &mut live);
    assert_eq!(service.capacity()["dsp"], 8);

    for instance in live.drain(..) {
        instance.release();
    }
    let released = service.capacity_when(|capacity| capacity["dsp"] == 0);
    assert_eq!(released["cleanup_unconfirmed"], false);

    service.restart();
    let restarted = service.capacity();
    assert_eq!((restarted["dsp"].as_u64(), restarted["keepers"].as_u64()), (Some(0), Some(0)), "{restarted:#}");
    assert_eq!(restarted["cleanup_unconfirmed"], false);
    assert_eq!(service.pulse(), 0, "nothing blocks new admissions");
    service.stop();
}

/// Not an oracle: the harness re-executes this test binary as the native
/// proxy's admission client, so that the service sees a separate DAW process
/// with its own environment and mapped engine. Ordinary runs ignore it.
#[test]
#[ignore = "re-executed by the real-service harness as the native admission client"]
fn native_admission_client() {
    service::admission_client();
}

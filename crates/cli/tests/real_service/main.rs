//! Behaviour oracle through the real manager service.
//!
//! Each test builds a fixture state root from the shared manager fixtures,
//! starts the real `linux-vst-bridge serve` binary on it as a child process,
//! and observes it only through the entry points a real client uses: the
//! window's exchanges through the client contract (`service::contract`) —
//! status reads from the service socket, actions through the `operator`
//! command — and the socket the native proxy and status readers use. These behaviours must stay identical
//! while the manager is restructured around one long-running service (#489).
//!
//! How the service runs without systemd is documented in `service.rs`.
#![cfg(target_os = "linux")]

use linux_vst_bridge::{service::contract, *};
// The shared fixture resolves these through `crate::*`, as in the manager binary.
#[allow(unused_imports)]
use std::{
    fs,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

#[allow(dead_code, unused_imports)]
#[path = "../../../service/src/test_fixture.rs"]
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
    let overview = service.read(contract::Read::Overview);
    assert_eq!(overview["schema"], 1, "{overview:#}");
    assert_eq!(overview["operator_schema"], operator_model::OPERATOR_SCHEMA);
    assert_eq!(operator_model::OPERATOR_SCHEMA, 30, "a wire change must be deliberate");
    assert_eq!(overview["scope"], "current_only");
    let current = &overview["current"];
    assert_eq!(current["schema"], 30);
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
    assert_eq!(detail["operator_schema"], 30, "{detail:#}");
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
    let request = json!({"schema":30,"state_token":detail["state_token"],"action":action});
    let receipt = service.request(&request);
    assert_eq!(receipt["schema"], 30, "{receipt:#}");
    assert_eq!(receipt["accepted"], true, "{receipt:#}");
    assert!(receipt["refusal"].is_null());
    let operation = receipt["operation"].as_str().unwrap().to_owned();
    let result = service.finished_operation(&operation);
    assert_eq!(result["state"], "completed", "{result:#}");
    assert_eq!(result["result"], json!({"remembered_buffered_frames":256,"added_bridge_frames":256,
        "applies_to":"next_activation","compatibility_qualified":false}));

    // The offer has moved: replaying it is refused, and 512 is offered back.
    let replay = service.request(&request);
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
    // configuration changes for that plug-in. The admission's own records
    // settle first: the page is read once the census shows the instance.
    service.capacity_when(|capacity| capacity["dsp"] == 1);
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

/// Oracle 4. The running service answers the window's status reads from
/// views it holds, and what it answers is exactly what the command line
/// composes afresh, after each kind of writer: an independent process
/// changing a record directly, a runtime file changing outside every watched
/// tree, an operation's worker, and the service's own admission and
/// retirement of an instance.
#[test]
fn service_reads_follow_every_writer() {
    use contract::{Read, Request, Transport};
    let published = service::published();
    let retained_proton = service::retained_runtime(&published.fixture.m);
    let service = RealService::start(published);
    let p = service.plugin();
    let class = p.class_id.clone();
    let both = |read: Read| -> Value {
        let request = Request::Read(read);
        let held = service.service().exchange(&request, None)
            .unwrap_or_else(|error| panic!("service {request:?}: {error:?}"));
        let composed = service.manager_command(None).exchange(&request, None).unwrap();
        assert_eq!(String::from_utf8_lossy(&held), String::from_utf8_lossy(&composed), "{request:?}");
        serde_json::from_slice(&held).unwrap()
    };
    let current_frames = || {
        let detail = both(Read::Product(p.key()));
        let offered = service::offers(&detail["product"]);
        [256, 512, 1024].into_iter().filter(|frames| service::offer_state(&offered, "buffering_set", &class,
            &json!({"added_frames":frames})).is_none()).collect::<Vec<_>>()
    };
    for read in [Read::CurrentOverview, Read::Product(p.key()), Read::Snapshot, Read::Activity, Read::Pulse] {
        both(read.clone());
        both(read);
    }
    assert_eq!(current_frames(), [512], "the installed default");
    // The held pages were answered again, not composed again.
    let views = service.views();
    assert_eq!(views["covering"], true, "{views:#}");
    assert!(views["watched_directories"].as_u64().unwrap() > 0, "{views:#}");
    let again = || service.views()["answered_again"].as_u64().unwrap();
    let before = again();
    both(Read::CurrentOverview);
    both(Read::Product(p.key()));
    assert_eq!(again(), before + 4, "both transports read each page from the service-held views");

    // A retained runtime's pinned file goes away. No tree and no stamp covers
    // its directory, only the files the held page opened while it was
    // composed; the next read must be composed again and say so.
    let runtime_failure = || both(Read::CurrentOverview)["current"]["managed_runtime_records"][0]["failure"].clone();
    assert_eq!(runtime_failure(), Value::Null);
    let pinned = fs::read(&retained_proton).unwrap();
    fs::remove_file(&retained_proton).unwrap();
    assert!(runtime_failure().is_string(), "the held overview is not answered after its file went away");
    fs::write(&retained_proton, &pinned).unwrap();
    fs::set_permissions(&retained_proton, fs::Permissions::from_mode(0o500)).unwrap();
    assert_eq!(runtime_failure(), Value::Null);

    // A runtime file the snapshot verifies by its bytes changes. Runtime
    // trees are not watched; the snapshot is composed for every read.
    let proton = service.published.fixture.r.environment.runner.proton.clone();
    let original = fs::read(&proton).unwrap();
    let runner_valid = || service.product(&both(Read::Snapshot))["details"]["runner_valid"].clone();
    assert_eq!(runner_valid(), true);
    fs::write(&proton, b"changed runtime bytes").unwrap();
    assert_eq!(runner_valid(), false);
    both(Read::CurrentOverview);
    // Readable retained identity never authorizes executing damaged runtime
    // bytes. An offered inspection reaches the owner and refuses before it
    // creates a scan or changes inspection/publication records.
    let manager = &service.published.fixture.m;
    let candidates = preparation::retained_candidates(manager).unwrap();
    let selection = &candidates[0].selection;
    let inspections = preparation::inspections(manager, selection).unwrap();
    let registry = fs::read(manager.root.join("registry.json")).unwrap();
    let reports = || fs::read_dir(manager.root.join("runtime/results")).unwrap()
        .map(|entry| entry.unwrap().path()).collect::<std::collections::BTreeSet<_>>();
    let prior_reports = reports();
    let detail = both(Read::Product(p.key()));
    let action = offered_action(&detail, "plugin_reinspect");
    let receipt = service.request(&json!({"schema":30,"state_token":detail["state_token"],"action":action}));
    assert_eq!(receipt["accepted"], true, "{receipt:#}");
    let refused = service.finished_operation(receipt["operation"].as_str().unwrap());
    assert_eq!(refused["state"], "refused", "{refused:#}");
    assert_eq!(refused["reason"], "Operator action: artifact missing or changed", "{refused:#}");
    assert_eq!(reports(), prior_reports, "no inspection supervisor was started");
    assert_eq!(preparation::inspections(manager, selection).unwrap(), inspections);
    assert_eq!(preparation::retained_candidates(manager).unwrap(), candidates);
    assert_eq!(fs::read(manager.root.join("registry.json")).unwrap(), registry);
    fs::write(&proton, &original).unwrap();
    assert_eq!(runner_valid(), true);

    // An independent writer, as the removed set-delay command was.
    service.published.fixture.m.select_delay(&class, 256).unwrap();
    assert_eq!(current_frames(), [256]);
    both(Read::CurrentOverview);

    // An operation's executor, owned by this same service process. The
    // page's state token is current, so the request is accepted.
    let detail = both(Read::Product(p.key()));
    let request = json!({"schema":30,"state_token":detail["state_token"],
        "action":{"kind":"buffering_set","class_id":class,"added_frames":512}});
    let receipt = service.request(&request);
    assert_eq!(receipt["accepted"], true, "{receipt:#}");
    let result = service.finished_operation(receipt["operation"].as_str().unwrap());
    assert_eq!(result["state"], "completed", "{result:#}");
    assert_eq!(current_frames(), [512]);
    both(Read::CurrentOverview);
    both(Read::Activity);

    // The service's own admission and retirement.
    let instance = service.admit(p);
    service.capacity_when(|capacity| capacity["dsp"] == 1);
    assert_eq!(both(Read::Product(p.key()))["system"]["dsp"], 1);
    both(Read::CurrentOverview);
    instance.release();
    service.capacity_when(|capacity| capacity["dsp"] == 0);
    assert_eq!(both(Read::Product(p.key()))["system"]["dsp"], 0);
    service.stop();
}

/// An offer the plug-in page shows, exactly as offered.
fn offered_action(detail: &Value, kind: &str) -> Value {
    service::offers(&detail["product"]).into_iter().find(|offer| offer["action"]["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} offer: {detail:#}"))["action"].clone()
}

/// Oracle 5 (#382). An offered action is validated once, when it is
/// accepted. Records unrelated to it that change before its worker runs — a
/// new installer arriving through the window's import, another plug-in of the
/// setup playing and stopping — leave it to complete, although the page it
/// was requested from is no longer current.
#[test]
#[cfg(feature = "service-executor-fixture")]
fn an_accepted_action_completes_after_unrelated_changes_before_its_worker_runs() {
    let service = RealService::start(service::published_classes(2));
    let [first, second] = &service.published.plugins[..] else { unreachable!() };
    let detail = service.read(contract::Read::Product(first.key()));
    let request = json!({"schema":30,"state_token":detail["state_token"],
        "action":{"kind":"buffering_set","class_id":first.class_id,"added_frames":256}});
    service.hold_workers();
    let receipt = service.request(&request);
    assert_eq!(receipt["accepted"], true, "{receipt:#}");
    let operation = receipt["operation"].as_str().unwrap().to_owned();
    service.worker_held(&operation);

    let imported = service.import_installer("Another Setup.exe", &service::windows_executable(1));
    assert_eq!(imported["format"], "pe_executable", "{imported:#}");
    let instance = service.admit(second);
    service.capacity_when(|capacity| capacity["dsp"] == 1);
    instance.release();
    service.capacity_when(|capacity| capacity["dsp"] == 0);
    let now = service.read(contract::Read::Product(first.key()));
    assert_ne!(now["state_token"], detail["state_token"], "the requesting page is out of date");

    service.release_workers();
    let result = service.finished_operation(&operation);
    assert_eq!(result["state"], "completed", "{result:#}");
    assert_eq!(result["result"]["remembered_buffered_frames"], 256, "{result:#}");
    service.stop();
}

/// Oracle 6 (#382). When what an accepted action acts on changes before its
/// worker runs, the worker refuses and names what changed. Two offers of one
/// plug-in's page are accepted together; the first to run removes the
/// plug-in from the DAW, so the other, a buffer change, no longer applies to
/// the plug-in it was chosen for.
#[test]
#[cfg(feature = "service-executor-fixture")]
fn an_accepted_action_is_refused_by_name_when_its_plugin_changed_before_its_worker_runs() {
    let service = RealService::start(service::published());
    let class = service.plugin().class_id.clone();
    let detail = service.product_detail();
    let request = |action: Value| json!({"schema":30,"state_token":detail["state_token"],"action":action});
    service.hold_workers();
    let buffering = service.request(&request(json!({"kind":"buffering_set","class_id":class,"added_frames":256})));
    assert_eq!(buffering["accepted"], true, "{buffering:#}");
    let buffering = buffering["operation"].as_str().unwrap().to_owned();
    service.worker_held(&buffering);
    let removal = service.request(&request(offered_action(&detail, "plugin_remove_from_daw")));
    assert_eq!(removal["accepted"], true, "{removal:#}");
    let removal = removal["operation"].as_str().unwrap().to_owned();
    service.worker_held(&removal);

    service.release_worker(&removal);
    let removed = service.operation_result(&removal);
    assert_eq!(removed["state"], "completed", "{removed:#}");
    service.release_worker(&buffering);
    let refused = service.operation_result(&buffering);
    assert_eq!(refused["state"], "refused", "{refused:#}");
    assert_eq!(refused["reason"], "This plug-in changed after you chose this. Look at it again, then choose again.",
        "{refused:#}");
    assert_eq!(refused["precondition"], "plugin", "{refused:#}");
    service.release_workers();
    service.stop();
}

/// Not an oracle: the latency of each window status read on this fixture,
/// through the manager command (one process per read, as the window read
/// before #489 step 3) and from the running service.
#[test]
#[ignore = "measurement; run with --ignored --nocapture"]
fn read_latency() {
    use contract::{Read, Request, Transport};
    let service = RealService::start(service::published());
    let p = service.plugin();
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let service_cpu = || {
        let stat = fs::read_to_string(format!("/proc/{}/stat", service.pid())).unwrap();
        let fields: Vec<&str> = stat.rsplit_once(") ").unwrap().1.split_whitespace().collect();
        (fields[11].parse::<f64>().unwrap() + fields[12].parse::<f64>().unwrap()) * 1000.0 / ticks
    };
    let children_cpu = || {
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        unsafe { libc::getrusage(libc::RUSAGE_CHILDREN, &mut usage) };
        let ms = |t: libc::timeval| t.tv_sec as f64 * 1000.0 + t.tv_usec as f64 / 1000.0;
        ms(usage.ru_utime) + ms(usage.ru_stime)
    };
    for read in [Read::CurrentOverview, Read::Product(p.key()), Read::Snapshot, Read::Activity, Read::Pulse] {
        let request = Request::Read(read.clone());
        let measure = |transport: &dyn Transport, rounds: u32| {
            transport.exchange(&request, None).unwrap();
            let (children, held, started) = (children_cpu(), service_cpu(), std::time::Instant::now());
            for _ in 0..rounds { transport.exchange(&request, None).unwrap(); }
            let per = |total: f64| total / f64::from(rounds);
            (per(started.elapsed().as_secs_f64() * 1000.0), per(children_cpu() - children), per(service_cpu() - held))
        };
        let slow = matches!(read, Read::Snapshot);
        let (wall, process, during) = measure(&service.manager_command(None), if slow { 3 } else { 20 });
        let (held_wall, _, held_cpu) = measure(&service.service(), if slow { 3 } else { 200 });
        eprintln!("LATENCY {read:?}: command {wall:.1} ms, CPU {process:.1} ms + service {during:.1} ms; \
            service {held_wall:.2} ms, CPU {held_cpu:.2} ms");
    }
    service.stop();
}

/// The exchanges themselves are unchanged. With `LVB_REFERENCE_MANAGER`
/// naming a manager built from an earlier revision, each window read, a
/// refused request and a refused browser return give the same bytes from
/// both managers on one state: the published plug-in alone, and beside a
/// long-used installation's history (#504).
#[test]
#[ignore = "compares with the manager named by LVB_REFERENCE_MANAGER"]
fn exchanges_match_the_reference_manager() {
    let reference = PathBuf::from(std::env::var_os("LVB_REFERENCE_MANAGER").expect("LVB_REFERENCE_MANAGER"));
    exchanges_match(&reference, service::published());
    let published = service::published();
    test_fixture::retained_history(&published.fixture, test_fixture::Changes::Runtimes, 15, 8, 380);
    exchanges_match(&reference, published);
}
fn exchanges_match(reference: &Path, published: service::Published) {
    use contract::Transport;
    let service = RealService::start(published);
    let p = service.plugin();
    let stale = contract::Request::Action(operator_model::Request { schema: operator_model::OPERATOR_SCHEMA,
        state_token: "stale".into(),
        action: operator_model::Action::BufferingSet { class_id: p.class_id.clone(), added_frames: 256 } });
    let requests = [
        contract::Request::Read(contract::Read::CurrentOverview),
        contract::Request::Read(contract::Read::Overview),
        contract::Request::Read(contract::Read::Product(p.key())),
        contract::Request::Read(contract::Read::Snapshot),
        contract::Request::Read(contract::Read::Activity),
        contract::Request::Read(contract::Read::Pulse),
        stale,
        contract::Request::BrowserReturn("native-access://auth?code=fixture".into()),
    ];
    for request in &requests {
        let exchange = |program: Option<&Path>| normalized(request,
            service.manager_command(program).exchange(request, None));
        // A first read may settle retained records; compare the reads after it.
        let _ = exchange(None);
        let mut pair = (exchange(Some(reference)), exchange(None));
        // The interactive overview's platform probe takes registry.lock
        // without waiting, so when the lock is held it names its DAW consumers
        // unverified: a pre-existing race, tracked separately. Only that read
        // is read again.
        for _ in 0..2 {
            if pair.0 == pair.1 || !matches!(request, contract::Request::Read(contract::Read::Overview)) { break; }
            pair = (exchange(Some(reference)), exchange(None));
        }
        assert_eq!(pair.0, pair.1, "{request:?}");
        assert!(!matches!(request, contract::Request::Read(_)) || pair.1.is_ok(), "{request:?}: {:?}", pair.1);
    }
    service.stop();
}
/// The reply as text. An overview's platform assessment records when it was
/// observed, and every request receipt names its own new operation; both
/// differ between any two exchanges.
fn normalized(request: &contract::Request, reply: std::result::Result<Vec<u8>, contract::Failure>)
    -> std::result::Result<String, contract::Failure> {
    fn without_times(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                fields.remove("observed_at");
                fields.values_mut().for_each(without_times);
            }
            Value::Array(items) => items.iter_mut().for_each(without_times),
            _ => {}
        }
    }
    let bytes = reply?;
    let mut value: Value = match request {
        contract::Request::Read(contract::Read::Overview) | contract::Request::Action(_) =>
            serde_json::from_slice(&bytes).unwrap(),
        _ => return Ok(String::from_utf8(bytes).unwrap()),
    };
    without_times(&mut value);
    if value["operation"].is_string() {
        value["operation"] = "operation".into();
    }
    Ok(value.to_string())
}

/// Not an oracle: the harness re-executes this test binary as the native
/// proxy's admission client, so that the service sees a separate DAW process
/// with its own environment and mapped engine. Ordinary runs ignore it.
#[test]
#[ignore = "re-executed by the real-service harness as the native admission client"]
fn native_admission_client() {
    service::admission_client();
}

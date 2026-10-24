use super::*;
use crate::test_fixture::Fixture;
use std::time::Duration;

const SESSION: &str = "01010101010101010101010101010101";

fn progress(generation: u64, epoch: u64, end: u64, phase: u64) -> Progress {
    let mut words = [0; 24];
    words[0] = 1;
    words[1] = u64::from_le_bytes([1; 8]);
    words[2] = words[1];
    words[3] = generation;
    words[4] = epoch;
    words[5] = 2;
    words[6] = end;
    words[16] = end;
    words[17] = phase;
    Progress { words }
}

fn key(generation: u64, epoch: u64) -> CounterKey {
    CounterKey { device:1, inode:2, mapping_device:1, mapping_inode:3, generation, epoch }
}

#[test]
fn progress_requires_an_increase_in_the_same_exact_transport_and_epoch() {
    let now = Instant::now();
    let mut observer = Observer::default();
    let first = observer.processing(SESSION, key(1, 1), progress(1, 1, 64, 11), now);
    assert!(matches!(first, ProcessingObservation::NotObservedAdvancing { .. }));
    assert!(matches!(observer.processing(SESSION, key(1, 1), progress(1, 1, 128, 11), now),
        ProcessingObservation::Advancing { .. }));
    assert!(matches!(observer.processing(SESSION, key(1, 1), progress(1, 1, 128, 11), now + Duration::from_millis(250)),
        ProcessingObservation::NotObservedAdvancing { observed_age_ms:250, .. }));
    assert!(matches!(observer.processing(SESSION, key(2, 1), progress(2, 1, 192, 11), now),
        ProcessingObservation::NotObservedAdvancing { .. }));
    assert!(matches!(observer.processing(SESSION, key(2, 2), progress(2, 2, 256, 11), now),
        ProcessingObservation::NotObservedAdvancing { .. }));
    let mut replacement = key(2, 2);
    replacement.mapping_inode += 1;
    assert!(matches!(observer.processing(SESSION, replacement, progress(2, 2, 320, 11), now),
        ProcessingObservation::NotObservedAdvancing { .. }));
    assert!(matches!(observer.processing("02020202020202020202020202020202", key(2, 2), progress(2, 2, 384, 11), now),
        ProcessingObservation::NotObservedAdvancing { .. }));
}

#[test]
fn stopped_is_only_reported_from_lifecycle_and_incomplete_progress_breaks_comparison() {
    let mut observer = Observer::default();
    let now = Instant::now();
    observer.processing(SESSION, key(1, 1), progress(1, 1, 64, 11), now);
    let mut torn = progress(1, 1, 64, 11);
    torn.words[5] = u64::MAX;
    assert!(matches!(observer.processing(SESSION, key(1, 1), torn, now), ProcessingObservation::Unavailable { .. }));
    assert!(matches!(observer.processing(SESSION, key(1, 1), progress(1, 1, 128, 11), now),
        ProcessingObservation::NotObservedAdvancing { .. }));
    assert!(matches!(observer.processing(SESSION, key(1, 1), progress(1, 1, 64, 11), now),
        ProcessingObservation::Unavailable { .. }));
    assert!(matches!(observer.processing(SESSION, key(1, 1), progress(1, 1, 128, 13), now),
        ProcessingObservation::LastReportedStopped { .. }));
    assert!(matches!(observer.processing(SESSION, key(1, 1), progress(1, 1, 128, 15), now),
        ProcessingObservation::LastReportedStopped { .. }));
    assert!(matches!(lifecycle(999), Observation::Unavailable(_)));
}

fn mapping(f: &Fixture) -> (PathBuf, PathBuf, File, transport_storage::MemoryTransport) {
    let source = f.outer.join("transport");
    let durable = f.outer.join("durable");
    private_dir(&source).unwrap();
    private_dir(&durable).unwrap();
    let file = OpenOptions::new().read(true).write(true).create_new(true)
        .mode(0o600).open(source.join("if1.terminal")).unwrap();
    file.set_len(2048).unwrap();
    let mut header = [0; 32];
    header[..4].copy_from_slice(b"LVIF");
    header[4..8].copy_from_slice(&1u32.to_le_bytes());
    header[8..12].copy_from_slice(&2048u32.to_le_bytes());
    header[16..32].copy_from_slice(&[1; 16]);
    file.write_all_at(&header, 0).unwrap();
    symlink(source.join("if1.terminal"), durable.join("if1.terminal")).unwrap();
    let metadata = fs::metadata(&source).unwrap();
    (source, durable, file, transport_storage::MemoryTransport { schema:1, device:metadata.dev(), inode:metadata.ino() })
}

fn publish(file: &File, counter: u64, progress: &Progress) {
    let bytes = progress.words.iter().flat_map(|word| word.to_le_bytes()).collect::<Vec<_>>();
    file.write_all_at(&bytes, 1088 + (counter & 1) * 256).unwrap();
    file.write_all_at(&counter.to_le_bytes(), 1024).unwrap();
}

#[test]
fn missing_unpublished_foreign_and_truncated_mappings_are_not_stopped_or_clean() {
    let f = Fixture::new();
    let (source, durable, file, identity) = mapping(&f);
    let first = read_mapping(&durable, &source, SESSION, identity.clone()).unwrap();
    assert!(first.progress.is_none());
    assert_eq!(first.terminal, Observation::Observed(None));
    publish(&file, 1, &progress(1, 1, 64, 11));
    assert_eq!(read_mapping(&durable, &source, SESSION, identity.clone()).unwrap().progress.unwrap().words[6], 64);
    assert!(read_mapping(&durable, &source, "02020202020202020202020202020202", identity.clone()).is_err());
    file.set_len(100).unwrap();
    assert!(read_mapping(&durable, &source, SESSION, identity.clone()).is_err());
    fs::remove_file(durable.join("if1.terminal")).unwrap();
    assert!(read_mapping(&durable, &source, SESSION, identity).is_err());
    assert_eq!(cleanup(&f.outer.join("report.json"), SESSION), CleanupState::NotReported);
}

#[test]
fn changing_progress_refuses_after_three_attempts_without_a_mixed_record() {
    let f = Fixture::new();
    let (_, _, file, _) = mapping(&f);
    publish(&file, 1, &progress(1, 1, 64, 11));
    let mut writes = 0;
    let result = read_progress(&file, SESSION, || {
        writes += 1;
        publish(&file, 1 + writes, &progress(1, 1, 64 + writes, 11));
    });
    assert!(result.is_err());
    assert_eq!(writes, 3);
    assert_eq!(read_progress(&file, SESSION, || {}).unwrap().unwrap().words[6], 67);
}

#[test]
fn replaced_mapping_cannot_publish_the_previous_inode_observation() {
    let f = Fixture::new();
    let (source, durable, file, identity) = mapping(&f);
    publish(&file, 1, &progress(1, 1, 64, 11));
    let result = read_mapping_with(&durable, &source, SESSION, identity, || {
        let replacement = source.join("replacement");
        fs::copy(source.join("if1.terminal"), &replacement).unwrap();
        fs::rename(replacement, source.join("if1.terminal")).unwrap();
    });
    assert!(result.is_err());
}

#[test]
fn terminal_failure_and_cleanup_are_independent_exact_observations() {
    let f = Fixture::new();
    let (_, _, file, _) = mapping(&f);
    let mut terminal = progress(1, 1, 64, 11);
    terminal.words[14] = 1;
    terminal.words[18] = 2;
    terminal.words[19] = 3;
    let bytes = terminal.words.iter().flat_map(|word| word.to_le_bytes()).collect::<Vec<_>>();
    file.write_all_at(&bytes, 384).unwrap();
    file.write_all_at(&2u64.to_le_bytes(), 64).unwrap();
    assert_eq!(read_terminal(&file, SESSION).unwrap(), Some(InstanceFailure::WindowsHostExited));
    let report = f.outer.join("windows.json");
    assert_eq!(cleanup(&report, SESSION), CleanupState::NotReported);
    let receipt = report.with_extension("ownership.json");
    atomic_json(&receipt, &serde_json::json!({"session":SESSION,"cleanup_confirmed":true,"transport_retired":false})).unwrap();
    assert_eq!(cleanup(&report, SESSION), CleanupState::Unconfirmed);
    atomic_json(&receipt, &serde_json::json!({"session":"another","cleanup_confirmed":true,"transport_retired":true})).unwrap();
    assert!(matches!(cleanup(&report, SESSION), CleanupState::Unavailable(_)));
    atomic_json(&receipt, &serde_json::json!({"session":SESSION,"cleanup_confirmed":true,"transport_retired":true})).unwrap();
    assert_eq!(cleanup(&report, SESSION), CleanupState::Confirmed);
}

fn lease(f: &Fixture, session: &str) -> PathBuf {
    lease_with_registration(f, session, &f.r)
}

fn lease_with_registration(f: &Fixture, session: &str, registration: &Registration) -> PathBuf {
    lease_for_peer(f, session, registration, 1, 20)
}

fn lease_for_peer(f: &Fixture, session: &str, registration: &Registration, pid: i32, ticks: u64) -> PathBuf {
    lease_with_kind(f, session, registration, pid, ticks, false)
}
fn lease_with_kind(f: &Fixture, session: &str, registration: &Registration, pid: i32, ticks: u64, detached: bool) -> PathBuf {
    let report = f.m.root.join("runtime/results").join(format!("windows-{session}.json"));
    let lease = f.m.root.join("runtime/leases").join(format!("{session}.json"));
    let path = registration.environment.root.join("compatdata/pfx/drive_c/bridge/sessions").join(session).join("owner.json");
    private_dir(path.parent().unwrap()).unwrap();
    private_dir(lease.parent().unwrap()).unwrap();
    private_dir(report.parent().unwrap()).unwrap();
    let document = serde_json::json!({"session":session,"report":report,"lease":lease,
        "registration":registration,"inspect":false,"keeper":false,"vendor_access":false,"detached":detached,"shared_runtime":detached,
        "graphical_session":{"schema":1,"peer_pid":pid,"peer_start_ticks":ticks,"display":":0"},
        "transport":{"schema":1,"device":1,"inode":2},
        "native_execution":{"engine":vec![0;32],"descriptor":vec![0;32]}});
    atomic_json(&path, &document).unwrap();
    f.m.retain_lease_owner(&path).unwrap();
    atomic_json(&lease, &report).unwrap();
    path
}

#[test]
fn census_keeps_admitted_product_after_republication_and_refuses_unresolved_leases() {
    let f = Fixture::new();
    let lock = f.m.lock("registry.lock").unwrap();
    lease(&f, SESSION);
    let mut replacement = f.r.clone();
    replacement.module.sha256 = "ab".repeat(32);
    let mut registry = f.m.registry().unwrap();
    registry.classes.insert(replacement.metadata.class_id.clone(), Entry {
        registration:replacement, publication:Publication::Published, managed_revision:None,
    });
    atomic_json(&f.m.root.join("registry.json"), &registry).unwrap();
    drop(lock);
    let mut observer = Observer::default();
    // No current publication lookup is needed; the exact old module remains bound.
    let activity = observer.sample(&f.m);
    let rows = activity.instances.observed().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].product.observed().unwrap().module_sha256, f.r.module.sha256);
    assert_eq!(rows[0].signal, SignalObservation::NotMeasured);
    assert!(matches!(rows[0].consumer, Observation::Unavailable(_)));
    assert!(matches!(rows[0].processing, ProcessingObservation::Unavailable { .. }));
    let unknown = f.m.root.join("runtime/leases/02020202020202020202020202020202.json");
    atomic_json(&unknown, &f.m.root.join("runtime/results/windows-02020202020202020202020202020202.json")).unwrap();
    assert!(matches!(observer.sample(&f.m).instances, Observation::Unavailable(_)));
    assert!(unknown.exists(), "Observation never erases unresolved ownership");
}

#[test]
fn two_equal_classes_remain_bound_to_their_distinct_admitted_setups() {
    let f = Fixture::new();
    let lock = f.m.lock("registry.lock").unwrap();
    lease(&f, SESSION);
    let mut sibling = f.r.clone();
    sibling.environment.id = "22222222-2222-4222-8222-222222222222".into();
    sibling.environment.root = f.m.root.join("environments").join(&sibling.environment.id);
    lease_with_registration(&f, "02020202020202020202020202020202", &sibling);
    drop(lock);
    let mut observer = Observer::default();
    let activity = observer.sample(&f.m);
    let rows = activity.instances.observed().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].product.observed().unwrap().class_id, rows[1].product.observed().unwrap().class_id);
    assert_ne!(rows[0].product.observed().unwrap().environment, rows[1].product.observed().unwrap().environment);
}

#[test]
fn consumer_requires_the_exact_before_launch_boot_record() {
    let f = Fixture::new();
    let path = lease(&f, SESSION);
    let document = read_json::<serde_json::Value>(&path).unwrap();
    let report = PathBuf::from(document["report"].as_str().unwrap());
    assert!(owner::lease_consumer(&f.m, SESSION, &report, &document).is_err());
    let generation = f.m.root.join("runtime/lease-generations").join(format!("{SESSION}.json"));
    private_dir(generation.parent().unwrap()).unwrap();
    let mut record = serde_json::json!({"schema":1,"session":SESSION,"report":report,
        "kernel_boot":"11111111-1111-4111-8111-111111111111","basis":"stopped_service_observation"});
    atomic_json(&generation, &record).unwrap();
    assert!(owner::lease_consumer(&f.m, SESSION, &report, &document).is_err());
    record["basis"] = "before_launch".into();
    atomic_json(&generation, &record).unwrap();
    assert_eq!(owner::lease_consumer(&f.m, SESSION, &report, &document).unwrap().start_ticks, 20);
    record["session"] = "02020202020202020202020202020202".into();
    atomic_json(&generation, &record).unwrap();
    assert!(owner::lease_consumer(&f.m, SESSION, &report, &document).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn live_process_identity_refuses_reused_ticks_and_prior_boot_without_claiming_cleanup() {
    let (socket, _) = std::os::unix::net::UnixStream::pair().unwrap();
    let peer = transport_storage::peer_process(&socket).unwrap();
    let mut identity = ConsumerIdentity {
        boot_id:owner::kernel_boot().unwrap().unwrap(), pid:peer.pid, uid:peer.uid, start_ticks:peer.start_ticks,
    };
    assert!(transport_storage::consumer_current(&identity).unwrap());
    identity.start_ticks += 1;
    assert!(!transport_storage::consumer_current(&identity).unwrap());
    identity.start_ticks = peer.start_ticks;
    identity.boot_id = "00000000-0000-4000-8000-000000000000".into();
    assert!(!transport_storage::consumer_current(&identity).unwrap());
}


#[cfg(target_os = "linux")]
fn host_fixture(f: &Fixture, pid: i32, ticks: u64, boot: &str) {
    // Actual admission owns this gate while creating the lease and generation.
    // The read-only observer must not create a missing registry lock for us.
    let _registry = f.m.lock("registry.lock").unwrap();
    let path = lease_for_peer(f, SESSION, &f.r, pid, ticks);
    let document = read_json::<serde_json::Value>(&path).unwrap();
    let generation = f.m.root.join("runtime/lease-generations").join(format!("{SESSION}.json"));
    private_dir(generation.parent().unwrap()).unwrap();
    atomic_json(&generation, &serde_json::json!({"schema":1,"session":SESSION,
        "report":document["report"],"kernel_boot":boot,"basis":"before_launch"})).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn host_self_report_requires_exact_authenticated_session_and_is_forgotten_on_restart() {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;
    let f = Fixture::new();
    let (mut server, mut client) = UnixStream::pair().unwrap();
    let peer = transport_storage::peer_process(&server).unwrap();
    host_fixture(&f, peer.pid, peer.start_ticks, &owner::kernel_boot().unwrap().unwrap());
    let hosts = Arc::new(HostNames::default());
    let send = |server: &mut UnixStream, client: &mut UnixStream, session, name| {
        let bytes = ap1_native_client::host_identity::encode(session, name).unwrap();
        client.write_all(&bytes[5..]).unwrap();
        hosts.receive(&f.m, server)
    };
    assert!(send(&mut server, &mut client, [2;16], "REAPER").is_err(), "foreign session cannot label another device");
    send(&mut server, &mut client, [1;16], "Unfamiliar host").unwrap();
    let mut ack=[0];client.read_exact(&mut ack).unwrap();assert_eq!(ack,[b'H']);
    let activity = Observer::with_hosts(hosts.clone()).sample(&f.m);
    let row = &activity.instances.observed().unwrap()[0];
    assert_eq!(row.daw, Observation::Observed(HostApplication { name:"Unfamiliar host".into(),
        source:HostIdentitySource::Vst3HostApplication }));
    assert_eq!(row.signal, SignalObservation::NotMeasured, "a name is no audio evidence");
    assert!(matches!(row.processing, ProcessingObservation::Unavailable { .. }));
    assert!(matches!(Observer::default().sample(&f.m).instances.observed().unwrap()[0].daw,
        Observation::Unavailable(_)), "a fresh service never infers the retained process's name");
    assert!(send(&mut server, &mut client, [1;16], "Different host").is_err(), "immutable self-report cannot retarget");
    let records=capture(&f.m).unwrap();let mut record=records.into_iter().next().unwrap();
    let consumer=row.consumer.clone();
    record.document["transport"]["inode"]=3.into();
    assert!(matches!(hosts.observed(&f.m,&record,&consumer),Observation::Unavailable(_)));
    record.document["transport"]["inode"]=2.into();
    record.document["registration"]["module"]["sha256"]="ab".repeat(32).into();
    assert!(matches!(hosts.observed(&f.m,&record,&consumer),Observation::Unavailable(_)));
    assert!(matches!(hosts.observed(&f.m,&record,&Observation::Unavailable("unreadable".into())),Observation::Unavailable(_)));
}

#[cfg(target_os = "linux")]
#[test]
fn host_report_refuses_other_process_reused_pid_and_prior_boot() {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    for change in 0..3 {
        let f=Fixture::new();let (mut server,mut client)=UnixStream::pair().unwrap();
        let peer=transport_storage::peer_process(&server).unwrap();
        let boot=owner::kernel_boot().unwrap().unwrap();
        host_fixture(&f,if change==0 {peer.pid+1} else {peer.pid},
            if change==1 {peer.start_ticks+1} else {peer.start_ticks},
            if change==2 {"00000000-0000-4000-8000-000000000000"} else {&boot});
        let hosts=HostNames::default();
        let bytes=ap1_native_client::host_identity::encode([1;16],"REAPER").unwrap();
        client.write_all(&bytes[5..]).unwrap();
        assert!(hosts.receive(&f.m,&mut server).is_err());
        let records=capture(&f.m).unwrap();
        let identity=owner::lease_consumer(&f.m,SESSION,&records[0].report,&records[0].document).unwrap();
        let claimed_current=Observation::Observed(ConsumerObservation {identity,current:Observation::Observed(true)});
        assert!(matches!(hosts.observed(&f.m,&records[0],&claimed_current),Observation::Unavailable(_)),
            "refused peer cannot seed even a later allegedly current read");
    }
}

#[test]
fn detached_instance_remains_visible_without_inventing_a_daw_or_audio_device() {
    let f=Fixture::new();
    {
        let _registry = f.m.lock("registry.lock").unwrap();
        lease_with_kind(&f,SESSION,&f.r,1,20,true);
    }
    let activity=Observer::default().sample(&f.m);
    let rows=activity.instances.observed().unwrap();assert_eq!(rows.len(),1);
    assert_eq!(rows[0].kind,InstanceKind::Standalone);
    assert_eq!(rows[0].product.observed().unwrap().module_sha256,f.r.module.sha256);
    assert!(matches!(rows[0].consumer,Observation::Unavailable(_)));
    assert!(matches!(rows[0].daw,Observation::Unavailable(_)));
    assert!(matches!(&rows[0].processing,ProcessingObservation::Unavailable { reason } if reason.contains("no DAW audio device")));
    assert_eq!(rows[0].signal,SignalObservation::NotMeasured);
}

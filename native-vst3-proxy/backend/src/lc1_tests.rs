//! Two-ended integration with tools/ap18-tests/lc1_session.cpp under the pinned
//! Windows runner. No vendor module, manager publication, DAW or editor is used.
use super::*;
use std::{
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

struct Run {
    child: Child,
    handle: u64,
}
impl Drop for Run {
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe {
                ap3_close(self.handle);
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn wait(shared: &Shared, mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(15);
    while !condition() {
        assert_eq!(
            shared.fault.load(Ordering::Acquire),
            0,
            "LC1 worker: {:?}",
            shared.detail.lock().unwrap()
        );
        assert!(Instant::now() < end, "LC1 bounded progress");
        thread::sleep(Duration::from_millis(1));
    }
}
fn wait_path(shared: &Shared, path: &std::path::Path) {
    wait(shared, || path.exists());
}
fn wait_request(shared: &Shared, root: &std::path::Path) -> ap1_native_client::Frame {
    let mut request = None;
    wait(shared, || {
        request = crate::mailbox::Mailbox::inspect_request(&root.join("ap10.delivery"), 15)
            .unwrap();
        request.is_some()
    });
    request.unwrap()
}
fn submit(handle: u64, shared: &Shared, epoch: u64, position: u64, n: usize) {
    let live = INSTANCES.lease(handle).unwrap();
    let callback = unsafe { &mut *live.callback.get() };
    let mut request = Item::control(AUDIO, 0);
    request.n = n as u32;
    request.gain = f64::NAN;
    request.data = [[0.25; CAP], [-0.5; CAP]];
    let mut out = [[0.; CAP]; 2];
    assert_eq!((callback.epoch, callback.position), (epoch, position));
    callback.process(shared, request, &mut out).unwrap();
}
fn take_result(shared: &Shared, processed: &mut u64, epoch: u64, position: u64, n: usize) {
    *processed += 1;
    wait(shared, || shared.processed.load(Ordering::Acquire) == *processed);
    let result = shared.results.pop().expect("exact completed result");
    assert_eq!((result.epoch, result.audio.position), (epoch, position));
    assert_eq!(result.audio.data[0][..n], vec![0.125; n]);
    assert_eq!(result.audio.data[1][..n], vec![-0.25; n]);
}
#[test]
#[ignore = "requires the built Windows LC1 fixture and pinned-runner launcher"]
fn same_session_reconfiguration_two_ended() {
        let _registry_owner = crate::registry_test();
    let root =
        PathBuf::from(std::env::var_os("LVB_LC1_DIRECTORY").expect("private empty test directory"));
    assert!(root.is_dir() && fs::read_dir(&root).unwrap().next().is_none());
    let launcher = std::env::var_os("LVB_LC1_LAUNCHER").expect("owned fixture launcher");
    let id = [0x1c; 16];
    // Launcher waits for the control file, then execs only the SDK fixture.
    let log = fs::File::create(root.join("windows.jsonl")).unwrap();
    let child = Command::new(launcher)
        .arg(&root)
        .arg("1c".repeat(16))
        .env("LVB_LC1_HOLD_STARTED", "3")
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let mut run = Run { child, handle: 0 };
    let mut session = Session::open_bound(&root, id, 256, 15, None).unwrap();
    // Fixture-only seed on both real sequence owners before any lifecycle call.
    session.state.next = 104684;
    // The fixture emits Ready only as a test barrier after its high-sequence
    // seed. It is not a Configure response and is consumed before the worker.
    let ready = ap1_native_client::endpoint::receive_version(&mut session.socket, 10, 15).unwrap();
    assert_eq!((ready.kind, ready.sequence, ready.session), (2, 0, id));
    assert!(ready.payload.is_empty());
    let mut fault_socket = session.socket.try_clone().unwrap();
    let mut shared = Shared::new();
    shared.gui = session.gui.clone();
    shared.state_capable.store(true, Ordering::Release);
    shared.ack.store(17, Ordering::Release);
    let shared = Arc::new(shared);
    let peer = shared.clone();
    let worker = thread::spawn(move || super::worker(session, peer, None, None));
    run.handle = INSTANCES
        .insert(|| {
            Ok::<_, io::Error>(Live {
                shared: shared.clone(),
                callback: UnsafeCell::new(Callback::new()),
                busy: AtomicBool::new(false),
                worker: Some(worker),
                report: None,
                max: 256,
                recovery_blocked: false,
                installed_delay: Some(512),
                delivery_mode: crate::performance::DeliveryMode::Buffered,
                minor: 15,
                setup: None,
            })
        })
        .unwrap()
        .unwrap();
    let handle = run.handle;
    let mut setup = crate::performance::wire_version(256, 0, 48000., true).unwrap();
    setup[20..24].copy_from_slice(&3u32.to_le_bytes());
    setup.extend(2u32.to_le_bytes());
    for direction in 0u32..2 {
        for value in [0, direction, 0, 2, 0, 1] {
            setup.extend(value.to_le_bytes());
        }
        setup.extend(3u64.to_le_bytes());
    }
    let mut processed = 0;
    let mut prior_early_sequence = None;
    let mut expected_after_epoch_two = 0;
    for epoch in 1..=3 {
        assert!(unsafe { control(handle, 20, setup.clone()) }.is_ok());
        if epoch == 2 {
            assert!(unsafe { control(handle, 20, setup.clone()) }.is_ok());
        }
        assert_eq!(unsafe { ap4_activate(handle, 256, 0) }, 0);
        if epoch == 2 && std::env::var_os("LVB_LC1_PRESTART_DEACTIVATE").is_some() {
            // Native phase 9 explicitly admits this VST3 lifecycle transition.
            // The retained live fault was worker_op=14 while Windows awaited Start.
            assert_eq!(unsafe { ap4_deactivate(handle) }, 0);
            assert!(unsafe { control(handle, 20, setup.clone()) }.is_ok());
            assert_eq!(unsafe { ap4_activate(handle, 256, 0) }, 0);
        }
        assert_eq!(unsafe { ap3_transition(handle, START) }, 0);
        let held = root.join(format!("lc1-start-held-{epoch}"));
        let release = root.join(format!("lc1-release-start-{epoch}"));
        wait_path(&shared, &held);
        if epoch == 2 {
            let wakes = shared.pending_start_wakes.load(Ordering::Acquire);
            shared.work.notify();
            wait(&shared, || shared.pending_start_wakes.load(Ordering::Acquire) > wakes);
        }
        if epoch == 3 {
            let saver = thread::spawn(move || unsafe { control(handle, 16, vec![]) });
            wait(&shared, || shared.pending_control.load(Ordering::Acquire));
            submit(handle, &shared, epoch, 0, 0);
            wait(&shared, || {
                shared.pending_start_control_fences.load(Ordering::Acquire) != 0
            });
            assert!(crate::mailbox::Mailbox::inspect_request(
                &root.join("ap10.delivery"), 15).unwrap().is_none(),
                "an admitted pre-audio control must retain its earlier barrier");
            fs::write(&release, b"release").unwrap();
            wait(&shared, || shared.ack.load(Ordering::Acquire) == (epoch << 8) | 11);
            let _capture = saver.join().unwrap();
            assert!(!shared.pending_control.load(Ordering::Acquire));
            let early = wait_request(&shared, &root);
            assert_eq!((early.kind, early.session), (ap1_native_client::PROCESS, id));
            assert_eq!((ap1_native_client::get(&early.payload[..4]),
                ap1_native_client::get(&early.payload[32..40]),
                ap1_native_client::get(&early.payload[40..48])), (0, epoch, 0));
            take_result(&shared, &mut processed, epoch, 0, 0);
        } else {
            submit(handle, &shared, epoch, 0, 0);
            let early = wait_request(&shared, &root);
            assert_eq!((early.kind, early.session), (ap1_native_client::PROCESS, id));
            assert_eq!((ap1_native_client::get(&early.payload[..4]),
                ap1_native_client::get(&early.payload[32..40]),
                ap1_native_client::get(&early.payload[40..48])), (0, epoch, 0));
            if let Some(sequence) = prior_early_sequence {
                assert!(early.sequence > sequence, "restart reused an earlier request identity");
            }
            prior_early_sequence = Some(early.sequence);
            assert_eq!(shared.ack.load(Ordering::Acquire) & 0xff, 9,
                "request publication must not grant Started");
            fs::write(&release, b"release").unwrap();
            wait(&shared, || shared.ack.load(Ordering::Acquire) == (epoch << 8) | 11);
            take_result(&shared, &mut processed, epoch, 0, 0);
            let lengths: &[usize] = if epoch == 1 { &[1, 0] } else { &[128] };
            let mut position = 0u64;
            for &n in lengths {
                submit(handle, &shared, epoch, position, n);
                take_result(&shared, &mut processed, epoch, position, n);
                position += n as u64;
            }
            if epoch == 2 {
                expected_after_epoch_two = early.sequence + lengths.len() as u64 + 1;
            }
        }
        assert_eq!(unsafe { ap3_transition(handle, STOP) }, 0);
        wait(&shared, || {
            shared.ack.load(Ordering::Acquire) == (epoch << 8) | 13
        });
        if epoch == 2 && std::env::var_os("LVB_LC1_BAD_DEACTIVATE_SEQUENCE").is_some() {
            // Fault injection only while the real queued worker is idle after
            // its acknowledged Stop. Windows must not accept next+1 here.
            use ap1_native_client::endpoint::{receive_version, send_version};
            let wrong = ap1_native_client::Frame {
                kind: 14,
                session: id,
                sequence: expected_after_epoch_two + 1,
                payload: vec![],
            };
            send_version(&mut fault_socket, &wrong, 5, 15).unwrap();
            let rejected = receive_version(&mut fault_socket, 5, 15).unwrap();
            assert_eq!(
                (rejected.kind, rejected.session, rejected.sequence),
                (7, id, expected_after_epoch_two)
            );
            assert_eq!(unsafe { ap3_close(handle) }, 2);
            run.handle = 0;
            eprintln!("LC1 wrong Deactivate sequence refused");
            return;
        }
        assert_eq!(unsafe { ap4_deactivate(handle) }, 0);
        assert_eq!(shared.ack.load(Ordering::Acquire), 15);
    }
    // A Start followed immediately by Stop has no audio authority to grant or
    // discard. Started and Stopped still retain their exact ordered custody.
    let epoch = 4;
    assert!(unsafe { control(handle, 20, setup.clone()) }.is_ok());
    assert_eq!(unsafe { ap4_activate(handle, 256, 0) }, 0);
    assert_eq!(unsafe { ap3_transition(handle, START) }, 0);
    assert_eq!(unsafe { ap3_transition(handle, STOP) }, 0);
    wait(&shared, || shared.ack.load(Ordering::Acquire) == (epoch << 8) | 13);
    assert_eq!(shared.processed.load(Ordering::Acquire), processed);
    assert_eq!(unsafe { ap4_deactivate(handle) }, 0);
    assert_eq!(shared.ack.load(Ordering::Acquire), 15);
    assert_eq!(unsafe { ap3_close(handle) }, 0);
    run.handle = 0;
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = run.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < end, "Windows fixture exit deadline");
        thread::sleep(Duration::from_millis(10));
    }
    eprintln!(
        "LC1 overlap passed: two early N0 epochs, control barrier, and empty Start/Stop"
    );
}

#[test]
#[ignore = "requires the built Windows LC1 fixture and pinned-runner launcher"]
fn refused_vendor_start_never_grants_audio_authority() {
    let _registry_owner = crate::registry_test();
    let root = PathBuf::from(
        std::env::var_os("LVB_LC1_REFUSAL_DIRECTORY")
            .expect("private empty start_refusal test directory"),
    );
    assert_eq!(root.file_name().and_then(|name| name.to_str()), Some("start_refusal"));
    assert!(root.is_dir() && fs::read_dir(&root).unwrap().next().is_none());
    let launcher = std::env::var_os("LVB_LC1_LAUNCHER").expect("owned fixture launcher");
    let id = [0x1c; 16];
    let log = fs::File::create(root.join("windows.jsonl")).unwrap();
    let child = Command::new(launcher)
        .arg(&root)
        .arg("1c".repeat(16))
        .env("LVB_LC1_REFUSE_FIRST_START", "1")
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let mut run = Run { child, handle: 0 };
    let mut session = Session::open_bound(&root, id, 256, 15, None).unwrap();
    session.state.next = 104684;
    let ready = ap1_native_client::endpoint::receive_version(&mut session.socket, 10, 15).unwrap();
    assert_eq!((ready.kind, ready.sequence, ready.session), (2, 0, id));
    let shared = Shared::new();
    shared.state_capable.store(true, Ordering::Release);
    shared.ack.store(17, Ordering::Release);
    let shared = Arc::new(shared);
    let peer = shared.clone();
    let worker = thread::spawn(move || super::worker(session, peer, None, None));
    run.handle = INSTANCES.insert(|| Ok::<_, io::Error>(Live {
        shared: shared.clone(),
        callback: UnsafeCell::new(Callback::new()),
        busy: AtomicBool::new(false),
        worker: Some(worker),
        report: None,
        max: 256,
        recovery_blocked: false,
        installed_delay: Some(512),
        delivery_mode: crate::performance::DeliveryMode::Buffered,
        minor: 15,
        setup: None,
    })).unwrap().unwrap();
    let handle = run.handle;
    let mut setup = crate::performance::wire_version(256, 0, 48000., true).unwrap();
    setup[20..24].copy_from_slice(&3u32.to_le_bytes());
    setup.extend(2u32.to_le_bytes());
    for direction in 0u32..2 {
        for value in [0, direction, 0, 2, 0, 1] { setup.extend(value.to_le_bytes()); }
        setup.extend(3u64.to_le_bytes());
    }
    assert!(unsafe { control(handle, 20, setup) }.is_ok());
    assert_eq!(unsafe { ap4_activate(handle, 256, 0) }, 0);
    assert_eq!(unsafe { ap3_transition(handle, START) }, 0);
    let until = Instant::now() + Duration::from_secs(15);
    while shared.fault.load(Ordering::Acquire) == 0 {
        assert!(Instant::now() < until, "refused Start containment deadline");
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(shared.processing_ready_epoch.load(Ordering::Acquire), 0);
    assert_eq!(shared.processed.load(Ordering::Acquire), 0);
    assert!(crate::mailbox::Mailbox::inspect_request(
        &root.join("ap10.delivery"), 15).unwrap().is_none());
    assert_eq!(unsafe { ap3_close(handle) }, 2);
    run.handle = 0;
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = run.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(Instant::now() < until, "refusal fixture exit deadline");
        thread::sleep(Duration::from_millis(10));
    }
}

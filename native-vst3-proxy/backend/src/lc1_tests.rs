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
fn wait(shared: &Shared, condition: impl Fn() -> bool) {
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
#[test]
#[ignore = "requires the built Windows LC1 fixture and pinned-runner launcher"]
fn same_session_reconfiguration_two_ended() {
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
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let mut run = Run { child, handle: 0 };
    let mut session = Session::open_bound(&root, id, 256, 12, None).unwrap();
    // Fixture-only seed on both real sequence owners before any lifecycle call.
    session.state.next = 104684;
    // The fixture emits Ready only as a test barrier after its high-sequence
    // seed. It is not a Configure response and is consumed before the worker.
    let ready = ap1_native_client::endpoint::receive_version(&mut session.socket, 10, 12).unwrap();
    assert_eq!((ready.kind, ready.sequence, ready.session), (2, 0, id));
    assert!(ready.payload.is_empty());
    let mut shared = Shared::new();
    shared.gui = session.gui.clone();
    shared.state_capable.store(true, Ordering::Release);
    shared.ack.store(17, Ordering::Release);
    let shared = Arc::new(shared);
    let peer = shared.clone();
    let worker = thread::spawn(move || super::worker(session, peer, None));
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
                minor: 12,
                setup: None,
            })
        })
        .unwrap()
        .unwrap();
    let handle = run.handle;
    let mut setup = crate::performance::wire(256, 0, 48000.).unwrap();
    setup[20..24].copy_from_slice(&1u32.to_le_bytes());
    setup.extend(2u32.to_le_bytes());
    for direction in 0u32..2 {
        for value in [0, direction, 0, 2, 0, 1] {
            setup.extend(value.to_le_bytes());
        }
        setup.extend(3u64.to_le_bytes());
    }
    for epoch in 1..=2 {
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
        wait(&shared, || {
            shared.ack.load(Ordering::Acquire) == (epoch << 8) | 11
        });
        let blocks = if epoch == 1 { 3 } else { 1 };
        for block in 0..blocks {
            let live = INSTANCES.lease(handle).unwrap();
            let cb = unsafe { &mut *live.callback.get() };
            let mut request = Item::control(AUDIO, 0);
            request.n = 256;
            request.gain = f64::NAN;
            request.data = [[0.25; CAP], [-0.5; CAP]];
            let mut out = [[0.; CAP]; 2];
            assert_eq!(cb.epoch, epoch);
            assert_eq!(cb.position, block * 256);
            cb.process(&shared, request, &mut out).unwrap();
            drop(live);
            let expected = if epoch == 1 { block + 1 } else { 4 };
            wait(&shared, || {
                shared.processed.load(Ordering::Acquire) == expected
            });
            let result = shared.results.pop().expect("exact completed result");
            assert_eq!((result.epoch, result.audio.position), (epoch, block * 256));
            assert_eq!(result.audio.data, [[0.125; CAP], [-0.25; CAP]]);
        }
        assert_eq!(unsafe { ap3_transition(handle, STOP) }, 0);
        assert_eq!(unsafe { ap4_deactivate(handle) }, 0);
        assert_eq!(shared.ack.load(Ordering::Acquire), 15);
    }
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
        "LC1 two intervals passed: Start2 sequence104687 epoch2 position0, four exact SDK results"
    );
}

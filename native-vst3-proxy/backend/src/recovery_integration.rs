//! Narrow SDK regression seam, derived from queued's two-instance transport
//! fixture. Only admission and opaque restore are supplied here. The production
//! worker exchanges ACTIVATE/START/audio with a deterministic socket peer.
use super::*;
use ap1_native_client::{
    endpoint::{receive_version, send_version},
    mapping::Mapping,
    ClientState, Frame, Slot, BLOCK_OUTPUT as OUTPUT, BLOCK_STRIDE as STRIDE,
};
use std::os::unix::fs::FileExt;

fn transport(generation: u64) -> (Arc<Shared>, thread::JoinHandle<()>) {
    let path = std::env::temp_dir().join(format!(
        "recovery-sdk-{:032x}",
        u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
    ));
    let mapping = Mapping::with_layout(&path, 64, true).unwrap();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let socket = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    socket.set_nodelay(true).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let (mut remote, _) = listener.accept().unwrap();
    remote.set_nodelay(true).unwrap();
    remote
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let peer = thread::spawn(move || {
        let mut next = 1;
        let mut started = false;
        while let Ok(f) = receive_version(&mut remote, 5, 14) {
            assert_eq!(f.sequence, next);
            if f.kind == 3 {
                next += 1;
            }
            let payload = match f.kind {
                8 => {
                    assert!(!started);
                    vec![]
                }
                10 => {
                    assert!(!started);
                    started = true;
                    f.payload.clone()
                }
                12 => {
                    assert!(started);
                    started = false;
                    f.payload.clone()
                }
                14 | 5 => {
                    assert!(!started);
                    vec![]
                }
                20 => {
                    let mut b = vec![0; 16];
                    b[8] = 1;
                    b
                }
                3 => {
                    assert!(started, "audio before actual peer START");
                    let n = ap1_native_client::get(&f.payload[..4]) as usize;
                    for ch in 0..2 {
                        let value: f32 = if ch == 0 { 0.25 } else { -0.5 };
                        for i in 0..n {
                            file.write_all_at(
                                &value.to_le_bytes(),
                                (OUTPUT + ch * STRIDE + 4 * (i + 1)) as u64,
                            )
                            .unwrap();
                        }
                    }
                    let mut b = vec![0; 72 + 64 + 16];
                    b[..4].copy_from_slice(&(n as u32).to_le_bytes());
                    b[4..8].copy_from_slice(&(OUTPUT as u32).to_le_bytes());
                    b[16..32].copy_from_slice(&f.payload[32..48]);
                    // One Note On and one automation point, through real wire decoding.
                    b[56] = 1;
                    b[60] = 1;
                    b[72 + 28..72 + 32].copy_from_slice(&42i32.to_le_bytes());
                    b[72 + 36..72 + 40].copy_from_slice(&60i32.to_le_bytes());
                    b[72 + 48..72 + 52].copy_from_slice(&0.75f32.to_le_bytes());
                    b[136 + 8..152].copy_from_slice(&0.375f64.to_le_bytes());
                    b
                }
                _ => panic!("unexpected fixture operation {}", f.kind),
            };
            send_version(
                &mut remote,
                &Frame {
                    kind: f.kind + 1,
                    session: f.session,
                    sequence: f.sequence,
                    payload,
                },
                5,
                14,
            )
            .unwrap();
            if f.kind == 5 {
                break;
            }
        }
    });
    let session = Session {
        notifications: None,
        configured_mode: 0,
        gui: None,
        gui_revision: 0,
        mailbox: None,
        mailbox_enabled: false,
        #[cfg(target_os = "linux")]
        direct_requested: false,
        capture: None,
        fault_status: None,
        notices: (0, 0),
        returned: Default::default(),
        processing: crate::ProcessingScratch::new(),
        mapping: Some(mapping),
        socket,
        state: ClientState {
            session: [4; 16],
            next: 1,
            slot: Slot::Writable,
        },
        phase: 17,
        max: CAP,
        minor: 14,
        identity: None,
        epoch: 0,
        position: 0,
        witness: None,
        trace: Default::default(),
        sample_rate: 48000,
        armed: false,
        owner: None,
    };
    let mut s = Shared::try_new().unwrap();
    s.generation = generation;
    let shared = Arc::new(s);
    shared.state_capable.store(true, Ordering::Release);
    shared.ack.store(17, Ordering::Release);
    let service = shared.clone();
    let worker = thread::spawn(move || {
        super::worker(session, service, None, None);
        peer.join().unwrap();
        let _ = std::fs::remove_file(path);
    });
    (shared, worker)
}

#[no_mangle]
pub extern "C" fn lvb_test_recovery_open() -> u64 {
    let (shared, worker) = transport(7);
    INSTANCES
        .insert(|| {
            Ok::<_, ()>(Live {
                shared,
                callback: UnsafeCell::new(Callback::new()),
                busy: AtomicBool::new(false),
                worker: Some(worker),
                report: None,
                max: 128,
                recovery_blocked: false,
                proxy: None,
                installed_delay: Some(512),
                delivery_mode: crate::performance::DeliveryMode::Buffered,
                minor: 14,
                setup: None,
            })
        })
        .unwrap()
        .unwrap()
}

#[no_mangle]
pub extern "C" fn lvb_test_recovery_begin(id: u64, maximum: u32, mode: u32) {
    let control = crate::proxy_recovery::Control::new([1; 16], std::path::PathBuf::new());
    INSTANCES
        .update(id, |live| {
            // Retire the old transport, then publish a distinct healthy replacement
            // with the same retained callback configuration as recover_state_bound.
            live.shared.quit.store(true, Ordering::Release);
            live.shared.work.notify();
            live.worker.take().unwrap().join().unwrap();
            let (shared, worker) = transport(8);
            shared
                .prepare_outputs(live.setup.as_ref().unwrap())
                .unwrap();
            live.callback = UnsafeCell::new(live.callback.get_mut().replacement());
            live.shared = shared;
            live.worker = Some(worker);
            live.proxy = Some(control.clone());
        })
        .unwrap();
    control.integration_begin(id, maximum, mode);
}

#[no_mangle]
pub extern "C" fn lvb_test_recovery_started(id: u64) -> u32 {
    u32::from(wait_started(id).is_ok())
}

#[no_mangle]
pub extern "C" fn lvb_test_recovery_present(id: u64) -> u32 {
    u32::from(INSTANCES.lease(id).is_some())
}

// Also callable from a tiny local driver on hosts where unrelated Linux-only
// unit tests prevent building the crate's test binary.
#[no_mangle]
pub extern "C" fn lvb_test_recovery_smoke() {
    let id = lvb_test_recovery_open();
    unsafe {
        let mut latency = [0; 3];
        assert_eq!(
            setup(
                id,
                128,
                0,
                48000.,
                &[],
                false,
                latency.as_mut_ptr(),
                latency.as_mut_ptr().add(2)
            ),
            0
        );
        assert_eq!(ap4_activate(id, 128, 0), 0);
        assert_eq!(ap3_transition(id, START), 0);
        assert_eq!(lvb_test_recovery_started(id), 1);
        let input = [0.; 128];
        let mut left = [0.; 128];
        let mut right = [0.; 128];
        let out = [left.as_mut_ptr(), right.as_mut_ptr()];
        let mut audible = false;
        for _ in 0..100 {
            let mut flags = 0;
            let mut delivery = Delivery::default();
            assert_eq!(
                ap23_process_outputs(
                    id,
                    128,
                    0,
                    std::ptr::null(),
                    0,
                    &crate::context::Context::default(),
                    0,
                    input.as_ptr(),
                    input.as_ptr(),
                    out.as_ptr(),
                    2,
                    &mut flags,
                    &mut delivery,
                    0
                ),
                0
            );
            let mut results = crate::process_results::Packet::default();
            assert_eq!(ap10_take_results(id, &mut results), 0);
            assert_eq!(ap23_finish_callback(id), 0);
            if left.iter().all(|v| *v == 0.25) && right.iter().all(|v| *v == -0.5) {
                assert_eq!((results.events, results.points), (1, 1));
                audible = true;
                break;
            }
            thread::sleep(Duration::from_millis(3));
        }
        assert!(
            audible,
            "socket peer must exercise decoded audio and results"
        );
        lvb_test_recovery_begin(id, 128, 0);
        let mut event = crate::proxy_recovery::Event::default();
        let mut bytes = crate::state::OwnedState {
            data: std::ptr::null(),
            length: 0,
            abi_version: 1,
        };
        assert_eq!(
            crate::proxy_recovery::lvb_recovery_poll_v1(id, 1, &mut event, &mut bytes),
            0
        );
        assert_eq!(event.stage, 2);
        crate::state::ap4_state_release_v1(&mut bytes);
        assert_eq!(
            crate::proxy_recovery::lvb_recovery_ack_v1(id, [2; 16].as_ptr(), 2, 1),
            0
        );
        let until = Instant::now() + Duration::from_secs(3);
        while crate::proxy_recovery::lvb_test_recovery_phase(id) != 3 {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(3));
        }
        assert_eq!(lvb_test_recovery_started(id), 1);
        let began = Instant::now();
        while crate::proxy_recovery::lvb_test_recovery_phase(id) != 4 {
            assert!(began.elapsed() < Duration::from_secs(7));
            thread::sleep(Duration::from_millis(3));
        }
        assert!(began.elapsed() > Duration::from_secs(4));
        assert_ne!(
            crate::proxy_recovery::lvb_recovery_ack_v1(id, [2; 16].as_ptr(), 3, 1),
            0
        );
        // No supervisor terminal record in this fixture: do not relabel
        // failed transport cleanup as a confirmed contained close.
        let settled = Instant::now() + Duration::from_secs(2);
        while crate::proxy_recovery::lookup(id)
            .unwrap()
            .integration_busy()
        {
            assert!(Instant::now() < settled);
            thread::sleep(Duration::from_millis(3));
        }
        assert_eq!(if2_close(id), 2);
        assert_eq!(lvb_test_recovery_present(id), 0);
    }
}

#[cfg(test)]
#[test]
fn socket_peer_exercises_real_start_audio_results_and_final_timeout() {
    let _registry = crate::registry_test();
    lvb_test_recovery_smoke();
}

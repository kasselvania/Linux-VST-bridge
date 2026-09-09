//! Real callback -> queue -> transport with a delayed/fragmented state peer.
use super::*;
use ap1_native_client::{
    endpoint::{receive_version, send_version},
    get,
    mapping::Mapping,
    ClientState, Frame, Slot, INPUT, OUTPUT, STRIDE,
};
use std::io::Write;
use std::os::unix::fs::FileExt;
fn until(mut f: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    while !f() {
        assert!(Instant::now() < end, "capture test progress deadline");
        thread::sleep(Duration::from_micros(50));
    }
}
#[test]
fn pending_save_does_not_hold_parent_callback_batches_or_replace_a_refused_snapshot() {
    let dir = std::env::temp_dir().join(format!(
        "ap13-capture-{:032x}",
        u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
    ));
    std::fs::create_dir(&dir).unwrap();
    let mapping = Mapping::new(&dir.join("audio")).unwrap();
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(dir.join("audio"))
        .unwrap();
    let mailbox = crate::mailbox::Mailbox::create(&dir.join("mailbox"), [13; 16]).unwrap();
    let mut peer_box = mailbox.counterpart();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let socket = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut socket_peer, _) = listener.accept().unwrap();
    let entered = Arc::new(AtomicU64::new(0));
    let entered_peer = entered.clone();
    let quit = Arc::new(AtomicBool::new(false));
    let peer_quit = quit.clone();
    let peer = thread::spawn(move || {
        let mut active = true;
        let mut next = 1;
        let mut position = 0;
        let mut saves = 0;
        let mut pending = None;
        let mut writers = vec![];
        while !peer_quit.load(Ordering::Acquire) {
            let Some(mut f) = (if active {
                peer_box.take_request(12)
            } else {
                Some(receive_version(&mut socket_peer, 5, 12).unwrap())
            }) else {
                thread::sleep(Duration::from_micros(50));
                continue;
            };
            if f.kind == 0 {
                f = receive_version(&mut socket_peer, 5, 12).unwrap();
            }
            assert_eq!((f.sequence, f.session), (next, [13; 16]));
            match f.kind {
                16 => {
                    saves += 1;
                    next += 1;
                    pending = Some((f, position + 4096, saves));
                    entered_peer.store(saves, Ordering::Release);
                }
                3 => {
                    assert_eq!(get(&f.payload[32..40]), 1);
                    assert_eq!(get(&f.payload[40..48]), position);
                    assert_eq!(get(&f.payload[..4]), 256);
                    for ch in 0..2 {
                        let mut samples = [0u8; 1024];
                        file.read_exact_at(&mut samples, (INPUT + ch * STRIDE + 4) as u64)
                            .unwrap();
                        file.write_all_at(&samples, (OUTPUT + ch * STRIDE + 4) as u64)
                            .unwrap();
                    }
                    let mut b = vec![0; 72];
                    b[..4].copy_from_slice(&256u32.to_le_bytes());
                    b[4..8].copy_from_slice(&(OUTPUT as u32).to_le_bytes());
                    b[16..32].copy_from_slice(&f.payload[32..48]);
                    peer_box.respond(
                        Frame {
                            kind: 4,
                            session: f.session,
                            sequence: f.sequence,
                            payload: b,
                        },
                        12,
                    );
                    next += 1;
                    position += 256;
                    if pending
                        .as_ref()
                        .is_some_and(|(_, through, _)| position >= *through)
                    {
                        let (request, _, count) = pending.take().unwrap();
                        let mut sender = socket_peer.try_clone().unwrap();
                        writers.push(thread::spawn(move || {
                            let (kind, payload) = if count == 1 {
                                let mut p = vec![0; 16 + 262144 + 16];
                                p[..4].copy_from_slice(&262144u32.to_le_bytes());
                                p[8..12].copy_from_slice(&1u32.to_le_bytes());
                                p[12..16].copy_from_slice(&2u32.to_le_bytes());
                                p[16..16 + 262144].fill(0xD3);
                                p[16 + 262144..20 + 262144].copy_from_slice(&42u32.to_le_bytes());
                                (17, p)
                            } else {
                                (
                                    7,
                                    [1u32, 16, 1, 1]
                                        .into_iter()
                                        .flat_map(u32::to_le_bytes)
                                        .collect(),
                                )
                            };
                            let b = Frame {
                                kind,
                                session: request.session,
                                sequence: request.sequence,
                                payload,
                            }
                            .encode_version(12)
                            .unwrap();
                            // Force a partial header, then a payload larger than the per-turn read budget.
                            sender.write_all(&b[..7]).unwrap();
                            thread::sleep(Duration::from_millis(2));
                            sender.write_all(&b[7..]).unwrap();
                        }));
                    }
                }
                12 | 14 | 5 => {
                    active = false;
                    send_version(
                        &mut socket_peer,
                        &Frame {
                            kind: f.kind + 1,
                            session: f.session,
                            sequence: f.sequence,
                            payload: f.payload,
                        },
                        5,
                        12,
                    )
                    .unwrap();
                    if f.kind == 5 {
                        break;
                    }
                }
                _ => panic!("unexpected control"),
            }
        }
        for writer in writers {
            writer.join().unwrap();
        }
    });
    let identity = Some(state::Identity {
        class: [13; 16],
        module: [14; 32],
    });
    let session = Session {
        gui: None,
        gui_revision: 0,
        mapping: Some(mapping),
        mailbox: Some(mailbox),
        mailbox_enabled: true,
        capture: None,
        fault_status: None,
        notices: (0, 0),
        returned: Default::default(),
        socket,
        state: ClientState {
            session: [13; 16],
            next: 1,
            slot: Slot::Writable,
        },
        phase: 11,
        max: 256,
        minor: 12,
        epoch: 1,
        position: 0,
        witness: None,
        identity,
        trace: Default::default(),
        sample_rate: 48000,
        armed: false,
        owner: None,
    };
    let mut shared = Shared::new();
    shared.identity = identity;
    shared.state_capable.store(true, Ordering::Release);
    shared.wanted.store(1, Ordering::Release);
    let shared = Arc::new(shared);
    let mut callback = Callback::new();
    callback.running = true;
    callback.epoch = 1;
    callback.delay = 512;
    let service = shared.clone();
    let worker = thread::spawn(move || worker(session, service, None));
    let id = INSTANCES
        .insert(|| {
            Ok::<_, ()>(Live {
                shared: shared.clone(),
                callback: UnsafeCell::new(callback),
                busy: AtomicBool::new(false),
                worker: Some(worker),
                report: None,
                max: 512,
                recovery_blocked: false,
                installed_delay: Some(512),
                minor: 12,
                setup: None,
            })
        })
        .unwrap()
        .unwrap();
    let mut parents = 0u64;
    let mut previous = [0f32; 512];
    let mut original = None;
    for save in 1..=2 {
        let saver = thread::spawn(move || unsafe { control(id, 16, vec![]) });
        until(|| entered.load(Ordering::Acquire) == save);
        for parent in 0..24 {
            let input = std::array::from_fn::<_, 512, _>(|i| {
                ((parents * 512 + i as u64) % 8093) as f32 / 8192.
            });
            let mut left = [0.; 512];
            let mut right = [0.; 512];
            let mut flags = 0;
            let mut d = Delivery::default();
            assert_eq!(
                unsafe {
                    ap13_process(
                        id,
                        512,
                        std::ptr::null(),
                        0,
                        &crate::context::Context::default(),
                        0,
                        input.as_ptr(),
                        input.as_ptr(),
                        left.as_mut_ptr(),
                        right.as_mut_ptr(),
                        &mut flags,
                        &mut d,
                        crate::observer::monotonic_ns(),
                    )
                },
                0
            );
            assert_eq!(left, previous);
            assert_eq!(right, previous);
            assert_eq!(d.missing_frames, 0);
            assert_eq!(d.expired_frames, 0);
            previous = input;
            parents += 1;
            // External host driver waits only after the entire 512-frame callback.
            until(|| {
                shared.processed.load(Ordering::Acquire) >= parents * 2
                    || shared.fault.load(Ordering::Acquire) != 0
            });
            assert_eq!(
                shared.fault.load(Ordering::Acquire),
                0,
                "{:?}",
                shared.detail.lock().unwrap()
            );
            if parent < 6 {
                assert!(
                    !saver.is_finished(),
                    "save cannot finish before its deliberately held reply"
                );
            }
        }
        let result = saver.join().unwrap();
        if save == 1 {
            let bytes = result.unwrap();
            assert!(bytes.len() > 262144);
            original = Some(bytes);
        } else {
            assert!(state::save_refused(&result.unwrap_err()));
        }
        let snapshot = shared.snapshots.lock().unwrap().latest().unwrap().clone();
        assert_eq!(snapshot.revision, 1);
        assert_eq!(snapshot.bytes, *original.as_ref().unwrap());
    }
    assert_eq!(unsafe { ap3_transition(id, STOP) }, 0);
    until(|| shared.ack.load(Ordering::Acquire) == (1 << 8) | 13);
    assert_eq!(unsafe { ap3_transition(id, DEACTIVATE) }, 0);
    assert_eq!(unsafe { ap3_close(id) }, 0);
    quit.store(true, Ordering::Release);
    peer.join().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

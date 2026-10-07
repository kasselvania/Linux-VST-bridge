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
fn processing_restore_waits_for_stop_ack_and_restarts_the_same_session_at_a_fresh_epoch() {
    let _registry_owner = crate::registry_test();
    let directory = std::env::temp_dir().join(format!(
        "processing-restore-{:032x}",
        u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
    ));
    std::fs::create_dir(&directory).unwrap();
    let mapping = Mapping::new(&directory.join("audio")).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let socket = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut remote, _) = listener.accept().unwrap();
    let (stop_seen, await_stop) = std::sync::mpsc::channel();
    let (release_stop, await_release) = std::sync::mpsc::channel();
    let mut payload = vec![0; 24];
    payload[0] = 8;
    payload[12] = 2;
    payload[16..].copy_from_slice(&0.375f64.to_le_bytes());
    let peer_payload = payload.clone();
    let peer = thread::spawn(move || {
        let stopped = receive_version(&mut remote, 3, 15).unwrap();
        assert_eq!((stopped.kind, stopped.session, stopped.sequence), (12, [31; 16], 1));
        assert_eq!(get(&stopped.payload), 1);
        stop_seen.send(()).unwrap();
        await_release.recv_timeout(Duration::from_secs(3)).unwrap();
        // The real worker is still awaiting this STOP acknowledgment. An
        // admitted restore must not already have crossed the socket boundary.
        remote.set_nonblocking(true).unwrap();
        let mut byte = [0];
        assert_eq!(remote.peek(&mut byte).unwrap_err().kind(), io::ErrorKind::WouldBlock);
        remote.set_nonblocking(false).unwrap();
        send_version(&mut remote, &Frame { kind: 13, ..stopped }, 3, 15).unwrap();
        let restored = receive_version(&mut remote, 3, 15).unwrap();
        assert_eq!((restored.kind, restored.session, restored.sequence), (18, [31; 16], 1));
        assert_eq!(restored.payload, peer_payload);
        send_version(&mut remote, &Frame { kind: 19, ..restored }, 3, 15).unwrap();
        let started = receive_version(&mut remote, 3, 15).unwrap();
        assert_eq!((started.kind, started.session, started.sequence), (10, [31; 16], 2));
        assert_eq!(get(&started.payload), 2);
        send_version(&mut remote, &Frame { kind: 11, ..started }, 3, 15).unwrap();
        for expected in [12, 14, 5] {
            let request = receive_version(&mut remote, 3, 15).unwrap();
            assert_eq!((request.kind, request.session, request.sequence), (expected, [31; 16], 2));
            if expected == 12 { assert_eq!(get(&request.payload), 2); }
            send_version(&mut remote, &Frame { kind: expected + 1, ..request }, 3, 15).unwrap();
        }
    });
    let identity = Some(state::Identity { class: [31; 16], module: [32; 32] });
    let session = Session {
        gui: None, gui_revision: 0, mapping: Some(mapping), socket,
        mailbox: None, mailbox_enabled: false,
        #[cfg(target_os = "linux")]
        direct_requested: false,
        notifications: None, configured_mode: 0, capture: None, fault_status: None,
        notices: (0, 0), returned: Default::default(), processing: crate::ProcessingScratch::new(),
        state: ClientState { session: [31; 16], next: 1, slot: Slot::Writable },
        phase: 11, max: 64, minor: 15, identity, epoch: 1, position: 0,
        witness: None, trace: Default::default(), sample_rate: 48000, armed: false, owner: None,
    };
    let mut shared = Shared::new();
    shared.identity = identity;
    shared.state_capable.store(true, Ordering::Release);
    shared.wanted.store(1, Ordering::Release);
    let shared = Arc::new(shared);
    let mut callback = Callback::new();
    callback.prepare(64, 2, 0);
    callback.running = true;
    callback.epoch = 1;
    let service = shared.clone();
    let worker = thread::spawn(move || worker(session, service, None, None));
    let id = INSTANCES.insert(|| Ok::<_, ()>(Live {
        shared: shared.clone(), callback: UnsafeCell::new(callback), busy: AtomicBool::new(false),
        worker: Some(worker), report: None, max: 64, recovery_blocked: false,
        installed_delay: Some(0), delivery_mode: crate::performance::DeliveryMode::SameCallback,
        minor: 15, setup: None,
    })).unwrap().unwrap();
    assert_eq!(unsafe { ap3_transition(id, STOP) }, 0);
    await_stop.recv_timeout(Duration::from_secs(3)).unwrap();
    let saved = state::bound_envelope(identity, &payload).unwrap();
    let restore = thread::spawn(move || unsafe {
        let mut owned = state::OwnedState { data: std::ptr::null(), length: 0, abi_version: 1 };
        assert_eq!(ap4_state_owned_v1(id, saved.as_ptr(), saved.len() as u32, &mut owned), 0);
        let captured = std::slice::from_raw_parts(owned.data, owned.length as usize).to_vec();
        state::ap4_state_release_v1(&mut owned);
        captured
    });
    until(|| shared.pending_control.load(Ordering::Acquire));
    assert!(!restore.is_finished());
    release_stop.send(()).unwrap();
    let captured = restore.join().unwrap();
    assert_eq!(state::bound_payload(identity, &captured).unwrap(), payload);
    assert_eq!(shared.ack.load(Ordering::Acquire), (1 << 8) | 13);
    assert_eq!(shared.curve_state_revision.load(Ordering::Acquire), 1);
    assert_eq!(unsafe { ap3_transition(id, START) }, 0);
    until(|| shared.ack.load(Ordering::Acquire) == (2 << 8) | 11);
    assert_eq!(shared.processing_ready_epoch.load(Ordering::Acquire), 2);
    assert_eq!(unsafe { ap3_transition(id, STOP) }, 0);
    until(|| shared.ack.load(Ordering::Acquire) == (2 << 8) | 13);
    assert_eq!(unsafe { ap3_transition(id, DEACTIVATE) }, 0);
    assert_eq!(unsafe { ap3_close(id) }, 0);
    peer.join().unwrap();
    assert_eq!(shared.fault.load(Ordering::Acquire), 0);
    std::fs::remove_dir_all(directory).unwrap();
}
#[test]
fn pending_save_does_not_hold_parent_callback_batches_or_replace_a_refused_snapshot() {
        let _registry_owner = crate::registry_test();
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
                                let mut p = vec![0; 16 + 2097152 + 16];
                                p[..4].copy_from_slice(&2097152u32.to_le_bytes());
                                p[8..12].copy_from_slice(&1u32.to_le_bytes());
                                p[12..16].copy_from_slice(&2u32.to_le_bytes());
                                p[16..16 + 2097152].fill(0xD3);
                                p[16 + 2097152..20 + 2097152].copy_from_slice(&42u32.to_le_bytes());
                                (17, p)
                            } else {
                                (
                                    7,
                                    [1u32, 16, 3, 1]
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
            #[cfg(target_os="linux")] direct_requested:false, notifications:None, configured_mode:0,
        capture: None,
        fault_status: None,
        notices: (0, 0),
        returned: Default::default(),
        processing: crate::ProcessingScratch::new(),
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
    let worker = thread::spawn(move || worker(session, service, None, None));
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
                delivery_mode: crate::performance::DeliveryMode::Buffered,
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
            // Processing precedes queue publication in the production worker;
            // that statistic cannot synchronize a deterministic next callback.
            until(|| {
                shared.results.published() >= parents * 2
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
            assert!(bytes.len() > 2097152);
            original = Some(bytes);
        } else {
            let error=result.unwrap_err();
            assert!(state::save_refused(&error));
            assert!(error.to_string().contains("256 MiB save limit"));
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

#[test]
fn offline_audio_longer_than_capture_deadline_publishes_its_save_before_audio_completion() {
    let _registry_owner=crate::registry_test();
    let directory=std::env::temp_dir().join(format!("ipc15-offline-capture-{:032x}",u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
    std::fs::create_dir(&directory).unwrap();
    let mapping=Mapping::with_layout(&directory.join("audio"),ap1_native_client::MULTI_CHANNELS,true).unwrap();
    let file=std::fs::OpenOptions::new().read(true).write(true).open(directory.join("audio")).unwrap();
    let mailbox=crate::mailbox::Mailbox::create(&directory.join("mailbox"),[23;16]).unwrap();
    let mut peer_box=mailbox.counterpart();
    let pair=|| {
        let listener=std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let socket=std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (peer, _)=listener.accept().unwrap();(socket,peer)
    };
    let (socket,mut control_peer)=pair();let (notification,mut notification_peer)=pair();
    let entered=Arc::new(AtomicBool::new(false));let peer_entered=entered.clone();
    let quit=Arc::new(AtomicBool::new(false));let peer_quit=quit.clone();
    let peer=thread::spawn(move || {
        let capture=receive_version(&mut control_peer,2,15).unwrap();
        assert_eq!((capture.kind,capture.sequence),(16,1));
        until(|| peer_box.take_request(15).is_some_and(|frame|frame.kind==0));
        notification_peer.write_all(&[1]).unwrap();peer_entered.store(true,Ordering::Release);
        let audio=loop { if let Some(frame)=peer_box.take_request(15) { break frame; }thread::yield_now(); };
        assert_eq!((audio.kind,audio.sequence,get(&audio.payload[..4])),(3,2,64));
        assert_eq!(get(&audio.payload[audio.payload.len()-8..audio.payload.len()-4]),2);
        let started=Instant::now();
        // A genuine admitted state frame arrives while this single DSP reply
        // stays outstanding past Capture's original 10-second allowance.
        let mut state=vec![0;16+65536];state[..4].copy_from_slice(&65536u32.to_le_bytes());state[16..].fill(0xd3);
        let reply=Frame{kind:17,session:capture.session,sequence:capture.sequence,payload:state}.encode_version(15).unwrap();
        control_peer.write_all(&reply[..7]).unwrap();thread::sleep(Duration::from_millis(2));control_peer.write_all(&reply[7..]).unwrap();
        while started.elapsed()<Duration::from_millis(10200) { thread::sleep(Duration::from_millis(4)); }
        for ch in 0..2 {
            let mut samples=[0;64*4];file.read_exact_at(&mut samples,(ap1_native_client::INPUT+ch*ap1_native_client::BLOCK_STRIDE+4) as u64).unwrap();
            file.write_all_at(&samples,(ap1_native_client::BLOCK_OUTPUT+ch*ap1_native_client::BLOCK_STRIDE+4) as u64).unwrap();
        }
        let mut payload=vec![0;72];payload[..4].copy_from_slice(&64u32.to_le_bytes());payload[4..8].copy_from_slice(&(ap1_native_client::BLOCK_OUTPUT as u32).to_le_bytes());payload[16..32].copy_from_slice(&audio.payload[32..48]);
        peer_box.respond(Frame{kind:4,session:audio.session,sequence:audio.sequence,payload},15);notification_peer.write_all(&[1]).unwrap();
        until(||peer_quit.load(Ordering::Acquire));
    });
    let identity=Some(state::Identity{class:[23;16],module:[24;32]});
    let session=Session{gui:None,gui_revision:0,mapping:Some(mapping),mailbox:Some(mailbox),mailbox_enabled:true,
            #[cfg(target_os="linux")] direct_requested:false,
        notifications:Some(ap1_native_client::notification::Channel::new(notification).unwrap()),configured_mode:2,capture:None,
        fault_status:None,notices:(0,0),returned:Default::default(),processing:crate::ProcessingScratch::new(),socket,
        state:ClientState{session:[23;16],next:1,slot:Slot::Writable},phase:11,max:64,minor:15,epoch:1,position:0,
        witness:None,identity,trace:Default::default(),sample_rate:48000,armed:false,owner:None};
    let mut shared=Shared::new();shared.identity=identity;shared.state_capable.store(true,Ordering::Release);shared.wanted.store(1,Ordering::Release);
    let shared=Arc::new(shared);let mut callback=Callback::new();callback.running=true;callback.epoch=1;callback.delay=0;
    let service=shared.clone();let worker=thread::spawn(move ||worker(session,service,None,None));
    let id=INSTANCES.insert(||Ok::<_,()>(Live{shared:shared.clone(),callback:UnsafeCell::new(callback),busy:AtomicBool::new(false),worker:Some(worker),report:None,
        max:64,recovery_blocked:false,installed_delay:Some(256),delivery_mode:crate::performance::DeliveryMode::Buffered,minor:15,
        setup:Some(crate::performance::wire_version(64,2,48000.,true).unwrap())})).unwrap().unwrap();
    let saver=thread::spawn(move ||unsafe{control(id,16,vec![])});until(||entered.load(Ordering::Acquire));
    let audio=thread::spawn(move || {
        let input=[0.375;64];let mut output=[[9.;64];2];let pointers=[output[0].as_mut_ptr(),output[1].as_mut_ptr()];let mut flags=0;let mut delivery=Delivery::default();
        let started=Instant::now();let (result,allocations)=crate::allocation_test::measure(||unsafe {
            ap23_process_outputs(id,64,2,std::ptr::null(),0,&crate::context::Context::default(),0,input.as_ptr(),input.as_ptr(),pointers.as_ptr(),2,&mut flags,&mut delivery,crate::observer::monotonic_ns())
        });(result,allocations,output,delivery,started.elapsed())
    });
    until(||saver.is_finished());let saved=saver.join().unwrap().unwrap();
    assert!(!audio.is_finished(),"save acknowledgment waited behind the outstanding offline AUDIO");
    assert_eq!(state::bound_payload(identity,&saved).unwrap()[16..],[0xd3;65536]);
    assert!(!shared.pending_control.load(Ordering::Acquire));assert_eq!(shared.snapshots.lock().unwrap().latest().unwrap().revision,1);
    let (result,allocations,output,delivery,elapsed)=audio.join().unwrap();
    assert_eq!(result,0);assert_eq!(allocations,[0;3]);assert_eq!(output,[[0.375;64];2]);assert_eq!(delivery.missing_frames,0);assert!(elapsed>=Duration::from_secs(10));assert_eq!(shared.fault.load(Ordering::Acquire),0);
    INSTANCES.remove(id,|live| {live.shared.quit.store(true,Ordering::Release);live.shared.work.notify();live.worker.take().unwrap().join().unwrap();}).unwrap();
    quit.store(true,Ordering::Release);peer.join().unwrap();std::fs::remove_dir_all(directory).unwrap();
}

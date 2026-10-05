//! Discrimination tests use the real Session/IPC15 worker and owned mailbox.
//! The peer is protocol instrumentation, not a Windows DSP timing fixture.
use super::*;
use ap1_native_client::{mapping::Mapping, notification::Channel, ClientState, Frame, Slot};
use std::{fs::File, io::Write, net::TcpStream, os::unix::fs::FileExt, path::PathBuf};

fn sockets() -> (TcpStream, TcpStream) {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (peer, _) = listener.accept().unwrap();
    (client, peer)
}
fn await_predicate(mut ready: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(3);
    while !ready() {
        assert!(Instant::now() < until, "test owner did not reach the declared boundary");
        thread::yield_now();
    }
}
struct Peer {
    mailbox: crate::mailbox::Mailbox,
    wake: TcpStream,
    samples: File,
    output: usize,
    stride: usize,
    channels: usize,
}
impl Peer {
    fn request_or_fault(&mut self, shared: &Shared) -> Option<Frame> {
        let mut request = None;
        await_predicate(|| {
            request = self.mailbox.take_request(15);
            request.is_some() || shared.fault.load(Ordering::Acquire) != 0
                || shared.cancelled.load(Ordering::Acquire)
        });
        request
    }
    fn request(&mut self) -> Frame {
        let mut request = None;
        await_predicate(|| {
            request = self.mailbox.take_request(15);
            request.is_some()
        });
        let request = request.unwrap();
        assert_eq!(request.kind, ap1_native_client::PROCESS);
        request
    }
    fn reply(&mut self, request: &Frame) {
        self.reply_timed(request, 0);
    }
    fn reply_timed(&mut self, request: &Frame, process_ns: u64) {
        let n = ap1_native_client::get(&request.payload[..4]) as usize;
        if n != 0 {
            let mut bytes = vec![0; n * 4];
            for channel in 0..2 {
                self.samples.read_exact_at(&mut bytes,
                    (ap1_native_client::INPUT + channel * self.stride + 4) as u64).unwrap();
                self.samples.write_all_at(&bytes,
                    (self.output + channel * self.stride + 4) as u64).unwrap();
            }
            bytes.fill(0);
            for channel in 2..self.channels {
                self.samples.write_all_at(&bytes,
                    (self.output + channel * self.stride + 4) as u64).unwrap();
            }
        }
        // Exact Done identity and validated sample ownership. N0 returns one
        // parameter point at offset zero; it never reads/writes sample planes.
        let mut payload = vec![0; if n == 0 { 88 } else { 72 }];
        payload[..4].copy_from_slice(&(n as u32).to_le_bytes());
        payload[4..8].copy_from_slice(&(self.output as u32).to_le_bytes());
        payload[16..32].copy_from_slice(&request.payload[32..48]);
        payload[32..40].copy_from_slice(&process_ns.to_le_bytes());
        if n == 0 {
            payload[60..64].copy_from_slice(&1u32.to_le_bytes());
            payload[76..80].copy_from_slice(&7u32.to_le_bytes());
            payload[80..88].copy_from_slice(&(request.sequence as f64 / 100.).to_le_bytes());
        }
        self.mailbox.respond(Frame { kind: ap1_native_client::DONE,
            session: request.session, sequence: request.sequence, payload }, 15);
        self.wake.write_all(&[ap1_native_client::notification::WAKE]).unwrap();
    }
}
struct Fixture {
    shared: Arc<Shared>,
    callback: Callback,
    worker: Option<thread::JoinHandle<()>>,
    _control_peer: TcpStream,
    directory: PathBuf,
}
impl Fixture {
    fn new(delay: u64) -> (Self, Peer) {
        Self::with_extra(delay, 0)
    }
    fn with_extra(delay: u64, extra: usize) -> (Self, Peer) {
        let directory = std::env::temp_dir().join(format!("completion-contract-{:032x}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("audio");
        let mut mapping = Mapping::with_channels(&path, if extra > 0 { 64 } else { 2 }).unwrap();
        mapping.output_channels = extra + 2;
        let samples = File::options().read(true).write(true).open(&path).unwrap();
        let output = mapping.output;
        let stride = mapping.stride;
        let mailbox = crate::mailbox::Mailbox::create(&directory.join("delivery"), [41; 16]).unwrap();
        let peer_mailbox = mailbox.counterpart();
        let (socket, control_peer) = sockets();
        let (wake, wake_peer) = sockets();
        let identity = Some(state::Identity { class: [1; 16], module: [2; 32] });
        let session = Session {
            gui: None, gui_revision: 0, mapping: Some(mapping), mailbox: Some(mailbox),
            mailbox_enabled: true, notifications: Some(Channel::new(wake).unwrap()),
            configured_mode: 0, capture: None, fault_status: None, notices: (0, 0),
            returned: Default::default(), processing: crate::ProcessingScratch::new(), socket,
            state: ClientState { session: [41; 16], next: 1, slot: Slot::Writable },
            phase: 11, max: 64, minor: 15, epoch: 1, position: 0, witness: None,
            identity, trace: Default::default(), sample_rate: 48000, armed: false, owner: None,
        };
        let mut shared = Shared::new(); shared.identity = identity;
        if extra > 0 { assert!(shared.extra.set(crate::output_pool::Pool::new(extra, 1)).is_ok()); }
        let shared = Arc::new(shared);
        shared.wanted.store(1, Ordering::Release);
        shared.state_capable.store(true, Ordering::Release);
        let service = shared.clone();
        let worker = thread::spawn(move || super::worker(session, service, None, None));
        let mut callback = Callback::new();
        callback.prepare(64, 2 + extra, delay as usize);
        callback.running = true;
        callback.epoch = 1;
        (Self { shared, callback, worker: Some(worker), _control_peer: control_peer, directory },
            Peer { mailbox: peer_mailbox, wake: wake_peer, samples, output, stride, channels: 2 + extra })
    }
    fn call(&mut self, frames: usize, harness_bound: Option<Duration>) -> Result<u64, u32> {
        let now = Instant::now();
        let mut policy = crate::performance::CompletionPolicy::new(0,
            if self.callback.delay == 0 { crate::performance::DeliveryMode::SameCallback }
            else { crate::performance::DeliveryMode::Buffered }, frames, 0, 0, now);
        // Positive control tests FIFO semantics with a deliberately generous
        // finite harness bound. It neither selects nor justifies product policy.
        if let Some(bound) = harness_bound { policy.deadline = now + bound; policy.allowance = bound; }
        let mut item = Item::control(AUDIO, 1);
        item.n = frames as u32;
        item.gain = f64::NAN;
        item.data = [[0.25; CAP], [-0.5; CAP]];
        item.completion = Some(policy);
        self.callback.host_call += 1;
        item.parent = [self.callback.host_call, frames as u64, 0, 0];
        self.callback.returned.window(self.callback.position, frames);
        self.callback.process_outputs_until(&self.shared, item, &mut [[0.; CAP]; 2], &[], 0,
            Some(policy.deadline))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Test endpoint has no supervisor. Cancellation joins the actual native
        // worker before its mappings are removed; this is not product retirement proof.
        self.shared.cancel();
        self.shared.quit.store(true, Ordering::Release);
        self.shared.work.notify();
        if let Some(worker) = self.worker.take() { worker.join().unwrap(); }
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
}

#[test]
fn real_worker_windows_timer_matches_own_ticket_including_zero_frames() {
    let (mut fixture, mut peer) = Fixture::new(0);
    let responder = thread::spawn(move || {
        for (n, ns) in [(64, 3_000_000), (0, 1234), (13, 4567)] {
            let request = peer.request();
            assert_eq!(ap1_native_client::get(&request.payload[..4]), n);
            peer.reply_timed(&request, ns);
        }
        peer
    });
    for (sequence, frames, ns) in [(1, 64, 3_000_000), (2, 0, 1234), (3, 13, 4567)] {
        fixture.callback.windows_timing = WindowsProcessTiming::default();
        assert_eq!(
            fixture.call(frames, None),
            Ok(if frames == 0 { 3 } else { 0 })
        );
        let t = fixture.callback.windows_timing;
        assert_eq!(
            (
                t.epoch,
                t.requests,
                t.process_ns,
                t.first_sequence,
                t.last_sequence,
                t.valid
            ),
            (1, 1, ns, sequence, sequence, 1)
        );
    }
    let _peer = responder.join().unwrap();
}

#[test]
fn real_worker_zero_frame_refusal_discriminates_predecessor_reply_from_publication() {
    for validated in [false, true] {
        let (mut fixture, mut peer) = Fixture::new(256);
        if validated { fixture.shared.publication_hold_ticket.store(1, Ordering::Release); }
        for _ in 0..4 { assert_eq!(fixture.call(64, None), Ok(3)); }
        let predecessor = peer.request();
        assert_eq!((predecessor.sequence, ap1_native_client::get(&predecessor.payload[40..48])), (1, 0));
        if validated {
            peer.reply(&predecessor);
            await_predicate(|| fixture.shared.publication_held_ticket.load(Ordering::Acquire) == 1);
        }
        assert_eq!(fixture.shared.results.published(), 0);
        assert_eq!(fixture.callback.position, 256);
        assert_eq!(fixture.call(0, Some(Duration::from_millis(1))), Err(COMPLETION_EXPIRED));
        assert_eq!(fixture.callback.position, 256, "failed N0 must not advance sample time");
        assert_eq!((fixture.callback.submitted_operation, fixture.callback.completed_operation), (5, 0));
        assert_eq!(fixture.shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
        assert_eq!(fixture.shared.first_requests[0].load(Ordering::Acquire), 5);
        assert_eq!(fixture.shared.first_results[0].load(Ordering::Acquire), 0);
        eprintln!("N0 predecessor discrimination: validated={validated}, refused_ticket=5, predecessor=1, presentation=256, completed=0");
        if !validated { peer.reply(&predecessor); }
        assert_eq!(fixture.call(0, None), Err(2), "late owned results cannot authorize a new success");
        drop(fixture);
    }
}

#[test]
fn real_worker_zero_frame_completes_fifo_debt_without_advancing_position() {
    let (mut fixture, mut peer) = Fixture::new(256);
    for n in [64, 13, 32] { assert_eq!(fixture.call(n, None), Ok(3)); }
    let responder = thread::spawn(move || {
        let mut identity = Vec::new();
        for _ in 0..7 {
            let request = peer.request();
            identity.push((request.sequence, ap1_native_client::get(&request.payload[40..48]),
                ap1_native_client::get(&request.payload[..4])));
            peer.reply(&request);
        }
        (identity, peer)
    });
    for (n, position, ticket) in [(0, 109, 4), (0, 109, 5), (13, 122, 6), (0, 122, 7)] {
        assert!(fixture.call(n, Some(Duration::from_secs(1))).is_ok());
        assert_eq!(fixture.callback.position, position);
        if n == 0 {
            assert_eq!(fixture.callback.completed_operation, ticket);
            let mut returned = crate::process_results::Packet::default();
            fixture.callback.returned.take(&mut returned);
            assert_eq!((returned.events, returned.points), (0, 1));
            assert_eq!((returned.point[0].offset, returned.point[0].id, returned.point[0].value),
                (0, 7, ticket as f64 / 100.));
        }
    }
    let (identity, _peer) = responder.join().unwrap();
    assert_eq!(identity, vec![(1, 0, 64), (2, 64, 13), (3, 77, 32),
        (4, 109, 0), (5, 109, 0), (6, 109, 13), (7, 122, 0)]);
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), 0);
    assert_eq!((fixture.callback.submitted_operation, fixture.callback.completed_operation), (7, 7));
    eprintln!("N0 FIFO positive: 7 real worker replies, short/N0 positions exact, N0 parameter results delivered; 1s harness bound is not product policy");
}

#[test]
fn real_worker_same_callback_main_refusal_preserves_admission_and_reply_ownership() {
    let (mut fixture, mut peer) = Fixture::new(0);
    let observer = fixture.shared.clone();
    let withheld = thread::spawn(move || {
        let mut request = None;
        await_predicate(|| {
            request = peer.mailbox.take_request(15);
            request.is_some() || observer.fault.load(Ordering::Acquire) != 0
        });
        if let Some(request) = &request {
            assert_eq!((request.sequence, ap1_native_client::get(&request.payload[..4])), (1, 64));
            await_predicate(|| observer.fault.load(Ordering::Acquire) != 0);
            peer.reply(request);
        }
        (request.is_some(), peer)
    });
    let result = fixture.call(64, Some(Duration::from_nanos(1_333_333)));
    assert!(matches!(result, Err(COMPLETION_EXPIRED | 2)),
        "either callback or worker may first observe the same finite expiry: {result:?}");
    let (observed, _peer) = withheld.join().unwrap();
    assert_eq!((fixture.callback.position, fixture.callback.completed_operation), (0, 0));
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
    assert_eq!(fixture.shared.first_results[0].load(Ordering::Acquire), 0);
    eprintln!("D0/N64 finite-expiry control: mapped_request_observed={observed}, exact completion unsatisfied under injected 1.333333ms harness bound; if not observed admission/render stage is unobserved, not proven absent");
}

#[test]
fn approved_completion_accepts_healthy_zero_frame_after_fifo_debt() {
    for validated in [false, true] {
        let (mut fixture, mut peer) = Fixture::new(256);
        if validated { fixture.shared.publication_hold_ticket.store(1, Ordering::Release); }
        for _ in 0..4 { assert_eq!(fixture.call(64, None), Ok(3)); }
        let predecessor = peer.request();
        if validated {
            peer.reply(&predecessor);
            await_predicate(|| fixture.shared.publication_held_ticket.load(Ordering::Acquire) == 1);
        }
        let shared = fixture.shared.clone();
        let responder = thread::spawn(move || {
            thread::sleep(Duration::from_millis(5));
            if validated { shared.publication_hold_ticket.store(0, Ordering::Release); }
            else { peer.reply(&predecessor); }
            let mut count = 1;
            while count < 5 {
                let Some(request) = peer.request_or_fault(&shared) else { break; };
                peer.reply(&request); count += 1;
            }
            (count, peer)
        });
        let result = fixture.call(0, None);
        let (count, _peer) = responder.join().unwrap();
        assert_eq!(result, Ok(3), "healthy FIFO debt completes, validated={validated}");
        assert_eq!(count, 5);
        assert_eq!((fixture.callback.position, fixture.callback.completed_operation), (256, 5));
        let mut returned = crate::process_results::Packet::default();
        fixture.callback.returned.take(&mut returned);
        assert_eq!((returned.points, returned.point[0].value), (1, 0.05));
        assert_eq!(fixture.shared.fault.load(Ordering::Acquire), 0);
    }
}

fn healthy_same_callback(frames: usize) {
    let (mut fixture, mut peer) = Fixture::new(0);
    let shared = fixture.shared.clone();
    let responder = thread::spawn(move || {
        let request = peer.request_or_fault(&shared);
        if let Some(request) = request {
            thread::sleep(Duration::from_millis(5));
            peer.reply(&request);
        }
        peer
    });
    let result = fixture.call(frames, None);
    let _peer = responder.join().unwrap();
    assert_eq!(result, Ok(0), "healthy ordered completion is independent of N/Fs");
    assert_eq!((fixture.callback.position, fixture.callback.completed_operation), (frames as u64, 1));
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), 0);
}
#[test]
fn approved_completion_accepts_healthy_short_tail() { healthy_same_callback(13); }
#[test]
fn approved_completion_accepts_healthy_main_n64() { healthy_same_callback(64); }

#[test]
fn already_expired_queued_audio_is_not_published_to_the_peer() {
    let (mut fixture, mut peer) = Fixture::new(256);
    let now = Instant::now();
    let mut item = Item::control(AUDIO, 1); item.n = 64; item.ticket = 1;
    let mut policy = crate::performance::CompletionPolicy::new(0,
        crate::performance::DeliveryMode::Buffered, 64, 0, 0, now);
    policy.deadline = now - Duration::from_nanos(1); item.completion = Some(policy);
    assert!(fixture.shared.requests.push(item)); fixture.shared.work.notify();
    await_predicate(|| fixture.shared.fault.load(Ordering::Acquire) != 0);
    fixture.worker.take().unwrap().join().unwrap();
    // Observe only after worker termination; no publication race remains.
    assert!(peer.mailbox.take_request(15).is_none());
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
    assert_eq!(fixture.shared.first_requests[0].load(Ordering::Acquire), 1);
    assert_eq!(fixture.shared.results.published(), 0);
}

#[test]
fn original_predecessor_expiry_is_not_renewed_by_zero_frame_flush() {
    let (mut fixture, mut peer) = Fixture::new(256);
    assert_eq!(fixture.call(64, Some(Duration::from_millis(15))), Ok(3));
    let request = peer.request();
    let shared = fixture.shared.clone();
    let held = thread::spawn(move || {
        await_predicate(|| shared.fault.load(Ordering::Acquire) != 0);
        peer.reply(&request); // Late result retains ownership, never authorizes success.
        peer
    });
    assert_eq!(fixture.call(0, None), Err(2));
    let _peer = held.join().unwrap();
    assert_eq!(fixture.shared.results.published(), 0);
    assert_eq!(fixture.callback.completed_operation, 0);
    assert_eq!(fixture.shared.first_position.load(Ordering::Acquire), 0,
        "the earlier predecessor's position remains primary, rather than N0 at 64");
    assert_eq!(fixture.shared.first_requests[0].load(Ordering::Acquire), 2);
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
}

#[test]
fn cancel_wakes_exact_completion_without_releasing_reply_custody() {
    let (mut fixture, mut peer) = Fixture::new(0);
    let shared = fixture.shared.clone();
    let held = thread::spawn(move || {
        let request = peer.request();
        shared.cancel();
        (request, peer)
    });
    assert_eq!(fixture.call(64, None), Err(COMPLETION_CANCELLED));
    let (request, mut peer) = held.join().unwrap();
    peer.reply(&request);
    fixture.worker.take().unwrap().join().unwrap();
    assert_eq!(fixture.shared.results.published(), 0);
    assert_eq!(fixture.callback.completed_operation, 0);
}

#[test]
fn buffered_due_audio_is_complete_while_admitted_capture_waits() {
    let (mut fixture, mut peer) = Fixture::new(256);
    *fixture.shared.control.lock().unwrap() = Some(Control {
        barrier: 0, op: 16, bytes: vec![], result: None,
    });
    fixture.shared.pending_control.store(true, Ordering::Release);
    fixture.shared.work.notify();
    let mut control_socket = fixture._control_peer.try_clone().unwrap();
    let mut handoff = None;
    await_predicate(|| { handoff = peer.mailbox.take_request(15); handoff.is_some() });
    assert_eq!(handoff.unwrap().kind, 0, "acknowledge the real admitted control handoff");
    let capture = ap1_native_client::endpoint::receive_version(&mut control_socket, 3, 15).unwrap();
    assert_eq!((capture.kind, capture.sequence), (16, 1));
    for _ in 0..4 { assert_eq!(fixture.call(64, None), Ok(3)); }
    let first = peer.request(); assert_eq!(first.sequence, 2);
    let shared = fixture.shared.clone();
    let responder = thread::spawn(move || {
        thread::sleep(Duration::from_millis(5));
        peer.reply(&first);
        for _ in 0..4 { let request = peer.request(); peer.reply(&request); }
        // Capture has remained uncompleted throughout the due-audio wait. It
        // has independent identity and does not license successful missing spans.
        let mut payload = vec![0; 20]; payload[..4].copy_from_slice(&4u32.to_le_bytes());
        let response = Frame { kind: 17, session: capture.session,
            sequence: capture.sequence, payload };
        control_socket.write_all(&response.encode_version(15).unwrap()).unwrap();
        shared.work.notify();
        peer
    });
    assert_eq!(fixture.call(64, None), Ok(0));
    assert_eq!((fixture.callback.delivery.missing_frames, fixture.callback.delivery.delivered_frames), (0, 64));
    let _peer = responder.join().unwrap();
    await_predicate(|| !fixture.shared.pending_control.load(Ordering::Acquire));
    assert!(fixture.shared.control.lock().unwrap().as_ref().unwrap().result.as_ref().unwrap().is_ok());
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), 0);
}


#[test]
fn dead_notification_peer_fails_exact_completion_without_audio_success() {
    let (mut fixture, mut peer) = Fixture::new(0);
    let held = thread::spawn(move || { let request = peer.request(); drop(peer); request });
    assert_eq!(fixture.call(64, None), Err(2));
    let request = held.join().unwrap(); assert_eq!(request.sequence, 1);
    fixture.worker.take().unwrap().join().unwrap();
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), WORKER);
    assert_eq!(fixture.shared.results.published(), 0);
    assert_eq!(fixture.callback.completed_operation, 0);
}

#[test]
fn final_publication_expiry_or_cancel_releases_the_owned_extra_slot() {
    for cancel in [false, true] {
        let (mut fixture, mut peer) = Fixture::with_extra(256, 2);
        fixture.shared.final_publication_hold_ticket.store(1, Ordering::Release);
        // Finite harness bound exposes original producer expiry while the N0
        // observer retains the actual5s policy. It does not change production.
        assert_eq!(fixture.call(64, Some(Duration::from_secs(1))), Ok(3));
        let request = peer.request(); peer.reply(&request);
        await_predicate(|| fixture.shared.final_publication_held_ticket.load(Ordering::Acquire) == 1
            || fixture.shared.fault.load(Ordering::Acquire) != 0);
        assert_eq!(fixture.shared.final_publication_held_ticket.load(Ordering::Acquire), 1,
            "held publication not reached: {}", fixture.shared.detail.lock().unwrap());
        let slot_owner = fixture.shared.clone();
        let pool = slot_owner.extra.get().unwrap();
        let samples = [[0.; CAP]; 2];
        assert!(pool.publish_available(&samples, 64).is_none(), "worker owns the sole extra slot");
        let shared = fixture.shared.clone();
        let release = thread::spawn(move || {
            if cancel { shared.cancel(); }
            else {
                thread::sleep(Duration::from_millis(1010));
                shared.final_publication_hold_ticket.store(0, Ordering::Release);
            }
        });
        let result = fixture.call(0, None);
        assert_eq!(result, Err(if cancel { COMPLETION_CANCELLED } else { 2 }));
        release.join().unwrap(); fixture.worker.take().unwrap().join().unwrap();
        assert_eq!(fixture.shared.results.published(), 0);
        assert_eq!(fixture.callback.completed_operation, 0);
        if !cancel {
            assert_eq!(fixture.shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
            assert_eq!(fixture.shared.first_position.load(Ordering::Acquire), 0);
        }
        let slot = pool.publish_available(&samples, 64).expect("refusal retired the acquired slot");
        pool.release(slot);
    }
}

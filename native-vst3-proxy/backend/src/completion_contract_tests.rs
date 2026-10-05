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
}
impl Peer {
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
        let n = ap1_native_client::get(&request.payload[..4]) as usize;
        if n != 0 {
            let mut bytes = vec![0; n * 4];
            for channel in 0..2 {
                self.samples.read_exact_at(&mut bytes,
                    (ap1_native_client::INPUT + channel * self.stride + 4) as u64).unwrap();
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
        let directory = std::env::temp_dir().join(format!("completion-contract-{:032x}",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("audio");
        let mapping = Mapping::new(&path).unwrap();
        let samples = File::options().read(true).write(true).open(&path).unwrap();
        let output = mapping.output;
        let stride = mapping.stride;
        let mailbox = crate::mailbox::Mailbox::create(&directory.join("delivery"), [41; 16]).unwrap();
        let peer_mailbox = mailbox.counterpart();
        let (socket, control_peer) = sockets();
        let (wake, wake_peer) = sockets();
        let session = Session {
            gui: None, gui_revision: 0, mapping: Some(mapping), mailbox: Some(mailbox),
            mailbox_enabled: true, notifications: Some(Channel::new(wake).unwrap()),
            configured_mode: 0, capture: None, fault_status: None, notices: (0, 0),
            returned: Default::default(), processing: crate::ProcessingScratch::new(), socket,
            state: ClientState { session: [41; 16], next: 1, slot: Slot::Writable },
            phase: 11, max: 64, minor: 15, epoch: 1, position: 0, witness: None,
            identity: None, trace: Default::default(), sample_rate: 48000, armed: false, owner: None,
        };
        let shared = Arc::new(Shared::new());
        shared.wanted.store(1, Ordering::Release);
        shared.state_capable.store(true, Ordering::Release);
        let service = shared.clone();
        let worker = thread::spawn(move || super::worker(session, service, None, None));
        let mut callback = Callback::new();
        callback.prepare(64, 2, delay as usize);
        callback.running = true;
        callback.epoch = 1;
        (Self { shared, callback, worker: Some(worker), _control_peer: control_peer, directory },
            Peer { mailbox: peer_mailbox, wake: wake_peer, samples, output, stride })
    }
    fn call(&mut self, frames: usize, harness_bound: Option<Duration>) -> Result<u64, u32> {
        let now = Instant::now();
        let mut policy = crate::performance::CompletionPolicy::new(0,
            if self.callback.delay == 0 { crate::performance::DeliveryMode::SameCallback }
            else { crate::performance::DeliveryMode::Buffered }, frames, 48000., 0, 0, now);
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
        self.worker.take().unwrap().join().unwrap();
        std::fs::remove_dir_all(&self.directory).unwrap();
    }
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
        assert_eq!(fixture.call(0, None), Err(COMPLETION_EXPIRED));
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
    assert_eq!(fixture.call(64, None), Err(COMPLETION_EXPIRED));
    let (observed, _peer) = withheld.join().unwrap();
    assert_eq!((fixture.callback.position, fixture.callback.completed_operation), (0, 0));
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), COMPLETION_DEADLINE);
    assert_eq!(fixture.shared.first_results[0].load(Ordering::Acquire), 0);
    eprintln!("D0/N64 refusal: mapped_request_observed={observed}, exact completion unsatisfied under current 1.333333ms policy; if not observed admission/render stage is unobserved, not proven absent");
}

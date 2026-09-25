//! A source-owned delayed peer exercises the real queued worker and mapped audio.
use super::*;
use ap1_native_client::{
    endpoint::{receive_version, send_version},
    events::{self, NOTE_OFF, NOTE_ON, PARAMETER},
    mapping::Mapping,
    ClientState, Frame, Slot, OUTPUT, STRIDE,
};
use std::{os::unix::fs::FileExt, sync::atomic::AtomicBool};

#[derive(Default)]
struct Gate {
    arm: AtomicBool,
    stalled: AtomicBool,
    release: AtomicBool,
    internal_preset_change: AtomicBool,
}

#[derive(Default, Debug)]
struct PeerResult {
    old_note: bool,
    held_note: bool,
    parameter: f64,
    starts: u32,
    stops: u32,
    new_epoch_audio: u32,
    third_epoch_audio: u32,
    old_note_offs_new_epoch: u32,
    held_note_offs_third_epoch: u32,
    parameter_updates_new_epoch: u32,
    parameter_09_new_epoch: u32,
}

struct Fixture {
    shared: Arc<Shared>,
    callback: Callback,
    gate: Arc<Gate>,
    peer: JoinHandle<PeerResult>,
    transport: JoinHandle<()>,
    path: std::path::PathBuf,
}

fn until(mut condition: impl FnMut() -> bool) {
    let end = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < end, "source-owned peer did not advance");
        thread::sleep(Duration::from_millis(1));
    }
}

fn note(kind: u32, id: u32, pitch: i16) -> Event {
    Event {
        kind,
        id,
        channel: 0,
        pitch,
        value: if kind == NOTE_ON { 0.8 } else { 0. },
        ..Event::default()
    }
}
fn parameter(value: f64) -> Event {
    Event {
        kind: PARAMETER,
        id: 77,
        value,
        ..Event::default()
    }
}

impl Fixture {
    fn new(deadline_ms: u32) -> Self {
        Self::with_policy(deadline_ms, 1, 0)
    }
    fn with_policy(deadline_ms: u32, max_recoveries: u32, rearm_healthy_frames: u32) -> Self {
        let path = std::env::temp_dir().join(format!(
            "br1-source-owned-{}.audio",
            u128::from_le_bytes(ap1_native_client::mapping::random().unwrap())
        ));
        let mapping = Mapping::new(&path).unwrap();
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let socket = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        socket.set_nodelay(true).unwrap();
        let (mut remote, _) = listener.accept().unwrap();
        remote.set_nodelay(true).unwrap();
        let gate = Arc::new(Gate::default());
        let peer_gate = gate.clone();
        let peer = thread::spawn(move || {
            let mut result = PeerResult {
                parameter: 0.5,
                ..PeerResult::default()
            };
            while let Ok(f) = receive_version(&mut remote, 5, 5) {
                let payload = if f.kind == AUDIO as u16 {
                    let n = ap1_native_client::get(&f.payload[..4]) as usize;
                    if peer_gate.arm.swap(false, Ordering::AcqRel) {
                        if peer_gate.internal_preset_change.swap(false, Ordering::AcqRel) {
                            // A source-owned stand-in for a preset's internal
                            // parameter change after the last host automation.
                            result.parameter = 0.7;
                        }
                        peer_gate.stalled.store(true, Ordering::Release);
                        until(|| peer_gate.release.load(Ordering::Acquire));
                    }
                    let epoch = ap1_native_client::get(&f.payload[32..40]);
                    let new_epoch = epoch == 2;
                    for event in events::decode(&f.payload[48..], n).unwrap() {
                        if new_epoch && (event.kind, event.id) == (NOTE_OFF, 1) {
                            result.old_note_offs_new_epoch += 1;
                        }
                        if new_epoch && (event.kind, event.id) == (PARAMETER, 77) {
                            result.parameter_updates_new_epoch += 1;
                            if event.value == 0.9 { result.parameter_09_new_epoch += 1; }
                        }
                        if epoch == 3 && (event.kind, event.id) == (NOTE_OFF, 2) {
                            result.held_note_offs_third_epoch += 1;
                        }
                        match (event.kind, event.id) {
                            (NOTE_ON, 1) => result.old_note = true,
                            (NOTE_OFF, 1) => result.old_note = false,
                            (NOTE_ON, 2) => result.held_note = true,
                            (NOTE_OFF, 2) => result.held_note = false,
                            (PARAMETER, 77) => result.parameter = event.value,
                            _ => panic!("unexpected source-owned event"),
                        }
                    }
                    let sample = ((result.old_note as u32) as f64 * 0.4
                        + (result.held_note as u32) as f64 * 0.8)
                        * result.parameter;
                    let plane = (sample as f32).to_le_bytes().repeat(n);
                    for ch in 0..2 {
                        file.write_all_at(&plane, (OUTPUT + ch * STRIDE + 4) as u64)
                            .unwrap();
                    }
                    if new_epoch {
                        result.new_epoch_audio += 1;
                    } else if epoch == 3 {
                        result.third_epoch_audio += 1;
                    }
                    [
                        (n as u32).to_le_bytes().as_slice(),
                        (OUTPUT as u32).to_le_bytes().as_slice(),
                        0u64.to_le_bytes().as_slice(),
                        &f.payload[32..48],
                    ]
                    .concat()
                } else if matches!(f.kind, 10 | 12) {
                    if f.kind == 10 {
                        result.starts += 1;
                    } else {
                        result.stops += 1;
                    }
                    f.payload.clone()
                } else {
                    vec![]
                };
                if send_version(
                    &mut remote,
                    &Frame {
                        kind: f.kind + 1,
                        session: f.session,
                        sequence: f.sequence,
                        payload,
                    },
                    5,
                    5,
                )
                .is_err()
                {
                    break;
                }
                if f.kind == CLOSE as u16 {
                    break;
                }
            }
            result
        });
        let session = Session {
            gui: None,
            gui_revision: 0,
            mailbox: None,
            mailbox_enabled: false,
            capture: None,
            fault_status: None,
            notices: (0, 0),
            returned: crate::process_results::Packet::default(),
            mapping: Some(mapping),
            socket,
            state: ClientState {
                session: [1; 16],
                next: 1,
                slot: Slot::Writable,
            },
            phase: 9,
            max: CAP,
            minor: 5,
            identity: None,
            epoch: 0,
            position: 0,
            witness: None,
            trace: Default::default(),
            sample_rate: 48_000,
            armed: false,
            owner: None,
        };
        let shared = Arc::new(Shared::new());
        let service = shared.clone();
        let transport = thread::spawn(move || worker(session, service, None));
        let mut callback = Callback::new();
        callback.delay = 2048;
        callback.live = Some(Box::new(LiveCallback {
            policy: crate::live_recovery::Policy {
                horizon_frames: 256,
                worker_deadline_ms: deadline_ms,
                max_recoveries,
                rearm_healthy_frames,
            },
            ledger: crate::live_recovery::Ledger::default(),
            phase: 0,
            requested_at: None,
            recoveries: 0,
            prime_until: 0,
            healthy_frames: 0,
            rearmed: false,
        }));
        assert_eq!(callback.transition(&shared, START), 0);
        until(|| shared.ack.load(Ordering::Acquire) == started_ack(1).unwrap());
        Self {
            shared,
            callback,
            gate,
            peer,
            transport,
            path,
        }
    }
    fn block(&mut self, events: &[Event]) -> Result<f32, u32> {
        let mut item = Item::control(AUDIO, 0);
        item.n = 256;
        item.gain = f64::NAN;
        item.event_count = events.len() as u32;
        item.events[..events.len()].copy_from_slice(events);
        let mut output = [[0.; CAP]; 2];
        self.callback.process(&self.shared, item, &mut output)?;
        // One 256-frame callback at 48 kHz. The deliberate peer delay thus
        // crosses the configured frame horizon in physical time as well.
        thread::sleep(Duration::from_micros(5_333));
        assert!(output[0].iter().all(|sample| sample.is_finite()));
        assert_eq!(output[0], output[1]);
        Ok(output[0][0])
    }
    fn initial_audio(&mut self) {
        self.block(&[note(NOTE_ON, 1, 60)]).unwrap();
        until(|| self.shared.processed.load(Ordering::Acquire) >= 1);
        let mut sample = 0.;
        for expected in 2..=12 {
            sample = self.block(&[]).unwrap();
            until(|| self.shared.processed.load(Ordering::Acquire) >= expected);
        }
        assert_eq!(sample, 0.2);
    }
    fn completed_parameter(&mut self, value: f64) {
        self.block(&[parameter(value)]).unwrap();
        let published = self.shared.requests.published();
        until(|| self.shared.requests.consumed() >= published);
        let mut sample = 0.;
        for _ in 0..12 {
            sample = self.block(&[]).unwrap();
            if (sample - (0.4 * value) as f32).abs() < 0.00001 { break; }
        }
        assert!((sample - (0.4 * value) as f32).abs() < 0.00001);
    }
    fn arm_stall(&mut self) {
        self.gate.release.store(false, Ordering::Release);
        self.gate.stalled.store(false, Ordering::Release);
        self.gate.arm.store(true, Ordering::Release);
        self.block(&[]).unwrap();
        until(|| self.gate.stalled.load(Ordering::Acquire));
    }
    fn request_recovery(&mut self) {
        self.request_recovery_with(&[note(NOTE_OFF, 1, 60), note(NOTE_ON, 2, 64), parameter(0.4)]);
        self.block(&[parameter(0.9)]).unwrap();
    }
    fn request_recovery_with(&mut self, closure_events: &[Event]) {
        for _ in 0..16 {
            self.block(&[]).unwrap();
            if self.shared.live_stage.load(Ordering::Acquire) == LIVE_REQUESTED {
                break;
            }
        }
        assert_eq!(
            self.shared.live_stage.load(Ordering::Acquire),
            LIVE_REQUESTED
        );
        assert!(self.shared.requests.high_water() < 64);
        let published = self.shared.requests.published();
        self.block(closure_events).unwrap();
        assert_eq!(
            self.shared.requests.published(),
            published,
            "admission stayed closed"
        );
        assert_eq!(self.shared.fault.load(Ordering::Acquire), 0);
    }
    fn release_and_resume(&mut self) {
        self.release_and_resume_with(0.72);
    }
    fn release_and_resume_with(&mut self, expected_sample: f32) {
        self.release_and_resume_epoch(2, expected_sample);
    }
    fn release_and_resume_epoch(&mut self, expected_epoch: u64, expected_sample: f32) {
        let old_published = self.shared.results.published();
        self.gate.release.store(true, Ordering::Release);
        until(|| self.shared.live_stage.load(Ordering::Acquire) == LIVE_ACKNOWLEDGED);
        assert_eq!(
            self.shared.results.published(),
            old_published,
            "stale completion published"
        );
        assert_eq!(self.shared.live_ack_epoch.load(Ordering::Acquire), expected_epoch);
        assert_eq!(self.shared.live_recoveries.load(Ordering::Acquire), expected_epoch - 1);
        assert!(self.shared.live_discard_requests.load(Ordering::Acquire) > 0);
        assert!(!state_control_stage_ready(&self.shared),
            "state control must refuse after worker acknowledgement");
        let mut resumed = false;
        for _ in 0..16 {
            let previous = self.shared.processed.load(Ordering::Acquire);
            let sample = self.block(&[]).unwrap();
            until(|| self.shared.processed.load(Ordering::Acquire) > previous);
            if sample > 0. {
                assert!((sample - expected_sample).abs() < 0.00001);
                assert!(state_control_stage_ready(&self.shared),
                    "state control becomes available on resumed audio");
                resumed = true;
                break;
            }
            assert!(!state_control_stage_ready(&self.shared),
                "state control must refuse throughout reconciliation and re-prime");
        }
        assert!(resumed, "current held note did not resume");
        assert_eq!(self.callback.epoch, expected_epoch);
        assert_eq!(self.callback.live.as_ref().unwrap().phase, 0);
        let requested = self.shared.live_request_ns.load(Ordering::Acquire);
        let returned = self.shared.live_worker_return_ns.load(Ordering::Acquire);
        let acknowledged = self.shared.live_ack_ns.load(Ordering::Acquire);
        let resumed_at = self.shared.live_resume_ns.load(Ordering::Acquire);
        assert!(requested > 0 && returned >= requested);
        assert!(acknowledged >= returned && resumed_at >= acknowledged);
        #[cfg(target_os = "linux")]
        assert!(self.shared.live_resume_ns.load(Ordering::Acquire) > 0);
    }
    fn finish(mut self, clean: bool) -> PeerResult {
        self.gate.release.store(true, Ordering::Release);
        if clean {
            assert_eq!(self.callback.transition(&self.shared, STOP), 0);
            let epoch = self.callback.epoch;
            until(|| self.shared.ack.load(Ordering::Acquire) == (epoch << 8) | 13);
            assert!(self.shared.requests.push(Item::control(DEACTIVATE, 0)));
            until(|| self.shared.ack.load(Ordering::Acquire) == 15);
            assert!(self.shared.requests.push(Item::control(CLOSE, 0)));
        }
        self.transport.join().unwrap();
        let result = self.peer.join().unwrap();
        std::fs::remove_file(self.path).unwrap();
        result
    }
}

#[test]
fn live_finite_stall_reconciles_same_instance_and_retires() {
    let mut fixture = Fixture::new(1500);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.request_recovery();
    fixture.release_and_resume();
    println!("BR1_SOURCE_OWNED_FINITE old_epoch={} new_epoch={} request_high={} request_discarded={} result_discarded={} request_to_worker_return_ns={} worker_return_to_audio_ns={} recovery_count={}",
        fixture.shared.live_requested_epoch.load(Ordering::Acquire), fixture.callback.epoch,
        fixture.shared.requests.high_water(),
        fixture.shared.live_discard_requests.load(Ordering::Acquire),
        fixture.shared.live_discard_results.load(Ordering::Acquire),
        fixture.shared.live_worker_return_ns.load(Ordering::Acquire).saturating_sub(
            fixture.shared.live_request_ns.load(Ordering::Acquire)),
        fixture.shared.live_resume_ns.load(Ordering::Acquire).saturating_sub(
            fixture.shared.live_worker_return_ns.load(Ordering::Acquire)),
        fixture.shared.live_recoveries.load(Ordering::Acquire));
    let result = fixture.finish(true);
    assert_eq!((result.starts, result.stops), (2, 2));
    assert!(!result.old_note && result.held_note);
    assert_eq!(result.parameter, 0.9);
    assert!(result.new_epoch_audio > 0);
}

#[test]
fn queued_note_off_discard_still_cleans_confirmed_old_note() {
    let mut fixture = Fixture::new(1500);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.block(&[note(NOTE_OFF, 1, 60)]).unwrap();
    fixture.request_recovery_with(&[note(NOTE_ON, 2, 64), parameter(0.9)]);
    fixture.release_and_resume();
    let result = fixture.finish(true);
    assert_eq!(result.old_note_offs_new_epoch, 1);
    assert!(!result.old_note && result.held_note);
    assert_eq!(result.parameter, 0.9);
}

#[test]
fn queued_parameter_discard_replays_latest_value() {
    let mut fixture = Fixture::new(1500);
    fixture.initial_audio();
    fixture.completed_parameter(0.2);
    fixture.arm_stall();
    fixture.block(&[parameter(0.9)]).unwrap();
    fixture.request_recovery_with(&[note(NOTE_OFF, 1, 60), note(NOTE_ON, 2, 64)]);
    fixture.release_and_resume();
    let result = fixture.finish(true);
    assert_eq!(result.parameter_09_new_epoch, 1);
    assert_eq!(result.parameter_updates_new_epoch, 1);
    assert_eq!(result.parameter, 0.9);
}

#[test]
fn completed_historical_parameter_does_not_overwrite_internal_preset_change() {
    let mut fixture = Fixture::new(1500);
    fixture.initial_audio();
    fixture.completed_parameter(0.2);
    fixture.gate.internal_preset_change.store(true, Ordering::Release);
    fixture.arm_stall();
    fixture.request_recovery_with(&[note(NOTE_OFF, 1, 60), note(NOTE_ON, 2, 64)]);
    fixture.release_and_resume_with(0.56);
    let result = fixture.finish(true);
    assert_eq!(result.parameter_updates_new_epoch, 0);
    assert_eq!(result.parameter, 0.7);
}

#[test]
fn live_worker_deadline_is_not_request_overflow() {
    let mut fixture = Fixture::new(100);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.request_recovery();
    let published = fixture.shared.requests.published();
    thread::sleep(Duration::from_millis(110));
    assert_eq!(fixture.block(&[]), Err(2));
    assert_eq!(
        fixture.shared.fault.load(Ordering::Acquire),
        RECOVERY_TIMEOUT
    );
    assert_eq!(fixture.shared.requests.published(), published);
    assert!(fixture.shared.requests.high_water() < 64);
    assert_eq!(fixture.shared.live_recoveries.load(Ordering::Acquire), 0);
    let result = fixture.finish(false);
    assert_eq!((result.starts, result.stops), (1, 0));
}

#[test]
fn live_second_incident_is_explicitly_exhausted() {
    let mut fixture = Fixture::new(1500);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.request_recovery();
    fixture.release_and_resume();
    fixture.arm_stall();
    for _ in 0..16 {
        if fixture.block(&[]) == Err(2) {
            break;
        }
    }
    assert_eq!(
        fixture.shared.fault.load(Ordering::Acquire),
        RECOVERY_EXHAUSTED
    );
    assert_eq!(fixture.shared.live_recoveries.load(Ordering::Acquire), 1);
    assert!(fixture.shared.requests.high_water() < 64);
    let result = fixture.finish(false);
    assert_eq!((result.starts, result.stops), (2, 1));
}

#[test]
fn live_second_incident_before_healthy_rearm_is_exhausted() {
    let mut fixture = Fixture::with_policy(1500, 2, 48_000);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.request_recovery();
    fixture.release_and_resume();
    assert!(!fixture.callback.live.as_ref().unwrap().rearmed);
    fixture.arm_stall();
    for _ in 0..16 {
        if fixture.block(&[]) == Err(2) { break; }
    }
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), RECOVERY_EXHAUSTED);
    assert_eq!(fixture.shared.live_recoveries.load(Ordering::Acquire), 1);
    assert!(fixture.shared.requests.high_water() < 64);
    let result = fixture.finish(false);
    assert_eq!((result.starts, result.stops), (2, 1));
}

fn rearm_after_healthy_delivery(fixture: &mut Fixture) {
    for _ in 0..512 {
        if fixture.callback.live.as_ref().unwrap().rearmed { break; }
        fixture.block(&[]).unwrap();
    }
    let live = fixture.callback.live.as_ref().unwrap();
    assert!(live.rearmed, "one second of timely delivery did not rearm recovery");
    assert!(live.healthy_frames >= 48_000);
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), 0);
}

#[test]
fn live_separated_second_stall_reconciles_and_retires() {
    let mut fixture = Fixture::with_policy(1500, 2, 48_000);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.request_recovery();
    fixture.release_and_resume();
    rearm_after_healthy_delivery(&mut fixture);
    fixture.arm_stall();
    fixture.request_recovery_with(&[
        note(NOTE_OFF, 2, 64), note(NOTE_ON, 1, 60), parameter(0.6),
    ]);
    fixture.release_and_resume_epoch(3, 0.24);
    assert_eq!(fixture.shared.live_recoveries.load(Ordering::Acquire), 2);
    assert!(fixture.shared.requests.high_water() < 64);
    println!("REARM_SOURCE_OWNED second_epoch=3 recoveries={} request_high={} request_discarded={} result_discarded={} second_request_to_worker_return_ns={} second_worker_return_to_audio_ns={}",
        fixture.shared.live_recoveries.load(Ordering::Acquire),
        fixture.shared.requests.high_water(),
        fixture.shared.live_discard_requests.load(Ordering::Acquire),
        fixture.shared.live_discard_results.load(Ordering::Acquire),
        fixture.shared.live_worker_return_ns.load(Ordering::Acquire).saturating_sub(
            fixture.shared.live_request_ns.load(Ordering::Acquire)),
        fixture.shared.live_resume_ns.load(Ordering::Acquire).saturating_sub(
            fixture.shared.live_worker_return_ns.load(Ordering::Acquire)));
    let result = fixture.finish(true);
    assert_eq!((result.starts, result.stops), (3, 3));
    assert_eq!(result.held_note_offs_third_epoch, 1);
    assert!(result.old_note && !result.held_note);
    assert_eq!(result.parameter, 0.6);
    assert!(result.third_epoch_audio > 0);
}

#[test]
fn live_third_incident_stays_terminal_after_two_recoveries() {
    let mut fixture = Fixture::with_policy(1500, 2, 48_000);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.request_recovery();
    fixture.release_and_resume();
    rearm_after_healthy_delivery(&mut fixture);
    fixture.arm_stall();
    fixture.request_recovery_with(&[]);
    fixture.release_and_resume_epoch(3, 0.72);
    fixture.arm_stall();
    for _ in 0..16 {
        if fixture.block(&[]) == Err(2) { break; }
    }
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), RECOVERY_EXHAUSTED);
    assert_eq!(fixture.shared.live_recoveries.load(Ordering::Acquire), 2);
    assert!(fixture.shared.requests.high_water() < 64);
    let result = fixture.finish(false);
    assert_eq!((result.starts, result.stops), (3, 2));
}

#[test]
fn live_second_attempt_retains_its_own_worker_deadline() {
    let mut fixture = Fixture::with_policy(100, 2, 48_000);
    fixture.initial_audio();
    fixture.arm_stall();
    fixture.request_recovery();
    fixture.release_and_resume();
    rearm_after_healthy_delivery(&mut fixture);
    fixture.arm_stall();
    fixture.request_recovery_with(&[]);
    let published = fixture.shared.requests.published();
    thread::sleep(Duration::from_millis(110));
    assert_eq!(fixture.block(&[]), Err(2));
    assert_eq!(fixture.shared.fault.load(Ordering::Acquire), RECOVERY_TIMEOUT);
    assert_eq!(fixture.shared.live_recoveries.load(Ordering::Acquire), 1);
    assert_eq!(fixture.shared.requests.published(), published);
    assert!(fixture.shared.requests.high_water() < 64);
    let result = fixture.finish(false);
    assert_eq!((result.starts, result.stops), (2, 1));
}

#[test]
fn live_capacity_refuses_an_unrepresentable_note_without_truncation() {
    let shared = Shared::new();
    let mut callback = Callback::new();
    callback.delay = 512;
    callback.live = Some(Box::new(LiveCallback {
        policy: crate::live_recovery::Policy {
            horizon_frames: 256,
            worker_deadline_ms: 1000,
            max_recoveries: 1,
            rearm_healthy_frames: 0,
        },
        ledger: crate::live_recovery::Ledger::default(),
        phase: 0,
        requested_at: None,
        recoveries: 0,
        prime_until: 0,
        healthy_frames: 0,
        rearmed: false,
    }));
    assert_eq!(callback.transition(&shared, START), 0);
    assert_eq!(shared.requests.pop().unwrap().kind, START);
    let mut first = Item::control(AUDIO, 0);
    first.n = 256;
    first.gain = f64::NAN;
    first.event_count = 256;
    for (id, event) in first.events.iter_mut().enumerate() {
        *event = note(NOTE_ON, id as u32, 60);
    }
    callback
        .process(&shared, first, &mut [[0.; CAP]; 2])
        .unwrap();
    let mut extra = Item::control(AUDIO, 0);
    extra.n = 256;
    extra.gain = f64::NAN;
    extra.event_count = 1;
    extra.events[0] = note(NOTE_ON, 256, 61);
    assert_eq!(
        callback.process(&shared, extra, &mut [[0.; CAP]; 2]),
        Err(2)
    );
    assert_eq!(shared.fault.load(Ordering::Acquire), RECOVERY_CAPACITY);
    assert_eq!(shared.requests.published(), 2); // START plus first audio only
}

#[test]
fn old_result_is_discarded_before_new_epoch_audio_and_returned_state() {
    let shared = Shared::new();
    let mut callback = Callback::new();
    callback.delay = 512;
    callback.live = Some(Box::new(LiveCallback {
        policy: crate::live_recovery::Policy {
            horizon_frames: 256,
            worker_deadline_ms: 1000,
            max_recoveries: 1,
            rearm_healthy_frames: 0,
        },
        ledger: crate::live_recovery::Ledger::default(),
        phase: 0,
        requested_at: None,
        recoveries: 0,
        prime_until: 0,
        healthy_frames: 0,
        rearmed: false,
    }));
    assert_eq!(callback.transition(&shared, START), 0);
    shared.requests.pop().unwrap();
    let mut item = Item::control(AUDIO, 0);
    item.n = 256;
    item.gain = f64::NAN;
    let mut out = [[0.; CAP]; 2];
    while shared.live_stage.load(Ordering::Acquire) != LIVE_REQUESTED {
        callback.process(&shared, item, &mut out).unwrap();
        assert!(callback.position < 4096);
    }
    let stale = Completion {
        audio: AudioResult {
            n: 256,
            position: 0,
            flags: 0,
            data: [[1.; CAP]; 2],
        },
        epoch: 1,
        intent_sequence: 1,
        returned: crate::process_results::Packet::default(),
    };
    assert!(shared.results.push(stale));
    let mut returned = crate::process_results::Packet::default();
    returned.points = 1;
    returned.point[0] = crate::process_results::Point {
        offset: 0,
        id: 77,
        value: 0.5,
    };
    assert!(callback.returned.append(&returned, 0, 256, callback.delay));
    assert_eq!(callback.returned.stats().pending_points, 1);
    shared
        .live_worker_return_ns
        .store(shared.live_now_ns(), Ordering::Release);
    shared.live_ack_epoch.store(2, Ordering::Release);
    shared
        .live_stage
        .store(LIVE_ACKNOWLEDGED, Ordering::Release);
    let mut flush = Item::control(AUDIO, 0);
    flush.n = 0;
    flush.gain = f64::NAN;
    flush.event_count = 1;
    flush.events[0] = parameter(0.9);
    let before_flush = shared.requests.published();
    callback.process(&shared, flush, &mut out).unwrap();
    assert_eq!(shared.requests.published(), before_flush);
    assert_eq!(callback.live.as_ref().unwrap().phase, 2);
    callback.process(&shared, item, &mut out).unwrap();
    assert_eq!(out, [[0.; CAP]; 2]);
    assert_eq!(shared.live_discard_results.load(Ordering::Acquire), 1);
    assert_eq!(callback.epoch, 2);
    assert_eq!(callback.next_result, 0);
    assert_eq!(callback.audio.len(), 0);
    assert_eq!(callback.returned.stats().pending_points, 0);
    assert_eq!(callback.returned.stats().discarded_on_reset, 1);
}

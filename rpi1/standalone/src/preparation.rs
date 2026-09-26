//! Binding-owned, opt-in state preparation before JACK can deliver audio.
use crate::binding::{Binding, Preparation};
use ap2_backend::rpi0::{
    self, Context, Delivery, Event, Identity, Instance, NOTE_OFF, NOTE_ON, STATE_CAPACITY,
};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    thread,
    time::{Duration, Instant},
};

const MAX_QUANTUM: usize = 256;

pub fn selected_plan<'a>(
    binding: &'a Binding,
    authorized: bool,
    has_state: bool,
) -> io::Result<Option<&'a Preparation>> {
    if !authorized {
        return Ok(None);
    }
    let plan = binding
        .preparation
        .as_ref()
        .ok_or_else(|| invalid("--prewarm requires a v2 preparation plan"))?;
    if !has_state {
        return Err(invalid("--prewarm requires an exact selected state"));
    }
    if binding.stereo_input || !binding.midi_input || binding.sha256.is_none() || plan.blocks() > 24
    {
        return Err(invalid("preparation binding capability/identity"));
    }
    Ok(Some(plan))
}

pub fn read_private_state(path: &Path, identity: Identity) -> io::Result<Vec<u8>> {
    if !path.is_absolute() {
        return Err(invalid("preparation requires absolute state path"));
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.len() > STATE_CAPACITY as u64
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        return Err(invalid("preparation state file owner, mode, or extent"));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take((STATE_CAPACITY + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() != metadata.len() as usize || bytes.len() > STATE_CAPACITY {
        return Err(invalid("preparation state file changed during read"));
    }
    rpi0::validate_bound_state(identity, &bytes)?;
    Ok(bytes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Readback {
    NotRun,
    Accepted,
    Exact,
    Refused,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Step {
    NotRun,
    Ok,
    Refused,
}

#[derive(Clone, Debug)]
pub struct Receipt {
    pub module: [u8; 32],
    pub class: [u8; 16],
    pub binding_sha256: [u8; 32],
    pub state_sha256: [u8; 32],
    pub schema: &'static str,
    pub blocks: usize,
    pub completed_blocks: usize,
    pub slowest_completion_us: u64,
    pub total_elapsed_us: u64,
    pub first_readback: Readback,
    pub final_readback: Readback,
    pub processing_fault: Step,
    pub stop: Step,
    pub deactivate: Step,
}
impl fmt::Display for Receipt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{b:02x}")).collect()
        }
        write!(f, "module={} class={} binding_sha256={} schema={} state_sha256={} blocks={} completed={} slowest_us={} elapsed_us={} first_readback={:?} final_readback={:?} fault={:?} stop={:?} deactivate={:?}",
            hex(&self.module), hex(&self.class), hex(&self.binding_sha256), self.schema, hex(&self.state_sha256), self.blocks,
            self.completed_blocks, self.slowest_completion_us, self.total_elapsed_us, self.first_readback, self.final_readback,
            self.processing_fault, self.stop, self.deactivate)
    }
}
#[derive(Debug)]
pub struct Failure {
    pub receipt: Receipt,
    pub error: io::Error,
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

// A narrow test seam over the real shared Instance; no alternative transport.
trait Peer {
    fn quantum(&self) -> u32;
    fn restore(&self, state: &[u8], readback: &mut [u8]) -> io::Result<usize>;
    fn activate(&self, frames: u32) -> io::Result<()>;
    fn start(&self) -> io::Result<()>;
    fn process(
        &self,
        frames: u32,
        input: [&[f32]; 2],
        output: [&mut [f32]; 2],
        events: &[Event],
        context: &Context,
        entered: u64,
        delivery: &mut Delivery,
    ) -> u32;
    fn stats(&self) -> io::Result<(u64, u64)>;
    fn stop(&self) -> io::Result<()>;
    fn deactivate(&self) -> io::Result<()>;
}

pub trait PlaybackGate {
    fn pause(&mut self) -> io::Result<()>;
    fn resume(&mut self);
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
impl PlaybackGate for lvb_arm_standalone::jack::Client {
    fn pause(&mut self) -> io::Result<()> {
        self.pause_processing()
    }
    fn resume(&mut self) {
        self.resume_processing()
    }
}
impl Peer for Instance {
    fn quantum(&self) -> u32 {
        self.processing_quantum()
    }
    fn restore(&self, state: &[u8], readback: &mut [u8]) -> io::Result<usize> {
        self.restore_state(state, readback)
    }
    fn activate(&self, frames: u32) -> io::Result<()> {
        self.activate(frames)
    }
    fn start(&self) -> io::Result<()> {
        self.start()
    }
    fn process(
        &self,
        frames: u32,
        input: [&[f32]; 2],
        output: [&mut [f32]; 2],
        events: &[Event],
        context: &Context,
        entered: u64,
        delivery: &mut Delivery,
    ) -> u32 {
        unsafe { self.process(frames, input, output, 3, events, context, entered, delivery) }
    }
    fn stats(&self) -> io::Result<(u64, u64)> {
        let s = self.stats()?;
        Ok((s.processed, s.fault))
    }
    fn stop(&self) -> io::Result<()> {
        self.stop()
    }
    fn deactivate(&self) -> io::Result<()> {
        self.deactivate()
    }
}

pub fn execute(
    instance: &Instance,
    binding: &Binding,
    identity: Identity,
    state: &[u8],
) -> Result<Receipt, Failure> {
    execute_with(instance, binding, identity, state)
}

pub fn restore_prepared<G: PlaybackGate>(
    instance: &Instance,
    gate: &mut G,
    binding: &Binding,
    identity: Identity,
    state: &[u8],
) -> Result<Receipt, Failure> {
    restore_prepared_with(instance, gate, binding, identity, state)
}

fn restore_prepared_with<P: Peer, G: PlaybackGate>(
    peer: &P,
    gate: &mut G,
    binding: &Binding,
    identity: Identity,
    state: &[u8],
) -> Result<Receipt, Failure> {
    let fail = |error| Failure {
        receipt: receipt_for(binding, identity, state),
        error,
    };
    selected_plan(binding, true, true).map_err(fail)?;
    rpi0::validate_bound_state(identity, state).map_err(fail)?;
    gate.pause().map_err(fail)?;
    peer.stop().map_err(fail)?;
    peer.deactivate().map_err(fail)?;
    let receipt = execute_with(peer, binding, identity, state)?;
    peer.activate(peer.quantum()).map_err(|error| Failure {
        receipt: receipt.clone(),
        error,
    })?;
    peer.start().map_err(|error| Failure {
        receipt: receipt.clone(),
        error,
    })?;
    gate.resume();
    Ok(receipt)
}

fn receipt_for(binding: &Binding, identity: Identity, state: &[u8]) -> Receipt {
    Receipt {
        module: identity.module,
        class: identity.class,
        binding_sha256: binding.sha256.unwrap_or_default(),
        state_sha256: Sha256::digest(state).into(),
        schema: Preparation::SCHEMA,
        blocks: binding.preparation.map_or(0, Preparation::blocks),
        completed_blocks: 0,
        slowest_completion_us: 0,
        total_elapsed_us: 0,
        first_readback: Readback::NotRun,
        final_readback: Readback::NotRun,
        processing_fault: Step::NotRun,
        stop: Step::NotRun,
        deactivate: Step::NotRun,
    }
}

fn execute_with<P: Peer>(
    peer: &P,
    binding: &Binding,
    identity: Identity,
    state: &[u8],
) -> Result<Receipt, Failure> {
    let started = Instant::now();
    let mut receipt = receipt_for(binding, identity, state);
    let operation = (|| -> io::Result<()> {
        let plan = selected_plan(binding, true, true)?
            .ok_or_else(|| invalid("preparation plan absent"))?;
        if identity.class != binding.class || binding.sha256.is_none() {
            return Err(invalid("preparation identity"));
        }
        rpi0::validate_bound_state(identity, state)?;
        let frames = peer.quantum() as usize;
        if frames == 0 || frames > MAX_QUANTUM || frames != 256 {
            return Err(invalid("unsupported preparation quantum"));
        }
        let mut readback = vec![0; STATE_CAPACITY];
        let first = peer.restore(state, &mut readback).map_err(|error| {
            receipt.first_readback = Readback::Refused;
            error
        })?;
        if first > readback.len()
            || rpi0::validate_bound_state(identity, &readback[..first]).is_err()
        {
            receipt.first_readback = Readback::Refused;
            return Err(invalid("initial state readback refused"));
        }
        receipt.first_readback = if readback[..first] == *state {
            Readback::Exact
        } else {
            Readback::Accepted
        };
        peer.activate(frames as u32)?;
        let processed = (|| -> io::Result<()> {
            peer.start()?;
            let (before, fault) = peer.stats()?;
            if fault != 0 {
                receipt.processing_fault = Step::Refused;
                return Err(invalid("preparation preexisting processing fault"));
            }
            receipt.processing_fault = Step::Ok;
            let zero = [0.0_f32; MAX_QUANTUM];
            let mut left = [0.0_f32; MAX_QUANTUM];
            let mut right = [0.0_f32; MAX_QUANTUM];
            let note = Event {
                kind: NOTE_ON,
                id: 1,
                channel: plan.channel as i16,
                pitch: plan.pitch as i16,
                value: f64::from(plan.velocity) / 127.0,
                ..Event::default()
            };
            let off = Event {
                kind: NOTE_OFF,
                value: 0.0,
                ..note
            };
            if !note.valid(frames) || !off.valid(frames) {
                return Err(invalid("preparation event contract"));
            }
            for block in 0..plan.blocks() {
                let call = Instant::now();
                let entered = monotonic_ns()?;
                let position = (block * frames) as i64;
                let music = position as f64 * 120.0 / (60.0 * 48_000.0);
                let context = Context {
                    present: 1,
                    state: 0x22f02,
                    rate: 48_000.0,
                    project: position,
                    system: entered.min(i64::MAX as u64) as i64,
                    continuous: position,
                    music,
                    bar: (music / 4.0).floor() * 4.0,
                    tempo: 120.0,
                    numerator: 4,
                    denominator: 4,
                    ..Context::default()
                };
                let event: &[Event] = if block == 0 {
                    std::slice::from_ref(&note)
                } else if block == usize::from(plan.hold_blocks) {
                    std::slice::from_ref(&off)
                } else {
                    &[]
                };
                let mut delivery = Delivery::default();
                let code = peer.process(
                    frames as u32,
                    [&zero[..frames], &zero[..frames]],
                    [&mut left[..frames], &mut right[..frames]],
                    event,
                    &context,
                    entered,
                    &mut delivery,
                );
                if code != 0 {
                    return Err(invalid(format!("preparation processing refused: {code}")));
                }
                let expected = before + block as u64 + 1;
                let deadline =
                    Instant::now() + Duration::from_millis(u64::from(plan.completion_timeout_ms));
                loop {
                    let (completed, fault) = peer.stats()?;
                    if fault != 0 {
                        receipt.processing_fault = Step::Refused;
                        return Err(invalid("preparation processing fault"));
                    }
                    if completed >= expected {
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err(invalid("preparation completion deadline"));
                    }
                    thread::sleep(Duration::from_micros(100));
                }
                receipt.completed_blocks += 1;
                receipt.slowest_completion_us = receipt
                    .slowest_completion_us
                    .max(call.elapsed().as_micros().min(u128::from(u64::MAX)) as u64);
            }
            receipt.processing_fault = Step::Ok;
            Ok(())
        })();
        let stop = peer.stop();
        receipt.stop = if stop.is_ok() {
            Step::Ok
        } else {
            Step::Refused
        };
        let off_result = peer.deactivate();
        receipt.deactivate = if off_result.is_ok() {
            Step::Ok
        } else {
            Step::Refused
        };
        processed?;
        stop?;
        off_result?;
        let final_count = peer.restore(state, &mut readback).map_err(|error| {
            receipt.final_readback = Readback::Refused;
            error
        })?;
        if final_count > readback.len()
            || rpi0::validate_bound_state(identity, &readback[..final_count]).is_err()
            || final_count != state.len()
            || readback[..final_count] != *state
        {
            receipt.final_readback = Readback::Refused;
            return Err(invalid("final state readback differs"));
        }
        receipt.final_readback = Readback::Exact;
        let (_, fault) = peer.stats()?;
        if fault != 0 {
            receipt.processing_fault = Step::Refused;
            return Err(invalid("preparation final processing fault"));
        }
        Ok(())
    })();
    receipt.total_elapsed_us = started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
    match operation {
        Ok(()) => Ok(receipt),
        Err(error) => Err(Failure { receipt, error }),
    }
}

fn monotonic_ns() -> io::Result<u64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((time.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(time.tv_nsec as u64))
}
fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Trace {
        calls: Vec<&'static str>,
        events: Vec<Vec<Event>>,
        processed: u64,
        held: bool,
        refuse_block: Option<usize>,
        stall_block: Option<usize>,
        bad_final: bool,
        bad_first: bool,
    }
    struct Fake {
        state: RefCell<Trace>,
        quantum: u32,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                state: RefCell::new(Trace::default()),
                quantum: 256,
            }
        }
        fn with(f: impl FnOnce(&mut Trace)) -> Self {
            let fake = Self::new();
            f(&mut fake.state.borrow_mut());
            fake
        }
    }
    #[derive(Default)]
    struct Gate {
        pauses: usize,
        resumes: usize,
    }
    impl PlaybackGate for Gate {
        fn pause(&mut self) -> io::Result<()> {
            self.pauses += 1;
            Ok(())
        }
        fn resume(&mut self) {
            self.resumes += 1;
        }
    }
    impl Peer for Fake {
        fn quantum(&self) -> u32 {
            self.quantum
        }
        fn restore(&self, state: &[u8], readback: &mut [u8]) -> io::Result<usize> {
            let mut trace = self.state.borrow_mut();
            let final_restore = trace.calls.contains(&"deactivate");
            trace.calls.push("restore");
            readback[..state.len()].copy_from_slice(state);
            if (final_restore && trace.bad_final) || (!final_restore && trace.bad_first) {
                readback[16] ^= 1;
            }
            trace.held = false;
            Ok(state.len())
        }
        fn activate(&self, _: u32) -> io::Result<()> {
            self.state.borrow_mut().calls.push("activate");
            Ok(())
        }
        fn start(&self) -> io::Result<()> {
            self.state.borrow_mut().calls.push("start");
            Ok(())
        }
        fn process(
            &self,
            _: u32,
            _: [&[f32]; 2],
            output: [&mut [f32]; 2],
            events: &[Event],
            _: &Context,
            _: u64,
            _: &mut Delivery,
        ) -> u32 {
            let mut trace = self.state.borrow_mut();
            let index = trace.events.len();
            trace.calls.push("process");
            trace.events.push(events.to_vec());
            for event in events {
                if event.kind == NOTE_ON {
                    trace.held = true;
                }
                if event.kind == NOTE_OFF {
                    trace.held = false;
                }
            }
            for channel in output {
                channel.fill(if trace.held { 0.75 } else { 0.0 });
            }
            if trace.refuse_block == Some(index) {
                return 1;
            }
            if trace.stall_block != Some(index) {
                trace.processed += 1;
            }
            0
        }
        fn stats(&self) -> io::Result<(u64, u64)> {
            let trace = self.state.borrow();
            Ok((trace.processed, 0))
        }
        fn stop(&self) -> io::Result<()> {
            self.state.borrow_mut().calls.push("stop");
            Ok(())
        }
        fn deactivate(&self) -> io::Result<()> {
            self.state.borrow_mut().calls.push("deactivate");
            Ok(())
        }
    }
    fn fixture() -> (Binding, Identity, Vec<u8>) {
        let mut binding = Binding::pigments();
        binding.legacy_master = false;
        binding.sha256 = Some([3; 32]);
        binding.preparation = Some(Preparation {
            channel: 0,
            pitch: 60,
            velocity: 100,
            hold_blocks: 1,
            settle_blocks: 6,
            completion_timeout_ms: 1,
        });
        let identity = Identity {
            class: binding.class,
            module: [5; 32],
        };
        // Valid, empty commercial component/controller/parameter state in the real envelope.
        let payload = [0u8; 16];
        let mut state = vec![0; 104];
        state[..8].copy_from_slice(b"LVBSTATE");
        state[8..12].copy_from_slice(&2u32.to_le_bytes());
        state[12..16].copy_from_slice(&104u32.to_le_bytes());
        state[16..32].copy_from_slice(&identity.class);
        state[32..64].copy_from_slice(&identity.module);
        state[64..68].copy_from_slice(&16u32.to_le_bytes());
        state[72..104].copy_from_slice(&Sha256::digest(payload));
        state.extend_from_slice(&payload);
        rpi0::validate_bound_state(identity, &state).unwrap();
        (binding, identity, state)
    }
    #[test]
    fn authorization_is_separate_from_binding_policy() {
        let (mut binding, _, _) = fixture();
        assert!(selected_plan(&binding, false, false).unwrap().is_none());
        assert!(selected_plan(&binding, false, true).unwrap().is_none());
        assert!(selected_plan(&binding, true, false).is_err());
        assert!(selected_plan(&binding, true, true).unwrap().is_some());
        binding.preparation = None;
        assert!(selected_plan(&binding, true, true).is_err());
    }
    #[test]
    fn exact_plan_completion_discards_output_and_leaves_no_note() {
        let (binding, identity, state) = fixture();
        let fake = Fake::new();
        let receipt = execute_with(&fake, &binding, identity, &state).unwrap();
        assert_eq!(receipt.blocks, 8);
        assert_eq!(receipt.completed_blocks, 8);
        assert_eq!(receipt.first_readback, Readback::Exact);
        assert_eq!(receipt.final_readback, Readback::Exact);
        assert_eq!(receipt.stop, Step::Ok);
        assert_eq!(receipt.deactivate, Step::Ok);
        assert_eq!(&receipt.state_sha256[..], Sha256::digest(&state).as_slice());
        let trace = fake.state.borrow();
        assert_eq!(&trace.calls[..3], &["restore", "activate", "start"]);
        assert_eq!(&trace.calls[11..], &["stop", "deactivate", "restore"]);
        assert_eq!(trace.events.len(), 8);
        assert_eq!(trace.events[0][0].kind, NOTE_ON);
        assert_eq!(trace.events[1][0].kind, NOTE_OFF);
        assert!(trace.events[2..].iter().all(Vec::is_empty));
        assert!(!trace.held);
        // The executor exposes a receipt, never its private process output.
        assert!(!format!("{receipt}").contains("0.75"));
        drop(trace);
        let zero = [0.0_f32; 256];
        let mut left = [1.0_f32; 256];
        let mut right = [1.0_f32; 256];
        assert_eq!(
            fake.process(
                256,
                [&zero, &zero],
                [&mut left, &mut right],
                &[],
                &Context::default(),
                0,
                &mut Delivery::default()
            ),
            0
        );
        assert!(left.iter().chain(&right).all(|sample| *sample == 0.0));
    }
    #[test]
    fn refusal_timeout_and_readback_mismatch_refuse_readiness_with_bounded_receipt() {
        let (binding, identity, state) = fixture();
        for fake in [
            Fake::with(|s| s.refuse_block = Some(1)),
            Fake::with(|s| s.stall_block = Some(0)),
            Fake::with(|s| s.bad_first = true),
            Fake::with(|s| s.bad_final = true),
        ] {
            let failure = execute_with(&fake, &binding, identity, &state)
                .err()
                .unwrap();
            let trace = fake.state.borrow();
            if trace.calls.contains(&"activate") {
                assert!(trace.calls.contains(&"stop"));
                assert!(trace.calls.contains(&"deactivate"));
            }
            assert!(
                failure.receipt.completed_blocks < 8
                    || failure.receipt.final_readback == Readback::Refused
            );
        }
    }
    #[test]
    fn wrong_state_identity_refuses_before_activation() {
        let (binding, identity, mut state) = fixture();
        state[16] ^= 1;
        let fake = Fake::new();
        assert!(execute_with(&fake, &binding, identity, &state).is_err());
        assert!(fake.state.borrow().calls.is_empty());
        let (_, _, valid) = fixture();
        let mut incompatible = Fake::new();
        incompatible.quantum = 512;
        assert!(execute_with(&incompatible, &binding, identity, &valid).is_err());
        assert!(incompatible.state.borrow().calls.is_empty());
    }

    #[test]
    fn explicit_restore_reuses_executor_and_resumes_only_after_success() {
        let (binding, identity, state) = fixture();
        let fake = Fake::new();
        let mut gate = Gate::default();
        let receipt = restore_prepared_with(&fake, &mut gate, &binding, identity, &state).unwrap();
        assert_eq!(receipt.completed_blocks, 8);
        assert_eq!((gate.pauses, gate.resumes), (1, 1));
        let calls = &fake.state.borrow().calls;
        assert_eq!(&calls[..3], &["stop", "deactivate", "restore"]);
        assert_eq!(&calls[calls.len() - 2..], &["activate", "start"]);
        let failed = Fake::with(|s| s.refuse_block = Some(0));
        let mut gate = Gate::default();
        assert!(restore_prepared_with(&failed, &mut gate, &binding, identity, &state).is_err());
        assert_eq!((gate.pauses, gate.resumes), (1, 0));
        let mut no_plan = binding.clone();
        no_plan.preparation = None;
        let mut gate = Gate::default();
        assert!(restore_prepared_with(&fake, &mut gate, &no_plan, identity, &state).is_err());
        assert_eq!(gate.pauses, 0);
    }
}

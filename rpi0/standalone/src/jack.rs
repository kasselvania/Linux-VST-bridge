use crate::{
    audio::ParameterUpdate, callback_gate::CallbackGate, midi::Parser,
    protocol::supported_jack_block, spsc::Queue, MAX_JACK_FRAMES, MAX_MIDI_EVENTS, SAMPLE_RATE,
};
use ap2_backend::rpi0::{Context, Delivery, Event, Instance};
use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    io, ptr, slice,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

type JackNFrames = u32;
enum JackClient {}
enum JackPort {}

#[repr(C)]
struct JackMidiEvent {
    time: JackNFrames,
    size: usize,
    buffer: *mut u8,
}

const JACK_NULL_OPTION: u32 = 0;
const JACK_PORT_IS_INPUT: u64 = 1;
const JACK_PORT_IS_OUTPUT: u64 = 2;
const JACK_DEFAULT_AUDIO_TYPE: &[u8] = b"32 bit float mono audio\0";
const JACK_DEFAULT_MIDI_TYPE: &[u8] = b"8 bit raw midi\0";

#[link(name = "jack")]
unsafe extern "C" {
    fn jack_client_open(
        name: *const c_char,
        options: u32,
        status: *mut u32,
        ...
    ) -> *mut JackClient;
    fn jack_client_close(client: *mut JackClient) -> c_int;
    fn jack_activate(client: *mut JackClient) -> c_int;
    fn jack_deactivate(client: *mut JackClient) -> c_int;
    fn jack_get_sample_rate(client: *mut JackClient) -> JackNFrames;
    fn jack_get_buffer_size(client: *mut JackClient) -> JackNFrames;
    fn jack_set_process_callback(
        client: *mut JackClient,
        callback: unsafe extern "C" fn(JackNFrames, *mut c_void) -> c_int,
        argument: *mut c_void,
    ) -> c_int;
    fn jack_port_register(
        client: *mut JackClient,
        name: *const c_char,
        port_type: *const c_char,
        flags: u64,
        buffer_size: u64,
    ) -> *mut JackPort;
    fn jack_port_get_buffer(port: *mut JackPort, frames: JackNFrames) -> *mut c_void;
    fn jack_port_name(port: *const JackPort) -> *const c_char;
    fn jack_midi_get_event_count(port_buffer: *mut c_void) -> u32;
    fn jack_midi_event_get(
        event: *mut JackMidiEvent,
        port_buffer: *mut c_void,
        index: u32,
    ) -> c_int;
}

#[derive(Default)]
pub struct Metrics {
    pub callbacks: AtomicU64,
    pub process_failures: AtomicU64,
    pub unsupported_blocks: AtomicU64,
    pub deadline_misses: AtomicU64,
    pub missing_frames: AtomicU64,
    pub expired_frames: AtomicU64,
    pub gaps: AtomicU64,
    pub delivered_frames: AtomicU64,
    pub priming_frames: AtomicU64,
    pub paused_frames: AtomicU64,
    pub callback_ns_max: AtomicU64,
    pub unsupported_midi: AtomicU64,
    pub malformed_midi: AtomicU64,
    pub overflow_midi: AtomicU64,
}

pub struct Control {
    pub parameters: Queue<ParameterUpdate, 64>,
    pub metrics: Metrics,
}
impl Control {
    pub const fn new() -> Self {
        Self {
            parameters: Queue::new(),
            metrics: Metrics {
                callbacks: AtomicU64::new(0),
                process_failures: AtomicU64::new(0),
                unsupported_blocks: AtomicU64::new(0),
                deadline_misses: AtomicU64::new(0),
                missing_frames: AtomicU64::new(0),
                expired_frames: AtomicU64::new(0),
                gaps: AtomicU64::new(0),
                delivered_frames: AtomicU64::new(0),
                priming_frames: AtomicU64::new(0),
                paused_frames: AtomicU64::new(0),
                callback_ns_max: AtomicU64::new(0),
                unsupported_midi: AtomicU64::new(0),
                malformed_midi: AtomicU64::new(0),
                overflow_midi: AtomicU64::new(0),
            },
        }
    }
}
impl Default for Control {
    fn default() -> Self {
        Self::new()
    }
}

struct Rt {
    instance: *const Instance,
    control: *const Control,
    midi: *mut JackPort,
    left: *mut JackPort,
    right: *mut JackPort,
    position: u64,
    parser: Parser,
    events: [Event; MAX_MIDI_EVENTS],
    zero: [f32; MAX_JACK_FRAMES],
    gate: CallbackGate,
}

pub struct Client {
    client: *mut JackClient,
    rt: Box<Rt>,
    buffer_size: u32,
}

unsafe impl Send for Client {}

impl Client {
    pub fn open(name: &str, instance: &Instance, control: &Control) -> io::Result<Self> {
        let name = CString::new(name).map_err(|_| invalid("JACK name"))?;
        let mut status = 0;
        let client = unsafe { jack_client_open(name.as_ptr(), JACK_NULL_OPTION, &mut status) };
        if client.is_null() {
            return Err(invalid(format!("JACK open failed: {status}")));
        }
        let register = |name: &str, kind: &[u8], flags| -> io::Result<*mut JackPort> {
            let name = CString::new(name).unwrap();
            let port = unsafe {
                jack_port_register(client, name.as_ptr(), kind.as_ptr().cast(), flags, 0)
            };
            if port.is_null() {
                Err(invalid("JACK port registration"))
            } else {
                Ok(port)
            }
        };
        let midi = register("midi_in", JACK_DEFAULT_MIDI_TYPE, JACK_PORT_IS_INPUT)?;
        let left = register("audio_out_l", JACK_DEFAULT_AUDIO_TYPE, JACK_PORT_IS_OUTPUT)?;
        let right = register("audio_out_r", JACK_DEFAULT_AUDIO_TYPE, JACK_PORT_IS_OUTPUT)?;
        if unsafe { jack_get_sample_rate(client) } != SAMPLE_RATE {
            unsafe {
                jack_client_close(client);
            }
            return Err(invalid("JACK sample rate must be 48000"));
        }
        let block = unsafe { jack_get_buffer_size(client) } as usize;
        if !supported_jack_block(block) {
            unsafe {
                jack_client_close(client);
            }
            return Err(invalid("JACK block size outside qualified set"));
        }
        let mut rt = Box::new(Rt {
            instance,
            control,
            midi,
            left,
            right,
            position: 0,
            parser: Parser::default(),
            events: [Event::default(); MAX_MIDI_EVENTS],
            zero: [0.; MAX_JACK_FRAMES],
            gate: CallbackGate::new(),
        });
        let code =
            unsafe { jack_set_process_callback(client, process, (&mut *rt as *mut Rt).cast()) };
        if code != 0 {
            unsafe {
                jack_client_close(client);
            }
            return Err(invalid("JACK callback registration"));
        }
        Ok(Self {
            client,
            rt,
            buffer_size: block as u32,
        })
    }

    pub fn activate(&mut self) -> io::Result<()> {
        self.rt.gate.resume();
        let code = unsafe { jack_activate(self.client) };
        if code == 0 {
            Ok(())
        } else {
            let _ = self.rt.gate.pause(Duration::from_millis(10));
            Err(invalid("JACK activation"))
        }
    }

    pub fn ports(&self) -> [String; 3] {
        unsafe {
            [self.rt.midi, self.rt.left, self.rt.right].map(|port| {
                let name = jack_port_name(port);
                if name.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(name).to_string_lossy().into_owned()
                }
            })
        }
    }

    pub fn buffer_size(&self) -> u32 {
        self.buffer_size
    }

    pub fn pause_processing(&mut self) -> io::Result<()> {
        self.rt.gate.pause(Duration::from_secs(1))
    }

    pub fn resume_processing(&mut self) {
        self.rt.gate.resume();
    }

    pub fn deactivate(&mut self) -> io::Result<()> {
        self.rt.gate.pause(Duration::from_secs(1))?;
        let code = unsafe { jack_deactivate(self.client) };
        if code == 0 {
            Ok(())
        } else {
            Err(invalid("JACK deactivation"))
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        unsafe {
            let _ = jack_deactivate(self.client);
            let _ = jack_client_close(self.client);
        }
    }
}

unsafe extern "C" fn process(frames: JackNFrames, argument: *mut c_void) -> c_int {
    let rt = &mut *argument.cast::<Rt>();
    let control = &*rt.control;
    let metrics = &control.metrics;
    metrics.callbacks.fetch_add(1, Ordering::Relaxed);
    let count = frames as usize;
    let left =
        slice::from_raw_parts_mut(jack_port_get_buffer(rt.left, frames).cast::<f32>(), count);
    let right =
        slice::from_raw_parts_mut(jack_port_get_buffer(rt.right, frames).cast::<f32>(), count);
    if count > MAX_JACK_FRAMES || !supported_jack_block(count) {
        left.fill(0.);
        right.fill(0.);
        metrics.unsupported_blocks.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    let Some(_lease) = rt.gate.enter() else {
        left.fill(0.);
        right.fill(0.);
        rt.position = rt.position.saturating_add(frames as u64);
        metrics
            .paused_frames
            .fetch_add(frames as u64, Ordering::Relaxed);
        return 0;
    };
    let before = monotonic_ns();
    let mut event_count = 0usize;
    while let Some(update) = control.parameters.pop() {
        if event_count == rt.events.len() {
            metrics.overflow_midi.fetch_add(1, Ordering::Relaxed);
            break;
        }
        if update.value.is_finite() && (0.0..=1.0).contains(&update.value) {
            rt.events[event_count] = Event {
                kind: ap2_backend::rpi0::PARAMETER,
                id: update.id,
                value: update.value,
                ..Event::default()
            };
            event_count += 1;
        }
    }
    let midi_buffer = jack_port_get_buffer(rt.midi, frames);
    let midi_count = jack_midi_get_event_count(midi_buffer);
    let prior = rt.parser.counters;
    for index in 0..midi_count {
        let mut raw = JackMidiEvent {
            time: 0,
            size: 0,
            buffer: ptr::null_mut(),
        };
        if jack_midi_event_get(&mut raw, midi_buffer, index) != 0
            || raw.buffer.is_null()
            || raw.size > 3
        {
            metrics.malformed_midi.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        if event_count == rt.events.len() {
            metrics.overflow_midi.fetch_add(1, Ordering::Relaxed);
            break;
        }
        let bytes = slice::from_raw_parts(raw.buffer, raw.size);
        if let Ok(event) = rt.parser.parse(raw.time, bytes, frames) {
            rt.events[event_count] = event;
            event_count += 1;
        }
    }
    metrics.unsupported_midi.fetch_add(
        rt.parser.counters.unsupported - prior.unsupported,
        Ordering::Relaxed,
    );
    metrics.malformed_midi.fetch_add(
        rt.parser.counters.malformed - prior.malformed,
        Ordering::Relaxed,
    );
    metrics.overflow_midi.fetch_add(
        rt.parser.counters.overflow - prior.overflow,
        Ordering::Relaxed,
    );
    let position = rt.position.min(i64::MAX as u64) as i64;
    let music = position as f64 * 120.0 / (60.0 * SAMPLE_RATE as f64);
    let context = Context {
        present: 1,
        state: 0x22f02,
        rate: SAMPLE_RATE as f64,
        project: position,
        system: before.min(i64::MAX as u64) as i64,
        continuous: position,
        music,
        bar: (music / 4.0).floor() * 4.0,
        cycle_start: 0.0,
        cycle_end: 0.0,
        tempo: 120.0,
        numerator: 4,
        denominator: 4,
        clock: 0,
        reserved: 0,
    };
    let mut delivery = Delivery::default();
    let result = (*rt.instance).process(
        frames,
        [&rt.zero[..count], &rt.zero[..count]],
        [&mut *left, &mut *right],
        3,
        &rt.events[..event_count],
        &context,
        before,
        &mut delivery,
    );
    rt.position = rt.position.saturating_add(frames as u64);
    if result != 0 {
        left.fill(0.);
        right.fill(0.);
        metrics.process_failures.fetch_add(1, Ordering::Relaxed);
    }
    metrics
        .missing_frames
        .fetch_add(delivery.missing_frames, Ordering::Relaxed);
    metrics
        .expired_frames
        .fetch_add(delivery.expired_frames, Ordering::Relaxed);
    metrics.gaps.fetch_add(delivery.gaps, Ordering::Relaxed);
    metrics
        .delivered_frames
        .fetch_add(delivery.delivered_frames, Ordering::Relaxed);
    metrics
        .priming_frames
        .fetch_add(delivery.priming_frames, Ordering::Relaxed);
    let elapsed = monotonic_ns().saturating_sub(before);
    metrics
        .callback_ns_max
        .fetch_max(elapsed, Ordering::Relaxed);
    if elapsed > (frames as u64 * 1_000_000_000 / SAMPLE_RATE as u64) {
        metrics.deadline_misses.fetch_add(1, Ordering::Relaxed);
    }
    0
}

fn monotonic_ns() -> u64 {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut time);
    }
    (time.tv_sec as u64)
        .saturating_mul(1_000_000_000)
        .saturating_add(time.tv_nsec as u64)
}
fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

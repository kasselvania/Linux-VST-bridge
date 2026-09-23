use super::{note_offset, polyphony_events, stress_events, two_note_events, tone, Capture, Meter, NOTE_EVENTS, RATE};
use std::{
    ffi::{c_char, c_int, c_void, CStr, CString},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    ptr, slice,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

enum Client {}
enum Port {}
#[link(name = "jack")]
unsafe extern "C" {
    fn jack_client_open(name: *const c_char, options: u32, status: *mut u32, ...) -> *mut Client;
    fn jack_client_close(client: *mut Client) -> c_int;
    fn jack_activate(client: *mut Client) -> c_int;
    fn jack_deactivate(client: *mut Client) -> c_int;
    fn jack_get_sample_rate(client: *mut Client) -> u32;
    fn jack_get_buffer_size(client: *mut Client) -> u32;
    fn jack_port_register(
        client: *mut Client,
        name: *const c_char,
        kind: *const c_char,
        flags: u64,
        size: u64,
    ) -> *mut Port;
    fn jack_port_by_name(client: *mut Client, name: *const c_char) -> *mut Port;
    fn jack_port_name(port: *const Port) -> *const c_char;
    fn jack_port_type(port: *const Port) -> *const c_char;
    fn jack_port_flags(port: *const Port) -> c_int;
    fn jack_port_get_buffer(port: *mut Port, frames: u32) -> *mut c_void;
    fn jack_set_process_callback(
        client: *mut Client,
        callback: unsafe extern "C" fn(u32, *mut c_void) -> c_int,
        arg: *mut c_void,
    ) -> c_int;
    fn jack_set_xrun_callback(
        client: *mut Client,
        callback: unsafe extern "C" fn(*mut c_void) -> c_int,
        arg: *mut c_void,
    ) -> c_int;
    fn jack_connect(
        client: *mut Client,
        source: *const c_char,
        destination: *const c_char,
    ) -> c_int;
    fn jack_disconnect(
        client: *mut Client,
        source: *const c_char,
        destination: *const c_char,
    ) -> c_int;
    fn jack_port_connected_to(port: *const Port, destination: *const c_char) -> c_int;
    fn jack_midi_clear_buffer(buffer: *mut c_void);
    fn jack_midi_event_write(buffer: *mut c_void, time: u32, data: *const u8, size: usize)
        -> c_int;
}
const AUDIO: &[u8] = b"32 bit float mono audio\0";
const MIDI: &[u8] = b"8 bit raw midi\0";
const NAME: &str = "lvb-rpi1-qualification";

#[derive(Default)]
struct Signals {
    armed: AtomicBool,
    done: AtomicBool,
    xruns: AtomicU64,
}
struct Rt {
    signals: *const Signals,
    midi: *mut Port,
    inputs: [*mut Port; 2],
    outputs: [*mut Port; 2],
    tones: Vec<[f32; 2]>,
    notes: Vec<(u64, [u8; 3])>,
    meters: [Meter; 2],
    capture: Option<Capture>,
    input_capture: Option<Capture>,
    physical_inputs: Option<[*mut Port; 2]>,
    position: u64,
    total: u64,
    midi_sent: u64,
    midi_errors: u64,
    bad_blocks: u64,
    tone_mode: bool,
}
unsafe extern "C" fn xrun(arg: *mut c_void) -> c_int {
    (*arg.cast::<Signals>())
        .xruns
        .fetch_add(1, Ordering::Relaxed);
    0
}
unsafe extern "C" fn process(frames: u32, arg: *mut c_void) -> c_int {
    let rt = &mut *arg.cast::<Rt>();
    let signals = &*rt.signals;
    let midi = jack_port_get_buffer(rt.midi, frames);
    jack_midi_clear_buffer(midi);
    let armed = signals.armed.load(Ordering::Acquire) && !signals.done.load(Ordering::Acquire);
    if rt.tone_mode {
        for ch in 0..2 {
            let output = slice::from_raw_parts_mut(
                jack_port_get_buffer(rt.outputs[ch], frames).cast::<f32>(),
                frames as usize,
            );
            output.fill(0.0);
            if armed && frames <= 4096 {
                for (index, value) in output.iter_mut().enumerate() {
                    if let Some(sample) = rt.tones.get(rt.position as usize + index) {
                        *value = sample[ch];
                    }
                }
                if let Some(capture) = &mut rt.input_capture {
                    let count = u64::from(frames).min(rt.total.saturating_sub(rt.position)) as usize;
                    if !capture.store_channel(rt.position as usize, ch, &output[..count]) {
                        rt.bad_blocks += 1;
                        signals.done.store(true, Ordering::Release);
                    }
                }
            }
        }
    }
    if !armed {
        return 0;
    }
    if frames == 0 || frames > 4096 {
        rt.bad_blocks += 1;
        signals.done.store(true, Ordering::Release);
        return 0;
    }
    let count = u64::from(frames).min(rt.total.saturating_sub(rt.position)) as usize;
    if let (Some(ports), Some(capture)) = (rt.physical_inputs, &mut rt.input_capture) {
        for ch in 0..2 {
            let input = slice::from_raw_parts(jack_port_get_buffer(ports[ch], frames).cast::<f32>(), count);
            if !capture.store_channel(rt.position as usize, ch, input) {
                rt.bad_blocks += 1; signals.done.store(true, Ordering::Release); return 0;
            }
        }
    }
    for ch in 0..2 {
        let input = slice::from_raw_parts(
            jack_port_get_buffer(rt.inputs[ch], frames).cast::<f32>(),
            count,
        );
        rt.meters[ch].observe(input);
        if let Some(capture) = &mut rt.capture {
            if !capture.store_channel(rt.position as usize, ch, input) {
                rt.bad_blocks += 1;
                signals.done.store(true, Ordering::Release);
                return 0;
            }
        }
    }
    if !rt.tone_mode {
        for &(time, data) in &rt.notes {
            if let Some(offset) = note_offset(time, rt.position, frames) {
                if jack_midi_event_write(midi, offset, data.as_ptr(), data.len()) == 0 {
                    rt.midi_sent += 1;
                } else {
                    rt.midi_errors += 1;
                }
            }
        }
    }
    rt.position += u64::from(frames);
    if rt.position >= rt.total {
        signals.done.store(true, Ordering::Release);
    }
    0
}

struct Owner {
    client: *mut Client,
    connections: Vec<(CString, CString)>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            jack_deactivate(self.client);
            for (source, destination) in &self.connections {
                jack_disconnect(self.client, source.as_ptr(), destination.as_ptr());
            }
            jack_client_close(self.client);
        }
    }
}
fn fail(message: &str) -> io::Error {
    io::Error::other(message)
}
fn c(value: &str) -> CString {
    CString::new(value).unwrap()
}

pub fn run() -> io::Result<()> {
    let arguments = std::env::args().collect::<Vec<_>>();
    let bridge_client = std::env::var("LVB_QUALIFICATION_CLIENT")
        .unwrap_or_else(|_| "lvb-arm-pigments".to_owned());
    if bridge_client.is_empty() || !bridge_client.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')) {
        return Err(fail("invalid JACK bridge client name"));
    }
    if !matches!(arguments.len(), 2 | 4)
        || !matches!(arguments[1].as_str(), "tone" | "effect" | "input" | "notes" | "pigments" | "polyphony" | "stress" | "two-notes")
        || (arguments.len() == 4 && arguments[2] != "--capture")
    {
        return Err(fail(
            "usage: qualification tone|effect|input|notes|pigments|polyphony|stress|two-notes [--capture NEW_PRIVATE_F32LE_FILE]",
        ));
    }
    let effect_mode = arguments[1] == "effect";
    let physical_mode = arguments[1] == "input";
    let tone_mode = arguments[1] == "tone" || effect_mode;
    let polyphony = arguments[1] == "polyphony";
    let stress = arguments[1] == "stress";
    let two_notes = arguments[1] == "two-notes";
    let total = RATE
        * if effect_mode {
            5
        } else if tone_mode {
            3
        } else if two_notes {
            20
        } else if stress {
            120
        } else if polyphony {
            30
        } else {
            5
        };
    // Refuse an existing destination before any note or tone. Never overwrite recordings.
    let capture_file = if arguments.len() == 4 {
        Some(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&arguments[3])?,
        )
    } else {
        None
    };
    let input_file = if (effect_mode || physical_mode) && arguments.len() == 4 {
        Some(std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
            .open(format!("{}.input.f32le", arguments[3]))?)
    } else { None };
    let mut status = 0;
    let client = unsafe { jack_client_open(c(NAME).as_ptr(), 3, &mut status) }; // NoStartServer | UseExactName
    if client.is_null() {
        return Err(fail("exact JACK client unavailable; server not started"));
    }
    // Declare owner last so it quiesces callbacks before Rt/signals are dropped.
    let signals = Box::<Signals>::default();
    let mut rt = Box::new(Rt {
        signals: &*signals,
        midi: ptr::null_mut(),
        inputs: [ptr::null_mut(); 2],
        outputs: [ptr::null_mut(); 2],
        tones: if tone_mode {
            (0..RATE * 3).map(|i| [tone(i, 0), tone(i, 1)]).collect()
        } else {
            Vec::new()
        },
        notes: if physical_mode { Vec::new() } else if two_notes {
            two_note_events()
        } else if stress {
            stress_events()
        } else if polyphony {
            polyphony_events()
        } else {
            NOTE_EVENTS.to_vec()
        },
        meters: [Meter::default(), Meter::default()],
        capture: capture_file.as_ref().map(|_| Capture::new(total as usize)),
        input_capture: input_file.as_ref().map(|_| Capture::new(total as usize)),
        physical_inputs: None,
        position: 0,
        total,
        midi_sent: 0,
        midi_errors: 0,
        bad_blocks: 0,
        tone_mode,
    });
    let mut owner = Owner {
        client,
        connections: Vec::new(),
    };
    let rate = unsafe { jack_get_sample_rate(client) };
    let period = unsafe { jack_get_buffer_size(client) };
    if rate != RATE as u32 || period != 512 {
        return Err(fail("requires exact 48000 Hz / 512-frame JACK graph"));
    }
    let register = |name: &str, kind: &[u8], flags| -> io::Result<*mut Port> {
        let port =
            unsafe { jack_port_register(client, c(name).as_ptr(), kind.as_ptr().cast(), flags, 0) };
        if port.is_null() {
            return Err(fail("port registration"));
        }
        if unsafe { CStr::from_ptr(jack_port_name(port)) }.to_bytes()
            != format!("{NAME}:{name}").as_bytes()
        {
            return Err(fail("port was renamed"));
        }
        Ok(port)
    };
    rt.midi = register("midi_out", MIDI, 2)?;
    rt.inputs = [
        register("meter_l", AUDIO, 1)?,
        register("meter_r", AUDIO, 1)?,
    ];
    if tone_mode {
        rt.outputs = [register("tone_l", AUDIO, 2)?, register("tone_r", AUDIO, 2)?];
    }
    if physical_mode {
        rt.physical_inputs = Some([register("source_l", AUDIO, 1)?, register("source_r", AUDIO, 1)?]);
    }
    let mut routes = Vec::new();
    if !tone_mode && !physical_mode {
        routes.push((
            format!("{NAME}:midi_out"),
            format!("{bridge_client}:midi_in"),
            MIDI,
        ));
    }
    for (index, suffix) in ["l", "r"].into_iter().enumerate() {
        if physical_mode {
            let source = format!("system:capture_{}", index + 1);
            routes.push((source.clone(), format!("{NAME}:source_{suffix}"), AUDIO));
            routes.push((source, format!("{bridge_client}:audio_in_{suffix}"), AUDIO));
        }
        if effect_mode {
            routes.push((format!("{NAME}:tone_{suffix}"), format!("{bridge_client}:audio_in_{suffix}"), AUDIO));
        }
        let source = if tone_mode && !effect_mode {
            format!("{NAME}:tone_{suffix}")
        } else {
            format!("{bridge_client}:audio_out_{suffix}")
        };
        routes.push((source.clone(), format!("{NAME}:meter_{suffix}"), AUDIO));
        routes.push((source, format!("system:playback_{}", index + 1), AUDIO));
    }
    // Validate the entire graph before activating or sending any note/tone.
    for (source, destination, kind) in &routes {
        for (name, flag) in [(source, 2), (destination, 1)] {
            let port = unsafe { jack_port_by_name(client, c(name).as_ptr()) };
            if port.is_null()
                || unsafe { jack_port_flags(port) } & flag == 0
                || unsafe { CStr::from_ptr(jack_port_type(port)) }.to_bytes_with_nul() != *kind
            {
                return Err(fail("exact expected port name, direction or type differs"));
            }
        }
    }
    if unsafe { jack_set_process_callback(client, process, (&mut *rt as *mut Rt).cast()) } != 0
        || unsafe {
            jack_set_xrun_callback(
                client,
                xrun,
                (&*signals as *const Signals as *mut Signals).cast(),
            )
        } != 0
    {
        return Err(fail("callback registration"));
    }
    if unsafe { jack_activate(client) } != 0 {
        return Err(fail("activation"));
    }
    for (source, destination, _) in &routes {
        let (source, destination) = (c(source), c(destination));
        let result = unsafe { jack_connect(client, source.as_ptr(), destination.as_ptr()) };
        if result == 0 {
            owner
                .connections
                .push((source.clone(), destination.clone()));
        } else if result != 17 {
            return Err(fail("exact connection refused"));
        }
        let port = unsafe { jack_port_by_name(client, source.as_ptr()) };
        if unsafe { jack_port_connected_to(port, destination.as_ptr()) } == 0 {
            return Err(fail("connection readback absent"));
        }
    }
    println!(
        "RPI1_QUALIFICATION_GRAPH mode={} rate={} period={} routes={:?}",
        arguments[1],
        rate,
        period,
        routes.iter().map(|(a, b, _)| [a, b]).collect::<Vec<_>>()
    );
    io::stdout().flush()?;
    signals.armed.store(true, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(total / RATE + 10);
    while !signals.done.load(Ordering::Acquire) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    let finished = signals.done.load(Ordering::Acquire);
    if unsafe { jack_deactivate(client) } != 0 {
        return Err(fail("callback retirement failed"));
    }
    for (ch, meter) in rt.meters.iter().enumerate() {
        println!("RPI1_QUALIFICATION_AUDIO channel={} samples={} nonzero_samples={} nonzero_windows={} peak={} rms={} nonfinite={}",ch,meter.samples,meter.nonzero,meter.nonzero_windows,meter.peak,meter.rms(),meter.nonfinite);
    }
    let good = finished
        && rt.bad_blocks == 0
        && rt.midi_errors == 0
        && (tone_mode || rt.midi_sent == rt.notes.len() as u64)
        && rt.meters.iter().all(|m| m.nonzero > 0 && m.nonfinite == 0);
    println!("RPI1_QUALIFICATION_RESULT mode={} completed={} midi_sent={} midi_errors={} xruns={} bad_blocks={} stereo_nonzero={}",arguments[1],finished,rt.midi_sent,rt.midi_errors,signals.xruns.load(Ordering::Acquire),rt.bad_blocks,good);
    drop(owner);
    println!("RPI1_QUALIFICATION_RETIRED");
    if let (Some(file), Some(capture)) = (capture_file, rt.capture.as_ref()) {
        if !finished || rt.bad_blocks != 0 {
            return Err(fail("incomplete capture; empty destination retained"));
        }
        let mut writer = io::BufWriter::new(file);
        capture.write_interleaved(&mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        println!(
            "RPI1_QUALIFICATION_CAPTURE format=f32le channels=2 rate={} frames={} bytes={}",
            RATE,
            total,
            total * 8
        );
    }
    if let (Some(file), Some(capture)) = (input_file, rt.input_capture.as_ref()) {
        if !finished || rt.bad_blocks != 0 { return Err(fail("incomplete input capture")); }
        let mut writer = io::BufWriter::new(file);
        capture.write_interleaved(&mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        println!("RPI1_QUALIFICATION_INPUT_CAPTURE frames={total} channels=2 format=f32le");
    }
    if good {
        Ok(())
    } else {
        Err(fail("bounded MIDI/audio observation did not pass"))
    }
}

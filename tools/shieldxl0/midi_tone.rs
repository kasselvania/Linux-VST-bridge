//! Audible UART MIDI wiring fixture on the existing JACK server, not a plug-in host.
use std::{
    ffi::{c_char, c_int, c_void, CString},
    io::{self, Write},
    ptr, slice,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};
enum Client {}
enum Port {}
#[repr(C)]
struct MidiEvent {
    time: u32,
    size: usize,
    buffer: *mut u8,
}
#[link(name = "jack")]
unsafe extern "C" {
    fn jack_client_open(n: *const c_char, o: u32, s: *mut u32, ...) -> *mut Client;
    fn jack_client_close(c: *mut Client) -> c_int;
    fn jack_activate(c: *mut Client) -> c_int;
    fn jack_deactivate(c: *mut Client) -> c_int;
    fn jack_get_sample_rate(c: *mut Client) -> u32;
    fn jack_port_register(
        c: *mut Client,
        n: *const c_char,
        t: *const c_char,
        f: u64,
        b: u64,
    ) -> *mut Port;
    fn jack_port_by_name(c: *mut Client, n: *const c_char) -> *mut Port;
    fn jack_port_flags(p: *const Port) -> c_int;
    fn jack_port_get_buffer(p: *mut Port, n: u32) -> *mut c_void;
    fn jack_set_process_callback(
        c: *mut Client,
        f: unsafe extern "C" fn(u32, *mut c_void) -> c_int,
        a: *mut c_void,
    ) -> c_int;
    fn jack_connect(c: *mut Client, s: *const c_char, d: *const c_char) -> c_int;
    fn jack_midi_get_event_count(b: *mut c_void) -> u32;
    fn jack_midi_event_get(e: *mut MidiEvent, b: *mut c_void, i: u32) -> c_int;
}
fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}
struct Signals {
    armed: AtomicBool,
    messages: AtomicU64,
    notes: AtomicU64,
    cc: AtomicU64,
    errors: AtomicU64,
}
struct Rt {
    signals: *const Signals,
    midi: *mut Port,
    out: [*mut Port; 2],
    wave: [f32; 2048],
    increments: [f64; 128],
    held: [bool; 2048],
    selected: usize,
    phase: f64,
    gain: f32,
    chirp: u32,
    at: u64,
    rate: u32,
}
unsafe extern "C" fn process(n: u32, arg: *mut c_void) -> c_int {
    let rt = &mut *arg.cast::<Rt>();
    let s = &*rt.signals;
    let left =
        slice::from_raw_parts_mut(jack_port_get_buffer(rt.out[0], n).cast::<f32>(), n as usize);
    let right =
        slice::from_raw_parts_mut(jack_port_get_buffer(rt.out[1], n).cast::<f32>(), n as usize);
    left.fill(0.0);
    right.fill(0.0);
    if !s.armed.load(Ordering::Acquire) {
        return 0;
    }
    let b = jack_port_get_buffer(rt.midi, n);
    let count = jack_midi_get_event_count(b);
    if count > 256 {
        s.errors.fetch_add(1, Ordering::Relaxed);
        return 0;
    }
    for i in 0..count {
        let mut e = MidiEvent {
            time: 0,
            size: 0,
            buffer: ptr::null_mut(),
        };
        if jack_midi_event_get(&mut e, b, i) != 0 || e.size != 3 {
            continue;
        }
        let m = slice::from_raw_parts(e.buffer, 3);
        if m[1] > 127 || m[2] > 127 {
            s.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let index = usize::from(m[0] & 15) * 128 + usize::from(m[1]);
        match m[0] & 0xf0 {
            0x90 if m[2] > 0 => {
                rt.held[index] = true;
                rt.selected = index;
                s.notes.fetch_add(1, Ordering::Relaxed);
            }
            0x80 | 0x90 => {
                rt.held[index] = false;
                if rt.selected == index {
                    rt.selected = rt.held.iter().position(|v| *v).unwrap_or(0);
                }
            }
            0xb0 => {
                if m[1] == 120 || m[1] == 123 {
                    rt.held[usize::from(m[0] & 15) * 128..usize::from(m[0] & 15) * 128 + 128]
                        .fill(false);
                }
                rt.chirp = rt.rate / 5;
                s.cc.fetch_add(1, Ordering::Relaxed);
            }
            _ => continue,
        }
        s.messages.fetch_add(1, Ordering::Relaxed);
    }
    for i in 0..n as usize {
        // Three brief startup beeps prove the listening route. No periodic automatic tones.
        let startup =
            rt.at < u64::from(rt.rate) && rt.at % u64::from(rt.rate / 3) < u64::from(rt.rate / 12);
        let note = rt.held[rt.selected];
        let on = startup || note || rt.chirp > 0;
        let target = if on { 0.05 } else { 0.0 };
        rt.gain += (target - rt.gain).clamp(-0.00025, 0.00025);
        let increment = if note {
            rt.increments[rt.selected % 128]
        } else {
            440.0 * 2048.0 / f64::from(rt.rate)
        };
        rt.phase = (rt.phase + increment) % 2048.0;
        let sample = rt.wave[rt.phase as usize] * rt.gain;
        left[i] = sample;
        right[i] = sample;
        rt.chirp = rt.chirp.saturating_sub(1);
        rt.at += 1;
    }
    0
}
struct Owner(*mut Client);
impl Drop for Owner {
    fn drop(&mut self) {
        unsafe {
            jack_deactivate(self.0);
            jack_client_close(self.0);
        }
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: midi-tone EXACT_JACK_MIDI_SOURCE".into());
    }
    let mut status = 0;
    let client = unsafe { jack_client_open(c("shieldxl-midi-tone").as_ptr(), 3, &mut status) };
    if client.is_null() {
        return Err("JACK unavailable or fixture already active".into());
    }
    let rate = unsafe { jack_get_sample_rate(client) };
    let signals = Box::new(Signals {
        armed: AtomicBool::new(false),
        messages: AtomicU64::new(0),
        notes: AtomicU64::new(0),
        cc: AtomicU64::new(0),
        errors: AtomicU64::new(0),
    });
    let mut rt = Box::new(Rt {
        signals: &*signals,
        midi: ptr::null_mut(),
        out: [ptr::null_mut(); 2],
        wave: std::array::from_fn(|i| (std::f64::consts::TAU * i as f64 / 2048.0).sin() as f32),
        increments: std::array::from_fn(|i| {
            440.0 * 2.0_f64.powf((i as f64 - 69.0) / 12.0) * 2048.0 / f64::from(rate)
        }),
        held: [false; 2048],
        selected: 0,
        phase: 0.0,
        gain: 0.0,
        chirp: 0,
        at: 0,
        rate,
    });
    let owner = Owner(client);
    if rate != 48000 {
        return Err("expected 48000 Hz JACK".into());
    }
    rt.midi =
        unsafe { jack_port_register(client, c("in").as_ptr(), c("8 bit raw midi").as_ptr(), 1, 0) };
    for (i, name) in ["left", "right"].iter().enumerate() {
        rt.out[i] = unsafe {
            jack_port_register(
                client,
                c(name).as_ptr(),
                c("32 bit float mono audio").as_ptr(),
                2,
                0,
            )
        };
    }
    if rt.midi.is_null() || rt.out.iter().any(|p| p.is_null()) {
        return Err("port registration failed".into());
    }
    let source = unsafe { jack_port_by_name(client, c(&args[1]).as_ptr()) };
    if source.is_null() || unsafe { jack_port_flags(source) } & 2 == 0 {
        return Err("expected MIDI source absent".into());
    }
    if unsafe { jack_set_process_callback(client, process, (&mut *rt as *mut Rt).cast()) } != 0
        || unsafe { jack_activate(client) } != 0
    {
        return Err("callback activation failed".into());
    }
    for (src, dst) in [
        (args[1].as_str(), "shieldxl-midi-tone:in"),
        ("shieldxl-midi-tone:left", "system:playback_1"),
        ("shieldxl-midi-tone:right", "system:playback_2"),
    ] {
        if unsafe { jack_connect(client, c(src).as_ptr(), c(dst).as_ptr()) } != 0 {
            return Err("exact routing failed".into());
        }
    }
    signals.armed.store(true, Ordering::Release);
    println!(
        "MIDI_TONE_READY duration_minutes=30 notes=held_tone cc=short_beep startup=three_beeps"
    );
    io::stdout().flush()?;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1800) {
        thread::sleep(Duration::from_secs(1));
        println!(
            "MIDI_TONE_STATUS messages={} note_on={} cc={} errors={}",
            signals.messages.load(Ordering::Relaxed),
            signals.notes.load(Ordering::Relaxed),
            signals.cc.load(Ordering::Relaxed),
            signals.errors.load(Ordering::Relaxed)
        );
    }
    drop(owner);
    println!("MIDI_TONE_STOPPED");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("MIDI_TONE_FAILED {e}");
        std::process::exit(1);
    }
}

use alsa::{
    seq::{EventType, MidiEvent, PortCap, PortType, Seq},
    Direction,
};
use shieldxl_uart_midi::SerialInput;
use std::{
    error::Error,
    fs::OpenOptions,
    io::{self, Read},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

fn run() -> Result<(), Box<dyn Error>> {
    if std::env::args().len() != 1 {
        return Err("no command-line arguments accepted".into());
    }
    let device = "/dev/ttyAMA0";
    let node = std::fs::canonicalize("/sys/class/tty/ttyAMA0/device/of_node")?;
    if !node.ends_with("axi/pcie@1000120000/rp1/serial@30000") {
        return Err("ttyAMA0 is not the expected Pi 5 RP1 header UART0".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(device)?;
    let mut serial = SerialInput::configure(file)?;
    let seq = Seq::open(None, Some(Direction::Playback), true)?;
    seq.set_client_name(c"ShieldXL UART")?;
    let port = seq.create_simple_port(
        c"TRS In",
        PortCap::READ | PortCap::SUBS_READ,
        PortType::MIDI_GENERIC | PortType::HARDWARE,
    )?;
    let mut parser = MidiEvent::new(256)?;
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = stop as *const () as usize;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [libc::SIGTERM, libc::SIGINT] {
            if libc::sigaction(signal, &action, std::ptr::null_mut()) < 0 {
                return Err(io::Error::last_os_error().into());
            }
        }
    }
    println!("SHIELDXL_UART_READY device=ttyAMA0 baud=31250 framing=8N1 input_only=true alsa_client={} alsa_port={} sysex=discarded", seq.client_id()?, port);
    let (mut bytes, mut events, mut sysex_fragments) = (0_u64, 0_u64, 0_u64);
    let mut buffer = [0_u8; 256];
    let mut next_report = Instant::now() + Duration::from_secs(1);
    while !STOP.load(Ordering::Relaxed) {
        let mut descriptor = libc::pollfd {
            fd: serial.file().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let result = unsafe { libc::poll(&mut descriptor, 1, 100) };
        if result < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(io::Error::last_os_error().into());
        }
        if descriptor.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err("UART poll reported a device error".into());
        }
        if descriptor.revents & libc::POLLIN != 0 {
            let count = match serial.file().read(&mut buffer) {
                Ok(0) => return Err("UART returned EOF".into()),
                Ok(count) => count,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    continue
                }
                Err(e) => return Err(e.into()),
            };
            bytes += count as u64;
            let mut offset = 0;
            while offset < count {
                let (consumed, event) = parser.encode(&buffer[offset..count])?;
                if consumed == 0 {
                    return Err("ALSA MIDI parser made no progress".into());
                }
                offset += consumed;
                if let Some(mut event) = event {
                    if event.get_type() == EventType::Sysex {
                        sysex_fragments += 1;
                        continue;
                    }
                    event.set_source(port);
                    event.set_subs();
                    event.set_direct();
                    seq.event_output_direct(&mut event)?;
                    events += 1;
                }
            }
        }
        if Instant::now() >= next_report {
            println!(
                "SHIELDXL_UART_COUNTS bytes={} events={} sysex_fragments_discarded={}",
                bytes, events, sysex_fragments
            );
            next_report = Instant::now() + Duration::from_secs(1);
        }
    }
    // Close the published ALSA input before releasing/restoring the UART.
    drop(seq);
    serial.restore()?;
    println!(
        "SHIELDXL_UART_RETIRED bytes={} events={} sysex_fragments_discarded={}",
        bytes, events, sysex_fragments
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("SHIELDXL_UART_FAILED {error}");
        std::process::exit(1);
    }
}

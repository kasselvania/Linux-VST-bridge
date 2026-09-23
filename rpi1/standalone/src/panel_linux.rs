//! Linux hardware access and a bounded display worker; never called by JACK.
use crate::panel::Model;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::AsRawFd,
        unix::{fs::OpenOptionsExt, net::UnixStream},
    },
    sync::mpsc::{self, SyncSender},
    thread,
    time::{Duration, Instant},
};
struct Device {
    file: File,
    index: usize,
    encoder: bool,
}
pub struct Hardware {
    devices: Vec<Device>,
    display: SyncSender<Vec<String>>,
    last_display: Instant,
}
impl Hardware {
    pub fn open() -> io::Result<Self> {
        let mut devices = Vec::new();
        for encoder in [true, false] {
            for index in 0..3 {
                let kind = if encoder { "encoder" } else { "button" };
                let file = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
                    .open(format!("/dev/input/shieldxl-{kind}-{}", index + 1))?;
                // Only one application owns physical control semantics at a time.
                if unsafe { libc::ioctl(file.as_raw_fd(), 0x40044590u64, 1) } != 0 {
                    return Err(io::Error::last_os_error());
                }
                devices.push(Device {
                    file,
                    index,
                    encoder,
                });
            }
        }
        let (display, receiver) = mpsc::sync_channel::<Vec<String>>(1);
        thread::spawn(move || {
            let mut failed = false;
            while let Ok(lines) = receiver.recv() {
                let result = send_display(&lines);
                if result.is_err() && !failed {
                    eprintln!("RPI2_PANEL_DISPLAY unavailable");
                }
                if result.is_ok() && failed {
                    eprintln!("RPI2_PANEL_DISPLAY recovered");
                }
                failed = result.is_err();
            }
            let _ = send_display(&["AUDIO STOPPED".into()]);
        });
        Ok(Self {
            devices,
            display,
            last_display: Instant::now() - Duration::from_secs(1),
        })
    }
    pub fn poll(&mut self, model: &mut Model) -> io::Result<()> {
        for device in &mut self.devices {
            let mut data = [0u8; 24 * 64];
            let count = match device.file.read(&mut data) {
                Ok(0) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "physical control disappeared",
                    ))
                }
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e),
            };
            if count % 24 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "partial evdev event",
                ));
            }
            for event in data[..count].chunks_exact(24) {
                let kind = u16::from_ne_bytes(event[16..18].try_into().unwrap());
                let code = u16::from_ne_bytes(event[18..20].try_into().unwrap());
                let value = i32::from_ne_bytes(event[20..24].try_into().unwrap());
                if kind == 0 && code == 3 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "evdev input overflow",
                    ));
                }
                if device.encoder && kind == 2 && code == 0 && value != 0 {
                    println!(
                        "RPI2_PANEL_INPUT encoder={} delta={value}",
                        device.index + 1
                    );
                    model.encoder(device.index, value);
                } else if !device.encoder
                    && kind == 1
                    && code == 0x100 + device.index as u16
                    && matches!(value, 0 | 1)
                {
                    println!(
                        "RPI2_PANEL_INPUT button={} pressed={value}",
                        device.index + 1
                    );
                    model.button(device.index, value == 1);
                }
            }
        }
        Ok(())
    }
    pub fn update_display(&mut self, model: &Model) {
        if self.last_display.elapsed() < Duration::from_millis(200) {
            return;
        }
        self.last_display = Instant::now();
        let temperature = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .map(|v| v / 1000.0);
        let _ = self.display.try_send(model.lines(temperature));
    }
}
fn send_display(lines: &[String]) -> io::Result<()> {
    // Nonblocking connect avoids stalling on a full local service backlog.
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    use std::os::fd::FromRawFd;
    let mut stream = unsafe { UnixStream::from_raw_fd(fd) };
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as _;
    let path = b"/run/shieldxl0/oled.sock\0";
    for (slot, byte) in address.sun_path.iter_mut().zip(path) {
        *slot = *byte as _;
    }
    let result = unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&address) as _,
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_millis(150)))?;
    stream.set_write_timeout(Some(Duration::from_millis(150)))?;
    let payload = serde_json::to_vec(&serde_json::json!({"op":"lines","lines":lines}))?;
    stream.write_all(&payload)?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut reply = String::new();
    stream.take(4096).read_to_string(&mut reply)?;
    let reply: serde_json::Value = serde_json::from_str(&reply)?;
    if reply["ok"] != true {
        return Err(io::Error::other("OLED request refused"));
    }
    Ok(())
}

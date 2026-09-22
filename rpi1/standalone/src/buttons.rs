//! ShieldXL preset buttons. Evdev reads and message delivery stay off the JACK callback.
use std::{
    fs::File,
    io::{self, Read},
    os::fd::AsRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread::{self, JoinHandle},
};

const PATHS: [&str; 2] = [
    "/dev/input/shieldxl-button-2",
    "/dev/input/shieldxl-button-3",
];
const CODES: [u16; 2] = [0x101, 0x102];
const EVENT_BYTES: usize = 24;

pub struct Reader {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Reader {
    pub fn start(sender: Sender<String>) -> io::Result<Self> {
        let mut files = [File::open(PATHS[0])?, File::open(PATHS[1])?];
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            let mut descriptors = [
                libc::pollfd {
                    fd: files[0].as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
                libc::pollfd {
                    fd: files[1].as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                },
            ];
            while !stopping.load(Ordering::Acquire) {
                let ready = unsafe { libc::poll(descriptors.as_mut_ptr(), 2, 100) };
                if ready < 0 {
                    if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    let _ = sender.send("button-error".to_owned());
                    break;
                }
                for index in 0..2 {
                    if descriptors[index].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)
                        != 0
                    {
                        let _ = sender.send("button-error".to_owned());
                        return;
                    }
                    if descriptors[index].revents & libc::POLLIN == 0 {
                        continue;
                    }
                    let mut bytes = [0u8; EVENT_BYTES];
                    if files[index].read_exact(&mut bytes).is_err() {
                        let _ = sender.send("button-error".to_owned());
                        return;
                    }
                    if let Some(pressed) = decode(index, &bytes) {
                        let direction = if index == 0 { "prev" } else { "next" };
                        if sender
                            .send(format!("preset-{direction} {}", u8::from(pressed)))
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
        });
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for Reader {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn decode(index: usize, bytes: &[u8; EVENT_BYTES]) -> Option<bool> {
    let kind = u16::from_ne_bytes([bytes[16], bytes[17]]);
    let code = u16::from_ne_bytes([bytes[18], bytes[19]]);
    let value = i32::from_ne_bytes(bytes[20..24].try_into().ok()?);
    if kind == 1 && code == CODES[index] && matches!(value, 0 | 1) {
        Some(value == 1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_button_press_and_release_are_commands() {
        let mut event = [0u8; EVENT_BYTES];
        event[16..18].copy_from_slice(&1u16.to_ne_bytes());
        event[18..20].copy_from_slice(&0x101u16.to_ne_bytes());
        event[20..24].copy_from_slice(&1i32.to_ne_bytes());
        assert_eq!(decode(0, &event), Some(true));
        assert_eq!(decode(1, &event), None);
        event[20..24].copy_from_slice(&0i32.to_ne_bytes());
        assert_eq!(decode(0, &event), Some(false));
        event[20..24].copy_from_slice(&2i32.to_ne_bytes());
        assert_eq!(decode(0, &event), None);
        event[16..18].copy_from_slice(&2u16.to_ne_bytes());
        assert_eq!(decode(0, &event), None);
    }
}

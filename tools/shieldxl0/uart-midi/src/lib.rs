//! ShieldXL input-only UART configuration and ALSA byte-stream parsing.
//! This process does not own audio callbacks, synthesis or plug-in policy.
use std::{fs::File, io, os::fd::AsRawFd};

pub const BAUD: u32 = 31_250;

pub struct SerialInput {
    file: File,
    previous: libc::termios2,
    restored: bool,
}

impl SerialInput {
    pub fn configure(file: File) -> io::Result<Self> {
        let fd = file.as_raw_fd();
        let mut previous: libc::termios2 = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(fd, libc::TCGETS2, &mut previous) } < 0 {
            return Err(io::Error::last_os_error());
        }
        if unsafe { libc::ioctl(fd, libc::TIOCEXCL) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let owner = Self {
            file,
            previous,
            restored: false,
        };
        let mut settings: libc::termios2 = unsafe { std::mem::zeroed() };
        settings.c_cflag = libc::BOTHER | libc::CS8 | libc::CLOCAL | libc::CREAD;
        settings.c_ispeed = BAUD;
        settings.c_ospeed = BAUD;
        settings.c_cc[libc::VMIN] = 1;
        settings.c_cc[libc::VTIME] = 0;
        if unsafe { libc::ioctl(fd, libc::TCSETS2, &settings) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut actual: libc::termios2 = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(fd, libc::TCGETS2, &mut actual) } < 0 {
            return Err(io::Error::last_os_error());
        }
        if actual.c_ispeed != BAUD
            || actual.c_ospeed != BAUD
            || actual.c_cflag != settings.c_cflag
            || actual.c_iflag != 0
            || actual.c_oflag != 0
            || actual.c_lflag != 0
        {
            return Err(io::Error::other("UART 31250 8N1 raw readback differs"));
        }
        if unsafe { libc::tcflush(fd, libc::TCIFLUSH) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(owner)
    }
    pub fn file(&mut self) -> &mut File {
        &mut self.file
    }
    pub fn restore(mut self) -> io::Result<()> {
        self.restore_inner()
    }
    fn restore_inner(&mut self) -> io::Result<()> {
        let fd = self.file.as_raw_fd();
        if unsafe { libc::ioctl(fd, libc::TCSETS2, &self.previous) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut actual: libc::termios2 = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(fd, libc::TCGETS2, &mut actual) } < 0 {
            return Err(io::Error::last_os_error());
        }
        if actual.c_cflag != self.previous.c_cflag
            || actual.c_iflag != self.previous.c_iflag
            || actual.c_oflag != self.previous.c_oflag
            || actual.c_lflag != self.previous.c_lflag
            || actual.c_ispeed != self.previous.c_ispeed
            || actual.c_ospeed != self.previous.c_ospeed
            || actual.c_cc != self.previous.c_cc
        {
            return Err(io::Error::other("UART restoration readback differs"));
        }
        if unsafe { libc::ioctl(fd, libc::TIOCNXCL) } < 0 {
            return Err(io::Error::last_os_error());
        }
        self.restored = true;
        Ok(())
    }
}

impl Drop for SerialInput {
    fn drop(&mut self) {
        if !self.restored {
            if let Err(error) = self.restore_inner() {
                eprintln!("SHIELDXL_UART_RESTORE_FAILED {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alsa::seq::{EvNote, EventType, MidiEvent};
    use std::os::fd::FromRawFd;

    #[test]
    fn running_status_and_interleaved_clock_preserve_notes() {
        let mut parser = MidiEvent::new(256).unwrap();
        let mut found = Vec::new();
        // Each byte can arrive in a separate serial read. Clock must not cancel
        // the in-progress note or the running channel status.
        for byte in [0x90, 60, 0xf8, 96, 61, 100, 0x80, 60, 0, 61, 0] {
            let (consumed, event) = parser.encode(&[byte]).unwrap();
            assert_eq!(consumed, 1);
            if let Some(event) = event {
                found.push((event.get_type(), event.get_data::<EvNote>()));
            }
        }
        assert_eq!(found[0].0, EventType::Clock);
        for (index, kind, note, velocity) in [
            (1, EventType::Noteon, 60, 96),
            (2, EventType::Noteon, 61, 100),
            (3, EventType::Noteoff, 60, 0),
            (4, EventType::Noteoff, 61, 0),
        ] {
            assert_eq!(found[index].0, kind);
            let n = found[index].1.unwrap();
            assert_eq!((n.channel, n.note, n.velocity), (0, note, velocity));
        }
        assert_eq!(found.len(), 5);
    }

    #[test]
    fn uart_custom_speed_is_read_back_and_restored_on_retirement() {
        let (mut master, mut slave) = (0, 0);
        assert_eq!(
            unsafe {
                libc::openpty(
                    &mut master,
                    &mut slave,
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    std::ptr::null(),
                )
            },
            0
        );
        let _master = unsafe { File::from_raw_fd(master) };
        let slave_file = unsafe { File::from_raw_fd(slave) };
        let retained = slave_file.try_clone().unwrap();
        let mut before: libc::termios2 = unsafe { std::mem::zeroed() };
        assert_eq!(unsafe { libc::ioctl(slave, libc::TCGETS2, &mut before) }, 0);
        let serial = SerialInput::configure(slave_file).unwrap();
        serial.restore().unwrap();
        let mut after: libc::termios2 = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { libc::ioctl(retained.as_raw_fd(), libc::TCGETS2, &mut after) },
            0
        );
        assert_eq!(
            (after.c_cflag, after.c_ispeed, after.c_ospeed),
            (before.c_cflag, before.c_ispeed, before.c_ospeed)
        );
    }
}

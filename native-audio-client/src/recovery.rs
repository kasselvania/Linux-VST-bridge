//! Non-audio proxy control. Fixed metadata only; vendor state never crosses it.
use crate::{invalid, need};
use std::io::{self, Read, Write};
/// One absolute frame budget, including short reads/writes. A peer cannot keep
/// a control worker alive indefinitely by dripping a byte per socket timeout.
#[cfg(unix)]
pub struct SocketDeadline<'a> {
    stream: &'a mut std::os::unix::net::UnixStream,
    end: std::time::Instant,
}
#[cfg(unix)]
impl<'a> SocketDeadline<'a> {
    pub fn new(stream: &'a mut std::os::unix::net::UnixStream, limit: std::time::Duration) -> Self {
        Self {
            stream,
            end: std::time::Instant::now() + limit,
        }
    }
    fn remaining(&self) -> io::Result<std::time::Duration> {
        self.end
            .checked_duration_since(std::time::Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "proxy control frame deadline"))
    }
}
#[cfg(unix)]
impl Read for SocketDeadline<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(bytes)
    }
}
#[cfg(unix)]
impl Write for SocketDeadline<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
pub const GREETING: &[u8; 5] = b"LVR1\n";
pub const FRAME_BYTES: usize = 96;
pub const COMMAND_BYTES: usize = 64;
pub const FAILED: u32 = 1;
pub const RETIRED: u32 = 2;
pub const OWNER_TIMER: u32 = 4;
pub const SNAPSHOT: u32 = 8;
pub const LATER_EDITS: u32 = 16;
pub const IDLE: u32 = 0;
pub const ACCEPTED: u32 = 1;
pub const RESTORING: u32 = 2;
pub const CONTROLLER: u32 = 3;
pub const COMPLETE: u32 = 4;
pub const REFUSED: u32 = 5;
pub const UNCERTAIN: u32 = 6;
/// Includes existing containment80 + admission65 + host180 + state/config20
/// + activation40 + slot20 + controller10 + failed-startup cleanup60 seconds.
pub const RECOVERY_SECONDS: u64 = 480;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Status {
    pub session: [u8; 16],
    pub generation: u64,
    pub revision: u64,
    pub digest: [u8; 32],
    pub flags: u32,
    pub operation: [u8; 16],
    pub phase: u32,
    pub reason: u32,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Command {
    pub operation: [u8; 16],
    pub generation: u64,
    pub revision: u64,
    pub digest: [u8; 32],
}
impl Status {
    pub fn validate(&self) -> io::Result<()> {
        need(
            self.session != [0; 16]
                && self.generation != 0
                && self.flags & !31 == 0
                && self.phase <= UNCERTAIN
                && self.reason <= 9,
            "proxy recovery status",
        )?;
        need(
            (self.flags & SNAPSHOT != 0) == (self.revision != 0 && self.digest != [0; 32]),
            "proxy snapshot extent",
        )?;
        need(
            (self.phase == IDLE) == (self.operation == [0; 16]),
            "proxy command identity",
        )
    }
    pub fn ready(&self) -> bool {
        self.flags & (FAILED | RETIRED | OWNER_TIMER | SNAPSHOT)
            == FAILED | RETIRED | OWNER_TIMER | SNAPSHOT
            && matches!(self.phase, IDLE | COMPLETE | REFUSED)
    }
    pub fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.validate()?;
        let mut bytes = [0; FRAME_BYTES];
        bytes[..16].copy_from_slice(&self.session);
        bytes[16..24].copy_from_slice(&self.generation.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.revision.to_le_bytes());
        bytes[32..64].copy_from_slice(&self.digest);
        bytes[64..68].copy_from_slice(&self.flags.to_le_bytes());
        bytes[68..84].copy_from_slice(&self.operation);
        bytes[84..88].copy_from_slice(&self.phase.to_le_bytes());
        bytes[88..92].copy_from_slice(&self.reason.to_le_bytes());
        writer.write_all(&bytes)
    }
    pub fn read(reader: &mut impl Read) -> io::Result<Self> {
        let mut bytes = [0; FRAME_BYTES];
        reader.read_exact(&mut bytes)?;
        need(bytes[92..] == [0; 4], "proxy recovery reserved bytes")?;
        let value = Self {
            session: bytes[..16].try_into().unwrap(),
            generation: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            revision: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            digest: bytes[32..64].try_into().unwrap(),
            flags: u32::from_le_bytes(bytes[64..68].try_into().unwrap()),
            operation: bytes[68..84].try_into().unwrap(),
            phase: u32::from_le_bytes(bytes[84..88].try_into().unwrap()),
            reason: u32::from_le_bytes(bytes[88..92].try_into().unwrap()),
        };
        value.validate()?;
        Ok(value)
    }
}
impl Command {
    pub fn validate(&self) -> io::Result<()> {
        need(
            *self == Self::default()
                || self.operation != [0; 16]
                    && (1..u64::MAX).contains(&self.generation)
                    && self.revision != 0
                    && self.digest != [0; 32],
            "proxy restart identity",
        )
    }
    pub fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        self.validate()?;
        let mut bytes = [0; COMMAND_BYTES];
        bytes[..16].copy_from_slice(&self.operation);
        bytes[16..24].copy_from_slice(&self.generation.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.revision.to_le_bytes());
        bytes[32..].copy_from_slice(&self.digest);
        writer.write_all(&bytes)
    }
    pub fn read(reader: &mut impl Read) -> io::Result<Self> {
        let mut bytes = [0; COMMAND_BYTES];
        reader.read_exact(&mut bytes)?;
        let value = Self {
            operation: bytes[..16].try_into().unwrap(),
            generation: u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
            revision: u64::from_le_bytes(bytes[24..32].try_into().unwrap()),
            digest: bytes[32..].try_into().unwrap(),
        };
        value.validate()?;
        Ok(value)
    }
    pub fn matches(&self, status: &Status) -> bool {
        self.operation != [0; 16]
            && self.generation == status.generation
            && self.revision == status.revision
            && self.digest == status.digest
    }
}
pub fn reason(code: u32) -> &'static str {
    match code {
        0 => "",
        1 => "The confirmed snapshot or instance changed",
        2 => "The DAW owner thread did not accept the restart",
        3 => "The original host has not confirmed retirement",
        4 => "The exact original setup or runtime is no longer available",
        5 => "The plug-in state could not be restored",
        6 => "The DAW controller did not confirm the restored state",
        7 => "The restored plug-in did not acknowledge activation",
        9 => "The DAW changed this instance's active or processing state during restart",
        _ => "The restart outcome is unconfirmed; it will not be replayed",
    }
}
pub fn decode_id(value: &str) -> io::Result<[u8; 16]> {
    need(
        value.len() == 32 && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "proxy instance identity",
    )?;
    let mut id = [0; 16];
    for (i, b) in id.iter_mut().enumerate() {
        *b = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)
            .map_err(|_| invalid("proxy identity"))?;
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_metadata_and_commands_roundtrip_without_state_payloads() {
        let status = Status {
            session: [1; 16],
            generation: 3,
            revision: 7,
            digest: [8; 32],
            flags: FAILED | RETIRED | OWNER_TIMER | SNAPSHOT | LATER_EDITS,
            ..Status::default()
        };
        let command = Command {
            operation: [2; 16],
            generation: 3,
            revision: 7,
            digest: [8; 32],
        };
        let mut bytes = vec![];
        status.write(&mut bytes).unwrap();
        assert_eq!(bytes.len(), FRAME_BYTES);
        assert_eq!(Status::read(&mut bytes.as_slice()).unwrap(), status);
        assert!(status.ready());
        assert!(command.matches(&status));
        let mut changed = status;
        changed.digest[0] ^= 1;
        assert!(!command.matches(&changed));
        bytes[92] = 1;
        assert!(Status::read(&mut bytes.as_slice()).is_err());
        let mut bytes = vec![];
        command.write(&mut bytes).unwrap();
        assert_eq!(bytes.len(), COMMAND_BYTES);
        assert_eq!(Command::read(&mut bytes.as_slice()).unwrap(), command);
        assert!(Command::read(&mut &bytes[..63]).is_err());
        assert!(Command {
            revision: 0,
            ..command
        }
        .validate()
        .is_err());
        assert!(Status {
            flags: status.flags | 32,
            ..status
        }
        .validate()
        .is_err());
    }
}

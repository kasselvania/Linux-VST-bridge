//! Optional display observation, separate from launch and audio authority.
use crate::{invalid, need};
use std::io::{self, Read};
pub const GREETING: &[u8; 5] = b"LVH1\n";
pub const MAX_NAME: usize = 512;
pub const HEADER: usize = 18;

pub fn validate_name(name: &str) -> io::Result<()> {
    need(!name.trim().is_empty() && name.len() <= MAX_NAME
        && !name.chars().any(char::is_control), "host_name_invalid")
}
pub fn encode(session: [u8; 16], name: &str) -> io::Result<Vec<u8>> {
    need(session != [0; 16], "host_session_invalid")?;
    validate_name(name)?;
    let mut bytes = Vec::with_capacity(GREETING.len() + HEADER + name.len());
    bytes.extend_from_slice(GREETING);
    bytes.extend_from_slice(&session);
    bytes.extend_from_slice(&(name.len() as u16).to_le_bytes());
    bytes.extend_from_slice(name.as_bytes());
    Ok(bytes)
}
/// The owner consumed the greeting and supplies a deadline-bounded reader.
pub fn read(reader: &mut impl Read) -> io::Result<([u8; 16], String)> {
    let mut header = [0; HEADER]; reader.read_exact(&mut header)?;
    let session = header[..16].try_into().unwrap();
    need(session != [0; 16], "host_session_invalid")?;
    let length = u16::from_le_bytes(header[16..].try_into().unwrap()) as usize;
    need((1..=MAX_NAME).contains(&length), "host_name_bound")?;
    let mut bytes = vec![0; length]; reader.read_exact(&mut bytes)?;
    let name = String::from_utf8(bytes).map_err(|_| invalid("host_name_encoding"))?;
    validate_name(&name)?;
    Ok((session, name))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_report_is_exact_bounded_text_never_a_path_or_process_guess() {
        for name in ["REAPER", "Bitwig Studio", "Unfamiliar host α"] {
            let bytes = encode([7; 16], name).unwrap();
            assert_eq!(read(&mut &bytes[5..]).unwrap(), ([7; 16], name.into()));
        }
        for name in ["", " ", "A\nB", "A\0B"] { assert!(encode([7; 16], name).is_err()); }
        assert!(encode([0; 16], "Host").is_err());
        assert!(encode([7; 16], &"x".repeat(MAX_NAME + 1)).is_err());
        let mut bytes = encode([7; 16], "host").unwrap();
        bytes[21..23].copy_from_slice(&((MAX_NAME + 1) as u16).to_le_bytes());
        assert!(read(&mut &bytes[5..]).is_err(), "bound checked before payload read");
        let mut bytes = encode([7; 16], "host").unwrap(); bytes[23] = 255;
        assert!(read(&mut &bytes[5..]).is_err());
        assert!(read(&mut &encode([7; 16], "host").unwrap()[5..24]).is_err());
    }
}

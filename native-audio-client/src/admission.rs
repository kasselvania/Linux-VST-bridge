//! LVB3 manager admission, separate from the audio protocol and GUI ABI.
//! Used only during non-RT startup. A refusal never conveys session ownership.
use crate::{invalid, need};
use std::io;

pub const GREETING: &[u8; 5] = b"LVB3\n";
pub const GREETING_V4: &[u8; 5] = b"LVB4\n";
pub const MAX_REPLY: usize = 1024;
pub const HEADER: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Refusal {
    ServiceBusy = 1,
    GlobalCapacity = 2,
    ClassCapacity = 3,
    NativeImageCapacity = 4,
    CleanupUnconfirmed = 5,
    MaintenanceActive = 6,
    BindingInvalid = 7,
}
impl Refusal {
    pub fn code(self) -> &'static str {
        match self {
            Self::ServiceBusy => "admission_service_busy",
            Self::GlobalCapacity => "admission_global_capacity",
            Self::ClassCapacity => "admission_class_capacity",
            Self::NativeImageCapacity => "admission_native_image_capacity",
            Self::CleanupUnconfirmed => "admission_cleanup_unconfirmed",
            Self::MaintenanceActive => "admission_maintenance_active",
            Self::BindingInvalid => "admission_binding_invalid",
        }
    }
    fn decode(value: u8) -> io::Result<Self> {
        match value {
            1 => Ok(Self::ServiceBusy),
            2 => Ok(Self::GlobalCapacity),
            3 => Ok(Self::ClassCapacity),
            4 => Ok(Self::NativeImageCapacity),
            5 => Ok(Self::CleanupUnconfirmed),
            6 => Ok(Self::MaintenanceActive),
            7 => Ok(Self::BindingInvalid),
            _ => Err(invalid("admission refusal encoding")),
        }
    }
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Refusal {}

#[derive(Debug, PartialEq, Eq)]
pub struct Binding {
    pub session: [u8; 16],
    pub added_frames: u32,
    pub delivery_mode: u32,
    pub directory: String,
}
fn header(request: [u8; 16], code: u8) -> Vec<u8> {
    let mut bytes = vec![0; HEADER];
    bytes[..4].copy_from_slice(b"LVR3");
    bytes[4] = code;
    bytes[8..24].copy_from_slice(&request);
    bytes
}
pub fn refused(request: [u8; 16], reason: Refusal) -> Vec<u8> {
    header(request, reason as u8)
}
pub fn accepted(request: [u8; 16], binding: &Binding) -> io::Result<Vec<u8>> {
    need(binding.delivery_mode == 0, "legacy admission supports buffered delivery only")?;
    encode(request, binding, false)
}
pub fn accepted_v4(request: [u8; 16], binding: &Binding) -> io::Result<Vec<u8>> {
    encode(request, binding, true)
}
pub fn refused_v4(request: [u8; 16], reason: Refusal) -> Vec<u8> {
    let mut bytes = refused(request, reason); bytes[3] = b'4'; bytes
}
fn encode(request: [u8; 16], binding: &Binding, version4: bool) -> io::Result<Vec<u8>> {
    need(binding.delivery_mode <= 1, "admission delivery mode")?;
    need(request != [0; 16], "admission request identity")?;
    need(binding.session != [0; 16], "admission session identity")?;
    need(matches!(binding.added_frames, 256 | 512 | 1024), "admission delay")?;
    need(
        binding.directory.starts_with('/')
            && !binding.directory.bytes().any(|b| b == 0 || b == b'\n')
            && HEADER + 20 + if version4 {4} else {0} + binding.directory.len() <= MAX_REPLY,
        "admission directory extent",
    )?;
    let mut bytes = header(request, 0);
    if version4 { bytes[3] = b'4'; }
    bytes.extend(binding.session);
    bytes.extend(binding.added_frames.to_le_bytes());
    if version4 { bytes.extend(binding.delivery_mode.to_le_bytes()); }
    bytes.extend(binding.directory.as_bytes());
    Ok(bytes)
}
pub fn decode(bytes: &[u8], request: [u8; 16]) -> io::Result<Result<Binding, Refusal>> {
    decode_version(bytes, request, false)
}
pub fn decode_v4(bytes: &[u8], request: [u8; 16]) -> io::Result<Result<Binding, Refusal>> {
    // The unclassified bounded-service refusal precedes reading any greeting.
    if bytes == refused([0; 16], Refusal::ServiceBusy) { return decode(bytes, request); }
    decode_version(bytes, request, true)
}
fn decode_version(bytes: &[u8], request: [u8; 16], version4: bool) -> io::Result<Result<Binding, Refusal>> {
    need(request != [0; 16], "admission request identity")?;
    need(
        (HEADER..=MAX_REPLY).contains(&bytes.len()),
        "admission reply extent",
    )?;
    need(
        &bytes[..4] == if version4 { b"LVR4" } else { b"LVR3" } && bytes[5..8] == [0; 3],
        "admission reply version",
    )?;
    // At the bounded connection ceiling the service can refuse classification
    // itself. This zero-token Busy result grants no ownership and acknowledges
    // no request. Every classified result, including other refusals, is exact.
    let busy = bytes[4] == Refusal::ServiceBusy as u8 && bytes[8..24] == [0; 16];
    need(busy || bytes[8..24] == request, "admission stale reply")?;
    if bytes[4] != 0 {
        need(bytes.len() == HEADER, "admission refusal payload")?;
        return Ok(Err(Refusal::decode(bytes[4])?));
    }
    let path_offset = if version4 { 48 } else { 44 };
    need(bytes.len() > path_offset, "admission binding extent")?;
    let binding = Binding {
        session: bytes[24..40].try_into().unwrap(),
        added_frames: u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
        delivery_mode: if version4 { u32::from_le_bytes(bytes[44..48].try_into().unwrap()) } else { 0 },
        directory: std::str::from_utf8(&bytes[path_offset..])
            .map_err(|_| invalid("admission directory encoding"))?
            .into(),
    };
    // One validator owns the accepted shape on both ends.
    need(
        encode(request, &binding, version4)? == bytes,
        "admission binding encoding",
    )?;
    Ok(Ok(binding))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_binding_and_refusals_never_overlap() {
        let request = [1; 16];
        for added_frames in [256, 512, 1024] {
            let binding = Binding {session:[2;16],added_frames,delivery_mode:0,directory:"/owned/session".into()};
            let bytes = accepted(request, &binding).unwrap();
            assert_eq!(decode(&bytes, request).unwrap(), Ok(binding));
        }
        let binding = Binding {
            session: [2; 16],
            added_frames: 512,
            delivery_mode: 0,
            directory: "/owned/session".into(),
        };
        let bytes = accepted(request, &binding).unwrap();
        assert_eq!(decode(&bytes, request).unwrap(), Ok(binding));
        for reason in [
            Refusal::ServiceBusy,
            Refusal::GlobalCapacity,
            Refusal::ClassCapacity,
            Refusal::NativeImageCapacity,
            Refusal::CleanupUnconfirmed,
            Refusal::MaintenanceActive,
            Refusal::BindingInvalid,
        ] {
            assert_eq!(
                decode(&refused(request, reason), request).unwrap(),
                Err(reason)
            );
            assert!(decode(&refused(request, reason), [3; 16]).is_err());
        }
        assert_eq!(
            decode(&refused([0; 16], Refusal::ServiceBusy), request).unwrap(),
            Err(Refusal::ServiceBusy)
        );
        assert!(decode(&refused([0; 16], Refusal::GlobalCapacity), request).is_err());
    }
    #[test]
    fn explicit_delivery_preserves_remembered_buffer_and_legacy_cannot_accept_it() {
        let request = [1; 16];
        for mode in [0, 1] {
            let binding = Binding {session:[2;16],added_frames:1024,delivery_mode:mode,directory:"/owned/session".into()};
            let bytes = accepted_v4(request, &binding).unwrap();
            assert!(decode(&bytes, request).is_err());
            assert_eq!(decode_v4(&bytes, request).unwrap(), Ok(binding));
        }
        let binding = Binding {session:[2;16],added_frames:1024,delivery_mode:1,directory:"/owned/session".into()};
        assert!(accepted(request, &binding).is_err());
        let mut bytes = accepted_v4(request, &binding).unwrap();
        bytes[44] = 2;
        assert!(decode_v4(&bytes, request).is_err());
        assert_eq!(decode_v4(&refused([0;16], Refusal::ServiceBusy), request).unwrap(), Err(Refusal::ServiceBusy));
        assert!(decode_v4(&refused(request, Refusal::BindingInvalid), request).is_err());
    }
    #[test]
    fn malformed_stale_and_unsupported_messages_fail_closed() {
        let request = [1; 16];
        let binding = Binding {
            session: [2; 16],
            added_frames: 512,
            delivery_mode: 0,
            directory: "/owned/session".into(),
        };
        let bytes = accepted(request, &binding).unwrap();
        assert!(decode(&bytes, [2; 16]).is_err());
        assert!(decode(&bytes, [0; 16]).is_err());
        for length in 0..45 {
            assert!(decode(&bytes[..length.min(bytes.len())], request).is_err());
        }
        for offset in [0, 3, 4, 5, 8, 40, 44] {
            let mut corrupt = bytes.clone();
            corrupt[offset] = 255;
            assert!(decode(&corrupt, request).is_err(), "offset {offset}");
        }
        let mut extra = refused(request, Refusal::ClassCapacity);
        extra.push(0);
        assert!(decode(&extra, request).is_err());
        let mut zero = bytes.clone();
        zero[24..40].fill(0);
        assert!(decode(&zero, request).is_err());
        assert!(decode(&vec![0; MAX_REPLY + 1], request).is_err());
    }
}

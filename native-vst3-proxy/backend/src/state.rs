//! Durable state binds only logical processor/module/content, never a session.
use crate::*;
use sha2::{Digest, Sha256};
pub const LIMIT: usize = 1 << 20;
pub const HEADER_SIZE: usize = 104;
const CLASS: [u8; 16] = [
    0x84, 0xe8, 0xde, 0x5f, 0x92, 0x55, 0x4f, 0x53, 0x96, 0xfa, 0xe4, 0x13, 0x3c, 0x93, 0x5a, 0x18,
];
const MODULE: [u8; 32] = [
    0x60, 0xaa, 0x9f, 0xf6, 0xb9, 0x91, 0x8d, 0x43, 0x30, 0x44, 0x9e, 0x7b, 0x3a, 0xb3, 0x4b, 0x58,
    0x8d, 0xd9, 0x3c, 0xba, 0x09, 0xf3, 0x74, 0x13, 0xa3, 0xcd, 0x91, 0xf6, 0xe7, 0xd2, 0xe1, 0x8f,
];
// This is reference-specific validation, not the transport's state model.
pub fn reference(payload: &[u8]) -> io::Result<f64> {
    need(payload.len() == 12, "unsupported AGain state extent")?;
    let gain = f32::from_le_bytes(payload[0..4].try_into().unwrap());
    let reduction = f32::from_le_bytes(payload[4..8].try_into().unwrap());
    let bypass = u32::from_le_bytes(payload[8..12].try_into().unwrap());
    need(
        gain.is_finite()
            && (0.0..=1.0).contains(&gain)
            && reduction.is_finite()
            && (0.0..=1.0).contains(&reduction)
            && bypass <= 1,
        "unsupported AGain state values",
    )?;
    Ok(gain as f64)
}
pub fn envelope(payload: &[u8]) -> io::Result<Vec<u8>> {
    need(payload.len() <= LIMIT, "component state exceeds cap")?;
    reference(payload)?;
    let mut blob = vec![0; HEADER_SIZE];
    blob[..8].copy_from_slice(b"LVBSTATE");
    blob[8..12].copy_from_slice(&1u32.to_le_bytes());
    blob[12..16].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
    blob[16..32].copy_from_slice(&CLASS);
    blob[32..64].copy_from_slice(&MODULE);
    blob[64..68].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    blob[72..104].copy_from_slice(&Sha256::digest(payload));
    blob.extend_from_slice(payload);
    Ok(blob)
}
pub fn payload(blob: &[u8]) -> io::Result<&[u8]> {
    need(
        blob.len() >= HEADER_SIZE && blob.len() <= HEADER_SIZE + LIMIT,
        "state envelope extent",
    )?;
    need(
        &blob[..8] == b"LVBSTATE"
            && get(&blob[8..12]) == 1
            && get(&blob[12..16]) == HEADER_SIZE as u64,
        "state envelope version",
    )?;
    need(
        blob[16..32] == CLASS && blob[32..64] == MODULE,
        "state class/module mismatch",
    )?;
    need(
        get(&blob[64..68]) as usize == blob.len() - HEADER_SIZE && get(&blob[68..72]) == 0,
        "state length/reserved",
    )?;
    let p = &blob[HEADER_SIZE..];
    need(
        Sha256::digest(p).as_slice() == &blob[72..104],
        "state integrity mismatch",
    )?;
    reference(p)?;
    Ok(p)
}
#[no_mangle]
pub unsafe extern "C" fn ap4_validate(blob: *const u8, n: u32, gain: *mut f64) -> u32 {
    ffi(|| {
        if blob.is_null() || gain.is_null() || n as usize > LIMIT + HEADER_SIZE {
            return 1;
        }
        match payload(std::slice::from_raw_parts(blob, n as usize)).and_then(reference) {
            Ok(g) => {
                *gain = g;
                0
            }
            Err(e) => retain(&e),
        }
    }) as u32
}
impl Session {
    pub fn component_state(&mut self, restore: Option<&[u8]>) -> io::Result<Vec<u8>> {
        need(
            self.minor == 4
                && self.phase != ERROR
                && self.state.slot == Slot::Writable
                && (restore.is_none() || self.phase != 11),
            "state lifecycle/ownership",
        )?;
        let kind = if restore.is_some() { 18 } else { 16 };
        let request = Frame {
            kind,
            session: self.state.session,
            sequence: self.state.next,
            payload: restore.unwrap_or(&[]).to_vec(),
        };
        let result = (|| {
            send_version(&mut self.socket, &request, 5, 4)?;
            let reply = receive_version(&mut self.socket, 10, 4)?;
            need(
                reply.kind == kind + 1
                    && reply.session == request.session
                    && reply.sequence == request.sequence,
                "state response correlation",
            )?;
            need(reply.payload.len() <= LIMIT, "state response cap")?;
            if let Some(p) = restore {
                need(
                    reply.payload == p,
                    "restored Windows state readback differs",
                )?;
            }
            self.state.next = self
                .state
                .next
                .checked_add(1)
                .ok_or_else(|| invalid("sequence exhausted"))?;
            Ok(reply.payload)
        })();
        if result.is_err() {
            self.phase = ERROR;
            self.state.failed();
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_reference_payload_and_identity() {
        let p = [
            0.25f32.to_le_bytes(),
            0.125f32.to_le_bytes(),
            1u32.to_le_bytes(),
        ]
        .concat();
        let b = envelope(&p).unwrap();
        assert_eq!(payload(&b).unwrap(), p);
        assert_eq!(reference(&p).unwrap(), 0.25);
        for i in [0, 8, 12, 16, 32, 64, 68, 72, 104, 115] {
            let mut bad = b.clone();
            bad[i] ^= 1;
            assert!(payload(&bad).is_err());
        }
        for n in 0..b.len() {
            assert!(payload(&b[..n]).is_err());
        }
        assert!(payload(&[0; HEADER_SIZE + LIMIT + 1]).is_err());
    }
    #[test]
    fn nonfinite_and_unsupported_values_refused() {
        for g in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(
                reference(&[g.to_le_bytes(), 0f32.to_le_bytes(), 0u32.to_le_bytes()].concat())
                    .is_err()
            );
        }
        assert!(
            reference(&[0f32.to_le_bytes(), 0f32.to_le_bytes(), 2u32.to_le_bytes()].concat())
                .is_err()
        );
    }
}

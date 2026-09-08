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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Identity {
    pub class: [u8; 16],
    pub module: [u8; 32],
}
const REFERENCE: Identity = Identity {
    class: CLASS,
    module: MODULE,
};
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
    reference(payload)?;
    envelope_for(REFERENCE, 1, payload)
}
pub fn envelope_for(identity: Identity, version: u32, payload: &[u8]) -> io::Result<Vec<u8>> {
    need(matches!(version, 1 | 2), "state envelope version")?;
    need(payload.len() <= LIMIT, "component state exceeds cap")?;
    let mut blob = vec![0; HEADER_SIZE];
    blob[..8].copy_from_slice(b"LVBSTATE");
    blob[8..12].copy_from_slice(&version.to_le_bytes());
    blob[12..16].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
    blob[16..32].copy_from_slice(&identity.class);
    blob[32..64].copy_from_slice(&identity.module);
    blob[64..68].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    blob[72..104].copy_from_slice(&Sha256::digest(payload));
    blob.extend_from_slice(payload);
    Ok(blob)
}
pub fn payload(blob: &[u8]) -> io::Result<&[u8]> {
    let p = payload_for(REFERENCE, 1, blob)?;
    reference(p)?;
    Ok(p)
}
pub fn payload_for(identity: Identity, version: u32, blob: &[u8]) -> io::Result<&[u8]> {
    need(
        blob.len() >= HEADER_SIZE && blob.len() <= HEADER_SIZE + LIMIT,
        "state envelope extent",
    )?;
    need(
        &blob[..8] == b"LVBSTATE"
            && get(&blob[8..12]) == u64::from(version)
            && get(&blob[12..16]) == HEADER_SIZE as u64,
        "state envelope version",
    )?;
    need(
        blob[16..32] == identity.class && blob[32..64] == identity.module,
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
            self.minor >= 4
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
            send_version(&mut self.socket, &request, 5, self.minor)?;
            let reply = receive_version(&mut self.socket, 10, self.minor)?;
            need(
                reply.kind == kind + 1
                    && reply.session == request.session
                    && reply.sequence == request.sequence,
                "state response correlation",
            )?;
            need(reply.payload.len() <= LIMIT, "state response cap")?;
            if let Some(p) = restore.filter(|_| self.minor == 4) {
                need(
                    reply.payload == p,
                    "restored Windows state readback differs",
                )?;
            }
            if self.minor == 4 {
                reference(&reply.payload)?;
            } else {
                commercial_payload(&reply.payload)?;
            }
            if let Some(w) = &mut self.witness {
                w.state(&reply.payload, restore.is_some());
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
// Optional fixture observer on its independent consumer thread. It never writes an audio
// buffer or supplies/restores a parameter. Actual snapshots seed its reference;
// actual returned samples are compared independently, including hidden fields.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct WitnessReport {
    pub samples: u64,
    pub restored_samples: u64,
    pub before_edit_samples: u64,
    pub restores: u64,
    pub edits: u64,
    pub nonzero_samples: u64,
    pub maximum_error: f64,
    pub restored_gain: f64,
}
pub struct Witness {
    pub report: WitnessReport,
    pub ready: bool,
    pub input_hash: u64,
    pub output_hash: u64,
    gain: f32,
    reduction: f32,
    bypass: bool,
    edited: bool,
}
impl Witness {
    pub fn new() -> Self {
        Self {
            report: WitnessReport::default(),
            ready: false,
            input_hash: 14695981039346656037,
            output_hash: 14695981039346656037,
            gain: 0.,
            reduction: 0.,
            bypass: false,
            edited: false,
        }
    }
    pub fn state(&mut self, p: &[u8], restore: bool) -> io::Result<()> {
        reference(p)?;
        self.gain = f32::from_le_bytes(p[..4].try_into().unwrap());
        self.reduction = f32::from_le_bytes(p[4..8].try_into().unwrap());
        self.bypass = get(&p[8..]) != 0;
        self.ready = true;
        if restore {
            self.report.restores += 1;
            self.report.restored_gain = self.gain as f64;
            self.edited = false;
        }
        Ok(())
    }
    pub fn compare(
        &mut self,
        n: usize,
        gain: f64,
        input: [&[f32]; 2],
        output: &[[u32; CAP + 2]; 2],
    ) -> io::Result<()> {
        need(
            self.ready,
            "fixture comparison requires actual state readback",
        )?;
        if !gain.is_nan() {
            if self.gain != gain as f32 {
                self.edited = true;
                self.report.edits += 1;
            }
            self.gain = gain as f32;
        }
        let factor = if self.bypass {
            1.
        } else {
            let f = self.gain - self.reduction;
            if f < 0.0000001 {
                0.
            } else {
                f
            }
        };
        for ch in 0..2 {
            for i in 0..n {
                let expected = input[ch][i] * factor;
                let actual = f32::from_bits(output[ch][i + 1]);
                self.report.maximum_error = self
                    .report
                    .maximum_error
                    .max((actual as f64 - expected as f64).abs());
                // Per-instance routing fingerprints, only on the observation consumer.
                for (hash, value) in [
                    (&mut self.input_hash, input[ch][i]),
                    (&mut self.output_hash, actual),
                ] {
                    for byte in (if value == 0. { 0f32 } else { value })
                        .to_bits()
                        .to_le_bytes()
                    {
                        *hash = (*hash ^ u64::from(byte)).wrapping_mul(1099511628211);
                    }
                }
                self.report.samples += 1;
                self.report.nonzero_samples += u64::from(actual != 0.);
                if self.report.restores > 0 {
                    self.report.restored_samples += 1;
                    self.report.before_edit_samples += u64::from(!self.edited);
                }
            }
        }
        Ok(())
    }
}
pub fn commercial_payload(p: &[u8]) -> io::Result<()> {
    need(p.len() >= 16, "commercial state header")?;
    let a = get(&p[..4]) as usize;
    let b = get(&p[4..8]) as usize;
    let n = get(&p[8..12]) as usize;
    let flags = get(&p[12..16]);
    need(
        flags <= 1
            && n <= 8192
            && (flags == 1 || b == 0)
            && p.len() == 16 + a + b + n * 12
            && p.len() <= LIMIT,
        "commercial state extent",
    )?;
    let mut ids = std::collections::HashSet::new();
    for b in p[16 + a + b..].chunks_exact(12) {
        let v = f64::from_le_bytes(b[4..].try_into().unwrap());
        need(
            v.is_finite() && (0.0..=1.0).contains(&v) && ids.insert(get(&b[..4])),
            "state parameter value/identity",
        )?;
    }
    Ok(())
}
pub fn bound_envelope(identity: Option<Identity>, p: &[u8]) -> io::Result<Vec<u8>> {
    if let Some(id) = identity {
        commercial_payload(p)?;
        envelope_for(id, 2, p)
    } else {
        envelope(p)
    }
}
pub fn bound_payload(identity: Option<Identity>, b: &[u8]) -> io::Result<&[u8]> {
    if let Some(id) = identity {
        let p = payload_for(id, 2, b)?;
        commercial_payload(p)?;
        Ok(p)
    } else {
        payload(b)
    }
}
#[no_mangle]
pub unsafe extern "C" fn ap8_validate(identity: *const u8, blob: *const u8, n: u32) -> u32 {
    crate::ffi(|| {
        if identity.is_null() || blob.is_null() || n as usize > LIMIT + HEADER_SIZE {
            return 1;
        }
        let b = std::slice::from_raw_parts(identity, 48);
        let id = Identity {
            class: b[..16].try_into().unwrap(),
            module: b[16..].try_into().unwrap(),
        };
        match bound_payload(Some(id), std::slice::from_raw_parts(blob, n as usize)) {
            Ok(_) => 0,
            Err(e) => retain(&e),
        }
    }) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_commercial_state_has_identity_integrity_and_no_gain_layout() {
        let identity = Identity {
            class: [17; 16],
            module: [42; 32],
        };
        let vendor_bytes = vec![0xff; 65537];
        let envelope = envelope_for(identity, 2, &vendor_bytes).unwrap();
        assert_eq!(payload_for(identity, 2, &envelope).unwrap(), vendor_bytes);
        assert!(payload(&envelope).is_err());
        assert!(payload_for(identity, 1, &envelope).is_err());
        assert!(payload_for(
            Identity {
                module: [43; 32],
                ..identity
            },
            2,
            &envelope
        )
        .is_err());
        let mut corrupt = envelope.clone();
        corrupt[HEADER_SIZE + 1] ^= 1;
        assert!(payload_for(identity, 2, &corrupt).is_err());
        assert!(envelope_for(identity, 2, &vec![0; LIMIT + 1]).is_err());
    }
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

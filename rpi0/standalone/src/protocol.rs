use ap2_backend::rpi0::{Context, Event, Message};

#[cfg(not(target_has_atomic = "64"))]
compile_error!("RPI0 requires native 64-bit atomics");
#[cfg(not(target_has_atomic = "32"))]
compile_error!("RPI0 requires native 32-bit atomics");

pub const HANDSHAKE_MAGIC: [u8; 8] = *b"LVBRPI0\0";
pub const HANDSHAKE_VERSION: u32 = 1;
pub const LAYOUT_DIGEST: u64 = 0x715f_58ad_ee37_950c;

pub const fn supported_jack_block(frames: usize) -> bool {
    matches!(frames, 64 | 128 | 256 | 512 | 1024)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ArchitectureHandshake {
    pub magic: [u8; 8],
    pub version: u32,
    pub extent: u32,
    pub endian: u32,
    pub pointer_bits: u32,
    pub event_extent: u32,
    pub context_extent: u32,
    pub gui_extent: u32,
    pub atomic32_lock_free: u32,
    pub atomic64_lock_free: u32,
    pub page_size: u32,
    pub reserved: u32,
    pub padding: u32,
    pub layout_digest: u64,
}

impl ArchitectureHandshake {
    pub fn native(page_size: u32) -> Self {
        Self {
            magic: HANDSHAKE_MAGIC,
            version: HANDSHAKE_VERSION,
            extent: std::mem::size_of::<Self>() as u32,
            endian: 0x0403_0201,
            pointer_bits: usize::BITS,
            event_extent: std::mem::size_of::<Event>() as u32,
            context_extent: std::mem::size_of::<Context>() as u32,
            gui_extent: std::mem::size_of::<Message>() as u32,
            // The module is compiled only when Rust exposes native 32/64-bit
            // atomics for the target. The accepted Pi AArch64 build lowers
            // these operations to the architecture's atomic instructions.
            atomic32_lock_free: 1,
            atomic64_lock_free: 1,
            page_size,
            reserved: 0,
            padding: 0,
            layout_digest: LAYOUT_DIGEST,
        }
    }

    pub fn accepts(&self, peer: &Self) -> bool {
        self.magic == HANDSHAKE_MAGIC
            && peer.magic == HANDSHAKE_MAGIC
            && self.version == peer.version
            && self.extent == peer.extent
            && self.endian == peer.endian
            && self.pointer_bits == peer.pointer_bits
            && self.event_extent == peer.event_extent
            && self.context_extent == peer.context_extent
            && self.gui_extent == peer.gui_extent
            && self.atomic32_lock_free == peer.atomic32_lock_free
            && self.atomic64_lock_free == peer.atomic64_lock_free
            && self.reserved == peer.reserved
            && self.padding == peer.padding
            && self.layout_digest == peer.layout_digest
            && self.pointer_bits == 64
            && self.atomic32_lock_free == 1
            && self.atomic64_lock_free == 1
            && self.page_size.is_power_of_two()
            && (4096..=65536).contains(&self.page_size)
            && peer.page_size.is_power_of_two()
            && (4096..=65536).contains(&peer.page_size)
    }

    pub fn encode(&self) -> [u8; 64] {
        let mut bytes = [0; 64];
        unsafe {
            std::ptr::copy_nonoverlapping(
                (self as *const Self).cast::<u8>(),
                bytes.as_mut_ptr(),
                bytes.len(),
            );
        }
        bytes
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != std::mem::size_of::<Self>() {
            return None;
        }
        Some(unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<Self>()) })
    }
}

const _: () = assert!(std::mem::size_of::<ArchitectureHandshake>() == 64);
const _: () = assert!(std::mem::align_of::<ArchitectureHandshake>() == 8);

pub fn instrument_bus_contract() -> [u8; 68] {
    let mut bytes = [0u8; 68];
    bytes[..4].copy_from_slice(&2u32.to_le_bytes());
    let mut write = |ordinal: usize, fields: [u32; 6], arrangement: u64| {
        let start = 4 + ordinal * 32;
        for (index, value) in fields.into_iter().enumerate() {
            bytes[start + index * 4..start + index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[start + 24..start + 32].copy_from_slice(&arrangement.to_le_bytes());
    };
    // Exact SDK census order: audio output, then event input.
    write(0, [0, 1, 0, 2, 0, 1], 3);
    write(1, [1, 0, 0, 16, 0, 1], 0);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mismatch_refuses_instead_of_reinterpreting() {
        let native = ArchitectureHandshake::native(4096);
        assert!(native.accepts(&native));
        let mut wrong = native;
        wrong.context_extent -= 8;
        assert!(!native.accepts(&wrong));
        wrong = native;
        wrong.pointer_bits = 32;
        assert!(!native.accepts(&wrong));
        wrong = native;
        wrong.page_size = 16384;
        assert!(native.accepts(&wrong));
        wrong.page_size = 2048;
        assert!(!native.accepts(&wrong));
    }

    #[test]
    fn bus_contract_has_only_one_event_input_and_stereo_output() {
        let wire = instrument_bus_contract();
        assert_eq!(u32::from_le_bytes(wire[..4].try_into().unwrap()), 2);
        assert_eq!(
            &wire[4..28],
            &[0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]
        );
        assert_eq!(u64::from_le_bytes(wire[28..36].try_into().unwrap()), 3);
        assert_eq!(u32::from_le_bytes(wire[36..40].try_into().unwrap()), 1);
        assert_eq!(u32::from_le_bytes(wire[48..52].try_into().unwrap()), 16);
    }

    #[test]
    fn jack_block_qualification_set_is_closed() {
        for frames in [64, 128, 256, 512, 1024] {
            assert!(supported_jack_block(frames));
        }
        for frames in [0, 1, 63, 2048] {
            assert!(!supported_jack_block(frames));
        }
    }
}

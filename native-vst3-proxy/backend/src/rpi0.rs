//! Experimental RPI0 entry surface over the reviewed AP10/AP11 backend.
//!
//! This module adds no DSP and no alternate transport. It only supplies an
//! exact, manager-independent binding for the standalone appliance owner.
use crate::{context, gui, preview, process_results, queued, state};
use ap1_native_client::events;
use std::{io, path::PathBuf};

pub use ap1_native_client::events::{Event, MAX_EVENTS, NOTE_OFF, NOTE_ON, PARAMETER};
pub use context::Context;
pub use gui::Message;
pub use process_results::Packet;
pub use queued::{Delivery, Stats};

pub const ABI_VERSION: u32 = 1;
pub const PROTOCOL_MINOR: u32 = 12;
pub const STATE_CAPACITY: usize = state::HEADER_SIZE + state::LIMIT;

const _: () = assert!(std::mem::size_of::<Event>() == 32);
const _: () = assert!(std::mem::align_of::<Event>() == 8);
const _: () = assert!(std::mem::size_of::<Context>() == 96);
const _: () = assert!(std::mem::align_of::<Context>() == 8);
const _: () = assert!(std::mem::size_of::<Delivery>() == 40);
const _: () = assert!(std::mem::align_of::<Delivery>() == 8);
const _: () = assert!(std::mem::size_of::<Message>() == 608);
const _: () = assert!(std::mem::size_of::<Packet>() == 10256);
const _: () = assert!(cfg!(target_endian = "little"));
const _: () = assert!(cfg!(target_pointer_width = "64"));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Identity {
    pub class: [u8; 16],
    pub module: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessingTraits {
    pub total_frames: u32,
    pub tail_frames: u32,
    pub vendor_frames: u32,
}

impl ProcessingTraits {
    fn from_raw(raw: [u32; 3]) -> Self {
        Self {
            total_frames: raw[0],
            tail_frames: raw[1],
            vendor_frames: raw[2],
        }
    }
}

pub struct Instance {
    handle: u64,
    closed: bool,
}

impl Instance {
    /// Opens protocol 12 against an already-created private session directory.
    /// The caller owns process launch and must run this concurrently with the
    /// Windows host because the reviewed transport performs a bounded accept.
    pub fn open_bound(
        directory: PathBuf,
        session: [u8; 16],
        identity: Identity,
        bridge_frames: u32,
    ) -> io::Result<Self> {
        if !matches!(bridge_frames, 512 | 1024 | 2048) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "RPI0 bridge delay",
            ));
        }
        let metadata = std::fs::symlink_metadata(&directory)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            unsafe extern "C" {
                fn getuid() -> u32;
            }
            if !metadata.is_dir()
                || metadata.uid() != unsafe { getuid() }
                || metadata.mode() & 0o077 != 0
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "RPI0 session directory is not private",
                ));
            }
        }
        let binding = preview::Binding {
            directory,
            session,
            installed_delay: Some(bridge_frames),
            owner: None,
        };
        let mut handle = 0;
        let code = unsafe {
            queued::open_bound(
                256,
                &mut handle,
                state::Identity {
                    class: identity.class,
                    module: identity.module,
                },
                binding,
            )
        };
        if code != 0 || handle == 0 {
            return Err(io::Error::other(format!(
                "RPI0 backend open failed: {code}"
            )));
        }
        Ok(Self {
            handle,
            closed: false,
        })
    }

    pub fn handle(&self) -> u64 {
        self.handle
    }

    #[cfg(feature = "rpi1-observe")]
    pub fn phase_ring(&self) -> io::Result<std::sync::Arc<crate::rpi1_phase::Ring>> {
        Ok(self.phase_rings()?.0)
    }

    #[cfg(feature = "rpi1-observe")]
    pub fn phase_rings(&self) -> io::Result<(std::sync::Arc<crate::rpi1_phase::Ring>,
                                           std::sync::Arc<crate::rpi1_phase::Ring>)> {
        queued::rpi1_phase_rings(self.handle)
            .ok_or_else(|| io::Error::other("RPI1 phase rings unavailable"))
    }

    #[cfg(feature = "rpi1-observe")]
    pub fn fault_site(&self) -> io::Result<u32> {
        queued::rpi1_fault_site(self.handle)
            .ok_or_else(|| io::Error::other("RPI1 fault site unavailable"))
    }

    pub fn setup(
        &self,
        maximum: u32,
        rate: f64,
        buses: &[u8],
    ) -> io::Result<ProcessingTraits> {
        let mut traits = [0; 3];
        let code = unsafe {
            queued::ap10_setup(
                self.handle,
                maximum,
                0,
                rate,
                buses.as_ptr(),
                buses.len() as u32,
                0,
                traits.as_mut_ptr(),
            )
        };
        result(code, "setup")?;
        Ok(ProcessingTraits::from_raw(traits))
    }

    pub fn activate(&self, maximum: u32) -> io::Result<()> {
        result(
            unsafe { queued::ap4_activate(self.handle, maximum.min(256), 0) },
            "activate",
        )
    }

    pub fn start(&self) -> io::Result<()> {
        result(unsafe { queued::ap3_transition(self.handle, 10) }, "start")?;
        queued::wait_started(self.handle)
    }

    pub fn stop(&self) -> io::Result<()> {
        result(unsafe { queued::ap3_transition(self.handle, 12) }, "stop")
    }

    pub fn deactivate(&self) -> io::Result<()> {
        result(unsafe { queued::ap4_deactivate(self.handle) }, "deactivate")
    }

    /// The caller must supply distinct preallocated stereo buffers of `frames`
    /// samples. This function performs only the existing bounded callback path.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn process(
        &self,
        frames: u32,
        input: [&[f32]; 2],
        output: [&mut [f32]; 2],
        input_silence: u64,
        events: &[events::Event],
        context: &context::Context,
        entered_ns: u64,
        delivery: &mut queued::Delivery,
    ) -> u32 {
        if input.iter().any(|v| v.len() != frames as usize)
            || output.iter().any(|v| v.len() != frames as usize)
            || events.len() > events::MAX_EVENTS
        {
            return 0x101;
        }
        let mut flags = 0;
        queued::if2_process(
            self.handle,
            frames,
            events.as_ptr(),
            events.len() as u32,
            context,
            input_silence,
            input[0].as_ptr(),
            input[1].as_ptr(),
            output[0].as_mut_ptr(),
            output[1].as_mut_ptr(),
            &mut flags,
            delivery,
            entered_ns,
        )
    }

    pub fn gui_generation(&self) -> io::Result<u64> {
        let mut generation = 0;
        result(
            unsafe { queued::ap11_gui_generation(self.handle, &mut generation) },
            "GUI generation",
        )?;
        Ok(generation)
    }

    pub fn gui_capabilities(&self, generation: u64, capabilities: u32) -> io::Result<()> {
        result(
            queued::ap11_gui_capabilities(self.handle, generation, capabilities),
            "GUI capabilities",
        )
    }

    pub fn gui_command(&self, generation: u64, message: &mut Message) -> io::Result<()> {
        result(
            unsafe { queued::ap11_gui_command(self.handle, generation, message) },
            "GUI command",
        )
    }

    pub fn gui_take(&self, generation: u64, message: &mut Message) -> io::Result<()> {
        result(
            unsafe { queued::ap11_gui_take(self.handle, generation, message) },
            "GUI take",
        )
    }

    pub fn capture_state(&self, output: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        result(
            unsafe {
                queued::ap4_state(
                    self.handle,
                    std::ptr::null(),
                    0,
                    output.as_mut_ptr(),
                    output.len() as u32,
                    &mut written,
                )
            },
            "capture state",
        )?;
        Ok(written as usize)
    }

    pub fn restore_state(&self, state: &[u8], output: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        result(
            unsafe {
                queued::ap4_state(
                    self.handle,
                    state.as_ptr(),
                    state.len() as u32,
                    output.as_mut_ptr(),
                    output.len() as u32,
                    &mut written,
                )
            },
            "restore state",
        )?;
        Ok(written as usize)
    }

    pub fn stats(&self) -> io::Result<Stats> {
        let mut stats = Stats::default();
        result(
            unsafe { queued::ap3_stats(self.handle, &mut stats) },
            "statistics",
        )?;
        Ok(stats)
    }

    pub fn close(mut self) -> io::Result<()> {
        let code = unsafe { queued::if2_close(self.handle) };
        self.closed = true;
        result(code, "close")
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        if !self.closed {
            let _ = unsafe { queued::if2_close(self.handle) };
        }
    }
}

fn result(code: u32, operation: &str) -> io::Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!("RPI0 {operation} failed: {code}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_architecture_layout_contract_is_exact() {
        assert_eq!(ABI_VERSION, 1);
        assert_eq!(PROTOCOL_MINOR, 12);
        assert_eq!(std::mem::size_of::<Event>(), 32);
        assert_eq!(std::mem::size_of::<Context>(), 96);
        assert_eq!(std::mem::size_of::<Delivery>(), 40);
        assert_eq!(std::mem::size_of::<Message>(), 608);
        assert_eq!(std::mem::size_of::<Packet>(), 10256);
        assert_eq!(std::mem::offset_of!(Message, activation), 560);
        assert_eq!(std::mem::offset_of!(Message, native_view), 592);
    }

    #[test]
    fn appliance_delay_set_is_closed() {
        for value in [512, 1024, 2048] {
            assert!(crate::performance::validate_delay(256, value).is_ok());
        }
        for value in [0, 128, 256, 4096] {
            assert!(Instance::open_bound(
                PathBuf::new(),
                [0; 16],
                Identity {
                    class: [0; 16],
                    module: [0; 32]
                },
                value
            )
            .is_err());
        }
    }

    #[test]
    fn processing_traits_do_not_swap_vendor_total_and_tail() {
        assert_eq!(
            ProcessingTraits::from_raw([2048, u32::MAX, 0]),
            ProcessingTraits {
                total_frames: 2048,
                tail_frames: u32::MAX,
                vendor_frames: 0,
            }
        );
    }
}

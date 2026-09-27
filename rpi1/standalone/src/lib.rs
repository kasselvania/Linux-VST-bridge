//! Standalone ARM audio and execution adapters for the shared bridge backend.
pub mod binding;
pub mod config;
pub mod contract;
pub mod master;
#[cfg(unix)]
pub mod retirement;
#[cfg(unix)]
pub mod startup;
#[cfg(unix)]
pub mod supervisor;

const _: () = assert!(cfg!(target_endian = "little"));
const _: () = assert!(cfg!(target_pointer_width = "64"));

pub mod panel;

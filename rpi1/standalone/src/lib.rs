//! Experiment-owned Pigments binding over the accepted RPI0 ARM appliance edge.
#[cfg(target_os = "linux")]
pub mod buttons;
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

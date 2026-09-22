//! Experiment-owned Pigments binding over the accepted RPI0 ARM appliance edge.
pub mod config;
pub mod contract;
#[cfg(unix)]
pub mod retirement;
#[cfg(unix)]
pub mod supervisor;

const _: () = assert!(cfg!(target_endian = "little"));
const _: () = assert!(cfg!(target_pointer_width = "64"));

//! Experiment-owned Pigments binding over the accepted RPI0 ARM appliance edge.
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
#[cfg(target_os = "linux")]
pub mod panel_linux;
#[cfg(unix)]
pub mod panel_state;

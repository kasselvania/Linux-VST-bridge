//! The manager service and its clients (#489 §4).
//!
//! `contract` is the one typed exchange the window, the `operator` command
//! line and the Native Access browser return share with the service. `view`
//! holds the status replies the service composed, and `changes` is the watch
//! that tells it when one is no longer current.
pub mod changes;
pub mod contract;
pub mod view;

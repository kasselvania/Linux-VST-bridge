//! The wire contract between the bridge service and its clients: the operator
//! model they exchange, and the typed service contract that carries it, with
//! its bounds, refusals and the `LVV1` status-read frame. It also holds the
//! client transports, which start the installed manager command or connect to
//! the service socket but never write records. The service, the command line
//! and every window depend on this crate; it depends on none of them. The
//! few words both sides use (the busy-lock refusal, how a version is shown)
//! are in `wording`.
//!
//! The command transport (`Subprocess`, `Subprocess::installed` and the
//! `Installed` fallback) goes away when the window's command fallback is
//! removed (#504 B3); `Service::installed` names the socket the window keeps.
pub mod contract;
pub mod operator_model;
pub mod wording;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn require(ok: bool, why: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(why.into())
    }
}
/// A fresh name for a test's scratch directory.
#[cfg(test)]
fn random_id() -> Result<String> {
    use std::io::Read;
    let mut bytes = [0u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

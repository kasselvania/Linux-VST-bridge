//! The wire contract between the bridge service and its clients: the operator
//! model they exchange, and the typed service contract that carries it, with
//! its bounds, refusals, the `LVV1` status-read frame and the transports that
//! reach the service. Nothing product-specific: the service, the command line
//! and every window depend on this crate, and it depends on none of them.
pub mod contract;
pub mod operator_model;

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

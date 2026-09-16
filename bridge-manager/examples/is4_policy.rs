//! Source-owned generated proof adapter to the production Rust construction owner.
//! Not installed, not reachable through normal CLI/operator requests.
use linux_vst_bridge::{catalogue::Software, installer_policy::{bind, Powershell}};
use std::{fs, io::Write};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len()!=5 {return Err("fixture spec/software/policy/output required".into())}
    let mut spec=serde_json::from_slice(&fs::read(&args[1])?)?;
    let software: Software=serde_json::from_slice(&fs::read(&args[2])?)?;
    let policy: Powershell=serde_json::from_value(serde_json::Value::String(args[3].clone()))?;
    bind(&mut spec,&software,policy)?;
    let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&args[4])?;
    file.write_all(&serde_json::to_vec(&spec)?)?;file.sync_all()?;
    Ok(())
}

//! Development-only admission/report command. Does not install product software.
use linux_vst_bridge::{ui_observation::*, Manager, Result};
fn run()->Result<()> {
    let a:Vec<_>=std::env::args().skip(1).collect();
    match a.as_slice() {
        [command,class] if command=="admit" => println!("{}",serde_json::to_string(&admit(&Manager::installed()?,class)?)?),
        [command,path] if command=="validate" => {let report:Waterfall=linux_vst_bridge::read_json(std::path::Path::new(path))?;report.validate()?;println!("{}",serde_json::to_string(&report)?);},
        _=>return Err("usage: uio1 admit CLASS | validate REPORT".into()),
    }
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("UIO1 refused: {e}");std::process::exit(1);}}

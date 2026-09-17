//! Source-owned generated fixture driver; absent from installed manager CLI.
use linux_vst_bridge::{catalogue::Software, renderer_application::{Application,RendererPolicy,bind}};
use std::{fs,io::Write,path::Path};
fn main()->Result<(),Box<dyn std::error::Error+Send+Sync>> {
    let args:Vec<_>=std::env::args().collect();
    if args.len()!=7{return Err("fixture app/software/operation/policy/report/output required".into());}
    let app:Application=serde_json::from_slice(&fs::read(&args[1])?)?;
    let sw:Software=serde_json::from_slice(&fs::read(&args[2])?)?;
    let policy:RendererPolicy=serde_json::from_value(serde_json::Value::String(args[4].clone()))?;
    let spec=bind(&app,&sw,&args[3],policy,Path::new(&args[5]))?;
    let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&args[6])?;
    f.write_all(&serde_json::to_vec(&spec)?)?;f.sync_all()?;Ok(())
}

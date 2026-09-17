//! Source-owned generated fixture driver; absent from installed manager CLI.
use linux_vst_bridge::{catalogue::Software, renderer_application::{Application,RendererPolicy,bind}};
use std::{fs,io::Write,path::Path};
fn main()->Result<(),Box<dyn std::error::Error+Send+Sync>> {
    let args:Vec<_>=std::env::args().collect();
    if args.get(1).is_some_and(|s|matches!(s.as_str(),"submit"|"stop"|"reconcile")) {
        use linux_vst_bridge::{Manager,renderer_session as lifecycle};
        use std::process::Command;
        if args.len()<4{return Err("fixture lifecycle arguments".into());}
        let m=Manager{root:args[2].clone().into(),publications:std::path::PathBuf::from(&args[2]).join("unused-publications")};
        match args[1].as_str(){
            "submit"=>{
                if args.len()!=6{return Err("fixture submit arguments".into());}
                let spec:serde_json::Value=serde_json::from_slice(&fs::read(&args[3])?)?;
                let op=spec["operation"].as_str().ok_or("operation")?;
                let result=lifecycle::submit_with(&m,&spec,|path|Ok(Command::new("systemd-run").args(["--user","--collect","--slice=app.slice","--property=UMask=0077","--property=KillMode=control-group","--property=TimeoutStopSec=30","--property=StandardOutput=null","--property=StandardError=null"])
                    .arg(format!("--unit={}",lifecycle::unit(op)?)).arg("/usr/bin/python3").arg("-B").arg(Path::new(&args[4]).join("supervise.py")).arg(&args[5]).arg(path).status()?.success()),||lifecycle::inspect(op))?;
                println!("{}",serde_json::to_string(&result)?);
            }
            "stop"=>{if args.len()!=4{return Err("fixture stop arguments".into());}lifecycle::stop(&m,&args[3])?;}
            "reconcile"=>{if args.len()!=4{return Err("fixture reconcile arguments".into());}println!("{}",serde_json::to_string(&lifecycle::reconcile(&m,&args[3])?)?);}
            _=>unreachable!()
        }
        return Ok(());
    }
    if args.len()!=7{return Err("fixture app/software/operation/policy/report/output required".into());}
    let app:Application=serde_json::from_slice(&fs::read(&args[1])?)?;
    let sw:Software=serde_json::from_slice(&fs::read(&args[2])?)?;
    let policy:RendererPolicy=serde_json::from_value(serde_json::Value::String(args[4].clone()))?;
    let spec=bind(&app,&sw,&args[3],policy,Path::new(&args[5]))?;
    let mut f=fs::OpenOptions::new().write(true).create_new(true).open(&args[6])?;
    f.write_all(&serde_json::to_vec(&spec)?)?;f.sync_all()?;Ok(())
}

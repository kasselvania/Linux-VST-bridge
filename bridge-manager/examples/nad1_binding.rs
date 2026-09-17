//! Source-owned generated fixture driver; absent from installed manager CLI.
use linux_vst_bridge::{
    catalogue::Software,
    renderer_application::{bind, Application, RendererPolicy},
};
use std::{fs, io::Write, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        libc::umask(0o077);
    }
    let args: Vec<_> = std::env::args().collect();
    if args
        .get(1)
        .is_some_and(|s| matches!(s.as_str(), "submit" | "stop" | "reconcile" | "application-submit" | "application-stop" | "application-reconcile"))
    {
        let application=args[1].starts_with("application-");
        let mut args=args.clone();
        if application { args[1]=args[1].trim_start_matches("application-").into(); }
        macro_rules! dispatch { ($owner:ident) => {{
        use linux_vst_bridge::{ $owner as lifecycle, Manager };
        use std::process::Command;
        if args.len() < 4 {
            return Err("fixture lifecycle arguments".into());
        }
        let m = Manager {
            root: args[2].clone().into(),
            publications: std::path::PathBuf::from(&args[2]).join("unused-publications"),
        };
        match args[1].as_str() {
            "submit" => {
                if args.len() != 6 {
                    return Err("fixture submit arguments".into());
                }
                let spec: serde_json::Value = serde_json::from_slice(&fs::read(&args[3])?)?;
                let op = spec["operation"].as_str().ok_or("operation")?;
                let result = lifecycle::submit_with(
                    &m,
                    &spec,
                    |path| {
                        Ok(Command::new("systemd-run")
                            .args([
                                "--user",
                                "--collect",
                                "--slice=app.slice",
                                "--property=UMask=0077",
                                "--property=KillMode=mixed",
                                "--property=TimeoutStopSec=90",
                                "--property=StandardOutput=null",
                                "--property=StandardError=null",
                            ])
                            .arg(format!("--unit={}", lifecycle::unit(op)?))
                            .arg("/usr/bin/python3")
                            .arg("-B")
                            .arg(Path::new(&args[4]).join(if application {"application_supervise.py"} else {"owner_supervise.py"}))
                            .arg(&args[5])
                            .arg(path)
                            .status()?
                            .success())
                    },
                    || lifecycle::inspect(op),
                )?;
                println!("{}", serde_json::to_string(&result)?);
            }
            "stop" => {
                if args.len() != 4 {
                    return Err("fixture stop arguments".into());
                }
                lifecycle::stop(&m, &args[3])?;
            }
            "reconcile" => {
                if args.len() != 4 {
                    return Err("fixture reconcile arguments".into());
                }
                println!(
                    "{}",
                    serde_json::to_string(&lifecycle::reconcile(&m, &args[3])?)?
                );
            }
            _ => unreachable!(),
        }
        }}; }
        if application { dispatch!(renderer_session); } else { dispatch!(dependency_session); }
        return Ok(());
    }
    if args.len() != 7 && args.len() != 8 {
        return Err("fixture app/software/operation/policy/report/output required".into());
    }
    let app: Application = serde_json::from_slice(&fs::read(&args[1])?)?;
    let sw: Software = serde_json::from_slice(&fs::read(&args[2])?)?;
    let policy: RendererPolicy =
        serde_json::from_value(serde_json::Value::String(args[4].clone()))?;
    let mut spec = bind(&app, &sw, &args[3], policy, Path::new(&args[5]))?;
    if args.len()==7 { spec["kind"] = serde_json::json!("native_access_dependency"); spec["schema"]=serde_json::json!(2); spec["dependency_mode"]=serde_json::json!("prepare"); } else if args[7]!="application" { return Err("fixture kind".into()); }
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[6])?;
    f.write_all(&serde_json::to_vec(&spec)?)?;
    f.sync_all()?;
    Ok(())
}

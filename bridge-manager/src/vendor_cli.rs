use super::*;
use linux_vst_bridge::vendor_application::{
    self, Application, ApplicationId, LaunchMode, OperationResult,
};

pub fn run(m: &Manager, args: &[String]) -> Result<()> {
    require(args.len() == 2, "vendor-app ACTION arturia-software-center")?;
    ApplicationId::parse(&args[1])?;
    let directory = m.root.join("vendor-applications/arturia-software-center");
    private_dir(&directory)?;
    let record = directory.join("application.json");
    match args[0].as_str() {
        "rescan" => {
            let _guard = m.lock("registry.lock")?;
            m.require_inactive(None)?;
            let environments: std::collections::BTreeMap<_, _> = m
                .registry()?
                .classes
                .values()
                .map(|e| {
                    (
                        e.registration.environment.id.clone(),
                        e.registration.environment.clone(),
                    )
                })
                .collect();
            require(
                environments.len() == 1,
                "vendor_application_environment_ambiguous",
            )?;
            let env = environments
                .into_values()
                .next()
                .ok_or("vendor_application_environment_absent")?;
            let operation = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(env.root.join("operation.lock"))?;
            use std::os::fd::AsRawFd;
            require(
                unsafe { libc::flock(operation.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
                "vendor_application_environment_busy",
            )?;
            let app = vendor_application::discover(&m.root, env)?;
            if record.exists() {
                require(
                    read_json::<Application>(&record)? == app,
                    "vendor_application_update_requires_transition",
                )?;
            } else {
                atomic_json(&record, &app)?;
            }
            println!("{}", serde_json::to_string(&app)?);
            Ok(())
        }
        "status" => {
            let app: Application = read_json(&record)?;
            app.verify(&m.root)?;
            let report = directory.join("operation-result.json");
            let mut operation = if report.exists() {
                serde_json::to_value(read_json::<OperationResult>(&report)?)?
            } else {
                serde_json::json!({"state":"absent"})
            };
            let live = Command::new("systemctl")
                .args([
                    "--user",
                    "is-active",
                    "linux-vst-bridge-vendor-arturia-software-center",
                ])
                .output()?
                .status
                .success();
            if !live && matches!(operation["state"].as_str(), Some("running" | "unknown")) {
                operation["state"] = "cleanup_unconfirmed".into();
            }
            println!(
                "{}",
                serde_json::json!({"schema":1,"application":app,"operation":operation,"owner_active":live})
            );
            Ok(())
        }
        "cancel" => {
            let _guard = m.lock("registry.lock")?;
            let prior: OperationResult = read_json(&directory.join("operation-result.json"))?;
            if prior.retired() {
                let state = Command::new("systemctl")
                    .args([
                        "--user",
                        "show",
                        "linux-vst-bridge-vendor-arturia-software-center",
                        "-p",
                        "ActiveState",
                        "--value",
                    ])
                    .output()?;
                if state.status.success()
                    && matches!(
                        std::str::from_utf8(&state.stdout)?.trim(),
                        "inactive" | "failed"
                    )
                {
                    println!("{}", serde_json::to_string(&prior)?);
                    return Ok(());
                }
            }
            let app: Application = read_json(&record)?;
            app.verify(&m.root)?;
            let result = Command::new("systemctl")
                .args([
                    "--user",
                    "stop",
                    "linux-vst-bridge-vendor-arturia-software-center",
                ])
                .status()?;
            require(
                result.success(),
                "vendor_application_cancellation_unconfirmed",
            )?;
            let result: OperationResult = read_json(&directory.join("operation-result.json"))?;
            require(result.retired(), "vendor_application_cleanup_unconfirmed")?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        }
        "launch" | "diagnose-agent" | "diagnose-runinprefix" | "diagnose-initialized" => {
            let mode = LaunchMode::action(&args[0])?;
            let _guard = m.lock("registry.lock")?;
            m.require_inactive(None)?;
            let app: Application = read_json(&record)?;
            app.verify(&m.root)?;
            let sw = software(m)?;
            let unit = "linux-vst-bridge-vendor-arturia-software-center";
            // systemd owns the launch beyond this CLI/SSH lifetime. An existing
            // unit refuses a second launch rather than duplicating the app.
            let job = directory.join("operation.json");
            if job.exists() {
                let old: OperationResult = read_json(&directory.join("operation-result.json"))?;
                require(
                    old.retired(),
                    "vendor_application_operation_requires_retirement",
                )?;
                let state = Command::new("systemctl")
                    .args(["--user", "show", unit, "-p", "ActiveState", "--value"])
                    .output()?;
                require(
                    state.status.success()
                        && matches!(
                            std::str::from_utf8(&state.stdout)?.trim(),
                            "inactive" | "failed"
                        ),
                    "vendor_application_owner_active",
                )?;
                let history = directory.join(format!("retired-{}", random_id()?));
                private_dir(&history)?;
                fs::rename(&job, history.join("operation.json"))?;
                fs::rename(
                    directory.join("operation-result.json"),
                    history.join("result.json"),
                )?;
                let _ = Command::new("systemctl")
                    .args(["--user", "reset-failed", unit])
                    .status()?;
            }
            atomic_json(
                &job,
                &serde_json::json!({"application":app,"report":directory.join("operation-result.json"),"mode":mode}),
            )?;
            let result = Command::new("systemd-run")
                .args([
                    "--user",
                    "--collect",
                    "--property=UMask=0077",
                    "--property=KillMode=control-group",
                    "--property=TimeoutStopSec=30",
                    "--property=StandardOutput=null",
                    "--property=StandardError=null",
                ])
                .arg(format!("--unit={unit}"))
                .arg("/usr/bin/python3")
                .arg(&sw.supervisor.path)
                .arg("--vendor-application")
                .arg(&job)
                .status()?;
            require(result.success(), "vendor_application_launch_unconfirmed")
        }
        _ => Err("vendor_application_action_unknown".into()),
    }
}

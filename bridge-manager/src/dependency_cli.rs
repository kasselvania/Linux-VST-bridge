//! Closed NAD1 preparation dispatch. It never launches Native Access.
use super::*;
use linux_vst_bridge::{
    dependency_session as life, native_access_dependency as dep, operator_model as ui,
    renderer_application as app,
};
use serde_json::{json, Value};
pub(super) fn all_retired(m: &Manager) -> Result<bool> {
    let c = life::current(m)?;
    if c.is_null() {
        return Ok(true);
    }
    life::retired(m, c["operation"].as_str().ok_or("dependency_current")?)
}
pub(super) fn launch(m: &Manager, op: &str) -> Result<life::Submission> {
    let application: app::Application =
        read_json(&renderer_cli::directory(m).join("application.json"))?;
    application.verify(&m.root)?;
    let software = software(m)?;
    let directory = life::operation_dir(m, op)?;
    let spec = dep::bind(&application, &software, op, &directory.join("result.json"))?;
    let _guard = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(
        all_retired(m)? && renderer_cli::all_retired(m)?,
        "dependency_previous_operation_live",
    )?;
    life::submit_with(
        m,
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
                .arg(format!("--unit={}", life::unit(op)?))
                .arg("/usr/bin/python3")
                .arg(&software.supervisor.path)
                .arg("--vendor-application")
                .arg(path)
                .status()?
                .success())
        },
        || life::inspect(op),
    )
}
pub(super) fn retain(m: &Manager, op: &str) -> Result<()> {
    let application: app::Application =
        read_json(&renderer_cli::directory(m).join("application.json"))?;
    dep::retain_prepared(m, &application, &software(m)?, op)
}
pub(super) fn prepared(m: &Manager) -> Result<Value> {
    let app = read_json(&renderer_cli::directory(m).join("application.json"))?;
    dep::prepared(m, &app, &software(m)?)
}
pub(super) fn project(
    m: &Manager,
    card: &mut ui::VendorApplication,
    busy: Option<&str>,
) -> Result<()> {
    let c = life::current(m)?;
    let retired = all_retired(m)?;
    let prepared = prepared(m).is_ok();
    let make = |label: &str, action, why: Option<&str>| ui::AvailableAction {
        label: label.into(),
        action,
        disabled_reason: why.map(Into::into),
    };
    if renderer_cli::directory(m).join("application.json").is_file() {card.actions.push(make(
        "Prepare / recover Native Access dependency",
        ui::Action::DependencyPrepare {},
        if !retired {
            Some("Retire the exact dependency operation first")
        } else {
            busy
        },
    ));}
    if let Some(op) = c["operation"].as_str() {
        card.details["dependency_operation"] = life::result(m, op)?;
        card.details["dependency_manager_operation"]=json!({"operation":op,"state":if retired {"completed"} else if card.details["dependency_operation"].is_null(){"submission_uncertain"}else{"vendor_running"}});
        card.actions.push(make(
            "Stop / reconcile Native Access dependency",
            ui::Action::DependencyStop {
                operation: op.into(),
            },
            if retired {
                Some("Dependency operation is retired")
            } else {
                None
            },
        ));
    }
    card.details["dependency_prepared"] = json!(prepared);
    card.details["dependency_ready_now"] = Value::Null;
    for a in &mut card.actions {
        if matches!(a.action, ui::Action::RendererOpen { .. }) && !prepared {
            a.disabled_reason=Some("Prepare the exact Native Access dependency first; live service readiness is checked before application launch".into());
        }
    }
    Ok(())
}

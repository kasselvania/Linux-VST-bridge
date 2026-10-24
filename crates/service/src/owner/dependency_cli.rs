//! Closed NAD1 preparation dispatch. It never launches Native Access.
use super::*;
use linux_vst_bridge::{
    dependency_session as life, native_access_dependency as dep, operator_model as ui,
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
    vendor_app::prepare_retained_service(m, op)
}
pub(super) fn retain(m: &Manager, op: &str) -> Result<()> {
    let application=renderer_cli::current_application(m)?;
    dep::retain_prepared(m, &application, &linux_vst_bridge::preparation::build::launch_software(m)?, op)
}
pub(super) fn prepared(m: &Manager) -> Result<Value> {
    let app=renderer_cli::current_application(m)?;
    dep::prepared(m, &app, &linux_vst_bridge::preparation::build::launch_software_record(m)?)
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
    Ok(())
}

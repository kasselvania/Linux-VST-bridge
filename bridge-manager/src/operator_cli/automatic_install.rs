//! Initial user intent carried through existing installer, discovery and
//! preparation owners. Readback only projects these retained results.
use super::*;
use linux_vst_bridge::preparation as prep;
use linux_vst_bridge::publication::{Revision,RevisionRef};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Intent {
    pub owner: String,
    pub active_operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor: Option<VendorWork>,
    pub journey: ui::InstallationJourney,
    pub scan: Option<String>,
    pub prior_selections: Vec<String>,
    pub prior_modules: Vec<String>,
    pub failed_modules: Vec<FailedModule>,
    pub work: Vec<ClassWork>,
    #[serde(default)]
    pub predecessors: Vec<Revision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous: Option<Artifact>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VendorWork {
    pub application: String,
    pub selected: Artifact,
    pub operation: String,
    pub result: Option<Artifact>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClassWork {
    selection: String,
    #[serde(default)]
    predecessor: Option<RevisionRef>,
    result_index: usize,
    check: Option<String>,
    candidate: Option<String>,
    active_operation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FailedModule {
    module_index: usize,
    result_index: usize,
    artifact: Artifact,
    report: Artifact,
}

type ExecuteStage<'a> = dyn FnMut(&ui::Action, &str, &mut dyn FnMut(&str)) -> Result<Value> + 'a;

/// Initial installer history is optional for a later, already owned application.
/// The real Environment and its existing operator revision job own that work.
trait InstallationTarget {
    fn environment(&self) -> &Environment;
    fn installation(&self) -> Option<&onboarding::Record>;
    fn renderer(&self) -> bool {false}
    fn id(&self) -> &str { &self.environment().id }
}
impl InstallationTarget for onboarding::Record {
    fn environment(&self) -> &Environment { &self.environment }
    fn installation(&self) -> Option<&onboarding::Record> { Some(self) }
}
impl InstallationTarget for Environment {
    fn environment(&self) -> &Environment { self }
    fn installation(&self) -> Option<&onboarding::Record> { None }
}

struct RendererTarget(Environment);
impl InstallationTarget for RendererTarget {
    fn environment(&self) -> &Environment {&self.0}
    fn installation(&self) -> Option<&onboarding::Record> {None}
    fn renderer(&self) -> bool {true}
}
fn refresh_target(record:&dyn InstallationTarget,intent:&Intent)->bool {
    intent.vendor.is_none() && record.installation().is_none() && !record.renderer()
}

fn checkpoint(m: &Manager, owner: &str) -> Result<PathBuf> {
    Ok(job_dir(m, owner)?.join("automatic.json"))
}

/// Current application work is selected by its owned revision, not recency of
/// arbitrary files or whichever unrelated operation is globally latest.
pub(super) fn vendor_intent(m: &Manager, application: &str) -> Result<Option<Intent>> {
    let app = vendor_cli::application(m, application)?;
    let mut bindings = environment_revision::vendor_records(m)?.into_iter()
        .filter(|b| b.application == application && b.environment.id == app.environment.id)
        .collect::<Vec<_>>();
    bindings.sort_by_key(|b| b.environment.revision);
    let Some(bound) = bindings.pop() else { return Ok(None) };
    require(app.environment == bound.before
        || environment_revision::resolves_to(m, &bound.environment, &app.environment)?,
        "automatic_vendor_revision_changed")?;
    let path = checkpoint(m, &bound.operation)?;
    let intent = if path.exists() {
        Some(read_json::<Intent>(&path)?)
    } else {
        // Preserve the earlier shipped owner's checkpoint in its real history.
        onboarding::retained_environment_record(m, &app.environment.id)?
            .and_then(|r| r.automatic).filter(|i| i.owner == bound.operation)
    };
    require(intent.is_some(), "automatic_vendor_intent_missing")?;
    if let Some(intent) = &intent {
        let vendor = intent.vendor.as_ref().ok_or("automatic_vendor_intent_missing")?;
        require(intent.owner == bound.operation && vendor.application == application
            && vendor.operation == bound.vendor_operation && vendor.selected == bound.selected,
            "automatic_vendor_intent_binding")?;
    }
    Ok(intent)
}

pub(super) fn current_vendor_bindings(m: &Manager) -> Result<Vec<environment_revision::VendorBinding>> {
    let mut selected=std::collections::BTreeMap::new();
    for binding in environment_revision::vendor_records(m)? {
        let app=vendor_cli::application(m,&binding.application)?;
        if binding.environment.id != app.environment.id { continue; }
        let entry=selected.entry(binding.application.clone()).or_insert_with(||binding.clone());
        if binding.environment.revision > entry.environment.revision { *entry=binding; }
    }
    Ok(selected.into_values().collect())
}
pub(super) fn vendor_checkpoint_authority(m: &Manager) -> Result<Vec<Value>> {
    current_vendor_bindings(m)?.into_iter().map(|binding| {
        let path=checkpoint(m,&binding.operation)?;
        let artifact=if path.exists() { Some(control_artifact(path)?) } else { None };
        Ok(json!({"binding":binding,"checkpoint":artifact}))
    }).collect()
}

fn record_intent(m: &Manager, record: &onboarding::Record) -> Result<Option<Intent>> {
    let Some(original) = &record.automatic else { return Ok(None) };
    if let Some(vendor) = &original.vendor {
        if let Some(current) = vendor_intent(m, &vendor.application)? { return Ok(Some(current)); }
    }
    Ok(Some(original.clone()))
}

pub(super) fn empty_vendor_handoff(m: &Manager, intent: &Intent) -> Result<bool> {
    let Some(vendor) = &intent.vendor else { return Ok(false) };
    if intent.scan.is_none() || !intent.work.is_empty() || !intent.failed_modules.is_empty()
        || !intent.journey.plugins.is_empty() || intent.journey.phase != ui::InstallationPhase::NeedsAttention
        || !intent.journey.failure.as_ref().is_some_and(|f| f.stage == ui::InstallationFailureStage::Discovery
            && f.code == "no_supported_instruments_or_effects_discovered") { return Ok(false); }
    require_vendor_success(m, vendor)?;
    Ok(true)
}
fn current_journey(m: &Manager, intent: &Intent) -> Result<ui::InstallationJourney> {
    let mut journey=intent.journey.clone();
    if empty_vendor_handoff(m,intent)? {
        journey.phase=ui::InstallationPhase::AwaitingApplication;journey.failure=None;
    }
    Ok(journey)
}

fn retain_vendor_intent(m: &Manager, prior: &Intent) -> Result<Artifact> {
    let directory = job_dir(m, &prior.owner)?;
    let mut retained = prior.clone();
    if let Some(result) = retained.vendor.as_mut().and_then(|v| v.result.as_mut()) {
        result.verify()?;
        let path = directory.join("retained-vendor-result.json");
        if !path.exists() { fs::copy(&result.path, &path)?; }
        let saved = control_artifact(path)?;
        require(saved.sha256 == result.sha256, "automatic_vendor_history_changed")?;
        *result = saved;
    }
    let path = directory.join("retained-automatic.json");
    if !path.exists() { atomic_json(&path, &retained)?; }
    require(serde_json::to_value(read_json::<Intent>(&path)?)? == serde_json::to_value(retained)?,
        "automatic_vendor_history_changed")?;
    control_artifact(path)
}

pub(super) fn empty_bootstrap_handoff(m: &Manager, record: &onboarding::Record,
    intent: &Intent, application: &str) -> Result<bool> {
    let Some(app) = bootstrap_handoff_application(m, record, intent, application)? else { return Ok(false); };
    app.verify(&m.root)?;
    Ok(true)
}

pub(super) fn empty_bootstrap_handoff_record(m: &Manager, record: &onboarding::Record,
    intent: &Intent, application: &str) -> Result<bool> {
    let Some(app) = bootstrap_handoff_application(m, record, intent, application)? else { return Ok(false); };
    app.validate_record(&m.root)?;
    Ok(true)
}

fn bootstrap_handoff_application(m: &Manager, record: &onboarding::Record,
    intent: &Intent, application: &str) -> Result<Option<vendor_application::Application>> {
    if intent.vendor.is_some() || !intent.work.is_empty() || !intent.failed_modules.is_empty()
        || !intent.journey.plugins.is_empty() || intent.journey.phase != ui::InstallationPhase::NeedsAttention
        || !intent.journey.failure.as_ref().is_some_and(|f| f.stage == ui::InstallationFailureStage::Discovery
            && f.code == "no_supported_instruments_or_effects_discovered") { return Ok(None); }
    let installation = onboarding::result(m, record)?;
    if installation["state"] != "completed" || installation["raw_exit"] != 0
        || !onboarding::retired(&installation) { return Ok(None); }
    let app = vendor_cli::application(m, application)?;
    if app.environment != record.environment || app.installer_sha256.as_deref() != Some(record.installer.as_str())
        || app.installer_result.as_ref().unwrap().path != onboarding::directory(m, &record.id)?.join(format!("{}-result.json",
            record.installation_operation.as_deref().ok_or("vendor_application_installation_missing")?)) { return Ok(None); }
    Ok(Some(app))
}

fn handoff_choices(m: &Manager, record: &onboarding::Record, intent: &Intent,
    busy: Option<&str>) -> Result<Vec<ui::VendorApplication>> {
    if intent.vendor.is_some() || !intent.journey.plugins.is_empty() || !intent.work.is_empty()
        || !intent.failed_modules.is_empty() || intent.journey.phase != ui::InstallationPhase::NeedsAttention
        || !intent.journey.failure.as_ref().is_some_and(|f|f.stage == ui::InstallationFailureStage::Discovery
            && f.code == "no_supported_instruments_or_effects_discovered") { return Ok(vec![]); }
    let mut selectors = vec![];
    for selector in vendor_cli::selectors(m)? {
        if empty_bootstrap_handoff_record(m, record, intent, &selector)? { selectors.push(selector); }
    }
    if selectors.is_empty() {
        for choice in vendor_cli::selection::choices(m, record)? {
            let selector = choice.application.selector()?;
            if empty_bootstrap_handoff_record(m, record, intent, &selector)? { selectors.push(selector); }
        }
    }
    selectors.into_iter().map(|selector|vendor_projection(m, &selector, busy))
        .collect::<Result<Vec<_>>>().map(|rows|rows.into_iter().flatten().collect())
}

fn project_handoff(m: &Manager, record: &onboarding::Record, intent: &Intent,
    setup: &mut ui::InstallerSetup, busy: Option<&str>) -> Result<bool> {
    let choices = match handoff_choices(m, record, intent, busy) {
        Ok(choices) => choices,
        Err(error) => {
            setup.journey = Some(intent.journey.clone());
            setup.status = "The installed app's observed identity needs recovery before it can be opened.".into();
            if let Some(journey) = &mut setup.journey {
                journey.failure = Some(failure(ui::InstallationFailureStage::Discovery, error));
                journey.failure.as_mut().unwrap().reason = setup.status.clone();
            }
            setup.primary = None;
            setup.secondary.clear();
            return Ok(true);
        }
    };
    if choices.is_empty() { return Ok(false); }
    // A later exact app visit satisfies this bootstrap's missing next step.
    // Its checkpoint supplies current progress; the original failure stays in history.
    let current = choices.iter().map(|app|vendor_intent(m, &app.id))
        .collect::<Result<Vec<_>>>()?.into_iter().flatten().next();
    let mut journey = current.as_ref().map(|i|current_journey(m,i)).transpose()?
        .unwrap_or_else(||intent.journey.clone());
    if current.is_none() {
        journey.phase = ui::InstallationPhase::AwaitingApplication;
        journey.failure = None;
    }
    setup.journey = Some(journey.clone());
    setup.status = match journey.phase {
        ui::InstallationPhase::AwaitingApplication => "The app is installed. Open it to install your instruments and effects.",
        ui::InstallationPhase::Available if journey.plugins.is_empty() => "The app visit finished. No plug-ins were added.",
        ui::InstallationPhase::Available => "Installed plug-ins are available through the native DAW connection.",
        ui::InstallationPhase::NeedsAttention => "The app task needs attention; its files and results are retained.",
        _ => "Continue in the app; installed plug-ins are connected automatically.",
    }.into();
    setup.primary = None;
    setup.secondary.clear();
    if journey.phase != ui::InstallationPhase::Available {
        let offers = choices.into_iter().flat_map(|app|app.actions)
            .filter(|offer|matches!(offer.action, ui::Action::VendorSoftwareInstall { .. }
                | ui::Action::VendorApplicationFocus { .. } | ui::Action::VendorApplicationStop { .. }))
            .collect::<Vec<_>>();
        if offers.iter().filter(|offer|matches!(offer.action,ui::Action::VendorSoftwareInstall { .. })).count() == 1 {
            setup.primary = offers.iter().find(|offer|matches!(offer.action,ui::Action::VendorSoftwareInstall { .. })).cloned();
        }
        setup.secondary = offers.into_iter().filter(|offer|setup.primary.as_ref()
            .is_none_or(|primary|primary.action != offer.action)).collect();
    }
    Ok(true)
}

fn stable_identity(selection: &prep::Selection) -> String {
    format!("{}:{}", selection.module.sha256, selection.class.id)
}

pub(super) fn current_operation(m: &Manager) -> Result<Option<Value>> {
    let receipt = optional(&m.root.join("operator/latest.json"))?;
    let Some(mut receipt) = receipt.as_object().cloned().map(Value::Object) else {
        return Ok(None);
    };
    receipt
        .as_object_mut()
        .unwrap()
        .remove("owned_service_pause");
    if let Some(application)=receipt["application"].as_str() {
        if let Some(intent)=vendor_intent(m,application)? {
            if receipt["operation"] == intent.active_operation {
                receipt["journey"]=serde_json::to_value(current_journey(m,&intent)?)?;
            }
        }
    }
    if let Some(owner)=receipt["renderer_owner"].as_str() {
        let retained=renderer_continuation::load(m,owner)?;
        if receipt["operation"].as_str()==Some(retained.intent.active_operation.as_str()) {
            receipt["journey"]=serde_json::to_value(renderer_continuation::current_journey(m,&retained)?)?;
        }
    }
    if let Some(pause) = owned_service_pause_with(m, &receipt, |operation| {
        Ok(
            readiness::bounded_user_unit_state(&format!("linux-vst-bridge-operator-{operation}"))
                .is_ok_and(|state| state == "active"),
        )
    })? {
        receipt["owned_service_pause"] = pause;
    }
    Ok(Some(receipt))
}

// The immutable installer outcome remains in history. The current notification
// follows the same positively verified app handoff as its exact Setup owner.
pub(super) fn project_current_operation(mut receipt: Option<Value>,
    setups: &[ui::InstallerSetup], records: &[onboarding::Record]) -> Result<Option<Value>> {
    let Some(current) = receipt.as_mut() else { return Ok(None); };
    for record in records {
        let Some(intent) = record.automatic.as_ref().filter(|i|i.vendor.is_none()) else { continue; };
        if current["operation"].as_str() != Some(intent.active_operation.as_str()) { continue; }
        if let Some(journey) = setups.iter().find(|setup|setup.installer == record.installer
            && setup.environment.as_deref() == Some(record.id.as_str()))
            .and_then(|setup|setup.journey.as_ref())
            .filter(|journey|journey.phase == ui::InstallationPhase::AwaitingApplication
                && journey.failure.is_none()) {
            current["journey"] = serde_json::to_value(journey)?;
        }
    }
    Ok(receipt)
}

fn owned_service_pause_with(
    m: &Manager,
    receipt: &Value,
    is_live: impl FnOnce(&str) -> Result<bool>,
) -> Result<Option<Value>> {
    let Some(operation) = receipt["operation"].as_str().filter(|id| valid_hex(id, 32)) else {
        return Ok(None);
    };
    if !matches!(
        receipt["state"].as_str(),
        Some("waiting" | "running" | "vendor_running")
    ) {
        return Ok(None);
    }
    let Ok(journey) = serde_json::from_value::<ui::InstallationJourney>(receipt["journey"].clone())
    else {
        return Ok(None);
    };
    if matches!(
        journey.phase,
        ui::InstallationPhase::Available | ui::InstallationPhase::NeedsAttention
    ) {
        return Ok(None);
    }
    let Some(saved) = resume_record(m)? else {
        return Ok(None);
    };
    if saved.schema != 2 || !saved.resume || saved.software != software_record(m)?.manager.sha256 {
        return Ok(None);
    }
    let request: ui::Request = read_json(&job_dir(m, operation)?.join("request.json"))?;
    if request.schema != ui::OPERATOR_SCHEMA {
        return Ok(None);
    }
    let bound = match &request.action {
        ui::Action::SoftwareInstall { daw, .. } if saved.owner_operation == operation => {
            *daw == journey.daw
        }
        ui::Action::VendorSoftwareInstall { application, .. } | ui::Action::VendorApplicationOpen { application } => {
            vendor_intent(m, application)?.is_some_and(|intent| intent.active_operation == operation
                && intent.owner == saved.owner_operation && intent.journey.daw == journey.daw
                && intent.vendor.as_ref().is_some_and(|vendor| saved.vendor_operation.as_deref()
                    == Some(vendor.operation.as_str())))
        }
        ui::Action::EnvironmentRefresh { .. } | ui::Action::EnvironmentRefreshResume { .. } => {
            let owner=match &request.action {
                ui::Action::EnvironmentRefresh {..}=>operation,
                ui::Action::EnvironmentRefreshResume {operation}=>operation.as_str(),
                _=>unreachable!(),
            };
            let renderer_owner=renderer_continuation::owns(m,owner)?;
            let intent=if renderer_owner {renderer_continuation::load(m,owner)?.intent}
                else {environment_refresh::load(m,owner)?.intent};
            intent.active_operation==operation && intent.journey.daw==journey.daw
                && intent.journey.phase==journey.phase
                && (saved.owner_operation==operation && (!renderer_owner || renderer_continuation::scan_custody(m,&saved).is_ok())
                    || intent.work.iter().any(|work|
                    work.active_operation.as_deref()==Some(saved.owner_operation.as_str())))
        }
        ui::Action::RendererSoftwareInstall {daw,..}=> {
            let intent=renderer_continuation::load(m,operation)?.intent;
            intent.active_operation==operation && intent.journey.daw==*daw && journey.daw==*daw
                && intent.journey.phase==journey.phase
                && (saved.owner_operation==operation && (saved.vendor_operation.as_deref()==Some(operation)
                        || saved.vendor_operation.is_none() && renderer_continuation::scan_custody(m,&saved).is_ok())
                    || intent.work.iter().any(|work|work.active_operation.as_deref()==Some(saved.owner_operation.as_str())))
        }
        ui::Action::SoftwareInstall { .. }
        | ui::Action::SoftwareInstallResume { .. } => {
            let mut owners=vec![];
            for record in onboarding::history_records(m)? {
                if let Some(intent)=record_intent(m,&record)? { owners.push((record,intent)); }
            }
            owners.iter().any(|(record,intent)| {
                let same_request = match &request.action {
                    ui::Action::SoftwareInstall { installer, daw } => {
                        *installer == record.installer && *daw == intent.journey.daw
                    }
                    ui::Action::VendorSoftwareInstall { application, daw } => {
                        intent
                            .vendor
                            .as_ref()
                            .is_some_and(|v| v.application == *application)
                            && *daw == intent.journey.daw
                    }
                    ui::Action::SoftwareInstallResume { onboarding } => *onboarding == record.id,
                    _ => false,
                };
                same_request
                    && intent.active_operation == operation
                    && intent.journey.daw == journey.daw
                    && intent.journey.phase == journey.phase
                    && (saved.owner_operation == operation
                        || intent
                            .vendor
                            .as_ref()
                            .is_some_and(|v| v.operation == saved.owner_operation)
                        || intent.work.iter().any(|work| {
                            work.active_operation.as_ref() == Some(&saved.owner_operation)
                        }))
            })
        }
        _ => false,
    };
    if !bound || !is_live(operation)? {
        return Ok(None);
    }
    if saved.owner_operation != operation {
        let child = optional(&job_dir(m, &saved.owner_operation)?.join("result.json"))?;
        if matches!(child["state"].as_str(), Some("completed" | "refused")) {
            return Ok(None);
        }
    }
    // Display-only explanation of an existing live owner's suspension. Capacity
    // and cleanup remain unknown until the existing restore owner verifies them.
    Ok(Some(
        json!({"schema":1,"owner_operation":saved.owner_operation,
        "automatic_operation":operation}),
    ))
}

pub(super) fn failure(stage: ui::InstallationFailureStage, reason: impl ToString) -> ui::InstallationFailure {
    ui::InstallationFailure {
        stage,
        code: reason.to_string().chars().take(512).collect(),
        reason: match stage {
            ui::InstallationFailureStage::EnvironmentPreparation => {
                "The installation location or required runtime could not be prepared."
            }
            ui::InstallationFailureStage::VendorInstallation => {
                "The vendor installation has not finished safely."
            }
            ui::InstallationFailureStage::Discovery => {
                "The installed instruments or effects could not be found or read."
            }
            ui::InstallationFailureStage::Inspection => {
                "This plug-in could not be checked for connection to your DAW."
            }
            ui::InstallationFailureStage::Preparation => {
                "The native DAW connection could not be prepared."
            }
            ui::InstallationFailureStage::Publication => {
                "The connection could not be placed where your DAW can find it."
            }
        }
        .into(),
        retryable: true,
    }
}
fn save(m: &Manager, record: &dyn InstallationTarget, intent: &Intent, operation: &str) -> Result<()> {
    if record.renderer() {return renderer_continuation::save(m,record.environment(),intent,operation);}
    if refresh_target(record,intent) {
        return environment_refresh::save(m,record.environment(),intent,operation);
    }
    if let Some(vendor) = &intent.vendor {
        let request: ui::Request = read_json(&job_dir(m, &intent.owner)?.join("request.json"))?;
        require(matches!(&request.action, ui::Action::VendorSoftwareInstall { application, .. }
            | ui::Action::VendorApplicationOpen { application } if application == &vendor.application),
            "automatic_vendor_owner_binding")?;
        vendor.selected.verify()?;
        let mut retained = intent.clone();
        retained.active_operation = operation.into();
        atomic_json(&checkpoint(m, &intent.owner)?, &retained)?;
        return write_operation(m, operation,
            &json!({"schema":2,"operation":operation,"state":if intent.journey.phase == ui::InstallationPhase::VendorRunning {"vendor_running"} else {"running"},
                "environment":record.id(),"application":vendor.application,"journey":intent.journey}), false);
    }
    let record = record.installation().ok_or("automatic_installation_record_missing")?;
    let _guard = m.lock("onboarding.lock")?;
    let mut current = onboarding::retained_environment_record(m, &record.id)?
        .ok_or("automatic_installation_record_missing")?;
    require(
        current.installer == record.installer
            && current.creation_operation == record.creation_operation,
        "automatic_installation_binding_changed",
    )?;
    let owner_ok=current.automatic.as_ref().is_none_or(|saved| saved.owner == intent.owner);
    require(owner_ok,"automatic_installation_owner_changed")?;
    let mut retained = intent.clone();
    retained.active_operation = operation.into();
    current.automatic = Some(retained);
    atomic_json(
        &onboarding::directory(m, &record.id)?.join("record.json"),
        &current,
    )?;
    let state = if intent.journey.phase == ui::InstallationPhase::VendorRunning {
        "vendor_running"
    } else {
        "running"
    };
    write_operation(
        m,
        operation,
        &json!({"schema":2,"operation":operation,"state":state,
        "onboarding":record.id,"journey":intent.journey}),
        false,
    )
}

pub(super) fn execute(
    m: &Manager,
    action: &ui::Action,
    operation: &str,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
) -> Result<Value> {
    // One user intent has one executor across all child operations. This lock
    // never blocks readback or holds registry/canonical ownership during GUI work.
    let installer = match action {
        ui::Action::SoftwareInstall { installer, .. } => installer.clone(),
        ui::Action::VendorSoftwareInstall { application, .. } => {
            vendor_cli::application(m, application)?.installer_sha256.ok_or("This program has no installer provenance")?
        }
        ui::Action::SoftwareInstallResume { onboarding } => {
            onboarding::retained_environment_record(m, onboarding)?
                .ok_or("automatic_installation_record_missing")?
                .installer
        }
        _ => return Err("automatic_installation_action".into()),
    };
    require(valid_hex(&installer, 64), "installer_identity")?;
    let _intent_owner = m.lock(&format!("automatic-install-{installer}.lock"))?;
    if let ui::Action::SoftwareInstall { daw, .. } = action {
        write_operation(
            m,
            operation,
            &json!({"schema":2,"operation":operation,
            "state":"running","journey":ui::InstallationJourney {
                daw:*daw,phase:ui::InstallationPhase::PreparingEnvironment,
                plugins:vec![],failure:None,
            }}),
            false,
        )?;
    }
    let result = execute_owned(m, action, operation, capacity_read, timeout, waits);
    if let Err(error) = &result {
        fail_early_preparation(m, operation, error)?;
    }
    result
}

fn fail_early_preparation(m: &Manager, operation: &str, error: impl ToString) -> Result<()> {
    let receipt = optional(&job_dir(m, operation)?.join("result.json"))?;
    if receipt.get("onboarding").is_none() {
        if let Ok(mut journey) =
            serde_json::from_value::<ui::InstallationJourney>(receipt["journey"].clone())
        {
            if journey.phase != ui::InstallationPhase::PreparingEnvironment {
                return Ok(());
            }
            journey.phase = ui::InstallationPhase::NeedsAttention;
            journey.failure = Some(failure(
                ui::InstallationFailureStage::EnvironmentPreparation,
                error,
            ));
            write_operation(
                m,
                operation,
                &json!({"schema":2,"operation":operation,
                "state":"running","journey":journey}),
                false,
            )?;
        }
    }
    Ok(())
}

fn execute_owned(
    m: &Manager,
    action: &ui::Action,
    operation: &str,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
) -> Result<Value> {
    let (record, mut intent): (Box<dyn InstallationTarget>, Intent) = match action {
        ui::Action::SoftwareInstall { installer, daw } => {
            let history = onboarding::history_records(m)?;
            let existing: Vec<_> = history
                .iter()
                .filter(|r| r.installer == *installer)
                .collect();
            require(
                existing.len() <= 1,
                "automatic_installation_existing_attempt_requires_resume",
            )?;
            let record = if let Some(record) = existing.first() {
                require(
                    record.installation_operation.is_none() && record.automatic.is_none(),
                    "automatic_installation_existing_attempt_requires_resume",
                )?;
                onboarding::load(m, &record.id)?
            } else {
                let runners = onboarding::runners(m)?;
                let runner = match onboarding::default_runtime(m, &runners)? {
                    Some((key, _)) => key,
                    None => {
                        let value = execute_with_receipt_policy(
                            m,
                            &ui::Action::RuntimeInstall {},
                            Some(operation),
                            capacity_read,
                            timeout,
                            waits,
                        )?;
                        value["runner_key"]
                            .as_str()
                            .ok_or("automatic_runtime_selection_missing")?
                            .to_owned()
                    }
                };
                let value = execute_with_receipt_policy(
                    m,
                    &ui::Action::InstallerEnvironmentCreate {
                        installer: installer.clone(),
                        runner,
                    },
                    Some(operation),
                    capacity_read,
                    timeout,
                    waits,
                )?;
                onboarding::load(
                    m,
                    value["onboarding"]
                        .as_str()
                        .ok_or("automatic_environment_missing")?,
                )?
            };
            let sw = software(m)?;
            let prior_selections = prep::selections(m, &sw.host, &sw.source_sha256)?
                .iter()
                .filter(|s| s.environment.id == record.id)
                .map(stable_identity)
                .collect::<Vec<_>>();
            let scan_path = m.root.join("inventory").join(format!("{}.json", record.id));
            let prior_modules = if scan_path.exists() {
                read_json::<inventory::Scan>(&scan_path)?
                    .modules
                    .into_iter()
                    .map(|module| module.artifact.sha256)
                    .collect()
            } else {
                vec![]
            };
            let mut intent = Intent {
                owner: operation.into(),
                active_operation: operation.into(),
                vendor: None,
                scan: None,
                prior_selections,
                prior_modules,
                failed_modules: vec![],
                work: vec![],
                predecessors:vec![],
                previous:None,
                journey: ui::InstallationJourney {
                    daw: *daw,
                    phase: ui::InstallationPhase::PreparingEnvironment,
                    plugins: vec![],
                    failure: None,
                },
            };
            save(m, &record, &intent, operation)?;
            let installed = execute_with_receipt_policy(
                m,
                &ui::Action::InstallerStart {
                    onboarding: record.id.clone(),
                },
                Some(operation),
                capacity_read,
                timeout,
                waits,
            );
            if let Err(error) = installed {
                intent.journey.phase = ui::InstallationPhase::NeedsAttention;
                intent.journey.failure = Some(failure(
                    ui::InstallationFailureStage::VendorInstallation,
                    &error,
                ));
                save(m, &record, &intent, operation)?;
                return Err(error);
            }
            (Box::new(onboarding::load(m, &record.id)?), intent)
        }
        ui::Action::VendorSoftwareInstall { application, daw } => {
            let app = vendor_cli::selection::select(m, application)?;
            app.verify(&m.root)?;
            let record = app.environment.clone();
            let prior = vendor_intent(m, application)?;
            let restarting = prior.as_ref().map(|i| restartable_vendor(m, i))
                .transpose()?.unwrap_or(false);
            if let Some(initial) = onboarding::retained_environment_record(m, &record.id)? {
                require(initial.automatic.as_ref().is_none_or(|i|
                    i.vendor.is_some() || i.journey.phase == ui::InstallationPhase::Available
                    || empty_bootstrap_handoff(m, &initial, i, application).unwrap_or(false)),
                    "automatic_installation_existing_attempt_requires_resume")?;
                require(onboarding::retired(&onboarding::result(m, &initial)?),
                    "installer_retirement_required")?;
            }
            let sw = software(m)?;
            let prior_selections = prep::selections(m, &sw.host, &sw.source_sha256)?
                .iter()
                .filter(|s| s.environment.id == record.id)
                .map(stable_identity)
                .collect();
            let path = m.root.join("inventory").join(format!("{}.json", record.id));
            let prior_modules = if path.exists() {
                read_json::<inventory::Scan>(&path)?
                    .modules
                    .into_iter()
                    .map(|m| m.artifact.sha256)
                    .collect()
            } else {
                vec![]
            };
            let mut predecessors=vec![];
            for (class,entry) in m.registry()?.classes {
                if entry.registration.environment.id==record.id {
                    let selected=entry.managed_revision.as_ref().ok_or("vendor_existing_publication_authority_missing")?;
                    predecessors.push(m.load_revision(&class,selected)?);
                }
            }
            let selected =
                control_artifact(vendor_cli::directory(m, application)?.join("application.json"))?;
            let mut intent = Intent {
                owner: operation.into(),
                active_operation: operation.into(),
                vendor: Some(VendorWork {
                    application: application.clone(),
                    selected,
                    operation: random_id()?,
                    result: None,
                }),
                scan: None,
                prior_selections,
                prior_modules,
                failed_modules: vec![],
                work: vec![],
                predecessors,
                previous:None,
                journey: ui::InstallationJourney {
                    daw: *daw,
                    phase: ui::InstallationPhase::PreparingEnvironment,
                    plugins: vec![],
                    failure: None,
                },
            };
            if let Some(prior) = prior {
                if restarting {
                    intent = restart_vendor_intent(m, &prior, operation, *daw)?;
                } else if prior.journey.phase != ui::InstallationPhase::Available
                    && !empty_vendor_handoff(m, &prior)? {
                    // Continue the exact interrupted work, including its original
                    // inventory, selected siblings and partial additions.
                    intent = prior;
                } else {
                    intent.previous = Some(retain_vendor_intent(m, &prior)?);
                }
            }
            save(m, &record, &intent, operation)?;
            let record = reserve_vendor_revision(m, &record, &intent)?;
            run_vendor(m, &record, &mut intent, operation, timeout, waits)?;
            (Box::new(record), intent)
        }
        ui::Action::SoftwareInstallResume { onboarding: id } => {
            let record = onboarding::retained_environment_record(m, id)?
                .ok_or("automatic_installation_record_missing")?;
            let intent = record_intent(m, &record)?
                .ok_or("automatic_installation_intent_missing")?;
            require(
                valid_hex(&intent.owner, 32),
                "automatic_installation_intent_binding",
            )?;
            let mut intent = intent;
            let record: Box<dyn InstallationTarget> = if intent.vendor.is_some() {
                Box::new(reserve_vendor_revision(m, &record, &intent)?)
            } else { Box::new(record) };
            if intent.vendor.is_some() {run_vendor(m,record.as_ref(),&mut intent,operation,timeout,waits)?;}
            let record: Box<dyn InstallationTarget> = if intent.vendor.is_none() && record.installation().unwrap().installation_operation.is_none() {
                // The prior worker died or was refused before reserving vendor
                // work. Retry the launch in this exact retained environment.
                intent.journey.phase = ui::InstallationPhase::PreparingEnvironment;
                intent.journey.failure = None;
                save(m, record.as_ref(), &intent, operation)?;
                let launched = execute_with_receipt_policy(
                    m,
                    &ui::Action::InstallerStart {
                        onboarding: record.id().to_owned(),
                    },
                    Some(operation),
                    capacity_read,
                    timeout,
                    waits,
                );
                if let Err(error) = launched {
                    intent.journey.phase = ui::InstallationPhase::NeedsAttention;
                    intent.journey.failure = Some(failure(
                        ui::InstallationFailureStage::VendorInstallation,
                        &error,
                    ));
                    save(m, record.as_ref(), &intent, operation)?;
                    return Err(error);
                }
                Box::new(onboarding::retained_environment_record(m, record.id())?
                    .ok_or("automatic_installation_record_missing")?)
            } else {
                record
            };
            (record, intent)
        }
        _ => return Err("automatic_installation_action".into()),
    };
    let result = continue_installed(
        m,
        record.as_ref(),
        &mut intent,
        operation,
        capacity_read,
        timeout,
        waits,
    );
    if let Err(error) = &result {
        intent.journey.phase = ui::InstallationPhase::NeedsAttention;
        if intent.journey.failure.is_none() {
            let mut detail = failure(ui::InstallationFailureStage::Discovery, error);
            if detail.code == "automatic_discovery_retry_inventory_changed"
                || detail.code == "automatic_discovery_retry_module_changed"
            {
                detail.reason="The discovery records changed after this task. Review its retained details before retrying.".into();
                detail.retryable = false;
            }
            if intent.journey.plugins.iter().any(|p|p.failure.as_ref().is_some_and(|f|!f.retryable)) {
                detail.retryable=false;
                detail.reason="An existing plug-in needs attention after this installation. Its saved setup is retained.".into();
            }
            intent.journey.failure = Some(detail);
        }
        save(m, record.as_ref(), &intent, operation)?;
    }
    result
}

pub(super) fn restartable_vendor(m:&Manager,intent:&Intent)->Result<bool> {
    restartable_vendor_with(m,intent,vendor_cli::live)
}
fn restartable_vendor_with(m:&Manager,intent:&Intent,live:impl FnOnce(&str)->Result<bool>)->Result<bool> {
    let Some(vendor)=&intent.vendor else{return Ok(false)};
    if intent.journey.phase!=ui::InstallationPhase::NeedsAttention
        || !intent.journey.failure.as_ref().is_some_and(|f|f.stage==ui::InstallationFailureStage::VendorInstallation)
        || intent.scan.is_some() || !intent.work.is_empty(){return Ok(false)}
    let receipt=optional(&job_dir(m,&intent.active_operation)?.join("result.json"))?;
    if !matches!(receipt["state"].as_str(),Some("completed"|"refused")) || live(&vendor.application)? {return Ok(false)}
    let result=vendor_cli::result(m,&vendor.application,&vendor.operation)?;
    Ok(result.schema==3 && result.retired() && matches!(result.state,
        vendor_application::OperationState::Failed|vendor_application::OperationState::Cancelled))
}
fn restart_vendor_intent(m:&Manager,prior:&Intent,operation:&str,daw:ui::DawChoice)->Result<Intent> {
    let mut retained=prior.clone();
    let vendor=retained.vendor.as_mut().ok_or("automatic_vendor_intent_missing")?;
    let result=vendor_cli::result(m,&vendor.application,&vendor.operation)?;
    require(result.retired() && !result.successful(),"vendor_restart_requires_retired_failure")?;
    let directory=job_dir(m,&prior.owner)?;
    let result_path=directory.join("retained-vendor-result.json");
    if !result_path.exists(){atomic_json(&result_path,&result)?;}
    require(read_json::<vendor_application::OperationResult>(&result_path)?.operation_id.as_deref()==Some(vendor.operation.as_str()),"vendor_restart_result_changed")?;
    vendor.result=Some(control_artifact(result_path)?);
    let previous_path=directory.join("retained-automatic.json");
    if !previous_path.exists(){atomic_json(&previous_path,&retained)?;}
    require(serde_json::to_value(read_json::<Intent>(&previous_path)?)?==serde_json::to_value(&retained)?,"vendor_restart_intent_changed")?;
    let mut next=prior.clone();next.previous=Some(control_artifact(previous_path)?);
    next.owner=operation.into();next.active_operation=operation.into();next.journey.daw=daw;
    next.journey.phase=ui::InstallationPhase::PreparingEnvironment;next.journey.failure=None;
    let vendor=next.vendor.as_mut().unwrap();vendor.operation=random_id()?;vendor.result=None;
    Ok(next)
}
fn reserve_vendor_revision(m:&Manager,record:&dyn InstallationTarget,intent:&Intent)->Result<Environment> {
    let _ownership=m.lock("onboarding.lock")?;
    let _registry=m.lock("registry.lock")?;
    let directory=job_dir(m,&intent.owner)?;
    if directory.join("vendor-maintenance.json").exists(){
        let binding=environment_revision::vendor_record(m,&intent.owner)?;
        require(binding.application==intent.vendor.as_ref().unwrap().application
            && binding.vendor_operation==intent.vendor.as_ref().unwrap().operation,"automatic_vendor_revision_changed")?;
        let current:Environment=read_json(&binding.environment.root.join("environment.json"))?;
        if current==binding.before {
            atomic_json(&binding.environment.root.join("environment.json"),&binding.environment)?;
        } else { require(current==binding.environment,"automatic_vendor_revision_changed")?; }
        drop(_registry);drop(_ownership);
        return Ok(binding.environment)
    }
    m.require_inactive(None)?;
    require(vendor_retired(m)? && onboarding::all_retired(m)?,"operator_vendor_cleanup_unconfirmed")?;
    environment_runtime::require_settled(m,record.id())?;
    let before=environment_install::target(m,record.id())?;
    let mut after=before.clone();after.revision=after.revision.checked_add(1).ok_or("environment_revision_exhausted")?;
    let vendor=intent.vendor.as_ref().ok_or("automatic_vendor_intent_missing")?;
    let binding=environment_revision::VendorBinding{schema:1,operation:intent.owner.clone(),
        application:vendor.application.clone(),selected:vendor.selected.clone(),vendor_operation:vendor.operation.clone(),before,environment:after};
    atomic_json(&directory.join("vendor-maintenance.json"),&binding)?;
    atomic_json(&binding.environment.root.join("environment.json"),&binding.environment)?;
    drop(_registry);drop(_ownership);
    Ok(binding.environment)
}
fn require_vendor_success(m: &Manager, vendor: &VendorWork) -> Result<()> {
    vendor.selected.verify()?;
    require(
        vendor.selected.path
            == vendor_cli::directory(m, &vendor.application)?.join("application.json"),
        "vendor_application_selection_changed",
    )?;
    let result = vendor_cli::result(m, &vendor.application, &vendor.operation)?;
    require(
        !vendor_cli::live(&vendor.application)? && result.retired(),
        "operator_vendor_cleanup_unconfirmed",
    )?;
    require(
        result.successful(),
        "vendor_application_task_failed_or_stopped",
    )
}
fn run_vendor(
    m: &Manager,
    record: &dyn InstallationTarget,
    intent: &mut Intent,
    operation: &str,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
) -> Result<()> {
    let vendor = intent
        .vendor
        .clone()
        .ok_or("automatic_vendor_intent_missing")?;
    vendor.selected.verify()?;
    let app = vendor_cli::application(m, &vendor.application)?;
    app.verify(&m.root)?;
    require(
        app.environment == *record.environment(),
        "vendor_application_environment_changed",
    )?;
    if vendor.result.is_none() {
        let directory = vendor_cli::directory(m, &vendor.application)?;
        let job = optional(&directory.join("operation.json"))?;
        intent.journey.phase = ui::InstallationPhase::VendorRunning;
        intent.journey.failure = None;
        save(m, record, intent, operation)?;
        let result = if job["operation_id"] == vendor.operation {
            // The retained supervisor outlives the frontend/outer worker. Never
            // launch a second hub or discard its partial/continuing work.
            while vendor_cli::live(&vendor.application)? {
                std::thread::sleep(Duration::from_millis(500));
            }
            vendor_cli::reconcile(m,&vendor.application,&vendor.operation)?;
            let result = vendor_cli::result(m, &vendor.application, &vendor.operation)?;
            require(result.retired(), "operator_vendor_cleanup_unconfirmed")?;
            resume_interrupted(m)?;
            Ok(json!({"result":result}))
        } else {
            open_vendor_owned(
                m,
                &vendor.application,
                operation,
                &vendor.operation,
                timeout,
                waits,
                || Ok(()),
            )
        };
        if let Err(error) = result {
            intent.journey.phase = ui::InstallationPhase::NeedsAttention;
            intent.journey.failure = Some(failure(
                ui::InstallationFailureStage::VendorInstallation,
                &error,
            ));
            save(m, record, intent, operation)?;
            return Err(error);
        }
        let result = control_artifact(directory.join("operation-result.json"))?;
        intent.vendor.as_mut().unwrap().result = Some(result);
        save(m, record, intent, operation)?;
    }
    if let Err(error) = require_vendor_success(m, intent.vendor.as_ref().unwrap()) {
        intent.journey.phase = ui::InstallationPhase::NeedsAttention;
        let mut failed = failure(ui::InstallationFailureStage::VendorInstallation, &error);
        failed.retryable = false;
        failed.reason =
            "The application session stopped or failed. Its files and results are retained.".into();
        intent.journey.failure = Some(failed);
        save(m, record, intent, operation)?;
        return Err(error);
    }
    Ok(())
}

fn continue_installed(
    m: &Manager,
    record: &dyn InstallationTarget,
    intent: &mut Intent,
    operation: &str,
    capacity_read: &dyn Fn() -> Option<CapacityReadback>,
    timeout: Duration,
    waits: &mut Vec<ui::LockFacts>,
) -> Result<Value> {
    continue_with(
        m,
        record,
        intent,
        operation,
        &mut |action, owner, progress| {
            execute_with_receipt_policy_progress(
                m,
                action,
                Some(owner),
                capacity_read,
                timeout,
                waits,
                progress,
            )
        },
    )
}

pub(super) fn continue_refresh(m:&Manager,environment:&Environment,intent:&mut Intent,
    operation:&str,capacity:&dyn Fn()->Option<CapacityReadback>,timeout:Duration,
    waits:&mut Vec<ui::LockFacts>)->Result<Value> {
    continue_installed(m,environment,intent,operation,capacity,timeout,waits)
}
#[cfg(test)]
pub(super) fn continue_refresh_fixture(m:&Manager,environment:&Environment,intent:&mut Intent,
    operation:&str,run:&mut ExecuteStage<'_>)->Result<Value> {
    continue_with(m,environment,intent,operation,run)
}
pub(super) fn continue_renderer(m:&Manager,environment:&Environment,intent:&mut Intent,
    operation:&str,capacity:&dyn Fn()->Option<CapacityReadback>,timeout:Duration,
    waits:&mut Vec<ui::LockFacts>)->Result<Value> {
    continue_installed(m,&RendererTarget(environment.clone()),intent,operation,capacity,timeout,waits)
}
#[cfg(test)]
pub(super) fn continue_renderer_fixture(m:&Manager,environment:&Environment,intent:&mut Intent,
    operation:&str,run:&mut ExecuteStage<'_>)->Result<Value> {
    continue_with(m,&RendererTarget(environment.clone()),intent,operation,run)
}

pub(super) fn cleanup_connections(m:&Manager,intent:&Intent)->Result<()> {
    for work in &intent.work {
        if let Some(check)=&work.check {
            prep::build::cleanup_work(m,check)?;
            if let Some(active)=&work.active_operation {resume_owned(m,active)?;}
            resume_owned(m,check)?;
            refuse_unfinished(m,check,"automatic_installation_worker_terminated")?;
        }
    }
    Ok(())
}

fn continue_with(
    m: &Manager,
    record: &dyn InstallationTarget,
    intent: &mut Intent,
    operation: &str,
    run: &mut ExecuteStage<'_>,
) -> Result<Value> {
    let installation = if let Some(vendor) = &intent.vendor {
        let result = vendor
            .result
            .as_ref()
            .ok_or("vendor_application_completion_missing")?;
        result.verify()?;
        require_vendor_success(m, vendor)?;
        Some(json!({"state":"completed","cleanup_confirmed":true,"owned_live":0}))
    } else if record.renderer() {
        Some(renderer_continuation::completed_receipt(m,intent,record.environment())?)
    } else if let Some(record)=record.installation() {
        Some(onboarding::result(m,record)?)
    } else {
        environment_refresh::require_owner(m,record.environment(),intent)?;
        None
    };
    if let Some(installation)=&installation {require(onboarding::retired(installation),"installer_retirement_required")?;}
    let installation=installation.unwrap_or(Value::Null);
    if installation["state"] == "cancelled" {
        intent.journey.phase = ui::InstallationPhase::NeedsAttention;
        let mut stopped = failure(
            ui::InstallationFailureStage::VendorInstallation,
            "installation_cancelled",
        );
        stopped.reason =
            "Installation was stopped. Its partial files and results are retained.".into();
        stopped.retryable = false;
        intent.journey.failure = Some(stopped);
        return Err("installation_cancelled".into());
    }
    intent.journey.failure = None;
    if intent.scan.is_none() {
        intent.journey.phase = ui::InstallationPhase::Discovering;
        save(m, record, intent, operation)?;
        let scanned = run(
            &if intent.vendor.is_none() && record.installation().is_none() {
                ui::Action::EnvironmentRescan {environment:record.id().into()}
            } else if intent.vendor.is_some() {
                ui::Action::EnvironmentInstallerScan {
                    environment: record.id().to_owned(),
                }
            } else {
                ui::Action::InstallerScan {
                onboarding: record.id().to_owned(),
                }
            },
            operation,
            &mut |_| {},
        )?;
        let mut collected=intent.clone();
        collected.scan = Some(
            scanned["scan"]
                .as_str()
                .ok_or("automatic_scan_identity_missing")?
                .into(),
        );
        collect_new_results(m, record, &mut collected)?;
        *intent=collected;
        save(m, record, intent, operation)?;
    } else if !intent.failed_modules.is_empty() {
        retry_failed_modules(m, record, intent, operation, run)?;
    }
    finish_connections(m,record,intent,operation,run,&installation)
}
fn finish_connections(m:&Manager,record:&dyn InstallationTarget,intent:&mut Intent,operation:&str,
    run:&mut ExecuteStage<'_>,installation:&Value)->Result<Value> {
    if refresh_target(record,intent) {
        environment_refresh::require_preserved_baseline(m,intent)?;
    }
    if intent.journey.plugins.is_empty() {
        require(intent.vendor.is_some() || record.renderer(), "no_supported_instruments_or_effects_discovered")?;
        // A completed app session is not a DAW connection. Retain its result and
        // keep the real app next step actionable, without an empty Available claim.
        intent.journey.phase=ui::InstallationPhase::NeedsAttention;
        intent.journey.failure=Some(failure(ui::InstallationFailureStage::Discovery,
            "no_supported_instruments_or_effects_discovered"));
        save(m,record,intent,operation)?;
        return Ok(json!({"environment":record.id(),"journey":intent.journey,
            "available":false,"no_plugins_added":true,"installer_result":installation,"automatic":true}));
    }
    require(
        intent
            .work
            .iter()
            .all(|work| work.result_index < intent.journey.plugins.len()),
        "automatic_selection_results_binding",
    )?;
    intent.journey.phase = ui::InstallationPhase::PreparingPlugins;
    save(m, record, intent, operation)?;
    // Shared environment inspection/preparation/publication must be serialized.
    for index in 0..intent.work.len() {
        let result_index = intent.work[index].result_index;
        if intent.journey.plugins[result_index].stage == ui::InstallationPluginStage::Available {
            continue;
        }
        let result = connect_class(m, record, intent, index, operation, run);
        if let Err(error) = result {
            let stage = match intent.journey.plugins[result_index].stage {
                ui::InstallationPluginStage::Inspecting => ui::InstallationFailureStage::Inspection,
                ui::InstallationPluginStage::Preparing => ui::InstallationFailureStage::Preparation,
                ui::InstallationPluginStage::Pending => ui::InstallationFailureStage::Discovery,
                _ => ui::InstallationFailureStage::Publication,
            };
            if intent.journey.plugins[result_index].stage
                != ui::InstallationPluginStage::ReplacementRequired
            {
                intent.journey.plugins[result_index].stage =
                    ui::InstallationPluginStage::NeedsAttention;
            }
            intent.journey.plugins[result_index].failure = Some(failure(stage, error));
            save(m, record, intent, operation)?;
        }
    }
    if refresh_target(record,intent) {
        environment_refresh::require_preserved_baseline(m,intent)?;
    }
    let complete = intent
        .journey
        .plugins
        .iter()
        .all(|p| p.stage == ui::InstallationPluginStage::Available);
    intent.journey.phase = if complete {
        ui::InstallationPhase::Available
    } else {
        ui::InstallationPhase::NeedsAttention
    };
    save(m, record, intent, operation)?;
    let mut result=json!({"journey":intent.journey,"available":complete && !intent.journey.plugins.is_empty(),
        "no_plugins_added":intent.work.iter().all(|w|w.predecessor.is_some()),
        "installer_result":installation,"automatic":true});
    result[if intent.vendor.is_none() && record.installation().is_none() {"environment"} else {"onboarding"}]=json!(record.id());
    Ok(result)
}

fn collect_new_results(
    m: &Manager,
    record: &dyn InstallationTarget,
    intent: &mut Intent,
) -> Result<()> {
    let sw = software(m)?;
    let scan: inventory::Scan =
        read_json(&m.root.join("inventory").join(format!("{}.json", record.id())))?;
    require(
        scan.environment == *record.environment()
            && scan.host == sw.host
            && scan.host_source_sha256 == sw.source_sha256,
        "automatic_scan_binding_changed",
    )?;
    intent.scan = Some(scan.id.clone());
    let mut selected = prep::selections(m, &sw.host, &sw.source_sha256)?
        .into_iter()
        .filter(|s| s.environment.id == record.id())
        .collect::<Vec<_>>();
    selected.sort_by_key(|s| s.id().unwrap_or_default());
    for selection in selected {
        let id = selection.id()?;
        let predecessor=intent.predecessors.iter().find(|p|p.class_id==selection.class.id);
        if refresh_target(record,intent)
            && intent.prior_selections.contains(&stable_identity(&selection)) {
            environment_refresh::require_preserved(m,intent,&selection)?;
            if !intent.journey.plugins.iter().any(|p|p.selection==id) {
                intent.journey.plugins.push(ui::InstallationPlugin {selection:id,class_id:selection.class.id.clone(),
                    name:selection.class.name.clone(),stage:ui::InstallationPluginStage::Available,failure:None});
            }
            continue;
        }
        if (intent
            .prior_selections
            .contains(&stable_identity(&selection)) && predecessor.is_none())
            || intent.work.iter().any(|w| w.selection == id)
        {
            continue;
        }
        let result_index = intent.journey.plugins.len();
        intent.journey.plugins.push(ui::InstallationPlugin {
            selection: id.clone(),
            class_id: selection.class.id.clone(),
            name: selection.class.name.clone(),
            stage: ui::InstallationPluginStage::Pending,
            failure: None,
        });
        let predecessor=predecessor.map(|p| {
            let path=m.root.join("publications").join(&p.class_id).join("revisions").join(&p.id).join("revision.json");
            digest(&path).map(|sha256|RevisionRef{id:p.id.clone(),sha256})
        }).transpose()?;
        intent.work.push(ClassWork {
            predecessor,
            selection: id,
            result_index,
            check: None,
            candidate: None,
            active_operation: None,
        });
    }
    for original in &intent.predecessors {
        if intent.journey.plugins.iter().any(|p|p.class_id == original.class_id) { continue; }
        if scan.modules.iter().any(|module| module.artifact == original.registration.module
            && (module.quarantine_reason.is_some() || module.inspection_error.is_some())) { continue; }
        let mut missing = failure(ui::InstallationFailureStage::Discovery, "existing_class_missing_after_vendor_work");
        missing.reason = "This existing plug-in was not found after the application finished. Its saved setup is retained.".into();
        missing.retryable = false;
        intent.journey.plugins.push(ui::InstallationPlugin { selection: original.registration.module.sha256.clone(),
            class_id: original.class_id.clone(), name: original.registration.metadata.name.clone(),
            stage: ui::InstallationPluginStage::NeedsAttention, failure: Some(missing) });
    }
    for (module_index, module) in scan.modules.iter().enumerate() {
        if (intent.prior_modules.contains(&module.artifact.sha256)
            && !intent.predecessors.iter().any(|p|p.registration.module.sha256 == module.artifact.sha256))
            || intent
                .failed_modules
                .iter()
                .any(|f| f.module_index == module_index)
        {
            continue;
        }
        // Complete factory discovery can intentionally require a class choice.
        // The shared executor chooses each exact class below; that observation
        // is not an additional failed module once per-class work is available.
        if module.quarantine_reason.is_none() && module.inspection_error.is_some()
            && module.classes.iter().filter(|class|class.category=="Audio Module Class").count()>1
            && inventory::inspection_hint(module)?.is_some() {continue;}
        let Some(reason) = module
            .quarantine_reason
            .as_ref()
            .or(module.inspection_error.as_ref())
        else {
            continue;
        };
        let result_index = intent.journey.plugins.len();
        let name = module
            .artifact
            .path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("Installed plug-in module")
            .to_owned();
        intent.journey.plugins.push(ui::InstallationPlugin {
            selection: module.artifact.sha256.clone(),
            class_id: String::new(),
            name,
            stage: ui::InstallationPluginStage::NeedsAttention,
            failure: Some(failure(ui::InstallationFailureStage::Discovery, reason)),
        });
        intent.failed_modules.push(FailedModule {
            module_index,
            result_index,
            artifact: module.artifact.clone(),
            report: module.report.clone(),
        });
    }
    Ok(())
}

fn retry_failed_modules(
    m: &Manager,
    record: &dyn InstallationTarget,
    intent: &mut Intent,
    operation: &str,
    run: &mut ExecuteStage<'_>,
) -> Result<()> {
    intent.journey.phase = ui::InstallationPhase::Discovering;
    save(m, record, intent, operation)?;
    let pending = intent
        .failed_modules
        .iter()
        .map(|f| f.module_index)
        .collect::<Vec<_>>();
    for module_index in pending {
        let path = m.root.join("inventory").join(format!("{}.json", record.id()));
        let scan: inventory::Scan = read_json(&path)?;
        let module = scan
            .modules
            .get(module_index)
            .ok_or("automatic_failed_module_missing")?;
        let retained = intent
            .failed_modules
            .iter()
            .find(|f| f.module_index == module_index)
            .ok_or("automatic_failed_module_missing")?;
        require(
            intent.scan.as_deref() == Some(scan.id.as_str())
                && module.artifact == retained.artifact
                && module.report == retained.report,
            "automatic_discovery_retry_inventory_changed",
        )?;
        let action = ui::Action::QuarantinedModuleRetry {
            environment: record.id().to_owned(),
            scan: scan.id.clone(),
            module_index,
            module_sha256: module.artifact.sha256.clone(),
            report_sha256: module.report.sha256.clone(),
        };
        if let Err(error) = run(&action, operation, &mut |_| {}) {
            let row = intent
                .failed_modules
                .iter()
                .find(|f| f.module_index == module_index)
                .unwrap()
                .result_index;
            intent.journey.plugins[row].failure =
                Some(failure(ui::InstallationFailureStage::Discovery, error));
            save(m, record, intent, operation)?;
            continue;
        }
        let current: inventory::Scan = read_json(&path)?;
        let module = current
            .modules
            .get(module_index)
            .ok_or("automatic_failed_module_missing")?;
        intent.scan = Some(current.id.clone());
        if let Some(retained) = intent
            .failed_modules
            .iter_mut()
            .find(|f| f.module_index == module_index)
        {
            require(
                module.artifact == retained.artifact,
                "automatic_discovery_retry_module_changed",
            )?;
            retained.report = module.report.clone();
        }
        if module.quarantine_reason.is_none() && module.inspection_error.is_none() {
            let failed = intent
                .failed_modules
                .iter()
                .position(|f| f.module_index == module_index)
                .unwrap();
            let row = intent.failed_modules.remove(failed).result_index;
            intent.journey.plugins.remove(row);
            for work in &mut intent.work {
                if work.result_index > row {
                    work.result_index -= 1;
                }
            }
            for failure in &mut intent.failed_modules {
                if failure.result_index > row {
                    failure.result_index -= 1;
                }
            }
            collect_new_results(m, record, intent)?;
        }
        save(m, record, intent, operation)?;
    }
    Ok(())
}

fn connect_class(
    m: &Manager,
    record: &dyn InstallationTarget,
    intent: &mut Intent,
    index: usize,
    operation: &str,
    run: &mut ExecuteStage<'_>,
) -> Result<()> {
    let result_index = intent.work[index].result_index;
    let sw = software(m)?;
    let selection = prep::select(
        m,
        &intent.work[index].selection,
        &sw.host,
        &sw.source_sha256,
    )?;
    require(
        selection.environment.id == record.id(),
        "automatic_selection_environment_changed",
    )?;
    intent.journey.plugins[result_index].failure = None;
    if let Some(predecessor) = intent.work[index].predecessor.clone() {
        let original = intent.predecessors.iter().find(|p|p.id == predecessor.id)
            .ok_or("unchanged_publication_predecessor_missing")?;
        if selection.module != original.registration.module || selection.class.id != original.class_id {
            intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::ReplacementRequired;
            return Err("existing_working_configuration_requires_separate_change_review".into());
        }
        let expected_action=ui::Action::CompatibilityRevalidate{selection:selection.id()?,
            expected_current:ui::PublicationIdentity{id:predecessor.id.clone(),sha256:predecessor.sha256.clone()}};
        let mut retained_candidate=None;
        if let Some(child)=intent.work[index].check.as_ref() {
            if let Some(candidate)=preparation_cli::revalidation_candidate_checkpoint(m,child,&expected_action)? {
                require(candidate.selection==selection,"unchanged_publication_candidate_changed")?;
                if prep::selected_publication_candidate_record(m,&candidate)? {
                    intent.work[index].candidate=Some(candidate.id()?);
                    intent.journey.plugins[result_index].stage=ui::InstallationPluginStage::Available;
                    save(m,record,intent,operation)?;return Ok(())
                }
                retained_candidate=Some(candidate.id()?);
            }
        }
        intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Inspecting;
        let child = random_id()?;
        let action=expected_action;
        intent.work[index].check = Some(child.clone());
        intent.work[index].active_operation = Some(child.clone());
        let dir = job_dir(m, &child)?;
        private_dir(&dir)?;
        atomic_json(&dir.join("request.json"), &ui::Request { schema: ui::OPERATOR_SCHEMA,
            state_token: token(m)?, action: action.clone() })?;
        if let Some(candidate)=retained_candidate {
            preparation_cli::retain_revalidation_candidate_checkpoint(m,&child,&action,&candidate)?;
        }
        save(m, record, intent, operation)?;
        let mut progress_error = None;
        let result = run(&action, &child, &mut |stage| {
            intent.journey.plugins[result_index].stage = if stage == "publication" {
                ui::InstallationPluginStage::Publishing
            } else { ui::InstallationPluginStage::Preparing };
            if let Err(error) = save(m, record, intent, operation) { progress_error = Some(error); }
        });
        let receipt = match &result {
            Ok(value) => json!({"schema":1,"operation":child,"state":"completed",
                "parent_operation":operation,"action":action,"result":value}),
            Err(error) => json!({"schema":1,"operation":child,"state":"refused",
                "parent_operation":operation,"action":action,"reason":error.to_string()}),
        };
        write_operation(m, &child, &receipt, false)?;
        if let Some(error) = progress_error { return Err(error); }
        let result = result?;
        let candidate = result["candidate"].as_str().ok_or("unchanged_publication_candidate_missing")?;
        intent.work[index].candidate = Some(candidate.into());
        intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Available;
        save(m, record, intent, operation)?;
        return Ok(());
    }
    // Initial intent may never silently replace an existing product/build/context.
    if let Some(entry) = m.registry()?.classes.get(&selection.class.id) {
        let ours = intent.work[index].candidate.as_ref().is_some_and(|id| {
            prep::candidate_record(m, id)
                .and_then(|c| prep::selected_publication_candidate_record(m, &c))
                .is_ok_and(|selected| selected)
        });
        if ours
            && entry.registration.environment.id == record.id()
            && entry.registration.module == selection.module
        {
            intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Available;
            save(m, record, intent, operation)?;
            return Ok(());
        }
        intent.journey.plugins[result_index].stage =
            ui::InstallationPluginStage::ReplacementRequired;
        return Err("existing_working_configuration_requires_separate_change_review".into());
    }
    if intent.work[index].candidate.is_none() {
        intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Inspecting;
        save(m, record, intent, operation)?;
        let view = prep::view(m, &selection, &sw.host, &sw.source_sha256)?;
        require(
            view.current_revision.is_none(),
            "existing_working_configuration_requires_separate_change_review",
        )?;
        let original = ui::Action::CompatibilityCheck {
            selection: selection.id()?,
            audio_layout: view.recommended_audio_layout.clone(),
            recipe: prep::build::recipe(m)?.sha256,
            predecessor: view.candidate.clone(),
        };
        let source = intent.work[index].check.clone();
        let resumable = if let Some(source) = &source {
            prep::guided_check_stage(m, source, "intent")?.is_some()
                && prep::guided_check_stage(m, source, "completed")?.is_none()
        } else {
            false
        };
        let action = if resumable {
            ui::Action::CompatibilityResumeCheck {
                operation: source.clone().unwrap(),
            }
        } else {
            original
        };
        let child = random_id()?;
        let dir = job_dir(m, &child)?;
        private_dir(&dir)?;
        atomic_json(
            &dir.join("request.json"),
            &ui::Request {
                schema: ui::OPERATOR_SCHEMA,
                state_token: token(m)?,
                action: action.clone(),
            },
        )?;
        if !resumable {
            intent.work[index].check = Some(child.clone());
        }
        intent.work[index].active_operation = Some(child.clone());
        save(m, record, intent, operation)?;
        let mut progress_error = None;
        let checked = run(&action, &child, &mut |stage| {
            if stage == "inspection" {
                intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Preparing;
                if let Err(error) = save(m, record, intent, operation) {
                    progress_error = Some(error);
                }
            }
        });
        if let Some(error) = progress_error {
            return Err(error);
        }

        let receipt = match &checked {
            Ok(value) => json!({"schema":1,"operation":child,"state":"completed","action":action,
                "parent_operation":operation,"result":value}),
            Err(error) => json!({"schema":1,"operation":child,"state":"refused","action":action,
                "parent_operation":operation,"reason":error.to_string()}),
        };
        write_operation(m, &child, &receipt, false)?;
        if checked.is_err() {
            if let Some(completed) = prep::guided_check_stage(
                m,
                intent.work[index].check.as_ref().unwrap(),
                "completed",
            )? {
                if completed["stage"] == "candidate_preparation_failed" {
                    intent.journey.plugins[result_index].stage =
                        ui::InstallationPluginStage::Preparing;
                }
            }
        }
        let value = checked?;
        let candidate = value["candidate"]
            .as_str()
            .ok_or("automatic_preparation_candidate_missing")?;
        intent.work[index].candidate = Some(candidate.into());
        intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Preparing;
        save(m, record, intent, operation)?;
    }
    let candidate = intent.work[index]
        .candidate
        .clone()
        .ok_or("automatic_preparation_candidate_missing")?;
    let prepared = prep::candidate_record(m, &candidate)?;
    require(
        prepared.selection == selection,
        "automatic_candidate_selection_changed",
    )?;
    intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Publishing;
    save(m, record, intent, operation)?;
    let published = run(
        &ui::Action::CompatibilityPublishTest {
            candidate,
            expected_current: None,
        },
        operation,
        &mut |_| {},
    )?;
    require(
        !published["publication"].is_null(),
        "automatic_publication_missing",
    )?;
    require(
        prep::selected_publication_candidate_record(m, &prepared)?,
        "automatic_publication_not_current",
    )?;
    intent.journey.plugins[result_index].stage = ui::InstallationPluginStage::Available;
    save(m, record, intent, operation)
}

pub(super) fn project_revalidation(m: &Manager, products: &mut [ui::Product],
    records: &[onboarding::Record]) -> Result<()> {
    let receipt = optional(&m.root.join("operator/latest.json"))?;
    let Some(operation) = receipt["operation"].as_str().filter(|s|valid_hex(s,32)) else {return Ok(())};
    if !matches!(receipt["state"].as_str(),Some("waiting"|"running"|"vendor_running")) {
        return Ok(());
    }
    let application = receipt["application"].as_str();
    let owned = if let Some(owner)=receipt["renderer_owner"].as_str() {
        Some(renderer_continuation::load(m,owner)?.intent)
    } else {application.map(|app|vendor_intent(m, app)).transpose()?.flatten()};
    let Some(intent) = owned.as_ref().into_iter().chain(records.iter().filter_map(|r|r.automatic.as_ref()))
        .find(|i|i.active_operation == operation && (i.vendor.is_some() || receipt["renderer_owner"]==i.owner)
            && i.journey.phase == ui::InstallationPhase::PreparingPlugins) else {return Ok(())};
    for work in &intent.work {
        let Some(predecessor) = &work.predecessor else {continue};
        let Some(stage) = intent.journey.plugins.get(work.result_index) else {continue};
        if stage.failure.is_some() || !matches!(stage.stage, ui::InstallationPluginStage::Pending
            | ui::InstallationPluginStage::Inspecting | ui::InstallationPluginStage::Preparing
            | ui::InstallationPluginStage::Publishing) {continue}
        let Some(original) = intent.predecessors.iter().find(|p|p.id == predecessor.id) else {continue};
        let registry = m.registry()?;
        if registry.classes.get(&original.class_id).and_then(|e|e.managed_revision.as_ref()) != Some(predecessor) {
            continue;
        }
        for product in products.iter_mut().filter(|p|p.class_id == original.class_id
            && p.environment == original.registration.environment.id
            && p.module_sha256 == original.registration.module.sha256) {
            if product.details["module_binding_valid"] != true
                || product.details["runner_binding_valid"] != true
                || product.details["native_binding_valid"] != true
                || product.details["publication_valid"] != true {continue}
            product.details["automatic_revalidation"] = json!({"operation":operation,
                "predecessor":{"id":predecessor.id,"sha256":predecessor.sha256}});
        }
    }
    Ok(())
}
pub(super) fn project_from(
    m: &Manager,
    setups: &mut [ui::InstallerSetup],
    busy: Option<&str>,
    records: &[onboarding::Record],
) -> Result<()> {
    project_from_with(m,setups,busy,records,&mut |selector|vendor_cli::live(selector))
}
fn project_from_with(m:&Manager,setups:&mut [ui::InstallerSetup],busy:Option<&str>,
    records:&[onboarding::Record],vendor_live:&mut dyn FnMut(&str)->Result<bool>)->Result<()> {
    let receipt = optional(&m.root.join("operator/latest.json"))?;
    let early =
        if let Some(operation) = receipt["operation"].as_str().filter(|id| valid_hex(id, 32)) {
            let request = serde_json::from_value::<ui::Request>(optional(
                &job_dir(m, operation)?.join("request.json"),
            )?)
            .ok()
            .filter(|request| request.schema == ui::OPERATOR_SCHEMA);
            let journey =
                serde_json::from_value::<ui::InstallationJourney>(receipt["journey"].clone()).ok();
            request.zip(journey).filter(|(request, journey)| {
                matches!(&request.action,
            ui::Action::SoftwareInstall { daw, .. } if daw == &journey.daw)
                    && receipt.get("onboarding").is_none()
                    && matches!(
                        journey.phase,
                        ui::InstallationPhase::PreparingEnvironment
                            | ui::InstallationPhase::NeedsAttention
                    )
                    && matches!(
                        receipt["state"].as_str(),
                        Some("queued" | "waiting" | "running" | "refused")
                    )
            })
        } else {
            None
        };
    for setup in setups {
        let record = setup
            .environment
            .as_ref()
            .and_then(|id| records.iter().find(|r| r.id == *id));
        if let Some(record) = record.filter(|r| r.automatic.is_some()) {
            let intent = record_intent(m,record)?.ok_or("automatic_installation_intent_missing")?;
            if project_handoff(m, record, &intent, setup, busy)? { continue; }
            let mut journey = current_journey(m,&intent)?;
            let receipt = optional(&job_dir(m, &intent.active_operation)?.join("result.json"))?;
            if receipt["state"] == "vendor_running" {
                journey.phase = ui::InstallationPhase::VendorRunning;
            }
            setup.journey = Some(journey);
            if let Some(vendor) = &intent.vendor {
                if vendor_live(&vendor.application)? {
                    setup.primary = Some(action(
                        "Show the application window",
                        ui::Action::VendorApplicationFocus {
                            application: vendor.application.clone(),
                        },
                        None,
                    ));
                    setup.secondary = vec![action(
                        "Stop this application session",
                        ui::Action::VendorApplicationStop {
                            application: vendor.application.clone(),
                        },
                        None,
                    )];
                }
            }
            setup.status = if onboarding::result(m, record)?["state"] == "cancelled" {
                "Installation was stopped. Its partial files and results are retained."
            } else {
                match setup.journey.as_ref().unwrap().phase {
                    ui::InstallationPhase::VendorRunning => "Continue in the installer window.",
                    ui::InstallationPhase::Available => {
                        if intent.journey.plugins.is_empty() {"Application session finished. No plug-ins were added."}
                        else {"Installed plug-ins are available through the native DAW connection."}
                    }
                    ui::InstallationPhase::NeedsAttention => {
                        "Some installation work needs attention; available plug-ins stay available."
                    }
                    ui::InstallationPhase::AwaitingApplication => "Continue in the installed app.",
                    _ => "Connecting the installed plug-ins automatically.",
                }
            }
            .into();
            if !matches!(
                setup.primary.as_ref().map(|p| &p.action),
                Some(ui::Action::InstallerFocus { .. } | ui::Action::VendorApplicationFocus { .. })
            ) || receipt["state"] != "vendor_running"
            {
                setup.primary = None;
            }
            if setup.journey.as_ref().unwrap().phase == ui::InstallationPhase::Available {
                setup.secondary.clear();
            } else if setup.journey.as_ref().unwrap().phase == ui::InstallationPhase::AwaitingApplication {
                setup.primary=Some(action("Open the installed app",ui::Action::VendorSoftwareInstall {
                    application:intent.vendor.as_ref().ok_or("automatic_vendor_intent_missing")?.application.clone(),
                    daw:intent.journey.daw },busy));
                setup.status="No instruments or effects were installed. Open the app to install them.".into();
                setup.secondary.clear();
            } else if intent.journey.phase == ui::InstallationPhase::NeedsAttention
                && intent
                    .journey
                    .failure
                    .as_ref()
                    .is_none_or(|failure| failure.retryable)
                && matches!(
                    optional(&job_dir(m, &intent.active_operation)?.join("result.json"))?["state"]
                        .as_str(),
                    Some("completed" | "refused")
                )
                && (intent.vendor.is_some()
                    || record.installation_operation.is_none()
                    || (onboarding::retired(&onboarding::result(m, record)?)
                        && onboarding::result(m, record)?["state"] != "cancelled"))
            {
                setup.primary = Some(action(
                    if record.installation_operation.is_none() {
                        "Retry installation"
                    } else {
                        "Retry unfinished connection"
                    },
                    ui::Action::SoftwareInstallResume {
                        onboarding: record.id.clone(),
                    },
                    busy,
                ));
            }
        } else if record.is_none_or(|r| r.installation_operation.is_none()) {
            if let Some((request, journey)) = early.as_ref().filter(|(request, _)| {
                matches!(&request.action,
                ui::Action::SoftwareInstall {installer,..} if installer == &setup.installer)
            }) {
                setup.journey = Some(journey.clone());
                setup.status = if journey.phase == ui::InstallationPhase::NeedsAttention {
                    "The installation could not be prepared. Retry the same installation."
                } else {
                    "Getting ready to install."
                }
                .into();
                setup.secondary.clear();
                setup.primary = if receipt["state"] == "refused" {
                    Some(action("Retry installation", request.action.clone(), busy))
                } else {
                    None
                };
                continue;
            }
            setup.primary = Some(action(
                "Install and connect",
                ui::Action::SoftwareInstall {
                    installer: setup.installer.clone(),
                    daw: ui::DawChoice::Other,
                },
                busy,
            ));
        }
    }
    Ok(())
}

pub(super) fn finish(m: &Manager, operation: &str, action: &ui::Action) -> Result<()> {
    if matches!(action,ui::Action::RendererSoftwareInstall {..})
        || matches!(action,ui::Action::EnvironmentRefreshResume {operation} if renderer_continuation::owns(m,operation)?) {
        return renderer_continuation::finish(m,operation,action);
    }
    if matches!(action,ui::Action::EnvironmentRefresh {..} | ui::Action::EnvironmentRefreshResume {..}) {
        return environment_refresh::finish(m,operation,action);
    }
    if !matches!(
        action,
        ui::Action::SoftwareInstall { .. } | ui::Action::VendorSoftwareInstall { .. }
            | ui::Action::VendorApplicationOpen { .. } | ui::Action::SoftwareInstallResume { .. }
    ) {
        return Ok(());
    }
    fail_early_preparation(
        m,
        operation,
        "Automatic installation preparation was interrupted.",
    )?;
    let mut retained=vec![];
    for record in onboarding::history_records(m)? {
        if let Some(intent)=record_intent(m,&record)? {
            retained.push((Box::new(record) as Box<dyn InstallationTarget>,intent));
        }
    }
    if let ui::Action::VendorSoftwareInstall { application, .. } | ui::Action::VendorApplicationOpen { application } = action {
        if let Some(intent) = vendor_intent(m, application)? {
            retained.retain(|(_, prior)|prior.owner != intent.owner);
            retained.push((Box::new(vendor_cli::application(m, application)?.environment), intent));
        }
    }
    for (record, intent) in retained {
        let belongs = intent.active_operation == operation;
        if !belongs {
            continue;
        }
        if record.installation().is_some_and(|r|r.installation_operation.as_deref() == Some(operation)) {
            if onboarding::live(operation)? {
                let _ = onboarding::stop(m, record.id(), operation);
            }
            onboarding::mark_dead(m, record.installation().unwrap())?;
        }
        let unfinished =
            optional(&job_dir(m, operation)?.join("result.json"))?["state"] != "completed";
        if unfinished && intent.journey.phase != ui::InstallationPhase::Available
            && intent.journey.failure.is_none() {
            let mut interrupted = intent.clone();
            interrupted.journey.phase = ui::InstallationPhase::NeedsAttention;
            interrupted.journey.failure = Some(failure(
                ui::InstallationFailureStage::Preparation,
                "Automatic connection was interrupted. Resume the unfinished step.",
            ));
            save(m, record.as_ref(), &interrupted, operation)?;
        }
        cleanup_connections(m,&intent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_vendor_restart_retains_original_scope_and_prior_terminal_result() {
        for state in ["failed","cancelled"] {
            let (f,c,record,mut intent)=setup();
            let _after=vendor_edge(&f,&c,&record,&mut intent);
            let vendor=intent.vendor.as_ref().unwrap().clone();
            let directory=vendor_cli::directory(&f.m,&vendor.application).unwrap();
            let app=vendor_cli::application(&f.m,&vendor.application).unwrap();
            atomic_json(&directory.join("operation.json"),&vendor_application::Operation{
                application:app,report:directory.join("operation-result.json"),mode:vendor_application::LaunchMode::Normal,operation_id:vendor.operation.clone(),launcher:None}).unwrap();
            atomic_json(&directory.join("operation-result.json"),&json!({"schema":3,"operation_id":vendor.operation,
                "state":state,"launcher_exit":1,"owned_live":0,"cleanup_confirmed":true,
                "error":"stopped vendor task","discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
            intent.prior_selections=vec!["original-module:original-class".into()];
            intent.prior_modules=vec!["aa".repeat(32)];
            intent.journey.phase=ui::InstallationPhase::NeedsAttention;
            let mut failure=failure(ui::InstallationFailureStage::VendorInstallation,"stopped");failure.retryable=false;
            intent.journey.failure=Some(failure);
            write_operation(&f.m,&intent.active_operation,&json!({"schema":1,"operation":intent.active_operation,"state":"refused"}),false).unwrap();
            assert!(restartable_vendor_with(&f.m,&intent,|_|Ok(false)).unwrap());
            assert!(!restartable_vendor_with(&f.m,&intent,|_|Ok(true)).unwrap());
            // Vendor work left a module and somebody rescanned it. A restart
            // must retain the original baseline, so partial additions remain new.
            let next=restart_vendor_intent(&f.m,&intent,&random_id().unwrap(),ui::DawChoice::Reaper).unwrap();
            assert_eq!(next.prior_selections,intent.prior_selections);
            assert_eq!(next.prior_modules,intent.prior_modules);
            assert_eq!(next.predecessors,intent.predecessors);
            assert!(next.scan.is_none());assert_ne!(next.vendor.as_ref().unwrap().operation,vendor.operation);
            assert_eq!(next.journey.daw,ui::DawChoice::Reaper);
            let previous=next.previous.as_ref().unwrap();previous.verify().unwrap();
            let retained:Intent=read_json(&previous.path).unwrap();
            assert_eq!(retained.journey.failure.unwrap().code,"stopped");
            let result:vendor_application::OperationResult=read_json(&retained.vendor.unwrap().result.unwrap().path).unwrap();
            assert!(result.retired());assert!(!result.successful());
            let mut resumed=next;
            collect_new_results(&f.m,&record,&mut resumed).unwrap();
            assert!(!resumed.work.is_empty(),"Partial additions cannot be excluded as a fresh baseline");
        }
    }
    #[test]
    fn empty_successful_vendor_visit_keeps_an_actionable_app_handoff() {
        let (f,record,_)=vendor_cli::selection::tests::installed_app();
        let (selector,mut intent)=owned_app_visit(&f,&record);
        let directory=vendor_cli::directory(&f.m,&selector).unwrap();
        let vendor=intent.vendor.as_ref().unwrap().clone();
        atomic_json(&directory.join("operation.json"),&vendor_application::Operation {
            application:vendor_cli::application(&f.m,&selector).unwrap(),report:directory.join("operation-result.json"),
            mode:vendor_application::LaunchMode::Normal,operation_id:vendor.operation.clone(),launcher:None}).unwrap();
        atomic_json(&directory.join("operation-result.json"),&json!({"schema":3,
            "operation_id":vendor.operation,"state":"completed","launcher_exit":0,
            "owned_live":0,"cleanup_confirmed":true,"discarded_diagnostic_bytes":0,"account_posture":"unknown"})).unwrap();
        intent.vendor.as_mut().unwrap().result=Some(control_artifact(directory.join("operation-result.json")).unwrap());
        let before=test_fixture::snapshot(&onboarding::directory(&f.m,&record.id).unwrap());
        readiness::with_unit_state_fixture(&[vendor_cli::unit(&selector).unwrap()],"inactive",|| {
            let owner=intent.owner.clone();
            let returned=finish_connections(&f.m,&record.environment,&mut intent,&owner,
                &mut |_,_,_|panic!("No classes to process"),&json!({"state":"completed"})).unwrap();
            assert_eq!(returned["available"],false);
            assert_eq!(returned["journey"]["phase"],"needs_attention");
            assert!(!serde_json::to_string(&returned).unwrap().contains("awaiting_application"));
            assert_eq!(current_journey(&f.m,&intent).unwrap().phase,ui::InstallationPhase::AwaitingApplication);
            assert_ne!(intent.journey.phase,ui::InstallationPhase::Available);
            assert!(empty_vendor_handoff(&f.m,&intent).unwrap());
            let row=vendor_projection(&f.m,&selector,None).unwrap().unwrap();
            assert!(row.actions.iter().any(|a|a.disabled_reason.is_none()
                && matches!(&a.action,ui::Action::VendorSoftwareInstall {application,..} if application == &selector)));
        });
        assert!(f.m.registry().unwrap().classes.is_empty());
        assert_eq!(test_fixture::snapshot(&onboarding::directory(&f.m,&record.id).unwrap()),before);
    }
    fn vendor_edge(f:&test_fixture::Fixture,c:&prep::Candidate,record:&onboarding::Record,intent:&mut Intent)->Environment {
        let app=vendor_application::Application{schema:2,id:vendor_application::ApplicationId::Selected,
            environment:record.environment.clone(),executable:c.selection.module.clone(),helpers:vec![],
            installer_sha256:Some(record.installer.clone()),installer_result:Some(c.inspection.report.clone()),observed_installer_version:String::new(),user_selection:None};
        let selector=app.selector().unwrap();let directory=vendor_cli::directory(&f.m,&selector).unwrap();private_dir(&directory).unwrap();
        atomic_json(&directory.join("application.json"),&app).unwrap();
        let selected=control_artifact(directory.join("application.json")).unwrap();
        intent.vendor=Some(VendorWork{application:selector.clone(),selected:selected.clone(),operation:random_id().unwrap(),result:None});
        atomic_json(&job_dir(&f.m,&intent.owner).unwrap().join("request.json"),&ui::Request{
            schema:ui::OPERATOR_SCHEMA,state_token:"test".into(),action:ui::Action::VendorSoftwareInstall{application:selector.clone(),daw:intent.journey.daw}}).unwrap();
        save(&f.m,record,intent,&intent.owner).unwrap();
        let mut after=record.environment.clone();after.revision+=1;
        atomic_json(&job_dir(&f.m,&intent.owner).unwrap().join("vendor-maintenance.json"),&environment_revision::VendorBinding{
            schema:1,operation:intent.owner.clone(),application:selector,selected,
            vendor_operation:intent.vendor.as_ref().unwrap().operation.clone(),before:record.environment.clone(),environment:after.clone()}).unwrap();
        after
    }
    #[test]
    fn interrupted_vendor_revision_settles_exact_edge_once_and_preserves_original_install() {
        let (f,c,record,mut intent)=setup();
        let original=fs::read(onboarding::directory(&f.m,&record.id).unwrap().join(format!("{}-result.json",record.installation_operation.as_ref().unwrap()))).unwrap();
        let after=vendor_edge(&f,&c,&record,&mut intent);
        assert_eq!(read_json::<Environment>(&after.root.join("environment.json")).unwrap(),record.environment);
        assert_eq!(reserve_vendor_revision(&f.m,&record,&intent).unwrap(),after);
        assert_eq!(reserve_vendor_revision(&f.m,&record,&intent).unwrap(),after);
        assert_eq!(fs::read(onboarding::directory(&f.m,&record.id).unwrap().join(format!("{}-result.json",record.installation_operation.as_ref().unwrap()))).unwrap(),original);
        let mut foreign=after.clone();foreign.revision+=1;atomic_json(&foreign.root.join("environment.json"),&foreign).unwrap();
        assert_eq!(reserve_vendor_revision(&f.m,&record,&intent).unwrap_err().to_string(),"automatic_vendor_revision_changed");
    }
    fn initial_intent(record: &onboarding::Record) -> Intent {
        Intent { owner:record.creation_operation.clone(), active_operation:record.creation_operation.clone(),
            vendor:None, journey:ui::InstallationJourney { daw:ui::DawChoice::Bitwig,
                phase:ui::InstallationPhase::NeedsAttention, plugins:vec![],
                failure:Some(failure(ui::InstallationFailureStage::Discovery,
                    "no_supported_instruments_or_effects_discovered")) },
            scan:Some(random_id().unwrap()), prior_selections:vec![], prior_modules:vec![],
            failed_modules:vec![], work:vec![], predecessors:vec![], previous:None }
    }
    fn owned_app_visit(f: &test_fixture::Fixture, record: &onboarding::Record) -> (String, Intent) {
        let choice=vendor_cli::selection::choices(&f.m, record).unwrap().into_iter()
            .find(|c|c.image_name == "Main").unwrap();
        let app=choice.application;
        app.verify(&f.m.root).unwrap();
        let selector=app.selector().unwrap();
        let directory=vendor_cli::directory(&f.m,&selector).unwrap();private_dir(&directory).unwrap();
        atomic_json(&directory.join("application.json"),&app).unwrap();
        let owner=random_id().unwrap();private_dir(&job_dir(&f.m,&owner).unwrap()).unwrap();
        atomic_json(&job_dir(&f.m,&owner).unwrap().join("request.json"),&ui::Request {
            schema:ui::OPERATOR_SCHEMA, state_token:"test".into(), action:ui::Action::VendorSoftwareInstall {
                application:selector.clone(), daw:ui::DawChoice::Bitwig }}).unwrap();
        let mut intent=initial_intent(record);intent.owner=owner.clone();intent.active_operation=owner.clone();
        intent.vendor=Some(VendorWork { application:selector.clone(),
            selected:control_artifact(directory.join("application.json")).unwrap(),
            operation:random_id().unwrap(), result:None });
        intent.journey.failure=None;intent.journey.phase=ui::InstallationPhase::VendorRunning;
        save(&f.m,&app.environment,&intent,&owner).unwrap();
        readiness::with_unit_state_fixture(&[vendor_cli::unit(&selector).unwrap()],"inactive",||
            reserve_vendor_revision(&f.m,&app.environment,&intent).unwrap());
        (selector,intent)
    }
    #[test]
    fn terminal_vendor_failure_survives_worker_finish_without_same_session_retry() {
        let (f,record,_)=vendor_cli::selection::tests::installed_app();
        let original=test_fixture::snapshot(&onboarding::directory(&f.m,&record.id).unwrap());
        let (selector,mut intent)=owned_app_visit(&f,&record);
        intent.journey.phase=ui::InstallationPhase::NeedsAttention;
        let mut error=failure(ui::InstallationFailureStage::VendorInstallation,"vendor_cancelled");error.retryable=false;
        intent.journey.failure=Some(error);
        let operation=intent.owner.clone();
        save(&f.m,&record.environment,&intent,&operation).unwrap();
        write_operation(&f.m,&operation,&json!({"schema":1,"operation":operation,"state":"refused"}),false).unwrap();
        finish(&f.m,&operation,&ui::Action::VendorSoftwareInstall { application:selector.clone(),daw:ui::DawChoice::Bitwig }).unwrap();
        let retained=vendor_intent(&f.m,&selector).unwrap().unwrap();
        let failed=retained.journey.failure.as_ref().unwrap();
        assert_eq!(failed.stage,ui::InstallationFailureStage::VendorInstallation);assert_eq!(failed.code,"vendor_cancelled");assert!(!failed.retryable);
        assert_eq!(test_fixture::snapshot(&onboarding::directory(&f.m,&record.id).unwrap()),original);
        assert!(onboarding::retained_environment_record(&f.m,&record.id).unwrap().unwrap().automatic.is_none());
    }
    #[test]
    fn earlier_onboarding_vendor_intent_reads_through_the_same_exact_owner() {
        let (f,mut record,_)=vendor_cli::selection::tests::installed_app();
        let (selector,intent)=owned_app_visit(&f,&record);
        fs::remove_file(checkpoint(&f.m,&intent.owner).unwrap()).unwrap();
        assert_eq!(vendor_intent(&f.m,&selector).unwrap_err().to_string(),"automatic_vendor_intent_missing");
        record.automatic=Some(intent.clone());
        let path=onboarding::directory(&f.m,&record.id).unwrap().join("record.json");
        atomic_json(&path,&record).unwrap();let original=fs::read(&path).unwrap();
        assert_eq!(serde_json::to_value(vendor_intent(&f.m,&selector).unwrap().unwrap()).unwrap(),serde_json::to_value(&intent).unwrap());
        save(&f.m,&record.environment,&intent,&intent.owner).unwrap();
        assert_eq!(fs::read(&path).unwrap(),original);
        assert_eq!(vendor_intent(&f.m,&selector).unwrap().unwrap().owner,intent.owner);
        let mut progressed=intent.clone();
        progressed.scan=Some(random_id().unwrap());progressed.prior_modules=vec!["ae".repeat(32)];
        progressed.journey.phase=ui::InstallationPhase::NeedsAttention;
        progressed.journey.failure=Some(failure(ui::InstallationFailureStage::Preparation,"latest_owned_checkpoint"));
        let resume=random_id().unwrap();private_dir(&job_dir(&f.m,&resume).unwrap()).unwrap();
        save(&f.m,&record.environment,&progressed,&resume).unwrap();
        write_operation(&f.m,&resume,&json!({"schema":1,"operation":resume,"state":"refused"}),false).unwrap();
        finish(&f.m,&resume,&ui::Action::SoftwareInstallResume {onboarding:record.id.clone()}).unwrap();
        let resolved=record_intent(&f.m,&record).unwrap().unwrap();
        assert_eq!(resolved.active_operation,resume);assert_eq!(resolved.prior_modules,progressed.prior_modules);
        assert_eq!(resolved.scan,progressed.scan);
        let mut setup=installer_setup(&record);
        readiness::with_unit_state_fixture(&[vendor_cli::unit(&selector).unwrap()],"inactive",|| {
            project_from_with(&f.m,std::slice::from_mut(&mut setup),None,&[record.clone()],&mut |_|Ok(false)).unwrap();
        });
        assert_eq!(setup.journey.unwrap().failure.unwrap().code,"latest_owned_checkpoint");
        assert_eq!(fs::read(&path).unwrap(),original);
        let mut changed:Intent=read_json(&checkpoint(&f.m,&intent.owner).unwrap()).unwrap();
        changed.vendor.as_mut().unwrap().operation=random_id().unwrap();
        atomic_json(&checkpoint(&f.m,&intent.owner).unwrap(),&changed).unwrap();
        assert_eq!(vendor_intent(&f.m,&selector).unwrap_err().to_string(),"automatic_vendor_intent_binding");
    }
    fn installer_setup(record: &onboarding::Record) -> ui::InstallerSetup {
        ui::InstallerSetup {installer:record.installer.clone(),name:"Vendor App".into(),label_source:"user".into(),byte_size:1,
            format:"exe".into(),imported_at:1,phase:ui::SetupPhase::DiscoveryComplete,journey:None,status:String::new(),
            environment:Some(record.id.clone()),compatibility:None,discovered:vec![],primary:None,secondary:vec![],
            rename:action("Rename",ui::Action::InstallerRename {installer:record.installer.clone(),label:String::new()},None),history:vec![]}
    }
    #[test]
    fn app_only_cards_use_record_observation_and_selection_still_checks_runtime() {
        let (f,mut record,_)=vendor_cli::selection::tests::installed_app();
        let manifest=record.environment.runner.proton.parent().unwrap().join("runtime-tree.json");
        fs::write(&manifest,b"not a runtime tree").unwrap();
        record.environment.runner.files.push(Artifact {path:manifest.clone(),sha256:digest(&manifest).unwrap()});
        atomic_json(&record.environment.root.join("environment.json"),&record.environment).unwrap();
        let intent=initial_intent(&record);record.automatic=Some(intent.clone());
        atomic_json(&onboarding::directory(&f.m,&record.id).unwrap().join("record.json"),&record).unwrap();
        let choices=vendor_cli::selection::choices(&f.m,&record).unwrap();
        let selector=choices[0].application.selector().unwrap();
        let units=choices.iter().map(|c|vendor_cli::unit(&c.application.selector().unwrap()).unwrap()).collect::<Vec<_>>();
        let before=test_fixture::snapshot(&f.m.root);
        readiness::with_unit_state_fixture(&units,"inactive",|| {
            let mut setup=installer_setup(&record);
            project_from_with(&f.m,std::slice::from_mut(&mut setup),None,std::slice::from_ref(&record),&mut |_|Ok(false)).unwrap();
            assert_eq!(setup.journey.unwrap().phase,ui::InstallationPhase::AwaitingApplication);
            assert_eq!(setup.secondary.iter().filter(|a|matches!(a.action,ui::Action::VendorSoftwareInstall {..})
                && a.disabled_reason.is_none()).count(),2);
        });
        assert_eq!(test_fixture::snapshot(&f.m.root),before,"Readback does not warm or rewrite runtime records");
        assert!(empty_bootstrap_handoff(&f.m,&record,&intent,&selector).is_err());
        assert!(vendor_cli::selection::select(&f.m,&selector).is_err());
        assert!(!vendor_cli::directory(&f.m,&selector).unwrap().join("application.json").exists());
        fs::remove_file(&choices[0].application.executable.path).unwrap();
        let mut setup=installer_setup(&record);
        project_from_with(&f.m,std::slice::from_mut(&mut setup),None,std::slice::from_ref(&record),&mut |_|Ok(false)).unwrap();
        assert_eq!(setup.journey.unwrap().phase,ui::InstallationPhase::NeedsAttention);
    }

    #[test]
    fn app_only_handoff_is_current_projection_then_settles_home_without_rewriting_history() {
        let (f,mut record,_)=vendor_cli::selection::tests::installed_app();
        let intent=initial_intent(&record);record.automatic=Some(intent);
        let path=onboarding::directory(&f.m,&record.id).unwrap().join("record.json");
        atomic_json(&path,&record).unwrap();let original=fs::read(&path).unwrap();
        let units=vendor_cli::selection::choices(&f.m,&record).unwrap().iter()
            .map(|c|vendor_cli::unit(&c.application.selector().unwrap()).unwrap()).collect::<Vec<_>>();
        readiness::with_unit_state_fixture(&units,"inactive",|| {
            let mut setup=installer_setup(&record);
            project_from_with(&f.m,std::slice::from_mut(&mut setup),None,&[record.clone()],&mut |_|Ok(false)).unwrap();
            assert_eq!(setup.journey.as_ref().unwrap().phase,ui::InstallationPhase::AwaitingApplication);
            assert!(setup.journey.as_ref().unwrap().failure.is_none());
            assert_eq!(setup.secondary.iter().filter(|a|matches!(a.action,ui::Action::VendorSoftwareInstall { .. })).count(),2);
            assert!(setup.primary.is_none());
            let raw=json!({"operation":record.automatic.as_ref().unwrap().active_operation,
                "state":"refused","reason":"no_supported_instruments_or_effects_discovered"});
            let projected=project_current_operation(Some(raw.clone()),std::slice::from_ref(&setup),
                std::slice::from_ref(&record)).unwrap().unwrap();
            assert_eq!(projected["journey"]["phase"],"awaiting_application");
            assert_eq!(projected["state"],raw["state"]);assert_eq!(projected["reason"],raw["reason"]);
            let mut foreign=setup.clone();foreign.environment=Some("ff".repeat(16));
            assert_eq!(project_current_operation(Some(raw.clone()),&[foreign],std::slice::from_ref(&record)).unwrap(),Some(raw.clone()));
            let mut wrong=raw.clone();wrong["operation"]=json!("ab".repeat(16));
            assert_eq!(project_current_operation(Some(wrong.clone()),std::slice::from_ref(&setup),
                std::slice::from_ref(&record)).unwrap(),Some(wrong));
            let (selector,mut continuation)=owned_app_visit(&f,&record);
            continuation.journey.phase=ui::InstallationPhase::Available;
            continuation.journey.plugins.push(ui::InstallationPlugin {name:"Instrument".into(),
                class_id:"cd".repeat(16),selection:"ef".repeat(32),stage:ui::InstallationPluginStage::Available,failure:None});
            save(&f.m,&record.environment,&continuation,&continuation.owner).unwrap();
            let current=onboarding::history_records(&f.m).unwrap();
            let mut setup=installer_setup(&record);
            project_from_with(&f.m,std::slice::from_mut(&mut setup),None,&current,&mut |_|Ok(false)).unwrap();
            assert_eq!(setup.journey.as_ref().unwrap().phase,ui::InstallationPhase::Available);
            assert_eq!(setup.journey.as_ref().unwrap().plugins.len(),1);
            assert!(setup.journey.as_ref().unwrap().failure.is_none());assert!(setup.primary.is_none());assert!(setup.secondary.is_empty());
            assert_eq!(vendor_intent(&f.m,&selector).unwrap().unwrap().owner,continuation.owner);
        });
        assert_eq!(fs::read(path).unwrap(),original);
    }
    #[test]
    fn empty_without_an_app_and_unrelated_failure_do_not_become_a_handoff() {
        let (f,mut record,transaction)=vendor_cli::selection::tests::installed_app();
        record.automatic=Some(initial_intent(&record));
        fs::remove_file(transaction).unwrap();
        let mut setup=installer_setup(&record);
        project_from_with(&f.m,std::slice::from_mut(&mut setup),None,&[record.clone()],&mut |_|Ok(false)).unwrap();
        assert_eq!(setup.journey.unwrap().phase,ui::InstallationPhase::NeedsAttention);
        record.automatic.as_mut().unwrap().journey.failure=Some(failure(ui::InstallationFailureStage::Inspection,"unrelated_failure"));
        let mut setup=installer_setup(&record);
        project_from_with(&f.m,std::slice::from_mut(&mut setup),None,&[record],&mut |_|Ok(false)).unwrap();
        assert_eq!(setup.journey.unwrap().failure.unwrap().code,"unrelated_failure");
    }
    #[test]
    fn vendor_revision_invalidates_then_revalidates_exact_selected_sibling() {
        let (f,c,record,_intent)=setup();
        prep::retain_inspection(&f.m,&c.inspection).unwrap();
        prep::record_candidate(&f.m,&c).unwrap();
        let selected=prep::enable(&f.m,&c,false).unwrap();
        let predecessor=f.m.load_revision(&c.selection.class.id,&selected).unwrap();
        let original=fs::read(f.m.link(&c.selection.class.id).join("Contents/x86_64-linux").join(format!("LVB_{}.so",c.selection.class.id))).unwrap();
        let before=record.environment.clone();
        let mut after=before.clone();after.revision+=1;
        let app=vendor_application::Application {schema:2,id:vendor_application::ApplicationId::Selected,
            environment:before.clone(),executable:c.selection.module.clone(),helpers:vec![],
            installer_sha256:Some(record.installer.clone()),installer_result:Some(c.inspection.report.clone()),observed_installer_version:String::new(),user_selection:None};
        let application=app.selector().unwrap();
        let directory=vendor_cli::directory(&f.m,&application).unwrap();private_dir(&directory).unwrap();
        atomic_json(&directory.join("application.json"),&app).unwrap();
        let choice=control_artifact(directory.join("application.json")).unwrap();
        let op=_intent.owner.clone();
        atomic_json(&job_dir(&f.m,&op).unwrap().join("request.json"),&ui::Request{schema:ui::OPERATOR_SCHEMA,
            state_token:"test".into(),action:ui::Action::VendorSoftwareInstall{application:application.clone(),daw:ui::DawChoice::Bitwig}}).unwrap();
        atomic_json(&job_dir(&f.m,&op).unwrap().join("vendor-maintenance.json"),&environment_revision::VendorBinding{
            schema:1,operation:op.clone(),application:application.clone(),selected:choice,
            vendor_operation:"de".repeat(16),before:before.clone(),environment:after.clone()}).unwrap();
        atomic_json(&after.root.join("environment.json"),&after).unwrap();
        environment_revision::require_owned_revision(&f.m,&before,&after).unwrap();
        assert!(!prep::selected_publication_candidate_record(&f.m,&c).unwrap());
        // A fresh scan retains the identical class/module in the new revision.
        let path=f.m.root.join("inventory").join(format!("{}.json",record.id));
        let mut scan:inventory::Scan=read_json(&path).unwrap();scan.environment=after.clone();scan.id=random_id().unwrap();
        atomic_json(&path,&scan).unwrap();
        let s=prep::selections(&f.m,&c.selection.scanner,&c.selection.scanner_source).unwrap().pop().unwrap();
        let child=random_id().unwrap();
        let action=ui::Action::CompatibilityRevalidate{selection:s.id().unwrap(),expected_current:ui::PublicationIdentity{id:selected.id.clone(),sha256:selected.sha256.clone()}};
        let child_directory=job_dir(&f.m,&child).unwrap();
        private_dir(&child_directory).unwrap();
        atomic_json(&child_directory.join("request.json"),&ui::Request {schema:ui::OPERATOR_SCHEMA,
            state_token:"test".into(),action:action.clone()}).unwrap();
        let mut probe=c.clone();probe.inspection.selection=s.clone();
        let result=preparation_cli::with_automatic_fixture(&probe,||
            preparation_cli::execute(&f.m,&action,&child,||f.m.lock("registry.lock"))).unwrap();
        let refreshed=prep::candidate_record(&f.m,result["candidate"].as_str().unwrap()).unwrap();
        let next:RevisionRef=serde_json::from_value(result["publication"].clone()).unwrap();
        prep::build::cleanup_work(&f.m,&child).unwrap();
        refreshed.native.artifact.verify().unwrap();
        assert_eq!(refreshed.profile.capabilities.compatibility(),predecessor.registration.compatibility);
        assert_eq!(refreshed.native.artifact.sha256,predecessor.registration.native.sha256);
        assert!(prep::guided_checks(&f.m,&s.id().unwrap()).is_ok(),
            "Automatic revalidation must not poison ordinary guided history");
        let mut products=vec![preparation_cli::tests::projection_product(&refreshed)];
        preparation_cli::project(&f.m,&software(&f.m).unwrap(),&mut products,None).unwrap();
        assert_eq!(f.m.load_revision(&c.selection.class.id,&next).unwrap().parent,Some(selected.clone()));
        assert_eq!(f.m.load_revision(&c.selection.class.id,&selected).unwrap(),predecessor);
        assert!(prep::selected_publication_candidate_record(&f.m,&refreshed).unwrap());
        assert_eq!(fs::read(f.m.link(&c.selection.class.id).join("Contents/x86_64-linux").join(format!("LVB_{}.so",c.selection.class.id))).unwrap(),original);
        // The publication committed, but the child/parent terminal receipt did
        // not. Recover the exact current candidate from its durable checkpoint.
        let mut replay=_intent.clone();
        replay.predecessors.push(predecessor.clone());
        replay.journey.plugins.push(ui::InstallationPlugin{selection:s.id().unwrap(),class_id:s.class.id.clone(),name:s.class.name.clone(),stage:ui::InstallationPluginStage::Publishing,failure:None});
        replay.work.push(ClassWork{selection:s.id().unwrap(),predecessor:Some(selected.clone()),result_index:0,check:Some(child.clone()),candidate:None,active_operation:None});
        let current=onboarding::retained_environment_record(&f.m,&record.id).unwrap().unwrap();
        connect_class(&f.m,&current,&mut replay,0,&op,&mut |_,_,_|panic!("Already committed refresh must not run again")).unwrap();
        assert_eq!(replay.journey.plugins[0].stage,ui::InstallationPluginStage::Available);
        assert_eq!(f.m.registry().unwrap().classes[&c.selection.class.id].managed_revision,Some(next.clone()));
        // Content/class drift cannot borrow the old setup's refresh authority.
        let mut changed=s.clone();changed.class.name="Different class metadata".into();
        assert!(prep::revalidate_candidate(&f.m,&predecessor,changed,
            prep::build::existing_runtime(&f.m,&c.recipe_sha256).unwrap(),c.inspection.report.clone(),&random_id().unwrap()).is_err());
        let mut changed_action=action.clone();
        if let ui::Action::CompatibilityRevalidate {selection,..}=&mut changed_action {*selection="ab".repeat(32);}
        assert_eq!(preparation_cli::revalidation_candidate_checkpoint(&f.m,&child,&changed_action).unwrap_err().to_string(),
            "unchanged_publication_request_changed");
        let checkpoint_path=child_directory.join("revalidation-candidate.json");
        let original_checkpoint=fs::read(&checkpoint_path).unwrap();
        let mut checkpoint:Value=serde_json::from_slice(&original_checkpoint).unwrap();
        checkpoint["candidate"]=c.id().unwrap().into();
        atomic_json(&checkpoint_path,&checkpoint).unwrap();
        assert_eq!(preparation_cli::revalidation_candidate_checkpoint(&f.m,&child,&action).unwrap_err().to_string(),
            "unchanged_publication_candidate_changed");
        checkpoint=serde_json::from_slice(&original_checkpoint).unwrap();checkpoint["operation"]="cd".repeat(16).into();
        atomic_json(&checkpoint_path,&checkpoint).unwrap();
        assert_eq!(preparation_cli::revalidation_candidate_checkpoint(&f.m,&child,&action).unwrap_err().to_string(),
            "unchanged_publication_intent_changed");
        fs::write(checkpoint_path,original_checkpoint).unwrap();
        // Corrupt generic guided history remains a real refusal, not skipped.
        prep::retain_guided_check_stage(&f.m,&random_id().unwrap(),"candidate",
            &serde_json::to_value(&action).unwrap(),&json!({"id":refreshed.id().unwrap()})).unwrap();
        assert_eq!(prep::guided_checks(&f.m,&s.id().unwrap()).unwrap_err().to_string(),"guided_check_intent_missing");

    }
    fn worker_receipt(m: &Manager, operation: &str) -> Value {
        optional(&job_dir(m, operation).unwrap().join("result.json")).unwrap()
    }
    fn setup() -> (
        test_fixture::Fixture,
        prep::Candidate,
        onboarding::Record,
        Intent,
    ) {
        let (f, c, _, _) = preparation_cli::tests::guided_fixture_with_role(false);
        let owner = random_id().unwrap();
        private_dir(&job_dir(&f.m, &owner).unwrap()).unwrap();
        let record = onboarding::Record {
            schema: 1,
            id: c.selection.environment.id.clone(),
            installer: "ab".repeat(32),
            environment: c.selection.environment.clone(),
            created_at: 1,
            creation_operation: owner.clone(),
            installation_operation: Some(owner.clone()),
            published: false,
            automatic: None,
            previous_attempt: None,
        };
        let dir = onboarding::directory(&f.m, &record.id).unwrap();
        private_dir(&dir).unwrap();
        atomic_json(&dir.join("record.json"), &record).unwrap();
        atomic_json(
            &dir.join(format!("{owner}-result.json")),
            &json!({"schema":2,"operation":owner,"state":"completed","cleanup_confirmed":true,
                "owned_live":0,"authorization":"not_inspected"}),
        )
        .unwrap();
        let intent = Intent {
            active_operation: owner.clone(),
            vendor: None,
            owner,
            scan: None,
            prior_selections: vec![],
            prior_modules: vec![],
            failed_modules: vec![],
            work: vec![],
            predecessors:vec![],
            previous:None,
            journey: ui::InstallationJourney {
                daw: ui::DawChoice::Bitwig,
                phase: ui::InstallationPhase::Discovering,
                plugins: vec![],
                failure: None,
            },
        };
        (f, c, record, intent)
    }
    fn actual(m: &Manager, action: &ui::Action, operation: &str) -> Result<Value> {
        match action {
            ui::Action::InstallerScan { onboarding } => {
                let scan: inventory::Scan =
                    read_json(&m.root.join("inventory").join(format!("{onboarding}.json")))?;
                Ok(json!({"scan":scan.id}))
            }
            _ => preparation_cli::execute(m, action, operation, || m.lock("registry.lock")),
        }
    }
    #[test]
    fn owned_service_pause_requires_exact_live_owner_and_preserves_real_failures() {
        let (f, candidate, record, mut intent) = setup();
        let operation = intent.owner.clone();
        let request = ui::Request {
            schema: ui::OPERATOR_SCHEMA,
            state_token: token(&f.m).unwrap(),
            action: ui::Action::SoftwareInstall {
                installer: record.installer.clone(),
                daw: intent.journey.daw,
            },
        };
        atomic_json(
            &job_dir(&f.m, &operation).unwrap().join("request.json"),
            &request,
        )
        .unwrap();
        save(&f.m, &record, &intent, &operation).unwrap();
        let receipt = worker_receipt(&f.m, &operation);
        let mut saved = ResumeRecord {
            schema: 2,
            owner_operation: operation.clone(),
            resume: true,
            software: software_record(&f.m).unwrap().manager.sha256,
            vendor_operation: None,
        };
        atomic_json(&f.m.root.join("operator/resume.json"), &saved).unwrap();
        let before = test_fixture::snapshot(&f.m.root);
        let pause = owned_service_pause_with(&f.m, &receipt, |id| {
            assert_eq!(id, operation);
            Ok(true)
        })
        .unwrap()
        .unwrap();
        assert_eq!(pause["automatic_operation"], operation);
        assert_eq!(pause["owner_operation"], operation);
        assert_eq!(
            test_fixture::snapshot(&f.m.root),
            before,
            "Readback must not resume or write"
        );
        assert!(
            !system_from_capacity(None, 0, 0).capacity_available(),
            "A planned pause does not grant capacity"
        );
        assert!(
            owned_service_pause_with(&f.m, &receipt, |_| Ok(false))
                .unwrap()
                .is_none(),
            "Orphaned owner needs recovery"
        );
        for state in ["refused", "completed"] {
            let mut terminal = receipt.clone();
            terminal["state"] = json!(state);
            assert!(owned_service_pause_with(&f.m, &terminal, |_| Ok(true))
                .unwrap()
                .is_none());
        }
        let mut failed = receipt.clone();
        failed["journey"]["phase"] = json!("needs_attention");
        assert!(owned_service_pause_with(&f.m, &failed, |_| Ok(true))
            .unwrap()
            .is_none());
        let matching_software = saved.software.clone();
        saved.software = "cd".repeat(32);
        atomic_json(&f.m.root.join("operator/resume.json"), &saved).unwrap();
        assert!(owned_service_pause_with(&f.m, &receipt, |_| Ok(true))
            .unwrap()
            .is_none());
        saved.software = matching_software;
        saved.resume = false;
        atomic_json(&f.m.root.join("operator/resume.json"), &saved).unwrap();
        assert!(
            owned_service_pause_with(&f.m, &receipt, |_| Ok(true))
                .unwrap()
                .is_none(),
            "An already stopped service is not our pause"
        );
        saved.resume = true;
        saved.owner_operation = random_id().unwrap();
        atomic_json(&f.m.root.join("operator/resume.json"), &saved).unwrap();
        assert!(
            owned_service_pause_with(&f.m, &receipt, |_| Ok(true))
                .unwrap()
                .is_none(),
            "Another owner must remain visible as a problem"
        );

        let child = random_id().unwrap();
        private_dir(&job_dir(&f.m, &child).unwrap()).unwrap();
        intent.work.push(ClassWork {
            predecessor:None,
            selection: candidate.selection.id().unwrap(),
            result_index: 0,
            check: Some(child.clone()),
            candidate: None,
            active_operation: Some(child.clone()),
        });
        save(&f.m, &record, &intent, &operation).unwrap();
        saved.owner_operation = child.clone();
        atomic_json(&f.m.root.join("operator/resume.json"), &saved).unwrap();
        let pause = owned_service_pause_with(&f.m, &receipt, |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(pause["owner_operation"], child);
        assert_eq!(pause["automatic_operation"], operation);
        let mut resume_request = request;
        resume_request.action = ui::Action::SoftwareInstallResume {
            onboarding: record.id.clone(),
        };
        atomic_json(
            &job_dir(&f.m, &operation).unwrap().join("request.json"),
            &resume_request,
        )
        .unwrap();
        assert!(owned_service_pause_with(&f.m, &receipt, |_| Ok(true))
            .unwrap()
            .is_some());
        for state in ["refused", "completed"] {
            atomic_json(
                &job_dir(&f.m, &child).unwrap().join("result.json"),
                &json!({"schema":1,"operation":child,"state":state}),
            )
            .unwrap();
            assert!(
                owned_service_pause_with(&f.m, &receipt, |_| Ok(true))
                    .unwrap()
                    .is_none(),
                "A terminal child's retained suspension needs real recovery"
            );
        }
        fs::remove_file(f.m.root.join("operator/resume.json")).unwrap();
        assert!(
            owned_service_pause_with(&f.m, &receipt, |_| Ok(true))
                .unwrap()
                .is_none(),
            "Restoration clears the display fact"
        );
    }
    #[test]
    fn cold_install_progress_precedes_runtime_work_and_failure_keeps_exact_retry() {
        let (f, _c, record, _) = setup();
        fs::remove_dir_all(onboarding::directory(&f.m, &record.id).unwrap()).unwrap();
        let request = ui::Request {
            schema: ui::OPERATOR_SCHEMA,
            state_token: token(&f.m).unwrap(),
            action: ui::Action::SoftwareInstall {
                installer: record.installer.clone(),
                daw: ui::DawChoice::Reaper,
            },
        };
        let operation = begin_request(&f.m, &request).unwrap();
        let card = ui::InstallerSetup {
            installer: record.installer.clone(),
            name: "Chosen software".into(),
            label_source: "user".into(),
            byte_size: 1,
            format: "exe".into(),
            imported_at: 1,
            phase: ui::SetupPhase::Imported,
            journey: None,
            status: "Install compatibility runtime in Setup, then continue.".into(),
            environment: None,
            compatibility: None,
            discovered: vec![],
            primary: None,
            secondary: vec![],
            rename: action(
                "Rename",
                ui::Action::InstallerRename {
                    installer: record.installer.clone(),
                    label: String::new(),
                },
                None,
            ),
            history: vec![],
        };
        let observed = std::cell::Cell::new(false);
        let capacity = || {
            let receipt = worker_receipt(&f.m, &operation);
            assert_eq!(receipt["journey"]["phase"], "preparing_environment");
            assert_eq!(receipt["journey"]["daw"], "reaper");
            assert!(receipt.get("onboarding").is_none());
            let mut projected = card.clone();
            project_from(&f.m, std::slice::from_mut(&mut projected), None, &[]).unwrap();
            assert_eq!(
                projected.journey.unwrap().phase,
                ui::InstallationPhase::PreparingEnvironment
            );
            assert_eq!(projected.status, "Getting ready to install.");
            assert!(
                projected.primary.is_none(),
                "No duplicate install while acquiring runtime"
            );
            observed.set(true);
            None
        };
        let error = execute(
            &f.m,
            &request.action,
            &operation,
            &capacity,
            Duration::from_millis(20),
            &mut vec![],
        )
        .unwrap_err();
        assert!(
            observed.get(),
            "The actual runtime owner must reach its capacity check"
        );
        let failed = worker_receipt(&f.m, &operation);
        assert_eq!(failed["journey"]["phase"], "needs_attention");
        assert_eq!(
            failed["journey"]["failure"]["stage"],
            "environment_preparation"
        );
        assert_eq!(failed["journey"]["failure"]["code"], error.to_string());
        finish(&f.m, &operation, &request.action).unwrap();
        refuse_unfinished(&f.m, &operation, "worker finished").unwrap();
        let mut projected = card.clone();
        project_from(&f.m, std::slice::from_mut(&mut projected), None, &[]).unwrap();
        let retry = projected.primary.unwrap();
        assert_eq!(retry.action, request.action);
        assert!(
            retry.disabled_reason.is_none(),
            "Retired worker must not stay busy"
        );
        assert_eq!(
            worker_receipt(&f.m, &operation)["journey"]["failure"],
            failed["journey"]["failure"]
        );
        assert!(onboarding::history_records(&f.m).unwrap().is_empty());

        let mut other = card;
        other.installer = "cd".repeat(32);
        project_from(&f.m, std::slice::from_mut(&mut other), None, &[]).unwrap();
        assert!(
            other.journey.is_none(),
            "Progress belongs only to the requested installer"
        );
        assert!(
            matches!(other.primary.unwrap().action, ui::Action::SoftwareInstall { installer, .. }
            if installer == other.installer)
        );
    }
    #[test]
    fn automatic_chain_publishes_actual_native_connection_without_report_or_licence_gate() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        let mut calls = Vec::new();
        let value = preparation_cli::with_automatic_fixture(&c, || {
            continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
                calls.push(a.clone());
                actual(&f.m, a, o)
            })
        })
        .unwrap();
        assert_eq!(value["available"], true);
        assert!(matches!(
            calls.as_slice(),
            [
                ui::Action::InstallerScan { .. },
                ui::Action::CompatibilityCheck { .. },
                ui::Action::CompatibilityPublishTest {
                    expected_current: None,
                    ..
                }
            ]
        ));
        assert_eq!(intent.journey.phase, ui::InstallationPhase::Available);
        let current =
            prep::candidate_record(&f.m, intent.work[0].candidate.as_ref().unwrap()).unwrap();
        assert!(prep::selected_publication_candidate_record(&f.m, &current).unwrap());
        assert!(!current.selection.class.name.is_empty());
        assert_eq!(
            onboarding::result(&f.m, &r).unwrap()["authorization"],
            "not_inspected"
        );
        assert!(!f.m.root.join("preparation/reviews").exists());
        assert!(!calls.iter().any(|a| matches!(
            a,
            ui::Action::CompatibilityResult { .. } | ui::Action::CandidateReview { .. }
        )));
    }
    #[test]
    fn repeated_continuation_never_reinspects_or_republishes_completed_classes() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        preparation_cli::with_automatic_fixture(&c, || {
            continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
                actual(&f.m, a, o)
            })
        })
        .unwrap();
        let before = test_fixture::snapshot(&f.m.publications);
        let registry = fs::read(f.m.root.join("registry.json")).unwrap();
        continue_with(&f.m, &r, &mut intent, &operation, &mut |_, _, _| {
            panic!("completed step ran again")
        })
        .unwrap();
        assert_eq!(test_fixture::snapshot(&f.m.publications), before);
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry);
    }
    #[test]
    fn failed_discovery_or_inspection_never_reaches_publication() {
        for discovery in [true, false] {
            let (f, _c, r, mut intent) = setup();
            let operation = intent.owner.clone();
            let mut publication = false;
            let result = continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| match a {
                ui::Action::InstallerScan { .. } if discovery => {
                    Err("factory_discovery_failed".into())
                }
                ui::Action::CompatibilityCheck { .. } => {
                    Err("component_initialization_failed".into())
                }
                ui::Action::CompatibilityPublishTest { .. } => {
                    publication = true;
                    Err("must_not_publish".into())
                }
                _ => actual(&f.m, a, o),
            });
            assert!(!publication);
            assert!(f.m.registry().unwrap().classes.is_empty());
            if discovery {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap()["available"], false);
                assert_eq!(
                    intent.journey.plugins[0].failure.as_ref().unwrap().stage,
                    ui::InstallationFailureStage::Inspection
                );
                assert_eq!(
                    intent.journey.plugins[0].failure.as_ref().unwrap().code,
                    "component_initialization_failed"
                );
            }
        }
    }
    #[test]
    fn failed_publication_retains_preparation_and_retry_does_not_repeat_it() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        preparation_cli::with_automatic_fixture(&c, || {
            continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
                if matches!(a, ui::Action::CompatibilityPublishTest { .. }) {
                    Err("publication_access_denied".into())
                } else {
                    actual(&f.m, a, o)
                }
            })
        })
        .unwrap();
        assert!(f.m.registry().unwrap().classes.is_empty());
        assert!(intent.work[0].candidate.is_some());
        assert_eq!(
            intent.journey.plugins[0].failure.as_ref().unwrap().stage,
            ui::InstallationFailureStage::Publication
        );
        continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
            assert!(matches!(
                a,
                ui::Action::CompatibilityPublishTest {
                    expected_current: None,
                    ..
                }
            ));
            actual(&f.m, a, o)
        })
        .unwrap();
        assert_eq!(intent.journey.phase, ui::InstallationPhase::Available);
    }
    #[test]
    fn existing_working_configuration_is_preserved_until_separate_change_review() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        prep::retain_lineage(&f.m, &c, &random_id().unwrap(), None).unwrap();
        prep::record_candidate(&f.m, &c).unwrap();
        prep::enable(&f.m, &c, false).unwrap();
        let before = fs::read(f.m.root.join("registry.json")).unwrap();
        continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
            actual(&f.m, a, o)
        })
        .unwrap();
        assert_eq!(
            intent.journey.plugins[0].stage,
            ui::InstallationPluginStage::ReplacementRequired
        );
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
    }
    #[test]
    fn passive_projection_does_not_run_work_and_only_failed_work_offers_resume() {
        let (f, _c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        save(&f.m, &r, &intent, &operation).unwrap();
        let mut setup = ui::InstallerSetup {
            installer: r.installer.clone(),
            name: "Chosen software".into(),
            label_source: "user".into(),
            byte_size: 1,
            format: "exe".into(),
            imported_at: 1,
            phase: ui::SetupPhase::InstallerRetired,
            journey: None,
            status: String::new(),
            environment: Some(r.id.clone()),
            compatibility: None,
            discovered: vec![],
            primary: None,
            secondary: vec![],
            rename: action(
                "Rename",
                ui::Action::InstallerRename {
                    installer: r.installer.clone(),
                    label: String::new(),
                },
                None,
            ),
            history: vec![],
        };
        let record = onboarding::retained_environment_record(&f.m, &r.id)
            .unwrap()
            .unwrap();
        let before = test_fixture::snapshot(&f.m.root);
        project_from(
            &f.m,
            std::slice::from_mut(&mut setup),
            None,
            std::slice::from_ref(&record),
        )
        .unwrap();
        assert_eq!(test_fixture::snapshot(&f.m.root), before);
        assert!(setup.primary.is_none());
        intent.journey.phase = ui::InstallationPhase::NeedsAttention;
        save(&f.m, &r, &intent, &operation).unwrap();
        write_operation(
            &f.m,
            &operation,
            &json!({"schema":2,"operation":operation,"state":"refused"}),
            false,
        )
        .unwrap();
        let record = onboarding::retained_environment_record(&f.m, &r.id)
            .unwrap()
            .unwrap();
        project_from(&f.m, std::slice::from_mut(&mut setup), None, &[record]).unwrap();
        assert!(matches!(
            setup.primary.unwrap().action,
            ui::Action::SoftwareInstallResume { .. }
        ));
    }
    #[test]
    fn failed_discovery_module_remains_visible_beside_available_sibling() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        let path = f.m.root.join("inventory").join(format!("{}.json", r.id));
        let mut scan: inventory::Scan = read_json(&path).unwrap();
        let module = r
            .environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Common Files/VST3/Unfinished.vst3");
        fs::write(&module, b"first-party incomplete discovery fixture").unwrap();
        scan.modules.push(inventory::Module {
            artifact: Artifact {
                sha256: digest(&module).unwrap(),
                path: module,
            },
            classes: vec![],
            report: c.selection.factory_report.clone(),
            inspection_error: Some("factory_initialization_failed".into()),
            quarantine_reason: Some("factory_initialization_failed".into()),
        });
        atomic_json(&path, &scan).unwrap();
        let value = preparation_cli::with_automatic_fixture(&c, || {
            continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
                actual(&f.m, a, o)
            })
        })
        .unwrap();
        assert_eq!(value["available"], false);
        assert_eq!(intent.journey.phase, ui::InstallationPhase::NeedsAttention);
        assert_eq!(
            intent
                .journey
                .plugins
                .iter()
                .filter(|p| p.stage == ui::InstallationPluginStage::Available)
                .count(),
            1
        );
        assert_eq!(
            intent
                .journey
                .plugins
                .iter()
                .filter(|p| p
                    .failure
                    .as_ref()
                    .is_some_and(|f| f.stage == ui::InstallationFailureStage::Discovery))
                .count(),
            1
        );
    }
    #[test]
    fn unchanged_prior_class_is_not_new_when_scan_evidence_changes() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        intent.prior_selections = vec![stable_identity(&c.selection)];
        let path = f.m.root.join("inventory").join(format!("{}.json", r.id));
        let mut scan: inventory::Scan = read_json(&path).unwrap();
        let report = f.outer.join("fresh-factory-report.json");
        fs::copy(&scan.modules[0].report.path, &report).unwrap();
        scan.modules[0].report.path = report;
        let mut new = scan.modules[0].classes[0].clone();
        new.id = "03".repeat(16);
        new.name = "New instrument".into();
        scan.modules[0].classes.push(new);
        atomic_json(&path, &scan).unwrap();
        continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
            if matches!(a, ui::Action::CompatibilityCheck { .. }) {
                Err("new_class_not_initialized".into())
            } else {
                actual(&f.m, a, o)
            }
        })
        .unwrap();
        assert_eq!(intent.journey.plugins.len(), 1);
        assert_eq!(intent.journey.plugins[0].name, "New instrument");
    }
    #[test]
    fn second_prevalidated_resume_cannot_enter_an_owned_installation() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        preparation_cli::with_automatic_fixture(&c, || {
            continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
                actual(&f.m, a, o)
            })
        })
        .unwrap();
        let next = random_id().unwrap();
        private_dir(&job_dir(&f.m, &next).unwrap()).unwrap();
        let _first_owner =
            f.m.lock(&format!("automatic-install-{}.lock", r.installer))
                .unwrap();
        let before = fs::read(
            onboarding::directory(&f.m, &r.id)
                .unwrap()
                .join("record.json"),
        )
        .unwrap();
        let result = execute(
            &f.m,
            &ui::Action::SoftwareInstallResume {
                onboarding: r.id.clone(),
            },
            &next,
            &|| None,
            Duration::from_millis(10),
            &mut vec![],
        );
        assert!(result.is_err());
        assert_eq!(
            fs::read(
                onboarding::directory(&f.m, &r.id)
                    .unwrap()
                    .join("record.json")
            )
            .unwrap(),
            before
        );
    }

    #[test]
    fn interrupted_real_check_resumes_original_checkpoints_through_resume_executor() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        for boundary in ["inspection", "candidate"] {
            let (f, c, r, mut intent) = setup();
            let first = random_id().unwrap();
            private_dir(&job_dir(&f.m, &first).unwrap()).unwrap();
            let interrupted = catch_unwind(AssertUnwindSafe(|| {
                preparation_cli::with_automatic_fixture_interrupt(&c, Some(boundary), || {
                    continue_with(&f.m, &r, &mut intent, &first, &mut |a, o, _| {
                        actual(&f.m, a, o)
                    })
                })
            }));
            assert!(interrupted.is_err());
            let check = intent.work[0].check.clone().unwrap();
            finish(
                &f.m,
                &first,
                &ui::Action::SoftwareInstallResume {
                    onboarding: r.id.clone(),
                },
            )
            .unwrap();
            let child: Value =
                read_json(&job_dir(&f.m, &check).unwrap().join("result.json")).unwrap();
            assert_eq!(child["schema"], 1);
            assert_eq!(child["state"], "refused");
            let next = random_id().unwrap();
            private_dir(&job_dir(&f.m, &next).unwrap()).unwrap();
            let mut resumed = false;
            preparation_cli::with_automatic_fixture(&c,||continue_with(&f.m,&r,&mut intent,&next,
                &mut |a,o,_| {if matches!(a,ui::Action::CompatibilityResumeCheck {operation} if *operation==check) {resumed=true;}
                    actual(&f.m,a,o)})).unwrap();
            assert!(resumed);
            assert_eq!(intent.journey.phase, ui::InstallationPhase::Available);
            assert_eq!(intent.work[0].check.as_deref(), Some(check.as_str()));
            assert_eq!(
                onboarding::retained_environment_record(&f.m, &r.id)
                    .unwrap()
                    .unwrap()
                    .environment,
                r.environment
            );
        }
    }
    #[test]
    fn prelaunch_refusal_keeps_same_environment_and_offers_actual_launch_retry() {
        let (f, _c, mut r, mut intent) = setup();
        let owner = intent.owner.clone();
        r.installation_operation = None;
        intent.journey.phase = ui::InstallationPhase::NeedsAttention;
        r.automatic = Some(intent.clone());
        atomic_json(
            &onboarding::directory(&f.m, &r.id)
                .unwrap()
                .join("record.json"),
            &r,
        )
        .unwrap();
        write_operation(
            &f.m,
            &owner,
            &json!({"schema":1,"operation":owner,"state":"refused"}),
            false,
        )
        .unwrap();
        let mut card = ui::InstallerSetup {
            installer: r.installer.clone(),
            name: "Chosen software".into(),
            label_source: "user".into(),
            byte_size: 1,
            format: "exe".into(),
            imported_at: 1,
            phase: ui::SetupPhase::EnvironmentReady,
            journey: None,
            status: String::new(),
            environment: Some(r.id.clone()),
            compatibility: None,
            discovered: vec![],
            primary: None,
            secondary: vec![],
            rename: action(
                "Rename",
                ui::Action::InstallerRename {
                    installer: r.installer.clone(),
                    label: String::new(),
                },
                None,
            ),
            history: vec![],
        };
        project_from(
            &f.m,
            std::slice::from_mut(&mut card),
            None,
            std::slice::from_ref(&r),
        )
        .unwrap();
        assert!(
            matches!(card.primary.unwrap().action,ui::Action::SoftwareInstallResume {onboarding} if onboarding==r.id)
        );
        let retained = onboarding::retained_environment_record(&f.m, &r.id)
            .unwrap()
            .unwrap();
        assert!(retained.installation_operation.is_none());
        assert_eq!(retained.environment, r.environment);
    }
    #[test]
    fn cancellation_retains_partial_result_without_offering_an_endless_retry() {
        let (f, _c, r, mut intent) = setup();
        let owner = intent.owner.clone();
        let path = onboarding::directory(&f.m, &r.id)
            .unwrap()
            .join(format!("{owner}-result.json"));
        let mut value = onboarding::result(&f.m, &r).unwrap();
        value["state"] = json!("cancelled");
        atomic_json(&path, &value).unwrap();
        assert!(
            continue_with(&f.m, &r, &mut intent, &owner, &mut |_, _, _| panic!(
                "cancelled installer continued automatically"
            ))
            .is_err()
        );
        save(&f.m, &r, &intent, &owner).unwrap();
        write_operation(
            &f.m,
            &owner,
            &json!({"schema":1,"operation":owner,"state":"refused"}),
            false,
        )
        .unwrap();
        assert!(!intent.journey.failure.as_ref().unwrap().retryable);
        let mut card = ui::InstallerSetup {
            installer: r.installer.clone(),
            name: "Chosen software".into(),
            label_source: "user".into(),
            byte_size: 1,
            format: "exe".into(),
            imported_at: 1,
            phase: ui::SetupPhase::InstallerRetired,
            journey: None,
            status: String::new(),
            environment: Some(r.id.clone()),
            compatibility: None,
            discovered: vec![],
            primary: None,
            secondary: vec![],
            rename: action(
                "Rename",
                ui::Action::InstallerRename {
                    installer: r.installer.clone(),
                    label: String::new(),
                },
                None,
            ),
            history: vec![],
        };
        let retained = onboarding::retained_environment_record(&f.m, &r.id)
            .unwrap()
            .unwrap();
        project_from(&f.m, std::slice::from_mut(&mut card), None, &[retained]).unwrap();
        assert!(card.primary.is_none());
        assert_eq!(onboarding::result(&f.m, &r).unwrap(), value);
    }

    #[test]
    fn live_vendor_keeps_exact_focus_control() {
        let (f, _c, r, mut intent) = setup();
        let owner = intent.owner.clone();
        intent.journey.phase = ui::InstallationPhase::PreparingEnvironment;
        save(&f.m, &r, &intent, &owner).unwrap();
        write_operation(
            &f.m,
            &owner,
            &json!({"schema":1,"operation":owner,"state":"vendor_running"}),
            false,
        )
        .unwrap();
        let focus = ui::Action::InstallerFocus {
            onboarding: r.id.clone(),
            operation: owner.clone(),
        };
        let mut card = ui::InstallerSetup {
            installer: r.installer.clone(),
            name: "Chosen software".into(),
            label_source: "user".into(),
            byte_size: 1,
            format: "exe".into(),
            imported_at: 1,
            phase: ui::SetupPhase::InstallerRunning,
            journey: None,
            status: String::new(),
            environment: Some(r.id.clone()),
            compatibility: None,
            discovered: vec![],
            primary: Some(action("Show installer", focus.clone(), None)),
            secondary: vec![],
            rename: action(
                "Rename",
                ui::Action::InstallerRename {
                    installer: r.installer.clone(),
                    label: String::new(),
                },
                None,
            ),
            history: vec![],
        };
        let record = onboarding::retained_environment_record(&f.m, &r.id)
            .unwrap()
            .unwrap();
        project_from(&f.m, std::slice::from_mut(&mut card), None, &[record]).unwrap();
        assert_eq!(card.primary.unwrap().action, focus);
        assert_eq!(
            card.journey.unwrap().phase,
            ui::InstallationPhase::VendorRunning
        );
    }
    #[test]
    fn failed_module_retry_refuses_other_scan_instead_of_adopting_its_inventory() {
        let (f, c, r, mut intent) = setup();
        let operation = intent.owner.clone();
        let path = f.m.root.join("inventory").join(format!("{}.json", r.id));
        let mut scan: inventory::Scan = read_json(&path).unwrap();
        let mut failed = scan.modules[0].clone();
        failed.classes.clear();
        failed.quarantine_reason = Some("failed_module".into());
        scan.modules.push(failed);
        atomic_json(&path, &scan).unwrap();
        preparation_cli::with_automatic_fixture(&c, || {
            continue_with(&f.m, &r, &mut intent, &operation, &mut |a, o, _| {
                actual(&f.m, a, o)
            })
        })
        .unwrap();
        let before = fs::read(f.m.root.join("registry.json")).unwrap();
        let mut other: inventory::Scan = read_json(&path).unwrap();
        other.id = random_id().unwrap();
        other.modules.reverse();
        atomic_json(&path, &other).unwrap();
        let result = continue_with(&f.m, &r, &mut intent, &operation, &mut |_, _, _| {
            panic!("unowned scan target ran")
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("automatic_discovery_retry_inventory_changed"));
        assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
    }

    #[test]
    fn current_home_keeps_real_failed_publication_recovery_separate_from_availability() {
        let (f, c, _r, _intent) = setup();
        prep::retain_lineage(&f.m, &c, &random_id().unwrap(), None).unwrap();
        let candidate = prep::record_candidate(&f.m, &c).unwrap();
        let revision = prep::enable(&f.m, &c, false).unwrap();
        let report = random_id().unwrap();
        let action = ui::Action::CompatibilityResult {
            candidate: candidate.clone(),
            expected_current: ui::PublicationIdentity {
                id: revision.id,
                sha256: revision.sha256,
            },
            result: ui::TestResultKind::Problem {
                category: ui::ProblemCategory::NoAudio,
            },
            passed: vec![],
            failed_area: Some(ui::TestArea::Audio),
            note: "No sound in my DAW".into(),
        };
        prep::retain_guided_result(&f.m, &report, &serde_json::to_value(action).unwrap()).unwrap();
        let mut product = ui::Product {
            class_id: c.selection.class.id.clone(),
            name: c.selection.class.name.clone(),
            vendor: c.selection.class.vendor.clone(),
            role: c.selection.class.role.clone(),
            version: c.selection.class.version.clone(),
            disposition: "experimental".into(),
            active_revision: None,
            recommended_revision: None,
            environment: c.selection.environment.id.clone(),
            runner: c.selection.environment.runner.id.clone(),
            module_sha256: c.selection.module.sha256.clone(),
            limitations: vec![],
            history: vec![],
            actions: vec![],
            compatibility: None,
            availability: Some(ui::NativeAvailability {
                available: true,
                reason: None,
            }),
            details: json!({}),
        };
        let records = prep::RecordReadback::capture(&f.m).unwrap();
        let before = test_fixture::snapshot(&f.m.root);
        preparation_cli::project_required_recovery(
            &f.m,
            std::slice::from_mut(&mut product),
            &records,
            &|_| None,
        )
        .unwrap();
        assert_eq!(test_fixture::snapshot(&f.m.root), before);
        assert!(product.availability.unwrap().available);
        let recovery = product.compatibility.unwrap();
        assert_eq!(recovery.phase, ui::CompatibilityPhase::AwaitingRetirement);
        assert!(
            matches!(recovery.primary.unwrap().action,ui::Action::CompatibilityFinishResult {operation} if operation==report)
        );
    }
}

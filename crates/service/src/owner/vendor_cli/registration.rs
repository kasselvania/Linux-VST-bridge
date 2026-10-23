//! Ordinary application registration, including retained installation custody.
//! Historical readers do not become another launch mechanism.
use super::*;
use linux_vst_bridge::{operator_model::RendererPolicy, renderer_application};

pub(crate) const RENDERING_UNSUPPORTED: &str =
    "Keep using the existing controls for this rendering choice. Ordinary application controls currently use software rendering.";

fn from_installation(m: &Manager, environment: Environment, executable: Artifact,
    helpers: Vec<Artifact>, installation: Artifact) -> Result<Application> {
    let record: onboarding::Record = read_json(&onboarding::directory(m, &environment.id)?.join("record.json"))?;
    require(record.schema == 1 && record.id == environment.id
        && environment_revision::same_space(&record.environment, &environment),
        "vendor_application_installation_binding")?;
    let app = Application {schema: 2, id: ApplicationId::Selected, environment, executable, helpers,
        installer_sha256: Some(record.installer), installer_result: Some(installation),
        observed_installer_version: String::new(), user_selection: None};
    // The genuine retained receipt and imported installer are still mandatory.
    app.verify(&m.root)?;
    Ok(app)
}

fn matching(m: &Manager, app: &Application) -> Result<Option<Application>> {
    let mut matches = vec![];
    for selector in selectors(m)? {
        let selected = application(m, &selector)?;
        if selected.environment.id == app.environment.id && selected.executable.path == app.executable.path {
            require(selected.schema == 2 && selected.environment == app.environment
                && selected.executable == app.executable && selected.installer_sha256 == app.installer_sha256
                && selected.installer_result == app.installer_result && selected.user_selection.is_none(),
                "vendor_application_retained_selection_changed")?;
            matches.push(selected);
        }
    }
    require(matches.len() <= 1, "vendor_application_retained_selection_ambiguous")?;
    Ok(matches.pop())
}

struct PreparedRegistration {
    app: Application,
    files: Vec<(PathBuf, crate::DigestFileIdentity)>,
    selected: Option<Artifact>,
    installer_record: Artifact,
}
impl PreparedRegistration {
    fn new(m: &Manager, app: &Application) -> Result<Self> {
        let current = matching(m, app)?;
        let selected = current.as_ref().map(|app| operator_cli::control_artifact(
            directory(m, &app.selector()?)?.join("application.json"))).transpose()?;
        let app = current.unwrap_or_else(|| app.clone());
        let installer_id = app.installer_sha256.as_deref().ok_or("vendor_application_installation_missing")?;
        let installer_record = operator_cli::control_artifact(m.root.join("installers").join(format!("{installer_id}.json")))?;
        let installer = installer_import::load_record(m, installer_id)?;
        let mut artifacts = vec![&app.executable, &installer.artifact,
            app.installer_result.as_ref().ok_or("vendor_application_installation_missing")?];
        artifacts.extend(&app.helpers);
        let files = artifacts.into_iter().map(|artifact| Ok((artifact.path.clone(),
            crate::DigestFileIdentity::from(&file(&artifact.path)?.metadata()?))))
            .collect::<Result<Vec<_>>>()?;
        app.verify(&m.root)?;
        Ok(Self {app, files, selected, installer_record})
    }
    fn recheck(&self, m: &Manager) -> Result<()> {
        require(operator_cli::control_artifact(self.installer_record.path.clone())? == self.installer_record,
            "vendor_application_retained_installer_changed")?;
        self.app.validate_record(&m.root)?;
        for (path, before) in &self.files {
            require(crate::DigestFileIdentity::from(&file(path)?.metadata()?) == *before
                && crate::DigestFileIdentity::from(&fs::symlink_metadata(path)?) == *before,
                "vendor_application_retained_files_changed")?;
        }
        if let Some(selected) = &self.selected {
            require(operator_cli::control_artifact(selected.path.clone())? == *selected,
                "vendor_application_retained_selection_changed")?;
        }
        let current = matching(m, &self.app)?;
        require(current.as_ref().is_none_or(|app| app == &self.app),
            "vendor_application_retained_selection_changed")
    }
}

/// Only retained identities are read during composition. Execution verifies all
/// files again; an ordinary selection never grants authority to changed bytes.
pub(crate) fn renderer_selected(m: &Manager, old: &renderer_application::Application) -> Result<bool> {
    old.validate_record(&m.root)?;
    // An older ordinary registration is not permission to discard a different
    // rendering choice retained by the historical owner.
    if !rendering_compatible(m, old)? {return Ok(false);}
    let receipt = &old.installation;
    let mut matches = 0;
    for selector in selectors(m)? {
        let app = application(m, &selector)?;
        if app.environment.id == old.environment.id && app.executable == old.executable()?.artifact
            && app.installer_result.as_ref() == Some(receipt) && app.schema == 2 && app.user_selection.is_none() {
            app.validate_record(&m.root)?;
            matches += 1;
        }
    }
    require(matches <= 1, "vendor_application_retained_selection_ambiguous")?;
    Ok(matches == 1)
}

pub(crate) fn saved_rendering(m: &Manager, old: &renderer_application::Application) -> Result<RendererPolicy> {
    let current = renderer_cli::current(m)?;
    if current.is_null() {return Ok(vendor_profile::native_access()?.rendering);}
    let operation = current["operation"].as_str().ok_or("renderer_current_identity")?;
    require(valid_hex(operation, 32), "renderer_operation_identity")?;
    let spec: serde_json::Value = read_json(&renderer_session::operation_dir(m, operation)?.join("spec.json"))?;
    let bound: renderer_application::Application = serde_json::from_value(spec["application"].clone())?;
    bound.validate_record(&m.root)?;
    require(spec["operation"] == operation && current["application"] == bound.identity()?
        && old.same_installation(&bound) && spec["renderer_policy"] == current["policy"],
        "vendor_application_retained_rendering_binding")?;
    Ok(serde_json::from_value(current["policy"].clone())?)
}

pub(crate) fn rendering_compatible(m: &Manager, old: &renderer_application::Application) -> Result<bool> {
    Ok(saved_rendering(m, old)? == vendor_profile::native_access()?.rendering)
}

/// Register the already installed application without touching its setup or
/// rewriting any historical record. No executable is selected by filename.
pub(crate) fn retained_renderer(m: &Manager) -> Result<serde_json::Value> {
    let old = if renderer_cli::directory(m).join("application.json").try_exists()? {
        renderer_cli::current_application(m)?
    } else {renderer_application::discover(&m.root)?};
    old.verify(&m.root)?;
    operator_cli::renderer_registration_allowed(m)?;
    let rendering = saved_rendering(m, &old)?;
    require(rendering_compatible(m, &old)?, RENDERING_UNSUPPORTED)?;
    let executable = old.executable()?.artifact.clone();
    let app = from_installation(m, old.environment.clone(), executable.clone(),
        old.files.values().map(|image| image.artifact.clone()).filter(|image| *image != executable).collect(),
        old.installation.clone())?;
    let prepared = PreparedRegistration::new(m, &app)?;
    let app = admit(m, &prepared, || {
        operator_cli::renderer_registration_allowed(m)?;
        require(saved_rendering(m, &old)? == rendering, "vendor_application_retained_rendering_changed")?;
        let historical = renderer_cli::directory(m).join("application.json");
        if historical.try_exists()? {
            require(renderer_cli::current_application(m)? == old, "renderer_application_selection_changed")?;
        } else {
            private_dir(historical.parent().ok_or("renderer_directory")?)?;
            atomic_json(&historical, &old)?;
        }
        Ok(())
    })?;
    Ok(json!({"application": app.selector()?, "selection": "retained_installation", "rendering": rendering}))
}

fn admit(m: &Manager, prepared: &PreparedRegistration,
    recheck_source: impl FnOnce() -> Result<()>) -> Result<Application> {
    let app = &prepared.app;
    let retired = operator_cli::vendor_maintenance::observe_retired(m, &app.environment)?;
    let guard = m.lock("registry.lock")?;
    capacity::require_environment_inactive(m, &app.environment)?;
    require(!operator_cli::vendor_maintenance::blocks(m, &app.environment)?, "setup_maintenance_active")?;
    retired.verify(m)?;
    prepared.recheck(m)?;
    recheck_source()?;
    register(m, prepared, &guard)
}

fn register(m: &Manager, prepared: &PreparedRegistration, guard: &Lock) -> Result<Application> {
    guard.require_registry(m)?;
    prepared.recheck(m)?;
    publish_application(m, &prepared.app, guard)?;
    Ok(prepared.app.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_installation_preserves_receipt_and_files_without_manual_selection() {
        let (f, source, _) = selection::tests::structured_application(41);
        let receipt = source.installer_result.clone().unwrap();
        let before = fs::read(&receipt.path).unwrap();
        let executable = fs::read(&source.executable.path).unwrap();
        let app = from_installation(&f.m, source.environment.clone(), source.executable.clone(),
            source.helpers.clone(), receipt.clone()).unwrap();
        assert_eq!(app.schema, 2); assert!(app.user_selection.is_none());
        assert!(vendor_profile::inspect(&app, &f.m.root).unwrap().capabilities.is_some());
        let prepared = PreparedRegistration::new(&f.m, &app).unwrap();
        let guard = f.m.lock("registry.lock").unwrap();
        let selected = register(&f.m, &prepared, &guard).unwrap();
        assert_eq!(selected, app);
        assert_eq!(register(&f.m, &prepared, &guard).unwrap(), app);
        assert_eq!(fs::read(&receipt.path).unwrap(), before);
        assert_eq!(fs::read(&source.executable.path).unwrap(), executable);
        let selected_before = fs::read(directory(&f.m, &app.selector().unwrap()).unwrap().join("application.json")).unwrap();
        fs::write(&source.executable.path, b"changed executable").unwrap();
        assert!(register(&f.m, &prepared, &guard).is_err());
        assert_eq!(fs::read(directory(&f.m, &app.selector().unwrap()).unwrap().join("application.json")).unwrap(), selected_before);
    }
    #[test]
    fn changed_import_record_cannot_redirect_prepared_registration() {
        let (f, app, _) = selection::tests::structured_application(45);
        let guard = f.m.lock("registry.lock").unwrap();
        register_application(&f.m, &app, &guard).unwrap();
        let selected = directory(&f.m, &app.selector().unwrap()).unwrap().join("application.json");
        let before = fs::read(&selected).unwrap();
        let prepared = PreparedRegistration::new(&f.m, &app).unwrap();
        let mut imported: installer_import::Installer = read_json(&prepared.installer_record.path).unwrap();
        let redirected = f.m.root.join("installers/redirected.exe");
        fs::write(&redirected, b"not the verified installer").unwrap();
        imported.artifact.path = redirected;
        atomic_json(&prepared.installer_record.path, &imported).unwrap();
        // Record-only application validation accepts the claimed digest; the
        // prepared registration must still bind its original imported record.
        app.validate_record(&f.m.root).unwrap();
        assert_eq!(register(&f.m, &prepared, &guard).unwrap_err().to_string(),
            "vendor_application_retained_installer_changed");
        assert_eq!(fs::read(selected).unwrap(), before);
    }
    #[test]
    fn retained_registration_preserves_unrelated_users_and_refuses_affected_users() {
        for affected in [false, true] {
            let (f, app, _) = selection::tests::structured_application(44);
            let prepared = PreparedRegistration::new(&f.m, &app).unwrap();
            let mut other = app.environment.clone();
            other.id = random_id().unwrap(); other.root = f.m.root.join("environments").join(&other.id);
            let environment = if affected {&app.environment} else {&other};
            let lease = selection::tests::dsp_lease(&f, Some(environment));
            let before = fs::read(&lease).unwrap();
            let record: onboarding::Record = read_json(&onboarding::directory(&f.m, &app.environment.id)
                .unwrap().join("record.json")).unwrap();
            let units = vec![onboarding::unit(record.installation_operation.as_deref().unwrap()).unwrap()];
            let result = readiness::with_unit_state_fixture(&units, "inactive", ||
                admit(&f.m, &prepared, ||Ok(())));
            assert_eq!(result.is_ok(), !affected);
            assert_eq!(directory(&f.m, &app.selector().unwrap()).unwrap().join("application.json").exists(), !affected);
            assert_eq!(fs::read(&lease).unwrap(), before);
        }
    }
    #[test]
    fn unretired_historical_session_keeps_its_custody_and_prevents_registration() {
        let (f, app, _) = selection::tests::structured_application(43);
        let prepared = PreparedRegistration::new(&f.m, &app).unwrap();
        let operation = "ab".repeat(16);
        let legacy = renderer_session::operation_dir(&f.m, &operation).unwrap();
        // Only the owned setup and operation are consumed by the retirement
        // reader; this synthetic record cannot authorize a legacy executable.
        let old = renderer_application::Application {schema: 1, id: renderer_application::ID.into(),
            environment: app.environment.clone(), files: Default::default(),
            installation: app.installer_result.clone().unwrap(), observation_sha256: "00".repeat(32),
            source_seal_sha256: "00".repeat(32)};
        // Use the real reservation path: retirement observers must share an
        // existing writer gate, never create the missing owner's custody.
        renderer_session::submit_with(&f.m, &json!({"operation":operation,"application":old,
            "application_identity":"00".repeat(32),"renderer_policy":"inherited",
            "report":legacy.join("result.json")}), |_|Ok(true),
            ||panic!("acknowledged reservation needs no unit probe")).unwrap();
        for state in ["running", "submission_uncertain"] {
            atomic_json(&legacy.join("result.json"), &json!({"schema":1,"operation":operation,
                "state":state,"cleanup_confirmed":false,"owned_live":null})).unwrap();
            let before = test_fixture::snapshot(&f.m.root);
            assert_eq!(admit(&f.m, &prepared,
                ||panic!("unretired custody cannot publish registration")).unwrap_err().to_string(),
                "setup_legacy_cleanup_unconfirmed");
            assert!(!directory(&f.m, &app.selector().unwrap()).unwrap().join("application.json").exists());
            assert_eq!(test_fixture::snapshot(&f.m.root), before,
                "registration refusal must preserve all retained custody and user bytes");
        }
    }
    #[test]
    fn retained_registration_refuses_ambiguous_choices_and_changed_receipts() {
        let (f, app, _) = selection::tests::structured_application(42);
        let guard = f.m.lock("registry.lock").unwrap();
        register_application(&f.m, &app, &guard).unwrap();
        let mut other = app.clone(); other.helpers.clear();
        assert_ne!(other.selector().unwrap(), app.selector().unwrap());
        register_application(&f.m, &other, &guard).unwrap();
        assert!(PreparedRegistration::new(&f.m, &app).err().unwrap().to_string().contains("ambiguous"));
        let receipt = app.installer_result.clone().unwrap();
        fs::write(&receipt.path, b"{}").unwrap();
        assert!(from_installation(&f.m, app.environment, app.executable, app.helpers, receipt).is_err());
    }
}

//! Exact AP18 companion application identity, separate from VST3 registration.
//! No account data or vendor logs enter this record.
use crate::*;

pub const ASC_INSTALLER: &str = "e92baaaa94f4635fb33b6cbad4f6397d619d07c9e0997e6460a8d64544f1bfb8";
const ASC_FILES: [(&str, &str); 3] = [
    (
        "Arturia Software Center.exe",
        "188afb698a0d7838bf491d208f56e9614846e56a60c90919d1018857ca3e7e25",
    ),
    (
        "ArturiaSoftwareCenterAgent.exe",
        "d0851a06448ac7babda20411387a816fd412069af6139e1a1d286b51822de37e",
    ),
    (
        "updater.exe",
        "4362427643337b3ee881307870359066401da35bb35d02cd55dbfd3a643243bb",
    ),
];

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApplicationId {
    ArturiaSoftwareCenter,
    Selected,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Running,
    Unknown,
    Completed,
    Failed,
    Cancelled,
    CleanupUnconfirmed,
}

/// Closed, temporary AP18 observations; never accepts a command or path.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMode {
    Normal,
    AgentProbe,
    RuninprefixProbe,
    InitializedProbe,
    AccessibilityProbe,
}

impl LaunchMode {
    pub fn action(s: &str) -> Result<Self> {
        match s {
            "launch" => Ok(Self::Normal),
            "diagnose-agent" => Ok(Self::AgentProbe),
            "diagnose-runinprefix" => Ok(Self::RuninprefixProbe),
            "diagnose-initialized" => Ok(Self::InitializedProbe),
            "diagnose-accessibility" => Ok(Self::AccessibilityProbe),
            _ => Err("vendor_application_launch_mode".into()),
        }
    }
}

/// Bounded public window-control outcomes emitted by the companion supervisor.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowControlOutcome {
    Focused,
    CloseRequested,
    WindowManagerRefused,
    RefusedExactWindowUnavailable,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowControlResult {
    pub request: Option<String>,
    pub result: WindowControlOutcome,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationResult {
    pub schema: u32,
    #[serde(default)]
    pub operation_id: Option<String>,
    #[serde(default)]
    pub focus_result: Option<WindowControlResult>,
    #[serde(default)]
    pub close_result: Option<WindowControlResult>,
    pub state: OperationState,
    pub launcher_exit: Option<i32>,
    #[serde(default)]
    pub owned_live: Option<usize>,
    #[serde(default)]
    pub cleanup_confirmed: bool,
    #[serde(default)]
    pub error: Option<String>,
    pub discarded_diagnostic_bytes: u64,
    #[serde(default)]
    pub retained_diagnostic_bytes: u64,
    #[serde(default)]
    pub diagnostic_enabled: bool,
    #[serde(default)]
    pub windows_accessibility_disabled: bool,
    pub account_posture: AccountPosture,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_return: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountPosture {
    Unknown,
}

impl OperationResult {
    /// A cleanup receipt alone is not successful vendor work. Automatic
    /// continuation uses the current exact-operation protocol and zero cohort.
    pub fn successful(&self) -> bool {
        self.schema == 3 && self.operation_id.as_ref().is_some_and(|id|valid_hex(id,32))
            && self.retired() && self.owned_live == Some(0)
            && self.state == OperationState::Completed && self.launcher_exit == Some(0)
            && self.error.is_none()
    }
    pub fn retired(&self) -> bool {
        matches!(self.schema, 1..=3)
            && self.cleanup_confirmed
            && self.owned_live.unwrap_or(0) == 0
            && matches!(
                self.state,
                OperationState::Completed | OperationState::Cancelled | OperationState::Failed
            )
    }
}
impl ApplicationId {
    pub fn parse(s: &str) -> Result<Self> {
        if s == "arturia-software-center" {
            return Ok(Self::ArturiaSoftwareCenter);
        }
        require(valid_hex(s, 64), "vendor_application_unknown")?;
        Ok(Self::Selected)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Application {
    pub schema: u32,
    pub id: ApplicationId,
    pub environment: Environment,
    pub executable: Artifact,
    pub helpers: Vec<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer_result: Option<Artifact>,
    #[serde(default)]
    pub observed_installer_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_selection: Option<ProgramSelection>,
}

/// User intent, independent of any installer observation. Files stay in place.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProgramSelection {
    pub working_directory: PathBuf,
    pub arguments: Vec<String>,
    pub previous: Option<String>,
}

/// Exact companion launch binding, retained separately from mutable progress.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub application: Application,
    pub report: PathBuf,
    pub mode: LaunchMode,
    pub operation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launcher: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed: Option<crate::vendor_profile::ManagedLaunch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_notice: Option<String>,
}
impl Application {
    pub fn selector(&self) -> Result<String> {
        if self.schema == 1 && self.id == ApplicationId::ArturiaSoftwareCenter {
            return Ok("arturia-software-center".into());
        }
        if self.schema == 3 && self.id == ApplicationId::Selected {
            let selection = self.user_selection.as_ref().ok_or("program_selection_missing")?;
            return Ok(hex(&Sha256::digest(serde_json::to_vec(&serde_json::json!({
                "environment":self.environment.id,"executable":self.executable,
                "user_selection":selection
            }))?)));
        }
        require(self.schema == 2 && self.id == ApplicationId::Selected,
            "vendor_application_schema",
        )?;
        // Runtime revision is independently checked at launch; selection keeps
        // the installed environment and exact chosen file identities stable.
        Ok(hex(&Sha256::digest(serde_json::to_vec(
            &serde_json::json!({
                "environment":self.environment.id,"executable":self.executable,
                "helpers":self.helpers,"installer":self.installer_sha256,
                "installation":self.installer_result
            }),
        )?)))
    }
    pub fn display_name(&self) -> String {
        if self.id == ApplicationId::ArturiaSoftwareCenter {
            "Arturia Software Center".into()
        } else {
            self.executable
                .path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        }
    }
    fn validate_selected_record(&self, root: &Path) -> Result<Artifact> {
        let installer_sha256 = self.installer_sha256.as_ref().ok_or("vendor_application_installation_missing")?;
        let installer_result = self.installer_result.as_ref().ok_or("vendor_application_installation_missing")?;
        require(
            self.id == ApplicationId::Selected
                && valid_hex(&self.environment.id, 32)
                && self.environment.root == root.join("environments").join(&self.environment.id)
                && read_json::<Environment>(&self.environment.root.join("environment.json"))?
                    == self.environment,
            "vendor_application_environment",
        )?;
        self.environment.runner.validate_record()?;
        require(
            self.helpers.len() <= 16 && valid_hex(installer_sha256, 64) && self.user_selection.is_none(),
            "vendor_application_selection_bound",
        )?;
        let prefix = self.environment.root.join("compatdata/pfx");
        let mut paths = std::collections::BTreeSet::new();
        for artifact in std::iter::once(&self.executable).chain(self.helpers.iter()) {
            require(
                artifact.path.starts_with(prefix.join("drive_c"))
                    && artifact.path.canonicalize()? == artifact.path
                    && paths.insert(&artifact.path),
                "vendor_application_selected_artifact",
            )?;
            let metadata = file(&artifact.path)?.metadata()?;
            require(
                metadata.nlink() == 1 && metadata.len() > 0 && metadata.len() <= 512 * 1024 * 1024,
                "vendor_application_selected_artifact_bound",
            )?;
            artifact.validate_record()?;
        }
        // This witness proves the selected environment's installation custody.
        // It does not assert that the installer created the selected files.
        let directory = root.join("onboarding").join(&self.environment.id);
        let record: serde_json::Value = read_json(&directory.join("record.json"))?;
        let operation = record["installation_operation"]
            .as_str()
            .ok_or("vendor_application_installation_missing")?;
        require(
            record["schema"] == 1
                && record["id"] == self.environment.id
                && record["environment"]["id"] == self.environment.id
                && record["environment"]["root"] == serde_json::to_value(&self.environment.root)?
                && record["installer"] == *installer_sha256
                && valid_hex(operation, 32)
                && installer_result.path == directory.join(format!("{operation}-result.json")),
            "vendor_application_installation_binding",
        )?;
        let result: serde_json::Value = installer_result.read_record(8 * 1024 * 1024)?;
        require(
            result["schema"] == 2
                && result["operation"] == operation
                && result["state"] == "completed"
                && result["raw_exit"] == 0
                && result["cleanup_confirmed"] == true
                && result["owned_live"] == 0
                && result["error"].is_null(),
            "vendor_application_installation_incomplete",
        )?;
        let imported: serde_json::Value = read_json(
            &root
                .join("installers")
                .join(format!("{installer_sha256}.json")),
        )?;
        require(
            imported["schema"] == 1
                && imported["id"] == *installer_sha256
                && imported["artifact"]["sha256"] == *installer_sha256,
            "vendor_application_import_binding",
        )?;
        let installer: Artifact = serde_json::from_value(imported["artifact"].clone())?;
        require(
            installer.path.parent() == Some(root.join("installers").as_path()),
            "vendor_application_import_binding",
        )?;
        installer.validate_record()?;
        Ok(installer)
    }

    /// Current record/binding observation for read-only cards and choices.
    /// Executable payloads and the runtime tree are verified by execution owners.
    pub fn validate_record(&self, root: &Path) -> Result<()> {
        if self.schema == 3 {
            let selection = self.user_selection.as_ref().ok_or("program_selection_missing")?;
            require(self.id == ApplicationId::Selected && self.helpers.is_empty()
                && self.installer_sha256.is_none() && self.installer_result.is_none()
                && self.observed_installer_version.is_empty()
                && valid_hex(&self.environment.id,32)
                && self.environment.root == root.join("environments").join(&self.environment.id)
                && read_json::<Environment>(&self.environment.root.join("environment.json"))? == self.environment,
                "program_selection_environment")?;
            self.environment.runner.validate_record()?;
            require(self.executable.path.is_absolute()
                && self.executable.path.canonicalize()? == self.executable.path,
                "program_selection_path")?;
            self.executable.validate_record()?;
            require(selection.working_directory.is_absolute()
                && selection.working_directory.canonicalize()? == selection.working_directory
                && selection.working_directory.is_dir()
                && selection.arguments.is_empty()
                && selection.previous.as_ref().is_none_or(|p|valid_hex(p,64)),
                "program_selection_options")?;
            return Ok(());
        }
        if self.schema == 2 {
            return self.validate_selected_record(root).map(|_| ());
        }
        require(
            self.schema == 1 && self.id == ApplicationId::ArturiaSoftwareCenter, "vendor_application_schema",
        )?;
        require(
            self.environment.root == root.join("environments").join(&self.environment.id)
                && read_json::<Environment>(&self.environment.root.join("environment.json"))?
                    == self.environment,
            "vendor_application_environment",
        )?;
        self.environment.runner.validate_record()?;
        require(
            self.installer_sha256.as_deref() == Some(ASC_INSTALLER)
                && self.observed_installer_version == "2.12.0"
                && self.helpers.len() == 2 && self.user_selection.is_none(),
            "vendor_application_release",
        )?;
        let directory = self
            .environment
            .root
            .join("compatdata/pfx/drive_c/Program Files (x86)/Arturia/Arturia Software Center");
        for (artifact, (name, sha)) in std::iter::once(&self.executable)
            .chain(self.helpers.iter())
            .zip(ASC_FILES)
        {
            require(
                artifact.path == directory.join(name)
                    && artifact.path.canonicalize()? == artifact.path
                    && artifact.sha256 == sha,
                "vendor_application_artifact",
            )?;
            artifact.validate_record()?;
        }
        let installer_result = self.installer_result.as_ref().ok_or("vendor_application_installation_missing")?;
        require(
            installer_result.path.parent()
                == Some(self.environment.root.join("installations").as_path()),
            "vendor_application_installation_receipt",
        )?;
        let result: serde_json::Value = installer_result.read_record(8 * 1024 * 1024)?;
        require(
            result["installer"]["sha256"] == ASC_INSTALLER
                && result["raw_exit"] == 0
                && result["cleanup_confirmed"] == true
                && result["error"].is_null(),
            "vendor_application_installation_incomplete",
        )
    }

    pub fn verify(&self, root: &Path) -> Result<()> {
        let installer = if self.schema == 2 {
            Some(self.validate_selected_record(root)?)
        } else {
            self.validate_record(root)?;
            None
        };
        self.environment.runner.verify()?;
        for artifact in std::iter::once(&self.executable).chain(self.helpers.iter()) {
            artifact.verify()?;
        }
        if let Some(installer) = installer {
            installer.verify()?;
        }
        Ok(())
    }
}

pub fn discover(root: &Path, environment: Environment) -> Result<Application> {
    let mut reports = Vec::new();
    let mut count = 0;
    for entry in fs::read_dir(environment.root.join("installations"))? {
        count += 1;
        require(count <= 256, "vendor_application_installation_bound")?;
        let path = entry?.path();
        if !path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with("-result.json"))
        {
            continue;
        }
        let v: serde_json::Value = read_json(&path)?;
        if v["installer"]["sha256"] == ASC_INSTALLER {
            reports.push(Artifact {
                sha256: digest(&path)?,
                path,
            });
        }
    }
    require(
        reports.len() == 1,
        "vendor_application_installation_ambiguous",
    )?;
    let directory = environment
        .root
        .join("compatdata/pfx/drive_c/Program Files (x86)/Arturia/Arturia Software Center");
    let artifacts: Vec<_> = ASC_FILES
        .into_iter()
        .map(|(name, sha)| Artifact {
            path: directory.join(name),
            sha256: sha.into(),
        })
        .collect();
    let app = Application {
        schema: 1,
        id: ApplicationId::ArturiaSoftwareCenter,
        environment,
        executable: artifacts[0].clone(),
        helpers: artifacts[1..].to_vec(),
        installer_sha256: Some(ASC_INSTALLER.into()),
        installer_result: Some(reports.remove(0)),
        observed_installer_version: "2.12.0".into(),
        user_selection: None,
    };
    app.verify(root)?;
    Ok(app)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    #[test]
    fn legacy_application_records_round_trip_without_new_fields_or_identity_changes() {
        let (_f, app) = selected_application();
        let original = serde_json::to_value(&app).unwrap();
        assert!(original.get("user_selection").is_none());
        assert!(original["installer_sha256"].is_string());
        assert!(original["installer_result"].is_object());
        let reopened:Application = serde_json::from_value(original.clone()).unwrap();
        assert_eq!(reopened.selector().unwrap(),app.selector().unwrap());
        assert_eq!(serde_json::to_value(&reopened).unwrap(),original);
        let mut legacy = app;legacy.schema=1;legacy.id=ApplicationId::ArturiaSoftwareCenter;
        let original = serde_json::to_value(&legacy).unwrap();
        let reopened:Application = serde_json::from_value(original.clone()).unwrap();
        assert_eq!(reopened.selector().unwrap(),"arturia-software-center");
        assert_eq!(serde_json::to_value(reopened).unwrap(),original);
    }
    pub(crate) fn selected_application() -> (crate::test_fixture::Fixture, Application) {
        let f = crate::test_fixture::Fixture::new();
        let mut environment = f.r.environment.clone();
        environment.id = "ac".repeat(16);
        environment.root = f.m.root.join("environments").join(&environment.id);
        let prefix = environment.root.join("compatdata/pfx/drive_c");
        private_dir(&prefix).unwrap();
        atomic_json(&environment.root.join("environment.json"), &environment).unwrap();
        let executable = prefix.join("Hub.exe");
        fs::write(&executable, b"MZ selected first-party hub").unwrap();
        let installer = f.m.root.join("installers");
        private_dir(&installer).unwrap();
        let source = installer.join("source.bin");
        fs::write(&source, b"MZ owned installer").unwrap();
        let installer_sha256 = digest(&source).unwrap();
        atomic_json(&installer.join(format!("{installer_sha256}.json")),
            &serde_json::json!({"schema":1,"id":installer_sha256,
                "artifact":{"path":source,"sha256":installer_sha256}})).unwrap();
        let operation = "cb".repeat(16);
        let record = f.m.root.join("onboarding").join(&environment.id);
        private_dir(&record).unwrap();
        atomic_json(&record.join("record.json"), &serde_json::json!({"schema":1,
            "id":environment.id,"environment":environment,"installer":installer_sha256,
            "installation_operation":operation})).unwrap();
        let witness = record.join(format!("{operation}-result.json"));
        atomic_json(&witness, &serde_json::json!({"schema":2,"operation":operation,
            "state":"completed","raw_exit":0,"cleanup_confirmed":true,"owned_live":0,
            "error":null})).unwrap();
        let app = Application { schema:2,id:ApplicationId::Selected,environment,
            executable:Artifact {sha256:digest(&executable).unwrap(),path:executable},
            helpers:vec![],installer_sha256:Some(installer_sha256),installer_result:Some(Artifact {sha256:digest(&witness).unwrap(),path:witness}),
            observed_installer_version:String::new(),user_selection:None };
        (f, app)
    }
    #[test]
    fn record_observation_keeps_payload_verification_at_execution() {
        let (f, app) = selected_application();
        let before = crate::test_fixture::snapshot(&f.m.root);
        app.validate_record(&f.m.root).unwrap();
        assert_eq!(crate::test_fixture::snapshot(&f.m.root), before);
        let executable = fs::read(&app.executable.path).unwrap();
        fs::write(&app.executable.path, b"MZ changed payload").unwrap();
        app.validate_record(&f.m.root).unwrap();
        assert!(app.verify(&f.m.root).is_err(), "Readable records cannot authorize changed app bytes");
        fs::write(&app.executable.path, executable).unwrap();
        fs::write(&app.environment.runner.proton, b"changed runtime payload").unwrap();
        app.validate_record(&f.m.root).unwrap();
        assert!(app.verify(&f.m.root).is_err(), "Execution still verifies runtime bytes");
        fs::remove_file(&app.environment.runner.entry_point).unwrap();
        assert!(app.validate_record(&f.m.root).is_err(), "Missing runtime bindings are visible in readback");
    }

    #[test]
    fn record_observation_rechecks_environment_and_receipt_identity() {
        let (f, app) = selected_application();
        let witness = fs::read(&app.installer_result.as_ref().unwrap().path).unwrap();
        let mut result:serde_json::Value = read_json(&app.installer_result.as_ref().unwrap().path).unwrap();
        result["operation"] = serde_json::json!("dd".repeat(16));
        atomic_json(&app.installer_result.as_ref().unwrap().path, &result).unwrap();
        assert!(app.validate_record(&f.m.root).is_err(), "Changed receipt bytes must match their digest");
        let mut changed = app.clone();
        changed.installer_result.as_mut().unwrap().sha256 = digest(&app.installer_result.as_ref().unwrap().path).unwrap();
        assert!(changed.validate_record(&f.m.root).is_err(), "A foreign receipt operation cannot replace the owned witness");
        fs::write(&app.installer_result.as_ref().unwrap().path, witness).unwrap();
        let mut environment = app.environment.clone();
        environment.revision += 1;
        atomic_json(&environment.root.join("environment.json"), &environment).unwrap();
        assert!(app.validate_record(&f.m.root).is_err(), "Current environment binding is re-observed");
    }

    #[test]
    fn selected_application_requires_owned_operation_and_exact_observed_bytes() {
        let (f, app) = selected_application();
        app.verify(&f.m.root).unwrap();
        assert!(valid_hex(&app.selector().unwrap(),64));
        let original = app.selector().unwrap();
        let mut revised = app.clone();
        revised.environment.revision += 1;
        assert_eq!(revised.selector().unwrap(), original);
        let witness = app.installer_result.as_ref().unwrap().path.clone();
        let mut result:serde_json::Value = read_json(&witness).unwrap();
        result["operation"] = serde_json::json!("dd".repeat(16));
        atomic_json(&witness, &result).unwrap();
        revised = app.clone();
        revised.installer_result.as_mut().unwrap().sha256 = digest(&witness).unwrap();
        assert!(revised.verify(&f.m.root).unwrap_err().to_string().contains("installation_incomplete"));
        fs::write(&app.executable.path, b"MZ changed hub").unwrap();
        assert!(app.verify(&f.m.root).is_err());
        revised.executable.sha256 = digest(&app.executable.path).unwrap();
        assert_ne!(revised.selector().unwrap(), original);
    }
    #[test]
    fn selected_application_cannot_adopt_foreign_receipt_or_executable() {
        let (f, app) = selected_application();
        let mut changed = app.clone();
        changed.installer_result.as_mut().unwrap().path = f.outer.join("copied-result.json");
        fs::copy(&app.installer_result.as_ref().unwrap().path,&changed.installer_result.as_mut().unwrap().path).unwrap();
        assert!(changed.validate_record(&f.m.root).is_err());
        assert!(changed.verify(&f.m.root).is_err());
        changed = app.clone();
        changed.executable.path = f.outer.join("outside.exe");
        fs::copy(&app.executable.path,&changed.executable.path).unwrap();
        assert!(changed.validate_record(&f.m.root).is_err());
        assert!(changed.verify(&f.m.root).is_err());
        changed = app.clone();
        changed.helpers.push(app.executable.clone());
        assert!(changed.validate_record(&f.m.root).is_err());
        assert!(changed.verify(&f.m.root).is_err());
        let mut record:serde_json::Value=read_json(&f.m.root.join("onboarding").join(&app.environment.id).join("record.json")).unwrap();
        record["installer"] = serde_json::json!("aa".repeat(32));
        atomic_json(&f.m.root.join("onboarding").join(&app.environment.id).join("record.json"),&record).unwrap();
        assert!(app.validate_record(&f.m.root).is_err());
        assert!(app.verify(&f.m.root).is_err());
    }
    #[test]
    fn cancelled_failed_live_and_unknown_sessions_never_authorize_automatic_continuation() {
        let successful=serde_json::json!({"schema":3,"operation_id":"ad".repeat(16),
            "state":"completed","launcher_exit":0,"owned_live":0,"cleanup_confirmed":true,
            "error":null,"discarded_diagnostic_bytes":0,"account_posture":"unknown"});
        assert!(serde_json::from_value::<OperationResult>(successful.clone()).unwrap().successful());
        for (key,value) in [("state",serde_json::json!("cancelled")),("state",serde_json::json!("failed")),
            ("state",serde_json::json!("unknown")),("owned_live",serde_json::json!(1)),
            ("owned_live",serde_json::Value::Null),("cleanup_confirmed",serde_json::json!(false)),
            ("launcher_exit",serde_json::json!(1)),("operation_id",serde_json::Value::Null)] {
            let mut changed=successful.clone();changed[key]=value;
            assert!(!serde_json::from_value::<OperationResult>(changed).unwrap().successful());
        }
    }
    #[test]
    fn application_selector_cannot_supply_executable() {
        assert_eq!(
            ApplicationId::parse("arturia-software-center").unwrap(),
            ApplicationId::ArturiaSoftwareCenter
        );
        for bad in [
            "pigments",
            "/tmp/program.exe",
            "arturia-software-center --exec x",
            "",
        ] {
            assert!(ApplicationId::parse(bad).is_err());
        }
        assert!(serde_json::from_str::<ApplicationId>("\"arbitrary\"").is_err());
    }
    #[test]
    fn operation_retirement_and_secret_fields_are_closed() {
        let mut value = serde_json::json!({"schema":1,"state":"unknown","launcher_exit":0,"owned_live":1,"discarded_diagnostic_bytes":0,"account_posture":"unknown"});
        assert!(!serde_json::from_value::<OperationResult>(value.clone())
            .unwrap()
            .retired());
        value["state"] = "completed".into();
        value["owned_live"] = 0.into();
        assert!(!serde_json::from_value::<OperationResult>(value.clone())
            .unwrap()
            .retired());
        value["cleanup_confirmed"] = true.into();
        assert!(serde_json::from_value::<OperationResult>(value.clone())
            .unwrap()
            .retired());
        value["account_email"] = "must not be retained".into();
        assert!(serde_json::from_value::<OperationResult>(value).is_err());
    }

    #[test]
    fn supervisor_window_results_are_typed_without_opening_private_fields() {
        let base = serde_json::json!({"schema":3,"state":"completed","launcher_exit":0,
            "owned_live":0,"cleanup_confirmed":true,"discarded_diagnostic_bytes":0,
            "account_posture":"unknown"});
        for schema in 1..=3 {
            let mut old = base.clone();
            old["schema"] = schema.into();
            assert!(serde_json::from_value::<OperationResult>(old).unwrap().retired());
        }
        for outcome in ["focused","close_requested","window_manager_refused",
            "refused_exact_window_unavailable",
        ] {
            let mut value = base.clone();
            value["focus_result"] = serde_json::json!({"request":null,"result":outcome});
            value["close_result"] = serde_json::json!({"request":"ab".repeat(16),"result":outcome});
            assert!(serde_json::from_value::<OperationResult>(value.clone()).unwrap().retired());
            value["close_result"]["account_email"] = "private".into();
            assert!(serde_json::from_value::<OperationResult>(value).is_err());
        }
        let mut unknown = base;
        unknown["close_result"] = serde_json::json!({"request":null,"result":"arbitrary"});
        assert!(serde_json::from_value::<OperationResult>(unknown).is_err());
    }
    #[test]
    fn diagnostic_modes_and_public_result_cannot_supply_or_expose_private_data() {
        for action in [
            "launch",
            "diagnose-agent",
            "diagnose-runinprefix",
            "diagnose-initialized",
            "diagnose-accessibility",
        ] {
            assert!(LaunchMode::action(action).is_ok());
        }
        assert!(LaunchMode::action("diagnose /tmp/arbitrary.exe").is_err());
        let value = serde_json::json!({"schema":2,"state":"unknown","launcher_exit":5,
            "owned_live":1,"cleanup_confirmed":false,"discarded_diagnostic_bytes":17,
            "retained_diagnostic_bytes":2048,"diagnostic_enabled":true,"account_posture":"unknown"});
        let result: OperationResult = serde_json::from_value(value.clone()).unwrap();
        assert!(!result.retired());
        assert!(!result.windows_accessibility_disabled);
        let mut scoped = value.clone();
        scoped["schema"] = 3.into();
        scoped["windows_accessibility_disabled"] = true.into();
        assert!(
            serde_json::from_value::<OperationResult>(scoped.clone())
                .unwrap()
                .windows_accessibility_disabled
        );
        scoped["windows_accessibility_disabled"] = "global".into();
        assert!(serde_json::from_value::<OperationResult>(scoped).is_err());
        let rendered = serde_json::to_string(&result).unwrap();
        assert!(!rendered.contains("private-diagnostic"));
        for field in ["stdout", "stderr", "log_path", "account_email", "token"] {
            let mut bad = value.clone();
            bad[field] = "private".into();
            assert!(serde_json::from_value::<OperationResult>(bad).is_err());
        }
    }
}

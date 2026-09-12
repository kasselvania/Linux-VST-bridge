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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationResult {
    pub schema: u32,
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
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountPosture {
    Unknown,
}

impl OperationResult {
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
        require(s == "arturia-software-center", "vendor_application_unknown")?;
        Ok(Self::ArturiaSoftwareCenter)
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
    pub installer_sha256: String,
    pub installer_result: Artifact,
    pub observed_installer_version: String,
}
impl Application {
    pub fn verify(&self, root: &Path) -> Result<()> {
        require(self.schema == 1, "vendor_application_schema")?;
        require(
            self.environment.root == root.join("environments").join(&self.environment.id)
                && read_json::<Environment>(&self.environment.root.join("environment.json"))?
                    == self.environment,
            "vendor_application_environment",
        )?;
        self.environment.runner.verify()?;
        require(
            self.installer_sha256 == ASC_INSTALLER
                && self.observed_installer_version == "2.12.0"
                && self.helpers.len() == 2,
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
            artifact.verify()?;
        }
        require(
            self.installer_result.path.parent()
                == Some(self.environment.root.join("installations").as_path()),
            "vendor_application_installation_receipt",
        )?;
        self.installer_result.verify()?;
        let result: serde_json::Value = read_json(&self.installer_result.path)?;
        require(
            result["installer"]["sha256"] == ASC_INSTALLER
                && result["raw_exit"] == 0
                && result["cleanup_confirmed"] == true
                && result["error"].is_null(),
            "vendor_application_installation_incomplete",
        )
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
        installer_sha256: ASC_INSTALLER.into(),
        installer_result: reports.remove(0),
        observed_installer_version: "2.12.0".into(),
    };
    app.verify(root)?;
    Ok(app)
}

#[cfg(test)]
mod tests {
    use super::*;
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

//! What an accepted action acts on (#382).
//!
//! An offer is validated once, when its request is accepted: the request
//! names an action the current view offers, made from that view (its state
//! token). Admission then records, beside the request, the facts the action
//! depends on: the exact identities of what it acts on and whether another
//! task is changing its setup. Its worker checks those facts again under the
//! operator locks instead of the whole view's state token. A record that
//! changed elsewhere (another setup, another plug-in, a new installer, a
//! playing instance) no longer refuses an action that is still exactly safe;
//! a change to what the action acts on refuses it, and the refusal says what
//! changed.
//!
//! Each action's own executor still performs its own checks before it
//! changes anything: inactivity, exclusive locks, custody and exact bytes.
use super::*;

const RECORD: &str = "preconditions.json";

/// One kind of fact an action depends on. Facts are checked in this order,
/// so the most specific explanation is given first.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(super) enum Fact {
    /// The bridge build the request was accepted under.
    Bridge,
    /// Whether another task (an installer, uninstaller or app) is changing
    /// a setup.
    SetupMaintenance,
    /// A plug-in: its registration, DAW connection and published revision.
    Plugin,
    /// A setup's environment record.
    Setup,
    /// The plug-ins a setup has registered.
    SetupPlugins,
    /// A setup's creation record, which names its first installer run.
    SetupCreation,
    /// A setup's last scan.
    SetupScan,
    /// An imported installer.
    Installer,
    /// The setups made from an installer.
    InstallerSetups,
    /// An app's record.
    Application,
    /// The task a control acts on.
    Operation,
    /// The delivered runtimes.
    Runtimes,
}
impl Fact {
    /// What the musician is told when this fact changed after acceptance.
    fn refusal(self) -> &'static str {
        match self {
            Fact::Bridge => "The bridge was updated after you chose this. Look at it again, then choose again.",
            Fact::SetupMaintenance => "Another task, such as an installer or an app, is now changing this setup. \
                Try again when it finishes.",
            Fact::Plugin => "This plug-in changed after you chose this. Look at it again, then choose again.",
            Fact::Setup | Fact::SetupPlugins | Fact::SetupCreation | Fact::SetupScan =>
                "This setup changed after you chose this. Look at it again, then choose again.",
            Fact::Installer => "This installer changed after you chose this. Look at it again, then choose again.",
            Fact::InstallerSetups => "The setups made from this installer changed after you chose this. \
                Look at them again, then choose again.",
            Fact::Application => "This app changed after you chose this. Look at it again, then choose again.",
            Fact::Operation => "The task this controls changed after you chose this. Look at it again, then choose again.",
            Fact::Runtimes => "The available runtimes changed after you chose this. Look at them again, then choose again.",
        }
    }
    /// The fact's record name, as an operation's result names it.
    pub(super) fn name(self) -> String {
        serde_json::to_value(self).ok().and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default()
    }
}

/// One fact as observed: what it is about, and the digest of what it was.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub(super) struct Observed {
    pub fact: Fact,
    pub subject: String,
    pub sha256: String,
}

/// The record admission keeps beside an accepted request.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema: u32,
    operation: String,
    action: ui::Action,
    facts: Vec<Observed>,
}

/// A fact an accepted action depends on changed. Its text is the refusal.
#[derive(Debug)]
pub(super) struct Changed(pub Fact);
impl std::fmt::Display for Changed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.refusal())
    }
}
impl std::error::Error for Changed {}

/// The facts `action` depends on, or `None` for an action whose worker still
/// checks its whole view again: the compatibility and preparation workflow,
/// the problem report, the DAW workspace and the remaining engineering
/// actions.
fn subjects(m: &Manager, action: &ui::Action) -> Result<Option<Vec<(Fact, String)>>> {
    use ui::Action as A;
    use Fact::*;
    let setup = |environment: &str| vec![(Setup, environment.to_owned()), (SetupMaintenance, environment.to_owned())];
    let plugin = |class: &str| -> Result<Vec<(Fact, String)>> {
        let mut facts = vec![(Plugin, class.to_owned())];
        if let Some(entry) = m.registry()?.classes.get(class) {
            facts.push((SetupMaintenance, entry.registration.environment.id.clone()));
        }
        Ok(facts)
    };
    let application = |selector: &str| -> Result<Vec<(Fact, String)>> {
        let app = vendor_cli::application(m, selector)?;
        Ok([vec![(Application, selector.to_owned())], setup(&app.environment.id)].concat())
    };
    let mut facts = match action {
        A::BufferingSet { class_id, .. } | A::DeliverySet { class_id, .. }
        | A::PluginRemoveFromDaw { class_id, .. } | A::PluginAddBackToDaw { class_id, .. }
        | A::OrdinaryRollback { class_id, .. } | A::OrdinaryRestoreRecommended { class_id }
        | A::CaptureArm { class_id } => plugin(class_id)?,
        A::EnvironmentRescan { environment } | A::EnvironmentRefresh { environment, .. }
        | A::SetupRemoveFromDaw { environment } | A::ProductUninstall { environment, .. } =>
            [setup(environment), vec![(SetupPlugins, environment.clone())]].concat(),
        A::EnvironmentInstallerStart { environment, installer }
        | A::EnvironmentInstallerStartWithPolicy { environment, installer, .. } =>
            [setup(environment), vec![(Installer, installer.clone())]].concat(),
        A::EnvironmentInstallerScan { environment } | A::ProgramChoose { environment, .. } => setup(environment),
        A::EnvironmentRuntimeSelect { environment, .. } =>
            [setup(environment), vec![(Runtimes, String::new())]].concat(),
        A::QuarantinedModuleRetry { environment, .. } | A::PluginFinishSetup { environment, .. } =>
            [setup(environment), vec![(SetupScan, environment.clone())]].concat(),
        A::SetupDelete { installer, environment, .. } => [setup(environment),
            vec![(SetupPlugins, environment.clone()), (SetupCreation, environment.clone()),
                (Installer, installer.clone())]].concat(),
        A::InstallerStart { onboarding } | A::InstallerStartWithPolicy { onboarding, .. }
        | A::InstallerScan { onboarding } | A::SoftwareInstallResume { onboarding } =>
            [setup(onboarding), vec![(SetupCreation, onboarding.clone())]].concat(),
        A::InstallerNewAttempt { previous, .. } =>
            vec![(SetupCreation, previous.clone()), (Runtimes, String::new())],
        A::InstallerEnvironmentCreate { installer, .. } | A::SoftwareInstall { installer, .. }
        | A::SoftwareInstallWithPolicy { installer, .. } => vec![(Installer, installer.clone()),
            (InstallerSetups, installer.clone()), (Runtimes, String::new())],
        A::InstallerRename { installer, .. } => vec![(Installer, installer.clone())],
        A::VendorSoftwareInstall { application: app, .. } | A::VendorApplicationOpen { application: app }
        | A::ProgramAdopt { application: app, .. } => application(app)?,
        // A running app's controls act on the app; its own task changes its setup.
        A::VendorApplicationFocus { application: app } | A::VendorApplicationClose { application: app }
        | A::VendorApplicationStop { application: app } => vec![(Application, app.clone())],
        A::RendererOpen { .. } | A::RendererSoftwareInstall { .. } => [vec![(Application, RENDERER.into())],
            setup(renderer::ENVIRONMENT)].concat(),
        // A control of one exact task: the task's own request. The worker also
        // requires that task to be the live one.
        A::InstallerFocus { operation, .. } | A::InstallerStop { operation, .. }
        | A::EnvironmentInstallerFocus { operation } | A::EnvironmentInstallerStop { operation }
        | A::InstallerCloseCheck { operation } | A::EnvironmentRefreshResume { operation }
        | A::PluginSetupFocus { operation } | A::PluginSetupStop { operation }
        | A::RendererFocus { operation } | A::RendererStop { operation } =>
            vec![(Operation, operation.clone())],
        A::RuntimeInstall {} => vec![(Runtimes, String::new())],
        // Recovery rechecks every owner it settles; it acts on no one target.
        A::TransactionReconcile {} => vec![],
        _ => return Ok(None),
    };
    facts.push((Bridge, String::new()));
    Ok(Some(facts))
}
/// The managed Native Access application's directory name.
const RENDERER: &str = "native-access";

/// A control record's digest, or `absent`.
fn file(path: &Path) -> Result<String> {
    if !path.try_exists()? { return Ok("absent".into()); }
    Ok(read_json_identity::<serde::de::IgnoredAny>(path)?.1)
}
fn value(value: &impl Serialize) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(value)?)))
}
fn environment_record(m: &Manager, id: &str) -> Result<PathBuf> {
    require(environment_revision::valid_environment_id(id), "operator_precondition_setup_identity")?;
    Ok(m.root.join("environments").join(id).join("environment.json"))
}
/// What `fact` about `subject` is now.
fn observe_one(m: &Manager, fact: Fact, subject: &str) -> Result<String> {
    match fact {
        Fact::Bridge => file(&m.root.join("software.json")),
        Fact::SetupMaintenance => {
            let path = environment_record(m, subject)?;
            if !path.try_exists()? { return Ok("absent".into()); }
            let environment: Environment = read_json(&path)?;
            Ok(if vendor_maintenance::blocks(m, &environment)? { "maintained" } else { "free" }.into())
        }
        Fact::Plugin => value(&m.registry()?.classes.get(subject)),
        Fact::Setup => file(&environment_record(m, subject)?),
        Fact::SetupPlugins => value(&m.registry()?.classes.iter()
            .filter(|(_, entry)| entry.registration.environment.id == subject)
            .collect::<std::collections::BTreeMap<_, _>>()),
        Fact::SetupCreation => file(&onboarding::directory(m, subject)?.join("record.json")),
        Fact::SetupScan => {
            require(environment_revision::valid_environment_id(subject), "operator_precondition_setup_identity")?;
            file(&m.root.join("inventory").join(format!("{subject}.json")))
        }
        Fact::Installer => {
            require(valid_hex(subject, 64), "operator_precondition_installer_identity")?;
            file(&m.root.join("installers").join(format!("{subject}.json")))
        }
        Fact::InstallerSetups => value(&onboarding::history_records(m)?.iter()
            .filter(|record| record.installer == subject).map(|record| &record.id).collect::<Vec<_>>()),
        Fact::Application => {
            if subject != RENDERER { vendor_application::ApplicationId::parse(subject)?; }
            file(&m.root.join("vendor-applications").join(subject).join("application.json"))
        }
        Fact::Operation => file(&job_dir(m, subject)?.join("request.json")),
        Fact::Runtimes => value(&linux_vst_bridge::runtime_delivery::readback_records(m)?),
    }
}
/// The facts `action` depends on, as they are now, in checking order; `None`
/// for an action that is not checked this way.
pub(super) fn observe(m: &Manager, action: &ui::Action) -> Result<Option<Vec<Observed>>> {
    let Some(subjects) = subjects(m, action)? else { return Ok(None) };
    let mut facts = subjects.into_iter().map(|(fact, subject)| Ok(Observed {
        sha256: observe_one(m, fact, &subject)?, fact, subject })).collect::<Result<Vec<_>>>()?;
    facts.sort();
    facts.dedup();
    Ok(Some(facts))
}
/// Admission observed the facts before and after it validated the offer; a
/// fact that changed meanwhile refuses the request by name.
pub(super) fn require_same(before: &[Observed], after: &[Observed]) -> Result<()> {
    for index in 0..before.len().max(after.len()) {
        let (was, is) = (before.get(index), after.get(index));
        if was != is {
            let fact = was.or(is).map_or(Fact::Bridge, |observed| observed.fact);
            return Err(Changed(fact).into());
        }
    }
    Ok(())
}
/// Keep the facts beside the accepted request.
pub(super) fn record(m: &Manager, operation: &str, request: &ui::Request, facts: &[Observed]) -> Result<()> {
    atomic_json(&job_dir(m, operation)?.join(RECORD), &Record { schema: 1, operation: operation.into(),
        action: request.action.clone(), facts: facts.to_vec() })
}
/// The facts admission kept for this operation's request, or `None` for a
/// request accepted without them.
pub(super) fn recorded(m: &Manager, operation: &str, request: &ui::Request) -> Result<Option<Vec<Observed>>> {
    let path = job_dir(m, operation)?.join(RECORD);
    if !path.try_exists()? { return Ok(None); }
    let record: Record = read_json(&path)?;
    require(record.schema == 1 && record.operation == operation && record.action == request.action,
        "operator_precondition_record")?;
    Ok(Some(record.facts))
}
/// Each recorded fact as it is now, in checking order. The first that
/// changed refuses, by name.
pub(super) fn recheck(m: &Manager, facts: &[Observed]) -> Result<()> {
    for observed in facts {
        if observe_one(m, observed.fact, &observed.subject)? != observed.sha256 {
            return Err(Changed(observed.fact).into());
        }
    }
    Ok(())
}

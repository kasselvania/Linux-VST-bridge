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
    /// Whether an app's own work (installing or updating through it) holds
    /// a setup.
    SetupMaintenance,
    /// A plug-in: its registration, DAW connection and published revision.
    Plugin,
    /// A setup's environment record.
    Setup,
    /// The plug-ins registered from the exact setup an action changes.
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
    /// Which run of an app is current: the one its controls act on.
    ApplicationRun,
    /// The task a control acts on.
    Operation,
    /// How far an interrupted connection journey got: its checkpoint.
    Checkpoint,
    /// The delivered runtimes.
    Runtimes,
}
impl Fact {
    /// What the musician is told when this fact changed after acceptance.
    fn refusal(self) -> &'static str {
        match self {
            Fact::Bridge => "The bridge was updated after you chose this. Look at it again, then choose again.",
            Fact::SetupMaintenance => "An app is now installing or updating in this setup. \
                Try again when it finishes.",
            Fact::Plugin => "This plug-in changed after you chose this. Look at it again, then choose again.",
            Fact::Setup | Fact::SetupPlugins | Fact::SetupCreation | Fact::SetupScan =>
                "This setup changed after you chose this. Look at it again, then choose again.",
            Fact::Installer => "This installer changed after you chose this. Look at it again, then choose again.",
            Fact::InstallerSetups => "The setups made from this installer changed after you chose this. \
                Look at them again, then choose again.",
            Fact::Application => "This app changed after you chose this. Look at it again, then choose again.",
            Fact::ApplicationRun => "This app was closed or opened again after you chose this. \
                Look at it again, then choose again.",
            Fact::Operation => "The task this controls changed after you chose this. Look at it again, then choose again.",
            Fact::Checkpoint => "Connecting this setup's plug-ins moved on after you chose this. \
                Look at it again, then choose again.",
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
/// the problem report and the remaining engineering actions.
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
        A::PluginRestart { .. } => vec![], // exact live proxy/snapshot is rechecked by its owner
        A::PluginOpen { product, .. } => plugin(&product.class_id)?,
        A::BufferingSet { class_id, .. } | A::DeliverySet { class_id, .. }
        | A::PluginRemoveFromDaw { class_id, .. } | A::PluginAddBackToDaw { class_id, .. }
        | A::OrdinaryRollback { class_id, .. } | A::OrdinaryRestoreRecommended { class_id }
        | A::CaptureArm { class_id } => plugin(class_id)?,
        A::EnvironmentRescan { environment } | A::EnvironmentRefresh { environment, .. }
        | A::SetupRemoveFromDaw { environment } =>
            [setup(environment), vec![(SetupPlugins, environment.clone())]].concat(),
        // The plug-ins it takes out of the DAW are the ones its scan found.
        A::ProductUninstall { environment, .. } => [setup(environment),
            vec![(SetupPlugins, environment.clone()), (SetupScan, environment.clone())]].concat(),
        A::EnvironmentInstallerStart { environment, installer }
        | A::EnvironmentInstallerStartWithPolicy { environment, installer, .. } =>
            [setup(environment), vec![(Installer, installer.clone())]].concat(),
        A::EnvironmentInstallerScan { environment } | A::ProgramChoose { environment, .. } => setup(environment),
        A::EnvironmentRuntimeSelect { environment, .. } =>
            [setup(environment), vec![(Runtimes, String::new())]].concat(),
        A::QuarantinedModuleRetry { environment, .. } | A::PluginFinishSetup { environment, .. } =>
            [setup(environment), vec![(SetupScan, environment.clone())]].concat(),
        // Only explicit installer forgetting depends on other attempts.
        A::SetupDelete { installer, environment, forget_installer, .. } => {
            let mut facts = [setup(environment),
                vec![(SetupPlugins, environment.clone()), (SetupCreation, environment.clone()),
                    (Installer, installer.clone())]].concat();
            if *forget_installer { facts.push((InstallerSetups, installer.clone())); }
            facts
        }
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
        // A running app's controls act on its current run; that run's own
        // work changes its setup.
        A::VendorApplicationFocus { application: app } | A::VendorApplicationClose { application: app }
        | A::VendorApplicationStop { application: app } =>
            vec![(Application, app.clone()), (ApplicationRun, app.clone())],
        A::RendererOpen { .. } | A::RendererSoftwareInstall { .. } => [vec![(Application, RENDERER.into())],
            setup(renderer::ENVIRONMENT)].concat(),
        // One exact task: its own request. For an installer's or a plug-in
        // setup's Show and Stop, the worker also requires it to be the live one.
        A::InstallerFocus { operation, .. } | A::InstallerStop { operation, .. }
        | A::EnvironmentInstallerFocus { operation } | A::EnvironmentInstallerStop { operation }
        | A::InstallerCloseCheck { operation }
        | A::PluginSetupFocus { operation } | A::PluginSetupStop { operation }
        | A::PluginFocus { operation, .. } | A::PluginClose { operation, .. }
        | A::RendererFocus { operation } | A::RendererStop { operation } =>
            vec![(Operation, operation.clone())],
        A::EnvironmentRefreshResume { operation } => vec![(Checkpoint, operation.clone())],
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
            Ok(if maintenance::blocks(m, &environment)? || vendor_maintenance::blocks(m, &environment)? { "maintained" } else { "free" }.into())
        }
        Fact::Plugin => value(&m.registry()?.classes.get(subject)),
        Fact::Setup => file(&environment_record(m, subject)?),
        Fact::SetupPlugins => {
            value(&m.registry()?.classes.iter()
                .filter(|(_, entry)| entry.registration.environment.id == subject)
                .collect::<std::collections::BTreeMap<_, _>>())
        }
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
        Fact::ApplicationRun => {
            // Only the run's identity: its record is revised during the run.
            vendor_application::ApplicationId::parse(subject)?;
            let path = m.root.join("vendor-applications").join(subject).join("operation.json");
            if !path.try_exists()? { return Ok("absent".into()); }
            value(&read_json::<Value>(&path)?["operation_id"])
        }
        Fact::Operation => file(&job_dir(m, subject)?.join("request.json")),
        Fact::Checkpoint => {
            let directory = job_dir(m, subject)?;
            value(&[file(&directory.join("refresh.json"))?, file(&directory.join("renderer-continuation.json"))?])
        }
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
thread_local! {
    /// The operation whose worker validated its request through its record.
    static ACCEPTED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}
/// Run the executor of `operation`, whose worker validated its request
/// through its recorded facts: its record must still be there.
pub(super) fn with_accepted<T>(operation: &str, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) { ACCEPTED.with(|accepted| *accepted.borrow_mut() = self.0.take()); }
    }
    let _restore = Restore(ACCEPTED.with(|accepted| accepted.replace(Some(operation.into()))));
    run()
}
/// As `recheck`, for an executor of `operation` once it holds operator
/// serialization again. A record must be this operation's, as `recorded`
/// requires, and the executor whose worker validated through it requires
/// it. A stage run for another action under the same operation is not the
/// accepted request: its facts are not checked. With no record and no such
/// worker, the request was accepted without facts (`recorded`).
pub(super) fn recheck_accepted(m: &Manager, operation: &str, action: &ui::Action) -> Result<()> {
    let path = job_dir(m, operation)?.join(RECORD);
    if !path.try_exists()? {
        let expected = ACCEPTED.with(|accepted| accepted.borrow().as_deref() == Some(operation));
        return require(!expected, "operator_precondition_record");
    }
    let record: Record = read_json(&path)?;
    require(record.schema == 1 && record.operation == operation, "operator_precondition_record")?;
    if record.action != *action { return Ok(()); }
    recheck(m, &record.facts)
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

#[cfg(test)]
mod tests {
    //! Each action's facts, pinned by what refuses it once accepted.
    use super::*;
    /// A published plug-in: its class and setup.
    fn published() -> (test_fixture::Fixture, String, Environment) {
        let (f, c) = preparation_cli::tests::projection_fixture();
        atomic_json(&f.m.root.join("software.json"), &preparation_cli::tests::projection_software(&c)).unwrap();
        linux_vst_bridge::preparation::record_candidate(&f.m, &c).unwrap();
        linux_vst_bridge::preparation::enable(&f.m, &c, false).unwrap();
        let (class, entry) = f.m.registry().unwrap().classes.into_iter().next().unwrap();
        (f, class, entry.registration.environment)
    }
    /// `action` accepted now, its facts recorded as admission records them.
    fn accepted(m: &Manager, action: ui::Action) -> String {
        let request = ui::Request { schema: ui::OPERATOR_SCHEMA, state_token: "accepted".into(), action };
        let id = launch_queued(m, &request, |_| Ok(true)).unwrap().operation.unwrap();
        record(m, &id, &request, &observe(m, &request.action).unwrap().unwrap()).unwrap();
        id
    }
    fn refused_for(m: &Manager, id: &str, fact: &str) {
        worker_with_capacity(m, id, OPERATOR_WAIT, &idle_capacity_test).unwrap();
        let result: Value = read_json(&job_dir(m, id).unwrap().join("result.json")).unwrap();
        assert_eq!(result["state"], "refused", "{result}");
        assert_eq!(result["precondition"], fact, "{result}");
    }
    /// A setup made from installer `ab…`, with its record.
    fn attempt(m: &Manager, environment: &Environment) {
        private_dir(&environment.root).unwrap();
        atomic_json(&environment.root.join("environment.json"), environment).unwrap();
        let directory = onboarding::directory(m, &environment.id).unwrap();
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("record.json"), &onboarding::Record { schema: 1, id: environment.id.clone(),
            installer: "ab".repeat(32), environment: environment.clone(), created_at: 1,
            creation_operation: "cd".repeat(16), installation_operation: None, published: false,
            automatic: None, previous_attempt: None }).unwrap();
    }
    #[test]
    fn rollback_and_restore_are_refused_when_their_plugin_changed() {
        let (f, class, _) = published();
        let rollback = accepted(&f.m, ui::Action::OrdinaryRollback { class_id: class.clone(),
            publication: "ab".repeat(16) });
        let restore = accepted(&f.m, ui::Action::OrdinaryRestoreRecommended { class_id: class.clone() });
        let mut db = f.m.registry().unwrap();
        db.classes.get_mut(&class).unwrap().registration.metadata.name = "Renamed meanwhile".into();
        atomic_json(&f.m.root.join("registry.json"), &db).unwrap();
        for id in [rollback, restore] { refused_for(&f.m, &id, "plugin"); }
    }
    #[test]
    fn removing_a_setup_rechecks_its_plugins_and_preserves_other_attempts() {
        for same_setup in [false, true] {
            let (f, class, environment) = published();
            attempt(&f.m, &environment);
            let mut other = environment.clone();
            other.id = "5a".repeat(16);
            other.root = f.m.root.join("environments").join(&other.id);
            attempt(&f.m, &other);
            let removal = accepted(&f.m, ui::Action::SetupRemoveFromDaw { environment: environment.id.clone() });
            let mut db = f.m.registry().unwrap();
            let sibling = "5B".repeat(16);
            let mut entry = db.classes[&class].clone();
            entry.registration.metadata.class_id = sibling.clone();
            entry.registration.environment = if same_setup { environment.clone() } else { other };
            entry.publication = Publication::Removed;
            entry.managed_revision = None;
            db.classes.insert(sibling.clone(), entry);
            atomic_json(&f.m.root.join("registry.json"), &db).unwrap();
            let before = fs::read(f.m.root.join("registry.json")).unwrap();
            if same_setup {
                refused_for(&f.m, &removal, "setup_plugins");
                assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
            } else {
                worker_with_capacity(&f.m, &removal, OPERATOR_WAIT, &idle_capacity_test).unwrap();
                let result: Value = read_json(&job_dir(&f.m, &removal).unwrap().join("result.json")).unwrap();
                assert_eq!(result["state"], "completed", "{result}");
                let after = f.m.registry().unwrap();
                assert_eq!(after.classes[&class].publication, Publication::Removed);
                assert_eq!(serde_json::to_value(&after.classes[&sibling]).unwrap(),
                    serde_json::to_value(&db.classes[&sibling]).unwrap());
            }
        }
    }
    #[test]
    fn an_uninstall_is_refused_when_its_setup_was_scanned_again() {
        // The plug-ins it takes out of the DAW are the ones its scan found.
        let (f, _, environment) = published();
        let uninstall = accepted(&f.m, ui::Action::ProductUninstall { environment: environment.id.clone(),
            registration: "Vendor Product".into(), command_sha256: "ab".repeat(32), confirmed: true });
        let inventory = f.m.root.join("inventory");
        private_dir(&inventory).unwrap();
        atomic_json(&inventory.join(format!("{}.json", environment.id)), &json!({"scanned":"again"})).unwrap();
        refused_for(&f.m, &uninstall, "setup_scan");
    }
    #[test]
    fn a_runtime_change_is_refused_when_its_setup_changed() {
        let (f, _, environment) = published();
        let change = accepted(&f.m, ui::Action::EnvironmentRuntimeSelect { environment: environment.id.clone(),
            runner: "exact-runner-2".into(), expected_environment: "ab".repeat(32) });
        let mut changed = environment.clone();
        changed.revision += 1;
        atomic_json(&environment.root.join("environment.json"), &changed).unwrap();
        refused_for(&f.m, &change, "setup");
    }
    #[test]
    fn an_apps_controls_are_refused_once_it_was_opened_again() {
        let (f, _) = preparation_cli::tests::projection_fixture();
        let app = super::super::current::tests::selected_vendor_application(&f);
        let selector = app.selector().unwrap();
        let run = |operation: &str, revision: u64| atomic_json(&vendor_cli::directory(&f.m, &selector).unwrap()
            .join("operation.json"), &json!({"operation_id":operation,"revision":revision})).unwrap();
        run(&"a1".repeat(16), 1);
        let action = ui::Action::VendorApplicationStop { application: selector.clone() };
        let stop = accepted(&f.m, action.clone());
        let facts = observe(&f.m, &action).unwrap().unwrap();
        // The same run revises its record: Stop is still for this run.
        run(&"a1".repeat(16), 2);
        recheck(&f.m, &facts).unwrap();
        // Closed and opened again: a new run Stop was not chosen for.
        run(&"b2".repeat(16), 1);
        assert_eq!(recheck(&f.m, &facts).unwrap_err().downcast_ref::<Changed>().map(|changed| changed.0),
            Some(Fact::ApplicationRun));
        refused_for(&f.m, &stop, "application_run");
    }
    #[test]
    fn continuing_a_connection_is_refused_once_its_checkpoint_moved_on() {
        let f = test_fixture::Fixture::new();
        let owner = "c3".repeat(16);
        let directory = job_dir(&f.m, &owner).unwrap();
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("refresh.json"), &json!({"phase":"discovering"})).unwrap();
        let resume = accepted(&f.m, ui::Action::EnvironmentRefreshResume { operation: owner.clone() });
        atomic_json(&directory.join("refresh.json"),
            &json!({"phase":"needs_attention","failure":{"retryable":false}})).unwrap();
        refused_for(&f.m, &resume, "checkpoint");
    }
    #[test]
    fn the_executor_checks_its_accepted_facts_again_once_it_holds_serialization() {
        // Through the executor's own entry: the facts changed after the
        // worker checked them, before the executor took serialization again.
        let (f, class, _) = published();
        let performance = f.m.performance(&class).unwrap().added_frames;
        let action = ui::Action::BufferingSet { class_id: class.clone(), added_frames: 256 };
        let id = accepted(&f.m, action.clone());
        let execute = |action: &ui::Action, operation: &str| execute_with_receipt_policy(&f.m, action,
            Some(operation), &idle_capacity_test, OPERATOR_WAIT, &mut vec![]);
        let mut db = f.m.registry().unwrap();
        db.classes.get_mut(&class).unwrap().registration.metadata.name = "Renamed meanwhile".into();
        atomic_json(&f.m.root.join("registry.json"), &db).unwrap();
        let refused = execute(&action, &id).unwrap_err();
        assert_eq!(refused.downcast_ref::<Changed>().map(|changed| changed.0), Some(Fact::Plugin));
        assert_eq!(f.m.performance(&class).unwrap().added_frames, performance, "nothing changed");
        // A stage another action runs under the same operation is not the
        // accepted request: its facts are not checked.
        let stage = ui::Action::DeliverySet { class_id: class.clone(), mode: ui::DeliveryMode::SameCallback };
        execute(&stage, &id).unwrap();
        // A record that is another operation's refuses.
        let other = random_id().unwrap();
        private_dir(&job_dir(&f.m, &other).unwrap()).unwrap();
        fs::copy(job_dir(&f.m, &id).unwrap().join(RECORD), job_dir(&f.m, &other).unwrap().join(RECORD)).unwrap();
        assert_eq!(execute(&action, &other).unwrap_err().to_string(), "operator_precondition_record");
        // Its worker validated through its record: the record must remain.
        fs::remove_file(job_dir(&f.m, &id).unwrap().join(RECORD)).unwrap();
        assert_eq!(with_accepted(&id, || execute(&action, &id)).unwrap_err().to_string(),
            "operator_precondition_record");
        assert_eq!(f.m.performance(&class).unwrap().added_frames, performance, "nothing changed");
    }
}

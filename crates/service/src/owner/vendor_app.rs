//! Application launch authority. Installation provenance and local process
//! choices form one plan; record codecs preserve supported rollback readers.
use super::*;
use crate::application_session::{RecordFormat, Reservation, Submission};
use crate::vendor_application::{Application, LaunchMode, Operation, OperationResult};
use crate::vendor_profile::ManagedLaunch;
use serde_json::{json, Value};

enum Installation {
    Selected {
        selector: String,
        application: Box<Application>,
    },
    Retained(Box<crate::renderer_application::Application>),
}
impl Installation {
    fn environment(&self) -> &Environment {
        match self {
            Self::Selected { application, .. } => &application.environment,
            Self::Retained(application) => &application.environment,
        }
    }
    fn recheck(&self, m: &Manager) -> Result<()> {
        match self {
            Self::Selected {
                selector,
                application,
            } => require(
                vendor_cli::current_application(
                    m,
                    &vendor_cli::directory(m, selector)?.join("application.json"),
                )? == **application,
                "vendor_application_selection_changed",
            ),
            Self::Retained(application) => require(
                renderer_cli::current_application(m)? == **application,
                "renderer_application_selection_changed",
            ),
        }
    }
}

/// The effective process choice is separate from immutable profile evidence.
/// Retained installations keep the explicit choice in their genuine record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProcessChoice {
    rendering: Option<operator_model::RendererPolicy>,
    supervised_service: bool,
}

enum Record {
    Ordinary {
        job: Box<Operation>,
        prior: Option<Artifact>,
    },
    Retained {
        format: RecordFormat,
        spec: Value,
    },
}

struct LaunchPlan {
    installation: Installation,
    operation: String,
    software: Software,
    process: ProcessChoice,
    record: Record,
    // The ordinary supervisor shares this writer gate. Retained formats take
    // their operation's gate when the compatible reservation is published.
    _writer: Option<Lock>,
}

impl LaunchPlan {
    fn ordinary(m: &Manager, selector: &str, operation: &str) -> Result<Self> {
        require(
            valid_hex(operation, 32),
            "vendor_application_operation_identity",
        )?;
        let directory = vendor_cli::directory(m, selector)?;
        private_dir(&directory)?;
        let writer = m.lock(
            directory
                .join("writer.lock")
                .strip_prefix(&m.root)?
                .to_str()
                .ok_or("vendor_writer_gate")?,
        )?;
        let application = vendor_cli::current_application(m, &directory.join("application.json"))?;
        require(
            application.selector()? == selector,
            "vendor_application_selection_changed",
        )?;
        application.verify(&m.root)?;
        let assisted_before = vendor_cli::recheck_bound_assistance(m, &directory, &application)?;
        let inspected = vendor_profile::inspect(&application, &m.root)?;
        require(
            !assisted_before || inspected.capabilities.is_some(),
            vendor_cli::ASSISTANCE_LOST,
        )?;
        let software = if inspected.capabilities.is_some() || application.user_selection.is_some() {
            preparation::build::launch_software(m)?
        } else {
            software(m)?
        };
        let rendering = inspected
            .capabilities
            .as_ref()
            .map(|capabilities| capabilities.profile.rendering);
        let managed = inspected
            .capabilities
            .map(|capabilities| ManagedLaunch::new(capabilities, software.clone()))
            .transpose()?;
        let launcher = if managed.is_some() || application.user_selection.is_some() {
            software.installer_launch.clone()
        } else {
            None
        };
        let path = directory.join("operation.json");
        let prior = path
            .try_exists()?
            .then(|| operator_cli::control_artifact(path))
            .transpose()?;
        let process = ProcessChoice {
            rendering,
            supervised_service: managed.is_some(),
        };
        let job = Operation {
            application: application.clone(),
            report: directory.join("operation-result.json"),
            mode: LaunchMode::Normal,
            operation_id: operation.into(),
            launcher,
            managed,
            capability_notice: inspected.missing_capability,
        };
        Ok(Self {
            installation: Installation::Selected {
                selector: selector.into(),
                application: Box::new(application),
            },
            operation: operation.into(),
            software,
            process,
            record: Record::Ordinary {
                job: Box::new(job),
                prior,
            },
            _writer: Some(writer),
        })
    }

    fn retained(
        m: &Manager,
        identity: Option<&str>,
        rendering: operator_model::RendererPolicy,
        operation: &str,
        format: RecordFormat,
    ) -> Result<Self> {
        require(valid_hex(operation, 32), "renderer_operation_identity")?;
        let application = renderer_cli::current_application(m)?;
        require(
            !vendor_cli::registration::renderer_selected(m, &application)?,
            "Use this application's ordinary Open action",
        )?;
        if let Some(identity) = identity {
            require(
                application.identity()? == identity,
                "renderer_application_selection_changed",
            )?;
        }
        application.verify(&m.root)?;
        let software = preparation::build::launch_software(m)?;
        let report = format.operation_dir(m, operation)?.join("result.json");
        let spec = match format {
            RecordFormat::Application => native_access_dependency::bind_session(
                m,
                &application,
                &software,
                operation,
                rendering,
                &report,
            )?,
            RecordFormat::Dependency => {
                native_access_dependency::bind(&application, &software, operation, &report)?
            }
        };
        // The codec must represent the admitted choice exactly. It cannot
        // turn a local inherited choice into an invented bundled profile.
        require(
            spec["renderer_policy"] == serde_json::to_value(rendering)?,
            "application_rendering_changed",
        )?;
        Ok(Self {
            installation: Installation::Retained(Box::new(application)),
            operation: operation.into(),
            software,
            process: ProcessChoice {
                rendering: Some(rendering),
                supervised_service: true,
            },
            record: Record::Retained { format, spec },
            _writer: None,
        })
    }

    fn unit(&self) -> Result<String> {
        match &self.record {
            Record::Ordinary { job, .. } => vendor_cli::unit(&job.application.selector()?),
            Record::Retained { format, .. } => format.unit(&self.operation),
        }
    }

    fn reserve(&self, m: &Manager) -> Result<Reserved> {
        match &self.record {
            Record::Ordinary { job, prior } => {
                let directory = vendor_cli::directory(m, &job.application.selector()?)?;
                let path = directory.join("operation.json");
                let current = path
                    .try_exists()?
                    .then(|| operator_cli::control_artifact(path.clone()))
                    .transpose()?;
                require(current == *prior, "vendor_application_operation_changed")?;
                if current.is_some() {
                    let old: OperationResult = read_json(&job.report)?;
                    require(
                        old.retired(),
                        "vendor_application_operation_requires_retirement",
                    )?;
                    let history = directory.join(format!("retired-{}", random_id()?));
                    private_dir(&history)?;
                    fs::rename(&path, history.join("operation.json"))?;
                    fs::rename(&job.report, history.join("result.json"))?;
                }
                if let Some(launch) = &job.managed {
                    require(
                        self.process.rendering == Some(launch.capabilities.profile.rendering),
                        "application_rendering_changed",
                    )?;
                    atomic_json(
                        &directory.join(vendor_profile::LAUNCH_SETTINGS),
                        &vendor_profile::launch_settings(
                            &launch.capabilities.profile,
                            &self.operation,
                        )?,
                    )?;
                }
                atomic_json(&path, job)?;
                atomic_json(
                    &job.report,
                    &json!({"schema":3,"operation_id":self.operation,"state":"unknown",
                    "launcher_exit":null,"owned_live":null,"cleanup_confirmed":false,"error":null,
                    "discarded_diagnostic_bytes":0,"account_posture":"unknown"}),
                )?;
                Ok(Reserved::Ordinary {
                    path,
                    report: job.report.clone(),
                    reset_failed: prior.is_some(),
                })
            }
            Record::Retained { format, spec } => {
                require(
                    spec["renderer_policy"]
                        == serde_json::to_value(
                            self.process
                                .rendering
                                .ok_or("application_rendering_missing")?,
                        )?,
                    "application_rendering_changed",
                )?;
                Ok(Reserved::Retained(format.reserve(m, spec)?))
            }
        }
    }
}

enum Reserved {
    Ordinary {
        path: PathBuf,
        report: PathBuf,
        reset_failed: bool,
    },
    Retained(Reservation),
}
impl Reserved {
    fn path(&self) -> &Path {
        match self {
            Self::Ordinary { path, .. } => path,
            Self::Retained(reservation) => &reservation.path,
        }
    }
}

fn execute(m: &Manager, plan: LaunchPlan) -> Result<Submission> {
    execute_with(
        m,
        plan,
        |unit, service, software, path| {
            let mut command = vendor_application::session_command(unit, service, software, path)?;
            Ok(submit_command(|| command.spawn()))
        },
        renderer_session::inspect_unit,
    )
}

fn execute_with(
    m: &Manager,
    plan: LaunchPlan,
    submit: impl FnOnce(&str, bool, &Software, &Path) -> Result<CommandSubmission>,
    probe: impl FnOnce(&str) -> Result<renderer_session::UnitState>,
) -> Result<Submission> {
    // Observe every actual user of this exact setup outside native admission.
    // The short registry phase checks the same retained custody before writing.
    let environment = plan.installation.environment();
    let retired = operator_cli::vendor_maintenance::observe_retired(m, environment)?;
    let reserved = {
        let _registry = m.lock("registry.lock")?;
        capacity::require_environment_inactive(m, environment)?;
        if let Some(owned) = operator_cli::vendor_maintenance::for_vendor(m, &plan.operation)? {
            require(
                owned == *environment,
                "vendor_maintenance_environment_changed",
            )?;
        } else {
            require(
                !operator_cli::vendor_maintenance::blocks(m, environment)?,
                "setup_maintenance_active",
            )?;
        }
        retired.verify(m)?;
        plan.installation.recheck(m)?;
        plan.reserve(m)?
    };
    let unit = plan.unit()?;
    if matches!(
        reserved,
        Reserved::Ordinary {
            reset_failed: true,
            ..
        }
    ) {
        let _ = readiness::reset_application_unit(&unit);
    }
    // One physical submission. Lost acknowledgment never authorizes a replay.
    let submitted = submit(
        &unit,
        plan.process.supervised_service,
        &plan.software,
        reserved.path(),
    )?;
    match reserved {
        Reserved::Ordinary { report, .. } => {
            finish_ordinary_submission(
                &report,
                &plan.operation,
                plan.process.supervised_service,
                submitted,
            )?;
            Ok(Submission::SubmittedOrLive)
        }
        Reserved::Retained(reservation) => {
            reservation.complete(m, submitted == CommandSubmission::Acknowledged, || {
                probe(&unit)
            })
        }
    }
}

pub(super) fn open(m: &Manager, application: &str, operation: &str) -> Result<()> {
    execute(m, LaunchPlan::ordinary(m, application, operation)?).map(|_| ())
}
pub(super) fn open_retained(
    m: &Manager,
    identity: &str,
    rendering: operator_model::RendererPolicy,
    operation: &str,
) -> Result<Submission> {
    execute(
        m,
        LaunchPlan::retained(
            m,
            Some(identity),
            rendering,
            operation,
            RecordFormat::Application,
        )?,
    )
}
pub(super) fn prepare_retained_service(m: &Manager, operation: &str) -> Result<Submission> {
    execute(
        m,
        LaunchPlan::retained(
            m,
            None,
            operator_model::RendererPolicy::SoftwareRendering,
            operation,
            RecordFormat::Dependency,
        )?,
    )
}

pub(crate) fn stop_retained(m: &Manager, operation: &str, dependency: bool) -> Result<()> {
    let format = if dependency {
        RecordFormat::Dependency
    } else {
        RecordFormat::Application
    };
    let unit = format.unit(operation)?;
    stop_retained_with(
        m,
        operation,
        format,
        Duration::from_secs(90),
        || format.inspect(operation),
        || readiness::request_user_unit_stop(&unit),
    )
}

fn stop_retained_with(
    m: &Manager,
    operation: &str,
    format: RecordFormat,
    timeout: Duration,
    probe: impl FnMut() -> Result<renderer_session::UnitState>,
    stop: impl FnOnce() -> Result<()>,
) -> Result<()> {
    use std::cell::{Cell, RefCell};
    let directory = format.operation_dir(m, operation)?;
    let spec = operator_cli::control_artifact(directory.join("spec.json"))?;
    let probe = RefCell::new(probe);
    let requested = Cell::new(false);
    let check = || {
        format.exact(m, operation)?;
        require(
            operator_cli::control_artifact(spec.path.clone())? == spec,
            "vendor_application_operation_changed",
        )
    };
    let finish = || {
        check()?;
        let observed = (probe.borrow_mut())();
        format.record_probe(m, operation, &observed)?;
        let state = observed?;
        require(state.empty, "vendor_application_cleanup_unconfirmed")?;
        format.close_empty(m, operation, &state)?;
        require(
            format.terminal(&format.result(m, operation)?, operation),
            "vendor_application_cleanup_unconfirmed",
        )
    };
    Retirement {
        manager: m,
        timeout,
        writer: &directory.join("writer.lock"),
    }
    .run(
        check,
        || {
            let state = (probe.borrow_mut())()?;
            Ok(!state.empty || (!requested.get() && state.exists))
        },
        || {
            stop()?;
            requested.set(true);
            Ok(())
        },
        |registry| {
            drop(registry);
            let operator_lock::LockAttempt::Acquired(_writer) = format.gate(m, operation)? else {
                return Err("vendor_application_cleanup_unconfirmed".into());
            };
            finish()
        },
        finish,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CommandSubmission {
    NotStarted,
    Acknowledged,
    Unconfirmed,
}
const SUBMISSION_ACKNOWLEDGMENT: Duration = Duration::from_secs(30);
fn submit_command(
    spawn: impl FnOnce() -> std::io::Result<std::process::Child>,
) -> CommandSubmission {
    submit_command_with(SUBMISSION_ACKNOWLEDGMENT, spawn)
}
fn submit_command_with(
    limit: Duration,
    spawn: impl FnOnce() -> std::io::Result<std::process::Child>,
) -> CommandSubmission {
    let mut child = match spawn() {
        Ok(child) => child,
        Err(_) => return CommandSubmission::NotStarted,
    };
    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    CommandSubmission::Acknowledged
                } else {
                    CommandSubmission::Unconfirmed
                }
            }
            Ok(None) if Instant::now() < deadline => (),
            _ => {
                // This is only the submission client, never the accepted unit
                // or its cgroup. Losing its acknowledgment retains uncertainty.
                let _ = child.kill();
                let _ = child.wait();
                return CommandSubmission::Unconfirmed;
            }
        }
        std::thread::sleep(
            Duration::from_millis(10).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

fn finish_ordinary_submission(
    report: &Path,
    operation: &str,
    managed: bool,
    submitted: CommandSubmission,
) -> Result<()> {
    if submitted == CommandSubmission::NotStarted {
        let mut value: Value = read_json(report)?;
        require(
            value["operation_id"] == operation,
            "vendor_application_operation_changed",
        )?;
        value["error"] =
            "The app could not be started. Try again; if it still fails, send Setup details."
                .into();
        value["error_code"] = "vendor_application_not_submitted".into();
        if managed {
            value["dependency"] = json!({"dependency_cleanup_disposition":"not_started",
            "service_actions_started":false,"process_cleanup_confirmed":false});
        }
        atomic_json(report, &value)?;
        return Err(
            "The app could not be started. Try again; if it still fails, send Setup details."
                .into(),
        );
    }
    require(
        submitted == CommandSubmission::Acknowledged,
        "vendor_application_launch_unconfirmed",
    )
}
#[cfg(test)]
pub(super) fn submit_with(
    report: &Path,
    operation: &str,
    managed: bool,
    spawn: impl FnOnce() -> std::io::Result<std::process::Child>,
) -> Result<()> {
    finish_ordinary_submission(report, operation, managed, submit_command(spawn))
}
/// Recover a session whose app is not running. Plug-in admission takes the
/// registry lock without waiting, so it is released before the re-check (up to
/// 30 s). The app's own writer lock, held by the re-check, keeps this app from
/// launching meanwhile; a later launch binds a new operation.
pub(super) fn recover_stopped(
    m: &Manager,
    selector: &str,
    operation: &str,
    registry: Lock,
    reconcile: impl FnOnce(&Manager, &str, &str) -> Result<()>,
) -> Result<()> {
    drop(registry);
    reconcile(m, selector, operation)?;
    println!(
        "{}",
        serde_json::to_string(&vendor_cli::result(m, selector, operation)?)?
    );
    Ok(())
}
/// One cancellation barrier for every application record format. Record
/// adapters verify exact custody and interpret their own cleanup evidence.
struct Retirement<'a> {
    manager: &'a Manager,
    timeout: Duration,
    writer: &'a Path,
}
impl Retirement<'_> {
    fn run(
        self,
        check: impl Fn() -> Result<()>,
        mut live: impl FnMut() -> Result<bool>,
        request_stop: impl FnOnce() -> Result<()>,
        recover: impl FnOnce(Lock) -> Result<()>,
        finished: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        let Self {
            manager: m,
            timeout,
            writer,
        } = self;
        require(
            timeout <= Duration::from_secs(90),
            "vendor_application_stop_wait_bound",
        )?;
        let registry = m.lock("registry.lock")?;
        check()?;
        drop(registry);
        let running = live()?;
        let registry = m.lock("registry.lock")?;
        check()?;
        if !running {
            return recover(registry);
        }
        let stop_error = request_stop().err();
        drop(registry);
        if let Some(error) = stop_error {
            // A collected unit may disappear while Stop is queued. Only fresh
            // absence plus exclusive custody permits its exact receipt recovery.
            check()?;
            if live()? {
                return Err(error);
            }
            let gate = writer
                .strip_prefix(&m.root)?
                .to_str()
                .ok_or("vendor_writer_gate")?;
            let operator_lock::LockAttempt::Acquired(_writer) = m.try_lock(gate)? else {
                return Err(error);
            };
            check()?;
            return finished();
        }
        let deadline = Instant::now() + timeout;
        let writer = writer
            .strip_prefix(&m.root)?
            .to_str()
            .ok_or("vendor_writer_gate")?;
        loop {
            check()?;
            if !live()? {
                if let operator_lock::LockAttempt::Acquired(_writer) = m.try_lock(writer)? {
                    check()?;
                    return finished();
                }
            }
            require(
                Instant::now() < deadline,
                "vendor_application_cleanup_unconfirmed",
            )?;
            std::thread::sleep(
                Duration::from_millis(50).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    }
}

pub(super) fn cancel_with(
    m: &Manager,
    selector: &str,
    operation: Option<&str>,
    timeout: Duration,
    unit_live: impl FnMut() -> Result<bool>,
    request_stop: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let directory = vendor_cli::directory(m, selector)?;
    let job_path = directory.join("operation.json");
    let pinned = operator_cli::control_artifact(job_path.clone())?;
    let job: Operation = read_json(&job_path)?;
    let current = || -> Result<OperationResult> {
        require(
            operator_cli::control_artifact(job_path.clone())? == pinned,
            "vendor_application_operation_changed",
        )?;
        let result = vendor_cli::result(m, selector, &job.operation_id)?;
        if let Some(id) = operation {
            require(
                job.operation_id == id && result.operation_id.as_deref() == Some(id),
                "vendor_application_operation_changed",
            )?;
        }
        Ok(result)
    };
    Retirement {
        manager: m,
        timeout,
        writer: &directory.join("writer.lock"),
    }
    .run(
        || current().map(|_| ()),
        unit_live,
        request_stop,
        |registry| {
            recover_stopped(
                m,
                selector,
                &job.operation_id,
                registry,
                vendor_cli::reconcile,
            )
        },
        || {
            let result = current()?;
            require(result.retired(), "vendor_application_cleanup_unconfirmed")?;
            println!("{}", serde_json::to_string(&result)?);
            Ok(())
        },
    )
}

pub(super) fn stop(m: &Manager, application: &str, operation: &str) -> Result<()> {
    require(
        valid_hex(operation, 32),
        "vendor_application_operation_identity",
    )?;
    let unit = vendor_cli::unit(application)?;
    cancel_with(
        m,
        application,
        Some(operation),
        Duration::from_secs(90),
        || vendor_cli::live(application),
        || readiness::request_user_unit_stop(&unit),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn fixture(format: Option<RecordFormat>) -> (test_fixture::Fixture, LaunchPlan, Vec<String>) {
        let (f, application, _) = vendor_cli::selection::tests::structured_application(51);
        let software = Software {
            manager: f.r.host.clone(),
            supervisor: f.r.host.clone(),
            ownership: f.r.host.clone(),
            host: f.r.host.clone(),
            source_manifest: f.r.host.clone(),
            source_sha256: f.r.host.sha256.clone(),
            installer_launch: Some(f.r.host.clone()),
            preparation_kit: None,
            operator_frontend: None,
            native_catalogue: None,
        };
        atomic_json(&f.m.root.join("software.json"), &software).unwrap();
        let record: onboarding::Record = read_json(
            &onboarding::directory(&f.m, &application.environment.id)
                .unwrap()
                .join("record.json"),
        )
        .unwrap();
        let mut units =
            vec![onboarding::unit(record.installation_operation.as_deref().unwrap()).unwrap()];
        let operation = random_id().unwrap();
        let plan = if let Some(format) = format {
            // Synthetic retained metadata tests the shared executor after input
            // verification. It cannot pass the production retained constructor.
            let retained = renderer_application::Application {
                schema: 1,
                id: renderer_application::ID.into(),
                environment: application.environment,
                files: Default::default(),
                installation: application.installer_result.unwrap(),
                observation_sha256: "00".repeat(32),
                source_seal_sha256: "00".repeat(32),
            };
            private_dir(&renderer_cli::directory(&f.m)).unwrap();
            atomic_json(
                &renderer_cli::directory(&f.m).join("application.json"),
                &retained,
            )
            .unwrap();
            let rendering = if format == RecordFormat::Application {
                operator_model::RendererPolicy::Inherited
            } else {
                operator_model::RendererPolicy::SoftwareRendering
            };
            let spec = json!({"operation":operation,"application":retained,"application_identity":retained.identity().unwrap(),
                "renderer_policy":rendering,"report":format.operation_dir(&f.m,&operation).unwrap().join("result.json")});
            assert!(
                LaunchPlan::retained(&f.m, None, rendering, &operation, format).is_err(),
                "synthetic metadata must never become executable provenance"
            );
            LaunchPlan {
                installation: Installation::Retained(Box::new(retained)),
                operation,
                software,
                process: ProcessChoice {
                    rendering: Some(rendering),
                    supervised_service: true,
                },
                record: Record::Retained { format, spec },
                _writer: None,
            }
        } else {
            let selector = application.selector().unwrap();
            let directory = vendor_cli::directory(&f.m, &selector).unwrap();
            private_dir(&directory).unwrap();
            atomic_json(&directory.join("application.json"), &application).unwrap();
            units.push(vendor_cli::unit(&selector).unwrap());
            LaunchPlan::ordinary(&f.m, &selector, &operation).unwrap()
        };
        (f, plan, units)
    }

    #[test]
    fn all_launch_records_share_exact_setup_admission_and_release_registry_before_submission() {
        for format in [
            None,
            Some(RecordFormat::Application),
            Some(RecordFormat::Dependency),
        ] {
            for affected in [false, true] {
                let (f, plan, units) = fixture(format);
                let operation = plan.operation.clone();
                let expected_unit = plan.unit().unwrap();
                let mut other = plan.installation.environment().clone();
                other.id = random_id().unwrap();
                other.root = f.m.root.join("environments").join(&other.id);
                let lease = vendor_cli::selection::tests::dsp_lease(
                    &f,
                    Some(if affected {
                        plan.installation.environment()
                    } else {
                        &other
                    }),
                );
                let before = fs::read(&lease).unwrap();
                let inputs = test_fixture::snapshot(&f.m.root);
                let submitted = Cell::new(0);
                let outcome = readiness::with_unit_state_fixture(&units, "inactive", || {
                    execute_with(
                        &f.m,
                        plan,
                        |unit, service, software, path| {
                            submitted.set(submitted.get() + 1);
                            assert_eq!(unit, expected_unit);
                            assert!(service && software.installer_launch.is_some());
                            let _registry =
                                f.m.lock("registry.lock")
                                    .expect("physical submission must not hold native admission");
                            let saved: Value = read_json(path)?;
                            if let Some(format) = format {
                                assert_eq!(saved["operation"], operation);
                                let rendering = if format == RecordFormat::Application {
                                    "inherited"
                                } else {
                                    "software_rendering"
                                };
                                assert_eq!(saved["renderer_policy"], rendering);
                                assert_eq!(format.current(&f.m)?["policy"], rendering);
                                assert!(matches!(format.gate(&f.m,&operation)?,operator_lock::LockAttempt::Busy),
                                "the actual reservation must exclude a late writer until acknowledgment");
                            } else {
                                let job: Operation = serde_json::from_value(saved)?;
                                let capabilities = &job.managed.unwrap().capabilities;
                                assert_eq!(capabilities.profile, vendor_profile::native_access()?);
                                assert_eq!(
                                    capabilities.profile_sha256,
                                    vendor_profile::profile_sha256(&capabilities.profile)?
                                );
                            }
                            Ok(CommandSubmission::Acknowledged)
                        },
                        |_| panic!("acknowledged submission needs no recovery probe"),
                    )
                });
                assert_eq!(outcome.is_ok(), !affected, "{format:?}: {outcome:?}");
                assert_eq!(submitted.get(), usize::from(!affected));
                assert_eq!(fs::read(&lease).unwrap(), before);
                if affected {
                    assert_eq!(test_fixture::snapshot(&f.m.root), inputs);
                }
            }
        }
    }

    #[test]
    fn changed_selection_and_local_rendering_cannot_publish_a_prepared_launch() {
        for change in ["selection", "rendering"] {
            let (f, mut plan, units) = fixture(Some(RecordFormat::Application));
            if change == "rendering" {
                plan.process.rendering = Some(operator_model::RendererPolicy::SoftwareRendering);
            } else {
                let mut record: Value =
                    read_json(&renderer_cli::directory(&f.m).join("application.json")).unwrap();
                record["source_seal_sha256"] = json!("11".repeat(32));
                atomic_json(
                    &renderer_cli::directory(&f.m).join("application.json"),
                    &record,
                )
                .unwrap();
            }
            let before = test_fixture::snapshot(&f.m.root);
            let error = readiness::with_unit_state_fixture(&units, "inactive", || {
                execute_with(
                    &f.m,
                    plan,
                    |_, _, _, _| panic!("changed plan must not submit"),
                    |_| panic!("no submission"),
                )
            })
            .unwrap_err();
            assert_eq!(
                error.to_string(),
                if change == "rendering" {
                    "application_rendering_changed"
                } else {
                    "renderer_application_selection_changed"
                }
            );
            assert_eq!(test_fixture::snapshot(&f.m.root), before);
        }
    }

    #[test]
    fn uncertain_submission_is_never_replayed_and_keeps_compatible_recovery_custody() {
        for format in [RecordFormat::Application, RecordFormat::Dependency] {
            let (f, plan, units) = fixture(Some(format));
            let operation = plan.operation.clone();
            let submitted = Cell::new(0);
            let result = readiness::with_unit_state_fixture(&units, "inactive", || {
                execute_with(
                    &f.m,
                    plan,
                    |_, _, _, _| {
                        submitted.set(submitted.get() + 1);
                        Ok(CommandSubmission::Unconfirmed)
                    },
                    |_| Err("unit unavailable".into()),
                )
            })
            .unwrap();
            assert_eq!(result, Submission::AcknowledgmentUncertain);
            assert_eq!(submitted.get(), 1);
            assert_eq!(format.current(&f.m).unwrap()["operation"], operation);
            let path = format
                .operation_dir(&f.m, &operation)
                .unwrap()
                .join("spec.json");
            let before = fs::read(&path).unwrap();
            assert_eq!(
                format
                    .reconcile_with(&f.m, &operation, || Err("still unavailable".into()))
                    .unwrap(),
                Submission::AcknowledgmentUncertain
            );
            assert_eq!(fs::read(path).unwrap(), before);
            assert!(!format
                .retired_with(&f.m, &operation, || panic!("no terminal receipt"))
                .unwrap());
        }
    }

    #[test]
    fn delayed_submission_acknowledgment_keeps_the_accepted_process_and_exact_reservation() {
        struct OwnedChild(std::process::Child);
        impl Drop for OwnedChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let mut accepted = OwnedChild(
            Command::new("python3")
                .args(["-I", "-c", "import sys; sys.stdin.buffer.read(1)"])
                .stdin(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let accepted_pid = accepted.0.id();
        for format in [RecordFormat::Application, RecordFormat::Dependency] {
            let (f, plan, units) = fixture(Some(format));
            let operation = plan.operation.clone();
            let starts = Cell::new(0);
            let started = Instant::now();
            let state = readiness::with_unit_state_fixture(&units, "inactive", || {
                execute_with(
                    &f.m,
                    plan,
                    |_, _, _, _| {
                        Ok(submit_command_with(Duration::from_millis(20), || {
                            starts.set(starts.get() + 1);
                            Command::new("python3")
                                .args(["-I", "-c", "import time; time.sleep(30)"])
                                .spawn()
                        }))
                    },
                    |_| {
                        Ok(renderer_session::UnitState {
                            exists: true,
                            empty: false,
                            observation: None,
                        })
                    },
                )
            })
            .unwrap();
            assert!(started.elapsed() < Duration::from_secs(5));
            assert_eq!(starts.get(), 1);
            assert_eq!(state, Submission::SubmittedOrLive);
            assert_eq!(accepted.0.id(), accepted_pid);
            assert!(
                accepted.0.try_wait().unwrap().is_none(),
                "the accepted process must survive a lost client acknowledgment"
            );
            let saved: Value = read_json(
                &format
                    .operation_dir(&f.m, &operation)
                    .unwrap()
                    .join("submission.json"),
            )
            .unwrap();
            assert_eq!(saved["acknowledged"], false);
            assert_eq!(format.current(&f.m).unwrap()["operation"], operation);
            assert!(!format
                .retired_with(&f.m, &operation, || panic!("no terminal receipt"))
                .unwrap());
        }
        accepted.0.stdin.take().unwrap().write_all(b"x").unwrap();
        assert!(accepted.0.wait().unwrap().success());
        assert_eq!(
            submit_command_with(Duration::from_secs(2), || Command::new("/bin/true").spawn()),
            CommandSubmission::Acknowledged
        );
        assert_eq!(
            submit_command_with(Duration::from_secs(2), || Err(
                std::io::ErrorKind::NotFound.into()
            )),
            CommandSubmission::NotStarted
        );
    }

    #[test]
    fn retained_stop_uses_the_common_barrier_without_blocking_unrelated_admission() {
        for format in [RecordFormat::Application, RecordFormat::Dependency] {
            let (f, plan, units) = fixture(Some(format));
            let operation = plan.operation.clone();
            readiness::with_unit_state_fixture(&units, "inactive", || {
                execute_with(
                    &f.m,
                    plan,
                    |_, _, _, _| Ok(CommandSubmission::Acknowledged),
                    |_| panic!(),
                )
            })
            .unwrap();
            let directory = format.operation_dir(&f.m, &operation).unwrap();
            let spec = fs::read(directory.join("spec.json")).unwrap();
            let writer = RefCell::new(Some(
                f.m.lock(
                    directory
                        .join("writer.lock")
                        .strip_prefix(&f.m.root)
                        .unwrap()
                        .to_str()
                        .unwrap(),
                )
                .unwrap(),
            ));
            let stopping = Cell::new(false);
            let polls = Cell::new(0);
            stop_retained_with(
                &f.m,
                &operation,
                format,
                Duration::from_secs(2),
                || {
                    drop(
                        f.m.lock("registry.lock")
                            .expect("process observation must not hold native admission"),
                    );
                    if stopping.get() {
                        polls.set(polls.get() + 1);
                        if polls.get() >= 2 {
                            drop(writer.borrow_mut().take());
                        }
                    }
                    Ok(renderer_session::UnitState {
                        exists: writer.borrow().is_some(),
                        empty: writer.borrow().is_none(),
                        observation: None,
                    })
                },
                || {
                    assert!(matches!(
                        f.m.try_lock("registry.lock")?,
                        operator_lock::LockAttempt::Busy
                    ));
                    stopping.set(true);
                    Ok(())
                },
            )
            .unwrap();
            assert!(polls.get() >= 2);
            assert_eq!(fs::read(directory.join("spec.json")).unwrap(), spec);
            let result = format.result(&f.m, &operation).unwrap();
            assert!(format.terminal(&result, &operation));
            assert_eq!(
                result["dependency_retirement"]["service_retirement_confirmed"], false,
                "process cleanup cannot invent a Windows service receipt"
            );
        }
    }
}

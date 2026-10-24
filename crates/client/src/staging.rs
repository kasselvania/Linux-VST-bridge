//! First-run package intake. One worker retains the verified installer handle;
//! the window can request staging and opening Setup, never package activation.
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};

#[cfg(target_os = "linux")]
mod installed;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    pub version: String,
    pub internal: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Install,
    OpenSetup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailedAt {
    Verification,
    Installation,
    Opening,
    Worker,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Checking,
    Ready,
    Installing,
    AddingShortcut,
    Staged,
    Opening,
    Opened,
    Failed { at: FailedAt, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub package: Option<Package>,
    pub phase: Phase,
    pub closing: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            package: None,
            phase: Phase::Checking,
            closing: false,
        }
    }
}

impl View {
    pub fn offers(&self, step: Step) -> bool {
        !self.closing
            && matches!(
                (&self.phase, step),
                (Phase::Ready, Step::Install) | (Phase::Staged, Step::OpenSetup)
            )
    }

    fn changing_files_or_opening(&self) -> bool {
        matches!(
            self.phase,
            Phase::Installing | Phase::AddingShortcut | Phase::Opening
        )
    }

    pub fn should_close(&self) -> bool {
        self.phase == Phase::Opened || (self.closing && !self.changing_files_or_opening())
    }
}

enum Event {
    Verified(Package),
    Phase(Phase),
}

/// This interface is private: the installed implementation delegates all trust
/// and filesystem authority to portable_package. Tests replace only the I/O.
trait Intake: Send {
    fn package(&self) -> Package;
    fn stage(&mut self) -> Result<(), String>;
    fn expose_setup(&mut self) -> Result<(), String>;
    fn open_setup(&mut self) -> Result<(), String>;
}

pub struct Worker {
    view: View,
    commands: SyncSender<Step>,
    events: Receiver<Event>,
}

impl Worker {
    /// Inspect only this process's executable, on the worker. No bundle path or
    /// operation supplied by the caller can select a different package.
    pub fn start(wake: impl Fn() + Send + 'static) -> Self {
        Self::spawn(wake, || {
            #[cfg(target_os = "linux")]
            {
                installed::inspect().map(|intake| Box::new(intake) as Box<dyn Intake>)
            }
            #[cfg(not(target_os = "linux"))]
            {
                Err("This installer runs on Linux. Open the Linux installer on your Linux computer.".into())
            }
        })
    }

    fn spawn(
        wake: impl Fn() + Send + 'static,
        inspect: impl FnOnce() -> Result<Box<dyn Intake>, String> + Send + 'static,
    ) -> Self {
        let (commands, incoming) = mpsc::sync_channel(1);
        let (outgoing, events) = mpsc::sync_channel(8);
        let mut worker = Self {
            view: View::default(),
            commands,
            events,
        };
        let spawned = std::thread::Builder::new()
            .name("bridge-package-intake".into())
            .spawn(move || {
                let send = |event| {
                    let sent = outgoing.send(event).is_ok();
                    if sent {
                        wake();
                    }
                    sent
                };
                let mut intake = match inspect() {
                    Ok(intake) => intake,
                    Err(reason) => {
                        send(Event::Phase(Phase::Failed {
                            at: FailedAt::Verification,
                            reason,
                        }));
                        return;
                    }
                };
                if !send(Event::Verified(intake.package())) {
                    return;
                }
                // Each explicit step runs at most once. A failed operation cannot
                // be replayed by a stale/double click or by a disconnected window.
                if incoming.recv() != Ok(Step::Install) {
                    return;
                }
                let staged = intake.stage().and_then(|()| {
                    if !send(Event::Phase(Phase::AddingShortcut)) {
                        return Err("The installer window closed.".into());
                    }
                    intake.expose_setup()
                });
                if let Err(reason) = staged {
                    send(Event::Phase(Phase::Failed {
                        at: FailedAt::Installation,
                        reason,
                    }));
                    return;
                }
                if !send(Event::Phase(Phase::Staged)) {
                    return;
                }
                if incoming.recv() != Ok(Step::OpenSetup) {
                    return;
                }
                let phase = match intake.open_setup() {
                    Ok(()) => Phase::Opened,
                    Err(reason) => Phase::Failed {
                        at: FailedAt::Opening,
                        reason,
                    },
                };
                send(Event::Phase(phase));
            });
        if let Err(error) = spawned {
            worker.view.phase = Phase::Failed {
                at: FailedAt::Worker,
                reason: error.to_string(),
            };
        }
        worker
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    /// Returns immediately; pending feedback is visible before the first I/O.
    pub fn press(&mut self, step: Step) -> bool {
        if !self.view.offers(step) {
            return false;
        }
        self.view.phase = match step {
            Step::Install => Phase::Installing,
            Step::OpenSetup => Phase::Opening,
        };
        if self.commands.try_send(step).is_err() {
            self.view.phase = Phase::Failed { at: FailedAt::Worker,
                reason: "The installer worker is unavailable. Close this window and reopen the installer.".into() };
        }
        true
    }

    /// File intake is atomic but cannot be interrupted partway through by the
    /// UI. Finish that one operation before closing. Closing while staging
    /// never queues Open Setup.
    pub fn request_close(&mut self) {
        self.view.closing = true;
    }

    /// Called only outside drawing. No filesystem or process work occurs here.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        for _ in 0..8 {
            match self.events.try_recv() {
                Ok(Event::Verified(package)) => {
                    self.view.package = Some(package);
                    self.view.phase = Phase::Ready;
                    changed = true;
                }
                Ok(Event::Phase(phase)) => {
                    self.view.phase = phase;
                    changed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !matches!(self.view.phase, Phase::Opened | Phase::Failed { .. }) {
                        self.view.phase = Phase::Failed { at: FailedAt::Worker,
                            reason: "The installer worker stopped before reporting a result. Reopen the installer to check the package again.".into() };
                        changed = true;
                    }
                    break;
                }
            }
        }
        changed
    }
}

/// Call before starting any window or worker threads, as in the existing
/// first-run launcher. This applies only to the installer process.
pub fn prepare_process() {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::umask(0o077);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    struct Fake {
        calls: Arc<Mutex<Vec<&'static str>>>,
        stage_gate: Option<Receiver<()>>,
        failed: Option<&'static str>,
    }
    impl Fake {
        fn call(&self, call: &'static str) -> Result<(), String> {
            self.calls.lock().unwrap().push(call);
            if self.failed == Some(call) {
                Err(format!("{call} refused"))
            } else {
                Ok(())
            }
        }
    }
    impl Intake for Fake {
        fn package(&self) -> Package {
            Package {
                version: "test".into(),
                internal: true,
            }
        }
        fn stage(&mut self) -> Result<(), String> {
            self.call("stage")?;
            if let Some(gate) = self.stage_gate.take() {
                gate.recv_timeout(Duration::from_secs(2))
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        }
        fn expose_setup(&mut self) -> Result<(), String> {
            self.call("expose")
        }
        fn open_setup(&mut self) -> Result<(), String> {
            self.call("open")
        }
    }
    fn until(worker: &mut Worker, predicate: impl Fn(&Phase) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            worker.poll();
            if predicate(&worker.view().phase) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "worker did not settle: {:?}",
                worker.view()
            );
            std::thread::yield_now();
        }
    }
    fn fake(
        failed: Option<&'static str>,
        gate: Option<Receiver<()>>,
    ) -> (Worker, Arc<Mutex<Vec<&'static str>>>) {
        let calls = Arc::new(Mutex::new(vec![]));
        let retained = calls.clone();
        let worker = Worker::spawn(
            || {},
            move || {
                Ok(Box::new(Fake {
                    calls: retained,
                    stage_gate: gate,
                    failed,
                }))
            },
        );
        (worker, calls)
    }

    #[test]
    fn verification_is_off_thread_and_closing_before_install_writes_nothing() {
        let (release, gate) = mpsc::sync_channel(1);
        let (entered, entry) = mpsc::sync_channel(1);
        let caller = std::thread::current().id();
        let mut worker = Worker::spawn(
            || {},
            move || {
                assert_ne!(std::thread::current().id(), caller);
                entered.send(()).unwrap();
                gate.recv_timeout(Duration::from_secs(2)).unwrap();
                Err("signature refused".into())
            },
        );
        entry.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(worker.view().phase, Phase::Checking);
        assert!(!worker.press(Step::Install));
        worker.request_close();
        assert!(worker.view().should_close());
        release.send(()).unwrap();
        until(&mut worker, |phase| {
            matches!(
                phase,
                Phase::Failed {
                    at: FailedAt::Verification,
                    ..
                }
            )
        });
        assert!(worker.view().package.is_none());

        let (mut worker, calls) = fake(None, None);
        until(&mut worker, |phase| *phase == Phase::Ready);
        worker.request_close();
        assert!(!worker.press(Step::Install));
        drop(worker);
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn staging_and_opening_are_separate_explicit_steps_and_double_clicks_do_not_replay() {
        let (mut worker, calls) = fake(None, None);
        until(&mut worker, |phase| *phase == Phase::Ready);
        assert_eq!(worker.view().package.as_ref().unwrap().version, "test");
        assert!(!worker.press(Step::OpenSetup));
        assert!(worker.press(Step::Install));
        assert_eq!(worker.view().phase, Phase::Installing);
        assert!(!worker.press(Step::Install));
        until(&mut worker, |phase| *phase == Phase::Staged);
        assert_eq!(*calls.lock().unwrap(), ["stage", "expose"]);
        assert!(!worker.view().should_close());
        assert!(worker.press(Step::OpenSetup));
        assert!(!worker.press(Step::OpenSetup));
        until(&mut worker, |phase| *phase == Phase::Opened);
        assert!(worker.view().should_close());
        assert_eq!(*calls.lock().unwrap(), ["stage", "expose", "open"]);
    }

    #[test]
    fn close_during_staging_waits_for_atomic_intake_and_never_opens_setup() {
        let (release, gate) = mpsc::sync_channel(1);
        let (mut worker, calls) = fake(None, Some(gate));
        until(&mut worker, |phase| *phase == Phase::Ready);
        assert!(worker.press(Step::Install));
        worker.request_close();
        assert!(!worker.view().should_close());
        assert!(!worker.press(Step::OpenSetup));
        release.send(()).unwrap();
        until(&mut worker, |phase| *phase == Phase::Staged);
        assert!(worker.view().should_close());
        assert!(!worker.press(Step::OpenSetup));
        assert_eq!(*calls.lock().unwrap(), ["stage", "expose"]);
    }

    #[test]
    fn failures_keep_the_exact_step_and_never_continue_or_replay() {
        for (failed, expected, at) in [
            ("stage", vec!["stage"], FailedAt::Installation),
            ("expose", vec!["stage", "expose"], FailedAt::Installation),
            ("open", vec!["stage", "expose", "open"], FailedAt::Opening),
        ] {
            let (mut worker, calls) = fake(Some(failed), None);
            until(&mut worker, |phase| *phase == Phase::Ready);
            worker.press(Step::Install);
            if failed == "open" {
                until(&mut worker, |phase| *phase == Phase::Staged);
                worker.press(Step::OpenSetup);
            }
            until(&mut worker, |phase| matches!(phase, Phase::Failed { .. }));
            assert_eq!(
                worker.view().phase,
                Phase::Failed {
                    at,
                    reason: format!("{failed} refused")
                }
            );
            assert!(!worker.press(Step::Install));
            assert!(!worker.press(Step::OpenSetup));
            assert!(!worker.view().should_close());
            assert_eq!(*calls.lock().unwrap(), expected);
        }
    }
}

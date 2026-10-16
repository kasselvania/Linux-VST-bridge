//! Bounded non-RT acquisition only. Admission keeps Manager::lock fail-fast.
use crate::{operator_model::*, *};
use std::cell::Cell;
use std::time::{Duration, Instant};
pub mod timing;
thread_local! {
    static OPERATOR_REGISTRY_WAIT: Cell<Option<Duration>> = const { Cell::new(None) };
}
/// Inside an operator action (its worker, its finishing step and request
/// dispatch) a busy registry.lock is waited out for up to `wait` before the
/// usual busy refusal. Status reads hold that lock only briefly; failing at
/// once made actions lose to an open manager window. Every other lock keeps
/// fail-fast ownership, and the service and audio paths never enter this scope.
pub fn with_operator_registry_wait<T>(wait: Duration, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<Duration>);
    impl Drop for Restore {
        fn drop(&mut self) { OPERATOR_REGISTRY_WAIT.with(|w| w.set(self.0)); }
    }
    let _restore = Restore(OPERATOR_REGISTRY_WAIT.with(|w| w.replace(Some(wait))));
    run()
}
pub(crate) fn operator_registry_wait() -> Option<Duration> {
    OPERATOR_REGISTRY_WAIT.with(Cell::get)
}
/// An operator action waiting for registry.lock holds this marker. Status
/// readers (overview, pulse, details, readiness) look at it before every
/// attempt and step aside while it is held, so readers that take the lock back
/// to back cannot starve the action: it gets the lock within one in-progress
/// reader hold plus one 10 ms poll, unless another action holds the lock.
pub const REGISTRY_WRITER_WAITING: &str = "registry-writer-waiting.lock";
/// The marker sits beside registry.lock from its first use, so a waiting
/// action never adds a file to the managed root. Only fairness depends on it.
pub(crate) fn ensure_registry_writer_marker(m: &Manager) {
    let path = m.root.join(REGISTRY_WRITER_WAITING);
    if fs::symlink_metadata(&path).is_err() {
        let _ = fs::OpenOptions::new().write(true).create(true).truncate(false).mode(0o600)
            .custom_flags(libc::O_NOFOLLOW).open(path);
    }
}
/// True while an operator action is waiting for registry.lock. Only fairness
/// depends on this; an unreadable marker counts as no waiting action.
pub fn registry_writer_waiting(m: &Manager) -> bool {
    matches!(m.try_lock_shared(REGISTRY_WRITER_WAITING), Ok(LockAttempt::Busy))
}
#[derive(Debug)]
pub struct LockBusy;
impl std::fmt::Display for LockBusy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("operation already running")
    }
}
impl std::error::Error for LockBusy {}
pub enum LockAttempt {
    Acquired(Lock),
    Busy,
}
#[cfg(test)]
thread_local! {
    pub(crate) static BUSY_OBSERVER: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}
#[derive(Debug)]
pub struct AcquisitionFailure {
    pub facts: LockFacts,
}
impl std::fmt::Display for AcquisitionFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Manager {:?} lock {:?} after {} attempts",
            self.facts.name, self.facts.outcome, self.facts.attempts
        )
    }
}
impl std::error::Error for AcquisitionFailure {}
impl OperatorLock {
    pub fn filename(self) -> &'static str {
        match self {
            Self::Registry => "registry.lock",
            Self::Canonical => "operator-canonical.lock",
            Self::Receipt => "operator-receipt.lock",
            Self::Resume => "operator-resume.lock",
        }
    }
}
impl Manager {
    pub fn lock_bounded(
        &self,
        name: OperatorLock,
        purpose: LockPurpose,
        operation: Option<&str>,
        timeout: Duration,
    ) -> Result<(Lock, LockFacts)> {
        require(
            timeout <= Duration::from_secs(75),
            "operator_lock_wait_bound",
        )?;
        require(
            operation.is_none_or(|id| valid_hex(id, 32)),
            "operator_lock_operation",
        )?;
        let start = Instant::now();
        // Status readback yields to a waiting action; everything else that
        // waits for registry.lock is an action and announces that it waits.
        let reader = purpose == LockPurpose::OperatorReadback;
        let announces = name == OperatorLock::Registry && !reader;
        let mut waiting: Option<Lock> = None;
        let mut facts = LockFacts {
            name,
            mode: LockMode::Exclusive,
            purpose,
            operation: operation.map(Into::into),
            policy: WaitPolicy::Bounded,
            elapsed_wait_us: 0,
            attempts: 0,
            timeout_ms: timeout.as_millis() as u64,
            outcome: LockOutcome::Acquired,
            holder: LockHolder::Unknown,
        };
        loop {
            facts.attempts += 1;
            let attempt = if reader && name == OperatorLock::Registry
                && registry_writer_waiting(self) {
                Ok(LockAttempt::Busy)
            } else {
                self.try_lock(name.filename())
            };
            #[cfg(test)]
            if matches!(&attempt, Ok(LockAttempt::Busy)) {
                BUSY_OBSERVER.with(|observer| {
                    if let Some(run) = observer.borrow_mut().take() { run(); }
                });
            }
            facts.elapsed_wait_us = start.elapsed().as_micros().min(u64::MAX as u128) as u64;
            match attempt {
                Ok(LockAttempt::Acquired(lock)) => return Ok((lock, facts)),
                Ok(LockAttempt::Busy) if start.elapsed() < timeout => {
                    // One waiting action holds the marker; others keep polling.
                    if announces && waiting.is_none() {
                        if let Ok(LockAttempt::Acquired(marker)) = self.try_lock(REGISTRY_WRITER_WAITING) {
                            waiting = Some(marker);
                        }
                    }
                    std::thread::sleep(
                        Duration::from_millis(10).min(timeout.saturating_sub(start.elapsed())))
                }
                Ok(LockAttempt::Busy) => {
                    facts.outcome = LockOutcome::Timeout;
                    return Err(AcquisitionFailure { facts }.into());
                }
                Err(_) => {
                    facts.outcome = LockOutcome::Error;
                    return Err(AcquisitionFailure { facts }.into());
                }
            }
        }
    }
}

impl Manager {
    /// registry.lock for an operator action: a bounded wait that announces
    /// itself to status readers. Its stated result when the wait expires is
    /// the same busy refusal an immediate attempt gave before.
    pub(crate) fn lock_registry_waiting(&self, wait: Duration) -> Result<Lock> {
        match self.lock_bounded(OperatorLock::Registry, LockPurpose::OperatorValidationReadback,
            None, wait) {
            Ok((lock, _)) => Ok(lock),
            Err(error) if error.downcast_ref::<AcquisitionFailure>()
                .is_some_and(|f| f.facts.outcome == LockOutcome::Timeout) => Err(LockBusy.into()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    #[test]
    fn admission_and_mutation_stay_fail_fast_and_busy_is_typed() {
        let f = Fixture::new();
        let held = f.m.lock("registry.lock").unwrap();
        assert!(matches!(
            f.m.try_lock("registry.lock").unwrap(),
            LockAttempt::Busy
        ));
        assert!(f.m.lock("registry.lock").err().unwrap().is::<LockBusy>());
        let limits = capacity::fixture_limits();
        assert!(matches!(
            capacity::reserve(&f.m, &limits, None, false)
                .err()
                .unwrap()
                .downcast_ref::<capacity::Refusal>(),
            Some(capacity::Refusal::ServiceBusy)
        ));
        assert!(f.m.reconcile_inactive().err().unwrap().is::<LockBusy>());
        drop(held);
        assert!(matches!(
            f.m.try_lock("registry.lock").unwrap(),
            LockAttempt::Acquired(_)
        ));
    }
    #[test]
    fn operator_wait_is_bounded_scoped_and_registry_only() {
        let f = Fixture::new();
        let held = f.m.lock("registry.lock").unwrap();
        // Its stated result on expiry is the same busy refusal as before.
        let started = Instant::now();
        let error = with_operator_registry_wait(Duration::from_millis(60),
            || f.m.lock("registry.lock")).err().unwrap();
        assert!(error.is::<LockBusy>() && started.elapsed() >= Duration::from_millis(60));
        // Ownership locks and everything outside an action stay fail-fast.
        let owner = f.m.lock("operator-environment.lock").unwrap();
        let started = Instant::now();
        assert!(with_operator_registry_wait(Duration::from_secs(5),
            || f.m.lock("operator-environment.lock")).err().unwrap().is::<LockBusy>());
        assert!(f.m.lock("registry.lock").err().unwrap().is::<LockBusy>());
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(operator_registry_wait().is_none());
        drop((held, owner));
    }
    #[test]
    fn back_to_back_status_reads_cannot_starve_a_waiting_action() {
        use std::sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc};
        let f = Fixture::new();
        let stop = Arc::new(AtomicBool::new(false));
        let rounds = Arc::new(AtomicUsize::new(0));
        let hold = Duration::from_millis(50);
        // An open manager window: one status read after another, each taking
        // registry.lock again as soon as the previous one released it.
        let reader = {
            let m = Manager {root: f.m.root.clone(), publications: f.m.publications.clone()};
            let (stop, rounds) = (stop.clone(), rounds.clone());
            std::thread::spawn(move || while !stop.load(Ordering::SeqCst) {
                let (guard, _) = m.lock_bounded(OperatorLock::Registry,
                    LockPurpose::OperatorReadback, None, Duration::from_secs(5)).unwrap();
                rounds.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(hold);
                drop(guard);
            })
        };
        while rounds.load(Ordering::SeqCst) < 2 { std::thread::sleep(Duration::from_millis(5)); }
        let started = Instant::now();
        let action = with_operator_registry_wait(Duration::from_secs(3), || f.m.lock("registry.lock"));
        let waited = started.elapsed();
        assert!(action.is_ok(), "the action was starved for {waited:?}");
        // Stated bound: one in-progress reader hold plus one poll (with slack
        // for a loaded test machine), far below the action's own 3 s bound.
        assert!(waited < hold + Duration::from_millis(450), "waited {waited:?}");
        // While the action holds the lock no status read slips in, and the
        // marker is released with the action's wait.
        let before = rounds.load(Ordering::SeqCst);
        std::thread::sleep(hold * 3);
        assert_eq!(rounds.load(Ordering::SeqCst), before);
        assert!(!registry_writer_waiting(&f.m));
        drop(action);
        stop.store(true, Ordering::SeqCst);
        reader.join().unwrap();
    }
    #[test]
    fn real_open_failure_is_not_busy_and_does_not_retry() {
        let f = Fixture::new();
        fs::create_dir(f.m.root.join("registry.lock")).unwrap();
        assert!(!f.m.lock("registry.lock").err().unwrap().is::<LockBusy>());
        let e =
            f.m.lock_bounded(
                OperatorLock::Registry,
                LockPurpose::OperatorValidationReadback,
                None,
                Duration::from_secs(1),
            )
            .err()
            .unwrap();
        let facts = &e.downcast_ref::<AcquisitionFailure>().unwrap().facts;
        assert_eq!(facts.outcome, LockOutcome::Error);
        assert_eq!(facts.attempts, 1);
    }
    #[test]
    fn readback_requires_the_exact_manager_registry_guard() {
        let f = Fixture::new();
        let other = Fixture::new();
        let wrong = f.m.lock("operator-canonical.lock").unwrap();
        assert!(f
            .m
            .managed_status_locked(&f.r.host, &f.r.host_source_sha256, &wrong)
            .is_err());
        let registry = f.m.lock("registry.lock").unwrap();
        assert!(other
            .m
            .managed_status_locked(&other.r.host, &other.r.host_source_sha256, &registry)
            .is_err());
        assert!(f
            .m
            .managed_status_locked(&f.r.host, &f.r.host_source_sha256, &registry)
            .is_ok());
    }
}

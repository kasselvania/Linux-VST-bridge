//! Bounded non-RT acquisition only. Admission keeps Manager::lock fail-fast.
use crate::{operator_model::*, *};
use std::time::{Duration, Instant};
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
            let attempt = self.try_lock(name.filename());
            facts.elapsed_wait_us = start.elapsed().as_micros().min(u64::MAX as u128) as u64;
            match attempt {
                Ok(LockAttempt::Acquired(lock)) => return Ok((lock, facts)),
                Ok(LockAttempt::Busy) if start.elapsed() < timeout => std::thread::sleep(
                    Duration::from_millis(10).min(timeout.saturating_sub(start.elapsed())),
                ),
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

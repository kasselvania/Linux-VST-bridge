//! Completion of one bounded inspection scan: exact readiness, then exact
//! retirement of its owner and lease.
use super::*;

#[cfg(test)]
fn finish_scan(
    child: Child,
    job: &SessionSpec,
    owner: &Path,
    pending: PendingAdmission,
) -> Result<()> {
    finish_scan_bounded(child,job,owner,pending,Duration::from_secs(190))
}
pub(super) fn finish_scan_bounded(
    child: Child,job:&SessionSpec,owner:&Path,pending:PendingAdmission,
    timeout:Duration,
)->Result<()> {
    finish_scan_retirement(child,job,owner,pending,timeout,None)
}
pub(super) fn finish_scan_with_registry(m:&Manager,child:Child,job:&SessionSpec,
    owner:&Path,pending:PendingAdmission)->Result<()> {
    finish_scan_retirement(child,job,owner,pending,Duration::from_secs(190),Some(m))
}
fn finish_scan_retirement(mut child:Child,job:&SessionSpec,owner:&Path,
    mut pending:PendingAdmission,timeout:Duration,manager:Option<&Manager>)->Result<()> {
    // Inspection uses the same supervisor as DSP/editor sessions. Its first
    // stdout record is therefore the preflight-ready ownership boundary, not
    // the terminal retirement receipt. Consume readiness before exposing the
    // lease so the later exact LVO1 comparison cannot mistake LVO0+LVO1 for a
    // malformed retirement.
    if let Err(readiness) = supervisor_ready(&mut child, job, Duration::from_secs(4)) {
        retire_unready_supervisor(&mut child, owner)?;
        return Err(readiness);
    }
    pending.expose();
    let deadline=Instant::now()+timeout;
    let status=loop {
        if let Some(status)=child.try_wait()? {break status;}
        if Instant::now()>=deadline {
            // Contain the supervisor. PendingAdmission retains the lease when
            // its exact retirement receipt is absent; never invent cleanup.
            retire_unready_supervisor(&mut child,owner)?;
            return Err("inspection_supervisor_deadline".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut receipt = String::new();
    if let Some(stdout) = child.stdout.take() {
        stdout.take(128).read_to_string(&mut receipt)?;
    }
    let _registry=manager.map(|m|m.lock_bounded(operator_model::OperatorLock::Registry,
        operator_model::LockPurpose::ServiceRecovery,None,Duration::from_secs(2))).transpose()?;
    pending.complete(
        &job.session,
        status.success(),
        &receipt,
        Some(&job.directory),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_scan_consumes_readiness_then_exact_retirement() {
        let f = test_fixture::Fixture::new();
        let (job, path) = spec(&f.m, f.r.clone().into(), true, true, false).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        let owner = || {
            let mut child = Command::new("/bin/sh")
                .args([
                    "-c",
                    "printf 'LVO0 %s ready\\nLVO1 %s retired\\n' \"$1\" \"$1\"",
                    "scan-owner",
                    &job.session,
                ])
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            // Prove that readiness already buffered by a fast, exited owner
            // remains consumable. Process exit must not outrank the exact
            // lifecycle record written before it.
            while child.try_wait().unwrap().is_none() {
                std::thread::yield_now();
            }
            child
        };
        // Receipt alone cannot hide a still-present transport/session directory.
        let pending = PendingAdmission::new(job.lease.clone(), Arc::new(AdmissionCleanup::default()));
        assert!(finish_scan(owner(), &job, &path, pending).is_err());
        assert!(job.lease.exists());
        fs::remove_dir_all(&job.directory).unwrap();
        let pending = PendingAdmission::new(job.lease.clone(), Arc::new(AdmissionCleanup::default()));
        finish_scan(owner(), &job, &path, pending).unwrap();
        assert!(!job.lease.exists());
    }
    #[test]
    fn terminal_receipt_without_readiness_never_exposes_scan_ownership() {
        let f = test_fixture::Fixture::new();
        let (job, path) = spec(&f.m, f.r.clone().into(), true, true, false).unwrap();
        atomic_json(&job.lease, &job.report).unwrap();
        let blocked = Arc::new(AdmissionCleanup::default());
        let pending = PendingAdmission::new(job.lease.clone(), blocked.clone());
        let child = Command::new("/bin/sh")
            .args([
                "-c",
                "printf 'LVO1 %s retired\\n' \"$1\"",
                "scan-owner",
                &job.session,
            ])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        assert!(finish_scan(child, &job, &path, pending).is_err());
        assert!(!job.lease.exists());
        assert!(!job.directory.exists());
        assert!(!blocked.blocked());
    }
}

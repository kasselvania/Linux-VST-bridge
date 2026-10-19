//! AP18 discovery from the installed ASC environment. No caller-authored IDs,
//! hashes, compatibility flags or paths; this operation never publishes.
use super::*;
use linux_vst_bridge::{
    catalogue::Catalogue,
    observation::{Census, ModuleStamp},
    profiles::{role, Role},
    vendor_application::Application,
};

fn nominate(directory: &Path) -> Result<Artifact> {
    require(
        directory.canonicalize()? == directory,
        "product_module_root_alias",
    )?;
    let mut found = Vec::new();
    for (n, entry) in fs::read_dir(directory)?.enumerate() {
        require(n < 256, "product_module_directory_bound")?;
        let entry = entry?;
        // Filename nominates only. SDK factory/class metadata below authorizes
        // product identity; multiple nominated files are never tried in turn.
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case("Pigments.vst3")
        {
            let path = entry.path();
            require(path.canonicalize()? == path, "product_module_alias")?;
            let bytes = file(&path)?.metadata()?.len();
            require(
                bytes > 0 && bytes <= 512 * 1024 * 1024,
                "product_module_size",
            )?;
            found.push(Artifact {
                sha256: digest(&path)?,
                path,
            });
        }
    }
    require(found.len() == 1, "product_module_absent_or_ambiguous")?;
    Ok(found.remove(0))
}
fn selected_id(records: &[serde_json::Value]) -> Result<String> {
    require(records.len() <= 4096, "inspection_record_bound")?;
    let selected: Vec<_> = records
        .iter()
        .filter(|r| r["state"] == "ap12_class")
        .collect();
    require(selected.len() == 1, "product_class_absent_or_ambiguous")?;
    let id = selected[0]["class_id"]
        .as_str()
        .ok_or("product_class_identity")?;
    require(valid_hex(id, 32), "product_class_identity")?;
    Ok(id.to_uppercase())
}
fn product(c: &Census) -> Result<()> {
    require(
        c.factory_vendor == "Arturia"
            && c.selected.vendor == "Arturia"
            && c.selected.name == "Pigments"
            && role(&c.selected)? == Role::Instrument,
        "product_sdk_identity_mismatch",
    )
}

pub(super) fn finish_scan(
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

pub fn run(m: &Manager, args: &[String]) -> Result<()> {
    require(args == ["scan", "pigments"], "vendor-product scan pigments")?;
    let _guard = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let app: Application = read_json(
        &m.root
            .join("vendor-applications/arturia-software-center/application.json"),
    )?;
    app.verify(&m.root)?;
    let result: vendor_application::OperationResult = read_json(
        &m.root
            .join("vendor-applications/arturia-software-center/operation-result.json"),
    )?;
    require(
        result.retired(),
        "vendor_application_operation_requires_retirement",
    )?;
    let sw = software(m)?;
    let artifact = sw
        .native_catalogue
        .as_ref()
        .ok_or("native_catalogue_absent")?;
    artifact.verify()?;
    let catalogue: Catalogue = read_json(&artifact.path)?;
    catalogue.validate(&m.root)?;
    let bindings: Vec<_> = catalogue
        .environments
        .iter()
        .filter(|b| b.environment == app.environment)
        .collect();
    require(
        bindings.len() == 1,
        "product_environment_absent_or_ambiguous",
    )?;
    let binding = bindings[0].clone();
    let module = nominate(
        &app.environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Common Files/VST3"),
    )?;
    let stamp = ModuleStamp::read(&module.path)?;
    let r = HostBinding {
        // Empty selection is explicit module-level inspection. The existing
        // SDK host refuses multiple audio classes rather than choosing one.
        metadata: ClassSelection {
            class_id: String::new(),
        },
        environment: app.environment,
        module: module.clone(),
        host: sw.host.clone(),
        host_source_sha256: sw.source_sha256.clone(),
        compatibility: Compatibility::default(),
    };
    let (job, path) = spec(m, r, true, true, false)?;
    let pending = PendingAdmission::new(job.lease.clone(), Arc::new(AdmissionCleanup::default()));
    let child = spawn(m, &sw, &path, None)?;
    finish_scan(child, &job, &path, pending)?;
    module.verify()?;
    require(
        ModuleStamp::read(&module.path)? == stamp,
        "product_module_changed_during_scan",
    )?;
    let report = Artifact {
        sha256: digest(&job.report)?,
        path: job.report,
    };
    let value: serde_json::Value = read_json(&report.path)?;
    let id = selected_id(value["records"].as_array().ok_or("inspection_records")?)?;
    let census = Census::from_report(
        binding,
        module,
        stamp,
        sw.host,
        sw.source_sha256,
        report,
        &id,
    )?;
    product(&census)?;
    let directory = m.root.join("observations/pigments");
    private_dir(&directory)?;
    let path = directory.join(format!("{}.json", census.id));
    atomic_json(&path, &census)?;
    println!(
        "{}",
        serde_json::json!({"schema":1,"product":"pigments","activation_permitted":false,"census":path,"metadata":census.selected,"module_sha256":census.module.sha256,"parameter_count":census.parameter_count,"float32":census.float32,"float64":census.float64})
    );
    Ok(())
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
    #[test]
    fn nomination_is_bounded_and_not_sdk_authority() {
        let f = test_fixture::Fixture::new();
        let dir = f.r.module.path.parent().unwrap();
        assert!(nominate(dir).is_err());
        fs::write(dir.join("Pigments.vst3"), b"nomination only").unwrap();
        assert_eq!(
            nominate(dir).unwrap().sha256,
            digest(&dir.join("Pigments.vst3")).unwrap()
        );
        // A differently spelled second entry must fail on case-sensitive Linux.
        fs::write(dir.join("PIGMENTS.VST3"), b"second").unwrap();
        if fs::read_dir(dir).unwrap().count() == 3 {
            assert!(nominate(dir).is_err());
        }
        assert!(!f.m.root.join("observations").exists());
        assert!(!f.m.publications.exists());
    }
    #[test]
    fn automatic_class_selection_refuses_duplicates_and_malformed_ids() {
        let row = serde_json::json!({"state":"ap12_class","class_id":"ab".repeat(16)});
        assert_eq!(
            selected_id(std::slice::from_ref(&row)).unwrap(),
            "AB".repeat(16)
        );
        assert!(selected_id(&[row.clone(), row]).is_err());
        assert!(selected_id(&[]).is_err());
        assert!(
            selected_id(&[serde_json::json!({"state":"ap12_class","class_id":"Pigments"})])
                .is_err()
        );
        let (_, _, mut census, _) = test_fixture::prepared();
        assert!(product(&census).is_err());
        census.factory_vendor = "Arturia".into();
        census.selected.vendor = "Arturia".into();
        census.selected.name = "Pigments".into();
        assert!(product(&census).is_ok());
        census.selected.subcategories = "Fx|Filter".into();
        assert!(product(&census).is_err());
    }
}

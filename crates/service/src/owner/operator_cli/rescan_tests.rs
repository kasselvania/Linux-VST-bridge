//! Exercise the real scan/census/admission owners; only the Windows SDK child
//! is a bounded first-party process fixture.
use super::*;

struct Gate { entered: PathBuf, release: PathBuf }
impl Gate {
    fn wait(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.entered.exists() {
            assert!(Instant::now() < deadline, "scanner did not reach its bounded gate");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn release(&self) { fs::write(&self.release, b"continue").unwrap(); }
}
impl Drop for Gate {
    fn drop(&mut self) { let _ = fs::write(&self.release, b"continue"); }
}
fn fixture() -> (test_fixture::Fixture, String, Gate) {
    let (f, _) = tests::onboarding_worker_fixture();
    let entered = f.outer.join("scanner-entered");
    let release = f.outer.join("scanner-release");
    let report = f.outer.join("scanner-report.json");
    atomic_json(&report, &json!({"cleanup_confirmed":true,"transport_retired":true,"records":[{
        "state":"ap8_factory","factory":{"vendor_hex":hex(b"First-party fixture")},
        "class_count":1,"classes":[{"tier":"IPluginFactory2.PClassInfo2",
            "raw_tuid_hex":f.r.metadata.class_id,"name_hex":hex(b"Reference"),
            "category_hex":hex(b"Audio Module Class"),"subcategories_hex":hex(b"Fx"),
            "vendor_hex":"","version_hex":hex(b"1.0")}]
    }]})).unwrap();
    let script = f.m.root.join("software/scanner-gate.py");
    fs::write(&script, format!(r#"import json,pathlib,shutil,sys,time
job=json.load(open(sys.argv[1]))
print('LVO0 '+job['session']+' ready',flush=True)
pathlib.Path({entered}).write_text(job['session'])
deadline=time.monotonic()+20
while not pathlib.Path({release}).exists() and time.monotonic()<deadline: time.sleep(.01)
released=pathlib.Path({release}).exists()
with open({report},'rb') as source,open(job['report'],'wb') as target: target.write(source.read())
shutil.rmtree(job['directory'])
print('LVO1 '+job['session']+' retired',flush=True)
sys.exit(0 if released else 1)
"#, entered=serde_json::to_string(&entered).unwrap(),
        release=serde_json::to_string(&release).unwrap(), report=serde_json::to_string(&report).unwrap())).unwrap();
    let mut sw = software(&f.m).unwrap();
    test_fixture::set_source_supervisor(&mut sw, &script);
    atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
    let owner = tests::reserve_test_scan(&f.m, ui::Action::EnvironmentRescan {
        environment:f.r.environment.id.clone(),
    }).unwrap();
    (f, owner, Gate { entered, release })
}
fn sibling(f: &test_fixture::Fixture) -> Registration {
    let mut sibling = f.r.clone();
    sibling.metadata.class_id = "02".repeat(16);
    sibling.environment.id = "33".repeat(16);
    sibling.environment.root = f.m.root.join("environments").join(&sibling.environment.id);
    private_dir(&sibling.environment.root).unwrap();
    atomic_json(&sibling.environment.root.join("environment.json"), &sibling.environment).unwrap();
    sibling.module.path = sibling.environment.root.join("compatdata/pfx/drive_c/other.vst3");
    private_dir(sibling.module.path.parent().unwrap()).unwrap();
    fs::copy(&f.r.module.path, &sibling.module.path).unwrap();
    f.m.register(sibling.clone()).unwrap();
    sibling
}
fn limits(f: &test_fixture::Fixture, sibling: &Registration) -> capacity::Limits {
    capacity::Limits { global_dsp:4, maintenance:1, service_workers:16, native_image_hard:4,
        classes:vec![capacity::ClassLimit {class_id:f.r.key(),dsp:2},
            capacity::ClassLimit {class_id:sibling.key(),dsp:2}] }
}

fn mixed_reused_and_scanned_modules(retry: bool) {
    let (f, initial_owner, gate) = fixture();
    let path = f.r.module.path.with_file_name("z.vst3");
    fs::write(&path, b"second first-party module").unwrap();
    let second = Artifact {sha256:digest(&path).unwrap(),path};
    let path = f.outer.join("scanner-report.json");
    let report = Artifact {sha256:digest(&path).unwrap(),path};
    let retained_report = fs::read(&report.path).unwrap();
    let retained = inventory::Module {artifact:f.r.module.clone(),classes:vec![],
        report:report.clone(),inspection_error:Some("first module's retained refusal".into()),
        quarantine_reason:Some("inventory_factory_absent_or_duplicate".into())};
    let mut prior = inventory::Scan {schema:1,id:"ab".repeat(16),
        environment:f.r.environment.clone(),host:f.r.host.clone(),
        host_source_sha256:f.r.host_source_sha256.clone(),completed_at:1,
        changes:Default::default(),modules:vec![retained.clone()]};
    if retry {
        prior.modules.push(inventory::Module {artifact:second.clone(),..retained.clone()});
    }
    let current = f.m.root.join("inventory").join(format!("{}.json", f.r.environment.id));
    private_dir(current.parent().unwrap()).unwrap();
    atomic_json(&current, &prior).unwrap();
    let owner = if retry {
        maintenance::release(&f.m, &initial_owner).unwrap();
        tests::reserve_test_scan(&f.m, ui::Action::QuarantinedModuleRetry {
            environment:f.r.environment.id.clone(),scan:prior.id.clone(),module_index:1,
            module_sha256:second.sha256.clone(),report_sha256:report.sha256.clone(),
        }).unwrap()
    } else { initial_owner };
    std::thread::scope(|threads| {
        let scan = threads.spawn(|| {
            if retry {
                retry_quarantined_module(&f.m, &f.r.environment.id, &prior.id, 1,
                    &second.sha256, &report.sha256, &owner)
            } else { rescan(&f.m, &f.r.environment.id, &owner) }
        });
        let gate = gate;
        gate.wait();
        let session = fs::read_to_string(&gate.entered).unwrap();
        let path = f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(session).join("owner.json");
        let job: SessionSpec = read_json(&path).unwrap();
        assert_eq!(job.registration.module, second);
        gate.release();
        let result = scan.join().unwrap().unwrap();
        assert_eq!(result["modules"], 2);
        let completed: inventory::Scan = read_json(&current).unwrap();
        assert_eq!(serde_json::to_value(&completed.modules[0]).unwrap(),
            serde_json::to_value(&retained).unwrap());
        assert_eq!(completed.modules[1].artifact, second);
        assert_eq!(completed.modules[1].classes.len(), 1);
        assert!(completed.modules[1].quarantine_reason.is_none());
        assert!(completed.modules[1].inspection_error.is_none());
        assert_eq!(fs::read(&report.path).unwrap(), retained_report);
        assert!(capacity::owners(&f.m).unwrap().is_empty());
        maintenance::release(&f.m, &owner).unwrap();
    });
}

#[test]
fn reused_quarantine_report_is_prepared_before_a_later_new_module_scan() {
    mixed_reused_and_scanned_modules(false);
}

#[test]
fn exact_retry_prepares_the_retained_sibling_before_scanning_its_later_target() {
    mixed_reused_and_scanned_modules(true);
}

#[test]
fn changed_prepared_runtime_refuses_before_scanner_spawn_and_removes_unexposed_record() {
    let (f, owner, _gate) = fixture();
    let input = f.r.environment.runner.files[0].path.clone();
    SCAN_PREPARED_BARRIER.with(|barrier| *barrier.borrow_mut() = Some(Box::new(move || {
        fs::write(input, b"changed after preparation").unwrap();
    })));
    let registry = fs::read(f.m.root.join("registry.json")).unwrap();
    let publications = test_fixture::snapshot(&f.m.publications);
    SCAN_SPAWN_COUNT.with(|count| count.set(0));
    let error = rescan(&f.m, &f.r.environment.id, &owner).unwrap_err().to_string();
    assert_eq!(error, "verified_scan_input_changed");
    assert_eq!(SCAN_SPAWN_COUNT.with(std::cell::Cell::get), 0);
    assert!(capacity::owners(&f.m).unwrap().is_empty());
    let sessions = f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions");
    assert_eq!(fs::read_dir(sessions).unwrap().count(), 0);
    assert!(!f.m.root.join("inventory").exists());
    assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), registry);
    assert_eq!(test_fixture::snapshot(&f.m.publications), publications);
}

#[test]
fn blocked_scanner_keeps_capacity_and_unrelated_native_admission_available() {
    let (f, owner, gate) = fixture();
    let sibling = sibling(&f);
    let limits = limits(&f, &sibling);
    let retained = test_fixture::snapshot(&f.m.publications);
    std::thread::scope(|threads| {
        let scan = threads.spawn(|| rescan(&f.m, &f.r.environment.id, &owner));
        // Drop this guard before scope joins even if an assertion fails.
        let gate = gate;
        gate.wait();
        let value = capacity_value(&f.m, &limits, 1, false);
        assert_eq!(value["ok"], true, "{value}");
        assert_eq!(value["capacity"]["maintenance"], 1);
        let cleanup = AdmissionCleanup::default();
        let affected = capacity::reserve_environment(&f.m, &limits, &f.r.key(), &f.r.environment,
            || native_admission_blocked(&f.m, &cleanup, &f.r.environment)).err().unwrap();
        assert!(matches!(affected.downcast_ref::<capacity::Refusal>(),
            Some(capacity::Refusal::MaintenanceActive)), "{affected}");
        let (job, path) = {
            let _admission = capacity::reserve_environment(&f.m, &limits, &sibling.key(), &sibling.environment,
                || native_admission_blocked(&f.m, &cleanup, &sibling.environment)).unwrap();
            let (job, path) = spec(&f.m, sibling.clone().into(), false, true, false).unwrap();
            f.m.retain_lease_owner(&path).unwrap();
            atomic_json(&job.lease, &job.report).unwrap();
            (job, path)
        };
        let retained_owner = fs::read(&path).unwrap();
        let during = capacity::status(&f.m, limits.clone(), 2, false).unwrap();
        assert_eq!(during.dsp, 1);
        assert!(during.owners.iter().any(|row| row.session == job.session && row.kind == capacity::Kind::Dsp));
        gate.release();
        let completed = scan.join().unwrap().unwrap();
        assert_eq!(completed["modules"], 1);
        let after = capacity::status(&f.m, limits, 1, false).unwrap();
        assert_eq!(after.dsp, 1);
        assert_eq!(after.maintenance, 0);
        assert_eq!(fs::read(path).unwrap(), retained_owner);
        assert!(job.lease.exists());
        assert_eq!(test_fixture::snapshot(&f.m.publications), retained);
        // The scan never releases someone else's instance or its own setup
        // reservation; the accepted operation's normal finalizer owns that.
        assert_eq!(maintenance::owned(&f.m, &owner).unwrap(), Some(f.r.environment.clone()));
        maintenance::release(&f.m, &owner).unwrap();
        assert!(job.lease.exists());
    });
}

#[test]
fn scan_rechecks_target_inputs_before_inventory_commit_without_stale_overwrite() {
    for race in ["inventory", "publication", "runtime", "module_added", "custody"] {
        let (f, owner, gate) = fixture();
        let inventory = f.m.root.join("inventory");
        let current = inventory.join(format!("{}.json", f.r.environment.id));
        private_dir(&inventory).unwrap();
        // No module is reusable: the actual supervisor must run in every case.
        atomic_json(&current, &inventory::Scan {schema:1,id:"ab".repeat(16),
            environment:f.r.environment.clone(),host:f.r.host.clone(),
            host_source_sha256:f.r.host_source_sha256.clone(),completed_at:1,
            changes:Default::default(),modules:vec![]}).unwrap();
        std::thread::scope(|threads| {
            let scan = threads.spawn(|| rescan(&f.m, &f.r.environment.id, &owner));
            let gate = gate;
            gate.wait();
            {
                let _registry = f.m.lock("registry.lock").unwrap();
                match race {
                    "inventory" => {
                        let mut changed: inventory::Scan = read_json(&current).unwrap();
                        changed.id = "cd".repeat(16);
                        atomic_json(&current, &changed).unwrap();
                    }
                    "publication" => {
                        let mut changed = f.m.registry().unwrap();
                        changed.classes.get_mut(&f.r.key()).unwrap().publication = Publication::Removed;
                        atomic_json(&f.m.root.join("registry.json"), &changed).unwrap();
                    }
                    "runtime" => fs::write(&f.r.environment.runner.files[0].path, b"changed pinned runtime").unwrap(),
                    "module_added" => fs::write(f.r.module.path.with_file_name("added.vst3"), b"new module").unwrap(),
                    "custody" => {
                        let path = job_dir(&f.m, &owner).unwrap().join("maintenance.json");
                        let mut changed: Value = read_json(&path).unwrap();
                        changed["held"] = false.into();
                        atomic_json(&path, &changed).unwrap();
                    }
                    _ => unreachable!(),
                }
            }
            let retained_inventory = test_fixture::snapshot(&inventory);
            let retained_registry = fs::read(f.m.root.join("registry.json")).unwrap();
            let retained_publications = test_fixture::snapshot(&f.m.publications);
            gate.release();
            let error = scan.join().unwrap().unwrap_err().to_string();
            let expected = match race {
                "inventory" => "control_record_digest_changed",
                "publication" => "inventory_publication_changed",
                "runtime" => "compatibility runtime recorded for this setup",
                "module_added" => "inventory_module_directory_changed",
                "custody" => "setup_maintenance_owner_missing",
                _ => unreachable!(),
            };
            assert!(error.contains(expected), "{race}: {error}");
            assert_eq!(test_fixture::snapshot(&inventory), retained_inventory, "{race}");
            assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), retained_registry, "{race}");
            assert_eq!(test_fixture::snapshot(&f.m.publications), retained_publications, "{race}");
            assert!(capacity::owners(&f.m).unwrap().is_empty(), "{race}");
        });
    }
}

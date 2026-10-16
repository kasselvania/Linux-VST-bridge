use super::*;
use crate::operator_cli::tests::{capacity_fixture, onboarding_worker_fixture};

const NAME: &str = "ValhallaSpaceModulator version 1.1.6v3";
const COMMAND: &str = "\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\"";
const UNINSTALLER: &str = "Program Files/Valhalla DSP, LLC/unins000.exe";
const MODULE: &str = "Program Files/Common Files/VST3/ValhallaSpaceModulator.vst3";
const KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ValhallaSpaceModulator_is1";
const HIVE: &str = r#"WINE REGISTRY Version 2
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ValhallaSpaceModulator_is1] 1729030000
"DisplayName"="ValhallaSpaceModulator version 1.1.6v3"
"UninstallString"="\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\""
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Unrelated] 1729030000
"DisplayName"="Something this setup did not install"
"UninstallString"="\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\""
"#;
/// The hive after the vendor's uninstaller removed its own key.
const HIVE_AFTER: &str = r#"WINE REGISTRY Version 2
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Unrelated] 1729030000
"DisplayName"="Something this setup did not install"
"UninstallString"="\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\""
"#;

struct Installed {
    f: test_fixture::Fixture,
    record: onboarding::Record,
    registration: String,
    module: PathBuf,
    class: String,
}
/// The supervisor's own identity for a registration, as the parser reads it.
fn registration_named(environment: &Environment, name: &str) -> String {
    uninstall_entry::read(environment).unwrap().into_iter()
        .find(|entry| entry.display_name.as_deref() == Some(name)).unwrap().id
}

/// A setup whose own installer run added one Uninstall registration and the
/// plug-in module it covers; the run closed completely.
fn installed() -> Installed {
    let (f, first) = onboarding_worker_fixture();
    let request: ui::Request = read_json(&job_dir(&f.m, &first).unwrap().join("request.json")).unwrap();
    let created = execute_with_receipt_policy(&f.m, &request.action, Some(&first),
        &|| capacity_fixture(&f.m), OPERATOR_WAIT, &mut vec![]).unwrap();
    let environment = created["onboarding"].as_str().unwrap().to_owned();
    let record = onboarding::history_records(&f.m).unwrap().into_iter().find(|r| r.id == environment).unwrap();
    let drive = drive_c(&record.environment);
    private_dir(&drive.join("Program Files/Valhalla DSP, LLC")).unwrap();
    fs::write(drive.join(UNINSTALLER), b"MZ uninstaller").unwrap();
    private_dir(&drive.join("Program Files/Common Files/VST3")).unwrap();
    let module = drive.join(MODULE);
    fs::write(&module, b"MZ module").unwrap();
    fs::write(record.environment.root.join("compatdata/pfx/system.reg"), HIVE).unwrap();
    let registration = registration_named(&record.environment, NAME);
    assert!(uninstall_entry::read(&record.environment).unwrap().iter().any(|e| e.id == registration && e.key == KEY));
    let operation = random_id().unwrap();
    let directory = onboarding::directory(&f.m, &environment).unwrap();
    let path = directory.join("record.json");
    let mut raw: Value = read_json(&path).unwrap();
    raw["installation_operation"] = operation.clone().into();
    atomic_json(&path, &raw).unwrap();
    atomic_json(&directory.join(format!("{operation}-result.json")), &json!({"schema":2,"operation":operation,
        "state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
    witness(&directory.join(format!("{operation}-transaction-private.json")), &[MODULE, UNINSTALLER],
        &[&registration], &drive, &[]);
    let sw = software(&f.m).unwrap();
    let class = "5D".repeat(16);
    let artifact = Artifact {sha256:digest(&module).unwrap(),path:module.clone()};
    let scan = inventory::Scan {schema:1,id:random_id().unwrap(),environment:record.environment.clone(),
        host:sw.host.clone(),host_source_sha256:sw.source_sha256.clone(),completed_at:1,changes:Default::default(),
        modules:vec![inventory::Module {artifact:artifact.clone(),
            classes:vec![inventory::Class {id:class.clone(),name:"ValhallaSpaceModulator".into(),
                vendor:"Valhalla DSP, LLC".into(),version:"1.1.6".into(),category:"Audio Module Class".into(),
                subcategories:"Fx".into(),role:"effect".into()}],
            report:artifact,inspection_error:None,quarantine_reason:None}]};
    private_dir(&f.m.root.join("inventory")).unwrap();
    atomic_json(&f.m.root.join("inventory").join(format!("{environment}.json")), &scan).unwrap();
    let record = onboarding::history_records(&f.m).unwrap().into_iter().find(|r| r.id == environment).unwrap();
    Installed { f, record, registration, module, class }
}
/// An installer run's private witness, as the supervisor records it.
fn witness(path: &Path, files: &[&str], registrations: &[&str], drive: &Path, incomplete: &[&str]) {
    let after: serde_json::Map<String, Value> = files.iter().map(|file| ((*file).to_owned(),
        json!({"size":1,"sha256":digest(&drive.join(file)).ok()}))).collect();
    atomic_json(path, &json!({"schema":1,"after":{"files":after},"durable_installation":{
        "delta":{"files":{"added":files,"changed":[],"removed":[]},
            "uninstall":{"added":registrations,"changed":[],"removed":[]}},
        "incomplete":incomplete}})).unwrap();
}
fn card(s: &Installed) -> ui::InstallerSetup {
    ui::InstallerSetup {installer:s.record.installer.clone(),name:"Valhalla".into(),label_source:"user".into(),
        byte_size:1,format:"pe_executable".into(),imported_at:1,phase:ui::SetupPhase::DiscoveryComplete,
        status:String::new(),journey:None,environment:Some(s.record.id.clone()),compatibility:None,
        discovered:vec![],primary:None,secondary:vec![],
        rename:action("Rename",ui::Action::InstallerRename {installer:s.record.installer.clone(),label:String::new()},None),
        history:vec![]}
}
/// The accepted request a worker would have retained for this operation.
fn accepted(s: &Installed, operation: &str) {
    let directory = job_dir(&s.f.m, operation).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("request.json"), &ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:"t".into(),
        action:ui::Action::ProductUninstall {environment:s.record.id.clone(),registration:s.registration.clone(),
            command_sha256:uninstall_entry::command_sha256(COMMAND),confirmed:true}}).unwrap();
}
/// Run the whole action with a fixture in place of the supervised unit.
fn run(s: &Installed, supervisor: impl Fn(&Manager, &Record) -> Result<()>) -> (String, Result<Value>) {
    let operation = random_id().unwrap();
    accepted(s, &operation);
    let launch = |m: &Manager, record: &Record| supervisor(m, record);
    let mut running = || Ok(());
    let outcome = execute(&s.f.m, &s.record.id, &s.registration, &uninstall_entry::command_sha256(COMMAND), &operation,
        &mut Hooks {launch:&launch, is_live:&|_| Ok(false), vendor_retired:&|| Ok(true), running:&mut running,
            rescan:&|_, _| Ok(json!({}))});
    (operation, outcome)
}
fn report(m: &Manager, record: &Record, state: &str, raw_exit: i64) {
    atomic_json(&result_path(m, &record.operation).unwrap(), &json!({"schema":2,"operation":record.operation,
        "state":state,"raw_exit":raw_exit,"cleanup_confirmed":true,"owned_live":0})).unwrap();
}

#[test]
fn only_registrations_this_setup_installed_are_offered_with_their_plugins() {
    let s = installed();
    let offered = project(&s.f.m, &[card(&s)], std::slice::from_ref(&s.record), &s.f.m.registry().unwrap(),
        &[], None).unwrap();
    assert_eq!(offered.len(), 1, "a registration the setup's installers did not add is never offered");
    assert_eq!(offered[0].name, NAME);
    assert_eq!(offered[0].action.label, format!("Uninstall {NAME}"));
    assert_eq!(offered[0].affected, vec!["ValhallaSpaceModulator".to_owned()]);
    assert_eq!(offered[0].affected_classes, vec![s.class.clone()]);
    assert_eq!(offered[0].coverage, INSTALLED_WITH_THIS);
    assert!(matches!(&offered[0].action.action, ui::Action::ProductUninstall {registration, command_sha256,
        confirmed:false, ..} if *registration == s.registration && *command_sha256 == uninstall_entry::command_sha256(COMMAND)));
    let busy = project(&s.f.m, &[card(&s)], std::slice::from_ref(&s.record), &s.f.m.registry().unwrap(),
        &[], Some("Wait for the current scan to finish")).unwrap();
    assert_eq!(busy[0].action.disabled_reason.as_deref(), Some("Wait for the current scan to finish"));
    // A fresh index answers the overview without reading the hives again; a
    // changed hive is read again.
    refresh_index(&s.f.m, &s.record.id).unwrap();
    assert!(index_path(&s.f.m, &s.record.id).exists());
    let cached = indexed(&s.f.m, &s.record, &s.record.environment).unwrap();
    assert_eq!(cached.0.len(), 2);
    fs::write(s.record.environment.root.join("compatdata/pfx/system.reg"), HIVE_AFTER).unwrap();
    assert_eq!(indexed(&s.f.m, &s.record, &s.record.environment).unwrap().0.len(), 1);
}

#[test]
fn attribution_needs_one_registration_per_run_and_a_complete_record() {
    let s = installed();
    let environment = &s.record.environment;
    let drive = drive_c(environment);
    let modules = modules(&s.f.m, environment, &s.f.m.registry().unwrap());
    let entries = || uninstall_entry::read(environment).unwrap();
    let other = registration_named(environment, "Something this setup did not install");
    let files = |list: &[&str]| list.iter().map(|f| (f.to_lowercase(), None)).collect::<BTreeMap<_, _>>();
    // One run added the plug-in's registration and another one (a bundled runtime).
    let bundle = Run {files:files(&[MODULE]),registrations:[s.registration.clone(),other.clone()].into(),incomplete:false};
    let found = choose(environment, entries(), &[bundle], &modules);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|c| c.coverage == UNKNOWN && c.affected.len() == modules.len()),
        "every plug-in of the setup is named, for both");
    // A run that recorded no plug-in files for it.
    let tool = Run {files:files(&["Program Files/Valhalla DSP, LLC/tool.dll"]),registrations:[s.registration.clone()].into(),
        incomplete:false};
    assert_eq!(choose(environment, entries(), &[tool], &modules)[0].coverage, UNKNOWN);
    // A record that hit its bounds is never taken as complete.
    let bounded = Run {files:files(&[MODULE]),registrations:[s.registration.clone()].into(),incomplete:true};
    assert_eq!(choose(environment, entries(), &[bounded], &modules)[0].coverage, UNKNOWN);
    let complete = Run {files:files(&[MODULE]),registrations:[s.registration.clone()].into(),incomplete:false};
    assert_eq!(choose(environment, entries(), &[complete], &modules)[0].coverage, INSTALLED_WITH_THIS);
    // Windows Installer removal only for a key named by its own product code.
    let msi = |key: &str| Entry {id:"aa".repeat(32),hive:"system.reg".into(),key:key.into(),
        display_name:Some("Runtime".into()),uninstall_string:Some("MsiExec.exe /X{0000000A-1111-2222-3333-44444444444B}".into()),
        ..Default::default()};
    let run = Run {files:BTreeMap::new(),registrations:["aa".repeat(32)].into(),incomplete:false};
    let named = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{0000000A-1111-2222-3333-44444444444B}";
    assert_eq!(choose(environment, vec![msi(named)], std::slice::from_ref(&run), &modules).len(), 1);
    assert!(choose(environment, vec![msi("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Other")],
        &[run], &modules).is_empty());
    assert!(drive.join(UNINSTALLER).exists());
}

#[test]
fn only_the_exact_offered_unchanged_program_is_checked() {
    let s = installed();
    let digest = uninstall_entry::command_sha256(COMMAND);
    let plan = check(&s.f.m, &s.record.id, &s.registration, &digest).unwrap();
    assert_eq!(plan.choice.entry.display_name.as_deref(), Some(NAME));
    assert_eq!(plan.image.path, drive_c(&s.record.environment).join(UNINSTALLER));
    assert_eq!(plan.before, s.record.environment);
    assert_eq!(check(&s.f.m, &s.record.id, &s.registration, &"00".repeat(32)).err().unwrap().to_string(), CHANGED);
    let unrelated = registration_named(&s.record.environment, "Something this setup did not install");
    assert_eq!(check(&s.f.m, &s.record.id, &unrelated, &digest).err().unwrap().to_string(), CHANGED);
    // The uninstaller program changed since the setup's installer left it.
    let program = drive_c(&s.record.environment).join(UNINSTALLER);
    fs::write(&program, b"MZ something else").unwrap();
    assert_eq!(check(&s.f.m, &s.record.id, &s.registration, &digest).err().unwrap().to_string(), UNINSTALLER_CHANGED);
    fs::write(&program, b"MZ uninstaller").unwrap();
    // The registration now names a Windows system program: nothing is offered or run.
    fs::write(s.record.environment.root.join("compatdata/pfx/system.reg"), r#"WINE REGISTRY Version 2
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ValhallaSpaceModulator_is1] 1729030000
"DisplayName"="ValhallaSpaceModulator version 1.1.6v3"
"UninstallString"="C:\\windows\\system32\\cmd.exe /c del"
"#).unwrap();
    assert_eq!(check(&s.f.m, &s.record.id, &s.registration, &digest).err().unwrap().to_string(), CHANGED);
}

#[test]
fn a_finished_uninstall_reserves_the_setup_and_reports_what_is_gone() {
    let s = installed();
    let before = environment_install::current_environment(&s.f.m, &s.record.environment).unwrap();
    let (operation, outcome) = run(&s, |m, record| {
        // The vendor's uninstaller removed its files and its own key.
        fs::remove_file(drive_c(&record.environment).join(MODULE)).unwrap();
        fs::write(record.environment.root.join("compatdata/pfx/system.reg"), HIVE_AFTER).unwrap();
        report(m, record, "completed", 0);
        Ok(())
    });
    let outcome = outcome.unwrap();
    assert_eq!(outcome["message"], format!("{NAME} was uninstalled."));
    assert_eq!(outcome["uninstalled"], json!(["ValhallaSpaceModulator"]));
    let record = control(&s.f.m, &operation).unwrap();
    assert_eq!(record.before, before, "the setup's current environment");
    assert_eq!(record.environment.revision, before.revision + 1, "a new revision before the vendor ran");
    // The new revision is owned, so the setup's records still read.
    assert_eq!(environment_install::current_environment(&s.f.m, &s.record.environment).unwrap(), record.environment);
    assert!(onboarding::history_records(&s.f.m).unwrap().iter().any(|r| r.id == s.record.id));
    assert!(settled(&s.f.m, &operation).unwrap() && all_retired(&s.f.m).unwrap());
    assert!(!s.module.exists());
    // The registration is gone, so nothing is offered any more.
    assert!(project(&s.f.m, &[card(&s)], std::slice::from_ref(&s.record), &s.f.m.registry().unwrap(), &[], None)
        .unwrap().is_empty());
}

#[test]
fn a_cancelled_failed_or_declined_uninstall_is_never_reported_as_uninstalled() {
    for (state, raw_exit, expected) in [("cancelled", 0, "didn't finish (it was stopped)"),
        ("failed", 1, "didn't finish (it was cancelled in its window or ended with an error)"),
        ("completed", 0, "is still installed in this setup")] {
        let s = installed();
        let (_, outcome) = run(&s, |m, record| { report(m, record, state, raw_exit); Ok(()) });
        let outcome = outcome.unwrap();
        let message = outcome["message"].as_str().unwrap();
        assert!(message.contains(expected), "{state}: {message}");
        assert_eq!(outcome["uninstalled"], json!([]), "{state}");
        assert!(s.module.exists());
    }
}

#[test]
fn an_uninstaller_that_never_started_or_never_closed_says_so_and_keeps_its_card() {
    let s = installed();
    let (operation, outcome) = run(&s, |m, record| {
        atomic_json(&result_path(m, &record.operation).unwrap(), &json!({"schema":2,"operation":record.operation,
            "state":"failed","reason":"installer_launch_failed","cleanup_confirmed":true,"owned_live":0})).unwrap();
        Err("installer_launch_failed".into())
    });
    let message = outcome.unwrap_err().to_string();
    assert!(message.starts_with(NOT_STARTED), "{message}");
    assert!(settled(&s.f.m, &operation).unwrap(), "nothing can still run, so the bridge resumes");
    // The supervisor started but never confirmed that it closed.
    let s = installed();
    let (operation, outcome) = run(&s, |m, record| {
        fs::write(job_dir(m, &record.operation).unwrap().join("uninstall-spec.json"), b"{}").unwrap();
        atomic_json(&result_path(m, &record.operation).unwrap(), &json!({"schema":2,"operation":record.operation,
            "state":"running","cleanup_confirmed":false,"owned_live":1})).unwrap();
        Ok(())
    });
    assert_eq!(outcome.unwrap_err().to_string(), UNCONFIRMED_CLOSE);
    assert!(!settled(&s.f.m, &operation).unwrap());
    assert!(!all_retired(&s.f.m).unwrap() && !onboarding::all_retired(&s.f.m).unwrap(),
        "other installers and deletions wait for it");
    assert_eq!(unresolved(&s.f.m, &s.record.id, &mut |_| Ok(false)).unwrap(), Some(operation));
}

#[test]
fn a_record_from_an_earlier_build_is_read_for_custody_only() {
    let s = installed();
    let operation = random_id().unwrap();
    let directory = job_dir(&s.f.m, &operation).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("uninstall.json"), &json!({"schema":1,"operation":operation,
        "environment":s.record.environment,"registration":s.registration,"name":NAME,
        "command_sha256":"00","command":{"kind":"msi","product_code":"{0000000A-1111-2222-3333-44444444444B}"},
        "image":{"path":"/x","sha256":"00"},"affected":[],"created_at":1})).unwrap();
    atomic_json(&result_path(&s.f.m, &operation).unwrap(), &json!({"schema":2,"operation":operation,
        "state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
    assert_eq!(records(&s.f.m).unwrap().len(), 1);
    assert!(all_retired(&s.f.m).unwrap() && settled(&s.f.m, &operation).unwrap());
    assert!(control(&s.f.m, &operation).is_err(), "never controlled or run again");
    // The overview still reads.
    assert_eq!(project(&s.f.m, &[card(&s)], std::slice::from_ref(&s.record), &s.f.m.registry().unwrap(), &[], None)
        .unwrap().len(), 1);
}

#[test]
fn an_uninstall_after_a_runtime_change_uses_the_setups_current_runtime() {
    let s = installed();
    let before = environment_install::current_environment(&s.f.m, &s.record.environment).unwrap();
    let mut after = before.clone();
    after.revision += 1;
    after.runner.id = "different-fixture-runtime".into();
    let operation = random_id().unwrap();
    let directory = job_dir(&s.f.m, &operation).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("request.json"), &ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:"t".into(),
        action:ui::Action::EnvironmentRuntimeSelect {environment:before.id.clone(),
            runner:catalogue::runner_key(&after.runner).unwrap(),
            expected_environment:environment_revision::identity(&before).unwrap()}}).unwrap();
    atomic_json(&directory.join("runtime-transition.json"), &environment_revision::RuntimeBinding {
        schema:1,operation:operation.clone(),before:before.clone(),environment:after.clone(),
        affected:vec![environment_revision::RuntimeOwner::Setup {environment:before.clone(),record:{
            let path = onboarding::directory(&s.f.m, &s.record.id).unwrap().join("record.json");
            Artifact {sha256:digest(&path).unwrap(),path}}}],
        state:environment_revision::RuntimeState::Scanned}).unwrap();
    atomic_json(&directory.join("result.json"), &json!({"schema":1,"operation":operation,"state":"completed","result":{}})).unwrap();
    atomic_json(&after.root.join("environment.json"), &after).unwrap();
    let plan = check(&s.f.m, &s.record.id, &s.registration, &uninstall_entry::command_sha256(COMMAND)).unwrap();
    assert_eq!(plan.before, after, "never the setup's original runtime");
}

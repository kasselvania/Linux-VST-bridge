use super::*;
use crate::operator_cli::tests::{capacity_fixture, onboarding_worker_fixture};
use linux_vst_bridge::preparation as prep;

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
            checking:&mut |_, _| Ok(()), rescan:&|_, _| Ok(json!({})), revalidate:&|_, _, _, _| Err("not expected".into())});
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
    let installs = environment_install::records(&s.f.m).unwrap();
    let cached = indexed(&s.f.m, &s.record, &s.record.environment, &installs).unwrap();
    assert_eq!(cached.0.len(), 2);
    fs::write(s.record.environment.root.join("compatdata/pfx/system.reg"), HIVE_AFTER).unwrap();
    assert_eq!(indexed(&s.f.m, &s.record, &s.record.environment, &installs).unwrap().0.len(), 1);
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
    assert_eq!(unresolved(&s.f.m, &s.record.id).unwrap(), Some(operation.clone()));
    // Check that it closed: nothing runs from the setup, so it is recorded as closed.
    let mut held = std::process::Command::new("sleep").arg("30")
        .env("WINEPREFIX", s.record.environment.root.join("compatdata/pfx")).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !fs::read(format!("/proc/{}/environ", held.id())).is_ok_and(|e| e.windows(11).any(|w| w == b"WINEPREFIX=")) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let refused = environment_install::confirm_closed(&s.f.m, &operation, &|_| Ok(false));
    held.kill().unwrap();
    held.wait().unwrap();
    assert!(refused.unwrap_err().to_string().contains("is still running from this setup"));
    assert_eq!(environment_install::confirm_closed(&s.f.m, &operation, &|_| Ok(true)).unwrap_err().to_string(),
        "It is still running. Use Stop instead.");
    let closed = environment_install::confirm_closed(&s.f.m, &operation, &|_| Ok(false)).unwrap();
    assert_eq!(closed["cleanup_confirmed"], true);
    assert!(settled(&s.f.m, &operation).unwrap() && all_retired(&s.f.m).unwrap());
    assert_eq!(unresolved(&s.f.m, &s.record.id).unwrap(), None);
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

/// A setup with a real published plug-in and the setup's own uninstaller,
/// which `covers` the published plug-in or only a second, unregistered one.
struct Published {
    f: test_fixture::Fixture,
    c: prep::Candidate,
    current: RevisionRef,
    record: onboarding::Record,
    registration: String,
}
fn published_setup(covers: bool) -> Published {
    let (f, c, _, _) = preparation_cli::tests::guided_fixture_with_role(false);
    prep::retain_inspection(&f.m, &c.inspection).unwrap();
    prep::record_candidate(&f.m, &c).unwrap();
    let current = prep::enable(&f.m, &c, false).unwrap();
    let environment = c.selection.environment.clone();
    let operation = random_id().unwrap();
    let record = onboarding::Record {schema:1,id:environment.id.clone(),installer:"ab".repeat(32),
        environment:environment.clone(),created_at:1,creation_operation:operation.clone(),
        installation_operation:Some(operation.clone()),published:false,automatic:None,previous_attempt:None};
    let directory = onboarding::directory(&f.m, &record.id).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("record.json"), &record).unwrap();
    atomic_json(&directory.join(format!("{operation}-result.json")), &json!({"schema":2,"operation":operation,
        "state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
    let drive = drive_c(&environment);
    private_dir(&drive.join("Program Files/Valhalla DSP, LLC")).unwrap();
    fs::write(drive.join(UNINSTALLER), b"MZ uninstaller").unwrap();
    private_dir(&drive.join("Program Files/Common Files/VST3")).unwrap();
    fs::write(drive.join(MODULE), b"MZ module").unwrap();
    fs::write(environment.root.join("compatdata/pfx/system.reg"), HIVE).unwrap();
    let registration = registration_named(&environment, NAME);
    let published = c.selection.module.path.strip_prefix(&drive).unwrap().to_string_lossy().into_owned();
    let mut files = vec![UNINSTALLER, MODULE];
    if covers { files.push(&published); }
    witness(&directory.join(format!("{operation}-transaction-private.json")), &files, &[&registration], &drive, &[]);
    // The second plug-in this registration installed, in the setup's scan.
    let path = f.m.root.join("inventory").join(format!("{}.json", environment.id));
    let mut scan: inventory::Scan = read_json(&path).unwrap();
    let artifact = Artifact {sha256:digest(&drive.join(MODULE)).unwrap(),path:drive.join(MODULE)};
    scan.modules.push(inventory::Module {artifact:artifact.clone(),
        classes:vec![inventory::Class {id:"5D".repeat(16),name:"ValhallaSpaceModulator".into(),
            vendor:"Valhalla DSP, LLC".into(),version:"1.1.6".into(),category:"Audio Module Class".into(),
            subcategories:"Fx".into(),role:"effect".into()}],
        report:artifact,inspection_error:None,quarantine_reason:None});
    atomic_json(&path, &scan).unwrap();
    let record = onboarding::history_records(&f.m).unwrap().into_iter().find(|r| r.id == environment.id).unwrap();
    Published {f, c, current, record, registration}
}
/// The setup's scan, read again in its new revision without the files that are gone.
fn rescanned(m: &Manager, environment: Environment) -> Result<Value> {
    let path = m.root.join("inventory").join(format!("{}.json", environment.id));
    let mut scan: inventory::Scan = read_json(&path)?;
    scan.modules.retain(|module| module.artifact.path.exists());
    scan.environment = environment;
    scan.id = random_id()?;
    atomic_json(&path, &scan)?;
    Ok(json!({}))
}
/// The same plug-in, inspected again in its new revision (no Windows run).
fn inspector(c: &prep::Candidate) -> impl Fn(&Manager, &prep::Selection, &Software, Option<profiles::AudioLayoutPolicy>)
    -> Result<prep::Inspection> + '_ {
    move |m, selection, _, _| {
        let mut inspection = c.inspection.clone();
        inspection.selection = selection.clone();
        prep::retain_inspection(m, &inspection)?;
        Ok(inspection)
    }
}
fn run_published(p: &Published, supervisor: impl Fn(&Manager, &Record) -> Result<()>,
    is_live: &dyn Fn(&str) -> Result<bool>) -> Result<Value> {
    let inspect = inspector(&p.c);
    let revalidate = |m: &Manager, class: &str, current: &RevisionRef, work: &str|
        preparation_cli::revalidate_in_place(m, class, current, work, &inspect);
    run_published_with(p, supervisor, is_live, &revalidate).1
}
/// As `run_published`, with the re-check supplied; returns the operation too.
fn run_published_with(p: &Published, supervisor: impl Fn(&Manager, &Record) -> Result<()>,
    is_live: &dyn Fn(&str) -> Result<bool>, revalidate: &dyn Fn(&Manager, &str, &RevisionRef, &str) -> Result<RevisionRef>)
    -> (String, Result<Value>) {
    let operation = random_id().unwrap();
    let directory = job_dir(&p.f.m, &operation).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("request.json"), &ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:"t".into(),
        action:ui::Action::ProductUninstall {environment:p.record.id.clone(),registration:p.registration.clone(),
            command_sha256:uninstall_entry::command_sha256(COMMAND),confirmed:true}}).unwrap();
    let launch = |m: &Manager, record: &Record| supervisor(m, record);
    let mut running = || Ok(());
    let outcome = execute(&p.f.m, &p.record.id, &p.registration, &uninstall_entry::command_sha256(COMMAND), &operation,
        &mut Hooks {launch:&launch, is_live, vendor_retired:&|| Ok(true), running:&mut running,
            checking:&mut |_, _| Ok(()), rescan:&rescanned, revalidate});
    (operation, outcome)
}
fn loadable(m: &Manager, class: &str) -> linux_vst_bridge::Entry {
    let entry = m.registry().unwrap().classes[class].clone();
    assert_eq!(entry.publication, Publication::Published);
    entry.registration.verify(&m.root).unwrap();
    assert!(linux_vst_bridge::publication::physical(&m.link(class)).unwrap().is_some());
    entry
}

#[test]
fn a_declined_uninstall_puts_its_plugins_back_checked_again() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    // "No" in the vendor's own prompt.
    let outcome = run_published(&p, |m, record| { report(m, record, "failed", 1); Ok(()) }, &|_| Ok(false)).unwrap();
    let message = outcome["message"].as_str().unwrap();
    assert!(message.contains("nothing is reported as uninstalled"), "{message}");
    assert!(message.contains(&format!("{name} is back in your DAW, checked again with this setup.")), "{message}");
    let entry = loadable(&p.f.m, &class);
    assert_ne!(entry.managed_revision.as_ref(), Some(&p.current), "checked again, as a new publication");
    assert_eq!(entry.registration.environment.revision, p.record.environment.revision + 1);
    assert_eq!(outcome["back_in_daw"], json!([name]));
}

#[test]
fn the_setups_other_plugins_are_checked_again_or_named_when_their_files_are_gone() {
    let p = published_setup(false);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    let uninstalled = |m: &Manager, record: &Record| {
        fs::remove_file(drive_c(&record.environment).join(MODULE)).unwrap();
        fs::write(record.environment.root.join("compatdata/pfx/system.reg"), HIVE_AFTER).unwrap();
        report(m, record, "completed", 0);
        Ok(())
    };
    let outcome = run_published(&p, uninstalled, &|_| Ok(false)).unwrap();
    let message = outcome["message"].as_str().unwrap();
    assert!(message.starts_with(&format!("{NAME} was uninstalled.")), "{message}");
    assert!(message.contains(&format!("{name} was checked again and stays in your DAW.")), "{message}");
    let entry = loadable(&p.f.m, &class);
    assert_eq!(entry.registration.environment.revision, p.record.environment.revision + 1, "loadable in the new revision");
    // The uninstaller also removed another plug-in's files: named, and out of the DAW.
    let p = published_setup(false);
    let module = p.c.selection.module.path.clone();
    let outcome = run_published(&p, |m, record| {
        fs::remove_file(&module).unwrap();
        uninstalled(m, record)
    }, &|_| Ok(false)).unwrap();
    assert_eq!(outcome["also_removed"], json!([name.clone()]));
    assert!(outcome["message"].as_str().unwrap().contains(&format!("{name} also left your DAW because its files are gone.")));
    assert_eq!(p.f.m.registry().unwrap().classes[&class].publication, Publication::Removed);
}

#[test]
fn add_back_checks_the_same_file_again_after_its_setup_changed() {
    let p = published_setup(false);
    let class = p.c.selection.class.id.clone();
    // The musician took it out first; an uninstall of other software then changed its setup.
    setup_removal::remove_from_daw(&p.f.m, &class, &ui::PublicationIdentity {id:p.current.id.clone(),
        sha256:p.current.sha256.clone()}).unwrap();
    run_published(&p, |m, record| { report(m, record, "completed", 0); Ok(()) }, &|_| Ok(false)).unwrap();
    let entry = p.f.m.registry().unwrap().classes[&class].clone();
    assert_eq!(entry.publication, Publication::Removed, "the musician's own removal stays");
    assert!(setup_removal::setup_changed(&p.f.m, &class, &p.current).unwrap());
    let inspect = inspector(&p.c);
    let added = setup_removal::add_back(&p.f.m, &class, &ui::PublicationIdentity {id:p.current.id.clone(),
        sha256:p.current.sha256.clone()}, |key, removed|
        preparation_cli::revalidate_in_place(&p.f.m, key, removed, &random_id().unwrap(), &inspect)).unwrap();
    assert!(added["message"].as_str().unwrap().contains("with its setup as it is now"));
    loadable(&p.f.m, &class);
}

#[test]
fn nothing_leaves_the_daw_when_the_uninstaller_may_not_run() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let other = random_id().unwrap();
    // Another installer run is still live.
    let directory = onboarding::directory(&p.f.m, &p.record.id).unwrap();
    let mut raw: Value = read_json(&directory.join("record.json")).unwrap();
    let own = raw["installation_operation"].as_str().unwrap().to_owned();
    raw["installation_operation"] = other.clone().into();
    atomic_json(&directory.join("record.json"), &raw).unwrap();
    fs::rename(directory.join(format!("{own}-result.json")), directory.join(format!("{other}-result.json"))).unwrap();
    let mut result: Value = read_json(&directory.join(format!("{other}-result.json"))).unwrap();
    result["operation"] = other.clone().into();
    atomic_json(&directory.join(format!("{other}-result.json")), &result).unwrap();
    fs::rename(directory.join(format!("{own}-transaction-private.json")),
        directory.join(format!("{other}-transaction-private.json"))).unwrap();
    let live = other.clone();
    let refused = run_published(&p, |_, _| panic!("never launched"), &move |op| Ok(op == live)).unwrap_err();
    assert_eq!(refused.to_string(), OTHER_OWNER);
    loadable(&p.f.m, &class);
    // An interrupted runtime change in the setup.
    let q = published_setup(true);
    let before = q.record.environment.clone();
    let mut after = before.clone();
    after.revision += 1;
    after.runner.id = "different-fixture-runtime".into();
    let change = random_id().unwrap();
    let directory = job_dir(&q.f.m, &change).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("request.json"), &ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:"t".into(),
        action:ui::Action::EnvironmentRuntimeSelect {environment:before.id.clone(),
            runner:catalogue::runner_key(&after.runner).unwrap(),
            expected_environment:environment_revision::identity(&before).unwrap()}}).unwrap();
    atomic_json(&directory.join("runtime-transition.json"), &environment_revision::RuntimeBinding {
        schema:1,operation:change.clone(),before:before.clone(),environment:after,
        affected:vec![environment_revision::RuntimeOwner::Setup {environment:before.clone(),record:{
            let path = onboarding::directory(&q.f.m, &q.record.id).unwrap().join("record.json");
            Artifact {sha256:digest(&path).unwrap(),path}}}],
        state:environment_revision::RuntimeState::Prepared}).unwrap();
    let refused = run_published(&q, |_, _| panic!("never launched"), &|_| Ok(false)).unwrap_err();
    assert_eq!(refused.to_string(), "Finish the interrupted runtime change in this setup first.");
    loadable(&q.f.m, &q.c.selection.class.id);
    // The uninstaller program changed between the check and the reservation.
    let plan = check(&p.f.m, &p.record.id, &p.registration, &uninstall_entry::command_sha256(COMMAND)).unwrap();
    fs::write(drive_c(&p.record.environment).join(UNINSTALLER), b"MZ changed").unwrap();
    let mut running = || Ok(());
    let mut checking = |_: usize, _: usize| Ok(());
    let hooks = Hooks {launch:&|_, _| Ok(()), is_live:&|_| Ok(false), vendor_retired:&|| Ok(true), running:&mut running,
        checking:&mut checking, rescan:&|_, _| Ok(json!({})), revalidate:&|_, _, _, _| Ok(p.current.clone())};
    assert_eq!(reserve(&p.f.m, &plan, &random_id().unwrap(), &[], &hooks).unwrap_err().to_string(), UNINSTALLER_CHANGED);
    assert_eq!(environment_install::current_environment(&p.f.m, &p.record.environment).unwrap(), p.record.environment,
        "no revision was written");
}

/// A supervised run the manager never saw close: the computer or the manager stopped.
fn never_closed(m: &Manager, record: &Record) -> Result<()> {
    fs::write(job_dir(m, &record.operation).unwrap().join("uninstall-spec.json"), b"{}").unwrap();
    atomic_json(&result_path(m, &record.operation).unwrap(), &json!({"schema":2,"operation":record.operation,
        "state":"running","raw_exit":null,"cleanup_confirmed":false,"owned_live":1,"error":null})).unwrap();
    Ok(())
}

#[test]
fn an_uninstaller_shown_closed_later_scans_and_checks_its_setup_again() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    let inspect = inspector(&p.c);
    let revalidate = |m: &Manager, class: &str, current: &RevisionRef, work: &str|
        preparation_cli::revalidate_in_place(m, class, current, work, &inspect);
    let (operation, outcome) = run_published_with(&p, never_closed, &|_| Ok(false), &revalidate);
    assert_eq!(outcome.unwrap_err().to_string(), UNCONFIRMED_CLOSE);
    assert_eq!(p.f.m.registry().unwrap().classes[&class].publication, Publication::Removed, "taken out for the uninstall");
    assert!(needs_check(&p.f.m, &operation).unwrap());
    // Check that it closed records the run as closed; it never finished, so it's a failure.
    let closed = environment_install::confirm_closed(&p.f.m, &operation, &|_| Ok(false)).unwrap();
    assert_eq!(closed["state"], "failed");
    // The first scan fails: nothing is checked, and the check stays pending for the card.
    let mut first = vec![];
    let failed = complete(&p.f.m, &operation, &mut Hooks {launch:&|_, _| panic!("never run again"),
        is_live:&|_| Ok(false), vendor_retired:&|| Ok(true), running:&mut || Ok(()),
        checking:&mut |n, total| { first.push((n, total)); Ok(()) },
        rescan:&|_, _| Err("inventory_scan_failed".into()), revalidate:&revalidate}).unwrap();
    let message = failed["message"].as_str().unwrap();
    assert!(message.contains("its plug-ins weren't checked again; use Finish checking this setup's plug-ins"), "{message}");
    assert!(!message.contains("stays out of your DAW"), "{message}");
    assert_eq!(first, vec![(0, 0)]);
    assert_eq!(unchecked(&p.f.m, &p.record.id).unwrap(), Some(operation.clone()), "the card offers Finish checking");
    let card = environment_install::projection(&p.f.m, &[], None, |_| Ok(false)).unwrap();
    let offer = card.iter().flat_map(|setup| setup.actions.iter())
        .find(|offer| offer.action == ui::Action::InstallerCloseCheck {operation:operation.clone()}).unwrap();
    assert_eq!((offer.label.as_str(), offer.disabled_reason.as_deref()), (FINISH_CHECKING, None));
    // Finish checking: the setup is scanned and the plug-in it took out is back, checked again.
    let mut second = vec![];
    let done = complete(&p.f.m, &operation, &mut Hooks {launch:&|_, _| panic!("never run again"),
        is_live:&|_| Ok(false), vendor_retired:&|| Ok(true), running:&mut || Ok(()),
        checking:&mut |n, total| { second.push((n, total)); Ok(()) }, rescan:&rescanned, revalidate:&revalidate}).unwrap();
    let message = done["message"].as_str().unwrap();
    assert!(message.contains("nothing is reported as uninstalled"), "{message}");
    assert!(message.contains(&format!("{name} is back in your DAW, checked again with this setup.")), "{message}");
    assert_eq!(second, vec![(0, 0), (1, 1)]);
    let entry = loadable(&p.f.m, &class);
    assert_eq!(entry.registration.environment.revision, p.record.environment.revision + 1);
    assert!(!needs_check(&p.f.m, &operation).unwrap() && unchecked(&p.f.m, &p.record.id).unwrap().is_none());
    // Asked again, the recorded result stands; nothing is checked twice.
    let again = complete(&p.f.m, &operation, &mut Hooks {launch:&|_, _| panic!("never run again"),
        is_live:&|_| Ok(false), vendor_retired:&|| Ok(true), running:&mut || Ok(()), checking:&mut |_, _| Ok(()),
        rescan:&|_, _| panic!("scanned twice"), revalidate:&|_, _, _, _| panic!("checked twice")}).unwrap();
    assert_eq!(again, done);
}

#[test]
fn a_recheck_that_stopped_midway_finishes_without_checking_twice() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    let inspect = inspector(&p.c);
    let revalidate = |m: &Manager, class: &str, current: &RevisionRef, work: &str|
        preparation_cli::revalidate_in_place(m, class, current, work, &inspect);
    let (operation, outcome) = run_published_with(&p, never_closed, &|_| Ok(false), &revalidate);
    assert!(outcome.is_err());
    environment_install::confirm_closed(&p.f.m, &operation, &|_| Ok(false)).unwrap();
    // The setup was scanned and the plug-in put back before the manager
    // stopped; the result was never recorded.
    rescanned(&p.f.m, control(&p.f.m, &operation).unwrap().environment).unwrap();
    let current = p.f.m.registry().unwrap().classes[&class].managed_revision.clone().unwrap();
    revalidate(&p.f.m, &class, &current, &random_id().unwrap()).unwrap();
    assert!(needs_check(&p.f.m, &operation).unwrap());
    let done = complete(&p.f.m, &operation, &mut Hooks {launch:&|_, _| panic!("never run again"),
        is_live:&|_| Ok(false), vendor_retired:&|| Ok(true), running:&mut || Ok(()), checking:&mut |_, _| Ok(()),
        rescan:&rescanned, revalidate:&|_, _, _, _| panic!("checked twice")}).unwrap();
    assert!(done["message"].as_str().unwrap().contains(&format!("{name} is back in your DAW")));
    loadable(&p.f.m, &class);
}

#[test]
fn a_failed_recheck_is_named_plainly_and_never_reported_back() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    let (_, outcome) = run_published_with(&p, |m, record| { report(m, record, "failed", 1); Ok(()) }, &|_| Ok(false),
        &|_, _, _, _| Err("unchanged_publication_host_changed".into()));
    let outcome = outcome.unwrap();
    let message = outcome["message"].as_str().unwrap();
    assert!(message.contains(&format!("{name} couldn't be checked again: prepared by another version of the bridge. Run its installer again in this setup, or delete the setup.")), "{message}");
    assert!(!message.contains("add it back"), "it can't be added back as it was: {message}");
    assert!(!message.contains("back in your DAW") && !message.contains('_'), "{message}");
    assert_eq!(outcome["back_in_daw"], json!([]));
    assert_eq!(outcome["recheck_errors"], json!([{"name":name,"reason":"unchanged_publication_host_changed"}]));
    assert_eq!(p.f.m.registry().unwrap().classes[&class].publication, Publication::Removed);
    // Mixed results, one plain sentence per reason.
    let text = reconnected_text(&Reconnected {back:vec!["A".into()], rechecked:vec!["B".into(), "C".into()],
        failed:vec![("D".into(), "unchanged_publication_selection_missing".into()), ("E".into(), "operation already running".into()),
            ("F".into(), "unchanged_publication_selection_missing".into()), ("G".into(), "something_unexpected".into())], ..Default::default()});
    assert!(text.contains(" A is back in your DAW, checked again with this setup."), "{text}");
    assert!(text.contains(" B, C were checked again and stay in your DAW. If you opened them while the bridge was paused, close them and open them again."), "{text}");
    assert!(text.contains(" D, F couldn't be checked again: not in the setup's latest scan. Run their installer again in this setup, or delete the setup."), "{text}");
    assert!(text.contains(" E couldn't be checked again: the bridge was busy; use Finish checking this setup's plug-ins on the setup's card to try again."), "{text}");
    assert!(text.contains(" G couldn't be checked again: the check didn't pass; use Finish checking this setup's plug-ins on the setup's card to try again."), "{text}");
    assert!(!text.contains('_'), "{text}");
}

#[test]
fn an_unreadable_registry_after_closing_still_checks_the_setup_again() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let outcome = run_published(&p, |m, record| {
        let hive = record.environment.root.join("compatdata/pfx/system.reg");
        fs::remove_file(&hive).unwrap();
        private_dir(&hive).unwrap();
        report(m, record, "completed", 0);
        Ok(())
    }, &|_| Ok(false)).unwrap();
    let message = outcome["message"].as_str().unwrap();
    assert!(message.contains("couldn't read this setup's Windows registry, so nothing is reported as uninstalled"), "{message}");
    assert_eq!(outcome["uninstalled"], json!([]));
    assert_eq!(outcome["registration_remains"], Value::Null);
    loadable(&p.f.m, &class);
}

#[test]
fn a_whole_setup_uninstall_puts_back_every_plugin_whose_files_remain() {
    let p = published_setup(false);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    // The record of what the setup's installer added is incomplete: every plug-in is affected.
    let directory = onboarding::directory(&p.f.m, &p.record.id).unwrap();
    let op = p.record.installation_operation.clone().unwrap();
    let drive = drive_c(&p.record.environment);
    witness(&directory.join(format!("{op}-transaction-private.json")), &[UNINSTALLER, MODULE], &[&p.registration],
        &drive, &["file_count"]);
    let (operation, outcome) = run_published_with(&p, |m, record| {
        fs::remove_file(drive_c(&record.environment).join(MODULE)).unwrap();
        fs::write(record.environment.root.join("compatdata/pfx/system.reg"), HIVE_AFTER).unwrap();
        report(m, record, "completed", 0);
        Ok(())
    }, &|_| Ok(false), &|m, class, current, work| preparation_cli::revalidate_in_place(m, class, current, work,
        &inspector(&p.c)));
    assert_eq!(control(&p.f.m, &operation).unwrap().coverage, UNKNOWN);
    let outcome = outcome.unwrap();
    let message = outcome["message"].as_str().unwrap();
    assert!(message.contains(&format!("{name} is back in your DAW, checked again with this setup.")), "{message}");
    assert_eq!(outcome["removed_from_daw"], json!([]));
    loadable(&p.f.m, &class);
}

#[test]
fn add_back_after_a_runtime_change_or_bridge_update_asks_for_check_compatibility() {
    const PLAIN: &str = "setup changed since it was removed, so it can't be added back as it was. Run its installer again in this setup, or delete the setup and set it up again.";
    let p = published_setup(false);
    let class = p.c.selection.class.id.clone();
    let identity = ui::PublicationIdentity {id:p.current.id.clone(), sha256:p.current.sha256.clone()};
    setup_removal::remove_from_daw(&p.f.m, &class, &identity).unwrap();
    run_published(&p, |m, record| { report(m, record, "completed", 0); Ok(()) }, &|_| Ok(false)).unwrap();
    assert!(setup_removal::setup_changed(&p.f.m, &class, &p.current).unwrap());
    let offer = |m: &Manager| setup_removal::plugin_offers(m, &class, &m.registry().unwrap().classes[&class], None, None)
        .into_iter().find(|offer| matches!(offer.action, ui::Action::PluginAddBackToDaw { .. })).unwrap();
    assert_eq!(offer(&p.f.m).disabled_reason, None);
    // The setup's scan was taken by another bridge since it was prepared.
    let path = p.f.m.root.join("inventory").join(format!("{}.json", p.record.id));
    let saved = fs::read(&path).unwrap();
    let mut scan: inventory::Scan = read_json(&path).unwrap();
    scan.host.sha256 = "00".repeat(32);
    atomic_json(&path, &scan).unwrap();
    assert!(!setup_removal::setup_changed(&p.f.m, &class, &p.current).unwrap());
    assert!(offer(&p.f.m).disabled_reason.unwrap().ends_with(PLAIN));
    let refused = setup_removal::add_back(&p.f.m, &class, &identity, |_, _| panic!("no check starts")).unwrap_err();
    assert!(refused.to_string().ends_with(PLAIN), "{refused}");
    fs::write(&path, saved).unwrap();
    assert!(setup_removal::setup_changed(&p.f.m, &class, &p.current).unwrap());
    // The bridge was updated since it was prepared: the installed bridge's
    // runtime isn't the one that prepared it (here, not staged yet).
    let kit = prep::build::recipe_available(&p.f.m).unwrap();
    let staged = prep::build::existing_runtime_record(&p.f.m, &kit.sha256).unwrap().host.path.with_file_name("runtime.json");
    fs::rename(&staged, staged.with_extension("json.aside")).unwrap();
    assert!(!setup_removal::setup_changed(&p.f.m, &class, &p.current).unwrap());
    assert!(offer(&p.f.m).disabled_reason.unwrap().ends_with(PLAIN));
    // The installed bridge's files can't be read just now: a passing state,
    // said plainly, never taken as "updated" or "not updated".
    private_dir(&staged).unwrap();
    assert_eq!(offer(&p.f.m).disabled_reason.as_deref(), Some("The manager couldn't read the bridge's files just now. Try again in a moment."));
    let refused = setup_removal::add_back(&p.f.m, &class, &identity, |_, _| panic!("no check starts")).unwrap_err();
    assert!(refused.to_string().ends_with("Try again in a moment."), "{refused}");
    fs::remove_dir(&staged).unwrap();
    fs::rename(staged.with_extension("json.aside"), &staged).unwrap();
    // The setup's runtime changed.
    let before = environment_install::current_environment(&p.f.m, &p.record.environment).unwrap();
    let mut after = before.clone();
    after.revision += 1;
    after.runner.id = "different-fixture-runtime".into();
    let change = random_id().unwrap();
    let directory = job_dir(&p.f.m, &change).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("request.json"), &ui::Request {schema:ui::OPERATOR_SCHEMA,state_token:"t".into(),
        action:ui::Action::EnvironmentRuntimeSelect {environment:before.id.clone(),
            runner:catalogue::runner_key(&after.runner).unwrap(),
            expected_environment:environment_revision::identity(&before).unwrap()}}).unwrap();
    atomic_json(&directory.join("runtime-transition.json"), &environment_revision::RuntimeBinding {
        schema:1,operation:change.clone(),before:before.clone(),environment:after.clone(),
        affected:vec![environment_revision::RuntimeOwner::Setup {environment:before.clone(),record:{
            let path = onboarding::directory(&p.f.m, &p.record.id).unwrap().join("record.json");
            Artifact {sha256:digest(&path).unwrap(),path}}}],
        state:environment_revision::RuntimeState::Scanned}).unwrap();
    atomic_json(&directory.join("result.json"), &json!({"schema":1,"operation":change,"state":"completed","result":{}})).unwrap();
    atomic_json(&after.root.join("environment.json"), &after).unwrap();
    assert_eq!(environment_install::current_environment(&p.f.m, &p.record.environment).unwrap(), after);
    assert!(!setup_removal::setup_changed(&p.f.m, &class, &p.current).unwrap());
    assert!(offer(&p.f.m).disabled_reason.unwrap().ends_with(PLAIN));
    let refused = setup_removal::add_back(&p.f.m, &class, &identity, |_, _| panic!("no check starts")).unwrap_err();
    assert!(refused.to_string().ends_with(PLAIN), "{refused}");
}

#[test]
fn a_first_installer_run_shown_closed_later_keeps_how_it_ended() {
    let s = installed();
    let op = s.record.installation_operation.clone().unwrap();
    let path = onboarding::directory(&s.f.m, &s.record.id).unwrap().join(format!("{op}-result.json"));
    let unconfirmed = |error: Value| json!({"schema":2,"operation":op,"state":"cleanup_unconfirmed","raw_exit":0,
        "cleanup_confirmed":false,"owned_live":null,"error":error,
        "transaction":{"schema":1,"operation":op,"outcome":"cleanup_unconfirmed","durable_installation":"installed"}});
    // The installer exited 0; only its last helper outlived the supervisor.
    atomic_json(&path, &unconfirmed(Value::Null)).unwrap();
    let closed = environment_install::confirm_closed(&s.f.m, &op, &|_| Ok(false)).unwrap();
    assert_eq!((closed["state"].as_str(), closed["transaction"]["outcome"].as_str()), (Some("completed"), Some("installed")));
    let read = onboarding::result(&s.f.m, &s.record).unwrap();
    assert!(onboarding::retired(&read) && read["state"] == "completed");
    // The supervisor itself stopped first: never a success.
    atomic_json(&path, &unconfirmed("installer_supervisor_terminated_without_retirement".into())).unwrap();
    let closed = environment_install::confirm_closed(&s.f.m, &op, &|_| Ok(false)).unwrap();
    assert_eq!(closed["state"], "failed");
}

#[test]
fn plugins_waiting_for_their_recheck_keep_the_registry_managed() {
    let p = published_setup(false);
    let class = p.c.selection.class.id.clone();
    // The setup moves on, and this plug-in's re-check fails: it stays in the
    // DAW at the setup's older revision until it is checked again.
    let (_, outcome) = run_published_with(&p, |m, record| { report(m, record, "completed", 0); Ok(()) },
        &|_| Ok(false), &|_, _, _, _| Err("unchanged_publication_host_changed".into()));
    outcome.unwrap();
    let entry = p.f.m.registry().unwrap().classes[&class].clone();
    assert_eq!((entry.publication, entry.registration.environment.revision),
        (Publication::Published, p.record.environment.revision));
    // Other setups' work still runs: the registry is still the manager's own.
    assert!(linux_vst_bridge::catalogue::catalogue_free_registry(&p.f.m, &p.f.m.registry().unwrap()).unwrap());
    // A revision no job of the manager owns is never accepted.
    let mut forged = environment_install::current_environment(&p.f.m, &p.record.environment).unwrap();
    forged.revision += 1;
    atomic_json(&forged.root.join("environment.json"), &forged).unwrap();
    assert_eq!(linux_vst_bridge::catalogue::catalogue_free_registry(&p.f.m, &p.f.m.registry().unwrap())
        .unwrap_err().to_string(), "installer_environment_revision_unowned");
}

#[test]
fn a_recheck_that_may_pass_later_stays_offered_until_it_does() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    // The bridge was busy for this plug-in's check.
    let (operation, outcome) = run_published_with(&p, |m, record| { report(m, record, "failed", 1); Ok(()) },
        &|_| Ok(false), &|_, _, _, _| Err("operation already running".into()));
    let message = outcome.unwrap()["message"].as_str().unwrap().to_owned();
    assert!(message.contains(&format!("{name} couldn't be checked again: the bridge was busy; use Finish checking this setup's plug-ins on the setup's card to try again.")), "{message}");
    assert_eq!(unchecked(&p.f.m, &p.record.id).unwrap(), Some(operation.clone()));
    // Finish checking: this time it passes, and the plug-in is back.
    let inspect = inspector(&p.c);
    let revalidate = |m: &Manager, class: &str, current: &RevisionRef, work: &str|
        preparation_cli::revalidate_in_place(m, class, current, work, &inspect);
    let done = complete(&p.f.m, &operation, &mut Hooks {launch:&|_, _| panic!("never run again"),
        is_live:&|_| Ok(false), vendor_retired:&|| Ok(true), running:&mut || Ok(()), checking:&mut |_, _| Ok(()),
        rescan:&rescanned, revalidate:&revalidate}).unwrap();
    assert!(done["message"].as_str().unwrap().contains(&format!("{name} is back in your DAW")));
    assert_eq!(unchecked(&p.f.m, &p.record.id).unwrap(), None);
    loadable(&p.f.m, &class);
}

/// A published setup's card, as the overview shows it.
fn card_for(p: &Published) -> ui::InstallerSetup {
    ui::InstallerSetup {installer:p.record.installer.clone(),name:"Valhalla".into(),label_source:"user".into(),
        byte_size:1,format:"pe_executable".into(),imported_at:1,phase:ui::SetupPhase::DiscoveryComplete,
        status:String::new(),journey:None,environment:Some(p.record.id.clone()),compatibility:None,
        discovered:vec![],primary:None,secondary:vec![],
        rename:action("Rename",ui::Action::InstallerRename {installer:p.record.installer.clone(),label:String::new()},None),
        history:vec![]}
}
/// The setup card's offers for this setup.
fn card_actions(m: &Manager, environment: &str, busy: Option<&str>) -> Vec<ui::AvailableAction> {
    environment_install::projection(m, &[], busy, |_| Ok(false)).unwrap().into_iter()
        .filter(|setup| setup.environment == environment).flat_map(|setup| setup.actions).collect()
}

#[test]
fn buffering_changed_on_the_card_is_carried_into_the_recheck() {
    // A plug-in that stays in the DAW, with 256-frame buffering chosen on its card.
    let p = published_setup(false);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    p.f.m.select_delay(&class, 256).unwrap();
    let outcome = run_published(&p, |m, record| { report(m, record, "completed", 0); Ok(()) }, &|_| Ok(false)).unwrap();
    let message = outcome["message"].as_str().unwrap();
    assert!(message.contains(&format!("{name} was checked again and stays in your DAW.")), "{message}");
    let entry = loadable(&p.f.m, &class);
    assert_eq!(p.f.m.load_revision(&class, entry.managed_revision.as_ref().unwrap()).unwrap().performance.added_frames, 256);
    // The plug-in the uninstall took out ("No" in the vendor's prompt) goes back with its settings too.
    let q = published_setup(true);
    let class = q.c.selection.class.id.clone();
    q.f.m.select_delay(&class, 256).unwrap();
    let outcome = run_published(&q, |m, record| { report(m, record, "failed", 1); Ok(()) }, &|_| Ok(false)).unwrap();
    assert!(outcome["message"].as_str().unwrap().contains("is back in your DAW"), "{outcome}");
    let entry = loadable(&q.f.m, &class);
    assert_eq!(q.f.m.load_revision(&class, entry.managed_revision.as_ref().unwrap()).unwrap().performance.added_frames, 256);
}

#[test]
fn a_recheck_that_fails_every_time_is_final_and_blocks_only_another_uninstall() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    // The uninstaller removed something the plug-in needs: its module no longer scans.
    let (operation, outcome) = run_published_with(&p, |m, record| { report(m, record, "failed", 1); Ok(()) },
        &|_| Ok(false), &|_, _, _, _| Err("unchanged_publication_selection_missing".into()));
    let message = outcome.unwrap()["message"].as_str().unwrap().to_owned();
    assert!(message.contains(&format!("{name} couldn't be checked again: not in the setup's latest scan. Run its installer again in this setup, or delete the setup.")), "{message}");
    assert!(!message.contains("add it back"), "{message}");
    // Recorded as the result: nothing waits for it, and the card offers no endless retry.
    assert_eq!(unchecked(&p.f.m, &p.record.id).unwrap(), None);
    assert!(!needs_check(&p.f.m, &operation).unwrap());
    let actions = card_actions(&p.f.m, &p.record.id, None);
    assert!(actions.iter().all(|offer| offer.label != FINISH_CHECKING));
    assert!(actions.iter().all(|offer| offer.disabled_reason.as_deref() != Some(CHECK_PENDING)), "{actions:?}");
    // A failure that passes when tried again keeps Finish checking, and still
    // doesn't block installers, runtime changes or rescans on the card.
    let q = published_setup(true);
    let (operation, _) = run_published_with(&q, |m, record| { report(m, record, "failed", 1); Ok(()) },
        &|_| Ok(false), &|_, _, _, _| Err("active_maintenance_lease".into()));
    assert_eq!(unchecked(&q.f.m, &q.record.id).unwrap(), Some(operation.clone()));
    let actions = card_actions(&q.f.m, &q.record.id, None);
    assert!(actions.iter().any(|offer| offer.label == FINISH_CHECKING && offer.disabled_reason.is_none()));
    assert!(actions.iter().all(|offer| offer.disabled_reason.as_deref() != Some(CHECK_PENDING)), "{actions:?}");
    // Only another uninstall in the setup waits for it.
    let offered = project(&q.f.m, &[card_for(&q)], std::slice::from_ref(&q.record), &q.f.m.registry().unwrap(), &[], None).unwrap();
    assert!(offered.iter().all(|u| u.action.disabled_reason.as_deref() == Some(CHECK_PENDING)), "{offered:?}");
}

#[test]
fn finish_checking_is_offered_while_the_bridge_is_paused_for_it() {
    let p = published_setup(true);
    let (operation, _) = run_published_with(&p, |m, record| { report(m, record, "failed", 1); Ok(()) },
        &|_| Ok(false), &|_, _, _, _| Err("operation already running".into()));
    // The uninstall's worker ended while it still held the bridge's pause, so
    // every other action reads as busy; Finish checking is what releases it.
    write_operation(&p.f.m, &operation, &json!({"schema":2,"operation":operation,"state":"refused"}), false).unwrap();
    {
        let _resume = resume_lock(&p.f.m).unwrap();
        create_resume(&p.f.m, &ResumeRecord {schema:2,owner_operation:operation.clone(),resume:true,
            software:"ab".repeat(32),vendor_operation:None}).unwrap();
    }
    let paused = Some("The bridge is paused");
    let finish = card_actions(&p.f.m, &p.record.id, paused).into_iter()
        .find(|offer| offer.label == FINISH_CHECKING).unwrap();
    assert_eq!(finish.disabled_reason, None);
    assert_eq!(finish.action, ui::Action::InstallerCloseCheck {operation:operation.clone()});
    // A pause held by something else keeps it waiting like any other action.
    {
        let _resume = resume_lock(&p.f.m).unwrap();
        fs::remove_file(p.f.m.root.join("operator/resume.json")).unwrap();
        create_resume(&p.f.m, &ResumeRecord {schema:2,owner_operation:random_id().unwrap(),resume:true,
            software:"ab".repeat(32),vendor_operation:None}).unwrap();
    }
    let finish = card_actions(&p.f.m, &p.record.id, paused).into_iter()
        .find(|offer| offer.label == FINISH_CHECKING).unwrap();
    assert_eq!(finish.disabled_reason.as_deref(), paused);
}

#[test]
fn a_finished_uninstall_keeps_its_own_plugin_out_even_if_its_file_remains() {
    let p = published_setup(true);
    let class = p.c.selection.class.id.clone();
    let name = p.f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    // Uninstalled, but the vendor's uninstaller left the plug-in's own file.
    let outcome = run_published(&p, |m, record| {
        fs::write(record.environment.root.join("compatdata/pfx/system.reg"), HIVE_AFTER).unwrap();
        report(m, record, "completed", 0);
        Ok(())
    }, &|_| Ok(false)).unwrap();
    let message = outcome["message"].as_str().unwrap();
    assert!(message.contains("these plug-ins' files are still there"), "{message}");
    assert!(message.contains(&format!("{name} stays out of your DAW")), "{message}");
    assert!(!message.contains("back in your DAW"), "{message}");
    assert_eq!(p.f.m.registry().unwrap().classes[&class].publication, Publication::Removed);
}

use super::*;
use crate::operator_cli::tests::{capacity_fixture, onboarding_worker_fixture};

const NAME: &str = "ValhallaSpaceModulator version 1.1.6v3";
const COMMAND: &str = "\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\"";
const HIVE: &str = r#"WINE REGISTRY Version 2
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ValhallaSpaceModulator_is1] 1729030000
"DisplayName"="ValhallaSpaceModulator version 1.1.6v3"
"UninstallString"="\"C:\\Program Files\\Valhalla DSP, LLC\\unins000.exe\""
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

/// A setup whose own installer run added one Uninstall registration and the
/// plug-in module it covers; the run closed completely.
fn installed() -> Installed {
    let (f, first) = onboarding_worker_fixture();
    let request: ui::Request = read_json(&job_dir(&f.m, &first).unwrap().join("request.json")).unwrap();
    let created = execute_with_receipt_policy(&f.m, &request.action, Some(&first),
        &|| capacity_fixture(&f.m), OPERATOR_WAIT, &mut vec![]).unwrap();
    let environment = created["onboarding"].as_str().unwrap().to_owned();
    let record = onboarding::history_records(&f.m).unwrap().into_iter().find(|r| r.id == environment).unwrap();
    let drive = record.environment.root.join("compatdata/pfx/drive_c");
    let vendor = drive.join("Program Files/Valhalla DSP, LLC");
    private_dir(&vendor).unwrap();
    fs::write(vendor.join("unins000.exe"), b"MZ uninstaller").unwrap();
    let vst3 = drive.join("Program Files/Common Files/VST3");
    private_dir(&vst3).unwrap();
    let module = vst3.join("ValhallaSpaceModulator.vst3");
    fs::write(&module, b"MZ module").unwrap();
    fs::write(record.environment.root.join("compatdata/pfx/system.reg"), HIVE).unwrap();
    let registration = uninstall_entry::read(&record.environment).unwrap().into_iter()
        .find(|e| e.display_name.as_deref() == Some(NAME)).unwrap().id;
    let operation = random_id().unwrap();
    let directory = onboarding::directory(&f.m, &environment).unwrap();
    let path = directory.join("record.json");
    let mut raw: Value = read_json(&path).unwrap();
    raw["installation_operation"] = operation.clone().into();
    atomic_json(&path, &raw).unwrap();
    atomic_json(&directory.join(format!("{operation}-result.json")), &json!({"schema":2,"operation":operation,
        "state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
    atomic_json(&directory.join(format!("{operation}-transaction-private.json")), &json!({"durable_installation":
        {"delta":{"files":{"added":["Program Files/Common Files/VST3/ValhallaSpaceModulator.vst3",
            "Program Files/Valhalla DSP, LLC/unins000.exe"],"changed":[],"removed":[]},
        "uninstall":{"added":[registration],"changed":[],"removed":[]}}}})).unwrap();
    let sw = software(&f.m).unwrap();
    let class = "5D".repeat(16);
    let module_artifact = Artifact {sha256:digest(&module).unwrap(),path:module.clone()};
    let scan = inventory::Scan {schema:1,id:random_id().unwrap(),environment:record.environment.clone(),
        host:sw.host.clone(),host_source_sha256:sw.source_sha256.clone(),completed_at:1,changes:Default::default(),
        modules:vec![inventory::Module {artifact:module_artifact.clone(),
            classes:vec![inventory::Class {id:class.clone(),name:"ValhallaSpaceModulator".into(),
                vendor:"Valhalla DSP, LLC".into(),version:"1.1.6".into(),category:"Audio Module Class".into(),
                subcategories:"Fx".into(),role:"effect".into()}],
            report:module_artifact,inspection_error:None,quarantine_reason:None}]};
    private_dir(&f.m.root.join("inventory")).unwrap();
    atomic_json(&f.m.root.join("inventory").join(format!("{environment}.json")), &scan).unwrap();
    let record = onboarding::history_records(&f.m).unwrap().into_iter().find(|r| r.id == environment).unwrap();
    Installed { f, record, registration, module, class }
}
fn card(s: &Installed) -> ui::InstallerSetup {
    ui::InstallerSetup {installer:s.record.installer.clone(),name:"Valhalla".into(),label_source:"user".into(),
        byte_size:1,format:"pe_executable".into(),imported_at:1,phase:ui::SetupPhase::DiscoveryComplete,
        status:String::new(),journey:None,environment:Some(s.record.id.clone()),compatibility:None,
        discovered:vec![],primary:None,secondary:vec![],
        rename:action("Rename",ui::Action::InstallerRename {installer:s.record.installer.clone(),label:String::new()},None),
        history:vec![]}
}

#[test]
fn only_registrations_this_setup_installed_are_offered_with_their_plugins() {
    let s = installed();
    let found = choices(&s.f.m, &s.record, &s.f.m.registry().unwrap()).unwrap();
    assert_eq!(found.len(), 1, "a registration the setup's installers did not add is never offered");
    assert_eq!(found[0].entry.id, s.registration);
    assert_eq!(found[0].command, UninstallCommand::Executable {
        relative:"Program Files/Valhalla DSP, LLC/unins000.exe".into(), arguments:vec![]});
    assert_eq!(found[0].affected.iter().map(|a| a.class_id.as_str()).collect::<Vec<_>>(), vec![s.class.as_str()]);
    let offered = project(&s.f.m, &[card(&s)], std::slice::from_ref(&s.record), &s.f.m.registry().unwrap(),
        &[], None, &[]).unwrap();
    assert_eq!(offered.len(), 1);
    assert_eq!(offered[0].name, NAME);
    assert_eq!(offered[0].action.label, format!("Uninstall {NAME}"));
    assert_eq!(offered[0].affected, vec!["ValhallaSpaceModulator".to_owned()]);
    assert_eq!(offered[0].coverage, "installed_with_this");
    assert!(offered[0].action.disabled_reason.is_none());
    assert!(matches!(&offered[0].action.action, ui::Action::ProductUninstall {registration, command_sha256,
        confirmed:false, ..} if *registration == s.registration && *command_sha256 == uninstall_entry::command_sha256(COMMAND)));
    // Another work in progress disables it with that reason.
    let busy = project(&s.f.m, &[card(&s)], std::slice::from_ref(&s.record), &s.f.m.registry().unwrap(),
        &[], Some("Wait for the current scan to finish"), &[]).unwrap();
    assert_eq!(busy[0].action.disabled_reason.as_deref(), Some("Wait for the current scan to finish"));
}

#[test]
fn only_the_exact_offered_command_is_prepared() {
    let s = installed();
    let digest = uninstall_entry::command_sha256(COMMAND);
    let operation = random_id().unwrap();
    let record = prepare(&s.f.m, &s.record.id, &s.registration, &digest, &operation).unwrap();
    assert_eq!(record.name, NAME);
    assert_eq!(record.image.path, s.record.environment.root
        .join("compatdata/pfx/drive_c/Program Files/Valhalla DSP, LLC/unins000.exe"));
    assert_eq!(record.affected.len(), 1);
    assert_eq!(prepare(&s.f.m, &s.record.id, &s.registration, &"00".repeat(32), &operation)
        .unwrap_err().to_string(), CHANGED);
    let unrelated = uninstall_entry::read(&s.record.environment).unwrap().into_iter()
        .find(|e| e.display_name.as_deref() != Some(NAME)).unwrap();
    assert_eq!(prepare(&s.f.m, &s.record.id, &unrelated.id, &digest, &operation).unwrap_err().to_string(), CHANGED);
    // The registration now names a Windows system program: nothing is offered or run.
    fs::write(s.record.environment.root.join("compatdata/pfx/system.reg"), r#"WINE REGISTRY Version 2
[Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ValhallaSpaceModulator_is1] 1729030000
"DisplayName"="ValhallaSpaceModulator version 1.1.6v3"
"UninstallString"="C:\\windows\\system32\\cmd.exe /c del"
"#).unwrap();
    assert!(choices(&s.f.m, &s.record, &s.f.m.registry().unwrap()).unwrap().is_empty());
    assert_eq!(prepare(&s.f.m, &s.record.id, &s.registration, &digest, &operation).unwrap_err().to_string(), CHANGED);
}

#[test]
fn the_result_says_plainly_what_the_uninstaller_removed() {
    let s = installed();
    let operation = random_id().unwrap();
    let record = prepare(&s.f.m, &s.record.id, &s.registration, &uninstall_entry::command_sha256(COMMAND),
        &operation).unwrap();
    let kept = outcome(&record, &["ValhallaSpaceModulator".into()]);
    assert_eq!(kept["still_installed"], json!(["ValhallaSpaceModulator"]));
    assert_eq!(kept["message"], "The uninstaller finished, but ValhallaSpaceModulator's files are still there. It stays removed from your DAW; add it back from Plug-ins if you still want it.");
    fs::remove_file(&s.module).unwrap();
    let gone = outcome(&record, &[]);
    assert_eq!(gone["uninstalled"], json!(["ValhallaSpaceModulator"]));
    assert_eq!(gone["still_installed"], json!([]));
    assert_eq!(gone["message"], format!("{NAME} was uninstalled."));
    assert_eq!(gone["history_retained"], true);
}

#[test]
fn an_uninstaller_that_never_closed_blocks_other_work_like_an_installer() {
    let s = installed();
    assert!(onboarding::all_retired(&s.f.m).unwrap());
    let operation = random_id().unwrap();
    let record = prepare(&s.f.m, &s.record.id, &s.registration, &uninstall_entry::command_sha256(COMMAND),
        &operation).unwrap();
    let directory = job_dir(&s.f.m, &operation).unwrap();
    private_dir(&directory).unwrap();
    atomic_json(&directory.join("uninstall.json"), &record).unwrap();
    atomic_json(&directory.join("uninstall-result.json"), &json!({"schema":2,"operation":operation,
        "state":"running","cleanup_confirmed":false,"owned_live":1})).unwrap();
    assert_eq!(records(&s.f.m).unwrap(), vec![record]);
    assert!(!all_retired(&s.f.m).unwrap());
    assert!(!onboarding::all_retired(&s.f.m).unwrap(), "other installers and deletions wait for it");
    atomic_json(&directory.join("uninstall-result.json"), &json!({"schema":2,"operation":operation,
        "state":"completed","cleanup_confirmed":true,"owned_live":0})).unwrap();
    assert!(all_retired(&s.f.m).unwrap());
}

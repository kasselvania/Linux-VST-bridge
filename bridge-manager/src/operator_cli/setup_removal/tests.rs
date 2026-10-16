use super::*;
use crate::operator_cli::tests::{capacity_fixture, onboarding_worker_fixture};
use linux_vst_bridge::preparation as prep;

fn published() -> (test_fixture::Fixture, prep::Candidate, RevisionRef) {
    let (f, c) = preparation_cli::tests::projection_fixture();
    atomic_json(&f.m.root.join("software.json"), &preparation_cli::tests::projection_software(&c)).unwrap();
    prep::record_candidate(&f.m, &c).unwrap();
    let current = prep::enable(&f.m, &c, false).unwrap();
    (f, c, current)
}
fn product<'a>(snapshot: &'a ui::Snapshot, class: &str) -> &'a ui::Product {
    snapshot.products.iter().find(|p| p.class_id.eq_ignore_ascii_case(class)).unwrap()
}
fn offer(actions: &[ui::AvailableAction], wanted: impl Fn(&ui::Action) -> bool) -> Option<ui::AvailableAction> {
    actions.iter().find(|offer| wanted(&offer.action)).cloned()
}
fn dsp_lease(m: &Manager, environment: &Environment, class: &str) -> PathBuf {
    let sid = random_id().unwrap();
    let report = m.root.join("runtime/results").join(format!("windows-{sid}.json"));
    private_dir(report.parent().unwrap()).unwrap();
    let owner = environment.root.join("compatdata/pfx/drive_c/bridge/sessions").join(&sid).join("owner.json");
    private_dir(owner.parent().unwrap()).unwrap();
    atomic_json(&owner, &json!({"session":sid,"report":report,"keeper":false,"inspect":false,
        "registration":{"metadata":{"class_id":class},"environment":{"id":environment.id,"root":environment.root}}})).unwrap();
    let leases = m.root.join("runtime/leases");
    private_dir(&leases).unwrap();
    let lease = leases.join(format!("{sid}.json"));
    atomic_json(&lease, &report).unwrap();
    lease
}

#[test]
fn removing_and_adding_back_restores_the_same_publication() {
    let (f, c, current) = published();
    let class = c.selection.class.id.clone();
    let target = publication::physical(&f.m.link(&class)).unwrap().unwrap();
    let snapshot = snapshot_idle_test(&f.m).unwrap();
    let remove = offer(&product(&snapshot, &class).actions,
        |a| matches!(a, ui::Action::PluginRemoveFromDaw { .. })).unwrap();
    assert!(remove.disabled_reason.is_none(), "{remove:?}");
    assert_eq!(remove.action, ui::Action::PluginRemoveFromDaw {class_id:class.clone(),
        expected_current:identity(&current)});
    let operation = test_submit_offered(&f.m, &remove.action).unwrap();
    let removed = test_run_offered_worker(&f.m, &operation).unwrap();
    assert_eq!(removed["state"], "completed", "{removed}");
    assert_eq!(removed["result"]["removed_from_daw"], true);
    let entry = f.m.registry().unwrap().classes[&class].clone();
    assert_eq!(entry.publication, Publication::Removed);
    assert_eq!(entry.managed_revision.as_ref(), Some(&current), "the publication record is retained");
    assert!(publication::physical(&f.m.link(&class)).unwrap().is_none());

    let snapshot = snapshot_idle_test(&f.m).unwrap();
    let removed_product = product(&snapshot, &class);
    assert_eq!(removed_product.disposition, "removed_from_daw");
    assert_eq!(removed_product.availability.as_ref().unwrap().reason.as_deref(), Some(REMOVED_FROM_DAW));
    assert!(offer(&removed_product.actions, |a| matches!(a, ui::Action::PluginRemoveFromDaw { .. })).is_none());
    let add = offer(&removed_product.actions, |a| matches!(a, ui::Action::PluginAddBackToDaw { .. })).unwrap();
    assert!(add.disabled_reason.is_none(), "{add:?}");
    assert_eq!(removed_product.compatibility.as_ref().unwrap().primary.as_ref().map(|o| &o.action),
        Some(&add.action), "Add back leads the removed plug-in's card");
    // The plain-status overview agrees with the product detail.
    let current_view = current::capture(&f.m).unwrap();
    assert_eq!(product(&current_view.snapshot, &class).disposition, "removed_from_daw");

    let operation = test_submit_offered(&f.m, &add.action).unwrap();
    let added = test_run_offered_worker(&f.m, &operation).unwrap();
    assert_eq!(added["state"], "completed", "{added}");
    let entry = f.m.registry().unwrap().classes[&class].clone();
    assert_eq!(entry.publication, Publication::Published);
    assert_eq!(entry.managed_revision.as_ref(), Some(&current), "the same publication, not a new revision");
    assert_eq!(publication::physical(&f.m.link(&class)).unwrap(), Some(target));
    assert_eq!(test_submit_offered(&f.m, &add.action).unwrap_err().to_string(), "operator_action_not_available");
}

#[test]
fn a_loaded_plugin_is_refused_in_plain_words_and_left_in_the_daw() {
    let (f, c, current) = published();
    let class = c.selection.class.id.clone();
    let name = f.m.registry().unwrap().classes[&class].registration.metadata.name.clone();
    dsp_lease(&f.m, &c.selection.environment, &class);
    let snapshot = snapshot_for_operation(&f.m, None, OPERATOR_WAIT, &mut vec![],
        &|| capacity_fixture(&f.m)).unwrap();
    let remove = offer(&product(&snapshot, &class).actions,
        |a| matches!(a, ui::Action::PluginRemoveFromDaw { .. })).unwrap();
    assert_eq!(remove.disabled_reason.as_deref(), Some(loaded(&name).as_str()));
    let before = fs::read(f.m.root.join("registry.json")).unwrap();
    let refused = execute_with_receipt_policy(&f.m, &remove.action, Some(&random_id().unwrap()),
        &|| capacity_fixture(&f.m), OPERATOR_WAIT, &mut vec![]).unwrap_err().to_string();
    assert_eq!(refused, loaded(&name));
    // The owner's own check under registry.lock refuses the same way.
    assert_eq!(remove_from_daw(&f.m, &class, &identity(&current)).unwrap_err().to_string(), loaded(&name));
    assert_eq!(fs::read(f.m.root.join("registry.json")).unwrap(), before);
    assert!(publication::physical(&f.m.link(&class)).unwrap().is_some());
}

#[test]
fn stale_identity_and_changed_files_refuse_without_changing_anything() {
    let (f, c, current) = published();
    let class = c.selection.class.id.clone();
    let mut other = identity(&current);
    other.id = "ee".repeat(16);
    assert_eq!(remove_from_daw(&f.m, &class, &other).unwrap_err().to_string(), CHANGED);
    remove_from_daw(&f.m, &class, &identity(&current)).unwrap();
    // A replayed removal of the same publication is already done.
    assert_eq!(remove_from_daw(&f.m, &class, &identity(&current)).unwrap()["already_removed"], true);
    assert_eq!(add_back(&f.m, &class, &other, |_, _| unreachable!()).unwrap_err().to_string(), CHANGED);
    // The setup changed after removal: Add back is offered but disabled.
    let entry = f.m.registry().unwrap().classes[&class].clone();
    let mut changed = entry.registration.environment.clone();
    changed.revision += 1;
    atomic_json(&changed.root.join("environment.json"), &changed).unwrap();
    let offers = plugin_offers(&f.m, &class, &entry, None, None);
    assert_eq!(offers.len(), 1);
    assert!(offers[0].disabled_reason.as_deref().unwrap().contains("Check compatibility"));
}

fn created_setup() -> (test_fixture::Fixture, String, String) {
    let (f, first) = onboarding_worker_fixture();
    let request: ui::Request = read_json(&job_dir(&f.m, &first).unwrap().join("request.json")).unwrap();
    let created = execute_with_receipt_policy(&f.m, &request.action, Some(&first),
        &|| capacity_fixture(&f.m), OPERATOR_WAIT, &mut vec![]).unwrap();
    write_operation(&f.m, &first, &json!({"schema":1,"operation":first,"state":"completed"}), false).unwrap();
    let environment = created["onboarding"].as_str().unwrap().to_owned();
    let installer = onboarding::history_records(&f.m).unwrap().into_iter()
        .find(|r| r.id == environment).unwrap().installer;
    (f, environment, installer)
}
fn delete_offer(snapshot: &ui::Snapshot, installer: &str) -> Option<ui::AvailableAction> {
    snapshot.installer_setups.iter().find(|s| s.installer == installer)
        .and_then(|s| offer(&s.secondary, |a| matches!(a, ui::Action::SetupDelete { .. })))
}

#[test]
fn deleting_a_setup_removes_its_windows_files_and_keeps_its_history() {
    let (f, environment, installer) = created_setup();
    let root = f.m.root.join("environments").join(&environment);
    // Vendor files, a licence and a Wine drive link that points outside.
    let drive = root.join("compatdata/pfx/drive_c/ProgramData/Vendor");
    private_dir(&drive).unwrap();
    fs::write(drive.join("licence.dat"), b"activation").unwrap();
    let outside = f.outer.join("musician-documents");
    private_dir(&outside).unwrap();
    fs::write(outside.join("song.rpp"), b"project").unwrap();
    let devices = root.join("compatdata/pfx/dosdevices");
    private_dir(&devices).unwrap();
    std::os::unix::fs::symlink(&outside, devices.join("z:")).unwrap();
    let locked = root.join("compatdata/pfx/drive_c/locked");
    private_dir(&locked).unwrap();
    fs::write(locked.join("file"), b"x").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o500)).unwrap();
    let inventory = f.m.root.join("inventory");
    private_dir(&inventory).unwrap();
    let sw = software(&f.m).unwrap();
    let bound = onboarding::history_records(&f.m).unwrap().into_iter().find(|r| r.id == environment).unwrap();
    atomic_json(&inventory.join(format!("{environment}.json")), &inventory::Scan {schema:1,id:"ab".repeat(16),
        environment:bound.environment,host:sw.host.clone(),host_source_sha256:sw.source_sha256.clone(),
        completed_at:1,changes:Default::default(),modules:vec![]}).unwrap();
    let record = onboarding::directory(&f.m, &environment).unwrap().join("record.json");
    let record_before = fs::read(&record).unwrap();

    let snapshot = snapshot_idle_test(&f.m).unwrap();
    let delete = delete_offer(&snapshot, &installer).unwrap();
    assert!(delete.disabled_reason.is_none(), "{delete:?}");
    let operation = test_submit_offered(&f.m, &delete.action).unwrap();
    let deleted = test_run_offered_worker(&f.m, &operation).unwrap();
    assert_eq!(deleted["state"], "completed", "{deleted}");
    assert_eq!(deleted["result"]["setup_deleted"], true);

    let mut left: Vec<_> = fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name()).collect();
    left.sort();
    assert_eq!(left, ["environment.json", "operation.lock"], "only the identity record remains");
    assert_eq!(fs::read(outside.join("song.rpp")).unwrap(), b"project", "links are removed, never followed");
    assert_eq!(fs::read(&record).unwrap(), record_before, "installation history is kept in place");
    assert!(!inventory.join(format!("{environment}.json")).exists());
    assert!(deleted_setup::history_directory(&f.m, &environment).unwrap()
        .join(format!("inventory-{environment}.json")).exists());
    let tombstone = deleted_setup::load(&f.m, &environment).unwrap().unwrap();
    assert_eq!(tombstone.state, Deletion::Deleted);
    assert!(onboarding::history_records(&f.m).unwrap().iter().all(|r| r.id != environment));
    assert!(onboarding::retained_environment_record(&f.m, &environment).unwrap().is_none());
    assert_eq!(onboarding::load(&f.m, &environment).unwrap_err().to_string(), "setup_deleted");

    let snapshot = snapshot_idle_test(&f.m).unwrap();
    let card = snapshot.installer_setups.iter().find(|s| s.installer == installer).unwrap();
    assert_eq!(card.environment, None, "the installer can be set up again");
    assert!(delete_offer(&snapshot, &installer).is_none());
    assert_eq!(snapshot.deleted_setups.len(), 1);
    assert_eq!(snapshot.deleted_setups[0].state, "deleted");
    assert!(snapshot.deleted_setups[0].actions.is_empty());
    assert_eq!(test_submit_offered(&f.m, &delete.action).unwrap_err().to_string(), "operator_action_not_available");
    // A replay of the same accepted request is a no-op.
    assert_eq!(delete_setup(&f.m, &installer, &environment, &operation).unwrap()["already_deleted"], true);
}

#[test]
fn a_setup_in_use_or_with_uncertain_records_is_refused_and_unchanged() {
    let (f, environment, installer) = created_setup();
    let root = f.m.root.join("environments").join(&environment);
    let marker = root.join("compatdata/pfx/drive_c/marker");
    private_dir(marker.parent().unwrap()).unwrap();
    fs::write(&marker, b"keep").unwrap();
    let unchanged = |f: &test_fixture::Fixture| {
        assert_eq!(fs::read(&marker).unwrap(), b"keep");
        assert!(deleted_setup::load(&f.m, &environment).unwrap().is_none());
    };
    // Something holds the environment's operation lock (a plug-in session).
    {
        let held = OpenOptions::new().read(true).write(true).create(true).truncate(false).mode(0o600)
            .open(root.join("operation.lock")).unwrap();
        use std::os::fd::AsRawFd;
        assert_eq!(unsafe { libc::flock(held.as_raw_fd(), libc::LOCK_SH) }, 0);
        assert_eq!(delete_setup_with(&f.m, &installer, &environment, &"aa".repeat(16), &mut |_| Ok(false))
            .unwrap_err().to_string(), IN_USE);
        unchanged(&f);
    }
    // A plug-in session that never retired.
    let session = root.join("compatdata/pfx/drive_c/bridge/sessions").join("ab".repeat(16));
    private_dir(&session).unwrap();
    assert!(delete_setup_with(&f.m, &installer, &environment, &"aa".repeat(16), &mut |_| Ok(false))
        .unwrap_err().to_string().contains("never finished closing"));
    unchanged(&f);
    fs::remove_dir(&session).unwrap();
    // An installer whose cleanup is not confirmed: refuse, keep the record.
    let operation = "cd".repeat(16);
    let reserved = onboarding::reserve(&f.m, &environment, &operation).unwrap();
    let result = onboarding::directory(&f.m, &environment).unwrap().join(format!("{operation}-result.json"));
    atomic_json(&result, &json!({"schema":2,"operation":operation,"state":"cleanup_unconfirmed",
        "cleanup_confirmed":false,"owned_live":null})).unwrap();
    let records = onboarding::history_records(&f.m).unwrap();
    let retired = plan(&f.m, &installer, &environment, &records, &f.m.registry().unwrap(), &mut |_| Ok(false)).unwrap();
    assert!(retired.blockers[0].contains("can't confirm"), "{:?}", retired.blockers);
    let live = plan(&f.m, &installer, &environment, &records, &f.m.registry().unwrap(), &mut |_| Ok(true)).unwrap();
    assert!(live.blockers[0].contains("still running"));
    assert_eq!(delete_setup_with(&f.m, &installer, &environment, &"aa".repeat(16), &mut |_| Ok(false))
        .unwrap_err().to_string(), retired.blockers[0]);
    unchanged(&f);
    assert!(onboarding::retained_environment_record(&f.m, &reserved.id).unwrap().is_some());
}

#[test]
fn plug_ins_still_in_the_daw_must_be_removed_before_the_setup_is_deleted() {
    let (f, environment, installer) = created_setup();
    let records = onboarding::history_records(&f.m).unwrap();
    let mut db = f.m.registry().unwrap();
    let mut entry = db.classes.values().next().unwrap().clone();
    entry.registration.environment = records.iter().find(|r| r.id == environment).unwrap().environment.clone();
    entry.registration.metadata.class_id = "0A".repeat(16);
    entry.registration.metadata.name = "Freq Echo".into();
    entry.managed_revision = Some(RevisionRef {id:"0b".repeat(16),sha256:"0c".repeat(32)});
    db.classes.insert("0A".repeat(16), entry);
    let planned = plan(&f.m, &installer, &environment, &records, &db, &mut |_| Ok(false)).unwrap();
    assert_eq!(planned.in_daw.len(), 1);
    assert_eq!(delete_reason(&planned, None).as_deref(), Some("Remove Freq Echo from your DAW first."));
    assert!(planned.plugins.contains(&"Freq Echo".to_owned()));
}

#[test]
fn an_interrupted_deletion_finishes_from_its_retained_record() {
    let (f, environment, installer) = created_setup();
    let root = f.m.root.join("environments").join(&environment);
    private_dir(&root.join("compatdata/pfx/drive_c")).unwrap();
    fs::write(root.join("compatdata/pfx/drive_c/partly-deleted"), b"x").unwrap();
    let environments = vec![onboarding::history_records(&f.m).unwrap().into_iter()
        .find(|r| r.id == environment).unwrap().environment];
    deleted_setup::write(&f.m, &Tombstone {schema:1,environment:environment.clone(),installer:installer.clone(),
        name:"Fixture".into(),environments,registrations:vec![],plugins:vec![],operation:"ab".repeat(16),
        requested_at:1,deleted_at:None,state:Deletion::Deleting}).unwrap();
    let snapshot = snapshot_idle_test(&f.m).unwrap();
    assert_eq!(snapshot.deleted_setups[0].state, "deleting");
    let finish = snapshot.deleted_setups[0].actions[0].clone();
    assert_eq!(finish.label, "Finish deleting this setup");
    let operation = test_submit_offered(&f.m, &finish.action).unwrap();
    let finished = test_run_offered_worker(&f.m, &operation).unwrap();
    assert_eq!(finished["state"], "completed", "{finished}");
    assert!(!root.join("compatdata").exists());
    assert_eq!(deleted_setup::load(&f.m, &environment).unwrap().unwrap().state, Deletion::Deleted);
}

#[test]
fn readers_skip_a_deleted_setup_with_prepared_and_removed_history() {
    let (f, c, current) = published();
    let class = c.selection.class.id.clone();
    remove_from_daw(&f.m, &class, &identity(&current)).unwrap();
    let entry = f.m.registry().unwrap().classes[&class].clone();
    let environment = c.selection.environment.clone();
    let record = Tombstone {schema:1,environment:environment.id.clone(),installer:"ab".repeat(32),
        name:"Fixture".into(),environments:vec![environment.clone()],
        registrations:vec![RetiredRegistration {class_id:class.clone(),name:"Fixture".into(),entry}],
        plugins:vec![],operation:"cd".repeat(16),requested_at:1,deleted_at:None,state:Deletion::Deleting};
    deleted_setup::write(&f.m, &record).unwrap();
    let held = vec![hold(&f.m, &environment).unwrap()];
    finish(&f.m, record, held).unwrap();
    assert!(!f.m.registry().unwrap().classes.contains_key(&class), "retired into the setup's history");
    assert!(f.m.root.join("publications").join(&class).join("revisions").join(&current.id).exists(),
        "publication history stays");
    assert!(!environment.root.join("compatdata").exists());
    // Prepared candidates, revisions and scans of the deleted setup no longer
    // reach any reader, and no reader fails on them.
    let snapshot = snapshot_idle_test(&f.m).unwrap();
    assert!(snapshot.products.iter().all(|p| p.environment != environment.id));
    assert_eq!(snapshot.deleted_setups[0].state, "deleted");
    let overview = current::capture(&f.m).unwrap();
    assert!(overview.snapshot.products.iter().all(|p| p.environment != environment.id));
    assert_eq!(overview.snapshot.deleted_setups.len(), 1);
}

#[test]
fn adding_back_after_a_bridge_update_rechecks_the_same_plugin_in_one_action() {
    let (f, c, current) = published();
    let class = c.selection.class.id.clone();
    remove_from_daw(&f.m, &class, &identity(&current)).unwrap();
    let entry = f.m.registry().unwrap().classes[&class].clone();
    let name = entry.registration.metadata.name.clone();
    assert!(!package_authority::add_back_needs_refresh(&f.m, &class, &current).unwrap());
    // The installed bridge kit changed after the plug-in was removed.
    let mut sw = software(&f.m).unwrap();
    sw.preparation_kit.as_mut().unwrap().sha256 = "ee".repeat(32);
    atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
    assert!(package_authority::add_back_needs_refresh(&f.m, &class, &current).unwrap());
    let offers = plugin_offers(&f.m, &class, &entry, None, Some(ANY_LOADED));
    assert_eq!(offers[0].disabled_reason.as_deref(), Some(recheck_first(&name).as_str()));
    assert!(plugin_offers(&f.m, &class, &entry, None, None)[0].disabled_reason.is_none());
    let fresh = RevisionRef {id:"0d".repeat(16),sha256:"0e".repeat(32)};
    let mut seen = None;
    let added = add_back(&f.m, &class, &identity(&current), |key, removed| {
        seen = Some((key.to_owned(), removed.clone()));
        Ok(fresh.clone())
    }).unwrap();
    assert_eq!(seen, Some((class.clone(), current.clone())), "the same removed module and class");
    assert_eq!(added["rechecked_with_current_bridge"], true);
    assert_eq!(added["publication"]["id"], fresh.id);
    assert_eq!(add_back(&f.m, &class, &identity(&current), |_, _| Err("active_device_lease".into()))
        .unwrap_err().to_string(), recheck_first(&name));
}

#[test]
fn a_finished_setup_states_what_is_in_the_daw_now() {
    let plugin = |class: &str, name: &str| ui::InstallationPlugin {selection:"selection".into(),
        class_id:class.into(),name:name.into(),stage:ui::InstallationPluginStage::Available,failure:None};
    let installed = "Installed plug-ins are available through the native DAW connection.";
    let mut setup = ui::InstallerSetup {installer:"ab".repeat(32),name:"Valhalla".into(),
        label_source:"user".into(),byte_size:1,format:"pe_executable".into(),imported_at:1,
        phase:ui::SetupPhase::DiscoveryComplete,status:installed.into(),
        journey:Some(ui::InstallationJourney {daw:ui::DawChoice::Reaper,phase:ui::InstallationPhase::Available,
            plugins:vec![plugin(&"0a".repeat(16),"Freq Echo"),plugin(&"0b".repeat(16),"Space Modulator")],failure:None}),
        environment:None,compatibility:None,discovered:vec![],primary:None,secondary:vec![],
        rename:action("Rename",ui::Action::InstallerRename {installer:"ab".repeat(32),label:String::new()},None),
        history:vec![]};
    let reference = RevisionRef {id:"0c".repeat(16),sha256:"0d".repeat(32)};
    let mut plan = SetupPlan {name:"Valhalla".into(),environments:vec![],removed:vec![],plugins:vec![],blockers:vec![],
        in_daw:vec![("0A".repeat(16),"Freq Echo".into(),reference.clone()),("0B".repeat(16),"Space Modulator".into(),reference)]};
    describe_daw_state(&mut setup, &plan);
    assert_eq!(setup.status, installed, "all still in the DAW");
    plan.in_daw.truncate(1);
    describe_daw_state(&mut setup, &plan);
    assert_eq!(setup.status, "In your DAW: Freq Echo. Not in your DAW now: Space Modulator.");
    plan.in_daw.clear();
    describe_daw_state(&mut setup, &plan);
    assert_eq!(setup.status, "None of this setup's plug-ins are in your DAW now. Add them back from Plug-ins.");
}

#[test]
fn add_back_stays_offered_when_a_bridge_update_made_the_scan_historical() {
    let (f, c, current) = published();
    let class = c.selection.class.id.clone();
    remove_from_daw(&f.m, &class, &identity(&current)).unwrap();
    let entry = f.m.registry().unwrap().classes[&class].clone();
    let snapshot = snapshot_idle_test(&f.m).unwrap();
    let base = product(&snapshot, &class).clone();
    let add = offer(&base.actions, |a| matches!(a, ui::Action::PluginAddBackToDaw { .. })).unwrap();
    for (phase, kept) in [(ui::CompatibilityPhase::HistoricalOnly, true), (ui::CompatibilityPhase::CheckFailed, true),
        (ui::CompatibilityPhase::NeedsWork, false), (ui::CompatibilityPhase::TestResultIncomplete, false)] {
        let mut p = base.clone();
        let workflow = p.compatibility.as_mut().unwrap();
        workflow.phase = phase;
        workflow.primary = None;
        p.disposition = "needs_attention".into();
        present_removed(&f.m, &mut p, &entry);
        assert_eq!(offer(&p.actions, |a| matches!(a, ui::Action::PluginAddBackToDaw { .. })).is_some(), kept, "{phase:?}");
        assert_eq!(p.disposition == "removed_from_daw", kept, "{phase:?}");
        if kept { assert_eq!(p.compatibility.unwrap().primary.map(|o| o.action), Some(add.action.clone())); }
    }
}

#[test]
fn owner_refusals_read_as_plain_words() {
    assert_eq!(plain("Freq Echo", "active_device_lease".into()).to_string(), loaded("Freq Echo"));
    assert!(plain("Freq Echo", "bridge_update_required: use Update Bridge".into()).to_string()
        .contains("older version of the manager"));
    assert_eq!(plain("Freq Echo", "something_new".into()).to_string(), "something_new");
    assert_eq!(class_reason(Some(CLASS_LOADED), "Freq Echo"), Some(loaded("Freq Echo")));
    assert_eq!(class_reason(Some("Previous instance cleanup is unconfirmed"), "Freq Echo").as_deref(),
        Some("Previous instance cleanup is unconfirmed"));
}

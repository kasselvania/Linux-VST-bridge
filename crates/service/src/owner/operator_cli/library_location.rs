//! One selected setup, one retained operator request and one revision edge.
use super::*;
use linux_vst_bridge::library_locations as locations;
use std::cell::RefCell;
use std::os::unix::fs::MetadataExt;
thread_local! {static INPUT:RefCell<Option<fs::File>>=const{RefCell::new(None)};}
pub(super) fn with_input<T>(input:Option<fs::File>,run:impl FnOnce()->T)->T {
    struct Reset;impl Drop for Reset{fn drop(&mut self){INPUT.with(|slot|{slot.borrow_mut().take();});}}
    INPUT.with(|slot|{assert!(slot.borrow().is_none());*slot.borrow_mut()=input;});
    let _reset=Reset;run()
}
#[derive(Clone,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Change {
    schema:u32,operation:String,before:Environment,after:Environment,
    added:Option<locations::Location>,removed:Option<locations::Location>,
    publications:Vec<linux_vst_bridge::publication::PreparedTransition>,
}
fn path(m:&Manager,operation:&str)->Result<PathBuf>{Ok(job_dir(m,operation)?.join("library-locations.json"))}
fn exact_environment(m:&Manager,a:&ui::Action)->Result<Environment> {
    let (id,expected)=match a {
        ui::Action::AddLibraryLocation{environment,expected_environment,..}
        |ui::Action::RemoveLibraryLocation{environment,expected_environment,..}=>(environment,expected_environment),
        _=>return Err("library_action_kind".into()),
    };
    let environment=environment_install::target_record(m,id)?;
    require(locations::environment_digest(&environment)?==*expected,"library_setup_changed")?;
    Ok(environment)
}
pub(super) fn project(m:&Manager,environment:&Environment)->Result<(ui::Observation<Vec<ui::LibraryLocation>>,Vec<ui::AvailableAction>)> {
    let expected=locations::environment_digest(environment)?;
    let set=locations::load(m,environment)?;
    let mut actions=vec![];
    for read_only in [false,true] {
        actions.push(action(if read_only{"Add a read-only library folder…"}else{"Add a library folder…"},
            ui::Action::AddLibraryLocation{environment:environment.id.clone(),expected_environment:expected.clone(),read_only},None));
    }
    let mut rows=vec![];
    for location in set.locations {
        let availability=match locations::verify_link(environment,&location).and_then(|_|locations::observe(&location).map(|_|())) {
            Ok(())=>ui::Observation::Observed(()),
            Err(_)=>ui::Observation::Unavailable("This folder is unavailable or its original drive has changed. Reconnect the original drive.".into()),
        };
        actions.push(action("Remove folder access",ui::Action::RemoveLibraryLocation{environment:environment.id.clone(),
            expected_environment:expected.clone(),location:location.id.clone()},None));
        rows.push(ui::LibraryLocation{id:location.id,name:location.path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            path:location.path,drive:location.drive,read_only:location.read_only,availability});
    }
    for offer in &mut actions {
        offer.disabled_reason=match maintenance::offer_refusal(m,&offer.action) {
            Ok(reason)=>reason,Err(error)=>Some(format!("This setup is unavailable for folder changes: {error}")),
        };
    }
    Ok((ui::Observation::Observed(rows),actions))
}
pub(super) fn execute(m:&Manager,a:&ui::Action,operation:&str,timeout:Duration,waits:&mut Vec<ui::LockFacts>)->Result<Value> {
    let _serialization=acquire_readback(m,ui::OperatorLock::Canonical,Some(operation),timeout,waits)?;
    preconditions::recheck_accepted(m,operation,a)?;
    let _setup=maintenance::action_lock(m,a)?;
    let before=exact_environment(m,a)?;
    // Reject before reservation or mapping changes if any publication cannot
    // preserve its exact saved native identity through this operation.
    let entries=m.registry()?.classes.values().filter(|entry|entry.registration.environment.id==before.id)
        .cloned().collect::<Vec<_>>();
    for entry in &entries {
        require(entry.registration.environment==before && matches!(entry.publication,Publication::Published|Publication::Removed)
            && entry.managed_revision.is_some(),"library_location_publication_revision_required")?;
    }
    maintenance::acquire(m,operation,None,timeout,waits)?;
    require(maintenance::owned(m,operation)?==Some(before.clone()),"library_maintenance_owner_changed")?;
    let mut set=locations::load(m,&before)?;
    let mut added=None;let mut removed=None;let mut selected_directory=None;
    match a {
        ui::Action::AddLibraryLocation{read_only,..}=>{
            let selected=INPUT.with(|slot|slot.borrow_mut().take()).ok_or("library_directory_descriptor_missing")?;
            let (selected_path,identity)=locations::selected_directory(m,&selected)?;
            require(!set.locations.iter().any(|l|l.identity==identity || l.path.starts_with(&selected_path)
                || selected_path.starts_with(&l.path)),"library_folder_overlap_or_duplicate")?;
            let drive=locations::unused_drive(&before,&set)?;
            let anchor=job_dir(m,operation)?.join("library-directory-link");
            std::os::unix::fs::symlink(&selected_path,&anchor)?;
            fs::File::open(anchor.parent().unwrap())?.sync_all()?;
            let id=linux_vst_bridge::preparation::key(&(operation,&selected_path,&identity,&drive,read_only))?;
            let location=locations::Location{id,path:selected_path,identity,drive,read_only:*read_only,
                link_inode:fs::symlink_metadata(&anchor)?.ino()};
            // Hold the selected descriptor through preparation. Re-read its
            // persistent identity again before publishing any path grant.
            locations::selected_directory(m,&selected)?;
            set.locations.push(location.clone());added=Some(location);selected_directory=Some(selected);
        }
        ui::Action::RemoveLibraryLocation{location,..}=>{
            let index=set.locations.iter().position(|l|&l.id==location).ok_or("library_location_not_owned")?;
            let selected=set.locations.remove(index);locations::verify_link(&before,&selected)?;removed=Some(selected);
        }
        _=>return Err("library_action_kind".into()),
    }
    let mut after=before.clone();after.revision=after.revision.checked_add(1).ok_or("library_revision_bound")?;
    after.library_locations=Some(locations::retain(m,&set)?);
    let edge=locations::RevisionBinding{schema:1,operation:operation.into(),before:before.clone(),environment:after.clone(),cancelled:false};
    atomic_json(&job_dir(m,operation)?.join(locations::REVISION_FILE),&edge)?;
    let mut publications=vec![];
    for entry in &entries {
        let reference=entry.managed_revision.as_ref().ok_or("library_publication_revision_missing")?;
        let prior=m.load_revision(&entry.registration.key(),reference)?;
        let candidate=linux_vst_bridge::preparation::library_locations::prepare(m,&prior,&after,operation)?;
        publications.push(m.prepare_library_location_revision(&candidate,reference,operation)?);
    }
    let change=Change{schema:1,operation:operation.into(),before:before.clone(),after:after.clone(),added,removed,publications};
    atomic_json(&path(m,operation)?,&change)?;
    if let Some(selected)=&selected_directory {
        let (path,identity)=locations::selected_directory(m,selected)?;
        let added=change.added.as_ref().ok_or("library_selected_directory_missing")?;
        require(path==added.path && identity==added.identity,"library_selected_directory_changed")?;
    }
    apply_mapping(m,&change,true)?;
    // This exact selection is the recovery decision. The reservation prevents
    // launch while its per-class publication transactions catch up.
    atomic_json(&after.root.join("environment.json"),&after)?;
    for transition in &change.publications {m.commit_package_publication(transition)?;}
    Ok(json!({"environment":after.id,"revision":after.revision,"library_locations":after.library_locations}))
}
fn apply_mapping(m:&Manager,change:&Change,forward:bool)->Result<()> {
    let environment=&change.before;
    if let Some(location)=&change.added {
        let destination=locations::link_path(environment,&location.drive)?;
        let anchor=job_dir(m,&change.operation)?.join("library-directory-link");
        if forward {
            if fs::symlink_metadata(&destination).is_err() {
                let metadata=fs::symlink_metadata(&anchor)?;
                require(metadata.file_type().is_symlink() && metadata.ino()==location.link_inode
                    && fs::read_link(&anchor)?==location.path,"library_link_anchor_changed")?;
                // hard_link preserves the owned symlink inode and never
                // replaces an occupied name or follows the external target.
                fs::hard_link(&anchor,&destination)?;
            }
            locations::verify_link(environment,location)?;
        } else if fs::symlink_metadata(&destination).is_ok() {
            locations::verify_link(environment,location)?;fs::remove_file(&destination)?;
        }
    }
    if let Some(location)=&change.removed {
        let destination=locations::link_path(environment,&location.drive)?;
        let anchor=job_dir(m,&change.operation)?.join("removed-library-link");
        if forward {
            if fs::symlink_metadata(&destination).is_ok() {
                locations::verify_link(environment,location)?;
                if fs::symlink_metadata(&anchor).is_err(){
                    fs::hard_link(&destination,&anchor)?;
                    fs::File::open(anchor.parent().unwrap())?.sync_all()?;
                }
                require(fs::symlink_metadata(&anchor)?.ino()==location.link_inode,"library_removed_anchor_changed")?;
                fs::remove_file(&destination)?;
            } else {
                require(fs::symlink_metadata(&anchor)?.ino()==location.link_inode
                    && fs::read_link(&anchor)?==location.path,"library_removed_mapping_missing")?;
            }
        } else if fs::symlink_metadata(&destination).is_err() {
            require(fs::symlink_metadata(&anchor)?.ino()==location.link_inode
                && fs::read_link(&anchor)?==location.path,"library_removed_anchor_changed")?;
            fs::hard_link(&anchor,&destination)?;locations::verify_link(environment,location)?;
        } else {locations::verify_link(environment,location)?;}
    }
    fs::File::open(locations::dosdevices(environment))?.sync_all()?;
    Ok(())
}
fn validate_change(m:&Manager,change:&Change,edge:&locations::RevisionBinding)->Result<()> {
    require(change.schema==1 && change.operation==edge.operation && change.before==edge.before
        && change.after==edge.environment,"library_operation_binding")?;
    let before=locations::load(m,&change.before)?;let after=locations::load(m,&change.after)?;
    let added=after.locations.iter().filter(|l|!before.locations.contains(l)).cloned().collect::<Vec<_>>();
    let removed=before.locations.iter().filter(|l|!after.locations.contains(l)).cloned().collect::<Vec<_>>();
    require(added==change.added.iter().cloned().collect::<Vec<_>>()
        && removed==change.removed.iter().cloned().collect::<Vec<_>>(),"library_operation_mapping_changed")?;
    let db=m.registry()?;
    let current=db.classes.iter().filter(|(_,entry)|entry.registration.environment.id==change.before.id)
        .map(|(class,_)|class.clone()).collect::<std::collections::BTreeSet<_>>();
    let classes=change.publications.iter().map(|t|t.before.class_id.clone()).collect::<std::collections::BTreeSet<_>>();
    require(classes.len()==change.publications.len() && classes==current,"library_operation_publications_changed")?;
    for transition in &change.publications {
        require(transition.before.entry.registration.environment==change.before
            && transition.after.entry.registration.environment==change.after
            && db.classes.get(&transition.before.class_id).is_some_and(|entry|
                entry==&transition.before.entry || entry==&transition.after.entry),"library_operation_publication_scope")?;
        let before=fs::symlink_metadata(&transition.before.entry.registration.native.path)?;
        let after=fs::symlink_metadata(&transition.after.entry.registration.native.path)?;
        require(before.is_file() && after.is_file() && before.dev()==after.dev() && before.ino()==after.ino()
            && before.mode() & 0o222 == 0 && after.mode() & 0o222 == 0,"library_operation_native_identity_changed")?;
    }
    Ok(())
}
/// Called before the existing reservation releases custody, on normal finish
/// and interrupted-executor reconciliation. Never replay a directory chooser.
pub(super) fn recover(m:&Manager,operation:&str)->Result<()> {
    let edge_path=job_dir(m,operation)?.join(locations::REVISION_FILE);
    if !edge_path.try_exists()?{return Ok(())}
    let mut edge=locations::revision(m,operation)?;
    let current:Environment=read_json(&edge.before.root.join("environment.json"))?;
    if path(m,operation)?.try_exists()? {
        let change:Change=read_json(&path(m,operation)?)?;
        validate_change(m,&change,&edge)?;
        require(current==change.before || current==change.after,"library_recovery_environment_changed")?;
        let selected=current==change.after;
        apply_mapping(m,&change,selected)?;
        for transition in &change.publications {
            if selected {m.commit_package_publication(transition)?;}else{m.restore_package_publication(transition)?;}
        }
        if !selected {edge.cancelled=true;atomic_json(&edge_path,&edge)?;}
    } else {
        require(current==edge.before,"library_recovery_plan_missing")?;
        edge.cancelled=true;atomic_json(&edge_path,&edge)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn removal_fixture()->(test_fixture::Fixture,Change) {
        let f=test_fixture::Fixture::new();let mut before=f.r.environment.clone();
        let outside=f.outer.join("samples");fs::create_dir(&outside).unwrap();
        fs::write(outside.join("owned-by-musician"),b"must survive").unwrap();
        let root=locations::dosdevices(&before);private_dir(&root).unwrap();
        std::os::unix::fs::symlink(&outside,root.join("l:")).unwrap();
        let location=locations::Location{id:"ab".repeat(32),path:outside,drive:"L:".into(),read_only:true,
            identity:locations::DirectoryIdentity{volume:"fixture-original".into(),handle_type:1,handle:vec![1]},
            link_inode:fs::symlink_metadata(root.join("l:")).unwrap().ino()};
        before.library_locations=Some(locations::retain(&f.m,&locations::LocationSet{schema:1,locations:vec![location.clone()]}).unwrap());
        atomic_json(&before.root.join("environment.json"),&before).unwrap();
        let mut after=before.clone();after.revision+=1;after.library_locations=Some(locations::retain(&f.m,&locations::LocationSet::default()).unwrap());
        let operation="cd".repeat(16);private_dir(&job_dir(&f.m,&operation).unwrap()).unwrap();
        atomic_json(&job_dir(&f.m,&operation).unwrap().join("request.json"),&ui::Request{schema:ui::OPERATOR_SCHEMA,
            state_token:"fixture".into(),action:ui::Action::RemoveLibraryLocation{environment:before.id.clone(),
                expected_environment:locations::environment_digest(&before).unwrap(),location:location.id.clone()}}).unwrap();
        atomic_json(&job_dir(&f.m,&operation).unwrap().join(locations::REVISION_FILE),&locations::RevisionBinding{
            schema:1,operation:operation.clone(),before:before.clone(),environment:after.clone(),cancelled:false}).unwrap();
        let change=Change{schema:1,operation,before,after,added:None,removed:Some(location),publications:vec![]};
        atomic_json(&path(&f.m,&change.operation).unwrap(),&change).unwrap();
        (f,change)
    }
    fn addition_fixture()->(test_fixture::Fixture,Change) {
        let f=test_fixture::Fixture::new();let before=f.r.environment.clone();
        private_dir(&locations::dosdevices(&before)).unwrap();
        let outside=f.outer.join("external-samples");fs::create_dir(&outside).unwrap();
        fs::write(outside.join("user-content"),b"unchanged").unwrap();
        let operation="cd".repeat(16);let directory=job_dir(&f.m,&operation).unwrap();private_dir(&directory).unwrap();
        let anchor=directory.join("library-directory-link");std::os::unix::fs::symlink(&outside,&anchor).unwrap();
        let identity=locations::DirectoryIdentity{volume:"fixture-original".into(),handle_type:1,handle:vec![1]};
        let id=linux_vst_bridge::preparation::key(&(&operation,&outside,&identity,"L:",false)).unwrap();
        let location=locations::Location{id,path:outside,drive:"L:".into(),read_only:false,identity,
            link_inode:fs::symlink_metadata(anchor).unwrap().ino()};
        let mut after=before.clone();after.revision+=1;
        after.library_locations=Some(locations::retain(&f.m,&locations::LocationSet{schema:1,locations:vec![location.clone()]}).unwrap());
        atomic_json(&directory.join("request.json"),&ui::Request{schema:ui::OPERATOR_SCHEMA,
            state_token:"fixture".into(),action:ui::Action::AddLibraryLocation{environment:before.id.clone(),
                expected_environment:locations::environment_digest(&before).unwrap(),read_only:false}}).unwrap();
        atomic_json(&directory.join(locations::REVISION_FILE),&locations::RevisionBinding{
            schema:1,operation:operation.clone(),before:before.clone(),environment:after.clone(),cancelled:false}).unwrap();
        let change=Change{schema:1,operation,before,after,added:Some(location),removed:None,publications:vec![]};
        atomic_json(&path(&f.m,&change.operation).unwrap(),&change).unwrap();
        (f,change)
    }
    #[test]
    fn library_location_add_is_scoped_refuses_collisions_and_recovers_exact_selection() {
        for selected in [false,true] {
            let (f,change)=addition_fixture();let added=change.added.as_ref().unwrap();
            let other=f.m.root.join("environments").join("ef".repeat(16)).join("compatdata/pfx/dosdevices");private_dir(&other).unwrap();
            std::os::unix::fs::symlink("/foreign/folder",other.join("l:")).unwrap();
            let destination=locations::link_path(&change.before,"L:").unwrap();
            std::os::unix::fs::symlink(&added.path,&destination).unwrap();
            assert!(apply_mapping(&f.m,&change,true).is_err(),"even an equal-target foreign link is not ours");
            assert_eq!(fs::read_link(&destination).unwrap(),added.path);
            fs::remove_file(&destination).unwrap(); // Only the deliberate foreign fixture.
            apply_mapping(&f.m,&change,true).unwrap();
            if selected {atomic_json(&change.after.root.join("environment.json"),&change.after).unwrap();}
            recover(&f.m,&change.operation).unwrap();
            if selected {locations::verify_link(&change.after,added).unwrap();}
            else {assert!(fs::symlink_metadata(&destination).is_err());}
            assert_eq!(fs::read_link(other.join("l:")).unwrap(),Path::new("/foreign/folder"));
            assert_eq!(fs::read(added.path.join("user-content")).unwrap(),b"unchanged");
            assert_eq!(read_json::<Environment>(&change.before.root.join("environment.json")).unwrap(),
                if selected {change.after}else{change.before});
        }
    }
    #[test]
    fn library_location_offer_binds_exact_current_setup_before_directory_use() {
        let f=test_fixture::Fixture::new();f.m.register(f.r.clone()).unwrap();
        let mut action=ui::Action::AddLibraryLocation{environment:f.r.environment.id.clone(),
            expected_environment:locations::environment_digest(&f.r.environment).unwrap(),read_only:false};
        assert_eq!(exact_environment(&f.m,&action).unwrap(),f.r.environment);
        if let ui::Action::AddLibraryLocation{expected_environment,..}=&mut action {
            *expected_environment="ff".repeat(32);
        }
        assert!(exact_environment(&f.m,&action).unwrap_err().to_string().contains("library_setup_changed"));
        if let ui::Action::AddLibraryLocation{environment,expected_environment,..}=&mut action {
            *environment="ef".repeat(16);*expected_environment=locations::environment_digest(&f.r.environment).unwrap();
        }
        assert!(exact_environment(&f.m,&action).is_err());
        assert_eq!(read_json::<Environment>(&f.r.environment.root.join("environment.json")).unwrap(),f.r.environment);
    }
    #[test]
    fn removal_is_setup_scoped_and_never_touches_external_content() {
        let (f,change)=removal_fixture();let selected=change.removed.as_ref().unwrap();
        let other=f.m.root.join("environments").join("ef".repeat(16)).join("compatdata/pfx/dosdevices");private_dir(&other).unwrap();
        std::os::unix::fs::symlink(&selected.path,other.join("l:")).unwrap();
        apply_mapping(&f.m,&change,true).unwrap();
        assert!(fs::symlink_metadata(locations::link_path(&change.before,"L:").unwrap()).is_err());
        assert_eq!(fs::read(selected.path.join("owned-by-musician")).unwrap(),b"must survive");
        assert_eq!(fs::read_link(other.join("l:")).unwrap(),selected.path);
        apply_mapping(&f.m,&change,false).unwrap();
        locations::verify_link(&change.before,selected).unwrap();
    }
    #[test]
    fn missing_drive_removal_and_interruption_preserve_exact_prior_or_selected_state() {
        for selected in [false,true] {
            let (f,change)=removal_fixture();let location=change.removed.as_ref().unwrap();
            let disconnected=f.outer.join("disconnected");fs::rename(&location.path,&disconnected).unwrap();
            apply_mapping(&f.m,&change,true).unwrap();
            if selected {atomic_json(&change.after.root.join("environment.json"),&change.after).unwrap();}
            recover(&f.m,&change.operation).unwrap();
            let current:Environment=read_json(&change.before.root.join("environment.json")).unwrap();
            if selected {
                assert_eq!(current,change.after);
                assert!(fs::symlink_metadata(locations::link_path(&change.before,"L:").unwrap()).is_err());
                assert!(locations::load(&f.m,&current).unwrap().locations.is_empty());
                assert!(current.library_locations.is_some());
            } else {
                assert_eq!(current,change.before);locations::verify_link(&current,location).unwrap();
                assert!(locations::revision(&f.m,&change.operation).unwrap().cancelled);
            }
            assert_eq!(fs::read(disconnected.join("owned-by-musician")).unwrap(),b"must survive");
        }
    }
    #[test]
    fn foreign_link_and_wrong_removal_identity_refuse_without_deleting_contents() {
        let (f,change)=removal_fixture();let location=change.removed.as_ref().unwrap();
        let link=locations::link_path(&change.before,"L:").unwrap();
        fs::rename(&link,link.with_file_name("old-link")).unwrap();
        std::os::unix::fs::symlink(&location.path,&link).unwrap();
        assert!(apply_mapping(&f.m,&change,true).is_err());
        assert!(fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        let mut wrong=change.clone();wrong.removed.as_mut().unwrap().id="ff".repeat(32);
        atomic_json(&path(&f.m,&change.operation).unwrap(),&wrong).unwrap();
        assert!(recover(&f.m,&change.operation).is_err());
        assert_eq!(fs::read(location.path.join("owned-by-musician")).unwrap(),b"must survive");
    }
}

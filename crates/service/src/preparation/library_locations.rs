//! A location-only revision preserves the exact inspected module and engine.
//! It does not manufacture a new inspection or a support result.
use super::*;

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct LocationRevision {
    pub operation:String,
    pub predecessor:String,
}
fn successor(prior:&Candidate,after:&Environment,origin:LocationRevision)->Result<Candidate> {
    let mut expected=prior.selection.environment.clone();
    expected.revision=expected.revision.checked_add(1).ok_or("library_revision_bound")?;
    expected.library_locations=after.library_locations.clone();
    require(expected==*after && after.library_locations.is_some()
        && after.library_locations!=prior.selection.environment.library_locations,
        "library_candidate_environment_changed")?;
    let mut next=prior.clone();
    next.selection.environment=after.clone();
    next.inspection.selection=next.selection.clone();
    next.inspection.origin=Origin::LibraryLocationRevision;
    next.origin=Origin::LibraryLocationRevision;
    next.profile.id=format!("managed.{}",key(&(prior.id()?,after,&origin))?);
    next.profile.revision=1;
    next.profile.requirements.environment_revision=after.revision;
    next.location_revision=Some(origin);
    // An earlier runtime migration remains in the immutable predecessor.
    next.touch_carry_forward=None;
    next.settings_trial=None;
    Ok(next)
}
pub fn prepare(m:&Manager,prior:&Revision,after:&Environment,operation:&str)->Result<Candidate> {
    let source=for_profile(m,&prior.profile)?.ok_or("library_prepared_publication_required")?;
    validate_candidate_record(m,&source)?;
    require(source.selection.environment==prior.registration.environment
        && source.selection.module==prior.registration.module
        && source.host==prior.registration.host
        && source.source_manifest.sha256==prior.registration.host_source_sha256
        && source.native.artifact.sha256==prior.registration.native.sha256,
        "library_publication_predecessor_changed")?;
    prior.registration.verify(&m.root)?;
    let next=successor(&source,after,LocationRevision{operation:operation.into(),predecessor:source.id()?})?;
    record_candidate_with_predecessor(m,&next,Some(&source.id()?))?;
    Ok(next)
}
pub(super) fn validate_binding(m:&Manager,c:&Candidate)->Result<Candidate> {
    let edge=c.location_revision.as_ref().ok_or("library_candidate_edge_missing")?;
    let prior=candidate_record(m,&edge.predecessor)?;
    let binding=crate::library_locations::revision(m,&edge.operation)?;
    require(!binding.cancelled && binding.before==prior.selection.environment && binding.environment==c.selection.environment
        && *c==successor(&prior,&binding.environment,edge.clone())?,"library_candidate_edge_changed")?;
    // Exact immutable ancestry, bounded by monotonically decreasing setup
    // revisions; do not reinterpret the original report as a new observation.
    require(prior.selection.environment.revision<c.selection.environment.revision,
        "library_candidate_cycle")?;
    Ok(prior)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn edge(f:&test_fixture::Fixture,operation:&str,prior:&Environment)->Environment {
        use crate::library_locations as locations;
        let mut after=prior.clone();after.revision+=1;
        let target=f.outer.join(format!("external-{operation}"));fs::create_dir(&target).unwrap();
        let directory=f.m.root.join("operator").join(operation);private_dir(&directory).unwrap();
        let anchor=directory.join("library-directory-link");symlink(&target,&anchor).unwrap();
        let identity=locations::DirectoryIdentity{volume:"fixture-volume".into(),handle_type:1,handle:vec![1,2]};
        let id=key(&(operation,&target,&identity,"L:",false)).unwrap();
        let set=locations::LocationSet{schema:1,locations:vec![locations::Location{id,path:target,identity,
            drive:"L:".into(),read_only:false,link_inode:fs::symlink_metadata(anchor).unwrap().ino()}]};
        after.library_locations=Some(locations::retain(&f.m,&set).unwrap());
        atomic_json(&directory.join("request.json"),&operator_model::Request{schema:operator_model::OPERATOR_SCHEMA,
            state_token:"fixture".into(),action:operator_model::Action::AddLibraryLocation{
                environment:prior.id.clone(),expected_environment:locations::environment_digest(prior).unwrap(),read_only:false}}).unwrap();
        atomic_json(&directory.join(locations::REVISION_FILE),&locations::RevisionBinding{schema:1,
            operation:operation.into(),before:prior.clone(),environment:after.clone(),cancelled:false}).unwrap();
        after
    }
    #[test]
    fn location_revision_preserves_native_inode_descriptor_and_performance() {
        let (f,c)=test_fixture::projection_candidate_in(test_fixture::Fixture::new(),false);
        record_candidate(&f.m,&c).unwrap();let reference=enable(&f.m,&c,false).unwrap();
        let prior=f.m.load_revision(&c.selection.class.id,&reference).unwrap();
        let original=fs::metadata(&prior.registration.native.path).unwrap();
        let after=edge(&f,&"ab".repeat(16),&prior.registration.environment);
        let candidate=prepare(&f.m,&prior,&after,&"ab".repeat(16)).unwrap();
        let transition=f.m.prepare_library_location_revision(&candidate,&reference,&"ab".repeat(16)).unwrap();
        let staged=fs::metadata(&transition.after.entry.registration.native.path).unwrap();
        assert_eq!((original.dev(),original.ino()),(staged.dev(),staged.ino()));
        assert_eq!(transition.before.performance,transition.after.performance);
        assert_eq!(prior.registration.descriptor.as_ref().map(|a|&a.sha256),transition.after.entry.registration.descriptor.as_ref().map(|a|&a.sha256));
        // Staging has not selected the setup or moved its DAW pointer.
        assert_eq!(f.m.registry().unwrap().classes[&prior.class_id].registration,prior.registration);
        assert_eq!(read_json::<Environment>(&after.root.join("environment.json")).unwrap(),prior.registration.environment);
        atomic_json(&after.root.join("environment.json"),&after).unwrap();
        f.m.commit_package_publication(&transition).unwrap();
        let selected=&f.m.registry().unwrap().classes[&prior.class_id].registration;
        selected.verify(&f.m.root).unwrap();
        let mapped=fs::metadata(&selected.native.path).unwrap();
        assert_eq!((original.dev(),original.ino()),(mapped.dev(),mapped.ino()));
        let different=f.outer.join("equal-content-different-inode");fs::copy(&selected.native.path,&different).unwrap();
        assert_eq!(digest(&different).unwrap(),selected.native.sha256);
        assert_ne!(fs::metadata(different).unwrap().ino(),mapped.ino());
        // Existing transaction recovery retains exactly whichever pointer won.
        f.m.commit_package_publication(&transition).unwrap();
        f.m.restore_package_publication(&transition).unwrap();
        atomic_json(&after.root.join("environment.json"),&prior.registration.environment).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&prior.class_id].registration,prior.registration);
    }
    #[test]
    fn changed_configuration_and_aborted_retry_do_not_reuse_profile_identity() {
        let (f,c)=test_fixture::projection_candidate_in(test_fixture::Fixture::new(),false);
        record_candidate(&f.m,&c).unwrap();let reference=enable(&f.m,&c,false).unwrap();
        let prior=f.m.load_revision(&c.selection.class.id,&reference).unwrap();
        let after=edge(&f,&"ab".repeat(16),&prior.registration.environment);
        let first=prepare(&f.m,&prior,&after,&"ab".repeat(16)).unwrap();
        let other=edge(&f,&"cd".repeat(16),&prior.registration.environment);
        let second=prepare(&f.m,&prior,&other,&"cd".repeat(16)).unwrap();
        assert_ne!(first.profile.id,second.profile.id);
        let mut changed=first;changed.profile.class.name="different descriptor".into();
        assert!(f.m.prepare_library_location_revision(&changed,&reference,&"ab".repeat(16)).is_err());
        assert_eq!(f.m.registry().unwrap().classes[&prior.class_id].registration,prior.registration);
    }
}

#[cfg(test)]
mod removed_publication_tests {
    use super::*;
    #[test]
    fn location_change_does_not_reconnect_a_removed_plugin() {
        let (f,c)=test_fixture::projection_candidate_in(test_fixture::Fixture::new(),false);
        record_candidate(&f.m,&c).unwrap();let reference=enable(&f.m,&c,false).unwrap();
        let prior=f.m.load_revision(&c.selection.class.id,&reference).unwrap();
        f.m.unpublish(&prior.class_id).unwrap();
        let after=super::tests::edge(&f,&"ab".repeat(16),&prior.registration.environment);
        let candidate=prepare(&f.m,&prior,&after,&"ab".repeat(16)).unwrap();
        let transition=f.m.prepare_library_location_revision(&candidate,&reference,&"ab".repeat(16)).unwrap();
        assert_eq!(transition.after.entry.publication,Publication::Removed);
        atomic_json(&after.root.join("environment.json"),&after).unwrap();
        f.m.commit_package_publication(&transition).unwrap();
        let db=f.m.registry().unwrap();let current=&db.classes[&prior.class_id];
        assert_eq!(current.publication,Publication::Removed);
        assert_eq!(current.registration.environment,after);
        assert!(physical(&f.m.link(&prior.class_id)).unwrap().is_none());
        f.m.commit_package_publication(&transition).unwrap();
        f.m.restore_package_publication(&transition).unwrap();
        atomic_json(&after.root.join("environment.json"),&prior.registration.environment).unwrap();
        let db=f.m.registry().unwrap();
        assert_eq!(db.classes[&prior.class_id].publication,Publication::Removed);
        assert_eq!(db.classes[&prior.class_id].registration,prior.registration);
        assert!(physical(&f.m.link(&prior.class_id)).unwrap().is_none());
    }
}

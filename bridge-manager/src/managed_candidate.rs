//! One sealed new-class candidate from accepted managed-installer discovery.
//! No caller-supplied class, environment or compatibility policy is authority.
use crate::{
    catalogue::EnvironmentBinding,
    observation::{Census, ModuleStamp},
    profiles::*,
    publication::*,
    qualification::InstalledCandidate,
    *,
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    schema: u32,
    environment: String,
    onboarding_sha256: String,
    environment_sha256: String,
    inventory_sha256: String,
    module_relative: String,
    controller: String,
    inspection_sha256: String,
}
fn identity() -> Result<Binding> {
    #[cfg(test)]
    if let Some((_, b)) = TEST_CONTRACT.with(|c| c.borrow().clone()) {
        return Ok(b);
    }
    let b: Binding =
        serde_json::from_slice(include_bytes!("../../compatibility/sv1/binding.json"))?;
    require(
        b.schema == 1 && valid_hex(&b.environment, 32) && valid_hex(&b.controller, 32),
        "candidate_binding_identity",
    )?;
    Ok(b)
}
pub fn candidate() -> Result<Profile> {
    #[cfg(test)]
    if let Some((p, _)) = TEST_CONTRACT.with(|c| c.borrow().clone()) {
        return Ok(p);
    }
    let p = Profile::parse(include_bytes!("../../compatibility/sv1/xfer-serum2.json"))?;
    require(
        p.claim == Claim::ReviewCandidate
            && p.revision == 1
            && p.requirements.environment_family == Family::ManagedInstallerV1
            && p.role == Role::Instrument
            && p.capabilities.compatibility() == Compatibility::default(),
        "candidate_contract",
    )?;
    Ok(p)
}
fn exact_record(path: PathBuf, sha: String) -> Result<()> {
    require(
        path.canonicalize()? == path && file(&path)?.metadata()?.len() <= 2 * 1024 * 1024,
        "candidate_record_location",
    )?;
    Artifact { path, sha256: sha }.verify()
}
fn facts(m: &Manager, p: &Profile) -> Result<(Environment, Artifact)> {
    let b = identity()?;
    let root = m.root.join("environments").join(&b.environment);
    exact_record(root.join("environment.json"), b.environment_sha256)?;
    exact_record(
        m.root
            .join("onboarding")
            .join(&b.environment)
            .join("record.json"),
        b.onboarding_sha256,
    )?;
    exact_record(
        m.root
            .join("inventory")
            .join(format!("{}.json", b.environment)),
        b.inventory_sha256,
    )?;
    let env: Environment = read_json(&root.join("environment.json"))?;
    require(
        env.id == b.environment && env.root == root && root.canonicalize()? == root,
        "candidate_environment_identity",
    )?;
    p.verify_environment(&env, &Family::ManagedInstallerV1)?;
    env.runner.verify()?;
    let relative = Path::new(&b.module_relative);
    require(
        relative
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_))),
        "candidate_module_relative",
    )?;
    let module = Artifact {
        path: root.join("compatdata/pfx/drive_c").join(relative),
        sha256: p.module_sha256.clone(),
    };
    require(
        module.path.canonicalize()? == module.path,
        "candidate_module_alias",
    )?;
    module.verify()?;
    Ok((env, module))
}
fn directory(m: &Manager, p: &Profile) -> Result<PathBuf> {
    Ok(m.root
        .join("software/sv1-qualification")
        .join(p.fingerprint()?))
}
fn sibling_registry(m: &Manager, p: &Profile) -> Result<Registry> {
    let mut db = m.registry()?;
    db.classes.remove(&p.class.class_id);
    db.revision = 0;
    Ok(db)
}
fn baseline(m: &Manager, p: &Profile) -> Result<()> {
    let path = directory(m, p)?.join("baseline.json");
    require(
        path.canonicalize()? == path && file(&path)?.metadata()?.mode() & 0o222 == 0,
        "candidate_baseline_mutable",
    )?;
    let expected: Registry = read_json(&path)?;
    require(
        serde_json::to_vec(&expected)? == serde_json::to_vec(&sibling_registry(m, p)?)?,
        "candidate_siblings_changed",
    )
}
pub fn stage(m: &Manager, package: &Path) -> Result<()> {
    let p = candidate()?;
    let b = identity()?;
    facts(m, &p)?;
    let roster = [
        (
            "host.exe".to_string(),
            p.requirements.host_sha256.clone(),
            "host.exe",
        ),
        (
            "host-source-manifest.json".into(),
            p.requirements.host_source_sha256.clone(),
            "host-source-manifest.json",
        ),
        (
            format!("{}.so", p.class.class_id),
            p.requirements.native_sha256.clone(),
            "native.so",
        ),
        (
            "inspection.json".into(),
            b.inspection_sha256,
            "inspection.json",
        ),
    ];
    for (name, hash, _) in &roster {
        Artifact {
            path: package.join(name),
            sha256: hash.clone(),
        }
        .verify()?;
    }
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(
        !m.publication_pending(&p.class.class_id)?
            && physical(&m.link(&p.class.class_id))?.is_none(),
        "candidate_already_exposed",
    )?;
    let dest = directory(m, &p)?;
    if dest.exists() {
        baseline(m, &p)?;
        crate::qualification::load_for(m, p, Qualification::Sv1Instrument)?;
        return Ok(());
    }
    require(
        !m.registry()?.classes.contains_key(&p.class.class_id),
        "candidate_existing_class",
    )?;
    private_dir(dest.parent().unwrap())?;
    let temp = dest.with_file_name(format!(".stage-{}", random_id()?));
    private_dir(&temp)?;
    let result = (|| -> Result<()> {
        for (name, hash, target) in roster {
            let path = temp.join(target);
            fs::copy(package.join(name), &path)?;
            Artifact {
                path: path.clone(),
                sha256: hash,
            }
            .verify()?;
            fs::set_permissions(
                &path,
                fs::Permissions::from_mode(if target == "native.so" { 0o500 } else { 0o400 }),
            )?;
            File::open(path)?.sync_all()?;
        }
        atomic_json(&temp.join("baseline.json"), &sibling_registry(m, &p)?)?;
        fs::set_permissions(
            temp.join("baseline.json"),
            fs::Permissions::from_mode(0o400),
        )?;
        File::open(&temp)?.sync_all()?;
        rename_link(&temp, &dest, false)?;
        File::open(dest.parent().unwrap())?.sync_all()?;
        Ok(())
    })();
    if result.is_err() && temp.exists() {
        fs::remove_dir_all(temp)?;
    }
    result
}
pub fn binding(m: &Manager) -> Result<Registration> {
    let p = candidate()?;
    baseline(m, &p)?;
    let (environment, module) = facts(m, &p)?;
    let c = crate::qualification::load_for(m, p.clone(), Qualification::Sv1Instrument)?;
    let r = Registration {
        metadata: p.class,
        module,
        environment,
        host: c.host,
        host_source_sha256: c.source_manifest.sha256,
        native: c.native.artifact,
        compatibility: p.capabilities.compatibility(),
    };
    r.verify(&m.root)?;
    Ok(r)
}
/// Candidate sessions use their sealed installation binding, not the MF2
/// initial-install owner (which correctly refuses registered environments).
pub fn check_session(
    m: &Manager,
    class: &str,
    environment: &Environment,
    module: &Artifact,
    host: &Artifact,
    source: &str,
) -> Result<()> {
    let expected = binding(m)?;
    require(
        class == expected.metadata.class_id
            && *environment == expected.environment
            && *module == expected.module
            && host.sha256 == expected.host.sha256
            && source == expected.host_source_sha256,
        "candidate_session_binding",
    )
}
pub(crate) fn check_publication(m: &Manager, p: &Profile, r: &Registration) -> Result<()> {
    require(
        *p == candidate()? && *r == binding(m)?,
        "qualification_exact_candidate_required",
    )?;
    m.require_inactive(None)?;
    require(
        m.performance(&p.class.class_id)?.added_frames == 512,
        "candidate_frame_posture",
    )
}
pub fn publish(m: &Manager) -> Result<RevisionRef> {
    publish_with_boundary(m, None)
}
fn publish_with_boundary(m: &Manager, fail: Option<Boundary>) -> Result<RevisionRef> {
    let p = candidate()?;
    let b = identity()?;
    let r = binding(m)?;
    let report = Artifact {
        path: directory(m, &p)?.join("inspection.json"),
        sha256: b.inspection_sha256,
    };
    let census = Census::from_report(
        EnvironmentBinding {
            family: Family::ManagedInstallerV1,
            environment: r.environment.clone(),
        },
        r.module.clone(),
        ModuleStamp::read(&r.module.path)?,
        r.host.clone(),
        r.host_source_sha256.clone(),
        report,
        &p.class.class_id,
    )?;
    require(
        census.classes.contains(&b.controller),
        "candidate_controller_absent",
    )?;
    m.qualify_for(&census, fail, Qualification::Sv1Instrument)
}
pub(crate) fn retained(m: &Manager, r: &Revision, c: &InstalledCandidate) -> Result<()> {
    let mut expected = binding(m)?;
    expected.native.path = r.registration.native.path.clone();
    require(
        r.profile == c.profile
            && r.registration == expected
            && r.parent.is_none()
            && r.qualification == Some(Qualification::Sv1Instrument),
        "qualification_exact_candidate_required",
    )?;
    let db = m.registry()?;
    let e = db
        .classes
        .get(&r.class_id)
        .ok_or("candidate_not_published")?;
    let reference = e
        .managed_revision
        .as_ref()
        .ok_or("candidate_not_published")?;
    require(
        reference.id == r.id
            && e.registration == r.registration
            && e.publication == Publication::Published
            && physical(&m.link(&r.class_id))? == Some(r.target.clone())
            && !m.publication_pending(&r.class_id)?,
        "candidate_not_published",
    )?;
    m.verify_completed_publication(r, reference)
}
pub(crate) fn served(m: &Manager, r: &Registration, host: &Artifact, source: &str) -> Result<()> {
    let p = candidate()?;
    require(
        host.sha256 == p.requirements.host_sha256 && source == p.requirements.host_source_sha256,
        "installed_host_mismatch",
    )?;
    let db = m.registry()?;
    let e = db.classes.get(&r.key()).ok_or("candidate_not_published")?;
    let rev = m.load_revision(
        &r.key(),
        e.managed_revision
            .as_ref()
            .ok_or("candidate_not_published")?,
    )?;
    require(rev.registration == *r, "installed_host_mismatch")?;
    m.verify_retained_authority(&rev, &[p])
}
pub fn restore(m: &Manager) -> Result<()> {
    let p = candidate()?;
    if let Some(e) = m.registry()?.classes.get(&p.class.class_id) {
        let r = m.load_revision(
            &p.class.class_id,
            e.managed_revision.as_ref().ok_or("candidate_identity")?,
        )?;
        require(
            r.profile == p && r.qualification == Some(Qualification::Sv1Instrument),
            "candidate_identity",
        )?;
        if e.publication == Publication::Removed {
            require(
                physical(&m.link(&p.class.class_id))?.is_none(),
                "foreign_publication",
            )?;
        } else {
            m.unpublish(&p.class.class_id)?;
        }
    }
    Ok(())
}

#[cfg(test)]
thread_local! { static TEST_CONTRACT: std::cell::RefCell<Option<(Profile,Binding)>> = const { std::cell::RefCell::new(None) }; }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::{inspection_report, prepared, Fixture};
    struct Contract;
    impl Drop for Contract {
        fn drop(&mut self) {
            TEST_CONTRACT.with(|c| *c.borrow_mut() = None);
        }
    }
    fn fixture() -> (Fixture, PathBuf, Contract) {
        let (f, mut p, mut census, _) = prepared();
        f.m.unpublish(&p.class.class_id).unwrap();
        atomic_json(&f.m.root.join("registry.json"), &Registry::default()).unwrap();
        p.requirements.environment_family = Family::ManagedInstallerV1;
        p.claim = Claim::ReviewCandidate;
        p.revision = 1;
        p.capabilities.accessibility = Accessibility::WindowsDefault;
        p.capabilities.editor = Editor::DetachedDirectVendorLifecycle;
        census.environment.family = Family::ManagedInstallerV1;
        let env = &f.r.environment;
        let op = f.m.root.join("onboarding").join(&env.id);
        private_dir(&op).unwrap();
        atomic_json(
            &op.join("record.json"),
            &serde_json::json!({"fixture":"onboarding"}),
        )
        .unwrap();
        let inv = f.m.root.join("inventory");
        private_dir(&inv).unwrap();
        atomic_json(
            &inv.join(format!("{}.json", env.id)),
            &serde_json::json!({"fixture":"inventory"}),
        )
        .unwrap();
        let package = f.outer.join("package");
        private_dir(&package).unwrap();
        fs::copy(&f.r.host.path, package.join("host.exe")).unwrap();
        fs::write(package.join("host-source-manifest.json"), b"manifest").unwrap();
        p.requirements.host_source_sha256 =
            digest(&package.join("host-source-manifest.json")).unwrap();
        census.host_source_sha256 = p.requirements.host_source_sha256.clone();
        fs::copy(
            &f.r.native.path,
            package.join(format!("{}.so", p.class.class_id)),
        )
        .unwrap();
        atomic_json(
            &package.join("inspection.json"),
            &inspection_report(&census),
        )
        .unwrap();
        let b = Binding {
            schema: 1,
            environment: env.id.clone(),
            onboarding_sha256: digest(&op.join("record.json")).unwrap(),
            environment_sha256: digest(&env.root.join("environment.json")).unwrap(),
            inventory_sha256: digest(&inv.join(format!("{}.json", env.id))).unwrap(),
            module_relative: "Program Files/Common Files/VST3/a.vst3".into(),
            controller: p.class.class_id.clone(),
            inspection_sha256: digest(&package.join("inspection.json")).unwrap(),
        };
        TEST_CONTRACT.with(|c| *c.borrow_mut() = Some((p, b)));
        (f, package, Contract)
    }
    #[test]
    fn stage_is_inactive_exact_and_drift_refused() {
        let (f, package, _contract) = fixture();
        stage(&f.m, &package).unwrap();
        stage(&f.m, &package).unwrap();
        assert!(f.m.registry().unwrap().classes.is_empty());
        assert!(!f.m.link(&candidate().unwrap().class.class_id).exists());
        let r = binding(&f.m).unwrap();
        assert_eq!(r.compatibility, Compatibility::default());
        fs::write(&r.module.path, b"changed").unwrap();
        assert!(binding(&f.m).is_err());
    }
    #[test]
    fn new_class_publication_retains_candidate_and_restores_to_absence() {
        let (f, package, _contract) = fixture();
        stage(&f.m, &package).unwrap();
        let p = candidate().unwrap();
        assert!(p.claim.require(SelectionPurpose::Activation).is_err());
        let reference = publish(&f.m).unwrap();
        let r = binding(&f.m).unwrap();
        check_session(
            &f.m,
            &r.metadata.class_id,
            &r.environment,
            &r.module,
            &r.host,
            &r.host_source_sha256,
        )
        .unwrap();
        assert!(check_session(
            &f.m,
            &"ff".repeat(16),
            &r.environment,
            &r.module,
            &r.host,
            &r.host_source_sha256
        )
        .is_err());
        let revision = f.m.load_revision(&p.class.class_id, &reference).unwrap();
        assert!(revision.parent.is_none());
        assert_eq!(revision.qualification, Some(Qualification::Sv1Instrument));
        f.m.verify_retained_authority(&revision, std::slice::from_ref(&p))
            .unwrap();
        restore(&f.m).unwrap();
        restore(&f.m).unwrap();
        assert_eq!(
            f.m.registry().unwrap().classes[&p.class.class_id].publication,
            Publication::Removed
        );
        assert!(!f.m.link(&p.class.class_id).exists());
        assert!(f.m.load_revision(&p.class.class_id, &reference).is_ok());
    }
    #[test]
    fn wrong_package_and_environment_records_never_grant_authority() {
        let (f, package, _contract) = fixture();
        fs::write(package.join("host.exe"), b"other host").unwrap();
        assert!(stage(&f.m, &package).is_err());
        assert!(f.m.registry().unwrap().classes.is_empty());
        fs::copy(&f.r.host.path, package.join("host.exe")).unwrap();
        fs::write(f.r.environment.root.join("environment.json"), b"{}").unwrap();
        assert!(stage(&f.m, &package).is_err());
    }
    #[test]
    fn interrupted_new_class_publication_always_reconciles_to_absence() {
        for point in BOUNDARIES {
            let (f, package, _contract) = fixture();
            stage(&f.m, &package).unwrap();
            assert!(publish_with_boundary(&f.m, Some(point)).is_err());
            f.m.reconcile().unwrap();
            assert!(
                !f.m.link(&candidate().unwrap().class.class_id).exists(),
                "{point:?}"
            );
            assert!(!f
                .m
                .publication_pending(&candidate().unwrap().class.class_id)
                .unwrap());
        }
    }
    #[test]
    fn compiled_instrument_is_not_fx_or_ordinary_authority() {
        let p = candidate().unwrap();
        assert_eq!(p.class.class_id, "56534558667350736572756D20320000");
        assert_eq!(p.claim, Claim::ReviewCandidate);
        assert!(!p.claim.permits(SelectionPurpose::Activation));
        assert_eq!(p.capabilities.compatibility(), Compatibility::default());
        assert!(!installed_profiles()
            .unwrap()
            .iter()
            .any(|o| o.class.class_id == p.class.class_id));
    }
}

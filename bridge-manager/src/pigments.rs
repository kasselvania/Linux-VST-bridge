//! Sealed AP18 new-class qualification. Existing Arturia publications are the
//! exact baseline, never fictional parents of the new Pigments VST3 class.
use crate::{profiles::*, publication::*, qualification::InstalledCandidate, *};

pub fn candidate() -> Result<Profile> {
    Profile::parse(include_bytes!(
        "../../compatibility/ap18/arturia-pigments.json"
    ))
}
/// Failed candidate history can precede a replacement, never grant runtime authority.
pub(crate) fn replacement_prior(prior: &Profile, next: &Profile) -> Result<bool> {
    if prior == next { return Ok(true); }
    let first = Profile::parse(include_bytes!("../../compatibility/ap18/revision-1/arturia-pigments.json"))?;
    let second = Profile::parse(include_bytes!("../../compatibility/ap18/revision-2/arturia-pigments.json"))?;
    let third = Profile::parse(include_bytes!("../../compatibility/ap18/revision-3/arturia-pigments.json"))?;
    let fourth = Profile::parse(include_bytes!("../../compatibility/ap18/revision-4/arturia-pigments.json"))?;
    let fifth = Profile::parse(include_bytes!("../../compatibility/ap18/revision-5/arturia-pigments.json"))?;
    let sixth = Profile::parse(include_bytes!("../../compatibility/ap18/revision-6/arturia-pigments.json"))?;
    let seventh = Profile::parse(include_bytes!("../../compatibility/ap18/revision-7/arturia-pigments.json"))?;
    let eighth = Profile::parse(include_bytes!("../../compatibility/ap18/revision-8/arturia-pigments.json"))?;
    let ninth = Profile::parse(include_bytes!("../../compatibility/ap18/revision-9/arturia-pigments.json"))?;
    let tenth = candidate()?;
    Ok((*prior == first && *next == second)
        || (*prior == second && *next == third)
        || (*prior == third && *next == fourth)
        || (*prior == fourth && *next == fifth && fifth.revision == 5)
        || (*prior == fifth && *next == sixth && sixth.revision == 6)
        || (*prior == sixth && *next == seventh && seventh.revision == 7)
        || (*prior == seventh && *next == eighth && eighth.revision == 8)
        || (*prior == eighth && *next == ninth && ninth.revision == 9)
        || (*prior == ninth && *next == tenth && tenth.revision == 10))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Anchor {
    class_id: String,
    revision: RevisionRef,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Baseline {
    schema: u32,
    candidate: String,
    parents: Vec<Anchor>,
}
fn directory(m: &Manager, p: &Profile) -> Result<PathBuf> {
    Ok(m.root
        .join("software/ap18-qualification")
        .join(p.fingerprint()?))
}
fn current_baseline(
    m: &Manager,
    p: &Profile,
    policies: &[Profile],
) -> Result<(Baseline, Environment)> {
    require(
        p.claim == Claim::ReviewCandidate
            && p.revision == 10
            && p.id == "arturia-pigments"
            && p.role == Role::Instrument
            && p.capabilities.editor == Editor::DetachedDirectVendorLifecycle,
        "qualification_candidate_contract",
    )?;
    validate_set(policies)?;
    require(policies.len() == 2, "pigments_baseline_required")?;
    let transactions = m.root.join("transactions");
    if transactions.try_exists()? {
        for (n, entry) in fs::read_dir(transactions)?.enumerate() {
            require(n < 4096, "publication_record_bound")?;
            require(
                !entry?
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".pending.json"),
                "publication_recovery_pending",
            )?;
        }
    }
    let db = m.registry()?;
    let mut parents = Vec::new();
    let mut environment = None;
    for policy in policies {
        require(
            policy.claim == Claim::VerifiedExactFixture
                && policy.revision == 10
                && policy.class.class_id != p.class.class_id,
            "pigments_baseline_required",
        )?;
        let e = db
            .classes
            .get(&policy.class.class_id)
            .ok_or("pigments_baseline_required")?;
        let reference = e
            .managed_revision
            .as_ref()
            .ok_or("pigments_baseline_required")?;
        let r = m.load_revision(&policy.class.class_id, reference)?;
        require(
            r.profile == *policy
                && r.qualification.is_none()
                && r.registration == e.registration
                && e.publication == Publication::Published
                && crate::publication::physical(&m.link(&policy.class.class_id))?
                    == Some(r.target.clone())
                && !m.publication_pending(&policy.class.class_id)?,
            "pigments_baseline_changed",
        )?;
        m.verify_completed_publication(&r, reference)?;
        r.registration.verify(&m.root)?;
        p.verify_environment(
            &r.registration.environment,
            &policy.requirements.environment_family,
        )?;
        if let Some(prior) = &environment {
            require(prior == &r.registration.environment, "environment_mismatch")?;
        }
        environment = Some(r.registration.environment);
        parents.push(Anchor {
            class_id: policy.class.class_id.clone(),
            revision: reference.clone(),
        });
    }
    for (key, e) in &db.classes {
        require(
            !m.publication_pending(key)? && e.publication != Publication::Pending,
            "publication_recovery_pending",
        )?;
    }
    Ok((
        Baseline {
            schema: 1,
            candidate: p.fingerprint()?,
            parents,
        },
        environment.ok_or("pigments_baseline_required")?,
    ))
}
fn baseline(m: &Manager, p: &Profile) -> Result<Environment> {
    let (observed, environment) = current_baseline(m, p, &installed_profiles()?)?;
    let path = directory(m, p)?.join("baseline.json");
    require(
        file(&path)?.metadata()?.mode() & 0o222 == 0,
        "pigments_baseline_mutable",
    )?;
    require(
        read_json::<Baseline>(&path)? == observed,
        "pigments_baseline_changed",
    )?;
    Ok(environment)
}
fn module(environment: &Environment, p: &Profile) -> Result<Artifact> {
    let a = Artifact {
        path: environment
            .root
            .join("compatdata/pfx/drive_c/Program Files/Common Files/VST3/Pigments.vst3"),
        sha256: p.module_sha256.clone(),
    };
    require(a.path.canonicalize()? == a.path, "product_module_alias")?;
    a.verify()?;
    Ok(a)
}
fn bytes(p: &Profile) -> [(String, String); 3] {
    [
        ("host.exe".into(), p.requirements.host_sha256.clone()),
        (
            "host-source-manifest.json".into(),
            p.requirements.host_source_sha256.clone(),
        ),
        (
            format!("{}.so", p.class.class_id),
            p.requirements.native_sha256.clone(),
        ),
    ]
}
pub fn stage(m: &Manager, package: &Path) -> Result<()> {
    stage_exact(m, package, &candidate()?, &installed_profiles()?)
}
fn stage_exact(m: &Manager, package: &Path, p: &Profile, policies: &[Profile]) -> Result<()> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let (receipt, environment) = current_baseline(m, p, policies)?;
    module(&environment, p)?;
    let db = m.registry()?;
    if let Some(e) = db.classes.get(&p.class.class_id) {
        require(
            e.publication == Publication::Removed,
            "qualification_active_restore_first",
        )?;
        let prior = m.load_revision(
            &p.class.class_id,
            e.managed_revision
                .as_ref()
                .ok_or("qualification_exact_candidate_required")?,
        )?;
        require(
            replacement_prior(&prior.profile, p)? && prior.qualification == Some(Qualification::Ap18Pigments),
            "qualification_exact_candidate_required",
        )?;
    }
    require(
        crate::publication::physical(&m.link(&p.class.class_id))?.is_none(),
        "foreign_publication",
    )?;
    for (name, sha256) in bytes(p) {
        Artifact {
            path: package.join(name),
            sha256,
        }
        .verify()?;
    }
    let dest = directory(m, p)?;
    if dest.try_exists()? {
        require(
            read_json::<Baseline>(&dest.join("baseline.json"))? == receipt,
            "pigments_baseline_changed",
        )?;
        crate::qualification::load_for(m, p.clone(), Qualification::Ap18Pigments)?;
        return Ok(());
    }
    private_dir(dest.parent().ok_or("qualification_directory")?)?;
    let temp = dest.with_file_name(format!(".stage-{}", random_id()?));
    private_dir(&temp)?;
    for ((name, sha256), target) in
        bytes(p)
            .into_iter()
            .zip(["host.exe", "host-source-manifest.json", "native.so"])
    {
        let path = temp.join(target);
        fs::copy(package.join(name), &path)?;
        Artifact {
            path: path.clone(),
            sha256,
        }
        .verify()?;
        fs::set_permissions(
            &path,
            fs::Permissions::from_mode(if target == "native.so" { 0o500 } else { 0o400 }),
        )?;
        File::open(path)?.sync_all()?;
    }
    atomic_json(&temp.join("baseline.json"), &receipt)?;
    fs::set_permissions(
        temp.join("baseline.json"),
        fs::Permissions::from_mode(0o400),
    )?;
    File::open(temp.join("baseline.json"))?.sync_all()?;
    File::open(&temp)?.sync_all()?;
    crate::publication::rename_link(&temp, &dest, false)?;
    File::open(dest.parent().unwrap())?.sync_all()?;
    Ok(())
}
pub fn binding(m: &Manager) -> Result<Registration> {
    let p = candidate()?;
    let environment = baseline(m, &p)?;
    let c = crate::qualification::load_for(m, p.clone(), Qualification::Ap18Pigments)?;
    let r = Registration {
        metadata: p.class.clone(),
        module: module(&environment, &p)?,
        environment,
        host: c.host,
        host_source_sha256: c.source_manifest.sha256,
        native: c.native.artifact,
        compatibility: p.capabilities.compatibility(),
    };
    r.verify(&m.root)?;
    Ok(r)
}
pub(crate) fn check_publication(m: &Manager, p: &Profile, r: &Registration) -> Result<()> {
    require(
        *p == candidate()? && *r == binding(m)?,
        "qualification_exact_candidate_required",
    )?;
    m.require_inactive(None)?;
    require(
        m.performance(&p.class.class_id)?.added_frames == 512,
        "qualification_candidate_contract",
    )
}
pub(crate) fn retained(m: &Manager, r: &Revision, exact: &InstalledCandidate) -> Result<()> {
    let mut expected = binding(m)?;
    expected.native.path = r.registration.native.path.clone();
    require(
        r.profile == exact.profile
            && r.registration == expected
            && r.qualification == Some(Qualification::Ap18Pigments),
        "qualification_exact_candidate_required",
    )?;
    let db = m.registry()?;
    let e = db
        .classes
        .get(&r.class_id)
        .ok_or("qualification_publication_changed")?;
    let reference = e
        .managed_revision
        .as_ref()
        .ok_or("qualification_publication_changed")?;
    require(
        reference.id == r.id
            && e.registration == r.registration
            && e.publication == Publication::Published
            && crate::publication::physical(&m.link(&r.class_id))? == Some(r.target.clone())
            && !m.publication_pending(&r.class_id)?,
        "qualification_publication_changed",
    )?;
    m.verify_completed_publication(r, reference)
}
/// Current ordinary host remains the accepted LoFi/FRAGMENTS host. Only this
/// exact retained new class may use its separately sealed corrected host.
pub(crate) fn served(
    m: &Manager,
    r: &Registration,
    installed: &Artifact,
    source: &str,
) -> Result<()> {
    let p = candidate()?;
    baseline(m, &p)?;
    let policies = installed_profiles()?;
    require(
        policies.iter().all(|x| {
            x.requirements.host_sha256 == installed.sha256
                && x.requirements.host_source_sha256 == source
        }),
        "installed_host_mismatch",
    )?;
    let db = m.registry()?;
    let e = db
        .classes
        .get(&r.key())
        .ok_or("qualification_publication_changed")?;
    let revision = m.load_revision(
        &r.key(),
        e.managed_revision
            .as_ref()
            .ok_or("qualification_publication_changed")?,
    )?;
    require(revision.registration == *r, "installed_host_mismatch")?;
    m.verify_retained_authority(&revision, &[p])
}
fn already_restored(m: &Manager, key: &str, state: &Publication) -> Result<bool> {
    if *state != Publication::Removed { return Ok(false); }
    require(crate::publication::physical(&m.link(key))?.is_none(), "foreign_publication")?;
    Ok(true)
}
pub fn restore(m: &Manager) -> Result<()> {
    restore_selected(m, &candidate()?)
}
fn restore_selected(m: &Manager, p: &Profile) -> Result<()> {
    let db = m.registry()?;
    if let Some(e) = db.classes.get(&p.class.class_id) {
        let r = m.load_revision(
            &p.class.class_id,
            e.managed_revision
                .as_ref()
                .ok_or("qualification_publication_changed")?,
        )?;
        require(
            replacement_prior(&r.profile, p)? && r.qualification == Some(Qualification::Ap18Pigments),
            "qualification_exact_candidate_required",
        )?;
        if already_restored(m, &p.class.class_id, &e.publication)? { return Ok(()); }
        m.unpublish(&p.class.class_id)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observation::{derive, Census},
        test_fixture::{inspection_report, prepared, snapshot, Fixture},
    };
    fn fixture() -> (Fixture, Profile, Vec<Profile>, PathBuf) {
        let (f, mut p, c, n) = prepared();
        p.revision = 10;
        let mut policies = Vec::new();
        for (index, id) in ["01".repeat(16), "02".repeat(16)].into_iter().enumerate() {
            let mut policy = p.clone();
            policy.id = format!("fixture.parent{index}");
            policy.class.class_id = id.clone();
            let mut facts = c.clone();
            facts.selected = policy.class.clone();
            facts.report.path = f.outer.join(format!("report{index}"));
            atomic_json(&facts.report.path, &inspection_report(&facts)).unwrap();
            facts.report.sha256 = digest(&facts.report.path).unwrap();
            facts = Census::from_report(
                facts.environment,
                facts.module,
                facts.module_stamp,
                facts.host,
                facts.host_source_sha256,
                facts.report,
                &id,
            )
            .unwrap();
            let mut native = n.clone();
            native.class = policy.class.clone();
            native.external_ids = external_ids(&id).unwrap();
            f.m.managed_publish(
                &policy,
                &facts,
                derive(&policy, &facts, &native).unwrap(),
                &facts.host,
                &facts.host_source_sha256,
                None,
            )
            .unwrap();
            policies.push(policy);
        }
        p.id = "arturia-pigments".into();
        p.revision = 10;
        p.claim = Claim::ReviewCandidate;
        p.class.class_id = "03".repeat(16);
        p.capabilities.editor = Editor::DetachedDirectVendorLifecycle;
        let module = module(&f.r.environment, &p).err();
        assert!(module.is_some());
        fs::copy(
            &f.r.module.path,
            f.r.module.path.with_file_name("Pigments.vst3"),
        )
        .unwrap();
        let package = f.outer.join("package");
        private_dir(&package).unwrap();
        fs::copy(&f.r.host.path, package.join("host.exe")).unwrap();
        fs::copy(
            f.r.host.path.with_file_name("host-source-manifest.json"),
            package.join("host-source-manifest.json"),
        )
        .unwrap();
        fs::copy(
            &n.artifact.path,
            package.join(format!("{}.so", p.class.class_id)),
        )
        .unwrap();
        (f, p, policies, package)
    }
    #[test]
    fn new_class_stage_is_exact_inactive_and_does_not_publish() {
        let (f, p, policies, package) = fixture();
        let before = snapshot(&f.outer);
        for name in [
            "host.exe".into(),
            "host-source-manifest.json".into(),
            format!("{}.so", p.class.class_id),
        ] {
            let path = package.join(name);
            let bytes = fs::read(&path).unwrap();
            fs::write(&path, b"wrong").unwrap();
            let damaged = snapshot(&f.outer);
            assert!(stage_exact(&f.m, &package, &p, &policies).is_err());
            assert_eq!(snapshot(&f.outer), damaged);
            fs::write(path, bytes).unwrap();
        }
        assert_eq!(snapshot(&f.outer), before);
        let links: Vec<_> = policies
            .iter()
            .map(|p| fs::read_link(f.m.link(&p.class.class_id)).unwrap())
            .collect();
        stage_exact(&f.m, &package, &p, &policies).unwrap();
        let staged = snapshot(&f.outer);
        stage_exact(&f.m, &package, &p, &policies).unwrap();
        assert_eq!(snapshot(&f.outer), staged);
        assert!(!f.m.link(&p.class.class_id).exists());
        assert!(!f
            .m
            .registry()
            .unwrap()
            .classes
            .contains_key(&p.class.class_id));
        for (p, target) in policies.iter().zip(links) {
            assert_eq!(fs::read_link(f.m.link(&p.class.class_id)).unwrap(), target);
        }
        assert!(binding(&f.m).is_err()); // synthetic stage cannot become sealed runtime authority
    }
    #[test]
    fn baseline_changes_foreign_links_pending_and_unknown_leases_refuse_before_stage() {
        let (f, p, policies, package) = fixture();
        let mut wrong = policies.clone();
        wrong[0].revision = 7;
        assert!(stage_exact(&f.m, &package, &p, &wrong).is_err());
        let mut changed = p.clone();
        changed.requirements.runner.id.push('x');
        assert!(stage_exact(&f.m, &package, &changed, &policies).is_err());
        changed = p.clone();
        changed.module_sha256 = "00".repeat(32);
        assert!(stage_exact(&f.m, &package, &changed, &policies).is_err());
        fs::create_dir_all(&f.m.publications).unwrap();
        fs::write(f.m.link(&p.class.class_id), b"foreign").unwrap();
        let before = snapshot(&f.outer);
        assert!(stage_exact(&f.m, &package, &p, &policies).is_err());
        assert_eq!(snapshot(&f.outer), before);
        fs::remove_file(f.m.link(&p.class.class_id)).unwrap();
        let pending =
            f.m.root
                .join("transactions")
                .join(format!("{}.pending.json", policies[0].class.class_id));
        fs::write(&pending, b"unresolved").unwrap();
        let before = snapshot(&f.outer);
        assert!(stage_exact(&f.m, &package, &p, &policies).is_err());
        assert_eq!(snapshot(&f.outer), before);
        fs::remove_file(pending).unwrap();
        private_dir(&f.m.root.join("runtime/leases")).unwrap();
        fs::write(f.m.root.join("runtime/leases/unresolved.json"), b"{}").unwrap();
        let before = snapshot(&f.outer);
        assert!(stage_exact(&f.m, &package, &p, &policies).is_err());
        assert_eq!(snapshot(&f.outer), before);
    }
    #[test]
    fn ordinary_publish_and_unstaged_engineering_refuse_without_mutation() {
        let (f, _, c, n) = prepared();
        let p = candidate().unwrap();
        let before = snapshot(&f.outer);
        assert!(f
            .m
            .managed_publish(&p, &c, f.r.clone(), &c.host, &c.host_source_sha256, None)
            .is_err());
        assert_eq!(snapshot(&f.outer), before);
        assert!(f
            .m
            .qualify_for(&c, None, Qualification::Ap18Pigments)
            .is_err());
        assert_eq!(snapshot(&f.outer), before);
        assert_ne!(n.external_ids, external_ids(&p.class.class_id).unwrap());
    }
    #[test]
    fn removed_candidate_reconciliation_is_inert_and_refuses_foreign_link() {
        let (f, p, _, _) = fixture();
        let before = snapshot(&f.outer);
        assert!(already_restored(&f.m, &p.class.class_id, &Publication::Removed).unwrap());
        assert!(already_restored(&f.m, &p.class.class_id, &Publication::Removed).unwrap());
        assert!(!already_restored(&f.m, &p.class.class_id, &Publication::Published).unwrap());
        assert_eq!(snapshot(&f.outer), before);
        fs::write(f.m.link(&p.class.class_id), b"foreign").unwrap();
        let before = snapshot(&f.outer);
        assert!(already_restored(&f.m, &p.class.class_id, &Publication::Removed).is_err());
        assert_eq!(snapshot(&f.outer), before);
    }
    #[test]
    fn failed_candidate_is_history_only_and_replacement_is_exact() {
        let first_bytes = include_bytes!("../../compatibility/ap18/revision-1/arturia-pigments.json");
        let second_bytes = include_bytes!("../../compatibility/ap18/revision-2/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(first_bytes)), "681dae01625f2d4c07a106a92c5c605396b666c6cd7160fe8402efb3bf7fe3fd");
        assert_eq!(crate::hex(&sha2::Sha256::digest(second_bytes)), "e6cf278f2cf444e9a0b68dc048622ca9076783e768f1917f7ffae6c44f58f795");
        let first = Profile::parse(first_bytes).unwrap();
        let second = Profile::parse(second_bytes).unwrap();
        let third_bytes = include_bytes!("../../compatibility/ap18/revision-3/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(third_bytes)), "ad1e73d42d0049d52b02a44b9b0737c50f57749e5cec530f5f387a580647f675");
        let third = Profile::parse(third_bytes).unwrap();
        let fourth_bytes = include_bytes!("../../compatibility/ap18/revision-4/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(fourth_bytes)), "64e9773239ec910dcefd8ddfc0a15d36d8bca1689b9998c07ffc74da5583b00b");
        let fourth = Profile::parse(fourth_bytes).unwrap();
        let fifth_bytes=include_bytes!("../../compatibility/ap18/revision-5/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(fifth_bytes)),"dde2e25eb108c98d80926621f8fe9501d87a1ced93bc59bf917278f664c016e9");
        let fifth=Profile::parse(fifth_bytes).unwrap();
        let sixth_bytes=include_bytes!("../../compatibility/ap18/revision-6/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(sixth_bytes)),"ca00ca5373f76d10a3d4b60421eecbfa57e60e19e1e06e3b9a4332b20b900879");
        let sixth=Profile::parse(sixth_bytes).unwrap();
        let seventh_bytes=include_bytes!("../../compatibility/ap18/revision-7/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(seventh_bytes)),"e7c0eba8db18b8a05a4acdc996bb369f11f38ae2e2706eb50a5581dfc6254d8d");
        let seventh=Profile::parse(seventh_bytes).unwrap();
        assert_eq!(seventh.fingerprint().unwrap(),"e16221395dcf6cb8c45521ad12528f1e5b910c5a1b5c6d7cdc5f1091498dcc04");
        let eighth_bytes=include_bytes!("../../compatibility/ap18/revision-8/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(eighth_bytes)),"bbb68e8a5cf0d401fc39ab33ff4e39bd71385d3bc5bf9a557f5b23af1e4a439b");
        let eighth=Profile::parse(eighth_bytes).unwrap();
        let ninth_bytes=include_bytes!("../../compatibility/ap18/revision-9/arturia-pigments.json");
        assert_eq!(crate::hex(&sha2::Sha256::digest(ninth_bytes)),"a7ee1933ef287c730c496e88ab66e17c4948b5386e24b88606c8296d2d925dad");
        let ninth=Profile::parse(ninth_bytes).unwrap();
        assert_eq!(ninth.fingerprint().unwrap(),"13f6ec3d3230a5bc96a2089cfcb040c386cdd6acd258e5ce099f094c4102123c");
        let tenth=candidate().unwrap();
        assert_eq!(tenth.revision,10);
        assert!(replacement_prior(&ninth,&tenth).unwrap());
        assert!(!replacement_prior(&eighth,&tenth).unwrap());
        assert!(!replacement_prior(&tenth,&ninth).unwrap());
        assert!(tenth.claim.require(SelectionPurpose::Activation).is_err());
        let mut normalized=tenth.clone();normalized.revision=ninth.revision;
        normalized.requirements.host_sha256=ninth.requirements.host_sha256.clone();
        normalized.requirements.host_source_sha256=ninth.requirements.host_source_sha256.clone();
        normalized.evidence=ninth.evidence.clone();assert_eq!(normalized,ninth);
        assert_eq!(external_ids(&tenth.class.class_id).unwrap(),external_ids(&ninth.class.class_id).unwrap());
        for change in 0..3 {
            let mut wrong=tenth.clone();
            match change {0=>wrong.module_sha256="00".repeat(32),1=>wrong.requirements.native_sha256="00".repeat(32),_=>wrong.requirements.host_sha256="00".repeat(32)}
            assert!(!replacement_prior(&ninth,&wrong).unwrap());
        }
        println!("LC1 revision10 fingerprint {}",tenth.fingerprint().unwrap());
        assert!(replacement_prior(&eighth,&ninth).unwrap());
        assert!(!replacement_prior(&seventh,&ninth).unwrap());
        assert!(!replacement_prior(&ninth,&eighth).unwrap());
        assert!(ninth.claim.require(SelectionPurpose::Activation).is_err());
        let mut report_normalized=ninth.clone();
        report_normalized.revision=eighth.revision;
        report_normalized.requirements.host_sha256=eighth.requirements.host_sha256.clone();
        report_normalized.requirements.host_source_sha256=eighth.requirements.host_source_sha256.clone();
        report_normalized.evidence=eighth.evidence.clone();
        assert_eq!(report_normalized,eighth);
        for change in 0..3 {
            let mut wrong=ninth.clone();
            match change {0=>wrong.module_sha256="00".repeat(32),1=>wrong.requirements.native_sha256="00".repeat(32),_=>wrong.requirements.host_sha256="00".repeat(32)}
            assert!(!replacement_prior(&eighth,&wrong).unwrap());
        }
        assert_eq!(eighth.fingerprint().unwrap(),"bd52a44bd7e786776361a172a99599c9082233caf4643a293cd1ebc194e0acc4");
        assert!(replacement_prior(&seventh,&eighth).unwrap());
        assert!(!replacement_prior(&sixth,&eighth).unwrap());
        assert!(!replacement_prior(&eighth,&seventh).unwrap());
        assert!(eighth.claim.require(SelectionPurpose::Activation).is_err());
        assert_eq!(eighth.capabilities.vendor_retirement,Some(VendorRetirement::ProcessScopedVendorRetirement));
        assert_eq!(external_ids(&eighth.class.class_id).unwrap(),external_ids(&seventh.class.class_id).unwrap());
        let mut retirement_normalized=eighth.clone();
        retirement_normalized.revision=seventh.revision;
        retirement_normalized.requirements.host_sha256=seventh.requirements.host_sha256.clone();
        retirement_normalized.requirements.host_source_sha256=seventh.requirements.host_source_sha256.clone();
        retirement_normalized.capabilities.vendor_retirement=None;
        retirement_normalized.evidence=seventh.evidence.clone();
        assert_eq!(retirement_normalized,seventh);
        for change in 0..3 {
            let mut wrong=eighth.clone();
            match change {0=>wrong.module_sha256="00".repeat(32),1=>wrong.requirements.native_sha256="00".repeat(32),_=>wrong.requirements.host_sha256="00".repeat(32)}
            assert!(!replacement_prior(&seventh,&wrong).unwrap());
        }
        assert!(replacement_prior(&sixth,&seventh).unwrap());
        assert!(!replacement_prior(&fifth,&seventh).unwrap());
        assert!(!replacement_prior(&seventh,&sixth).unwrap());
        assert!(seventh.claim.require(SelectionPurpose::Activation).is_err());
        assert_eq!(seventh.capabilities.editor_lifetime,Some(EditorLifetime::RetainEditorViewUntilInstanceRetirement));
        let mut lifetime_normalized=seventh.clone();
        lifetime_normalized.revision=sixth.revision;
        lifetime_normalized.requirements.host_sha256=sixth.requirements.host_sha256.clone();
        lifetime_normalized.requirements.host_source_sha256=sixth.requirements.host_source_sha256.clone();
        lifetime_normalized.capabilities.editor_lifetime=None;
        lifetime_normalized.evidence=sixth.evidence.clone();
        assert_eq!(lifetime_normalized,sixth);
        assert_eq!(external_ids(&seventh.class.class_id).unwrap(),external_ids(&sixth.class.class_id).unwrap());
        assert!(replacement_prior(&fifth,&sixth).unwrap());
        assert!(!replacement_prior(&fourth,&sixth).unwrap());
        assert!(!replacement_prior(&sixth,&fifth).unwrap());
        assert_eq!(sixth.capabilities.event_output,Some(EventOutputPolicy::ReportedZeroEventChannelsUnspecified));
        assert!(sixth.claim.require(SelectionPurpose::Activation).is_err());
        assert_eq!(external_ids(&sixth.class.class_id).unwrap(),external_ids(&fifth.class.class_id).unwrap());
        let mut normalized=sixth.clone();normalized.revision=fifth.revision;
        normalized.requirements=fifth.requirements.clone();normalized.capabilities.event_output=None;normalized.evidence=fifth.evidence.clone();
        assert_eq!(normalized,fifth);
        assert!(replacement_prior(&fourth, &fifth).unwrap());
        assert!(!replacement_prior(&third, &fifth).unwrap());
        assert!(!replacement_prior(&fifth, &fourth).unwrap());
        let mut custody_normalized=fifth.clone();
        custody_normalized.revision=fourth.revision;
        custody_normalized.requirements.host_sha256=fourth.requirements.host_sha256.clone();
        custody_normalized.requirements.host_source_sha256=fourth.requirements.host_source_sha256.clone();
        custody_normalized.evidence=fourth.evidence.clone();
        assert_eq!(custody_normalized,fourth);
        assert!(replacement_prior(&third, &fourth).unwrap());
        assert!(!replacement_prior(&second, &fourth).unwrap());
        assert!(!replacement_prior(&first, &fourth).unwrap());
        assert!(!replacement_prior(&fourth, &third).unwrap());
        assert!(fourth.limitations.contains(&Limitation::ReturnedResultDiagnosis));
        let mut diagnostic_normalized=fourth.clone();
        diagnostic_normalized.revision=third.revision;
        diagnostic_normalized.requirements.host_sha256=third.requirements.host_sha256.clone();
        diagnostic_normalized.requirements.host_source_sha256=third.requirements.host_source_sha256.clone();
        diagnostic_normalized.limitations.retain(|x|*x!=Limitation::ReturnedResultDiagnosis);
        diagnostic_normalized.evidence=third.evidence.clone();
        assert_eq!(diagnostic_normalized,third);
        assert!(replacement_prior(&first, &second).unwrap());
        assert!(replacement_prior(&second, &third).unwrap());
        assert!(!replacement_prior(&first, &third).unwrap());
        assert!(!replacement_prior(&third, &second).unwrap());
        assert!(!replacement_prior(&second, &first).unwrap());
        for p in [&first,&second,&third,&fourth,&fifth] {
            assert!(p.claim.require(SelectionPurpose::Activation).is_err());
            assert_eq!(external_ids(&p.class.class_id).unwrap(),external_ids(&first.class.class_id).unwrap());
        }
        for change in 0..3 {
            let mut wrong=second.clone();
            match change {0=>wrong.module_sha256="00".repeat(32),1=>wrong.requirements.native_sha256="00".repeat(32),_=>wrong.requirements.host_sha256="00".repeat(32)}
            assert!(!replacement_prior(&wrong,&third).unwrap());
        }
        assert!(third.limitations.contains(&Limitation::SoleStereoAuxiliaryInputOnly));
        assert!(!third.limitations.contains(&Limitation::AuxiliaryInputInactive));
        let mut normalized=third;
        normalized.revision=second.revision;
        normalized.requirements.native_sha256=second.requirements.native_sha256.clone();
        normalized.requirements.native_source_commit=second.requirements.native_source_commit.clone();
        normalized.requirements.host_sha256=second.requirements.host_sha256.clone();
        normalized.requirements.host_source_sha256=second.requirements.host_source_sha256.clone();
        normalized.limitations=second.limitations.clone();
        normalized.evidence=second.evidence.clone();
        assert_eq!(normalized,second);
    }
    #[test]
    fn compiled_candidate_is_new_identity_not_ordinary_or_ap17_policy() {
        let p = candidate().unwrap();
        assert_eq!(p.revision, 10);
        assert_eq!(p.class.name, "Pigments");
        assert!(p.claim.require(SelectionPurpose::Activation).is_err());
        assert!(p.claim.require(SelectionPurpose::Qualification).is_ok());
        assert_eq!(p.capabilities.accessibility, Accessibility::WindowsDefault);
        assert!(installed_profiles()
            .unwrap()
            .iter()
            .all(|x| x.revision == 10 && x.class.class_id != p.class.class_id));
        let base = capacity::fixture_limits();
        let extended = capacity::service_limits().unwrap();
        assert_eq!(&extended.classes[..2], base.classes);
        assert_eq!(extended.classes[2].class_id, p.class.class_id);
        assert_eq!(extended.classes[2].dsp, 1);
        assert_eq!(extended.global_dsp, 6);
        assert_eq!(extended.native_image_hard, 4);
        assert_eq!(extended.service_workers, 16);
    }
}

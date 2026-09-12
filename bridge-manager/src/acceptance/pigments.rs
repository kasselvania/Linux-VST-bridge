//! The single AP18 accepted candidate -> ordinary product transition.
//! All policy inputs are compiled. Public commands supply no profile or artifact.
use super::*;
pub const REVIEW: &[u8] = include_bytes!("../../../evidence/ap18/acceptance/review.json");
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct History {
    reference: RevisionRef,
    transaction: String,
    profile_revision: u32,
    profile_sha256: String,
    parent: Option<RevisionRef>,
    qualification: Qualification,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Baseline {
    class_id: String,
    revision: RevisionRef,
    profile_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProfileIdentity {
    revision: u32,
    path: String,
    sha256: String,
    fingerprint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Seal {
    schema: u32,
    review: u64,
    head: String,
    tree: String,
    product_source: String,
    candidate_profile_sha256: String,
    candidate: RevisionRef,
    transaction: String,
    prior_software_sha256: String,
    prior_manager_sha256: String,
    baseline: Vec<Baseline>,
    history: Vec<History>,
    profiles: Vec<ProfileIdentity>,
    evidence: Vec<EvidenceIdentity>,
    disposition: String,
}
const EVIDENCE: &[(&str, &[u8])] = &[
    (
        "evidence/ap18/lc1/artifacts.json",
        include_bytes!("../../../evidence/ap18/lc1/artifacts.json"),
    ),
    (
        "evidence/ap18/lc1/regression-before.json",
        include_bytes!("../../../evidence/ap18/lc1/regression-before.json"),
    ),
    (
        "evidence/ap18/lc1/regression-after.json",
        include_bytes!("../../../evidence/ap18/lc1/regression-after.json"),
    ),
    (
        "evidence/ap18/lc1/live-completion.json",
        include_bytes!("../../../evidence/ap18/lc1/live-completion.json"),
    ),
    (
        "evidence/ap18/lc1/installed-final.json",
        include_bytes!("../../../evidence/ap18/lc1/installed-final.json"),
    ),
    (
        "evidence/ap18/lc1/validation.json",
        include_bytes!("../../../evidence/ap18/lc1/validation.json"),
    ),
];
const HISTORY: &[&[u8]] = &[
    include_bytes!("../../../compatibility/ap18/revision-1/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-2/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-3/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-4/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-5/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-6/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-7/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-8/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/revision-9/arturia-pigments.json"),
    include_bytes!("../../../compatibility/ap18/arturia-pigments.json"),
];
fn verify_seal(seal: &Seal) -> Result<()> {
    let compiled: Seal = serde_json::from_slice(REVIEW)?;
    require(
        serde_json::to_vec(seal)? == serde_json::to_vec(&compiled)?
            && seal.review == 5185372983
            && seal.head == "f8d0dbfb481324e84e8b337014b66947978b146c"
            && seal.tree == "89fb026133ef585a9fcd7475ed4aec919c1236c7"
            && seal.schema == 1
            && seal.profiles.len() == 10
            && seal.evidence.len() == EVIDENCE.len(),
        "acceptance_review_identity",
    )?;
    for (s, (path, bytes)) in seal.evidence.iter().zip(EVIDENCE) {
        require(
            s.path == *path && s.sha256 == hex(&sha2::Sha256::digest(bytes)),
            "acceptance_evidence_identity",
        )?;
    }
    for (index, (s, bytes)) in seal.profiles.iter().zip(HISTORY).enumerate() {
        let p = Profile::parse(bytes)?;
        require(
            s.revision == index as u32 + 1
                && p.revision == s.revision
                && s.sha256 == hex(&sha2::Sha256::digest(bytes))
                && s.fingerprint == p.fingerprint()?
                && p.claim == Claim::ReviewCandidate,
            "acceptance_history_identity",
        )?;
    }
    Ok(())
}
fn normalized(p: &Profile, c: &Profile) -> Result<()> {
    p.validate()?;
    c.validate()?;
    let mut expected = c.clone();
    expected.revision = 11;
    expected.claim = Claim::VerifiedExactFixture;
    expected.limitations.retain(|l| {
        !matches!(
            l,
            Limitation::PigmentsUnderQualification | Limitation::ReturnedResultDiagnosis
        )
    });
    expected.evidence = p.evidence.clone();
    require(
        c.revision == 10
            && c.claim == Claim::ReviewCandidate
            && *p == expected
            && c.evidence.iter().all(|e| p.evidence.contains(e)),
        "acceptance_profile_transition",
    )
}
fn candidate_identity(p: &Profile, c: &Profile, r: &Revision, seal: &Seal) -> Result<()> {
    normalized(p, c)?;
    require(
        c.fingerprint()? == seal.candidate_profile_sha256
            && r.id == seal.candidate.id
            && r.profile == *c
            && r.profile_sha256 == seal.candidate_profile_sha256
            && r.transaction == seal.transaction
            && r.qualification == Some(Qualification::Ap18Pigments)
            && r.external_ids == external_ids(&c.class.class_id)?
            && r.performance.added_frames == 512,
        "acceptance_candidate_identity",
    )
}

/// Existing setup owns the registry lock and stopped-service requirement.
pub fn prepare(m: &Manager) -> Result<AcceptedSoftware> {
    let seal: Seal = serde_json::from_slice(REVIEW)?;
    verify_seal(&seal)?;
    let sw: Software = read_json(&m.root.join("software.json"))?;
    verify_prior_software(m, &seal, &sw)?;
    for a in [
        &sw.manager,
        &sw.supervisor,
        &sw.ownership,
        &sw.host,
        &sw.source_manifest,
    ] {
        a.verify()?;
    }
    for owner in capacity::owners(m)? {
        require(
            owner.kind == capacity::Kind::Keeper,
            "active_lease_unresolved",
        )?;
        let report: PathBuf = read_json(
            &m.root
                .join("runtime/leases")
                .join(format!("{}.json", owner.session)),
        )?;
        require(
            super::retired_keeper(&report, &owner.session)?,
            "active_lease_unresolved",
        )?;
    }
    let p = pigments_verified()?;
    let c = crate::pigments::candidate()?;
    let result = prepare_selected(m, &seal, &p, &c, &ap17_profiles()?, &sw)?;
    let db = m.registry()?;
    for anchor in &seal.baseline {
        let e = db
            .classes
            .get(&anchor.class_id)
            .ok_or("acceptance_parent_identity")?;
        require(
            e.managed_revision.as_ref() == Some(&anchor.revision),
            "acceptance_parent_identity",
        )?;
        require(
            m.load_revision(&anchor.class_id, &anchor.revision)?
                .profile_sha256
                == anchor.profile_sha256,
            "acceptance_parent_identity",
        )?;
    }
    let mut next = Some(seal.candidate.clone());
    for h in &seal.history {
        require(
            next.as_ref() == Some(&h.reference),
            "acceptance_history_identity",
        )?;
        let r = m.load_revision(&p.class.class_id, &h.reference)?;
        require(
            r.profile.revision == h.profile_revision
                && r.profile_sha256 == h.profile_sha256
                && r.transaction == h.transaction
                && r.parent == h.parent
                && r.qualification == Some(h.qualification),
            "acceptance_history_identity",
        )?;
        m.verify_completed_publication(&r, &h.reference)?;
        next = r.parent;
    }
    require(next.is_none(), "acceptance_history_identity")?;
    Ok(result)
}

// Setup may complete before the first ordinary inspection/publication. Permit
// an installation repair from this same immutable acceptance receipt, still
// subject to every exact inactive-candidate/history/artifact check below.
// This does not admit another review, candidate, or already-active publication.
fn verify_prior_software(m: &Manager, seal: &Seal, sw: &Software) -> Result<()> {
    if digest(&m.root.join("software.json"))? == seal.prior_software_sha256 {
        return require(
            sw.manager.sha256 == seal.prior_manager_sha256,
            "acceptance_prior_software_identity",
        );
    }
    sw.manager.verify()?;
    let receipt = sw.manager.path.with_file_name("acceptance-review.json");
    for path in [&sw.manager.path, &receipt] {
        require(
            path.starts_with(m.root.join("software"))
                && path.canonicalize()? == *path
                && file(path)?.metadata()?.mode() & 0o222 == 0,
            "acceptance_prior_software_identity",
        )?;
    }
    require(
        fs::read(receipt)? == REVIEW,
        "acceptance_prior_software_identity",
    )?;
    let catalogue = sw.catalogue(m)?;
    require(
        catalogue.schema == 2 && catalogue.natives.len() == 3 && catalogue.hosts.len() == 1,
        "acceptance_catalogue_identity",
    )
}
fn prepare_selected(
    m: &Manager,
    seal: &Seal,
    p: &Profile,
    c: &Profile,
    baseline: &[Profile],
    sw: &Software,
) -> Result<AcceptedSoftware> {
    m.require_inactive(None)?;
    let environment = crate::pigments::baseline_for(m, c, baseline)?;
    let db = m.registry()?;
    let e = db
        .classes
        .get(&c.class.class_id)
        .ok_or("acceptance_candidate_identity")?;
    require(
        e.publication == Publication::Removed
            && e.managed_revision.as_ref() == Some(&seal.candidate)
            && physical(&m.link(&c.class.class_id))?.is_none(),
        "acceptance_candidate_must_be_inactive",
    )?;
    let r = m.load_revision(&c.class.class_id, &seal.candidate)?;
    candidate_identity(p, c, &r, seal)?;
    require(
        r.registration == e.registration && r.registration.environment == environment,
        "acceptance_candidate_identity",
    )?;
    m.verify_completed_publication(&r, &seal.candidate)?;
    require(
        read_json::<Profile>(&m.root.join("profiles").join(&c.id).join("10.json"))? == *c,
        "acceptance_profile_identity",
    )?;
    let exact = qualification::load_for(m, c.clone(), Qualification::Ap18Pigments)?;
    r.census.verify_current(
        &m.root,
        &exact.host,
        &exact.source_manifest.sha256,
        r.census.captured_at,
    )?;
    let mut derived = derive_for(c, &r.census, &exact.native, SelectionPurpose::Qualification)?;
    derived.native = r.registration.native.clone();
    require(derived == r.registration, "acceptance_artifact_identity")?;
    let mut catalogue = sw.catalogue(m)?;
    if catalogue.natives.len() == baseline.len() + 1 && catalogue.hosts.len() == 1 {
        // Exact already-installed acceptance artifacts can be recopied to a new
        // immutable software revision. All unrelated entries remain refused.
        catalogue.native(p)?;
        let host = catalogue.host(p, &sw.host, &sw.source_sha256)?;
        require(host == catalogue.hosts[0], "acceptance_catalogue_identity")?;
        catalogue
            .natives
            .retain(|n| n.class.class_id != p.class.class_id);
        catalogue.hosts.clear();
    }
    require(
        catalogue.natives.len() == baseline.len() && catalogue.hosts.is_empty(),
        "acceptance_catalogue_identity",
    )?;
    for p in baseline {
        catalogue.native(p)?;
        catalogue.host(p, &sw.host, &sw.source_sha256)?;
    }
    require(
        catalogue.environments == vec![r.census.environment.clone()],
        "acceptance_catalogue_identity",
    )?;
    catalogue.schema = 2;
    catalogue.natives.push(exact.native);
    catalogue.hosts.push(HostArtifact {
        host: exact.host,
        source_manifest: exact.source_manifest,
    });
    catalogue.validate(&m.root)?;
    Ok(AcceptedSoftware {
        host: sw.host.clone(),
        source_manifest: sw.source_manifest.clone(),
        catalogue,
    })
}

/// Narrow exception to the historical-candidate predecessor prohibition. The
/// publication itself still enters through managed_publish/Activation, carries
/// no qualification marker, and uses the existing immutable transaction.
pub(crate) fn accepted_predecessor(m: &Manager, p: &Profile, r: &Revision) -> Result<bool> {
    if *p != pigments_verified()? || r.profile != crate::pigments::candidate()? {
        return Ok(false);
    }
    let seal: Seal = serde_json::from_slice(REVIEW)?;
    verify_seal(&seal)?;
    candidate_identity(p, &crate::pigments::candidate()?, r, &seal)?;
    m.verify_completed_publication(r, &seal.candidate)?;
    let sw: Software = read_json(&m.root.join("software.json"))?;
    require(
        fs::read(sw.manager.path.with_file_name("acceptance-review.json"))? == REVIEW,
        "acceptance_review_identity",
    )?;
    let catalogue = sw.catalogue(m)?;
    catalogue.native(p)?;
    catalogue.host(p, &sw.host, &sw.source_sha256)?;
    crate::pigments::baseline_for(m, &r.profile, &ap17_profiles()?)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::{prepared, snapshot};
    #[test]
    fn revision_eleven_is_only_the_accepted_normalization_and_history_is_immutable() {
        let seal: Seal = serde_json::from_slice(REVIEW).unwrap();
        verify_seal(&seal).unwrap();
        let p = pigments_verified().unwrap();
        let c = crate::pigments::candidate().unwrap();
        normalized(&p, &c).unwrap();
        assert_eq!(p.revision, 11);
        assert!(p.claim.permits(SelectionPurpose::Activation));
        for bytes in HISTORY {
            assert!(!Profile::parse(bytes)
                .unwrap()
                .claim
                .permits(SelectionPurpose::Activation));
        }
        assert_eq!(p.requirements, c.requirements);
        assert_eq!(p.capabilities, c.capabilities);
        assert_eq!(
            external_ids(&p.class.class_id).unwrap(),
            [
                "2a87961b6a485bdea59c9009fc661460",
                "ce18a28bb102517c9e5db63bb7d72d9b"
            ]
        );
        assert_eq!(
            installed_profiles().unwrap(),
            [ap17_profiles().unwrap(), vec![p.clone()]].concat()
        );
        println!(
            "AP18 ordinary revision 11 fingerprint {}",
            p.fingerprint().unwrap()
        );
        for change in 0..13 {
            let mut wrong = p.clone();
            match change {
                0 => wrong.requirements.host_sha256 = "aa".repeat(32),
                1 => wrong.requirements.host_source_sha256 = "aa".repeat(32),
                2 => wrong.requirements.native_sha256 = "aa".repeat(32),
                3 => wrong.requirements.native_source_commit = "aa".repeat(20),
                4 => wrong.requirements.descriptor_sha256 = "aa".repeat(32),
                5 => wrong.module_sha256 = "aa".repeat(32),
                6 => wrong.class.class_id = "AA".repeat(16),
                7 => wrong.requirements.runner.version.push('x'),
                8 => wrong.requirements.environment_revision += 1,
                9 => wrong.capabilities.event_output = None,
                10 => wrong.capabilities.vendor_retirement = None,
                11 => wrong
                    .limitations
                    .retain(|l| *l != Limitation::ShortDeliveryGaps),
                _ => wrong.evidence.clear(),
            }
            assert!(normalized(&wrong, &c).is_err(), "change {change}");
        }
    }
    #[test]
    fn acceptance_seal_refuses_changed_review_history_evidence_or_artifact_authority() {
        let seal: Seal = serde_json::from_slice(REVIEW).unwrap();
        for change in 0..17 {
            let mut wrong = seal.clone();
            match change {
                0 => wrong.review += 1,
                1 => wrong.head = "ab".repeat(20),
                2 => wrong.tree = "ab".repeat(20),
                3 => wrong.product_source = "ab".repeat(20),
                4 => wrong.candidate_profile_sha256 = "ab".repeat(32),
                5 => wrong.candidate.id = "ab".repeat(16),
                6 => wrong.candidate.sha256 = "ab".repeat(32),
                7 => wrong.transaction = "ab".repeat(16),
                8 => wrong.prior_software_sha256 = "ab".repeat(32),
                9 => wrong.prior_manager_sha256 = "ab".repeat(32),
                10 => wrong.baseline[0].revision.id = "ab".repeat(16),
                11 => wrong.profiles[9].fingerprint = "ab".repeat(32),
                12 => wrong.profiles[9].sha256 = "ab".repeat(32),
                13 => wrong.history[0].transaction = "ab".repeat(16),
                14 => wrong.evidence[0].sha256 = "ab".repeat(32),
                15 => wrong.history.pop().map(|_| ()).unwrap(),
                _ => wrong.baseline[0].class_id = "AB".repeat(16),
            }
            assert!(verify_seal(&wrong).is_err(), "change {change}");
        }
        let (f, _, _, _) = prepared();
        let before = snapshot(&f.outer);
        assert!(prepare(&f.m).is_err());
        assert_eq!(before, snapshot(&f.outer));
    }
    #[test]
    fn supplemental_host_is_exact_ordinary_software_authority_and_preserves_default() {
        let (f, mut p, mut census, native) = prepared();
        let original =
            f.m.managed_publish(
                &p,
                &census,
                derive(&p, &census, &native).unwrap(),
                &census.host,
                &census.host_source_sha256,
                None,
            )
            .unwrap();
        let old = census.host.clone();
        let old_source = census.host_source_sha256.clone();
        let dir = f.m.root.join("software/accepted-host");
        private_dir(&dir).unwrap();
        let make = |name: &str, bytes: &[u8]| {
            let path = dir.join(name);
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
            Artifact {
                sha256: digest(&path).unwrap(),
                path,
            }
        };
        let host = HostArtifact {
            host: make("host.exe", b"accepted host"),
            source_manifest: make("host-source-manifest.json", b"exact accepted source"),
        };
        p.requirements.host_sha256 = host.host.sha256.clone();
        p.requirements.host_source_sha256 = host.source_manifest.sha256.clone();
        p.revision = 11;
        let mut cat = Catalogue {
            schema: 2,
            natives: vec![native.clone()],
            environments: vec![census.environment.clone()],
            hosts: vec![host.clone()],
        };
        cat.validate(&f.m.root).unwrap();
        assert_eq!(cat.host(&p, &old, &old_source).unwrap(), host);
        let mut legacy = p.clone();
        legacy.requirements.host_sha256 = old.sha256.clone();
        legacy.requirements.host_source_sha256 = old_source.clone();
        assert_eq!(cat.host(&legacy, &old, &old_source).unwrap().host, old);
        assert!(current_host(&f.m, &old, &old_source, &p).is_err());
        let cat_path = f.m.root.join("software/native-catalogue.json");
        atomic_json(&cat_path, &cat).unwrap();
        let sw = Software {
            manager: old.clone(),
            supervisor: old.clone(),
            ownership: old.clone(),
            host: old.clone(),
            source_manifest: Artifact {
                path: old.path.with_file_name("host-source-manifest.json"),
                sha256: old_source.clone(),
            },
            source_sha256: old_source.clone(),
            native_catalogue: Some(Artifact {
                sha256: digest(&cat_path).unwrap(),
                path: cat_path.clone(),
            }),
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        assert_eq!(current_host(&f.m, &old, &old_source, &p).unwrap(), host);
        census.host = host.host.clone();
        census.host_source_sha256 = host.source_manifest.sha256.clone();
        census.report.path = f.outer.join("accepted-inspection.json");
        atomic_json(
            &census.report.path,
            &crate::test_fixture::inspection_report(&census),
        )
        .unwrap();
        census.report.sha256 = digest(&census.report.path).unwrap();
        let r = derive(&p, &census, &native).unwrap();
        let reference =
            f.m.managed_publish(
                &p,
                &census,
                r.clone(),
                &host.host,
                &host.source_manifest.sha256,
                None,
            )
            .unwrap();
        let installed = f.m.load_revision(&p.class.class_id, &reference).unwrap();
        assert!(installed.qualification.is_none());
        assert_eq!(installed.parent, Some(original));
        f.m.verify_served_host(&installed.registration, &old, &old_source, &[p.clone()])
            .unwrap();
        assert_eq!(
            f.m.managed_publish(
                &p,
                &census,
                r.clone(),
                &host.host,
                &host.source_manifest.sha256,
                None
            )
            .unwrap(),
            reference
        );
        let before = snapshot(&f.outer);
        let mut candidate = p.clone();
        candidate.claim = Claim::ReviewCandidate;
        assert!(f
            .m
            .managed_publish(
                &candidate,
                &census,
                r,
                &host.host,
                &host.source_manifest.sha256,
                None
            )
            .is_err());
        assert_eq!(snapshot(&f.outer), before);
        cat.schema = 1;
        assert!(cat.validate(&f.m.root).is_err());
        cat.schema = 2;
        cat.hosts.push(host.clone());
        assert!(cat.validate(&f.m.root).is_err());
        cat.hosts.pop();
        for change in 0..4 {
            let mut wrong = p.clone();
            match change {
                0 => wrong.requirements.host_sha256 = old.sha256.clone(),
                1 => wrong.requirements.host_source_sha256 = old_source.clone(),
                2 => wrong.requirements.native_sha256 = "00".repeat(32),
                _ => wrong.class.class_id = "AA".repeat(16),
            }
            assert!(current_host(&f.m, &old, &old_source, &wrong).is_err());
        }
        fs::set_permissions(&host.host.path, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(cat.validate(&f.m.root).is_err());
        fs::write(&host.host.path, b"foreign").unwrap();
        assert!(current_host(&f.m, &old, &old_source, &p).is_err());
    }
    fn accepted_fixture() -> (
        crate::test_fixture::Fixture,
        Seal,
        Profile,
        Profile,
        Vec<Profile>,
        Software,
    ) {
        let (f, mut c, baseline, package) = crate::pigments::tests::fixture();
        c.limitations.extend([
            Limitation::PigmentsUnderQualification,
            Limitation::ReturnedResultDiagnosis,
        ]);
        fs::write(package.join("host.exe"), b"distinct accepted host").unwrap();
        c.requirements.host_sha256 = digest(&package.join("host.exe")).unwrap();
        crate::pigments::stage_exact(&f.m, &package, &c, &baseline).unwrap();
        let exact = qualification::load_for(&f.m, c.clone(), Qualification::Ap18Pigments).unwrap();
        let mut cat = adoption(&f.m, &baseline).unwrap();
        for n in &mut cat.natives {
            let path =
                f.m.root
                    .join("software")
                    .join(format!("{}.so", n.class.class_id));
            fs::copy(&n.artifact.path, &path).unwrap();
            n.artifact.path = path;
        }
        let mut census =
            f.m.load_revision(
                &baseline[0].class.class_id,
                f.m.registry().unwrap().classes[&baseline[0].class.class_id]
                    .managed_revision
                    .as_ref()
                    .unwrap(),
            )
            .unwrap()
            .census;
        census.selected = c.class.clone();
        census.module.path = census.module.path.with_file_name("Pigments.vst3");
        census.module_stamp = ModuleStamp::read(&census.module.path).unwrap();
        census.host = exact.host.clone();
        census.host_source_sha256 = exact.source_manifest.sha256.clone();
        census.report.path = f.outer.join("accepted-census.json");
        atomic_json(
            &census.report.path,
            &crate::test_fixture::inspection_report(&census),
        )
        .unwrap();
        census.report.sha256 = digest(&census.report.path).unwrap();
        census = Census::from_report(
            census.environment,
            census.module,
            census.module_stamp,
            census.host,
            census.host_source_sha256,
            census.report,
            &c.class.class_id,
        )
        .unwrap();
        // Materialize synthetic retained input records through the production
        // publication writer, then encode their candidate claim for this reader
        // fixture. No private test policy can enter the public acceptance seal.
        let mut written = c.clone();
        written.claim = Claim::VerifiedExactFixture;
        let reference =
            f.m.managed_publish(
                &written,
                &census,
                derive(&written, &census, &exact.native).unwrap(),
                &census.host,
                &census.host_source_sha256,
                None,
            )
            .unwrap();
        f.m.unpublish(&c.class.class_id).unwrap();
        let mut r = f.m.load_revision(&c.class.class_id, &reference).unwrap();
        r.profile = c.clone();
        r.profile_sha256 = c.fingerprint().unwrap();
        r.qualification = Some(Qualification::Ap18Pigments);
        let write = |path: &Path, bytes: &[u8]| {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(path, bytes).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
        };
        let mut bytes = serde_json::to_vec(&r).unwrap();
        bytes.push(b'\n');
        write(&r.target.parent().unwrap().join("revision.json"), &bytes);
        write(&r.target.join("bridge-provenance.json"), &bytes);
        let reference = RevisionRef {
            id: r.id.clone(),
            sha256: hex(&sha2::Sha256::digest(&bytes)),
        };
        let mut profile_bytes = serde_json::to_vec(&c).unwrap();
        profile_bytes.push(b'\n');
        write(
            &f.m.root.join("profiles").join(&c.id).join("10.json"),
            &profile_bytes,
        );
        let intent_path =
            f.m.root
                .join("transactions")
                .join(format!("{}.json", r.transaction));
        let mut intent: serde_json::Value = read_json(&intent_path).unwrap();
        intent["candidate"] = serde_json::to_value(&reference).unwrap();
        write(&intent_path, &serde_json::to_vec(&intent).unwrap());
        let mut db = f.m.registry().unwrap();
        db.classes
            .get_mut(&c.class.class_id)
            .unwrap()
            .managed_revision = Some(reference.clone());
        atomic_json(&f.m.root.join("registry.json"), &db).unwrap();
        let mut seal: Seal = serde_json::from_slice(REVIEW).unwrap();
        seal.candidate = reference;
        seal.candidate_profile_sha256 = c.fingerprint().unwrap();
        seal.transaction = r.transaction;
        let mut p = c.clone();
        p.revision = 11;
        p.claim = Claim::VerifiedExactFixture;
        p.limitations.retain(|l| {
            !matches!(
                l,
                Limitation::PigmentsUnderQualification | Limitation::ReturnedResultDiagnosis
            )
        });
        let path = f.m.root.join("software/native-catalogue.json");
        atomic_json(&path, &cat).unwrap();
        let sw = Software {
            manager: f.r.host.clone(),
            supervisor: f.r.host.clone(),
            ownership: f.r.host.clone(),
            host: f.r.host.clone(),
            source_manifest: Artifact {
                path: f.r.host.path.with_file_name("host-source-manifest.json"),
                sha256: f.r.host_source_sha256.clone(),
            },
            source_sha256: f.r.host_source_sha256.clone(),
            native_catalogue: Some(Artifact {
                sha256: digest(&path).unwrap(),
                path,
            }),
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        (f, seal, p, c, baseline, sw)
    }
    #[test]
    fn exact_retained_candidate_assembles_without_mutation_and_refuses_drift() {
        let (f, seal, p, c, baseline, sw) = accepted_fixture();
        let before = snapshot(&f.outer);
        let result = prepare_selected(&f.m, &seal, &p, &c, &baseline, &sw).unwrap();
        assert_eq!(result.host, sw.host);
        assert_eq!(result.catalogue.natives.len(), 3);
        assert_eq!(result.catalogue.hosts.len(), 1);
        assert_eq!(
            result
                .catalogue
                .host(&p, &sw.host, &sw.source_sha256)
                .unwrap()
                .host
                .sha256,
            c.requirements.host_sha256
        );
        assert_eq!(snapshot(&f.outer), before);
        assert!(prepare(&f.m).is_err()); // private fixture never meets compiled seal
        for change in 0..9 {
            let mut wrong = seal.clone();
            let mut profile = p.clone();
            match change {
                0 => wrong.candidate.id = "ab".repeat(16),
                1 => wrong.candidate.sha256 = "ab".repeat(32),
                2 => wrong.candidate_profile_sha256 = "ab".repeat(32),
                3 => wrong.transaction = "ab".repeat(16),
                4 => profile.requirements.descriptor_sha256 = "ab".repeat(32),
                5 => profile.requirements.environment_revision += 1,
                6 => profile.requirements.runner.version.push('x'),
                7 => profile.module_sha256 = "ab".repeat(32),
                _ => profile.class.class_id = "AB".repeat(16),
            }
            assert!(
                prepare_selected(&f.m, &wrong, &profile, &c, &baseline, &sw).is_err(),
                "change {change}"
            );
            assert_eq!(snapshot(&f.outer), before);
        }
        let pending = f.m.root.join("transactions/foreign.pending.json");
        fs::write(&pending, b"unresolved").unwrap();
        let pending_state = snapshot(&f.outer);
        assert!(prepare_selected(&f.m, &seal, &p, &c, &baseline, &sw).is_err());
        assert_eq!(snapshot(&f.outer), pending_state);
        fs::remove_file(pending).unwrap();
        fs::write(f.m.link(&c.class.class_id), b"foreign").unwrap();
        assert!(prepare_selected(&f.m, &seal, &p, &c, &baseline, &sw).is_err());
        fs::remove_file(f.m.link(&c.class.class_id)).unwrap();
        let lease = f.m.root.join("runtime/leases");
        private_dir(&lease).unwrap();
        fs::write(lease.join("unknown.json"), b"{}").unwrap();
        assert!(prepare_selected(&f.m, &seal, &p, &c, &baseline, &sw).is_err());
    }

    #[test]
    fn same_acceptance_installation_retry_requires_exact_receipt_and_artifacts() {
        let (f, seal, p, c, baseline, mut sw) = accepted_fixture();
        let installed = prepare_selected(&f.m, &seal, &p, &c, &baseline, &sw).unwrap();
        let manager = f.m.root.join("software/retry/linux-vst-bridge");
        private_dir(manager.parent().unwrap()).unwrap();
        fs::write(&manager, b"accepted manager").unwrap();
        fs::set_permissions(&manager, fs::Permissions::from_mode(0o500)).unwrap();
        sw.manager = Artifact {
            sha256: digest(&manager).unwrap(),
            path: manager,
        };
        let path = sw.manager.path.with_file_name("native-catalogue.json");
        atomic_json(&path, &installed.catalogue).unwrap();
        sw.native_catalogue = Some(Artifact {
            sha256: digest(&path).unwrap(),
            path,
        });
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        assert!(verify_prior_software(&f.m, &seal, &sw).is_err());
        let receipt = sw.manager.path.with_file_name("acceptance-review.json");
        fs::write(&receipt, REVIEW).unwrap();
        fs::set_permissions(&receipt, fs::Permissions::from_mode(0o400)).unwrap();
        verify_prior_software(&f.m, &seal, &sw).unwrap();
        let before = snapshot(&f.outer);
        let retry = prepare_selected(&f.m, &seal, &p, &c, &baseline, &sw).unwrap();
        assert_eq!(retry.catalogue, installed.catalogue);
        assert_eq!(snapshot(&f.outer), before);
        for field in 0..3 {
            let mut wrong = installed.catalogue.clone();
            match field {
                0 => wrong.natives[2].descriptor_sha256 = "ab".repeat(32),
                1 => wrong.hosts[0].host.sha256 = "ab".repeat(32),
                _ => wrong.hosts[0].source_manifest.sha256 = "ab".repeat(32),
            }
            let a = sw.native_catalogue.as_mut().unwrap();
            atomic_json(&a.path, &wrong).unwrap();
            a.sha256 = digest(&a.path).unwrap();
            assert!(prepare_selected(&f.m, &seal, &p, &c, &baseline, &sw).is_err());
        }
        fs::set_permissions(&receipt, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&receipt, b"another review").unwrap();
        fs::set_permissions(&receipt, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(verify_prior_software(&f.m, &seal, &sw).is_err());
    }
}

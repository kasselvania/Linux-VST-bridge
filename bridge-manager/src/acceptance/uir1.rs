//! Exact reviewed UIR1 host -> ordinary Pigments 13. No caller policy inputs.
//! Setup owns copying/pointer replacement; normal managed publication owns links.
use super::*;
pub const REVIEW: &[u8] = include_bytes!("../../../evidence/uir1/acceptance/review.json");
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Seal {
    schema: u32,
    review: u64,
    head: String,
    tree: String,
    candidate: RevisionRef,
    candidate_fingerprint: String,
    transaction: String,
    parent: RevisionRef,
    prior_software_sha256: String,
    prior_manager_sha256: String,
    baseline: Vec<(String, RevisionRef)>,
    history: Vec<EvidenceIdentity>,
    evidence: Vec<EvidenceIdentity>,
    disposition: String,
}
const EVIDENCE: &[(&str, &[u8])] = &[
    (
        "evidence/uir1/differential.json",
        include_bytes!("../../../evidence/uir1/differential.json"),
    ),
    (
        "evidence/uir1/repair/differential-after.json",
        include_bytes!("../../../evidence/uir1/repair/differential-after.json"),
    ),
    (
        "evidence/uir1/repair/product.json",
        include_bytes!("../../../evidence/uir1/repair/product.json"),
    ),
    (
        "evidence/uir1/repair/artifacts.json",
        include_bytes!("../../../evidence/uir1/repair/artifacts.json"),
    ),
    (
        "evidence/uir1/repair/installed-final.json",
        include_bytes!("../../../evidence/uir1/repair/installed-final.json"),
    ),
];
const HISTORY: &[(&str, &[u8])] = &[
    (
        "compatibility/ap18/revision-11/arturia-pigments.json",
        include_bytes!("../../../compatibility/ap18/revision-11/arturia-pigments.json"),
    ),
    (
        "compatibility/uir1/arturia-pigments.json",
        include_bytes!("../../../compatibility/uir1/arturia-pigments.json"),
    ),
];
fn verify_seal(s: &Seal) -> Result<()> {
    let compiled: Seal = serde_json::from_slice(REVIEW)?;
    require(
        serde_json::to_vec(s)? == serde_json::to_vec(&compiled)?
            && s.schema == 1
            && s.review == 5187281110
            && s.head == "9f745d75b44b63dd0c095061f1f273189c7a19f8"
            && s.tree == "d991280785278e193ed7187fcc45e7dbbb4cd98a",
        "acceptance_review_identity",
    )?;
    for (records, inputs) in [(&s.history, HISTORY), (&s.evidence, EVIDENCE)] {
        require(
            records.len() == inputs.len(),
            "acceptance_evidence_identity",
        )?;
        for (r, (path, bytes)) in records.iter().zip(inputs) {
            require(
                r.path == *path && r.sha256 == hex(&sha2::Sha256::digest(bytes)),
                "acceptance_evidence_identity",
            )?;
        }
    }
    Ok(())
}
fn normalized(p: &Profile, c: &Profile) -> Result<()> {
    p.validate()?;
    c.validate()?;
    let mut expected = c.clone();
    expected.revision = 13;
    expected.claim = Claim::VerifiedExactFixture;
    expected.evidence = p.evidence.clone();
    require(
        c.revision == 12 && c.claim == Claim::ReviewCandidate && *p == expected,
        "acceptance_profile_transition",
    )
}
/// Read-only preflight under setup's locks, before any software/link mutation.
pub fn prepare(m: &Manager) -> Result<AcceptedSoftware> {
    let seal: Seal = serde_json::from_slice(REVIEW)?;
    verify_seal(&seal)?;
    let sw: Software = read_json(&m.root.join("software.json"))?;
    if digest(&m.root.join("software.json"))? == seal.prior_software_sha256 {
        require(
            sw.manager.sha256 == seal.prior_manager_sha256,
            "acceptance_prior_software_identity",
        )?;
    } else {
        // Retry only the same completed immutable acceptance installation,
        // still requiring the exact restored parent and candidate below.
        let receipt = sw.manager.path.with_file_name("acceptance-review.json");
        require(
            sw.manager.path.starts_with(m.root.join("software"))
                && sw.manager.path.canonicalize()? == sw.manager.path
                && file(&sw.manager.path)?.metadata()?.mode() & 0o222 == 0
                && file(&receipt)?.metadata()?.mode() & 0o222 == 0
                && fs::read(receipt)? == REVIEW,
            "acceptance_prior_software_identity",
        )?;
    }
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
    prepare_selected(
        m,
        &seal,
        &pigments_verified()?,
        &qualification::uir1_candidate()?,
        &pigments_eleven()?,
        &ap17_profiles()?,
        &sw,
    )
}
fn prepare_selected(
    m: &Manager,
    s: &Seal,
    p: &Profile,
    c: &Profile,
    parent: &Profile,
    siblings: &[Profile],
    sw: &Software,
) -> Result<AcceptedSoftware> {
    normalized(p, c)?;
    m.require_inactive(None)?;
    let db = m.registry()?;
    for key in db.classes.keys() {
        require(!m.publication_pending(key)?, "publication_recovery_pending")?;
    }
    for entry in fs::read_dir(m.root.join("transactions"))? {
        require(
            !entry?
                .file_name()
                .to_string_lossy()
                .ends_with(".pending.json"),
            "publication_recovery_pending",
        )?;
    }
    let r = m.load_revision(&c.class.class_id, &s.candidate)?;
    require(
        r.profile == *c
            && r.profile_sha256 == s.candidate_fingerprint
            && c.fingerprint()? == s.candidate_fingerprint
            && r.transaction == s.transaction
            && r.parent.as_ref() == Some(&s.parent)
            && r.qualification == Some(Qualification::Uir1Input),
        "acceptance_candidate_identity",
    )?;
    require(
        read_json::<Profile>(&m.root.join("profiles").join(&c.id).join("12.json"))? == *c,
        "acceptance_profile_identity",
    )?;
    m.verify_completed_publication(&r, &s.candidate)?;
    let exact = qualification::load_for(m, c.clone(), Qualification::Uir1Input)?;
    r.registration.verify(&m.root)?;
    r.census.verify_current(
        &m.root,
        &exact.host,
        &exact.source_manifest.sha256,
        r.census.captured_at,
    )?;
    let mut derived = derive_for(c, &r.census, &exact.native, SelectionPurpose::Qualification)?;
    derived.native = r.registration.native.clone();
    require(
        derived == r.registration
            && exact.host == r.registration.host
            && exact.source_manifest.sha256 == r.registration.host_source_sha256
            && exact.native.artifact.sha256 == r.registration.native.sha256,
        "acceptance_artifact_identity",
    )?;
    let prior =
        m.verify_qualification_parent_for(&db, c, &r.registration, Qualification::Uir1Input)?;
    require(
        prior.profile == *parent
            && db.classes[&c.class.class_id].managed_revision.as_ref() == Some(&s.parent),
        "acceptance_parent_identity",
    )?;
    m.verify_completed_publication(&prior, &s.parent)?;
    require(
        s.baseline.len() == siblings.len(),
        "acceptance_parent_identity",
    )?;
    for sibling in siblings {
        let (key, reference) = s
            .baseline
            .iter()
            .find(|(key, _)| *key == sibling.class.class_id)
            .ok_or("acceptance_parent_identity")?;
        let e = db.classes.get(key).ok_or("acceptance_parent_identity")?;
        let old = m.load_revision(key, reference)?;
        require(
            old.profile == *sibling
                && old.qualification.is_none()
                && e.managed_revision.as_ref() == Some(reference)
                && e.registration == old.registration
                && e.publication == Publication::Published
                && physical(&m.link(key))? == Some(old.target.clone()),
            "acceptance_parent_identity",
        )?;
        old.registration.verify(&m.root)?;
        m.verify_completed_publication(&old, reference)?;
    }
    let mut catalogue = sw.catalogue(m)?;
    require(
        catalogue.schema == 2
            && catalogue.natives.len() == siblings.len() + 1
            && catalogue.environments == vec![r.census.environment.clone()],
        "acceptance_catalogue_identity",
    )?;
    catalogue.native(parent)?;
    let old_host = catalogue.host(parent, &sw.host, &sw.source_sha256)?;
    let new_host = HostArtifact {
        host: exact.host,
        source_manifest: exact.source_manifest,
    };
    for sibling in siblings {
        catalogue.native(sibling)?;
        catalogue.host(sibling, &sw.host, &sw.source_sha256)?;
    }
    let same = |a: &HostArtifact, b: &HostArtifact| {
        a.host.sha256 == b.host.sha256 && a.source_manifest.sha256 == b.source_manifest.sha256
    };
    require(
        !catalogue.hosts.is_empty()
            && catalogue
                .hosts
                .iter()
                .all(|h| same(h, &old_host) || same(h, &new_host)),
        "acceptance_catalogue_identity",
    )?;
    if !catalogue.hosts.iter().any(|h| same(h, &new_host)) {
        catalogue.hosts.push(new_host);
    }
    catalogue.native(p)?;
    catalogue.host(p, &sw.host, &sw.source_sha256)?;
    catalogue.validate(&m.root)?;
    Ok(AcceptedSoftware {
        host: sw.host.clone(),
        source_manifest: sw.source_manifest.clone(),
        catalogue,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::{inspection_report, prepared, snapshot, Fixture};
    #[test]
    fn thirteen_is_only_reviewed_twelve_and_eleven_remains_immutable() {
        let seal: Seal = serde_json::from_slice(REVIEW).unwrap();
        verify_seal(&seal).unwrap();
        let p = pigments_verified().unwrap();
        let c = qualification::uir1_candidate().unwrap();
        normalized(&p, &c).unwrap();
        assert_eq!(p.revision, 13);
        assert_eq!(p.requirements, c.requirements);
        assert_eq!(p.capabilities, c.capabilities);
        assert_eq!(p.limitations, c.limitations);
        assert_eq!(
            external_ids(&p.class.class_id).unwrap(),
            external_ids(&c.class.class_id).unwrap()
        );
        assert!(installed_profiles().unwrap().contains(&p));
        assert!(!installed_profiles().unwrap().contains(&c));
        assert!(p.claim.permits(SelectionPurpose::Activation));
        assert!(!c.claim.permits(SelectionPurpose::Activation));
        assert_eq!(
            pigments_eleven().unwrap().fingerprint().unwrap(),
            "3126fea7ea72c02bae08fd21cef87e271e5575b09bb419bc1d650ab273e172ad"
        );
        for change in 0..10 {
            let mut wrong = seal.clone();
            match change {
                0 => wrong.review += 1,
                1 => wrong.tree.push('0'),
                2 => wrong.head.push('0'),
                3 => wrong.candidate.sha256 = "ab".repeat(32),
                4 => wrong.candidate_fingerprint = "ab".repeat(32),
                5 => wrong.transaction = "ab".repeat(16),
                6 => wrong.parent.id = "ab".repeat(16),
                7 => wrong.evidence[0].sha256 = "ab".repeat(32),
                8 => wrong.history[0].sha256 = "ab".repeat(32),
                _ => wrong.prior_manager_sha256 = "ab".repeat(32),
            }
            assert!(verify_seal(&wrong).is_err());
        }
        println!("UIR1 ordinary 13 fingerprint {}", p.fingerprint().unwrap());
    }
    fn fixture() -> (Fixture, Seal, Profile, Profile, Profile, Software) {
        let (f, mut old, mut census, native) = prepared();
        old.revision = 11;
        old.capabilities.editor = Editor::DetachedDirectVendorLifecycle;
        let parent =
            f.m.managed_publish(
                &old,
                &census,
                derive(&old, &census, &native).unwrap(),
                &census.host,
                &census.host_source_sha256,
                None,
            )
            .unwrap();
        let old_host = HostArtifact {
            host: census.host.clone(),
            source_manifest: Artifact {
                path: census.host.path.with_file_name("host-source-manifest.json"),
                sha256: census.host_source_sha256.clone(),
            },
        };
        for a in [&old_host.host, &old_host.source_manifest] {
            fs::set_permissions(&a.path, fs::Permissions::from_mode(0o400)).unwrap();
        }
        let cat = Catalogue {
            schema: 2,
            natives: vec![native.clone()],
            environments: vec![census.environment.clone()],
            hosts: vec![old_host.clone()],
        };
        let path = f.m.root.join("software/native-catalogue.json");
        atomic_json(&path, &cat).unwrap();
        let default_path = f.m.root.join("software/default-host.exe");
        fs::write(&default_path, b"unchanged default host").unwrap();
        let default_manifest = f.m.root.join("software/default-source.json");
        fs::write(&default_manifest, b"unchanged default source").unwrap();
        let default_host = Artifact {
            sha256: digest(&default_path).unwrap(),
            path: default_path,
        };
        let default_source = Artifact {
            sha256: digest(&default_manifest).unwrap(),
            path: default_manifest,
        };
        let sw = Software {
            manager: old_host.host.clone(),
            supervisor: old_host.host.clone(),
            ownership: old_host.host.clone(),
            host: default_host,
            source_sha256: default_source.sha256.clone(),
            source_manifest: default_source,
            native_catalogue: Some(Artifact {
                sha256: digest(&path).unwrap(),
                path,
            }),
        };
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        let mut c = old.clone();
        c.revision = 12;
        c.claim = Claim::ReviewCandidate;
        let package = f.outer.join("package");
        private_dir(&package).unwrap();
        fs::write(package.join("host.exe"), b"fair host").unwrap();
        fs::write(package.join("host-source-manifest.json"), b"fair source").unwrap();
        fs::copy(
            &native.artifact.path,
            package.join(format!("{}.so", c.class.class_id)),
        )
        .unwrap();
        c.requirements.host_sha256 = digest(&package.join("host.exe")).unwrap();
        c.requirements.host_source_sha256 =
            digest(&package.join("host-source-manifest.json")).unwrap();
        qualification::stage_selected_for(&f.m, &package, &[c.clone()], Qualification::Uir1Input)
            .unwrap();
        let exact = qualification::load_for(&f.m, c.clone(), Qualification::Uir1Input).unwrap();
        census.host = exact.host.clone();
        census.host_source_sha256 = exact.source_manifest.sha256.clone();
        census.report.path = f.outer.join("ui-census.json");
        atomic_json(&census.report.path, &inspection_report(&census)).unwrap();
        census.report.sha256 = digest(&census.report.path).unwrap();
        let reference =
            f.m.publish_selected(
                &c,
                &census,
                derive_for(&c, &census, &exact.native, SelectionPurpose::Qualification).unwrap(),
                (&census.host, &census.host_source_sha256),
                Some(Qualification::Uir1Input),
                None,
            )
            .unwrap();
        let candidate = f.m.load_revision(&c.class.class_id, &reference).unwrap();
        f.m.restore_editor_qualifications().unwrap();
        let mut seal: Seal = serde_json::from_slice(REVIEW).unwrap();
        seal.candidate = reference;
        seal.parent = parent;
        seal.transaction = candidate.transaction;
        seal.candidate_fingerprint = c.fingerprint().unwrap();
        seal.baseline.clear();
        let mut p = c.clone();
        p.revision = 13;
        p.claim = Claim::VerifiedExactFixture;
        (f, seal, p, c, old, sw)
    }
    #[test]
    fn exact_preparation_refuses_drift_then_ordinary_publish_and_rollback() {
        let (f, s, p, c, old, mut sw) = fixture();
        let before = snapshot(&f.outer);
        let accepted = prepare_selected(&f.m, &s, &p, &c, &old, &[], &sw).unwrap();
        assert_eq!(snapshot(&f.outer), before);
        assert_eq!(accepted.catalogue.hosts.len(), 2);
        assert!(prepare(&f.m).is_err()); // synthetic fixture cannot satisfy public seal
        for change in 0..10 {
            let mut q = p.clone();
            let mut seal = s.clone();
            match change {
                0 => q.requirements.host_sha256 = "ab".repeat(32),
                1 => q.requirements.host_source_sha256 = "ab".repeat(32),
                2 => q.requirements.native_sha256 = "ab".repeat(32),
                3 => q.requirements.descriptor_sha256 = "ab".repeat(32),
                4 => q.module_sha256 = "ab".repeat(32),
                5 => q.requirements.runner.version.push('x'),
                6 => q.requirements.environment_revision += 1,
                7 => q.class.class_id = "AB".repeat(16),
                8 => seal.transaction = "ab".repeat(16),
                _ => seal.parent.id = "ab".repeat(16),
            }
            assert!(prepare_selected(&f.m, &seal, &q, &c, &old, &[], &sw).is_err());
            assert_eq!(snapshot(&f.outer), before);
        }
        for (path, bytes) in [
            (
                f.m.root.join("transactions/foreign.pending.json"),
                b"pending".as_slice(),
            ),
            (
                f.m.root.join("runtime/leases/unknown.json"),
                b"{}".as_slice(),
            ),
        ] {
            private_dir(path.parent().unwrap()).unwrap();
            fs::write(&path, bytes).unwrap();
            let blocked = snapshot(&f.outer);
            assert!(prepare_selected(&f.m, &s, &p, &c, &old, &[], &sw).is_err());
            assert_eq!(snapshot(&f.outer), blocked);
            fs::remove_file(path).unwrap();
        }
        let link = f.m.link(&p.class.class_id);
        let target = fs::read_link(&link).unwrap();
        fs::remove_file(&link).unwrap();
        fs::write(&link, b"foreign").unwrap();
        assert!(prepare_selected(&f.m, &s, &p, &c, &old, &[], &sw).is_err());
        fs::remove_file(&link).unwrap();
        std::os::unix::fs::symlink(target, &link).unwrap();
        let a = sw.native_catalogue.as_mut().unwrap();
        atomic_json(&a.path, &accepted.catalogue).unwrap();
        a.sha256 = digest(&a.path).unwrap();
        atomic_json(&f.m.root.join("software.json"), &sw).unwrap();
        assert_eq!(
            prepare_selected(&f.m, &s, &p, &c, &old, &[], &sw)
                .unwrap()
                .catalogue,
            accepted.catalogue
        );
        let r = f.m.load_revision(&p.class.class_id, &s.candidate).unwrap();
        let n = accepted.catalogue.native(&p).unwrap();
        let reg = derive(&p, &r.census, n).unwrap();
        assert!(f
            .m
            .managed_publish(
                &c,
                &r.census,
                reg.clone(),
                &r.census.host,
                &r.census.host_source_sha256,
                None
            )
            .is_err());
        let pub13 =
            f.m.managed_publish(
                &p,
                &r.census,
                reg.clone(),
                &r.census.host,
                &r.census.host_source_sha256,
                None,
            )
            .unwrap();
        let active = f.m.load_revision(&p.class.class_id, &pub13).unwrap();
        assert_eq!(active.parent, Some(s.parent.clone()));
        assert!(active.qualification.is_none());
        assert_eq!(
            f.m.managed_publish(
                &p,
                &r.census,
                reg,
                &r.census.host,
                &r.census.host_source_sha256,
                None
            )
            .unwrap(),
            pub13
        );
        f.m.verify_served_host(
            &active.registration,
            &sw.host,
            &sw.source_sha256,
            std::slice::from_ref(&p),
        )
        .unwrap();
        f.m.rollback(&p.class.class_id, &s.parent.id, None).unwrap();
        let restored = f.m.load_revision(&p.class.class_id, &s.parent).unwrap();
        f.m.verify_served_host(&restored.registration, &sw.host, &sw.source_sha256, &[p])
            .unwrap();
        assert_eq!(fs::read_link(link).unwrap(), restored.target);
    }
}

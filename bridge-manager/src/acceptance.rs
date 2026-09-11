//! Sealed AP15 and AP17 independent-review transitions. No caller-selected profile, artifact,
//! review or publication can grant acceptance authority.
use crate::{catalogue::*, observation::*, profiles::*, publication::*, *};

pub const REVIEW: &[u8] = include_bytes!("../../evidence/ap15/acceptance/review.json");
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AcceptedProduct {
    pub class_id: String,
    pub candidate_profile_sha256: String,
    pub candidate: RevisionRef,
    pub parent: RevisionRef,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Review {
    pub schema: u32,
    pub review: u64,
    pub head: String,
    pub tree: String,
    pub products: Vec<AcceptedProduct>,
}
pub struct AcceptedSoftware {
    pub host: Artifact,
    pub source_manifest: Artifact,
    pub catalogue: Catalogue,
}

/// Called under the existing setup and registry locks. Read-only: rejection
/// precedes candidate software creation, commands, registry or link mutation.
pub fn prepare(m: &Manager) -> Result<AcceptedSoftware> {
    let review: Review = serde_json::from_slice(REVIEW)?;
    let profiles = ap15_profiles()?;
    let candidates = qualification::candidates()?;
    require(
        profiles.len() == 2 && candidates.len() == 2,
        "acceptance_roster",
    )?;
    prepare_selected(m, &review, &profiles, &candidates, &ap14_profiles()?)
}

// Private production seam: synthetic fixture bytes exercise the same readers,
// derivation and physical laws. No CLI input reaches these policy arguments.
pub(crate) fn prepare_selected(
    m: &Manager,
    review: &Review,
    profiles: &[Profile],
    candidates: &[Profile],
    parents: &[Profile],
) -> Result<AcceptedSoftware> {
    prepare_selected_for(
        m,
        review,
        profiles,
        candidates,
        parents,
        Qualification::Ap15Editor,
    )
}

pub(crate) fn prepare_selected_for(
    m: &Manager,
    review: &Review,
    profiles: &[Profile],
    candidates: &[Profile],
    parents: &[Profile],
    purpose: Qualification,
) -> Result<AcceptedSoftware> {
    let (review_id, head, tree, revision, candidate_revision, limitation) = match purpose {
        Qualification::Ap15Editor => (
            5161767138,
            "a84761133f15897a9526269f2eeb35a268419c15",
            "d99e259837bec91eb5be6ef302644a3b72595dee",
            7,
            6,
            Limitation::DirectEditorUnderQualification,
        ),
        Qualification::Ap17Capacity => (
            5174642190,
            "20f2c3a7382aa7dd0abb973c7ab09d708919ea29",
            "7e4fa0866b51a7ec415cb4a4039444835178ced1",
            10,
            9,
            Limitation::CapacityUnderQualification,
        ),
    };
    require(
        review.schema == 1
            && review.review == review_id
            && review.head == head
            && review.tree == tree
            && review.products.len() == profiles.len()
            && candidates.len() == profiles.len()
            && parents.len() == profiles.len(),
        "acceptance_review_identity",
    )?;
    for set in [profiles, candidates, parents] {
        validate_set(set)?;
    }
    m.require_inactive(None)?;
    let db = m.registry()?;
    for key in db.classes.keys() {
        require(!m.publication_pending(key)?, "publication_recovery_pending")?;
    }
    let transactions = m.root.join("transactions");
    if transactions.exists() {
        for item in fs::read_dir(transactions)? {
            require(
                !item?
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".pending.json"),
                "publication_recovery_pending",
            )?;
        }
    }
    let mut natives = Vec::new();
    let mut environments: Vec<EnvironmentBinding> = Vec::new();
    let mut host = None;
    let mut source_manifest = None;
    for ((p, candidate), parent_profile) in profiles.iter().zip(candidates).zip(parents) {
        let mut normalized = p.clone();
        normalized.revision = candidate_revision;
        normalized.claim = Claim::ReviewCandidate;
        normalized.evidence = candidate.evidence.clone();
        normalized.limitations.push(limitation.clone());
        require(
            p.revision == revision
                && p.claim == Claim::VerifiedExactFixture
                && p.capabilities.editor == Editor::DetachedDirectVendorLifecycle
                && candidate.revision == candidate_revision
                && normalized == *candidate
                && candidate.evidence.iter().all(|e| p.evidence.contains(e)),
            "acceptance_profile_transition",
        )?;
        let seals: Vec<_> = review
            .products
            .iter()
            .filter(|s| s.class_id == p.class.class_id)
            .collect();
        require(seals.len() == 1, "acceptance_roster")?;
        let seal = seals[0];
        require(
            seal.candidate_profile_sha256 == candidate.fingerprint()?,
            "acceptance_profile_identity",
        )?;
        let retained = m.load_revision(&p.class.class_id, &seal.candidate)?;
        require(
            read_json::<Profile>(
                &m.root
                    .join("profiles")
                    .join(&candidate.id)
                    .join(format!("{candidate_revision}.json")),
            )? == *candidate,
            "acceptance_profile_identity",
        )?;
        require(
            retained.profile == *candidate
                && retained.parent.as_ref() == Some(&seal.parent)
                && retained.qualification == Some(purpose)
                && retained.external_ids == external_ids(&p.class.class_id)?
                && retained.performance.added_frames == 512,
            "acceptance_candidate_identity",
        )?;
        m.verify_completed_publication(&retained, &seal.candidate)?;
        // Existing qualification law verifies the active physical parent and
        // all unchanged registration/environment/module/SDK constraints.
        let prior = match purpose {
            Qualification::Ap15Editor => {
                m.verify_qualification_parent(&db, candidate, &retained.registration)?
            }
            Qualification::Ap17Capacity => {
                m.verify_qualification_parent_for(&db, candidate, &retained.registration, purpose)?
            }
        };
        require(
            prior.profile == *parent_profile
                && prior.id == seal.parent.id
                && db.classes[&p.class.class_id].managed_revision.as_ref() == Some(&seal.parent),
            "acceptance_parent_identity",
        )?;
        let exact = match purpose {
            Qualification::Ap15Editor => qualification::load(m, candidate.clone())?,
            Qualification::Ap17Capacity => qualification::load_for(m, candidate.clone(), purpose)?,
        };
        require(
            exact.host == retained.registration.host
                && exact.source_manifest.sha256 == retained.registration.host_source_sha256,
            "acceptance_artifact_identity",
        )?;
        // Retained evidence is not a fresh scan: validate its original timestamp
        // and reconsume its exact supervised report against CURRENT local files.
        // Normal managed publication still obtains a current <=600s census.
        retained.census.verify_current(
            &m.root,
            &exact.host,
            &exact.source_manifest.sha256,
            retained.census.captured_at,
        )?;
        let mut derived = derive_for(
            candidate,
            &retained.census,
            &exact.native,
            SelectionPurpose::Qualification,
        )?;
        derived.native = retained.registration.native.clone();
        require(
            derived == retained.registration,
            "acceptance_candidate_identity",
        )?;
        require(
            host.as_ref().is_none_or(|h| h == &exact.host)
                || host
                    .as_ref()
                    .is_some_and(|h: &Artifact| h.sha256 == exact.host.sha256),
            "acceptance_host_identity",
        )?;
        require(
            source_manifest
                .as_ref()
                .is_none_or(|s: &Artifact| s.sha256 == exact.source_manifest.sha256),
            "acceptance_host_identity",
        )?;
        host.get_or_insert(exact.host);
        source_manifest.get_or_insert(exact.source_manifest);
        let binding = retained.census.environment.clone();
        if let Some(old) = environments
            .iter()
            .find(|e| e.environment.id == binding.environment.id)
        {
            require(*old == binding, "environment_mismatch")?;
        } else {
            environments.push(binding);
        }
        natives.push(exact.native);
    }
    let catalogue = Catalogue {
        schema: 1,
        natives,
        environments,
    };
    catalogue.validate(&m.root)?;
    Ok(AcceptedSoftware {
        host: host.ok_or("acceptance_roster")?,
        source_manifest: source_manifest.ok_or("acceptance_roster")?,
        catalogue,
    })
}

/// Finite AP17 acceptance seal. The CLI accepts no policy inputs.
pub const CAPACITY_REVIEW: &[u8] = include_bytes!("../../evidence/ap17/acceptance/review.json");
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceIdentity {
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapacityReview {
    pub review: Review,
    pub prior_software_sha256: String,
    pub prior_manager_sha256: String,
    pub prior_manager_source: String,
    pub limits: capacity::Limits,
    pub parallel_tracks: usize,
    pub serial_bridged_depth: usize,
    pub simultaneous_editors: usize,
    pub evidence: Vec<EvidenceIdentity>,
}
pub(crate) fn verify_capacity_seal(seal: &CapacityReview) -> Result<()> {
    let compiled: CapacityReview = serde_json::from_slice(CAPACITY_REVIEW)?;
    require(
        serde_json::to_vec(seal)? == serde_json::to_vec(&compiled)?
            && seal.limits == capacity::fixture_limits()
            && (
                seal.parallel_tracks,
                seal.serial_bridged_depth,
                seal.simultaneous_editors,
            ) == (3, 3, 2),
        "acceptance_capacity_identity",
    )?;
    let evidence: &[(&str, &[u8])] = &[
        (
            "evidence/ap17/r1/artifacts.json",
            include_bytes!("../../evidence/ap17/r1/artifacts.json"),
        ),
        (
            "evidence/ap17/r1/deployment-and-restoration.json",
            include_bytes!("../../evidence/ap17/r1/deployment-and-restoration.json"),
        ),
        (
            "evidence/ap17/r1/recall-and-failure.json",
            include_bytes!("../../evidence/ap17/r1/recall-and-failure.json"),
        ),
        (
            "evidence/ap17/r1/serial-corner.json",
            include_bytes!("../../evidence/ap17/r1/serial-corner.json"),
        ),
        (
            "evidence/ap17/r1/service-recovery.json",
            include_bytes!("../../evidence/ap17/r1/service-recovery.json"),
        ),
        (
            "evidence/ap17/r1/reboot-recovery.json",
            include_bytes!("../../evidence/ap17/r1/reboot-recovery.json"),
        ),
        (
            "evidence/ap17/r1/installed-final.json",
            include_bytes!("../../evidence/ap17/r1/installed-final.json"),
        ),
        (
            "evidence/ap17/r1/completion-validation.json",
            include_bytes!("../../evidence/ap17/r1/completion-validation.json"),
        ),
    ];
    require(
        seal.evidence.len() == evidence.len(),
        "acceptance_evidence_identity",
    )?;
    for (expected, (path, bytes)) in seal.evidence.iter().zip(evidence) {
        require(
            expected.path == *path && expected.sha256 == hex(&sha2::Sha256::digest(bytes)),
            "acceptance_evidence_identity",
        )?;
    }
    Ok(())
}
pub fn prepare_capacity(m: &Manager) -> Result<AcceptedSoftware> {
    let seal: CapacityReview = serde_json::from_slice(CAPACITY_REVIEW)?;
    verify_capacity_seal(&seal)?;
    require(
        digest(&m.root.join("software.json"))? == seal.prior_software_sha256,
        "acceptance_prior_software_identity",
    )?;
    // The exact prior software record pins the manager, helpers and catalogue.
    let old: serde_json::Value = read_json(&m.root.join("software.json"))?;
    require(
        old["manager"]["sha256"].as_str() == Some(&seal.prior_manager_sha256),
        "acceptance_prior_software_identity",
    )?;
    let profiles = installed_profiles()?;
    let candidates = qualification::candidates_for(Qualification::Ap17Capacity)?;
    require(
        profiles.len() == 2 && candidates.len() == 2,
        "acceptance_roster",
    )?;
    // Unknown or unresolved lease metadata is never permission for setup.
    require(
        capacity::owners(m)?
            .iter()
            .all(|o| o.kind == capacity::Kind::Keeper),
        "active_lease_unresolved",
    )?;
    for owner in capacity::owners(m)? {
        // Setup requires the old service stopped. A retained keeper lease is
        // harmless only with its exact positive ownership retirement receipt.
        let report: PathBuf = read_json(
            &m.root
                .join("runtime/leases")
                .join(format!("{}.json", owner.session)),
        )?;
        let receipt: serde_json::Value = read_json(&report.with_extension("ownership.json"))?;
        require(
            receipt["session"].as_str() == Some(&owner.session)
                && receipt["transport_retired"] == true
                && receipt["cleanup_confirmed"] == true,
            "active_lease_unresolved",
        )?;
    }
    prepare_selected_for(
        m,
        &seal.review,
        &profiles,
        &candidates,
        &ap15_profiles()?,
        Qualification::Ap17Capacity,
    )
}

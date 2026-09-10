//! One sealed AP15 review transition. No caller-selected profile, artifact,
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
    let profiles = installed_profiles()?;
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
    require(
        review.schema == 1
            && review.review == 5161767138
            && review.head == "a84761133f15897a9526269f2eeb35a268419c15"
            && review.tree == "d99e259837bec91eb5be6ef302644a3b72595dee"
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
        normalized.revision = 6;
        normalized.claim = Claim::ReviewCandidate;
        normalized.evidence = candidate.evidence.clone();
        normalized
            .limitations
            .push(Limitation::DirectEditorUnderQualification);
        require(
            p.revision == 7
                && p.claim == Claim::VerifiedExactFixture
                && p.capabilities.editor == Editor::DetachedDirectVendorLifecycle
                && candidate.revision == 6
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
            read_json::<Profile>(&m.root.join("profiles").join(&candidate.id).join("6.json"))?
                == *candidate,
            "acceptance_profile_identity",
        )?;
        require(
            retained.profile == *candidate
                && retained.parent.as_ref() == Some(&seal.parent)
                && retained.qualification == Some(Qualification::Ap15Editor)
                && retained.external_ids == external_ids(&p.class.class_id)?
                && retained.performance.added_frames == 512,
            "acceptance_candidate_identity",
        )?;
        m.verify_completed_publication(&retained, &seal.candidate)?;
        // Existing qualification law verifies the active physical parent and
        // all unchanged registration/environment/module/SDK constraints.
        let prior = m.verify_qualification_parent(&db, candidate, &retained.registration)?;
        require(
            prior.profile == *parent_profile
                && prior.id == seal.parent.id
                && db.classes[&p.class.class_id].managed_revision.as_ref() == Some(&seal.parent),
            "acceptance_parent_identity",
        )?;
        let exact = qualification::load(m, candidate.clone())?;
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

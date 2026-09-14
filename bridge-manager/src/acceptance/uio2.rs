//! Exact accepted candidate 17 to ordinary 18; ordinary 11 remains rollback.
use super::uir1::{normalized, prepare_review, Seal};
use super::*;
pub const REVIEW: &[u8] = include_bytes!("../../../evidence/uio2/acceptance/review.json");
const EVIDENCE: &[(&str, &[u8])] = &[
    (
        "evidence/uio2/tsg1/generated.json",
        include_bytes!("../../../evidence/uio2/tsg1/generated.json"),
    ),
    (
        "evidence/uio2/tsg1/native-moonlight-boundary.json",
        include_bytes!("../../../evidence/uio2/tsg1/native-moonlight-boundary.json"),
    ),
    (
        "evidence/uio2/tsg1/operator-resize-and-cleanup.json",
        include_bytes!("../../../evidence/uio2/tsg1/operator-resize-and-cleanup.json"),
    ),
];
const HISTORY: &[(&str, &[u8])] = &[
    (
        "compatibility/ap18/revision-11/arturia-pigments.json",
        include_bytes!("../../../compatibility/ap18/revision-11/arturia-pigments.json"),
    ),
    (
        "compatibility/uio2/arturia-pigments.json",
        include_bytes!("../../../compatibility/uio2/arturia-pigments.json"),
    ),
];
fn verify_seal(s: &Seal) -> Result<()> {
    let compiled: Seal = serde_json::from_slice(REVIEW)?;
    require(
        serde_json::to_vec(s)? == serde_json::to_vec(&compiled)?
            && s.schema == 1
            && s.review == 5192912686
            && s.head == "953e246ed2943eaafbac0d83e0dadc1d23bed69f"
            && s.tree == "d256b236b789dea6a2f1d687cb22d92a9b2054cd",
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
pub fn prepare(m: &Manager) -> Result<AcceptedSoftware> {
    let seal: Seal = serde_json::from_slice(REVIEW)?;
    verify_seal(&seal)?;
    let p = pigments_verified()?;
    let c = qualification::if1_candidate()?;
    require(
        p.revision == 18 && c.revision == 17,
        "acceptance_profile_transition",
    )?;
    normalized(&p, &c)?;
    prepare_review(m, &seal, REVIEW, &p, &c)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eighteen_is_exact_accepted_seventeen() {
        let seal: Seal = serde_json::from_slice(REVIEW).unwrap();
        verify_seal(&seal).unwrap();
        let p = pigments_verified().unwrap();
        let c = qualification::if1_candidate().unwrap();
        normalized(&p, &c).unwrap();
        assert_eq!(p.revision, 18);
        assert_eq!(c.fingerprint().unwrap(), seal.candidate_fingerprint);
        assert!(p.claim.permits(SelectionPurpose::Activation));
        assert!(!c.claim.permits(SelectionPurpose::Activation));
        assert!(installed_profiles().unwrap().contains(&p));
        for mutation in 0..5 {
            let mut q = p.clone();
            match mutation {
                0 => q.requirements.native_sha256 = "ab".repeat(32),
                1 => q.requirements.host_sha256 = "ab".repeat(32),
                2 => q.capabilities = pigments_eleven().unwrap().capabilities,
                3 => q.limitations.clear(),
                _ => q.revision = 19,
            }
            assert!(normalized(&q, &c).is_err());
        }
        let mut wrong = seal.clone();
        wrong.candidate_fingerprint = "ab".repeat(32);
        assert!(verify_seal(&wrong).is_err());
    }
}

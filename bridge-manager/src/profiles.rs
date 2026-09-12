//! Portable, closed policy. Profiles never execute work or identify local paths.
use crate::*;
use std::collections::BTreeSet;

pub const PROFILE_LIMIT: usize = 16 * 1024;
pub const PROFILE_COUNT: usize = 16;

macro_rules! closed_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
closed_enum!(Claim {
    ReviewCandidate,
    VerifiedExactFixture,
    Withdrawn
});
/// New policy selection is separate from loading exact retained history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionPurpose {
    Activation,
    Qualification,
}
impl Claim {
    pub fn permits(&self, purpose: SelectionPurpose) -> bool {
        self.require(purpose).is_ok()
    }
    pub fn require(&self, purpose: SelectionPurpose) -> Result<()> {
        match (self, purpose) {
            (Self::VerifiedExactFixture, _)
            | (Self::ReviewCandidate, SelectionPurpose::Qualification) => Ok(()),
            (Self::ReviewCandidate, SelectionPurpose::Activation) => {
                Err("profile_review_candidate_not_activatable".into())
            }
            (Self::Withdrawn, _) => Err("profile_withdrawn".into()),
        }
    }
}
closed_enum!(Role { Instrument, Effect });
closed_enum!(Family {
    ArturiaPersistentV1
});
closed_enum!(Accessibility {
    WindowsDefault,
    DisabledForVendorProcess
});
closed_enum!(Editor {
    DetachedOwnerThreadWithNativePanel,
    DetachedDirectVendorLifecycle
});
closed_enum!(State {
    ConcurrentReadOnlyCaptureV12
});
closed_enum!(Precision { Float32Only });
closed_enum!(VendorRetirement { ProcessScopedVendorRetirement });
closed_enum!(EditorLifetime { RetainEditorViewUntilInstanceRetirement });
closed_enum!(EventOutputPolicy { ReportedZeroEventChannelsUnspecified });
closed_enum!(PerformancePolicy {
    Frames512Recommended256Unqualified
});
closed_enum!(Limitation {
    ShortDeliveryGaps,
    Unqualified256,
    WindowsAccessibilityUnavailable,
    DetachedFocusRefusal,
    FragmentsAdvancedRedraw,
    ExactOperatorArtifactOnly,
    CapacityUnderQualification,
    DirectEditorUnderQualification,
    PigmentsUnderQualification,
    AuxiliaryInputInactive,
    SoleStereoAuxiliaryInputOnly,
    ReturnedResultDiagnosis
});

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vendor_retirement: Option<VendorRetirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_lifetime: Option<EditorLifetime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_output: Option<EventOutputPolicy>,
    pub accessibility: Accessibility,
    pub editor: Editor,
    pub state: State,
    pub precision: Precision,
    pub performance: PerformancePolicy,
}
impl Capabilities {
    pub fn compatibility(&self) -> Compatibility {
        Compatibility {
            disable_windows_accessibility: self.accessibility == Accessibility::DisabledForVendorProcess,
            event_output: self.event_output.clone(),
            editor_lifetime: self.editor_lifetime.clone(),
            vendor_retirement: self.vendor_retirement.clone(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunnerMatch {
    pub id: String,
    pub version: String,
    pub proton_sha256: String,
    pub entry_point_sha256: String,
    pub file_sha256: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    pub runner: RunnerMatch,
    pub environment_family: Family,
    pub environment_revision: u64,
    pub host_sha256: String,
    pub host_source_sha256: String,
    pub native_sha256: String,
    pub native_source_commit: String,
    pub descriptor_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: u32,
    pub id: String,
    pub revision: u32,
    pub claim: Claim,
    pub module_sha256: String,
    pub factory_vendor: String,
    pub class: Metadata,
    pub role: Role,
    pub requirements: Requirements,
    pub capabilities: Capabilities,
    pub limitations: Vec<Limitation>,
    pub evidence: Vec<String>,
}

pub fn role(metadata: &Metadata) -> Result<Role> {
    metadata.verify()?;
    Ok(
        if metadata.subcategories.split('|').any(|s| s == "Instrument") {
            Role::Instrument
        } else {
            Role::Effect
        },
    )
}
fn text(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}
impl Profile {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        require(bytes.len() <= PROFILE_LIMIT, "profile_size")?;
        // Fixed typed nesting plus serde's recursion bound. No flattened maps,
        // untagged alternatives, arbitrary values, hooks or unknown fields.
        let p: Self = serde_json::from_slice(bytes)?;
        p.validate()?;
        Ok(p)
    }
    pub fn validate(&self) -> Result<()> {
        require(self.schema == 1, "profile_schema")?;
        require(self.capabilities.vendor_retirement.is_none()
            || self.capabilities.editor_lifetime == Some(EditorLifetime::RetainEditorViewUntilInstanceRetirement),
            "profile_retirement_requires_retained_view")?;
        require(
            text(&self.id, 96)
                && self.id.as_bytes()[0].is_ascii_alphanumeric()
                && self
                    .id
                    .bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b".-".contains(&c))
                && self.revision > 0,
            "profile_identity",
        )?;
        self.class.verify()?;
        require(
            self.class.class_id == self.class.class_id.to_uppercase(),
            "profile_class_identity",
        )?;
        require(role(&self.class)? == self.role, "profile_role")?;
        require(text(&self.factory_vendor, 63), "profile_factory_vendor")?;
        let r = &self.requirements;
        for h in [
            &self.module_sha256,
            &r.host_sha256,
            &r.host_source_sha256,
            &r.native_sha256,
            &r.descriptor_sha256,
        ] {
            require(valid_hex(h, 64) && *h == h.to_lowercase(), "profile_hash")?;
        }
        require(
            valid_hex(&r.native_source_commit, 40) && r.environment_revision > 0,
            "profile_artifact_constraint",
        )?;
        require(
            text(&r.runner.id, 128)
                && text(&r.runner.version, 256)
                && (2..=128).contains(&r.runner.file_sha256.len()),
            "profile_runner",
        )?;
        let hashes: BTreeSet<_> = r.runner.file_sha256.iter().collect();
        require(
            hashes.len() == r.runner.file_sha256.len()
                && hashes
                    .iter()
                    .all(|h| valid_hex(h, 64) && h.as_str() == h.to_lowercase())
                && hashes.contains(&r.runner.proton_sha256)
                && hashes.contains(&r.runner.entry_point_sha256),
            "profile_runner_hashes",
        )?;
        require(
            !self.limitations.is_empty()
                && self.limitations.len() <= 16
                && self.evidence.len() <= 16,
            "profile_collection_bound",
        )?;
        for e in &self.evidence {
            require(
                text(e, 256)
                    && (e.starts_with("docs/") || e.starts_with("evidence/"))
                    && !e.contains("..")
                    && !e.contains([':', '\\']),
                "profile_evidence_reference",
            )?;
        }
        Ok(())
    }
    pub fn fingerprint(&self) -> Result<String> {
        Ok(hex(&Sha256::digest(serde_json::to_vec(self)?)))
    }
    pub fn verify_environment(&self, e: &Environment, family: &Family) -> Result<()> {
        let r = &self.requirements;
        require(
            *family == r.environment_family && e.revision == r.environment_revision,
            "environment_mismatch",
        )?;
        let actual: BTreeSet<_> = e.runner.files.iter().map(|a| a.sha256.as_str()).collect();
        let expected: BTreeSet<_> = r.runner.file_sha256.iter().map(String::as_str).collect();
        require(
            e.runner.id == r.runner.id
                && e.runner.version == r.runner.version
                && actual == expected
                && e.runner
                    .files
                    .iter()
                    .any(|a| a.path == e.runner.proton && a.sha256 == r.runner.proton_sha256)
                && e.runner.files.iter().any(|a| {
                    a.path == e.runner.entry_point && a.sha256 == r.runner.entry_point_sha256
                }),
            "runner_mismatch",
        )
    }
}

pub fn validate_set(profiles: &[Profile]) -> Result<()> {
    require(
        !profiles.is_empty() && profiles.len() <= PROFILE_COUNT,
        "profile_count",
    )?;
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    for p in profiles {
        p.validate()?;
        require(
            ids.insert((&p.id, p.revision)),
            "duplicate_profile_revision",
        )?;
        require(names.insert(&p.id), "duplicate_profile_identity")?;
    }
    Ok(())
}

pub fn installed_profiles() -> Result<Vec<Profile>> {
    let result = [
        include_bytes!("../../compatibility/arturia-pure-lofi.json").as_slice(),
        include_bytes!("../../compatibility/arturia-efx-fragments.json").as_slice(),
    ]
    .into_iter()
    .map(Profile::parse)
    .collect::<Result<Vec<_>>>()?;
    validate_set(&result)?;
    Ok(result)
}

/// Immutable AP14 history, never current ordinary selection authority.
pub fn ap14_profiles() -> Result<Vec<Profile>> {
    [
        include_bytes!("../../compatibility/ap14/revision-3/arturia-pure-lofi.json").as_slice(),
        include_bytes!("../../compatibility/ap14/revision-3/arturia-efx-fragments.json").as_slice(),
    ]
    .into_iter()
    .map(Profile::parse)
    .collect()
}

/// Immutable AP15 ordinary history and AP17 rollback parent; not new selection.
pub fn ap15_profiles() -> Result<Vec<Profile>> {
    [include_bytes!("../../compatibility/ap15/revision-7/arturia-pure-lofi.json").as_slice(),
     include_bytes!("../../compatibility/ap15/revision-7/arturia-efx-fragments.json").as_slice()]
    .into_iter().map(Profile::parse).collect()
}

/// UUIDv5 law retained from tools/ap8_descriptor.py, independent of revisions.
pub fn external_ids(class_id: &str) -> Result<[String; 2]> {
    require(valid_hex(class_id, 32), "class_identity")?;
    let namespace = [
        0x93, 0x89, 0x48, 0x0f, 0xb4, 0xb0, 0x5e, 0x02, 0xa1, 0xd7, 0x68, 0x7a, 0x57, 0xb5, 0x4b,
        0x3f,
    ];
    Ok([":processor", ":controller"].map(|suffix| {
        let mut h = sha1::Sha1::new();
        h.update(namespace);
        h.update(format!("{}{suffix}", class_id.to_uppercase()).as_bytes());
        let mut bytes: [u8; 16] = h.finalize()[..16].try_into().unwrap();
        bytes[6] = (bytes[6] & 0x0f) | 0x50;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        hex(&bytes)
    }))
}

#[cfg(test)]
mod event_policy_tests {
    use super::*;
    #[test]
    fn policy_is_closed_optional_and_preserves_old_fingerprints() {
        let original=Profile::parse(include_bytes!("../../compatibility/ap18/revision-5/arturia-pigments.json")).unwrap();
        let bytes=serde_json::to_vec(&original).unwrap();
        assert!(!String::from_utf8(bytes).unwrap().contains("event_output"));
        assert_eq!(original.fingerprint().unwrap(),"5775b0f11dc60fa3d14da31edc35354a1445013dac56456685fb2bebf0e261b4");
        let mut corrected=original.clone();
        corrected.capabilities.event_output=Some(EventOutputPolicy::ReportedZeroEventChannelsUnspecified);
        corrected.capabilities.editor_lifetime=Some(EditorLifetime::RetainEditorViewUntilInstanceRetirement);
        corrected.capabilities.vendor_retirement=Some(VendorRetirement::ProcessScopedVendorRetirement);
        assert_eq!(corrected.capabilities.compatibility().vendor_retirement,corrected.capabilities.vendor_retirement);
        let mut wrong=corrected.clone();wrong.capabilities.editor_lifetime=None;
        assert!(wrong.validate().is_err());
        assert_eq!(corrected.capabilities.compatibility().editor_lifetime,corrected.capabilities.editor_lifetime);
        assert_eq!(corrected.capabilities.compatibility().event_output,corrected.capabilities.event_output);
        assert!(Profile::parse(&serde_json::to_vec(&corrected).unwrap()).is_ok());
        assert!(corrected.claim.require(SelectionPurpose::Activation).is_err());
        let mut bad=serde_json::to_value(&corrected).unwrap();bad["capabilities"]["editor_lifetime"]=serde_json::json!("retain_everything");
        assert!(Profile::parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        let mut value=serde_json::to_value(corrected).unwrap();value["capabilities"]["event_output"]=serde_json::json!("all_zero_buses");
        assert!(Profile::parse(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

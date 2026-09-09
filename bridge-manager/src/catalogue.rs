//! Exact installed native artifacts, separate from portable compatibility policy.
use crate::{profiles::*, *};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeArtifact {
    pub class: Metadata,
    pub module_sha256: String,
    pub artifact: Artifact,
    pub source_commit: String,
    pub descriptor_sha256: String,
    pub external_ids: [String; 2],
}
impl NativeArtifact {
    pub fn matches(&self, p: &Profile) -> Result<()> {
        require(
            self.class == p.class
                && self.module_sha256 == p.module_sha256
                && self.artifact.sha256 == p.requirements.native_sha256
                && self.source_commit == p.requirements.native_source_commit
                && self.descriptor_sha256 == p.requirements.descriptor_sha256
                && self.external_ids == external_ids(&p.class.class_id)?,
            "native_artifact_mismatch",
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentBinding {
    pub family: Family,
    pub environment: Environment,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub schema: u32,
    pub natives: Vec<NativeArtifact>,
    pub environments: Vec<EnvironmentBinding>,
}
impl Catalogue {
    pub fn validate(&self, root: &Path) -> Result<()> {
        require(
            self.schema == 1
                && !self.natives.is_empty()
                && self.natives.len() <= PROFILE_COUNT
                && !self.environments.is_empty()
                && self.environments.len() <= 16,
            "catalogue_schema_or_bound",
        )?;
        let mut classes = std::collections::BTreeSet::new();
        for n in &self.natives {
            n.class.verify()?;
            require(
                classes.insert(&n.class.class_id)
                    && n.external_ids == external_ids(&n.class.class_id)?
                    && n.artifact.path.starts_with(root.join("software"))
                    && n.artifact.path.canonicalize()? == n.artifact.path,
                "catalogue_identity",
            )?;
            n.artifact.verify()?;
        }
        let mut environments = std::collections::BTreeSet::new();
        for e in &self.environments {
            require(
                environments.insert(&e.environment.id),
                "duplicate_environment_binding",
            )?;
        }
        Ok(())
    }
    pub fn native(&self, p: &Profile) -> Result<&NativeArtifact> {
        let n = self
            .natives
            .iter()
            .find(|n| n.class.class_id == p.class.class_id)
            .ok_or("native_artifact_absent")?;
        n.matches(p)?;
        n.artifact.verify()?;
        Ok(n)
    }
}

/// AP14 adopts only already managed, verified generated artifacts. Setup is an
/// inactive product operation; neither playback nor managed publication needs
/// the original generator checkout/build path after this copy.
pub fn adoption(m: &Manager, profiles: &[Profile]) -> Result<Catalogue> {
    validate_set(profiles)?;
    let db = m.registry()?;
    let mut natives = Vec::new();
    let mut environments: Vec<EnvironmentBinding> = Vec::new();
    for p in profiles.iter().filter(|p| p.claim != Claim::Withdrawn) {
        let e = db
            .classes
            .get(&p.class.class_id)
            .ok_or("adoption_requires_existing_managed_artifact")?;
        let r = &e.registration;
        r.verify(&m.root)?;
        require(
            r.native.path.starts_with(m.root.join("publications"))
                && r.metadata == p.class
                && r.module.sha256 == p.module_sha256
                && r.compatibility == p.capabilities.compatibility(),
            "adoption_binding_mismatch",
        )?;
        // Native adoption validates the native/module/environment identity.
        // A candidate Windows host is verified independently by scan/matching;
        // requiring the prior registration to already use it prevents an
        // explicit host/profile update while the known-good target stays active.
        p.verify_environment(&r.environment, &p.requirements.environment_family)?;
        let n = NativeArtifact {
            class: r.metadata.clone(),
            module_sha256: r.module.sha256.clone(),
            artifact: r.native.clone(),
            source_commit: p.requirements.native_source_commit.clone(),
            descriptor_sha256: p.requirements.descriptor_sha256.clone(),
            external_ids: external_ids(&r.key())?,
        };
        n.matches(p)?;
        natives.push(n);
        let binding = EnvironmentBinding {
            family: p.requirements.environment_family.clone(),
            environment: r.environment.clone(),
        };
        if let Some(old) = environments
            .iter()
            .find(|e| e.environment.id == binding.environment.id)
        {
            require(*old == binding, "environment_family_conflict")?;
        } else {
            environments.push(binding);
        }
    }
    Ok(Catalogue {
        schema: 1,
        natives,
        environments,
    })
}

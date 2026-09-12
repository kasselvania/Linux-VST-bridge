//! Exact installed native artifacts, separate from portable compatibility policy.
use crate::{profiles::*, *};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostArtifact {
    pub host: Artifact,
    pub source_manifest: Artifact,
}
impl HostArtifact {
    /// One digest for the pair avoids two SHA-256 path components exceeding
    /// the pinned Windows launcher's executable-path extent.
    pub fn directory_name(&self) -> String {
        hex(&sha2::Sha256::digest(
            format!("{}{}", self.host.sha256, self.source_manifest.sha256).as_bytes(),
        ))
    }
    fn matches(&self, p: &Profile) -> bool {
        self.host.sha256 == p.requirements.host_sha256
            && self.source_manifest.sha256 == p.requirements.host_source_sha256
    }
}
pub fn verify_host_path(path: &Path) -> Result<()> {
    // The supervisor renders product-software paths as Z:\... . The leading
    // Linux slash becomes that drive root, adding two UTF-16 code units.
    require(
        path.to_str()
            .is_some_and(|p| p.encode_utf16().count() + 2 < 260),
        "host_executable_path_extent",
    )
}

/// Existing immutable software record, shared by setup and ordinary admission.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Software {
    pub manager: Artifact,
    pub supervisor: Artifact,
    pub ownership: Artifact,
    pub host: Artifact,
    pub source_manifest: Artifact,
    pub source_sha256: String,
    #[serde(default)]
    pub native_catalogue: Option<Artifact>,
}
impl Software {
    pub fn catalogue(&self, m: &Manager) -> Result<Catalogue> {
        let a = self
            .native_catalogue
            .as_ref()
            .ok_or("native_catalogue_absent_run_product_setup")?;
        require(
            a.path.parent() == self.manager.path.parent(),
            "catalogue_software_binding",
        )?;
        a.verify()?;
        require(
            file(&a.path)?.metadata()?.len() <= 512 * 1024,
            "catalogue_size",
        )?;
        let c: Catalogue = read_json(&a.path)?;
        c.validate(&m.root)?;
        Ok(c)
    }
}

/// The default host remains the keeper/legacy-product host. An additional
/// exact profile host is authority only through the current immutable software
/// catalogue. Retained engineering packages alone cannot select it.
pub fn current_host(
    m: &Manager,
    installed: &Artifact,
    source: &str,
    p: &Profile,
) -> Result<HostArtifact> {
    if p.requirements.host_sha256 == installed.sha256 && p.requirements.host_source_sha256 == source
    {
        installed.verify()?;
        let manifest = Artifact {
            path: installed.path.with_file_name("host-source-manifest.json"),
            sha256: source.into(),
        };
        manifest.verify()?;
        return Ok(HostArtifact {
            host: installed.clone(),
            source_manifest: manifest,
        });
    }
    let sw: Software =
        read_json(&m.root.join("software.json")).map_err(|_| "installed_host_mismatch")?;
    require(
        sw.host == *installed && sw.source_sha256 == source && sw.source_manifest.sha256 == source,
        "installed_host_mismatch",
    )?;
    sw.host.verify()?;
    sw.source_manifest.verify()?;
    let c = sw.catalogue(m)?;
    c.native(p)?;
    c.host(p, installed, source)
}

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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub schema: u32,
    pub natives: Vec<NativeArtifact>,
    pub environments: Vec<EnvironmentBinding>,
    // Version 1 has no supplemental hosts. Version 2 binds them to exact
    // profile requirements; it never changes the legacy default host.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hosts: Vec<HostArtifact>,
}
impl Catalogue {
    pub fn validate(&self, root: &Path) -> Result<()> {
        require(
            ((self.schema == 1 && self.hosts.is_empty()) || self.schema == 2)
                && !self.natives.is_empty()
                && self.natives.len() <= PROFILE_COUNT
                && self.hosts.len() <= PROFILE_COUNT
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
        let mut hosts = std::collections::BTreeSet::new();
        for h in &self.hosts {
            require(
                hosts.insert((&h.host.sha256, &h.source_manifest.sha256))
                    && h.source_manifest.path
                        == h.host.path.with_file_name("host-source-manifest.json"),
                "catalogue_host_identity",
            )?;
            for a in [&h.host, &h.source_manifest] {
                require(
                    a.path.starts_with(root.join("software"))
                        && a.path.canonicalize()? == a.path
                        && file(&a.path)?.metadata()?.mode() & 0o222 == 0,
                    "catalogue_host_identity",
                )?;
                a.verify()?;
            }
        }
        Ok(())
    }
    pub fn host(&self, p: &Profile, default: &Artifact, source: &str) -> Result<HostArtifact> {
        let main = HostArtifact {
            host: default.clone(),
            source_manifest: Artifact {
                path: default.path.with_file_name("host-source-manifest.json"),
                sha256: source.into(),
            },
        };
        let matches: Vec<_> = self.hosts.iter().filter(|h| h.matches(p)).collect();
        require(
            matches.len() <= 1 && (!main.matches(p) || matches.is_empty()),
            "installed_host_mismatch",
        )?;
        let selected = if main.matches(p) {
            main
        } else {
            matches
                .first()
                .ok_or("installed_host_mismatch")?
                .to_owned()
                .clone()
        };
        selected.host.verify()?;
        selected.source_manifest.verify()?;
        Ok(selected)
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
    for p in profiles {
        p.claim.require(SelectionPurpose::Activation)?;
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
    let hosts = if m.root.join("software.json").try_exists()? {
        let sw: Software = read_json(&m.root.join("software.json"))?;
        if sw.native_catalogue.is_some() {
            sw.catalogue(m)?.hosts
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    Ok(Catalogue {
        schema: if hosts.is_empty() { 1 } else { 2 },
        natives,
        environments,
        hosts,
    })
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn supplemental_host_pair_has_one_digest_and_launch_extent_is_checked() {
        let mut h = HostArtifact {
            host: Artifact {
                path: "host.exe".into(),
                sha256: "ab".repeat(32),
            },
            source_manifest: Artifact {
                path: "manifest.json".into(),
                sha256: "cd".repeat(32),
            },
        };
        let key = h.directory_name();
        assert!(valid_hex(&key, 64));
        assert_eq!(key, h.directory_name());
        h.source_manifest.sha256 = "ef".repeat(32);
        assert_ne!(key, h.directory_name());
        h.source_manifest.sha256 = "cd".repeat(32);
        h.host.sha256 = "ef".repeat(32);
        assert_ne!(key, h.directory_name());
        assert!(verify_host_path(Path::new(&"x".repeat(257))).is_ok());
        assert!(verify_host_path(Path::new(&"x".repeat(258))).is_err());
        assert!(verify_host_path(Path::new(&"x".repeat(267))).is_err());
        assert!(verify_host_path(Path::new(&"😀".repeat(129))).is_err());
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installer_launch: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preparation_kit: Option<Artifact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator_frontend: Option<Artifact>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub onboarding_runtime: Option<OnboardingRuntimePolicy>,
}
pub const STANDARD_ONBOARDING_RUNNER: &str = "proton-11.0-2c-25118279-slr4-4.0.20260805.254769";
/// The serialized runner identity used for exact catalogue selection.
pub fn runner_key(runner: &Runner) -> Result<String> {
    Ok(hex(&sha2::Sha256::digest(serde_json::to_vec(runner)?)))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OnboardingRuntimePolicy {
    pub schema: u32,
    pub default_runner_key: String,
    pub display_name: String,
    pub posture: String,
}
impl OnboardingRuntimePolicy {
    pub fn from_environments(environments: &[EnvironmentBinding]) -> Result<Option<Self>> {
        let mut selected = None;
        for binding in environments {
            let runner = &binding.environment.runner;
            if runner.id == STANDARD_ONBOARDING_RUNNER && runner.policy.is_none() {
                let key = runner_key(runner)?;
                if let Some(previous) = &selected {
                    require(previous == &key, "onboarding_standard_runner_ambiguous")?;
                }
                selected = Some(key);
            }
        }
        Ok(selected.map(|default_runner_key| Self { schema: 1, default_runner_key,
            display_name: "Standard".into(), posture: "recommended".into() }))
    }
}
impl Catalogue {
    pub fn validate(&self, root: &Path) -> Result<()> {
        require(
            ((self.schema == 1 && self.hosts.is_empty()) || matches!(self.schema, 2..=4))
                && !self.natives.is_empty()
                && self.natives.len() <= PROFILE_COUNT
                && self.hosts.len() <= PROFILE_COUNT
                && !self.environments.is_empty()
                && self.environments.len() <= 16,
            "catalogue_schema_or_bound",
        )?;
        // Schemas 3 and 4 retain multiple immutable native builds per class for rollback.
        // Older schemas preserve their single-class-image uniqueness law.
        let mut classes = std::collections::BTreeSet::new();
        for n in &self.natives {
            n.class.verify()?;
            require(
                classes.insert((
                    &n.class.class_id,
                    if self.schema >= 3 {
                        n.artifact.sha256.as_str()
                    } else {
                        ""
                    },
                )) && n.external_ids == external_ids(&n.class.class_id)?
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
        require((self.schema >= 3 || self.onboarding_runtime.is_none())
            && (self.schema != 4 || self.onboarding_runtime.is_some()),
            "onboarding_runtime_catalogue_schema")?;
        if let Some(policy) = &self.onboarding_runtime {
            require(policy.schema == 1 && policy.display_name == "Standard"
                && policy.posture == "recommended"
                && valid_hex(&policy.default_runner_key, 64),
                "onboarding_runtime_policy_binding")?;
            let mut standard = std::collections::BTreeSet::new();
            for binding in &self.environments {
                let runner = &binding.environment.runner;
                if runner.id == STANDARD_ONBOARDING_RUNNER && runner.policy.is_none() {
                    runner.verify()?;
                    standard.insert(runner_key(runner)?);
                }
            }
            require(standard.len() == 1
                && standard.contains(&policy.default_runner_key),
                "onboarding_runtime_policy_binding")?;
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
            .find(|n| n.matches(p).is_ok())
            .ok_or("native_artifact_absent")?;
        n.matches(p)?;
        n.artifact.verify()?;
        Ok(n)
    }
}

/// AP14 adopts only already managed, verified generated artifacts. Setup is an
/// inactive product operation; neither playback nor managed publication needs
/// the original generator checkout/build path after this copy.
pub fn setup_adoption(m: &Manager, profiles: &[Profile]) -> Result<Option<Catalogue>> {
    let registry = m.registry()?;
    if crate::frg1::catalogue_free_registry(m, &registry)? {
        return Ok(None);
    }
    Ok(Some(adoption(m, profiles)?))
}

/// Keep an exact registered host needed by a current profile when setup changes
/// the default host. A retained path alone cannot authorize another profile.
pub fn setup_adoption_for_host(m: &Manager, profiles: &[Profile], default_host: &str, default_source: &str) -> Result<Option<Catalogue>> {
    let Some(mut catalogue) = setup_adoption(m, profiles)? else { return Ok(None) };
    let db = m.registry()?;
    catalogue.hosts.retain(|h| h.host.sha256 != default_host || h.source_manifest.sha256 != default_source);
    for profile in profiles {
        let registration = &db.classes.get(&profile.class.class_id)
            .ok_or("adoption_requires_existing_managed_artifact")?.registration;
        if registration.host.sha256 != profile.requirements.host_sha256
            || registration.host_source_sha256 != profile.requirements.host_source_sha256
            || (registration.host.sha256 == default_host && registration.host_source_sha256 == default_source) {
            continue;
        }
        let retained = HostArtifact {
            host: registration.host.clone(),
            source_manifest: Artifact {
                path: registration.host.path.with_file_name("host-source-manifest.json"),
                sha256: registration.host_source_sha256.clone(),
            },
        };
        for artifact in [&retained.host, &retained.source_manifest] {
            require(artifact.path.starts_with(m.root.join("software"))
                && artifact.path.canonicalize()? == artifact.path
                && file(&artifact.path)?.metadata()?.mode() & 0o222 == 0,
                "catalogue_host_identity")?;
            artifact.verify()?;
        }
        if !catalogue.hosts.iter().any(|h| h.host.sha256 == retained.host.sha256
            && h.source_manifest.sha256 == retained.source_manifest.sha256) {
            catalogue.hosts.push(retained);
        }
    }
    if catalogue.schema == 1 && !catalogue.hosts.is_empty() { catalogue.schema = 2; }
    Ok(Some(catalogue))
}

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
    // Retain accepted rollback ancestry even when it uses an older native image.
    // Reconstruct from exact immutable publication records, not current filenames.
    for entry in db.classes.values() {
        let mut reference = entry.managed_revision.clone();
        let mut visited = std::collections::BTreeSet::new();
        while let Some(current) = reference {
            require(visited.len() < 256 && visited.insert(current.id.clone()), "adoption_history_bound")?;
            let revision = m.load_revision(&entry.registration.key(), &current)?;
            if !revision.adopted_legacy { m.verify_completed_publication(&revision, &current)?; }
            if revision.qualification.is_none() && revision.profile.claim == Claim::VerifiedExactFixture {
                let p = &revision.profile;
                let r = &revision.registration;
                let n = NativeArtifact {class:r.metadata.clone(),module_sha256:r.module.sha256.clone(),artifact:r.native.clone(),source_commit:p.requirements.native_source_commit.clone(),descriptor_sha256:p.requirements.descriptor_sha256.clone(),external_ids:external_ids(&r.key())?};
                n.artifact.verify()?;
                n.matches(p)?;
                retain_native(&mut natives,n)?;
            }
            reference = revision.parent;
        }
    }
    let mut schema = 1;
    let hosts = if m.root.join("software.json").try_exists()? {
        let sw: Software = read_json(&m.root.join("software.json"))?;
        if sw.native_catalogue.is_some() {
            let old = sw.catalogue(m)?;
            schema = old.schema;
            for n in old.natives { retain_native(&mut natives,n)?; }
            old.hosts
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let onboarding_runtime = OnboardingRuntimePolicy::from_environments(&environments)?;
    Ok(Catalogue {
        schema: adopted_schema(schema, &natives, &hosts, &onboarding_runtime),
        natives,
        environments,
        hosts,
        onboarding_runtime,
    })
}

fn adopted_schema(previous: u32, natives: &[NativeArtifact], hosts: &[HostArtifact],
    runtime: &Option<OnboardingRuntimePolicy>) -> u32 {
    if runtime.is_some() { 4 }
    else if previous >= 3 || natives.iter().map(|n| &n.class.class_id)
        .collect::<std::collections::BTreeSet<_>>().len() != natives.len() { 3 }
    else if hosts.is_empty() { 1 } else { 2 }
}

fn retain_native(natives: &mut Vec<NativeArtifact>, n: NativeArtifact) -> Result<()> {
    if let Some(old) = natives.iter().find(|v|v.class.class_id == n.class.class_id && v.artifact.sha256 == n.artifact.sha256) {
        let mut same = n;
        same.artifact.path = old.artifact.path.clone();
        require(&same == old, "adoption_native_metadata_conflict")?;
    } else {
        require(natives.len() < PROFILE_COUNT, "adoption_native_bound")?;
        natives.push(n);
    }
    Ok(())
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn setup_retains_exact_registered_host_when_default_changes() {
        let (f, p, c, n) = crate::test_fixture::prepared();
        let report = crate::observation::derive(&p, &c, &n).unwrap();
        f.m.managed_publish(&p, &c, report, &c.host, &c.host_source_sha256, None).unwrap();
        for path in [f.r.host.path.clone(), f.r.host.path.with_file_name("host-source-manifest.json")] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
        }
        let profiles = std::slice::from_ref(&p);
        let retained = setup_adoption_for_host(&f.m, profiles, &"cd".repeat(32), &"ef".repeat(32)).unwrap().unwrap();
        assert_eq!(retained.schema, 2);
        assert_eq!(retained.hosts.len(), 1);
        assert_eq!(retained.hosts[0].host, f.r.host);
        assert_eq!(retained.hosts[0].source_manifest.sha256, f.r.host_source_sha256);
        retained.host(&p, &Artifact { path: f.outer.join("new-host"), sha256: "cd".repeat(32) }, &"ef".repeat(32)).unwrap();

        let same_default = setup_adoption_for_host(&f.m, profiles, &f.r.host.sha256, &f.r.host_source_sha256).unwrap().unwrap();
        assert!(same_default.hosts.is_empty());
        let mut changed_profile = p.clone();
        changed_profile.requirements.host_sha256 = "cd".repeat(32);
        changed_profile.requirements.host_source_sha256 = "ef".repeat(32);
        let changed = setup_adoption_for_host(&f.m, std::slice::from_ref(&changed_profile), &"cd".repeat(32), &"ef".repeat(32)).unwrap().unwrap();
        assert!(changed.hosts.is_empty());

        fs::set_permissions(&f.r.host.path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&f.r.host.path, b"changed host").unwrap();
        fs::set_permissions(&f.r.host.path, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(setup_adoption_for_host(&f.m, profiles, &"cd".repeat(32), &"ef".repeat(32)).is_err());
    }
    #[test]
    fn setup_retains_ordinary_rollback_native_after_native_update() {
        let (f,p,c,n) = crate::test_fixture::prepared();
        let r = crate::observation::derive(&p,&c,&n).unwrap();
        let first = f.m.managed_publish(&p,&c,r,&c.host,&c.host_source_sha256,None).unwrap();
        let mut updated = p.clone(); updated.revision += 1;
        let mut native = n.clone(); native.artifact.path = f.m.root.join("software/new-native.so");
        fs::write(&native.artifact.path,b"new native image").unwrap();
        native.artifact.sha256 = digest(&native.artifact.path).unwrap();
        updated.requirements.native_sha256 = native.artifact.sha256.clone();
        let r = crate::observation::derive(&updated,&c,&native).unwrap();
        f.m.managed_publish(&updated,&c,r,&c.host,&c.host_source_sha256,None).unwrap();
        let adopted = adoption(&f.m,std::slice::from_ref(&updated)).unwrap();
        assert_eq!(adopted.schema,3);
        assert_eq!(adopted.natives.len(),2);
        assert_eq!(adopted.native(&p).unwrap().artifact.sha256,n.artifact.sha256);
        assert_eq!(adopted.native(&updated).unwrap().artifact.sha256,native.artifact.sha256);
        f.m.rollback(&p.class.class_id,&first.id,None).unwrap();
        assert_eq!(f.m.registry().unwrap().classes[&p.class.class_id].registration.native.sha256,n.artifact.sha256);
    }
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

#[cfg(test)]
mod ui1_format_tests {
    use super::*;

    fn standard_catalogue() -> (crate::test_fixture::Fixture, Catalogue) {
        let (f, p, _, native) = crate::test_fixture::prepared();
        let mut environment = f.r.environment.clone();
        environment.id = "ab".repeat(16);
        environment.root = f.m.root.join("environments").join(&environment.id);
        environment.runner.id = STANDARD_ONBOARDING_RUNNER.into();
        environment.runner.policy = None;
        let environments = vec![EnvironmentBinding {
            family: p.requirements.environment_family, environment,
        }];
        let policy = OnboardingRuntimePolicy::from_environments(&environments).unwrap();
        (f, Catalogue { schema: 4, natives: vec![native], environments,
            hosts: vec![], onboarding_runtime: policy })
    }

    #[test]
    fn installed_standard_runtime_field_round_trips_without_rewriting_legacy_catalogues() {
        let old = serde_json::json!({"schema":3,"natives":[],"environments":[]});
        let legacy: Catalogue = serde_json::from_value(old.clone()).unwrap();
        assert!(legacy.onboarding_runtime.is_none());
        assert_eq!(serde_json::to_value(&legacy).unwrap(), old);

        let mut installed = old;
        installed["onboarding_runtime"] = serde_json::json!({
            "schema":1,
            "default_runner_key":"72d6da6c18cd30cb9027952cbbcc7ca2e7753c86933e310a7c8db32d8261be63",
            "display_name":"Standard",
            "posture":"recommended"
        });
        let current: Catalogue = serde_json::from_value(installed.clone()).unwrap();
        assert_eq!(serde_json::to_value(&current).unwrap(), installed);
        assert!(current.onboarding_runtime.is_some());
    }

    #[test]
    fn schema_four_writer_and_historical_schema_three_runtime_are_exact() {
        let (f, current) = standard_catalogue();
        current.validate(&f.m.root).unwrap();
        assert_eq!(adopted_schema(3, &current.natives, &current.hosts,
            &current.onboarding_runtime), 4);
        let mut historical = current.clone();
        historical.schema = 3;
        let bytes = serde_json::to_vec(&historical).unwrap();
        let loaded: Catalogue = serde_json::from_slice(&bytes).unwrap();
        loaded.validate(&f.m.root).unwrap();
        assert_eq!(serde_json::to_vec(&loaded).unwrap(), bytes);
        assert_eq!(loaded.schema, 3);
        historical.onboarding_runtime = None;
        let bytes = serde_json::to_vec(&historical).unwrap();
        let loaded: Catalogue = serde_json::from_slice(&bytes).unwrap();
        loaded.validate(&f.m.root).unwrap();
        assert_eq!(serde_json::to_vec(&loaded).unwrap(), bytes);
        assert_eq!(loaded.schema, 3);
    }

    #[test]
    fn adoption_with_one_exact_standard_runner_writes_schema_four() {
        let (f, mut profile, _, _) = crate::test_fixture::prepared();
        let mut registry = f.m.registry().unwrap();
        let entry = registry.classes.get_mut(&profile.class.class_id).unwrap();
        entry.registration.environment.runner.id = STANDARD_ONBOARDING_RUNNER.into();
        let environment = entry.registration.environment.clone();
        profile.requirements.runner.id = STANDARD_ONBOARDING_RUNNER.into();
        atomic_json(&environment.root.join("environment.json"), &environment).unwrap();
        atomic_json(&f.m.root.join("registry.json"), &registry).unwrap();
        let adopted = adoption(&f.m, &[profile]).unwrap();
        assert_eq!(adopted.schema, 4);
        assert_eq!(adopted.onboarding_runtime.as_ref().unwrap().default_runner_key,
            runner_key(&environment.runner).unwrap());
        // setup_install copies the native artifact into immutable software
        // custody before validating the final installed catalogue.
    }

    #[test]
    fn runtime_policy_requires_one_exact_verified_standard_runner() {
        let (f, valid) = standard_catalogue();
        valid.validate(&f.m.root).unwrap();
        let mut changed = valid.clone();
        changed.onboarding_runtime.as_mut().unwrap().default_runner_key = "ff".repeat(32);
        assert!(changed.validate(&f.m.root).is_err());
        let mut changed = valid.clone();
        let mut specialty = changed.environments[0].clone();
        specialty.environment.id = "cd".repeat(16);
        specialty.environment.runner.id = "specialty".into();
        specialty.environment.runner.policy = Some(RunnerPolicy::X11TouchRoutingV2);
        changed.onboarding_runtime.as_mut().unwrap().default_runner_key =
            runner_key(&specialty.environment.runner).unwrap();
        changed.environments.push(specialty);
        assert!(changed.validate(&f.m.root).is_err());
        let mut changed = valid.clone();
        changed.environments[0].environment.runner.id = "other".into();
        assert!(changed.validate(&f.m.root).is_err());
        let mut changed = valid.clone();
        let mut conflicting = changed.environments[0].clone();
        conflicting.environment.id = "ef".repeat(16);
        conflicting.environment.runner.version = "conflicting-standard-version".into();
        changed.environments.push(conflicting);
        assert!(changed.validate(&f.m.root).is_err());
        let mut changed = valid.clone();
        changed.environments[0].environment.runner.files[0].sha256 = "aa".repeat(32);
        assert!(changed.validate(&f.m.root).is_err());
        for (field, replacement) in [("display_name", "Other"), ("posture", "optional")] {
            let mut changed = valid.clone();
            let policy = changed.onboarding_runtime.as_mut().unwrap();
            if field == "display_name" { policy.display_name = replacement.into(); }
            else { policy.posture = replacement.into(); }
            assert!(changed.validate(&f.m.root).is_err());
        }
        let mut changed = valid.clone();
        changed.onboarding_runtime.as_mut().unwrap().schema = 2;
        assert!(changed.validate(&f.m.root).is_err());
        let mut changed = valid;
        changed.onboarding_runtime = None;
        assert!(changed.validate(&f.m.root).is_err());
    }
}

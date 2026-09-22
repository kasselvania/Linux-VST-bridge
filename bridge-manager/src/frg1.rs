//! Sealed Ubuntu FRG1 qualification. The compiled revision-11 candidate is the
//! only policy authority; operator inputs identify an exact environment and an
//! artifact package, never a profile or registration.
use crate::{
    catalogue::{EnvironmentBinding, Software},
    inventory,
    observation::Census,
    profiles::*,
    publication::*,
    qualification::InstalledCandidate,
    *,
};

const OWNER_SHA256: &str = "6626531f7466ccff1be37f93305ebf807e0f0965cd2d4241b175bdd60ef9da31";
const MODULE_RELATIVE: &str = "pfx/drive_c/Program Files/Common Files/VST3/Efx FRAGMENTS.vst3";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PackageManifest {
    schema: u32,
    profile_fingerprint: String,
    class_id: String,
    module_sha256: String,
    host_sha256: String,
    host_source_sha256: String,
    native_sha256: String,
    native_source_commit: String,
    descriptor_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Adoption {
    schema: u32,
    profile_fingerprint: String,
    family: Family,
    environment: Environment,
    module: Artifact,
    owner_marker: Artifact,
    prior_inventory_sha256: Option<String>,
    adopted_at: u64,
}

#[cfg(test)]
#[derive(Clone)]
struct TestContract {
    profile: Profile,
    owner_sha256: String,
    module_relative: String,
}
#[cfg(test)]
thread_local! {
    static TEST_CONTRACT: std::cell::RefCell<Option<TestContract>> = const { std::cell::RefCell::new(None) };
}

pub fn candidate() -> Result<Profile> {
    #[cfg(test)]
    if let Some(contract) = TEST_CONTRACT.with(|slot| slot.borrow().clone()) {
        return Ok(contract.profile);
    }
    let profile = crate::profiles::frg1_candidate()?;
    require(
        profile.id == "arturia-efx-fragments"
            && profile.revision == 11
            && profile.claim == Claim::ReviewCandidate
            && profile.requirements.environment_family == Family::ArturiaPersistentV1
            && profile.requirements.environment_revision == 2,
        "frg1_candidate_contract",
    )?;
    Ok(profile)
}

fn owner_sha256() -> String {
    #[cfg(test)]
    if let Some(contract) = TEST_CONTRACT.with(|slot| slot.borrow().clone()) {
        return contract.owner_sha256;
    }
    OWNER_SHA256.into()
}

fn module_relative() -> String {
    #[cfg(test)]
    if let Some(contract) = TEST_CONTRACT.with(|slot| slot.borrow().clone()) {
        return contract.module_relative;
    }
    MODULE_RELATIVE.into()
}

fn package_manifest(profile: &Profile) -> Result<PackageManifest> {
    let expected = PackageManifest {
        schema: 1,
        profile_fingerprint: profile.fingerprint()?,
        class_id: profile.class.class_id.clone(),
        module_sha256: profile.module_sha256.clone(),
        host_sha256: profile.requirements.host_sha256.clone(),
        host_source_sha256: profile.requirements.host_source_sha256.clone(),
        native_sha256: profile.requirements.native_sha256.clone(),
        native_source_commit: profile.requirements.native_source_commit.clone(),
        descriptor_sha256: profile.requirements.descriptor_sha256.clone(),
    };
    let sealed: PackageManifest = serde_json::from_slice(include_bytes!(
        "../../compatibility/frg1/qualification-package.json"
    ))?;
    if profile == &crate::profiles::frg1_candidate()? {
        require(sealed == expected, "frg1_package_manifest")?;
        return Ok(sealed);
    }
    #[cfg(test)]
    return Ok(expected);
    #[cfg(not(test))]
    Err("frg1_candidate_contract".into())
}

fn directory(m: &Manager, profile: &Profile) -> Result<PathBuf> {
    Ok(m.root
        .join("software/frg1-qualification")
        .join(profile.fingerprint()?))
}

fn adoption_path(m: &Manager) -> PathBuf {
    m.root.join("qualifications/frg1/adoption.json")
}

fn environment_record(m: &Manager, id: &str) -> Result<Environment> {
    require(
        !id.is_empty()
            && id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b'-'),
        "frg1_environment_id",
    )?;
    let root = m.root.join("environments").join(id);
    let environment: Environment = read_json(&root.join("environment.json"))?;
    require(
        environment.id == id && environment.root == root && root.canonicalize()? == root,
        "frg1_environment_identity",
    )?;
    environment.runner.verify()?;
    Ok(environment)
}

fn exact_environment(m: &Manager, id: &str) -> Result<(Environment, Artifact, Artifact)> {
    let profile = candidate()?;
    let environment = environment_record(m, id)?;
    profile.verify_environment(&environment, &Family::ArturiaPersistentV1)?;
    let relative = Path::new(&module_relative()).to_path_buf();
    require(
        relative
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_))),
        "frg1_module_relative",
    )?;
    let module = Artifact {
        path: environment.root.join("compatdata").join(relative),
        sha256: profile.module_sha256.clone(),
    };
    require(
        module
            .path
            .starts_with(environment.root.join("compatdata/pfx/drive_c"))
            && module.path.canonicalize()? == module.path,
        "frg1_module_location",
    )?;
    module.verify()?;
    let owner_marker = Artifact {
        path: environment.root.join("compatdata/.ua1-owner.json"),
        sha256: owner_sha256(),
    };
    let metadata = fs::symlink_metadata(&owner_marker.path)?;
    require(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "frg1_owner_marker",
    )?;
    owner_marker.verify()?;
    Ok((environment, module, owner_marker))
}

fn prior_inventory(m: &Manager, environment: &Environment) -> Result<Option<String>> {
    let path = m
        .root
        .join("inventory")
        .join(format!("{}.json", environment.id));
    if path.try_exists()? {
        Ok(Some(digest(&path)?))
    } else {
        Ok(None)
    }
}

fn verify_adoption(m: &Manager, adoption: &Adoption) -> Result<()> {
    let profile = candidate()?;
    let (environment, module, owner_marker) = exact_environment(m, &adoption.environment.id)?;
    require(
        adoption.schema == 1
            && adoption.profile_fingerprint == profile.fingerprint()?
            && adoption.family == Family::ArturiaPersistentV1
            && adoption.environment == environment
            && adoption.module == module
            && adoption.owner_marker == owner_marker
            && adoption.adopted_at > 0
            && adoption
                .prior_inventory_sha256
                .as_ref()
                .is_none_or(|sha| valid_hex(sha, 64)),
        "frg1_adoption_identity",
    )
}

fn adoption(m: &Manager) -> Result<Adoption> {
    let adoption: Adoption = read_json(&adoption_path(m))?;
    verify_adoption(m, &adoption)?;
    Ok(adoption)
}

/// Create a product-owned custody crossing without altering the Ubuntu-lab
/// marker or vendor prefix bytes. The environment id selects only a canonical
/// manager record; all compatibility authority is compiled above.
pub fn adopt(m: &Manager, environment_id: &str) -> Result<()> {
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let profile = candidate()?;
    require(
        !m.registry()?.classes.contains_key(&profile.class.class_id)
            && !m.publication_pending(&profile.class.class_id)?
            && physical(&m.link(&profile.class.class_id))?.is_none(),
        "frg1_local_predecessor_present",
    )?;
    if adoption_path(m).try_exists()? {
        let existing = adoption(m)?;
        return require(
            existing.environment.id == environment_id,
            "frg1_adoption_conflict",
        );
    }
    let (environment, module, owner_marker) = exact_environment(m, environment_id)?;
    let record = Adoption {
        schema: 1,
        profile_fingerprint: profile.fingerprint()?,
        family: Family::ArturiaPersistentV1,
        prior_inventory_sha256: prior_inventory(m, &environment)?,
        adopted_at: crate::observation::now()?,
        environment,
        module,
        owner_marker,
    };
    private_dir(&m.root.join("qualifications"))?;
    private_dir(&m.root.join("qualifications/frg1"))?;
    atomic_json(&adoption_path(m), &record)?;
    verify_adoption(m, &record)
}

pub fn adopted_environment(m: &Manager) -> Result<Option<EnvironmentBinding>> {
    if !adoption_path(m).try_exists()? {
        return Ok(None);
    }
    let record = adoption(m)?;
    Ok(Some(EnvironmentBinding {
        family: record.family,
        environment: record.environment,
    }))
}

/// An adoption always requires one new default-host inventory, even when an
/// older byte-current inventory happened to exist before custody crossed.
pub fn inventory_refresh_required(m: &Manager, environment: &Environment) -> Result<bool> {
    let record = adoption(m)?;
    require(record.environment == *environment, "frg1_adoption_identity")?;
    let path = m
        .root
        .join("inventory")
        .join(format!("{}.json", environment.id));
    if !path.try_exists()? {
        return Ok(true);
    }
    Ok(record
        .prior_inventory_sha256
        .as_ref()
        .is_some_and(|prior| digest(&path).is_ok_and(|current| current == *prior)))
}

fn current_inventory(m: &Manager, software: &Software) -> Result<inventory::Scan> {
    let record = adoption(m)?;
    let path = m
        .root
        .join("inventory")
        .join(format!("{}.json", record.environment.id));
    let scan: inventory::Scan = read_json(&path)?;
    require(
        scan.schema == 1
            && scan.environment == record.environment
            && scan.host == software.host
            && scan.host_source_sha256 == software.source_sha256
            && scan.completed_at >= record.adopted_at
            && record
                .prior_inventory_sha256
                .as_ref()
                .is_none_or(|prior| digest(&path).is_ok_and(|current| current != *prior)),
        "frg1_current_inventory_required",
    )?;
    software.host.verify()?;
    software.source_manifest.verify()?;
    let selected: Vec<_> = scan
        .modules
        .iter()
        .filter(|module| module.artifact == record.module)
        .collect();
    require(selected.len() == 1, "frg1_inventory_module")?;
    let module = selected[0];
    require(
        module.quarantine_reason.is_none()
            && module.inspection_error.is_none()
            && inventory::stale_reason(
                module,
                &scan.environment,
                &scan.host,
                &scan.host_source_sha256,
                &record.environment,
                &software.host,
                &software.source_sha256,
            )
            .is_none(),
        "frg1_current_inventory_required",
    )?;
    module.report.verify()?;
    let profile = candidate()?;
    let classes: Vec<_> = module
        .classes
        .iter()
        .filter(|class| class.id == profile.class.class_id)
        .collect();
    require(
        classes.len() == 1
            && classes[0].name == profile.class.name
            && classes[0].vendor == profile.class.vendor
            && classes[0].version == profile.class.version
            && classes[0].subcategories == profile.class.subcategories
            && classes[0].role == "effect",
        "frg1_inventory_class",
    )?;
    Ok(scan)
}

pub fn qualification_environment(m: &Manager, software: &Software) -> Result<EnvironmentBinding> {
    let record = adoption(m)?;
    current_inventory(m, software)?;
    Ok(EnvironmentBinding {
        family: record.family,
        environment: record.environment,
    })
}

pub fn stage(m: &Manager, package: &Path) -> Result<()> {
    let profile = candidate()?;
    adoption(m)?;
    let expected_manifest = package_manifest(&profile)?;
    let supplied: PackageManifest = read_json(&package.join("qualification.json"))?;
    require(supplied == expected_manifest, "frg1_package_manifest")?;
    let roster = [
        (
            "host.exe".to_string(),
            profile.requirements.host_sha256.clone(),
            "host.exe",
        ),
        (
            "host-source-manifest.json".into(),
            profile.requirements.host_source_sha256.clone(),
            "host-source-manifest.json",
        ),
        (
            format!("{}.so", profile.class.class_id),
            profile.requirements.native_sha256.clone(),
            "native.so",
        ),
    ];
    for (source, sha256, _) in &roster {
        Artifact {
            path: package.join(source),
            sha256: sha256.clone(),
        }
        .verify()?;
    }
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    require(
        !m.registry()?.classes.contains_key(&profile.class.class_id)
            && !m.publication_pending(&profile.class.class_id)?
            && physical(&m.link(&profile.class.class_id))?.is_none(),
        "frg1_local_predecessor_present",
    )?;
    let destination = directory(m, &profile)?;
    if destination.try_exists()? {
        let installed: PackageManifest = read_json(&destination.join("qualification.json"))?;
        require(installed == expected_manifest, "frg1_package_manifest")?;
        crate::qualification::load_for(m, profile, Qualification::Frg1Ubuntu)?;
        return Ok(());
    }
    private_dir(destination.parent().ok_or("frg1_stage_parent")?)?;
    let pending = destination.with_file_name(format!(".stage-{}", random_id()?));
    private_dir(&pending)?;
    let result = (|| -> Result<()> {
        for (source, sha256, target) in roster {
            let path = pending.join(target);
            fs::copy(package.join(source), &path)?;
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
        atomic_json(&pending.join("qualification.json"), &expected_manifest)?;
        fs::set_permissions(
            pending.join("qualification.json"),
            fs::Permissions::from_mode(0o400),
        )?;
        File::open(&pending)?.sync_all()?;
        rename_link(&pending, &destination, false)?;
        File::open(destination.parent().ok_or("frg1_stage_parent")?)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() && pending.try_exists()? {
        fs::remove_dir_all(pending)?;
    }
    result?;
    crate::qualification::load_for(m, profile, Qualification::Frg1Ubuntu)?;
    Ok(())
}

fn installed(m: &Manager) -> Result<InstalledCandidate> {
    let profile = candidate()?;
    let expected = package_manifest(&profile)?;
    let actual: PackageManifest = read_json(&directory(m, &profile)?.join("qualification.json"))?;
    require(actual == expected, "frg1_package_manifest")?;
    crate::qualification::load_for(m, profile, Qualification::Frg1Ubuntu)
}

pub fn binding(m: &Manager) -> Result<Registration> {
    let record = adoption(m)?;
    let candidate = installed(m)?;
    let registration = Registration {
        metadata: candidate.profile.class.clone(),
        module: record.module,
        environment: record.environment,
        host: candidate.host,
        host_source_sha256: candidate.source_manifest.sha256,
        native: candidate.native.artifact,
        compatibility: candidate.profile.capabilities.compatibility(),
    };
    registration.verify(&m.root)?;
    Ok(registration)
}

pub fn check_binding(m: &Manager, profile: &Profile, registration: &Registration) -> Result<()> {
    require(
        *profile == candidate()? && *registration == binding(m)?,
        "qualification_exact_candidate_required",
    )?;
    m.require_inactive(None)?;
    require(
        m.performance(&profile.class.class_id)?.added_frames == 512,
        "candidate_frame_posture",
    )
}

pub(crate) fn check_publication(
    m: &Manager,
    profile: &Profile,
    census: &Census,
    registration: &Registration,
) -> Result<()> {
    check_binding(m, profile, registration)?;
    let software: Software = read_json(&m.root.join("software.json"))?;
    let inventory = current_inventory(m, &software)?;
    require(
        census.captured_at >= inventory.completed_at
            && census.environment
                == EnvironmentBinding {
                    family: Family::ArturiaPersistentV1,
                    environment: registration.environment.clone(),
                }
            && census.module == registration.module
            && census.host == registration.host
            && census.host_source_sha256 == registration.host_source_sha256,
        "frg1_fresh_qualification_required",
    )?;
    crate::observation::select_for(
        std::slice::from_ref(profile),
        census,
        SelectionPurpose::Qualification,
    )?;
    require(
        !m.registry()?.classes.contains_key(&profile.class.class_id)
            && physical(&m.link(&profile.class.class_id))?.is_none(),
        "frg1_local_predecessor_present",
    )
}

pub(crate) fn retained(m: &Manager, revision: &Revision, exact: &InstalledCandidate) -> Result<()> {
    let mut expected = binding(m)?;
    expected.native.path = revision.registration.native.path.clone();
    require(
        revision.profile == exact.profile
            && revision.registration == expected
            && revision.parent.is_none()
            && revision.qualification == Some(Qualification::Frg1Ubuntu),
        "qualification_exact_candidate_required",
    )?;
    let db = m.registry()?;
    let entry = db
        .classes
        .get(&revision.class_id)
        .ok_or("candidate_not_published")?;
    let reference = entry
        .managed_revision
        .as_ref()
        .ok_or("candidate_not_published")?;
    require(
        reference.id == revision.id
            && entry.registration == revision.registration
            && entry.publication == Publication::Published
            && physical(&m.link(&revision.class_id))? == Some(revision.target.clone())
            && !m.publication_pending(&revision.class_id)?,
        "candidate_not_published",
    )?;
    m.verify_completed_publication(revision, reference)
}

pub(crate) fn served(m: &Manager, registration: &Registration) -> Result<()> {
    let profile = candidate()?;
    let db = m.registry()?;
    let entry = db
        .classes
        .get(&registration.key())
        .ok_or("candidate_not_published")?;
    let reference = entry
        .managed_revision
        .as_ref()
        .ok_or("candidate_not_published")?;
    let revision = m.load_revision(&registration.key(), reference)?;
    require(
        revision.registration == *registration,
        "installed_host_mismatch",
    )?;
    m.verify_retained_authority(&revision, &[profile])
}

pub fn restore(m: &Manager) -> Result<()> {
    let profile = candidate()?;
    if let Some(entry) = m.registry()?.classes.get(&profile.class.class_id) {
        let revision = m.load_revision(
            &profile.class.class_id,
            entry
                .managed_revision
                .as_ref()
                .ok_or("candidate_identity")?,
        )?;
        require(
            revision.profile == profile
                && revision.qualification == Some(Qualification::Frg1Ubuntu)
                && revision.parent.is_none(),
            "candidate_identity",
        )?;
        if entry.publication == Publication::Removed {
            require(
                physical(&m.link(&profile.class.class_id))?.is_none(),
                "foreign_publication",
            )?;
        } else {
            m.unpublish(&profile.class.class_id)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::{inspection_report, prepared, Fixture};

    struct Contract;
    impl Drop for Contract {
        fn drop(&mut self) {
            TEST_CONTRACT.with(|slot| *slot.borrow_mut() = None);
        }
    }

    #[test]
    fn compiled_package_manifest_is_the_exact_revision_eleven_roster() {
        let profile = crate::profiles::frg1_candidate().unwrap();
        let manifest = package_manifest(&profile).unwrap();
        assert_eq!(manifest.profile_fingerprint, profile.fingerprint().unwrap());
        assert_eq!(manifest.descriptor_sha256, profile.requirements.descriptor_sha256);
        assert_eq!(manifest.native_sha256, profile.requirements.native_sha256);
    }

    struct Prepared {
        fixture: Fixture,
        package: PathBuf,
        profile: Profile,
        census: Census,
        software: Software,
        _contract: Contract,
    }

    fn fixture() -> Prepared {
        let (fixture, mut profile, mut census, native) = prepared();
        fixture.m.unpublish(&profile.class.class_id).unwrap();
        atomic_json(&fixture.m.root.join("registry.json"), &Registry::default()).unwrap();
        profile.id = "frg1.test.fragments".into();
        profile.revision = 11;
        profile.claim = Claim::ReviewCandidate;
        profile.role = Role::Effect;
        profile.class.name = "Efx FRAGMENTS".into();
        profile.class.vendor = "Arturia".into();
        profile.class.version = "1.3.1.6566".into();
        profile.class.subcategories = "Fx|Tools".into();
        profile.factory_vendor = "Arturia".into();
        profile.requirements.environment_family = Family::ArturiaPersistentV1;
        profile.requirements.environment_revision = 2;
        profile.requirements.native_sha256 = native.artifact.sha256.clone();
        profile.requirements.native_source_commit = native.source_commit.clone();
        profile.requirements.descriptor_sha256 = native.descriptor_sha256.clone();
        profile.capabilities.editor = Editor::DetachedDirectVendorLifecycle;
        profile.capabilities.accessibility = Accessibility::DisabledForVendorProcess;
        profile.limitations = vec![
            Limitation::ShortDeliveryGaps,
            Limitation::ExactOperatorArtifactOnly,
        ];

        let mut environment = fixture.r.environment.clone();
        environment.revision = 2;
        atomic_json(&environment.root.join("environment.json"), &environment).unwrap();
        let owner = environment.root.join("compatdata/.ua1-owner.json");
        fs::write(&owner, b"exact retained ubuntu owner").unwrap();
        let relative = fixture
            .r
            .module
            .path
            .strip_prefix(environment.root.join("compatdata"))
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        TEST_CONTRACT.with(|slot| {
            *slot.borrow_mut() = Some(TestContract {
                profile: profile.clone(),
                owner_sha256: digest(&owner).unwrap(),
                module_relative: relative,
            })
        });

        census.environment = EnvironmentBinding {
            family: Family::ArturiaPersistentV1,
            environment: environment.clone(),
        };
        census.selected = profile.class.clone();
        census.factory_vendor = profile.factory_vendor.clone();
        census.classes = vec![profile.class.class_id.clone()];
        census.module = fixture.r.module.clone();
        census.module_stamp = crate::observation::ModuleStamp::read(&census.module.path).unwrap();
        atomic_json(&census.report.path, &inspection_report(&census)).unwrap();
        census.report.sha256 = digest(&census.report.path).unwrap();
        census = Census::from_report(
            census.environment.clone(),
            census.module.clone(),
            census.module_stamp.clone(),
            census.host.clone(),
            census.host_source_sha256.clone(),
            census.report.clone(),
            &profile.class.class_id,
        )
        .unwrap();

        let source_manifest = Artifact {
            path: fixture
                .r
                .host
                .path
                .with_file_name("host-source-manifest.json"),
            sha256: fixture.r.host_source_sha256.clone(),
        };
        let software = Software {
            installer_launch: None,
            preparation_kit: None,
            operator_frontend: None,
            manager: fixture.r.host.clone(),
            supervisor: fixture.r.host.clone(),
            ownership: fixture.r.host.clone(),
            host: fixture.r.host.clone(),
            source_manifest,
            source_sha256: fixture.r.host_source_sha256.clone(),
            native_catalogue: None,
        };
        atomic_json(&fixture.m.root.join("software.json"), &software).unwrap();

        let package = fixture.outer.join("frg1-package");
        private_dir(&package).unwrap();
        fs::copy(&fixture.r.host.path, package.join("host.exe")).unwrap();
        fs::copy(
            fixture
                .r
                .host
                .path
                .with_file_name("host-source-manifest.json"),
            package.join("host-source-manifest.json"),
        )
        .unwrap();
        fs::copy(
            &native.artifact.path,
            package.join(format!("{}.so", profile.class.class_id)),
        )
        .unwrap();
        atomic_json(
            &package.join("qualification.json"),
            &package_manifest(&profile).unwrap(),
        )
        .unwrap();
        Prepared {
            fixture,
            package,
            profile,
            census,
            software,
            _contract: Contract,
        }
    }

    fn retain_inventory(prepared: &Prepared) -> inventory::Scan {
        let adoption = adoption(&prepared.fixture.m).unwrap();
        let scan = inventory::Scan {
            schema: 1,
            id: "ab".repeat(16),
            environment: adoption.environment,
            host: prepared.software.host.clone(),
            host_source_sha256: prepared.software.source_sha256.clone(),
            completed_at: adoption.adopted_at,
            changes: inventory::Changes {
                initial_scan: true,
                added: 1,
                ..inventory::Changes::default()
            },
            modules: vec![inventory::Module {
                artifact: adoption.module,
                classes: vec![inventory::Class {
                    id: prepared.profile.class.class_id.clone(),
                    name: prepared.profile.class.name.clone(),
                    vendor: prepared.profile.class.vendor.clone(),
                    version: prepared.profile.class.version.clone(),
                    category: "Audio Module Class".into(),
                    subcategories: prepared.profile.class.subcategories.clone(),
                    role: "effect".into(),
                }],
                report: prepared.census.report.clone(),
                inspection_error: None,
                quarantine_reason: None,
            }],
        };
        private_dir(&prepared.fixture.m.root.join("inventory")).unwrap();
        atomic_json(
            &prepared
                .fixture
                .m
                .root
                .join("inventory")
                .join(format!("{}.json", scan.environment.id)),
            &scan,
        )
        .unwrap();
        scan
    }

    fn exact_census(prepared: &Prepared, scan: &inventory::Scan) -> Census {
        let installed = installed(&prepared.fixture.m).unwrap();
        let mut source = prepared.census.clone();
        source.environment = EnvironmentBinding {
            family: Family::ArturiaPersistentV1,
            environment: scan.environment.clone(),
        };
        source.module = scan.modules[0].artifact.clone();
        source.module_stamp = crate::observation::ModuleStamp::read(&source.module.path).unwrap();
        source.host = installed.host;
        source.host_source_sha256 = installed.source_manifest.sha256;
        let report = prepared.fixture.outer.join("frg1-fresh-inspection.json");
        atomic_json(&report, &inspection_report(&source)).unwrap();
        let report = Artifact {
            sha256: digest(&report).unwrap(),
            path: report,
        };
        Census::from_report(
            source.environment,
            source.module,
            source.module_stamp,
            source.host,
            source.host_source_sha256,
            report,
            &prepared.profile.class.class_id,
        )
        .unwrap()
    }

    #[test]
    fn sealed_adoption_fresh_selection_and_parentless_publication_refuse_cross_generation_inputs() {
        let prepared = fixture();
        let manager = &prepared.fixture.m;
        let owner = prepared.census.environment.environment.root.join("compatdata/.ua1-owner.json");
        let owner_before = digest(&owner).unwrap();
        adopt(manager, &prepared.census.environment.environment.id).unwrap();
        assert_eq!(digest(&owner).unwrap(), owner_before);
        assert!(qualification_environment(manager, &prepared.software).is_err());
        let scan = retain_inventory(&prepared);
        assert!(!inventory_refresh_required(manager, &scan.environment).unwrap());
        stage(manager, &prepared.package).unwrap();
        stage(manager, &prepared.package).unwrap();
        let registration = binding(manager).unwrap();

        let mut wrong = registration.clone();
        wrong.module.sha256 = "11".repeat(32);
        assert!(check_binding(manager, &prepared.profile, &wrong).is_err());
        wrong = registration.clone();
        wrong.environment.runner.policy = Some(RunnerPolicy::DcompWineBuiltinsReferenceV1);
        assert!(check_binding(manager, &prepared.profile, &wrong).is_err());
        wrong = registration.clone();
        wrong.host.sha256 = "22".repeat(32);
        assert!(check_binding(manager, &prepared.profile, &wrong).is_err());
        wrong = registration.clone();
        wrong.host_source_sha256 = "33".repeat(32);
        assert!(check_binding(manager, &prepared.profile, &wrong).is_err());
        wrong = registration.clone();
        wrong.native.sha256 = "44".repeat(32);
        assert!(check_binding(manager, &prepared.profile, &wrong).is_err());

        let manifest_path = directory(manager, &prepared.profile)
            .unwrap()
            .join("qualification.json");
        fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o600)).unwrap();
        let mut manifest: PackageManifest = read_json(&manifest_path).unwrap();
        manifest.descriptor_sha256 = "55".repeat(32);
        atomic_json(&manifest_path, &manifest).unwrap();
        assert!(installed(manager).is_err());
        atomic_json(
            &manifest_path,
            &package_manifest(&prepared.profile).unwrap(),
        )
        .unwrap();
        fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o400)).unwrap();

        let native_path = directory(manager, &prepared.profile)
            .unwrap()
            .join("native.so");
        fs::set_permissions(&native_path, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(&native_path, b"wrong native").unwrap();
        assert!(installed(manager).is_err());
        fs::copy(
            prepared
                .package
                .join(format!("{}.so", prepared.profile.class.class_id)),
            &native_path,
        )
        .unwrap();
        fs::set_permissions(&native_path, fs::Permissions::from_mode(0o500)).unwrap();

        let mut stale = exact_census(&prepared, &scan);
        stale.captured_at = scan.completed_at.saturating_sub(1);
        assert!(manager
            .qualify_for(&stale, None, Qualification::Frg1Ubuntu)
            .is_err());
        let census = exact_census(&prepared, &scan);
        let reference = manager
            .qualify_for(&census, None, Qualification::Frg1Ubuntu)
            .unwrap();
        let revision = manager
            .load_revision(&prepared.profile.class.class_id, &reference)
            .unwrap();
        assert!(revision.parent.is_none());
        assert_eq!(revision.qualification, Some(Qualification::Frg1Ubuntu));
        assert!(prepared
            .profile
            .claim
            .require(SelectionPurpose::Activation)
            .is_err());
        assert!(!installed_profiles()
            .unwrap()
            .iter()
            .any(|profile| profile == &prepared.profile));
        manager
            .verify_retained_authority(&revision, std::slice::from_ref(&prepared.profile))
            .unwrap();
        served(manager, &revision.registration).unwrap();
        restore(manager).unwrap();
        assert_eq!(
            manager.registry().unwrap().classes[&prepared.profile.class.class_id].publication,
            Publication::Removed
        );
        assert!(!manager.link(&prepared.profile.class.class_id).exists());
    }
}

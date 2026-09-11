//! Sealed AP15 editor and AP17 capacity engineering qualifications. Ordinary activation policy is unchanged.
//! The sealed package catalogue is compiled with the manager; callers supply
//! neither a profile nor a native image as activation authority.
use crate::{catalogue::NativeArtifact, observation::Census, profiles::*, publication::*, *};

pub fn candidates() -> Result<Vec<Profile>> {
    let result = [
        include_bytes!("../../compatibility/ap15/arturia-pure-lofi.json").as_slice(),
        include_bytes!("../../compatibility/ap15/arturia-efx-fragments.json").as_slice(),
    ]
    .into_iter()
    .map(Profile::parse)
    .collect::<Result<Vec<_>>>()?;
    validate_set(&result)?;
    Ok(result)
}
// A missing finite roster is a hard refusal, never arbitrary profile input.
fn capacity_candidates() -> Result<Vec<Profile>> {
    let result = [
        include_bytes!("../../compatibility/ap17/arturia-pure-lofi.json").as_slice(),
        include_bytes!("../../compatibility/ap17/arturia-efx-fragments.json").as_slice(),
    ].into_iter().map(Profile::parse).collect::<Result<Vec<_>>>()?;
    validate_set(&result)?;
    Ok(result)
}
pub fn candidates_for(purpose: Qualification) -> Result<Vec<Profile>> {
    match purpose {
        Qualification::Ap15Editor => candidates(),
        Qualification::Ap17Capacity => capacity_candidates(),
        Qualification::Ap18Pigments => Ok(vec![crate::pigments::candidate()?]),
    }
}
fn parent_revision(purpose: Qualification) -> u32 {
    match purpose {
        Qualification::Ap15Editor => 3,
        Qualification::Ap17Capacity => 7,
        Qualification::Ap18Pigments => 10,
    }
}
#[derive(Clone)]
pub struct InstalledCandidate {
    pub profile: Profile,
    pub native: NativeArtifact,
    pub host: Artifact,
    pub source_manifest: Artifact,
}
fn directory(m: &Manager, p: &Profile, purpose: Qualification) -> Result<PathBuf> {
    Ok(m.root
        .join(match purpose {
            Qualification::Ap15Editor => "software/ap15-qualification",
            Qualification::Ap17Capacity => "software/ap17-qualification",
            Qualification::Ap18Pigments => "software/ap18-qualification",
        })
        .join(p.fingerprint()?))
}
pub(crate) fn load(m: &Manager, p: Profile) -> Result<InstalledCandidate> {
    load_for(m, p, Qualification::Ap15Editor)
}
pub(crate) fn load_for(
    m: &Manager,
    p: Profile,
    purpose: Qualification,
) -> Result<InstalledCandidate> {
    let dir = directory(m, &p, purpose)?;
    let host = Artifact {
        path: dir.join("host.exe"),
        sha256: p.requirements.host_sha256.clone(),
    };
    let source_manifest = Artifact {
        path: dir.join("host-source-manifest.json"),
        sha256: p.requirements.host_source_sha256.clone(),
    };
    let native = NativeArtifact {
        class: p.class.clone(),
        module_sha256: p.module_sha256.clone(),
        artifact: Artifact {
            path: dir.join("native.so"),
            sha256: p.requirements.native_sha256.clone(),
        },
        source_commit: p.requirements.native_source_commit.clone(),
        descriptor_sha256: p.requirements.descriptor_sha256.clone(),
        external_ids: external_ids(&p.class.class_id)?,
    };
    for a in [&host, &source_manifest, &native.artifact] {
        require(
            a.path.canonicalize()? == a.path && file(&a.path)?.metadata()?.mode() & 0o222 == 0,
            "qualification_artifact_location_or_mutability",
        )?;
        a.verify()?;
    }
    native.matches(&p)?;
    Ok(InstalledCandidate {
        profile: p,
        native,
        host,
        source_manifest,
    })
}
pub fn installed(m: &Manager) -> Result<Vec<InstalledCandidate>> {
    installed_for(m, Qualification::Ap15Editor)
}
pub fn installed_for(m: &Manager, purpose: Qualification) -> Result<Vec<InstalledCandidate>> {
    candidates_for(purpose)?
        .into_iter()
        .map(|p| load_for(m, p, purpose))
        .collect()
}
/// Engineering package input is a location only. Every accepted byte and the
/// finite file roster are fixed by the compiled candidate profiles.
pub fn stage(m: &Manager, package: &Path) -> Result<()> {
    let policies = candidates()?;
    stage_selected(m, package, &policies)
}
// Private production helper; tests supply synthetic sealed bytes, never CLI policy.
pub(crate) fn stage_selected(m: &Manager, package: &Path, policies: &[Profile]) -> Result<()> {
    stage_selected_for(m, package, policies, Qualification::Ap15Editor)
}
pub fn stage_for(m: &Manager, package: &Path, purpose: Qualification) -> Result<()> {
    if purpose == Qualification::Ap18Pigments { return crate::pigments::stage(m, package); }
    stage_selected_for(m, package, &candidates_for(purpose)?, purpose)
}
pub(crate) fn stage_selected_for(
    m: &Manager,
    package: &Path,
    policies: &[Profile],
    purpose: Qualification,
) -> Result<()> {
    validate_set(policies)?;
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let db = m.registry()?;
    // Validate the complete candidate set and all source files before copying.
    for p in policies {
        require(
            !m.publication_pending(&p.class.class_id)?,
            "qualification_publication_pending",
        )?;
        let e = db
            .classes
            .get(&p.class.class_id)
            .ok_or("qualification_verified_parent_required")?;
        let mut registration = e.registration.clone();
        registration.host.sha256 = p.requirements.host_sha256.clone();
        registration.native.sha256 = p.requirements.native_sha256.clone();
        registration.host_source_sha256 = p.requirements.host_source_sha256.clone();
        m.verify_qualification_parent_for(&db, p, &registration, purpose)?;
        for (name, hash) in package_files(p) {
            Artifact {
                path: package.join(name),
                sha256: hash,
            }
            .verify()?;
        }
    }
    for p in policies.iter().cloned() {
        let dest = directory(m, &p, purpose)?;
        if fs::symlink_metadata(&dest).is_ok() {
            load_for(m, p, purpose)?;
            continue;
        }
        private_dir(dest.parent().unwrap())?;
        let pending = dest.with_file_name(format!(".stage-{}", random_id()?));
        private_dir(&pending)?;
        for ((source, sha256), name) in package_files(&p).into_iter().zip([
            "host.exe",
            "host-source-manifest.json",
            "native.so",
        ]) {
            let target = pending.join(name);
            fs::copy(package.join(source), &target)?;
            Artifact {
                path: target.clone(),
                sha256,
            }
            .verify()?;
            fs::set_permissions(
                &target,
                fs::Permissions::from_mode(if name == "native.so" { 0o500 } else { 0o400 }),
            )?;
            File::open(target)?.sync_all()?;
        }
        File::open(&pending)?.sync_all()?;
        crate::publication::rename_link(&pending, &dest, false)?;
        File::open(dest.parent().unwrap())?.sync_all()?;
        load_for(m, p, purpose)?;
    }
    Ok(())
}
fn package_files(p: &Profile) -> [(String, String); 3] {
    [
        ("host.exe".into(), p.requirements.host_sha256.clone()),
        (
            "host-source-manifest.json".into(),
            p.requirements.host_source_sha256.clone(),
        ),
        (
            format!("{}.so", p.class.class_id),
            p.requirements.native_sha256.clone(),
        ),
    ]
}
impl Manager {
    // Both normal and cross-version serving use this single retained-claim
    // owner. A rollbackable historical record is not automatically serveable.
    // The roster argument is supplied only by compiled product policy (or the
    // deterministic private fixture); there is no CLI/profile input here.
    pub(crate) fn verify_retained_authority(&self, r: &Revision, roster: &[Profile]) -> Result<()> {
        let p = &r.profile;
        require(
            p.capabilities.state == State::ConcurrentReadOnlyCaptureV12
                && p.requirements.host_sha256 == r.registration.host.sha256
                && p.requirements.host_source_sha256 == r.registration.host_source_sha256,
            "installed_host_mismatch",
        )?;
        if p.claim.permits(SelectionPurpose::Activation) {
            return require(
                r.qualification.is_none(),
                "qualification_candidate_contract",
            );
        }
        let purpose = r.qualification.ok_or("qualification_candidate_contract")?;
        require(
            p.claim == Claim::ReviewCandidate
                && p.capabilities.editor == Editor::DetachedDirectVendorLifecycle,
            "qualification_candidate_contract",
        )?;
        require(!roster.is_empty(), "qualification_exact_candidate_required")?;
        validate_set(roster)?;
        require(
            roster.iter().filter(|candidate| *candidate == p).count() == 1,
            "qualification_exact_candidate_required",
        )?;
        let exact = load_for(self, p.clone(), purpose)?;
        require(
            exact.host == r.registration.host
                && exact.source_manifest.sha256 == r.registration.host_source_sha256
                && exact.native.artifact.sha256 == r.registration.native.sha256
                && r.external_ids == exact.native.external_ids,
            "qualification_exact_candidate_required",
        )?;
        r.registration.verify(&self.root)?;
        r.census.verify_current(
            &self.root,
            &exact.host,
            &exact.source_manifest.sha256,
            r.census.captured_at,
        )?;
        let mut derived = crate::observation::derive_for(
            p,
            &r.census,
            &exact.native,
            SelectionPurpose::Qualification,
        )?;
        derived.native.path = r.registration.native.path.clone();
        require(
            derived == r.registration && r.performance.added_frames == 512,
            "qualification_exact_candidate_required",
        )?;
        if purpose == Qualification::Ap18Pigments { return crate::pigments::retained(self, r, &exact); }
        let parent = r
            .parent
            .as_ref()
            .ok_or("qualification_verified_parent_required")?;
        let prior = self.load_revision(&r.class_id, parent)?;
        let mut prior_db = self.registry()?;
        let entry = prior_db
            .classes
            .get_mut(&r.class_id)
            .ok_or("qualification_verified_parent_required")?;
        require(
            entry
                .managed_revision
                .as_ref()
                .is_some_and(|reference| reference.id == r.id)
                && entry.registration == r.registration
                && entry.publication == Publication::Published
                && crate::publication::physical(&self.link(&r.class_id))? == Some(r.target.clone())
                && !self.publication_pending(&r.class_id)?,
            "qualification_publication_changed",
        )?;
        // Reuse the publication parent contract, supplying the recorded parent
        // instead of pretending the current physical pointer still names it.
        entry.managed_revision = Some(parent.clone());
        entry.registration = prior.registration.clone();
        self.verify_qualification_parent_record(&prior_db, p, &r.registration, false, purpose)?;
        Ok(())
    }
    /// Read-only guard used by the existing supervised inspection admission.
    pub fn check_editor_qualification_parent(&self, p: &Profile, r: &Registration) -> Result<()> {
        self.check_qualification_parent_for(p, r, Qualification::Ap15Editor)
    }
    pub fn check_qualification_parent_for(
        &self,
        p: &Profile,
        r: &Registration,
        purpose: Qualification,
    ) -> Result<()> {
        let candidates = installed_for(self, purpose)?;
        require(
            candidates.iter().any(|c| {
                c.profile == *p
                    && c.host == r.host
                    && c.native.artifact == r.native
                    && c.source_manifest.sha256 == r.host_source_sha256
            }),
            "qualification_exact_candidate_required",
        )?;
        if purpose == Qualification::Ap18Pigments { return crate::pigments::check_publication(self, p, r); }
        self.verify_qualification_parent_for(&self.registry()?, p, r, purpose)
            .map(|_| ())
    }
    pub(crate) fn verify_qualification_parent(
        &self,
        db: &Registry,
        p: &Profile,
        registration: &Registration,
    ) -> Result<Revision> {
        self.verify_qualification_parent_for(db, p, registration, Qualification::Ap15Editor)
    }
    pub(crate) fn verify_qualification_parent_for(
        &self,
        db: &Registry,
        p: &Profile,
        registration: &Registration,
        purpose: Qualification,
    ) -> Result<Revision> {
        self.verify_qualification_parent_record(db, p, registration, true, purpose)
    }
    fn verify_qualification_parent_record(
        &self,
        db: &Registry,
        p: &Profile,
        registration: &Registration,
        check_pointer: bool,
        purpose: Qualification,
    ) -> Result<Revision> {
        p.validate()?;
        require(
            p.claim == Claim::ReviewCandidate
                && match purpose {
                    Qualification::Ap15Editor => p.revision > 3,
                    Qualification::Ap17Capacity => matches!(p.revision, 8 | 9),
                    Qualification::Ap18Pigments => false,
                }
                && p.capabilities.editor == Editor::DetachedDirectVendorLifecycle,
            "qualification_candidate_contract",
        )?;
        let e = db
            .classes
            .get(&p.class.class_id)
            .ok_or("qualification_verified_parent_required")?;
        let reference = e
            .managed_revision
            .as_ref()
            .ok_or("qualification_verified_parent_required")?;
        let prior = self.load_revision(&p.class.class_id, reference)?;
        let r = &prior.registration;
        let mut capabilities = p.capabilities.clone();
        let mut limitations = prior.profile.limitations.clone();
        match purpose {
            Qualification::Ap18Pigments => return Err("pigments_has_no_same_class_parent".into()),
            Qualification::Ap15Editor => {
                capabilities.editor = Editor::DetachedOwnerThreadWithNativePanel;
                limitations.push(Limitation::DirectEditorUnderQualification);
            }
            Qualification::Ap17Capacity => {
                limitations.push(Limitation::CapacityUnderQualification);
                require(
                    p.requirements.host_sha256 == prior.profile.requirements.host_sha256
                        && p.requirements.host_source_sha256
                            == prior.profile.requirements.host_source_sha256
                        && p.requirements.native_sha256 != prior.profile.requirements.native_sha256,
                    "qualification_capacity_contract",
                )?;
            }
        }
        require(
            e.publication == Publication::Published
                && prior.qualification.is_none()
                && prior.profile.claim == Claim::VerifiedExactFixture
                && prior.profile.revision == parent_revision(purpose)
                && prior.profile.id == p.id
                && prior.profile.module_sha256 == p.module_sha256
                && prior.profile.class == p.class
                && prior.profile.factory_vendor == p.factory_vendor
                && prior.profile.role == p.role
                && prior.profile.capabilities == capabilities
                && prior.profile.requirements.runner == p.requirements.runner
                && prior.profile.requirements.environment_family
                    == p.requirements.environment_family
                && prior.profile.requirements.environment_revision
                    == p.requirements.environment_revision
                && prior.profile.requirements.descriptor_sha256 == p.requirements.descriptor_sha256
                && limitations == p.limitations
                && r.metadata == registration.metadata
                && r.module == registration.module
                && r.environment == registration.environment
                && r.compatibility == registration.compatibility
                && e.registration == *r
                && self.performance(&p.class.class_id)?.added_frames == 512
                && (!check_pointer
                    || crate::publication::physical(&self.link(&p.class.class_id))?
                        == Some(prior.target.clone()))
                && prior.external_ids == external_ids(&p.class.class_id)?,
            "qualification_verified_parent_mismatch",
        )?;
        r.verify(&self.root)?;
        Ok(prior)
    }
    /// A caller selects only a compiled exact candidate. Registration is derived
    /// from supervised facts and product-owned artifact records inside this call.
    pub fn qualify_editor(&self, census: &Census, fail: Option<Boundary>) -> Result<RevisionRef> {
        self.qualify_for(census, fail, Qualification::Ap15Editor)
    }
    pub fn qualify_for(
        &self,
        census: &Census,
        fail: Option<Boundary>,
        purpose: Qualification,
    ) -> Result<RevisionRef> {
        let candidates = installed_for(self, purpose)?;
        let selected: Vec<_> = candidates
            .iter()
            .filter(|c| c.profile.class.class_id == census.selected.class_id)
            .collect();
        require(
            selected.len() == 1,
            "qualification_exact_candidate_required",
        )?;
        let c = selected[0];
        let r = crate::observation::derive_for(
            &c.profile,
            census,
            &c.native,
            SelectionPurpose::Qualification,
        )?;
        self.publish_selected(
            &c.profile,
            census,
            r,
            (&c.host, &c.source_manifest.sha256),
            Some(purpose),
            fail,
        )
    }
    /// Explicit reconcile and service startup return completed engineering
    /// candidates to their exact immutable verified parents. No timer or audio
    /// callback participates. Active DSP blocks this normal rollback law.
    pub fn restore_editor_qualifications(&self) -> Result<()> {
        let db = self.registry()?;
        for (key, e) in db.classes {
            let Some(reference) = e.managed_revision else {
                continue;
            };
            let r = self.load_revision(&key, &reference)?;
            let Some(purpose) = r.qualification else {
                continue;
            };
            if purpose == Qualification::Ap18Pigments { crate::pigments::restore(self)?; continue; }
            let parent = r.parent.as_ref().ok_or("qualification_parent_absent")?;
            let prior = self.load_revision(&key, parent)?;
            require(
                prior.profile.claim == Claim::VerifiedExactFixture
                    && prior.profile.revision == parent_revision(purpose)
                    && prior.qualification.is_none(),
                "qualification_verified_parent_required",
            )?;
            self.rollback_expected(&key, &parent.id, None, Some(&reference))?;
        }
        Ok(())
    }
}

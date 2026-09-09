//! AP15 engineering qualification only. Ordinary activation policy is unchanged.
//! The sealed package catalogue is compiled with the manager; callers supply
//! neither a profile nor a native image as activation authority.
use crate::{catalogue::NativeArtifact, observation::Census, profiles::*, publication::*, *};

pub fn candidates() -> Result<Vec<Profile>> {
    // Exact immutable candidates are added after production host/native builds.
    // Until those identities exist this route has no activation authority.
    Err("ap15_candidate_artifacts_pending".into())
}
#[derive(Clone)]
pub struct InstalledCandidate {
    pub profile: Profile,
    pub native: NativeArtifact,
    pub host: Artifact,
    pub source_manifest: Artifact,
}
fn directory(m: &Manager, p: &Profile) -> Result<PathBuf> {
    Ok(m.root
        .join("software/ap15-qualification")
        .join(p.fingerprint()?))
}
fn load(m: &Manager, p: Profile) -> Result<InstalledCandidate> {
    let dir = directory(m, &p)?;
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
    candidates()?.into_iter().map(|p| load(m, p)).collect()
}
/// Engineering package input is a location only. Every accepted byte and the
/// finite file roster are fixed by the compiled candidate profiles.
pub fn stage(m: &Manager, package: &Path) -> Result<()> {
    let policies = candidates()?;
    validate_set(&policies)?;
    let _lock = m.lock("registry.lock")?;
    m.require_inactive(None)?;
    let db = m.registry()?;
    // Validate the complete candidate set and all source files before copying.
    for p in &policies {
        let e = db
            .classes
            .get(&p.class.class_id)
            .ok_or("qualification_verified_parent_required")?;
        let mut registration = e.registration.clone();
        registration.host.sha256 = p.requirements.host_sha256.clone();
        registration.native.sha256 = p.requirements.native_sha256.clone();
        registration.host_source_sha256 = p.requirements.host_source_sha256.clone();
        m.verify_qualification_parent(&db, p, &registration)?;
        for (name, hash) in package_files(p) {
            Artifact {
                path: package.join(name),
                sha256: hash,
            }
            .verify()?;
        }
    }
    for p in policies {
        let dest = directory(m, &p)?;
        if fs::symlink_metadata(&dest).is_ok() {
            load(m, p)?;
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
        load(m, p)?;
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
        require(
            p.claim == Claim::ReviewCandidate
                && r.qualification == Some(Qualification::Ap15Editor)
                && p.capabilities.editor == Editor::DetachedDirectVendorLifecycle,
            "qualification_candidate_contract",
        )?;
        require(!roster.is_empty(), "qualification_exact_candidate_required")?;
        validate_set(roster)?;
        require(
            roster.iter().filter(|candidate| *candidate == p).count() == 1,
            "qualification_exact_candidate_required",
        )?;
        let exact = load(self, p.clone())?;
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
        self.verify_qualification_parent_record(&prior_db, p, &r.registration, false)?;
        Ok(())
    }
    /// Read-only guard used by the existing supervised inspection admission.
    pub fn check_editor_qualification_parent(&self, p: &Profile, r: &Registration) -> Result<()> {
        let candidates = installed(self)?;
        require(
            candidates.iter().any(|c| {
                c.profile == *p
                    && c.host == r.host
                    && c.native.artifact == r.native
                    && c.source_manifest.sha256 == r.host_source_sha256
            }),
            "qualification_exact_candidate_required",
        )?;
        self.verify_qualification_parent(&self.registry()?, p, r)
            .map(|_| ())
    }
    pub(crate) fn verify_qualification_parent(
        &self,
        db: &Registry,
        p: &Profile,
        registration: &Registration,
    ) -> Result<Revision> {
        self.verify_qualification_parent_record(db, p, registration, true)
    }
    fn verify_qualification_parent_record(
        &self,
        db: &Registry,
        p: &Profile,
        registration: &Registration,
        check_pointer: bool,
    ) -> Result<Revision> {
        p.validate()?;
        require(
            p.claim == Claim::ReviewCandidate
                && p.revision > 3
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
        capabilities.editor = Editor::DetachedOwnerThreadWithNativePanel;
        require(
            e.publication == Publication::Published
                && prior.qualification.is_none()
                && prior.profile.claim == Claim::VerifiedExactFixture
                && prior.profile.revision == 3
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
                && prior.profile.limitations == p.limitations
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
        let candidates = installed(self)?;
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
            Some(Qualification::Ap15Editor),
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
            if r.qualification.is_none() {
                continue;
            }
            let parent = r.parent.as_ref().ok_or("qualification_parent_absent")?;
            let prior = self.load_revision(&key, parent)?;
            require(
                prior.profile.claim == Claim::VerifiedExactFixture
                    && prior.profile.revision == 3
                    && prior.qualification.is_none(),
                "qualification_verified_parent_required",
            )?;
            self.rollback_expected(&key, &parent.id, None, Some(&reference))?;
        }
        Ok(())
    }
}

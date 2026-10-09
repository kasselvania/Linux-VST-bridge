//! Revisioned transactions around the existing registry and discovery links.
//! Intent and immutable records precede activation; physical links decide recovery.
use crate::{observation::Census, profiles::*, *};
use crate::operator_lock::timing::{self, Stage};
use std::{ffi::CString, os::unix::ffi::OsStrExt};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RevisionRef {
    pub id: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Revision {
    pub schema: u32,
    pub id: String,
    pub class_id: String,
    pub external_ids: [String; 2],
    pub profile: Profile,
    pub profile_sha256: String,
    pub census: Census,
    pub registration: Registration,
    pub performance: Performance,
    pub target: PathBuf,
    pub parent: Option<RevisionRef>,
    pub transaction: String,
    pub adopted_legacy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualification: Option<Qualification>,
}
/// A bounded engineering publication is retained history, not ordinary policy.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Qualification {
    Ap15Editor,
    Ap17Capacity,
    Ap18Pigments,
    Uir1Input,
    If1Failure,
    Sv1Instrument,
    Frg1Ubuntu,
    ManagedExperimental,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Prior {
    entry: Entry,
    revision: RevisionRef,
    target: Option<PathBuf>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema: u32,
    id: String,
    class_id: String,
    prior: Option<Prior>,
    candidate: Option<RevisionRef>,
    candidate_target: Option<PathBuf>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[doc(hidden)]
pub struct PublicationState {
    pub class_id: String,
    pub entry: Entry,
    pub target: Option<PathBuf>,
    pub performance: Performance,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[doc(hidden)]
pub struct PreparedTransition {
    pub schema: u32,
    pub before: PublicationState,
    pub after: PublicationState,
    intent: Intent,
    reverse_intent: Intent,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Committed,
    Aborted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    schema: u32,
    transaction: String,
    class_id: String,
    outcome: Outcome,
}

/// Test injection is at durable production boundaries, never a profile hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Boundary {
    ProfileRetained,
    PriorRetained,
    IntentArchived,
    Intent,
    CandidateCreated,
    NativeCopied,
    BundleProvenanceWritten,
    RevisionWritten,
    ProvenanceWritten,
    CandidateReady,
    PointerStaged,
    PointerExchanged,
    PointerSynced,
    RegistryCommitted,
    ResultWritten,
    Cleanup,
}
pub const BOUNDARIES: [Boundary; 16] = [
    Boundary::ProfileRetained,
    Boundary::PriorRetained,
    Boundary::IntentArchived,
    Boundary::Intent,
    Boundary::CandidateCreated,
    Boundary::NativeCopied,
    Boundary::BundleProvenanceWritten,
    Boundary::RevisionWritten,
    Boundary::ProvenanceWritten,
    Boundary::CandidateReady,
    Boundary::PointerStaged,
    Boundary::PointerExchanged,
    Boundary::PointerSynced,
    Boundary::RegistryCommitted,
    Boundary::ResultWritten,
    Boundary::Cleanup,
];
#[cfg(test)]
thread_local! {pub(crate) static COPY_OBSERVER:std::cell::RefCell<Option<Box<dyn Fn()>>>=const {std::cell::RefCell::new(None)};}
fn boundary(fail: Option<Boundary>, here: Boundary) -> Result<()> {
    #[cfg(test)] if here==Boundary::NativeCopied { COPY_OBSERVER.with(|h|{if let Some(f)=h.borrow().as_ref(){f();}}); }
    require(fail != Some(here), &format!("injected_{here:?}"))
}
fn encoded<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}
fn sync(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}
fn immutable<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = encoded(value)?;
    if fs::symlink_metadata(path).is_ok() {
        let mut old = Vec::new();
        file(path)?.take(1024 * 1024).read_to_end(&mut old)?;
        return require(old == bytes, "immutable_record_conflict");
    }
    let temp = path.with_extension(format!("writing-{}", random_id()?));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o400)
        .open(&temp)?;
    f.write_all(&bytes)?;
    f.sync_all()?;
    rename_link(&temp, path, false)?;
    sync(path.parent().ok_or("record_parent")?)
}
pub fn physical(link: &Path) -> Result<Option<PathBuf>> {
    match fs::symlink_metadata(link) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
        Ok(m) => {
            require(
                m.file_type().is_symlink() && m.uid() == unsafe { libc::getuid() },
                "foreign_publication",
            )?;
            Ok(Some(fs::read_link(link)?))
        }
    }
}
/// Never replace an unobserved entry with ordinary rename. An exchange retains
/// the old link at the transaction's exact temporary name until it is checked.
pub(crate) fn rename_link(from: &Path, to: &Path, exchange: bool) -> Result<()> {
    let from = CString::new(from.as_os_str().as_bytes())?;
    let to = CString::new(to.as_os_str().as_bytes())?;
    #[cfg(target_os = "linux")]
    let code = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            from.as_ptr(),
            libc::AT_FDCWD,
            to.as_ptr(),
            if exchange {
                libc::RENAME_EXCHANGE
            } else {
                libc::RENAME_NOREPLACE
            },
        )
    };
    #[cfg(target_os = "macos")]
    let code = unsafe {
        libc::renamex_np(
            from.as_ptr(),
            to.as_ptr(),
            if exchange {
                libc::RENAME_SWAP
            } else {
                libc::RENAME_EXCL
            },
        )
    };
    if code != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(())
}

/// Stable operator entry point for the immutable installed manager. Foreign
/// commands are never replaced, including a change between check and exchange.
pub fn preflight_command(link: &Path, candidate: &Path, prior: Option<&Path>) -> Result<()> {
    let actual = physical(link)?;
    require(
        actual.is_none() || actual.as_deref() == Some(candidate) || actual.as_deref() == prior,
        "foreign_manager_command",
    )?;
    let parent = link.parent().ok_or("command_parent")?;
    if parent.try_exists()? {
        let metadata = fs::symlink_metadata(parent)?;
        require(
            metadata.is_dir() && metadata.uid() == unsafe { libc::getuid() },
            "command_directory_owner",
        )?;
    }
    Ok(())
}
pub fn install_command(link: &Path, candidate: &Path, prior: Option<&Path>) -> Result<()> {
    let actual = physical(link)?;
    if actual.as_deref() == Some(candidate) {
        return Ok(());
    }
    require(
        actual.is_none() || actual.as_deref() == prior,
        "foreign_manager_command",
    )?;
    let parent = link.parent().ok_or("command_parent")?;
    fs::create_dir_all(parent)?;
    let metadata = fs::symlink_metadata(parent)?;
    require(
        metadata.is_dir() && metadata.uid() == unsafe { libc::getuid() },
        "command_directory_owner",
    )?;
    let temp = parent.join(format!(".lvb-command-{}", random_id()?));
    symlink(candidate, &temp)?;
    sync(parent)?;
    rename_link(&temp, link, actual.is_some())?;
    if actual.is_some() {
        if physical(&temp)? != actual {
            rename_link(&temp, link, true)?;
            sync(parent)?;
            return Err("foreign_manager_command".into());
        }
        fs::remove_file(temp)?;
    }
    sync(parent)
}

impl Manager {
    /// A retained qualification must be a completed exact transaction, not a
    /// candidate directory abandoned before activation.
    pub fn verify_completed_publication(
        &self,
        r: &Revision,
        reference: &RevisionRef,
    ) -> Result<()> {
        let dir = self.root.join("transactions");
        let intent: Intent = read_json(&dir.join(format!("{}.json", r.transaction)))?;
        let result: Completion = read_json(&dir.join(format!("{}.result.json", r.transaction)))?;
        require(
            intent.schema == 1
                && intent.id == r.transaction
                && intent.class_id == r.class_id
                && intent.candidate.as_ref() == Some(reference)
                && intent.candidate_target.as_ref() == Some(&r.target)
                && intent.prior.as_ref().map(|p| &p.revision) == r.parent.as_ref()
                && result.schema == 1
                && result.transaction == r.transaction
                && result.class_id == r.class_id
                && result.outcome == Outcome::Committed,
            "acceptance_publication_incomplete",
        )
    }
    fn durable_dir(&self, path: &Path) -> Result<()> {
        require(path.starts_with(&self.root), "managed_directory_binding")?;
        private_dir(path)?;
        let mut current = Some(path);
        while let Some(dir) = current.filter(|dir| dir.starts_with(&self.root)) {
            sync(dir)?;
            current = dir.parent();
        }
        Ok(())
    }
    fn revisions(&self, key: &str) -> PathBuf {
        self.root.join("publications").join(key).join("revisions")
    }
    fn revision_dir(&self, key: &str, id: &str) -> Result<PathBuf> {
        require(
            valid_hex(key, 32) && valid_hex(id, 32),
            "publication_revision_identity",
        )?;
        Ok(self.revisions(key).join(id))
    }
    fn revision_ref(r: &Revision) -> Result<RevisionRef> {
        Ok(RevisionRef {
            id: r.id.clone(),
            sha256: hex(&Sha256::digest(encoded(r)?)),
        })
    }
    fn revision_record(&self, key: &str, reference: &RevisionRef) -> Result<Revision> {
        require(
            valid_hex(&reference.sha256, 64),
            "publication_revision_hash",
        )?;
        let path = self.revision_dir(key, &reference.id)?.join("revision.json");
        let r: Revision = Artifact { path, sha256:reference.sha256.clone() }
            .read_record(8 * 1024 * 1024)?;
        r.profile.validate()?;
        require(
            r.schema == 1
                && r.id == reference.id
                && r.class_id == key
                && r.registration.key() == key
                && r.profile.class.class_id == key
                && r.external_ids == external_ids(key)?
                && r.profile_sha256 == r.profile.fingerprint()?
                && valid_hex(&r.transaction, 32),
            "publication_revision_binding",
        )?;
        let target = if r.adopted_legacy {
            self.target(&r.registration)
        } else {
            self.revision_dir(key, &r.id)?
                .join(format!("LVB_{key}.vst3"))
        };
        require(
            r.target == target
                && r.registration.native.path
                    == target
                        .join("Contents/x86_64-linux")
                        .join(format!("LVB_{key}.so")),
            "publication_target_binding",
        )?;
        r.performance.verify()?;
        Ok(r)
    }
    pub fn load_revision(&self, key: &str, reference: &RevisionRef) -> Result<Revision> {
        let r = self.load_revision_record(key, reference)?;
        r.registration.native.verify()?;
        Ok(r)
    }
    /// Immutable revision/provenance/descriptor bindings. Executable bytes are
    /// deliberately left to load_revision and the mutation/launch owners.
    pub fn load_revision_record(&self, key: &str, reference: &RevisionRef) -> Result<Revision> {
        let r = self.revision_record(key, reference)?;
        r.registration.native.validate_record()?;
        r.registration.verify_descriptor()?;
        if !r.adopted_legacy {
            let provenance: Revision = Artifact { path:r.target.join("bridge-provenance.json"),
                sha256:reference.sha256.clone() }.read_record(8 * 1024 * 1024)?;
            require(provenance == r, "publication_provenance_changed")?;
        }
        r.performance.verify()?;
        Ok(r)
    }
    pub fn entry_target(&self, e: &Entry) -> Result<PathBuf> {
        if let Some(reference) = &e.managed_revision {
            let r = self.load_revision(&e.registration.key(), reference)?;
            require(
                r.registration == e.registration,
                "registry_revision_mismatch",
            )?;
            Ok(r.target)
        } else {
            Ok(self.target(&e.registration))
        }
    }
    fn entry_target_record(&self, e: &Entry) -> Result<PathBuf> {
        if let Some(reference) = &e.managed_revision {
            let r = self.load_revision_record(&e.registration.key(), reference)?;
            require(
                r.registration == e.registration,
                "registry_revision_mismatch",
            )?;
            Ok(r.target)
        } else {
            Ok(self.target(&e.registration))
        }
    }
    /// A rollback can retain an older exact Windows host while the environment
    /// keeper uses current product software. Only a complete retained managed
    /// revision with the reviewed protocol/state contract grants this route.
    /// Current policy always needs verified authority, including the same-host
    /// path. Retained candidate history is not a new policy selection.
    pub fn verify_served_host(
        &self,
        registration: &Registration,
        installed: &Artifact,
        source: &str,
        current_profiles: &[Profile],
    ) -> Result<()> {
        installed.verify()?;
        registration.host.verify()?;
        validate_set(current_profiles)?;
        let db = self.registry()?;
        let retained = db.classes.get(&registration.key()).and_then(|e| e.managed_revision.as_ref());
        if registration.key() == crate::pigments::candidate()?.class.class_id
            && retained.map(|r| self.load_revision(&registration.key(), r))
                .transpose()?.is_some_and(|r| r.qualification == Some(Qualification::Ap18Pigments)) {
            return crate::pigments::served(self, registration, installed, source);
        }
        if let Some(reference) = retained {
            let revision = self.load_revision(&registration.key(), reference)?;
            if revision.qualification == Some(Qualification::ManagedExperimental) || revision.profile.id.starts_with("managed.") || (revision.qualification.is_none() && crate::preparation::owns_profile(self, &revision.profile)?) {
                require(registration == &revision.registration, "managed_candidate_host_changed")?;
                return crate::preparation::retained(self, &revision);
            }
        }
        if registration.key() == crate::managed_candidate::candidate()?.class.class_id {
            return crate::managed_candidate::served(self, registration, installed, source);
        }
        if registration.key() == crate::frg1::candidate()?.class.class_id
            && retained
                .map(|reference| self.load_revision(&registration.key(), reference))
                .transpose()?
                .is_some_and(|revision| revision.qualification == Some(Qualification::Frg1Ubuntu))
        {
            return crate::frg1::served(self, registration);
        }
        let matching: Vec<_> = current_profiles
                .iter()
                .filter(|p| {
                    p.class.class_id == registration.key()
                        && p.claim.permits(SelectionPurpose::Activation)
                        && p.capabilities.state == State::ConcurrentReadOnlyCaptureV12
                })
                .collect();
        require(matching.len() == 1, "installed_host_mismatch")?;
        let current = crate::catalogue::current_host(self, installed, source, matching[0])?;
        if let Some(reference) = retained {
            let revision = self.load_revision(&registration.key(), reference)?;
            require(
                revision.registration == *registration,
                "installed_host_mismatch",
            )?;
            let roster = if revision.profile.claim == Claim::ReviewCandidate {
                revision
                    .qualification
                    .map(crate::qualification::candidates_for)
                    .transpose()?
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            self.verify_retained_authority(&revision, &roster)?;
        } else {
            require(
                registration.host.sha256 == current.host.sha256
                    && registration.host_source_sha256 == current.source_manifest.sha256,
                "installed_host_mismatch",
            )?;
        }
        Artifact {
            path: registration
                .host
                .path
                .with_file_name("host-source-manifest.json"),
            sha256: registration.host_source_sha256.clone(),
        }
        .verify()
    }
    fn pending_path(&self, key: &str) -> PathBuf {
        self.root
            .join("transactions")
            .join(format!("{key}.pending.json"))
    }
    pub fn publication_pending(&self, key: &str) -> Result<bool> {
        Ok(fs::symlink_metadata(self.pending_path(key)).is_ok())
    }
    fn package_publication_state(&self, key: &str, entry: &Entry) -> Result<PublicationState> {
        require(entry.registration.key() == key && entry.publication != Publication::Pending,
            "package_publication_state")?;
        let target = match entry.publication {
            Publication::Published => Some(self.entry_target(entry)?),
            Publication::Removed => None,
            Publication::Pending => unreachable!(),
        };
        require(physical(&self.link(key))? == target && !self.publication_pending(key)?,
            "package_publication_state")?;
        Ok(PublicationState { class_id:key.into(), entry:entry.clone(), target,
            performance:self.performance(key)? })
    }
    fn package_publication_record_state(
        &self,
        key: &str,
        entry: &Entry,
    ) -> Result<PublicationState> {
        require(
            entry.registration.key() == key && entry.publication != Publication::Pending,
            "package_publication_state",
        )?;
        let target = match entry.publication {
            Publication::Published => Some(self.entry_target_record(entry)?),
            Publication::Removed => None,
            Publication::Pending => unreachable!(),
        };
        require(
            physical(&self.link(key))? == target && !self.publication_pending(key)?,
            "package_publication_state",
        )?;
        Ok(PublicationState {
            class_id: key.into(),
            entry: entry.clone(),
            target,
            performance: self.performance(key)?,
        })
    }
    #[doc(hidden)]
    pub fn package_publication_snapshot(&self) -> Result<Vec<PublicationState>> {
        self.package_publication_snapshot_locked(&self.lock("registry.lock")?)
    }
    #[doc(hidden)]
    pub fn package_publication_snapshot_locked(&self, lock: &Lock) -> Result<Vec<PublicationState>> {
        lock.require_registry(self)?;
        let db = self.registry()?;
        require(db.classes.len() <= 256, "package_publication_bound")?;
        db.classes.iter().map(|(key, entry)| self.package_publication_state(key, entry)).collect()
    }
    /// Current registry/revision/link/performance control bindings without
    /// executable payload verification. Package status is its only authority.
    #[doc(hidden)]
    pub fn package_publication_record_snapshot(&self) -> Result<Vec<PublicationState>> {
        self.package_publication_record_snapshot_locked(&self.lock("registry.lock")?)
    }
    #[doc(hidden)]
    pub fn package_publication_record_snapshot_locked(&self, lock: &Lock) -> Result<Vec<PublicationState>> {
        lock.require_registry(self)?;
        let db = self.registry()?;
        require(db.classes.len() <= 256, "package_publication_bound")?;
        db.classes
            .iter()
            .map(|(key, entry)| self.package_publication_record_state(key, entry))
            .collect()
    }
    #[doc(hidden)]
    pub fn verify_package_publication_snapshot(&self,
        expected: &[PublicationState]) -> Result<()> {
        let lock = self.lock("registry.lock")?;
        self.verify_package_publication_snapshot_locked(expected, &lock)
    }
    #[doc(hidden)]
    pub fn verify_package_publication_snapshot_locked(&self,
        expected: &[PublicationState], lock: &Lock) -> Result<()> {
        lock.require_registry(self)?;
        require(expected.len() <= 256
            && expected.windows(2).all(|pair| pair[0].class_id < pair[1].class_id),
            "package_publication_order")?;
        let db = self.registry()?;
        let actual = db.classes.iter().map(|(key, entry)|
            self.package_publication_state(key, entry)).collect::<Result<Vec<_>>>()?;
        require(actual == expected,
            "package_publication_set_changed")
    }
    fn retain_profile(&self, profile: &Profile) -> Result<()> {
        let dir = self.root.join("profiles").join(&profile.id);
        self.durable_dir(&dir)?;
        immutable(&dir.join(format!("{}.json", profile.revision)), profile)
    }
    fn adopt_prior(
        &self,
        entry: &Entry,
        profile: &Profile,
        census: &Census,
        transaction: &str,
    ) -> Result<Prior> {
        let key = entry.registration.key();
        let target = self.entry_target(entry)?;
        let active = if entry.publication == Publication::Published {
            Some(target.clone())
        } else {
            None
        };
        require(
            physical(&self.link(&key))? == active,
            "foreign_or_missing_publication",
        )?;
        let reference = if let Some(reference) = &entry.managed_revision {
            reference.clone()
        } else {
            require(
                entry.publication != Publication::Pending,
                "legacy_publication_pending",
            )?;
            // The old target is checked in place and retained verbatim. It is
            // never rebuilt from a current input artifact during rollback.
            entry.registration.native.verify()?;
            entry.registration.verify_descriptor()?;
            let provenance: Registration = read_json(&target.join("bridge-provenance.json"))?;
            let mut normalized = provenance;
            normalized.relocate_native(entry.registration.native.path.clone());
            require(
                normalized == entry.registration,
                "legacy_provenance_mismatch",
            )?;
            require(
                entry.registration.metadata == profile.class
                    && entry.registration.module.sha256 == profile.module_sha256
                    && entry.registration.native.sha256 == profile.requirements.native_sha256
                    && entry.registration.host.sha256 == profile.requirements.host_sha256
                    && entry.registration.host_source_sha256
                        == profile.requirements.host_source_sha256
                    && entry.registration.compatibility == profile.capabilities.compatibility(),
                "legacy_profile_mismatch",
            )?;
            profile.verify_environment(
                &entry.registration.environment,
                &profile.requirements.environment_family,
            )?;
            let r = Revision {
                schema: 1,
                id: random_id()?,
                class_id: key.clone(),
                external_ids: external_ids(&key)?,
                profile: profile.clone(),
                profile_sha256: profile.fingerprint()?,
                census: census.clone(),
                registration: entry.registration.clone(),
                performance: self.performance(&key)?,
                target,
                parent: None,
                transaction: transaction.into(),
                adopted_legacy: true,
                qualification: None,
            };
            let dir = self.revision_dir(&key, &r.id)?;
            self.durable_dir(&dir)?;
            immutable(&dir.join("revision.json"), &r)?;
            Self::revision_ref(&r)?
        };
        Ok(Prior {
            entry: entry.clone(),
            revision: reference,
            target: active,
        })
    }
    fn write_intent(&self, intent: &Intent, fail: Option<Boundary>) -> Result<()> {
        let mutation = timing::span(Stage::IntentMutation);
        self.durable_dir(&self.root.join("transactions"))?;
        immutable(
            &self
                .root
                .join("transactions")
                .join(format!("{}.json", intent.id)),
            intent,
        )?;
        boundary(fail, Boundary::IntentArchived)?;
        let result = immutable(&self.pending_path(&intent.class_id), intent);
        mutation.end(result.is_ok());
        result
    }
    fn make_candidate(
        &self,
        r: &Revision,
        source: &Artifact,
        fail: Option<Boundary>,
    ) -> Result<()> {
        let mutation = timing::span(Stage::PublicationStageMutation);
        self.durable_dir(&self.revisions(&r.class_id))?;
        let stage = self.revisions(&r.class_id).join(format!(".stage-{}", r.id));
        private_dir(&stage)?;
        sync(stage.parent().unwrap())?;
        boundary(fail, Boundary::CandidateCreated)?;
        let bundle = stage.join(format!("LVB_{}.vst3", r.class_id));
        let binaries = bundle.join("Contents/x86_64-linux");
        private_dir(&binaries)?;
        let copy = binaries.join(format!("LVB_{}.so", r.class_id));
        fs::copy(&source.path, &copy)?;
        require(digest(&copy)? == source.sha256, "candidate_copy_changed")?;
        fs::set_permissions(&copy, fs::Permissions::from_mode(0o500))?;
        sync(&copy)?;
        copy_native_descriptor(&source.path, &copy, &r.registration.descriptor)?;
        sync(&binaries)?;
        boundary(fail, Boundary::NativeCopied)?;
        immutable(&bundle.join("bridge-provenance.json"), r)?;
        boundary(fail, Boundary::BundleProvenanceWritten)?;
        immutable(&stage.join("revision.json"), r)?;
        boundary(fail, Boundary::RevisionWritten)?;
        sync(&bundle.join("Contents"))?;
        sync(&bundle)?;
        sync(&stage)?;
        boundary(fail, Boundary::ProvenanceWritten)?;
        // Keep the candidate and prior record names immutable. Never overwrite
        // another revision, even if a random identity collision is observed.
        rename_link(&stage, &self.revision_dir(&r.class_id, &r.id)?, false)?;
        sync(&self.revisions(&r.class_id))?;
        self.load_revision(&r.class_id, &Self::revision_ref(r)?)?;
        let result = boundary(fail, Boundary::CandidateReady);
        mutation.end(result.is_ok());
        result
    }
    fn activate(&self, intent: &Intent, fail: Option<Boundary>) -> Result<()> {
        fs::create_dir_all(&self.publications)?;
        let meta = fs::symlink_metadata(&self.publications)?;
        require(
            meta.is_dir() && meta.uid() == unsafe { libc::getuid() },
            "publication_directory_owner",
        )?;
        let link = self.link(&intent.class_id);
        let prior = intent.prior.as_ref().and_then(|p| p.target.clone());
        require(physical(&link)? == prior, "foreign_publication")?;
        let candidate = intent
            .candidate
            .as_ref()
            .map(|r| self.load_revision(&intent.class_id, r))
            .transpose()?;
        let temp = self.publications.join(format!(".lvb-{}", intent.id));
        require(
            fs::symlink_metadata(&temp).is_err(),
            "transaction_pointer_occupied",
        )?;
        let mutation = timing::span(Stage::PointerMutation);
        if let Some(candidate) = candidate {
            symlink(&candidate.target, &temp)?;
            sync(&self.publications)?;
            boundary(fail, Boundary::PointerStaged)?;
            rename_link(&temp, &link, prior.is_some())?;
            if prior.is_some() && physical(&temp)? != prior {
                rename_link(&temp, &link, true)?;
                sync(&self.publications)?;
                return Err("foreign_publication_race".into());
            }
        } else {
            boundary(fail, Boundary::PointerStaged)?;
            require(prior.is_some(), "publication_already_absent")?;
            rename_link(&link, &temp, false)?;
            if physical(&temp)? != prior {
                rename_link(&temp, &link, false)?;
                sync(&self.publications)?;
                return Err("foreign_publication_race".into());
            }
        }
        boundary(fail, Boundary::PointerExchanged)?;
        sync(&self.publications)?;
        let result = boundary(fail, Boundary::PointerSynced);
        mutation.end(result.is_ok());
        result
    }
    fn finish(
        &self,
        db: &mut Registry,
        intent: &Intent,
        outcome: Outcome,
        fail: Option<Boundary>,
    ) -> Result<()> {
        if outcome == Outcome::Committed {
            if let Some(reference) = &intent.candidate {
                let r = self.load_revision(&intent.class_id, reference)?;
                db.classes.insert(
                    intent.class_id.clone(),
                    Entry {
                        registration: r.registration,
                        publication: Publication::Published,
                        managed_revision: Some(reference.clone()),
                    },
                );
            } else {
                let prior = intent.prior.as_ref().ok_or("unpublish_prior_absent")?;
                let mut e = prior.entry.clone();
                e.publication = Publication::Removed;
                e.managed_revision = Some(prior.revision.clone());
                db.classes.insert(intent.class_id.clone(), e);
            }
        } else if let Some(prior) = &intent.prior {
            db.classes
                .insert(intent.class_id.clone(), prior.entry.clone());
        } else {
            db.classes.remove(&intent.class_id);
        }
        let mutation = timing::span(Stage::RegistryMutation);
        self.save(db)?;
        boundary(fail, Boundary::RegistryCommitted)?;
        let complete = Completion {
            schema: 1,
            transaction: intent.id.clone(),
            class_id: intent.class_id.clone(),
            outcome,
        };
        let result = self
            .root
            .join("transactions")
            .join(format!("{}.result.json", intent.id));
        if result.try_exists()? && read_json::<Completion>(&result)?.outcome != complete.outcome {
            // A recovery after activation preserves the original completion.
            immutable(
                &self
                    .root
                    .join("transactions")
                    .join(format!("{}.recovery.json", intent.id)),
                &complete,
            )?;
        } else {
            immutable(&result, &complete)?;
        }
        boundary(fail, Boundary::ResultWritten)?;
        let temp = self.publications.join(format!(".lvb-{}", intent.id));
        if let Some(target) = physical(&temp)? {
            let prior = intent.prior.as_ref().and_then(|p| p.target.as_ref());
            require(
                Some(&target) == prior || intent.candidate_target.as_ref() == Some(&target),
                "foreign_transaction_pointer",
            )?;
            fs::remove_file(temp)?;
            sync(&self.publications)?;
        }
        fs::remove_file(self.pending_path(&intent.class_id))?;
        sync(&self.root.join("transactions"))?;
        let result = boundary(fail, Boundary::Cleanup);
        mutation.end(result.is_ok());
        result
    }
    fn read_intent(&self, path: &Path) -> Result<Intent> {
        let i: Intent = read_json(path)?;
        require(
            i.schema == 1 && valid_hex(&i.id, 32) && path == self.pending_path(&i.class_id),
            "transaction_identity",
        )?;
        let original = self
            .root
            .join("transactions")
            .join(format!("{}.json", i.id));
        require(
            digest(&original)? == digest(path)?,
            "transaction_intent_changed",
        )?;
        Ok(i)
    }
    pub(crate) fn pending_physical_revision(
        &self,
        key: &str,
        target: &Path,
    ) -> Result<Option<(RevisionRef, Revision)>> {
        let i = self.read_intent(&self.pending_path(key))?;
        require(i.class_id == key, "transaction_identity")?;
        for reference in [i.candidate.as_ref(), i.prior.as_ref().map(|p| &p.revision)]
            .into_iter()
            .flatten()
        {
            if let Ok(revision) = self.load_revision(key, reference) {
                if revision.target == target {
                    return Ok(Some((reference.clone(), revision)));
                }
            }
        }
        Ok(None)
    }
    /// Used under the existing registry lock by normal reconcile and admission.
    pub(crate) fn reconcile_revisions(&self, db: &mut Registry) -> Result<()> {
        let dir = self.root.join("transactions");
        if !dir.try_exists()? {
            return Ok(());
        }
        let mut count = 0;
        for item in fs::read_dir(&dir)? {
            let path = item?.path();
            if !path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.ends_with(".pending.json"))
            {
                continue;
            }
            count += 1;
            require(count <= 256, "pending_transaction_bound")?;
            let i = self.read_intent(&path)?;
            let actual = physical(&self.link(&i.class_id))?;
            let prior = i.prior.as_ref().and_then(|p| p.target.clone());
            if actual == prior {
                self.finish(db, &i, Outcome::Aborted, None)?;
            } else {
                require(actual == i.candidate_target, "foreign_publication")?;
                match i
                    .candidate
                    .as_ref()
                    .map(|r| self.load_revision(&i.class_id, r))
                    .transpose()
                {
                    Ok(candidate) => {
                        require(
                            candidate.as_ref().map(|r| r.target.clone()) == i.candidate_target,
                            "transaction_candidate_binding",
                        )?;
                        self.finish(db, &i, Outcome::Committed, None)?;
                    }
                    Err(_) => {
                        self.require_inactive(Some(&i.class_id))?;
                        if let Some(p) = &i.prior {
                            self.load_revision(&i.class_id, &p.revision)?;
                        }
                        let link = self.link(&i.class_id);
                        let temp = self.publications.join(format!(".lvb-{}", i.id));
                        if let Some(prior) = &prior {
                            if physical(&temp)?.is_none() {
                                symlink(prior, &temp)?;
                                sync(&self.publications)?;
                            }
                            require(
                                physical(&temp)?.as_ref() == Some(prior),
                                "foreign_transaction_pointer",
                            )?;
                            rename_link(&temp, &link, true)?;
                        } else {
                            require(physical(&temp)?.is_none(), "foreign_transaction_pointer")?;
                            rename_link(&link, &temp, false)?;
                        }
                        sync(&self.publications)?;
                        self.finish(db, &i, Outcome::Aborted, None)?;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn managed_publish(
        &self,
        profile: &Profile,
        census: &Census,
        registration: Registration,
        installed_host: &Artifact,
        source: &str,
        fail: Option<Boundary>,
    ) -> Result<RevisionRef> {
        profile.validate()?;
        // Refuse before lock/reconcile: even pending work must not be mutated
        // as a side effect of attempting to activate an unverified profile.
        profile.claim.require(SelectionPurpose::Activation)?;
        self.publish_selected(
            profile,
            census,
            registration,
            (installed_host, source),
            None,
            fail,
        )
    }
    #[doc(hidden)]
    pub fn prepare_package_refresh(
        &self,
        candidate: &crate::preparation::Candidate,
        expected: &RevisionRef,
    ) -> Result<PreparedTransition> {
        self.prepare_package_refresh_with_registry(candidate, expected,
            || self.lock("registry.lock"))
    }
    #[doc(hidden)]
    pub fn prepare_package_refresh_with_registry(
        &self,
        candidate: &crate::preparation::Candidate,
        expected: &RevisionRef,
        acquire_registry: impl FnOnce() -> Result<Lock>,
    ) -> Result<PreparedTransition> {
        let prior_revision = self.load_revision(&candidate.selection.class.id, expected)?;
        crate::preparation::verify_refresh_candidate(self, candidate, &prior_revision)?;
        require(candidate.profile.claim == Claim::ReviewCandidate
            && crate::preparation::build::supports_loaded_engine_admission(self, candidate)?,
            "loaded_engine_admission_contract_missing")?;
        let census = candidate.census()?;
        // verify_refresh_candidate has just fully verified this candidate's
        // runner and artifacts. Retain the census freshness/projection checks
        // and registration constraints without repeating that tree traversal.
        census.verify_current_artifacts(&self.root, &candidate.host,
            &candidate.source_manifest.sha256, crate::observation::now()?)?;
        let registration = crate::preparation::configuration::registration(candidate)?;
        registration.validate_record(&self.root)?;
        let key = registration.key();
        let lock = timing::measure(Stage::RegistryLock, acquire_registry)?;
        lock.require_registry(self)?;
        let db = self.registry()?;
        let entry = db.classes.get(&key).ok_or("registration_absent")?;
        require(entry.publication == Publication::Published
            && entry.managed_revision.as_ref() == Some(expected),
            "bridge_refresh_current_changed")?;
        let before = self.package_publication_state(&key, entry)?;
        require(prior_revision.registration.module == registration.module
            && prior_revision.registration.environment == registration.environment
            && prior_revision.registration.metadata == registration.metadata
            && prior_revision.registration.compatibility == registration.compatibility
            && prior_revision.external_ids == candidate.native.external_ids,
            "bridge_refresh_configuration_changed")?;
        require(before.performance.added_frames != 1024
            || crate::preparation::build::candidate_maximum_bridge_frames(self, candidate)?
                == Some(1024),
            "bridge_refresh_buffering_unsupported")?;
        require(before.performance.delivery_mode != DeliveryMode::SameCallback
            || crate::preparation::build::candidate_supports_audio_completion(self, candidate)?,
            "bridge_refresh_delivery_unsupported")?;
        self.retain_profile(&candidate.profile)?;
        let transaction = random_id()?;
        let prior = self.adopt_prior(entry, &candidate.profile, &census, &transaction)?;
        require(prior.revision == *expected, "bridge_refresh_current_changed")?;
        let id = random_id()?;
        let target = self.revision_dir(&key, &id)?
            .join(format!("LVB_{key}.vst3"));
        let source = registration.native.clone();
        let mut installed = registration;
        installed.relocate_native(target.join("Contents/x86_64-linux")
            .join(format!("LVB_{key}.so")));
        let revision = Revision {
            schema:1, id, class_id:key.clone(), external_ids:external_ids(&key)?,
            profile:candidate.profile.clone(), profile_sha256:candidate.profile.fingerprint()?,
            census, registration:installed.clone(), performance:before.performance.clone(),
            target:target.clone(), parent:Some(expected.clone()), transaction:transaction.clone(),
            adopted_legacy:false, qualification:Some(Qualification::ManagedExperimental),
        };
        let reference = Self::revision_ref(&revision)?;
        let intent = Intent { schema:1, id:transaction, class_id:key.clone(),
            prior:Some(prior), candidate:Some(reference.clone()),
            candidate_target:Some(target.clone()) };
        self.make_candidate(&revision, &source, None)?;
        let after = PublicationState { class_id:key,
            entry:Entry { registration:installed, publication:Publication::Published,
                managed_revision:Some(reference) },
            target:Some(target), performance:before.performance.clone() };
        let reverse_intent = Intent { schema:1, id:random_id()?,
            class_id:after.class_id.clone(),
            prior:Some(Prior {
                entry:after.entry.clone(),
                revision:after.entry.managed_revision.clone()
                    .ok_or("package_publication_transition_changed")?,
                target:after.target.clone(),
            }),
            candidate:before.entry.managed_revision.clone(),
            candidate_target:before.target.clone(),
        };
        Ok(PreparedTransition { schema:2, before, after, intent, reverse_intent })
    }
    #[doc(hidden)]
    pub fn package_transition_states(t: &PreparedTransition)
        -> (&PublicationState, &PublicationState) {
        (&t.before, &t.after)
    }
    fn verify_prepared_transition(&self, transition: &PreparedTransition) -> Result<()> {
        let t = transition;
        require(t.schema == 2 && t.before.class_id == t.after.class_id
            && t.intent.class_id == t.before.class_id
            && t.intent.prior.as_ref().map(|prior| &prior.entry) == Some(&t.before.entry)
            && t.intent.prior.as_ref().and_then(|prior| prior.target.as_ref()) == t.before.target.as_ref()
            && t.intent.candidate.as_ref() == t.after.entry.managed_revision.as_ref()
            && t.intent.candidate_target.as_ref() == t.after.target.as_ref()
            && t.reverse_intent.schema == 1
            && valid_hex(&t.reverse_intent.id, 32)
            && t.reverse_intent.class_id == t.before.class_id
            && t.reverse_intent.id != t.intent.id
            && t.reverse_intent.prior.as_ref().map(|prior| &prior.entry)
                == Some(&t.after.entry)
            && t.reverse_intent.prior.as_ref().and_then(|prior| prior.target.as_ref())
                == t.after.target.as_ref()
            && t.reverse_intent.prior.as_ref().map(|prior| &prior.revision)
                == t.after.entry.managed_revision.as_ref()
            && t.reverse_intent.candidate.as_ref()
                == t.before.entry.managed_revision.as_ref()
            && t.reverse_intent.candidate_target.as_ref() == t.before.target.as_ref()
            && t.before.performance == t.after.performance,
            "package_publication_transition_changed")?;
        let reference = t.after.entry.managed_revision.as_ref()
            .ok_or("package_publication_transition_changed")?;
        let revision = self.load_revision(&t.after.class_id, reference)?;
        require(revision.registration == t.after.entry.registration
            && Some(&revision.target) == t.after.target.as_ref()
            && revision.performance == t.after.performance
            && revision.transaction == t.intent.id,
            "package_publication_transition_changed")
    }
    #[doc(hidden)]
    pub fn verify_package_pending_intents(&self,
        transitions: &[PreparedTransition]) -> Result<()> {
        for transition in transitions {
            self.verify_prepared_transition(transition)?;
        }
        let directory = self.root.join("transactions");
        if !directory.try_exists()? { return Ok(()); }
        let mut count = 0usize;
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if !path.file_name().and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".pending.json")) {
                continue;
            }
            count += 1;
            require(count <= transitions.len(),
                "package_transaction_pending")?;
            let intent = self.read_intent(&path)?;
            let owned = transitions.iter().any(|transition|
                intent == transition.intent || intent == transition.reverse_intent);
            require(owned, "package_transaction_pending")?;
        }
        Ok(())
    }
    #[doc(hidden)]
    pub fn reconcile_package_pending_intent(&self,
        transition: &PreparedTransition) -> Result<()> {
        self.reconcile_package_pending_intent_with_registry(transition, || self.lock("registry.lock"))
    }
    #[doc(hidden)]
    pub fn reconcile_package_pending_intent_with_registry(&self,
        transition: &PreparedTransition, acquire_registry: impl FnOnce() -> Result<Lock>) -> Result<()> {
        self.verify_prepared_transition(transition)?;
        let key = &transition.before.class_id;
        let lock = acquire_registry()?;
        lock.require_registry(self)?;
        let pending = self.pending_path(key);
        if !pending.try_exists()? { return Ok(()); }
        let intent = self.read_intent(&pending)?;
        require(intent == transition.intent || intent == transition.reverse_intent,
            "package_publication_transaction_changed")?;
        let mut db = self.registry()?;
        let actual = physical(&self.link(key))?;
        let prior = intent.prior.as_ref().and_then(|value| value.target.clone());
        if actual == prior {
            self.finish(&mut db, &intent, Outcome::Aborted, None)
        } else {
            require(actual == intent.candidate_target,
                "foreign_publication")?;
            self.finish(&mut db, &intent, Outcome::Committed, None)
        }
    }
    #[doc(hidden)]
    pub fn commit_package_publication(&self,
        transition: &PreparedTransition) -> Result<()> {
        self.commit_package_publication_with_registry(transition, || self.lock("registry.lock"))
    }
    #[doc(hidden)]
    pub fn commit_package_publication_with_registry(&self,
        transition: &PreparedTransition, mut acquire_registry: impl FnMut() -> Result<Lock>) -> Result<()> {
        self.reconcile_package_pending_intent_with_registry(transition, &mut acquire_registry)?;
        let lock = acquire_registry()?;
        lock.require_registry(self)?;
        self.verify_prepared_transition(transition)?;
        let current = self.package_publication_state(&transition.before.class_id,
            self.registry()?.classes.get(&transition.before.class_id)
                .ok_or("package_publication_set_changed")?)?;
        if current == transition.after { return Ok(()); }
        require(current == transition.before, "package_publication_set_changed")?;
        let mut db = self.registry()?;
        self.write_intent(&transition.intent, None)?;
        self.activate(&transition.intent, None)?;
        self.finish(&mut db, &transition.intent, Outcome::Committed, None)?;
        require(self.package_publication_state(&transition.after.class_id,
            self.registry()?.classes.get(&transition.after.class_id)
                .ok_or("package_publication_set_changed")?)? == transition.after,
            "package_publication_set_changed")
    }
    #[doc(hidden)]
    pub fn restore_package_publication(&self,
        transition: &PreparedTransition) -> Result<()> {
        self.restore_package_publication_with_registry(transition, || self.lock("registry.lock"))
    }
    #[doc(hidden)]
    pub fn restore_package_publication_with_registry(&self,
        transition: &PreparedTransition, mut acquire_registry: impl FnMut() -> Result<Lock>) -> Result<()> {
        self.verify_prepared_transition(transition)?;
        let key = &transition.before.class_id;
        self.reconcile_package_pending_intent_with_registry(transition, &mut acquire_registry)?;
        let lock = acquire_registry()?;
        lock.require_registry(self)?;
        let current = self.registry()?.classes.get(key)
            .ok_or("package_publication_set_changed")?.clone();
        let state = self.package_publication_state(key, &current)?;
        if state == transition.before { return Ok(()); }
        require(state == transition.after, "package_publication_set_changed")?;
        self.require_inactive(Some(key))?;
        let mut db = self.registry()?;
        self.write_intent(&transition.reverse_intent, None)?;
        self.activate(&transition.reverse_intent, None)?;
        self.finish(&mut db, &transition.reverse_intent, Outcome::Committed, None)?;
        require(self.package_publication_state(key,
            self.registry()?.classes.get(key).ok_or("package_publication_set_changed")?)?
            == transition.before, "package_publication_set_changed")
    }
    /// MF1's global inactivity law is checked again under the publication lock.
    pub fn managed_publish_inactive(
        &self,
        profile: &Profile,
        census: &Census,
        registration: Registration,
        installed_host: &Artifact,
        source: &str,
        fail: Option<Boundary>,
    ) -> Result<RevisionRef> {
        profile.validate()?;
        profile.claim.require(SelectionPurpose::Activation)?;
        self.publish_with_scope(
            profile, census, registration, (installed_host, source), (None, true), fail,
        )
    }
    // Only the sealed AP15 qualification owner may choose this purpose. The
    // ordinary public method above retains its verified-only gate.
    pub(crate) fn publish_selected(
        &self,
        profile: &Profile,
        census: &Census,
        registration: Registration,
        host: (&Artifact, &str),
        qualification: Option<Qualification>,
        fail: Option<Boundary>,
    ) -> Result<RevisionRef> {
        self.publish_with_scope(profile, census, registration, host, (qualification, false), fail)
    }
    pub(crate) fn publish_with_scope(
        &self,
        profile: &Profile,
        census: &Census,
        registration: Registration,
        host: (&Artifact, &str),
        scope: (Option<Qualification>, bool),
        fail: Option<Boundary>,
    ) -> Result<RevisionRef> {
        self.publish_with_expected(profile,census,registration,host,scope,fail,None)
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn publish_with_expected(
        &self,profile:&Profile,census:&Census,registration:Registration,
        host:(&Artifact,&str),scope:(Option<Qualification>,bool),fail:Option<Boundary>,
        expected:Option<&RevisionRef>,
    )->Result<RevisionRef> {
        self.publish_with_expected_registry(profile,census,registration,host,scope,fail,
            expected,|| self.lock("registry.lock"))
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn publish_with_expected_registry(
        &self,profile:&Profile,census:&Census,registration:Registration,
        host:(&Artifact,&str),scope:(Option<Qualification>,bool),fail:Option<Boundary>,
        expected:Option<&RevisionRef>,mut acquire_registry:impl FnMut()->Result<Lock>,
    )->Result<RevisionRef> {
        require(!self.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        let validation = timing::span(Stage::PublicationValidation);
        let (qualification, global_inactive) = scope;
        let (installed_host, source) = host;
        let managed = qualification == Some(Qualification::ManagedExperimental) || crate::preparation::owns_profile(self,profile)?;
        if managed {crate::preparation::check_publication(self,profile,&registration)?;}
        #[cfg(test)]
        let enforce_loaded_engine = self.root.join("test-loaded-engine-enforcement").exists();
        #[cfg(not(test))]
        let enforce_loaded_engine = true;
        if managed && enforce_loaded_engine {
            require(crate::preparation::build::publication_supports_loaded_engine_admission(
                self, profile, &registration)?,
                "bridge_update_required: use Update Bridge")?;
        }
        // Digest and inspection work precede registry admission. Immutable inputs
        // are rechecked outside the guard again before the short commit below.
        census.verify_current(&self.root,installed_host,source,crate::observation::now()?)?;
        registration.verify(&self.root)?;
        let key = registration.key();
        let performance = self.performance(&key)?;
        require(performance.delivery_mode != DeliveryMode::SameCallback || managed
            && crate::preparation::build::publication_supports_audio_completion(self, profile, &registration)?,
            "publication_delivery_unsupported_by_target: Select buffered delivery before selecting this older publication.")?;
        let admitted = timing::measure(Stage::RegistryLock, &mut acquire_registry)?;
        admitted.require_registry(self)?;
        let mut guard = Some(admitted);
        require(self.performance(&key)? == performance, "publication_performance_changed")?;
        self.require_inactive(if global_inactive { None } else { Some(&key) })?;
        let mut db = self.registry()?;
        if let Some(expected)=expected {
            require(db.classes.get(&key).and_then(|e|e.managed_revision.as_ref())==Some(expected)
                && db.classes[&key].publication==Publication::Published && !self.publication_pending(&key)?,"replacement_current_changed")?;
            let prior=self.load_revision(&key,expected)?;
            require(physical(&self.link(&key))?==Some(prior.target),"replacement_physical_changed")?;
        }
        if managed && qualification.is_none() {
            if let Some(e) = db.classes.get(&key).filter(|e| e.publication == Publication::Published) {
                if let Some(current) = &e.managed_revision {
                    let prior = self.load_revision(&key, current)?;
                    require(prior.profile != *profile || prior.qualification.is_some(), "candidate_already_ordinary")?;
                }
            }
        }
        timing::measure(Stage::PublicationRecovery, || self.reconcile_revisions(&mut db))?;
        let purpose = if qualification.is_some() {
            SelectionPurpose::Qualification
        } else {
            SelectionPurpose::Activation
        };
        crate::observation::select_for(std::slice::from_ref(profile), census, purpose)?;
        if let Some(current) = db
            .classes
            .get(&key)
            .and_then(|e| e.managed_revision.as_ref())
        {
            let prior = self.load_revision(&key, current)?;
            require(
                prior.qualification.is_none()
                    || (managed && matches!(prior.qualification,Some(Qualification::ManagedExperimental | Qualification::Sv1Instrument)) && (expected.is_some() || db.classes[&key].publication==Publication::Removed))
                    || crate::preparation::permits_transition(self, profile, &prior, qualification)?
                    || (qualification.is_none() && db.classes[&key].publication == Publication::Removed
                        && crate::acceptance::pigments::accepted_predecessor(self, profile, &prior)?)
                    || (qualification == Some(Qualification::Ap18Pigments)
                        && prior.qualification == qualification && crate::pigments::replacement_prior(&prior.profile, profile)?
                        && db.classes[&key].publication == Publication::Removed)
                    || (qualification == Some(Qualification::Frg1Ubuntu)
                        && crate::frg1::permits_retired_predecessor(self, &db.classes[&key], &prior)?),
                "qualification_active_restore_first",
            )?;
        }
        if let Some(purpose) = qualification {
            if purpose == Qualification::ManagedExperimental {
                require(managed,"candidate_preparation_required")?;
            } else if purpose == Qualification::Ap18Pigments {
                crate::pigments::check_publication(self, profile, &registration)?;
            } else if purpose == Qualification::Sv1Instrument {
                crate::managed_candidate::check_publication(self, profile, &registration)?;
            } else if purpose == Qualification::Frg1Ubuntu {
                crate::frg1::check_publication(self, profile, census, &registration)?;
            } else {
                self.verify_qualification_parent_for(&db, profile, &registration, purpose)?;
            }
        }
        require(
            registration.metadata == census.selected
                && registration.module == census.module
                && registration.environment == census.environment.environment
                && registration.host == census.host
                && registration.host_source_sha256 == census.host_source_sha256
                && registration.native.sha256 == profile.requirements.native_sha256
                && registration.compatibility == profile.capabilities.compatibility(),
            "derived_registration_mismatch",
        )?;
        if let Some(e) = db.classes.get(&key) {
            if let Some(reference) = &e.managed_revision {
                let r = self.load_revision(&key, reference)?;
                let mut old = r.registration.clone();
                old.native = registration.native.clone();
                old.descriptor = registration.descriptor.clone();
                old.host = registration.host.clone();
                if fail.is_none()
                    && e.publication == Publication::Published
                    && r.profile == *profile
                    && old == registration
                    && r.performance == performance
                {
                    require(
                        physical(&self.link(&key))? == Some(r.target),
                        "foreign_or_missing_publication",
                    )?;
                    validation.end(true);
                    return Ok(reference.clone());
                }
            }
        }
        validation.end(true);
        let retained_mutation = timing::span(Stage::PublicationRetainMutation);
        self.retain_profile(profile)?;
        boundary(fail, Boundary::ProfileRetained)?;
        let transaction = random_id()?;
        let prior = db
            .classes
            .get(&key)
            .map(|e| self.adopt_prior(e, profile, census, &transaction))
            .transpose()?;
        boundary(fail, Boundary::PriorRetained)?;
        if prior.is_none() {
            require(physical(&self.link(&key))?.is_none(), "foreign_publication")?;
        }
        retained_mutation.end(true);
        let id = random_id()?;
        let target = self
            .revision_dir(&key, &id)?
            .join(format!("LVB_{key}.vst3"));
        let source_artifact = registration.native.clone();
        let mut installed = registration;
        installed.relocate_native(target
            .join("Contents/x86_64-linux")
            .join(format!("LVB_{key}.so")));
        let r = Revision {
            schema: 1,
            id,
            class_id: key.clone(),
            external_ids: external_ids(&key)?,
            profile: profile.clone(),
            profile_sha256: profile.fingerprint()?,
            census: census.clone(),
            registration: installed,
            performance,
            target,
            parent: prior.as_ref().map(|p| p.revision.clone()),
            transaction: transaction.clone(),
            adopted_legacy: false,
            qualification,
        };
        let reference = Self::revision_ref(&r)?;
        let intent = Intent {
            schema: 1,
            id: transaction,
            class_id: key,
            prior,
            candidate: Some(reference.clone()),
            candidate_target: Some(r.target.clone()),
        };
        if managed {
            let snapshot=serde_json::to_vec(&db)?;
            drop(guard.take());
            // No physical pointer or transaction is exposed during package copy.
            // A failed/interrupted copy cannot authorize service admission.
            self.make_candidate(&r,&source_artifact,fail)?;
            let authority = timing::span(Stage::PublicationFinalAuthority);
            census.verify_current(&self.root,installed_host,source,crate::observation::now()?)?;
            source_artifact.verify()?;
            let committed = timing::measure(Stage::RegistryLock, &mut acquire_registry)?;
            committed.require_registry(self)?;
            guard=Some(committed);
            self.require_inactive(if global_inactive { None } else { Some(&r.class_id) })?;
            require(self.performance(&r.class_id)? == r.performance, "publication_performance_changed")?;
            require(serde_json::to_vec(&self.registry()?)?==snapshot && !self.publication_pending(&r.class_id)?
                && physical(&self.link(&r.class_id))?==intent.prior.as_ref().and_then(|p|p.target.clone()),"preparation_publication_state_changed")?;
            authority.end(true);
            self.write_intent(&intent,fail)?;
            boundary(fail,Boundary::Intent)?;
        } else {
            self.write_intent(&intent, fail)?;
            boundary(fail, Boundary::Intent)?;
            self.make_candidate(&r, &source_artifact, fail)?;
        }
        timing::measure(Stage::PublicationMutation, || {
            self.activate(&intent, fail)?;
            self.finish(&mut db, &intent, Outcome::Committed, fail)
        })?;
        drop(guard);
        Ok(reference)
    }
    pub fn rollback(&self, key: &str, id: &str, fail: Option<Boundary>) -> Result<RevisionRef> {
        self.rollback_expected(key, id, fail, None)
    }
    pub(crate) fn rollback_expected(
        &self,
        key: &str,
        id: &str,
        fail: Option<Boundary>,
        expected: Option<&RevisionRef>,
    ) -> Result<RevisionRef> {
        self.rollback_with_scope(key, id, fail, expected, false, false)
    }
    #[doc(hidden)]
    pub fn rollback_managed_expected(&self, key: &str, id: &str,
        expected: &RevisionRef) -> Result<RevisionRef> {
        self.rollback_expected(key, id, None, Some(expected))
    }
    pub fn rollback_inactive(&self, key: &str, id: &str, fail: Option<Boundary>) -> Result<RevisionRef> {
        self.rollback_with_scope(key, id, fail, None, true, false)
    }
    pub(crate) fn rollback_exact(&self, key: &str, id: &str, expected: &RevisionRef) -> Result<RevisionRef> {
        self.rollback_with_scope(key, id, None, Some(expected), false, false)
    }
    fn rollback_with_scope(
        &self,
        key: &str,
        id: &str,
        fail: Option<Boundary>,
        expected: Option<&RevisionRef>,
        global_inactive: bool,
        package_transition: bool,
    ) -> Result<RevisionRef> {
        require(package_transition || !self.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        let authority = timing::span(Stage::RestoreAuthority);
        require(valid_hex(key, 32) && valid_hex(id, 32), "rollback_identity")?;
        let _lock = timing::measure(Stage::RegistryLock, || self.lock("registry.lock"))?;
        self.require_inactive(if global_inactive { None } else { Some(key) })?;
        let mut db = self.registry()?;
        timing::measure(Stage::PublicationRecovery, || self.reconcile_revisions(&mut db))?;
        let e = db.classes.get(key).ok_or("registration_absent")?.clone();
        let current = e.managed_revision.as_ref().ok_or("no_managed_revision")?;
        require(
            expected.is_none_or(|expected| expected == current),
            "qualification_publication_changed",
        )?;
        let mut reference = Some(current.clone());
        let mut selected = None;
        for _ in 0..256 {
            let Some(candidate) = reference else {
                break;
            };
            let r = self.revision_record(key, &candidate)?;
            if r.id == id {
                selected = Some((candidate, r));
                break;
            }
            reference = r.parent;
        }
        let (selected, r) = selected.ok_or("rollback_revision_not_retained_ancestor")?;
        timing::measure(Stage::CandidateVerification, || r.registration.verify(&self.root))?;
        #[cfg(test)]
        let enforce_loaded_engine = self.root.join("test-loaded-engine-enforcement").exists();
        #[cfg(not(test))]
        let enforce_loaded_engine = true;
        if !package_transition && enforce_loaded_engine {
            let modern = crate::preparation::publication_candidate(
                self, &r.profile, &r.registration).and_then(|candidate|
                    crate::preparation::build::supports_loaded_engine_admission(self, &candidate))
                .unwrap_or(false);
            require(modern,
                "bridge_update_required: use Update Bridge or Restore previous setup")?;
        }
        // The revision retains the publication-time preference as evidence.
        // Explicit buffering changes are independently owned class settings;
        // restoring a publication preserves them, subject to the target's
        // actual capacity, rather than demanding the historical value.
        let performance = self.performance(key)?;
        require(performance.added_frames != 1024
            || preparation::build::revision_maximum_bridge_frames(self, &r)? == Some(1024),
            "rollback_buffering_unsupported_by_target")?;
        require(performance.delivery_mode != DeliveryMode::SameCallback
            || preparation::build::revision_supports_audio_completion(self, &r)?,
            "rollback_delivery_unsupported_by_target: Select buffered delivery before restoring this older publication.")?;
        if e.publication == Publication::Published && current == &selected {
            require(
                physical(&self.link(key))? == Some(r.target),
                "foreign_or_missing_publication",
            )?;
            authority.end(true);
            return Ok(selected);
        }
        let transaction = random_id()?;
        let prior = self.adopt_prior(&e, &r.profile, &r.census, &transaction)?;
        let intent = Intent {
            schema: 1,
            id: transaction,
            class_id: key.into(),
            prior: Some(prior),
            candidate: Some(selected.clone()),
            candidate_target: Some(r.target.clone()),
        };
        authority.end(true);
        timing::measure(Stage::RestoreMutation, || {
            self.write_intent(&intent, fail)?;
            boundary(fail, Boundary::Intent)?;
            self.activate(&intent, fail)?;
            self.finish(&mut db, &intent, Outcome::Committed, fail)
        })?;
        Ok(selected)
    }
    pub(crate) fn remove_revision(
        &self,
        db: &mut Registry,
        key: &str,
        fail: Option<Boundary>,
    ) -> Result<()> {
        require(!self.root.join("package-transition.json").try_exists()?,
            "package_transition_needs_recovery")?;
        self.require_inactive(Some(key))?;
        self.reconcile_revisions(db)?;
        let e = db.classes.get(key).ok_or("registration_absent")?.clone();
        if e.publication == Publication::Removed {
            return require(physical(&self.link(key))?.is_none(), "foreign_publication");
        }
        let r = self.load_revision(
            key,
            e.managed_revision.as_ref().ok_or("no_managed_revision")?,
        )?;
        let transaction = random_id()?;
        let prior = self.adopt_prior(&e, &r.profile, &r.census, &transaction)?;
        let intent = Intent {
            schema: 1,
            id: transaction,
            class_id: key.into(),
            prior: Some(prior),
            candidate: None,
            candidate_target: None,
        };
        self.write_intent(&intent, fail)?;
        boundary(fail, Boundary::Intent)?;
        self.activate(&intent, fail)?;
        self.finish(db, &intent, Outcome::Committed, fail)
    }
}

/// Materialize pre-R1 immutable history with the existing record/bundle writers.
/// Test-only: no production entry point bypasses new-selection eligibility.
#[cfg(test)]
pub(crate) fn retained_candidate_fixture(
    m: &Manager,
    profile: &Profile,
    census: &Census,
    registration: Registration,
) -> RevisionRef {
    assert_eq!(profile.claim, Claim::ReviewCandidate);
    let mut db = m.registry().unwrap();
    let key = registration.key();
    let transaction = random_id().unwrap();
    m.retain_profile(profile).unwrap();
    let prior = m
        .adopt_prior(&db.classes[&key], profile, census, &transaction)
        .unwrap();
    let id = random_id().unwrap();
    let target = m
        .revision_dir(&key, &id)
        .unwrap()
        .join(format!("LVB_{key}.vst3"));
    let source = registration.native.clone();
    let mut registration = registration;
    registration.relocate_native(target
        .join("Contents/x86_64-linux")
        .join(format!("LVB_{key}.so")));
    let r = Revision {
        schema: 1,
        id,
        class_id: key.clone(),
        external_ids: external_ids(&key).unwrap(),
        profile: profile.clone(),
        profile_sha256: profile.fingerprint().unwrap(),
        census: census.clone(),
        registration,
        performance: m.performance(&key).unwrap(),
        target,
        parent: Some(prior.revision.clone()),
        transaction: transaction.clone(),
        adopted_legacy: false,
        qualification: None,
    };
    let reference = Manager::revision_ref(&r).unwrap();
    let intent = Intent {
        schema: 1,
        id: transaction,
        class_id: key,
        prior: Some(prior),
        candidate: Some(reference.clone()),
        candidate_target: Some(r.target.clone()),
    };
    m.write_intent(&intent, None).unwrap();
    m.make_candidate(&r, &source, None).unwrap();
    m.activate(&intent, None).unwrap();
    m.finish(&mut db, &intent, Outcome::Committed, None)
        .unwrap();
    reference
}

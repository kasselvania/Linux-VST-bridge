//! Revisioned transactions around the existing registry and discovery links.
//! Intent and immutable records precede activation; physical links decide recovery.
use crate::{observation::Census, profiles::*, *};
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
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Prior {
    entry: Entry,
    revision: RevisionRef,
    target: Option<PathBuf>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
fn boundary(fail: Option<Boundary>, here: Boundary) -> Result<()> {
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
pub(crate) fn physical(link: &Path) -> Result<Option<PathBuf>> {
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
fn rename_link(from: &Path, to: &Path, exchange: bool) -> Result<()> {
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
        require(
            digest(&path)? == reference.sha256,
            "publication_revision_changed",
        )?;
        let r: Revision = read_json(&path)?;
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
        let r = self.revision_record(key, reference)?;
        r.registration.native.verify()?;
        if !r.adopted_legacy {
            require(
                digest(&r.target.join("bridge-provenance.json"))? == reference.sha256,
                "publication_provenance_changed",
            )?;
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
    fn pending_path(&self, key: &str) -> PathBuf {
        self.root
            .join("transactions")
            .join(format!("{key}.pending.json"))
    }
    pub fn publication_pending(&self, key: &str) -> Result<bool> {
        Ok(fs::symlink_metadata(self.pending_path(key)).is_ok())
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
            let provenance: Registration = read_json(&target.join("bridge-provenance.json"))?;
            let mut normalized = provenance;
            normalized.native.path = entry.registration.native.path.clone();
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
        self.durable_dir(&self.root.join("transactions"))?;
        immutable(
            &self
                .root
                .join("transactions")
                .join(format!("{}.json", intent.id)),
            intent,
        )?;
        boundary(fail, Boundary::IntentArchived)?;
        immutable(&self.pending_path(&intent.class_id), intent)
    }
    fn make_candidate(
        &self,
        r: &Revision,
        source: &Artifact,
        fail: Option<Boundary>,
    ) -> Result<()> {
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
        boundary(fail, Boundary::CandidateReady)
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
        boundary(fail, Boundary::PointerSynced)
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
        boundary(fail, Boundary::Cleanup)
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
        let _lock = self.lock("registry.lock")?;
        let key = registration.key();
        self.require_inactive(Some(&key))?;
        let mut db = self.registry()?;
        self.reconcile_revisions(&mut db)?;
        census.verify_current(
            &self.root,
            installed_host,
            source,
            crate::observation::now()?,
        )?;
        crate::observation::select(std::slice::from_ref(profile), census)?;
        registration.verify(&self.root)?;
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
        let performance = self.performance(&key)?;
        if let Some(e) = db.classes.get(&key) {
            if let Some(reference) = &e.managed_revision {
                let r = self.load_revision(&key, reference)?;
                let mut old = r.registration.clone();
                old.native = registration.native.clone();
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
                    return Ok(reference.clone());
                }
            }
        }
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
        let id = random_id()?;
        let target = self
            .revision_dir(&key, &id)?
            .join(format!("LVB_{key}.vst3"));
        let source_artifact = registration.native.clone();
        let mut installed = registration;
        installed.native.path = target
            .join("Contents/x86_64-linux")
            .join(format!("LVB_{key}.so"));
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
        self.write_intent(&intent, fail)?;
        boundary(fail, Boundary::Intent)?;
        self.make_candidate(&r, &source_artifact, fail)?;
        self.activate(&intent, fail)?;
        self.finish(&mut db, &intent, Outcome::Committed, fail)?;
        Ok(reference)
    }
    pub fn rollback(&self, key: &str, id: &str, fail: Option<Boundary>) -> Result<RevisionRef> {
        require(valid_hex(key, 32) && valid_hex(id, 32), "rollback_identity")?;
        let _lock = self.lock("registry.lock")?;
        self.require_inactive(Some(key))?;
        let mut db = self.registry()?;
        self.reconcile_revisions(&mut db)?;
        let e = db.classes.get(key).ok_or("registration_absent")?.clone();
        let current = e.managed_revision.as_ref().ok_or("no_managed_revision")?;
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
        r.registration.verify(&self.root)?;
        require(
            self.performance(key)? == r.performance,
            "rollback_performance_mismatch",
        )?;
        if e.publication == Publication::Published && current == &selected {
            require(
                physical(&self.link(key))? == Some(r.target),
                "foreign_or_missing_publication",
            )?;
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
        self.write_intent(&intent, fail)?;
        boundary(fail, Boundary::Intent)?;
        self.activate(&intent, fail)?;
        self.finish(&mut db, &intent, Outcome::Committed, fail)?;
        Ok(selected)
    }
    pub(crate) fn remove_revision(
        &self,
        db: &mut Registry,
        key: &str,
        fail: Option<Boundary>,
    ) -> Result<()> {
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

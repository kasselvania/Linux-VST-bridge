//! Retained history of setups the musician deleted. Deleting a setup removes
//! its Windows files; its installer, operation and onboarding records stay
//! where they are, marked by one tombstone. Readers skip marked environments.
use crate::*;

pub const DIRECTORY: &str = "deleted-setups";
const BOUND: usize = 128;
/// New deletions stop here; reading tolerates more so a full history never
/// hides a deleted setup from the readers that skip it.
pub const LIMIT: usize = 128;
const READ_BOUND: usize = 1024;
pub const HISTORY_FULL: &str = "The manager keeps the history of up to 128 deleted setups, and that history is full. Nothing was deleted. Send Setup details.";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Records are marked; Windows files may still be partly present.
    Deleting,
    Deleted,
}

/// A registry entry retired with its setup. It was already removed from the
/// DAW (no link, no pending transaction); its revision records stay in place.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetiredRegistration {
    pub class_id: String,
    pub name: String,
    pub entry: Entry,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Tombstone {
    pub schema: u32,
    /// The setup's current environment, also this record's file name.
    pub environment: String,
    pub installer: String,
    pub name: String,
    /// Every attempt environment of the setup, as recorded at deletion.
    pub environments: Vec<Environment>,
    pub registrations: Vec<RetiredRegistration>,
    pub plugins: Vec<String>,
    pub operation: String,
    pub requested_at: u64,
    pub deleted_at: Option<u64>,
    pub state: State,
}

impl Tombstone {
    fn validate(&self, m: &Manager) -> Result<()> {
        require(self.schema == 1
            && environment_revision::valid_environment_id(&self.environment)
            && valid_hex(&self.installer, 64)
            && valid_hex(&self.operation, 32)
            && !self.environments.is_empty() && self.environments.len() <= BOUND
            && self.environments.iter().any(|e| e.id == self.environment)
            && self.environments.iter().all(|e| environment_revision::valid_environment_id(&e.id)
                && e.root == m.root.join("environments").join(&e.id))
            && self.registrations.iter().all(|r| valid_hex(&r.class_id, 32)
                && r.entry.registration.key() == r.class_id
                && r.entry.publication == Publication::Removed)
            && (self.state == State::Deleted) == self.deleted_at.is_some(),
            "deleted_setup_record_binding")
    }
}

pub fn directory(m: &Manager) -> PathBuf {
    m.root.join(DIRECTORY)
}
pub fn path(m: &Manager, environment: &str) -> Result<PathBuf> {
    require(environment_revision::valid_environment_id(environment), "deleted_setup_identity")?;
    Ok(directory(m).join(format!("{environment}.json")))
}
/// Where records moved out of active readers (inventory) are retained.
pub fn history_directory(m: &Manager, environment: &str) -> Result<PathBuf> {
    require(environment_revision::valid_environment_id(environment), "deleted_setup_identity")?;
    Ok(directory(m).join(environment))
}

pub fn load(m: &Manager, environment: &str) -> Result<Option<Tombstone>> {
    let path = path(m, environment)?;
    if !path.try_exists()? { return Ok(None); }
    let record: Tombstone = read_json(&path)?;
    require(record.environment == environment, "deleted_setup_record_binding")?;
    record.validate(m)?;
    Ok(Some(record))
}

/// Every tombstone: the readable ones, and the environment named by each one
/// that can't be read. An unreadable record never breaks a reader; its own
/// environment still counts as deleted.
pub struct Scan {
    pub records: Vec<Tombstone>,
    pub unreadable: Vec<String>,
}
pub fn scan(m: &Manager) -> Result<Scan> {
    let dir = directory(m);
    let mut out = Scan { records: vec![], unreadable: vec![] };
    if !dir.try_exists()? { return Ok(out); }
    let mut seen = 0usize;
    for entry in fs::read_dir(dir)? {
        seen += 1;
        // Tombstones and their history folders; far beyond the deletion limit.
        require(seen <= 2 * READ_BOUND, "deleted_setup_bound")?;
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let Some(id) = name.strip_suffix(".json") else { continue };
        if !environment_revision::valid_environment_id(id) { continue; }
        match load(m, id) {
            Ok(Some(record)) => out.records.push(record),
            _ => out.unreadable.push(id.to_owned()),
        }
    }
    out.records.sort_by_key(|record| (record.requested_at, record.environment.clone()));
    out.unreadable.sort();
    Ok(out)
}
/// Readable tombstones, oldest first.
pub fn records(m: &Manager) -> Result<Vec<Tombstone>> {
    Ok(scan(m)?.records)
}

/// Every environment marked by any tombstone, deleting or deleted.
pub fn environments(m: &Manager) -> Result<std::collections::BTreeSet<String>> {
    let scan = scan(m)?;
    Ok(scan.records.into_iter().flat_map(|record| record.environments.into_iter().map(|e| e.id))
        .chain(scan.unreadable).collect())
}
pub fn is_deleted(m: &Manager, environment: &str) -> Result<bool> {
    Ok(environments(m)?.contains(environment))
}

pub fn write(m: &Manager, record: &Tombstone) -> Result<()> {
    record.validate(m)?;
    let path = path(m, &record.environment)?;
    // A new deletion at the limit is refused before anything is written.
    if !path.try_exists()? {
        let scan = scan(m)?;
        require(scan.records.len() + scan.unreadable.len() < LIMIT, HISTORY_FULL)?;
    }
    private_dir(&directory(m))?;
    atomic_json(&path, record)
}

impl Manager {
    /// Caller holds registry.lock. Retire exactly the listed registrations,
    /// which must still be removed from the DAW with no pending transaction.
    /// Their full entries are first retained in the tombstone by the caller.
    /// A class registered from another environment since (the same plug-in
    /// set up again) belongs to that setup and is left alone.
    pub fn retire_removed_registrations(&self, lock: &Lock,
        retained: &[RetiredRegistration]) -> Result<()> {
        lock.require_registry(self)?;
        let mut db = self.registry()?;
        let mut changed = false;
        for retired in retained {
            let Some(entry) = db.classes.get(&retired.class_id) else { continue };
            if entry.registration.environment.id != retired.entry.registration.environment.id { continue; }
            require(*entry == retired.entry && entry.publication == Publication::Removed
                && !self.publication_pending(&retired.class_id)?
                && publication::physical(&self.link(&retired.class_id))?.is_none(),
                "deleted_setup_registration_changed")?;
            db.classes.remove(&retired.class_id);
            changed = true;
        }
        if changed { self.save(&mut db)?; }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    fn tombstone(f: &Fixture, state: State) -> Tombstone {
        Tombstone {schema:1,environment:f.r.environment.id.clone(),installer:"ab".repeat(32),
            name:"Fixture setup".into(),environments:vec![f.r.environment.clone()],registrations:vec![],
            plugins:vec!["Actual class".into()],operation:"cd".repeat(16),requested_at:1,
            deleted_at:(state == State::Deleted).then_some(2),state}
    }
    #[test]
    fn tombstones_mark_every_recorded_environment_and_refuse_foreign_paths() {
        let f = Fixture::new();
        assert!(!is_deleted(&f.m, &f.r.environment.id).unwrap());
        write(&f.m, &tombstone(&f, State::Deleting)).unwrap();
        assert!(is_deleted(&f.m, &f.r.environment.id).unwrap());
        assert_eq!(load(&f.m, &f.r.environment.id).unwrap().unwrap().state, State::Deleting);
        let mut foreign = tombstone(&f, State::Deleted);
        foreign.environments[0].root = f.outer.join("elsewhere");
        assert_eq!(write(&f.m, &foreign).unwrap_err().to_string(), "deleted_setup_record_binding");
        let mut inconsistent = tombstone(&f, State::Deleted);
        inconsistent.deleted_at = None;
        assert!(write(&f.m, &inconsistent).is_err());
    }
    #[test]
    fn an_unreadable_record_still_marks_its_setup_and_a_full_history_refuses_new_deletions() {
        let f = Fixture::new();
        write(&f.m, &tombstone(&f, State::Deleted)).unwrap();
        let broken = "77".repeat(16);
        fs::write(directory(&f.m).join(format!("{broken}.json")), b"{not json").unwrap();
        let scan = scan(&f.m).unwrap();
        assert_eq!(scan.records.len(), 1);
        assert_eq!(scan.unreadable, vec![broken.clone()]);
        assert!(is_deleted(&f.m, &broken).unwrap(), "fails closed for its own setup");
        assert!(is_deleted(&f.m, &f.r.environment.id).unwrap());
        assert_eq!(records(&f.m).unwrap().len(), 1, "other history stays readable");
        // Fill the history to its limit with other setups' records.
        for n in 0..LIMIT - 2 {
            fs::write(directory(&f.m).join(format!("{:032x}.json", n + 1)), b"{}").unwrap();
        }
        let mut next = tombstone(&f, State::Deleting);
        next.environment = "78".repeat(16);
        let mut environment = f.r.environment.clone();
        environment.id = next.environment.clone();
        environment.root = f.m.root.join("environments").join(&environment.id);
        next.environments = vec![environment];
        assert_eq!(write(&f.m, &next).unwrap_err().to_string(), HISTORY_FULL);
        assert!(!path(&f.m, &next.environment).unwrap().exists(), "refused before writing");
        // Finishing an existing record is never refused by the limit.
        write(&f.m, &tombstone(&f, State::Deleted)).unwrap();
    }
    #[test]
    fn retiring_a_registration_requires_its_exact_removed_entry() {
        let f = Fixture::new();
        f.m.register(f.r.clone()).unwrap();
        let key = f.r.key();
        let entry = f.m.registry().unwrap().classes[&key].clone();
        let retired = RetiredRegistration {class_id:key.clone(),name:"Actual class".into(),entry};
        // Still published: refuse rather than drop a live DAW connection.
        let lock = f.m.lock("registry.lock").unwrap();
        assert_eq!(f.m.retire_removed_registrations(&lock, std::slice::from_ref(&retired))
            .unwrap_err().to_string(), "deleted_setup_registration_changed");
        drop(lock);
        f.m.unpublish(&key).unwrap();
        let removed = f.m.registry().unwrap().classes[&key].clone();
        let retired = RetiredRegistration {entry:removed,..retired};
        let lock = f.m.lock("registry.lock").unwrap();
        f.m.retire_removed_registrations(&lock, std::slice::from_ref(&retired)).unwrap();
        assert!(!f.m.registry().unwrap().classes.contains_key(&key));
        // The same class registered again from another setup is never retired by this one.
        let mut db = f.m.registry().unwrap();
        let mut again = retired.entry.clone();
        again.registration.environment.id = "6c".repeat(16);
        db.classes.insert(key.clone(), again.clone());
        f.m.save(&mut db).unwrap();
        f.m.retire_removed_registrations(&lock, std::slice::from_ref(&retired)).unwrap();
        assert_eq!(f.m.registry().unwrap().classes.get(&key), Some(&again));
    }
}

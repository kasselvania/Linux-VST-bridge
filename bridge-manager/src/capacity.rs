//! Service admission uses the existing registry lock and durable owner leases.
//! There is no second permit registry to forget on service loss. Nothing in
//! this module is called from the DAW or an audio callback.
use crate::*;
pub use ap1_native_client::admission::Refusal;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Dsp,
    Keeper,
    Inspection,
    VendorAccess,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub session: String,
    pub class_id: String,
    pub kind: Kind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClassLimit {
    pub class_id: String,
    pub dsp: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub global_dsp: usize,
    pub classes: Vec<ClassLimit>,
    pub service_workers: usize,
    pub maintenance: usize,
    pub native_image_hard: usize,
}

/// AP17's independently reviewed exact-fixture envelope for the exact retained Arturia
/// fixture. These are class identities, never friendly-name dispatch. The
/// native four-slot mechanic is deliberately not raised to match the service.
pub fn fixture_limits() -> Limits {
    Limits {
        global_dsp: 6,
        classes: vec![
            ClassLimit {
                class_id: "417274754156495350724C4650726F63".into(),
                dsp: 3,
            },
            ClassLimit {
                class_id: "41727475415649536772616E50726F63".into(),
                dsp: 4,
            },
        ],
        service_workers: 16,
        maintenance: 1,
        native_image_hard: 4,
    }
}

/// Single additional AP18 class; the accepted AP17 two-class table is intact.
/// Successful capacity reservation still requires exact retained publication
/// and host admission before a processing instance can be exposed.
pub fn service_limits() -> Result<Limits> {
    let mut limits = fixture_limits();
    limits.classes.push(ClassLimit { class_id: crate::profiles::pigments_verified()?.class.class_id, dsp: 1 });
    Ok(limits)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub schema: u32,
    pub limits: Limits,
    pub engineering_classes: Vec<ClassLimit>,
    #[serde(default)]
    pub verified_additional_classes: Vec<ClassLimit>,
    pub current_workers: usize,
    pub dsp: usize,
    pub available_dsp: usize,
    pub per_class: BTreeMap<String, usize>,
    pub keepers: usize,
    pub maintenance: usize,
    pub maintenance_admissible: bool,
    pub cleanup_unconfirmed: bool,
    pub owners: Vec<Owner>,
    pub envelope: Envelope,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeClaim {
    EngineeringCandidate,
    VerifiedExactFixture,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub claim: EnvelopeClaim,
    pub parallel_tracks: usize,
    pub serial_bridged_depth: usize,
    pub simultaneous_editors: usize,
    pub evidence: String,
    pub fixture: String,
    pub seven: String,
    pub eight: String,
}

pub fn status(m: &Manager, limits: Limits, workers: usize, blocked: bool) -> Result<Status> {
    limits.verify()?;
    let _lock = m.lock("registry.lock")?;
    let extended = limits == service_limits()?;
    let ordinary_limits = fixture_limits();
    let verified = verified_envelope(m, if extended { &ordinary_limits } else { &limits })?;
    let additional_verified = extended && verified_additional(m)?;
    let engineering_classes = if extended && !additional_verified { vec![limits.classes[2].clone()] } else { Vec::new() };
    let verified_additional_classes = if additional_verified { vec![limits.classes[2].clone()] } else { Vec::new() };
    let owners = owners(m)?;
    let dsp = owners.iter().filter(|o| o.kind == Kind::Dsp).count();
    let maintenance = owners
        .iter()
        .filter(|o| matches!(o.kind, Kind::Inspection | Kind::VendorAccess))
        .count();
    let keepers = owners.iter().filter(|o| o.kind == Kind::Keeper).count();
    let per_class: BTreeMap<_, _> = limits
        .classes
        .iter()
        .map(|c| {
            (
                c.class_id.clone(),
                owners
                    .iter()
                    .filter(|o| o.kind == Kind::Dsp && o.class_id == c.class_id)
                    .count(),
            )
        })
        .collect();
    Ok(Status {
        schema: 1,
        available_dsp: if blocked || maintenance != 0 {
            0
        } else {
            limits.global_dsp.saturating_sub(dsp).min(
                limits
                    .classes
                    .iter()
                    .map(|c| c.dsp.saturating_sub(per_class[&c.class_id]))
                    .sum(),
            )
        },
        limits,
        engineering_classes,
        verified_additional_classes,
        current_workers: workers,
        dsp,
        per_class,
        keepers,
        maintenance,
        maintenance_admissible: !blocked && dsp == 0 && maintenance == 0,
        cleanup_unconfirmed: blocked,
        owners,
        envelope: Envelope {
            claim: if verified { EnvelopeClaim::VerifiedExactFixture } else { EnvelopeClaim::EngineeringCandidate },
            parallel_tracks: 3,
            serial_bridged_depth: 3,
            simultaneous_editors: 2,
            evidence: "docs/AP17.md".into(),
            fixture: "SteamOS 3.8.16 / Bitwig 6.1 / exact pinned AP17 Arturia installation; not a universal hardware or DAW limit".into(),
            seven: "unqualified".into(),
            eight: "excluded_for_tested_workload".into(),
        },
    })
}

/// Caller holds registry.lock. Read physical revision authority, never infer it
/// from the manager version or from a retained candidate's mere presence.
fn verified_envelope(m: &Manager, limits: &Limits) -> Result<bool> {
    verified_envelope_for(m, limits, &crate::profiles::ap17_profiles()?)
}
fn verified_additional(m: &Manager) -> Result<bool> {
    let p = crate::profiles::pigments_verified()?;
    let db = m.registry()?;
    let Some(e) = db.classes.get(&p.class.class_id) else { return Ok(false); };
    let Some(reference) = &e.managed_revision else { return Ok(false); };
    let r = m.load_revision(&p.class.class_id, reference)?;
    if r.profile != p || r.qualification.is_some() || e.publication != Publication::Published
        || r.registration != e.registration || m.publication_pending(&p.class.class_id)?
        || crate::publication::physical(&m.link(&p.class.class_id))? != Some(r.target.clone()) {
        return Ok(false);
    }
    m.verify_completed_publication(&r, reference)?;
    Ok(true)
}
pub(crate) fn verified_envelope_for(
    m: &Manager,
    limits: &Limits,
    profiles: &[crate::profiles::Profile],
) -> Result<bool> {
    if *limits != fixture_limits() {
        return Ok(false);
    }
    let db = m.registry()?;
    for p in profiles {
        let Some(e) = db.classes.get(&p.class.class_id) else {
            return Ok(false);
        };
        let Some(reference) = &e.managed_revision else {
            return Ok(false);
        };
        let r = m.load_revision(&p.class.class_id, reference)?;
        if p.revision != 10
            || p.claim != crate::profiles::Claim::VerifiedExactFixture
            || r.profile != *p
            || r.qualification.is_some()
            || e.publication != Publication::Published
            || m.publication_pending(&p.class.class_id)?
            || crate::publication::physical(&m.link(&p.class.class_id))? != Some(r.target.clone())
        {
            return Ok(false);
        }
        m.verify_completed_publication(&r, reference)?;
    }
    Ok(true)
}

impl Limits {
    fn verify(&self) -> Result<()> {
        require(
            (1..=8).contains(&self.global_dsp)
                && self.service_workers > self.global_dsp + self.maintenance
                && self.service_workers <= 16
                && self.maintenance == 1
                && self.native_image_hard == 4
                && ((1..=2).contains(&self.classes.len()) || *self == service_limits()?),
            "capacity_policy_invalid",
        )?;
        let mut ids = std::collections::BTreeSet::new();
        for c in &self.classes {
            require(
                valid_hex(&c.class_id, 32)
                    && c.class_id == c.class_id.to_uppercase()
                    && ids.insert(&c.class_id)
                    && (1..=self.native_image_hard).contains(&c.dsp),
                "capacity_policy_invalid",
            )?;
        }
        Ok(())
    }
}

/// Caller holds registry.lock. Bound both collections before opening owner
/// documents. Malformed, missing or duplicate ownership never counts as free.
pub fn owners(m: &Manager) -> Result<Vec<Owner>> {
    let leases = m.root.join("runtime/leases");
    if !leases.try_exists()? {
        return Ok(Vec::new());
    }
    let paths = fs::read_dir(leases)?
        .take(65)
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    require(paths.len() <= 64, "active_lease_unresolved")?;
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let envs = fs::read_dir(m.root.join("environments"))?
        .take(129)
        .map(|e| e.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    require(envs.len() <= 128, "active_lease_unresolved")?;
    let mut result = Vec::new();
    for lease in paths {
        let sid = lease
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("lease_identity")?;
        require(
            valid_hex(sid, 32) && lease.extension().is_some_and(|e| e == "json"),
            "active_lease_unresolved",
        )?;
        let report: PathBuf = read_json(&lease)?;
        require(
            report.parent() == Some(m.root.join("runtime/results").as_path()),
            "lease_identity",
        )?;
        let mut found = None;
        for env in &envs {
            let p = env
                .join("compatdata/pfx/drive_c/bridge/sessions")
                .join(sid)
                .join("owner.json");
            if p.try_exists()? {
                require(found.is_none(), "duplicate_lease_identity")?;
                found = Some(read_json::<serde_json::Value>(&p)?);
            }
        }
        let o = found.ok_or("active_lease_unresolved")?;
        require(
            o["session"].as_str() == Some(sid) && o["report"].as_str() == report.to_str(),
            "lease_identity",
        )?;
        let class = o["registration"]["metadata"]["class_id"]
            .as_str()
            .ok_or("active_lease_unresolved")?;
        require(valid_hex(class, 32), "active_lease_unresolved")?;
        let keeper = o["keeper"].as_bool().ok_or("active_lease_unresolved")?;
        let inspect = o["inspect"].as_bool().ok_or("active_lease_unresolved")?;
        let access = match o.get("vendor_access") {
            None => false,
            Some(v) => v.as_bool().ok_or("active_lease_unresolved")?,
        };
        let kind = if keeper {
            require(
                inspect
                    && !access
                    && report.file_name().and_then(|s| s.to_str())
                        == Some(format!("environment-{sid}.json").as_str()),
                "lease_identity",
            )?;
            Kind::Keeper
        } else {
            require(
                !(inspect && access)
                    && report.file_name().and_then(|s| s.to_str())
                        == Some(format!("windows-{sid}.json").as_str()),
                "lease_identity",
            )?;
            if inspect {
                Kind::Inspection
            } else if access {
                Kind::VendorAccess
            } else {
                Kind::Dsp
            }
        };
        result.push(Owner {
            session: sid.into(),
            class_id: class.to_uppercase(),
            kind,
        });
    }
    result.sort_by(|a, b| a.session.cmp(&b.session));
    Ok(result)
}

fn check(owners: &[Owner], limits: &Limits, class: Option<&str>) -> Result<()> {
    limits.verify()?;
    if owners
        .iter()
        .any(|o| matches!(o.kind, Kind::Inspection | Kind::VendorAccess))
    {
        return Err(Refusal::MaintenanceActive.into());
    }
    let dsp = owners.iter().filter(|o| o.kind == Kind::Dsp).count();
    let Some(class) = class else {
        return if dsp == 0 {
            Ok(())
        } else {
            Err(Refusal::MaintenanceActive.into())
        };
    };
    let policy = limits
        .classes
        .iter()
        .find(|c| c.class_id == class)
        .ok_or(Refusal::BindingInvalid)?;
    if dsp >= limits.global_dsp {
        return Err(Refusal::GlobalCapacity.into());
    }
    if owners
        .iter()
        .filter(|o| o.kind == Kind::Dsp && o.class_id == class)
        .count()
        >= policy.dsp
    {
        return Err(Refusal::ClassCapacity.into());
    }
    Ok(())
}

/// This exclusive reservation remains held through exact validation, session
/// creation and durable lease publication. On unexposed failure it returns by
/// dropping the lock. After publication the existing lease, not this lock or an
/// in-memory count, retains the unit until positive owner retirement.
pub fn reserve(m: &Manager, limits: &Limits, class: Option<&str>, blocked: bool) -> Result<Lock> {
    if blocked {
        return Err(Refusal::CleanupUnconfirmed.into());
    }
    let lock = m.lock("registry.lock").map_err(|e| {
        if e.to_string() == "operation already running" {
            Refusal::ServiceBusy.into()
        } else {
            e
        }
    })?;
    let records = owners(m).map_err(|_| Refusal::CleanupUnconfirmed)?;
    check(&records, limits, class)?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    fn limits() -> Limits {
        Limits {
            global_dsp: 3,
            classes: vec![
                ClassLimit {
                    class_id: "01".repeat(16),
                    dsp: 2,
                },
                ClassLimit {
                    class_id: "02".repeat(16),
                    dsp: 2,
                },
            ],
            service_workers: 8,
            maintenance: 1,
            native_image_hard: 4,
        }
    }
    fn lease(f: &Fixture, class: &str, kind: Kind) -> PathBuf {
        let sid = random_id().unwrap();
        let report = f.m.root.join("runtime/results").join(format!(
            "{}-{sid}.json",
            if kind == Kind::Keeper {
                "environment"
            } else {
                "windows"
            }
        ));
        private_dir(report.parent().unwrap()).unwrap();
        let path =
            f.r.environment
                .root
                .join("compatdata/pfx/drive_c/bridge/sessions")
                .join(&sid)
                .join("owner.json");
        private_dir(path.parent().unwrap()).unwrap();
        atomic_json(
            &path,
            &serde_json::json!({"session":sid,"report":report,
            "keeper":kind==Kind::Keeper,"inspect":matches!(kind, Kind::Keeper|Kind::Inspection),
            "vendor_access":kind==Kind::VendorAccess,
            "registration":{"metadata":{"class_id":class}}}),
        )
        .unwrap();
        let path = f.m.root.join("runtime/leases").join(format!("{sid}.json"));
        private_dir(path.parent().unwrap()).unwrap();
        atomic_json(&path, &report).unwrap();
        path
    }
    fn reason(r: Result<Lock>) -> String {
        r.err().unwrap().to_string()
    }
    #[test]
    fn global_class_maintenance_and_keeper_are_distinct() {
        let f = Fixture::new();
        let policy = limits();
        let a = &policy.classes[0].class_id;
        let b = &policy.classes[1].class_id;
        lease(&f, a, Kind::Keeper);
        for _ in 0..2 {
            let _reservation = reserve(&f.m, &policy, Some(a), false).unwrap();
            lease(&f, a, Kind::Dsp);
        }
        assert_eq!(
            reason(reserve(&f.m, &policy, Some(a), false)),
            Refusal::ClassCapacity.code()
        );
        assert_eq!(
            reason(reserve(&f.m, &policy, None, false)),
            Refusal::MaintenanceActive.code()
        );
        {
            let _reservation = reserve(&f.m, &policy, Some(b), false).unwrap();
            lease(&f, b, Kind::Dsp);
        }
        let before = owners(&f.m).unwrap();
        assert_eq!(
            reason(reserve(&f.m, &policy, Some(b), false)),
            Refusal::GlobalCapacity.code()
        );
        assert_eq!(owners(&f.m).unwrap(), before); // no partial owner on refusal
        assert_eq!(
            reason(reserve(&f.m, &policy, Some(b), true)),
            Refusal::CleanupUnconfirmed.code()
        );
    }
    #[test]
    fn durable_ownership_survives_service_reconstruction_and_exact_release() {
        let f = Fixture::new();
        let p = limits();
        let a = &p.classes[0].class_id;
        let first = {
            let _r = reserve(&f.m, &p, Some(a), false).unwrap();
            lease(&f, a, Kind::Dsp)
        };
        let sibling = {
            let _r = reserve(&f.m, &p, Some(a), false).unwrap();
            lease(&f, a, Kind::Dsp)
        };
        let restarted = Manager {
            root: f.m.root.clone(),
            publications: f.m.publications.clone(),
        };
        assert_eq!(
            reason(reserve(&restarted, &p, Some(a), false)),
            Refusal::ClassCapacity.code()
        );
        fs::remove_file(&first).unwrap(); // production retirement receipt owns this operation
        let replacement = {
            let _r = reserve(&restarted, &p, Some(a), false).unwrap();
            lease(&f, a, Kind::Dsp)
        };
        assert_ne!(replacement, first);
        assert!(fs::remove_file(first).is_err());
        assert!(sibling.exists() && replacement.exists());
        assert_eq!(
            reason(reserve(&restarted, &p, Some(a), false)),
            Refusal::ClassCapacity.code()
        );
    }
    #[test]
    fn failed_startup_returns_only_unexposed_reservation_and_unknown_owner_blocks() {
        let f = Fixture::new();
        let p = limits();
        let a = &p.classes[0].class_id;
        {
            let _r = reserve(&f.m, &p, Some(a), false).unwrap();
        }
        assert!(owners(&f.m).unwrap().is_empty());
        let lease = {
            let _r = reserve(&f.m, &p, Some(a), false).unwrap();
            lease(&f, a, Kind::Dsp)
        };
        let report: PathBuf = read_json(&lease).unwrap();
        // An ordinary report (or its absence) does not release exposed capacity.
        atomic_json(&report, &serde_json::json!({"cleanup_confirmed":true})).unwrap();
        assert_eq!(owners(&f.m).unwrap().len(), 1);
        fs::write(lease, b"invalid").unwrap();
        assert_eq!(
            reason(reserve(&f.m, &p, Some(a), false)),
            Refusal::CleanupUnconfirmed.code()
        );
    }
    #[test]
    fn concurrent_reservations_cannot_check_then_over_admit() {
        let f = Fixture::new();
        let p = limits();
        // A held reservation deterministically refuses a contender without
        // publishing a lease. Scheduler yield counts are not a progress law.
        let held = reserve(&f.m, &p, Some(&p.classes[0].class_id), false).unwrap();
        std::thread::scope(|scope| {
            assert_eq!(
                scope
                    .spawn(|| reason(reserve(&f.m, &p, Some(&p.classes[1].class_id), false)))
                    .join()
                    .unwrap(),
                Refusal::ServiceBusy.code()
            );
        });
        assert!(owners(&f.m).unwrap().is_empty());
        drop(held);

        let barrier = std::sync::Barrier::new(16);
        let accepted = std::thread::scope(|scope| {
            let mut jobs = Vec::new();
            for i in 0..16 {
                let f = &f;
                let p = &p;
                let barrier = &barrier;
                jobs.push(scope.spawn(move || {
                    barrier.wait();
                    let class = &p.classes[i % 2].class_id;
                    // The production API refuses contention, never waits. Test
                    // clients retry Busy with a finite bound outside any DAW.
                    for _ in 0..1000 {
                        match reserve(&f.m, p, Some(class), false) {
                            Ok(_guard) => {
                                lease(f, class, Kind::Dsp);
                                return true;
                            }
                            Err(e) if e.to_string() == Refusal::ServiceBusy.code() => {
                                std::thread::yield_now()
                            }
                            Err(e) => {
                                assert!(matches!(
                                    e.to_string().as_str(),
                                    code if code == Refusal::ClassCapacity.code()
                                        || code == Refusal::GlobalCapacity.code()
                                ));
                                return false;
                            }
                        }
                    }
                    false
                }));
            }
            jobs.into_iter()
                .map(|j| j.join().unwrap())
                .filter(|accepted| *accepted)
                .count()
        });
        assert!((1..=3).contains(&accepted));
        assert_eq!(owners(&f.m).unwrap().len(), accepted);
        // All contenders have joined. Prove every remaining capacity unit is
        // available, without assuming bounded Busy retries must fill it first.
        for c in &p.classes {
            for _ in 0..c.dsp {
                match reserve(&f.m, &p, Some(&c.class_id), false) {
                    Ok(_guard) => {
                        lease(&f, &c.class_id, Kind::Dsp);
                    }
                    Err(e) => assert!(matches!(
                        e.to_string().as_str(),
                        code if code == Refusal::ClassCapacity.code()
                            || code == Refusal::GlobalCapacity.code()
                    )),
                }
            }
        }
        let records = owners(&f.m).unwrap();
        assert_eq!(records.len(), 3);
        for c in &p.classes {
            assert!(records.iter().filter(|o| o.class_id == c.class_id).count() <= 2);
        }
    }
    #[test]
    fn maintenance_never_becomes_dsp_and_excludes_new_dsp() {
        for kind in [Kind::Inspection, Kind::VendorAccess] {
            let f = Fixture::new();
            let p = limits();
            let a = &p.classes[0].class_id;
            lease(&f, a, Kind::Keeper);
            {
                let _r = reserve(&f.m, &p, None, false).unwrap();
                lease(&f, a, kind);
            }
            assert_eq!(
                owners(&f.m)
                    .unwrap()
                    .iter()
                    .filter(|o| o.kind == Kind::Dsp)
                    .count(),
                0
            );
            assert_eq!(
                reason(reserve(&f.m, &p, Some(a), false)),
                Refusal::MaintenanceActive.code()
            );
            assert_eq!(
                reason(reserve(&f.m, &p, None, false)),
                Refusal::MaintenanceActive.code()
            );
        }
    }
}

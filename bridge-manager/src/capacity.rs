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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceTerminal {
    WindowsHostExited,
    EditorControllerFailed,
    TransportFailed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub session: String,
    pub class_id: String,
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<InstanceTerminal>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TerminalSummary {
    pub schema: u32,
    pub session: String,
    pub class_id: String,
    pub terminal: InstanceTerminal,
    pub producer: u64,
    pub status_domain: u64,
    pub cleanup_confirmed: bool,
    pub transport_retired: bool,
    pub observed_at: u64,
}

fn summary_directory(m: &Manager) -> PathBuf {
    m.root.join("runtime/terminal-summaries")
}

pub fn retain_terminal_summary(
    m: &Manager,
    session: &str,
    class_id: &str,
    report: &Path,
) -> Result<()> {
    require(
        valid_hex(session, 32)
            && valid_hex(class_id, 32)
            && class_id == class_id.to_uppercase()
            && report.parent() == Some(m.root.join("runtime/results").as_path()),
        "terminal_summary_binding",
    )?;
    let result: serde_json::Value = read_json(report)?;
    require(
        result["session"].as_str() == Some(session),
        "terminal_summary_binding",
    )?;
    let directory = summary_directory(m);
    private_dir(&directory)?;
    let path = directory.join(format!("{class_id}.json"));
    let terminal = &result["fault_status"]["before_containment"]["terminal_instance"];
    if terminal.is_object() {
        require(
            terminal["session"].as_str() == Some(session),
            "terminal_summary_binding",
        )?;
        let terminal_class = match terminal["failure_class"].as_u64() {
            Some(1) => InstanceTerminal::WindowsHostExited,
            Some(2) => InstanceTerminal::EditorControllerFailed,
            Some(3) => InstanceTerminal::TransportFailed,
            _ => return Err("terminal_summary_class".into()),
        };
        let producer = terminal["producer"]
            .as_u64()
            .ok_or("terminal_summary_producer")?;
        let status_domain = terminal["status_domain"]
            .as_u64()
            .ok_or("terminal_summary_status_domain")?;
        require(
            (1..=3).contains(&producer) && (1..=5).contains(&status_domain),
            "terminal_summary_status",
        )?;
        let summary = TerminalSummary {
            schema: 1,
            session: session.into(),
            class_id: class_id.into(),
            terminal: terminal_class,
            producer,
            status_domain,
            cleanup_confirmed: result["cleanup_confirmed"]
                .as_bool()
                .ok_or("terminal_summary_cleanup")?,
            transport_retired: result["transport_retired"]
                .as_bool()
                .ok_or("terminal_summary_transport")?,
            observed_at: observation::now()?,
        };
        atomic_json(&path, &summary)?;
    } else if result["error"].is_null()
        && result["gated"] == true
        && result["cleanup_confirmed"] == true
        && result["transport_retired"] == true
    {
        // A later fully started and cleanly retired instance is the modest
        // replacement policy. Merely opening the manager never erases the
        // most recent failure.
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

pub fn terminal_summaries(m: &Manager) -> Result<Vec<TerminalSummary>> {
    let directory = summary_directory(m);
    if !directory.try_exists()? {
        return Ok(Vec::new());
    }
    private_dir(&directory)?;
    let entries = fs::read_dir(&directory)?
        .take(129)
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    require(entries.len() <= 128, "terminal_summary_capacity")?;
    let mut result = Vec::with_capacity(entries.len());
    for path in entries {
        let summary: TerminalSummary = read_json(&path)?;
        require(
            summary.schema == 1
                && valid_hex(&summary.session, 32)
                && valid_hex(&summary.class_id, 32)
                && summary.class_id == summary.class_id.to_uppercase()
                && (1..=3).contains(&summary.producer)
                && (1..=5).contains(&summary.status_domain)
                && summary.observed_at != 0
                && path.file_name().and_then(|name| name.to_str())
                    == Some(format!("{}.json", summary.class_id).as_str()),
            "terminal_summary_binding",
        )?;
        result.push(summary);
    }
    result.sort_by_key(|summary| summary.observed_at);
    Ok(result)
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
    limits.classes.push(ClassLimit {class_id: crate::managed_candidate::candidate()?.class.class_id, dsp:1});
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
    status_with_wait(m, limits, workers, blocked, std::time::Duration::from_secs(2))
}

fn status_with_wait(
    m: &Manager,
    limits: Limits,
    workers: usize,
    blocked: bool,
    wait: std::time::Duration,
) -> Result<Status> {
    limits.verify()?;
    // UI polls and operator validation share this registry. A readback may
    // briefly wait for their lock; admission below remains fail-fast. Read all
    // owners only after acquisition, never reuse an earlier healthy snapshot.
    // Two seconds leaves room inside the LVC1 client's five-second read bound.
    let (_lock, _) = m.lock_bounded(
        operator_model::OperatorLock::Registry,
        operator_model::LockPurpose::OperatorReadback,
        None,
        wait,
    )?;
    let extended = limits == service_limits()?;
    let ordinary_limits = fixture_limits();
    let verified = verified_envelope(m, if extended { &ordinary_limits } else { &limits })?;
    let additional_verified = extended && verified_additional(m)?;
    let mut engineering_classes = if extended && !additional_verified { vec![limits.classes[2].clone()] } else { Vec::new() };
    if extended { engineering_classes.push(limits.classes[3].clone()); }
    let mut current_classes=limits.classes.clone();
    for (key,e) in m.registry()?.classes {
        if e.publication!=Publication::Published || current_classes.iter().any(|c|c.class_id==key){continue;}
        let Some(reference)=e.managed_revision else {continue};
        let r=m.load_revision(&key,&reference)?;
        if crate::preparation::owns_profile(m,&r.profile)? {
            let limit=ClassLimit{class_id:key,dsp:1};
            engineering_classes.push(limit.clone());current_classes.push(limit);
        }
    }
    let verified_additional_classes = if additional_verified { vec![limits.classes[2].clone()] } else { Vec::new() };
    let owners = owners(m)?;
    let dsp = owners.iter().filter(|o| o.kind == Kind::Dsp).count();
    let maintenance = owners
        .iter()
        .filter(|o| matches!(o.kind, Kind::Inspection | Kind::VendorAccess))
        .count();
    let keepers = owners.iter().filter(|o| o.kind == Kind::Keeper).count();
    let per_class: BTreeMap<_, _> = current_classes
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
                current_classes
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
    if (r.profile != p && r.profile != crate::profiles::pigments_eleven()?) || r.qualification.is_some() || e.publication != Publication::Published
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

fn session_identity(session: &str) -> Result<[u8; 16]> {
    require(valid_hex(session, 32), "terminal_session_identity")?;
    let mut result = [0u8; 16];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&session[index * 2..index * 2 + 2], 16)?;
    }
    Ok(result)
}

fn terminal_status_from(
    durable: &Path,
    source_directory: &Path,
    session: &str,
) -> Result<Option<InstanceTerminal>> {
    use std::os::unix::fs::FileExt;
    let target = durable.join("if1.terminal");
    let target_metadata = match fs::symlink_metadata(&target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    require(
        target_metadata.file_type().is_symlink()
            && target_metadata.uid() == unsafe { libc::getuid() },
        "terminal_status_view",
    )?;
    let source = source_directory.join("if1.terminal");
    require(fs::read_link(&target)? == source, "terminal_status_view")?;
    let file = file(&source)?;
    let opened = file.metadata()?;
    let visible = fs::metadata(&target)?;
    require(
        (opened.dev(), opened.ino()) == (visible.dev(), visible.ino()),
        "terminal_status_source_replaced",
    )?;
    require(file.metadata()?.len() == 2048, "terminal_status_extent")?;
    let mut header = [0u8; 32];
    file.read_exact_at(&mut header, 0)?;
    let identity = session_identity(session)?;
    require(&header[..4] == b"LVIF"
        && u32::from_le_bytes(header[4..8].try_into().unwrap()) == 1
        && u32::from_le_bytes(header[8..12].try_into().unwrap()) == 2048
        && header[16..32] == identity, "terminal_status_binding")?;
    for _ in 0..3 {
        let mut before = [0u8; 8];
        file.read_exact_at(&mut before, 64)?;
        let generation = u64::from_le_bytes(before);
        if generation == 0 { return Ok(None); }
        require((1..=3).contains(&generation), "terminal_status_generation")?;
        let offset = 128 + (generation - 1) * 256;
        let mut raw = [0u8; 192];
        file.read_exact_at(&mut raw, offset)?;
        let mut after = [0u8; 8];
        file.read_exact_at(&mut after, 64)?;
        if before != after { continue; }
        let words: [u64; 24] = std::array::from_fn(|index| {
            u64::from_le_bytes(raw[index * 8..index * 8 + 8].try_into().unwrap())
        });
        require(words[0] == 1
            && words[1].to_le_bytes() == identity[..8]
            && words[2].to_le_bytes() == identity[8..]
            && words[3] != 0
            && words[18] == generation
            && (1..=5).contains(&words[19])
            && words[20..].iter().all(|value| *value == 0), "terminal_status_record")?;
        return Ok(Some(match words[14] {
            1 => InstanceTerminal::WindowsHostExited,
            2 => InstanceTerminal::EditorControllerFailed,
            3 => InstanceTerminal::TransportFailed,
            _ => return Err("terminal_status_class".into()),
        }));
    }
    Err("terminal_status_unstable".into())
}

fn terminal_status(
    durable: &Path,
    session: &str,
    transport: Option<&transport_storage::MemoryTransport>,
) -> Result<Option<InstanceTerminal>> {
    let target = durable.join("if1.terminal");
    if fs::symlink_metadata(&target).is_err_and(|error| {
        error.kind() == std::io::ErrorKind::NotFound
    }) {
        // Older immutable native builds did not create IF1. Absence remains
        // compatible; a present mapping must use the exact volatile owner.
        return Ok(None);
    }
    let identity = transport.ok_or("terminal_status_transport_absent")?;
    let source = transport_storage::session_directory(session, identity)?;
    terminal_status_from(durable, &source, session)
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
                found = Some((read_json::<serde_json::Value>(&p)?, p));
            }
        }
        let (o, owner_path) = found.ok_or("active_lease_unresolved")?;
        require(
            o["session"].as_str() == Some(sid) && o["report"].as_str() == report.to_str(),
            "lease_identity",
        )?;
        let class = o["registration"]["metadata"]["class_id"]
            .as_str()
            .ok_or("active_lease_unresolved")?;
        let keeper = o["keeper"].as_bool().ok_or("active_lease_unresolved")?;
        let inspect = o["inspect"].as_bool().ok_or("active_lease_unresolved")?;
        let access = match o.get("vendor_access") {
            None => false,
            Some(v) => v.as_bool().ok_or("active_lease_unresolved")?,
        };
        // The existing standalone module census intentionally has no selected
        // class yet. It is maintenance, never an unclassified DSP admission.
        let module_census = class.is_empty()
            && inspect
            && !keeper
            && !access
            && o["first_audio"] == true
            && o["shared_runtime"] == false
            && o["shared_inspection"] == false;
        require(
            valid_hex(class, 32) || module_census,
            "active_lease_unresolved",
        )?;
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
        let transport = match o.get("transport") {
            Some(value) if !value.is_null() => {
                Some(serde_json::from_value::<transport_storage::MemoryTransport>(value.clone())?)
            }
            _ => None,
        };
        result.push(Owner {
            session: sid.into(),
            class_id: class.to_uppercase(),
            kind,
            terminal: if kind == Kind::Dsp {
                terminal_status(owner_path.parent().unwrap(), sid, transport.as_ref())?
            } else {
                None
            },
        });
    }
    result.sort_by(|a, b| a.session.cmp(&b.session));
    Ok(result)
}

fn check_selected(owners: &[Owner], limits: &Limits, class: Option<&str>, managed:bool) -> Result<()> {
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
        .find(|c| c.class_id == class);
    let ceiling=policy.map(|p|p.dsp).or(if managed{Some(1)}else{None}).ok_or(Refusal::BindingInvalid)?;
    if dsp >= limits.global_dsp {
        return Err(Refusal::GlobalCapacity.into());
    }
    if owners
        .iter()
        .filter(|o| o.kind == Kind::Dsp && o.class_id == class)
        .count()
        >= ceiling
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
        if e.is::<crate::operator_lock::LockBusy>() {
            Refusal::ServiceBusy.into()
        } else {
            e
        }
    })?;
    let records = owners(m).map_err(|_| Refusal::CleanupUnconfirmed)?;
    let managed = if let Some(class)=class.filter(|class|!limits.classes.iter().any(|c|c.class_id==*class)) {
        let db=m.registry()?;
        if let Some(e)=db.classes.get(class).filter(|e|e.publication==Publication::Published) {
            let r=m.load_revision(class,e.managed_revision.as_ref().ok_or(Refusal::BindingInvalid)?)?;
            crate::preparation::owns_profile(m,&r.profile)?
        } else {false}
    }else{false};
    check_selected(&records, limits, class,managed)?;
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
    #[test]
    fn status_waits_for_contention_and_reads_new_owners() {
        let f = Fixture::new();
        let held = f.m.lock("registry.lock").unwrap();
        std::thread::scope(|scope| {
            let (tx, rx) = std::sync::mpsc::channel();
            let m = &f.m;
            let reader = scope.spawn(move || {
                tx.send(()).unwrap();
                status(m, limits(), 1, false).unwrap()
            });
            rx.recv().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(40));
            lease(&f, &limits().classes[0].class_id, Kind::Inspection);
            drop(held);
            let result = reader.join().unwrap();
            assert_eq!(result.maintenance, 1);
            assert_eq!(result.available_dsp, 0);
            assert!(!result.maintenance_admissible);
        });
    }
    #[test]
    fn status_contention_is_bounded_and_admission_stays_fail_fast() {
        let f = Fixture::new();
        let _held = f.m.lock("registry.lock").unwrap();
        let error = status_with_wait(
            &f.m, limits(), 1, false, std::time::Duration::from_millis(20),
        ).err().unwrap();
        let failure = error.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap();
        assert_eq!(failure.facts.outcome, operator_model::LockOutcome::Timeout);
        assert!(matches!(reserve(&f.m, &limits(), None, false).err().unwrap()
            .downcast_ref::<Refusal>(), Some(Refusal::ServiceBusy)));
    }
    #[test]
    fn status_does_not_hide_invalid_owners_or_cleanup_blocking() {
        let f = Fixture::new();
        let result = status(&f.m, limits(), 1, true).unwrap();
        assert!(result.cleanup_unconfirmed);
        assert_eq!(result.available_dsp, 0);
        assert!(!result.maintenance_admissible);
        private_dir(&f.m.root.join("runtime/leases")).unwrap();
        fs::write(f.m.root.join("runtime/leases/broken.json"), b"invalid").unwrap();
        assert!(status(&f.m, limits(), 1, false).is_err());
    }
    #[test]
    fn standalone_module_census_is_only_maintenance() {
        let f = Fixture::new();
        let path = lease(&f, "", Kind::Inspection);
        let sid = path.file_stem().unwrap();
        let owner =
            f.r.environment
                .root
                .join("compatdata/pfx/drive_c/bridge/sessions")
                .join(sid)
                .join("owner.json");
        let mut row: serde_json::Value = read_json(&owner).unwrap();
        assert!(owners(&f.m).is_err());
        row["first_audio"] = true.into();
        row["shared_runtime"] = false.into();
        row["shared_inspection"] = false.into();
        atomic_json(&owner, &row).unwrap();
        let observed = owners(&f.m).unwrap();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].kind, Kind::Inspection);
        assert_eq!(
            reason(reserve(
                &f.m,
                &limits(),
                Some(&limits().classes[0].class_id),
                false
            )),
            Refusal::MaintenanceActive.code()
        );
        for (key, value) in [
            ("inspect", false),
            ("keeper", true),
            ("vendor_access", true),
            ("first_audio", false),
            ("shared_runtime", true),
            ("shared_inspection", true),
        ] {
            let mut malformed = row.clone();
            malformed[key] = value.into();
            atomic_json(&owner, &malformed).unwrap();
            assert!(owners(&f.m).is_err(), "{key}");
        }
    }
    fn reason(r: Result<Lock>) -> String {
        r.err().unwrap().to_string()
    }
    fn terminal_bytes(session: &str, class: u64) -> Vec<u8> {
        let mut bytes = vec![0u8; 2048];
        bytes[..4].copy_from_slice(b"LVIF");
        bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&2048u32.to_le_bytes());
        let identity = session_identity(session).unwrap();
        bytes[16..32].copy_from_slice(&identity);
        bytes[64..72].copy_from_slice(&2u64.to_le_bytes());
        let at = 128 + 256;
        let mut words = [0u64; 24];
        words[0] = 1;
        words[1] = u64::from_le_bytes(identity[..8].try_into().unwrap());
        words[2] = u64::from_le_bytes(identity[8..].try_into().unwrap());
        words[3] = 7;
        words[14] = class;
        words[15] = 1;
        words[18] = 2;
        words[19] = 3;
        for (index, value) in words.into_iter().enumerate() {
            bytes[at + index * 8..at + index * 8 + 8].copy_from_slice(&value.to_le_bytes());
        }
        bytes
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
    fn production_transport_view_projects_terminal_failure_without_following_the_symlink() {
        let f = Fixture::new();
        let class = &limits().classes[0].class_id;
        let lease = lease(&f, class, Kind::Dsp);
        let session = lease.file_stem().unwrap().to_str().unwrap();
        let durable = f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions").join(session);
        let root = f.outer.join("volatile");
        private_dir(&root).unwrap();
        let source = root.join(session);
        private_dir(&source).unwrap();
        let metadata = fs::symlink_metadata(&source).unwrap();
        let identity = transport_storage::MemoryTransport {
            schema: 1,
            device: metadata.dev(),
            inode: metadata.ino(),
        };
        let exact = transport_storage::fixture_session_directory(&root, session, &identity).unwrap();
        fs::write(exact.join("if1.terminal"), terminal_bytes(session, 2)).unwrap();
        symlink(exact.join("if1.terminal"), durable.join("if1.terminal")).unwrap();
        assert_eq!(
            terminal_status_from(&durable, &exact, session).unwrap(),
            Some(InstanceTerminal::EditorControllerFailed)
        );

        fs::remove_file(durable.join("if1.terminal")).unwrap();
        symlink(f.outer.join("wrong/if1.terminal"), durable.join("if1.terminal")).unwrap();
        assert!(terminal_status_from(&durable, &exact, session).is_err());
        fs::remove_file(durable.join("if1.terminal")).unwrap();
        symlink(exact.join("if1.terminal"), durable.join("if1.terminal")).unwrap();

        let replaced = transport_storage::MemoryTransport { inode: identity.inode + 1, ..identity };
        assert!(transport_storage::fixture_session_directory(&root, session, &replaced).is_err());
        fs::remove_file(exact.join("if1.terminal")).unwrap();
        let foreign=f.outer.join("foreign-if1");
        fs::write(&foreign,terminal_bytes(session,2)).unwrap();
        symlink(&foreign,exact.join("if1.terminal")).unwrap();
        assert!(terminal_status_from(&durable,&exact,session).is_err());
        fs::remove_file(exact.join("if1.terminal")).unwrap();
        fs::write(exact.join("if1.terminal"), b"malformed").unwrap();
        assert!(terminal_status_from(&durable, &exact, session).is_err());
        assert!(terminal_status(&durable,session,None).is_err());
        fs::remove_file(durable.join("if1.terminal")).unwrap();
        assert_eq!(terminal_status_from(&durable, &exact, session).unwrap(), None);
    }
    #[test]
    fn terminal_failure_survives_confirmed_owner_retirement_without_capture() {
        let f = Fixture::new();
        let class = limits().classes[0].class_id.clone();
        let lease = lease(&f, &class, Kind::Dsp);
        let session = lease.file_stem().unwrap().to_str().unwrap().to_owned();
        let report:PathBuf = read_json(&lease).unwrap();
        atomic_json(&report,&serde_json::json!({
            "session":session,"error":"Windows host exited without successful close",
            "gated":true,"cleanup_confirmed":true,"transport_retired":true,
            "fault_status":{"before_containment":{"terminal_instance":{
                "schema":1,"session":session,"failure_class":1,
                "producer":2,"status_domain":3
            }}}
        })).unwrap();
        retain_terminal_summary(&f.m,&session,&class,&report).unwrap();
        fs::remove_file(&lease).unwrap();
        let owner=f.r.environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&session);
        fs::remove_dir_all(owner).unwrap();
        assert!(owners(&f.m).unwrap().is_empty());
        let retained=terminal_summaries(&f.m).unwrap();
        assert_eq!(retained.len(),1);
        assert_eq!(retained[0].terminal,InstanceTerminal::WindowsHostExited);
        assert!(retained[0].cleanup_confirmed&&retained[0].transport_retired);

        atomic_json(&report,&serde_json::json!({
            "session":session,"error":null,"gated":true,
            "cleanup_confirmed":true,"transport_retired":true,
            "fault_status":null
        })).unwrap();
        retain_terminal_summary(&f.m,&session,&class,&report).unwrap();
        assert!(terminal_summaries(&f.m).unwrap().is_empty());
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

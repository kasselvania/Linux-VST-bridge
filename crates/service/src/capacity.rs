//! Service admission uses the existing registry lock and durable owner leases.
//! There is no second permit registry to forget on service loss. Nothing in
//! this module is called from the DAW or an audio callback.
use crate::*;
pub use ap1_native_client::admission::Refusal;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Dsp,
    /// Ordinary plug-in instance with an editor and no audio-device connection.
    Detached,
    Keeper,
    Inspection,
    VendorAccess,
}

impl Kind {
    pub fn is_instance(self) -> bool { matches!(self, Self::Dsp | Self::Detached) }
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

/// Slots in one native engine image's instance registry
/// (`native-vst3-proxy/backend/src/instances.rs`). The service applies this
/// structural bound to each plug-in class; it is not a workload claim.
pub const NATIVE_IMAGE_SLOTS: usize = 4;

/// Historical qualification policy only: AP17's independently reviewed
/// envelope for the exact retained Arturia fixture, kept for its sealed review
/// and regression tests. Production admission uses `service_limits`. These are
/// class identities, never friendly-name dispatch.
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

/// Production admission policy: one finite reservation with no class table.
/// - up to eight live bridged instances across all of this user's DAWs and setups;
/// - up to `NATIVE_IMAGE_SLOTS` instances of any one plug-in class;
/// - one maintenance owner (inspection or vendor application) at a time;
/// - sixteen service workers. Every live instance and the maintenance owner
///   holds one worker for its lifetime, so seven stay free for status,
///   recovery and refusals when instances and maintenance are both full.
///
/// A reservation grants no authority: exact publication and host admission
/// still decide whether a class may run at all.
pub fn service_limits() -> Limits {
    Limits {
        global_dsp: 8,
        classes: Vec::new(),
        service_workers: 16,
        maintenance: 1,
        native_image_hard: NATIVE_IMAGE_SLOTS,
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub schema: u32,
    pub limits: Limits,
    /// Every class ceiling in force: historical table rows and each exactly
    /// published class bounded by the native image.
    pub class_ceilings: Vec<ClassLimit>,
    pub current_workers: usize,
    pub dsp: usize,
    pub available_dsp: usize,
    pub per_class: BTreeMap<String, usize>,
    pub keepers: usize,
    pub maintenance: usize,
    pub maintenance_admissible: bool,
    pub cleanup_unconfirmed: bool,
    pub owners: Vec<Owner>,
    /// Present only for the historical qualification policy it describes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<Envelope>,
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

/// Read the same census without waiting behind a mutation. A missed read is
/// unavailable, never a cached idle result. Yield to an already waiting writer.
pub fn status_observed(m: &Manager, limits: Limits, workers: usize, blocked: bool) -> Result<Status> {
    limits.verify()?;
    require(!operator_lock::registry_writer_waiting(m), "capacity_observation_busy")?;
    let _lock = match m.try_lock_shared("registry.lock")? {
        operator_lock::LockAttempt::Acquired(lock) => lock,
        operator_lock::LockAttempt::Busy => return Err("capacity_observation_busy".into()),
    };
    status_locked(m, limits, workers, blocked)
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
    status_locked(m, limits, workers, blocked)
}

/// Caller holds shared or exclusive registry custody.
fn status_locked(m: &Manager, limits: Limits, workers: usize, blocked: bool) -> Result<Status> {
    // The envelope describes the sealed historical fixture policy. Production
    // admission is a different policy and claims no qualification envelope.
    let envelope = if limits == fixture_limits() {
        let verified = retained_qualification_envelope(m, &limits)?;
        Some(Envelope {
            claim: if verified { EnvelopeClaim::VerifiedExactFixture } else { EnvelopeClaim::EngineeringCandidate },
            parallel_tracks: 3,
            serial_bridged_depth: 3,
            simultaneous_editors: 2,
            evidence: "docs/AP17.md".into(),
            fixture: "SteamOS 3.8.16 / Bitwig 6.1 / exact pinned AP17 Arturia installation; not a universal hardware or DAW limit".into(),
            seven: "unqualified".into(),
            eight: "excluded_for_tested_workload".into(),
        })
    } else {
        None
    };
    let mut class_ceilings = limits.classes.clone();
    let mut setups = BTreeMap::new();
    for (key,e) in m.registry()?.classes {
        if e.publication!=Publication::Published {continue;}
        if !class_ceilings.iter().any(|c|c.class_id==key) {
            // Publication eligibility controls only this class's slots. An
            // invalid or pending publication must not hide canonical owners or
            // prevent its ordinary reconciliation. Admission below still returns
            // the exact publication error; malformed owners still fail readback.
            let Ok(Some(capacity)) = image_capacity_with(m, limits.native_image_hard, &key, false) else { continue; };
            class_ceilings.push(ClassLimit{class_id:key.clone(),dsp:capacity});
        }
        setups.insert(key, e.registration.environment);
    }
    let owners = owners(m)?;
    let dsp = owners.iter().filter(|o| o.kind.is_instance()).count();
    let maintenance = owners
        .iter()
        .filter(|o| matches!(o.kind, Kind::Inspection | Kind::VendorAccess))
        .count();
    // Admission excludes maintenance only from its own setup, and an owner
    // without a resolvable setup from every setup (reserve_environment).
    let maintained = owners
        .iter()
        .filter(|o| matches!(o.kind, Kind::Inspection | Kind::VendorAccess))
        .map(|o| m.lease_environment(&o.session).ok())
        .collect::<Option<Vec<_>>>();
    let keepers = owners.iter().filter(|o| o.kind == Kind::Keeper).count();
    let per_class: BTreeMap<_, _> = class_ceilings
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
    let available_dsp = match maintained {
        Some(maintained) if !blocked => limits.global_dsp.saturating_sub(dsp).min(
            class_ceilings
                .iter()
                .filter(|c| setups.get(&c.class_id).is_none_or(|setup: &Environment|
                    !maintained.iter().any(|scope| scope.id == setup.id || scope.root == setup.root)))
                .map(|c| c.dsp.saturating_sub(per_class[&c.class_id]))
                .sum(),
        ),
        _ => 0,
    };
    Ok(Status {
        schema: 1,
        available_dsp,
        limits,
        class_ceilings,
        current_workers: workers,
        dsp,
        per_class,
        keepers,
        maintenance,
        maintenance_admissible: !blocked && dsp == 0 && maintenance == 0,
        cleanup_unconfirmed: blocked,
        owners,
        envelope,
    })
}

/// Caller holds registry.lock. Match retained qualification and physical
/// publication records. This projects the evidence's claim, never fresh
/// executable verification or permission to launch. Reservation/launch owners
/// retain their independent current checks.
fn retained_qualification_envelope(m: &Manager, limits: &Limits) -> Result<bool> {
    retained_qualification_envelope_for(m, limits, &crate::profiles::ap17_profiles()?)
}
pub(crate) fn retained_qualification_envelope_for(
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
        let r = m.load_revision_record(&p.class.class_id, reference)?;
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
        // Production has no class table; the historical one has two rows.
        require(
            (1..=8).contains(&self.global_dsp)
                && self.service_workers > self.global_dsp + self.maintenance
                && self.service_workers <= 16
                && self.maintenance == 1
                && self.native_image_hard == NATIVE_IMAGE_SLOTS
                && self.classes.len() <= 2,
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

pub(crate) fn terminal_file_from(
    durable: &Path,
    source_directory: &Path,
    session: &str,
) -> Result<Option<File>> {
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
    Ok(Some(file))
}

fn terminal_status_from(durable: &Path, source_directory: &Path, session: &str)
    -> Result<Option<InstanceTerminal>> {
    use std::os::unix::fs::FileExt;
    let Some(file) = terminal_file_from(durable, source_directory, session)? else { return Ok(None) };
    let identity = session_identity(session)?;
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
    owners_for(m, None)
}
/// Exact retained records under registry custody; shared by admission and observation.
/// The observation path must not resolve instances through today's publication.
pub(crate) struct ActiveRecord {
    pub owner: Owner,
    pub document: serde_json::Value,
    pub path: PathBuf,
    pub report: PathBuf,
}

pub fn owners_for(m: &Manager, environment: Option<&Environment>) -> Result<Vec<Owner>> {
    active_records_for(m, environment)?.into_iter().map(|mut record| {
        let transport = match record.document.get("transport") {
            Some(value) if !value.is_null() => Some(serde_json::from_value::<transport_storage::MemoryTransport>(value.clone())?),
            _ => None,
        };
        if record.owner.kind == Kind::Dsp {
            record.owner.terminal = terminal_status(record.path.parent().unwrap(),
                &record.owner.session, transport.as_ref())?;
        }
        Ok(record.owner)
    }).collect()
}

pub(crate) fn active_records_for(m: &Manager, environment: Option<&Environment>) -> Result<Vec<ActiveRecord>> {
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
        let (o, owner_path) = m.lease_owner(sid, &report)?;
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
        let detached = match o.get("detached") {
            None => false,
            Some(v) => v.as_bool().ok_or("active_lease_unresolved")?,
        };
        require(!detached || (!keeper && !inspect && !access && o["shared_runtime"] == true
            && o["finish_setup"].as_bool().is_none_or(|flag| !flag)), "lease_identity")?;
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
            // A retained keeper lease is not a healthy capacity owner after
            // its exact finalizer explicitly disputes cleanup. This durable
            // fact also survives a service restart; an in-memory DSP guard
            // alone cannot authorize the readback or another reservation.
            if report.try_exists()? {
                let result: serde_json::Value = read_json(&report)?;
                if result["ready"] == false && result["cleanup_confirmed"] == false
                    && !environment.is_some_and(|selected| m.lease_environment(sid)
                        .is_ok_and(|owner| owner.id != selected.id && owner.root != selected.root)) {
                    return Err(Refusal::CleanupUnconfirmed.into());
                }
            }
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
            } else if detached {
                Kind::Detached
            } else {
                Kind::Dsp
            }
        };
        result.push(ActiveRecord {
            owner: Owner { session: sid.into(), class_id: class.to_uppercase(), kind, terminal: None },
            document: o, path: owner_path, report,
        });
    }
    result.sort_by(|a, b| a.owner.session.cmp(&b.owner.session));
    Ok(result)
}

/// Caller holds registry.lock. Keepers may be retired only when this setup
/// has no DSP or maintenance users. An unscoped owner cannot be excluded.
pub fn require_environment_inactive(m: &Manager, environment: &Environment) -> Result<()> {
    for owner in owners_for(m, Some(environment))? {
        if owner.kind == Kind::Keeper { continue; }
        require(m.lease_environment(&owner.session).is_ok_and(|scope|
            scope.id != environment.id && scope.root != environment.root),
            if owner.kind.is_instance() { "active_device_lease" } else { "active_maintenance_lease" })?;
    }
    Ok(())
}

fn check_selected(
    owners: &[Owner], limits: &Limits, class: Option<&str>,
    image_capacity: Option<usize>,
) -> Result<()> {
    limits.verify()?;
    if owners
        .iter()
        .any(|o| matches!(o.kind, Kind::Inspection | Kind::VendorAccess))
    {
        return Err(Refusal::MaintenanceActive.into());
    }
    let dsp = owners.iter().filter(|o| o.kind.is_instance()).count();
    let Some(class) = class else {
        return if dsp == 0 {
            Ok(())
        } else {
            Err(Refusal::MaintenanceActive.into())
        };
    };
    // A historical table row names its own class ceiling. Every other class is
    // bounded by the slots of one native engine image, and says so on refusal.
    let (ceiling, full) = match limits.classes.iter().find(|c| c.class_id == class) {
        Some(row) => (row.dsp, Refusal::ClassCapacity),
        None => (image_capacity.ok_or(Refusal::BindingInvalid)?, Refusal::NativeImageCapacity),
    };
    if dsp >= limits.global_dsp {
        return Err(Refusal::GlobalCapacity.into());
    }
    if owners
        .iter()
        .filter(|o| o.kind == Kind::Dsp && o.class_id == class)
        .count()
        >= ceiling
    {
        return Err(full.into());
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
    validate_reservation(m, limits, class)?;
    Ok(lock)
}

/// Native admission is independent of maintenance and uncertain cleanup in
/// other exact setups. Global DSP and per-class ceilings still count every
/// instance. Unknown ownership remains a refusal, not an assumed other setup.
pub fn reserve_environment(
    m: &Manager, limits: &Limits, class: &str, environment: &Environment,
    blocked: impl FnOnce() -> Result<bool>,
) -> Result<Lock> {
    let lock = m.lock("registry.lock").map_err(|e| {
        if e.is::<crate::operator_lock::LockBusy>() { Refusal::ServiceBusy.into() } else { e }
    })?;
    if blocked()? { return Err(Refusal::CleanupUnconfirmed.into()); }
    let mut records = owners_for(m, Some(environment)).map_err(|_| Refusal::CleanupUnconfirmed)?;
    records.retain(|owner| !matches!(owner.kind, Kind::Inspection | Kind::VendorAccess)
        || !m.lease_environment(&owner.session).is_ok_and(|scope|
            scope.id != environment.id && scope.root != environment.root));
    let image = if limits.classes.iter().any(|policy| policy.class_id == class) { None }
        else { image_capacity(m, limits.native_image_hard, class)? };
    check_selected(&records, limits, Some(class), image)?;
    Ok(lock)
}

/// A detached instance owns a Windows host, but no slot in a loaded native
/// engine image. It shares the finite service instance budget and exact setup
/// exclusion with DAW instances; it never acquires maintenance authority.
pub fn reserve_detached(m: &Manager, limits: &Limits, environment: &Environment,
    blocked: impl FnOnce() -> Result<bool>) -> Result<Lock> {
    let lock = m.lock("registry.lock").map_err(|error| {
        if error.is::<crate::operator_lock::LockBusy>() { Refusal::ServiceBusy.into() } else { error }
    })?;
    limits.verify()?;
    if blocked()? { return Err(Refusal::CleanupUnconfirmed.into()); }
    let records = owners_for(m, Some(environment)).map_err(|_| Refusal::CleanupUnconfirmed)?;
    for owner in &records {
        if matches!(owner.kind, Kind::Inspection | Kind::VendorAccess) {
            let scope = m.lease_environment(&owner.session)?;
            if scope.id == environment.id || scope.root == environment.root {
                return Err(Refusal::MaintenanceActive.into());
            }
        }
    }
    if records.iter().filter(|owner| owner.kind.is_instance()).count() >= limits.global_dsp {
        return Err(Refusal::GlobalCapacity.into());
    }
    Ok(lock)
}

/// Service recovery and inspection are control-plane work. Brief fresh
/// readback contention must not close their connection as an apparent plug-in
/// failure. This wait grants no authority: owners and cleanup are checked only
/// after the exact registry guard has been acquired. Native DSP admission keeps
/// the fail-fast crossing above.
pub fn reserve_maintenance(
    m: &Manager, limits: &Limits, blocked: impl Fn() -> bool,
) -> Result<Lock> {
    reserve_maintenance_with_wait(m, limits, blocked, std::time::Duration::from_secs(2))
}
/// Recovery has one overall deadline. Its initial and subsequent reservations
/// may wait for fresh readback, but cannot extend that deadline or reuse owners.
pub fn reserve_maintenance_until(
    m: &Manager, limits: &Limits, blocked: impl Fn() -> bool,
    deadline: std::time::Instant,
) -> Result<Lock> {
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if remaining.is_zero() { return Err(Refusal::ServiceBusy.into()); }
    reserve_maintenance_with_wait(m, limits, blocked, remaining.min(std::time::Duration::from_secs(60)))
}
fn reserve_maintenance_with_wait(
    m: &Manager, limits: &Limits, blocked: impl Fn() -> bool, wait: std::time::Duration,
) -> Result<Lock> {
    if blocked() { return Err(Refusal::CleanupUnconfirmed.into()); }
    let (lock, _) = m.lock_bounded(
        operator_model::OperatorLock::Registry,
        operator_model::LockPurpose::ServiceRecovery, None, wait,
    )?;
    if blocked() { return Err(Refusal::CleanupUnconfirmed.into()); }
    validate_reservation(m, limits, None)?;
    Ok(lock)
}
fn validate_reservation(m: &Manager, limits: &Limits, class: Option<&str>) -> Result<()> {
    let records = owners(m).map_err(|_| Refusal::CleanupUnconfirmed)?;
    let image = match class {
        Some(class) if !limits.classes.iter().any(|policy| policy.class_id == class) =>
            image_capacity(m, limits.native_image_hard, class)?,
        _ => None,
    };
    check_selected(&records, limits, class, image)?;
    Ok(())
}

/// A class with an exact, currently published managed revision may use the
/// native image's structural slot count. This is only an admission ceiling;
/// the shared global reservation and host admission remain separate. Which
/// profile owns the revision is publication authority, not capacity. A stale
/// or ambiguous publication never receives this capacity.
/// Caller holds registry.lock, the same guard used by admission and publication.
fn image_capacity(m: &Manager, native_image_hard: usize, class: &str) -> Result<Option<usize>> {
    image_capacity_with(m, native_image_hard, class, true)
}
fn image_capacity_with(m: &Manager, native_image_hard: usize, class: &str,
    execution: bool) -> Result<Option<usize>> {
    require(valid_hex(class, 32) && class == class.to_uppercase(), "capacity_class_identity")?;
    let registry = m.registry()?;
    let Some(entry) = registry.classes.get(class) else { return Ok(None); };
    if entry.publication != Publication::Published { return Ok(None); }
    // A legacy/manual publication is not reusable-engine admission authority,
    // but its presence must not make the entire service readback unavailable.
    let Some(reference) = entry.managed_revision.as_ref() else { return Ok(None); };
    let revision = if execution { m.load_revision(class, reference)? }
        else { m.load_revision_record(class, reference)? };
    require(revision.class_id == class
        && revision.registration.key() == class
        && revision.registration.metadata.class_id == class
        && entry.registration == revision.registration, "capacity_publication_identity")?;
    require(!m.publication_pending(class)?
        && crate::publication::physical(&m.link(class))? == Some(revision.target.clone()),
        "capacity_publication_changed")?;
    // An adopted legacy revision has no publication transaction to complete.
    if !revision.adopted_legacy { m.verify_completed_publication(&revision, reference)?; }
    Ok(Some(native_image_hard))
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
            "vendor_access":kind==Kind::VendorAccess,"detached":kind==Kind::Detached,
            "shared_runtime":kind==Kind::Detached,
            "registration":{"metadata":{"class_id":class}}}),
        )
        .unwrap();
        let path = f.m.root.join("runtime/leases").join(format!("{sid}.json"));
        private_dir(path.parent().unwrap()).unwrap();
        atomic_json(&path, &report).unwrap();
        path
    }
    fn scoped_lease(f: &Fixture, class: &str, kind: Kind) -> PathBuf {
        let lease = lease(f, class, kind);
        let session = lease.file_stem().unwrap().to_str().unwrap();
        let report: PathBuf = read_json(&lease).unwrap();
        let (mut owner, path) = f.m.lease_owner(session, &report).unwrap();
        owner["lease"] = serde_json::to_value(&lease).unwrap();
        owner["registration"]["environment"] = serde_json::to_value(&f.r.environment).unwrap();
        atomic_json(&path, &owner).unwrap();
        f.m.retain_lease_owner(&path).unwrap();
        lease
    }
    fn other_environment(f: &Fixture) -> Environment {
        let mut other = f.r.environment.clone();
        other.id = "22222222-2222-4222-8222-222222222222".into();
        other.root = f.m.root.join("environments").join(&other.id);
        other
    }
    #[test]
    fn native_admission_isolated_from_exact_other_setup_maintenance() {
        let f = Fixture::new();
        let class = limits().classes[0].class_id.clone();
        let owned = scoped_lease(&f, &class, Kind::Inspection);
        let other = other_environment(&f);
        assert!(reserve_environment(&f.m, &limits(), &class, &other, || Ok(false)).is_ok());
        assert_eq!(reason(reserve_environment(&f.m, &limits(), &class, &f.r.environment,
            || Ok(false))), Refusal::MaintenanceActive.code());
        let mut revised = f.r.environment.clone();
        revised.revision += 1;
        assert_eq!(reason(reserve_environment(&f.m, &limits(), &class, &revised,
            || Ok(false))), Refusal::MaintenanceActive.code());
        assert!(owned.exists(), "Unrelated admission never retires another setup's owner");
        // An unbound historical lease is not permission to assume independence.
        lease(&f, &class, Kind::VendorAccess);
        assert_eq!(reason(reserve_environment(&f.m, &limits(), &class, &other,
            || Ok(false))), Refusal::MaintenanceActive.code());
    }
    #[test]
    fn native_setup_scope_keeps_global_and_class_limits() {
        let f = Fixture::new();
        let class = limits().classes[0].class_id.clone();
        scoped_lease(&f, &class, Kind::Inspection);
        let other = other_environment(&f);
        lease(&f, &class, Kind::Dsp);
        lease(&f, &class, Kind::Dsp);
        assert_eq!(reason(reserve_environment(&f.m, &limits(), &class, &other,
            || Ok(false))), Refusal::ClassCapacity.code());
        let second = limits().classes[1].class_id.clone();
        lease(&f, &second, Kind::Dsp);
        assert_eq!(reason(reserve_environment(&f.m, &limits(), &second, &other,
            || Ok(false))), Refusal::GlobalCapacity.code());
    }
    #[test]
    fn failed_keeper_custody_blocks_its_setup_and_global_maintenance() {
        let f = Fixture::new();
        let class = limits().classes[0].class_id.clone();
        let lease = scoped_lease(&f, &class, Kind::Keeper);
        let report: PathBuf = read_json(&lease).unwrap();
        atomic_json(&report, &serde_json::json!({"ready":false,"cleanup_confirmed":false})).unwrap();
        assert!(reserve_environment(&f.m, &limits(), &class, &other_environment(&f), || Ok(false)).is_ok());
        assert_eq!(reason(reserve_environment(&f.m, &limits(), &class, &f.r.environment,
            || Ok(false))), Refusal::CleanupUnconfirmed.code());
        assert!(owners(&f.m).is_err(), "Global readback still reports unresolved custody");
        assert!(lease.exists());
    }
    #[test]
    fn maintenance_wait_observes_new_owner_before_granting_authority() {
        let f = Fixture::new();
        let held = f.m.lock("registry.lock").unwrap();
        std::thread::scope(|scope| {
            let (tx, rx) = std::sync::mpsc::channel();
            let m = &f.m;
            let reader = scope.spawn(move || {
                tx.send(()).unwrap();
                reason(reserve_maintenance(m, &limits(), || false))
            });
            rx.recv().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(40));
            lease(&f, &limits().classes[0].class_id, Kind::Dsp);
            drop(held);
            assert_eq!(reader.join().unwrap(), Refusal::MaintenanceActive.code());
        });
    }
    #[test]
    fn maintenance_wait_rechecks_cleanup_and_has_a_bound() {
        let f = Fixture::new();
        let held = f.m.lock("registry.lock").unwrap();
        let blocked = std::sync::atomic::AtomicBool::new(false);
        std::thread::scope(|scope| {
            let (tx, rx) = std::sync::mpsc::channel();
            let m = &f.m;
            let blocked = &blocked;
            let reader = scope.spawn(move || {
                tx.send(()).unwrap();
                reason(reserve_maintenance(m, &limits(), ||
                    blocked.load(std::sync::atomic::Ordering::Acquire)))
            });
            rx.recv().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(40));
            blocked.store(true, std::sync::atomic::Ordering::Release);
            drop(held);
            assert_eq!(reader.join().unwrap(), Refusal::CleanupUnconfirmed.code());
        });
        let _held = f.m.lock("registry.lock").unwrap();
        let error = reserve_maintenance_with_wait(&f.m, &limits(), || false,
            std::time::Duration::from_millis(20)).err().unwrap();
        let failure = error.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap();
        assert_eq!(failure.facts.outcome, operator_model::LockOutcome::Timeout);
        assert_eq!(failure.facts.purpose, operator_model::LockPurpose::ServiceRecovery);
        assert!(matches!(reserve(&f.m, &limits(), None, false).err().unwrap()
            .downcast_ref::<Refusal>(), Some(Refusal::ServiceBusy)));
    }
    #[test]
    fn maintenance_wait_returns_a_real_guard_and_refuses_broken_ownership() {
        let f = Fixture::new();
        let held = reserve_maintenance(&f.m, &limits(), || false).unwrap();
        held.require_registry(&f.m).unwrap();
        assert!(reserve(&f.m, &limits(), None, false).is_err());
        drop(held);
        private_dir(&f.m.root.join("runtime/leases")).unwrap();
        fs::write(f.m.root.join("runtime/leases/broken.json"), b"invalid").unwrap();
        assert_eq!(reason(reserve_maintenance(&f.m, &limits(), || false)),
            Refusal::CleanupUnconfirmed.code());
    }
    #[test]
    fn recovery_reservation_uses_remaining_budget_and_rechecks_cleanup() {
        let f=Fixture::new();
        let blocked=std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let held=f.m.lock("registry.lock").unwrap();
        let changed=blocked.clone();
        let owner=std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(30));
            changed.store(true,std::sync::atomic::Ordering::Release);
            drop(held);
        });
        let error=reserve_maintenance_until(&f.m,&limits(),
            || blocked.load(std::sync::atomic::Ordering::Acquire),
            std::time::Instant::now()+std::time::Duration::from_secs(1)).err().unwrap();
        owner.join().unwrap();
        assert_eq!(error.downcast_ref::<Refusal>(),Some(&Refusal::CleanupUnconfirmed));
        assert_eq!(reason(reserve_maintenance_until(&f.m,&limits(),|| false,
            std::time::Instant::now())),Refusal::ServiceBusy.code());
        let held=f.m.lock("registry.lock").unwrap();
        let started=std::time::Instant::now();
        let error=reserve_maintenance_until(&f.m,&limits(),|| false,
            started+std::time::Duration::from_millis(20)).err().unwrap();
        assert!(started.elapsed()<std::time::Duration::from_secs(1));
        assert_eq!(error.downcast_ref::<operator_lock::AcquisitionFailure>().unwrap()
            .facts.outcome,operator_model::LockOutcome::Timeout);
        drop(held);
    }
    #[test]
    fn observed_status_yields_to_writer_and_preserves_the_actual_census() {
        let f = Fixture::new();
        let held = f.m.lock("registry.lock").unwrap();
        assert_eq!(status_observed(&f.m, limits(), 1, false).unwrap_err().to_string(),
            "capacity_observation_busy");
        lease(&f, &limits().classes[0].class_id, Kind::Inspection);
        drop(held);
        let expected = status(&f.m, limits(), 1, true).unwrap();
        let shared = f.m.try_lock_shared("registry.lock").unwrap();
        assert!(matches!(shared, operator_lock::LockAttempt::Acquired(_)));
        let observed = status_observed(&f.m, limits(), 1, true).unwrap();
        assert_eq!(serde_json::to_value(observed).unwrap(), serde_json::to_value(expected).unwrap());
        drop(shared);
        fs::write(f.m.root.join("runtime/leases/broken.json"), b"invalid").unwrap();
        assert!(status_observed(&f.m, limits(), 1, false).is_err());
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
    fn manager_ownership_survives_supervisor_directory_retirement() {
        let f = Fixture::new();
        let class = limits().classes[0].class_id.clone();
        let lease = lease(&f, &class, Kind::Dsp);
        let sid = lease.file_stem().unwrap().to_str().unwrap();
        let report: PathBuf = read_json(&lease).unwrap();
        let temporary = f.r.environment.root
            .join("compatdata/pfx/drive_c/bridge/sessions").join(sid).join("owner.json");
        let mut owner: serde_json::Value = read_json(&temporary).unwrap();
        owner["lease"] = serde_json::to_value(&lease).unwrap();
        owner["registration"]["environment"] = serde_json::to_value(&f.r.environment).unwrap();
        atomic_json(&temporary, &owner).unwrap();
        {
            let _registry = f.m.lock("registry.lock").unwrap();
            f.m.retain_lease_owner(&temporary).unwrap();
            f.m.retain_lease_owner(&temporary).unwrap();
        }
        // Reproduce the actual installed ordering: the supervisor has removed
        // its Windows session view and reported cleanup, but Rust owns the lease.
        fs::remove_dir_all(temporary.parent().unwrap()).unwrap();
        atomic_json(&report, &serde_json::json!({"session":sid,
            "cleanup_confirmed":true,"transport_retired":true})).unwrap();
        let restarted = Manager::new(f.m.root.clone(), f.m.publications.clone());
        let _registry = restarted.lock("registry.lock").unwrap();
        let observed = owners(&restarted).unwrap();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].kind, Kind::Dsp);
        assert_eq!(observed[0].class_id, class);
        assert_eq!(restarted.require_inactive(None).unwrap_err().to_string(), "active_device_lease");
        // Only the manager's exact release makes this capacity free.
        fs::remove_file(&lease).unwrap();
        assert!(owners(&restarted).unwrap().is_empty());
        restarted.require_inactive(None).unwrap();
    }
    #[test]
    fn missing_legacy_owner_and_changed_retained_binding_never_count_as_free() {
        let f = Fixture::new();
        let class = limits().classes[0].class_id.clone();
        let lease = lease(&f, &class, Kind::Dsp);
        let sid = lease.file_stem().unwrap().to_str().unwrap();
        let report: PathBuf = read_json(&lease).unwrap();
        let temporary = f.r.environment.root
            .join("compatdata/pfx/drive_c/bridge/sessions").join(sid).join("owner.json");
        let mut owner: serde_json::Value = read_json(&temporary).unwrap();
        owner["lease"] = serde_json::to_value(&lease).unwrap();
        owner["registration"]["environment"] = serde_json::to_value(&f.r.environment).unwrap();
        atomic_json(&temporary, &owner).unwrap();
        let _registry = f.m.lock("registry.lock").unwrap();
        f.m.retain_lease_owner(&temporary).unwrap();
        let retained = f.m.root.join("runtime/lease-owners").join(format!("{sid}.json"));
        let original: serde_json::Value = read_json(&retained).unwrap();
        for changed in [serde_json::json!({"schema":2}),
            serde_json::json!({"owner":{"report":f.m.root.join("runtime/results/other.json")}}),
            serde_json::json!({"temporary_owner":f.m.root.join("outside/owner.json")})] {
            let mut bad = original.clone();
            for (key, value) in changed.as_object().unwrap() {
                if key == "owner" { bad["owner"]["report"] = value["report"].clone(); }
                else { bad[key] = value.clone(); }
            }
            atomic_json(&retained, &bad).unwrap();
            assert!(owners(&f.m).is_err());
            assert!(f.m.require_inactive(None).is_err());
        }
        fs::remove_file(&retained).unwrap();
        fs::remove_dir_all(temporary.parent().unwrap()).unwrap();
        atomic_json(&report, &serde_json::json!({"cleanup_confirmed":true,"transport_retired":true})).unwrap();
        assert_eq!(owners(&f.m).unwrap_err().to_string(), "active_lease_unresolved");
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
        let restarted = Manager::new(f.m.root.clone(), f.m.publications.clone());
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
    fn exact_failed_keeper_report_blocks_readback_and_admission_after_reconstruction() {
        let f = Fixture::new();
        let p = limits();
        let a = &p.classes[0].class_id;
        let retained = lease(&f, a, Kind::Keeper);
        let report: PathBuf = read_json(&retained).unwrap();
        atomic_json(&report, &serde_json::json!({"ready":false,"cleanup_confirmed":false})).unwrap();
        let restarted = Manager::new(f.m.root.clone(), f.m.publications.clone());
        let failure=status(&restarted,p.clone(),1,false).unwrap_err();
        assert_eq!(failure.downcast_ref::<Refusal>(),Some(&Refusal::CleanupUnconfirmed));
        assert_eq!(reason(reserve(&restarted,&p,Some(a),false)),Refusal::CleanupUnconfirmed.code());
        assert!(retained.exists());
        // A positive final report alone still does not remove the owner lease.
        atomic_json(&report, &serde_json::json!({"ready":false,"cleanup_confirmed":true})).unwrap();
        assert_eq!(owners(&restarted).unwrap().len(),1);
        assert!(retained.exists());
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
    fn a_legacy_publication_does_not_hide_service_capacity_or_gain_managed_slots() {
        let f = Fixture::new();
        let key = f.r.key();
        let mut policy = limits();
        policy.classes[0].class_id = "03".repeat(16);
        f.m.register(f.r.clone()).unwrap();
        assert!(!policy.classes.iter().any(|c| c.class_id == key));
        let observed = status(&f.m, policy.clone(), 0, false).unwrap();
        assert_eq!(observed.dsp, 0);
        assert!(!observed.limits.classes.iter().any(|c| c.class_id == key));
        assert_eq!(reason(reserve(&f.m, &policy, Some(&key), false)), Refusal::BindingInvalid.code());
    }

    #[test]
    fn readable_unfamiliar_capacity_records_do_not_grant_changed_native_admission() {
        let (f, candidate) = crate::preparation::tests::fixture();
        crate::preparation::record_candidate(&f.m, &candidate).unwrap();
        let reference = crate::preparation::enable(&f.m, &candidate, false).unwrap();
        let class = &candidate.selection.class.id;
        let mut policy = limits();
        policy.classes[0].class_id = "03".repeat(16);
        let publication = f.m.load_revision(class, &reference).unwrap();
        let path = &publication.registration.native.path;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(path, b"changed selected native bytes").unwrap();
        let status = status(&f.m, policy.clone(), 0, false).unwrap();
        assert_eq!(status.dsp, 0);
        assert!(status.class_ceilings.iter().any(|row| row.class_id == *class));
        assert!(f.m.load_revision_record(class, &reference).is_ok());
        assert_eq!(reserve(&f.m, &policy, Some(class), false).err().unwrap().to_string(),
            "artifact missing or changed");
        assert_eq!(f.m.resolve(&f.identity()).err().unwrap().to_string(), "artifact missing or changed");
        assert_eq!(f.m.registry().unwrap().classes[class].managed_revision, Some(reference));
        assert!(owners(&f.m).unwrap().is_empty());
    }
    #[test]
    fn unfamiliar_slots_require_a_current_managed_publication_and_durable_reservations() {
        let (f, candidate) = crate::preparation::tests::fixture();
        crate::preparation::record_candidate(&f.m, &candidate).unwrap();
        let revision = crate::preparation::enable(&f.m, &candidate, false).unwrap();
        let class = &candidate.selection.class.id;
        let mut policy = limits();
        policy.global_dsp = 6;
        policy.classes[0] = ClassLimit {class_id:"03".repeat(16), dsp:3};
        policy.classes[1].dsp = 4;
        assert!(!policy.classes.iter().any(|row| row.class_id == *class));
        let observed = status(&f.m, policy.clone(), 0, false).unwrap();
        assert_eq!(observed.class_ceilings.iter().find(|row| row.class_id == *class).unwrap().dsp, 4);
        for _ in 0..4 {
            let guard = reserve(&f.m, &policy, Some(class), false).unwrap();
            lease(&f, class, Kind::Dsp);
            drop(guard);
        }
        assert_eq!(reason(reserve(&f.m, &policy, Some(class), false)), Refusal::NativeImageCapacity.code());
        // Valid leases cannot grant admission for a substituted publication.
        let selected = f.m.load_revision(class, &revision).unwrap();
        fs::remove_file(f.m.link(class)).unwrap();
        std::os::unix::fs::symlink(selected.target.with_extension("changed"), f.m.link(class)).unwrap();
        let observed = status(&f.m, policy.clone(), 0, false).unwrap();
        assert_eq!(observed.dsp, 4);
        assert_eq!(observed.owners, owners(&f.m).unwrap());
        assert_eq!(observed.available_dsp, 2);
        assert!(!observed.class_ceilings.iter().any(|row| row.class_id == *class));
        assert!(reserve(&f.m, &policy, Some(class), false).is_err());
    }

    fn interrupted_unfamiliar_replacement() -> (Fixture, crate::preparation::Candidate, Limits, PathBuf) {
        let (f, candidate) = crate::preparation::tests::fixture();
        crate::preparation::record_candidate(&f.m, &candidate).unwrap();
        let baseline = crate::preparation::enable(&f.m, &candidate, false).unwrap();
        let mut policy = limits();
        policy.global_dsp = 6;
        policy.classes[0].class_id = "03".repeat(16);
        assert!(!policy.classes.iter().any(|row| row.class_id == candidate.selection.class.id));
        let sibling = {
            let guard = reserve(&f.m, &policy, Some(&policy.classes[1].class_id), false).unwrap();
            let lease = lease(&f, &policy.classes[1].class_id, Kind::Dsp);
            drop(guard);
            lease
        };
        let next = crate::preparation::configuration::prepare(&f.m, &candidate,
            Some(operator_model::GraphicsBackend::WineD3d11), Some(&baseline)).unwrap();
        f.m.publish_with_expected(&next.profile, &next.census().unwrap(),
            crate::preparation::configuration::registration(&next).unwrap(),
            (&next.host, &next.source_manifest.sha256),
            (Some(crate::publication::Qualification::ManagedExperimental), false),
            Some(crate::publication::Boundary::Intent), Some(&baseline)).unwrap_err();
        assert!(f.m.publication_pending(&candidate.selection.class.id).unwrap());
        (f, candidate, policy, sibling)
    }

    #[test]
    fn interrupted_unfamiliar_publication_keeps_canonical_owners_and_reconciliation_available() {
        let (f, candidate, policy, sibling) = interrupted_unfamiliar_replacement();
        let class = &candidate.selection.class.id;
        let entry = f.m.registry().unwrap().classes[class].clone();
        assert_eq!(entry.publication, Publication::Published);
        let live_owners = owners(&f.m).unwrap();
        let observed = status(&f.m, policy.clone(), 0, false).unwrap();
        assert_eq!(observed.owners, live_owners);
        assert_eq!(observed.dsp, 1);
        assert_eq!(observed.per_class[&policy.classes[1].class_id], 1);
        // Only the two eligible policy classes contribute free slots: two
        // for the first class and one remaining beside the live sibling.
        assert_eq!(observed.available_dsp, 3);
        assert!(!observed.cleanup_unconfirmed);
        assert!(!observed.maintenance_admissible);
        assert!(!observed.class_ceilings.iter().any(|row| row.class_id == *class));
        assert_eq!(reason(reserve(&f.m, &policy, Some(class), false)), "capacity_publication_changed");
        drop(reserve(&f.m, &policy, Some(&policy.classes[1].class_id), false).unwrap());
        assert_eq!(owners(&f.m).unwrap(), live_owners);
        assert!(f.m.reconcile_inactive().is_err());
        assert_eq!(f.m.registry().unwrap().classes[class], entry);

        // Explicitly retire this fixture's exact sibling lease. A positive
        // report alone cannot make it disappear from canonical readback.
        let report: PathBuf = read_json(&sibling).unwrap();
        atomic_json(&report, &serde_json::json!({"cleanup_confirmed":true,
            "transport_retired":true})).unwrap();
        assert_eq!(status(&f.m, policy.clone(), 0, false).unwrap().dsp, 1);
        fs::remove_file(sibling).unwrap();
        let retired = status(&f.m, policy.clone(), 0, false).unwrap();
        assert!(retired.owners.is_empty() && retired.maintenance_admissible);
        f.m.reconcile_inactive().unwrap();
        assert!(!f.m.publication_pending(class).unwrap());
        assert_eq!(f.m.registry().unwrap().classes[class], entry);
        drop(reserve(&f.m, &policy, Some(class), false).unwrap());
    }

    #[test]
    fn invalid_owner_still_refuses_readback_and_admission_during_publication_recovery() {
        let (f, _, policy, _) = interrupted_unfamiliar_replacement();
        private_dir(&f.m.root.join("runtime/leases")).unwrap();
        fs::write(f.m.root.join("runtime/leases/broken.json"), b"invalid").unwrap();
        assert!(status(&f.m, policy.clone(), 0, false).is_err());
        assert_eq!(reason(reserve(&f.m, &policy, Some(&policy.classes[1].class_id), false)),
            Refusal::CleanupUnconfirmed.code());
        assert_eq!(reason(reserve_maintenance(&f.m, &policy, || false)),
            Refusal::CleanupUnconfirmed.code());
        assert!(f.m.reconcile_inactive().is_err());
    }

    #[test]
    fn unfamiliar_managed_classes_use_native_slots_and_share_the_global_budget() {
        let mut policy = limits();
        policy.global_dsp = 6;
        policy.classes[0].dsp = 3;
        policy.classes[1].dsp = 4;
        let unfamiliar = "03".repeat(16);
        let mut active = Vec::new();

        // The managed-class resolver supplies the structural native image
        // ceiling, while every reservation still consumes the shared global
        // pool. This does not make the limit a tested workload claim.
        for _ in 0..policy.native_image_hard {
            check_selected(&active, &policy, Some(&unfamiliar), Some(policy.native_image_hard)).unwrap();
            active.push(Owner { session: random_id().unwrap(), class_id: unfamiliar.clone(), kind: Kind::Dsp, terminal: None });
        }
        assert_eq!(
            check_selected(&active, &policy, Some(&unfamiliar), Some(policy.native_image_hard))
                .unwrap_err().downcast_ref::<Refusal>(),
            Some(&Refusal::NativeImageCapacity)
        );

        // A separate mixed-class run demonstrates that independent class
        // slots still draw from one finite service-wide reservation.
        active.clear();
        for _ in 0..3 {
            check_selected(&active, &policy, Some(&policy.classes[0].class_id), None).unwrap();
            active.push(Owner { session: random_id().unwrap(), class_id: policy.classes[0].class_id.clone(), kind: Kind::Dsp, terminal: None });
        }
        for _ in 0..3 {
            check_selected(&active, &policy, Some(&unfamiliar), Some(policy.native_image_hard)).unwrap();
            active.push(Owner { session: random_id().unwrap(), class_id: unfamiliar.clone(), kind: Kind::Dsp, terminal: None });
        }
        assert_eq!(
            check_selected(&active, &policy, Some(&unfamiliar), Some(policy.native_image_hard))
                .unwrap_err().downcast_ref::<Refusal>(),
            Some(&Refusal::GlobalCapacity)
        );
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
                    .filter(|o| o.kind.is_instance())
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

    /// A durable owner bound to `environment`, as admission and maintenance retain it.
    fn lease_in(f: &Fixture, class: &str, kind: Kind, environment: &Environment) -> PathBuf {
        let sid = random_id().unwrap();
        let report = f.m.root.join("runtime/results").join(format!(
            "{}-{sid}.json", if kind == Kind::Keeper { "environment" } else { "windows" }));
        private_dir(report.parent().unwrap()).unwrap();
        let lease = f.m.root.join("runtime/leases").join(format!("{sid}.json"));
        let owner = environment.root.join("compatdata/pfx/drive_c/bridge/sessions")
            .join(&sid).join("owner.json");
        private_dir(owner.parent().unwrap()).unwrap();
        atomic_json(&owner, &serde_json::json!({"session":sid,"report":report,"lease":lease,
            "keeper":kind==Kind::Keeper,"inspect":matches!(kind, Kind::Keeper|Kind::Inspection),
            "vendor_access":kind==Kind::VendorAccess,"detached":kind==Kind::Detached,
            "shared_runtime":kind==Kind::Detached,
            "registration":{"environment":environment,"metadata":{"class_id":class}}})).unwrap();
        f.m.retain_lease_owner(&owner).unwrap();
        private_dir(lease.parent().unwrap()).unwrap();
        atomic_json(&lease, &report).unwrap();
        lease
    }
    /// One exactly published plug-in class in the fixture setup, and a second setup.
    fn production_fixture() -> (Fixture, String, Environment) {
        let (f, candidate) = crate::preparation::tests::fixture();
        crate::preparation::record_candidate(&f.m, &candidate).unwrap();
        crate::preparation::enable(&f.m, &candidate, false).unwrap();
        let other = other_environment(&f);
        (f, candidate.selection.class.id.clone(), other)
    }
    #[test]
    fn detached_windows_share_instance_budget_and_setup_custody_but_not_native_image_slots() {
        let (f,class,other)=production_fixture();
        let limits=service_limits();
        let mut leases=vec![];
        for _ in 0..limits.native_image_hard {
            let _guard=reserve_environment(&f.m,&limits,&class,&f.r.environment,||Ok(false)).unwrap();
            leases.push(lease_in(&f,&class,Kind::Dsp,&f.r.environment));
        }
        assert_eq!(reason(reserve_environment(&f.m,&limits,&class,&f.r.environment,||Ok(false))),
            Refusal::NativeImageCapacity.code());
        // A full native image does not own this separate host's editor.
        {
            let _guard=reserve_detached(&f.m,&limits,&f.r.environment,||Ok(false)).unwrap();
            leases.push(lease_in(&f,&class,Kind::Detached,&f.r.environment));
        }
        assert!(require_environment_inactive(&f.m,&f.r.environment).is_err());
        assert!(require_environment_inactive(&f.m,&other).is_ok());
        assert_eq!(owners(&f.m).unwrap().iter().filter(|o|o.kind.is_instance()).count(),5);
        let other_maintenance=lease_in(&f,&class,Kind::Inspection,&other);
        drop(reserve_detached(&f.m,&limits,&f.r.environment,||Ok(false)).unwrap());
        assert_eq!(reason(reserve_detached(&f.m,&limits,&other,||Ok(false))),Refusal::MaintenanceActive.code());
        fs::remove_file(other_maintenance).unwrap();
        while leases.len()<limits.global_dsp {
            let _guard=reserve_detached(&f.m,&limits,&f.r.environment,||Ok(false)).unwrap();
            leases.push(lease_in(&f,&class,Kind::Detached,&f.r.environment));
        }
        assert_eq!(reason(reserve_detached(&f.m,&limits,&other,||Ok(false))),Refusal::GlobalCapacity.code());
        // Exact retirement returns only its ordinary unit, preserving siblings.
        fs::remove_file(leases.pop().unwrap()).unwrap();
        drop(reserve_detached(&f.m,&limits,&other,||Ok(false)).unwrap());
        assert!(leases.iter().all(|path|path.exists()));
    }
    /// The production admission path: reserve, then publish the lease under the reservation.
    fn admit(f: &Fixture, class: &str) -> Result<PathBuf> {
        let _reservation = reserve_environment(&f.m, &service_limits(), class, &f.r.environment,
            || Ok(false))?;
        Ok(lease_in(f, class, Kind::Dsp, &f.r.environment))
    }
    fn available(f: &Fixture) -> usize {
        status(&f.m, service_limits(), 0, false).unwrap().available_dsp
    }
    #[test]
    fn production_policy_is_explicit_finite_and_inspectable() {
        let production = service_limits();
        production.verify().unwrap();
        assert_eq!((production.global_dsp, production.service_workers, production.maintenance,
            production.native_image_hard), (8, 16, 1, 4));
        assert!(production.classes.is_empty());
        assert!(production.service_workers > production.global_dsp + production.maintenance);
        // The historical qualification policy is unchanged and still valid.
        let historical = fixture_limits();
        historical.verify().unwrap();
        assert_eq!((historical.global_dsp, historical.classes.len()), (6, 2));
        let f = Fixture::new();
        let observed = status(&f.m, production.clone(), 0, false).unwrap();
        assert_eq!(observed.limits, production);
        assert!(observed.envelope.is_none());
        assert!(serde_json::to_value(&observed).unwrap().get("envelope").is_none());
        assert!(status(&f.m, historical, 0, false).unwrap().envelope.is_some());
    }
    #[test]
    fn production_policy_admits_a_seventh_and_eighth_and_names_the_global_limit() {
        let (f, class, other) = production_fixture();
        // Six instances of three other classes across two setups; no image is full.
        for id in ["03", "04", "05"] {
            lease_in(&f, &id.repeat(16), Kind::Dsp, &f.r.environment);
            lease_in(&f, &id.repeat(16), Kind::Dsp, &other);
        }
        assert_eq!(available(&f), 2);
        let seventh = admit(&f, &class).unwrap();
        let eighth = admit(&f, &class).unwrap();
        assert_ne!(seventh, eighth);
        let before = owners(&f.m).unwrap();
        assert_eq!(before.len(), 8);
        assert_eq!(available(&f), 0);
        assert_eq!(admit(&f, &class).unwrap_err().to_string(), Refusal::GlobalCapacity.code());
        assert_eq!(owners(&f.m).unwrap(), before, "a refusal leaves no partial owner");
    }
    #[test]
    fn production_policy_names_a_full_native_image() {
        let (f, class, other) = production_fixture();
        lease_in(&f, &"03".repeat(16), Kind::Dsp, &other);
        for _ in 0..NATIVE_IMAGE_SLOTS {
            admit(&f, &class).unwrap();
        }
        assert_eq!(admit(&f, &class).unwrap_err().to_string(), Refusal::NativeImageCapacity.code());
        let observed = status(&f.m, service_limits(), 0, false).unwrap();
        assert_eq!((observed.dsp, observed.per_class[&class], observed.available_dsp), (5, 4, 0));
    }
    #[test]
    fn production_concurrent_admission_cannot_over_admit() {
        let (f, class, other) = production_fixture();
        for id in ["03", "04", "05"] {
            lease_in(&f, &id.repeat(16), Kind::Dsp, &other);
        }
        let barrier = std::sync::Barrier::new(12);
        let (f, class, barrier) = (&f, &class, &barrier);
        let accepted = std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..12).map(move |_| scope.spawn(move || {
                barrier.wait();
                // The production API refuses contention; test clients retry
                // Busy with a finite bound, outside any DAW.
                for _ in 0..1000 {
                    match admit(f, class) {
                        Ok(_) => return true,
                        Err(e) if e.to_string() == Refusal::ServiceBusy.code() => std::thread::yield_now(),
                        Err(e) => {
                            assert!(e.to_string() == Refusal::NativeImageCapacity.code()
                                || e.to_string() == Refusal::GlobalCapacity.code(), "{e}");
                            return false;
                        }
                    }
                }
                false
            })).collect();
            jobs.into_iter().map(|job| job.join().unwrap()).filter(|accepted| *accepted).count()
        });
        assert!(accepted <= NATIVE_IMAGE_SLOTS);
        // All contenders have joined; every remaining unit is still available.
        for _ in accepted..NATIVE_IMAGE_SLOTS {
            admit(f, class).unwrap();
        }
        assert_eq!(admit(f, class).unwrap_err().to_string(), Refusal::NativeImageCapacity.code());
        let records = owners(&f.m).unwrap();
        assert_eq!(records.iter().filter(|o| o.class_id == *class).count(), NATIVE_IMAGE_SLOTS);
        assert_eq!(records.len(), 3 + NATIVE_IMAGE_SLOTS);
    }
    #[test]
    fn production_failed_startup_and_release_return_capacity() {
        let (f, class, other) = production_fixture();
        for id in ["03", "04"] {
            for _ in 0..3 {
                lease_in(&f, &id.repeat(16), Kind::Dsp, &other);
            }
        }
        let seventh = admit(&f, &class).unwrap();
        // A startup that fails before publishing its lease returns its unit.
        drop(reserve_environment(&f.m, &service_limits(), &class, &f.r.environment,
            || Ok(false)).unwrap());
        assert_eq!(owners(&f.m).unwrap().len(), 7);
        let eighth = admit(&f, &class).unwrap();
        assert_eq!(admit(&f, &class).unwrap_err().to_string(), Refusal::GlobalCapacity.code());
        // Removing an instance releases exactly one unit, and reloading it is admitted.
        fs::remove_file(&eighth).unwrap();
        assert_eq!(available(&f), 1);
        let reloaded = admit(&f, &class).unwrap();
        assert_ne!(reloaded, eighth);
        assert!(seventh.exists());
        assert_eq!(admit(&f, &class).unwrap_err().to_string(), Refusal::GlobalCapacity.code());
    }
    #[test]
    fn production_policy_slots_do_not_depend_on_preparation_owning_the_profile() {
        // The Serum review candidate is published by its own qualification
        // path; preparation does not own its profile.
        let (f, package, _contract) = crate::managed_candidate::tests::fixture();
        crate::managed_candidate::stage(&f.m, &package).unwrap();
        crate::managed_candidate::publish(&f.m).unwrap();
        let class = crate::managed_candidate::candidate().unwrap().class.class_id;
        assert!(!crate::preparation::owns_profile(&f.m, &crate::managed_candidate::candidate().unwrap()).unwrap());
        for _ in 0..NATIVE_IMAGE_SLOTS {
            admit(&f, &class).unwrap();
        }
        assert_eq!(admit(&f, &class).unwrap_err().to_string(), Refusal::NativeImageCapacity.code());
    }
    #[test]
    fn production_policy_slots_include_an_adopted_legacy_revision() {
        let (f, p, census, native) = crate::test_fixture::prepared();
        let class = f.r.key();
        let current = f.m.managed_publish(&p, &census, crate::observation::derive(&p, &census, &native).unwrap(),
            &census.host, &census.host_source_sha256, None).unwrap();
        let legacy = f.m.load_revision(&class, &current).unwrap().parent.unwrap();
        f.m.rollback(&class, &legacy.id, None).unwrap();
        assert!(f.m.load_revision(&class, &legacy).unwrap().adopted_legacy);
        let observed = status(&f.m, service_limits(), 0, false).unwrap();
        assert_eq!(observed.class_ceilings.iter().find(|c| c.class_id == class).map(|c| c.dsp),
            Some(NATIVE_IMAGE_SLOTS));
        admit(&f, &class).unwrap();
        assert_eq!(status(&f.m, service_limits(), 0, false).unwrap().per_class[&class], 1);
    }
    #[test]
    fn availability_follows_setup_scoped_maintenance_like_admission() {
        let (f, class, other) = production_fixture();
        let admissible = |f: &Fixture| reserve_environment(&f.m, &service_limits(), &class,
            &f.r.environment, || Ok(false)).map(drop);
        assert_eq!(available(&f), NATIVE_IMAGE_SLOTS);
        // Maintenance in another setup leaves this setup's plug-in admissible.
        let unrelated = lease_in(&f, &"03".repeat(16), Kind::VendorAccess, &other);
        let observed = status(&f.m, service_limits(), 0, false).unwrap();
        assert_eq!((observed.maintenance, observed.available_dsp), (1, NATIVE_IMAGE_SLOTS));
        assert!(!observed.maintenance_admissible);
        admissible(&f).unwrap();
        fs::remove_file(&unrelated).unwrap();
        // Maintenance in the plug-in's own setup excludes it in both views.
        let own = lease_in(&f, &class, Kind::Inspection, &f.r.environment);
        assert_eq!(available(&f), 0);
        assert_eq!(admissible(&f).unwrap_err().to_string(), Refusal::MaintenanceActive.code());
        fs::remove_file(&own).unwrap();
        assert_eq!(available(&f), NATIVE_IMAGE_SLOTS);
        // Maintenance whose setup cannot be resolved excludes every setup.
        lease(&f, &class, Kind::Inspection);
        assert_eq!(available(&f), 0);
        assert_eq!(admissible(&f).unwrap_err().to_string(), Refusal::MaintenanceActive.code());
    }
}

//! One bounded, read-only live-instance census, shared by Activity and pushes.
//! No executable verification, audio payload access, mutation, or callback work.
use crate::{operator_model::*, *};
use std::{os::unix::fs::FileExt, sync::Arc, time::Instant};
mod host;
pub use host::{HostNames, GREETING as HOST_GREETING};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Progress { words: [u64; 24] }

#[derive(Clone, Debug, PartialEq, Eq)]
struct CounterKey {
    device: u64,
    inode: u64,
    mapping_device: u64,
    mapping_inode: u64,
    generation: u64,
    epoch: u64,
}

struct Previous { key: CounterKey, progress: Progress, observed: Instant }

#[derive(Default)]
pub struct Observer { previous: BTreeMap<String, Previous>, hosts: Arc<HostNames> }

impl Observer {
    pub fn with_hosts(hosts: Arc<HostNames>) -> Self { Self { hosts, ..Self::default() } }
    /// At most the existing 64 leases. Busy, unresolved or changing custody
    /// is unavailable coverage, never an empty/clean census. No blocking wait.
    pub fn sample(&mut self, manager: &Manager) -> LiveActivity {
        let records = match capture(manager) {
            Ok(records) => records,
            Err(_) => {
                self.previous.clear();
                return LiveActivity::unavailable("Current instance ownership is unavailable");
            }
        };
        let now = Instant::now();
        let rows = records.iter().filter(|record| record.owner.kind.is_instance())
            .map(|record| self.instance(manager, record, now)).collect::<Vec<_>>();
        // The slow reads above never hold registry custody. Do not publish a
        // replaced census or continue comparisons through a missed generation.
        if !capture(manager).is_ok_and(|current| same_records(&records, &current)) {
            self.previous.clear();
            return LiveActivity::unavailable("Instance ownership changed during this read");
        }
        self.previous.retain(|session, _| rows.iter().any(|row| &row.session == session));
        LiveActivity { instances: Observation::Observed(rows) }
    }

    fn instance(&mut self, manager: &Manager, record: &capacity::ActiveRecord, now: Instant) -> LiveInstance {
        let session = &record.owner.session;
        let product = observed(product(record), "The admitted plug-in identity is unavailable");
        let consumer = observed(owner::lease_consumer(manager, session, &record.report, &record.document)
            .map(|identity| ConsumerObservation {
                current: observed(transport_storage::consumer_current(&identity), "The admitted process could not be checked"),
                identity,
            }), "The admitted process has no complete kernel identity");
        let daw = self.hosts.observed(manager, record, &consumer);
        let mut row = LiveInstance {
            kind: if record.owner.kind == capacity::Kind::Detached { InstanceKind::Standalone } else { InstanceKind::Daw },
            session: session.clone(), product, consumer,
            daw,
            transport: Observation::Unavailable("Transport progress is unavailable".into()),
            lifecycle: Observation::Unavailable("No lifecycle report is available".into()),
            processing: ProcessingObservation::Unavailable { reason:"Transport progress is unavailable".into() },
            signal: SignalObservation::NotMeasured,
            terminal: Observation::Unavailable("Terminal status is unavailable".into()),
            cleanup: cleanup(&record.report, session),
        };
        if record.owner.kind == capacity::Kind::Detached {
            self.previous.remove(session);
            row.consumer = Observation::Unavailable("Opened on its own; no DAW consumer".into());
            row.daw = Observation::Unavailable("Opened on its own; no DAW host report".into());
            row.processing = ProcessingObservation::Unavailable { reason:"Opened on its own; no DAW audio device".into() };
            return row;
        }
        let snapshot = read_transport(record);
        match snapshot {
            Ok(snapshot) => {
                row.terminal = snapshot.terminal;
                if let Some(progress) = snapshot.progress {
                    let generation = progress.words[3];
                    let epoch = progress.words[4];
                    row.transport = Observation::Observed(TransportIdentity {
                        device:snapshot.identity.device, inode:snapshot.identity.inode, generation,
                    });
                    row.lifecycle = lifecycle(progress.words[17]);
                    let key = CounterKey {
                        device:snapshot.identity.device, inode:snapshot.identity.inode,
                        mapping_device:snapshot.mapping.0, mapping_inode:snapshot.mapping.1,
                        generation, epoch,
                    };
                    if matches!(&row.consumer, Observation::Observed(consumer)
                        if consumer.current == Observation::Observed(true))
                        && matches!(row.terminal, Observation::Observed(None)) {
                        row.processing = self.processing(session, key, progress, now);
                    } else {
                        self.previous.remove(session);
                        row.processing = ProcessingObservation::Unavailable {
                            reason:"The admitted consumer or terminal status is not confirmed current".into(),
                        };
                    }
                } else { self.previous.remove(session); }
            }
            Err(_) => { self.previous.remove(session); }
        }
        row
    }

    fn processing(&mut self, session: &str, key: CounterKey, progress: Progress, now: Instant)
        -> ProcessingObservation {
        let previous = self.previous.remove(session).filter(|previous| previous.key == key);
        let observed = previous.as_ref().filter(|previous| previous.progress == progress)
            .map_or(now, |previous| previous.observed);
        let age = now.saturating_duration_since(observed).as_millis().try_into().unwrap_or(u64::MAX);
        let epoch = key.epoch;
        let completed_end = progress.words[6];
        let result = if progress.words[5] == u64::MAX || progress.words[16] == u64::MAX {
            ProcessingObservation::Unavailable { reason:"Native progress publication is incomplete".into() }
        } else if previous.as_ref().is_some_and(|previous| completed_end < previous.progress.words[6]) {
            ProcessingObservation::Unavailable { reason:"Native progress moved backwards within one epoch".into() }
        } else if matches!(progress.words[17], 13 | 15) {
            ProcessingObservation::LastReportedStopped { epoch, observed_age_ms:age }
        } else if epoch != 0 && progress.words[17] == 11 && previous.as_ref()
            .is_some_and(|previous| completed_end > previous.progress.words[6]) {
            ProcessingObservation::Advancing { epoch, completed_end, observed_age_ms:age }
        } else {
            ProcessingObservation::NotObservedAdvancing { epoch, completed_end, observed_age_ms:age }
        };
        if !matches!(result, ProcessingObservation::Unavailable { .. }) {
            self.previous.insert(session.into(), Previous { key, progress, observed });
        }
        result
    }
}

fn observed<T>(result: Result<T>, reason: &str) -> Observation<T> {
    match result { Ok(value) => Observation::Observed(value), Err(_) => Observation::Unavailable(reason.into()) }
}

fn capture(manager: &Manager) -> Result<Vec<capacity::ActiveRecord>> {
    require(!operator_lock::registry_writer_waiting(manager), "instance_registry_busy")?;
    let _lock = match manager.try_lock_shared("registry.lock")? {
        operator_lock::LockAttempt::Acquired(lock) => lock,
        operator_lock::LockAttempt::Busy => return Err("instance_registry_busy".into()),
    };
    capacity::active_records_for(manager, None)
}

fn same_records(left: &[capacity::ActiveRecord], right: &[capacity::ActiveRecord]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(left, right)|
        left.owner == right.owner && left.document == right.document
            && left.path == right.path && left.report == right.report)
}

fn product(record: &capacity::ActiveRecord) -> Result<ProductIdentity> {
    let environment: crate::Environment = serde_json::from_value(record.document["registration"]["environment"].clone())?;
    let module: Artifact = serde_json::from_value(record.document["registration"]["module"].clone())?;
    require(!environment.id.is_empty() && valid_hex(&module.sha256, 64)
        && valid_hex(&record.owner.class_id, 32), "instance_product_binding")?;
    Ok(ProductIdentity { environment:environment.id, module_sha256:module.sha256, class_id:record.owner.class_id.clone() })
}

fn cleanup(report: &Path, session: &str) -> CleanupState {
    // The small authoritative retirement receipt is sufficient. Never read
    // private diagnostic bodies or treat absent/torn reports as clean.
    let receipt = report.with_extension("ownership.json");
    match fs::symlink_metadata(&receipt) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return CleanupState::NotReported,
        Err(_) => return CleanupState::Unavailable("The exact retirement receipt could not be read".into()),
        Ok(_) => (),
    }
    match read_json::<serde_json::Value>(&receipt).and_then(|value| {
        require(value["session"].as_str() == Some(session), "instance_cleanup_binding")?;
        let clean = value["cleanup_confirmed"].as_bool().ok_or("instance_cleanup_record")?;
        let retired = value["transport_retired"].as_bool().ok_or("instance_cleanup_record")?;
        Ok(clean && retired)
    }) {
        Ok(true) => CleanupState::Confirmed,
        Ok(false) => CleanupState::Unconfirmed,
        Err(_) => CleanupState::Unavailable("The exact retirement receipt could not be read".into()),
    }
}

struct Snapshot {
    identity: transport_storage::MemoryTransport,
    mapping: (u64, u64),
    progress: Option<Progress>,
    terminal: Observation<Option<InstanceFailure>>,
}

fn read_transport(record: &capacity::ActiveRecord) -> Result<Snapshot> {
    let identity: transport_storage::MemoryTransport = serde_json::from_value(record.document["transport"].clone())?;
    let source = transport_storage::session_directory(&record.owner.session, &identity)?;
    let durable = record.path.parent().ok_or("instance_owner_path")?;
    let snapshot = read_mapping(durable, &source, &record.owner.session, identity.clone())?;
    require(transport_storage::session_directory(&record.owner.session, &identity)? == source,
        "instance_transport_changed")?;
    Ok(snapshot)
}

fn read_mapping(durable: &Path, source: &Path, session: &str,
    identity: transport_storage::MemoryTransport) -> Result<Snapshot> {
    read_mapping_with(durable, source, session, identity, || {})
}

fn read_mapping_with(durable: &Path, source: &Path, session: &str,
    identity: transport_storage::MemoryTransport, after_copy: impl FnMut()) -> Result<Snapshot> {
    let file = capacity::terminal_file_from(durable, source, session)?.ok_or("instance_mapping_absent")?;
    let first = file.metadata()?;
    // A torn progress publication does not erase an independently committed
    // terminal failure. Each section retains its own availability.
    let progress = read_progress(&file, session, after_copy).ok().flatten();
    let terminal = observed(read_terminal(&file, session), "The terminal record could not be read");
    // Reopen the exact view after copying; no successful stale mapping read
    // survives replacement of its source, symlink, header or extent.
    let after = capacity::terminal_file_from(durable, source, session)?.ok_or("instance_mapping_absent")?.metadata()?;
    require((first.dev(), first.ino()) == (after.dev(), after.ino()), "instance_mapping_replaced")?;
    Ok(Snapshot { identity, mapping:(first.dev(), first.ino()), progress, terminal })
}

fn words(file: &File, offset: u64) -> Result<[u64; 24]> {
    let mut raw = [0u8; 192];
    file.read_exact_at(&mut raw, offset)?;
    Ok(std::array::from_fn(|index| u64::from_le_bytes(raw[index * 8..index * 8 + 8].try_into().unwrap())))
}

fn counter(file: &File, offset: u64) -> Result<u64> {
    let mut bytes = [0u8; 8];
    file.read_exact_at(&mut bytes, offset)?;
    Ok(u64::from_le_bytes(bytes))
}

fn bound(words: &[u64; 24], session: &str) -> bool {
    words[0] == 1 && words[3] != 0 && valid_hex(session, 32)
        && hex(&[words[1].to_le_bytes(), words[2].to_le_bytes()].concat()) == session
        && words[20..].iter().all(|value| *value == 0)
}

fn read_progress(file: &File, session: &str, mut after_copy: impl FnMut()) -> Result<Option<Progress>> {
    for _ in 0..3 {
        let before = counter(file, 1024)?;
        if before == 0 { return Ok(None); }
        let row = words(file, 1088 + (before & 1) * 256)?;
        after_copy();
        if counter(file, 1024)? != before { continue; }
        require(bound(&row, session) && row[14] == 0 && row[15] == 0 && row[18] == 0
            && row[19] == 0, "instance_progress_binding")?;
        return Ok(Some(Progress { words:row }));
    }
    Err("instance_progress_unstable".into())
}

fn read_terminal(file: &File, session: &str) -> Result<Option<InstanceFailure>> {
    let producer = counter(file, 64)?;
    if producer == 0 { return Ok(None); }
    require((1..=3).contains(&producer), "instance_terminal_producer")?;
    let row = words(file, 128 + (producer - 1) * 256)?;
    require(counter(file, 64)? == producer && bound(&row, session) && row[18] == producer
        && (1..=5).contains(&row[19]), "instance_terminal_binding")?;
    Ok(Some(match row[14] {
        1 => InstanceFailure::WindowsHostExited,
        2 => InstanceFailure::EditorControllerFailed,
        3 => InstanceFailure::TransportFailed,
        _ => return Err("instance_terminal_class".into()),
    }))
}

fn lifecycle(phase: u64) -> Observation<ReportedLifecycle> {
    match phase {
        0 | 8 => Observation::Observed(ReportedLifecycle::Initializing),
        9 => Observation::Observed(ReportedLifecycle::Activated),
        11 => Observation::Observed(ReportedLifecycle::Started),
        13 => Observation::Observed(ReportedLifecycle::Stopped),
        15 => Observation::Observed(ReportedLifecycle::Deactivated),
        17 => Observation::Observed(ReportedLifecycle::Created),
        _ => Observation::Unavailable("Native lifecycle phase is unavailable".into()),
    }
}

//! Live proxy control, independent of disposable Windows host leases.
//! Retained ownership authenticates a live connection; it never creates one.
use super::*;
use crate::operator_model as ui;
use ap1_native_client::recovery as wire;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, OnceLock,
};

mod custody;
use custody::*;
pub(super) use custody::retain_fresh_instance;

pub(super) const GREETING: &[u8; 5] = wire::GREETING;
struct Runtime {
    manager: Arc<Manager>,
    entries: Mutex<BTreeMap<String, Arc<Entry>>>,
    changed: Arc<dyn Fn() + Send + Sync>,
    revision: AtomicU64,
}
static RUNTIME: OnceLock<Runtime> = OnceLock::new();
struct Entry {
    original: String,
    registration: Registration,
    consumer: ui::ConsumerIdentity,
    state: Mutex<State>,
}
pub(super) fn initialize(
    manager: Arc<Manager>,
    changed: Arc<dyn Fn() + Send + Sync>,
) -> Result<()> {
    RUNTIME
        .set(Runtime {
            manager,
            entries: Mutex::new(BTreeMap::new()),
            changed,
            revision: AtomicU64::new(1),
        })
        .map_err(|_| "proxy_recovery_owner_already_initialized".into())
}
pub(super) fn revision(_m: &Manager) -> u64 {
    RUNTIME
        .get()
        .map_or(0, |r| r.revision.load(Ordering::Acquire))
}
fn changed() {
    if let Some(runtime) = RUNTIME.get() {
        let mut revision = runtime.revision.load(Ordering::Acquire);
        while let Some(next) = revision.checked_add(1) {
            match runtime.revision.compare_exchange_weak(revision, next, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => break,
                Err(current) => revision = current,
            }
        }
        (runtime.changed)();
    }
}
fn runtime() -> Result<&'static Runtime> {
    RUNTIME
        .get()
        .ok_or_else(|| "proxy_recovery_owner_unavailable".into())
}
fn forgettable(m: &Manager, entry: &Entry) -> bool {
    let Ok(state) = entry.state.lock() else {
        return false;
    };
    can_forget(m, &state)
}

fn retained_current(
    m: &Manager,
    original: &str,
    status: &wire::Status,
    peer: &transport_storage::PeerProcess,
) -> Result<(Registration, ui::ConsumerIdentity)> {
    let session = hex(&status.session);
    let current = retained(m, &session, peer)?;
    if session == original {
        require(status.generation == 1, "proxy_original_generation")?;
        return Ok(current);
    }
    let owner = m.retained_lease_owner(&session, &report(m, &session))?;
    let origin = &owner["restart_origin"];
    let operation = origin["operation"]
        .as_str()
        .ok_or("proxy_replacement_origin")?;
    require(
        origin["instance"] == original && valid_hex(operation, 32),
        "proxy_replacement_origin",
    )?;
    let request: serde_json::Value =
        read_json(&m.root.join("operator").join(operation).join("request.json"))?;
    require(
        request["action"]["kind"] == "plugin_restart"
            && request["action"]["instance"] == original
            && request["action"]["generation"]
                .as_u64()
                .and_then(|g| g.checked_add(1))
                == Some(status.generation),
        "proxy_replacement_generation",
    )?;
    Ok(current)
}
fn retained(
    m: &Manager,
    session: &str,
    peer: &transport_storage::PeerProcess,
) -> Result<(Registration, ui::ConsumerIdentity)> {
    require(valid_hex(session, 32), "proxy_original_session")?;
    let _lock = match m.try_lock_shared("registry.lock")? {
        operator_lock::LockAttempt::Acquired(lock) => lock,
        operator_lock::LockAttempt::Busy => return Err("proxy_registry_busy".into()),
    };
    let report = report(m, session);
    let owner = m.retained_lease_owner(session, &report)?;
    require(
        owner["inspect"] == false
            && owner["keeper"] == false
            && owner["vendor_access"] == false
            && owner["detached"] != true,
        "proxy_original_native_instance",
    )?;
    let consumer = lease_consumer(m, session, &report, &owner)?;
    require(
        consumer.pid == peer.pid
            && consumer.uid == peer.uid
            && consumer.start_ticks == peer.start_ticks
            && transport_storage::consumer_current(&consumer)?,
        "proxy_original_consumer",
    )?;
    // SessionSpec retains only the Windows host binding, not a Registration.
    // Recover the native side from the current registration only when both the
    // complete retained host binding and the admitted native hashes agree.
    let binding: HostBinding = serde_json::from_value(owner["registration"].clone())?;
    let execution: NativeExecution = serde_json::from_value(owner["native_execution"].clone())?;
    let registry = m.registry()?;
    let current = registry
        .classes
        .get(&binding.metadata.class_id.to_uppercase())
        .ok_or("proxy_registration_absent")?;
    require(
        current.publication == Publication::Published
            && HostBinding::from(current.registration.clone()) == binding,
        "proxy_registration_changed",
    )?;
    current.registration.verify_loaded_execution(&execution.identity())?;
    Ok((current.registration.clone(), consumer))
}
/// The normal service classifier releases its worker slot immediately; at most
/// 64 bounded control associations exist, without holding registry custody.
pub(super) fn attach(mut peer: UnixStream) -> Result<()> {
    let runtime = runtime()?;
    let process = transport_storage::peer_process(&peer)?;
    let mut frame = wire::SocketDeadline::new(&mut peer, Duration::from_millis(250));
    let mut original = [0; 16];
    frame.read_exact(&mut original)?;
    let original = hex(&original);
    let initial = wire::Status::read(&mut frame)?;
    let (registration, consumer) = retained(&runtime.manager, &original, &process)?;
    let (current, current_consumer) =
        retained_current(&runtime.manager, &original, &initial, &process)?;
    require(
        current == registration && current_consumer == consumer,
        "proxy_current_binding",
    )?;
    let entry = {
        let mut entries = runtime
            .entries
            .lock()
            .map_err(|_| "proxy_registry_poisoned")?;
        entries.retain(|key, entry| key == &original || !forgettable(&runtime.manager, entry));
        if let Some(entry) = entries.get(&original) {
            require(
                entry.registration == registration && entry.consumer == consumer,
                "proxy_reconnect_binding",
            )?;
            let mut state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
            require(!state.connected, "proxy_already_connected")?;
            reconnect(&mut state, initial)?;
            changed();
            entry.clone()
        } else {
            require(entries.len() < 64, "proxy_connection_capacity")?;
            let uncertain = unresolved_journal(&runtime.manager, &original, &initial);
            let pending = uncertain.then(|| Pending {
                command: retained_restart(&runtime.manager, &original).ok().flatten().unwrap_or(wire::Command {
                    operation: initial.operation,
                    generation: initial.generation,
                    revision: initial.revision,
                    digest: initial.digest,
                }),
                sent: true,
                launch_claimed: true,
                uncertain: true,
                accepted: Instant::now(),
            });
            let entry = Arc::new(Entry {
                original: original.clone(),
                registration,
                consumer,
                state: Mutex::new(State {
                    status: initial,
                    seen: Instant::now(),
                    connected: true,
                    pending,
                }),
            });
            entries.insert(original, entry.clone());
            changed();
            entry
        }
    };
    peer.set_write_timeout(Some(Duration::from_millis(250)))?;
    if let Err(error) = wire::Command::default().write(&mut wire::SocketDeadline::new(
        &mut peer,
        Duration::from_millis(250),
    )) {
        if let Ok(mut state) = entry.state.lock() {
            disconnected(&mut state);
        }
        return Err(error.into());
    }
    let worker_entry = entry.clone();
    let spawned = std::thread::Builder::new()
        .name("bridge-proxy-control".into())
        .spawn(move || {
            let entry = worker_entry;
            let result = (|| -> Result<()> {
                loop {
                    // A heartbeat interval is not a synchronous command wait.
                    peer.set_read_timeout(Some(Duration::from_secs(2)))?;
                    let mut first = [0];
                    peer.read_exact(&mut first)?;
                    let mut bytes = [0; wire::FRAME_BYTES];
                    bytes[0] = first[0];
                    wire::SocketDeadline::new(&mut peer, Duration::from_millis(250))
                        .read_exact(&mut bytes[1..])?;
                    let status = wire::Status::read(&mut bytes.as_slice())?;
                    require(
                        transport_storage::peer_process(&peer)? == process
                            && transport_storage::consumer_current(&entry.consumer)?,
                        "proxy_consumer_changed",
                    )?;
                    let mut state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
                    require(
                        status.generation >= state.status.generation,
                        "proxy_generation_reversed",
                    )?;
                    if status.session != state.status.session {
                        let (registration, consumer) =
                            retained_current(&runtime.manager, &entry.original, &status, &process)?;
                        require(
                            registration == entry.registration
                                && consumer == entry.consumer
                                && state.pending.as_ref().is_some_and(|p| {
                                    p.launch_claimed && p.command.operation == status.operation
                                }),
                            "proxy_replacement_not_authorized",
                        )?;
                    }
                    if state.status != status {
                        state.status = status;
                        changed();
                    }
                    state.seen = Instant::now();
                    let command = outgoing(&runtime.manager, &mut state);
                    // Keep command selection and the bounded write under the same
                    // custody as execute: it cannot finish unconfirmed between them.
                    command.write(&mut wire::SocketDeadline::new(
                        &mut peer,
                        Duration::from_millis(250),
                    ))?;
                    drop(state);
                }
            })();
            if let Ok(mut state) = entry.state.lock() {
                disconnected(&mut state);
            }
            // No command, URI, state or account bytes enter logs.
            let _ = result;
        });
    if let Err(error) = spawned {
        if let Ok(mut state) = entry.state.lock() {
            disconnected(&mut state);
        }
        return Err(error.into());
    }
    Ok(())
}
fn entry(instance: &str) -> Result<Arc<Entry>> {
    runtime()?
        .entries
        .lock()
        .map_err(|_| "proxy_registry_poisoned")?
        .get(instance)
        .cloned()
        .ok_or_else(|| "The original DAW instance is not connected".into())
}
fn admissible(m: &Manager, entry: &Entry, state: &State, command: &wire::Command) -> Result<()> {
    require(
        state.connected
            && state.seen.elapsed() < Duration::from_secs(2)
            && transport_storage::consumer_current(&entry.consumer)?,
        "The original DAW instance is no longer connected",
    )?;
    require_restart_status(&state.status, command)?;
    require(
        state.pending.as_ref().is_none_or(|pending| {
            !pending.uncertain
                && state.status.operation == pending.command.operation
                && matches!(state.status.phase, wire::COMPLETE | wire::REFUSED)
        }),
        "This instance has an unfinished or unconfirmed restart request",
    )?;
    require(
        !unresolved_journal(m, &entry.original, &state.status),
        "The previous restart outcome is unconfirmed; it will not be replayed",
    )?;
    retired(m, &hex(&state.status.session))
}
pub(super) fn validate(m: &Manager, action: &ui::Action) -> Result<()> {
    let ui::Action::PluginRestart { instance, .. } = action else {
        return Err("proxy_restart_action".into());
    };
    let entry = entry(instance)?;
    let state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
    admissible(m, &entry, &state, &command(action, [1; 16])?)
}
pub(super) fn execute(
    m: &Manager,
    action: &ui::Action,
    operation: &str,
) -> Result<serde_json::Value> {
    let ui::Action::PluginRestart { instance, .. } = action else {
        return Err("proxy_restart_action".into());
    };
    let entry = entry(instance)?;
    let command = command(action, wire::decode_id(operation)?)?;
    {
        let mut state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
        admissible(m, &entry, &state, &command)?;
        retain_restart(m, instance, &command)?;
        state.pending = Some(Pending {
            command,
            sent: false,
            launch_claimed: false,
            uncertain: false,
            accepted: Instant::now(),
        });
        changed();
    }
    let deadline = Instant::now() + Duration::from_secs(wire::RECOVERY_SECONDS);
    loop {
        let mut state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
        let status = state.status;
        if status.operation == command.operation && !state.pending.as_ref().is_some_and(|p| p.uncertain) {
            if status.phase == wire::COMPLETE {
                require(
                    status.generation == command.generation + 1,
                    "proxy_restart_generation",
                )?;
                return Ok(
                    serde_json::json!({"instance":instance,"operation":operation,"generation":status.generation,
                    "session":hex(&status.session),"snapshot_revision":command.revision,
                    "snapshot_sha256":hex(&command.digest),"controller_confirmed":true,"restart":"completed"}),
                );
            }
            if status.phase == wire::REFUSED {
                return Err(wire::reason(status.reason).into());
            }
        }
        if status.operation == command.operation && status.phase == wire::UNCERTAIN
            || !state.connected
            || state.pending.as_ref().is_some_and(|p| p.uncertain)
            || Instant::now() >= deadline
            || state.pending.as_ref().is_some_and(|p| {
                p.accepted.elapsed() > Duration::from_secs(5)
                    && status.operation != p.command.operation
            })
        {
            if let Some(pending) = state.pending.as_mut() {
                pending.uncertain = true;
            }
            changed();
            return Ok(
                serde_json::json!({"instance":instance,"operation":operation,"restart":"unconfirmed",
                "detail":"The restart outcome is unconfirmed; the bridge will not replay it"}),
            );
        }
        drop(state);
        std::thread::sleep(Duration::from_millis(20));
    }
}
/// Called once for LVB6 before the ordinary native admission path. The caller
/// must compare this exact registration at each existing publication recheck.
pub(super) fn authorize_launch(
    m: &Manager,
    peer: &UnixStream,
    original: [u8; 16],
    operation: [u8; 16],
) -> Result<Registration> {
    let entry = entry(&hex(&original))?;
    let process = transport_storage::peer_process(peer)?;
    let mut state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
    require(
        state.connected
            && entry.consumer.pid == process.pid
            && entry.consumer.uid == process.uid
            && entry.consumer.start_ticks == process.start_ticks
            && transport_storage::consumer_current(&entry.consumer)?,
        "proxy_restart_peer",
    )?;
    let pending = state
        .pending
        .as_ref()
        .ok_or("proxy_restart_not_requested")?;
    require(
        pending.sent
            && !pending.launch_claimed
            && !pending.uncertain
            && pending.command.operation == operation
            && pending.command.matches(&state.status),
        "proxy_restart_not_authorized",
    )?;
    retired(m, &hex(&state.status.session))?;
    state.pending.as_mut().unwrap().launch_claimed = true;
    changed();
    Ok(entry.registration.clone())
}
fn recovery_phase(state: &State) -> ui::RecoveryPhase {
    use ui::RecoveryPhase as P;
    if !state.connected || state.seen.elapsed() >= Duration::from_secs(2) { return P::Unknown; }
    match state.status.phase {
        wire::COMPLETE if state.status.flags & wire::FAILED != 0 => P::Failed,
        wire::COMPLETE => P::Restored,
        wire::REFUSED => P::Refused,
        wire::UNCERTAIN => P::Uncertain,
        wire::ACCEPTED | wire::RESTORING | wire::CONTROLLER => P::Restarting,
        wire::IDLE if state.pending.is_some() => P::Restarting,
        wire::IDLE if state.status.flags & wire::FAILED != 0 => P::Failed,
        wire::IDLE => P::Connected,
        _ => P::Unknown,
    }
}

fn observed(entry: &Entry, state: &State, m: &Manager) -> ui::InstanceRecovery {
    let status = state.status;
    let snapshot = ui::RecoverySnapshot {
        revision: status.revision,
        sha256: hex(&status.digest),
    };
    let action = ui::Action::PluginRestart {
        instance: entry.original.clone(),
        generation: status.generation,
        snapshot: snapshot.clone(),
    };
    let refusal = admissible(
        m,
        entry,
        state,
        &command(&action, [1; 16]).unwrap_or_default(),
    )
    .err()
    .map(|e| e.to_string());
    let phase = recovery_phase(state);
    let detail = match phase {
        ui::RecoveryPhase::Restored => "The exact confirmed state and DAW controller were restored".into(),
        ui::RecoveryPhase::Refused | ui::RecoveryPhase::Uncertain => wire::reason(status.reason).into(),
        ui::RecoveryPhase::Restarting => "Restarting this exact plug-in instance; its track stays silent until it is ready".into(),
        ui::RecoveryPhase::Failed => "This plug-in instance failed. Restart restores the last confirmed state; later changes may be lost".into(),
        ui::RecoveryPhase::Connected => "This live DAW instance is connected".into(),
        ui::RecoveryPhase::Unknown => "The current recovery state could not be confirmed".into(),
    };
    ui::InstanceRecovery {
        phase,
        instance: entry.original.clone(),
        generation: status.generation,
        snapshot: if status.flags & wire::SNAPSHOT != 0 {
            ui::Observation::Observed(snapshot)
        } else {
            ui::Observation::Unavailable("No complete confirmed state has been captured".into())
        },
        uncaptured_changes_possible: status.flags & wire::LATER_EDITS != 0,
        detail,
        restart: (status.flags & wire::FAILED != 0).then(|| ui::AvailableAction {
            label: "Restart this plug-in".into(),
            action,
            disabled_reason: refusal,
        }),
    }
}
fn connected() -> Vec<Arc<Entry>> {
    RUNTIME
        .get()
        .and_then(|r| r.entries.lock().ok().map(|e| e.values().cloned().collect()))
        .unwrap_or_default()
}
pub(super) fn project(m: &Manager, products: &mut [ui::Product]) -> Result<()> {
    for entry in connected() {
        let state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
        if !state.connected || state.seen.elapsed() >= Duration::from_secs(2) {
            continue;
        }
        if let Some(offer) = observed(&entry, &state, m).restart {
            for product in products
                .iter_mut()
                .filter(|p| product_identity(&entry).matches(p))
            {
                product.actions.push(offer.clone());
            }
        }
    }
    Ok(())
}
fn product_identity(entry: &Entry) -> ui::ProductIdentity {
    ui::ProductIdentity {
        environment: entry.registration.environment.id.clone(),
        module_sha256: entry.registration.module.sha256.clone(),
        class_id: entry.registration.metadata.class_id.clone(),
    }
}
pub(super) fn augment_activity(m: &Manager, activity: &mut ui::LiveActivity) -> Result<()> {
    let ui::Observation::Observed(rows) = &mut activity.instances else {
        return Ok(());
    };
    for entry in connected() {
        let state = entry.state.lock().map_err(|_| "proxy_state_poisoned")?;
        if !state.connected || state.seen.elapsed() >= Duration::from_secs(2) {
            continue;
        }
        let recovery = observed(&entry, &state, m);
        let session = hex(&state.status.session);
        if let Some(row) = rows.iter_mut().find(|row| row.session == session) {
            row.recovery = Some(recovery);
            continue;
        }
        // Positive live connection, not a historical row promoted into liveness.
        let positively_retired = retired(m, &session).is_ok();
        rows.push(ui::LiveInstance {
            kind: ui::InstanceKind::Daw,
            session: session.clone(),
            product: ui::Observation::Observed(product_identity(&entry)),
            consumer: ui::Observation::Observed(ui::ConsumerObservation {
                identity: entry.consumer.clone(),
                current: ui::Observation::Observed(true),
            }),
            daw: ui::Observation::Unavailable("No current VST3 host-name observation".into()),
            transport: ui::Observation::Unavailable(
                if positively_retired {
                    "The previous Windows transport retired"
                } else {
                    "Current transport ownership is unavailable"
                }
                .into(),
            ),
            lifecycle: ui::Observation::Unavailable(
                "Current Windows host lifecycle is unavailable".into(),
            ),
            processing: ui::ProcessingObservation::Unavailable {
                reason: if positively_retired {
                    "The DAW proxy remains; its Windows host retired"
                } else {
                    "The DAW proxy is connected; current processing is unavailable"
                }
                .into(),
            },
            signal: ui::SignalObservation::NotMeasured,
            terminal: ui::Observation::Unavailable(
                "Read the exact retained failure for its cause".into(),
            ),
            cleanup: if positively_retired {
                ui::CleanupState::Confirmed
            } else {
                ui::CleanupState::Unconfirmed
            },
            recovery: Some(recovery),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    #[test]
    fn restart_status_names_missing_capability_without_claiming_snapshot_changed() {
        let ready = status();
        let command = pending(&ready).command;
        require_restart_status(&ready, &command).unwrap();
        for (flag, reason) in [
            (wire::FAILED, "has not reported a failure"),
            (wire::SNAPSHOT, "No complete confirmed state"),
            (wire::RETIRED, "have not confirmed retirement"),
            (wire::OWNER_TIMER, "Open its controls in your DAW"),
        ] {
            let missing = wire::Status {flags: ready.flags & !flag, ..ready};
            assert!(command.matches(&missing), "capability absence does not change the exact binding");
            let error = require_restart_status(&missing, &command).unwrap_err().to_string();
            assert!(error.contains(reason), "{error}");
            assert!(!error.contains("snapshot or instance changed"));
        }
        for phase in [wire::ACCEPTED, wire::RESTORING, wire::CONTROLLER, wire::UNCERTAIN] {
            let changing = wire::Status {phase, ..ready};
            assert!(require_restart_status(&changing, &command).unwrap_err().to_string()
                .contains("unfinished or unconfirmed restart"));
        }
        for changed in [
            wire::Command {generation:command.generation + 1, ..command},
            wire::Command {revision:command.revision + 1, ..command},
            wire::Command {digest:[9;32], ..command},
        ] {
            assert!(require_restart_status(&ready, &changed).unwrap_err().to_string()
                .contains("snapshot or instance changed"));
        }
        // Naming each missing capability must preserve the wire's admission set.
        for flags in 0..32 {
            for phase in 0..=wire::UNCERTAIN + 1 {
                let observed = wire::Status {flags, phase, ..ready};
                assert_eq!(require_restart_status(&observed, &command).is_ok(),
                    observed.ready() && command.matches(&observed));
            }
        }
    }
    #[test]
    fn later_failure_supersedes_completed_restart_in_phase_and_detail() {
        let f = Fixture::new();
        let entry = entry(&f);
        let mut state = entry.state.lock().unwrap();
        state.status.phase = wire::COMPLETE;
        state.pending = Some(pending(&state.status));
        state.status.flags &= !wire::FAILED;
        let completed = observed(&entry, &state, &f.m);
        assert_eq!(completed.phase, ui::RecoveryPhase::Restored);
        assert!(completed.detail.contains("were restored"));
        state.status.flags |= wire::FAILED;
        let failed = observed(&entry, &state, &f.m);
        assert_eq!(failed.phase, ui::RecoveryPhase::Failed);
        assert!(failed.detail.contains("instance failed"));
        assert!(!failed.detail.contains("were restored"));
        state.status.phase = wire::RESTORING;
        assert_eq!(observed(&entry, &state, &f.m).phase, ui::RecoveryPhase::Restarting);
    }

    #[test]
    fn recovery_phase_requires_a_current_proxy_observation() {
        use ui::RecoveryPhase as P;
        let mut state = State {status: status(), seen:Instant::now(), connected:true, pending:None};
        assert_eq!(recovery_phase(&state), P::Failed);
        for phase in [wire::ACCEPTED, wire::RESTORING, wire::CONTROLLER] {
            state.status.phase = phase;
            assert_eq!(recovery_phase(&state), P::Restarting);
        }
        state.status.phase = wire::COMPLETE;
        assert_eq!(recovery_phase(&state), P::Failed, "a later failure supersedes a completed restart");
        state.status.flags &= !wire::FAILED;
        assert_eq!(recovery_phase(&state), P::Restored);
        state.connected = false;
        assert_eq!(recovery_phase(&state), P::Unknown);
        state.connected = true;
        state.seen = Instant::now() - Duration::from_secs(3);
        assert_eq!(recovery_phase(&state), P::Unknown);
        state.seen = Instant::now();
        state.status.phase = wire::UNCERTAIN;
        assert_eq!(recovery_phase(&state), P::Uncertain);
        state.status.phase = wire::REFUSED;
        assert_eq!(recovery_phase(&state), P::Refused);
        state.status.phase = 999;
        assert_eq!(recovery_phase(&state), P::Unknown);
    }

    fn status() -> wire::Status {
        wire::Status {
            session: [1; 16],
            generation: 1,
            revision: 3,
            digest: [4; 32],
            flags: wire::FAILED | wire::RETIRED | wire::SNAPSHOT | wire::OWNER_TIMER,
            ..Default::default()
        }
    }
    fn peer() -> transport_storage::PeerProcess {
        let (socket, _) = UnixStream::pair().unwrap();
        transport_storage::peer_process(&socket).unwrap()
    }
    fn entry(f: &Fixture) -> Entry {
        let peer = peer();
        Entry {
            original: hex(&[1; 16]),
            registration: f.r.clone(),
            consumer: ui::ConsumerIdentity {
                boot_id: kernel_boot().unwrap().unwrap(),
                pid: peer.pid,
                uid: peer.uid,
                start_ticks: peer.start_ticks,
            },
            state: Mutex::new(State {
                status: status(),
                seen: Instant::now(),
                connected: true,
                pending: None,
            }),
        }
    }
    fn receipt(f: &Fixture, operation: [u8; 16], instance: &str, outcome: &str) {
        let path =
            f.m.root
                .join("operator")
                .join(hex(&operation))
                .join("result.json");
        private_dir(path.parent().unwrap()).unwrap();
        atomic_json(
            &path.with_file_name("request.json"),
            &serde_json::json!({"action":{"kind":"plugin_restart","instance":instance}}),
        )
        .unwrap();
        atomic_json(
            &path,
            &serde_json::json!({"schema":1,"operation":hex(&operation),"state":"completed",
            "outcome":outcome,
            "result":{"restart":outcome}}),
        )
        .unwrap();
    }
    fn pending(status: &wire::Status) -> Pending {
        Pending {
            command: wire::Command {
                operation: [2; 16],
                generation: status.generation,
                revision: status.revision,
                digest: status.digest,
            },
            sent: true,
            launch_claimed: true,
            uncertain: true,
            accepted: Instant::now(),
        }
    }
    #[test]
    fn reconnect_and_new_owner_keep_exact_unconfirmed_journal_without_replay() {
        let f = Fixture::new();
        let entry = entry(&f);
        let mut state = entry.state.lock().unwrap();
        state.pending = Some(pending(&state.status));
        state.status.operation = [2; 16];
        state.status.phase = wire::UNCERTAIN;
        let original = state.pending.as_ref().unwrap().command;
        state.connected = false;
        let returning = state.status;
        reconnect(&mut state, returning).unwrap();
        assert_eq!(state.pending.as_ref().unwrap().command, original);
        assert!(state.pending.as_ref().unwrap().uncertain);
        receipt(&f, [2; 16], &entry.original, "unconfirmed");
        assert!(
            unresolved_journal(&f.m, &entry.original, &returning),
            "new service cannot promote terminal executor into successful restart"
        );
        receipt(&f, [2; 16], "another-instance", "completed");
        assert!(
            unresolved_journal(&f.m, &entry.original, &returning),
            "another instance's receipt cannot settle this request"
        );
        receipt(&f, [2; 16], &entry.original, "completed");
        assert!(unresolved_journal(&f.m, &entry.original, &returning),
            "a settled proxy-selected receipt cannot repair missing legacy custody");
        assert!(
            state.pending.as_ref().unwrap().uncertain,
            "a reconnect cannot erase the current owner's uncertain delivery"
        );
    }
    #[test]
    fn terminal_uncertain_history_does_not_exhaust_live_control_capacity() {
        let f = Fixture::new();
        let mut entries = BTreeMap::new();
        for i in 1..=80u8 {
            let entry = entry(&f);
            let operation = [i; 16];
            {
                let mut state = entry.state.lock().unwrap();
                state.connected = false;
                state.pending = Some(pending(&state.status));
                state.pending.as_mut().unwrap().command.operation = operation;
            }
            assert!(
                !forgettable(&f.m, &entry),
                "in-flight custody retained before terminal receipt"
            );
            receipt(&f, operation, &entry.original, "unconfirmed");
            entries.insert(i, Arc::new(entry));
            entries.retain(|_, entry| !forgettable(&f.m, entry));
            assert!(entries.is_empty());
            let status = wire::Status {
                operation,
                phase: wire::UNCERTAIN,
                ..status()
            };
            assert!(
                unresolved_journal(&f.m, &hex(&[1; 16]), &status),
                "eviction leaves replay fence intact"
            );
        }
    }
    #[test]
    fn exact_snapshot_and_positive_retirement_are_required_beyond_proxy_failure() {
        let f = Fixture::new();
        let entry = entry(&f);
        let state = entry.state.lock().unwrap();
        retain_fresh_instance(&f.m, &entry.original).unwrap();
        let command = wire::Command {
            operation: [2; 16],
            generation: 1,
            revision: 3,
            digest: [4; 32],
        };
        assert!(
            admissible(&f.m, &entry, &state, &command).is_err(),
            "claimed retired flag cannot replace owner receipt"
        );
        let display = observed(&entry, &state, &f.m);
        assert!(display.detail.contains("instance failed"));
        assert!(!display.detail.contains("host stopped"));
        let path = report(&f.m, &entry.original).with_extension("ownership.json");
        private_dir(path.parent().unwrap()).unwrap();
        atomic_json(&path,&serde_json::json!({"session":entry.original,"cleanup_confirmed":true,"transport_retired":true})).unwrap();
        admissible(&f.m, &entry, &state, &command).unwrap();
        let before = fs::read(&path).unwrap();
        assert!(admissible(
            &f.m,
            &entry,
            &state,
            &wire::Command {
                digest: [5; 32],
                ..command
            }
        )
        .is_err());
        assert!(admissible(
            &f.m,
            &entry,
            &state,
            &wire::Command {
                generation: 2,
                ..command
            }
        )
        .is_err());
        assert_eq!(fs::read(path).unwrap(), before);
    }
    fn registered_fixture() -> Fixture {
        let mut f = Fixture::new();
        let descriptor = lvb_plugin_descriptor::Descriptor {
            schema: 1,
            engine_sha256: f.r.native.sha256.clone(),
            class_id: f.r.metadata.class_id.clone(),
            module_sha256: f.r.module.sha256.clone(),
            class_name: f.r.metadata.name.clone(),
            vendor: f.r.metadata.vendor.clone(),
            version: f.r.metadata.version.clone(),
            subcategories: f.r.metadata.subcategories.clone(),
            buses: vec![lvb_plugin_descriptor::Bus {
                media: 0, direction: 1, index: 0, channels: 2,
                r#type: 0, flags: 1, arrangement: 3, name: "Output".into(),
            }],
            parameters: vec![],
        };
        descriptor.validate().unwrap();
        let path = f.r.native.path.with_file_name(lvb_plugin_descriptor::FILE_NAME);
        atomic_json(&path, &descriptor).unwrap();
        f.r.descriptor = Some(Artifact { sha256: digest(&path).unwrap(), path });
        f.m.register(f.r.clone()).unwrap();
        f.r = f.m.registry().unwrap().classes[&f.r.key()].registration.clone();
        f
    }
    fn retain(f: &Fixture, origin: Option<(&str, &str)>) -> [u8; 16] {
        let _lock = f.m.lock("registry.lock").unwrap();
        let peer = peer();
        let (mut job, path) = spec(&f.m, f.r.clone().into(), false, false, false).unwrap();
        job.graphical_session = Some(transport_storage::GraphicalSession {
            schema: 1, peer_pid: peer.pid, peer_start_ticks: peer.start_ticks,
            display: ":0".into(), wayland_display: None, xauthority: None,
            dbus_session_bus_address: None,
        });
        let hash = |s: &str| std::array::from_fn(|i| u8::from_str_radix(&s[i*2..i*2+2], 16).unwrap());
        job.native_execution = Some(NativeExecution {
            engine: hash(&f.r.native.sha256),
            descriptor: hash(&f.r.descriptor.as_ref().unwrap().sha256),
        });
        job.restart_origin = origin.map(|(instance, operation)| NativeRestartOrigin {
            instance: instance.into(), operation: operation.into(),
        });
        atomic_json(&path, &job).unwrap();
        f.m.retain_lease_owner(&path).unwrap();
        record_lease_generation(&job).unwrap();
        wire::decode_id(&job.session).unwrap()
    }
    #[test]
    fn real_session_spec_recovery_joins_only_its_current_host_and_native_binding() {
        let f = registered_fixture();
        let session = hex(&retain(&f, None));
        let peer = peer();
        let owner = f.m.retained_lease_owner(&session, &report(&f.m, &session)).unwrap();
        assert!(owner["registration"]["metadata"].get("name").is_none());
        assert!(owner["registration"].get("native").is_none());
        assert!(serde_json::from_value::<Registration>(owner["registration"].clone()).is_err());
        let retained_path = f.m.root.join("runtime/lease-owners").join(format!("{session}.json"));
        let before = fs::read(&retained_path).unwrap();
        assert_eq!(retained(&f.m, &session, &peer).unwrap().0, f.r);
        let registry = f.m.registry().unwrap();
        for change in ["module", "setup", "host", "settings", "native", "descriptor", "removed", "absent"] {
            let mut changed = registry.clone();
            let current = changed.classes.get_mut(&f.r.key()).unwrap();
            let expected = match change {
                "native" => { current.registration.native.sha256 = "ee".repeat(32); "loaded_execution_binding_differs" }
                "descriptor" => { current.registration.descriptor.as_mut().unwrap().sha256 = "ee".repeat(32); "loaded_execution_binding_differs" }
                "absent" => { changed.classes.clear(); "proxy_registration_absent" }
                _ => {
                    match change {
                        "module" => current.registration.module.sha256 = "ee".repeat(32),
                        "setup" => current.registration.environment.revision += 1,
                        "host" => current.registration.host.sha256 = "ee".repeat(32),
                        "settings" => current.registration.compatibility.disable_windows_accessibility = !current.registration.compatibility.disable_windows_accessibility,
                        "removed" => current.publication = Publication::Removed,
                        _ => unreachable!(),
                    }
                    "proxy_registration_changed"
                }
            };
            atomic_json(&f.m.root.join("registry.json"), &changed).unwrap();
            assert_eq!(retained(&f.m, &session, &peer).unwrap_err().to_string(), expected, "{change}");
            assert_eq!(fs::read(&retained_path).unwrap(), before);
        }
        atomic_json(&f.m.root.join("registry.json"), &registry).unwrap();
        let foreign = transport_storage::PeerProcess { start_ticks: peer.start_ticks + 1, ..peer };
        assert_eq!(retained(&f.m, &session, &foreign).unwrap_err().to_string(), "proxy_original_consumer");
        assert_eq!(retained(&f.m, &session, &peer).unwrap().0, f.r);
        assert_eq!(fs::read(retained_path).unwrap(), before);
    }
    #[test]
    fn recovery_registration_retries_after_registry_custody_without_changing_identity() {
        let f = registered_fixture();
        let session = hex(&retain(&f, None));
        let peer = peer();
        let lock = f.m.lock("registry.lock").unwrap();
        assert_eq!(retained(&f.m, &session, &peer).unwrap_err().to_string(), "proxy_registry_busy");
        drop(lock);
        assert_eq!(retained(&f.m, &session, &peer).unwrap().0, f.r);
    }
    #[test]
    fn replacement_connection_requires_owner_bound_origin_not_same_class_or_daw() {
        let f = registered_fixture();
        let initial = retain(&f, None);
        let original = hex(&initial);
        let operation = hex(&[2; 16]);
        let peer = peer();
        let other = retain(&f, None);
        retained_current(&f.m, &original, &wire::Status { session: initial, ..status() }, &peer).unwrap();
        let replacement = wire::Status {
            session: other,
            generation: 2,
            operation: [2; 16],
            phase: wire::COMPLETE,
            ..status()
        };
        assert!(
            retained_current(&f.m, &original, &replacement, &peer).is_err(),
            "same binding and process do not prove replacement"
        );
        let renewed = retain(&f, Some((&original, &operation)));
        let replacement = wire::Status {
            session: renewed,
            ..replacement
        };
        let path =
            f.m.root
                .join("operator")
                .join(&operation)
                .join("request.json");
        private_dir(path.parent().unwrap()).unwrap();
        atomic_json(&path,&serde_json::json!({"action":{"kind":"plugin_restart","instance":original,"generation":1}})).unwrap();
        retained_current(&f.m, &original, &replacement, &peer).unwrap();
        assert!(retained_current(
            &f.m,
            &original,
            &wire::Status {
                generation: 3,
                ..replacement
            },
            &peer
        )
        .is_err());
        let mut foreign = peer;
        foreign.start_ticks += 1;
        assert!(retained_current(&f.m, &original, &replacement, &foreign).is_err());
    }
}

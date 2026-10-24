//! Durable restart identity and one-shot command custody.
use super::*;

pub(super) struct State {
    pub(super) status: wire::Status,
    pub(super) seen: Instant,
    pub(super) connected: bool,
    // Delivery owned by this service executor, never reconstructed from disk.
    pub(super) pending: Option<Pending>,
}
pub(super) struct Pending {
    pub(super) command: wire::Command,
    pub(super) sent: bool,
    pub(super) launch_claimed: bool,
    pub(super) uncertain: bool,
    pub(super) accepted: Instant,
}

// One direct index per logical instance, retained before a command can escape.
// The proxy's latest operation is an observation, never the index of authority.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RestartBinding {
    schema: u32,
    instance: String,
    operation: String,
    generation: u64,
    revision: u64,
    digest: String,
}
pub(super) fn binding_path(m: &Manager, instance: &str) -> Result<PathBuf> {
    require(valid_hex(instance, 32), "proxy_restart_instance")?;
    Ok(m.root
        .join("runtime/instance-recovery")
        .join(format!("{instance}.json")))
}
/// Called only for a newly allocated original native session, before publishing
/// its lease. Never call on attach/reconnect or for a replacement session.
/// Create-only: even a partial or malformed existing record retains custody.
pub(in crate::owner) fn retain_fresh_instance(m: &Manager, instance: &str) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let path = binding_path(m, instance)?;
    private_dir(path.parent().ok_or("proxy_restart_parent")?)?;
    let record = RestartBinding {
        schema: 1,
        instance: instance.into(),
        operation: hex(&[0; 16]),
        generation: 0,
        revision: 0,
        digest: hex(&[0; 32]),
    };
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    serde_json::to_writer(&mut file, &record)?;
    file.sync_all()?;
    File::open(path.parent().ok_or("proxy_restart_parent")?)?.sync_all()?;
    File::open(m.root.join("runtime"))?.sync_all()?;
    File::open(&m.root)?.sync_all()?;
    Ok(())
}
pub(super) fn retain_restart(m: &Manager, instance: &str, command: &wire::Command) -> Result<()> {
    command.validate()?;
    require(command.operation != [0; 16], "proxy_restart_operation")?;
    let path = binding_path(m, instance)?;
    private_dir(path.parent().ok_or("proxy_restart_parent")?)?;
    atomic_json(
        &path,
        &RestartBinding {
            schema: 1,
            instance: instance.into(),
            operation: hex(&command.operation),
            generation: command.generation,
            revision: command.revision,
            digest: hex(&command.digest),
        },
    )?;
    // atomic_json syncs the record directory; retain its newly created ancestry too.
    File::open(m.root.join("runtime"))?.sync_all()?;
    File::open(&m.root)?.sync_all()?;
    Ok(())
}
pub(super) fn retained_restart(m: &Manager, instance: &str) -> Result<Option<wire::Command>> {
    let path = binding_path(m, instance)?;
    let binding: RestartBinding = read_json(&path)?;
    require(
        binding.schema == 1 && binding.instance == instance,
        "proxy_restart_binding",
    )?;
    // Only a durable, positively initialized empty binding means no command.
    // Missing legacy custody is an error, even if the proxy reports operation zero.
    if binding.operation == hex(&[0; 16]) {
        require(
            binding.generation == 0 && binding.revision == 0 && binding.digest == hex(&[0; 32]),
            "proxy_restart_empty_binding",
        )?;
        return Ok(None);
    }
    let action = ui::Action::PluginRestart {
        instance: instance.into(),
        generation: binding.generation,
        snapshot: ui::RecoverySnapshot {
            revision: binding.revision,
            sha256: binding.digest,
        },
    };
    Ok(Some(command(
        &action,
        wire::decode_id(&binding.operation)?,
    )?))
}
pub(super) fn unresolved_operation(
    m: &Manager,
    instance: &str,
    operation: [u8; 16],
    exact: Option<&wire::Command>,
) -> bool {
    if operation == [0; 16] {
        return false;
    }
    let directory = m.root.join("operator").join(hex(&operation));
    let Ok(request) = read_json::<serde_json::Value>(&directory.join("request.json")) else {
        return true;
    };
    let Ok(action) = serde_json::from_value::<ui::Action>(request["action"].clone()) else {
        return true;
    };
    let Ok(requested) = command(&action, operation) else {
        return true;
    };
    if request["action"]["instance"] != instance
        || exact.is_some_and(|expected| *expected != requested)
    {
        return true;
    }
    let Ok(record) = read_json::<serde_json::Value>(&directory.join("result.json")) else {
        return true;
    };
    if record["schema"] != 1 || record["operation"] != hex(&operation) {
        return true;
    }
    // Positive shapes emitted by operator_cli and execute. A terminal executor
    // alone does not establish its mutation outcome. Refusals have no outcome
    // or result; interrupted running restarts are completed/unconfirmed.
    let settled = match record["state"].as_str() {
        Some("refused") => {
            record["reason"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty())
                && record.get("outcome").is_none()
                && record.get("result").is_none()
        }
        Some("completed") => {
            let result = &record["result"];
            record["outcome"] == "confirmed"
                && result["restart"] == "completed"
                && result["instance"] == instance
                && result["operation"] == hex(&operation)
                && requested
                    .generation
                    .checked_add(1)
                    .is_some_and(|g| result["generation"] == g)
                && result["snapshot_revision"] == requested.revision
                && result["snapshot_sha256"] == hex(&requested.digest)
                && result["controller_confirmed"] == true
                && result["session"]
                    .as_str()
                    .is_some_and(|s| valid_hex(s, 32) && s != hex(&[0; 16]))
        }
        _ => false,
    };
    !settled
}
#[cfg(test)]
pub(super) fn require_settled_custody(
    m: &Manager,
    instance: &str,
    status: &wire::Status,
) -> Result<()> {
    require_settled_custody_for(m, instance, status, None)
}
fn require_settled_custody_for(
    m: &Manager,
    instance: &str,
    status: &wire::Status,
    executing: Option<&wire::Command>,
) -> Result<()> {
    let retained = match retained_restart(m, instance) {
        Ok(retained) => retained,
        Err(_) => {
            // Only an absent index is a legacy input. Never erase or replace a
            // malformed record, including a dangling symlink, through migration.
            require(matches!(fs::symlink_metadata(binding_path(m, instance)?),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound),
                "Restart is unavailable because this instance's durable restart custody is missing or unreadable. Keep the project open and choose Send a problem report in the bridge.")?;
            return require_legacy_history(m, instance, status, executing);
        }
    };
    require(
        !retained.is_some_and(|command| unresolved_operation(m, instance, command.operation, Some(&command)))
            && !unresolved_operation(m, instance, status.operation, None),
        "The previous restart outcome is unconfirmed; it will not be replayed. Keep the project open and choose Send a problem report in the bridge to report this problem.",
    )
}
/// Caller holds the authenticated original instance's state mutex. Journals
/// predate the direct index and are retained, not pruned. Read the entire bounded
/// inventory, never just the proxy's last reported operation. New requests can
/// be recorded concurrently, but cannot deliver until they take this same mutex
/// and retain their nonempty binding. No migration/empty sentinel is written.
fn require_legacy_history(
    m: &Manager,
    instance: &str,
    status: &wire::Status,
    executing: Option<&wire::Command>,
) -> Result<()> {
    status.validate()?;
    require_restart_status(status, &wire::Command {
        operation: [1; 16], generation: status.generation,
        revision: status.revision, digest: status.digest,
    })?;
    // A proxy flag or absent lease alone cannot authorize a replacement.
    retired(m, &hex(&status.session))?;
    let directory = m.root.join("operator");
    require(fs::symlink_metadata(&directory).is_ok_and(|meta| meta.is_dir()),
        "The retained restart journal is unavailable. Keep the project open so its latest live snapshot is preserved, and choose Send a problem report in the bridge.")?;
    for (index, entry) in fs::read_dir(directory)?.enumerate() {
        require(index < 2048, "The retained restart journal exceeds the readable inventory limit")?;
        let entry = entry?;
        let name = entry.file_name();
        let Some(id) = name.to_str().filter(|id| valid_hex(id, 32)) else { continue };
        require(entry.file_type()?.is_dir(), "A retained operation directory is unreadable")?;
        let request: serde_json::Value = read_json(&entry.path().join("request.json"))?;
        let kind = request["action"]["kind"].as_str().ok_or("A retained operation's action is unreadable")?;
        if kind != "plugin_restart" { continue; }
        let original = request["action"]["instance"].as_str()
            .filter(|id| valid_hex(id, 32)).ok_or("A retained restart's instance identity is unreadable")?;
        if original != instance { continue; }
        let action: ui::Action = serde_json::from_value(request["action"].clone())?;
        let requested = command(&action, wire::decode_id(id)?)?;
        let record: serde_json::Value = read_json(&entry.path().join("result.json"))?;
        // These are positive pre-delivery stages, not an unconfirmed mutation.
        // An interrupted running executor remains completed/unconfirmed and
        // cannot enter here. Only this exact new execute call may name running.
        let undelivered = record["schema"] == 1 && record["operation"] == id
            && record["action"] == request["action"]
            && record.get("outcome").is_none() && record.get("result").is_none()
            && match record["state"].as_str() {
                Some("unconfirmed") => record["stage"] == "operator_request_admission"
                    && record["worker_started"] == false,
                Some("queued") => true,
                Some("waiting") => record["stage"] == "operator_validation_readback",
                Some("running") => executing == Some(&requested),
                _ => false,
            };
        require(undelivered || !unresolved_operation(m, instance, requested.operation, Some(&requested)),
            "A previous restart outcome is unconfirmed; it will not be replayed. Keep the project open so its latest live snapshot is preserved, and choose Send a problem report in the bridge.")?;
    }
    require(!unresolved_operation(m, instance, status.operation, None),
        "The proxy's previous restart outcome is unconfirmed; it will not be replayed")
}

pub(super) fn require_restart(
    m: &Manager, instance: &str, state: &State, command: &wire::Command,
    executing: Option<&wire::Command>,
) -> Result<()> {
    require_restart_status(&state.status, command)?;
    require(state.pending.as_ref().is_none_or(|pending| {
        !pending.uncertain && state.status.operation == pending.command.operation
            && matches!(state.status.phase, wire::COMPLETE | wire::REFUSED)
    }), "This instance has an unfinished or unconfirmed restart request")?;
    require_settled_custody_for(m, instance, &state.status, executing)?;
    retired(m, &hex(&state.status.session))
}
/// The owner has checked the live consumer and holds its state mutex. Retain
/// the new request before exposing delivery; a crash keeps this exact fence.
pub(super) fn begin_restart(
    m: &Manager, instance: &str, state: &mut State, command: wire::Command,
) -> Result<()> {
    require_restart(m, instance, state, &command, Some(&command))?;
    retain_restart(m, instance, &command)?;
    state.pending = Some(Pending {
        command, sent: false, launch_claimed: false, uncertain: false,
        accepted: Instant::now(),
    });
    changed();
    Ok(())
}
#[cfg(test)]
pub(super) fn unresolved_journal(m: &Manager, instance: &str, status: &wire::Status) -> bool {
    require_settled_custody(m, instance, status).is_err()
}
/// Attach observes a proxy; only execute may install a live delivery request.
pub(super) fn observation(status: wire::Status) -> State {
    State {
        status,
        seen: Instant::now(),
        connected: true,
        pending: None,
    }
}
pub(super) fn recovery_phase(state: &State) -> ui::RecoveryPhase {
    use ui::RecoveryPhase as P;
    if !state.connected || state.seen.elapsed() >= Duration::from_secs(2) {
        return P::Unknown;
    }
    match state.status.phase {
        wire::COMPLETE if state.status.flags & wire::FAILED != 0 => P::Failed,
        wire::COMPLETE => P::Restored,
        wire::REFUSED => P::Refused,
        wire::UNCERTAIN => P::Uncertain,
        wire::ACCEPTED | wire::RESTORING | wire::CONTROLLER => {
            if state.pending.as_ref().is_some_and(|p| !p.uncertain) {
                P::Restarting
            } else {
                P::Uncertain
            }
        }
        wire::IDLE if state.pending.as_ref().is_some_and(|p| p.uncertain) => P::Uncertain,
        wire::IDLE if state.pending.is_some() => P::Restarting,
        wire::IDLE if state.status.flags & wire::FAILED != 0 => P::Failed,
        wire::IDLE => P::Connected,
        _ => P::Unknown,
    }
}

pub(super) fn disconnected(state: &mut State) {
    state.connected = false;
    if let Some(pending) = state.pending.as_mut() {
        if state.status.operation != pending.command.operation
            || !(state.status.phase == wire::REFUSED
                || state.status.phase == wire::COMPLETE
                    && state.status.generation == pending.command.generation + 1)
        {
            pending.uncertain = true;
        }
    }
    changed();
}
pub(super) fn outgoing(m: &Manager, state: &mut State) -> wire::Command {
    let Some(pending) = state.pending.as_mut() else {
        return wire::Command::default();
    };
    if pending.sent || pending.uncertain || !state.connected {
        return wire::Command::default();
    }
    // Only the still-running executor may deliver. Missing/terminal receipts
    // cannot be promoted into permission to send by a later heartbeat.
    let path = m
        .root
        .join("operator")
        .join(hex(&pending.command.operation))
        .join("result.json");
    if !read_json::<serde_json::Value>(&path).is_ok_and(|record| {
        record["schema"] == 1
            && record["operation"] == hex(&pending.command.operation)
            && record["state"] == "running"
    }) {
        pending.uncertain = true;
        changed();
        return wire::Command::default();
    }
    pending.sent = true;
    changed();
    pending.command
}
pub(super) fn reconnect(state: &mut State, status: wire::Status) -> Result<()> {
    require(
        !state.connected && status.generation >= state.status.generation,
        "proxy_reconnect_generation",
    )?;
    require(
        status.session == state.status.session
            || state
                .pending
                .as_ref()
                .is_some_and(|p| p.launch_claimed && p.command.operation == status.operation),
        "proxy_reconnect_replacement",
    )?;
    state.status = status;
    state.connected = true;
    state.seen = Instant::now();
    Ok(())
}
pub(super) fn command(action: &ui::Action, operation: [u8; 16]) -> Result<wire::Command> {
    let ui::Action::PluginRestart {
        generation,
        snapshot,
        ..
    } = action
    else {
        return Err("proxy_restart_action".into());
    };
    require(valid_hex(&snapshot.sha256, 64), "proxy_snapshot_digest")?;
    let mut digest = [0; 32];
    for (i, b) in digest.iter_mut().enumerate() {
        *b = u8::from_str_radix(&snapshot.sha256[i * 2..i * 2 + 2], 16)?;
    }
    let value = wire::Command {
        operation,
        generation: *generation,
        revision: snapshot.revision,
        digest,
    };
    value.validate()?;
    Ok(value)
}

pub(super) fn report(m: &Manager, session: &str) -> PathBuf {
    m.root
        .join("runtime/results")
        .join(format!("windows-{session}.json"))
}
pub(super) fn retired(m: &Manager, session: &str) -> Result<()> {
    require(valid_hex(session, 32), "proxy_retirement_session")?;
    require(
        !m.root
            .join("runtime/leases")
            .join(format!("{session}.json"))
            .try_exists()?,
        "proxy_host_not_retired",
    )?;
    let value: serde_json::Value = read_json(&report(m, session).with_extension("ownership.json"))?;
    require(
        value["session"].as_str() == Some(session)
            && value["cleanup_confirmed"] == true
            && value["transport_retired"] == true,
        "proxy_host_retirement_unconfirmed",
    )
}

pub(super) fn require_restart_status(status: &wire::Status, command: &wire::Command) -> Result<()> {
    require(
        status.flags & wire::FAILED != 0,
        "This plug-in instance has not reported a failure requiring restart",
    )?;
    require(
        matches!(status.phase, wire::IDLE | wire::COMPLETE | wire::REFUSED),
        "This instance has an unfinished or unconfirmed restart request",
    )?;
    require(
        status.flags & wire::SNAPSHOT != 0,
        "No complete confirmed state is available to restore",
    )?;
    require(
        status.flags & wire::RETIRED != 0,
        "The previous Windows host and transport have not confirmed retirement",
    )?;
    require(status.flags & wire::OWNER_TIMER != 0,
        "The DAW is not servicing this plug-in's controls. Open its controls in your DAW, then try restarting here.")?;
    require(
        command.matches(status),
        "The confirmed snapshot or instance changed; refresh before restarting",
    )
}
pub(super) fn can_forget(m: &Manager, state: &State) -> bool {
    if state.connected {
        return false;
    }
    let Some(pending) = &state.pending else {
        return true;
    };
    // Retire the in-memory association only after its exact executor journal is
    // terminal. An unconfirmed outcome remains fenced by that durable record.
    read_json::<serde_json::Value>(
        &m.root
            .join("operator")
            .join(hex(&pending.command.operation))
            .join("result.json"),
    )
    .is_ok_and(|value| {
        value["schema"] == 1
            && value["operation"] == hex(&pending.command.operation)
            && matches!(value["state"].as_str(), Some("completed" | "refused"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::Fixture;
    #[test]
    fn authority_exact_snapshot_and_positive_retirement() {
        let f = Fixture::new();
        let status = status();
        let command = pending(&status).command;
        require_restart_status(&status, &command).unwrap();
        for changed in [
            wire::Command {
                generation: 2,
                ..command
            },
            wire::Command {
                revision: 4,
                ..command
            },
            wire::Command {
                digest: [9; 32],
                ..command
            },
        ] {
            assert!(require_restart_status(&status, &changed).is_err());
        }
        let session = hex(&status.session);
        assert!(retired(&f.m, &session).is_err());
        let path = report(&f.m, &session).with_extension("ownership.json");
        private_dir(path.parent().unwrap()).unwrap();
        for (cleanup, transport) in [(false, true), (true, false), (true, true)] {
            atomic_json(
                &path,
                &serde_json::json!({"session":session,
                "cleanup_confirmed":cleanup,"transport_retired":transport}),
            )
            .unwrap();
            assert_eq!(retired(&f.m, &session).is_ok(), cleanup && transport);
        }
        let lease =
            f.m.root
                .join("runtime/leases")
                .join(format!("{session}.json"));
        private_dir(lease.parent().unwrap()).unwrap();
        atomic_json(&lease, &serde_json::json!({})).unwrap();
        assert!(retired(&f.m, &session).is_err());
    }
    fn portable_state() -> State {
        State {
            status: status(),
            seen: Instant::now(),
            connected: true,
            pending: None,
        }
    }
    fn legacy_retirement(f: &Fixture, status: &wire::Status) {
        let session = hex(&status.session);
        let path = report(&f.m, &session).with_extension("ownership.json");
        private_dir(path.parent().unwrap()).unwrap();
        private_dir(&f.m.root.join("operator")).unwrap();
        atomic_json(&path, &serde_json::json!({"session":session,
            "cleanup_confirmed":true,"transport_retired":true})).unwrap();
    }
    #[test]
    fn legacy_retired_snapshot_with_settled_history_can_restart_without_empty_custody() {
        let f = Fixture::new();
        let instance = hex(&status().session);
        let state = observation(status());
        legacy_retirement(&f, &state.status);
        // No restart ever requested. The live proxy retains a newer snapshot
        // than the one saved in the DAW; migration must not touch either.
        require_settled_custody(&f.m, &instance, &state.status).unwrap();
        assert!(!binding_path(&f.m, &instance).unwrap().exists());
        // Nor does an old, positively settled request prevent a fresh restart.
        let prior = pending(&state.status).command;
        running(&f, &instance, &prior);
        finish(&f, &prior, "refused");
        let path = f.m.root.join("operator").join(hex(&prior.operation));
        let request = fs::read(path.join("request.json")).unwrap();
        let result = fs::read(path.join("result.json")).unwrap();
        require_settled_custody(&f.m, &instance, &state.status).unwrap();
        assert!(state.pending.is_none());
        assert_eq!(state.status, status());
        assert_eq!(fs::read(path.join("request.json")).unwrap(), request);
        assert_eq!(fs::read(path.join("result.json")).unwrap(), result);
        assert!(!binding_path(&f.m, &instance).unwrap().exists());
    }
    #[test]
    fn legacy_ordinary_restart_preserves_latest_snapshot_and_original_identity() {
        for history in ["never", "refused", "confirmed"] {
            let f = Fixture::new();
            let instance = hex(&status().session);
            let prior = pending(&status()).command;
            let mut live = status();
            if history != "never" {
                running(&f, &instance, &prior);
                finish(&f, &prior, history);
                live.operation = prior.operation;
                live.phase = wire::REFUSED;
                if history == "confirmed" {
                    live.session = [5; 16];
                    live.generation = 2;
                    live.phase = wire::COMPLETE;
                }
            }
            // The current snapshot is newer than both DAW-saved state and the
            // prior restart's receipt. Neither can substitute for this binding.
            live.revision = 9;
            live.digest = [8; 32];
            live.flags = wire::SNAPSHOT | wire::OWNER_TIMER;
            let mut state = observation(live);
            let next = wire::Command { operation: [7; 16], generation: live.generation,
                revision: live.revision, digest: live.digest };
            assert!(matches!(recovery_phase(&state), ui::RecoveryPhase::Connected
                | ui::RecoveryPhase::Refused | ui::RecoveryPhase::Restored));
            assert!(require_restart(&f.m, &instance, &state, &next, None).unwrap_err()
                .to_string().contains("has not reported a failure"));
            assert!(state.pending.is_none());
            assert!(!binding_path(&f.m, &instance).unwrap().exists());
            state.status.flags |= wire::FAILED | wire::RETIRED;
            legacy_retirement(&f, &state.status);
            require_restart(&f.m, &instance, &state, &next, None).unwrap();
            let retained_status = state.status;
            // Ordinary request creation/validation must not block itself.
            running(&f, &instance, &next);
            let path = f.m.root.join("operator").join(hex(&next.operation)).join("result.json");
            let running: serde_json::Value = read_json(&path).unwrap();
            for (phase, stage) in [("unconfirmed", "operator_request_admission"),
                ("queued", ""), ("waiting", "operator_validation_readback")] {
                let mut record = running.clone();
                record["state"] = phase.into();
                record["stage"] = stage.into();
                record["worker_started"] = false.into();
                atomic_json(&path, &record).unwrap();
                require_restart(&f.m, &instance, &state, &next, None).unwrap();
                assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
                assert!(!binding_path(&f.m, &instance).unwrap().exists());
            }
            atomic_json(&path, &running).unwrap();
            // A merely observed running request is still unconfirmed. Only
            // its own new execute call can retain it and expose delivery.
            assert!(require_restart(&f.m, &instance, &state, &next, None).is_err());
            begin_restart(&f.m, &instance, &mut state, next).unwrap();
            assert_eq!(state.status, retained_status);
            assert_eq!(retained_restart(&f.m, &instance).unwrap(), Some(next));
            assert_eq!(outgoing(&f.m, &mut state), next);
            assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
            // The real completion shape settles the migration's new request.
            atomic_json(&path, &serde_json::json!({"schema":1,"operation":hex(&next.operation),
                "state":"completed","outcome":"confirmed","result":{
                    "restart":"completed","instance":instance,"operation":hex(&next.operation),
                    "generation":next.generation + 1,"session":hex(&[6;16]),
                    "snapshot_revision":next.revision,"snapshot_sha256":hex(&next.digest),
                    "controller_confirmed":true}})).unwrap();
            state.status.operation = next.operation;
            state.status.phase = wire::COMPLETE;
            state.status.generation += 1;
            state.status.session = [6; 16];
            state.status.flags &= !wire::FAILED;
            assert_eq!(recovery_phase(&state), ui::RecoveryPhase::Restored);
            require_settled_custody(&f.m, &instance, &state.status).unwrap();
            assert_eq!(state.status.revision, live.revision);
            assert_eq!(state.status.digest, live.digest);
        }
    }
    #[test]
    fn legacy_unreported_uncertainty_and_missing_evidence_still_fence_restart() {
        let f = Fixture::new();
        let instance = hex(&status().session);
        let prior = pending(&status()).command;
        let next = wire::Command { operation: [7; 16], ..prior };
        let mut state = observation(status());
        legacy_retirement(&f, &state.status);
        for outcome in ["running", "unconfirmed", "interrupted", "malformed"] {
            running(&f, &instance, &prior);
            if outcome != "running" { finish(&f, &prior, "unconfirmed"); }
            let path = f.m.root.join("operator").join(hex(&prior.operation)).join("result.json");
            if outcome == "malformed" {
                atomic_json(&path, &serde_json::json!({"schema":1,"operation":hex(&prior.operation),
                    "state":"completed"})).unwrap();
            }
            if outcome == "interrupted" {
                atomic_json(&path, &serde_json::json!({"schema":1,"operation":hex(&prior.operation),
                    "state":"completed","outcome":"unconfirmed","result":{"restart":"unconfirmed"}})).unwrap();
            }
            let before = fs::read(&path).unwrap();
            running(&f, &instance, &next);
            for report_prior in [false, true] {
                state.status.operation = if report_prior { [3; 16] } else { [0; 16] };
                state.status.phase = if report_prior { wire::REFUSED } else { wire::IDLE };
                if report_prior {
                    let older = wire::Command { operation: [3; 16], ..prior };
                    running(&f, &instance, &older);
                    finish(&f, &older, "refused");
                }
                assert!(begin_restart(&f.m, &instance, &mut state, next).is_err(), "{outcome}");
                assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
                assert!(state.pending.is_none());
                assert!(!binding_path(&f.m, &instance).unwrap().exists());
            }
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        // Retirement and a settled selected operation do not repair an
        // unavailable census, a malformed request, or an existing broken index.
        finish(&f, &prior, "refused");
        for missing in ["request", "result", "index", "inventory"] {
            let request = f.m.root.join("operator").join(hex(&prior.operation)).join("request.json");
            let result = request.with_file_name("result.json");
            let request_bytes = fs::read(&request).unwrap();
            let result_bytes = fs::read(&result).unwrap();
            let index = binding_path(&f.m, &instance).unwrap();
            match missing {
                "request" => fs::write(&request, b"{}").unwrap(),
                "result" => fs::remove_file(&result).unwrap(),
                "index" => { private_dir(index.parent().unwrap()).unwrap(); fs::write(&index, b"{}").unwrap(); }
                _ => fs::rename(f.m.root.join("operator"), f.m.root.join("saved-operator")).unwrap(),
            }
            assert!(begin_restart(&f.m, &instance, &mut state, next).is_err(), "{missing}");
            assert!(state.pending.is_none());
            if missing == "inventory" {
                fs::rename(f.m.root.join("saved-operator"), f.m.root.join("operator")).unwrap();
            }
            if missing == "index" {
                assert_eq!(fs::read(&index).unwrap(), b"{}");
                fs::remove_file(index).unwrap();
            }
            fs::write(request, &request_bytes).unwrap();
            fs::write(result, &result_bytes).unwrap();
        }
        begin_restart(&f.m, &instance, &mut state, next).unwrap();
        // Delivery lost after this transition keeps the a6c27/fe83de fence.
        disconnected(&mut state);
        finish(&f, &next, "unconfirmed");
        let returning = state.status;
        reconnect(&mut state, returning).unwrap();
        assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
        let fresh = Manager::new(f.m.root.clone(), f.outer.join("vst3"));
        assert!(unresolved_journal(&fresh, &instance, &returning));
    }
    fn running(f: &Fixture, instance: &str, command: &wire::Command) {
        receipt(f, command.operation, instance, "confirmed");
        let path = f.m.root.join("operator").join(hex(&command.operation));
        atomic_json(
            &path.join("request.json"),
            &serde_json::json!({"action": {
                "kind":"plugin_restart", "instance":instance, "generation":command.generation,
                "snapshot":{"revision":command.revision, "sha256":hex(&command.digest)}
            }}),
        )
        .unwrap();
        atomic_json(
            &path.join("result.json"),
            &serde_json::json!({
                "schema":1, "operation":hex(&command.operation), "state":"running",
                "action": {
                    "kind":"plugin_restart", "instance":instance, "generation":command.generation,
                    "snapshot":{"revision":command.revision, "sha256":hex(&command.digest)}
                }
            }),
        )
        .unwrap();
    }
    fn finish(f: &Fixture, command: &wire::Command, outcome: &str) {
        let value = if outcome == "refused" {
            serde_json::json!({"schema":1,"operation":hex(&command.operation),
                "state":"refused","reason":"Operator action: snapshot changed"})
        } else {
            serde_json::json!({"schema":1,"operation":hex(&command.operation),
                "state":"completed","outcome":outcome,
                "result":fixture_result(command.operation, &hex(&status().session), outcome)})
        };
        atomic_json(
            &f.m.root
                .join("operator")
                .join(hex(&command.operation))
                .join("result.json"),
            &value,
        )
        .unwrap();
    }
    #[test]
    fn authority_disconnect_before_send_and_terminal_receipts_never_dispatch() {
        let f = Fixture::new();
        let instance = hex(&status().session);
        let mut state = portable_state();
        let command = pending(&state.status).command;
        running(&f, &instance, &command);
        retain_restart(&f.m, &instance, &command).unwrap();
        state.pending = Some(Pending {
            sent: false,
            uncertain: false,
            ..pending(&state.status)
        });
        disconnected(&mut state);
        // Reconnect even before execute has had a chance to notice the loss.
        reconnect(&mut state, status()).unwrap();
        assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
        finish(&f, &command, "unconfirmed");
        assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
        // The journal also fences a terminal executor independently of RAM.
        state.pending.as_mut().unwrap().uncertain = false;
        assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
        let result =
            f.m.root
                .join("operator")
                .join(hex(&command.operation))
                .join("result.json");
        for terminal in ["completed", "refused"] {
            atomic_json(
                &result,
                &serde_json::json!({"schema":1,
                "operation":hex(&command.operation),"state":terminal}),
            )
            .unwrap();
            state.pending.as_mut().unwrap().uncertain = false;
            assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
        }
        fs::remove_file(&result).unwrap();
        state.pending.as_mut().unwrap().uncertain = false;
        assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
        // Positive control: an active connected request sends exactly once.
        running(&f, &instance, &command);
        state.pending.as_mut().unwrap().uncertain = false;
        assert_eq!(outgoing(&f.m, &mut state), command);
        assert_eq!(outgoing(&f.m, &mut state), wire::Command::default());
        state.status.operation = command.operation;
        state.status.generation = command.generation + 1;
        state.status.phase = wire::COMPLETE;
        disconnected(&mut state);
        assert!(
            !state.pending.as_ref().unwrap().uncertain,
            "a confirmed completion stays settled on later disconnect"
        );
    }
    #[test]
    fn authority_restart_and_eviction_keep_unreported_exact_operation() {
        let f = Fixture::new();
        let instance = hex(&status().session);
        let command = pending(&status()).command;
        let prior = [3; 16];
        receipt(&f, prior, &instance, "confirmed");
        running(&f, &instance, &command);
        retain_restart(&f.m, &instance, &command).unwrap();
        let fresh = Manager::new(f.m.root.clone(), f.outer.join("vst3"));
        for outcome in ["running", "unconfirmed"] {
            if outcome == "unconfirmed" {
                finish(&f, &command, outcome);
            }
            for operation in [[0; 16], prior, command.operation] {
                let status = wire::Status {
                    operation,
                    ..status()
                };
                assert!(unresolved_journal(&fresh, &instance, &status), "{outcome}");
                assert_eq!(retained_restart(&fresh, &instance).unwrap(), Some(command));
            }
        }
        let state = State {
            connected: false,
            pending: Some(pending(&status())),
            ..portable_state()
        };
        assert!(can_forget(&fresh, &state));
        drop(state);
        assert!(unresolved_journal(&fresh, &instance, &status()));
        // Exact confirmed terminal receipt settles; another identity cannot.
        finish(&f, &command, "confirmed");
        assert!(!unresolved_journal(&fresh, &instance, &status()));
        let request =
            f.m.root
                .join("operator")
                .join(hex(&command.operation))
                .join("request.json");
        let original: serde_json::Value = read_json(&request).unwrap();
        for field in ["instance", "generation", "revision", "sha256"] {
            let mut changed = original.clone();
            match field {
                "instance" => changed["action"][field] = hex(&[9; 16]).into(),
                "generation" => changed["action"][field] = 9.into(),
                "revision" => changed["action"]["snapshot"][field] = 9.into(),
                _ => changed["action"]["snapshot"][field] = hex(&[9; 32]).into(),
            }
            atomic_json(&request, &changed).unwrap();
            assert!(unresolved_journal(&fresh, &instance, &status()), "{field}");
        }
        atomic_json(&request, &original).unwrap();
        fs::write(binding_path(&fresh, &instance).unwrap(), b"broken").unwrap();
        assert!(unresolved_journal(&fresh, &instance, &status()));
    }
    #[test]
    fn authority_legacy_history_without_retirement_refuses_and_preserves_journals() {
        let f = Fixture::new();
        let instance = hex(&status().session);
        let command = pending(&status()).command;
        let prior = [3; 16];
        receipt(&f, prior, &instance, "confirmed");
        for outcome in [
            "running",
            "interrupted",
            "unconfirmed",
            "confirmed",
            "refused",
        ] {
            running(&f, &instance, &command);
            if outcome != "running" {
                finish(
                    &f,
                    &command,
                    if outcome == "interrupted" {
                        "unconfirmed"
                    } else {
                        outcome
                    },
                );
            }
            // Interrupted workers are reconciled to completed/unconfirmed.
            let directory = f.m.root.join("operator").join(hex(&command.operation));
            let request = fs::read(directory.join("request.json")).unwrap();
            let result = fs::read(directory.join("result.json")).unwrap();
            let fresh = Manager::new(f.m.root.clone(), f.outer.join("vst3"));
            for operation in [[0; 16], prior, command.operation] {
                assert!(
                    unresolved_journal(
                        &fresh,
                        &instance,
                        &wire::Status {
                            operation,
                            ..status()
                        }
                    ),
                    "{outcome}"
                );
            }
            assert_eq!(fs::read(directory.join("request.json")).unwrap(), request);
            assert_eq!(fs::read(directory.join("result.json")).unwrap(), result);
            assert!(!binding_path(&f.m, &instance).unwrap().exists());
        }
        // An unrelated positively new instance remains eligible despite legacy history.
        let new_instance = hex(&[9; 16]);
        retain_fresh_instance(&f.m, &new_instance).unwrap();
        assert!(!unresolved_journal(&f.m, &new_instance, &status()));
        // No inventory is read: even unavailable operator inventory cannot turn
        // missing custody into settlement or block positively empty custody.
        fs::rename(
            f.m.root.join("operator"),
            f.m.root.join("preserved-operator"),
        )
        .unwrap();
        fs::write(f.m.root.join("operator"), b"unavailable inventory").unwrap();
        assert!(unresolved_journal(&f.m, &instance, &status()));
        assert!(!unresolved_journal(&f.m, &new_instance, &status()));
    }
    #[test]
    fn authority_fresh_custody_is_create_only_and_missing_or_malformed_is_not_empty() {
        let f = Fixture::new();
        let instance = hex(&status().session);
        assert!(unresolved_journal(&f.m, &instance, &status()));
        retain_fresh_instance(&f.m, &instance).unwrap();
        assert_eq!(retained_restart(&f.m, &instance).unwrap(), None);
        assert!(!unresolved_journal(&f.m, &instance, &status()));
        let command = pending(&status()).command;
        running(&f, &instance, &command);
        retain_restart(&f.m, &instance, &command).unwrap();
        assert!(retain_fresh_instance(&f.m, &instance).is_err());
        assert!(unresolved_journal(&f.m, &instance, &status()));
        finish(&f, &command, "confirmed");
        assert!(!unresolved_journal(&f.m, &instance, &status()));
        let path = binding_path(&f.m, &instance).unwrap();
        for malformed in [
            serde_json::json!({"schema":1,"instance":instance,"operation":hex(&[0;16]),
                "generation":1,"revision":0,"digest":hex(&[0;32])}),
            serde_json::json!({"schema":1,"instance":hex(&[9;16]),"operation":hex(&[0;16]),
                "generation":0,"revision":0,"digest":hex(&[0;32])}),
            serde_json::json!({}),
        ] {
            atomic_json(&path, &malformed).unwrap();
            let before = fs::read(&path).unwrap();
            assert!(unresolved_journal(&f.m, &instance, &status()));
            assert!(retain_fresh_instance(&f.m, &instance).is_err());
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        fs::remove_file(&path).unwrap();
        assert!(unresolved_journal(&f.m, &instance, &status()));
        std::os::unix::fs::symlink("missing-custody", &path).unwrap();
        assert!(unresolved_journal(&f.m, &instance, &status()));
        assert!(retain_fresh_instance(&f.m, &instance).is_err());
    }
    #[test]
    fn authority_legacy_observations_release_capacity_without_erasing_custody() {
        let f = Fixture::new();
        let mut entries = std::collections::BTreeMap::new();
        for i in 1..=80u8 {
            entries.retain(|_, state| !can_forget(&f.m, state));
            assert!(entries.len() < 64);
            let instance = hex(&[i; 16]);
            let mut state = observation(wire::Status {
                session: [i; 16],
                flags: 0,
                ..status()
            });
            assert!(state.pending.is_none());
            assert_eq!(recovery_phase(&state), ui::RecoveryPhase::Connected);
            assert!(require_settled_custody(&f.m, &instance, &state.status).is_err());
            state.status.flags = status().flags;
            assert_eq!(recovery_phase(&state), ui::RecoveryPhase::Failed);
            disconnected(&mut state);
            assert!(can_forget(&f.m, &state));
            assert!(unresolved_journal(&f.m, &instance, &state.status));
            assert!(!binding_path(&f.m, &instance).unwrap().exists());
            entries.insert(instance, state);
        }
        entries.retain(|_, state| !can_forget(&f.m, state));
        assert!(entries.is_empty());
        let instance = hex(&[90; 16]);
        retain_fresh_instance(&f.m, &instance).unwrap();
        let mut state = observation(status());
        require_settled_custody(&f.m, &instance, &state.status).unwrap();
        require_restart_status(&state.status, &pending(&state.status).command).unwrap();
        let command = pending(&state.status).command;
        running(&f, &instance, &command);
        retain_restart(&f.m, &instance, &command).unwrap();
        state.pending = Some(Pending {
            uncertain: false,
            ..pending(&state.status)
        });
        disconnected(&mut state);
        assert!(!can_forget(&f.m, &state), "real executor retains its entry");
        // A new owner's observation is not that executor; its durable fence suffices.
        let mut replacement = observation(status());
        disconnected(&mut replacement);
        assert!(can_forget(&f.m, &replacement));
        assert!(unresolved_journal(&f.m, &instance, &replacement.status));
    }
    #[test]
    fn authority_only_positive_production_receipts_settle_exact_requests() {
        let f = Fixture::new();
        let instance = hex(&status().session);
        let command = pending(&status()).command;
        running(&f, &instance, &command);
        retain_restart(&f.m, &instance, &command).unwrap();
        finish(&f, &command, "confirmed");
        let path =
            f.m.root
                .join("operator")
                .join(hex(&command.operation))
                .join("result.json");
        let confirmed: serde_json::Value = read_json(&path).unwrap();
        assert!(!unresolved_journal(&f.m, &instance, &status()));
        for field in ["schema", "operation", "state", "outcome", "result"] {
            for invalid in [
                None,
                Some(serde_json::json!(false)),
                Some(serde_json::json!("unknown")),
            ] {
                let mut record = confirmed.clone();
                if let Some(value) = invalid {
                    record[field] = value;
                } else {
                    record.as_object_mut().unwrap().remove(field);
                }
                atomic_json(&path, &record).unwrap();
                assert!(unresolved_journal(&f.m, &instance, &status()), "{field}");
            }
        }
        for field in [
            "restart",
            "instance",
            "operation",
            "generation",
            "session",
            "snapshot_revision",
            "snapshot_sha256",
            "controller_confirmed",
        ] {
            for invalid in [
                None,
                Some(serde_json::json!(false)),
                Some(serde_json::json!("unknown")),
            ] {
                let mut record = confirmed.clone();
                if let Some(value) = invalid {
                    record["result"][field] = value;
                } else {
                    record["result"].as_object_mut().unwrap().remove(field);
                }
                atomic_json(&path, &record).unwrap();
                assert!(unresolved_journal(&f.m, &instance, &status()), "{field}");
            }
        }
        let refused = serde_json::json!({"schema":1,"operation":hex(&command.operation),
            "state":"refused","reason":"Operator action: snapshot changed"});
        atomic_json(&path, &refused).unwrap();
        assert!(!unresolved_journal(&f.m, &instance, &status()));
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(false),
            serde_json::json!(""),
        ] {
            let mut record = refused.clone();
            record["reason"] = invalid;
            atomic_json(&path, &record).unwrap();
            assert!(unresolved_journal(&f.m, &instance, &status()));
        }
        for outcome in ["unconfirmed", "completed", "unknown"] {
            finish(&f, &command, outcome);
            assert!(unresolved_journal(&f.m, &instance, &status()));
            let mut record = refused.clone();
            record["outcome"] = outcome.into();
            atomic_json(&path, &record).unwrap();
            assert!(unresolved_journal(&f.m, &instance, &status()));
        }
    }
    #[test]
    fn recovery_phase_requires_a_current_proxy_observation() {
        use ui::RecoveryPhase as P;
        let mut state = State {
            status: status(),
            seen: Instant::now(),
            connected: true,
            pending: None,
        };
        assert_eq!(recovery_phase(&state), P::Failed);
        for phase in [wire::ACCEPTED, wire::RESTORING, wire::CONTROLLER] {
            state.status.phase = phase;
            state.pending = None;
            assert_eq!(recovery_phase(&state), P::Uncertain);
            state.pending = Some(Pending {
                uncertain: false,
                ..pending(&state.status)
            });
            assert_eq!(recovery_phase(&state), P::Restarting);
        }
        state.status.phase = wire::COMPLETE;
        assert_eq!(
            recovery_phase(&state),
            P::Failed,
            "a later failure supersedes a completed restart"
        );
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
    fn fixture_result(operation: [u8; 16], instance: &str, outcome: &str) -> serde_json::Value {
        if outcome == "unconfirmed" {
            return serde_json::json!({"instance":instance,"operation":hex(&operation),
                "restart":"unconfirmed","detail":"The restart outcome is unconfirmed; the bridge will not replay it"});
        }
        serde_json::json!({"restart":if outcome == "confirmed" {"completed"} else {outcome},
            "instance":instance,"operation":hex(&operation),"generation":2,
            "session":hex(&[5;16]),"snapshot_revision":3,"snapshot_sha256":hex(&[4;32]),
            "controller_confirmed":true})
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
            &serde_json::json!({"action":{"kind":"plugin_restart","instance":instance,"generation":1,"snapshot":{"revision":3,"sha256":hex(&[4;32])}}}),
        )
        .unwrap();
        atomic_json(
            &path,
            &serde_json::json!({"schema":1,"operation":hex(&operation),"state":"completed",
            "outcome":outcome,
            "result":fixture_result(operation, instance, outcome)}),
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
}

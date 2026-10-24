//! Durable restart identity and one-shot command custody.
use super::*;

pub(super) struct State {
    pub(super) status: wire::Status,
    pub(super) seen: Instant,
    pub(super) connected: bool,
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
pub(super) fn retain_restart(m: &Manager, instance: &str, command: &wire::Command) -> Result<()> {
    command.validate()?;
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
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    let binding: RestartBinding = read_json(&path)?;
    require(
        binding.schema == 1 && binding.instance == instance,
        "proxy_restart_binding",
    )?;
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
    let request = read_json::<serde_json::Value>(&directory.join("request.json"));
    if !request.is_ok_and(|request| {
        request["action"]["kind"] == "plugin_restart"
            && request["action"]["instance"] == instance
            && exact.is_none_or(|expected| {
                request["action"]["generation"] == expected.generation
                    && request["action"]["snapshot"]["revision"] == expected.revision
                    && request["action"]["snapshot"]["sha256"] == hex(&expected.digest)
            })
    }) {
        return true;
    }
    match read_json::<serde_json::Value>(&directory.join("result.json")) {
        Ok(record) => {
            record["schema"] != 1
                || record["operation"] != hex(&operation)
                || record["outcome"] == "unconfirmed"
                || record["result"]["restart"] == "unconfirmed"
                || !matches!(record["state"].as_str(), Some("completed" | "refused"))
        }
        Err(_) => true,
    }
}
pub(super) fn unresolved_journal(m: &Manager, instance: &str, status: &wire::Status) -> bool {
    match retained_restart(m, instance) {
        Err(_) => return true,
        Ok(Some(command))
            if unresolved_operation(m, instance, command.operation, Some(&command)) =>
        {
            return true
        }
        _ => {}
    }
    unresolved_operation(m, instance, status.operation, None)
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
    fn running(f: &Fixture, instance: &str, command: &wire::Command) {
        receipt(f, command.operation, instance, "completed");
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
                "schema":1, "operation":hex(&command.operation), "state":"running"
            }),
        )
        .unwrap();
    }
    fn finish(f: &Fixture, command: &wire::Command, outcome: &str) {
        atomic_json(
            &f.m.root
                .join("operator")
                .join(hex(&command.operation))
                .join("result.json"),
            &serde_json::json!({"schema":1,"operation":hex(&command.operation),
                "state":"completed","outcome":outcome,"result":{"restart":outcome}}),
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
        receipt(&f, prior, &instance, "completed");
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
        finish(&f, &command, "completed");
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
}

"""Diagnostic-only copies of pinned PC0 functions, with explicit runtime identity.

Source: 309b8918c128c0b9e6701d0453dc841a111d5ac5. All process, plug-in,
normalization and retirement operations are preserved; focused tests compare
these function bodies against that source. Frozen Deck files are never edited.
"""
from __future__ import annotations
from environment import *
from environment import _copy_regular, _bundle_manifest
from supervise import *
from run import *
from run import _deck_keys, _deck_hex, _pc0_validate_durable_records
from pc0_diagnostic_runtime import verify_diagnostic_runner


def verify_environment(environment: ScanEnvironment, *, runner_identity_sha256: str) -> None:
    parent = environment_parent()
    require_contained(environment.root, parent, "WF0 stage")
    expected_name = f".wf0-factory-census.stage-{environment.run_id}"
    if environment.root.name != expected_name or environment.root.is_symlink():
        fail("WF0 stage identity mismatch")
    marker_path = environment.root / ".wf0-owner.json"
    if json.loads(marker_path.read_bytes()) != environment.marker:
        fail("WF0 marker readback mismatch")
    if environment.marker.get("runner_identity_sha256") != runner_identity_sha256:
        fail("WF0 marker runner identity differs")
    if sha256_file(environment.scanner) != environment.marker["scanner_sha256"]:
        fail("staged scanner changed")
    if environment.marker["fixture"] == "adapter" and (
        not environment.adapter.is_file() or environment.adapter.is_symlink()
        or sha256_file(environment.adapter) != environment.marker.get("adapter_sha256")
    ):
        fail("staged loader adapter changed or disappeared")
    if sha256_file(environment.module) != environment.marker["module_sha256"]:
        fail("staged module changed")
    current_bundle = _bundle_manifest(environment.module.parents[2])
    if current_bundle != environment.marker["bundle_manifest"]:
        fail("staged bundle manifest changed")


def retire_environment(environment: ScanEnvironment, *, runner_identity_sha256: str) -> dict[str, Any]:
    verify_environment(environment, runner_identity_sha256=runner_identity_sha256)
    parent = environment_parent()
    require_contained(environment.root, parent, "WF0 retirement stage")
    shutil.rmtree(environment.root)
    if environment.root.exists() or environment.root.is_symlink():
        fail("WF0 stage retirement did not reach exact absence")
    descriptor = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    return {
        "environment_retired": True,
        "stage_absent": True,
        "run_id": environment.run_id,
        "fixture": environment.marker["fixture"],
    }


def create_dx0_environment(run_id: str, *, host: dict[str, Any],
                           fixture: dict[str, Any], execution_source: dict[str, Any],
                           deck_execution_input_sha256: str, runner_identity_sha256: str) -> ScanEnvironment:
    """Compose a verified host-only payload and accepted fixture in one stage."""
    from artifacts import verify_fixture_store, verify_host_store

    if not re.fullmatch(r"[0-9a-f]{32}", run_id):
        fail("DX0 Deck run ID must be 32 lowercase hexadecimal characters")
    host_root = pathlib.Path(host["root"])
    fixture_root = pathlib.Path(fixture["root"])
    verified_host = verify_host_store(
        host_root, host["build_receipt"]["windows_build_input"]["sha256"]
    )
    verified_fixture = verify_fixture_store(fixture_root)
    if (
        verified_host["manifest_sha256"] != host["manifest_sha256"]
        or verified_fixture["identity_sha256"] != fixture["identity_sha256"]
        or verified_fixture["identity"]["bundle_manifest"]["sha256"]
        != DX0_AGAIN_BUNDLE_MANIFEST_SHA256
    ):
        fail("DX0 environment host/fixture identity join differs")

    parent = environment_parent()
    parent.mkdir(parents=True, exist_ok=True)
    root = parent / f".wf0-factory-census.stage-{run_id}"
    require_contained(root, parent, "DX0 execution environment")
    if root.exists() or root.is_symlink():
        fail("DX0 execution environment already exists")
    root.mkdir(mode=0o700)
    for relative in (
        "compatdata/pfx/drive_c/wf0/bin", "compatdata/pfx/drive_c/wf0/fixture",
        "compatdata/pfx/drive_c/wf0/session", "runtime-var", "host-cache",
        "host-config", "host-data", "host-tmp",
    ):
        (root / relative).mkdir(parents=True, mode=0o700, exist_ok=True)
    wf0 = root / "compatdata/pfx/drive_c/wf0"
    _copy_regular(host_root / "bin/wf0-factory-probe.exe",
                  wf0 / "bin/wf0-factory-probe.exe")
    bundle = wf0 / "fixture/again.vst3"
    for record in verified_fixture["identity"]["bundle_manifest"]["records"]:
        _copy_regular(fixture_root / "again.vst3" / record["path"],
                      bundle / record["path"])
    bundle_identity = _bundle_manifest(bundle)
    if bundle_identity != verified_fixture["identity"]["bundle_manifest"]:
        fail("DX0 staged accepted fixture manifest differs")
    marker = {
        "schema": MARKER_SCHEMA,
        "run_id": run_id,
        "fixture": "again",
        "source_commit": execution_source["commit"],
        "implementation_source_manifest_sha256": execution_source["manifest_sha256"],
        "artifact_manifest_sha256": verified_host["manifest_sha256"],
        "accepted_fixture_identity_sha256": verified_fixture["identity_sha256"],
        "deck_execution_input_sha256": deck_execution_input_sha256,
        "runner_identity_sha256": runner_identity_sha256,
        "scanner_sha256": sha256_file(wf0 / "bin/wf0-factory-probe.exe"),
        "module_sha256": sha256_file(bundle / "Contents/x86_64-win/again.vst3"),
        "bundle_manifest": bundle_identity,
    }
    write_atomic(root / ".wf0-owner.json", canonical_json(marker))
    environment = ScanEnvironment(run_id=run_id, root=root, marker=marker)
    verify_environment(environment, runner_identity_sha256=runner_identity_sha256)
    return environment


def supervise(environment: ScanEnvironment, *, hold_gate: bool = False,
              component_case: str = "exact-again",
              mode: str = "wa0-audio-processor-interface-admission") -> dict[str, Any]:
    runner_identity = verify_diagnostic_runner()
    verify_environment(environment, runner_identity_sha256=runner_identity["launch_critical_manifest_sha256"])
    session = secrets.token_hex(16)
    ready = environment.session / f"{session}.ready"
    gate = environment.session / f"{session}.gate"
    if ready.exists() or gate.exists():
        fail("handshake artifacts exist before spawn")
    expected = handshake(environment, session, component_case, mode)
    before = protected_snapshot()
    command_line = command_vector(environment, session, component_case, mode)
    spawn_command_vector_sha256 = sha256_bytes(canonical_json(command_line))
    started = time.monotonic()
    root = subprocess.Popen(command_line, env=controlled_environment(environment),
                            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, start_new_session=True, bufsize=0)
    root_identity = process_identity(root.pid)
    assert root.stdout is not None and root.stderr is not None
    selector = selectors.DefaultSelector()
    selector.register(root.stdout, selectors.EVENT_READ, "stdout")
    selector.register(root.stderr, selectors.EVENT_READ, "stderr")
    streams = StreamState()
    seen_owned: set[tuple[int, int]] = {
        (root_identity["pid"], root_identity["start_ticks"])
    }
    topology_receipt: dict[str, Any] | None = None
    gated_at: float | None = None
    timed_out = False
    held_receipt: dict[str, Any] | None = None
    cleanup_error: str | None = None
    supervision_error: Exception | None = None
    cleanup = {"owned_descendants_zero": False, "process_group_empty": False}
    try:
        while True:
            pump(selector, streams, POLL_SECONDS)
            for record in descendants(root.pid):
                seen_owned.add((record["pid"], record["start_ticks"]))
            now = time.monotonic()
            ready_event = next((record for record in streams.records
                                if record.get("state") == "readiness_announced"), None)
            if gated_at is None and ready_event is not None and ready.is_file():
                if ready.read_bytes() != expected or len(expected) > 1024:
                    fail("ready binding differs from exact expected bytes")
                topology_receipt = topology(
                    root, session, root_identity, spawn_command_vector_sha256
                )
                verify_environment(environment)
                if protected_snapshot() != before:
                    fail("protected state drifted before supervisor gate")
                if hold_gate:
                    hold_start = time.monotonic()
                    while time.monotonic() - hold_start < HELD_GATE_SECONDS:
                        pump(selector, streams, POLL_SECONDS)
                        if root.poll() is not None:
                            fail("scanner departed while the supervisor held its gate")
                        if any(record.get("state") == "module_open_started" or
                               (record.get("event") == "call_started" and
                                record.get("operation") == "load_library")
                               for record in streams.records):
                            fail("module-open activity occurred while gate was absent")
                    mapped = False
                    module_text = str(environment.module.resolve()).lower()
                    for record in descendants(root.pid):
                        try:
                            if module_text in pathlib.Path(f"/proc/{record['pid']}/maps").read_text(
                                    errors="replace").lower():
                                mapped = True
                        except (FileNotFoundError, PermissionError, ProcessLookupError):
                            continue
                    if mapped:
                        fail("staged module was mapped while gate was absent")
                    held_receipt = {
                        "module_open_started": False, "load_library_attempted": False,
                        "again_module_mapped": False, "factory_or_class_event": False,
                        "scanner_waiting": True, "held_seconds": HELD_GATE_SECONDS,
                    }
                    break
                write_atomic(gate, expected)
                gated_at = time.monotonic()
            if gated_at is None and now - started > FIRST_READY_SECONDS:
                timed_out = True
                break
            if streams.in_flight_at is not None and now - streams.in_flight_at > CALL_SECONDS:
                timed_out = True
                break
            if streams.class_started_at is not None and now - streams.class_started_at > CLASS_SECONDS:
                timed_out = True
                break
            if gated_at is not None and now - gated_at > POST_GATE_SECONDS:
                timed_out = True
                break
            if root.poll() is not None:
                for _ in range(20):
                    if not selector.get_map():
                        break
                    pump(selector, streams, 0.01)
                break
    except Exception as error:
        # Capture the first supervision/stream failure so the cleanup attempt
        # can finish and its result can be composed without Python replacing
        # one exception with another from the finally block.
        supervision_error = error
    finally:
        try:
            cleanup = cleanup_process(root, sorted(seen_owned))
        except RuntimeError as error:
            cleanup_error = str(error)
        selector.close()

    if cleanup_error is not None:
        # Once bounded physical containment fails, later stream, protected-state,
        # or runner readback cannot replace that failure or authorize deleting
        # the marker-bound stage.  Return the durable call facts to the Deck
        # owner, which preserves the environment and reports the primary plus
        # the secondary containment blocker.
        last_state = next((record.get("state") for record in reversed(streams.records)
                           if record.get("event") == "lifecycle"), None)
        in_flight = streams.in_flight
        outcome = classify_observed_outcome(
            mode=mode, raw_exit=root.returncode, timed_out=timed_out,
            in_flight=in_flight, last_state=last_state, records=streams.records,
            gated=gated_at is not None, hold_gate=hold_gate, cleanup_failed=True,
        )
        if mode != PC0_MODE:
            fail(cleanup_error)
        semantic_blockers = {
            "PC0_BUS_COUNT_BLOCKED", "PC0_BUS_INFO_BLOCKED",
            "PC0_BUS_ARRANGEMENT_BLOCKED", "PC0_SAMPLE_FORMAT_BLOCKED",
            "PC0_CONTRACT_INCOMPLETE", "PC0_EVIDENCE_BLOCKED",
        }
        if outcome["blocker"] in semantic_blockers:
            outcome["secondary_cleanup_blocker"] = "PC0_PROCESS_CLEANUP_BLOCKED"
        elif supervision_error is not None:
            outcome.update(
                blocker="PC0_EVIDENCE_BLOCKED",
                primary_blocker="PC0_EVIDENCE_BLOCKED",
                secondary_cleanup_blocker="PC0_PROCESS_CLEANUP_BLOCKED",
                classification="supervision_and_process_cleanup_failed",
            )
        else:
            outcome.update(
                blocker="PC0_PROCESS_CLEANUP_BLOCKED",
                primary_blocker="PC0_PROCESS_CLEANUP_BLOCKED",
                secondary_cleanup_blocker=None,
                classification="process_cleanup_failed",
            )
        return {
            "schema": "linux-vst-bridge-wa0-supervised-run/v1",
            "run_id": environment.run_id, "fixture": environment.marker["fixture"],
            "session_binding_sha256": sha256_bytes(expected), "records": streams.records,
            "stdout_sha256": hashlib.sha256(streams.stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(streams.stderr).hexdigest(),
            "stderr_bytes": len(streams.stderr), "raw_exit": root.returncode,
            "classification": outcome["classification"], "blocker": outcome["blocker"],
            "secondary_cleanup_blocker": outcome["secondary_cleanup_blocker"],
            "last_lifecycle": last_state,
            "last_in_flight_operation": None if in_flight is None else in_flight["operation"],
            "audio_processor_observer_state": "audio_processor_ownership_unknown",
            "audio_interface_quiescence": False,
            "call_counts": {operation: sum(
                record.get("event") == "call_started" and record.get("operation") == operation
                for record in streams.records) for operation in sorted(OPERATIONS)},
            "component_case": component_case,
            "inherited_shutdown": {"operations": {}, "clean_in_process_shutdown": False,
                                   "physical_containment_only": True},
            "topology": topology_receipt, "held_gate": held_receipt,
            "cleanup": cleanup, "protected_snapshot": before,
            "runner_identity": runner_identity,
        }

    if supervision_error is not None:
        if mode != PC0_MODE:
            raise supervision_error
        # A stream or supervision failure makes any returned scalar/output
        # unconsumable. Preserve an earlier durable PC0 semantic blocker, but
        # otherwise assign the failure to the evidence/output owner. Physical
        # containment succeeded; this is still not clean in-process shutdown.
        last_state = next((record.get("state") for record in reversed(streams.records)
                           if record.get("event") == "lifecycle"), None)
        in_flight = streams.in_flight
        blocker = pc0_durable_primary(streams.records) or "PC0_EVIDENCE_BLOCKED"
        return {
            "schema": "linux-vst-bridge-wa0-supervised-run/v1",
            "run_id": environment.run_id, "fixture": environment.marker["fixture"],
            "session_binding_sha256": sha256_bytes(expected), "records": streams.records,
            "stdout_sha256": hashlib.sha256(streams.stdout).hexdigest(),
            "stderr_sha256": hashlib.sha256(streams.stderr).hexdigest(),
            "stderr_bytes": len(streams.stderr), "raw_exit": root.returncode,
            "classification": "supervision_failed", "blocker": blocker,
            "secondary_cleanup_blocker": None,
            "last_lifecycle": last_state,
            "last_in_flight_operation": None if in_flight is None else in_flight["operation"],
            "audio_processor_observer_state": "audio_processor_ownership_unknown",
            "audio_interface_quiescence": False,
            "call_counts": {operation: sum(
                record.get("event") == "call_started" and record.get("operation") == operation
                for record in streams.records) for operation in sorted(OPERATIONS)},
            "component_case": component_case,
            "inherited_shutdown": {"operations": {}, "clean_in_process_shutdown": False,
                                   "physical_containment_only": True},
            "topology": topology_receipt, "held_gate": held_receipt,
            "cleanup": cleanup, "protected_snapshot": before,
            "runner_identity": runner_identity,
        }

    if streams.pending:
        fail("scanner stdout ended with a partial JSONL record")
    after = protected_snapshot()
    if after != before:
        fail("protected state drifted during supervised execution")
    if verify_diagnostic_runner() != runner_identity:
        fail("Runtime 4 / Proton 11 identity changed during supervised execution")
    last_state = next((record.get("state") for record in reversed(streams.records)
                       if record.get("event") == "lifecycle"), None)
    in_flight = streams.in_flight
    raw_exit = root.returncode
    outcome = classify_observed_outcome(
        mode=mode, raw_exit=raw_exit, timed_out=timed_out, in_flight=in_flight,
        last_state=last_state, records=streams.records, gated=gated_at is not None,
        hold_gate=hold_gate, cleanup_failed=cleanup_error is not None,
    )
    blocker = outcome["blocker"]
    classification = outcome["classification"]

    started_operations = [
        record.get("operation") for record in streams.records
        if record.get("event") == "call_started"
    ]
    completed_operations = [
        record.get("operation") for record in streams.records
        if record.get("event") == "call_completed"
    ]
    scanner_suppressed = {
        record.get("operation"): record.get("disposition")
        for record in streams.records
        if record.get("event") == "lifecycle"
        and record.get("state") == "inherited_shutdown_suppressed"
    }
    closed_component_sessions = [
        record.get("component_session") for record in streams.records
        if record.get("event") == "lifecycle"
        and record.get("state") == "component_session_closed"
        and isinstance(record.get("component_session"), dict)
    ]
    closed_component_session = (
        closed_component_sessions[-1] if closed_component_sessions else None
    )
    pc0_session_unclosed = pc0_session_is_unclosed(
        mode, streams.records, closed_component_session
    )
    component_abnormal = pc0_session_unclosed or (
        in_flight is not None and in_flight.get("operation") in COMPONENT_OPERATIONS
    )
    audio_interface_abnormal = pc0_session_unclosed or (
        in_flight is not None
        and in_flight.get("operation") in AUDIO_INTERFACE_OPERATIONS
    )
    if audio_interface_abnormal:
        audio_processor_observer_state = "audio_processor_ownership_unknown"
        audio_interface_quiescence = False
    elif closed_component_session is not None:
        closed_audio = closed_component_session.get("audio_processor_lease", {})
        audio_processor_observer_state = closed_audio.get(
            "state", "audio_processor_absent"
        )
        audio_interface_quiescence = closed_audio.get(
            "audio_interface_quiescence", False
        )
    else:
        audio_processor_observer_state = "audio_processor_absent"
        audio_interface_quiescence = True
    shutdown_dispositions: dict[str, dict[str, str]] = {}
    for operation in INHERITED_SHUTDOWN_OPERATIONS:
        if operation in started_operations:
            disposition = (
                "completed" if operation in completed_operations
                else "attempted_without_ordinary_return"
            )
            source = "scanner_call_ledger"
        elif scanner_suppressed.get(operation) in {
            "not_attempted_object_quiescence_unproved",
            "not_attempted_audio_interface_quiescence_unproved",
        }:
            disposition = scanner_suppressed[operation]
            source = "scanner_suppression_record"
        elif component_abnormal:
            disposition = (
                "not_attempted_audio_interface_quiescence_unproved"
                if audio_interface_abnormal
                else "not_attempted_object_quiescence_unproved"
            )
            source = "supervisor_unmatched_component_call"
        else:
            disposition = "not_attempted_prior_stage"
            source = "supervisor_call_ledger_absence"
        shutdown_dispositions[operation] = {
            "disposition": disposition,
            "source": source,
        }
    clean_in_process_shutdown = (
        all(
            value["disposition"] == "completed"
            for value in shutdown_dispositions.values()
        )
        and any(
            record.get("event") == "lifecycle"
            and record.get("state") == "module_unloaded"
            for record in streams.records
        )
    )

    return {
        "schema": "linux-vst-bridge-wa0-supervised-run/v1",
        "run_id": environment.run_id, "fixture": environment.marker["fixture"],
        "session_binding_sha256": sha256_bytes(expected), "records": streams.records,
        "stdout_sha256": hashlib.sha256(streams.stdout).hexdigest(),
        "stderr_sha256": hashlib.sha256(streams.stderr).hexdigest(),
        "stderr_bytes": len(streams.stderr), "raw_exit": raw_exit,
        "classification": classification, "blocker": blocker,
        "secondary_cleanup_blocker": outcome["secondary_cleanup_blocker"],
        "last_lifecycle": last_state,
        "last_in_flight_operation": None if in_flight is None else in_flight["operation"],
        "audio_processor_observer_state": audio_processor_observer_state,
        "audio_interface_quiescence": audio_interface_quiescence,
        "call_counts": {
            operation: started_operations.count(operation)
            for operation in sorted(OPERATIONS)
        },
        "component_case": component_case,
        "inherited_shutdown": {
            "operations": shutdown_dispositions,
            "clean_in_process_shutdown": clean_in_process_shutdown,
            "physical_containment_only": (
                component_abnormal
                or any(
                    value["disposition"] in {
                        "not_attempted_object_quiescence_unproved",
                        "not_attempted_audio_interface_quiescence_unproved",
                    }
                    for value in shutdown_dispositions.values()
                )
            ),
        },
        "topology": topology_receipt, "held_gate": held_receipt,
        "cleanup": cleanup, "protected_snapshot": before,
        "runner_identity": runner_identity,
    }


def validate_failure_diagnostic(value: Any, *, expected_source: dict[str, Any],
                                expected_execution_input_sha256: str,
                                expected_plan_sha256: str,
                                expected_operation_nonce: str,
                                expected_phase_nonce: str, expected_runner_identity_sha256: str) -> dict[str, Any]:
    diagnostic = _deck_keys(
        value, PC0_FAILURE_DIAGNOSTIC_KEYS, "PC0 failure diagnostic"
    )
    if diagnostic["schema"] != PC0_FAILURE_DIAGNOSTIC_SCHEMA:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic schema differs")
    if diagnostic["execution_source"] != expected_source:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic source differs")
    for observed, expected, label, pattern in (
        (diagnostic["operation_nonce"], expected_operation_nonce,
         "operation nonce", DECK_HEX32),
        (diagnostic["phase_nonce"], expected_phase_nonce,
         "phase nonce", DECK_HEX32),
        (diagnostic["execution_input_sha256"], expected_execution_input_sha256,
         "execution input", DECK_HEX64),
        (diagnostic["proof_plan_sha256"], expected_plan_sha256,
         "proof plan", DECK_HEX64),
    ):
        _deck_hex(observed, pattern, label)
        if observed != expected:
            fail(f"PC0_EVIDENCE_BLOCKED: failure diagnostic {label} differs")
    _deck_hex(diagnostic["run_id"], DECK_HEX32, "diagnostic run ID")
    raw_exit = diagnostic["raw_exit"]
    if raw_exit is not None and (type(raw_exit) is not int or not -255 <= raw_exit <= 255):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic raw exit differs")
    classification = diagnostic["classification"]
    if classification not in PC0_DIAGNOSTIC_CLASSIFICATIONS:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic classification differs")
    if (classification == "output_publication_failed" and raw_exit != 99) or (
            raw_exit == 99 and classification not in {
                "output_publication_failed", "supervision_failed",
                "supervision_and_process_cleanup_failed"}):
        fail("PC0_EVIDENCE_BLOCKED: output-publication classification differs")
    if diagnostic["primary_blocker"] not in PC0_DIAGNOSTIC_PRIMARY_BLOCKERS:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic primary blocker differs")
    if diagnostic["secondary_cleanup_blocker"] not in {
            None, "PC0_PROCESS_CLEANUP_BLOCKED"}:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic secondary blocker differs")
    if diagnostic["last_in_flight_operation"] not in {
            None, *PC0_DIAGNOSTIC_OPERATIONS}:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic in-flight operation differs")
    if (type(diagnostic["durable_record_count"]) is not int
            or diagnostic["durable_record_count"] != len(diagnostic["durable_records"])):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic record count differs")
    _pc0_validate_durable_records(
        diagnostic["durable_records"], diagnostic["call_counts"],
        diagnostic["last_in_flight_operation"], diagnostic["last_lifecycle"],
    )
    if diagnostic["audio_processor_observer_state"] not in {
        "audio_processor_absent", "audio_processor_query_in_flight",
        "audio_processor_query_returned_without_lease", "audio_processor_lease_acquired",
        "audio_processor_release_in_flight", "audio_processor_lease_retired",
        "audio_processor_retirement_incomplete", "audio_processor_ownership_unknown",
    } or type(diagnostic["audio_interface_quiescence"]) is not bool:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic interface state differs")
    shutdown = _deck_keys(diagnostic["inherited_shutdown"], {
        "operations", "clean_in_process_shutdown", "physical_containment_only",
    }, "diagnostic inherited shutdown")
    if (set(shutdown["operations"]) != set(PC0_SHUTDOWN_OPERATIONS)
            or type(shutdown["clean_in_process_shutdown"]) is not bool
            or type(shutdown["physical_containment_only"]) is not bool):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic shutdown differs")
    for operation, disposition in shutdown["operations"].items():
        if (operation not in PC0_SHUTDOWN_OPERATIONS
                or not isinstance(disposition, dict)
                or set(disposition) != {"disposition", "source"}
                or disposition["disposition"] not in {
                    "completed", "attempted_without_ordinary_return",
                    "not_attempted_prior_stage",
                    "not_attempted_object_quiescence_unproved",
                    "not_attempted_audio_interface_quiescence_unproved",
                }
                or disposition["source"] not in {
                    "scanner_call_ledger", "scanner_suppression_record",
                    "supervisor_unmatched_component_call",
                    "supervisor_call_ledger_absence",
                }):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic shutdown entry differs")
    cleanup = _deck_keys(diagnostic["cleanup"], {
        "owned_descendants_zero", "process_group_empty",
    }, "diagnostic cleanup")
    if any(type(value) is not bool for value in cleanup.values()):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic cleanup differs")
    if diagnostic["environment_retirement_disposition"] not in {
        "retired", "not_attempted_process_containment_unproved", "failed",
    }:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic retirement differs")
    for key in ("stdout_sha256", "stderr_sha256", "protected_snapshot_sha256",
                "runner_identity_sha256"):
        _deck_hex(diagnostic[key], DECK_HEX64, f"diagnostic {key}")
    if diagnostic["runner_identity_sha256"] != expected_runner_identity_sha256:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic runner differs")
    if (type(diagnostic["stderr_bytes"]) is not int
            or not 0 <= diagnostic["stderr_bytes"] <= 65536):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic stderr bound differs")
    return diagnostic

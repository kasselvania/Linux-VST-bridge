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
from normalize import (validate_wa0_event_order, decode_field, logical_fuid,
    sanitized_timeline, normalize_pc0_census, EXPECTED_CLASSES,
    AUDIO_PROCESSOR_LEASE_SCHEMA, COMPONENT_SESSION_SCHEMA)
from pc0_diagnostic_runtime import verify_diagnostic_runner, sanitized_supervision_error, exception_detail


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
                           deck_execution_input_sha256: str, runner_identity_sha256: str, verify_host=None) -> ScanEnvironment:
    """Compose a verified host-only payload and accepted fixture in one stage."""
    from artifacts import verify_fixture_store, verify_host_store

    if not re.fullmatch(r"[0-9a-f]{32}", run_id):
        fail("DX0 Deck run ID must be 32 lowercase hexadecimal characters")
    host_root = pathlib.Path(host["root"])
    fixture_root = pathlib.Path(fixture["root"])
    verified_host = (verify_host or verify_host_store)(
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
              mode: str = "wa0-audio-processor-interface-admission", checkpoint=None, profile=None) -> dict[str, Any]:
    runner_identity = verify_diagnostic_runner()
    verify_environment(environment, runner_identity_sha256=runner_identity["launch_critical_manifest_sha256"])
    session = secrets.token_hex(16)
    ready = environment.session / f"{session}.ready"
    gate = environment.session / f"{session}.gate"
    if ready.exists() or gate.exists():
        fail("handshake artifacts exist before spawn")
    expected = handshake(environment, session, component_case, mode)
    before = protected_snapshot()
    command_line = (profile.command_vector if profile else command_vector)(environment, session, component_case, mode)
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
    streams = profile.StreamState() if profile else StreamState()
    seen_owned: set[tuple[int, int]] = {
        (root_identity["pid"], root_identity["start_ticks"])
    }
    topology_receipt: dict[str, Any] | None = None
    gated_at: float | None = None
    timed_out = False
    held_receipt: dict[str, Any] | None = None
    cleanup_error: str | None = None
    cleanup_exception = None
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
                verify_environment(environment, runner_identity_sha256=runner_identity["launch_critical_manifest_sha256"])
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
        # The callback is supplied by the reservation worker and never raises.
        # Preserve stream facts before cleanup/retirement can lose their source.
        if checkpoint is not None:
            checkpoint("supervisor_cleanup", {"records": streams.records,
                "raw_exit": root.returncode, "cleanup": cleanup}, supervision_error)
        try:
            cleanup = cleanup_process(root, sorted(seen_owned))
        except RuntimeError as error:
            cleanup_error = str(error)
            cleanup_exception = error
        try:
            selector.close()
        finally:
            if checkpoint is not None:
                checkpoint("supervisor_containment", {"records": streams.records,
                    "raw_exit": root.returncode, "cleanup": cleanup},
                    cleanup_exception)

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
        if mode != PC0_MODE and profile is None:
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
            "supervision_error": sanitized_supervision_error(supervision_error),
            "supervision_exception": exception_detail(supervision_error),
        }

    if supervision_error is not None:
        if mode != PC0_MODE and profile is None:
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
            "supervision_error": sanitized_supervision_error(supervision_error),
            "supervision_exception": exception_detail(supervision_error),
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


# Diagnostic projection of the pinned normalizer. Preserve every product
# assertion; use the returned validated call pairs instead of an undefined
# local from validate_wa0_event_order. Frozen Deck modules remain untouched.
def normalize_wa0_positive(
    run: dict[str, Any], build: dict[str, Any], *, pre_setup: bool = False
) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    """Normalize one exact AGain interface lease without calling its methods."""
    if (run.get("classification"), run.get("blocker"), run.get("raw_exit"),
        run.get("last_in_flight_operation"), run.get("component_case")) != (
            "scanner_completed", None, 0, None, "exact-again"
        ):
        fail("positive WA0 run did not terminate as one complete interface lease")
    records = run.get("records")
    if not isinstance(records, list) or not records:
        fail("positive WC0 event stream is absent")
    starts, completes = validate_wa0_event_order(records, pre_setup=pre_setup)

    final = records[-1]
    raw_session = final.get("component_session")
    if (
        final.get("event") != "lifecycle"
        or final.get("state") != "scanner_completed"
        or final.get("create_instance_called") is not True
        or final.get("controller_instance_created") is not False
        or final.get("audio_processor_interface_queried") is not True
        or final.get("audio_processor_method_called") is not pre_setup
        or not isinstance(raw_session, dict)
    ):
        fail("positive WA0 final record is absent or crosses the claim ceiling")
    expected_processor_raw = "5FDEE8845592534F96FAE4133C935A18"
    expected_component_iid_raw = "31FF31E8D5F20143928EBBEE25697802"
    expected_controller_raw = "655B9DD3AFD7FA42843F4AC841EB04F0"
    expected_audio_iid_raw = "993F0442DAB73C45A569E79D9AAEC33D"
    create = raw_session.get("create", {})
    controller = raw_session.get("controller_id", {})
    initialize = raw_session.get("initialize", {})
    terminate = raw_session.get("terminate", {})
    release = raw_session.get("release", {})
    host = raw_session.get("host", {})
    audio = raw_session.get("audio_processor_lease", {})
    callbacks_raw = raw_session.get("callbacks", {})
    if (
        raw_session.get("state") != "component_released"
        or raw_session.get("primary_blocker") is not None
        or raw_session.get("processor_cid_raw_tuid_hex") != expected_processor_raw
        or raw_session.get("requested_iid_raw_tuid_hex") != expected_component_iid_raw
        or create != {
            "attempted": True, "result_u32_hex": "00000000",
            "output_nonnull": True, "tuple_consistent": True,
        }
        or controller != {
            "attempted": True, "buffer_zero_initialized": True,
            "result_u32_hex": "00000000", "raw_tuid_hex": expected_controller_raw,
            "matches_expected": True,
        }
        or initialize != {
            "attempted": True, "result_u32_hex": "00000000", "succeeded": True,
        }
        or terminate != {
            "attempted": True, "returned_ordinary": True,
            "result_u32_hex": "00000000",
        }
        or release != {
            "attempted": True, "returned_ordinary": True, "reference_count": 0,
            "pointer_cleared": True,
        }
        or host != {
            "created": True, "name": "Linux VST Bridge WC0",
            "reference_baseline": 1, "reference_after_initialize": 2,
            "reference_after_terminate": 1,
            "reference_returned_to_baseline": True,
            "owner_release_attempted": True,
            "owner_release_result": 0,
        }
        or raw_session.get("audio_processor_session_ran") is not True
        or audio != {
            "state": "audio_processor_lease_retired",
            "primary_blocker": None,
            "requested_interface": "Steinberg::Vst::IAudioProcessor",
            "requested_iid_raw_tuid_hex": expected_audio_iid_raw,
            "query": {
                "output_zero_initialized": True,
                "attempted": True,
                "result_u32_hex": "00000000",
                "output_nonnull": True,
                "tuple_consistent": True,
            },
            "lease_acquired": True,
            "release": {
                "attempted": True,
                "returned_ordinary": True,
                "reference_count": 1,
            },
            "pointer_cleared": True,
            "call_in_flight": False,
            "callback_ledger_unchanged": True,
            "audio_interface_quiescence": True,
        }
        or raw_session.get("component_call_in_flight") is not False
        or raw_session.get("callback_ledger_closed") is not True
        or raw_session.get("object_quiescence") is not True
        or raw_session.get("inherited_shutdown_permitted") is not True
    ):
        fail("positive AGain audio lease or component lifecycle differs")

    callback_records = callbacks_raw.get("records")
    expected_callback_shape = [
        ("addRef", "component", "initialize_component", 2),
        ("release", "component", "terminate_component", 1),
        ("release", "owner_local", None, 0),
    ]
    if (
        callbacks_raw.get("capacity") != 64
        or callbacks_raw.get("record_count") != 3
        or callbacks_raw.get("closed") is not True
        or callbacks_raw.get("overflowed") is not False
        or callbacks_raw.get("output_failed") is not False
        or callbacks_raw.get("wrong_thread") is not False
        or callbacks_raw.get("unexpected_object_request") is not False
        or callbacks_raw.get("callback_in_flight") is not False
        or not isinstance(callback_records, list)
        or len(callback_records) != 3
    ):
        fail("positive AGain callback ledger closure differs")
    for raw, expected in zip(callback_records, expected_callback_shape):
        operation, origin, enclosing, count = expected
        if (
            raw.get("operation") != operation
            or raw.get("origin") != origin
            or raw.get("enclosing_operation") != enclosing
            or raw.get("reference_count") != count
            or (origin == "component") != isinstance(
                raw.get("enclosing_attempt_sequence"), int
            )
        ):
            fail("positive AGain callback reference sequence differs")
    callback_events = [item for item in records if item.get("event") == "host_callback"]
    if len(callback_events) != 3:
        fail("positive event stream callback count differs")
    for event, retained in zip(callback_events, callback_records):
        for key in ("operation", "origin", "enclosing_attempt_sequence",
                    "enclosing_operation", "reference_count"):
            if event.get(key) != retained.get(key):
                fail("retained callback ledger differs from synchronously flushed events")

    if final.get("class_count") != 3 or len(final.get("classes", [])) != 3:
        fail("inherited AGain census regression is not the exact three-class roster")
    factory = final.get("factory", {})
    vendor = decode_field(factory.get("vendor_hex"), "utf8", 64)
    url = decode_field(factory.get("url_hex"), "utf8", 256)
    email = decode_field(factory.get("email_hex"), "utf8", 128)
    if (vendor["text"], url["text"], email["text"], factory.get("flags_i32")) != (
        "Steinberg Media Technologies", "http://www.steinberg.net",
        "mailto:info@steinberg.de", 16
    ):
        fail("inherited AGain factory metadata regression differs")
    class_ids = []
    for ordinal, raw in enumerate(final["classes"]):
        raw_id = raw.get("raw_tuid_hex")
        class_id = logical_fuid(raw_id)
        encoding = "utf16le" if raw.get("tier") == "IPluginFactory3.PClassInfoW" else "utf8"
        name = decode_field(raw.get("name_hex"), encoding, 64)["text"]
        category = decode_field(raw.get("category_hex"), "utf8", 32)["text"]
        expected = EXPECTED_CLASSES[ordinal]
        if (class_id, name, category) != expected[:3]:
            fail("inherited AGain ordered class regression differs")
        class_ids.append({
            "ordinal": ordinal, "logical_class_id": class_id,
            "raw_windows_tuid": raw_id, "name": name, "category": category,
        })

    timeline = sanitized_timeline(run)
    callback_ledger = {
        "host_name": "Linux VST Bridge WC0",
        "capacity": 64,
        "record_count": 3,
        "records": callback_records,
        "component_originated_reference_sequence": [2, 1],
        "owner_final_release": 0,
        "closed": True,
        "overflowed": False,
        "output_failed": False,
        "wrong_thread": False,
        "unexpected_host_object_request": False,
        "unchanged_across_audio_processor_lease": True,
    }
    source = build["implementation_source_manifest"]
    scanner_record = next(item for item in build["artifact_set"]["records"]
                          if item["path"] == "bin/wf0-factory-probe.exe")
    module_sha256 = next(
        item["sha256"] for item in build["again_bundle_manifest"]["records"]
        if item["path"] == "Contents/x86_64-win/again.vst3"
    )
    source_identity = {
        "schema": source["schema"], "commit": source["commit"],
        "tree": build["source_tree"], "record_count": source["record_count"],
        "manifest_sha256": build["implementation_source_manifest_sha256"],
    }
    audio_processor_lease = {
        "schema": AUDIO_PROCESSOR_LEASE_SCHEMA,
        "implementation_source_identity": source_identity,
        "scanner_sha256": scanner_record["sha256"],
        "module_sha256": module_sha256,
        "owner": "AudioProcessorInterfaceLease",
        "component_owner_reference_baseline": 1,
        "interface": {
            "name": "Steinberg::Vst::IAudioProcessor",
            "logical_iid": logical_fuid(expected_audio_iid_raw),
            "raw_windows_tuid": expected_audio_iid_raw,
            "pointer_identity_retained": False,
        },
        "state": audio["state"],
        "query": audio["query"],
        "lease_acquired": audio["lease_acquired"],
        "release": audio["release"],
        "pointer_cleared": audio["pointer_cleared"],
        "call_in_flight": audio["call_in_flight"],
        "callback_ledger_unchanged": audio["callback_ledger_unchanged"],
        "audio_interface_quiescence": audio["audio_interface_quiescence"],
        "call_attribution": {
            "closed_operations": [
                "query_audio_processor", "release_audio_processor"
            ],
            "new_operation_count": 2,
            "total_operation_count": 22,
            "started_count": 2,
            "completed_count": 2,
            "last_in_flight_operation": None,
        },
        "audio_processor_method_called": False,
        "explicit_nonclaims": [
            "no_audio_processor_method", "no_interface_pointer_identity",
            "no_controller_creation", "no_connection_point", "no_bus_access",
            "no_parameter_access", "no_state_access", "no_processing_setup",
            "no_audio", "no_events", "no_editor", "no_proxy", "no_ipc",
            "no_bitwig", "no_serum",
        ],
        "stage_timeline_sha256": sha256_bytes(canonical_json(timeline)),
    }
    component_session = {
        "schema": COMPONENT_SESSION_SCHEMA,
        "implementation_source_identity": source_identity,
        "scanner_sha256": scanner_record["sha256"],
        "module_sha256": module_sha256,
        "processor": {
            "logical_cid": logical_fuid(expected_processor_raw),
            "raw_windows_tuid": expected_processor_raw,
            "requested_interface": "Steinberg::Vst::IComponent",
            "requested_interface_logical_iid": logical_fuid(expected_component_iid_raw),
            "requested_interface_raw_windows_tuid": expected_component_iid_raw,
        },
        "controller": {
            "logical_cid": logical_fuid(expected_controller_raw),
            "raw_windows_tuid": expected_controller_raw,
            "result_u32_hex": controller["result_u32_hex"],
            "queried_before_initialize": True,
            "complete_output_zero_initialized": True,
            "matched": True,
        },
        "create": create,
        "initialize": initialize,
        "audio_processor_lease": {
            "state": audio["state"],
            "query_result_u32_hex": audio["query"]["result_u32_hex"],
            "query_output_nonnull": audio["query"]["output_nonnull"],
            "release_reference_count": audio["release"]["reference_count"],
            "pointer_cleared": audio["pointer_cleared"],
            "quiescence": audio["audio_interface_quiescence"],
        },
        "terminate": terminate,
        "component_release": release,
        "host_reference_sequence": [1, 2, 1, 0],
        "callback_ledger": callback_ledger,
        "component_state": "component_released",
        "object_quiescence": {
            "value": True,
            "facts": {
                "audio_interface_quiescent_before_terminate": True,
                "initialize_succeeded": True,
                "terminate_attempted_once_and_returned_ordinary": True,
                "component_release_returned_zero": True,
                "component_call_in_flight": False,
                "component_pointer_cleared": True,
                "host_reference_returned_to_baseline": True,
                "host_owner_final_release_returned_zero": True,
                "host_callback_in_flight": False,
                "callback_ledger_closed": True,
            },
        },
        "inherited_wf0_regression": {
            "factory_vendor": vendor["text"],
            "ordered_class_census": class_ids,
            "factory_release_order": [
                "release_factory_3", "release_factory_2", "release_factory_base"
            ],
            "module_exit": final.get("module_exit"),
            "module_unload": final.get("module_unload"),
            "clean_in_process_shutdown": True,
        },
        "call_attribution": {
            "closed_operation_count": 22,
            "new_wa0_operation_count": 2,
            "started_count": len(starts),
            "completed_count": len(completes),
            "last_in_flight_operation": None,
        },
        "controller_instance_created": False,
        "forbidden_component_method_called": False,
        "audio_processor_method_called": False,
        "explicit_nonclaims": [
            "no_controller_creation", "no_connection_point",
            "no_audio_processor_method", "no_bus_host_call",
            "no_parameter_host_call", "no_state_host_call", "no_processing",
            "no_audio", "no_events", "no_editor", "no_bitwig", "no_serum",
            "no_proxy", "no_ipc",
        ],
        "stage_timeline_sha256": sha256_bytes(canonical_json(timeline)),
        "callback_ledger_sha256": sha256_bytes(canonical_json(callback_ledger)),
    }
    if pre_setup:
        contract = normalize_pc0_census(records, raw_session.get("processing_contract"))
        audio_processor_lease["processing_contract"] = contract
        for projection in (audio_processor_lease, component_session):
            projection["audio_processor_method_called"] = True
            projection["explicit_nonclaims"] = [claim for claim in projection["explicit_nonclaims"]
                if claim not in {"no_audio_processor_method", "no_bus_host_call", "no_bus_access"}]
        component_session["call_attribution"]["closed_operation_count"] = 33
    return audio_processor_lease, component_session, timeline

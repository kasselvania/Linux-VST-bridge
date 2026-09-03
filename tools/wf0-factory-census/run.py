#!/usr/bin/env python3
"""Deck-side DX0 positive batch with durable pre-acknowledgement publication."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import re
import secrets
import sys
from typing import Any

sys.dont_write_bytecode = True

from artifacts import (
    read_canonical_json, verify_fixture_store, verify_hash_sidecar,
    verify_host_store, verify_source_handoff,
)
from common import (
    PC0_BLOCKED_OUTCOMES, PC0_REF, PC0_PLAN_ID, PC0_RESULT_SCHEMA, PC0_MODE,
    PC0_OPERATIONS,
    pc0_validate_contract, pc0_validate_call_facts, dx0_closed_plan,
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_AGAIN_MODULE_SHA256,
    DX0_DECK_EXECUTION_INPUT_SCHEMA, DX0_PLAN_ID, DX0_REF, DX0_RESULT_SCHEMA,
    RUNNER_DIGEST, canonical_json, deck_fixture_identity,
    dx0_closed_plan, dx0_complete_source, dx0_deck_execution_input,
    dx0_deck_fixture_parent, dx0_deck_host_artifact_parent,
    dx0_deck_result_parent, dx0_deck_source_parent, dx0_identity_sha256,
    dx0_require_frozen_source, dx0_source_role, fail, parse_json_no_duplicates,
    process_guard, protected_snapshot, repo_root, sha256_bytes, sha256_file,
    verify_runner_identity, write_atomic,
)
from environment import create_dx0_environment, retire_environment
from normalize import normalize_wa0_positive
from supervise import supervise


DECK_RESULT_KEYS = {
    "schema", "operation_nonce", "artifact_producer_source",
    "deck_execution_source", "execution_input", "host_artifact",
    "accepted_fixture", "source_handoff", "closed_plan",
    "original_observation", "positive_result", "call_facts", "quiescence",
    "shutdown", "cleanup", "protected_state", "integrity",
}
DECK_SOURCE_KEYS = {
    "identity_sha256", "commit", "tree", "parent", "ref", "manifest_sha256",
}
DECK_HEX40 = re.compile(r"[0-9a-f]{40}")
DECK_HEX64 = re.compile(r"[0-9a-f]{64}")
DECK_HEX32 = re.compile(r"[0-9a-f]{32}")


def pc0_compose_failure(error: Exception | None,
                        retirement_failed: bool = False) -> RuntimeError | None:
    """Retain the first PC0 owner while recording later physical-cleanup failure."""
    if error is None and not retirement_failed:
        return None
    if error is None:
        return RuntimeError(
            "PC0_PROCESS_CLEANUP_BLOCKED: environment retirement failed"
        )
    match = re.match(r"^([A-Z0-9_]+)(?::|$)", str(error))
    blocker = match.group(1) if match and match.group(1) in PC0_BLOCKED_OUTCOMES else (
        "PC0_EVIDENCE_BLOCKED"
    )
    message = f"{blocker}: positive batch did not complete"
    if retirement_failed and blocker != "PC0_PROCESS_CLEANUP_BLOCKED":
        message += "; secondary=PC0_PROCESS_CLEANUP_BLOCKED"
    return RuntimeError(message)


def _deck_keys(value: Any, expected: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        fail(f"DX0 {label} key roster differs")
    return value


def _deck_hex(value: Any, pattern: re.Pattern[str], label: str) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        fail(f"DX0 {label} is malformed")
    return value


def _deck_positive_int(value: Any, label: str) -> int:
    if type(value) is not int or value <= 0:
        fail(f"DX0 {label} is not a positive JSON integer")
    return value


def _deck_typed_exact(value: dict[str, Any], expected: dict[str, Any],
                      label: str) -> None:
    if set(value) != set(expected):
        fail(f"DX0 {label} key roster differs")
    for key, expected_value in expected.items():
        observed = value[key]
        if type(observed) is not type(expected_value) or observed != expected_value:
            fail(f"DX0 {label} differs: {key}")


def _deck_timestamp(value: Any, label: str) -> dt.datetime:
    if (not isinstance(value, str)
            or re.fullmatch(
                r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
                value,
            ) is None):
        fail(f"DX0 {label} is malformed")
    try:
        return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        fail(f"DX0 {label} is invalid")


def _deck_source(value: Any, label: str) -> dict[str, Any]:
    source = _deck_keys(value, DECK_SOURCE_KEYS, label)
    _deck_hex(source["identity_sha256"], DECK_HEX64, f"{label} identity")
    _deck_hex(source["commit"], DECK_HEX40, f"{label} commit")
    _deck_hex(source["tree"], DECK_HEX40, f"{label} tree")
    _deck_hex(source["parent"], DECK_HEX40, f"{label} parent")
    _deck_hex(source["manifest_sha256"], DECK_HEX64, f"{label} manifest")
    if source["ref"] not in {DX0_REF, PC0_REF}:
        fail(f"DX0 {label} ref is malformed")
    return source


def validate_retained_result(
        value: Any, *, expected_execution_input_sha256: str | None = None,
        expected_plan_sha256: str | None = None) -> dict[str, Any]:
    """Deck-local strict admission for one retained DX0 positive result."""
    result = _deck_keys(value, DECK_RESULT_KEYS, "transaction result")
    pc0 = result["schema"] == PC0_RESULT_SCHEMA
    expected_plan = dx0_closed_plan(PC0_PLAN_ID if pc0 else DX0_PLAN_ID)
    if result["schema"] not in {DX0_RESULT_SCHEMA, PC0_RESULT_SCHEMA}:
        fail("DX0 transaction-result schema differs")
    _deck_hex(result["operation_nonce"], DECK_HEX32, "operation nonce")
    _deck_source(result["artifact_producer_source"], "artifact producer source")
    _deck_source(result["deck_execution_source"], "Deck execution source")

    execution = _deck_keys(result["execution_input"], {
        "identity_sha256", "schema", "proof_plan_sha256", "runtime_proton_sha256",
        "source_handoff_ref", "detached_worktree_commit",
        "host_artifact_manifest_sha256", "accepted_fixture_identity_sha256",
    }, "execution input")
    if execution["schema"] != DX0_DECK_EXECUTION_INPUT_SCHEMA:
        fail("DX0 Deck-execution-input schema differs")
    for key in (
            "identity_sha256", "proof_plan_sha256", "runtime_proton_sha256",
            "host_artifact_manifest_sha256", "accepted_fixture_identity_sha256"):
        _deck_hex(execution[key], DECK_HEX64, f"execution input {key}")
    _deck_hex(
        execution["detached_worktree_commit"], DECK_HEX40,
        "execution worktree commit",
    )
    if execution["runtime_proton_sha256"] != RUNNER_DIGEST:
        fail("DX0 result Runtime/Proton identity differs")
    if (expected_execution_input_sha256 is not None
            and execution["identity_sha256"] != expected_execution_input_sha256):
        fail("DX0 cached result execution-input identity differs")
    if (expected_plan_sha256 is not None
            and execution["proof_plan_sha256"] != expected_plan_sha256):
        fail("DX0 cached result proof-plan identity differs")

    host = _deck_keys(result["host_artifact"], {
        "windows_build_input_sha256", "workflow_run_id", "run_attempt", "artifact_id",
        "manifest_sha256", "build_receipt_sha256", "mac_custody_receipt_sha256",
    }, "host artifact")
    for key in ("workflow_run_id", "run_attempt", "artifact_id"):
        _deck_positive_int(host[key], f"host artifact {key}")
    for key in (
            "windows_build_input_sha256", "manifest_sha256",
            "build_receipt_sha256", "mac_custody_receipt_sha256"):
        _deck_hex(host[key], DECK_HEX64, f"host artifact {key}")
    if host["manifest_sha256"] != execution["host_artifact_manifest_sha256"]:
        fail("DX0 result host manifest/execution-input join differs")

    fixture = _deck_keys(result["accepted_fixture"], {
        "identity_sha256", "bundle_manifest_sha256", "module_sha256",
        "mac_store_receipt_sha256", "deck_store_receipt_sha256",
    }, "accepted fixture")
    for key in fixture:
        _deck_hex(fixture[key], DECK_HEX64, f"accepted fixture {key}")
    if (fixture["bundle_manifest_sha256"] != DX0_AGAIN_BUNDLE_MANIFEST_SHA256
            or fixture["module_sha256"] != DX0_AGAIN_MODULE_SHA256
            or fixture["identity_sha256"]
            != execution["accepted_fixture_identity_sha256"]):
        fail("DX0 accepted fixture/result join differs")

    handoff = _deck_keys(result["source_handoff"], {
        "bundle_sha256", "receipt_sha256", "advertised_ref", "worktree_commit",
        "worktree_clean",
    }, "source handoff")
    _deck_hex(handoff["bundle_sha256"], DECK_HEX64, "source bundle")
    _deck_hex(handoff["receipt_sha256"], DECK_HEX64, "source handoff receipt")
    _deck_hex(handoff["worktree_commit"], DECK_HEX40, "source handoff worktree")
    if (handoff["worktree_clean"] is not True
            or handoff["worktree_commit"]
            != result["deck_execution_source"]["commit"]
            or execution["detached_worktree_commit"]
            != result["deck_execution_source"]["commit"]
            or execution["detached_worktree_commit"] != handoff["worktree_commit"]
            or handoff["advertised_ref"] != execution["source_handoff_ref"]
            or handoff["advertised_ref"] != (
                "refs/handoff/dx0-source/"
                + result["deck_execution_source"]["commit"])):
        fail("DX0 source handoff/execution-source join differs")

    plan = _deck_keys(result["closed_plan"], {
        "plan_id", "sha256", "expected_result", "live_exercise_ceiling",
    }, "closed plan")
    _deck_hex(plan["sha256"], DECK_HEX64, "proof plan")
    if (plan["plan_id"] != expected_plan["plan_id"]
            or plan["sha256"] != execution["proof_plan_sha256"]
            or plan["expected_result"]
            != expected_plan["expected_result"]
            or type(plan["live_exercise_ceiling"]) is not int
            or plan["live_exercise_ceiling"] != 1):
        fail("DX0 retained closed plan differs")

    if plan["sha256"] != dx0_identity_sha256(expected_plan):
        fail("PC0 retained plan digest differs")
    if pc0 and result["deck_execution_source"]["ref"] != PC0_REF:
        fail("PC0 original execution source branch differs")

    observation = _deck_keys(result["original_observation"], {
        "run_id", "phase_nonce", "event_stream_sha256", "completion_disposition",
        "started_utc", "completed_utc",
    }, "original observation")
    _deck_hex(observation["run_id"], DECK_HEX32, "Deck observation run ID")
    _deck_hex(observation["phase_nonce"], DECK_HEX32, "Deck phase nonce")
    _deck_hex(observation["event_stream_sha256"], DECK_HEX64, "event stream")
    started = _deck_timestamp(observation["started_utc"], "observation start")
    completed = _deck_timestamp(
        observation["completed_utc"], "observation completion"
    )
    if (observation["completion_disposition"] != "completed"
            or completed < started):
        fail("DX0 original observation is incomplete")

    positive = _deck_keys(result["positive_result"], {
        "query_result_u32_hex", "query_output_nonnull", "query_tuple_consistent",
        "interface_release_reference_count", "audio_processor_method_called",
        "fixture", "interface_logical_iid", "interface_raw_windows_tuid",
    } | ({"processing_contract"} if pc0 else set()), "positive result")
    if pc0:
        pc0_validate_contract(positive["processing_contract"])
    _deck_typed_exact({key: item for key, item in positive.items() if key != "processing_contract"}, {
        "query_result_u32_hex": "00000000", "query_output_nonnull": True,
        "query_tuple_consistent": True, "interface_release_reference_count": 1,
        "audio_processor_method_called": pc0, "fixture": "AGain VST3",
        "interface_logical_iid": "42043F99B7DA453CA569E79D9AAEC33D",
        "interface_raw_windows_tuid": "993F0442DAB73C45A569E79D9AAEC33D",
    }, "cached positive result")

    calls = _deck_keys(result["call_facts"], {
        "paired_call_ledger", "started_count", "completed_count",
        "last_in_flight_operation", "query_audio_processor_count",
        "release_audio_processor_count", "ledger_overflowed",
    } | ({"pc0_operation_counts", "ledger"} if pc0 else set()), "call facts")
    if pc0:
        pc0_validate_call_facts(calls)
    _deck_typed_exact({key: item for key, item in calls.items() if key not in {"pc0_operation_counts", "ledger"}}, {
        "paired_call_ledger": True, "started_count": 33 if pc0 else 22, "completed_count": 33 if pc0 else 22,
        "last_in_flight_operation": None, "query_audio_processor_count": 1,
        "release_audio_processor_count": 1, "ledger_overflowed": False,
    }, "cached call ledger")
    quiescence = _deck_keys(
        result["quiescence"], {"interface_quiescence", "object_quiescence"},
        "quiescence",
    )
    _deck_typed_exact(
        quiescence, {"interface_quiescence": True, "object_quiescence": True},
        "cached quiescence",
    )
    shutdown = _deck_keys(result["shutdown"], {
        "component_release_reference_count", "reverse_factory_release", "exit_dll",
        "free_library", "scanner_completed", "clean_in_process_shutdown",
    }, "shutdown")
    _deck_typed_exact(shutdown, {
        "component_release_reference_count": 0, "reverse_factory_release": True,
        "exit_dll": True, "free_library": True, "scanner_completed": True,
        "clean_in_process_shutdown": True,
    }, "cached in-process shutdown")
    cleanup = _deck_keys(result["cleanup"], {
        "owned_descendant_count", "process_group_empty", "environment_retired",
        "stage_absent",
    }, "cleanup")
    _deck_typed_exact(cleanup, {
        "owned_descendant_count": 0, "process_group_empty": True,
        "environment_retired": True, "stage_absent": True,
    }, "cached physical cleanup")
    protected = _deck_keys(result["protected_state"], {
        "pre_sha256", "post_sha256", "equal", "comparison_completed",
    }, "protected state")
    _deck_hex(protected["pre_sha256"], DECK_HEX64, "protected pre-state")
    _deck_hex(protected["post_sha256"], DECK_HEX64, "protected post-state")
    if (protected["equal"] is not True
            or protected["comparison_completed"] is not True
            or protected["pre_sha256"] != protected["post_sha256"]):
        fail("DX0 cached protected-state comparison is incomplete")

    integrity = _deck_keys(result["integrity"], {
        "positive_result_sha256", "call_facts_sha256", "quiescence_sha256",
        "shutdown_sha256", "cleanup_sha256", "protected_state_sha256",
    }, "integrity")
    projections = {
        "positive_result_sha256": positive,
        "call_facts_sha256": calls,
        "quiescence_sha256": quiescence,
        "shutdown_sha256": shutdown,
        "cleanup_sha256": cleanup,
        "protected_state_sha256": protected,
    }
    for key, projection in projections.items():
        _deck_hex(integrity[key], DECK_HEX64, f"integrity {key}")
        if integrity[key] != sha256_bytes(canonical_json(projection)):
            fail(f"DX0 cached result integrity differs: {key}")
    return result


def validate_retained_result_file(
        path: pathlib.Path, *, expected_execution_input_sha256: str | None = None,
        expected_plan_sha256: str | None = None) -> dict[str, Any]:
    """Deck-local canonical JSON and external-sidecar admission."""
    if (not path.is_file() or path.is_symlink()
            or path.stat().st_size > 2 * 1024 * 1024):
        fail("DX0 retained result is absent or unsafe")
    sidecar = path.with_suffix(path.suffix + ".sha256")
    if (not sidecar.is_file() or sidecar.is_symlink()
            or sidecar.stat().st_size > 256):
        fail("DX0 retained result sidecar is absent or unsafe")
    if sidecar.read_bytes() != f"{sha256_file(path)}  {path.name}\n".encode():
        fail("DX0 retained result sidecar differs")
    data = path.read_bytes()
    value = parse_json_no_duplicates(data, "DX0 retained result")
    if canonical_json(value) != data:
        fail("DX0 retained result is not canonical")
    return validate_retained_result(
        value,
        expected_execution_input_sha256=expected_execution_input_sha256,
        expected_plan_sha256=expected_plan_sha256,
    )


def _utc() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _protected_digest(value: dict[str, Any]) -> str:
    return sha256_bytes(canonical_json(value))


def _load_intent(path: pathlib.Path) -> dict[str, Any]:
    value = read_canonical_json(path, "linux-vst-bridge-dx0-deck-intent/v1")
    expected = {
        "schema", "operation_nonce", "phase_nonce", "execution_source",
        "deck_execution_input", "deck_execution_input_sha256", "proof_plan", "proof_plan_sha256",
        "host_artifact_manifest_sha256", "accepted_fixture_identity_sha256",
        "source_handoff_receipt_sha256",
    }
    if set(value) != expected:
        fail("DX0 Deck intent key roster differs")
    if (not re.fullmatch(r"[0-9a-f]{32}", str(value["operation_nonce"]))
            or not re.fullmatch(r"[0-9a-f]{32}", str(value["phase_nonce"]))):
        fail("DX0 Deck intent nonce differs")
    return value


def _build_for_normalizer(source: dict[str, Any], host: dict[str, Any],
                          fixture: dict[str, Any]) -> dict[str, Any]:
    return {
        "implementation_source_manifest": {
            "schema": source["schema"], "commit": source["commit"],
            "record_count": source["record_count"], "records": source["records"],
        },
        "implementation_source_manifest_sha256": dx0_source_role(source)["manifest_sha256"],
        "source_tree": source["tree"],
        "artifact_set": {"records": host["manifest"]["records"]},
        "again_bundle_manifest": fixture["identity"]["bundle_manifest"],
    }


def _publish_result(result: dict[str, Any], execution_input_sha: str) -> pathlib.Path:
    validate_retained_result(
        result, expected_execution_input_sha256=execution_input_sha,
        expected_plan_sha256=result["closed_plan"]["sha256"],
    )
    parent = dx0_deck_result_parent()
    parent.mkdir(parents=True, exist_ok=True)
    if parent.is_symlink():
        fail("DX0 Deck result parent is a symlink")
    target = parent / execution_input_sha
    if target.is_symlink():
        fail("DX0 Deck result root is a symlink")
    if target.exists():
        if not target.is_dir():
            fail("DX0 Deck result root is unsafe")
        existing = target / "DX0_TRANSACTION_RESULT.json"
        accepted = validate_retained_result_file(
            existing, expected_execution_input_sha256=execution_input_sha,
            expected_plan_sha256=result["closed_plan"]["sha256"],
        )
        if accepted != result:
            fail("DX0 Deck result publication conflicts with retained result")
        return existing
    stage = parent / f".dx0-result-stage-{execution_input_sha}"
    if stage.exists() or stage.is_symlink():
        fail("DX0 Deck result publication outcome is unresolved")
    stage.mkdir(mode=0o700)
    path = stage / "DX0_TRANSACTION_RESULT.json"
    write_atomic(path, canonical_json(result), 0o400)
    digest = sha256_file(path)
    write_atomic(stage / "DX0_TRANSACTION_RESULT.json.sha256",
                 f"{digest}  DX0_TRANSACTION_RESULT.json\n".encode(), 0o400)
    stage.chmod(0o500)
    os.replace(stage, target)
    validate_retained_result_file(
        target / path.name,
        expected_execution_input_sha256=execution_input_sha,
        expected_plan_sha256=result["closed_plan"]["sha256"],
    )
    return target / path.name


def execute(intent_path: pathlib.Path) -> dict[str, Any]:
    if sys.executable != "/usr/bin/python3":
        fail(f"DX0 Deck execution requires /usr/bin/python3; observed {sys.executable}")
    intent = _load_intent(intent_path)
    source_commit = intent["execution_source"]["commit"]
    source = dx0_require_frozen_source(source_commit, detached=True)
    source_role = dx0_source_role(source)
    if source_role != intent["execution_source"]:
        fail("DX0 Deck execution source differs from intent")
    plan = dx0_closed_plan(intent["proof_plan"].get("plan_id"))
    pc0 = plan["plan_id"] == PC0_PLAN_ID
    plan_sha = dx0_identity_sha256(plan)
    if plan != intent["proof_plan"] or plan_sha != intent["proof_plan_sha256"]:
        fail("DX0 Deck closed plan differs from intent")

    handoff_root = dx0_deck_source_parent() / source_commit
    handoff = verify_source_handoff(handoff_root, source_commit)
    if handoff["receipt_sha256"] != intent["source_handoff_receipt_sha256"]:
        fail("DX0 Deck source-handoff receipt differs from intent")
    host_root = dx0_deck_host_artifact_parent() / intent["host_artifact_manifest_sha256"]
    build_input_sha = read_canonical_json(
        host_root / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json"
    )["windows_build_input"]["sha256"]
    host = verify_host_store(host_root, build_input_sha)
    fixture_root = dx0_deck_fixture_parent() / DX0_AGAIN_BUNDLE_MANIFEST_SHA256
    fixture = verify_fixture_store(fixture_root)
    if fixture["identity_sha256"] != intent["accepted_fixture_identity_sha256"]:
        fail("DX0 Deck accepted fixture identity differs from intent")
    deck_input = dx0_deck_execution_input(
        source_commit, host["manifest_sha256"], fixture["identity_sha256"], plan_sha
    )
    deck_input_sha = dx0_identity_sha256(deck_input)
    if (deck_input != intent["deck_execution_input"]
            or deck_input_sha != intent["deck_execution_input_sha256"]):
        fail("DX0 Deck execution-input readback differs")
    result_parent = dx0_deck_result_parent()
    if result_parent.is_symlink():
        fail("DX0_DECK_TRANSACTION_BLOCKED: retained result parent is a symlink")
    result_path = result_parent / deck_input_sha / "DX0_TRANSACTION_RESULT.json"
    if result_path.parent.is_symlink():
        fail("DX0_DECK_TRANSACTION_BLOCKED: retained result root is a symlink")
    if result_path.exists():
        retained = validate_retained_result_file(
            result_path, expected_execution_input_sha256=deck_input_sha,
            expected_plan_sha256=plan_sha,
        )
        return {"disposition": "reused", "result_sha256": sha256_file(result_path),
                "execution_input_sha256": deck_input_sha,
                "operation_nonce": retained["operation_nonce"]}

    lock_parent = dx0_deck_result_parent() / ".locks"
    lock_parent.mkdir(parents=True, exist_ok=True)
    lock = lock_parent / f"{deck_input_sha}-{plan_sha}"
    try:
        lock.mkdir(mode=0o700)
    except FileExistsError:
        fail("DX0_DECK_TRANSACTION_BLOCKED: execution single-writer outcome is unresolved")
    write_atomic(lock / "prepared-intent.json", canonical_json(intent))
    started_utc = _utc()
    environment = None
    primary_error: RuntimeError | None = None
    try:
        process_guard()
        deck_fixture_identity()
        runner = verify_runner_identity()
        if runner["launch_critical_manifest_sha256"] != RUNNER_DIGEST:
            fail("DX0 Runtime/Proton identity differs")
        before = protected_snapshot()
        environment = create_dx0_environment(
            secrets.token_hex(16), host=host, fixture=fixture,
            execution_source=source_role, deck_execution_input_sha256=deck_input_sha,
        )
        run = supervise(environment, mode=PC0_MODE if pc0 else "wa0-audio-processor-interface-admission")
        if (run.get("classification") != "scanner_completed" or run.get("blocker") is not None
                or run.get("last_in_flight_operation") is not None
                or run.get("cleanup") != {
                    "owned_descendants_zero": True, "process_group_empty": True,
                }):
            if run.get("cleanup") != {
                    "owned_descendants_zero": True, "process_group_empty": True,
            }:
                # Failed physical containment leaves the exact marker-bound
                # environment intact for diagnosis.  Retiring files beneath a
                # potentially live process would be neither cleanup nor proof.
                environment = None
            blocker = run.get("blocker") or (
                "PC0_PROCESS_CLEANUP_BLOCKED" if pc0 and run.get("cleanup") != {
                    "owned_descendants_zero": True, "process_group_empty": True,
                } else "DX0_DECK_TRANSACTION_BLOCKED")
            secondary = run.get("secondary_cleanup_blocker")
            fail(f"{blocker}: positive batch did not complete"
                 + ("; secondary=" + secondary if isinstance(secondary, str) else ""))
        audio, component, timeline = normalize_wa0_positive(
            run, _build_for_normalizer(source, host, fixture), pre_setup=pc0
        )
        retiring_environment = environment
        environment = None
        retirement = retire_environment(retiring_environment)
        process_guard()
        after = protected_snapshot()
        if after != before:
            fail("DX0 protected state differs after live positive batch")

        custody = host["custody"]
        build_receipt = host["build_receipt"]
        positive = {
            "query_result_u32_hex": audio["query"]["result_u32_hex"],
            "query_output_nonnull": audio["query"]["output_nonnull"],
            "query_tuple_consistent": audio["query"]["tuple_consistent"],
            "interface_release_reference_count": audio["release"]["reference_count"],
            "audio_processor_method_called": audio["audio_processor_method_called"],
            "fixture": "AGain VST3",
            "interface_logical_iid": audio["interface"]["logical_iid"],
            "interface_raw_windows_tuid": audio["interface"]["raw_windows_tuid"],
        }
        calls = {
            "paired_call_ledger": True,
            "started_count": component["call_attribution"]["started_count"],
            "completed_count": component["call_attribution"]["completed_count"],
            "last_in_flight_operation": component["call_attribution"]["last_in_flight_operation"],
            "query_audio_processor_count": run["call_counts"]["query_audio_processor"],
            "release_audio_processor_count": run["call_counts"]["release_audio_processor"],
            "ledger_overflowed": False,
        }
        if pc0:
            positive["processing_contract"] = audio["processing_contract"]
            calls["pc0_operation_counts"] = {op: run["call_counts"][op] for op in PC0_OPERATIONS}
            calls["ledger"] = [record for record in timeline["positive"]
                               if record.get("event") in {"call_started", "call_completed"}]
        quiescence = {
            "interface_quiescence": audio["audio_interface_quiescence"],
            "object_quiescence": component["object_quiescence"]["value"],
        }
        inherited = component["inherited_wf0_regression"]
        shutdown = {
            "component_release_reference_count": component["component_release"]["reference_count"],
            "reverse_factory_release": inherited["factory_release_order"]
            == ["release_factory_3", "release_factory_2", "release_factory_base"],
            "exit_dll": inherited["module_exit"] == {"called": True, "present": True, "result": True},
            "free_library": inherited["module_unload"] == {"attempted": True, "succeeded": True},
            "scanner_completed": run["classification"] == "scanner_completed",
            "clean_in_process_shutdown": inherited["clean_in_process_shutdown"],
        }
        cleanup = {
            "owned_descendant_count": 0,
            "process_group_empty": run["cleanup"]["process_group_empty"],
            "environment_retired": retirement["environment_retired"],
            "stage_absent": retirement["stage_absent"],
        }
        protected = {
            "pre_sha256": _protected_digest(before),
            "post_sha256": _protected_digest(after),
            "equal": before == after,
            "comparison_completed": True,
        }
        result = {
            "schema": PC0_RESULT_SCHEMA if pc0 else DX0_RESULT_SCHEMA,
            "operation_nonce": intent["operation_nonce"],
            "artifact_producer_source": custody["producer_source"],
            "deck_execution_source": source_role,
            "execution_input": {
                "identity_sha256": deck_input_sha,
                "schema": DX0_DECK_EXECUTION_INPUT_SCHEMA,
                "proof_plan_sha256": plan_sha,
                "runtime_proton_sha256": RUNNER_DIGEST,
                "source_handoff_ref": handoff["receipt"]["bundle"]["advertised_ref"],
                "detached_worktree_commit": source_commit,
                "host_artifact_manifest_sha256": host["manifest_sha256"],
                "accepted_fixture_identity_sha256": fixture["identity_sha256"],
            },
            "host_artifact": {
                "windows_build_input_sha256": build_input_sha,
                "workflow_run_id": build_receipt["workflow"]["run_id"],
                "run_attempt": build_receipt["workflow"]["run_attempt"],
                "artifact_id": custody["artifact"]["id"],
                "manifest_sha256": host["manifest_sha256"],
                "build_receipt_sha256": host["build_receipt_sha256"],
                "mac_custody_receipt_sha256": host["custody_sha256"],
            },
            "accepted_fixture": {
                "identity_sha256": fixture["identity_sha256"],
                "bundle_manifest_sha256": DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
                "module_sha256": DX0_AGAIN_MODULE_SHA256,
                "mac_store_receipt_sha256": fixture["receipt_sha256"],
                "deck_store_receipt_sha256": fixture["receipt_sha256"],
            },
            "source_handoff": {
                "bundle_sha256": handoff["receipt"]["bundle"]["sha256"],
                "receipt_sha256": handoff["receipt_sha256"],
                "advertised_ref": handoff["receipt"]["bundle"]["advertised_ref"],
                "worktree_commit": source_commit, "worktree_clean": True,
            },
            "closed_plan": {
                "plan_id": plan["plan_id"], "sha256": plan_sha,
                "expected_result": plan["expected_result"],
                "live_exercise_ceiling": 1,
            },
            "original_observation": {
                "run_id": run["run_id"], "phase_nonce": intent["phase_nonce"],
                "event_stream_sha256": sha256_bytes(canonical_json(timeline)),
                "completion_disposition": "completed", "started_utc": started_utc,
                "completed_utc": _utc(),
            },
            "positive_result": positive, "call_facts": calls,
            "quiescence": quiescence, "shutdown": shutdown,
            "cleanup": cleanup, "protected_state": protected,
            "integrity": {
                "positive_result_sha256": sha256_bytes(canonical_json(positive)),
                "call_facts_sha256": sha256_bytes(canonical_json(calls)),
                "quiescence_sha256": sha256_bytes(canonical_json(quiescence)),
                "shutdown_sha256": sha256_bytes(canonical_json(shutdown)),
                "cleanup_sha256": sha256_bytes(canonical_json(cleanup)),
                "protected_state_sha256": sha256_bytes(canonical_json(protected)),
            },
        }
        path = _publish_result(result, deck_input_sha)
        write_atomic(lock / "completion.json", canonical_json({
            "disposition": "completed", "result_sha256": sha256_file(path),
            "execution_input_sha256": deck_input_sha,
        }))
        return {"disposition": "completed", "result_sha256": sha256_file(path),
                "execution_input_sha256": deck_input_sha,
                "operation_nonce": intent["operation_nonce"]}
    except Exception as error:
        if not pc0:
            raise
        primary_error = pc0_compose_failure(error)
    finally:
        if environment is not None:
            retiring_environment = environment
            environment = None
            try:
                retire_environment(retiring_environment)
            except Exception:
                if not pc0:
                    raise
                primary_error = pc0_compose_failure(
                    primary_error, retirement_failed=True
                )
        # A completed publication makes this lock stale and safe to retire. On
        # every other path it is preserved as an unresolved-outcome marker.
        if result_path.exists() and lock.exists():
            for child in lock.iterdir():
                child.unlink()
            lock.rmdir()
    if primary_error is not None:
        raise primary_error
    fail("PC0_EVIDENCE_BLOCKED: positive batch ended without a result")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    execute_parser = sub.add_parser("execute")
    execute_parser.add_argument("--intent", type=pathlib.Path, required=True)
    args = parser.parse_args()
    result = execute(args.intent.resolve())
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"DX0_ERROR: {error}", file=sys.stderr, flush=True)
        raise

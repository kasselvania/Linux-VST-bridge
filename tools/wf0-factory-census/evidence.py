#!/usr/bin/env python3
"""Strict DX0 cached-result admission and local five-file evidence renderer."""

from __future__ import annotations

import datetime as dt
import os
import pathlib
import re
import shutil
import tempfile
from typing import Any
from common import (PC0_PACKET_SCHEMA, PC0_COST_SCHEMA, PC0_EVIDENCE_PATHS, PC0_PROOF_CLAIMS,
                    PC0_DESIGN_BLOB, PC0_DESIGN_SHA256, PC0_BASIS_COMMIT)

from common import (
    PC0_REF, PC0_PLAN_ID, PC0_RESULT_SCHEMA, PC0_MODE, PC0_OPERATIONS,
    pc0_validate_contract, pc0_validate_call_facts, dx0_closed_plan,
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_AGAIN_MODULE_SHA256,
    DX0_COST_SCHEMA, DX0_DECK_EXECUTION_INPUT_SCHEMA, DX0_EVIDENCE_PATHS,
    DX0_EVIDENCE_RENDERER_SCHEMA, DX0_PACKET_SCHEMA, DX0_PLAN_ID,
    DX0_PLAN_SCHEMA, DX0_REF, DX0_RESULT_SCHEMA, DX0_RETAINED_TRANSACTION_SCHEMA,
    DX0_WINDOWS_BUILD_INPUT_SCHEMA,
    RUNNER_DIGEST, canonical_json, dx0_complete_source, dx0_evidence_renderer,
    dx0_identity_sha256, dx0_source_role, fail,
    parse_json_no_duplicates, sha256_bytes,
    sha256_file, write_atomic,
)

RESULT_KEYS = {
    "schema", "operation_nonce", "artifact_producer_source",
    "deck_execution_source", "execution_input", "host_artifact",
    "accepted_fixture", "source_handoff", "closed_plan",
    "original_observation", "positive_result", "call_facts", "quiescence",
    "shutdown", "cleanup", "protected_state", "integrity",
}
SOURCE_KEYS = {"identity_sha256", "commit", "tree", "parent", "ref", "manifest_sha256"}
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
HEX32 = re.compile(r"[0-9a-f]{32}")


def _keys(value: Any, expected: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        fail(f"DX0 {label} key roster differs")
    return value


def _hex(value: Any, pattern: re.Pattern[str], label: str) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        fail(f"DX0 {label} is malformed")
    return value


def _positive_int(value: Any, label: str) -> int:
    if type(value) is not int or value <= 0:
        fail(f"DX0 {label} is not a positive JSON integer")
    return value


def _typed_exact(value: dict[str, Any], expected: dict[str, Any], label: str) -> None:
    if set(value) != set(expected):
        fail(f"DX0 {label} key roster differs")
    for key, expected_value in expected.items():
        observed = value[key]
        if type(observed) is not type(expected_value) or observed != expected_value:
            fail(f"DX0 {label} differs: {key}")


def _timestamp(value: Any, label: str) -> dt.datetime:
    if (not isinstance(value, str)
            or re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
                            value) is None):
        fail(f"DX0 {label} is malformed")
    try:
        return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        fail(f"DX0 {label} is invalid")


def _source(value: Any, label: str) -> dict[str, Any]:
    source = _keys(value, SOURCE_KEYS, label)
    _hex(source["identity_sha256"], HEX64, f"{label} identity")
    _hex(source["commit"], HEX40, f"{label} commit")
    _hex(source["tree"], HEX40, f"{label} tree")
    _hex(source["parent"], HEX40, f"{label} parent")
    _hex(source["manifest_sha256"], HEX64, f"{label} manifest")
    if source["ref"] not in {DX0_REF, PC0_REF}:
        fail(f"DX0 {label} ref is malformed")
    return source


def _pc0_renderer(value: Any, consumer_source: dict[str, Any]) -> dict[str, Any]:
    renderer = _keys(value, {
        "schema", "record_count", "records", "evidence_schema", "evidence_paths",
    }, "PC0 evidence renderer")
    expected_source = dx0_source_role(dx0_complete_source(consumer_source["commit"]))
    if canonical_json(consumer_source) != canonical_json(expected_source):
        fail("PC0_EVIDENCE_BLOCKED: evidence consumer Git identity differs")
    expected_renderer = dx0_evidence_renderer(
        consumer_source["commit"], plan_id=PC0_PLAN_ID
    )
    if canonical_json(renderer) != canonical_json(expected_renderer):
        fail("PC0_EVIDENCE_BLOCKED: renderer records do not belong to consumer C")
    return renderer


def validate_result(value: Any, *, expected_execution_input_sha256: str | None = None,
                    expected_plan_sha256: str | None = None) -> dict[str, Any]:
    result = _keys(value, RESULT_KEYS, "transaction result")
    pc0 = result["schema"] == PC0_RESULT_SCHEMA
    expected_plan = dx0_closed_plan(PC0_PLAN_ID if pc0 else DX0_PLAN_ID)
    if result["schema"] not in {DX0_RESULT_SCHEMA, PC0_RESULT_SCHEMA}:
        fail("DX0 transaction-result schema differs")
    _hex(result["operation_nonce"], HEX32, "operation nonce")
    _source(result["artifact_producer_source"], "artifact producer source")
    _source(result["deck_execution_source"], "Deck execution source")

    execution = _keys(result["execution_input"], {
        "identity_sha256", "schema", "proof_plan_sha256", "runtime_proton_sha256",
        "source_handoff_ref", "detached_worktree_commit",
        "host_artifact_manifest_sha256", "accepted_fixture_identity_sha256",
    }, "execution input")
    if execution["schema"] != DX0_DECK_EXECUTION_INPUT_SCHEMA:
        fail("DX0 Deck-execution-input schema differs")
    for key in ("identity_sha256", "proof_plan_sha256", "runtime_proton_sha256",
                "host_artifact_manifest_sha256", "accepted_fixture_identity_sha256"):
        _hex(execution[key], HEX64, f"execution input {key}")
    _hex(execution["detached_worktree_commit"], HEX40, "execution worktree commit")
    if execution["runtime_proton_sha256"] != RUNNER_DIGEST:
        fail("DX0 result Runtime/Proton identity differs")
    if expected_execution_input_sha256 is not None and execution["identity_sha256"] != expected_execution_input_sha256:
        fail("DX0 cached result execution-input identity differs")
    if expected_plan_sha256 is not None and execution["proof_plan_sha256"] != expected_plan_sha256:
        fail("DX0 cached result proof-plan identity differs")

    host = _keys(result["host_artifact"], {
        "windows_build_input_sha256", "workflow_run_id", "run_attempt", "artifact_id",
        "manifest_sha256", "build_receipt_sha256", "mac_custody_receipt_sha256",
    }, "host artifact")
    for key in ("workflow_run_id", "run_attempt", "artifact_id"):
        _positive_int(host[key], f"host artifact {key}")
    for key in ("windows_build_input_sha256", "manifest_sha256", "build_receipt_sha256",
                "mac_custody_receipt_sha256"):
        _hex(host[key], HEX64, f"host artifact {key}")
    if host["manifest_sha256"] != execution["host_artifact_manifest_sha256"]:
        fail("DX0 result host manifest/execution-input join differs")

    fixture = _keys(result["accepted_fixture"], {
        "identity_sha256", "bundle_manifest_sha256", "module_sha256",
        "mac_store_receipt_sha256", "deck_store_receipt_sha256",
    }, "accepted fixture")
    for key in fixture:
        _hex(fixture[key], HEX64, f"accepted fixture {key}")
    if (fixture["bundle_manifest_sha256"] != DX0_AGAIN_BUNDLE_MANIFEST_SHA256
            or fixture["module_sha256"] != DX0_AGAIN_MODULE_SHA256
            or fixture["identity_sha256"] != execution["accepted_fixture_identity_sha256"]):
        fail("DX0 accepted fixture/result join differs")

    handoff = _keys(result["source_handoff"], {
        "bundle_sha256", "receipt_sha256", "advertised_ref", "worktree_commit",
        "worktree_clean",
    }, "source handoff")
    _hex(handoff["bundle_sha256"], HEX64, "source bundle")
    _hex(handoff["receipt_sha256"], HEX64, "source handoff receipt")
    _hex(handoff["worktree_commit"], HEX40, "source handoff worktree")
    if (handoff["worktree_clean"] is not True
            or handoff["worktree_commit"] != result["deck_execution_source"]["commit"]
            or execution["detached_worktree_commit"]
            != result["deck_execution_source"]["commit"]
            or execution["detached_worktree_commit"] != handoff["worktree_commit"]
            or handoff["advertised_ref"] != execution["source_handoff_ref"]
            or handoff["advertised_ref"]
            != f"refs/handoff/dx0-source/{result['deck_execution_source']['commit']}"):
        fail("DX0 source handoff/execution-source join differs")

    plan = _keys(result["closed_plan"], {
        "plan_id", "sha256", "expected_result", "live_exercise_ceiling",
    }, "closed plan")
    _hex(plan["sha256"], HEX64, "proof plan")
    if (plan["plan_id"] != expected_plan["plan_id"]
            or plan["sha256"] != execution["proof_plan_sha256"]
            or plan["expected_result"] != expected_plan["expected_result"]
            or type(plan["live_exercise_ceiling"]) is not int
            or plan["live_exercise_ceiling"] != 1):
        fail("DX0 retained closed plan differs")

    if plan["sha256"] != dx0_identity_sha256(expected_plan):
        fail("PC0 retained plan digest differs")
    if pc0 and result["deck_execution_source"]["ref"] != PC0_REF:
        fail("PC0 original execution source branch differs")

    observation = _keys(result["original_observation"], {
        "run_id", "phase_nonce", "event_stream_sha256", "completion_disposition",
        "started_utc", "completed_utc",
    }, "original observation")
    _hex(observation["run_id"], HEX32, "Deck observation run ID")
    _hex(observation["phase_nonce"], HEX32, "Deck phase nonce")
    _hex(observation["event_stream_sha256"], HEX64, "event stream")
    started = _timestamp(observation["started_utc"], "observation start")
    completed = _timestamp(observation["completed_utc"], "observation completion")
    if (observation["completion_disposition"] != "completed"
            or completed < started):
        fail("DX0 original observation is incomplete")

    positive = _keys(result["positive_result"], {
        "query_result_u32_hex", "query_output_nonnull", "query_tuple_consistent",
        "interface_release_reference_count", "audio_processor_method_called",
        "fixture", "interface_logical_iid", "interface_raw_windows_tuid",
    } | ({"processing_contract"} if pc0 else set()), "positive result")
    if pc0:
        pc0_validate_contract(positive["processing_contract"])
    _typed_exact({key: item for key, item in positive.items() if key != "processing_contract"}, {
        "query_result_u32_hex": "00000000", "query_output_nonnull": True,
        "query_tuple_consistent": True, "interface_release_reference_count": 1,
        "audio_processor_method_called": pc0, "fixture": "AGain VST3",
        "interface_logical_iid": "42043F99B7DA453CA569E79D9AAEC33D",
        "interface_raw_windows_tuid": "993F0442DAB73C45A569E79D9AAEC33D",
    }, "cached positive result")

    calls = _keys(result["call_facts"], {
        "paired_call_ledger", "started_count", "completed_count",
        "last_in_flight_operation", "query_audio_processor_count",
        "release_audio_processor_count", "ledger_overflowed",
    } | ({"pc0_operation_counts", "ledger"} if pc0 else set()), "call facts")
    if pc0:
        pc0_validate_call_facts(calls)
    _typed_exact({key: item for key, item in calls.items() if key not in {"pc0_operation_counts", "ledger"}}, {
        "paired_call_ledger": True, "started_count": 33 if pc0 else 22, "completed_count": 33 if pc0 else 22,
        "last_in_flight_operation": None, "query_audio_processor_count": 1,
        "release_audio_processor_count": 1, "ledger_overflowed": False,
    }, "cached call ledger")
    quiescence = _keys(result["quiescence"], {"interface_quiescence", "object_quiescence"},
                       "quiescence")
    _typed_exact(quiescence,
                 {"interface_quiescence": True, "object_quiescence": True},
                 "cached quiescence")
    shutdown = _keys(result["shutdown"], {
        "component_release_reference_count", "reverse_factory_release", "exit_dll",
        "free_library", "scanner_completed", "clean_in_process_shutdown",
    }, "shutdown")
    _typed_exact(shutdown, {"component_release_reference_count": 0,
                            "reverse_factory_release": True, "exit_dll": True,
                            "free_library": True, "scanner_completed": True,
                            "clean_in_process_shutdown": True},
                 "cached in-process shutdown")
    cleanup = _keys(result["cleanup"], {
        "owned_descendant_count", "process_group_empty", "environment_retired", "stage_absent",
    }, "cleanup")
    _typed_exact(cleanup, {"owned_descendant_count": 0, "process_group_empty": True,
                           "environment_retired": True, "stage_absent": True},
                 "cached physical cleanup")
    protected = _keys(result["protected_state"], {
        "pre_sha256", "post_sha256", "equal", "comparison_completed",
    }, "protected state")
    _hex(protected["pre_sha256"], HEX64, "protected pre-state")
    _hex(protected["post_sha256"], HEX64, "protected post-state")
    if protected["equal"] is not True or protected["comparison_completed"] is not True \
            or protected["pre_sha256"] != protected["post_sha256"]:
        fail("DX0 cached protected-state comparison is incomplete")

    integrity = _keys(result["integrity"], {
        "positive_result_sha256", "call_facts_sha256", "quiescence_sha256",
        "shutdown_sha256", "cleanup_sha256", "protected_state_sha256",
    }, "integrity")
    projections = {
        "positive_result_sha256": positive, "call_facts_sha256": calls,
        "quiescence_sha256": quiescence, "shutdown_sha256": shutdown,
        "cleanup_sha256": cleanup, "protected_state_sha256": protected,
    }
    for key, projection in projections.items():
        _hex(integrity[key], HEX64, f"integrity {key}")
        if integrity[key] != sha256_bytes(canonical_json(projection)):
            fail(f"DX0 cached result integrity differs: {key}")
    return result


def validate_result_file(path: pathlib.Path, *, expected_execution_input_sha256: str | None = None,
                         expected_plan_sha256: str | None = None) -> dict[str, Any]:
    if not path.is_file() or path.is_symlink() or path.stat().st_size > 2 * 1024 * 1024:
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
    return validate_result(value, expected_execution_input_sha256=expected_execution_input_sha256,
                           expected_plan_sha256=expected_plan_sha256)


def result_admission_receipt(result: dict[str, Any], *, consumer_source: dict[str, Any],
                             renderer: dict[str, Any], original_observation: bool = False) -> dict[str, Any]:
    validate_result(result)
    if renderer.get("schema") != DX0_EVIDENCE_RENDERER_SCHEMA:
        fail("DX0 evidence renderer identity differs")
    if result["schema"] == PC0_RESULT_SCHEMA:
        _source(consumer_source, "evidence consumer source")
        _pc0_renderer(renderer, consumer_source)
        if original_observation and consumer_source != result["deck_execution_source"]:
            fail("PC0_EVIDENCE_BLOCKED: renderer or observation disposition differs")
        return {
            "schema": "linux-vst-bridge-dx0-result-admission/v1",
            "private_result_schema": PC0_RESULT_SCHEMA,
            "operation_nonce": result["operation_nonce"],
            "retained_result_sha256": sha256_bytes(canonical_json(result)),
            "execution_input_sha256": result["execution_input"]["identity_sha256"],
            "plan_sha256": result["closed_plan"]["sha256"],
            "predicate_result": "accepted",
            "disposition": "original_observation" if original_observation else "reused_original_observation",
            "fresh_deck_execution_for_consumer": original_observation,
            "current_deck_state_inspected": original_observation,
        }
    return {
        "schema": "linux-vst-bridge-dx0-result-admission/v1",
        "retained_result_sha256": sha256_bytes(canonical_json(result)),
        "artifact_producer_source": result["artifact_producer_source"],
        "deck_execution_source": result["deck_execution_source"],
        "evidence_consumer_source": consumer_source,
        "evidence_renderer_identity_sha256": dx0_identity_sha256(renderer),
        "disposition": "reused_original_observation",
        "predicate_result": "accepted",
        "fresh_deck_execution_for_consumer": False,
        "current_deck_state_inspected": False,
    }


def _markdown(title: str, paragraphs: list[str]) -> bytes:
    return (f"# {title}\n\n" + "\n\n".join(paragraphs) + "\n").encode("utf-8")


def _render_packet_into(output: pathlib.Path,
                        transaction: dict[str, Any]) -> dict[str, Any]:
    if output.exists() or output.is_symlink():
        fail("DX0 evidence staging output already exists")
    result = validate_result(transaction.get("transaction_result"))
    if result["schema"] == PC0_RESULT_SCHEMA:
        return _render_pc0_packet(output, transaction, result)
    rows = transaction.get("proof_rows")
    costs = transaction.get("costs")
    consumer = transaction.get("evidence_consumer_source")
    renderer = transaction.get("evidence_renderer")
    admission = transaction.get("result_admission")
    if (not isinstance(rows, list) or len(rows) != 14
            or [row.get("row") for row in rows] != list(range(1, 15))
            or any(row.get("result") != "PASS" for row in rows)
            or not isinstance(costs, dict)
            or not isinstance(consumer, dict)
            or not isinstance(renderer, dict)
            or admission != result_admission_receipt(result, consumer_source=consumer,
                                                      renderer=renderer)):
        fail("DX0 evidence transaction/proof closure differs")
    output.mkdir(parents=True)
    basis = _markdown("DX0 basis", [
        "DX0 implements the approved split build-identity and single-command proof transaction.",
        f"The retained producer source is `{result['artifact_producer_source']['commit']}`; "
        f"the original Deck execution source is `{result['deck_execution_source']['commit']}`; "
        f"the evidence consumer source is `{consumer['commit']}`. These roles are intentionally distinct.",
        f"The accepted closed plan is `{DX0_PLAN_ID}`. The retained result SHA-256 is "
        f"`{admission['retained_result_sha256']}`.",
        "The current consumer reused the original observation; this packet does not claim that the Deck "
        "executed the consumer source or that current Deck state was freshly inspected.",
    ])
    write_atomic(output / "BASIS.md", basis)

    retained = {
        "schema": DX0_RETAINED_TRANSACTION_SCHEMA,
        "packet_schema": DX0_PACKET_SCHEMA,
        "proof_plan": transaction["proof_plan"],
        "artifact_producer_source": result["artifact_producer_source"],
        "deck_execution_source": result["deck_execution_source"],
        "evidence_consumer_source": consumer,
        "windows_build_input": transaction["windows_build_input"],
        "accepted_fixture": result["accepted_fixture"],
        "deck_execution_input": result["execution_input"],
        "evidence_renderer": renderer,
        "result_admission": admission,
        "host_artifact": result["host_artifact"],
        "source_handoff": result["source_handoff"],
        "original_observation": result["original_observation"],
        "positive_result": result["positive_result"],
        "call_facts": result["call_facts"],
        "quiescence": result["quiescence"],
        "shutdown": result["shutdown"],
        "cleanup": result["cleanup"],
        "protected_state": result["protected_state"],
        "phase_dispositions": transaction["phase_dispositions"],
        "proof_row_count": 14,
        "proof_rows": rows,
        "no_deck_github_operations": True,
        "implementation_capability_added": False,
    }
    write_atomic(output / "TRANSACTION.json", canonical_json(retained))

    cost_value = {
        "schema": DX0_COST_SCHEMA,
        "packet_schema": DX0_PACKET_SCHEMA,
        "static_wa0_baseline": {
            "manual_commands": 14,
            "manually_copied_identifiers": 9,
            "classification": "static_baseline_not_measured_dx0_savings",
        },
        "actual_transaction": costs,
        "one_time_fixture_seeding": transaction["fixture_seeding"],
        "invalidation": transaction["invalidation_results"],
        "renderer_only_reuse": {
            "windows_builds": 0, "custody_operations": 0,
            "artifact_transfers": 0, "source_transfers": 0,
            "deck_executions": 0, "evidence_renders": 1,
            "observation_source": result["deck_execution_source"]["commit"],
            "consumer_source": consumer["commit"],
        },
        "duration_or_monetary_savings_claimed": False,
    }
    write_atomic(output / "COST_AND_INVALIDATION.json", canonical_json(cost_value))
    findings = _markdown("DX0 findings", [
        "The exact Windows build-input identity is independent from complete source identity. "
        "The accepted AGain fixture was reused and was not compiled by the host-only producer.",
        "One repository-owned command derived all identities and carried every required identifier. "
        "No identifier was manually copied into a custody, handoff, Deck, or evidence command.",
        "The retained positive observation proves the inherited WA0 interface lease, quiescence, clean "
        "shutdown, zero-descendant cleanup, environment retirement, and protected-state equality.",
        "Synthetic deterministic cases prove invalidation, malformed-result rejection, lost-ack recovery, "
        "and single-writer behavior. They are not represented as live Runtime/Proton exercises.",
        "DX0 adds no VST3 capability and makes no immutable-runner, signing, release, audio, GUI, IPC, "
        "Bitwig, Serum, or general-compatibility claim.",
    ])
    write_atomic(output / "FINDINGS.md", findings)
    members = ["BASIS.md", "COST_AND_INVALIDATION.json", "FINDINGS.md", "TRANSACTION.json"]
    hashes = "".join(f"{sha256_file(output / name)}  {name}\n" for name in members)
    write_atomic(output / "hashes.sha256", hashes.encode())
    validate_packet(output)
    return packet_identity(output)


def render_packet(output: pathlib.Path, transaction: dict[str, Any], *,
                  staging_parent: pathlib.Path | None = None) -> dict[str, Any]:
    """Render off-worktree, then atomically publish or admit an exact packet."""
    if output.is_symlink() or output.parent.is_symlink():
        fail("DX0 evidence output boundary is a symlink")
    output.parent.mkdir(parents=True, exist_ok=True)
    parent = staging_parent or output.parent
    if not parent.is_dir() or parent.is_symlink():
        fail("DX0 evidence staging parent is absent or unsafe")
    if os.stat(parent).st_dev != os.stat(output.parent).st_dev:
        fail("DX0 evidence staging parent is not on the publication filesystem")
    stage = pathlib.Path(tempfile.mkdtemp(prefix=".dx0-evidence-render-", dir=parent))
    try:
        # mkdtemp reserves the name; the inner renderer owns initial creation.
        stage.rmdir()
        rendered = _render_packet_into(stage, transaction)
        validate_packet(stage)
        if output.exists():
            if not output.is_dir():
                fail("DX0 evidence output exists as a non-directory")
            validate_packet(output)
            if any((output / path.name).read_bytes() != path.read_bytes()
                   for path in stage.iterdir()):
                fail("DX0 existing evidence packet differs from exact rerender")
            return rendered
        os.rename(stage, output)
        validate_packet(output)
        return rendered
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def packet_identity(root: pathlib.Path) -> dict[str, Any]:
    validate_packet(root)
    retained = parse_json_no_duplicates((root / "TRANSACTION.json").read_bytes(), "packet")
    return {"schema": PC0_PACKET_SCHEMA if retained.get("schema") == PC0_PACKET_SCHEMA else DX0_PACKET_SCHEMA, "record_count": 5,
            "packet_sha256": sha256_bytes(canonical_json([
                {"path": name, "sha256": sha256_file(root / name)}
                for name in sorted(path.name for path in root.iterdir())
            ]))}


def publish_packet(source: pathlib.Path, output: pathlib.Path, *,
                   staging_parent: pathlib.Path) -> dict[str, Any]:
    """Atomically deliver an already-hashed private packet to the worktree."""
    identity = packet_identity(source)
    if (output.is_symlink() or output.parent.is_symlink()
            or not staging_parent.is_dir() or staging_parent.is_symlink()):
        fail("DX0 evidence publication boundary is unsafe")
    output.parent.mkdir(parents=True, exist_ok=True)
    if os.stat(staging_parent).st_dev != os.stat(output.parent).st_dev:
        fail("DX0 evidence publication stage is on another filesystem")
    if output.exists():
        if not output.is_dir():
            fail("DX0 evidence output exists as a non-directory")
        if packet_identity(output) != identity or any(
                (output / path.name).read_bytes() != path.read_bytes()
                for path in source.iterdir()):
            fail("DX0 existing evidence packet differs from retained packet")
        return identity
    stage = pathlib.Path(tempfile.mkdtemp(
        prefix=".dx0-evidence-publish-", dir=staging_parent
    ))
    try:
        for path in source.iterdir():
            destination = stage / path.name
            shutil.copyfile(path, destination)
            destination.chmod(0o644)
        validate_packet(stage)
        os.rename(stage, output)
        if packet_identity(output) != identity:
            fail("DX0 published evidence packet identity differs")
        return identity
    finally:
        if stage.exists():
            shutil.rmtree(stage)


def validate_packet(root: pathlib.Path) -> None:
    expected_names = {pathlib.PurePosixPath(path).name for path in DX0_EVIDENCE_PATHS}
    if (not root.is_dir() or root.is_symlink()
            or {path.name for path in root.iterdir()} != expected_names):
        fail("DX0 five-file evidence roster differs")
    expected_hashes = "".join(
        f"{sha256_file(root / name)}  {name}\n"
        for name in ["BASIS.md", "COST_AND_INVALIDATION.json", "FINDINGS.md", "TRANSACTION.json"]
    ).encode()
    if (root / "hashes.sha256").read_bytes() != expected_hashes:
        fail("DX0 evidence hash closure differs")
    prohibited = re.compile(
        rb"(?:/Users/|/home/|BEGIN (?:RSA |OPENSSH )?PRIVATE KEY|github_pat_|ghp_|"
        rb"(?:^|[^A-Za-z])PID(?:[^A-Za-z]|$)|(?:[0-9]{1,3}\.){3}[0-9]{1,3})",
        re.IGNORECASE,
    )
    for path in root.iterdir():
        if not path.is_file() or path.is_symlink() or path.stat().st_size > 2 * 1024 * 1024:
            fail(f"DX0 evidence file is absent, unsafe, or oversized: {path.name}")
        data = path.read_bytes()
        if b"\0" in data or prohibited.search(data):
            fail(f"DX0 evidence contains private/binary data: {path.name}")
        data.decode("utf-8", "strict")
    transaction = parse_json_no_duplicates(
        (root / "TRANSACTION.json").read_bytes(), "DX0 retained transaction"
    )
    cost = parse_json_no_duplicates(
        (root / "COST_AND_INVALIDATION.json").read_bytes(), "DX0 cost ledger"
    )
    if transaction.get("schema") == PC0_PACKET_SCHEMA:
        if (canonical_json(transaction) != (root / "TRANSACTION.json").read_bytes()
                or canonical_json(cost) != (root / "COST_AND_INVALIDATION.json").read_bytes()):
            fail("PC0_EVIDENCE_BLOCKED: noncanonical packet")
        validate_pc0_packet_value(transaction, cost)
        return
    if (canonical_json(transaction) != (root / "TRANSACTION.json").read_bytes()
            or transaction.get("schema") != DX0_RETAINED_TRANSACTION_SCHEMA
            or transaction.get("proof_row_count") != 14
            or canonical_json(cost) != (root / "COST_AND_INVALIDATION.json").read_bytes()
            or cost.get("schema") != DX0_COST_SCHEMA):
        fail("DX0 evidence JSON/schema differs")


PC0_PACKET_KEYS = {
    "schema", "artifact_producer_source", "deck_execution_source", "evidence_consumer_source",
    "observation_disposition", "consumer_executed_on_deck", "consumer_deck_state_freshly_inspected",
    "admitted_private_result_sha256", "result_admission", "windows_build_input", "deck_execution_input",
    "host_artifact", "accepted_fixture", "source_handoff", "closed_plan", "original_observation",
    "processing_contract", "call_facts", "quiescence", "shutdown", "cleanup", "protected_state",
    "proof_rows", "renderer", "integrity",
}
PC0_COST_KEYS = {"schema", "external_effect_counts", "phase_dispositions", "invalidation_cases",
                 "renderer_only_reuse", "ordinary_driver_command_count", "manually_copied_identifier_count",
                 "fixture_accounting"}


def _pc0_integrity(value: dict[str, Any]) -> dict[str, str]:
    projections = {key: value[key] for key in (
        "processing_contract", "call_facts", "protected_state", "proof_rows", "renderer")}
    projections["lifecycle_closure"] = {key: value[key] for key in ("quiescence", "shutdown", "cleanup")}
    return {"admitted_private_result_sha256": value["admitted_private_result_sha256"],
            **{key + "_sha256": sha256_bytes(canonical_json(projection))
               for key, projection in projections.items()}}


def validate_pc0_packet_value(packet: dict[str, Any], cost: dict[str, Any]) -> None:
    _keys(packet, PC0_PACKET_KEYS, "PC0 evidence packet")
    _keys(cost, PC0_COST_KEYS, "PC0 cost ledger")
    if packet["schema"] != PC0_PACKET_SCHEMA or cost["schema"] != PC0_COST_SCHEMA:
        fail("PC0_EVIDENCE_BLOCKED: evidence schemas differ")
    for key in ("artifact_producer_source", "deck_execution_source", "evidence_consumer_source"):
        _source(packet[key], key)
    _pc0_renderer(packet["renderer"], packet["evidence_consumer_source"])
    pc0_validate_contract(packet["processing_contract"])
    pc0_validate_call_facts(packet["call_facts"])
    if packet["integrity"] != _pc0_integrity(packet):
        fail("PC0_EVIDENCE_BLOCKED: nested projection integrity differs")
    _hex(packet["admitted_private_result_sha256"], HEX64, "private observation")
    original = packet["observation_disposition"] == "original_observation"
    if (packet["observation_disposition"] not in {"original_observation", "reused_original_observation"}
            or packet["consumer_executed_on_deck"] is not original
            or packet["consumer_deck_state_freshly_inspected"] is not original
            or (original and packet["deck_execution_source"] != packet["evidence_consumer_source"])):
        fail("PC0_EVIDENCE_BLOCKED: P/E/C observation claim differs")
    expected_admission = {
        "schema": "linux-vst-bridge-dx0-result-admission/v1", "private_result_schema": PC0_RESULT_SCHEMA,
        "operation_nonce": packet["result_admission"].get("operation_nonce"),
        "retained_result_sha256": packet["admitted_private_result_sha256"],
        "execution_input_sha256": packet["deck_execution_input"]["identity_sha256"],
        "plan_sha256": packet["closed_plan"]["sha256"], "predicate_result": "accepted",
        "disposition": packet["observation_disposition"],
        "fresh_deck_execution_for_consumer": original, "current_deck_state_inspected": original,
    }
    _hex(expected_admission["operation_nonce"], HEX32, "admitted operation nonce")
    if canonical_json(packet["result_admission"]) != canonical_json(expected_admission):
        fail("PC0_EVIDENCE_BLOCKED: strict admission receipt differs")
    # Reconstruct and re-admit the exact private P/E object named by the packet.
    # The operation nonce is retained in the admission receipt; positive query
    # and release facts come from the already-validated closed call ledger.
    ledger = packet["call_facts"]["ledger"]
    query_start, query_completed = ledger[26], ledger[27]
    release_completed = ledger[51]
    private = {key: packet[key] for key in (
        "artifact_producer_source", "deck_execution_source", "host_artifact",
        "accepted_fixture", "source_handoff", "closed_plan", "original_observation",
        "call_facts", "quiescence", "shutdown", "cleanup", "protected_state",
    )}
    private.update(schema=PC0_RESULT_SCHEMA,
                   operation_nonce=expected_admission["operation_nonce"],
                   execution_input=packet["deck_execution_input"],
                   positive_result={
                       "query_result_u32_hex": query_completed["result_u32_hex"],
                       "query_output_nonnull": query_completed["output_nonnull"],
                       "query_tuple_consistent": (
                           query_completed["result_u32_hex"] == "00000000"
                           and query_completed["output_nonnull"] is True),
                       "interface_release_reference_count": release_completed["u32_result"],
                       "audio_processor_method_called": any(
                           value > 0 for value in packet["call_facts"]["pc0_operation_counts"].values()),
                       "fixture": "AGain VST3",
                       "interface_logical_iid": "42043F99B7DA453CA569E79D9AAEC33D",
                       "interface_raw_windows_tuid": query_start["requested_iid_raw_tuid_hex"],
                       "processing_contract": packet["processing_contract"],
                   })
    private["integrity"] = {key + "_sha256": sha256_bytes(canonical_json(private[key]))
                            for key in ("positive_result", "call_facts", "quiescence", "shutdown", "cleanup", "protected_state")}
    validate_result(private)
    if sha256_bytes(canonical_json(private)) != packet["admitted_private_result_sha256"]:
        fail("PC0_EVIDENCE_BLOCKED: packet does not reconstruct the admitted private result")
    build = _keys(packet["windows_build_input"], {"schema", "sha256", "record_count"},
                  "Windows build input projection")
    if (build["schema"] != DX0_WINDOWS_BUILD_INPUT_SCHEMA
            or type(build["record_count"]) is not int or build["record_count"] != 17
            or build["sha256"] != packet["host_artifact"]["windows_build_input_sha256"]
            or packet["renderer"]["schema"] != DX0_EVIDENCE_RENDERER_SCHEMA
            or packet["renderer"]["record_count"] != 2
            or packet["renderer"]["evidence_schema"] != PC0_PACKET_SCHEMA
            or packet["renderer"]["evidence_paths"] != list(PC0_EVIDENCE_PATHS)):
        fail("PC0_EVIDENCE_BLOCKED: build/renderer joins differ")
    rows = packet["proof_rows"]
    synthetic = {1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15}
    if (type(rows) is not list or len(rows) != 16
            or any(set(row) != {"row", "result", "validation", "claim"}
                   or type(row["row"]) is not int or row["row"] != index
                   or row["result"] != "PASS" or row["claim"] != PC0_PROOF_CLAIMS[index - 1]
                   or row["validation"] != ("deterministic" if index in synthetic else "live")
                   for index, row in enumerate(rows, 1))):
        fail("PC0_EVIDENCE_BLOCKED: sixteen proof dispositions required")
    effects = cost["external_effect_counts"]
    effect_keys = {"windows_builds", "artifact_downloads", "custody_operations",
                   "artifact_transfers", "source_transfers", "deck_executions", "evidence_renders"}
    phase_keys = {"derive_identities", "plan_external_work", "freeze_source", "verify_fixture",
                  "reuse_or_produce_host", "custody_host_artifact", "create_source_handoff",
                  "transfer_and_admit_deck_inputs", "execute_deck_batch",
                  "retrieve_and_retain_result", "render_and_validate_evidence", "close_transaction"}
    invalidation_keys = {"synthetic_git_mutations", "renderer_only_external_effects",
        "deck_import_closure", "result_validator_parity_cases", "production_owner_cases",
        "missing_completions_rejected", "writer_boundary_passed", "renderer_only",
        "mac_only", "deck_only", "host_only", "lost_ack_and_duplicate_work"}
    reuse_keys = {"windows_builds", "artifact_downloads", "custody_operations",
                  "artifact_transfers", "source_transfers", "deck_executions", "evidence_renders",
                  "actual_retained_observation_reused", "consumer_mutation", "observation_source"}
    phases = cost["phase_dispositions"]
    expected_fixed_phases = {
        "derive_identities": "completed",
        "plan_external_work": "completed",
        "freeze_source": "completed",
        "verify_fixture": "reused",
        "render_and_validate_evidence": "completed",
        "close_transaction": "completed",
    }
    effect_phase = {
        "windows_builds": "reuse_or_produce_host",
        "artifact_downloads": "custody_host_artifact",
        "custody_operations": "custody_host_artifact",
        "source_transfers": "create_source_handoff",
        "artifact_transfers": "transfer_and_admit_deck_inputs",
        "deck_executions": "execute_deck_batch",
    }
    if (type(effects) is not dict or set(effects) != effect_keys
            or any(type(n) is not int or n < 0 for n in effects.values())
            or any(value > 1 for value in effects.values())
            or effects.get("evidence_renders") != 1
            or type(phases) is not dict
            or set(phases) != phase_keys
            or any(value not in {"completed", "reused"} for value in phases.values())
            or any(phases.get(name) != disposition
                   for name, disposition in expected_fixed_phases.items())
            or any(effects[key] != (1 if phases.get(phase) == "completed" else 0)
                   for key, phase in effect_phase.items())
            or phases.get("retrieve_and_retain_result") != (
                "completed" if packet["deck_execution_source"]
                == packet["evidence_consumer_source"] else "reused")
            or type(cost["invalidation_cases"]) is not dict
            or set(cost["invalidation_cases"]) != invalidation_keys
            or cost["invalidation_cases"].get("synthetic_git_mutations") is not True
            or cost["invalidation_cases"].get("renderer_only_external_effects") != 0
            or cost["invalidation_cases"].get("deck_import_closure") != [
                "tools/wf0-factory-census/artifacts.py",
                "tools/wf0-factory-census/common.py",
                "tools/wf0-factory-census/environment.py",
                "tools/wf0-factory-census/normalize.py",
                "tools/wf0-factory-census/run.py",
                "tools/wf0-factory-census/supervise.py",
                "tools/wr0-proton-bootstrap/launch.py",
            ]
            or cost["invalidation_cases"].get("result_validator_parity_cases") != 16
            or cost["invalidation_cases"].get("production_owner_cases") != 22
            or cost["invalidation_cases"].get("missing_completions_rejected") != 11
            or cost["invalidation_cases"].get("writer_boundary_passed") is not True
            or cost["invalidation_cases"].get("renderer_only") != {
                "build": False, "deck": False, "renderer": True}
            or cost["invalidation_cases"].get("mac_only") != {"build": False, "deck": False}
            or cost["invalidation_cases"].get("deck_only") != {"build": False, "deck": True}
            or cost["invalidation_cases"].get("host_only") != {"build": True}
            or cost["invalidation_cases"].get("lost_ack_and_duplicate_work") != 0
            or set(cost["renderer_only_reuse"]) != reuse_keys
            or cost["renderer_only_reuse"].get("actual_retained_observation_reused") is not True
            or cost["renderer_only_reuse"].get("consumer_mutation") != "synthetic_renderer_only_Git_revision"
            or cost["renderer_only_reuse"].get("observation_source") != packet["deck_execution_source"]["commit"]
            or cost["renderer_only_reuse"].get("evidence_renders") != 1
            or cost["ordinary_driver_command_count"] != 1 or type(cost["ordinary_driver_command_count"]) is not int
            or cost["manually_copied_identifier_count"] != 0
            or canonical_json(cost["fixture_accounting"]) != canonical_json({"again_builds": 0, "fixture_seeds": 0})
            or any(cost["renderer_only_reuse"].get(key) != 0 for key in (
                "windows_builds", "artifact_downloads", "custody_operations", "artifact_transfers", "source_transfers", "deck_executions"))):
        fail("PC0_EVIDENCE_BLOCKED: cost ceiling/reuse differs")


def _render_pc0_packet(output: pathlib.Path, transaction: dict[str, Any], result: dict[str, Any]) -> dict[str, Any]:
    admission = transaction["result_admission"]
    consumer = transaction["evidence_consumer_source"]
    renderer = transaction["evidence_renderer"]
    original = admission.get("disposition") == "original_observation"
    if admission != result_admission_receipt(result, consumer_source=consumer, renderer=renderer,
                                              original_observation=original):
        fail("PC0_EVIDENCE_BLOCKED: admission differs before render")
    packet = {key: result[key] for key in (
        "artifact_producer_source", "deck_execution_source", "host_artifact", "accepted_fixture", "source_handoff",
        "closed_plan", "original_observation", "call_facts", "quiescence", "shutdown", "cleanup", "protected_state")}
    packet.update({
        "schema": PC0_PACKET_SCHEMA, "evidence_consumer_source": consumer,
        "observation_disposition": admission["disposition"],
        "consumer_executed_on_deck": original, "consumer_deck_state_freshly_inspected": original,
        "admitted_private_result_sha256": admission["retained_result_sha256"], "result_admission": admission,
        "windows_build_input": transaction["windows_build_input"], "deck_execution_input": result["execution_input"],
        "processing_contract": result["positive_result"]["processing_contract"],
        "proof_rows": transaction["proof_rows"], "renderer": renderer,
    })
    packet["integrity"] = _pc0_integrity(packet)
    costs = transaction["costs"]
    cost = {
        "schema": PC0_COST_SCHEMA,
        "external_effect_counts": {key: costs[key] for key in ("windows_builds", "artifact_downloads", "custody_operations",
            "artifact_transfers", "source_transfers", "deck_executions", "evidence_renders")},
        "phase_dispositions": transaction["phase_dispositions"],
        "invalidation_cases": transaction["invalidation_results"],
        "renderer_only_reuse": transaction["renderer_only_reuse"],
        "ordinary_driver_command_count": costs["manual_commands"], "manually_copied_identifier_count": costs["manually_copied_identifiers"],
        "fixture_accounting": {"again_builds": 0, "fixture_seeds": 0},
    }
    validate_pc0_packet_value(packet, cost)
    output.mkdir(parents=True)
    write_atomic(output / "TRANSACTION.json", canonical_json(packet))
    write_atomic(output / "COST_AND_INVALIDATION.json", canonical_json(cost))
    write_atomic(output / "BASIS.md", _markdown("PC0 basis", [
        f"Approved pc0-design-v2: blob `{PC0_DESIGN_BLOB}`, SHA-256 `{PC0_DESIGN_SHA256}`; basis `{PC0_BASIS_COMMIT}`.",
        f"Producer P `{packet['artifact_producer_source']['commit']}`; original execution E `{packet['deck_execution_source']['commit']}`; consumer C `{consumer['commit']}`.",
        f"Observation disposition: `{admission['disposition']}`. Original private result: `{admission['retained_result_sha256']}`.",
        "One Initialized-state read-only census; accepted AGain, interface ownership and infrastructure reused.",
    ]))
    write_atomic(output / "FINDINGS.md", _markdown("PC0 findings", [
        "AGain reports Stereo In and Stereo Out, two channels each, kStereo; Event In, one channel; no event output. All three buses are main/default-active, not control-voltage. Both sample sizes are supported.",
        "Eleven PC0 calls extend the accepted lifecycle to 33 paired calls. Interface release returns 1; component release returns 0. Quiescence, factory/module shutdown, zero descendants, environment retirement and the original protected-state equality are retained machine-readably in TRANSACTION.json.",
        "Deterministic failures use the production borrower and injected event streams, not live plug-ins. One positive Deck batch owns the original observation. A later renderer consumes that historical result and does not claim fresh Deck inspection.",
        "No setup, latency, tail, activation, processing, audio/event buffers, controller, state, parameter, IPC, proxy, Bitwig, Serum, packaging, signing, runner-selection or general compatibility claim.",
    ]))
    names = ("BASIS.md", "COST_AND_INVALIDATION.json", "FINDINGS.md", "TRANSACTION.json")
    write_atomic(output / "hashes.sha256", "".join(f"{sha256_file(output / name)}  {name}\n" for name in names).encode())
    return packet_identity(output)


if __name__ == "__main__":
    raise SystemExit("evidence.py is a library; use tools/host-proof.py")

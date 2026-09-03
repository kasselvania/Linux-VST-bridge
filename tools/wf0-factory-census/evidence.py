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

from common import (
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_AGAIN_MODULE_SHA256,
    DX0_COST_SCHEMA, DX0_DECK_EXECUTION_INPUT_SCHEMA, DX0_EVIDENCE_PATHS,
    DX0_EVIDENCE_RENDERER_SCHEMA, DX0_PACKET_SCHEMA, DX0_PLAN_ID,
    DX0_PLAN_SCHEMA, DX0_REF, DX0_RESULT_SCHEMA, DX0_RETAINED_TRANSACTION_SCHEMA,
    RUNNER_DIGEST, canonical_json, dx0_identity_sha256, fail,
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
    if source["ref"] != DX0_REF:
        fail(f"DX0 {label} ref is malformed")
    return source


def validate_result(value: Any, *, expected_execution_input_sha256: str | None = None,
                    expected_plan_sha256: str | None = None) -> dict[str, Any]:
    result = _keys(value, RESULT_KEYS, "transaction result")
    if result["schema"] != DX0_RESULT_SCHEMA:
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
    if (plan["plan_id"] != DX0_PLAN_ID
            or plan["sha256"] != execution["proof_plan_sha256"]
            or plan["expected_result"] != "wa0-positive-interface-lease-complete-v1"
            or type(plan["live_exercise_ceiling"]) is not int
            or plan["live_exercise_ceiling"] != 1):
        fail("DX0 retained closed plan differs")

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
    }, "positive result")
    _typed_exact(positive, {
        "query_result_u32_hex": "00000000", "query_output_nonnull": True,
        "query_tuple_consistent": True, "interface_release_reference_count": 1,
        "audio_processor_method_called": False, "fixture": "AGain VST3",
        "interface_logical_iid": "42043F99B7DA453CA569E79D9AAEC33D",
        "interface_raw_windows_tuid": "993F0442DAB73C45A569E79D9AAEC33D",
    }, "cached positive result")

    calls = _keys(result["call_facts"], {
        "paired_call_ledger", "started_count", "completed_count",
        "last_in_flight_operation", "query_audio_processor_count",
        "release_audio_processor_count", "ledger_overflowed",
    }, "call facts")
    _typed_exact(calls, {
        "paired_call_ledger": True, "started_count": 22, "completed_count": 22,
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
                             renderer: dict[str, Any]) -> dict[str, Any]:
    validate_result(result)
    if renderer.get("schema") != DX0_EVIDENCE_RENDERER_SCHEMA:
        fail("DX0 evidence renderer identity differs")
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
    return {"schema": DX0_PACKET_SCHEMA, "record_count": 5,
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
    if (canonical_json(transaction) != (root / "TRANSACTION.json").read_bytes()
            or transaction.get("schema") != DX0_RETAINED_TRANSACTION_SCHEMA
            or transaction.get("proof_row_count") != 14
            or canonical_json(cost) != (root / "COST_AND_INVALIDATION.json").read_bytes()
            or cost.get("schema") != DX0_COST_SCHEMA):
        fail("DX0 evidence JSON/schema differs")


if __name__ == "__main__":
    raise SystemExit("evidence.py is a library; use tools/host-proof.py")

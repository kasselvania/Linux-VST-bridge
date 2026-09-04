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
import tempfile
from typing import Any

sys.dont_write_bytecode = True

from artifacts import (
    read_canonical_json, verify_fixture_store, verify_hash_sidecar,
    verify_host_store, verify_source_handoff,
)
from common import (
    PC0_BLOCKED_OUTCOMES, PC0_BRANCH, PC0_COMPLETE_SOURCE_SCHEMA, PC0_REF,
    PC0_PLAN_ID, PC0_RESULT_SCHEMA, PC0_MODE, PC0_OPERATIONS, PC0_SOURCE_PATHS,
    pc0_validate_contract, pc0_validate_call_facts, dx0_closed_plan,
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_AGAIN_MODULE_SHA256,
    DX0_DECK_EXECUTION_INPUT_SCHEMA, DX0_PLAN_ID, DX0_REF, DX0_RESULT_SCHEMA,
    DX0_SOURCE_HANDOFF_SCHEMA, REPOSITORY, RUNNER_DIGEST, canonical_json,
    command, command_text, deck_fixture_identity,
    dx0_closed_plan, dx0_complete_source, dx0_complete_source_sha256,
    dx0_deck_execution_input, dx0_windows_build_input,
    dx0_deck_fixture_parent, dx0_deck_host_artifact_parent,
    dx0_deck_result_parent, dx0_deck_source_parent, dx0_identity_sha256,
    dx0_records, dx0_require_frozen_source, dx0_source_role, fail,
    parse_json_no_duplicates,
    process_guard, protected_snapshot, repo_root, sha256_bytes, sha256_file,
    verify_runner_identity, write_atomic,
)
from environment import create_dx0_environment, retire_environment
from normalize import normalize_wa0_positive, sanitized_timeline
from supervise import EXIT_BLOCKER, IN_FLIGHT_BLOCKER, supervise


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

PC0_V3_AUTHORITY_COMMIT = "7ed1fbf985b5bb717883e0cd2132620b960e3aa6"
PC0_V3_AUTHORITY_TREE = "07efd2d95c7f72205ac5c201361c4291bb8eaf6d"
PC0_V3_DESIGN_COMMIT = "c2349780f9ed1aa6077b118be000cbab5aba698a"
PC0_V3_DESIGN_BLOB = "b991e204681e56869a0977cad091c7de7345cbeb"
PC0_V3_DESIGN_SHA256 = "4dcdce46f5f7d478fe2687c3d685418c4dd4b940804cb7a6744b890a6acff3cc"
PC0_V3_REVIEW_ID = 5108043079
PC0_V3_APPROVAL_BLOB = "32cec9c63d908687cf5e8071656413db48c049cb"
PC0_V3_REVIEW_HISTORY_BLOB = "3498f6e0cf342c33c7c5023a4c193ab3c9dc7c79"
PC0_V3_CURRENT_SLICE_BLOB = "97e57738235a85a2d86559719adfe30d653a4cd2"
PC0_V3_FROZEN_IMPLEMENTATION_BLOBS = {
    ".github/workflows/wf0-windows-msvc-build.yml": "c4bbcb03d2bc9b464433bdb37931fb3d5a0f169c",
    "tools/wf0-factory-census/artifacts.py": "78f03c49cf313ac761578fed9287678d40178ec9",
    "tools/wf0-factory-census/build.py": "b81de04c2fd7dd6af29f27b7ef736ab9f76b075f",
    "tools/wf0-factory-census/common.py": "dc5e2e2309733bd77a4f528e43940bdc571ce67d",
    "tools/wf0-factory-census/normalize.py": "6efeeb4e4b568d623358841e1b4bba34eb8c9464",
    "tools/wf0-factory-census/supervise.py": "7bcd5f0ad93b0acf919af2ce9c17081d9ef1addb",
    "tools/wf0-factory-census/verify.py": "589ef7594303ebd5e8741d5303c56d1ee17348e8",
    "windows-factory-probe/source/component_instance_session.cpp": "30ddecfdbc245990211acaea7d8326e35b45713e",
    "windows-factory-probe/source/component_instance_session.h": "3f15fa52792ca75e4238c8f707e2154424a4f910",
    "windows-factory-probe/source/main.cpp": "c37b4ca23787de515b9c256ea2215ae7f10d2c1c",
}
PC0_FAILED_TRANSACTION = "a14bcc65d15e9fbdf15810a658b2ee87"
PC0_FAILED_INPUT_SHA256 = "ae89ee61636074feac5c875bab9b8a9621e3a0c6f7fa83b6d33bc11b94e631a3"
PC0_PROOF_PLAN_SHA256 = "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248"
PC0_FAILED_INTENT_SHA256 = "80510b7dfe12415e13b9af2bf29164e36fab7db99abd239cdab26d8accbe1ddc"
PC0_WINDOWS_BUILD_INPUT_SHA256 = "575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849"
PC0_HOST_MANIFEST_SHA256 = "d0e11c374b7b1cb99b357faaa109e9310edc265c159559bd59a2098148484e9c"
PC0_PRODUCER_RUN_ID = 33812659869
PC0_PRODUCER_RUN_ATTEMPT = 1
PC0_ARTIFACT_ID = 9915439437
PC0_FIXTURE_IDENTITY_SHA256 = "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6"
PC0_FAILURE_DIAGNOSTIC_SCHEMA = "linux-vst-bridge-pc0-failure-diagnostic/v1"
PC0_FAILURE_DIAGNOSTIC_KEYS = {
    "schema", "operation_nonce", "phase_nonce", "execution_source",
    "execution_input_sha256", "proof_plan_sha256", "run_id", "raw_exit",
    "classification", "primary_blocker", "secondary_cleanup_blocker",
    "last_lifecycle", "last_in_flight_operation", "durable_record_count",
    "durable_records", "call_counts", "audio_processor_observer_state",
    "audio_interface_quiescence", "inherited_shutdown", "cleanup",
    "environment_retirement_disposition", "stdout_sha256", "stderr_sha256",
    "stderr_bytes", "protected_snapshot_sha256", "runner_identity_sha256",
}
PC0_DIAGNOSTIC_CLASSIFICATIONS = {
    "output_publication_failed", "supervision_failed",
    "supervision_and_process_cleanup_failed", "process_cleanup_failed",
    "call_timeout", "stage_timeout", "abnormal_termination_in_flight",
    "scanner_blocked",
}
PC0_DIAGNOSTIC_OPERATIONS = tuple(sorted({
    "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
    "query_factory_2", "query_factory_3", "count_classes",
    "get_class_info_unicode", "get_class_info_2", "get_class_info_1",
    "create_component", "get_controller_class_id", "initialize_component",
    "query_audio_processor", "get_bus_count", "get_bus_info",
    "get_bus_arrangement", "can_process_sample_size", "release_audio_processor",
    "terminate_component", "release_component", "release_factory_3",
    "release_factory_2", "release_factory_base", "exit_dll", "free_library",
}))
PC0_DIAGNOSTIC_PRIMARY_BLOCKERS = frozenset(EXIT_BLOCKER.values()) | \
    frozenset(IN_FLIGHT_BLOCKER.values()) | {"PC0_PROCESS_CLEANUP_BLOCKED"}
PC0_SHUTDOWN_OPERATIONS = (
    "terminate_component", "release_component", "release_factory_3",
    "release_factory_2", "release_factory_base", "exit_dll", "free_library",
)
PC0_DIAGNOSTIC_INTERFACES = {
    "load_library": None, "init_dll": None, "get_plugin_factory": None,
    "get_factory_info": "IPluginFactory", "query_factory_2": "IPluginFactory",
    "query_factory_3": "IPluginFactory", "count_classes": "IPluginFactory",
    "get_class_info_unicode": "IPluginFactory3",
    "get_class_info_2": "IPluginFactory2", "get_class_info_1": "IPluginFactory",
    "create_component": "IPluginFactory", "get_controller_class_id": "IComponent",
    "initialize_component": "IComponent", "query_audio_processor": "IComponent",
    "get_bus_count": "IComponent", "get_bus_info": "IComponent",
    "get_bus_arrangement": "IAudioProcessor",
    "can_process_sample_size": "IAudioProcessor",
    "release_audio_processor": "IAudioProcessor",
    "terminate_component": "IComponent", "release_component": "IComponent",
    "release_factory_3": "IPluginFactory3",
    "release_factory_2": "IPluginFactory2",
    "release_factory_base": "IPluginFactory", "exit_dll": None,
    "free_library": None,
}
PC0_DIAGNOSTIC_TIERS = {
    "get_class_info_unicode": "factory_3_unicode",
    "get_class_info_2": "factory_2", "get_class_info_1": "factory_1",
}
PC0_DIAGNOSTIC_RETURN_KINDS = {
    "load_library": {"win32_error", "handle_nonnull"}, "init_dll": {"bool"},
    "get_plugin_factory": {"pointer_null", "pointer_nonnull"},
    "get_factory_info": {"tresult"}, "query_factory_2": {"tresult"},
    "query_factory_3": {"tresult"}, "count_classes": {"i32"},
    "get_class_info_unicode": {"tresult"}, "get_class_info_2": {"tresult"},
    "get_class_info_1": {"tresult"}, "create_component": {"tresult"},
    "get_controller_class_id": {"tresult"}, "initialize_component": {"tresult"},
    "query_audio_processor": {"tresult"}, "get_bus_count": {"int32"},
    "get_bus_info": {"tresult"}, "get_bus_arrangement": {"tresult"},
    "can_process_sample_size": {"tresult"},
    "release_audio_processor": {"reference_count"},
    "terminate_component": {"tresult"}, "release_component": {"reference_count"},
    "release_factory_3": {"u32"}, "release_factory_2": {"u32"},
    "release_factory_base": {"u32"}, "exit_dll": {"bool"},
    "free_library": {"bool_true", "win32_error"},
}
PC0_DIAGNOSTIC_LIFECYCLE_RANKS = {
    "scanner_started": 0, "readiness_announced": 1,
    "supervisor_gate_accepted": 2, "module_open_started": 3,
    "module_opened": 4, "factory_export_missing": 5,
    "module_entry_succeeded": 5, "module_entry_failed": 5,
    "module_entry_absent": 5, "factory_export_found": 6,
    "factory_get_started": 7, "factory_get_failed": 8, "factory_obtained": 8,
    "factory_info_obtained": 9, "factory_interface_versions_recorded": 10,
    "class_count_obtained": 11, "class_enumeration_in_progress": 12,
    "class_enumeration_complete": 13, "component_create_in_flight": 14,
    "component_absent": 15, "component_created": 15,
    "controller_id_in_flight": 16, "controller_id_verified": 17,
    "host_context_ready": 18, "component_initialize_in_flight": 19,
    "component_initialized": 20, "audio_processor_query_in_flight": 21,
    "audio_processor_absent": 22,
    "audio_processor_query_returned_without_lease": 22,
    "audio_processor_lease_acquired": 22,
    "pre_setup_census_absent": 22, "bus_count_in_flight": 23,
    "bus_counts_validated": 24, "detail_call_in_flight": 25,
    "bus_info_complete": 26, "speaker_arrangements_complete": 27,
    "sample_format_call_in_flight": 28, "pre_setup_contract_complete": 29,
    "pre_setup_census_blocked": 29, "audio_processor_release_in_flight": 30,
    "audio_processor_lease_retired": 31,
    "audio_processor_retirement_incomplete": 31,
    "audio_processor_ownership_unknown": 31,
    "audio_interface_quiescence_proved": 32,
    "audio_interface_quiescence_unproved": 32,
    "component_terminate_in_flight": 33, "component_terminated": 34,
    "component_release_in_flight": 35, "component_released": 36,
    "component_retirement_incomplete": 36, "object_quiescence_proved": 37,
    "object_quiescence_unproved": 37, "component_session_closed": 38,
    "inherited_shutdown_suppressed": 39, "module_exit_succeeded": 40,
    "module_exit_failed": 40, "module_exit_absent": 40,
    "module_unloaded": 41, "scanner_completed": 42,
}
PC0_DIAGNOSTIC_LIFECYCLE_REGRESSIONS = {
    ("controller_id_in_flight", "component_created"),
    ("component_initialize_in_flight", "host_context_ready"),
    ("component_terminate_in_flight", "component_initialized"),
}
PC0_DIAGNOSTIC_AUDIO_STATES = {
    "audio_processor_absent", "audio_processor_query_in_flight",
    "audio_processor_query_returned_without_lease", "audio_processor_lease_acquired",
    "audio_processor_release_in_flight", "audio_processor_lease_retired",
    "audio_processor_retirement_incomplete", "audio_processor_ownership_unknown",
}
PC0_DIAGNOSTIC_COMPONENT_STATES = {
    "component_absent", "component_create_in_flight", "component_created",
    "controller_id_in_flight", "controller_id_verified", "host_context_ready",
    "component_initialize_in_flight", "component_initialized",
    "component_terminate_in_flight", "component_terminated",
    "component_release_in_flight", "component_released",
    "component_retirement_incomplete",
}
PC0_DIAGNOSTIC_CALLBACK_OPERATIONS = {
    "queryInterface", "addRef", "release", "getName", "createInstance",
}


def pc0_v3_source_authority() -> dict[str, Any]:
    return {
        "reviewed_design_commit": PC0_V3_DESIGN_COMMIT,
        "design_blob": PC0_V3_DESIGN_BLOB,
        "design_sha256": PC0_V3_DESIGN_SHA256,
        "technical_lead_review": PC0_V3_REVIEW_ID,
        "approval_blob": PC0_V3_APPROVAL_BLOB,
    }


def pc0_v3_complete_source(commit: str, *, root: pathlib.Path | None = None) -> dict[str, Any]:
    repository = root or repo_root()
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source commit is malformed")
    parent = command_text(["git", "rev-parse", f"{commit}^"], cwd=repository)
    tree = command_text(["git", "rev-parse", f"{commit}^{{tree}}"], cwd=repository)
    parents = command_text(
        ["git", "rev-list", "--parents", "-n", "1", commit], cwd=repository
    ).split()
    changed = command_text(
        ["git", "diff", "--name-only", PC0_V3_AUTHORITY_COMMIT, commit], cwd=repository
    ).splitlines()
    if (parent != PC0_V3_AUTHORITY_COMMIT or parents != [commit, parent]
            or command_text(["git", "rev-parse", f"{parent}^{{tree}}"], cwd=repository)
            != PC0_V3_AUTHORITY_TREE or changed != list(PC0_SOURCE_PATHS)):
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source authority/topology differs")
    authority_blobs = {
        "CURRENT_SLICE.md": PC0_V3_CURRENT_SLICE_BLOB,
        "docs/slices/PC0/IMPLEMENTATION_DESIGN.md": PC0_V3_DESIGN_BLOB,
        "docs/slices/PC0/DESIGN_APPROVAL.md": PC0_V3_APPROVAL_BLOB,
        "docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md": PC0_V3_REVIEW_HISTORY_BLOB,
    }
    for path, expected in authority_blobs.items():
        if command_text(["git", "rev-parse", f"{parent}:{path}"], cwd=repository) != expected:
            fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 authority blob differs")
    design = command(
        ["git", "cat-file", "blob", f"{parent}:docs/slices/PC0/IMPLEMENTATION_DESIGN.md"],
        cwd=repository,
    ).stdout
    if sha256_bytes(design) != PC0_V3_DESIGN_SHA256:
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 design bytes differ")
    records = dx0_records(commit, PC0_SOURCE_PATHS, root=repository)
    frozen_records = {record["path"]: record for record in records}
    if any(frozen_records[path] != {
            "path": path, "git_mode": "100644", "git_blob": blob,
    } for path, blob in PC0_V3_FROZEN_IMPLEMENTATION_BLOBS.items()):
        fail("RETURN_TO_DESIGN_GATE: fifth PC0 repair path or frozen blob drift")
    build_input = dx0_windows_build_input(commit, root=repository)
    if (build_input["record_count"] != 17
            or dx0_identity_sha256(build_input) != PC0_WINDOWS_BUILD_INPUT_SHA256):
        fail("RETURN_TO_DESIGN_GATE: WindowsBuildInputIdentity differs")
    value = {
        "schema": PC0_COMPLETE_SOURCE_SCHEMA,
        "commit": commit, "tree": tree, "parent": parent, "ref": PC0_REF,
        "record_count": len(records), "records": records,
    }
    dx0_complete_source_sha256(value)
    return value


def pc0_v3_require_frozen_source(commit: str, *, detached: bool) -> dict[str, Any]:
    root = repo_root()
    if (command_text(["git", "rev-parse", "HEAD"], cwd=root) != commit
            or command_text(["git", "status", "--porcelain=v1", "--untracked-files=all"],
                            cwd=root)
            or (detached and command_text(["git", "branch", "--show-current"], cwd=root))):
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 detached clean worktree differs")
    return pc0_v3_complete_source(commit, root=root)


def _pc0_v3_bundle_header(path: pathlib.Path) -> tuple[int, list[tuple[str, str]]]:
    if (not path.is_file() or path.is_symlink()
            or path.stat().st_size > 128 * 1024 * 1024):
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source bundle is absent or unsafe")
    output = command(["git", "bundle", "list-heads", str(path)]).stdout.decode(
        "utf-8", "strict"
    )
    refs: list[tuple[str, str]] = []
    for line in output.splitlines():
        fields = line.split()
        if len(fields) != 2 or not re.fullmatch(r"[0-9a-f]{40}", fields[0]):
            fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source bundle header differs")
        refs.append((fields[0], fields[1]))
    verified = command(["git", "bundle", "verify", str(path)], cwd=repo_root())
    text = (verified.stdout + verified.stderr).decode("utf-8", "replace").lower()
    prerequisites = sum(line.startswith("-") for line in text.splitlines())
    if "complete history" not in text:
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source bundle is not complete")
    return prerequisites, refs


def pc0_v3_verify_source_handoff(stage: pathlib.Path, source_commit: str, *,
                                 reconstruct: bool = True) -> dict[str, Any]:
    expected_names = {
        f"dx0-execution-source-{source_commit}.bundle",
        "DX0_SOURCE_HANDOFF_RECEIPT.json", "DX0_SOURCE_HANDOFF_RECEIPT.sha256",
    }
    if (not stage.is_dir() or stage.is_symlink()
            or {path.name for path in stage.iterdir()} != expected_names):
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source-handoff roster differs")
    receipt_path = stage / "DX0_SOURCE_HANDOFF_RECEIPT.json"
    receipt_sha = verify_hash_sidecar(
        stage / "DX0_SOURCE_HANDOFF_RECEIPT.sha256", receipt_path
    )
    receipt = read_canonical_json(receipt_path, DX0_SOURCE_HANDOFF_SCHEMA)
    bundle = stage / f"dx0-execution-source-{source_commit}.bundle"
    prerequisites, refs = _pc0_v3_bundle_header(bundle)
    expected_ref = f"refs/handoff/dx0-source/{source_commit}"
    source = pc0_v3_complete_source(source_commit)
    if reconstruct:
        with tempfile.TemporaryDirectory(prefix="pc0-v3-source-verify-") as temporary:
            bare = pathlib.Path(temporary) / "verify.git"
            command(["git", "init", "--bare", str(bare)])
            command(["git", "-C", str(bare), "fetch", "--no-tags", str(bundle),
                     f"{expected_ref}:refs/verify/source"])
            if command_text(["git", "-C", str(bare), "rev-parse",
                             "refs/verify/source"]) != source_commit:
                fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source bundle commit differs")
            source = pc0_v3_complete_source(source_commit, root=bare)
    expected = {
        "schema": DX0_SOURCE_HANDOFF_SCHEMA,
        "repository": REPOSITORY,
        "design_authority": pc0_v3_source_authority(),
        "implementation_basis": {
            "commit": PC0_V3_AUTHORITY_COMMIT, "tree": PC0_V3_AUTHORITY_TREE,
        },
        "implementation_source": dx0_source_role(source),
        "implementation_branch": PC0_BRANCH,
        "implementation_ref": PC0_REF,
        "complete_source_schema": source["schema"],
        "complete_source_record_count": source["record_count"],
        "bundle": {
            "name": bundle.name, "advertised_ref": expected_ref,
            "sha256": sha256_file(bundle), "size": bundle.stat().st_size,
            "prerequisite_count": 0, "advertised_ref_count": 1,
            "git_bundle_verify": "passed",
        },
    }
    if (receipt != expected or prerequisites != 0
            or refs != [(source_commit, expected_ref)]):
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source-handoff identity differs")
    return {"receipt": receipt, "receipt_sha256": receipt_sha,
            "bundle": str(bundle)}


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


def _pc0_read_exact_file(path: pathlib.Path, maximum: int, label: str) -> bytes:
    if (not path.is_file() or path.is_symlink() or path.stat().st_size > maximum):
        fail(f"RETURN_TO_DESIGN_GATE: {label} is absent or unsafe")
    return path.read_bytes()


def _pc0_absent(path: pathlib.Path, label: str) -> None:
    if path.exists() or path.is_symlink():
        fail(f"RETURN_TO_DESIGN_GATE: contradictory {label} exists")


def _pc0_contradictory_remote_object_count(
        proof_root: pathlib.Path, expected: set[pathlib.Path]) -> int:
    candidates: list[pathlib.Path] = []
    patterns = (
        "intents/*/DX0_DECK_EXECUTION_INTENT.json",
        "results/by-execution-input/*/DX0_TRANSACTION_RESULT.json",
        "results/by-execution-input/.locks/*/prepared-intent.json",
        "results/.driver-locks/*/prepared-intent.json",
    )
    for pattern in patterns:
        observed = sorted(proof_root.glob(pattern))
        if len(observed) > 256:
            fail("RETURN_TO_DESIGN_GATE: remote proof object census exceeds bound")
        candidates.extend(observed)
    contradictions = 0
    for path in candidates:
        data = _pc0_read_exact_file(path, 2 * 1024 * 1024, "remote proof object")
        value = parse_json_no_duplicates(data, "remote proof object")
        if canonical_json(value) != data:
            fail("RETURN_TO_DESIGN_GATE: remote proof object is not canonical")
        source = value.get("execution_source") or value.get("deck_execution_source")
        if isinstance(source, dict) and source.get("ref") == PC0_REF \
                and path not in expected:
            contradictions += 1
    return contradictions


def corrective_preflight(*, operation_nonce: str, source_commit: str,
                         execution_input_sha256: str, proof_plan_sha256: str,
                         source_handoff_receipt_sha256: str) -> dict[str, Any]:
    """One V3-only read-only Deck safety proof before any reservation or intent."""
    if (sys.executable != "/usr/bin/python3" or not sys.dont_write_bytecode
            or any(os.environ.get(name) for name in (
                "GH_TOKEN", "GITHUB_TOKEN", "GITHUB_PAT", "SSH_AUTH_SOCK"))):
        fail("RETURN_TO_DESIGN_GATE: corrective preflight interpreter/authority differs")
    _deck_hex(operation_nonce, DECK_HEX32, "corrective operation nonce")
    _deck_hex(execution_input_sha256, DECK_HEX64, "corrective execution input")
    if proof_plan_sha256 != PC0_PROOF_PLAN_SHA256:
        fail("RETURN_TO_DESIGN_GATE: corrective proof plan differs")

    source = pc0_v3_require_frozen_source(source_commit, detached=True)
    source_role = dx0_source_role(source)
    handoff_root = dx0_deck_source_parent() / source_commit
    handoff = pc0_v3_verify_source_handoff(
        handoff_root, source_commit, reconstruct=False
    )
    if handoff["receipt_sha256"] != source_handoff_receipt_sha256:
        fail("RETURN_TO_DESIGN_GATE: corrective source handoff differs")

    counts = process_guard()
    fixture_identity = deck_fixture_identity()
    runner = verify_runner_identity()
    if runner["launch_critical_manifest_sha256"] != RUNNER_DIGEST:
        fail("RETURN_TO_DESIGN_GATE: corrective runner differs")
    protected = protected_snapshot()

    host_root = dx0_deck_host_artifact_parent() / PC0_HOST_MANIFEST_SHA256
    build_input_sha = read_canonical_json(
        host_root / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json"
    )["windows_build_input"]["sha256"]
    host = verify_host_store(host_root, build_input_sha)
    fixture = verify_fixture_store(
        dx0_deck_fixture_parent() / DX0_AGAIN_BUNDLE_MANIFEST_SHA256
    )
    workflow = host["build_receipt"]["workflow"]
    custody = host["custody"]
    if (build_input_sha != PC0_WINDOWS_BUILD_INPUT_SHA256
            or host["manifest_sha256"] != PC0_HOST_MANIFEST_SHA256
            or workflow["run_id"] != PC0_PRODUCER_RUN_ID
            or workflow["run_attempt"] != PC0_PRODUCER_RUN_ATTEMPT
            or custody["artifact"]["id"] != PC0_ARTIFACT_ID
            or fixture["identity_sha256"] != PC0_FIXTURE_IDENTITY_SHA256):
        fail("RETURN_TO_DESIGN_GATE: corrective Deck stores differ")

    proof_root = dx0_deck_result_parent().parent.parent
    historical_intent = (
        proof_root / "intents" / PC0_FAILED_TRANSACTION
        / "DX0_DECK_EXECUTION_INTENT.json"
    )
    historical_bytes = _pc0_read_exact_file(
        historical_intent, 2 * 1024 * 1024, "historical Deck intent"
    )
    historical_value = parse_json_no_duplicates(
        historical_bytes, "historical Deck intent"
    )
    if (canonical_json(historical_value) != historical_bytes
            or sha256_bytes(historical_bytes) != PC0_FAILED_INTENT_SHA256
            or historical_value.get("operation_nonce") != PC0_FAILED_TRANSACTION
            or historical_value.get("deck_execution_input_sha256")
            != PC0_FAILED_INPUT_SHA256
            or historical_value.get("proof_plan_sha256") != PC0_PROOF_PLAN_SHA256):
        fail("RETURN_TO_DESIGN_GATE: historical Deck intent differs")

    historical_result_root = dx0_deck_result_parent() / PC0_FAILED_INPUT_SHA256
    _pc0_absent(historical_result_root, "historical success result root")
    lock_name = f"{PC0_FAILED_INPUT_SHA256}-{PC0_PROOF_PLAN_SHA256}"
    inner_lock = dx0_deck_result_parent() / ".locks" / lock_name
    outer_lock = proof_root / "results" / ".driver-locks" / lock_name
    for lock, label in ((inner_lock, "historical inner lock"),
                        (outer_lock, "historical outer lock")):
        prepared = lock / "prepared-intent.json"
        if (not lock.is_dir() or lock.is_symlink()
                or {path.name for path in lock.iterdir()} != {"prepared-intent.json"}
                or _pc0_read_exact_file(prepared, 2 * 1024 * 1024, label)
                != historical_bytes):
            fail(f"RETURN_TO_DESIGN_GATE: {label} differs")
    contradictory_count = _pc0_contradictory_remote_object_count(
        proof_root, {
            historical_intent,
            inner_lock / "prepared-intent.json",
            outer_lock / "prepared-intent.json",
        },
    )
    if contradictory_count != 0:
        fail("RETURN_TO_DESIGN_GATE: contradictory PC0 remote object exists")

    current_intent_root = proof_root / "intents" / operation_nonce
    current_result_root = dx0_deck_result_parent() / execution_input_sha256
    current_lock_name = f"{execution_input_sha256}-{proof_plan_sha256}"
    current_inner = dx0_deck_result_parent() / ".locks" / current_lock_name
    current_outer = proof_root / "results" / ".driver-locks" / current_lock_name
    for path, label in (
        (current_intent_root, "current execution intent"),
        (current_result_root, "current result"),
        (current_inner, "current inner execution lock"),
        (current_outer, "current outer execution lock"),
    ):
        _pc0_absent(path, label)

    # Recheck the worktree after every potentially expensive read-only probe.
    pc0_v3_require_frozen_source(source_commit, detached=True)
    return {
        "operation_nonce": operation_nonce,
        "execution_source": source_role,
        "execution_input_sha256": execution_input_sha256,
        "proof_plan_sha256": proof_plan_sha256,
        "source_handoff": {
            "receipt_sha256": handoff["receipt_sha256"],
            "bundle_sha256": handoff["receipt"]["bundle"]["sha256"],
            "advertised_ref": handoff["receipt"]["bundle"]["advertised_ref"],
        },
        "worktree": {"commit": source_commit, "detached": True, "clean": True},
        "process_guard": {"process_counts": counts, "prohibited_sibling_count": 0},
        "fixture": {
            **fixture_identity, "github_authority_absent": True,
            "forwarded_ssh_agent_absent": True,
        },
        "runner_identity_sha256": RUNNER_DIGEST,
        "protected_snapshot_sha256": sha256_bytes(canonical_json(protected)),
        "stores": {
            "host_artifact_manifest_sha256": PC0_HOST_MANIFEST_SHA256,
            "producer_run_id": PC0_PRODUCER_RUN_ID,
            "producer_run_attempt": PC0_PRODUCER_RUN_ATTEMPT,
            "artifact_id": PC0_ARTIFACT_ID,
            "again_bundle_manifest_sha256": DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
            "accepted_fixture_identity_sha256": PC0_FIXTURE_IDENTITY_SHA256,
            "host_transfer_fallback_selected": False,
            "fixture_transfer_fallback_selected": False,
        },
        "historical_failed_transaction": {
            "transaction_id": PC0_FAILED_TRANSACTION,
            "execution_input_sha256": PC0_FAILED_INPUT_SHA256,
            "expected_intent_sha256": PC0_FAILED_INTENT_SHA256,
            "intent": "present_matched", "result": "absent",
            "result_sidecar": "absent", "inner_lock": "present_intent_matched",
            "inner_prepared_intent_sha256": PC0_FAILED_INTENT_SHA256,
            "outer_lock": "present_intent_matched",
            "outer_prepared_intent_sha256": PC0_FAILED_INTENT_SHA256,
            "contradictory_pc0_object_count": contradictory_count,
        },
        "current_corrective_absence": {
            "execution_intent_absent": True, "outer_lock_absent": True,
            "inner_lock_absent": True, "result_absent": True,
            "result_sidecar_absent": True, "diagnostic_absent": True,
            "diagnostic_sidecar_absent": True,
        },
        "write_effect_counts": {
            "environment_creations": 0, "execution_intent_publications": 0,
            "execution_lock_creations": 0, "result_publications": 0,
            "diagnostic_publications": 0, "protected_state_mutations": 0,
            "deck_execution_reservations": 0,
            "deck_execution_count_increments": 0, "proton_launches": 0,
        },
    }


def _pc0_diagnostic_coordinates(record: dict[str, Any]) -> dict[str, Any]:
    fields = {key: record[key] for key in (
        "media_type", "direction", "index", "audio_index", "symbolic_size"
    ) if key in record}
    required = {
        "get_bus_count": {"media_type", "direction"},
        "get_bus_info": {"media_type", "direction", "index"},
        "get_bus_arrangement": {"direction", "audio_index"},
        "can_process_sample_size": {"symbolic_size"},
    }.get(record.get("operation"), set())
    if set(fields) != required:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call coordinates differ")
    if ("media_type" in fields and fields["media_type"] not in {"kAudio", "kEvent"}) \
            or ("direction" in fields and fields["direction"] not in {"kInput", "kOutput"}) \
            or ("symbolic_size" in fields
                and fields["symbolic_size"] not in {"kSample32", "kSample64"}):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic coordinate enum differs")
    for key in ("index", "audio_index"):
        if key in fields and (type(fields[key]) is not int or not 0 <= fields[key] < 32):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic coordinate bound differs")
    return fields


def _pc0_validate_diagnostic_call(record: dict[str, Any], *, completed: bool) -> None:
    operation = record["operation"]
    if (record["interface"] != PC0_DIAGNOSTIC_INTERFACES[operation]
            or record["tier"] != PC0_DIAGNOSTIC_TIERS.get(operation)):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call enum differs")
    ordinal = record["ordinal"]
    if operation in PC0_DIAGNOSTIC_TIERS:
        if type(ordinal) is not int or not 0 <= ordinal < 256:
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic ordinal differs")
    elif ordinal is not None:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic ordinal differs")
    coordinates = set(_pc0_diagnostic_coordinates(record))
    base = {"event", "sequence", "operation", "interface", "ordinal", "tier"}
    start_payloads = {
        "create_component": {"object_role", "processor_cid_raw_tuid_hex",
                             "requested_iid_raw_tuid_hex"},
        "get_controller_class_id": {"object_role"},
        "initialize_component": {"object_role"},
        "query_audio_processor": {"object_role", "requested_interface",
                                  "requested_iid_raw_tuid_hex"},
        "release_audio_processor": {"object_role"},
        "terminate_component": {"object_role"},
        "release_component": {"object_role"},
    }
    completed_payloads = {
        "load_library": {"win32_error_u32_hex"}, "init_dll": {"bool_result"},
        "get_plugin_factory": set(), "get_factory_info": {"result_u32_hex"},
        "query_factory_2": {"result_u32_hex"},
        "query_factory_3": {"result_u32_hex"}, "count_classes": {"i32_result"},
        "get_class_info_unicode": {"result_u32_hex"},
        "get_class_info_2": {"result_u32_hex"},
        "get_class_info_1": {"result_u32_hex"},
        "create_component": {"result_u32_hex", "output_nonnull", "object_role"},
        "get_controller_class_id": {"result_u32_hex",
                                    "controller_cid_raw_tuid_hex", "object_role"},
        "initialize_component": {"result_u32_hex", "host_reference_count",
                                 "object_role"},
        "query_audio_processor": {"result_u32_hex", "output_nonnull",
                                  "object_role", "requested_interface"},
        "get_bus_count": {"i32_result"}, "get_bus_info": {"result_u32_hex"},
        "get_bus_arrangement": {"result_u32_hex"},
        "can_process_sample_size": {"result_u32_hex"},
        "release_audio_processor": {"u32_result", "object_role"},
        "terminate_component": {"result_u32_hex", "host_reference_count",
                                "object_role"},
        "release_component": {"u32_result", "object_role"},
        "release_factory_3": {"u32_result"}, "release_factory_2": {"u32_result"},
        "release_factory_base": {"u32_result"}, "exit_dll": {"bool_result"},
        "free_library": {"win32_error_u32_hex"},
    }
    if completed:
        if (not {"attempt_sequence", "return_kind"}.issubset(record)
                or record["return_kind"] not in PC0_DIAGNOSTIC_RETURN_KINDS[operation]):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic return kind differs")
        expected_keys = (base | coordinates | {"attempt_sequence", "return_kind"}
                         | completed_payloads[operation])
    elif "attempt_sequence" in record or "return_kind" in record:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic attempt schema differs")
    else:
        expected_keys = base | coordinates | start_payloads.get(operation, set())
    if set(record) != expected_keys:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call value roster differs")
    for key in ("result_u32_hex", "win32_error_u32_hex"):
        if key in record and (not isinstance(record[key], str)
                              or re.fullmatch(r"[0-9a-f]{8}", record[key]) is None):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic scalar hex differs")
    for key in ("processor_cid_raw_tuid_hex", "requested_iid_raw_tuid_hex"):
        if key in record and (not isinstance(record[key], str)
                              or re.fullmatch(r"[0-9A-F]{32}", record[key]) is None):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic TUID differs")
    if ("controller_cid_raw_tuid_hex" in record
            and record["controller_cid_raw_tuid_hex"] is not None
            and (not isinstance(record["controller_cid_raw_tuid_hex"], str)
                 or re.fullmatch(r"[0-9A-F]{32}",
                                 record["controller_cid_raw_tuid_hex"]) is None)):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic controller TUID differs")
    for key in ("bool_result", "output_nonnull"):
        if key in record and type(record[key]) is not bool:
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call boolean differs")
    if ("i32_result" in record
            and (type(record["i32_result"]) is not int
                 or not -(2 ** 31) <= record["i32_result"] < 2 ** 31)):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic signed result differs")
    for key in ("u32_result", "host_reference_count"):
        if key in record and (type(record[key]) is not int
                              or not 0 <= record[key] <= 0xffffffff):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic unsigned result differs")
    if ("object_role" in record and record["object_role"] not in {
            "again_processor_component", "again_audio_processor_interface"}):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic object role differs")
    if ("requested_interface" in record
            and record["requested_interface"] != "Steinberg::Vst::IAudioProcessor"):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic requested interface differs")


def _pc0_validate_diagnostic_callback(record: dict[str, Any],
                                      active: dict[str, Any] | None) -> None:
    base = {"event", "sequence", "operation", "origin",
            "enclosing_attempt_sequence", "enclosing_operation", "thread_role"}
    operation = record.get("operation")
    extra = ({"reference_count"} if operation in {"addRef", "release"} else
             {"result_u32_hex", "output_null", "cid_raw_tuid_hex",
              "iid_raw_tuid_hex"} if operation == "createInstance" else
             {"result_u32_hex", "output_null"})
    if (operation not in PC0_DIAGNOSTIC_CALLBACK_OPERATIONS
            or set(record) != base | extra
            or record.get("thread_role") != "scanner_main_thread"):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback schema differs")
    if operation in {"addRef", "release"}:
        if (type(record["reference_count"]) is not int
                or not 0 <= record["reference_count"] <= 0xffffffff):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback result differs")
    else:
        if (not isinstance(record["result_u32_hex"], str)
                or re.fullmatch(r"[0-9a-f]{8}", record["result_u32_hex"]) is None
                or type(record["output_null"]) is not bool):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback result differs")
        for key in ("cid_raw_tuid_hex", "iid_raw_tuid_hex"):
            if key in record and (not isinstance(record[key], str)
                                  or re.fullmatch(r"[0-9A-F]{32}", record[key]) is None):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback TUID differs")
    origin = record.get("origin")
    if origin == "component":
        if (active is None
                or record.get("enclosing_attempt_sequence") != active.get("sequence")
                or record.get("enclosing_operation") != active.get("operation")
                or active.get("operation") not in {
                    "initialize_component", "terminate_component"}):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback attribution differs")
    elif origin == "owner_local":
        if (active is not None or record.get("enclosing_attempt_sequence") is not None
                or record.get("enclosing_operation") is not None):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback attribution differs")
    else:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback origin differs")


def _pc0_validate_durable_records(records: Any, call_counts: dict[str, Any],
                                  last_in_flight: Any,
                                  last_lifecycle: Any) -> list[dict[str, Any]]:
    if (not isinstance(records, list) or len(records) > 2048
            or set(call_counts) != set(PC0_DIAGNOSTIC_OPERATIONS)
            or any(type(value) is not int or not 0 <= value <= 2048
                   for value in call_counts.values())):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic record/count bounds differ")
    lifecycle_keys = {
        "event", "sequence", "state", "class_count", "operation",
        "disposition", "object_quiescence", "component_state",
        "primary_blocker", "audio_interface_quiescence",
        "audio_processor_state", "callback_ledger_unchanged",
        "release_reference_count", "pointer_cleared",
    }
    call_keys = {
        "event", "sequence", "attempt_sequence", "operation", "interface",
        "ordinal", "tier", "return_kind", "result_u32_hex",
        "win32_error_u32_hex", "i32_result", "u32_result", "bool_result",
        "output_nonnull", "host_reference_count", "object_role",
        "processor_cid_raw_tuid_hex", "requested_iid_raw_tuid_hex",
        "controller_cid_raw_tuid_hex", "requested_interface", "media_type",
        "direction", "index", "audio_index", "symbolic_size",
    }
    callback_keys = {
        "event", "sequence", "operation", "origin",
        "enclosing_attempt_sequence", "enclosing_operation", "thread_role",
        "reference_count", "result_u32_hex", "output_null",
        "cid_raw_tuid_hex", "iid_raw_tuid_hex",
    }
    active: dict[str, Any] | None = None
    observed_counts = {name: 0 for name in PC0_DIAGNOSTIC_OPERATIONS}
    observed_lifecycle = None
    lifecycle_rank = -1
    callback_count = 0
    for expected_sequence, record in enumerate(records, 1):
        if (not isinstance(record, dict)
                or type(record.get("sequence")) is not int
                or record["sequence"] != expected_sequence):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic sequence differs")
        event = record.get("event")
        allowed = (lifecycle_keys if event == "lifecycle" else call_keys
                   if event in {"call_started", "call_completed"} else
                   callback_keys if event == "host_callback" else set())
        if not allowed or not set(record).issubset(allowed):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic record schema differs")
        if event == "lifecycle":
            state = record.get("state")
            rank = PC0_DIAGNOSTIC_LIFECYCLE_RANKS.get(state)
            if rank is None or (rank < lifecycle_rank and
                                (observed_lifecycle, state)
                                not in PC0_DIAGNOSTIC_LIFECYCLE_REGRESSIONS):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic lifecycle differs")
            lifecycle_rank = max(lifecycle_rank, rank)
            if ("class_count" in record and
                    (type(record["class_count"]) is not int
                     or not 0 <= record["class_count"] <= 256)):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic lifecycle value differs")
            if ("operation" in record
                    and record["operation"] not in PC0_SHUTDOWN_OPERATIONS):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic lifecycle operation differs")
            if ("disposition" in record and record["disposition"] not in {
                    "not_attempted_object_quiescence_unproved",
                    "not_attempted_audio_interface_quiescence_unproved"}):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic lifecycle disposition differs")
            for key in ("object_quiescence", "audio_interface_quiescence",
                        "callback_ledger_unchanged", "pointer_cleared"):
                if key in record and type(record[key]) is not bool:
                    fail("PC0_EVIDENCE_BLOCKED: failure diagnostic lifecycle boolean differs")
            if ("component_state" in record
                    and record["component_state"] not in PC0_DIAGNOSTIC_COMPONENT_STATES):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic component state differs")
            if ("audio_processor_state" in record
                    and record["audio_processor_state"] not in PC0_DIAGNOSTIC_AUDIO_STATES):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic audio state differs")
            if ("primary_blocker" in record and record["primary_blocker"] not in {
                    None, *PC0_DIAGNOSTIC_PRIMARY_BLOCKERS}):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic lifecycle blocker differs")
            if ("release_reference_count" in record
                    and (type(record["release_reference_count"]) is not int
                         or not 0 <= record["release_reference_count"] <= 0xffffffff)):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic reference count differs")
            observed_lifecycle = state
        elif event == "call_started":
            operation = record.get("operation")
            if (not {"event", "sequence", "operation", "interface", "ordinal", "tier"}
                    .issubset(record) or operation not in observed_counts
                    or active is not None):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call start differs")
            _pc0_validate_diagnostic_call(record, completed=False)
            observed_counts[operation] += 1
            active = record
        elif event == "call_completed":
            if (active is None or record.get("operation") != active.get("operation")
                    or record.get("interface") != active.get("interface")
                    or record.get("ordinal") != active.get("ordinal")
                    or record.get("tier") != active.get("tier")
                    or record.get("attempt_sequence") != active.get("sequence")
                    or _pc0_diagnostic_coordinates(record)
                    != _pc0_diagnostic_coordinates(active)):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call pairing differs")
            _pc0_validate_diagnostic_call(record, completed=True)
            active = None
        else:
            callback_count += 1
            _pc0_validate_diagnostic_callback(record, active)
            if callback_count > 64:
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback bound differs")
    if observed_counts != call_counts:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call counts differ")
    expected_in_flight = None if active is None else active["operation"]
    if last_in_flight != expected_in_flight or last_lifecycle != observed_lifecycle:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic terminal record joins differ")
    return records


def validate_failure_diagnostic(value: Any, *, expected_source: dict[str, Any],
                                expected_execution_input_sha256: str,
                                expected_plan_sha256: str,
                                expected_operation_nonce: str,
                                expected_phase_nonce: str) -> dict[str, Any]:
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
    if diagnostic["runner_identity_sha256"] != RUNNER_DIGEST:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic runner differs")
    if (type(diagnostic["stderr_bytes"]) is not int
            or not 0 <= diagnostic["stderr_bytes"] <= 65536):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic stderr bound differs")
    return diagnostic


def validate_failure_diagnostic_file(path: pathlib.Path, *,
                                     expected_source: dict[str, Any],
                                     expected_execution_input_sha256: str,
                                     expected_plan_sha256: str,
                                     expected_operation_nonce: str,
                                     expected_phase_nonce: str) -> dict[str, Any]:
    data = _pc0_read_exact_file(path, 2 * 1024 * 1024, "failure diagnostic")
    sidecar = _pc0_read_exact_file(
        path.with_suffix(path.suffix + ".sha256"), 256,
        "failure diagnostic sidecar",
    )
    digest = sha256_bytes(data)
    if sidecar != f"{digest}  {path.name}\n".encode():
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic sidecar differs")
    value = parse_json_no_duplicates(data, "PC0 failure diagnostic")
    if canonical_json(value) != data:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic is not canonical")
    return validate_failure_diagnostic(
        value, expected_source=expected_source,
        expected_execution_input_sha256=expected_execution_input_sha256,
        expected_plan_sha256=expected_plan_sha256,
        expected_operation_nonce=expected_operation_nonce,
        expected_phase_nonce=expected_phase_nonce,
    )


def publish_failure_diagnostic(lock: pathlib.Path, run: dict[str, Any], *,
                               intent: dict[str, Any], source_role: dict[str, Any],
                               retirement_disposition: str) -> dict[str, Any]:
    timeline = sanitized_timeline(run)["positive"]
    inherited_shutdown = run["inherited_shutdown"]
    if inherited_shutdown.get("operations") == {}:
        started = [record.get("operation") for record in timeline
                   if record.get("event") == "call_started"]
        completed = [record.get("operation") for record in timeline
                     if record.get("event") == "call_completed"]
        operations = {}
        for operation in PC0_SHUTDOWN_OPERATIONS:
            if operation in started:
                operations[operation] = {
                    "disposition": ("completed" if operation in completed
                                    else "attempted_without_ordinary_return"),
                    "source": "scanner_call_ledger",
                }
            else:
                operations[operation] = {
                    "disposition": "not_attempted_prior_stage",
                    "source": "supervisor_call_ledger_absence",
                }
        inherited_shutdown = {**inherited_shutdown, "operations": operations}
    diagnostic = {
        "schema": PC0_FAILURE_DIAGNOSTIC_SCHEMA,
        "operation_nonce": intent["operation_nonce"],
        "phase_nonce": intent["phase_nonce"],
        "execution_source": source_role,
        "execution_input_sha256": intent["deck_execution_input_sha256"],
        "proof_plan_sha256": intent["proof_plan_sha256"],
        "run_id": run["run_id"], "raw_exit": run["raw_exit"],
        "classification": run["classification"],
        "primary_blocker": run["blocker"],
        "secondary_cleanup_blocker": run["secondary_cleanup_blocker"],
        "last_lifecycle": run["last_lifecycle"],
        "last_in_flight_operation": run["last_in_flight_operation"],
        "durable_record_count": len(timeline), "durable_records": timeline,
        "call_counts": run["call_counts"],
        "audio_processor_observer_state": run["audio_processor_observer_state"],
        "audio_interface_quiescence": run["audio_interface_quiescence"],
        "inherited_shutdown": inherited_shutdown,
        "cleanup": run["cleanup"],
        "environment_retirement_disposition": retirement_disposition,
        "stdout_sha256": run["stdout_sha256"],
        "stderr_sha256": run["stderr_sha256"], "stderr_bytes": run["stderr_bytes"],
        "protected_snapshot_sha256": sha256_bytes(canonical_json(run["protected_snapshot"])),
        "runner_identity_sha256": run["runner_identity"][
            "launch_critical_manifest_sha256"
        ],
    }
    validate_failure_diagnostic(
        diagnostic, expected_source=source_role,
        expected_execution_input_sha256=intent["deck_execution_input_sha256"],
        expected_plan_sha256=intent["proof_plan_sha256"],
        expected_operation_nonce=intent["operation_nonce"],
        expected_phase_nonce=intent["phase_nonce"],
    )
    path = lock / "PC0_FAILURE_DIAGNOSTIC.json"
    sidecar = lock / "PC0_FAILURE_DIAGNOSTIC.json.sha256"
    data = canonical_json(diagnostic)
    if len(data) > 2 * 1024 * 1024:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic exceeds bound")
    if path.exists() or path.is_symlink() or sidecar.exists() or sidecar.is_symlink():
        if (not path.is_file() or path.is_symlink() or not sidecar.is_file()
                or sidecar.is_symlink() or path.read_bytes() != data):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic publication conflicts")
    else:
        write_atomic(path, data, 0o400)
        write_atomic(sidecar, f"{sha256_bytes(data)}  {path.name}\n".encode(), 0o400)
    accepted = validate_failure_diagnostic_file(
        path, expected_source=source_role,
        expected_execution_input_sha256=intent["deck_execution_input_sha256"],
        expected_plan_sha256=intent["proof_plan_sha256"],
        expected_operation_nonce=intent["operation_nonce"],
        expected_phase_nonce=intent["phase_nonce"],
    )
    allowed = {"prepared-intent.json", path.name, sidecar.name}
    if {child.name for child in lock.iterdir()} != allowed:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic lock roster differs")
    return {"diagnostic": accepted, "sha256": sha256_file(path), "path": path}


def execute(intent_path: pathlib.Path) -> dict[str, Any]:
    if sys.executable != "/usr/bin/python3":
        fail(f"DX0 Deck execution requires /usr/bin/python3; observed {sys.executable}")
    intent = _load_intent(intent_path)
    source_commit = intent["execution_source"]["commit"]
    source = (pc0_v3_require_frozen_source(source_commit, detached=True)
              if intent["proof_plan"].get("plan_id") == PC0_PLAN_ID
              else dx0_require_frozen_source(source_commit, detached=True))
    source_role = dx0_source_role(source)
    if source_role != intent["execution_source"]:
        fail("DX0 Deck execution source differs from intent")
    plan = dx0_closed_plan(intent["proof_plan"].get("plan_id"))
    pc0 = plan["plan_id"] == PC0_PLAN_ID
    plan_sha = dx0_identity_sha256(plan)
    if plan != intent["proof_plan"] or plan_sha != intent["proof_plan_sha256"]:
        fail("DX0 Deck closed plan differs from intent")

    handoff_root = dx0_deck_source_parent() / source_commit
    handoff = (pc0_v3_verify_source_handoff(handoff_root, source_commit)
               if pc0 else verify_source_handoff(handoff_root, source_commit))
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
    run: dict[str, Any] | None = None
    retirement_disposition = "not_attempted_process_containment_unproved"
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
        retirement_disposition = "retired"
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
                retirement_disposition = "retired"
            except Exception:
                if not pc0:
                    raise
                retirement_disposition = "failed"
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
        if pc0 and run is not None:
            try:
                publish_failure_diagnostic(
                    lock, run, intent=intent, source_role=source_role,
                    retirement_disposition=retirement_disposition,
                )
            except Exception as diagnostic_error:
                primary_error = RuntimeError(
                    "PC0_EVIDENCE_BLOCKED: durable failure diagnostic publication failed"
                )
        raise primary_error
    fail("PC0_EVIDENCE_BLOCKED: positive batch ended without a result")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    execute_parser = sub.add_parser("execute")
    execute_parser.add_argument("--intent", type=pathlib.Path, required=True)
    preflight_parser = sub.add_parser("corrective-preflight")
    preflight_parser.add_argument("--operation-nonce", required=True)
    preflight_parser.add_argument("--source-commit", required=True)
    preflight_parser.add_argument("--execution-input-sha256", required=True)
    preflight_parser.add_argument("--proof-plan-sha256", required=True)
    preflight_parser.add_argument("--source-handoff-receipt-sha256", required=True)
    args = parser.parse_args()
    if args.operation == "execute":
        result = execute(args.intent.resolve())
    else:
        result = corrective_preflight(
            operation_nonce=args.operation_nonce,
            source_commit=args.source_commit,
            execution_input_sha256=args.execution_input_sha256,
            proof_plan_sha256=args.proof_plan_sha256,
            source_handoff_receipt_sha256=args.source_handoff_receipt_sha256,
        )
    if args.operation == "corrective-preflight":
        sys.stdout.buffer.write(canonical_json(result))
        sys.stdout.buffer.flush()
    else:
        print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"DX0_ERROR: {error}", file=sys.stderr, flush=True)
        raise

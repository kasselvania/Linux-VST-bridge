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
from common import (PC0_PACKET_SCHEMA, PC0_COST_SCHEMA, PC0_EVIDENCE_PATHS,
                    PC0_PROOF_CLAIMS, PC0_COMPLETE_SOURCE_SCHEMA,
                    PC0_DESIGN_BLOB, PC0_DESIGN_SHA256, PC0_BASIS_COMMIT,
                    PC0_SOURCE_PATHS)

from common import (
    PC0_REF, PC0_PLAN_ID, PC0_RESULT_SCHEMA, PC0_MODE, PC0_OPERATIONS,
    pc0_validate_contract, pc0_validate_call_facts, dx0_closed_plan,
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_AGAIN_MODULE_SHA256,
    DX0_COST_SCHEMA, DX0_DECK_EXECUTION_INPUT_SCHEMA, DX0_EVIDENCE_PATHS,
    DX0_EVIDENCE_RENDERER_SCHEMA, DX0_PACKET_SCHEMA, DX0_PLAN_ID,
    DX0_PLAN_SCHEMA, DX0_REF, DX0_RESULT_SCHEMA, DX0_RETAINED_TRANSACTION_SCHEMA,
    DX0_TRANSACTION_STATE_SCHEMA, DX0_WINDOWS_BUILD_INPUT_SCHEMA,
    RUNNER_DIGEST, canonical_json, command, command_text, dx0_complete_source,
    dx0_complete_source_sha256, dx0_evidence_renderer, dx0_identity_sha256,
    dx0_records, dx0_source_role, dx0_windows_build_input, fail, repo_root,
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
PC0_COST_V2_SCHEMA = "linux-vst-bridge-pc0-cost-and-invalidation/v2"
PC0_COST_SCHEMA = PC0_COST_V2_SCHEMA
PC0_FAILURE_DIAGNOSTIC_SCHEMA = "linux-vst-bridge-pc0-failure-diagnostic/v1"
PC0_CORRECTIVE_HISTORY_SCHEMA = "linux-vst-bridge-pc0-corrective-history/v1"
PC0_CORRECTIVE_PREFLIGHT_SCHEMA = "linux-vst-bridge-pc0-corrective-deck-preflight/v1"
PC0_FAILED_TRANSACTION = "a14bcc65d15e9fbdf15810a658b2ee87"
PC0_FAILED_JOURNAL_SHA256 = "06473755eb7ccfa2529522bb29e3fb44a5a4a67ea38fd0e797d198939e1ed966"
PC0_RECOVERY_SHA256 = "870039310c0f6ee4d0bf4044d629e6f6e7d92d2f901b7b40a60f9724bb84f9e5"
PC0_WINDOWS_BUILD_INPUT_SHA256 = "575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849"
PC0_FAILED_INPUT_SHA256 = "ae89ee61636074feac5c875bab9b8a9621e3a0c6f7fa83b6d33bc11b94e631a3"
PC0_PROOF_PLAN_SHA256 = "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248"
PC0_RUNTIME_DISCOVERY_COMMENT_ID = 5533164226
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
PC0_DIAGNOSTIC_PRIMARY_BLOCKERS = {
    "PC0_BUS_ARRANGEMENT_BLOCKED", "PC0_BUS_COUNT_BLOCKED",
    "PC0_BUS_INFO_BLOCKED", "PC0_CONTRACT_INCOMPLETE", "PC0_EVIDENCE_BLOCKED",
    "PC0_PROCESS_CLEANUP_BLOCKED", "PC0_SAMPLE_FORMAT_BLOCKED",
    "WA0_INTERFACE_QUERY_BLOCKED", "WA0_INTERFACE_QUERY_INCONSISTENT",
    "WA0_INTERFACE_RELEASE_BLOCKED", "WC0_COMPONENT_CREATE_BLOCKED",
    "WC0_COMPONENT_INITIALIZE_BLOCKED", "WC0_COMPONENT_RELEASE_BLOCKED",
    "WC0_COMPONENT_TERMINATE_BLOCKED", "WC0_CONTROLLER_ID_BLOCKED",
    "WC0_CONTROLLER_ID_MISMATCH", "WC0_HOST_CONTEXT_BLOCKED",
    "WF0_CLASS_ENUMERATION_BLOCKED", "WF0_FACTORY_GET_BLOCKED",
    "WF0_FACTORY_INFO_BLOCKED", "WF0_FACTORY_RELEASE_BLOCKED",
    "WF0_MODULE_ENTRY_BLOCKED", "WF0_MODULE_EXIT_BLOCKED",
    "WF0_MODULE_OPEN_BLOCKED", "WF0_MODULE_UNLOAD_BLOCKED",
    "WF0_OUTPUT_NORMALIZATION_BLOCKED", "WF0_PROCESS_IDENTITY_BLOCKED",
    "WF0_SCANNER_LAUNCH_BLOCKED",
}
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


def _diagnostic_coordinates(record: dict[str, Any]) -> dict[str, Any]:
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


def _validate_diagnostic_call(record: dict[str, Any], *, completed: bool) -> None:
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
    coordinates = set(_diagnostic_coordinates(record))
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


def _validate_diagnostic_callback(record: dict[str, Any],
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


def _validate_diagnostic_records(records: Any, call_counts: dict[str, Any],
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
        if (not isinstance(record, dict) or type(record.get("sequence")) is not int
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
            _validate_diagnostic_call(record, completed=False)
            observed_counts[operation] += 1
            active = record
        elif event == "call_completed":
            if (active is None or record.get("operation") != active.get("operation")
                    or record.get("interface") != active.get("interface")
                    or record.get("ordinal") != active.get("ordinal")
                    or record.get("tier") != active.get("tier")
                    or record.get("attempt_sequence") != active.get("sequence")
                    or _diagnostic_coordinates(record)
                    != _diagnostic_coordinates(active)):
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call pairing differs")
            _validate_diagnostic_call(record, completed=True)
            active = None
        else:
            callback_count += 1
            _validate_diagnostic_callback(record, active)
            if callback_count > 64:
                fail("PC0_EVIDENCE_BLOCKED: failure diagnostic callback bound differs")
    if observed_counts != call_counts:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic call counts differ")
    if last_in_flight != (None if active is None else active["operation"]) \
            or last_lifecycle != observed_lifecycle:
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic terminal joins differ")
    return records


def validate_failure_diagnostic(value: Any, *, expected_source: dict[str, Any],
                                expected_execution_input_sha256: str,
                                expected_plan_sha256: str,
                                expected_operation_nonce: str,
                                expected_phase_nonce: str) -> dict[str, Any]:
    diagnostic = _keys(value, PC0_FAILURE_DIAGNOSTIC_KEYS,
                       "PC0 failure diagnostic")
    if (diagnostic["schema"] != PC0_FAILURE_DIAGNOSTIC_SCHEMA
            or diagnostic["execution_source"] != expected_source):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic identity differs")
    for observed, expected, label, pattern in (
        (diagnostic["operation_nonce"], expected_operation_nonce,
         "operation nonce", HEX32),
        (diagnostic["phase_nonce"], expected_phase_nonce, "phase nonce", HEX32),
        (diagnostic["execution_input_sha256"], expected_execution_input_sha256,
         "execution input", HEX64),
        (diagnostic["proof_plan_sha256"], expected_plan_sha256,
         "proof plan", HEX64),
    ):
        _hex(observed, pattern, label)
        if observed != expected:
            fail(f"PC0_EVIDENCE_BLOCKED: failure diagnostic {label} differs")
    _hex(diagnostic["run_id"], HEX32, "diagnostic run ID")
    raw_exit = diagnostic["raw_exit"]
    if raw_exit is not None and (type(raw_exit) is not int or not -255 <= raw_exit <= 255):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic raw exit differs")
    classification = diagnostic["classification"]
    if (classification not in PC0_DIAGNOSTIC_CLASSIFICATIONS
            or (classification == "output_publication_failed" and raw_exit != 99)
            or (raw_exit == 99 and classification not in {
                "output_publication_failed", "supervision_failed",
                "supervision_and_process_cleanup_failed"})
            or diagnostic["primary_blocker"] not in PC0_DIAGNOSTIC_PRIMARY_BLOCKERS
            or diagnostic["secondary_cleanup_blocker"] not in {
                None, "PC0_PROCESS_CLEANUP_BLOCKED"}
            or diagnostic["last_in_flight_operation"] not in {
                None, *PC0_DIAGNOSTIC_OPERATIONS}):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic classification differs")
    if (type(diagnostic["durable_record_count"]) is not int
            or diagnostic["durable_record_count"] != len(diagnostic["durable_records"])):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic record count differs")
    _validate_diagnostic_records(
        diagnostic["durable_records"], diagnostic["call_counts"],
        diagnostic["last_in_flight_operation"], diagnostic["last_lifecycle"],
    )
    if (diagnostic["audio_processor_observer_state"] not in {
            "audio_processor_absent", "audio_processor_query_in_flight",
            "audio_processor_query_returned_without_lease",
            "audio_processor_lease_acquired", "audio_processor_release_in_flight",
            "audio_processor_lease_retired", "audio_processor_retirement_incomplete",
            "audio_processor_ownership_unknown"}
            or type(diagnostic["audio_interface_quiescence"]) is not bool):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic interface state differs")
    shutdown = _keys(diagnostic["inherited_shutdown"], {
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
                    "not_attempted_audio_interface_quiescence_unproved"}
                or disposition["source"] not in {
                    "scanner_call_ledger", "scanner_suppression_record",
                    "supervisor_unmatched_component_call",
                    "supervisor_call_ledger_absence"}):
            fail("PC0_EVIDENCE_BLOCKED: failure diagnostic shutdown entry differs")
    cleanup = _keys(diagnostic["cleanup"], {
        "owned_descendants_zero", "process_group_empty"}, "diagnostic cleanup")
    if (any(type(value) is not bool for value in cleanup.values())
            or diagnostic["environment_retirement_disposition"] not in {
                "retired", "not_attempted_process_containment_unproved", "failed"}):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic cleanup/retirement differs")
    for key in ("stdout_sha256", "stderr_sha256", "protected_snapshot_sha256",
                "runner_identity_sha256"):
        _hex(diagnostic[key], HEX64, f"diagnostic {key}")
    if (diagnostic["runner_identity_sha256"] != RUNNER_DIGEST
            or type(diagnostic["stderr_bytes"]) is not int
            or not 0 <= diagnostic["stderr_bytes"] <= 65536):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic runner/stderr differs")
    return diagnostic


def validate_failure_diagnostic_file(path: pathlib.Path, *,
                                     expected_source: dict[str, Any],
                                     expected_execution_input_sha256: str,
                                     expected_plan_sha256: str,
                                     expected_operation_nonce: str,
                                     expected_phase_nonce: str) -> dict[str, Any]:
    if (not path.is_file() or path.is_symlink() or path.stat().st_size > 2 * 1024 * 1024):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic is absent or unsafe")
    sidecar = path.with_suffix(path.suffix + ".sha256")
    if (not sidecar.is_file() or sidecar.is_symlink() or sidecar.stat().st_size > 256
            or sidecar.read_bytes() != f"{sha256_file(path)}  {path.name}\n".encode()):
        fail("PC0_EVIDENCE_BLOCKED: failure diagnostic sidecar differs")
    data = path.read_bytes()
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


def validate_corrective_preflight(value: Any, *, expected_source: dict[str, Any],
                                  expected_execution_input_sha256: str,
                                  expected_plan_sha256: str,
                                  expected_operation_nonce: str,
                                  expected_handoff: dict[str, Any]) -> dict[str, Any]:
    receipt = _keys(value, {
        "schema", "operation_nonce", "execution_source",
        "execution_input_sha256", "proof_plan_sha256", "source_handoff",
        "worktree", "process_guard", "fixture", "runner_identity_sha256",
        "protected_snapshot_sha256", "stores", "historical_failed_transaction",
        "current_corrective_absence", "write_effect_counts",
    }, "corrective Deck preflight")
    if (receipt["schema"] != PC0_CORRECTIVE_PREFLIGHT_SCHEMA
            or receipt["operation_nonce"] != expected_operation_nonce
            or receipt["execution_source"] != expected_source
            or receipt["execution_input_sha256"] != expected_execution_input_sha256
            or receipt["proof_plan_sha256"] != expected_plan_sha256):
        fail("RETURN_TO_DESIGN_GATE: corrective preflight identity differs")
    _hex(receipt["operation_nonce"], HEX32, "corrective operation nonce")
    for key in ("execution_input_sha256", "proof_plan_sha256",
                "runner_identity_sha256", "protected_snapshot_sha256"):
        _hex(receipt[key], HEX64, f"corrective preflight {key}")
    if receipt["source_handoff"] != expected_handoff:
        fail("RETURN_TO_DESIGN_GATE: corrective preflight source handoff differs")
    _typed_exact(receipt["worktree"], {
        "commit": expected_source["commit"], "detached": True, "clean": True,
    }, "corrective worktree")
    guard = _keys(receipt["process_guard"], {
        "process_counts", "prohibited_sibling_count"}, "corrective process guard")
    _typed_exact(guard["process_counts"], {
        key: 0 for key in ("bitwig", "validator", "wine", "proton", "runtime",
                           "umu", "yabridge", "wf0")
    }, "corrective process counts")
    if guard["prohibited_sibling_count"] != 0 \
            or type(guard["prohibited_sibling_count"]) is not int:
        fail("RETURN_TO_DESIGN_GATE: corrective sibling guard differs")
    _typed_exact(receipt["fixture"], {
        "hardware": "Steam Deck Galileo", "os": "SteamOS 3.8.16",
        "architecture": "x86_64", "read_only_mode": "enabled",
        "github_authority_absent": True, "forwarded_ssh_agent_absent": True,
    }, "corrective fixture")
    if receipt["runner_identity_sha256"] != RUNNER_DIGEST:
        fail("RETURN_TO_DESIGN_GATE: corrective runner identity differs")
    _typed_exact(receipt["stores"], {
        "host_artifact_manifest_sha256":
            "d0e11c374b7b1cb99b357faaa109e9310edc265c159559bd59a2098148484e9c",
        "producer_run_id": 33812659869, "producer_run_attempt": 1,
        "artifact_id": 9915439437,
        "again_bundle_manifest_sha256": DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
        "accepted_fixture_identity_sha256":
            "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6",
        "host_transfer_fallback_selected": False,
        "fixture_transfer_fallback_selected": False,
    }, "corrective stores")
    _typed_exact(receipt["historical_failed_transaction"], {
        "transaction_id": PC0_FAILED_TRANSACTION,
        "execution_input_sha256":
            "ae89ee61636074feac5c875bab9b8a9621e3a0c6f7fa83b6d33bc11b94e631a3",
        "expected_intent_sha256":
            "80510b7dfe12415e13b9af2bf29164e36fab7db99abd239cdab26d8accbe1ddc",
        "intent": "present_matched", "result": "absent",
        "result_sidecar": "absent", "inner_lock": "present_intent_matched",
        "inner_prepared_intent_sha256":
            "80510b7dfe12415e13b9af2bf29164e36fab7db99abd239cdab26d8accbe1ddc",
        "outer_lock": "present_intent_matched",
        "outer_prepared_intent_sha256":
            "80510b7dfe12415e13b9af2bf29164e36fab7db99abd239cdab26d8accbe1ddc",
        "contradictory_pc0_object_count": 0,
    }, "historical failed transaction")
    absence = _keys(receipt["current_corrective_absence"], {
        "execution_intent_absent", "reservation_absent",
        "mac_execution_lock_absent", "outer_lock_absent", "inner_lock_absent",
        "result_absent", "result_sidecar_absent", "diagnostic_absent",
        "diagnostic_sidecar_absent",
    }, "corrective absence")
    if any(value is not True for value in absence.values()):
        fail("RETURN_TO_DESIGN_GATE: current corrective absence differs")
    writes = _keys(receipt["write_effect_counts"], {
        "environment_creations", "execution_intent_publications",
        "execution_lock_creations", "result_publications",
        "diagnostic_publications", "protected_state_mutations",
        "deck_execution_reservations", "deck_execution_count_increments",
        "proton_launches",
    }, "corrective preflight writes")
    if any(type(value) is not int or value != 0 for value in writes.values()):
        fail("RETURN_TO_DESIGN_GATE: corrective preflight performed a write")
    return receipt


def validate_pre_evidence_snapshot(value: Any, *,
                                   expected_source: dict[str, Any],
                                   expected_operation_nonce: str,
                                   expected_execution_input_sha256: str,
                                   expected_result_sha256: str,
                                   expected_preflight_sha256: str) -> dict[str, Any]:
    state = _keys(value, {
        "schema", "operation_nonce", "transaction_key", "state", "source",
        "plan_sha256", "created_utc", "run_invocation_count", "phases",
        "effect_counts",
    }, "corrective pre-evidence state")
    if (state["schema"] != DX0_TRANSACTION_STATE_SCHEMA
            or state["operation_nonce"] != expected_operation_nonce
            or state["state"] != "transaction_result_retained"
            or state["source"] != expected_source
            or state["plan_sha256"]
            != "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248"
            or state["run_invocation_count"] != 1
            or type(state["run_invocation_count"]) is not int):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence state identity differs")
    effects = _keys(state["effect_counts"], {
        "windows_builds", "artifact_downloads", "custody_operations",
        "artifact_transfers", "source_transfers", "deck_executions",
        "evidence_renders",
    }, "pre-evidence effects")
    if (effects["windows_builds"] != 0 or effects["artifact_downloads"] != 0
            or effects["custody_operations"] != 0 or effects["artifact_transfers"] != 0
            or effects["source_transfers"] not in {0, 1}
            or effects["deck_executions"] != 1 or effects["evidence_renders"] != 0
            or any(type(count) is not int for count in effects.values())):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence effect facts differ")
    phases = state["phases"]
    if (not isinstance(phases, dict)
            or "render_and_validate_evidence" in phases
            or "close_transaction" in phases):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence snapshot phase boundary differs")
    transfer = phases.get("transfer_and_admit_deck_inputs")
    execute = phases.get("execute_deck_batch")
    retrieve = phases.get("retrieve_and_retain_result")
    if (not isinstance(transfer, dict) or not isinstance(transfer.get("outputs"), dict)
            or transfer["outputs"].get("corrective_preflight_sha256")
            != expected_preflight_sha256
            or not isinstance(execute, dict) or execute.get("disposition") != "completed"
            or not isinstance(retrieve, dict) or retrieve.get("disposition") != "completed"
            or retrieve.get("outputs", {}).get("retained_result_sha256")
            != expected_result_sha256):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence phase/result join differs")
    reservation = execute.get("inputs", {}).get("corrective_reservation")
    expected_reservation = {
        "kind": "v3_single_corrective",
        "historical_transaction_id": PC0_FAILED_TRANSACTION,
        "reservation_ordinal": 1,
        "additional_positive_deck_batches_maximum": 1,
        "corrective_preflight_sha256": expected_preflight_sha256,
    }
    if (reservation != expected_reservation
            or execute.get("inputs", {}).get("deck_execution_input_sha256")
            != expected_execution_input_sha256
            or execute.get("outputs") != {
                "retained_result_sha256": expected_result_sha256}):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence corrective reservation differs")
    admission = retrieve["outputs"].get("result_admission")
    if (not isinstance(admission, dict)
            or admission.get("retained_result_sha256") != expected_result_sha256
            or admission.get("operation_nonce") != expected_operation_nonce
            or admission.get("execution_input_sha256")
            != expected_execution_input_sha256):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence result admission differs")
    return state


def validate_pre_evidence_snapshot_file(path: pathlib.Path, *,
                                        expected_source: dict[str, Any],
                                        expected_operation_nonce: str,
                                        expected_execution_input_sha256: str,
                                        expected_result_sha256: str,
                                        expected_preflight_sha256: str,
                                        live_journal: pathlib.Path | None = None,
                                        require_live_equality: bool = True) -> dict[str, Any]:
    if (not path.is_file() or path.is_symlink() or path.stat().st_size > 4 * 1024 * 1024):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence snapshot is absent or unsafe")
    sidecar = path.with_suffix(path.suffix + ".sha256")
    if (not sidecar.is_file() or sidecar.is_symlink()
            or sidecar.read_bytes() != f"{sha256_file(path)}  {path.name}\n".encode()):
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence snapshot sidecar differs")
    data = path.read_bytes()
    value = parse_json_no_duplicates(data, "PC0 corrective pre-evidence state")
    if canonical_json(value) != data:
        fail("PC0_EVIDENCE_BLOCKED: pre-evidence snapshot is not canonical")
    if require_live_equality and (live_journal is None or live_journal.read_bytes() != data):
        fail("PC0_EVIDENCE_BLOCKED: live journal differs before evidence render")
    return validate_pre_evidence_snapshot(
        value, expected_source=expected_source,
        expected_operation_nonce=expected_operation_nonce,
        expected_execution_input_sha256=expected_execution_input_sha256,
        expected_result_sha256=expected_result_sha256,
        expected_preflight_sha256=expected_preflight_sha256,
    )


def _pc0_renderer(value: Any, consumer_source: dict[str, Any]) -> dict[str, Any]:
    renderer = _keys(value, {
        "schema", "record_count", "records", "evidence_schema", "evidence_paths",
    }, "PC0 evidence renderer")
    expected_source = dx0_source_role(pc0_v3_complete_source(consumer_source["commit"]))
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
    if pc0:
        expected_execution = dx0_source_role(
            pc0_v3_complete_source(result["deck_execution_source"]["commit"])
        )
        if result["deck_execution_source"] != expected_execution:
            fail("PC0_EVIDENCE_BLOCKED: repaired execution source E differs")

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


def validate_pc0_corrective_history(
        value: Any, packet: dict[str, Any],
        current_effects: dict[str, int]) -> dict[str, Any]:
    history = _keys(value, {
        "schema", "v2_failed_execution", "v2_operator_recovery",
        "v3_corrective_authority", "v3_corrective_execution",
        "cumulative_external_effect_counts", "cumulative_orchestration_counts",
    }, "PC0 corrective history")
    if history["schema"] != PC0_CORRECTIVE_HISTORY_SCHEMA:
        fail("PC0_EVIDENCE_BLOCKED: corrective-history schema differs")
    effect_keys = {
        "windows_builds", "artifact_downloads", "custody_operations",
        "artifact_transfers", "source_transfers", "deck_executions",
        "evidence_renders",
    }
    v2 = _keys(history["v2_failed_execution"], {
        "transaction_id", "journal_sha256", "source",
        "deck_execution_input_sha256", "proof_plan_sha256",
        "execute_phase_disposition", "transaction_state", "primary_blocker",
        "failure_classification", "local_result_disposition",
        "remote_preflight", "effect_counts", "driver_invocation_count",
    }, "V2 failed execution history")
    recovery = _keys(history["v2_operator_recovery"], {
        "receipt_sha256", "continuation_count",
        "original_driver_invocation_count", "original_failure_boundary",
        "disposition", "runtime_discovery_comment_id",
    }, "V2 operator recovery history")
    authority = _keys(history["v3_corrective_authority"], {
        "merge_commit", "merge_tree", "design_blob", "design_sha256",
        "review_id", "approval_blob", "prior_journal_sha256",
        "prior_recovery_receipt_sha256",
        "additional_positive_deck_batches_maximum",
        "additional_windows_builds_maximum",
    }, "V3 corrective authority history")
    execution = _keys(history["v3_corrective_execution"], {
        "transaction_id", "pre_evidence_journal_sha256", "execution_source",
        "evidence_consumer_source", "driver_invocation_count",
        "reservation_count", "effect_counts", "result_sha256",
    }, "V3 corrective execution history")
    remote = _keys(v2["remote_preflight"], {
        "result", "result_sidecar", "inner_lock", "outer_lock",
    }, "historical remote preflight")
    _source(v2["source"], "historical V2 source")
    _source(execution["execution_source"], "corrective execution source")
    _source(execution["evidence_consumer_source"], "corrective consumer source")
    expected_v2_effects = {
        "windows_builds": 1, "artifact_downloads": 1,
        "custody_operations": 1, "artifact_transfers": 1,
        "source_transfers": 1, "deck_executions": 1,
        "evidence_renders": 0,
    }
    expected_authority = {
        "merge_commit": PC0_V3_AUTHORITY_COMMIT,
        "merge_tree": PC0_V3_AUTHORITY_TREE,
        "design_blob": PC0_V3_DESIGN_BLOB,
        "design_sha256": PC0_V3_DESIGN_SHA256,
        "review_id": PC0_V3_REVIEW_ID,
        "approval_blob": PC0_V3_APPROVAL_BLOB,
        "prior_journal_sha256": PC0_FAILED_JOURNAL_SHA256,
        "prior_recovery_receipt_sha256": PC0_RECOVERY_SHA256,
        "additional_positive_deck_batches_maximum": 1,
        "additional_windows_builds_maximum": 0,
    }
    if (v2["transaction_id"] != PC0_FAILED_TRANSACTION
            or v2["journal_sha256"] != PC0_FAILED_JOURNAL_SHA256
            or v2["source"] != packet["artifact_producer_source"]
            or v2["deck_execution_input_sha256"] != PC0_FAILED_INPUT_SHA256
            or v2["proof_plan_sha256"] != PC0_PROOF_PLAN_SHA256
            or v2["execute_phase_disposition"] != "failed"
            or v2["transaction_state"] != "deck_batch_in_flight"
            or v2["primary_blocker"] != "PC0_EVIDENCE_BLOCKED"
            or v2["failure_classification"] != "unresolved_v2_no_diagnostic"
            or v2["local_result_disposition"] != "absent"
            or remote != {
                "result": "absent", "result_sidecar": "absent",
                "inner_lock": "present_intent_matched",
                "outer_lock": "present_intent_matched",
            }
            or v2["effect_counts"] != expected_v2_effects
            or v2["driver_invocation_count"] != 1):
        fail("PC0_EVIDENCE_BLOCKED: V2 failed history differs")
    if recovery != {
            "receipt_sha256": PC0_RECOVERY_SHA256,
            "continuation_count": 1,
            "original_driver_invocation_count": 1,
            "original_failure_boundary":
                "mac_execution_reservation_created_before_exact_deck_ssh_discovery",
            "disposition": "continuation_stopped",
            "runtime_discovery_comment_id": PC0_RUNTIME_DISCOVERY_COMMENT_ID,
    }:
        fail("PC0_EVIDENCE_BLOCKED: operator recovery history differs")
    if authority != expected_authority:
        fail("PC0_EVIDENCE_BLOCKED: V3 authority history differs")
    _hex(execution["transaction_id"], HEX32, "corrective transaction ID")
    _hex(execution["pre_evidence_journal_sha256"], HEX64,
         "corrective pre-evidence journal")
    if (execution["transaction_id"]
            != packet["result_admission"]["operation_nonce"]
            or execution["execution_source"] != packet["deck_execution_source"]
            or execution["evidence_consumer_source"]
            != packet["evidence_consumer_source"]
            or execution["driver_invocation_count"] != 1
            or execution["reservation_count"] != 1
            or execution["effect_counts"] != current_effects
            or execution["result_sha256"]
            != packet["admitted_private_result_sha256"]):
        fail("PC0_EVIDENCE_BLOCKED: V3 corrective execution history differs")
    cumulative = _keys(history["cumulative_external_effect_counts"],
                       effect_keys | {
                           "workflow_dispatches", "again_builds", "fixture_seeds",
                           "live_negative_exercises"},
                       "cumulative external effects")
    expected_cumulative = {
        key: expected_v2_effects[key] + current_effects[key]
        for key in effect_keys
    }
    expected_cumulative.update({
        "workflow_dispatches": 1, "again_builds": 0,
        "fixture_seeds": 0, "live_negative_exercises": 0,
    })
    orchestration = _keys(history["cumulative_orchestration_counts"], {
        "v2_driver_invocations", "operator_recovery_continuations",
        "v3_driver_invocations", "total_orchestration_entries",
    }, "cumulative orchestration counts")
    if (cumulative != expected_cumulative
            or orchestration != {
                "v2_driver_invocations": 1,
                "operator_recovery_continuations": 1,
                "v3_driver_invocations": 1,
                "total_orchestration_entries": 3,
            }):
        fail("PC0_EVIDENCE_BLOCKED: cumulative corrective costs differ")
    return history


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
        "mac_only", "deck_only", "host_only", "lost_ack_and_duplicate_work",
        "corrective_history"}
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
    validate_pc0_corrective_history(
        cost["invalidation_cases"]["corrective_history"], packet, effects
    )


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
        f"Approved pc0-design-v3: blob `{PC0_V3_DESIGN_BLOB}`, SHA-256 `{PC0_V3_DESIGN_SHA256}`; merged authority `{PC0_V3_AUTHORITY_COMMIT}`.",
        f"Producer P `{packet['artifact_producer_source']['commit']}`; repaired execution E `{packet['deck_execution_source']['commit']}`; consumer C `{consumer['commit']}`.",
        f"Observation disposition: `{admission['disposition']}`. Original private result: `{admission['retained_result_sha256']}`.",
        "The final transaction reused the accepted Windows producer and AGain fixture. Corrective history separately retains the failed V2 Deck execution, operator recovery continuation, V3 authority, and one corrective reservation.",
    ]))
    write_atomic(output / "FINDINGS.md", _markdown("PC0 findings", [
        "AGain reports Stereo In and Stereo Out, two channels each, kStereo; Event In, one channel; no event output. All three buses are main/default-active, not control-voltage. Both sample sizes are supported.",
        "Eleven PC0 calls extend the accepted lifecycle to 33 paired calls. Interface release returns 1; component release returns 0. Quiescence, factory/module shutdown, zero descendants, environment retirement and the original protected-state equality are retained machine-readably in TRANSACTION.json.",
        "Deterministic failures use the production borrower and injected event streams, not live plug-ins. The repaired source owns one corrective positive Deck observation; cumulative accounting retains both the failed V2 batch and this corrective batch.",
        "No setup, latency, tail, activation, processing, audio/event buffers, controller, state, parameter, IPC, proxy, Bitwig, Serum, packaging, signing, runner-selection or general compatibility claim.",
    ]))
    names = ("BASIS.md", "COST_AND_INVALIDATION.json", "FINDINGS.md", "TRANSACTION.json")
    write_atomic(output / "hashes.sha256", "".join(f"{sha256_file(output / name)}  {name}\n" for name in names).encode())
    return packet_identity(output)


if __name__ == "__main__":
    raise SystemExit("evidence.py is a library; use tools/host-proof.py")

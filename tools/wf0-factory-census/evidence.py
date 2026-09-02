#!/usr/bin/env python3
"""Render and validate the fixed sanitized fourteen-file WA0 evidence packet."""

from __future__ import annotations

import copy
import json
import pathlib
import re
import subprocess
from typing import Any

from common import (
    ACCEPTED_WC0_ARTIFACT_MANIFEST_SHA256, ACCEPTED_WC0_EVIDENCE_COMMIT,
    ACCEPTED_WC0_EVIDENCE_TREE, ACCEPTED_WC0_IMPLEMENTATION_MERGE,
    ACCEPTED_WC0_SCANNER_SHA256, ACCEPTED_WC0_SOURCE_COMMIT,
    ACCEPTED_WC0_SOURCE_MANIFEST_SHA256, ACCEPTED_WC0_SOURCE_TREE,
    APPROVAL_BLOB, AUTHORITY_MERGE_COMMIT, AUTHORITY_MERGE_TREE, BASIS_COMMIT,
    BASIS_TREE, DESIGN_BLOB, DESIGN_COMMIT, DESIGN_SHA256, DESIGN_TREE,
    EVIDENCE_FILES, EXPECTED_BRANCH, EXPECTED_REF, REPOSITORY,
    REVIEW_GITHUB_ID, RUNNER_DIGEST, SDK_COMMIT,
    SDK_POSITIVE_FIXTURE_BLOBS, SDK_SOURCE_BLOBS, SDK_SUBMODULES, SDK_TREE,
    SOURCE_PATHS, SOURCE_SCHEMA, WF0Error, WORKFLOW_PATH,
    canonical_json, fail, repo_root, sha256_bytes, sha256_file,
    source_manifest, source_manifest_sha256, write_atomic,
)


PACKET = pathlib.Path(
    "evidence/wa0-windows-vst3-audio-processor-interface-admission"
)
PROOF_ROWS = (
    "accepted_wc0_wf0_prerequisite_identity",
    "exact_audio_processor_interface_identity",
    "query_after_initialize_and_retirement_before_terminate",
    "four_case_query_result_output_ownership",
    "positive_exactly_one_lease_and_query",
    "audio_release_one_then_component_release_zero",
    "audio_interface_quiescence_is_tenth_wc0_gate_fact",
    "no_audio_processor_method_or_controller_path",
    "failure_null_query_cleanup",
    "success_null_query_cleanup",
    "failure_nonnull_query_cleanup",
    "query_timeout_attribution",
    "query_crash_attribution",
    "unexpected_release_count_retirement_incomplete",
    "release_timeout_attribution",
    "release_crash_attribution",
    "unproved_interface_retirement_suppresses_shutdown",
    "bounded_wc0_positive_regression",
    "nine_exercises_zero_physical_residue",
    "source_build_custody_deck_evidence_identity_and_preservation",
)

JSON_TOP_LEVEL = {
    "AUDIO_PROCESSOR_LEASE.json": {
        "schema", "implementation_source_identity", "scanner_sha256",
        "module_sha256", "owner", "component_owner_reference_baseline",
        "interface", "state", "query", "lease_acquired", "release",
        "pointer_cleared", "call_in_flight", "callback_ledger_unchanged",
        "audio_interface_quiescence", "call_attribution",
        "audio_processor_method_called", "explicit_nonclaims",
        "stage_timeline_sha256",
    },
    "COMPONENT_SESSION.json": {
        "schema", "implementation_source_identity", "scanner_sha256",
        "module_sha256", "processor", "controller", "create", "initialize",
        "audio_processor_lease", "terminate", "component_release",
        "host_reference_sequence", "callback_ledger", "component_state",
        "object_quiescence", "inherited_wf0_regression", "call_attribution",
        "controller_instance_created", "forbidden_component_method_called",
        "audio_processor_method_called", "explicit_nonclaims",
        "stage_timeline_sha256", "callback_ledger_sha256",
    },
    "STAGE_TIMELINE.json": {
        "schema", "positive", "negative_results", "negative_exercise_count",
        "total_live_exercise_count", "positive_cleanup",
        "positive_environment_retired",
    },
    "BUILD_MANIFEST.json": {
        "schema", "repository", "authority", "implementation_source_manifest",
        "implementation_source_manifest_sha256", "implementation_source_tree",
        "windows_build_receipt", "mac_artifact_custody_receipt",
        "source_handoff_receipt", "artifact_manifest", "again_bundle_manifest",
        "deck_admission", "runtime_proton_digest", "deterministic_regression",
        "audio_processor_lease_sha256",
        "component_session_sha256", "negative_fault_roster_complete",
        "positive_interface_lease_complete", "interface_quiescence",
        "object_quiescence", "zero_owned_process_environment_residue",
        "protected_state_equal", "no_deck_github", "proof_disposition",
        "proof_row_count",
    },
    "fixture.json": {
        "schema", "host", "implementation_source", "workflow", "artifact",
        "vst3_sdk", "runner_identity_sha256", "processor_logical_cid",
        "processor_raw_windows_tuid", "component_interface",
        "component_logical_iid", "component_raw_windows_tuid",
        "admitted_interface", "admitted_interface_logical_iid",
        "admitted_interface_raw_windows_tuid", "positive_result",
        "interface_quiescence", "object_quiescence",
        "audio_processor_method_called", "controller_instance_created",
        "protected_state_equal", "deck_github_operations",
    },
}
JSON_SCHEMAS = {
    "AUDIO_PROCESSOR_LEASE.json":
        "linux-vst-bridge-wa0-audio-processor-lease/v1",
    "COMPONENT_SESSION.json":
        "linux-vst-bridge-wa0-component-regression/v1",
    "STAGE_TIMELINE.json": "linux-vst-bridge-wa0-stage-timeline/v1",
    "BUILD_MANIFEST.json":
        "linux-vst-bridge-wa0-retained-build-manifest/v1",
    "fixture.json": "linux-vst-bridge-wa0-fixture/v1",
}

# The evidence schema is contextual: every object path has its own exact key
# roster.  A field admitted in one owner (for example an Actions artifact `id`)
# is not thereby admitted at any other nesting point.  The one variable-shape
# object is a sanitized call/lifecycle event; it has a closed union of keys and
# still requires `event` and `sequence`.
def _build_json_shape() -> tuple[
    dict[str, dict[str, tuple[frozenset[str], frozenset[str]]]],
    dict[str, frozenset[str]],
]:
    objects: dict[str, dict[str, tuple[frozenset[str], frozenset[str]]]] = {
        name: {} for name in JSON_TOP_LEVEL
    }
    lists: dict[str, set[str]] = {name: set() for name in JSON_TOP_LEVEL}

    def obj(name: str, path: str, keys: str, required: str | None = None) -> None:
        allowed = frozenset(keys.split())
        objects[name][path] = (
            frozenset(required.split()) if required is not None else allowed,
            allowed,
        )

    def array(name: str, *paths: str) -> None:
        lists[name].update(paths)

    for name, keys in JSON_TOP_LEVEL.items():
        allowed = frozenset(keys)
        objects[name]["$"] = (allowed, allowed)

    audio = "AUDIO_PROCESSOR_LEASE.json"
    obj(audio, "$.call_attribution", "closed_operations completed_count last_in_flight_operation new_operation_count started_count total_operation_count")
    obj(audio, "$.implementation_source_identity", "commit manifest_sha256 record_count schema tree")
    obj(audio, "$.interface", "logical_iid name pointer_identity_retained raw_windows_tuid")
    obj(audio, "$.query", "attempted output_nonnull output_zero_initialized result_u32_hex tuple_consistent")
    obj(audio, "$.release", "attempted reference_count returned_ordinary")
    array(audio, "$.call_attribution.closed_operations", "$.explicit_nonclaims")

    component = "COMPONENT_SESSION.json"
    obj(component, "$.audio_processor_lease", "pointer_cleared query_output_nonnull query_result_u32_hex quiescence release_reference_count state")
    obj(component, "$.call_attribution", "closed_operation_count completed_count last_in_flight_operation new_wa0_operation_count started_count")
    obj(component, "$.callback_ledger", "capacity closed component_originated_reference_sequence host_name output_failed overflowed owner_final_release record_count records unchanged_across_audio_processor_lease unexpected_host_object_request wrong_thread")
    obj(component, "$.callback_ledger.records[]", "enclosing_attempt_sequence enclosing_operation operation origin reference_count")
    obj(component, "$.component_release", "attempted pointer_cleared reference_count returned_ordinary")
    obj(component, "$.controller", "complete_output_zero_initialized logical_cid matched queried_before_initialize raw_windows_tuid result_u32_hex")
    obj(component, "$.create", "attempted output_nonnull result_u32_hex tuple_consistent")
    obj(component, "$.implementation_source_identity", "commit manifest_sha256 record_count schema tree")
    obj(component, "$.inherited_wf0_regression", "clean_in_process_shutdown factory_release_order factory_vendor module_exit module_unload ordered_class_census")
    obj(component, "$.inherited_wf0_regression.module_exit", "called present result")
    obj(component, "$.inherited_wf0_regression.module_unload", "attempted succeeded")
    obj(component, "$.inherited_wf0_regression.ordered_class_census[]", "category logical_class_id name ordinal raw_windows_tuid")
    obj(component, "$.initialize", "attempted result_u32_hex succeeded")
    obj(component, "$.object_quiescence", "facts value")
    obj(component, "$.object_quiescence.facts", "audio_interface_quiescent_before_terminate callback_ledger_closed component_call_in_flight component_pointer_cleared component_release_returned_zero host_callback_in_flight host_owner_final_release_returned_zero host_reference_returned_to_baseline initialize_succeeded terminate_attempted_once_and_returned_ordinary")
    obj(component, "$.processor", "logical_cid raw_windows_tuid requested_interface requested_interface_logical_iid requested_interface_raw_windows_tuid")
    obj(component, "$.terminate", "attempted result_u32_hex returned_ordinary")
    array(
        component,
        "$.callback_ledger.component_originated_reference_sequence",
        "$.callback_ledger.records", "$.explicit_nonclaims",
        "$.host_reference_sequence",
        "$.inherited_wf0_regression.factory_release_order",
        "$.inherited_wf0_regression.ordered_class_census",
    )

    timeline = "STAGE_TIMELINE.json"
    obj(timeline, "$.negative_results[]", "audio_interface_quiescence audio_processor_lease audio_processor_observer_state call_counts classification cleanup component_case component_session_closed environment_retired exercise expected_blocker fixture forbidden_audio_processor_method_marker_absent forbidden_component_method_marker_absent inherited_shutdown last_in_flight_operation observed_blocker")
    obj(timeline, "$.negative_results[].audio_processor_lease", "audio_interface_quiescence call_in_flight callback_ledger_unchanged lease_acquired pointer_cleared primary_blocker query release requested_iid_raw_tuid_hex requested_interface state")
    obj(timeline, "$.negative_results[].audio_processor_lease.query", "attempted output_nonnull output_zero_initialized result_u32_hex tuple_consistent")
    obj(timeline, "$.negative_results[].audio_processor_lease.release", "attempted reference_count returned_ordinary")
    obj(timeline, "$.negative_results[].call_counts", "count_classes create_component exit_dll free_library get_class_info_1 get_class_info_2 get_class_info_unicode get_controller_class_id get_factory_info get_plugin_factory init_dll initialize_component load_library query_audio_processor query_factory_2 query_factory_3 release_audio_processor release_component release_factory_2 release_factory_3 release_factory_base terminate_component")
    obj(timeline, "$.negative_results[].cleanup", "owned_descendants_zero process_group_empty")
    obj(timeline, "$.negative_results[].inherited_shutdown", "clean_in_process_shutdown operations physical_containment_only")
    obj(timeline, "$.negative_results[].inherited_shutdown.operations", "exit_dll free_library release_component release_factory_2 release_factory_3 release_factory_base terminate_component")
    for operation in (
        "exit_dll", "free_library", "release_component", "release_factory_2",
        "release_factory_3", "release_factory_base", "terminate_component",
    ):
        obj(timeline, f"$.negative_results[].inherited_shutdown.operations.{operation}", "disposition source")
    obj(
        timeline, "$.positive[]",
        "attempt_sequence audio_interface_quiescence audio_processor_state bool_result callback_ledger_unchanged class_count component_state controller_cid_raw_tuid_hex enclosing_attempt_sequence enclosing_operation event host_reference_count i32_result interface object_quiescence object_role operation ordinal origin output_nonnull pointer_cleared primary_blocker processor_cid_raw_tuid_hex reference_count requested_iid_raw_tuid_hex requested_interface result_u32_hex return_kind sequence state thread_role tier u32_result win32_error_u32_hex",
        required="event sequence",
    )
    obj(timeline, "$.positive_cleanup", "owned_descendants_zero process_group_empty")
    array(timeline, "$.negative_results", "$.positive")

    build = "BUILD_MANIFEST.json"
    obj(build, "$.again_bundle_manifest", "binary_safe_path records schema sha256")
    obj(build, "$.again_bundle_manifest.records[]", "path sha256 size")
    obj(build, "$.artifact_manifest", "build_identity_core_sha256 implementation_source_manifest_sha256 record_count records schema")
    obj(build, "$.artifact_manifest.records[]", "path portable_mode role sha256 size type")
    obj(build, "$.authority", "accepted_wc0 approval_blob authority_merge_commit authority_merge_tree design_blob design_commit design_sha256 design_tree implementation_basis_commit implementation_basis_tree review_github_id review_result")
    obj(build, "$.authority.accepted_wc0", "artifact_manifest_sha256 evidence_commit evidence_tree implementation_merge implementation_merge_tree scanner_sha256 source_commit source_manifest_sha256 source_tree")
    obj(build, "$.deck_admission", "artifact_cache_manifest_sha256 detached_worktree_clean detached_worktree_commit source_bundle_sha256 source_handoff_receipt_sha256 source_manifest_readback_before_each_proof source_ref")
    obj(build, "$.deterministic_regression", "audio_method_verifier_regression cases component_call_surface evidence_allowlist_regression failed passed proof_disposition_regression release_baseline_regression")
    obj(build, "$.deterministic_regression.audio_method_verifier_regression", "mutated_call_samples_rejected non_call_samples_accepted receiver_independent")
    obj(build, "$.deterministic_regression.component_call_surface", "audio_bus_parameter_state_processing_editor_calls_absent audio_processor_method_calls_absent closed_plugin_operation_count controller_creation_absent new_audio_interface_operation_count operation_call_counts")
    obj(build, "$.deterministic_regression.component_call_surface.operation_call_counts", "create_component get_controller_class_id initialize_component query_audio_processor release_audio_processor release_component terminate_component")
    obj(build, "$.deterministic_regression.evidence_allowlist_regression", "contextual_object_key_schemas contextual_scalar_rules mutation_cases public_samples_accepted rejected_count")
    obj(build, "$.deterministic_regression.proof_disposition_regression", "cases failed passed rejected_at_exact_row")
    obj(build, "$.deterministic_regression.release_baseline_regression", "accepted_counts expected_component_owner_baseline production_helper rejected_counts static_asserts_bound_to_production_helper")
    obj(build, "$.implementation_source_manifest", "commit record_count records schema")
    obj(build, "$.implementation_source_manifest.records[]", "git_blob git_mode path")
    obj(build, "$.mac_artifact_custody_receipt", "artifact authenticated_private_repository_download credentials_retained explicit_nonclaims implementation_source inner_envelope repository schema workflow")
    obj(build, "$.mac_artifact_custody_receipt.artifact", "digest expired human_facing_url id name raw_download_route raw_github_artifact_zip_sha256 raw_wrapper_sha256 rest_artifact_digest size_in_bytes upload_artifact_digest_bare url workflow_head_branch workflow_head_sha workflow_run_id")
    obj(build, "$.mac_artifact_custody_receipt.implementation_source", "branch commit manifest_sha256 parent record_count ref schema tree")
    obj(build, "$.mac_artifact_custody_receipt.inner_envelope", "artifact_manifest_sha256 build_receipt_sha256 payload_archive_sha256 record_count records")
    obj(build, "$.mac_artifact_custody_receipt.inner_envelope.records[]", "path sha256 size")
    obj(build, "$.mac_artifact_custody_receipt.repository", "full_name owner_type visibility")
    obj(build, "$.mac_artifact_custody_receipt.workflow", "conclusion event git_blob head_branch head_sha path run_attempt run_id")
    obj(build, "$.no_deck_github", "github_credentials_received github_operations ssh_agent_forwarded")
    obj(build, "$.proof_disposition[]", "claim evidence result row")
    obj(build, "$.source_handoff_receipt", "bundle implementation_source repository schema wa0_authority")
    obj(build, "$.source_handoff_receipt.bundle", "advertised_ref git_bundle_verify max_size_bytes name prerequisite_count self_contained sha256 size")
    obj(build, "$.source_handoff_receipt.implementation_source", "branch commit manifest_record_count manifest_schema manifest_sha256 parent ref tree")
    obj(build, "$.source_handoff_receipt.wa0_authority", "design_authority_commit design_authority_tree implementation_basis_commit implementation_basis_tree")
    obj(build, "$.windows_build_receipt", "artifacts build explicit_nonclaims repository runner schema source tool_receipts toolchain vst3_sdk workflow")
    obj(build, "$.windows_build_receipt.artifacts", "again_bundle_roster artifact_manifest_schema artifact_manifest_sha256 copied_runtime_dependencies payload_archive_sha256 pe_receipts scanner_and_fault_roster")
    obj(build, "$.windows_build_receipt.artifacts.again_bundle_roster[]", "path sha256 size")
    obj(build, "$.windows_build_receipt.artifacts.pe_receipts[]", "dumpbin_sha256 entry_point_present exports imports machine path pe_kind sha256 size")
    obj(build, "$.windows_build_receipt.artifacts.scanner_and_fault_roster[]", "path sha256 size")
    obj(build, "$.windows_build_receipt.build", "build_a build_b cmake_options_sha256 comparison configuration dependency_network_during_configure_build deterministic_build_root_mapping msbuild_max_cpu_count msvc_post_options msvc_runtime pdb_embedded_path source_date_epoch targets")
    obj(build, "$.windows_build_receipt.build.build_a", "artifact_manifest_sha256")
    obj(build, "$.windows_build_receipt.build.build_b", "artifact_manifest_sha256")
    obj(build, "$.windows_build_receipt.build.comparison", "level manifest_sha256 path_count")
    obj(build, "$.windows_build_receipt.repository", "full_name owner_type visibility")
    obj(build, "$.windows_build_receipt.runner", "architecture image_os image_version requested_label windows_build windows_product_name windows_version")
    obj(build, "$.windows_build_receipt.source", "branch commit implementation_source_manifest parent ref tree")
    obj(build, "$.windows_build_receipt.source.implementation_source_manifest", "record_count schema sha256")
    obj(build, "$.windows_build_receipt.tool_receipts", "cl_bv_sha256")
    obj(build, "$.windows_build_receipt.toolchain", "cl_bv_sha256 cl_version cmake_version generator generator_platform generator_toolset host_architecture link_version platform_toolset target_architecture visual_studio_edition visual_studio_installation_sha256 visual_studio_version vscmd_version windows_sdk_version")
    obj(build, "$.windows_build_receipt.workflow", "commit event git_blob path ref run_attempt run_id")
    array(
        build, "$.again_bundle_manifest.records", "$.artifact_manifest.records",
        "$.deterministic_regression.cases",
        "$.deterministic_regression.evidence_allowlist_regression.mutation_cases",
        "$.deterministic_regression.proof_disposition_regression.cases",
        "$.deterministic_regression.proof_disposition_regression.rejected_at_exact_row",
        "$.deterministic_regression.release_baseline_regression.accepted_counts",
        "$.deterministic_regression.release_baseline_regression.rejected_counts",
        "$.implementation_source_manifest.records",
        "$.mac_artifact_custody_receipt.explicit_nonclaims",
        "$.mac_artifact_custody_receipt.inner_envelope.records",
        "$.proof_disposition",
        "$.windows_build_receipt.artifacts.again_bundle_roster",
        "$.windows_build_receipt.artifacts.copied_runtime_dependencies",
        "$.windows_build_receipt.artifacts.pe_receipts",
        "$.windows_build_receipt.artifacts.pe_receipts[].exports",
        "$.windows_build_receipt.artifacts.pe_receipts[].imports",
        "$.windows_build_receipt.artifacts.scanner_and_fault_roster",
        "$.windows_build_receipt.build.targets",
        "$.windows_build_receipt.explicit_nonclaims",
    )

    fixture = "fixture.json"
    obj(fixture, "$.artifact", "id manifest_sha256 name")
    obj(fixture, "$.host", "architecture hardware os read_only_mode")
    obj(fixture, "$.implementation_source", "commit manifest_sha256 parent record_count schema tree")
    obj(fixture, "$.workflow", "git_blob path run_attempt run_id")

    def sdk_schema(name: str, base: str) -> None:
        obj(name, base, "checkout_regression dirty_path_count failure_source_blobs positive_fixture_source_blobs root_checkout_configuration root_commit root_tree source_patch_count submodules")
        obj(name, f"{base}.checkout_regression", "canonical_lf_blob canonical_lf_sha256 canonical_submodule_blob canonical_submodule_sha256 command_scoped_configuration filtered_crlf_blob_masked_mismatch hostile_ambient_core_autocrlf raw_crlf_sha256_rejected recursive_submodule_checkout_canonical_lf recursive_submodule_local_configuration root_checkout_canonical_lf root_local_configuration schema unfiltered_crlf_blob_rejected")
        for config in (
            "command_scoped_configuration", "recursive_submodule_local_configuration",
            "root_local_configuration",
        ):
            obj(name, f"{base}.checkout_regression.{config}", "core.autocrlf core.eol")
        obj(name, f"{base}.root_checkout_configuration", "core.autocrlf core.eol")
        for roster in ("failure_source_blobs", "positive_fixture_source_blobs"):
            obj(name, f"{base}.{roster}[]", "git_blob path repository repository_commit repository_path sha256 unfiltered_worktree_git_blob")
        obj(name, f"{base}.submodules[]", "checkout_configuration commit path")
        obj(name, f"{base}.submodules[].checkout_configuration", "core.autocrlf core.eol")
        array(
            name, f"{base}.failure_source_blobs",
            f"{base}.positive_fixture_source_blobs", f"{base}.submodules",
        )

    sdk_schema(build, "$.windows_build_receipt.vst3_sdk")
    sdk_schema(fixture, "$.vst3_sdk")
    return objects, {name: frozenset(paths) for name, paths in lists.items()}


JSON_OBJECT_RULES, JSON_LIST_PATHS = _build_json_shape()

MARKDOWN_HEADINGS = {
    "BASIS.md": "# WA0 implementation basis\n",
    "BUILD.md": "# WA0 Windows build and custody\n",
    "ENVIRONMENT.md": "# WA0 disposable environments\n",
    "LAUNCH_AND_PROCESS.md": "# WA0 interface lease and process result\n",
    "NEGATIVE_TESTS.md": "# WA0 focused negative proofs\n",
    "PRESERVATION.md": "# WA0 protected-state preservation\n",
    "FINDINGS.md": "# WA0 findings and claim ceiling\n",
    "SANITIZATION.md": "# WA0 evidence sanitization\n",
}


def md(title: str, body: str) -> bytes:
    return f"# {title}\n\n{body.rstrip()}\n".encode("utf-8")


PRIVATE_SCALAR = re.compile(
    r"(?:/home/|/Users/|/private/|/tmp/|/var/folders/|/run/user/|"
    r"BEGIN (?:RSA |OPENSSH )?PRIVATE KEY|github_pat_|ghp_|gho_|ghu_|ghs_|"
    r"SSH_AUTH_SOCK|\.ssh/|compatdata|(?:^|[^A-Za-z])PID(?:[^A-Za-z]|$)|"
    r"0x[0-9a-fA-F]{8,16}|(?:^|[^0-9])(?:[0-9]{1,3}\.){3}[0-9]{1,3}"
    r"(?:[^0-9]|$)|(?:^|[^0-9A-Fa-f])(?:[0-9A-Fa-f]{2}:){5}"
    r"[0-9A-Fa-f]{2}(?:[^0-9A-Fa-f]|$)|"
    r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,})",
    re.IGNORECASE,
)
PUBLIC_URLS = (
    re.compile(
        r"https://github\.com/kasselvania/Linux-VST-bridge/actions/runs/"
        r"[1-9][0-9]*/artifacts/[1-9][0-9]*"
    ),
    re.compile(
        r"https://api\.github\.com/repos/kasselvania/Linux-VST-bridge/"
        r"actions/artifacts/[1-9][0-9]*"
    ),
)
PUBLIC_API_ROUTES = re.compile(
    r"/repos/kasselvania/Linux-VST-bridge/actions/artifacts/"
    r"[1-9][0-9]*/zip"
)


OPERATION_VALUES = frozenset({
    "addRef", "release", "load_library", "init_dll", "get_plugin_factory",
    "get_factory_info", "query_factory_2", "query_factory_3", "count_classes",
    "get_class_info_unicode", "get_class_info_2", "get_class_info_1",
    "create_component", "get_controller_class_id", "initialize_component",
    "query_audio_processor", "release_audio_processor", "terminate_component",
    "release_component", "release_factory_3", "release_factory_2",
    "release_factory_base", "exit_dll", "free_library",
})
STATE_VALUES = frozenset({
    "audio_processor_absent", "audio_processor_query_in_flight",
    "audio_processor_query_returned_without_lease",
    "audio_processor_lease_acquired", "audio_processor_release_in_flight",
    "audio_processor_lease_retired", "audio_processor_retirement_incomplete",
    "audio_processor_ownership_unknown", "audio_interface_quiescence_proved",
    "readiness_announced", "scanner_started", "supervisor_gate_accepted",
    "module_open_started", "module_opened", "module_entry_succeeded",
    "factory_export_found", "factory_get_started", "factory_obtained",
    "factory_info_obtained", "factory_interface_versions_recorded",
    "class_count_obtained", "class_enumeration_in_progress",
    "class_enumeration_complete", "component_create_in_flight",
    "component_created", "controller_id_in_flight", "controller_id_verified",
    "host_context_ready", "component_initialize_in_flight",
    "component_initialized", "component_terminate_in_flight",
    "component_terminated", "component_release_in_flight", "component_released",
    "component_session_closed", "object_quiescence_proved",
    "module_exit_succeeded", "module_unloaded", "scanner_completed",
})
BLOCKER_VALUES = frozenset({
    "WA0_INTERFACE_QUERY_BLOCKED", "WA0_INTERFACE_QUERY_INCONSISTENT",
    "WA0_INTERFACE_RELEASE_BLOCKED",
})
BOOL_FIELDS = frozenset({
    "attempted", "audio_interface_quiescence",
    "audio_interface_quiescent_before_terminate", "audio_processor_method_called",
    "authenticated_private_repository_download", "bool_result", "called",
    "callback_ledger_closed", "callback_ledger_unchanged", "call_in_flight",
    "clean_in_process_shutdown", "closed", "complete_output_zero_initialized",
    "component_call_in_flight", "component_pointer_cleared",
    "component_release_returned_zero", "component_session_closed",
    "controller_creation_absent", "controller_instance_created",
    "credentials_retained", "dependency_network_during_configure_build",
    "detached_worktree_clean", "entry_point_present", "environment_retired",
    "expired", "filtered_crlf_blob_masked_mismatch", "present",
    "forbidden_audio_processor_method_marker_absent",
    "forbidden_component_method_called",
    "forbidden_component_method_marker_absent", "github_credentials_received",
    "host_callback_in_flight", "host_owner_final_release_returned_zero",
    "host_reference_returned_to_baseline", "initialize_succeeded",
    "interface_quiescence", "lease_acquired", "matched", "object_quiescence",
    "output_failed", "output_nonnull", "output_null", "output_zero_initialized",
    "overflowed", "owned_descendants_zero", "physical_containment_only",
    "pointer_cleared", "pointer_identity_retained", "positive_environment_retired",
    "negative_fault_roster_complete", "positive_interface_lease_complete",
    "process_group_empty",
    "protected_state_equal", "queried_before_initialize",
    "query_output_nonnull", "quiescence",
    "raw_crlf_sha256_rejected", "receiver_independent",
    "recursive_submodule_checkout_canonical_lf", "returned_ordinary",
    "root_checkout_canonical_lf", "self_contained", "ssh_agent_forwarded",
    "static_asserts_bound_to_production_helper", "succeeded",
    "terminate_attempted_once_and_returned_ordinary", "tuple_consistent",
    "unchanged_across_audio_processor_lease", "unexpected_host_object_request",
    "unfiltered_crlf_blob_rejected", "value", "wrong_thread",
    "zero_owned_process_environment_residue",
    "audio_processor_method_calls_absent",
    "audio_bus_parameter_state_processing_editor_calls_absent",
    "contextual_object_key_schemas", "contextual_scalar_rules",
    "public_samples_accepted", "source_manifest_readback_before_each_proof",
})
INTEGER_FIELDS = frozenset({
    "attempt_sequence", "capacity", "class_count", "closed_operation_count",
    "closed_plugin_operation_count", "completed_count",
    "component_owner_reference_baseline", "deck_github_operations",
    "dirty_path_count",
    "enclosing_attempt_sequence",
    "expected_component_owner_baseline", "failed", "github_operations",
    "host_reference_count", "i32_result", "manifest_record_count", "max_size_bytes",
    "msbuild_max_cpu_count", "mutated_call_samples_rejected",
    "negative_exercise_count", "new_audio_interface_operation_count",
    "new_operation_count", "new_wa0_operation_count", "non_call_samples_accepted",
    "ordinal", "owner_final_release", "passed", "path_count", "prerequisite_count",
    "proof_row_count", "record_count", "reference_count", "release_reference_count",
    "rejected_count", "row", "sequence", "size", "source_patch_count",
    "started_count", "total_live_exercise_count", "total_operation_count",
    "u32_result", "component_originated_reference_sequence",
    "host_reference_sequence", "accepted_counts", "rejected_counts",
    "rejected_at_exact_row",
    *OPERATION_VALUES,
})
NULLABLE_PATHS = frozenset({
    "AUDIO_PROCESSOR_LEASE.json:$.call_attribution.last_in_flight_operation",
    "COMPONENT_SESSION.json:$.call_attribution.last_in_flight_operation",
    "COMPONENT_SESSION.json:$.callback_ledger.records[].enclosing_attempt_sequence",
    "COMPONENT_SESSION.json:$.callback_ledger.records[].enclosing_operation",
    "STAGE_TIMELINE.json:$.negative_results[].audio_processor_lease",
    "STAGE_TIMELINE.json:$.negative_results[].last_in_flight_operation",
})
POSITIVE_NULLABLE_FIELDS = frozenset({
    "enclosing_attempt_sequence", "enclosing_operation", "interface", "ordinal",
    "primary_blocker", "tier",
})

HEX40 = re.compile(r"[0-9a-f]{40}")
SHA256 = re.compile(r"[0-9a-f]{64}")
TYPED_SHA256 = re.compile(r"sha256:[0-9a-f]{64}")
DECIMAL = re.compile(r"[1-9][0-9]*")
VERSION = re.compile(r"[0-9]+(?:\.[0-9]+){1,3}")
FAULT_NAMES = frozenset({
    "wa0-query-failure-null", "wa0-query-success-null",
    "wa0-query-failure-nonnull", "wa0-query-hang", "wa0-query-crash",
    "wa0-release-unexpected-count", "wa0-release-hang",
    "wa0-release-crash",
})
AUDIO_NONCLAIMS = frozenset({
    "no_audio_processor_method", "no_interface_pointer_identity",
    "no_controller_creation", "no_connection_point", "no_bus_access",
    "no_parameter_access", "no_state_access", "no_processing_setup",
    "no_audio", "no_events", "no_editor", "no_proxy", "no_ipc",
    "no_bitwig", "no_serum",
})
COMPONENT_NONCLAIMS = frozenset({
    "no_controller_creation", "no_connection_point",
    "no_audio_processor_method", "no_bus_host_call",
    "no_parameter_host_call", "no_state_host_call", "no_processing",
    "no_audio", "no_events", "no_editor", "no_bitwig", "no_serum",
    "no_proxy", "no_ipc",
})
CUSTODY_NONCLAIMS = frozenset({
    "no_cryptographic_provenance", "no_trusted_builder", "no_slsa",
    "no_code_signing", "no_release_signing",
})
WINDOWS_BUILD_NONCLAIMS = frozenset({
    "no_windows_runtime_proof", "no_windows_binary_execution", "no_proton",
    "no_iaudioprocessor_method", "no_bitwig", "no_serum",
    "no_release_build", "no_reproducible_runner_image_claim",
})
DETERMINISTIC_CASES = frozenset({
    "closed_operation_mapping_22_of_22", "completion_tuple_validation",
    "sequence_gap_rejection", "second_completion_rejection",
    "relative_module_path_rejection", "default_search_flag_lock",
    "non_null_hfile_prohibited", "exact_seven_call_component_surface",
    "unload_failure_adapter_mapping_compiled",
    "runtime_entrypoint_exec_identity_continuity",
    "runtime_root_included_in_topology_role_census",
    "closed_sdk_encoding_label_mapping",
    "controlled_git_checkout_eol_regression",
    "typed_actions_digest_regression",
    "seven_component_and_audio_unmatched_attributions",
    "eight_state_audio_lease_lifecycle_lock",
    "single_audio_query_and_release_call_sites",
    "external_callback_sink_explicitly_closed",
    "audio_interface_quiescence_shutdown_gate",
    "production_release_baseline_zero_and_multi_reference_rejection",
    "receiver_independent_audio_method_mutation_rejection",
    "evidence_schema_value_allowlist_mutation_rejection",
    "exact_again_fixture_and_serial_msvc_build_lock",
})
EVIDENCE_ALLOWLIST_MUTATION_CASES = frozenset({
    "unknown_root_id", "private_host_name", "numeric_artifact_id_type",
    "traversal_artifact_path", "unallowlisted_url",
    "email_in_allowed_name_context", "ip_in_allowed_name_context",
    "pointer_in_allowed_name_context", "wrong_context_architecture",
    "wrong_context_read_only_mode", "wrong_context_interface_name",
    "wrong_context_branch_ref", "wrong_context_lifecycle_state",
    "wrong_context_repository_identity",
})
PROOF_MUTATION_CASES = frozenset({
    # The first five labels are accepted only so an older packet can be
    # rejected on semantic-regression identity rather than scalar syntax.
    "audio_pointer_retirement", "static_forbidden_call_surface",
    "production_release_baseline", "accepted_wc0_reference_sequence",
    "source_build_custody_deck_join",
    "runtime_prerequisite_digest", "fixture_interface_identity",
    "positive_lifecycle_cardinality", "positive_query_tuple_consistency",
    "negative_query_tuple_consistency", "exact_lease_call_attribution",
    "audio_release_ordinary_return", "callback_ledger_unchanged",
    "component_audio_method_absence", "fixture_audio_method_absence",
    "failure_null_cleanup", "success_null_cleanup",
    "failure_nonnull_cleanup", "query_timeout_suppression",
    "query_crash_suppression", "unexpected_release_attempt",
    "release_timeout_suppression", "release_crash_suppression",
    "all_blocked_shutdown_suppression", "accepted_wc0_processor_identity",
    "accepted_wc0_call_counts", "accepted_wc0_class_census",
    "physical_cleanup", "source_bundle_digest_join",
    "source_handoff_receipt_join", "source_build_custody_join",
    "workflow_identity_join",
    "artifact_identity_join", "runner_fixture_join",
})
PROOF_EVIDENCE = frozenset({
    "BUILD_MANIFEST.authority.accepted_wc0",
    "AUDIO_PROCESSOR_LEASE.interface",
    "STAGE_TIMELINE.positive lifecycle order",
    "positive lease and three query-tuple fixture receipts",
    "AUDIO_PROCESSOR_LEASE and positive call ledger",
    "audio lease and component release receipts",
    "COMPONENT_SESSION.object_quiescence.facts",
    "positive nonclaims and all nine marker checks",
    "wa0-query-failure-null retained lease and shutdown",
    "wa0-query-success-null retained lease and shutdown",
    "wa0-query-failure-nonnull retained cleanup",
    "wa0-query-hang observer and suppression receipt",
    "wa0-query-crash observer and suppression receipt",
    "wa0-release-unexpected-count retained retirement state",
    "wa0-release-hang observer and suppression receipt",
    "wa0-release-crash observer and suppression receipt",
    "five blocked interface-retirement shutdown ledgers",
    "COMPONENT_SESSION accepted-WC0 bounded regression",
    "STAGE_TIMELINE nine exercise cleanup receipts",
    "source/build/custody/Deck/evidence identity and preservation joins",
})
AGAIN_BUNDLE_PATHS = frozenset({
    "Contents/Resources/Snapshots/41347FD6FED64094AFBB12B7DBA1D441_snapshot.png",
    "Contents/Resources/Snapshots/41347FD6FED64094AFBB12B7DBA1D441_snapshot_2.0x.png",
    "Contents/Resources/Snapshots/84E8DE5F92554F5396FAE4133C935A18_snapshot.png",
    "Contents/Resources/Snapshots/84E8DE5F92554F5396FAE4133C935A18_snapshot_2.0x.png",
    "Contents/Resources/again.uidesc", "Contents/Resources/background.png",
    "Contents/Resources/slider_background.png",
    "Contents/Resources/slider_handle.png",
    "Contents/Resources/slider_handle_2.0x.png",
    "Contents/Resources/vu_off.png", "Contents/Resources/vu_on.png",
    "Contents/x86_64-win/again.vst3", "PlugIn.ico", "desktop.ini",
})
ARTIFACT_PATHS = frozenset({
    "BUILD_IDENTITY_CORE.json",
    *(f"again.vst3/{path}" for path in AGAIN_BUNDLE_PATHS),
    "bin/wf0-factory-probe.exe", "bin/wf0-loader-adapter-tests.exe",
    *(f"fixtures/{name}.dll" for name in FAULT_NAMES),
    "fixtures/wf0-no-entry.dll", "licenses/vst3-base.txt",
    "licenses/vst3-cmake.txt", "licenses/vst3-doc.txt",
    "licenses/vst3-pluginterfaces.txt", "licenses/vst3-public-sdk.txt",
    "licenses/vst3sdk.txt", "licenses/vstgui.txt",
})
PE_PATHS = frozenset({
    "bin/wf0-factory-probe.exe", "bin/wf0-loader-adapter-tests.exe",
    *(f"fixtures/{name}.dll" for name in FAULT_NAMES),
    "fixtures/wf0-no-entry.dll",
    "again.vst3/Contents/x86_64-win/again.vst3",
})
PE_EXPORTS = frozenset({"ExitDll", "GetPluginFactory", "InitDll"})
PE_IMPORTS = frozenset({
    "advapi32.dll", "d2d1.dll", "d3d11.dll", "dwmapi.dll", "dwrite.dll",
    "gdi32.dll", "imm32.dll", "kernel32.dll", "ole32.dll",
    "opengl32.dll", "shell32.dll", "shlwapi.dll", "user32.dll",
})
BUILD_TARGETS = frozenset({
    "wf0-factory-probe", "wf0-loader-adapter-tests", "wa0-fault-fixtures",
    "again",
})
SDK_REPOSITORIES = frozenset({
    "root", "base", "cmake", "pluginterfaces", "public.sdk", "vstgui4",
})
SDK_COMMITS = frozenset({SDK_COMMIT, *SDK_SUBMODULES.values()})
SDK_SOURCE_PATHS = frozenset(SDK_SOURCE_BLOBS)
SDK_POSITIVE_PATHS = frozenset(SDK_POSITIVE_FIXTURE_BLOBS)
SDK_BLOBS = frozenset(
    blob for blob, _ in (*SDK_SOURCE_BLOBS.values(),
                         *SDK_POSITIVE_FIXTURE_BLOBS.values())
)
SDK_SHA256S = frozenset(
    digest for _, digest in (*SDK_SOURCE_BLOBS.values(),
                             *SDK_POSITIVE_FIXTURE_BLOBS.values())
)


def _sdk_scalar_allowed(path: str, prefix: str, value: str) -> bool | None:
    """Return the exact SDK-context decision, or None outside that context."""

    if not path.startswith(prefix + "."):
        return None
    suffix = path[len(prefix):]
    exact = {
        ".root_commit": {SDK_COMMIT},
        ".root_tree": {SDK_TREE},
        ".checkout_regression.schema": {
            "linux-vst-bridge-wf0-git-checkout-regression/v1"
        },
        ".checkout_regression.command_scoped_configuration.core.autocrlf":
            {"false"},
        ".checkout_regression.command_scoped_configuration.core.eol": {"lf"},
        ".checkout_regression.recursive_submodule_local_configuration.core.autocrlf":
            {"false"},
        ".checkout_regression.recursive_submodule_local_configuration.core.eol":
            {"lf"},
        ".checkout_regression.root_local_configuration.core.autocrlf":
            {"false"},
        ".checkout_regression.root_local_configuration.core.eol": {"lf"},
        ".checkout_regression.hostile_ambient_core_autocrlf": {"true"},
        ".root_checkout_configuration.core.autocrlf": {"false"},
        ".root_checkout_configuration.core.eol": {"lf"},
        ".submodules[].checkout_configuration.core.autocrlf": {"false"},
        ".submodules[].checkout_configuration.core.eol": {"lf"},
        ".submodules[].path": set(SDK_SUBMODULES),
        ".submodules[].commit": set(SDK_SUBMODULES.values()),
        ".failure_source_blobs[].path": set(SDK_SOURCE_PATHS),
        ".positive_fixture_source_blobs[].path": set(SDK_POSITIVE_PATHS),
        ".failure_source_blobs[].repository": set(SDK_REPOSITORIES),
        ".positive_fixture_source_blobs[].repository": {"public.sdk"},
        ".failure_source_blobs[].repository_commit": set(SDK_COMMITS),
        ".positive_fixture_source_blobs[].repository_commit":
            {SDK_SUBMODULES["public.sdk"]},
        ".failure_source_blobs[].git_blob": set(SDK_BLOBS),
        ".failure_source_blobs[].unfiltered_worktree_git_blob": set(SDK_BLOBS),
        ".positive_fixture_source_blobs[].git_blob": set(SDK_BLOBS),
        ".positive_fixture_source_blobs[].unfiltered_worktree_git_blob":
            set(SDK_BLOBS),
        ".failure_source_blobs[].sha256": set(SDK_SHA256S),
        ".positive_fixture_source_blobs[].sha256": set(SDK_SHA256S),
    }
    if suffix in exact:
        return value in exact[suffix]
    if suffix in {
        ".checkout_regression.canonical_lf_blob",
        ".checkout_regression.canonical_submodule_blob",
    }:
        return HEX40.fullmatch(value) is not None
    if suffix in {
        ".checkout_regression.canonical_lf_sha256",
        ".checkout_regression.canonical_submodule_sha256",
    }:
        return SHA256.fullmatch(value) is not None
    if suffix in {
        ".failure_source_blobs[].repository_path",
        ".positive_fixture_source_blobs[].repository_path",
    }:
        return _safe_relative(value)
    return False


def _contextual_scalar_allowed(filename: str, path: str, value: str) -> bool:
    """Closed filename/path-aware scalar grammar for the five JSON records."""

    key = (filename, path)
    exact: dict[tuple[str, str], frozenset[str]] = {
        ("AUDIO_PROCESSOR_LEASE.json", "$.schema"):
            frozenset({JSON_SCHEMAS["AUDIO_PROCESSOR_LEASE.json"]}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.owner"):
            frozenset({"AudioProcessorInterfaceLease"}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.interface.name"):
            frozenset({"Steinberg::Vst::IAudioProcessor"}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.interface.logical_iid"):
            frozenset({"42043F99B7DA453CA569E79D9AAEC33D"}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.interface.raw_windows_tuid"):
            frozenset({"993F0442DAB73C45A569E79D9AAEC33D"}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.state"):
            frozenset({"audio_processor_lease_retired"}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.query.result_u32_hex"):
            frozenset({"00000000"}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.call_attribution.closed_operations[]"):
            frozenset({"query_audio_processor", "release_audio_processor"}),
        ("AUDIO_PROCESSOR_LEASE.json", "$.explicit_nonclaims[]"):
            AUDIO_NONCLAIMS,
        ("COMPONENT_SESSION.json", "$.schema"):
            frozenset({JSON_SCHEMAS["COMPONENT_SESSION.json"]}),
        ("COMPONENT_SESSION.json", "$.audio_processor_lease.state"):
            frozenset({"audio_processor_lease_retired"}),
        ("COMPONENT_SESSION.json", "$.audio_processor_lease.query_result_u32_hex"):
            frozenset({"00000000"}),
        ("COMPONENT_SESSION.json", "$.callback_ledger.host_name"):
            frozenset({"Linux VST Bridge WC0"}),
        ("COMPONENT_SESSION.json", "$.callback_ledger.records[].enclosing_operation"):
            frozenset({"initialize_component", "terminate_component"}),
        ("COMPONENT_SESSION.json", "$.callback_ledger.records[].operation"):
            frozenset({"addRef", "release"}),
        ("COMPONENT_SESSION.json", "$.callback_ledger.records[].origin"):
            frozenset({"component", "owner_local"}),
        ("COMPONENT_SESSION.json", "$.component_state"):
            frozenset({"component_released"}),
        ("COMPONENT_SESSION.json", "$.controller.logical_cid"):
            frozenset({"D39D5B65D7AF42FA843F4AC841EB04F0"}),
        ("COMPONENT_SESSION.json", "$.controller.raw_windows_tuid"):
            frozenset({"655B9DD3AFD7FA42843F4AC841EB04F0"}),
        ("COMPONENT_SESSION.json", "$.processor.logical_cid"):
            frozenset({"84E8DE5F92554F5396FAE4133C935A18"}),
        ("COMPONENT_SESSION.json", "$.processor.raw_windows_tuid"):
            frozenset({"5FDEE8845592534F96FAE4133C935A18"}),
        ("COMPONENT_SESSION.json", "$.processor.requested_interface"):
            frozenset({"Steinberg::Vst::IComponent"}),
        ("COMPONENT_SESSION.json", "$.processor.requested_interface_logical_iid"):
            frozenset({"E831FF31F2D54301928EBBEE25697802"}),
        ("COMPONENT_SESSION.json", "$.processor.requested_interface_raw_windows_tuid"):
            frozenset({"31FF31E8D5F20143928EBBEE25697802"}),
        ("COMPONENT_SESSION.json", "$.controller.result_u32_hex"):
            frozenset({"00000000"}),
        ("COMPONENT_SESSION.json", "$.create.result_u32_hex"):
            frozenset({"00000000"}),
        ("COMPONENT_SESSION.json", "$.initialize.result_u32_hex"):
            frozenset({"00000000"}),
        ("COMPONENT_SESSION.json", "$.terminate.result_u32_hex"):
            frozenset({"00000000"}),
        ("COMPONENT_SESSION.json", "$.explicit_nonclaims[]"):
            COMPONENT_NONCLAIMS,
        ("COMPONENT_SESSION.json", "$.inherited_wf0_regression.factory_release_order[]"):
            frozenset({"release_factory_3", "release_factory_2",
                       "release_factory_base"}),
        ("COMPONENT_SESSION.json", "$.inherited_wf0_regression.factory_vendor"):
            frozenset({"Steinberg Media Technologies"}),
        ("COMPONENT_SESSION.json", "$.inherited_wf0_regression.ordered_class_census[].category"):
            frozenset({"Audio Module Class", "Component Controller Class"}),
        ("COMPONENT_SESSION.json", "$.inherited_wf0_regression.ordered_class_census[].logical_class_id"):
            frozenset({"84E8DE5F92554F5396FAE4133C935A18",
                       "D39D5B65D7AF42FA843F4AC841EB04F0",
                       "41347FD6FED64094AFBB12B7DBA1D441"}),
        ("COMPONENT_SESSION.json", "$.inherited_wf0_regression.ordered_class_census[].raw_windows_tuid"):
            frozenset({"5FDEE8845592534F96FAE4133C935A18",
                       "655B9DD3AFD7FA42843F4AC841EB04F0",
                       "D67F3441D6FE9440AFBB12B7DBA1D441"}),
        ("COMPONENT_SESSION.json", "$.inherited_wf0_regression.ordered_class_census[].name"):
            frozenset({"AGain VST3", "AGain VST3Controller",
                       "AGain SideChain VST3"}),
        ("STAGE_TIMELINE.json", "$.schema"):
            frozenset({JSON_SCHEMAS["STAGE_TIMELINE.json"]}),
        ("STAGE_TIMELINE.json", "$.negative_results[].audio_processor_lease.primary_blocker"):
            BLOCKER_VALUES,
        ("STAGE_TIMELINE.json", "$.negative_results[].audio_processor_lease.query.result_u32_hex"):
            frozenset({"00000000", "80004002"}),
        ("STAGE_TIMELINE.json", "$.negative_results[].audio_processor_lease.requested_iid_raw_tuid_hex"):
            frozenset({"993F0442DAB73C45A569E79D9AAEC33D"}),
        ("STAGE_TIMELINE.json", "$.negative_results[].audio_processor_lease.requested_interface"):
            frozenset({"Steinberg::Vst::IAudioProcessor"}),
        ("STAGE_TIMELINE.json", "$.negative_results[].audio_processor_lease.state"):
            frozenset({"audio_processor_query_returned_without_lease",
                       "audio_processor_lease_retired",
                       "audio_processor_retirement_incomplete"}),
        ("STAGE_TIMELINE.json", "$.negative_results[].audio_processor_observer_state"):
            frozenset({"audio_processor_query_returned_without_lease",
                       "audio_processor_lease_retired",
                       "audio_processor_retirement_incomplete",
                       "audio_processor_ownership_unknown"}),
        ("STAGE_TIMELINE.json", "$.negative_results[].classification"):
            frozenset({"scanner_blocked", "call_timeout",
                       "abnormal_termination_in_flight"}),
        ("STAGE_TIMELINE.json", "$.negative_results[].component_case"):
            frozenset({"exact-again"}),
        ("STAGE_TIMELINE.json", "$.negative_results[].exercise"):
            FAULT_NAMES,
        ("STAGE_TIMELINE.json", "$.negative_results[].fixture"):
            FAULT_NAMES,
        ("STAGE_TIMELINE.json", "$.negative_results[].expected_blocker"):
            BLOCKER_VALUES,
        ("STAGE_TIMELINE.json", "$.negative_results[].observed_blocker"):
            BLOCKER_VALUES,
        ("STAGE_TIMELINE.json", "$.negative_results[].last_in_flight_operation"):
            frozenset({"query_audio_processor", "release_audio_processor"}),
        ("STAGE_TIMELINE.json", "$.positive[].audio_processor_state"):
            frozenset({"audio_processor_lease_retired"}),
        ("STAGE_TIMELINE.json", "$.positive[].component_state"):
            frozenset({"component_released"}),
        ("STAGE_TIMELINE.json", "$.positive[].controller_cid_raw_tuid_hex"):
            frozenset({"655B9DD3AFD7FA42843F4AC841EB04F0"}),
        ("STAGE_TIMELINE.json", "$.positive[].enclosing_operation"):
            frozenset({"initialize_component", "terminate_component"}),
        ("STAGE_TIMELINE.json", "$.positive[].event"):
            frozenset({"call_started", "call_completed", "host_callback",
                       "lifecycle"}),
        ("STAGE_TIMELINE.json", "$.positive[].interface"):
            frozenset({"IAudioProcessor", "IComponent", "IPluginFactory",
                       "IPluginFactory2", "IPluginFactory3"}),
        ("STAGE_TIMELINE.json", "$.positive[].object_role"):
            frozenset({"again_audio_processor_interface",
                       "again_processor_component"}),
        ("STAGE_TIMELINE.json", "$.positive[].operation"):
            OPERATION_VALUES,
        ("STAGE_TIMELINE.json", "$.positive[].origin"):
            frozenset({"component", "owner_local"}),
        ("STAGE_TIMELINE.json", "$.positive[].processor_cid_raw_tuid_hex"):
            frozenset({"5FDEE8845592534F96FAE4133C935A18"}),
        ("STAGE_TIMELINE.json", "$.positive[].requested_iid_raw_tuid_hex"):
            frozenset({"31FF31E8D5F20143928EBBEE25697802",
                       "993F0442DAB73C45A569E79D9AAEC33D"}),
        ("STAGE_TIMELINE.json", "$.positive[].requested_interface"):
            frozenset({"Steinberg::Vst::IAudioProcessor"}),
        ("STAGE_TIMELINE.json", "$.positive[].result_u32_hex"):
            frozenset({"00000000"}),
        ("STAGE_TIMELINE.json", "$.positive[].return_kind"):
            frozenset({"bool", "bool_true", "handle_nonnull", "i32",
                       "pointer_nonnull", "reference_count", "tresult", "u32"}),
        ("STAGE_TIMELINE.json", "$.positive[].state"):
            STATE_VALUES,
        ("STAGE_TIMELINE.json", "$.positive[].thread_role"):
            frozenset({"scanner_main_thread"}),
        ("STAGE_TIMELINE.json", "$.positive[].tier"):
            frozenset({"factory_3_unicode"}),
        ("STAGE_TIMELINE.json", "$.positive[].win32_error_u32_hex"):
            frozenset({"00000000"}),
        ("fixture.json", "$.schema"):
            frozenset({JSON_SCHEMAS["fixture.json"]}),
        ("fixture.json", "$.host.architecture"): frozenset({"x86_64"}),
        ("fixture.json", "$.host.hardware"):
            frozenset({"Steam Deck Galileo"}),
        ("fixture.json", "$.host.os"): frozenset({"SteamOS 3.8.16"}),
        ("fixture.json", "$.host.read_only_mode"): frozenset({"enabled"}),
        ("fixture.json", "$.admitted_interface"):
            frozenset({"Steinberg::Vst::IAudioProcessor"}),
        ("fixture.json", "$.admitted_interface_logical_iid"):
            frozenset({"42043F99B7DA453CA569E79D9AAEC33D"}),
        ("fixture.json", "$.admitted_interface_raw_windows_tuid"):
            frozenset({"993F0442DAB73C45A569E79D9AAEC33D"}),
        ("fixture.json", "$.component_interface"):
            frozenset({"Steinberg::Vst::IComponent"}),
        ("fixture.json", "$.component_logical_iid"):
            frozenset({"E831FF31F2D54301928EBBEE25697802"}),
        ("fixture.json", "$.component_raw_windows_tuid"):
            frozenset({"31FF31E8D5F20143928EBBEE25697802"}),
        ("fixture.json", "$.processor_logical_cid"):
            frozenset({"84E8DE5F92554F5396FAE4133C935A18"}),
        ("fixture.json", "$.processor_raw_windows_tuid"):
            frozenset({"5FDEE8845592534F96FAE4133C935A18"}),
        ("fixture.json", "$.positive_result"):
            frozenset({"scanner_completed"}),
    }
    if key in exact:
        return value in exact[key]

    hash_paths = {
        "AUDIO_PROCESSOR_LEASE.json": {
            "$.implementation_source_identity.manifest_sha256",
            "$.module_sha256", "$.scanner_sha256", "$.stage_timeline_sha256",
        },
        "COMPONENT_SESSION.json": {
            "$.callback_ledger_sha256",
            "$.implementation_source_identity.manifest_sha256",
            "$.module_sha256", "$.scanner_sha256", "$.stage_timeline_sha256",
        },
        "fixture.json": {
            "$.artifact.manifest_sha256",
            "$.implementation_source.manifest_sha256",
            "$.runner_identity_sha256",
        },
    }
    git_paths = {
        "AUDIO_PROCESSOR_LEASE.json": {
            "$.implementation_source_identity.commit",
            "$.implementation_source_identity.tree",
        },
        "COMPONENT_SESSION.json": {
            "$.implementation_source_identity.commit",
            "$.implementation_source_identity.tree",
        },
        "fixture.json": {
            "$.implementation_source.commit", "$.implementation_source.parent",
            "$.implementation_source.tree", "$.workflow.git_blob",
        },
    }
    source_schema_paths = {
        ("AUDIO_PROCESSOR_LEASE.json", "$.implementation_source_identity.schema"),
        ("COMPONENT_SESSION.json", "$.implementation_source_identity.schema"),
        ("fixture.json", "$.implementation_source.schema"),
    }
    if path in hash_paths.get(filename, set()):
        return SHA256.fullmatch(value) is not None
    if path in git_paths.get(filename, set()):
        return HEX40.fullmatch(value) is not None
    if key in source_schema_paths:
        return value == SOURCE_SCHEMA
    if filename == "STAGE_TIMELINE.json" and re.fullmatch(
        r"\$\.negative_results\[\]\.inherited_shutdown\.operations\."
        r"(?:terminate_component|release_component|release_factory_3|"
        r"release_factory_2|release_factory_base|exit_dll|free_library)\."
        r"disposition",
        path,
    ):
        return value in {
            "completed", "not_attempted_audio_interface_quiescence_unproved"
        }
    if filename == "STAGE_TIMELINE.json" and re.fullmatch(
        r"\$\.negative_results\[\]\.inherited_shutdown\.operations\."
        r"(?:terminate_component|release_component|release_factory_3|"
        r"release_factory_2|release_factory_base|exit_dll|free_library)\."
        r"source",
        path,
    ):
        return value in {
            "scanner_call_ledger", "scanner_suppression_record",
            "supervisor_unmatched_component_call",
        }
    if filename == "fixture.json" and path == "$.artifact.id":
        return DECIMAL.fullmatch(value) is not None
    if filename == "fixture.json" and path == "$.artifact.name":
        return re.fullmatch(
            r"wa0-windows-build-[0-9a-f]{40}-run-[1-9][0-9]*-attempt-[1-9][0-9]*",
            value,
        ) is not None
    if filename == "fixture.json" and path in {
        "$.workflow.run_id", "$.workflow.run_attempt",
    }:
        return DECIMAL.fullmatch(value) is not None
    if filename == "fixture.json" and path == "$.workflow.path":
        return value == WORKFLOW_PATH
    if filename == "fixture.json":
        sdk = _sdk_scalar_allowed(path, "$.vst3_sdk", value)
        if sdk is not None:
            return sdk

    if filename != "BUILD_MANIFEST.json":
        return False

    build_exact: dict[str, frozenset[str]] = {
        "$.schema": frozenset({JSON_SCHEMAS["BUILD_MANIFEST.json"]}),
        "$.repository": frozenset({REPOSITORY}),
        "$.again_bundle_manifest.binary_safe_path":
            frozenset({"again.vst3/Contents/x86_64-win/again.vst3"}),
        "$.again_bundle_manifest.records[].path": AGAIN_BUNDLE_PATHS,
        "$.again_bundle_manifest.schema":
            frozenset({"linux-vst-bridge-wf0-bundle-manifest/v1"}),
        "$.artifact_manifest.records[].path": ARTIFACT_PATHS,
        "$.artifact_manifest.records[].portable_mode": frozenset({"0444"}),
        "$.artifact_manifest.records[].role": frozenset({
            "adapter_environment_carrier", "approved_fault_module",
            "build_identity_core", "loader_adapter_executable",
            "positive_fixture_module", "positive_fixture_resource",
            "required_license_notice", "scanner_executable",
        }),
        "$.artifact_manifest.records[].type": frozenset({"file"}),
        "$.artifact_manifest.schema":
            frozenset({"linux-vst-bridge-wf0-artifact-manifest/v1"}),
        "$.authority.approval_blob": frozenset({APPROVAL_BLOB}),
        "$.authority.authority_merge_commit":
            frozenset({AUTHORITY_MERGE_COMMIT}),
        "$.authority.authority_merge_tree": frozenset({AUTHORITY_MERGE_TREE}),
        "$.authority.design_blob": frozenset({DESIGN_BLOB}),
        "$.authority.design_commit": frozenset({DESIGN_COMMIT}),
        "$.authority.design_sha256": frozenset({DESIGN_SHA256}),
        "$.authority.design_tree": frozenset({DESIGN_TREE}),
        "$.authority.implementation_basis_commit": frozenset({BASIS_COMMIT}),
        "$.authority.implementation_basis_tree": frozenset({BASIS_TREE}),
        "$.authority.review_github_id": frozenset({REVIEW_GITHUB_ID}),
        "$.authority.review_result": frozenset({"DESIGN_CLEAR"}),
        "$.deterministic_regression.cases[]": DETERMINISTIC_CASES,
        "$.deterministic_regression.evidence_allowlist_regression.mutation_cases[]":
            EVIDENCE_ALLOWLIST_MUTATION_CASES,
        "$.deterministic_regression.proof_disposition_regression.cases[]":
            PROOF_MUTATION_CASES,
        "$.deterministic_regression.release_baseline_regression.production_helper":
            frozenset({"audio_release_matches_component_baseline"}),
        "$.implementation_source_manifest.records[].git_mode":
            frozenset({"100644"}),
        "$.implementation_source_manifest.records[].path":
            frozenset(SOURCE_PATHS),
        "$.implementation_source_manifest.schema": frozenset({SOURCE_SCHEMA}),
        "$.mac_artifact_custody_receipt.explicit_nonclaims[]":
            CUSTODY_NONCLAIMS,
        "$.mac_artifact_custody_receipt.implementation_source.branch":
            frozenset({EXPECTED_BRANCH}),
        "$.mac_artifact_custody_receipt.implementation_source.ref":
            frozenset({EXPECTED_REF}),
        "$.mac_artifact_custody_receipt.implementation_source.schema":
            frozenset({SOURCE_SCHEMA}),
        "$.mac_artifact_custody_receipt.inner_envelope.records[].path":
            frozenset({"WF0_WINDOWS_BUILD_RECEIPT.json",
                       "WF0_WINDOWS_BUILD_RECEIPT.sha256", "wf0-payload.zip"}),
        "$.mac_artifact_custody_receipt.repository.full_name":
            frozenset({REPOSITORY}),
        "$.mac_artifact_custody_receipt.repository.owner_type":
            frozenset({"User"}),
        "$.mac_artifact_custody_receipt.repository.visibility":
            frozenset({"private"}),
        "$.mac_artifact_custody_receipt.schema":
            frozenset({"linux-vst-bridge-wf0-mac-artifact-custody/v1"}),
        "$.mac_artifact_custody_receipt.workflow.conclusion":
            frozenset({"success"}),
        "$.mac_artifact_custody_receipt.workflow.event": frozenset({"push"}),
        "$.mac_artifact_custody_receipt.workflow.head_branch":
            frozenset({EXPECTED_BRANCH}),
        "$.mac_artifact_custody_receipt.workflow.path":
            frozenset({WORKFLOW_PATH}),
        "$.proof_disposition[].claim": frozenset(PROOF_ROWS),
        "$.proof_disposition[].evidence": PROOF_EVIDENCE,
        "$.proof_disposition[].result": frozenset({"passed"}),
        "$.runtime_proton_digest": frozenset({RUNNER_DIGEST}),
        "$.source_handoff_receipt.bundle.git_bundle_verify":
            frozenset({"passed"}),
        "$.source_handoff_receipt.implementation_source.branch":
            frozenset({EXPECTED_BRANCH}),
        "$.source_handoff_receipt.implementation_source.manifest_schema":
            frozenset({SOURCE_SCHEMA}),
        "$.source_handoff_receipt.implementation_source.ref":
            frozenset({EXPECTED_REF}),
        "$.source_handoff_receipt.repository": frozenset({REPOSITORY}),
        "$.source_handoff_receipt.schema":
            frozenset({"linux-vst-bridge-wa0-source-handoff/v1"}),
        "$.source_handoff_receipt.wa0_authority.design_authority_commit":
            frozenset({AUTHORITY_MERGE_COMMIT}),
        "$.source_handoff_receipt.wa0_authority.design_authority_tree":
            frozenset({AUTHORITY_MERGE_TREE}),
        "$.source_handoff_receipt.wa0_authority.implementation_basis_commit":
            frozenset({BASIS_COMMIT}),
        "$.source_handoff_receipt.wa0_authority.implementation_basis_tree":
            frozenset({BASIS_TREE}),
        "$.windows_build_receipt.artifacts.again_bundle_roster[].path":
            frozenset(f"again.vst3/{item}" for item in AGAIN_BUNDLE_PATHS),
        "$.windows_build_receipt.artifacts.artifact_manifest_schema":
            frozenset({"linux-vst-bridge-wf0-artifact-manifest/v1"}),
        "$.windows_build_receipt.artifacts.pe_receipts[].exports[]": PE_EXPORTS,
        "$.windows_build_receipt.artifacts.pe_receipts[].imports[]": PE_IMPORTS,
        "$.windows_build_receipt.artifacts.pe_receipts[].machine":
            frozenset({"AMD64"}),
        "$.windows_build_receipt.artifacts.pe_receipts[].path": PE_PATHS,
        "$.windows_build_receipt.artifacts.pe_receipts[].pe_kind":
            frozenset({"PE32+"}),
        "$.windows_build_receipt.artifacts.scanner_and_fault_roster[].path":
            frozenset(PE_PATHS - {"again.vst3/Contents/x86_64-win/again.vst3"}),
        "$.windows_build_receipt.build.comparison.level":
            frozenset({"byte_identical"}),
        "$.windows_build_receipt.build.configuration": frozenset({"Release"}),
        "$.windows_build_receipt.build.deterministic_build_root_mapping":
            frozenset({r"C:\wf0\build"}),
        "$.windows_build_receipt.build.msvc_post_options": frozenset({"/MP1"}),
        "$.windows_build_receipt.build.msvc_runtime":
            frozenset({"MultiThreaded"}),
        "$.windows_build_receipt.build.pdb_embedded_path":
            frozenset({"%_PDB%"}),
        "$.windows_build_receipt.build.targets[]": BUILD_TARGETS,
        "$.windows_build_receipt.explicit_nonclaims[]": WINDOWS_BUILD_NONCLAIMS,
        "$.windows_build_receipt.repository.full_name": frozenset({REPOSITORY}),
        "$.windows_build_receipt.repository.owner_type": frozenset({"User"}),
        "$.windows_build_receipt.repository.visibility": frozenset({"private"}),
        "$.windows_build_receipt.runner.architecture": frozenset({"X64"}),
        "$.windows_build_receipt.runner.image_os": frozenset({"win22"}),
        "$.windows_build_receipt.runner.requested_label":
            frozenset({"windows-2022"}),
        "$.windows_build_receipt.runner.windows_product_name":
            frozenset({"Microsoft Windows Server 2022 Datacenter"}),
        "$.windows_build_receipt.schema":
            frozenset({"linux-vst-bridge-wf0-windows-build/v1"}),
        "$.windows_build_receipt.source.branch": frozenset({EXPECTED_BRANCH}),
        "$.windows_build_receipt.source.implementation_source_manifest.schema":
            frozenset({SOURCE_SCHEMA}),
        "$.windows_build_receipt.source.ref": frozenset({EXPECTED_REF}),
        "$.windows_build_receipt.toolchain.generator":
            frozenset({"Visual Studio 17 2022"}),
        "$.windows_build_receipt.toolchain.generator_platform":
            frozenset({"x64"}),
        "$.windows_build_receipt.toolchain.generator_toolset":
            frozenset({"v143"}),
        "$.windows_build_receipt.toolchain.host_architecture":
            frozenset({"x64"}),
        "$.windows_build_receipt.toolchain.platform_toolset":
            frozenset({"v143"}),
        "$.windows_build_receipt.toolchain.target_architecture":
            frozenset({"x64"}),
        "$.windows_build_receipt.toolchain.visual_studio_edition":
            frozenset({"Enterprise"}),
        "$.windows_build_receipt.toolchain.windows_sdk_version":
            frozenset({"10.0.19041.0"}),
        "$.windows_build_receipt.workflow.event": frozenset({"push"}),
        "$.windows_build_receipt.workflow.path": frozenset({WORKFLOW_PATH}),
        "$.windows_build_receipt.workflow.ref": frozenset({EXPECTED_REF}),
    }
    if path in build_exact:
        return value in build_exact[path]

    sdk = _sdk_scalar_allowed(
        path, "$.windows_build_receipt.vst3_sdk", value
    )
    if sdk is not None:
        return sdk

    sha_paths = {
        "$.again_bundle_manifest.sha256",
        "$.again_bundle_manifest.records[].sha256",
        "$.artifact_manifest.build_identity_core_sha256",
        "$.artifact_manifest.implementation_source_manifest_sha256",
        "$.artifact_manifest.records[].sha256", "$.audio_processor_lease_sha256",
        "$.authority.accepted_wc0.artifact_manifest_sha256",
        "$.authority.accepted_wc0.scanner_sha256",
        "$.authority.accepted_wc0.source_manifest_sha256",
        "$.component_session_sha256",
        "$.deck_admission.artifact_cache_manifest_sha256",
        "$.deck_admission.source_bundle_sha256",
        "$.deck_admission.source_handoff_receipt_sha256",
        "$.implementation_source_manifest_sha256",
        "$.mac_artifact_custody_receipt.artifact.raw_github_artifact_zip_sha256",
        "$.mac_artifact_custody_receipt.artifact.raw_wrapper_sha256",
        "$.mac_artifact_custody_receipt.artifact.upload_artifact_digest_bare",
        "$.mac_artifact_custody_receipt.implementation_source.manifest_sha256",
        "$.mac_artifact_custody_receipt.inner_envelope.artifact_manifest_sha256",
        "$.mac_artifact_custody_receipt.inner_envelope.build_receipt_sha256",
        "$.mac_artifact_custody_receipt.inner_envelope.payload_archive_sha256",
        "$.mac_artifact_custody_receipt.inner_envelope.records[].sha256",
        "$.source_handoff_receipt.bundle.sha256",
        "$.source_handoff_receipt.implementation_source.manifest_sha256",
        "$.windows_build_receipt.artifacts.again_bundle_roster[].sha256",
        "$.windows_build_receipt.artifacts.artifact_manifest_sha256",
        "$.windows_build_receipt.artifacts.payload_archive_sha256",
        "$.windows_build_receipt.artifacts.pe_receipts[].dumpbin_sha256",
        "$.windows_build_receipt.artifacts.pe_receipts[].sha256",
        "$.windows_build_receipt.artifacts.scanner_and_fault_roster[].sha256",
        "$.windows_build_receipt.build.build_a.artifact_manifest_sha256",
        "$.windows_build_receipt.build.build_b.artifact_manifest_sha256",
        "$.windows_build_receipt.build.cmake_options_sha256",
        "$.windows_build_receipt.build.comparison.manifest_sha256",
        "$.windows_build_receipt.source.implementation_source_manifest.sha256",
        "$.windows_build_receipt.tool_receipts.cl_bv_sha256",
        "$.windows_build_receipt.toolchain.cl_bv_sha256",
        "$.windows_build_receipt.toolchain.visual_studio_installation_sha256",
    }
    git_paths = {
        "$.authority.accepted_wc0.evidence_commit",
        "$.authority.accepted_wc0.evidence_tree",
        "$.authority.accepted_wc0.implementation_merge",
        "$.authority.accepted_wc0.implementation_merge_tree",
        "$.authority.accepted_wc0.source_commit",
        "$.authority.accepted_wc0.source_tree",
        "$.deck_admission.detached_worktree_commit",
        "$.implementation_source_manifest.commit",
        "$.implementation_source_manifest.records[].git_blob",
        "$.implementation_source_tree",
        "$.mac_artifact_custody_receipt.artifact.workflow_head_sha",
        "$.mac_artifact_custody_receipt.implementation_source.commit",
        "$.mac_artifact_custody_receipt.implementation_source.parent",
        "$.mac_artifact_custody_receipt.implementation_source.tree",
        "$.mac_artifact_custody_receipt.workflow.git_blob",
        "$.mac_artifact_custody_receipt.workflow.head_sha",
        "$.source_handoff_receipt.implementation_source.commit",
        "$.source_handoff_receipt.implementation_source.parent",
        "$.source_handoff_receipt.implementation_source.tree",
        "$.windows_build_receipt.source.commit",
        "$.windows_build_receipt.source.parent",
        "$.windows_build_receipt.source.tree",
        "$.windows_build_receipt.workflow.commit",
        "$.windows_build_receipt.workflow.git_blob",
    }
    if path in sha_paths:
        return SHA256.fullmatch(value) is not None
    if path in git_paths:
        return HEX40.fullmatch(value) is not None
    if path in {
        "$.mac_artifact_custody_receipt.artifact.digest",
        "$.mac_artifact_custody_receipt.artifact.rest_artifact_digest",
    }:
        return TYPED_SHA256.fullmatch(value) is not None
    if path in {
        "$.mac_artifact_custody_receipt.artifact.id",
        "$.mac_artifact_custody_receipt.artifact.size_in_bytes",
        "$.mac_artifact_custody_receipt.artifact.workflow_run_id",
        "$.mac_artifact_custody_receipt.workflow.run_attempt",
        "$.mac_artifact_custody_receipt.workflow.run_id",
        "$.windows_build_receipt.build.source_date_epoch",
        "$.windows_build_receipt.workflow.run_attempt",
        "$.windows_build_receipt.workflow.run_id",
    }:
        return DECIMAL.fullmatch(value) is not None
    if path == "$.mac_artifact_custody_receipt.artifact.name":
        return re.fullmatch(
            r"wa0-windows-build-[0-9a-f]{40}-run-[1-9][0-9]*-attempt-[1-9][0-9]*",
            value,
        ) is not None
    if path == "$.source_handoff_receipt.bundle.name":
        return re.fullmatch(r"wa0-execution-source-[0-9a-f]{40}\.bundle",
                            value) is not None
    if path in {
        "$.deck_admission.source_ref",
        "$.source_handoff_receipt.bundle.advertised_ref",
    }:
        return re.fullmatch(r"refs/handoff/wa0-source/[0-9a-f]{40}",
                            value) is not None
    if path == "$.mac_artifact_custody_receipt.artifact.human_facing_url":
        return PUBLIC_URLS[0].fullmatch(value) is not None
    if path == "$.mac_artifact_custody_receipt.artifact.url":
        return PUBLIC_URLS[1].fullmatch(value) is not None
    if path == "$.mac_artifact_custody_receipt.artifact.raw_download_route":
        return PUBLIC_API_ROUTES.fullmatch(value) is not None
    if path == "$.mac_artifact_custody_receipt.artifact.workflow_head_branch":
        return value == EXPECTED_BRANCH
    if path in {
        "$.windows_build_receipt.runner.image_version",
    }:
        return re.fullmatch(r"[0-9]{8}\.[0-9]+\.[0-9]+", value) is not None
    if path == "$.windows_build_receipt.runner.windows_build":
        return re.fullmatch(r"[0-9]{5}", value) is not None
    if path in {
        "$.windows_build_receipt.runner.windows_version",
        "$.windows_build_receipt.toolchain.cl_version",
        "$.windows_build_receipt.toolchain.cmake_version",
        "$.windows_build_receipt.toolchain.link_version",
        "$.windows_build_receipt.toolchain.visual_studio_version",
        "$.windows_build_receipt.toolchain.vscmd_version",
    }:
        return VERSION.fullmatch(value) is not None
    return False


def _safe_relative(value: str) -> bool:
    return (
        0 < len(value.encode("utf-8")) <= 240
        and not value.startswith(("/", "\\"))
        and "\\" not in value and ":" not in value
        and all(part not in {"", ".", ".."} for part in value.split("/"))
    )


def _validate_scalar_string(value: str, filename: str, path: str) -> None:
    context = f"{filename}:{path}"
    if (
        len(value.encode("utf-8")) > 4096
        or any(ord(character) < 0x20 for character in value)
        or PRIVATE_SCALAR.search(value)
    ):
        fail(f"WA0 evidence scalar is private or outside bounds: {context}")
    if value.startswith(("http://", "https://")) and not any(
        pattern.fullmatch(value) for pattern in PUBLIC_URLS
    ):
        fail(f"WA0 evidence URL is outside the public allow-list: {context}")
    if value.startswith("/") and value != "/MP1" and not PUBLIC_API_ROUTES.fullmatch(value):
        fail(f"WA0 evidence absolute path is outside the allow-list: {context}")
    if re.search(r"[A-Za-z]:\\", value) and not re.fullmatch(
        r"C:\\wf0(?:\\build)?", value
    ):
        fail(f"WA0 evidence Windows path is outside the allow-list: {context}")

    if not _contextual_scalar_allowed(filename, path, value):
        fail(f"WA0 evidence scalar differs from its contextual rule: {context}")


def _validate_json_value(
    value: Any, filename: str, path: str = "$", *, depth: int = 0
) -> None:
    context = f"{filename}:{path}"
    if depth > 32:
        fail(f"WA0 evidence JSON nesting exceeds its bound: {context}")
    if isinstance(value, dict):
        if len(value) > 512:
            fail(f"WA0 evidence JSON object exceeds its bound: {context}")
        rule = JSON_OBJECT_RULES.get(filename, {}).get(path)
        if rule is None:
            fail(f"WA0 evidence object path is outside the schema: {context}")
        required, allowed = rule
        keys = frozenset(value)
        if not required.issubset(keys) or not keys.issubset(allowed):
            fail(f"WA0 evidence object keys differ from the schema: {context}")
        for key, child in value.items():
            if not isinstance(key, str):
                fail(f"WA0 evidence JSON field is not a string: {context}")
            _validate_json_value(
                child, filename, f"{path}.{key}", depth=depth + 1
            )
    elif isinstance(value, list):
        if len(value) > 2048:
            fail(f"WA0 evidence JSON array exceeds its bound: {context}")
        if path not in JSON_LIST_PATHS.get(filename, frozenset()):
            fail(f"WA0 evidence array path is outside the schema: {context}")
        for index, child in enumerate(value):
            _validate_json_value(
                child, filename, f"{path}[]", depth=depth + 1
            )
    elif isinstance(value, str):
        _validate_scalar_string(value, filename, path)
    elif value is None:
        if (
            context not in NULLABLE_PATHS
            and not (
                filename == "STAGE_TIMELINE.json"
                and path.startswith("$.positive[].")
                and path.rsplit(".", 1)[-1] in POSITIVE_NULLABLE_FIELDS
            )
        ):
            fail(f"WA0 evidence null is outside the schema: {context}")
    elif isinstance(value, bool):
        leaf = path.rsplit(".", 1)[-1].removesuffix("[]")
        if leaf == "result" and path.endswith(".module_exit.result"):
            return
        if leaf not in BOOL_FIELDS:
            fail(f"WA0 evidence boolean is outside the schema: {context}")
    elif isinstance(value, int) and not isinstance(value, bool):
        leaf = path.rsplit(".", 1)[-1].removesuffix("[]")
        if leaf not in INTEGER_FIELDS or not -(2**63) <= value < 2**63:
            fail(f"WA0 evidence integer exceeds its bound: {context}")
    else:
        fail(f"WA0 evidence JSON type is outside the allow-list: {context}")


def validate_evidence_allowlist(root: pathlib.Path) -> None:
    for name, expected_top in JSON_TOP_LEVEL.items():
        path = root / name
        data = path.read_bytes()
        try:
            value = json.loads(data)
        except (json.JSONDecodeError, UnicodeDecodeError):
            fail(f"WA0 evidence JSON is malformed: {name}")
        if (
            not isinstance(value, dict)
            or set(value) != expected_top
            or value.get("schema") != JSON_SCHEMAS[name]
            or canonical_json(value) != data
        ):
            fail(f"WA0 evidence JSON schema/top-level/canonical form differs: {name}")
        _validate_json_value(value, name)
    for name, heading in MARKDOWN_HEADINGS.items():
        text = (root / name).read_text(encoding="utf-8", errors="strict")
        if (
            not text.startswith(heading + "\n")
            or "http://" in text
            or "https://" in text
            or PRIVATE_SCALAR.search(text)
            or re.search(r"[A-Za-z]:\\", text)
        ):
            fail(f"WA0 Markdown is outside its fixed public template: {name}")


def evidence_allowlist_regression() -> dict[str, Any]:
    mutations = (
        ("unknown_root_id", lambda: _validate_json_value(
            {"id": "1234"}, "fixture.json"
        )),
        ("private_host_name", lambda: _validate_scalar_string(
            "deck-private", "COMPONENT_SESSION.json",
            "$.callback_ledger.host_name",
        )),
        ("numeric_artifact_id_type", lambda: _validate_json_value(
            1234, "BUILD_MANIFEST.json",
            "$.mac_artifact_custody_receipt.artifact.id",
        )),
        ("traversal_artifact_path", lambda: _validate_scalar_string(
            "../private", "BUILD_MANIFEST.json",
            "$.artifact_manifest.records[].path",
        )),
        ("unallowlisted_url", lambda: _validate_scalar_string(
            "https://example.invalid/private", "BUILD_MANIFEST.json",
            "$.mac_artifact_custody_receipt.artifact.url",
        )),
        ("email_in_allowed_name_context", lambda: _validate_scalar_string(
            "operator@example.invalid", "BUILD_MANIFEST.json",
            "$.source_handoff_receipt.bundle.name",
        )),
        ("ip_in_allowed_name_context", lambda: _validate_scalar_string(
            "192.0.2.10", "COMPONENT_SESSION.json",
            "$.inherited_wf0_regression.ordered_class_census[].name",
        )),
        ("pointer_in_allowed_name_context", lambda: _validate_scalar_string(
            "0x1234567890abcdef", "AUDIO_PROCESSOR_LEASE.json",
            "$.interface.name",
        )),
        ("wrong_context_architecture", lambda: _validate_scalar_string(
            "my-workstation-17", "fixture.json", "$.host.architecture",
        )),
        ("wrong_context_read_only_mode", lambda: _validate_scalar_string(
            "disabled", "fixture.json", "$.host.read_only_mode",
        )),
        ("wrong_context_interface_name", lambda: _validate_scalar_string(
            "Steinberg::Vst::IComponent", "fixture.json",
            "$.admitted_interface",
        )),
        ("wrong_context_branch_ref", lambda: _validate_scalar_string(
            "refs/heads/main", "BUILD_MANIFEST.json",
            "$.windows_build_receipt.workflow.ref",
        )),
        ("wrong_context_lifecycle_state", lambda: _validate_scalar_string(
            "scanner_completed", "AUDIO_PROCESSOR_LEASE.json", "$.state",
        )),
        ("wrong_context_repository_identity", lambda: _validate_scalar_string(
            "root", "BUILD_MANIFEST.json", "$.repository",
        )),
    )
    rejected = 0
    for _, mutation in mutations:
        try:
            mutation()
        except Exception:
            rejected += 1
    _validate_scalar_string(
        "Linux VST Bridge WC0", "COMPONENT_SESSION.json",
        "$.callback_ledger.host_name",
    )
    _validate_scalar_string(
        "1234", "BUILD_MANIFEST.json",
        "$.mac_artifact_custody_receipt.artifact.id",
    )
    _validate_scalar_string(
        "fixtures/wa0-query-crash.dll", "BUILD_MANIFEST.json",
        "$.artifact_manifest.records[].path",
    )
    _validate_scalar_string(
        "https://api.github.com/repos/kasselvania/Linux-VST-bridge/"
        "actions/artifacts/1", "BUILD_MANIFEST.json",
        "$.mac_artifact_custody_receipt.artifact.url",
    )
    _validate_scalar_string("x86_64", "fixture.json", "$.host.architecture")
    _validate_scalar_string(
        "Steinberg::Vst::IAudioProcessor", "fixture.json",
        "$.admitted_interface",
    )
    if rejected != len(mutations):
        fail("WA0 evidence allow-list mutation regression accepted private data")
    return {
        "mutation_cases": [name for name, _ in mutations],
        "rejected_count": rejected,
        "public_samples_accepted": True,
        "contextual_object_key_schemas": True,
        "contextual_scalar_rules": True,
    }


EXPECTED_CENSUS = [
    {
        "ordinal": 0,
        "logical_class_id": "84E8DE5F92554F5396FAE4133C935A18",
        "raw_windows_tuid": "5FDEE8845592534F96FAE4133C935A18",
        "name": "AGain VST3", "category": "Audio Module Class",
    },
    {
        "ordinal": 1,
        "logical_class_id": "D39D5B65D7AF42FA843F4AC841EB04F0",
        "raw_windows_tuid": "655B9DD3AFD7FA42843F4AC841EB04F0",
        "name": "AGain VST3Controller",
        "category": "Component Controller Class",
    },
    {
        "ordinal": 2,
        "logical_class_id": "41347FD6FED64094AFBB12B7DBA1D441",
        "raw_windows_tuid": "D67F3441D6FE9440AFBB12B7DBA1D441",
        "name": "AGain SideChain VST3", "category": "Audio Module Class",
    },
]


def _sdk_blob_records(
    lock: dict[str, tuple[str, str]],
) -> list[dict[str, str]]:
    records = []
    for path in sorted(lock, key=lambda value: value.encode("utf-8")):
        blob, digest = lock[path]
        first, separator, remainder = path.partition("/")
        repository = first if separator and first in SDK_SUBMODULES else "root"
        records.append({
            "path": path,
            "repository": repository,
            "repository_commit": (
                SDK_COMMIT if repository == "root" else SDK_SUBMODULES[repository]
            ),
            "repository_path": path if repository == "root" else remainder,
            "git_blob": blob,
            "unfiltered_worktree_git_blob": blob,
            "sha256": digest,
        })
    return records


def _sdk_receipt_exact(value: Any) -> bool:
    if not isinstance(value, dict):
        return False
    return value == {
        "root_commit": SDK_COMMIT,
        "root_tree": SDK_TREE,
        "root_checkout_configuration": {
            "core.autocrlf": "false", "core.eol": "lf",
        },
        "submodules": [
            {
                "path": path, "commit": commit,
                "checkout_configuration": {
                    "core.autocrlf": "false", "core.eol": "lf",
                },
            }
            for path, commit in SDK_SUBMODULES.items()
        ],
        "dirty_path_count": 0,
        "source_patch_count": 0,
        "failure_source_blobs": _sdk_blob_records(SDK_SOURCE_BLOBS),
        "positive_fixture_source_blobs": _sdk_blob_records(
            SDK_POSITIVE_FIXTURE_BLOBS
        ),
        "checkout_regression": {
            "schema": "linux-vst-bridge-wf0-git-checkout-regression/v1",
            "hostile_ambient_core_autocrlf": "true",
            "command_scoped_configuration": {
                "core.autocrlf": "false", "core.eol": "lf",
            },
            "root_local_configuration": {
                "core.autocrlf": "false", "core.eol": "lf",
            },
            "recursive_submodule_local_configuration": {
                "core.autocrlf": "false", "core.eol": "lf",
            },
            "root_checkout_canonical_lf": True,
            "recursive_submodule_checkout_canonical_lf": True,
            "canonical_lf_blob": "fec6d032d7a9139d8e1ddaa0363d5b7ad1ee691a",
            "canonical_lf_sha256":
                "a9b428d9c8c85f87dd3dc9e4524f793557bfaef3c01aa6a649bd7172d33737c8",
            "canonical_submodule_blob":
                "7b015aea25ced907ba60f8ee0aee380e4ad6eb07",
            "canonical_submodule_sha256":
                "630b8c6331dab42032b0a5ddcbb20d88432f3b4e5eedaff6c525bc28815e8a94",
            "filtered_crlf_blob_masked_mismatch": True,
            "unfiltered_crlf_blob_rejected": True,
            "raw_crlf_sha256_rejected": True,
        },
    }


def _negative_call_counts(*, release_audio: int, inherited_shutdown: int) \
        -> dict[str, int]:
    return {
        "load_library": 1, "init_dll": 1, "get_plugin_factory": 1,
        "get_factory_info": 1, "query_factory_2": 1, "query_factory_3": 1,
        "count_classes": 1, "get_class_info_unicode": 1,
        "get_class_info_2": 0, "get_class_info_1": 0,
        "create_component": 1, "get_controller_class_id": 1,
        "initialize_component": 1, "query_audio_processor": 1,
        "release_audio_processor": release_audio,
        "terminate_component": inherited_shutdown,
        "release_component": inherited_shutdown,
        "release_factory_3": inherited_shutdown,
        "release_factory_2": inherited_shutdown,
        "release_factory_base": inherited_shutdown,
        "exit_dll": inherited_shutdown, "free_library": inherited_shutdown,
    }


def _query_receipt(result: str, output_nonnull: bool,
                   tuple_consistent: bool) -> dict[str, Any]:
    return {
        "attempted": True, "output_zero_initialized": True,
        "result_u32_hex": result, "output_nonnull": output_nonnull,
        "tuple_consistent": tuple_consistent,
    }


def _lease_receipt(
    *, blocker: str, query: dict[str, Any], state: str, acquired: bool,
    release_attempted: bool, release_count: int, release_ordinary: bool,
    quiescent: bool,
) -> dict[str, Any]:
    return {
        "state": state,
        "requested_interface": "Steinberg::Vst::IAudioProcessor",
        "requested_iid_raw_tuid_hex": "993F0442DAB73C45A569E79D9AAEC33D",
        "query": query,
        "lease_acquired": acquired,
        "release": {
            "attempted": release_attempted,
            "returned_ordinary": release_ordinary,
            "reference_count": release_count,
        },
        "pointer_cleared": True,
        "call_in_flight": False,
        "callback_ledger_unchanged": True,
        "audio_interface_quiescence": quiescent,
        "primary_blocker": blocker,
    }


def _proof_disposition(
    retained: dict[str, Any], audio: dict[str, Any],
    component: dict[str, Any], timeline: dict[str, Any], fixture: dict[str, Any],
) -> list[dict[str, Any]]:
    negatives = {
        item.get("exercise"): item
        for item in timeline.get("negative_results", [])
        if isinstance(item, dict)
    }
    expected_negative_names = {
        "wa0-query-failure-null", "wa0-query-success-null",
        "wa0-query-failure-nonnull", "wa0-query-hang", "wa0-query-crash",
        "wa0-release-unexpected-count", "wa0-release-hang",
        "wa0-release-crash",
    }
    shutdown_operations = {
        "terminate_component", "release_component", "release_factory_3",
        "release_factory_2", "release_factory_base", "exit_dll", "free_library",
    }

    def lease(name: str) -> dict[str, Any]:
        value = negatives.get(name, {}).get("audio_processor_lease")
        return value if isinstance(value, dict) else {}

    def calls(name: str) -> dict[str, Any]:
        value = negatives.get(name, {}).get("call_counts")
        return value if isinstance(value, dict) else {}

    def clean(name: str) -> bool:
        shutdown = negatives.get(name, {}).get("inherited_shutdown", {})
        operations = shutdown.get("operations", {})
        return (
            set(operations) == shutdown_operations
            and all(value == {
                "disposition": "completed", "source": "scanner_call_ledger",
            } for value in operations.values())
            and shutdown.get("clean_in_process_shutdown") is True
            and shutdown.get("physical_containment_only") is False
        )

    def suppressed(name: str) -> bool:
        shutdown = negatives.get(name, {}).get("inherited_shutdown", {})
        operations = shutdown.get("operations", {})
        expected_source = (
            "scanner_suppression_record"
            if name == "wa0-release-unexpected-count"
            else "supervisor_unmatched_component_call"
        )
        return (
            set(operations) == shutdown_operations
            and all(value == {
                "disposition":
                    "not_attempted_audio_interface_quiescence_unproved",
                "source": expected_source,
            } for value in operations.values())
            and shutdown.get("clean_in_process_shutdown") is False
            and shutdown.get("physical_containment_only") is True
            and all(calls(name).get(operation) == 0 for operation in operations)
        )

    positive_states = [
        item.get("state") for item in timeline.get("positive", [])
        if item.get("event") == "lifecycle"
    ]
    positive_calls = [
        item.get("operation") for item in timeline.get("positive", [])
        if item.get("event") == "call_started"
    ]
    positive_callbacks = [
        {
            key: item.get(key) for key in (
                "enclosing_attempt_sequence", "enclosing_operation",
                "operation", "origin", "reference_count",
            )
        }
        for item in timeline.get("positive", [])
        if item.get("event") == "host_callback"
    ]
    accepted_wc0 = retained.get("authority", {}).get("accepted_wc0", {})
    quiescence_facts = component.get("object_quiescence", {}).get("facts", {})
    failure_null = lease("wa0-query-failure-null")
    success_null = lease("wa0-query-success-null")
    failure_nonnull = lease("wa0-query-failure-nonnull")
    unexpected_release = lease("wa0-release-unexpected-count")
    expected_failure_null = _lease_receipt(
        blocker="WA0_INTERFACE_QUERY_BLOCKED",
        query=_query_receipt("80004002", False, True),
        state="audio_processor_query_returned_without_lease", acquired=False,
        release_attempted=False, release_count=0, release_ordinary=False,
        quiescent=True,
    )
    expected_success_null = _lease_receipt(
        blocker="WA0_INTERFACE_QUERY_INCONSISTENT",
        query=_query_receipt("00000000", False, False),
        state="audio_processor_query_returned_without_lease", acquired=False,
        release_attempted=False, release_count=0, release_ordinary=False,
        quiescent=True,
    )
    expected_failure_nonnull = _lease_receipt(
        blocker="WA0_INTERFACE_QUERY_INCONSISTENT",
        query=_query_receipt("80004002", True, False),
        state="audio_processor_lease_retired", acquired=True,
        release_attempted=True, release_count=1, release_ordinary=True,
        quiescent=True,
    )
    expected_unexpected_release = _lease_receipt(
        blocker="WA0_INTERFACE_RELEASE_BLOCKED",
        query=_query_receipt("00000000", True, True),
        state="audio_processor_retirement_incomplete", acquired=True,
        release_attempted=True, release_count=2, release_ordinary=True,
        quiescent=False,
    )

    def ordinary_negative_exact(
        name: str, expected_lease: dict[str, Any], blocker: str,
        release_audio: int,
    ) -> bool:
        item = negatives.get(name, {})
        return (
            item.get("exercise") == name
            and item.get("fixture") == name
            and item.get("component_case") == "exact-again"
            and item.get("expected_blocker") == blocker
            and item.get("observed_blocker") == blocker
            and item.get("classification") == "scanner_blocked"
            and item.get("last_in_flight_operation") is None
            and item.get("audio_processor_lease") == expected_lease
            and item.get("audio_processor_observer_state") ==
                expected_lease["state"]
            and item.get("audio_interface_quiescence") is True
            and item.get("component_session_closed") is True
            and item.get("call_counts") == _negative_call_counts(
                release_audio=release_audio, inherited_shutdown=1
            )
            and item.get("forbidden_audio_processor_method_marker_absent") is True
            and item.get("forbidden_component_method_marker_absent") is True
            and clean(name)
        )
    abnormal_expected = {
        "wa0-query-hang": ("query_audio_processor", "call_timeout"),
        "wa0-query-crash": (
            "query_audio_processor", "abnormal_termination_in_flight"
        ),
        "wa0-release-hang": ("release_audio_processor", "call_timeout"),
        "wa0-release-crash": (
            "release_audio_processor", "abnormal_termination_in_flight"
        ),
    }
    abnormal_exact = {
        name: (
            negatives.get(name, {}).get("exercise") == name
            and negatives.get(name, {}).get("fixture") == name
            and negatives.get(name, {}).get("component_case") == "exact-again"
            and negatives.get(name, {}).get("expected_blocker") == (
                "WA0_INTERFACE_QUERY_BLOCKED"
                if expected[0] == "query_audio_processor"
                else "WA0_INTERFACE_RELEASE_BLOCKED"
            )
            and negatives.get(name, {}).get("observed_blocker") == (
                "WA0_INTERFACE_QUERY_BLOCKED"
                if expected[0] == "query_audio_processor"
                else "WA0_INTERFACE_RELEASE_BLOCKED"
            )
            and negatives.get(name, {}).get(
                "last_in_flight_operation"
            ) == expected[0]
            and negatives.get(name, {}).get("classification") == expected[1]
            and negatives.get(name, {}).get("audio_processor_observer_state") ==
                "audio_processor_ownership_unknown"
            and negatives.get(name, {}).get("audio_interface_quiescence") is False
            and negatives.get(name, {}).get("component_session_closed") is False
            and negatives.get(name, {}).get("audio_processor_lease") is None
            and negatives.get(name, {}).get("call_counts") ==
                _negative_call_counts(
                    release_audio=(
                        1 if expected[0] == "release_audio_processor" else 0
                    ),
                    inherited_shutdown=0,
                )
            and negatives.get(name, {}).get(
                "forbidden_audio_processor_method_marker_absent"
            ) is True
            and negatives.get(name, {}).get(
                "forbidden_component_method_marker_absent"
            ) is True
            and suppressed(name)
        )
        for name, expected in abnormal_expected.items()
    }
    all_negative_cleanup = (
        set(negatives) == expected_negative_names
        and all(
            item.get("cleanup") == {
                "owned_descendants_zero": True, "process_group_empty": True,
            }
            and item.get("environment_retired") is True
            for item in negatives.values()
        )
    )
    source = retained.get("implementation_source_manifest", {})
    deterministic = retained.get("deterministic_regression", {})
    call_surface = deterministic.get("component_call_surface", {})
    release_baseline = deterministic.get("release_baseline_regression", {})
    build_receipt = retained.get("windows_build_receipt", {})
    custody = retained.get("mac_artifact_custody_receipt", {})
    source_handoff = retained.get("source_handoff_receipt", {})
    artifact_manifest = retained.get("artifact_manifest", {})
    source_commit = source.get("commit")
    source_tree = retained.get("implementation_source_tree")
    source_digest = retained.get("implementation_source_manifest_sha256")
    artifact_digest = sha256_bytes(canonical_json(artifact_manifest))
    workflow = build_receipt.get("workflow", {})
    custody_artifact = custody.get("artifact", {})
    custody_workflow = custody.get("workflow", {})
    source_bundle = source_handoff.get("bundle", {})
    source_handoff_receipt_sha256 = sha256_bytes(canonical_json(source_handoff))
    expected_source_receipt = {
        "commit": source_commit, "tree": source_tree, "parent": BASIS_COMMIT,
        "branch": EXPECTED_BRANCH, "ref": EXPECTED_REF,
        "implementation_source_manifest": {
            "schema": SOURCE_SCHEMA, "record_count": 17,
            "sha256": source_digest,
        },
    }
    expected_custody_source = {
        "commit": source_commit, "tree": source_tree, "parent": BASIS_COMMIT,
        "branch": EXPECTED_BRANCH, "ref": EXPECTED_REF,
        "schema": SOURCE_SCHEMA, "record_count": 17,
        "manifest_sha256": source_digest,
    }
    source_record_map = {
        item.get("path"): item for item in source.get("records", [])
        if isinstance(item, dict)
    }
    source_identity_closure = (
        source.get("schema") == SOURCE_SCHEMA
        and source.get("record_count") == 17
        and len(source_record_map) == 17
        and set(source_record_map) == set(SOURCE_PATHS)
        and source_manifest_sha256(source) == source_digest
        and build_receipt.get("source") == expected_source_receipt
        and custody.get("implementation_source") == expected_custody_source
        and source_handoff.get("implementation_source") == {
            "commit": source_commit, "tree": source_tree,
            "parent": BASIS_COMMIT, "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF, "manifest_schema": SOURCE_SCHEMA,
            "manifest_record_count": 17, "manifest_sha256": source_digest,
        }
        and fixture.get("implementation_source") == {
            "schema": SOURCE_SCHEMA, "commit": source_commit,
            "tree": source_tree, "parent": BASIS_COMMIT,
            "record_count": 17, "manifest_sha256": source_digest,
        }
        and retained.get("deck_admission") == {
            "source_ref": f"refs/handoff/wa0-source/{source_commit}",
            "detached_worktree_commit": source_commit,
            "detached_worktree_clean": True,
            "source_manifest_readback_before_each_proof": True,
            "source_bundle_sha256": source_bundle.get("sha256"),
            "source_handoff_receipt_sha256": source_handoff_receipt_sha256,
            "artifact_cache_manifest_sha256": artifact_digest,
        }
    )
    workflow_closure = (
        workflow.get("commit") == source_commit
        and workflow.get("event") == "push"
        and workflow.get("ref") == EXPECTED_REF
        and workflow.get("path") == WORKFLOW_PATH
        and workflow.get("git_blob") == source_record_map.get(
            WORKFLOW_PATH, {}
        ).get("git_blob")
        and custody_workflow == {
            "path": WORKFLOW_PATH, "git_blob": workflow.get("git_blob"),
            "event": "push", "head_branch": EXPECTED_BRANCH,
            "head_sha": source_commit, "run_id": workflow.get("run_id"),
            "run_attempt": workflow.get("run_attempt"), "conclusion": "success",
        }
        and fixture.get("workflow") == {
            "path": WORKFLOW_PATH, "git_blob": workflow.get("git_blob"),
            "run_id": workflow.get("run_id"),
            "run_attempt": workflow.get("run_attempt"),
        }
    )
    artifact_closure = (
        artifact_manifest.get("schema") ==
            "linux-vst-bridge-wf0-artifact-manifest/v1"
        and artifact_manifest.get("implementation_source_manifest_sha256") ==
            source_digest
        and artifact_manifest.get("record_count") == len(
            artifact_manifest.get("records", [])
        )
        and build_receipt.get("artifacts", {}).get(
            "artifact_manifest_sha256"
        ) == artifact_digest
        and build_receipt.get("build", {}).get("build_a", {}).get(
            "artifact_manifest_sha256"
        ) == artifact_digest
        and build_receipt.get("build", {}).get("build_b", {}).get(
            "artifact_manifest_sha256"
        ) == artifact_digest
        and custody.get("inner_envelope", {}).get(
            "artifact_manifest_sha256"
        ) == artifact_digest
        and fixture.get("artifact", {}) == {
            "id": custody_artifact.get("id"),
            "name": custody_artifact.get("name"),
            "manifest_sha256": artifact_digest,
        }
        and custody_artifact.get("workflow_run_id") == workflow.get("run_id")
        and custody_artifact.get("workflow_head_branch") == EXPECTED_BRANCH
        and custody_artifact.get("workflow_head_sha") == source_commit
        and custody_artifact.get("upload_artifact_digest_bare") ==
            custody_artifact.get("raw_wrapper_sha256")
        and custody_artifact.get("raw_github_artifact_zip_sha256") ==
            custody_artifact.get("raw_wrapper_sha256")
        and custody_artifact.get("rest_artifact_digest") ==
            f"sha256:{custody_artifact.get('raw_wrapper_sha256')}"
        and source_handoff.get("bundle", {}).get("name") ==
            f"wa0-execution-source-{source_commit}.bundle"
        and source_handoff.get("bundle", {}).get("advertised_ref") ==
            f"refs/handoff/wa0-source/{source_commit}"
        and source_handoff.get("bundle", {}).get("prerequisite_count") == 0
        and source_handoff.get("bundle", {}).get("self_contained") is True
        and source_handoff.get("bundle", {}).get("git_bundle_verify") == "passed"
    )
    artifact_records = artifact_manifest.get("records", [])
    artifact_record_map = {
        item.get("path"): item for item in artifact_records
        if isinstance(item, dict)
    }
    again_manifest = retained.get("again_bundle_manifest", {})
    build_artifacts = build_receipt.get("artifacts", {})
    pe_receipts = build_artifacts.get("pe_receipts", [])
    pe_record_map = {
        item.get("path"): item for item in pe_receipts if isinstance(item, dict)
    }
    scanner_artifact = artifact_record_map.get("bin/wf0-factory-probe.exe", {})
    again_artifact = artifact_record_map.get(
        "again.vst3/Contents/x86_64-win/again.vst3", {}
    )
    build_plane_closure = (
        build_receipt.get("schema") == "linux-vst-bridge-wf0-windows-build/v1"
        and build_receipt.get("repository") == {
            "full_name": REPOSITORY, "visibility": "private",
            "owner_type": "User",
        }
        and build_receipt.get("runner", {}).get("requested_label") ==
            "windows-2022"
        and build_receipt.get("runner", {}).get("architecture") == "X64"
        and build_receipt.get("runner", {}).get("image_os") == "win22"
        and build_receipt.get("runner", {}).get("windows_product_name") ==
            "Microsoft Windows Server 2022 Datacenter"
        and build_receipt.get("toolchain", {}).get("generator") ==
            "Visual Studio 17 2022"
        and build_receipt.get("toolchain", {}).get("generator_platform") == "x64"
        and build_receipt.get("toolchain", {}).get("generator_toolset") == "v143"
        and build_receipt.get("toolchain", {}).get("platform_toolset") == "v143"
        and build_receipt.get("toolchain", {}).get("host_architecture") == "x64"
        and build_receipt.get("toolchain", {}).get("target_architecture") == "x64"
        and build_receipt.get("toolchain", {}).get("windows_sdk_version") ==
            "10.0.19041.0"
        and build_receipt.get("build", {}).get("configuration") == "Release"
        and build_receipt.get("build", {}).get("msvc_runtime") == "MultiThreaded"
        and build_receipt.get("build", {}).get("msbuild_max_cpu_count") == 1
        and build_receipt.get("build", {}).get("msvc_post_options") == "/MP1"
        and build_receipt.get("build", {}).get(
            "dependency_network_during_configure_build"
        ) is False
        and build_receipt.get("build", {}).get("targets") == [
            "wf0-factory-probe", "wf0-loader-adapter-tests",
            "wa0-fault-fixtures", "again",
        ]
        and build_receipt.get("build", {}).get("comparison", {}).get(
            "level"
        ) == "byte_identical"
        and build_receipt.get("build", {}).get("comparison", {}).get(
            "path_count"
        ) == 35
        and _sdk_receipt_exact(build_receipt.get("vst3_sdk"))
        and fixture.get("vst3_sdk") == build_receipt.get("vst3_sdk")
        and set(artifact_record_map) == ARTIFACT_PATHS
        and len(artifact_records) == len(ARTIFACT_PATHS)
        and artifact_manifest.get("record_count") == len(ARTIFACT_PATHS)
        and set(pe_record_map) == PE_PATHS
        and len(pe_receipts) == len(PE_PATHS)
        and build_artifacts.get("copied_runtime_dependencies") == []
        and {
            item.get("path") for item in build_artifacts.get(
                "again_bundle_roster", []
            ) if isinstance(item, dict)
        } == {f"again.vst3/{path}" for path in AGAIN_BUNDLE_PATHS}
        and len(build_artifacts.get("again_bundle_roster", [])) ==
            len(AGAIN_BUNDLE_PATHS)
        and {
            item.get("path") for item in build_artifacts.get(
                "scanner_and_fault_roster", []
            ) if isinstance(item, dict)
        } == PE_PATHS - {"again.vst3/Contents/x86_64-win/again.vst3"}
        and len(build_artifacts.get("scanner_and_fault_roster", [])) ==
            len(PE_PATHS) - 1
        and again_manifest.get("schema") ==
            "linux-vst-bridge-wf0-bundle-manifest/v1"
        and again_manifest.get("binary_safe_path") ==
            "again.vst3/Contents/x86_64-win/again.vst3"
        and {item.get("path") for item in again_manifest.get("records", [])
             if isinstance(item, dict)} == AGAIN_BUNDLE_PATHS
        and len(again_manifest.get("records", [])) == len(AGAIN_BUNDLE_PATHS)
        and all(
            artifact_record_map.get(item.get("path"), {}).get("sha256") ==
                item.get("sha256")
            and artifact_record_map.get(item.get("path"), {}).get("size") ==
                item.get("size")
            for item in pe_receipts
        )
        and all(
            artifact_record_map.get(f"again.vst3/{item.get('path')}", {}).get(
                "sha256"
            ) == item.get("sha256")
            and artifact_record_map.get(
                f"again.vst3/{item.get('path')}", {}
            ).get("size") == item.get("size")
            for item in again_manifest.get("records", [])
        )
        and all(
            artifact_record_map.get(item.get("path"), {}).get("sha256") ==
                item.get("sha256")
            and artifact_record_map.get(item.get("path"), {}).get("size") ==
                item.get("size")
            for roster in (
                build_artifacts.get("again_bundle_roster", []),
                build_artifacts.get("scanner_and_fault_roster", []),
            )
            for item in roster
        )
        and scanner_artifact.get("role") == "scanner_executable"
        and scanner_artifact.get("sha256") == audio.get("scanner_sha256")
        and scanner_artifact.get("sha256") == component.get("scanner_sha256")
        and again_artifact.get("role") == "positive_fixture_module"
        and again_artifact.get("sha256") ==
            "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
        and again_artifact.get("sha256") == audio.get("module_sha256")
        and again_artifact.get("sha256") == component.get("module_sha256")
        and set(pe_record_map.get("again.vst3/Contents/x86_64-win/again.vst3", {}).get(
            "exports", []
        )) == PE_EXPORTS
        and all(item.get("machine") == "AMD64" and item.get("pe_kind") == "PE32+"
                for item in pe_receipts)
    )
    custody_closure = (
        custody.get("schema") == "linux-vst-bridge-wf0-mac-artifact-custody/v1"
        and custody.get("repository") == {
            "full_name": REPOSITORY, "visibility": "private",
            "owner_type": "User",
        }
        and custody.get("authenticated_private_repository_download") is True
        and custody.get("credentials_retained") is False
        and set(custody.get("explicit_nonclaims", [])) == CUSTODY_NONCLAIMS
        and custody_artifact.get("name") ==
            f"wa0-windows-build-{source_commit}-run-{workflow.get('run_id')}-"
            f"attempt-{workflow.get('run_attempt')}"
        and custody_artifact.get("human_facing_url") ==
            f"https://github.com/{REPOSITORY}/actions/runs/"
            f"{workflow.get('run_id')}/artifacts/{custody_artifact.get('id')}"
        and custody_artifact.get("url") ==
            f"https://api.github.com/repos/{REPOSITORY}/actions/artifacts/"
            f"{custody_artifact.get('id')}"
        and custody_artifact.get("raw_download_route") ==
            f"/repos/{REPOSITORY}/actions/artifacts/"
            f"{custody_artifact.get('id')}/zip"
        and custody_artifact.get("expired") is False
        and custody_artifact.get("digest") ==
            custody_artifact.get("rest_artifact_digest")
        and custody.get("inner_envelope", {}).get("record_count") == 3
        and len(custody.get("inner_envelope", {}).get("records", [])) == 3
        and {item.get("path") for item in custody.get(
            "inner_envelope", {}
        ).get("records", []) if isinstance(item, dict)} == {
            "wf0-payload.zip", "WF0_WINDOWS_BUILD_RECEIPT.json",
            "WF0_WINDOWS_BUILD_RECEIPT.sha256",
        }
        and custody.get("inner_envelope", {}).get("build_receipt_sha256") ==
            sha256_bytes(canonical_json(build_receipt))
        and custody.get("inner_envelope", {}).get("payload_archive_sha256") ==
            build_artifacts.get("payload_archive_sha256")
    )
    source_handoff_closure = (
        source_handoff.get("schema") ==
            "linux-vst-bridge-wa0-source-handoff/v1"
        and source_handoff.get("repository") == REPOSITORY
        and source_handoff.get("wa0_authority") == {
            "design_authority_commit": AUTHORITY_MERGE_COMMIT,
            "design_authority_tree": AUTHORITY_MERGE_TREE,
            "implementation_basis_commit": BASIS_COMMIT,
            "implementation_basis_tree": BASIS_TREE,
        }
        and source_bundle == {
            "name": f"wa0-execution-source-{source_commit}.bundle",
            "advertised_ref": f"refs/handoff/wa0-source/{source_commit}",
            "sha256": source_bundle.get("sha256"),
            "size": source_bundle.get("size"),
            "max_size_bytes": 134217728,
            "prerequisite_count": 0,
            "self_contained": True,
            "git_bundle_verify": "passed",
        }
        and isinstance(source_bundle.get("size"), int)
        and 0 < source_bundle.get("size", 0) <= 134217728
        and SHA256.fullmatch(str(source_bundle.get("sha256", ""))) is not None
        and retained.get("deck_admission", {}).get("source_bundle_sha256") ==
            source_bundle.get("sha256")
        and retained.get("deck_admission", {}).get(
            "source_handoff_receipt_sha256"
        ) == source_handoff_receipt_sha256
    )
    fixture_closure = (
        fixture.get("host") == {
            "hardware": "Steam Deck Galileo", "os": "SteamOS 3.8.16",
            "architecture": "x86_64", "read_only_mode": "enabled",
        }
        and fixture.get("runner_identity_sha256") == RUNNER_DIGEST
        and fixture.get("processor_logical_cid") ==
            "84E8DE5F92554F5396FAE4133C935A18"
        and fixture.get("processor_raw_windows_tuid") ==
            "5FDEE8845592534F96FAE4133C935A18"
        and fixture.get("component_interface") == "Steinberg::Vst::IComponent"
        and fixture.get("component_logical_iid") ==
            "E831FF31F2D54301928EBBEE25697802"
        and fixture.get("component_raw_windows_tuid") ==
            "31FF31E8D5F20143928EBBEE25697802"
        and fixture.get("admitted_interface") ==
            "Steinberg::Vst::IAudioProcessor"
        and fixture.get("admitted_interface_logical_iid") ==
            "42043F99B7DA453CA569E79D9AAEC33D"
        and fixture.get("admitted_interface_raw_windows_tuid") ==
            "993F0442DAB73C45A569E79D9AAEC33D"
        and fixture.get("positive_result") == "scanner_completed"
        and fixture.get("interface_quiescence") is True
        and fixture.get("object_quiescence") is True
        and fixture.get("audio_processor_method_called") is False
        and fixture.get("controller_instance_created") is False
    )
    conditions = [
        (
            retained.get("authority") == {
                "implementation_basis_commit": BASIS_COMMIT,
                "implementation_basis_tree": BASIS_TREE,
                "design_commit": DESIGN_COMMIT,
                "design_tree": DESIGN_TREE,
                "design_blob": DESIGN_BLOB,
                "design_sha256": DESIGN_SHA256,
                "review_github_id": REVIEW_GITHUB_ID,
                "review_result": "DESIGN_CLEAR",
                "approval_blob": APPROVAL_BLOB,
                "authority_merge_commit": AUTHORITY_MERGE_COMMIT,
                "authority_merge_tree": AUTHORITY_MERGE_TREE,
                "accepted_wc0": {
                    "implementation_merge": ACCEPTED_WC0_IMPLEMENTATION_MERGE,
                    "implementation_merge_tree": ACCEPTED_WC0_EVIDENCE_TREE,
                    "source_commit": ACCEPTED_WC0_SOURCE_COMMIT,
                    "source_tree": ACCEPTED_WC0_SOURCE_TREE,
                    "evidence_commit": ACCEPTED_WC0_EVIDENCE_COMMIT,
                    "evidence_tree": ACCEPTED_WC0_EVIDENCE_TREE,
                    "source_manifest_sha256":
                        ACCEPTED_WC0_SOURCE_MANIFEST_SHA256,
                    "scanner_sha256": ACCEPTED_WC0_SCANNER_SHA256,
                    "artifact_manifest_sha256":
                        ACCEPTED_WC0_ARTIFACT_MANIFEST_SHA256,
                },
            }
            and accepted_wc0 == retained.get("authority", {}).get("accepted_wc0")
            and retained.get("runtime_proton_digest") == RUNNER_DIGEST
            and fixture.get("runner_identity_sha256") == RUNNER_DIGEST
            and _sdk_receipt_exact(build_receipt.get("vst3_sdk"))
            and fixture.get("vst3_sdk") == build_receipt.get("vst3_sdk"),
            "BUILD_MANIFEST.authority.accepted_wc0",
        ),
        (
            audio.get("owner") == "AudioProcessorInterfaceLease"
            and audio.get("interface") == {
                "name": "Steinberg::Vst::IAudioProcessor",
                "logical_iid": "42043F99B7DA453CA569E79D9AAEC33D",
                "raw_windows_tuid": "993F0442DAB73C45A569E79D9AAEC33D",
                "pointer_identity_retained": False,
            }
            and fixture.get("admitted_interface") ==
                "Steinberg::Vst::IAudioProcessor"
            and fixture.get("admitted_interface_logical_iid") ==
                "42043F99B7DA453CA569E79D9AAEC33D"
            and fixture.get("admitted_interface_raw_windows_tuid") ==
                "993F0442DAB73C45A569E79D9AAEC33D",
            "AUDIO_PROCESSOR_LEASE.interface",
        ),
        (
            all(positive_states.count(state) == 1 for state in (
                "component_initialized", "audio_processor_query_in_flight",
                "audio_processor_lease_acquired",
                "audio_processor_release_in_flight",
                "audio_processor_lease_retired",
                "audio_interface_quiescence_proved",
                "component_terminate_in_flight",
            ))
            and positive_states.index("component_initialized")
            < positive_states.index("audio_processor_query_in_flight")
            < positive_states.index("audio_processor_lease_acquired")
            < positive_states.index("audio_processor_release_in_flight")
            < positive_states.index("audio_processor_lease_retired")
            < positive_states.index("audio_interface_quiescence_proved")
            < positive_states.index("component_terminate_in_flight"),
            "STAGE_TIMELINE.positive lifecycle order",
        ),
        (
            audio.get("query") == _query_receipt("00000000", True, True)
            and failure_null.get("query") == expected_failure_null["query"]
            and success_null.get("query") == expected_success_null["query"]
            and failure_nonnull.get("query") ==
                expected_failure_nonnull["query"],
            "positive lease and three query-tuple fixture receipts",
        ),
        (
            audio.get("component_owner_reference_baseline") == 1
            and audio.get("lease_acquired") is True
            and audio.get("call_attribution") == {
                "closed_operations": [
                    "query_audio_processor", "release_audio_processor",
                ],
                "new_operation_count": 2,
                "started_count": 2,
                "completed_count": 2,
                "total_operation_count": 22,
                "last_in_flight_operation": None,
            }
            and positive_calls.count("query_audio_processor") == 1
            and positive_calls.count("release_audio_processor") == 1,
            "AUDIO_PROCESSOR_LEASE and positive call ledger",
        ),
        (
            audio.get("release") == {
                "attempted": True, "returned_ordinary": True,
                "reference_count": 1,
            }
            and component.get("component_release") == {
                "attempted": True, "returned_ordinary": True,
                "reference_count": 0, "pointer_cleared": True,
            },
            "audio lease and component release receipts",
        ),
        (
            audio.get("state") == "audio_processor_lease_retired"
            and audio.get("pointer_cleared") is True
            and audio.get("call_in_flight") is False
            and audio.get("callback_ledger_unchanged") is True
            and audio.get("audio_interface_quiescence") is True
            and component.get("audio_processor_lease") == {
                "state": "audio_processor_lease_retired",
                "query_result_u32_hex": "00000000",
                "query_output_nonnull": True,
                "release_reference_count": 1,
                "pointer_cleared": True,
                "quiescence": True,
            }
            and fixture.get("interface_quiescence") is True
            and fixture.get("object_quiescence") is True
            and component.get("object_quiescence", {}).get("value") is True
            and quiescence_facts == {
                "initialize_succeeded": True,
                "terminate_attempted_once_and_returned_ordinary": True,
                "component_release_returned_zero": True,
                "component_call_in_flight": False,
                "component_pointer_cleared": True,
                "host_reference_returned_to_baseline": True,
                "host_owner_final_release_returned_zero": True,
                "host_callback_in_flight": False,
                "callback_ledger_closed": True,
                "audio_interface_quiescent_before_terminate": True,
            },
            "COMPONENT_SESSION.object_quiescence.facts",
        ),
        (
            audio.get("audio_processor_method_called") is False
            and component.get("audio_processor_method_called") is False
            and component.get("controller_instance_created") is False
            and component.get("forbidden_component_method_called") is False
            and fixture.get("audio_processor_method_called") is False
            and fixture.get("controller_instance_created") is False
            and set(audio.get("explicit_nonclaims", [])) == AUDIO_NONCLAIMS
            and len(audio.get("explicit_nonclaims", [])) == len(AUDIO_NONCLAIMS)
            and set(component.get("explicit_nonclaims", [])) ==
                COMPONENT_NONCLAIMS
            and len(component.get("explicit_nonclaims", [])) ==
                len(COMPONENT_NONCLAIMS)
            and call_surface == {
                "closed_plugin_operation_count": 7,
                "new_audio_interface_operation_count": 2,
                "operation_call_counts": {
                    "create_component": 1, "get_controller_class_id": 1,
                    "initialize_component": 1, "query_audio_processor": 1,
                    "release_audio_processor": 1, "terminate_component": 1,
                    "release_component": 1,
                },
                "controller_creation_absent": True,
                "audio_processor_method_calls_absent": True,
                "audio_bus_parameter_state_processing_editor_calls_absent": True,
            }
            and deterministic.get("audio_method_verifier_regression") == {
                "mutated_call_samples_rejected": 3,
                "non_call_samples_accepted": 3,
                "receiver_independent": True,
            }
            and all(
                item.get("forbidden_audio_processor_method_marker_absent") is True
                and item.get("forbidden_component_method_marker_absent") is True
                for item in negatives.values()
            ),
            "positive nonclaims and all nine marker checks",
        ),
        (
            ordinary_negative_exact(
                "wa0-query-failure-null", expected_failure_null,
                "WA0_INTERFACE_QUERY_BLOCKED", 0,
            ),
            "wa0-query-failure-null retained lease and shutdown",
        ),
        (
            ordinary_negative_exact(
                "wa0-query-success-null", expected_success_null,
                "WA0_INTERFACE_QUERY_INCONSISTENT", 0,
            ),
            "wa0-query-success-null retained lease and shutdown",
        ),
        (
            ordinary_negative_exact(
                "wa0-query-failure-nonnull", expected_failure_nonnull,
                "WA0_INTERFACE_QUERY_INCONSISTENT", 1,
            ),
            "wa0-query-failure-nonnull retained cleanup",
        ),
        (abnormal_exact.get("wa0-query-hang") is True,
         "wa0-query-hang observer and suppression receipt"),
        (abnormal_exact.get("wa0-query-crash") is True,
         "wa0-query-crash observer and suppression receipt"),
        (
            unexpected_release == expected_unexpected_release
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "exercise"
            ) == "wa0-release-unexpected-count"
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "fixture"
            ) == "wa0-release-unexpected-count"
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "component_case"
            ) == "exact-again"
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "expected_blocker"
            ) == "WA0_INTERFACE_RELEASE_BLOCKED"
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "observed_blocker"
            ) == "WA0_INTERFACE_RELEASE_BLOCKED"
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "classification"
            ) == "scanner_blocked"
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "last_in_flight_operation"
            ) is None
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "audio_processor_observer_state"
            ) == "audio_processor_retirement_incomplete"
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "audio_interface_quiescence"
            ) is False
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "component_session_closed"
            ) is True
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "call_counts"
            ) == _negative_call_counts(
                release_audio=1, inherited_shutdown=0
            )
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "forbidden_audio_processor_method_marker_absent"
            ) is True
            and negatives.get("wa0-release-unexpected-count", {}).get(
                "forbidden_component_method_marker_absent"
            ) is True
            and release_baseline == {
                "production_helper": "audio_release_matches_component_baseline",
                "expected_component_owner_baseline": 1,
                "accepted_counts": [1],
                "rejected_counts": [0, 2, 0xffffffff],
                "static_asserts_bound_to_production_helper": True,
            }
            and suppressed("wa0-release-unexpected-count"),
            "wa0-release-unexpected-count retained retirement state",
        ),
        (abnormal_exact.get("wa0-release-hang") is True,
         "wa0-release-hang observer and suppression receipt"),
        (abnormal_exact.get("wa0-release-crash") is True,
         "wa0-release-crash observer and suppression receipt"),
        (
            all(suppressed(name) for name in {
                "wa0-query-hang", "wa0-query-crash",
                "wa0-release-unexpected-count", "wa0-release-hang",
                "wa0-release-crash",
            }),
            "five blocked interface-retirement shutdown ledgers",
        ),
        (
            component.get("processor") == {
                "logical_cid": "84E8DE5F92554F5396FAE4133C935A18",
                "raw_windows_tuid": "5FDEE8845592534F96FAE4133C935A18",
                "requested_interface": "Steinberg::Vst::IComponent",
                "requested_interface_logical_iid":
                    "E831FF31F2D54301928EBBEE25697802",
                "requested_interface_raw_windows_tuid":
                    "31FF31E8D5F20143928EBBEE25697802",
            }
            and component.get("controller") == {
                "logical_cid": "D39D5B65D7AF42FA843F4AC841EB04F0",
                "raw_windows_tuid": "655B9DD3AFD7FA42843F4AC841EB04F0",
                "complete_output_zero_initialized": True,
                "queried_before_initialize": True,
                "result_u32_hex": "00000000", "matched": True,
            }
            and component.get("create") == {
                "attempted": True, "result_u32_hex": "00000000",
                "output_nonnull": True, "tuple_consistent": True,
            }
            and component.get("component_state") == "component_released"
            and component.get("audio_processor_lease") == {
                "state": "audio_processor_lease_retired",
                "query_result_u32_hex": "00000000",
                "query_output_nonnull": True,
                "release_reference_count": 1,
                "pointer_cleared": True, "quiescence": True,
            }
            and component.get("call_attribution") == {
                "closed_operation_count": 22,
                "new_wa0_operation_count": 2,
                "started_count": 22, "completed_count": 22,
                "last_in_flight_operation": None,
            }
            and component.get("callback_ledger", {}).get("capacity") == 64
            and component.get("callback_ledger", {}).get("closed") is True
            and component.get("callback_ledger", {}).get(
                "component_originated_reference_sequence"
            ) == [2, 1]
            and component.get("callback_ledger", {}).get("host_name") ==
                "Linux VST Bridge WC0"
            and component.get("callback_ledger", {}).get("output_failed") is False
            and component.get("callback_ledger", {}).get("overflowed") is False
            and component.get("callback_ledger", {}).get("owner_final_release") == 0
            and component.get("callback_ledger", {}).get("record_count") == 3
            and component.get("callback_ledger", {}).get("records") ==
                positive_callbacks
            and component.get("callback_ledger", {}).get(
                "unchanged_across_audio_processor_lease"
            ) is True
            and component.get("callback_ledger", {}).get(
                "unexpected_host_object_request"
            ) is False
            and component.get("callback_ledger", {}).get("wrong_thread") is False
            and component.get("callback_ledger_sha256") == sha256_bytes(
                canonical_json(component.get("callback_ledger", {}))
            )
            and component.get("audio_processor_method_called") is False
            and component.get("controller_instance_created") is False
            and component.get("forbidden_component_method_called") is False
            and set(component.get("explicit_nonclaims", [])) ==
                COMPONENT_NONCLAIMS
            and len(component.get("explicit_nonclaims", [])) ==
                len(COMPONENT_NONCLAIMS)
            and component.get("inherited_wf0_regression", {}).get(
                "clean_in_process_shutdown") is True
            and component.get("host_reference_sequence") == [1, 2, 1, 0]
            and component.get("component_release") == {
                "attempted": True, "returned_ordinary": True,
                "reference_count": 0, "pointer_cleared": True,
            }
            and component.get("inherited_wf0_regression", {}).get(
                "factory_release_order"
            ) == ["release_factory_3", "release_factory_2", "release_factory_base"]
            and component.get("inherited_wf0_regression", {}).get(
                "factory_vendor"
            ) == "Steinberg Media Technologies"
            and component.get("inherited_wf0_regression", {}).get(
                "ordered_class_census"
            ) == EXPECTED_CENSUS
            and component.get("inherited_wf0_regression", {}).get("module_exit") == {
                "present": True, "called": True, "result": True,
            }
            and component.get("inherited_wf0_regression", {}).get("module_unload") == {
                "attempted": True, "succeeded": True,
            }
            and component.get("initialize") == {
                "attempted": True, "result_u32_hex": "00000000",
                "succeeded": True,
            }
            and component.get("terminate") == {
                "attempted": True, "result_u32_hex": "00000000",
                "returned_ordinary": True,
            }
            and component.get("object_quiescence", {}).get("value") is True
            and component.get("call_attribution", {}).get(
                "last_in_flight_operation"
            ) is None,
            "COMPONENT_SESSION accepted-WC0 bounded regression",
        ),
        (
            all_negative_cleanup
            and timeline.get("positive_cleanup") == {
                "owned_descendants_zero": True, "process_group_empty": True,
            }
            and timeline.get("positive_environment_retired") is True
            and timeline.get("negative_exercise_count") == 8
            and timeline.get("total_live_exercise_count") == 9,
            "STAGE_TIMELINE nine exercise cleanup receipts",
        ),
        (
            source_identity_closure
            and workflow_closure
            and artifact_closure
            and build_plane_closure
            and custody_closure
            and source_handoff_closure
            and fixture_closure
            and retained.get("schema") ==
                "linux-vst-bridge-wa0-retained-build-manifest/v1"
            and retained.get("repository") == REPOSITORY
            and audio.get("schema") ==
                "linux-vst-bridge-wa0-audio-processor-lease/v1"
            and component.get("schema") ==
                "linux-vst-bridge-wa0-component-regression/v1"
            and timeline.get("schema") ==
                "linux-vst-bridge-wa0-stage-timeline/v1"
            and fixture.get("schema") == "linux-vst-bridge-wa0-fixture/v1"
            and audio.get("implementation_source_identity") == {
                "schema": SOURCE_SCHEMA, "commit": source_commit,
                "tree": source_tree, "record_count": 17,
                "manifest_sha256": source_digest,
            }
            and component.get("implementation_source_identity") == {
                "schema": SOURCE_SCHEMA, "commit": source_commit,
                "tree": source_tree, "record_count": 17,
                "manifest_sha256": source_digest,
            }
            and retained.get("audio_processor_lease_sha256") ==
                sha256_bytes(canonical_json(audio))
            and retained.get("component_session_sha256") ==
                sha256_bytes(canonical_json(component))
            and audio.get("stage_timeline_sha256") ==
                sha256_bytes(canonical_json(timeline))
            and component.get("stage_timeline_sha256") ==
                sha256_bytes(canonical_json(timeline))
            and retained.get("protected_state_equal") is True
            and retained.get("zero_owned_process_environment_residue") is True
            and retained.get("interface_quiescence") is True
            and retained.get("object_quiescence") is True
            and retained.get("negative_fault_roster_complete") is True
            and retained.get("positive_interface_lease_complete") is True
            and retained.get("no_deck_github") == {
                "github_credentials_received": False,
                "github_operations": 0,
                "ssh_agent_forwarded": False,
            }
            and fixture.get("protected_state_equal") is True
            and fixture.get("deck_github_operations") == 0,
            "source/build/custody/Deck/evidence identity and preservation joins",
        ),
    ]
    if len(conditions) != len(PROOF_ROWS):
        fail("WA0 proof-condition roster differs")
    result = []
    for index, (claim, (passed, evidence)) in enumerate(
        zip(PROOF_ROWS, conditions), 1
    ):
        if passed is not True:
            fail(f"WA0 proof row {index} is not established: {claim}")
        result.append({
            "row": index, "claim": claim, "result": "passed",
            "evidence": evidence,
        })
    return result


def _proof_disposition_mutation_regression(
    retained: dict[str, Any], audio: dict[str, Any],
    component: dict[str, Any], timeline: dict[str, Any], fixture: dict[str, Any],
) -> dict[str, Any]:
    """Mutate every proof-owner family and require its first owning row."""

    def assign(root: Any, path: tuple[Any, ...], value: Any) -> None:
        target = root
        for key in path[:-1]:
            target = target[key]
        target[path[-1]] = value

    def field(owner: int, path: tuple[Any, ...], value: Any):
        def mutation(*copies: dict[str, Any]) -> None:
            assign(copies[owner], path, value)
        return mutation

    def negative(name: str, path: tuple[Any, ...], value: Any):
        def mutation(*copies: dict[str, Any]) -> None:
            item = next(
                entry for entry in copies[3]["negative_results"]
                if entry["exercise"] == name
            )
            assign(item, path, value)
        return mutation

    def lifecycle_duplicate(*copies: dict[str, Any]) -> None:
        item = next(
            entry for entry in copies[3]["positive"]
            if entry.get("event") == "lifecycle"
            and entry.get("state") == "audio_processor_lease_acquired"
        )
        item["state"] = "audio_processor_query_in_flight"

    cases = (
        ("runtime_prerequisite_digest", 1,
         field(0, ("runtime_proton_digest",), "0" * 64)),
        ("fixture_interface_identity", 2,
         field(4, ("admitted_interface_logical_iid",), "0" * 32)),
        ("positive_lifecycle_cardinality", 3, lifecycle_duplicate),
        ("positive_query_tuple_consistency", 4,
         field(1, ("query", "tuple_consistent"), False)),
        ("negative_query_tuple_consistency", 4,
         negative("wa0-query-success-null", (
             "audio_processor_lease", "query", "tuple_consistent"
         ), True)),
        ("exact_lease_call_attribution", 5,
         field(1, ("call_attribution", "started_count"), 1)),
        ("audio_release_ordinary_return", 6,
         field(1, ("release", "returned_ordinary"), False)),
        ("callback_ledger_unchanged", 7,
         field(1, ("callback_ledger_unchanged",), False)),
        ("component_audio_method_absence", 8,
         field(2, ("audio_processor_method_called",), True)),
        ("fixture_audio_method_absence", 8,
         field(4, ("audio_processor_method_called",), True)),
        ("failure_null_cleanup", 9,
         negative("wa0-query-failure-null", (
             "audio_processor_lease", "release", "attempted"
         ), True)),
        ("success_null_cleanup", 10,
         negative("wa0-query-success-null", (
             "audio_processor_lease", "release", "attempted"
         ), True)),
        ("failure_nonnull_cleanup", 11,
         negative("wa0-query-failure-nonnull", (
             "audio_processor_lease", "release", "returned_ordinary"
         ), False)),
        ("query_timeout_suppression", 12,
         negative("wa0-query-hang", ("call_counts", "terminate_component"), 1)),
        ("query_crash_suppression", 13,
         negative("wa0-query-crash", ("classification",), "scanner_blocked")),
        ("unexpected_release_attempt", 14,
         negative("wa0-release-unexpected-count", (
             "audio_processor_lease", "release", "attempted"
         ), False)),
        ("release_timeout_suppression", 15,
         negative("wa0-release-hang", ("call_counts", "free_library"), 1)),
        ("release_crash_suppression", 16,
         negative("wa0-release-crash", ("call_counts", "release_component"), 1)),
        ("all_blocked_shutdown_suppression", 14,
         negative("wa0-release-unexpected-count", (
             "inherited_shutdown", "physical_containment_only"
         ), False)),
        ("accepted_wc0_processor_identity", 18,
         field(2, ("processor", "logical_cid"), "0" * 32)),
        ("accepted_wc0_call_counts", 18,
         field(2, ("call_attribution", "started_count"), 21)),
        ("accepted_wc0_class_census", 18,
         field(2, (
             "inherited_wf0_regression", "ordered_class_census", 0, "name"
         ), "AGain SideChain VST3")),
        ("physical_cleanup", 19,
         field(3, ("positive_cleanup", "process_group_empty"), False)),
        ("source_bundle_digest_join", 20,
         field(0, ("source_handoff_receipt", "bundle", "sha256"), "0" * 64)),
        ("source_handoff_receipt_join", 20,
         field(0, ("deck_admission", "source_handoff_receipt_sha256"),
               "0" * 64)),
        ("source_build_custody_join", 20,
         field(0, (
             "mac_artifact_custody_receipt", "implementation_source", "commit"
         ), "0" * 40)),
        ("workflow_identity_join", 20,
         field(0, (
             "mac_artifact_custody_receipt", "workflow", "run_id"
         ), "1")),
        ("artifact_identity_join", 20,
         field(4, ("artifact", "manifest_sha256"), "0" * 64)),
        ("runner_fixture_join", 20,
         field(0, (
             "windows_build_receipt", "runner", "requested_label"
         ), "windows-latest")),
    )
    exact_rows: list[int] = []
    for name, row, mutation in cases:
        copies = tuple(copy.deepcopy(value) for value in (
            retained, audio, component, timeline, fixture,
        ))
        mutation(*copies)
        try:
            _proof_disposition(*copies)
        except WF0Error as error:
            if not str(error).startswith(f"WA0 proof row {row} is not established:"):
                fail(
                    f"WA0 proof mutation {name} failed at an unexpected row: "
                    f"{error}"
                )
            exact_rows.append(row)
        else:
            fail(f"WA0 proof mutation was accepted: {name}")
    return {
        "cases": [name for name, _, _ in cases],
        "passed": len(cases), "failed": 0,
        "rejected_at_exact_row": exact_rows,
    }


def command_tree(source_commit: str) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", f"{source_commit}^{{tree}}"],
        cwd=repo_root(), text=True,
    ).strip()


def _negative_summary(negative: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "exercise": item["exercise"],
            "fixture": item["fixture"],
            "component_case": item["component_case"],
            "expected_blocker": item["expected_blocker"],
            "observed_blocker": item["observed_blocker"],
            "classification": item["classification"],
            "last_in_flight_operation": item["last_in_flight_operation"],
            "audio_processor_observer_state":
                item["audio_processor_observer_state"],
            "audio_interface_quiescence": item["audio_interface_quiescence"],
            "component_session_closed": item["component_session_closed"],
            "audio_processor_lease": item["audio_processor_lease"],
            "call_counts": item["call_counts"],
            "forbidden_component_method_marker_absent":
                item["forbidden_component_method_marker_absent"],
            "forbidden_audio_processor_method_marker_absent":
                item["forbidden_audio_processor_method_marker_absent"],
            "inherited_shutdown": item["inherited_shutdown"],
            "cleanup": item["cleanup"],
            "environment_retired": item["environment_retired"],
        }
        for item in negative["live_fault_results"]
    ]


def render_packet(
    source_commit: str,
    build: dict[str, Any],
    negative: dict[str, Any],
    positive: dict[str, Any],
    audio_processor_lease: dict[str, Any],
    component_session: dict[str, Any],
    timeline: dict[str, Any],
    preflight: dict[str, Any],
) -> None:
    root = repo_root() / PACKET
    if root.exists() or root.is_symlink():
        fail("WA0 evidence root already exists; implicit overwrite is prohibited")
    root.mkdir(parents=True)

    source = build["implementation_source_manifest"]
    source_digest = build["implementation_source_manifest_sha256"]
    if (
        source.get("schema") != SOURCE_SCHEMA
        or source.get("commit") != source_commit
        or source.get("record_count") != 17
        or source_manifest_sha256(source) != source_digest
        or build["source_tree"] != command_tree(source_commit)
    ):
        fail("WA0 evidence source identity differs from build/live identity")

    receipt = build["build_receipt"]
    custody = build["mac_custody"]
    source_handoff = build["source_handoff"]
    artifact = custody["artifact"]
    toolchain = receipt["toolchain"]
    sdk = receipt["vst3_sdk"]
    comparison = receipt["build"]["comparison"]
    pe = receipt["artifacts"]["pe_receipts"]
    scanner = next(
        item for item in pe if item["path"] == "bin/wf0-factory-probe.exe"
    )
    again = next(
        item for item in pe
        if item["path"] == "again.vst3/Contents/x86_64-win/again.vst3"
    )
    negative_summary = _negative_summary(negative)

    timeline = dict(timeline)
    timeline["negative_results"] = negative_summary
    timeline["negative_exercise_count"] = len(negative_summary)
    timeline["total_live_exercise_count"] = len(negative_summary) + 1
    timeline["positive_cleanup"] = positive["cleanup"]
    timeline["positive_environment_retired"] = positive["retirement"]["stage_absent"]
    final_timeline_sha256 = sha256_bytes(canonical_json(timeline))
    # The normalizer can only hash the positive-only timeline.  Evidence owns
    # the final retained timeline after the negative family is appended, so it
    # replaces both cross-file references before any object hash or handoff
    # receipt is computed.  The dictionaries are intentionally updated in
    # place because run.py later hashes these same final evidence objects.
    audio_processor_lease["stage_timeline_sha256"] = final_timeline_sha256
    component_session["stage_timeline_sha256"] = final_timeline_sha256

    deterministic = negative["deterministic"]
    retained_deterministic = {
        "cases": deterministic["cases"],
        "passed": deterministic["passed"],
        "failed": deterministic["failed"],
        "component_call_surface": deterministic["component_call_surface"],
        "audio_method_verifier_regression":
            deterministic["audio_method_verifier_regression"],
        "release_baseline_regression":
            deterministic["release_baseline_regression"],
        "evidence_allowlist_regression":
            deterministic["evidence_allowlist_regression"],
    }

    basis_md = md("WA0 implementation basis", f"""The implementation is the
direct child of merged WA0 design-authority/readback basis `{BASIS_COMMIT}` at
tree `{BASIS_TREE}`. The immutable `wa0-design-v1` identity is commit
`{DESIGN_COMMIT}`, tree `{DESIGN_TREE}`, Git blob `{DESIGN_BLOB}`, and SHA-256
`{DESIGN_SHA256}`. Independent GitHub review `{REVIEW_GITHUB_ID}` returned
`DESIGN_CLEAR`; the approval blob is `{APPROVAL_BLOB}`. The authority merge is
`{AUTHORITY_MERGE_COMMIT}` at tree `{AUTHORITY_MERGE_TREE}`.

Implementation branch/ref: `{EXPECTED_BRANCH}` / `{EXPECTED_REF}`.
Implementation source commit/tree: `{source_commit}` / `{build['source_tree']}`.
Exact 17-record `{SOURCE_SCHEMA}` digest: `{source_digest}`.""")

    build_md = md("WA0 Windows build and custody", f"""Private workflow
`{WORKFLOW_PATH}` at Git blob `{receipt['workflow']['git_blob']}` ran as exact
run `{receipt['workflow']['run_id']}`, attempt
`{receipt['workflow']['run_attempt']}`, on `windows-2022` for this source.

The supported toolchain was Visual Studio `{toolchain['visual_studio_version']}`
with MSVC `{toolchain['cl_version']}`, v143, x64, Windows SDK
`{toolchain['windows_sdk_version']}`, and CMake `{toolchain['cmake_version']}`.
The official VST3 SDK was `{sdk['root_commit']}` at tree `{sdk['root_tree']}`
with all seven locked recursive submodules clean and no source patch.

Two distinct clean build roots compared `{comparison['path_count']}` complete
transfer paths at level `{comparison['level']}`. PE/export/import/dependency
verification passed. Scanner SHA-256: `{scanner['sha256']}`. Exact AGain module
SHA-256: `{again['sha256']}`.

The exact Actions artifact is ID `{artifact['id']}`, name `{artifact['name']}`,
upload-action bare digest `{artifact['upload_artifact_digest_bare']}`, REST
digest `{artifact['rest_artifact_digest']}`, and raw wrapper SHA-256
`{artifact['raw_wrapper_sha256']}`. The three values name the same bytes under
their typed representations. This is byte custody, not signing, provenance,
SLSA, or release suitability.""")

    environment_md = md("WA0 disposable environments", f"""The exact eight
focused negative exercises and one positive AGain transaction each used a
fresh, marker-bound `.wf0-factory-census.stage-<32-lowercase-hex-run-id>` under
the accepted WF0 environment owner. Each environment bound the exact WA0 source
manifest `{source_digest}`, artifact manifest
`{build['artifact_manifest_sha256']}`, Runtime/Proton identity, scanner, module,
and run.

No separate bootstrap workload ran. Every exercise drained its exact owned
process tree and retired its exact disposable environment. The inherited
environment-marker and artifact-cache ownership protocols were reused without
algorithm or wire-schema changes.""")

    launch_md = md("WA0 interface lease and process result", f"""The accepted
Runtime 4 / Proton 11 route retained launch-critical digest `{RUNNER_DIGEST}`.
The positive scanner completed all 22 closed calls with no operation left in
flight. After accepted WC0 component initialization, it queried the exact
`IAudioProcessor` IID once, acquired one non-null lease, released that lease
once to component-owner reference baseline 1, cleared the interface pointer,
and proved audio-interface quiescence before WC0 termination.

The inherited host reference sequence remained 1 -> 2 -> 1 -> 0; final
`IComponent` release returned 0, followed by reverse factory releases,
`ExitDll`, and `FreeLibrary`. Query/release timeouts, crashes, and unexpected
release counts used physical containment and made no clean in-process shutdown
claim.""")

    negative_lines = [
        f"- `{item['exercise']}` -> `{item['observed_blocker']}`; "
        f"classification `{item['classification']}`; in flight "
        f"`{item['last_in_flight_operation'] or 'none'}`; clean in-process "
        f"shutdown `{str(item['inherited_shutdown']['clean_in_process_shutdown']).lower()}`"
        for item in negative_summary
    ]
    negative_md = md("WA0 focused negative proofs", f"""All
`{negative['deterministic']['passed']}` deterministic owner checks passed. The
exact eight WA0 fault fixtures ran live; the WF0 and WC0 negative families and
standalone adapter runtime exercise were intentionally not replayed.

{chr(10).join(negative_lines)}

The failure/null and success/null query cases proved no lease and permitted
ordinary WC0 teardown. Failure/non-null retained and released the anomalous
reference once while preserving the query inconsistency as primary. Unexpected
release count 2 and every unmatched query/release operation prevented later
WC0 terminate, component/factory release, `ExitDll`, and `FreeLibrary`.""")

    snapshot = preflight["protected_snapshot"]
    preservation_md = md("WA0 protected-state preservation", f"""The accepted
WR0 environment identity remained `{snapshot['wr0']['environment_identity']}`
and its Runtime/Proton identity remained `{snapshot['wr0']['runner_identity']}`.
Bitwig remained unlaunched at version `{snapshot['bitwig']['version']}` with
application commit `{snapshot['bitwig']['commit']}` and runtime commit
`{snapshot['bitwig']['runtime_commit']}`.

SteamOS read-only posture, accepted runner assets, historical evidence,
protected configuration projections, and unrelated processes remained exact.
No Deck GitHub login, fetch, push, API call, PR operation, or Actions artifact
download occurred; no credential or forwarded SSH agent reached the Deck.""")

    findings_md = md("WA0 findings and claim ceiling", """WA0 establishes only
that the exact initialized AGain processor component exposes the mandatory
`Steinberg::Vst::IAudioProcessor` interface through the exact query, and that
one acquired reference can be retired to the component-owner baseline before
the accepted WC0 lifecycle and inherited module shutdown continue.

No `IAudioProcessor` method was called. WA0 did not create a controller; query
or connect `IConnectionPoint`; access buses, parameters, state, processing,
audio, events, timing, automation, presets, or an editor; create IPC or a native
proxy; run Bitwig or Serum; authorize, package, sign, or select a runner; test
another plug-in or DAW; or establish general VST3 or Linux compatibility.""")

    sanitization_md = md("WA0 evidence sanitization", """The packet contains
only allow-listed public authority/source/dependency identities, safe-relative
artifact paths, hashes, typed public workflow metadata, normalized lease and
component-regression facts, bounded proof dispositions, and explicit nonclaims.

It contains no credentials, private absolute paths, private host/network
identifiers, raw process or thread identifiers, pointers, handles, addresses,
compiled binaries, complete prefixes, proprietary state, SDK source, source
bundle, artifact archive, build tree, installer, or commercial plug-in. All
fourteen files were roster-, hash-, UTF-8-, NUL-, size-, JSON-, Markdown-, and
redaction-checked.""")

    retained_build = {
        "schema": "linux-vst-bridge-wa0-retained-build-manifest/v1",
        "repository": REPOSITORY,
        "authority": {
            "implementation_basis_commit": BASIS_COMMIT,
            "implementation_basis_tree": BASIS_TREE,
            "design_commit": DESIGN_COMMIT,
            "design_tree": DESIGN_TREE,
            "design_blob": DESIGN_BLOB,
            "design_sha256": DESIGN_SHA256,
            "review_github_id": REVIEW_GITHUB_ID,
            "review_result": "DESIGN_CLEAR",
            "approval_blob": APPROVAL_BLOB,
            "authority_merge_commit": AUTHORITY_MERGE_COMMIT,
            "authority_merge_tree": AUTHORITY_MERGE_TREE,
            "accepted_wc0": preflight["authority"]["accepted_wc0"],
        },
        "implementation_source_manifest": source,
        "implementation_source_manifest_sha256": source_digest,
        "implementation_source_tree": build["source_tree"],
        "windows_build_receipt": receipt,
        "mac_artifact_custody_receipt": custody,
        "source_handoff_receipt": source_handoff,
        "artifact_manifest": build["artifact_manifest"],
        "again_bundle_manifest": build["again_bundle_manifest"],
        "deck_admission": {
            "source_ref": f"refs/handoff/wa0-source/{source_commit}",
            "detached_worktree_commit": source_commit,
            "detached_worktree_clean": True,
            "source_manifest_readback_before_each_proof": True,
            "source_bundle_sha256": source_handoff["bundle"]["sha256"],
            "source_handoff_receipt_sha256": sha256_bytes(
                canonical_json(source_handoff)
            ),
            "artifact_cache_manifest_sha256": build["artifact_manifest_sha256"],
        },
        "runtime_proton_digest": RUNNER_DIGEST,
        "deterministic_regression": retained_deterministic,
        "audio_processor_lease_sha256": sha256_bytes(
            canonical_json(audio_processor_lease)
        ),
        "component_session_sha256": sha256_bytes(canonical_json(component_session)),
        "negative_fault_roster_complete": True,
        "positive_interface_lease_complete": True,
        "interface_quiescence": True,
        "object_quiescence": True,
        "zero_owned_process_environment_residue": True,
        "protected_state_equal": True,
        "no_deck_github": preflight["no_deck_github"],
    }

    fixture = {
        "schema": "linux-vst-bridge-wa0-fixture/v1",
        "host": preflight["fixture"],
        "implementation_source": {
            "schema": SOURCE_SCHEMA,
            "commit": source_commit,
            "tree": build["source_tree"],
            "parent": BASIS_COMMIT,
            "record_count": 17,
            "manifest_sha256": source_digest,
        },
        "workflow": {
            "path": WORKFLOW_PATH,
            "git_blob": receipt["workflow"]["git_blob"],
            "run_id": receipt["workflow"]["run_id"],
            "run_attempt": receipt["workflow"]["run_attempt"],
        },
        "artifact": {
            "id": artifact["id"], "name": artifact["name"],
            "manifest_sha256": build["artifact_manifest_sha256"],
        },
        "vst3_sdk": sdk,
        "runner_identity_sha256": RUNNER_DIGEST,
        "processor_logical_cid": "84E8DE5F92554F5396FAE4133C935A18",
        "processor_raw_windows_tuid": "5FDEE8845592534F96FAE4133C935A18",
        "component_interface": "Steinberg::Vst::IComponent",
        "component_logical_iid": "E831FF31F2D54301928EBBEE25697802",
        "component_raw_windows_tuid": "31FF31E8D5F20143928EBBEE25697802",
        "admitted_interface": "Steinberg::Vst::IAudioProcessor",
        "admitted_interface_logical_iid": "42043F99B7DA453CA569E79D9AAEC33D",
        "admitted_interface_raw_windows_tuid": "993F0442DAB73C45A569E79D9AAEC33D",
        "positive_result": positive["classification"],
        "interface_quiescence": True,
        "object_quiescence": True,
        "audio_processor_method_called": False,
        "controller_instance_created": False,
        "protected_state_equal": True,
        "deck_github_operations": 0,
    }
    retained_build["proof_disposition"] = _proof_disposition(
        retained_build, audio_processor_lease, component_session, timeline, fixture
    )
    retained_deterministic["proof_disposition_regression"] = (
        _proof_disposition_mutation_regression(
            retained_build, audio_processor_lease, component_session,
            timeline, fixture,
        )
    )
    retained_build["proof_row_count"] = len(PROOF_ROWS)

    files = {
        "BASIS.md": basis_md,
        "BUILD.md": build_md,
        "BUILD_MANIFEST.json": canonical_json(retained_build),
        "ENVIRONMENT.md": environment_md,
        "LAUNCH_AND_PROCESS.md": launch_md,
        "AUDIO_PROCESSOR_LEASE.json": canonical_json(audio_processor_lease),
        "COMPONENT_SESSION.json": canonical_json(component_session),
        "STAGE_TIMELINE.json": canonical_json(timeline),
        "NEGATIVE_TESTS.md": negative_md,
        "PRESERVATION.md": preservation_md,
        "FINDINGS.md": findings_md,
        "SANITIZATION.md": sanitization_md,
        "fixture.json": canonical_json(fixture),
    }
    for name, data in files.items():
        write_atomic(root / name, data, mode=0o644)
    hash_names = sorted(files, key=lambda value: value.encode())
    write_atomic(
        root / "hashes.sha256",
        ("\n".join(
            f"{sha256_file(root / name)}  {name}" for name in hash_names
        ) + "\n").encode("utf-8"),
        mode=0o644,
    )
    validate_packet(source_commit, treeish=source_commit)


def validate_packet(source_commit: str, *, treeish: str | None = None) -> None:
    root = repo_root() / PACKET
    if not root.is_dir() or root.is_symlink():
        fail("WA0 evidence packet root is absent or unsafe")
    observed = sorted(item.name for item in root.iterdir())
    if observed != sorted(EVIDENCE_FILES):
        fail(f"WA0 evidence packet roster differs: {observed}")
    validate_evidence_allowlist(root)
    expected_names = sorted(
        (name for name in EVIDENCE_FILES if name != "hashes.sha256"),
        key=lambda value: value.encode(),
    )
    lines = (root / "hashes.sha256").read_text(encoding="utf-8").splitlines()
    if lines != [f"{sha256_file(root / name)}  {name}" for name in expected_names]:
        fail("WA0 evidence hash manifest differs")

    retained = json.loads((root / "BUILD_MANIFEST.json").read_bytes())
    regenerated = source_manifest(source_commit, treeish=treeish or source_commit)
    audio = json.loads((root / "AUDIO_PROCESSOR_LEASE.json").read_bytes())
    component = json.loads((root / "COMPONENT_SESSION.json").read_bytes())
    timeline = json.loads((root / "STAGE_TIMELINE.json").read_bytes())
    fixture = json.loads((root / "fixture.json").read_bytes())
    timeline_sha256 = sha256_bytes(canonical_json(timeline))
    derived_proof = _proof_disposition(
        retained, audio, component, timeline, fixture
    )
    derived_proof_regression = _proof_disposition_mutation_regression(
        retained, audio, component, timeline, fixture
    )
    if (
        retained.get("implementation_source_manifest") != regenerated
        or retained.get("implementation_source_manifest_sha256")
        != source_manifest_sha256(regenerated)
        or retained.get("proof_row_count") != 20
        or retained.get("proof_disposition") != derived_proof
        or retained.get("deterministic_regression", {}).get(
            "proof_disposition_regression"
        ) != derived_proof_regression
        or audio.get("schema") != "linux-vst-bridge-wa0-audio-processor-lease/v1"
        or audio.get("state") != "audio_processor_lease_retired"
        or audio.get("release", {}).get("reference_count") != 1
        or audio.get("audio_interface_quiescence") is not True
        or audio.get("audio_processor_method_called") is not False
        or audio.get("stage_timeline_sha256") != timeline_sha256
        or component.get("schema") != "linux-vst-bridge-wa0-component-regression/v1"
        or component.get("object_quiescence", {}).get("value") is not True
        or component.get("component_release", {}).get("reference_count") != 0
        or component.get("callback_ledger", {}).get("closed") is not True
        or component.get("stage_timeline_sha256") != timeline_sha256
        or timeline.get("negative_exercise_count") != 8
        or timeline.get("total_live_exercise_count") != 9
    ):
        fail("WA0 retained identities, lease, regression, or 20-row proof differs")

    prohibited = re.compile(
        rb"(?:/home/|/Users/|192\.168\."
        rb"|BEGIN (?:RSA |OPENSSH )?PRIVATE KEY|github_pat_|ghp_|compatdata|\.ssh/"
        rb"|SSH_AUTH_SOCK|0x[0-9a-f]{8,16})",
        re.IGNORECASE,
    )
    binary_magic = (b"MZ", b"\x7fELF", b"PK\x03\x04")
    for path in root.iterdir():
        if (
            not path.is_file() or path.is_symlink()
            or path.stat().st_size > 2 * 1024 * 1024
        ):
            fail(f"WA0 evidence object is unsafe or oversized: {path.name}")
        data = path.read_bytes()
        if b"\0" in data or prohibited.search(data) or data.startswith(binary_magic):
            fail(f"WA0 evidence contains private, pointer, or binary data: {path.name}")
        data.decode("utf-8", "strict")
        if path.suffix == ".md" and data.count(b"```") % 2:
            fail(f"Markdown fence is unbalanced: {path.name}")


if __name__ == "__main__":
    raise SystemExit("evidence.py is a library; use run.py")

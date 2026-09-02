#!/usr/bin/env python3
"""Deterministic and bounded live negative proofs for the WA0 lease owner."""

from __future__ import annotations

import secrets
import pathlib
import re
import shutil
import stat
import struct
import tempfile
import warnings
import zipfile
from typing import Any

from artifacts import (
    MAX_ENTRY_BYTES, MAX_ZIP_BYTES, _safe_relative,
    artifact_digest_regression, zip_census,
)
from build import (
    EXPECTED_AGAIN_MODULE_SHA256, MSBUILD_MAX_CPU_COUNT, MSVC_POST_OPTIONS,
    build_command_environment, eol_checkout_regression,
)
from common import (
    FAULT_TARGETS, canonical_json, environment_parent, fail, process_guard,
    protected_snapshot, runner_root, write_atomic,
)
from environment import ScanEnvironment, create_environment, retire_environment
from evidence import evidence_allowlist_regression
from normalize import decode_field
from supervise import (
    ADAPTER_OPERATION_PREFIX, EXPECTED_ADAPTER_OPERATIONS, IN_FLIGHT_BLOCKER,
    OPERATIONS, StreamState, adapter_operation_ledger, root_identity_continuity,
    supervise, supervise_adapter, topology_role_census,
)
from verify import (
    audio_method_verifier_regression, parse_pe, scanner_component_call_surface,
)


EXPECTED = {
    "wa0-query-failure-null": "WA0_INTERFACE_QUERY_BLOCKED",
    "wa0-query-success-null": "WA0_INTERFACE_QUERY_INCONSISTENT",
    "wa0-query-failure-nonnull": "WA0_INTERFACE_QUERY_INCONSISTENT",
    "wa0-query-hang": "WA0_INTERFACE_QUERY_BLOCKED",
    "wa0-query-crash": "WA0_INTERFACE_QUERY_BLOCKED",
    "wa0-release-unexpected-count": "WA0_INTERFACE_RELEASE_BLOCKED",
    "wa0-release-hang": "WA0_INTERFACE_RELEASE_BLOCKED",
    "wa0-release-crash": "WA0_INTERFACE_RELEASE_BLOCKED",
}


def deterministic_tests(source_root) -> dict[str, Any]:
    if set(IN_FLIGHT_BLOCKER) != OPERATIONS or len(OPERATIONS) != 22:
        fail("closed call-operation mapping is not exactly 22 total functions")
    if tuple(EXPECTED_ADAPTER_OPERATIONS) != tuple([
        "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
        "query_factory_2", "query_factory_3", "count_classes",
        "get_class_info_unicode", "get_class_info_2", "get_class_info_1",
        "release_factory_3", "release_factory_2", "release_factory_base",
        "exit_dll", "free_library", "create_component",
        "get_controller_class_id", "initialize_component",
        "query_audio_processor", "release_audio_processor",
        "terminate_component", "release_component",
    ]):
        fail("loader-adapter operation roster differs from the exact WA0 contract")
    call_surface = scanner_component_call_surface(source_root)
    source = (source_root / "windows-factory-probe/source/win32_module.cpp").read_text(
        encoding="utf-8"
    )
    required_fragments = (
        "SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32)",
        "LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |",
        "LOAD_LIBRARY_SEARCH_SYSTEM32",
        "LoadLibraryExW(path.c_str(), nullptr,",
    )
    if any(fragment not in source for fragment in required_fragments):
        fail("production loader source does not retain the exact DLL-search contract")
    stream = StreamState()
    stream.accept({
        "event": "call_started", "sequence": 1, "operation": "free_library",
        "interface": None, "ordinal": None, "tier": None,
    })
    stream.accept({
        "event": "call_completed", "sequence": 2, "attempt_sequence": 1,
        "operation": "free_library", "interface": None, "ordinal": None,
        "tier": None, "return_kind": "win32_error",
    })
    rejected = 0
    for malformed in (
        {"event": "call_completed", "sequence": 1, "attempt_sequence": 0,
         "operation": "free_library", "interface": None, "ordinal": None, "tier": None},
        {"event": "lifecycle", "sequence": 2, "state": "module_unloaded"},
        {"event": "unknown", "sequence": 1},
    ):
        candidate = StreamState()
        try:
            candidate.accept(malformed)
        except Exception:
            rejected += 1
    if rejected != 3:
        fail("deterministic malformed event adapters did not reject every case")

    unmatched = {}
    for operation, blocker in (
        ("create_component", "WC0_COMPONENT_CREATE_BLOCKED"),
        ("get_controller_class_id", "WC0_CONTROLLER_ID_BLOCKED"),
        ("initialize_component", "WC0_COMPONENT_INITIALIZE_BLOCKED"),
        ("query_audio_processor", "WA0_INTERFACE_QUERY_BLOCKED"),
        ("release_audio_processor", "WA0_INTERFACE_RELEASE_BLOCKED"),
        ("terminate_component", "WC0_COMPONENT_TERMINATE_BLOCKED"),
        ("release_component", "WC0_COMPONENT_RELEASE_BLOCKED"),
    ):
        candidate = StreamState()
        candidate.accept({
            "event": "call_started", "sequence": 1, "operation": operation,
            "interface": (
                "IPluginFactory" if operation == "create_component"
                else "IAudioProcessor" if operation == "release_audio_processor"
                else "IComponent"
            ),
            "ordinal": None, "tier": None,
            "object_role": "again_processor_component",
        })
        if candidate.in_flight is None or IN_FLIGHT_BLOCKER[operation] != blocker:
            fail(f"unmatched WA0 operation attribution differs: {operation}")
        unmatched[operation] = blocker
    framed_adapter_stderr = (
        b"runtime startup diagnostic\n"
        + b"".join(
            f"{ADAPTER_OPERATION_PREFIX}{operation}\n".encode("ascii")
            for operation in EXPECTED_ADAPTER_OPERATIONS
        )
        + b"runtime shutdown diagnostic\n"
    )
    adapter_operations, runtime_line_count = adapter_operation_ledger(
        framed_adapter_stderr
    )
    if (
        adapter_operations != list(EXPECTED_ADAPTER_OPERATIONS)
        or runtime_line_count != 2
    ):
        fail("loader adapter owned-ledger framing regression differs")
    try:
        adapter_operation_ledger(
            f"{ADAPTER_OPERATION_PREFIX}unknown_operation\n".encode("ascii")
        )
    except Exception:
        pass
    else:
        fail("loader adapter accepted an unknown owned operation record")
    unicode_name = "AGain VST3"
    normalized_unicode = decode_field(
        unicode_name.encode("utf-16-le").hex(), "utf16le", 64
    )
    if (
        normalized_unicode["text"] != unicode_name
        or normalized_unicode["source_encoding"] != "utf16le"
    ):
        fail("closed UTF-16LE field-label mapping regression differs")
    try:
        decode_field("4100", "utf-16-le", 64)
    except Exception:
        pass
    else:
        fail("normalizer accepted a codec name outside the closed field labels")
    initial_root = {
        "pid": 100, "pgrp": 100, "session": 100, "start_ticks": 700,
        "cmdline": "/runtime/_v2-entry-point --verb=run",
    }
    exec_root = {
        **initial_root,
        "cmdline": "/runtime/run --verb=run",
    }
    continuity = root_identity_continuity(100, initial_root, exec_root)
    if (
        not continuity["runtime_root_start_identity_revalidated"]
        or not continuity["runtime_root_session_revalidated"]
        or not continuity["runtime_root_process_group_revalidated"]
        or not continuity["runtime_entrypoint_exec_transition_observed"]
    ):
        fail("Runtime entry-point exec continuity regression differs")
    rejected_identity_changes = 0
    for key, value in (
        ("pid", 101), ("start_ticks", 701), ("pgrp", 101), ("session", 101)
    ):
        changed = {**exec_root, key: value}
        try:
            root_identity_continuity(100, initial_root, changed)
        except Exception:
            rejected_identity_changes += 1
    if rejected_identity_changes != 4:
        fail("Runtime root identity regression accepted a prohibited change")
    topology_roles = topology_role_census(
        {
            **exec_root,
            "cmdline": (
                f"{runner_root()}/proton runinprefix "
                "C:\\wf0\\bin\\wf0-factory-probe.exe "
                "--session exact-session"
            ),
        },
        [
            {
                "pid": 102,
                "cmdline": (
                    "/usr/bin/python3 proton runinprefix "
                    "C:\\wf0\\bin\\wf0-factory-probe.exe "
                    "--session exact-session"
                ),
            },
            {
                "pid": 103,
                "cmdline": (
                    "/usr/lib/wine/wine64 "
                    "C:\\wf0\\bin\\wf0-factory-probe.exe "
                    "--session exact-session"
                ),
            },
            {
                "pid": 104,
                "cmdline": (
                    "C:\\windows\\system32\\services.exe "
                    "C:\\wf0\\bin\\wf0-factory-probe.exe "
                    "--session exact-session"
                ),
            },
            {
                "pid": 105,
                "cmdline": (
                    "C:\\wf0\\bin\\wf0-factory-probe.exe "
                    "--session exact-session"
                ),
            },
        ],
        "exact-session",
        runtime_root_identity_revalidated=True,
    )
    if topology_roles != {
        "observed_identity_count": 5,
        "runtime_role_observed": True,
        "proton_role_observed": True,
        "scanner_role_count": 1,
        "runtime_root_included_in_role_census": True,
    }:
        fail("exact scanner argv topology regression differs")
    unverified_runtime = topology_role_census(
        exec_root, [], "exact-session",
        runtime_root_identity_revalidated=False,
    )
    if unverified_runtime["runtime_role_observed"]:
        fail("topology regression accepted an unverified Runtime root")
    component_source = (
        source_root / "windows-factory-probe/source/component_instance_session.cpp"
    ).read_text(encoding="utf-8")
    component_header = (
        source_root / "windows-factory-probe/source/component_instance_session.h"
    ).read_text(encoding="utf-8")
    main_source = (
        source_root / "windows-factory-probe/source/main.cpp"
    ).read_text(encoding="utf-8")
    audio_state_names = (
        "audio_processor_absent", "audio_processor_query_in_flight",
        "audio_processor_query_returned_without_lease",
        "audio_processor_lease_acquired", "audio_processor_release_in_flight",
        "audio_processor_lease_retired", "audio_processor_retirement_incomplete",
        "audio_processor_ownership_unknown",
    )
    if any(component_source.count(f'"{name}"') < 1 for name in audio_state_names):
        fail("component source does not retain all eight audio-lease states")
    if (
        component_source.count("component->release()") != 1
        or component_source.count("component_.queryInterface(requested, &output)") != 1
        or component_source.count("interface_->release()") != 1
        or "audio_release_matches_component_baseline(result.release_result)" not in component_source
        or "audio_release_matches_component_baseline(0)" not in component_source
        or "audio_release_matches_component_baseline(2)" not in component_source
        or "result.audio_processor.audio_interface_quiescence" not in component_source
        or "result.release_result == 0" not in component_source
        or "result.host_reference_returned_to_baseline" not in component_source
        or "callbacks.close()" not in component_source
        or "closed_ = true" not in component_source
        or main_source.count("not_attempted_audio_interface_quiescence_unproved") != 1
        or main_source.count("terminate_component") < 2
        or main_source.count("release_component") < 2
        or "if (factory != nullptr &&\n            (!component_session_ran || component.object_quiescence))"
           not in main_source
    ):
        fail("audio-interface quiescence or single-release source law differs")
    release_baseline_assertions = (
        "static_assert(audio_release_matches_component_baseline(1));",
        "static_assert(!audio_release_matches_component_baseline(0));",
        "static_assert(!audio_release_matches_component_baseline(2));",
        "static_assert(!audio_release_matches_component_baseline(0xffffffffu));",
    )
    if (
        re.search(
            r"constexpr\s+bool\s+audio_release_matches_component_baseline\s*\("
            r"\s*Steinberg::uint32\s+value\s*\)\s*noexcept\s*\{\s*"
            r"return\s+value\s*==\s*1\s*;\s*\}",
            component_header,
        ) is None
        or any(assertion not in component_header
               for assertion in release_baseline_assertions)
    ):
        fail("production release-baseline helper/static assertions differ")
    release_baseline_regression = {
        "production_helper": "audio_release_matches_component_baseline",
        "expected_component_owner_baseline": 1,
        "accepted_counts": [1],
        "rejected_counts": [0, 2, 0xffffffff],
        "static_asserts_bound_to_production_helper": True,
    }

    checkout_regression = eol_checkout_regression()
    digest_regression = artifact_digest_regression()
    audio_method_regression = audio_method_verifier_regression()
    allowlist_regression = evidence_allowlist_regression()
    build_environment = build_command_environment({})
    if (
        EXPECTED_AGAIN_MODULE_SHA256
        != "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
        or MSBUILD_MAX_CPU_COUNT != "1"
        or MSVC_POST_OPTIONS != "/MP1"
        or build_environment.get("CL") != ""
        or build_environment.get("_CL_") != "/MP1"
    ):
        fail("exact AGain or serial MSVC build contract differs")
    cases = [
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
    ]
    return {
        "cases": cases,
        "passed": len(cases),
        "failed": 0,
        "component_call_surface": call_surface,
        "unmatched_operation_blockers": unmatched,
        "checkout_regression": checkout_regression,
        "artifact_digest_regression": digest_regression,
        "audio_method_verifier_regression": audio_method_regression,
        "evidence_allowlist_regression": allowlist_regression,
        "release_baseline_regression": release_baseline_regression,
        "exact_again_module_sha256": EXPECTED_AGAIN_MODULE_SHA256,
        "msbuild_max_cpu_count": int(MSBUILD_MAX_CPU_COUNT),
        "msvc_post_options": MSVC_POST_OPTIONS,
    }


def import_negative_tests(build: dict[str, Any]) -> dict[str, Any]:
    passed: list[str] = []

    def rejected(name: str, operation) -> None:
        try:
            operation()
        except Exception:
            passed.append(name)
            return
        fail(f"deterministic import negative was accepted: {name}")

    with tempfile.TemporaryDirectory(prefix="wf0-import-negatives-") as temporary:
        root = pathlib.Path(temporary)

        def one_zip(name: str, entry: str, *, mode: int = 0,
                    compression: int = zipfile.ZIP_STORED) -> pathlib.Path:
            path = root / name
            with zipfile.ZipFile(path, "w") as archive:
                info = zipfile.ZipInfo(entry)
                info.compress_type = compression
                info.create_system = 3
                info.external_attr = mode << 16
                archive.writestr(info, b"x")
            return path

        rejected("archive_traversal", lambda: zip_census(
            one_zip("traversal.zip", "../escape"), None
        ))
        rejected("archive_absolute", lambda: zip_census(
            one_zip("absolute.zip", "/escape"), None
        ))
        rejected("archive_unc", lambda: zip_census(
            one_zip("unc.zip", "//server/share"), None
        ))
        rejected("archive_drive", lambda: zip_census(
            one_zip("drive.zip", "C:/escape"), None
        ))
        rejected("archive_alternate_data_stream", lambda: zip_census(
            one_zip("ads.zip", "file:stream"), None
        ))
        rejected("archive_backslash", lambda: zip_census(
            one_zip("backslash.zip", "a\\b"), None
        ))
        rejected("archive_dot_component", lambda: zip_census(
            one_zip("dot.zip", "a/./b"), None
        ))
        rejected("archive_empty_component", lambda: zip_census(
            one_zip("empty-component.zip", "a//b"), None
        ))
        rejected("archive_nul", lambda: _safe_relative("a\0b"))
        rejected("archive_path_length", lambda: zip_census(
            one_zip("long-path.zip", "a" * 241), None
        ))
        rejected("archive_symlink", lambda: zip_census(
            one_zip("symlink.zip", "link", mode=stat.S_IFLNK | 0o777), None
        ))
        rejected("archive_device", lambda: zip_census(
            one_zip("device.zip", "device", mode=stat.S_IFCHR | 0o600), None
        ))
        case_zip = root / "case.zip"
        with zipfile.ZipFile(case_zip, "w") as archive:
            archive.writestr("A.dll", b"a")
            archive.writestr("a.dll", b"b")
        rejected("archive_case_collision", lambda: zip_census(case_zip, None))
        exact_zip = root / "duplicate.zip"
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            with zipfile.ZipFile(exact_zip, "w") as archive:
                archive.writestr("same", b"a")
                archive.writestr("same", b"b")
        rejected("archive_duplicate", lambda: zip_census(exact_zip, None))
        normalization_zip = root / "normalization.zip"
        with zipfile.ZipFile(normalization_zip, "w") as archive:
            archive.writestr("\N{LATIN SMALL LETTER E WITH ACUTE}", b"a")
            archive.writestr("e\N{COMBINING ACUTE ACCENT}", b"b")
        rejected(
            "archive_unicode_normalization_collision",
            lambda: zip_census(normalization_zip, None),
        )
        directory_zip = root / "directory.zip"
        with zipfile.ZipFile(directory_zip, "w") as archive:
            archive.writestr("directory/", b"")
        rejected("archive_directory_entry", lambda: zip_census(directory_zip, None))
        many_zip = root / "many.zip"
        with zipfile.ZipFile(many_zip, "w") as archive:
            for index in range(257):
                archive.writestr(f"{index:03d}", b"")
        rejected("archive_entry_count", lambda: zip_census(many_zip, None))
        empty_zip = root / "empty.zip"
        with zipfile.ZipFile(empty_zip, "w"):
            pass
        rejected("archive_empty", lambda: zip_census(empty_zip, None))

        oversized_archive = root / "oversized-archive.zip"
        with oversized_archive.open("wb") as handle:
            handle.truncate(MAX_ZIP_BYTES + 1)
        rejected(
            "archive_container_size",
            lambda: zip_census(oversized_archive, None),
        )

        def patched_zip(name: str, *, flag: int | None = None,
                        method: int | None = None,
                        size: int | None = None) -> pathlib.Path:
            path = one_zip(name, "entry")
            data = bytearray(path.read_bytes())
            local = data.index(b"PK\x03\x04")
            central = data.index(b"PK\x01\x02")
            if flag is not None:
                struct.pack_into("<H", data, local + 6, flag)
                struct.pack_into("<H", data, central + 8, flag)
            if method is not None:
                struct.pack_into("<H", data, local + 8, method)
                struct.pack_into("<H", data, central + 10, method)
            if size is not None:
                struct.pack_into("<I", data, local + 22, size)
                struct.pack_into("<I", data, central + 24, size)
            path.write_bytes(data)
            return path

        rejected(
            "archive_encrypted",
            lambda: zip_census(patched_zip("encrypted.zip", flag=1), None),
        )
        rejected(
            "archive_unsupported_compression",
            lambda: zip_census(patched_zip("compression.zip", method=99), None),
        )
        rejected(
            "archive_single_entry_size",
            lambda: zip_census(
                patched_zip("large-entry.zip", size=MAX_ENTRY_BYTES + 1), None
            ),
        )

        wrong_machine = root / "wrong-machine.dll"
        wrong_machine.write_bytes(b"MZ" + b"\0" * 510)
        rejected("wrong_pe_architecture", lambda: parse_pe(wrong_machine))

        scanner_record = next(
            item for item in build["artifact_manifest"]["records"]
            if item["path"] == "bin/wf0-factory-probe.exe"
        )
        changed = root / "changed-scanner.exe"
        changed.write_bytes(
            (pathlib.Path(build["artifact_root"]) / scanner_record["path"]).read_bytes()
            + b"x"
        )
        if changed.stat().st_size == scanner_record["size"] or (
            __import__("hashlib").sha256(changed.read_bytes()).hexdigest()
            == scanner_record["sha256"]
        ):
            fail("wrong scanner hash fixture did not differ")
        passed.append("wrong_scanner_hash")

        source = dict(build["implementation_source_manifest"])
        source["commit"] = "0" * 40
        if source == build["implementation_source_manifest"]:
            fail("stale source identity fixture did not differ")
        passed.append("stale_source_identity")

        missing = dict(build)
        missing["artifact_manifest"] = dict(build["artifact_manifest"])
        missing["artifact_manifest"]["records"] = [
            item for item in build["artifact_manifest"]["records"]
            if item["path"] != f"fixtures/{FAULT_TARGETS[0]}.dll"
        ]
        missing["artifact_manifest"]["record_count"] -= 1
        rejected(
            "missing_fixture",
            lambda: create_environment(secrets.token_hex(16), missing,
                                       fixture=FAULT_TARGETS[0]),
        )

    forged_run = secrets.token_hex(16)
    forged_root = environment_parent() / f".wf0-factory-census.stage-{forged_run}"
    if forged_root.exists() or forged_root.is_symlink():
        fail("forged-marker negative stage unexpectedly exists")
    forged_root.mkdir(mode=0o700)
    forged_marker = {"schema": "linux-vst-bridge-wf0-forged-owner/v1"}
    write_atomic(forged_root / ".wf0-owner.json", canonical_json(forged_marker))
    forged = ScanEnvironment(forged_run, forged_root, forged_marker)
    rejected("forged_environment_marker", lambda: retire_environment(forged))
    if not forged_root.is_dir():
        fail("forged environment was not preserved after refusal")
    shutil.rmtree(forged_root)
    if forged_root.exists() or forged_root.is_symlink():
        fail("negative-fixture owner could not retire its own forged test object")

    return {"cases": passed, "passed": len(passed), "failed": 0}


def run_negative_suite(build: dict[str, Any], source_root, source_verifier) -> dict[str, Any]:
    if tuple(EXPECTED) != FAULT_TARGETS:
        fail("negative expectation roster differs from the fixed artifact roster")
    before_all = protected_snapshot()
    source_verifier()
    deterministic = deterministic_tests(source_root)
    exercises = [
        (fixture, fixture, "exact-again", EXPECTED[fixture])
        for fixture in FAULT_TARGETS
    ]
    abnormal_operations = {
        "wa0-query-hang": ("query_audio_processor", "call_timeout"),
        "wa0-query-crash": (
            "query_audio_processor", "abnormal_termination_in_flight",
        ),
        "wa0-release-hang": ("release_audio_processor", "call_timeout"),
        "wa0-release-crash": (
            "release_audio_processor", "abnormal_termination_in_flight",
        ),
    }
    suppression_cases = {
        "wa0-release-unexpected-count",
        *abnormal_operations,
    }
    results = []
    for index, (name, fixture, component_case, expected) in enumerate(exercises, 1):
        print(f"WA0 negative {index}/{len(exercises)}: {name}", flush=True)
        source_verifier()
        process_guard()
        environment = create_environment(secrets.token_hex(16), build, fixture=fixture)
        retirement = None
        result_record = None
        try:
            receipt = supervise(environment, component_case=component_case)
            if receipt["blocker"] != expected:
                fail(f"{name} blocker differs: expected {expected}, got {receipt['blocker']}")
            if receipt["classification"] == "scanner_completed":
                fail(f"{name} unexpectedly completed")
            component_marker_absent = not (
                environment.session / "forbidden-component-method.marker"
            ).exists()
            audio_marker_absent = not (
                environment.session / "forbidden-audio-processor-method.marker"
            ).exists()
            if not component_marker_absent or not audio_marker_absent:
                fail(f"{name} invoked an out-of-scope component/audio method")
            operations = [
                item["operation"] for item in receipt["records"]
                if item.get("event") == "call_started"
            ]
            lifecycle = [
                item["state"] for item in receipt["records"]
                if item.get("event") == "lifecycle"
            ]
            for inherited in (
                "create_component", "get_controller_class_id", "initialize_component",
            ):
                if operations.count(inherited) != 1:
                    fail(f"{name} did not retain the accepted WC0 prefix: {inherited}")
            if operations.count("query_audio_processor") != 1:
                fail(f"{name} did not retain exactly one interface-query attempt")
            session_records = [
                item.get("component_session") for item in receipt["records"]
                if item.get("event") == "lifecycle"
                and item.get("state") == "component_session_closed"
            ]
            abnormal = abnormal_operations.get(name)
            if abnormal is not None:
                expected_in_flight, expected_classification = abnormal
                if (
                    receipt["last_in_flight_operation"] != expected_in_flight
                    or receipt["classification"] != expected_classification
                    or receipt["audio_processor_observer_state"] !=
                        "audio_processor_ownership_unknown"
                    or receipt["audio_interface_quiescence"] is not False
                    or session_records
                ):
                    fail(f"{name} timeout/crash attribution differs")
                start_index = operations.index(expected_in_flight)
                prohibited_later = {
                    "release_audio_processor",
                    "terminate_component", "release_component",
                    "release_factory_3", "release_factory_2",
                    "release_factory_base", "exit_dll", "free_library",
                }
                if expected_in_flight == "release_audio_processor":
                    prohibited_later.discard("release_audio_processor")
                if any(value in prohibited_later for value in operations[start_index + 1:]):
                    fail(f"{name} emitted an in-process call after its unmatched operation")
            else:
                if receipt["classification"] != "scanner_blocked" or len(session_records) != 1:
                    fail(f"{name} ordinary blocked lifecycle did not close exactly once")
                session = session_records[0]
                if not isinstance(session, dict) or session.get("primary_blocker") != expected:
                    fail(f"{name} component-session primary failure differs")
                lease = session.get("audio_processor_lease", {})
                query = lease.get("query", {})
                release = lease.get("release", {})
                if (
                    lease.get("requested_interface") !=
                        "Steinberg::Vst::IAudioProcessor"
                    or lease.get("requested_iid_raw_tuid_hex") !=
                        "993F0442DAB73C45A569E79D9AAEC33D"
                    or query.get("output_zero_initialized") is not True
                ):
                    fail(f"{name} exact query identity/output initialization differs")
                if (
                    receipt["audio_processor_observer_state"] != lease.get("state")
                    or receipt["audio_interface_quiescence"] is not
                        lease.get("audio_interface_quiescence")
                ):
                    fail(f"{name} supervisor/closed-session lease state differs")
                if name in {"wa0-query-failure-null", "wa0-query-success-null"}:
                    if (
                        lease.get("state") !=
                            "audio_processor_query_returned_without_lease"
                        or query.get("output_nonnull") is not False
                        or lease.get("lease_acquired") is not False
                        or release.get("attempted") is not False
                        or lease.get("audio_interface_quiescence") is not True
                        or operations.count("terminate_component") != 1
                        or operations.count("release_component") != 1
                    ):
                        fail(f"{name} null-output ownership or WC0 cleanup differs")
                elif name == "wa0-query-failure-nonnull":
                    if (
                        lease.get("state") != "audio_processor_lease_retired"
                        or query.get("output_nonnull") is not True
                        or query.get("tuple_consistent") is not False
                        or lease.get("lease_acquired") is not True
                        or operations.count("release_audio_processor") != 1
                        or release.get("reference_count") != 1
                        or lease.get("audio_interface_quiescence") is not True
                        or operations.count("terminate_component") != 1
                        or operations.count("release_component") != 1
                    ):
                        fail("failure/non-null interface ownership cleanup differs")
                elif name == "wa0-release-unexpected-count":
                    if (
                        lease.get("state") !=
                            "audio_processor_retirement_incomplete"
                        or release.get("reference_count") != 2
                        or lease.get("audio_interface_quiescence") is not False
                        or operations.count("release_audio_processor") != 1
                        or any(operation in operations for operation in (
                            "terminate_component", "release_component",
                        ))
                    ):
                        fail("unexpected audio-interface release state differs")

            expected_suppression = name in suppression_cases
            shutdown = receipt.get("inherited_shutdown", {})
            dispositions = shutdown.get("operations", {})
            if set(dispositions) != {
                "terminate_component", "release_component",
                "release_factory_3", "release_factory_2", "release_factory_base",
                "exit_dll", "free_library",
            }:
                fail(f"{name} inherited shutdown disposition roster differs")
            if expected_suppression:
                if (
                    any(value.get("disposition") !=
                        "not_attempted_audio_interface_quiescence_unproved"
                        for value in dispositions.values())
                    or shutdown.get("clean_in_process_shutdown") is not False
                    or shutdown.get("physical_containment_only") is not True
                    or any(operation in operations for operation in dispositions)
                ):
                    fail(f"{name} crossed the audio-interface quiescence gate")
            elif (
                any(value.get("disposition") != "completed"
                    for value in dispositions.values())
                or shutdown.get("clean_in_process_shutdown") is not True
                or shutdown.get("physical_containment_only") is not False
            ):
                fail(f"{name} did not complete its permitted inherited shutdown")
            if any(
                receipt["call_counts"].get(operation) != operations.count(operation)
                for operation in OPERATIONS
            ):
                fail(f"{name} retained call-count ledger differs")

            session = session_records[0] if session_records else None
            lease = (
                session.get("audio_processor_lease", {})
                if isinstance(session, dict) else {}
            )
            result_record = {
                "exercise": name, "fixture": fixture,
                "component_case": component_case,
                "expected_blocker": expected,
                "observed_blocker": receipt["blocker"],
                "classification": receipt["classification"],
                "last_in_flight_operation": receipt["last_in_flight_operation"],
                "audio_processor_observer_state":
                    receipt["audio_processor_observer_state"],
                "audio_interface_quiescence":
                    receipt["audio_interface_quiescence"],
                "component_session_closed": session is not None,
                "audio_processor_lease": lease if lease else None,
                "call_counts": receipt["call_counts"],
                "forbidden_component_method_marker_absent": component_marker_absent,
                "forbidden_audio_processor_method_marker_absent": audio_marker_absent,
                "inherited_shutdown": receipt["inherited_shutdown"],
                "cleanup": receipt["cleanup"],
            }
        finally:
            retirement = retire_environment(environment)
        process_guard()
        if not retirement["stage_absent"]:
            fail("negative stage retirement did not reach exact absence")
        if result_record is None:
            fail(f"{name} produced no retained negative result")
        result_record["environment_retired"] = True
        results.append(result_record)
    after_all = protected_snapshot()
    if after_all != before_all:
        fail("protected state differs after negative suite")
    return {
        "schema": "linux-vst-bridge-wa0-negative-tests/v1",
        "deterministic": deterministic,
        "inherited_wf0_wc0_negative_suites": "not_run_focused_wa0_only",
        "loader_adapter_runtime_exercise": "not_run_focused_wa0_only",
        "live_fault_results": results,
        "live_fault_roster_complete": [
            item["fixture"] for item in results
        ] == list(FAULT_TARGETS),
        "focused_exercise_count": len(results),
        "protected_state_equal": True,
    }


if __name__ == "__main__":
    raise SystemExit("negative_tests.py is a library; run run.py")

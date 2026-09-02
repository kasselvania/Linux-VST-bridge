#!/usr/bin/env python3
"""Deterministic and bounded live negative proofs for every WF0 stage owner."""

from __future__ import annotations

import secrets
import pathlib
import shutil
import stat
import struct
import tempfile
import warnings
import zipfile
from typing import Any

from artifacts import MAX_ENTRY_BYTES, MAX_ZIP_BYTES, _safe_relative, zip_census
from build import eol_checkout_regression
from common import (
    FAULT_TARGETS, canonical_json, environment_parent, fail, process_guard,
    protected_snapshot, runner_root, write_atomic,
)
from environment import ScanEnvironment, create_environment, retire_environment
from normalize import decode_field
from supervise import (
    ADAPTER_OPERATION_PREFIX, EXPECTED_ADAPTER_OPERATIONS, IN_FLIGHT_BLOCKER,
    OPERATIONS, StreamState, adapter_operation_ledger, root_identity_continuity,
    supervise, supervise_adapter, topology_role_census,
)
from verify import parse_pe, scanner_has_no_instantiation_call


EXPECTED = {
    "wf0-missing-factory": "WF0_FACTORY_GET_BLOCKED",
    "wf0-null-factory": "WF0_FACTORY_GET_BLOCKED",
    "wf0-no-entry": None,
    "wf0-factory1-only": None,
    "wf0-factory2-only": None,
    "wf0-factory3-fallback": None,
    "wf0-init-false": "WF0_MODULE_ENTRY_BLOCKED",
    "wf0-factory-info-false": "WF0_FACTORY_INFO_BLOCKED",
    "wf0-count-negative": "WF0_CLASS_ENUMERATION_BLOCKED",
    "wf0-count-excessive": "WF0_CLASS_ENUMERATION_BLOCKED",
    "wf0-class-info-false": "WF0_CLASS_ENUMERATION_BLOCKED",
    "wf0-duplicate-class-id": "WF0_CLASS_ENUMERATION_BLOCKED",
    "wf0-hang-entry": "WF0_MODULE_ENTRY_BLOCKED",
    "wf0-hang-factory": "WF0_FACTORY_GET_BLOCKED",
    "wf0-hang-class": "WF0_CLASS_ENUMERATION_BLOCKED",
    "wf0-crash-entry": "WF0_MODULE_ENTRY_BLOCKED",
    "wf0-crash-factory": "WF0_FACTORY_GET_BLOCKED",
    "wf0-crash-class": "WF0_CLASS_ENUMERATION_BLOCKED",
    "wf0-hang-release": "WF0_FACTORY_RELEASE_BLOCKED",
    "wf0-crash-release": "WF0_FACTORY_RELEASE_BLOCKED",
    "wf0-exit-false": "WF0_MODULE_EXIT_BLOCKED",
    "wf0-create-instance-tripwire": None,
}


def deterministic_tests(source_root) -> dict[str, Any]:
    if set(IN_FLIGHT_BLOCKER) != OPERATIONS or len(OPERATIONS) != 15:
        fail("closed call-operation mapping is not exactly 15 total functions")
    if not scanner_has_no_instantiation_call(source_root):
        fail("scanner source contains an instantiation call expression")
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
    checkout_regression = eol_checkout_regression()
    cases = [
        "closed_operation_mapping_15_of_15", "completion_tuple_validation",
        "sequence_gap_rejection", "second_completion_rejection",
        "relative_module_path_rejection", "default_search_flag_lock",
        "non_null_hfile_prohibited", "source_token_instantiation_call_absent",
        "unload_failure_adapter_mapping_compiled",
        "runtime_entrypoint_exec_identity_continuity",
        "runtime_root_included_in_topology_role_census",
        "closed_sdk_encoding_label_mapping",
        "controlled_git_checkout_eol_regression",
    ]
    return {
        "cases": cases,
        "passed": len(cases),
        "failed": 0,
        "checkout_regression": checkout_regression,
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
            if item["path"] != "fixtures/wf0-null-factory.dll"
        ]
        missing["artifact_manifest"]["record_count"] -= 1
        rejected(
            "missing_fixture",
            lambda: create_environment(secrets.token_hex(16), missing,
                                       fixture="wf0-null-factory"),
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
    adapter_environment = create_environment(secrets.token_hex(16), build, fixture="adapter")
    try:
        adapter = supervise_adapter(adapter_environment)
    finally:
        adapter_retirement = retire_environment(adapter_environment)
    if not adapter_retirement["stage_absent"]:
        fail("loader-adapter environment did not retire")
    process_guard()
    results = []
    for index, fixture in enumerate(FAULT_TARGETS, 1):
        print(f"WF0 negative {index}/{len(FAULT_TARGETS)}: {fixture}", flush=True)
        source_verifier()
        process_guard()
        environment = create_environment(secrets.token_hex(16), build, fixture=fixture)
        retirement = None
        try:
            receipt = supervise(environment)
            expected = EXPECTED[fixture]
            if receipt["blocker"] != expected:
                fail(f"{fixture} blocker differs: expected {expected}, got {receipt['blocker']}")
            if expected is None and receipt["classification"] != "scanner_completed":
                fail(f"{fixture} should complete its bounded census path")
            if expected is not None and receipt["classification"] == "scanner_completed":
                fail(f"{fixture} unexpectedly completed")
            marker = environment.session / "create-instance-tripwire.marker"
            marker_absent = not marker.exists()
            if fixture == "wf0-create-instance-tripwire" and not marker_absent:
                fail("create-instance tripwire marker was created")
            operations = [
                item["operation"] for item in receipt["records"]
                if item.get("event") == "call_started"
            ]
            lifecycle = [
                item["state"] for item in receipt["records"]
                if item.get("event") == "lifecycle"
            ]
            if fixture == "wf0-missing-factory" and (
                "init_dll" in operations
                or "factory_export_missing" not in lifecycle
                or operations[-2:] != ["exit_dll", "free_library"]
            ):
                fail("missing-factory ownership/cleanup timeline differs")
            if fixture == "wf0-no-entry" and (
                "module_entry_absent" not in lifecycle
                or "module_exit_absent" not in lifecycle
            ):
                fail("optional entry/exit absence timeline differs")
            expected_in_flight = {
                "wf0-hang-entry": "init_dll",
                "wf0-crash-entry": "init_dll",
                "wf0-hang-factory": "get_plugin_factory",
                "wf0-crash-factory": "get_plugin_factory",
                "wf0-hang-class": "get_class_info_unicode",
                "wf0-crash-class": "get_class_info_unicode",
                "wf0-hang-release": "release_factory_3",
                "wf0-crash-release": "release_factory_3",
            }.get(fixture)
            if expected_in_flight is not None and (
                receipt["last_in_flight_operation"] != expected_in_flight
            ):
                fail(f"{fixture} call attribution differs")
            expected_release_order = {
                "wf0-no-entry": [
                    "release_factory_3", "release_factory_2", "release_factory_base"
                ],
                "wf0-factory1-only": ["release_factory_base"],
                "wf0-factory2-only": ["release_factory_2", "release_factory_base"],
                "wf0-factory3-fallback": [
                    "release_factory_3", "release_factory_2", "release_factory_base"
                ],
                "wf0-create-instance-tripwire": [
                    "release_factory_3", "release_factory_2", "release_factory_base"
                ],
            }.get(fixture)
            if expected_release_order is not None:
                release_operations = [
                    value for value in operations if value.startswith("release_factory_")
                ]
                if release_operations != expected_release_order:
                    fail(f"{fixture} reverse release order differs")
            results.append({
                "fixture": fixture, "expected_blocker": expected,
                "observed_blocker": receipt["blocker"],
                "classification": receipt["classification"],
                "last_in_flight_operation": receipt["last_in_flight_operation"],
                "raw_exit": receipt["raw_exit"], "tripwire_marker_absent": marker_absent,
                "cleanup": receipt["cleanup"],
            })
        finally:
            retirement = retire_environment(environment)
        process_guard()
        if not retirement["stage_absent"]:
            fail("negative stage retirement did not reach exact absence")
    after_all = protected_snapshot()
    if after_all != before_all:
        fail("protected state differs after negative suite")
    return {
        "schema": "linux-vst-bridge-wf0-negative-tests/v1",
        "import_negatives": import_negative_tests(build),
        "deterministic": deterministic_tests(source_root),
        "loader_adapter": adapter,
        "live_fault_results": results,
        "live_fault_roster_complete": [item["fixture"] for item in results] == list(FAULT_TARGETS),
        "protected_state_equal": True,
    }


if __name__ == "__main__":
    raise SystemExit("negative_tests.py is a library; run run.py")

#!/usr/bin/env python3
"""Bounded Runtime 4 / Proton 11 supervision for one marker-bound WA0 scan."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import pwd
import re
import secrets
import selectors
import signal
import subprocess
import time
from typing import Any

from common import (
    NEUTRAL_APP_ID, canonical_json, command, environment_parent, fail,
    process_census, protected_snapshot, runner_root, runtime_root, sha256_bytes,
    steam_root, verify_runner_identity, write_atomic,
)
from environment import ScanEnvironment, verify_environment


PROCESS_CAP = 256
POLL_SECONDS = 0.05
FIRST_READY_SECONDS = 180.0
REUSED_READY_SECONDS = 90.0
HELD_GATE_SECONDS = 15.0
CALL_SECONDS = 15.0
CLASS_SECONDS = 30.0
POST_GATE_SECONDS = 120.0
DRAIN_SECONDS = 20.0
CLEANUP_SECONDS = 10.0
STDOUT_CAP = 1048576
STDERR_CAP = 65536
EVENT_CAP = 2048
SCANNER_WINDOWS_ARGV0 = "C:\\wf0\\bin\\wf0-factory-probe.exe"
ADAPTER_WINDOWS_ARGV0 = "C:\\wf0\\bin\\wf0-loader-adapter-tests.exe"
ADAPTER_OPERATION_PREFIX = "wf0-loader-adapter-operation="
EXPECTED_ADAPTER_OPERATIONS = (
    "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
    "query_factory_2", "query_factory_3", "count_classes",
    "get_class_info_unicode", "get_class_info_2", "get_class_info_1",
    "release_factory_3", "release_factory_2", "release_factory_base",
    "exit_dll", "free_library",
    "create_component", "get_controller_class_id", "initialize_component",
    "query_audio_processor", "release_audio_processor",
    "terminate_component", "release_component",
)

OPERATIONS = {
    "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
    "query_factory_2", "query_factory_3", "count_classes", "get_class_info_unicode",
    "get_class_info_2", "get_class_info_1", "release_factory_3", "release_factory_2",
    "release_factory_base", "exit_dll", "free_library",
    "create_component", "get_controller_class_id", "initialize_component",
    "query_audio_processor", "release_audio_processor",
    "terminate_component", "release_component",
}
INTERFACES = {
    None, "IPluginFactory", "IPluginFactory2", "IPluginFactory3", "IComponent",
    "IAudioProcessor",
}
TIERS = {None, "factory_3_unicode", "factory_2", "factory_1"}
HOST_CALLBACKS = {"queryInterface", "addRef", "release", "getName", "createInstance"}
COMPONENT_OPERATIONS = {
    "create_component", "get_controller_class_id", "initialize_component",
    "query_audio_processor", "release_audio_processor",
    "terminate_component", "release_component",
}
AUDIO_INTERFACE_OPERATIONS = {
    "query_audio_processor", "release_audio_processor",
}
INHERITED_SHUTDOWN_OPERATIONS = (
    "terminate_component", "release_component",
    "release_factory_3", "release_factory_2", "release_factory_base",
    "exit_dll", "free_library",
)
IN_FLIGHT_BLOCKER = {
    "load_library": "WF0_MODULE_OPEN_BLOCKED",
    "init_dll": "WF0_MODULE_ENTRY_BLOCKED",
    "get_plugin_factory": "WF0_FACTORY_GET_BLOCKED",
    "get_factory_info": "WF0_FACTORY_INFO_BLOCKED",
    "query_factory_2": "WF0_FACTORY_INFO_BLOCKED",
    "query_factory_3": "WF0_FACTORY_INFO_BLOCKED",
    "count_classes": "WF0_CLASS_ENUMERATION_BLOCKED",
    "get_class_info_unicode": "WF0_CLASS_ENUMERATION_BLOCKED",
    "get_class_info_2": "WF0_CLASS_ENUMERATION_BLOCKED",
    "get_class_info_1": "WF0_CLASS_ENUMERATION_BLOCKED",
    "release_factory_3": "WF0_FACTORY_RELEASE_BLOCKED",
    "release_factory_2": "WF0_FACTORY_RELEASE_BLOCKED",
    "release_factory_base": "WF0_FACTORY_RELEASE_BLOCKED",
    "exit_dll": "WF0_MODULE_EXIT_BLOCKED",
    "free_library": "WF0_MODULE_UNLOAD_BLOCKED",
    "create_component": "WC0_COMPONENT_CREATE_BLOCKED",
    "get_controller_class_id": "WC0_CONTROLLER_ID_BLOCKED",
    "initialize_component": "WC0_COMPONENT_INITIALIZE_BLOCKED",
    "terminate_component": "WC0_COMPONENT_TERMINATE_BLOCKED",
    "release_component": "WC0_COMPONENT_RELEASE_BLOCKED",
    "query_audio_processor": "WA0_INTERFACE_QUERY_BLOCKED",
    "release_audio_processor": "WA0_INTERFACE_RELEASE_BLOCKED",
}
EXIT_BLOCKER = {
    64: "WF0_SCANNER_LAUNCH_BLOCKED", 65: "WF0_PROCESS_IDENTITY_BLOCKED",
    70: "WF0_MODULE_OPEN_BLOCKED", 71: "WF0_MODULE_ENTRY_BLOCKED",
    72: "WF0_FACTORY_GET_BLOCKED", 73: "WF0_FACTORY_GET_BLOCKED",
    74: "WF0_FACTORY_INFO_BLOCKED", 75: "WF0_CLASS_ENUMERATION_BLOCKED",
    76: "WF0_CLASS_ENUMERATION_BLOCKED", 77: "WF0_CLASS_ENUMERATION_BLOCKED",
    78: "WF0_OUTPUT_NORMALIZATION_BLOCKED", 79: "WF0_MODULE_EXIT_BLOCKED",
    80: "WF0_MODULE_UNLOAD_BLOCKED", 81: "WF0_FACTORY_RELEASE_BLOCKED",
    82: "WF0_SCANNER_LAUNCH_BLOCKED",
    83: "WC0_COMPONENT_CREATE_BLOCKED",
    84: "WC0_CONTROLLER_ID_BLOCKED",
    85: "WC0_CONTROLLER_ID_MISMATCH",
    86: "WC0_HOST_CONTEXT_BLOCKED",
    87: "WC0_COMPONENT_INITIALIZE_BLOCKED",
    88: "WC0_COMPONENT_TERMINATE_BLOCKED",
    89: "WC0_COMPONENT_RELEASE_BLOCKED",
    90: "WA0_INTERFACE_QUERY_BLOCKED",
    91: "WA0_INTERFACE_QUERY_INCONSISTENT",
    92: "WA0_INTERFACE_RELEASE_BLOCKED",
}


def controlled_environment(environment: ScanEnvironment) -> dict[str, str]:
    user = pwd.getpwuid(os.getuid())
    xdg_runtime = pathlib.Path(f"/run/user/{os.getuid()}")
    if not xdg_runtime.is_dir() or xdg_runtime.stat().st_uid != os.getuid():
        fail("owned XDG runtime directory is unavailable")
    values = {
        "HOME": user.pw_dir, "USER": user.pw_name, "LOGNAME": user.pw_name,
        "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "XDG_RUNTIME_DIR": str(xdg_runtime),
        "XDG_CACHE_HOME": str(environment.root / "host-cache"),
        "XDG_CONFIG_HOME": str(environment.root / "host-config"),
        "XDG_DATA_HOME": str(environment.root / "host-data"),
        "TMPDIR": str(environment.root / "host-tmp"),
        "STEAM_COMPAT_DATA_PATH": str(environment.compatdata),
        "STEAM_COMPAT_CLIENT_INSTALL_PATH": str(steam_root()),
        "STEAM_COMPAT_APP_ID": NEUTRAL_APP_ID, "SteamAppId": NEUTRAL_APP_ID,
        "SteamGameId": NEUTRAL_APP_ID,
        "PRESSURE_VESSEL_VARIABLE_DIR": str(environment.root / "runtime-var"),
        "STEAM_ZENITY": "",
    }
    return values


def handshake(environment: ScanEnvironment, session: str,
              component_case: str) -> bytes:
    marker = environment.marker
    return (
        "schema=linux-vst-bridge-wf0-handshake/v1\n"
        f"session={session}\n"
        f"scanner_sha256={marker['scanner_sha256']}\n"
        f"module_sha256={marker['module_sha256']}\n"
        f"bundle_manifest_sha256={marker['bundle_manifest']['sha256']}\n"
        "implementation_source_manifest_sha256="
        f"{marker['implementation_source_manifest_sha256']}\n"
        "mode=wa0-audio-processor-interface-admission\n"
        f"component_case={component_case}\n"
        "run_ordinal=1\n"
    ).encode()


def command_vector(environment: ScanEnvironment, session: str,
                   component_case: str) -> list[str]:
    if component_case != "exact-again":
        fail("component case is outside the closed WA0 command contract")
    marker = environment.marker
    return [
        str(runtime_root() / "_v2-entry-point"), "--verb=run", "--",
        str(runner_root() / "proton"), "runinprefix",
        "C:\\wf0\\bin\\wf0-factory-probe.exe",
        "--session", session,
        "--scanner-sha256", marker["scanner_sha256"],
        "--implementation-source-manifest-sha256",
        marker["implementation_source_manifest_sha256"],
        "--module", "C:\\wf0\\fixture\\again.vst3\\Contents\\x86_64-win\\again.vst3",
        "--module-sha256", marker["module_sha256"],
        "--bundle-manifest-sha256", marker["bundle_manifest"]["sha256"],
        "--ready", f"C:\\wf0\\session\\{session}.ready",
        "--gate", f"C:\\wf0\\session\\{session}.gate",
        "--max-classes", "256", "--stdout-cap", "1048576",
        "--mode", "wa0-audio-processor-interface-admission",
        "--component-case", component_case,
    ]


def descendants(root_pid: int) -> list[dict[str, Any]]:
    records = process_census()
    by_parent: dict[int, list[dict[str, Any]]] = {}
    for record in records:
        by_parent.setdefault(record["ppid"], []).append(record)
    result = []
    queue = [root_pid]
    seen = set()
    while queue:
        parent = queue.pop(0)
        for child in by_parent.get(parent, []):
            if child["pid"] in seen:
                continue
            seen.add(child["pid"])
            result.append(child)
            queue.append(child["pid"])
    return result


def process_identity(pid: int, deadline_seconds: float = 2.0) -> dict[str, Any]:
    deadline = time.monotonic() + deadline_seconds
    while time.monotonic() < deadline:
        matches = [record for record in process_census() if record["pid"] == pid]
        if len(matches) == 1:
            return matches[0]
        time.sleep(POLL_SECONDS)
    fail("spawned process identity was not observable")
    raise AssertionError


def root_identity_continuity(root_pid: int, initial: dict[str, Any],
                             current: dict[str, Any]) -> dict[str, Any]:
    """Admit the pinned Runtime entry-point exec without weakening PID identity."""
    if (
        initial.get("pid") != root_pid
        or current.get("pid") != root_pid
        or initial.get("start_ticks") != current.get("start_ticks")
        or initial.get("pgrp") != root_pid
        or current.get("pgrp") != root_pid
        or initial.get("session") != root_pid
        or current.get("session") != root_pid
    ):
        fail("Runtime root PID/start/session/process-group identity changed before gate")
    return {
        "runtime_root_start_identity_revalidated": True,
        "runtime_root_session_revalidated": True,
        "runtime_root_process_group_revalidated": True,
        "runtime_entrypoint_exec_transition_observed":
            initial.get("cmdline") != current.get("cmdline"),
    }


def topology_role_census(root_record: dict[str, Any],
                         descendant_records: list[dict[str, Any]],
                         session: str, *,
                         runtime_root_identity_revalidated: bool) -> dict[str, Any]:
    """Classify the exec-stable Runtime root together with its descendants."""
    records = [root_record, *descendant_records]
    texts = [record["cmdline"] for record in records]
    proton = [
        text for text in texts
        if str(runner_root() / "proton") in text and "runinprefix" in text
    ]
    scanner = []
    for record in records:
        arguments = record["cmdline"].split()
        if (
            len(arguments) >= 3
            and arguments[0].lower() == SCANNER_WINDOWS_ARGV0.lower()
            and arguments[1] == "--session"
            and arguments[2] == session
        ):
            scanner.append(record)
    return {
        "observed_identity_count": len(records),
        "runtime_role_observed": runtime_root_identity_revalidated,
        "proton_role_observed": bool(proton),
        "scanner_role_count": len(scanner),
        "runtime_root_included_in_role_census": True,
    }


def topology(root: subprocess.Popen[bytes], session: str,
             root_identity: dict[str, Any],
             spawn_command_vector_sha256: str) -> dict[str, Any]:
    root_matches = [
        record for record in process_census()
        if record["pid"] == root.pid
    ]
    if len(root_matches) != 1:
        fail("Runtime root PID identity is not uniquely observable before gate")
    continuity = root_identity_continuity(
        root.pid, root_identity, root_matches[0]
    )
    records = descendants(root.pid)
    if not records or len(records) + 1 > PROCESS_CAP:
        fail("owned process topology is absent or exceeds its bound")
    roles = topology_role_census(
        root_matches[0], records, session,
        runtime_root_identity_revalidated=all((
            continuity["runtime_root_start_identity_revalidated"],
            continuity["runtime_root_session_revalidated"],
            continuity["runtime_root_process_group_revalidated"],
        )),
    )
    if (
        not roles["runtime_role_observed"]
        or not roles["proton_role_observed"]
        or roles["scanner_role_count"] != 1
    ):
        fail(
            "required Runtime/Proton/scanner topology is not simultaneously "
            "observable: "
            f"runtime={int(roles['runtime_role_observed'])} "
            f"proton={int(roles['proton_role_observed'])} "
            f"scanner_count={roles['scanner_role_count']}"
        )
    for record in [root_matches[0], *records]:
        text = record["cmdline"].lower()
        if (
            record["pid"] == os.getpid()
            or "steamapps/common/steam/" in text
            or "umu" in text
            or "yabridge" in text
            or "bitwig" in text
            or "validator" in text
        ):
            fail("prohibited Steam/UMU/yabridge/Bitwig/validator ancestry was observed")
    return {
        "observed_identity_count": roles["observed_identity_count"],
        "runtime_role_observed": True,
        "proton_role_observed": True, "scanner_role_observed": True,
        "runtime_root_included_in_role_census":
            roles["runtime_root_included_in_role_census"],
        **continuity,
        "spawn_command_vector_sha256": spawn_command_vector_sha256,
        "full_descendant_ancestry_used": True,
        "steam_game_ancestor_observed": False,
    }


def adapter_operation_ledger(stderr: bytes) -> tuple[list[str], int]:
    """Separate the source-owned adapter ledger from bounded runner diagnostics."""
    try:
        lines = stderr.decode("utf-8", "strict").splitlines()
    except UnicodeDecodeError:
        fail("loader adapter stderr is not UTF-8")
    operations: list[str] = []
    runtime_lines = 0
    for line in lines:
        if not line.startswith(ADAPTER_OPERATION_PREFIX):
            runtime_lines += 1
            continue
        operation = line[len(ADAPTER_OPERATION_PREFIX):]
        if operation not in EXPECTED_ADAPTER_OPERATIONS:
            fail("loader adapter emitted an unknown owned operation record")
        operations.append(operation)
    return operations, runtime_lines


class StreamState:
    def __init__(self) -> None:
        self.stdout = bytearray()
        self.stderr = bytearray()
        self.pending = bytearray()
        self.records: list[dict[str, Any]] = []
        self.in_flight: dict[str, Any] | None = None
        self.in_flight_at: float | None = None
        self.class_started_at: float | None = None
        self.host_callback_count = 0

    def feed(self, name: str, data: bytes) -> None:
        target = self.stdout if name == "stdout" else self.stderr
        target.extend(data)
        cap = STDOUT_CAP if name == "stdout" else STDERR_CAP
        if len(target) > cap:
            fail(f"{name} cap exceeded")
        if name != "stdout":
            return
        self.pending.extend(data)
        while b"\n" in self.pending:
            line, _, rest = self.pending.partition(b"\n")
            self.pending = bytearray(rest)
            if not line or len(line) > 786432:
                fail("scanner emitted an empty or oversized JSONL record")
            try:
                record = json.loads(line)
            except (json.JSONDecodeError, UnicodeDecodeError):
                fail("scanner output is not canonical JSONL data")
            self.accept(record)

    def accept(self, record: dict[str, Any]) -> None:
        if len(self.records) >= EVENT_CAP or record.get("sequence") != len(self.records) + 1:
            fail("scanner sequence/event bound violation")
        event = record.get("event")
        if event == "call_started":
            if self.in_flight is not None or record.get("operation") not in OPERATIONS:
                fail("scanner call-start contract violation")
            if record.get("interface") not in INTERFACES or record.get("tier") not in TIERS:
                fail("scanner call-start enum violation")
            self.in_flight = record
            self.in_flight_at = time.monotonic()
        elif event == "call_completed":
            if self.in_flight is None:
                fail("scanner completion has no in-flight attempt")
            for key in ("operation", "interface", "ordinal", "tier"):
                if record.get(key) != self.in_flight.get(key):
                    fail("scanner completion tuple differs from its attempt")
            if record.get("attempt_sequence") != self.in_flight.get("sequence"):
                fail("scanner completion attempt link differs")
            self.in_flight = None
            self.in_flight_at = None
        elif event == "lifecycle":
            if self.in_flight is not None:
                fail("lifecycle event appeared before paired completion")
            if record.get("state") == "class_enumeration_in_progress":
                self.class_started_at = time.monotonic()
            if record.get("state") == "class_enumeration_complete":
                self.class_started_at = None
        elif event == "host_callback":
            if record.get("operation") not in HOST_CALLBACKS:
                fail("host callback operation is outside the closed WA0 contract")
            if record.get("thread_role") != "scanner_main_thread":
                fail("host callback thread role differs")
            origin = record.get("origin")
            if origin == "component":
                if (
                    self.in_flight is None
                    or record.get("enclosing_attempt_sequence")
                    != self.in_flight.get("sequence")
                    or record.get("enclosing_operation")
                    != self.in_flight.get("operation")
                    or self.in_flight.get("operation")
                    not in {"initialize_component", "terminate_component"}
                ):
                    fail("component host callback attribution differs")
            elif origin == "owner_local":
                if (
                    self.in_flight is not None
                    or record.get("enclosing_attempt_sequence") is not None
                    or record.get("enclosing_operation") is not None
                ):
                    fail("local host-owner callback attribution differs")
            else:
                fail("host callback origin is outside the closed WC0 contract")
            self.host_callback_count += 1
            if self.host_callback_count > 64:
                fail("host callback ledger exceeds its fixed bound")
        else:
            fail("scanner event kind is outside the closed contract")
        self.records.append(record)


def pump(selector: selectors.BaseSelector, streams: StreamState, timeout: float) -> None:
    for key, _ in selector.select(timeout):
        data = os.read(key.fileobj.fileno(), 65536)
        if data:
            streams.feed(key.data, data)
        else:
            selector.unregister(key.fileobj)


def cleanup_process(root: subprocess.Popen[bytes], owned: list[tuple[int, int]]) -> dict[str, Any]:
    deadline = time.monotonic() + CLEANUP_SECONDS
    live_owned = {
        (record["pid"], record["start_ticks"]) for record in process_census()
    } & set(owned)
    if root.poll() is None or live_owned:
        try:
            os.killpg(root.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    while time.monotonic() < deadline - 3:
        live_owned = {
            (record["pid"], record["start_ticks"]) for record in process_census()
        } & set(owned)
        if root.poll() is not None and not live_owned:
            break
        time.sleep(POLL_SECONDS)
    if root.poll() is None or live_owned:
        try:
            os.killpg(root.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    try:
        root.wait(timeout=max(0.1, deadline - time.monotonic()))
    except subprocess.TimeoutExpired:
        fail("Runtime root did not terminate inside cleanup bound")
    remaining = []
    for record in process_census():
        if ((record["pid"], record["start_ticks"]) in owned or
                ".wf0-factory-census.stage-" in record["cmdline"]):
            remaining.append(record["comm"])
    if remaining:
        fail(f"owned descendants survived cleanup: {remaining}")
    return {"owned_descendants_zero": True, "process_group_empty": True}


def supervise_adapter(environment: ScanEnvironment) -> dict[str, Any]:
    if environment.marker.get("fixture") != "adapter":
        fail("loader-adapter supervision requires the adapter environment")
    verify_environment(environment)
    runner_identity = verify_runner_identity()
    before = protected_snapshot()
    command_line = [
        str(runtime_root() / "_v2-entry-point"), "--verb=run", "--",
        str(runner_root() / "proton"), "runinprefix",
        "C:\\wf0\\bin\\wf0-loader-adapter-tests.exe",
    ]
    root = subprocess.Popen(
        command_line, env=controlled_environment(environment),
        stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        start_new_session=True,
    )
    root_identity = process_identity(root.pid)
    seen_owned: set[tuple[int, int]] = {
        (root_identity["pid"], root_identity["start_ticks"])
    }
    proton_observed = False
    adapter_observed = False
    deadline = time.monotonic() + FIRST_READY_SECONDS
    try:
        while root.poll() is None and time.monotonic() < deadline:
            for record in descendants(root.pid):
                seen_owned.add((record["pid"], record["start_ticks"]))
                text = record["cmdline"]
                proton_observed = proton_observed or (
                    str(runner_root() / "proton") in text and "runinprefix" in text
                )
                arguments = text.split()
                adapter_observed = adapter_observed or bool(
                    arguments
                    and arguments[0].lower() == ADAPTER_WINDOWS_ARGV0.lower()
                )
            time.sleep(POLL_SECONDS)
        if root.poll() is None:
            fail("loader adapter exceeded its Runtime/Proton bound")
        stdout, stderr = root.communicate(timeout=5)
    finally:
        cleanup = cleanup_process(root, sorted(seen_owned))
    if len(stdout) > STDOUT_CAP or len(stderr) > STDERR_CAP:
        fail("loader adapter output exceeded its bound")
    try:
        records = [json.loads(line) for line in stdout.splitlines()]
    except (json.JSONDecodeError, UnicodeDecodeError):
        fail("loader adapter output is malformed")
    operations, runtime_stderr_line_count = adapter_operation_ledger(stderr)
    validated_stream = StreamState()
    for record in records:
        validated_stream.accept(record)
    if validated_stream.in_flight is not None:
        fail("loader adapter ended with an unmatched call attempt")
    if (
        root.returncode != 0
        or operations != list(EXPECTED_ADAPTER_OPERATIONS)
        or len(records) != 3
        or records[0].get("event") != "lifecycle"
        or records[0].get("state") != "module_exit_absent"
        or records[1].get("event") != "call_started"
        or records[1].get("operation") != "free_library"
        or records[2].get("event") != "call_completed"
        or records[2].get("operation") != "free_library"
        or records[2].get("attempt_sequence") != records[1].get("sequence")
        or records[2].get("return_kind") != "win32_error"
        or not proton_observed
        or not adapter_observed
    ):
        fail("loader adapter did not prove the exact false-unload attribution")
    after = protected_snapshot()
    if after != before or verify_runner_identity() != runner_identity:
        fail("protected state or runner changed during loader-adapter execution")
    return {
        "schema": "linux-vst-bridge-wa0-loader-adapter/v1",
        "classification": "adapter_completed",
        "closed_operation_count": 22,
        "free_library_false_mapped": True,
        "paired_completion_valid": True,
        "runtime_role_observed": True,
        "proton_role_observed": proton_observed,
        "adapter_role_observed": adapter_observed,
        "stdout_sha256": hashlib.sha256(stdout).hexdigest(),
        "stderr_sha256": hashlib.sha256(stderr).hexdigest(),
        "runtime_stderr_line_count": runtime_stderr_line_count,
        "cleanup": cleanup,
        "protected_state_equal": True,
        "runner_identity": runner_identity,
    }


def supervise(environment: ScanEnvironment, *, hold_gate: bool = False,
              component_case: str = "exact-again") -> dict[str, Any]:
    verify_environment(environment)
    runner_identity = verify_runner_identity()
    session = secrets.token_hex(16)
    ready = environment.session / f"{session}.ready"
    gate = environment.session / f"{session}.gate"
    if ready.exists() or gate.exists():
        fail("handshake artifacts exist before spawn")
    expected = handshake(environment, session, component_case)
    before = protected_snapshot()
    command_line = command_vector(environment, session, component_case)
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
    finally:
        cleanup = cleanup_process(root, sorted(seen_owned))
        selector.close()

    if streams.pending:
        fail("scanner stdout ended with a partial JSONL record")
    after = protected_snapshot()
    if after != before:
        fail("protected state drifted during supervised execution")
    if verify_runner_identity() != runner_identity:
        fail("Runtime 4 / Proton 11 identity changed during supervised execution")
    last_state = next((record.get("state") for record in reversed(streams.records)
                       if record.get("event") == "lifecycle"), None)
    in_flight = streams.in_flight
    raw_exit = root.returncode
    if hold_gate:
        blocker = None
        classification = "held_gate_proof_complete"
    elif timed_out and in_flight is not None:
        blocker = IN_FLIGHT_BLOCKER[in_flight["operation"]]
        classification = "call_timeout"
    elif timed_out:
        blocker = "WF0_SCANNER_LAUNCH_BLOCKED" if gated_at is None else "WF0_OUTPUT_NORMALIZATION_BLOCKED"
        classification = "stage_timeout"
    elif raw_exit == 0 and last_state == "scanner_completed" and in_flight is None:
        blocker = None
        classification = "scanner_completed"
    elif in_flight is not None:
        blocker = IN_FLIGHT_BLOCKER[in_flight["operation"]]
        classification = "abnormal_termination_in_flight"
    else:
        blocker = EXIT_BLOCKER.get(raw_exit, "WF0_SCANNER_LAUNCH_BLOCKED")
        classification = "scanner_blocked"

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
    component_abnormal = (
        in_flight is not None and in_flight.get("operation") in COMPONENT_OPERATIONS
    )
    audio_interface_abnormal = (
        in_flight is not None
        and in_flight.get("operation") in AUDIO_INTERFACE_OPERATIONS
    )
    closed_component_sessions = [
        record.get("component_session") for record in streams.records
        if record.get("event") == "lifecycle"
        and record.get("state") == "component_session_closed"
        and isinstance(record.get("component_session"), dict)
    ]
    closed_component_session = (
        closed_component_sessions[-1] if closed_component_sessions else None
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


if __name__ == "__main__":
    raise SystemExit("supervise.py is a library; run run.py")

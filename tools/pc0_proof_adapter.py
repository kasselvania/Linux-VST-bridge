#!/usr/bin/env python3
"""Closed diagnostic-only PC0 adapter for the classified proof backend.

PX2 owns every campaign reservation.  This module binds one immutable plan,
performs read-only admission before that reservation, and invokes a reservation-bound diagnostic worker over pinned PC0 primitives.  It has no product
evidence renderer and never calls the retired top-level transaction driver.
"""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
import os
import pathlib
import pwd
import re
import shlex
import subprocess
import sys
from types import MappingProxyType
from typing import Any, Mapping, Protocol, Sequence

from pc0_diagnostic_runtime import validate_runtime_observation

from classified_proof_backend import (
    BackendError,
    DiagnosticPlanAdapter,
    Observation,
    ObservationKind,
    OutcomeUnknown,
    PlanDescriptor,
    PreflightContext,
    RESERVATION_SCHEMA,
    ReservationContext,
)
from proof_execution_policy import (
    ExecutionClass,
    HEX40,
    HEX64,
    canonical_json,
    parse_canonical_json,
    sha256_bytes,
)


PLAN_ID = "pc0-pre-setup-processing-contract-diagnostic-v1"
PRODUCT_CONTRACT_IDENTITY = "pc0-selection-v2"
SELECTION_PATH = "docs/slices/PC0/SLICE_SELECTION.md"
SELECTION_GIT_BLOB = "9bd3f17a792baed8d122cfa765c38146ee51627f"
SELECTION_SHA256 = "daa042e4184cb5fffdf1ff08d59cc51f7755b4635c85e02597adc2584c4b4c1d"

STOPPED_PC0_SOURCE = "309b8918c128c0b9e6701d0453dc841a111d5ac5"
STOPPED_PC0_TREE = "a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92"
STOPPED_PC0_ARCHIVE_REF = "refs/heads/codex/archive/pc0-v3-stopped-309b8918"
STOPPED_PC0_EXECUTION_REF = \
    "refs/heads/codex/pc0-windows-vst3-pre-setup-processing-contract"
PC0_LEGACY_PLAN_SHA256 = \
    "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248"
PC0_RESULT_SCHEMA = "linux-vst-bridge-pc0-transaction-result/v1"
PC0_FAILURE_DIAGNOSTIC_SCHEMA = "linux-vst-bridge-pc0-failure-diagnostic/v1"
PC0_DECK_EXECUTION_INPUT_SCHEMA = "linux-vst-bridge-dx0-deck-execution-input/v1"
PC0_LEGACY_PLAN = MappingProxyType({
    "schema": "linux-vst-bridge-dx0-proof-plan/v1",
    "plan_id": "pc0-pre-setup-processing-contract-v1",
    "accepted_fixture_id": "wa0-again-accepted-v1",
    "host_mode": "host_only",
    "deterministic_validation_set": "pc0-pre-setup-deterministic-v1",
    "live_deck_batch": "pc0-positive-only-v1",
    "expected_result": "pc0-pre-setup-contract-complete-v1",
    "evidence_renderer": "pc0-five-file-renderer-v1",
})
if sha256_bytes(canonical_json(dict(PC0_LEGACY_PLAN))) != PC0_LEGACY_PLAN_SHA256:
    raise RuntimeError("closed PC0 lower-level plan identity differs")

STOPPED_SOURCE_ROLE = MappingProxyType({
    "identity_sha256": "7561b330c6fc64c3b7f217d7f886a2029b1435e4a7fa81dab8a016b5cca7b03a",
    "commit": STOPPED_PC0_SOURCE,
    "tree": STOPPED_PC0_TREE,
    "parent": "7ed1fbf985b5bb717883e0cd2132620b960e3aa6",
    "ref": STOPPED_PC0_EXECUTION_REF,
    "manifest_sha256": "524bc1bc1fdbf067f940f473c477c4dd3039e10999c46456c474663f7c46973b",
})

WINDOWS_BUILD_INPUT_IDENTITY = \
    "575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849"
PRODUCER_SOURCE = "7ac6095a488d0077fcc78fedc5abd870b5ffb1cb"
PRODUCER_RUN_ID = 33812659869
PRODUCER_RUN_ATTEMPT = 1
PRODUCER_ARTIFACT_ID = 9915439437
PRODUCER_SOURCE_ROLE = MappingProxyType({
    "identity_sha256": "f695d7d280485c0bc5c230c1a1f01ade1d43aa4bc0e63c8587a3e2388f2cb5a2",
    "commit": PRODUCER_SOURCE,
    "tree": "ad4230a713a2bb644476be8be9d374a1feb36b06",
    "parent": "1c0c31c4ab69a40303cd00b155ca30626323c451",
    "ref": STOPPED_PC0_EXECUTION_REF,
    "manifest_sha256": "b4460526006243ed4b384924552494efa53208a2196ff3b067190b2e66144363",
})
HOST_MANIFEST_SHA256 = \
    "d0e11c374b7b1cb99b357faaa109e9310edc265c159559bd59a2098148484e9c"
AGAIN_MODULE_SHA256 = \
    "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
AGAIN_BUNDLE_MANIFEST_SHA256 = \
    "bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164"
ACCEPTED_FIXTURE_IDENTITY = \
    "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6"
RUNTIME_PROTON_IDENTITY = \
    "2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547"

PREFLIGHT_SCHEMA = "linux-vst-bridge-pc0-classified-diagnostic-preflight/v1"
PAYLOAD_SCHEMA = "linux-vst-bridge-pc0-classified-diagnostic-observation/v2"
ADMITTED_SCHEMA = "linux-vst-bridge-pc0-classified-diagnostic-result/v2"
MAX_PREFLIGHT_BYTES = 256 * 1024
MAX_REMOTE_DOCUMENT_BYTES = 2 * 1024 * 1024
MAX_SIDECAR_BYTES = 256
MAX_INTENT_BYTES = 256 * 1024

PC0_RESULT_KEYS = {
    "schema", "operation_nonce", "artifact_producer_source",
    "deck_execution_source", "execution_input", "host_artifact",
    "accepted_fixture", "source_handoff", "closed_plan",
    "original_observation", "positive_result", "call_facts", "quiescence",
    "shutdown", "cleanup", "protected_state", "integrity",
}
PC0_DIAGNOSTIC_KEYS = {
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
PC0_INCONCLUSIVE_CLASSIFICATIONS = {
    "output_publication_failed", "supervision_failed",
    "supervision_and_process_cleanup_failed", "process_cleanup_failed",
}
PC0_DIAGNOSTIC_OPERATIONS = {
    "load_library", "init_dll", "get_plugin_factory", "get_factory_info",
    "query_factory_2", "query_factory_3", "count_classes",
    "get_class_info_unicode", "get_class_info_2", "get_class_info_1",
    "create_component", "get_controller_class_id", "initialize_component",
    "query_audio_processor", "get_bus_count", "get_bus_info",
    "get_bus_arrangement", "can_process_sample_size", "release_audio_processor",
    "terminate_component", "release_component", "release_factory_3",
    "release_factory_2", "release_factory_base", "exit_dll", "free_library",
}
PC0_AUDIO_OBSERVER_STATES = {
    "audio_processor_absent", "audio_processor_query_in_flight",
    "audio_processor_query_returned_without_lease",
    "audio_processor_lease_acquired", "audio_processor_release_in_flight",
    "audio_processor_lease_retired", "audio_processor_retirement_incomplete",
    "audio_processor_ownership_unknown",
}
PC0_SHUTDOWN_OPERATIONS = {
    "terminate_component", "release_component", "release_factory_3",
    "release_factory_2", "release_factory_base", "exit_dll", "free_library",
}
PROCESS_KEYS = {
    "bitwig", "validator", "wine", "proton", "runtime", "umu",
    "yabridge", "wf0",
}
WRITE_EFFECT_KEYS = {
    "environment_creations", "execution_intent_publications",
    "execution_lock_creations", "result_publications",
    "diagnostic_publications", "protected_state_mutations",
    "deck_execution_reservations", "deck_execution_count_increments",
    "proton_launches",
}

REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parents[1]


class AdapterBoundaryError(BackendError):
    """A PC0 adapter identity, transport, or private diagnostic was refused."""


class PortTimeout(RuntimeError):
    """A bounded port call ended without a knowable remote outcome."""


@dataclass(frozen=True, slots=True)
class CommandReply:
    returncode: int
    stdout: bytes
    stderr: bytes


class CommandPort(Protocol):
    def run(
        self, argv: Sequence[str], *, cwd: pathlib.Path | None = None,
        timeout: float = 30.0, input_bytes: bytes | None = None,
    ) -> CommandReply: ...


class FileSystemPort(Protocol):
    def read_bytes(self, path: pathlib.Path, maximum: int) -> bytes: ...


class SSHPort(Protocol):
    def run_python(
        self, worktree: str, program: str, arguments: Sequence[str], *,
        timeout: float,
    ) -> bytes: ...


@dataclass(frozen=True, slots=True)
class AdapterPorts:
    command: CommandPort
    filesystem: FileSystemPort
    ssh: SSHPort


class SubprocessCommandPort:
    """Bounded shell-free local command port."""

    def run(
        self, argv: Sequence[str], *, cwd: pathlib.Path | None = None,
        timeout: float = 30.0, input_bytes: bytes | None = None,
    ) -> CommandReply:
        try:
            result = subprocess.run(
                list(argv), cwd=cwd, input=input_bytes,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                timeout=timeout, check=False,
            )
        except subprocess.TimeoutExpired as exc:
            raise PortTimeout("bounded command outcome is unknown") from exc
        if len(result.stdout) > MAX_REMOTE_DOCUMENT_BYTES or len(result.stderr) > 65536:
            raise AdapterBoundaryError("bounded command output exceeded its limit")
        return CommandReply(result.returncode, result.stdout, result.stderr)


class OSFileSystemPort:
    """Read-only regular-file port with a caller-supplied byte ceiling."""

    def read_bytes(self, path: pathlib.Path, maximum: int) -> bytes:
        if (not path.is_file() or path.is_symlink() or maximum < 1
                or path.stat().st_size > maximum):
            raise AdapterBoundaryError(f"required regular file is absent or unsafe: {path.name}")
        value = path.read_bytes()
        if not value or len(value) > maximum:
            raise AdapterBoundaryError(f"required file is empty or oversized: {path.name}")
        return value


class StrictSSHPort:
    """Lazy no-forwarding SSH lane used only after live authority admits it."""

    def __init__(self, command: CommandPort) -> None:
        self.command = command
        self._destination: str | None = None

    def _discover(self) -> str:
        if self._destination is not None:
            return self._destination
        tailscale = pathlib.Path("/Applications/Tailscale.app/Contents/MacOS/Tailscale")
        reply = self.command.run((str(tailscale), "status", "--json"), timeout=30.0)
        if reply.returncode != 0:
            raise AdapterBoundaryError("established Tailscale status is unavailable")
        try:
            status = json.loads(reply.stdout.decode("utf-8", "strict"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise AdapterBoundaryError("Tailscale status is malformed") from exc
        candidates: list[str] = []
        for peer in (status.get("Peer") or {}).values():
            if (peer.get("Online") is True and peer.get("HostName") == "steamdeck"
                    and str(peer.get("OS", "")).lower() == "linux"):
                addresses = peer.get("TailscaleIPs") or []
                if addresses and isinstance(addresses[0], str):
                    candidates.append(f"deck@{addresses[0]}")
        matches: list[str] = []
        for candidate in candidates:
            probe = self._ssh(
                candidate,
                "test \"$(cat /sys/devices/virtual/dmi/id/product_name)\" = Galileo",
                timeout=10.0,
            )
            if probe.returncode == 0:
                matches.append(candidate)
        if len(matches) != 1:
            raise AdapterBoundaryError("exact online Steam Deck is absent or ambiguous")
        self._destination = matches[0]
        return matches[0]

    def _ssh(self, destination: str, script: str, *, timeout: float, input_bytes: bytes | None = None) -> CommandReply:
        return self.command.run((
            "/usr/bin/ssh", "-o", "ForwardAgent=no", "-o", "BatchMode=yes",
            "-o", "StrictHostKeyChecking=yes", destination, script,
        ), timeout=timeout, input_bytes=input_bytes)

    def _run(self, script: str, *, timeout: float, input_bytes: bytes | None = None) -> CommandReply:
        return self._ssh(self._discover(), script, timeout=timeout, input_bytes=input_bytes)

    def run_python(
        self, worktree: str, program: str, arguments: Sequence[str], *,
        timeout: float,
    ) -> bytes:
        raw = program.encode("utf-8")
        # The inherited guard scans argv for runtime names. Verified helper bytes
        # travel only on stdin; neither the SSH command nor Python argv embeds it.
        loader = ("import hashlib,sys; b=sys.stdin.buffer.read(262145); "
                  "assert len(b)<=262144 and hashlib.sha256(b).hexdigest()==sys.argv[1]; "
                  "exec(compile(b,'<diagnostic-worker>','exec'))")
        invocation = shlex.join((
            "env", "-u", "GH_TOKEN", "-u", "GITHUB_TOKEN", "-u",
            "GITHUB_PAT", "-u", "SSH_AUTH_SOCK", "PYTHONDONTWRITEBYTECODE=1",
            "/usr/bin/python3", "-B", "-c", loader, sha256_bytes(raw), *arguments,
        ))
        reply = self._run(f"cd {shlex.quote(worktree)} && {invocation}",
                          timeout=timeout, input_bytes=raw)
        if reply.returncode != 0:
            raise AdapterBoundaryError("bounded diagnostic worker failed")
        if not reply.stdout or len(reply.stdout) > MAX_REMOTE_DOCUMENT_BYTES:
            raise AdapterBoundaryError("diagnostic worker output differs")
        return reply.stdout


ARTIFACT_REQUIREMENT = MappingProxyType({
    "windows_build_input_identity": WINDOWS_BUILD_INPUT_IDENTITY,
    "producer_source_commit": PRODUCER_SOURCE,
    "producer_run_id": PRODUCER_RUN_ID,
    "producer_run_attempt": PRODUCER_RUN_ATTEMPT,
    "artifact_id": PRODUCER_ARTIFACT_ID,
    "host_manifest_sha256": HOST_MANIFEST_SHA256,
})
FIXTURE_REQUIREMENT = MappingProxyType({
    "again_module_sha256": AGAIN_MODULE_SHA256,
    "again_bundle_manifest_sha256": AGAIN_BUNDLE_MANIFEST_SHA256,
    "accepted_fixture_identity_sha256": ACCEPTED_FIXTURE_IDENTITY,
})
RUNTIME_REQUIREMENT = MappingProxyType({
    "runtime_proton_identity_sha256": RUNTIME_PROTON_IDENTITY,
    "stopped_pc0_source_commit": STOPPED_PC0_SOURCE,
    "stopped_pc0_source_tree": STOPPED_PC0_TREE,
    "stopped_pc0_archive_ref": STOPPED_PC0_ARCHIVE_REF,
    "remote_pc0_plan_sha256": PC0_LEGACY_PLAN_SHA256,
})


def _git_blob_sha1(raw: bytes) -> str:
    return hashlib.sha1(f"blob {len(raw)}\0".encode("ascii") + raw).hexdigest()


def _selection_bytes() -> bytes:
    raw = (REPOSITORY_ROOT / SELECTION_PATH).read_bytes()
    if (_git_blob_sha1(raw) != SELECTION_GIT_BLOB
            or sha256_bytes(raw) != SELECTION_SHA256):
        raise AdapterBoundaryError("PC0 selection contract identity differs")
    return raw


PLAN_DESCRIPTOR = PlanDescriptor.create(
    plan_id=PLAN_ID,
    execution_class=ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE,
    product_contract_identity=PRODUCT_CONTRACT_IDENTITY,
    product_contract_bytes=_selection_bytes(),
    operation="pc0-pre-setup-processing-contract-diagnostic",
    artifact_requirement=ARTIFACT_REQUIREMENT,
    fixture_requirement=FIXTURE_REQUIREMENT,
    runtime_requirement=RUNTIME_REQUIREMENT,
)
PLAN_CONTENT_SHA256 = PLAN_DESCRIPTOR.plan_content_sha256


def _real_proof_root() -> pathlib.Path:
    home = pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)
    if not home.is_absolute():
        raise AdapterBoundaryError("OS account home is not absolute")
    if sys.platform == "darwin":
        return home / "Library" / "Application Support" / "Linux VST Bridge" / "proof"
    return home / ".local" / "share" / "linux-vst-bridge" / "proof"


def _as_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, Mapping):
        raise AdapterBoundaryError(f"{label} is not one object")
    return dict(value)


def _keys(value: Any, expected: set[str], label: str) -> dict[str, Any]:
    item = _as_object(value, label)
    if set(item) != expected:
        raise AdapterBoundaryError(f"{label} key roster differs")
    return item


def _canonical_object(raw: bytes, maximum: int, label: str) -> dict[str, Any]:
    try:
        value = parse_canonical_json(raw, maximum=maximum)
    except Exception as exc:
        raise AdapterBoundaryError(f"{label} is not bounded canonical JSON") from exc
    return _as_object(value, label)


def _pair(raw: bytes, sidecar: bytes, filename: str, label: str) -> tuple[dict[str, Any], str]:
    if len(sidecar) > MAX_SIDECAR_BYTES:
        raise AdapterBoundaryError(f"{label} sidecar exceeds its bound")
    value = _canonical_object(raw, MAX_REMOTE_DOCUMENT_BYTES, label)
    digest = sha256_bytes(raw)
    if sidecar != f"{digest}  {filename}\n".encode("ascii"):
        raise AdapterBoundaryError(f"{label} sidecar differs")
    return value, digest


def _checked_text(reply: CommandReply, label: str) -> str:
    if reply.returncode != 0:
        raise AdapterBoundaryError(f"{label} failed")
    try:
        value = reply.stdout.decode("utf-8", "strict").strip()
    except UnicodeDecodeError as exc:
        raise AdapterBoundaryError(f"{label} output is not UTF-8") from exc
    if not value or len(value) > 4096:
        raise AdapterBoundaryError(f"{label} output differs")
    return value


WORKER_PATH = "tools/pc0_diagnostic_worker.py"
WORKER_SOURCE = (REPOSITORY_ROOT / WORKER_PATH).read_text(encoding="utf-8")
SUPPORT_SOURCES = {name: (REPOSITORY_ROOT / ("tools/" + name + ".py")).read_text(encoding="utf-8")
                   for name in ("pc0_diagnostic_runtime", "pc0_diagnostic_primitives")}
REMOTE_PREFLIGHT_PROGRAM = WORKER_SOURCE.replace("_SUPPORT_SOURCES = {}", "_SUPPORT_SOURCES = " + repr(SUPPORT_SOURCES))
WORKER_SHA256 = sha256_bytes(REMOTE_PREFLIGHT_PROGRAM.encode("utf-8"))
WORKER_SCHEMA = "linux-vst-bridge-pc0-reservation-diagnostic/v2"


class PC0DiagnosticRuntime:
    """Four closed adapter operations over injected local, filesystem, and SSH ports."""

    def __init__(self, ports: AdapterPorts, *, repository: pathlib.Path = REPOSITORY_ROOT,
                 proof_root: pathlib.Path | None = None) -> None:
        self.ports = ports
        self.repository = repository
        self.proof_root = proof_root or _real_proof_root()

    def preflight(self, context: PreflightContext) -> None:
        delegation = self._validate_context(context.delegation, context.plan_descriptor)
        request = self._request(delegation, None)
        # PX2 calls preflight even on resume. Bounded retrieval must precede any
        # launch-safety probe, including local custody checks that may now fail.
        existing = self._remote("inspect", request)
        if existing["state"] != "absent":
            return
        self._local_preflight(delegation)
        self._remote("preflight", request)

    def reconcile(self, context: ReservationContext) -> Observation | None:
        delegation = self._validate_context(context.delegation, context.plan_descriptor)
        request = self._request(delegation, context.reservation_identity)
        try:
            return self._observation(self._remote("inspect", request), request)
        except Exception:
            return None

    def invoke_diagnostic(self, context: ReservationContext) -> Observation:
        delegation = self._validate_context(context.delegation, context.plan_descriptor)
        request = self._request(delegation, context.reservation_identity)
        attempted = False
        try:
            existing = self._remote("inspect", request)
            if existing["state"] == "absent":
                self._local_preflight(delegation)
                self._remote("preflight", request)
                attempted = True
                existing = self._remote("execute", request)
            observation = self._observation(existing, request)
            if observation is not None:
                return observation
            raise OutcomeUnknown(
                effects=existing.get("effects", {"deck_workloads": "unknown"}),
                cleanup_disposition="UNKNOWN", protected_state_disposition="UNKNOWN")
        except OutcomeUnknown:
            raise
        except (PortTimeout, AdapterBoundaryError) as exc:
            raise OutcomeUnknown(
                effects={"deck_workloads": "unknown" if attempted else 0,
                         "diagnostic_publications": "unknown" if attempted else 0},
                cleanup_disposition="UNKNOWN", protected_state_disposition="UNKNOWN") from exc

    def admit_diagnostic(self, context: ReservationContext, observation: Observation) -> Mapping[str, Any]:
        delegation = self._validate_context(context.delegation, context.plan_descriptor)
        request = self._request(delegation, context.reservation_identity)
        payload = _keys(observation.payload, {
            "schema", "execution_class", "acceptance_eligible", "binding",
            "document_sha256", "diagnostic_data", "runtime_observation"}, "diagnostic payload")
        if (observation.kind is not ObservationKind.SUCCESS
                or observation.cleanup_disposition != "COMPLETE"
                or observation.protected_state_disposition != "UNCHANGED"
                or payload["schema"] != PAYLOAD_SCHEMA
                or payload["execution_class"] != request["execution_class"]
                or payload["acceptance_eligible"] is not False
                or payload["binding"] != request
                or HEX64.fullmatch(str(payload["document_sha256"])) is None):
            raise AdapterBoundaryError("successful diagnostic binding differs")
        return {"schema": ADMITTED_SCHEMA, **{k: v for k, v in payload.items() if k != "schema"}}

    def _validate_context(
        self, delegation_value: Mapping[str, Any], descriptor_value: Mapping[str, Any],
    ) -> dict[str, Any]:
        delegation = dict(delegation_value)
        descriptor = dict(descriptor_value)
        if descriptor != PLAN_DESCRIPTOR.record():
            raise AdapterBoundaryError("PC0 diagnostic plan descriptor differs")
        if (delegation.get("plan_id") != PLAN_ID
                or delegation.get("execution_class")
                != ExecutionClass.DIAGNOSTIC_NON_AUTHORITATIVE.value
                or delegation.get("acceptance_eligible") is not False
                or delegation.get("product_contract_identity")
                != PRODUCT_CONTRACT_IDENTITY
                or delegation.get("product_contract_sha256") != SELECTION_SHA256
                or delegation.get("plan_content_sha256") != PLAN_CONTENT_SHA256
                or HEX40.fullmatch(str(delegation.get("source_commit", ""))) is None
                or HEX64.fullmatch(str(delegation.get("execution_identity", ""))) is None):
            raise AdapterBoundaryError("PC0 diagnostic delegation differs")
        return delegation

    def _request(
        self, delegation: Mapping[str, Any], reservation_identity: str | None,
    ) -> dict[str, Any]:
        projected = sha256_bytes(canonical_json({
            "schema": RESERVATION_SCHEMA,
            "execution_class": delegation["execution_class"],
            "execution_identity": delegation["execution_identity"],
            "source_commit": delegation["source_commit"],
            "product_contract_identity": delegation["product_contract_identity"],
            "product_contract_sha256": delegation["product_contract_sha256"],
            "plan_id": delegation["plan_id"],
            "plan_content_sha256": delegation["plan_content_sha256"],
        }))
        if reservation_identity is not None and reservation_identity != projected:
            raise AdapterBoundaryError("reservation identity differs from delegation")
        return {
            "execution_class": delegation["execution_class"],
            "acceptance_eligible": False,
            "campaign_identity": delegation["execution_identity"],
            "reservation_identity": projected,
            "adapter_source_commit": delegation["source_commit"],
            "worker_sha256": WORKER_SHA256,
            "execution_source": dict(STOPPED_SOURCE_ROLE),
            "producer_source": dict(PRODUCER_SOURCE_ROLE),
            "windows_build_input_identity": WINDOWS_BUILD_INPUT_IDENTITY,
            "producer_run_id": PRODUCER_RUN_ID,
            "producer_run_attempt": PRODUCER_RUN_ATTEMPT,
            "artifact_id": PRODUCER_ARTIFACT_ID,
            "host_manifest_sha256": HOST_MANIFEST_SHA256,
            "fixture_identity": ACCEPTED_FIXTURE_IDENTITY,
            "fixture_bundle_sha256": AGAIN_BUNDLE_MANIFEST_SHA256,
            "fixture_module_sha256": AGAIN_MODULE_SHA256,
            "runtime_identity": RUNTIME_PROTON_IDENTITY,
            "legacy_plan_sha256": PC0_LEGACY_PLAN_SHA256,
            "plan_content_sha256": PLAN_CONTENT_SHA256,
        }

    def _local_preflight(self, delegation: Mapping[str, Any]) -> None:
        selection = self.ports.filesystem.read_bytes(
            self.repository / SELECTION_PATH, 256 * 1024,
        )
        if (_git_blob_sha1(selection) != SELECTION_GIT_BLOB
                or sha256_bytes(selection) != SELECTION_SHA256):
            raise AdapterBoundaryError("PC0 selection bytes differ")
        head = _checked_text(
            self.ports.command.run(
                ("git", "rev-parse", "HEAD"), cwd=self.repository,
            ),
            "diagnostic source HEAD",
        )
        clean = self.ports.command.run(
            ("git", "status", "--porcelain=v1", "--untracked-files=all"),
            cwd=self.repository,
        )
        if (head != delegation["source_commit"] or clean.returncode != 0
                or clean.stdout != b"" or clean.stderr != b""):
            raise AdapterBoundaryError("diagnostic source checkout differs")
        commands = (
            (("git", "rev-parse", "--verify", f"{delegation['source_commit']}^{{commit}}"),
             str(delegation["source_commit"]), "diagnostic source"),
            (("git", "rev-parse", f"{delegation['source_commit']}:{SELECTION_PATH}"),
             SELECTION_GIT_BLOB, "diagnostic source contract"),
            (("git", "rev-parse", f"{delegation['source_commit']}:{WORKER_PATH}"),
             _git_blob_sha1(WORKER_SOURCE.encode("utf-8")), "diagnostic worker"),
            (("git", "rev-parse", "--verify", f"{STOPPED_PC0_ARCHIVE_REF}^{{commit}}"),
             STOPPED_PC0_SOURCE, "stopped source archive"),
            (("git", "rev-parse", f"{STOPPED_PC0_SOURCE}^{{tree}}"),
             STOPPED_PC0_TREE, "stopped source tree"),
        )
        for argv, expected, label in commands:
            observed = _checked_text(
                self.ports.command.run(argv, cwd=self.repository), label,
            )
            if observed != expected:
                raise AdapterBoundaryError(f"{label} identity differs")
        for name, source in SUPPORT_SOURCES.items():
            blob = _checked_text(self.ports.command.run(
                ("git", "rev-parse", f"{delegation['source_commit']}:tools/{name}.py"),
                cwd=self.repository), "diagnostic helper")
            if blob != _git_blob_sha1(source.encode()):
                raise AdapterBoundaryError("diagnostic helper source differs")
        self._validate_mac_stores()

    def _validate_mac_stores(self) -> None:
        host = self.proof_root / "host-artifacts" / "by-manifest" / HOST_MANIFEST_SHA256
        manifest = self._local_pair(
            host / "DX0_HOST_ARTIFACT_MANIFEST.json", HOST_MANIFEST_SHA256,
        )
        if manifest.get("windows_build_input_sha256") != WINDOWS_BUILD_INPUT_IDENTITY:
            raise AdapterBoundaryError("Mac host manifest build identity differs")
        build, _build_digest = self._local_pair_any(
            host / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json",
        )
        custody, _custody_digest = self._local_pair_any(
            host / "DX0_MAC_HOST_CUSTODY_RECEIPT.json",
        )
        if (build.get("producer_source") != dict(PRODUCER_SOURCE_ROLE)
                or build.get("windows_build_input", {}).get("sha256")
                != WINDOWS_BUILD_INPUT_IDENTITY
                or build.get("workflow", {}).get("run_id") != PRODUCER_RUN_ID
                or build.get("workflow", {}).get("run_attempt") != PRODUCER_RUN_ATTEMPT
                or build.get("artifact", {}).get("manifest_sha256")
                != HOST_MANIFEST_SHA256
                or custody.get("producer_source") != dict(PRODUCER_SOURCE_ROLE)
                or custody.get("windows_build_input_sha256")
                != WINDOWS_BUILD_INPUT_IDENTITY
                or custody.get("workflow", {}).get("run_id") != PRODUCER_RUN_ID
                or custody.get("workflow", {}).get("run_attempt")
                != PRODUCER_RUN_ATTEMPT
                or custody.get("artifact", {}).get("id") != PRODUCER_ARTIFACT_ID
                or custody.get("inner_envelope", {}).get(
                    "host_artifact_manifest_sha256") != HOST_MANIFEST_SHA256):
            raise AdapterBoundaryError("Mac host store receipt identity differs")

        fixture = (self.proof_root / "fixtures" / "by-manifest"
                   / AGAIN_BUNDLE_MANIFEST_SHA256)
        receipt = self._local_pair(
            fixture / "DX0_ACCEPTED_FIXTURE_RECEIPT.json",
            ACCEPTED_FIXTURE_IDENTITY,
        )
        bundle = receipt.get("bundle_manifest", {})
        records = bundle.get("records", []) if isinstance(bundle, Mapping) else []
        module = next((item for item in records
                       if isinstance(item, Mapping)
                       and item.get("path") == "Contents/x86_64-win/again.vst3"), None)
        if (bundle.get("sha256") != AGAIN_BUNDLE_MANIFEST_SHA256
                or not isinstance(module, Mapping)
                or module.get("sha256") != AGAIN_MODULE_SHA256):
            raise AdapterBoundaryError("Mac AGain fixture receipt differs")
        module_bytes = self.ports.filesystem.read_bytes(
            fixture / "again.vst3" / "Contents" / "x86_64-win" / "again.vst3",
            128 * 1024 * 1024,
        )
        if sha256_bytes(module_bytes) != AGAIN_MODULE_SHA256:
            raise AdapterBoundaryError("Mac AGain module bytes differ")

    def _local_pair(self, path: pathlib.Path, expected_digest: str) -> dict[str, Any]:
        value, digest = self._local_pair_any(path)
        if digest != expected_digest:
            raise AdapterBoundaryError(f"{path.name} identity differs")
        return value

    def _local_pair_any(self, path: pathlib.Path) -> tuple[dict[str, Any], str]:
        raw = self.ports.filesystem.read_bytes(path, MAX_REMOTE_DOCUMENT_BYTES)
        sidecar = self.ports.filesystem.read_bytes(
            path.with_suffix(".sha256"), MAX_SIDECAR_BYTES,
        )
        value, digest = _pair(raw, sidecar, path.name, path.name)
        return value, digest

    def _worktree(self) -> str:
        return ("/home/deck/.local/share/linux-vst-bridge/worktrees/dx0/"
                + STOPPED_PC0_SOURCE)

    def _remote(self, action: str, request: Mapping[str, Any]) -> dict[str, Any]:
        raw = self.ports.ssh.run_python(
            self._worktree(), REMOTE_PREFLIGHT_PROGRAM,
            (action, canonical_json(dict(request)).hex()),
            timeout=900.0 if action == "execute" else 60.0)
        reply = _canonical_object(raw, MAX_REMOTE_DOCUMENT_BYTES, "diagnostic reply")
        if reply.get("state") not in {"ready", "absent", "unknown", "observed"}:
            raise AdapterBoundaryError("diagnostic state differs")
        if action == "preflight" and reply["state"] != "ready":
            raise AdapterBoundaryError("launch preflight is not ready")
        if action != "preflight" and reply["state"] == "ready":
            raise AdapterBoundaryError("unexpected preflight reply")
        # Retrieval never inherits historical execution/publication counts.
        if action == "inspect":
            reply["effects"] = {"deck_workloads": 0, "diagnostic_publications": 0}
        return reply

    def _observation(self, reply: Mapping[str, Any], request: Mapping[str, Any]) -> Observation | None:
        if reply["state"] in {"absent", "unknown"}:
            return None
        value = _keys(reply.get("observation"), {
            "schema", "binding", "execution_input_sha256", "kind",
            "classification", "summary", "cleanup", "protected", "runtime_observation"}, "reservation observation")
        if (value["schema"] != WORKER_SCHEMA or value["binding"] != request
                or HEX64.fullmatch(str(value["execution_input_sha256"])) is None
                or value["classification"] not in {
                    "PC0_DIAGNOSTIC_OBSERVED", "PC0_DIAGNOSTIC_FAILED", "PC0_DIAGNOSTIC_INCONCLUSIVE"}):
            raise AdapterBoundaryError("reservation observation binding differs")
        runtime_digest = validate_runtime_observation(value["runtime_observation"])
        kind = ObservationKind(value["kind"])
        summary = _as_object(value["summary"], "diagnostic summary")
        if len(canonical_json(summary)) > MAX_REMOTE_DOCUMENT_BYTES - 16384:
            raise AdapterBoundaryError("diagnostic summary exceeds bound")
        if kind is ObservationKind.SUCCESS:
            if (value["cleanup"] != "COMPLETE" or value["protected"] != "UNCHANGED"
                    or set(summary) != {"run_id", "processing_contract"}
                    or not isinstance(summary["processing_contract"], dict)):
                raise AdapterBoundaryError("successful diagnostic facts differ")
        elif set(summary) != {"failure"}:
            raise AdapterBoundaryError("failure diagnostic facts absent")
        else:
            failure = _as_object(summary["failure"], "failure diagnostic")
            raw = canonical_json(failure)
            compact, _digest, _kind, _cleanup = self._validate_diagnostic_pair(
                raw, f"{sha256_bytes(raw)}  PC0_FAILURE_DIAGNOSTIC.json\n".encode(),
                value["execution_input_sha256"], request["reservation_identity"][:32],
                request["reservation_identity"][32:], failure.get("protected_snapshot_sha256"),
                expected_runtime_identity=runtime_digest)
            summary = compact
        return Observation(
            kind, value["classification"],
            {"schema": PAYLOAD_SCHEMA, "execution_class": request["execution_class"],
             "acceptance_eligible": False, "binding": dict(request),
             "document_sha256": sha256_bytes(canonical_json(value)), "diagnostic_data": summary,
             "runtime_observation": value["runtime_observation"]},
            value["cleanup"], value["protected"],
            reply.get("effects", {"deck_workloads": "unknown", "diagnostic_publications": "unknown"}))

    def _validate_diagnostic_pair(
        self, raw: bytes, sidecar: bytes,
        expected_execution_input_sha256: str,
        expected_operation_nonce: str,
        expected_phase_nonce: str,
        expected_protected_snapshot_sha256: str,
        *, expected_runtime_identity: str = RUNTIME_PROTON_IDENTITY,
    ) -> tuple[dict[str, Any], str, ObservationKind, str]:
        diagnostic, digest = _pair(
            raw, sidecar, "PC0_FAILURE_DIAGNOSTIC.json", "PC0 diagnostic",
        )
        _keys(diagnostic, PC0_DIAGNOSTIC_KEYS, "PC0 diagnostic")
        source = _keys(
            diagnostic["execution_source"], set(STOPPED_SOURCE_ROLE),
            "diagnostic source",
        )
        classification = diagnostic["classification"]
        raw_exit = diagnostic["raw_exit"]
        cleanup = _keys(
            diagnostic["cleanup"], {"owned_descendants_zero", "process_group_empty"},
            "diagnostic cleanup",
        )
        call_counts = _keys(
            diagnostic["call_counts"], PC0_DIAGNOSTIC_OPERATIONS,
            "diagnostic call counts",
        )
        records = diagnostic["durable_records"]
        shutdown = _keys(diagnostic["inherited_shutdown"], {
            "operations", "clean_in_process_shutdown", "physical_containment_only",
        }, "diagnostic inherited shutdown")
        shutdown_operations = _keys(
            shutdown["operations"], PC0_SHUTDOWN_OPERATIONS,
            "diagnostic shutdown operations",
        )
        if (diagnostic["schema"] != PC0_FAILURE_DIAGNOSTIC_SCHEMA
                or diagnostic["operation_nonce"] != expected_operation_nonce
                or diagnostic["phase_nonce"] != expected_phase_nonce
                or source != dict(STOPPED_SOURCE_ROLE)
                or diagnostic["proof_plan_sha256"] != PC0_LEGACY_PLAN_SHA256
                or diagnostic["execution_input_sha256"]
                != expected_execution_input_sha256
                or classification not in PC0_DIAGNOSTIC_CLASSIFICATIONS
                or (classification == "output_publication_failed" and raw_exit != 99)
                or (raw_exit == 99 and classification not in {
                    "output_publication_failed", "supervision_failed",
                    "supervision_and_process_cleanup_failed"})
                or (raw_exit is not None and (type(raw_exit) is not int
                                               or not -255 <= raw_exit <= 255))
                or diagnostic["runner_identity_sha256"] != expected_runtime_identity
                or diagnostic["protected_snapshot_sha256"]
                != expected_protected_snapshot_sha256
                or re.fullmatch(r"[0-9a-f]{32}", str(diagnostic["run_id"])) is None
                or re.fullmatch(r"[A-Z][A-Z0-9_]{1,127}",
                                str(diagnostic["primary_blocker"])) is None
                or diagnostic["secondary_cleanup_blocker"] not in {
                    None, "PC0_PROCESS_CLEANUP_BLOCKED"}
                or type(diagnostic["durable_record_count"]) is not int
                or not isinstance(records, list)
                or diagnostic["durable_record_count"]
                != len(records)
                or diagnostic["durable_record_count"] > 2048
                or any(type(value) is not int or not 0 <= value <= 2048
                       for value in call_counts.values())
                or diagnostic["last_in_flight_operation"] not in {
                    None, *PC0_DIAGNOSTIC_OPERATIONS}
                or diagnostic["audio_processor_observer_state"]
                not in PC0_AUDIO_OBSERVER_STATES
                or type(diagnostic["audio_interface_quiescence"]) is not bool
                or type(shutdown["clean_in_process_shutdown"]) is not bool
                or type(shutdown["physical_containment_only"]) is not bool
                or diagnostic["environment_retirement_disposition"] not in {
                    "retired", "not_attempted_process_containment_unproved", "failed"}
                or type(diagnostic["stderr_bytes"]) is not int
                or not 0 <= diagnostic["stderr_bytes"] <= 65536
                or any(type(value) is not bool for value in cleanup.values())):
            raise AdapterBoundaryError("PC0 diagnostic identity or bounds differ")
        for sequence, record in enumerate(records, 1):
            if (not isinstance(record, Mapping)
                    or record.get("sequence") != sequence
                    or record.get("event") not in {
                        "lifecycle", "call_started", "call_completed",
                        "host_callback"}):
                raise AdapterBoundaryError("PC0 diagnostic record sequence differs")
        for disposition in shutdown_operations.values():
            item = _keys(
                disposition, {"disposition", "source"},
                "diagnostic shutdown entry",
            )
            if (item["disposition"] not in {
                    "completed", "attempted_without_ordinary_return",
                    "not_attempted_prior_stage",
                    "not_attempted_object_quiescence_unproved",
                    "not_attempted_audio_interface_quiescence_unproved"}
                    or item["source"] not in {
                        "scanner_call_ledger", "scanner_suppression_record",
                        "supervisor_unmatched_component_call",
                        "supervisor_call_ledger_absence"}):
                raise AdapterBoundaryError("PC0 diagnostic shutdown entry differs")
        for key in ("stdout_sha256", "stderr_sha256", "protected_snapshot_sha256"):
            if HEX64.fullmatch(str(diagnostic[key])) is None:
                raise AdapterBoundaryError(f"PC0 diagnostic {key} differs")
        kind = (ObservationKind.INCONCLUSIVE
                if classification in PC0_INCONCLUSIVE_CLASSIFICATIONS
                else ObservationKind.FAILED)
        cleanup_disposition = (
            "COMPLETE" if all(cleanup.values())
            and diagnostic["environment_retirement_disposition"] == "retired"
            else "INCOMPLETE"
        )
        summary = {
            "classification": classification,
            "primary_blocker": diagnostic["primary_blocker"],
            "secondary_cleanup_blocker": diagnostic["secondary_cleanup_blocker"],
            "raw_exit": raw_exit,
            "run_id": diagnostic["run_id"],
            "last_lifecycle": diagnostic["last_lifecycle"],
            "last_in_flight_operation": diagnostic["last_in_flight_operation"],
            "durable_record_count": diagnostic["durable_record_count"],
            "stdout_sha256": diagnostic["stdout_sha256"],
            "stderr_sha256": diagnostic["stderr_sha256"],
            "stderr_bytes": diagnostic["stderr_bytes"],
            "protected_snapshot_sha256": diagnostic["protected_snapshot_sha256"],
        }
        return summary, digest, kind, cleanup_disposition


def production_ports() -> AdapterPorts:
    command = SubprocessCommandPort()
    return AdapterPorts(command, OSFileSystemPort(), StrictSSHPort(command))


def create_pc0_diagnostic_adapter(
    ports: AdapterPorts | None = None, *, repository: pathlib.Path = REPOSITORY_ROOT,
    proof_root: pathlib.Path | None = None,
) -> DiagnosticPlanAdapter:
    runtime = PC0DiagnosticRuntime(
        ports or production_ports(), repository=repository, proof_root=proof_root,
    )
    return DiagnosticPlanAdapter(
        descriptor=PLAN_DESCRIPTOR,
        preflight=runtime.preflight,
        reconcile=runtime.reconcile,
        invoke=runtime.invoke_diagnostic,
        admit=runtime.admit_diagnostic,
    )


PC0_DIAGNOSTIC_ADAPTER = create_pc0_diagnostic_adapter()
PRODUCTION_ADAPTERS: Mapping[str, DiagnosticPlanAdapter] = MappingProxyType({
    PLAN_ID: PC0_DIAGNOSTIC_ADAPTER,
})

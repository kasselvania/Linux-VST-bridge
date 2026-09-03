#!/usr/bin/env python3
"""One-command DX0 proof transaction driver."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from typing import Any, Callable

ROOT = pathlib.Path(__file__).resolve().parents[1]
MODULES = ROOT / "tools/wf0-factory-census"
sys.path.insert(0, str(MODULES))
sys.dont_write_bytecode = True

from artifacts import (  # noqa: E402
    accepted_fixture_identity_sha256, create_source_handoff,
    publish_host_custody, seed_fixture, verify_fixture_store,
    verify_host_store, verify_source_handoff,
)
from common import (  # noqa: E402
    DX0_ACCEPTED_WA0_ARTIFACT_ID, DX0_ACCEPTED_WA0_WRAPPER_SHA256,
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_BASIS_COMMIT, DX0_BRANCH,
    DX0_DECK_EXECUTION_INPUT_SCHEMA, DX0_EVIDENCE_PATHS, DX0_HOST_MODE,
    DX0_MAC_HOST_CUSTODY_SCHEMA, DX0_PLAN_ID, DX0_REF, DX0_SOURCE_PATHS,
    DX0_TRANSACTION_SCHEMA, DX0_TRANSACTION_STATE_SCHEMA, REPOSITORY,
    canonical_json, command, command_text, dx0_closed_plan,
    dx0_complete_source, dx0_complete_source_sha256, dx0_deck_execution_input,
    dx0_evidence_renderer, dx0_identity_sha256, dx0_mac_host_artifact_parent,
    dx0_mac_fixture_parent, dx0_mac_result_parent, dx0_mac_transaction_parent,
    dx0_require_frozen_source, dx0_source_role, dx0_windows_build_input,
    fail, parse_json_no_duplicates, repo_root, sha256_bytes, sha256_file, write_atomic,
)
from evidence import (  # noqa: E402
    packet_identity, publish_packet, render_packet, result_admission_receipt,
    validate_result, validate_result_file,
)

API_VERSION = "2026-03-10"
WORKFLOW_PATH = ".github/workflows/wf0-windows-msvc-build.yml"
WORKFLOW_API_ID = "wf0-windows-msvc-build.yml"
ALLOWED_DISPOSITIONS = {
    "prepared", "in_flight", "recovered_in_flight", "completed", "reused",
    "failed", "outcome_unresolved",
}
DX0_OPERATIONS = {
    "derive_identities", "plan_external_work", "freeze_source", "verify_fixture",
    "reuse_or_produce_host", "custody_host_artifact", "create_source_handoff",
    "transfer_and_admit_deck_inputs", "execute_deck_batch",
    "retrieve_and_retain_result", "render_and_validate_evidence", "close_transaction",
}
EFFECT_COUNT_KEYS = {
    "windows_builds", "artifact_downloads", "custody_operations",
    "artifact_transfers", "source_transfers", "deck_executions", "evidence_renders",
}
DX0_STATES = {
    "transaction_absent", "planned", "source_frozen", "fixture_verified",
    "host_artifact_verified", "handoff_admitted", "deck_batch_in_flight",
    "transaction_result_retained", "evidence_rendered", "transaction_complete",
}
DX0_STATE_ORDER = (
    "transaction_absent", "planned", "source_frozen", "fixture_verified",
    "host_artifact_verified", "handoff_admitted", "deck_batch_in_flight",
    "transaction_result_retained", "evidence_rendered", "transaction_complete",
)
DX0_PHASE_TRANSITIONS = {
    "prepared": {"in_flight", "recovered_in_flight", "completed", "reused",
                 "failed", "outcome_unresolved"},
    "in_flight": {"recovered_in_flight", "completed", "failed",
                  "outcome_unresolved"},
    "recovered_in_flight": {"completed", "failed", "outcome_unresolved"},
    "outcome_unresolved": {"recovered_in_flight", "completed", "failed"},
}
DX0_TERMINAL_PHASE_DISPOSITIONS = {"completed", "reused", "failed"}
DX0_TRANSACTION_KEYS = {
    "schema", "operation_nonce", "artifact_producer_source",
    "deck_execution_source", "evidence_consumer_source",
    "windows_build_input", "host_artifact", "accepted_fixture",
    "deck_execution_input", "proof_plan", "transaction_result",
    "evidence_renderer", "phase_receipts",
}


class RemoteOutcomeUnknown(RuntimeError):
    pass


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request: urllib.request.Request, file_pointer: Any,
                         code: int, message: str, headers: Any,
                         new_url: str) -> None:
        return None


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def public_source(value: dict[str, Any]) -> dict[str, Any]:
    return dx0_source_role(value)


def _load_driver_source(commit: str) -> tuple[dict[str, Any], list[tuple[str, str]]]:
    """Load the exact committed source and retain bounded output-only dirt."""
    root = repo_root()
    if command_text(["git", "rev-parse", "HEAD"], cwd=root) != commit:
        fail("DX0 worktree HEAD differs from the requested source")
    if command_text(["git", "branch", "--show-current"], cwd=root) != DX0_BRANCH:
        fail("DX0 implementation branch differs")
    raw = command(
        ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"],
        cwd=root,
    ).stdout
    entries: list[tuple[str, str]] = []
    for item in raw.split(b"\0"):
        if not item:
            continue
        text = item.decode("utf-8", "strict")
        if len(text) < 4 or text[2] != " ":
            fail("DX0 source worktree status is malformed")
        entries.append((text[:2], text[3:]))
    return dx0_complete_source(commit), entries


class GitHubAdapter:
    """Pinned private GitHub REST 2026-03-10 transport; token never leaves memory."""

    def __init__(self) -> None:
        token = command(["gh", "auth", "token"], timeout=30.0).stdout.strip()
        if not token or len(token) > 4096:
            fail("DX0_HOST_ARTIFACT_BLOCKED: authenticated GitHub token is unavailable")
        self._token = token.decode("utf-8", "strict")

    def _request(self, method: str, route: str, body: dict[str, Any] | None = None) -> tuple[int, bytes]:
        url = route if route.startswith("https://") else f"https://api.github.com{route}"
        data = None if body is None else json.dumps(body, separators=(",", ":")).encode()
        request = urllib.request.Request(url, data=data, method=method)
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("Authorization", f"Bearer {self._token}")
        request.add_header("X-GitHub-Api-Version", API_VERSION)
        request.add_header("User-Agent", "linux-vst-bridge-dx0")
        if data is not None:
            request.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                payload = response.read(512 * 1024 * 1024 + 1)
                if len(payload) > 512 * 1024 * 1024:
                    fail("GitHub response exceeds DX0 bound")
                return response.status, payload
        except urllib.error.HTTPError as error:
            diagnostic = error.read(4096).decode("utf-8", "replace")
            fail(f"GitHub REST {method} {route} returned {error.code}: {diagnostic}")
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            raise RemoteOutcomeUnknown("GitHub request outcome is unknown") from error

    def json(self, route: str) -> dict[str, Any]:
        status, payload = self._request("GET", route)
        if status != 200:
            fail(f"GitHub REST read returned HTTP {status}")
        value = parse_json_no_duplicates(payload, "GitHub REST response")
        if not isinstance(value, dict):
            fail("GitHub REST read did not return an object")
        return value

    def dispatch(self, *, source_sha: str, build_input_sha256: str,
                 phase_nonce: str) -> dict[str, Any]:
        route = f"/repos/{REPOSITORY}/actions/workflows/{WORKFLOW_API_ID}/dispatches"
        body = {
            "ref": DX0_BRANCH,
            "inputs": {
                "source_sha": source_sha,
                "build_input_sha256": build_input_sha256,
                "host_mode": DX0_HOST_MODE,
                "phase_nonce": phase_nonce,
            },
        }
        status, payload = self._request("POST", route, body)
        if status != 200:
            fail(f"DX0 workflow dispatch returned HTTP {status}, expected 200")
        value = parse_json_no_duplicates(payload, "DX0 workflow dispatch response")
        if (not isinstance(value, dict)
                or set(value) != {"workflow_run_id", "run_url", "html_url"}
                or not isinstance(value["workflow_run_id"], int)
                or value["workflow_run_id"] <= 0
                or not all(isinstance(value[key], str) and value[key].startswith("https://")
                           for key in ("run_url", "html_url"))):
            fail("DX0 workflow dispatch response contract differs")
        return value

    def download(self, route: str, destination: pathlib.Path) -> None:
        if destination.exists() or destination.is_symlink():
            fail("GitHub download destination exists")
        api_url = route if route.startswith("https://") else f"https://api.github.com{route}"
        request = urllib.request.Request(api_url, method="GET")
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("Authorization", f"Bearer {self._token}")
        request.add_header("X-GitHub-Api-Version", API_VERSION)
        request.add_header("User-Agent", "linux-vst-bridge-dx0")
        opener = urllib.request.build_opener(_NoRedirect())
        response: Any
        try:
            response = opener.open(request, timeout=120)
        except urllib.error.HTTPError as error:
            if error.code not in {301, 302, 303, 307, 308}:
                diagnostic = error.read(4096).decode("utf-8", "replace")
                fail(f"GitHub exact download returned HTTP {error.code}: {diagnostic}")
            location = error.headers.get("Location")
            parsed = urllib.parse.urlsplit(location or "")
            if (parsed.scheme != "https" or not parsed.hostname
                    or parsed.username is not None or parsed.password is not None):
                fail("GitHub exact download redirect is unsafe")
            # The signed object URL receives no GitHub Authorization header.
            redirected = urllib.request.Request(location, method="GET", headers={
                "User-Agent": "linux-vst-bridge-dx0",
            })
            try:
                response = urllib.request.urlopen(redirected, timeout=120)
            except (urllib.error.HTTPError, urllib.error.URLError,
                    TimeoutError, OSError) as redirected_error:
                raise RemoteOutcomeUnknown("GitHub object download outcome is unknown") \
                    from redirected_error
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            raise RemoteOutcomeUnknown("GitHub download outcome is unknown") from error
        total = 0
        try:
            with response, destination.open("xb") as output:
                while True:
                    chunk = response.read(1024 * 1024)
                    if not chunk:
                        break
                    total += len(chunk)
                    if total > 512 * 1024 * 1024:
                        fail("GitHub exact download exceeds DX0 bound")
                    output.write(chunk)
                output.flush()
                os.fsync(output.fileno())
            if getattr(response, "status", 200) != 200 or total == 0:
                fail("GitHub exact download returned an empty/non-200 object")
        except Exception:
            if destination.exists():
                destination.unlink()
            raise


class ProofTransactionDriver:
    """Bounded state owner for planning, recovery, remote effects, and rendering."""

    def __init__(self, source_commit: str, plan_id: str, *,
                 github_factory: Callable[[], GitHubAdapter] = GitHubAdapter,
                 proof_root: pathlib.Path | None = None) -> None:
        self.source_commit = source_commit
        self.plan = dx0_closed_plan(plan_id)
        self.plan_sha = dx0_identity_sha256(self.plan)
        self.source, self._initial_status = _load_driver_source(source_commit)
        self.source_role = public_source(self.source)
        self.build_input = dx0_windows_build_input(source_commit)
        self.build_input_sha = dx0_identity_sha256(self.build_input)
        self.renderer = dx0_evidence_renderer(source_commit)
        self.renderer_sha = dx0_identity_sha256(self.renderer)
        self.github_factory = github_factory
        self.proof_root = proof_root or dx0_mac_transaction_parent()
        key = sha256_bytes(canonical_json({
            "source": self.source_role, "plan_sha256": self.plan_sha,
        }))
        self.transaction_key = key
        self.root, self.state = self._open_or_create_transaction()
        self._admit_resumable_published_output()
        self.effect_counts = self.state.setdefault("effect_counts", {
            "windows_builds": 0, "artifact_downloads": 0,
            "custody_operations": 0, "artifact_transfers": 0,
            "source_transfers": 0, "deck_executions": 0,
            "evidence_renders": 0,
        })
        self.phase("derive_identities", "completed", inputs={
            "source_commit": self.source_commit, "proof_plan_sha256": self.plan_sha,
        }, outputs={
            "complete_source_identity_sha256": self.source_role["identity_sha256"],
            "windows_build_input_sha256": self.build_input_sha,
            "evidence_renderer_identity_sha256": self.renderer_sha,
        })
        self.phase("plan_external_work", "completed", inputs={
            "proof_plan": self.plan,
        }, outputs={"proof_plan_sha256": self.plan_sha, "closed": True})

    def _admit_resumable_published_output(self) -> None:
        if not self._initial_status:
            return
        expected = {path for path in DX0_EVIDENCE_PATHS}
        observed = {path for status, path in self._initial_status if status == "??"}
        if (len(observed) != len(self._initial_status)
                or observed != expected
                or self.state.get("state") != "transaction_complete"):
            fail("DX0 source worktree is not clean")
        retained = self.root / "evidence-packet"
        published = repo_root() / "evidence/dx0-split-build-identity-proof-transaction"
        if packet_identity(retained) != packet_identity(published):
            fail("DX0 published completion output differs from retained evidence")
        if any((retained / name).read_bytes() != (published / name).read_bytes()
               for name in sorted(path.name for path in retained.iterdir())):
            fail("DX0 published completion bytes differ from retained evidence")

    def _open_or_create_transaction(self) -> tuple[pathlib.Path, dict[str, Any]]:
        self.proof_root.mkdir(parents=True, exist_ok=True)
        matches: list[tuple[pathlib.Path, dict[str, Any]]] = []
        for candidate in self.proof_root.iterdir():
            state_path = candidate / "DX0_TRANSACTION_STATE.json"
            if candidate.is_dir() and state_path.is_file():
                try:
                    value = parse_json_no_duplicates(
                        state_path.read_bytes(), "DX0 transaction state"
                    )
                except RuntimeError:
                    continue
                if value.get("transaction_key") == self.transaction_key:
                    matches.append((candidate, value))
        if len(matches) > 1:
            fail("DX0_DECK_TRANSACTION_BLOCKED: duplicate transaction ownership exists")
        if matches:
            root, value = matches[0]
            if canonical_json(value) != (root / "DX0_TRANSACTION_STATE.json").read_bytes():
                fail("DX0 transaction state is not canonical")
            self._validate_state(value)
            return root, value
        nonce = os.urandom(16).hex()
        root = self.proof_root / nonce
        root.mkdir(mode=0o700)
        value = {
            "schema": DX0_TRANSACTION_STATE_SCHEMA,
            "operation_nonce": nonce, "transaction_key": self.transaction_key,
            "state": "planned", "source": self.source_role,
            "plan_sha256": self.plan_sha, "created_utc": utc_now(),
            "run_invocation_count": 0,
            "phases": {}, "effect_counts": {
                "windows_builds": 0, "artifact_downloads": 0,
                "custody_operations": 0, "artifact_transfers": 0,
                "source_transfers": 0, "deck_executions": 0,
                "evidence_renders": 0,
            },
        }
        write_atomic(root / "DX0_TRANSACTION_STATE.json", canonical_json(value))
        return root, value

    def _validate_state(self, value: dict[str, Any]) -> None:
        if (set(value) != {"schema", "operation_nonce", "transaction_key", "state",
                           "source", "plan_sha256", "created_utc",
                           "run_invocation_count", "phases", "effect_counts"}
                or value.get("schema") != DX0_TRANSACTION_STATE_SCHEMA
                or not re.fullmatch(r"[0-9a-f]{32}", str(value.get("operation_nonce")))
                or value.get("transaction_key") != self.transaction_key
                or value.get("state") not in DX0_STATES
                or value.get("source") != self.source_role
                or value.get("plan_sha256") != self.plan_sha
                or not isinstance(value.get("created_utc"), str)
                or type(value.get("run_invocation_count")) is not int
                or value["run_invocation_count"] < 0
                or not isinstance(value.get("phases"), dict)
                or not isinstance(value.get("effect_counts"), dict)
                or set(value["effect_counts"]) != EFFECT_COUNT_KEYS
                or any(type(count) is not int or count < 0
                       for count in value["effect_counts"].values())):
            fail("DX0 transaction state contract differs")
        for name, receipt in value["phases"].items():
            if (name not in DX0_OPERATIONS or not isinstance(receipt, dict)
                    or set(receipt) != {"phase", "phase_nonce", "disposition",
                                        "input_sha256", "inputs", "outputs"}
                    or receipt.get("phase") != name
                    or not re.fullmatch(r"[0-9a-f]{32}", str(receipt.get("phase_nonce")))
                    or receipt.get("disposition") not in ALLOWED_DISPOSITIONS
                    or not isinstance(receipt.get("inputs"), dict)
                    or (receipt.get("outputs") is not None
                        and not isinstance(receipt.get("outputs"), dict))
                    or receipt.get("input_sha256")
                    != sha256_bytes(canonical_json(receipt.get("inputs")))):
                fail(f"DX0 transaction phase receipt differs: {name}")

    def record_run_invocation(self) -> None:
        if self.state["state"] == "transaction_complete":
            fail("DX0 completed transaction history is immutable")
        self.state["run_invocation_count"] += 1
        self.save()

    def save(self) -> None:
        write_atomic(self.root / "DX0_TRANSACTION_STATE.json", canonical_json(self.state))

    def set_state(self, state: str) -> None:
        if state not in DX0_STATES:
            fail("DX0 transaction state is outside the closed set")
        current = self.state["state"]
        if DX0_STATE_ORDER.index(state) < DX0_STATE_ORDER.index(current):
            return
        if state == current:
            return
        allowed = {
            "planned": {"source_frozen"},
            "source_frozen": {"fixture_verified"},
            "fixture_verified": {"host_artifact_verified"},
            "host_artifact_verified": {"handoff_admitted",
                                       "transaction_result_retained"},
            "handoff_admitted": {"deck_batch_in_flight",
                                 "transaction_result_retained"},
            "deck_batch_in_flight": {"transaction_result_retained"},
            "transaction_result_retained": {"evidence_rendered"},
            "evidence_rendered": {"transaction_complete"},
        }
        if state not in allowed.get(current, set()):
            fail(f"DX0 transaction state transition is invalid: {current} -> {state}")
        self.state["state"] = state
        self.save()

    def phase(self, name: str, disposition: str, *, inputs: dict[str, Any],
              outputs: dict[str, Any] | None = None, phase_nonce: str | None = None) -> dict[str, Any]:
        if name not in DX0_OPERATIONS or disposition not in ALLOWED_DISPOSITIONS:
            fail("DX0 phase disposition is outside the closed set")
        current = self.state["phases"].get(name)
        nonce = phase_nonce or (current or {}).get("phase_nonce") or os.urandom(16).hex()
        if current and current.get("phase_nonce") != nonce:
            fail("DX0 resume attempted to replace a stable phase nonce")
        receipt = {
            "phase": name, "phase_nonce": nonce, "disposition": disposition,
            "input_sha256": sha256_bytes(canonical_json(inputs)),
            "inputs": inputs, "outputs": outputs,
        }
        if current is not None:
            current_disposition = current["disposition"]
            if current == receipt:
                return current
            if current_disposition in DX0_TERMINAL_PHASE_DISPOSITIONS:
                if current_disposition == "failed":
                    fail(f"DX0 failed phase is immutable: {name}")
                # Completed/reused provenance is immutable. A resume may ask
                # for the same operation through a later reuse path, but it
                # cannot relabel or replace the original receipt.
                return current
            if (disposition == current_disposition
                    or disposition not in DX0_PHASE_TRANSITIONS.get(
                        current_disposition, set())
                    or current["input_sha256"] != receipt["input_sha256"]):
                fail(f"DX0 phase transition is invalid: {name}")
        self.state["phases"][name] = receipt
        self.save()
        return receipt

    @staticmethod
    def acquire_single_writer(parent: pathlib.Path, key: str,
                              intent: dict[str, Any]) -> pathlib.Path:
        parent.mkdir(parents=True, exist_ok=True)
        if not parent.is_dir() or parent.is_symlink():
            fail("DX0 expensive-phase single-writer parent is unsafe")
        lock = parent / key
        try:
            lock.mkdir(mode=0o700)
        except FileExistsError:
            fail("DX0 expensive-phase single-writer outcome is unresolved")
        write_atomic(lock / "prepared-intent.json", canonical_json(intent))
        return lock

    @staticmethod
    def validate_single_writer(lock: pathlib.Path,
                               expected_intent: dict[str, Any]) -> None:
        intent_path = lock / "prepared-intent.json"
        if (not lock.is_dir() or lock.is_symlink()
                or {path.name for path in lock.iterdir()} != {"prepared-intent.json"}
                or not intent_path.is_file() or intent_path.is_symlink()
                or intent_path.read_bytes() != canonical_json(expected_intent)):
            fail("DX0 expensive-phase single-writer intent differs")

    @classmethod
    def retire_single_writer(cls, lock: pathlib.Path, *, publication_valid: bool,
                             expected_intent: dict[str, Any]) -> None:
        if not publication_valid:
            return
        cls.validate_single_writer(lock, expected_intent)
        (lock / "prepared-intent.json").unlink()
        lock.rmdir()

    @classmethod
    def acquire_after_publication_recheck(
            cls, parent: pathlib.Path, key: str, intent: dict[str, Any],
            read_publication: Callable[[], Any | None],
            recover_publication: Callable[[], Any | None] | None = None,
            ) -> tuple[pathlib.Path | None, Any | None, bool]:
        """Reconcile publication and a matching persisted lock before new work."""
        parent.mkdir(parents=True, exist_ok=True)
        if not parent.is_dir() or parent.is_symlink():
            fail("DX0 expensive-phase single-writer parent is unsafe")
        lock = parent / key
        publication = read_publication()
        if lock.exists() or lock.is_symlink():
            cls.validate_single_writer(lock, intent)
            if publication is None and recover_publication is not None:
                publication = recover_publication()
            if publication is None:
                publication = read_publication()
            # The existing writer may still be live. With no complete atomic
            # publication, the caller must block rather than duplicate work.
            return lock, publication, False
        if publication is not None:
            return None, publication, False
        lock = cls.acquire_single_writer(parent, key, intent)
        publication = read_publication()
        if publication is None and recover_publication is not None:
            publication = recover_publication()
        return lock, publication, publication is None

    def _validate_transaction_identity(self, value: Any) -> dict[str, Any]:
        if (not isinstance(value, dict) or set(value) != DX0_TRANSACTION_KEYS
                or value.get("schema") != DX0_TRANSACTION_SCHEMA
                or value.get("operation_nonce") != self.state["operation_nonce"]
                or value.get("evidence_consumer_source") != self.source_role
                or value.get("windows_build_input") != {"sha256": self.build_input_sha}
                or value.get("proof_plan") != {"sha256": self.plan_sha}
                or value.get("evidence_renderer") != {"sha256": self.renderer_sha}
                or value.get("phase_receipts") != self.state["phases"]
                or set(value.get("phase_receipts", {})) != DX0_OPERATIONS
                or any(receipt.get("disposition") not in {"completed", "reused"}
                       for receipt in value.get("phase_receipts", {}).values())):
            fail("DX0 final transaction manifest closure differs")
        return value

    def finalize_transaction(self, value: dict[str, Any], *,
                             _test_fail_after: str | None = None) -> None:
        """Idempotently publish the final identity, then mark state complete."""
        self._validate_transaction_identity(value)
        manifest_path = self.root / "DX0_TRANSACTION.json"
        sidecar = self.root / "DX0_TRANSACTION.sha256"
        data = canonical_json(value)
        if manifest_path.exists() or manifest_path.is_symlink():
            if (not manifest_path.is_file() or manifest_path.is_symlink()
                    or manifest_path.read_bytes() != data):
                fail("DX0 partial final transaction manifest conflicts")
        else:
            if sidecar.exists() or sidecar.is_symlink():
                fail("DX0 final transaction sidecar exists without its manifest")
            write_atomic(manifest_path, data)
        if _test_fail_after == "manifest":
            raise RuntimeError("injected DX0 finalization failure after manifest")
        digest_line = f"{sha256_file(manifest_path)}  DX0_TRANSACTION.json\n".encode()
        if sidecar.exists() or sidecar.is_symlink():
            if (not sidecar.is_file() or sidecar.is_symlink()
                    or sidecar.read_bytes() != digest_line):
                fail("DX0 partial final transaction sidecar conflicts")
        else:
            write_atomic(sidecar, digest_line)
        if _test_fail_after == "sidecar":
            raise RuntimeError("injected DX0 finalization failure after sidecar")
        self.set_state("transaction_complete")

    def completed_result(self) -> dict[str, Any]:
        if self.state["state"] != "transaction_complete":
            fail("DX0 transaction is not complete")
        manifest_path = self.root / "DX0_TRANSACTION.json"
        sidecar = self.root / "DX0_TRANSACTION.sha256"
        if (not manifest_path.is_file() or manifest_path.is_symlink()
                or not sidecar.is_file() or sidecar.is_symlink()):
            fail("DX0 completed transaction manifest is absent or unsafe")
        data = manifest_path.read_bytes()
        value = parse_json_no_duplicates(data, "DX0 final transaction manifest")
        if canonical_json(value) != data:
            fail("DX0 final transaction manifest is not canonical")
        digest = sha256_file(manifest_path)
        if sidecar.read_bytes() != f"{digest}  DX0_TRANSACTION.json\n".encode():
            fail("DX0 final transaction manifest sidecar differs")
        self._validate_transaction_identity(value)
        retained_packet = self.root / "evidence-packet"
        packet = packet_identity(retained_packet)
        packet_sha = value["phase_receipts"]["render_and_validate_evidence"] \
            ["outputs"]["packet_sha256"]
        if packet.get("packet_sha256") != packet_sha:
            fail("DX0 retained private evidence packet differs")
        publish_packet(
            retained_packet,
            repo_root() / "evidence/dx0-split-build-identity-proof-transaction",
            staging_parent=repo_root().parent,
        )
        return {
            "schema": "linux-vst-bridge-dx0-completion/v1",
            "source_commit": self.source_commit,
            "windows_build_input_sha256": self.build_input_sha,
            "host_artifact_manifest_sha256": value["host_artifact"]["manifest_sha256"],
            "deck_execution_input_sha256": value["deck_execution_input"]["identity_sha256"],
            "result_sha256": value["transaction_result"]["sha256"],
            "evidence_packet_sha256": packet_sha,
            "effect_counts": dict(self.state["effect_counts"]),
            "proof_row_count": 14,
        }

    def validate_local(self) -> dict[str, Any]:
        from negative_tests import deterministic_tests

        result = deterministic_tests(repo_root(), source_commit=self.source_commit,
                                     driver_class=ProofTransactionDriver,
                                     result_recovery=_copy_result_to_mac,
                                     store_join_validator=_validate_result_store_joins,
                                     observation_state_validator=
                                         _validate_original_observation_state,
                                     execution_writer_recovery=
                                         _reconcile_deck_execution_writer,
                                     deck_result_recorder=_record_deck_result,
                                     transaction_keys=DX0_TRANSACTION_KEYS,
                                     seed_fixture_acquirer=
                                         _obtain_fixture_for_seed)
        if result.get("proof_row_count") != 14 or result.get("all_passed") is not True:
            fail("DX0 deterministic fourteen-row suite did not pass")
        return result

    def freeze_source(self) -> None:
        dx0_require_frozen_source(self.source_commit, detached=False)
        remote = command_text(["git", "ls-remote", "--heads", "origin", DX0_REF], cwd=repo_root())
        fields = remote.split()
        if fields != [self.source_commit, DX0_REF]:
            fail(f"DX0_SOURCE_FREEZE_BLOCKED: remote source differs: {fields[:1]}")
        self.phase("freeze_source", "completed", inputs={
            "source_identity_sha256": self.source_role["identity_sha256"],
            "remote_ref": DX0_REF,
        }, outputs={"remote_head": self.source_commit})
        self.set_state("source_frozen")

    def verify_fixture(self) -> dict[str, Any]:
        fixture = verify_fixture_store()
        self.phase("verify_fixture", "reused", inputs={
            "bundle_manifest_sha256": DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
        }, outputs={"fixture_identity_sha256": fixture["identity_sha256"]})
        self.set_state("fixture_verified")
        return fixture

    def _matching_host_cache(self) -> dict[str, Any] | None:
        parent = dx0_mac_host_artifact_parent()
        if not parent.exists():
            return None
        matches: list[dict[str, Any]] = []
        for candidate in parent.iterdir():
            if not candidate.is_dir() or candidate.name.startswith("."):
                continue
            try:
                value = verify_host_store(candidate, self.build_input_sha)
            except Exception:
                continue
            matches.append(value)
        if len(matches) > 1:
            fail("DX0_HOST_ARTIFACT_BLOCKED: ambiguous host-artifact cache matches")
        return None if not matches else matches[0]

    def _verify_dispatch_eligibility(self, github: GitHubAdapter) -> dict[str, Any]:
        repository = github.json(f"/repos/{REPOSITORY}")
        workflow = github.json(
            f"/repos/{REPOSITORY}/actions/workflows/{WORKFLOW_API_ID}"
        )
        ref = github.json(
            f"/repos/{REPOSITORY}/git/ref/heads/{urllib.parse.quote(DX0_BRANCH, safe='')}"
        )
        if (repository.get("full_name") != REPOSITORY
                or repository.get("visibility") != "private"
                or workflow.get("path") != WORKFLOW_PATH
                or workflow.get("state") != "active"
                or ref.get("object", {}).get("sha") != self.source_commit):
            fail("DX0_HOST_ARTIFACT_BLOCKED: workflow registration/ref eligibility differs")
        return {"workflow_id": workflow.get("id"), "remote_head": self.source_commit,
                "repository_private": True}

    def _reconcile_run(self, github: GitHubAdapter, phase_nonce: str) -> int:
        route = (f"/repos/{REPOSITORY}/actions/workflows/"
                 f"{WORKFLOW_API_ID}/runs?"
                 f"branch={urllib.parse.quote(DX0_BRANCH)}&event=workflow_dispatch&per_page=100")
        listing = github.json(route)
        matches = [run for run in listing.get("workflow_runs", [])
                   if run.get("head_sha") == self.source_commit
                   and run.get("event") == "workflow_dispatch"
                   and phase_nonce in str(run.get("display_title", ""))]
        if len(matches) != 1:
            fail("DX0_HOST_ARTIFACT_BLOCKED: dispatched remote outcome is unresolved")
        run_id = matches[0].get("id")
        if not isinstance(run_id, int) or run_id <= 0:
            fail("DX0 recovered workflow run ID is malformed")
        return run_id

    @staticmethod
    def _upload_result_from_logs(log_zip: pathlib.Path, phase_nonce: str) -> dict[str, Any]:
        if not log_zip.is_file() or log_zip.stat().st_size > 64 * 1024 * 1024:
            fail("DX0 workflow log archive is absent or oversized")
        matches: list[dict[str, Any]] = []
        total = 0
        with zipfile.ZipFile(log_zip, "r") as archive:
            for info in archive.infolist():
                if info.is_dir() or info.file_size > 16 * 1024 * 1024:
                    continue
                total += info.file_size
                if total > 64 * 1024 * 1024:
                    fail("DX0 workflow log content exceeds bound")
                text = archive.read(info).decode("utf-8", "replace")
                for match in re.finditer(r"DX0_UPLOAD_RESULT=(\{[^\r\n]+\})", text):
                    try:
                        value = parse_json_no_duplicates(
                            match.group(1).encode(), "DX0 upload-action result"
                        )
                    except RuntimeError:
                        continue
                    if value.get("phase_nonce") == phase_nonce:
                        matches.append(value)
        unique = {canonical_json(value) for value in matches}
        if len(unique) != 1:
            fail("DX0 exact upload-action result is absent or ambiguous")
        value = parse_json_no_duplicates(next(iter(unique)), "DX0 upload-action result")
        expected_keys = {"artifact_id", "artifact_name", "artifact_human_url",
                         "artifact_digest_bare", "artifact_digest_rest_form", "phase_nonce"}
        if (set(value) != expected_keys
                or not isinstance(value.get("artifact_id"), int)
                or value["artifact_id"] <= 0
                or not re.fullmatch(r"[0-9a-f]{64}", str(value.get("artifact_digest_bare")))
                or value.get("artifact_digest_rest_form")
                != f"sha256:{value.get('artifact_digest_bare')}"):
            fail("DX0 upload-action result key roster differs")
        return value

    def _validate_run(self, run: dict[str, Any], phase_nonce: str) -> dict[str, Any]:
        workflow_blob = command_text(
            ["git", "rev-parse", f"{self.source_commit}:{WORKFLOW_PATH}"], cwd=repo_root()
        )
        run_path = str(run.get("path", "")).split("@", 1)[0]
        expected_title = f"DX0 host {self.build_input_sha} nonce {phase_nonce}"
        if (run.get("event") != "workflow_dispatch"
                or run.get("head_branch") != DX0_BRANCH
                or run.get("head_sha") != self.source_commit
                or run_path != WORKFLOW_PATH
                or run.get("display_title") != expected_title
                or run.get("repository", {}).get("full_name") != REPOSITORY
                or not isinstance(run.get("id"), int)
                or run.get("id") <= 0
                or not isinstance(run.get("run_attempt"), int)
                or run.get("run_attempt") <= 0):
            fail("DX0 exact workflow run metadata differs")
        return {"id": run["id"], "run_attempt": run["run_attempt"],
                "workflow_blob": workflow_blob}

    def _read_exact_run(self, github: GitHubAdapter, run_id: int,
                        phase_nonce: str, *, attempts: int = 30,
                        delay_seconds: float = 2.0) -> tuple[dict[str, Any], dict[str, Any]]:
        """Allow bounded API-field settling for one already-identified run only."""
        if type(run_id) is not int or run_id <= 0 or attempts < 1:
            fail("DX0 exact workflow run read parameters are malformed")
        last_error: RuntimeError | None = None
        for attempt in range(attempts):
            run = github.json(f"/repos/{REPOSITORY}/actions/runs/{run_id}")
            try:
                return run, self._validate_run(run, phase_nonce)
            except RuntimeError as error:
                last_error = error
                if attempt + 1 < attempts:
                    time.sleep(delay_seconds)
        assert last_error is not None
        raise last_error

    def _wait_for_run(self, github: GitHubAdapter, run_id: int,
                      phase_nonce: str, *, timeout_seconds: float = 6000.0,
                      poll_seconds: float = 15.0,
                      clock: Callable[[], float] = time.monotonic,
                      sleeper: Callable[[float], None] = time.sleep,
                      ) -> tuple[dict[str, Any], dict[str, Any]] | None:
        """Wait within the workflow's 90-minute limit plus a bounded margin."""
        if timeout_seconds <= 0 or poll_seconds <= 0:
            fail("DX0 workflow completion wait parameters are malformed")
        deadline = clock() + timeout_seconds
        while True:
            run_object, run_identity = self._read_exact_run(
                github, run_id, phase_nonce
            )
            if run_object.get("status") == "completed":
                return run_object, run_identity
            remaining = deadline - clock()
            if remaining <= 0:
                return None
            sleeper(min(poll_seconds, remaining))

    def _retain_cached_host(
            self, cached: dict[str, Any], *,
            lock_parent: pathlib.Path | None = None) -> dict[str, Any]:
        """Admit one cache publication before retiring its producer lock."""
        parent = lock_parent or (dx0_mac_host_artifact_parent() / ".locks")
        lock = parent / f"{self.build_input_sha}-{DX0_HOST_MODE}"
        intent: dict[str, Any] | None = None
        if lock.exists() or lock.is_symlink():
            intent_path = lock / "prepared-intent.json"
            if not lock.is_dir() or lock.is_symlink() or not intent_path.is_file():
                fail("DX0_HOST_ARTIFACT_BLOCKED: stale host lock is unsafe")
            try:
                intent = parse_json_no_duplicates(
                    intent_path.read_bytes(), "DX0 stale host intent"
                )
            except RuntimeError:
                fail("DX0_HOST_ARTIFACT_BLOCKED: stale host intent is malformed")
            expected_intent_keys = {
                "windows_build_input_sha256", "host_mode", "source_sha",
                "phase_nonce", "operation_nonce",
            }
            producer_commit = cached["custody"]["producer_source"]["commit"]
            if (canonical_json(intent) != intent_path.read_bytes()
                    or set(intent) != expected_intent_keys
                    or intent.get("windows_build_input_sha256") != self.build_input_sha
                    or intent.get("host_mode") != DX0_HOST_MODE
                    or intent.get("source_sha")
                    not in {self.source_commit, producer_commit}
                    or not re.fullmatch(
                        r"[0-9a-f]{32}", str(intent.get("phase_nonce")))
                    or not re.fullmatch(
                        r"[0-9a-f]{32}", str(intent.get("operation_nonce")))):
                fail("DX0_HOST_ARTIFACT_BLOCKED: stale host intent differs")

        workflow = cached["build_receipt"]["workflow"]
        artifact_id = cached["custody"]["artifact"]["id"]
        host_outputs = {
            "run_id": workflow["run_id"],
            "run_attempt": workflow["run_attempt"],
            "artifact_id": artifact_id,
            "host_artifact_manifest_sha256": cached["manifest_sha256"],
        }
        custody_inputs = {
            "run_id": workflow["run_id"], "artifact_id": artifact_id,
            "windows_build_input_sha256": self.build_input_sha,
        }
        custody_outputs = {
            "host_artifact_manifest_sha256": cached["manifest_sha256"],
            "mac_custody_receipt_sha256": cached["custody_sha256"],
        }
        phase = self.state["phases"].get("reuse_or_produce_host")
        recovering_current_phase = (
            phase is not None
            and phase["disposition"] not in DX0_TERMINAL_PHASE_DISPOSITIONS
        )
        if recovering_current_phase:
            if (intent is None or phase.get("inputs") != intent
                    or phase.get("phase_nonce") != intent["phase_nonce"]
                    or intent.get("operation_nonce")
                    != self.state["operation_nonce"]):
                fail("DX0_HOST_ARTIFACT_BLOCKED: recovered host phase intent differs")
            self.phase(
                "reuse_or_produce_host", "completed", inputs=phase["inputs"],
                outputs=host_outputs, phase_nonce=phase["phase_nonce"],
            )
        elif phase is None:
            self.phase("reuse_or_produce_host", "reused", inputs={
                "windows_build_input_sha256": self.build_input_sha,
                "host_mode": DX0_HOST_MODE,
            }, outputs={
                "host_artifact_manifest_sha256": cached["manifest_sha256"],
            })
        elif phase["disposition"] == "completed":
            if phase.get("outputs") != host_outputs:
                fail("DX0_HOST_ARTIFACT_BLOCKED: completed host phase differs")
        elif (phase["disposition"] != "reused"
              or phase.get("outputs") != {
                  "host_artifact_manifest_sha256": cached["manifest_sha256"],
              }):
            fail("DX0_HOST_ARTIFACT_BLOCKED: reused host phase differs")

        custody_phase = self.state["phases"].get("custody_host_artifact")
        if custody_phase is None:
            self.phase(
                "custody_host_artifact",
                "completed" if recovering_current_phase else "reused",
                inputs=(custody_inputs if recovering_current_phase else {
                    "host_artifact_manifest_sha256": cached["manifest_sha256"],
                }),
                outputs=custody_outputs,
            )
        elif custody_phase["disposition"] == "completed":
            if (custody_phase.get("inputs") != custody_inputs
                    or custody_phase.get("outputs") != custody_outputs):
                fail("DX0_HOST_ARTIFACT_BLOCKED: completed custody phase differs")
        elif (custody_phase["disposition"] != "reused"
              or custody_phase.get("inputs") != {
                  "host_artifact_manifest_sha256": cached["manifest_sha256"],
              }
              or custody_phase.get("outputs") != custody_outputs):
            fail("DX0_HOST_ARTIFACT_BLOCKED: reused custody phase differs")

        self.set_state("host_artifact_verified")
        if intent is not None:
            self.retire_single_writer(
                lock, publication_valid=True, expected_intent=intent
            )
        return cached

    def reuse_or_produce_host(self) -> dict[str, Any]:
        cached = self._matching_host_cache()
        if cached is not None:
            return self._retain_cached_host(cached)

        github = self.github_factory()
        eligibility = self._verify_dispatch_eligibility(github)
        phase = self.state["phases"].get("reuse_or_produce_host")
        phase_nonce = (phase or {}).get("phase_nonce") or os.urandom(16).hex()
        intent = {"windows_build_input_sha256": self.build_input_sha,
                  "host_mode": DX0_HOST_MODE, "source_sha": self.source_commit,
                  "phase_nonce": phase_nonce, "operation_nonce": self.state["operation_nonce"]}
        lock_parent = dx0_mac_host_artifact_parent() / ".locks"
        lock_key = f"{self.build_input_sha}-{DX0_HOST_MODE}"
        lock = lock_parent / lock_key
        prepared_here = False
        if phase is None:
            lock, late_cache, may_start = self.acquire_after_publication_recheck(
                lock_parent, lock_key, intent, self._matching_host_cache
            )
            if late_cache is not None:
                return self._retain_cached_host(late_cache)
            assert lock is not None
            self.phase("reuse_or_produce_host", "prepared", inputs=intent,
                       outputs={"eligibility": eligibility}, phase_nonce=phase_nonce)
            prepared_here = may_start
        elif not lock.is_dir():
            fail("DX0_HOST_ARTIFACT_BLOCKED: persisted host intent has no single-writer lock")
        else:
            prepared_path = lock / "prepared-intent.json"
            if (phase.get("inputs") != intent or not prepared_path.is_file()
                    or prepared_path.is_symlink()
                    or prepared_path.read_bytes() != canonical_json(intent)):
                fail("DX0_HOST_ARTIFACT_BLOCKED: persisted host intent/lock differs")

        run_id = (self.state["phases"]["reuse_or_produce_host"].get("outputs") or {}).get("run_id")
        if run_id is None:
            if not prepared_here:
                run_id = self._reconcile_run(github, phase_nonce)
                disposition = "recovered_in_flight"
            else:
                # Reserve the one producer before crossing the remote boundary.
                # A crash after this durable write can only reconcile; it can
                # never redispatch from an ambiguous prepared intent.
                self.effect_counts["windows_builds"] += 1
                self.save()
                try:
                    dispatched = github.dispatch(source_sha=self.source_commit,
                                                 build_input_sha256=self.build_input_sha,
                                                 phase_nonce=phase_nonce)
                    run_id = dispatched["workflow_run_id"]
                    disposition = "in_flight"
                    dispatch_output = {
                        "run_id": run_id, "run_url": dispatched["run_url"],
                        "html_url": dispatched["html_url"],
                    }
                except RemoteOutcomeUnknown:
                    # The request may have been accepted even when its response was lost.
                    run_id = self._reconcile_run(github, phase_nonce)
                    disposition = "recovered_in_flight"
                    dispatch_output = {"run_id": run_id, "response": "lost_reconciled"}
            self.phase("reuse_or_produce_host", disposition, inputs=intent,
                       outputs=(dispatch_output if prepared_here else {
                           "run_id": run_id, "response": "persisted_intent_reconciled",
                       }), phase_nonce=phase_nonce)

        completed_run = self._wait_for_run(github, run_id, phase_nonce)
        if completed_run is None:
            self.phase("reuse_or_produce_host", "outcome_unresolved", inputs=intent,
                       outputs={"run_id": run_id,
                                "reason": "bounded_completion_wait_expired"},
                       phase_nonce=phase_nonce)
            fail("DX0_HOST_ARTIFACT_BLOCKED: workflow completion outcome is unresolved")
        run_object, run_identity = completed_run
        if run_object.get("conclusion") != "success":
            self.phase("reuse_or_produce_host", "failed", inputs=intent,
                       outputs={"run_id": run_id, "conclusion": run_object.get("conclusion")},
                       phase_nonce=phase_nonce)
            fail("DX0_HOST_ARTIFACT_BLOCKED: the one Windows acceptance producer failed")
        listing = github.json(f"/repos/{REPOSITORY}/actions/runs/{run_id}/artifacts?per_page=100")
        entries = listing.get("artifacts")
        if listing.get("total_count") != 1 or not isinstance(entries, list) or len(entries) != 1:
            fail("DX0 host workflow did not publish one exact artifact")
        artifact_id = entries[0].get("id")
        if not isinstance(artifact_id, int) or artifact_id <= 0:
            fail("DX0 host artifact ID is malformed")
        artifact = github.json(f"/repos/{REPOSITORY}/actions/artifacts/{artifact_id}")
        if artifact != entries[0]:
            fail("DX0 run artifact list and exact artifact object differ")
        with tempfile.TemporaryDirectory(prefix="dx0-actions-") as temporary:
            stage = pathlib.Path(temporary)
            logs = stage / "logs.zip"
            github.download(f"/repos/{REPOSITORY}/actions/runs/{run_id}/logs", logs)
            upload = self._upload_result_from_logs(logs, phase_nonce)
            wrapper = stage / "artifact.zip"
            github.download(f"/repos/{REPOSITORY}/actions/artifacts/{artifact_id}/zip", wrapper)
            self.effect_counts["artifact_downloads"] += 1
            host = publish_host_custody(
                wrapper, run=run_identity, artifact=artifact, upload_result=upload,
                producer_source=self.source_role, build_input=self.build_input,
            )
            self.effect_counts["custody_operations"] += 1
            self.phase("custody_host_artifact", "completed", inputs={
                "run_id": run_id, "artifact_id": artifact_id,
                "windows_build_input_sha256": self.build_input_sha,
            }, outputs={
                "host_artifact_manifest_sha256": host["manifest_sha256"],
                "mac_custody_receipt_sha256": host["custody_sha256"],
            })
        self.phase("reuse_or_produce_host", "completed", inputs=intent,
                   outputs={"run_id": run_id, "run_attempt": run_identity["run_attempt"],
                            "artifact_id": artifact_id,
                            "host_artifact_manifest_sha256": host["manifest_sha256"]},
                   phase_nonce=phase_nonce)
        self.set_state("host_artifact_verified")
        self.retire_single_writer(
            lock, publication_valid=True, expected_intent=intent
        )
        return host


class SSHAdapter:
    """Private Mac-to-Deck lane with exact fixture probing and no agent forwarding."""

    def __init__(self) -> None:
        self.destination = self._discover()

    @staticmethod
    def _tailscale_binary() -> pathlib.Path:
        path = pathlib.Path("/Applications/Tailscale.app/Contents/MacOS/Tailscale")
        if not path.is_file():
            fail("DX0_HANDOFF_BLOCKED: established Tailscale client is unavailable")
        return path

    def _discover(self) -> str:
        result = command([str(self._tailscale_binary()), "status", "--json"], timeout=30.0)
        status = parse_json_no_duplicates(result.stdout, "Tailscale status")
        candidates: list[str] = []
        for peer in (status.get("Peer") or {}).values():
            if (peer.get("Online") is not True
                    or str(peer.get("OS", "")).lower() != "linux"
                    or peer.get("HostName") != "steamdeck"):
                continue
            values = peer.get("TailscaleIPs") or []
            if values and isinstance(values[0], str):
                candidates.append(f"deck@{values[0]}")
        matches: list[str] = []
        probe = "test \"$(cat /sys/devices/virtual/dmi/id/product_name)\" = Galileo"
        for destination in candidates:
            result = subprocess.run(
                ["ssh", "-o", "ForwardAgent=no", "-o", "BatchMode=yes",
                 "-o", "StrictHostKeyChecking=yes", "-o", "ConnectTimeout=5",
                 destination, probe],
                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL, timeout=10, check=False,
            )
            if result.returncode == 0:
                matches.append(destination)
        if len(matches) != 1:
            fail("DX0_HANDOFF_BLOCKED: exact online Steam Deck SSH destination is absent or ambiguous")
        return matches[0]

    def run(self, script: str, *, timeout: float = 120.0) -> bytes:
        result = subprocess.run(
            ["ssh", "-o", "ForwardAgent=no", "-o", "BatchMode=yes",
             "-o", "StrictHostKeyChecking=yes", self.destination, script],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            timeout=timeout, check=False,
        )
        if result.returncode != 0:
            fail(f"DX0_HANDOFF_BLOCKED: private Deck command failed ({result.returncode})")
        return result.stdout

    def copy(self, source: pathlib.Path, destination: str, *, timeout: float = 300.0) -> None:
        result = subprocess.run(
            ["scp", "-q", "-o", "ForwardAgent=no", "-o", "BatchMode=yes",
             "-o", "StrictHostKeyChecking=yes", str(source), f"{self.destination}:{destination}"],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            timeout=timeout, check=False,
        )
        if result.returncode != 0:
            fail("DX0_HANDOFF_BLOCKED: private Deck file transfer failed")

    def fetch(self, source: str, destination: pathlib.Path, *, timeout: float = 300.0) -> None:
        if destination.exists() or destination.is_symlink():
            fail("DX0 result retrieval destination exists")
        result = subprocess.run(
            ["scp", "-q", "-o", "ForwardAgent=no", "-o", "BatchMode=yes",
             "-o", "StrictHostKeyChecking=yes", f"{self.destination}:{source}", str(destination)],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            timeout=timeout, check=False,
        )
        if result.returncode != 0:
            fail("DX0_DECK_TRANSACTION_BLOCKED: result retrieval failed")

    def publish_tree(self, local: pathlib.Path, remote_parent: str, name: str) -> str:
        if not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", name):
            fail("DX0_HANDOFF_BLOCKED: publication identity is malformed")
        objects = sorted(local.rglob("*"),
                         key=lambda item: item.relative_to(local).as_posix().encode())
        if (not local.is_dir() or local.is_symlink()
                or any(path.is_symlink() or not (path.is_file() or path.is_dir())
                       for path in objects)):
            fail("DX0_HANDOFF_BLOCKED: local publication tree is unsafe")
        records = [path for path in objects if path.is_file()]
        if not records:
            fail("DX0_HANDOFF_BLOCKED: local publication tree is empty")
        target = f"{remote_parent}/{name}"
        exists = self.run(f"if test -e {shlex.quote(target)}; then printf present; else printf absent; fi").decode()
        if exists == "present":
            self.run(f"test -d {shlex.quote(target)} && test ! -L {shlex.quote(target)}")
            unsafe = self.run(
                f"cd {shlex.quote(target)} && find . ! -type d ! -type f -print -quit"
            ).decode()
            if unsafe:
                fail("DX0_HANDOFF_BLOCKED: existing Deck store contains an unsafe object")
            observed_names = self.run(
                f"cd {shlex.quote(target)} && find . -type f -print | sed 's#^./##' | LC_ALL=C sort"
            ).decode().splitlines()
            expected_names = [path.relative_to(local).as_posix() for path in records]
            if observed_names != expected_names:
                fail("DX0_HANDOFF_BLOCKED: existing Deck store roster differs")
            for path in records:
                relative = path.relative_to(local).as_posix()
                remote = f"{target}/{relative}"
                observed = self.run(f"sha256sum {shlex.quote(remote)} | cut -d' ' -f1").decode().strip()
                if observed != sha256_file(path):
                    fail("DX0_HANDOFF_BLOCKED: existing Deck store differs")
            return "reused"
        stage = f"{remote_parent}/.dx0-stage-{name}"
        self.run(f"test ! -e {shlex.quote(stage)} && mkdir -p {shlex.quote(stage)}")
        for path in records:
            relative = path.relative_to(local).as_posix()
            remote = f"{stage}/{relative}"
            self.run(f"mkdir -p {shlex.quote(str(pathlib.PurePosixPath(remote).parent))}")
            self.copy(path, remote)
            observed = self.run(f"sha256sum {shlex.quote(remote)} | cut -d' ' -f1").decode().strip()
            if observed != sha256_file(path):
                fail("DX0_HANDOFF_BLOCKED: Deck transfer byte equality failed")
        self.run(
            f"find {shlex.quote(stage)} -type f -exec chmod 0444 {{}} + && "
            f"find {shlex.quote(stage)} -type d -exec chmod 0555 {{}} + && "
            f"test ! -e {shlex.quote(target)} && mv {shlex.quote(stage)} {shlex.quote(target)}"
        )
        return "transferred"

    def verify_tree(self, local: pathlib.Path, remote_parent: str, name: str) -> None:
        target = f"{remote_parent}/{name}"
        objects = sorted(local.rglob("*"),
                         key=lambda item: item.relative_to(local).as_posix().encode())
        if (not local.is_dir() or local.is_symlink()
                or any(path.is_symlink() or not (path.is_file() or path.is_dir())
                       for path in objects)):
            fail("DX0_ACCEPTED_FIXTURE_BLOCKED: local fixture store is unsafe")
        records = [path for path in objects if path.is_file()]
        unsafe = self.run(
            f"test -d {shlex.quote(target)} && test ! -L {shlex.quote(target)} && "
            f"cd {shlex.quote(target)} && "
            "find . ! -type d ! -type f -print -quit"
        ).decode()
        if unsafe:
            fail("DX0_ACCEPTED_FIXTURE_BLOCKED: Deck fixture store contains an unsafe object")
        observed_names = self.run(
            f"test -d {shlex.quote(target)} && cd {shlex.quote(target)} && "
            "find . -type f -print | sed 's#^./##' | LC_ALL=C sort"
        ).decode().splitlines()
        expected_names = [path.relative_to(local).as_posix() for path in records]
        if observed_names != expected_names:
            fail("DX0_ACCEPTED_FIXTURE_BLOCKED: Deck fixture roster differs")
        for path in records:
            remote = f"{target}/{path.relative_to(local).as_posix()}"
            observed = self.run(f"sha256sum {shlex.quote(remote)} | cut -d' ' -f1").decode().strip()
            if observed != sha256_file(path):
                fail("DX0_ACCEPTED_FIXTURE_BLOCKED: Deck fixture bytes differ")


def _copy_result_to_mac(ssh: SSHAdapter, execution_input_sha: str,
                        plan_sha: str, *,
                        result_parent: pathlib.Path | None = None) -> dict[str, Any] | None:
    remote_root = ("/home/deck/.local/share/linux-vst-bridge/proof/results/"
                   f"by-execution-input/{execution_input_sha}")
    remote_result = remote_root + "/DX0_TRANSACTION_RESULT.json"
    remote_sidecar = remote_root + "/DX0_TRANSACTION_RESULT.json.sha256"
    exists = ssh.run(
        f"if test -d {shlex.quote(remote_root)} && test ! -L {shlex.quote(remote_root)} && "
        f"test \"$(readlink -f {shlex.quote(remote_root)})\" = {shlex.quote(remote_root)} && "
        f"test -f {shlex.quote(remote_result)} && test ! -L {shlex.quote(remote_result)} && "
        f"test -f {shlex.quote(remote_sidecar)} && test ! -L {shlex.quote(remote_sidecar)}; "
        f"then printf present; elif test -e {shlex.quote(remote_root)} || "
        f"test -L {shlex.quote(remote_root)}; then printf unsafe; else printf absent; fi"
    ).decode()
    if exists == "unsafe":
        fail("DX0_DECK_TRANSACTION_BLOCKED: retained Deck result boundary is unsafe")
    if exists != "present":
        return None
    local_parent = result_parent or dx0_mac_result_parent()
    if local_parent.is_symlink():
        fail("DX0_DECK_TRANSACTION_BLOCKED: Mac result parent is a symlink")
    target = local_parent / execution_input_sha
    if target.is_symlink():
        fail("DX0_DECK_TRANSACTION_BLOCKED: retained Mac result root is a symlink")
    if target.exists():
        if not target.is_dir():
            fail("DX0_DECK_TRANSACTION_BLOCKED: retained Mac result root is unsafe")
        result = validate_result_file(target / "DX0_TRANSACTION_RESULT.json",
                                      expected_execution_input_sha256=execution_input_sha,
                                      expected_plan_sha256=plan_sha)
        return {"result": result, "path": target / "DX0_TRANSACTION_RESULT.json",
                "disposition": "reused"}
    parent = local_parent
    parent.mkdir(parents=True, exist_ok=True)
    if not parent.is_dir() or parent.is_symlink():
        fail("DX0_DECK_TRANSACTION_BLOCKED: Mac result parent is unsafe")
    stage = parent / f".dx0-result-retrieval-{execution_input_sha}"
    expected_stage_names = {
        "DX0_TRANSACTION_RESULT.json", "DX0_TRANSACTION_RESULT.json.sha256"
    }
    if stage.exists() or stage.is_symlink():
        if (not stage.is_dir() or stage.is_symlink()
                or any(not path.is_file() or path.is_symlink()
                       for path in stage.iterdir())
                or not {path.name for path in stage.iterdir()}
                    .issubset(expected_stage_names)):
            fail("DX0_DECK_TRANSACTION_BLOCKED: result retrieval stage is unsafe")
        observed_names = {path.name for path in stage.iterdir()}
        if observed_names == expected_stage_names:
            result = validate_result_file(
                stage / "DX0_TRANSACTION_RESULT.json",
                expected_execution_input_sha256=execution_input_sha,
                expected_plan_sha256=plan_sha,
            )
            os.replace(stage, target)
            return {"result": result,
                    "path": target / "DX0_TRANSACTION_RESULT.json",
                    "disposition": "recovered_complete_stage"}
        quarantine = parent / (
            f"{stage.name}.partial-{os.urandom(8).hex()}"
        )
        if quarantine.exists() or quarantine.is_symlink():
            fail("DX0_DECK_TRANSACTION_BLOCKED: partial retrieval quarantine exists")
        os.rename(stage, quarantine)
    stage.mkdir(mode=0o700)
    ssh.fetch(remote_root + "/DX0_TRANSACTION_RESULT.json",
              stage / "DX0_TRANSACTION_RESULT.json")
    ssh.fetch(remote_root + "/DX0_TRANSACTION_RESULT.json.sha256",
              stage / "DX0_TRANSACTION_RESULT.json.sha256")
    result = validate_result_file(stage / "DX0_TRANSACTION_RESULT.json",
                                  expected_execution_input_sha256=execution_input_sha,
                                  expected_plan_sha256=plan_sha)
    os.replace(stage, target)
    return {"result": result, "path": target / "DX0_TRANSACTION_RESULT.json",
            "disposition": "retrieved"}


def _public_phase_dispositions(state: dict[str, Any]) -> dict[str, str]:
    return {name: value["disposition"] for name, value in sorted(state["phases"].items())}


def _validate_result_store_joins(result: dict[str, Any], host: dict[str, Any],
                                 fixture: dict[str, Any], *,
                                 verified_handoff: dict[str, Any] | None = None) -> None:
    """Join a retained observation to the independently admitted private stores."""
    build = host["build_receipt"]
    custody = host["custody"]
    expected_host = {
        "windows_build_input_sha256":
            build["windows_build_input"]["sha256"],
        "workflow_run_id": build["workflow"]["run_id"],
        "run_attempt": build["workflow"]["run_attempt"],
        "artifact_id": custody["artifact"]["id"],
        "manifest_sha256": host["manifest_sha256"],
        "build_receipt_sha256": host["build_receipt_sha256"],
        "mac_custody_receipt_sha256": host["custody_sha256"],
    }
    expected_fixture = {
        "identity_sha256": fixture["identity_sha256"],
        "bundle_manifest_sha256": DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
        "module_sha256": next(
            item["sha256"]
            for item in fixture["identity"]["bundle_manifest"]["records"]
            if item["path"] == "Contents/x86_64-win/again.vst3"
        ),
        "mac_store_receipt_sha256": fixture["receipt_sha256"],
        "deck_store_receipt_sha256": fixture["receipt_sha256"],
    }
    expected_ref = (
        "refs/handoff/dx0-source/" + result["deck_execution_source"]["commit"]
    )
    if verified_handoff is None:
        handoff_stage = (dx0_mac_transaction_parent() / result["operation_nonce"]
                         / "source-handoff")
        verified_handoff = verify_source_handoff(
            handoff_stage, result["deck_execution_source"]["commit"]
        )
    handoff_receipt = verified_handoff["receipt"]
    bundle = handoff_receipt["bundle"]
    expected_handoff = {
        "bundle_sha256": bundle["sha256"],
        "receipt_sha256": verified_handoff["receipt_sha256"],
        "advertised_ref": bundle["advertised_ref"],
        "worktree_commit": result["deck_execution_source"]["commit"],
        "worktree_clean": True,
    }
    if (result["artifact_producer_source"] != custody["producer_source"]
            or result["deck_execution_source"]
            != handoff_receipt["implementation_source"]
            or result["host_artifact"] != expected_host
            or result["accepted_fixture"] != expected_fixture
            or result["source_handoff"] != expected_handoff
            or bundle["advertised_ref"] != expected_ref
            or result["execution_input"]["source_handoff_ref"] != expected_ref
            or result["execution_input"]["detached_worktree_commit"]
            != result["deck_execution_source"]["commit"]
            or result["execution_input"]["detached_worktree_commit"]
            != handoff_receipt["implementation_source"]["commit"]):
        fail("DX0 retained result/private-store identity closure differs")


def _proof_rows(deterministic: dict[str, Any], *,
                original_observation_transaction: dict[str, Any],
                result: dict[str, Any]) -> list[dict[str, Any]]:
    descriptions = [
        "authority, ten-record source roster, and closed classification",
        "historical sources reproduce one Windows build-input identity",
        "renderer-only mutation reuses P/E and performs zero external work",
        "Mac-driver-only mutation reuses host, fixture, and result",
        "Deck mutation changes execution but not Windows build identity",
        "host mutation invalidates the host artifact",
        "accepted AGain is manifest-admitted and never compiled",
        "distinct producer, execution, and consumer sources retain exact joins",
        "eligibility and freeze precede one pinned dispatch",
        "closed plan rejects commands, paths, hooks, and extra keys",
        "one Mac command completes the live positive proof",
        "lost acknowledgements and duplicate drivers start no duplicate work",
        "live supervision reaches zero residue and protected-state equality",
        "typed result admission rejects incomplete or failed cached facts",
    ]
    counts = original_observation_transaction.get("effect_counts")
    ledger_disposition = original_observation_transaction.get("ledger_disposition")
    expected_phase_count = {
        "complete": len(DX0_OPERATIONS),
        "result_retained_before_consumer_render": len(DX0_OPERATIONS) - 2,
        "evidence_rendered_before_consumer_closure": len(DX0_OPERATIONS) - 1,
    }.get(ledger_disposition)
    expected_evidence_renders = {
        "complete": 1,
        "result_retained_before_consumer_render": 0,
        "evidence_rendered_before_consumer_closure": 1,
    }.get(ledger_disposition)
    if (deterministic.get("all_passed") is not True
            or deterministic.get("proof_row_count") != 14
            or original_observation_transaction.get("retained_private_state_available")
            is not True
            or original_observation_transaction.get("manual_commands") != 1
            or expected_phase_count is None
            or original_observation_transaction.get("phase_count")
            != expected_phase_count
            or original_observation_transaction.get("execution_phase_join_valid") is not True
            or not isinstance(counts, dict)
            or counts.get("deck_executions") != 1
            or counts.get("evidence_renders") != expected_evidence_renders
            or result["cleanup"] != {
                "owned_descendant_count": 0,
                "process_group_empty": True,
                "environment_retired": True,
                "stage_absent": True,
            }
            or result["protected_state"]["equal"] is not True
            or result["protected_state"]["comparison_completed"] is not True):
        fail("DX0_EVIDENCE_BLOCKED: one-command live proof ledger differs")
    synthetic = set(range(1, 11)) | {12, 14}
    return [
        {"row": index, "result": "PASS",
         "validation": "deterministic" if index in synthetic else "live",
         "claim": description}
        for index, description in enumerate(descriptions, 1)
    ]


def _fixture_seeding_receipt() -> dict[str, Any]:
    path = dx0_mac_transaction_parent().parent / "fixture-seeding/DX0_FIXTURE_SEEDING_RECEIPT.json"
    if not path.is_file():
        return {"performed_separately": False, "ordinary_run_seeded_fixture": False}
    value = parse_json_no_duplicates(path.read_bytes(), "DX0 fixture-seeding receipt")
    if canonical_json(value) != path.read_bytes():
        fail("DX0 fixture-seeding receipt is not canonical")
    return {"performed_separately": True, "ordinary_run_seeded_fixture": False,
            "mac_disposition": value["mac_disposition"],
            "deck_disposition": value["deck_disposition"],
            "artifact_id": value["artifact_id"]}


def _validate_original_observation_state(
        result: dict[str, Any], value: dict[str, Any], *,
        require_complete: bool = True) -> dict[str, Any]:
    """Validate the immutable transaction ledger that produced observation E."""
    result_sha = sha256_bytes(canonical_json(result))
    expected_key = sha256_bytes(canonical_json({
        "source": result["deck_execution_source"],
        "plan_sha256": result["closed_plan"]["sha256"],
    }))
    counts = value.get("effect_counts")
    phases = value.get("phases")
    expected_state_keys = {
        "schema", "operation_nonce", "transaction_key", "state", "source",
        "plan_sha256", "created_utc", "run_invocation_count", "phases",
        "effect_counts",
    }
    state = value.get("state")
    if require_complete:
        expected_phases = DX0_OPERATIONS
        expected_evidence_renders = 1
        ledger_disposition = "complete"
    elif state == "transaction_result_retained":
        expected_phases = DX0_OPERATIONS - {
            "render_and_validate_evidence", "close_transaction",
        }
        expected_evidence_renders = 0
        ledger_disposition = "result_retained_before_consumer_render"
    elif state == "evidence_rendered":
        expected_phases = DX0_OPERATIONS - {"close_transaction"}
        expected_evidence_renders = 1
        ledger_disposition = "evidence_rendered_before_consumer_closure"
    else:
        fail("DX0 original transaction state differs")
    if (set(value) != expected_state_keys
            or value.get("schema") != DX0_TRANSACTION_STATE_SCHEMA
            or value.get("operation_nonce") != result["operation_nonce"]
            or value.get("transaction_key") != expected_key
            or value.get("source") != result["deck_execution_source"]
            or value.get("plan_sha256") != result["closed_plan"]["sha256"]
            or not isinstance(value.get("created_utc"), str)
            or type(value.get("run_invocation_count")) is not int
            or value["run_invocation_count"] != 1
            or not isinstance(phases, dict)
            or set(phases) != expected_phases
            or not isinstance(counts, dict) or set(counts) != EFFECT_COUNT_KEYS
            or any(type(count) is not int or count < 0 for count in counts.values())
            or counts["deck_executions"] != 1
            or counts["evidence_renders"] != expected_evidence_renders):
        fail("DX0 original transaction effect ledger differs")
    if require_complete and value.get("state") != "transaction_complete":
        fail("DX0 original transaction is not durably complete")
    for name, receipt in phases.items():
        if (not isinstance(receipt, dict)
                or set(receipt) != {"phase", "phase_nonce", "disposition",
                                    "input_sha256", "inputs", "outputs"}
                or receipt.get("phase") != name
                or not re.fullmatch(r"[0-9a-f]{32}", str(receipt.get("phase_nonce")))
                or receipt.get("disposition") not in {"completed", "reused"}
                or not isinstance(receipt.get("inputs"), dict)
                or not isinstance(receipt.get("outputs"), dict)
                or receipt.get("input_sha256")
                != sha256_bytes(canonical_json(receipt.get("inputs")))):
            fail(f"DX0 original transaction phase receipt differs: {name}")
    execute = phases.get("execute_deck_batch")
    expected_execute_inputs = {
        "deck_execution_input_sha256": result["execution_input"]["identity_sha256"],
        "proof_plan_sha256": result["closed_plan"]["sha256"],
        "phase_nonce": result["original_observation"]["phase_nonce"],
    }
    if (not isinstance(execute, dict)
            or execute.get("disposition") != "completed"
            or execute.get("phase_nonce")
            != result["original_observation"]["phase_nonce"]
            or execute.get("inputs") != expected_execute_inputs
            or execute.get("outputs") != {"retained_result_sha256": result_sha}):
        fail("DX0 original Deck execution phase/result join differs")
    retained = phases.get("retrieve_and_retain_result")
    retained_outputs = None if not isinstance(retained, dict) else retained.get("outputs")
    if (not isinstance(retained_outputs, dict)
            or retained_outputs.get("retained_result_sha256") != result_sha):
        fail("DX0 original retained-result phase/result join differs")
    transfer = phases.get("transfer_and_admit_deck_inputs")
    transfer_outputs = None if not isinstance(transfer, dict) else transfer.get("outputs")
    if (not isinstance(transfer_outputs, dict)
            or transfer_outputs.get("source_ref")
            != result["source_handoff"]["advertised_ref"]
            or transfer_outputs.get("detached_worktree_commit")
            != result["deck_execution_source"]["commit"]
            or transfer_outputs.get("deck_github_operations") != 0):
        fail("DX0 original Deck admission/source join differs")
    return {
        "retained_private_state_available": True,
        "deck_execution_source": result["deck_execution_source"]["commit"],
        "manual_commands": value["run_invocation_count"],
        "effect_counts": dict(counts),
        "phase_count": len(phases),
        "execution_phase_join_valid": True,
        "ledger_disposition": ledger_disposition,
    }


def _original_observation_effects(
        result: dict[str, Any], *,
        active_driver: ProofTransactionDriver | None = None) -> dict[str, Any]:
    state_path = (dx0_mac_transaction_parent() / result["operation_nonce"]
                  / "DX0_TRANSACTION_STATE.json")
    if not state_path.is_file() or state_path.is_symlink():
        return {"retained_private_state_available": False}
    value = parse_json_no_duplicates(
        state_path.read_bytes(), "DX0 original transaction state"
    )
    if canonical_json(value) != state_path.read_bytes():
        fail("DX0 original transaction state is not canonical")
    if value.get("state") != "transaction_complete":
        return _validate_original_observation_state(
            result, value, require_complete=False
        )
    return _validate_original_observation_state(result, value)


def _obtain_fixture_for_seed(
        wrapper: pathlib.Path | None = None, *,
        store: pathlib.Path | None = None,
        verify_store: Callable[[pathlib.Path], dict[str, Any]] = verify_fixture_store,
        seed_from_wrapper: Callable[[pathlib.Path], dict[str, Any]] = seed_fixture,
        github_factory: Callable[[], GitHubAdapter] = GitHubAdapter,
        ) -> tuple[dict[str, Any], str]:
    """Reuse exact Mac fixture state before considering an authenticated fetch."""
    if wrapper is not None:
        return seed_from_wrapper(wrapper.resolve()), "operator_supplied_wrapper"
    selected_store = (store or dx0_mac_fixture_parent()
                      / DX0_AGAIN_BUNDLE_MANIFEST_SHA256)
    if selected_store.exists() or selected_store.is_symlink():
        return verify_store(selected_store), "reused_verified_mac_store"
    with tempfile.TemporaryDirectory(prefix="dx0-fixture-download-") as temporary:
        target = pathlib.Path(temporary) / "artifact.zip"
        github = github_factory()
        artifact = github.json(
            f"/repos/{REPOSITORY}/actions/artifacts/{DX0_ACCEPTED_WA0_ARTIFACT_ID}"
        )
        if artifact.get("digest") != f"sha256:{DX0_ACCEPTED_WA0_WRAPPER_SHA256}":
            fail("DX0_ACCEPTED_FIXTURE_BLOCKED: accepted Actions artifact digest differs")
        github.download(
            f"/repos/{REPOSITORY}/actions/artifacts/{DX0_ACCEPTED_WA0_ARTIFACT_ID}/zip",
            target,
        )
        return seed_from_wrapper(target), "downloaded_exact_artifact_id"


def seed_fixture_command(wrapper: pathlib.Path | None = None) -> dict[str, Any]:
    disposition = "reused"
    fixture, disposition = _obtain_fixture_for_seed(wrapper)
    ssh = SSHAdapter()
    remote = ssh.publish_tree(
        pathlib.Path(fixture["root"]),
        "/home/deck/.local/share/linux-vst-bridge/fixtures/by-manifest",
        DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
    )
    receipt = {"schema": "linux-vst-bridge-dx0-fixture-seeding/v1",
               "mac_disposition": fixture["disposition"],
               "acquisition_disposition": disposition,
               "deck_disposition": remote,
               "fixture_identity_sha256": fixture["identity_sha256"],
               "artifact_id": DX0_ACCEPTED_WA0_ARTIFACT_ID,
               "external_operation": "one_time_fixture_seeding"}
    root = dx0_mac_transaction_parent().parent / "fixture-seeding"
    root.mkdir(parents=True, exist_ok=True)
    write_atomic(root / "DX0_FIXTURE_SEEDING_RECEIPT.json", canonical_json(receipt))
    return receipt


def _source_handoff_and_admission(driver: ProofTransactionDriver,
                                  ssh: SSHAdapter) -> dict[str, Any]:
    stage = driver.root / "source-handoff"
    if stage.exists():
        local = verify_source_handoff(stage, driver.source_commit)
    else:
        created = create_source_handoff(driver.source_commit, stage)
        local = verify_source_handoff(stage, driver.source_commit)
        if local["receipt_sha256"] != created["receipt_sha256"]:
            fail("DX0 source-handoff creation/readback differs")
    disposition = ssh.publish_tree(
        stage, "/home/deck/.local/share/linux-vst-bridge/handoffs/dx0/source/by-commit",
        driver.source_commit,
    )
    if disposition == "transferred":
        driver.effect_counts["source_transfers"] += 1
        driver.save()
    receipt = local["receipt"]
    remote_root = ("/home/deck/.local/share/linux-vst-bridge/handoffs/dx0/source/"
                   f"by-commit/{driver.source_commit}")
    bundle = f"{remote_root}/{receipt['bundle']['name']}"
    advertised = receipt["bundle"]["advertised_ref"]
    repository = "/home/deck/code/Linux-VST-bridge"
    worktree = f"/home/deck/.local/share/linux-vst-bridge/worktrees/dx0/{driver.source_commit}"
    script = (
        f"test -d {shlex.quote(repository)} && "
        f"git -C {shlex.quote(repository)} bundle verify {shlex.quote(bundle)} >/dev/null 2>&1 && "
        f"if git -C {shlex.quote(repository)} show-ref --verify --quiet {shlex.quote(advertised)}; "
        f"then test \"$(git -C {shlex.quote(repository)} rev-parse {shlex.quote(advertised)})\" = {driver.source_commit}; "
        f"else git -C {shlex.quote(repository)} fetch {shlex.quote(bundle)} "
        f"{shlex.quote(advertised + ':' + advertised)} >/dev/null 2>&1; fi && "
        f"if test -d {shlex.quote(worktree)}; then :; "
        f"else mkdir -p {shlex.quote(str(pathlib.PurePosixPath(worktree).parent))} && "
        f"git -C {shlex.quote(repository)} worktree add --detach {shlex.quote(worktree)} "
        f"{driver.source_commit} >/dev/null 2>&1; fi && "
        f"test \"$(git -C {shlex.quote(worktree)} rev-parse HEAD)\" = {driver.source_commit} && "
        f"test ! -L {shlex.quote(worktree)} && "
        f"test -z \"$(git -C {shlex.quote(worktree)} branch --show-current)\" && "
        f"test -z \"$(git -C {shlex.quote(worktree)} status --porcelain=v1 --untracked-files=all)\""
    )
    ssh.run(script, timeout=180.0)
    driver.phase("create_source_handoff", "completed" if disposition == "transferred" else "reused",
                 inputs={"consumer_source": driver.source_role},
                 outputs={"bundle_sha256": receipt["bundle"]["sha256"],
                          "receipt_sha256": local["receipt_sha256"],
                          "advertised_ref": advertised})
    return {"receipt": receipt, "receipt_sha256": local["receipt_sha256"],
            "worktree": worktree, "remote_root": remote_root}


def _admit_deck_inputs(driver: ProofTransactionDriver, ssh: SSHAdapter,
                       host: dict[str, Any], fixture: dict[str, Any],
                       handoff: dict[str, Any], deck_input: dict[str, Any],
                       deck_input_sha: str) -> None:
    ssh.verify_tree(pathlib.Path(fixture["root"]),
                    "/home/deck/.local/share/linux-vst-bridge/fixtures/by-manifest",
                    DX0_AGAIN_BUNDLE_MANIFEST_SHA256)
    disposition = ssh.publish_tree(
        pathlib.Path(host["root"]),
        "/home/deck/.local/share/linux-vst-bridge/host-artifacts/by-manifest",
        host["manifest_sha256"],
    )
    if disposition == "transferred":
        driver.effect_counts["artifact_transfers"] += 1
        driver.save()
    driver.phase("transfer_and_admit_deck_inputs",
                 "completed" if disposition == "transferred" else "reused",
                 inputs={"deck_execution_input_sha256": deck_input_sha,
                         "host_artifact_manifest_sha256": host["manifest_sha256"],
                         "accepted_fixture_identity_sha256": fixture["identity_sha256"]},
                 outputs={"source_ref": handoff["receipt"]["bundle"]["advertised_ref"],
                          "detached_worktree_commit": driver.source_commit,
                          "deck_github_operations": 0})
    driver.set_state("handoff_admitted")


def _write_deck_intent(driver: ProofTransactionDriver, host: dict[str, Any],
                       fixture: dict[str, Any], handoff: dict[str, Any],
                       deck_input: dict[str, Any], deck_input_sha: str,
                       phase_nonce: str) -> pathlib.Path:
    intent = {
        "schema": "linux-vst-bridge-dx0-deck-intent/v1",
        "operation_nonce": driver.state["operation_nonce"],
        "phase_nonce": phase_nonce,
        "execution_source": driver.source_role,
        "deck_execution_input": deck_input,
        "deck_execution_input_sha256": deck_input_sha,
        "proof_plan": driver.plan,
        "proof_plan_sha256": driver.plan_sha,
        "host_artifact_manifest_sha256": host["manifest_sha256"],
        "accepted_fixture_identity_sha256": fixture["identity_sha256"],
        "source_handoff_receipt_sha256": handoff["receipt_sha256"],
    }
    path = driver.root / "DX0_DECK_EXECUTION_INTENT.json"
    data = canonical_json(intent)
    if path.exists():
        if path.read_bytes() != data:
            fail("DX0 persisted Deck intent differs")
    else:
        write_atomic(path, data)
    return path


def _record_deck_result(driver: ProofTransactionDriver,
                        result: dict[str, Any], deck_input_sha: str) -> None:
    """Advance one exact execute phase without rewriting its persisted intent."""
    result_sha = sha256_bytes(canonical_json(result))
    phase = driver.state["phases"].get("execute_deck_batch")
    if phase is None:
        driver.phase(
            "execute_deck_batch", "reused",
            inputs={"deck_execution_input_sha256": deck_input_sha,
                    "proof_plan_sha256": driver.plan_sha},
            outputs={"retained_result_sha256": result_sha},
        )
    elif phase["disposition"] in {"completed", "reused"}:
        if phase.get("outputs") != {"retained_result_sha256": result_sha}:
            fail("DX0_DECK_TRANSACTION_BLOCKED: retained execute result differs")
    else:
        expected_inputs = {
            "deck_execution_input_sha256": deck_input_sha,
            "proof_plan_sha256": driver.plan_sha,
            "phase_nonce": phase["phase_nonce"],
        }
        if (phase.get("inputs") != expected_inputs
                or result.get("operation_nonce") != driver.state["operation_nonce"]
                or result.get("original_observation", {}).get("phase_nonce")
                != phase["phase_nonce"]):
            fail("DX0_DECK_TRANSACTION_BLOCKED: recovered execute intent differs")
        driver.phase(
            "execute_deck_batch", "completed",
            inputs=phase["inputs"],
            outputs={"retained_result_sha256": result_sha},
            phase_nonce=phase["phase_nonce"],
        )
    driver.set_state("transaction_result_retained")


def _run_or_retrieve_deck(driver: ProofTransactionDriver, ssh: SSHAdapter,
                          host: dict[str, Any], fixture: dict[str, Any],
                          handoff: dict[str, Any], deck_input: dict[str, Any],
                          deck_input_sha: str) -> dict[str, Any]:
    mac_result = dx0_mac_result_parent() / deck_input_sha / "DX0_TRANSACTION_RESULT.json"
    if mac_result.exists():
        result = validate_result_file(mac_result,
                                      expected_execution_input_sha256=deck_input_sha,
                                      expected_plan_sha256=driver.plan_sha)
        _validate_result_store_joins(result, host, fixture,
                                     verified_handoff=handoff)
        _record_deck_result(driver, result, deck_input_sha)
        return result
    recovered = _copy_result_to_mac(ssh, deck_input_sha, driver.plan_sha)
    if recovered is not None:
        _validate_result_store_joins(recovered["result"], host, fixture,
                                     verified_handoff=handoff)
        _record_deck_result(driver, recovered["result"], deck_input_sha)
        return recovered["result"]

    prior = driver.state["phases"].get("execute_deck_batch")
    phase_nonce = (prior or {}).get("phase_nonce") or os.urandom(16).hex()
    if prior is not None or driver.effect_counts["deck_executions"] != 0:
        fail("DX0_DECK_TRANSACTION_BLOCKED: prior Deck launch has no recoverable result")
    intent_path = _write_deck_intent(driver, host, fixture, handoff, deck_input,
                                     deck_input_sha, phase_nonce)
    remote_intent_parent = ("/home/deck/.local/share/linux-vst-bridge/proof/intents/"
                            f"{driver.state['operation_nonce']}")
    remote_intent = f"{remote_intent_parent}/DX0_DECK_EXECUTION_INTENT.json"
    remote_state = ssh.run(
        f"if test -f {shlex.quote(remote_intent)}; then printf complete; "
        f"elif test -e {shlex.quote(remote_intent_parent)}; then printf partial; "
        "else printf absent; fi"
    ).decode()
    if remote_state == "absent":
        ssh.run(f"mkdir -p {shlex.quote(remote_intent_parent)}")
        ssh.copy(intent_path, remote_intent)
    elif remote_state != "complete":
        fail("DX0_DECK_TRANSACTION_BLOCKED: preserved Deck intent publication is partial")
    observed = ssh.run(f"sha256sum {shlex.quote(remote_intent)} | cut -d' ' -f1").decode().strip()
    if observed != sha256_file(intent_path):
        fail("DX0_HANDOFF_BLOCKED: Deck execution-intent bytes differ")
    phase_inputs = {"deck_execution_input_sha256": deck_input_sha,
                    "proof_plan_sha256": driver.plan_sha,
                    "phase_nonce": phase_nonce}
    driver.phase("execute_deck_batch", "prepared", inputs=phase_inputs,
                 outputs=None, phase_nonce=phase_nonce)
    driver.effect_counts["deck_executions"] += 1
    driver.set_state("deck_batch_in_flight")
    remote_driver_lock_parent = (
        "/home/deck/.local/share/linux-vst-bridge/proof/results/.driver-locks"
    )
    remote_driver_lock = (
        f"{remote_driver_lock_parent}/{deck_input_sha}-{driver.plan_sha}"
    )
    remote_result_root = (
        "/home/deck/.local/share/linux-vst-bridge/proof/results/"
        f"by-execution-input/{deck_input_sha}"
    )
    remote_result = f"{remote_result_root}/DX0_TRANSACTION_RESULT.json"
    remote_sidecar = f"{remote_result}.sha256"
    # This outer Deck marker composes with run.py's process-level marker. A
    # contender that acquires after publication invokes run.py only after its
    # own retained-result recheck, so it cannot launch Proton a second time.
    command_line = (
        f"mkdir -p {shlex.quote(remote_driver_lock_parent)} && "
        f"test ! -L {shlex.quote(remote_driver_lock_parent)} && "
        f"if mkdir {shlex.quote(remote_driver_lock)}; then "
        f"cp {shlex.quote(remote_intent)} "
        f"{shlex.quote(remote_driver_lock + '/prepared-intent.json')} && "
        f"if cd {shlex.quote(handoff['worktree'])} && "
        "env -u GH_TOKEN -u GITHUB_TOKEN -u GITHUB_PAT -u SSH_AUTH_SOCK "
        f"/usr/bin/python3 tools/wf0-factory-census/run.py execute "
        f"--intent {shlex.quote(remote_intent)}; then "
        f"test -f {shlex.quote(remote_result)} && "
        f"test ! -L {shlex.quote(remote_result)} && "
        f"test -f {shlex.quote(remote_sidecar)} && "
        f"test ! -L {shlex.quote(remote_sidecar)} && "
        f"unlink {shlex.quote(remote_driver_lock + '/prepared-intent.json')} && "
        f"rmdir {shlex.quote(remote_driver_lock)}; "
        "else exit $?; fi; "
        "else exit 73; fi"
    )
    try:
        ssh.run(command_line, timeout=900.0)
    except Exception:
        recovered = _copy_result_to_mac(ssh, deck_input_sha, driver.plan_sha)
        if recovered is None:
            driver.phase("execute_deck_batch", "outcome_unresolved", inputs=phase_inputs,
                         outputs=None, phase_nonce=phase_nonce)
            fail("DX0_DECK_TRANSACTION_BLOCKED: Deck outcome is unresolved; no duplicate launch permitted")
    recovered = _copy_result_to_mac(ssh, deck_input_sha, driver.plan_sha)
    if recovered is None:
        driver.phase("execute_deck_batch", "outcome_unresolved", inputs=phase_inputs,
                     outputs=None, phase_nonce=phase_nonce)
        fail("DX0_DECK_TRANSACTION_BLOCKED: completed Deck acknowledgement lacks a retained result")
    _validate_result_store_joins(recovered["result"], host, fixture,
                                 verified_handoff=handoff)
    _record_deck_result(driver, recovered["result"], deck_input_sha)
    return recovered["result"]


def _reconcile_deck_execution_writer(
        driver: ProofTransactionDriver, deck_input_sha: str,
        read_local: Callable[[], dict[str, Any] | None],
        recover_remote: Callable[[], dict[str, Any] | None],
        *, lock_parent: pathlib.Path | None = None,
        ) -> tuple[dict[str, Any], pathlib.Path | None, dict[str, Any] | None, bool]:
    """Production ordering for lost-ack recovery before any Deck relaunch."""
    intent = {
        "deck_execution_input_sha256": deck_input_sha,
        "proof_plan_sha256": driver.plan_sha,
        "operation_nonce": driver.state["operation_nonce"],
    }
    lock, publication, may_start = driver.acquire_after_publication_recheck(
        lock_parent or (dx0_mac_result_parent() / ".locks"),
        f"{deck_input_sha}-{driver.plan_sha}",
        intent,
        read_local,
        recover_remote,
    )
    return intent, lock, publication, may_start


def run_transaction(driver: ProofTransactionDriver) -> dict[str, Any]:
    deterministic = driver.validate_local()
    driver.freeze_source()
    fixture = driver.verify_fixture()
    cached_host = driver._matching_host_cache()
    planned_deck_input_sha: str | None = None
    planned_deck_executions: int | str = "blocked_until_host_is_accepted"
    if cached_host is not None:
        planned_deck_input = dx0_deck_execution_input(
            driver.source_commit, cached_host["manifest_sha256"],
            fixture["identity_sha256"], driver.plan_sha,
        )
        planned_deck_input_sha = dx0_identity_sha256(planned_deck_input)
        planned_result_path = (
            dx0_mac_result_parent() / planned_deck_input_sha
            / "DX0_TRANSACTION_RESULT.json"
        )
        planned_deck_executions = 1
        if planned_result_path.exists():
            planned_result = validate_result_file(
                planned_result_path,
                expected_execution_input_sha256=planned_deck_input_sha,
                expected_plan_sha256=driver.plan_sha,
            )
            _validate_result_store_joins(planned_result, cached_host, fixture)
            planned_deck_executions = 0
    plan_report = {
        "schema": "linux-vst-bridge-dx0-execution-plan/v1",
        "source_commit": driver.source_commit,
        "complete_source_identity_sha256": driver.source_role["identity_sha256"],
        "windows_build_input_sha256": driver.build_input_sha,
        "deck_execution_input_sha256": planned_deck_input_sha,
        "fixture_disposition": fixture["disposition"],
        "planned_counts": {
            "windows_builds": 0 if cached_host else 1,
            "artifact_downloads": 0 if cached_host else 1,
            "custody_operations": 0 if cached_host else 1,
            "fixture_seeds": 0,
            "source_transfers": "zero_or_one_if_live_execution_required",
            "artifact_transfers": "zero_or_one_if_deck_cache_misses",
            "deck_executions": planned_deck_executions,
            "evidence_renders": 1,
        },
        "local_validation": "passed",
        "source_frozen": True,
    }
    print("DX0_EXECUTION_PLAN=" + json.dumps(plan_report, sort_keys=True), flush=True)
    host = driver.reuse_or_produce_host()
    deck_input = dx0_deck_execution_input(
        driver.source_commit, host["manifest_sha256"], fixture["identity_sha256"],
        driver.plan_sha,
    )
    deck_input_sha = dx0_identity_sha256(deck_input)
    mac_result = dx0_mac_result_parent() / deck_input_sha / "DX0_TRANSACTION_RESULT.json"
    def read_mac_result() -> dict[str, Any] | None:
        if not mac_result.exists():
            return None
        return validate_result_file(
            mac_result, expected_execution_input_sha256=deck_input_sha,
            expected_plan_sha256=driver.plan_sha,
        )

    # The canonical Mac driver is the authoritative outer single writer.
    # A restart first reconciles the exact persisted intent with local and
    # remote atomic publication; only a newly acquired lock may launch work.
    ssh_holder: dict[str, SSHAdapter] = {}

    def get_ssh() -> SSHAdapter:
        if "adapter" not in ssh_holder:
            ssh_holder["adapter"] = SSHAdapter()
        return ssh_holder["adapter"]

    def recover_remote_result() -> dict[str, Any] | None:
        recovered = _copy_result_to_mac(
            get_ssh(), deck_input_sha, driver.plan_sha
        )
        return None if recovered is None else recovered["result"]

    execution_intent, execution_lock, result, may_start_execution = (
        _reconcile_deck_execution_writer(
            driver, deck_input_sha, read_mac_result, recover_remote_result
        )
    )
    reused_publication = result is not None
    if result is None:
        assert execution_lock is not None
        if not may_start_execution:
            fail("DX0_DECK_TRANSACTION_BLOCKED: persisted execution outcome is unresolved")
    if result is None:
        ssh = get_ssh()
        handoff = _source_handoff_and_admission(driver, ssh)
        _admit_deck_inputs(driver, ssh, host, fixture, handoff, deck_input, deck_input_sha)
        result = _run_or_retrieve_deck(driver, ssh, host, fixture, handoff,
                                       deck_input, deck_input_sha)
    validate_result(result, expected_execution_input_sha256=deck_input_sha,
                    expected_plan_sha256=driver.plan_sha)
    _validate_result_store_joins(result, host, fixture)
    if reused_publication:
        driver.phase("create_source_handoff", "reused", inputs={
            "deck_execution_source": result["deck_execution_source"],
        }, outputs={
            "bundle_sha256": result["source_handoff"]["bundle_sha256"],
            "receipt_sha256": result["source_handoff"]["receipt_sha256"],
            "advertised_ref": result["source_handoff"]["advertised_ref"],
        })
        driver.phase("transfer_and_admit_deck_inputs", "reused", inputs={
            "deck_execution_input_sha256": deck_input_sha,
        }, outputs={"deck_github_operations": 0, "external_transfer_count": 0})
        _record_deck_result(driver, result, deck_input_sha)
    if execution_lock is not None:
        driver.retire_single_writer(
            execution_lock, publication_valid=True,
            expected_intent=execution_intent,
        )
    admission = result_admission_receipt(
        result, consumer_source=driver.source_role, renderer=driver.renderer
    )
    driver.phase("retrieve_and_retain_result", "completed" if result["deck_execution_source"] == driver.source_role else "reused",
                 inputs={"deck_execution_input_sha256": deck_input_sha},
                 outputs={"retained_result_sha256": admission["retained_result_sha256"],
                          "observation_disposition": admission["disposition"],
                          "result_admission": admission})
    output = driver.root / "evidence-packet"
    render_receipt = driver.state["phases"].get("render_and_validate_evidence")
    render_already_counted = (isinstance(render_receipt, dict)
                              and render_receipt.get("disposition")
                              in {"completed", "reused"})
    projected_costs = {
        **driver.effect_counts,
        "evidence_renders": (driver.effect_counts["evidence_renders"]
                             + (0 if render_already_counted else 1)),
    }
    performed_phases = {
        name for name, receipt in driver.state["phases"].items()
        if receipt["disposition"] in {"completed", "in_flight"}
    } | {"render_and_validate_evidence", "close_transaction"}
    projected_phase_dispositions = _public_phase_dispositions(driver.state)
    projected_phase_dispositions.update({
        "render_and_validate_evidence": "completed",
        "close_transaction": "completed",
    })
    original_observation_transaction = _original_observation_effects(
        result, active_driver=driver
    )
    transaction = {
        "transaction_result": result,
        "proof_rows": _proof_rows(
            deterministic,
            original_observation_transaction=original_observation_transaction,
            result=result,
        ),
        "costs": {"manual_commands": driver.state["run_invocation_count"],
                  "manually_copied_identifiers": 0,
                  **projected_costs,
                  "phase_timings_observed": {
                      "deck_original_observation": {
                          "started_utc": result["original_observation"]["started_utc"],
                          "completed_utc": result["original_observation"]["completed_utc"],
                      },
                      "other_phase_durations_measured": False,
                  },
                  "reused_phases": sorted(name for name, receipt in driver.state["phases"].items()
                                          if receipt["disposition"] == "reused"),
                  "performed_phases": sorted(performed_phases),
                  "original_observation_transaction":
                      original_observation_transaction},
        "evidence_consumer_source": driver.source_role,
        "evidence_renderer": driver.renderer,
        "result_admission": admission,
        "proof_plan": driver.plan,
        "windows_build_input": {"schema": driver.build_input["schema"],
                                "sha256": driver.build_input_sha, "record_count": 17},
        "phase_dispositions": projected_phase_dispositions,
        "fixture_seeding": _fixture_seeding_receipt(),
        "invalidation_results": deterministic["invalidation_results"],
    }
    packet = render_packet(output, transaction, staging_parent=driver.root)
    if not render_already_counted:
        driver.effect_counts["evidence_renders"] += 1
        driver.save()
    driver.phase("render_and_validate_evidence", "completed",
                 inputs={"renderer_identity_sha256": driver.renderer_sha,
                         "retained_result_sha256": admission["retained_result_sha256"]},
                 outputs={"packet_sha256": packet["packet_sha256"], "record_count": 5})
    driver.set_state("evidence_rendered")
    driver.phase("close_transaction", "completed",
                 inputs={"packet_sha256": packet["packet_sha256"],
                         "retained_result_sha256": admission["retained_result_sha256"]},
                 outputs={"closure": "ready_for_final_manifest"})
    transaction_identity = {
        "schema": DX0_TRANSACTION_SCHEMA,
        "operation_nonce": driver.state["operation_nonce"],
        "artifact_producer_source": result["artifact_producer_source"],
        "deck_execution_source": result["deck_execution_source"],
        "evidence_consumer_source": driver.source_role,
        "windows_build_input": {"sha256": driver.build_input_sha},
        "host_artifact": result["host_artifact"],
        "accepted_fixture": result["accepted_fixture"],
        "deck_execution_input": result["execution_input"],
        "proof_plan": {"sha256": driver.plan_sha},
        "transaction_result": {"sha256": admission["retained_result_sha256"]},
        "evidence_renderer": {"sha256": driver.renderer_sha},
        "phase_receipts": driver.state["phases"],
    }
    driver.finalize_transaction(transaction_identity)
    exact_observation = _original_observation_effects(result)
    if _proof_rows(
            deterministic,
            original_observation_transaction=exact_observation,
            result=result) != transaction["proof_rows"]:
        fail("DX0 final original-observation readback differs from rendered proof")
    return driver.completed_result()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    for name in ("run", "validate", "plan"):
        item = sub.add_parser(name)
        item.add_argument("--source", required=True)
        item.add_argument("--plan", required=True)
    seed = sub.add_parser("seed-fixture")
    seed.add_argument("--wrapper", type=pathlib.Path)
    args = parser.parse_args()
    if args.operation == "seed-fixture":
        result = seed_fixture_command(args.wrapper)
    else:
        driver = ProofTransactionDriver(args.source, args.plan)
        if args.operation == "validate":
            result = driver.validate_local()
        elif args.operation == "plan":
            result = {
                "schema": "linux-vst-bridge-dx0-execution-plan/v1",
                "source": driver.source_role,
                "windows_build_input_sha256": driver.build_input_sha,
                "proof_plan_sha256": driver.plan_sha,
                "evidence_renderer_sha256": driver.renderer_sha,
                "source_frozen": False,
                "external_effects": 0,
            }
        else:
            if driver.state["state"] == "transaction_complete":
                result = driver.completed_result()
            else:
                driver.record_run_invocation()
                result = run_transaction(driver)
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"DX0_ERROR: {error}", file=sys.stderr, flush=True)
        raise

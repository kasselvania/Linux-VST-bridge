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
    publish_host_custody, read_canonical_json, seed_fixture, verify_fixture_store,
    verify_hash_sidecar, verify_host_store, verify_source_handoff,
)
from common import (  # noqa: E402
    PC0_BLOCKED_OUTCOMES, PC0_BRANCH, PC0_COMPLETE_SOURCE_SCHEMA,
    PC0_PLAN_ID, PC0_REF, PC0_RESULT_SCHEMA, PC0_SOURCE_PATHS,
    PC0_PROOF_CLAIMS,
    evidence_paths, source_contract,
    DX0_ACCEPTED_WA0_ARTIFACT_ID, DX0_ACCEPTED_WA0_WRAPPER_SHA256,
    DX0_AGAIN_BUNDLE_MANIFEST_SHA256, DX0_BASIS_COMMIT, DX0_BRANCH,
    DX0_DECK_EXECUTION_INPUT_SCHEMA, DX0_EVIDENCE_PATHS, DX0_HOST_MODE,
    DX0_MAC_HOST_CUSTODY_SCHEMA, DX0_PLAN_ID, DX0_REF, DX0_SOURCE_PATHS,
    DX0_SOURCE_HANDOFF_SCHEMA, DX0_TRANSACTION_SCHEMA,
    DX0_TRANSACTION_STATE_SCHEMA, REPOSITORY,
    canonical_json, command, command_text, dx0_closed_plan,
    dx0_complete_source, dx0_complete_source_sha256, dx0_deck_execution_input,
    dx0_evidence_renderer, dx0_identity_sha256, dx0_mac_host_artifact_parent,
    dx0_mac_fixture_parent, dx0_mac_result_parent, dx0_mac_transaction_parent,
    dx0_records, dx0_require_frozen_source, dx0_source_role, dx0_windows_build_input,
    fail, parse_json_no_duplicates, repo_root, sha256_bytes, sha256_file, write_atomic,
)
from evidence import (  # noqa: E402
    packet_identity, publish_packet, render_packet, result_admission_receipt,
    validate_corrective_preflight, validate_failure_diagnostic_file,
    validate_pre_evidence_snapshot, validate_pre_evidence_snapshot_file,
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
PC0_FAILED_SOURCE = "7ac6095a488d0077fcc78fedc5abd870b5ffb1cb"
PC0_FAILED_TREE = "ad4230a713a2bb644476be8be9d374a1feb36b06"
PC0_FAILED_TRANSACTION = "a14bcc65d15e9fbdf15810a658b2ee87"
PC0_FAILED_JOURNAL_SHA256 = "06473755eb7ccfa2529522bb29e3fb44a5a4a67ea38fd0e797d198939e1ed966"
PC0_RECOVERY_SHA256 = "870039310c0f6ee4d0bf4044d629e6f6e7d92d2f901b7b40a60f9724bb84f9e5"
PC0_FAILED_INPUT_SHA256 = "ae89ee61636074feac5c875bab9b8a9621e3a0c6f7fa83b6d33bc11b94e631a3"
PC0_PROOF_PLAN_SHA256 = "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248"
PC0_FAILED_INTENT_SHA256 = "80510b7dfe12415e13b9af2bf29164e36fab7db99abd239cdab26d8accbe1ddc"
PC0_WINDOWS_BUILD_INPUT_SHA256 = "575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849"
PC0_HOST_MANIFEST_SHA256 = "d0e11c374b7b1cb99b357faaa109e9310edc265c159559bd59a2098148484e9c"
PC0_HOST_BUILD_RECEIPT_SHA256 = "7de6884d12b3144ff725fe9bc2550c7884356cc4379ed86f6f68e1ed8c3b2419"
PC0_HOST_CUSTODY_SHA256 = "e2a06cb0f5ae69570e070bf6654bf1715f1b26fbfeb646f669d4fefb55db4f84"
PC0_PRODUCER_RUN_ID = 33812659869
PC0_PRODUCER_RUN_ATTEMPT = 1
PC0_ARTIFACT_ID = 9915439437
PC0_CORRECTIVE_PREFLIGHT_SCHEMA = "linux-vst-bridge-pc0-corrective-deck-preflight/v1"
PC0_CORRECTIVE_HISTORY_SCHEMA = "linux-vst-bridge-pc0-corrective-history/v1"
PC0_FIXTURE_IDENTITY_SHA256 = "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6"
PC0_LOCAL_RESERVATION_INTENT_SHA256 = "6ac44f367507c89993637e3efa81af100b1dd162e403f1437510812d3f25fd85"
PC0_RUNTIME_DISCOVERY_COMMENT_ID = 5533164226
PC0_PRE_EVIDENCE_NAME = "PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json"
PC0_CORRECTIVE_RESERVATION_KIND = "v3_single_corrective"
PC0_HISTORICAL_EFFECT_COUNTS = {
    "windows_builds": 1, "artifact_downloads": 1,
    "custody_operations": 1, "artifact_transfers": 1,
    "source_transfers": 1, "deck_executions": 1,
    "evidence_renders": 0,
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
    """Admit exactly one fourteen-path source child of merged V3 authority."""
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
        "commit": commit,
        "tree": tree,
        "parent": parent,
        "ref": PC0_REF,
        "record_count": len(records),
        "records": records,
    }
    dx0_complete_source_sha256(value)
    return value


def pc0_v3_require_frozen_source(commit: str, *, detached: bool) -> dict[str, Any]:
    root = repo_root()
    if command_text(["git", "status", "--porcelain=v1", "--untracked-files=all"], cwd=root):
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source worktree is not clean")
    if command_text(["git", "rev-parse", "HEAD"], cwd=root) != commit:
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source worktree HEAD differs")
    branch = command_text(["git", "branch", "--show-current"], cwd=root)
    if detached and branch:
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 Deck worktree is not detached")
    if not detached and "refs/heads/" + branch != PC0_REF:
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 implementation branch differs")
    return pc0_v3_complete_source(commit, root=root)


def _pc0_read_canonical_exact(path: pathlib.Path, expected_sha256: str,
                              label: str) -> dict[str, Any]:
    if (not path.is_file() or path.is_symlink()
            or path.stat().st_size > 4 * 1024 * 1024):
        fail(f"PC0_EVIDENCE_BLOCKED: {label} is absent or unsafe")
    data = path.read_bytes()
    if sha256_bytes(data) != expected_sha256:
        fail(f"PC0_EVIDENCE_BLOCKED: {label} hash differs")
    value = parse_json_no_duplicates(data, label)
    if not isinstance(value, dict) or canonical_json(value) != data:
        fail(f"PC0_EVIDENCE_BLOCKED: {label} is not canonical")
    return value


def _pc0_failed_source_role() -> dict[str, Any]:
    source = dx0_complete_source(PC0_FAILED_SOURCE)
    if source["tree"] != PC0_FAILED_TREE or source["ref"] != PC0_REF:
        fail("PC0_EVIDENCE_BLOCKED: failed source identity differs")
    return dx0_source_role(source)


def pc0_admit_historical_authority(proof_root: pathlib.Path) -> dict[str, Any]:
    """Admit the immutable V2 journal, recovery receipt, intent, and cost facts."""
    root = proof_root / PC0_FAILED_TRANSACTION
    journal_path = root / "DX0_TRANSACTION_STATE.json"
    recovery_path = root / "PC0_OPERATOR_RECOVERY.json"
    intent_path = root / "DX0_DECK_EXECUTION_INTENT.json"
    journal = _pc0_read_canonical_exact(
        journal_path, PC0_FAILED_JOURNAL_SHA256, "historical PC0 journal"
    )
    recovery = _pc0_read_canonical_exact(
        recovery_path, PC0_RECOVERY_SHA256, "historical PC0 recovery receipt"
    )
    intent = _pc0_read_canonical_exact(
        intent_path, PC0_FAILED_INTENT_SHA256, "historical PC0 Deck intent"
    )
    source = _pc0_failed_source_role()
    expected_phases = {
        "derive_identities", "plan_external_work", "freeze_source",
        "verify_fixture", "reuse_or_produce_host", "custody_host_artifact",
        "create_source_handoff", "transfer_and_admit_deck_inputs",
        "execute_deck_batch",
    }
    execute = journal.get("phases", {}).get("execute_deck_batch", {})
    host = journal.get("phases", {}).get("reuse_or_produce_host", {})
    transfer = journal.get("phases", {}).get("transfer_and_admit_deck_inputs", {})
    if (set(journal) != {
            "schema", "operation_nonce", "transaction_key", "state", "source",
            "plan_sha256", "created_utc", "run_invocation_count", "phases",
            "effect_counts"}
            or journal.get("schema") != DX0_TRANSACTION_STATE_SCHEMA
            or journal.get("operation_nonce") != PC0_FAILED_TRANSACTION
            or journal.get("source") != source
            or journal.get("state") != "deck_batch_in_flight"
            or journal.get("plan_sha256") != PC0_PROOF_PLAN_SHA256
            or journal.get("run_invocation_count") != 1
            or journal.get("effect_counts") != PC0_HISTORICAL_EFFECT_COUNTS
            or set(journal.get("phases", {})) != expected_phases
            or execute.get("disposition") != "failed"
            or execute.get("inputs", {}).get("deck_execution_input_sha256")
            != PC0_FAILED_INPUT_SHA256
            or execute.get("inputs", {}).get("proof_plan_sha256")
            != PC0_PROOF_PLAN_SHA256
            or execute.get("outputs") != {"blocker": "PC0_EVIDENCE_BLOCKED"}
            or host.get("outputs") != {
                "artifact_id": PC0_ARTIFACT_ID,
                "host_artifact_manifest_sha256": PC0_HOST_MANIFEST_SHA256,
                "run_attempt": PC0_PRODUCER_RUN_ATTEMPT,
                "run_id": PC0_PRODUCER_RUN_ID,
            }
            or transfer.get("outputs", {}).get("detached_worktree_commit")
            != PC0_FAILED_SOURCE):
        fail("PC0_EVIDENCE_BLOCKED: historical PC0 journal facts differ")
    if (set(recovery) != {
            "schema", "created_utc", "continuation_started_utc",
            "continuation_stopped_utc", "operation_nonce", "source_commit",
            "deck_execution_input_sha256", "proof_plan_sha256",
            "original_driver_run_invocation_count", "original_effect_counts",
            "original_failure_boundary", "local_reservation_intent_sha256",
            "remote_prelaunch_reconciliation",
            "operator_authorized_recovery_continuation_count",
            "successful_windows_artifact_reused", "local_result", "disposition",
            "error_type"}
            or recovery.get("schema")
            != "linux-vst-bridge-pc0-operator-recovery/v1"
            or recovery.get("operation_nonce") != PC0_FAILED_TRANSACTION
            or recovery.get("source_commit") != PC0_FAILED_SOURCE
            or recovery.get("deck_execution_input_sha256")
            != PC0_FAILED_INPUT_SHA256
            or recovery.get("proof_plan_sha256") != PC0_PROOF_PLAN_SHA256
            or recovery.get("original_driver_run_invocation_count") != 1
            or recovery.get("original_effect_counts") != {
                "windows_builds": 1, "artifact_downloads": 1,
                "custody_operations": 1, "artifact_transfers": 0,
                "source_transfers": 0, "deck_executions": 0,
                "evidence_renders": 0,
            }
            or recovery.get("operator_authorized_recovery_continuation_count") != 1
            or recovery.get("original_failure_boundary")
            != "mac_execution_reservation_created_before_exact_deck_ssh_discovery"
            or recovery.get("local_reservation_intent_sha256")
            != PC0_LOCAL_RESERVATION_INTENT_SHA256
            or recovery.get("successful_windows_artifact_reused") is not True
            or recovery.get("local_result") != "absent"
            or recovery.get("disposition") != "continuation_stopped"
            or recovery.get("error_type") != "WF0Error"):
        fail("PC0_EVIDENCE_BLOCKED: historical PC0 recovery facts differ")
    if recovery.get("remote_prelaunch_reconciliation") != {
            "driver_lock": "absent", "intent": "absent",
            "result": "absent", "result_sidecar": "absent",
            "source_handoff": "absent", "source_worktree": "absent",
    }:
        fail("PC0_EVIDENCE_BLOCKED: historical recovery reconciliation differs")
    if (intent.get("operation_nonce") != PC0_FAILED_TRANSACTION
            or intent.get("execution_source") != source
            or intent.get("deck_execution_input_sha256")
            != PC0_FAILED_INPUT_SHA256
            or intent.get("proof_plan_sha256") != PC0_PROOF_PLAN_SHA256):
        fail("PC0_EVIDENCE_BLOCKED: historical PC0 intent facts differ")
    result_root = dx0_mac_result_parent() / PC0_FAILED_INPUT_SHA256
    if result_root.exists() or result_root.is_symlink():
        fail("PC0_EVIDENCE_BLOCKED: historical PC0 success result exists")
    local_lock = (dx0_mac_result_parent() / ".locks" /
                  f"{PC0_FAILED_INPUT_SHA256}-{PC0_PROOF_PLAN_SHA256}")
    local_prepared = local_lock / "prepared-intent.json"
    if (not local_lock.is_dir() or local_lock.is_symlink()
            or {path.name for path in local_lock.iterdir()} != {"prepared-intent.json"}
            or not local_prepared.is_file() or local_prepared.is_symlink()
            or sha256_file(local_prepared) != PC0_LOCAL_RESERVATION_INTENT_SHA256):
        fail("PC0_EVIDENCE_BLOCKED: historical Mac execution lock differs")
    return {
        "journal": journal, "journal_sha256": PC0_FAILED_JOURNAL_SHA256,
        "recovery": recovery, "recovery_sha256": PC0_RECOVERY_SHA256,
        "intent": intent, "intent_sha256": PC0_FAILED_INTENT_SHA256,
        "source": source,
    }


def pc0_corrective_reservation(preflight_sha256: str) -> dict[str, Any]:
    if re.fullmatch(r"[0-9a-f]{64}", preflight_sha256) is None:
        fail("RETURN_TO_DESIGN_GATE: corrective preflight digest is malformed")
    return {
        "kind": PC0_CORRECTIVE_RESERVATION_KIND,
        "historical_transaction_id": PC0_FAILED_TRANSACTION,
        "reservation_ordinal": 1,
        "additional_positive_deck_batches_maximum": 1,
        "corrective_preflight_sha256": preflight_sha256,
    }
class RemoteOutcomeUnknown(RuntimeError):
    pass


class RemoteDeckBlocked(RuntimeError):
    """A completed Deck command reported one exact approved PC0 blocker."""

    def __init__(self, blocker: str) -> None:
        super().__init__(blocker)
        self.blocker = blocker


class _NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request: urllib.request.Request, file_pointer: Any,
                         code: int, message: str, headers: Any,
                         new_url: str) -> None:
        return None


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def public_source(value: dict[str, Any]) -> dict[str, Any]:
    return dx0_source_role(value)


def _load_driver_source(commit: str, plan_id: str) -> tuple[dict[str, Any], list[tuple[str, str]]]:
    """Load the exact committed source and retain bounded output-only dirt."""
    root = repo_root()
    if command_text(["git", "rev-parse", "HEAD"], cwd=root) != commit:
        fail("DX0 worktree HEAD differs from the requested source")
    source = (pc0_v3_complete_source(commit, root=root)
              if plan_id == PC0_PLAN_ID else dx0_complete_source(commit))
    if "refs/heads/" + command_text(["git", "branch", "--show-current"], cwd=root) != source["ref"]:
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
    return source, entries


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
            "ref": dx0_complete_source(source_sha)["ref"].removeprefix("refs/heads/"),
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
        self.source, self._initial_status = _load_driver_source(source_commit, plan_id)
        self.source_role = public_source(self.source)
        self.branch = self.source["ref"].removeprefix("refs/heads/")
        self.evidence_paths = evidence_paths(plan_id)
        if source_contract(ref=self.source["ref"])["plan_id"] != plan_id:
            fail("PC0_DESIGN_PREFLIGHT_BLOCKED: source/closed plan mismatch")
        self.build_input = dx0_windows_build_input(source_commit)
        self.build_input_sha = dx0_identity_sha256(self.build_input)
        self.renderer = dx0_evidence_renderer(source_commit, plan_id=plan_id)
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
        expected = set(self.evidence_paths)
        observed = {path for status, path in self._initial_status if status == "??"}
        if (len(observed) != len(self._initial_status)
                or observed != expected
                or self.state.get("state") != "transaction_complete"):
            fail("DX0 source worktree is not clean")
        retained = self.root / "evidence-packet"
        published = repo_root() / pathlib.PurePosixPath(self.evidence_paths[0]).parent
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

    def _pc0_exact_host_cache(self) -> dict[str, Any]:
        if self.plan["plan_id"] != PC0_PLAN_ID:
            fail("PC0_DESIGN_PREFLIGHT_BLOCKED: corrective host admission used outside PC0")
        if self.build_input_sha != PC0_WINDOWS_BUILD_INPUT_SHA256:
            fail("RETURN_TO_DESIGN_GATE: WindowsBuildInputIdentity Ready differs")
        cached = self._matching_host_cache()
        if cached is None:
            fail("RETURN_TO_DESIGN_GATE: exact retained PC0 Mac host store is absent")
        workflow = cached.get("build_receipt", {}).get("workflow", {})
        custody = cached.get("custody", {})
        artifact = custody.get("artifact", {})
        if (cached.get("manifest_sha256") != PC0_HOST_MANIFEST_SHA256
                or cached.get("build_receipt_sha256")
                != PC0_HOST_BUILD_RECEIPT_SHA256
                or cached.get("custody_sha256") != PC0_HOST_CUSTODY_SHA256
                or workflow.get("run_id") != PC0_PRODUCER_RUN_ID
                or workflow.get("run_attempt") != PC0_PRODUCER_RUN_ATTEMPT
                or workflow.get("source_sha") != PC0_FAILED_SOURCE
                or artifact.get("id") != PC0_ARTIFACT_ID
                or custody.get("producer_source") != _pc0_failed_source_role()
                or custody.get("windows_build_input_sha256")
                != PC0_WINDOWS_BUILD_INPUT_SHA256):
            fail("RETURN_TO_DESIGN_GATE: retained PC0 Windows producer/store differs")
        return cached

    def _pc0_scan_corrective_history(self) -> None:
        current_seen = False
        historical_seen = False
        for path in sorted(self.proof_root.glob("*/DX0_TRANSACTION_STATE.json")):
            data = path.read_bytes()
            value = parse_json_no_duplicates(data, "PC0 proof-root transaction")
            if canonical_json(value) != data:
                fail("PC0_EVIDENCE_BLOCKED: noncanonical proof-root transaction")
            if value.get("source", {}).get("ref") != PC0_REF:
                continue
            operation_nonce = value.get("operation_nonce")
            if operation_nonce == PC0_FAILED_TRANSACTION:
                historical_seen = True
                continue
            if operation_nonce == self.state["operation_nonce"]:
                current_seen = True
                continue
            effects = value.get("effect_counts")
            phases = value.get("phases")
            if (not isinstance(effects, dict) or set(effects) != EFFECT_COUNT_KEYS
                    or not isinstance(phases, dict)):
                fail("PC0_EVIDENCE_BLOCKED: unresolved PC0 proof-root history")
            execute = phases.get("execute_deck_batch")
            result_path = path.parent / "DX0_TRANSACTION.json"
            reservation = (execute.get("inputs", {}).get("corrective_reservation")
                           if isinstance(execute, dict) else None)
            if (effects.get("deck_executions") != 0 or reservation is not None
                    or result_path.exists() or result_path.is_symlink()
                    or value.get("state") == "transaction_complete"):
                fail("RETURN_TO_DESIGN_GATE: another PC0 execution history exists")
        if not historical_seen or not current_seen:
            fail("PC0_EVIDENCE_BLOCKED: exact historical/current PC0 ownership differs")

    def require_pc0_corrective_authority(
            self, preflight: dict[str, Any], preflight_sha256: str,
            host: dict[str, Any]) -> dict[str, Any]:
        """Prove the twelve-part V3 conjunction before one reservation."""
        if self.plan["plan_id"] != PC0_PLAN_ID:
            fail("PC0_DESIGN_PREFLIGHT_BLOCKED: corrective authority used outside PC0")
        pc0_v3_complete_source(self.source_commit)
        history = pc0_admit_historical_authority(self.proof_root)
        exact_host = self._pc0_exact_host_cache()
        if exact_host["manifest_sha256"] != host.get("manifest_sha256"):
            fail("RETURN_TO_DESIGN_GATE: corrective host admission changed")
        transfer = self.state["phases"].get("transfer_and_admit_deck_inputs")
        execute = self.state["phases"].get("execute_deck_batch")
        local_intent = self.root / "DX0_DECK_EXECUTION_INTENT.json"
        local_result = dx0_mac_result_parent() / preflight["execution_input_sha256"]
        local_diagnostic = self.root / "failure-diagnostic"
        expected_handoff = {
            "receipt_sha256": preflight["source_handoff"]["receipt_sha256"],
            "bundle_sha256": preflight["source_handoff"]["bundle_sha256"],
            "advertised_ref": preflight["source_handoff"]["advertised_ref"],
        }
        validate_corrective_preflight(
            preflight, expected_source=self.source_role,
            expected_execution_input_sha256=preflight["execution_input_sha256"],
            expected_plan_sha256=self.plan_sha,
            expected_operation_nonce=self.state["operation_nonce"],
            expected_handoff=expected_handoff,
        )
        if (sha256_bytes(canonical_json(preflight)) != preflight_sha256
                or getattr(self, "_pc0_preflight_invocation_count", None)
                != self.state["run_invocation_count"]
                or self.state["run_invocation_count"] != 1
                or not isinstance(transfer, dict)
                or transfer.get("disposition") != "reused"
                or transfer.get("outputs", {}).get("corrective_preflight") != preflight
                or transfer.get("outputs", {}).get("corrective_preflight_sha256")
                != preflight_sha256
                or self.state.get("state") != "handoff_admitted"
                or execute is not None
                or self.effect_counts != {
                    "windows_builds": 0, "artifact_downloads": 0,
                    "custody_operations": 0, "artifact_transfers": 0,
                    "source_transfers": self.effect_counts.get("source_transfers"),
                    "deck_executions": 0, "evidence_renders": 0,
                }
                or self.effect_counts["source_transfers"] not in {0, 1}
                or local_intent.exists() or local_intent.is_symlink()
                or local_result.exists() or local_result.is_symlink()
                or local_diagnostic.exists() or local_diagnostic.is_symlink()):
            fail("RETURN_TO_DESIGN_GATE: corrective authority/current absence differs")
        self._pc0_scan_corrective_history()
        self._pc0_corrective_authority_preflight_sha256 = preflight_sha256
        return history

    def reserve_pc0_corrective(self, deck_input_sha: str,
                               preflight_sha256: str) -> dict[str, Any]:
        if (getattr(self, "_pc0_corrective_authority_preflight_sha256", None)
                != preflight_sha256):
            fail("RETURN_TO_DESIGN_GATE: exact corrective authority is absent")
        if (self.state["phases"].get("execute_deck_batch") is not None
                or self.effect_counts["deck_executions"] != 0):
            fail("RETURN_TO_DESIGN_GATE: corrective reservation is already consumed")
        phase_nonce = os.urandom(16).hex()
        reservation = pc0_corrective_reservation(preflight_sha256)
        inputs = {
            "deck_execution_input_sha256": deck_input_sha,
            "proof_plan_sha256": self.plan_sha,
            "phase_nonce": phase_nonce,
            "corrective_reservation": reservation,
        }
        # One atomic journal replacement contains both reservation facts.
        self.effect_counts["deck_executions"] = 1
        receipt = self.phase(
            "execute_deck_batch", "prepared", inputs=inputs,
            outputs=None, phase_nonce=phase_nonce,
        )
        if (receipt.get("inputs") != inputs
                or self.effect_counts["deck_executions"] != 1):
            fail("RETURN_TO_DESIGN_GATE: corrective reservation persistence differs")
        del self._pc0_corrective_authority_preflight_sha256
        return {"phase_nonce": phase_nonce, "inputs": inputs,
                "reservation": reservation}

    def require_pc0_budget(self, effect: str) -> None:
        """Retain the legacy call-site guard as an unconditional V3 producer ban."""
        if self.plan["plan_id"] != PC0_PLAN_ID:
            return
        if effect != "windows_builds":
            fail("RETURN_TO_DESIGN_GATE: PC0 V3 budget gate was misused")
        fail("RETURN_TO_DESIGN_GATE: PC0 V3 authorizes no Windows producer")

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
            repo_root() / pathlib.PurePosixPath(self.evidence_paths[0]).parent,
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
            "proof_row_count": 16 if self.plan["plan_id"] == PC0_PLAN_ID else 14,
        }

    def validate_local(self) -> dict[str, Any]:
        if self.plan["plan_id"] == PC0_PLAN_ID:
            from negative_tests import pc0_deterministic_tests
            return pc0_deterministic_tests(repo_root(), self.source_commit, ProofTransactionDriver)
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
        if self.plan["plan_id"] == PC0_PLAN_ID:
            pc0_v3_require_frozen_source(self.source_commit, detached=False)
        else:
            dx0_require_frozen_source(self.source_commit, detached=False)
        remote = command_text(["git", "ls-remote", "--heads", "origin", self.source["ref"]], cwd=repo_root())
        fields = remote.split()
        if fields != [self.source_commit, self.source["ref"]]:
            fail(f"DX0_SOURCE_FREEZE_BLOCKED: remote source differs: {fields[:1]}")
        self.phase("freeze_source", "completed", inputs={
            "source_identity_sha256": self.source_role["identity_sha256"],
            "remote_ref": self.source["ref"],
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
            f"/repos/{REPOSITORY}/git/ref/heads/{urllib.parse.quote(self.branch, safe='')}"
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
                 f"branch={urllib.parse.quote(self.branch)}&event=workflow_dispatch&per_page=100")
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
                or run.get("head_branch") != self.branch
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
        if self.plan["plan_id"] == PC0_PLAN_ID:
            # V3 has no producer, dispatch, download, or custody fallback.
            return self._retain_cached_host(self._pc0_exact_host_cache())
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
                self.require_pc0_budget("windows_builds")
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
            diagnostic = result.stderr.decode("utf-8", "replace")[-16384:]
            matches = [value for value in re.findall(
                r"^DX0_ERROR: ([A-Z0-9_]+)(?::|$)", diagnostic, re.MULTILINE)
                if value in PC0_BLOCKED_OUTCOMES]
            if len(set(matches)) == 1:
                raise RemoteDeckBlocked(matches[0])
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


def _copy_failure_diagnostic_to_mac(
        driver: ProofTransactionDriver, ssh: SSHAdapter,
        execution_input_sha: str, plan_sha: str,
        phase_nonce: str) -> dict[str, Any] | None:
    """Retrieve only one exact committed diagnostic pair from its inner lock."""
    lock_name = f"{execution_input_sha}-{plan_sha}"
    proof = "/home/deck/.local/share/linux-vst-bridge/proof"
    inner = f"{proof}/results/by-execution-input/.locks/{lock_name}"
    outer = f"{proof}/results/.driver-locks/{lock_name}"
    result_root = f"{proof}/results/by-execution-input/{execution_input_sha}"
    remote_json = f"{inner}/PC0_FAILURE_DIAGNOSTIC.json"
    remote_sidecar = f"{inner}/PC0_FAILURE_DIAGNOSTIC.json.sha256"
    local_intent = driver.root / "DX0_DECK_EXECUTION_INTENT.json"
    if not local_intent.is_file() or local_intent.is_symlink():
        fail("PC0_EVIDENCE_BLOCKED: current corrective intent is absent")
    intent_sha = sha256_file(local_intent)
    state = ssh.run(
        f"if test ! -e {shlex.quote(inner)} && test ! -L {shlex.quote(inner)}; "
        "then printf absent; else "
        f"test -d {shlex.quote(inner)} && test ! -L {shlex.quote(inner)} && "
        f"test -d {shlex.quote(outer)} && test ! -L {shlex.quote(outer)} && "
        f"test ! -e {shlex.quote(result_root)} && test ! -L {shlex.quote(result_root)} && "
        f"test \"$(find {shlex.quote(inner)} -mindepth 1 -maxdepth 1 -printf '%f\\n' | LC_ALL=C sort)\" = "
        "\"$(printf '%s\\n' PC0_FAILURE_DIAGNOSTIC.json "
        "PC0_FAILURE_DIAGNOSTIC.json.sha256 prepared-intent.json | LC_ALL=C sort)\" && "
        f"test \"$(find {shlex.quote(outer)} -mindepth 1 -maxdepth 1 -printf '%f\\n')\" = prepared-intent.json && "
        f"test \"$(sha256sum {shlex.quote(inner + '/prepared-intent.json')} | cut -d' ' -f1)\" = {intent_sha} && "
        f"test \"$(sha256sum {shlex.quote(outer + '/prepared-intent.json')} | cut -d' ' -f1)\" = {intent_sha} && "
        f"test -f {shlex.quote(remote_json)} && test ! -L {shlex.quote(remote_json)} && "
        f"test -f {shlex.quote(remote_sidecar)} && test ! -L {shlex.quote(remote_sidecar)} && "
        "printf present; fi"
    ).decode()
    if state == "absent":
        return None
    if state != "present":
        fail("PC0_EVIDENCE_BLOCKED: remote failure-diagnostic boundary differs")
    target = driver.root / "failure-diagnostic"
    if target.exists() or target.is_symlink():
        if not target.is_dir() or target.is_symlink():
            fail("PC0_EVIDENCE_BLOCKED: Mac failure-diagnostic custody is unsafe")
        diagnostic = validate_failure_diagnostic_file(
            target / "PC0_FAILURE_DIAGNOSTIC.json",
            expected_source=driver.source_role,
            expected_execution_input_sha256=execution_input_sha,
            expected_plan_sha256=plan_sha,
            expected_operation_nonce=driver.state["operation_nonce"],
            expected_phase_nonce=phase_nonce,
        )
        if {path.name for path in target.iterdir()} != {
                "PC0_FAILURE_DIAGNOSTIC.json",
                "PC0_FAILURE_DIAGNOSTIC.json.sha256"}:
            fail("PC0_EVIDENCE_BLOCKED: Mac failure-diagnostic roster differs")
        return {"diagnostic": diagnostic,
                "sha256": sha256_file(target / "PC0_FAILURE_DIAGNOSTIC.json"),
                "disposition": "reused"}
    stage = driver.root / ".pc0-failure-diagnostic-retrieval"
    if stage.exists() or stage.is_symlink():
        fail("PC0_EVIDENCE_BLOCKED: failure-diagnostic retrieval is partial")
    stage.mkdir(mode=0o700)
    try:
        ssh.fetch(remote_json, stage / "PC0_FAILURE_DIAGNOSTIC.json")
        ssh.fetch(remote_sidecar, stage / "PC0_FAILURE_DIAGNOSTIC.json.sha256")
        if {path.name for path in stage.iterdir()} != {
                "PC0_FAILURE_DIAGNOSTIC.json",
                "PC0_FAILURE_DIAGNOSTIC.json.sha256"}:
            fail("PC0_EVIDENCE_BLOCKED: staged failure-diagnostic roster differs")
        diagnostic = validate_failure_diagnostic_file(
            stage / "PC0_FAILURE_DIAGNOSTIC.json",
            expected_source=driver.source_role,
            expected_execution_input_sha256=execution_input_sha,
            expected_plan_sha256=plan_sha,
            expected_operation_nonce=driver.state["operation_nonce"],
            expected_phase_nonce=phase_nonce,
        )
        digest = sha256_file(stage / "PC0_FAILURE_DIAGNOSTIC.json")
        os.replace(stage, target)
    finally:
        if stage.exists():
            shutil.rmtree(stage)
    return {"diagnostic": diagnostic, "sha256": digest,
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
        verified_handoff = (
            pc0_v3_verify_source_handoff(
                handoff_stage, result["deck_execution_source"]["commit"])
            if result.get("schema") == PC0_RESULT_SCHEMA
            else verify_source_handoff(
                handoff_stage, result["deck_execution_source"]["commit"])
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
    pc0 = result["schema"] == PC0_RESULT_SCHEMA
    if pc0:
        descriptions = list(PC0_PROOF_CLAIMS)
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
            or deterministic.get("proof_row_count") != (16 if pc0 else 14)
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
    synthetic = {1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 15} if pc0 else set(range(1, 11)) | {12, 14}
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


def _pc0_v3_source_from_bundle(bundle: pathlib.Path, source_commit: str,
                                advertised_ref: str) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="pc0-v3-source-verify-") as temporary:
        bare = pathlib.Path(temporary) / "verify.git"
        command(["git", "init", "--bare", str(bare)])
        command(["git", "-C", str(bare), "fetch", "--no-tags", str(bundle),
                 f"{advertised_ref}:refs/verify/source"])
        if command_text(["git", "-C", str(bare), "rev-parse", "refs/verify/source"]) != source_commit:
            fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source bundle commit differs")
        return pc0_v3_complete_source(source_commit, root=bare)


def pc0_v3_create_source_handoff(source_commit: str,
                                 stage: pathlib.Path) -> dict[str, Any]:
    source = pc0_v3_complete_source(source_commit)
    role = dx0_source_role(source)
    if stage.exists() or stage.is_symlink():
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source-handoff stage exists")
    stage.mkdir(parents=True)
    advertised_ref = f"refs/handoff/dx0-source/{source_commit}"
    bundle_name = f"dx0-execution-source-{source_commit}.bundle"
    bundle = stage / bundle_name
    with tempfile.TemporaryDirectory(prefix="pc0-v3-source-bare-") as temporary:
        bare = pathlib.Path(temporary) / "handoff.git"
        command(["git", "init", "--bare", str(bare)])
        command(["git", "-C", str(bare), "fetch", "--no-tags", str(repo_root()),
                 source_commit])
        command(["git", "-C", str(bare), "update-ref", advertised_ref, source_commit])
        command(["git", "-C", str(bare), "bundle", "create", str(bundle), advertised_ref])
    prerequisites, refs = _pc0_v3_bundle_header(bundle)
    if prerequisites != 0 or refs != [(source_commit, advertised_ref)]:
        fail("PC0_DESIGN_PREFLIGHT_BLOCKED: V3 source bundle closure differs")
    receipt = {
        "schema": DX0_SOURCE_HANDOFF_SCHEMA,
        "repository": REPOSITORY,
        "design_authority": pc0_v3_source_authority(),
        "implementation_basis": {
            "commit": PC0_V3_AUTHORITY_COMMIT, "tree": PC0_V3_AUTHORITY_TREE,
        },
        "implementation_source": role,
        "implementation_branch": PC0_BRANCH,
        "implementation_ref": PC0_REF,
        "complete_source_schema": source["schema"],
        "complete_source_record_count": source["record_count"],
        "bundle": {
            "name": bundle_name, "advertised_ref": advertised_ref,
            "sha256": sha256_file(bundle), "size": bundle.stat().st_size,
            "prerequisite_count": 0, "advertised_ref_count": 1,
            "git_bundle_verify": "passed",
        },
    }
    receipt_path = stage / "DX0_SOURCE_HANDOFF_RECEIPT.json"
    write_atomic(receipt_path, canonical_json(receipt))
    digest = sha256_file(receipt_path)
    write_atomic(stage / "DX0_SOURCE_HANDOFF_RECEIPT.sha256",
                 f"{digest}  DX0_SOURCE_HANDOFF_RECEIPT.json\n".encode())
    return {"receipt": receipt, "receipt_sha256": digest, "stage": str(stage)}


def pc0_v3_verify_source_handoff(stage: pathlib.Path,
                                 source_commit: str) -> dict[str, Any]:
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
    source = _pc0_v3_source_from_bundle(bundle, source_commit, expected_ref)
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


def _source_handoff_and_admission(driver: ProofTransactionDriver,
                                  ssh: SSHAdapter) -> dict[str, Any]:
    stage = driver.root / "source-handoff"
    if stage.exists():
        local = (pc0_v3_verify_source_handoff(stage, driver.source_commit)
                 if driver.plan["plan_id"] == PC0_PLAN_ID
                 else verify_source_handoff(stage, driver.source_commit))
    else:
        created = (pc0_v3_create_source_handoff(driver.source_commit, stage)
                   if driver.plan["plan_id"] == PC0_PLAN_ID
                   else create_source_handoff(driver.source_commit, stage))
        local = (pc0_v3_verify_source_handoff(stage, driver.source_commit)
                 if driver.plan["plan_id"] == PC0_PLAN_ID
                 else verify_source_handoff(stage, driver.source_commit))
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
                       deck_input_sha: str) -> dict[str, Any]:
    if driver.plan["plan_id"] != PC0_PLAN_ID:
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
        return {}

    lock_key = f"{deck_input_sha}-{driver.plan_sha}"
    local_intent = driver.root / "DX0_DECK_EXECUTION_INTENT.json"
    local_result_root = dx0_mac_result_parent() / deck_input_sha
    local_lock = dx0_mac_result_parent() / ".locks" / lock_key
    if (local_intent.exists() or local_intent.is_symlink()
            or local_result_root.exists() or local_result_root.is_symlink()
            or local_lock.exists() or local_lock.is_symlink()
            or driver.state["phases"].get("execute_deck_batch") is not None
            or driver.effect_counts["deck_executions"] != 0):
        fail("RETURN_TO_DESIGN_GATE: current corrective state exists before preflight")
    command_line = (
        f"cd {shlex.quote(handoff['worktree'])} && "
        "env -u GH_TOKEN -u GITHUB_TOKEN -u GITHUB_PAT -u SSH_AUTH_SOCK "
        "PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -B "
        "tools/wf0-factory-census/run.py corrective-preflight "
        f"--operation-nonce {shlex.quote(driver.state['operation_nonce'])} "
        f"--source-commit {shlex.quote(driver.source_commit)} "
        f"--execution-input-sha256 {shlex.quote(deck_input_sha)} "
        f"--proof-plan-sha256 {shlex.quote(driver.plan_sha)} "
        f"--source-handoff-receipt-sha256 {shlex.quote(handoff['receipt_sha256'])}"
    )
    raw = ssh.run(command_line, timeout=300.0)
    remote = parse_json_no_duplicates(raw, "PC0 corrective Deck preflight")
    if canonical_json(remote) != raw:
        fail("RETURN_TO_DESIGN_GATE: corrective Deck preflight is not canonical")
    if not isinstance(remote, dict) or "schema" in remote:
        fail("RETURN_TO_DESIGN_GATE: corrective Deck preflight remote roster differs")
    remote_absence = remote.get("current_corrective_absence")
    if (not isinstance(remote_absence, dict)
            or set(remote_absence) != {
                "execution_intent_absent", "outer_lock_absent", "inner_lock_absent",
                "result_absent", "result_sidecar_absent", "diagnostic_absent",
                "diagnostic_sidecar_absent"}):
        fail("RETURN_TO_DESIGN_GATE: corrective remote absence roster differs")
    receipt = dict(remote)
    receipt["schema"] = PC0_CORRECTIVE_PREFLIGHT_SCHEMA
    receipt["current_corrective_absence"] = {
        **remote_absence, "reservation_absent": True,
        "mac_execution_lock_absent": True,
    }
    expected_handoff = {
        "receipt_sha256": handoff["receipt_sha256"],
        "bundle_sha256": handoff["receipt"]["bundle"]["sha256"],
        "advertised_ref": handoff["receipt"]["bundle"]["advertised_ref"],
    }
    validate_corrective_preflight(
        receipt, expected_source=driver.source_role,
        expected_execution_input_sha256=deck_input_sha,
        expected_plan_sha256=driver.plan_sha,
        expected_operation_nonce=driver.state["operation_nonce"],
        expected_handoff=expected_handoff,
    )
    post = ssh.run(
        f"test \"$(git -C {shlex.quote(handoff['worktree'])} rev-parse HEAD)\" = "
        f"{shlex.quote(driver.source_commit)} && "
        f"test -z \"$(git -C {shlex.quote(handoff['worktree'])} branch --show-current)\" && "
        f"test -z \"$(git -C {shlex.quote(handoff['worktree'])} status "
        "--porcelain=v1 --untracked-files=all)\" && printf clean"
    ).decode()
    if post != "clean":
        fail("RETURN_TO_DESIGN_GATE: Deck worktree changed during corrective preflight")
    preflight_sha = sha256_bytes(canonical_json(receipt))
    phase = driver.phase(
        "transfer_and_admit_deck_inputs", "reused",
        inputs={"deck_execution_input_sha256": deck_input_sha,
                "host_artifact_manifest_sha256": host["manifest_sha256"],
                "accepted_fixture_identity_sha256": fixture["identity_sha256"]},
        outputs={"source_ref": handoff["receipt"]["bundle"]["advertised_ref"],
                 "detached_worktree_commit": driver.source_commit,
                 "deck_github_operations": 0,
                 "corrective_preflight": receipt,
                 "corrective_preflight_sha256": preflight_sha},
    )
    if (phase.get("disposition") != "reused"
            or phase.get("outputs", {}).get("corrective_preflight") != receipt
            or phase.get("outputs", {}).get("corrective_preflight_sha256")
            != preflight_sha):
        fail("RETURN_TO_DESIGN_GATE: persisted corrective preflight differs")
    driver._pc0_preflight_invocation_count = driver.state["run_invocation_count"]
    driver.set_state("handoff_admitted")
    return {"receipt": receipt, "sha256": preflight_sha}


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
        if driver.plan["plan_id"] == PC0_PLAN_ID:
            fail("RETURN_TO_DESIGN_GATE: PC0 result has no corrective reservation")
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
        if driver.plan["plan_id"] == PC0_PLAN_ID:
            preflight_sha = phase.get("inputs", {}).get(
                "corrective_reservation", {}
            ).get("corrective_preflight_sha256")
            expected_inputs["corrective_reservation"] = \
                pc0_corrective_reservation(preflight_sha)
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


def _recover_after_known_deck_blocker(
        driver: ProofTransactionDriver, blocker: str,
        phase_inputs: dict[str, Any], phase_nonce: str,
        recover: Callable[[], dict[str, Any] | None],
        admit: Callable[[dict[str, Any]], dict[str, Any]],
        recover_diagnostic: Callable[[], dict[str, Any] | None],
        ) -> dict[str, Any]:
    """Preserve a durable Deck primary and admit its exact private diagnostic."""
    try:
        recovered = recover()
        if recovered is not None:
            return admit(recovered)
    except Exception as error:
        fail(f"RETURN_TO_DESIGN_GATE: PC0_EVIDENCE_BLOCKED: success-result recovery failed: {error}")
    try:
        retained = recover_diagnostic()
    except Exception as error:
        fail(f"RETURN_TO_DESIGN_GATE: PC0_EVIDENCE_BLOCKED: failure diagnostic admission failed: {error}")
    if retained is None:
        fail("RETURN_TO_DESIGN_GATE: PC0_EVIDENCE_BLOCKED: failure diagnostic is absent")
    diagnostic = retained["diagnostic"]
    if diagnostic["primary_blocker"] != blocker:
        fail("RETURN_TO_DESIGN_GATE: PC0_EVIDENCE_BLOCKED: diagnostic primary differs")
    driver.phase("execute_deck_batch", "failed", inputs=phase_inputs,
                 outputs={
                     "outward_blocker": blocker,
                     "primary_blocker": diagnostic["primary_blocker"],
                     "secondary_cleanup_blocker":
                         diagnostic["secondary_cleanup_blocker"],
                     "classification": diagnostic["classification"],
                     "failure_diagnostic_sha256": retained["sha256"],
                     "custody_disposition": retained["disposition"],
                 }, phase_nonce=phase_nonce)
    fail(blocker)


def _recover_after_unknown_deck_outcome(
        driver: ProofTransactionDriver, phase_inputs: dict[str, Any],
        phase_nonce: str, recover: Callable[[], dict[str, Any] | None],
        admit: Callable[[dict[str, Any]], dict[str, Any]],
        recover_diagnostic: Callable[[], dict[str, Any] | None],
        ) -> dict[str, Any]:
    """Perform one recovery read; never turn ambiguity into a second launch."""
    try:
        recovered = recover()
        if recovered is not None:
            return admit(recovered)
    except Exception:
        pass
    try:
        diagnostic = recover_diagnostic()
    except Exception:
        diagnostic = None
    if diagnostic is not None:
        retained = diagnostic["diagnostic"]
        driver.phase("execute_deck_batch", "failed", inputs=phase_inputs,
                     outputs={
                         "outward_blocker": retained["primary_blocker"],
                         "primary_blocker": retained["primary_blocker"],
                         "secondary_cleanup_blocker":
                             retained["secondary_cleanup_blocker"],
                         "classification": retained["classification"],
                         "failure_diagnostic_sha256": diagnostic["sha256"],
                         "custody_disposition": diagnostic["disposition"],
                     }, phase_nonce=phase_nonce)
        fail(retained["primary_blocker"])
    driver.phase("execute_deck_batch", "outcome_unresolved", inputs=phase_inputs,
                 outputs=None, phase_nonce=phase_nonce)
    fail("DX0_DECK_TRANSACTION_BLOCKED: Deck outcome is unresolved; no duplicate launch permitted")


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
    if driver.plan["plan_id"] == PC0_PLAN_ID:
        transfer = driver.state["phases"].get("transfer_and_admit_deck_inputs", {})
        preflight_sha = transfer.get("outputs", {}).get(
            "corrective_preflight_sha256"
        )
        expected_inputs = {
            "deck_execution_input_sha256": deck_input_sha,
            "proof_plan_sha256": driver.plan_sha,
            "phase_nonce": phase_nonce,
            "corrective_reservation": pc0_corrective_reservation(preflight_sha),
        }
        if (not isinstance(prior, dict)
                or prior.get("disposition") != "prepared"
                or prior.get("inputs") != expected_inputs
                or driver.effect_counts["deck_executions"] != 1):
            fail("RETURN_TO_DESIGN_GATE: corrective reservation/launch boundary differs")
    elif prior is not None or driver.effect_counts["deck_executions"] != 0:
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
    phase_inputs = ({
        "deck_execution_input_sha256": deck_input_sha,
        "proof_plan_sha256": driver.plan_sha,
        "phase_nonce": phase_nonce,
        "corrective_reservation": pc0_corrective_reservation(preflight_sha),
    } if driver.plan["plan_id"] == PC0_PLAN_ID else {
        "deck_execution_input_sha256": deck_input_sha,
        "proof_plan_sha256": driver.plan_sha,
        "phase_nonce": phase_nonce,
    })
    if driver.plan["plan_id"] != PC0_PLAN_ID:
        driver.phase("execute_deck_batch", "prepared", inputs=phase_inputs,
                     outputs=None, phase_nonce=phase_nonce)
        driver.effect_counts["deck_executions"] += 1
        driver.save()
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
        "PYTHONDONTWRITEBYTECODE=1 /usr/bin/python3 -B "
        "tools/wf0-factory-census/run.py execute "
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

    def admit_recovered_result(recovered: dict[str, Any]) -> dict[str, Any]:
        _validate_result_store_joins(recovered["result"], host, fixture,
                                     verified_handoff=handoff)
        _record_deck_result(driver, recovered["result"], deck_input_sha)
        return recovered["result"]

    try:
        ssh.run(command_line, timeout=900.0)
    except RemoteDeckBlocked as error:
        return _recover_after_known_deck_blocker(
            driver, error.blocker, phase_inputs, phase_nonce,
            lambda: _copy_result_to_mac(ssh, deck_input_sha, driver.plan_sha),
            admit_recovered_result,
            lambda: _copy_failure_diagnostic_to_mac(
                driver, ssh, deck_input_sha, driver.plan_sha, phase_nonce
            ),
        )
    except Exception:
        return _recover_after_unknown_deck_outcome(
            driver, phase_inputs, phase_nonce,
            lambda: _copy_result_to_mac(ssh, deck_input_sha, driver.plan_sha),
            admit_recovered_result,
            (lambda: _copy_failure_diagnostic_to_mac(
                driver, ssh, deck_input_sha, driver.plan_sha, phase_nonce
            )) if driver.plan["plan_id"] == PC0_PLAN_ID else (lambda: None),
        )
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


def _pc0_validate_consumed_reservation(
        driver: ProofTransactionDriver, deck_input_sha: str) -> dict[str, Any]:
    phase = driver.state["phases"].get("execute_deck_batch")
    transfer = driver.state["phases"].get("transfer_and_admit_deck_inputs")
    if not isinstance(phase, dict) or not isinstance(transfer, dict):
        fail("RETURN_TO_DESIGN_GATE: corrective reservation history is incomplete")
    preflight_sha = transfer.get("outputs", {}).get("corrective_preflight_sha256")
    expected = {
        "deck_execution_input_sha256": deck_input_sha,
        "proof_plan_sha256": driver.plan_sha,
        "phase_nonce": phase.get("phase_nonce"),
        "corrective_reservation": pc0_corrective_reservation(preflight_sha),
    }
    if (phase.get("inputs") != expected
            or phase.get("disposition") not in {
                "prepared", "in_flight", "recovered_in_flight", "completed",
                "failed", "outcome_unresolved"}
            or driver.effect_counts["deck_executions"] != 1
            or driver.effect_counts["windows_builds"] != 0
            or driver.effect_counts["artifact_downloads"] != 0
            or driver.effect_counts["custody_operations"] != 0
            or driver.effect_counts["artifact_transfers"] != 0
            or driver.effect_counts["source_transfers"] not in {0, 1}
            or driver.effect_counts["evidence_renders"] != 0):
        fail("RETURN_TO_DESIGN_GATE: consumed corrective reservation differs")
    return {"phase": phase, "preflight_sha256": preflight_sha,
            "preflight": transfer.get("outputs", {}).get("corrective_preflight")}


def _freeze_pc0_pre_evidence_snapshot(
        driver: ProofTransactionDriver, deck_input_sha: str,
        result_sha: str, preflight_sha: str) -> dict[str, Any]:
    live = driver.root / "DX0_TRANSACTION_STATE.json"
    data = live.read_bytes()
    value = parse_json_no_duplicates(data, "PC0 live pre-evidence journal")
    if canonical_json(value) != data:
        fail("PC0_EVIDENCE_BLOCKED: live pre-evidence journal is not canonical")
    driver._validate_state(value)
    validate_pre_evidence_snapshot(
        value, expected_source=driver.source_role,
        expected_operation_nonce=driver.state["operation_nonce"],
        expected_execution_input_sha256=deck_input_sha,
        expected_result_sha256=result_sha,
        expected_preflight_sha256=preflight_sha,
    )
    snapshot = driver.root / PC0_PRE_EVIDENCE_NAME
    sidecar = snapshot.with_suffix(snapshot.suffix + ".sha256")
    digest = sha256_bytes(data)
    expected_sidecar = f"{digest}  {snapshot.name}\n".encode()
    if (snapshot.exists() or snapshot.is_symlink()
            or sidecar.exists() or sidecar.is_symlink()):
        if (not snapshot.is_file() or snapshot.is_symlink()
                or not sidecar.is_file() or sidecar.is_symlink()
                or snapshot.read_bytes() != data
                or sidecar.read_bytes() != expected_sidecar):
            fail("PC0_EVIDENCE_BLOCKED: pre-evidence snapshot publication conflicts")
    else:
        write_atomic(snapshot, data)
        write_atomic(sidecar, expected_sidecar)
    accepted = validate_pre_evidence_snapshot_file(
        snapshot, expected_source=driver.source_role,
        expected_operation_nonce=driver.state["operation_nonce"],
        expected_execution_input_sha256=deck_input_sha,
        expected_result_sha256=result_sha,
        expected_preflight_sha256=preflight_sha,
        live_journal=live, require_live_equality=True,
    )
    return {"state": accepted, "sha256": digest, "path": snapshot}


def _pc0_corrective_history(
        driver: ProofTransactionDriver, admitted: dict[str, Any],
        snapshot: dict[str, Any], result: dict[str, Any],
        result_sha256: str, projected_effects: dict[str, int]) -> dict[str, Any]:
    journal = admitted["journal"]
    recovery = admitted["recovery"]
    state = snapshot["state"]
    preflight = state["phases"]["transfer_and_admit_deck_inputs"]["outputs"][
        "corrective_preflight"
    ]
    historical_remote = preflight["historical_failed_transaction"]
    effect_keys = (
        "windows_builds", "artifact_downloads", "custody_operations",
        "artifact_transfers", "source_transfers", "deck_executions",
        "evidence_renders",
    )
    current_effects = {key: projected_effects[key] for key in effect_keys}
    historical_effects = {key: journal["effect_counts"][key] for key in effect_keys}
    history = {
        "schema": PC0_CORRECTIVE_HISTORY_SCHEMA,
        "v2_failed_execution": {
            "transaction_id": journal["operation_nonce"],
            "journal_sha256": admitted["journal_sha256"],
            "source": journal["source"],
            "deck_execution_input_sha256":
                journal["phases"]["execute_deck_batch"]["inputs"][
                    "deck_execution_input_sha256"],
            "proof_plan_sha256": journal["plan_sha256"],
            "execute_phase_disposition":
                journal["phases"]["execute_deck_batch"]["disposition"],
            "transaction_state": journal["state"],
            "primary_blocker":
                journal["phases"]["execute_deck_batch"]["outputs"]["blocker"],
            "failure_classification": "unresolved_v2_no_diagnostic",
            "local_result_disposition": "absent",
            "remote_preflight": {
                "result": historical_remote["result"],
                "result_sidecar": historical_remote["result_sidecar"],
                "inner_lock": historical_remote["inner_lock"],
                "outer_lock": historical_remote["outer_lock"],
            },
            "effect_counts": historical_effects,
            "driver_invocation_count": journal["run_invocation_count"],
        },
        "v2_operator_recovery": {
            "receipt_sha256": admitted["recovery_sha256"],
            "continuation_count":
                recovery["operator_authorized_recovery_continuation_count"],
            "original_driver_invocation_count":
                recovery["original_driver_run_invocation_count"],
            "original_failure_boundary": recovery["original_failure_boundary"],
            "disposition": recovery["disposition"],
            "runtime_discovery_comment_id": PC0_RUNTIME_DISCOVERY_COMMENT_ID,
        },
        "v3_corrective_authority": {
            "merge_commit": PC0_V3_AUTHORITY_COMMIT,
            "merge_tree": PC0_V3_AUTHORITY_TREE,
            "design_blob": PC0_V3_DESIGN_BLOB,
            "design_sha256": PC0_V3_DESIGN_SHA256,
            "review_id": PC0_V3_REVIEW_ID,
            "approval_blob": PC0_V3_APPROVAL_BLOB,
            "prior_journal_sha256": admitted["journal_sha256"],
            "prior_recovery_receipt_sha256": admitted["recovery_sha256"],
            "additional_positive_deck_batches_maximum": 1,
            "additional_windows_builds_maximum": 0,
        },
        "v3_corrective_execution": {
            "transaction_id": state["operation_nonce"],
            "pre_evidence_journal_sha256": snapshot["sha256"],
            "execution_source": result["deck_execution_source"],
            "evidence_consumer_source": driver.source_role,
            "driver_invocation_count": state["run_invocation_count"],
            "reservation_count": int(
                state["phases"]["execute_deck_batch"]["inputs"].get(
                    "corrective_reservation") is not None
            ),
            "effect_counts": current_effects,
            "result_sha256": result_sha256,
        },
        "cumulative_external_effect_counts": {
            **{key: historical_effects[key] + current_effects[key]
               for key in effect_keys},
            "workflow_dispatches": int(
                journal["phases"]["reuse_or_produce_host"]["disposition"]
                == "completed" and historical_effects["windows_builds"] == 1
            ),
            "again_builds": 0, "fixture_seeds": 0,
            "live_negative_exercises": 0,
        },
        "cumulative_orchestration_counts": {
            "v2_driver_invocations": journal["run_invocation_count"],
            "operator_recovery_continuations":
                recovery["operator_authorized_recovery_continuation_count"],
            "v3_driver_invocations": state["run_invocation_count"],
            "total_orchestration_entries": (
                journal["run_invocation_count"]
                + recovery["operator_authorized_recovery_continuation_count"]
                + state["run_invocation_count"]
            ),
        },
    }
    return history


def run_transaction(driver: ProofTransactionDriver) -> dict[str, Any]:
    deterministic = driver.validate_local()
    driver.freeze_source()
    fixture = driver.verify_fixture()
    if (driver.plan["plan_id"] == PC0_PLAN_ID
            and fixture.get("identity_sha256") != PC0_FIXTURE_IDENTITY_SHA256):
        fail("RETURN_TO_DESIGN_GATE: accepted PC0 Mac fixture store differs")
    cached_host = (driver._pc0_exact_host_cache()
                   if driver.plan["plan_id"] == PC0_PLAN_ID
                   else driver._matching_host_cache())
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
            "windows_builds": (0 if driver.plan["plan_id"] == PC0_PLAN_ID
                               else 0 if cached_host else 1),
            "artifact_downloads": (0 if driver.plan["plan_id"] == PC0_PLAN_ID
                                    else 0 if cached_host else 1),
            "custody_operations": (0 if driver.plan["plan_id"] == PC0_PLAN_ID
                                    else 0 if cached_host else 1),
            "fixture_seeds": 0,
            "source_transfers": "zero_or_one_if_live_execution_required",
            "artifact_transfers": (0 if driver.plan["plan_id"] == PC0_PLAN_ID
                                   else "zero_or_one_if_deck_cache_misses"),
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

    execution_intent: dict[str, Any] | None = None
    execution_lock: pathlib.Path | None = None
    result: dict[str, Any] | None = None
    reused_publication = False
    historical_authority: dict[str, Any] | None = None

    if driver.plan["plan_id"] == PC0_PLAN_ID:
        phase = driver.state["phases"].get("execute_deck_batch")
        reservation_consumed = phase is not None or driver.effect_counts["deck_executions"] != 0
        if reservation_consumed:
            consumed = _pc0_validate_consumed_reservation(driver, deck_input_sha)
            historical_authority = pc0_admit_historical_authority(driver.proof_root)
            driver._pc0_scan_corrective_history()
            execution_intent = {
                "deck_execution_input_sha256": deck_input_sha,
                "proof_plan_sha256": driver.plan_sha,
                "operation_nonce": driver.state["operation_nonce"],
            }
            execution_lock = (dx0_mac_result_parent() / ".locks" /
                              f"{deck_input_sha}-{driver.plan_sha}")
            result = read_mac_result()
            if result is not None and not (execution_lock.exists()
                                           or execution_lock.is_symlink()):
                execution_lock = None
            if result is None:
                if (not execution_lock.exists() and not execution_lock.is_symlink()):
                    fail("RETURN_TO_DESIGN_GATE: consumed corrective Mac lock is absent")
                driver.validate_single_writer(execution_lock, execution_intent)
                result = recover_remote_result()
            if result is None:
                diagnostic = _copy_failure_diagnostic_to_mac(
                    driver, get_ssh(), deck_input_sha, driver.plan_sha,
                    consumed["phase"]["phase_nonce"],
                )
                if diagnostic is None:
                    fail("DX0_DECK_TRANSACTION_BLOCKED: consumed corrective outcome is unresolved")
                retained = diagnostic["diagnostic"]
                expected_outputs = {
                    "outward_blocker": retained["primary_blocker"],
                    "primary_blocker": retained["primary_blocker"],
                    "secondary_cleanup_blocker":
                        retained["secondary_cleanup_blocker"],
                    "classification": retained["classification"],
                    "failure_diagnostic_sha256": diagnostic["sha256"],
                    "custody_disposition": diagnostic["disposition"],
                }
                if consumed["phase"]["disposition"] == "failed":
                    if consumed["phase"].get("outputs") != expected_outputs:
                        fail("RETURN_TO_DESIGN_GATE: retained failure phase differs")
                else:
                    driver.phase(
                        "execute_deck_batch", "failed",
                        inputs=consumed["phase"]["inputs"],
                        outputs=expected_outputs,
                        phase_nonce=consumed["phase"]["phase_nonce"],
                    )
                fail(retained["primary_blocker"])
            _validate_result_store_joins(result, host, fixture)
            _record_deck_result(driver, result, deck_input_sha)
        else:
            ssh = get_ssh()
            handoff = _source_handoff_and_admission(driver, ssh)
            preflight = _admit_deck_inputs(
                driver, ssh, host, fixture, handoff, deck_input, deck_input_sha
            )
            historical_authority = driver.require_pc0_corrective_authority(
                preflight["receipt"], preflight["sha256"], host
            )
            execution_intent, execution_lock, result, may_start_execution = (
                _reconcile_deck_execution_writer(
                    driver, deck_input_sha, read_mac_result, recover_remote_result
                )
            )
            if result is not None or execution_lock is None or not may_start_execution:
                fail("RETURN_TO_DESIGN_GATE: corrective publication changed after preflight")
            driver.reserve_pc0_corrective(deck_input_sha, preflight["sha256"])
            result = _run_or_retrieve_deck(
                driver, ssh, host, fixture, handoff, deck_input, deck_input_sha
            )
    else:
        execution_intent, execution_lock, result, may_start_execution = (
            _reconcile_deck_execution_writer(
                driver, deck_input_sha, read_mac_result, recover_remote_result
            )
        )
        reused_publication = result is not None
        if result is None:
            if execution_lock is None or not may_start_execution:
                fail("DX0_DECK_TRANSACTION_BLOCKED: persisted execution outcome is unresolved")
            ssh = get_ssh()
            handoff = _source_handoff_and_admission(driver, ssh)
            _admit_deck_inputs(
                driver, ssh, host, fixture, handoff, deck_input, deck_input_sha
            )
            result = _run_or_retrieve_deck(
                driver, ssh, host, fixture, handoff, deck_input, deck_input_sha
            )

    assert result is not None
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
        result, consumer_source=driver.source_role, renderer=driver.renderer,
        original_observation=(driver.plan["plan_id"] == PC0_PLAN_ID and not reused_publication
                              and result["operation_nonce"] == driver.state["operation_nonce"])
    )
    retrieve_inputs = {"deck_execution_input_sha256": deck_input_sha}
    retrieve_outputs = {
        "retained_result_sha256": admission["retained_result_sha256"],
        "observation_disposition": admission["disposition"],
        "result_admission": admission,
    }
    retrieve = driver.phase(
        "retrieve_and_retain_result",
        "completed" if result["deck_execution_source"] == driver.source_role else "reused",
        inputs=retrieve_inputs, outputs=retrieve_outputs,
    )
    if (retrieve.get("inputs") != retrieve_inputs
            or retrieve.get("outputs") != retrieve_outputs):
        fail("PC0_EVIDENCE_BLOCKED: retained result-admission phase differs")
    pre_evidence_snapshot: dict[str, Any] | None = None
    if driver.plan["plan_id"] == PC0_PLAN_ID:
        consumed = _pc0_validate_consumed_reservation(driver, deck_input_sha)
        pre_evidence_snapshot = _freeze_pc0_pre_evidence_snapshot(
            driver, deck_input_sha, admission["retained_result_sha256"],
            consumed["preflight_sha256"],
        )
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
    invalidation_results = dict(deterministic["invalidation_results"])
    if driver.plan["plan_id"] == PC0_PLAN_ID:
        if historical_authority is None or pre_evidence_snapshot is None:
            fail("PC0_EVIDENCE_BLOCKED: corrective evidence inputs are absent")
        invalidation_results["corrective_history"] = _pc0_corrective_history(
            driver, historical_authority, pre_evidence_snapshot, result,
            admission["retained_result_sha256"], projected_costs,
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
        "invalidation_results": invalidation_results,
    }
    if driver.plan["plan_id"] == PC0_PLAN_ID:
        from negative_tests import pc0_renderer_reuse_proof
        transaction["renderer_only_reuse"] = pc0_renderer_reuse_proof(
            result, driver, transaction)
        assert pre_evidence_snapshot is not None
        consumed = _pc0_validate_consumed_reservation(driver, deck_input_sha)
        validate_pre_evidence_snapshot_file(
            pre_evidence_snapshot["path"], expected_source=driver.source_role,
            expected_operation_nonce=driver.state["operation_nonce"],
            expected_execution_input_sha256=deck_input_sha,
            expected_result_sha256=admission["retained_result_sha256"],
            expected_preflight_sha256=consumed["preflight_sha256"],
            live_journal=driver.root / "DX0_TRANSACTION_STATE.json",
            require_live_equality=True,
        )
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
    if driver.plan["plan_id"] == PC0_PLAN_ID:
        assert pre_evidence_snapshot is not None
        validate_pre_evidence_snapshot_file(
            pre_evidence_snapshot["path"], expected_source=driver.source_role,
            expected_operation_nonce=driver.state["operation_nonce"],
            expected_execution_input_sha256=deck_input_sha,
            expected_result_sha256=admission["retained_result_sha256"],
            expected_preflight_sha256=consumed["preflight_sha256"],
            require_live_equality=False,
        )
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

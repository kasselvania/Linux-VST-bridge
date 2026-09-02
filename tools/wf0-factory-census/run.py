#!/usr/bin/env python3
"""Deck-only dependency-ordered WF0 V7 execution and evidence transaction."""

from __future__ import annotations

import argparse
import json
import pathlib
import secrets
import subprocess
import sys
import time
from typing import Any

sys.dont_write_bytecode = True

from artifacts import (
    create_evidence_handoff, read_canonical_json, verify_evidence_packet,
    verify_execution_source, verify_hash_sidecar,
)
from common import (
    APPROVAL_BLOB, BASIS_COMMIT, BASIS_TREE, BUNDLE_SCHEMA, DESIGN_BLOB,
    DESIGN_SHA256, EVIDENCE_FILES, EXPECTED_BRANCH, FAULT_TARGETS, REVIEW_BLOB,
    RUNNER_DIGEST, SOURCE_HANDOFF_SCHEMA, WINDOWS_BUILD_SCHEMA,
    artifact_cache_parent, canonical_json, command_text, deck_fixture_identity,
    fail, process_census, process_guard, protected_snapshot, repo_root,
    sha256_bytes, sha256_file,
    source_handoff_parent, source_manifest_sha256, verify_runner_identity,
    write_atomic,
)
from environment import create_environment, retire_environment, verify_artifact_cache
from evidence import render_packet, validate_packet
from negative_tests import run_negative_suite
from normalize import normalize_positive
from supervise import supervise


def _bundle_identity(manifest: dict[str, Any]) -> dict[str, Any]:
    records = [
        {
            "path": item["path"][len("again.vst3/"):],
            "size": item["size"],
            "sha256": item["sha256"],
        }
        for item in manifest["records"]
        if item["path"].startswith("again.vst3/")
    ]
    records.sort(key=lambda item: item["path"].encode())
    value = {
        "schema": BUNDLE_SCHEMA,
        "binary_safe_path": "again.vst3/Contents/x86_64-win/again.vst3",
        "records": records,
    }
    value["sha256"] = sha256_bytes(canonical_json(value))
    return value


def _source_handoff(source_commit: str, source_tree: str,
                    source_manifest_digest: str) -> dict[str, Any]:
    root = source_handoff_parent() / source_commit
    if not root.is_dir() or root.is_symlink():
        fail("persistent source handoff root is absent or unsafe")
    receipt_path = root / "WF0_SOURCE_HANDOFF_RECEIPT.json"
    verify_hash_sidecar(root / "WF0_SOURCE_HANDOFF_RECEIPT.sha256", receipt_path)
    receipt = read_canonical_json(receipt_path, SOURCE_HANDOFF_SCHEMA)
    bundle = root / receipt.get("bundle", {}).get("name", "")
    advertised = f"refs/handoff/wf0-v7-source/{source_commit}"
    if (
        receipt.get("implementation_source", {}).get("commit") != source_commit
        or receipt.get("implementation_source", {}).get("tree") != source_tree
        or receipt.get("implementation_source", {}).get("parent") != BASIS_COMMIT
        or receipt.get("implementation_source", {}).get("manifest_sha256")
        != source_manifest_digest
        or receipt.get("bundle", {}).get("advertised_ref") != advertised
        or not bundle.is_file() or bundle.is_symlink()
        or sha256_file(bundle) != receipt.get("bundle", {}).get("sha256")
        or command_text(["git", "rev-parse", advertised], cwd=repo_root())
        != source_commit
    ):
        fail("persistent source handoff/ref/worktree join differs")
    return receipt


def load_build(source_commit: str, artifact_digest: str) -> dict[str, Any]:
    root = artifact_cache_parent() / artifact_digest
    manifest = read_canonical_json(root / "ARTIFACT_MANIFEST.json")
    if sha256_file(root / "ARTIFACT_MANIFEST.json") != artifact_digest:
        fail("artifact cache manifest path/digest differs")
    core = read_canonical_json(
        root / "BUILD_IDENTITY_CORE.json",
        "linux-vst-bridge-wf0-build-identity-core/v1",
    )
    receipt = read_canonical_json(
        root / "WF0_WINDOWS_BUILD_RECEIPT.json", WINDOWS_BUILD_SCHEMA
    )
    custody = read_canonical_json(
        root / "WF0_MAC_ARTIFACT_CUSTODY_RECEIPT.json",
        "linux-vst-bridge-wf0-mac-artifact-custody/v1",
    )
    source = core.get("source", {}).get("implementation_source_manifest")
    source_digest = core.get("source", {}).get(
        "implementation_source_manifest_sha256"
    )
    source_tree = core.get("source", {}).get("tree")
    if (
        core.get("source", {}).get("commit") != source_commit
        or source_manifest_sha256(source) != source_digest
        or receipt.get("source", {}).get("commit") != source_commit
        or receipt.get("source", {}).get("tree") != source_tree
        or receipt.get("source", {}).get("implementation_source_manifest", {}).get("sha256")
        != source_digest
        or receipt.get("artifacts", {}).get("artifact_manifest_sha256")
        != artifact_digest
        or custody.get("inner_envelope", {}).get("artifact_manifest_sha256")
        != artifact_digest
    ):
        fail("source/build/artifact/custody identity join differs")
    verify_execution_source(source_commit, source_tree, source_digest)
    handoff = _source_handoff(source_commit, source_tree, source_digest)
    build = {
        "source_commit": source_commit,
        "source_tree": source_tree,
        "implementation_source_manifest": source,
        "implementation_source_manifest_sha256": source_digest,
        "artifact_root": str(root),
        "artifact_manifest": manifest,
        "artifact_manifest_sha256": artifact_digest,
        "artifact_set": {"id": artifact_digest, "records": manifest["records"]},
        "again_bundle_manifest": _bundle_identity(manifest),
        "fault_targets": list(FAULT_TARGETS),
        "build_identity_core": core,
        "build_receipt": receipt,
        "mac_custody": custody,
        "source_handoff": handoff,
        "toolchain": receipt["toolchain"],
        "sdk": receipt["vst3_sdk"],
        "builds": {"comparison": receipt["build"]["comparison"]},
    }
    verify_artifact_cache(build)
    return build


def verify_authority() -> dict[str, Any]:
    root = repo_root()
    observed = {
        "basis_commit": BASIS_COMMIT,
        "basis_tree": command_text(
            ["git", "rev-parse", f"{BASIS_COMMIT}^{{tree}}"], cwd=root
        ),
        "design_blob": command_text(
            ["git", "rev-parse", "HEAD:docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md"],
            cwd=root,
        ),
        "review_blob": command_text(
            ["git", "rev-parse", "HEAD:docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V7.md"],
            cwd=root,
        ),
        "approval_blob": command_text(
            ["git", "rev-parse", "HEAD:docs/slices/WF0/DESIGN_APPROVAL_V7.md"],
            cwd=root,
        ),
        "design_sha256": sha256_file(
            root / "docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md"
        ),
    }
    if observed != {
        "basis_commit": BASIS_COMMIT,
        "basis_tree": BASIS_TREE,
        "design_blob": DESIGN_BLOB,
        "review_blob": REVIEW_BLOB,
        "approval_blob": APPROVAL_BLOB,
        "design_sha256": DESIGN_SHA256,
    }:
        fail(f"V7 implementation authority differs: {observed}")
    current_slice = (root / "CURRENT_SLICE.md").read_text(encoding="utf-8")
    if (
        "authority_phase: implementation" not in current_slice
        or "implementation_authorized: true" not in current_slice
    ):
        fail("CURRENT_SLICE implementation authority is absent")
    return observed


def reverify(build: dict[str, Any]) -> None:
    verify_execution_source(
        build["source_commit"], build["source_tree"],
        build["implementation_source_manifest_sha256"],
    )
    verify_artifact_cache(build)
    verify_runner_identity()
    protected_snapshot()
    process_guard()


def preflight(build: dict[str, Any]) -> dict[str, Any]:
    if sys.executable != "/usr/bin/python3":
        fail(f"Deck execution requires /usr/bin/python3; observed {sys.executable}")
    authority = verify_authority()
    fixture = deck_fixture_identity()
    reverify(build)
    return {
        "authority": authority,
        "fixture": fixture,
        "runner": verify_runner_identity(),
        "source_commit": build["source_commit"],
        "source_tree": build["source_tree"],
        "source_manifest_sha256": build["implementation_source_manifest_sha256"],
        "artifact_manifest_sha256": build["artifact_manifest_sha256"],
        "protected_snapshot": protected_snapshot(),
        "no_deck_github": {
            "github_operations": 0,
            "github_credentials_received": False,
            "ssh_agent_forwarded": False,
        },
    }


def run_held(build: dict[str, Any]) -> dict[str, Any]:
    reverify(build)
    sentinel = subprocess.Popen(
        [
            "/usr/bin/bash", "-c",
            "exec -a wf0-factory-probe-sentinel.exe /usr/bin/sleep 300",
        ],
        stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL, start_new_session=True,
    )
    sentinel_identity = None
    deadline = time.monotonic() + 2.0
    while time.monotonic() < deadline:
        matches = [
            item for item in process_census()
            if item["pid"] == sentinel.pid
            and "wf0-factory-probe-sentinel.exe" in item["cmdline"]
        ]
        if len(matches) == 1:
            sentinel_identity = matches[0]
            break
        time.sleep(0.05)
    if sentinel_identity is None:
        sentinel.terminate()
        sentinel.wait(timeout=5)
        fail("same-family cleanup sentinel was not observable")
    environment = None
    retirement = None
    try:
        environment = create_environment(secrets.token_hex(16), build, fixture="again")
        result = supervise(environment, hold_gate=True)
        if (
            result["classification"] != "held_gate_proof_complete"
            or result["blocker"] is not None
            or result["held_gate"] != {
                "module_open_started": False,
                "load_library_attempted": False,
                "again_module_mapped": False,
                "factory_or_class_event": False,
                "scanner_waiting": True,
                "held_seconds": 15.0,
            }
        ):
            fail("held-gate exercise did not complete its exact causal absence proof")
    finally:
        try:
            if environment is not None:
                retirement = retire_environment(environment)
        finally:
            live = [
                item for item in process_census()
                if item["pid"] == sentinel_identity["pid"]
                and item["start_ticks"] == sentinel_identity["start_ticks"]
            ]
            if len(live) == 1:
                sentinel.terminate()
                try:
                    sentinel.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    sentinel.kill()
                    sentinel.wait(timeout=5)
            else:
                sentinel.wait(timeout=5)
    if not live:
        fail("scoped WF0 cleanup terminated the unrelated same-family sentinel")
    if retirement is None:
        fail("held-gate environment did not reach retirement")
    result["retirement"] = retirement
    result["unrelated_same_family_sentinel_survived"] = True
    process_guard()
    return result


def run_positive(build: dict[str, Any]) -> dict[str, Any]:
    reverify(build)
    environment = create_environment(secrets.token_hex(16), build, fixture="again")
    try:
        result = supervise(environment)
        if (environment.session / "create-instance-tripwire.marker").exists():
            fail("positive run created the prohibited instantiation tripwire")
        if result["classification"] != "scanner_completed" or result["blocker"] is not None:
            fail(f"positive AGain run did not complete: {result['classification']}")
    finally:
        retirement = retire_environment(environment)
    result["retirement"] = retirement
    process_guard()
    return result


def receipt_root(build: dict[str, Any]) -> pathlib.Path:
    return (
        pathlib.Path.home()
        / ".local/share/linux-vst-bridge/handoffs/wf0/execution/by-source"
        / build["source_commit"]
        / build["artifact_manifest_sha256"]
    )


def _write_receipt(root: pathlib.Path, name: str, value: dict[str, Any]) -> None:
    write_atomic(root / name, canonical_json(value))


def run_all(build: dict[str, Any], evidence_handoff: pathlib.Path) -> dict[str, Any]:
    receipts = receipt_root(build)
    if receipts.exists() or receipts.is_symlink():
        fail("WF0 execution receipt root already exists; reuse or repair is prohibited")
    receipts.mkdir(parents=True)
    initial = preflight(build)
    _write_receipt(receipts, "preflight.json", initial)

    held = run_held(build)
    _write_receipt(receipts, "held-gate.json", held)

    negative = run_negative_suite(build, repo_root(), lambda: reverify(build))
    _write_receipt(receipts, "negative.json", negative)

    positive = run_positive(build)
    _write_receipt(receipts, "positive.json", positive)

    reverify(build)
    census, timeline = normalize_positive(positive, build)
    final_protected = protected_snapshot()
    if final_protected != initial["protected_snapshot"]:
        fail("protected state differs across the complete WF0 execution transaction")
    reverify(build)
    render_packet(
        build["source_commit"], build, negative, held, positive, census, timeline,
        initial,
    )
    validate_packet(build["source_commit"], treeish=build["source_commit"])
    records = verify_evidence_packet(
        repo_root() / "evidence/wf0-windows-vst3-factory-census"
    )
    handoff_identity = {
        "implementation_source": {
            "commit": build["source_commit"], "tree": build["source_tree"],
            "manifest_sha256": build["implementation_source_manifest_sha256"],
        },
        "source_handoff": {
            "bundle_name": build["source_handoff"]["bundle"]["name"],
            "bundle_sha256": build["source_handoff"]["bundle"]["sha256"],
            "advertised_ref": build["source_handoff"]["bundle"]["advertised_ref"],
            "receipt_sha256": sha256_bytes(
                canonical_json(build["source_handoff"])
            ),
            "detached_worktree": True,
        },
        "windows_build": {
            "workflow_run_id": build["build_receipt"]["workflow"]["run_id"],
            "workflow_run_attempt": build["build_receipt"]["workflow"]["run_attempt"],
            "receipt_sha256": sha256_bytes(canonical_json(build["build_receipt"])),
        },
        "mac_artifact_custody_receipt_sha256": sha256_bytes(
            canonical_json(build["mac_custody"])
        ),
        "artifact_manifest_sha256": build["artifact_manifest_sha256"],
        "payload_archive_sha256":
            build["build_receipt"]["artifacts"]["payload_archive_sha256"],
        "runtime_proton_digest": RUNNER_DIGEST,
        "exercise_receipt_sha256": {
            "held_gate": sha256_bytes(canonical_json(held)),
            "negative": sha256_bytes(canonical_json(negative)),
            "positive": sha256_bytes(canonical_json(positive)),
        },
        "no_deck_github": initial["no_deck_github"],
        "protected_state_equal": True,
    }
    handoff = create_evidence_handoff(
        repo_root() / "evidence/wf0-windows-vst3-factory-census",
        evidence_handoff,
        handoff_identity,
    )
    process_guard()
    if any(
        item.name.startswith(".wf0-factory-census.stage-")
        for item in pathlib.Path.home().joinpath(
            ".local/share/linux-vst-bridge/environments"
        ).iterdir()
    ):
        fail("WF0 stage root remains after evidence handoff")
    return {
        "source_commit": build["source_commit"],
        "artifact_manifest_sha256": build["artifact_manifest_sha256"],
        "evidence_record_count": len(records),
        "evidence_handoff": handoff,
        "protected_state_equal": True,
        "no_deck_github": initial["no_deck_github"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "phase", choices=("preflight", "held-gate", "negative", "positive",
                          "evidence", "validate", "all")
    )
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--artifact-manifest-sha256", required=True)
    parser.add_argument("--evidence-handoff", type=pathlib.Path)
    args = parser.parse_args()
    build = load_build(args.source_commit, args.artifact_manifest_sha256)

    if args.phase == "preflight":
        result = preflight(build)
    elif args.phase == "validate":
        validate_packet(args.source_commit, treeish=args.source_commit)
        result = {"evidence_packet": "valid"}
    elif args.phase == "all":
        if args.evidence_handoff is None:
            fail("all requires --evidence-handoff")
        result = run_all(build, args.evidence_handoff.resolve())
    else:
        fail("individual mutating phases are intentionally available only through one fresh all transaction")
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"WF0_ERROR: {error}", file=sys.stderr, flush=True)
        raise

#!/usr/bin/env python3
"""Deck-only dependency-ordered WA0 execution and evidence transaction."""

from __future__ import annotations

import argparse
import json
import pathlib
import secrets
import sys
from typing import Any

sys.dont_write_bytecode = True

from artifacts import (
    create_evidence_handoff, read_canonical_json, verify_evidence_packet,
    verify_execution_source, verify_hash_sidecar,
)
from common import (
    ACCEPTED_WC0_ARTIFACT_MANIFEST_SHA256, ACCEPTED_WC0_EVIDENCE_COMMIT,
    ACCEPTED_WC0_EVIDENCE_TREE, ACCEPTED_WC0_IMPLEMENTATION_MERGE,
    ACCEPTED_WC0_SCANNER_SHA256, ACCEPTED_WC0_SOURCE_COMMIT,
    ACCEPTED_WC0_SOURCE_MANIFEST_SHA256, ACCEPTED_WC0_SOURCE_TREE,
    APPROVAL_BLOB, AUTHORITY_MERGE_COMMIT, AUTHORITY_MERGE_TREE, BASIS_COMMIT,
    BASIS_TREE, BUNDLE_SCHEMA, DESIGN_BLOB, DESIGN_COMMIT, DESIGN_SHA256,
    DESIGN_TREE, EVIDENCE_FILES, EXPECTED_BRANCH, EXPECTED_REF, FAULT_TARGETS,
    MAC_CUSTODY_SCHEMA, REPOSITORY, REVIEW_GITHUB_ID, RUNNER_DIGEST,
    SOURCE_HANDOFF_SCHEMA, SOURCE_SCHEMA, WINDOWS_BUILD_SCHEMA,
    artifact_cache_parent, canonical_json, command_text, deck_fixture_identity,
    fail, process_guard, protected_snapshot, repo_root,
    sha256_bytes, sha256_file,
    source_handoff_parent, source_manifest_sha256, verify_runner_identity,
    write_atomic,
)
from environment import create_environment, retire_environment, verify_artifact_cache
from evidence import render_packet, validate_packet
from negative_tests import run_negative_suite
from normalize import normalize_wa0_positive
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
    receipt_path = root / "WA0_SOURCE_HANDOFF_RECEIPT.json"
    verify_hash_sidecar(root / "WA0_SOURCE_HANDOFF_RECEIPT.sha256", receipt_path)
    receipt = read_canonical_json(receipt_path, SOURCE_HANDOFF_SCHEMA)
    bundle = root / receipt.get("bundle", {}).get("name", "")
    advertised = f"refs/handoff/wa0-source/{source_commit}"
    implementation = receipt.get("implementation_source", {})
    authority = receipt.get("wa0_authority", {})
    bundle_identity = receipt.get("bundle", {})
    if (
        receipt.get("repository") != REPOSITORY
        or authority != {
            "design_authority_commit": AUTHORITY_MERGE_COMMIT,
            "design_authority_tree": AUTHORITY_MERGE_TREE,
            "implementation_basis_commit": BASIS_COMMIT,
            "implementation_basis_tree": BASIS_TREE,
        }
        or implementation != {
            "commit": source_commit,
            "tree": source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "manifest_schema": SOURCE_SCHEMA,
            "manifest_record_count": 17,
            "manifest_sha256": source_manifest_digest,
        }
        or not bundle.is_file() or bundle.is_symlink()
        or bundle_identity != {
            "name": f"wa0-execution-source-{source_commit}.bundle",
            "advertised_ref": advertised,
            "sha256": sha256_file(bundle),
            "size": bundle.stat().st_size,
            "max_size_bytes": 134217728,
            "git_bundle_verify": "passed",
            "self_contained": True,
            "prerequisite_count": 0,
        }
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
        MAC_CUSTODY_SCHEMA,
    )
    source = core.get("source", {}).get("implementation_source_manifest")
    source_digest = core.get("source", {}).get(
        "implementation_source_manifest_sha256"
    )
    source_tree = core.get("source", {}).get("tree")
    source_identity = {
        "commit": source_commit,
        "tree": source_tree,
        "parent": BASIS_COMMIT,
        "branch": EXPECTED_BRANCH,
        "ref": EXPECTED_REF,
        "schema": SOURCE_SCHEMA,
        "record_count": 17,
        "manifest_sha256": source_digest,
    }
    if (
        core.get("source") != {
            "commit": source_commit,
            "tree": source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "implementation_source_schema": SOURCE_SCHEMA,
            "implementation_source_record_count": 17,
            "implementation_source_manifest": source,
            "implementation_source_manifest_sha256": source_digest,
        }
        or source_manifest_sha256(source) != source_digest
        or receipt.get("source") != {
            "commit": source_commit,
            "tree": source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "implementation_source_manifest": {
                "schema": SOURCE_SCHEMA,
                "record_count": 17,
                "sha256": source_digest,
            },
        }
        or custody.get("implementation_source") != source_identity
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
        "fault_targets": [*FAULT_TARGETS, "wf0-no-entry"],
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
    wc0_retained = read_canonical_json(
        root / "evidence/wc0-windows-vst3-processor-component-admission"
        / "BUILD_MANIFEST.json",
        "linux-vst-bridge-wc0-retained-build-manifest/v1",
    )
    wc0_scanner = next(
        (
            item.get("sha256")
            for item in wc0_retained.get("artifact_manifest", {}).get("records", [])
            if item.get("path") == "bin/wf0-factory-probe.exe"
        ),
        None,
    )
    observed = {
        "basis_commit": BASIS_COMMIT,
        "basis_tree": command_text(
            ["git", "rev-parse", f"{BASIS_COMMIT}^{{tree}}"], cwd=root
        ),
        "authority_merge_commit": AUTHORITY_MERGE_COMMIT,
        "authority_merge_tree": command_text(
            ["git", "rev-parse", f"{AUTHORITY_MERGE_COMMIT}^{{tree}}"], cwd=root
        ),
        "design_commit": DESIGN_COMMIT,
        "design_tree": command_text(
            ["git", "rev-parse", f"{DESIGN_COMMIT}^{{tree}}"], cwd=root
        ),
        "design_blob": command_text(
            ["git", "rev-parse", "HEAD:docs/slices/WA0/IMPLEMENTATION_DESIGN.md"],
            cwd=root,
        ),
        "approval_blob": command_text(
            ["git", "rev-parse", "HEAD:docs/slices/WA0/DESIGN_APPROVAL.md"],
            cwd=root,
        ),
        "design_sha256": sha256_file(
            root / "docs/slices/WA0/IMPLEMENTATION_DESIGN.md"
        ),
        "accepted_wc0": {
            "implementation_merge": ACCEPTED_WC0_IMPLEMENTATION_MERGE,
            "implementation_merge_tree": command_text(
                ["git", "rev-parse", f"{ACCEPTED_WC0_IMPLEMENTATION_MERGE}^{{tree}}"],
                cwd=root,
            ),
            "source_commit": ACCEPTED_WC0_SOURCE_COMMIT,
            "source_tree": command_text(
                ["git", "rev-parse", f"{ACCEPTED_WC0_SOURCE_COMMIT}^{{tree}}"],
                cwd=root,
            ),
            "evidence_commit": ACCEPTED_WC0_EVIDENCE_COMMIT,
            "evidence_tree": command_text(
                ["git", "rev-parse", f"{ACCEPTED_WC0_EVIDENCE_COMMIT}^{{tree}}"],
                cwd=root,
            ),
            "source_manifest_sha256":
                wc0_retained.get("implementation_source_manifest_sha256"),
            "scanner_sha256": wc0_scanner,
            "artifact_manifest_sha256": wc0_retained.get(
                "deck_admission", {}
            ).get("artifact_cache_manifest_sha256"),
        },
    }
    if observed != {
        "basis_commit": BASIS_COMMIT,
        "basis_tree": BASIS_TREE,
        "authority_merge_commit": AUTHORITY_MERGE_COMMIT,
        "authority_merge_tree": AUTHORITY_MERGE_TREE,
        "design_commit": DESIGN_COMMIT,
        "design_tree": DESIGN_TREE,
        "design_blob": DESIGN_BLOB,
        "approval_blob": APPROVAL_BLOB,
        "design_sha256": DESIGN_SHA256,
        "accepted_wc0": {
            "implementation_merge": ACCEPTED_WC0_IMPLEMENTATION_MERGE,
            "implementation_merge_tree": ACCEPTED_WC0_EVIDENCE_TREE,
            "source_commit": ACCEPTED_WC0_SOURCE_COMMIT,
            "source_tree": ACCEPTED_WC0_SOURCE_TREE,
            "evidence_commit": ACCEPTED_WC0_EVIDENCE_COMMIT,
            "evidence_tree": ACCEPTED_WC0_EVIDENCE_TREE,
            "source_manifest_sha256": ACCEPTED_WC0_SOURCE_MANIFEST_SHA256,
            "scanner_sha256": ACCEPTED_WC0_SCANNER_SHA256,
            "artifact_manifest_sha256":
                ACCEPTED_WC0_ARTIFACT_MANIFEST_SHA256,
        },
    }:
        fail(f"WA0 implementation authority differs: {observed}")
    current_slice = (root / "CURRENT_SLICE.md").read_text(encoding="utf-8")
    approval = (
        root / "docs/slices/WA0/DESIGN_APPROVAL.md"
    ).read_text(encoding="utf-8")
    if (
        "status: active_implementation_slice" not in current_slice
        or "authority_phase: implementation" not in current_slice
        or "implementation_authorized: true" not in current_slice
        or f"design_review: {REVIEW_GITHUB_ID} / DESIGN_CLEAR" not in current_slice
        or f"adversarial_review_github_id: {REVIEW_GITHUB_ID}" not in approval
        or "adversarial_review_result: DESIGN_CLEAR" not in approval
        or "implementation_authorized: true" not in approval
    ):
        fail("WA0 implementation authority readback is absent")
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


def run_positive(build: dict[str, Any]) -> dict[str, Any]:
    reverify(build)
    environment = create_environment(secrets.token_hex(16), build, fixture="again")
    try:
        result = supervise(environment)
        if (
            (environment.session / "forbidden-component-method.marker").exists()
            or (environment.session / "forbidden-audio-processor-method.marker").exists()
        ):
            fail("positive run invoked a method beyond the WA0 ceiling")
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
        / ".local/share/linux-vst-bridge/handoffs/wa0/execution/by-source"
        / build["source_commit"]
        / build["artifact_manifest_sha256"]
    )


def _write_receipt(root: pathlib.Path, name: str, value: dict[str, Any]) -> None:
    write_atomic(root / name, canonical_json(value))


def run_all(build: dict[str, Any], evidence_handoff: pathlib.Path) -> dict[str, Any]:
    receipts = receipt_root(build)
    if receipts.exists() or receipts.is_symlink():
        fail("WA0 execution receipt root already exists; reuse or repair is prohibited")
    receipts.mkdir(parents=True)
    initial = preflight(build)
    _write_receipt(receipts, "preflight.json", initial)

    negative = run_negative_suite(build, repo_root(), lambda: reverify(build))
    _write_receipt(receipts, "negative.json", negative)

    positive = run_positive(build)
    _write_receipt(receipts, "positive.json", positive)

    reverify(build)
    audio_processor_lease, component_session, timeline = normalize_wa0_positive(
        positive, build
    )
    final_protected = protected_snapshot()
    if final_protected != initial["protected_snapshot"]:
        fail("protected state differs across the complete WA0 execution transaction")
    reverify(build)
    render_packet(
        build["source_commit"], build, negative, positive, audio_processor_lease,
        component_session, timeline, initial,
    )
    validate_packet(build["source_commit"], treeish=build["source_commit"])
    records = verify_evidence_packet(
        repo_root() / "evidence/wa0-windows-vst3-audio-processor-interface-admission"
    )
    artifact = build["mac_custody"]["artifact"]
    handoff_identity = {
        "implementation_source": {
            "commit": build["source_commit"], "tree": build["source_tree"],
            "parent": BASIS_COMMIT, "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF, "schema": SOURCE_SCHEMA,
            "record_count": 17,
            "manifest_sha256": build["implementation_source_manifest_sha256"],
        },
        "source_handoff": {
            "schema": SOURCE_HANDOFF_SCHEMA,
            "bundle_name": build["source_handoff"]["bundle"]["name"],
            "bundle_sha256": build["source_handoff"]["bundle"]["sha256"],
            "advertised_ref": build["source_handoff"]["bundle"]["advertised_ref"],
            "receipt_sha256": sha256_bytes(
                canonical_json(build["source_handoff"])
            ),
            "deck_local_ref": f"refs/handoff/wa0-source/{build['source_commit']}",
            "detached_worktree_commit": build["source_commit"],
            "detached_worktree_clean": True,
        },
        "windows_build": {
            "schema": WINDOWS_BUILD_SCHEMA,
            "workflow_run_id": build["build_receipt"]["workflow"]["run_id"],
            "workflow_run_attempt": build["build_receipt"]["workflow"]["run_attempt"],
            "receipt_sha256": sha256_bytes(canonical_json(build["build_receipt"])),
        },
        "artifact_custody": {
            "schema": MAC_CUSTODY_SCHEMA,
            "artifact_id": artifact["id"], "artifact_name": artifact["name"],
            "upload_artifact_digest_bare":
                artifact["upload_artifact_digest_bare"],
            "rest_artifact_digest": artifact["rest_artifact_digest"],
            "raw_wrapper_sha256": artifact["raw_wrapper_sha256"],
            "receipt_sha256": sha256_bytes(canonical_json(build["mac_custody"])),
            "artifact_manifest_sha256": build["artifact_manifest_sha256"],
            "payload_archive_sha256":
                build["build_receipt"]["artifacts"]["payload_archive_sha256"],
            "artifact_cache_manifest_sha256": build["artifact_manifest_sha256"],
        },
        "runtime_proton_digest": RUNNER_DIGEST,
        "exercise_receipt_sha256": {
            "negative": sha256_bytes(canonical_json(negative)),
            "positive": sha256_bytes(canonical_json(positive)),
        },
        "component_session_sha256": sha256_bytes(
            canonical_json(component_session)
        ),
        "audio_processor_lease_sha256": sha256_bytes(
            canonical_json(audio_processor_lease)
        ),
        "interface_quiescence": True,
        "object_quiescence": True,
        "clean_in_process_shutdown": True,
        "proof_row_count": 20,
        "no_deck_github": initial["no_deck_github"],
        "protected_state_equal": True,
    }
    handoff = create_evidence_handoff(
        repo_root() / "evidence/wa0-windows-vst3-audio-processor-interface-admission",
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
        fail("WA0 stage root remains after evidence handoff")
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
        "phase", choices=("preflight", "validate", "all")
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
        print(f"WA0_ERROR: {error}", file=sys.stderr, flush=True)
        raise

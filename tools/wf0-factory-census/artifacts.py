#!/usr/bin/env python3
"""Exact Mac custody and offline source/artifact/evidence handoff admission for WC0."""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unicodedata
import zipfile
from typing import Any, Iterable, Sequence

sys.dont_write_bytecode = True

from common import (
    ARTIFACT_SCHEMA, AUTHORITY_MERGE_COMMIT, AUTHORITY_MERGE_TREE, BASIS_COMMIT,
    BASIS_TREE, EVIDENCE_FILES, EVIDENCE_HANDOFF_SCHEMA, EXPECTED_BRANCH,
    EXPECTED_REF, MAC_CUSTODY_SCHEMA, REPOSITORY, SOURCE_HANDOFF_SCHEMA,
    SOURCE_PATHS, SOURCE_SCHEMA, WINDOWS_BUILD_SCHEMA, WORKFLOW_PATH, artifact_cache_parent,
    canonical_json, command, command_text, execution_worktree_parent, fail,
    repo_root, require_clean_source, require_contained, sha256_bytes, sha256_file,
    source_handoff_parent, source_manifest, source_manifest_sha256, write_atomic,
)


MAX_ZIP_BYTES = 256 * 1024 * 1024
MAX_EXTRACTED_BYTES = 512 * 1024 * 1024
MAX_ENTRY_BYTES = 128 * 1024 * 1024
MAX_ENTRIES = 256
MAX_PATH_BYTES = 240
ALLOWED_ROLES = {
    "scanner_executable", "loader_adapter_executable", "approved_fault_module",
    "adapter_environment_carrier",
    "positive_fixture_module", "positive_fixture_resource",
    "required_runtime_dependency", "build_identity_core", "required_license_notice",
}


def normalize_artifact_digests(upload_digest_bare: str,
                               rest_artifact_digest: str) -> dict[str, str]:
    """Validate and join the upload-action and REST digest representations."""
    if not re.fullmatch(r"[0-9a-f]{64}", upload_digest_bare):
        fail("upload action artifact digest is malformed")
    match = re.fullmatch(r"sha256:([0-9a-f]{64})", rest_artifact_digest)
    if match is None:
        fail("REST artifact digest is malformed")
    rest_hex = match.group(1)
    if rest_hex != upload_digest_bare:
        fail("upload action and REST artifact digests differ")
    return {
        "upload_artifact_digest_bare": upload_digest_bare,
        "rest_algorithm": "sha256",
        "rest_hex": rest_hex,
        "rest_artifact_digest": rest_artifact_digest,
    }


def artifact_digest_regression() -> dict[str, Any]:
    """Prove typed digest acceptance and fail-closed malformed handling."""
    valid = "0123456789abcdef" * 4
    identity = normalize_artifact_digests(valid, f"sha256:{valid}")
    if identity["rest_hex"] != valid or identity["rest_algorithm"] != "sha256":
        fail("valid typed artifact digest normalization differs")
    rejected: list[str] = []
    cases = {
        "prefixed_upload": (f"sha256:{valid}", f"sha256:{valid}"),
        "uppercase_upload": (valid.upper(), f"sha256:{valid}"),
        "short_upload": (valid[:-1], f"sha256:{valid[:-1]}"),
        "bare_rest": (valid, valid),
        "uppercase_rest": (valid, f"sha256:{valid.upper()}"),
        "short_rest": (valid, f"sha256:{valid[:-1]}"),
        "wrong_algorithm": (valid, f"sha512:{valid}"),
        "different_hex": (valid, f"sha256:{'f' * 64}"),
    }
    for name, values in cases.items():
        try:
            normalize_artifact_digests(*values)
        except RuntimeError:
            rejected.append(name)
    if rejected != list(cases):
        fail("typed artifact digest regression did not reject every invalid case")
    return {
        "schema": "linux-vst-bridge-wf0-artifact-digest-regression/v1",
        "bare_upload_accepted": True,
        "prefixed_rest_normalized_separately": True,
        "invalid_cases_rejected": rejected,
    }


def read_canonical_json(path: pathlib.Path, schema: str | None = None) -> dict[str, Any]:
    if not path.is_file() or path.is_symlink() or path.stat().st_size > 2 * 1024 * 1024:
        fail(f"canonical JSON is absent or unsafe: {path.name}")
    data = path.read_bytes()
    try:
        value = json.loads(data)
    except (json.JSONDecodeError, UnicodeDecodeError):
        fail(f"canonical JSON is malformed: {path.name}")
    if not isinstance(value, dict) or canonical_json(value) != data:
        fail(f"JSON is not canonical one-LF form: {path.name}")
    if schema is not None and value.get("schema") != schema:
        fail(f"JSON schema differs: {path.name}")
    return value


def _safe_relative(value: str) -> str:
    if not isinstance(value, str) or not value or "\\" in value or "\0" in value:
        fail("archive path is empty or contains a prohibited separator/NUL")
    if value.startswith(("/", "//")) or re.match(r"^[A-Za-z]:", value) or ":" in value:
        fail(f"archive path is absolute, drive, UNC, or ADS shaped: {value}")
    if len(value.encode("utf-8")) > MAX_PATH_BYTES:
        fail("archive path exceeds the UTF-8 bound")
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        fail(f"archive path contains an empty/dot/traversal component: {value}")
    if any(ord(character) < 32 or ord(character) == 127 for character in value):
        fail("archive path contains a control character")
    return value


def zip_census(path: pathlib.Path, expected: Iterable[str] | None = None) -> list[dict[str, Any]]:
    if not path.is_file() or path.is_symlink() or path.stat().st_size > MAX_ZIP_BYTES:
        fail("archive is absent, unsafe, or oversized")
    expected_set = set(expected) if expected is not None else None
    records: list[dict[str, Any]] = []
    exact: set[str] = set()
    folded: set[str] = set()
    normalized: set[str] = set()
    total = 0
    with zipfile.ZipFile(path, "r") as archive:
        infos = archive.infolist()
        if not 1 <= len(infos) <= MAX_ENTRIES:
            fail("archive entry count is outside bounds")
        for info in infos:
            if info.orig_filename != info.filename:
                fail("archive path contained a NUL or was otherwise truncated")
            name = _safe_relative(info.orig_filename)
            if info.is_dir():
                fail("archive contains an explicit directory entry")
            if info.flag_bits & 1:
                fail("archive contains an encrypted entry")
            if info.compress_type not in {zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED}:
                fail("archive uses unsupported compression")
            mode = (info.external_attr >> 16) & 0xFFFF
            kind = stat.S_IFMT(mode)
            if kind not in {0, stat.S_IFREG}:
                fail("archive contains a link, device, or other non-regular entry")
            if info.file_size > MAX_ENTRY_BYTES:
                fail("archive entry exceeds the single-entry bound")
            total += info.file_size
            if total > MAX_EXTRACTED_BYTES:
                fail("archive extracted size exceeds the bound")
            folded_name = name.casefold()
            normalized_name = unicodedata.normalize("NFC", name)
            if name in exact or folded_name in folded or normalized_name in normalized:
                fail("archive contains duplicate, case-colliding, or normalization-colliding paths")
            exact.add(name)
            folded.add(folded_name)
            normalized.add(normalized_name)
            records.append({
                "path": name,
                "size": info.file_size,
                "compressed_size": info.compress_size,
                "compression": info.compress_type,
            })
    if expected_set is not None and exact != expected_set:
        fail(f"archive roster differs: {sorted(exact)}")
    return sorted(records, key=lambda item: item["path"].encode())


def safe_extract(path: pathlib.Path, destination: pathlib.Path,
                 expected: Iterable[str] | None = None) -> list[dict[str, Any]]:
    records = zip_census(path, expected)
    if destination.exists() or destination.is_symlink():
        fail("archive extraction destination already exists")
    destination.mkdir(parents=True)
    with zipfile.ZipFile(path, "r") as archive:
        for record in records:
            target = destination / record["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(record["path"], "r") as source, target.open("xb") as output:
                shutil.copyfileobj(source, output, length=1024 * 1024)
            if target.stat().st_size != record["size"]:
                fail(f"extracted archive size differs: {record['path']}")
    return records


def verify_hash_sidecar(sidecar: pathlib.Path, target: pathlib.Path) -> str:
    if not sidecar.is_file() or sidecar.is_symlink() or sidecar.stat().st_size > 256:
        fail(f"hash sidecar is absent or unsafe: {sidecar.name}")
    digest = sha256_file(target)
    expected = f"{digest}  {target.name}\n".encode()
    if sidecar.read_bytes() != expected:
        fail(f"hash sidecar differs: {sidecar.name}")
    return digest


def verify_payload(payload: pathlib.Path, extraction: pathlib.Path,
                   expected_source_commit: str,
                   expected_source_tree: str,
                   expected_source_manifest_sha256: str) -> dict[str, Any]:
    safe_extract(payload, extraction)
    manifest_path = extraction / "ARTIFACT_MANIFEST.json"
    manifest = read_canonical_json(manifest_path, ARTIFACT_SCHEMA)
    digest = verify_hash_sidecar(
        extraction / "ARTIFACT_MANIFEST.sha256", manifest_path
    )
    records = manifest.get("records")
    if not isinstance(records, list) or manifest.get("record_count") != len(records):
        fail("artifact manifest record count differs")
    paths = [item.get("path") for item in records if isinstance(item, dict)]
    if len(paths) != len(records) or paths != sorted(paths, key=lambda value: value.encode()):
        fail("artifact manifest records are not raw-UTF-8 sorted")
    if len(paths) != len(set(paths)):
        fail("artifact manifest contains duplicate records")
    declared = set(paths) | {"ARTIFACT_MANIFEST.json", "ARTIFACT_MANIFEST.sha256"}
    observed = {
        item.relative_to(extraction).as_posix()
        for item in extraction.rglob("*") if item.is_file()
    }
    if observed != declared:
        fail(f"payload extracted roster differs: {sorted(observed ^ declared)}")
    for item in records:
        relative = _safe_relative(item["path"])
        if item.get("type") != "file" or item.get("portable_mode") != "0444":
            fail(f"artifact record type/mode differs: {relative}")
        if item.get("role") not in ALLOWED_ROLES:
            fail(f"artifact record role differs: {relative}")
        target = extraction / relative
        if (target.stat().st_size, sha256_file(target)) != (
            item.get("size"), item.get("sha256")
        ):
            fail(f"artifact record bytes differ: {relative}")
    core = read_canonical_json(extraction / "BUILD_IDENTITY_CORE.json",
                               "linux-vst-bridge-wf0-build-identity-core/v1")
    source = core.get("source", {})
    source_manifest_value = source.get("implementation_source_manifest")
    if (
        source.get("commit") != expected_source_commit
        or source.get("tree") != expected_source_tree
        or source.get("parent") != BASIS_COMMIT
        or source.get("branch") != EXPECTED_BRANCH
        or source.get("ref") != EXPECTED_REF
        or source.get("implementation_source_schema") != SOURCE_SCHEMA
        or source.get("implementation_source_record_count") != 19
        or source.get("implementation_source_manifest_sha256")
        != expected_source_manifest_sha256
        or source_manifest_sha256(source_manifest_value) != expected_source_manifest_sha256
        or manifest.get("implementation_source_manifest_sha256")
        != expected_source_manifest_sha256
        or sha256_file(extraction / "BUILD_IDENTITY_CORE.json")
        != manifest.get("build_identity_core_sha256")
    ):
        fail("payload source/build identity join differs")
    return {
        "manifest": manifest,
        "manifest_sha256": digest,
        "build_identity_core": core,
    }


def _gh_json(route: str) -> dict[str, Any]:
    output = command(["gh", "api", "-H", "Accept: application/vnd.github+json", route],
                     timeout=120.0).stdout
    if len(output) > 8 * 1024 * 1024:
        fail("GitHub API response exceeds bound")
    try:
        value = json.loads(output)
    except (json.JSONDecodeError, UnicodeDecodeError):
        fail("GitHub API response is not JSON")
    if not isinstance(value, dict):
        fail("GitHub API response is not an object")
    return value


def _gh_download(route: str, destination: pathlib.Path) -> None:
    if destination.exists() or destination.is_symlink():
        fail("raw artifact wrapper destination exists")
    with destination.open("xb") as output:
        result = subprocess.run(
            ["gh", "api", "-H", "Accept: application/vnd.github+json", route],
            stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.PIPE,
            check=False, timeout=300.0,
        )
    if result.returncode != 0:
        destination.unlink(missing_ok=True)
        fail(f"exact-ID artifact download failed: {result.stderr.decode(errors='replace')[-2000:]}")


def mac_custody(run_id: str, artifact_id: str, source_commit: str,
                human_url: str, upload_digest_bare: str,
                stage: pathlib.Path) -> dict[str, Any]:
    if (not re.fullmatch(r"[1-9][0-9]*", run_id)
            or not re.fullmatch(r"[1-9][0-9]*", artifact_id)):
        fail("run ID or artifact ID is malformed")
    local_source = require_clean_source(source_commit, detached=False)
    local_source_sha = source_manifest_sha256(local_source)
    local_source_tree = command_text(
        ["git", "rev-parse", f"{source_commit}^{{tree}}"], cwd=repo_root()
    )
    if stage.exists() or stage.is_symlink():
        fail("Mac custody stage already exists")
    stage.mkdir(parents=True)
    repository = _gh_json(f"/repos/{REPOSITORY}")
    if (
        repository.get("full_name") != REPOSITORY
        or repository.get("visibility") != "private"
        or repository.get("owner", {}).get("type") != "User"
    ):
        fail("private repository posture differs")
    run = _gh_json(f"/repos/{REPOSITORY}/actions/runs/{run_id}")
    run_attempt = str(run.get("run_attempt"))
    expected_name = (
        f"wc0-windows-build-{source_commit}-run-{run_id}-attempt-{run_attempt}"
    )
    run_path = str(run.get("path", "")).split("@", 1)[0]
    if (
        str(run.get("id")) != run_id
        or run.get("event") != "push"
        or run.get("head_branch") != EXPECTED_BRANCH
        or run.get("head_sha") != source_commit
        or run.get("status") != "completed"
        or run.get("conclusion") != "success"
        or run_path != WORKFLOW_PATH
        or run.get("repository", {}).get("full_name") != REPOSITORY
    ):
        fail("exact workflow run metadata differs")
    artifacts = _gh_json(
        f"/repos/{REPOSITORY}/actions/runs/{run_id}/artifacts?per_page=100"
    )
    entries = artifacts.get("artifacts")
    if artifacts.get("total_count") != 1 or not isinstance(entries, list) or len(entries) != 1:
        fail("workflow run does not have one exact artifact")
    listed = entries[0]
    exact = _gh_json(f"/repos/{REPOSITORY}/actions/artifacts/{artifact_id}")
    if listed != exact:
        fail("run artifact list and exact artifact-ID object differ")
    rest_digest = str(exact.get("digest"))
    digest_identity = normalize_artifact_digests(
        upload_digest_bare, rest_digest
    )
    workflow_run = exact.get("workflow_run", {})
    api_url = f"https://api.github.com/repos/{REPOSITORY}/actions/artifacts/{artifact_id}"
    expected_human = (
        f"https://github.com/{REPOSITORY}/actions/runs/{run_id}/artifacts/{artifact_id}"
    )
    if (
        str(exact.get("id")) != artifact_id
        or exact.get("name") != expected_name
        or exact.get("url") != api_url
        or not isinstance(exact.get("size_in_bytes"), int)
        or exact.get("expired") is not False
        or str(workflow_run.get("id")) != run_id
        or workflow_run.get("head_branch") != EXPECTED_BRANCH
        or workflow_run.get("head_sha") != source_commit
        or human_url != expected_human
    ):
        fail("exact Actions artifact metadata differs")
    wrapper = stage / "github-actions-artifact-wrapper.zip"
    route = f"/repos/{REPOSITORY}/actions/artifacts/{artifact_id}/zip"
    _gh_download(route, wrapper)
    wrapper_sha = sha256_file(wrapper)
    if (
        wrapper_sha != digest_identity["upload_artifact_digest_bare"]
        or wrapper_sha != digest_identity["rest_hex"]
    ):
        fail("WC0_BUILD_CUSTODY_BLOCKED: raw wrapper digest differs from Actions digest")
    inner = stage / "inner"
    safe_extract(
        wrapper, inner,
        {
            "wf0-payload.zip",
            "WF0_WINDOWS_BUILD_RECEIPT.json",
            "WF0_WINDOWS_BUILD_RECEIPT.sha256",
        },
    )
    receipt_path = inner / "WF0_WINDOWS_BUILD_RECEIPT.json"
    receipt_sha = verify_hash_sidecar(
        inner / "WF0_WINDOWS_BUILD_RECEIPT.sha256", receipt_path
    )
    build_receipt = read_canonical_json(receipt_path, WINDOWS_BUILD_SCHEMA)
    workflow_blob = command_text(
        ["git", "rev-parse", f"{source_commit}:{WORKFLOW_PATH}"], cwd=repo_root()
    )
    if (
        build_receipt.get("source") != {
            "commit": source_commit,
            "tree": local_source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "implementation_source_manifest": {
                "schema": SOURCE_SCHEMA,
                "record_count": 19,
                "sha256": local_source_sha,
            },
        }
        or build_receipt.get("workflow", {}).get("run_id") != run_id
        or build_receipt.get("workflow", {}).get("run_attempt") != run_attempt
        or build_receipt.get("workflow", {}).get("git_blob") != workflow_blob
    ):
        fail("Windows build receipt does not join the exact Mac source/run")
    payload = inner / "wf0-payload.zip"
    if sha256_file(payload) != build_receipt.get("artifacts", {}).get(
        "payload_archive_sha256"
    ):
        fail("payload archive hash differs from the Windows receipt")
    payload_check = stage / "payload-check"
    payload_identity = verify_payload(
        payload, payload_check, source_commit, local_source_tree, local_source_sha
    )
    if (
        payload_identity["manifest_sha256"]
        != build_receipt.get("artifacts", {}).get("artifact_manifest_sha256")
    ):
        fail("canonical artifact manifest differs from the Windows receipt")
    inner_records = [
        {"path": name, "size": (inner / name).stat().st_size,
         "sha256": sha256_file(inner / name)}
        for name in sorted(item.name for item in inner.iterdir())
    ]
    custody = {
        "schema": MAC_CUSTODY_SCHEMA,
        "repository": {
            "full_name": REPOSITORY, "visibility": "private", "owner_type": "User",
        },
        "workflow": {
            "path": WORKFLOW_PATH, "git_blob": workflow_blob, "event": "push",
            "head_branch": EXPECTED_BRANCH, "head_sha": source_commit,
            "run_id": run_id, "run_attempt": run_attempt, "conclusion": "success",
        },
        "implementation_source": {
            "commit": source_commit, "tree": local_source_tree,
            "parent": BASIS_COMMIT, "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF, "schema": SOURCE_SCHEMA,
            "record_count": 19, "manifest_sha256": local_source_sha,
        },
        "artifact": {
            "id": artifact_id, "name": expected_name, "url": api_url,
            "human_facing_url": human_url,
            "size_in_bytes": str(exact["size_in_bytes"]),
            "digest": digest_identity["rest_artifact_digest"],
            "upload_artifact_digest_bare":
                digest_identity["upload_artifact_digest_bare"],
            "rest_artifact_digest": digest_identity["rest_artifact_digest"],
            "raw_wrapper_sha256": wrapper_sha,
            "expired": False, "workflow_run_id": run_id,
            "workflow_head_branch": EXPECTED_BRANCH,
            "workflow_head_sha": source_commit,
            "raw_download_route": route,
            "raw_github_artifact_zip_sha256": wrapper_sha,
        },
        "inner_envelope": {
            "record_count": 3, "records": inner_records,
            "build_receipt_sha256": receipt_sha,
            "payload_archive_sha256": sha256_file(payload),
            "artifact_manifest_sha256": payload_identity["manifest_sha256"],
        },
        "authenticated_private_repository_download": True,
        "credentials_retained": False,
        "explicit_nonclaims": [
            "no_cryptographic_provenance", "no_trusted_builder", "no_slsa",
            "no_code_signing", "no_release_signing",
        ],
    }
    write_atomic(
        stage / "WF0_MAC_ARTIFACT_CUSTODY_RECEIPT.json",
        canonical_json(custody),
    )
    return custody


def _bundle_header(path: pathlib.Path) -> tuple[int, list[tuple[str, str]]]:
    prerequisites = 0
    refs: list[tuple[str, str]] = []
    with path.open("rb") as handle:
        first = handle.readline()
        if first not in {b"# v2 git bundle\n", b"# v3 git bundle\n"}:
            fail("Git bundle header version is unsupported")
        for _ in range(1024):
            line = handle.readline()
            if line == b"\n":
                break
            if not line or len(line) > 4096:
                fail("Git bundle header is malformed or oversized")
            if line.startswith(b"-"):
                prerequisites += 1
            elif line.startswith(b"@"):
                continue
            else:
                try:
                    commit, ref = line.rstrip(b"\n").decode("ascii").split(" ", 1)
                except ValueError:
                    fail("Git bundle advertised-ref header is malformed")
                refs.append((commit, ref))
        else:
            fail("Git bundle header exceeds its line bound")
    return prerequisites, refs


def create_source_handoff(source_commit: str, stage: pathlib.Path) -> dict[str, Any]:
    root = repo_root()
    source = require_clean_source(source_commit, detached=False)
    source_digest = source_manifest_sha256(source)
    tree = command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=root)
    parent = command_text(["git", "rev-parse", "HEAD^"], cwd=root)
    if parent != BASIS_COMMIT:
        fail("source handoff implementation parent differs")
    if stage.exists() or stage.is_symlink():
        fail("source handoff stage already exists")
    stage.mkdir(parents=True)
    advertised_ref = f"refs/handoff/wc0-source/{source_commit}"
    bundle_name = f"wc0-execution-source-{source_commit}.bundle"
    bundle = stage / bundle_name
    temporary = pathlib.Path(tempfile.mkdtemp(prefix=".wf0-source-bare-", dir=stage.parent))
    try:
        bare = temporary / "repository.git"
        command(["git", "init", "--bare", str(bare)])
        command(["git", "-C", str(bare), "fetch", "--no-tags", str(root), source_commit],
                timeout=300.0)
        command(["git", "-C", str(bare), "update-ref", advertised_ref, source_commit])
        command(["git", "-C", str(bare), "bundle", "create", str(bundle), advertised_ref],
                timeout=300.0)
        verify = command(["git", "-C", str(bare), "bundle", "verify", str(bundle)],
                         timeout=120.0)
        verify_text = (verify.stdout + verify.stderr).decode("utf-8", "replace")
        if "complete history" not in verify_text.lower():
            fail("source Git bundle does not report complete history")
    finally:
        shutil.rmtree(temporary)
    if bundle.stat().st_size > 134217728:
        fail("source Git bundle exceeds 128 MiB")
    prerequisites, refs = _bundle_header(bundle)
    if prerequisites != 0 or refs != [(source_commit, advertised_ref)]:
        fail(f"source Git bundle header differs: prerequisites={prerequisites}, refs={refs}")
    receipt = {
        "schema": SOURCE_HANDOFF_SCHEMA,
        "repository": REPOSITORY,
        "wc0_authority": {
            "design_authority_commit": AUTHORITY_MERGE_COMMIT,
            "design_authority_tree": AUTHORITY_MERGE_TREE,
            "implementation_basis_commit": BASIS_COMMIT,
            "implementation_basis_tree": BASIS_TREE,
        },
        "implementation_source": {
            "commit": source_commit, "tree": tree, "parent": parent,
            "branch": EXPECTED_BRANCH, "ref": EXPECTED_REF,
            "manifest_schema": source["schema"],
            "manifest_record_count": source["record_count"],
            "manifest_sha256": source_digest,
        },
        "bundle": {
            "name": bundle_name, "advertised_ref": advertised_ref,
            "sha256": sha256_file(bundle), "size": bundle.stat().st_size,
            "max_size_bytes": 134217728, "git_bundle_verify": "passed",
            "self_contained": True, "prerequisite_count": 0,
        },
    }
    receipt_path = stage / "WC0_SOURCE_HANDOFF_RECEIPT.json"
    write_atomic(receipt_path, canonical_json(receipt))
    digest = sha256_file(receipt_path)
    write_atomic(
        stage / "WC0_SOURCE_HANDOFF_RECEIPT.sha256",
        f"{digest}  WC0_SOURCE_HANDOFF_RECEIPT.json\n".encode(),
    )
    names = sorted(item.name for item in stage.iterdir())
    if names != sorted([
        bundle_name, "WC0_SOURCE_HANDOFF_RECEIPT.json",
        "WC0_SOURCE_HANDOFF_RECEIPT.sha256",
    ]):
        fail("source handoff stage roster differs")
    return receipt


def import_source_handoff(handoff: pathlib.Path, source_commit: str) -> dict[str, Any]:
    """Verify an already imported fixed ref/worktree and publish its private bundle custody."""
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        fail("source handoff commit is malformed")
    expected_bundle = f"wc0-execution-source-{source_commit}.bundle"
    expected_names = {
        expected_bundle,
        "WC0_SOURCE_HANDOFF_RECEIPT.json",
        "WC0_SOURCE_HANDOFF_RECEIPT.sha256",
    }
    if (
        not handoff.is_dir()
        or handoff.is_symlink()
        or {item.name for item in handoff.iterdir()} != expected_names
        or any(not item.is_file() or item.is_symlink() for item in handoff.iterdir())
    ):
        fail("Deck source handoff envelope differs")

    receipt_path = handoff / "WC0_SOURCE_HANDOFF_RECEIPT.json"
    receipt_sha256 = verify_hash_sidecar(
        handoff / "WC0_SOURCE_HANDOFF_RECEIPT.sha256", receipt_path
    )
    receipt = read_canonical_json(receipt_path, SOURCE_HANDOFF_SCHEMA)
    bundle = handoff / expected_bundle
    advertised_ref = f"refs/handoff/wc0-source/{source_commit}"
    implementation = receipt.get("implementation_source", {})
    authority = receipt.get("wc0_authority", {})
    bundle_identity = receipt.get("bundle", {})
    if (
        receipt.get("repository") != REPOSITORY
        or authority != {
            "design_authority_commit": AUTHORITY_MERGE_COMMIT,
            "design_authority_tree": AUTHORITY_MERGE_TREE,
            "implementation_basis_commit": BASIS_COMMIT,
            "implementation_basis_tree": BASIS_TREE,
        }
        or implementation.get("commit") != source_commit
        or implementation.get("parent") != BASIS_COMMIT
        or implementation.get("branch") != EXPECTED_BRANCH
        or implementation.get("ref") != EXPECTED_REF
        or implementation.get("manifest_schema")
        != SOURCE_SCHEMA
        or implementation.get("manifest_record_count") != 19
        or not re.fullmatch(
            r"[0-9a-f]{40}", str(implementation.get("tree", ""))
        )
        or not re.fullmatch(
            r"[0-9a-f]{64}", str(implementation.get("manifest_sha256", ""))
        )
        or bundle_identity != {
            "name": expected_bundle,
            "advertised_ref": advertised_ref,
            "sha256": sha256_file(bundle),
            "size": bundle.stat().st_size,
            "max_size_bytes": 134217728,
            "git_bundle_verify": "passed",
            "self_contained": True,
            "prerequisite_count": 0,
        }
        or bundle.stat().st_size > 134217728
    ):
        fail("Deck source handoff receipt/bundle identity differs")
    prerequisites, refs = _bundle_header(bundle)
    if prerequisites != 0 or refs != [(source_commit, advertised_ref)]:
        fail("Deck source bundle header is not one self-contained exact ref")

    root = repo_root()
    verify = command(["git", "bundle", "verify", str(bundle)], cwd=root)
    verify_text = (verify.stdout + verify.stderr).decode("utf-8", "replace")
    if "complete history" not in verify_text.lower():
        fail("Deck source bundle verification did not report complete history")
    if (
        command_text(["git", "rev-parse", advertised_ref], cwd=root) != source_commit
        or command_text(["git", "rev-parse", "HEAD"], cwd=root) != source_commit
        or command_text(["git", "rev-parse", "HEAD^"], cwd=root) != BASIS_COMMIT
        or command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=root)
        != implementation["tree"]
        or command_text(
            ["git", "rev-parse", f"{BASIS_COMMIT}^{{tree}}"], cwd=root
        ) != BASIS_TREE
        or command_text(
            ["git", "rev-parse", f"{AUTHORITY_MERGE_COMMIT}^{{tree}}"], cwd=root
        ) != AUTHORITY_MERGE_TREE
    ):
        fail("Deck fixed ref/worktree/authority history differs")
    ancestor = command(
        ["git", "merge-base", "--is-ancestor", AUTHORITY_MERGE_COMMIT, source_commit],
        cwd=root, check=False,
    )
    if ancestor.returncode != 0:
        fail("Deck source bundle omits required WC0 authority history")
    reproduced = verify_execution_source(
        source_commit, implementation["tree"], implementation["manifest_sha256"]
    )

    parent = source_handoff_parent()
    parent.mkdir(parents=True, exist_ok=True)
    if parent.is_symlink():
        fail("Deck persistent source-handoff parent is a symlink")
    target = parent / source_commit
    require_contained(target, parent, "Deck persistent source handoff")
    if target.exists() or target.is_symlink():
        fail("Deck persistent source handoff already exists; adoption is prohibited")
    staging = parent / f".wc0-source-import-{os.urandom(16).hex()}"
    staging.mkdir(mode=0o700)
    for name in sorted(expected_names):
        shutil.copy2(handoff / name, staging / name)
        if sha256_file(staging / name) != sha256_file(handoff / name):
            fail(f"Deck persistent source-handoff copy differs: {name}")
        (staging / name).chmod(0o444)
    os.replace(staging, target)
    target.chmod(0o555)
    return {
        "schema": "linux-vst-bridge-wc0-deck-source-admission/v1",
        "implementation_source": {
            "commit": source_commit,
            "tree": implementation["tree"],
            "parent": BASIS_COMMIT,
            "manifest_sha256": implementation["manifest_sha256"],
            "record_count": reproduced["record_count"],
        },
        "bundle": bundle_identity,
        "source_handoff_receipt_sha256": receipt_sha256,
        "local_ref": advertised_ref,
        "detached_execution_worktree": True,
        "persistent_handoff_read_only": True,
    }


def verify_execution_source(source_commit: str, expected_tree: str,
                            expected_manifest_sha256: str) -> dict[str, Any]:
    root = repo_root()
    if root != execution_worktree_parent() / source_commit:
        fail("execution worktree path differs from the fixed Deck path")
    if command_text(["git", "branch", "--show-current"], cwd=root):
        fail("Deck execution source is not detached")
    if command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=root) != expected_tree:
        fail("Deck execution source tree differs")
    source = require_clean_source(source_commit, detached=True)
    digest = source_manifest_sha256(source)
    if digest != expected_manifest_sha256:
        fail("Deck reproduced source-manifest digest differs")
    return source


def import_artifact(handoff: pathlib.Path, source_commit: str,
                    source_manifest_digest: str) -> dict[str, Any]:
    expected = {
        "wf0-payload.zip", "WF0_WINDOWS_BUILD_RECEIPT.json",
        "WF0_WINDOWS_BUILD_RECEIPT.sha256",
        "WF0_MAC_ARTIFACT_CUSTODY_RECEIPT.json",
    }
    if (
        not handoff.is_dir() or handoff.is_symlink()
        or {item.name for item in handoff.iterdir()} != expected
        or any(not item.is_file() or item.is_symlink() for item in handoff.iterdir())
    ):
        fail("Deck artifact handoff envelope differs")
    receipt_path = handoff / "WF0_WINDOWS_BUILD_RECEIPT.json"
    receipt_sha = verify_hash_sidecar(
        handoff / "WF0_WINDOWS_BUILD_RECEIPT.sha256", receipt_path
    )
    build_receipt = read_canonical_json(receipt_path, WINDOWS_BUILD_SCHEMA)
    custody_path = handoff / "WF0_MAC_ARTIFACT_CUSTODY_RECEIPT.json"
    custody = read_canonical_json(custody_path, MAC_CUSTODY_SCHEMA)
    custody_artifact = custody.get("artifact", {})
    source_tree = command_text(
        ["git", "rev-parse", f"{source_commit}^{{tree}}"], cwd=repo_root()
    )
    expected_source = {
        "commit": source_commit,
        "tree": source_tree,
        "parent": BASIS_COMMIT,
        "branch": EXPECTED_BRANCH,
        "ref": EXPECTED_REF,
        "schema": SOURCE_SCHEMA,
        "record_count": 19,
        "manifest_sha256": source_manifest_digest,
    }
    custody_digest = normalize_artifact_digests(
        str(custody_artifact.get("upload_artifact_digest_bare")),
        str(custody_artifact.get("rest_artifact_digest")),
    )
    payload = handoff / "wf0-payload.zip"
    if (
        build_receipt.get("source") != {
            "commit": source_commit,
            "tree": source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "implementation_source_manifest": {
                "schema": SOURCE_SCHEMA,
                "record_count": 19,
                "sha256": source_manifest_digest,
            },
        }
        or custody.get("implementation_source") != expected_source
        or custody.get("workflow", {}).get("head_sha") != source_commit
        or custody.get("inner_envelope", {}).get("build_receipt_sha256") != receipt_sha
        or custody.get("inner_envelope", {}).get("payload_archive_sha256")
        != sha256_file(payload)
        or custody_artifact.get("digest")
        != custody_digest["rest_artifact_digest"]
        or custody_artifact.get("raw_wrapper_sha256")
        != custody_digest["rest_hex"]
        or custody_artifact.get("raw_github_artifact_zip_sha256")
        != custody_digest["rest_hex"]
    ):
        fail("Deck artifact/source/custody identity join differs")
    parent = artifact_cache_parent()
    parent.mkdir(parents=True, exist_ok=True)
    extraction = parent / f".wf0-artifact-import-{os.urandom(16).hex()}"
    payload_identity = verify_payload(
        payload, extraction, source_commit, source_tree, source_manifest_digest
    )
    digest = payload_identity["manifest_sha256"]
    if (
        digest != build_receipt.get("artifacts", {}).get("artifact_manifest_sha256")
        or digest != custody.get("inner_envelope", {}).get("artifact_manifest_sha256")
    ):
        fail("Deck canonical artifact manifest identity differs")
    target = parent / digest
    if target.exists() or target.is_symlink():
        fail("artifact cache target already exists; current WF0 requires a new exact import")
    shutil.copy2(receipt_path, extraction / receipt_path.name)
    shutil.copy2(handoff / "WF0_WINDOWS_BUILD_RECEIPT.sha256",
                 extraction / "WF0_WINDOWS_BUILD_RECEIPT.sha256")
    shutil.copy2(custody_path, extraction / custody_path.name)
    marker = {
        "schema": "linux-vst-bridge-wf0-artifact-cache-owner/v1",
        "artifact_manifest_sha256": digest,
        "source_commit": source_commit,
        "implementation_source_manifest_sha256": source_manifest_digest,
        "windows_build_receipt_sha256": receipt_sha,
        "mac_custody_receipt_sha256": sha256_file(custody_path),
    }
    write_atomic(extraction / ".wf0-artifact-owner.json", canonical_json(marker))
    os.replace(extraction, target)
    for directory in sorted(
        (item for item in target.rglob("*") if item.is_dir()), reverse=True
    ):
        directory.chmod(0o555)
    for file in (item for item in target.rglob("*") if item.is_file()):
        file.chmod(0o444)
    target.chmod(0o555)
    return {
        "artifact_root": str(target),
        "artifact_manifest_sha256": digest,
        "artifact_manifest": payload_identity["manifest"],
        "build_identity_core": payload_identity["build_identity_core"],
        "build_receipt": build_receipt,
        "mac_custody": custody,
        "cache_marker": marker,
    }


def verify_evidence_packet(root: pathlib.Path) -> list[dict[str, Any]]:
    if not root.is_dir() or root.is_symlink():
        fail("evidence packet root is absent or unsafe")
    observed = sorted(item.name for item in root.iterdir())
    if observed != sorted(EVIDENCE_FILES):
        fail(f"evidence packet roster differs: {observed}")
    records: list[dict[str, Any]] = []
    prohibited = re.compile(
        rb"(?:/home/|/Users/|192\.168\.|BEGIN (?:RSA |OPENSSH )?PRIVATE KEY|"
        rb"github_pat_|ghp_|compatdata|(?:^|[^A-Za-z])PID(?:[^A-Za-z]|$))",
        re.IGNORECASE,
    )
    for name in sorted(EVIDENCE_FILES, key=lambda value: value.encode()):
        path = root / name
        if not path.is_file() or path.is_symlink() or path.stat().st_size > 2 * 1024 * 1024:
            fail(f"evidence file is absent, unsafe, or oversized: {name}")
        data = path.read_bytes()
        if b"\0" in data or prohibited.search(data):
            fail(f"evidence contains prohibited private/binary data: {name}")
        data.decode("utf-8", "strict")
        records.append({"path": name, "size": len(data), "sha256": sha256_bytes(data)})
    lines = (root / "hashes.sha256").read_text(encoding="utf-8").splitlines()
    expected = [
        f"{sha256_file(root / name)}  {name}"
        for name in sorted(
            (item for item in EVIDENCE_FILES if item != "hashes.sha256"),
            key=lambda value: value.encode(),
        )
    ]
    if lines != expected:
        fail("evidence hashes.sha256 differs")
    return records


def create_evidence_handoff(packet: pathlib.Path, destination: pathlib.Path,
                            identity: dict[str, Any]) -> dict[str, Any]:
    records = verify_evidence_packet(packet)
    if destination.exists() or destination.is_symlink():
        fail("evidence handoff destination already exists")
    destination.mkdir(parents=True)
    packet_out = destination / "packet"
    shutil.copytree(packet, packet_out)
    aggregate = sha256_bytes(canonical_json(records))
    receipt = {
        "schema": EVIDENCE_HANDOFF_SCHEMA,
        **identity,
        "evidence": {
            "record_count": 14, "records": records,
            "aggregate_manifest_sha256": aggregate,
        },
        "credentials_retained": False,
        "private_paths_retained": False,
        "raw_process_identifiers_retained": False,
        "compiled_binaries_retained": False,
    }
    write_atomic(destination / "WC0_EVIDENCE_HANDOFF_RECEIPT.json", canonical_json(receipt))
    digest = sha256_file(destination / "WC0_EVIDENCE_HANDOFF_RECEIPT.json")
    write_atomic(
        destination / "WC0_EVIDENCE_HANDOFF_RECEIPT.sha256",
        f"{digest}  WC0_EVIDENCE_HANDOFF_RECEIPT.json\n".encode(),
    )
    return receipt


def verify_evidence_handoff(root: pathlib.Path) -> dict[str, Any]:
    expected = {
        "packet", "WC0_EVIDENCE_HANDOFF_RECEIPT.json",
        "WC0_EVIDENCE_HANDOFF_RECEIPT.sha256",
    }
    if not root.is_dir() or root.is_symlink() or {item.name for item in root.iterdir()} != expected:
        fail("evidence handoff roster differs")
    receipt_path = root / "WC0_EVIDENCE_HANDOFF_RECEIPT.json"
    verify_hash_sidecar(root / "WC0_EVIDENCE_HANDOFF_RECEIPT.sha256", receipt_path)
    receipt = read_canonical_json(receipt_path, EVIDENCE_HANDOFF_SCHEMA)
    packet = root / "packet"
    records = verify_evidence_packet(packet)
    retained = read_canonical_json(
        packet / "BUILD_MANIFEST.json",
        "linux-vst-bridge-wc0-retained-build-manifest/v1",
    )
    component = read_canonical_json(
        packet / "COMPONENT_SESSION.json",
        "linux-vst-bridge-wc0-component-session/v1",
    )
    callbacks = read_canonical_json(
        packet / "CALLBACK_LEDGER.json",
        "linux-vst-bridge-wc0-callback-ledger/v1",
    )
    implementation = receipt.get("implementation_source", {})
    source_commit = implementation.get("commit")
    if not isinstance(source_commit, str):
        fail("evidence handoff source commit is absent")
    reproduced = source_manifest(source_commit, treeish=source_commit)
    source_digest = source_manifest_sha256(reproduced)
    source_tree = command_text(
        ["git", "rev-parse", f"{source_commit}^{{tree}}"], cwd=repo_root()
    )
    retained_build = retained.get("windows_build_receipt", {})
    retained_custody = retained.get("mac_artifact_custody_receipt", {})
    retained_source_handoff = retained.get("source_handoff_receipt", {})
    artifact_custody = receipt.get("artifact_custody", {})
    if (
        receipt.get("evidence", {}).get("record_count") != 14
        or receipt.get("evidence", {}).get("records") != records
        or receipt.get("evidence", {}).get("aggregate_manifest_sha256")
        != sha256_bytes(canonical_json(records))
        or implementation != {
            "commit": source_commit,
            "tree": source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "schema": SOURCE_SCHEMA,
            "record_count": 19,
            "manifest_sha256": source_digest,
        }
        or retained.get("implementation_source_manifest") != reproduced
        or retained.get("implementation_source_manifest_sha256") != source_digest
        or retained.get("implementation_source_tree") != source_tree
        or receipt.get("source_handoff", {}).get("schema")
        != SOURCE_HANDOFF_SCHEMA
        or receipt.get("source_handoff", {}).get("bundle_name")
        != retained_source_handoff.get("bundle", {}).get("name")
        or receipt.get("source_handoff", {}).get("bundle_sha256")
        != retained_source_handoff.get("bundle", {}).get("sha256")
        or receipt.get("source_handoff", {}).get("advertised_ref")
        != retained_source_handoff.get("bundle", {}).get("advertised_ref")
        or receipt.get("windows_build", {}).get("receipt_sha256")
        != sha256_bytes(canonical_json(retained_build))
        or artifact_custody.get("receipt_sha256")
        != sha256_bytes(canonical_json(retained_custody))
        or artifact_custody.get("artifact_id")
        != retained_custody.get("artifact", {}).get("id")
        or artifact_custody.get("artifact_name")
        != retained_custody.get("artifact", {}).get("name")
        or artifact_custody.get("artifact_manifest_sha256")
        != retained.get("deck_admission", {}).get(
            "artifact_cache_manifest_sha256"
        )
        or receipt.get("component_session_sha256")
        != sha256_bytes(canonical_json(component))
        or receipt.get("callback_ledger_sha256")
        != sha256_bytes(canonical_json(callbacks))
        or receipt.get("object_quiescence") is not True
        or receipt.get("clean_in_process_shutdown") is not True
        or receipt.get("proof_row_count") != 29
        or receipt.get("protected_state_equal") is not True
        or receipt.get("no_deck_github", {}).get("github_operations") != 0
    ):
        fail("evidence handoff source/build/artifact/Deck/packet join differs")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="operation", required=True)
    custody = sub.add_parser("mac-custody")
    custody.add_argument("--run-id", required=True)
    custody.add_argument("--artifact-id", required=True)
    custody.add_argument("--source-commit", required=True)
    custody.add_argument("--human-url", required=True)
    custody.add_argument("--upload-digest-bare", required=True)
    custody.add_argument("--stage", type=pathlib.Path, required=True)
    sub.add_parser("digest-regression")
    source = sub.add_parser("create-source-handoff")
    source.add_argument("--source-commit", required=True)
    source.add_argument("--stage", type=pathlib.Path, required=True)
    import_source = sub.add_parser("import-source-handoff")
    import_source.add_argument("--handoff", type=pathlib.Path, required=True)
    import_source.add_argument("--source-commit", required=True)
    verify_source = sub.add_parser("verify-execution-source")
    verify_source.add_argument("--source-commit", required=True)
    verify_source.add_argument("--source-tree", required=True)
    verify_source.add_argument("--source-manifest-sha256", required=True)
    admit = sub.add_parser("import-artifact")
    admit.add_argument("--handoff", type=pathlib.Path, required=True)
    admit.add_argument("--source-commit", required=True)
    admit.add_argument("--source-manifest-sha256", required=True)
    evidence = sub.add_parser("verify-evidence-handoff")
    evidence.add_argument("--handoff", type=pathlib.Path, required=True)
    args = parser.parse_args()

    if args.operation == "mac-custody":
        result = mac_custody(
            args.run_id, args.artifact_id, args.source_commit,
            args.human_url, args.upload_digest_bare, args.stage.resolve(),
        )
    elif args.operation == "digest-regression":
        result = artifact_digest_regression()
    elif args.operation == "create-source-handoff":
        result = create_source_handoff(args.source_commit, args.stage.resolve())
    elif args.operation == "import-source-handoff":
        result = import_source_handoff(
            args.handoff.resolve(), args.source_commit
        )
    elif args.operation == "verify-execution-source":
        result = verify_execution_source(
            args.source_commit, args.source_tree, args.source_manifest_sha256
        )
    elif args.operation == "import-artifact":
        result = import_artifact(
            args.handoff.resolve(), args.source_commit,
            args.source_manifest_sha256,
        )
    else:
        result = verify_evidence_handoff(args.handoff.resolve())
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"WC0_ERROR: {error}", file=os.sys.stderr, flush=True)
        raise

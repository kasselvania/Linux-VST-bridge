#!/usr/bin/env python3
"""DX0 fixture, host-artifact, and exact source-handoff custody primitives."""

from __future__ import annotations

import os
import pathlib
import re
import shutil
import stat
import tempfile
import unicodedata
import zipfile
from typing import Any, Iterable

from common import (
    source_contract, source_authority,
    DX0_ACCEPTED_FIXTURE_ID, DX0_ACCEPTED_FIXTURE_SCHEMA,
    DX0_ACCEPTED_WA0_ARTIFACT_ID, DX0_ACCEPTED_WA0_ARTIFACT_MANIFEST_SHA256,
    DX0_ACCEPTED_WA0_BUILD_RECEIPT_SHA256, DX0_ACCEPTED_WA0_CUSTODY_RECEIPT_SHA256,
    DX0_ACCEPTED_WA0_PAYLOAD_SHA256, DX0_ACCEPTED_WA0_PRODUCER,
    DX0_ACCEPTED_WA0_RUN_ATTEMPT, DX0_ACCEPTED_WA0_RUN_ID,
    DX0_ACCEPTED_WA0_WRAPPER_SHA256, DX0_AGAIN_BUNDLE_MANIFEST_SHA256,
    DX0_AGAIN_MODULE_SHA256, DX0_APPROVAL_BLOB, DX0_BASIS_COMMIT,
    DX0_BASIS_TREE, DX0_BRANCH, DX0_DESIGN_BLOB, DX0_DESIGN_COMMIT,
    DX0_DESIGN_SHA256,
    DX0_COMPLETE_SOURCE_SCHEMA, DX0_HOST_ARTIFACT_SCHEMA, DX0_HOST_BUILD_SCHEMA,
    DX0_MAC_HOST_CUSTODY_SCHEMA, DX0_REF, DX0_REVIEW_ID,
    DX0_SOURCE_HANDOFF_SCHEMA,
    REPOSITORY, canonical_json, command, dx0_complete_source,
    dx0_deck_fixture_parent, dx0_deck_host_artifact_parent, dx0_identity_sha256,
    dx0_mac_fixture_parent, dx0_mac_host_artifact_parent,
    dx0_source_role, fail, parse_json_no_duplicates, repo_root, require_contained, sha256_bytes,
    sha256_file, write_atomic,
)

MAX_ZIP_BYTES = 256 * 1024 * 1024
MAX_EXTRACTED_BYTES = 512 * 1024 * 1024
MAX_ENTRY_BYTES = 128 * 1024 * 1024
MAX_ENTRIES = 256
MAX_PATH_BYTES = 240


def read_canonical_json(path: pathlib.Path, schema: str | None = None) -> dict[str, Any]:
    if not path.is_file() or path.is_symlink() or path.stat().st_size > 4 * 1024 * 1024:
        fail(f"canonical JSON is absent or unsafe: {path.name}")
    data = path.read_bytes()
    value = parse_json_no_duplicates(data, path.name)
    if not isinstance(value, dict) or canonical_json(value) != data:
        fail(f"JSON is not canonical one-LF form: {path.name}")
    if schema is not None and value.get("schema") != schema:
        fail(f"JSON schema differs: {path.name}")
    return value


def verify_hash_sidecar(sidecar: pathlib.Path, target: pathlib.Path) -> str:
    if not sidecar.is_file() or sidecar.is_symlink() or sidecar.stat().st_size > 256:
        fail(f"hash sidecar is absent or unsafe: {sidecar.name}")
    digest = sha256_file(target)
    if sidecar.read_bytes() != f"{digest}  {target.name}\n".encode():
        fail(f"hash sidecar differs: {sidecar.name}")
    return digest


def _safe_relative(value: str) -> str:
    if not isinstance(value, str) or not value or "\\" in value or "\0" in value:
        fail("archive path is empty or contains a prohibited separator/NUL")
    if value.startswith(("/", "//")) or re.match(r"^[A-Za-z]:", value) or ":" in value:
        fail(f"archive path is absolute, drive, UNC, or ADS shaped: {value}")
    if len(value.encode("utf-8")) > MAX_PATH_BYTES:
        fail("archive path exceeds the UTF-8 bound")
    if any(part in {"", ".", ".."} for part in value.split("/")):
        fail(f"archive path contains an empty/dot/traversal component: {value}")
    if any(ord(character) < 32 or ord(character) == 127 for character in value):
        fail("archive path contains a control character")
    return value


def zip_census(path: pathlib.Path, expected: Iterable[str] | None = None) -> list[dict[str, Any]]:
    if not path.is_file() or path.is_symlink() or path.stat().st_size > MAX_ZIP_BYTES:
        fail("archive is absent, unsafe, or oversized")
    expected_set = None if expected is None else set(expected)
    exact: set[str] = set()
    folded: set[str] = set()
    normalized: set[str] = set()
    normalized_folded: set[str] = set()
    total = 0
    records: list[dict[str, Any]] = []
    with zipfile.ZipFile(path, "r") as archive:
        infos = archive.infolist()
        if not 1 <= len(infos) <= MAX_ENTRIES:
            fail("archive entry count is outside bounds")
        for info in infos:
            if info.orig_filename != info.filename:
                fail("archive path was truncated")
            name = _safe_relative(info.filename)
            if info.is_dir() or info.flag_bits & 1:
                fail("archive contains a directory or encrypted entry")
            if info.compress_type not in {zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED}:
                fail("archive uses unsupported compression")
            kind = stat.S_IFMT((info.external_attr >> 16) & 0xFFFF)
            if kind not in {0, stat.S_IFREG}:
                fail("archive contains a link, device, or non-regular entry")
            if info.file_size > MAX_ENTRY_BYTES:
                fail("archive entry exceeds the single-entry bound")
            total += info.file_size
            if total > MAX_EXTRACTED_BYTES:
                fail("archive extracted size exceeds the bound")
            folded_name = name.casefold()
            normalized_name = unicodedata.normalize("NFC", name)
            normalized_folded_name = normalized_name.casefold()
            if (name in exact or folded_name in folded
                    or normalized_name in normalized
                    or normalized_folded_name in normalized_folded):
                fail("archive contains duplicate or colliding paths")
            exact.add(name)
            folded.add(folded_name)
            normalized.add(normalized_name)
            normalized_folded.add(normalized_folded_name)
            records.append({"path": name, "size": info.file_size,
                            "compressed_size": info.compress_size,
                            "compression": info.compress_type})
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


def _validate_manifest_files(root: pathlib.Path, manifest: dict[str, Any],
                             *, sidecars: set[str]) -> None:
    records = manifest.get("records")
    if not isinstance(records, list) or manifest.get("record_count") != len(records):
        fail("artifact manifest record count differs")
    paths = [item.get("path") for item in records if isinstance(item, dict)]
    if len(paths) != len(records) or paths != sorted(paths, key=lambda item: item.encode()):
        fail("artifact manifest paths are not complete raw-UTF-8 order")
    if len(paths) != len(set(paths)):
        fail("artifact manifest contains duplicate paths")
    objects = list(root.rglob("*"))
    if any(path.is_symlink() or not (path.is_file() or path.is_dir()) for path in objects):
        fail("artifact tree contains an unsafe object")
    expected_files = set(paths) | sidecars
    observed = {path.relative_to(root).as_posix() for path in objects if path.is_file()}
    expected_directories = {
        parent.as_posix()
        for relative in expected_files
        for parent in pathlib.PurePosixPath(relative).parents
        if parent.as_posix() != "."
    }
    observed_directories = {
        path.relative_to(root).as_posix() for path in objects if path.is_dir()
    }
    if observed != expected_files or observed_directories != expected_directories:
        fail("artifact file/directory roster differs")
    for record in records:
        if set(record) != {"path", "portable_mode", "role", "sha256", "size", "type"}:
            fail("artifact record shape differs")
        relative = _safe_relative(record["path"])
        path = root / relative
        if (record["type"] != "file" or record["portable_mode"] != "0444"
                or not path.is_file() or path.is_symlink()
                or path.stat().st_size != record["size"]
                or sha256_file(path) != record["sha256"]):
            fail(f"artifact record bytes differ: {relative}")


def accepted_wa0_evidence() -> dict[str, Any]:
    path = repo_root() / "evidence/wa0-windows-vst3-audio-processor-interface-admission/BUILD_MANIFEST.json"
    value = read_canonical_json(path, "linux-vst-bridge-wa0-retained-build-manifest/v1")
    bundle = value.get("again_bundle_manifest")
    receipt = value.get("windows_build_receipt")
    custody = value.get("mac_artifact_custody_receipt")
    if (not isinstance(bundle, dict)
            or bundle.get("schema") != "linux-vst-bridge-wf0-bundle-manifest/v1"
            or bundle.get("sha256") != DX0_AGAIN_BUNDLE_MANIFEST_SHA256
            or len(bundle.get("records", [])) != 14
            or sha256_bytes(canonical_json({k: v for k, v in bundle.items() if k != "sha256"}))
            != DX0_AGAIN_BUNDLE_MANIFEST_SHA256
            or next((item.get("sha256") for item in bundle["records"]
                     if item.get("path") == "Contents/x86_64-win/again.vst3"), None)
            != DX0_AGAIN_MODULE_SHA256
            or sha256_bytes(canonical_json(receipt)) != DX0_ACCEPTED_WA0_BUILD_RECEIPT_SHA256
            or sha256_bytes(canonical_json(custody)) != DX0_ACCEPTED_WA0_CUSTODY_RECEIPT_SHA256
            or value.get("artifact_manifest", {}).get("record_count") != 33
            or custody.get("artifact", {}).get("id") != str(DX0_ACCEPTED_WA0_ARTIFACT_ID)):
        fail("accepted WA0 evidence/fixture custody identity differs")
    return value


def accepted_fixture_identity() -> dict[str, Any]:
    retained = accepted_wa0_evidence()
    return {
        "schema": DX0_ACCEPTED_FIXTURE_SCHEMA,
        "fixture_id": DX0_ACCEPTED_FIXTURE_ID,
        "bundle_manifest": retained["again_bundle_manifest"],
        "accepted_wa0_custody": {
            "artifact_id": DX0_ACCEPTED_WA0_ARTIFACT_ID,
            "workflow_run_id": DX0_ACCEPTED_WA0_RUN_ID,
            "workflow_run_attempt": DX0_ACCEPTED_WA0_RUN_ATTEMPT,
            "producer_source_commit": DX0_ACCEPTED_WA0_PRODUCER,
            "artifact_manifest_sha256": DX0_ACCEPTED_WA0_ARTIFACT_MANIFEST_SHA256,
            "raw_wrapper_sha256": DX0_ACCEPTED_WA0_WRAPPER_SHA256,
            "payload_sha256": DX0_ACCEPTED_WA0_PAYLOAD_SHA256,
            "build_receipt_sha256": DX0_ACCEPTED_WA0_BUILD_RECEIPT_SHA256,
            "mac_custody_receipt_sha256": DX0_ACCEPTED_WA0_CUSTODY_RECEIPT_SHA256,
        },
    }


def accepted_fixture_identity_sha256() -> str:
    return dx0_identity_sha256(accepted_fixture_identity())


def verify_fixture_store(root: pathlib.Path | None = None) -> dict[str, Any]:
    identity = accepted_fixture_identity()
    digest = dx0_identity_sha256(identity)
    mac_root = dx0_mac_fixture_parent() / DX0_AGAIN_BUNDLE_MANIFEST_SHA256
    deck_root = dx0_deck_fixture_parent() / DX0_AGAIN_BUNDLE_MANIFEST_SHA256
    target = mac_root if root is None else root
    if target not in {mac_root, deck_root} or not target.is_dir() or target.is_symlink():
        fail("DX0 accepted fixture store root differs")
    receipt_path = target / "DX0_ACCEPTED_FIXTURE_RECEIPT.json"
    receipt_sha = verify_hash_sidecar(target / "DX0_ACCEPTED_FIXTURE_RECEIPT.sha256", receipt_path)
    receipt = read_canonical_json(receipt_path, DX0_ACCEPTED_FIXTURE_SCHEMA)
    if receipt != identity or receipt_sha != digest:
        fail("DX0 accepted fixture receipt differs")
    bundle = target / "again.vst3"
    expected_records = identity["bundle_manifest"]["records"]
    objects = list(target.rglob("*"))
    if any(path.is_symlink() or not (path.is_file() or path.is_dir()) for path in objects):
        fail("DX0 accepted fixture store contains an unsafe object")
    observed = {path.relative_to(bundle).as_posix()
                for path in bundle.rglob("*") if path.is_file()}
    if observed != {item["path"] for item in expected_records}:
        fail("DX0 accepted fixture bundle roster differs")
    expected_files = {
        "DX0_ACCEPTED_FIXTURE_RECEIPT.json", "DX0_ACCEPTED_FIXTURE_RECEIPT.sha256",
        *(f"again.vst3/{item['path']}" for item in expected_records),
    }
    expected_directories = {
        parent.as_posix()
        for relative in expected_files
        for parent in pathlib.PurePosixPath(relative).parents
        if parent.as_posix() != "."
    }
    if ({path.relative_to(target).as_posix() for path in objects if path.is_file()}
            != expected_files
            or {path.relative_to(target).as_posix() for path in objects if path.is_dir()}
            != expected_directories):
        fail("DX0 accepted fixture complete store roster differs")
    for record in expected_records:
        path = bundle / record["path"]
        if (not path.is_file() or path.is_symlink()
                or path.stat().st_size != record["size"]
                or sha256_file(path) != record["sha256"]):
            fail(f"DX0 accepted fixture bytes differ: {record['path']}")
    for path in target.rglob("*"):
        if stat.S_IMODE(path.stat().st_mode) & 0o222:
            fail("DX0 accepted fixture store is writable")
    return {"root": str(target), "identity": identity, "identity_sha256": digest,
            "receipt_sha256": receipt_sha, "disposition": "reused"}


def seed_fixture(wrapper: pathlib.Path) -> dict[str, Any]:
    target = dx0_mac_fixture_parent() / DX0_AGAIN_BUNDLE_MANIFEST_SHA256
    if target.exists() or target.is_symlink():
        return verify_fixture_store(target)
    if sha256_file(wrapper) != DX0_ACCEPTED_WA0_WRAPPER_SHA256:
        fail("accepted WA0 raw wrapper SHA-256 differs")
    parent = dx0_mac_fixture_parent()
    parent.mkdir(parents=True, exist_ok=True)
    if parent.is_symlink():
        fail("DX0 Mac fixture-store parent is a symlink")
    with tempfile.TemporaryDirectory(prefix="dx0-fixture-verify-", dir=parent) as temporary:
        stage = pathlib.Path(temporary)
        inner = stage / "inner"
        safe_extract(wrapper, inner, {"wf0-payload.zip", "WF0_WINDOWS_BUILD_RECEIPT.json",
                                      "WF0_WINDOWS_BUILD_RECEIPT.sha256"})
        receipt_path = inner / "WF0_WINDOWS_BUILD_RECEIPT.json"
        if (verify_hash_sidecar(inner / "WF0_WINDOWS_BUILD_RECEIPT.sha256", receipt_path)
                != DX0_ACCEPTED_WA0_BUILD_RECEIPT_SHA256
                or sha256_file(inner / "wf0-payload.zip") != DX0_ACCEPTED_WA0_PAYLOAD_SHA256):
            fail("accepted WA0 inner envelope identity differs")
        payload = stage / "payload"
        safe_extract(inner / "wf0-payload.zip", payload)
        manifest_path = payload / "ARTIFACT_MANIFEST.json"
        if (sha256_file(manifest_path) != DX0_ACCEPTED_WA0_ARTIFACT_MANIFEST_SHA256
                or verify_hash_sidecar(payload / "ARTIFACT_MANIFEST.sha256", manifest_path)
                != DX0_ACCEPTED_WA0_ARTIFACT_MANIFEST_SHA256):
            fail("accepted WA0 payload manifest identity differs")
        manifest = read_canonical_json(manifest_path, "linux-vst-bridge-wf0-artifact-manifest/v1")
        _validate_manifest_files(payload, manifest,
                                 sidecars={"ARTIFACT_MANIFEST.json", "ARTIFACT_MANIFEST.sha256"})
        if manifest != accepted_wa0_evidence()["artifact_manifest"]:
            fail("downloaded WA0 artifact manifest differs from accepted evidence")
        publish = parent / f".dx0-fixture-stage-{DX0_AGAIN_BUNDLE_MANIFEST_SHA256}"
        require_contained(publish, parent, "DX0 fixture publication stage")
        if publish.exists() or publish.is_symlink():
            fail("DX0 accepted-fixture publication outcome is unresolved")
        publish.mkdir(mode=0o700)
        shutil.copytree(payload / "again.vst3", publish / "again.vst3")
        identity = accepted_fixture_identity()
        write_atomic(publish / "DX0_ACCEPTED_FIXTURE_RECEIPT.json", canonical_json(identity), 0o400)
        receipt_digest = dx0_identity_sha256(identity)
        write_atomic(publish / "DX0_ACCEPTED_FIXTURE_RECEIPT.sha256",
                     f"{receipt_digest}  DX0_ACCEPTED_FIXTURE_RECEIPT.json\n".encode(), 0o400)
        for path in (item for item in publish.rglob("*") if item.is_file()):
            path.chmod(0o400)
        for directory in sorted((item for item in publish.rglob("*") if item.is_dir()), reverse=True):
            directory.chmod(0o500)
        publish.chmod(0o500)
        os.replace(publish, target)
    result = verify_fixture_store(target)
    result["disposition"] = "seeded"
    result["source_artifact_id"] = DX0_ACCEPTED_WA0_ARTIFACT_ID
    return result


def verify_host_payload(payload: pathlib.Path, destination: pathlib.Path,
                        build_input_sha256: str) -> dict[str, Any]:
    safe_extract(payload, destination)
    manifest_path = destination / "DX0_HOST_ARTIFACT_MANIFEST.json"
    digest = verify_hash_sidecar(destination / "DX0_HOST_ARTIFACT_MANIFEST.sha256", manifest_path)
    manifest = read_canonical_json(manifest_path, DX0_HOST_ARTIFACT_SCHEMA)
    if (manifest.get("windows_build_input_sha256") != build_input_sha256
            or manifest.get("build_identity_core_sha256")
            != sha256_file(destination / "DX0_BUILD_IDENTITY_CORE.json")):
        fail("DX0 host payload identity join differs")
    _validate_manifest_files(
        destination, manifest,
        sidecars={"DX0_HOST_ARTIFACT_MANIFEST.json", "DX0_HOST_ARTIFACT_MANIFEST.sha256"},
    )
    roles = {item["path"]: item["role"] for item in manifest["records"]}
    if roles.get("bin/wf0-factory-probe.exe") != "scanner_executable":
        fail("DX0 host payload scanner is absent")
    if any("again" in path.lower() or path.startswith("fixtures/") for path in roles):
        fail("DX0 host payload improperly contains a fixture")
    return {"manifest": manifest, "manifest_sha256": digest}


def publish_host_custody(wrapper: pathlib.Path, *, run: dict[str, Any],
                         artifact: dict[str, Any], upload_result: dict[str, Any],
                         producer_source: dict[str, Any],
                         build_input: dict[str, Any]) -> dict[str, Any]:
    build_input_sha = dx0_identity_sha256(build_input)
    expected_name = (
        f"dx0-windows-host-{build_input_sha}-run-{run['id']}-attempt-{run['run_attempt']}"
    )
    rest_digest = artifact.get("digest")
    bare_digest = upload_result.get("artifact_digest_bare")
    expected_human_url = (
        f"https://github.com/{REPOSITORY}/actions/runs/{run['id']}"
        f"/artifacts/{artifact.get('id')}"
    )
    expected_rest_url = (
        f"https://api.github.com/repos/{REPOSITORY}/actions/artifacts/"
        f"{artifact.get('id')}"
    )
    workflow_run = artifact.get("workflow_run")
    if (not re.fullmatch(r"[0-9a-f]{64}", str(bare_digest))
            or rest_digest != f"sha256:{bare_digest}"
            or upload_result.get("artifact_digest_rest_form") != rest_digest
            or sha256_file(wrapper) != bare_digest
            or artifact.get("name") != expected_name
            or artifact.get("expired") is not False
            or not isinstance(artifact.get("size_in_bytes"), int)
            or artifact.get("size_in_bytes") <= 0
            or artifact.get("id") != upload_result.get("artifact_id")
            or upload_result.get("artifact_name") != expected_name
            or upload_result.get("artifact_human_url") != expected_human_url
            or artifact.get("url") != expected_rest_url
            or not isinstance(workflow_run, dict)
            or workflow_run.get("id") != run["id"]
            or workflow_run.get("head_branch") != producer_source["ref"].removeprefix("refs/heads/")
            or workflow_run.get("head_sha") != producer_source["commit"]):
        fail("DX0 host Actions artifact digest/name closure differs")
    with tempfile.TemporaryDirectory(prefix="dx0-host-custody-") as temporary:
        stage = pathlib.Path(temporary)
        inner = stage / "inner"
        safe_extract(wrapper, inner, {"dx0-host-payload.zip",
                                      "DX0_WINDOWS_HOST_BUILD_RECEIPT.json",
                                      "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256"})
        receipt_path = inner / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json"
        receipt_sha = verify_hash_sidecar(
            inner / "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256", receipt_path
        )
        receipt = read_canonical_json(receipt_path, DX0_HOST_BUILD_SCHEMA)
        if (receipt.get("windows_build_input") != {
                "schema": build_input["schema"], "sha256": build_input_sha,
                "record_count": 17,
            } or receipt.get("producer_source") != producer_source
                or receipt.get("workflow", {}).get("run_id") != run["id"]
                or receipt.get("workflow", {}).get("run_attempt") != run["run_attempt"]
                or receipt.get("workflow", {}).get("phase_nonce")
                != upload_result.get("phase_nonce")
                or sha256_file(inner / "dx0-host-payload.zip")
                != receipt.get("artifact", {}).get("payload_sha256")):
            fail("DX0 host build receipt does not join producer/run/input")
        extracted = stage / "payload"
        payload_check = verify_host_payload(inner / "dx0-host-payload.zip", extracted,
                                            build_input_sha)
        if payload_check["manifest_sha256"] != receipt["artifact"]["manifest_sha256"]:
            fail("DX0 host manifest differs from build receipt")
        manifest_digest = payload_check["manifest_sha256"]
        target = dx0_mac_host_artifact_parent() / manifest_digest
        if target.exists() or target.is_symlink():
            return verify_host_store(target, build_input_sha)
        parent = dx0_mac_host_artifact_parent()
        parent.mkdir(parents=True, exist_ok=True)
        publish = parent / f".dx0-host-stage-{manifest_digest}"
        if publish.exists() or publish.is_symlink():
            fail("DX0 host-custody publication outcome is unresolved")
        shutil.copytree(extracted, publish)
        shutil.copy2(receipt_path, publish / receipt_path.name)
        shutil.copy2(inner / "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256",
                     publish / "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256")
        custody = {
            "schema": DX0_MAC_HOST_CUSTODY_SCHEMA,
            "repository": REPOSITORY,
            "workflow": {
                "path": ".github/workflows/wf0-windows-msvc-build.yml",
                "git_blob": run["workflow_blob"], "event": "workflow_dispatch",
                "head_branch": producer_source["ref"].removeprefix("refs/heads/"), "head_sha": producer_source["commit"],
                "run_id": run["id"], "run_attempt": run["run_attempt"],
                "phase_nonce": upload_result["phase_nonce"], "conclusion": "success",
            },
            "producer_source": producer_source,
            "windows_build_input_sha256": build_input_sha,
            "artifact": {
                "id": artifact["id"], "name": expected_name,
                "human_url": upload_result["artifact_human_url"],
                "rest_url": artifact["url"], "size_in_bytes": artifact["size_in_bytes"],
                "upload_digest_bare": bare_digest, "rest_digest": rest_digest,
                "raw_wrapper_sha256": sha256_file(wrapper),
            },
            "inner_envelope": {
                "payload_sha256": sha256_file(inner / "dx0-host-payload.zip"),
                "build_receipt_sha256": receipt_sha,
                "host_artifact_manifest_sha256": manifest_digest,
            },
            "explicit_nonclaims": ["no_signing", "no_attestation", "no_slsa",
                                   "no_immutable_runner_claim"],
        }
        write_atomic(publish / "DX0_MAC_HOST_CUSTODY_RECEIPT.json",
                     canonical_json(custody), 0o400)
        custody_sha = sha256_bytes(canonical_json(custody))
        write_atomic(publish / "DX0_MAC_HOST_CUSTODY_RECEIPT.sha256",
                     f"{custody_sha}  DX0_MAC_HOST_CUSTODY_RECEIPT.json\n".encode(), 0o400)
        for path in (item for item in publish.rglob("*") if item.is_file()):
            path.chmod(0o400)
        for directory in sorted((item for item in publish.rglob("*") if item.is_dir()), reverse=True):
            directory.chmod(0o500)
        publish.chmod(0o500)
        os.replace(publish, target)
    return verify_host_store(target, build_input_sha)


def verify_host_store(root: pathlib.Path, build_input_sha256: str) -> dict[str, Any]:
    if (root.parent not in {dx0_mac_host_artifact_parent(), dx0_deck_host_artifact_parent()}
            or not root.is_dir() or root.is_symlink()):
        fail("DX0 Mac host-artifact store root differs")
    manifest_path = root / "DX0_HOST_ARTIFACT_MANIFEST.json"
    manifest_sha = verify_hash_sidecar(root / "DX0_HOST_ARTIFACT_MANIFEST.sha256",
                                       manifest_path)
    if root.name != manifest_sha:
        fail("DX0 host store is not content addressed")
    manifest = read_canonical_json(manifest_path, DX0_HOST_ARTIFACT_SCHEMA)
    if manifest.get("windows_build_input_sha256") != build_input_sha256:
        fail("DX0 host store build-input identity differs")
    _validate_manifest_files(
        root, manifest,
        sidecars={"DX0_HOST_ARTIFACT_MANIFEST.json", "DX0_HOST_ARTIFACT_MANIFEST.sha256",
                  "DX0_WINDOWS_HOST_BUILD_RECEIPT.json",
                  "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256",
                  "DX0_MAC_HOST_CUSTODY_RECEIPT.json",
                  "DX0_MAC_HOST_CUSTODY_RECEIPT.sha256"},
    )
    build_receipt_path = root / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json"
    custody_path = root / "DX0_MAC_HOST_CUSTODY_RECEIPT.json"
    build_sha = verify_hash_sidecar(root / "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256",
                                    build_receipt_path)
    custody_sha = verify_hash_sidecar(root / "DX0_MAC_HOST_CUSTODY_RECEIPT.sha256",
                                      custody_path)
    build_receipt = read_canonical_json(build_receipt_path, DX0_HOST_BUILD_SCHEMA)
    custody = read_canonical_json(custody_path, DX0_MAC_HOST_CUSTODY_SCHEMA)
    core = read_canonical_json(
        root / "DX0_BUILD_IDENTITY_CORE.json",
        "linux-vst-bridge-dx0-build-identity-core/v1",
    )
    producer = build_receipt.get("producer_source")
    workflow = build_receipt.get("workflow")
    custody_workflow = custody.get("workflow")
    artifact = custody.get("artifact")
    expected_artifact_name = None
    if isinstance(workflow, dict):
        expected_artifact_name = (
            f"dx0-windows-host-{build_input_sha256}-run-{workflow.get('run_id')}"
            f"-attempt-{workflow.get('run_attempt')}"
        )
    if (build_receipt.get("artifact", {}).get("manifest_sha256") != manifest_sha
            or custody.get("inner_envelope", {}).get("host_artifact_manifest_sha256")
            != manifest_sha
            or custody.get("inner_envelope", {}).get("build_receipt_sha256") != build_sha
            or custody.get("inner_envelope", {}).get("payload_sha256")
            != build_receipt.get("artifact", {}).get("payload_sha256")
            or custody.get("windows_build_input_sha256") != build_input_sha256
            or build_receipt.get("repository") != REPOSITORY
            or build_receipt.get("windows_build_input") != {
                "schema": "linux-vst-bridge-dx0-windows-build-input/v1",
                "sha256": build_input_sha256, "record_count": 17,
            }
            or not isinstance(producer, dict)
            or custody.get("producer_source") != producer
            or core.get("producer_source") != producer
            or core.get("windows_build_input_sha256") != build_input_sha256
            or not isinstance(core.get("windows_build_input"), dict)
            or dx0_identity_sha256(core["windows_build_input"]) != build_input_sha256
            or core.get("workflow") != workflow
            or not isinstance(workflow, dict)
            or workflow.get("event") != "workflow_dispatch"
            or workflow.get("ref") != source_contract(ref=producer.get("ref"))["ref"]
            or workflow.get("source_sha") != producer.get("commit")
            or workflow.get("host_mode") != "host_only"
            or not isinstance(workflow.get("run_id"), int)
            or workflow.get("run_id") <= 0
            or not isinstance(workflow.get("run_attempt"), int)
            or workflow.get("run_attempt") <= 0
            or not re.fullmatch(r"[0-9a-f]{32}", str(workflow.get("phase_nonce")))
            or not isinstance(custody_workflow, dict)
            or custody_workflow.get("path") != workflow.get("path")
            or custody_workflow.get("git_blob") != workflow.get("git_blob")
            or custody_workflow.get("event") != workflow.get("event")
            or custody_workflow.get("head_branch") != source_contract(ref=producer.get("ref"))["branch"]
            or custody_workflow.get("head_sha") != producer.get("commit")
            or custody_workflow.get("run_id") != workflow.get("run_id")
            or custody_workflow.get("run_attempt") != workflow.get("run_attempt")
            or custody_workflow.get("phase_nonce") != workflow.get("phase_nonce")
            or custody_workflow.get("conclusion") != "success"
            or not isinstance(artifact, dict)
            or artifact.get("name") != expected_artifact_name
            or not isinstance(artifact.get("id"), int) or artifact.get("id") <= 0
            or not isinstance(artifact.get("size_in_bytes"), int)
            or artifact.get("size_in_bytes") <= 0
            or artifact.get("rest_digest")
            != f"sha256:{artifact.get('upload_digest_bare')}"
            or artifact.get("raw_wrapper_sha256") != artifact.get("upload_digest_bare")
            or not re.fullmatch(r"[0-9a-f]{64}", str(artifact.get("upload_digest_bare")))
            or artifact.get("human_url")
            != f"https://github.com/{REPOSITORY}/actions/runs/{workflow.get('run_id')}"
               f"/artifacts/{artifact.get('id')}"
            or artifact.get("rest_url")
            != f"https://api.github.com/repos/{REPOSITORY}/actions/artifacts/"
               f"{artifact.get('id')}"):
        fail("DX0 host store receipt closure differs")
    for path in root.rglob("*"):
        if stat.S_IMODE(path.stat().st_mode) & 0o222:
            fail("DX0 Mac host-artifact store is writable")
    return {"root": str(root), "manifest": manifest, "manifest_sha256": manifest_sha,
            "build_receipt": build_receipt, "build_receipt_sha256": build_sha,
            "custody": custody, "custody_sha256": custody_sha, "disposition": "reused"}


def _bundle_header(path: pathlib.Path) -> tuple[int, list[tuple[str, str]]]:
    if (not path.is_file() or path.is_symlink()
            or path.stat().st_size > 128 * 1024 * 1024):
        fail("source bundle is absent, unsafe, or oversized")
    output = command(["git", "bundle", "list-heads", str(path)]).stdout.decode("utf-8", "strict")
    refs: list[tuple[str, str]] = []
    for line in output.splitlines():
        fields = line.split()
        if len(fields) != 2 or not re.fullmatch(r"[0-9a-f]{40}", fields[0]):
            fail("source bundle advertised-ref header is malformed")
        refs.append((fields[0], fields[1]))
    verify = command(["git", "bundle", "verify", str(path)], cwd=repo_root())
    text = (verify.stdout + verify.stderr).decode("utf-8", "replace").lower()
    prerequisites = sum(1 for line in text.splitlines() if line.startswith("-"))
    if "complete history" not in text:
        fail("source bundle does not report complete history")
    return prerequisites, refs


def _source_from_bundle(bundle: pathlib.Path, source_commit: str,
                        advertised_ref: str) -> dict[str, Any]:
    """Reproduce E from its self-contained bundle, not ambient Git retention."""
    with tempfile.TemporaryDirectory(prefix="dx0-source-verify-") as temporary:
        bare = pathlib.Path(temporary) / "verify.git"
        command(["git", "init", "--bare", str(bare)])
        command([
            "git", "-C", str(bare), "fetch", "--no-tags", str(bundle),
            f"{advertised_ref}:refs/verify/source",
        ])
        imported = command([
            "git", "-C", str(bare), "rev-parse", "refs/verify/source",
        ]).stdout.decode("utf-8", "strict").strip()
        if imported != source_commit:
            fail("DX0 source bundle imported commit differs")
        return dx0_complete_source(source_commit, root=bare)


def create_source_handoff(source_commit: str, stage: pathlib.Path) -> dict[str, Any]:
    source = dx0_complete_source(source_commit)
    role = dx0_source_role(source)
    contract = source_contract(ref=source["ref"])
    if stage.exists() or stage.is_symlink():
        fail("DX0 source-handoff stage already exists")
    stage.mkdir(parents=True)
    advertised_ref = f"refs/handoff/dx0-source/{source_commit}"
    bundle_name = f"dx0-execution-source-{source_commit}.bundle"
    bundle = stage / bundle_name
    with tempfile.TemporaryDirectory(prefix="dx0-source-bare-") as temporary:
        bare = pathlib.Path(temporary) / "handoff.git"
        command(["git", "init", "--bare", str(bare)])
        command(["git", "-C", str(bare), "fetch", "--no-tags", str(repo_root()), source_commit])
        command(["git", "-C", str(bare), "update-ref", advertised_ref, source_commit])
        command(["git", "-C", str(bare), "bundle", "create", str(bundle), advertised_ref])
    prerequisites, refs = _bundle_header(bundle)
    if (prerequisites != 0 or refs != [(source_commit, advertised_ref)]
            or bundle.stat().st_size > 128 * 1024 * 1024):
        fail("DX0 source bundle is not one self-contained advertised ref")
    receipt = {
        "schema": DX0_SOURCE_HANDOFF_SCHEMA, "repository": REPOSITORY,
        "design_authority": source_authority(source),
        "implementation_basis": {
            "commit": contract["basis"], "tree": contract["basis_tree"],
        },
        "implementation_source": role, "implementation_branch": contract["branch"],
        "implementation_ref": source["ref"], "complete_source_schema": source["schema"],
        "complete_source_record_count": source["record_count"],
        "bundle": {"name": bundle_name, "advertised_ref": advertised_ref,
                   "sha256": sha256_file(bundle), "size": bundle.stat().st_size,
                   "prerequisite_count": 0, "advertised_ref_count": 1,
                   "git_bundle_verify": "passed"},
    }
    receipt_path = stage / "DX0_SOURCE_HANDOFF_RECEIPT.json"
    write_atomic(receipt_path, canonical_json(receipt))
    digest = sha256_file(receipt_path)
    write_atomic(stage / "DX0_SOURCE_HANDOFF_RECEIPT.sha256",
                 f"{digest}  DX0_SOURCE_HANDOFF_RECEIPT.json\n".encode())
    return {"receipt": receipt, "receipt_sha256": digest, "stage": str(stage)}


def verify_source_handoff(stage: pathlib.Path, source_commit: str) -> dict[str, Any]:
    expected_names = {f"dx0-execution-source-{source_commit}.bundle",
                      "DX0_SOURCE_HANDOFF_RECEIPT.json",
                      "DX0_SOURCE_HANDOFF_RECEIPT.sha256"}
    if (not stage.is_dir() or stage.is_symlink()
            or {path.name for path in stage.iterdir()} != expected_names):
        fail("DX0 source-handoff roster differs")
    receipt_path = stage / "DX0_SOURCE_HANDOFF_RECEIPT.json"
    receipt_sha = verify_hash_sidecar(stage / "DX0_SOURCE_HANDOFF_RECEIPT.sha256",
                                      receipt_path)
    receipt = read_canonical_json(receipt_path, DX0_SOURCE_HANDOFF_SCHEMA)
    bundle = stage / f"dx0-execution-source-{source_commit}.bundle"
    prerequisites, refs = _bundle_header(bundle)
    expected_ref = f"refs/handoff/dx0-source/{source_commit}"
    source = _source_from_bundle(bundle, source_commit, expected_ref)
    contract = source_contract(ref=source["ref"])
    if (receipt.get("repository") != REPOSITORY
            or receipt.get("design_authority") != source_authority(source)
            or receipt.get("implementation_basis") != {
                "commit": contract["basis"], "tree": contract["basis_tree"],
            }
            or receipt.get("implementation_source") != dx0_source_role(source)
            or receipt.get("implementation_branch") != contract["branch"]
            or receipt.get("implementation_ref") != source["ref"]
            or receipt.get("complete_source_schema") != source["schema"]
            or receipt.get("complete_source_record_count") != source["record_count"]
            or receipt.get("bundle", {}).get("sha256") != sha256_file(bundle)
            or receipt.get("bundle", {}).get("name") != bundle.name
            or receipt.get("bundle", {}).get("advertised_ref") != expected_ref
            or receipt.get("bundle", {}).get("size") != bundle.stat().st_size
            or bundle.stat().st_size > 128 * 1024 * 1024
            or receipt.get("bundle", {}).get("prerequisite_count") != 0
            or receipt.get("bundle", {}).get("advertised_ref_count") != 1
            or receipt.get("bundle", {}).get("git_bundle_verify") != "passed"
            or prerequisites != 0 or refs != [(source_commit, expected_ref)]):
        fail("DX0 source-handoff identity differs")
    return {"receipt": receipt, "receipt_sha256": receipt_sha, "bundle": str(bundle)}


if __name__ == "__main__":
    raise SystemExit("artifacts.py is a library; use tools/host-proof.py")

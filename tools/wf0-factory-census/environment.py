#!/usr/bin/env python3
"""Marker-bound cache copy, scan-environment verification, and retirement for WF0."""

from __future__ import annotations

import json
import os
import pathlib
import re
import shutil
import stat
from dataclasses import dataclass
from typing import Any

from common import (
    BUNDLE_SCHEMA, MARKER_SCHEMA, RUNNER_DIGEST, artifact_cache_parent,
    canonical_json, environment_parent, fail, require_contained, sha256_bytes,
    sha256_file, write_atomic,
)


@dataclass(frozen=True)
class ScanEnvironment:
    run_id: str
    root: pathlib.Path
    marker: dict[str, Any]

    @property
    def compatdata(self) -> pathlib.Path:
        return self.root / "compatdata"

    @property
    def prefix(self) -> pathlib.Path:
        return self.compatdata / "pfx"

    @property
    def wf0(self) -> pathlib.Path:
        return self.prefix / "drive_c/wf0"

    @property
    def scanner(self) -> pathlib.Path:
        return self.wf0 / "bin/wf0-factory-probe.exe"

    @property
    def adapter(self) -> pathlib.Path:
        return self.wf0 / "bin/wf0-loader-adapter-tests.exe"

    @property
    def module(self) -> pathlib.Path:
        return self.wf0 / "fixture/again.vst3/Contents/x86_64-win/again.vst3"

    @property
    def session(self) -> pathlib.Path:
        return self.wf0 / "session"


def _copy_regular(source: pathlib.Path, destination: pathlib.Path) -> None:
    if not source.is_file() or source.is_symlink() or not stat.S_ISREG(source.stat().st_mode):
        fail(f"payload source is absent or unsafe: {source.name}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    destination.chmod(0o500 if destination.suffix.lower() in {".exe", ".dll", ".vst3"} else 0o400)
    if sha256_file(source) != sha256_file(destination):
        fail(f"payload copy digest mismatch: {destination.name}")


def _bundle_manifest(bundle: pathlib.Path) -> dict[str, Any]:
    records: list[dict[str, Any]] = []
    for path in sorted(bundle.rglob("*"), key=lambda item: item.relative_to(bundle).as_posix().encode()):
        if path.is_symlink() or (path.exists() and not (path.is_file() or path.is_dir())):
            fail("staged bundle contains an unsafe object")
        if path.is_file():
            records.append({
                "path": path.relative_to(bundle).as_posix(),
                "size": path.stat().st_size,
                "sha256": sha256_file(path),
            })
    value = {
        "schema": BUNDLE_SCHEMA,
        "binary_safe_path": "again.vst3/Contents/x86_64-win/again.vst3",
        "records": records,
    }
    value["sha256"] = sha256_bytes(canonical_json(value))
    return value


def verify_artifact_cache(build: dict[str, Any]) -> pathlib.Path:
    digest = build["artifact_manifest_sha256"]
    root = pathlib.Path(build["artifact_root"])
    if root != artifact_cache_parent() / digest or not root.is_dir() or root.is_symlink():
        fail("artifact cache root identity differs")
    manifest = build["artifact_manifest"]
    if (
        manifest.get("implementation_source_manifest_sha256")
        != build["implementation_source_manifest_sha256"]
        or manifest.get("record_count") != len(manifest.get("records", []))
    ):
        fail("artifact cache manifest/source identity differs")
    declared = {
        item["path"]: item for item in manifest["records"]
    }
    for relative, record in declared.items():
        path = root / relative
        if (
            not path.is_file() or path.is_symlink()
            or path.stat().st_size != record["size"]
            or sha256_file(path) != record["sha256"]
        ):
            fail(f"artifact cache record changed: {relative}")
    if sha256_file(root / "ARTIFACT_MANIFEST.json") != digest:
        fail("artifact cache manifest digest differs from directory identity")
    if (
        (root / "ARTIFACT_MANIFEST.json").read_bytes() != canonical_json(manifest)
        or (root / "ARTIFACT_MANIFEST.sha256").read_bytes()
        != f"{digest}  ARTIFACT_MANIFEST.json\n".encode()
    ):
        fail("artifact cache canonical manifest or sidecar differs")
    required_sidecars = {
        "ARTIFACT_MANIFEST.json", "ARTIFACT_MANIFEST.sha256",
        "WF0_WINDOWS_BUILD_RECEIPT.json", "WF0_WINDOWS_BUILD_RECEIPT.sha256",
        "WF0_MAC_ARTIFACT_CUSTODY_RECEIPT.json", ".wf0-artifact-owner.json",
    }
    observed = {
        item.relative_to(root).as_posix() for item in root.rglob("*") if item.is_file()
    }
    if observed != set(declared) | required_sidecars:
        fail(f"artifact cache complete roster differs: {sorted(observed ^ (set(declared) | required_sidecars))}")
    receipt_path = root / "WF0_WINDOWS_BUILD_RECEIPT.json"
    custody_path = root / "WF0_MAC_ARTIFACT_CUSTODY_RECEIPT.json"
    core_path = root / "BUILD_IDENTITY_CORE.json"
    marker_path = root / ".wf0-artifact-owner.json"
    try:
        receipt = json.loads(receipt_path.read_bytes())
        custody = json.loads(custody_path.read_bytes())
        core = json.loads(core_path.read_bytes())
        marker = json.loads(marker_path.read_bytes())
    except (json.JSONDecodeError, UnicodeDecodeError):
        fail("artifact cache contains malformed JSON")
    receipt_sha256 = sha256_file(receipt_path)
    custody_sha256 = sha256_file(custody_path)
    expected_marker = {
        "schema": "linux-vst-bridge-wf0-artifact-cache-owner/v1",
        "artifact_manifest_sha256": digest,
        "source_commit": build["source_commit"],
        "implementation_source_manifest_sha256":
            build["implementation_source_manifest_sha256"],
        "windows_build_receipt_sha256": receipt_sha256,
        "mac_custody_receipt_sha256": custody_sha256,
    }
    if (
        receipt_path.read_bytes() != canonical_json(receipt)
        or custody_path.read_bytes() != canonical_json(custody)
        or core_path.read_bytes() != canonical_json(core)
        or marker_path.read_bytes() != canonical_json(marker)
        or receipt != build["build_receipt"]
        or custody != build["mac_custody"]
        or core != build["build_identity_core"]
        or marker != expected_marker
        or (root / "WF0_WINDOWS_BUILD_RECEIPT.sha256").read_bytes()
        != f"{receipt_sha256}  WF0_WINDOWS_BUILD_RECEIPT.json\n".encode()
        or sha256_file(core_path) != manifest.get("build_identity_core_sha256")
    ):
        fail("artifact cache sidecar/owner/source custody closure differs")
    for path in root.rglob("*"):
        if path.is_file() and stat.S_IMODE(path.stat().st_mode) & 0o222:
            fail(f"artifact cache file is writable: {path.name}")
        if path.is_dir() and stat.S_IMODE(path.stat().st_mode) & 0o222:
            fail(f"artifact cache directory is writable: {path.name}")
    return root


def create_environment(run_id: str, build: dict[str, Any], *, fixture: str) -> ScanEnvironment:
    if not re.fullmatch(r"[0-9a-f]{32}", run_id):
        fail("WF0 run ID must be 32 lowercase hexadecimal characters")
    artifact_root = verify_artifact_cache(build)
    parent = environment_parent()
    parent.mkdir(parents=True, exist_ok=True)
    root = parent / f".wf0-factory-census.stage-{run_id}"
    require_contained(root, parent, "WF0 stage")
    if root.exists() or root.is_symlink():
        fail("WF0 stage already exists; adoption is prohibited")
    root.mkdir(mode=0o700)

    for relative in (
        "compatdata/pfx/drive_c/wf0/bin", "compatdata/pfx/drive_c/wf0/fixture",
        "compatdata/pfx/drive_c/wf0/session", "runtime-var", "host-cache", "host-config",
        "host-data", "host-tmp",
    ):
        (root / relative).mkdir(parents=True, mode=0o700, exist_ok=True)
    wf0 = root / "compatdata/pfx/drive_c/wf0"
    _copy_regular(
        artifact_root / "bin/wf0-factory-probe.exe",
        wf0 / "bin/wf0-factory-probe.exe",
    )
    if fixture == "adapter":
        _copy_regular(
            artifact_root / "bin/wf0-loader-adapter-tests.exe",
            wf0 / "bin/wf0-loader-adapter-tests.exe",
        )

    bundle = wf0 / "fixture/again.vst3"
    if fixture == "again":
        positive = [
            item for item in build["artifact_manifest"]["records"]
            if item["path"].startswith("again.vst3/")
        ]
        if not positive:
            fail("artifact manifest has no exact AGain bundle")
        for record in positive:
            relative = record["path"][len("again.vst3/"):]
            _copy_regular(artifact_root / record["path"], bundle / relative)
    else:
        selected_fixture = "wf0-no-entry" if fixture == "adapter" else fixture
        if selected_fixture not in build["fault_targets"]:
            fail(f"fault fixture is outside the approved roster: {fixture}")
        _copy_regular(
            artifact_root / f"fixtures/{selected_fixture}.dll",
            bundle / "Contents/x86_64-win/again.vst3",
        )
    bundle_identity = _bundle_manifest(bundle)

    marker = {
        "schema": MARKER_SCHEMA,
        "run_id": run_id,
        "fixture": fixture,
        "source_commit": build["source_commit"],
        "implementation_source_manifest_sha256":
            build["implementation_source_manifest_sha256"],
        "artifact_manifest_sha256": build["artifact_manifest_sha256"],
        "runner_identity_sha256": RUNNER_DIGEST,
        "scanner_sha256": sha256_file(wf0 / "bin/wf0-factory-probe.exe"),
        "module_sha256": sha256_file(bundle / "Contents/x86_64-win/again.vst3"),
        "bundle_manifest": bundle_identity,
    }
    if fixture == "adapter":
        marker["adapter_sha256"] = sha256_file(
            wf0 / "bin/wf0-loader-adapter-tests.exe"
        )
    write_atomic(root / ".wf0-owner.json", canonical_json(marker))
    environment = ScanEnvironment(run_id=run_id, root=root, marker=marker)
    verify_environment(environment)
    return environment


def verify_environment(environment: ScanEnvironment) -> None:
    parent = environment_parent()
    require_contained(environment.root, parent, "WF0 stage")
    expected_name = f".wf0-factory-census.stage-{environment.run_id}"
    if environment.root.name != expected_name or environment.root.is_symlink():
        fail("WF0 stage identity mismatch")
    marker_path = environment.root / ".wf0-owner.json"
    if json.loads(marker_path.read_bytes()) != environment.marker:
        fail("WF0 marker readback mismatch")
    if environment.marker.get("runner_identity_sha256") != RUNNER_DIGEST:
        fail("WF0 marker runner identity differs")
    if sha256_file(environment.scanner) != environment.marker["scanner_sha256"]:
        fail("staged scanner changed")
    if environment.marker["fixture"] == "adapter" and (
        not environment.adapter.is_file() or environment.adapter.is_symlink()
        or sha256_file(environment.adapter) != environment.marker.get("adapter_sha256")
    ):
        fail("staged loader adapter changed or disappeared")
    if sha256_file(environment.module) != environment.marker["module_sha256"]:
        fail("staged module changed")
    current_bundle = _bundle_manifest(environment.module.parents[2])
    if current_bundle != environment.marker["bundle_manifest"]:
        fail("staged bundle manifest changed")


def retire_environment(environment: ScanEnvironment) -> dict[str, Any]:
    verify_environment(environment)
    parent = environment_parent()
    require_contained(environment.root, parent, "WF0 retirement stage")
    shutil.rmtree(environment.root)
    if environment.root.exists() or environment.root.is_symlink():
        fail("WF0 stage retirement did not reach exact absence")
    descriptor = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    return {
        "environment_retired": True,
        "stage_absent": True,
        "run_id": environment.run_id,
        "fixture": environment.marker["fixture"],
    }


if __name__ == "__main__":
    raise SystemExit("environment.py is a library; run run.py")

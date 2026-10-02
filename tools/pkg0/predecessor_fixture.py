#!/usr/bin/env python3
"""Build a nonrelease six-artifact PKG0 adoption fixture for PB0-R3.

The real package builder owns the complete customer archive. This tool only
binds an exact paired Linux build, canonical supervisor source, and the
unchanged selected host pair into the six-file manifest consumed by the
read-only predecessor gate. No files are installed or signed.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import py_compile
import subprocess
import sys
import tarfile
from assemble import parse_operator_schema

NAMES = ("linux-vst-bridge", "linux-audio-compatibility-manager",
         "session.pyc", "ownership.pyc", "host.exe",
         "host-source-manifest.json")


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value: dict) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def checked(path: Path, expected: str | None = None) -> bytes:
    if path.is_symlink() or not path.is_file():
        raise ValueError("fixture input must be a regular file")
    data = path.read_bytes()
    if expected is not None and digest(data) != expected:
        raise ValueError("fixture input digest changed")
    return data


def run(repo: Path, pair: Path, host: Path, host_source: Path,
        external: Path, output: Path, version: str) -> dict:
    if sys.version_info[:2] != (3, 13):
        raise ValueError("canonical supervisor bytecode requires Python 3.13")
    if output.exists():
        raise ValueError("fixture output already exists")
    pair_manifest = json.loads(checked(pair / "manifest.json"))
    head, tree = pair_manifest["source_head"], pair_manifest["source_tree"]
    actual_tree = subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", f"{head}^{{tree}}"], text=True).strip()
    if actual_tree != tree:
        raise ValueError("paired build source tree changed")
    operator_schema = parse_operator_schema(subprocess.check_output(
        ["git", "-C", str(repo), "show", f"{head}:bridge-manager/src/operator_model.rs"],
        text=True))
    if not version or not version[0].isdigit() or not version.replace(".", "").isalnum():
        raise ValueError("fixture package version")
    external_bytes = checked(external)
    external_record = json.loads(external_bytes)
    if (set(external_record) != {"schema", "runners"}
            or external_record["schema"] != 1 or not external_record["runners"]):
        raise ValueError("external runtime roster")
    output.mkdir(mode=0o700)
    files = {}
    for name in NAMES[:2]:
        files[name] = checked(pair / name, pair_manifest["sha256"][name])
        if not files[name].startswith(b"\x7fELF"):
            raise ValueError("paired Linux executable format")
    for stem in ("session", "ownership"):
        source = subprocess.check_output(
            ["git", "-C", str(repo), "show", f"{head}:bridge-manager/runtime/{stem}.py"])
        source_path = output / f".{stem}.source.py"
        source_path.write_bytes(source)
        target = output / f"{stem}.pyc"
        try:
            py_compile.compile(str(source_path), cfile=str(target),
                dfile=f"bridge-manager/runtime/{stem}.py", doraise=True,
                invalidation_mode=py_compile.PycInvalidationMode.CHECKED_HASH)
            files[target.name] = target.read_bytes()
        finally:
            source_path.unlink()
    files["host.exe"] = checked(host)
    files["host-source-manifest.json"] = checked(host_source)
    if not files["host.exe"].startswith(b"MZ"):
        raise ValueError("selected Windows host format")
    json.loads(files["host-source-manifest.json"])
    for name, data in files.items():
        (output / name).write_bytes(data)
    manifest = {
        "schema": 1, "package": "linux-vst-bridge-beta", "version": version,
        "pkgrel": 1, "source_head": head, "source_tree": tree,
        "operator_schema": operator_schema,
        "files": [{"name": name, "sha256": digest(files[name]),
                   "size": len(files[name])} for name in NAMES],
        "external_runtime": {
            "id": "selected-external-proton-slr-set",
            "manifest_sha256": digest(external_bytes),
        },
    }
    manifest_bytes = canonical(manifest)
    (output / "pkg0-manifest.json").write_bytes(manifest_bytes)
    archive_path = output / "candidate-adoption.tar"
    with tarfile.open(archive_path, "w", format=tarfile.PAX_FORMAT) as archive:
        for name in (*NAMES, "pkg0-manifest.json"):
            data = (output / name).read_bytes()
            item = tarfile.TarInfo(name)
            item.size = len(data)
            item.mode = 0o400
            item.uid = item.gid = item.mtime = 0
            archive.addfile(item, io.BytesIO(data))
    with tarfile.open(archive_path) as archive:
        if archive.getnames() != [*NAMES, "pkg0-manifest.json"]:
            raise ValueError("candidate adoption roster")
        for name in (*NAMES, "pkg0-manifest.json"):
            if archive.extractfile(name).read() != (output / name).read_bytes():
                raise ValueError("candidate adoption bytes changed")
    return {"source_head": head, "source_tree": tree,
            "manifest_sha256": digest(manifest_bytes),
            "archive_sha256": digest(archive_path.read_bytes()),
            "artifact_sha256": {name: digest(files[name]) for name in NAMES},
            "external_runtime_manifest_sha256": digest(external_bytes)}


def main() -> None:
    parser = argparse.ArgumentParser()
    for name in ("repo", "pair", "host", "host-source", "external", "output"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    print(json.dumps(run(args.repo, args.pair, args.host, args.host_source,
                         args.external, args.output, args.version), sort_keys=True))


if __name__ == "__main__":
    main()

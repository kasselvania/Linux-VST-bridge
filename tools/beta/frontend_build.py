#!/usr/bin/env python3
"""Build an internal frontend from exact public Bridge and private UI sources.

Only public source identities, relative manifest names and hashes enter the
record. The private checkout and Cargo cache stay on the build machine.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import urllib.parse

TARGET = "x86_64-unknown-linux-gnu"
BINARY = "linux-audio-compatibility-manager"
BRIDGE_REPOSITORY = "https://github.com/kasselvania/Linux-VST-bridge"
BRIDGE_CRATES = {"lvb-model", "lvb-view-model", "lvb-client"}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


def source_identity(root):
    root = Path(root).resolve(strict=True)
    if Path(git(root, "rev-parse", "--show-toplevel")).resolve() != root:
        raise ValueError("source must be a checkout root")
    if git(root, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("commit exact frontend build sources first")
    return {"head": git(root, "rev-parse", "HEAD"),
            "tree": git(root, "rev-parse", "HEAD^{tree}")}


def release_metadata(manifest):
    return json.loads(subprocess.check_output([
        "cargo", "metadata", "--locked", "--format-version", "1",
        "--filter-platform", TARGET, "--no-default-features", "--features", "live",
        "--manifest-path", str(manifest)]))


def release_packages(metadata):
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    todo, seen = [metadata["resolve"]["root"]], set()
    while todo:
        identity = todo.pop()
        if identity in seen:
            continue
        seen.add(identity)
        todo.extend(dep["pkg"] for dep in nodes[identity]["deps"]
                    if any(kind["kind"] != "dev" for kind in dep["dep_kinds"]))
    return [package for package in metadata["packages"] if package["id"] in seen]


def bridge_dependency(source, head):
    if not isinstance(source, str) or not source.startswith("git+"):
        return False
    url = urllib.parse.urlsplit(source[4:])
    return (urllib.parse.urlunsplit((url.scheme, url.netloc,
                                    url.path.removesuffix(".git"), "", "")) == BRIDGE_REPOSITORY
            and urllib.parse.parse_qs(url.query) == {"rev": [head]}
            and url.fragment == head)


def describe(bridge, frontend, manifest_name, public_key, key_class):
    bridge, frontend = Path(bridge).resolve(), Path(frontend).resolve()
    public, private = source_identity(bridge), source_identity(frontend)
    relative = Path(manifest_name)
    if relative.is_absolute() or ".." in relative.parts or relative.name != "Cargo.toml":
        raise ValueError("frontend manifest must be a relative Cargo.toml")
    manifest = frontend / relative
    if manifest.resolve() != manifest or not manifest.is_file():
        raise ValueError("frontend manifest custody")
    if not re.fullmatch(r"[0-9a-f]{64}", public_key) or key_class != "internal_test":
        raise ValueError("qualification frontend requires an internal public trust root")
    metadata = release_metadata(manifest)
    if Path(metadata["workspace_root"]).resolve() != manifest.parent:
        raise ValueError("qualification frontend requires its own locked workspace")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root_id = metadata["resolve"]["root"]
    if set(nodes[root_id]["features"]) != {"live"}:
        raise ValueError("qualification frontend must contain only the live feature")
    packages = release_packages(metadata)
    found, checked = set(), set()
    for package in packages:
        source = package.get("source")
        if source and source.startswith("registry+"):
            continue
        package_root = Path(package["manifest_path"]).resolve().parent
        if source:
            if not bridge_dependency(source, public["head"]):
                raise ValueError("frontend dependency is not the exact public Bridge revision")
            checkout = Path(git(package_root, "rev-parse", "--show-toplevel")).resolve()
            if checkout not in checked:
                if source_identity(checkout) != public:
                    raise ValueError("resolved Bridge source tree differs from package source")
                checked.add(checkout)
            if package["name"] in BRIDGE_CRATES:
                found.add(package["name"])
        elif not package_root.is_relative_to(frontend):
            raise ValueError("frontend path dependency escapes exact private source")
        if package["name"] == "lvb-view-model" and "samples" in nodes[package["id"]]["features"]:
            raise ValueError("sample fixtures cannot enter the qualification frontend")
    if found != BRIDGE_CRATES:
        raise ValueError("frontend must use exact public model, view model and client")
    root = next(package for package in packages if package["id"] == root_id)
    if not any(target["name"] == BINARY and "bin" in target["kind"] for target in root["targets"]):
        raise ValueError("frontend must declare the standard product executable")
    private.update(manifest=relative.as_posix(), manifest_sha256=sha(manifest),
                   cargo_lock_sha256=sha(manifest.with_name("Cargo.lock")))
    return {"schema": 1, "bridge_source": public, "frontend_source": private,
            "target": TARGET, "binary": BINARY, "features": ["live"],
            "portable_public_key": public_key, "portable_key_class": key_class}


def verify_record(record, bridge, frontend, binary):
    metadata = {key: value for key, value in record.items() if key != "artifact_sha256"}
    actual = describe(bridge, frontend, metadata["frontend_source"]["manifest"],
                      metadata["portable_public_key"], metadata["portable_key_class"])
    if metadata != actual or record.get("artifact_sha256") != sha(binary):
        raise ValueError("frontend build sources or artifact changed")
    embedded = json.loads(subprocess.check_output([str(Path(binary).resolve()), "--package-info"],
                                                 timeout=15))
    if embedded != metadata:
        raise ValueError("frontend embedded build identity differs")
    return record


def build(bridge, frontend, manifest, output, target_dir, public_key, key_class):
    output = Path(output)
    if output.exists():
        raise ValueError("frontend build output already exists")
    metadata = describe(bridge, frontend, manifest, public_key, key_class)
    env = {**os.environ, "CARGO_BUILD_JOBS": "2", "CARGO_TARGET_DIR": str(Path(target_dir).resolve()),
           "LVB_PORTABLE_ED25519_PUBLIC_KEY": public_key, "LVB_PORTABLE_KEY_CLASS": key_class,
           "MAKEPAD_PACKAGE_DIR": ".",
           "LVB_FRONTEND_PACKAGE_INFO": json.dumps(metadata, sort_keys=True, separators=(",", ":"))}
    subprocess.run(["cargo", "build", "--release", "--locked", "--target", TARGET,
                    "--no-default-features", "--features", "live", "--bin", BINARY,
                    "--manifest-path", str(Path(frontend).resolve() / manifest)], env=env, check=True)
    binary = Path(target_dir).resolve() / TARGET / "release" / BINARY
    data = binary.read_bytes()
    if not data.startswith(b"\x7fELF") or public_key.encode() not in data:
        raise ValueError("frontend ELF or compiled public trust root absent")
    record = {**metadata, "artifact_sha256": sha(binary)}
    verify_record(record, bridge, frontend, binary)
    output.mkdir(parents=True)
    shutil.copyfile(binary, output / BINARY)
    (output / BINARY).chmod(0o555)
    (output / "frontend-build.json").write_text(json.dumps(record, sort_keys=True, indent=2) + "\n")
    return record


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bridge-source", type=Path, required=True)
    parser.add_argument("--frontend-source", type=Path, required=True)
    parser.add_argument("--frontend-manifest", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--public-key", required=True)
    args = parser.parse_args()
    print(json.dumps(build(args.bridge_source, args.frontend_source, args.frontend_manifest,
                           args.output, args.target_dir, args.public_key, "internal_test"), sort_keys=True))


if __name__ == "__main__":
    main()

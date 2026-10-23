#!/usr/bin/env python3
"""Build the Bridge-owned window with exact private toolkit provenance.

Only public source identities, relative manifest names and hashes enter the
record. The private checkout and Cargo cache stay on the build machine.
"""
import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib
import urllib.parse

TARGET = "x86_64-unknown-linux-gnu"
BINARY = "linux-audio-compatibility-manager"
TOOLKIT_REPOSITORY = "https://github.com/kasselvania/makepad"
WINDOW_MANIFEST = Path("crates/window/Cargo.toml")
TOOLKIT_CRATES = {"makepad-widgets", "makepad-product-ui"}
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
    return git_identity(root)


def git_identity(root):
    return {"head": git(root, "rev-parse", "HEAD"),
            "tree": git(root, "rev-parse", "HEAD^{tree}")}


def tracked_files(root):
    """The recorded tree, not ignored files that happen to be beside it."""
    return {Path(name) for name in subprocess.check_output([
        "git", "-C", str(root), "ls-tree", "-rz", "--name-only", "HEAD"
    ]).decode().split("\0") if name}


def tracked_blob(root, relative):
    relative = Path(relative)
    entry = subprocess.check_output([
        "git", "-C", str(root), "ls-tree", "HEAD", "--", relative.as_posix()
    ]).decode().split()
    if len(entry) < 3 or entry[0] not in ("100644", "100755") or entry[1] != "blob":
        raise ValueError("source input is not a tracked regular file")
    return subprocess.check_output(["git", "-C", str(root), "cat-file", "blob", entry[2]])


@contextmanager
def recorded_sources(bridge, frontend):
    """Materialize commits and let Cargo extract dependencies into a fresh area.

    Existing worktrees and Cargo Git checkouts may contain ignored source files.
    Only registry archives matching a recorded lockfile are copied as cache
    inputs. No existing dependency source directory is reused.
    """
    identities = [source_identity(root) for root in (bridge, frontend)]
    with tempfile.TemporaryDirectory(prefix="lvb-frontend-") as temporary:
        temporary = Path(temporary)
        roots = []
        for name, original, identity in zip(("bridge", "frontend"), (bridge, frontend), identities):
            root = temporary / name
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "fetch", "-q", "--depth=1", "--no-tags",
                            str(Path(original).resolve()), identity["head"]], check=True)
            subprocess.run(["git", "-C", str(root), "checkout", "-q", "--detach", identity["head"]], check=True)
            if source_identity(root) != identity:
                raise ValueError("materialized source identity differs")
            roots.append(root)
        cargo = temporary / "cargo"
        registry = cargo / "registry"
        registry.mkdir(parents=True)
        existing_cache = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")) / "registry/cache"
        if existing_cache.is_dir():
            locked = {}
            for root in roots:
                for name in tracked_files(root):
                    if name.name != "Cargo.lock":
                        continue
                    try:
                        lock = tomllib.loads(tracked_blob(root, name).decode())
                    except tomllib.TOMLDecodeError:
                        continue  # A test fixture is not a dependency lockfile.
                    for package in lock.get("package", []):
                        if package.get("source", "").startswith("registry+"):
                            archive_name = f"{package['name']}-{package['version']}.crate"
                            locked.setdefault(archive_name, set()).add(package.get("checksum"))
            for archive in existing_cache.glob("*/*.crate"):
                checksums = locked.get(archive.name, set())
                if len(checksums) != 1:
                    continue
                target = registry / "cache" / archive.relative_to(existing_cache)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(archive, target)
                if sha(target) not in checksums:
                    target.unlink()  # Fetch a verified archive, never use changed cache bytes.
        # Fetch the exact private commit from the clean materialization. No
        # credential or pre-existing Cargo source checkout enters this build.
        env = {**os.environ, "CARGO_HOME": str(cargo), "CARGO_NET_GIT_FETCH_WITH_CLI": "true"}
        index = int(env.get("GIT_CONFIG_COUNT", "0"))
        env.update({"GIT_CONFIG_COUNT": str(index + 1),
                    f"GIT_CONFIG_KEY_{index}": f"url.{roots[1]}.insteadOf",
                    f"GIT_CONFIG_VALUE_{index}": TOOLKIT_REPOSITORY + ".git"})
        yield roots[0], roots[1], env


def release_metadata(manifest, env=None):
    return json.loads(subprocess.check_output([
        "cargo", "metadata", "--locked", "--format-version", "1",
        "--filter-platform", TARGET, "--no-default-features", "--features", "live",
        "--manifest-path", str(manifest)], cwd=Path(manifest).parent, env=env))


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


def toolkit_dependency(source, head):
    if not isinstance(source, str) or not source.startswith("git+"):
        return False
    url = urllib.parse.urlsplit(source[4:])
    return (urllib.parse.urlunsplit((url.scheme, url.netloc,
                                    url.path.removesuffix(".git"), "", "")) == TOOLKIT_REPOSITORY
            and urllib.parse.parse_qs(url.query) == {"rev": [head]}
            and url.fragment == head)


def describe(bridge, frontend, manifest_name, public_key, key_class):
    with recorded_sources(bridge, frontend) as (public, private, env):
        return describe_sources(public, private, manifest_name, public_key, key_class, env)


def describe_sources(bridge, frontend, manifest_name, public_key, key_class, env=None):
    bridge, frontend = Path(bridge).resolve(), Path(frontend).resolve()
    public, private = source_identity(bridge), source_identity(frontend)
    relative = Path(manifest_name)
    if relative != WINDOW_MANIFEST:
        raise ValueError("application is crates/window; private application manifests are unsupported")
    manifest = bridge / relative
    if manifest.resolve() != manifest or not manifest.is_file():
        raise ValueError("window manifest custody")
    tracked_blob(bridge, WINDOW_MANIFEST)
    tracked_blob(bridge, WINDOW_MANIFEST.with_name("Cargo.lock"))
    declared = tomllib.loads(manifest.read_text())
    for name in TOOLKIT_CRATES:
        dependency = declared.get("dependencies", {}).get(name, {})
        if (dependency.get("git") != TOOLKIT_REPOSITORY + ".git"
                or dependency.get("rev") != private["head"]):
            raise ValueError("window must pin both private toolkit crates to the recorded revision")
    if not re.fullmatch(r"[0-9a-f]{64}", public_key) or key_class != "internal_test":
        raise ValueError("qualification frontend requires an internal public trust root")
    metadata = release_metadata(manifest, env=env)
    if Path(metadata["workspace_root"]).resolve() != manifest.parent:
        raise ValueError("qualification frontend requires its own locked workspace")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    root_id = metadata["resolve"]["root"]
    if set(nodes[root_id]["features"]) != {"live"}:
        raise ValueError("qualification frontend must contain only the live feature")
    packages = release_packages(metadata)
    found, toolkit, checked = set(), set(), set()
    for package in packages:
        source = package.get("source")
        if source and source.startswith("registry+"):
            continue
        package_root = Path(package["manifest_path"]).resolve().parent
        if source:
            if not toolkit_dependency(source, private["head"]):
                raise ValueError("window dependency is not the exact private toolkit revision")
            checkout = Path(git(package_root, "rev-parse", "--show-toplevel")).resolve()
            if checkout not in checked:
                if env is not None and not checkout.is_relative_to(Path(env["CARGO_HOME"]).resolve() / "git/checkouts"):
                    raise ValueError("resolved toolkit source escapes fresh Cargo checkouts")
                if git_identity(checkout) != private or git(checkout, "diff", "HEAD", "--name-only"):
                    raise ValueError("resolved toolkit source tree differs from recorded source")
                checked.add(checkout)
            tracked_blob(checkout, (package_root / "Cargo.toml").relative_to(checkout))
            if package["name"] in TOOLKIT_CRATES:
                toolkit.add(package["name"])
        elif not package_root.is_relative_to(bridge):
            raise ValueError("window path dependency escapes exact public source")
        else:
            tracked_blob(bridge, (package_root / "Cargo.toml").relative_to(bridge))
            if package["name"] in BRIDGE_CRATES:
                found.add(package["name"])
        if package["name"] == "lvb-view-model" and "samples" in nodes[package["id"]]["features"]:
            raise ValueError("sample fixtures cannot enter the qualification frontend")
    if found != BRIDGE_CRATES:
        raise ValueError("window must use local public model, view model and client")
    if toolkit != TOOLKIT_CRATES:
        raise ValueError("window must use both exact private toolkit crates")
    root = next(package for package in packages if package["id"] == root_id)
    if not any(target["name"] == BINARY and "bin" in target["kind"] for target in root["targets"]):
        raise ValueError("frontend must declare the standard product executable")
    # The same build declaration now names the actual public application.
    # Its locked Git dependencies bind the private toolkit commit transitively;
    # the loop above verifies that commit's tree against the supplied checkout.
    application = {**public, "manifest": relative.as_posix(), "manifest_sha256": sha(manifest),
                   "cargo_lock_sha256": sha(manifest.with_name("Cargo.lock"))}
    return {"schema": 1, "bridge_source": public, "frontend_source": application,
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
    output, target_dir = Path(output).resolve(), Path(target_dir).resolve()
    if output.exists():
        raise ValueError("frontend build output already exists")
    if any(path.is_relative_to(Path(root).resolve())
           for path in (output, target_dir) for root in (bridge, frontend)):
        raise ValueError("frontend outputs must be outside source checkouts")
    with recorded_sources(bridge, frontend) as (public, private, cargo_env):
        metadata = describe_sources(public, private, manifest, public_key, key_class, cargo_env)
        env = {**cargo_env, "CARGO_BUILD_JOBS": "2", "CARGO_TARGET_DIR": str(target_dir),
           "LVB_PORTABLE_ED25519_PUBLIC_KEY": public_key, "LVB_PORTABLE_KEY_CLASS": key_class,
           "MAKEPAD_PACKAGE_DIR": ".",
           "LVB_FRONTEND_PACKAGE_INFO": json.dumps(metadata, sort_keys=True, separators=(",", ":"))}
        subprocess.run(["cargo", "build", "--release", "--locked", "--target", TARGET,
                    "--no-default-features", "--features", "live", "--bin", BINARY,
                    "--manifest-path", str(public / WINDOW_MANIFEST)], cwd=public, env=env, check=True)
        if (source_identity(public) != metadata["bridge_source"]
                or source_identity(private) != source_identity(frontend)):
            raise ValueError("frontend build changed recorded sources")
    binary = target_dir / TARGET / "release" / BINARY
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
    parser.add_argument("--frontend-source", type=Path, required=True,
                        help="Exact clean private Makepad toolkit checkout")
    parser.add_argument("--frontend-manifest", default=WINDOW_MANIFEST.as_posix(),
                        help="Bridge application manifest (crates/window/Cargo.toml)")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--target-dir", type=Path, required=True)
    parser.add_argument("--public-key", required=True)
    args = parser.parse_args()
    print(json.dumps(build(args.bridge_source, args.frontend_source, args.frontend_manifest,
                           args.output, args.target_dir, args.public_key, "internal_test"), sort_keys=True))


if __name__ == "__main__":
    main()

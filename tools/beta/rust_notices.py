"""Capture license texts and exact Cargo package identities on the build host.

The release dependency closure excludes dev-only dependencies. This inventory
does not authorize distribution or invent a license conclusion.
"""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
import urllib.parse
import urllib.request


def license_name(name):
    return re.match(r"^(LICENSE|LICENCE|NOTICE|COPYING|COPYRIGHT)([._-]|$)",
                    name, re.IGNORECASE) is not None


def upstream_root_licenses(package, root, cache):
    """Workspace crates sometimes omit root licenses from their crate archive.

    Retain the upstream revision declared by that exact archive. No branch,
    tag or latest version is substituted for it.
    """
    vcs = json.loads((root / ".cargo_vcs_info.json").read_text())
    revision = vcs["git"]["sha1"]
    repository = urllib.parse.urlparse(package.get("repository") or "")
    parts = repository.path.strip("/").split("/")
    if (repository.scheme != "https" or repository.hostname != "github.com"
            or len(parts) < 2 or not re.fullmatch(r"[0-9a-f]{40}", revision)
            or any(not re.fullmatch(r"[A-Za-z0-9_.-]+", part) for part in parts[:2])):
        raise ValueError("Missing license requires an exact supported upstream revision")
    repo = "/".join(parts[:2]).removesuffix(".git")
    key = (repo, revision)
    if key not in cache:
        def get(url):
            request = urllib.request.Request(url, headers={"User-Agent": "LVB-build-license-inventory"})
            with urllib.request.urlopen(request, timeout=15) as response:
                data = response.read(2_000_001)
            if len(data) > 2_000_000:
                raise ValueError("Upstream license extent")
            return data
        tree = json.loads(get(f"https://api.github.com/repos/{repo}/git/trees/{revision}"))
        selected = [item for item in tree["tree"] if item["type"] == "blob"
                    and license_name(item["path"])]
        if not selected or len(selected) > 32:
            raise ValueError("Upstream root license inventory incomplete")
        cache[key] = [(item["path"], get(
            f"https://raw.githubusercontent.com/{repo}/{revision}/" + urllib.parse.quote(item["path"])),
            f"https://github.com/{repo}/blob/{revision}/" + urllib.parse.quote(item["path"]))
            for item in selected]
    return cache[key]


def capture(source, output, target="x86_64-unknown-linux-gnu"):
    packages, locked = {}, {}
    for manifest in ("manager-ui/Cargo.toml", "native-vst3-proxy/backend/Cargo.toml"):
        lock = tomllib.loads((source / manifest).with_name("Cargo.lock").read_text())
        for item in lock["package"]:
            if item.get("source", "").startswith("registry+"):
                key = (item["name"], item["version"])
                if key in locked and locked[key] != item["checksum"]:
                    raise ValueError("Conflicting locked dependency archives")
                locked[key] = item["checksum"]
        metadata = json.loads(subprocess.check_output([
            "cargo", "metadata", "--locked", "--format-version", "1",
            "--filter-platform", target, "--manifest-path", str(source / manifest)]))
        nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
        todo = [metadata["resolve"]["root"]]
        seen = set()
        while todo:
            identity = todo.pop()
            if identity in seen:
                continue
            seen.add(identity)
            todo.extend(dep["pkg"] for dep in nodes[identity]["deps"]
                        if any(kind["kind"] != "dev" for kind in dep["dep_kinds"]))
        for package in metadata["packages"]:
            if package["id"] in seen and package.get("source"):
                if not package["source"].startswith("registry+"):
                    raise ValueError("Uninventoried non-registry Rust dependency")
                packages[package["id"]] = package
    inventory, texts, upstream = [], [], {}
    for package in sorted(packages.values(), key=lambda p: (p["name"], p["version"])):
        root = Path(package["manifest_path"]).parent
        files = sorted(path for path in root.rglob("*") if path.is_file()
                       and (license_name(path.name)
                            or ("fonts" in path.relative_to(root).parts and path.suffix == ".txt")))
        if package.get("license_file"):
            declared = root / package["license_file"]
            if declared.resolve().is_relative_to(root.resolve()) and declared.is_file():
                files = sorted(set(files) | {declared})
        if not package.get("license"):
            raise ValueError(f"Missing declared license/text: {package['name']} {package['version']}")
        missing_root = not any(path.parent == root and license_name(path.name) for path in files)
        additional = upstream_root_licenses(package, root, upstream) if missing_root else []
        checksum = locked[(package["name"], package["version"])]
        if not isinstance(checksum, str) or not re.fullmatch(r"[0-9a-f]{64}", checksum):
            raise ValueError("Registry package checksum absent")
        archive = root.parents[2] / "cache" / root.parent.name / (root.name + ".crate")
        if hashlib.sha256(archive.read_bytes()).hexdigest() != checksum:
            raise ValueError("Registry archive differs from the locked dependency")
        captured = []
        texts.append(f"\n=== {package['name']} {package['version']} ({package['license']}) ===\n")
        for path in files:
            if path.is_symlink() or path.stat().st_size > 2_000_000:
                raise ValueError("License text custody/extent")
            data = path.read_bytes()
            text = data.decode("utf-8")
            name = path.relative_to(root).as_posix()
            captured.append({"file": name, "sha256": hashlib.sha256(data).hexdigest(),
                             "source": "registry_archive"})
            texts.append(f"--- {name} ---\n{text}\n")
        for name, data, url in additional:
            captured.append({"file": name, "sha256": hashlib.sha256(data).hexdigest(),
                             "source": "upstream_revision", "url": url})
            texts.append(f"--- {url} ---\n{data.decode('utf-8')}\n")
        inventory.append({"name": package["name"], "version": package["version"],
                          "license_declared": package["license"],
                          "registry_archive_sha256": checksum, "license_texts": captured})
    (output / "Rust-Dependency-Notices.txt").write_text(
        "Resolved Linux release dependencies; upstream declarations and texts.\n" + "".join(texts))
    (output / "Rust-Dependency-Inventory.json").write_text(json.dumps(
        {"schema": 1, "target": target, "dev_dependencies_included": False,
         "packages": inventory}, sort_keys=True, indent=2) + "\n")
    return inventory

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
import frontend_build as qualification


def license_name(name):
    return re.match(r"^(LICENSE|LICENCE|NOTICE|COPYING|COPYRIGHT|OFL|UNLICENSE)([._-]|$)",
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


def capture(source, output, target="x86_64-unknown-linux-gnu", *,
            frontend_source=None, frontend_build=None):
    source, output = Path(source).resolve(), Path(output)
    packages, locked, origins = {}, {}, {}
    manifests = [source / "manager-ui/Cargo.toml", source / "native-vst3-proxy/backend/Cargo.toml"]
    frontend_manifest = None
    if frontend_build is not None:
        frontend_source = Path(frontend_source).resolve()
        identity = frontend_build["frontend_source"]
        if (qualification.source_identity(frontend_source) != {k: identity[k] for k in ("head", "tree")}
                or qualification.source_identity(source) != frontend_build["bridge_source"]):
            raise ValueError("notice sources differ from frontend build")
        frontend_manifest = frontend_source / identity["manifest"]
        manifests = [source / "bridge-manager/Cargo.toml", manifests[1], frontend_manifest]
    for manifest in manifests:
        lock = tomllib.loads(manifest.with_name("Cargo.lock").read_text())
        for item in lock["package"]:
            if item.get("source", "").startswith("registry+"):
                key = (item["name"], item["version"])
                if key in locked and locked[key] != item["checksum"]:
                    raise ValueError("Conflicting locked dependency archives")
                locked[key] = item["checksum"]
        metadata = (qualification.release_metadata(manifest) if manifest == frontend_manifest else
                    json.loads(subprocess.check_output([
                        "cargo", "metadata", "--locked", "--format-version", "1",
                        "--filter-platform", target, "--manifest-path", str(manifest)])))
        for package in qualification.release_packages(metadata):
            origin = package.get("source")
            root = Path(package["manifest_path"]).resolve().parent
            if origin and origin.startswith("registry+"):
                packages[package["id"]] = package
            elif (frontend_build is not None and origin
                  and qualification.bridge_dependency(origin, frontend_build["bridge_source"]["head"])):
                continue  # First-party Bridge crates; the package already includes COPYRIGHT.md.
            elif origin:
                raise ValueError("Uninventoried non-registry Rust dependency")
            elif frontend_build is not None and root.is_relative_to(frontend_source):
                packages[package["id"]] = package
                origins[package["id"]] = {"kind": "git", "head": identity["head"],
                    "tree": identity["tree"], "path": root.relative_to(frontend_source).as_posix()}
            elif not root.is_relative_to(source):
                raise ValueError("Uninventoried path Rust dependency")
    inventory, texts, upstream = [], [], {}
    for package in sorted(packages.values(), key=lambda p: (p["name"], p["version"])):
        root = Path(package["manifest_path"]).resolve().parent
        origin = origins.get(package["id"])
        files = sorted(path for path in root.rglob("*") if path.is_file()
                       and (license_name(path.name)
                            or ("fonts" in path.relative_to(root).parts and path.suffix == ".txt")))
        if package.get("license_file"):
            declared = root / package["license_file"]
            boundary = frontend_source if origin else root.resolve()
            if declared.resolve().is_relative_to(boundary) and declared.is_file():
                files = sorted(set(files) | {declared.resolve()})
        if not package.get("license"):
            raise ValueError(f"Missing declared license/text: {package['name']} {package['version']}")
        missing_root = not any(path.parent == root and license_name(path.name) for path in files)
        if origin:
            # Makepad's own crates share its root MIT license. Vendored crates
            # must bring their own texts; a foreign crate cannot inherit it.
            if missing_root and package["name"].startswith("makepad-"):
                files += sorted(path for path in frontend_source.iterdir()
                                if path.is_file() and license_name(path.name))
            if not any(license_name(path.name) for path in files):
                raise ValueError(f"Missing exact-source license text: {package['name']}")
            additional, checksum = [], None
        else:
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
            name = path.relative_to(frontend_source if origin else root).as_posix()
            captured.append({"file": name, "sha256": hashlib.sha256(data).hexdigest(),
                             "source": "frontend_source" if origin else "registry_archive"})
            texts.append(f"--- {name} ---\n{text}\n")
        for name, data, url in additional:
            captured.append({"file": name, "sha256": hashlib.sha256(data).hexdigest(),
                             "source": "upstream_revision", "url": url})
            texts.append(f"--- {url} ---\n{data.decode('utf-8')}\n")
        item = {"name": package["name"], "version": package["version"],
                "license_declared": package["license"], "license_texts": captured}
        item.update({"source": origin} if origin else {"registry_archive_sha256": checksum})
        inventory.append(item)
    (output / "Rust-Dependency-Notices.txt").write_text(
        "Resolved Linux release dependencies; upstream declarations and texts.\n" + "".join(texts))
    (output / "Rust-Dependency-Inventory.json").write_text(json.dumps(
        {"schema": 2 if frontend_build else 1, "target": target, "dev_dependencies_included": False,
         "packages": inventory}, sort_keys=True, indent=2) + "\n")
    return inventory

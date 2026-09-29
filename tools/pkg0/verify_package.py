#!/usr/bin/env python3
"""Compare an Arch binary package with PKG0's exact signed file roster."""
import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import subprocess
import tempfile
from assemble import (ADOPTED, ADOPTED_WITH_KIT, ADOPTION_MANIFEST, DEPENDENCIES,
                      KIT_DESTINATION, PKGREL, verify_kit, verify_kit_backend,
                      verify_kit_source, validate_release_roster)

OPTIONAL_META = {".BUILDINFO": 1024 * 1024, ".MTREE": 4 * 1024 * 1024}


def package_info(path, manifest):
    data = path.read_bytes()
    fields = {}
    for line in data.decode("utf-8").splitlines():
        if not line or line.startswith("#"):
            continue
        if " = " not in line:
            raise ValueError("package metadata syntax")
        key, value = line.split(" = ", 1)
        if not key or not value:
            raise ValueError("package metadata syntax")
        fields.setdefault(key, []).append(value)
    for key, expected in (("pkgname", "linux-vst-bridge-beta"),
                          ("pkgver", f"{manifest['version']}-{manifest['pkgrel']}"),
                          ("arch", "x86_64")):
        if fields.get(key) != [expected]:
            raise ValueError(f"package {key} differs")
    if set(fields.get("depend", [])) != set(DEPENDENCIES) or len(fields.get("depend", [])) != len(DEPENDENCIES):
        raise ValueError("package dependencies differ")
    if "install" in fields or "installfile" in fields:
        raise ValueError("package install hook forbidden")


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def verify_adoption(adopted, manifest, expected):
    names = ADOPTED_WITH_KIT if KIT_DESTINATION in expected else ADOPTED
    external = adopted.get("external_runtime")
    if (set(adopted) != {"schema", "package", "version", "pkgrel", "source_head",
                        "source_tree", "operator_schema", "files", "external_runtime"}
            or adopted["schema"] != (2 if KIT_DESTINATION in expected else 1)
            or adopted["package"] != manifest["package"]
            or adopted["version"] != manifest["version"]
            or adopted["pkgrel"] != manifest["pkgrel"]
            or adopted["source_head"] != manifest["source_head"]
            or adopted["source_tree"] != manifest["source_tree"]
            or adopted["operator_schema"] != 12
            or not isinstance(external, dict)
            or set(external) != {"id", "manifest_sha256"}
            or not isinstance(external["id"], str)
            or re.fullmatch(r"[A-Za-z0-9_-]{1,128}", external["id"]) is None
            or not isinstance(external["manifest_sha256"], str)
            or re.fullmatch(r"[0-9a-f]{64}", external["manifest_sha256"]) is None):
        raise ValueError("package adoption identity differs")
    roster = [{"name": adopted_name, "sha256": expected[path]["sha256"],
               "size": expected[path]["size"]} for path, adopted_name in names.items()]
    if adopted["files"] != roster:
        raise ValueError("package adoption artifact roster differs")


def verify(package, manifest, structure_only=False, source_root=None, rebuild_backend=False):
    if rebuild_backend and source_root is None:
        raise ValueError("release backend rebuild requires exact source")
    if (manifest.get("schema") != 1 or manifest.get("package") != "linux-vst-bridge-beta"
            or manifest.get("pkgrel") != PKGREL
            or not isinstance(manifest.get("version"), str)
            or re.fullmatch(r"[0-9][A-Za-z0-9.]*", manifest["version"]) is None
            or not isinstance(manifest.get("files"), list)
            or not isinstance(manifest.get("source_head"), str)
            or not isinstance(manifest.get("source_tree"), str)):
        raise ValueError("release manifest schema")
    validate_release_roster(manifest)
    expected = {row["destination"]: row for row in manifest["files"]}
    if len(expected) != len(manifest["files"]):
        raise ValueError("duplicate release roster")
    listing = subprocess.check_output(["bsdtar", "-tf", str(package)], text=True).splitlines()
    counts = Counter(listing)
    if any(n > 1 for n in counts.values()):
        raise ValueError("duplicate package archive entry")
    if counts[".PKGINFO"] != 1 or ".INSTALL" in counts:
        raise ValueError("package metadata or install hook differs")
    files = set()
    for name in listing:
        p = PurePosixPath(name)
        if name == ".PKGINFO" or name in OPTIONAL_META:
            continue
        if p.is_absolute() or ".." in p.parts or not name.startswith("usr/"):
            raise ValueError("unexpected package path")
        if not name.endswith("/"):
            files.add(name)
    if files != set(expected):
        raise ValueError("package file roster differs")
    with tempfile.TemporaryDirectory(prefix="lvb-dist0-verify-") as td:
        root = Path(td)
        subprocess.run(["bsdtar", "-xf", str(package), "-C", str(root), "--no-same-owner"], check=True)
        for name, limit in {".PKGINFO": 64 * 1024, **OPTIONAL_META}.items():
            path = root / name
            if name == ".PKGINFO" or path.exists() or path.is_symlink():
                md = path.lstat()
                if not stat.S_ISREG(md.st_mode) or md.st_size > limit:
                    raise ValueError("package metadata type or extent")
        package_info(root / ".PKGINFO", manifest)
        for name, row in expected.items():
            path = root / name
            if "target" in row:
                if not path.is_symlink() or os.readlink(path) != row["target"]:
                    raise ValueError("package symlink differs")
                continue
            md = path.lstat()
            if not stat.S_ISREG(md.st_mode) or path.is_symlink():
                raise ValueError("package file type differs")
            if digest(path) != row["sha256"] or md.st_size != row["size"]:
                raise ValueError("package bytes differ")
            if stat.S_IMODE(md.st_mode) != int(row["mode"], 8):
                raise ValueError("package mode differs")
            if not structure_only and row["kind"] in ("manager", "frontend", "proxy"):
                elf = subprocess.check_output(["readelf", "-h", str(path)], text=True)
                if "Advanced Micro Devices X86-64" not in elf or "ELF64" not in elf:
                    raise ValueError("package ELF architecture differs")
            if not structure_only and row["kind"] in ("windows_host", "fixture"):
                pe = subprocess.check_output(["file", "-b", str(path)], text=True)
                if "PE32+" not in pe or "x86-64" not in pe:
                    raise ValueError("package PE architecture differs")
        adopted = json.loads((root / ADOPTION_MANIFEST).read_bytes())
        verify_adoption(adopted, manifest, expected)
        if KIT_DESTINATION in expected:
            kit_bytes = (root / KIT_DESTINATION).read_bytes()
            verify_kit(kit_bytes, manifest["source_head"],
                       expected["usr/lib/linux-vst-bridge/host/bridge-host.exe"]["sha256"],
                       expected["usr/lib/linux-vst-bridge/host/source-manifest.json"]["sha256"])
            if source_root is not None:
                verify_kit_source(kit_bytes, source_root, manifest["source_head"],
                                  manifest["source_tree"])
                if rebuild_backend:
                    verify_kit_backend(kit_bytes, source_root)
                    verify_kit_source(kit_bytes, source_root, manifest["source_head"],
                                      manifest["source_tree"])
    return {"package_sha256": digest(package), "files": len(expected),
            "structure_only": structure_only}


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--package", type=Path, required=True)
    p.add_argument("--manifest", type=Path, required=True)
    p.add_argument("--structure-only", action="store_true")
    a = p.parse_args()
    manifest = json.loads(a.manifest.read_bytes())
    print(json.dumps(verify(a.package, manifest, a.structure_only), sort_keys=True))


if __name__ == "__main__":
    main()

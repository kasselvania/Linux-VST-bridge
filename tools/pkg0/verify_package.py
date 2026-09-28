#!/usr/bin/env python3
"""Compare an Arch binary package with PKG0's exact signed file roster."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import subprocess
import tempfile

META = {".BUILDINFO", ".INSTALL", ".MTREE", ".PKGINFO"}


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        for block in iter(lambda: f.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def verify(package, manifest, structure_only=False):
    if (manifest.get("schema") != 1 or manifest.get("package") != "linux-vst-bridge-beta"
            or not isinstance(manifest.get("files"), list)
            or not isinstance(manifest.get("source_head"), str)
            or not isinstance(manifest.get("source_tree"), str)):
        raise ValueError("release manifest schema")
    expected = {row["destination"]: row for row in manifest["files"]}
    if len(expected) != len(manifest["files"]):
        raise ValueError("duplicate release roster")
    listing = subprocess.check_output(["bsdtar", "-tf", str(package)], text=True).splitlines()
    files = set()
    for name in listing:
        p = PurePosixPath(name)
        if name in META:
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

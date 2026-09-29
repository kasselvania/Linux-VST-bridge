#!/usr/bin/env python3
"""Build and verify an amd64 Debian package from one exact PKG0 payload.

This is a package-format adapter. PKG0 owns the file roster and adoption
manifest. A .deb installation only places root-owned files under /usr; it never
selects a user software generation or runs a maintainer script.
"""

import argparse
from collections import Counter
import gzip
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
import tarfile
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "pkg0"))
from pkg0.assemble import (KIT_DESTINATION, verify_kit, verify_kit_source,
                           verify_kit_backend, validate_release_roster)  # noqa: E402
from verify_package import verify_adoption  # noqa: E402

PACKAGE = "linux-vst-bridge-beta"
ARCH = "amd64"
MAX_MANIFEST = 16 * 1024 * 1024
MAX_PAYLOAD = 2 * 1024 * 1024 * 1024
MAX_FILES = 100_002
MAX_CONTROL = 64 * 1024
DEPENDENCIES = (
    "libc6 (>= 2.39)", "libstdc++6", "python3", "systemd", "libx11-6",
    "libxcb1", "libxkbcommon0", "libgl1", "libegl1", "pipewire",
    "xdg-desktop-portal",
)


def sha_file(source):
    digest = hashlib.sha256()
    for chunk in iter(lambda: source.read(1024 * 1024), b""):
        digest.update(chunk)
    return digest.hexdigest()


def exact_file(path, limit):
    path = Path(path)
    if not path.is_absolute():
        raise ValueError("package input must be an absolute regular file")
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC)
    except OSError as error:
        raise ValueError("package input must be an absolute regular file") from error
    source = os.fdopen(descriptor, "rb")
    before = os.fstat(source.fileno())
    if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
        source.close()
        raise ValueError("package input type or extent")
    return source, before


def stable(source, before):
    after = os.fstat(source.fileno())
    identity = lambda st: (st.st_dev, st.st_ino, st.st_size, st.st_mtime_ns, st.st_ctime_ns)
    if identity(before) != identity(after):
        raise ValueError("package input changed while reading")


def read_manifest(path):
    source, before = exact_file(path, MAX_MANIFEST)
    with source:
        raw = source.read(MAX_MANIFEST + 1)
        stable(source, before)
    if len(raw) > MAX_MANIFEST:
        raise ValueError("release manifest extent")
    manifest = json.loads(raw)
    if (set(manifest) != {"schema", "package", "version", "pkgrel", "source_head",
                          "source_tree", "payload_sha256", "files"}
            or manifest["schema"] != 1 or manifest["package"] != PACKAGE
            or manifest["pkgrel"] != 1
            or not isinstance(manifest["version"], str)
            or not re.fullmatch(r"[0-9][A-Za-z0-9.]*", manifest["version"])
            or any(not isinstance(manifest[key], str)
                   or not re.fullmatch(r"[0-9a-f]{40}", manifest[key])
                   for key in ("source_head", "source_tree"))
            or not isinstance(manifest["payload_sha256"], str)
            or not re.fullmatch(r"[0-9a-f]{64}", manifest["payload_sha256"])
            or not isinstance(manifest["files"], list)
            or not 1 <= len(manifest["files"]) <= MAX_FILES):
        raise ValueError("release manifest identity")
    validate_release_roster(manifest)
    if sum(row["size"] for row in manifest["files"]) > MAX_PAYLOAD:
        raise ValueError("release payload extent")
    return manifest, raw


def verify_payload(path, manifest):
    source, before = exact_file(path, MAX_PAYLOAD)
    expected = {row["destination"]: row for row in manifest["files"]}
    seen = set()
    with source:
        with tarfile.open(fileobj=source, mode="r:") as archive:
            for member in archive:
                if member.name in seen or member.name not in expected or not member.isfile():
                    raise ValueError("payload roster or type differs")
                row = expected[member.name]
                if (member.size != row["size"] or member.mode != int(row["mode"], 8)
                        or member.uid != 0 or member.gid != 0):
                    raise ValueError("payload metadata differs")
                content = archive.extractfile(member)
                if content is None or sha_file(content) != row["sha256"]:
                    raise ValueError("payload bytes differ")
                seen.add(member.name)
        if seen != set(expected):
            raise ValueError("payload roster differs")
        source.seek(0)
        if sha_file(source) != manifest["payload_sha256"]:
            raise ValueError("payload digest differs")
        stable(source, before)


def control_bytes(manifest):
    return (f"Package: {PACKAGE}\n"
            f"Version: {manifest['version']}-{manifest['pkgrel']}\n"
            f"Architecture: {ARCH}\n"
            "Maintainer: Linux VST Bridge project\n"
            "Section: sound\n"
            "Priority: optional\n"
            f"Depends: {', '.join(DEPENDENCIES)}\n"
            "Description: Managed Windows audio compatibility for native Linux DAWs\n"
            " Exact private beta build. Proton/SLR is an external verified prerequisite.\n").encode()


def tar_control(manifest, epoch):
    control = control_bytes(manifest)
    if len(control) > MAX_CONTROL:
        raise ValueError("control metadata extent")
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", mtime=epoch, filename="") as compressed:
        with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.GNU_FORMAT) as archive:
            entry = tarfile.TarInfo("./control")
            entry.size = len(control)
            entry.mode = 0o644
            entry.uid = entry.gid = 0
            entry.mtime = epoch
            archive.addfile(entry, io.BytesIO(control))
    return output.getvalue()


def parent_directories(names):
    parents = set()
    for name in names:
        parents.update(str(path) for path in PurePosixPath(name).parents if str(path) != ".")
    return sorted(parents, key=lambda name: (name.count("/"), name))


def ar_member(output, name, source, size, epoch):
    encoded = (f"{name + '/':<16}{epoch:<12}{0:<6}{0:<6}{0o100644:<8}{size:<10}`\n").encode("ascii")
    if len(encoded) != 60:
        raise ValueError("ar member header extent")
    output.write(encoded)
    remaining = size
    while remaining:
        chunk = source.read(min(1024 * 1024, remaining))
        if not chunk:
            raise ValueError("ar member source truncated")
        output.write(chunk)
        remaining -= len(chunk)
    if source.read(1):
        raise ValueError("ar member source grew")
    if size % 2:
        output.write(b"\n")


def build(staged, output, epoch):
    staged = Path(staged).resolve(strict=True)
    output = Path(output).absolute()
    manifest, raw = read_manifest(staged / "RELEASE_MANIFEST.json")
    verify_payload(staged / "payload.tar", manifest)
    expected = f"{PACKAGE}_{manifest['version']}-{manifest['pkgrel']}_{ARCH}.deb"
    if output.name != expected or output.exists() or output.is_symlink():
        raise ValueError("Debian package output identity")
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="lvb-deb-stage-", dir=output.parent) as tmp:
        data = Path(tmp) / "data.tar.gz"
        rows = {row["destination"]: row for row in manifest["files"]}
        source, before = exact_file(staged / "payload.tar", MAX_PAYLOAD)
        with source, data.open("wb") as target:
            with gzip.GzipFile(fileobj=target, mode="wb", mtime=epoch, filename="") as compressed:
                with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.PAX_FORMAT) as package:
                    for name in parent_directories(row["destination"] for row in manifest["files"]):
                        directory = tarfile.TarInfo(name + "/")
                        directory.type = tarfile.DIRTYPE
                        directory.mode = 0o755
                        directory.uid = directory.gid = 0
                        directory.mtime = epoch
                        package.addfile(directory)
                    with tarfile.open(fileobj=source, mode="r:") as payload:
                        for member in payload:
                            row = rows[member.name]
                            item = tarfile.TarInfo(member.name)
                            item.size = row["size"]
                            item.mode = int(row["mode"], 8)
                            item.uid = item.gid = 0
                            item.mtime = epoch
                            package.addfile(item, payload.extractfile(member))
            stable(source, before)
        control = tar_control(manifest, epoch)
        temporary = Path(tmp) / output.name
        with temporary.open("wb") as package:
            package.write(b"!<arch>\n")
            ar_member(package, "debian-binary", io.BytesIO(b"2.0\n"), 4, epoch)
            ar_member(package, "control.tar.gz", io.BytesIO(control), len(control), epoch)
            with data.open("rb") as source:
                ar_member(package, "data.tar.gz", source, data.stat().st_size, epoch)
        verify(temporary, staged)
        temporary.replace(output)
    with output.open("rb") as source:
        package_digest = sha_file(source)
    return {"schema": 1, "format": "deb", "package": PACKAGE,
            "version": manifest["version"], "pkgrel": manifest["pkgrel"],
            "architecture": ARCH, "source_head": manifest["source_head"],
            "source_tree": manifest["source_tree"],
            "canonical_release_sha256": hashlib.sha256(raw).hexdigest(),
            "package_sha256": package_digest}


def ar_entries(package, destination):
    destination = Path(destination)
    source, before = exact_file(package, MAX_PAYLOAD)
    with source as archive:
        if archive.read(8) != b"!<arch>\n":
            raise ValueError("Debian archive header")
        entries = {}
        while True:
            header = archive.read(60)
            if not header:
                break
            if len(header) != 60 or header[58:] != b"`\n":
                raise ValueError("Debian archive member header")
            name = header[:16].decode("ascii").strip().removesuffix("/")
            size = int(header[48:58].decode("ascii").strip())
            if name in entries or name not in ("debian-binary", "control.tar.gz", "data.tar.gz") or size > MAX_PAYLOAD:
                raise ValueError("Debian archive member identity")
            target = destination / name
            with target.open("xb") as member:
                remaining = size
                while remaining:
                    chunk = archive.read(min(1024 * 1024, remaining))
                    if not chunk:
                        raise ValueError("Debian archive member truncated")
                    member.write(chunk)
                    remaining -= len(chunk)
            entries[name] = target
            if size % 2 and archive.read(1) != b"\n":
                raise ValueError("Debian archive member alignment")
        if list(entries) != ["debian-binary", "control.tar.gz", "data.tar.gz"]:
            raise ValueError("Debian archive roster")
        stable(archive, before)
        return entries


def verify(package, staged, source_root=None, rebuild_backend=False):
    if rebuild_backend and source_root is None:
        raise ValueError("Debian release backend rebuild requires exact source")
    package = Path(package).absolute()
    manifest, _ = read_manifest(Path(staged).resolve(strict=True) / "RELEASE_MANIFEST.json")
    expected_name = f"{PACKAGE}_{manifest['version']}-{manifest['pkgrel']}_{ARCH}.deb"
    if package.name != expected_name:
        raise ValueError("Debian package filename")
    with tempfile.TemporaryDirectory(prefix="lvb-deb-verify-") as tmp:
        entries = ar_entries(package, tmp)
        if entries["debian-binary"].read_bytes() != b"2.0\n":
            raise ValueError("Debian format version")
        with tarfile.open(entries["control.tar.gz"], mode="r:gz") as archive:
            members = archive.getmembers()
            if (len(members) != 1 or members[0].name != "./control"
                    or not members[0].isfile() or members[0].size > MAX_CONTROL):
                raise ValueError("Debian control roster")
            data = archive.extractfile(members[0]).read()
            if data != control_bytes(manifest):
                raise ValueError("Debian control metadata differs")
        expected = {row["destination"]: row for row in manifest["files"]}
        seen = Counter()
        directories = Counter()
        exact_directories = set(parent_directories(expected))
        adoption = None
        kit = None
        with tarfile.open(entries["data.tar.gz"], mode="r:gz") as archive:
            for member in archive:
                if member.isdir():
                    name = member.name.rstrip("/")
                    directories[name] += 1
                    if (name not in exact_directories or directories[name] != 1
                            or member.mode != 0o755 or member.uid != 0
                            or member.gid != 0 or member.size != 0):
                        raise ValueError("Debian directory roster")
                    continue
                seen[member.name] += 1
                if member.name not in expected or seen[member.name] != 1 or not member.isfile():
                    raise ValueError("Debian payload roster")
                row = expected[member.name]
                if (member.size != row["size"] or member.mode != int(row["mode"], 8)
                        or member.uid != 0 or member.gid != 0):
                    raise ValueError("Debian payload metadata differs")
                content = archive.extractfile(member)
                if content is None:
                    raise ValueError("Debian payload differs")
                if member.name in ("usr/share/linux-vst-bridge/pkg0-manifest.json", KIT_DESTINATION):
                    data = content.read()
                    member_sha = hashlib.sha256(data).hexdigest()
                    if member.name == KIT_DESTINATION:
                        kit = data
                    else:
                        adoption = json.loads(data)
                else:
                    member_sha = sha_file(content)
                if member_sha != row["sha256"]:
                    raise ValueError("Debian payload differs")
        if set(seen) != set(expected) or set(directories) != exact_directories:
            raise ValueError("Debian payload incomplete")
        if not isinstance(adoption, dict):
            raise ValueError("Debian adoption authority absent")
        verify_adoption(adoption, manifest, expected)
        if kit is not None:
            verify_kit(kit, manifest["source_head"],
                       expected["usr/lib/linux-vst-bridge/host/bridge-host.exe"]["sha256"],
                       expected["usr/lib/linux-vst-bridge/host/source-manifest.json"]["sha256"])
            if source_root is not None:
                verify_kit_source(kit, source_root, manifest["source_head"], manifest["source_tree"])
                if rebuild_backend:
                    verify_kit_backend(kit, source_root)
        elif source_root is not None:
            raise ValueError("Debian release preparation kit absent")
    with package.open("rb") as source:
        package_digest = sha_file(source)
    return {"files": len(expected), "package_sha256": package_digest}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("build", "verify"))
    parser.add_argument("--staged", type=Path, required=True)
    parser.add_argument("--package", type=Path, required=True)
    parser.add_argument("--epoch", type=int)
    args = parser.parse_args()
    if args.command == "build":
        if args.epoch is None or not 0 <= args.epoch <= 4_102_444_800:
            raise ValueError("bounded source epoch required")
        print(json.dumps(build(args.staged, args.package, args.epoch), sort_keys=True))
    else:
        print(json.dumps(verify(args.package, args.staged), sort_keys=True))


if __name__ == "__main__":
    main()

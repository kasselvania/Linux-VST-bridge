#!/usr/bin/env python3
"""Bounded read-only Deck authority fingerprint; prints no paths or record data."""
import hashlib
import json
import os
import stat
from pathlib import Path


HOME = Path.home()
ROOT = HOME / ".local/share/linux-vst-bridge/managed"
OWNERS = (
    "software", "publications", "installers", "onboarding", "inventory",
    "preparation", "operator", "daw-workspaces", "runtime",
    "vendor-applications", "transactions", "package-transitions",
    "private-rollback", "runners", "profiles", "environments",
)
ROUTES = (
    HOME / ".local/bin/linux-vst-bridge",
    HOME / ".local/bin/linux-audio-compatibility-manager",
    HOME / ".local/share/applications/linux-audio-compatibility-manager.desktop",
    HOME / ".local/share/applications/linux-vst-bridge-native-access.desktop",
    HOME / ".config/mimeapps.list",
    HOME / ".config/systemd/user/linux-vst-bridge.service",
)
MAX_ENTRIES = 10000
MAX_BYTES = 8 * 1024 * 1024


def digest(data):
    return hashlib.sha256(data).hexdigest()


def one_file(path):
    before = path.lstat()
    if stat.S_ISLNK(before.st_mode):
        return "link:" + digest(os.readlink(path).encode())
    if not stat.S_ISREG(before.st_mode):
        raise RuntimeError("metadata_type")
    if before.st_size > MAX_BYTES:
        raise RuntimeError("metadata_extent")
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode) or (before.st_dev, before.st_ino) != (
                opened.st_dev, opened.st_ino):
            raise RuntimeError("metadata_identity_changed")
        with os.fdopen(descriptor, "rb", closefd=False) as stream:
            data = stream.read(MAX_BYTES + 1)
        closed = os.fstat(descriptor)
    finally:
        os.close(descriptor)
    after = path.lstat()
    if len(data) > MAX_BYTES or (before.st_ino, before.st_size, before.st_mtime_ns,
            before.st_ctime_ns) != (closed.st_ino, closed.st_size,
            closed.st_mtime_ns, closed.st_ctime_ns) or (
            before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (
            after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
        raise RuntimeError("metadata_changed_during_inventory")
    return digest(data)


def visit(folder, category, rows):
    if not folder.exists():
        return
    for base, directories, files in os.walk(folder, followlinks=False):
        relative = Path(base).relative_to(folder)
        directories[:] = sorted(name for name in directories
            if name not in {"compatdata", "drive_c", "vendor-content", "content"})
        for name in sorted(files):
            if Path(name).suffix not in {".json", ".toml"}:
                continue
            if category == "environments" and name != "environment.json":
                continue
            path = Path(base) / name
            try:
                identity = one_file(path)
            except RuntimeError as error:
                raise RuntimeError(f"{category}:{error}") from error
            rows.append((f"{category}/{relative}/{name}", identity))
            if len(rows) > MAX_ENTRIES:
                raise RuntimeError("metadata_count_bound")


def main():
    rows = []
    for name in ("software.json", "registry.json"):
        path = ROOT / name
        if path.exists():
            rows.append((name, one_file(path)))
    counts = {}
    for category in OWNERS:
        start = len(rows)
        visit(ROOT / category, category, rows)
        counts[category] = len(rows) - start
    start = len(rows)
    environments = ROOT / "environments"
    if environments.is_dir():
        for environment in sorted(environments.iterdir()):
            if not environment.is_dir() or environment.is_symlink():
                raise RuntimeError("environment_directory_type")
            bridge = environment / "compatdata/pfx/drive_c/bridge"
            visit(bridge, f"environment-bridge/{environment.name}", rows)
    counts["environment-bridge"] = len(rows) - start
    routes = []
    for index, path in enumerate(ROUTES):
        try:
            mode = path.lstat().st_mode
        except FileNotFoundError:
            identity = "absent"
        else:
            if stat.S_ISLNK(mode):
                identity = "symlink:" + digest(os.readlink(path).encode())
            elif stat.S_ISREG(mode):
                identity = "file:" + one_file(path)
            else:
                raise RuntimeError("route_type")
        routes.append((index, identity))
    encoded = lambda value: json.dumps(value, sort_keys=True,
        separators=(",", ":")).encode()
    print(json.dumps({"schema": 1, "metadata_count": len(rows),
        "metadata_sha256": digest(encoded(sorted(rows))),
        "category_counts": counts, "route_count": len(routes),
        "routes_sha256": digest(encoded(routes))}, sort_keys=True))


if __name__ == "__main__":
    main()

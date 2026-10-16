#!/usr/bin/env python3
"""Immutable runner entry: verify a declared fixed browser, then run Proton.

Copied into the sealed runner with the component-manifest digest substituted.
No downloads, registry edits, user-data relocation, flags or licensing actions.
All verification runs before Wine starts, outside the audio callback.
"""
import hashlib
import json
import os
from pathlib import Path
import stat
import sys

COMPONENTS_SHA256 = "@COMPONENTS_SHA256@"


def digest(path):
    metadata = path.lstat()
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_uid != os.getuid():
        raise RuntimeError("runtime component ownership/type changed")
    with os.fdopen(os.open(path, os.O_RDONLY | os.O_NOFOLLOW), "rb") as source:
        opened = os.fstat(source.fileno())
        if (opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino):
            raise RuntimeError("runtime component replaced")
        return hashlib.file_digest(source, "sha256").hexdigest()


def prepare(root, expected, inherited):
    manifest = root / "runtime-components.json"
    if digest(manifest) != expected:
        raise RuntimeError("runtime component manifest changed")
    component = json.loads(manifest.read_bytes())
    if (component["schema"] != 1 or component["kind"] != "fixed_webview2"
            or not 1 <= len(component["files"]) <= 512):
        raise RuntimeError("unsupported runtime component manifest")
    original = root / "proton-original"
    if digest(original) != component["proton_sha256"]:
        raise RuntimeError("original Proton changed")
    browser = root / "files/share/webview2"
    if browser.is_symlink() or not browser.is_dir():
        raise RuntimeError("runtime component directory changed")
    actual = set()
    for directory, folders, files in os.walk(browser, followlinks=False):
        for name in folders + files:
            path = Path(directory) / name
            if path.is_symlink():
                raise RuntimeError("runtime component link refused")
        actual.update(str((Path(directory) / name).relative_to(browser)) for name in files)
        if len(actual) > 512:
            raise RuntimeError("runtime component file bound")
    expected_files = {item["path"] for item in component["files"]}
    if len(expected_files) != len(component["files"]) or actual != expected_files:
        raise RuntimeError("runtime component roster changed")
    for item in component["files"]:
        relative = Path(item["path"])
        if relative.is_absolute() or ".." in relative.parts:
            raise RuntimeError("runtime component path refused")
        path = browser / relative
        if path.stat().st_size != item["bytes"] or digest(path) != item["sha256"]:
            raise RuntimeError("runtime component changed")
    env = inherited.copy()
    env["WEBVIEW2_BROWSER_EXECUTABLE_FOLDER"] = "Z:" + str(browser).replace("/", "\\")
    return original, env


if __name__ == "__main__":
    try:
        executable, environment = prepare(Path(__file__).resolve().parent,
                                          COMPONENTS_SHA256, os.environ)
    except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
        print("LVB fixed browser runtime unavailable: " + str(error), file=sys.stderr)
        sys.exit(1)
    os.execve(sys.executable, [sys.executable, str(executable), *sys.argv[1:]], environment)

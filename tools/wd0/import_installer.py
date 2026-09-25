#!/usr/bin/env python3
"""Pass one exact, unchanged operator-selected installer FD to the manager.

The canonical manager performs the private import; this only opens the supplied
Downloads file without following links and binds its expected read-only digest.
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import stat


def stable(fd):
    m = os.fstat(fd)
    if not stat.S_ISREG(m.st_mode) or m.st_uid != os.getuid() or m.st_nlink != 1:
        raise ValueError("installer file custody")
    return (m.st_dev, m.st_ino, m.st_size, m.st_mtime_ns, m.st_ctime_ns, m.st_nlink)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--file", required=True)
    p.add_argument("--sha256", required=True)
    p.add_argument("--size", type=int, required=True)
    a = p.parse_args()
    if not re.fullmatch("[0-9a-f]{64}", a.sha256) or not 512 <= a.size <= 4 * 1024**3:
        raise ValueError("installer expected identity")
    path = pathlib.Path(a.file)
    if not path.is_absolute() or path.is_symlink() or path.resolve() != path:
        raise ValueError("installer path alias")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        before = stable(fd)
        if before[2] != a.size:
            raise ValueError("installer size changed")
        h = hashlib.sha256()
        while data := os.read(fd, 65536):
            h.update(data)
        if h.hexdigest() != a.sha256 or stable(fd) != before:
            raise ValueError("installer bytes changed")
        selected = pathlib.Path.home() / ".local/share/linux-vst-bridge/managed/daw-workspaces/tooling/selection.json"
        selection = json.loads(selected.read_bytes())
        manager = pathlib.Path(selection["manager"]["path"])
        generation = selection["generation"]
        if manager != selected.parent / "generations" / generation / "linux-vst-bridge":
            raise ValueError("manager generation path")
        if not manager.is_file() or manager.is_symlink() or manager.stat().st_uid != os.getuid():
            raise ValueError("manager artifact custody")
        with manager.open("rb") as source:
            if hashlib.file_digest(source, "sha256").hexdigest() != selection["manager"]["sha256"]:
                raise ValueError("manager artifact changed")
        os.lseek(fd, 0, os.SEEK_SET)
        os.dup2(fd, 0)
        os.execv(manager, [str(manager), "workspace", "import"])
    finally:
        os.close(fd)


if __name__ == "__main__":
    main()

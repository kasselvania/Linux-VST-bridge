#!/usr/bin/env python3
"""Build/install one immutable WD0-only manager + supervisor closure.

This does not replace the native bridge service, software.json, or any proxy.
The ordinary desktop entry calls the same typed manager command as the CLI.
"""
import argparse
import hashlib
import json
import os
import pathlib
import stat
import subprocess
import sys

NAMES = ("linux-vst-bridge", "session.py", "ownership.py", "import_installer.py")
DESKTOP = "linux-vst-bridge-fl-studio.desktop"


def checked_file(path):
    path = pathlib.Path(path)
    if not path.is_absolute() or path.is_symlink() or path.resolve() != path:
        raise ValueError("WD0 artifact path alias")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode) or before.st_uid != os.getuid() or before.st_nlink != 1:
            raise ValueError("WD0 artifact owner/type")
        h = hashlib.sha256()
        with os.fdopen(fd, "rb", closefd=False) as f:
            while chunk := f.read(65536):
                h.update(chunk)
        after = os.fstat(fd)
        stamp = lambda m: (m.st_dev, m.st_ino, m.st_size, m.st_mtime_ns, m.st_ctime_ns, m.st_nlink)
        if stamp(before) != stamp(after):
            raise ValueError("WD0 artifact changed while hashing")
        return {"sha256": h.hexdigest(), "size": before.st_size}
    finally:
        os.close(fd)


def linux_x64_manager(path):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        header = os.read(fd, 20)
    finally:
        os.close(fd)
    if len(header) != 20 or header[:6] != b"\x7fELF\x02\x01" or header[18:20] != b"\x3e\x00":
        raise ValueError("WD0 manager must be Linux x86-64 ELF")


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def write_new(path, data, mode=0o600):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    with os.fdopen(fd, "wb") as f:
        f.write(data)
        f.flush()
        os.fsync(f.fileno())


def build(args):
    source = pathlib.Path(args.source).resolve(strict=True)
    if git("-C", str(source), "status", "--porcelain"):
        raise ValueError("WD0 source checkout must be committed and clean")
    head = git("-C", str(source), "rev-parse", "HEAD")
    tree = git("-C", str(source), "rev-parse", "HEAD^{tree}")
    manager = pathlib.Path(args.manager).resolve(strict=True)
    linux_x64_manager(manager)
    supervisor = source / "bridge-manager/runtime/session.py"
    ownership = source / "bridge-manager/runtime/ownership.py"
    importer = source / "tools/wd0/import_installer.py"
    meta = {"schema": 1, "source_head": head, "source_tree": tree,
            "files": {NAMES[0]: checked_file(manager), NAMES[1]: checked_file(supervisor),
                      NAMES[2]: checked_file(ownership), NAMES[3]: checked_file(importer)}}
    generation = hashlib.sha256(canonical(meta)).hexdigest()
    meta["generation"] = generation
    package = pathlib.Path(args.output).resolve()
    if package.exists():
        raise ValueError("WD0 package output already exists")
    package.mkdir(mode=0o700, parents=True)
    for name, source_file in [(NAMES[0], manager), (NAMES[1], supervisor),
                              (NAMES[2], ownership), (NAMES[3], importer)]:
        dest = package / name
        with source_file.open("rb") as src:
            write_new(dest, src.read(), 0o500 if name == NAMES[0] else 0o400)
        if checked_file(dest) != meta["files"][name]:
            raise ValueError("WD0 package copy differs")
    write_new(package / "package.json", canonical(meta) + b"\n")
    print(json.dumps({"generation": generation, "source_head": head, "source_tree": tree}))


def private_dir(path):
    path = pathlib.Path(path)
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    md = path.lstat()
    if not stat.S_ISDIR(md.st_mode) or md.st_uid != os.getuid() or md.st_mode & 0o077 or path.resolve() != path:
        raise ValueError("WD0 private directory differs")


def owned_dir(path):
    path = pathlib.Path(path)
    path.mkdir(mode=0o700, parents=True, exist_ok=True)
    md = path.lstat()
    if not stat.S_ISDIR(md.st_mode) or md.st_uid != os.getuid() or path.resolve() != path:
        raise ValueError("WD0 desktop directory differs")


def atomic_replace(path, data, mode=0o600):
    tmp = path.with_name("." + path.name + "-" + os.urandom(8).hex())
    write_new(tmp, data, mode)
    os.replace(tmp, path)
    fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def install(args):
    package = pathlib.Path(args.package).resolve(strict=True)
    meta = json.loads((package / "package.json").read_bytes())
    if set(meta) != {"schema", "source_head", "source_tree", "files", "generation"} or meta["schema"] != 1:
        raise ValueError("WD0 package schema")
    if set(meta["files"]) != set(NAMES) or len(meta["source_head"]) != 40 or len(meta["source_tree"]) != 40:
        raise ValueError("WD0 package authority")
    generation = meta["generation"]
    if len(generation) != 64 or generation != hashlib.sha256(canonical({k: v for k, v in meta.items() if k != "generation"})).hexdigest():
        raise ValueError("WD0 package generation")
    for name in NAMES:
        if checked_file(package / name) != meta["files"][name]:
            raise ValueError("WD0 package artifact differs")
    linux_x64_manager(package / NAMES[0])
    home = pathlib.Path.home()
    root = home / ".local/share/linux-vst-bridge/managed"
    private_dir(root)
    tooling = root / "daw-workspaces/tooling"
    generations = tooling / "generations"
    private_dir(tooling.parent)
    private_dir(generations)
    dest = generations / generation
    if dest.exists():
        private_dir(dest)
        for name in NAMES:
            if checked_file(dest / name) != meta["files"][name]:
                raise ValueError("WD0 installed generation differs")
    else:
        dest.mkdir(mode=0o700)
        for name in NAMES:
            with (package / name).open("rb") as src:
                write_new(dest / name, src.read(), 0o500 if name == NAMES[0] else 0o400)
            if checked_file(dest / name) != meta["files"][name]:
                raise ValueError("WD0 installed artifact differs")
    selection = {"schema": 1, "generation": generation,
                 "source_head": meta["source_head"], "source_tree": meta["source_tree"],
                 "manager": {"path": str(dest / NAMES[0]), "sha256": meta["files"][NAMES[0]]["sha256"]},
                 "supervisor": {"path": str(dest / NAMES[1]), "sha256": meta["files"][NAMES[1]]["sha256"]},
                 "ownership": {"path": str(dest / NAMES[2]), "sha256": meta["files"][NAMES[2]]["sha256"]}}
    selection["importer"] = {"path": str(dest / NAMES[3]), "sha256": meta["files"][NAMES[3]]["sha256"]}
    selected = tooling / "selection.json"
    if selected.exists():
        prior = json.loads(selected.read_bytes())
        if prior != selection:
            rollback = tooling / ("selection-prior-" + prior["generation"] + ".json")
            if rollback.exists() and json.loads(rollback.read_bytes()) != prior:
                raise ValueError("WD0 prior selection differs")
            if not rollback.exists():
                write_new(rollback, canonical(prior) + b"\n")
    applications = home / ".local/share/applications"
    owned_dir(applications)
    desktop = applications / DESKTOP
    marker = "X-LinuxVSTBridge-Owner=WD0\n"
    if desktop.exists() and (desktop.is_symlink() or marker not in desktop.read_text()):
        raise ValueError("WD0 desktop entry foreign")
    entry = ("[Desktop Entry]\n" + marker + "Type=Application\nName=FL Studio (Managed)\n"
             + "Comment=Managed Windows DAW workspace\n"
             + f"Exec={dest / NAMES[0]} workspace launch\n"
             + "Terminal=false\nCategories=AudioVideo;Audio;Music;\n")
    atomic_replace(selected, canonical(selection) + b"\n")
    atomic_replace(desktop, entry.encode())
    print(json.dumps({"installed_generation": generation, "desktop_entry": str(desktop)}))


def main():
    os.umask(0o077)
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="action", required=True)
    b = sub.add_parser("build")
    b.add_argument("--source", required=True)
    b.add_argument("--manager", required=True)
    b.add_argument("--output", required=True)
    i = sub.add_parser("install")
    i.add_argument("--package", required=True)
    args = p.parse_args()
    (build if args.action == "build" else install)(args)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        print(f"WD0 package refused: {exc}", file=sys.stderr)
        sys.exit(1)

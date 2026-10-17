#!/usr/bin/env python3
"""Integrate recovered corrections into one pinned Wine source comparison.

This build tool has no installed catalog, profile, prefix or runtime-selection
authority. Checks use temporary copies; preparation requires pristine Git input.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parent
TOOLS = ROOT.parent
RECIPE = json.loads((ROOT / "source.json").read_bytes())


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_sources(source, state):
    for name, hashes in RECIPE["sources"].items():
        path = source / name
        expected = hashes[state]
        if path.is_symlink() or (expected is None and path.exists()):
            raise ValueError("unexpected source: " + name)
        if expected is not None and (not path.is_file() or sha(path) != expected):
            raise ValueError(state + " source identity differs: " + name)


def integrate(source):
    verify_sources(source, "before")
    for patch in RECIPE["patches"]:
        path = TOOLS / patch["path"]
        if sha(path) != patch["sha256"]:
            raise ValueError("patch identity differs: " + patch["path"])
        subprocess.run(["git", "apply", "--check", str(path)], cwd=source,
                       check=True, timeout=30)
        subprocess.run(["git", "apply", str(path)], cwd=source,
                       check=True, timeout=30)
    for fixture in RECIPE["fixtures"]:
        path = TOOLS / fixture["path"]
        if sha(path) != fixture["sha256"]:
            raise ValueError("fixture identity differs: " + fixture["path"])
        shutil.copyfile(path, source / fixture["target"])
    verify_sources(source, "after")


def check(source=None):
    if source is not None:
        verify_sources(source, "before")
    with tempfile.TemporaryDirectory(prefix="lvb-common-source-") as directory:
        stage = Path(directory)
        for name, hashes in RECIPE["sources"].items():
            if hashes["before"] is None:
                continue
            target = stage / name
            target.parent.mkdir(parents=True, exist_ok=True)
            if source is not None:
                if (source / name).is_symlink():
                    raise ValueError("source link: " + name)
                shutil.copyfile(source / name, target)
            else:
                url = ("https://raw.githubusercontent.com/giang17/wine/"
                       + RECIPE["wine"]["commit"] + "/" + name)
                with urllib.request.urlopen(url, timeout=30) as response:
                    data = response.read(2 * 1024 * 1024 + 1)
                if len(data) > 2 * 1024 * 1024:
                    raise ValueError("source file exceeds check bound: " + name)
                target.write_bytes(data)
        integrate(stage)


def prepare(source):
    for ref, expected in (("HEAD", RECIPE["wine"]["commit"]),
                          ("HEAD^{tree}", RECIPE["wine"]["tree"])):
        actual = subprocess.check_output(["git", "rev-parse", ref], cwd=source,
                                         text=True, timeout=30).strip()
        if actual != expected:
            raise ValueError("Wine Git input differs: " + ref)
    if subprocess.check_output(["git", "status", "--porcelain=v1",
                                "--untracked-files=all"], cwd=source,
                               text=True, timeout=30):
        raise ValueError("Wine build checkout is not pristine")
    integrate(source)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check-upstream", action="store_true")
    mode.add_argument("--check-source", type=Path,
                      help="Check pristine source without modifying it")
    mode.add_argument("--prepare-source", type=Path,
                      help="Apply to a pristine pinned build checkout")
    mode.add_argument("--verify-prepared-source", type=Path,
                      help="Check integrated source postimages before composition")
    args = parser.parse_args()
    if args.verify_prepared_source:
        verify_sources(args.verify_prepared_source.resolve(), "after")
    elif args.prepare_source:
        prepare(args.prepare_source.resolve())
    else:
        check(args.check_source.resolve() if args.check_source else None)
    print("Common Wine source inputs and integrated postimages verified")


if __name__ == "__main__":
    main()

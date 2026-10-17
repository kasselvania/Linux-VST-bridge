#!/usr/bin/env python3
"""Emit allow-listed construction facts for the completed common Wine build.

Canonical declaration semantics live in runtime_delivery/wine_build.rs. This
builder writes no qualification, installed selection or readiness observation.
"""
import argparse
import hashlib
import json
from pathlib import Path
import runpy
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HERE = ROOT / "tools/runtime"
HELPER_SOURCES = ("tools/direct-audio-handoff/linux_wait.h",
                  "tools/direct-audio-handoff/linux_wait.rs",
                  "tools/direct-audio-handoff/shim.cpp")
RECIPE_FILES = ("source.json", "build-inputs.json", "redist-inputs.json",
                "prepare.py", "build-inputs.py", "redist-inputs.py",
                "build-inner.sh", "sdk.sh", "build-work.sh", "complete-work.sh",
                "build-identity.py")


def sha(path):
    if path.is_symlink() or not path.is_file():
        raise ValueError("Build input missing or linked: " + path.name)
    return hashlib.sha256(path.read_bytes()).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def recipe_identity():
    recipe = json.loads((HERE / "source.json").read_bytes())
    names = {"tools/runtime/" + name for name in RECIPE_FILES}
    for item in [*recipe["patches"], *recipe["fixtures"]]:
        name = "tools/" + item["path"]
        if sha(ROOT / name) != item["sha256"]:
            raise ValueError("Pinned recipe input differs: " + name)
        names.add(name)
    names.update("tools/runtime/patches/" + name for name in
                 ("proton-redist-inputs.patch", "proton-audio-composition.patch"))
    return hashlib.sha256(canonical({name: sha(ROOT / name) for name in sorted(names)})).hexdigest()


def declaration(output):
    recipe = json.loads((HERE / "source.json").read_bytes())
    prepared = output / "proton/wine"
    runpy.run_path(str(HERE / "prepare.py"))["verify_sources"](prepared, "after")
    redist = runpy.run_path(str(HERE / "redist-inputs.py"))
    redist["verify_commits"](output / "proton")
    if redist["source_state"](output / "proton") != 1:
        raise ValueError("Proton source is not prepared")
    if sha(output / "build-inner.sh") != sha(HERE / "build-inner.sh"):
        raise ValueError("Executed build invocation differs")
    wanted_makefile = ("SRCDIR := /work/proton\nBUILD_NAME := lvb-source-comparison\n"
                       "include /work/proton/Makefile.in\n")
    if (output / "build/Makefile").read_text() != wanted_makefile:
        raise ValueError("Build invocation Makefile differs")
    expected_tracked = {name for name, value in recipe["sources"].items()
                        if value["before"] is not None and value["before"] != value["after"]}
    expected_untracked = {name for name, value in recipe["sources"].items() if value["before"] is None}
    inputs = json.loads((HERE / "build-inputs.json").read_bytes())
    for name in ("vk.xml", "video.xml"):
        relative = "dlls/winevulkan/" + name
        if sha(prepared / relative) != inputs[name]["sha256"]:
            raise ValueError("Vulkan source input differs")
        expected_untracked.add(relative)
    for arguments, expected in ((["diff", "--name-only", "HEAD"], expected_tracked),
                                 (["ls-files", "--others", "--exclude-standard"], expected_untracked)):
        actual = subprocess.check_output(["git", *arguments], cwd=prepared, text=True).splitlines()
        if set(actual) != expected:
            raise ValueError("Prepared Wine source roster differs")
    configurations = {}
    for arch in ("i386", "x86_64"):
        if not (output / ("build/.wine-" + arch + "-build")).is_file():
            raise ValueError("Wine build target is incomplete: " + arch)
        configurations[arch] = sha(output / ("build/obj-wine-" + arch + "/Makefile"))
    tool = output / "build/dst-wine-x86_64/bin/winebuild"
    elf = tool.read_bytes()[:20]
    if len(elf) != 20 or elf[:6] != b"\x7fELF\x02\x01" or int.from_bytes(elf[18:20], "little") != 62:
        raise ValueError("Own winebuild is not Linux x86-64 ELF")
    epoch = recipe["proton"]["source_date_epoch"]
    return dict(schema=1, kind="wine_build", recipe_sha256=recipe_identity(),
                wine_revision=recipe["wine"]["commit"], sdk_image=recipe["sdk"],
                configuration_sha256=configurations,
                invocation=dict(target="wine", source_date_epoch=epoch,
                                base_source_date_epoch=epoch, build_name="lvb-source-comparison",
                                make_arguments=["-j3", "J=3", "CONTAINER=1", "WINEMONO_VER=10.4.1"]),
                helper_abi=1, helper_sources={name: sha(ROOT / name) for name in HELPER_SOURCES},
                winebuild_sha256=sha(tool))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-root", required=True, type=Path)
    args = parser.parse_args()
    output = args.build_root.resolve(strict=True)
    path = output / "build/redist/wine-build.json"
    data = canonical(declaration(output))
    if path.exists():
        if path.is_symlink() or path.read_bytes() != data:
            raise ValueError("Existing build declaration differs")
    else:
        with path.open("xb") as stream:
            stream.write(data)
    print("Exact common Wine build declaration: " + hashlib.sha256(data).hexdigest())


if __name__ == "__main__":
    main()

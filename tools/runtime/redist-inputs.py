#!/usr/bin/env python3
"""Acquire exact upstream redist packages and corresponding release source.

This prepares private build inputs. It does not publish a runtime or discharge
the remaining notice/source distribution obligations.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import runpy
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parent
INPUTS = json.loads((ROOT / "redist-inputs.json").read_bytes())
RECIPE = json.loads((ROOT / "source.json").read_bytes())
SOURCES = {
    "Makefile.in": (
        "25d1250ece9fe6d406784237242e1f01548ac7449ba67e316d314356f3b5d1f5",
        "5d015d2c44bb37ea9127e3c0ead6fde254974a13a5dcce3837b4f4af256564b1"),
    "make/rules-cargo.mk": (
        "178b08f2877677a0c2c404682af7b6fe1492f1b6bcaab0a95aa8a35ad84b5861",
        "d2fc3ed4bd31e81e474da31bcae0d55c655431eca52ec7d66e744a53999ad5be"),
    "default_pfx.py": (
        "474c182df7ea0fa5c6b0a6495728b24655cc504848c45b638fb48c3b25a5b794",
        "14a5a426ad497b2624d1fbbef16b0f7ac011124f40a3ff9726c832cb51f42b90"),
}


def source_state(source):
    values = []
    for name, hashes in SOURCES.items():
        path = source / name
        if path.is_symlink() or not path.is_file():
            raise ValueError("redist source missing or linked: " + name)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest not in hashes:
            raise ValueError("redist source identity differs: " + name)
        values.append(hashes.index(digest))
    if len(set(values)) != 1:
        raise ValueError("redist source preparation is incomplete")
    return values[0]


def patch_source(source):
    patch = ROOT / "patches/proton-redist-inputs.patch"
    if hashlib.sha256(patch.read_bytes()).hexdigest() != "d7948496aa7f68aafd6009bdbd18dd6a93319640dd3b824e2baa3ee1b9c30e08":
        raise ValueError("redist build patch changed")
    if source_state(source) == 0:
        subprocess.run(["git", "apply", "--check", str(patch)], cwd=source, check=True)
        subprocess.run(["git", "apply", str(patch)], cwd=source, check=True)
    if source_state(source) != 1:
        raise ValueError("redist source postimages differ")


def acquire(path, spec):
    if path.exists():
        if path.is_symlink() or path.stat().st_size != spec["size"]:
            raise ValueError("redist input changed: " + path.name)
    else:
        with tempfile.NamedTemporaryFile(dir=path.parent, prefix=path.name + ".", delete=False) as out:
            temporary = Path(out.name)
            try:
                with urllib.request.urlopen(spec["url"], timeout=60) as response:
                    count = 0
                    digest = hashlib.sha256()
                    while block := response.read(1024 * 1024):
                        count += len(block)
                        if count > spec["size"]:
                            raise ValueError("redist input exceeds pinned extent: " + path.name)
                        out.write(block)
                        digest.update(block)
                if count != spec["size"] or digest.hexdigest() != spec["sha256"]:
                    raise ValueError("redist input identity differs: " + path.name)
                out.close()
                os.replace(temporary, path)
            finally:
                temporary.unlink(missing_ok=True)
    digest = hashlib.sha256()
    with path.open("rb") as source:
        while block := source.read(1024 * 1024):
            digest.update(block)
    if path.stat().st_size != spec["size"] or digest.hexdigest() != spec["sha256"]:
        raise ValueError("redist input identity differs: " + path.name)


def notices(output):
    destination = output / "third-party-notices"
    for name, wanted in INPUTS["source_notices"].items():
        found = set()
        with tarfile.open(output / "corresponding-source" / name, mode="r|xz") as archive:
            for member in archive:
                if member.name not in wanted:
                    continue
                spec = wanted[member.name]
                if not member.isfile() or member.size != spec["size"] or member.name in found:
                    raise ValueError("source notice member differs: " + member.name)
                data = archive.extractfile(member).read(spec["size"] + 1)
                if hashlib.sha256(data).hexdigest() != spec["sha256"]:
                    raise ValueError("source notice identity differs: " + member.name)
                (destination / spec["name"]).write_bytes(data)
                found.add(member.name)
        if found != wanted.keys():
            raise ValueError("source notice missing: " + name)
    # The verified ONNX auxiliary archive carries these original notices.
    onnx = output / "proton/external-inputs/onnxruntime.tgz"
    spec = json.loads((ROOT / "build-inputs.json").read_bytes())["onnxruntime.tgz"]
    if onnx.stat().st_size != spec["size"] or hashlib.sha256(onnx.read_bytes()).hexdigest() != spec["sha256"]:
        raise ValueError("ONNX notice archive identity differs")
    wanted = {"LICENSE", "ThirdPartyNotices.txt", "Privacy.md"}
    with tarfile.open(onnx, mode="r|gz") as archive:
        for member in archive:
            name = Path(member.name).name
            if member.name != "onnxruntime-linux-x64-1.14.1/" + name or name not in wanted:
                continue
            if not member.isfile() or member.size > 1024 * 1024:
                raise ValueError("ONNX notice member differs")
            (destination / ("onnxruntime-" + name)).write_bytes(archive.extractfile(member).read())
            wanted.remove(name)
    if wanted:
        raise ValueError("ONNX auxiliary notices missing")
    shutil.copyfile(ROOT / "redist-inputs.json", destination / "inputs.json")


def verify_commits(source):
    for name in ("proton", "wine"):
        path = source if name == "proton" else source / "wine"
        actual = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=path, text=True).strip()
        if actual != RECIPE[name]["commit"]:
            raise ValueError("redist source commit differs: " + name)


def prepare(output):
    source = output / "proton"
    verify_commits(source)
    status = subprocess.check_output(["git", "submodule", "status", "--recursive"],
                                     cwd=source, text=True)
    for line in status.splitlines():
        path = line[1:].split()[1]
        if path == "wine":  # Independently pinned and checked above.
            continue
        if path == "FEX" and line[0] == "-":  # Not used by this x86_64 build.
            continue
        if line[0] != " ":
            raise ValueError("redist auxiliary Git input differs: " + path)
    source_state(source)
    for group, directory in (("packages", source / "contrib"),
                             ("corresponding_source", output / "corresponding-source"),
                             ("notices", output / "third-party-notices")):
        directory.mkdir(exist_ok=True)
        for name, spec in INPUTS[group].items():
            acquire(directory / name, spec)
    patch_source(source)
    notices(output)
    print("Matching Mono, Gecko, Xalia and release-source inputs verified")


def check():
    with tempfile.TemporaryDirectory(prefix="lvb-redist-source-") as directory:
        source = Path(directory).resolve()
        for name in SOURCES:
            url = ("https://raw.githubusercontent.com/ValveSoftware/Proton/"
                   + RECIPE["proton"]["commit"] + "/" + name)
            target = source / name
            target.parent.mkdir(parents=True, exist_ok=True)
            with urllib.request.urlopen(url, timeout=30) as response:
                data = response.read(1024 * 1024 + 1)
            if len(data) > 1024 * 1024:
                raise ValueError("redist source exceeds check bound")
            target.write_bytes(data)
        patch_source(source)
        patch_source(source)  # Composition can resume after an acquisition failure.
        # Execute the pinned x64 post-install recipe with only c27's real file
        # layout. Obsolete wine64 aliases must not be required or cross-copied.
        installed = source / "installed-x64"
        (installed / "bin").mkdir(parents=True)
        (installed / "share/wine").mkdir(parents=True)
        for name in ("wine", "wineserver", "msidb"):
            (installed / "bin" / name).write_text("x86_64 " + name)
        (installed / "share/wine/wine.inf").write_text("same-source metadata")
        distribution = source / "distribution"
        (source / "post-obj").mkdir()
        target = "$(OBJ)/.wine-x86_64-post-build:"
        make_source = (source / "Makefile.in").read_text()
        recipe = make_source[make_source.index(target):make_source.index("$(OBJ)/.wine-aarch64-post-build:")]
        post_makefile = source / "post-install.mk"
        post_makefile.write_text("OBJ := " + str(source / "post-obj") + "\n"
                                "WINE_x86_64_DST := " + str(installed) + "\n"
                                "DST_DIR := " + str(distribution) + "\n"
                                "install-strip = cp $(1) $(2)/\n" + recipe)
        subprocess.run(["make", "--no-print-directory", "-f", str(post_makefile)],
                       cwd=source, env={"PATH": os.defpath}, check=True)
        if (distribution / "bin/wine").read_text() != "x86_64 wine" or (distribution / "lib/wine/i386-unix").exists():
            raise ValueError("native loader post-install layout differs")
        package = source / "package"
        package.mkdir()
        obj = source / "obj"
        obj.mkdir()
        binary = source / "bin"
        binary.mkdir()
        cargo = binary / "cargo"
        cargo.write_text("#!" + sys.executable + "\n"
                         "import os,sys\n"
                         "assert os.getcwd() == os.environ['LVB_EXPECT_PACKAGE']\n"
                         "assert os.environ['LVB_CARGO_TEST'] == 'target-environment'\n"
                         "assert sys.argv[1:3] == ['build', '--frozen']\n"
                         "assert '--target' in sys.argv and 'x86_64-unknown-linux-gnu' in sys.argv\n")
        cargo.chmod(0o755)
        makefile = source / "fixture.mk"
        makefile.write_text("OBJ := " + str(obj) + "\nARCHS := x86_64-unix\n"
                            "PKG_SRC := " + str(package) + "\n"
                            "PKG_x86_64_OBJ := " + str(obj / "target") + "\n"
                            "PKG_x86_64_ENV := LVB_CARGO_TEST=target-environment PATH="
                            + str(binary) + ":/usr/bin:/bin\n"
                            "include " + str(source / "make/rules-cargo.mk") + "\n"
                            "$(eval $(call create-rules-cargo,fixture,PKG,x86_64,unix))\n")
        subprocess.run(["make", "--no-print-directory", "-f", str(makefile),
                        str(obj / ".fixture-x86_64-build")], cwd=source,
                       env={"PATH": os.defpath, "LVB_EXPECT_PACKAGE": str(package)}, check=True)
        drives = source / "prefix/dosdevices"
        drives.mkdir(parents=True)
        (drives / "c:").symlink_to(source, target_is_directory=True)
        (drives / "com1:").symlink_to("/dev/ttyS0")
        (drives / "z:").symlink_to("/definitely-absent-lvb-build-path")
        (drives / "keep.txt").write_text("keep")
        (drives / "keep-link").symlink_to("keep.txt")
        namespace = runpy.run_path(str(source / "default_pfx.py"))
        namespace["fixup_drive_links"](str(drives.parent))
        if sorted(path.name for path in drives.iterdir()) != ["keep-link", "keep.txt"]:
            raise ValueError("generated prefix retains host drive/device links")
        (source / "Makefile.in").write_text("changed")
        try:
            patch_source(source)
        except ValueError:
            pass
        else:
            raise ValueError("modified redist source was accepted")
    print("Pinned source, native loader layout, Cargo execution and generated-prefix regressions verified")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--build-root", type=Path)
    mode.add_argument("--prepare-build-source", type=Path,
                      help="Apply pinned Proton integration before the Wine target")
    mode.add_argument("--check-upstream", action="store_true")
    args = parser.parse_args()
    if args.check_upstream:
        check()
    elif args.prepare_build_source:
        verify_commits(args.prepare_build_source.resolve())
        patch_source(args.prepare_build_source.resolve())
    else:
        prepare(args.build_root.resolve())

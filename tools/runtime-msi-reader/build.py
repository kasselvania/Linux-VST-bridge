#!/usr/bin/env python3
"""Build Wine's MSI module with the Kontakt table-reader correction.

Source is GE-Proton11-7's exact Wine submodule commit plus the one wine-staging
MSI change GE applies. Two upstream Wine MSI commits, the narrow ambiguous
string-reference correction and dated LGPL modification notices follow. Only
msi.dll is compiled, for i386-windows and x86_64-windows, in the pinned SDK.

This produces reference DLLs and a manifest. It never adopts a runner or
touches a prefix. Wine source and binaries stay outside the proprietary build.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tarfile
import urllib.request

WINE = "46b29104e3741fe23bf5e2547196a253aab88c89"
ARCHIVE_URL = "https://codeload.github.com/ValveSoftware/wine/tar.gz/" + WINE
ARCHIVE_SHA = "7962bca6cb05628a56cc03008ea7af6cee7245f5b98a828452465e2775e18340"
SDK = "registry.gitlab.steamos.cloud/proton/steamrt4/sdk/x86_64@sha256:6c1789cad862fd8ed9d46d71bbf8752bb7cb2336ab5e9680c0f0e3e7c88d77f1"
TABLE = "dlls/msi/table.c"
# Git blob IDs (pristine, fully patched) of every changed file. Pristine table.c
# equals the parent of 24bbf46.
SOURCES = {
    TABLE: ("6031dd9c80582d03c616408950ada043c520ca4a", "1fed69f5c2cb1f21a29667b68228176d99f01f7b"),
    "dlls/msi/files.c": ("e7686d07627d93ce9ec6a106bb78270b1e8ed7bb", "d81163ac1b4f4fdab4ef1b2dbb4454722408107a"),
}
# After the Kontakt patch, table.c is the exact reader the Deck runner used
# (the patch's own target blob); only the notice comment follows.
READER_PATCH = "0003-kontakt-msi-ambiguous-strrefs.patch"
READER_BLOB = "87e9854679255706253001714380774f087e1223"
PATCHES = [
    ("0000-staging-msi-cabinet.patch",
     "24e2a1f32b96f2e377038333027469aabc5bbdda69a72848e62de00df1609ace",
     "wine-staging 6cc805ea57132eeaf44764e9213823c9b8d0d300 msi-cabinet, applied by GE-Proton11-7"),
    ("0001-upstream-24bbf46-table-stream-padding.patch",
     "63d9d4f5968e502186b9a3711d6daa03fc67394e892bf485496097f5080965e5",
     "ValveSoftware/wine 24bbf46c6f055077df428341595873fe23475a60"),
    ("0002-upstream-e013097-mismatched-strref-widths.patch",
     "ae67f622483e2de851393e5542afca9ff1478e2f8e79f2399d7cbeb46a8a3a47",
     "ValveSoftware/wine e0130972d5ffe578a4a25d75f4c1d0229c880a8c"),
    ("0003-kontakt-msi-ambiguous-strrefs.patch",
     "ed4a47d3b167f711088251f56f6377347a8539d4c308281c55f92825a174d979",
     "Linux-VST-bridge; see docs/KONTAKT_MSI_FINDINGS.md"),
    ("0004-lgpl-modification-notices.patch",
     "6e9258da9e5b6a8b0bdba44939d2dd95009f77f1ac53076a2691ccebafc4ec11",
     "Linux-VST-bridge; dated LGPL-2.1 section 2(a) notices"),
]
ARTIFACTS = {
    "i386-windows/msi.dll": ("obj/dlls/msi/i386-windows/msi.dll", 0x014C),
    "x86_64-windows/msi.dll": ("obj/dlls/msi/x86_64-windows/msi.dll", 0x8664),
}
SCRIPT = """set -eu
export XDG_CACHE_HOME=/work/cache
cd /work/wine
dlls/winevulkan/make_vulkan -x vk.xml -X video.xml > /work/vulkan-generation.log 2>&1
tools/make_specfiles > /work/spec-generation.log 2>&1
tools/make_requests > /work/request-generation.log 2>&1
autoreconf -fi > /work/autoconf.log 2>&1
mkdir /work/obj
cd /work/obj
../wine/configure --enable-win64 --enable-archs=i386,x86_64 --with-mingw=gcc --without-x --without-wayland CROSSLDFLAGS=-Wl,--no-insert-timestamp > /work/configure.log 2>&1
make -j2 dlls/msi/i386-windows/msi.dll dlls/msi/x86_64-windows/msi.dll > /work/make.log 2>&1
"""


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        while block := f.read(1024 * 1024):
            h.update(block)
    return h.hexdigest()


def git_blob(path):
    data = path.read_bytes()
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


def check_builtin_pe(path, machine):
    """Return True for a Wine builtin PE image of the expected machine."""
    data = path.read_bytes()
    if data[:2] != b"MZ" or data[0x40:0x50] != b"Wine builtin DLL":
        return False
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    return data[pe:pe + 4] == b"PE\0\0" and struct.unpack_from("<H", data, pe + 4)[0] == machine


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--wine-archive", type=Path, help="Reuse an exact verified source archive")
    args = parser.parse_args()
    root = args.output.resolve()
    root.mkdir(mode=0o700)
    here = Path(__file__).resolve().parent
    for name, digest, _ in PATCHES:
        if sha(here / name) != digest:
            raise ValueError("patch identity changed: " + name)
    archive = root / "wine.tar.gz"
    if args.wine_archive:
        if sha(args.wine_archive) != ARCHIVE_SHA:
            raise ValueError("reference source changed")
        shutil.copyfile(args.wine_archive, archive)
    else:
        with urllib.request.urlopen(ARCHIVE_URL, timeout=60) as response, archive.open("xb") as out:
            total = 0
            while block := response.read(1024 * 1024):
                total += len(block)
                if total > 128 * 1024 * 1024:
                    raise ValueError("reference source extent")
                out.write(block)
    if sha(archive) != ARCHIVE_SHA:
        raise ValueError("reference Wine source changed")
    source = root / "wine"
    source.mkdir(mode=0o700)
    prefix = "wine-" + WINE + "/"
    with tarfile.open(archive) as tar:
        for member in tar.getmembers():
            if member.name == prefix[:-1]:
                continue
            if not member.name.startswith(prefix):
                raise ValueError("source archive root")
            member.name = member.name[len(prefix):]
            tar.extract(member, source, filter="data")
    for name, (before, _) in SOURCES.items():
        if git_blob(source / name) != before:
            raise ValueError("unexpected pristine " + name)
    for name, _, _ in PATCHES:
        subprocess.run(["patch", "--batch", "--fuzz=0", "-p1", "-i", str(here / name)],
                       cwd=source, check=True, timeout=30)
        if name == READER_PATCH and git_blob(source / TABLE) != READER_BLOB:
            raise ValueError("unexpected reader " + TABLE)
    for name, (_, after) in SOURCES.items():
        if git_blob(source / name) != after:
            raise ValueError("unexpected patched " + name)
    consumed = {name: sha(source / name) for name in SOURCES}
    for path in [Path(__file__).resolve()] + [here / name for name, _, _ in PATCHES]:
        shutil.copyfile(path, root / path.name)
    shutil.copyfile(source / "COPYING.LIB", root / "COPYING.LIB")
    script = root / "build.sh"
    script.write_text(SCRIPT)
    script_sha = sha(script)
    # One capped developer builder, matching the UIA reference build limits.
    subprocess.run(["docker", "run", "--rm", "--name", "lvb-msi-reader-builder",
                    "--cpus", "2", "--cpuset-cpus", "2,3", "--memory", "4g",
                    "--memory-swap", "4g", "--pids-limit", "256", "--network", "none",
                    "--user", f"{os.getuid()}:{os.getgid()}",
                    "--env", "XDG_CACHE_HOME=/work/cache",
                    "--entrypoint", "/bin/sh", "--volume",
                    str(root) + ":/work", SDK, "/work/build.sh"], check=True, timeout=3600)
    if sha(script) != script_sha or any(sha(source / name) != digest
                                      for name, digest in consumed.items()):
        raise ValueError("consumed source or build recipe changed during compilation")
    artifacts = {}
    out = root / "out"
    for name, (relative, machine) in ARTIFACTS.items():
        built = root / relative
        if not check_builtin_pe(built, machine):
            raise ValueError("not a Wine builtin PE image for its architecture: " + name)
        target = out / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(built, target)
        artifacts[name] = {"sha256": sha(target), "size": target.stat().st_size,
                           "wine_builtin_marker": True}
    manifest = {"schema": 1, "kind": "wine_msi_reader_reference_build",
                "wine_commit": WINE, "wine_archive_url": ARCHIVE_URL,
                "wine_archive_sha256": ARCHIVE_SHA, "sdk_image": SDK,
                "patches": [{"file": name, "sha256": digest, "source": source_ref}
                            for name, digest, source_ref in PATCHES],
                "patched_sources": {name: {"sha256": consumed[name], "git_blob_before": before,
                                           "git_blob_after": after}
                                    for name, (before, after) in SOURCES.items()},
                "reader_checkpoint": {"after": READER_PATCH, "path": TABLE,
                                      "git_blob": READER_BLOB},
                "license": "LGPL-2.1-or-later", "build_script_sha256": script_sha,
                "builder_sha256": sha(root / "build.py"),
                "artifacts": artifacts, "adopts_runtime": False}
    (root / "build-manifest.json").write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n")
    print(json.dumps(manifest, sort_keys=True))


if __name__ == "__main__":
    main()

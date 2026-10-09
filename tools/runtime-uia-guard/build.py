#!/usr/bin/env python3
"""Build the exact GE-based x64 Wine UI Automation correction and its regression.

This produces a reference DLL/test, never adopts a runner or touches a prefix.
Wine source and binaries stay outside the proprietary bridge build.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request

WINE = "46b29104e3741fe23bf5e2547196a253aab88c89"
ARCHIVE_SHA = "7962bca6cb05628a56cc03008ea7af6cee7245f5b98a828452465e2775e18340"
SDK = "registry.gitlab.steamos.cloud/proton/steamrt4/sdk/x86_64@sha256:6c1789cad862fd8ed9d46d71bbf8752bb7cb2336ab5e9680c0f0e3e7c88d77f1"
PATCH_SHA = "9c1a38d9d7363be02e9e03159ba3fcfa6e2bc266c5dd5ce42d4439c12842da8d"
GUARD_SHA = "89d3f94d3f508f04192242d41512c010ea77e9c7d3324ac1f1af0df0fb42c025"
SCRIPT = """set -eu
export XDG_CACHE_HOME=/work/cache
cd /work/wine
dlls/winevulkan/make_vulkan -x vk.xml -X video.xml > /work/vulkan-generation.log 2>&1
tools/make_specfiles > /work/spec-generation.log 2>&1
tools/make_requests > /work/request-generation.log 2>&1
autoreconf -fi > /work/autoconf.log 2>&1
mkdir /work/obj
cd /work/obj
../wine/configure --enable-win64 --enable-archs=x86_64 --with-mingw=gcc --without-x --without-wayland > /work/configure.log 2>&1
make -j2 dlls/uiautomationcore/x86_64-windows/uiautomationcore.dll dlls/uiautomationcore/tests/x86_64-windows/uiautomationcore_test.exe > /work/make.log 2>&1
"""


def sha(path):
    h = hashlib.sha256()
    with path.open("rb") as f:
        while block := f.read(1024 * 1024):
            h.update(block)
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--wine-archive", type=Path, help="Reuse an exact verified source archive")
    args = parser.parse_args()
    root = args.output.resolve()
    root.mkdir(mode=0o700)
    patch = Path(__file__).with_name("uia-null-provider.patch")
    guard = Path(__file__).with_name("guard_provider.c")
    if sha(patch) != PATCH_SHA or sha(guard) != GUARD_SHA:
        raise ValueError("reference patch identity changed")
    archive = root / "wine.tar.gz"
    if args.wine_archive:
        if sha(args.wine_archive) != ARCHIVE_SHA:
            raise ValueError("reference source changed")
        shutil.copyfile(args.wine_archive, archive)
    else:
        with urllib.request.urlopen("https://codeload.github.com/ValveSoftware/wine/tar.gz/" + WINE,
                                    timeout=60) as response, archive.open("xb") as out:
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
    subprocess.run(["patch", "--batch", "--fuzz=0", "-p1", "-i", str(patch)],
                   cwd=source, check=True, timeout=30)
    shutil.copyfile(guard, source / "dlls/uiautomationcore/tests/guard_provider.c")
    source_inputs = {name: sha(source / name) for name in [
        "dlls/uiautomationcore/uia_client.c", "dlls/uiautomationcore/tests/Makefile.in",
        "dlls/uiautomationcore/tests/guard_provider.c"]}
    for path in [Path(__file__), patch, guard]:
        shutil.copyfile(path, root / path.name)
    script = root / "build.sh"
    script.write_text(SCRIPT)
    script_sha = sha(script)
    # One capped developer builder. Never run the old unbounded -j4 recipe.
    subprocess.run(["docker", "run", "--rm", "--name", "lvb-uia-builder",
                    "--cpus", "2", "--cpuset-cpus", "2,3", "--memory", "4g",
                    "--memory-swap", "4g", "--pids-limit", "256", "--network", "none",
                    "--user", f"{os.getuid()}:{os.getgid()}",
                    "--env", "XDG_CACHE_HOME=/work/cache",
                    "--entrypoint", "/bin/sh", "--volume",
                    str(root) + ":/work", SDK, "/work/build.sh"], check=True, timeout=1800)
    if sha(script) != script_sha or any(sha(source / name) != digest
                                      for name, digest in source_inputs.items()):
        raise ValueError("consumed source or build recipe changed during compilation")
    artifacts = {name: sha(root / relative) for name, relative in {
        "uiautomationcore.dll": "obj/dlls/uiautomationcore/x86_64-windows/uiautomationcore.dll",
        "uiautomationcore_test.exe": "obj/dlls/uiautomationcore/tests/x86_64-windows/uiautomationcore_test.exe",
    }.items()}
    receipt = {"schema": 1, "kind": "wine_uia_null_provider_reference_build",
               "wine_commit": WINE, "wine_archive_sha256": ARCHIVE_SHA, "sdk_image": SDK,
               "patch_sha256": PATCH_SHA, "license": "LGPL-2.1-or-later",
               "guard_source_sha256": GUARD_SHA, "architecture": "x86_64",
               "source_inputs": source_inputs, "build_script_sha256": script_sha,
               "builder_sha256": sha(root / "build.py"),
               "artifacts": artifacts, "adopts_runtime": False, "vendor_test_passed": False}
    (root / "build-receipt.json").write_text(json.dumps(receipt, sort_keys=True, indent=2) + "\n")
    print(json.dumps(receipt, sort_keys=True))


if __name__ == "__main__":
    main()

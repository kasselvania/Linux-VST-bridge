#!/usr/bin/env python3
"""Disposable-container package/dependency install fixture, never product proof.

The packaged executables here are inert test bytes. This proves that dpkg/apt
accept the exact archive and dependency metadata and that removal leaves
unrelated user-owned data untouched. It does not start the bridge or a DAW.
"""

import json
import os
from pathlib import Path
import subprocess

from test_deb import DebianPackage
import assemble

FIXTURE_PYTHON_MAGIC = {
    ("ubuntu", "26.04"): bytes.fromhex("2b0e0d0a"),
    ("debian", "13"): bytes.fromhex("f30d0d0a"),
}


def run(*args):
    result = subprocess.run(args, timeout=900, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True)
    if result.returncode:
        raise ValueError("package-manager fixture failed:\n" +
                         "\n".join(result.stdout.splitlines()[-20:]))


def main():
    if (os.geteuid() != 0 or os.environ.get("LVB_DISPOSABLE_CONTAINER") != "1"
            or not Path("/.dockerenv").exists()):
        raise ValueError("disposable root-owned container required")
    release = {}
    for line in Path("/etc/os-release").read_text().splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            release[key] = value.strip('"')
    target = (release.get("ID"), release.get("VERSION_ID"))
    if (target not in FIXTURE_PYTHON_MAGIC
            or subprocess.check_output(["dpkg", "--print-architecture"], text=True).strip() != "amd64"):
        raise ValueError("fixture distribution or architecture differs")
    fixture = DebianPackage()
    fixture.setUp()
    # The inert package still carries exact target ABI metadata. Build its two
    # supervisor headers for this declared fixture rather than inheriting the
    # Debian unit test's 3.13 default on Ubuntu 26.04.
    magic = FIXTURE_PYTHON_MAGIC[target]
    fixture.staged = fixture.stage_with_python_magics(magic, magic, flags=3)
    identity = fixture.build()
    retained = Path("/root/.local/share/linux-vst-bridge/retained-fixture")
    retained.parent.mkdir(parents=True, exist_ok=True)
    retained.write_text("user-owned sentinel\n")
    run("apt-get", "install", "-y", "--no-install-recommends", str(fixture.package))
    installed = subprocess.check_output([
        "dpkg-query", "-W", "-f=${Status} ${Version}", "linux-vst-bridge-beta"], text=True).strip()
    if installed != "install ok installed 0.1.0beta1-1":
        raise ValueError("package installation state")
    desktop = Path("/usr/share/applications/linux-audio-compatibility-manager.desktop")
    if (desktop.read_bytes() != assemble.SYSTEM_DESKTOP_BYTES
            or desktop.stat().st_uid != 0 or desktop.stat().st_gid != 0):
        raise ValueError("system desktop identity")
    if any(Path("/var/lib/dpkg/info/linux-vst-bridge-beta." + suffix).exists()
           for suffix in ("preinst", "postinst", "prerm", "postrm")):
        raise ValueError("package maintainer script appeared")
    run("apt-get", "remove", "-y", "linux-vst-bridge-beta")
    if desktop.exists() or retained.read_text() != "user-owned sentinel\n":
        raise ValueError("package removal touched retained user state")
    print(json.dumps({"schema": 1, "fixture": "synthetic_package_install_only",
                      "distribution": release["ID"], "version": release["VERSION"],
                      "architecture": "amd64", "package_version": "0.1.0beta1-1",
                      "supervisor_python_magic": magic.hex(),
                      "package_sha256": identity["package_sha256"],
                      "dependencies_resolved": True, "root_owned_desktop": True,
                      "install_hooks": False, "unrelated_user_file_preserved": True,
                      "product_startup": False, "audio": False}, sort_keys=True))
    fixture.doCleanups()


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Exercise fixed /usr intake with real root-owned files as an ordinary user."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FILES = {
    "linux-vst-bridge": "bin/linux-vst-bridge",
    "linux-audio-compatibility-manager": "bin/linux-audio-compatibility-manager",
    "session.pyc": "lib/linux-vst-bridge/supervisor/session.pyc",
    "ownership.pyc": "lib/linux-vst-bridge/supervisor/ownership.pyc",
    "host.exe": "lib/linux-vst-bridge/host/bridge-host.exe",
    "host-source-manifest.json": "lib/linux-vst-bridge/host/source-manifest.json",
}


def run(*args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def fixture(base, case):
    usr = base / case / "usr"
    roster = []
    for name, relative in FILES.items():
        path = usr / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        data = name.encode()
        path.write_bytes(data)
        path.chmod(0o444)
        roster.append({"name": name, "sha256": hashlib.sha256(data).hexdigest(),
                       "size": len(data)})
    manifest = usr / "share/linux-vst-bridge/pkg0-manifest.json"
    manifest.parent.mkdir(parents=True)
    manifest.write_text(json.dumps({
        "schema": 1, "package": "linux-vst-bridge-beta", "version": "0.1.0beta1",
        "source_head": "a" * 40, "source_tree": "b" * 40,
        "operator_schema": 10, "files": roster,
        "external_runtime": {"id": "exact-proton-slr", "manifest_sha256": "c" * 64},
    }, sort_keys=True) + "\n")
    manifest.chmod(0o444)
    if case == "symlink":
        retained = manifest.with_name("manifest.actual.json")
        manifest.rename(retained)
        manifest.symlink_to(retained.name)
    elif case == "writable":
        manifest.chmod(0o666)
    elif case == "changed_bytes":
        artifact = usr / FILES["host.exe"]
        artifact.chmod(0o644)
        artifact.write_bytes(b"changed host")
        artifact.chmod(0o444)
    elif case == "oversize":
        manifest.chmod(0o644)
        manifest.write_bytes(manifest.read_bytes() + b" " * 65536)
        manifest.chmod(0o444)
    run("sudo", "chown", "-R", "root:root", str(usr))
    if case == "changed_owner":
        run("sudo", "chown", f"{os.getuid()}:{os.getgid()}", str(manifest))
    return usr


def main():
    if sys.platform != "linux" or os.geteuid() == 0:
        raise SystemExit("run as an ordinary Linux user with sudo fixture setup")
    with tempfile.TemporaryDirectory(prefix="pkg0-root-intake-") as temporary:
        base = Path(temporary)
        for case in ("valid", "changed_owner", "symlink", "writable", "changed_bytes", "oversize"):
            usr = fixture(base, case)
            try:
                env = dict(os.environ, PKG0_ROOT_INPUT_FIXTURE=str(usr),
                           PKG0_ROOT_EXPECT="ok" if case == "valid" else "reject")
                run("cargo", "test", "--manifest-path", str(ROOT / "bridge-manager/Cargo.toml"),
                    "--locked", "--bin", "linux-vst-bridge", "--",
                    "--ignored", "--exact", "package_authority::tests::root_owned_package_intake",
                    cwd=ROOT, env=env, stdout=subprocess.DEVNULL)
                print(f"root-owned package intake {case}: passed")
            finally:
                run("sudo", "chown", "-R", f"{os.getuid()}:{os.getgid()}", str(usr))


if __name__ == "__main__":
    main()

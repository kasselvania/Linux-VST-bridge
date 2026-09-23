#!/usr/bin/python3.13
"""Private Pi comparison adapter; copy as compare-ge.py and compare-box64.py.

Both variants keep a native Python leader for the existing pinned-launcher
supervisor. The filename selects the runtime; it does not describe guest ISA.
Only the Box64 variant's --prepare creates its account-free prefix.
"""
import hashlib
import os
from pathlib import Path
import subprocess
import sys


def check(path, digest):
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise RuntimeError(f"runtime digest mismatch: {path.name}")


def main():
    root = Path(__file__).resolve().parent
    lane = Path(sys.argv[0]).stem
    env = dict(os.environ)
    env.update(LANG="C.UTF-8", PROTON_USE_XALIA="0", WINEDLLOVERRIDES="uiautomationcore=")
    arguments = sys.argv[1:]
    if lane == "compare-ge":
        check(root / "launch-host.sh", "642d846eb8054d1597810a2c7af17a69ce2843e3263dd46007f055ff62378ced")
        return subprocess.call([str(root / "launch-host.sh"), *arguments], env=env)
    if lane != "compare-box64":
        raise RuntimeError("copy fixture adapter under an explicit comparison lane name")

    runtime = root.parent / "rpi1-private"
    pins = {
        "emulator/bin/box64": "79cdd30e5480f5dfb8cd717af99d0e5a89eb55b31701de6fde7896a6a0e79ab6",
        "emulator/adapter": "99e1dc907c53fba755c1839922654753c09435fa3c474ded00f8dc2d499b7828",
        "emulator/emulator.json": "9a30bd4f6cd6ec7e888a90af81f26aa9c3967abc9410e3535dea7a804f18e7c1",
        "emulator/graphics-provider.json": "a076eea9d36c398b94b9a9b1f093d31811659a731169a42bf9ad6bae54bf871d",
        "runtime/SteamLinuxRuntime_4/_v2-entry-point": "caa39b5cde8ea955288b574b49b3416fd16e7be3f53a17249d673f5ecfdce2c1",
        "runtime/Proton 11.0/proton": "787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad",
    }
    for name, digest in pins.items():
        check(runtime / name, digest)
    private = root / "box64-reference"
    prefix = private / "compatdata/pfx"
    prepare = arguments == ["--prepare"]
    if prepare:
        for name in ("compatdata", "cache", "config", "data", "tmp", "runtime-var", "steam-root"):
            (private / name).mkdir(parents=True, exist_ok=True, mode=0o700)
    elif env.get("WINEPREFIX") != str(prefix):
        raise RuntimeError("Box64 comparison requires its separate reference prefix")

    env.update({
        "WINEPREFIX": str(prefix),
        "DISPLAY": ":1", "XAUTHORITY": str(Path.home() / ".Xauthority"),
        "PYTHONHOME": "/usr", "STEAM_ZENITY": "",
        "SteamAppId": "0", "SteamGameId": "0", "STEAM_COMPAT_APP_ID": "0",
        "STEAM_COMPAT_MACHINE_ARCHITECTURE": "aarch64-linux-gnu",
        "XDG_CACHE_HOME": str(private / "cache"), "XDG_CONFIG_HOME": str(private / "config"),
        "XDG_DATA_HOME": str(private / "data"), "TMPDIR": str(private / "tmp"),
        "STEAM_COMPAT_CLIENT_INSTALL_PATH": str(private / "steam-root"),
        "STEAM_COMPAT_DATA_PATH": str(private / "compatdata"),
        "STEAM_COMPAT_INSTALL_PATH": str(prefix / "drive_c/bridge/fixtures"),
        "STEAM_COMPAT_EMULATOR": str(runtime / "emulator/emulator.json"),
        "STEAM_COMPAT_GRAPHICS_PROVIDER": str(runtime / "emulator/graphics-provider.json"),
        "PRESSURE_VESSEL_FILESYSTEMS_RO": str(Path.home() / ".Xauthority"),
        "PRESSURE_VESSEL_FILESYSTEMS_RW": str(private),
        "PRESSURE_VESSEL_VARIABLE_DIR": str(private / "runtime-var"),
    })
    if prepare:
        arguments = ["getcompatpath", "/"]
    else:
        if not arguments or not Path(arguments[0]).is_relative_to(prefix / "drive_c"):
            raise RuntimeError("host must be inside the comparison prefix")
        arguments = ["runinprefix", *arguments]
    return subprocess.call([
        str(runtime / "runtime/SteamLinuxRuntime_4/_v2-entry-point"), "--verb=run", "--",
        str(runtime / "runtime/Proton 11.0/proton"), *arguments,
    ], env=env)


if __name__ == "__main__":
    sys.exit(main())

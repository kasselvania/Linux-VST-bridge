"""Private, temporary KDE sharing process; invoked over SSH by start.py."""
import configparser
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import sys
import tarfile
import tempfile
import urllib.request


def checked_work(value):
    work = Path(value)
    runtime = Path(f"/run/user/{os.getuid()}")
    if (work.parent != runtime or not work.name.startswith("lpb-deck-vnc-")
            or work.is_symlink() or work.stat().st_uid != os.getuid()
            or (work / "owner").read_text() != "Linux Plugin Bridge browser helper\n"):
        raise RuntimeError("Unexpected temporary sharing directory")
    return work


def unit(work):
    return work.name + ".service"


def prepare():
    work = Path(tempfile.mkdtemp(prefix="lpb-deck-vnc-", dir=f"/run/user/{os.getuid()}"))
    try:
        (work / "owner").write_text("Linux Plugin Bridge browser helper\n")
        for name in ("config", "state", "data", "root"):
            (work / name).mkdir(mode=0o700)
        dbpath = subprocess.check_output(["pacman-conf", "DBPath"], text=True).strip()
        dbpath = Path(dbpath.split("=", 1)[-1].strip()) / "sync"
        packages = subprocess.check_output(
            ["pacman", "-Sp", "--print-format", "%n %l", "krfb", "libvncserver"], text=True)
        for line in packages.splitlines():
            name, url = line.split(" ", 1)
            filename = url.rsplit("/", 1)[-1]
            expected = None
            for database in dbpath.glob("*.db"):
                with tarfile.open(database) as archive:
                    for member in archive:
                        if member.name.startswith(name + "-") and member.name.endswith("/desc"):
                            fields = archive.extractfile(member).read().decode().split("\n\n")
                            fields = {f.split("\n", 1)[0].strip("%"):
                                      f.split("\n", 1)[-1].strip() for f in fields if f}
                            if fields.get("FILENAME") == filename:
                                expected = fields.get("SHA256SUM")
            package = work / filename
            with urllib.request.urlopen(url, timeout=30) as response, package.open("wb") as out:
                shutil.copyfileobj(response, out)
            if not expected or hashlib.sha256(package.read_bytes()).hexdigest() != expected:
                raise RuntimeError("Package differs from the installed distro package database")
            subprocess.run(["tar", "-xf", str(package), "-C", str(work / "root")],
                           check=True, capture_output=True)
        return {"work": str(work)}
    except BaseException:
        shutil.rmtree(work)
        raise


def start(settings):
    work = checked_work(settings["work"])
    with socket.socket() as probe:
        probe.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        probe.bind(("127.0.0.1", settings["port"]))
    config = configparser.ConfigParser()
    config.optionxform = str
    config["MainWindow"] = {"startMinimized": "true"}
    config["TCP"] = {"useDefaultPort": "false", "port": str(settings["port"]), "publishService": "false"}
    # KDE's config scrambling is only a storage format. The directory is private.
    obscured = "".join(c if ord(c) <= 0x21 else chr(0x1001F - ord(c)) for c in settings["password"])
    config["Security"] = {"noWallet": "true", "allowDesktopControl": "true",
                          "allowUnattendedAccess": "true", "desktopPassword": obscured,
                          "unattendedPassword": obscured}
    config["FrameBuffer"] = {"preferredFrameBufferPlugin": "pw"}
    with (work / "config/krfbrc").open("w") as out:
        config.write(out)
    (work / "config/krfbrc").chmod(0o600)
    environment = dict(line.split("=", 1) for line in subprocess.check_output(
        ["systemctl", "--user", "show-environment"], text=True).splitlines() if "=" in line)
    if environment.get("XDG_SESSION_TYPE") != "wayland":
        raise RuntimeError("This helper requires a running KDE Wayland desktop")
    env = {key: environment[key] for key in ("DISPLAY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR",
           "DBUS_SESSION_BUS_ADDRESS") if key in environment}
    env.update(XDG_CONFIG_HOME=str(work / "config"), XDG_STATE_HOME=str(work / "state"),
               XDG_DATA_HOME=str(work / "data"), LD_LIBRARY_PATH=str(work / "root/usr/lib"),
               QT_PLUGIN_PATH=str(work / "root/usr/lib/qt6/plugins"), LD_PRELOAD=str(work / "loopback.so"))
    cmd = ["systemd-run", "--user", "--quiet", "--collect",
           "--description=Temporary browser desktop sharing", "--unit=" + unit(work),
           "--property=RuntimeMaxSec=" + str(settings["seconds"]),
           "--property=StandardOutput=file:" + str(work / "server.log"),
           "--property=StandardError=file:" + str(work / "server.log")]
    for key, value in env.items():
        cmd += ["--setenv=" + key + "=" + value]
    result = subprocess.run(cmd + [str(work / "root/usr/bin/krfb"), "--nodialog"], capture_output=True)
    if result.returncode:
        raise RuntimeError("Temporary sharing service could not start")
    return {"started": True}


def stop(settings):
    work = checked_work(settings["work"])
    subprocess.run(["systemctl", "--user", "stop", unit(work)], capture_output=True)
    active = subprocess.run(["systemctl", "--user", "is-active", "--quiet", unit(work)], capture_output=True)
    if active.returncode not in (3, 4):
        raise RuntimeError("Sharing service stop could not be confirmed; temporary files were preserved")
    shutil.rmtree(work)
    return {"stopped": True}


if __name__ == "__main__":
    try:
        operation = sys.argv[1]
        result = prepare() if operation == "prepare" else {"start": start, "stop": stop}[operation](json.load(sys.stdin))
        print(json.dumps(result))
    except Exception as error:
        # Never print argv or a password-bearing settings dictionary.
        print(str(error), file=sys.stderr)
        raise SystemExit(1)

#!/usr/bin/env python3
"""Run the retained CachyOS fixture exclusively; never force or restore a VM."""
import fcntl
import json
import os
from pathlib import Path
import signal
import socket
import stat
import subprocess
import time

BASE = Path("/mnt/Storage2/LinuxPluginBridge")
VM = "lvb430-test205-cachyos"
QMP = BASE / "runtime-430/fresh-cachyos/qmp.sock"
GIB = 1024**3
LEASE_SECONDS = 25 * 60


def emit(message):
    try:
        print(message, flush=True)
    except OSError:
        pass  # Losing a log reader must never release ownership of a live VM.


def run(*args):
    return subprocess.check_output(args, text=True, timeout=15).strip()


def inspect(name=VM):
    return json.loads(run("docker", "inspect", name))[0]


def memory_available():
    rows = Path("/proc/meminfo").read_text().splitlines()
    return int(next(row.split()[1] for row in rows if row.startswith("MemAvailable:"))) * 1024


def pressure():
    return {name: Path(f"/proc/pressure/{name}").read_text().strip()
            for name in ("cpu", "memory", "io")}


def require_capacity(config, available):
    if config["NanoCpus"] != 2_000_000_000:
        raise RuntimeError("fixture must be limited to two CPUs")
    if config["Memory"] != 5 * GIB or config["MemorySwap"] != 5 * GIB:
        raise RuntimeError("fixture must have a 5 GiB ceiling with no extra swap")
    if config["RestartPolicy"]["Name"] != "no":
        raise RuntimeError("automatic fixture restart is forbidden")
    if available < config["Memory"] + 4 * GIB:
        raise RuntimeError("insufficient host memory reserve; leave fixture stopped")


def require_idle():
    ids = run("docker", "ps", "-q").split()
    for item in json.loads(run("docker", "inspect", *ids)) if ids else []:
        project_mount = any(m["Source"] == str(BASE) or m["Source"].startswith(str(BASE) + "/")
                            for m in item["Mounts"])
        if item["Name"].startswith("/lvb") or project_mount:
            raise RuntimeError(f"project container already running: {item['Name']}")
    qemu = subprocess.run(["pgrep", "-f", "[q]emu-system"], capture_output=True, timeout=5)
    if qemu.returncode != 1:
        raise RuntimeError("QEMU is running or process census failed")


def powerdown():
    with socket.socket(socket.AF_UNIX) as sock:
        sock.settimeout(5)
        sock.connect(str(QMP))
        with sock.makefile("rwb", buffering=0) as stream:
            json.loads(stream.readline())
            for seq, command in enumerate(("qmp_capabilities", "system_powerdown")):
                stream.write((json.dumps({"execute": command, "id": seq}) + "\n").encode())
                deadline = time.monotonic() + 5
                while time.monotonic() < deadline:
                    reply = json.loads(stream.readline())
                    if reply.get("id") == seq:
                        if "error" in reply:
                            raise RuntimeError(str(reply["error"]))
                        break
                else:
                    raise RuntimeError("QMP powerdown acknowledgement timed out")


def prepare_qmp():
    # QEMU creates this private socket as container root on every boot.
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        observed = None
        try:
            if stat.S_ISSOCK(QMP.lstat().st_mode):
                observed = QMP.lstat()
                try:
                    run("docker", "exec", VM, "chown", "--no-dereference",
                        f"{os.getuid()}:{os.getgid()}", "/vm/qmp.sock")
                except subprocess.CalledProcessError:
                    current = QMP.lstat()
                    if (current.st_dev, current.st_ino) == (observed.st_dev, observed.st_ino):
                        raise  # A persistent ownership failure needs attention.
                    continue
                with socket.socket(socket.AF_UNIX) as sock:
                    sock.settimeout(1)
                    sock.connect(str(QMP))
                    with sock.makefile("rb") as stream:
                        if "QMP" in json.loads(stream.readline()):
                            return
        except PermissionError:
            if observed is None:
                raise
            try:
                current = QMP.lstat()
            except FileNotFoundError:
                continue
            if (current.st_dev, current.st_ino) == (observed.st_dev, observed.st_ino):
                raise
        except (FileNotFoundError, ConnectionRefusedError, TimeoutError, ValueError):
            pass  # A stale socket from the preceding boot is not readiness.
        time.sleep(0.2)
    raise RuntimeError("fixture control socket did not appear")


def retire():
    """Keep both locks until positive shutdown, even if graceful shutdown fails."""
    requested = False
    last_notice = 0
    while True:
        try:
            if not inspect()["State"]["Running"]:
                emit("VM stopped; both workload locks can be released.")
                return
            if not requested:
                powerdown()
                requested = True
                emit("Graceful guest shutdown requested.")
        except Exception as error:
            if time.monotonic() - last_notice >= 60:
                emit(f"Shutdown needs attention; retaining both locks: {error}")
                last_notice = time.monotonic()
        if time.monotonic() - last_notice >= 60:
            emit("Waiting for normal shutdown; no forced stop and no replacement VM.")
            last_notice = time.monotonic()
        time.sleep(5)


def main():
    if run("findmnt", "-n", "-o", "TARGET", "--mountpoint", "/mnt/Storage2") != "/mnt/Storage2":
        raise RuntimeError("Storage2 is not mounted")
    locks = []
    try:
        for name in ("heavy-build.lock", "vm.lock"):
            fd = os.open(BASE / name, os.O_RDWR)  # Existing locks only; never replace.
            locks.append(fd)
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        require_idle()
        vm = inspect()
        if vm["State"]["Running"]:
            raise RuntimeError("fixture is already running")
        args = vm["Config"]["Cmd"]
        if args[args.index("-m") + 1] != "4096":
            raise RuntimeError("fixture guest memory must be 4 GiB")
        pairs = list(zip(args, args[1:]))
        if ("-drive", "file=/vm/test224-data.qcow2,if=none,format=qcow2,id=lvb-test224-blank") not in pairs or (
            "-device", "usb-storage,drive=lvb-test224-blank,id=lvbtest224disk,serial=LVB-TEST224-BLANK,bus=xhci.0"
        ) not in pairs:
            raise RuntimeError("retained test-account disk is not attached")
        available = memory_available()
        require_capacity(vm["HostConfig"], available)
        emit(json.dumps({"guard_pid": os.getpid(), "available_bytes": available,
                         "pressure": pressure(), "lease_seconds": LEASE_SECONDS}))
        deadline = time.monotonic() + LEASE_SECONDS
        stopping = False

        def stop(signum, frame):
            nonlocal stopping
            stopping = True

        def renew(signum, frame):
            nonlocal deadline
            if not stopping:
                deadline = time.monotonic() + LEASE_SECONDS
                emit("Attended lease renewed for 25 minutes.")

        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(sig, stop)
        signal.signal(signal.SIGUSR1, renew)
        try:
            run("docker", "start", VM)
            prepare_qmp()
            last_sample = 0
            while not stopping and time.monotonic() < deadline:
                if not inspect()["State"]["Running"]:
                    break
                available = memory_available()
                if time.monotonic() - last_sample >= 60:
                    emit(json.dumps({"available_bytes": available, "pressure": pressure()}))
                    last_sample = time.monotonic()
                if available < 4 * GIB:
                    emit("Host memory reserve crossed; ending fixture session.")
                    break
                time.sleep(5)
        finally:
            stopping = True
            retire()
    finally:
        for fd in reversed(locks):
            os.close(fd)


if __name__ == "__main__":
    main()

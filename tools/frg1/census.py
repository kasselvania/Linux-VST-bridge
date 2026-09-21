#!/usr/bin/env python3
"""Closed FRG1 factory-census supervisor.

The commercial module remains an ignored private input.  This program accepts only
the tracked FRG1 lock, the exact admitted host, and the exact UA1 runner receipts.
It runs from a private snapshot of the retained Ubuntu vendor environment in a
networkless Bubblewrap namespace and emits a bounded report.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import selectors
import secrets
import shutil
import signal
import stat
import struct
import subprocess
import sys
import time
from typing import Any, Iterable


SCHEMA = "linux-vst-bridge-frg1-input-lock/v1"
LOCK_SHA256 = "17cd4b147a1423329d00b6e11d3712030f8bf6392683e521f7e1c840dcf583fc"
OWNER_FILE = ".ua1-owner.json"
INSTALL_RECEIPT = ".ua1-install.json"
STDOUT_LIMIT = 1_048_576
STDERR_LIMIT = 4_194_304
X11_STDOUT_LIMIT = 65_536
X11_STDERR_LIMIT = 262_144
READY_TIMEOUT = 180.0
EXECUTION_TIMEOUT = 180.0
XWAYLAND_PATH = pathlib.Path("/usr/bin/Xwayland")
XWAYLAND_BYTE_LENGTH = 2_410_256
XWAYLAND_SHA256 = "700d3cba7f946c3ff3100ee36f15012c1c027bb67f29e84def1a1b67bb8f3587"
WAYLAND_SOCKET_NAME = "wayland-0"


class CensusError(RuntimeError):
    pass


class PrivateX11:
    def __init__(
        self,
        process: subprocess.Popen[bytes],
        drain: "BoundedDrain",
        display_number: int,
        socket: pathlib.Path,
        authority: pathlib.Path,
    ) -> None:
        self.process = process
        self.drain = drain
        self.display_number = display_number
        self.socket = socket
        self.authority = authority


class BoundedDrain:
    """Continuously drain both child pipes without allowing unbounded custody."""

    def __init__(
        self,
        child: subprocess.Popen[bytes],
        *,
        stdout_limit: int,
        stderr_limit: int,
        label: str,
    ) -> None:
        if child.stdout is None or child.stderr is None:
            raise CensusError(f"{label} pipes unavailable")
        self.child = child
        self.label = label
        self.selector = selectors.DefaultSelector()
        self.buffers = {"stdout": bytearray(), "stderr": bytearray()}
        self.limits = {"stdout": stdout_limit, "stderr": stderr_limit}
        self.exhausted: set[str] = set()
        self.closed = False
        for name, stream in (("stdout", child.stdout), ("stderr", child.stderr)):
            os.set_blocking(stream.fileno(), False)
            self.selector.register(stream, selectors.EVENT_READ, name)

    def pump(self, timeout: float, *, enforce_capacity: bool = True) -> None:
        for key, _ in self.selector.select(timeout):
            name = str(key.data)
            while True:
                try:
                    chunk = os.read(key.fileobj.fileno(), 65_536)
                except BlockingIOError:
                    break
                if not chunk:
                    self.selector.unregister(key.fileobj)
                    break
                remaining = self.limits[name] - len(self.buffers[name])
                if remaining > 0:
                    self.buffers[name].extend(chunk[:remaining])
                if len(chunk) > remaining:
                    self.exhausted.add(name)
                    break
        if enforce_capacity and self.exhausted:
            names = ",".join(sorted(self.exhausted))
            raise CensusError(f"{self.label} stream capacity exhausted: {names}")

    def finish(self, timeout: float = 5.0) -> None:
        if self.closed:
            return
        deadline = time.monotonic() + timeout
        while self.selector.get_map():
            self.pump(0.05, enforce_capacity=False)
            if time.monotonic() >= deadline:
                raise CensusError(f"{self.label} pipe retirement timeout")
        self.selector.close()
        if self.child.stdout is not None:
            self.child.stdout.close()
        if self.child.stderr is not None:
            self.child.stderr.close()
        self.closed = True

    def output(self) -> tuple[bytes, bytes]:
        return bytes(self.buffers["stdout"]), bytes(self.buffers["stderr"])


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path: pathlib.Path, limit: int = 128 * 1024) -> Any:
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit:
        raise CensusError(f"invalid JSON file: {path.name}")
    return json.loads(path.read_text(encoding="utf-8"))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise CensusError(message)


def exact_directory(path: pathlib.Path) -> pathlib.Path:
    require(path.is_absolute() and not path.is_symlink() and path.is_dir(), "directory unavailable")
    require(path.resolve(strict=True) == path, "directory path is aliased")
    require(path.stat().st_uid == os.getuid(), "directory is not owned by this user")
    return path


def exact_file(path: pathlib.Path, byte_length: int, sha256: str) -> None:
    require(path.is_file() and not path.is_symlink(), f"missing regular input: {path.name}")
    require(path.stat().st_size == byte_length, f"input size changed: {path.name}")
    require(sha256_file(path) == sha256, f"input digest changed: {path.name}")


def xauthority_record(display_number: int, cookie: bytes) -> bytes:
    require(0 <= display_number <= 65535, "private X11 display number invalid")
    require(len(cookie) == 16, "private X11 cookie invalid")

    def field(value: bytes) -> bytes:
        return struct.pack(">H", len(value)) + value

    # FamilyWild keeps the record independent of a host name while the exact
    # display number and operation-random cookie remain closed.
    return (
        struct.pack(">H", 65535)
        + field(b"")
        + field(str(display_number).encode("ascii"))
        + field(b"MIT-MAGIC-COOKIE-1")
        + field(cookie)
    )


def start_private_x11(root: pathlib.Path) -> PrivateX11:
    exact_file(XWAYLAND_PATH, XWAYLAND_BYTE_LENGTH, XWAYLAND_SHA256)
    uid = os.getuid()
    runtime_root = pathlib.Path(f"/run/user/{uid}")
    wayland_socket = runtime_root / WAYLAND_SOCKET_NAME
    info = wayland_socket.lstat()
    require(stat.S_ISSOCK(info.st_mode), "exact Wayland session socket unavailable")
    require(info.st_uid == uid, "Wayland session socket owner changed")
    require(runtime_root.is_dir() and runtime_root.stat().st_uid == uid, "runtime root owner changed")

    x11_root = root / "x11"
    x11_root.mkdir(parents=True, mode=0o700)
    home = x11_root / "home"
    home.mkdir(mode=0o700)
    authority = x11_root / "Xauthority"
    authority.write_bytes(b"")
    authority.chmod(0o600)
    candidates = list(range(120, 220))
    secrets.SystemRandom().shuffle(candidates)
    last_failure = "no private display candidate"
    for display_number in candidates:
        socket = pathlib.Path(f"/tmp/.X11-unix/X{display_number}")
        lock_path = pathlib.Path(f"/tmp/.X{display_number}-lock")
        if socket.exists() or lock_path.exists() or socket.is_symlink() or lock_path.is_symlink():
            continue
        authority.write_bytes(xauthority_record(display_number, secrets.token_bytes(16)))
        authority.chmod(0o600)
        environment = {
            "HOME": str(home),
            "XDG_RUNTIME_DIR": str(runtime_root),
            "WAYLAND_DISPLAY": WAYLAND_SOCKET_NAME,
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
        }
        process = subprocess.Popen(
            [
                str(XWAYLAND_PATH),
                f":{display_number}",
                "-rootless",
                "-noreset",
                "-nolisten",
                "tcp",
                "-auth",
                str(authority),
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=environment,
            start_new_session=True,
        )
        drain = BoundedDrain(
            process,
            stdout_limit=X11_STDOUT_LIMIT,
            stderr_limit=X11_STDERR_LIMIT,
            label="private Xwayland",
        )
        deadline = time.monotonic() + 10.0
        try:
            while True:
                drain.pump(0.05)
                if socket.exists():
                    socket_info = socket.lstat()
                    require(stat.S_ISSOCK(socket_info.st_mode), "private X11 endpoint is not a socket")
                    require(socket_info.st_uid == uid, "private X11 socket owner changed")
                    return PrivateX11(process, drain, display_number, socket, authority)
                if process.poll() is not None:
                    drain.finish()
                    output, error = drain.output()
                    last_failure = (
                        "private Xwayland exited before socket publication; "
                        f"stdout_sha256={sha256_bytes(output)}; stderr_sha256={sha256_bytes(error)}"
                    )
                    break
                if time.monotonic() >= deadline:
                    last_failure = "private Xwayland socket publication timeout"
                    break
        except Exception:
            terminate(process)
            drain.finish()
            raise
        terminate(process)
        drain.finish()
    raise CensusError(last_failure)


def within(path: pathlib.Path, root: pathlib.Path) -> bool:
    try:
        path.resolve(strict=False).relative_to(root.resolve(strict=False))
        return True
    except ValueError:
        return False


def tree_identity(root: pathlib.Path, *, exclude: Iterable[str] = ()) -> dict[str, Any]:
    excluded = set(exclude)
    roster: list[dict[str, Any]] = []
    paths = sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix())
    for path in paths:
        relative = path.relative_to(root).as_posix()
        if relative in excluded:
            continue
        info = path.lstat()
        base = {"path": relative, "mode": stat.S_IMODE(info.st_mode)}
        if stat.S_ISDIR(info.st_mode):
            roster.append({**base, "type": "directory"})
        elif stat.S_ISREG(info.st_mode):
            roster.append(
                {
                    **base,
                    "type": "file",
                    "byte_length": info.st_size,
                    "sha256": sha256_file(path),
                }
            )
        elif stat.S_ISLNK(info.st_mode):
            target = os.readlink(path)
            require(not pathlib.PurePosixPath(target).is_absolute(), "installed tree has absolute symlink")
            require(within(path.parent / target, root), "installed tree has escaping symlink")
            roster.append({**base, "type": "symlink", "target": target})
        else:
            raise CensusError("installed tree contains special file")
    # UA1 receipts deliberately hash their newline-terminated canonical JSON.
    return {"entries": len(roster), "sha256": sha256_bytes(canonical_json(roster) + b"\n")}


def load_lock(path: pathlib.Path) -> dict[str, Any]:
    value = read_json(path)
    require(sha256_bytes(canonical_json(value)) == LOCK_SHA256, "FRG1 lock changed")
    require(value.get("schema") == SCHEMA, "FRG1 lock schema changed")
    require(value["fixture"]["module"]["sha256"] == "5846dfe91396596715f01a85d51c5ca21a808ef7dad1bfb44b02ab444345e1a5", "module lock changed")
    require(value["windows_host"]["artifact"]["sha256"] == "348a4bbc6ea34f57fc5899c279d9e43999bf9ecae4563a6cfee88967d36f66be", "host lock changed")
    require(value["windows_host"]["source_manifest"]["sha256"] == "ad7f860633f871fbe0d76cb3c546e8f7f33e8291ac46844fbd088806ba53814c", "host source lock changed")
    require(value["runner"]["archive"]["sha256"] == "c5448b76a230384e2d7bc6beb5ccb97bafb7e2c3b6c527cb03a1a546bbcb00a0", "runner lock changed")
    predecessor = value["historical_profile"]
    expected = value["successor_verification"]
    require(predecessor["profile_file_sha256"] == "bb07dd980c89861dfb5da96a094e3603f57e4231b1d240d34f6ed9bd5c3ced3b", "predecessor profile changed")
    require(predecessor["class"]["class_id"] == "41727475415649536772616E50726F63", "predecessor class changed")
    require(predecessor["parameter_count"] == 2348, "predecessor parameter count changed")
    require(expected["expected_class"]["class_id"] == predecessor["class"]["class_id"], "successor class continuity changed")
    require(expected["required_class_continuity"] is True, "successor class continuity disabled")
    retained = value["retained_ubuntu_environment"]
    require(retained["custody"]["owner_identity"] == "8fdea6abe1be5a42cf7db1f61006d24c84109e3ad6acdf9421f01c7797bc9af5", "retained prefix owner changed")
    require(retained["custody"]["source_lock_sha256"] == "2f7772e9779fb4a44ccbef14ad8026f22ecca45b2ca45428c675328c897d73ee", "retained prefix source lock changed")
    require(retained["execution"]["launcher_verb"] == "runinprefix", "retained prefix launch verb changed")
    require(retained["execution"]["component_case"] == "class:" + predecessor["class"]["class_id"], "retained prefix class selection changed")
    require(retained["execution"]["network_shared"] is False, "retained prefix network policy changed")
    return value


def _home_relative(home: pathlib.Path, relative: str) -> pathlib.Path:
    value = pathlib.PurePosixPath(relative)
    require(not value.is_absolute() and ".." not in value.parts, "retained custody path is not home-relative")
    return home / value


def _validate_prefix_tree(root: pathlib.Path, lock: dict[str, Any]) -> dict[str, Any]:
    exact_directory(root)
    retained = lock["retained_ubuntu_environment"]
    require(set(item.name for item in root.iterdir()) == set(retained["top_level_roster"]), "retained prefix top-level roster changed")
    owner_path = root / OWNER_FILE
    owner = read_json(owner_path)
    expected_owner = {
        "identity": retained["custody"]["owner_identity"],
        "kind": "asc-prefix",
        "owner": "kasselvania/Linux-VST-bridge-ubuntu-lab",
        "schema": "linux-vst-bridge-ubuntu-lab-ua1-owner/v1",
        "slice": "UA1",
    }
    require(owner == expected_owner, "retained prefix owner receipt changed")
    require(sha256_file(owner_path) == retained["custody"]["owner_file_sha256"], "retained prefix owner bytes changed")
    pfx = exact_directory(root / "pfx")
    pfx_mode = f"{stat.S_IMODE(pfx.stat().st_mode):04o}"
    require(pfx_mode == retained["custody"]["pfx_mode"], "retained prefix pfx mode changed")
    module = root / retained["module_prefix_relative"]
    exact_file(module, lock["fixture"]["module"]["byte_length"], lock["fixture"]["module"]["sha256"])
    return {
        "owner_identity": owner["identity"],
        "owner_file_sha256": retained["custody"]["owner_file_sha256"],
        "module_sha256": lock["fixture"]["module"]["sha256"],
        "pfx_mode": pfx_mode,
    }


def validate_retained_prefix(
    root: pathlib.Path,
    source_lock: pathlib.Path,
    lock: dict[str, Any],
    *,
    home: pathlib.Path | None = None,
) -> dict[str, Any]:
    retained = lock["retained_ubuntu_environment"]
    custody = retained["custody"]
    actual_home = pathlib.Path.home().resolve(strict=True) if home is None else home.resolve(strict=True)
    require(root == _home_relative(actual_home, custody["prefix_home_relative"]), "retained prefix path changed")
    require(source_lock == _home_relative(actual_home, custody["source_lock_home_relative"]), "retained prefix source-lock path changed")
    exact_file(source_lock, custody["source_lock_byte_length"], custody["source_lock_sha256"])
    result = _validate_prefix_tree(root, lock)
    result["source_lock_sha256"] = custody["source_lock_sha256"]
    result["source_marker_preserved"] = True
    return result


def fsync_directory(path: pathlib.Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def copy_prefix_tree(source: pathlib.Path, stage: pathlib.Path) -> None:
    linux_cp = pathlib.Path("/usr/bin/cp")
    if linux_cp.is_file():
        copied = subprocess.run(
            [str(linux_cp), "-a", "--reflink=auto", "--", str(source) + "/.", str(stage)],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        require(copied.returncode == 0, "retained prefix snapshot failed")
        return
    # Local non-Linux validation has no GNU reflink flag. Physical FRG1 execution
    # is Ubuntu-only and takes the exact branch above.
    shutil.copytree(source, stage, symlinks=True, dirs_exist_ok=True, copy_function=shutil.copy2)


def snapshot_retained_prefix(source: pathlib.Path, state: pathlib.Path, lock: dict[str, Any]) -> tuple[pathlib.Path, dict[str, Any]]:
    require(not state.exists() and not state.is_symlink(), "private snapshot state already exists")
    state.mkdir(mode=0o700)
    stage = state / ".prefix-stage"
    destination = state / "prefix"
    stage.mkdir(mode=0o700)
    source_before = _validate_prefix_tree(source, lock)
    try:
        copy_prefix_tree(source, stage)
        snapshot = _validate_prefix_tree(stage, lock)
        require(snapshot == source_before, "retained prefix snapshot identity changed")
        require(not destination.exists() and not destination.is_symlink(), "private snapshot destination collision")
        stage.rename(destination)
        fsync_directory(state)
        source_after = _validate_prefix_tree(source, lock)
        require(source_after == source_before, "retained prefix source changed during snapshot")
        return destination, {
            **source_before,
            "snapshot_policy": lock["retained_ubuntu_environment"]["execution"]["snapshot_policy"],
            "source_unchanged_after_snapshot": True,
        }
    except Exception:
        if stage.exists() and not stage.is_symlink():
            shutil.rmtree(stage)
        if state.exists() and not any(state.iterdir()):
            state.rmdir()
        raise


def exact_gpu_sysfs_argv(
    dri_root: pathlib.Path = pathlib.Path("/dev/dri"),
    sys_char_root: pathlib.Path = pathlib.Path("/sys/dev/char"),
    sys_devices_root: pathlib.Path = pathlib.Path("/sys/devices"),
) -> list[str]:
    devices = sorted(dri_root.glob("card[0-9]*")) + sorted(dri_root.glob("renderD[0-9]*"))
    require(
        bool(devices)
        and any(path.name.startswith("card") for path in devices)
        and any(path.name.startswith("renderD") for path in devices),
        "exact DRM card/render device roster is unavailable",
    )
    links: dict[str, str] = {}
    gpu_roots: set[pathlib.Path] = set()
    for device in devices:
        info = device.stat()
        char_name = f"{os.major(info.st_rdev)}:{os.minor(info.st_rdev)}"
        link = sys_char_root / char_name
        require(link.is_symlink(), "DRM sysfs character-device link is absent")
        target = os.readlink(link)
        resolved = link.resolve(strict=True)
        require(within(resolved, sys_devices_root), "DRM sysfs link leaves the devices boundary")
        gpu_root = (resolved / "device").resolve(strict=True)
        require(within(gpu_root, sys_devices_root) and gpu_root.is_dir(), "DRM device sysfs root differs")
        links[char_name] = target
        gpu_roots.add(gpu_root)
    require(len(gpu_roots) == 1, "DRM devices do not share one exact GPU sysfs root")
    gpu_root = next(iter(gpu_roots))
    gpu_destination = pathlib.Path("/sys/devices") / gpu_root.relative_to(sys_devices_root.resolve(strict=True))
    parents: list[pathlib.Path] = []
    current = gpu_destination.parent
    while current != pathlib.Path("/sys"):
        parents.append(current)
        current = current.parent
    argv = ["--dir", "/sys", "--dir", "/sys/dev", "--dir", "/sys/dev/char"]
    for parent in reversed(parents):
        argv.extend(["--dir", str(parent)])
    argv.extend(["--ro-bind", str(gpu_root), str(gpu_destination)])
    for char_name, target in sorted(links.items()):
        argv.extend(["--symlink", target, f"/sys/dev/char/{char_name}"])
    cpu_online = sys_devices_root / "system/cpu/online"
    require(cpu_online.is_file() and not cpu_online.is_symlink(), "exact CPU-online sysfs file is unavailable")
    for directory in ("/sys/devices/system", "/sys/devices/system/cpu"):
        argv.extend(["--dir", directory])
    argv.extend(["--ro-bind", str(cpu_online), "/sys/devices/system/cpu/online"])
    return argv


def validate_owned_runner(root: pathlib.Path, lock: dict[str, Any]) -> None:
    exact_directory(root)
    require(root.name == "GE-Proton11-7-c5448b76a230384e", "runner installation name changed")
    owner = read_json(root / OWNER_FILE)
    receipt = read_json(root / INSTALL_RECEIPT)
    expected = lock["runner"]
    require(
        owner
        == {
            "identity": expected["archive"]["sha256"],
            "kind": "runner-installation",
            "owner": "kasselvania/Linux-VST-bridge-ubuntu-lab",
            "schema": "linux-vst-bridge-ubuntu-lab-ua1-owner/v1",
            "slice": "UA1",
        },
        "runner owner receipt changed",
    )
    require(receipt.get("archive_sha256") == expected["archive"]["sha256"], "runner archive changed")
    require(receipt.get("archive_byte_length") == expected["archive"]["byte_length"], "runner archive size changed")
    require(receipt.get("launcher") == "proton", "runner launcher changed")
    require(receipt.get("launcher_sha256") == expected["entry_points"][0]["sha256"], "runner launcher digest changed")
    require(receipt.get("tree") == expected["installed_tree"], "runner receipt tree changed")
    require(
        tree_identity(root, exclude={OWNER_FILE, INSTALL_RECEIPT}) == expected["installed_tree"],
        "runner installed tree changed",
    )
    for entry in expected["entry_points"]:
        exact_file(root / entry["path"], entry["byte_length"], entry["sha256"])
        require(stat.S_IMODE((root / entry["path"]).stat().st_mode) == int(entry["mode"], 8), "runner entry mode changed")


def validate_owned_runtime(root: pathlib.Path, lock: dict[str, Any]) -> None:
    exact_directory(root)
    require(root.name == "ubuntu-26.04-i386-freetype-x11-gcc16-20260920", "runner runtime name changed")
    owner = read_json(root / OWNER_FILE)
    receipt = read_json(root / INSTALL_RECEIPT)
    expected = lock["runner"]["runtime"]
    require(
        owner
        == {
            "identity": expected["identity_sha256"],
            "kind": "runner-runtime",
            "owner": "kasselvania/Linux-VST-bridge-ubuntu-lab",
            "schema": "linux-vst-bridge-ubuntu-lab-ua1-owner/v1",
            "slice": "UA1",
        },
        "runner runtime owner receipt changed",
    )
    require(receipt.get("identity_sha256") == expected["identity_sha256"], "runner runtime identity changed")
    require(receipt.get("tree") == expected["installed_tree"], "runner runtime receipt tree changed")
    require(
        tree_identity(root, exclude={OWNER_FILE, INSTALL_RECEIPT}) == expected["installed_tree"],
        "runner runtime tree changed",
    )


def validate_scratch(root: pathlib.Path, lock: dict[str, Any]) -> None:
    exact_directory(root)
    require(root.parent == pathlib.Path("/tmp") and root.name.startswith("lvb-fragments-census."), "scratch root is outside the closed /tmp namespace")
    require(stat.S_IMODE(root.stat().st_mode) == 0o700, "scratch root mode changed")
    require(set(item.name for item in root.iterdir()) == {"module.vst3", "wf0-factory-probe.exe", "host-source-manifest.json", "input.lock.json", "census.py"}, "scratch input roster changed")
    exact_file(root / "module.vst3", lock["fixture"]["module"]["byte_length"], lock["fixture"]["module"]["sha256"])
    exact_file(root / "wf0-factory-probe.exe", lock["windows_host"]["artifact"]["byte_length"], lock["windows_host"]["artifact"]["sha256"])
    exact_file(root / "host-source-manifest.json", lock["windows_host"]["source_manifest"]["byte_length"], lock["windows_host"]["source_manifest"]["sha256"])


def add_optional_ro_bind(argv: list[str], path: str) -> None:
    source = pathlib.Path(path)
    if source.exists() and not source.is_symlink():
        argv.extend(["--ro-bind", path, path])


def sandbox_argv(
    root: pathlib.Path,
    runner: pathlib.Path,
    runtime: pathlib.Path,
    lock: dict[str, Any],
    session: str,
    x11: PrivateX11,
) -> tuple[list[str], pathlib.Path, pathlib.Path]:
    state = root / "state"
    exact_directory(state / "prefix")
    windows = state / "prefix/pfx/drive_c/bridge/sessions" / session
    windows.mkdir(parents=True, mode=0o700)
    target = windows / "wf0-factory-probe.exe"
    shutil.copyfile(root / "wf0-factory-probe.exe", target)
    target.chmod(0o500)
    home = state / "home"
    runtime_state = state / "runtime"
    empty_client = state / "empty-client"
    config = state / "config"
    for directory in (home, runtime_state, empty_client, config):
        directory.mkdir(parents=True, mode=0o700)
    uid, gid = os.getuid(), os.getgid()
    (config / "passwd").write_text(f"ua1:x:{uid}:{gid}:Ubuntu Lab:/home/ua1:/usr/sbin/nologin\n", encoding="utf-8")
    (config / "group").write_text(f"ua1:x:{gid}:\n", encoding="utf-8")
    (config / "machine-id").write_text(LOCK_SHA256[:32] + "\n", encoding="ascii")
    ready = windows / f"{session}.ready"
    gate = windows / f"{session}.gate"
    module_sha = lock["fixture"]["module"]["sha256"]
    host_sha = lock["windows_host"]["artifact"]["sha256"]
    source_sha = lock["windows_host"]["source_manifest"]["sha256"]
    internal_run = f"/run/user/{uid}"
    argv = [
        "/usr/bin/bwrap", "--unshare-all", "--die-with-parent", "--new-session", "--cap-drop", "ALL",
        "--tmpfs", "/", "--dir", "/usr", "--overlay-src", "/usr", "--overlay-src", str(runtime / "usr"), "--ro-overlay", "/usr",
        "--symlink", "usr/bin", "/bin", "--symlink", "usr/sbin", "/sbin", "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib64", "/lib64",
        "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/dev/shm", "--chmod", "1777", "/dev/shm",
        "--dev-bind", "/dev/dri", "/dev/dri",
        "--dir", "/etc", "--ro-bind", str(config / "passwd"), "/etc/passwd", "--ro-bind", str(config / "group"), "/etc/group", "--ro-bind", str(config / "machine-id"), "/etc/machine-id",
        "--dir", "/var", "--dir", "/var/lib", "--dir", "/var/lib/dbus", "--ro-bind", str(config / "machine-id"), "/var/lib/dbus/machine-id",
        "--dir", "/home", "--dir", "/home/ua1", "--bind", str(home), "/home/ua1",
        "--dir", "/opt", "--dir", "/opt/frg1", "--ro-bind", str(runner), "/opt/frg1/runner", "--bind", str(state), "/opt/frg1/state",
        "--dir", "/run", "--dir", "/run/user", "--dir", internal_run,
        "--dir", "/run/frg1", "--ro-bind", str(x11.authority), "/run/frg1/Xauthority", "--tmpfs", "/tmp",
        "--dir", "/tmp/.X11-unix", "--ro-bind", str(x11.socket), f"/tmp/.X11-unix/X{x11.display_number}",
    ]
    for system_path in (
        "/etc/fonts",
        "/etc/ssl/certs",
        "/etc/ca-certificates",
        "/etc/ld.so.cache",
        "/etc/localtime",
        "/etc/vulkan",
        "/etc/OpenCL",
    ):
        add_optional_ro_bind(argv, system_path)
    argv.extend(exact_gpu_sysfs_argv())
    environment = {
        "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        "HOME": "/home/ua1",
        "USER": "ua1",
        "LOGNAME": "ua1",
        "XDG_DATA_HOME": "/home/ua1/.local/share",
        "XDG_CONFIG_HOME": "/home/ua1/.config",
        "XDG_CACHE_HOME": "/home/ua1/.cache",
        "XDG_RUNTIME_DIR": internal_run,
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "DISPLAY": f":{x11.display_number}",
        "XAUTHORITY": "/run/frg1/Xauthority",
        "LD_LIBRARY_PATH": "/usr/lib32:/usr/lib/i386-linux-gnu",
        "TMPDIR": "/tmp",
        "STEAM_COMPAT_DATA_PATH": "/opt/frg1/state/prefix",
        "STEAM_COMPAT_CLIENT_INSTALL_PATH": "/opt/frg1/state/empty-client",
        "STEAM_COMPAT_APP_ID": "0",
        "SteamAppId": "0",
        "SteamGameId": "0",
        "STEAM_ZENITY": "",
        "PRESSURE_VESSEL_VARIABLE_DIR": "/opt/frg1/state/runtime/pressure-vessel-var",
        "WINEDEBUG": "-all",
        "WINEDLLOVERRIDES": "uiautomationcore=",
        "PROTON_LOG": "0",
        "FRG1_INPUT_LOCK_SHA256": LOCK_SHA256,
    }
    argv.append("--clearenv")
    for key, value in environment.items():
        argv.extend(["--setenv", key, value])
    windows_root = f"C:\\bridge\\sessions\\{session}"
    retained_execution = lock["retained_ubuntu_environment"]["execution"]
    command = [
        "/opt/frg1/runner/proton", retained_execution["launcher_verb"], windows_root + "\\wf0-factory-probe.exe",
        "--session", session,
        "--scanner-sha256", host_sha,
        "--implementation-source-manifest-sha256", source_sha,
        "--module", retained_execution["windows_module_path"],
        "--module-sha256", module_sha,
        "--bundle-manifest-sha256", LOCK_SHA256,
        "--ready", windows_root + f"\\{session}.ready",
        "--gate", windows_root + f"\\{session}.gate",
        "--max-classes", "256",
        "--stdout-cap", str(STDOUT_LIMIT),
        "--mode", "ap8-module-inspection",
        "--component-case", retained_execution["component_case"],
    ]
    argv.extend(["--chdir", retained_execution["synthetic_home"], "--", *command])
    return argv, ready, gate


def expected_handshake(lock: dict[str, Any], session: str) -> bytes:
    return (
        "schema=linux-vst-bridge-wf0-handshake/v1\n"
        f"session={session}\n"
        f"scanner_sha256={lock['windows_host']['artifact']['sha256']}\n"
        f"module_sha256={lock['fixture']['module']['sha256']}\n"
        f"bundle_manifest_sha256={LOCK_SHA256}\n"
        f"implementation_source_manifest_sha256={lock['windows_host']['source_manifest']['sha256']}\n"
        "mode=ap8-module-inspection\n"
        f"component_case={lock['retained_ubuntu_environment']['execution']['component_case']}\n"
        "run_ordinal=1\n"
    ).encode()


def terminate(child: subprocess.Popen[bytes]) -> None:
    if child.poll() is not None:
        return
    try:
        os.killpg(child.pid, signal.SIGTERM)
    except ProcessLookupError:
        return
    try:
        child.wait(timeout=5)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        child.wait(timeout=5)


def pump_graphical_environment(x11: PrivateX11) -> None:
    x11.drain.pump(0.0)
    require(x11.process.poll() is None, "private Xwayland exited during scanner operation")


def wait_for_readiness(
    child: subprocess.Popen[bytes],
    drain: BoundedDrain,
    ready: pathlib.Path,
    timeout: float,
    x11: PrivateX11 | None = None,
) -> None:
    deadline = time.monotonic() + timeout
    while True:
        drain.pump(0.05)
        if x11 is not None:
            pump_graphical_environment(x11)
        if ready.exists():
            return
        if child.poll() is not None:
            drain.finish()
            raise CensusError(f"scanner exited before readiness: {child.returncode}")
        if time.monotonic() >= deadline:
            raise CensusError("scanner readiness timeout")


def wait_for_completion(
    child: subprocess.Popen[bytes],
    drain: BoundedDrain,
    timeout: float,
    x11: PrivateX11 | None = None,
) -> None:
    deadline = time.monotonic() + timeout
    while child.poll() is None:
        drain.pump(0.05)
        if x11 is not None:
            pump_graphical_environment(x11)
        if time.monotonic() >= deadline:
            raise CensusError("scanner execution timeout")
    drain.finish()


def retire_drained_process(child: subprocess.Popen[bytes], drain: BoundedDrain) -> tuple[bytes, bytes]:
    terminate(child)
    drain.finish()
    return drain.output()


def retain_private_streams(root: pathlib.Path, output: bytes, error: bytes) -> dict[str, Any]:
    require(len(output) <= STDOUT_LIMIT, "scanner stdout exceeded bound")
    require(len(error) <= STDERR_LIMIT, "scanner stderr exceeded bound")
    for name, value in (("raw.stdout", output), ("raw.stderr", error)):
        path = root / name
        path.write_bytes(value)
        path.chmod(0o600)
    return {"stdout_sha256": sha256_bytes(output), "stderr_sha256": sha256_bytes(error)}


def retain_private_x11_streams(root: pathlib.Path, output: bytes, error: bytes) -> dict[str, Any]:
    require(len(output) <= X11_STDOUT_LIMIT, "private Xwayland stdout exceeded bound")
    require(len(error) <= X11_STDERR_LIMIT, "private Xwayland stderr exceeded bound")
    for name, value in (("raw.x11.stdout", output), ("raw.x11.stderr", error)):
        path = root / name
        path.write_bytes(value)
        path.chmod(0o600)
    return {"x11_stdout_sha256": sha256_bytes(output), "x11_stderr_sha256": sha256_bytes(error)}


def parse_records(output: bytes) -> list[dict[str, Any]]:
    require(len(output) <= STDOUT_LIMIT, "scanner stdout exceeded bound")
    records: list[dict[str, Any]] = []
    for line in output.splitlines():
        require(len(line) <= 64 * 1024, "scanner record exceeded bound")
        value = json.loads(line)
        require(isinstance(value, dict), "scanner record is not an object")
        records.append(value)
    require(0 < len(records) <= 4096, "scanner record count invalid")
    return records


def one(records: list[dict[str, Any]], state: str) -> dict[str, Any]:
    found = [record for record in records if record.get("state") == state]
    require(len(found) == 1, f"expected one {state} record")
    return found[0]


def validate_successor_delta(records: list[dict[str, Any]], lock: dict[str, Any]) -> dict[str, Any]:
    predecessor = lock["historical_profile"]
    policy = lock["successor_verification"]
    observed_class = one(records, "ap12_class")
    expected_class = policy["expected_class"]
    for key in ("class_id", "name", "vendor", "version", "subcategories", "metadata_tier"):
        require(observed_class.get(key) == expected_class[key], f"successor {key} differs from expected migration")
    require(
        ("Fx" in observed_class["subcategories"].split("|"))
        and ("Instrument" not in observed_class["subcategories"].split("|")),
        "successor role differs from predecessor effect",
    )
    parameter_count = one(records, "ap8_parameter_count").get("count")
    require(isinstance(parameter_count, int) and 0 <= parameter_count <= 8192, "successor parameter count invalid")
    capabilities = one(records, "ap12_capabilities")
    require(capabilities.get("float32_result") == 0, "successor lost predecessor float32 baseline")
    controller = one(records, "ap8_controller_association")
    require(isinstance(controller.get("combined"), bool), "successor controller association invalid")
    if controller["combined"] is False:
        require(
            isinstance(controller.get("class_id"), str)
            and len(controller["class_id"]) == 32
            and all(character in "0123456789ABCDEF" for character in controller["class_id"]),
            "successor controller class identity invalid",
        )
    return {
        "predecessor_profile_sha256": predecessor["profile_file_sha256"],
        "predecessor_revision": predecessor["revision"],
        "predecessor_module_sha256": predecessor["module_sha256"],
        "successor_module_sha256": lock["fixture"]["module"]["sha256"],
        "class_continuity": True,
        "class_id": observed_class["class_id"],
        "predecessor_version": predecessor["class"]["version"],
        "successor_version": observed_class["version"],
        "predecessor_parameter_count": predecessor["parameter_count"],
        "successor_parameter_count": parameter_count,
        "parameter_count_changed": parameter_count != predecessor["parameter_count"],
        "controller_association": {
            "combined": controller["combined"],
            "class_id": controller.get("class_id"),
        },
        "candidate_capabilities_inherited_not_yet_qualified": predecessor["capabilities"],
    }


def execute(
    root: pathlib.Path,
    runner: pathlib.Path,
    runtime: pathlib.Path,
    prefix_source: pathlib.Path,
    lock: dict[str, Any],
) -> dict[str, Any]:
    session = secrets.token_hex(16)
    x11: PrivateX11 | None = None
    child: subprocess.Popen[bytes] | None = None
    drain: BoundedDrain | None = None
    output = b""
    error = b""
    x11_output = b""
    x11_error = b""
    records: list[dict[str, Any]] = []
    successor_delta: dict[str, Any] = {}
    custody: dict[str, Any] = {}
    phase = "prefix_snapshot"
    try:
        _, custody = snapshot_retained_prefix(prefix_source, root / "state", lock)
        phase = "readiness"
        x11 = start_private_x11(root)
        argv, ready, gate = sandbox_argv(root, runner, runtime, lock, session, x11)
        child = subprocess.Popen(
            argv,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        drain = BoundedDrain(
            child,
            stdout_limit=STDOUT_LIMIT,
            stderr_limit=STDERR_LIMIT,
            label="scanner",
        )
        wait_for_readiness(child, drain, ready, READY_TIMEOUT, x11)
        handshake = ready.read_bytes()
        require(handshake == expected_handshake(lock, session), "scanner readiness binding changed")
        phase = "execution"
        with gate.open("xb") as handle:
            handle.write(handshake)
            handle.flush()
            os.fsync(handle.fileno())
        wait_for_completion(child, drain, EXECUTION_TIMEOUT, x11)
        output, error = drain.output()
        x11_output, x11_error = retire_drained_process(x11.process, x11.drain)
        phase = "record_parse"
        records = parse_records(output)
        one(records, "ap8_factory")
        one(records, "ap12_class")
        one(records, "ap8_controller_association")
        successor_delta = validate_successor_delta(records, lock)
        source_after = _validate_prefix_tree(prefix_source, lock)
        require(
            source_after
            == {
                key: custody[key]
                for key in ("owner_identity", "owner_file_sha256", "module_sha256", "pfx_mode")
            },
            "retained prefix source changed during execution",
        )
        custody["source_unchanged_after_execution"] = True
    except Exception as failure:
        if child is not None:
            if drain is not None:
                output, error = retire_drained_process(child, drain)
            else:
                terminate(child)
        if x11 is not None:
            x11_output, x11_error = retire_drained_process(x11.process, x11.drain)
        retained_source_unchanged = False
        try:
            source_now = _validate_prefix_tree(prefix_source, lock)
            baseline = custody or source_now
            retained_source_unchanged = source_now == {
                key: baseline[key]
                for key in ("owner_identity", "owner_file_sha256", "module_sha256", "pfx_mode")
            }
        except Exception:
            retained_source_unchanged = False
        digests = retain_private_streams(root, output, error)
        x11_digests = retain_private_x11_streams(root, x11_output, x11_error)
        failure_record = {
            "schema": "linux-vst-bridge-frg1-private-failure/v1",
            "classification": "FRG1_FACTORY_CENSUS_FAILED",
            "phase": phase,
            "reason": str(failure),
            "exit_code": None if child is None else child.returncode,
            "scanner_stream_capacity_exhausted": [] if drain is None else sorted(drain.exhausted),
            "x11_stream_capacity_exhausted": [] if x11 is None else sorted(x11.drain.exhausted),
            "retained_prefix_source_unchanged": retained_source_unchanged,
            **digests,
            **x11_digests,
        }
        private_failure = root / "failure.json"
        private_failure.write_bytes(canonical_json(failure_record) + b"\n")
        private_failure.chmod(0o600)
        raise
    finally:
        if child is not None and child.poll() is None:
            terminate(child)
        if x11 is not None and x11.process.poll() is None:
            terminate(x11.process)
    require(child is not None and drain is not None and x11 is not None, "scanner launch state incomplete")
    digests = retain_private_streams(root, output, error)
    x11_digests = retain_private_x11_streams(root, x11_output, x11_error)
    closed = one(records, "ap8_inspection_closed")
    completed = one(records, "scanner_completed")
    result = {
        "schema": "linux-vst-bridge-frg1-factory-census/v1",
        "input_lock_sha256": LOCK_SHA256,
        "module_sha256": lock["fixture"]["module"]["sha256"],
        "scanner_sha256": lock["windows_host"]["artifact"]["sha256"],
        "implementation_source_manifest_sha256": lock["windows_host"]["source_manifest"]["sha256"],
        "runner": {
            "archive_sha256": lock["runner"]["archive"]["sha256"],
            "installed_tree": lock["runner"]["installed_tree"],
            "runtime_identity_sha256": lock["runner"]["runtime"]["identity_sha256"],
            "runtime_tree": lock["runner"]["runtime"]["installed_tree"],
            "environment_policy": lock["runner"]["environment_policy"],
        },
        "retained_ubuntu_environment": {
            **custody,
            "launcher_verb": lock["retained_ubuntu_environment"]["execution"]["launcher_verb"],
            "component_case": lock["retained_ubuntu_environment"]["execution"]["component_case"],
        },
        "gated": True,
        "network_shared": False,
        "real_home_visible": False,
        "graphical_environment": {
            "kind": "operation_owned_xwayland",
            "binary_sha256": XWAYLAND_SHA256,
            "binary_byte_length": XWAYLAND_BYTE_LENGTH,
            "wayland_socket": WAYLAND_SOCKET_NAME,
            "real_x11_session_shared": False,
            "drm_devices_projected": True,
            "gpu_sysfs_projection": "exact_single_gpu_roster",
        },
        "successor_delta": successor_delta,
        "readiness_timeout_seconds": int(READY_TIMEOUT),
        "execution_timeout_seconds": int(EXECUTION_TIMEOUT),
        "cleanup_confirmed": child.returncode is not None and x11.process.returncode is not None,
        "transport_retired": child.returncode is not None,
        "exit_code": child.returncode,
        "error": None if child.returncode == 0 else "scanner_nonzero_exit",
        "inspection_exit_code": closed.get("exit_code"),
        "inspection_complete": completed.get("inspection_complete"),
        **digests,
        **x11_digests,
        "records": records,
    }
    (root / "report.json").write_bytes(canonical_json(result) + b"\n")
    (root / "report.json").chmod(0o600)
    require(child.returncode == 0, "scanner returned nonzero")
    require(closed.get("exit_code") == 0 and completed.get("inspection_complete") is True, "inspection incomplete")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("validate", "execute"))
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--runner", type=pathlib.Path, required=True)
    parser.add_argument("--runtime", type=pathlib.Path, required=True)
    parser.add_argument("--prefix-source", type=pathlib.Path, required=True)
    parser.add_argument("--custody-lock", type=pathlib.Path, required=True)
    parser.add_argument("--lock", type=pathlib.Path, required=True)
    args = parser.parse_args()
    lock = load_lock(args.lock.resolve(strict=True))
    root = args.root.resolve(strict=True)
    runner = args.runner.resolve(strict=True)
    runtime = args.runtime.resolve(strict=True)
    prefix_source = args.prefix_source.resolve(strict=True)
    custody_lock = args.custody_lock.resolve(strict=True)
    validate_scratch(root, lock)
    validate_owned_runner(runner, lock)
    validate_owned_runtime(runtime, lock)
    custody = validate_retained_prefix(prefix_source, custody_lock, lock)
    if args.action == "validate":
        print(json.dumps({
            "classification": "FRG1_INPUTS_VALIDATED",
            "input_lock_sha256": LOCK_SHA256,
            "retained_prefix": custody,
        }, sort_keys=True))
        return 0
    print(json.dumps(execute(root, runner, runtime, prefix_source, lock), sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CensusError, OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"FRG1_REFUSAL: {error}", file=sys.stderr)
        raise SystemExit(1)

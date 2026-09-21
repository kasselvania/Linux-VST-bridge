#!/usr/bin/env python3
"""Closed FRG1 factory-census supervisor.

The commercial module remains an ignored private input.  This program accepts only
the tracked FRG1 lock, the exact admitted host, and the exact UA1 runner receipts.
It runs in a fresh networkless Bubblewrap namespace and emits a bounded report.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import secrets
import shutil
import signal
import stat
import subprocess
import sys
import time
from typing import Any, Iterable


SCHEMA = "linux-vst-bridge-frg1-input-lock/v1"
LOCK_SHA256 = "9c1c93820bfff45d06ade73c7ffebbd9014b9f8e38b85e3aebeea20827d2de43"
OWNER_FILE = ".ua1-owner.json"
INSTALL_RECEIPT = ".ua1-install.json"
STDOUT_LIMIT = 1_048_576
STDERR_LIMIT = 4_194_304
READY_TIMEOUT = 90.0
EXECUTION_TIMEOUT = 180.0


class CensusError(RuntimeError):
    pass


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
    return value


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


def sandbox_argv(root: pathlib.Path, runner: pathlib.Path, runtime: pathlib.Path, lock: dict[str, Any], session: str) -> tuple[list[str], pathlib.Path, pathlib.Path]:
    state = root / "state"
    windows = state / "prefix/pfx/drive_c/bridge/sessions" / session
    windows.mkdir(parents=True, mode=0o700)
    for source, name in ((root / "module.vst3", "module.vst3"), (root / "wf0-factory-probe.exe", "wf0-factory-probe.exe")):
        target = windows / name
        shutil.copyfile(source, target)
        target.chmod(0o500 if name.endswith(".exe") else 0o400)
    home = state / "home"
    runtime_state = state / "runtime"
    empty_client = state / "empty-client"
    config = state / "config"
    for directory in (home, runtime_state, empty_client, config):
        directory.mkdir(parents=True, mode=0o700)
    uid, gid = os.getuid(), os.getgid()
    (config / "passwd").write_text(f"frg1:x:{uid}:{gid}:FRG1:/home/frg1:/usr/sbin/nologin\n", encoding="utf-8")
    (config / "group").write_text(f"frg1:x:{gid}:\n", encoding="utf-8")
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
        "--dir", "/etc", "--ro-bind", str(config / "passwd"), "/etc/passwd", "--ro-bind", str(config / "group"), "/etc/group", "--ro-bind", str(config / "machine-id"), "/etc/machine-id",
        "--dir", "/var", "--dir", "/var/lib", "--dir", "/var/lib/dbus", "--ro-bind", str(config / "machine-id"), "/var/lib/dbus/machine-id",
        "--dir", "/home", "--dir", "/home/frg1", "--bind", str(home), "/home/frg1",
        "--dir", "/opt", "--dir", "/opt/frg1", "--ro-bind", str(runner), "/opt/frg1/runner", "--bind", str(state), "/opt/frg1/state",
        "--dir", "/run", "--dir", "/run/user", "--dir", internal_run, "--tmpfs", "/tmp",
    ]
    for system_path in ("/etc/fonts", "/etc/ssl/certs", "/etc/ca-certificates", "/etc/ld.so.cache", "/etc/localtime"):
        add_optional_ro_bind(argv, system_path)
    cpu_online = pathlib.Path("/sys/devices/system/cpu/online")
    if cpu_online.is_file() and not cpu_online.is_symlink():
        argv.extend(["--dir", "/sys", "--dir", "/sys/devices", "--dir", "/sys/devices/system", "--dir", "/sys/devices/system/cpu", "--ro-bind", str(cpu_online), str(cpu_online)])
    environment = {
        "PATH": "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        "HOME": "/home/frg1",
        "USER": "frg1",
        "LOGNAME": "frg1",
        "XDG_DATA_HOME": "/home/frg1/.local/share",
        "XDG_CONFIG_HOME": "/home/frg1/.config",
        "XDG_CACHE_HOME": "/home/frg1/.cache",
        "XDG_RUNTIME_DIR": internal_run,
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
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
    command = [
        "/opt/frg1/runner/proton", "run", windows_root + "\\wf0-factory-probe.exe",
        "--session", session,
        "--scanner-sha256", host_sha,
        "--implementation-source-manifest-sha256", source_sha,
        "--module", windows_root + "\\module.vst3",
        "--module-sha256", module_sha,
        "--bundle-manifest-sha256", LOCK_SHA256,
        "--ready", windows_root + f"\\{session}.ready",
        "--gate", windows_root + f"\\{session}.gate",
        "--max-classes", "256",
        "--stdout-cap", str(STDOUT_LIMIT),
        "--mode", "ap8-module-inspection",
        "--component-case", "first-audio",
    ]
    argv.extend(["--chdir", "/home/frg1", "--", *command])
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
        "component_case=first-audio\n"
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


def execute(root: pathlib.Path, runner: pathlib.Path, runtime: pathlib.Path, lock: dict[str, Any]) -> dict[str, Any]:
    session = secrets.token_hex(16)
    argv, ready, gate = sandbox_argv(root, runner, runtime, lock, session)
    child = subprocess.Popen(argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    try:
        deadline = time.monotonic() + READY_TIMEOUT
        while not ready.exists():
            if child.poll() is not None:
                output, error = child.communicate(timeout=5)
                raise CensusError(f"scanner exited before readiness: {child.returncode}; stderr_sha256={sha256_bytes(error)}; stdout_sha256={sha256_bytes(output)}")
            if time.monotonic() >= deadline:
                raise CensusError("scanner readiness timeout")
            time.sleep(0.05)
        handshake = ready.read_bytes()
        require(handshake == expected_handshake(lock, session), "scanner readiness binding changed")
        with gate.open("xb") as handle:
            handle.write(handshake)
            handle.flush()
            os.fsync(handle.fileno())
        output, error = child.communicate(timeout=EXECUTION_TIMEOUT)
    except Exception:
        terminate(child)
        raise
    finally:
        if child.poll() is None:
            terminate(child)
    require(len(error) <= STDERR_LIMIT, "scanner stderr exceeded bound")
    (root / "raw.stdout").write_bytes(output)
    (root / "raw.stderr").write_bytes(error)
    (root / "raw.stdout").chmod(0o600)
    (root / "raw.stderr").chmod(0o600)
    records = parse_records(output)
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
        "gated": True,
        "network_shared": False,
        "real_home_visible": False,
        "cleanup_confirmed": child.returncode is not None,
        "transport_retired": child.returncode is not None,
        "exit_code": child.returncode,
        "error": None if child.returncode == 0 else "scanner_nonzero_exit",
        "inspection_exit_code": closed.get("exit_code"),
        "inspection_complete": completed.get("inspection_complete"),
        "stdout_sha256": sha256_bytes(output),
        "stderr_sha256": sha256_bytes(error),
        "records": records,
    }
    (root / "report.json").write_bytes(canonical_json(result) + b"\n")
    (root / "report.json").chmod(0o600)
    require(child.returncode == 0, "scanner returned nonzero")
    require(closed.get("exit_code") == 0 and completed.get("inspection_complete") is True, "inspection incomplete")
    one(records, "ap8_factory")
    one(records, "ap12_class")
    one(records, "ap8_controller_association")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=("validate", "execute"))
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--runner", type=pathlib.Path, required=True)
    parser.add_argument("--runtime", type=pathlib.Path, required=True)
    parser.add_argument("--lock", type=pathlib.Path, required=True)
    args = parser.parse_args()
    lock = load_lock(args.lock.resolve(strict=True))
    root = args.root.resolve(strict=True)
    runner = args.runner.resolve(strict=True)
    runtime = args.runtime.resolve(strict=True)
    validate_scratch(root, lock)
    validate_owned_runner(runner, lock)
    validate_owned_runtime(runtime, lock)
    if args.action == "validate":
        print(json.dumps({"classification": "FRG1_INPUTS_VALIDATED", "input_lock_sha256": LOCK_SHA256}, sort_keys=True))
        return 0
    print(json.dumps(execute(root, runner, runtime, lock), sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (CensusError, OSError, ValueError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"FRG1_REFUSAL: {error}", file=sys.stderr)
        raise SystemExit(1)

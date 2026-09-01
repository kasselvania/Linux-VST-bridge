#!/usr/bin/env python3
"""HP1 bounded fixture capture, process monitor, and evidence renderer.

Only Python's standard library is used. Raw session state remains beneath the
canonical user cache; committed evidence is generated separately and contains
no durable PID, private home path, or complete process/maps dump.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import pwd
import re
import secrets
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterable


SCHEMA_SESSION = "linux-vst-bridge-hp1-session/v1"
SCHEMA_FIXTURE = "linux-vst-bridge-hp1-fixture-snapshot/v1"
SCHEMA_MONITOR = "linux-vst-bridge-hp1-monitor/v1"
SCHEMA_STATE_SCAN = "linux-vst-bridge-hp1-bitwig-state-scan/v1"
SCHEMA_ATTESTATION = "linux-vst-bridge-hp1-operator-attestation/v1"
SCHEMA_EVIDENCE = "linux-vst-bridge-hp1-fixture/v1"

EXPECTED_REPO = pathlib.Path("/home/deck/code/Linux-VST-bridge")
EXPECTED_BRANCH = "codex/hp1-bitwig-native-discovery-admission"
EXPECTED_BASIS_COMMIT = "7cda2d85eb426c2ed6e4eb3d86e52c114c4aa1c4"
EXPECTED_BASIS_TREE = "e9534822625ceff0f06549b78c85546050bddf42"

HP0_BUNDLE = "LabHostProbe.vst3"
HP0_MODULE_RELATIVE = "Contents/x86_64-linux/LabHostProbe.so"
HP0_MODULE_SHA256 = "3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7"
HP0_PUBLICATION_RECEIPT_SHA256 = "d328adb27fef326b8e5b1104f072c246a803389a77d35ee5babc9834067668d1"
HP0_PUBLICATION_RECEIPT_SCHEMA = "linux-vst-bridge-hp0-publication/v2"
HP0_PROCESSOR_CID = "6F4E7A5392E54B54A98AD6F714E0C201"
HP0_CONTROLLER_CID = "B9C42F0736C34E218E5A71D40C8F1B62"
HP0_BUILD_SOURCE_SCHEMA = "linux-vst-bridge-hp0-build-source/v1"
HP0_BUILD_SOURCE_SHA256 = "ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab"
HP0_BUNDLE_MANIFEST_SHA256 = "ec8a4531a3b73da3c3e29c6e1f3c90c989fb61e3f3397af32d4c1576b3624ea9"
HP0_VALIDATOR_SHA256 = "cccae776eb87fbbbf6ac9c34c78bf1548937c48db3843e0cf0f12d312650466f"

BITWIG_APP_ID = "com.bitwig.BitwigStudio"
BITWIG_APP_REF = "app/com.bitwig.BitwigStudio/x86_64/stable"
BITWIG_VERSION = "6.0.11"
BITWIG_APP_COMMIT = "7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e"
BITWIG_RUNTIME_REF = "org.freedesktop.Platform/x86_64/25.08"
BITWIG_RUNTIME_COMMIT = "bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8"
BITWIG_USER_OVERRIDE_SHA256 = "1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e"
BITWIG_SYSTEM_OVERRIDE_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
BITWIG_VST_PATH = "/app/extensions/Plugins/vst;=/usr/lib/"

SERUM_FIXTURES = (
    {
        "role": "existing_native_proxy",
        "relative": ".vst3/yabridge/Serum2.vst3/Contents/x86_64-linux/Serum2.so",
        "sha256": "317d70f95a3c7559ff3d43b014c3e7b792fd03362d5e999d550a5566cf25b184",
        "size": 88888,
        "mtime": "2025-04-29 21:58:29.370060240 -0700",
    },
    {
        "role": "existing_windows_module",
        "relative": ".wine/drive_c/Program Files/Common Files/VST3/Serum2.vst3/Contents/x86_64-win/Serum2.vst3",
        "sha256": "838bc7ab42d5d039156768680ffc3e0175d6e6bed9f99802989a01d695e13175",
        "size": 18062336,
        "mtime": "2025-04-26 10:45:32.000000000 -0700",
    },
)

BITWIG_NAMES = {"BitwigStudio", "bitwig-studio"}
VALIDATOR_NAMES = {"validator"}
FORBIDDEN_NAMES = {
    "wine",
    "wine64",
    "wine-preloader",
    "wine64-preloader",
    "wineserver",
    "umu",
    "umu-run",
    "proton",
    "proton-waitforexit",
    "proton-waitforexitandrun",
    "yabridge-host",
    "yabridge-host.exe",
    "yabridge-host-32.exe",
}

DISCOVERY_TOKENS = (
    "LAB Host Probe",
    "Kasselvania Research",
    HP0_PROCESSOR_CID,
    HP0_CONTROLLER_CID,
    "LabHostProbe.so",
)

SESSION_ID_RE = re.compile(r"^hp1-[0-9]{8}T[0-9]{6}Z-[0-9a-f]{16}$")
NONCE_RE = re.compile(r"^[0-9a-f]{32}$")
HEX64_RE = re.compile(r"^[0-9a-f]{64}$")


class HP1Error(RuntimeError):
    pass


def fail(message: str) -> None:
    raise HP1Error(message)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path, max_bytes: int | None = None) -> str:
    digest = hashlib.sha256()
    count = 0
    with path.open("rb", buffering=0) as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            count += len(chunk)
            if max_bytes is not None and count > max_bytes:
                fail(f"bounded hash input exceeds {max_bytes} bytes: {path.name}")
            digest.update(chunk)
    return digest.hexdigest()


def atomic_write_bytes(path: pathlib.Path, data: bytes, mode: int = 0o600) -> None:
    path.parent.mkdir(parents=False, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        os.fchmod(fd, mode)
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except Exception:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise


def atomic_write_json(path: pathlib.Path, value: Any) -> None:
    data = (json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")
    atomic_write_bytes(path, data)


def read_json(path: pathlib.Path) -> Any:
    if not path.is_file() or path.is_symlink():
        fail(f"required JSON is missing, non-regular, or a symlink: {path.name}")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        fail(f"invalid JSON in {path.name}: {exc}")


def run_command(
    arguments: list[str],
    *,
    cwd: pathlib.Path | None = None,
    timeout: float = 20.0,
    check: bool = True,
) -> subprocess.CompletedProcess[bytes]:
    try:
        result = subprocess.run(
            arguments,
            cwd=cwd,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        fail(f"command timed out after {timeout:g}s: {arguments[0]}")
    except OSError as exc:
        fail(f"command could not execute: {arguments[0]}: {exc}")
    if check and result.returncode != 0:
        detail = result.stderr.decode("utf-8", "replace").strip().splitlines()
        safe_detail = detail[-1][:300] if detail else "no diagnostic"
        fail(f"command failed ({result.returncode}): {arguments[0]}: {safe_detail}")
    return result


def command_text(arguments: list[str], **kwargs: Any) -> str:
    return run_command(arguments, **kwargs).stdout.decode("utf-8", "strict").rstrip("\n")


def real_home() -> pathlib.Path:
    home = pathlib.Path(pwd.getpwuid(os.getuid()).pw_dir)
    validate_absolute_path(home, "real user home", require_existing=True, require_directory=True)
    if pathlib.Path(os.path.realpath(home)) != home:
        fail("real user home is not canonical")
    return home


def validate_absolute_path(
    path: pathlib.Path,
    label: str,
    *,
    require_existing: bool = False,
    require_directory: bool = False,
    regular_or_absent: bool = False,
) -> pathlib.Path:
    raw = str(path)
    if not path.is_absolute() or raw == "/" or raw.endswith("/") or "//" in raw:
        fail(f"{label} must be an unambiguous absolute non-root path")
    parts = pathlib.PurePath(raw).parts
    if any(part in {"", ".", ".."} for part in parts[1:]):
        fail(f"{label} contains an unsafe path component")

    current = pathlib.Path(parts[0])
    for index, component in enumerate(parts[1:], start=1):
        current = current / component
        try:
            metadata = os.lstat(current)
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(metadata.st_mode):
            fail(f"{label} has a symlinked path component")
        if index < len(parts) - 1 and not stat.S_ISDIR(metadata.st_mode):
            fail(f"{label} has a non-directory ancestor")

    if require_existing and not path.exists():
        fail(f"{label} must exist")
    if require_directory and (not path.is_dir() or path.is_symlink()):
        fail(f"{label} must be a non-symlink directory")
    if regular_or_absent and path.exists() and (not path.is_file() or path.is_symlink()):
        fail(f"{label} must be a regular file or absent")

    canonical = pathlib.Path(os.path.realpath(path))
    if path.exists() and canonical != path:
        fail(f"{label} is not canonical")
    if not path.exists():
        existing = path
        suffix: list[str] = []
        while not existing.exists():
            suffix.append(existing.name)
            existing = existing.parent
        projected = pathlib.Path(os.path.realpath(existing))
        for component in reversed(suffix):
            projected /= component
        if projected != path:
            fail(f"{label} would not resolve canonically")
    return path


def user_cache_root() -> pathlib.Path:
    configured = os.environ.get("XDG_CACHE_HOME")
    cache = pathlib.Path(configured) if configured else real_home() / ".cache"
    validate_absolute_path(cache, "user cache root", require_existing=True, require_directory=True)
    return cache


def require_descendant(
    path: pathlib.Path,
    root: pathlib.Path,
    label: str,
    *,
    existing_directory: bool = False,
    directory_or_absent: bool = False,
    regular_or_absent: bool = False,
) -> pathlib.Path:
    validate_absolute_path(root, "containment root", require_existing=True, require_directory=True)
    validate_absolute_path(
        path,
        label,
        require_existing=existing_directory,
        require_directory=existing_directory,
        regular_or_absent=regular_or_absent,
    )
    if directory_or_absent and path.exists() and (not path.is_dir() or path.is_symlink()):
        fail(f"{label} must be a directory or absent")
    try:
        path.relative_to(root)
    except ValueError:
        fail(f"{label} must be a canonical descendant of the declared root")
    if path == root:
        fail(f"{label} must be below, not equal to, the declared root")
    return path


def mkdir_descendant(path: pathlib.Path, root: pathlib.Path, label: str) -> None:
    require_descendant(path, root, label, directory_or_absent=True)
    relative = path.relative_to(root)
    current = root
    for component in relative.parts:
        current /= component
        if current.exists():
            validate_absolute_path(current, label, require_existing=True, require_directory=True)
        else:
            current.mkdir(mode=0o700)
    validate_absolute_path(path, label, require_existing=True, require_directory=True)


def session_root_from_id(session_id: str) -> pathlib.Path:
    if not SESSION_ID_RE.fullmatch(session_id):
        fail("session ID has the wrong format")
    root = user_cache_root() / "linux-vst-bridge" / "hp1" / "sessions" / session_id
    require_descendant(root, user_cache_root(), "HP1 session root", existing_directory=True)
    meta = read_json(root / "session.json")
    if meta.get("schema") != SCHEMA_SESSION or meta.get("session_id") != session_id:
        fail("session metadata identity differs from the requested session")
    return root


def parse_key_values(data: bytes, label: str) -> dict[str, str]:
    result: dict[str, str] = {}
    try:
        text = data.decode("utf-8", "strict")
    except UnicodeError as exc:
        fail(f"{label} is not UTF-8: {exc}")
    for line in text.splitlines():
        if not line or "=" not in line:
            fail(f"{label} contains a malformed line")
        key, value = line.split("=", 1)
        if not re.fullmatch(r"[A-Za-z0-9_]+", key) or key in result:
            fail(f"{label} contains a malformed or duplicate key")
        result[key] = value
    return result


def require_exact(mapping: dict[str, str], key: str, expected: str, label: str) -> None:
    observed = mapping.get(key)
    if observed != expected:
        fail(f"{label} differs for {key}: expected {expected}, observed {observed!r}")


def proc_stat(path: pathlib.Path) -> tuple[int, str, str, int, int]:
    raw = path.read_text(encoding="utf-8", errors="strict").strip()
    match = re.match(r"^(\d+) \((.*)\) ([A-Za-z]) (.*)$", raw)
    if not match:
        fail(f"malformed proc stat: {path}")
    pid = int(match.group(1))
    comm = match.group(2)
    state = match.group(3)
    remaining = match.group(4).split()
    if len(remaining) < 19:
        fail(f"short proc stat: {path}")
    ppid = int(remaining[0])
    starttime = int(remaining[18])
    return pid, comm, state, ppid, starttime


def read_proc_table(proc_root: pathlib.Path, module_path: pathlib.Path | None = None) -> dict[str, Any]:
    validate_absolute_path(proc_root, "proc root", require_existing=True, require_directory=True)
    numeric = sorted((item for item in proc_root.iterdir() if item.name.isdigit()), key=lambda p: int(p.name))
    if len(numeric) > 4096:
        return {"complete": False, "error": "process_count_cap", "processes": {}}
    processes: dict[int, dict[str, Any]] = {}
    for directory in numeric:
        try:
            pid, stat_comm, state, ppid, starttime = proc_stat(directory / "stat")
            comm = (directory / "comm").read_text(encoding="utf-8", errors="strict").rstrip("\n")
            if not comm:
                comm = stat_comm
            uid = None
            for line in (directory / "status").read_text(encoding="utf-8", errors="strict").splitlines():
                if line.startswith("Uid:"):
                    uid = int(line.split()[1])
                    break
            if uid is None:
                continue
            cgroup = (directory / "cgroup").read_text(encoding="utf-8", errors="replace")[:16384]
            exe_name = "unavailable"
            try:
                exe_name = pathlib.Path(os.readlink(directory / "exe")).name
            except OSError:
                if (directory / "exe.name").is_file():
                    exe_name = (directory / "exe.name").read_text(encoding="utf-8").strip()
            process: dict[str, Any] = {
                "pid": pid,
                "ppid": ppid,
                "starttime": starttime,
                "state": state,
                "comm": comm,
                "exe_basename": exe_name,
                "uid": uid,
                "cgroup": cgroup,
                "mapping": None,
            }
            if module_path is not None:
                maps_path = directory / "maps"
                try:
                    with maps_path.open("r", encoding="utf-8", errors="replace") as handle:
                        for line in handle:
                            stripped = line.rstrip("\n")
                            pieces = stripped.split(None, 5)
                            if len(pieces) == 6 and pieces[5] == str(module_path):
                                process["mapping"] = {
                                    "address_range": pieces[0],
                                    "permissions": pieces[1],
                                    "offset": pieces[2],
                                    "device": pieces[3],
                                    "inode": pieces[4],
                                    "path": pieces[5],
                                }
                                break
                except (FileNotFoundError, PermissionError, ProcessLookupError):
                    pass
            processes[pid] = process
        except (FileNotFoundError, PermissionError, ProcessLookupError, UnicodeError, ValueError, HP1Error):
            continue
    return {"complete": True, "error": None, "processes": processes}


def process_names(process: dict[str, Any]) -> set[str]:
    return {str(process.get("comm", "")), str(process.get("exe_basename", ""))}


def bitwig_cgroup_scope(cgroup: str) -> str | None:
    pattern = re.compile(
        rf"(?:^|/)(app-flatpak-{re.escape(BITWIG_APP_ID)}-[^/\s]+\.scope)(?:/|$)"
    )
    matches: list[str] = []
    for line in cgroup.splitlines():
        pieces = line.split(":", 2)
        if len(pieces) != 3:
            continue
        path = pieces[2]
        match = pattern.search(path)
        if match:
            matches.append(path[: match.end(1)])
    unique = sorted(set(matches))
    return unique[0] if len(unique) == 1 else None


def highest_same_scope_ancestor(
    process: dict[str, Any],
    processes: dict[int, dict[str, Any]],
    scope: str,
) -> dict[str, Any]:
    current = process
    seen: set[int] = set()
    for _ in range(32):
        if current["pid"] in seen:
            break
        seen.add(current["pid"])
        parent = processes.get(current["ppid"])
        if parent is None:
            break
        if parent.get("uid") != process.get("uid"):
            break
        if bitwig_cgroup_scope(str(parent.get("cgroup", ""))) != scope:
            break
        current = parent
    return current


def classify_processes(table: dict[str, Any], module_path: pathlib.Path | None = None) -> dict[str, Any]:
    processes: dict[int, dict[str, Any]] = table["processes"]
    bitwig_candidates: list[dict[str, Any]] = []
    forbidden: list[dict[str, Any]] = []
    validators: list[dict[str, Any]] = []
    mappings: list[dict[str, Any]] = []

    for process in processes.values():
        names = process_names(process)
        if names & BITWIG_NAMES:
            bitwig_candidates.append(process)
        matched_forbidden = sorted(names & FORBIDDEN_NAMES)
        if matched_forbidden:
            forbidden.append({**process, "matched_name": matched_forbidden[0]})
        if names & VALIDATOR_NAMES:
            validators.append(process)
        if module_path is not None and process.get("mapping") is not None:
            mappings.append(process)

    roots_by_identity: dict[str, dict[str, Any]] = {}
    for candidate in bitwig_candidates:
        scope = bitwig_cgroup_scope(str(candidate.get("cgroup", "")))
        if candidate.get("uid") != os.getuid() or scope is None:
            continue
        root = highest_same_scope_ancestor(candidate, processes, scope)
        roots_by_identity[f"{root['pid']}:{root['starttime']}"] = root
    bitwig_roots = list(roots_by_identity.values())

    def ancestry(pid: int, root_pid: int) -> tuple[bool, list[dict[str, Any]]]:
        chain: list[dict[str, Any]] = []
        seen: set[int] = set()
        current = pid
        for _ in range(32):
            if current in seen:
                return False, chain
            seen.add(current)
            process = processes.get(current)
            if process is None:
                return False, chain
            chain.append(process)
            if current == root_pid:
                return True, chain
            if process["ppid"] <= 0 or process["ppid"] == current:
                return False, chain
            current = process["ppid"]
        return False, chain

    accepted_mappings: list[dict[str, Any]] = []
    rejected_mappings: list[dict[str, Any]] = []
    for mapping_process in mappings:
        matching_roots: list[tuple[dict[str, Any], list[dict[str, Any]]]] = []
        for root in bitwig_roots:
            belongs, chain = ancestry(mapping_process["pid"], root["pid"])
            same_scope = bitwig_cgroup_scope(str(mapping_process.get("cgroup", ""))) == bitwig_cgroup_scope(
                str(root.get("cgroup", ""))
            )
            if belongs and same_scope and mapping_process.get("uid") == root.get("uid"):
                matching_roots.append((root, chain))
        if len(matching_roots) == 1:
            root, chain = matching_roots[0]
            accepted_mappings.append({"process": mapping_process, "root": root, "chain": chain})
        else:
            rejected_mappings.append(
                {
                    "process": mapping_process,
                    "reason": "no_unique_verified_flatpak_app_tree_ancestry",
                    "candidate_root_count": len(bitwig_roots),
                }
            )

    return {
        "complete": table["complete"],
        "error": table["error"],
        "bitwig_candidates": bitwig_candidates,
        "bitwig_roots": bitwig_roots,
        "forbidden": forbidden,
        "validators": validators,
        "accepted_mappings": accepted_mappings,
        "rejected_mappings": rejected_mappings,
    }


def safe_process(process: dict[str, Any]) -> dict[str, Any]:
    return {
        "pid": process["pid"],
        "ppid": process["ppid"],
        "starttime": process["starttime"],
        "comm": process["comm"],
        "exe_basename": process["exe_basename"],
        "uid": process["uid"],
        "cgroup": process["cgroup"][:16384],
    }


def guard_processes(proc_root: pathlib.Path) -> dict[str, Any]:
    table = read_proc_table(proc_root)
    result = classify_processes(table)
    if not result["complete"]:
        fail("process guard is incomplete")
    return {
        "bitwig": [safe_process(item) for item in result["bitwig_candidates"]],
        "forbidden": [safe_process(item) | {"matched_name": item["matched_name"]} for item in result["forbidden"]],
        "validator": [safe_process(item) for item in result["validators"]],
    }


def require_empty_process_guard(proc_root: pathlib.Path) -> dict[str, Any]:
    result = guard_processes(proc_root)
    if result["bitwig"]:
        fail("Bitwig is already running")
    if result["forbidden"]:
        fail("forbidden Wine/Proton/UMU/yabridge process is running")
    if result["validator"]:
        fail("official validator process is running")
    return {"classification": "passed", "bitwig": 0, "forbidden": 0, "validator": 0}


def file_record(path: pathlib.Path) -> dict[str, Any]:
    validate_absolute_path(path, "fixture file", require_existing=True, regular_or_absent=True)
    if not path.is_file() or path.is_symlink():
        fail(f"fixture file is missing, non-regular, or a symlink: {path.name}")
    metadata = path.stat()
    return {
        "path": str(path),
        "size": metadata.st_size,
        "mtime_ns": metadata.st_mtime_ns,
        "mtime": command_text(["stat", "-c", "%y", str(path)], timeout=5),
        "sha256": sha256_file(path),
    }


def parse_flatpak_environment(data: str) -> dict[str, str]:
    section = None
    result: dict[str, str] = {}
    for line in data.splitlines():
        if line.startswith("[") and line.endswith("]"):
            section = line
            continue
        if section == "[Environment]" and "=" in line:
            key, value = line.split("=", 1)
            result[key] = value
    return result


def require_preedit_proof(proof_root: pathlib.Path) -> dict[str, Any]:
    require_descendant(proof_root, user_cache_root(), "pre-edit proof root", existing_directory=True)
    paths = {
        "publication": proof_root / "hp0-publication.inspect",
        "build": proof_root / "hp0-build.verify",
        "sandbox": proof_root / "hp0-sandbox.verify",
    }
    parsed: dict[str, dict[str, str]] = {}
    hashes: dict[str, str] = {}
    now = time.time()
    for role, path in paths.items():
        validate_absolute_path(path, f"pre-edit {role} proof", require_existing=True, regular_or_absent=True)
        if now - path.stat().st_mtime > 24 * 60 * 60 or path.stat().st_mtime > now + 5:
            fail(f"pre-edit {role} proof is stale or future-dated")
        data = path.read_bytes()
        parsed[role] = parse_key_values(data, f"pre-edit {role} proof")
        hashes[role] = sha256_bytes(data)

    require_exact(parsed["publication"], "publication_status", "verified", "HP0 publication proof")
    require_exact(parsed["publication"], "module_sha256", HP0_MODULE_SHA256, "HP0 publication proof")
    require_exact(parsed["publication"], "receipt_status", "verified_owned_receipt", "HP0 publication proof")
    require_exact(parsed["publication"], "receipt_schema", HP0_PUBLICATION_RECEIPT_SCHEMA, "HP0 publication proof")
    require_exact(parsed["publication"], "receipt_sha256", HP0_PUBLICATION_RECEIPT_SHA256, "HP0 publication proof")

    require_exact(parsed["build"], "build_receipt_status", "verified", "HP0 build proof")
    require_exact(parsed["build"], "current_repository_commit", EXPECTED_BASIS_COMMIT, "HP0 build proof")
    require_exact(parsed["build"], "current_repository_tree", EXPECTED_BASIS_TREE, "HP0 build proof")
    require_exact(parsed["build"], "build_source_manifest_schema", HP0_BUILD_SOURCE_SCHEMA, "HP0 build proof")
    require_exact(parsed["build"], "build_source_manifest_sha256", HP0_BUILD_SOURCE_SHA256, "HP0 build proof")

    for key, value in {
        "sandbox_status": "passed",
        "sandbox_validator_exit": "0",
        "build_source_manifest_schema": HP0_BUILD_SOURCE_SCHEMA,
        "build_source_manifest_sha256": HP0_BUILD_SOURCE_SHA256,
        "published_module_sha256": HP0_MODULE_SHA256,
        "published_manifest_sha256": HP0_BUNDLE_MANIFEST_SHA256,
        "sandbox_validator_sha256": HP0_VALIDATOR_SHA256,
        "bitwig_app_commit": BITWIG_APP_COMMIT,
        "bitwig_runtime_commit": BITWIG_RUNTIME_COMMIT,
        "bitwig_scope": "system",
        "effective_vst_path": BITWIG_VST_PATH,
        "effective_vst3_path": "empty",
        "effective_clap_path": "empty",
        "user_override_sha256": BITWIG_USER_OVERRIDE_SHA256,
        "system_override_sha256": BITWIG_SYSTEM_OVERRIDE_SHA256,
        "fixture_before_after": "byte_identical",
        "override_before_after": "byte_identical",
    }.items():
        require_exact(parsed["sandbox"], key, value, "HP0 sandbox proof")
    return {"classification": "passed", "sha256": hashes, "captured_before_editing": True}


def verify_bundle_manifest(bundle: pathlib.Path, manifest: pathlib.Path) -> None:
    validate_absolute_path(bundle, "HP0 build bundle", require_existing=True, require_directory=True)
    validate_absolute_path(
        manifest,
        "HP0 build bundle manifest",
        require_existing=True,
        regular_or_absent=True,
    )
    if not manifest.is_file() or manifest.is_symlink():
        fail("HP0 build bundle manifest is missing or a symlink")
    expected: dict[str, str] = {}
    for line in manifest.read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (\./.+)", line)
        if not match or match.group(2) in expected:
            fail("HP0 build bundle manifest is malformed")
        relative = match.group(2)[2:]
        candidate = bundle / relative
        if not candidate.is_file() or candidate.is_symlink():
            fail("HP0 build bundle manifest names a missing or unsafe file")
        if sha256_file(candidate) != match.group(1):
            fail("HP0 build bundle file hash differs from receipt manifest")
        expected[relative] = match.group(1)
    actual = sorted(
        str(path.relative_to(bundle))
        for path in bundle.rglob("*")
        if path.is_file() and not path.is_symlink()
    )
    if actual != sorted(expected):
        fail("HP0 build bundle roster differs from receipt manifest")


def capture_build_identity(repo: pathlib.Path, build_dir: pathlib.Path) -> dict[str, Any]:
    require_descendant(build_dir, repo / "build", "accepted HP0 build directory", existing_directory=True)
    receipt_path = build_dir / "hp0-build.receipt"
    validate_absolute_path(
        receipt_path,
        "HP0 build receipt",
        require_existing=True,
        regular_or_absent=True,
    )
    receipt = parse_key_values(receipt_path.read_bytes(), "HP0 build receipt")
    required = {
        "schema": "linux-vst-bridge-hp0-build/v2",
        "build_source_manifest_schema": HP0_BUILD_SOURCE_SCHEMA,
        "build_source_manifest_sha256": HP0_BUILD_SOURCE_SHA256,
        "sdk_ref": "runtime/org.freedesktop.Sdk/x86_64/25.08",
        "sdk_commit": "b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8",
        "vst3_sdk_commit": "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96",
        "processor_class_id": HP0_PROCESSOR_CID,
        "controller_class_id": HP0_CONTROLLER_CID,
        "gain_parameter_id": "0x4C485001",
        "bypass_parameter_id": "0x4C485002",
        "module_sha256": HP0_MODULE_SHA256,
        "bundle_manifest_sha256": HP0_BUNDLE_MANIFEST_SHA256,
        "validator_sha256": HP0_VALIDATOR_SHA256,
    }
    for key, value in required.items():
        require_exact(receipt, key, value, "HP0 build receipt")

    manifest_data = run_command(
        [str(repo / "tools/hp0-native-probe/build.sh"), "--print-source-manifest"],
        cwd=repo,
        timeout=30,
    ).stdout
    if sha256_bytes(manifest_data) != HP0_BUILD_SOURCE_SHA256:
        fail("current HP0 build-source manifest differs from the accepted identity")
    retained_manifest = build_dir / "hp0-build-source.manifest"
    validate_absolute_path(
        retained_manifest,
        "HP0 retained build-source manifest",
        require_existing=True,
        regular_or_absent=True,
    )
    if sha256_file(retained_manifest) != HP0_BUILD_SOURCE_SHA256 or retained_manifest.read_bytes() != manifest_data:
        fail("receipt-bound HP0 build-source manifest differs from the current exact manifest")

    bundle = pathlib.Path(receipt["bundle"])
    module = pathlib.Path(receipt["module"])
    validator = pathlib.Path(receipt["validator"])
    expected_bundle = build_dir / "VST3" / "Release" / HP0_BUNDLE
    if bundle != expected_bundle or module != bundle / HP0_MODULE_RELATIVE:
        fail("HP0 build receipt bundle/module path differs")
    validate_absolute_path(module, "HP0 build module", require_existing=True, regular_or_absent=True)
    validate_absolute_path(validator, "HP0 validator", require_existing=True, regular_or_absent=True)
    if not validator.is_file() or validator.is_symlink() or not os.access(validator, os.X_OK):
        fail("HP0 validator is missing, non-regular, symlinked, or non-executable")
    if sha256_file(module) != HP0_MODULE_SHA256 or sha256_file(validator) != HP0_VALIDATOR_SHA256:
        fail("HP0 build module or validator hash differs")
    bundle_manifest = build_dir / "hp0-built-bundle.sha256"
    validate_absolute_path(
        bundle_manifest,
        "HP0 build bundle manifest",
        require_existing=True,
        regular_or_absent=True,
    )
    if sha256_file(bundle_manifest) != HP0_BUNDLE_MANIFEST_SHA256:
        fail("HP0 build bundle-manifest identity differs")
    verify_bundle_manifest(bundle, bundle_manifest)
    return {
        "classification": "passed",
        "receipt_schema": receipt["schema"],
        "historical_repository_commit": receipt["historical_repository_commit"],
        "historical_repository_tree": receipt["historical_repository_tree"],
        "build_source_manifest_schema": HP0_BUILD_SOURCE_SCHEMA,
        "build_source_manifest_sha256": HP0_BUILD_SOURCE_SHA256,
        "module_sha256": HP0_MODULE_SHA256,
        "bundle_manifest_sha256": HP0_BUNDLE_MANIFEST_SHA256,
        "validator_sha256": HP0_VALIDATOR_SHA256,
        "build_dir": str(build_dir),
    }


def active_wayland_session() -> dict[str, Any]:
    listing = command_text(["loginctl", "list-sessions", "--no-legend"], timeout=10)
    observed = 0
    for line in listing.splitlines()[:64]:
        pieces = line.split()
        if not pieces:
            continue
        session_id = pieces[0]
        result = command_text(
            [
                "loginctl",
                "show-session",
                session_id,
                "-p",
                "Active",
                "-p",
                "Type",
                "-p",
                "Class",
                "-p",
                "Remote",
                "-p",
                "State",
            ],
            timeout=5,
        )
        values = dict(line.split("=", 1) for line in result.splitlines() if "=" in line)
        if (
            values.get("Active") == "yes"
            and values.get("Type") == "wayland"
            and values.get("Class") == "user"
            and values.get("Remote") == "no"
            and values.get("State") == "active"
        ):
            observed += 1
    if observed != 1:
        fail(f"expected exactly one active local KDE Wayland user session; observed {observed}")
    table = read_proc_table(pathlib.Path("/proc"))
    names = {name for process in table["processes"].values() for name in process_names(process)}
    if "plasmashell" not in names or "kwin_wayland" not in names:
        fail("active KDE Wayland session processes are unavailable")
    return {"classification": "passed", "desktop": "KDE", "type": "wayland", "active_local_sessions": 1}


def capture_bitwig_fixture() -> dict[str, Any]:
    shadow = run_command(["flatpak", "info", "--user", "--show-ref", BITWIG_APP_ID], check=False, timeout=10)
    if shadow.returncode == 0:
        fail("user-scope Bitwig shadow is present")
    app_ref = command_text(["flatpak", "info", "--system", "--show-ref", BITWIG_APP_ID], timeout=10)
    app_commit = command_text(["flatpak", "info", "--system", "--show-commit", BITWIG_APP_ID], timeout=10)
    runtime_ref = command_text(["flatpak", "info", "--system", "--show-runtime", BITWIG_APP_ID], timeout=10)
    runtime_commit = command_text(
        ["flatpak", "info", "--system", "--show-commit", BITWIG_RUNTIME_REF], timeout=10
    )
    app_info = command_text(["flatpak", "info", "--system", BITWIG_APP_ID], timeout=10)
    version_matches = re.findall(r"^[ \t]*Version:[ \t]*(.+)$", app_info, flags=re.MULTILINE)
    if len(version_matches) != 1:
        fail("Bitwig version could not be read exactly")
    user_override = run_command(["flatpak", "override", "--user", "--show", BITWIG_APP_ID], timeout=10).stdout
    system_override = run_command(["flatpak", "override", "--system", "--show", BITWIG_APP_ID], timeout=10).stdout
    permissions = command_text(["flatpak", "info", "--show-permissions", BITWIG_APP_ID], timeout=10)
    environment = parse_flatpak_environment(permissions)
    fixture = {
        "app_id": BITWIG_APP_ID,
        "app_ref": app_ref,
        "version": version_matches[0],
        "app_commit": app_commit,
        "scope": "system",
        "user_scope_shadow": "absent",
        "runtime_ref": runtime_ref,
        "runtime_commit": runtime_commit,
        "runtime_scope": "system",
        "user_override_sha256": sha256_bytes(user_override),
        "system_override_sha256": sha256_bytes(system_override),
        "effective_vst_path": environment.get("VST_PATH"),
        "effective_vst3_path": environment.get("VST3_PATH"),
        "effective_clap_path": environment.get("CLAP_PATH"),
        "user_override_bytes": user_override.decode("utf-8", "strict"),
        "system_override_bytes": system_override.decode("utf-8", "strict"),
    }
    check_bitwig_fixture(fixture)
    return fixture


def check_bitwig_fixture(fixture: dict[str, Any]) -> None:
    expected = {
        "app_id": BITWIG_APP_ID,
        "app_ref": BITWIG_APP_REF,
        "version": BITWIG_VERSION,
        "app_commit": BITWIG_APP_COMMIT,
        "scope": "system",
        "user_scope_shadow": "absent",
        "runtime_ref": BITWIG_RUNTIME_REF,
        "runtime_commit": BITWIG_RUNTIME_COMMIT,
        "runtime_scope": "system",
        "user_override_sha256": BITWIG_USER_OVERRIDE_SHA256,
        "system_override_sha256": BITWIG_SYSTEM_OVERRIDE_SHA256,
        "effective_vst_path": BITWIG_VST_PATH,
        "effective_vst3_path": "",
        "effective_clap_path": "",
    }
    for key, value in expected.items():
        if fixture.get(key) != value:
            fail(f"Bitwig fixture differs for {key}: expected {value!r}, observed {fixture.get(key)!r}")


def capture_live_fixture(repo: pathlib.Path, build_dir: pathlib.Path, proof_root: pathlib.Path) -> dict[str, Any]:
    repo = pathlib.Path(os.path.realpath(repo))
    if repo != EXPECTED_REPO:
        fail(f"repository path differs: expected {EXPECTED_REPO}, observed {repo}")
    validate_absolute_path(repo, "repository", require_existing=True, require_directory=True)
    branch = command_text(["git", "branch", "--show-current"], cwd=repo)
    head = command_text(["git", "rev-parse", "HEAD"], cwd=repo)
    tree = command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=repo)
    origin_main = command_text(["git", "rev-parse", "origin/main"], cwd=repo)
    origin_tree = command_text(["git", "rev-parse", "origin/main^{tree}"], cwd=repo)
    if (branch, head, tree, origin_main, origin_tree) != (
        EXPECTED_BRANCH,
        EXPECTED_BASIS_COMMIT,
        EXPECTED_BASIS_TREE,
        EXPECTED_BASIS_COMMIT,
        EXPECTED_BASIS_TREE,
    ):
        fail("repository branch, HEAD, tree, or origin/main differs from the exact HP1 basis")
    remote = command_text(["git", "remote", "get-url", "origin"], cwd=repo)
    if remote not in {
        "git@github.com:kasselvania/Linux-VST-bridge.git",
        "https://github.com/kasselvania/Linux-VST-bridge.git",
    }:
        fail("repository origin is not kasselvania/Linux-VST-bridge")
    status = command_text(["git", "status", "--porcelain=v1", "--untracked-files=all"], cwd=repo)
    allowed = ("CURRENT_SLICE.md", "tools/hp1-bitwig-admission/", "evidence/hp1-bitwig-admission/")
    status_count = 0
    for line in status.splitlines():
        if len(line) < 4:
            fail("malformed Git status output")
        path = line[3:]
        if " -> " in path:
            fail("renamed paths are not accepted during HP1 preflight")
        if path != allowed[0] and not path.startswith(allowed[1:]):
            fail(f"worktree contains an out-of-envelope change: {path}")
        status_count += 1

    os_release: dict[str, str] = {}
    for line in pathlib.Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            os_release[key] = value.strip('"')
    model = ""
    for candidate in (
        pathlib.Path("/sys/devices/virtual/dmi/id/product_name"),
        pathlib.Path("/sys/class/dmi/id/product_name"),
    ):
        if candidate.is_file():
            model = candidate.read_text(encoding="utf-8").strip()
            break
    if os_release.get("NAME") != "SteamOS" or os_release.get("VERSION_ID") != "3.8.16" or model != "Galileo":
        fail("host is not the exact accepted Steam Deck fixture")
    if os.uname().machine != "x86_64":
        fail("host architecture is not x86_64")

    guard = require_empty_process_guard(pathlib.Path("/proc"))
    session = active_wayland_session()
    proof = require_preedit_proof(proof_root)

    publication_output = run_command(
        [str(repo / "tools/hp0-native-probe/publish.sh"), "inspect"], cwd=repo, timeout=60
    ).stdout
    publication = parse_key_values(publication_output, "current HP0 publication inspection")
    require_exact(publication, "publication_status", "verified", "current HP0 publication")
    require_exact(publication, "module_sha256", HP0_MODULE_SHA256, "current HP0 publication")
    require_exact(publication, "receipt_status", "verified_owned_receipt", "current HP0 publication")
    require_exact(publication, "receipt_schema", HP0_PUBLICATION_RECEIPT_SCHEMA, "current HP0 publication")
    require_exact(publication, "receipt_sha256", HP0_PUBLICATION_RECEIPT_SHA256, "current HP0 publication")
    home = real_home()
    publication_root = home / ".vst3" / "linux-vst-bridge" / HP0_BUNDLE
    module = publication_root / HP0_MODULE_RELATIVE
    receipt_path = user_cache_root() / "linux-vst-bridge" / "hp0-publication.receipt"
    module_record = file_record(module)
    receipt_record = file_record(receipt_path)
    if module_record["sha256"] != HP0_MODULE_SHA256:
        fail("published HP0 module hash differs")
    if receipt_record["sha256"] != HP0_PUBLICATION_RECEIPT_SHA256:
        fail("ordinary HP0 publication receipt hash differs")

    build = capture_build_identity(repo, build_dir)
    bitwig = capture_bitwig_fixture()
    serum = []
    for expected in SERUM_FIXTURES:
        record = file_record(home / expected["relative"])
        record["role"] = expected["role"]
        if (
            record["sha256"] != expected["sha256"]
            or record["size"] != expected["size"]
            or record["mtime"] != expected["mtime"]
        ):
            fail(f"accepted SR0 Serum artifact drifted: {expected['role']}")
        serum.append(record)

    fixture = {
        "schema": SCHEMA_FIXTURE,
        "captured_at": utc_now(),
        "repository": {
            "path": str(repo),
            "branch": branch,
            "head": head,
            "tree": tree,
            "origin_main": origin_main,
            "origin_main_tree": origin_tree,
            "origin_repository": "kasselvania/Linux-VST-bridge",
            "allowed_dirty_entry_count": status_count,
        },
        "host": {
            "device": "Steam Deck",
            "model": model,
            "os": os_release.get("NAME"),
            "version_id": os_release.get("VERSION_ID"),
            "build_id": os_release.get("BUILD_ID"),
            "architecture": os.uname().machine,
            "kernel_release": os.uname().release,
            "hostname": "not_retained",
        },
        "graphical_session": session,
        "process_guard": guard,
        "hp0": {
            "preedit_accepted_readbacks": proof,
            "publication": {
                "status": publication["publication_status"],
                "bundle": str(publication_root),
                "module": module_record,
                "source_manifest_sha256": publication["source_manifest_sha256"],
                "receipt_status": publication["receipt_status"],
                "receipt_schema": publication["receipt_schema"],
                "receipt": receipt_record,
            },
            "build": build,
        },
        "bitwig": bitwig,
        "serum": serum,
    }
    verify_fixture_snapshot(fixture)
    return fixture


def verify_fixture_snapshot(fixture: dict[str, Any]) -> None:
    if fixture.get("schema") != SCHEMA_FIXTURE:
        fail("fixture snapshot schema differs")
    repository = fixture.get("repository", {})
    expected_repo = {
        "path": str(EXPECTED_REPO),
        "branch": EXPECTED_BRANCH,
        "head": EXPECTED_BASIS_COMMIT,
        "tree": EXPECTED_BASIS_TREE,
        "origin_main": EXPECTED_BASIS_COMMIT,
        "origin_main_tree": EXPECTED_BASIS_TREE,
        "origin_repository": "kasselvania/Linux-VST-bridge",
    }
    for key, value in expected_repo.items():
        if repository.get(key) != value:
            fail(f"fixture repository differs for {key}")
    hp0 = fixture.get("hp0", {})
    preedit = hp0.get("preedit_accepted_readbacks", {})
    preedit_hashes = preedit.get("sha256", {})
    if (
        preedit.get("classification") != "passed"
        or preedit.get("captured_before_editing") is not True
        or set(preedit_hashes) != {"build", "publication", "sandbox"}
        or any(
            re.fullmatch(r"[0-9a-f]{64}", str(value)) is None
            for value in preedit_hashes.values()
        )
    ):
        fail("fixture pre-edit HP0 readback proof is missing or invalid")
    publication = hp0.get("publication", {})
    module = publication.get("module", {})
    receipt = publication.get("receipt", {})
    if module.get("sha256") != HP0_MODULE_SHA256:
        fail("fixture HP0 module hash differs")
    if publication.get("receipt_status") != "verified_owned_receipt":
        fail("fixture HP0 publication receipt is missing or invalid")
    if publication.get("receipt_schema") != HP0_PUBLICATION_RECEIPT_SCHEMA:
        fail("fixture HP0 publication receipt schema differs")
    if receipt.get("sha256") != HP0_PUBLICATION_RECEIPT_SHA256:
        fail("fixture HP0 publication receipt hash differs")
    build = hp0.get("build", {})
    if (
        build.get("build_source_manifest_schema") != HP0_BUILD_SOURCE_SCHEMA
        or build.get("build_source_manifest_sha256") != HP0_BUILD_SOURCE_SHA256
        or build.get("module_sha256") != HP0_MODULE_SHA256
        or build.get("validator_sha256") != HP0_VALIDATOR_SHA256
    ):
        fail("fixture HP0 build identity differs")
    check_bitwig_fixture(fixture.get("bitwig", {}))
    serum = fixture.get("serum", [])
    if len(serum) != len(SERUM_FIXTURES):
        fail("fixture Serum artifact roster differs")
    by_role = {record.get("role"): record for record in serum}
    for expected in SERUM_FIXTURES:
        record = by_role.get(expected["role"], {})
        if (
            record.get("sha256") != expected["sha256"]
            or record.get("size") != expected["size"]
            or record.get("mtime") != expected["mtime"]
        ):
            fail(f"fixture Serum artifact identity differs: {expected['role']}")


def compare_fixture_snapshots(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    verify_fixture_snapshot(before)
    verify_fixture_snapshot(after)
    paths = (
        ("hp0.publication.module.sha256", before["hp0"]["publication"]["module"]["sha256"], after["hp0"]["publication"]["module"]["sha256"]),
        ("hp0.publication.module.size", before["hp0"]["publication"]["module"]["size"], after["hp0"]["publication"]["module"]["size"]),
        ("hp0.publication.module.mtime", before["hp0"]["publication"]["module"]["mtime"], after["hp0"]["publication"]["module"]["mtime"]),
        ("hp0.publication.receipt.sha256", before["hp0"]["publication"]["receipt"]["sha256"], after["hp0"]["publication"]["receipt"]["sha256"]),
        ("hp0.publication.receipt.size", before["hp0"]["publication"]["receipt"]["size"], after["hp0"]["publication"]["receipt"]["size"]),
        ("hp0.publication.receipt.mtime", before["hp0"]["publication"]["receipt"]["mtime"], after["hp0"]["publication"]["receipt"]["mtime"]),
        ("hp0.preedit.classification", before["hp0"]["preedit_accepted_readbacks"]["classification"], after["hp0"]["preedit_accepted_readbacks"]["classification"]),
        ("hp0.preedit.captured_before_editing", before["hp0"]["preedit_accepted_readbacks"]["captured_before_editing"], after["hp0"]["preedit_accepted_readbacks"]["captured_before_editing"]),
        ("hp0.preedit.build", before["hp0"]["preedit_accepted_readbacks"]["sha256"]["build"], after["hp0"]["preedit_accepted_readbacks"]["sha256"]["build"]),
        ("hp0.preedit.publication", before["hp0"]["preedit_accepted_readbacks"]["sha256"]["publication"], after["hp0"]["preedit_accepted_readbacks"]["sha256"]["publication"]),
        ("hp0.preedit.sandbox", before["hp0"]["preedit_accepted_readbacks"]["sha256"]["sandbox"], after["hp0"]["preedit_accepted_readbacks"]["sha256"]["sandbox"]),
        ("bitwig.app_commit", before["bitwig"]["app_commit"], after["bitwig"]["app_commit"]),
        ("bitwig.runtime_commit", before["bitwig"]["runtime_commit"], after["bitwig"]["runtime_commit"]),
        ("bitwig.user_override", before["bitwig"]["user_override_sha256"], after["bitwig"]["user_override_sha256"]),
        ("bitwig.system_override", before["bitwig"]["system_override_sha256"], after["bitwig"]["system_override_sha256"]),
        ("bitwig.user_override_bytes", before["bitwig"]["user_override_bytes"], after["bitwig"]["user_override_bytes"]),
        ("bitwig.system_override_bytes", before["bitwig"]["system_override_bytes"], after["bitwig"]["system_override_bytes"]),
    )
    drift = [name for name, old, new in paths if old != new]
    before_serum = {record["role"]: (record["sha256"], record["size"], record["mtime"]) for record in before["serum"]}
    after_serum = {record["role"]: (record["sha256"], record["size"], record["mtime"]) for record in after["serum"]}
    if before_serum != after_serum:
        drift.append("serum")
    if drift:
        fail("before/after fixture preservation failed: " + ", ".join(drift))
    return {"classification": "passed", "drift": [], "byte_identical_overrides": True}


def create_session() -> dict[str, Any]:
    cache = user_cache_root()
    sessions = cache / "linux-vst-bridge" / "hp1" / "sessions"
    mkdir_descendant(sessions, cache, "HP1 sessions root")
    for _ in range(20):
        stamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        session_id = f"hp1-{stamp}-{secrets.token_hex(8)}"
        root = sessions / session_id
        try:
            root.mkdir(mode=0o700)
            break
        except FileExistsError:
            continue
    else:
        fail("could not allocate a unique HP1 session root")
    require_descendant(root, cache, "HP1 session root", existing_directory=True)
    nonce = secrets.token_hex(16)
    meta = {
        "schema": SCHEMA_SESSION,
        "session_id": session_id,
        "nonce": nonce,
        "nonce_sha256": sha256_bytes(nonce.encode("ascii")),
        "created_at": utc_now(),
        "created_at_ns": time.time_ns(),
        "status": "created",
        "basis_commit": EXPECTED_BASIS_COMMIT,
        "basis_tree": EXPECTED_BASIS_TREE,
        "branch": EXPECTED_BRANCH,
    }
    atomic_write_json(root / "session.json", meta)
    return {"session_id": session_id, "nonce": nonce, "session_root": str(root)}


def candidate_roots(home: pathlib.Path) -> list[dict[str, Any]]:
    return [
        {"label": "bitwig_log", "path": home / ".BitwigStudio" / "log", "all_files": True, "max_depth": 2},
        {"label": "bitwig_index", "path": home / ".BitwigStudio" / "index", "all_files": True, "max_depth": 3},
        {"label": "bitwig_cache", "path": home / ".BitwigStudio" / "cache", "all_files": False, "max_depth": 3},
        {"label": "bitwig_prefs", "path": home / ".BitwigStudio" / "prefs", "all_files": False, "max_depth": 3},
        {"label": "flatpak_data", "path": home / ".var" / "app" / BITWIG_APP_ID / "data", "all_files": False, "max_depth": 5},
        {"label": "flatpak_config", "path": home / ".var" / "app" / BITWIG_APP_ID / "config", "all_files": False, "max_depth": 5},
        {
            "label": "flatpak_cache",
            "path": home / ".var" / "app" / BITWIG_APP_ID / "cache",
            "all_files": False,
            "max_depth": 4,
            "skip_directory_names": (
                "JNA",
                "fontconfig",
                "mesa_shader_cache_db",
                "radv_builtin_shaders",
                "tmp",
            ),
        },
    ]


def relevant_name(name: str) -> bool:
    return bool(re.search(r"(?i)(log|plug[-_ ]?in|vst3?|cache|index|database|\.db$|\.ids$)", name))


def scan_root(
    root_spec: dict[str, Any],
    *,
    deadline: float,
    max_candidates: int,
    candidates: list[dict[str, Any]],
) -> tuple[bool, str | None]:
    root: pathlib.Path = root_spec["path"]
    if not root.exists():
        return True, "root_absent"
    validate_absolute_path(root, f"Bitwig evidence root {root_spec['label']}", require_existing=True, require_directory=True)
    root_device = root.stat().st_dev
    stack: list[tuple[pathlib.Path, int]] = [(root, 0)]
    while stack:
        if time.monotonic() > deadline:
            return False, "timeout"
        directory, depth = stack.pop()
        try:
            entries = sorted(os.scandir(directory), key=lambda item: item.name)
        except (OSError, PermissionError):
            return False, "inaccessible_directory"
        for entry in entries:
            if time.monotonic() > deadline:
                return False, "timeout"
            try:
                if entry.is_symlink():
                    return False, "symlink_encountered"
                metadata = entry.stat(follow_symlinks=False)
            except OSError:
                return False, "stat_failed"
            if metadata.st_dev != root_device:
                continue
            path = pathlib.Path(entry.path)
            if stat.S_ISDIR(metadata.st_mode):
                if entry.name in root_spec.get("skip_directory_names", ()):
                    continue
                if depth < root_spec["max_depth"]:
                    if root_spec["label"].startswith("flatpak_") and path.name == "Rack2":
                        continue
                    if root_spec["label"] == "bitwig_index" and path.name == "plugin_presets":
                        continue
                    stack.append((path, depth + 1))
                continue
            if not stat.S_ISREG(metadata.st_mode):
                continue
            if not root_spec["all_files"] and not relevant_name(entry.name):
                continue
            candidates.append(
                {
                    "root_label": root_spec["label"],
                    "root": str(root),
                    "path": str(path),
                    "relative_path": str(path.relative_to(root)),
                    "size": metadata.st_size,
                    "mtime_ns": metadata.st_mtime_ns,
                    "mode": stat.S_IMODE(metadata.st_mode),
                }
            )
            if len(candidates) > max_candidates:
                return False, "candidate_cap"
    return True, None


def inspect_candidate_content(
    candidate: dict[str, Any],
    *,
    remaining_bytes: int,
    max_file_bytes: int,
) -> tuple[dict[str, Any], int, str | None]:
    path = pathlib.Path(candidate["path"])
    size = candidate["size"]
    result = dict(candidate)
    if size > max_file_bytes:
        result["content_classification"] = "unknown_file_too_large"
        return result, remaining_bytes, "file_size_cap"
    if size > remaining_bytes:
        result["content_classification"] = "unknown_total_byte_cap"
        return result, remaining_bytes, "total_byte_cap"
    try:
        data = path.read_bytes()
    except OSError:
        result["content_classification"] = "unknown_read_failed"
        return result, remaining_bytes, "read_failed"
    remaining_bytes -= len(data)
    result["sha256"] = sha256_bytes(data)
    strings = []
    for token in DISCOVERY_TOKENS:
        token_bytes = token.encode("utf-8")
        if token_bytes in data:
            strings.append({"value": token, "classification": "exact_string_present"})
    result["matching_strings"] = strings
    error_terms = re.compile(rb"(?i)(error|failed|failure|reject|cannot|unable)")
    matching_token_bytes = tuple(
        item["value"].encode("utf-8") for item in strings
    )
    result["applicable_scan_error"] = any(
        any(token in line for token in matching_token_bytes)
        and error_terms.search(line) is not None
        for line in data.splitlines()
    )
    result["applicable_scan_error_basis"] = (
        "same_line_exact_discovery_token_and_error_term"
    )
    result["content_classification"] = "bounded_exact_string_scan_complete"
    return result, remaining_bytes, None


def scan_state(
    output: pathlib.Path,
    *,
    baseline_path: pathlib.Path | None = None,
    test_root: pathlib.Path | None = None,
    roots: list[pathlib.Path] | None = None,
    max_candidates: int = 256,
    max_file_bytes: int = 4 * 1024 * 1024,
    max_total_bytes: int = 16 * 1024 * 1024,
    timeout_seconds: float = 15.0,
) -> dict[str, Any]:
    if test_root is None:
        root_specs = candidate_roots(real_home())
    else:
        require_descendant(test_root, user_cache_root(), "state-scan test root", existing_directory=True)
        if not roots:
            fail("test state scan requires at least one explicit root")
        root_specs = []
        for index, root in enumerate(roots):
            require_descendant(root, test_root, "test evidence root", existing_directory=True)
            root_specs.append({"label": f"test_root_{index}", "path": root, "all_files": True, "max_depth": 6})
    if baseline_path is not None:
        require_descendant(
            baseline_path,
            test_root or user_cache_root(),
            "state-scan baseline",
            regular_or_absent=True,
        )
        if not baseline_path.is_file() or baseline_path.is_symlink():
            fail("state-scan baseline is missing or unsafe")
    baseline: dict[str, Any] | None = read_json(baseline_path) if baseline_path else None
    if baseline is not None and baseline.get("schema") != SCHEMA_STATE_SCAN:
        fail("state-scan baseline schema differs")
    baseline_by_path = {item["path"]: item for item in baseline.get("candidates", [])} if baseline else {}

    candidates: list[dict[str, Any]] = []
    root_status: list[dict[str, Any]] = []
    complete = True
    incomplete_reasons: list[str] = []
    deadline = time.monotonic() + timeout_seconds
    for spec in root_specs:
        if time.monotonic() > deadline:
            root_complete, reason = False, "timeout"
        else:
            root_complete, reason = scan_root(
                spec, deadline=deadline, max_candidates=max_candidates, candidates=candidates
            )
        root_status.append(
            {
                "label": spec["label"],
                "path": str(spec["path"]),
                "classification": "observed" if root_complete else "unknown",
                "status": "completed" if root_complete else "search_incomplete",
                "detail": reason or "completed",
            }
        )
        if not root_complete:
            complete = False
            incomplete_reasons.append(f"{spec['label']}:{reason}")
            if reason in {"candidate_cap", "timeout"}:
                break
    retained = sorted(candidates[:max_candidates], key=lambda item: (item["root_label"], item["relative_path"]))
    inspected: list[dict[str, Any]] = []
    remaining = max_total_bytes
    for candidate in retained:
        previous = baseline_by_path.get(candidate["path"])
        candidate["change"] = (
            "new"
            if baseline is not None and previous is None
            else "modified"
            if baseline is not None
            and (candidate["size"], candidate["mtime_ns"]) != (previous.get("size"), previous.get("mtime_ns"))
            else "unchanged"
            if baseline is not None
            else "baseline"
        )
        inspect_content = baseline is None or candidate["change"] in {"new", "modified"}
        if inspect_content:
            inspected_item, remaining, reason = inspect_candidate_content(
                candidate, remaining_bytes=remaining, max_file_bytes=max_file_bytes
            )
            if reason:
                complete = False
                incomplete_reasons.append(f"{candidate['root_label']}:{reason}")
            inspected.append(inspected_item)
        else:
            inspected.append(candidate)
    result = {
        "schema": SCHEMA_STATE_SCAN,
        "captured_at": utc_now(),
        "classification": "observed" if complete else "unknown",
        "status": "completed" if complete else "search_incomplete",
        "limits": {
            "max_candidates": max_candidates,
            "max_file_bytes": max_file_bytes,
            "max_total_bytes": max_total_bytes,
            "timeout_seconds": timeout_seconds,
        },
        "candidate_count": len(inspected),
        "candidate_cap_exhausted": len(candidates) > max_candidates,
        "incomplete_reasons": sorted(set(incomplete_reasons)),
        "roots": root_status,
        "candidates": inspected,
    }
    require_descendant(output, test_root or user_cache_root(), "state scan output", regular_or_absent=True)
    atomic_write_json(output, result)
    return result


def identity(process: dict[str, Any]) -> str:
    return f"{process['pid']}:{process['starttime']}"


def ancestry_chain(
    processes: dict[int, dict[str, Any]], child_pid: int, root_pid: int
) -> tuple[bool, list[dict[str, Any]]]:
    chain: list[dict[str, Any]] = []
    seen: set[int] = set()
    current = child_pid
    for _ in range(32):
        if current in seen:
            return False, chain
        seen.add(current)
        process = processes.get(current)
        if process is None:
            return False, chain
        chain.append(process)
        if current == root_pid:
            return True, chain
        if process["ppid"] <= 0 or process["ppid"] == current:
            return False, chain
        current = process["ppid"]
    return False, chain


def choose_bitwig_root(classified: dict[str, Any], processes: dict[int, dict[str, Any]]) -> dict[str, Any] | None:
    roots = classified["bitwig_roots"]
    top_level: list[dict[str, Any]] = []
    for candidate in roots:
        nested = False
        for possible_parent in roots:
            if candidate["pid"] == possible_parent["pid"]:
                continue
            belongs, _ = ancestry_chain(processes, candidate["pid"], possible_parent["pid"])
            if belongs:
                nested = True
                break
        if not nested:
            top_level.append(candidate)
    if not top_level:
        return None
    if len(top_level) != 1:
        fail(f"ambiguous Bitwig process roots: observed {len(top_level)}")
    return top_level[0]


def append_event(path: pathlib.Path, event: dict[str, Any]) -> None:
    data = (json.dumps(event, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")
    with path.open("ab", buffering=0) as handle:
        handle.write(data)
        os.fsync(handle.fileno())


def sanitized_cgroup_identity(cgroup: str) -> str:
    return "bitwig_flatpak_scope_observed" if BITWIG_APP_ID in cgroup else "bitwig_flatpak_scope_not_observed"


def monitor_session(session_root: pathlib.Path, proc_root: pathlib.Path, poll_seconds: float, max_seconds: float) -> None:
    require_descendant(session_root, user_cache_root(), "HP1 session root", existing_directory=True)
    meta_path = session_root / "session.json"
    meta = read_json(meta_path)
    if meta.get("schema") != SCHEMA_SESSION or not SESSION_ID_RE.fullmatch(str(meta.get("session_id", ""))):
        fail("session metadata is invalid")
    fixture = read_json(session_root / "fixture.before.json")
    verify_fixture_snapshot(fixture)
    module = pathlib.Path(fixture["hp0"]["publication"]["module"]["path"])
    if sha256_file(module) != HP0_MODULE_SHA256:
        fail("monitor start refused a changed HP0 module")
    require_empty_process_guard(proc_root)

    lock_path = session_root / "monitor.lock"
    try:
        lock_fd = os.open(lock_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError:
        fail("session monitor lock already exists")
    os.write(lock_fd, f"{os.getpid()}\n".encode("ascii"))
    os.close(lock_fd)

    events_path = session_root / "monitor.events.jsonl"
    state_path = session_root / "monitor.state.json"
    state: dict[str, Any] = {
        "schema": SCHEMA_MONITOR,
        "session_id": meta["session_id"],
        "status": "waiting_session_1",
        "started_at": utc_now(),
        "poll_seconds": poll_seconds,
        "max_seconds": max_seconds,
        "sessions": [],
        "forbidden_process_observed": False,
        "validator_observed": False,
        "unrelated_mapping_observed": False,
        "last_update": utc_now(),
    }
    atomic_write_json(state_path, state)
    append_event(events_path, {"event": "monitor_started", "at": utc_now(), "status": state["status"]})
    meta["status"] = "monitoring"
    meta["monitor_started_at"] = state["started_at"]
    atomic_write_json(meta_path, meta)

    current: dict[str, Any] | None = None
    root_missing_since: float | None = None
    contamination_reason: str | None = None
    contamination_identities: set[str] = set()
    started = time.monotonic()

    def save_state() -> None:
        state["last_update"] = utc_now()
        atomic_write_json(state_path, state)

    def block(reason: str, detail: dict[str, Any] | None = None) -> None:
        state["status"] = reason
        state["blocked_detail"] = detail or {}
        append_event(events_path, {"event": "monitor_blocked", "at": utc_now(), "reason": reason})
        save_state()

    def begin_contamination(reason: str, detail: dict[str, Any] | None = None) -> None:
        nonlocal contamination_reason
        if contamination_reason is not None:
            return
        contamination_reason = reason
        state["status"] = f"{reason}_waiting_quit"
        state["blocked_detail"] = detail or {}
        append_event(
            events_path,
            {"event": "monitor_contaminated", "at": utc_now(), "reason": reason},
        )
        print("operator_action=quit_bitwig_normally_now", flush=True)
        save_state()

    try:
        while True:
            if time.monotonic() - started > max_seconds:
                if contamination_reason is not None:
                    block(f"{contamination_reason}_cleanup_timeout", state.get("blocked_detail"))
                else:
                    block("blocked_monitor_timeout")
                return
            table = read_proc_table(proc_root, module)
            if not table["complete"]:
                block("blocked_process_census_incomplete", {"reason": table["error"]})
                return
            classified = classify_processes(table, module)
            processes = table["processes"]

            if classified["forbidden"]:
                state["forbidden_process_observed"] = True
                detail = classified["forbidden"][0]
                begin_contamination(
                    "blocked_forbidden_process",
                    {
                        "basename": detail["matched_name"],
                        "starttime": detail["starttime"],
                        "bitwig_ancestry": any(
                            ancestry_chain(processes, detail["pid"], root["pid"])[0]
                            for root in classified["bitwig_roots"]
                        ),
                    },
                )
            if classified["validators"]:
                state["validator_observed"] = True
                begin_contamination("blocked_validator_process")
            if classified["rejected_mappings"]:
                state["unrelated_mapping_observed"] = True
                rejected = classified["rejected_mappings"][0]
                begin_contamination(
                    "blocked_unrelated_module_mapping",
                    {
                        "reason": rejected["reason"],
                        "candidate_root_count": rejected["candidate_root_count"],
                        "mapping_process": safe_process(rejected["process"]),
                        "matching_map_entry": rejected["process"].get("mapping"),
                        "verified_app_roots": [safe_process(root) for root in classified["bitwig_roots"]],
                    },
                )

            if classified["bitwig_candidates"] and not classified["bitwig_roots"]:
                begin_contamination("blocked_unverified_bitwig_flatpak_ancestry")

            if contamination_reason is not None:
                for root in classified["bitwig_roots"]:
                    for process in processes.values():
                        belongs, _ = ancestry_chain(processes, process["pid"], root["pid"])
                        if belongs:
                            contamination_identities.add(identity(process))
                if current is not None:
                    contamination_identities.add(current["root_identity"])
                    contamination_identities.update(current["observed_descendants"])
                live_contaminated = [
                    process
                    for process in processes.values()
                    if identity(process) in contamination_identities
                ]
                any_mapping = any(process.get("mapping") is not None for process in processes.values())
                clean = (
                    not classified["bitwig_candidates"]
                    and not classified["forbidden"]
                    and not classified["validators"]
                    and not any_mapping
                    and not live_contaminated
                )
                if clean:
                    state["status"] = f"{contamination_reason}_clean_exit"
                    state["contamination_clean_exit"] = {
                        "captured_at": utc_now(),
                        "bitwig_processes": 0,
                        "observed_descendants": 0,
                        "exact_module_mappings": 0,
                        "forbidden_processes": 0,
                        "validator_processes": 0,
                    }
                    append_event(
                        events_path,
                        {
                            "event": "monitor_contamination_clean_exit",
                            "at": state["contamination_clean_exit"]["captured_at"],
                            "reason": contamination_reason,
                        },
                    )
                    save_state()
                    return
                time.sleep(poll_seconds)
                continue
            selected_root = choose_bitwig_root(classified, processes)

            if current is None and selected_root is not None:
                session_number = len(state["sessions"]) + 1
                if session_number > 2:
                    block("blocked_unexpected_third_bitwig_session")
                    return
                new_identity = identity(selected_root)
                if state["sessions"] and new_identity == state["sessions"][0]["root_identity"]:
                    block("blocked_reused_bitwig_process_identity")
                    return
                current = {
                    "number": session_number,
                    "root_identity": new_identity,
                    "root": safe_process(selected_root),
                    "root_cgroup_classification": sanitized_cgroup_identity(selected_root["cgroup"]),
                    "launched_at": utc_now(),
                    "mapping": None,
                    "observed_descendants": {},
                    "clean_shutdown": False,
                }
                state["sessions"].append(current)
                state["status"] = f"session_{session_number}_active_waiting_mapping"
                append_event(
                    events_path,
                    {
                        "event": "bitwig_session_started",
                        "at": current["launched_at"],
                        "session": session_number,
                        "root_identity": new_identity,
                    },
                )
                print(f"session_{session_number}_started=true", flush=True)
                save_state()

            if current is not None:
                root_pid = current["root"]["pid"]
                root_process = processes.get(root_pid)
                root_alive = root_process is not None and identity(root_process) == current["root_identity"]
                if root_process is not None and identity(root_process) != current["root_identity"]:
                    block("blocked_stale_bitwig_pid_reuse")
                    return

                if root_alive:
                    root_missing_since = None
                    for process in processes.values():
                        belongs, chain = ancestry_chain(processes, process["pid"], root_pid)
                        if not belongs:
                            continue
                        if process["pid"] != root_pid:
                            current["observed_descendants"][identity(process)] = safe_process(process)
                        if process.get("mapping") is None:
                            continue
                        if current["mapping"] is None:
                            if sha256_file(module) != HP0_MODULE_SHA256:
                                block("blocked_module_hash_drift")
                                return
                            metadata = module.stat()
                            current["mapping"] = {
                                "captured_at": utc_now(),
                                "process": safe_process(process),
                                "process_identity": identity(process),
                                "ancestry": [safe_process(item) for item in chain],
                                "mapping": process["mapping"],
                                "module_sha256": HP0_MODULE_SHA256,
                                "module_device": f"{os.major(metadata.st_dev):x}:{os.minor(metadata.st_dev):x}",
                                "module_inode": metadata.st_ino,
                                "bitwig_ancestry": True,
                            }
                            state["status"] = f"session_{current['number']}_mapping_captured_waiting_quit"
                            append_event(
                                events_path,
                                {
                                    "event": "exact_module_mapping_captured",
                                    "at": current["mapping"]["captured_at"],
                                    "session": current["number"],
                                    "root_identity": current["root_identity"],
                                    "mapping_process_identity": current["mapping"]["process_identity"],
                                },
                            )
                            print(f"session_{current['number']}_mapping=captured", flush=True)
                            save_state()
                else:
                    if root_missing_since is None:
                        root_missing_since = time.monotonic()
                    live_observed_descendants = []
                    for process in processes.values():
                        if identity(process) in current["observed_descendants"]:
                            live_observed_descendants.append(process)
                    any_mapping = any(process.get("mapping") is not None for process in processes.values())
                    if not live_observed_descendants and not classified["bitwig_candidates"] and not any_mapping:
                        current["clean_shutdown"] = True
                        current["shutdown_at"] = utc_now()
                        current["post_shutdown"] = {
                            "bitwig_processes": 0,
                            "observed_descendants": 0,
                            "exact_module_mappings": 0,
                            "forbidden_processes": 0,
                            "validator_processes": 0,
                        }
                        append_event(
                            events_path,
                            {
                                "event": "bitwig_session_clean_shutdown",
                                "at": current["shutdown_at"],
                                "session": current["number"],
                            },
                        )
                        if current["mapping"] is None:
                            state["status"] = f"session_{current['number']}_exited_without_mapping"
                            save_state()
                            print(f"session_{current['number']}_mapping=not_observed", flush=True)
                            return
                        if current["number"] == 1:
                            state["status"] = "waiting_session_2"
                            current = None
                            root_missing_since = None
                            print("session_1_clean_shutdown=true", flush=True)
                            save_state()
                        else:
                            state["status"] = "completed_two_sessions"
                            state["completed_at"] = utc_now()
                            print("session_2_clean_shutdown=true", flush=True)
                            save_state()
                            return
                    elif root_missing_since is not None and time.monotonic() - root_missing_since > 30:
                        block(
                            "blocked_lingering_bitwig_descendant_or_mapping",
                            {
                                "descendant_count": len(live_observed_descendants),
                                "mapping_present": any_mapping,
                            },
                        )
                        return
            time.sleep(poll_seconds)
    finally:
        try:
            lock_path.unlink()
        except FileNotFoundError:
            pass


def process_fixture_evaluation(
    proc_root: pathlib.Path,
    module: pathlib.Path,
    expected_root: str | None,
    previous_root: str | None,
) -> dict[str, Any]:
    table = read_proc_table(proc_root, module)
    if not table["complete"]:
        return {"classification": "unknown", "reason": table["error"]}
    classified = classify_processes(table, module)
    processes = table["processes"]
    root = choose_bitwig_root(classified, processes)
    result: dict[str, Any] = {
        "classification": "observed",
        "root_identity": identity(root) if root else None,
        "bitwig_root_count": len(classified["bitwig_roots"]),
        "accepted_mapping_count": len(classified["accepted_mappings"]),
        "rejected_mapping_count": len(classified["rejected_mappings"]),
        "forbidden_count": len(classified["forbidden"]),
        "validator_count": len(classified["validators"]),
    }
    if expected_root is not None and result["root_identity"] != expected_root:
        result["classification"] = "rejected_stale_process_identity"
    if previous_root is not None and result["root_identity"] == previous_root:
        result["classification"] = "rejected_reused_process_identity"
    if classified["forbidden"]:
        result["classification"] = "blocked_forbidden_process"
    elif classified["rejected_mappings"]:
        result["classification"] = "rejected_mapping_without_bitwig_ancestry"
    elif classified["accepted_mappings"] and result["classification"] == "observed":
        result["classification"] = "accepted_bitwig_descendant_mapping"
    return result


def normalize_confirmation_transport(confirmation: str) -> tuple[str, str]:
    if "\\" not in confirmation:
        return confirmation, "literal"
    remainder = confirmation.replace("\\_", "")
    if "\\" in remainder:
        fail("operator confirmation contains an unsupported escape")
    if "_" in remainder:
        fail("operator confirmation mixes raw and Markdown-escaped underscores")
    normalized = confirmation.replace("\\_", "_")
    if normalized == confirmation:
        fail("operator confirmation escape normalization made no change")
    return normalized, "markdown_underscore_escaped"


def expected_confirmation(
    nonce: str,
    kind: str,
    gain_generic_view: str = "NOT_EVALUATED",
    bypass_generic_view: str = "NOT_EVALUATED",
) -> str:
    if kind == "success":
        return (
            f"HP1_GUI_CONFIRM {nonce} SESSION1_FOUND INSERTED REMOVED QUIT "
            "SESSION2_FOUND_WITHOUT_RESCAN INSERTED REMOVED QUIT "
            f"GAIN_GENERIC_VIEW={gain_generic_view} "
            f"BYPASS_GENERIC_VIEW={bypass_generic_view} "
            "HOST_BYPASS_BINDING=UNKNOWN"
        )
    if kind == "blocked":
        return f"HP1_GUI_CONFIRM {nonce} NOT_FOUND_AFTER_ALLOWED_SCAN QUIT"
    fail("unsupported confirmation kind")


def verify_confirmation(
    session_root: pathlib.Path,
    confirmation: str,
    *,
    consume: bool,
    test_state: pathlib.Path | None = None,
    confirmed_at: str | None = None,
) -> dict[str, Any]:
    meta = read_json(session_root / "session.json")
    nonce = meta.get("nonce")
    if not isinstance(nonce, str) or not NONCE_RE.fullmatch(nonce):
        fail("session nonce is invalid")
    if confirmation.endswith("\n"):
        confirmation = confirmation[:-1]
    if "\n" in confirmation or "\r" in confirmation:
        fail("operator confirmation must be exactly one line")
    normalized_confirmation, confirmation_transport = normalize_confirmation_transport(confirmation)
    success_observations = {
        expected_confirmation(nonce, "success", "OBSERVED", "OBSERVED"): (
            "observed",
            "observed",
        ),
        expected_confirmation(nonce, "success", "OBSERVED", "NOT_OBSERVED"): (
            "observed",
            "not_observed",
        ),
        expected_confirmation(nonce, "success", "NOT_EVALUATED", "NOT_EVALUATED"): (
            "not_evaluated",
            "not_evaluated",
        ),
    }
    blocked = expected_confirmation(nonce, "blocked")
    if normalized_confirmation in success_observations:
        kind = "success"
        gain_generic_view, bypass_generic_view = success_observations[normalized_confirmation]
        host_bypass_binding = "unknown"
    elif normalized_confirmation == blocked:
        kind = "blocked"
        gain_generic_view = "not_applicable"
        bypass_generic_view = "not_applicable"
        host_bypass_binding = "not_applicable"
    else:
        fail("operator confirmation has the wrong nonce, grammar, or required steps")
    state = read_json(test_state or session_root / "monitor.state.json")
    if state.get("schema") != SCHEMA_MONITOR:
        fail("monitor state schema differs")
    if kind == "success":
        if state.get("status") != "completed_two_sessions" or len(state.get("sessions", [])) != 2:
            fail("success confirmation is not backed by two completed monitor sessions")
        first, second = state["sessions"]
        if not first.get("mapping") or not second.get("mapping"):
            fail("success confirmation lacks both exact module mappings")
        if first.get("root_identity") == second.get("root_identity"):
            fail("success confirmation reused the first Bitwig process identity")
        if not first.get("clean_shutdown") or not second.get("clean_shutdown"):
            fail("success confirmation lacks clean shutdown evidence")
    else:
        if state.get("status") != "session_1_exited_without_mapping":
            fail("blocked discovery confirmation does not match monitor state")
    if state.get("forbidden_process_observed") or state.get("validator_observed"):
        fail("operator confirmation cannot override a contaminated process session")

    receipt = {
        "schema": SCHEMA_ATTESTATION,
        "classification": "operator_observed_supporting" if kind == "success" else "blocked",
        "kind": kind,
        "parameter_observation_classification": (
            "operator_observed_supporting" if kind == "success" else "not_applicable"
        ),
        "gain_generic_view": gain_generic_view,
        "bypass_generic_view": bypass_generic_view,
        "host_bypass_binding": host_bypass_binding,
        "nonce_sha256": sha256_bytes(nonce.encode("ascii")),
        "confirmation_sha256": sha256_bytes(confirmation.encode("utf-8")),
        "normalized_confirmation_sha256": sha256_bytes(normalized_confirmation.encode("utf-8")),
        "confirmation_transport": confirmation_transport,
        "confirmed_at": confirmed_at or utc_now(),
        "normal_desktop_launch_attested": True,
        "session_1_steps_exact": True,
        "session_2_steps_exact": kind == "success",
        "allowed_scan_result_attested": "not_applicable_success" if kind == "success" else "not_found_after_at_most_one_allowed_scan",
    }
    if consume:
        consumed = session_root / "operator-confirmation.consumed"
        try:
            fd = os.open(consumed, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        except FileExistsError:
            fail("operator nonce was already consumed")
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(receipt["confirmation_sha256"] + "\n")
            handle.flush()
            os.fsync(handle.fileno())
        atomic_write_json(session_root / "operator-attestation.json", receipt)
    return receipt


def sanitize_text(value: str) -> str:
    home = str(real_home())
    value = value.replace(home, "<HOME>").replace(str(EXPECTED_REPO), "<REPO>")
    username = pwd.getpwuid(os.getuid()).pw_name
    value = re.sub(rf"(?<![A-Za-z0-9_.-]){re.escape(username)}(?![A-Za-z0-9_.-])", "<USER>", value)
    value = re.sub(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b", "<EMAIL>", value)
    value = re.sub(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b", "<IPV4>", value)
    value = re.sub(r"(?i)\b(?:[0-9a-f]{2}:){5}[0-9a-f]{2}\b", "<MAC>", value)
    return value


def sanitized_process(process: dict[str, Any], label: str) -> dict[str, Any]:
    return {
        "label": label,
        "pid": "redacted_volatile",
        "proc_start_ticks": process.get("starttime"),
        "comm": process.get("comm"),
        "executable_basename": process.get("exe_basename"),
        "uid_classification": "fixture_user" if process.get("uid") == os.getuid() else "unexpected_uid",
        "cgroup_identity": sanitized_cgroup_identity(str(process.get("cgroup", ""))),
        "identity_sha256": sha256_bytes(
            f"{process.get('pid')}:{process.get('starttime')}".encode("ascii", "strict")
        ),
    }


def sanitized_monitor_session(session: dict[str, Any]) -> dict[str, Any]:
    number = session["number"]
    result: dict[str, Any] = {
        "session": number,
        "root": sanitized_process(session["root"], f"BITWIG_SESSION_{number}_ROOT"),
        "launched_at": session.get("launched_at"),
        "shutdown_at": session.get("shutdown_at"),
        "clean_shutdown": bool(session.get("clean_shutdown")),
        "post_shutdown": session.get("post_shutdown"),
        "mapping": None,
    }
    mapping = session.get("mapping")
    if mapping:
        mapping_process = sanitized_process(mapping["process"], f"BITWIG_SESSION_{number}_MAPPER")
        ancestry = [
            sanitized_process(item, f"BITWIG_SESSION_{number}_ANCESTOR_{index + 1}")
            for index, item in enumerate(mapping.get("ancestry", []))
        ]
        map_data = mapping["mapping"]
        result["mapping"] = {
            "captured_at": mapping.get("captured_at"),
            "process": mapping_process,
            "ancestry": ancestry,
            "bitwig_ancestry": mapping.get("bitwig_ancestry"),
            "map_entry": {
                "address_range": "redacted_volatile",
                "permissions": map_data.get("permissions"),
                "offset": map_data.get("offset"),
                "device": map_data.get("device"),
                "inode": map_data.get("inode"),
                "path": sanitize_text(str(map_data.get("path", ""))),
            },
            "module_sha256": mapping.get("module_sha256"),
            "module_device": mapping.get("module_device"),
            "module_inode": mapping.get("module_inode"),
        }
    return result


def sanitized_readback(scan: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    for item in scan.get("candidates", []):
        if item.get("change") not in {"new", "modified"} and not item.get("matching_strings"):
            continue
        candidates.append(
            {
                "root_label": item.get("root_label"),
                "relative_path": sanitize_text(str(item.get("relative_path", ""))),
                "type": "regular_file",
                "size": item.get("size"),
                "mtime_ns": item.get("mtime_ns"),
                "sha256": item.get("sha256"),
                "change": item.get("change"),
                "content_classification": item.get("content_classification", "metadata_only"),
                "matching_strings": item.get("matching_strings", []),
                "applicable_scan_error": item.get("applicable_scan_error", False),
                "applicable_scan_error_basis": item.get(
                    "applicable_scan_error_basis", "not_evaluated"
                ),
            }
        )
    return {
        "classification": scan.get("classification"),
        "status": scan.get("status"),
        "limits": scan.get("limits"),
        "candidate_count": scan.get("candidate_count"),
        "candidate_cap_exhausted": scan.get("candidate_cap_exhausted"),
        "incomplete_reasons": scan.get("incomplete_reasons", []),
        "changed_or_matching_candidates": candidates,
    }


def markdown_table(rows: Iterable[tuple[str, str]]) -> str:
    output = ["| Fact | Result |", "|---|---|"]
    output.extend(f"| {name} | {value} |" for name, value in rows)
    return "\n".join(output)


def render_evidence(session_root: pathlib.Path, output: pathlib.Path) -> dict[str, Any]:
    require_descendant(session_root, user_cache_root(), "HP1 session root", existing_directory=True)
    repo = pathlib.Path(command_text(["git", "rev-parse", "--show-toplevel"], cwd=EXPECTED_REPO))
    expected_output = repo / "evidence" / "hp1-bitwig-admission"
    if output != expected_output:
        fail("HP1 evidence output must be the exact authorized evidence directory")
    if output.exists() and (not output.is_dir() or output.is_symlink()):
        fail("HP1 evidence output is an unsafe existing object")

    meta = read_json(session_root / "session.json")
    before = read_json(session_root / "fixture.before.json")
    after = read_json(session_root / "fixture.after.json")
    preservation = read_json(session_root / "preservation.json")
    monitor = read_json(session_root / "monitor.state.json")
    attestation = read_json(session_root / "operator-attestation.json")
    readback = read_json(session_root / "bitwig-state.after.json")
    negative = read_json(session_root / "negative-tests.json")
    verify_fixture_snapshot(before)
    verify_fixture_snapshot(after)
    compare_fixture_snapshots(before, after)
    if preservation.get("classification") != "passed":
        fail("fixture preservation receipt is not passed")
    if negative.get("classification") != "passed" or negative.get("passed_count") != 17:
        fail("negative-test ledger is incomplete")

    kind = attestation.get("kind")
    if kind == "success":
        claim_classification = "passed"
        claim_name = "HP1_COMPLETE"
    elif kind == "blocked":
        claim_classification = "blocked"
        claim_name = "HP1_HOST_PATH_BLOCKED"
    else:
        fail("operator attestation kind is unsupported")
    sessions = [sanitized_monitor_session(item) for item in monitor.get("sessions", [])]
    readback_safe = sanitized_readback(readback)
    exact_strings = sorted(
        {
            match["value"]
            for candidate in readback_safe["changed_or_matching_candidates"]
            for match in candidate.get("matching_strings", [])
        }
    )
    evidence_fixture = {
        "schema": SCHEMA_EVIDENCE,
        "basis": {
            "classification": "observed",
            "commit": EXPECTED_BASIS_COMMIT,
            "tree": EXPECTED_BASIS_TREE,
            "branch": EXPECTED_BRANCH,
            "preflight": "PRE-FLIGHT_CLEAR",
        },
        "session": {
            "classification": claim_classification,
            "session_id": meta["session_id"],
            "nonce_receipt_sha256": attestation["nonce_sha256"],
            "operator_confirmation": attestation["classification"],
            "monitor_status": monitor["status"],
        },
        "hp0": {
            "classification": "observed",
            "bundle": "<HOME>/.vst3/linux-vst-bridge/LabHostProbe.vst3",
            "module_sha256": HP0_MODULE_SHA256,
            "publication_receipt_sha256": HP0_PUBLICATION_RECEIPT_SHA256,
            "build_source_manifest_schema": HP0_BUILD_SOURCE_SCHEMA,
            "build_source_manifest_sha256": HP0_BUILD_SOURCE_SHA256,
            "validator_sha256": HP0_VALIDATOR_SHA256,
            "preedit_accepted_readbacks": before["hp0"]["preedit_accepted_readbacks"],
            "before_after": "byte_identical",
        },
        "bitwig": {
            "classification": "observed",
            "app_id": BITWIG_APP_ID,
            "version": BITWIG_VERSION,
            "app_commit": BITWIG_APP_COMMIT,
            "scope": "system",
            "runtime": BITWIG_RUNTIME_REF,
            "runtime_commit": BITWIG_RUNTIME_COMMIT,
            "user_override_sha256": BITWIG_USER_OVERRIDE_SHA256,
            "system_override_sha256": BITWIG_SYSTEM_OVERRIDE_SHA256,
            "effective_vst_path": BITWIG_VST_PATH,
            "effective_vst3_path": "empty",
            "effective_clap_path": "empty",
            "before_after": "byte_identical",
        },
        "process_sessions": sessions,
        "discovery_readback": readback_safe,
        "operator_attestation": attestation,
        "process_guards": {
            "classification": "passed" if not monitor.get("forbidden_process_observed") else "blocked",
            "forbidden_process_observed": monitor.get("forbidden_process_observed"),
            "validator_observed": monitor.get("validator_observed"),
            "unrelated_mapping_observed": monitor.get("unrelated_mapping_observed"),
        },
        "serum_preservation": {
            "classification": "passed",
            "regular_file_hash_size_mtime": "unchanged",
        },
        "negative_tests": negative,
        "claim": {
            "classification": claim_classification,
            "name": claim_name,
            "primary": "native_bitwig_discovery_and_instance_admission" if kind == "success" else "native_host_path_blocked",
            "audio_processing": "explicitly_out_of_scope",
            "project_state": "explicitly_out_of_scope",
            "serum_operation": "explicitly_out_of_scope",
            "windows_bridge": "explicitly_out_of_scope",
            "general_linux": "explicitly_out_of_scope",
        },
        "sanitization": {
            "classification": "passed",
            "durable_pids_retained": False,
            "private_home_retained": False,
            "complete_maps_retained": False,
            "command_lines_or_environments_retained": False,
            "raw_session_retained": False,
        },
    }

    staging = output.parent / f".{output.name}.stage-{secrets.token_hex(6)}"
    if staging.exists():
        fail("evidence staging collision")
    staging.mkdir(mode=0o700)
    try:
        basis_md = f"""# HP1 basis and fixture\n\n{markdown_table((
            ('Pre-flight', '`PRE-FLIGHT_CLEAR`'),
            ('Basis commit', f'`{EXPECTED_BASIS_COMMIT}`'),
            ('Basis tree', f'`{EXPECTED_BASIS_TREE}`'),
            ('Branch', f'`{EXPECTED_BRANCH}`'),
            ('Session ID', f'`{meta["session_id"]}`'),
            ('Host', 'Steam Deck Galileo / SteamOS 3.8.16 / x86_64 / KDE Wayland'),
            ('Pre-edit HP0 publication readback', f'`passed` / `{before["hp0"]["preedit_accepted_readbacks"]["sha256"]["publication"]}`'),
            ('Pre-edit HP0 build-receipt readback', f'`passed` / `{before["hp0"]["preedit_accepted_readbacks"]["sha256"]["build"]}`'),
            ('Pre-edit HP0 Bitwig-sandbox validator readback', f'`passed` / `{before["hp0"]["preedit_accepted_readbacks"]["sha256"]["sandbox"]}`'),
        ))}\n\nThe real hostname, user name, home prefix, display socket, and volatile process IDs are not retained.\n"""
        protocol_md = f"""# HP1 operator-assisted session protocol\n\nThe session used one canonical, symlink-free cache root, cryptographically random session ID, and one-use nonce. The monitor began before the ordinary desktop launch and captured only bounded process identity, ancestry, the exact matching module map entry, and forbidden-process classes.\n\n- Session ID: `{meta['session_id']}`\n- Nonce receipt SHA-256: `{attestation['nonce_sha256']}`\n- Confirmation classification: `{attestation['classification']}`\n- Monitor terminal state: `{monitor['status']}`\n\nNo terminal-modified Bitwig environment, Flatpak override, plug-in location, validator process, project save, command-line dump, process environment, or complete process map was used.\n"""
        discovery_value = "operator observed the exact class in both launches" if kind == "success" else "operator reported absence after normal startup and the one allowed built-in scan"
        discovery_md = f"""# HP1 discovery result\n\nClassification: `{claim_classification}`\n\n- Operator result: {discovery_value}.\n- Exact readback strings: {', '.join(f'`{item}`' for item in exact_strings) if exact_strings else '`not_observed`'}.\n- Bitwig-owned readback status: `{readback_safe['status']}`.\n- Explicit-path HP0 app-sandbox visibility remained accepted; this session did not alter effective paths.\n\nA GUI sighting is supporting evidence only. Success additionally requires the exact Bitwig-descendant mapping retained in each session.\n"""

        def session_markdown(number: int) -> str:
            if len(sessions) < number:
                return f"# HP1 instance Session {number}\n\nClassification: `not_observed`\n\nAdmission stage was not reached.\n"
            item = sessions[number - 1]
            mapping = item.get("mapping")
            lines = [
                f"# HP1 instance Session {number}",
                "",
                f"Classification: `{'passed' if mapping and item['clean_shutdown'] else 'not_observed'}`",
                "",
                f"- Bitwig root start ticks: `{item['root']['proc_start_ticks']}`; PID is redacted.",
                f"- Bitwig root identity SHA-256: `{item['root']['identity_sha256']}`.",
                f"- Clean shutdown: `{str(item['clean_shutdown']).lower()}`.",
            ]
            if mapping:
                lines.extend(
                    [
                        f"- Mapping process: `{mapping['process']['comm']}` / `{mapping['process']['executable_basename']}`.",
                        f"- Exact module SHA-256: `{mapping['module_sha256']}`.",
                        f"- Mapping path: `{mapping['map_entry']['path']}`.",
                        f"- Mapping permissions/offset/device/inode: `{mapping['map_entry']['permissions']}` / `{mapping['map_entry']['offset']}` / `{mapping['map_entry']['device']}` / `{mapping['map_entry']['inode']}`.",
                        f"- Proven Bitwig ancestry: `{str(mapping['bitwig_ancestry']).lower()}`.",
                    ]
                )
            else:
                lines.append("- Exact accepted module mapping: `not_observed`.")
            return "\n".join(lines) + "\n"

        readback_lines = [
            "# HP1 bounded Bitwig-owned state readback",
            "",
            f"Classification: `{readback_safe['classification']}`; status: `{readback_safe['status']}`.",
            "",
            f"Limits: {json.dumps(readback_safe['limits'], sort_keys=True)}",
            "",
        ]
        if readback_safe["changed_or_matching_candidates"]:
            readback_lines.extend(["| Root | Relative path | Change | Size | SHA-256 | Exact strings |", "|---|---|---|---:|---|---|"])
            for item in readback_safe["changed_or_matching_candidates"]:
                strings = ", ".join(match["value"] for match in item.get("matching_strings", [])) or "none"
                readback_lines.append(
                    f"| `{item['root_label']}` | `{item['relative_path']}` | `{item['change']}` | {item['size']} | `{item.get('sha256') or 'not_retained'}` | {strings} |"
                )
        else:
            readback_lines.append("No exact class/vendor/CID/module string was observed in a changed bounded candidate.")
        readback_md = "\n".join(readback_lines) + "\n"
        guards_md = f"""# HP1 process guards\n\n{markdown_table((
            ('Forbidden Wine/Proton/UMU/yabridge process observed', f'`{str(monitor.get("forbidden_process_observed", False)).lower()}`'),
            ('Official validator observed during graphical sessions', f'`{str(monitor.get("validator_observed", False)).lower()}`'),
            ('Unrelated exact module mapping observed', f'`{str(monitor.get("unrelated_mapping_observed", False)).lower()}`'),
            ('Terminal monitor status', f'`{monitor["status"]}`'),
        ))}\n\nOrdinary Steam and web-helper processes were not classified as Windows plug-in workloads. No command line or process environment was retained.\n"""
        attestation_md = f"""# HP1 operator attestation\n\n- Classification: `{attestation['classification']}`.\n- Exact one-use nonce receipt SHA-256: `{attestation['nonce_sha256']}`.\n- Operator transport SHA-256: `{attestation['confirmation_sha256']}`.\n- Normalized semantic confirmation SHA-256: `{attestation['normalized_confirmation_sha256']}`.\n- Confirmation transport: `{attestation['confirmation_transport']}`.\n- Normal desktop launch attested: `{str(attestation['normal_desktop_launch_attested']).lower()}`.\n- Session 1 bounded steps attested: `{str(attestation['session_1_steps_exact']).lower()}`.\n- Session 2 bounded steps attested: `{str(attestation['session_2_steps_exact']).lower()}`.\n- Gain in Bitwig generic parameter view: `{attestation['gain_generic_view']}`.\n- Bypass as a separately exposed generic parameter: `{attestation['bypass_generic_view']}`.\n- Binding between the VST3 `kIsBypass` parameter and Bitwig's host device bypass control: `{attestation['host_bypass_binding']}`.\n\nOnly uniform Markdown underscore escaping is normalized; mixed or other escapes are refused. The parameter observations are supporting evidence only. HP1 did not exercise parameter changes or DSP, so host bypass binding remains unknown. The nonce itself is raw session material and is not committed.\n"""
        negative_rows = "\n".join(
            f"| {item['case']} | `{item['result']}` |" for item in negative["cases"]
        )
        negative_md = f"""# HP1 deterministic negative tests\n\n| Case | Result |\n|---|---|\n{negative_rows}\n\nAggregate: `passed` ({negative['passed_count']}/17). Fixtures were synthetic and stayed beneath the canonical cache test root.\n"""
        sanitization_md = """# HP1 evidence sanitization\n\nClassification: `passed`.\n\n- Raw session, nonce, PIDs, full `/proc` maps, command lines, environments, Bitwig projects, account/license data, raw databases, and proprietary plug-in content are not retained.\n- The private home prefix is `<HOME>` and the repository is `<REPO>`.\n- Only exact relevant strings and bounded metadata from Bitwig-owned state are retained; cap, timeout, access, and parse failures propagate to `unknown/search_incomplete`.\n- Process IDs are replaced by session labels; start ticks and identity digests preserve launch distinction without treating a PID as durable identity.\n- Every evidence file is UTF-8 text or intentional JSON, and `hashes.sha256` covers every file other than itself.\n"""
        finding_text = (
            "The exact accepted probe was discovered and mapped by a proven Bitwig descendant in two distinct normal launches, with clean shutdown after each."
            if kind == "success"
            else "The exact accepted probe was not discovered after the bounded normal-launch/allowed-scan path; no path or override was changed."
        )
        findings_md = f"""# HP1 findings\n\n## Result\n\n`{claim_name}` — {finding_text}\n\n## Preserved fixture\n\nThe Bitwig app/runtime and override bytes, HP0 publication/module/receipt, HP0 build identity, and both accepted Serum regular-file hash/size/mtime identities remained exact before and after. No forbidden compatibility process or official validator participated in a graphical session.\n\n## Claim ceiling\n\nHP1 does not prove audio or DSP correctness, automation, state or project recall, custom UI, Serum operation, a Windows host or bridge, IPC/shared memory, manager/broker behavior, CLAP, performance, crash recovery, another DAW, or general Linux compatibility.\n"""

        documents = {
            "BASIS.md": basis_md,
            "SESSION_PROTOCOL.md": protocol_md,
            "DISCOVERY.md": discovery_md,
            "INSTANCE_SESSION_1.md": session_markdown(1),
            "INSTANCE_SESSION_2.md": session_markdown(2),
            "BITWIG_STATE_READBACK.md": readback_md,
            "PROCESS_GUARDS.md": guards_md,
            "OPERATOR_ATTESTATION.md": attestation_md,
            "NEGATIVE_TESTS.md": negative_md,
            "SANITIZATION.md": sanitization_md,
            "FINDINGS.md": findings_md,
            "fixture.json": json.dumps(evidence_fixture, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        }
        for name, content in documents.items():
            (staging / name).write_text(content, encoding="utf-8", newline="\n")
        hashes = []
        for path in sorted(staging.iterdir(), key=lambda item: item.name):
            if path.name == "hashes.sha256":
                continue
            hashes.append(f"{sha256_file(path)}  {path.name}")
        (staging / "hashes.sha256").write_text("\n".join(hashes) + "\n", encoding="utf-8", newline="\n")
        if output.exists():
            known = set(documents) | {"hashes.sha256"}
            if {path.name for path in output.iterdir()} != known:
                fail("refusing to replace an unknown HP1 evidence directory")
            backup = output.parent / f".{output.name}.previous-{secrets.token_hex(6)}"
            os.replace(output, backup)
            try:
                os.replace(staging, output)
            except Exception:
                os.replace(backup, output)
                raise
            shutil.rmtree(backup)
        else:
            os.replace(staging, output)
    except Exception:
        if staging.exists():
            shutil.rmtree(staging)
        raise
    return {"classification": "passed", "output": str(output), "file_count": 13, "claim": claim_name}


def finalize_session(
    session_root: pathlib.Path,
    confirmation_file: pathlib.Path,
    repo: pathlib.Path,
    build_dir: pathlib.Path,
    proof_root: pathlib.Path,
) -> dict[str, Any]:
    require_descendant(session_root, user_cache_root(), "HP1 session root", existing_directory=True)
    require_descendant(confirmation_file, session_root, "operator confirmation input", regular_or_absent=True)
    if not confirmation_file.is_file() or confirmation_file.is_symlink():
        fail("operator confirmation input is missing or unsafe")
    confirmation = confirmation_file.read_text(encoding="utf-8")
    attestation_preview = verify_confirmation(session_root, confirmation, consume=False)
    require_empty_process_guard(pathlib.Path("/proc"))

    before_path = session_root / "fixture.before.json"
    after_path = session_root / "fixture.after.json"
    before = read_json(before_path)
    after = capture_live_fixture(repo, build_dir, proof_root)
    atomic_write_json(after_path, after)
    preservation = compare_fixture_snapshots(before, after)
    atomic_write_json(session_root / "preservation.json", preservation)

    readback = scan_state(
        session_root / "bitwig-state.after.json",
        baseline_path=session_root / "bitwig-state.baseline.json",
    )
    if readback.get("status") != "completed":
        fail("bounded Bitwig-owned state readback is incomplete")
    require_empty_process_guard(pathlib.Path("/proc"))
    attestation = verify_confirmation(
        session_root,
        confirmation,
        consume=True,
        confirmed_at=attestation_preview["confirmed_at"],
    )
    if attestation != attestation_preview:
        fail("operator attestation changed between preview and commit")
    result = {
        "classification": "passed" if attestation["kind"] == "success" else "blocked",
        "result": "HP1_COMPLETE" if attestation["kind"] == "success" else "HP1_HOST_PATH_BLOCKED",
        "collected_at": utc_now(),
        "fixture_preservation": preservation,
        "state_readback_status": readback["status"],
        "process_guard_after": {"bitwig": 0, "forbidden": 0, "validator": 0},
        "nonce_confirmation": "accepted_once",
    }
    atomic_write_json(session_root / "collection.json", result)
    meta = read_json(session_root / "session.json")
    meta["status"] = "collected_success" if attestation["kind"] == "success" else "collected_blocked"
    meta["collected_at"] = result["collected_at"]
    atomic_write_json(session_root / "session.json", meta)
    return result


def mutate_fixture(source: pathlib.Path, output: pathlib.Path, field: str, value: str) -> None:
    fixture = read_json(source)
    current: Any = fixture
    parts = field.split(".")
    for part in parts[:-1]:
        if part.isdigit():
            current = current[int(part)]
        else:
            current = current[part]
    last = parts[-1]
    parsed: Any = value
    if value.isdigit():
        parsed = int(value)
    if last.isdigit():
        current[int(last)] = parsed
    else:
        current[last] = parsed
    require_descendant(output, user_cache_root(), "mutated fixture output", regular_or_absent=True)
    atomic_write_json(output, fixture)


def make_proc_process(
    root: pathlib.Path,
    pid: int,
    ppid: int,
    starttime: int,
    comm: str,
    exe: str,
    cgroup: str,
    mapping: pathlib.Path | None,
) -> None:
    directory = root / str(pid)
    directory.mkdir()
    remaining = [str(ppid)] + ["0"] * 17 + [str(starttime)]
    (directory / "stat").write_text(f"{pid} ({comm}) S {' '.join(remaining)}\n", encoding="utf-8")
    (directory / "comm").write_text(comm + "\n", encoding="utf-8")
    (directory / "status").write_text(f"Name:\t{comm}\nUid:\t{os.getuid()}\t{os.getuid()}\t{os.getuid()}\t{os.getuid()}\n", encoding="utf-8")
    (directory / "cgroup").write_text(cgroup + "\n", encoding="utf-8")
    (directory / "exe.name").write_text(exe + "\n", encoding="utf-8")
    maps = ""
    if mapping is not None:
        maps = f"7f000000-7f001000 r-xp 00000000 00:01 12345 {mapping}\n"
    (directory / "maps").write_text(maps, encoding="utf-8")


def make_proc_fixture(root: pathlib.Path, case: str) -> pathlib.Path:
    require_descendant(root, user_cache_root(), "synthetic proc root", directory_or_absent=True)
    if root.exists():
        fail("synthetic proc root must be absent")
    root.mkdir(mode=0o700)
    module = root.parent / f"{root.name}-LabHostProbe.so"
    module.write_bytes(b"synthetic HP1 module fixture\n")
    cgroup = f"0::/user.slice/app-flatpak-{BITWIG_APP_ID}-fixture.scope"
    make_proc_process(root, 1, 0, 1, "init", "init", "0::/", None)
    if case == "bitwig_running":
        make_proc_process(root, 90, 1, 111, "bwrap", "bwrap", cgroup, None)
        make_proc_process(root, 100, 90, 112, "BitwigStudio", "BitwigStudio", cgroup, None)
    elif case == "unrelated_mapping":
        make_proc_process(root, 90, 1, 111, "bwrap", "bwrap", cgroup, None)
        make_proc_process(root, 100, 90, 112, "BitwigStudio", "BitwigStudio", cgroup, None)
        make_proc_process(root, 200, 1, 211, "other-host", "other-host", "0::/user.slice/other.scope", module)
    elif case in {"valid_mapping", "stale", "reused"}:
        root_start = 222 if case == "stale" else 111
        make_proc_process(root, 90, 1, root_start, "bwrap", "bwrap", cgroup, None)
        make_proc_process(root, 100, 90, 112, "BitwigStudio", "BitwigStudio", cgroup, None)
        make_proc_process(root, 101, 90, 113, "BitwigPluginHost", "BitwigPluginHost", cgroup, module)
    elif case == "forbidden":
        make_proc_process(root, 90, 1, 111, "bwrap", "bwrap", cgroup, None)
        make_proc_process(root, 100, 90, 112, "BitwigStudio", "BitwigStudio", cgroup, None)
        make_proc_process(root, 102, 90, 113, "yabridge-host", "yabridge-host", cgroup, None)
    else:
        fail("unsupported synthetic proc case")
    return module


def make_test_monitor_state(output: pathlib.Path, kind: str) -> None:
    base_session = {
        "number": 1,
        "root_identity": "100:111",
        "root": {"pid": 100, "ppid": 1, "starttime": 111, "comm": "BitwigStudio", "exe_basename": "BitwigStudio", "uid": os.getuid(), "cgroup": BITWIG_APP_ID},
        "mapping": {"synthetic": True},
        "clean_shutdown": True,
    }
    state: dict[str, Any] = {
        "schema": SCHEMA_MONITOR,
        "status": "completed_two_sessions" if kind in {"success", "reused"} else "session_1_exited_without_mapping",
        "sessions": [],
        "forbidden_process_observed": False,
        "validator_observed": False,
    }
    if kind == "blocked":
        blocked_session = dict(base_session)
        blocked_session["mapping"] = None
        state["sessions"] = [blocked_session]
    else:
        second = dict(base_session)
        second["number"] = 2
        second["root_identity"] = "100:111" if kind == "reused" else "200:222"
        second["root"] = dict(base_session["root"])
        second["root"]["pid"] = 100 if kind == "reused" else 200
        second["root"]["starttime"] = 111 if kind == "reused" else 222
        state["sessions"] = [base_session, second]
    require_descendant(output, user_cache_root(), "synthetic monitor state", regular_or_absent=True)
    atomic_write_json(output, state)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("cache-root")
    create = subparsers.add_parser("create-session")

    resolve = subparsers.add_parser("resolve-session")
    resolve.add_argument("--session-id", required=True)

    path_check = subparsers.add_parser("path-check")
    path_check.add_argument("--path", type=pathlib.Path, required=True)
    path_check.add_argument("--under-root", type=pathlib.Path, required=True)
    path_check.add_argument("--kind", choices=("existing-directory", "directory-or-absent", "regular-or-absent"), required=True)

    capture = subparsers.add_parser("capture-fixture")
    capture.add_argument("--repo-root", type=pathlib.Path, required=True)
    capture.add_argument("--build-dir", type=pathlib.Path, required=True)
    capture.add_argument("--preedit-proof", type=pathlib.Path, required=True)
    capture.add_argument("--output", type=pathlib.Path, required=True)

    fixture_check = subparsers.add_parser("fixture-check")
    fixture_check.add_argument("--input", type=pathlib.Path, required=True)

    fixture_mutate = subparsers.add_parser("fixture-mutate")
    fixture_mutate.add_argument("--input", type=pathlib.Path, required=True)
    fixture_mutate.add_argument("--output", type=pathlib.Path, required=True)
    fixture_mutate.add_argument("--field", required=True)
    fixture_mutate.add_argument("--value", required=True)

    fixture_compare = subparsers.add_parser("fixture-compare")
    fixture_compare.add_argument("--before", type=pathlib.Path, required=True)
    fixture_compare.add_argument("--after", type=pathlib.Path, required=True)
    fixture_compare.add_argument("--output", type=pathlib.Path)

    guard = subparsers.add_parser("guard")
    guard.add_argument("--proc-root", type=pathlib.Path, default=pathlib.Path("/proc"))

    scan = subparsers.add_parser("scan-state")
    scan.add_argument("--output", type=pathlib.Path, required=True)
    scan.add_argument("--baseline", type=pathlib.Path)
    scan.add_argument("--test-root", type=pathlib.Path)
    scan.add_argument("--root", type=pathlib.Path, action="append")
    scan.add_argument("--max-candidates", type=int, default=256)
    scan.add_argument("--max-file-bytes", type=int, default=4 * 1024 * 1024)
    scan.add_argument("--max-total-bytes", type=int, default=16 * 1024 * 1024)
    scan.add_argument("--timeout-seconds", type=float, default=15.0)

    monitor = subparsers.add_parser("monitor")
    monitor.add_argument("--session-id", required=True)
    monitor.add_argument("--proc-root", type=pathlib.Path, default=pathlib.Path("/proc"))
    monitor.add_argument("--poll-seconds", type=float, default=0.25)
    monitor.add_argument("--max-seconds", type=float, default=4 * 60 * 60)

    status = subparsers.add_parser("status")
    status.add_argument("--session-id", required=True)

    proc_fixture = subparsers.add_parser("make-proc-fixture")
    proc_fixture.add_argument("--root", type=pathlib.Path, required=True)
    proc_fixture.add_argument("--case", choices=("bitwig_running", "unrelated_mapping", "valid_mapping", "stale", "reused", "forbidden"), required=True)

    proc_eval = subparsers.add_parser("proc-evaluate")
    proc_eval.add_argument("--proc-root", type=pathlib.Path, required=True)
    proc_eval.add_argument("--module", type=pathlib.Path, required=True)
    proc_eval.add_argument("--expected-root")
    proc_eval.add_argument("--previous-root")
    proc_eval.add_argument("--output", type=pathlib.Path)

    confirmation = subparsers.add_parser("verify-confirmation")
    confirmation.add_argument("--session-id")
    confirmation.add_argument("--test-session-root", type=pathlib.Path)
    confirmation.add_argument("--confirmation-file", type=pathlib.Path, required=True)
    confirmation.add_argument("--consume", action="store_true")
    confirmation.add_argument("--test-state", type=pathlib.Path)

    test_state = subparsers.add_parser("make-test-monitor-state")
    test_state.add_argument("--output", type=pathlib.Path, required=True)
    test_state.add_argument("--kind", choices=("success", "blocked", "reused"), required=True)

    render = subparsers.add_parser("render-evidence")
    render.add_argument("--session-id", required=True)
    render.add_argument("--output", type=pathlib.Path, required=True)

    finalize = subparsers.add_parser("finalize-session")
    finalize.add_argument("--session-id", required=True)
    finalize.add_argument("--confirmation-file", type=pathlib.Path, required=True)
    finalize.add_argument("--repo-root", type=pathlib.Path, required=True)
    finalize.add_argument("--build-dir", type=pathlib.Path, required=True)
    finalize.add_argument("--preedit-proof", type=pathlib.Path, required=True)
    return parser


def main() -> int:
    arguments = build_parser().parse_args()
    if arguments.command == "cache-root":
        print(user_cache_root())
    elif arguments.command == "create-session":
        print(json.dumps(create_session(), sort_keys=True))
    elif arguments.command == "resolve-session":
        print(session_root_from_id(arguments.session_id))
    elif arguments.command == "path-check":
        kwargs = {
            "existing_directory": arguments.kind == "existing-directory",
            "directory_or_absent": arguments.kind == "directory-or-absent",
            "regular_or_absent": arguments.kind == "regular-or-absent",
        }
        print(require_descendant(arguments.path, arguments.under_root, "checked path", **kwargs))
    elif arguments.command == "capture-fixture":
        fixture = capture_live_fixture(arguments.repo_root, arguments.build_dir, arguments.preedit_proof)
        require_descendant(arguments.output, user_cache_root(), "fixture output", regular_or_absent=True)
        atomic_write_json(arguments.output, fixture)
        print("fixture_status=passed")
    elif arguments.command == "fixture-check":
        verify_fixture_snapshot(read_json(arguments.input))
        print("fixture_status=passed")
    elif arguments.command == "fixture-mutate":
        mutate_fixture(arguments.input, arguments.output, arguments.field, arguments.value)
    elif arguments.command == "fixture-compare":
        result = compare_fixture_snapshots(read_json(arguments.before), read_json(arguments.after))
        if arguments.output:
            require_descendant(arguments.output, user_cache_root(), "fixture comparison output", regular_or_absent=True)
            atomic_write_json(arguments.output, result)
        print("fixture_before_after=byte_identical")
    elif arguments.command == "guard":
        result = require_empty_process_guard(arguments.proc_root)
        print(json.dumps(result, sort_keys=True))
    elif arguments.command == "scan-state":
        result = scan_state(
            arguments.output,
            baseline_path=arguments.baseline,
            test_root=arguments.test_root,
            roots=arguments.root,
            max_candidates=arguments.max_candidates,
            max_file_bytes=arguments.max_file_bytes,
            max_total_bytes=arguments.max_total_bytes,
            timeout_seconds=arguments.timeout_seconds,
        )
        print(f"state_scan_status={result['status']}")
    elif arguments.command == "monitor":
        root = session_root_from_id(arguments.session_id)
        monitor_session(root, arguments.proc_root, arguments.poll_seconds, arguments.max_seconds)
    elif arguments.command == "status":
        root = session_root_from_id(arguments.session_id)
        state = read_json(root / "monitor.state.json")
        monitor_status = str(state.get("status"))
        print(f"monitor_status={monitor_status}")
        if monitor_status.startswith("blocked_") and monitor_status.endswith("_waiting_quit"):
            print("operator_action=quit_bitwig_normally_now")
        for item in state.get("sessions", []):
            mapping = "captured" if item.get("mapping") else "not_observed"
            shutdown = "clean" if item.get("clean_shutdown") else "pending"
            print(f"session_{item['number']}_mapping={mapping}")
            print(f"session_{item['number']}_shutdown={shutdown}")
    elif arguments.command == "make-proc-fixture":
        print(make_proc_fixture(arguments.root, arguments.case))
    elif arguments.command == "proc-evaluate":
        result = process_fixture_evaluation(arguments.proc_root, arguments.module, arguments.expected_root, arguments.previous_root)
        if arguments.output:
            require_descendant(arguments.output, user_cache_root(), "proc evaluation output", regular_or_absent=True)
            atomic_write_json(arguments.output, result)
        print(json.dumps(result, sort_keys=True))
    elif arguments.command == "verify-confirmation":
        if bool(arguments.session_id) == bool(arguments.test_session_root):
            fail("confirmation verification requires exactly one live session ID or test session root")
        if arguments.test_session_root:
            root = require_descendant(
                arguments.test_session_root,
                user_cache_root(),
                "confirmation test session root",
                existing_directory=True,
            )
            test_meta = read_json(root / "session.json")
            if test_meta.get("schema") != SCHEMA_SESSION:
                fail("confirmation test session metadata schema differs")
        else:
            root = session_root_from_id(arguments.session_id)
        require_descendant(arguments.confirmation_file, root, "operator confirmation input", regular_or_absent=True)
        confirmation = arguments.confirmation_file.read_text(encoding="utf-8")
        receipt = verify_confirmation(root, confirmation, consume=arguments.consume, test_state=arguments.test_state)
        print(json.dumps(receipt, sort_keys=True))
    elif arguments.command == "make-test-monitor-state":
        make_test_monitor_state(arguments.output, arguments.kind)
    elif arguments.command == "render-evidence":
        root = session_root_from_id(arguments.session_id)
        result = render_evidence(root, arguments.output)
        print(json.dumps(result, sort_keys=True))
    elif arguments.command == "finalize-session":
        root = session_root_from_id(arguments.session_id)
        result = finalize_session(
            root,
            arguments.confirmation_file,
            arguments.repo_root,
            arguments.build_dir,
            arguments.preedit_proof,
        )
        print(json.dumps(result, sort_keys=True))
    else:
        fail("unsupported command")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except HP1Error as exc:
        print(f"HP1_ERROR: {exc}", file=sys.stderr)
        raise SystemExit(1)

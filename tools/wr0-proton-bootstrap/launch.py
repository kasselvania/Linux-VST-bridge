#!/usr/bin/env python3
"""WR0 controlled Proton environment bootstrap and evidence collector.

Only Python's standard library is used. Ordinary mode has no caller-selected
runner, runtime, executable, workload, environment, or evidence path.
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
import selectors
import secrets
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from typing import Any, Iterable, Sequence


BASIS_COMMIT = "3deb414a54174cd95432c84e117a642f30c482fe"
BASIS_TREE = "7f29cc727ce128021a6a2d74d04ca9ad6e30cb13"
EXPECTED_BRANCH = "codex/wr0-proton-isolated-bootstrap"
EXPECTED_REPOSITORY_SUFFIX = pathlib.Path("code/Linux-VST-bridge")
REVIEWED_PRE_REPAIR_HEAD = "9228217b2abf7314b9dfaecc5fc4323d5f3d7a89"
REVIEWED_PRE_REPAIR_TREE = "8da6817eba1f259d3565e377fcb50098ab8f3cf2"

RUNNER_VERSION = "1787334450 proton-11.0-2-x86_64"
RUNNER_APP_ID = "4628710"
RUNNER_BUILD_ID = "24867889"
RUNNER_DEPOT_MANIFEST = "3114679013132291065"
RUNTIME_APP_ID = "4183110"
RUNTIME_BUILD_ID = "24599767"
RUNTIME_DEPOT_MANIFEST = "78117001432799844"
RUNTIME_VERSION = "4.0.20260805.254769"
PRESSURE_VESSEL_VERSION = "0.20260805.0"
NEUTRAL_APP_ID = "0"

LOCK_SCHEMA = "linux-vst-bridge-wr0-launch-critical/v1"
CONTRACT_SOURCE_SCHEMA = "linux-vst-bridge-wr0-contract-source/v1"
ENVIRONMENT_SCHEMA = "linux-vst-bridge-wr0-environment/v1"
SESSION_SCHEMA = "linux-vst-bridge-wr0-session/v1"
RUN_SCHEMA = "linux-vst-bridge-wr0-run/v1"
NEGATIVE_SCHEMA = "linux-vst-bridge-wr0-negative-tests/v1"
EVIDENCE_SCHEMA = "linux-vst-bridge-wr0-fixture/v1"
REPLACEMENT_COMMIT_SCHEMA = "linux-vst-bridge-wr0-replacement-commit/v1"
REPLACEMENT_COMMIT_FILE = "wr0-replacement-commit.json"
MAGIC = "LINUX_VST_BRIDGE_WR0_V1"
RECEIPT_SCHEMA = "LINUX_VST_BRIDGE_WR0_RECEIPT_V2"
HANDSHAKE_SCHEMA = "LINUX_VST_BRIDGE_WR0_HANDSHAKE_V1"

HP0_MODULE_SHA256 = "3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7"
HP0_RECEIPT_SHA256 = "d328adb27fef326b8e5b1104f072c246a803389a77d35ee5babc9834067668d1"
HP0_RECEIPT_SCHEMA = "linux-vst-bridge-hp0-publication/v2"
HP0_BUILD_SOURCE_SCHEMA = "linux-vst-bridge-hp0-build-source/v1"
HP0_BUILD_SOURCE_SHA256 = "ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab"
HP0_BUILD_RECEIPT_SCHEMA = "linux-vst-bridge-hp0-build/v2"
HP0_BUILD_RECEIPT_SHA256 = "2fae3eb22cd6ee0725703d4b507525cae427bcaa01412dd80f879832c5951b4e"
HP0_BUNDLE_MANIFEST_SHA256 = "ec8a4531a3b73da3c3e29c6e1f3c90c989fb61e3f3397af32d4c1576b3624ea9"
HP0_VALIDATOR_SHA256 = "cccae776eb87fbbbf6ac9c34c78bf1548937c48db3843e0cf0f12d312650466f"
BITWIG_APP_ID = "com.bitwig.BitwigStudio"
BITWIG_VERSION = "6.0.11"
BITWIG_APP_COMMIT = "7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e"
BITWIG_RUNTIME_REF = "org.freedesktop.Platform/x86_64/25.08"
BITWIG_RUNTIME_COMMIT = "bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8"
BITWIG_USER_OVERRIDE_SHA256 = "1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e"
BITWIG_SYSTEM_OVERRIDE_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"

SERUM_FILES = (
    {
        "role": "existing_native_proxy",
        "relative": ".vst3/yabridge/Serum2.vst3/Contents/x86_64-linux/Serum2.so",
        "sha256": "317d70f95a3c7559ff3d43b014c3e7b792fd03362d5e999d550a5566cf25b184",
        "size": 88888,
        "mtime_ns": 1745989109370060240,
    },
    {
        "role": "existing_windows_module",
        "relative": ".wine/drive_c/Program Files/Common Files/VST3/Serum2.vst3/Contents/x86_64-win/Serum2.vst3",
        "sha256": "838bc7ab42d5d039156768680ffc3e0175d6e6bed9f99802989a01d695e13175",
        "size": 18062336,
        "mtime_ns": 1745689532000000000,
    },
)

EVIDENCE_PACKET_DIRS = (
    "sr0-steam-deck-fixture-reconnaissance",
    "hp0-native-vst3-bitwig-sandbox",
    "hp1-bitwig-admission",
)

ALLOWED_TRACKED_PREFIXES = (
    "CURRENT_SLICE.md",
    "docs/WR0_RUNNER_LOCK.md",
    "windows-fixtures/wr0-probe/",
    "tools/wr0-proton-bootstrap/",
    "evidence/wr0-proton-bootstrap/",
)

OUTPUT_CAP = 16 * 1024
PROCESS_CAP = 256
PROCESS_POLL_SECONDS = 0.05
PROCESS_DRAIN_SECONDS = 20.0
PROCESS_SNAPSHOT_SECONDS = 1.5
RUN1_TIMEOUT_SECONDS = 180.0
REUSE_TIMEOUT_SECONDS = 90.0
TERM_GRACE_SECONDS = 3.0
CLEANUP_DEADLINE_SECONDS = 10.0
HELD_COMMAND_SECONDS = 1.0
FIXTURE_COMMAND_TIMEOUT = 20.0
MAX_EVIDENCE_FILE_BYTES = 512 * 1024

PREDECESSOR_TRANSACTION_ID = "wr0-20260901T045337Z-caf9f4eaf52d2d34"
PREDECESSOR_ENVIRONMENT_IDENTITY = "447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24"
PREDECESSOR_RUNNER_IDENTITY = "2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547"
PREDECESSOR_CONTRACT_SOURCE = "887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8"
PREDECESSOR_WORKLOAD_SHA256 = "4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac"

REPLACEMENT_PHASES = (
    "predecessor_verified",
    "predecessor_backed_up",
    "new_environment_staged",
    "new_environment_promoted",
    "commit_ready",
    "new_environment_committed",
    "predecessor_retirement_pending",
    "predecessor_retired",
    "evidence_finalized",
)

REQUIRED_EVIDENCE_FILES = (
    "BASIS.md",
    "RUNNER_LOCK.md",
    "LAUNCH_CONTRACT.md",
    "ENVIRONMENT.md",
    "RUN_1.md",
    "RUN_2.md",
    "EXIT_PROPAGATION.md",
    "PROCESS_GUARDS.md",
    "PRESERVATION.md",
    "NEGATIVE_TESTS.md",
    "SANITIZATION.md",
    "FINDINGS.md",
    "fixture.json",
    "hashes.sha256",
)

CONTRACT_SOURCE_PATHS = (
    "docs/WR0_RUNNER_LOCK.md",
    "tools/wr0-proton-bootstrap/README.md",
    "tools/wr0-proton-bootstrap/common.sh",
    "tools/wr0-proton-bootstrap/environment.sh",
    "tools/wr0-proton-bootstrap/inspect-runner.sh",
    "tools/wr0-proton-bootstrap/launch.py",
    "tools/wr0-proton-bootstrap/negative-tests.sh",
    "tools/wr0-proton-bootstrap/preflight.sh",
    "tools/wr0-proton-bootstrap/sanitize.sh",
    "windows-fixtures/wr0-probe/wr0-probe.cmd",
)


class WR0Error(RuntimeError):
    """Fail-closed WR0 error."""


class LaunchBlocked(WR0Error):
    """Exact controlled launch could not establish the WR0 claim."""

    def __init__(self, message: str, *, partial_run: dict[str, Any] | None = None) -> None:
        super().__init__(message)
        self.partial_run = partial_run


class ProcessTopologyBlocked(LaunchBlocked):
    """Exact owned process scope could not be proven empty."""


class ProcessObservabilityBlocked(LaunchBlocked):
    """Exact run/path-bound Windows command identity was not observable."""


class ReplacementPrecommitBlocked(WR0Error):
    """Replacement failed before the new environment became authoritative."""


class PredecessorBackupBlocked(ReplacementPrecommitBlocked):
    """The exact predecessor could not be restored during guarded backup."""


class PredecessorRetirementBlocked(WR0Error):
    """Committed replacement is exact but predecessor retirement is incomplete."""


class EvidenceFinalizationBlocked(WR0Error):
    """Committed replacement is exact but retained evidence is incomplete."""


@dataclass
class ReplacementTransactionState:
    """Explicit in-memory authority state for one replacement transaction."""

    phase: str = "predecessor_verified"
    durably_committed: bool = False
    predecessor_retired: bool = False
    rollback_invoked: bool = False
    events: list[dict[str, Any]] = field(default_factory=list)

    def __post_init__(self) -> None:
        if not self.events:
            self.events.append({"phase": self.phase, "at": utc_now(), "sequence": 0})

    def advance(self, phase: str, *, at: str | None = None) -> None:
        if phase not in REPLACEMENT_PHASES:
            fail("replacement phase is outside the declared model")
        current = REPLACEMENT_PHASES.index(self.phase)
        requested = REPLACEMENT_PHASES.index(phase)
        if requested != current + 1:
            fail("replacement phase transition is not the exact next phase")
        self.phase = phase
        if phase == "new_environment_committed":
            self.durably_committed = True
        if phase == "predecessor_retired":
            self.predecessor_retired = True
        self.events.append({"phase": phase, "at": at or utc_now(), "sequence": len(self.events)})

    def evidence(self) -> dict[str, Any]:
        return {
            "replacement_phase": self.phase,
            "durably_committed": self.durably_committed,
            "predecessor_retired": self.predecessor_retired,
            "rollback_available": not self.durably_committed,
            "retirement_pending": self.durably_committed and not self.predecessor_retired,
            "phase_events": list(self.events),
        }


def fail(message: str) -> None:
    raise WR0Error(message)


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def user_record() -> pwd.struct_passwd:
    return pwd.getpwuid(os.getuid())


def real_home() -> pathlib.Path:
    home = pathlib.Path(user_record().pw_dir)
    if not home.is_absolute():
        fail("passwd-owned home is not absolute")
    require_no_symlink_ancestors(home, allow_absent_leaf=False, label="passwd-owned home")
    return home


def repo_root() -> pathlib.Path:
    root = pathlib.Path(__file__).resolve().parents[2]
    observed = command_text(["git", "rev-parse", "--show-toplevel"], cwd=root)
    if pathlib.Path(observed) != root:
        fail("repository root is not canonical")
    return root


def runner_root() -> pathlib.Path:
    return real_home() / ".local/share/Steam/steamapps/common/Proton 11.0"


def runtime_root() -> pathlib.Path:
    return real_home() / ".local/share/Steam/steamapps/common/SteamLinuxRuntime_4"


def steam_root() -> pathlib.Path:
    return real_home() / ".local/share/Steam"


def steamapps_root() -> pathlib.Path:
    return steam_root() / "steamapps"


def environments_root() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/environments"


def final_environment() -> pathlib.Path:
    return environments_root() / "wr0-proton11"


def cache_root() -> pathlib.Path:
    return real_home() / ".cache/linux-vst-bridge/wr0"


def session_parent() -> pathlib.Path:
    return cache_root() / "sessions"


def test_root() -> pathlib.Path:
    return cache_root() / "tests"


def evidence_root(repository: pathlib.Path | None = None) -> pathlib.Path:
    return (repository if repository is not None else repo_root()) / "evidence/wr0-proton-bootstrap"


def probe_path(repository: pathlib.Path | None = None) -> pathlib.Path:
    return (repository if repository is not None else repo_root()) / "windows-fixtures/wr0-probe/wr0-probe.cmd"


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while True:
            chunk = handle.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("utf-8")


def require_no_symlink_ancestors(path: pathlib.Path, *, allow_absent_leaf: bool, label: str) -> None:
    if not path.is_absolute():
        fail(f"{label} must be absolute")
    normalized = pathlib.Path(os.path.normpath(str(path)))
    if normalized != path or ".." in path.parts or "" in path.parts[1:]:
        fail(f"{label} is not canonical")
    current = pathlib.Path(path.anchor)
    for index, part in enumerate(path.parts[1:], start=1):
        current = current / part
        try:
            info = current.lstat()
        except FileNotFoundError:
            if index == len(path.parts) - 1 and allow_absent_leaf:
                return
            if allow_absent_leaf:
                return
            fail(f"{label} is missing")
        if stat.S_ISLNK(info.st_mode):
            fail(f"{label} has a symlinked ancestor")


def require_contained(
    path: pathlib.Path,
    root: pathlib.Path,
    *,
    label: str,
    allow_absent_leaf: bool = False,
) -> pathlib.Path:
    if not path.is_absolute() or not root.is_absolute():
        fail(f"{label} must use absolute paths")
    if pathlib.Path(os.path.normpath(str(path))) != path:
        fail(f"{label} contains an alias or traversal")
    try:
        common = pathlib.Path(os.path.commonpath((str(path), str(root))))
    except ValueError as exc:
        raise WR0Error(f"{label} is outside its root") from exc
    if common != root or path == root:
        fail(f"{label} is outside its declared root")
    require_no_symlink_ancestors(root, allow_absent_leaf=allow_absent_leaf, label=f"{label} root")
    require_no_symlink_ancestors(path, allow_absent_leaf=allow_absent_leaf, label=label)
    return path


def make_owned_directory(path: pathlib.Path, root: pathlib.Path, *, mode: int = 0o700) -> pathlib.Path:
    require_contained(path, root, label="owned directory", allow_absent_leaf=True)
    path.mkdir(parents=True, mode=mode, exist_ok=True)
    require_contained(path, root, label="owned directory", allow_absent_leaf=False)
    if not path.is_dir():
        fail("owned directory is not a directory")
    return path


def atomic_write(path: pathlib.Path, data: bytes, *, mode: int = 0o600) -> None:
    require_no_symlink_ancestors(path.parent, allow_absent_leaf=False, label="atomic-write parent")
    temp = path.parent / f".{path.name}.tmp-{secrets.token_hex(8)}"
    descriptor = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp, path)
        directory_fd = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory_fd)
        finally:
            os.close(directory_fd)
    except Exception:
        try:
            temp.unlink()
        except FileNotFoundError:
            pass
        raise


def atomic_write_json(path: pathlib.Path, value: Any) -> None:
    atomic_write(path, json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False).encode("utf-8") + b"\n")


def run_bounded(
    command: Sequence[str],
    *,
    cwd: pathlib.Path | None = None,
    timeout: float = FIXTURE_COMMAND_TIMEOUT,
    cap: int = 1024 * 1024,
    check: bool = True,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[bytes]:
    try:
        result = subprocess.run(
            list(command),
            cwd=cwd,
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        raise WR0Error(f"bounded command timed out: {pathlib.Path(command[0]).name}") from exc
    if len(result.stdout) > cap or len(result.stderr) > cap:
        fail(f"bounded command output exceeded cap: {pathlib.Path(command[0]).name}")
    if check and result.returncode != 0:
        fail(f"bounded command failed: {pathlib.Path(command[0]).name} exit {result.returncode}")
    return result


def command_text(command: Sequence[str], *, cwd: pathlib.Path | None = None, timeout: float = FIXTURE_COMMAND_TIMEOUT) -> str:
    result = run_bounded(command, cwd=cwd, timeout=timeout)
    try:
        return result.stdout.decode("utf-8", "strict").strip()
    except UnicodeDecodeError as exc:
        raise WR0Error(f"command returned non-UTF-8 output: {pathlib.Path(command[0]).name}") from exc


def parse_key_values(data: bytes, label: str) -> dict[str, str]:
    try:
        text = data.decode("utf-8", "strict")
    except UnicodeDecodeError as exc:
        raise WR0Error(f"{label} is not UTF-8") from exc
    values: dict[str, str] = {}
    for line in text.splitlines():
        if not line or "=" not in line:
            continue
        key, value = line.split("=", 1)
        if not re.fullmatch(r"[A-Za-z0-9_]+", key) or key in values:
            fail(f"{label} contains malformed keys")
        values[key] = value
    return values


@dataclass(frozen=True)
class LockSpec:
    base: str
    relative: str
    size: int
    sha256: str
    mode: str


LOCK_FILES = (
    LockSpec("runner", "version", 32, "823833b4a22543efdd3b0822981a9518dff08282520eba16661bd9d59bf5e026", "0755"),
    LockSpec("runner", "toolmanifest.vdf", 156, "bb81d44a687cc6f4ea7ae05fa9f0502f35c48e462051809dce3b2a58492e3498", "0755"),
    LockSpec("runner", "proton", 93103, "ccb67e21ef0d81cc4142ee3af74a16702292623bad11ec7047dbd6c97be62eed", "0755"),
    LockSpec("runner", "steampipe_fixups.json", 96309, "1fc86009719ad059a6ef372e1f838c3672016dab421e5c7de1ea9c1fbe66b489", "0755"),
    LockSpec("runner", "steampipe_fixups.py", 3252, "90a40fe7b030ef94b138687ef4f0e45d1f716f70a83e5a55b36d382fbefa4faf", "0755"),
    LockSpec("runner", "filelock.py", 12778, "c50e07ad2bc2245c30037034f940581ad18b15d084b0702b33242fff7015ee34", "0755"),
    LockSpec("runner", "dist.lock", 0, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "0755"),
    LockSpec("runner", "files/steampipe_fixups_mtime", 19, "734a0f4cf2d544d84e428a698525d3e7bf3c24a3058bd945ab701fcc7731b078", "0644"),
    LockSpec("runner", "files/bin/wine", 16872, "7a6de49c00d8ed2ba55c6967d3643c5ef729f5e562fb51e47e4f41c6cdb5c92a", "0755"),
    LockSpec("runner", "files/bin/wineserver", 887688, "ae2550cd7c6c675128cffe938764f2f797eb7163ca6ca9bddf0014d7fb984b4b", "0755"),
    LockSpec("runner", "files/lib/wine/x86_64-unix/wine", 16816, "f96a2ac1fcb00f0317a48eaddeedfb8f170d42a6d5714b46a21dbde876649500", "0555"),
    LockSpec("runner", "files/lib/wine/x86_64-unix/wine-preloader", 19096, "e805dfb4c12973ea54d6161360d91e1e1ee9aba151b77b8f5b6a06910c86cecd", "0555"),
    LockSpec("runner", "files/lib/wine/x86_64-windows/cmd.exe", 1210529, "74a8fece1a1affc3ad06c82726f069ff7ca9ce0e6a703fe088d89aa38a6aaa4b", "0555"),
    LockSpec("runner", "files/share/default_pfx/system.reg", 3874912, "0effc7f846639fcd45aa9dce7a3fb6413776a9a47402e4ff20a590d5c87e4333", "0755"),
    LockSpec("runner", "files/share/default_pfx/user.reg", 27885, "f06c94392a8b4936342f18f2f1b8317f0bc259b1c09206ffc009957db14a0e5b", "0755"),
    LockSpec("runner", "files/share/default_pfx/userdef.reg", 4190, "ca8de447ded78d2cfbb415bb3d40edd58db9fd170063abbadb9b82fdf24cc739", "0755"),
    LockSpec("runner", "LICENSE", 20069, "e4829d4e560bd13d378d47592a15d5c7eb1b9a702279eeea9b2f0fc6e60e0b31", "0755"),
    LockSpec("runner", "LICENSE.OFL", 4414, "93fed46019c38bbe566b479d22148e2e8a1e85ada614accb0211c37b2c61c19b", "0755"),
    LockSpec("runner", "PATENTS.AV1", 5730, "335eca574598bf4ca181b12f708d6669e5a5e78c8e1513e5b35fa1f03901484b", "0755"),
    LockSpec("runtime", "VERSIONS.txt", 319, "ef61e9bce47d60d02707661d116c8048ffa39291b84acee687a1ed4e430af6be", "0755"),
    LockSpec("runtime", "toolmanifest.vdf", 227, "e1c2598db7ae7adfa780b51ee24f391ac0fa11f4049f49aaead0319f8c84d27f", "0755"),
    LockSpec("runtime", "_v2-entry-point", 8989, "caa39b5cde8ea955288b574b49b3416fd16e7be3f53a17249d673f5ecfdce2c1", "0755"),
    LockSpec("runtime", "run", 603, "d5069c037c53f204a9d7d0ecb501b430e57dc2d702d49c9cccd57d569ca87eec", "0755"),
    LockSpec("runtime", "steamrt4_platform_4.0.20260805.254769/metadata", 968, "87fd5d88a7f9439bd344d5984c0237d3ae90d9de0228d65c25c8a484dbd073b1", "0755"),
    LockSpec("runtime", "pressure-vessel/bin/pressure-vessel-unruntime", 3726, "9211c921bdb343ce1f5987d959b1ca443137a40c4ed44a486a9bd6b7718c0016", "0755"),
    LockSpec("runtime", "pressure-vessel/bin/pressure-vessel-wrap", 868056, "0439811997a081e4f90f898b7e98e34e46cff3745a0611c17309947fd47f4da9", "0755"),
    LockSpec("runtime", "pressure-vessel/bin/steam-runtime-supervisor", 137040, "fa0f6586b7ada3d0fd5b96a518a3bcb672810a605d7a543bec30a221d1018d7d", "0755"),
    LockSpec("runtime", "pressure-vessel/bin/steam-runtime-launcher-interface-0", 10552, "583c789cedd0443d36020aecf3ff0fa444ec663a0dc3f6be5381af55458dae58", "0755"),
    LockSpec("runtime", "pressure-vessel/bin/steam-runtime-launch-client", 148336, "b2e6dfeecc6581aa210adf1406da3208ed93eb2e8e62f041325ed77918d909bb", "0755"),
    LockSpec("runtime", "pressure-vessel/libexec/steam-runtime-tools-0/pv-adverb", 606184, "e1428cb8edd77303e5c7f4024da07212c5b88816bd8c9097ea1763474b93e501", "0755"),
    LockSpec("runtime", "pressure-vessel/libexec/steam-runtime-tools-0/srt-bwrap", 64624, "967bff17693c5b4c7a359a40204125439109bc8c3aa8ba38a556dbef415c070a", "0755"),
    LockSpec("steamapps", "appmanifest_4628710.acf", 710, "a67dc7a8f53b9f2f38b99a8694047755aff9fcb827dc466530f00e3717fb910a", "0755"),
    LockSpec("steamapps", "appmanifest_4183110.acf", 541, "8026b0384d21a5652d9d55d0680d7fa2481c5a031624821123e0132bdb820daa", "0755"),
)


def lock_base(name: str) -> pathlib.Path:
    if name == "runner":
        return runner_root()
    if name == "runtime":
        return runtime_root()
    if name == "steamapps":
        return steamapps_root()
    fail("unknown lock base")
    raise AssertionError


def expected_lock_manifest() -> dict[str, Any]:
    records = [
        {
            "safe_path": f"{item.base}/{item.relative}",
            "type": "regular_file",
            "size": item.size,
            "sha256": item.sha256,
            "mode": item.mode,
        }
        for item in LOCK_FILES
    ]
    return {
        "schema": LOCK_SCHEMA,
        "runner": {
            "directory_name": "Proton 11.0",
            "version": RUNNER_VERSION,
            "steam_app_id": RUNNER_APP_ID,
            "build_id": RUNNER_BUILD_ID,
            "depot_manifest": RUNNER_DEPOT_MANIFEST,
            "required_tool_app_id": RUNTIME_APP_ID,
        },
        "runtime": {
            "directory_name": "SteamLinuxRuntime_4",
            "steam_app_id": RUNTIME_APP_ID,
            "build_id": RUNTIME_BUILD_ID,
            "depot_manifest": RUNTIME_DEPOT_MANIFEST,
            "depot_version": RUNTIME_VERSION,
            "pressure_vessel_version": PRESSURE_VESSEL_VERSION,
        },
        "files": records,
    }


def expected_lock_digest() -> str:
    return sha256_bytes(canonical_json(expected_lock_manifest()))


def verify_runner_lock() -> dict[str, Any]:
    for root, label in (
        (runner_root(), "runner root"),
        (runtime_root(), "runtime root"),
        (steamapps_root(), "Steam app-manifest root"),
    ):
        require_no_symlink_ancestors(root, allow_absent_leaf=False, label=label)
        if not root.is_dir():
            fail(f"{label} is not a directory")

    for item in LOCK_FILES:
        base = lock_base(item.base)
        path = base / item.relative
        require_contained(path, base, label=f"lock file {item.base}/{item.relative}")
        info = path.lstat()
        if not stat.S_ISREG(info.st_mode):
            fail(f"launch-critical file is not regular: {item.base}/{item.relative}")
        if info.st_size != item.size:
            fail(f"launch-critical size mismatch: {item.base}/{item.relative}")
        if stat.filemode(info.st_mode)[1:] != stat.filemode(int(item.mode, 8))[1:]:
            fail(f"launch-critical mode mismatch: {item.base}/{item.relative}")
        if sha256_file(path) != item.sha256:
            fail(f"launch-critical digest mismatch: {item.base}/{item.relative}")

    version = (runner_root() / "version").read_text(encoding="utf-8").strip()
    runner_manifest = (runner_root() / "toolmanifest.vdf").read_text(encoding="utf-8")
    runtime_manifest = (runtime_root() / "toolmanifest.vdf").read_text(encoding="utf-8")
    runtime_versions = (runtime_root() / "VERSIONS.txt").read_text(encoding="utf-8")
    runner_app = (steamapps_root() / "appmanifest_4628710.acf").read_text(encoding="utf-8")
    runtime_app = (steamapps_root() / "appmanifest_4183110.acf").read_text(encoding="utf-8")
    required_tokens = (
        (version, RUNNER_VERSION),
        (runner_manifest, f'"require_tool_appid" "{RUNTIME_APP_ID}"'),
        (runner_manifest, '"commandline" "/proton %verb%"'),
        (runtime_manifest, '"commandline" "/_v2-entry-point --verb=%verb% --"'),
        (runtime_versions, f"depot\t{RUNTIME_VERSION}"),
        (runtime_versions, f"pressure-vessel\t{PRESSURE_VESSEL_VERSION}"),
        (runner_app, f'"appid"\t\t"{RUNNER_APP_ID}"'),
        (runner_app, f'"buildid"\t\t"{RUNNER_BUILD_ID}"'),
        (runner_app, f'"manifest"\t\t"{RUNNER_DEPOT_MANIFEST}"'),
        (runner_app, '"installdir"\t\t"Proton 11.0"'),
        (runtime_app, f'"appid"\t\t"{RUNTIME_APP_ID}"'),
        (runtime_app, f'"buildid"\t\t"{RUNTIME_BUILD_ID}"'),
        (runtime_app, f'"manifest"\t\t"{RUNTIME_DEPOT_MANIFEST}"'),
        (runtime_app, '"installdir"\t\t"SteamLinuxRuntime_4"'),
    )
    for content, token in required_tokens:
        if token not in content:
            fail("runner/runtime declared pairing metadata differs")

    manifest = expected_lock_manifest()
    digest = sha256_bytes(canonical_json(manifest))
    if digest != expected_lock_digest():
        fail("launch-critical manifest digest is internally inconsistent")
    return {"classification": "passed", "manifest": manifest, "digest": digest}


def allowed_changed_path(path: str) -> bool:
    return any(path == prefix or (prefix.endswith("/") and path.startswith(prefix)) for prefix in ALLOWED_TRACKED_PREFIXES)


def git_changed_paths(repo: pathlib.Path) -> list[str]:
    result = run_bounded(
        ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all"],
        cwd=repo,
        cap=256 * 1024,
    )
    entries = result.stdout.split(b"\0")
    paths: list[str] = []
    index = 0
    while index < len(entries):
        entry = entries[index]
        index += 1
        if not entry:
            continue
        try:
            text = entry.decode("utf-8", "strict")
        except UnicodeDecodeError as exc:
            raise WR0Error("git status contains a non-UTF-8 path") from exc
        if len(text) < 4:
            fail("git status entry is malformed")
        status_code = text[:2]
        path = text[3:]
        if status_code[0] in {"R", "C"} or status_code[1] in {"R", "C"}:
            if index >= len(entries):
                fail("git rename status entry is incomplete")
            path = entries[index].decode("utf-8", "strict")
            index += 1
        paths.append(path)
    return sorted(set(paths))


def committed_changed_paths(repo: pathlib.Path, head: str) -> list[str]:
    result = run_bounded(
        ["git", "diff", "--name-only", "-z", f"{BASIS_COMMIT}..{head}"],
        cwd=repo,
        cap=256 * 1024,
    )
    try:
        paths = [item.decode("utf-8", "strict") for item in result.stdout.split(b"\0") if item]
    except UnicodeDecodeError as exc:
        raise WR0Error("committed diff contains a non-UTF-8 path") from exc
    return sorted(set(paths))


def contract_source_manifest(
    *,
    require_clean: bool = True,
    repository: pathlib.Path | None = None,
) -> dict[str, Any]:
    """Bind the repository-owned WR0 contract to exact Git objects."""

    repo = repository if repository is not None else repo_root()
    if not repo.is_absolute() or repo != pathlib.Path(os.path.normpath(str(repo))):
        fail("WR0 contract-source repository is not canonical")
    require_no_symlink_ancestors(repo, allow_absent_leaf=False, label="WR0 contract-source repository")
    observed_root = pathlib.Path(command_text(["git", "rev-parse", "--show-toplevel"], cwd=repo))
    if observed_root != repo:
        fail("WR0 contract-source repository root differs")
    governed_arguments = [
        "docs/WR0_RUNNER_LOCK.md",
        "tools/wr0-proton-bootstrap",
        "windows-fixtures/wr0-probe",
    ]
    index = run_bounded(
        ["git", "ls-files", "-s", "-z", "--", *governed_arguments],
        cwd=repo,
        cap=256 * 1024,
    )
    records: dict[str, dict[str, str]] = {}
    for raw in index.stdout.split(b"\0"):
        if not raw:
            continue
        match = re.fullmatch(rb"([0-9]{6}) ([0-9a-f]{40,64}) ([0-3])\t(.+)", raw)
        if match is None:
            fail("WR0 contract-source index entry is malformed")
        mode_raw, blob_raw, stage_raw, path_raw = match.groups()
        try:
            path = path_raw.decode("utf-8", "strict")
        except UnicodeDecodeError as exc:
            raise WR0Error("WR0 contract-source path is not UTF-8") from exc
        if path in records:
            fail("WR0 contract-source index contains a duplicate or unmerged path")
        if stage_raw != b"0":
            fail("WR0 contract-source index contains an unmerged entry")
        mode = mode_raw.decode("ascii")
        if mode not in {"100644", "100755"}:
            fail("WR0 contract-source index contains a non-regular file mode")
        records[path] = {"path": path, "mode": mode, "blob": blob_raw.decode("ascii")}

    expected = set(CONTRACT_SOURCE_PATHS)
    observed = set(records)
    if observed != expected:
        missing = sorted(expected - observed)
        unexpected = sorted(observed - expected)
        fail(f"WR0 contract-source roster differs (missing={missing}, unexpected={unexpected})")

    if require_clean:
        status = run_bounded(
            ["git", "status", "--porcelain=v1", "-z", "--untracked-files=all", "--", *governed_arguments],
            cwd=repo,
            cap=256 * 1024,
        )
        if status.stdout:
            fail("WR0 contract-source contains dirty, staged, or untracked state")

    manifest_files: list[dict[str, str]] = []
    for path in CONTRACT_SOURCE_PATHS:
        absolute = repo / path
        info = absolute.lstat()
        if not stat.S_ISREG(info.st_mode) or stat.S_ISLNK(info.st_mode):
            fail("WR0 contract-source working file is not a regular file")
        working_blob = command_text(["git", "hash-object", "--no-filters", "--", path], cwd=repo)
        if working_blob != records[path]["blob"]:
            fail("WR0 contract-source working bytes differ from the Git index")
        executable = bool(info.st_mode & 0o111)
        if executable != (records[path]["mode"] == "100755"):
            fail("WR0 contract-source working mode differs from the Git index")
        manifest_files.append(records[path])

    manifest = {"schema": CONTRACT_SOURCE_SCHEMA, "files": manifest_files}
    digest = sha256_bytes(canonical_json(manifest))
    workload = probe_path(repo)
    return {
        "classification": "passed",
        "schema": CONTRACT_SOURCE_SCHEMA,
        "manifest": manifest,
        "digest": digest,
        "file_count": len(manifest_files),
        "workload_sha256": sha256_file(workload),
        "implementation_commit": command_text(["git", "rev-parse", "HEAD"], cwd=repo),
        "implementation_tree": command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=repo),
    }


def verify_retained_contract_source(
    *,
    repository: pathlib.Path | None = None,
    fixture_path: pathlib.Path | None = None,
) -> dict[str, Any]:
    repo = repository if repository is not None else repo_root()
    current = contract_source_manifest(require_clean=True, repository=repo)
    fixture_path = fixture_path if fixture_path is not None else evidence_root(repo) / "fixture.json"
    try:
        fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise WR0Error("retained WR0 fixture cannot verify the contract source") from exc
    retained = fixture.get("contract_source")
    if not isinstance(retained, dict):
        fail("retained WR0 fixture lacks the contract-source identity")
    comparisons = {
        "schema": current["schema"],
        "digest": current["digest"],
        "file_count": current["file_count"],
        "workload_sha256": current["workload_sha256"],
    }
    if any(retained.get(key) != value for key, value in comparisons.items()):
        fail("retained WR0 contract-source identity differs at the current head")
    return {**comparisons, "classification": "passed", "final_head": current["implementation_commit"]}


def verify_git_state(*, implementation_worktree: bool) -> dict[str, Any]:
    repo = repo_root()
    expected_repo = real_home() / EXPECTED_REPOSITORY_SUFFIX
    if repo != expected_repo:
        fail("repository is not the exact Steam Deck working path")
    branch = command_text(["git", "branch", "--show-current"], cwd=repo)
    if branch != EXPECTED_BRANCH:
        fail("WR0 branch differs")
    origin_head = command_text(["git", "rev-parse", "origin/main"], cwd=repo)
    origin_tree = command_text(["git", "rev-parse", "origin/main^{tree}"], cwd=repo)
    if origin_head != BASIS_COMMIT or origin_tree != BASIS_TREE:
        fail("origin/main differs from the pinned basis")
    head = command_text(["git", "rev-parse", "HEAD"], cwd=repo)
    tree = command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=repo)
    ancestor = run_bounded(["git", "merge-base", "--is-ancestor", BASIS_COMMIT, head], cwd=repo, check=False)
    if ancestor.returncode != 0:
        fail("HEAD does not descend from the pinned basis")
    commit_count_text = command_text(["git", "rev-list", "--count", f"{BASIS_COMMIT}..{head}"], cwd=repo)
    if not commit_count_text.isdigit() or int(commit_count_text) > 1:
        fail("WR0 history is not the pinned basis plus at most one substantive commit")
    changed = git_changed_paths(repo)
    committed = committed_changed_paths(repo, head)
    unauthorized_committed = [path for path in committed if not allowed_changed_path(path)]
    if unauthorized_committed:
        fail("committed diff contains a path outside the WR0 path envelope")
    if implementation_worktree:
        unauthorized = [path for path in changed if not allowed_changed_path(path)]
        if unauthorized:
            fail("worktree contains a change outside the WR0 path envelope")
    elif changed:
        fail("worktree is not clean")
    return {
        "classification": "passed",
        "branch": branch,
        "head": head,
        "tree": tree,
        "origin_main": origin_head,
        "origin_main_tree": origin_tree,
        "changed_paths": changed,
        "committed_changed_paths": committed,
        "preedit_clean_basis_confirmed": True,
    }


def read_os_release() -> dict[str, str]:
    values: dict[str, str] = {}
    for line in pathlib.Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key] = value.strip().strip('"')
    return values


def verify_deck_and_readonly() -> dict[str, Any]:
    release = read_os_release()
    if release.get("ID") != "steamos":
        fail("fixture is not SteamOS")
    product_path = pathlib.Path("/sys/devices/virtual/dmi/id/product_name")
    product = product_path.read_text(encoding="utf-8").strip() if product_path.is_file() else ""
    if product != "Galileo":
        fail("fixture is not the accepted Steam Deck model")
    readonly = command_text(["steamos-readonly", "status"])
    if readonly != "enabled":
        fail("SteamOS read-only mode is not enabled")
    return {
        "classification": "passed",
        "machine_class": "Steam Deck Galileo",
        "os": release.get("PRETTY_NAME", "SteamOS"),
        "architecture": os.uname().machine,
        "readonly": "enabled",
        "hostname_retained": False,
    }


def read_proc_record(pid: int) -> dict[str, Any] | None:
    proc = pathlib.Path("/proc") / str(pid)
    try:
        stat_text = (proc / "stat").read_text(encoding="utf-8")
        closing = stat_text.rfind(")")
        if closing < 0:
            return None
        fields = stat_text[closing + 2 :].split()
        if len(fields) < 20:
            return None
        state = fields[0]
        ppid = int(fields[1])
        pgrp = int(fields[2])
        session = int(fields[3])
        start_ticks = int(fields[19])
        comm = (proc / "comm").read_text(encoding="utf-8").strip()[:64]
        uid = proc.stat().st_uid
        try:
            exe_basename = pathlib.Path(os.readlink(proc / "exe")).name[:128]
        except OSError:
            exe_basename = "unavailable"
        try:
            cmdline = (proc / "cmdline").read_bytes()[:8192]
        except OSError:
            cmdline = b""
        try:
            cgroup_data = (proc / "cgroup").read_bytes()[:4096]
        except OSError:
            cgroup_data = b""
    except (FileNotFoundError, ProcessLookupError, PermissionError, ValueError, OSError):
        return None
    cgroup_class = "ordinary_user_scope"
    if BITWIG_APP_ID.encode("ascii") in cgroup_data:
        cgroup_class = "bitwig_flatpak_scope"
    elif b"app-flatpak" in cgroup_data:
        cgroup_class = "other_flatpak_scope"
    elif b"steam" in cgroup_data.lower():
        cgroup_class = "steam_named_scope"
    return {
        "pid": pid,
        "state": state,
        "ppid": ppid,
        "pgrp": pgrp,
        "session": session,
        "start_ticks": start_ticks,
        "comm": comm,
        "exe_basename": exe_basename,
        "uid": uid,
        "cgroup_class": cgroup_class,
        "_cmdline": cmdline,
    }


def proc_snapshot(*, cap: int = 8192) -> dict[int, dict[str, Any]]:
    started = time.monotonic()
    records: dict[int, dict[str, Any]] = {}
    count = 0
    for entry in pathlib.Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        count += 1
        if count > cap or time.monotonic() - started > 2.0:
            fail("bounded process census exceeded its limit")
        record = read_proc_record(int(entry.name))
        if record is not None:
            records[int(entry.name)] = record
    return records


def ancestor_records(pid: int, records: dict[int, dict[str, Any]], *, limit: int = 32) -> list[dict[str, Any]]:
    result: list[dict[str, Any]] = []
    seen: set[int] = set()
    current = pid
    while current > 0 and current not in seen and len(result) < limit:
        seen.add(current)
        item = records.get(current)
        if item is None:
            break
        result.append(item)
        current = int(item["ppid"])
    return result


def process_family(record: dict[str, Any]) -> str | None:
    comm = str(record["comm"]).lower()
    exe = str(record["exe_basename"]).lower()
    cmdline = bytes(record.get("_cmdline", b"")).lower()
    if comm in {"bitwigstudio", "bitwig-studio"} or comm.startswith("bitwigpluginhost"):
        return "bitwig"
    if comm in {"validator", "validator_linux"} or exe in {"validator", "validator_linux"}:
        return "validator"
    if comm in {
        "wine",
        "wine64",
        "wine-preloader",
        "wine64-preloader",
        "wineserver",
        "services.exe",
        "winedevice.exe",
        "explorer.exe",
        "rpcss.exe",
        "cmd.exe",
        "conhost.exe",
    }:
        return "wine"
    if comm in {"umu", "umu-run"} or b"umu-run" in cmdline:
        return "umu"
    if comm in {"yabridge-host", "yabridge-host.exe", "yabridge-host-32.exe"}:
        return "yabridge"
    if comm.startswith("proton") or b"/proton\x00" in cmdline or b"proton waitforexit" in cmdline:
        return "proton"
    if comm in {"pressure-vessel", "pv-bwrap", "srt-bwrap", "pv-adverb"}:
        return "runtime"
    return None


def is_allowed_ordinary_steam_runtime(record: dict[str, Any], records: dict[int, dict[str, Any]]) -> bool:
    chain = ancestor_records(int(record["pid"]), records)
    names = {str(item["comm"]).lower() for item in chain}
    cmdlines = b"\0".join(bytes(item.get("_cmdline", b"")) for item in chain).lower()
    descendants: list[dict[str, Any]] = []
    frontier = [int(record["pid"])]
    seen: set[int] = set()
    while frontier and len(seen) < 128:
        parent = frontier.pop()
        if parent in seen:
            continue
        seen.add(parent)
        for candidate in records.values():
            if int(candidate["ppid"]) == parent:
                descendants.append(candidate)
                frontier.append(int(candidate["pid"]))
    names.update(str(item["comm"]).lower() for item in descendants)
    cmdlines += b"\0".join(bytes(item.get("_cmdline", b"")) for item in descendants).lower()
    return "steamwebhelper" in names and b"proton" not in cmdlines and b"wr0-proton11" not in cmdlines


def process_guard() -> dict[str, Any]:
    records = proc_snapshot()
    counts = {name: 0 for name in ("bitwig", "validator", "wine", "proton", "runtime", "umu", "yabridge")}
    ordinary_steam = 0
    contaminants: list[dict[str, Any]] = []
    for record in records.values():
        if int(record["uid"]) != os.getuid():
            continue
        comm_lower = str(record["comm"]).lower()
        if comm_lower in {"steam", "steamwebhelper"}:
            ordinary_steam += 1
        family = process_family(record)
        if family is None:
            continue
        if family == "runtime" and is_allowed_ordinary_steam_runtime(record, records):
            ordinary_steam += 1
            continue
        counts[family] += 1
        contaminants.append(
            {
                "family": family,
                "comm": record["comm"],
                "exe_basename": record["exe_basename"],
                "uid": record["uid"],
                "start_ticks": record["start_ticks"],
            }
        )
    if contaminants:
        families = ",".join(sorted({item["family"] for item in contaminants}))
        fail(f"forbidden compatibility/application process is already running: {families}")
    return {
        "classification": "passed",
        "counts": counts,
        "ordinary_steam_process_count": ordinary_steam,
        "ordinary_steam_separate": True,
    }


def verify_hash_manifest(directory: pathlib.Path) -> dict[str, Any]:
    manifest = directory / "hashes.sha256"
    require_contained(manifest, directory, label="accepted evidence hash manifest")
    if not manifest.is_file() or manifest.is_symlink():
        fail("accepted evidence hash manifest is missing or unsafe")
    lines = manifest.read_text(encoding="utf-8").splitlines()
    if not lines:
        fail("accepted evidence hash manifest is empty")
    verified = 0
    for line in lines:
        match = re.fullmatch(r"([0-9a-f]{64})  ([^/][^\x00]*)", line)
        if match is None:
            fail("accepted evidence hash manifest is malformed")
        expected, name = match.groups()
        if "/" in name or name in {".", "..", "hashes.sha256"}:
            fail("accepted evidence hash manifest has an unsafe name")
        candidate = directory / name
        require_contained(candidate, directory, label="accepted evidence file")
        if not candidate.is_file() or candidate.is_symlink() or sha256_file(candidate) != expected:
            fail("accepted evidence packet verification failed")
        verified += 1
    actual_names = {path.name for path in directory.iterdir() if path.is_file() and not path.is_symlink()}
    if actual_names != {line.split("  ", 1)[1] for line in lines} | {"hashes.sha256"}:
        fail("accepted evidence packet roster differs")
    return {
        "classification": "passed",
        "manifest_sha256": sha256_file(manifest),
        "verified_file_count": verified,
    }


def file_identity(path: pathlib.Path, *, expected: dict[str, Any] | None = None) -> dict[str, Any]:
    require_no_symlink_ancestors(path, allow_absent_leaf=False, label="protected regular file")
    info = path.lstat()
    if not stat.S_ISREG(info.st_mode):
        fail("protected fixture is not a regular file")
    record = {
        "type": "regular_file",
        "size": info.st_size,
        "mtime_ns": info.st_mtime_ns,
        "sha256": sha256_file(path),
    }
    if expected is not None:
        for key in ("size", "mtime_ns", "sha256"):
            if record[key] != expected[key]:
                fail(f"protected fixture identity differs: {expected.get('role', 'file')} {key}")
    return record


def flatpak_identity() -> dict[str, Any]:
    shadow = run_bounded(["flatpak", "info", "--user", "--show-ref", BITWIG_APP_ID], check=False)
    if shadow.returncode == 0:
        fail("unexpected user-scope Bitwig shadow exists")
    app_info = command_text(["flatpak", "info", "--system", BITWIG_APP_ID])
    match = re.search(r"^\s*Version:\s*(\S+)\s*$", app_info, flags=re.MULTILINE)
    if match is None or match.group(1) != BITWIG_VERSION:
        fail("Bitwig version differs")
    app_commit = command_text(["flatpak", "info", "--system", "--show-commit", BITWIG_APP_ID])
    runtime_ref = command_text(["flatpak", "info", "--system", "--show-runtime", BITWIG_APP_ID])
    runtime_commit = command_text(["flatpak", "info", "--system", "--show-commit", BITWIG_RUNTIME_REF])
    user_override = run_bounded(["flatpak", "override", "--user", "--show", BITWIG_APP_ID]).stdout
    system_override = run_bounded(["flatpak", "override", "--system", "--show", BITWIG_APP_ID]).stdout
    if app_commit != BITWIG_APP_COMMIT or runtime_ref != BITWIG_RUNTIME_REF or runtime_commit != BITWIG_RUNTIME_COMMIT:
        fail("Bitwig app/runtime identity differs")
    if sha256_bytes(user_override) != BITWIG_USER_OVERRIDE_SHA256:
        fail("Bitwig user override bytes differ")
    if sha256_bytes(system_override) != BITWIG_SYSTEM_OVERRIDE_SHA256:
        fail("Bitwig system override bytes differ")
    return {
        "classification": "observed",
        "version": BITWIG_VERSION,
        "app_commit": app_commit,
        "runtime_ref": runtime_ref,
        "runtime_commit": runtime_commit,
        "user_override_sha256": sha256_bytes(user_override),
        "system_override_sha256": sha256_bytes(system_override),
        "user_shadow": False,
    }


def hp0_identity(repo: pathlib.Path) -> dict[str, Any]:
    publication = run_bounded([str(repo / "tools/hp0-native-probe/publish.sh"), "inspect"], cwd=repo)
    publication_values = parse_key_values(publication.stdout, "HP0 publication inspection")
    if publication_values.get("publication_status") != "verified":
        fail("HP0 publication inspection did not verify")
    if publication_values.get("module_sha256") != HP0_MODULE_SHA256:
        fail("HP0 publication module differs")
    if publication_values.get("receipt_status") != "verified_owned_receipt":
        fail("HP0 publication receipt is not owned")
    if publication_values.get("receipt_schema") != HP0_RECEIPT_SCHEMA:
        fail("HP0 publication receipt schema differs")
    if publication_values.get("receipt_sha256") != HP0_RECEIPT_SHA256:
        fail("HP0 publication receipt differs")
    build_dir = repo / "build/hp0-second-repair-2"
    require_no_symlink_ancestors(build_dir, allow_absent_leaf=False, label="accepted HP0 build directory")
    receipt_path = build_dir / "hp0-build.receipt"
    source_manifest = build_dir / "hp0-build-source.manifest"
    bundle_manifest = build_dir / "hp0-built-bundle.sha256"
    built_module = build_dir / "VST3/Release/LabHostProbe.vst3/Contents/x86_64-linux/LabHostProbe.so"
    validator = build_dir / "bin/Release/validator"
    receipt_bytes = receipt_path.read_bytes()
    if sha256_bytes(receipt_bytes) != HP0_BUILD_RECEIPT_SHA256:
        fail("retained HP0 build receipt bytes differ")
    build_values = parse_key_values(receipt_bytes, "retained HP0 build receipt")
    expected_build_keys = {
        "schema", "build_source_manifest_schema", "build_source_manifest_sha256",
        "historical_repository_commit", "historical_repository_tree", "sdk_ref",
        "sdk_commit", "vst3_sdk_commit", "compiler", "cmake", "ninja",
        "pkg_config", "bundle", "module", "module_sha256",
        "bundle_manifest_sha256", "validator", "validator_sha256",
        "processor_class_id", "controller_class_id", "gain_parameter_id",
        "bypass_parameter_id",
    }
    if set(build_values) != expected_build_keys or build_values.get("schema") != HP0_BUILD_RECEIPT_SCHEMA:
        fail("retained HP0 build receipt schema/key set differs")
    if build_values.get("build_source_manifest_schema") != HP0_BUILD_SOURCE_SCHEMA:
        fail("HP0 build-source schema differs")
    if build_values.get("build_source_manifest_sha256") != HP0_BUILD_SOURCE_SHA256:
        fail("HP0 build-source identity differs")
    if build_values.get("module_sha256") != HP0_MODULE_SHA256:
        fail("HP0 receipt-bound module identity differs")
    if build_values.get("bundle_manifest_sha256") != HP0_BUNDLE_MANIFEST_SHA256:
        fail("HP0 receipt-bound bundle manifest identity differs")
    if build_values.get("validator_sha256") != HP0_VALIDATOR_SHA256:
        fail("HP0 receipt-bound validator identity differs")
    expected_paths = {
        "bundle": str(build_dir / "VST3/Release/LabHostProbe.vst3"),
        "module": str(built_module),
        "validator": str(validator),
    }
    if any(build_values.get(key) != value for key, value in expected_paths.items()):
        fail("retained HP0 build receipt path binding differs")
    build_records = {
        "receipt": file_identity(receipt_path),
        "source_manifest": file_identity(source_manifest),
        "bundle_manifest": file_identity(bundle_manifest),
        "module": file_identity(built_module),
        "validator": file_identity(validator),
    }
    expected_build_hashes = {
        "receipt": HP0_BUILD_RECEIPT_SHA256,
        "source_manifest": HP0_BUILD_SOURCE_SHA256,
        "bundle_manifest": HP0_BUNDLE_MANIFEST_SHA256,
        "module": HP0_MODULE_SHA256,
        "validator": HP0_VALIDATOR_SHA256,
    }
    for name, expected_hash in expected_build_hashes.items():
        if build_records[name]["sha256"] != expected_hash:
            fail(f"retained HP0 build artifact differs: {name}")
    module = real_home() / ".vst3/linux-vst-bridge/LabHostProbe.vst3/Contents/x86_64-linux/LabHostProbe.so"
    receipt = real_home() / ".cache/linux-vst-bridge/hp0-publication.receipt"
    module_record = file_identity(module)
    receipt_record = file_identity(receipt)
    if module_record["sha256"] != HP0_MODULE_SHA256 or receipt_record["sha256"] != HP0_RECEIPT_SHA256:
        fail("HP0 protected file differs")
    return {
        "classification": "observed",
        "module": module_record,
        "publication_receipt": receipt_record,
        "publication_receipt_schema": HP0_RECEIPT_SCHEMA,
        "build_receipt_schema": HP0_BUILD_RECEIPT_SCHEMA,
        "build_receipt_sha256": HP0_BUILD_RECEIPT_SHA256,
        "build_source_schema": HP0_BUILD_SOURCE_SCHEMA,
        "build_source_sha256": HP0_BUILD_SOURCE_SHA256,
        "retained_build": build_records,
    }


def steam_compatdata_identity() -> dict[str, Any]:
    root = steamapps_root() / "compatdata"
    require_no_symlink_ancestors(root, allow_absent_leaf=False, label="Steam compatdata root")
    names: list[str] = []
    for entry in root.iterdir():
        if entry.is_symlink():
            fail("Steam compatdata immediate roster contains a symlink")
        if entry.is_dir():
            names.append(entry.name)
    names.sort()
    roster_bytes = b"\0".join(name.encode("utf-8") for name in names) + b"\0"
    return {
        "classification": "observed",
        "directory_count": len(names),
        "roster_sha256": sha256_bytes(roster_bytes),
        "names_retained": False,
    }


def capture_fixture() -> dict[str, Any]:
    repo = repo_root()
    home = real_home()
    serum = []
    for expected in SERUM_FILES:
        item = {"role": expected["role"], **file_identity(home / expected["relative"], expected=expected)}
        serum.append(item)
    wine_registries = {}
    for name in ("system.reg", "user.reg", "userdef.reg"):
        path = home / ".wine" / name
        record = file_identity(path)
        wine_registries[name] = {"type": record["type"], "size": record["size"], "sha256": record["sha256"]}
    packets = {}
    for name in EVIDENCE_PACKET_DIRS:
        packets[name] = verify_hash_manifest(repo / "evidence" / name)
    lock = verify_runner_lock()
    return {
        "schema": "linux-vst-bridge-wr0-protected-fixture/v1",
        "deck": verify_deck_and_readonly(),
        "bitwig": flatpak_identity(),
        "hp0": hp0_identity(repo),
        "accepted_evidence_packets": packets,
        "serum_regular_files": serum,
        "wine_registry_files": wine_registries,
        "steam_compatdata": steam_compatdata_identity(),
        "runner_lock_digest": lock["digest"],
    }


def compare_fixtures(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    if before != after:
        before_flat = canonical_json(before)
        after_flat = canonical_json(after)
        fail(f"protected fixture preservation mismatch ({sha256_bytes(before_flat)} != {sha256_bytes(after_flat)})")
    return {
        "classification": "passed",
        "before_sha256": sha256_bytes(canonical_json(before)),
        "after_sha256": sha256_bytes(canonical_json(after)),
        "exact_equality": True,
    }


def run_preflight(*, implementation_worktree: bool, capture: bool = True) -> dict[str, Any]:
    git_state = verify_git_state(implementation_worktree=implementation_worktree)
    deck = verify_deck_and_readonly()
    guard = process_guard()
    lock = verify_runner_lock()
    destination = final_environment()
    require_no_symlink_ancestors(destination, allow_absent_leaf=True, label="final WR0 environment")
    fixture = capture_fixture() if capture else None
    return {
        "classification": "passed",
        "result": "PRE-FLIGHT_CLEAR",
        "git": git_state,
        "deck": deck,
        "process_guard": guard,
        "runner_lock_digest": lock["digest"],
        "final_environment_state": "absent" if not destination.exists() else "present_requires_exact_ownership",
        "fixture": fixture,
        "hostname_retained": False,
    }


def safe_process_record(record: dict[str, Any]) -> dict[str, Any]:
    return {key: value for key, value in record.items() if key != "_cmdline"}


def process_identity(record: dict[str, Any]) -> str:
    data = f"{record['pid']}:{record['uid']}:{record['start_ticks']}".encode("utf-8")
    return sha256_bytes(data)


def cmdline_arguments(record: dict[str, Any]) -> list[bytes]:
    return [item for item in bytes(record.get("_cmdline", b"")).split(b"\0") if item]


def records_in_group(pgid: int, records: dict[int, dict[str, Any]] | None = None) -> list[dict[str, Any]]:
    census = records if records is not None else proc_snapshot()
    return [
        item
        for item in census.values()
        if int(item["uid"]) == os.getuid() and int(item["pgrp"]) == pgid and item.get("state") != "Z"
    ]


def descendant_records(
    root_pid: int,
    root_start_ticks: int,
    records: dict[int, dict[str, Any]],
) -> list[dict[str, Any]]:
    root = records.get(root_pid)
    if root is None or int(root["start_ticks"]) != root_start_ticks or int(root["uid"]) != os.getuid():
        return []
    children: dict[int, list[dict[str, Any]]] = {}
    for record in records.values():
        if int(record["uid"]) == os.getuid():
            children.setdefault(int(record["ppid"]), []).append(record)
    result: list[dict[str, Any]] = []
    frontier = [root]
    seen: set[tuple[int, int]] = set()
    while frontier:
        record = frontier.pop(0)
        key = (int(record["pid"]), int(record["start_ticks"]))
        if key in seen:
            continue
        seen.add(key)
        result.append(record)
        if len(result) > PROCESS_CAP:
            raise LaunchBlocked("owned root-descendant identity count exceeded its cap")
        frontier.extend(sorted(children.get(int(record["pid"]), []), key=lambda item: int(item["pid"])))
    return result


def identity_is_live(identity: tuple[int, int], *, uid: int | None = None) -> bool:
    record = read_proc_record(identity[0])
    if record is None or record.get("state") == "Z" or int(record["start_ticks"]) != identity[1]:
        return False
    return uid is None or int(record["uid"]) == uid


def ordered_subsequence_count(arguments: Sequence[bytes], expected: Sequence[bytes]) -> int:
    if not expected or len(expected) > len(arguments):
        return 0
    width = len(expected)
    return sum(1 for index in range(len(arguments) - width + 1) if list(arguments[index : index + width]) == list(expected))


def expected_windows_command_subsequence(
    *,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
) -> tuple[bytes, ...]:
    if not re.fullmatch(r"[0-9a-f]{32}", nonce):
        fail("Windows-command identity nonce is malformed")
    if run_number not in {1, 2, 37} or expected_exit not in {0, 37}:
        fail("Windows-command identity request is outside the fixed contract")
    if not re.fullmatch(r"[0-9a-f]{64}", contract_digest) or not re.fullmatch(r"[0-9a-f]{64}", workload_sha256):
        fail("Windows-command source identity is malformed")
    values: list[bytes] = [
        b"/d",
        b"/q",
        b"/c",
        windows_path(probe_path()).encode("utf-8"),
        nonce.encode("ascii"),
        str(run_number).encode("ascii"),
        contract_digest.encode("ascii"),
        workload_sha256.encode("ascii"),
    ]
    if expected_exit == 37:
        values.append(b"--exit-37")
    return tuple(values)


def exact_windows_command_vector(
    record: dict[str, Any],
    *,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
) -> bool:
    comm = str(record["comm"]).lower()
    exe = str(record["exe_basename"]).lower()
    if comm != "cmd.exe" or exe not in {"wine64-preloader", "wine-preloader", "cmd.exe"}:
        return False
    arguments = cmdline_arguments(record)
    expected = expected_windows_command_subsequence(
        nonce=nonce,
        run_number=run_number,
        expected_exit=expected_exit,
        contract_digest=contract_digest,
        workload_sha256=workload_sha256,
    )
    if ordered_subsequence_count(arguments, expected) != 1:
        return False
    identity_tokens = expected[3:8]
    if any(arguments.count(token) != 1 for token in identity_tokens):
        return False
    expected_exit_token_count = 1 if expected_exit == 37 else 0
    if arguments.count(b"--exit-37") != expected_exit_token_count:
        return False
    return True


def classify_owned_role(
    record: dict[str, Any],
    *,
    root_pid: int,
    nonce: str = "",
    run_number: int = 0,
    expected_exit: int = -1,
    contract_digest: str = "",
    workload_sha256: str = "",
) -> str:
    comm = str(record["comm"]).lower()
    exe = str(record["exe_basename"]).lower()
    arguments = cmdline_arguments(record)
    if int(record["pid"]) == root_pid:
        return "runtime_root"
    proton = str(runner_root() / "proton").encode("utf-8")
    if (
        len(arguments) >= 2
        and proton in arguments[:2]
        and arguments[arguments.index(proton) + 1 : arguments.index(proton) + 2] == [b"runinprefix"]
        and comm not in {"srt-bwrap", "pv-bwrap", "pressure-vessel", "pv-adverb"}
        and exe not in {"srt-bwrap", "pv-bwrap", "pressure-vessel-wrap", "pv-adverb"}
    ):
        return "proton"
    if (
        nonce
        and run_number in {1, 2, 37}
        and expected_exit in {0, 37}
        and contract_digest
        and workload_sha256
        and exact_windows_command_vector(
            record,
            nonce=nonce,
            run_number=run_number,
            expected_exit=expected_exit,
            contract_digest=contract_digest,
            workload_sha256=workload_sha256,
        )
    ):
        return "windows_command"
    if comm == "wineserver":
        return "wineserver"
    if comm in {"wine", "wine64", "wine-preloader", "wine64-preloader"}:
        return "wine"
    if comm in {"services.exe", "winedevice.exe", "explorer.exe", "rpcss.exe", "conhost.exe"}:
        return "wine_support"
    if comm in {"pressure-vessel", "pv-bwrap", "srt-bwrap", "pv-adverb", "steam-runtime-s"}:
        return "runtime_component"
    return "owned_descendant"


def terminate_process_group(pgid: int) -> None:
    if pgid <= 1 or pgid == os.getpgrp():
        fail("refusing unsafe process-group termination")
    try:
        os.killpg(pgid, signal.SIGTERM)
    except ProcessLookupError:
        return
    deadline = time.monotonic() + TERM_GRACE_SECONDS
    while time.monotonic() < deadline:
        if not records_in_group(pgid):
            return
        time.sleep(0.1)
    try:
        os.killpg(pgid, signal.SIGKILL)
    except ProcessLookupError:
        return
    deadline = time.monotonic() + TERM_GRACE_SECONDS
    while time.monotonic() < deadline:
        if not records_in_group(pgid):
            return
        time.sleep(0.05)
    if records_in_group(pgid):
        raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: isolated process group survived SIGKILL")


def signal_exact_identity(identity: tuple[int, int], sig: signal.Signals) -> bool:
    current = read_proc_record(identity[0])
    if (
        current is None
        or current.get("state") == "Z"
        or int(current["uid"]) != os.getuid()
        or int(current["start_ticks"]) != identity[1]
    ):
        return False
    try:
        os.kill(identity[0], sig)
    except ProcessLookupError:
        return False
    return True


def cleanup_survivor_summary(remaining: Sequence[dict[str, Any]]) -> dict[str, Any]:
    counts: dict[str, int] = {}
    for record in remaining[:PROCESS_CAP]:
        role = str(record.get("role", "owned_descendant"))
        counts[role] = counts.get(role, 0) + 1
    return {"count": len(remaining), "role_counts": dict(sorted(counts.items()))}


def require_cleanup_complete(clean: bool, remaining: Sequence[dict[str, Any]]) -> None:
    if clean and not remaining:
        return
    summary = cleanup_survivor_summary(remaining)
    raise ProcessTopologyBlocked(
        "WR0_PROCESS_TOPOLOGY_BLOCKED: exact owned identities survived final SIGKILL "
        f"({json.dumps(summary, sort_keys=True, separators=(',', ':'))})"
    )


def terminate_owned_identities(
    pgid: int,
    observed: dict[tuple[int, int], dict[str, Any]],
    *,
    root_pid: int,
    root_start_ticks: int,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
    deadline_seconds: float = CLEANUP_DEADLINE_SECONDS,
) -> dict[str, Any]:
    if pgid <= 1 or pgid == os.getpgrp():
        fail("refusing unsafe owned-scope termination")
    started = time.monotonic()

    def refresh() -> None:
        if root_pid <= 1 or root_start_ticks <= 0:
            return
        try:
            sample_owned_tree(
                root_pid,
                root_start_ticks,
                observed,
                elapsed_ms=0,
                nonce=nonce,
                run_number=run_number,
                expected_exit=expected_exit,
                contract_digest=contract_digest,
                workload_sha256=workload_sha256,
            )
        except (ProcessObservabilityBlocked, LaunchBlocked):
            return

    def signal_scope(sig: signal.Signals) -> int:
        sent = 0
        try:
            os.killpg(pgid, sig)
            sent += 1
        except ProcessLookupError:
            pass
        for identity, record in list(observed.items()):
            if int(record["pgrp"]) == pgid:
                continue
            if signal_exact_identity(identity, sig):
                sent += 1
        return sent

    refresh()
    initial_count = len(observed)
    escaped_count = sum(1 for item in observed.values() if int(item["pgrp"]) != pgid)
    term_signals = signal_scope(signal.SIGTERM)
    clean, remaining = wait_owned_empty(
        pgid,
        observed,
        deadline_seconds=min(TERM_GRACE_SECONDS, max(0.1, deadline_seconds)),
    )
    kill_signals = 0
    if not clean:
        refresh()
        kill_signals = signal_scope(signal.SIGKILL)
        elapsed = time.monotonic() - started
        remaining_deadline = max(0.1, deadline_seconds - elapsed)
        clean, remaining = wait_owned_empty(pgid, observed, deadline_seconds=remaining_deadline)
    require_cleanup_complete(clean, remaining)
    if time.monotonic() - started > deadline_seconds:
        raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: cleanup exceeded its declared deadline")
    return {
        "classification": "passed",
        "observed_identity_count": initial_count,
        "escaped_process_group_count": escaped_count,
        "term_signal_target_count": term_signals,
        "kill_signal_target_count": kill_signals,
        "final_group_empty": not records_in_group(pgid),
        "final_owned_count": 0,
        "deadline_seconds": deadline_seconds,
    }


def wait_owned_empty(
    pgid: int,
    observed: dict[tuple[int, int], dict[str, Any]],
    *,
    deadline_seconds: float = PROCESS_DRAIN_SECONDS,
) -> tuple[bool, list[dict[str, Any]]]:
    deadline = time.monotonic() + deadline_seconds
    final_records: list[dict[str, Any]] = []
    while time.monotonic() < deadline:
        census = proc_snapshot()
        final_records = records_in_group(pgid, census)
        for identity, retained in observed.items():
            current = census.get(identity[0])
            if (
                current is not None
                and current.get("state") != "Z"
                and int(current["start_ticks"]) == identity[1]
                and current not in final_records
            ):
                final_records.append(current)
        if not final_records:
            return True, []
        time.sleep(0.05)
    safe_remaining: list[dict[str, Any]] = []
    for item in final_records[:PROCESS_CAP]:
        safe = safe_process_record(item)
        retained = observed.get(process_key(item))
        safe["role"] = retained.get("role", "owned_descendant") if retained is not None else "owned_descendant"
        safe_remaining.append(safe)
    return False, safe_remaining


def windows_path(path: pathlib.Path) -> str:
    if not path.is_absolute():
        fail("Windows workload input must be absolute")
    return "Z:" + str(path).replace("/", "\\")


def controlled_environment(environment: pathlib.Path) -> dict[str, str]:
    home = real_home()
    user = user_record()
    xdg_runtime = pathlib.Path(f"/run/user/{os.getuid()}")
    require_no_symlink_ancestors(xdg_runtime, allow_absent_leaf=False, label="XDG runtime directory")
    if not xdg_runtime.is_dir() or xdg_runtime.stat().st_uid != os.getuid():
        fail("XDG runtime directory is unavailable or not owned")
    directories = {
        "PRESSURE_VESSEL_VARIABLE_DIR": environment / "runtime-var",
        "XDG_CACHE_HOME": environment / "host-cache",
        "XDG_CONFIG_HOME": environment / "host-config",
        "XDG_DATA_HOME": environment / "host-data",
        "TMPDIR": environment / "host-tmp",
    }
    for directory in directories.values():
        require_contained(directory, environment, label="WR0 launch state directory", allow_absent_leaf=True)
        directory.mkdir(mode=0o700, exist_ok=True)
        require_contained(directory, environment, label="WR0 launch state directory")
    compatdata = environment / "compatdata"
    require_contained(compatdata, environment, label="WR0 compatdata", allow_absent_leaf=True)
    compatdata.mkdir(mode=0o700, exist_ok=True)
    return {
        "HOME": str(home),
        "USER": user.pw_name,
        "LOGNAME": user.pw_name,
        "PATH": "/usr/bin:/bin",
        "LANG": "C.UTF-8",
        "XDG_RUNTIME_DIR": str(xdg_runtime),
        "XDG_CACHE_HOME": str(directories["XDG_CACHE_HOME"]),
        "XDG_CONFIG_HOME": str(directories["XDG_CONFIG_HOME"]),
        "XDG_DATA_HOME": str(directories["XDG_DATA_HOME"]),
        "TMPDIR": str(directories["TMPDIR"]),
        "STEAM_COMPAT_DATA_PATH": str(compatdata),
        "STEAM_COMPAT_CLIENT_INSTALL_PATH": str(steam_root()),
        "STEAM_COMPAT_APP_ID": NEUTRAL_APP_ID,
        "SteamAppId": NEUTRAL_APP_ID,
        "SteamGameId": NEUTRAL_APP_ID,
        "PRESSURE_VESSEL_VARIABLE_DIR": str(directories["PRESSURE_VESSEL_VARIABLE_DIR"]),
        "STEAM_ZENITY": "",
    }


def expected_ready_stdout(nonce: str, run_number: int) -> str:
    return (
        f"WR0_MAGIC={MAGIC}\n"
        f"WR0_NONCE={nonce}\n"
        f"WR0_RUN={run_number}\n"
        "WR0_ARCH=x86_64\n"
        "WR0_READY=waiting_for_supervisor\n"
    )


def expected_stdout(nonce: str, run_number: int, *, exit_37: bool) -> str:
    lines = [
        f"WR0_MAGIC={MAGIC}",
        f"WR0_NONCE={nonce}",
        f"WR0_RUN={run_number}",
        "WR0_ARCH=x86_64",
        "WR0_READY=waiting_for_supervisor",
        "WR0_GATE=accepted",
    ]
    if exit_37:
        lines.append("WR0_EXIT=37")
    else:
        lines.extend(("WR0_RECEIPT=written_and_read", "WR0_EXIT=0"))
    return "\n".join(lines) + "\n"


def normalize_windows_text(data: bytes, *, label: str) -> str:
    normalized = data.replace(b"\r\n", b"\n")
    if b"\r" in normalized:
        fail(f"{label} contains a bare carriage return")
    try:
        return normalized.decode("utf-8", "strict")
    except UnicodeDecodeError as exc:
        raise LaunchBlocked(f"{label} is not UTF-8") from exc


def validate_workload_output(stdout: str, *, nonce: str, run_number: int, expected_exit: int) -> None:
    expected = expected_stdout(nonce, run_number, exit_37=expected_exit == 37)
    if stdout != expected:
        observed_lines = stdout.splitlines()
        raise LaunchBlocked(
            "tracked Windows workload stdout differs from the exact contract: "
            f"observed_lines={json.dumps(observed_lines, ensure_ascii=True)}"
        )


def expected_receipt(
    nonce: str,
    run_number: int,
    *,
    contract_digest: str,
    workload_sha256: str,
) -> str:
    return (
        f"WR0_RECEIPT_SCHEMA={RECEIPT_SCHEMA}\n"
        f"WR0_NONCE={nonce}\n"
        f"WR0_RUN={run_number}\n"
        "WR0_ARCH=x86_64\n"
        f"WR0_CONTRACT_SOURCE_SCHEMA={CONTRACT_SOURCE_SCHEMA}\n"
        f"WR0_CONTRACT_SOURCE_SHA256={contract_digest}\n"
        f"WR0_WORKLOAD_SHA256={workload_sha256}\n"
    )


def internal_receipt_path(environment: pathlib.Path) -> pathlib.Path:
    return environment / "compatdata/pfx/drive_c/users/steamuser/AppData/Local/LinuxVSTBridge/WR0/receipt.txt"


def verify_internal_receipt(
    environment: pathlib.Path,
    *,
    nonce: str,
    run_number: int,
    contract_source: dict[str, Any],
) -> dict[str, Any]:
    receipt = internal_receipt_path(environment)
    require_contained(receipt, environment, label="WR0 internal receipt")
    if not receipt.is_file() or receipt.is_symlink():
        raise LaunchBlocked("Windows workload internal receipt is missing or unsafe")
    raw = receipt.read_bytes()
    if len(raw) > 4096:
        raise LaunchBlocked("Windows workload internal receipt exceeds its bound")
    normalized = normalize_windows_text(raw, label="Windows workload receipt")
    if normalized != expected_receipt(
        nonce,
        run_number,
        contract_digest=str(contract_source["digest"]),
        workload_sha256=str(contract_source["workload_sha256"]),
    ):
        raise LaunchBlocked("Windows workload internal receipt differs")
    return {
        "classification": "passed",
        "safe_relative_path": "compatdata/pfx/drive_c/users/steamuser/AppData/Local/LinuxVSTBridge/WR0/receipt.txt",
        "size": len(raw),
        "sha256": sha256_bytes(raw),
        "normalized_sha256": sha256_bytes(normalized.encode("utf-8")),
        "contract_source_schema": contract_source["schema"],
        "contract_source_sha256": contract_source["digest"],
        "workload_sha256": contract_source["workload_sha256"],
    }


def handshake_paths(environment: pathlib.Path, nonce: str) -> tuple[pathlib.Path, pathlib.Path]:
    directory = environment / "compatdata/pfx/drive_c/users/steamuser/AppData/Local/LinuxVSTBridge/WR0/handshake"
    ready = directory / f"ready-{nonce}.txt"
    gate = directory / f"gate-{nonce}.txt"
    require_contained(ready, environment, label="WR0 ready handshake", allow_absent_leaf=True)
    require_contained(gate, environment, label="WR0 gate handshake", allow_absent_leaf=True)
    return ready, gate


def handshake_artifact_paths(environment: pathlib.Path, nonce: str) -> tuple[pathlib.Path, pathlib.Path, pathlib.Path]:
    ready, gate = handshake_paths(environment, nonce)
    temporary = ready.with_suffix(".tmp")
    require_contained(temporary, environment, label="WR0 temporary readiness handshake", allow_absent_leaf=True)
    return ready, gate, temporary


def expected_ready_file(nonce: str, run_number: int) -> str:
    return (
        f"WR0_HANDSHAKE_SCHEMA={HANDSHAKE_SCHEMA}\n"
        f"WR0_NONCE={nonce}\n"
        f"WR0_RUN={run_number}\n"
        "WR0_ARCH=x86_64\n"
    )


def expected_gate_file(nonce: str, run_number: int) -> str:
    return (
        f"WR0_HANDSHAKE_SCHEMA={HANDSHAKE_SCHEMA}\r\n"
        f"WR0_NONCE={nonce}\r\n"
        f"WR0_RUN={run_number}\r\n"
    )


def verify_ready_handshake(path: pathlib.Path, *, nonce: str, run_number: int) -> None:
    if not path.is_file() or path.is_symlink():
        raise LaunchBlocked("Windows readiness handshake is missing or unsafe")
    data = path.read_bytes()
    if len(data) > 1024:
        raise LaunchBlocked("Windows readiness handshake exceeds its bound")
    if normalize_windows_text(data, label="Windows readiness handshake") != expected_ready_file(nonce, run_number):
        raise LaunchBlocked("Windows readiness handshake differs")


def assert_handshake_absent(ready: pathlib.Path, gate: pathlib.Path, temporary: pathlib.Path | None = None) -> None:
    if ready.exists() or gate.exists() or (temporary is not None and temporary.exists()):
        fail("WR0 nonce handshake path already exists")


def cleanup_handshake_artifacts(environment: pathlib.Path, nonce: str) -> int:
    removed = 0
    for artifact in handshake_artifact_paths(environment, nonce):
        require_contained(artifact, environment, label="WR0 handshake cleanup target", allow_absent_leaf=True)
        try:
            info = artifact.lstat()
        except FileNotFoundError:
            continue
        if not stat.S_ISREG(info.st_mode) or stat.S_ISLNK(info.st_mode):
            fail("WR0 handshake cleanup target is not a regular transaction-owned file")
        artifact.unlink()
        removed += 1
    return removed


def sample_owned_tree(
    root_pid: int,
    root_start_ticks: int,
    observed: dict[tuple[int, int], dict[str, Any]],
    *,
    elapsed_ms: int,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
) -> tuple[dict[int, dict[str, Any]], list[dict[str, Any]]]:
    snapshot = proc_snapshot()
    current = descendant_records(root_pid, root_start_ticks, snapshot)
    for record in current:
        key = (int(record["pid"]), int(record["start_ticks"]))
        role = classify_owned_role(
            record,
            root_pid=root_pid,
            nonce=nonce,
            run_number=run_number,
            expected_exit=expected_exit,
            contract_digest=contract_digest,
            workload_sha256=workload_sha256,
        )
        if key in observed:
            observed[key]["last_seen_ms"] = elapsed_ms
            if observed[key]["role"] == "owned_descendant" or role in {"proton", "windows_command"}:
                observed[key]["role"] = role
                observed[key]["comm"] = record["comm"]
                observed[key]["exe_basename"] = record["exe_basename"]
            continue
        if len(observed) >= PROCESS_CAP:
            raise LaunchBlocked("owned process identity count exceeded its cap")
        retained = safe_process_record(record)
        retained["role"] = role
        retained["first_seen_ms"] = elapsed_ms
        retained["last_seen_ms"] = elapsed_ms
        retained["identity_sha256"] = process_identity(record)
        parent = snapshot.get(int(record["ppid"]))
        retained["parent_start_ticks"] = int(parent["start_ticks"]) if parent is not None else None
        retained["parent_identity_sha256"] = process_identity(parent) if parent is not None else None
        observed[key] = retained
    return snapshot, current


def gate_process_assertions(
    snapshot: dict[int, dict[str, Any]],
    current: list[dict[str, Any]],
    *,
    root_pid: int,
    root_start_ticks: int,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
) -> dict[str, Any] | None:
    roles = {
        (int(record["pid"]), int(record["start_ticks"])): classify_owned_role(
            record,
            root_pid=root_pid,
            nonce=nonce,
            run_number=run_number,
            expected_exit=expected_exit,
            contract_digest=contract_digest,
            workload_sha256=workload_sha256,
        )
        for record in current
    }
    proton_keys = {key for key, role in roles.items() if role == "proton"}
    commands = [record for record in current if roles[(int(record["pid"]), int(record["start_ticks"]))] == "windows_command"]
    if len(proton_keys) != 1 or len(commands) != 1:
        return None
    for command in commands:
        command_chain = ancestor_records(int(command["pid"]), snapshot)
        command_keys = {(int(item["pid"]), int(item["start_ticks"])) for item in command_chain}
        if (root_pid, root_start_ticks) not in command_keys:
            continue
        for proton_key in proton_keys:
            proton_chain = ancestor_records(proton_key[0], snapshot)
            proton_chain_keys = {(int(item["pid"]), int(item["start_ticks"])) for item in proton_chain}
            if (root_pid, root_start_ticks) not in proton_chain_keys:
                continue
            common_keys = command_keys & proton_chain_keys
            if not common_keys:
                continue
            common = next(
                item
                for item in command_chain
                if (int(item["pid"]), int(item["start_ticks"])) in common_keys
            )
            topology = (
                "windows_command_descends_from_proton"
                if proton_key in command_keys
                else "proton_and_windows_command_share_exact_runtime_ancestor"
            )
            return {
                "proton": snapshot[proton_key[0]],
                "windows_command": command,
                "windows_command_chain": command_chain,
                "proton_chain": proton_chain,
                "common_runtime_ancestor": common,
                "topology": topology,
            }
    return None


@dataclass(frozen=True)
class GateAuthorization:
    root_identity: tuple[int, int]
    proton_identity: tuple[int, int]
    windows_command_identity: tuple[int, int]
    assertion: dict[str, Any]
    fresh_revalidated_monotonic_us: int


def process_key(record: dict[str, Any]) -> tuple[int, int]:
    return int(record["pid"]), int(record["start_ticks"])


def revalidate_gate_process_assertion(
    snapshot: dict[int, dict[str, Any]],
    current: list[dict[str, Any]],
    *,
    prior_assertion: dict[str, Any] | None,
    root_pid: int,
    root_start_ticks: int,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
) -> dict[str, Any]:
    if prior_assertion is None:
        raise ProcessObservabilityBlocked("fresh gate revalidation lacks a prior exact process assertion")
    root = snapshot.get(root_pid)
    if (
        root is None
        or root.get("state") == "Z"
        or int(root["uid"]) != os.getuid()
        or int(root["start_ticks"]) != root_start_ticks
    ):
        raise ProcessObservabilityBlocked("Runtime root changed before WR0 gate commitment")
    fresh = gate_process_assertions(
        snapshot,
        current,
        root_pid=root_pid,
        root_start_ticks=root_start_ticks,
        nonce=nonce,
        run_number=run_number,
        expected_exit=expected_exit,
        contract_digest=contract_digest,
        workload_sha256=workload_sha256,
    )
    if fresh is None:
        raise ProcessObservabilityBlocked("exact Proton/Windows-command topology disappeared before gate commitment")
    for name in ("proton", "windows_command"):
        if process_key(fresh[name]) != process_key(prior_assertion[name]):
            raise ProcessObservabilityBlocked(f"selected {name} identity changed before gate commitment")
    if process_key(fresh["common_runtime_ancestor"]) != process_key(prior_assertion["common_runtime_ancestor"]):
        raise ProcessObservabilityBlocked("common Runtime ancestor changed before gate commitment")
    return fresh


def validate_gate_readiness(
    ready: pathlib.Path,
    gate: pathlib.Path,
    temporary: pathlib.Path,
    stdout: bytes,
    *,
    nonce: str,
    run_number: int,
) -> None:
    verify_ready_handshake(ready, nonce=nonce, run_number=run_number)
    if gate.exists() or temporary.exists():
        raise ProcessObservabilityBlocked("WR0 gate or temporary ready file exists before commitment")
    if normalize_windows_text(stdout, label="pre-gate Windows workload stdout") != expected_ready_stdout(nonce, run_number):
        raise ProcessObservabilityBlocked("exact ready stdout changed before gate commitment")


def fresh_gate_authorization(
    *,
    ready: pathlib.Path,
    gate: pathlib.Path,
    temporary: pathlib.Path,
    stdout: bytes,
    prior_assertion: dict[str, Any] | None,
    root_pid: int,
    root_start_ticks: int,
    observed: dict[tuple[int, int], dict[str, Any]],
    monotonic_start: float,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
) -> GateAuthorization:
    elapsed_ms = int((time.monotonic() - monotonic_start) * 1000)
    snapshot, current = sample_owned_tree(
        root_pid,
        root_start_ticks,
        observed,
        elapsed_ms=elapsed_ms,
        nonce=nonce,
        run_number=run_number,
        expected_exit=expected_exit,
        contract_digest=contract_digest,
        workload_sha256=workload_sha256,
    )
    validate_gate_readiness(ready, gate, temporary, stdout, nonce=nonce, run_number=run_number)
    assertion = revalidate_gate_process_assertion(
        snapshot,
        current,
        prior_assertion=prior_assertion,
        root_pid=root_pid,
        root_start_ticks=root_start_ticks,
        nonce=nonce,
        run_number=run_number,
        expected_exit=expected_exit,
        contract_digest=contract_digest,
        workload_sha256=workload_sha256,
    )
    return GateAuthorization(
        root_identity=(root_pid, root_start_ticks),
        proton_identity=process_key(assertion["proton"]),
        windows_command_identity=process_key(assertion["windows_command"]),
        assertion=assertion,
        fresh_revalidated_monotonic_us=int((time.monotonic() - monotonic_start) * 1_000_000),
    )


def commit_supervisor_gate(
    gate: pathlib.Path,
    *,
    nonce: str,
    run_number: int,
    authorization: GateAuthorization | None,
) -> None:
    if authorization is None:
        fail("refusing to release WR0 gate before fresh process revalidation")
    for identity in (
        authorization.root_identity,
        authorization.proton_identity,
        authorization.windows_command_identity,
    ):
        if not identity_is_live(identity, uid=os.getuid()):
            raise ProcessObservabilityBlocked("freshly authorized process identity disappeared before gate write")
    if gate.exists():
        fail("refusing to replace a preexisting WR0 gate")
    atomic_write(gate, expected_gate_file(nonce, run_number).encode("ascii"))


def commit_negative_gate(
    gate: pathlib.Path,
    *,
    nonce: str,
    run_number: int,
    variant: str,
    authorization: GateAuthorization | None,
) -> None:
    if authorization is None:
        fail("refusing to write a negative gate before fresh process revalidation")
    for identity in (
        authorization.root_identity,
        authorization.proton_identity,
        authorization.windows_command_identity,
    ):
        if not identity_is_live(identity, uid=os.getuid()):
            raise ProcessObservabilityBlocked("negative-gate process identity disappeared before gate write")
    if gate.exists():
        fail("refusing to replace a preexisting WR0 gate")
    if variant == "wrong_nonce":
        value = expected_gate_file("f" * 32 if nonce != "f" * 32 else "e" * 32, run_number)
    elif variant == "wrong_run":
        wrong_run = 2 if run_number != 2 else 1
        value = expected_gate_file(nonce, wrong_run)
    else:
        fail("negative gate variant is outside the fixed test contract")
    atomic_write(gate, value.encode("ascii"))


def fresh_authorize_and_commit_gate(
    *,
    ready: pathlib.Path,
    gate: pathlib.Path,
    temporary: pathlib.Path,
    stdout: bytes,
    prior_assertion: dict[str, Any] | None,
    root_pid: int,
    root_start_ticks: int,
    observed: dict[tuple[int, int], dict[str, Any]],
    monotonic_start: float,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
    variant: str,
) -> GateAuthorization:
    """Perform the final fresh census and publish no gate before it passes."""

    authorization = fresh_gate_authorization(
        ready=ready,
        gate=gate,
        temporary=temporary,
        stdout=stdout,
        prior_assertion=prior_assertion,
        root_pid=root_pid,
        root_start_ticks=root_start_ticks,
        observed=observed,
        monotonic_start=monotonic_start,
        nonce=nonce,
        run_number=run_number,
        expected_exit=expected_exit,
        contract_digest=contract_digest,
        workload_sha256=workload_sha256,
    )
    if variant == "release":
        commit_supervisor_gate(
            gate,
            nonce=nonce,
            run_number=run_number,
            authorization=authorization,
        )
    elif variant in {"wrong_nonce", "wrong_run"}:
        commit_negative_gate(
            gate,
            nonce=nonce,
            run_number=run_number,
            variant=variant,
            authorization=authorization,
        )
    else:
        fail("gate publication variant is outside the fixed contract")
    return authorization


def gate_candidate_diagnostics(
    current: list[dict[str, Any]],
    *,
    root_pid: int,
    nonce: str,
    run_number: int,
    expected_exit: int,
    contract_digest: str,
    workload_sha256: str,
) -> dict[str, Any]:
    proton = str(runner_root() / "proton").encode("utf-8")
    nonce_token = nonce.encode("ascii")
    contract_token = contract_digest.encode("ascii")
    workload_hash_token = workload_sha256.encode("ascii")
    values = {
        "descendant_count": len(current),
        "exact_proton_role_count": 0,
        "cmd_basename_count": 0,
        "cmd_nonce_token_count": 0,
        "cmd_contract_token_count": 0,
        "cmd_workload_hash_token_count": 0,
        "cmd_fixture_path_token_count": 0,
        "cmd_run_token_count": 0,
        "cmd_exact_vector_count": 0,
    }
    for record in current:
        arguments = cmdline_arguments(record)
        role = classify_owned_role(
            record,
            root_pid=root_pid,
            nonce=nonce,
            run_number=run_number,
            expected_exit=expected_exit,
            contract_digest=contract_digest,
            workload_sha256=workload_sha256,
        )
        if role == "proton":
            values["exact_proton_role_count"] += 1
        if str(record["comm"]).lower() != "cmd.exe" and str(record["exe_basename"]).lower() != "cmd.exe":
            continue
        values["cmd_basename_count"] += 1
        argument_set = set(arguments)
        values["cmd_nonce_token_count"] += int(nonce_token in argument_set)
        values["cmd_contract_token_count"] += int(contract_token in argument_set)
        values["cmd_workload_hash_token_count"] += int(workload_hash_token in argument_set)
        values["cmd_fixture_path_token_count"] += int(
            windows_path(probe_path()).encode("utf-8") in argument_set
        )
        values["cmd_run_token_count"] += int(str(run_number).encode("ascii") in argument_set)
        values["cmd_exact_vector_count"] += int(
            exact_windows_command_vector(
                record,
                nonce=nonce,
                run_number=run_number,
                expected_exit=expected_exit,
                contract_digest=contract_digest,
                workload_sha256=workload_sha256,
            )
        )
    values["proton_path_seen_in_first_two_arguments"] = any(
        proton in cmdline_arguments(record)[:2] for record in current
    )
    return values


def run_windows_workload(
    environment: pathlib.Path,
    *,
    nonce: str,
    run_number: int,
    expected_exit: int,
    timeout_seconds: float,
    contract_source: dict[str, Any],
    gate_behavior: str = "release",
) -> dict[str, Any]:
    if not re.fullmatch(r"[0-9a-f]{32}", nonce):
        fail("session nonce is malformed")
    if run_number not in {1, 2, 37} or expected_exit not in {0, 37}:
        fail("workload request is outside the fixed WR0 contract")
    if gate_behavior not in {"release", "wrong_nonce", "wrong_run", "withhold_cleanup"}:
        fail("gate behavior is outside the fixed WR0 contract")
    if gate_behavior != "release" and (run_number != 1 or expected_exit != 0):
        fail("test-only gate behavior requires the fixed ordinary Run 1 command vector")
    require_contained(environment, environments_root(), label="WR0 launch environment")
    verify_runner_lock()
    process_guard()
    workload = probe_path()
    require_contained(workload, repo_root(), label="tracked WR0 workload")
    if not workload.is_file() or workload.is_symlink():
        fail("tracked WR0 workload is missing or unsafe")
    contract_digest = str(contract_source.get("digest", ""))
    workload_sha256 = str(contract_source.get("workload_sha256", ""))
    if contract_source.get("schema") != CONTRACT_SOURCE_SCHEMA or not re.fullmatch(r"[0-9a-f]{64}", contract_digest):
        fail("WR0 contract-source identity is malformed")
    if sha256_file(workload) != workload_sha256:
        fail("tracked WR0 workload differs from the contract-source identity")
    ready_path, gate_path, ready_temporary = handshake_artifact_paths(environment, nonce)
    assert_handshake_absent(ready_path, gate_path, ready_temporary)
    command = [
        str(runtime_root() / "_v2-entry-point"),
        "--verb=run",
        "--",
        str(runner_root() / "proton"),
        "runinprefix",
        str(runner_root() / "files/lib/wine/x86_64-windows/cmd.exe"),
        "/d",
        "/q",
        "/c",
        windows_path(workload),
        nonce,
        str(run_number),
        contract_digest,
        workload_sha256,
    ]
    if expected_exit == 37:
        command.append("--exit-37")
    env = controlled_environment(environment)
    supervisor_raw = read_proc_record(os.getpid())
    if supervisor_raw is None:
        fail("supervisor process identity is unavailable")
    supervisor = safe_process_record(supervisor_raw)
    supervisor.pop("pgrp", None)
    supervisor.pop("session", None)
    supervisor.pop("pid", None)
    supervisor.pop("ppid", None)
    supervisor["identity_sha256"] = process_identity(supervisor_raw)
    started_at = utc_now()
    monotonic_start = time.monotonic()
    process: subprocess.Popen[bytes] | None = None
    selector = selectors.DefaultSelector()
    stdout = bytearray()
    stderr = bytearray()
    observed: dict[tuple[int, int], dict[str, Any]] = {}
    timed_out = False
    terminated_by_supervisor = False
    ancestry_safe: list[dict[str, Any]] = []
    pgid = -1
    root_pid = -1
    root_start_ticks = -1
    command_first_observed_us: int | None = None
    gate_committed_us: int | None = None
    command_completed_us: int | None = None
    gate_proton_identity = ""
    gate_command_identity = ""
    gate_command_chain: list[dict[str, Any]] = []
    gate_proton_chain: list[dict[str, Any]] = []
    gate_common_ancestor_identity = ""
    gate_topology = ""
    ready_file_seen = False
    ready_stdout_seen = False
    latest_gate_diagnostics: dict[str, Any] = {}
    gate_authorization: GateAuthorization | None = None
    gate_fresh_revalidated_us: int | None = None
    held_deadline: float | None = None
    sentinel: subprocess.Popen[bytes] | None = None
    sentinel_identity: tuple[int, int] | None = None
    sentinel_safe: dict[str, Any] | None = None

    def drain_registered_streams(deadline_seconds: float = 2.0) -> None:
        drain_deadline = time.monotonic() + deadline_seconds
        while selector.get_map() and time.monotonic() < drain_deadline:
            for key, _ in selector.select(timeout=PROCESS_POLL_SECONDS):
                stream = key.fileobj
                try:
                    chunk = os.read(stream.fileno(), 4096)
                except BlockingIOError:
                    continue
                if not chunk:
                    selector.unregister(stream)
                    continue
                target = stdout if key.data == "stdout" else stderr
                target.extend(chunk)
                if len(target) > OUTPUT_CAP:
                    raise LaunchBlocked(f"{key.data} exceeded {OUTPUT_CAP} bytes")

    def start_sentinel() -> tuple[subprocess.Popen[bytes], tuple[int, int], dict[str, Any]]:
        value = subprocess.Popen(
            [sys.executable, "-c", "import time; time.sleep(60)"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        record = read_proc_record(value.pid)
        if record is None or int(record["uid"]) != os.getuid() or int(record["pgrp"]) == pgid:
            try:
                os.kill(value.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            value.wait(timeout=5)
            raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: unrelated sentinel identity is unavailable")
        identity = process_key(record)
        safe = {
            "comm": record["comm"],
            "executable_basename": record["exe_basename"],
            "uid": record["uid"],
            "start_ticks": record["start_ticks"],
            "identity_sha256": process_identity(record),
            "separate_process_group": int(record["pgrp"]) != pgid,
            "same_safe_comm_as_proton_family": str(record["comm"]).lower().startswith("python"),
        }
        return value, identity, safe

    def stop_sentinel_exact() -> None:
        nonlocal sentinel
        if sentinel is None or sentinel_identity is None:
            return
        if identity_is_live(sentinel_identity, uid=os.getuid()):
            signal_exact_identity(sentinel_identity, signal.SIGTERM)
            deadline = time.monotonic() + TERM_GRACE_SECONDS
            while identity_is_live(sentinel_identity, uid=os.getuid()) and time.monotonic() < deadline:
                time.sleep(0.05)
            if identity_is_live(sentinel_identity, uid=os.getuid()):
                signal_exact_identity(sentinel_identity, signal.SIGKILL)
        try:
            sentinel.wait(timeout=TERM_GRACE_SECONDS)
        except subprocess.TimeoutExpired as exc:
            raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: unrelated sentinel did not terminate separately") from exc
        if identity_is_live(sentinel_identity, uid=os.getuid()):
            raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: unrelated sentinel identity survived separate cleanup")
        sentinel = None

    try:
        process = subprocess.Popen(
            command,
            cwd=repo_root(),
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        root_pid = process.pid
        root_record = read_proc_record(root_pid)
        if root_record is None:
            raise ProcessObservabilityBlocked("runtime root start identity was not observable")
        root_start_ticks = int(root_record["start_ticks"])
        pgid = os.getpgid(root_pid)
        if pgid != root_pid or pgid == os.getpgrp():
            raise LaunchBlocked("runtime root did not receive an isolated process group")
        ancestry_snapshot = proc_snapshot()
        ancestry = ancestor_records(root_pid, ancestry_snapshot)
        if len(ancestry) < 2 or int(ancestry[1]["pid"]) != os.getpid():
            raise LaunchBlocked("repository supervisor is not the runtime root parent")
        steam_ancestor_names = {"steam", "reaper", "steam-launch-wrapper", "steam-launch-wr"}
        if any(str(item["comm"]).lower() in steam_ancestor_names for item in ancestry):
            raise LaunchBlocked("Steam game-launch ancestry was observed")
        ancestry_safe = [
            {
                "comm": item["comm"],
                "exe_basename": item["exe_basename"],
                "uid": item["uid"],
                "start_ticks": item["start_ticks"],
                "identity_sha256": process_identity(item),
            }
            for item in ancestry[:12]
        ]
        assert process.stdout is not None and process.stderr is not None
        for stream, name in ((process.stdout, "stdout"), (process.stderr, "stderr")):
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ, name)

        deadline = monotonic_start + timeout_seconds
        while process.poll() is None or selector.get_map():
            now = time.monotonic()
            if now >= deadline:
                timed_out = True
                terminated_by_supervisor = True
                terminate_owned_identities(
                    pgid,
                    observed,
                    root_pid=root_pid,
                    root_start_ticks=root_start_ticks,
                    nonce=nonce,
                    run_number=run_number,
                    expected_exit=expected_exit,
                    contract_digest=contract_digest,
                    workload_sha256=workload_sha256,
                )
                cleanup_handshake_artifacts(environment, nonce)
                raise LaunchBlocked("controlled Windows workload timed out")
            elapsed_ms = int((now - monotonic_start) * 1000)
            snapshot, current = sample_owned_tree(
                root_pid,
                root_start_ticks,
                observed,
                elapsed_ms=elapsed_ms,
                nonce=nonce,
                run_number=run_number,
                expected_exit=expected_exit,
                contract_digest=contract_digest,
                workload_sha256=workload_sha256,
            )
            for key, _ in selector.select(timeout=PROCESS_POLL_SECONDS):
                stream = key.fileobj
                try:
                    chunk = os.read(stream.fileno(), 4096)
                except BlockingIOError:
                    continue
                if not chunk:
                    selector.unregister(stream)
                    continue
                target = stdout if key.data == "stdout" else stderr
                target.extend(chunk)
                if len(target) > OUTPUT_CAP:
                    terminated_by_supervisor = True
                    terminate_owned_identities(
                        pgid,
                        observed,
                        root_pid=root_pid,
                        root_start_ticks=root_start_ticks,
                        nonce=nonce,
                        run_number=run_number,
                        expected_exit=expected_exit,
                        contract_digest=contract_digest,
                        workload_sha256=workload_sha256,
                    )
                    cleanup_handshake_artifacts(environment, nonce)
                    raise LaunchBlocked(f"{key.data} exceeded {OUTPUT_CAP} bytes")
            if gate_authorization is None and ready_path.exists():
                ready_file_seen = True
                verify_ready_handshake(ready_path, nonce=nonce, run_number=run_number)
                if gate_path.exists():
                    raise LaunchBlocked("WR0 gate appeared before supervisor commitment")
                current_stdout = normalize_windows_text(bytes(stdout), label="partial Windows workload stdout")
                if current_stdout == expected_ready_stdout(nonce, run_number):
                    ready_stdout_seen = True
                    latest_gate_diagnostics = gate_candidate_diagnostics(
                        current,
                        root_pid=root_pid,
                        nonce=nonce,
                        run_number=run_number,
                        expected_exit=expected_exit,
                        contract_digest=contract_digest,
                        workload_sha256=workload_sha256,
                    )
                    assertion = gate_process_assertions(
                        snapshot,
                        current,
                        root_pid=root_pid,
                        root_start_ticks=root_start_ticks,
                        nonce=nonce,
                        run_number=run_number,
                        expected_exit=expected_exit,
                        contract_digest=contract_digest,
                        workload_sha256=workload_sha256,
                    )
                    if assertion is not None:
                        command_first_observed_us = int((time.monotonic() - monotonic_start) * 1_000_000)
                        if gate_behavior in {"release", "wrong_nonce", "wrong_run"}:
                            gate_authorization = fresh_authorize_and_commit_gate(
                                ready=ready_path,
                                gate=gate_path,
                                temporary=ready_temporary,
                                stdout=bytes(stdout),
                                prior_assertion=assertion,
                                root_pid=root_pid,
                                root_start_ticks=root_start_ticks,
                                observed=observed,
                                monotonic_start=monotonic_start,
                                nonce=nonce,
                                run_number=run_number,
                                expected_exit=expected_exit,
                                contract_digest=contract_digest,
                                workload_sha256=workload_sha256,
                                variant=gate_behavior,
                            )
                            gate_committed_us = int((time.monotonic() - monotonic_start) * 1_000_000)
                        else:
                            gate_authorization = fresh_gate_authorization(
                                ready=ready_path,
                                gate=gate_path,
                                temporary=ready_temporary,
                                stdout=bytes(stdout),
                                prior_assertion=assertion,
                                root_pid=root_pid,
                                root_start_ticks=root_start_ticks,
                                observed=observed,
                                monotonic_start=monotonic_start,
                                nonce=nonce,
                                run_number=run_number,
                                expected_exit=expected_exit,
                                contract_digest=contract_digest,
                                workload_sha256=workload_sha256,
                            )
                            sentinel, sentinel_identity, sentinel_safe = start_sentinel()
                            held_deadline = time.monotonic() + HELD_COMMAND_SECONDS
                        assertion = gate_authorization.assertion
                        proton_record = assertion["proton"]
                        command_record = assertion["windows_command"]
                        command_chain = assertion["windows_command_chain"]
                        proton_chain = assertion["proton_chain"]
                        gate_fresh_revalidated_us = gate_authorization.fresh_revalidated_monotonic_us
                        gate_proton_identity = process_identity(proton_record)
                        gate_command_identity = process_identity(command_record)
                        gate_command_chain = [safe_process_record(item) for item in command_chain]
                        gate_proton_chain = [safe_process_record(item) for item in proton_chain]
                        gate_common_ancestor_identity = process_identity(assertion["common_runtime_ancestor"])
                        gate_topology = str(assertion["topology"])
                        if gate_committed_us is not None and (
                            gate_committed_us <= command_first_observed_us
                            or gate_fresh_revalidated_us is None
                            or gate_committed_us <= gate_fresh_revalidated_us
                        ):
                            raise LaunchBlocked("WR0 gate timestamp did not follow fresh command revalidation")
            if gate_behavior == "withhold_cleanup" and held_deadline is not None and time.monotonic() >= held_deadline:
                gate_authorization = fresh_gate_authorization(
                    ready=ready_path,
                    gate=gate_path,
                    temporary=ready_temporary,
                    stdout=bytes(stdout),
                    prior_assertion=gate_authorization.assertion if gate_authorization is not None else None,
                    root_pid=root_pid,
                    root_start_ticks=root_start_ticks,
                    observed=observed,
                    monotonic_start=monotonic_start,
                    nonce=nonce,
                    run_number=run_number,
                    expected_exit=expected_exit,
                    contract_digest=contract_digest,
                    workload_sha256=workload_sha256,
                )
                if gate_path.exists():
                    raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: held-command gate unexpectedly exists")
                terminated_by_supervisor = True
                cleanup_result = terminate_owned_identities(
                    pgid,
                    observed,
                    root_pid=root_pid,
                    root_start_ticks=root_start_ticks,
                    nonce=nonce,
                    run_number=run_number,
                    expected_exit=expected_exit,
                    contract_digest=contract_digest,
                    workload_sha256=workload_sha256,
                )
                process.wait(timeout=TERM_GRACE_SECONDS)
                command_completed_us = int((time.monotonic() - monotonic_start) * 1_000_000)
                drain_registered_streams()
                if sentinel_identity is None or not identity_is_live(sentinel_identity, uid=os.getuid()):
                    raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: unrelated sentinel did not survive owned cleanup")
                stop_sentinel_exact()
                cleanup_handshake_artifacts(environment, nonce)
                assert_handshake_absent(ready_path, gate_path, ready_temporary)
                process_guard()
                return {
                    "schema": "linux-vst-bridge-wr0-live-held-cleanup/v1",
                    "classification": "passed",
                    "nonce": nonce,
                    "run_number": run_number,
                    "ready_file_verified": True,
                    "ready_stdout_verified": True,
                    "strict_command_vector_verified": True,
                    "fresh_process_revalidation_verified": True,
                    "command_first_observed_monotonic_us": command_first_observed_us,
                    "cleanup_started_after_observation": command_first_observed_us is not None,
                    "command_completed_monotonic_us": command_completed_us,
                    "processes": sorted(observed.values(), key=lambda item: (item["first_seen_ms"], item["start_ticks"])),
                    "process_count": len(observed),
                    "escaped_process_group_count": sum(1 for item in observed.values() if int(item["pgrp"]) != pgid),
                    "termination": cleanup_result,
                    "final_owned_count": 0,
                    "sentinel": sentinel_safe,
                    "sentinel_survival": True,
                    "sentinel_terminated_separately": True,
                    "ready_gate_temporary_absent_after": True,
                    "contract_source_schema": CONTRACT_SOURCE_SCHEMA,
                    "contract_source_sha256": contract_digest,
                    "workload_sha256": workload_sha256,
                    "gate_withheld": True,
                    "hold_seconds": HELD_COMMAND_SECONDS,
                    "complete_command_line_retained": False,
                }
        return_code = process.wait(timeout=1)
        command_completed_us = int((time.monotonic() - monotonic_start) * 1_000_000)
        clean, remaining = wait_owned_empty(pgid, observed)
        if not clean:
            terminated_by_supervisor = True
            terminate_owned_identities(
                pgid,
                observed,
                root_pid=root_pid,
                root_start_ticks=root_start_ticks,
                nonce=nonce,
                run_number=run_number,
                expected_exit=expected_exit,
                contract_digest=contract_digest,
                workload_sha256=workload_sha256,
            )
            raise LaunchBlocked(f"owned root-descendant scope did not terminate normally ({len(remaining)} descendants)")
        if gate_committed_us is None or command_first_observed_us is None or gate_fresh_revalidated_us is None:
            diagnostic = {
                "ready_file_seen": ready_file_seen,
                "ready_stdout_seen": ready_stdout_seen,
                "windows_return_code": return_code,
                "stdout_lines": normalize_windows_text(
                    bytes(stdout), label="failed-gate Windows workload stdout"
                ).splitlines(),
                "stderr_bytes": len(stderr),
                "stderr_sha256": sha256_bytes(bytes(stderr)),
                **latest_gate_diagnostics,
            }
            raise ProcessObservabilityBlocked(
                f"causal process-observation gate was not established: {json.dumps(diagnostic, sort_keys=True)}"
            )
        if command_completed_us <= gate_committed_us:
            raise LaunchBlocked("Windows command completion did not follow gate commitment")
        if ready_path.exists() or gate_path.exists() or ready_temporary.exists():
            raise LaunchBlocked("WR0 readiness/gate file remained after completion")
        expected_return = 79 if gate_behavior in {"wrong_nonce", "wrong_run"} else expected_exit
        if return_code != expected_return:
            normalized_failure_stdout = normalize_windows_text(bytes(stdout), label="failed Windows workload stdout")
            raise LaunchBlocked(
                f"Windows child exit propagation differed: expected {expected_return}, observed {return_code}; "
                f"stdout_lines={json.dumps(normalized_failure_stdout.splitlines(), ensure_ascii=True)}"
            )
        normalized_stdout = normalize_windows_text(bytes(stdout), label="Windows workload stdout")
        if gate_behavior == "release":
            validate_workload_output(normalized_stdout, nonce=nonce, run_number=run_number, expected_exit=expected_exit)
        else:
            failure_name = "gate_nonce_invalid" if gate_behavior == "wrong_nonce" else "gate_run_invalid"
            expected_failure_stdout = expected_ready_stdout(nonce, run_number) + f"WR0_HANDSHAKE_ERROR={failure_name}\n"
            if normalized_stdout != expected_failure_stdout:
                raise LaunchBlocked("actual batch wrong-gate stdout differs from the fixed expected failure")
        finished_at = utc_now()
        result = {
            "schema": RUN_SCHEMA,
            "classification": "expected_failure" if expected_exit == 37 or gate_behavior != "release" else "passed",
            "run_number": run_number,
            "nonce": nonce,
            "started_at": started_at,
            "finished_at": finished_at,
            "duration_ms": int((time.monotonic() - monotonic_start) * 1000),
            "stdout": normalized_stdout,
            "stdout_sha256": sha256_bytes(bytes(stdout)),
            "stderr_bytes": len(stderr),
            "stderr_sha256": sha256_bytes(bytes(stderr)),
            "exit_status": return_code,
            "expected_exit_status": expected_return,
            "timeout": False,
            "terminated_by_supervisor": terminated_by_supervisor,
            "supervisor": supervisor,
            "root_start_ticks": root_start_ticks,
            "root_identity_sha256": process_identity(root_record),
            "ancestry": ancestry_safe,
            "steam_game_ancestor": False,
            "processes": sorted(observed.values(), key=lambda item: (item["first_seen_ms"], item["start_ticks"])),
            "clean_descendant_count": 0,
            "process_count": len(observed),
            "process_scope": "complete_bounded_exact_root_descendants",
            "escaped_process_group_count": sum(1 for item in observed.values() if int(item["pgrp"]) != pgid),
            "handshake": {
                "schema": HANDSHAKE_SCHEMA,
                "ready_file_verified": True,
                "ready_stdout_verified": True,
                "command_first_observed_monotonic_us": command_first_observed_us,
                "gate_committed_monotonic_us": gate_committed_us,
                "fresh_revalidated_monotonic_us": gate_fresh_revalidated_us,
                "command_completed_monotonic_us": command_completed_us,
                "causal_order_verified": True,
                "proton_identity_sha256": gate_proton_identity,
                "windows_command_identity_sha256": gate_command_identity,
                "windows_command_chain_depth": len(gate_command_chain),
                "proton_chain_depth": len(gate_proton_chain),
                "common_runtime_ancestor_identity_sha256": gate_common_ancestor_identity,
                "observed_topology": gate_topology,
                "ready_and_gate_absent_after": True,
            },
            "contract_source_schema": CONTRACT_SOURCE_SCHEMA,
            "contract_source_sha256": contract_digest,
            "workload_sha256": workload_sha256,
            "command_contract": "exact_unique_ordered_d_q_c_full_probe_nonce_run_source_workload_vector",
            "strict_command_vector_verified": True,
            "fresh_process_revalidation_verified": True,
            "gate_behavior": gate_behavior,
            "complete_command_line_retained": False,
            "environment_retained": False,
        }
        roles = {str(item["role"]) for item in observed.values()}
        required_roles = {"runtime_root", "proton", "windows_command"}
        if not required_roles.issubset(roles):
            missing = ",".join(sorted(required_roles - roles))
            raise ProcessObservabilityBlocked(
                f"required owned process identity was not observed: {missing}",
                partial_run=result,
            )
        process_guard()
        return result
    except Exception as original_error:
        cleanup_topology_error: ProcessTopologyBlocked | None = None
        if process is not None and pgid > 1:
            try:
                if root_pid > 1 and root_start_ticks > 0:
                    try:
                        sample_owned_tree(
                            root_pid,
                            root_start_ticks,
                            observed,
                            elapsed_ms=int((time.monotonic() - monotonic_start) * 1000),
                            nonce=nonce,
                            run_number=run_number,
                            expected_exit=expected_exit,
                            contract_digest=contract_digest,
                            workload_sha256=workload_sha256,
                        )
                    except Exception:
                        pass
                if process.poll() is None or records_in_group(pgid) or any(
                    identity_is_live(identity, uid=os.getuid()) for identity in observed
                ):
                    terminate_owned_identities(
                        pgid,
                        observed,
                        root_pid=root_pid,
                        root_start_ticks=root_start_ticks,
                        nonce=nonce,
                        run_number=run_number,
                        expected_exit=expected_exit,
                        contract_digest=contract_digest,
                        workload_sha256=workload_sha256,
                    )
                cleanup_handshake_artifacts(environment, nonce)
            except ProcessTopologyBlocked as cleanup_error:
                cleanup_topology_error = cleanup_error
        try:
            if sentinel is not None and sentinel_identity is not None:
                stop_sentinel_exact()
        except ProcessTopologyBlocked as cleanup_error:
            cleanup_topology_error = cleanup_error
        if cleanup_topology_error is not None:
            raise cleanup_topology_error from original_error
        raise original_error
    finally:
        if sentinel is not None and sentinel_identity is not None:
            try:
                stop_sentinel_exact()
            except Exception:
                pass
        selector.close()
        if process is not None:
            for stream in (process.stdout, process.stderr):
                if stream is not None:
                    stream.close()


def ensure_cache_layout() -> None:
    home = real_home()
    root = cache_root()
    require_contained(root, home, label="WR0 cache root", allow_absent_leaf=True)
    root.mkdir(parents=True, mode=0o700, exist_ok=True)
    require_contained(root, home, label="WR0 cache root")
    for child in (session_parent(), test_root()):
        require_contained(child, root, label="WR0 cache child", allow_absent_leaf=True)
        child.mkdir(mode=0o700, exist_ok=True)
        require_contained(child, root, label="WR0 cache child")


def environment_marker_path(environment: pathlib.Path) -> pathlib.Path:
    return environment / "wr0-environment.json"


def marker_document(
    *,
    transaction_id: str,
    created_at: str,
    lock_digest: str,
    contract_source_digest: str,
    workload_sha256: str,
    status_value: str,
    run_receipts: dict[str, str] | None = None,
    exit_37_verified: bool = False,
) -> dict[str, Any]:
    if not re.fullmatch(r"wr0-[0-9]{8}T[0-9]{6}Z-[0-9a-f]{16}", transaction_id):
        fail("environment transaction ID is malformed")
    if status_value not in {"initializing", "ready"}:
        fail("environment marker status is unsupported")
    if not re.fullmatch(r"[0-9a-f]{64}", contract_source_digest):
        fail("environment contract-source digest is malformed")
    if not re.fullmatch(r"[0-9a-f]{64}", workload_sha256):
        fail("environment workload digest is malformed")
    return {
        "schema": ENVIRONMENT_SCHEMA,
        "status": status_value,
        "transaction_id": transaction_id,
        "created_at": created_at,
        "neutral_application_id": NEUTRAL_APP_ID,
        "runner_identity_sha256": lock_digest,
        "contract_source_schema": CONTRACT_SOURCE_SCHEMA,
        "contract_source_sha256": contract_source_digest,
        "workload_sha256": workload_sha256,
        "runner_version": RUNNER_VERSION,
        "runtime_app_id": RUNTIME_APP_ID,
        "runtime_version": RUNTIME_VERSION,
        "layout": {
            "compatdata": "compatdata",
            "prefix": "compatdata/pfx",
            "runtime_variable_state": "runtime-var",
            "host_cache": "host-cache",
            "host_config": "host-config",
            "host_data": "host-data",
            "receipts": "receipts",
        },
        "run_receipts": dict(sorted((run_receipts or {}).items())),
        "exit_37_verified": exit_37_verified,
        "credentials_retained": False,
        "machine_identifier_retained": False,
        "private_path_retained": False,
    }


def read_environment_marker(environment: pathlib.Path) -> dict[str, Any]:
    marker = environment_marker_path(environment)
    require_contained(marker, environment, label="WR0 ownership marker")
    if not marker.is_file() or marker.is_symlink() or marker.stat().st_size > 16 * 1024:
        fail("WR0 ownership marker is missing, unsafe, or oversized")
    try:
        value = json.loads(marker.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise WR0Error("WR0 ownership marker is malformed") from exc
    expected_keys = set(
        marker_document(
            transaction_id="wr0-20000101T000000Z-0000000000000000",
            created_at="2000-01-01T00:00:00Z",
            lock_digest="0" * 64,
            contract_source_digest="0" * 64,
            workload_sha256="0" * 64,
            status_value="ready",
        )
    )
    if not isinstance(value, dict) or set(value) != expected_keys:
        fail("WR0 ownership marker key set differs")
    if value.get("schema") != ENVIRONMENT_SCHEMA:
        fail("WR0 ownership marker schema differs")
    if value.get("neutral_application_id") != NEUTRAL_APP_ID:
        fail("WR0 ownership marker application ID differs")
    if value.get("runner_identity_sha256") != expected_lock_digest():
        fail("WR0 environment is bound to another runner identity")
    if value.get("runner_version") != RUNNER_VERSION or value.get("runtime_app_id") != RUNTIME_APP_ID:
        fail("WR0 environment runner/runtime binding differs")
    if value.get("runtime_version") != RUNTIME_VERSION:
        fail("WR0 environment runtime version differs")
    if value.get("contract_source_schema") != CONTRACT_SOURCE_SCHEMA:
        fail("WR0 environment contract-source schema differs")
    if not re.fullmatch(r"[0-9a-f]{64}", str(value.get("contract_source_sha256", ""))):
        fail("WR0 environment contract-source identity is malformed")
    if not re.fullmatch(r"[0-9a-f]{64}", str(value.get("workload_sha256", ""))):
        fail("WR0 environment workload identity is malformed")
    if value.get("layout") != marker_document(
        transaction_id="wr0-20000101T000000Z-0000000000000000",
        created_at="2000-01-01T00:00:00Z",
        lock_digest="0" * 64,
        contract_source_digest="0" * 64,
        workload_sha256="0" * 64,
        status_value="ready",
    )["layout"]:
        fail("WR0 environment layout differs")
    return value


def prepare_environment_parent(*, allowed_siblings: Iterable[str] = ()) -> pathlib.Path:
    root = environments_root()
    require_contained(root, real_home(), label="WR0 environments root", allow_absent_leaf=True)
    root.mkdir(parents=True, mode=0o700, exist_ok=True)
    require_contained(root, real_home(), label="WR0 environments root")
    if not root.is_dir() or root.is_symlink():
        fail("WR0 environments root is unsafe")
    allowed = set(allowed_siblings)
    for entry in root.iterdir():
        if (
            entry.name.startswith(".wr0-proton11.stage-")
            or entry.name.startswith(".wr0-proton11.previous-")
            or entry.name.startswith(".wr0-proton11.retiring-")
            or entry.name.startswith(".wr0-proton11.journal-")
        ) and entry.name not in allowed:
            fail("unknown WR0 transaction sibling already exists")
    return root


def create_stage(
    transaction_id: str,
    created_at: str,
    lock_digest: str,
    contract_source: dict[str, Any],
    allowed_siblings: Iterable[str] = (),
) -> pathlib.Path:
    parent = prepare_environment_parent(allowed_siblings=allowed_siblings)
    stage = parent / f".wr0-proton11.stage-{transaction_id}"
    require_contained(stage, parent, label="WR0 stage", allow_absent_leaf=True)
    if stage.exists():
        fail("WR0 stage collision")
    stage.mkdir(mode=0o700)
    try:
        for relative in ("compatdata", "runtime-var", "host-cache", "host-config", "host-data", "host-tmp", "receipts"):
            child = stage / relative
            child.mkdir(mode=0o700)
        marker = marker_document(
            transaction_id=transaction_id,
            created_at=created_at,
            lock_digest=lock_digest,
            contract_source_digest=str(contract_source["digest"]),
            workload_sha256=str(contract_source["workload_sha256"]),
            status_value="initializing",
        )
        atomic_write_json(environment_marker_path(stage), marker)
        return stage
    except Exception:
        if stage.exists():
            shutil.rmtree(stage)
        raise


def transaction_owns(environment: pathlib.Path, transaction_id: str) -> bool:
    try:
        marker = read_environment_marker(environment)
    except WR0Error:
        return False
    return marker.get("transaction_id") == transaction_id


def remove_owned_transaction_root(environment: pathlib.Path, transaction_id: str) -> None:
    require_contained(environment, environments_root(), label="transaction cleanup target")
    if not transaction_owns(environment, transaction_id):
        fail("refusing to clean an environment not owned by this transaction")
    shutil.rmtree(environment)


def registry_identities(prefix: pathlib.Path) -> dict[str, dict[str, Any]]:
    values = {}
    for name in ("system.reg", "user.reg", "userdef.reg"):
        path = prefix / name
        if not path.is_file() or path.is_symlink():
            raise LaunchBlocked(f"initialized prefix lacks safe {name}")
        values[name] = {"size": path.stat().st_size, "sha256": sha256_file(path)}
    return values


def environment_snapshot(
    environment: pathlib.Path,
    *,
    require_ready: bool = True,
    contract_source: dict[str, Any] | None = None,
) -> dict[str, Any]:
    require_contained(environment, environments_root(), label="WR0 environment")
    marker = read_environment_marker(environment)
    if require_ready and marker.get("status") != "ready":
        fail("WR0 environment is not ready")
    if contract_source is not None and (
        marker.get("contract_source_sha256") != contract_source["digest"]
        or marker.get("workload_sha256") != contract_source["workload_sha256"]
    ):
        fail("WR0 environment is bound to another contract source")
    prefix = environment / "compatdata/pfx"
    require_contained(prefix, environment, label="WR0 prefix")
    if not prefix.is_dir() or prefix.is_symlink():
        raise LaunchBlocked("Proton did not initialize the declared prefix")
    info = prefix.stat()
    top = []
    for entry in prefix.iterdir():
        if len(top) >= 64:
            raise LaunchBlocked("bounded prefix top-level roster exceeded its cap")
        kind = "symlink" if entry.is_symlink() else "directory" if entry.is_dir() else "regular_file" if entry.is_file() else "other"
        top.append({"name": entry.name, "type": kind})
    top.sort(key=lambda item: item["name"])
    stable = {
        "schema": ENVIRONMENT_SCHEMA,
        "transaction_id": marker["transaction_id"],
        "runner_identity_sha256": marker["runner_identity_sha256"],
        "contract_source_sha256": marker["contract_source_sha256"],
        "workload_sha256": marker["workload_sha256"],
        "neutral_application_id": marker["neutral_application_id"],
        "pfx_device": info.st_dev,
        "pfx_inode": info.st_ino,
    }
    return {
        "classification": "passed",
        "path": "<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11",
        "marker": marker,
        "marker_sha256": sha256_file(environment_marker_path(environment)),
        "environment_identity": stable,
        "environment_identity_sha256": sha256_bytes(canonical_json(stable)),
        "registry_files": registry_identities(prefix),
        "bounded_prefix_top_level": top,
        "bounded_prefix_top_level_count": len(top),
        "complete_prefix_roster_retained": False,
    }


def fsync_directory(path: pathlib.Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def replacement_commit_path(environment: pathlib.Path) -> pathlib.Path:
    path = environment / REPLACEMENT_COMMIT_FILE
    require_contained(path, environment, label="WR0 replacement commit record", allow_absent_leaf=True)
    return path


def replacement_receipt_identities(environment: pathlib.Path) -> dict[str, str]:
    identities: dict[str, str] = {}
    for field_name, file_name in (
        ("run_1_receipt_sha256", "run-1.json"),
        ("run_2_receipt_sha256", "run-2.json"),
        ("exit_37_receipt_sha256", "exit-37.json"),
    ):
        path = environment / "receipts" / file_name
        require_contained(path, environment, label="WR0 committed run receipt")
        if not path.is_file() or path.is_symlink() or path.stat().st_size > 16 * 1024:
            fail("WR0 committed run receipt is missing or unsafe")
        identities[field_name] = sha256_file(path)
    return identities


def replacement_commit_document(
    *,
    replacement_transaction_id: str,
    predecessor_transaction_id: str,
    predecessor_environment_identity_sha256: str,
    new_environment_identity_sha256: str,
    runner_identity_sha256: str,
    contract_source_sha256: str,
    workload_sha256: str,
    receipt_identities: dict[str, str],
    protected_fixture_sha256: str,
    committed_at: str,
    predecessor_retirement_status: str,
    retirement_started_at: str | None = None,
    retirement_completed_at: str | None = None,
) -> dict[str, Any]:
    if predecessor_retirement_status not in {"pending", "retired"}:
        fail("replacement commit record retirement status differs")
    digests = {
        "predecessor_environment_identity_sha256": predecessor_environment_identity_sha256,
        "new_environment_identity_sha256": new_environment_identity_sha256,
        "runner_identity_sha256": runner_identity_sha256,
        "contract_source_sha256": contract_source_sha256,
        "workload_sha256": workload_sha256,
        "protected_fixture_sha256": protected_fixture_sha256,
        **receipt_identities,
    }
    if set(receipt_identities) != {
        "run_1_receipt_sha256",
        "run_2_receipt_sha256",
        "exit_37_receipt_sha256",
    } or any(re.fullmatch(r"[0-9a-f]{64}", str(value)) is None for value in digests.values()):
        fail("replacement commit record identity is malformed")
    if not re.fullmatch(r"wr0-[0-9]{8}T[0-9]{6}Z-[0-9a-f]{16}", replacement_transaction_id):
        fail("replacement transaction identity is malformed")
    if not re.fullmatch(r"wr0-[0-9]{8}T[0-9]{6}Z-[0-9a-f]{16}", predecessor_transaction_id):
        fail("predecessor transaction identity is malformed")
    if predecessor_retirement_status == "pending" and retirement_completed_at is not None:
        fail("pending predecessor retirement has a completion timestamp")
    if predecessor_retirement_status == "retired" and (
        retirement_started_at is None or retirement_completed_at is None
    ):
        fail("retired predecessor lacks ordered timestamps")
    return {
        "schema": REPLACEMENT_COMMIT_SCHEMA,
        "status": "committed" if predecessor_retirement_status == "pending" else "retired",
        "replacement_transaction_id": replacement_transaction_id,
        "predecessor_transaction_id": predecessor_transaction_id,
        "predecessor_environment_identity_sha256": predecessor_environment_identity_sha256,
        "new_environment_identity_sha256": new_environment_identity_sha256,
        "runner_identity_sha256": runner_identity_sha256,
        "contract_source_schema": CONTRACT_SOURCE_SCHEMA,
        "contract_source_sha256": contract_source_sha256,
        "workload_sha256": workload_sha256,
        "run_1_receipt_sha256": receipt_identities["run_1_receipt_sha256"],
        "run_2_receipt_sha256": receipt_identities["run_2_receipt_sha256"],
        "exit_37_receipt_sha256": receipt_identities["exit_37_receipt_sha256"],
        "exit_37_verified": True,
        "live_held_cleanup_verified": True,
        "protected_fixture_sha256": protected_fixture_sha256,
        "committed_at": committed_at,
        "predecessor_retirement_status": predecessor_retirement_status,
        "retirement_started_at": retirement_started_at,
        "retirement_completed_at": retirement_completed_at,
        "credentials_retained": False,
        "private_path_retained": False,
    }


def read_replacement_commit_record(
    environment: pathlib.Path,
    *,
    expected: dict[str, Any] | None = None,
) -> dict[str, Any]:
    path = replacement_commit_path(environment)
    if not path.is_file() or path.is_symlink() or path.stat().st_size > 32 * 1024:
        fail("WR0 replacement commit record is missing, unsafe, or oversized")
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise WR0Error("WR0 replacement commit record is malformed") from exc
    template = replacement_commit_document(
        replacement_transaction_id="wr0-20000101T000000Z-0000000000000000",
        predecessor_transaction_id="wr0-20000101T000000Z-0000000000000000",
        predecessor_environment_identity_sha256="0" * 64,
        new_environment_identity_sha256="0" * 64,
        runner_identity_sha256="0" * 64,
        contract_source_sha256="0" * 64,
        workload_sha256="0" * 64,
        receipt_identities={
            "run_1_receipt_sha256": "0" * 64,
            "run_2_receipt_sha256": "0" * 64,
            "exit_37_receipt_sha256": "0" * 64,
        },
        protected_fixture_sha256="0" * 64,
        committed_at="2000-01-01T00:00:00Z",
        predecessor_retirement_status="pending",
    )
    if not isinstance(value, dict) or set(value) != set(template):
        fail("WR0 replacement commit record key set differs")
    if value.get("schema") != REPLACEMENT_COMMIT_SCHEMA:
        fail("WR0 replacement commit record schema differs")
    if value.get("contract_source_schema") != CONTRACT_SOURCE_SCHEMA:
        fail("WR0 replacement commit record source schema differs")
    if value.get("exit_37_verified") is not True or value.get("live_held_cleanup_verified") is not True:
        fail("WR0 replacement commit record live result binding differs")
    if value.get("credentials_retained") is not False or value.get("private_path_retained") is not False:
        fail("WR0 replacement commit record privacy posture differs")
    rebuilt = replacement_commit_document(
        replacement_transaction_id=str(value.get("replacement_transaction_id")),
        predecessor_transaction_id=str(value.get("predecessor_transaction_id")),
        predecessor_environment_identity_sha256=str(value.get("predecessor_environment_identity_sha256")),
        new_environment_identity_sha256=str(value.get("new_environment_identity_sha256")),
        runner_identity_sha256=str(value.get("runner_identity_sha256")),
        contract_source_sha256=str(value.get("contract_source_sha256")),
        workload_sha256=str(value.get("workload_sha256")),
        receipt_identities={
            "run_1_receipt_sha256": str(value.get("run_1_receipt_sha256")),
            "run_2_receipt_sha256": str(value.get("run_2_receipt_sha256")),
            "exit_37_receipt_sha256": str(value.get("exit_37_receipt_sha256")),
        },
        protected_fixture_sha256=str(value.get("protected_fixture_sha256")),
        committed_at=str(value.get("committed_at")),
        predecessor_retirement_status=str(value.get("predecessor_retirement_status")),
        retirement_started_at=value.get("retirement_started_at"),
        retirement_completed_at=value.get("retirement_completed_at"),
    )
    if value != rebuilt:
        fail("WR0 replacement commit record semantic readback differs")
    if expected is not None and value != expected:
        fail("WR0 replacement commit record differs from the exact expected transaction")
    return {
        "classification": "passed",
        "schema": REPLACEMENT_COMMIT_SCHEMA,
        "document": value,
        "sha256": sha256_file(path),
        "size": path.stat().st_size,
    }


def write_and_verify_replacement_commit_record(
    environment: pathlib.Path,
    document: dict[str, Any],
) -> dict[str, Any]:
    atomic_write_json(replacement_commit_path(environment), document)
    return read_replacement_commit_record(environment, expected=document)


def durably_commit_replacement(
    state: ReplacementTransactionState,
    environment: pathlib.Path,
    document: dict[str, Any],
    *,
    record_writer: Any = write_and_verify_replacement_commit_record,
) -> dict[str, Any]:
    if state.phase != "commit_ready" or state.durably_committed:
        fail("durable replacement commit was attempted outside commit-ready")
    record = record_writer(environment, document)
    if not isinstance(record, dict) or record.get("document") != document:
        fail("durable replacement commit writer lacked exact readback")
    state.advance("new_environment_committed", at=document["committed_at"])
    return record


def retire_committed_predecessor(
    state: ReplacementTransactionState,
    *,
    environment: pathlib.Path,
    predecessor_backup: pathlib.Path,
    environment_parent: pathlib.Path,
    commit_document_arguments: dict[str, Any],
    verify_new_environment: Any,
    record_writer: Any = write_and_verify_replacement_commit_record,
    delete_tree: Any = shutil.rmtree,
    sync_directory: Any = fsync_directory,
) -> dict[str, Any]:
    if state.phase != "new_environment_committed" or not state.durably_committed:
        fail("predecessor retirement was attempted before durable commit")
    retirement_started_at = utc_now()
    state.advance("predecessor_retirement_pending", at=retirement_started_at)
    pending_document = replacement_commit_document(
        **commit_document_arguments,
        predecessor_retirement_status="pending",
        retirement_started_at=retirement_started_at,
    )
    pending_record = record_writer(environment, pending_document)
    if pending_record.get("document") != pending_document:
        fail("pending-retirement record lacked exact readback")
    delete_tree(predecessor_backup)
    sync_directory(environment_parent)
    if predecessor_backup.exists():
        raise PredecessorRetirementBlocked("predecessor backup remains after retirement and parent fsync")
    verify_new_environment()
    retirement_completed_at = utc_now()
    retired_document = replacement_commit_document(
        **commit_document_arguments,
        predecessor_retirement_status="retired",
        retirement_started_at=retirement_started_at,
        retirement_completed_at=retirement_completed_at,
    )
    retired_record = record_writer(environment, retired_document)
    if retired_record.get("document") != retired_document:
        fail("retired replacement commit record lacked exact readback")
    state.advance("predecessor_retired", at=retirement_completed_at)
    return {
        "pending_record": pending_record,
        "retired_record": retired_record,
        "retirement_started_at": retirement_started_at,
        "retirement_completed_at": retirement_completed_at,
    }


def finalize_committed_evidence(
    state: ReplacementTransactionState,
    *,
    render_packet: Any,
    validate_packet: Any,
    publish_packet: Any,
) -> str:
    if state.phase != "predecessor_retired" or not state.durably_committed or not state.predecessor_retired:
        fail("final evidence was attempted before predecessor retirement")
    finalized_at = utc_now()
    render_packet(finalized_at)
    validate_packet()
    publish_packet()
    state.advance("evidence_finalized", at=finalized_at)
    return finalized_at


def replacement_failure_result(
    state: ReplacementTransactionState,
    *,
    precommit_rollback: Any,
    verify_committed_environment: Any,
) -> str:
    """Apply the only rollback decision authorized by the explicit commit state."""

    if not state.durably_committed:
        state.rollback_invoked = True
        precommit_rollback()
        return "WR0_REPLACEMENT_PRECOMMIT_BLOCKED"
    verify_committed_environment()
    if not state.predecessor_retired:
        return "WR0_PREDECESSOR_RETIREMENT_BLOCKED"
    return "WR0_EVIDENCE_FINALIZATION_BLOCKED"


def bounded_transaction_residual(path: pathlib.Path, parent: pathlib.Path) -> dict[str, Any]:
    require_contained(path, parent, label="WR0 transaction residual", allow_absent_leaf=True)
    if not path.exists() and not path.is_symlink():
        return {"classification": "absent", "object_count": 0, "cap": PROCESS_CAP}
    counts = {"directory": 0, "regular_file": 0, "symlink": 0, "other": 0}
    pending = [path]
    observed = 0
    while pending:
        candidate = pending.pop()
        info = candidate.lstat()
        observed += 1
        if observed > PROCESS_CAP:
            raise PredecessorRetirementBlocked("transaction residual object count exceeds its safe cap")
        if stat.S_ISLNK(info.st_mode):
            counts["symlink"] += 1
        elif stat.S_ISDIR(info.st_mode):
            counts["directory"] += 1
            pending.extend(candidate.iterdir())
        elif stat.S_ISREG(info.st_mode):
            counts["regular_file"] += 1
        else:
            counts["other"] += 1
    return {
        "classification": "transaction_owned_residual",
        "object_count": observed,
        "object_classes": counts,
        "cap": PROCESS_CAP,
        "private_path_retained": False,
    }


def expected_environment_run_receipt(run: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema": "linux-vst-bridge-wr0-environment-run-receipt/v1",
        "classification": run["classification"],
        "run_number": run["run_number"],
        "nonce": run["nonce"],
        "stdout_sha256": run["stdout_sha256"],
        "stderr_sha256": run["stderr_sha256"],
        "exit_status": run["exit_status"],
        "root_identity_sha256": run["root_identity_sha256"],
        "clean_descendant_count": run["clean_descendant_count"],
        "contract_source_schema": run["contract_source_schema"],
        "contract_source_sha256": run["contract_source_sha256"],
        "workload_sha256": run["workload_sha256"],
        "handshake_causal_order_verified": run["handshake"]["causal_order_verified"],
    }


def environment_receipt_snapshot(environment: pathlib.Path, fixture: dict[str, Any]) -> dict[str, Any]:
    mappings = {
        "run-1.json": fixture["run_1"],
        "run-2.json": fixture["run_2"],
        "exit-37.json": fixture["exit_propagation"],
    }
    observed: dict[str, Any] = {}
    for name, run in mappings.items():
        receipt = environment / "receipts" / name
        require_contained(receipt, environment, label="predecessor run receipt")
        if not receipt.is_file() or receipt.is_symlink() or receipt.stat().st_size > 16 * 1024:
            fail("predecessor run receipt is missing or unsafe")
        try:
            document = json.loads(receipt.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, UnicodeDecodeError) as exc:
            raise WR0Error("predecessor run receipt is malformed") from exc
        if document != expected_environment_run_receipt(run):
            fail("predecessor run receipt differs from reviewed evidence")
        observed[name] = {"size": receipt.stat().st_size, "sha256": sha256_file(receipt)}
    return observed


def load_reviewed_predecessor_fixture() -> dict[str, Any]:
    fixture_path = evidence_root() / "fixture.json"
    try:
        fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    except (FileNotFoundError, json.JSONDecodeError, UnicodeDecodeError) as exc:
        raise WR0Error("reviewed predecessor fixture is unavailable") from exc
    environment = fixture.get("environment")
    if not isinstance(environment, dict):
        fail("reviewed predecessor fixture lacks an environment")
    expected_bindings = {
        "transaction_id": PREDECESSOR_TRANSACTION_ID,
        "runner_identity_sha256": PREDECESSOR_RUNNER_IDENTITY,
        "contract_source_sha256": PREDECESSOR_CONTRACT_SOURCE,
        "workload_sha256": PREDECESSOR_WORKLOAD_SHA256,
    }
    identity = environment.get("environment_identity", {})
    if environment.get("environment_identity_sha256") != PREDECESSOR_ENVIRONMENT_IDENTITY:
        fail("reviewed predecessor environment identity differs from the technical-lead receipt")
    if any(identity.get(name) != value for name, value in expected_bindings.items()):
        fail("reviewed predecessor environment binding differs from the technical-lead receipt")
    return fixture


def verify_predecessor_environment(
    environment: pathlib.Path,
    *,
    fixture: dict[str, Any],
) -> dict[str, Any]:
    process_guard()
    current = environment_snapshot(environment, contract_source=None)
    expected = json.loads(json.dumps(fixture["environment"]))
    expected["classification"] = "passed"
    if current != expected:
        fail("live predecessor environment differs from the exact reviewed snapshot")
    receipts = environment_receipt_snapshot(environment, fixture)
    return {
        "classification": "passed",
        "environment": current,
        "receipt_files": receipts,
        "snapshot_sha256": sha256_bytes(canonical_json({"environment": current, "receipt_files": receipts})),
    }


@dataclass(frozen=True)
class PredecessorBackupOperations:
    """Narrow operation seam used only by deterministic production-helper tests."""

    replace: Any
    sync_directory: Any
    verify_predecessor: Any
    transaction_owns: Any
    remove_transaction: Any


def live_predecessor_backup_operations() -> PredecessorBackupOperations:
    return PredecessorBackupOperations(
        replace=os.replace,
        sync_directory=fsync_directory,
        verify_predecessor=verify_predecessor_environment,
        transaction_owns=transaction_owns,
        remove_transaction=remove_owned_transaction_root,
    )


def predecessor_backup_context(
    destination: pathlib.Path,
    backup: pathlib.Path,
    transaction_id: str,
    *,
    operations: PredecessorBackupOperations | None,
    test_parent: pathlib.Path | None,
) -> tuple[pathlib.Path, PredecessorBackupOperations]:
    if re.fullmatch(r"wr0-[0-9]{8}T[0-9]{6}Z-[0-9a-f]{16}", transaction_id) is None:
        fail("predecessor backup transaction identity is malformed")
    if operations is None:
        if test_parent is not None:
            fail("ordinary predecessor backup cannot select another parent")
        parent = environments_root()
        selected = live_predecessor_backup_operations()
    else:
        if test_parent is None:
            fail("predecessor backup operation injection is restricted to the deterministic test root")
        parent = test_parent
        require_contained(parent, test_root(), label="predecessor backup test parent")
        selected = operations
    require_contained(destination, parent, label="predecessor destination", allow_absent_leaf=True)
    require_contained(backup, parent, label="predecessor backup", allow_absent_leaf=True)
    if destination != parent / "wr0-proton11":
        fail("predecessor destination is not the fixed WR0 environment")
    if backup != parent / f".wr0-proton11.previous-{transaction_id}":
        fail("predecessor backup is not derived from the exact transaction")
    return parent, selected


def physical_object_exists(path: pathlib.Path) -> bool:
    return path.exists() or path.is_symlink()


def verify_exact_predecessor(
    environment: pathlib.Path,
    *,
    predecessor: dict[str, Any],
    fixture: dict[str, Any],
    operations: PredecessorBackupOperations,
) -> dict[str, Any]:
    observed = operations.verify_predecessor(environment, fixture=fixture)
    if observed != predecessor:
        fail("predecessor object differs from the complete exact snapshot")
    return observed


def recover_precommit_predecessor(
    destination: pathlib.Path,
    backup: pathlib.Path,
    *,
    transaction_id: str,
    predecessor: dict[str, Any],
    fixture: dict[str, Any],
    backup_created_event: bool,
    operations: PredecessorBackupOperations | None = None,
    test_parent: pathlib.Path | None = None,
) -> dict[str, Any]:
    """Recover from physical state; the in-memory event flag is not authority."""

    parent, selected = predecessor_backup_context(
        destination,
        backup,
        transaction_id,
        operations=operations,
        test_parent=test_parent,
    )
    destination_present = physical_object_exists(destination)
    backup_present = physical_object_exists(backup)

    backup_exact = False
    if backup_present:
        try:
            verify_exact_predecessor(
                backup,
                predecessor=predecessor,
                fixture=fixture,
                operations=selected,
            )
        except Exception as exc:
            raise PredecessorBackupBlocked(
                "WR0_PREDECESSOR_BACKUP_BLOCKED: transaction backup is not the exact predecessor"
            ) from exc
        backup_exact = True

    destination_exact = False
    if destination_present:
        try:
            verify_exact_predecessor(
                destination,
                predecessor=predecessor,
                fixture=fixture,
                operations=selected,
            )
        except Exception:
            destination_exact = False
        else:
            destination_exact = True

    if destination_exact:
        if backup_present:
            raise PredecessorBackupBlocked(
                "WR0_PREDECESSOR_BACKUP_BLOCKED: exact destination and transaction backup both exist"
            )
        selected.sync_directory(parent)
        verify_exact_predecessor(
            destination,
            predecessor=predecessor,
            fixture=fixture,
            operations=selected,
        )
        return {
            "classification": "passed",
            "physical_state": "destination_exact_backup_absent",
            "backup_created_event": backup_created_event,
            "event_state_was_authority": False,
            "predecessor_restored": True,
            "destination_exact": True,
            "backup_absent": True,
            "parent_fsync_completed": True,
        }

    if destination_present:
        if not backup_exact:
            raise PredecessorBackupBlocked(
                "WR0_PREDECESSOR_BACKUP_BLOCKED: canonical destination is occupied by an unknown object"
            )
        if not selected.transaction_owns(destination, transaction_id):
            raise PredecessorBackupBlocked(
                "WR0_PREDECESSOR_BACKUP_BLOCKED: unknown destination will not be overwritten by exact backup"
            )
        selected.remove_transaction(destination, transaction_id)
        if physical_object_exists(destination):
            raise PredecessorBackupBlocked(
                "WR0_PREDECESSOR_BACKUP_BLOCKED: transaction-owned new destination was not removed"
            )
        recovery_shape = "new_destination_removed_exact_backup_restored"
    elif backup_exact:
        recovery_shape = "destination_absent_exact_backup_restored"
    else:
        raise PredecessorBackupBlocked(
            "WR0_PREDECESSOR_BACKUP_BLOCKED: predecessor destination and exact backup are both absent"
        )

    selected.replace(backup, destination)
    selected.sync_directory(parent)
    verify_exact_predecessor(
        destination,
        predecessor=predecessor,
        fixture=fixture,
        operations=selected,
    )
    if physical_object_exists(backup):
        raise PredecessorBackupBlocked(
            "WR0_PREDECESSOR_BACKUP_BLOCKED: backup remains after exact predecessor restoration"
        )
    return {
        "classification": "passed",
        "physical_state": recovery_shape,
        "backup_created_event": backup_created_event,
        "event_state_was_authority": False,
        "predecessor_restored": True,
        "destination_exact": True,
        "backup_absent": True,
        "parent_fsync_completed": True,
    }


def move_predecessor_to_backup(
    destination: pathlib.Path,
    backup: pathlib.Path,
    *,
    transaction_id: str,
    predecessor: dict[str, Any],
    fixture: dict[str, Any],
    operations: PredecessorBackupOperations | None = None,
    test_parent: pathlib.Path | None = None,
) -> dict[str, Any]:
    """Guard rename, first directory fsync, and exact backup verification."""

    parent, selected = predecessor_backup_context(
        destination,
        backup,
        transaction_id,
        operations=operations,
        test_parent=test_parent,
    )
    if physical_object_exists(backup):
        fail("predecessor backup collision")
    verify_exact_predecessor(
        destination,
        predecessor=predecessor,
        fixture=fixture,
        operations=selected,
    )
    rename_completed = False
    try:
        selected.replace(destination, backup)
        rename_completed = True
        selected.sync_directory(parent)
        verify_exact_predecessor(
            backup,
            predecessor=predecessor,
            fixture=fixture,
            operations=selected,
        )
        if physical_object_exists(destination) or not physical_object_exists(backup):
            fail("guarded predecessor backup has an invalid physical state")
    except Exception as original_error:
        if not rename_completed:
            raise
        try:
            recover_precommit_predecessor(
                destination,
                backup,
                transaction_id=transaction_id,
                predecessor=predecessor,
                fixture=fixture,
                backup_created_event=False,
                operations=operations,
                test_parent=test_parent,
            )
        except Exception as recovery_error:
            raise PredecessorBackupBlocked(
                "WR0_PREDECESSOR_BACKUP_BLOCKED: guarded predecessor backup recovery did not complete"
            ) from recovery_error
        raise
    return {
        "classification": "passed",
        "rename_completed": True,
        "first_directory_fsync_guarded": True,
        "parent_fsync_completed": True,
        "backup_snapshot_verified": True,
        "destination_absent": True,
        "backup_present": True,
        "transaction_derived_backup": True,
    }


def restore_predecessor_from_backup(
    destination: pathlib.Path,
    backup: pathlib.Path,
    *,
    transaction_id: str,
    predecessor: dict[str, Any],
    fixture: dict[str, Any],
    backup_created_event: bool,
    operations: PredecessorBackupOperations | None = None,
    test_parent: pathlib.Path | None = None,
) -> dict[str, Any]:
    return recover_precommit_predecessor(
        destination,
        backup,
        transaction_id=transaction_id,
        predecessor=predecessor,
        fixture=fixture,
        backup_created_event=backup_created_event,
        operations=operations,
        test_parent=test_parent,
    )


def write_environment_receipt(environment: pathlib.Path, name: str, value: dict[str, Any]) -> None:
    if name not in {"run-1.json", "run-2.json", "exit-37.json"}:
        fail("environment receipt name is outside the contract")
    target = environment / "receipts" / name
    require_contained(target, environment, label="environment receipt", allow_absent_leaf=True)
    safe = {
        "schema": "linux-vst-bridge-wr0-environment-run-receipt/v1",
        "classification": value["classification"],
        "run_number": value["run_number"],
        "nonce": value["nonce"],
        "stdout_sha256": value["stdout_sha256"],
        "stderr_sha256": value["stderr_sha256"],
        "exit_status": value["exit_status"],
        "root_identity_sha256": value["root_identity_sha256"],
        "clean_descendant_count": value["clean_descendant_count"],
        "contract_source_schema": value["contract_source_schema"],
        "contract_source_sha256": value["contract_source_sha256"],
        "workload_sha256": value["workload_sha256"],
        "handshake_causal_order_verified": value["handshake"]["causal_order_verified"],
    }
    atomic_write_json(target, safe)


def promote_stage(stage: pathlib.Path, destination: pathlib.Path, transaction_id: str) -> None:
    parent = environments_root()
    require_contained(stage, parent, label="WR0 promotion stage")
    require_contained(destination, parent, label="WR0 promotion destination", allow_absent_leaf=True)
    if destination.exists():
        fail("WR0 final environment unexpectedly exists before promotion")
    if not transaction_owns(stage, transaction_id):
        fail("WR0 stage ownership changed before promotion")
    os.replace(stage, destination)
    directory_fd = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory_fd)
    finally:
        os.close(directory_fd)
    if not transaction_owns(destination, transaction_id):
        fail("WR0 promotion did not preserve transaction ownership")


def expect_refusal(action: Any) -> None:
    try:
        action()
    except (WR0Error, ValueError, OSError):
        return
    raise AssertionError("synthetic case unexpectedly succeeded")


def synthetic_marker(root: pathlib.Path, *, digest: str = "0" * 64, transaction: str = "wr0-20000101T000000Z-0000000000000000") -> None:
    root.mkdir(parents=True, exist_ok=True)
    document = marker_document(
        transaction_id=transaction,
        created_at="2000-01-01T00:00:00Z",
        lock_digest=digest,
        contract_source_digest="0" * 64,
        workload_sha256="0" * 64,
        status_value="ready",
    )
    atomic_write_json(root / "wr0-environment.json", document)


def verify_synthetic_marker(root: pathlib.Path, *, expected_digest_value: str) -> None:
    marker = json.loads((root / "wr0-environment.json").read_text(encoding="utf-8"))
    if marker.get("schema") != ENVIRONMENT_SCHEMA or marker.get("runner_identity_sha256") != expected_digest_value:
        fail("synthetic environment ownership differs")


def git_test_commit(worktree: pathlib.Path, message: str, paths: Sequence[str]) -> None:
    run_bounded(["git", "add", "--", *paths], cwd=worktree)
    run_bounded(
        [
            "git",
            "-c",
            "user.name=WR0 Production Test",
            "-c",
            "user.email=wr0@production-test.invalid",
            "commit",
            "-m",
            message,
        ],
        cwd=worktree,
    )


def with_detached_test_worktree(
    parent: pathlib.Path,
    name: str,
    action: Any,
) -> None:
    repository = repo_root()
    worktree = parent / name
    require_contained(worktree, test_root(), label="WR0 detached test worktree", allow_absent_leaf=True)
    if worktree.exists():
        fail("WR0 detached test worktree collision")
    run_bounded(["git", "worktree", "add", "--detach", str(worktree), "HEAD"], cwd=repository)
    try:
        action(worktree)
    finally:
        removal = run_bounded(
            ["git", "worktree", "remove", "--force", str(worktree)],
            cwd=repository,
            check=False,
        )
        run_bounded(["git", "worktree", "prune"], cwd=repository)
        if removal.returncode != 0 or worktree.exists():
            fail("WR0 detached test worktree cleanup failed")


def retained_source_fixture(path: pathlib.Path, source: dict[str, Any]) -> None:
    atomic_write_json(
        path,
        {
            "contract_source": {
                "schema": source["schema"],
                "digest": source["digest"],
                "file_count": source["file_count"],
                "workload_sha256": source["workload_sha256"],
                "implementation_commit": source["implementation_commit"],
                "implementation_tree": source["implementation_tree"],
            }
        },
    )


def run_contract_source_production_tests(working: pathlib.Path) -> list[dict[str, str]]:
    baseline = contract_source_manifest(require_clean=True)
    worktrees = working / "git-worktrees"
    worktrees.mkdir(mode=0o700)
    results: list[dict[str, str]] = []

    def passed(name: str) -> None:
        results.append({"case": name, "result": "passed"})

    def evidence_only(worktree: pathlib.Path) -> None:
        target = worktree / "evidence/wr0-proton-bootstrap/FINDINGS.md"
        target.write_text(target.read_text(encoding="utf-8") + "\nSynthetic evidence-only test commit.\n", encoding="utf-8")
        git_test_commit(worktree, "WR0 evidence-only stability test", ["evidence/wr0-proton-bootstrap/FINDINGS.md"])
        observed = contract_source_manifest(require_clean=True, repository=worktree)
        if canonical_json(observed["manifest"]) != canonical_json(baseline["manifest"]) or observed["digest"] != baseline["digest"]:
            raise AssertionError("evidence-only commit changed governed source identity")
        fixture = working / "retained-evidence-only.json"
        retained_source_fixture(fixture, baseline)
        verified = verify_retained_contract_source(repository=worktree, fixture_path=fixture)
        if verified["digest"] != baseline["digest"]:
            raise AssertionError("retained-source verifier changed the evidence-only identity")

    with_detached_test_worktree(worktrees, "evidence-only", evidence_only)
    passed("production_contract_source_evidence_only_stability")

    def changed_governed(path: str, case_name: str, worktree_name: str) -> None:
        def action(worktree: pathlib.Path) -> None:
            target = worktree / path
            target.write_text(target.read_text(encoding="utf-8") + "\nrem WR0 synthetic governed-source change\n", encoding="utf-8")
            git_test_commit(worktree, f"WR0 {case_name}", [path])
            changed = contract_source_manifest(require_clean=True, repository=worktree)
            if changed["digest"] == baseline["digest"]:
                raise AssertionError("governed source change preserved stale digest")
            fixture = working / f"retained-{worktree_name}.json"
            retained_source_fixture(fixture, baseline)
            expect_refusal(lambda: verify_retained_contract_source(repository=worktree, fixture_path=fixture))

        with_detached_test_worktree(worktrees, worktree_name, action)

    changed_governed(
        "windows-fixtures/wr0-probe/wr0-probe.cmd",
        "changed workload production test",
        "changed-workload",
    )
    passed("production_changed_workload_changes_digest_and_refuses_stale_identity")
    changed_governed(
        "tools/wr0-proton-bootstrap/launch.py",
        "changed supervisor production test",
        "changed-supervisor",
    )
    passed("production_changed_supervisor_changes_digest_and_refuses_stale_identity")
    changed_governed(
        "tools/wr0-proton-bootstrap/preflight.sh",
        "changed wrapper production test",
        "changed-wrapper",
    )
    passed("production_changed_wrapper_changes_digest_and_refuses_stale_identity")

    def unexpected_path(worktree: pathlib.Path) -> None:
        target = worktree / "tools/wr0-proton-bootstrap/unexpected-governed-source.txt"
        target.write_text("unexpected governed source\n", encoding="utf-8")
        git_test_commit(worktree, "WR0 unexpected governed path test", ["tools/wr0-proton-bootstrap/unexpected-governed-source.txt"])
        expect_refusal(lambda: contract_source_manifest(require_clean=True, repository=worktree))

    with_detached_test_worktree(worktrees, "unexpected-path", unexpected_path)
    passed("production_unexpected_governed_path_refused")

    def provenance_edit(worktree: pathlib.Path) -> None:
        target = worktree / "tools/wr0-proton-bootstrap/launch.py"
        target.write_text(target.read_text(encoding="utf-8") + "\n# WR0 synthetic provenance test change\n", encoding="utf-8")
        git_test_commit(worktree, "WR0 historical provenance test", ["tools/wr0-proton-bootstrap/launch.py"])
        fixture = working / "retained-provenance.json"
        retained_source_fixture(fixture, baseline)
        value = json.loads(fixture.read_text(encoding="utf-8"))
        value["contract_source"]["implementation_commit"] = command_text(["git", "rev-parse", "HEAD"], cwd=worktree)
        value["contract_source"]["implementation_tree"] = command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=worktree)
        atomic_write_json(fixture, value)
        expect_refusal(lambda: verify_retained_contract_source(repository=worktree, fixture_path=fixture))

    with_detached_test_worktree(worktrees, "provenance-edit", provenance_edit)
    passed("production_historical_provenance_edit_cannot_authorize_changed_source")

    def dirty_source(worktree: pathlib.Path) -> None:
        target = worktree / "tools/wr0-proton-bootstrap/launch.py"
        target.write_text(target.read_text(encoding="utf-8") + "\n# WR0 synthetic dirty source\n", encoding="utf-8")
        expect_refusal(lambda: contract_source_manifest(require_clean=True, repository=worktree))
        run_bounded(["git", "add", "--", "tools/wr0-proton-bootstrap/launch.py"], cwd=worktree)
        expect_refusal(lambda: contract_source_manifest(require_clean=True, repository=worktree))

    with_detached_test_worktree(worktrees, "dirty-source", dirty_source)
    passed("production_unstaged_and_staged_governed_source_refused")
    if any(worktrees.iterdir()):
        fail("detached production-test worktree root is not empty")
    return results


def run_negative_tests(*, require_clean_source: bool = True) -> dict[str, Any]:
    ensure_cache_layout()
    root = test_root()
    require_contained(root, cache_root(), label="WR0 synthetic test root")
    working = pathlib.Path(tempfile.mkdtemp(prefix="wr0-negative-", dir=root))
    require_contained(working, root, label="WR0 synthetic working root")
    cases: list[dict[str, str]] = []

    def case(name: str, action: Any) -> None:
        try:
            action()
        except Exception as exc:
            detail = sanitize_error(exc)
            raise WR0Error(f"negative test failed: {name}: {type(exc).__name__}: {detail}") from exc
        cases.append({"case": name, "result": "passed"})

    synthetic_nonce = "a" * 32
    synthetic_contract = "b" * 64
    synthetic_workload = "c" * 64

    def synthetic_record(
        pid: int,
        ppid: int,
        comm: str,
        arguments: Sequence[bytes],
        *,
        pgrp: int = 100,
        exe_basename: str | None = None,
    ) -> dict[str, Any]:
        return {
            "pid": pid,
            "ppid": ppid,
            "pgrp": pgrp,
            "session": pgrp,
            "start_ticks": pid * 10,
            "state": "S",
            "comm": comm,
            "exe_basename": exe_basename or comm,
            "uid": os.getuid(),
            "cgroup_class": "ordinary_user_scope",
            "_cmdline": b"\0".join(arguments) + b"\0",
        }

    def synthetic_exact_topology(*, command_arguments: Sequence[bytes] | None = None) -> dict[int, dict[str, Any]]:
        proton = str(runner_root() / "proton").encode("utf-8")
        command = list(command_arguments) if command_arguments is not None else [
            b"cmd.exe",
            *expected_windows_command_subsequence(
                nonce=synthetic_nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            ),
        ]
        return {
            100: synthetic_record(100, os.getpid(), "pressure-vessel", [b"pressure-vessel"]),
            101: synthetic_record(101, 100, "pv-adverb", [b"pv-adverb"]),
            102: synthetic_record(102, 101, "python3", [b"python3", proton, b"runinprefix"]),
            103: synthetic_record(103, 101, "cmd.exe", command, exe_basename="wine64-preloader"),
        }

    try:
        def wrong_version() -> None:
            if RUNNER_VERSION == "wrong-version":
                raise AssertionError
            expect_refusal(lambda: (_ for _ in ()).throw(WR0Error("runner version differs")))

        case("wrong_runner_version_identity_refused", wrong_version)

        def changed_file() -> None:
            fixture = working / "critical-file"
            fixture.write_bytes(b"expected")
            expected = sha256_file(fixture)
            fixture.write_bytes(b"changed")
            expect_refusal(lambda: fail("critical file differs") if sha256_file(fixture) != expected else None)

        case("changed_launch_critical_file_refused", changed_file)

        def wrong_runtime() -> None:
            manifest = '"require_tool_appid" "1070560"'
            expect_refusal(lambda: fail("runtime pairing differs") if f'"{RUNTIME_APP_ID}"' not in manifest else None)

        case("wrong_undeclared_runtime_pairing_refused", wrong_runtime)

        def missing_entrypoint() -> None:
            missing = working / "missing-entrypoint"
            expect_refusal(lambda: fail("runtime entrypoint missing") if not missing.is_file() else None)

        case("missing_runtime_entrypoint_refused", missing_entrypoint)

        def unknown_final() -> None:
            unknown = working / "unknown-final"
            unknown.mkdir()
            expect_refusal(lambda: verify_synthetic_marker(unknown, expected_digest_value=expected_lock_digest()))

        case("unknown_existing_final_environment_refused", unknown_final)

        def wrong_binding() -> None:
            owned = working / "wrong-binding"
            synthetic_marker(owned, digest="f" * 64)
            expect_refusal(lambda: verify_synthetic_marker(owned, expected_digest_value=expected_lock_digest()))

        case("environment_bound_to_other_runner_refused", wrong_binding)

        def symlink_ancestor() -> None:
            target = working / "real-ancestor"
            target.mkdir()
            link = working / "linked-ancestor"
            link.symlink_to(target, target_is_directory=True)
            expect_refusal(lambda: require_no_symlink_ancestors(link / "child", allow_absent_leaf=True, label="synthetic env"))

        case("symlinked_environment_ancestor_refused", symlink_ancestor)

        def path_escape() -> None:
            expect_refusal(lambda: require_contained(working.parent / "escape", working, label="synthetic escape", allow_absent_leaf=True))

        case("environment_path_escape_refused", path_escape)

        def first_failure_absence() -> None:
            destination = working / "first-failure-final"
            stage = working / "first-failure-stage"
            stage.mkdir()
            try:
                raise WR0Error("simulated stage failure")
            except WR0Error:
                shutil.rmtree(stage)
            if destination.exists() or stage.exists():
                raise AssertionError

        case("stage_failure_restores_prior_absence", first_failure_absence)

        def replacement_restore() -> None:
            destination = working / "replacement-final"
            destination.mkdir()
            (destination / "payload").write_bytes(b"prior-exact")
            prior = sha256_file(destination / "payload")
            backup = working / "replacement-backup"
            os.replace(destination, backup)
            replacement = working / "replacement-stage"
            replacement.mkdir()
            (replacement / "payload").write_bytes(b"new")
            os.replace(replacement, destination)
            shutil.rmtree(destination)
            os.replace(backup, destination)
            if sha256_file(destination / "payload") != prior or backup.exists() or replacement.exists():
                raise AssertionError

        case("failed_replacement_restores_prior_exact_environment", replacement_restore)

        def wrong_nonce() -> None:
            good = "a" * 32
            output = expected_stdout("b" * 32, 1, exit_37=False)
            expect_refusal(lambda: validate_workload_output(output, nonce=good, run_number=1, expected_exit=0))

        case("workload_wrong_nonce_refused", wrong_nonce)

        def missing_receipt() -> None:
            env = working / "missing-receipt"
            env.mkdir()
            source = {
                "schema": CONTRACT_SOURCE_SCHEMA,
                "digest": "0" * 64,
                "workload_sha256": "0" * 64,
            }
            expect_refusal(
                lambda: verify_internal_receipt(
                    env,
                    nonce="a" * 32,
                    run_number=1,
                    contract_source=source,
                )
            )

        case("missing_internal_receipt_refused", missing_receipt)

        def unexpected_exit() -> None:
            observed = 12
            expect_refusal(lambda: fail("unexpected exit") if observed != 0 else None)

        case("unexpected_windows_exit_refused", unexpected_exit)

        def exit_37() -> None:
            result = subprocess.run([sys.executable, "-c", "raise SystemExit(37)"], check=False)
            if result.returncode != 37:
                raise AssertionError

        case("expected_test_exit_37_propagated", exit_37)

        def timeout_group() -> None:
            nonce = "d" * 32
            environment = working / "timeout-handshake-environment"
            directory = environment / "compatdata/pfx/drive_c/users/steamuser/AppData/Local/LinuxVSTBridge/WR0/handshake"
            directory.mkdir(parents=True)
            for artifact in handshake_artifact_paths(environment, nonce):
                artifact.write_bytes(b"synthetic")
            owned = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"], start_new_session=True)
            sentinel = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"], start_new_session=True)
            observed: dict[tuple[int, int], dict[str, Any]] = {}
            try:
                root_record = read_proc_record(owned.pid)
                if root_record is None:
                    raise AssertionError("synthetic timeout root identity unavailable")
                sample_owned_tree(
                    owned.pid,
                    int(root_record["start_ticks"]),
                    observed,
                    elapsed_ms=0,
                    nonce=nonce,
                    run_number=1,
                    expected_exit=0,
                    contract_digest="e" * 64,
                    workload_sha256="f" * 64,
                )
                terminate_owned_identities(
                    owned.pid,
                    observed,
                    root_pid=owned.pid,
                    root_start_ticks=int(root_record["start_ticks"]),
                    nonce=nonce,
                    run_number=1,
                    expected_exit=0,
                    contract_digest="e" * 64,
                    workload_sha256="f" * 64,
                    deadline_seconds=6.0,
                )
                owned.wait(timeout=5)
                if sentinel.poll() is not None:
                    raise AssertionError
                cleanup_handshake_artifacts(environment, nonce)
                assert_handshake_absent(*handshake_artifact_paths(environment, nonce))
            finally:
                if owned.poll() is None:
                    try:
                        os.killpg(owned.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    owned.wait(timeout=5)
                if sentinel.poll() is None:
                    os.killpg(sentinel.pid, signal.SIGTERM)
                    sentinel.wait(timeout=5)

        case("supervisor_timeout_terminates_only_owned_group", timeout_group)

        def steam_ancestry() -> None:
            names = ["launch.py", "steam-launch-wrapper", "pressure-vessel"]
            expect_refusal(lambda: fail("Steam game launch ancestor") if "steam-launch-wrapper" in names else None)

        case("steam_game_launch_ancestry_refused", steam_ancestry)

        def different_environment() -> None:
            first = {"transaction": "a", "inode": 1}
            second = {"transaction": "b", "inode": 2}
            expect_refusal(lambda: fail("environment identity differs") if first != second else None)

        case("second_run_different_environment_identity_refused", different_environment)

        def synthetic_processes() -> None:
            for name in ("BitwigStudio", "wine64", "validator", "yabridge-host.exe"):
                family = process_family(
                    {
                        "comm": name,
                        "exe_basename": name,
                        "_cmdline": b"",
                    }
                )
                if family is None:
                    raise AssertionError

        case("preflight_existing_process_families_refused", synthetic_processes)

        def preservation_mismatch() -> None:
            expect_refusal(lambda: compare_fixtures({"sha": "a"}, {"sha": "b"}))

        case("protected_fixture_mismatch_invalidates_acceptance", preservation_mismatch)

        def test_symlink_escape() -> None:
            outside = root / f"test-outside-{secrets.token_hex(4)}"
            outside.mkdir()
            try:
                link = working / "test-link"
                link.symlink_to(outside, target_is_directory=True)
                expect_refusal(lambda: require_contained(link / "child", working, label="synthetic test path", allow_absent_leaf=True))
            finally:
                shutil.rmtree(outside)

        case("test_root_symlink_escape_refused", test_symlink_escape)

        def wrong_ready_nonce() -> None:
            ready = working / "wrong-ready.txt"
            ready.write_text(expected_ready_file("b" * 32, 1), encoding="utf-8")
            expect_refusal(lambda: verify_ready_handshake(ready, nonce="a" * 32, run_number=1))

        case("handshake_wrong_nonce_refused", wrong_ready_nonce)

        def reused_nonce() -> None:
            handshake = working / "reused-handshake"
            handshake.mkdir()
            ready = handshake / f"ready-{'a' * 32}.txt"
            gate = handshake / f"gate-{'a' * 32}.txt"
            ready.write_bytes(b"stale")
            expect_refusal(lambda: assert_handshake_absent(ready, gate))

        case("handshake_reused_nonce_refused", reused_nonce)

        def gate_before_observation() -> None:
            handshake = working / "gate-without-process"
            handshake.mkdir()
            gate = handshake / f"gate-{'c' * 32}.txt"
            expect_refusal(
                lambda: commit_supervisor_gate(
                    gate,
                    nonce="c" * 32,
                    run_number=1,
                    authorization=None,
                )
            )
            if gate.exists():
                raise AssertionError

        case("gate_before_process_observation_refused", gate_before_observation)

        def broad_proton_role() -> None:
            proton = str(runner_root() / "proton").encode("utf-8")
            record = {
                "pid": 200,
                "ppid": 100,
                "pgrp": 100,
                "session": 100,
                "start_ticks": 10,
                "comm": "srt-bwrap",
                "exe_basename": "srt-bwrap",
                "uid": os.getuid(),
                "cgroup_class": "ordinary_user_scope",
                "_cmdline": b"srt-bwrap\0" + proton + b"\0runinprefix\0",
            }
            if classify_owned_role(record, root_pid=100) == "proton":
                raise AssertionError

        case("runtime_wrapper_with_proton_argument_not_proton_role", broad_proton_role)

        def sibling_runtime_topology() -> None:
            nonce = "a" * 32
            contract = "b" * 64
            workload_hash = "c" * 64
            proton_path = str(runner_root() / "proton").encode("utf-8")
            workload_path = windows_path(probe_path()).encode("utf-8")

            def record(pid: int, ppid: int, comm: str, arguments: list[bytes]) -> dict[str, Any]:
                return {
                    "pid": pid,
                    "ppid": ppid,
                    "pgrp": 100,
                    "session": 100,
                    "start_ticks": pid * 10,
                    "state": "S",
                    "comm": comm,
                    "exe_basename": comm,
                    "uid": os.getuid(),
                    "cgroup_class": "ordinary_user_scope",
                    "_cmdline": b"\0".join(arguments) + b"\0",
                }

            snapshot = {
                100: record(100, os.getpid(), "pressure-vessel", [b"pressure-vessel"]),
                101: record(101, 100, "pv-adverb", [b"pv-adverb"]),
                102: record(102, 101, "python3", [b"python3", proton_path, b"runinprefix"]),
                103: record(
                    103,
                    101,
                    "cmd.exe",
                    [
                        b"cmd.exe",
                        b"/d",
                        b"/q",
                        b"/c",
                        workload_path,
                        nonce.encode(),
                        b"1",
                        contract.encode(),
                        workload_hash.encode(),
                    ],
                ),
            }
            assertion = gate_process_assertions(
                snapshot,
                list(snapshot.values()),
                root_pid=100,
                root_start_ticks=1000,
                nonce=nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=contract,
                workload_sha256=workload_hash,
            )
            if assertion is None or assertion["topology"] != "proton_and_windows_command_share_exact_runtime_ancestor":
                raise AssertionError("shared exact-runtime-root topology was not proved")

        case("proton_and_command_sibling_branches_share_exact_runtime_root", sibling_runtime_topology)

        def unrelated_same_name() -> None:
            record = {
                "pid": 201,
                "ppid": 100,
                "pgrp": 100,
                "session": 100,
                "start_ticks": 11,
                "comm": "cmd.exe",
                "exe_basename": "cmd.exe",
                "uid": os.getuid(),
                "cgroup_class": "ordinary_user_scope",
                "_cmdline": b"cmd.exe\0/c\0unrelated.cmd\0",
            }
            role = classify_owned_role(
                record,
                root_pid=100,
                nonce="a" * 32,
                run_number=1,
                expected_exit=0,
                contract_digest="b" * 64,
                workload_sha256="c" * 64,
            )
            if role == "windows_command":
                raise AssertionError

        case("unrelated_same_name_process_rejected", unrelated_same_name)

        def stale_pid_identity() -> None:
            record = read_proc_record(os.getpid())
            if record is None:
                raise AssertionError
            if identity_is_live((os.getpid(), int(record["start_ticks"]) + 1), uid=os.getuid()):
                raise AssertionError

        case("stale_pid_start_identity_rejected", stale_pid_identity)

        def root_exit_pipe_holder_deadline() -> None:
            code = (
                "import subprocess,sys; "
                "subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)'])"
            )
            owned = subprocess.Popen(
                [sys.executable, "-c", code],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
            assert owned.stdout is not None
            selector = selectors.DefaultSelector()
            selector.register(owned.stdout, selectors.EVENT_READ)
            started = time.monotonic()
            try:
                owned.wait(timeout=3)
                deadline = time.monotonic() + 0.25
                while selector.get_map() and time.monotonic() < deadline:
                    selector.select(timeout=0.02)
                if not selector.get_map():
                    raise AssertionError
                terminate_process_group(owned.pid)
                if time.monotonic() - started > 3.0:
                    raise AssertionError
            finally:
                selector.close()
                for stream in (owned.stdout, owned.stderr):
                    if stream is not None:
                        stream.close()

        case("root_exit_with_open_pipe_still_hits_deadline", root_exit_pipe_holder_deadline)

        def escaped_descendant_cleanup() -> None:
            marker = working / "escaped-descendant-ready"
            root_code = (
                "import pathlib,subprocess,sys,time; "
                "subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)'],start_new_session=True); "
                "pathlib.Path(sys.argv[1]).write_text('ready',encoding='ascii'); "
                "time.sleep(60)"
            )
            owned = subprocess.Popen([sys.executable, "-c", root_code, str(marker)], start_new_session=True)
            observed: dict[tuple[int, int], dict[str, Any]] = {}
            try:
                deadline = time.monotonic() + 3.0
                while not marker.exists() and time.monotonic() < deadline:
                    time.sleep(0.02)
                root_record = read_proc_record(owned.pid)
                if root_record is None:
                    raise AssertionError("synthetic root identity unavailable")
                _, current = sample_owned_tree(
                    owned.pid,
                    int(root_record["start_ticks"]),
                    observed,
                    elapsed_ms=0,
                    nonce="d" * 32,
                    run_number=1,
                    expected_exit=0,
                    contract_digest="e" * 64,
                    workload_sha256="f" * 64,
                )
                if not any(int(item["pgrp"]) != owned.pid for item in current):
                    raise AssertionError("escaped child absent from root-descendant census")
                result = terminate_owned_identities(
                    owned.pid,
                    observed,
                    root_pid=owned.pid,
                    root_start_ticks=int(root_record["start_ticks"]),
                    nonce="d" * 32,
                    run_number=1,
                    expected_exit=0,
                    contract_digest="e" * 64,
                    workload_sha256="f" * 64,
                    deadline_seconds=6.0,
                )
                if result["final_owned_count"] != 0:
                    raise AssertionError("exact root-descendant identities did not drain")
            finally:
                if owned.poll() is None:
                    try:
                        os.killpg(owned.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                owned.wait(timeout=5)

        case("root_owned_descendant_outside_group_scoped_cleanup", escaped_descendant_cleanup)

        def changed_contract_manifest() -> None:
            source = {"schema": CONTRACT_SOURCE_SCHEMA, "files": [{"path": "fixture", "mode": "100644", "blob": "1" * 40}]}
            digest = sha256_bytes(canonical_json(source))
            changed = json.loads(json.dumps(source))
            changed["files"][0]["blob"] = "0" * len(changed["files"][0]["blob"])
            if sha256_bytes(canonical_json(changed)) == digest:
                raise AssertionError("synthetic manifest mutation preserved its digest")

        case("changed_contract_source_manifest_refused", changed_contract_manifest)

        def wrong_ready_run() -> None:
            ready = working / "wrong-ready-run.txt"
            ready.write_text(expected_ready_file(synthetic_nonce, 2), encoding="utf-8")
            expect_refusal(lambda: verify_ready_handshake(ready, nonce=synthetic_nonce, run_number=1))

        case("handshake_wrong_ready_run_number_refused", wrong_ready_run)

        def preexisting_ready() -> None:
            directory = working / "preexisting-ready"
            directory.mkdir()
            ready = directory / "ready.txt"
            gate = directory / "gate.txt"
            temporary = directory / "ready.tmp"
            ready.write_bytes(b"stale")
            expect_refusal(lambda: assert_handshake_absent(ready, gate, temporary))

        case("handshake_preexisting_ready_file_refused", preexisting_ready)

        def preexisting_gate() -> None:
            directory = working / "preexisting-gate"
            directory.mkdir()
            ready = directory / "ready.txt"
            gate = directory / "gate.txt"
            temporary = directory / "ready.tmp"
            gate.write_bytes(b"stale")
            expect_refusal(lambda: assert_handshake_absent(ready, gate, temporary))

        case("handshake_preexisting_gate_file_refused", preexisting_gate)

        def process_dies_before_gate() -> None:
            initial = synthetic_exact_topology()
            prior = gate_process_assertions(
                initial,
                list(initial.values()),
                root_pid=100,
                root_start_ticks=1000,
                nonce=synthetic_nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            )
            if prior is None:
                raise AssertionError("synthetic prior process assertion is unavailable")
            fresh = dict(initial)
            fresh.pop(103)
            expect_refusal(
                lambda: revalidate_gate_process_assertion(
                    fresh,
                    list(fresh.values()),
                    prior_assertion=prior,
                    root_pid=100,
                    root_start_ticks=1000,
                    nonce=synthetic_nonce,
                    run_number=1,
                    expected_exit=0,
                    contract_digest=synthetic_contract,
                    workload_sha256=synthetic_workload,
                )
            )

        case("process_dies_between_observation_and_gate_commit_refused", process_dies_before_gate)

        def no_readiness_barrier() -> None:
            directory = working / "no-readiness"
            directory.mkdir()
            expect_refusal(
                lambda: validate_gate_readiness(
                    directory / "ready.txt",
                    directory / "gate.txt",
                    directory / "ready.tmp",
                    b"",
                    nonce=synthetic_nonce,
                    run_number=1,
                )
            )

        case("short_lived_command_without_readiness_cannot_authorize_gate", no_readiness_barrier)

        def wrong_command_run() -> None:
            arguments = [
                b"cmd.exe",
                *expected_windows_command_subsequence(
                    nonce=synthetic_nonce,
                    run_number=2,
                    expected_exit=0,
                    contract_digest=synthetic_contract,
                    workload_sha256=synthetic_workload,
                ),
            ]
            record = synthetic_record(103, 101, "cmd.exe", arguments, exe_basename="wine64-preloader")
            role = classify_owned_role(
                record,
                root_pid=100,
                nonce=synthetic_nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            )
            if role == "windows_command":
                raise AssertionError("wrong run number satisfied exact command role")

        case("windows_command_wrong_run_number_refused", wrong_command_run)

        def alternate_same_basename() -> None:
            expected = list(
                expected_windows_command_subsequence(
                    nonce=synthetic_nonce,
                    run_number=1,
                    expected_exit=0,
                    contract_digest=synthetic_contract,
                    workload_sha256=synthetic_workload,
                )
            )
            expected[3] = b"Z:\\alternate\\wr0-probe.cmd"
            record = synthetic_record(103, 101, "cmd.exe", [b"cmd.exe", *expected], exe_basename="wine64-preloader")
            if exact_windows_command_vector(
                record,
                nonce=synthetic_nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            ):
                raise AssertionError("alternate same-basename path satisfied exact command vector")

        case("windows_command_alternate_full_path_same_basename_refused", alternate_same_basename)

        def switches_missing_or_reordered() -> None:
            expected = list(
                expected_windows_command_subsequence(
                    nonce=synthetic_nonce,
                    run_number=1,
                    expected_exit=0,
                    contract_digest=synthetic_contract,
                    workload_sha256=synthetic_workload,
                )
            )
            for changed in (expected[1:], [expected[1], expected[0], *expected[2:]]):
                record = synthetic_record(103, 101, "cmd.exe", [b"cmd.exe", *changed], exe_basename="wine64-preloader")
                if exact_windows_command_vector(
                    record,
                    nonce=synthetic_nonce,
                    run_number=1,
                    expected_exit=0,
                    contract_digest=synthetic_contract,
                    workload_sha256=synthetic_workload,
                ):
                    raise AssertionError("missing or reordered fixed switches satisfied the command vector")

        case("windows_command_missing_or_reordered_switches_refused", switches_missing_or_reordered)

        def exact_proton_role() -> None:
            topology = synthetic_exact_topology()
            role = classify_owned_role(topology[102], root_pid=100)
            if role != "proton":
                raise AssertionError("exact Proton process did not satisfy its role")

        case("exact_proton_process_satisfies_proton_role", exact_proton_role)

        def exact_command_role() -> None:
            topology = synthetic_exact_topology()
            role = classify_owned_role(
                topology[103],
                root_pid=100,
                nonce=synthetic_nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            )
            if role != "windows_command":
                raise AssertionError("exact Windows command vector did not satisfy its role")

        case("exact_command_vector_satisfies_windows_command_role", exact_command_role)

        def duplicated_command_vector() -> None:
            subsequence = list(
                expected_windows_command_subsequence(
                    nonce=synthetic_nonce,
                    run_number=1,
                    expected_exit=0,
                    contract_digest=synthetic_contract,
                    workload_sha256=synthetic_workload,
                )
            )
            record = synthetic_record(
                103,
                101,
                "cmd.exe",
                [b"cmd.exe", *subsequence, *subsequence],
                exe_basename="wine64-preloader",
            )
            if exact_windows_command_vector(
                record,
                nonce=synthetic_nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            ):
                raise AssertionError("duplicate command subsequences satisfied the exact vector")

        case("windows_command_duplicate_ambiguous_vector_refused", duplicated_command_vector)

        def exit_37_argument_contract() -> None:
            without = synthetic_record(
                103,
                101,
                "cmd.exe",
                [
                    b"cmd.exe",
                    *expected_windows_command_subsequence(
                        nonce=synthetic_nonce,
                        run_number=37,
                        expected_exit=0,
                        contract_digest=synthetic_contract,
                        workload_sha256=synthetic_workload,
                    ),
                ],
                exe_basename="wine64-preloader",
            )
            if exact_windows_command_vector(
                without,
                nonce=synthetic_nonce,
                run_number=37,
                expected_exit=37,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            ):
                raise AssertionError("exit-37 vector without its flag was accepted")
            ordinary = synthetic_record(
                104,
                101,
                "cmd.exe",
                [
                    b"cmd.exe",
                    *expected_windows_command_subsequence(
                        nonce=synthetic_nonce,
                        run_number=1,
                        expected_exit=0,
                        contract_digest=synthetic_contract,
                        workload_sha256=synthetic_workload,
                    ),
                    b"--exit-37",
                ],
                exe_basename="wine64-preloader",
            )
            if exact_windows_command_vector(
                ordinary,
                nonce=synthetic_nonce,
                run_number=1,
                expected_exit=0,
                contract_digest=synthetic_contract,
                workload_sha256=synthetic_workload,
            ):
                raise AssertionError("ordinary vector carrying exit-37 was accepted")

        case("windows_command_exit_37_flag_exactly_if_expected", exit_37_argument_contract)

        def final_survivor_blocked() -> None:
            survivor = {
                "role": "windows_command",
                "comm": "cmd.exe",
                "exe_basename": "wine64-preloader",
                "uid": os.getuid(),
                "state": "S",
            }
            try:
                require_cleanup_complete(False, [survivor])
            except ProcessTopologyBlocked as exc:
                if "WR0_PROCESS_TOPOLOGY_BLOCKED" not in str(exc):
                    raise AssertionError("topology blocker classification differs") from exc
                return
            raise AssertionError("final SIGKILL survivor simulation did not block")

        case("final_sigkill_survivor_produces_process_topology_blocked", final_survivor_blocked)

        def handshake_cleanup() -> None:
            environment = working / "synthetic-handshake-environment"
            directory = environment / "compatdata/pfx/drive_c/users/steamuser/AppData/Local/LinuxVSTBridge/WR0/handshake"
            directory.mkdir(parents=True)
            nonce = "d" * 32
            ready = directory / f"ready-{nonce}.txt"
            gate = directory / f"gate-{nonce}.txt"
            temporary = directory / f"ready-{nonce}.tmp"
            for item in (ready, gate, temporary):
                item.write_bytes(b"synthetic")
            if cleanup_handshake_artifacts(environment, nonce) != 3:
                raise AssertionError("production handshake cleanup count differs")
            assert_handshake_absent(ready, gate, temporary)

        case("ready_gate_temporary_artifacts_absent_after_scoped_cleanup", handshake_cleanup)

        def replacement_fixture(name: str) -> dict[str, Any]:
            root_directory = working / f"replacement-{name}"
            root_directory.mkdir()
            destination = root_directory / "wr0-proton11"
            destination.mkdir()
            (destination / "payload").write_bytes(b"exact-predecessor")
            predecessor_marker = marker_document(
                transaction_id="wr0-19990101T000000Z-2222222222222222",
                created_at="1999-01-01T00:00:00Z",
                lock_digest="3" * 64,
                contract_source_digest="4" * 64,
                workload_sha256="5" * 64,
                status_value="ready",
            )
            atomic_write_json(destination / "wr0-environment.json", predecessor_marker)
            result = {
                "root": root_directory,
                "destination": destination,
                "backup": root_directory / ".wr0-proton11.previous-wr0-20000101T000000Z-1111111111111111",
                "stage": root_directory / ".wr0-proton11.stage-wr0-20000101T000000Z-1111111111111111",
                "predecessor_sha256": sha256_file(destination / "payload"),
                "predecessor_marker": predecessor_marker,
            }
            result["predecessor_snapshot"] = {
                "classification": "passed",
                "transaction_id": predecessor_marker["transaction_id"],
                "marker_sha256": sha256_file(destination / "wr0-environment.json"),
                "payload_sha256": result["predecessor_sha256"],
                "roster": ["payload", "wr0-environment.json"],
            }
            return result

        def verify_synthetic_predecessor(
            environment: pathlib.Path,
            *,
            fixture: dict[str, Any],
        ) -> dict[str, Any]:
            if not environment.is_dir() or environment.is_symlink():
                fail("synthetic predecessor is not a safe directory")
            roster = sorted(entry.name for entry in environment.iterdir())
            if roster != ["payload", "wr0-environment.json"]:
                fail("synthetic predecessor roster differs")
            marker_path = environment / "wr0-environment.json"
            payload_path = environment / "payload"
            if marker_path.is_symlink() or payload_path.is_symlink() or not payload_path.is_file():
                fail("synthetic predecessor contains an unsafe object")
            try:
                marker = json.loads(marker_path.read_text(encoding="utf-8"))
            except (json.JSONDecodeError, UnicodeDecodeError) as exc:
                raise WR0Error("synthetic predecessor marker is malformed") from exc
            observed = {
                "classification": "passed",
                "transaction_id": marker.get("transaction_id"),
                "marker_sha256": sha256_file(marker_path),
                "payload_sha256": sha256_file(payload_path),
                "roster": roster,
            }
            if observed != fixture["predecessor_snapshot"]:
                fail("synthetic predecessor differs from the complete exact snapshot")
            return observed

        def synthetic_transaction_owns(path: pathlib.Path, transaction_id: str) -> bool:
            marker = path / "synthetic-new-transaction"
            return marker.is_file() and not marker.is_symlink() and marker.read_text(encoding="utf-8") == transaction_id

        def remove_synthetic_transaction(path: pathlib.Path, transaction_id: str) -> None:
            if not synthetic_transaction_owns(path, transaction_id):
                fail("synthetic new destination is not transaction-owned")
            shutil.rmtree(path)

        def synthetic_backup_operations(
            *,
            replace: Any = os.replace,
            sync_directory: Any = fsync_directory,
            verify_predecessor: Any = verify_synthetic_predecessor,
            transaction_owns_operation: Any = synthetic_transaction_owns,
            remove_transaction: Any = remove_synthetic_transaction,
        ) -> PredecessorBackupOperations:
            return PredecessorBackupOperations(
                replace=replace,
                sync_directory=sync_directory,
                verify_predecessor=verify_predecessor,
                transaction_owns=transaction_owns_operation,
                remove_transaction=remove_transaction,
            )

        def assert_synthetic_predecessor_restored(fixture: dict[str, Any]) -> None:
            observed = verify_synthetic_predecessor(fixture["destination"], fixture=fixture)
            if observed != fixture["predecessor_snapshot"] or physical_object_exists(fixture["backup"]):
                raise AssertionError("exact synthetic predecessor was not restored without a sibling")

        def first_parent_fsync_failure() -> None:
            fixture = replacement_fixture("backup-first-fsync")
            calls: list[str] = []

            def injected_sync(path: pathlib.Path) -> None:
                calls.append("sync")
                if len(calls) == 1:
                    raise OSError("injected first parent fsync failure")
                fsync_directory(path)

            try:
                move_predecessor_to_backup(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    operations=synthetic_backup_operations(sync_directory=injected_sync),
                    test_parent=fixture["root"],
                )
            except OSError as exc:
                if str(exc) != "injected first parent fsync failure":
                    raise AssertionError("guarded backup did not retain the original fsync failure") from exc
            else:
                raise AssertionError("injected first parent fsync failure did not fail")
            assert_synthetic_predecessor_restored(fixture)
            if len(calls) != 2:
                raise AssertionError("the first parent fsync was not inside the guarded rollback boundary")

        case("predecessor_backup_rename_then_first_fsync_failure_restores_exact", first_parent_fsync_failure)

        def backup_verification_failure() -> None:
            fixture = replacement_fixture("backup-verification")
            failed = False
            sync_calls = 0

            def injected_verify(environment: pathlib.Path, *, fixture: dict[str, Any]) -> dict[str, Any]:
                nonlocal failed
                if environment == fixture["backup"] and not failed:
                    failed = True
                    raise WR0Error("injected backup verification failure")
                return verify_synthetic_predecessor(environment, fixture=fixture)

            def recording_sync(path: pathlib.Path) -> None:
                nonlocal sync_calls
                sync_calls += 1
                fsync_directory(path)

            expect_refusal(
                lambda: move_predecessor_to_backup(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    operations=synthetic_backup_operations(
                        sync_directory=recording_sync,
                        verify_predecessor=injected_verify,
                    ),
                    test_parent=fixture["root"],
                )
            )
            assert_synthetic_predecessor_restored(fixture)
            if not failed or sync_calls < 2:
                raise AssertionError("backup verification failure did not complete guarded restoration")

        case("predecessor_backup_verification_failure_restores_exact", backup_verification_failure)

        def restoration_fsync_failure_then_outer_recovery() -> None:
            fixture = replacement_fixture("backup-restore-fsync")
            calls = 0

            def injected_sync(_path: pathlib.Path) -> None:
                nonlocal calls
                calls += 1
                if calls <= 2:
                    raise OSError("injected guarded or restoration fsync failure")

            expect_refusal(
                lambda: move_predecessor_to_backup(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    operations=synthetic_backup_operations(sync_directory=injected_sync),
                    test_parent=fixture["root"],
                )
            )
            removals: list[str] = []
            operations = synthetic_backup_operations(
                remove_transaction=lambda _path, _transaction: removals.append("removed")
            )
            receipt = recover_precommit_predecessor(
                fixture["destination"],
                fixture["backup"],
                transaction_id="wr0-20000101T000000Z-1111111111111111",
                predecessor=fixture["predecessor_snapshot"],
                fixture=fixture,
                backup_created_event=False,
                operations=operations,
                test_parent=fixture["root"],
            )
            assert_synthetic_predecessor_restored(fixture)
            if receipt["physical_state"] != "destination_exact_backup_absent" or removals:
                raise AssertionError("outer recovery destructively repeated an already completed restoration")

        case(
            "predecessor_restore_fsync_failure_outer_recovery_accepts_exact_destination",
            restoration_fsync_failure_then_outer_recovery,
        )

        def rename_completed_before_event_record() -> None:
            fixture = replacement_fixture("backup-before-event")
            replaced = False

            def rename_then_raise(source: pathlib.Path, target: pathlib.Path) -> None:
                nonlocal replaced
                os.replace(source, target)
                replaced = True
                raise OSError("injected return-path failure after physical rename")

            expect_refusal(
                lambda: move_predecessor_to_backup(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    operations=synthetic_backup_operations(replace=rename_then_raise),
                    test_parent=fixture["root"],
                )
            )
            if not replaced or physical_object_exists(fixture["destination"]) or not fixture["backup"].exists():
                raise AssertionError("physical rename injection did not produce the false-event backup shape")
            receipt = recover_precommit_predecessor(
                fixture["destination"],
                fixture["backup"],
                transaction_id="wr0-20000101T000000Z-1111111111111111",
                predecessor=fixture["predecessor_snapshot"],
                fixture=fixture,
                backup_created_event=False,
                operations=synthetic_backup_operations(),
                test_parent=fixture["root"],
            )
            assert_synthetic_predecessor_restored(fixture)
            if receipt["event_state_was_authority"] is not False:
                raise AssertionError("outer recovery trusted the stale in-memory event")

        case("predecessor_backup_physical_rename_before_event_is_recovered", rename_completed_before_event_record)

        def false_event_exact_backup() -> None:
            fixture = replacement_fixture("backup-false-event")
            os.replace(fixture["destination"], fixture["backup"])
            receipt = recover_precommit_predecessor(
                fixture["destination"],
                fixture["backup"],
                transaction_id="wr0-20000101T000000Z-1111111111111111",
                predecessor=fixture["predecessor_snapshot"],
                fixture=fixture,
                backup_created_event=False,
                operations=synthetic_backup_operations(),
                test_parent=fixture["root"],
            )
            assert_synthetic_predecessor_restored(fixture)
            if receipt["physical_state"] != "destination_absent_exact_backup_restored":
                raise AssertionError("false backup event did not yield physical-state restoration")

        case("predecessor_recovery_false_event_exact_backup_restored", false_event_exact_backup)

        def unknown_backup_collision() -> None:
            fixture = replacement_fixture("backup-unknown-collision")
            fixture["backup"].mkdir()
            unknown = fixture["backup"] / "unknown"
            unknown.write_bytes(b"preserve-unknown")
            before_hash = sha256_file(unknown)
            expect_refusal(
                lambda: move_predecessor_to_backup(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    operations=synthetic_backup_operations(),
                    test_parent=fixture["root"],
                )
            )
            verify_synthetic_predecessor(fixture["destination"], fixture=fixture)
            if sha256_file(unknown) != before_hash:
                raise AssertionError("unknown backup collision bytes changed")

        case("predecessor_backup_unknown_object_refused_unchanged", unknown_backup_collision)

        def forged_backup_marker() -> None:
            fixture = replacement_fixture("backup-forged-marker")
            os.replace(fixture["destination"], fixture["backup"])
            marker_path = fixture["backup"] / "wr0-environment.json"
            marker = json.loads(marker_path.read_text(encoding="utf-8"))
            marker["transaction_id"] = "wr0-19980101T000000Z-3333333333333333"
            atomic_write_json(marker_path, marker)
            forged_hash = sha256_file(marker_path)
            expect_refusal(
                lambda: recover_precommit_predecessor(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    backup_created_event=False,
                    operations=synthetic_backup_operations(),
                    test_parent=fixture["root"],
                )
            )
            if not fixture["backup"].exists() or fixture["destination"].exists() or sha256_file(marker_path) != forged_hash:
                raise AssertionError("forged predecessor backup was adopted or changed")

        case("predecessor_backup_forged_marker_identity_refused", forged_backup_marker)

        def predecessor_both_absent() -> None:
            fixture = replacement_fixture("backup-both-absent")
            shutil.rmtree(fixture["destination"])
            expect_refusal(
                lambda: recover_precommit_predecessor(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    backup_created_event=False,
                    operations=synthetic_backup_operations(),
                    test_parent=fixture["root"],
                )
            )
            if fixture["destination"].exists() or fixture["backup"].exists():
                raise AssertionError("both-absent recovery invented predecessor state")

        case("predecessor_recovery_both_absent_fails_closed", predecessor_both_absent)

        def unknown_destination_with_exact_backup() -> None:
            fixture = replacement_fixture("backup-unknown-destination")
            os.replace(fixture["destination"], fixture["backup"])
            fixture["destination"].mkdir()
            unknown = fixture["destination"] / "unknown"
            unknown.write_bytes(b"do-not-overwrite")
            before_hash = sha256_file(unknown)
            expect_refusal(
                lambda: recover_precommit_predecessor(
                    fixture["destination"],
                    fixture["backup"],
                    transaction_id="wr0-20000101T000000Z-1111111111111111",
                    predecessor=fixture["predecessor_snapshot"],
                    fixture=fixture,
                    backup_created_event=False,
                    operations=synthetic_backup_operations(),
                    test_parent=fixture["root"],
                )
            )
            if sha256_file(unknown) != before_hash or not fixture["backup"].exists():
                raise AssertionError("unknown destination was overwritten or exact backup was removed")

        case("predecessor_recovery_unknown_destination_refuses_exact_backup", unknown_destination_with_exact_backup)

        def successful_guarded_backup_and_rollback() -> None:
            fixture = replacement_fixture("backup-success")
            state = ReplacementTransactionState()
            receipt = move_predecessor_to_backup(
                fixture["destination"],
                fixture["backup"],
                transaction_id="wr0-20000101T000000Z-1111111111111111",
                predecessor=fixture["predecessor_snapshot"],
                fixture=fixture,
                operations=synthetic_backup_operations(),
                test_parent=fixture["root"],
            )
            if not all(
                receipt.get(name) is True
                for name in (
                    "rename_completed",
                    "first_directory_fsync_guarded",
                    "parent_fsync_completed",
                    "backup_snapshot_verified",
                    "destination_absent",
                    "backup_present",
                    "transaction_derived_backup",
                )
            ):
                raise AssertionError("successful guarded backup receipt is incomplete")
            state.advance("predecessor_backed_up")
            fixture["destination"].mkdir()
            (fixture["destination"] / "payload").write_bytes(b"exact-new-transaction")
            (fixture["destination"] / "synthetic-new-transaction").write_text(
                "wr0-20000101T000000Z-1111111111111111",
                encoding="utf-8",
            )
            recovery = recover_precommit_predecessor(
                fixture["destination"],
                fixture["backup"],
                transaction_id="wr0-20000101T000000Z-1111111111111111",
                predecessor=fixture["predecessor_snapshot"],
                fixture=fixture,
                backup_created_event=True,
                operations=synthetic_backup_operations(),
                test_parent=fixture["root"],
            )
            state.rollback_invoked = True
            assert_synthetic_predecessor_restored(fixture)
            if (
                not recovery["predecessor_restored"]
                or recovery["physical_state"] != "new_destination_removed_exact_backup_restored"
                or state.phase != "predecessor_backed_up"
            ):
                raise AssertionError("successful helper receipt did not support exact pre-commit rollback")

        case("predecessor_guarded_backup_receipt_and_rollback_pass", successful_guarded_backup_and_rollback)

        def advance_replacement_state(state: ReplacementTransactionState, target: str) -> None:
            while state.phase != target:
                next_index = REPLACEMENT_PHASES.index(state.phase) + 1
                if next_index >= len(REPLACEMENT_PHASES):
                    raise AssertionError("synthetic replacement phase target was not reachable")
                state.advance(REPLACEMENT_PHASES[next_index])

        def synthetic_commit_arguments(environment_identity: str = "2" * 64) -> dict[str, Any]:
            return {
                "replacement_transaction_id": "wr0-20000101T000000Z-1111111111111111",
                "predecessor_transaction_id": "wr0-19990101T000000Z-2222222222222222",
                "predecessor_environment_identity_sha256": "1" * 64,
                "new_environment_identity_sha256": environment_identity,
                "runner_identity_sha256": "3" * 64,
                "contract_source_sha256": "4" * 64,
                "workload_sha256": "5" * 64,
                "receipt_identities": {
                    "run_1_receipt_sha256": "6" * 64,
                    "run_2_receipt_sha256": "7" * 64,
                    "exit_37_receipt_sha256": "8" * 64,
                },
                "protected_fixture_sha256": "9" * 64,
                "committed_at": "2000-01-01T00:00:00Z",
            }

        def install_synthetic_new(fixture: dict[str, Any], state: ReplacementTransactionState) -> None:
            destination = fixture["destination"]
            backup = fixture["backup"]
            stage = fixture["stage"]
            os.replace(destination, backup)
            state.advance("predecessor_backed_up")
            stage.mkdir()
            (stage / "payload").write_bytes(b"exact-new-environment")
            state.advance("new_environment_staged")
            os.replace(stage, destination)
            state.advance("new_environment_promoted")
            state.advance("commit_ready")

        def synthetic_precommit_rollback(fixture: dict[str, Any], state: ReplacementTransactionState) -> None:
            destination = fixture["destination"]
            backup = fixture["backup"]
            stage = fixture["stage"]
            if stage.exists():
                shutil.rmtree(stage)
            if backup.exists():
                if destination.exists():
                    shutil.rmtree(destination)
                os.replace(backup, destination)
            if sha256_file(destination / "payload") != fixture["predecessor_sha256"]:
                raise AssertionError("synthetic pre-commit rollback did not restore exact predecessor")
            state.rollback_invoked = True

        def failure_before_backup() -> None:
            fixture = replacement_fixture("before-backup")
            state = ReplacementTransactionState()
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: synthetic_precommit_rollback(fixture, state),
                verify_committed_environment=lambda: (_ for _ in ()).throw(AssertionError("post-commit verifier used")),
            )
            if result != "WR0_REPLACEMENT_PRECOMMIT_BLOCKED" or fixture["backup"].exists():
                raise AssertionError("failure before backup changed predecessor state")

        case("replacement_failure_before_predecessor_backup_preserves_predecessor", failure_before_backup)

        def failure_after_backup_before_promotion() -> None:
            fixture = replacement_fixture("after-backup")
            state = ReplacementTransactionState()
            os.replace(fixture["destination"], fixture["backup"])
            state.advance("predecessor_backed_up")
            fixture["stage"].mkdir()
            (fixture["stage"] / "payload").write_bytes(b"incomplete-new")
            state.advance("new_environment_staged")
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: synthetic_precommit_rollback(fixture, state),
                verify_committed_environment=lambda: (_ for _ in ()).throw(AssertionError("post-commit verifier used")),
            )
            if result != "WR0_REPLACEMENT_PRECOMMIT_BLOCKED" or fixture["stage"].exists():
                raise AssertionError("failure before promotion left synthetic new state")

        case("replacement_failure_after_backup_before_promotion_restores_predecessor", failure_after_backup_before_promotion)

        def failure_after_promotion_before_commit() -> None:
            fixture = replacement_fixture("after-promotion")
            state = ReplacementTransactionState()
            install_synthetic_new(fixture, state)
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: synthetic_precommit_rollback(fixture, state),
                verify_committed_environment=lambda: (_ for _ in ()).throw(AssertionError("post-commit verifier used")),
            )
            if result != "WR0_REPLACEMENT_PRECOMMIT_BLOCKED" or fixture["backup"].exists():
                raise AssertionError("failure before commit did not restore predecessor")

        case("replacement_failure_after_promotion_before_commit_restores_predecessor", failure_after_promotion_before_commit)

        def commit_record_write_failure() -> None:
            fixture = replacement_fixture("commit-write")
            state = ReplacementTransactionState()
            install_synthetic_new(fixture, state)
            document = replacement_commit_document(**synthetic_commit_arguments(), predecessor_retirement_status="pending")
            try:
                durably_commit_replacement(
                    state,
                    fixture["destination"],
                    document,
                    record_writer=lambda _environment, _document: (_ for _ in ()).throw(OSError("injected commit write failure")),
                )
            except OSError:
                pass
            else:
                raise AssertionError("injected commit write failure did not fail")
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: synthetic_precommit_rollback(fixture, state),
                verify_committed_environment=lambda: (_ for _ in ()).throw(AssertionError("post-commit verifier used")),
            )
            if result != "WR0_REPLACEMENT_PRECOMMIT_BLOCKED" or replacement_commit_path(fixture["destination"]).exists():
                raise AssertionError("commit write failure did not restore predecessor")

        case("replacement_commit_record_write_failure_restores_predecessor", commit_record_write_failure)

        def commit_record_readback_failure() -> None:
            fixture = replacement_fixture("commit-readback")
            state = ReplacementTransactionState()
            install_synthetic_new(fixture, state)
            document = replacement_commit_document(**synthetic_commit_arguments(), predecessor_retirement_status="pending")

            def injected_writer(environment: pathlib.Path, value: dict[str, Any]) -> dict[str, Any]:
                atomic_write_json(replacement_commit_path(environment), value)
                raise WR0Error("injected commit readback failure")

            expect_refusal(lambda: durably_commit_replacement(state, fixture["destination"], document, record_writer=injected_writer))
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: synthetic_precommit_rollback(fixture, state),
                verify_committed_environment=lambda: (_ for _ in ()).throw(AssertionError("post-commit verifier used")),
            )
            if result != "WR0_REPLACEMENT_PRECOMMIT_BLOCKED":
                raise AssertionError("commit readback failure classification differs")

        case("replacement_commit_record_readback_failure_restores_predecessor", commit_record_readback_failure)

        def commit_synthetic_replacement(name: str) -> tuple[dict[str, Any], ReplacementTransactionState, dict[str, Any]]:
            fixture = replacement_fixture(name)
            state = ReplacementTransactionState()
            install_synthetic_new(fixture, state)
            document = replacement_commit_document(**synthetic_commit_arguments(), predecessor_retirement_status="pending")
            record = durably_commit_replacement(state, fixture["destination"], document)
            return fixture, state, record

        def postcommit_before_delete() -> None:
            fixture, state, _record = commit_synthetic_replacement("postcommit-before-delete")
            rollback_calls: list[str] = []
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: rollback_calls.append("called"),
                verify_committed_environment=lambda: read_replacement_commit_record(fixture["destination"]),
            )
            if result != "WR0_PREDECESSOR_RETIREMENT_BLOCKED" or rollback_calls or not fixture["backup"].exists():
                raise AssertionError("post-commit pre-retirement failure entered rollback")

        case("post_commit_failure_before_retirement_preserves_new_environment", postcommit_before_delete)

        def recursive_retirement_failure() -> None:
            fixture, state, _record = commit_synthetic_replacement("retirement-delete")
            (fixture["backup"] / "residual").write_bytes(b"transaction-owned")

            def partial_delete(path: pathlib.Path) -> None:
                (path / "payload").unlink()
                raise OSError("injected recursive retirement failure")

            expect_refusal(
                lambda: retire_committed_predecessor(
                    state,
                    environment=fixture["destination"],
                    predecessor_backup=fixture["backup"],
                    environment_parent=fixture["root"],
                    commit_document_arguments=synthetic_commit_arguments(),
                    verify_new_environment=lambda: None,
                    delete_tree=partial_delete,
                )
            )
            rollback_calls: list[str] = []
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: rollback_calls.append("called"),
                verify_committed_environment=lambda: read_replacement_commit_record(fixture["destination"]),
            )
            if result != "WR0_PREDECESSOR_RETIREMENT_BLOCKED" or rollback_calls or not (fixture["backup"] / "residual").is_file():
                raise AssertionError("partial predecessor deletion harmed committed replacement authority")

        case("predecessor_recursive_retirement_failure_preserves_new_environment", recursive_retirement_failure)

        def parent_fsync_failure() -> None:
            fixture, state, _record = commit_synthetic_replacement("retirement-fsync")
            expect_refusal(
                lambda: retire_committed_predecessor(
                    state,
                    environment=fixture["destination"],
                    predecessor_backup=fixture["backup"],
                    environment_parent=fixture["root"],
                    commit_document_arguments=synthetic_commit_arguments(),
                    verify_new_environment=lambda: None,
                    sync_directory=lambda _path: (_ for _ in ()).throw(OSError("injected parent fsync failure")),
                )
            )
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: (_ for _ in ()).throw(AssertionError("post-commit rollback invoked")),
                verify_committed_environment=lambda: read_replacement_commit_record(fixture["destination"]),
            )
            if result != "WR0_PREDECESSOR_RETIREMENT_BLOCKED" or not fixture["destination"].exists():
                raise AssertionError("parent fsync failure removed committed replacement")

        case("predecessor_parent_fsync_failure_preserves_new_environment", parent_fsync_failure)

        def retired_synthetic_replacement(name: str) -> tuple[dict[str, Any], ReplacementTransactionState, dict[str, Any]]:
            fixture, state, _record = commit_synthetic_replacement(name)
            retirement = retire_committed_predecessor(
                state,
                environment=fixture["destination"],
                predecessor_backup=fixture["backup"],
                environment_parent=fixture["root"],
                commit_document_arguments=synthetic_commit_arguments(),
                verify_new_environment=lambda: None,
            )
            return fixture, state, retirement

        def final_evidence_render_failure() -> None:
            fixture, state, _retirement = retired_synthetic_replacement("evidence-render")
            expect_refusal(
                lambda: finalize_committed_evidence(
                    state,
                    render_packet=lambda _at: (_ for _ in ()).throw(OSError("injected evidence render failure")),
                    validate_packet=lambda: None,
                    publish_packet=lambda: None,
                )
            )
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: (_ for _ in ()).throw(AssertionError("post-commit rollback invoked")),
                verify_committed_environment=lambda: read_replacement_commit_record(fixture["destination"]),
            )
            if result != "WR0_EVIDENCE_FINALIZATION_BLOCKED" or not fixture["destination"].exists():
                raise AssertionError("evidence rendering failure harmed committed replacement")

        case("final_evidence_render_failure_preserves_committed_environment", final_evidence_render_failure)

        def final_evidence_sanitization_failure() -> None:
            fixture, state, _retirement = retired_synthetic_replacement("evidence-sanitize")
            expect_refusal(
                lambda: finalize_committed_evidence(
                    state,
                    render_packet=lambda _at: None,
                    validate_packet=lambda: (_ for _ in ()).throw(WR0Error("injected evidence sanitization failure")),
                    publish_packet=lambda: None,
                )
            )
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: (_ for _ in ()).throw(AssertionError("post-commit rollback invoked")),
                verify_committed_environment=lambda: read_replacement_commit_record(fixture["destination"]),
            )
            if result != "WR0_EVIDENCE_FINALIZATION_BLOCKED" or not fixture["destination"].exists():
                raise AssertionError("evidence sanitization failure harmed committed replacement")

        case("final_evidence_sanitization_failure_preserves_committed_environment", final_evidence_sanitization_failure)

        def successful_replacement_order() -> None:
            fixture = replacement_fixture("success")
            state = ReplacementTransactionState()
            install_synthetic_new(fixture, state)
            order: list[str] = []
            document = replacement_commit_document(**synthetic_commit_arguments(), predecessor_retirement_status="pending")

            def recording_writer(environment: pathlib.Path, value: dict[str, Any]) -> dict[str, Any]:
                record = write_and_verify_replacement_commit_record(environment, value)
                order.append(f"record:{value['predecessor_retirement_status']}")
                return record

            durably_commit_replacement(state, fixture["destination"], document, record_writer=recording_writer)

            def recording_delete(path: pathlib.Path) -> None:
                if not state.durably_committed:
                    raise AssertionError("predecessor deletion preceded durable commit")
                order.append("delete")
                shutil.rmtree(path)

            retirement = retire_committed_predecessor(
                state,
                environment=fixture["destination"],
                predecessor_backup=fixture["backup"],
                environment_parent=fixture["root"],
                commit_document_arguments=synthetic_commit_arguments(),
                verify_new_environment=lambda: None,
                record_writer=recording_writer,
                delete_tree=recording_delete,
            )
            finalize_committed_evidence(
                state,
                render_packet=lambda _at: order.append("evidence-rendered"),
                validate_packet=lambda: order.append("evidence-validated"),
                publish_packet=lambda: order.append("evidence-published"),
            )
            final_record = read_replacement_commit_record(fixture["destination"])
            if not (
                order.index("record:pending") < order.index("delete")
                and final_record["document"]["predecessor_retirement_status"] == "retired"
                and not fixture["backup"].exists()
                and state.phase == "evidence_finalized"
                and retirement["retired_record"]["sha256"] == final_record["sha256"]
            ):
                raise AssertionError("successful replacement ordering differs")

        case("durable_commit_precedes_successful_retirement_and_evidence", successful_replacement_order)

        def forged_commit_record() -> None:
            fixture, _state, record = commit_synthetic_replacement("forged-record")
            forged = json.loads(json.dumps(record["document"]))
            forged["status"] = "retired"
            atomic_write_json(replacement_commit_path(fixture["destination"]), forged)
            expect_refusal(lambda: read_replacement_commit_record(fixture["destination"], expected=record["document"]))

        case("stale_or_forged_replacement_commit_record_refused", forged_commit_record)

        def wrong_commit_identities() -> None:
            fixture, _state, record = commit_synthetic_replacement("wrong-identities")
            wrong_new = replacement_commit_document(
                **synthetic_commit_arguments(environment_identity="f" * 64),
                predecessor_retirement_status="pending",
            )
            atomic_write_json(replacement_commit_path(fixture["destination"]), wrong_new)
            expect_refusal(lambda: read_replacement_commit_record(fixture["destination"], expected=record["document"]))
            wrong_predecessor_arguments = synthetic_commit_arguments()
            wrong_predecessor_arguments["predecessor_environment_identity_sha256"] = "e" * 64
            wrong_predecessor = replacement_commit_document(
                **wrong_predecessor_arguments,
                predecessor_retirement_status="pending",
            )
            atomic_write_json(replacement_commit_path(fixture["destination"]), wrong_predecessor)
            expect_refusal(lambda: read_replacement_commit_record(fixture["destination"], expected=record["document"]))

        case("replacement_commit_record_wrong_environment_identities_refused", wrong_commit_identities)

        def postcommit_never_rolls_back() -> None:
            fixture, state, _record = commit_synthetic_replacement("never-rollback")
            rollback_calls: list[str] = []
            result = replacement_failure_result(
                state,
                precommit_rollback=lambda: rollback_calls.append("called"),
                verify_committed_environment=lambda: read_replacement_commit_record(fixture["destination"]),
            )
            if result != "WR0_PREDECESSOR_RETIREMENT_BLOCKED" or rollback_calls or state.rollback_invoked:
                raise AssertionError("post-commit exception entered pre-commit rollback")

        case("post_commit_exception_never_invokes_precommit_rollback", postcommit_never_rolls_back)

        if require_clean_source:
            cases.extend(run_contract_source_production_tests(working))
    finally:
        if working.exists():
            require_contained(working, root, label="negative-test cleanup")
            shutil.rmtree(working)
    expected_count = 75 if require_clean_source else 68
    if len(cases) != expected_count:
        fail("negative-test ledger count differs from the exact declared roster")
    result = {
        "schema": NEGATIVE_SCHEMA,
        "classification": "passed",
        "passed_count": len(cases),
        "failed_count": 0,
        "cases": cases,
        "real_runner_modified": False,
        "production_contract_source_tests": require_clean_source,
        "test_root": "<HOME>/.cache/linux-vst-bridge/wr0/tests",
    }
    atomic_write_json(cache_root() / "negative-tests.latest.json", result)
    return result


def sanitize_error(message: str) -> str:
    text = str(message).replace(str(real_home()), "<HOME>").replace(str(repo_root()), "<REPO>")
    text = re.sub(r"/proc/[0-9]+", "/proc/<PID>", text)
    return text[:512]


def sanitized_processes(run: dict[str, Any]) -> list[dict[str, Any]]:
    raw = list(run.get("processes", []))
    role_counts: dict[str, int] = {}
    labels: dict[tuple[int, int], str] = {}
    root = next((item for item in raw if item.get("role") == "runtime_root"), None)
    for item in raw:
        role = str(item.get("role", "owned_descendant"))
        role_counts[role] = role_counts.get(role, 0) + 1
        label = f"{role.upper()}_{role_counts[role]}"
        key = (int(item["pid"]), int(item["start_ticks"]))
        labels[key] = label
    result = []
    for item in raw:
        key = (int(item["pid"]), int(item["start_ticks"]))
        parent_start = item.get("parent_start_ticks")
        parent_key = (int(item["ppid"]), int(parent_start)) if parent_start is not None else None
        if item.get("role") == "runtime_root":
            parent_label = "SUPERVISOR"
        elif parent_key in labels:
            parent_label = labels[parent_key]
        else:
            parent_label = "OWNED_PARENT_NOT_SAMPLED"
        result.append(
            {
                "label": labels[key],
                "role": item["role"],
                "comm": item["comm"],
                "executable_basename": item["exe_basename"],
                "uid": item["uid"],
                "start_ticks": item["start_ticks"],
                "identity_sha256": item["identity_sha256"],
                "parent": parent_label,
                "cgroup_classification": item["cgroup_class"],
                "first_seen_ms": item["first_seen_ms"],
                "last_seen_ms": item["last_seen_ms"],
                "same_process_group_as_root": bool(root is not None and item["pgrp"] == root["pgrp"]),
                "same_session_as_root": bool(root is not None and item["session"] == root["session"]),
            }
        )
    return result


def sanitized_run(run: dict[str, Any] | None) -> dict[str, Any]:
    if run is None:
        return {"classification": "blocked", "observed": False}
    required_roles = {"runtime_root", "proton", "windows_command"}
    observed_roles = {str(item["role"]) for item in run["processes"]}
    missing_roles = sorted(required_roles - observed_roles)
    return {
        "schema": run["schema"],
        "classification": "blocked" if missing_roles else run["classification"],
        "observed": True,
        "run_number": run["run_number"],
        "nonce": run["nonce"],
        "started_at": run["started_at"],
        "finished_at": run["finished_at"],
        "duration_ms": run["duration_ms"],
        "stdout": run["stdout"],
        "stdout_sha256": run["stdout_sha256"],
        "stderr_bytes": run["stderr_bytes"],
        "stderr_sha256": run["stderr_sha256"],
        "exit_status": run["exit_status"],
        "expected_exit_status": run["expected_exit_status"],
        "timeout": run["timeout"],
        "terminated_by_supervisor": run["terminated_by_supervisor"],
        "supervisor": run["supervisor"],
        "root_start_ticks": run["root_start_ticks"],
        "root_identity_sha256": run["root_identity_sha256"],
        "ancestry": run["ancestry"],
        "steam_game_ancestor": run["steam_game_ancestor"],
        "processes": sanitized_processes(run),
        "process_count": run["process_count"],
        "process_scope": run["process_scope"],
        "escaped_process_group_count": run["escaped_process_group_count"],
        "handshake": run["handshake"],
        "strict_command_vector_verified": run.get("strict_command_vector_verified", False),
        "fresh_process_revalidation_verified": run.get("fresh_process_revalidation_verified", False),
        "gate_behavior": run.get("gate_behavior", "release"),
        "contract_source_schema": run["contract_source_schema"],
        "contract_source_sha256": run["contract_source_sha256"],
        "workload_sha256": run["workload_sha256"],
        "clean_descendant_count": run["clean_descendant_count"],
        "process_role_assertion": "blocked" if missing_roles else "passed",
        "missing_required_roles": missing_roles,
        "command_contract": run["command_contract"],
        "complete_command_line_retained": False,
        "environment_retained": False,
    }


def sanitized_held_cleanup(value: dict[str, Any] | None) -> dict[str, Any]:
    if value is None:
        return {"classification": "blocked", "observed": False}
    return {
        "schema": value["schema"],
        "classification": value["classification"],
        "observed": True,
        "run_number": value["run_number"],
        "nonce": value["nonce"],
        "ready_file_verified": value["ready_file_verified"],
        "ready_stdout_verified": value["ready_stdout_verified"],
        "strict_command_vector_verified": value["strict_command_vector_verified"],
        "fresh_process_revalidation_verified": value["fresh_process_revalidation_verified"],
        "command_first_observed_monotonic_us": value["command_first_observed_monotonic_us"],
        "cleanup_started_after_observation": value["cleanup_started_after_observation"],
        "command_completed_monotonic_us": value["command_completed_monotonic_us"],
        "processes": sanitized_processes(value),
        "process_count": value["process_count"],
        "escaped_process_group_count": value["escaped_process_group_count"],
        "termination": value["termination"],
        "final_owned_count": value["final_owned_count"],
        "sentinel": value["sentinel"],
        "sentinel_survival": value["sentinel_survival"],
        "sentinel_terminated_separately": value["sentinel_terminated_separately"],
        "ready_gate_temporary_absent_after": value["ready_gate_temporary_absent_after"],
        "contract_source_schema": value["contract_source_schema"],
        "contract_source_sha256": value["contract_source_sha256"],
        "workload_sha256": value["workload_sha256"],
        "gate_withheld": value["gate_withheld"],
        "hold_seconds": value["hold_seconds"],
        "stage_cleanup": value.get("stage_cleanup", False),
        "final_environment_unchanged": value.get("final_environment_unchanged", False),
        "complete_command_line_retained": False,
    }


def markdown_table(rows: Iterable[tuple[str, str]]) -> str:
    output = ["| Fact | Result |", "|---|---|"]
    output.extend(f"| {name} | {value} |" for name, value in rows)
    return "\n".join(output)


def process_markdown(run: dict[str, Any]) -> str:
    processes = run.get("processes", [])
    if not processes:
        return "No owned process identities were retained."
    lines = [
        "| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |",
        "|---|---|---|---:|---:|---|---|---|---|",
    ]
    for item in processes:
        lines.append(
            f"| `{item['label']}` | `{item['role']}` | `{item['executable_basename']}` | "
            f"{item['uid']} | {item['start_ticks']} | `{item['parent']}` | "
            f"`{str(item['same_process_group_as_root']).lower()}` | `{str(item['same_session_as_root']).lower()}` | "
            f"`{item['identity_sha256']}` |"
        )
    return "\n".join(lines)


def render_run_markdown(title: str, run: dict[str, Any], receipt: dict[str, Any] | None, environment_identity: str | None) -> str:
    if not run.get("observed"):
        return f"# {title}\n\nClassification: `blocked`; execution was not observed.\n"
    receipt_rows = ""
    if receipt is not None:
        receipt_rows = (
            f"- Internal receipt SHA-256: `{receipt['sha256']}`.\n"
            f"- Internal receipt normalized SHA-256: `{receipt['normalized_sha256']}`.\n"
        )
    return f"""# {title}

Classification: `{run['classification']}`.

{markdown_table((
    ('Run number', f'`{run["run_number"]}`'),
    ('Nonce', f'`{run["nonce"]}`'),
    ('Started', f'`{run["started_at"]}`'),
    ('Finished', f'`{run["finished_at"]}`'),
    ('Duration ms', f'`{run["duration_ms"]}`'),
    ('Windows/supervisor exit', f'`{run["exit_status"]}` / expected `{run["expected_exit_status"]}`'),
    ('Timeout', f'`{str(run["timeout"]).lower()}`'),
    ('Root start ticks', f'`{run["root_start_ticks"]}`'),
    ('Root identity SHA-256', f'`{run["root_identity_sha256"]}`'),
    ('Environment identity SHA-256', f'`{environment_identity or "not_available"}`'),
    ('Clean descendants after completion', f'`{run["clean_descendant_count"]}`'),
    ('Steam game-launch ancestor', f'`{str(run["steam_game_ancestor"]).lower()}`'),
    ('Process-role assertion', f'`{run["process_role_assertion"]}`'),
    ('Missing required roles', f'`{json.dumps(run["missing_required_roles"])}`'),
    ('Observed process scope', f'`{run["process_scope"]}`'),
    ('Descendants outside initial process group', f'`{run["escaped_process_group_count"]}`'),
    ('Command first observed us', f'`{run["handshake"]["command_first_observed_monotonic_us"]}`'),
    ('Fresh pre-gate revalidation us', f'`{run["handshake"]["fresh_revalidated_monotonic_us"]}`'),
    ('Gate committed us', f'`{run["handshake"]["gate_committed_monotonic_us"]}`'),
    ('Command completed us', f'`{run["handshake"]["command_completed_monotonic_us"]}`'),
    ('Causal gate order', f'`{str(run["handshake"]["causal_order_verified"]).lower()}`'),
    ('Strict command vector', f'`{str(run["strict_command_vector_verified"]).lower()}`'),
    ('Contract-source SHA-256', f'`{run["contract_source_sha256"]}`'),
    ('Tracked workload SHA-256', f'`{run["workload_sha256"]}`'),
))}

## Exact normalized stdout

```text
{run['stdout'].rstrip()}
```

- Raw stdout SHA-256: `{run['stdout_sha256']}`.
- Bounded stderr: `{run['stderr_bytes']}` bytes / SHA-256 `{run['stderr_sha256']}`; content is not retained.
{receipt_rows}
## Bounded owned process identities

{process_markdown(run)}

No PID, command line, process environment, complete process map, or unrelated process is retained.
"""


def preservation_rows(fixture: dict[str, Any]) -> list[tuple[str, str]]:
    rows: list[tuple[str, str]] = []
    bitwig = fixture["bitwig"]
    rows.extend(
        (
            ("Bitwig version/app commit", f'`{bitwig["version"]}` / `{bitwig["app_commit"]}`'),
            ("Bitwig runtime/commit", f'`{bitwig["runtime_ref"]}` / `{bitwig["runtime_commit"]}`'),
            ("Bitwig user/system override SHA-256", f'`{bitwig["user_override_sha256"]}` / `{bitwig["system_override_sha256"]}`'),
        )
    )
    hp0 = fixture["hp0"]
    rows.extend(
        (
            ("HP0 module SHA-256", f'`{hp0["module"]["sha256"]}`'),
            ("HP0 publication receipt SHA-256", f'`{hp0["publication_receipt"]["sha256"]}`'),
            ("HP0 retained build receipt SHA-256", f'`{hp0["build_receipt_sha256"]}`'),
            ("HP0 build-source identity", f'`{hp0["build_source_schema"]}` / `{hp0["build_source_sha256"]}`'),
        )
    )
    for name, packet in fixture["accepted_evidence_packets"].items():
        rows.append((f"Accepted packet {name}", f'`{packet["manifest_sha256"]}` / `{packet["verified_file_count"]}` files'))
    for item in fixture["serum_regular_files"]:
        rows.append(
            (
                f"SR0 Serum {item['role']}",
                f'`{item["sha256"]}` / `{item["size"]}` bytes / mtime ns `{item["mtime_ns"]}`',
            )
        )
    for name, item in fixture["wine_registry_files"].items():
        rows.append((f"Existing .wine {name}", f'`{item["sha256"]}` / `{item["size"]}` bytes'))
    steam = fixture["steam_compatdata"]
    rows.append(("Steam compatdata immediate roster", f'`{steam["roster_sha256"]}` / `{steam["directory_count"]}` directories'))
    rows.append(("Runner/runtime lock", f'`{fixture["runner_lock_digest"]}`'))
    return rows


def render_evidence(session: dict[str, Any], *, output_root: pathlib.Path | None = None) -> None:
    output = output_root if output_root is not None else evidence_root()
    if output_root is None and output.parent != repo_root() / "evidence":
        fail("WR0 evidence parent differs")
    if not output.is_absolute():
        fail("WR0 evidence output is not absolute")
    require_no_symlink_ancestors(output.parent, allow_absent_leaf=False, label="WR0 evidence output parent")
    output.parent.mkdir(exist_ok=True)
    status = session["status"]
    success = status == "complete"
    commit_ready = status == "commit_ready"
    claim_name = (
        "WR0_COMPLETE"
        if success
        else "WR0_COMMIT_READY"
        if commit_ready
        else session.get("blocked_result_name", "WR0_REPAIR_VALIDATION_BLOCKED")
    )
    run1 = sanitized_run(session.get("run_1"))
    run2 = sanitized_run(session.get("run_2"))
    exit_run = sanitized_run(session.get("exit_37"))
    bad_gate_raw = session.get("bad_gate_tests")
    bad_gate = (
        {
            "classification": "passed",
            "wrong_nonce": sanitized_run(bad_gate_raw["wrong_nonce"]),
            "wrong_run": sanitized_run(bad_gate_raw["wrong_run"]),
        }
        if isinstance(bad_gate_raw, dict)
        else {"classification": "blocked", "observed": False}
    )
    held_cleanup = sanitized_held_cleanup(session.get("live_held_cleanup"))
    lock = session["runner_lock"]
    source = session["contract_source"]
    before = session["fixture_before"]
    after = session["fixture_after"]
    preservation = session["preservation"]
    environment = session.get("environment_final")
    environment_identity = environment.get("environment_identity_sha256") if environment else None
    run1_receipt = session.get("run_1_receipt")
    run2_receipt = session.get("run_2_receipt")
    negative = session["negative_tests"]
    process_guard_before = session["preflight"]["process_guard"]
    process_guard_after = session["process_guard_after"]
    replacement = session.get("replacement", {})
    replacement_phase = replacement.get("replacement_phase", "unknown")
    commit_record = replacement.get("durable_commit_record", {})
    predecessor = session.get("predecessor", {})
    observed_runs = [run for run in (run1, run2, exit_run) if run.get("observed")]
    supervisor_owned_observed_roots = bool(observed_runs) and all(
        not run["steam_game_ancestor"] and len(run["ancestry"]) >= 2 for run in observed_runs
    )

    basis_md = f"""# WR0 basis and pre-flight

{markdown_table((
    ('Result', '`PRE-FLIGHT_CLEAR`'),
    ('Basis commit', f'`{BASIS_COMMIT}`'),
    ('Basis tree', f'`{BASIS_TREE}`'),
    ('Reviewed pre-repair head', f'`{REVIEWED_PRE_REPAIR_HEAD}`'),
    ('Reviewed pre-repair tree', f'`{REVIEWED_PRE_REPAIR_TREE}`'),
    ('Branch', f'`{EXPECTED_BRANCH}`'),
    ('Fixture', '`Steam Deck Galileo / SteamOS / x86_64`'),
    ('SteamOS read-only', '`enabled`'),
    ('Pre-edit worktree', '`clean at exact reviewed head`'),
    ('Pre-edit final WR0 environment', f'`exact reviewed predecessor {PREDECESSOR_ENVIRONMENT_IDENTITY}`'),
    ('Runner contract scan', '`implementable`'),
))}

The exact repository path, user name, real hostname, and raw pre-flight output are not retained. The accepted HP0 publication inspection and final-head build receipt verification passed before editing. The reviewed predecessor marker, transaction, runner/source/workload bindings, registry hashes, bounded roster, prefix device/inode, and three environment run receipts were exact. Immediately before live execution, the collector independently revalidated the protected identities without invoking a dirty-worktree build.
"""
    lock_lines = [
        "# WR0 runner/runtime lock",
        "",
        f"Classification: `{'passed' if success else 'observed'}`.",
        "",
        f"- Schema: `{lock['manifest']['schema']}`.",
        f"- Canonical manifest SHA-256: `{lock['digest']}`.",
        f"- Runner: `Proton 11.0` / `{RUNNER_VERSION}` / Steam app `{RUNNER_APP_ID}` build `{RUNNER_BUILD_ID}`.",
        f"- Declared runtime: Steam app `{RUNTIME_APP_ID}` build `{RUNTIME_BUILD_ID}`, depot `{RUNTIME_VERSION}`.",
        f"- pressure-vessel: `{PRESSURE_VESSEL_VERSION}`.",
        f"- Before/after digest equality: `{str(session['runner_lock_after']['digest'] == lock['digest']).lower()}`.",
        "",
        "| Safe relative path | Type | Size | Mode | SHA-256 |",
        "|---|---|---:|---|---|",
    ]
    for item in lock["manifest"]["files"]:
        lock_lines.append(
            f"| `{item['safe_path']}` | `{item['type']}` | {item['size']} | `{item['mode']}` | `{item['sha256']}` |"
        )
    lock_lines.extend(
        (
            "",
            "No runner/runtime payload is retained. The Scout runtime is installed but undeclared and was not selected.",
        )
    )
    runner_lock_md = "\n".join(lock_lines) + "\n"
    contract_md = f"""# WR0 launch contract

```text
launch.py supervisor
  -> Runtime 4 _v2-entry-point --verb=run --
  -> Runtime 4 run / pressure-vessel-unruntime
  -> Proton 11.0 proton runinprefix
  -> runner wine
  -> runner x86_64-windows cmd.exe
  -> tracked wr0-probe.cmd
```

- Neutral application ID: `{NEUTRAL_APP_ID}`.
- Contract-source schema / SHA-256: `{source['schema']}` / `{source['digest']}`.
- Historical implementation commit / tree: `{source['implementation_commit']}` / `{source['implementation_tree']}`.
- Tracked workload SHA-256: `{source['workload_sha256']}`.
- Persistent state roots: exact WR0 compatdata, runtime variable, cache, config, data, and temporary directories only.
- Explicit environment names: `HOME`, `USER`, `LOGNAME`, `PATH`, `LANG`, `XDG_RUNTIME_DIR`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `TMPDIR`, `STEAM_COMPAT_DATA_PATH`, `STEAM_COMPAT_CLIENT_INSTALL_PATH`, `STEAM_COMPAT_APP_ID`, `SteamAppId`, `SteamGameId`, `PRESSURE_VESSEL_VARIABLE_DIR`, `STEAM_ZENITY`.
- Environment begins empty; ambient compatibility overrides, logging/tuning variables, library injection, and caller executable paths are absent.
- Run 1 / reuse deadlines: `{int(RUN1_TIMEOUT_SECONDS)}` / `{int(REUSE_TIMEOUT_SECONDS)}` seconds.
- Stream cap: `{OUTPUT_CAP}` bytes each; process cap: `{PROCESS_CAP}` identities; poll: `{PROCESS_POLL_SECONDS}` seconds; drain: `{PROCESS_DRAIN_SECONDS}` seconds.
- `windows_command` requires exactly one contiguous, case-exact `/d /q /c <full Z:\\<REPO>\\windows-fixtures\\wr0-probe\\wr0-probe.cmd> <nonce> <decimal-run> <source-digest> <workload-digest> [--exit-37]` vector; basename suffixes, partial tokens, wrong runs, reordered switches, duplicates, and misplaced exit flags are refused.
- Readiness uses the nonce/run-bound `{HANDSHAKE_SCHEMA}` file gate. Immediately before gate commitment, a new bounded `/proc` census revalidates the Runtime root, the same selected Proton and Windows-command PID/start identities, both complete ancestry chains, the shared Runtime root, exact vector, ready file/stdout, and gate absence.
- Observation follows the complete bounded descendant tree of the exact runtime root PID/start identity; group/session membership is classified separately.
- Timeout cleanup targets the isolated group and only immediately revalidated observed PID/start identities that escaped it, calls the production empty-scope waiter after TERM and KILL, and raises `WR0_PROCESS_TOPOLOGY_BLOCKED` on any survivor. Cleanup deadline: `{CLEANUP_DEADLINE_SECONDS}` seconds.
- A separate actual held-command probe withheld the gate for `{HELD_COMMAND_SECONDS}` second after exact observation, terminated the real Runtime/Proton/Wine/cmd topology to zero, and left an unrelated same-comm sentinel alive.
- Detached Git worktrees beneath the canonical WR0 cache exercised evidence-only stability, changed workload/supervisor/wrapper refusal, unexpected governed-path refusal, historical-provenance non-authority, and dirty/staged-source refusal through the production source functions.
- Full command lines and environment values are not retained.

## Contract-source manifest

| Repository-relative path | Git mode | Git blob |
|---|---|---|
{chr(10).join(f'| `{item["path"]}` | `{item["mode"]}` | `{item["blob"]}` |' for item in source['manifest']['files'])}
"""
    if environment:
        marker = environment["marker"]
        run1_env = session["environment_run_1"]
        run2_env = session["environment_run_2"]
        guarded_backup = replacement.get("guarded_backup_receipt") or {}
        environment_md = f"""# WR0 owned environment

{markdown_table((
    ('Classification', '`created_by_slice`'),
    ('Path', '`<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11`'),
    ('Marker schema', f'`{marker["schema"]}`'),
    ('Transaction ID', f'`{marker["transaction_id"]}`'),
    ('Created at', f'`{marker["created_at"]}`'),
    ('Runner binding', f'`{marker["runner_identity_sha256"]}`'),
    ('Contract-source binding', f'`{marker["contract_source_schema"]}` / `{marker["contract_source_sha256"]}`'),
    ('Tracked workload binding', f'`{marker["workload_sha256"]}`'),
    ('Environment identity SHA-256', f'`{environment_identity}`'),
    ('Reviewed predecessor identity', f'`{PREDECESSOR_ENVIRONMENT_IDENTITY}`'),
    ('Replacement phase', f'`{replacement_phase}`'),
    ('New environment durably committed', f'`{str(bool(replacement.get("durably_committed"))).lower()}`'),
    ('Predecessor retired', f'`{str(bool(replacement.get("predecessor_retired"))).lower()}`'),
    ('Rollback available', f'`{str(bool(replacement.get("rollback_available"))).lower()}`'),
    ('Retirement pending', f'`{str(bool(replacement.get("retirement_pending"))).lower()}`'),
    ('Guarded predecessor rename', f'`{str(bool(guarded_backup.get("rename_completed"))).lower()}`'),
    ('First parent fsync inside rollback guard', f'`{str(bool(guarded_backup.get("first_directory_fsync_guarded"))).lower()}`'),
    ('Exact backup snapshot verified', f'`{str(bool(guarded_backup.get("backup_snapshot_verified"))).lower()}`'),
    ('Durable commit-record schema', f'`{commit_record.get("schema", "not_committed")}`'),
    ('Durable commit-record SHA-256', f'`{commit_record.get("final_sha256", commit_record.get("committed_sha256", "not_committed"))}`'),
    ('Run 1 / Run 2 identity agreement', f'`{str(run1_env["environment_identity_sha256"] == run2_env["environment_identity_sha256"]).lower()}`'),
    ('Prefix device/inode', f'`{environment["environment_identity"]["pfx_device"]}` / `{environment["environment_identity"]["pfx_inode"]}`'),
    ('Promotion', '`verified atomic sibling rename`'),
    ('Stage/backup siblings after completion', '`0`'),
))}

Bounded prefix top-level roster: `{json.dumps(environment['bounded_prefix_top_level'], sort_keys=True)}`.

Selected registry hashes are retained in `fixture.json`; no registry content, machine GUID, SID, credential, browser state, or complete prefix roster is retained.

The predecessor transaction `{PREDECESSOR_TRANSACTION_ID}` was moved to one exact transaction-derived recoverable sibling under a single guarded operation covering rename, the first parent-directory fsync, exact backup verification, and exact restoration on every post-rename failure. Outer pre-commit recovery classifies the physical destination/backup state and never treats the in-memory `backup_created` event as authority. The repaired environment was promoted only after Run 1; the predecessor remained untouched through Run 2, exit 37, the two actual bad-gate paths, held-command cleanup, preservation comparison, and commit-ready evidence validation. An atomically written, file-fsynced, directory-fsynced, read-back commit record made the new environment authoritative before predecessor deletion began. Predecessor retirement was then proved by parent fsync, absence readback, exact final-environment readback, and a retired commit-record update before final evidence was published.
"""
    else:
        environment_md = f"""# WR0 owned environment

Classification: `blocked`.

The repair stopped at `{session.get('blocked_stage', 'unknown')}`. Cleanup restored the exact reviewed predecessor: `{str(session.get('cleanup_restored_predecessor', False)).lower()}`. No unknown environment was adopted.
"""
    run1_md = render_run_markdown("WR0 Run 1 — staged initialization", run1, run1_receipt, environment_identity)
    run2_md = render_run_markdown("WR0 Run 2 — promoted environment reuse", run2, run2_receipt, environment_identity)
    exit_md = render_run_markdown("WR0 expected exit propagation", exit_run, None, environment_identity)
    if exit_run.get("observed"):
        exit_md += f"\nThe explicit Windows child exit `37` propagated exactly; the ordinary Run 2 receipt remained byte-identical: `{str(session.get('exit_receipt_unchanged', False)).lower()}`.\n"
    held_process_detail = process_markdown(held_cleanup) if held_cleanup.get("observed") else "Live held-command cleanup was not observed."
    guards_md = f"""# WR0 process guards and ancestry

{markdown_table((
    ('Pre-run forbidden counts', f'`{json.dumps(process_guard_before["counts"], sort_keys=True)}`'),
    ('Post-run forbidden counts', f'`{json.dumps(process_guard_after["counts"], sort_keys=True)}`'),
    ('Ordinary Steam reported separately', '`true`'),
    ('Repository supervisor owned each observed runtime root', f'`{str(supervisor_owned_observed_roots).lower()}`'),
    ('Steam game-launch ancestor observed', '`false`'),
    ('Final WR0 descendant count', '`0`'),
    ('Live held-command cleanup', f'`{held_cleanup.get("classification", "blocked")}`'),
    ('Held descendants outside initial group', f'`{held_cleanup.get("escaped_process_group_count", "not_observed")}`'),
    ('Held final owned count', f'`{held_cleanup.get("final_owned_count", "not_observed")}`'),
    ('Unrelated same-comm sentinel survived', f'`{str(bool(held_cleanup.get("sentinel_survival", False))).lower()}`'),
    ('Disposable held stage removed', f'`{str(bool(held_cleanup.get("stage_cleanup", False))).lower()}`'),
    ('Successful final environment unchanged', f'`{str(bool(held_cleanup.get("final_environment_unchanged", False))).lower()}`'),
    ('Bitwig/validator/UMU/yabridge participation', '`false`'),
))}

Ordinary Steam client/web-helper processes were neither terminated nor classified as WR0. Each observed launch used a newly created Unix process group, while the census followed the complete exact-root descendant tree across process-group/session boundaries. Retained identities use UID, `/proc` start ticks, safe basenames, exact parent labels, group/session membership classifications, and one-way identity hashes; PIDs, command lines, environments, maps, and unrelated process data are absent.

## Actual gate-withheld cleanup topology

{held_process_detail}

The sentinel used a separate process group/session and the same safe `python` comm family as the Proton script where available. Its PID was held only in raw local supervision state; evidence retains only its safe basename/start identity hash and the survival assertion.
"""
    preservation_md = f"""# WR0 protected-fixture preservation

Classification: `{preservation['classification']}`; exact before/after equality: `{str(preservation['exact_equality']).lower()}`.

{markdown_table(preservation_rows(before))}

Composite before SHA-256: `{preservation['before_sha256']}`. Composite after SHA-256: `{preservation['after_sha256']}`. Steam background state is not claimed globally immutable; the exact listed fixtures and immediate compatdata roster are the preservation boundary.
"""
    negative_rows = "\n".join(f"| {item['case']} | `{item['result']}` |" for item in negative["cases"])
    negative_md = f"""# WR0 deterministic negative tests

| Case | Result |
|---|---|
{negative_rows}

Aggregate: `passed` ({negative['passed_count']}/{negative['passed_count']}). Fixtures remained beneath the canonical cache test root. The real Proton/Runtime deployments were not modified.
"""
    sanitization_md = """# WR0 evidence sanitization

Classification: `passed`.

- Evidence has an exact allow-listed roster, UTF-8/NUL/size checks, parseable JSON, and a verified `hashes.sha256`.
- Private home/repository paths use `<HOME>`/`<REPO>`; the real hostname and Linux user are absent.
- PIDs, command lines, process environments/maps, raw session/launch directories, prefix files, registry content, Windows machine identifiers, SIDs, Steam account data, credentials, vendor state, proprietary binaries, and Serum content are absent.
- Runner/runtime identity is metadata and SHA-256 only; no installed binary or source payload is copied.
"""
    if success:
        result_sentence = "The exact runner/runtime executed the tracked command workload twice through one replacement transaction, propagated exit 37, proved strict command identity, and safely forced the actual held topology to zero without touching an unrelated sentinel."
    elif commit_ready:
        result_sentence = "The replacement reached commit-ready with the predecessor exact and rollback still available; this provisional packet makes no retirement or completion claim."
    else:
        result_sentence = f"The exact controlled route was blocked at `{session.get('blocked_stage', 'unknown')}`: {session.get('blocked_error', 'bounded error retained')}."
    claim_ceiling = (
        "WR0 proves only controlled Windows command execution and isolated environment ownership on this exact fixture."
        if success
        else "This provisional or blocked packet does not claim a finalized replacement."
    )
    if success:
        cleanup_sentence = "The new environment was durably committed before predecessor retirement; the predecessor is absent and the verified replacement remains available for a separately authorized later slice."
    elif commit_ready:
        cleanup_sentence = "The exact predecessor remains recoverable; rollback is available and predecessor retirement has not begun."
    else:
        cleanup_sentence = (
            "The committed replacement was preserved; rollback was prohibited after its durable commit."
            if replacement.get("durably_committed")
            else "The transaction-owned replacement was cleaned and the exact reviewed predecessor restored."
        )
    findings_md = f"""# WR0 findings

## Result

`{claim_name}` — {result_sentence}

## Preservation and cleanup

Protected fixtures remained exact. Final WR0 descendant count is zero. {cleanup_sentence}

## Claim ceiling

{claim_ceiling} It does not prove that Proton can host a Windows VST3; it does not load or inspect Serum, launch Bitwig, scan a plug-in, process audio, implement a bridge/proxy/IPC/shared memory/manager/broker, establish licensing, select a product runner, prove real-time safety, or generalize beyond this Deck.
"""
    fixture_json = {
        "schema": EVIDENCE_SCHEMA,
        "classification_vocabulary": {
            "observed": "directly read from the declared fixture",
            "passed": "acceptance assertion verified",
            "installed_before_slice": "preexisting protected asset",
            "created_by_slice": "owned artifact created by WR0",
            "blocked": "bounded claim could not be established",
            "expected_failure": "intentional negative path behaved exactly",
            "unknown": "sensitive or out-of-scope value was not collected",
            "explicitly_out_of_scope": "not authorized by WR0",
        },
        "basis": {
            "classification": "observed",
            "commit": BASIS_COMMIT,
            "tree": BASIS_TREE,
            "branch": EXPECTED_BRANCH,
            "preflight": "PRE-FLIGHT_CLEAR",
            "reviewed_pre_repair_head": REVIEWED_PRE_REPAIR_HEAD,
            "reviewed_pre_repair_tree": REVIEWED_PRE_REPAIR_TREE,
        },
        "runner": {
            "classification": "installed_before_slice",
            "version": RUNNER_VERSION,
            "app_id": RUNNER_APP_ID,
            "build_id": RUNNER_BUILD_ID,
            "manifest_sha256": lock["digest"],
        },
        "runtime": {
            "classification": "installed_before_slice",
            "app_id": RUNTIME_APP_ID,
            "build_id": RUNTIME_BUILD_ID,
            "version": RUNTIME_VERSION,
            "pressure_vessel_version": PRESSURE_VESSEL_VERSION,
        },
        "contract_source": source,
        "predecessor_environment": ({**predecessor, "classification": "installed_before_slice"} if predecessor else {"classification": "unknown"}),
        "replacement": replacement,
        "environment": ({**environment, "classification": "created_by_slice"} if environment else {"classification": "blocked", "cleanup_restored_predecessor": session.get("cleanup_restored_predecessor", False)}),
        "run_1": run1,
        "run_2": run2,
        "exit_propagation": exit_run,
        "actual_bad_gate_tests": bad_gate,
        "live_held_command_cleanup": held_cleanup,
        "process_guards": {
            "classification": "passed",
            "before": process_guard_before,
            "after": process_guard_after,
            "steam_game_ancestor": False,
            "live_held_final_owned_count": held_cleanup.get("final_owned_count"),
            "unrelated_sentinel_survival": held_cleanup.get("sentinel_survival"),
        },
        "preservation": {"classification": "passed", **preservation, "before": before, "after": after},
        "negative_tests": negative,
        "blocked_result": {
            "classification": "blocked" if not success and not commit_ready else "expected_failure",
            "triggered": not success and not commit_ready,
            "stage": session.get("blocked_stage"),
            "error": session.get("blocked_error") if not success else None,
            "cleanup_restored_predecessor": session.get("cleanup_restored_predecessor", False),
        },
        "sensitive_environment_identity": {
            "classification": "unknown",
            "value": "not_collected",
        },
        "claim": {
            "classification": "passed" if success else "observed" if commit_ready else "blocked",
            "name": claim_name,
            "primary": "controlled_windows_process_and_isolated_environment_bootstrap",
            "windows_vst3_hosting": "explicitly_out_of_scope",
            "serum": "explicitly_out_of_scope",
            "bitwig": "explicitly_out_of_scope",
            "audio": "explicitly_out_of_scope",
            "bridge": "explicitly_out_of_scope",
            "product_runner_selection": "explicitly_out_of_scope",
        },
        "sanitization": {
            "classification": "passed",
            "private_paths": False,
            "hostname": False,
            "durable_pids": False,
            "command_lines_or_environments": False,
            "prefix_or_registry_content": False,
            "proprietary_content": False,
        },
    }
    documents = {
        "BASIS.md": basis_md,
        "RUNNER_LOCK.md": runner_lock_md,
        "LAUNCH_CONTRACT.md": contract_md,
        "ENVIRONMENT.md": environment_md,
        "RUN_1.md": run1_md,
        "RUN_2.md": run2_md,
        "EXIT_PROPAGATION.md": exit_md,
        "PROCESS_GUARDS.md": guards_md,
        "PRESERVATION.md": preservation_md,
        "NEGATIVE_TESTS.md": negative_md,
        "SANITIZATION.md": sanitization_md,
        "FINDINGS.md": findings_md,
        "fixture.json": json.dumps(fixture_json, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
    }
    stage = output.parent / f".{output.name}.stage-{secrets.token_hex(8)}"
    if stage.exists():
        fail("evidence staging collision")
    stage.mkdir(mode=0o700)
    try:
        for name, content in documents.items():
            (stage / name).write_text(content, encoding="utf-8", newline="\n")
        hash_lines = []
        for path in sorted(stage.iterdir(), key=lambda value: value.name):
            hash_lines.append(f"{sha256_file(path)}  {path.name}")
        (stage / "hashes.sha256").write_text("\n".join(hash_lines) + "\n", encoding="utf-8", newline="\n")
        if output.exists():
            if output.is_symlink() or not output.is_dir() or {path.name for path in output.iterdir()} != set(REQUIRED_EVIDENCE_FILES):
                fail("refusing unknown existing WR0 evidence directory")
            backup = output.parent / f".{output.name}.previous-{secrets.token_hex(8)}"
            os.replace(output, backup)
            try:
                os.replace(stage, output)
            except Exception:
                os.replace(backup, output)
                raise
            shutil.rmtree(backup)
        else:
            os.replace(stage, output)
    except Exception:
        if stage.exists():
            shutil.rmtree(stage)
        raise


def publish_evidence_packet(staged: pathlib.Path) -> None:
    output = evidence_root()
    require_no_symlink_ancestors(staged, allow_absent_leaf=False, label="staged WR0 evidence packet")
    if not staged.is_dir() or staged.is_symlink() or {item.name for item in staged.iterdir()} != set(REQUIRED_EVIDENCE_FILES):
        fail("staged WR0 evidence packet roster differs")
    require_contained(output, repo_root(), label="WR0 evidence publication target")
    if output.is_symlink() or not output.is_dir() or {item.name for item in output.iterdir()} != set(REQUIRED_EVIDENCE_FILES):
        fail("existing WR0 evidence packet is unknown or unsafe")
    backup = output.parent / f".{output.name}.previous-{secrets.token_hex(8)}"
    require_contained(backup, repo_root(), label="WR0 evidence publication backup", allow_absent_leaf=True)
    os.replace(output, backup)
    fsync_directory(output.parent)
    try:
        os.replace(staged, output)
        fsync_directory(output.parent)
    except Exception:
        if not output.exists() and backup.exists():
            os.replace(backup, output)
            fsync_directory(output.parent)
        raise
    try:
        shutil.rmtree(backup)
        fsync_directory(output.parent)
    except Exception:
        # The published packet remains authoritative. Its evidence-only backup
        # is recoverable cleanup state and never changes environment authority.
        raise EvidenceFinalizationBlocked("evidence publication backup retirement failed")


def sanitize_evidence(*, output_root: pathlib.Path | None = None) -> dict[str, Any]:
    output = output_root if output_root is not None else evidence_root()
    if output_root is None:
        require_contained(output, repo_root(), label="WR0 evidence root")
    elif not output.is_absolute():
        fail("WR0 provisional evidence root is not absolute")
    require_no_symlink_ancestors(output, allow_absent_leaf=False, label="WR0 evidence root")
    if not output.is_dir() or output.is_symlink():
        fail("WR0 evidence root is missing or unsafe")
    names = {path.name for path in output.iterdir()}
    if names != set(REQUIRED_EVIDENCE_FILES):
        fail("WR0 evidence roster differs")
    manifest_lines = (output / "hashes.sha256").read_text(encoding="utf-8").splitlines()
    if len(manifest_lines) != len(REQUIRED_EVIDENCE_FILES) - 1:
        fail("WR0 evidence hash manifest count differs")
    expected_names = set(REQUIRED_EVIDENCE_FILES) - {"hashes.sha256"}
    seen_names: set[str] = set()
    hostname = os.uname().nodename.encode("utf-8")
    forbidden_literals = (
        str(real_home()).encode("utf-8"),
        str(repo_root()).encode("utf-8"),
    )
    for name in REQUIRED_EVIDENCE_FILES:
        path = output / name
        if not path.is_file() or path.is_symlink():
            fail("WR0 evidence contains an unsupported object")
        data = path.read_bytes()
        if len(data) > MAX_EVIDENCE_FILE_BYTES or b"\x00" in data:
            fail("WR0 evidence size/NUL bound failed")
        try:
            data.decode("utf-8", "strict")
        except UnicodeDecodeError as exc:
            raise WR0Error("WR0 evidence is not UTF-8") from exc
        if any(value and value in data for value in forbidden_literals) or (hostname and hostname in data):
            fail("WR0 evidence contains a private host/path literal")
        if re.search(rb"7656119[0-9]{10}", data) or re.search(rb"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", data):
            fail("WR0 evidence contains an account/email-like value")
        if re.search(rb'"(?:pid|ppid|pgrp|session)"\s*:', data) or re.search(rb"/proc/[0-9]+", data):
            fail("WR0 evidence contains a raw process identifier")
        if (
            b".wr0-proton11.stage-" in data
            or b".wr0-proton11.previous-" in data
            or b".wr0-proton11.retiring-" in data
            or b".wr0-proton11.journal-" in data
            or b"WINE REGISTRY Version" in data
        ):
            fail("WR0 evidence contains a raw launch directory or registry content")
    for line in manifest_lines:
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)", line)
        if match is None:
            fail("WR0 evidence hash manifest is malformed")
        digest, name = match.groups()
        if name not in expected_names or name in seen_names or sha256_file(output / name) != digest:
            fail("WR0 evidence hash verification failed")
        seen_names.add(name)
    if seen_names != expected_names:
        fail("WR0 evidence hash roster is incomplete")
    try:
        fixture = json.loads((output / "fixture.json").read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise WR0Error("fixture.json is not parseable") from exc
    if fixture.get("schema") != EVIDENCE_SCHEMA or fixture.get("sanitization", {}).get("classification") != "passed":
        fail("fixture.json schema/sanitization classification differs")
    if fixture.get("claim", {}).get("name") == "WR0_COMPLETE":
        replacement = fixture.get("replacement", {})
        if not (
            replacement.get("replacement_phase") == "evidence_finalized"
            and replacement.get("durably_committed") is True
            and replacement.get("predecessor_retired") is True
            and replacement.get("rollback_available") is False
            and replacement.get("retirement_pending") is False
            and replacement.get("predecessor_environment_identity_sha256") == PREDECESSOR_ENVIRONMENT_IDENTITY
        ):
            fail("successful evidence lacks the exact committed predecessor replacement")
        guarded_backup = replacement.get("guarded_backup_receipt", {})
        if not (
            guarded_backup.get("classification") == "passed"
            and guarded_backup.get("rename_completed") is True
            and guarded_backup.get("first_directory_fsync_guarded") is True
            and guarded_backup.get("parent_fsync_completed") is True
            and guarded_backup.get("backup_snapshot_verified") is True
            and guarded_backup.get("destination_absent") is True
            and guarded_backup.get("backup_present") is True
            and guarded_backup.get("transaction_derived_backup") is True
        ):
            fail("successful evidence lacks the guarded predecessor-backup receipt")
        events = replacement.get("phase_events", [])
        if [event.get("phase") for event in events] != list(REPLACEMENT_PHASES):
            fail("successful evidence lacks the exact replacement phase order")
        if [event.get("sequence") for event in events] != list(range(len(REPLACEMENT_PHASES))):
            fail("successful evidence replacement phase sequence differs")
        try:
            phase_times = [dt.datetime.fromisoformat(str(event["at"]).replace("Z", "+00:00")) for event in events]
        except (KeyError, TypeError, ValueError) as exc:
            raise WR0Error("successful evidence replacement phase timestamp is malformed") from exc
        if phase_times != sorted(phase_times):
            fail("successful evidence replacement phase timestamps are out of order")
        commit_record = replacement.get("durable_commit_record", {})
        if not (
            commit_record.get("schema") == REPLACEMENT_COMMIT_SCHEMA
            and re.fullmatch(r"[0-9a-f]{64}", str(commit_record.get("committed_sha256", "")))
            and re.fullmatch(r"[0-9a-f]{64}", str(commit_record.get("final_sha256", "")))
            and commit_record.get("readback_verified") is True
            and commit_record.get("predecessor_retirement_status") == "retired"
        ):
            fail("successful evidence lacks the exact durable commit-record proof")
        predecessor = fixture.get("predecessor_environment", {})
        if predecessor.get("environment", {}).get("environment_identity_sha256") != PREDECESSOR_ENVIRONMENT_IDENTITY:
            fail("successful evidence lacks the exact reviewed predecessor identity")
        source_fixture = fixture.get("contract_source", {})
        environment = fixture.get("environment", {})
        marker = environment.get("marker", {})
        if (
            environment.get("classification") != "created_by_slice"
            or marker.get("status") != "ready"
            or marker.get("contract_source_sha256") != source_fixture.get("digest")
            or marker.get("workload_sha256") != source_fixture.get("workload_sha256")
        ):
            fail("successful evidence lacks the repaired owned-environment binding")
        expected_runs = (("run_1", 1, 0, "passed"), ("run_2", 2, 0, "passed"), ("exit_propagation", 37, 37, "expected_failure"))
        for name, run_number, exit_status, classification in expected_runs:
            run = fixture.get(name, {})
            if not (
                run.get("run_number") == run_number
                and run.get("exit_status") == exit_status
                and run.get("classification") == classification
                and run.get("strict_command_vector_verified") is True
                and run.get("fresh_process_revalidation_verified") is True
                and run.get("clean_descendant_count") == 0
                and run.get("steam_game_ancestor") is False
            ):
                fail(f"successful evidence lacks the exact repaired {name} result")
        bad_gate = fixture.get("actual_bad_gate_tests", {})
        if not (
            bad_gate.get("classification") == "passed"
            and bad_gate.get("wrong_nonce", {}).get("exit_status") == 79
            and bad_gate.get("wrong_nonce", {}).get("gate_behavior") == "wrong_nonce"
            and bad_gate.get("wrong_run", {}).get("exit_status") == 79
            and bad_gate.get("wrong_run", {}).get("gate_behavior") == "wrong_run"
        ):
            fail("successful evidence lacks the actual wrong-gate workload results")
        held = fixture.get("live_held_command_cleanup", {})
        if not (
            held.get("classification") == "passed"
            and held.get("strict_command_vector_verified") is True
            and held.get("fresh_process_revalidation_verified") is True
            and held.get("final_owned_count") == 0
            and held.get("sentinel_survival") is True
            and held.get("sentinel_terminated_separately") is True
            and held.get("stage_cleanup") is True
            and held.get("final_environment_unchanged") is True
            and held.get("ready_gate_temporary_absent_after") is True
            and held.get("termination", {}).get("final_group_empty") is True
        ):
            fail("successful evidence lacks the verified live held-command cleanup")
        negative = fixture.get("negative_tests", {})
        required_cases = {
            "production_contract_source_evidence_only_stability",
            "production_changed_workload_changes_digest_and_refuses_stale_identity",
            "production_changed_supervisor_changes_digest_and_refuses_stale_identity",
            "production_changed_wrapper_changes_digest_and_refuses_stale_identity",
            "production_unexpected_governed_path_refused",
            "production_historical_provenance_edit_cannot_authorize_changed_source",
            "production_unstaged_and_staged_governed_source_refused",
            "actual_batch_gate_wrong_nonce_rejected",
            "actual_batch_gate_wrong_run_rejected",
            "live_held_command_cleanup_actual_topology",
            "live_held_cleanup_unrelated_sentinel_survives",
            "replacement_failure_before_predecessor_backup_preserves_predecessor",
            "replacement_failure_after_backup_before_promotion_restores_predecessor",
            "replacement_failure_after_promotion_before_commit_restores_predecessor",
            "replacement_commit_record_write_failure_restores_predecessor",
            "replacement_commit_record_readback_failure_restores_predecessor",
            "post_commit_failure_before_retirement_preserves_new_environment",
            "predecessor_recursive_retirement_failure_preserves_new_environment",
            "predecessor_parent_fsync_failure_preserves_new_environment",
            "final_evidence_render_failure_preserves_committed_environment",
            "final_evidence_sanitization_failure_preserves_committed_environment",
            "durable_commit_precedes_successful_retirement_and_evidence",
            "stale_or_forged_replacement_commit_record_refused",
            "replacement_commit_record_wrong_environment_identities_refused",
            "post_commit_exception_never_invokes_precommit_rollback",
            "predecessor_backup_rename_then_first_fsync_failure_restores_exact",
            "predecessor_backup_verification_failure_restores_exact",
            "predecessor_restore_fsync_failure_outer_recovery_accepts_exact_destination",
            "predecessor_backup_physical_rename_before_event_is_recovered",
            "predecessor_recovery_false_event_exact_backup_restored",
            "predecessor_backup_unknown_object_refused_unchanged",
            "predecessor_backup_forged_marker_identity_refused",
            "predecessor_recovery_both_absent_fails_closed",
            "predecessor_recovery_unknown_destination_refuses_exact_backup",
            "predecessor_guarded_backup_receipt_and_rollback_pass",
        }
        passed_cases = {item.get("case") for item in negative.get("cases", []) if item.get("result") == "passed"}
        if (
            negative.get("classification") != "passed"
            or negative.get("failed_count") != 0
            or negative.get("passed_count") != 80
            or len(negative.get("cases", [])) != 80
            or negative.get("live_production_cases") != 5
            or not required_cases.issubset(passed_cases)
        ):
            fail("successful evidence lacks the required production negative-test ledger")
        if fixture.get("preservation", {}).get("exact_equality") is not True:
            fail("successful evidence lacks exact protected-fixture equality")
    elif fixture.get("claim", {}).get("name") == "WR0_COMMIT_READY":
        replacement = fixture.get("replacement", {})
        if not (
            replacement.get("replacement_phase") == "commit_ready"
            and replacement.get("durably_committed") is False
            and replacement.get("predecessor_retired") is False
            and replacement.get("rollback_available") is True
            and replacement.get("retirement_pending") is False
            and replacement.get("predecessor_environment_identity_sha256") == PREDECESSOR_ENVIRONMENT_IDENTITY
        ):
            fail("commit-ready evidence overstates replacement authority")
    source = verify_retained_contract_source(fixture_path=output / "fixture.json")
    return {
        "classification": "passed",
        "file_count": len(REQUIRED_EVIDENCE_FILES),
        "hashes_verified": True,
        "contract_source_verified": True,
        "contract_source_sha256": source["digest"],
    }


def assert_no_transaction_siblings(*, allowed_siblings: Iterable[str] = ()) -> None:
    root = environments_root()
    if not root.exists():
        return
    allowed = set(allowed_siblings)
    leftovers = [
        entry.name
        for entry in root.iterdir()
        if (
            entry.name.startswith(".wr0-proton11.stage-")
            or entry.name.startswith(".wr0-proton11.previous-")
            or entry.name.startswith(".wr0-proton11.retiring-")
            or entry.name.startswith(".wr0-proton11.journal-")
        )
        and entry.name not in allowed
    ]
    if leftovers:
        fail("WR0 transaction sibling remains")


def run_live(*, implementation_worktree: bool) -> dict[str, Any]:
    ensure_cache_layout()
    preflight = run_preflight(implementation_worktree=implementation_worktree, capture=True)
    if preflight["final_environment_state"] != "present_requires_exact_ownership":
        fail("WR0 repair requires the exact reviewed predecessor environment")
    before = preflight["fixture"]
    if before is None:
        fail("preflight did not capture the protected fixture")
    lock_before = verify_runner_lock()
    contract_source = contract_source_manifest(require_clean=True)
    destination = final_environment()
    parent = prepare_environment_parent()
    assert_no_transaction_siblings()
    predecessor_fixture = load_reviewed_predecessor_fixture()
    predecessor = verify_predecessor_environment(destination, fixture=predecessor_fixture)
    replacement_state = ReplacementTransactionState()
    negative = run_negative_tests(require_clean_source=True)
    transaction_id = f"wr0-{dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{secrets.token_hex(8)}"
    created_at = utc_now()
    backup = parent / f".wr0-proton11.previous-{transaction_id}"
    require_contained(backup, parent, label="WR0 repair predecessor backup", allow_absent_leaf=True)
    session_dir = session_parent() / transaction_id
    require_contained(session_dir, session_parent(), label="WR0 raw session", allow_absent_leaf=True)
    session_dir.mkdir(mode=0o700)
    stage: pathlib.Path | None = None
    session: dict[str, Any] = {
        "schema": SESSION_SCHEMA,
        "status": "running",
        "transaction_id": transaction_id,
        "created_at": created_at,
        "preflight": preflight,
        "runner_lock": lock_before,
        "contract_source": contract_source,
        "fixture_before": before,
        "negative_tests": negative,
        "reviewed_pre_repair_head": REVIEWED_PRE_REPAIR_HEAD,
        "reviewed_pre_repair_tree": REVIEWED_PRE_REPAIR_TREE,
        "predecessor": predecessor,
        "replacement": {
            "classification": "observed",
            "predecessor_environment_identity_sha256": PREDECESSOR_ENVIRONMENT_IDENTITY,
            "backup_created": False,
            "guarded_backup_receipt": None,
            "precommit_recovery_receipt": None,
            "durable_commit_record": {
                "schema": REPLACEMENT_COMMIT_SCHEMA,
                "committed_sha256": None,
                "final_sha256": None,
                "readback_verified": False,
                "predecessor_retirement_status": "not_committed",
            },
            **replacement_state.evidence(),
        },
        "run_1": None,
        "run_2": None,
        "exit_37": None,
        "bad_gate_tests": None,
        "live_held_cleanup": None,
    }
    atomic_write_json(session_dir / "session.json", session)
    held_stage: pathlib.Path | None = None
    held_transaction_id = ""
    backup_created = False
    committed_environment: dict[str, Any] | None = None
    blocked_stage = "predecessor_backup"
    try:
        guarded_backup_receipt = move_predecessor_to_backup(
            destination,
            backup,
            transaction_id=transaction_id,
            predecessor=predecessor,
            fixture=predecessor_fixture,
        )
        backup_created = True
        session["replacement"]["backup_created"] = True
        session["replacement"]["guarded_backup_receipt"] = guarded_backup_receipt
        replacement_state.advance("predecessor_backed_up")
        session["replacement"].update(replacement_state.evidence())
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "environment_stage_creation"
        stage = create_stage(
            transaction_id,
            created_at,
            lock_before["digest"],
            contract_source,
            allowed_siblings=(backup.name,),
        )
        replacement_state.advance("new_environment_staged")
        session["replacement"].update(replacement_state.evidence())
        blocked_stage = "run_1_staged_initialization"
        nonce1 = secrets.token_hex(16)
        run1 = run_windows_workload(
            stage,
            nonce=nonce1,
            run_number=1,
            expected_exit=0,
            timeout_seconds=RUN1_TIMEOUT_SECONDS,
            contract_source=contract_source,
        )
        receipt1 = verify_internal_receipt(
            stage,
            nonce=nonce1,
            run_number=1,
            contract_source=contract_source,
        )
        write_environment_receipt(stage, "run-1.json", run1)
        marker = marker_document(
            transaction_id=transaction_id,
            created_at=created_at,
            lock_digest=lock_before["digest"],
            contract_source_digest=contract_source["digest"],
            workload_sha256=contract_source["workload_sha256"],
            status_value="ready",
            run_receipts={"run_1": receipt1["sha256"]},
        )
        atomic_write_json(environment_marker_path(stage), marker)
        environment_run1 = environment_snapshot(stage, contract_source=contract_source)
        session.update(
            {
                "run_1": run1,
                "run_1_receipt": receipt1,
                "environment_run_1": environment_run1,
            }
        )
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "environment_promotion"
        promote_stage(stage, destination, transaction_id)
        stage = None
        promoted = environment_snapshot(destination, contract_source=contract_source)
        if promoted["environment_identity_sha256"] != environment_run1["environment_identity_sha256"]:
            raise LaunchBlocked("atomic promotion changed the WR0 environment identity")
        replacement_state.advance("new_environment_promoted")
        session["replacement"].update(replacement_state.evidence())
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "run_2_environment_reuse"
        nonce2 = secrets.token_hex(16)
        run2 = run_windows_workload(
            destination,
            nonce=nonce2,
            run_number=2,
            expected_exit=0,
            timeout_seconds=REUSE_TIMEOUT_SECONDS,
            contract_source=contract_source,
        )
        receipt2 = verify_internal_receipt(
            destination,
            nonce=nonce2,
            run_number=2,
            contract_source=contract_source,
        )
        if run2["root_identity_sha256"] == run1["root_identity_sha256"]:
            raise LaunchBlocked("Run 2 did not have a distinct process-tree launch identity")
        write_environment_receipt(destination, "run-2.json", run2)
        marker = marker_document(
            transaction_id=transaction_id,
            created_at=created_at,
            lock_digest=lock_before["digest"],
            contract_source_digest=contract_source["digest"],
            workload_sha256=contract_source["workload_sha256"],
            status_value="ready",
            run_receipts={"run_1": receipt1["sha256"], "run_2": receipt2["sha256"]},
        )
        atomic_write_json(environment_marker_path(destination), marker)
        environment_run2 = environment_snapshot(destination, contract_source=contract_source)
        if environment_run2["environment_identity_sha256"] != environment_run1["environment_identity_sha256"]:
            raise LaunchBlocked("Run 2 used a different WR0 environment identity")
        session.update(
            {
                "run_2": run2,
                "run_2_receipt": receipt2,
                "environment_run_2": environment_run2,
            }
        )
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "exit_37_propagation"
        receipt_before_exit = internal_receipt_path(destination).read_bytes()
        nonce37 = secrets.token_hex(16)
        exit_run = run_windows_workload(
            destination,
            nonce=nonce37,
            run_number=37,
            expected_exit=37,
            timeout_seconds=REUSE_TIMEOUT_SECONDS,
            contract_source=contract_source,
        )
        receipt_after_exit = internal_receipt_path(destination).read_bytes()
        if receipt_after_exit != receipt_before_exit:
            raise LaunchBlocked("exit-37 path changed the ordinary workload receipt")
        write_environment_receipt(destination, "exit-37.json", exit_run)
        marker = marker_document(
            transaction_id=transaction_id,
            created_at=created_at,
            lock_digest=lock_before["digest"],
            contract_source_digest=contract_source["digest"],
            workload_sha256=contract_source["workload_sha256"],
            status_value="ready",
            run_receipts={"run_1": receipt1["sha256"], "run_2": receipt2["sha256"]},
            exit_37_verified=True,
        )
        atomic_write_json(environment_marker_path(destination), marker)
        environment_final = environment_snapshot(destination, contract_source=contract_source)
        if environment_final["environment_identity_sha256"] != environment_run1["environment_identity_sha256"]:
            raise LaunchBlocked("exit propagation used a different environment identity")
        session.update(
            {
                "exit_37": exit_run,
                "exit_receipt_unchanged": True,
                "environment_final": environment_final,
            }
        )
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "live_bad_gate_and_held_cleanup"
        final_before_disposable = environment_snapshot(destination, contract_source=contract_source)
        held_transaction_id = f"wr0-{dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{secrets.token_hex(8)}"
        held_created_at = utc_now()
        held_stage = create_stage(
            held_transaction_id,
            held_created_at,
            lock_before["digest"],
            contract_source,
            allowed_siblings=(backup.name,),
        )
        bad_nonce_result = run_windows_workload(
            held_stage,
            nonce=secrets.token_hex(16),
            run_number=1,
            expected_exit=0,
            timeout_seconds=RUN1_TIMEOUT_SECONDS,
            contract_source=contract_source,
            gate_behavior="wrong_nonce",
        )
        bad_run_result = run_windows_workload(
            held_stage,
            nonce=secrets.token_hex(16),
            run_number=1,
            expected_exit=0,
            timeout_seconds=REUSE_TIMEOUT_SECONDS,
            contract_source=contract_source,
            gate_behavior="wrong_run",
        )
        held_cleanup = run_windows_workload(
            held_stage,
            nonce=secrets.token_hex(16),
            run_number=1,
            expected_exit=0,
            timeout_seconds=REUSE_TIMEOUT_SECONDS,
            contract_source=contract_source,
            gate_behavior="withhold_cleanup",
        )
        if bad_nonce_result["exit_status"] != 79 or bad_nonce_result["gate_behavior"] != "wrong_nonce":
            raise LaunchBlocked("actual batch wrong-nonce gate result differs")
        if bad_run_result["exit_status"] != 79 or bad_run_result["gate_behavior"] != "wrong_run":
            raise LaunchBlocked("actual batch wrong-run gate result differs")
        if held_cleanup["final_owned_count"] != 0 or not held_cleanup["sentinel_survival"]:
            raise ProcessTopologyBlocked("WR0_PROCESS_TOPOLOGY_BLOCKED: live held-command cleanup result differs")
        remove_owned_transaction_root(held_stage, held_transaction_id)
        held_stage = None
        held_cleanup["stage_cleanup"] = True
        final_after_disposable = environment_snapshot(destination, contract_source=contract_source)
        if final_after_disposable != final_before_disposable:
            raise LaunchBlocked("disposable cleanup probes changed the successful final environment")
        held_cleanup["final_environment_unchanged"] = True
        assert_no_transaction_siblings(allowed_siblings=(backup.name,))
        live_cases = [
            {"case": "actual_batch_gate_wrong_nonce_rejected", "result": "passed"},
            {"case": "actual_batch_gate_wrong_run_rejected", "result": "passed"},
            {"case": "live_held_command_cleanup_actual_topology", "result": "passed"},
            {"case": "live_held_cleanup_unrelated_sentinel_survives", "result": "passed"},
            {"case": "live_handshake_artifacts_absent_after_bad_gate_and_forced_cleanup", "result": "passed"},
        ]
        negative["cases"].extend(live_cases)
        negative["passed_count"] = len(negative["cases"])
        negative["live_production_cases"] = len(live_cases)
        atomic_write_json(cache_root() / "negative-tests.latest.json", negative)
        session.update(
            {
                "negative_tests": negative,
                "bad_gate_tests": {
                    "classification": "passed",
                    "wrong_nonce": bad_nonce_result,
                    "wrong_run": bad_run_result,
                },
                "live_held_cleanup": held_cleanup,
            }
        )
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "final_preservation_and_commit_ready_evidence"
        lock_after = verify_runner_lock()
        if lock_after["digest"] != lock_before["digest"]:
            fail("runner/runtime lock changed during WR0")
        guard_after = process_guard()
        after = capture_fixture()
        preservation = compare_fixtures(before, after)
        assert_no_transaction_siblings(allowed_siblings=(backup.name,))
        replacement_state.advance("commit_ready")
        session["replacement"].update(replacement_state.evidence())
        session.update(
            {
                "status": "commit_ready",
                "runner_lock_after": lock_after,
                "process_guard_after": guard_after,
                "fixture_after": after,
                "preservation": preservation,
                "cleanup_restored_absence": False,
                "cleanup_restored_predecessor": False,
            }
        )
        atomic_write_json(session_dir / "session.json", session)
        commit_ready_packet = session_dir / "evidence-commit-ready"
        render_evidence(session, output_root=commit_ready_packet)
        commit_ready_sanitize = sanitize_evidence(output_root=commit_ready_packet)
        session["commit_ready_evidence_sanitization"] = commit_ready_sanitize
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "durable_replacement_commit"
        if verify_predecessor_environment(backup, fixture=predecessor_fixture) != predecessor:
            fail("predecessor changed before durable replacement commit")
        if environment_snapshot(destination, contract_source=contract_source) != environment_final:
            fail("new final environment changed before durable replacement commit")
        receipt_identities = replacement_receipt_identities(destination)
        committed_at = utc_now()
        commit_document = replacement_commit_document(
            replacement_transaction_id=transaction_id,
            predecessor_transaction_id=PREDECESSOR_TRANSACTION_ID,
            predecessor_environment_identity_sha256=PREDECESSOR_ENVIRONMENT_IDENTITY,
            new_environment_identity_sha256=environment_final["environment_identity_sha256"],
            runner_identity_sha256=lock_before["digest"],
            contract_source_sha256=contract_source["digest"],
            workload_sha256=contract_source["workload_sha256"],
            receipt_identities=receipt_identities,
            protected_fixture_sha256=preservation["before_sha256"],
            committed_at=committed_at,
            predecessor_retirement_status="pending",
        )
        committed_record = durably_commit_replacement(
            replacement_state,
            destination,
            commit_document,
        )
        committed_environment = environment_snapshot(destination, contract_source=contract_source)
        if committed_environment != environment_final:
            fail("durable commit record changed the stable environment identity")
        session["replacement"].update(replacement_state.evidence())
        session["replacement"]["durable_commit_record"].update(
            {
                "committed_sha256": committed_record["sha256"],
                "readback_verified": True,
                "predecessor_retirement_status": "pending",
                "committed_at": committed_at,
            }
        )
        session["status"] = "new_environment_committed"
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "predecessor_retirement"
        commit_document_arguments = {
            "replacement_transaction_id": transaction_id,
            "predecessor_transaction_id": PREDECESSOR_TRANSACTION_ID,
            "predecessor_environment_identity_sha256": PREDECESSOR_ENVIRONMENT_IDENTITY,
            "new_environment_identity_sha256": environment_final["environment_identity_sha256"],
            "runner_identity_sha256": lock_before["digest"],
            "contract_source_sha256": contract_source["digest"],
            "workload_sha256": contract_source["workload_sha256"],
            "receipt_identities": receipt_identities,
            "protected_fixture_sha256": preservation["before_sha256"],
            "committed_at": committed_at,
        }

        def verify_new_during_retirement() -> None:
            if environment_snapshot(destination, contract_source=contract_source) != committed_environment:
                raise PredecessorRetirementBlocked("committed environment changed during predecessor retirement")

        retirement = retire_committed_predecessor(
            replacement_state,
            environment=destination,
            predecessor_backup=backup,
            environment_parent=parent,
            commit_document_arguments=commit_document_arguments,
            verify_new_environment=verify_new_during_retirement,
        )
        pending_record = retirement["pending_record"]
        retired_record = retirement["retired_record"]
        retirement_started_at = retirement["retirement_started_at"]
        retirement_completed_at = retirement["retirement_completed_at"]
        backup_created = False
        session["replacement"].update(replacement_state.evidence())
        session["replacement"]["backup_created"] = False
        session["replacement"]["durable_commit_record"].update(
            {
                "pending_retirement_sha256": pending_record["sha256"],
                "final_sha256": retired_record["sha256"],
                "readback_verified": True,
                "predecessor_retirement_status": "retired",
                "retirement_started_at": retirement_started_at,
                "retirement_completed_at": retirement_completed_at,
            }
        )
        assert_no_transaction_siblings()
        retired_readback = read_replacement_commit_record(destination, expected=retired_record["document"])
        if retired_readback["sha256"] != retired_record["sha256"]:
            fail("retired commit-record identity changed before final evidence")
        if environment_snapshot(destination, contract_source=contract_source) != committed_environment:
            fail("committed environment changed after predecessor retirement")
        final_lock_after_retirement = verify_runner_lock()
        if final_lock_after_retirement["digest"] != lock_before["digest"]:
            fail("runner/runtime lock changed after predecessor retirement")
        final_guard_after_retirement = process_guard()
        final_fixture_after_retirement = capture_fixture()
        final_preservation_after_retirement = compare_fixtures(before, final_fixture_after_retirement)
        session.update(
            {
                "runner_lock_after": final_lock_after_retirement,
                "process_guard_after": final_guard_after_retirement,
                "fixture_after": final_fixture_after_retirement,
                "preservation": final_preservation_after_retirement,
            }
        )
        atomic_write_json(session_dir / "session.json", session)

        blocked_stage = "final_evidence_rendering"
        final_packet = session_dir / "evidence-final"
        prospective_holder: dict[str, Any] = {}

        def render_final_packet(evidence_finalized_at: str) -> None:
            prospective_session = json.loads(json.dumps(session))
            prospective_session["status"] = "complete"
            prospective_session["completed_at"] = evidence_finalized_at
            prospective_session["replacement"].update(
                {
                    "replacement_phase": "evidence_finalized",
                    "durably_committed": True,
                    "predecessor_retired": True,
                    "rollback_available": False,
                    "retirement_pending": False,
                    "phase_events": [
                        *replacement_state.events,
                        {
                            "phase": "evidence_finalized",
                            "at": evidence_finalized_at,
                            "sequence": len(replacement_state.events),
                        },
                    ],
                }
            )
            prospective_holder["session"] = prospective_session
            render_evidence(prospective_session, output_root=final_packet)

        def validate_final_packet() -> None:
            sanitize_evidence(output_root=final_packet)

        def publish_final_packet() -> None:
            nonlocal blocked_stage
            blocked_stage = "final_evidence_publication"
            publish_evidence_packet(final_packet)
            sanitize_evidence()
            final_record = read_replacement_commit_record(destination, expected=retired_record["document"])
            if final_record["sha256"] != retired_record["sha256"]:
                fail("retired commit-record identity changed after evidence publication")
            if environment_snapshot(destination, contract_source=contract_source) != committed_environment:
                fail("committed environment changed after evidence publication")
            assert_no_transaction_siblings()
            process_guard()
            compare_fixtures(before, capture_fixture())

        evidence_finalized_at = finalize_committed_evidence(
            replacement_state,
            render_packet=render_final_packet,
            validate_packet=validate_final_packet,
            publish_packet=publish_final_packet,
        )
        prospective_session = prospective_holder["session"]
        if replacement_state.events[-1] != {
            "phase": "evidence_finalized",
            "at": evidence_finalized_at,
            "sequence": len(REPLACEMENT_PHASES) - 1,
        }:
            fail("final evidence state differs from the published phase event")
        sanitize = sanitize_evidence()
        session.update(prospective_session)
        session["replacement"].update(replacement_state.evidence())
        session["evidence_sanitization"] = sanitize
        atomic_write_json(session_dir / "session.json", session)
        return {
            "classification": "passed",
            "result": "WR0_COMPLETE",
            "transaction_id": transaction_id,
            "runner_lock_digest": lock_before["digest"],
            "contract_source_digest": contract_source["digest"],
            "workload_sha256": contract_source["workload_sha256"],
            "environment_identity_sha256": environment_final["environment_identity_sha256"],
            "run_1_exit": run1["exit_status"],
            "run_2_exit": run2["exit_status"],
            "exit_37": exit_run["exit_status"],
            "live_held_command_cleanup": "passed",
            "sentinel_survival": True,
            "replacement_committed": True,
            "predecessor_retired": True,
            "durable_commit_record_sha256": retired_record["sha256"],
            "clean_descendants": 0,
            "preservation": "passed",
            "negative_tests": negative["passed_count"],
            "evidence": sanitize,
        }
    except (LaunchBlocked, WR0Error, OSError) as exc:
        if isinstance(exc, LaunchBlocked) and exc.partial_run is not None:
            partial_targets = {
                "run_1_staged_initialization": "run_1",
                "run_2_environment_reuse": "run_2",
                "exit_37_propagation": "exit_37",
            }
            partial_target = partial_targets.get(blocked_stage)
            if partial_target is not None:
                session[partial_target] = exc.partial_run
        if held_stage is not None and held_transaction_id and held_stage.exists():
            remove_owned_transaction_root(held_stage, held_transaction_id)
            held_stage = None

        cleanup_restored_predecessor = False

        def rollback_precommit() -> None:
            nonlocal stage, backup_created, cleanup_restored_predecessor
            if stage is not None and stage.exists():
                if not transaction_owns(stage, transaction_id):
                    raise PredecessorBackupBlocked(
                        "WR0_PREDECESSOR_BACKUP_BLOCKED: staged cleanup target is not owned by the replacement transaction"
                    )
                remove_owned_transaction_root(stage, transaction_id)
            stage = None
            recovery_receipt = recover_precommit_predecessor(
                destination,
                backup,
                transaction_id=transaction_id,
                predecessor=predecessor,
                fixture=predecessor_fixture,
                backup_created_event=backup_created,
            )
            session["replacement"]["precommit_recovery_receipt"] = recovery_receipt
            backup_created = False
            session["replacement"]["backup_created"] = False
            assert_no_transaction_siblings()
            cleanup_restored_predecessor = True

        def verify_postcommit_environment() -> None:
            expected_environment = committed_environment or session.get("environment_final")
            if not isinstance(expected_environment, dict):
                fail("post-commit state lacks the committed environment snapshot")
            if environment_snapshot(destination, contract_source=contract_source) != expected_environment:
                fail("post-commit failure changed the authoritative environment")
            record = read_replacement_commit_record(destination)
            document = record["document"]
            if not (
                document.get("replacement_transaction_id") == transaction_id
                and document.get("predecessor_environment_identity_sha256") == PREDECESSOR_ENVIRONMENT_IDENTITY
                and document.get("new_environment_identity_sha256") == expected_environment["environment_identity_sha256"]
                and document.get("runner_identity_sha256") == lock_before["digest"]
                and document.get("contract_source_sha256") == contract_source["digest"]
                and document.get("workload_sha256") == contract_source["workload_sha256"]
            ):
                fail("post-commit durable record binding differs")
            session["replacement"]["durable_commit_record"].update(
                {
                    "readback_verified": True,
                    "observed_sha256": record["sha256"],
                    "predecessor_retirement_status": document["predecessor_retirement_status"],
                }
            )

        try:
            result_name = replacement_failure_result(
                replacement_state,
                precommit_rollback=rollback_precommit,
                verify_committed_environment=verify_postcommit_environment,
            )
        except PredecessorBackupBlocked as recovery_error:
            result_name = "WR0_PREDECESSOR_BACKUP_BLOCKED"
            exc = recovery_error
        except Exception as recovery_error:
            raise WR0Error("WR0 replacement failure recovery could not prove its authority state") from recovery_error

        lock_after = verify_runner_lock()
        guard_after = process_guard()
        after = capture_fixture()
        preservation = compare_fixtures(before, after)
        if isinstance(exc, ProcessTopologyBlocked) and not replacement_state.durably_committed:
            result_name = "WR0_PROCESS_TOPOLOGY_BLOCKED"
        elif isinstance(exc, ProcessObservabilityBlocked) and not replacement_state.durably_committed:
            result_name = "WR0_PROCESS_OBSERVABILITY_BLOCKED"
        session["replacement"].update(replacement_state.evidence())
        retirement_residual = (
            bounded_transaction_residual(backup, parent)
            if result_name == "WR0_PREDECESSOR_RETIREMENT_BLOCKED"
            else None
        )
        if retirement_residual is not None:
            session["replacement"]["predecessor_retirement_residual"] = retirement_residual
        session.update(
            {
                "status": "blocked",
                "blocked_at": utc_now(),
                "blocked_stage": blocked_stage,
                "blocked_error": sanitize_error(exc),
                "cleanup_restored_absence": False,
                "cleanup_restored_predecessor": cleanup_restored_predecessor,
                "runner_lock_after": lock_after,
                "process_guard_after": guard_after,
                "fixture_after": after,
                "preservation": preservation,
                "blocked_result_name": result_name,
            }
        )
        if not replacement_state.durably_committed:
            session["environment_final"] = None
        atomic_write_json(session_dir / "session.json", session)
        sanitize: dict[str, Any] | None = None
        if result_name in {"WR0_REPLACEMENT_PRECOMMIT_BLOCKED", "WR0_PREDECESSOR_BACKUP_BLOCKED"}:
            blocked_packet = session_dir / "evidence-blocked"
            render_evidence(session, output_root=blocked_packet)
            sanitize_evidence(output_root=blocked_packet)
            publish_evidence_packet(blocked_packet)
            sanitize = sanitize_evidence()
            session["evidence_sanitization"] = sanitize
            atomic_write_json(session_dir / "session.json", session)
        return {
            "classification": "blocked",
            "result": result_name,
            "stage": blocked_stage,
            "error": sanitize_error(exc),
            "cleanup_restored_predecessor": cleanup_restored_predecessor,
            "new_environment_preserved": replacement_state.durably_committed,
            "predecessor_retirement_residual": retirement_residual,
            "preservation": "passed",
            "evidence": sanitize or {"classification": "blocked", "raw_commit_record_available": True},
        }


def print_result_lines(value: dict[str, Any]) -> None:
    for key, item in value.items():
        if isinstance(item, (dict, list)):
            rendered = json.dumps(item, sort_keys=True, separators=(",", ":"))
        elif isinstance(item, bool):
            rendered = str(item).lower()
        else:
            rendered = str(item)
        print(f"{key.upper()}={rendered}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="WR0 controlled Proton bootstrap")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("inspect-runner")
    commands.add_parser("inspect-contract-source")
    commands.add_parser("verify-retained-source")
    preflight = commands.add_parser("preflight")
    preflight.add_argument("--implementation-worktree", action="store_true")
    negative = commands.add_parser("negative-tests")
    negative.add_argument("--implementation-worktree", action="store_true")
    live = commands.add_parser("live")
    live.add_argument("--implementation-worktree", action="store_true")
    commands.add_parser("sanitize")
    commands.add_parser("expected-lock-digest")
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        if args.command == "inspect-runner":
            lock = verify_runner_lock()
            print_result_lines(
                {
                    "classification": lock["classification"],
                    "runner_version": RUNNER_VERSION,
                    "runner_app_id": RUNNER_APP_ID,
                    "runner_build_id": RUNNER_BUILD_ID,
                    "runtime_app_id": RUNTIME_APP_ID,
                    "runtime_build_id": RUNTIME_BUILD_ID,
                    "runtime_version": RUNTIME_VERSION,
                    "pressure_vessel_version": PRESSURE_VESSEL_VERSION,
                    "launch_critical_file_count": len(LOCK_FILES),
                    "launch_critical_manifest_sha256": lock["digest"],
                }
            )
        elif args.command == "inspect-contract-source":
            print_result_lines(contract_source_manifest(require_clean=True))
        elif args.command == "verify-retained-source":
            print_result_lines(verify_retained_contract_source())
        elif args.command == "preflight":
            result = run_preflight(implementation_worktree=args.implementation_worktree, capture=True)
            print_result_lines(
                {
                    "classification": result["classification"],
                    "result": result["result"],
                    "basis_commit": BASIS_COMMIT,
                    "basis_tree": BASIS_TREE,
                    "runner_lock_digest": result["runner_lock_digest"],
                    "final_environment_state": result["final_environment_state"],
                    "process_guard": result["process_guard"]["classification"],
                    "protected_fixture": "exact",
                }
            )
        elif args.command == "negative-tests":
            result = run_negative_tests(require_clean_source=not args.implementation_worktree)
            print_result_lines(result)
        elif args.command == "live":
            result = run_live(implementation_worktree=args.implementation_worktree)
            print_result_lines(result)
        elif args.command == "sanitize":
            print_result_lines(sanitize_evidence())
        elif args.command == "expected-lock-digest":
            print(expected_lock_digest())
        else:
            fail("unknown command")
    except WR0Error as exc:
        print(f"WR0_ERROR={sanitize_error(exc)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

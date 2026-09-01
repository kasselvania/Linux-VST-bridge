#!/usr/bin/env python3
"""Fixed WR0A final-repair reconciliation owner.

This program intentionally has no configurable production inputs.  Its constants
are the approved WR0A design, not defaults for a reusable migration utility.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import secrets
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from collections.abc import Iterable, Sequence
from typing import Any


SCHEMA = "linux-vst-bridge-wr0a-final-repair-reconciliation/v1"
SOURCE_SCHEMA = "linux-vst-bridge-wr0a-implementation-source/v1"
SESSION_SCHEMA = "linux-vst-bridge-wr0a-session/v1"
EXPECTED_BRANCH = "codex/wr0a-final-repair-reconciliation"
IMPLEMENTATION_BASIS = "5aa757e46f6e8c15751743b68a8504386fbe840c"
IMPLEMENTATION_BASIS_TREE = "a779d2ac85ec74adc907327dde13f5d31201f914"
MERGED_WR0 = "9228217b2abf7314b9dfaecc5fc4323d5f3d7a89"
MERGED_WR0_TREE = "8da6817eba1f259d3565e377fcb50098ab8f3cf2"
MERGED_WR0_MERGE = "8237b96ce7c885edcf4e7a0923f2ac78d05a928d"
ARCHIVE = "52be94316664f88a19df630e164105a0ca50b875"
ARCHIVE_TREE = "21601814bd352114fe2e55833c88a81e47e13e41"
ARCHIVE_PARENT = "3deb414a"
ARCHIVE_PROVENANCE_REF = "refs/wr0a/provenance/archive-final-repair-52be943"

WR0_CONTRACT_SCHEMA = "linux-vst-bridge-wr0-contract-source/v1"
WR0_CONTRACT_DIGEST = "c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541"
WORKLOAD_DIGEST = "4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac"
RUNNER_DIGEST = "2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547"
ENVIRONMENT_DIGEST = "d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4"
TRANSACTION_ID = "wr0-20260901T053317Z-183afd5bbf0736e4"
RETIRED_RECORD_DIGEST = "917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e"
LAUNCH_MODULE_NAME = "_wr0a_readonly_wr0_launch_215718bb"
LAUNCH_PATH = "tools/wr0-proton-bootstrap/launch.py"
LAUNCH_ENTRY = ("100755", "215718bb641765da9163779c0e2145bd02d3198a")

TOOL_PATHS = (
    "tools/wr0a-reconciliation/README.md",
    "tools/wr0a-reconciliation/reconcile.py",
)

EVIDENCE_PATHS = (
    "evidence/wr0a-final-repair-reconciliation/BASIS.md",
    "evidence/wr0a-final-repair-reconciliation/SOURCE_DELTA.md",
    "evidence/wr0a-final-repair-reconciliation/ARCHIVE_AUDIT.md",
    "evidence/wr0a-final-repair-reconciliation/LIVE_READBACK.md",
    "evidence/wr0a-final-repair-reconciliation/GOVERNANCE.md",
    "evidence/wr0a-final-repair-reconciliation/FINDINGS.md",
    "evidence/wr0a-final-repair-reconciliation/fixture.json",
    "evidence/wr0a-final-repair-reconciliation/hashes.sha256",
)

IMPORT_ENTRIES = {
    "docs/WR0_RUNNER_LOCK.md": (("100644", "d2076de6c90f6611ea8ab55426b57a6edf5c8a4c"), ("100644", "94471e033951f735bd2eb792020c7197ce3536c8")),
    "evidence/wr0-proton-bootstrap/BASIS.md": (("100644", "e7259699aa7c66885ca36f6c0d5ab97d8396fa01"), ("100644", "feafc43b8ecc4652b1a524dba68827ff91655ea4")),
    "evidence/wr0-proton-bootstrap/ENVIRONMENT.md": (("100644", "eb927c55940862df8d7f4d22e5f058327b4db4ef"), ("100644", "4bdec8355fff12b1edf5bd9739e9c8d14068dad9")),
    "evidence/wr0-proton-bootstrap/EXIT_PROPAGATION.md": (("100644", "cd15fd2b55437497a3fb39801f919ea7dfb0eca7"), ("100644", "af250cfb53006d55c4808ea7a5b63ca48c9eb4a0")),
    "evidence/wr0-proton-bootstrap/LAUNCH_CONTRACT.md": (("100644", "f155a0d8594f8045869c784eea1d3d96ac80f1a0"), ("100644", "16c207a098f2a813d2dfaec780a12a616174eaaf")),
    "evidence/wr0-proton-bootstrap/NEGATIVE_TESTS.md": (("100644", "ed5486fc3c328a7414218ad3c963751029b88a04"), ("100644", "72a93fbf8e1d2b7ede510329df5a3853be65aa22")),
    "evidence/wr0-proton-bootstrap/PROCESS_GUARDS.md": (("100644", "2e0cb17837cd319ca9e62305eb7be96c228c993a"), ("100644", "0194ba983cb004535752105b066e44df87b622e7")),
    "evidence/wr0-proton-bootstrap/RUN_1.md": (("100644", "7f2dce634201e9fd6117fe6622b5cdc28fd3fb87"), ("100644", "d6bbcdb3dfdcfd993b2ca4ba5cb2cda4aa652a3e")),
    "evidence/wr0-proton-bootstrap/RUN_2.md": (("100644", "b7b1be520334718c5f3964f183fba6d3bb47d1e5"), ("100644", "281379a881c1ea526d6e08d28cbc5ffcb2dd43cd")),
    "evidence/wr0-proton-bootstrap/fixture.json": (("100644", "6f056a66c53c32522875fd831c3bb910b2de30bb"), ("100644", "7919772c25b97da0791095abbb470925feffc58d")),
    "evidence/wr0-proton-bootstrap/hashes.sha256": (("100644", "6863f7415bb0a271b082f57795a3d30a53332d41"), ("100644", "5369c4cb3e55314fd2c2589c9394518b40f5023f")),
    "tools/wr0-proton-bootstrap/README.md": (("100644", "4e2b71a7ae524b3ab76e57db6f15e8595de746d1"), ("100644", "d90180efb2812feb0e510e7f0fb697d08a20c466")),
    "tools/wr0-proton-bootstrap/launch.py": (("100755", "2a5337be3c2d1a330a245289f694b5ca3a18ab66"), ("100755", "215718bb641765da9163779c0e2145bd02d3198a")),
}

IDENTICAL_ENTRIES = {
    "evidence/wr0-proton-bootstrap/FINDINGS.md": ("100644", "a29ffdbbe9124153d827aceb43389176a87c77a7"),
    "evidence/wr0-proton-bootstrap/PRESERVATION.md": ("100644", "5ecca0e22809d1fa3dfe68fa0c5591df7fcba3ba"),
    "evidence/wr0-proton-bootstrap/RUNNER_LOCK.md": ("100644", "fbd74d5706d0eb47afe44f690e2c99d04b89bfce"),
    "evidence/wr0-proton-bootstrap/SANITIZATION.md": ("100644", "913c955d71c99223717fcd6b40cfcdbdfa968e0e"),
    "tools/wr0-proton-bootstrap/common.sh": ("100755", "2e382985402e19b09d71ceaf85078a77f52bab57"),
    "tools/wr0-proton-bootstrap/environment.sh": ("100755", "b5389c36e0d75484b2cdad5f159f8a9cbd1d73e6"),
    "tools/wr0-proton-bootstrap/inspect-runner.sh": ("100755", "e157340eff629127332d94650d4f369ad8688d94"),
    "tools/wr0-proton-bootstrap/negative-tests.sh": ("100755", "b1882ddb3f903a441145866bfb24650ff046a20e"),
    "tools/wr0-proton-bootstrap/preflight.sh": ("100755", "13bceffe6c75830ef6ed0828d32dcd7d7938068b"),
    "tools/wr0-proton-bootstrap/sanitize.sh": ("100755", "44fd2d9ecf369030ed58b7898cdcfb87c2075cfb"),
    "windows-fixtures/wr0-probe/wr0-probe.cmd": ("100644", "04c8707c01c92cfd007fe7f7a72a8adbd4de3588"),
}

EXCLUDED_ENTRY = {
    "path": "CURRENT_SLICE.md",
    "merged": ("100644", "3b2f868ac8ed28095113cafb7dae8ab6eb26cd32"),
    "archive": ("100644", "a4dafdd0e0a94bf15fe683453e733b9eb2383b84"),
    "basis": ("100644", "25091ea21eee2524f107785f5bae9156fdc85b21"),
}

DG0_ENTRIES = {
    "AGENTS.md": ("100644", "4ca16746d4c53341bb5aa330448283bd3e15f791"),
    "GOVERNANCE.md": ("100644", "1918ce7582c436d2f9c13d07acd32cedeb39c502"),
    "README.md": ("100644", "0eabc8b65822444646db7a5ea6323e4f29279df7"),
    "docs/DEVELOPMENT_PROCESS.md": ("100644", "162d9b106a22119946312becae8ec0d4b87cea68"),
    "docs/DECISION_REGISTER.md": ("100644", "a3645c00586b8bd1546e7caca4dd0f34280bb251"),
    "docs/prompts/ADVERSARIAL_DESIGN_REVIEW.md": ("100644", "a78bd60967594f8c5ebdf6fb0d463e9cae50b433"),
    "docs/prompts/CHOOSE_NEXT_SLICE.md": ("100644", "cf0a180c2913a839f541baf7335c2ff0a1973a64"),
    "docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md": ("100644", "8aecb1898c126e6600a0c6a551d9e90a50afc887"),
    "docs/templates/DESIGN_APPROVAL_RECEIPT.md": ("100644", "69bda7d4b32cfbc837b9e7f0df0843bb40e60034"),
    "docs/templates/IMPLEMENTATION_DESIGN_CARD.md": ("100644", "7bc5d812552d55519c2bdb9954c51dcde035403f"),
    "docs/templates/SLICE_SELECTION_RECEIPT.md": ("100644", "e10b4abf338e3d929752e4f7f2bc693b89e0fec8"),
}

AUTHORITY_ENTRIES = {
    "CURRENT_SLICE.md": ("100644", "25091ea21eee2524f107785f5bae9156fdc85b21"),
    "docs/slices/WR0A/SLICE_SELECTION.md": ("100644", "4f3131c93ef7b8178a623032146316226071a10b"),
    "docs/slices/WR0A/RECONNAISSANCE.md": ("100644", "01845a8f39b744f7546d91d2ff349f0b5140e0d9"),
    "docs/slices/WR0A/IMPLEMENTATION_DESIGN.md": ("100644", "18759d2297b3e0bc0e69d0939f72391b373ce9ec"),
    "docs/slices/WR0A/ADVERSARIAL_DESIGN_REVIEW.md": ("100644", "9110f9ef24682f90eb1dca6bb98eb8a8dfdd7bb1"),
    "docs/slices/WR0A/DESIGN_APPROVAL.md": ("100644", "40179957b349390f8484e114b4d23dc14a7c1a13"),
}

WR0_CONTRACT_ENTRIES = {
    "docs/WR0_RUNNER_LOCK.md": ("100644", "94471e033951f735bd2eb792020c7197ce3536c8"),
    "tools/wr0-proton-bootstrap/README.md": ("100644", "d90180efb2812feb0e510e7f0fb697d08a20c466"),
    "tools/wr0-proton-bootstrap/common.sh": ("100755", "2e382985402e19b09d71ceaf85078a77f52bab57"),
    "tools/wr0-proton-bootstrap/environment.sh": ("100755", "b5389c36e0d75484b2cdad5f159f8a9cbd1d73e6"),
    "tools/wr0-proton-bootstrap/inspect-runner.sh": ("100755", "e157340eff629127332d94650d4f369ad8688d94"),
    "tools/wr0-proton-bootstrap/launch.py": ("100755", "215718bb641765da9163779c0e2145bd02d3198a"),
    "tools/wr0-proton-bootstrap/negative-tests.sh": ("100755", "b1882ddb3f903a441145866bfb24650ff046a20e"),
    "tools/wr0-proton-bootstrap/preflight.sh": ("100755", "13bceffe6c75830ef6ed0828d32dcd7d7938068b"),
    "tools/wr0-proton-bootstrap/sanitize.sh": ("100755", "44fd2d9ecf369030ed58b7898cdcfb87c2075cfb"),
    "windows-fixtures/wr0-probe/wr0-probe.cmd": ("100644", "04c8707c01c92cfd007fe7f7a72a8adbd4de3588"),
}

ARCHIVE_EVIDENCE_HASHES = {
    "BASIS.md": "02b354f57f66d91a107136bbfaf1661a7c4f5f14bf97620c1a1bb64d3a422af8",
    "ENVIRONMENT.md": "99882e178da936b372f2898ebe6d76951b197d94d835613dd0e94b9966cf48e9",
    "EXIT_PROPAGATION.md": "0ba0fd3324a2b834ff9277cfe2b9a76d0ee59f9ab71b672cdd6d63d6a24eab2f",
    "FINDINGS.md": "5978db510a421637673f1f4bc8d5bda9d14620862fdd5ec0297ca1e1e1582325",
    "LAUNCH_CONTRACT.md": "3457eda68560ae91ea79fae13dc0c5a5dd098bdc601089310b0c699f0d0fd5dc",
    "NEGATIVE_TESTS.md": "343be77fa265cbf3752986f6b5546d9f68bff3e059f3237d30c4b66f129a4497",
    "PRESERVATION.md": "609be19026560b5a7cff1f481a902806365f0a0039ee930478f7849a49eabaf5",
    "PROCESS_GUARDS.md": "938580080015db06083db202e6122a53f14a72e21cd5328f244c14bbb5c5f45f",
    "RUNNER_LOCK.md": "60f297ae9e8a8776cbda515b3bc4088eae02cb7ccec66fb4ba82d32697578ca1",
    "RUN_1.md": "db68c8577e2c9263ea46fb81c6fd0fd0f7b8a91c26a3aeb2f519da03c0bbff77",
    "RUN_2.md": "b1807d204c1155778c3436d083498784c2ec9b00a03a459c8c3a699e9ae9c2a3",
    "SANITIZATION.md": "be718e1c493431a53f92d483bba2a73916794e20bbb729eafccb2c0754258f08",
    "fixture.json": "338ad6b2f4a812dac53350e5e3ee75459f9f8c3636dce6c126088b539ab85b74",
}

ARCHIVE_HASH_MANIFEST_BLOB = "5369c4cb3e55314fd2c2589c9394518b40f5023f"
ALL_WR0_PATHS = tuple(sorted(set(IMPORT_ENTRIES) | set(IDENTICAL_ENTRIES) | {"CURRENT_SLICE.md"}))
ALLOWED_FINAL_PATHS = frozenset(IMPORT_ENTRIES) | frozenset(TOOL_PATHS) | frozenset(EVIDENCE_PATHS)
FORBIDDEN_PROCESS_KEYS = ("bitwig", "proton", "runtime", "umu", "validator", "wine", "yabridge")
MAX_COMMAND_OUTPUT = 2 * 1024 * 1024
MAX_EVIDENCE_BYTES = 1024 * 1024
SESSION_ROOT_RELATIVE = pathlib.Path(".cache/linux-vst-bridge/wr0a")
TEST_ROOT_RELATIVE = pathlib.Path(".cache/linux-vst-bridge/wr0a-tests")


class WR0ABlocked(RuntimeError):
    def __init__(self, classification: str, detail: str):
        super().__init__(detail)
        self.classification = classification
        self.detail = detail


class InjectedFailure(RuntimeError):
    pass


@dataclasses.dataclass(frozen=True)
class Topology:
    head: str
    head_tree: str
    commits: tuple[str, ...]
    tool_commit: str
    tool_tree: str
    adoption_commit: str | None
    adoption_tree: str | None
    evidence_commit: str | None
    evidence_tree: str | None

    @property
    def state(self) -> str:
        if self.evidence_commit:
            return "final"
        if self.adoption_commit:
            return "adoption"
        return "tool"


def block(classification: str, detail: str) -> None:
    raise WR0ABlocked(classification, detail)


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def run(
    command: Sequence[str],
    *,
    cwd: pathlib.Path,
    input_bytes: bytes | None = None,
    check: bool = True,
    timeout: float = 30.0,
) -> subprocess.CompletedProcess[bytes]:
    try:
        result = subprocess.run(
            list(command),
            cwd=cwd,
            input=input_bytes,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        block("WR0A_COMMAND_BLOCKED", f"bounded command timed out: {pathlib.Path(command[0]).name}")
        raise AssertionError from exc
    if len(result.stdout) > MAX_COMMAND_OUTPUT or len(result.stderr) > MAX_COMMAND_OUTPUT:
        block("WR0A_COMMAND_BLOCKED", f"bounded command output exceeded cap: {pathlib.Path(command[0]).name}")
    if check and result.returncode:
        stderr = result.stderr.decode("utf-8", "replace").strip()[:500]
        block("WR0A_COMMAND_BLOCKED", f"{pathlib.Path(command[0]).name} exited {result.returncode}: {stderr}")
    return result


def git(repo: pathlib.Path, *arguments: str, input_bytes: bytes | None = None, check: bool = True) -> bytes:
    return run(("git", *arguments), cwd=repo, input_bytes=input_bytes, check=check).stdout


def git_text(repo: pathlib.Path, *arguments: str) -> str:
    try:
        return git(repo, *arguments).decode("utf-8", "strict").strip()
    except UnicodeDecodeError as exc:
        block("WR0A_GIT_IDENTITY_BLOCKED", "Git returned non-UTF-8 identity output")
        raise AssertionError from exc


def repo_root() -> pathlib.Path:
    script = pathlib.Path(__file__)
    if script.is_symlink():
        block("ADOPTION_TOOL_SOURCE_MISMATCH", "reconciler path is a symlink")
    root = pathlib.Path(git_text(script.parent, "rev-parse", "--show-toplevel"))
    if not root.is_absolute() or root.resolve() != root:
        block("WR0A_BASIS_MISMATCH", "repository root is not canonical")
    expected_script = root / "tools/wr0a-reconciliation/reconcile.py"
    if script.resolve() != expected_script:
        block("ADOPTION_TOOL_SOURCE_MISMATCH", "reconciler is not running from its tracked canonical path")
    return root


def tree_entry(repo: pathlib.Path, revision: str, path: str) -> tuple[str, str] | None:
    raw = git(repo, "ls-tree", "-z", revision, "--", path)
    if not raw:
        return None
    records = [item for item in raw.split(b"\0") if item]
    if len(records) != 1:
        block("WR0A_GIT_IDENTITY_BLOCKED", f"unexpected tree entry count for {path}")
    match = re.fullmatch(rb"([0-9]{6}) blob ([0-9a-f]{40,64})\t(.+)", records[0])
    if match is None or match.group(3).decode("utf-8", "strict") != path:
        block("WR0A_GIT_IDENTITY_BLOCKED", f"malformed tree entry for {path}")
    return match.group(1).decode("ascii"), match.group(2).decode("ascii")


def index_entry(repo: pathlib.Path, path: str) -> tuple[str, str] | None:
    raw = git(repo, "ls-files", "-s", "-z", "--", path)
    if not raw:
        return None
    records = [item for item in raw.split(b"\0") if item]
    if len(records) != 1:
        block("WR0A_GIT_IDENTITY_BLOCKED", f"unexpected index entry count for {path}")
    match = re.fullmatch(rb"([0-9]{6}) ([0-9a-f]{40,64}) 0\t(.+)", records[0])
    if match is None or match.group(3).decode("utf-8", "strict") != path:
        block("WR0A_GIT_IDENTITY_BLOCKED", f"malformed or unmerged index entry for {path}")
    return match.group(1).decode("ascii"), match.group(2).decode("ascii")


def working_entry(repo: pathlib.Path, path: str) -> tuple[str, str] | None:
    absolute = repo / path
    try:
        info = absolute.lstat()
    except FileNotFoundError:
        return None
    if not stat.S_ISREG(info.st_mode) or stat.S_ISLNK(info.st_mode):
        block("WR0A_GIT_IDENTITY_BLOCKED", f"non-regular governed path: {path}")
    mode = "100755" if info.st_mode & 0o111 else "100644"
    blob = git_text(repo, "hash-object", "--no-filters", "--", path)
    return mode, blob


def changed_paths(repo: pathlib.Path, old: str, new: str) -> frozenset[str]:
    raw = git(repo, "diff", "--name-only", "-z", old, new, "--")
    try:
        return frozenset(item.decode("utf-8", "strict") for item in raw.split(b"\0") if item)
    except UnicodeDecodeError as exc:
        block("WR0A_GIT_IDENTITY_BLOCKED", "changed path is not UTF-8")
        raise AssertionError from exc


def staged_paths(repo: pathlib.Path) -> frozenset[str]:
    raw = git(repo, "diff", "--cached", "--name-only", "-z", "--")
    return frozenset(item.decode("utf-8", "strict") for item in raw.split(b"\0") if item)


def unstaged_paths(repo: pathlib.Path) -> frozenset[str]:
    raw = git(repo, "diff", "--name-only", "-z", "--")
    return frozenset(item.decode("utf-8", "strict") for item in raw.split(b"\0") if item)


def untracked_paths(repo: pathlib.Path) -> frozenset[str]:
    raw = git(repo, "ls-files", "--others", "--exclude-standard", "-z", "--")
    return frozenset(item.decode("utf-8", "strict") for item in raw.split(b"\0") if item)


def require_clean(repo: pathlib.Path, classification: str = "WR0A_WORKTREE_STATE_BLOCKED") -> None:
    if staged_paths(repo) or unstaged_paths(repo) or untracked_paths(repo):
        block(classification, "worktree, index, or untracked state is not clean")


def require_entries(repo: pathlib.Path, revision: str, entries: dict[str, tuple[str, str]], classification: str) -> None:
    for path, expected in entries.items():
        if tree_entry(repo, revision, path) != expected:
            block(classification, f"path identity differs: {path}")


def object_identity(repo: pathlib.Path, revision: str, tree: str, parent_prefix: str | None = None) -> None:
    if git_text(repo, "rev-parse", f"{revision}^{{commit}}") != revision:
        block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", f"commit object differs: {revision[:12]}")
    if git_text(repo, "rev-parse", f"{revision}^{{tree}}") != tree:
        block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", f"tree object differs: {revision[:12]}")
    if parent_prefix is not None:
        parent = git_text(repo, "rev-parse", f"{revision}^")
        if not parent.startswith(parent_prefix):
            block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", f"parent object differs: {revision[:12]}")


def commit_delta(repo: pathlib.Path, commit: str) -> frozenset[str]:
    parent = git_text(repo, "rev-parse", f"{commit}^")
    return changed_paths(repo, parent, commit)


def topology(repo: pathlib.Path) -> Topology:
    if git_text(repo, "branch", "--show-current") != EXPECTED_BRANCH:
        block("WR0A_BASIS_MISMATCH", "implementation branch differs")
    object_identity(repo, IMPLEMENTATION_BASIS, IMPLEMENTATION_BASIS_TREE)
    head = git_text(repo, "rev-parse", "HEAD")
    commits_raw = git(repo, "rev-list", "--reverse", f"{IMPLEMENTATION_BASIS}..{head}")
    commits = tuple(item.decode("ascii") for item in commits_raw.splitlines() if item)
    if not 1 <= len(commits) <= 3:
        block("FINAL_HEAD_SOURCE_MISMATCH", "implementation topology is not one to three commits")
    if git_text(repo, "rev-parse", f"{commits[0]}^") != IMPLEMENTATION_BASIS:
        block("ADOPTION_TOOL_SOURCE_MISMATCH", "tool commit is not the direct basis child")
    if commit_delta(repo, commits[0]) != frozenset(TOOL_PATHS):
        block("ADOPTION_TOOL_SOURCE_MISMATCH", "first commit is not the exact two-tool delta")
    if len(commits) >= 2:
        if git_text(repo, "rev-parse", f"{commits[1]}^") != commits[0]:
            block("WR0A_BLOB_IMPORT_BLOCKED", "adoption commit parent differs")
        if commit_delta(repo, commits[1]) != frozenset(IMPORT_ENTRIES):
            block("WR0A_BLOB_IMPORT_BLOCKED", "second commit is not the exact thirteen-path delta")
    if len(commits) == 3:
        if git_text(repo, "rev-parse", f"{commits[2]}^") != commits[1]:
            block("FINAL_HEAD_SOURCE_MISMATCH", "evidence commit parent differs")
        if commit_delta(repo, commits[2]) != frozenset(EVIDENCE_PATHS):
            block("FINAL_HEAD_SOURCE_MISMATCH", "third commit is not the exact eight-evidence delta")
    return Topology(
        head=head,
        head_tree=git_text(repo, "rev-parse", "HEAD^{tree}"),
        commits=commits,
        tool_commit=commits[0],
        tool_tree=git_text(repo, "rev-parse", f"{commits[0]}^{{tree}}"),
        adoption_commit=commits[1] if len(commits) >= 2 else None,
        adoption_tree=git_text(repo, "rev-parse", f"{commits[1]}^{{tree}}") if len(commits) >= 2 else None,
        evidence_commit=commits[2] if len(commits) == 3 else None,
        evidence_tree=git_text(repo, "rev-parse", f"{commits[2]}^{{tree}}") if len(commits) == 3 else None,
    )


def source_manifest(repo: pathlib.Path, topo: Topology) -> dict[str, Any]:
    files = []
    for path in sorted(TOOL_PATHS):
        entry = tree_entry(repo, topo.tool_commit, path)
        if entry is None or tree_entry(repo, "HEAD", path) != entry:
            block("ADOPTION_TOOL_SOURCE_MISMATCH", f"tool source changed after its commit: {path}")
        if index_entry(repo, path) != entry or working_entry(repo, path) != entry:
            block("ADOPTION_TOOL_SOURCE_MISMATCH", f"tool source is not exact in index/worktree: {path}")
        files.append({"path": path, "mode": entry[0], "blob": entry[1]})
    manifest = {"schema": SOURCE_SCHEMA, "files": files}
    return {
        "schema": SOURCE_SCHEMA,
        "files": files,
        "file_count": len(files),
        "digest": sha256_bytes(canonical_json(manifest)),
        "tool_commit": topo.tool_commit,
        "tool_tree": topo.tool_tree,
    }


def verify_archive_packet_from_object(repo: pathlib.Path) -> dict[str, Any]:
    prefix = "evidence/wr0-proton-bootstrap"
    expected_names = frozenset(ARCHIVE_EVIDENCE_HASHES) | {"hashes.sha256"}
    raw = git(repo, "ls-tree", "-r", "-z", "--name-only", ARCHIVE, "--", prefix)
    names = frozenset(
        pathlib.PurePosixPath(item.decode("utf-8", "strict")).name
        for item in raw.split(b"\0")
        if item
    )
    if names != expected_names:
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "archive evidence roster differs")
    for name, expected_hash in ARCHIVE_EVIDENCE_HASHES.items():
        data = git(repo, "show", f"{ARCHIVE}:{prefix}/{name}")
        if sha256_bytes(data) != expected_hash:
            block("WR0A_EVIDENCE_ADOPTION_BLOCKED", f"archive evidence hash differs: {name}")
    entry = tree_entry(repo, ARCHIVE, f"{prefix}/hashes.sha256")
    if entry != ("100644", ARCHIVE_HASH_MANIFEST_BLOB):
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "archive hash-manifest blob differs")
    expected_lines = [f"{digest}  {name}" for name, digest in sorted(ARCHIVE_EVIDENCE_HASHES.items())]
    actual_lines = git(repo, "show", f"{ARCHIVE}:{prefix}/hashes.sha256").decode("utf-8", "strict").splitlines()
    if actual_lines != expected_lines:
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "archive hash-manifest content differs")
    fixture = json.loads(git(repo, "show", f"{ARCHIVE}:{prefix}/fixture.json").decode("utf-8", "strict"))
    if (
        fixture.get("claim", {}).get("name") != "WR0_COMPLETE"
        or fixture.get("negative_tests", {}).get("passed_count") != 80
        or fixture.get("negative_tests", {}).get("failed_count") != 0
        or fixture.get("negative_tests", {}).get("live_production_cases") != 5
        or fixture.get("contract_source", {}).get("digest") != WR0_CONTRACT_DIGEST
        or fixture.get("environment", {}).get("environment_identity_sha256") != ENVIRONMENT_DIGEST
        or fixture.get("runner", {}).get("manifest_sha256") != RUNNER_DIGEST
        or fixture.get("replacement", {}).get("durable_commit_record", {}).get("final_sha256") != RETIRED_RECORD_DIGEST
    ):
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "archive fixture semantic identities differ")
    return {
        "classification": "passed",
        "roster_count": len(expected_names),
        "payload_hash_count": len(ARCHIVE_EVIDENCE_HASHES),
        "hash_manifest_blob": ARCHIVE_HASH_MANIFEST_BLOB,
        "retained_tests": {
            "passed": 80,
            "failed": 0,
            "non_live": 75,
            "live": 5,
            "final_repair_helper_cases": 10,
        },
    }


def verify_authority_records(repo: pathlib.Path, revision: str = "HEAD") -> dict[str, Any]:
    require_entries(repo, revision, DG0_ENTRIES, "WR0A_GOVERNANCE_DRIFT_BLOCKED")
    require_entries(repo, revision, AUTHORITY_ENTRIES, "WR0A_AUTHORITY_MISMATCH")
    return {
        "classification": "passed",
        "dg0_count": len(DG0_ENTRIES),
        "authority_count": len(AUTHORITY_ENTRIES),
        "current_slice": {"mode": EXCLUDED_ENTRY["basis"][0], "blob": EXCLUDED_ENTRY["basis"][1]},
    }


def verify_provenance_ref(repo: pathlib.Path) -> None:
    result = run(("git", "rev-parse", "--verify", "-q", ARCHIVE_PROVENANCE_REF), cwd=repo, check=False)
    if result.returncode == 0:
        value = result.stdout.decode("ascii").strip()
        if value != ARCHIVE:
            block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", "existing local provenance ref points elsewhere")
    elif result.returncode == 1:
        update = run(
            ("git", "update-ref", ARCHIVE_PROVENANCE_REF, ARCHIVE, "0" * 40),
            cwd=repo,
            check=False,
        )
        if update.returncode:
            block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", "compare-and-set provenance ref creation failed")
    else:
        block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", "local provenance ref read failed")
    if git_text(repo, "rev-parse", ARCHIVE_PROVENANCE_REF) != ARCHIVE:
        block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", "local provenance ref readback differs")


def verify_plan(repo: pathlib.Path) -> dict[str, Any]:
    if len(ALL_WR0_PATHS) != 25 or len(IMPORT_ENTRIES) != 13 or len(IDENTICAL_ENTRIES) != 11:
        block("ADOPTION_PLAN_MISMATCH", "compiled reconciliation cardinality differs")
    merged_envelope = commit_delta(repo, MERGED_WR0)
    if merged_envelope != frozenset(ALL_WR0_PATHS):
        block("ADOPTION_PLAN_MISMATCH", "merged WR0 25-path envelope differs")
    differences: list[dict[str, Any]] = []
    preserved: list[dict[str, Any]] = []
    for path, (old, new) in sorted(IMPORT_ENTRIES.items()):
        if tree_entry(repo, MERGED_WR0, path) != old or tree_entry(repo, ARCHIVE, path) != new:
            block("ADOPTION_PLAN_MISMATCH", f"import ledger object differs: {path}")
        if tree_entry(repo, IMPLEMENTATION_BASIS, path) != old:
            block("ADOPTION_PLAN_MISMATCH", f"implementation-basis old object differs: {path}")
        differences.append(
            {
                "path": path,
                "merged": {"mode": old[0], "blob": old[1]},
                "archive": {"mode": new[0], "blob": new[1]},
                "adopt": True,
            }
        )
    for path, entry in sorted(IDENTICAL_ENTRIES.items()):
        if tree_entry(repo, MERGED_WR0, path) != entry or tree_entry(repo, ARCHIVE, path) != entry:
            block("ADOPTION_PLAN_MISMATCH", f"preserved ledger object differs: {path}")
        if tree_entry(repo, IMPLEMENTATION_BASIS, path) != entry:
            block("ADOPTION_PLAN_MISMATCH", f"implementation-basis preserved object differs: {path}")
        preserved.append({"path": path, "mode": entry[0], "blob": entry[1]})
    if (
        tree_entry(repo, MERGED_WR0, EXCLUDED_ENTRY["path"]) != EXCLUDED_ENTRY["merged"]
        or tree_entry(repo, ARCHIVE, EXCLUDED_ENTRY["path"]) != EXCLUDED_ENTRY["archive"]
        or tree_entry(repo, IMPLEMENTATION_BASIS, EXCLUDED_ENTRY["path"]) != EXCLUDED_ENTRY["basis"]
    ):
        block("ADOPTION_PLAN_MISMATCH", "excluded CURRENT_SLICE ledger differs")
    differences.insert(
        0,
        {
            "path": EXCLUDED_ENTRY["path"],
            "merged": {"mode": EXCLUDED_ENTRY["merged"][0], "blob": EXCLUDED_ENTRY["merged"][1]},
            "archive": {"mode": EXCLUDED_ENTRY["archive"][0], "blob": EXCLUDED_ENTRY["archive"][1]},
            "adopt": False,
            "candidate": {"mode": EXCLUDED_ENTRY["basis"][0], "blob": EXCLUDED_ENTRY["basis"][1]},
        },
    )
    return {
        "classification": "passed",
        "path_count": 25,
        "difference_count": 14,
        "import_count": 13,
        "preserved_count": 11,
        "excluded_count": 1,
        "differences": differences,
        "preserved": preserved,
        "excluded_path": EXCLUDED_ENTRY["path"],
    }


def verify_authority(repo: pathlib.Path, *, create_provenance: bool = True) -> dict[str, Any]:
    topo = topology(repo)
    source = source_manifest(repo, topo)
    object_identity(repo, MERGED_WR0, MERGED_WR0_TREE)
    object_identity(repo, ARCHIVE, ARCHIVE_TREE, ARCHIVE_PARENT)
    if git_text(repo, "rev-parse", f"{MERGED_WR0_MERGE}^2") != MERGED_WR0:
        block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", "merged WR0 merge relationship differs")
    authority = verify_authority_records(repo)
    plan = verify_plan(repo)
    archive_packet = verify_archive_packet_from_object(repo)
    if create_provenance:
        verify_provenance_ref(repo)
    return {
        "schema": SCHEMA,
        "classification": "passed",
        "mode": "verify-authority",
        "basis": {"commit": IMPLEMENTATION_BASIS, "tree": IMPLEMENTATION_BASIS_TREE},
        "topology": dataclasses.asdict(topo),
        "source": source,
        "archive": {"commit": ARCHIVE, "tree": ARCHIVE_TREE, "parent_prefix": ARCHIVE_PARENT},
        "merged_wr0": {"commit": MERGED_WR0, "tree": MERGED_WR0_TREE, "merge": MERGED_WR0_MERGE},
        "authority": authority,
        "plan_summary": {key: plan[key] for key in ("path_count", "difference_count", "import_count", "preserved_count", "excluded_count")},
        "archive_packet": archive_packet,
    }


def contract_source_manifest(repo: pathlib.Path) -> dict[str, Any]:
    files = []
    for path, expected in WR0_CONTRACT_ENTRIES.items():
        entry = index_entry(repo, path)
        if entry != expected or working_entry(repo, path) != expected:
            block("WR0A_CONTRACT_SOURCE_BLOCKED", f"contract-source path differs: {path}")
        files.append({"path": path, "mode": expected[0], "blob": expected[1]})
    files.sort(key=lambda item: item["path"])
    manifest = {"schema": WR0_CONTRACT_SCHEMA, "files": files}
    digest = sha256_bytes(canonical_json(manifest))
    workload = repo / "windows-fixtures/wr0-probe/wr0-probe.cmd"
    if digest != WR0_CONTRACT_DIGEST or sha256_file(workload) != WORKLOAD_DIGEST:
        block("WR0A_CONTRACT_SOURCE_BLOCKED", "WR0 contract-source or workload digest differs")
    return {
        "classification": "passed",
        "schema": WR0_CONTRACT_SCHEMA,
        "digest": digest,
        "file_count": len(files),
        "workload_sha256": WORKLOAD_DIGEST,
        "files": files,
    }


def verify_import_state(repo: pathlib.Path, *, staged: bool) -> dict[str, Any]:
    for path, (_old, new) in IMPORT_ENTRIES.items():
        expected_head = _old if staged else new
        if tree_entry(repo, "HEAD", path) != expected_head:
            block("WR0A_BLOB_IMPORT_BLOCKED", f"HEAD import state differs: {path}")
        if index_entry(repo, path) != new or working_entry(repo, path) != new:
            block("WR0A_BLOB_IMPORT_BLOCKED", f"candidate import state differs: {path}")
    for path, entry in IDENTICAL_ENTRIES.items():
        if tree_entry(repo, "HEAD", path) != entry or index_entry(repo, path) != entry or working_entry(repo, path) != entry:
            block("WR0A_BLOB_IMPORT_BLOCKED", f"preserved candidate path differs: {path}")
    if (
        tree_entry(repo, "HEAD", EXCLUDED_ENTRY["path"]) != EXCLUDED_ENTRY["basis"]
        or index_entry(repo, EXCLUDED_ENTRY["path"]) != EXCLUDED_ENTRY["basis"]
        or working_entry(repo, EXCLUDED_ENTRY["path"]) != EXCLUDED_ENTRY["basis"]
    ):
        block("WR0A_BLOB_IMPORT_BLOCKED", "current authority was changed or archive authority imported")
    if staged:
        if staged_paths(repo) != frozenset(IMPORT_ENTRIES) or unstaged_paths(repo) or untracked_paths(repo):
            block("WR0A_BLOB_IMPORT_BLOCKED", "staged candidate has an extra or missing path")
    else:
        require_clean(repo, "WR0A_BLOB_IMPORT_BLOCKED")
    return {
        "classification": "passed",
        "state": "staged" if staged else "committed",
        "import_count": len(IMPORT_ENTRIES),
        "preserved_count": len(IDENTICAL_ENTRIES),
        "excluded_current_slice": True,
    }


def validate_current_archive_packet(repo: pathlib.Path) -> dict[str, Any]:
    root = repo / "evidence/wr0-proton-bootstrap"
    names = frozenset(path.name for path in root.iterdir())
    expected = frozenset(ARCHIVE_EVIDENCE_HASHES) | {"hashes.sha256"}
    if names != expected:
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "candidate canonical WR0 packet roster differs")
    for name, expected_hash in ARCHIVE_EVIDENCE_HASHES.items():
        if sha256_file(root / name) != expected_hash:
            block("WR0A_EVIDENCE_ADOPTION_BLOCKED", f"candidate canonical evidence differs: {name}")
    if index_entry(repo, "evidence/wr0-proton-bootstrap/hashes.sha256") != ("100644", ARCHIVE_HASH_MANIFEST_BLOB):
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "candidate canonical hash-manifest object differs")
    fixture = json.loads((root / "fixture.json").read_text(encoding="utf-8"))
    if fixture.get("environment", {}).get("environment_identity_sha256") != ENVIRONMENT_DIGEST:
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "candidate fixture environment identity differs")
    return {"classification": "passed", "roster_count": 14, "payload_hash_count": 13}


def session_root(*, test: bool = False) -> pathlib.Path:
    relative = TEST_ROOT_RELATIVE if test else SESSION_ROOT_RELATIVE
    root = pathlib.Path.home() / relative
    if root.exists():
        info = root.lstat()
        if not stat.S_ISDIR(info.st_mode) or stat.S_ISLNK(info.st_mode) or info.st_uid != os.getuid():
            block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session root is unsafe")
        os.chmod(root, 0o700)
    else:
        root.mkdir(parents=True, mode=0o700)
    if root.resolve() != root:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session root is not canonical")
    return root


def list_sessions(root: pathlib.Path) -> list[pathlib.Path]:
    leaves = sorted(root.iterdir(), key=lambda path: path.name)
    if len(leaves) > 2:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session leaf count exceeds bound")
    for leaf in leaves:
        if not re.fullmatch(r"session-[0-9a-f]{32}", leaf.name) or leaf.is_symlink() or not leaf.is_dir():
            block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "unknown transient-cache object")
    return leaves


def fsync_directory(path: pathlib.Path) -> None:
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def atomic_write(path: pathlib.Path, data: bytes, mode: int = 0o600) -> None:
    temporary = path.parent / f".{path.name}.tmp-{secrets.token_hex(8)}"
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    try:
        with os.fdopen(descriptor, "wb", closefd=True) as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        fsync_directory(path.parent)
    except Exception:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass
        raise


def write_marker(leaf: pathlib.Path, marker: dict[str, Any]) -> None:
    marker_path = leaf / "marker.json"
    atomic_write(marker_path, json.dumps(marker, indent=2, sort_keys=True).encode("utf-8") + b"\n")


def read_marker(leaf: pathlib.Path) -> dict[str, Any]:
    if frozenset(path.name for path in leaf.iterdir()) != {"marker.json"}:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session payload roster differs")
    marker_path = leaf / "marker.json"
    if marker_path.is_symlink() or marker_path.stat().st_size > 64 * 1024:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session marker is unsafe")
    try:
        marker = json.loads(marker_path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session marker is malformed")
        raise AssertionError from exc
    if marker.get("schema") != SESSION_SCHEMA or marker.get("nonce") != leaf.name.removeprefix("session-"):
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session marker binding differs")
    return marker


def create_session(repo: pathlib.Path, topo: Topology, source: dict[str, Any], mode: str, target: str | None) -> tuple[pathlib.Path, dict[str, Any]]:
    root = session_root()
    if list_sessions(root):
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "an unresolved WR0A session already exists")
    nonce = secrets.token_hex(16)
    leaf = root / f"session-{nonce}"
    leaf.mkdir(mode=0o700)
    fsync_directory(root)
    marker = {
        "schema": SESSION_SCHEMA,
        "nonce": nonce,
        "mode": mode,
        "implementation_basis_commit": IMPLEMENTATION_BASIS,
        "implementation_basis_tree": IMPLEMENTATION_BASIS_TREE,
        "tool_commit": topo.tool_commit,
        "tool_tree": topo.tool_tree,
        "tool_source_digest": source["digest"],
        "adoption_commit": topo.adoption_commit,
        "adoption_tree": topo.adoption_tree,
        "worktree_identity_sha256": sha256_bytes(str(repo).encode("utf-8")),
        "target": target,
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
        "phase": "SESSION_MARKER_COMMITTED",
    }
    write_marker(leaf, marker)
    return leaf, marker


def cleanup_session(leaf: pathlib.Path, marker: dict[str, Any]) -> None:
    observed = read_marker(leaf)
    if observed != marker:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session marker changed before cleanup")
    (leaf / "marker.json").unlink()
    fsync_directory(leaf)
    leaf.rmdir()
    fsync_directory(leaf.parent)
    if leaf.exists():
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "session leaf remained after cleanup")


def object_bytes(repo: pathlib.Path, blob: str) -> bytes:
    return git(repo, "cat-file", "blob", blob)


def write_entry(repo: pathlib.Path, path: str, entry: tuple[str, str], data: bytes) -> None:
    if sha256_bytes(b"") == entry[1]:
        raise AssertionError
    absolute = repo / path
    if absolute.parent.resolve() != absolute.parent:
        block("WR0A_BLOB_IMPORT_BLOCKED", f"destination parent is not canonical: {path}")
    atomic_write(absolute, data, 0o755 if entry[0] == "100755" else 0o644)
    os.chmod(absolute, 0o755 if entry[0] == "100755" else 0o644)
    with absolute.open("rb") as handle:
        os.fsync(handle.fileno())
    fsync_directory(absolute.parent)
    if working_entry(repo, path) != entry:
        block("WR0A_BLOB_IMPORT_BLOCKED", f"physical entry readback differs: {path}")


def verify_entry_objects(repo: pathlib.Path, entries: dict[str, tuple[tuple[str, str], tuple[str, str]]]) -> dict[str, tuple[bytes, bytes]]:
    content: dict[str, tuple[bytes, bytes]] = {}
    for path, (old, new) in entries.items():
        old_bytes = object_bytes(repo, old[1])
        new_bytes = object_bytes(repo, new[1])
        old_hash = git(repo, "hash-object", "--stdin", input_bytes=old_bytes).decode("ascii").strip()
        new_hash = git(repo, "hash-object", "--stdin", input_bytes=new_bytes).decode("ascii").strip()
        if old_hash != old[1] or new_hash != new[1]:
            block("WR0A_ARCHIVE_PROVENANCE_BLOCKED", f"blob readback differs: {path}")
        content[path] = (old_bytes, new_bytes)
    return content


def recover_adoption(
    repo: pathlib.Path,
    entries: dict[str, tuple[tuple[str, str], tuple[str, str]]],
    content: dict[str, tuple[bytes, bytes]],
) -> None:
    physical: dict[str, tuple[str, str] | None] = {}
    indexed: dict[str, tuple[str, str] | None] = {}
    for path, (old, new) in entries.items():
        physical[path] = working_entry(repo, path)
        indexed[path] = index_entry(repo, path)
        if physical[path] not in {old, new} or indexed[path] not in {old, new}:
            block("WR0A_BLOB_IMPORT_RECOVERY_BLOCKED", f"unknown adoption state preserved: {path}")
    for path, (old, _new) in entries.items():
        write_entry(repo, path, old, content[path][0])
    for path, (old, _new) in entries.items():
        git(repo, "update-index", "--cacheinfo", f"{old[0]},{old[1]},{path}")
        if index_entry(repo, path) != old:
            block("WR0A_BLOB_IMPORT_RECOVERY_BLOCKED", f"old index entry was not restored: {path}")
    if staged_paths(repo) or unstaged_paths(repo) or untracked_paths(repo):
        block("WR0A_BLOB_IMPORT_RECOVERY_BLOCKED", "adoption recovery did not restore a clean repository")


def perform_adoption(
    repo: pathlib.Path,
    entries: dict[str, tuple[tuple[str, str], tuple[str, str]]],
    *,
    failure_phase: str | None = None,
    failure_number: int | None = None,
    progress: Any = None,
) -> None:
    content = verify_entry_objects(repo, entries)
    for path, (old, _new) in entries.items():
        if index_entry(repo, path) != old or working_entry(repo, path) != old:
            block("WR0A_BLOB_IMPORT_BLOCKED", f"old destination/index entry differs: {path}")
    try:
        for number, (path, (_old, new)) in enumerate(entries.items(), 1):
            absolute = repo / path
            atomic_write(absolute, content[path][1], 0o755 if new[0] == "100755" else 0o644)
            if progress:
                progress("write", number, path)
            if failure_phase == "write" and failure_number == number:
                raise InjectedFailure(f"after write {number}")
            expected_blob = git_text(repo, "hash-object", "--no-filters", "--", path)
            if expected_blob != new[1]:
                block("WR0A_BLOB_IMPORT_BLOCKED", f"archive bytes readback differs: {path}")
        for number, (path, (_old, new)) in enumerate(entries.items(), 1):
            absolute = repo / path
            os.chmod(absolute, 0o755 if new[0] == "100755" else 0o644)
            with absolute.open("rb") as handle:
                os.fsync(handle.fileno())
            fsync_directory(absolute.parent)
            if progress:
                progress("mode", number, path)
            if failure_phase == "mode" and failure_number == number:
                raise InjectedFailure(f"after mode {number}")
            if working_entry(repo, path) != new:
                block("WR0A_BLOB_IMPORT_BLOCKED", f"archive mode readback differs: {path}")
        for number, (path, (_old, new)) in enumerate(entries.items(), 1):
            git(repo, "add", "--", path)
            if progress:
                progress("index", number, path)
            if failure_phase == "index" and failure_number == number:
                raise InjectedFailure(f"after index {number}")
            if index_entry(repo, path) != new:
                block("WR0A_BLOB_IMPORT_BLOCKED", f"archive index readback differs: {path}")
    except Exception:
        recover_adoption(repo, entries, content)
        raise
    for path, (_old, new) in entries.items():
        if index_entry(repo, path) != new or working_entry(repo, path) != new:
            block("WR0A_BLOB_IMPORT_BLOCKED", f"final staged archive entry differs: {path}")


def recover_existing_adoption_session(repo: pathlib.Path, topo: Topology, source: dict[str, Any]) -> bool:
    root = session_root()
    leaves = list_sessions(root)
    if not leaves:
        return False
    if len(leaves) != 1:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "multiple WR0A sessions are unresolved")
    leaf = leaves[0]
    marker = read_marker(leaf)
    expected_binding = (
        marker.get("mode") == "stage-adoption"
        and marker.get("tool_commit") == topo.tool_commit
        and marker.get("tool_source_digest") == source["digest"]
        and marker.get("implementation_basis_commit") == IMPLEMENTATION_BASIS
        and marker.get("worktree_identity_sha256") == sha256_bytes(str(repo).encode("utf-8"))
    )
    if not expected_binding:
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "existing adoption session belongs to another identity")
    if marker.get("phase") == "ARCHIVE_BLOBS_STAGED":
        if topo.state == "adoption" and topo.adoption_commit:
            verify_import_state(repo, staged=False)
            marker["adoption_commit"] = topo.adoption_commit
            marker["adoption_tree"] = topo.adoption_tree
            marker["phase"] = "ADOPTION_COMMIT_VERIFIED"
            write_marker(leaf, marker)
            cleanup_session(leaf, marker)
            return True
        if topo.state == "tool" and staged_paths(repo) == frozenset(IMPORT_ENTRIES):
            return True
    content = verify_entry_objects(repo, IMPORT_ENTRIES)
    recover_adoption(repo, IMPORT_ENTRIES, content)
    marker["phase"] = "ADOPTION_RECOVERED"
    write_marker(leaf, marker)
    cleanup_session(leaf, marker)
    return True


def stage_adoption(repo: pathlib.Path) -> dict[str, Any]:
    authority = verify_authority(repo)
    topo = topology(repo)
    source = source_manifest(repo, topo)
    if topo.state != "tool" or topo.head != topo.tool_commit:
        block("WR0A_BLOB_IMPORT_BLOCKED", "stage-adoption requires the exact tool commit")
    recovered = recover_existing_adoption_session(repo, topo, source)
    if recovered and staged_paths(repo) == frozenset(IMPORT_ENTRIES):
        return {
            "schema": SCHEMA,
            "classification": "passed",
            "mode": "stage-adoption",
            "state": "archive_blobs_staged",
            "recovered_existing_session": True,
            "import_count": 13,
            "tool_commit": topo.tool_commit,
            "source_digest": source["digest"],
        }
    require_clean(repo, "WR0A_BLOB_IMPORT_BLOCKED")
    verify_plan(repo)
    leaf, marker = create_session(repo, topo, source, "stage-adoption", None)
    marker["phase"] = "ADOPTION_IN_PROGRESS"
    marker["original_index_tree"] = git_text(repo, "write-tree")
    write_marker(leaf, marker)

    def progress(phase: str, number: int, path: str) -> None:
        marker["operation"] = phase
        marker["operation_number"] = number
        marker["operation_path"] = path
        write_marker(leaf, marker)

    try:
        perform_adoption(repo, IMPORT_ENTRIES, progress=progress)
        verify_import_state(repo, staged=True)
        marker["phase"] = "ARCHIVE_BLOBS_STAGED"
        marker.pop("operation", None)
        marker.pop("operation_number", None)
        marker.pop("operation_path", None)
        write_marker(leaf, marker)
    except Exception as exc:
        marker["phase"] = "ADOPTION_RECOVERED"
        marker.pop("operation", None)
        marker.pop("operation_number", None)
        marker.pop("operation_path", None)
        write_marker(leaf, marker)
        cleanup_session(leaf, marker)
        if isinstance(exc, WR0ABlocked):
            raise
        block("WR0A_BLOB_IMPORT_BLOCKED", f"exact-object adoption failed and was recovered: {type(exc).__name__}")
    return {
        "schema": SCHEMA,
        "classification": "passed",
        "mode": "stage-adoption",
        "state": "archive_blobs_staged",
        "session_nonce": marker["nonce"],
        "import_count": 13,
        "tool_commit": topo.tool_commit,
        "source_digest": source["digest"],
        "authority_digest": sha256_bytes(canonical_json(authority["authority"])),
    }


def validate_wr0a_packet(repo: pathlib.Path, *, require_index: bool) -> dict[str, Any]:
    root = repo / "evidence/wr0a-final-repair-reconciliation"
    expected_names = frozenset(pathlib.PurePosixPath(path).name for path in EVIDENCE_PATHS)
    if not root.is_dir() or root.is_symlink() or frozenset(path.name for path in root.iterdir()) != expected_names:
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A evidence roster differs")
    total = 0
    for path in root.iterdir():
        if path.is_symlink() or not path.is_file():
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A evidence contains a non-regular object")
        data = path.read_bytes()
        total += len(data)
        if len(data) > MAX_EVIDENCE_BYTES or b"\0" in data:
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A evidence size or NUL bound failed")
        data.decode("utf-8", "strict")
    if total > 4 * MAX_EVIDENCE_BYTES:
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A evidence packet byte cap failed")
    expected_payloads = expected_names - {"hashes.sha256"}
    manifest_lines = (root / "hashes.sha256").read_text(encoding="utf-8").splitlines()
    if len(manifest_lines) != len(expected_payloads):
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A hash manifest count differs")
    seen: set[str] = set()
    for line in manifest_lines:
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9_.-]+)", line)
        if match is None:
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A hash manifest is malformed")
        digest, name = match.groups()
        if name not in expected_payloads or name in seen or sha256_file(root / name) != digest:
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A hash manifest readback differs")
        seen.add(name)
    if seen != expected_payloads:
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A hash manifest roster differs")
    forbidden = (
        str(pathlib.Path.home()).encode(),
        str(repo).encode(),
        os.uname().nodename.encode(),
    )
    for path in root.iterdir():
        data = path.read_bytes()
        if any(value and value in data for value in forbidden):
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A evidence contains a private host/path literal")
        if re.search(rb"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}", data):
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A evidence contains an email-like value")
        if re.search(rb'"(?:pid|ppid|pgrp|session)"\s*:', data) or re.search(rb"/proc/[0-9]+", data):
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A evidence contains a raw process identifier")
    fixture = json.loads((root / "fixture.json").read_text(encoding="utf-8"))
    if (
        fixture.get("schema") != SCHEMA
        or fixture.get("result") != "WR0A_COMPLETE"
        or fixture.get("accepted") is not False
        or fixture.get("source", {}).get("wr0_contract_digest") != WR0_CONTRACT_DIGEST
        or fixture.get("live", {}).get("environment_identity_sha256") != ENVIRONMENT_DIGEST
        or fixture.get("sanitization", {}).get("classification") != "passed"
    ):
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "WR0A fixture semantic identities differ")
    if require_index:
        for relative in EVIDENCE_PATHS:
            expected = working_entry(repo, relative)
            if expected is None or index_entry(repo, relative) != expected:
                block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", f"WR0A evidence index differs: {relative}")
    return {"classification": "passed", "file_count": 8, "payload_hash_count": 7}


def verify_candidate(repo: pathlib.Path) -> dict[str, Any]:
    topo = topology(repo)
    source = source_manifest(repo, topo)
    verify_authority_records(repo)
    verify_plan(repo)
    staged = topo.state == "tool"
    if staged:
        verify_import_state(repo, staged=True)
    else:
        verify_import_state(repo, staged=False)
        recover_existing_adoption_session(repo, topo, source)
    contract = contract_source_manifest(repo)
    canonical_packet = validate_current_archive_packet(repo)
    archive_packet = verify_archive_packet_from_object(repo)
    if topo.state == "final":
        wr0a_packet = validate_wr0a_packet(repo, require_index=True)
        if changed_paths(repo, IMPLEMENTATION_BASIS, "HEAD") != ALLOWED_FINAL_PATHS:
            block("FINAL_HEAD_SOURCE_MISMATCH", "final cumulative changed-path envelope differs")
    else:
        wr0a_packet = {"classification": "not_yet_rendered"}
    return {
        "schema": SCHEMA,
        "classification": "passed",
        "mode": "verify-candidate",
        "candidate_state": "staged" if staged else topo.state,
        "topology": dataclasses.asdict(topo),
        "source": source,
        "contract_source": contract,
        "canonical_packet": canonical_packet,
        "archive_packet": archive_packet,
        "wr0a_packet": wr0a_packet,
        "authority": verify_authority_records(repo),
    }


def safe_stat(path: pathlib.Path, *, hash_regular: bool) -> dict[str, Any]:
    info = path.lstat()
    if stat.S_ISLNK(info.st_mode):
        block("READ_ONLY_CALL_GRAPH_VIOLATION", "selected live object is a symlink")
    kind = "directory" if stat.S_ISDIR(info.st_mode) else "regular_file" if stat.S_ISREG(info.st_mode) else "other"
    if kind == "other":
        block("READ_ONLY_CALL_GRAPH_VIOLATION", "selected live object has unsupported type")
    record: dict[str, Any] = {
        "type": kind,
        "mode": stat.S_IMODE(info.st_mode),
        "size": info.st_size,
        "mtime_ns": info.st_mtime_ns,
        "ctime_ns": info.st_ctime_ns,
        "device": info.st_dev,
        "inode": info.st_ino,
    }
    if hash_regular:
        record["sha256"] = sha256_file(path)
    return record


def selected_live_physical_snapshot(environment: pathlib.Path) -> dict[str, Any]:
    if environment.resolve() != environment or not environment.is_dir() or environment.is_symlink():
        block("WR0A_LIVE_IDENTITY_BLOCKED", "fixed live environment is absent or unsafe")
    records: dict[str, Any] = {".": safe_stat(environment, hash_regular=False)}
    immediate = sorted(environment.iterdir(), key=lambda path: path.name)
    if len(immediate) > 32:
        block("WR0A_LIVE_IDENTITY_BLOCKED", "live environment top-level cap exceeded")
    for path in immediate:
        relative = path.name
        records[relative] = safe_stat(path, hash_regular=path.is_file())
    receipts = environment / "receipts"
    if receipts.is_dir() and not receipts.is_symlink():
        children = sorted(receipts.iterdir(), key=lambda path: path.name)
        if len(children) > 16:
            block("WR0A_LIVE_IDENTITY_BLOCKED", "live receipt cap exceeded")
        for path in children:
            records[f"receipts/{path.name}"] = safe_stat(path, hash_regular=path.is_file())
    prefix = environment / "compatdata/pfx"
    records["compatdata/pfx"] = safe_stat(prefix, hash_regular=False)
    prefix_children = sorted(prefix.iterdir(), key=lambda path: path.name)
    if len(prefix_children) > 64:
        block("WR0A_LIVE_IDENTITY_BLOCKED", "live prefix top-level cap exceeded")
    records["compatdata/pfx/roster"] = [
        {"name": path.name, "type": "directory" if path.is_dir() else "regular_file" if path.is_file() else "other"}
        for path in prefix_children
    ]
    for name in ("system.reg", "user.reg", "userdef.reg"):
        path = prefix / name
        records[f"compatdata/pfx/{name}"] = safe_stat(path, hash_regular=True)
    return records


def transaction_siblings(environment: pathlib.Path) -> list[str]:
    parent = environment.parent
    allowed_patterns = (
        re.compile(r"\.wr0-proton11\.stage-[A-Za-z0-9._-]+"),
        re.compile(r"\.wr0-proton11\.previous-[A-Za-z0-9._-]+"),
        re.compile(r"\.wr0-proton11\.retiring-[A-Za-z0-9._-]+"),
        re.compile(r"\.wr0-proton11\.journal-[A-Za-z0-9._-]+"),
    )
    siblings = []
    for path in parent.iterdir():
        if any(pattern.fullmatch(path.name) for pattern in allowed_patterns):
            siblings.append(path.name)
    if len(siblings) > 16:
        block("WR0A_LIVE_IDENTITY_BLOCKED", "transaction sibling cap exceeded")
    return sorted(siblings)


def source_tree_snapshot(repo: pathlib.Path) -> dict[str, Any]:
    root = repo / "tools/wr0-proton-bootstrap"
    records: dict[str, Any] = {}
    paths = sorted(root.rglob("*"), key=lambda path: path.as_posix())
    if len(paths) > 128:
        block("READ_ONLY_CALL_GRAPH_VIOLATION", "WR0 source-tree object cap exceeded")
    for path in paths:
        relative = path.relative_to(repo).as_posix()
        if path.is_symlink():
            block("READ_ONLY_CALL_GRAPH_VIOLATION", "WR0 source tree contains a symlink")
        records[relative] = safe_stat(path, hash_regular=path.is_file())
    bytecode = sorted(
        relative
        for relative in records
        if "__pycache__" in pathlib.PurePosixPath(relative).parts or relative.endswith((".pyc", ".pyo"))
    )
    return {
        "records": records,
        "bytecode": bytecode,
        "staged": sorted(staged_paths(repo)),
        "unstaged": sorted(unstaged_paths(repo)),
        "untracked": sorted(untracked_paths(repo)),
    }


def load_archive_fixture(repo: pathlib.Path) -> dict[str, Any]:
    path = repo / "evidence/wr0-proton-bootstrap/fixture.json"
    try:
        fixture = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, UnicodeDecodeError) as exc:
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "canonical fixture is malformed")
        raise AssertionError from exc
    if sha256_file(path) != ARCHIVE_EVIDENCE_HASHES["fixture.json"]:
        block("WR0A_EVIDENCE_ADOPTION_BLOCKED", "canonical fixture hash differs")
    return fixture


def verify_live_semantics(
    fixture: dict[str, Any],
    lock: dict[str, Any],
    source: dict[str, Any],
    environment: dict[str, Any],
    receipts: dict[str, Any],
    replacement: dict[str, Any],
) -> None:
    expected_environment = json.loads(json.dumps(fixture["environment"]))
    expected_environment["classification"] = "passed"
    if environment != expected_environment:
        block("WR0A_LIVE_IDENTITY_BLOCKED", "live environment differs from canonical archive evidence")
    if (
        lock.get("digest") != RUNNER_DIGEST
        or source.get("digest") != WR0_CONTRACT_DIGEST
        or source.get("workload_sha256") != WORKLOAD_DIGEST
        or environment.get("environment_identity_sha256") != ENVIRONMENT_DIGEST
        or environment.get("marker", {}).get("transaction_id") != TRANSACTION_ID
        or replacement.get("sha256") != RETIRED_RECORD_DIGEST
    ):
        block("WR0A_LIVE_IDENTITY_BLOCKED", "live fixed identity binding differs")
    expected_receipts = {
        "run-1.json": fixture["run_1"].get("internal_receipt_sha256"),
        "run-2.json": fixture["run_2"].get("internal_receipt_sha256"),
        "exit-37.json": fixture["exit_propagation"].get("internal_receipt_sha256"),
    }
    if set(receipts) != set(expected_receipts):
        block("WR0A_LIVE_IDENTITY_BLOCKED", "live receipt roster differs")


def dispatch_live_readonly(repo: pathlib.Path) -> dict[str, Any]:
    topo = topology(repo)
    if topo.state not in {"adoption", "final"}:
        block("READ_ONLY_CALL_GRAPH_VIOLATION", "live inspection requires committed archive adoption")
    require_clean(repo, "READ_ONLY_CALL_GRAPH_VIOLATION")
    if tree_entry(repo, "HEAD", LAUNCH_PATH) != LAUNCH_ENTRY or working_entry(repo, LAUNCH_PATH) != LAUNCH_ENTRY:
        block("READ_ONLY_CALL_GRAPH_VIOLATION", "frozen launch module identity differs")
    fixture = load_archive_fixture(repo)
    live_path = pathlib.Path.home() / ".local/share/linux-vst-bridge/environments/wr0-proton11"
    source_before = source_tree_snapshot(repo)
    if source_before["bytecode"]:
        block("READ_ONLY_CALL_GRAPH_VIOLATION", "preexisting WR0 bytecode artifact is present")
    physical_before = selected_live_physical_snapshot(live_path)
    siblings_before = transaction_siblings(live_path)
    if siblings_before:
        block("WR0A_LIVE_IDENTITY_BLOCKED", "WR0 transaction sibling exists before inspection")
    old_dont_write = sys.dont_write_bytecode
    sys.dont_write_bytecode = True
    os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
    module = None
    try:
        spec = importlib.util.spec_from_file_location(LAUNCH_MODULE_NAME, repo / LAUNCH_PATH)
        if spec is None or spec.loader is None or spec.name != LAUNCH_MODULE_NAME:
            block("READ_ONLY_CALL_GRAPH_VIOLATION", "fixed module loader construction failed")
        module = importlib.util.module_from_spec(spec)
        sys.modules[LAUNCH_MODULE_NAME] = module
        spec.loader.exec_module(module)
        required_calls = {
            "process_guard",
            "verify_runner_lock",
            "contract_source_manifest",
            "environment_snapshot",
            "environment_receipt_snapshot",
            "read_replacement_commit_record",
            "capture_fixture",
        }
        if any(not callable(getattr(module, name, None)) for name in required_calls):
            block("READ_ONLY_CALL_GRAPH_VIOLATION", "frozen allowlisted function is absent")
        guard_before = module.process_guard()
        lock = module.verify_runner_lock()
        source = module.contract_source_manifest(require_clean=True, repository=repo)
        environment = module.environment_snapshot(live_path, require_ready=True, contract_source=source)
        receipts = module.environment_receipt_snapshot(live_path, fixture)
        replacement = module.read_replacement_commit_record(live_path)
        protected_before = module.capture_fixture()
        protected_after = module.capture_fixture()
        guard_after = module.process_guard()
    except WR0ABlocked:
        raise
    except Exception as exc:
        block("WR0A_LIVE_IDENTITY_BLOCKED", f"frozen read-only dispatcher refused live state: {type(exc).__name__}: {exc}")
    finally:
        if module is not None:
            sys.modules.pop(LAUNCH_MODULE_NAME, None)
        sys.dont_write_bytecode = old_dont_write
    physical_after = selected_live_physical_snapshot(live_path)
    siblings_after = transaction_siblings(live_path)
    source_after = source_tree_snapshot(repo)
    if (
        physical_before != physical_after
        or siblings_after
        or source_before != source_after
        or protected_before != protected_after
    ):
        block("READ_ONLY_CALL_GRAPH_VIOLATION", "read-only dispatcher changed protected physical state")
    for guard in (guard_before, guard_after):
        counts = guard.get("counts", {})
        if any(counts.get(name) != 0 for name in FORBIDDEN_PROCESS_KEYS):
            block("WR0A_LIVE_IDENTITY_BLOCKED", "forbidden process count is nonzero")
    verify_live_semantics(fixture, lock, source, environment, receipts, replacement)
    return {
        "classification": "WR0A_LIVE_MATCH",
        "module_name": LAUNCH_MODULE_NAME,
        "dont_write_bytecode": True,
        "environment_identity_sha256": ENVIRONMENT_DIGEST,
        "transaction_id": TRANSACTION_ID,
        "runner_identity_sha256": RUNNER_DIGEST,
        "contract_source_sha256": WR0_CONTRACT_DIGEST,
        "workload_sha256": WORKLOAD_DIGEST,
        "retired_record_sha256": RETIRED_RECORD_DIGEST,
        "archive_live_equal": True,
        "before_after_physical_equal": True,
        "protected_fixture_equal": True,
        "protected_fixture_sha256": sha256_bytes(canonical_json(protected_before)),
        "process_counts_before": guard_before["counts"],
        "process_counts_after": guard_after["counts"],
        "transaction_sibling_count_before": 0,
        "transaction_sibling_count_after": 0,
        "source_tree_equal": True,
        "bytecode_artifact_count": 0,
        "prefix_device": environment["environment_identity"]["pfx_device"],
        "prefix_inode": environment["environment_identity"]["pfx_inode"],
        "registry_files": environment["registry_files"],
        "bounded_prefix_top_level": environment["bounded_prefix_top_level"],
        "receipt_files": receipts,
    }


def inspect_live(repo: pathlib.Path) -> dict[str, Any]:
    candidate = verify_candidate(repo)
    topo = topology(repo)
    source = source_manifest(repo, topo)
    leaf, marker = create_session(repo, topo, source, "inspect-live", None)
    try:
        live = dispatch_live_readonly(repo)
        marker["phase"] = "LIVE_READBACK_COMPLETE"
        write_marker(leaf, marker)
    except Exception:
        marker["phase"] = "DIAGNOSTIC_FAILED"
        write_marker(leaf, marker)
        cleanup_session(leaf, marker)
        raise
    cleanup_session(leaf, marker)
    return {
        "schema": SCHEMA,
        "classification": "passed",
        "mode": "inspect-live",
        "candidate_state": candidate["candidate_state"],
        "live": live,
        "retained_evidence_written": False,
        "session_cleaned": True,
    }


def synthetic_repository(parent: pathlib.Path) -> tuple[pathlib.Path, dict[str, tuple[tuple[str, str], tuple[str, str]]]]:
    repo = parent / "repository"
    repo.mkdir(mode=0o700)
    git(repo, "init", "-q")
    git(repo, "config", "user.name", "WR0A Synthetic Test")
    git(repo, "config", "user.email", "wr0a-test@invalid.example")
    paths = [f"governed/path-{number:02d}.bin" for number in range(1, 14)]
    old_modes: dict[str, str] = {}
    for number, path in enumerate(paths, 1):
        absolute = repo / path
        absolute.parent.mkdir(parents=True, exist_ok=True)
        mode = "100755" if number % 2 == 0 else "100644"
        atomic_write(absolute, f"old-{number}\n".encode(), 0o755 if mode == "100755" else 0o644)
        os.chmod(absolute, 0o755 if mode == "100755" else 0o644)
        old_modes[path] = mode
    git(repo, "add", "--", *paths)
    git(repo, "commit", "-q", "-m", "synthetic old state")
    entries: dict[str, tuple[tuple[str, str], tuple[str, str]]] = {}
    for number, path in enumerate(paths, 1):
        old = tree_entry(repo, "HEAD", path)
        if old is None or old[0] != old_modes[path]:
            raise AssertionError("synthetic old entry setup failed")
        new_data = f"new-{number}\n".encode()
        new_blob = git(repo, "hash-object", "-w", "--stdin", input_bytes=new_data).decode("ascii").strip()
        new_mode = "100644" if old[0] == "100755" else "100755"
        entries[path] = (old, (new_mode, new_blob))
    return repo, entries


def run_negative_test_ledger(repo: pathlib.Path) -> dict[str, Any]:
    topo = topology(repo)
    source = source_manifest(repo, topo)
    root = session_root(test=True)
    if list_sessions(root):
        block("TRANSIENT_CACHE_CLEANUP_BLOCKED", "synthetic test root is not empty")
    leaf = root / f"session-{secrets.token_hex(16)}"
    leaf.mkdir(mode=0o700)
    fsync_directory(root)
    cases: list[dict[str, str]] = []

    def case(name: str, function: Any) -> None:
        try:
            function()
        except Exception as exc:
            cases.append({"case": name, "result": "failed", "detail": f"{type(exc).__name__}: {exc}"[:300]})
        else:
            cases.append({"case": name, "result": "passed"})

    synthetic, entries = synthetic_repository(leaf)
    content = verify_entry_objects(synthetic, entries)
    try:
        for phase in ("write", "mode", "index"):
            for number in range(1, 14):
                def injected(phase: str = phase, number: int = number) -> None:
                    try:
                        perform_adoption(synthetic, entries, failure_phase=phase, failure_number=number)
                    except InjectedFailure:
                        pass
                    else:
                        raise AssertionError("injected failure did not fire")
                    require_clean(synthetic)
                    for path, (old, _new) in entries.items():
                        if working_entry(synthetic, path) != old or index_entry(synthetic, path) != old:
                            raise AssertionError("production recovery did not restore old state")

                case(f"production_recovery_after_{phase}_{number:02d}", injected)

        def successful_stage_and_recovery() -> None:
            perform_adoption(synthetic, entries)
            if staged_paths(synthetic) != frozenset(entries):
                raise AssertionError("synthetic success staged set differs")
            recover_adoption(synthetic, entries, content)
            require_clean(synthetic)

        case("production_exact_thirteen_stage_and_recovery", successful_stage_and_recovery)

        first_path = next(iter(entries))
        first_old = entries[first_path][0]

        def unknown_old_refused_before_write() -> None:
            unknown = b"unknown preflight bytes\n"
            atomic_write(synthetic / first_path, unknown)
            before = sha256_file(synthetic / first_path)
            try:
                perform_adoption(synthetic, entries)
            except WR0ABlocked:
                pass
            else:
                raise AssertionError("unknown old destination was accepted")
            if sha256_file(synthetic / first_path) != before:
                raise AssertionError("unknown destination was overwritten")
            write_entry(synthetic, first_path, first_old, content[first_path][0])
            require_clean(synthetic)

        case("production_unknown_old_destination_preserved_and_refused", unknown_old_refused_before_write)

        def unknown_recovery_refused_without_overwrite() -> None:
            unknown = b"unknown recovery bytes\n"
            atomic_write(synthetic / first_path, unknown)
            before = sha256_file(synthetic / first_path)
            try:
                recover_adoption(synthetic, entries, content)
            except WR0ABlocked as exc:
                if exc.classification != "WR0A_BLOB_IMPORT_RECOVERY_BLOCKED":
                    raise
            else:
                raise AssertionError("unknown recovery destination was accepted")
            if sha256_file(synthetic / first_path) != before:
                raise AssertionError("unknown recovery destination was overwritten")
            write_entry(synthetic, first_path, first_old, content[first_path][0])
            require_clean(synthetic)

        case("production_unknown_recovery_state_preserved_and_refused", unknown_recovery_refused_without_overwrite)
        case("fixed_twenty_five_path_envelope", lambda: (_ for _ in ()).throw(AssertionError()) if len(ALL_WR0_PATHS) != 25 else None)
        case("fixed_fourteen_difference_ledger", lambda: (_ for _ in ()).throw(AssertionError()) if len(IMPORT_ENTRIES) + 1 != 14 else None)
        case("fixed_thirteen_import_allowlist", lambda: (_ for _ in ()).throw(AssertionError()) if len(IMPORT_ENTRIES) != 13 else None)
        case("fixed_eleven_preserved_allowlist", lambda: (_ for _ in ()).throw(AssertionError()) if len(IDENTICAL_ENTRIES) != 11 else None)
        case("archive_current_slice_excluded", lambda: (_ for _ in ()).throw(AssertionError()) if "CURRENT_SLICE.md" in IMPORT_ENTRIES else None)
        case("tool_source_manifest_stable", lambda: source_manifest(repo, topology(repo)))
        case("authority_and_dg0_exact", lambda: verify_authority_records(repo))
        case("archive_object_packet_exact", lambda: verify_archive_packet_from_object(repo))
        case("archive_delta_plan_exact", lambda: verify_plan(repo))

        def lifecycle_envelope() -> None:
            current = topology(repo)
            if current.state == "tool":
                if commit_delta(repo, current.tool_commit) != frozenset(TOOL_PATHS):
                    raise AssertionError("tool lifecycle delta differs")
            elif current.state == "adoption":
                if commit_delta(repo, current.adoption_commit or "") != frozenset(IMPORT_ENTRIES):
                    raise AssertionError("adoption lifecycle delta differs")
            else:
                if commit_delta(repo, current.evidence_commit or "") != frozenset(EVIDENCE_PATHS):
                    raise AssertionError("final lifecycle delta differs")

        case("exact_commit_lifecycle_envelope", lifecycle_envelope)

        def fixed_dispatch_contract() -> None:
            if LAUNCH_MODULE_NAME != "_wr0a_readonly_wr0_launch_215718bb":
                raise AssertionError("fixed dispatch identity differs")
            if tree_entry(repo, ARCHIVE, LAUNCH_PATH) != LAUNCH_ENTRY:
                raise AssertionError("archive launch identity differs")

        case("fixed_read_only_dispatch_identity", fixed_dispatch_contract)

        def evidence_lifecycle() -> None:
            current = topology(repo)
            evidence_root = repo / "evidence/wr0a-final-repair-reconciliation"
            if current.state == "final":
                validate_wr0a_packet(repo, require_index=True)
            elif evidence_root.exists():
                raise AssertionError("WR0A evidence exists before evidence commit")

        case("evidence_packet_matches_lifecycle", evidence_lifecycle)
    finally:
        if leaf.exists():
            shutil.rmtree(leaf)
            fsync_directory(root)
    failed = [item for item in cases if item["result"] != "passed"]
    if failed:
        block("WR0A_NEGATIVE_TESTS_BLOCKED", f"{len(failed)} production negative tests failed")
    return {
        "classification": "passed",
        "source_digest": source["digest"],
        "lifecycle": topo.state,
        "passed_count": len(cases),
        "failed_count": 0,
        "cases": cases,
        "synthetic_output_used_as_real_evidence": False,
        "test_session_cleaned": True,
    }


def markdown_table(rows: Iterable[tuple[str, str]]) -> str:
    lines = ["| Fact | Result |", "|---|---|"]
    lines.extend(f"| {name} | {value} |" for name, value in rows)
    return "\n".join(lines)


def render_packet_documents(
    repo: pathlib.Path,
    topo: Topology,
    source: dict[str, Any],
    plan: dict[str, Any],
    live: dict[str, Any],
    tests: dict[str, Any],
) -> dict[str, bytes]:
    authority = verify_authority_records(repo)
    contract = contract_source_manifest(repo)
    basis = f"""# WR0A reconciliation basis

{markdown_table((
    ("Result", "`WR0A_COMPLETE`"),
    ("Accepted", "`false` — implementation PR evidence is not technical-lead acceptance"),
    ("Implementation basis", f"`{IMPLEMENTATION_BASIS}` / `{IMPLEMENTATION_BASIS_TREE}`"),
    ("Tool commit/tree", f"`{topo.tool_commit}` / `{topo.tool_tree}`"),
    ("Adoption commit/tree", f"`{topo.adoption_commit}` / `{topo.adoption_tree}`"),
    ("Archive commit/tree/parent", f"`{ARCHIVE}` / `{ARCHIVE_TREE}` / `{ARCHIVE_PARENT}...`"),
    ("Merged WR0 implementation/tree/merge", f"`{MERGED_WR0}` / `{MERGED_WR0_TREE}` / `{MERGED_WR0_MERGE}`"),
    ("Fixture", "`Steam Deck Galileo / SteamOS / x86_64`"),
))}

The repository, hostname, account, and private home path are intentionally not
retained. The third evidence commit is not self-referentially claimed by this
packet; its exact identity is established by final-head verification.
"""
    imported_rows = "\n".join(
        f"| `{path}` | `{old[0]}/{old[1]}` | `{new[0]}/{new[1]}` |"
        for path, (old, new) in sorted(IMPORT_ENTRIES.items())
    )
    preserved_rows = "\n".join(
        f"| `{path}` | `{entry[0]}/{entry[1]}` |"
        for path, entry in sorted(IDENTICAL_ENTRIES.items())
    )
    source_delta = f"""# WR0A source and archive delta

- WR0A source schema/digest: `{SOURCE_SCHEMA}` / `{source["digest"]}`.
- WR0 contract schema/digest: `{WR0_CONTRACT_SCHEMA}` / `{contract["digest"]}`.
- Tracked workload SHA-256: `{WORKLOAD_DIGEST}`.
- Complete comparison: `25` paths, `14` differences, `13` imports,
  `11` preserved paths, and one excluded archived authority card.

## Exact imported objects

| Path | Old mode/blob | Archive mode/blob |
|---|---|---|
{imported_rows}

## Exact preserved objects

| Path | Shared mode/blob |
|---|---|
{preserved_rows}

`CURRENT_SLICE.md` archive object
`{EXCLUDED_ENTRY["archive"][0]}/{EXCLUDED_ENTRY["archive"][1]}` was not
imported. Current authority remains
`{EXCLUDED_ENTRY["basis"][0]}/{EXCLUDED_ENTRY["basis"][1]}`.
"""
    archive_audit = f"""# WR0 final-repair archive audit

{markdown_table((
    ("Archive", f"`{ARCHIVE}` / `{ARCHIVE_TREE}`"),
    ("Canonical packet", "`14` files / `13` payload hashes"),
    ("Canonical hash-manifest blob", f"`{ARCHIVE_HASH_MANIFEST_BLOB}`"),
    ("Retained production cases", "`80/80`"),
    ("Non-live / actual live cases", "`75` / `5`"),
    ("Final-repair helper cases", "`10`"),
    ("Final WR0 result", "`WR0_COMPLETE`"),
))}

The ten final-repair cases exercise the production predecessor backup and
pre-commit recovery helpers, including rename/fsync, verification, stale-event,
unknown-object, and guarded-success boundaries. This reconciliation does not
rerun a Windows workload and does not promote archive evidence into broader
product claims.
"""
    live_readback = f"""# WR0A retained live readback

{markdown_table((
    ("Classification", "`WR0A_LIVE_MATCH`"),
    ("Environment identity", f"`{live['environment_identity_sha256']}`"),
    ("Transaction", f"`{live['transaction_id']}`"),
    ("Runner identity", f"`{live['runner_identity_sha256']}`"),
    ("Contract source", f"`{live['contract_source_sha256']}`"),
    ("Workload", f"`{live['workload_sha256']}`"),
    ("Retired replacement record", f"`{live['retired_record_sha256']}`"),
    ("Archive/live equality", "`true`"),
    ("Before/after physical equality", "`true`"),
    ("Protected fixture equality", "`true`"),
    ("Transaction siblings before/after", "`0 / 0`"),
    ("Forbidden process counts before/after", "`all zero / all zero`"),
    ("Bytecode artifacts", "`0`"),
    ("Source-tree change", "`none`"),
))}

The retained readback was produced by a fresh renderer-owned session. It did
not consume diagnostic stdout. The dispatcher imported the exact committed
`launch.py` under the fixed non-main module name with bytecode disabled and
called only the seven approved read-only functions.
"""
    dg0_rows = "\n".join(
        f"| `{path}` | `{entry[0]}/{entry[1]}` |"
        for path, entry in sorted(DG0_ENTRIES.items())
    )
    governance = f"""# WR0A governance preservation

- DG0 protected objects: `{authority["dg0_count"]}`, all exact.
- Active-authority objects: `{authority["authority_count"]}`, all exact.
- Archived `CURRENT_SLICE.md` imported: `false`.
- Implementation PR status closure performed: `false`.

| Protected path | Mode/blob |
|---|---|
{dg0_rows}

The implementation PR changes only the approved 23 paths. Acceptance, merge,
`CURRENT_SLICE.md` closure, and a decision-register entry remain separate
technical-lead-controlled work.
"""
    findings = f"""# WR0A findings

## Result

`WR0A_COMPLETE` — the current candidate adopts the exact thirteen accepted
final-repair archive objects, preserves the eleven shared WR0 objects and
current authority, reproduces the final WR0 source and packet identities, and
matches the read-only live Deck environment.

Production reconciliation tests: `{tests["passed_count"]}/{tests["passed_count"]}`;
failed: `0`. The two-file reconciler source remains
`{source["digest"]}`.

## Claim ceiling

This is implementation evidence, not acceptance. It proves only exact
repository reconciliation with the already-observed WR0 final-repair result on
the declared Deck fixture. It does not prove Windows VST3 hosting, Serum
operation, Bitwig scanning, audio, GUI, state recall, authorization, bridge
IPC, real-time safety, packaging, general Linux support, or product runner
selection. No live workload was launched and no live environment was mutated.
"""
    fixture = {
        "schema": SCHEMA,
        "result": "WR0A_COMPLETE",
        "accepted": False,
        "basis": {
            "implementation_commit": IMPLEMENTATION_BASIS,
            "implementation_tree": IMPLEMENTATION_BASIS_TREE,
            "tool_commit": topo.tool_commit,
            "tool_tree": topo.tool_tree,
            "adoption_commit": topo.adoption_commit,
            "adoption_tree": topo.adoption_tree,
        },
        "archive": {
            "commit": ARCHIVE,
            "tree": ARCHIVE_TREE,
            "parent_prefix": ARCHIVE_PARENT,
            "canonical_packet_files": 14,
            "payload_hashes": 13,
            "retained_tests": {"passed": 80, "failed": 0, "non_live": 75, "live": 5, "final_repair_helpers": 10},
        },
        "source": {
            "wr0a_schema": SOURCE_SCHEMA,
            "wr0a_digest": source["digest"],
            "wr0_contract_schema": WR0_CONTRACT_SCHEMA,
            "wr0_contract_digest": WR0_CONTRACT_DIGEST,
            "workload_sha256": WORKLOAD_DIGEST,
        },
        "delta": {
            "path_count": plan["path_count"],
            "difference_count": plan["difference_count"],
            "import_count": plan["import_count"],
            "preserved_count": plan["preserved_count"],
            "excluded_path": plan["excluded_path"],
            "current_slice_imported": False,
        },
        "live": live,
        "governance": {
            "dg0_count": len(DG0_ENTRIES),
            "authority_count": len(AUTHORITY_ENTRIES),
            "exact": True,
            "status_closed": False,
        },
        "negative_tests": tests,
        "claim": {
            "repository_final_repair_reconciled": True,
            "technical_lead_acceptance": False,
            "windows_vst3_hosting": "explicitly_out_of_scope",
            "serum": "explicitly_out_of_scope",
            "bitwig": "explicitly_out_of_scope",
            "audio": "explicitly_out_of_scope",
            "bridge": "explicitly_out_of_scope",
        },
        "sanitization": {
            "classification": "passed",
            "private_paths": False,
            "hostname": False,
            "raw_process_identifiers": False,
            "credentials": False,
            "proprietary_content": False,
        },
    }
    text_documents = {
        "BASIS.md": basis,
        "SOURCE_DELTA.md": source_delta,
        "ARCHIVE_AUDIT.md": archive_audit,
        "LIVE_READBACK.md": live_readback,
        "GOVERNANCE.md": governance,
        "FINDINGS.md": findings,
        "fixture.json": json.dumps(fixture, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
    }
    documents = {name: text.encode("utf-8") for name, text in text_documents.items()}
    manifest = "\n".join(f"{sha256_bytes(data)}  {name}" for name, data in sorted(documents.items())) + "\n"
    documents["hashes.sha256"] = manifest.encode("utf-8")
    return documents


def rollback_packet(repo: pathlib.Path, documents: dict[str, bytes]) -> None:
    root = repo / "evidence/wr0a-final-repair-reconciliation"
    for relative in EVIDENCE_PATHS:
        name = pathlib.PurePosixPath(relative).name
        path = repo / relative
        index = index_entry(repo, relative)
        if path.exists():
            if path.is_symlink() or not path.is_file() or path.read_bytes() != documents[name]:
                block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", f"unknown partial evidence preserved: {relative}")
            if index not in {None, working_entry(repo, relative)}:
                block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", f"unknown partial evidence index preserved: {relative}")
        elif index is not None:
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", f"index-only partial evidence preserved: {relative}")
    for relative in EVIDENCE_PATHS:
        path = repo / relative
        if index_entry(repo, relative) is not None:
            git(repo, "update-index", "--force-remove", "--", relative)
        if path.exists():
            path.unlink()
    if root.exists():
        root.rmdir()
    if any(index_entry(repo, relative) is not None or (repo / relative).exists() for relative in EVIDENCE_PATHS):
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "partial evidence rollback did not restore absence")


def render_evidence(repo: pathlib.Path) -> dict[str, Any]:
    candidate = verify_candidate(repo)
    topo = topology(repo)
    if topo.state != "adoption":
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "render-evidence requires the clean adoption commit")
    source = source_manifest(repo, topo)
    root = repo / "evidence/wr0a-final-repair-reconciliation"
    if root.exists() or any(index_entry(repo, path) is not None for path in EVIDENCE_PATHS):
        block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "final evidence destination is not absent")
    leaf, marker = create_session(repo, topo, source, "render-evidence", "evidence/wr0a-final-repair-reconciliation")
    documents: dict[str, bytes] = {}
    try:
        live = dispatch_live_readonly(repo)
        marker["phase"] = "LIVE_READBACK_COMPLETE"
        write_marker(leaf, marker)
        tests = run_negative_test_ledger(repo)
        plan = verify_plan(repo)
        documents = render_packet_documents(repo, topo, source, plan, live, tests)
        marker["phase"] = "PACKET_RENDERED_AND_VALIDATED"
        marker["rendered_hashes"] = {name: sha256_bytes(data) for name, data in sorted(documents.items())}
        write_marker(leaf, marker)
        root.mkdir(mode=0o755)
        fsync_directory(root.parent)
        for number, relative in enumerate(EVIDENCE_PATHS, 1):
            name = pathlib.PurePosixPath(relative).name
            marker["phase"] = "PUBLICATION_IN_PROGRESS"
            marker["publication_number"] = number
            marker["publication_path"] = relative
            write_marker(leaf, marker)
            atomic_write(repo / relative, documents[name], 0o644)
            os.chmod(repo / relative, 0o644)
            git(repo, "add", "--", relative)
            expected = working_entry(repo, relative)
            if expected is None or index_entry(repo, relative) != expected or sha256_file(repo / relative) != sha256_bytes(documents[name]):
                block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", f"published evidence readback differs: {relative}")
        if staged_paths(repo) != frozenset(EVIDENCE_PATHS) or unstaged_paths(repo) or untracked_paths(repo):
            block("WR0A_RECONCILIATION_EVIDENCE_BLOCKED", "published evidence staged envelope differs")
        packet = validate_wr0a_packet(repo, require_index=True)
        marker["phase"] = "PACKET_PUBLISHED_AND_STAGED"
        marker.pop("publication_number", None)
        marker.pop("publication_path", None)
        write_marker(leaf, marker)
    except Exception:
        if documents:
            rollback_packet(repo, documents)
        marker["phase"] = "PUBLICATION_RECOVERED"
        marker.pop("publication_number", None)
        marker.pop("publication_path", None)
        write_marker(leaf, marker)
        cleanup_session(leaf, marker)
        raise
    cleanup_session(leaf, marker)
    return {
        "schema": SCHEMA,
        "classification": "passed",
        "mode": "render-evidence",
        "candidate_state": candidate["candidate_state"],
        "tool_commit": topo.tool_commit,
        "adoption_commit": topo.adoption_commit,
        "source_digest": source["digest"],
        "live_classification": live["classification"],
        "negative_tests_passed": tests["passed_count"],
        "packet": packet,
        "staged_path_count": len(EVIDENCE_PATHS),
        "session_cleaned": True,
    }


def plan_mode(repo: pathlib.Path) -> dict[str, Any]:
    topo = topology(repo)
    if topo.state != "tool":
        block("ADOPTION_PLAN_MISMATCH", "plan is only valid at the exact tool commit")
    require_clean(repo, "ADOPTION_PLAN_MISMATCH")
    authority = verify_authority(repo, create_provenance=False)
    return {
        "schema": SCHEMA,
        "classification": "passed",
        "mode": "plan",
        "tool_commit": topo.tool_commit,
        "source_digest": authority["source"]["digest"],
        **verify_plan(repo),
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "mode",
        choices=(
            "verify-authority",
            "plan",
            "stage-adoption",
            "verify-candidate",
            "inspect-live",
            "render-evidence",
            "negative-tests",
        ),
    )
    arguments = parser.parse_args(argv)
    os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
    sys.dont_write_bytecode = True
    try:
        repo = repo_root()
        if arguments.mode == "verify-authority":
            topo = topology(repo)
            if topo.state != "tool":
                block("WR0A_BASIS_MISMATCH", "verify-authority is one-shot at the tool commit")
            require_clean(repo, "WR0A_BASIS_MISMATCH")
            result = verify_authority(repo)
        elif arguments.mode == "plan":
            result = plan_mode(repo)
        elif arguments.mode == "stage-adoption":
            result = stage_adoption(repo)
        elif arguments.mode == "verify-candidate":
            result = verify_candidate(repo)
        elif arguments.mode == "inspect-live":
            result = inspect_live(repo)
        elif arguments.mode == "render-evidence":
            result = render_evidence(repo)
        else:
            require_clean(repo, "WR0A_NEGATIVE_TESTS_BLOCKED")
            result = {
                "schema": SCHEMA,
                "classification": "passed",
                "mode": "negative-tests",
                **run_negative_test_ledger(repo),
            }
    except WR0ABlocked as exc:
        print(
            json.dumps(
                {
                    "schema": SCHEMA,
                    "classification": exc.classification,
                    "mode": arguments.mode,
                    "detail": exc.detail,
                    "mutating_live_state": False,
                },
                sort_keys=True,
            ),
            file=sys.stderr,
        )
        return 3
    print(json.dumps(result, indent=2, sort_keys=True, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

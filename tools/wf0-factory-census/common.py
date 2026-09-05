#!/usr/bin/env python3
"""Closed identities and fail-closed primitives shared by the WA0 harness."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import re
import stat
import subprocess
import tempfile
import time
from typing import Any, Iterable, Sequence


REPOSITORY = "kasselvania/Linux-VST-bridge"
BASIS_COMMIT = "47aeb7dcbaaec271292408fb9bbe0f2e4f9d00a9"
BASIS_TREE = "2dc9d64fc296670469b2c5b8ca0dccff59f45c01"
AUTHORITY_MERGE_COMMIT = BASIS_COMMIT
AUTHORITY_MERGE_TREE = BASIS_TREE
DESIGN_COMMIT = "0d5a41936c171f5d01d01b4c933875a1cfbe724a"
DESIGN_TREE = "787f1adf99e6083c3c90fa73996f6c151d2de50c"
DESIGN_BLOB = "d927c6dae430ffe7a811193e3d9119e573cfe316"
DESIGN_SHA256 = "eb5bd8f3aa439944f5933ecc9f0c4bcb90d46b3657499b365d11a82c0e2858ac"
REVIEW_GITHUB_ID = "5094205619"
APPROVAL_BLOB = "6817fe032bfdcee487f4dfb10caf22cb6686a9fa"
ACCEPTED_WC0_IMPLEMENTATION_MERGE = "cb831c38e1be88f4bb6a0ab6f2fca2d94164891b"
ACCEPTED_WC0_SOURCE_COMMIT = "9c0096930df86fc5b171cdebec40b306a198316a"
ACCEPTED_WC0_SOURCE_TREE = "e60286740a3aff7589e0dc9bb3b278e68a23374e"
ACCEPTED_WC0_EVIDENCE_COMMIT = "77bb40dbf35e19fd93b9f79b5286a43d56b9cc21"
ACCEPTED_WC0_EVIDENCE_TREE = "43fab0b6341e2549775b7f4223965031521ff338"
ACCEPTED_WC0_SOURCE_MANIFEST_SHA256 = (
    "38699a1d2026cb1078a569dc1997122b0111c78e608f294777dcf4afc49c8b25"
)
ACCEPTED_WC0_SCANNER_SHA256 = (
    "51b899b7936921b24265ead9ff12180f14249d49b532ead9a18414559f5f83f7"
)
ACCEPTED_WC0_ARTIFACT_MANIFEST_SHA256 = (
    "25bd47471f01ef57b06b3c8232bb6cc7e40c767f281c30186fa5426130f8ae82"
)
EXPECTED_BRANCH = "codex/wa0-windows-vst3-audio-processor-interface-admission"
EXPECTED_REF = f"refs/heads/{EXPECTED_BRANCH}"
WORKFLOW_PATH = ".github/workflows/wf0-windows-msvc-build.yml"
SDK_COMMIT = "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96"
SDK_TREE = "38343890fd1a0cedd48b7ec80ef17da15231b6c8"
SDK_SUBMODULES = {
    "base": "fcf9da0bd27a16f7f03773a3a39822f28f5c8477",
    "cmake": "054c9143cbb8d47fc4694e473f2ee3b4d951a8f5",
    "doc": "8bfca19d3b76a61d093951ba9297047f544caea1",
    "pluginterfaces": "4f547e8e102b47de4a8b8aaf343c73b700786372",
    "public.sdk": "586dc5e6c8012c3e4b01c79389375cbe96bdb1da",
    "tutorials": "33b73dfbb87f3fde3bce8c0a10cae934dc66ad34",
    "vstgui4": "5db272256172557818b6158cf0bb2c4410bddb25",
}
SDK_REMOTES = {
    "root": "https://github.com/steinbergmedia/vst3sdk.git",
    "base": "https://github.com/steinbergmedia/vst3_base.git",
    "cmake": "https://github.com/steinbergmedia/vst3_cmake.git",
    "doc": "https://github.com/steinbergmedia/vst3_doc.git",
    "pluginterfaces": "https://github.com/steinbergmedia/vst3_pluginterfaces.git",
    "public.sdk": "https://github.com/steinbergmedia/vst3_public_sdk.git",
    "tutorials": "https://github.com/steinbergmedia/vst3_tutorials.git",
    "vstgui4": "https://github.com/steinbergmedia/vstgui.git",
}
SDK_SOURCE_BLOBS = {
    "CMakeLists.txt": ("f006a70c9b5d115928cd14d0580ca541d72717b0",
                       "5f3d95f1037d501e5c0018e282cff2c5e4a0f35e4c7f0c20ee6cf5d3357d6497"),
    "cmake/modules/SMTG_VstGuiSupport.cmake":
        ("0c63c1c2322c9e1374dd45b297e2508a0f93e8bc",
         "4c99377c14b253aa7f7da699e7376db5239a129823516a6eaa89ec78c2ea25b5"),
    "vstgui4/CMakeLists.txt": ("8fa41c406756788e44c7c301c3bb9d9b1ede5335",
                              "f8c6bacb19cbc7d309326ac407db68049028aa5579489121d331b541843a730a"),
    "vstgui4/vstgui/standalone/CMakeLists.txt":
        ("a1bbc822ea153d1ef1036241da79b437c82ddf40",
         "70269facc46bb4094a93feccf0947e9efe92bfa76c2293612fbee857a06a0942"),
    "public.sdk/samples/vst/again/CMakeLists.txt":
        ("f2616195f4f0b92b55ade45ac2fa448a4674ec79",
         "b3b6865609cfe50338f210cc90b7f6d137204b145e05e01402baac19f95abc89"),
    "pluginterfaces/base/ipluginbase.h":
        ("859424bfc7f14b61df4b209a85c09d413513127a",
         "e10e9a4b9b0811c392af5758542e1f72875b56ebe7e1673d5075bc1160d9fd0b"),
    "pluginterfaces/base/funknown.h":
        ("3f6de83b104484e09097411417b37bcd28a46a1c",
         "e0d9609224fe15491c9ccd1463d964c303f1d5a6149fabf849e87f2c14be1951"),
    "pluginterfaces/base/smartpointer.h":
        ("ca64ae8f6260abc4225869bcb55ef6b27f3f3abf",
         "f690163a2d5fa71e76cce1909ae719561031b897f79342ddf38190b2ed9613b3"),
    "pluginterfaces/vst/ivsthostapplication.h":
        ("1818efe85a6674bf706fd8f8040e28cb329fbe27",
         "b7ef5b02f24c103e952e18b3e974e97b3c23d19c870a46c7c3aad36af8832332"),
    "pluginterfaces/vst/ivstcomponent.h":
        ("e20ef5f429349bdccf55367d45051fcdb0ff97c5",
         "cc587e34c009388d4187948c14a651df1481a01920abe895023094ca5b13faee"),
    "pluginterfaces/vst/ivstaudioprocessor.h":
        ("2a5428ceb3fd532a1e509a4a6a6dae4cda5191a3",
         "6289b19c8300fb381da7688414fae52d5ca139371f910830532204b860bd6549"),
    "base/source/fobject.h":
        ("6d092acfcc7bf3be91f8f00bcb3e6c27184d53eb",
         "6c8ef34413eed9fabbc61befaa40b213bbec4fe9cd7195738297caaaf1001767"),
    "base/source/fobject.cpp":
        ("a1da6cfffb23eb77d53d39da9429709f246955e6",
         "07ced0bc6398ea2d6d364e787d8cb7f09d8d8052af5b3e219e3df2b62341f54f"),
    "public.sdk/source/main/pluginfactory.cpp":
        ("a50c5000c1215ced5ae920a07c12153edb9b2498",
         "81dc1e6b5619ef1f22b83eb7243aa45d3ec7c54dd5b9fd42a32310bf40658f11"),
    "public.sdk/source/vst/vstcomponentbase.cpp":
        ("ccfde59797185b8a793b4a5bff2246c711faa18a",
         "00f279150eb1cccaa78a5e6c62fbbf19b6149e7af531c90c1482102162b4b7ab"),
    "public.sdk/source/vst/vstcomponent.cpp":
        ("d1dced4a441d35b73717da26eea0ada97406a874",
         "e9d8e5b4e25d319e378b8c8d547f34a3aa01f733c9ed3b60ddbe4c73237d7968"),
    "public.sdk/source/vst/vstaudioeffect.h":
        ("818cc4f3357c15c6f9a5ba649dbc70e87ced688f",
         "d61a3f92770bcab1b6dfafd49ea9935de8576ab92e251569c5bc2756835c35d9"),
    "public.sdk/source/vst/vstaudioeffect.cpp":
        ("2c3535be777ec777c5f066e42807092168019310",
         "997ff3d44bb9927def01d26c23a2aa11331d974921ee4a78b02e7345449e705b"),
    "public.sdk/source/vst/hosting/hostclasses.cpp":
        ("fd0e12498e7f9035e6d9f23c154cf50adfd8a6ab",
         "f9fbcd410d09ea3352342fdcb645e6a9dd1a424ff5b42430287fd4885a063c1d"),
}
SDK_POSITIVE_FIXTURE_BLOBS = {
    "public.sdk/samples/vst/again/source/again.h":
        ("061c0d4afb5f59410e75681d9998871f32fb1e72",
         "304b289e902c302928e2a3ebdc117eeef6af2781855445712f515301d4295e23"),
    "public.sdk/samples/vst/again/source/again.cpp":
        ("4676454679e37f188b99c2ec6e6def6b825da173",
         "05ff84588eac6ced18f26bba4e98632b28a2b5ee9d8c00340139fcc6b0efb00b"),
    "public.sdk/samples/vst/again/source/againentry.cpp":
        ("13b920b4b7a74137301bf213cf048e96e82861d4",
         "1cf23e867418578b4676a6298386d8eedaf463846d5ef8128389635731f72708"),
    "public.sdk/samples/vst/again/source/againcids.h":
        ("d32d1640ea187e718baba9cbb039d3a436d53cce",
         "5b6bd8bd5a714ababddbf490b55abd0d5d232b03b294882bf783e4421ee4715e"),
}

RUNNER_DIGEST = "2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547"
RUNTIME_VERSION = "4.0.20260805.254769"
RUNNER_VERSION = "1787334450 proton-11.0-2-x86_64"
RUNNER_BUILD_ID = "24867889"
NEUTRAL_APP_ID = "0"

SOURCE_SCHEMA = "linux-vst-bridge-wa0-implementation-source/v1"
WINDOWS_BUILD_SCHEMA = "linux-vst-bridge-wf0-windows-build/v1"
ARTIFACT_SCHEMA = "linux-vst-bridge-wf0-artifact-manifest/v1"
BUNDLE_SCHEMA = "linux-vst-bridge-wf0-bundle-manifest/v1"
MAC_CUSTODY_SCHEMA = "linux-vst-bridge-wf0-mac-artifact-custody/v1"
SOURCE_HANDOFF_SCHEMA = "linux-vst-bridge-wa0-source-handoff/v1"
EVIDENCE_HANDOFF_SCHEMA = "linux-vst-bridge-wa0-evidence-handoff/v1"
MARKER_SCHEMA = "linux-vst-bridge-wf0-scan-environment/v1"
SOURCE_PATHS = (
    ".github/workflows/wf0-windows-msvc-build.yml",
    "cmake/WF0DependencyLock.cmake",
    "tools/wf0-factory-census/README.md",
    "tools/wf0-factory-census/artifacts.py",
    "tools/wf0-factory-census/build.py",
    "tools/wf0-factory-census/common.py",
    "tools/wf0-factory-census/evidence.py",
    "tools/wf0-factory-census/negative_tests.py",
    "tools/wf0-factory-census/normalize.py",
    "tools/wf0-factory-census/run.py",
    "tools/wf0-factory-census/supervise.py",
    "tools/wf0-factory-census/verify.py",
    "windows-factory-probe/source/component_instance_session.cpp",
    "windows-factory-probe/source/component_instance_session.h",
    "windows-factory-probe/source/main.cpp",
    "windows-fixtures/wf0/CMakeLists.txt",
    "windows-fixtures/wf0/source/fault_fixture.cpp",
)
EVIDENCE_FILES = (
    "BASIS.md", "BUILD.md", "BUILD_MANIFEST.json", "ENVIRONMENT.md",
    "LAUNCH_AND_PROCESS.md", "AUDIO_PROCESSOR_LEASE.json",
    "COMPONENT_SESSION.json",
    "STAGE_TIMELINE.json", "NEGATIVE_TESTS.md", "PRESERVATION.md", "FINDINGS.md",
    "SANITIZATION.md", "fixture.json", "hashes.sha256",
)
FAULT_TARGETS = (
    "wa0-query-failure-null", "wa0-query-success-null",
    "wa0-query-failure-nonnull", "wa0-query-hang", "wa0-query-crash",
    "wa0-release-unexpected-count", "wa0-release-hang", "wa0-release-crash",
)


class WF0Error(RuntimeError):
    pass


def fail(message: str) -> None:
    raise WF0Error(message)


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode() + b"\n"


def parse_json_no_duplicates(data: bytes, label: str) -> Any:
    def object_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in pairs:
            if key in value:
                fail(f"{label} contains a duplicate JSON key")
            value[key] = item
        return value

    try:
        return json.loads(data, object_pairs_hook=object_pairs)
    except (json.JSONDecodeError, UnicodeDecodeError):
        fail(f"{label} is malformed JSON")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command(args: Sequence[str], *, cwd: pathlib.Path | None = None,
            env: dict[str, str] | None = None, timeout: float = 120.0,
            check: bool = True) -> subprocess.CompletedProcess[bytes]:
    result = subprocess.run(list(args), cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout,
                            check=False)
    if check and result.returncode != 0:
        diagnostic = (result.stdout + b"\n" + result.stderr).decode(errors="replace")[-8000:]
        fail(f"command failed ({result.returncode}): {args[0]}: {diagnostic}")
    return result


def command_text(args: Sequence[str], **kwargs: Any) -> str:
    return command(args, **kwargs).stdout.decode("utf-8", "strict").strip()


def real_home() -> pathlib.Path:
    return pathlib.Path.home().resolve(strict=True)


def repo_root() -> pathlib.Path:
    root = pathlib.Path(__file__).resolve().parents[2]
    if pathlib.Path(command_text(["git", "rev-parse", "--show-toplevel"], cwd=root)) != root:
        fail("repository root is not canonical")
    return root


def sdk_root() -> pathlib.Path:
    return real_home() / ".cache/linux-vst-bridge/dependencies/vst3sdk" / SDK_COMMIT


def runner_root() -> pathlib.Path:
    return real_home() / ".local/share/Steam/steamapps/common/Proton 11.0"


def runtime_root() -> pathlib.Path:
    return real_home() / ".local/share/Steam/steamapps/common/SteamLinuxRuntime_4"


def steam_root() -> pathlib.Path:
    return real_home() / ".local/share/Steam"


def wf0_cache() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/artifacts/wf0"


def artifact_cache_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/artifacts/wf0"


def source_handoff_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/handoffs/wa0/source/by-commit"


def execution_worktree_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/worktrees/wa0"


def environment_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/environments"


def require_regular(path: pathlib.Path, label: str) -> None:
    if not path.is_file() or path.is_symlink() or not stat.S_ISREG(path.stat().st_mode):
        fail(f"{label} is absent or unsafe")


def require_contained(path: pathlib.Path, parent: pathlib.Path, label: str) -> None:
    try:
        path.resolve(strict=False).relative_to(parent.resolve(strict=True))
    except (ValueError, FileNotFoundError):
        fail(f"{label} escapes its authority root")


def git_blob(treeish: str, path: str, root: pathlib.Path | None = None) -> tuple[str, str]:
    repository = root or repo_root()
    line = command_text(["git", "ls-tree", treeish, "--", path], cwd=repository)
    fields = line.split(None, 3)
    if len(fields) != 4 or fields[3] != path or fields[1] != "blob":
        fail(f"source roster path is absent or non-blob: {path}")
    return fields[0], fields[2]


def source_manifest(source_commit: str, *, treeish: str | None = None,
                    root: pathlib.Path | None = None) -> dict[str, Any]:
    repository = root or repo_root()
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        fail("implementation source commit is malformed")
    records = []
    for path in SOURCE_PATHS:
        mode, blob = git_blob(treeish or source_commit, path, repository)
        records.append({"path": path, "git_mode": mode, "git_blob": blob})
    if [item["path"] for item in records] != sorted(SOURCE_PATHS, key=lambda x: x.encode()):
        fail("implementation source roster is not raw UTF-8 sorted")
    return {"schema": SOURCE_SCHEMA, "commit": source_commit,
            "record_count": len(records), "records": records}


def source_manifest_sha256(value: dict[str, Any]) -> str:
    records = value.get("records")
    if (
        value.get("schema") != SOURCE_SCHEMA
        or not re.fullmatch(r"[0-9a-f]{40}", str(value.get("commit", "")))
        or value.get("record_count") != 17
        or not isinstance(records, list)
        or len(records) != 17
        or [item.get("path") for item in records if isinstance(item, dict)]
        != list(SOURCE_PATHS)
    ):
        fail("implementation-source manifest schema/count mismatch")
    for item in records:
        if (
            set(item) != {"path", "git_mode", "git_blob"}
            or not re.fullmatch(r"[0-7]{6}", str(item["git_mode"]))
            or not re.fullmatch(r"[0-9a-f]{40}", str(item["git_blob"]))
        ):
            fail("implementation-source manifest record shape differs")
    return sha256_bytes(canonical_json(value))


def require_clean_source(source_commit: str, *, treeish: str = "HEAD",
                         detached: bool | None = None) -> dict[str, Any]:
    root = repo_root()
    status = command_text(["git", "status", "--porcelain=v1", "--untracked-files=all"], cwd=root)
    if status:
        fail("implementation worktree must be clean")
    head = command_text(["git", "rev-parse", "HEAD"], cwd=root)
    if head != source_commit:
        fail(f"implementation source HEAD differs: {head}")
    parent = command_text(["git", "rev-parse", "HEAD^"], cwd=root)
    tree = command_text(["git", "rev-parse", "HEAD^{tree}"], cwd=root)
    basis_tree = command_text(["git", "rev-parse", f"{BASIS_COMMIT}^{{tree}}"], cwd=root)
    if parent != BASIS_COMMIT or basis_tree != BASIS_TREE:
        fail("implementation source parent or basis tree differs")
    changed = command_text(
        ["git", "diff", "--name-only", BASIS_COMMIT, source_commit], cwd=root
    ).splitlines()
    if changed != list(SOURCE_PATHS):
        fail(f"implementation source path envelope differs: {changed}")
    branch = command_text(["git", "branch", "--show-current"], cwd=root)
    if detached is True and branch:
        fail(f"Deck execution worktree is not detached: {branch}")
    if detached is False and branch != EXPECTED_BRANCH:
        fail(f"wrong WA0 branch: {branch}")
    # SOURCE_PATHS is the exact WA0 change envelope, not the complete set of
    # inherited WF0 files that remains present below these directories.  Keep
    # that inherited surface fail-closed by requiring its path/mode/blob map to
    # remain byte-identical to the accepted implementation basis.
    inherited_prefixes = (
        "windows-factory-probe/",
        "windows-fixtures/wf0/",
        "tools/wf0-factory-census/",
    )

    def governed_tree(value: str) -> dict[str, tuple[str, str]]:
        rows: dict[str, tuple[str, str]] = {}
        listing = command_text(
            ["git", "ls-tree", "-r", value, "--", *inherited_prefixes],
            cwd=root,
        )
        for line in listing.splitlines():
            metadata, path = line.split("\t", 1)
            mode, kind, blob = metadata.split()
            if kind != "blob":
                fail(f"governed inherited object is not a blob: {path}")
            if path not in SOURCE_PATHS:
                rows[path] = (mode, blob)
        return rows

    if governed_tree(treeish) != governed_tree(BASIS_COMMIT):
        fail("inherited WF0 source path/mode/blob identity differs from the basis")
    return source_manifest(source_commit, treeish=treeish, root=root)


def write_atomic(path: pathlib.Path, data: bytes, mode: int = 0o600) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        if hasattr(os, "fchmod"):
            os.fchmod(descriptor, mode)
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
        if hasattr(os, "O_DIRECTORY"):
            directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
        if path.read_bytes() != data:
            fail(f"atomic readback failed: {path.name}")
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def manifest_records(root: pathlib.Path) -> list[dict[str, Any]]:
    records = []
    for path in sorted(root.rglob("*"), key=lambda item: item.relative_to(root).as_posix().encode()):
        if path.is_symlink() or (path.exists() and not (path.is_file() or path.is_dir())):
            fail("unsafe artifact object")
        if path.is_file():
            relative = path.relative_to(root).as_posix()
            records.append({"path": relative, "mode": f"{stat.S_IMODE(path.stat().st_mode):04o}",
                            "size": path.stat().st_size, "sha256": sha256_file(path)})
    return records


def windows_path(path: pathlib.Path) -> str:
    if not path.is_absolute():
        fail("Windows path input is not absolute")
    return "Z:" + str(path).replace("/", "\\")


def process_census() -> list[dict[str, Any]]:
    records = []
    for entry in pathlib.Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        try:
            stat_fields = (entry / "stat").read_text().split()
            cmdline = (entry / "cmdline").read_bytes().replace(b"\0", b" ").decode("utf-8", "replace")
            comm = (entry / "comm").read_text().strip()
            records.append({"pid": int(entry.name), "ppid": int(stat_fields[3]),
                            "pgrp": int(stat_fields[4]), "session": int(stat_fields[5]),
                            "start_ticks": int(stat_fields[21]), "comm": comm,
                            "cmdline": cmdline})
        except (FileNotFoundError, ProcessLookupError, PermissionError, ValueError, IndexError):
            continue
    return records


def process_guard() -> dict[str, int]:
    counts = {name: 0 for name in ("bitwig", "validator", "wine", "proton", "runtime",
                                    "umu", "yabridge", "wf0")}
    for record in process_census():
        text = f"{record['comm']} {record['cmdline']}".lower()
        if "bitwig" in text: counts["bitwig"] += 1
        if "validator" in text: counts["validator"] += 1
        if re.search(r"(^|[ /])(wine64|wine|wineserver)([ $/]|$)", text): counts["wine"] += 1
        if "proton" in text and "steamwebhelper" not in text: counts["proton"] += 1
        if "steamlinuxruntime_4/_v2-entry-point" in text: counts["runtime"] += 1
        if re.search(r"(^|[ /])umu([ $/-]|$)", text): counts["umu"] += 1
        if "yabridge" in text: counts["yabridge"] += 1
        if "wf0-factory-probe" in text or ".wf0-factory-census.stage-" in text: counts["wf0"] += 1
    if any(counts.values()):
        fail(f"forbidden process guard is not empty: {counts}")
    siblings = [item.name for item in environment_parent().iterdir()
                if item.name.startswith((".wr0-proton11.", ".wf0-factory-census.stage-"))]
    if siblings:
        fail(f"transaction sibling guard is not empty: {siblings}")
    return counts


def deck_fixture_identity() -> dict[str, Any]:
    if os.uname().machine != "x86_64":
        fail("Deck architecture differs")
    product = pathlib.Path("/sys/devices/virtual/dmi/id/product_name").read_text(
        encoding="utf-8"
    ).strip()
    release: dict[str, str] = {}
    for line in pathlib.Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            release[key] = value.strip().strip('"')
    readonly = command_text(["steamos-readonly", "status"]).lower()
    if (
        product != "Galileo"
        or release.get("ID") != "steamos"
        or release.get("VERSION_ID") != "3.8.16"
        or "enabled" not in readonly
    ):
        fail("accepted Steam Deck/SteamOS/read-only fixture differs")
    if any(os.environ.get(name) for name in (
        "GH_TOKEN", "GITHUB_TOKEN", "GITHUB_PAT", "SSH_AUTH_SOCK"
    )):
        fail("Deck process environment exposes GitHub or forwarded-agent authority")
    return {
        "hardware": "Steam Deck Galileo",
        "os": "SteamOS 3.8.16",
        "architecture": "x86_64",
        "read_only_mode": "enabled",
    }


def verify_runner_identity() -> dict[str, Any]:
    helper = repo_root() / "tools/wr0-proton-bootstrap/launch.py"
    result = command(
        ["/usr/bin/python3", str(helper), "inspect-runner"],
        cwd=repo_root(), env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
        timeout=180.0,
    )
    values: dict[str, str] = {}
    for line in result.stdout.decode("utf-8", "strict").splitlines():
        if "=" not in line:
            fail("WR0 runner inspection emitted a malformed line")
        key, value = line.split("=", 1)
        values[key.lower()] = value
    if (
        values.get("launch_critical_manifest_sha256") != RUNNER_DIGEST
        or values.get("runtime_version") != RUNTIME_VERSION
        or values.get("runner_build_id") != RUNNER_BUILD_ID
        or values.get("runner_version") != RUNNER_VERSION
        or values.get("launch_critical_file_count") != "33"
    ):
        fail(f"Runtime 4 / Proton 11 identity differs: {values}")
    return {
        "runtime_version": values["runtime_version"],
        "runner_version": values["runner_version"],
        "runner_build_id": RUNNER_BUILD_ID,
        "launch_critical_file_count": 33,
        "launch_critical_manifest_sha256": RUNNER_DIGEST,
    }


def protected_snapshot() -> dict[str, Any]:
    app = "com.bitwig.BitwigStudio"
    app_commit = command_text(["flatpak", "info", "--show-commit", "--system", app])
    info_environment = dict(os.environ)
    info_environment.update({"LC_ALL": "C", "LANG": "C"})
    info_text = command(["flatpak", "info", "--system", app], env=info_environment).stdout.decode(
        "utf-8", "strict")
    version_match = re.search(r"^\s*Version:\s*(\S+)\s*$", info_text, re.MULTILINE)
    if version_match is None:
        fail("protected Bitwig version field is unavailable")
    app_version = version_match.group(1)
    app_origin = command_text(["flatpak", "info", "--show-origin", "--system", app])
    app_ref = command_text(["flatpak", "info", "--show-ref", "--system", app])
    runtime_ref = command_text(["flatpak", "info", "--show-runtime", "--system", app])
    user_shadow = command(["flatpak", "info", "--show-commit", "--user", app], check=False)
    if (app_version, app_commit, app_origin, app_ref, runtime_ref) != (
        "6.1", "8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231",
        "flathub", "app/com.bitwig.BitwigStudio/x86_64/stable",
        "org.freedesktop.Platform/x86_64/25.08") or user_shadow.returncode == 0:
        fail("current protected Bitwig 6.1 projection differs")
    runtime_commit = command_text([
        "flatpak", "info", "--show-commit", "--system", "org.freedesktop.Platform//25.08"])
    if runtime_commit != "bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8":
        fail("protected Bitwig runtime commit differs")
    user_override = command(["flatpak", "override", "--show", "--user", app]).stdout
    system_override = command(["flatpak", "override", "--show", "--system", app]).stdout
    permissions = command(["flatpak", "info", "--show-permissions", "--system", app]).stdout
    observed_hashes = {
        "user_override_sha256": sha256_bytes(user_override),
        "system_override_sha256": sha256_bytes(system_override),
        "permissions_sha256": sha256_bytes(permissions),
    }
    expected_hashes = {
        "user_override_sha256": "1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e",
        "system_override_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "permissions_sha256": "c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73",
    }
    if observed_hashes != expected_hashes:
        fail(f"protected Bitwig configuration differs: {observed_hashes}")

    wr0 = environment_parent() / "wr0-proton11"
    expected_wr0 = {
        "wr0-environment.json", "wr0-replacement-commit.json", "receipts/run-1.json",
        "receipts/run-2.json", "receipts/exit-37.json",
    }
    wr0_hashes = {path: sha256_file(wr0 / path) for path in sorted(expected_wr0)}
    environment_marker = json.loads((wr0 / "wr0-environment.json").read_bytes())
    replacement = json.loads((wr0 / "wr0-replacement-commit.json").read_bytes())
    if (environment_marker.get("status"), environment_marker.get("contract_source_sha256"),
        environment_marker.get("runner_identity_sha256"),
        replacement.get("new_environment_identity_sha256"),
        replacement.get("predecessor_retirement_status")) != (
        "ready", "c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541",
        RUNNER_DIGEST, "d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4",
        "retired"):
        fail("accepted WR0 environment marker differs")

    historical_paths = (
        "evidence/sr0-steam-deck-fixture-reconnaissance",
        "evidence/hp0-native-vst3-bitwig-sandbox", "evidence/hp1-bitwig-admission",
        "evidence/wr0-proton-bootstrap", "evidence/wr0a-final-repair-reconciliation",
        "tools/wr0-proton-bootstrap", "docs/WR0_RUNNER_LOCK.md", "docs/HP0_DEPENDENCY_LOCK.md",
        "native-probe",
    )
    current = command(["git", "ls-tree", "-r", "HEAD", "--", *historical_paths], cwd=repo_root()).stdout
    basis = command(["git", "ls-tree", "-r", BASIS_COMMIT, "--", *historical_paths], cwd=repo_root()).stdout
    if current != basis:
        fail("historical protected source/evidence bytes differ from the WF0 basis")
    return {
        "schema": "linux-vst-bridge-wf0-protected-snapshot/v1",
        "bitwig": {"version": app_version, "commit": app_commit, "origin": app_origin,
                   "ref": app_ref, "runtime_ref": runtime_ref,
                   "runtime_commit": runtime_commit, **observed_hashes,
                   "user_shadow_absent": True},
        "wr0": {"environment_identity": replacement["new_environment_identity_sha256"],
                "runner_identity": environment_marker["runner_identity_sha256"],
                "marker_hashes": wr0_hashes},
        "historical_tree_projection_sha256": sha256_bytes(current),
    }


# DX0 keeps the accepted WA0 execution primitives above intact and adds narrow
# identity domains around them.  These constants are deliberately separate
# from the historical WA0 constants: a complete implementation source, a
# Windows producer input, a Deck execution input, and an evidence renderer are
# different facts.
DX0_BASIS_COMMIT = "404966e6bfcd6403a1abb9ed8005210e93d9ad02"
DX0_BASIS_TREE = "dde8536b89c4b2e48dc9af5b28b64a86b5f2fa91"
DX0_BRANCH = "codex/dx0-split-build-identity-proof-transaction"
DX0_REF = f"refs/heads/{DX0_BRANCH}"
DX0_DESIGN_COMMIT = "f27695862f9287b225af739264169c2ca3f407ad"
DX0_DESIGN_BLOB = "5dd758d681f9712de02ab4c45d58d3d815aef53a"
DX0_DESIGN_SHA256 = "85de95acfe171678c0efedbbdc6aebdc0ff134a716dfd8d29f52feed9c53df6c"
DX0_REVIEW_ID = "5096747625"
DX0_APPROVAL_BLOB = "95bf313087f9e6ec0fd93d28d0ba796b1578f94c"
DX0_REVIEW_HISTORY_BLOB = "43b451cb665c20602e79a092c87f4731356c831c"

DX0_COMPLETE_SOURCE_SCHEMA = "linux-vst-bridge-dx0-complete-source/v1"
DX0_WINDOWS_BUILD_INPUT_SCHEMA = "linux-vst-bridge-dx0-windows-build-input/v1"
DX0_ACCEPTED_FIXTURE_SCHEMA = "linux-vst-bridge-dx0-accepted-fixture/v1"
DX0_DECK_EXECUTION_INPUT_SCHEMA = "linux-vst-bridge-dx0-deck-execution-input/v1"
DX0_EVIDENCE_RENDERER_SCHEMA = "linux-vst-bridge-dx0-evidence-renderer/v1"
DX0_TRANSACTION_SCHEMA = "linux-vst-bridge-dx0-proof-transaction/v1"
DX0_TRANSACTION_STATE_SCHEMA = "linux-vst-bridge-dx0-transaction-state/v1"
DX0_PLAN_SCHEMA = "linux-vst-bridge-dx0-proof-plan/v1"
DX0_RESULT_SCHEMA = "linux-vst-bridge-dx0-transaction-result/v1"
DX0_HOST_BUILD_SCHEMA = "linux-vst-bridge-dx0-windows-host-build/v1"
DX0_HOST_ARTIFACT_SCHEMA = "linux-vst-bridge-dx0-windows-host-artifact/v1"
DX0_MAC_HOST_CUSTODY_SCHEMA = "linux-vst-bridge-dx0-mac-host-custody/v1"
DX0_SOURCE_HANDOFF_SCHEMA = "linux-vst-bridge-dx0-source-handoff/v1"
DX0_PACKET_SCHEMA = "linux-vst-bridge-dx0-evidence-packet/v1"
DX0_RETAINED_TRANSACTION_SCHEMA = "linux-vst-bridge-dx0-retained-transaction/v1"
DX0_COST_SCHEMA = "linux-vst-bridge-dx0-cost-and-invalidation/v1"

DX0_SOURCE_PATHS = (
    ".github/workflows/wf0-windows-msvc-build.yml",
    "tools/host-proof.py",
    "tools/wf0-factory-census/README.md",
    "tools/wf0-factory-census/artifacts.py",
    "tools/wf0-factory-census/build.py",
    "tools/wf0-factory-census/common.py",
    "tools/wf0-factory-census/environment.py",
    "tools/wf0-factory-census/evidence.py",
    "tools/wf0-factory-census/negative_tests.py",
    "tools/wf0-factory-census/run.py",
)
DX0_WINDOWS_BUILD_PATHS = (
    ".github/workflows/wf0-windows-msvc-build.yml",
    "CMakeLists.txt",
    "cmake/WF0DependencyLock.cmake",
    "tools/wf0-factory-census/build.py",
    "tools/wf0-factory-census/common.py",
    "tools/wf0-factory-census/verify.py",
    "windows-factory-probe/CMakeLists.txt",
    "windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h",
    "windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h",
    "windows-factory-probe/source/component_instance_session.cpp",
    "windows-factory-probe/source/component_instance_session.h",
    "windows-factory-probe/source/factory_census.cpp",
    "windows-factory-probe/source/factory_census.h",
    "windows-factory-probe/source/main.cpp",
    "windows-factory-probe/source/win32_module.cpp",
    "windows-factory-probe/source/win32_module.h",
    "windows-fixtures/wf0/CMakeLists.txt",
)
DX0_DECK_EXECUTION_PATHS = (
    "tools/wf0-factory-census/artifacts.py",
    "tools/wf0-factory-census/common.py",
    "tools/wf0-factory-census/environment.py",
    "tools/wf0-factory-census/normalize.py",
    "tools/wf0-factory-census/run.py",
    "tools/wf0-factory-census/supervise.py",
    "tools/wr0-proton-bootstrap/launch.py",
)
DX0_RENDERER_PATHS = (
    "tools/wf0-factory-census/common.py",
    "tools/wf0-factory-census/evidence.py",
)
DX0_EVIDENCE_PATHS = (
    "evidence/dx0-split-build-identity-proof-transaction/BASIS.md",
    "evidence/dx0-split-build-identity-proof-transaction/COST_AND_INVALIDATION.json",
    "evidence/dx0-split-build-identity-proof-transaction/FINDINGS.md",
    "evidence/dx0-split-build-identity-proof-transaction/TRANSACTION.json",
    "evidence/dx0-split-build-identity-proof-transaction/hashes.sha256",
)

DX0_PLAN_ID = "wa0-positive-regression-v1"
DX0_ACCEPTED_FIXTURE_ID = "wa0-again-accepted-v1"
DX0_HOST_MODE = "host_only"
DX0_AGAIN_MODULE_SHA256 = (
    "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
)
DX0_AGAIN_BUNDLE_MANIFEST_SHA256 = (
    "bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164"
)
DX0_ACCEPTED_WA0_ARTIFACT_ID = 9869994854
DX0_ACCEPTED_WA0_RUN_ID = 33689659595
DX0_ACCEPTED_WA0_RUN_ATTEMPT = 2
DX0_ACCEPTED_WA0_PRODUCER = "24b7e6da7e29a5bd358097a6b89c5c59b747c413"
DX0_ACCEPTED_WA0_ARTIFACT_MANIFEST_SHA256 = (
    "028228c6a8cc638b4aaf1f477b317359a22eb1dd84e90a12193bb3c5d9159070"
)
DX0_ACCEPTED_WA0_WRAPPER_SHA256 = (
    "81f3d431a0cb00c4cc121799818ce5ac1b8487dfde583d03eb48b98adda7bf3b"
)
DX0_ACCEPTED_WA0_PAYLOAD_SHA256 = (
    "a9397ed53a070522230e6b75ce045e0aedaa3c0e73736cb8a901f203f09ef17c"
)
DX0_ACCEPTED_WA0_BUILD_RECEIPT_SHA256 = (
    "fdf9aabcfcf7eaefcedb50bd15d54f3a93c2babe7525aeb18515b3f13d67fe1e"
)
DX0_ACCEPTED_WA0_CUSTODY_RECEIPT_SHA256 = (
    "9d282701cde6f4576452044d58a13ab7380c19e4b0a5ccca0f950e0ebfcdef23"
)
DX0_CHECKOUT_ACTION = "11bd71901bbe5b1630ceea73d27597364c9af683"
DX0_UPLOAD_ACTION = "ea165f8d65b6e75b540449e92b4886f43607fa02"


def _require_closed_sorted(paths: Sequence[str], label: str) -> None:
    if list(paths) != sorted(paths, key=lambda value: value.encode("utf-8")):
        fail(f"{label} is not raw-UTF-8 sorted")
    if len(paths) != len(set(paths)):
        fail(f"{label} contains duplicates")


for _dx0_paths, _dx0_label in (
    (DX0_SOURCE_PATHS, "DX0 complete-source roster"),
    (DX0_WINDOWS_BUILD_PATHS, "DX0 Windows-build roster"),
    (DX0_DECK_EXECUTION_PATHS, "DX0 Deck-execution roster"),
    (DX0_RENDERER_PATHS, "DX0 evidence-renderer roster"),
    (DX0_EVIDENCE_PATHS, "DX0 evidence roster"),
):
    _require_closed_sorted(_dx0_paths, _dx0_label)


def dx0_records(commit: str, paths: Sequence[str], *,
                root: pathlib.Path | None = None) -> list[dict[str, str]]:
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        fail("DX0 source commit is malformed")
    repository = root or repo_root()
    records = [
        {"path": path, "git_mode": mode, "git_blob": blob}
        for path in paths
        for mode, blob in [git_blob(commit, path, repository)]
    ]
    _require_closed_sorted([record["path"] for record in records], "DX0 records")
    return records


def dx0_record_manifest_sha256(schema: str, records: list[dict[str, str]]) -> str:
    return sha256_bytes(canonical_json({
        "schema": schema,
        "record_count": len(records),
        "records": records,
    }))


def dx0_complete_source(commit: str, *, root: pathlib.Path | None = None,
                        strict: bool = True) -> dict[str, Any]:
    repository = root or repo_root()
    records = dx0_records(commit, DX0_SOURCE_PATHS, root=repository)
    parent = command_text(["git", "rev-parse", f"{commit}^"], cwd=repository)
    tree = command_text(["git", "rev-parse", f"{commit}^{{tree}}"], cwd=repository)
    value = {
        "schema": DX0_COMPLETE_SOURCE_SCHEMA,
        "commit": commit,
        "tree": tree,
        "parent": parent,
        "ref": DX0_REF,
        "record_count": len(records),
        "records": records,
    }
    if strict:
        if parent != DX0_BASIS_COMMIT:
            fail("DX0 source is not one direct child of the approved basis")
        basis_tree = command_text(
            ["git", "rev-parse", f"{DX0_BASIS_COMMIT}^{{tree}}"], cwd=repository
        )
        changed = command_text(
            ["git", "diff", "--name-only", DX0_BASIS_COMMIT, commit], cwd=repository
        ).splitlines()
        if basis_tree != DX0_BASIS_TREE or changed != list(DX0_SOURCE_PATHS):
            fail(f"DX0 source basis or ten-path envelope differs: {changed}")
    return value


def dx0_complete_source_sha256(value: dict[str, Any]) -> str:
    if (
        set(value) != {"schema", "commit", "tree", "parent", "ref",
                       "record_count", "records"}
        or value.get("schema") != DX0_COMPLETE_SOURCE_SCHEMA
        or value.get("record_count") != 10
        or not isinstance(value.get("records"), list)
        or [item.get("path") for item in value["records"]] != list(DX0_SOURCE_PATHS)
    ):
        fail("DX0 complete-source identity shape differs")
    return sha256_bytes(canonical_json(value))


def dx0_require_frozen_source(commit: str, *, detached: bool | None = False) -> dict[str, Any]:
    root = repo_root()
    if command_text(["git", "status", "--porcelain=v1", "--untracked-files=all"], cwd=root):
        fail("DX0 source worktree is not clean")
    if command_text(["git", "rev-parse", "HEAD"], cwd=root) != commit:
        fail("DX0 worktree HEAD differs from the requested source")
    branch = command_text(["git", "branch", "--show-current"], cwd=root)
    if detached is True and branch:
        fail("DX0 Deck worktree is not detached")
    if detached is False and branch != DX0_BRANCH:
        fail(f"DX0 implementation branch differs: {branch}")
    return dx0_complete_source(commit)


def dx0_windows_build_input(commit: str, *, root: pathlib.Path | None = None) -> dict[str, Any]:
    repository = root or repo_root()
    records = dx0_records(commit, DX0_WINDOWS_BUILD_PATHS, root=repository)
    return {
        "schema": DX0_WINDOWS_BUILD_INPUT_SCHEMA,
        "repository": REPOSITORY,
        "record_count": len(records),
        "records": records,
        "sdk": {
            "root_commit": SDK_COMMIT,
            "root_tree": SDK_TREE,
            "submodules": [
                {"path": path, "commit": SDK_SUBMODULES[path]}
                for path in sorted(SDK_SUBMODULES, key=lambda value: value.encode())
            ],
            "source_patch_count": 0,
            "checkout": {"core.autocrlf": "false", "core.eol": "lf"},
        },
        "build_contract": {
            "runner_label": "windows-2022",
            "visual_studio": "2022 Enterprise",
            "toolset": "v143",
            "architecture": "x64",
            "windows_sdk": "10.0.19041.0",
            "generator": "Visual Studio 17 2022",
            "configuration": "Release",
            "msvc_runtime": "MultiThreaded",
            "roots": 2,
            "targets": ["wf0-factory-probe"],
            "host_mode": DX0_HOST_MODE,
            "again_built": False,
            "fault_targets_built": False,
        },
        "action_commits": {
            "actions/checkout": DX0_CHECKOUT_ACTION,
            "actions/upload-artifact": DX0_UPLOAD_ACTION,
        },
    }


def dx0_identity_sha256(value: dict[str, Any]) -> str:
    return sha256_bytes(canonical_json(value))


def dx0_closed_plan(plan_id: str) -> dict[str, Any]:
    if plan_id != DX0_PLAN_ID:
        fail("DX0 plan identifier is outside the closed plan registry")
    return {
        "schema": DX0_PLAN_SCHEMA,
        "plan_id": DX0_PLAN_ID,
        "accepted_fixture_id": DX0_ACCEPTED_FIXTURE_ID,
        "host_mode": DX0_HOST_MODE,
        "deterministic_validation_set": "dx0-fourteen-row-v1",
        "live_deck_batch": "wa0-positive-only-v1",
        "expected_result": "wa0-positive-interface-lease-complete-v1",
        "evidence_renderer": "dx0-five-file-renderer-v1",
    }


def dx0_validate_plan(value: Any) -> dict[str, Any]:
    expected = dx0_closed_plan(DX0_PLAN_ID)
    if not isinstance(value, dict) or value != expected:
        fail("DX0 closed proof plan differs")
    return value


def dx0_deck_execution_input(commit: str, host_manifest_sha256: str,
                             fixture_identity_sha256: str,
                             plan_sha256: str,
                             *, root: pathlib.Path | None = None) -> dict[str, Any]:
    for label, digest in (
        ("host artifact manifest", host_manifest_sha256),
        ("accepted fixture identity", fixture_identity_sha256),
        ("proof plan", plan_sha256),
    ):
        if not re.fullmatch(r"[0-9a-f]{64}", digest):
            fail(f"DX0 {label} digest is malformed")
    records = dx0_records(commit, DX0_DECK_EXECUTION_PATHS, root=root)
    return {
        "schema": DX0_DECK_EXECUTION_INPUT_SCHEMA,
        "host_artifact_manifest_sha256": host_manifest_sha256,
        "accepted_fixture_identity_sha256": fixture_identity_sha256,
        "proof_plan_sha256": plan_sha256,
        "runtime_proton_sha256": RUNNER_DIGEST,
        "record_count": len(records),
        "records": records,
    }


def dx0_evidence_renderer(commit: str, *, root: pathlib.Path | None = None) -> dict[str, Any]:
    records = dx0_records(commit, DX0_RENDERER_PATHS, root=root)
    return {
        "schema": DX0_EVIDENCE_RENDERER_SCHEMA,
        "record_count": len(records),
        "records": records,
        "evidence_schema": DX0_PACKET_SCHEMA,
        "evidence_paths": list(DX0_EVIDENCE_PATHS),
    }


def dx0_source_role(value: dict[str, Any]) -> dict[str, Any]:
    identity_sha = dx0_complete_source_sha256(value)
    record_sha = dx0_record_manifest_sha256(
        DX0_COMPLETE_SOURCE_SCHEMA, value["records"]
    )
    return {
        "identity_sha256": identity_sha,
        "commit": value["commit"],
        "tree": value["tree"],
        "parent": value["parent"],
        "ref": value["ref"],
        "manifest_sha256": record_sha,
    }


def dx0_mac_proof_root() -> pathlib.Path:
    return real_home() / "Library/Application Support/Linux VST Bridge/proof"


def dx0_mac_fixture_parent() -> pathlib.Path:
    return dx0_mac_proof_root() / "fixtures/by-manifest"


def dx0_mac_host_artifact_parent() -> pathlib.Path:
    return dx0_mac_proof_root() / "host-artifacts/by-manifest"


def dx0_mac_transaction_parent() -> pathlib.Path:
    return dx0_mac_proof_root() / "transactions/by-id"


def dx0_mac_result_parent() -> pathlib.Path:
    return dx0_mac_proof_root() / "results/by-execution-input"


def dx0_deck_fixture_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/fixtures/by-manifest"


def dx0_deck_host_artifact_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/host-artifacts/by-manifest"


def dx0_deck_result_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/proof/results/by-execution-input"


def dx0_deck_source_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/handoffs/dx0/source/by-commit"


def dx0_deck_worktree_parent() -> pathlib.Path:
    return real_home() / ".local/share/linux-vst-bridge/worktrees/dx0"

# AP0 extends only the existing producer's exact input roster and source role.
AP0_BRANCH = "codex/ap0-offline-again-processing"
AP0_REF = "refs/heads/" + AP0_BRANCH
AP0_WINDOWS_BUILD_PATHS = tuple(sorted((*DX0_WINDOWS_BUILD_PATHS,
    "windows-factory-probe/source/offline_processing.cpp",
    "windows-factory-probe/source/offline_processing.h")))

def ap0_complete_source(commit, *, root=None):
    repository = root or repo_root()
    command(["git", "merge-base", "--is-ancestor",
             "b07cd337823a12fd0100a47d66733675ebf8bbb3", commit], cwd=repository)
    value = dx0_complete_source(commit, root=repository, strict=False)
    value["ref"] = AP0_REF
    return value

def ap0_windows_build_input(commit, *, root=None):
    value = dx0_windows_build_input(commit, root=root)
    value["records"] = dx0_records(commit, AP0_WINDOWS_BUILD_PATHS, root=root)
    value["record_count"] = len(value["records"])
    return value


AP1_BRANCH = "codex/ap1-linux-windows-audio-roundtrip"
AP1_REF = "refs/heads/" + AP1_BRANCH
AP1_WINDOWS_BUILD_PATHS = tuple(sorted((*AP0_WINDOWS_BUILD_PATHS,
    "windows-factory-probe/source/ap1_protocol.h",
    "windows-factory-probe/source/mapped_processing.cpp",
    "windows-factory-probe/source/mapped_processing.h")))

def ap1_complete_source(commit, *, root=None):
    repository = root or repo_root()
    command(["git", "merge-base", "--is-ancestor",
             "221c22df75ef090a0445233eefb71a7e29aa3f88", commit], cwd=repository)
    value = dx0_complete_source(commit, root=repository, strict=False)
    value["ref"] = AP1_REF
    return value

def ap1_windows_build_input(commit, *, root=None):
    value = dx0_windows_build_input(commit, root=root)
    value["records"] = dx0_records(commit, AP1_WINDOWS_BUILD_PATHS, root=root)
    value["record_count"] = len(value["records"])
    return value


# AP2 retains the AP1 source roster, extending its native host lifecycle mode.
AP2_BRANCH = "codex/ap2-native-vst3-offline-bridge"
AP2_REF = "refs/heads/" + AP2_BRANCH
AP2_WINDOWS_BUILD_PATHS = AP1_WINDOWS_BUILD_PATHS

def ap2_complete_source(commit, *, root=None):
    repository = root or repo_root()
    command(["git", "merge-base", "--is-ancestor",
             "d2e4e9f18dce1e0322aed152cf3309d2f8267a92", commit], cwd=repository)
    value = dx0_complete_source(commit, root=repository, strict=False)
    value["ref"] = AP2_REF
    return value

def ap2_windows_build_input(commit, *, root=None):
    value = dx0_windows_build_input(commit, root=root)
    value["records"] = dx0_records(commit, AP2_WINDOWS_BUILD_PATHS, root=root)
    value["record_count"] = len(value["records"])
    return value

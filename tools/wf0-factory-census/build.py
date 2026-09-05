#!/usr/bin/env python3
"""WindowsBuildPlane acquisition, comparison, and WA0 envelope publication."""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from typing import Any, Iterable, Sequence

sys.dont_write_bytecode = True

from common import (
    ARTIFACT_SCHEMA, BASIS_COMMIT, BASIS_TREE, EXPECTED_BRANCH, EXPECTED_REF,
    FAULT_TARGETS, REPOSITORY, SDK_COMMIT, SDK_POSITIVE_FIXTURE_BLOBS, SDK_REMOTES,
    SDK_SOURCE_BLOBS, SDK_SUBMODULES,
    SDK_TREE, WINDOWS_BUILD_SCHEMA, WORKFLOW_PATH, canonical_json, command,
    command_text, fail, repo_root, require_clean_source, sha256_bytes,
    sha256_file, source_manifest_sha256, write_atomic,
)
from common import (
    AP0_BRANCH as DX0_BRANCH, AP0_WINDOWS_BUILD_PATHS, DX0_HOST_ARTIFACT_SCHEMA, DX0_HOST_BUILD_SCHEMA, DX0_HOST_MODE,
    AP0_REF as DX0_REF, DX0_WINDOWS_BUILD_INPUT_SCHEMA, ap0_complete_source as dx0_complete_source,
    dx0_identity_sha256, dx0_source_role, ap0_windows_build_input as dx0_windows_build_input,
)
from verify import (
    artifact_file_records, artifact_manifest, compare_builds,
    scanner_component_call_surface, verify_pe,
)


GENERATOR = "Visual Studio 17 2022"
PLATFORM = "x64"
TOOLSET = "v143"
WINDOWS_SDK = "10.0.19041.0"
CONFIGURATION = "Release"
SOURCE_DATE_EPOCH = "1788314715"
EXPECTED_AGAIN_MODULE_SHA256 = (
    "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
)
MSBUILD_MAX_CPU_COUNT = "1"
MSVC_POST_OPTIONS = "/MP1"
BUILD_TARGETS = (
    "wf0-factory-probe", "wf0-loader-adapter-tests", "wa0-fault-fixtures", "again",
)
SDK_LICENSES = {
    "LICENSE.txt": "licenses/vst3sdk.txt",
    "base/LICENSE.txt": "licenses/vst3-base.txt",
    "cmake/LICENSE.txt": "licenses/vst3-cmake.txt",
    "doc/LICENSE.txt": "licenses/vst3-doc.txt",
    "pluginterfaces/LICENSE.txt": "licenses/vst3-pluginterfaces.txt",
    "public.sdk/LICENSE.txt": "licenses/vst3-public-sdk.txt",
    "vstgui4/LICENSE": "licenses/vstgui.txt",
}
CHECKOUT_CONFIGURATION = {
    "core.autocrlf": "false",
    "core.eol": "lf",
}


def _require_empty_new(path: pathlib.Path, label: str) -> None:
    if path.exists() or path.is_symlink():
        fail(f"{label} already exists; reuse or repair is prohibited")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.mkdir()


def git_checkout_environment(
    base: dict[str, str] | None = None, *, allow_file_protocol: bool = False,
) -> dict[str, str]:
    """Return a command-scoped Git environment that cannot inherit EOL policy."""
    environment = dict(os.environ if base is None else base)
    for key in list(environment):
        if re.fullmatch(r"GIT_CONFIG_(?:KEY|VALUE)_\d+", key):
            del environment[key]
    entries = list(CHECKOUT_CONFIGURATION.items())
    if allow_file_protocol:
        entries.append(("protocol.file.allow", "always"))
    environment["GIT_CONFIG_COUNT"] = str(len(entries))
    for index, (key, value) in enumerate(entries):
        environment[f"GIT_CONFIG_KEY_{index}"] = key
        environment[f"GIT_CONFIG_VALUE_{index}"] = value
    environment["GIT_TERMINAL_PROMPT"] = "0"
    return environment


def _git(
    root: pathlib.Path, *args: str, timeout: float = 300.0,
    env: dict[str, str] | None = None,
) -> str:
    return command_text(
        ["git", "-C", str(root), *args],
        timeout=timeout,
        env=git_checkout_environment() if env is None else env,
    )


def _local_checkout_configuration(
    root: pathlib.Path, *, env: dict[str, str] | None = None,
) -> dict[str, str]:
    observed: dict[str, str] = {}
    for key, expected in CHECKOUT_CONFIGURATION.items():
        values = _git(root, "config", "--local", "--get-all", key, env=env).splitlines()
        if values != [expected]:
            fail(f"Git checkout configuration differs at {root.name}: {key}={values}")
        observed[key] = expected
    return observed


def _set_local_checkout_configuration(
    root: pathlib.Path, *, env: dict[str, str] | None = None,
) -> dict[str, str]:
    for key, value in CHECKOUT_CONFIGURATION.items():
        _git(root, "config", "--local", "--replace-all", key, value, env=env)
    return _local_checkout_configuration(root, env=env)


def _sdk_source_record(
    root: pathlib.Path, relative: str, expected_blob: str, expected_sha256: str,
) -> dict[str, str]:
    owner = "root"
    repository = root
    repository_commit = SDK_COMMIT
    repository_path = relative
    for name, commit in sorted(SDK_SUBMODULES.items()):
        prefix = f"{name}/"
        if relative.startswith(prefix):
            owner = name
            repository = root / name
            repository_commit = commit
            repository_path = relative[len(prefix):]
            break
    path = root / relative
    locked_blob = _git(
        repository, "rev-parse", f"{repository_commit}:{repository_path}"
    )
    if locked_blob != expected_blob:
        fail(f"SDK locked repository blob differs: {relative}")
    unfiltered_blob = _git(
        repository, "hash-object", "--no-filters", "--", str(path)
    )
    if unfiltered_blob != expected_blob:
        fail(f"SDK unfiltered worktree blob differs: {relative}")
    observed_sha256 = sha256_file(path)
    if observed_sha256 != expected_sha256:
        fail(f"SDK raw worktree SHA-256 differs: {relative}")
    return {
        "path": relative,
        "repository": owner,
        "repository_commit": repository_commit,
        "repository_path": repository_path,
        "git_blob": locked_blob,
        "unfiltered_worktree_git_blob": unfiltered_blob,
        "sha256": observed_sha256,
    }


def eol_checkout_regression() -> dict[str, Any]:
    """Prove hostile ambient autocrlf cannot alter root or submodule bytes."""
    lf_root = b"root alpha\nroot beta\n"
    crlf_root = lf_root.replace(b"\n", b"\r\n")
    lf_child = b"child alpha\nchild beta\n"
    with tempfile.TemporaryDirectory(prefix="wf0-eol-regression-") as temporary:
        fixture = pathlib.Path(temporary)
        hostile_config = fixture / "hostile.gitconfig"
        hostile_config.write_bytes(b"[core]\n\tautocrlf = true\n\teol = crlf\n")
        hostile = dict(os.environ)
        hostile["GIT_CONFIG_GLOBAL"] = str(hostile_config)
        controlled = git_checkout_environment(hostile, allow_file_protocol=True)
        author = dict(controlled)
        author.update({
            "GIT_AUTHOR_NAME": "WF0 Regression",
            "GIT_AUTHOR_EMAIL": "wf0-regression.invalid",
            "GIT_COMMITTER_NAME": "WF0 Regression",
            "GIT_COMMITTER_EMAIL": "wf0-regression.invalid",
        })

        child_source = fixture / "child-source"
        child_source.mkdir()
        _git(child_source, "init", env=author)
        _set_local_checkout_configuration(child_source, env=author)
        child_file = child_source / "child-locked.txt"
        child_file.write_bytes(lf_child)
        _git(child_source, "add", "child-locked.txt", env=author)
        _git(child_source, "commit", "-m", "child fixture", env=author)
        child_commit = _git(child_source, "rev-parse", "HEAD", env=author)
        child_blob = _git(
            child_source, "rev-parse", f"{child_commit}:child-locked.txt", env=author
        )

        root_source = fixture / "root-source"
        root_source.mkdir()
        _git(root_source, "init", env=author)
        _set_local_checkout_configuration(root_source, env=author)
        root_file = root_source / "root-locked.txt"
        root_file.write_bytes(lf_root)
        _git(root_source, "add", "root-locked.txt", env=author)
        _git(
            root_source, "submodule", "add", str(child_source), "child", env=author
        )
        _git(root_source, "commit", "-m", "root fixture", env=author)
        root_commit = _git(root_source, "rev-parse", "HEAD", env=author)
        root_blob = _git(
            root_source, "rev-parse", f"{root_commit}:root-locked.txt", env=author
        )

        checkout = fixture / "checkout"
        command(
            ["git", "clone", "--no-checkout", str(root_source), str(checkout)],
            env=controlled,
        )
        root_configuration = _set_local_checkout_configuration(checkout, env=controlled)
        _git(checkout, "checkout", "--detach", root_commit, env=controlled)
        _git(
            checkout, "submodule", "update", "--init", "--recursive",
            env=controlled,
        )
        child_checkout = checkout / "child"
        child_configuration = _set_local_checkout_configuration(
            child_checkout, env=controlled
        )

        checked_root = checkout / "root-locked.txt"
        checked_child = child_checkout / "child-locked.txt"
        if checked_root.read_bytes() != lf_root or checked_child.read_bytes() != lf_child:
            fail("controlled checkout did not materialize canonical LF fixture bytes")
        if _git(checkout, "rev-parse", f"{root_commit}:root-locked.txt", env=controlled) != root_blob:
            fail("synthetic root locked blob changed")
        if _git(
            checkout, "hash-object", "--no-filters", "--", str(checked_root),
            env=controlled,
        ) != root_blob or sha256_file(checked_root) != sha256_bytes(lf_root):
            fail("synthetic root raw LF verification failed")
        if _git(
            child_checkout, "rev-parse", f"{child_commit}:child-locked.txt",
            env=controlled,
        ) != child_blob:
            fail("synthetic submodule locked blob changed")
        if _git(
            child_checkout, "hash-object", "--no-filters", "--", str(checked_child),
            env=controlled,
        ) != child_blob or sha256_file(checked_child) != sha256_bytes(lf_child):
            fail("synthetic submodule raw LF verification failed")

        checked_root.write_bytes(crlf_root)
        filtered_blob = command_text([
            "git", "-c", "core.autocrlf=true", "-C", str(checkout),
            "hash-object", "--path=root-locked.txt", str(checked_root),
        ], env=hostile)
        unfiltered_blob = _git(
            checkout, "hash-object", "--no-filters", "--", str(checked_root),
            env=controlled,
        )
        crlf_sha256 = sha256_file(checked_root)
        if filtered_blob != root_blob:
            fail("filtered Git hash did not reproduce the synthetic CRLF masking case")
        if unfiltered_blob == root_blob or crlf_sha256 == sha256_bytes(lf_root):
            fail("unfiltered blob or raw SHA-256 masked the synthetic CRLF mismatch")
        checked_root.write_bytes(lf_root)
        if _git(
            checkout, "hash-object", "--no-filters", "--", str(checked_root),
            env=controlled,
        ) != root_blob or sha256_file(checked_root) != sha256_bytes(lf_root):
            fail("canonical LF fixture did not pass after CRLF rejection")
        if _git(checkout, "status", "--porcelain=v1", "--untracked-files=all", env=controlled):
            fail("synthetic checkout is dirty after regression restoration")

        return {
            "schema": "linux-vst-bridge-wf0-git-checkout-regression/v1",
            "hostile_ambient_core_autocrlf": "true",
            "command_scoped_configuration": dict(CHECKOUT_CONFIGURATION),
            "root_local_configuration": root_configuration,
            "recursive_submodule_local_configuration": child_configuration,
            "root_checkout_canonical_lf": True,
            "recursive_submodule_checkout_canonical_lf": True,
            "filtered_crlf_blob_masked_mismatch": True,
            "unfiltered_crlf_blob_rejected": True,
            "raw_crlf_sha256_rejected": True,
            "canonical_lf_blob": root_blob,
            "canonical_lf_sha256": sha256_bytes(lf_root),
            "canonical_submodule_blob": child_blob,
            "canonical_submodule_sha256": sha256_bytes(lf_child),
        }


def acquire_sdk(destination: pathlib.Path) -> dict[str, Any]:
    regression = eol_checkout_regression()
    _require_empty_new(destination, "SDK acquisition root")
    destination.rmdir()
    checkout_environment = git_checkout_environment()
    command(
        ["git", "clone", "--no-checkout", SDK_REMOTES["root"], str(destination)],
        env=checkout_environment,
        timeout=900.0,
    )
    _set_local_checkout_configuration(destination, env=checkout_environment)
    _git(
        destination, "checkout", "--detach", SDK_COMMIT,
        timeout=300.0, env=checkout_environment,
    )
    paths = _git(
        destination, "config", "-f", ".gitmodules", "--get-regexp", r"^submodule\..*\.path$"
    ).splitlines()
    observed_paths = sorted(line.split(None, 1)[1] for line in paths)
    if observed_paths != sorted(SDK_SUBMODULES):
        fail(f"SDK submodule path roster differs: {observed_paths}")
    for name in sorted(SDK_SUBMODULES):
        _git(
            destination, "config", f"submodule.{name}.url", SDK_REMOTES[name],
            env=checkout_environment,
        )
    command(
        ["git", "-C", str(destination), "submodule", "update", "--init", "--recursive"],
        env=checkout_environment,
        timeout=1800.0,
    )
    for name in sorted(SDK_SUBMODULES):
        _set_local_checkout_configuration(destination / name, env=checkout_environment)
    identity = verify_sdk(destination)
    identity["checkout_regression"] = regression
    return identity


def verify_sdk(root: pathlib.Path) -> dict[str, Any]:
    if not root.is_absolute() or root.is_symlink() or not (root / ".git").exists():
        fail("SDK root is not an exact absolute Git checkout")
    if _git(root, "rev-parse", "HEAD") != SDK_COMMIT:
        fail("SDK root commit differs")
    if _git(root, "rev-parse", "HEAD^{tree}") != SDK_TREE:
        fail("SDK root tree differs")
    if _git(root, "status", "--porcelain=v1", "--untracked-files=all"):
        fail("SDK root is dirty")
    if _git(root, "remote", "get-url", "origin") != SDK_REMOTES["root"]:
        fail("SDK root remote is outside the official allow-list")
    root_checkout_configuration = _local_checkout_configuration(root)
    status_lines = _git(root, "submodule", "status", "--recursive").splitlines()
    if len(status_lines) != len(SDK_SUBMODULES):
        fail(f"SDK recursive submodule count differs: {len(status_lines)}")
    submodules: list[dict[str, Any]] = []
    for name, expected in sorted(SDK_SUBMODULES.items()):
        child = root / name
        if _git(root, "rev-parse", f"HEAD:{name}") != expected:
            fail(f"SDK gitlink differs: {name}")
        if _git(child, "rev-parse", "HEAD") != expected:
            fail(f"SDK submodule commit differs: {name}")
        if _git(child, "status", "--porcelain=v1", "--untracked-files=all"):
            fail(f"SDK submodule is dirty: {name}")
        if _git(child, "remote", "get-url", "origin") != SDK_REMOTES[name]:
            fail(f"SDK submodule remote is outside the official allow-list: {name}")
        submodules.append({
            "path": name,
            "commit": expected,
            "checkout_configuration": _local_checkout_configuration(child),
        })
    source_blobs: list[dict[str, str]] = []
    for relative, (expected_blob, expected_sha256) in sorted(SDK_SOURCE_BLOBS.items()):
        source_blobs.append(
            _sdk_source_record(root, relative, expected_blob, expected_sha256)
        )
    positive_blobs: list[dict[str, str]] = []
    for relative, (expected_blob, expected_sha256) in sorted(
        SDK_POSITIVE_FIXTURE_BLOBS.items()
    ):
        positive_blobs.append(
            _sdk_source_record(root, relative, expected_blob, expected_sha256)
        )
    return {
        "root_commit": SDK_COMMIT,
        "root_tree": SDK_TREE,
        "root_checkout_configuration": root_checkout_configuration,
        "submodules": submodules,
        "source_patch_count": 0,
        "dirty_path_count": 0,
        "failure_source_blobs": source_blobs,
        "positive_fixture_source_blobs": positive_blobs,
    }


def source_identity(source_commit: str) -> tuple[dict[str, Any], str, str, str]:
    root = repo_root()
    if os.environ.get("GITHUB_REPOSITORY") != REPOSITORY:
        fail("workflow repository identity differs")
    if os.environ.get("GITHUB_EVENT_NAME") != "push":
        fail("workflow event is not the fixed push event")
    if os.environ.get("GITHUB_REF") != EXPECTED_REF:
        fail("workflow ref is not the exact implementation branch")
    if os.environ.get("GITHUB_SHA") != source_commit:
        fail("workflow github.sha differs from requested source")
    if not re.fullmatch(r"[0-9a-f]{40}", source_commit):
        fail("source commit is malformed")
    source = require_clean_source(source_commit, detached=None)
    tree = _git(root, "rev-parse", "HEAD^{tree}")
    parent = _git(root, "rev-parse", "HEAD^")
    if parent != BASIS_COMMIT or _git(root, "rev-parse", f"{BASIS_COMMIT}^{{tree}}") != BASIS_TREE:
        fail("workflow source is not the exact direct authority child")
    workflow_blob = _git(root, "rev-parse", f"HEAD:{WORKFLOW_PATH}")
    digest = source_manifest_sha256(source)
    return source, digest, tree, workflow_blob


def _bounded_tool_output(argv: Sequence[str], *, accepted: Iterable[int] = (0,)) -> bytes:
    result = command(argv, check=False, timeout=120.0)
    if result.returncode not in set(accepted):
        fail(f"tool identity command failed: {argv[0]} ({result.returncode})")
    value = result.stdout + result.stderr
    if not value or len(value) > 1024 * 1024:
        fail(f"tool identity output is absent or oversized: {argv[0]}")
    return value


def toolchain_identity() -> tuple[dict[str, Any], bytes]:
    if platform.machine().upper() not in {"AMD64", "X86_64"}:
        fail(f"Windows runner architecture differs: {platform.machine()}")
    if os.environ.get("RUNNER_ARCH", "").upper() != "X64":
        fail("RUNNER_ARCH is not X64")
    image_os = os.environ.get("ImageOS", "")
    image_version = os.environ.get("ImageVersion", "")
    if not image_os or not image_version:
        fail("hosted runner image identity is absent")
    sdk = os.environ.get("WindowsSDKVersion", "").rstrip("\\/")
    if sdk != WINDOWS_SDK:
        fail(f"selected Windows SDK differs: {sdk}")
    vscmd = os.environ.get("VSCMD_VER", "")
    if not vscmd:
        fail("VSCMD version is absent")

    program_files_x86 = os.environ.get("ProgramFiles(x86)")
    if not program_files_x86:
        fail("ProgramFiles(x86) is absent")
    vswhere = pathlib.Path(program_files_x86) / "Microsoft Visual Studio/Installer/vswhere.exe"
    vs_json = _bounded_tool_output(
        [str(vswhere), "-latest", "-products",
         "Microsoft.VisualStudio.Product.Enterprise", "-requires",
         "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-format", "json", "-utf8"]
    )
    inventory = json.loads(vs_json)
    if not isinstance(inventory, list) or len(inventory) != 1:
        fail("Visual Studio inventory is not one bounded selected installation")
    selected = inventory[0]
    version = str(selected.get("installationVersion", ""))
    product_id = str(selected.get("productId", ""))
    selected_path = str(selected.get("installationPath", ""))
    if (
        not version.startswith("17.")
        or product_id != "Microsoft.VisualStudio.Product.Enterprise"
        or pathlib.Path(selected_path) != pathlib.Path(
            os.environ.get("WF0_VS_INSTALLATION", "")
        )
    ):
        fail(f"Visual Studio edition/version differs: {product_id} {version}")

    cl_bv = _bounded_tool_output(["cl", "/Bv"], accepted=(0, 2))
    cl_text = cl_bv.decode("utf-8", "replace")
    cl_match = re.search(r"Compiler Version\s+([0-9.]+)\s+for x64", cl_text, re.IGNORECASE)
    if cl_match is None:
        fail("complete cl /Bv output lacks the x64 compiler version")
    link_output = _bounded_tool_output(["link", "/?"], accepted=(0, 1100, 1104))
    link_text = link_output.decode("utf-8", "replace")
    link_match = re.search(r"Linker Version\s+([0-9.]+)", link_text, re.IGNORECASE)
    if link_match is None:
        fail("linker identity line is absent")
    cmake_output = _bounded_tool_output(["cmake", "--version"])
    cmake_match = re.search(rb"cmake version\s+([0-9.]+)", cmake_output)
    if cmake_match is None:
        fail("CMake version is absent")
    if tuple(int(item) for item in cmake_match.group(1).decode().split(".")[:2]) < (3, 25):
        fail("CMake is older than 3.25")

    os_output = _bounded_tool_output([
        "powershell", "-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
        "$o=Get-CimInstance Win32_OperatingSystem;"
        "[ordered]@{Caption=$o.Caption;Version=$o.Version;BuildNumber=$o.BuildNumber}"
        "|ConvertTo-Json -Compress",
    ])
    os_identity = json.loads(os_output)
    return ({
        "runner": {
            "requested_label": "windows-2022",
            "architecture": "X64",
            "image_os": image_os,
            "image_version": image_version,
            "windows_product_name": os_identity["Caption"],
            "windows_version": os_identity["Version"],
            "windows_build": os_identity["BuildNumber"],
        },
        "toolchain": {
            "visual_studio_edition": "Enterprise",
            "visual_studio_version": version,
            "visual_studio_installation_sha256": sha256_bytes(vs_json),
            "vscmd_version": vscmd,
            "platform_toolset": TOOLSET,
            "host_architecture": "x64",
            "target_architecture": "x64",
            "cl_bv_sha256": sha256_bytes(cl_bv),
            "cl_version": cl_match.group(1),
            "link_version": link_match.group(1),
            "windows_sdk_version": WINDOWS_SDK,
            "cmake_version": cmake_match.group(1).decode(),
            "generator": GENERATOR,
            "generator_platform": PLATFORM,
            "generator_toolset": TOOLSET,
        },
    }, cl_bv)


def cmake_contract_args(build_root: str) -> list[str]:
    if not build_root:
        fail("deterministic build-root mapping is absent")
    compiler_flags = f"/Brepro -pathmap:{build_root}=C:\\wf0\\build"
    linker_flags = "/Brepro /INCREMENTAL:NO /PDBALTPATH:%_PDB%"
    return [
        "-G", GENERATOR,
        "-A", PLATFORM,
        "-T", TOOLSET,
        f"-DCMAKE_SYSTEM_VERSION={WINDOWS_SDK}",
        "-DCMAKE_POLICY_DEFAULT_CMP0091=NEW",
        "-DCMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded",
        "-DCMAKE_MSVC_DEBUG_INFORMATION_FORMAT=",
        "-DSMTG_USE_STATIC_CRT=ON",
        "-DCMAKE_FIND_USE_PACKAGE_REGISTRY=OFF",
        "-DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF",
        "-DFETCHCONTENT_FULLY_DISCONNECTED=ON",
        f"-DCMAKE_C_FLAGS={compiler_flags}",
        f"-DCMAKE_CXX_FLAGS={compiler_flags}",
        f"-DCMAKE_EXE_LINKER_FLAGS={linker_flags}",
        f"-DCMAKE_SHARED_LINKER_FLAGS={linker_flags}",
        f"-DCMAKE_MODULE_LINKER_FLAGS={linker_flags}",
    ]


def build_command_environment(
    base: dict[str, str] | None = None,
) -> dict[str, str]:
    """Return the closed build environment, including serial compiler control."""
    environment = dict(os.environ if base is None else base)
    environment.update({
        "CL": "",
        # The pinned SDK adds an unbounded /MP option. Appending /MP1 through
        # _CL_ makes compilation order independent of hosted-runner CPU policy.
        "_CL_": MSVC_POST_OPTIONS,
        "LINK": "",
        "_LINK_": "",
        "SOURCE_DATE_EPOCH": SOURCE_DATE_EPOCH,
        "TZ": "UTC",
        "GIT_TERMINAL_PROMPT": "0",
        "CMAKE_TLS_VERIFY": "ON",
        "VCPKG_DISABLE_METRICS": "1",
        "HTTP_PROXY": "http://127.0.0.1:9",
        "HTTPS_PROXY": "http://127.0.0.1:9",
        "ALL_PROXY": "http://127.0.0.1:9",
        "NO_PROXY": "",
        "GIT_CONFIG_COUNT": "2",
        "GIT_CONFIG_KEY_0": "http.proxy",
        "GIT_CONFIG_VALUE_0": "http://127.0.0.1:9",
        "GIT_CONFIG_KEY_1": "https.proxy",
        "GIT_CONFIG_VALUE_1": "http://127.0.0.1:9",
    })
    return environment


def _build_command(argv: Sequence[str], *, timeout: float = 1800.0) -> None:
    environment = build_command_environment()
    command(argv, env=environment, timeout=timeout)


def configure_and_build(root: pathlib.Path, sdk: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    _require_empty_new(root, "build root")
    repository_build = root / "repository"
    again_build = root / "again"
    assembled = root / "assembled"
    assembled.mkdir()
    args = cmake_contract_args(str(root))
    _build_command([
        "cmake", "-S", str(repo_root()), "-B", str(repository_build), *args,
        f"-DWF0_VST3_SDK_ROOT:PATH={sdk}", "-DWF0_BUILD_ONLY=ON",
    ])
    _build_command([
        "cmake", "--build", str(repository_build), "--config", CONFIGURATION,
        "--target", "wf0-factory-probe", "wf0-loader-adapter-tests",
        "wa0-fault-fixtures", "--", f"/m:{MSBUILD_MAX_CPU_COUNT}",
    ])
    verify_sdk(sdk)
    _build_command([
        "cmake", "-S", str(sdk), "-B", str(again_build), *args,
        "-DSMTG_ENABLE_VST3_PLUGIN_EXAMPLES=ON",
        "-DSMTG_ENABLE_VST3_HOSTING_EXAMPLES=OFF",
        "-DSMTG_ENABLE_VSTGUI_SUPPORT=ON",
        "-DSMTG_ENABLE_WAYLAND_SUPPORT=OFF",
        "-DSMTG_CREATE_BUNDLE_FOR_WINDOWS=ON",
        "-DSMTG_RUN_VST_VALIDATOR=OFF",
        "-DSMTG_CREATE_MODULE_INFO=OFF",
        "-DSMTG_CREATE_PLUGIN_LINK=OFF",
    ])
    _build_command([
        "cmake", "--build", str(again_build), "--config", CONFIGURATION,
        "--target", "again", "--", f"/m:{MSBUILD_MAX_CPU_COUNT}",
    ])
    verify_sdk(sdk)
    return repository_build, again_build


def unique_file(root: pathlib.Path, name: str) -> pathlib.Path:
    matches = [item for item in root.rglob(name) if item.is_file() and not item.is_symlink()]
    if len(matches) != 1:
        fail(f"build output is not unique: {name}: {[str(item) for item in matches]}")
    return matches[0]


def _copy_regular(source: pathlib.Path, destination: pathlib.Path) -> None:
    if not source.is_file() or source.is_symlink():
        fail(f"build output is absent or unsafe: {source.name}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    if sha256_file(source) != sha256_file(destination):
        fail(f"assembled copy differs: {destination.name}")


def _copy_tree_regular(source: pathlib.Path, destination: pathlib.Path) -> None:
    if source.is_symlink() or not source.is_dir():
        fail("AGain bundle root is absent or unsafe")
    for item in source.rglob("*"):
        if item.is_symlink() or (item.exists() and not (item.is_file() or item.is_dir())):
            fail("AGain bundle contains an unsafe object")
    shutil.copytree(source, destination)


def assemble(repository_build: pathlib.Path, again_build: pathlib.Path,
             sdk: pathlib.Path, destination: pathlib.Path) -> dict[str, str]:
    destination.mkdir(parents=True, exist_ok=False)
    roles: dict[str, str] = {}
    scanner = unique_file(repository_build, "wf0-factory-probe.exe")
    adapter = unique_file(repository_build, "wf0-loader-adapter-tests.exe")
    _copy_regular(scanner, destination / "bin/wf0-factory-probe.exe")
    _copy_regular(adapter, destination / "bin/wf0-loader-adapter-tests.exe")
    roles["bin/wf0-factory-probe.exe"] = "scanner_executable"
    roles["bin/wf0-loader-adapter-tests.exe"] = "loader_adapter_executable"
    for target in FAULT_TARGETS:
        relative = f"fixtures/{target}.dll"
        _copy_regular(unique_file(repository_build, f"{target}.dll"), destination / relative)
        roles[relative] = "approved_fault_module"
    # The accepted immutable environment owner stages its adapter transaction with
    # this historical carrier path. WA0 copies one focused module there only as
    # verified inert payload; the adapter never loads it and no WF0 fixture is built.
    carrier = "fixtures/wf0-no-entry.dll"
    _copy_regular(
        destination / f"fixtures/{FAULT_TARGETS[0]}.dll",
        destination / carrier,
    )
    roles[carrier] = "adapter_environment_carrier"

    module_matches = [
        item for item in again_build.rglob("again.vst3")
        if item.is_file() and item.parent.name == "x86_64-win"
        and item.parent.parent.name == "Contents"
    ]
    if len(module_matches) != 1:
        fail(f"exact AGain module is not unique: {[str(item) for item in module_matches]}")
    observed_again_sha256 = sha256_file(module_matches[0])
    if observed_again_sha256 != EXPECTED_AGAIN_MODULE_SHA256:
        fail(
            "exact accepted AGain module SHA-256 differs: "
            f"{observed_again_sha256}"
        )
    bundle = module_matches[0].parents[2]
    _copy_tree_regular(bundle, destination / "again.vst3")
    for item in sorted((destination / "again.vst3").rglob("*")):
        if item.is_file():
            relative = item.relative_to(destination).as_posix()
            roles[relative] = (
                "positive_fixture_module"
                if relative == "again.vst3/Contents/x86_64-win/again.vst3"
                else "positive_fixture_resource"
            )

    for source_name, relative in SDK_LICENSES.items():
        _copy_regular(sdk / source_name, destination / relative)
        roles[relative] = "required_license_notice"
    return roles


def dumpbin(path: pathlib.Path) -> str:
    output = _bounded_tool_output(["dumpbin", "/headers", "/imports", "/exports", str(path)])
    return output.decode("utf-8", "replace")


def verify_assembled(root: pathlib.Path, roles: dict[str, str]) -> list[dict[str, Any]]:
    pe: list[dict[str, Any]] = []
    pe.append({
        "path": "bin/wf0-factory-probe.exe",
        **verify_pe(
            root / "bin/wf0-factory-probe.exe",
            dumpbin(root / "bin/wf0-factory-probe.exe"),
            forbidden_exports={"GetPluginFactory", "InitDll", "ExitDll"},
            executable=True,
        ),
    })
    pe.append({
        "path": "bin/wf0-loader-adapter-tests.exe",
        **verify_pe(
            root / "bin/wf0-loader-adapter-tests.exe",
            dumpbin(root / "bin/wf0-loader-adapter-tests.exe"),
            forbidden_exports={"GetPluginFactory", "InitDll", "ExitDll"},
            executable=True,
        ),
    })
    for target in FAULT_TARGETS:
        path = root / f"fixtures/{target}.dll"
        required = {"GetPluginFactory", "InitDll", "ExitDll"}
        forbidden: set[str] = set()
        pe.append({
            "path": f"fixtures/{target}.dll",
            **verify_pe(path, dumpbin(path), required_exports=required,
                        forbidden_exports=forbidden),
        })
    carrier = root / "fixtures/wf0-no-entry.dll"
    pe.append({
        "path": "fixtures/wf0-no-entry.dll",
        **verify_pe(
            carrier, dumpbin(carrier),
            required_exports={"GetPluginFactory", "InitDll", "ExitDll"},
        ),
    })
    again = root / "again.vst3/Contents/x86_64-win/again.vst3"
    pe.append({
        "path": "again.vst3/Contents/x86_64-win/again.vst3",
        **verify_pe(
            again, dumpbin(again),
            required_exports={"GetPluginFactory", "InitDll", "ExitDll"},
        ),
    })
    known = {item["path"] for item in pe}
    for candidate in sorted((root / "again.vst3").rglob("*")):
        relative = candidate.relative_to(root).as_posix()
        if (
            candidate.is_file()
            and candidate.suffix.lower() in {".dll", ".exe", ".vst3"}
            and relative not in known
        ):
            pe.append({"path": relative, **verify_pe(candidate, dumpbin(candidate))})
    artifact_file_records(root, roles)
    return pe


def write_manifest(root: pathlib.Path, roles: dict[str, str],
                   source_digest: str) -> tuple[dict[str, Any], str]:
    core = root / "BUILD_IDENTITY_CORE.json"
    if not core.is_file():
        fail("build identity core is absent")
    records = artifact_file_records(root, roles)
    value = artifact_manifest(source_digest, sha256_file(core), records)
    data = canonical_json(value)
    write_atomic(root / "ARTIFACT_MANIFEST.json", data, mode=0o444)
    digest = sha256_bytes(data)
    write_atomic(
        root / "ARTIFACT_MANIFEST.sha256",
        f"{digest}  ARTIFACT_MANIFEST.json\n".encode(),
        mode=0o444,
    )
    return value, digest


def deterministic_zip(source: pathlib.Path, destination: pathlib.Path) -> None:
    if destination.exists() or destination.is_symlink():
        fail("payload ZIP target already exists")
    with zipfile.ZipFile(destination, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for path in sorted(
            (item for item in source.rglob("*") if item.is_file()),
            key=lambda item: item.relative_to(source).as_posix().encode(),
        ):
            relative = path.relative_to(source).as_posix()
            info = zipfile.ZipInfo(relative, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = (0o100444 & 0xFFFF) << 16
            archive.writestr(info, path.read_bytes(), compress_type=zipfile.ZIP_DEFLATED,
                             compresslevel=9)


def dx0_source_identity(source_commit: str) -> tuple[dict[str, Any], dict[str, Any], str]:
    """Verify the dispatch checkout and reproduce both source identity domains."""
    root = repo_root()
    phase_nonce = os.environ.get("DX0_PHASE_NONCE", "")
    if (
        os.environ.get("GITHUB_REPOSITORY") != REPOSITORY
        or os.environ.get("GITHUB_EVENT_NAME") != "workflow_dispatch"
        or os.environ.get("GITHUB_REF") != DX0_REF
        or os.environ.get("GITHUB_SHA") != source_commit
        or os.environ.get("DX0_SOURCE_SHA") != source_commit
        or os.environ.get("DX0_HOST_MODE") != DX0_HOST_MODE
        or not re.fullmatch(r"[0-9a-f]{32}", phase_nonce)
    ):
        fail("DX0 workflow dispatch/source/mode/nonce identity differs")
    if command_text(["git", "status", "--porcelain=v1", "--untracked-files=all"], cwd=root):
        fail("DX0 Windows checkout is dirty")
    source = dx0_complete_source(source_commit)
    build_input = dx0_windows_build_input(source_commit)
    digest = dx0_identity_sha256(build_input)
    if (
        os.environ.get("DX0_BUILD_INPUT_SHA256") != digest
        or build_input.get("schema") != DX0_WINDOWS_BUILD_INPUT_SCHEMA
        or build_input.get("record_count") != len(AP0_WINDOWS_BUILD_PATHS)
    ):
        fail("DX0 Windows-build-input identity differs from dispatch")
    workflow_blob = _git(root, "rev-parse", f"{source_commit}:{WORKFLOW_PATH}")
    return source, build_input, workflow_blob


def configure_and_build_host(root: pathlib.Path, sdk: pathlib.Path) -> pathlib.Path:
    """Build only the repository-owned scanner in one clean root."""
    _require_empty_new(root, "DX0 host build root")
    repository_build = root / "repository"
    assembled = root / "assembled"
    assembled.mkdir()
    _build_command([
        "cmake", "-S", str(repo_root()), "-B", str(repository_build),
        *cmake_contract_args(str(root)), f"-DWF0_VST3_SDK_ROOT:PATH={sdk}",
        "-DWF0_BUILD_ONLY=ON",
    ])
    _build_command([
        "cmake", "--build", str(repository_build), "--config", CONFIGURATION,
        "--target", "wf0-factory-probe", "--", f"/m:{MSBUILD_MAX_CPU_COUNT}",
    ])
    verify_sdk(sdk)
    return repository_build


def assemble_host(repository_build: pathlib.Path, sdk: pathlib.Path,
                  destination: pathlib.Path) -> dict[str, str]:
    destination.mkdir(parents=True, exist_ok=False)
    roles: dict[str, str] = {}
    _copy_regular(unique_file(repository_build, "wf0-factory-probe.exe"),
                  destination / "bin/wf0-factory-probe.exe")
    roles["bin/wf0-factory-probe.exe"] = "scanner_executable"
    for source_name, relative in SDK_LICENSES.items():
        _copy_regular(sdk / source_name, destination / relative)
        roles[relative] = "required_license_notice"
    return roles


def verify_host_assembled(root: pathlib.Path, roles: dict[str, str]) -> dict[str, Any]:
    scanner = root / "bin/wf0-factory-probe.exe"
    pe = verify_pe(scanner, dumpbin(scanner),
                   forbidden_exports={"GetPluginFactory", "InitDll", "ExitDll"},
                   executable=True)
    if set(roles) != {
        "bin/wf0-factory-probe.exe",
        *SDK_LICENSES.values(),
    }:
        fail("DX0 host-only payload roster differs")
    artifact_file_records(root, roles)
    return {"path": "bin/wf0-factory-probe.exe", **pe}


def write_host_manifest(root: pathlib.Path, roles: dict[str, str],
                        build_input_sha256: str) -> tuple[dict[str, Any], str]:
    records = artifact_file_records(root, roles)
    value = {
        "schema": DX0_HOST_ARTIFACT_SCHEMA,
        "windows_build_input_sha256": build_input_sha256,
        "build_identity_core_sha256": sha256_file(root / "DX0_BUILD_IDENTITY_CORE.json"),
        "record_count": len(records),
        "records": records,
    }
    data = canonical_json(value)
    write_atomic(root / "DX0_HOST_ARTIFACT_MANIFEST.json", data, mode=0o444)
    digest = sha256_bytes(data)
    write_atomic(root / "DX0_HOST_ARTIFACT_MANIFEST.sha256",
                 f"{digest}  DX0_HOST_ARTIFACT_MANIFEST.json\n".encode(), mode=0o444)
    return value, digest


def build_dx0_workflow(source_commit: str, sdk: pathlib.Path,
                       transaction: pathlib.Path, output: pathlib.Path) -> dict[str, Any]:
    source, build_input, workflow_blob = dx0_source_identity(source_commit)
    build_input_sha = dx0_identity_sha256(build_input)
    call_surface = scanner_component_call_surface(repo_root(), ap0=True)
    sdk_identity = verify_sdk(sdk)
    sdk_identity["checkout_regression"] = eol_checkout_regression()
    observed, cl_bv = toolchain_identity()
    _require_empty_new(transaction, "DX0 Windows build transaction")
    _require_empty_new(output, "DX0 Windows envelope output")

    repository_a = configure_and_build_host(transaction / "a", sdk)
    repository_b = configure_and_build_host(transaction / "b", sdk)
    assembled_a = transaction / "payload-a"
    assembled_b = transaction / "payload-b"
    roles_a = assemble_host(repository_a, sdk, assembled_a)
    roles_b = assemble_host(repository_b, sdk, assembled_b)
    if roles_a != roles_b:
        fail("DX0 A/B host role roster differs")
    pe_a = verify_host_assembled(assembled_a, roles_a)
    pe_b = verify_host_assembled(assembled_b, roles_b)
    if {key: value for key, value in pe_a.items() if key != "dumpbin_sha256"} != {
        key: value for key, value in pe_b.items() if key != "dumpbin_sha256"
    }:
        fail("DX0 A/B PE identity differs")
    preliminary = compare_builds(assembled_a, assembled_b)

    run_id = os.environ.get("GITHUB_RUN_ID", "")
    run_attempt = os.environ.get("GITHUB_RUN_ATTEMPT", "")
    phase_nonce = os.environ.get("DX0_PHASE_NONCE", "")
    if (not re.fullmatch(r"[1-9][0-9]*", run_id)
            or not re.fullmatch(r"[1-9][0-9]*", run_attempt)):
        fail("DX0 workflow run ID/attempt is malformed")
    producer = dx0_source_role(source)
    workflow = {
        "path": WORKFLOW_PATH, "git_blob": workflow_blob,
        "event": "workflow_dispatch", "ref": DX0_REF,
        "source_sha": source_commit, "host_mode": DX0_HOST_MODE,
        "phase_nonce": phase_nonce, "run_id": int(run_id),
        "run_attempt": int(run_attempt),
    }
    core = {
        "schema": "linux-vst-bridge-dx0-build-identity-core/v1",
        "repository": REPOSITORY, "producer_source": producer,
        "windows_build_input": build_input,
        "windows_build_input_sha256": build_input_sha,
        "workflow": workflow, "runner": observed["runner"],
        "toolchain": observed["toolchain"], "vst3_sdk": sdk_identity,
        "build": {
            "configuration": CONFIGURATION, "targets": ["wf0-factory-probe"],
            "host_mode": DX0_HOST_MODE, "again_built": False,
            "fault_targets_built": False, "adapter_built": False,
            "msvc_runtime": "MultiThreaded", "roots_distinct": True,
            "dependency_network_during_configure_build": False,
            "deterministic_build_root_mapping": "C:\\wf0\\build",
            "source_date_epoch": SOURCE_DATE_EPOCH, "pdb_embedded_path": "%_PDB%",
            "msbuild_max_cpu_count": int(MSBUILD_MAX_CPU_COUNT),
            "msvc_post_options": MSVC_POST_OPTIONS,
            "pre_manifest_comparison": preliminary,
            "wa0_interface_call_surface": call_surface,
        },
    }
    core_data = canonical_json(core)
    for assembled, roles in ((assembled_a, roles_a), (assembled_b, roles_b)):
        write_atomic(assembled / "DX0_BUILD_IDENTITY_CORE.json", core_data, mode=0o444)
        roles["DX0_BUILD_IDENTITY_CORE.json"] = "build_identity_core"
    manifest_a, digest_a = write_host_manifest(assembled_a, roles_a, build_input_sha)
    manifest_b, digest_b = write_host_manifest(assembled_b, roles_b, build_input_sha)
    if manifest_a != manifest_b or digest_a != digest_b:
        fail("DX0 A/B host manifests differ")
    comparison = compare_builds(assembled_a, assembled_b)
    payload = output / "dx0-host-payload.zip"
    deterministic_zip(assembled_a, payload)
    receipt = {
        "schema": DX0_HOST_BUILD_SCHEMA,
        "repository": REPOSITORY, "producer_source": producer,
        "windows_build_input": {
            "schema": DX0_WINDOWS_BUILD_INPUT_SCHEMA,
            "sha256": build_input_sha, "record_count": len(AP0_WINDOWS_BUILD_PATHS),
        },
        "workflow": workflow, "runner": observed["runner"],
        "toolchain": observed["toolchain"], "vst3_sdk": sdk_identity,
        "build": {
            "configuration": CONFIGURATION, "targets": ["wf0-factory-probe"],
            "host_mode": DX0_HOST_MODE, "again_built": False,
            "fault_targets_built": False, "adapter_built": False,
            "roots_distinct": True, "comparison": comparison,
            "cmake_options_sha256": sha256_bytes(canonical_json(
                cmake_contract_args("<PHYSICAL_BUILD_ROOT>"))),
        },
        "artifact": {
            "manifest_schema": DX0_HOST_ARTIFACT_SCHEMA,
            "manifest_sha256": digest_a,
            "payload_sha256": sha256_file(payload),
            "pe_receipt": pe_a,
            "record_count": manifest_a["record_count"],
        },
        "tool_receipts": {"cl_bv_sha256": sha256_bytes(cl_bv)},
        "explicit_nonclaims": ["no_windows_runtime_proof", "no_fixture_build",
                               "no_fault_build", "no_audio_processor_method",
                               "no_immutable_runner_claim", "no_release_build"],
    }
    receipt_path = output / "DX0_WINDOWS_HOST_BUILD_RECEIPT.json"
    write_atomic(receipt_path, canonical_json(receipt), mode=0o444)
    receipt_sha = sha256_file(receipt_path)
    write_atomic(output / "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256",
                 f"{receipt_sha}  DX0_WINDOWS_HOST_BUILD_RECEIPT.json\n".encode(),
                 mode=0o444)
    if sorted(path.name for path in output.iterdir()) != [
        "DX0_WINDOWS_HOST_BUILD_RECEIPT.json",
        "DX0_WINDOWS_HOST_BUILD_RECEIPT.sha256",
        "dx0-host-payload.zip",
    ]:
        fail("DX0 Windows three-file envelope differs")
    if dx0_complete_source(source_commit) != source:
        fail("DX0 source changed during Windows production")
    return receipt


def build_workflow(source_commit: str, sdk: pathlib.Path,
                   transaction: pathlib.Path, output: pathlib.Path) -> dict[str, Any]:
    source, source_digest, source_tree, workflow_blob = source_identity(source_commit)
    call_surface = scanner_component_call_surface(repo_root(), ap0=True)
    sdk_identity = verify_sdk(sdk)
    sdk_identity["checkout_regression"] = eol_checkout_regression()
    observed, cl_bv = toolchain_identity()
    _require_empty_new(transaction, "Windows build transaction")
    _require_empty_new(output, "Windows envelope output")

    repo_a, again_a = configure_and_build(transaction / "a", sdk)
    repo_b, again_b = configure_and_build(transaction / "b", sdk)
    assembled_a = transaction / "payload-a"
    assembled_b = transaction / "payload-b"
    roles_a = assemble(repo_a, again_a, sdk, assembled_a)
    roles_b = assemble(repo_b, again_b, sdk, assembled_b)
    if roles_a != roles_b:
        fail("A/B artifact role roster differs")
    pe_a = verify_assembled(assembled_a, roles_a)
    pe_b = verify_assembled(assembled_b, roles_b)
    if [{k: v for k, v in item.items() if k != "dumpbin_sha256"} for item in pe_a] != [
        {k: v for k, v in item.items() if k != "dumpbin_sha256"} for item in pe_b
    ]:
        fail("A/B independent PE receipts differ")

    preliminary = compare_builds(assembled_a, assembled_b)
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    run_attempt = os.environ.get("GITHUB_RUN_ATTEMPT", "")
    if not run_id.isdecimal() or not run_attempt.isdecimal():
        fail("workflow run ID/attempt is malformed")
    workflow = {
        "path": WORKFLOW_PATH,
        "git_blob": workflow_blob,
        "commit": source_commit,
        "run_id": run_id,
        "run_attempt": run_attempt,
        "event": "push",
        "ref": EXPECTED_REF,
    }
    core = {
        "schema": "linux-vst-bridge-wf0-build-identity-core/v1",
        "repository": REPOSITORY,
        "source": {
            "commit": source_commit,
            "tree": source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "implementation_source_schema": source["schema"],
            "implementation_source_record_count": source["record_count"],
            "implementation_source_manifest": source,
            "implementation_source_manifest_sha256": source_digest,
        },
        "workflow": workflow,
        "runner": observed["runner"],
        "toolchain": observed["toolchain"],
        "vst3_sdk": sdk_identity,
        "build": {
            "configuration": CONFIGURATION,
            "targets": list(BUILD_TARGETS),
            "msvc_runtime": "MultiThreaded",
            "dependency_network_during_configure_build": False,
            "roots_distinct": True,
            "deterministic_build_root_mapping": "C:\\wf0\\build",
            "source_date_epoch": SOURCE_DATE_EPOCH,
            "pdb_embedded_path": "%_PDB%",
            "msbuild_max_cpu_count": int(MSBUILD_MAX_CPU_COUNT),
            "msvc_post_options": MSVC_POST_OPTIONS,
            "pre_manifest_comparison": preliminary,
            "wa0_interface_call_surface": call_surface,
        },
    }
    core_data = canonical_json(core)
    for assembled, roles in ((assembled_a, roles_a), (assembled_b, roles_b)):
        write_atomic(assembled / "BUILD_IDENTITY_CORE.json", core_data, mode=0o444)
        roles["BUILD_IDENTITY_CORE.json"] = "build_identity_core"
    manifest_a, digest_a = write_manifest(assembled_a, roles_a, source_digest)
    manifest_b, digest_b = write_manifest(assembled_b, roles_b, source_digest)
    if manifest_a != manifest_b or digest_a != digest_b:
        fail("A/B canonical artifact manifests differ")
    comparison = compare_builds(assembled_a, assembled_b)

    payload = output / "wf0-payload.zip"
    deterministic_zip(assembled_a, payload)
    payload_sha = sha256_file(payload)
    scanner_fault_roster = [
        {"path": item["path"], "size": item["size"], "sha256": item["sha256"]}
        for item in manifest_a["records"]
        if item["role"] in {"scanner_executable", "loader_adapter_executable",
                            "approved_fault_module", "adapter_environment_carrier"}
    ]
    again_roster = [
        {"path": item["path"], "size": item["size"], "sha256": item["sha256"]}
        for item in manifest_a["records"]
        if item["role"] in {"positive_fixture_module", "positive_fixture_resource"}
    ]
    options_digest = sha256_bytes(canonical_json(
        cmake_contract_args("<PHYSICAL_BUILD_ROOT>")
    ))
    receipt = {
        "schema": WINDOWS_BUILD_SCHEMA,
        "repository": {
            "full_name": REPOSITORY,
            "visibility": "private",
            "owner_type": "User",
        },
        "source": {
            "commit": source_commit,
            "tree": source_tree,
            "parent": BASIS_COMMIT,
            "branch": EXPECTED_BRANCH,
            "ref": EXPECTED_REF,
            "implementation_source_manifest": {
                "schema": source["schema"],
                "record_count": source["record_count"],
                "sha256": source_digest,
            },
        },
        "workflow": workflow,
        "runner": observed["runner"],
        "toolchain": observed["toolchain"],
        "vst3_sdk": sdk_identity,
        "build": {
            "configuration": CONFIGURATION,
            "cmake_options_sha256": options_digest,
            "targets": list(BUILD_TARGETS),
            "msvc_runtime": "MultiThreaded",
            "dependency_network_during_configure_build": False,
            "deterministic_build_root_mapping": "C:\\wf0\\build",
            "source_date_epoch": SOURCE_DATE_EPOCH,
            "pdb_embedded_path": "%_PDB%",
            "msbuild_max_cpu_count": int(MSBUILD_MAX_CPU_COUNT),
            "msvc_post_options": MSVC_POST_OPTIONS,
            "build_a": {"artifact_manifest_sha256": digest_a},
            "build_b": {"artifact_manifest_sha256": digest_b},
            "comparison": {
                "level": comparison["level"],
                "path_count": comparison["path_count"],
                "manifest_sha256": comparison["manifest_sha256"],
            },
        },
        "artifacts": {
            "scanner_and_fault_roster": scanner_fault_roster,
            "again_bundle_roster": again_roster,
            "pe_receipts": pe_a,
            "copied_runtime_dependencies": [],
            "artifact_manifest_schema": ARTIFACT_SCHEMA,
            "artifact_manifest_sha256": digest_a,
            "payload_archive_sha256": payload_sha,
        },
        "tool_receipts": {"cl_bv_sha256": sha256_bytes(cl_bv)},
        "explicit_nonclaims": [
            "no_windows_runtime_proof", "no_windows_binary_execution", "no_proton",
            "no_iaudioprocessor_method", "no_bitwig", "no_serum", "no_release_build",
            "no_reproducible_runner_image_claim",
        ],
    }
    receipt_data = canonical_json(receipt)
    receipt_path = output / "WF0_WINDOWS_BUILD_RECEIPT.json"
    write_atomic(receipt_path, receipt_data, mode=0o444)
    receipt_sha = sha256_bytes(receipt_data)
    write_atomic(
        output / "WF0_WINDOWS_BUILD_RECEIPT.sha256",
        f"{receipt_sha}  WF0_WINDOWS_BUILD_RECEIPT.json\n".encode(),
        mode=0o444,
    )
    names = sorted(item.name for item in output.iterdir())
    if names != [
        "WF0_WINDOWS_BUILD_RECEIPT.json",
        "WF0_WINDOWS_BUILD_RECEIPT.sha256",
        "wf0-payload.zip",
    ]:
        fail(f"Windows envelope roster differs: {names}")
    if require_clean_source(source_commit, detached=None) != source:
        fail("repository source changed during the Windows build")
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command_name", required=True)
    acquire = subparsers.add_parser("acquire")
    acquire.add_argument("--sdk-root", type=pathlib.Path, required=True)
    verify = subparsers.add_parser("verify-sdk")
    verify.add_argument("--sdk-root", type=pathlib.Path, required=True)
    subparsers.add_parser("checkout-regression")
    dx0_build = subparsers.add_parser("dx0-workflow-build")
    dx0_build.add_argument("--sdk-root", type=pathlib.Path, required=True)
    dx0_build.add_argument("--transaction-root", type=pathlib.Path, required=True)
    dx0_build.add_argument("--output", type=pathlib.Path, required=True)
    dx0_build.add_argument("--source-commit", required=True)
    args = parser.parse_args()

    if args.command_name == "acquire":
        print(json.dumps(acquire_sdk(args.sdk_root.resolve()), sort_keys=True))
    elif args.command_name == "verify-sdk":
        print(json.dumps(verify_sdk(args.sdk_root.resolve()), sort_keys=True))
    elif args.command_name == "checkout-regression":
        print(json.dumps(eol_checkout_regression(), sort_keys=True))
    else:
        receipt = build_dx0_workflow(
            args.source_commit, args.sdk_root.resolve(),
            args.transaction_root.resolve(), args.output.resolve(),
        )
        print(json.dumps({
            "schema": receipt["schema"],
            "source_commit": receipt["producer_source"]["commit"],
            "windows_build_input_sha256": receipt["windows_build_input"]["sha256"],
            "host_artifact_manifest_sha256": receipt["artifact"]["manifest_sha256"],
            "payload_sha256": receipt["artifact"]["payload_sha256"],
        }, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"WF0_ERROR: {error}", file=sys.stderr, flush=True)
        raise

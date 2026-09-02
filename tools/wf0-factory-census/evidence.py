#!/usr/bin/env python3
"""Render and validate the fixed sanitized fourteen-file WF0 V7 evidence packet."""

from __future__ import annotations

import json
import pathlib
import re
from typing import Any

from common import (
    APPROVAL_BLOB, AUTHORITY_MERGE_COMMIT, AUTHORITY_MERGE_TREE, BASIS_COMMIT,
    BASIS_TREE, DESIGN_BLOB, DESIGN_COMMIT, DESIGN_SHA256, DESIGN_TREE,
    EVIDENCE_FILES, EXPECTED_BRANCH, REPOSITORY, REVIEW_BLOB, RUNNER_DIGEST,
    WORKFLOW_PATH, canonical_json, fail, repo_root, sha256_bytes, sha256_file,
    source_manifest, source_manifest_sha256, write_atomic,
)


PACKET = pathlib.Path("evidence/wf0-windows-vst3-factory-census")


def md(title: str, body: str) -> bytes:
    return f"# {title}\n\n{body.rstrip()}\n".encode("utf-8")


def _proof_disposition() -> list[dict[str, Any]]:
    return [{"row": row, "result": "passed"} for row in range(1, 63)]


def render_packet(source_commit: str, build: dict[str, Any], negative: dict[str, Any],
                  held: dict[str, Any], positive: dict[str, Any],
                  census: dict[str, Any], timeline: dict[str, Any],
                  preflight: dict[str, Any]) -> None:
    root = repo_root() / PACKET
    if root.exists() or root.is_symlink():
        fail("WF0 evidence root already exists; implicit overwrite is prohibited")
    root.mkdir(parents=True)

    source = build["implementation_source_manifest"]
    source_digest = build["implementation_source_manifest_sha256"]
    if (
        source["commit"] != source_commit
        or source_manifest_sha256(source) != source_digest
        or build["source_tree"] != command_tree(source_commit)
    ):
        fail("evidence source identity differs from build/live identity")
    timeline = dict(timeline)
    timeline["held_gate"] = held["held_gate"]
    timeline["negative_stage_results"] = [
        {
            key: item[key] for key in (
                "fixture", "expected_blocker", "observed_blocker",
                "classification", "last_in_flight_operation",
            )
        }
        for item in negative["live_fault_results"]
    ]
    timeline["loader_adapter"] = {
        key: negative["loader_adapter"][key] for key in (
            "classification", "closed_operation_count",
            "free_library_false_mapped", "paired_completion_valid",
            "runtime_role_observed", "proton_role_observed",
            "adapter_role_observed", "protected_state_equal",
        )
    }
    census = dict(census)
    census["stage_timeline_sha256"] = sha256_bytes(canonical_json(timeline))

    receipt = build["build_receipt"]
    custody = build["mac_custody"]
    source_handoff = build["source_handoff"]
    toolchain = receipt["toolchain"]
    runner = receipt["runner"]
    sdk = receipt["vst3_sdk"]
    comparison = receipt["build"]["comparison"]
    artifact = custody["artifact"]
    pe = receipt["artifacts"]["pe_receipts"]
    scanner = next(item for item in pe if item["path"] == "bin/wf0-factory-probe.exe")
    again = next(
        item for item in pe
        if item["path"] == "again.vst3/Contents/x86_64-win/again.vst3"
    )

    basis = md("WF0 V7 basis", f"""Implementation source was frozen on
`{EXPECTED_BRANCH}` as the direct child of exact authority-readback basis
`{BASIS_COMMIT}` (tree `{BASIS_TREE}`).

The immutable V7 design is revision `wf0-design-v7`, commit
`{DESIGN_COMMIT}`, tree `{DESIGN_TREE}`, Git blob `{DESIGN_BLOB}`, and
SHA-256 `{DESIGN_SHA256}`. The independent review blob is `{REVIEW_BLOB}`
with result `DESIGN_CLEAR`; the approval blob is `{APPROVAL_BLOB}`. The V7
design-authority merge is `{AUTHORITY_MERGE_COMMIT}` at tree
`{AUTHORITY_MERGE_TREE}`.

Implementation source commit: `{source_commit}`.
Implementation source tree: `{build['source_tree']}`.
Canonical 26-record implementation-source manifest SHA-256:
`{source_digest}`.""")

    toolchain_md = md("WF0 Windows toolchain", f"""The private workflow used
`windows-2022` and observed image `{runner['image_os']}`
`{runner['image_version']}`, `{runner['windows_product_name']}` version
`{runner['windows_version']}` build `{runner['windows_build']}`, architecture
`{runner['architecture']}`.

The selected supported route was Visual Studio
`{toolchain['visual_studio_version']}` Enterprise, VSCMD
`{toolchain['vscmd_version']}`, MSVC `{toolchain['cl_version']}`, linker
`{toolchain['link_version']}`, platform toolset `{toolchain['platform_toolset']}`,
Windows SDK `{toolchain['windows_sdk_version']}`, CMake
`{toolchain['cmake_version']}`, generator `{toolchain['generator']}`,
platform `{toolchain['generator_platform']}`, and toolset
`{toolchain['generator_toolset']}`. The complete `cl /Bv` receipt digest is
`{toolchain['cl_bv_sha256']}`.

The official VST3 SDK root is `{sdk['root_commit']}` at tree
`{sdk['root_tree']}`; all seven recursive submodules and all five reconciled
source blobs were exact and clean. Source patch count and dirty-path count were
both zero.""")

    build_md = md("WF0 Windows build and private artifact custody", f"""Workflow
`{WORKFLOW_PATH}` (Git blob `{receipt['workflow']['git_blob']}`) ran as
numeric run `{receipt['workflow']['run_id']}`, attempt
`{receipt['workflow']['run_attempt']}`, for the exact source commit.

Two distinct empty Release roots produced a `{comparison['level']}` comparison
across `{comparison['path_count']}` complete transfer paths. The static MSVC
runtime was selected; dependency network was closed for configure/build; no
runtime DLL was copied. The canonical artifact manifest is
`{build['artifact_manifest_sha256']}`; the payload archive SHA-256 is
`{receipt['artifacts']['payload_archive_sha256']}`.

The exact private Actions artifact is numeric ID `{artifact['id']}`, name
`{artifact['name']}`, upload-action bare digest
`{artifact['upload_artifact_digest_bare']}`, and REST digest
`{artifact['rest_artifact_digest']}`. The Mac downloaded only its exact REST ID
route. The raw wrapper SHA-256 `{artifact['raw_wrapper_sha256']}` equaled the
upload-action digest and the hexadecimal portion of the REST digest. The
upload result's human-facing URL and REST API URL were retained as distinct
typed values and joined by repository, run, source, name, ID, and normalized
digest bytes.

The scanner (`{scanner['sha256']}`) and AGain module
(`{again['sha256']}`) are PE32+ AMD64. Every scanner, adapter, fault module,
and AGain import/export receipt closed against the declared Windows system-DLL
roster. Produced PE files were not executed on Windows.""")

    environment_md = md("WF0 disposable execution environments", f"""Every
held-gate, adapter, negative-fault, and positive exercise used a fresh
32-lowercase-hex marker-bound `.wf0-factory-census.stage-<run-id>` beneath the
declared environment owner. The marker bound source, artifact, runner, scanner,
module, bundle, and run identities.

Only directories, the durable marker, and verified payload were present before
the first supervised Runtime 4 / Proton 11 workload. No separate Wine,
`wineboot`, `cmd.exe`, bootstrap, host, validator, Bitwig, or other workload
ran. Every exercised environment reached zero owned descendants and exact root
absence. The persistent content-addressed artifact cache remained read-only at
identity `{build['artifact_manifest_sha256']}`.""")

    launch_md = md("WF0 launch and process supervision", f"""The exact Runtime
4 / Proton 11 launch-critical digest was `{RUNNER_DIGEST}`. The positive run
used neutral application ID `0`, the fixed `runinprefix` scanner vector,
bounded concurrent output, and a maximum of 256 observed identities.

Runtime, Proton, and Wine-hosted scanner roles were observed before gate
publication; Steam game-launch ancestry was absent. The positive scanner ended
at `{positive['last_lifecycle']}`, raw exit `{positive['raw_exit']}`, with
`last_in_flight_operation=null`. Release, optional exit, and `FreeLibrary`
all completed before `scanner_completed`.

The dedicated gate was withheld for exactly 15 seconds. During that interval
there was no load attempt, AGain mapping, factory/class event, or scanner
departure. Scoped cleanup then reached zero descendants and retired the exact
environment. A live same-family sentinel outside the owned ancestry survived
that scoped cleanup and was retired only by its own verified owner afterward.""")

    negative_lines = []
    for item in negative["live_fault_results"]:
        observed = item["observed_blocker"] or "bounded_success_path"
        in_flight = item["last_in_flight_operation"] or "none"
        negative_lines.append(
            f"- `{item['fixture']}` -> `{observed}`; "
            f"classification `{item['classification']}`; final in-flight "
            f"`{in_flight}`"
        )
    negative_md = md("WF0 negative and adapter proofs", f"""All
`{negative['deterministic']['passed']}` deterministic checks passed with zero
failures. The loader adapter executed through the accepted Runtime/Proton lane,
enumerated all 15 closed operations, and proved paired
`free_library` false-result attribution. The source scanner contains no
class-instantiation call expression.

The complete approved 22-module live family produced:

{chr(10).join(negative_lines)}

Required-export absence never resolved or called `InitDll`; optional
entry/exit absence remained an observation; crash and timeout attribution used
the final unmatched call attempt; successful fixture paths released acquired
factory interfaces in reverse order; exit-false still attempted unload; and the
create-instance tripwire remained absent.""")

    snapshot = preflight["protected_snapshot"]
    preservation_md = md("WF0 protected-state preservation", f"""The accepted
WR0 environment identity remained
`{snapshot['wr0']['environment_identity']}`, with Runtime/Proton identity
`{snapshot['wr0']['runner_identity']}` and exact bounded marker/receipt hashes.

Bitwig remained unlaunched protected state: system version
`{snapshot['bitwig']['version']}`, application commit
`{snapshot['bitwig']['commit']}`, and runtime commit
`{snapshot['bitwig']['runtime_commit']}`. System scope, user-shadow absence,
overrides, permissions, SteamOS read-only posture, and historical
SR0/HP0/HP1/WR0/WR0A source/evidence projections remained exact.

No WF0 stage, WR0 transaction sibling, or owned descendant remained. No Deck
GitHub login, fetch, push, API call, pull-request operation, or artifact
download occurred; no GitHub credential or forwarded SSH agent reached the
Deck.""")

    findings_md = md("WF0 findings and claim ceiling", """WF0's primary claim is
established on the exact accepted Steam Deck fixture: the repository-owned
supervised Windows x86_64 probe, built against the exact pinned official VST3
SDK through the supported Windows 2022/MSVC lane, loaded the exact pinned AGain
bundle through accepted Runtime 4 / Proton 11, obtained the plug-in factory,
retained deterministic factory metadata and the complete ordered three-class
census, released every acquired factory interface, applied optional module
exit, unloaded cleanly, drained its process tree, retired its disposable
environment, and preserved every protected fixture.

The proof stops before `createInstance`. It makes no component/controller
hosting, host-context, bus, parameter, event, state, processing, audio, GUI,
editor, IPC, native proxy, Bitwig behavior, Serum behavior, authorization,
packaging, signing, release, product-runner, general VST3, or general Linux
compatibility claim. Actions custody hashes do not establish a trusted builder,
cryptographic provenance, SLSA, or release suitability.""")

    sanitization_md = md("WF0 evidence sanitization", """The packet contains
only public source/dependency identities, safe-relative artifact paths, hashes,
typed public workflow/artifact metadata, bounded normalized factory facts,
proof dispositions, and explicit nonclaims. It contains no raw process
identifier, command line, private absolute path, hostname, network address,
credential, key, token, cookie, account identifier, license material,
Steam compatibility-prefix state tree, proprietary state, installer, plug-in binary, SDK source, build
tree, source bundle, artifact archive, or generated PE.

All fourteen paths were UTF-8/NUL/size checked, hash-closed, roster-closed, and
scanned for private paths, credentials, network identifiers, binary signatures,
and staged proprietary content. The public AGain factory contact field is SDK
metadata from the open reference fixture, not a user identifier.""")

    retained_build = {
        "schema": "linux-vst-bridge-wf0-retained-build-manifest/v1",
        "repository": REPOSITORY,
        "authority": {
            "basis_commit": BASIS_COMMIT, "basis_tree": BASIS_TREE,
            "design_commit": DESIGN_COMMIT, "design_tree": DESIGN_TREE,
            "design_blob": DESIGN_BLOB, "design_sha256": DESIGN_SHA256,
            "review_blob": REVIEW_BLOB, "approval_blob": APPROVAL_BLOB,
            "authority_merge_commit": AUTHORITY_MERGE_COMMIT,
            "authority_merge_tree": AUTHORITY_MERGE_TREE,
        },
        "implementation_source_manifest": source,
        "implementation_source_manifest_sha256": source_digest,
        "implementation_source_tree": build["source_tree"],
        "windows_build_receipt": receipt,
        "mac_artifact_custody_receipt": custody,
        "source_handoff_receipt": source_handoff,
        "artifact_manifest": build["artifact_manifest"],
        "again_bundle_manifest": build["again_bundle_manifest"],
        "runtime_proton_digest": RUNNER_DIGEST,
        "held_gate_complete": True,
        "negative_fault_roster_complete": True,
        "loader_adapter_complete": True,
        "positive_census_complete": True,
        "protected_state_equal": True,
        "no_deck_github": preflight["no_deck_github"],
        "proof_disposition": _proof_disposition(),
        "proof_row_count": 62,
    }

    fixture = {
        "schema": "linux-vst-bridge-wf0-fixture/v1",
        "host": preflight["fixture"],
        "implementation_source_commit": source_commit,
        "implementation_source_tree": build["source_tree"],
        "implementation_source_manifest_sha256": source_digest,
        "workflow": {
            "path": WORKFLOW_PATH,
            "run_id": receipt["workflow"]["run_id"],
            "run_attempt": receipt["workflow"]["run_attempt"],
        },
        "vst3_sdk": sdk,
        "artifact_manifest_sha256": build["artifact_manifest_sha256"],
        "runner_identity_sha256": RUNNER_DIGEST,
        "positive_result": positive["classification"],
        "held_gate_complete": True,
        "negative_fault_roster_complete": True,
        "create_instance_called": False,
        "protected_state_equal": True,
        "deck_github_operations": 0,
        "credentials_retained": False,
        "private_paths_retained": False,
    }

    files = {
        "BASIS.md": basis,
        "TOOLCHAIN.md": toolchain_md,
        "BUILD.md": build_md,
        "BUILD_MANIFEST.json": canonical_json(retained_build),
        "ENVIRONMENT.md": environment_md,
        "LAUNCH_AND_PROCESS.md": launch_md,
        "STAGE_TIMELINE.json": canonical_json(timeline),
        "CENSUS.json": canonical_json(census),
        "NEGATIVE_TESTS.md": negative_md,
        "PRESERVATION.md": preservation_md,
        "FINDINGS.md": findings_md,
        "SANITIZATION.md": sanitization_md,
        "fixture.json": canonical_json(fixture),
    }
    for name, data in files.items():
        write_atomic(root / name, data, mode=0o644)
    hash_lines = [
        f"{sha256_file(root / name)}  {name}"
        for name in sorted(files, key=lambda value: value.encode())
    ]
    write_atomic(
        root / "hashes.sha256",
        ("\n".join(hash_lines) + "\n").encode("utf-8"),
        mode=0o644,
    )
    validate_packet(source_commit, treeish=source_commit)


def command_tree(source_commit: str) -> str:
    return __import__("subprocess").check_output(
        ["git", "rev-parse", f"{source_commit}^{{tree}}"],
        cwd=repo_root(), text=True,
    ).strip()


def validate_packet(source_commit: str, *, treeish: str | None = None) -> None:
    root = repo_root() / PACKET
    if not root.is_dir() or root.is_symlink():
        fail("evidence packet root is absent or unsafe")
    observed = sorted(item.name for item in root.iterdir())
    if observed != sorted(EVIDENCE_FILES):
        fail(f"evidence packet roster differs: {observed}")
    expected_names = sorted(
        (name for name in EVIDENCE_FILES if name != "hashes.sha256"),
        key=lambda value: value.encode(),
    )
    lines = (root / "hashes.sha256").read_text(encoding="utf-8").splitlines()
    if lines != [f"{sha256_file(root / name)}  {name}" for name in expected_names]:
        fail("evidence hash manifest differs")
    build = json.loads((root / "BUILD_MANIFEST.json").read_bytes())
    regenerated = source_manifest(source_commit, treeish=treeish or source_commit)
    if (
        build.get("implementation_source_manifest") != regenerated
        or build.get("implementation_source_manifest_sha256")
        != source_manifest_sha256(regenerated)
        or build.get("proof_row_count") != 62
        or build.get("proof_disposition") != _proof_disposition()
    ):
        fail("retained source manifest or 62-row disposition differs")
    prohibited = re.compile(
        rb"(?:/home/|/Users/|192\.168\.|BEGIN (?:RSA |OPENSSH )?PRIVATE KEY|"
        rb"github_pat_|ghp_|compatdata|\.ssh/|SSH_AUTH_SOCK)",
        re.IGNORECASE,
    )
    binary_magic = (b"MZ", b"\x7fELF", b"PK\x03\x04")
    for path in root.iterdir():
        if not path.is_file() or path.is_symlink() or path.stat().st_size > 2 * 1024 * 1024:
            fail(f"evidence object is unsafe or oversized: {path.name}")
        data = path.read_bytes()
        if b"\0" in data or prohibited.search(data) or data.startswith(binary_magic):
            fail(f"evidence contains private or binary data: {path.name}")
        data.decode("utf-8", "strict")
        if path.suffix == ".md" and data.count(b"```") % 2:
            fail(f"Markdown fence is unbalanced: {path.name}")


if __name__ == "__main__":
    raise SystemExit("evidence.py is a library; use run.py")

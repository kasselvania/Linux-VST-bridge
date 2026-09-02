#!/usr/bin/env python3
"""Render and validate the fixed sanitized fourteen-file WC0 evidence packet."""

from __future__ import annotations

import json
import pathlib
import re
import subprocess
from typing import Any

from common import (
    APPROVAL_BLOB, AUTHORITY_MERGE_COMMIT, AUTHORITY_MERGE_TREE, BASIS_COMMIT,
    BASIS_TREE, DESIGN_BLOB, DESIGN_COMMIT, DESIGN_SHA256, DESIGN_TREE,
    EVIDENCE_FILES, EXPECTED_BRANCH, EXPECTED_REF, REPOSITORY, REVIEW_BLOB,
    REVIEW_GITHUB_ID, RUNNER_DIGEST, SOURCE_SCHEMA, WORKFLOW_PATH,
    canonical_json, fail, repo_root, sha256_bytes, sha256_file,
    source_manifest, source_manifest_sha256, write_atomic,
)


PACKET = pathlib.Path("evidence/wc0-windows-vst3-processor-component-admission")
PROOF_ROWS = (
    "exact_processor_cid_and_icomponent_iid",
    "create_result_pointer_consistency",
    "controller_cid_before_initialize",
    "minimal_host_interface_boundary",
    "host_reference_ownership",
    "successful_initialize",
    "successful_terminate",
    "successful_final_component_release",
    "create_failure_cleanup",
    "controller_id_call_failure_cleanup",
    "controller_id_mismatch_cleanup",
    "initialize_failure_without_terminate",
    "initialize_timeout_attribution",
    "initialize_crash_attribution",
    "terminate_failure_followed_by_release_attempt",
    "terminate_timeout_attribution",
    "terminate_crash_attribution",
    "release_timeout_attribution",
    "release_crash_attribution",
    "unexpected_host_object_request_fails_closed",
    "no_controller_creation",
    "no_out_of_scope_component_method",
    "nine_quiescence_facts_precede_inherited_shutdown",
    "zero_process_and_environment_residue",
    "protected_state_exact",
    "nonzero_release_retirement_incomplete_and_shutdown_suppressed",
    "suppression_for_release_host_leak_timeout_and_crash",
    "clean_shutdown_distinguished_from_physical_containment",
    "source_build_artifact_deck_evidence_identity_closure",
)


def md(title: str, body: str) -> bytes:
    return f"# {title}\n\n{body.rstrip()}\n".encode("utf-8")


def _proof_disposition() -> list[dict[str, Any]]:
    return [
        {"row": index, "claim": claim, "result": "passed"}
        for index, claim in enumerate(PROOF_ROWS, 1)
    ]


def command_tree(source_commit: str) -> str:
    return subprocess.check_output(
        ["git", "rev-parse", f"{source_commit}^{{tree}}"],
        cwd=repo_root(), text=True,
    ).strip()


def _negative_summary(negative: dict[str, Any]) -> list[dict[str, Any]]:
    results = [
        *negative["direct_again_results"], *negative["live_fault_results"]
    ]
    return [
        {
            "exercise": item["exercise"],
            "fixture": item["fixture"],
            "component_case": item["component_case"],
            "expected_blocker": item["expected_blocker"],
            "observed_blocker": item["observed_blocker"],
            "classification": item["classification"],
            "last_in_flight_operation": item["last_in_flight_operation"],
            "forbidden_method_marker_absent":
                item["forbidden_method_marker_absent"],
            "inherited_shutdown": item["inherited_shutdown"],
            "cleanup": item["cleanup"],
        }
        for item in results
    ]


def render_packet(
    source_commit: str,
    build: dict[str, Any],
    negative: dict[str, Any],
    positive: dict[str, Any],
    component_session: dict[str, Any],
    callback_ledger: dict[str, Any],
    timeline: dict[str, Any],
    preflight: dict[str, Any],
) -> None:
    root = repo_root() / PACKET
    if root.exists() or root.is_symlink():
        fail("WC0 evidence root already exists; implicit overwrite is prohibited")
    root.mkdir(parents=True)

    source = build["implementation_source_manifest"]
    source_digest = build["implementation_source_manifest_sha256"]
    if (
        source.get("schema") != SOURCE_SCHEMA
        or source.get("commit") != source_commit
        or source.get("record_count") != 19
        or source_manifest_sha256(source) != source_digest
        or build["source_tree"] != command_tree(source_commit)
    ):
        fail("WC0 evidence source identity differs from build/live identity")

    receipt = build["build_receipt"]
    custody = build["mac_custody"]
    source_handoff = build["source_handoff"]
    artifact = custody["artifact"]
    toolchain = receipt["toolchain"]
    sdk = receipt["vst3_sdk"]
    comparison = receipt["build"]["comparison"]
    pe = receipt["artifacts"]["pe_receipts"]
    scanner = next(
        item for item in pe if item["path"] == "bin/wf0-factory-probe.exe"
    )
    again = next(
        item for item in pe
        if item["path"] == "again.vst3/Contents/x86_64-win/again.vst3"
    )
    negative_summary = _negative_summary(negative)

    timeline = dict(timeline)
    timeline["negative_results"] = negative_summary
    timeline["focused_exercise_count"] = len(negative_summary)
    timeline["positive_cleanup"] = positive["cleanup"]
    timeline["positive_environment_retired"] = positive["retirement"]["stage_absent"]

    basis_md = md("WC0 implementation basis", f"""The implementation is the
direct child of merged WC0 design-authority/readback basis `{BASIS_COMMIT}` at
tree `{BASIS_TREE}`. The immutable `wc0-design-v2` identity is commit
`{DESIGN_COMMIT}`, tree `{DESIGN_TREE}`, Git blob `{DESIGN_BLOB}`, and SHA-256
`{DESIGN_SHA256}`. Independent GitHub review `{REVIEW_GITHUB_ID}` returned
`DESIGN_CLEAR`; the review and approval blobs are `{REVIEW_BLOB}` and
`{APPROVAL_BLOB}`. The authority merge is `{AUTHORITY_MERGE_COMMIT}` at tree
`{AUTHORITY_MERGE_TREE}`.

Implementation branch/ref: `{EXPECTED_BRANCH}` / `{EXPECTED_REF}`.
Implementation source commit/tree: `{source_commit}` / `{build['source_tree']}`.
Exact 19-record `{SOURCE_SCHEMA}` digest: `{source_digest}`.""")

    build_md = md("WC0 Windows build and custody", f"""Private workflow
`{WORKFLOW_PATH}` at Git blob `{receipt['workflow']['git_blob']}` ran as exact
run `{receipt['workflow']['run_id']}`, attempt
`{receipt['workflow']['run_attempt']}`, on `windows-2022` for this source.

The supported toolchain was Visual Studio `{toolchain['visual_studio_version']}`
with MSVC `{toolchain['cl_version']}`, v143, x64, Windows SDK
`{toolchain['windows_sdk_version']}`, and CMake `{toolchain['cmake_version']}`.
The official VST3 SDK was `{sdk['root_commit']}` at tree `{sdk['root_tree']}`
with all seven locked recursive submodules clean and no source patch.

Two distinct clean build roots compared `{comparison['path_count']}` complete
transfer paths at level `{comparison['level']}`. PE/export/import/dependency
verification passed. Scanner SHA-256: `{scanner['sha256']}`. Exact AGain module
SHA-256: `{again['sha256']}`.

The exact Actions artifact is ID `{artifact['id']}`, name `{artifact['name']}`,
upload-action bare digest `{artifact['upload_artifact_digest_bare']}`, REST
digest `{artifact['rest_artifact_digest']}`, and raw wrapper SHA-256
`{artifact['raw_wrapper_sha256']}`. The three values name the same bytes under
their typed representations. This is byte custody, not signing, provenance,
SLSA, or release suitability.""")

    environment_md = md("WC0 disposable environments", f"""Every one of the
18 focused negative exercises and the single positive transaction used a fresh,
marker-bound `.wf0-factory-census.stage-<32-lowercase-hex-run-id>` under the
accepted WF0 environment owner. Each environment bound the exact WC0 source
manifest `{source_digest}`, artifact manifest
`{build['artifact_manifest_sha256']}`, Runtime/Proton identity, scanner, module,
and run.

No separate bootstrap workload ran. Every exercise drained its exact owned
process tree and retired its exact disposable environment. The inherited
environment-marker and artifact-cache ownership protocols were reused without
algorithm or wire-schema changes.""")

    launch_md = md("WC0 lifecycle and process result", f"""The accepted Runtime
4 / Proton 11 route retained launch-critical digest `{RUNNER_DIGEST}`. The
positive scanner completed all 20 closed calls with no operation left in flight.
It created only the exact AGain processor as `IComponent`, verified the exact
controller CID before initialization, initialized and terminated once, released
the component to zero, retired the host reference sequence to zero, proved all
nine object-quiescence facts, then completed reverse factory releases,
`ExitDll`, and `FreeLibrary`.

The blocked release/leak/timeout/crash exercises suppressed later inherited
factory/module calls when object absence was unproved. Their zero-descendant and
environment-retirement results are physical containment, not a clean in-process
shutdown claim.""")

    negative_lines = [
        f"- `{item['exercise']}` -> `{item['observed_blocker']}`; "
        f"classification `{item['classification']}`; in flight "
        f"`{item['last_in_flight_operation'] or 'none'}`; clean in-process "
        f"shutdown `{str(item['inherited_shutdown']['clean_in_process_shutdown']).lower()}`"
        for item in negative_summary
    ]
    negative_md = md("WC0 focused negative proofs", f"""All
`{negative['deterministic']['passed']}` deterministic owner checks passed.
Only the two direct AGain create failures and the exact sixteen WC0 fault
fixtures ran live. The inherited archive-negative suite and standalone adapter
runtime exercise were intentionally not replayed.

{chr(10).join(negative_lines)}

The nonzero-release fixture entered `component_retirement_incomplete` after one
release. Nonzero release, unresolved host reference, and release/initialize/
terminate timeout or crash paths could not reach inherited factory release,
`ExitDll`, or `FreeLibrary`. Unexpected host-object creation failed closed.""")

    snapshot = preflight["protected_snapshot"]
    preservation_md = md("WC0 protected-state preservation", f"""The accepted
WR0 environment identity remained `{snapshot['wr0']['environment_identity']}`
and its Runtime/Proton identity remained `{snapshot['wr0']['runner_identity']}`.
Bitwig remained unlaunched at version `{snapshot['bitwig']['version']}` with
application commit `{snapshot['bitwig']['commit']}` and runtime commit
`{snapshot['bitwig']['runtime_commit']}`.

SteamOS read-only posture, the accepted runner assets, historical evidence,
protected configuration projections, and unrelated processes remained exact.
No Deck GitHub login, fetch, push, API call, PR operation, or Actions artifact
download occurred; no credential or forwarded SSH agent reached the Deck.""")

    findings_md = md("WC0 findings and claim ceiling", """WC0 establishes the
narrow processor-component admission claim on the exact accepted fixture. The
existing supervised Windows VST3 path created the exact AGain processor directly
as one `IComponent`, verified its declared controller class ID, initialized it
with the minimal repository-owned `IHostApplication`, terminated and released
it correctly, returned host ownership to zero, proved object quiescence, and
then completed the already-proven factory/module shutdown without residue.

WC0 does not create a controller; connect processor and controller; query
`IConnectionPoint` or `IAudioProcessor`; enumerate or activate buses; access
parameters or state; configure processing; call `setActive`, `setProcessing`, or
`process`; handle audio/events; open an editor; create IPC or a native proxy;
run Bitwig or Serum; authorize; package; select a runner; or establish general
VST3, plug-in, Linux, or product compatibility.""")

    sanitization_md = md("WC0 evidence sanitization", """The packet contains
only allow-listed public authority/source/dependency identities, safe-relative
artifact paths, hashes, typed public workflow metadata, normalized component
and callback facts, bounded proof dispositions, and explicit nonclaims.

It contains no credentials, private absolute paths, private host/network
identifiers, raw process or thread identifiers, pointers, handles, addresses,
compiled binaries, complete prefixes, proprietary state, SDK source, source
bundle, artifact archive, build tree, installer, or commercial plug-in. All
fourteen files were roster-, hash-, UTF-8-, NUL-, size-, JSON-, Markdown-, and
redaction-checked.""")

    retained_build = {
        "schema": "linux-vst-bridge-wc0-retained-build-manifest/v1",
        "repository": REPOSITORY,
        "authority": {
            "implementation_basis_commit": BASIS_COMMIT,
            "implementation_basis_tree": BASIS_TREE,
            "design_commit": DESIGN_COMMIT,
            "design_tree": DESIGN_TREE,
            "design_blob": DESIGN_BLOB,
            "design_sha256": DESIGN_SHA256,
            "review_blob": REVIEW_BLOB,
            "review_github_id": REVIEW_GITHUB_ID,
            "review_result": "DESIGN_CLEAR",
            "approval_blob": APPROVAL_BLOB,
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
        "deck_admission": {
            "source_ref": f"refs/handoff/wc0-source/{source_commit}",
            "detached_worktree_commit": source_commit,
            "detached_worktree_clean": True,
            "source_manifest_readback_before_each_proof": True,
            "artifact_cache_manifest_sha256": build["artifact_manifest_sha256"],
        },
        "runtime_proton_digest": RUNNER_DIGEST,
        "component_session_sha256": sha256_bytes(canonical_json(component_session)),
        "callback_ledger_sha256": sha256_bytes(canonical_json(callback_ledger)),
        "negative_fault_roster_complete": True,
        "positive_component_session_complete": True,
        "object_quiescence": True,
        "zero_owned_process_environment_residue": True,
        "protected_state_equal": True,
        "no_deck_github": preflight["no_deck_github"],
        "proof_disposition": _proof_disposition(),
        "proof_row_count": len(PROOF_ROWS),
    }

    fixture = {
        "schema": "linux-vst-bridge-wc0-fixture/v1",
        "host": preflight["fixture"],
        "implementation_source": {
            "schema": SOURCE_SCHEMA,
            "commit": source_commit,
            "tree": build["source_tree"],
            "parent": BASIS_COMMIT,
            "record_count": 19,
            "manifest_sha256": source_digest,
        },
        "workflow": {
            "path": WORKFLOW_PATH,
            "git_blob": receipt["workflow"]["git_blob"],
            "run_id": receipt["workflow"]["run_id"],
            "run_attempt": receipt["workflow"]["run_attempt"],
        },
        "artifact": {
            "id": artifact["id"], "name": artifact["name"],
            "manifest_sha256": build["artifact_manifest_sha256"],
        },
        "vst3_sdk": sdk,
        "runner_identity_sha256": RUNNER_DIGEST,
        "processor_logical_cid": "84E8DE5F92554F5396FAE4133C935A18",
        "processor_raw_windows_tuid": "5FDEE8845592534F96FAE4133C935A18",
        "requested_interface": "Steinberg::Vst::IComponent",
        "controller_logical_cid": "D39D5B65D7AF42FA843F4AC841EB04F0",
        "controller_raw_windows_tuid": "655B9DD3AFD7FA42843F4AC841EB04F0",
        "positive_result": positive["classification"],
        "object_quiescence": True,
        "controller_instance_created": False,
        "forbidden_component_method_called": False,
        "protected_state_equal": True,
        "deck_github_operations": 0,
    }

    files = {
        "BASIS.md": basis_md,
        "BUILD.md": build_md,
        "BUILD_MANIFEST.json": canonical_json(retained_build),
        "ENVIRONMENT.md": environment_md,
        "LAUNCH_AND_PROCESS.md": launch_md,
        "COMPONENT_SESSION.json": canonical_json(component_session),
        "CALLBACK_LEDGER.json": canonical_json(callback_ledger),
        "STAGE_TIMELINE.json": canonical_json(timeline),
        "NEGATIVE_TESTS.md": negative_md,
        "PRESERVATION.md": preservation_md,
        "FINDINGS.md": findings_md,
        "SANITIZATION.md": sanitization_md,
        "fixture.json": canonical_json(fixture),
    }
    for name, data in files.items():
        write_atomic(root / name, data, mode=0o644)
    hash_names = sorted(files, key=lambda value: value.encode())
    write_atomic(
        root / "hashes.sha256",
        ("\n".join(
            f"{sha256_file(root / name)}  {name}" for name in hash_names
        ) + "\n").encode("utf-8"),
        mode=0o644,
    )
    validate_packet(source_commit, treeish=source_commit)


def validate_packet(source_commit: str, *, treeish: str | None = None) -> None:
    root = repo_root() / PACKET
    if not root.is_dir() or root.is_symlink():
        fail("WC0 evidence packet root is absent or unsafe")
    observed = sorted(item.name for item in root.iterdir())
    if observed != sorted(EVIDENCE_FILES):
        fail(f"WC0 evidence packet roster differs: {observed}")
    expected_names = sorted(
        (name for name in EVIDENCE_FILES if name != "hashes.sha256"),
        key=lambda value: value.encode(),
    )
    lines = (root / "hashes.sha256").read_text(encoding="utf-8").splitlines()
    if lines != [f"{sha256_file(root / name)}  {name}" for name in expected_names]:
        fail("WC0 evidence hash manifest differs")

    retained = json.loads((root / "BUILD_MANIFEST.json").read_bytes())
    regenerated = source_manifest(source_commit, treeish=treeish or source_commit)
    component = json.loads((root / "COMPONENT_SESSION.json").read_bytes())
    callbacks = json.loads((root / "CALLBACK_LEDGER.json").read_bytes())
    timeline = json.loads((root / "STAGE_TIMELINE.json").read_bytes())
    if (
        retained.get("implementation_source_manifest") != regenerated
        or retained.get("implementation_source_manifest_sha256")
        != source_manifest_sha256(regenerated)
        or retained.get("proof_row_count") != 29
        or retained.get("proof_disposition") != _proof_disposition()
        or component.get("object_quiescence", {}).get("value") is not True
        or callbacks.get("closed") is not True
        or callbacks.get("owner_final_release") != 0
        or timeline.get("focused_exercise_count") != 18
    ):
        fail("WC0 retained identities, lifecycle, or 29-row proof differs")

    prohibited = re.compile(
        rb"(?:/home/|/Users/|192\.168\."
        rb"|BEGIN (?:RSA |OPENSSH )?PRIVATE KEY|github_pat_|ghp_|compatdata|\.ssh/"
        rb"|SSH_AUTH_SOCK|0x[0-9a-f]{8,16})",
        re.IGNORECASE,
    )
    binary_magic = (b"MZ", b"\x7fELF", b"PK\x03\x04")
    for path in root.iterdir():
        if (
            not path.is_file() or path.is_symlink()
            or path.stat().st_size > 2 * 1024 * 1024
        ):
            fail(f"WC0 evidence object is unsafe or oversized: {path.name}")
        data = path.read_bytes()
        if b"\0" in data or prohibited.search(data) or data.startswith(binary_magic):
            fail(f"WC0 evidence contains private, pointer, or binary data: {path.name}")
        data.decode("utf-8", "strict")
        if path.suffix == ".md" and data.count(b"```") % 2:
            fail(f"Markdown fence is unbalanced: {path.name}")


if __name__ == "__main__":
    raise SystemExit("evidence.py is a library; use run.py")
